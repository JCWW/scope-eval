//! Photometry: how bright a target is, and whether it can be detected.
//!
//! This module resolves entered photometric inputs and tracks which defaults
//! were assumed. Detection equations live in `calculations::detection`; the
//! judgments live in `regimes.rs`.
//!
//! Formulas and worked examples are in docs/10-target-brightness-and-detection.md.

use crate::constants::plausible_ranges as ranges;
use crate::constants::{
    DEFAULT_QE, DEFAULT_READ_NOISE_E, DEFAULT_SKY_MAG_ARCSEC2, DEFAULT_THROUGHPUT,
};
use crate::model::{plausible, Camera, Site, Telescope};

/// Photometric inputs with defaults substituted, and the names of whatever
/// was assumed rather than entered.
///
/// `assumed` is what stops the detection check reporting PASS on numbers the
/// user never supplied. A value present but outside its plausible range is
/// treated as absent; see [`crate::model::plausible`].
#[derive(Debug, Clone, PartialEq)]
pub struct Photometry {
    pub qe: f64,
    pub throughput: f64,
    pub sky_mag_arcsec2: f64,
    pub read_noise_e: f64,
    /// Human-readable names of the inputs that fell back to a default.
    /// Ordered as resolved, so the report reads consistently.
    pub assumed: Vec<&'static str>,
}

impl Photometry {
    pub fn resolve(t: &Telescope, c: &Camera, site: &Site) -> Self {
        let mut assumed: Vec<&'static str> = Vec::new();
        let mut take = |value: Option<f64>, lo: f64, hi: f64, default: f64, name: &'static str| {
            match plausible(value, lo, hi) {
                Some(v) => v,
                None => {
                    assumed.push(name);
                    default
                }
            }
        };
        let qe = take(c.qe, ranges::QE_MIN, ranges::QE_MAX, DEFAULT_QE, "quantum efficiency");
        let throughput = take(
            t.throughput,
            ranges::THROUGHPUT_MIN,
            ranges::THROUGHPUT_MAX,
            DEFAULT_THROUGHPUT,
            "throughput",
        );
        let sky_mag_arcsec2 = take(
            site.sky_mag_arcsec2,
            ranges::SKY_MAG_MIN,
            ranges::SKY_MAG_MAX,
            DEFAULT_SKY_MAG_ARCSEC2,
            "sky brightness",
        );
        let read_noise_e = take(
            c.read_noise_e,
            ranges::READ_NOISE_MIN,
            ranges::READ_NOISE_MAX,
            DEFAULT_READ_NOISE_E,
            "read noise",
        );
        Photometry { qe, throughput, sky_mag_arcsec2, read_noise_e, assumed }
    }

    /// True when any input fell back to a default. The detection check must
    /// not report PASS when this holds.
    pub fn any_assumed(&self) -> bool {
        !self.assumed.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calculations::detection::DetectionCalculator as Calculator;

    fn derived_target_mag(area: f64, albedo: f64, range_km: f64, phase: f64) -> f64 {
        Calculator::derived_target_mag(area, albedo, range_km, phase)
    }

    fn signal_e_per_s(mag: f64, area: f64, qe: f64, throughput: f64) -> f64 {
        Calculator::signal_e_per_s(mag, area, qe, throughput)
    }

    fn sky_e_per_px_s(mag: f64, scale: f64, area: f64, qe: f64, throughput: f64) -> f64 {
        Calculator::sky_e_per_px_s(mag, scale, area, qe, throughput)
    }

    fn signal_coefficient(area: f64, qe: f64, throughput: f64, exposure: f64) -> f64 {
        Calculator::signal_coefficient(area, qe, throughput, exposure)
    }

    fn trail_arcsec(rate: f64, exposure: f64) -> f64 {
        Calculator::trail_arcsec(rate, exposure)
    }

    fn trail_limited_exposure_s(seeing: f64, rate: f64) -> f64 {
        Calculator::trail_limited_exposure_s(seeing, rate)
    }

    fn footprint_px(seeing: f64, trail: f64, scale: f64) -> f64 {
        Calculator::footprint_px(seeing, trail, scale)
    }

    fn snr(signal: f64, sky: f64, read_noise: f64, pixels: f64) -> f64 {
        Calculator::snr(signal, sky, read_noise, pixels)
    }

    fn limiting_mag(threshold: f64, noise: f64, signal: f64) -> f64 {
        Calculator::limiting_mag(threshold, noise, signal)
    }

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

    // The worked configuration from the docs: DeltaRho 350 (0.0660 m^2
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

    #[test]
    fn geo_snr_deltarho350() {
        // 30 s on a 11.59-mag target: 276,671 e- of signal against 298 e- of
        // sky over an 11.46 px footprint and a 103 e- read term.
        let signal = signal_e_per_s(11.5914, AREA, QE, THRU) * 30.0;
        let n_px = footprint_px(2.5, 0.0, SCALE);
        let sky = sky_e_per_px_s(21.0, SCALE, AREA, QE, THRU) * 30.0 * n_px;
        assert!(close(snr(signal, sky, 3.0, n_px), 525.6, 1.0));
    }

    #[test]
    fn cislunar_snr_deltarho350() {
        let t = trail_limited_exposure_s(2.5, LUNAR_RATE);
        let n_px = footprint_px(2.5, trail_arcsec(LUNAR_RATE, t), SCALE);
        let signal = signal_e_per_s(16.6739, AREA, QE, THRU) * t;
        let sky = sky_e_per_px_s(21.0, SCALE, AREA, QE, THRU) * t * n_px;
        assert!(close(snr(signal, sky, 3.0, n_px), 14.86, 0.05));
    }

    #[test]
    fn cislunar_limiting_mag() {
        let t = trail_limited_exposure_s(2.5, LUNAR_RATE);
        let n_px = footprint_px(2.5, trail_arcsec(LUNAR_RATE, t), SCALE);
        let sky = sky_e_per_px_s(21.0, SCALE, AREA, QE, THRU) * t * n_px;
        let noise = sky + 9.0 * n_px;
        let k = signal_coefficient(AREA, QE, THRU, t);
        assert!(close(limiting_mag(5.0, noise, k), 18.155, 0.01));
    }

    #[test]
    fn limiting_mag_round_trip() {
        // A target at the limiting magnitude must come back out at exactly
        // the threshold. This is the check that the quadratic inversion is
        // the true inverse of snr(), not an approximation of it.
        let t = 10.0;
        let n_px = footprint_px(2.5, 0.0, SCALE);
        let sky = sky_e_per_px_s(21.0, SCALE, AREA, QE, THRU) * t * n_px;
        let noise = sky + 9.0 * n_px;
        let k = signal_coefficient(AREA, QE, THRU, t);
        let m = limiting_mag(5.0, noise, k);
        let signal = signal_e_per_s(m, AREA, QE, THRU) * t;
        assert!(close(snr(signal, sky, 3.0, n_px), 5.0, 1e-6));
    }

    #[test]
    fn limiting_mag_round_trip_at_several_thresholds() {
        let t = 10.0;
        let n_px = footprint_px(2.5, 0.0, SCALE);
        let sky = sky_e_per_px_s(21.0, SCALE, AREA, QE, THRU) * t * n_px;
        let noise = sky + 9.0 * n_px;
        let k = signal_coefficient(AREA, QE, THRU, t);
        for threshold in [3.0, 5.0, 10.0, 50.0] {
            let m = limiting_mag(threshold, noise, k);
            let signal = signal_e_per_s(m, AREA, QE, THRU) * t;
            assert!(
                close(snr(signal, sky, 3.0, n_px), threshold, 1e-6),
                "round trip failed at threshold {threshold}"
            );
        }
    }

    #[test]
    fn limiting_mag_survives_a_noiseless_detector() {
        // Zero sky and zero read noise is the signal-limited case: the
        // inversion must not divide by zero. S = T^2 at N = 0.
        let k = signal_coefficient(AREA, QE, THRU, 10.0);
        let m = limiting_mag(5.0, 0.0, k);
        assert!(m.is_finite());
        assert!(close(10f64.powf(-0.4 * m) * k, 25.0, 1e-6));
    }

    #[test]
    fn snr_of_no_signal_is_zero() {
        assert!(close(snr(0.0, 0.0, 0.0, 0.0), 0.0, 1e-12));
    }

    #[test]
    fn a_longer_exposure_detects_a_fainter_target() {
        let faint = |t: f64| {
            let n_px = footprint_px(2.5, 0.0, SCALE);
            let sky = sky_e_per_px_s(21.0, SCALE, AREA, QE, THRU) * t * n_px;
            limiting_mag(5.0, sky + 9.0 * n_px, signal_coefficient(AREA, QE, THRU, t))
        };
        assert!(faint(60.0) > faint(10.0));
    }

    // Camera, Site and Telescope arrive via `use super::*`; only these two
    // need importing here (pre-flight ruling).
    use crate::model::{Obstruction, Shutter};

    fn bare_telescope() -> Telescope {
        Telescope {
            name: "test".into(),
            aperture_mm: 350.0,
            focal_length_mm: 1050.0,
            obstruction: Obstruction::ByDiameter(0.56),
            image_circle_mm: Some(60.0),
            back_focus_mm: None,
            weight_lb: None,
            throughput: None,
            spot: None,
            source: "test".into(),
        }
    }

    fn bare_camera() -> Camera {
        Camera {
            name: "test".into(),
            pixel_um: 3.76,
            width_px: 9576,
            height_px: 6388,
            read_noise_e: None,
            qe: None,
            shutter: Shutter::Global,
            weight_lb: None,
            source: "test".into(),
        }
    }

    fn bare_site() -> Site {
        Site { seeing_arcsec: 2.5, wavelength_um: 0.55, sky_mag_arcsec2: None, location: None }
    }

    #[test]
    fn resolve_names_every_assumed_input() {
        let p = Photometry::resolve(&bare_telescope(), &bare_camera(), &bare_site());
        assert!(p.any_assumed());
        assert_eq!(p.assumed.len(), 4);
        assert!(close(p.qe, 0.80, 1e-12));
        assert!(close(p.throughput, 0.85, 1e-12));
        assert!(close(p.sky_mag_arcsec2, 21.0, 1e-12));
        assert!(close(p.read_noise_e, 3.0, 1e-12));
    }

    #[test]
    fn resolve_names_nothing_when_everything_is_entered() {
        let mut t = bare_telescope();
        let mut c = bare_camera();
        let mut s = bare_site();
        t.throughput = Some(0.9);
        c.qe = Some(0.7);
        c.read_noise_e = Some(1.5);
        s.sky_mag_arcsec2 = Some(21.9);
        let p = Photometry::resolve(&t, &c, &s);
        assert!(!p.any_assumed());
        assert!(p.assumed.is_empty());
        assert!(close(p.qe, 0.7, 1e-12));
        assert!(close(p.throughput, 0.9, 1e-12));
        assert!(close(p.sky_mag_arcsec2, 21.9, 1e-12));
        assert!(close(p.read_noise_e, 1.5, 1e-12));
    }

    #[test]
    fn resolve_rejects_a_quantum_efficiency_above_one() {
        // Review Focus 1: unphysical, and it would silently shift every
        // magnitude the tool reports.
        let mut c = bare_camera();
        c.qe = Some(1.5);
        let p = Photometry::resolve(&bare_telescope(), &c, &bare_site());
        assert!(close(p.qe, 0.80, 1e-12));
        assert!(p.assumed.contains(&"quantum efficiency"));
    }

    #[test]
    fn resolve_rejects_a_zero_or_negative_quantum_efficiency() {
        for bad in [0.0, -0.2] {
            let mut c = bare_camera();
            c.qe = Some(bad);
            let p = Photometry::resolve(&bare_telescope(), &c, &bare_site());
            assert!(close(p.qe, 0.80, 1e-12), "qe {bad} was trusted");
            assert!(p.assumed.contains(&"quantum efficiency"));
        }
    }

    #[test]
    fn resolve_rejects_nan() {
        // Review Focus 3: `.nan` is legal YAML, and NaN compares false
        // against everything, so grading would fall through to FAIL.
        let mut c = bare_camera();
        c.qe = Some(f64::NAN);
        let p = Photometry::resolve(&bare_telescope(), &c, &bare_site());
        assert!(p.qe.is_finite());
        assert!(close(p.qe, 0.80, 1e-12));
        assert!(p.assumed.contains(&"quantum efficiency"));
    }

    #[test]
    fn resolve_rejects_sky_brightness_at_the_wrong_scale() {
        // Review Focus 5: 2.1 for 21.0 is a plausible typo, and 2.1
        // mag/arcsec^2 is brighter than daylight. Trusting it would swamp
        // the signal and FAIL every regime with no hint of the real cause.
        let mut s = bare_site();
        s.sky_mag_arcsec2 = Some(2.1);
        let p = Photometry::resolve(&bare_telescope(), &bare_camera(), &s);
        assert!(close(p.sky_mag_arcsec2, 21.0, 1e-12));
        assert!(p.assumed.contains(&"sky brightness"));
    }

    #[test]
    fn resolve_rejects_an_impossibly_dark_sky() {
        let mut s = bare_site();
        s.sky_mag_arcsec2 = Some(30.0);
        let p = Photometry::resolve(&bare_telescope(), &bare_camera(), &s);
        assert!(close(p.sky_mag_arcsec2, 21.0, 1e-12));
        assert!(p.assumed.contains(&"sky brightness"));
    }

    #[test]
    fn resolve_rejects_one_bad_input_without_discarding_the_others() {
        let mut t = bare_telescope();
        let mut c = bare_camera();
        t.throughput = Some(0.9);
        c.qe = Some(99.0);
        c.read_noise_e = Some(1.5);
        let p = Photometry::resolve(&t, &c, &bare_site());
        assert!(close(p.throughput, 0.9, 1e-12));
        assert!(close(p.read_noise_e, 1.5, 1e-12));
        assert!(close(p.qe, 0.80, 1e-12));
        assert_eq!(p.assumed, vec!["quantum efficiency", "sky brightness"]);
    }
}
