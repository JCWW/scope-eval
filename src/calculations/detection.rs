//! Target brightness, detector signal, noise, and trailing calculations.

use crate::constants::{MAX_EXPOSURE_S, M_PER_KM, PHOTONS_M2_S_MAG0, SUN_APPARENT_MAG};
use std::f64::consts::PI;

/// Independent calculations for target detection and exposure planning.
pub struct DetectionCalculator;

impl DetectionCalculator {
    /// Apparent magnitude of a diffuse (Lambertian) target.
    pub fn derived_target_mag(
        cross_section_m2: f64,
        albedo: f64,
        range_km: f64,
        phase: f64,
    ) -> f64 {
        let d_m = range_km * M_PER_KM;
        SUN_APPARENT_MAG - 2.5 * (albedo * cross_section_m2 * phase / (PI * d_m * d_m)).log10()
    }

    /// Electrons per second from a point source of the given magnitude.
    pub fn signal_e_per_s(mag: f64, eff_area_m2: f64, qe: f64, throughput: f64) -> f64 {
        PHOTONS_M2_S_MAG0 * 10f64.powf(-0.4 * mag) * eff_area_m2 * qe * throughput
    }

    /// Electrons per second per pixel from sky brightness in magnitudes/arcsec^2.
    pub fn sky_e_per_px_s(
        sky_mag_arcsec2: f64,
        plate_scale: f64,
        eff_area_m2: f64,
        qe: f64,
        throughput: f64,
    ) -> f64 {
        Self::signal_e_per_s(sky_mag_arcsec2, eff_area_m2, qe, throughput)
            * plate_scale
            * plate_scale
    }

    /// Signal from a magnitude-zero target over an exposure.
    pub fn signal_coefficient(eff_area_m2: f64, qe: f64, throughput: f64, exposure_s: f64) -> f64 {
        PHOTONS_M2_S_MAG0 * eff_area_m2 * qe * throughput * exposure_s
    }

    /// Target movement across the sensor during exposure, arcsec.
    pub fn trail_arcsec(residual_rate_arcsec_s: f64, exposure_s: f64) -> f64 {
        residual_rate_arcsec_s * exposure_s
    }

    /// Longest exposure keeping target trail within one seeing disk.
    pub fn trail_limited_exposure_s(seeing_arcsec: f64, residual_rate_arcsec_s: f64) -> f64 {
        if residual_rate_arcsec_s <= 0.0 {
            MAX_EXPOSURE_S
        } else {
            (seeing_arcsec / residual_rate_arcsec_s).min(MAX_EXPOSURE_S)
        }
    }

    /// Approximate pixel footprint of a seeing disk smeared along a trail.
    pub fn footprint_px(seeing_arcsec: f64, trail_arcsec: f64, plate_scale: f64) -> f64 {
        let across = seeing_arcsec / plate_scale;
        let along = (seeing_arcsec + trail_arcsec) / plate_scale;
        across * along
    }

    /// Combined shot, sky, and read-noise variance, electrons squared.
    pub fn noise_variance_e2(signal_e: f64, sky_e: f64, read_noise_e: f64, n_px: f64) -> f64 {
        signal_e + sky_e + read_noise_e * read_noise_e * n_px
    }

    /// SNR against source shot noise, sky noise, and detector read noise.
    pub fn snr(signal_e: f64, sky_e_total: f64, read_noise_e: f64, n_px: f64) -> f64 {
        let variance = Self::noise_variance_e2(signal_e, sky_e_total, read_noise_e, n_px);
        if variance <= 0.0 {
            0.0
        } else {
            signal_e / variance.sqrt()
        }
    }

    /// Faintest magnitude that reaches the requested SNR threshold.
    pub fn limiting_mag(threshold: f64, noise_variance_e2: f64, signal_coefficient: f64) -> f64 {
        let t2 = threshold * threshold;
        let s_min = (t2 + (t2 * t2 + 4.0 * t2 * noise_variance_e2).sqrt()) / 2.0;
        -2.5 * (s_min / signal_coefficient).log10()
    }
}

#[cfg(test)]
mod tests {
    use super::DetectionCalculator;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn target_magnitude_matches_worked_examples() {
        let geo = DetectionCalculator::derived_target_mag(10.0, 0.2, 37_000.0, 1.0);
        let cislunar = DetectionCalculator::derived_target_mag(10.0, 0.2, 384_400.0, 1.0);
        assert!(close(geo, 11.59, 0.01));
        assert!(close(cislunar, 16.67, 0.01));
    }

    #[test]
    fn source_and_sky_rates_match_worked_example() {
        let signal = DetectionCalculator::signal_e_per_s(11.5914, 0.0660, 0.80, 0.85);
        let sky = DetectionCalculator::sky_e_per_px_s(21.0, 0.7386, 0.0660, 0.80, 0.85);
        assert!(close(signal, 9222.4, 1.0));
        assert!(close(sky, 0.8674, 0.0005));
    }

    #[test]
    fn trail_and_exposure_limits() {
        let rate = 0.549_017;
        let exposure = DetectionCalculator::trail_limited_exposure_s(2.5, rate);
        assert!(close(exposure, 4.5536, 0.001));
        assert!(close(
            DetectionCalculator::trail_arcsec(rate, exposure),
            2.5,
            1e-9
        ));
        assert_eq!(
            DetectionCalculator::trail_limited_exposure_s(2.5, 0.0),
            30.0
        );
    }

    #[test]
    fn limiting_magnitude_inverts_the_snr_equation() {
        let exposure = 30.0;
        let scale = 0.7386;
        let pixels = DetectionCalculator::footprint_px(2.5, 0.0, scale);
        let sky = DetectionCalculator::sky_e_per_px_s(21.0, scale, 0.0660, 0.80, 0.85)
            * exposure
            * pixels;
        let coefficient = DetectionCalculator::signal_coefficient(0.0660, 0.80, 0.85, exposure);
        let noise = DetectionCalculator::noise_variance_e2(0.0, sky, 3.0, pixels);
        let limiting = DetectionCalculator::limiting_mag(5.0, noise, coefficient);
        let signal = DetectionCalculator::signal_e_per_s(limiting, 0.0660, 0.80, 0.85) * exposure;
        assert!(close(
            DetectionCalculator::snr(signal, sky, 3.0, pixels),
            5.0,
            1e-6
        ));
    }
}
