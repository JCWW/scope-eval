//! Every named constant in the project: physical constants, unit
//! conversions, default assumptions, and the judgment thresholds used by
//! the checks in `checks.rs` and `regimes.rs`.

use std::f64::consts::PI;

// ---------------------------------------------------------------------------
// Physical constants
// ---------------------------------------------------------------------------

/// Arcseconds in one radian (180 / pi * 3600).
pub const ARCSEC_PER_RADIAN: f64 = 206_264.806;
/// Degrees in one radian.
pub const DEG_PER_RADIAN: f64 = 180.0 / PI;
/// Apparent drift of the stars past an Earth-fixed (GEO) object, arcsec per second.
/// One full turn (1,296,000") per sidereal day (86,164.09 s).
pub const SIDEREAL_RATE_ARCSEC_PER_S: f64 = 1_296_000.0 / 86_164.0905;
/// For a round Gaussian blur, FWHM = 2*sqrt(ln 2) * (RMS radius) = 1.665 * RMS radius.
pub const FWHM_PER_RMS_RADIUS: f64 = 1.665_109;
/// For a Gaussian profile, FWHM = 2*sqrt(2 ln 2) * sigma = 2.355 * sigma.
pub const FWHM_PER_SIGMA: f64 = 2.354_820;
/// FWHM of the Airy core of an unobstructed circular aperture, in units of
/// wavelength / aperture. A central obstruction narrows the core slightly and
/// moves light into the rings; see docs/17-point-spread-function.md.
pub const AIRY_FWHM_PER_LAMBDA_OVER_D: f64 = 1.029;
/// RMS width of a square pixel's response, as a fraction of the pixel pitch:
/// a uniform box of width p has standard deviation p / sqrt(12).
pub const PIXEL_BOX_SIGMA_PER_PITCH: f64 = 0.288_675;
/// MTF of an ideal square pixel at the Nyquist frequency: sinc(1/2) = 2 / pi.
/// A measured detector MTF at Nyquist below this means charge diffusion or
/// crosstalk blurs the image further.
pub const PIXEL_BOX_MTF_AT_NYQUIST: f64 = 2.0 / PI;
/// Earth's gravitational parameter (km^3/s^2) and equatorial radius (km),
/// shared with the `orbit-prop` library so the two can never disagree.
pub use orbit_prop::constants::{EARTH_RADIUS_KM, MU_EARTH};
/// One full circle in arcseconds.
pub const ARCSEC_PER_CIRCLE: f64 = 1_296_000.0;
/// Peak of the second derivative of `atan(v t / h)`, in units of `(v/h)^2`.
///
/// An overhead pass has `theta(t) = atan(v t / h)`, so with `u = v t / h`,
/// `theta'' = -2 (v/h)^2 u / (1 + u^2)^2`. That peaks at `u = 1/sqrt(3)`,
/// giving `(2/sqrt(3)) / (4/3)^2 = 3 sqrt(3) / 8`. A derived constant, not a
/// tuned threshold. See docs/09-mount-dynamics.md.
pub const PEAK_ACCEL_COEFF: f64 = 0.649_519_052_838_329;
/// Apparent V magnitude of the Sun.
pub const SUN_APPARENT_MAG: f64 = -26.74;
/// Photons per square metre per second from a magnitude-zero source in V band.
///
/// From the V-band zero point 3.64e-23 W/m^2/Hz over a 550 nm band of width
/// 89 nm (8.82e13 Hz), giving 3.21e-9 W/m^2, divided by the 3.61e-19 J energy
/// of a 550 nm photon.
pub const PHOTONS_M2_S_MAG0: f64 = 8.9e9;

// ---------------------------------------------------------------------------
// Unit conversions
// ---------------------------------------------------------------------------

/// Arcseconds in one degree.
pub const ARCSEC_PER_DEGREE: f64 = 3600.0;
/// Milliarcseconds in one arcsecond.
pub const MAS_PER_ARCSEC: f64 = 1000.0;
/// Arcminutes in one degree.
pub const ARCMIN_PER_DEGREE: f64 = 60.0;
/// Micrometers in one millimeter.
pub const UM_PER_MM: f64 = 1000.0;
/// Millimeters in one meter.
pub const MM_PER_M: f64 = 1000.0;
/// Seconds in one microsecond.
pub const S_PER_US: f64 = 1e-6;
/// Milliseconds in one second.
pub const MS_PER_S: f64 = 1000.0;
/// Kilograms in one pound.
pub const KG_PER_LB: f64 = 0.4536;
/// Metres in one kilometre.
pub const M_PER_KM: f64 = 1000.0;

// ---------------------------------------------------------------------------
// Default assumptions, used when a spec sheet doesn't say
// ---------------------------------------------------------------------------

/// Mount pointing error assumed when the mount's own figure was not entered, arcsec RMS.
pub const DEFAULT_POINTING_RMS_ARCSEC: f64 = 60.0;
/// Typical atmospheric seeing offered as the interactive menu's default, arcsec FWHM.
pub const DEFAULT_SEEING_ARCSEC: f64 = 2.5;
/// Pass prediction: default minimum elevation for a usable pass, degrees.
/// Below about 10 deg, extinction, seeing and horizon obstructions dominate.
pub const DEFAULT_MIN_ELEVATION_DEG: f64 = 10.0;
/// Pass prediction: longest search window, hours (the library's 30-day limit).
pub const MAX_PASS_SEARCH_HOURS: f64 = 720.0;
/// Pass prediction: TLE age beyond which pass times may be off by minutes, days.
pub const STALE_TLE_DAYS: f64 = 14.0;
/// Reference wavelength used for the focus calculation unless overridden, micrometers.
pub const DEFAULT_WAVELENGTH_UM: f64 = 0.55;
/// Typical PC clock + USB latency, ms. GPS hardware timestamping is sub-millisecond.
pub const DEFAULT_TIMESTAMP_MS: f64 = 20.0;
/// Illustrative GPS hardware timestamp accuracy used by the demo, ms.
pub const GPS_TIMESTAMP_MS: f64 = 0.1;
/// Demo-only assumption: a modeled mount points to about this RMS accuracy, arcsec.
pub const DEMO_ASSUMED_POINTING_RMS_ARCSEC: f64 = 30.0;
/// Illustrative timing-error interval used to show its positional effect, seconds (= 10 ms).
pub const TIMING_ERROR_EXAMPLE_S: f64 = 0.010;
/// Cross-sectional area of the representative target, m^2.
pub const REFERENCE_TARGET_CROSS_SECTION_M2: f64 = 10.0;
/// Albedo of the representative target.
pub const REFERENCE_TARGET_ALBEDO: f64 = 0.2;
/// Phase factor assumed for the representative target: full phase, phi = 0.
pub const DEFAULT_PHASE_FACTOR: f64 = 1.0;
/// Peak quantum efficiency assumed when not entered: generic back-illuminated CMOS.
pub const DEFAULT_QE: f64 = 0.80;
/// Optical throughput assumed when not entered: generic coated two-mirror train.
pub const DEFAULT_THROUGHPUT: f64 = 0.85;
/// Sky background assumed when not entered, V mag per square arcsec: rural site.
pub const DEFAULT_SKY_MAG_ARCSEC2: f64 = 21.0;
/// Read noise assumed when not entered, electrons RMS: generic CMOS.
pub const DEFAULT_READ_NOISE_E: f64 = 3.0;
/// Full-well capacity assumed when not entered, electrons. Deliberately at the
/// low end of modern CMOS (about 15k to 100k e- depending on gain mode), so an
/// unknown well flags saturation early rather than late.
pub const DEFAULT_FULL_WELL_E: f64 = 20_000.0;
/// Longest exposure the tool will derive, seconds. Caps the stationary-target case.
pub const MAX_EXPOSURE_S: f64 = 30.0;
/// Acquisition slew distance assumed by the slew-and-settle check, degrees.
pub const DEFAULT_SLEW_DISTANCE_DEG: f64 = 90.0;
/// Settle time assumed when not entered, seconds.
pub const DEFAULT_SETTLE_TIME_S: f64 = 2.0;

// ---------------------------------------------------------------------------
// Judgment thresholds. These are engineering rules of thumb, not physics.
// Change them here to tune the tool to your program's standards.
// ---------------------------------------------------------------------------

/// Thresholds for the eight checks in `checks.rs`.
pub mod checks_limits {
    /// Check 1: image circle may be this fraction of the sensor diagonal before failing.
    pub const FIT_WARN_FRACTION: f64 = 0.90;
    /// Check 1: image circle this much larger than the diagonal gets a "headroom" note.
    pub const FIT_HEADROOM_FACTOR: f64 = 1.15;
    /// Check 2: ideal number of pixels across a star's FWHM.
    pub const SAMPLING_TARGET: f64 = 2.0;
    pub const SAMPLING_UNDER_FAIL: f64 = 1.0;
    pub const SAMPLING_GOOD_MIN: f64 = 1.5;
    pub const SAMPLING_GOOD_MAX: f64 = 2.5;
    pub const SAMPLING_BIN2_MAX: f64 = 4.0;
    pub const SAMPLING_OVER_FAIL: f64 = 6.0;
    /// Largest square bin factor the tool will recommend.
    pub const MAX_BIN: u32 = 4;
    /// Check 3: effective pixel within this ratio of ideal counts as a match.
    pub const PIXEL_MATCH_LOW: f64 = 0.75;
    pub const PIXEL_MATCH_HIGH: f64 = 1.33;
    /// Check 4: allowed growth of the star image caused by the optics.
    pub const OPTICS_PASS_GROWTH: f64 = 0.15;
    pub const OPTICS_WARN_GROWTH: f64 = 0.35;
    /// Check 7: critical focus zone half-widths, micrometers.
    pub const CFZ_FORGIVING_UM: f64 = 40.0;
    pub const CFZ_DEMANDING_UM: f64 = 15.0;
    /// Check 8: payload as a fraction of mount capacity.
    pub const PAYLOAD_PASS_FRACTION: f64 = 0.70;
    pub const PAYLOAD_WARN_FRACTION: f64 = 0.90;
}

/// Thresholds for the orbital-regime checks in `regimes.rs`.
pub mod regimes_limits {
    /// Telescope: (half the short side of the field) / (acquisition uncertainty).
    pub const ACQ_PASS_MARGIN: f64 = 2.0;
    pub const ACQ_WARN_MARGIN: f64 = 1.0;
    /// Camera: allowed timing error, as a fraction of one binned pixel of motion.
    pub const TIMING_PIXEL_FRACTION: f64 = 0.25;
    /// Camera: timing error up to this multiple of the requirement is a WARN, beyond it FAIL.
    pub const TIMING_WARN_MULTIPLE: f64 = 4.0;
    /// Camera: rolling-shutter skew below this (binned pixels) needs no correction.
    pub const SKEW_NEGLIGIBLE_PX: f64 = 0.25;
    /// Camera: skew larger than this fraction of the frame height distorts frame geometry.
    pub const SKEW_FAIL_FRAME_FRACTION: f64 = 0.10;
    /// Mount: max axis rate / required rate.
    pub const RATE_PASS_HEADROOM: f64 = 3.0;
    pub const RATE_WARN_HEADROOM: f64 = 1.0;
    /// Mount: required rates above this (deg/s) need a known slew rate to judge.
    pub const RATE_MATTERS_DEG_S: f64 = 0.1;
    /// Mount (alt-az): highest pass elevation that can be followed without losing the target.
    pub const KEYHOLE_PASS_ELEV_DEG: f64 = 85.0;
    pub const KEYHOLE_WARN_ELEV_DEG: f64 = 70.0;
    /// System: SNR at which a target counts as detected.
    pub const DETECT_SNR_THRESHOLD: f64 = 5.0;
    /// System: SNR for comfortable detection.
    pub const SNR_PASS: f64 = 10.0;
    /// System: above this SNR, detection is simply not what limits the regime.
    pub const SNR_TRIVIAL: f64 = 100.0;
    /// System: a peak pixel above this fraction of full well is in the
    /// nonlinear range near saturation, so centroids and photometry degrade.
    pub const SATURATION_WARN_FRACTION: f64 = 0.8;
    /// System: if even an exposure this short saturates (seconds), shortening
    /// the exposure is no longer a practical fix.
    pub const MIN_PRACTICAL_EXPOSURE_S: f64 = 0.001;
    /// Mount: required accelerations above this (deg/s^2) need a known rating to judge.
    ///
    /// Set between LEO (0.008627 deg/s^2) and MEO (1.37e-6 deg/s^2) so that LEO
    /// alone trips the Warn branch. A value of 0.01 would sit above LEO's own
    /// requirement and the branch would be unreachable.
    pub const ACCEL_MATTERS_DEG_S2: f64 = 0.005;
    /// Mount: max axis acceleration / required acceleration.
    pub const ACCEL_PASS_HEADROOM: f64 = 3.0;
    pub const ACCEL_WARN_HEADROOM: f64 = 1.0;
    /// Mount: slew + settle as a fraction of the regime's usable window.
    pub const SLEW_PASS_WINDOW_FRACTION: f64 = 0.10;
    pub const SLEW_WARN_WINDOW_FRACTION: f64 = 0.25;
}

/// Plausible ranges for hand-entered inputs.
///
/// `presets.yaml` is edited by hand and nothing else validates it. A value
/// outside these ranges is treated as not entered rather than trusted, so a
/// typo degrades the report instead of corrupting it. See `model::plausible`.
pub mod plausible_ranges {
    /// Quantum efficiency and optical throughput are fractions of 1.
    pub const QE_MIN: f64 = 0.01;
    pub const QE_MAX: f64 = 1.0;
    pub const THROUGHPUT_MIN: f64 = 0.01;
    pub const THROUGHPUT_MAX: f64 = 1.0;
    /// Sky surface brightness, V mag per square arcsec. Below 15 is daylight,
    /// above 24 is darker than any real sky.
    pub const SKY_MAG_MIN: f64 = 15.0;
    pub const SKY_MAG_MAX: f64 = 24.0;
    /// Read noise, electrons RMS.
    pub const READ_NOISE_MIN: f64 = 0.1;
    pub const READ_NOISE_MAX: f64 = 100.0;
    /// Full-well capacity, electrons.
    pub const FULL_WELL_MIN: f64 = 1_000.0;
    pub const FULL_WELL_MAX: f64 = 2_000_000.0;
    /// Detector MTF at Nyquist, a fraction of 1.
    pub const MTF_NYQUIST_MIN: f64 = 0.01;
    pub const MTF_NYQUIST_MAX: f64 = 1.0;
    /// Mount axis acceleration, deg/s^2.
    pub const ACCEL_MIN_DEG_S2: f64 = 1e-4;
    pub const ACCEL_MAX_DEG_S2: f64 = 1000.0;
    /// Mount axis rate, deg/s.
    pub const SLEW_RATE_MIN_DEG_S: f64 = 1e-3;
    pub const SLEW_RATE_MAX_DEG_S: f64 = 1000.0;
    /// Settle time, seconds. Zero is allowed: it means no settle.
    pub const SETTLE_MIN_S: f64 = 0.0;
    pub const SETTLE_MAX_S: f64 = 600.0;
}
