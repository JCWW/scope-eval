//! A full telescope + camera + mount configuration to evaluate.

use super::camera::Camera;
use super::mount::Payload;
use super::optics::Telescope;

/// One telescope + camera + mount combination to evaluate.
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub label: String,
    pub telescope: Telescope,
    pub camera: Camera,
    pub payload: Payload,
    /// How accurately each image is timestamped, milliseconds.
    /// Roughly tens of ms for a PC clock plus USB latency, sub-millisecond for GPS hardware timestamping.
    pub timestamp_accuracy_ms: f64,
    /// Apparent magnitude of the target, if the user entered one.
    /// `None` uses the magnitude derived per regime from the reference target.
    pub target_mag_override: Option<f64>,
    /// Exposure time, if the user entered one. `None` uses the trail-limited exposure.
    pub exposure_override_s: Option<f64>,
}
