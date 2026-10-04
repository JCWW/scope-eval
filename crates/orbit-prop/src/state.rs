//! Position and velocity at an instant.

use crate::time::Epoch;

/// Position (km) and velocity (km/s) in the TEME frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StateVector {
    pub epoch: Epoch,
    pub r_km: [f64; 3],
    pub v_km_s: [f64; 3],
}
