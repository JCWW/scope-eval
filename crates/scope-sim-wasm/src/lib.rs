//! WebAssembly bindings for `scope-sim`.
//!
//! Every value crosses the boundary as a JSON string, so the JavaScript
//! side needs no generated types beyond these functions; `dashboard/src/sim/types.ts`
//! mirrors the Rust structs. Errors become JavaScript exceptions carrying
//! the `SimError` message.

use serde::Serialize;
use wasm_bindgen::prelude::*;

use scope_sim::{ConfigSpec, Presets, ScenarioSpec, SimError, Simulation};

/// scope-eval's hardware presets, compiled in so the dashboard and the
/// command-line tool can never disagree about a spec.
const PRESETS_YAML: &str = include_str!("../../../presets.yaml");

fn presets_parsed() -> Result<Presets, SimError> {
    Presets::from_yaml(PRESETS_YAML)
}

fn from_json<T: serde::de::DeserializeOwned>(text: &str, what: &str) -> Result<T, SimError> {
    serde_json::from_str(text).map_err(|e| SimError::new(format!("{what}: {e}")))
}

fn to_json<T: Serialize>(value: &T) -> Result<String, SimError> {
    serde_json::to_string(value).map_err(|e| SimError::new(e.to_string()))
}

fn js(e: SimError) -> JsError {
    JsError::new(&e.0)
}

/// The telescope, camera and mount presets, as JSON.
#[wasm_bindgen]
pub fn presets() -> Result<String, JsError> {
    presets_parsed().and_then(|p| to_json(&p)).map_err(js)
}

/// Resolve a configuration against the presets: the optics and the mount
/// model the simulation will use, with assumed values marked.
#[wasm_bindgen(js_name = resolveConfig)]
pub fn resolve_config(config_json: &str) -> Result<String, JsError> {
    let spec: ConfigSpec = from_json(config_json, "configuration").map_err(js)?;
    presets_parsed().and_then(|p| p.resolve(&spec)).and_then(|hw| to_json(&hw)).map_err(js)
}

/// Passes of the scenario's target over its site in its search window.
#[wasm_bindgen(js_name = findPasses)]
pub fn find_passes(scenario_json: &str) -> Result<String, JsError> {
    let scenario: ScenarioSpec = from_json(scenario_json, "scenario").map_err(js)?;
    scope_sim::find_passes(&scenario).and_then(|l| to_json(&l)).map_err(js)
}

/// Run one configuration through a whole pass and return its summary.
/// The dashboard calls this once per configuration to fill its comparison
/// table, yielding to the browser between calls.
#[wasm_bindgen(js_name = runToEnd)]
pub fn run_to_end(config_json: &str, scenario_json: &str, pass_index: usize) -> Result<String, JsError> {
    SimHandle::new(config_json, scenario_json, pass_index)?.sim.run_to_end().and_then(|s| to_json(&s)).map_err(js)
}

#[derive(Serialize)]
struct Advance<'a> {
    samples: &'a [scope_sim::Sample],
    current: scope_sim::Sample,
    summary: scope_sim::Summary,
    done: bool,
}

/// A running simulation. JavaScript owns the clock: it calls `advance`
/// with however much simulated time should pass, typically the frame time
/// multiplied by the playback speed.
#[wasm_bindgen]
pub struct SimHandle {
    sim: Simulation,
}

#[wasm_bindgen]
impl SimHandle {
    #[wasm_bindgen(constructor)]
    pub fn new(config_json: &str, scenario_json: &str, pass_index: usize) -> Result<SimHandle, JsError> {
        let spec: ConfigSpec = from_json(config_json, "configuration").map_err(js)?;
        let scenario: ScenarioSpec = from_json(scenario_json, "scenario").map_err(js)?;
        let hw = presets_parsed().and_then(|p| p.resolve(&spec)).map_err(js)?;
        Ok(SimHandle { sim: Simulation::new(hw, &scenario, pass_index).map_err(js)? })
    }

    /// Static facts about the run: hardware, pass, duration, pointing offset.
    pub fn info(&self) -> Result<String, JsError> {
        to_json(&self.sim.info()).map_err(js)
    }

    /// The predicted path across the sky.
    pub fn track(&self, step_s: f64) -> Result<String, JsError> {
        self.sim.track(step_s).and_then(|t| to_json(&t)).map_err(js)
    }

    /// Every sample recorded so far.
    pub fn samples(&self) -> Result<String, JsError> {
        to_json(&self.sim.samples()).map_err(js)
    }

    /// Simulate `seconds` more. Returns the newly recorded samples, the
    /// current state, the running summary and whether the pass is over.
    pub fn advance(&mut self, seconds: f64) -> Result<String, JsError> {
        let n = self.sim.advance(seconds).map_err(js)?.len();
        let all = self.sim.samples();
        let out = Advance {
            samples: &all[all.len() - n..],
            current: self.sim.current(),
            summary: self.sim.summary(),
            done: self.sim.is_done(),
        };
        to_json(&out).map_err(js)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_presets_parse() {
        let p = presets_parsed().unwrap();
        assert!(!p.telescopes.is_empty() && !p.cameras.is_empty() && !p.mounts.is_empty());
    }
}
