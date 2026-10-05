//! scope-eval as a library: the hardware model, the eight checks, the
//! orbital-regime evaluations, pass judgments and the text reports.
//!
//! Nothing in the library reads input or prints. The `scope-eval` binary
//! (src/main.rs and src/cli/) is a thin front end over it.
//!
//! Layers, lowest first:
//!
//! * `model`, `constants`, `presets`: data types, limits and preset hardware
//! * `calculations`: the physics, as pure functions
//! * `checks`, `regimes`, `photometry`, `passes`: evaluation, applying the
//!   PASS/WARN/FAIL rules to the physics
//! * `report`: turning evaluations into text

pub mod calculations;
pub mod checks;
pub mod constants;
pub mod model;
pub mod passes;
pub mod photometry;
pub mod presets;
pub mod regimes;
pub mod report;
