//! Physical constants shared by every module, and by `scope-eval`.

/// Earth's gravitational parameter, km^3/s^2 (WGS-84 / EGM96).
pub const MU_EARTH: f64 = 398_600.4418;
/// Earth's equatorial radius, km (WGS-84).
pub const EARTH_RADIUS_KM: f64 = 6_378.137;
/// Earth's flattening (WGS-84).
pub const EARTH_FLATTENING: f64 = 1.0 / 298.257_223_563;
/// Earth's second zonal harmonic, dimensionless (EGM96).
pub const J2: f64 = 1.082_626_68e-3;
/// Earth's rotation rate relative to the stars, rad/s (IERS).
pub const EARTH_ROTATION_RAD_S: f64 = 7.292_115_146_706_979e-5;
/// Sun's radius, km.
pub const SUN_RADIUS_KM: f64 = 696_000.0;
/// Astronomical unit, km.
pub const AU_KM: f64 = 149_597_870.7;
/// Seconds in one day.
pub const SECONDS_PER_DAY: f64 = 86_400.0;
/// Julian date of the J2000.0 epoch (2000-01-01 12:00).
pub const J2000_JD: f64 = 2_451_545.0;
/// Sun elevation below which the site counts as dark, degrees (nautical twilight).
pub const DARK_SUN_ELEVATION_DEG: f64 = -12.0;
/// Longest pass-search window `find_passes` accepts, seconds (30 days).
pub const MAX_SEARCH_WINDOW_S: f64 = 30.0 * SECONDS_PER_DAY;
