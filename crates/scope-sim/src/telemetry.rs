//! Where recorded samples go besides the simulation's own list.
//!
//! The simulation always keeps its samples in memory, for the dashboard. A
//! sink receives each one as it is recorded, so a long run, or one on real
//! hardware, can be streamed to a file or a socket while it happens.

use std::io::Write;

use crate::error::SimError;
use crate::sim::Sample;

pub trait TelemetrySink {
    fn record(&mut self, sample: &Sample) -> Result<(), SimError>;
}

/// Writes each sample as one line of JSON.
pub struct JsonLinesSink<W: Write> {
    out: W,
}

impl<W: Write> JsonLinesSink<W> {
    pub fn new(out: W) -> JsonLinesSink<W> {
        JsonLinesSink { out }
    }

    pub fn into_inner(self) -> W {
        self.out
    }
}

impl<W: Write> TelemetrySink for JsonLinesSink<W> {
    fn record(&mut self, sample: &Sample) -> Result<(), SimError> {
        serde_json::to_writer(&mut self.out, sample).map_err(|e| SimError::new(format!("telemetry: {e}")))?;
        self.out.write_all(b"\n").map_err(|e| SimError::new(format!("telemetry: {e}")))
    }
}
