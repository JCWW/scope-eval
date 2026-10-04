//! Photometry: how bright a target is, and whether it can be detected.
//!
//! Every function here is pure (numbers in, number out) so it can be unit
//! tested without the interactive front end, mirroring the calculation
//! section of `checks.rs`. The judgments live in `regimes.rs`.
//!
//! Formulas and worked examples are in README.md.

use std::f64::consts::PI;

use crate::constants::{M_PER_KM, PHOTONS_M2_S_MAG0, SUN_APPARENT_MAG};

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
}
