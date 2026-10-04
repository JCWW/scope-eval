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
/// Earth's gravitational parameter, km^3/s^2.
pub const MU_EARTH: f64 = 398_600.4418;
/// Earth's equatorial radius, km.
pub const EARTH_RADIUS_KM: f64 = 6_378.137;
/// One full circle in arcseconds.
pub const ARCSEC_PER_CIRCLE: f64 = 1_296_000.0;

// ---------------------------------------------------------------------------
// Unit conversions
// ---------------------------------------------------------------------------

/// Arcseconds in one degree.
pub const ARCSEC_PER_DEGREE: f64 = 3600.0;
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

// ---------------------------------------------------------------------------
// Default assumptions, used when a spec sheet doesn't say
// ---------------------------------------------------------------------------

/// Mount pointing error assumed when the mount's own figure was not entered, arcsec RMS.
pub const DEFAULT_POINTING_RMS_ARCSEC: f64 = 60.0;
/// Typical atmospheric seeing offered as the interactive menu's default, arcsec FWHM.
pub const DEFAULT_SEEING_ARCSEC: f64 = 2.5;
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
}
