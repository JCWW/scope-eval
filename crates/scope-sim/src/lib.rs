//! Time-stepped simulation of a telescope mount tracking a satellite pass.
//!
//! `orbit-prop` says where the satellite is. This crate adds what the
//! hardware does about it: a mount whose axes follow the predicted path
//! within their rate and acceleration limits, a pointing model that is never
//! perfect, and a prediction that is never exactly where the satellite is.
//! The output is the pointing error on the sky, sample by sample, and
//! whether the target stayed inside the camera's field.
//!
//! A run is assembled from swappable parts (see `sim`): a clock, a
//! tracker, a mount and a sensor. `Simulation::new` simulates all four;
//! `Simulation::from_parts` lets any of them be replaced, for instance by
//! real hardware.
//!
//! The model and its assumptions are documented in the crate README.

pub mod clock;
pub mod error;
pub mod evaluator;
pub mod geometry;
pub mod hardware;
pub mod mount;
pub mod presets;
pub mod rng;
pub mod run;
pub mod scenario;
pub mod sensor;
pub mod servo;
pub mod sim;
pub mod star;
pub mod telemetry;
pub mod tracker;

pub use clock::{Clock, SimClock, Tick};
pub use error::SimError;
pub use evaluator::Evaluator;
pub use hardware::{Hardware, MountModel, Optics, Param, Source};
pub use mount::{AxisCommand, AxisState, MountCapabilities, MountCommand, MountDriver, MountState, SimMount};
pub use presets::{ConfigSpec, MountOverrides, Presets, PRESETS_YAML};
pub use run::{PassBy, PassChoice, RunFile, Trace};
pub use scenario::{
    find_passes, PassLightingLabel, PassList, PassSummary, ScenarioSpec, SiteDarkLabel, SiteSpec, TargetSpec,
};
pub use sensor::{Measurement, Sensor, SyntheticSensor, Truth};
pub use sim::{
    Parts, RunSetup, Sample, SimInfo, Simulation, Summary, TargetLighting, TrackPoint, Verdict, SCHEMA_VERSION,
};
pub use star::{CenterCorner, Conditions, StarImage};
pub use telemetry::{JsonLinesSink, TelemetrySink};
pub use tracker::{OpenLoopTracker, Tracker};
