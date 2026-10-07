//! Run scope-sim from the command line, without the dashboard.
//!
//!     scope-sim passes RUN.yaml
//!     scope-sim run RUN.yaml [--pass N|highest|lowest] [--seed N]
//!                            [--format json|csv|summary] [--out FILE] [--presets FILE]
//!
//! A run file names a configuration, a scenario and a pass; examples are in
//! `crates/scope-sim/runs/`. `run` writes the trace (JSON, the same layout
//! `Trace::from_json` reads back), the samples as CSV, or just the summary.
//! A one-line summary always goes to standard error.

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::process::ExitCode;

use scope_sim::run::write_csv;
use scope_sim::{find_passes, PassChoice, Presets, RunFile, SimError, Trace};

const USAGE: &str = "\
usage:
  scope-sim passes RUN.yaml [--presets FILE]
  scope-sim run RUN.yaml [--pass N|highest|lowest] [--seed N]
                         [--format json|csv|summary] [--out FILE] [--presets FILE]

A run file names a configuration, a scenario and a pass. Examples are in
crates/scope-sim/runs/. Passes are numbered from 0, as `passes` lists them.";

#[derive(Debug, Clone, Copy, PartialEq)]
enum Format {
    Json,
    Csv,
    Summary,
}

struct Args {
    command: String,
    run_file: String,
    pass: Option<PassChoice>,
    seed: Option<u64>,
    format: Format,
    out: Option<String>,
    presets: Option<String>,
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Args, String> {
    let command = args.next().ok_or("no command given")?;
    if command == "-h" || command == "--help" {
        return Err(String::new());
    }
    if command != "run" && command != "passes" {
        return Err(format!("unknown command \"{command}\""));
    }
    let mut parsed = Args { command, run_file: String::new(), pass: None, seed: None, format: Format::Json, out: None, presets: None };
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or_else(|| format!("{name} needs a value"));
        match arg.as_str() {
            "--pass" => parsed.pass = Some(PassChoice::parse(&value("--pass")?).map_err(|e| e.0)?),
            "--seed" => {
                let v = value("--seed")?;
                parsed.seed = Some(v.parse().map_err(|_| format!("--seed must be a whole number, not \"{v}\""))?);
            }
            "--format" => {
                parsed.format = match value("--format")?.as_str() {
                    "json" => Format::Json,
                    "csv" => Format::Csv,
                    "summary" => Format::Summary,
                    other => return Err(format!("--format is json, csv or summary, not \"{other}\"")),
                }
            }
            "--out" => parsed.out = Some(value("--out")?),
            "--presets" => parsed.presets = Some(value("--presets")?),
            "-h" | "--help" => return Err(String::new()),
            flag if flag.starts_with('-') => return Err(format!("unknown option \"{flag}\"")),
            path if parsed.run_file.is_empty() => parsed.run_file = path.to_string(),
            extra => return Err(format!("unexpected argument \"{extra}\"")),
        }
    }
    if parsed.run_file.is_empty() {
        return Err("no run file given".into());
    }
    Ok(parsed)
}

fn read(path: &str) -> Result<String, SimError> {
    std::fs::read_to_string(path).map_err(|e| SimError::new(format!("{path}: {e}")))
}

fn output(path: &Option<String>) -> Result<Box<dyn Write>, SimError> {
    Ok(match path {
        Some(p) => Box::new(BufWriter::new(File::create(p).map_err(|e| SimError::new(format!("{p}: {e}")))?)),
        None => Box::new(BufWriter::new(io::stdout().lock())),
    })
}

fn io_error(e: io::Error) -> SimError {
    SimError::new(format!("writing output: {e}"))
}

/// A value as the JSON spells it, e.g. `"sunlit"`, without the quotes.
fn label<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default()
}

fn one_line(trace: &Trace) -> String {
    let s = &trace.summary;
    format!(
        "{} | {} pass {} (max el {:.1} deg) | {}: {} RMS error {:.1}\", max {:.1}\"",
        trace.info.hardware.name,
        trace.info.target,
        trace.info.pass.index,
        trace.info.pass.max_el_deg,
        label(&s.verdict).to_uppercase(),
        s.verdict_reason,
        s.rms_err_arcsec,
        s.max_err_arcsec,
    )
}

fn execute(args: &Args) -> Result<(), SimError> {
    let mut run = RunFile::from_yaml(&read(&args.run_file)?)?;
    if let Some(pass) = args.pass {
        run.pass = pass;
    }
    if let Some(seed) = args.seed {
        run.scenario.seed = seed;
    }
    let presets = match &args.presets {
        Some(path) => Presets::from_yaml(&read(path)?)?,
        None => Presets::builtin()?,
    };

    if args.command == "passes" {
        let list = find_passes(&run.scenario)?;
        let mut out = output(&args.out)?;
        writeln!(out, "{}: {} pass(es)", list.target, list.passes.len()).map_err(io_error)?;
        for p in &list.passes {
            writeln!(
                out,
                "{:>3}  {}  {:>6.0} s  max el {:>5.1} deg  satellite {}, site {}",
                p.index,
                p.rise,
                p.duration_s,
                p.max_el_deg,
                label(&p.lighting),
                label(&p.site_dark)
            )
            .map_err(io_error)?;
        }
        for w in &list.warnings {
            eprintln!("warning: {w}");
        }
        return out.flush().map_err(io_error);
    }

    let trace = run.run(&presets)?;
    let mut out = output(&args.out)?;
    match args.format {
        Format::Json => {
            serde_json::to_writer(&mut out, &trace).map_err(|e| SimError::new(e.to_string()))?;
            writeln!(out).map_err(io_error)?;
        }
        Format::Csv => write_csv(&trace.samples, &mut out).map_err(io_error)?,
        Format::Summary => {
            serde_json::to_writer_pretty(&mut out, &trace.summary).map_err(|e| SimError::new(e.to_string()))?;
            writeln!(out).map_err(io_error)?;
        }
    }
    out.flush().map_err(io_error)?;
    eprintln!("{}", one_line(&trace));
    Ok(())
}

fn main() -> ExitCode {
    let args = match parse_args(std::env::args().skip(1)) {
        Ok(a) => a,
        Err(message) if message.is_empty() => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(message) => {
            eprintln!("scope-sim: {message}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    match execute(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("scope-sim: {e}");
            ExitCode::FAILURE
        }
    }
}
