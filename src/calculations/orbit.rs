//! Orbital speed and apparent-rate calculations.

use crate::constants::{ARCSEC_PER_CIRCLE, ARCSEC_PER_RADIAN, EARTH_RADIUS_KM, MU_EARTH};

/// Independent orbital motion calculations, using kilometres and seconds.
pub struct OrbitCalculator;

impl OrbitCalculator {
    /// Circular-orbit speed at altitude h, km/s: v = sqrt(mu / (R + h)).
    pub fn circular_speed_km_s(altitude_km: f64) -> f64 {
        (MU_EARTH / (EARTH_RADIUS_KM + altitude_km)).sqrt()
    }

    /// Angular rate of a directly overhead circular-orbit pass, arcsec/s.
    /// Uses omega ~= v/h and ignores Earth's rotation.
    pub fn overhead_rate_arcsec_s(altitude_km: f64) -> f64 {
        Self::circular_speed_km_s(altitude_km) / altitude_km * ARCSEC_PER_RADIAN
    }

    /// Rate against the stars for an orbit with the given period, arcsec/s.
    pub fn rate_from_period_arcsec_s(period_s: f64) -> f64 {
        ARCSEC_PER_CIRCLE / period_s
    }

    /// Small-angle position uncertainty from cross-track distance, arcsec.
    pub fn position_uncertainty_arcsec(distance_km: f64, range_km: f64) -> f64 {
        distance_km / range_km * ARCSEC_PER_RADIAN
    }

    /// Speed at radius r on an orbit with semi-major axis a, km/s (vis-viva).
    pub fn vis_viva_km_s(radius_km: f64, semi_major_axis_km: f64) -> f64 {
        (MU_EARTH * (2.0 / radius_km - 1.0 / semi_major_axis_km)).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::OrbitCalculator;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn circular_orbit_speed_and_overhead_rates() {
        assert!(close(
            OrbitCalculator::circular_speed_km_s(500.0),
            7.613,
            0.002
        ));
        assert!(close(
            OrbitCalculator::overhead_rate_arcsec_s(500.0),
            3140.0,
            2.0
        ));
        assert!(close(
            OrbitCalculator::overhead_rate_arcsec_s(20_200.0),
            39.6,
            0.2
        ));
    }

    #[test]
    fn period_rate_and_vis_viva_speed() {
        assert!(close(
            OrbitCalculator::rate_from_period_arcsec_s(86_164.0905),
            15.041,
            0.001
        ));
        assert!(close(
            OrbitCalculator::rate_from_period_arcsec_s(27.321_661 * 86_400.0),
            0.549,
            0.001
        ));
        assert!(close(
            OrbitCalculator::vis_viva_km_s(26_560.0 * 1.74, 26_560.0),
            1.497,
            0.005
        ));
        assert!(close(
            OrbitCalculator::position_uncertainty_arcsec(2.0, 500.0),
            825.06,
            0.01
        ));
    }
}
