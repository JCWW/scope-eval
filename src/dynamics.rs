//! Mount axis dynamics: acceleration, the alt-az keyhole, and slew timing.
//!
//! Pure functions (numbers in, number out), like the calculation section of
//! `checks.rs`. The judgments live in `regimes.rs`.
//!
//! Angles are radians and rates are per second unless a name says otherwise.
//! Formulas and worked examples are in README.md.

use crate::constants::PEAK_ACCEL_COEFF;

/// Peak angular acceleration of an overhead pass, rad/s^2.
///
/// `alpha = PEAK_ACCEL_COEFF * omega^2`, where `omega = v/h` is the peak rate.
/// See the `PEAK_ACCEL_COEFF` doc comment for the derivation.
pub fn peak_tracking_accel_rad_s2(omega_rad_s: f64) -> f64 {
    PEAK_ACCEL_COEFF * omega_rad_s * omega_rad_s
}

/// Smallest zenith distance an alt-az mount can follow within its azimuth
/// acceleration limit, radians.
///
/// Near the zenith the azimuth angle sweeps through the same `atan` form as
/// the pass itself, with the minimum zenith distance `z` in place of the
/// altitude, so peak azimuth acceleration is `PEAK_ACCEL_COEFF * (omega/z)^2`.
/// Requiring that to stay within `max_accel` gives
/// `z >= omega * sqrt(PEAK_ACCEL_COEFF / max_accel)`.
///
/// A non-positive rating yields infinity: no pass is followable. Callers
/// should filter the rating with [`crate::model::plausible`] first so that a
/// typo reads as "not entered" rather than as an unfollowable mount.
pub fn accel_limited_keyhole_rad(omega_rad_s: f64, max_accel_rad_s2: f64) -> f64 {
    if max_accel_rad_s2 <= 0.0 {
        f64::INFINITY
    } else {
        omega_rad_s * (PEAK_ACCEL_COEFF / max_accel_rad_s2).sqrt()
    }
}

/// The alt-az keyhole, radians, and which limit sets it.
///
/// The rate limit gives `z >= omega / max_rate` and the acceleration limit
/// gives the value above. These are two constraints on one physical keyhole,
/// so the binding (larger) one is what the mount actually suffers.
/// `max_accel_rad_s2` of `None` means not entered, leaving the rate limit
/// alone — which is exactly the behaviour this tool had before acceleration
/// was modelled.
pub fn keyhole_rad(
    omega_rad_s: f64,
    max_rate_rad_s: f64,
    max_accel_rad_s2: Option<f64>,
) -> (f64, &'static str) {
    let z_rate = omega_rad_s / max_rate_rad_s;
    match max_accel_rad_s2 {
        Some(a) => {
            let z_accel = accel_limited_keyhole_rad(omega_rad_s, a);
            if z_accel > z_rate {
                (z_accel, "acceleration")
            } else {
                (z_rate, "rate")
            }
        }
        None => (z_rate, "rate"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    /// A 500 km overhead LEO pass: omega = v/h = 0.87234 deg/s.
    const LEO_OMEGA_DEG_S: f64 = 0.872_343;

    #[test]
    fn peak_accel_coeff_matches_its_closed_form() {
        // 3 sqrt(3) / 8, the peak of d2/dt2 atan(vt/h) in units of (v/h)^2.
        assert!(close(PEAK_ACCEL_COEFF, 3.0 * 3.0f64.sqrt() / 8.0, 1e-12));
    }

    #[test]
    fn leo_peak_tracking_accel() {
        let omega = LEO_OMEGA_DEG_S.to_radians();
        let accel_deg_s2 = peak_tracking_accel_rad_s2(omega).to_degrees();
        // 0.0086 deg/s^2: negligible for any real mount, which is why the
        // acceleration model cannot stop at this number.
        assert!(close(accel_deg_s2, 0.008_627, 1e-6));
    }

    #[test]
    fn peak_accel_scales_as_omega_squared() {
        let a = peak_tracking_accel_rad_s2(0.01);
        let b = peak_tracking_accel_rad_s2(0.02);
        assert!(close(b / a, 4.0, 1e-9));
    }

    #[test]
    fn keyhole_is_rate_limited_when_acceleration_is_unknown() {
        // The compatibility guarantee for Task 12: z = omega / v_max
        // = 0.87234 / 50 rad = 1.0 deg, so elevation 89.0 deg.
        let (z, binding) = keyhole_rad(LEO_OMEGA_DEG_S.to_radians(), 50.0f64.to_radians(), None);
        assert!(close(90.0 - z.to_degrees(), 89.0, 0.01));
        assert_eq!(binding, "rate");
    }

    #[test]
    fn acceleration_tightens_the_keyhole() {
        let omega = LEO_OMEGA_DEG_S.to_radians();
        let rate = 50.0f64.to_radians();
        for (accel_deg_s2, expected_elev) in [(10.0f64, 88.32f64), (2.0, 86.24), (0.5, 82.48)] {
            let (z, binding) = keyhole_rad(omega, rate, Some(accel_deg_s2.to_radians()));
            let elev = 90.0 - z.to_degrees();
            assert!(
                close(elev, expected_elev, 0.02),
                "at {accel_deg_s2} deg/s^2 got {elev} deg, wanted {expected_elev}"
            );
            assert_eq!(binding, "acceleration");
        }
    }

    #[test]
    fn a_very_fast_axis_leaves_the_rate_limit_binding() {
        // With enormous acceleration the rate limit is what remains.
        let omega = LEO_OMEGA_DEG_S.to_radians();
        let (z, binding) = keyhole_rad(omega, 50.0f64.to_radians(), Some(1e6f64.to_radians()));
        assert!(close(90.0 - z.to_degrees(), 89.0, 0.01));
        assert_eq!(binding, "rate");
    }

    #[test]
    fn keyhole_is_total_for_a_zero_acceleration_rating() {
        // Review Focus 2 is handled by the caller, but the pure function must
        // still be total rather than producing a NaN.
        let z = accel_limited_keyhole_rad(0.015, 0.0);
        assert!(z.is_infinite());
    }
}
