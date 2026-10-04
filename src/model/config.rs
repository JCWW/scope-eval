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
}
