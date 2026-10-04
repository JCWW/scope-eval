//! Mount tracking, axis-keyhole, and slew calculations.

use crate::constants::PEAK_ACCEL_COEFF;

/// Independent calculations for mount axis motion and slew profiles.
pub struct MountDynamicsCalculator;

impl MountDynamicsCalculator {
    /// Peak angular acceleration of an overhead pass, rad/s^2.
    pub fn peak_tracking_accel_rad_s2(omega_rad_s: f64) -> f64 {
        PEAK_ACCEL_COEFF * omega_rad_s * omega_rad_s
    }

    /// Smallest alt-az zenith distance allowed by the acceleration limit, radians.
    pub fn accel_limited_keyhole_rad(omega_rad_s: f64, max_accel_rad_s2: f64) -> f64 {
        if max_accel_rad_s2 <= 0.0 {
            f64::INFINITY
        } else {
            omega_rad_s * (PEAK_ACCEL_COEFF / max_accel_rad_s2).sqrt()
        }
    }

    /// Keyhole zenith distance and binding constraint (`rate` or `acceleration`).
    pub fn keyhole_rad(
        omega_rad_s: f64,
        max_rate_rad_s: f64,
        max_accel_rad_s2: Option<f64>,
    ) -> (f64, &'static str) {
        let z_rate = omega_rad_s / max_rate_rad_s;
        match max_accel_rad_s2 {
            Some(a) => {
                let z_accel = Self::accel_limited_keyhole_rad(omega_rad_s, a);
                if z_accel > z_rate {
                    (z_accel, "acceleration")
                } else {
                    (z_rate, "rate")
                }
            }
            None => (z_rate, "rate"),
        }
    }

    /// Time to slew a distance under rate and acceleration limits, seconds.
    /// Uses a trapezoidal profile when the rate cap is reached and a triangular
    /// profile for shorter slews.
    pub fn slew_time_s(distance_deg: f64, max_rate_deg_s: f64, max_accel_deg_s2: f64) -> f64 {
        if max_rate_deg_s <= 0.0 || max_accel_deg_s2 <= 0.0 {
            return f64::INFINITY;
        }
        let ramp_distance = max_rate_deg_s * max_rate_deg_s / max_accel_deg_s2;
        if distance_deg >= ramp_distance {
            max_rate_deg_s / max_accel_deg_s2 + distance_deg / max_rate_deg_s
        } else {
            2.0 * (distance_deg / max_accel_deg_s2).sqrt()
        }
    }
}

/// Independent calculations for the payload carried by a mount.
pub struct PayloadCalculator;

impl PayloadCalculator {
    /// Total supported load from telescope, camera, and accessories, pounds.
    pub fn total_weight_lb(telescope_lb: f64, camera_lb: f64, accessories_lb: f64) -> f64 {
        telescope_lb + camera_lb + accessories_lb
    }

    /// Payload as a fraction of the mount's rated capacity.
    pub fn capacity_fraction(payload_lb: f64, capacity_lb: f64) -> f64 {
        payload_lb / capacity_lb
    }

    /// Remaining back-focus margin, mm. A negative result is a shortfall.
    pub fn back_focus_margin_mm(available_mm: f64, required_mm: f64) -> f64 {
        available_mm - required_mm
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::PEAK_ACCEL_COEFF;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    const LEO_OMEGA_DEG_S: f64 = 0.872_343;

    #[test]
    fn peak_accel_coefficient_matches_closed_form() {
        assert!(close(PEAK_ACCEL_COEFF, 3.0 * 3.0f64.sqrt() / 8.0, 1e-12));
    }

    #[test]
    fn leo_peak_tracking_accel() {
        let omega = LEO_OMEGA_DEG_S.to_radians();
        let accel_deg_s2 = MountDynamicsCalculator::peak_tracking_accel_rad_s2(omega).to_degrees();
        assert!(close(accel_deg_s2, 0.008_627, 1e-6));
    }

    #[test]
    fn peak_accel_scales_as_omega_squared() {
        let a = MountDynamicsCalculator::peak_tracking_accel_rad_s2(0.01);
        let b = MountDynamicsCalculator::peak_tracking_accel_rad_s2(0.02);
        assert!(close(b / a, 4.0, 1e-9));
    }

    #[test]
    fn keyhole_is_rate_limited_when_acceleration_is_unknown() {
        let (z, binding) = MountDynamicsCalculator::keyhole_rad(
            LEO_OMEGA_DEG_S.to_radians(),
            50.0f64.to_radians(),
            None,
        );
        assert!(close(90.0 - z.to_degrees(), 89.0, 0.01));
        assert_eq!(binding, "rate");
    }

    #[test]
    fn acceleration_tightens_the_keyhole() {
        let omega = LEO_OMEGA_DEG_S.to_radians();
        let rate = 50.0f64.to_radians();
        for (accel_deg_s2, expected_elev) in [(10.0f64, 88.32f64), (2.0, 86.24), (0.5, 82.48)] {
            let (z, binding) =
                MountDynamicsCalculator::keyhole_rad(omega, rate, Some(accel_deg_s2.to_radians()));
            let elev = 90.0 - z.to_degrees();
            assert!(close(elev, expected_elev, 0.02));
            assert_eq!(binding, "acceleration");
        }
    }

    #[test]
    fn fast_axis_leaves_rate_limit_binding() {
        let omega = LEO_OMEGA_DEG_S.to_radians();
        let (z, binding) = MountDynamicsCalculator::keyhole_rad(
            omega,
            50.0f64.to_radians(),
            Some(1e6f64.to_radians()),
        );
        assert!(close(90.0 - z.to_degrees(), 89.0, 0.01));
        assert_eq!(binding, "rate");
    }

    #[test]
    fn slew_time_uses_triangular_profile_below_rate_limit() {
        assert!(close(
            MountDynamicsCalculator::slew_time_s(90.0, 50.0, 10.0),
            6.0,
            0.01
        ));
    }

    #[test]
    fn slew_time_uses_trapezoidal_profile_at_rate_limit() {
        assert!(close(
            MountDynamicsCalculator::slew_time_s(90.0, 50.0, 50.0),
            2.8,
            0.01
        ));
        assert!(close(
            MountDynamicsCalculator::slew_time_s(90.0, 6.0, 1.0),
            21.0,
            0.01
        ));
    }

    #[test]
    fn slew_time_is_continuous_at_profile_boundary() {
        let (v, a) = (50.0, 10.0);
        let boundary = v * v / a;
        let below = MountDynamicsCalculator::slew_time_s(boundary - 1e-6, v, a);
        let above = MountDynamicsCalculator::slew_time_s(boundary + 1e-6, v, a);
        assert!(close(below, above, 1e-4));
    }

    #[test]
    fn slew_time_scales_with_distance_and_mount_capability() {
        assert!(
            MountDynamicsCalculator::slew_time_s(180.0, 50.0, 10.0)
                > MountDynamicsCalculator::slew_time_s(90.0, 50.0, 10.0)
        );
        assert!(
            MountDynamicsCalculator::slew_time_s(90.0, 50.0, 20.0)
                < MountDynamicsCalculator::slew_time_s(90.0, 50.0, 10.0)
        );
        assert!(
            MountDynamicsCalculator::slew_time_s(90.0, 10.0, 10.0)
                > MountDynamicsCalculator::slew_time_s(90.0, 50.0, 10.0)
        );
    }

    #[test]
    fn slew_time_is_total_for_zero_capability() {
        assert!(MountDynamicsCalculator::slew_time_s(90.0, 0.0, 10.0).is_infinite());
        assert!(MountDynamicsCalculator::slew_time_s(90.0, 50.0, 0.0).is_infinite());
        assert!(MountDynamicsCalculator::slew_time_s(90.0, -1.0, -1.0).is_infinite());
    }

    #[test]
    fn zero_distance_slew_takes_no_time() {
        assert!(close(
            MountDynamicsCalculator::slew_time_s(0.0, 50.0, 10.0),
            0.0,
            1e-12
        ));
    }

    #[test]
    fn keyhole_is_total_for_zero_acceleration() {
        assert!(MountDynamicsCalculator::accel_limited_keyhole_rad(0.015, 0.0).is_infinite());
    }

    #[test]
    fn payload_and_back_focus_margins() {
        assert_eq!(PayloadCalculator::total_weight_lb(30.0, 10.0, 5.0), 45.0);
        assert_eq!(PayloadCalculator::capacity_fraction(45.0, 100.0), 0.45);
        assert_eq!(PayloadCalculator::back_focus_margin_mm(55.0, 40.0), 15.0);
        assert_eq!(PayloadCalculator::back_focus_margin_mm(35.0, 40.0), -5.0);
    }
}
