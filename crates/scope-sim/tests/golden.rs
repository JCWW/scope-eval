//! Golden traces: each run file in `runs/` is simulated and compared with
//! the trace recorded in `tests/golden/`. Any change to what the simulation
//! computes shows up here, so a refactor that should change nothing can be
//! shown to change nothing.
//!
//! Numbers must agree to a relative 1e-9 (absolute 1e-9 near zero), which
//! allows for platform differences in the last bits of `sin` and `cos`
//! but nothing a model change could hide in. Everything else must match
//! exactly. To record new golden traces after an intended change, run
//!
//!     UPDATE_GOLDEN=1 cargo test -p scope-sim --test golden
//!
//! and review the diff of `tests/golden/` before committing it.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use scope_sim::{Presets, RunFile, Trace};

/// Keep every `KEEP_EVERY`th sample, and the last, so the golden files
/// stay small enough to review: one sample every 5 s of the pass.
const KEEP_EVERY: usize = 10;
const REL_TOL: f64 = 1e-9;
const ABS_TOL: f64 = 1e-9;

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn thinned(mut trace: Trace) -> Trace {
    let last = trace.samples.len().saturating_sub(1);
    trace.samples = trace.samples.into_iter().enumerate().filter(|(i, _)| i % KEEP_EVERY == 0 || *i == last).map(|(_, s)| s).collect();
    trace
}

/// Where `actual` and `expected` differ, as JSON-pointer-like paths.
fn differences(path: &str, actual: &Value, expected: &Value, out: &mut Vec<String>) {
    match (actual, expected) {
        (Value::Number(a), Value::Number(e)) => {
            let (a, e) = (a.as_f64().unwrap(), e.as_f64().unwrap());
            if (a - e).abs() > ABS_TOL.max(REL_TOL * e.abs()) {
                out.push(format!("{path}: {a} != {e}"));
            }
        }
        (Value::Array(a), Value::Array(e)) if a.len() == e.len() => {
            for (i, (a, e)) in a.iter().zip(e).enumerate() {
                differences(&format!("{path}/{i}"), a, e, out);
            }
        }
        (Value::Object(a), Value::Object(e)) if a.len() == e.len() && a.keys().all(|k| e.contains_key(k)) => {
            for (k, a) in a {
                differences(&format!("{path}/{k}"), a, &e[k], out);
            }
        }
        (a, e) if a == e => {}
        (a, e) => out.push(format!("{path}: {a} != {e}")),
    }
}

fn check(run_path: &Path) {
    let name = run_path.file_stem().unwrap().to_string_lossy().into_owned();
    let golden_path = crate_dir().join("tests/golden").join(format!("{name}.json"));
    let run = RunFile::from_yaml(&fs::read_to_string(run_path).unwrap()).unwrap();
    let trace = thinned(run.run(&Presets::builtin().unwrap()).unwrap());
    let actual = serde_json::to_value(&trace).unwrap();

    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        fs::create_dir_all(golden_path.parent().unwrap()).unwrap();
        fs::write(&golden_path, serde_json::to_string_pretty(&actual).unwrap() + "\n").unwrap();
        return;
    }
    let text = fs::read_to_string(&golden_path).unwrap_or_else(|e| {
        panic!("{}: {e}. Record it with UPDATE_GOLDEN=1 cargo test -p scope-sim --test golden", golden_path.display())
    });
    // The golden file must also read back as a trace of the current layout.
    Trace::from_json(&text).unwrap();
    let expected: Value = serde_json::from_str(&text).unwrap();
    let mut diffs = Vec::new();
    differences("", &actual, &expected, &mut diffs);
    assert!(
        diffs.is_empty(),
        "{name} differs from {} in {} place(s); the first:\n  {}",
        golden_path.display(),
        diffs.len(),
        diffs.iter().take(10).cloned().collect::<Vec<_>>().join("\n  ")
    );
}

#[test]
fn every_run_file_matches_its_golden_trace() {
    let mut runs: Vec<PathBuf> = fs::read_dir(crate_dir().join("runs"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "yaml"))
        .collect();
    runs.sort();
    assert!(runs.len() >= 3, "expected the example run files, found {runs:?}");
    for run in &runs {
        check(run);
    }
}

#[test]
fn the_comparison_catches_a_change() {
    let expected: Value = serde_json::json!({"a": [1.0, 2.0], "b": "x", "c": true});
    let mut diffs = Vec::new();
    differences("", &serde_json::json!({"a": [1.0, 2.0 + 1e-12], "b": "x", "c": true}), &expected, &mut diffs);
    assert!(diffs.is_empty(), "{diffs:?}");
    differences("", &serde_json::json!({"a": [1.0, 2.001], "b": "y", "c": false}), &expected, &mut diffs);
    assert_eq!(diffs.len(), 3, "{diffs:?}");
    diffs.clear();
    differences("", &serde_json::json!({"a": [1.0], "b": "x", "c": true}), &expected, &mut diffs);
    assert_eq!(diffs.len(), 1);
}
