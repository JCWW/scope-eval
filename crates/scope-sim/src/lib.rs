//! Time-stepped simulation of a telescope mount tracking a satellite pass.
//!
//! `orbit-prop` says where the satellite is. This crate adds what the
//! hardware does about it: a mount whose axes follow the predicted path
//! within their rate and acceleration limits, a pointing model that is never
//! perfect, and a prediction that is never exactly where the satellite is.
//! The output is the pointing error on the sky, sample by sample, and
//! whether the target stayed inside the camera's field.
//!
//! The model and its assumptions are documented in the crate README.

pub mod error;
pub mod geometry;
pub mod hardware;
pub mod presets;
pub mod rng;
pub mod scenario;
pub mod servo;
pub mod sim;
pub mod star;

pub use error::SimError;
pub use hardware::{Hardware, MountModel, Optics, Param};
pub use presets::{ConfigSpec, MountOverrides, Presets};
pub use scenario::{find_passes, PassList, PassSummary, ScenarioSpec, SiteSpec, TargetSpec};
pub use sim::{Sample, SimInfo, Simulation, Summary, TrackPoint, Verdict};
pub use star::{CenterCorner, Conditions, StarImage};
