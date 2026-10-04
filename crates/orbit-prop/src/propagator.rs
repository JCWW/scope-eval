//! The common interface every orbit model implements.

use crate::error::OrbitPropError;
use crate::state::StateVector;
use crate::time::Epoch;

/// Anything that can say where a satellite is at a given time.
///
/// Downstream code (observation geometry, pass finding, the simulator)
/// depends only on this trait, never on a concrete orbit model.
pub trait Propagator {
    /// Position and velocity in TEME at `t`.
    fn propagate(&self, t: Epoch) -> Result<StateVector, OrbitPropError>;
    /// Nominal orbital period, seconds. Used to choose pass-search step sizes.
    fn period_s(&self) -> f64;
    /// Human-readable label, e.g. the TLE name or "what-if orbit".
    fn label(&self) -> &str;
}
