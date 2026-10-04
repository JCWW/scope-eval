//! Photometry: how bright a target is, and whether it can be detected.
//!
//! Every function here is pure (numbers in, number out) so it can be unit
//! tested without the interactive front end, mirroring the calculation
//! section of `checks.rs`. The judgments live in `regimes.rs`.
//!
//! Formulas and worked examples are in README.md.

use std::f64::consts::PI;

use crate::constants::{MAX_EXPOSURE_S, M_PER_KM, PHOTONS_M2_S_MAG0, SUN_APPARENT_MAG};

/// Apparent magnitude of a diffuse (Lambertian) target.
///
/// `m = m_sun - 2.5 log10(albedo * area * phase / (pi * d^2))`, with the area
/// in square metres and the range converted to metres. The `1/pi` is the
/// Lambertian scattering factor.
pub fn derived_target_mag(cross_section_m2: f64, albedo: f64, range_km: f64, phase: f64) -> f64 {
    let d_m = range_km * M_PER_KM;
    SUN_APPARENT_MAG - 2.5 * (albedo * cross_section_m2 * phase / (PI * d_m * d_m)).log10()
}

/// Electrons per second from a point source of the given magnitude.
pub fn signal_e_per_s(mag: f64, eff_area_m2: f64, qe: f64, throughput: f64) -> f64 {
    PHOTONS_M2_S_MAG0 * 10f64.powf(-0.4 * mag) * eff_area_m2 * qe * throughput
}

/// Electrons per second per pixel from the sky background.
///
/// The sky is quoted per square arcsecond, so this is the point-source rate
/// for that surface brightness scaled by the solid angle one pixel covers.
pub fn sky_e_per_px_s(
    sky_mag_arcsec2: f64,
    plate_scale: f64,
    eff_area_m2: f64,
    qe: f64,
    throughput: f64,
) -> f64 {
    signal_e_per_s(sky_mag_arcsec2, eff_area_m2, qe, throughput) * plate_scale * plate_scale
}

/// Electrons a magnitude-zero target would deposit over the whole exposure.
/// Inverting the SNR equation for a magnitude needs this; see `limiting_mag`.
pub fn signal_coefficient(eff_area_m2: f64, qe: f64, throughput: f64, exposure_s: f64) -> f64 {
    PHOTONS_M2_S_MAG0 * eff_area_m2 * qe * throughput * exposure_s
}

/// How far the target moves across the sensor during the exposure, arcsec.
pub fn trail_arcsec(residual_rate_arcsec_s: f64, exposure_s: f64) -> f64 {
    residual_rate_arcsec_s * exposure_s
}

/// Longest exposure that keeps the target's trail inside one seeing disk.
///
/// A target the mount holds still has no residual rate and so nothing to
/// trail, which would imply an unbounded exposure; that case and any
/// non-positive rate return `MAX_EXPOSURE_S`.
pub fn trail_limited_exposure_s(seeing_arcsec: f64, residual_rate_arcsec_s: f64) -> f64 {
    if residual_rate_arcsec_s <= 0.0 {
        MAX_EXPOSURE_S
    } else {
        (seeing_arcsec / residual_rate_arcsec_s).min(MAX_EXPOSURE_S)
    }
}

/// Pixels the target's light lands on: a seeing disk smeared along the trail.
///
/// Approximated as a rectangle, matching the `(pixels across)^2` footprint
/// approximation already used by check 5. The ratio between configurations is
/// what matters, not the absolute pixel count.
pub fn footprint_px(seeing_arcsec: f64, trail_arcsec: f64, plate_scale: f64) -> f64 {
    let across = seeing_arcsec / plate_scale;
    let along = (seeing_arcsec + trail_arcsec) / plate_scale;
    across * along
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    /// The reference target: 10 m^2 at albedo 0.2, full phase.
    fn mag_at(range_km: f64) -> f64 {
        derived_target_mag(10.0, 0.2, range_km, 1.0)
    }

    #[test]
    fn derived_mag_matches_worked_examples() {
        assert!(close(mag_at(500.0), 2.25, 0.01)); // LEO
        assert!(close(mag_at(20_200.0), 10.28, 0.01)); // MEO
        assert!(close(mag_at(37_000.0), 11.59, 0.01)); // GEO
        assert!(close(mag_at(39_836.0), 11.75, 0.01)); // HEO, Molniya apogee
        assert!(close(mag_at(384_400.0), 16.67, 0.01)); // cislunar
    }

    #[test]
    fn derived_mag_lands_where_real_objects_do() {
        // The absolute scale is the thing most easily wrong by a constant
        // factor. Real GEO objects run 11-15, real cislunar 16-20.
        let geo = mag_at(37_000.0);
        let cis = mag_at(384_400.0);
        assert!((11.0..=15.0).contains(&geo), "GEO magnitude {geo} outside the observed range");
        assert!((16.0..=20.0).contains(&cis), "cislunar magnitude {cis} outside the observed range");
    }

    #[test]
    fn four_times_the_range_is_three_magnitudes_fainter() {
        // Inverse square: a factor of 4 in range is 2.5*log10(16) = 3.01 mag.
        assert!(close(mag_at(40_000.0) - mag_at(10_000.0), 3.01, 0.01));
    }

    // The worked configuration from README.md: DeltaRho 350 (0.0660 m^2
    // effective area, 0.7386 "/px with 3.76 um pixels), QE 0.80,
    // throughput 0.85, sky 21.0 mag/arcsec^2.
    const AREA: f64 = 0.0660;
    const SCALE: f64 = 0.7386;
    const QE: f64 = 0.80;
    const THRU: f64 = 0.85;

    #[test]
    fn geo_signal_rate() {
        assert!(close(signal_e_per_s(11.5914, AREA, QE, THRU), 9222.4, 1.0));
    }

    #[test]
    fn sky_rate_per_pixel() {
        assert!(close(sky_e_per_px_s(21.0, SCALE, AREA, QE, THRU), 0.8674, 0.0005));
    }

    #[test]
    fn five_magnitudes_is_a_factor_of_one_hundred() {
        let bright = signal_e_per_s(10.0, AREA, QE, THRU);
        let faint = signal_e_per_s(15.0, AREA, QE, THRU);
        assert!(close(bright / faint, 100.0, 0.01));
    }

    #[test]
    fn signal_coefficient_is_the_magnitude_zero_signal() {
        // The coefficient is what a magnitude-zero target would deposit over
        // the whole exposure, so dividing the two must give 10^(-0.4 m).
        let k = signal_coefficient(AREA, QE, THRU, 30.0);
        let s = signal_e_per_s(11.5914, AREA, QE, THRU) * 30.0;
        assert!(close(s / k, 10f64.powf(-0.4 * 11.5914), 1e-12));
    }

    /// The Moon's rate against the stars: 1,296,000" per sidereal month.
    const LUNAR_RATE: f64 = 1_296_000.0 / (27.321_661 * 86_400.0);

    #[test]
    fn lunar_rate_is_half_an_arcsecond_per_second() {
        assert!(close(LUNAR_RATE, 0.549_017, 0.000_01));
    }

    #[test]
    fn trail_limited_exposure_cislunar() {
        // 2.5" of seeing at 0.549"/s gives 4.554 s before the trail exceeds
        // one seeing disk.
        assert!(close(trail_limited_exposure_s(2.5, LUNAR_RATE), 4.5536, 0.001));
    }

    #[test]
    fn trail_limited_exposure_caps_a_stationary_target() {
        // A rate-tracked or stared target does not trail, so nothing bounds
        // the exposure except the cap.
        assert!(close(trail_limited_exposure_s(2.5, 0.0), 30.0, 1e-12));
    }

    #[test]
    fn trail_limited_exposure_caps_a_very_slow_target() {
        // 0.001"/s would allow a 2500 s exposure; the cap must still bind.
        assert!(close(trail_limited_exposure_s(2.5, 0.001), 30.0, 1e-12));
    }

    #[test]
    fn trail_limited_exposure_rejects_a_negative_rate() {
        // Guards against a sign slip in a caller's rate difference.
        assert!(close(trail_limited_exposure_s(2.5, -1.0), 30.0, 1e-12));
    }

    #[test]
    fn trail_equals_seeing_at_the_trail_limited_exposure() {
        let t = trail_limited_exposure_s(2.5, LUNAR_RATE);
        assert!(close(trail_arcsec(LUNAR_RATE, t), 2.5, 1e-9));
    }

    #[test]
    fn footprint_cislunar() {
        // 2.5" across, 5.0" long at 0.7386"/px. Equivalently
        // seeing * (seeing + trail) / scale^2 = 12.5 / 0.545530 = 22.914 px.
        assert!(close(footprint_px(2.5, 2.5, SCALE), 22.9135, 0.005));
    }

    #[test]
    fn footprint_untrailed_is_the_seeing_disk_squared() {
        let across = 2.5 / SCALE;
        assert!(close(footprint_px(2.5, 0.0, SCALE), across * across, 1e-9));
    }

    #[test]
    fn footprint_grows_with_trail() {
        let a = footprint_px(2.5, 0.0, SCALE);
        let b = footprint_px(2.5, 5.0, SCALE);
        let c = footprint_px(2.5, 50.0, SCALE);
        assert!(a < b && b < c);
    }
}
