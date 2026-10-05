//! Time for the simulation loop.
//!
//! The loop decides when the next step is due; the clock says when it
//! actually arrived. A simulated clock arrives at once and exactly on
//! time. A real-time clock, for hardware in the loop, would sleep until the
//! wall clock reaches the step and report when it woke.

use crate::error::SimError;

/// One step of the loop: the time it ends, seconds since the start of the
/// pass, and how long it lasted. The first measurement, before any step,
/// is at `t_s = 0` with `dt_s = 0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tick {
    pub t_s: f64,
    pub dt_s: f64,
}

pub trait Clock {
    /// Return once `t_s` (seconds since the start of the pass) has been
    /// reached, with the time it actually was.
    fn wait_until(&mut self, t_s: f64) -> Result<f64, SimError>;
}

/// Simulated time: every step arrives at once and exactly when due.
#[derive(Debug, Clone, Default)]
pub struct SimClock;

impl Clock for SimClock {
    fn wait_until(&mut self, t_s: f64) -> Result<f64, SimError> {
        Ok(t_s)
    }
}
