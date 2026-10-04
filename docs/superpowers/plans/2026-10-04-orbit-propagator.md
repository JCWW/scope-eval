# orbit-prop Propagator Library and Pass Prediction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a reusable satellite-propagation library (`crates/orbit-prop`: SGP4 + Keplerian/J2 behind one trait, observer geometry, lighting, pass finding) and a `scope-eval` menu item that predicts passes over the user's site and judges, pass by pass, whether each evaluated mount can follow.

**Architecture:** The repo becomes a Cargo workspace. The root package stays `scope-eval`, and the new library crate lives at `crates/orbit-prop` with `sgp4` as its only dependency. All downstream code depends on the `Propagator` trait. `scope-eval` gains an optional site location, a `passes_report.rs` module (judgment and printing only) and a prompt flow in `main.rs`. All math lives in the library.

**Tech Stack:** Rust 2021, MSRV 1.70; `sgp4` 2.3 (pure Rust, `default-features = false, features = ["alloc", "std"]`).

**Spec:** `docs/superpowers/specs/2026-10-04-orbit-propagator-design.md`

## Global Constraints

- MSRV `rust-version = "1.70"`, `edition = "2021"`, for both crates.
- `orbit-prop`'s only dependency is `sgp4 = { version = "2.3", default-features = false, features = ["alloc", "std"] }`. It builds with `--offline` from the local Cargo cache (verified: `sgp4-2.3.0`, `chrono-0.4.41` are cached).
- The library never panics on user-supplied input. Every failure is an `OrbitPropError`.
- Units are part of the field names: `_km`, `_km_s`, `_deg`, `_deg_s`, `_deg_s2`, `_m`, `_s`. Angles are degrees in public types and radians internally.
- `MU_EARTH = 398_600.4418` and `EARTH_RADIUS_KM = 6_378.137` are defined once, in `orbit_prop::constants`. `scope-eval` re-exports them.
- Assessment grade, about 0.01 deg. Every simplification carries a `TODO(astrometric)` comment, and the crate README has a "Known limitations and future work" section.
- `scope-eval` output is plain ASCII. `cargo run -- --demo` output must stay byte-for-byte identical.
- `cargo test` at the workspace root runs both crates (`default-members`).
- Each task ends with `cargo test` green.
- Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Spec refinements made while verifying this plan

Every code block in this plan was compiled and its tests run in a scratch copy of the repo before the plan was written. Doing that surfaced these refinements to the spec. Each one is small and keeps the spec's intent:

1. **SGP4 runs in AFSPC compatibility mode.** The `sgp4` crate's default mode misses Vallado's 00005 case by about 15 m at 360 min, which is more than the spec's 1 m tolerance. AFSPC mode matches to the millimetre, and it is the implementation catalog TLEs are fitted with.
2. **The SGP4 module is `sgp4_propagator.rs`, not `sgp4.rs`,** so it doesn't shadow the `sgp4` crate's name.
3. **`Pass` also has `peak_ha_accel_deg_s2` and `peak_dec_accel_deg_s2`.** The equatorial-mount acceleration judgment needs them.
4. **Dense pass sampling is 1 s, or `duration / 7200` for passes over two hours.** This bounds the cost of a day-long GEO pass, and the 0.1 s fine window applies to every pass.
5. **`KeplerJ2` velocity includes the J2 drift terms,** so it is the exact time derivative of position. Without them, analytic angular rates disagree with the path by about 3 m/s.
6. **Vallado Example 3-3 uses an altitude of 2187 m.** Its published site vector matches that altitude to 0.1 m.
7. **There is a private `vec3.rs` helper module** for small vector operations.
8. **Site altitude is validated to -500 ... 10 000 m.**
9. **There is a stale-TLE note.** The tool warns when the TLE epoch is more than 14 days from the search start (see Review Focus).
10. **The what-if orbit's epoch is asked in the orbit step,** where a blank answer resolves to the search start, which is asked afterwards. The spec's question order is kept.

## Review Focus

These are the inputs most likely to bite a real user that the spec implies but doesn't test. Each has a test in its owning task:

1. A TLE pasted from a Windows clipboard (CRLF line endings, trailing spaces) must parse: `windows_line_endings_from_a_paste_are_accepted` (Task 4).
2. A TLE whose epoch is weeks from the search start must produce a warning, not silently wrong pass times: `stale_tle_is_flagged_after_two_weeks` (Task 8).
3. A search that starts while the satellite is already up must report that pass, marked clipped, with the correct set time: `search_starting_mid_pass_is_clipped_at_the_start` (Task 7).
4. A site near a pole must not break az/el or pass finding: `polar_orbit_over_a_polar_site_passes_every_revolution` (Task 7).
5. A search start or what-if epoch before the element epoch must propagate backwards: `propagates_backwards_from_epoch` (Task 3).

## File structure

| File | Responsibility |
|---|---|
| `Cargo.toml` (modify) | Workspace with root `scope-eval` plus `crates/orbit-prop`; path dependency |
| `crates/orbit-prop/Cargo.toml` | Library manifest |
| `crates/orbit-prop/README.md` | Models, units, errors, accuracy, astrometric future work; doubles as crate docs/doctest |
| `crates/orbit-prop/src/lib.rs` | Module list and public re-exports |
| `.../constants.rs` | mu, J2, WGS-84, Earth rotation, Sun radius, AU, dark threshold, search limit |
| `.../error.rs` | `OrbitPropError` |
| `.../time.rs` | `Epoch` (split Julian date, UTC), ISO-8601, GMST |
| `.../vec3.rs` | Private 3-vector helpers |
| `.../site.rs` | `GroundSite` |
| `.../frames.rs` | TEME<->ECEF, geodetic<->ECEF, SEZ, az/el |
| `.../state.rs` | `StateVector` |
| `.../propagator.rs` | `Propagator` trait |
| `.../keplerian.rs` | `KeplerElements`, `KeplerJ2` |
| `.../tle.rs` | `Tle` parse + checksum |
| `.../sgp4_propagator.rs` | `Sgp4Propagator` |
| `.../observe.rs` | `observe` -> `Observation` |
| `.../sun_moon.rs` | Low-precision Sun and Moon |
| `.../illumination.rs` | Shadow, phase angle, Sun elevation |
| `.../passes.rs` | `find_passes` |
| `src/constants.rs` (modify) | Re-export shared constants; pass-prediction defaults |
| `src/model/site.rs` (modify) | `Site.location` |
| `src/passes_report.rs` (create) | `judge_mount_for_pass`, `stale_tle_note`, `print_passes` |
| `src/main.rs` (modify) | Menu item, site-location prompt, orbit/window prompts |
| `README.md` (modify) | Pass prediction section, dependencies, assumptions, code tree |

---

### Task 1: Workspace, crate skeleton, constants, errors and time

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/constants.rs:20-23` (`MU_EARTH`, `EARTH_RADIUS_KM`)
- Create: `crates/orbit-prop/Cargo.toml`, `crates/orbit-prop/src/{lib,constants,error,time}.rs`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `orbit_prop::constants::{MU_EARTH, EARTH_RADIUS_KM, EARTH_FLATTENING, J2, EARTH_ROTATION_RAD_S, SUN_RADIUS_KM, AU_KM, SECONDS_PER_DAY, J2000_JD, DARK_SUN_ELEVATION_DEG, MAX_SEARCH_WINDOW_S}` (all `f64`).
  - `OrbitPropError { TleFormat { line: u8, column: Option<usize>, message: String }, Sgp4(String), InvalidElements(String), InvalidSite(String), InvalidTime(String), NoConvergence(String) }`: `Debug, Clone, PartialEq, Display, Error`.
  - `Epoch` (`Copy, PartialEq, PartialOrd`): `from_utc(i32, u32, u32, u32, u32, f64) -> Result<Epoch, _>`, `from_year_day(i32, f64) -> Result<Epoch, _>`, `now() -> Epoch`, `parse_iso8601(&str) -> Result<Epoch, _>`, `add_seconds(&self, f64) -> Epoch`, `seconds_since(&self, &Epoch) -> f64`, `jd(&self) -> f64`, `days_since_j2000(&self) -> f64`, `gmst_rad(&self) -> f64`, `to_utc(&self) -> UtcParts`, and `Display` as `YYYY-MM-DDTHH:MM:SS.sssZ`.
  - `UtcParts { year: i32, month: u32, day: u32, hour: u32, minute: u32, second: f64 }`.

- [ ] **Step 1: Capture the current demo output**

Run: `cargo run -q -- --demo > target/demo-before.txt`
Expected: the file is written. It is compared against the new output at the end of this task.

- [ ] **Step 2: Convert the repo to a workspace**

In `Cargo.toml`, add the path dependency at the top of `[dependencies]`:

~~~~toml
[dependencies]
orbit-prop = {{ path = "crates/orbit-prop" }}
~~~~

and insert this block immediately before `[profile.release]`:

~~~~toml
[workspace]
members = [".", "crates/orbit-prop"]
default-members = [".", "crates/orbit-prop"]
~~~~

`default-members` makes plain `cargo test` at the root run both crates. `cargo run` still runs `scope-eval`, the only binary.

Create `crates/orbit-prop/Cargo.toml`. `sgp4` is added in Task 4, when it is first used.

~~~~toml
[package]
name = "orbit-prop"
version = "0.1.0"
edition = "2021"
rust-version = "1.70"
description = "Satellite propagation (SGP4 and Keplerian + J2) and ground-site observation geometry"
license = "MIT"

[dependencies]
~~~~

Create `crates/orbit-prop/src/constants.rs`:

~~~~rust
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
~~~~


- [ ] **Step 3: Write the failing tests**

Create each file below with **only its test module** for now. The implementation goes above it in Step 5.

`crates/orbit-prop/src/error.rs`:

~~~~rust
#[cfg(test)]
mod tests {
    use super::OrbitPropError;

    #[test]
    fn tle_error_names_line_and_column() {
        let e = OrbitPropError::TleFormat { line: 2, column: Some(69), message: "bad checksum".into() };
        assert_eq!(e.to_string(), "TLE format error on line 2, column 69: bad checksum");
    }

    #[test]
    fn tle_error_without_line_omits_it() {
        let e = OrbitPropError::TleFormat { line: 0, column: None, message: "expected 2 or 3 lines".into() };
        assert_eq!(e.to_string(), "TLE format error: expected 2 or 3 lines");
    }
}
~~~~

`crates/orbit-prop/src/time.rs`:

~~~~rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn j2000_is_jd_2451545() {
        let t = Epoch::from_utc(2000, 1, 1, 12, 0, 0.0).unwrap();
        assert_eq!(t.jd(), 2_451_545.0);
        assert_eq!(t.days_since_j2000(), 0.0);
    }

    #[test]
    fn gmst_matches_vallado_example_3_5() {
        // Vallado, Fundamentals of Astrodynamics, Example 3-5:
        // 1992 August 20, 12:14 UT1 -> GMST = 152.578787886 deg.
        let t = Epoch::from_utc(1992, 8, 20, 12, 14, 0.0).unwrap();
        assert!((t.gmst_rad().to_degrees() - 152.578_787_886).abs() < 1e-5);
    }

    #[test]
    fn iso8601_round_trips_to_the_millisecond() {
        let t = Epoch::parse_iso8601("2026-10-04T23:59:59.123Z").unwrap();
        assert_eq!(t.to_string(), "2026-10-04T23:59:59.123Z");
        assert_eq!(Epoch::parse_iso8601("2026-10-04 06:30").unwrap().to_string(), "2026-10-04T06:30:00.000Z");
    }

    #[test]
    fn iso8601_rejects_nonsense() {
        for bad in ["", "2026-10-04", "2026-13-01T00:00", "2026-02-30T00:00", "2026-10-04T24:00", "noon"] {
            assert!(Epoch::parse_iso8601(bad).is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn add_seconds_crosses_midnight_and_month_end() {
        let t = Epoch::from_utc(2024, 2, 29, 23, 59, 30.0).unwrap().add_seconds(45.5);
        assert_eq!(t.to_string(), "2024-03-01T00:00:15.500Z");
        let back = t.add_seconds(-45.5);
        assert_eq!(back.to_string(), "2024-02-29T23:59:30.000Z");
    }

    #[test]
    fn seconds_since_keeps_microseconds_over_decades() {
        let a = Epoch::from_utc(1990, 1, 1, 0, 0, 0.0).unwrap();
        let b = a.add_seconds(1.000_001);
        assert!((b.seconds_since(&a) - 1.000_001).abs() < 1e-6);
    }

    #[test]
    fn tle_day_of_year_maps_to_calendar_date() {
        // TLE epoch 00179.78495062 (Vallado's test satellite 00005) is
        // 2000-06-27 18:50:19.734 UTC.
        let t = Epoch::from_year_day(2000, 179.784_950_62).unwrap();
        assert_eq!(t.to_string(), "2000-06-27T18:50:19.734Z");
        assert!(Epoch::from_year_day(2001, 366.5).is_err());
    }

    #[test]
    fn display_rounds_up_into_the_next_day() {
        let t = Epoch::from_utc(2026, 12, 31, 23, 59, 59.9996).unwrap();
        assert_eq!(t.to_string(), "2027-01-01T00:00:00.000Z");
    }
}
~~~~

Register the modules in `crates/orbit-prop/src/lib.rs` (full file):

~~~~rust
//! Satellite propagation and ground-site observation geometry.
//!
//! Assessment-grade (about 0.01 deg); see README.md for accuracy and limits.

pub mod constants;
pub mod error;
pub mod time;

pub use error::OrbitPropError;
pub use time::{Epoch, UtcParts};
~~~~

- [ ] **Step 4: Run the tests and confirm they fail**

Run: `cargo test -p orbit-prop `
Expected: compile errors (`cannot find ... in this scope`, unresolved imports), because nothing is implemented yet.

- [ ] **Step 5: Write the implementation**

Put each implementation **above** the test module already in the file (files without tests are created whole).

`crates/orbit-prop/src/error.rs`:

~~~~rust
//! The single error type returned by every fallible function in the crate.

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum OrbitPropError {
    /// A TLE could not be parsed. `line` is 1 or 2, or 0 when the problem is
    /// not tied to one line. `column` is 1-based when known.
    TleFormat { line: u8, column: Option<usize>, message: String },
    /// SGP4 failed, for example because the orbit has decayed.
    Sgp4(String),
    /// Orbital elements that cannot describe a closed orbit above the Earth.
    InvalidElements(String),
    /// A ground site outside the valid latitude range or with non-finite values.
    InvalidSite(String),
    /// An unparseable time, or an invalid search window.
    InvalidTime(String),
    /// An iterative solver did not converge.
    NoConvergence(String),
}

impl fmt::Display for OrbitPropError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrbitPropError::TleFormat { line, column, message } => {
                write!(f, "TLE format error")?;
                if *line > 0 {
                    write!(f, " on line {line}")?;
                }
                if let Some(c) = column {
                    write!(f, ", column {c}")?;
                }
                write!(f, ": {message}")
            }
            OrbitPropError::Sgp4(m) => write!(f, "SGP4 propagation failed: {m}"),
            OrbitPropError::InvalidElements(m) => write!(f, "invalid orbital elements: {m}"),
            OrbitPropError::InvalidSite(m) => write!(f, "invalid site: {m}"),
            OrbitPropError::InvalidTime(m) => write!(f, "invalid time: {m}"),
            OrbitPropError::NoConvergence(m) => write!(f, "no convergence: {m}"),
        }
    }
}

impl std::error::Error for OrbitPropError {}
~~~~

`crates/orbit-prop/src/time.rs`:

~~~~rust
//! UTC instants and sidereal time.
//!
//! An [`Epoch`] is stored as a split Julian date: a whole-day part and a
//! fraction in `[0, 1)`. Keeping the two apart preserves about 10
//! microseconds of precision, which a single `f64` Julian date (about
//! 2.5 million days) cannot.
//!
//! Leap seconds are ignored, and UTC is treated as UT1 for sidereal time.
//! Both are assessment-grade simplifications; see the crate README.

use std::f64::consts::TAU;
use std::fmt;

use crate::constants::{J2000_JD, SECONDS_PER_DAY};
use crate::error::OrbitPropError;

/// Julian date of the Unix epoch, 1970-01-01 00:00 UTC.
const UNIX_EPOCH_JD: f64 = 2_440_587.5;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Epoch {
    jd_whole: f64,
    jd_fraction: f64,
}

/// Calendar components of an [`Epoch`], rounded to the millisecond.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UtcParts {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: f64,
}

impl Epoch {
    fn from_parts(whole: f64, fraction: f64) -> Epoch {
        let carry = fraction.floor();
        Epoch { jd_whole: whole + carry, jd_fraction: fraction - carry }
    }

    /// An instant from a UTC calendar date and time of day.
    pub fn from_utc(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: f64) -> Result<Epoch, OrbitPropError> {
        let valid = (1..=12).contains(&month)
            && day >= 1
            && day <= days_in_month(year, month)
            && hour < 24
            && minute < 60
            && second.is_finite()
            && (0.0..60.0).contains(&second);
        if !valid {
            return Err(OrbitPropError::InvalidTime(format!(
                "{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second} is not a valid UTC date and time"
            )));
        }
        // Julian day number at noon (Fliegel and Van Flandern), then back half a day to midnight.
        let jdn = julian_day_number(year, month, day) as f64;
        let seconds = hour as f64 * 3600.0 + minute as f64 * 60.0 + second;
        Ok(Epoch::from_parts(jdn, seconds / SECONDS_PER_DAY - 0.5))
    }

    /// An instant from a year and a fractional day of year, where 1.0 is
    /// January 1 at 00:00. This is the TLE epoch format.
    pub fn from_year_day(year: i32, day_of_year: f64) -> Result<Epoch, OrbitPropError> {
        let max_day = if is_leap(year) { 367.0 } else { 366.0 };
        if !day_of_year.is_finite() || !(1.0..max_day).contains(&day_of_year) {
            return Err(OrbitPropError::InvalidTime(format!("day of year {day_of_year} is out of range for {year}")));
        }
        Ok(Epoch::from_utc(year, 1, 1, 0, 0, 0.0)?.add_seconds((day_of_year - 1.0) * SECONDS_PER_DAY))
    }

    /// The current time from the system clock.
    pub fn now() -> Epoch {
        let since_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        Epoch::from_parts(UNIX_EPOCH_JD.floor(), UNIX_EPOCH_JD.fract() + since_unix / SECONDS_PER_DAY)
    }

    /// Parse `YYYY-MM-DDTHH:MM[:SS[.sss]][Z]`. A space may replace the `T`.
    /// Times are always UTC; a trailing `Z` is optional.
    pub fn parse_iso8601(text: &str) -> Result<Epoch, OrbitPropError> {
        let bad = || OrbitPropError::InvalidTime(format!("'{text}' is not YYYY-MM-DDTHH:MM[:SS]"));
        let s = text.trim();
        let s = s.strip_suffix('Z').or_else(|| s.strip_suffix('z')).unwrap_or(s);
        let (date, time) = s.split_once(['T', 't', ' ']).ok_or_else(bad)?;
        let d: Vec<&str> = date.split('-').collect();
        let t: Vec<&str> = time.split(':').collect();
        if d.len() != 3 || !(2..=3).contains(&t.len()) {
            return Err(bad());
        }
        let year: i32 = d[0].parse().map_err(|_| bad())?;
        let month: u32 = d[1].parse().map_err(|_| bad())?;
        let day: u32 = d[2].parse().map_err(|_| bad())?;
        let hour: u32 = t[0].parse().map_err(|_| bad())?;
        let minute: u32 = t[1].parse().map_err(|_| bad())?;
        let second: f64 = match t.get(2) {
            Some(s) => s.parse().map_err(|_| bad())?,
            None => 0.0,
        };
        Epoch::from_utc(year, month, day, hour, minute, second)
    }

    pub fn add_seconds(&self, seconds: f64) -> Epoch {
        Epoch::from_parts(self.jd_whole, self.jd_fraction + seconds / SECONDS_PER_DAY)
    }

    /// `self - other`, in seconds.
    pub fn seconds_since(&self, other: &Epoch) -> f64 {
        ((self.jd_whole - other.jd_whole) + (self.jd_fraction - other.jd_fraction)) * SECONDS_PER_DAY
    }

    /// The Julian date as a single number. Precise to about 20 microseconds.
    pub fn jd(&self) -> f64 {
        self.jd_whole + self.jd_fraction
    }

    /// Days since J2000.0, keeping full precision.
    pub fn days_since_j2000(&self) -> f64 {
        (self.jd_whole - J2000_JD) + self.jd_fraction
    }

    /// Greenwich mean sidereal time, radians in `[0, 2 pi)`.
    ///
    /// IAU-1982 expression (Vallado eq. 3-47), with UTC used for UT1.
    // TODO(astrometric): use UT1 = UTC + DUT1 from IERS Bulletin A.
    pub fn gmst_rad(&self) -> f64 {
        let t = self.days_since_j2000() / 36_525.0;
        let seconds = 67_310.548_41 + (876_600.0 * 3600.0 + 8_640_184.812_866) * t + 0.093_104 * t * t
            - 6.2e-6 * t * t * t;
        (seconds.rem_euclid(SECONDS_PER_DAY) / SECONDS_PER_DAY) * TAU
    }

    /// Calendar components, rounded to the nearest millisecond.
    pub fn to_utc(&self) -> UtcParts {
        // Shift so the whole part counts days starting at midnight.
        let shifted = Epoch::from_parts(self.jd_whole, self.jd_fraction + 0.5);
        let mut jdn = shifted.jd_whole as i64;
        let mut ms = (shifted.jd_fraction * SECONDS_PER_DAY * 1000.0).round() as i64;
        if ms >= 86_400_000 {
            ms -= 86_400_000;
            jdn += 1;
        }
        let (year, month, day) = calendar_from_jdn(jdn);
        UtcParts {
            year,
            month,
            day,
            hour: (ms / 3_600_000) as u32,
            minute: (ms / 60_000 % 60) as u32,
            second: (ms % 60_000) as f64 / 1000.0,
        }
    }
}

impl fmt::Display for Epoch {
    /// ISO-8601 UTC with milliseconds, e.g. `2026-10-04T12:00:00.000Z`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let u = self.to_utc();
        write!(
            f,
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:06.3}Z",
            u.year, u.month, u.day, u.hour, u.minute, u.second
        )
    }
}

fn is_leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Julian day number (the JD at noon) of a Gregorian calendar date.
fn julian_day_number(year: i32, month: u32, day: u32) -> i64 {
    let (y, m, d) = (year as i64, month as i64, day as i64);
    let a = (14 - m) / 12;
    let yy = y + 4800 - a;
    let mm = m + 12 * a - 3;
    d + (153 * mm + 2) / 5 + 365 * yy + yy / 4 - yy / 100 + yy / 400 - 32_045
}

/// Inverse of [`julian_day_number`].
fn calendar_from_jdn(jdn: i64) -> (i32, u32, u32) {
    let a = jdn + 32_044;
    let b = (4 * a + 3) / 146_097;
    let c = a - 146_097 * b / 4;
    let d = (4 * c + 3) / 1461;
    let e = c - 1461 * d / 4;
    let m = (5 * e + 2) / 153;
    let day = e - (153 * m + 2) / 5 + 1;
    let month = m + 3 - 12 * (m / 10);
    let year = 100 * b + d - 4800 + m / 10;
    (year as i32, month as u32, day as u32)
}
~~~~

- [ ] **Step 6: Run the tests and confirm they pass**

Run: `cargo test -p orbit-prop`
Expected: `test result: ok. 10 passed; 0 failed`, with no compiler warnings.

- [ ] **Step 7: Share the constants with scope-eval and check nothing changed**

In `src/constants.rs`, replace

~~~~rust
/// Earth's gravitational parameter, km^3/s^2.
pub const MU_EARTH: f64 = 398_600.4418;
/// Earth's equatorial radius, km.
pub const EARTH_RADIUS_KM: f64 = 6_378.137;
~~~~

with

~~~~rust
/// Earth's gravitational parameter (km^3/s^2) and equatorial radius (km),
/// shared with the `orbit-prop` library so the two can never disagree.
pub use orbit_prop::constants::{EARTH_RADIUS_KM, MU_EARTH};
~~~~

Run: `cargo test`
Expected: `orbit-prop` reports `10 passed`, and `scope-eval` reports `119 passed` (unchanged; the values are identical).

Run: `cargo run -q -- --demo > target/demo-after.txt && cmp target/demo-before.txt target/demo-after.txt && echo SAME`
Expected: `SAME`.

- [ ] **Step 8: Commit**

~~~~bash
git add Cargo.toml Cargo.lock src/constants.rs crates/orbit-prop
git commit -m "Add orbit-prop crate skeleton with constants, errors and time

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
~~~~


---

### Task 2: Ground site and reference frames

**Files:**
- Create: `crates/orbit-prop/src/{vec3,site,frames}.rs`
- Modify: `crates/orbit-prop/src/lib.rs`

**Interfaces:**
- Consumes: `Epoch::gmst_rad`; constants `EARTH_RADIUS_KM`, `EARTH_FLATTENING`, `EARTH_ROTATION_RAD_S`.
- Produces:
  - `vec3` (crate-private): `type Vec3 = [f64; 3]`; `add, sub, scale, dot, cross, norm, rot_z(a, angle), angle_between(a, b) -> rad`.
  - `GroundSite { lat_deg, lon_deg, alt_m }` (`Copy, PartialEq`): `GroundSite::new(lat_deg, lon_deg, alt_m) -> Result<GroundSite, OrbitPropError>`. Longitude is wrapped to (-180, 180].
  - `frames::{teme_to_ecef(r, v, t) -> (Vec3, Vec3), ecef_to_teme(r, v, t) -> (Vec3, Vec3), site_ecef(&GroundSite) -> Vec3, ecef_to_geodetic(Vec3) -> (lat_deg, lon_deg, alt_m), ecef_to_sez(Vec3, &GroundSite) -> Vec3, sez_to_az_el(Vec3) -> (az_deg, el_deg)}`.

`vec3.rs` has no tests of its own; it is exercised through `frames` here and through `observe`, `illumination` and `keplerian` later.


- [ ] **Step 1: Write the failing tests**

Create each file below with **only its test module** for now. The implementation goes above it in Step 3.

`crates/orbit-prop/src/site.rs`:

~~~~rust
#[cfg(test)]
mod tests {
    use super::GroundSite;

    #[test]
    fn accepts_a_normal_site_and_wraps_longitude() {
        let s = GroundSite::new(39.007, 255.117, 2194.56).unwrap();
        assert!((s.lon_deg - (-104.883)).abs() < 1e-9);
    }

    #[test]
    fn rejects_bad_latitude_and_nan() {
        assert!(GroundSite::new(90.5, 0.0, 0.0).is_err());
        assert!(GroundSite::new(f64::NAN, 0.0, 0.0).is_err());
        assert!(GroundSite::new(0.0, f64::INFINITY, 0.0).is_err());
        assert!(GroundSite::new(0.0, 0.0, 50_000.0).is_err());
    }
}
~~~~

`crates/orbit-prop/src/frames.rs`:

~~~~rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::vec3::{norm, sub};

    #[test]
    fn site_ecef_matches_vallado_example_3_3() {
        // Vallado Example 3-3: lat 39.007 deg, lon -104.883 deg, alt 2187 m
        // -> r = (-1275.1219, -4797.9890, 3994.2975) km.
        let s = GroundSite::new(39.007, -104.883, 2187.0).unwrap();
        let r = site_ecef(&s);
        let expected = [-1275.1219, -4797.9890, 3994.2975];
        assert!(norm(sub(r, expected)) < 0.001, "{r:?}");
    }

    #[test]
    fn geodetic_round_trip() {
        for (lat, lon, alt) in [(39.007, -104.883, 2194.56), (-33.9, 18.4, 10.0), (89.99, 45.0, 0.0), (0.0, 180.0, 5000.0)] {
            let s = GroundSite::new(lat, lon, alt).unwrap();
            let (la, lo, h) = ecef_to_geodetic(site_ecef(&s));
            assert!((la - lat).abs() < 1e-9 && (h - alt).abs() < 1e-3, "{lat} {lon} {alt} -> {la} {lo} {h}");
            assert!(((lo - s.lon_deg + 540.0).rem_euclid(360.0) - 180.0).abs() < 1e-9);
        }
    }

    #[test]
    fn teme_ecef_round_trip() {
        let t = Epoch::from_utc(2026, 10, 4, 3, 0, 0.0).unwrap();
        let (r, v) = ([7000.0, -1200.0, 300.0], [1.0, 7.0, 2.0]);
        let (re, ve) = teme_to_ecef(r, v, t);
        let (r2, v2) = ecef_to_teme(re, ve, t);
        assert!(norm(sub(r, r2)) < 1e-9 && norm(sub(v, v2)) < 1e-12);
    }

    #[test]
    fn earth_fixed_point_has_zero_ecef_velocity() {
        // A point riding with the Earth: TEME velocity = omega x r.
        let t = Epoch::from_utc(2026, 10, 4, 3, 0, 0.0).unwrap();
        let r = [6378.137, 0.0, 0.0];
        let (rt, vt) = ecef_to_teme(r, [0.0; 3], t);
        assert!((norm(vt) - EARTH_ROTATION_RAD_S * 6378.137).abs() < 1e-12);
        let (_, ve) = teme_to_ecef(rt, vt, t);
        assert!(norm(ve) < 1e-12);
    }

    #[test]
    fn az_el_of_cardinal_directions() {
        let s = GroundSite::new(0.0, 0.0, 0.0).unwrap();
        let site = site_ecef(&s);
        let look = |target: Vec3| sez_to_az_el(ecef_to_sez(sub(target, site), &s));
        let (az, el) = look([6378.137, 0.0, 1000.0]); // north, on the horizon
        assert!(az.abs() < 1e-9 && el.abs() < 1e-9);
        let (az, el) = look([6378.137, 1000.0, 0.0]); // east, on the horizon
        assert!((az - 90.0).abs() < 1e-9 && el.abs() < 1e-9);
        let (_, el) = look([6878.137, 0.0, 0.0]); // straight up
        assert!((el - 90.0).abs() < 1e-9);
    }
}
~~~~

Register the modules in `crates/orbit-prop/src/lib.rs` (full file):

~~~~rust
//! Satellite propagation and ground-site observation geometry.
//!
//! Assessment-grade (about 0.01 deg); see README.md for accuracy and limits.

pub mod constants;
pub mod error;
pub mod frames;
pub mod site;
pub mod time;
// Some helpers are first used by observe and illumination (Tasks 5-6).
#[allow(dead_code)]
mod vec3;

pub use error::OrbitPropError;
pub use site::GroundSite;
pub use time::{Epoch, UtcParts};
~~~~

- [ ] **Step 2: Run the tests and confirm they fail**

Run: `cargo test -p orbit-prop `
Expected: compile errors (`cannot find ... in this scope`, unresolved imports), because nothing is implemented yet.

- [ ] **Step 3: Write the implementation**

Put each implementation **above** the test module already in the file (files without tests are created whole).

`crates/orbit-prop/src/vec3.rs`:

~~~~rust
//! Minimal 3-vector helpers used inside the crate.

pub type Vec3 = [f64; 3];

pub fn add(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

pub fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub fn scale(a: Vec3, k: f64) -> Vec3 {
    [a[0] * k, a[1] * k, a[2] * k]
}

pub fn dot(a: Vec3, b: Vec3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

pub fn norm(a: Vec3) -> f64 {
    dot(a, a).sqrt()
}

/// Rotation about the z axis by `angle` (the frame rotates, the vector stays):
/// `x' = cos x + sin y`, `y' = -sin x + cos y`.
pub fn rot_z(a: Vec3, angle: f64) -> Vec3 {
    let (s, c) = angle.sin_cos();
    [c * a[0] + s * a[1], -s * a[0] + c * a[1], a[2]]
}

/// Angle between two vectors, radians in `[0, pi]`.
pub fn angle_between(a: Vec3, b: Vec3) -> f64 {
    norm(cross(a, b)).atan2(dot(a, b))
}
~~~~

`crates/orbit-prop/src/site.rs`:

~~~~rust
//! A ground observing site.

use crate::error::OrbitPropError;

/// A site on the WGS-84 ellipsoid. Latitude is geodetic, positive north;
/// longitude is positive east; altitude is above the ellipsoid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroundSite {
    pub lat_deg: f64,
    pub lon_deg: f64,
    pub alt_m: f64,
}

impl GroundSite {
    /// Validates the inputs. Longitude is wrapped into `(-180, 180]`.
    pub fn new(lat_deg: f64, lon_deg: f64, alt_m: f64) -> Result<GroundSite, OrbitPropError> {
        if !lat_deg.is_finite() || !(-90.0..=90.0).contains(&lat_deg) {
            return Err(OrbitPropError::InvalidSite(format!("latitude {lat_deg} must be between -90 and 90 degrees")));
        }
        if !lon_deg.is_finite() {
            return Err(OrbitPropError::InvalidSite(format!("longitude {lon_deg} is not a number")));
        }
        if !alt_m.is_finite() || !(-500.0..=10_000.0).contains(&alt_m) {
            return Err(OrbitPropError::InvalidSite(format!("altitude {alt_m} m must be between -500 and 10000 m")));
        }
        let mut lon = lon_deg.rem_euclid(360.0);
        if lon > 180.0 {
            lon -= 360.0;
        }
        Ok(GroundSite { lat_deg, lon_deg: lon, alt_m })
    }
}
~~~~

`crates/orbit-prop/src/frames.rs`:

~~~~rust
//! Reference-frame conversions.
//!
//! **Accuracy: assessment grade, about 0.01 deg.** TEME is rotated to the
//! Earth-fixed frame by Greenwich mean sidereal time alone. Polar motion,
//! UT1 - UTC, and the equation of the equinoxes are ignored, and TEME is
//! treated as true-of-date. That is good enough for pass timing, rates and
//! lighting, and smaller than typical TLE error. Astrometric work needs the
//! upgrades listed under "Known limitations and future work" in the crate
//! README.
//!
//! Frames used:
//! * TEME: the inertial frame SGP4 outputs. x toward the mean equinox.
//! * ECEF: Earth-fixed, x through 0 deg latitude / 0 deg longitude.
//! * SEZ: topocentric south-east-zenith at a ground site.

use crate::constants::{EARTH_FLATTENING, EARTH_RADIUS_KM, EARTH_ROTATION_RAD_S};
use crate::site::GroundSite;
use crate::time::Epoch;
use crate::vec3::{rot_z, Vec3};

/// TEME position and velocity to ECEF.
// TODO(astrometric): add polar motion and use GCRS -> ITRS (IAU-2006/2000A).
pub fn teme_to_ecef(r_teme: Vec3, v_teme: Vec3, t: Epoch) -> (Vec3, Vec3) {
    let theta = t.gmst_rad();
    let r = rot_z(r_teme, theta);
    let v_rot = rot_z(v_teme, theta);
    // Subtract Earth rotation: v_ecef = R v_teme - omega x r_ecef.
    let w = EARTH_ROTATION_RAD_S;
    let v = [v_rot[0] + w * r[1], v_rot[1] - w * r[0], v_rot[2]];
    (r, v)
}

/// ECEF position and velocity to TEME (inverse of [`teme_to_ecef`]).
pub fn ecef_to_teme(r_ecef: Vec3, v_ecef: Vec3, t: Epoch) -> (Vec3, Vec3) {
    let w = EARTH_ROTATION_RAD_S;
    let v_rot = [v_ecef[0] - w * r_ecef[1], v_ecef[1] + w * r_ecef[0], v_ecef[2]];
    let theta = t.gmst_rad();
    (rot_z(r_ecef, -theta), rot_z(v_rot, -theta))
}

fn eccentricity_squared() -> f64 {
    EARTH_FLATTENING * (2.0 - EARTH_FLATTENING)
}

/// Geodetic site to ECEF position, km (WGS-84).
pub fn site_ecef(site: &GroundSite) -> Vec3 {
    let (lat, lon) = (site.lat_deg.to_radians(), site.lon_deg.to_radians());
    let h = site.alt_m / 1000.0;
    let e2 = eccentricity_squared();
    let n = EARTH_RADIUS_KM / (1.0 - e2 * lat.sin().powi(2)).sqrt();
    [
        (n + h) * lat.cos() * lon.cos(),
        (n + h) * lat.cos() * lon.sin(),
        (n * (1.0 - e2) + h) * lat.sin(),
    ]
}

/// ECEF position, km, to geodetic latitude and longitude (deg) and altitude (m).
/// Iterative; converges to well under a millimetre in a few steps.
pub fn ecef_to_geodetic(r: Vec3) -> (f64, f64, f64) {
    let e2 = eccentricity_squared();
    let p = (r[0] * r[0] + r[1] * r[1]).sqrt();
    let lon = r[1].atan2(r[0]);
    let mut lat = r[2].atan2(p * (1.0 - e2));
    let mut h = 0.0;
    for _ in 0..10 {
        let n = EARTH_RADIUS_KM / (1.0 - e2 * lat.sin().powi(2)).sqrt();
        h = if lat.cos().abs() > 1e-10 { p / lat.cos() - n } else { r[2].abs() - n * (1.0 - e2) };
        lat = r[2].atan2(p * (1.0 - e2 * n / (n + h)));
    }
    (lat.to_degrees(), lon.to_degrees(), h * 1000.0)
}

/// Rotate an ECEF vector into the site's south-east-zenith frame.
/// Uses geodetic latitude, so zenith is along the local vertical.
pub fn ecef_to_sez(v: Vec3, site: &GroundSite) -> Vec3 {
    let (lat, lon) = (site.lat_deg.to_radians(), site.lon_deg.to_radians());
    let (sl, cl) = lat.sin_cos();
    let (so, co) = lon.sin_cos();
    [
        sl * co * v[0] + sl * so * v[1] - cl * v[2],
        -so * v[0] + co * v[1],
        cl * co * v[0] + cl * so * v[1] + sl * v[2],
    ]
}

/// Azimuth (deg, from north through east, `[0, 360)`) and elevation (deg)
/// of a south-east-zenith vector.
pub fn sez_to_az_el(sez: Vec3) -> (f64, f64) {
    let north = -sez[0];
    let east = sez[1];
    let horizontal = (north * north + east * east).sqrt();
    let az = east.atan2(north).to_degrees().rem_euclid(360.0);
    let el = sez[2].atan2(horizontal).to_degrees();
    (az, el)
}
~~~~

- [ ] **Step 4: Run the tests and confirm they pass**

Run: `cargo test -p orbit-prop`
Expected: `test result: ok. 17 passed; 0 failed`, with no compiler warnings.

- [ ] **Step 5: Commit**

~~~~bash
git add crates/orbit-prop
git commit -m "Add ground site and frame conversions to orbit-prop

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
~~~~


---

### Task 3: Propagator trait and Keplerian + J2 propagator

**Files:**
- Create: `crates/orbit-prop/src/{state,propagator,keplerian}.rs`
- Modify: `crates/orbit-prop/src/lib.rs`

**Interfaces:**
- Consumes: `Epoch`, `vec3::{add, cross, scale, norm}`, constants `MU_EARTH`, `J2`, `EARTH_RADIUS_KM`.
- Produces:
  - `StateVector { epoch: Epoch, r_km: [f64; 3], v_km_s: [f64; 3] }` (TEME, `Copy, PartialEq`).
  - `trait Propagator { fn propagate(&self, t: Epoch) -> Result<StateVector, OrbitPropError>; fn period_s(&self) -> f64; fn label(&self) -> &str; }`
  - `KeplerElements { epoch, a_km, e, i_deg, raan_deg, argp_deg, mean_anomaly_deg }` and `KeplerElements::from_altitudes(epoch, perigee_alt_km, apogee_alt_km, i_deg, raan_deg, argp_deg, mean_anomaly_deg) -> Result<KeplerElements, _>`.
  - `KeplerJ2::new(KeplerElements, label: &str) -> Result<KeplerJ2, _>`, `KeplerJ2::elements(&self) -> &KeplerElements`, plus `impl Propagator`.


- [ ] **Step 1: Write the failing tests**

Create each file below with **only its test module** for now. The implementation goes above it in Step 3.

`crates/orbit-prop/src/keplerian.rs`:

~~~~rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::vec3::{cross, norm};

    fn epoch() -> Epoch {
        Epoch::from_utc(2026, 10, 4, 0, 0, 0.0).unwrap()
    }

    fn circular(alt_km: f64, i_deg: f64) -> KeplerJ2 {
        let el = KeplerElements::from_altitudes(epoch(), alt_km, alt_km, i_deg, 0.0, 0.0, 0.0).unwrap();
        KeplerJ2::new(el, "test").unwrap()
    }

    #[test]
    fn period_is_two_pi_root_a_cubed_over_mu() {
        let a: f64 = 6378.137 + 500.0;
        let expected = TAU * (a.powi(3) / MU_EARTH).sqrt();
        assert!((circular(500.0, 0.0).period_s() - expected).abs() < 1e-9);
        assert!((expected - 5676.98).abs() < 0.01);
    }

    #[test]
    fn equatorial_circular_orbit_starts_on_x_axis() {
        let prop = circular(500.0, 0.0);
        let s = prop.propagate(epoch()).unwrap();
        let a = 6878.137;
        assert!((s.r_km[0] - a).abs() < 1e-9 && s.r_km[1].abs() < 1e-9 && s.r_km[2].abs() < 1e-9);
        // Speed is a times the rate of the argument of latitude, which J2
        // makes slightly faster than the two-body mean motion.
        let u_dot = prop.m_dot + prop.argp_dot + prop.raan_dot;
        assert!(s.v_km_s[0].abs() < 1e-12 && (s.v_km_s[1] - a * u_dot).abs() < 1e-12);
        assert!(u_dot > prop.n && (u_dot - prop.n) / prop.n < 3e-3);
    }

    #[test]
    fn velocity_is_the_time_derivative_of_position() {
        let el = KeplerElements::from_altitudes(epoch(), 400.0, 39_000.0, 63.4, 40.0, 270.0, 10.0).unwrap();
        let prop = KeplerJ2::new(el, "molniya").unwrap();
        for hours in [0.0, 1.0, 5.5, 13.0, 100.0] {
            let t = epoch().add_seconds(hours * 3600.0);
            let a = prop.propagate(t.add_seconds(-0.5)).unwrap();
            let o = prop.propagate(t).unwrap();
            let b = prop.propagate(t.add_seconds(0.5)).unwrap();
            for k in 0..3 {
                assert!(((b.r_km[k] - a.r_km[k]) - o.v_km_s[k]).abs() < 1e-5, "hours {hours} axis {k}");
            }
        }
    }

    #[test]
    fn propagates_backwards_from_epoch() {
        // A search window that starts before the element epoch must work.
        let prop = circular(550.0, 53.0);
        let back = prop.propagate(epoch().add_seconds(-86_400.0)).unwrap();
        assert!((norm(back.r_km) - (6378.137 + 550.0)).abs() < 1e-6);
    }

    #[test]
    fn perigee_and_apogee_radii() {
        let at_m = |m: f64| {
            let el = KeplerElements::from_altitudes(epoch(), 400.0, 39_000.0, 63.4, 40.0, 270.0, m).unwrap();
            norm(KeplerJ2::new(el, "x").unwrap().propagate(epoch()).unwrap().r_km)
        };
        assert!((at_m(0.0) - (6378.137 + 400.0)).abs() < 1e-6);
        assert!((at_m(180.0) - (6378.137 + 39_000.0)).abs() < 1e-6);
    }

    #[test]
    fn sun_synchronous_orbit_regresses_about_one_degree_per_day() {
        // An 800 km orbit at 98.6 deg is sun-synchronous: the node advances
        // 360 deg per year, 0.9856 deg/day.
        let prop = circular(800.0, 98.6);
        let node = |t: Epoch| {
            let s = prop.propagate(t).unwrap();
            let h = cross(s.r_km, s.v_km_s);
            h[0].atan2(-h[1]).to_degrees()
        };
        let drift = node(epoch().add_seconds(86_400.0)) - node(epoch());
        assert!((drift - 0.9856).abs() < 0.01, "drift {drift}");
    }

    #[test]
    fn rejects_impossible_orbits() {
        let bad = |e: f64, a: f64, i: f64| {
            let el = KeplerElements { epoch: epoch(), a_km: a, e, i_deg: i, raan_deg: 0.0, argp_deg: 0.0, mean_anomaly_deg: 0.0 };
            KeplerJ2::new(el, "x").is_err()
        };
        assert!(bad(1.0, 10_000.0, 0.0), "parabolic");
        assert!(bad(-0.1, 10_000.0, 0.0), "negative e");
        assert!(bad(0.5, 10_000.0, 0.0), "perigee 5000 km radius is underground");
        assert!(bad(0.0, 7000.0, 200.0), "inclination over 180");
        assert!(bad(0.0, f64::NAN, 0.0), "NaN");
        assert!(KeplerElements::from_altitudes(epoch(), 1000.0, 500.0, 0.0, 0.0, 0.0, 0.0).is_err());
    }

    #[test]
    fn highly_eccentric_orbit_converges() {
        let el = KeplerElements { epoch: epoch(), a_km: 200_000.0, e: 0.96, i_deg: 10.0, raan_deg: 0.0, argp_deg: 0.0, mean_anomaly_deg: 0.0 };
        let prop = KeplerJ2::new(el, "x").unwrap();
        for k in 0..200 {
            assert!(prop.propagate(epoch().add_seconds(k as f64 * 3_000.0)).is_ok());
        }
    }
}
~~~~

Register the modules in `crates/orbit-prop/src/lib.rs` (full file):

~~~~rust
//! Satellite propagation and ground-site observation geometry.
//!
//! Assessment-grade (about 0.01 deg); see README.md for accuracy and limits.

pub mod constants;
pub mod error;
pub mod frames;
pub mod keplerian;
pub mod propagator;
pub mod site;
pub mod state;
pub mod time;
// Some helpers are first used by observe and illumination (Tasks 5-6).
#[allow(dead_code)]
mod vec3;

pub use error::OrbitPropError;
pub use keplerian::{KeplerElements, KeplerJ2};
pub use propagator::Propagator;
pub use site::GroundSite;
pub use state::StateVector;
pub use time::{Epoch, UtcParts};
~~~~

- [ ] **Step 2: Run the tests and confirm they fail**

Run: `cargo test -p orbit-prop `
Expected: compile errors (`cannot find ... in this scope`, unresolved imports), because nothing is implemented yet.

- [ ] **Step 3: Write the implementation**

Put each implementation **above** the test module already in the file (files without tests are created whole).

`crates/orbit-prop/src/state.rs`:

~~~~rust
//! Position and velocity at an instant.

use crate::time::Epoch;

/// Position (km) and velocity (km/s) in the TEME frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StateVector {
    pub epoch: Epoch,
    pub r_km: [f64; 3],
    pub v_km_s: [f64; 3],
}
~~~~

`crates/orbit-prop/src/propagator.rs`:

~~~~rust
//! The common interface every orbit model implements.

use crate::error::OrbitPropError;
use crate::state::StateVector;
use crate::time::Epoch;

/// Anything that can say where a satellite is at a given time.
///
/// Downstream code (observation geometry, pass finding, the simulator)
/// depends only on this trait, never on a concrete orbit model.
pub trait Propagator {
    /// Position and velocity in TEME at `t`.
    fn propagate(&self, t: Epoch) -> Result<StateVector, OrbitPropError>;
    /// Nominal orbital period, seconds. Used to choose pass-search step sizes.
    fn period_s(&self) -> f64;
    /// Human-readable label, e.g. the TLE name or "what-if orbit".
    fn label(&self) -> &str;
}
~~~~

`crates/orbit-prop/src/keplerian.rs`:

~~~~rust
//! Keplerian orbit with secular J2 drift, for what-if orbits.
//!
//! The elements are treated as mean elements. J2 makes the node regress,
//! the perigee rotate and the mean motion change (Vallado eqs. 9-41):
//!
//! ```text
//! k      = 1.5 J2 (R / p)^2 n,   p = a (1 - e^2)
//! dRAAN  = -k cos i
//! dargp  =  k (2 - 2.5 sin^2 i)
//! dM     =  n + k sqrt(1 - e^2) (1 - 1.5 sin^2 i)
//! ```
//!
//! Short-period J2 terms, drag and third bodies are ignored, so this
//! drifts from a real satellite by kilometres per day. It is meant for
//! "what would a 550 km, 53 deg orbit look like from my site", not for
//! tracking a specific object; use a TLE and SGP4 for that.

use std::f64::consts::TAU;

use crate::constants::{EARTH_RADIUS_KM, J2, MU_EARTH};
use crate::error::OrbitPropError;
use crate::propagator::Propagator;
use crate::state::StateVector;
use crate::time::Epoch;
use crate::vec3::{add, cross, scale};

/// Classical orbital elements at `epoch`. Angles in degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeplerElements {
    pub epoch: Epoch,
    pub a_km: f64,
    pub e: f64,
    pub i_deg: f64,
    pub raan_deg: f64,
    pub argp_deg: f64,
    pub mean_anomaly_deg: f64,
}

impl KeplerElements {
    /// Elements from perigee and apogee altitudes above the equatorial radius, km.
    pub fn from_altitudes(
        epoch: Epoch,
        perigee_alt_km: f64,
        apogee_alt_km: f64,
        i_deg: f64,
        raan_deg: f64,
        argp_deg: f64,
        mean_anomaly_deg: f64,
    ) -> Result<KeplerElements, OrbitPropError> {
        // Negated comparisons here and below also reject NaN.
        if !(apogee_alt_km >= perigee_alt_km) {
            return Err(OrbitPropError::InvalidElements(format!(
                "apogee altitude {apogee_alt_km} km is below perigee altitude {perigee_alt_km} km"
            )));
        }
        let rp = EARTH_RADIUS_KM + perigee_alt_km;
        let ra = EARTH_RADIUS_KM + apogee_alt_km;
        Ok(KeplerElements {
            epoch,
            a_km: (rp + ra) / 2.0,
            e: (ra - rp) / (ra + rp),
            i_deg,
            raan_deg,
            argp_deg,
            mean_anomaly_deg,
        })
    }
}

/// Keplerian + secular J2 propagator.
#[derive(Debug, Clone)]
pub struct KeplerJ2 {
    el: KeplerElements,
    label: String,
    n: f64,
    raan_dot: f64,
    argp_dot: f64,
    m_dot: f64,
}

impl KeplerJ2 {
    pub fn new(el: KeplerElements, label: &str) -> Result<KeplerJ2, OrbitPropError> {
        let finite = [el.a_km, el.e, el.i_deg, el.raan_deg, el.argp_deg, el.mean_anomaly_deg]
            .iter()
            .all(|x| x.is_finite());
        if !finite {
            return Err(OrbitPropError::InvalidElements("elements must be finite numbers".into()));
        }
        if !(0.0..1.0).contains(&el.e) {
            return Err(OrbitPropError::InvalidElements(format!(
                "eccentricity {} must be at least 0 and below 1",
                el.e
            )));
        }
        if !(0.0..=180.0).contains(&el.i_deg) {
            return Err(OrbitPropError::InvalidElements(format!(
                "inclination {} deg must be between 0 and 180",
                el.i_deg
            )));
        }
        if el.a_km * (1.0 - el.e) <= EARTH_RADIUS_KM {
            return Err(OrbitPropError::InvalidElements(format!(
                "perigee radius {:.1} km is inside the Earth",
                el.a_km * (1.0 - el.e)
            )));
        }
        let n = (MU_EARTH / el.a_km.powi(3)).sqrt();
        let p = el.a_km * (1.0 - el.e * el.e);
        let k = 1.5 * J2 * (EARTH_RADIUS_KM / p).powi(2) * n;
        let (si, ci) = el.i_deg.to_radians().sin_cos();
        Ok(KeplerJ2 {
            el,
            label: label.to_string(),
            n,
            raan_dot: -k * ci,
            argp_dot: k * (2.0 - 2.5 * si * si),
            m_dot: n + k * (1.0 - el.e * el.e).sqrt() * (1.0 - 1.5 * si * si),
        })
    }

    pub fn elements(&self) -> &KeplerElements {
        &self.el
    }
}

/// Solve Kepler's equation `E - e sin E = M` for the eccentric anomaly.
fn eccentric_anomaly(m: f64, e: f64) -> Result<f64, OrbitPropError> {
    let m = m.rem_euclid(TAU);
    let mut ea = if e < 0.8 { m } else { std::f64::consts::PI };
    for _ in 0..50 {
        let step = (ea - e * ea.sin() - m) / (1.0 - e * ea.cos());
        ea -= step;
        if step.abs() < 1e-12 {
            return Ok(ea);
        }
    }
    Err(OrbitPropError::NoConvergence(format!("Kepler's equation for M = {m}, e = {e}")))
}

impl Propagator for KeplerJ2 {
    fn propagate(&self, t: Epoch) -> Result<StateVector, OrbitPropError> {
        let dt = t.seconds_since(&self.el.epoch);
        let raan = self.el.raan_deg.to_radians() + self.raan_dot * dt;
        let argp = self.el.argp_deg.to_radians() + self.argp_dot * dt;
        let m = self.el.mean_anomaly_deg.to_radians() + self.m_dot * dt;
        let (a, e) = (self.el.a_km, self.el.e);
        let ea = eccentric_anomaly(m, e)?;
        let (se, ce) = ea.sin_cos();
        let root = (1.0 - e * e).sqrt();
        let r = a * (1.0 - e * ce);
        // Perifocal position and velocity.
        let (xp, yp) = (a * (ce - e), a * root * se);
        let k = (MU_EARTH * a).sqrt() / r;
        let (vxp, vyp) = (-k * se, k * root * ce);
        // Perifocal -> inertial: R3(-raan) R1(-i) R3(-argp).
        let (so, co) = raan.sin_cos();
        let (sw, cw) = argp.sin_cos();
        let (si, ci) = self.el.i_deg.to_radians().sin_cos();
        let p = [co * cw - so * sw * ci, so * cw + co * sw * ci, sw * si];
        let q = [-co * sw - so * cw * ci, -so * sw + co * cw * ci, cw * si];
        let rv = |x: f64, y: f64| [x * p[0] + y * q[0], x * p[1] + y * q[1], x * p[2] + y * q[2]];
        let r_vec = rv(xp, yp);
        // The velocity must be the time derivative of the drifting position,
        // or angular rates computed from it disagree with the path. Scale the
        // two-body velocity by M_dot / n, then add the in-plane rotation of
        // the perigee (about the orbit normal) and the nodal regression
        // (about z).
        let h_hat = cross(p, q);
        let v_vec = add(
            add(scale(rv(vxp, vyp), self.m_dot / self.n), scale(cross(h_hat, r_vec), self.argp_dot)),
            scale(cross([0.0, 0.0, 1.0], r_vec), self.raan_dot),
        );
        Ok(StateVector { epoch: t, r_km: r_vec, v_km_s: v_vec })
    }

    fn period_s(&self) -> f64 {
        TAU / self.n
    }

    fn label(&self) -> &str {
        &self.label
    }
}
~~~~

- [ ] **Step 4: Run the tests and confirm they pass**

Run: `cargo test -p orbit-prop`
Expected: `test result: ok. 25 passed; 0 failed`, with no compiler warnings.

The `velocity_is_the_time_derivative_of_position` test guards against a subtle bug: J2 drift moves the position, so the velocity has to include the drift terms, or every angular rate computed later disagrees with the actual path.

- [ ] **Step 5: Commit**

~~~~bash
git add crates/orbit-prop
git commit -m "Add Propagator trait and Keplerian + J2 propagator

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
~~~~


---

### Task 4: TLE parsing and SGP4 propagator

**Files:**
- Modify: `crates/orbit-prop/Cargo.toml` (add `sgp4`)
- Create: `crates/orbit-prop/src/{tle,sgp4_propagator}.rs`
- Modify: `crates/orbit-prop/src/lib.rs`

**Interfaces:**
- Consumes: `Epoch::from_year_day`, `Epoch::seconds_since`, `Propagator`, `StateVector`.
- Produces:
  - `Tle { name: Option<String>, line1: String, line2: String, norad_id: u64, epoch: Epoch, mean_motion_rev_day: f64 }` (plus a crate-private `elements`), and `Tle::parse(&str) -> Result<Tle, OrbitPropError>`.
  - `Sgp4Propagator::new(&Tle) -> Result<Sgp4Propagator, _>`, `Sgp4Propagator::tle(&self) -> &Tle`, plus `impl Propagator`. The label is the TLE name, or `NORAD <id>` when there is none.
  - Test fixtures in `tle::tests` (declared `pub(crate) mod tests`), reused by later tasks: `ISS`, `SAT_00005_L1`, `SAT_00005_L2`.

- [ ] **Step 0: Add the dependency**

Append to `[dependencies]` in `crates/orbit-prop/Cargo.toml`:

~~~~toml
sgp4 = { version = "2.3", default-features = false, features = ["alloc", "std"] }
~~~~

Run: `cargo build --offline -p orbit-prop`
Expected: it builds. Default features are off so that `serde` and `serde_json` are not pulled in.


- [ ] **Step 1: Write the failing tests**

Create each file below with **only its test module** for now. The implementation goes above it in Step 3.

`crates/orbit-prop/src/tle.rs`:

~~~~rust
#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub const ISS: &str = "ISS (ZARYA)
1 25544U 98067A   08264.51782528 -.00002182  00000-0 -11606-4 0  2927
2 25544  51.6416 247.4627 0006703 130.5360 325.0288 15.72125391563537";

    pub const SAT_00005_L1: &str = "1 00005U 58002B   00179.78495062  .00000023  00000-0  28098-4 0  4753";
    pub const SAT_00005_L2: &str = "2 00005  34.2682 348.7242 1859667 331.7664  19.3264 10.82419157413667";

    #[test]
    fn parses_three_line_tle() {
        let t = Tle::parse(ISS).unwrap();
        assert_eq!(t.name.as_deref(), Some("ISS (ZARYA)"));
        assert_eq!(t.norad_id, 25544);
        assert_eq!(t.epoch.to_string(), "2008-09-20T12:25:40.104Z");
        assert!((t.mean_motion_rev_day - 15.721_253_91).abs() < 1e-8);
    }

    #[test]
    fn parses_two_line_tle_with_surrounding_whitespace() {
        let t = Tle::parse(&format!("\n   {SAT_00005_L1}  \n{SAT_00005_L2}\n\n")).unwrap();
        assert_eq!(t.name, None);
        assert_eq!(t.norad_id, 5);
        assert_eq!(t.epoch.to_string(), "2000-06-27T18:50:19.734Z");
    }

    #[test]
    fn windows_line_endings_from_a_paste_are_accepted() {
        let t = Tle::parse(&format!("ISS\r\n{SAT_00005_L1}\r\n{SAT_00005_L2}\r\n")).unwrap();
        assert_eq!(t.name.as_deref(), Some("ISS"));
        assert_eq!(t.norad_id, 5);
    }

    #[test]
    fn three_line_name_drops_leading_zero() {
        let t = Tle::parse(&format!("0 VANGUARD 1\n{SAT_00005_L1}\n{SAT_00005_L2}")).unwrap();
        assert_eq!(t.name.as_deref(), Some("VANGUARD 1"));
    }

    #[test]
    fn bad_checksum_names_line_and_column() {
        let l2 = SAT_00005_L2.replace("13667", "13668");
        match Tle::parse(&format!("{SAT_00005_L1}\n{l2}")) {
            Err(OrbitPropError::TleFormat { line: 2, column: Some(69), .. }) => {}
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn short_line_and_swapped_lines_are_rejected() {
        match Tle::parse(&format!("{}\n{SAT_00005_L2}", &SAT_00005_L1[..60])) {
            Err(OrbitPropError::TleFormat { line: 1, column: None, .. }) => {}
            other => panic!("{other:?}"),
        }
        match Tle::parse(&format!("{SAT_00005_L2}\n{SAT_00005_L1}")) {
            Err(OrbitPropError::TleFormat { line: 1, column: Some(1), .. }) => {}
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn wrong_line_count_is_rejected() {
        assert!(matches!(Tle::parse(SAT_00005_L1), Err(OrbitPropError::TleFormat { line: 0, .. })));
        assert!(matches!(Tle::parse(""), Err(OrbitPropError::TleFormat { line: 0, .. })));
    }

    #[test]
    fn non_ascii_text_is_rejected_without_panicking() {
        let l1 = SAT_00005_L1.replacen('5', "é", 1);
        assert!(Tle::parse(&format!("{l1}\n{SAT_00005_L2}")).is_err());
    }
}
~~~~

`crates/orbit-prop/src/sgp4_propagator.rs`:

~~~~rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tle::tests::{ISS, SAT_00005_L1, SAT_00005_L2};
    use crate::vec3::{norm, sub};

    fn sat_00005() -> Sgp4Propagator {
        Sgp4Propagator::new(&Tle::parse(&format!("{SAT_00005_L1}\n{SAT_00005_L2}")).unwrap()).unwrap()
    }

    #[test]
    fn matches_vallado_verification_case_00005() {
        // Vallado et al. 2006, "Revisiting Spacetrack Report #3", tcppver.out.
        let prop = sat_00005();
        let epoch = prop.tle().epoch;
        let cases = [
            (0.0, [7022.46529266, -1400.08296755, 0.03995155], [1.893841015, 6.405893759, 4.534807250]),
            (360.0, [-7154.03120202, -3783.17682504, -3536.19412294], [4.741887409, -4.151817765, -2.093935425]),
        ];
        for (minutes, r, v) in cases {
            let s = prop.propagate(epoch.add_seconds(minutes * 60.0)).unwrap();
            assert!(norm(sub(s.r_km, r)) < 0.001, "t={minutes} r={:?}", s.r_km);
            assert!(norm(sub(s.v_km_s, v)) < 1e-6, "t={minutes} v={:?}", s.v_km_s);
        }
    }

    #[test]
    fn label_and_period() {
        let iss = Sgp4Propagator::new(&Tle::parse(ISS).unwrap()).unwrap();
        assert_eq!(iss.label(), "ISS (ZARYA)");
        assert!((iss.period_s() - 86_400.0 / 15.721_253_91).abs() < 1e-6);
        assert_eq!(sat_00005().label(), "NORAD 5");
    }

    #[test]
    fn decayed_orbit_returns_an_error() {
        // Vallado's case 28350 decays roughly 1460 minutes after epoch.
        let tle = Tle::parse(
            "1 28350U 04020A   06167.21788666  .16154492  76267-5  18678-3 0  8894
2 28350  64.9977 345.6130 0024870 260.7578  99.9590 16.47856722116490",
        )
        .unwrap();
        let prop = Sgp4Propagator::new(&tle).unwrap();
        assert!(prop.propagate(tle.epoch.add_seconds(1400.0 * 60.0)).is_ok());
        assert!(matches!(prop.propagate(tle.epoch.add_seconds(1500.0 * 60.0)), Err(OrbitPropError::Sgp4(_))));
    }
}
~~~~

Register the modules in `crates/orbit-prop/src/lib.rs` (full file):

~~~~rust
//! Satellite propagation and ground-site observation geometry.
//!
//! Assessment-grade (about 0.01 deg); see README.md for accuracy and limits.

pub mod constants;
pub mod error;
pub mod frames;
pub mod keplerian;
pub mod propagator;
pub mod sgp4_propagator;
pub mod site;
pub mod state;
pub mod time;
pub mod tle;
// Some helpers are first used by observe and illumination (Tasks 5-6).
#[allow(dead_code)]
mod vec3;

pub use error::OrbitPropError;
pub use keplerian::{KeplerElements, KeplerJ2};
pub use propagator::Propagator;
pub use sgp4_propagator::Sgp4Propagator;
pub use site::GroundSite;
pub use state::StateVector;
pub use time::{Epoch, UtcParts};
pub use tle::Tle;
~~~~

- [ ] **Step 2: Run the tests and confirm they fail**

Run: `cargo test -p orbit-prop `
Expected: compile errors (`cannot find ... in this scope`, unresolved imports), because nothing is implemented yet.

- [ ] **Step 3: Write the implementation**

Put each implementation **above** the test module already in the file (files without tests are created whole).

`crates/orbit-prop/src/tle.rs`:

~~~~rust
//! Two-line element sets.
//!
//! Format and checksum are checked here, so error messages can name the
//! line and column, before the `sgp4` crate parses the fields.

use crate::error::OrbitPropError;
use crate::time::Epoch;

/// A parsed, checksum-validated TLE.
#[derive(Debug, Clone)]
pub struct Tle {
    /// Name line, if the TLE had three lines (a leading `0 ` is removed).
    pub name: Option<String>,
    pub line1: String,
    pub line2: String,
    pub norad_id: u64,
    pub epoch: Epoch,
    /// Mean motion, revolutions per day.
    pub mean_motion_rev_day: f64,
    pub(crate) elements: sgp4::Elements,
}

const TLE_LINE_LENGTH: usize = 69;

fn format_error(line: u8, column: Option<usize>, message: impl Into<String>) -> OrbitPropError {
    OrbitPropError::TleFormat { line, column, message: message.into() }
}

/// Modulo-10 checksum of the first 68 characters: digits count their value,
/// minus signs count 1, everything else counts 0.
fn checksum(line: &str) -> u32 {
    line.chars()
        .take(68)
        .map(|c| match c {
            '-' => 1,
            d => d.to_digit(10).unwrap_or(0),
        })
        .sum::<u32>()
        % 10
}

fn check_line(text: &str, number: u8) -> Result<(), OrbitPropError> {
    if !text.is_ascii() {
        return Err(format_error(number, None, "contains non-ASCII characters"));
    }
    let expected_start = if number == 1 { "1 " } else { "2 " };
    if !text.starts_with(expected_start) {
        return Err(format_error(number, Some(1), format!("line {number} must start with '{expected_start}'")));
    }
    if text.len() != TLE_LINE_LENGTH {
        return Err(format_error(number, None, format!("is {} characters long, expected {TLE_LINE_LENGTH}", text.len())));
    }
    let stated = text[68..69].parse::<u32>().map_err(|_| format_error(number, Some(69), "checksum is not a digit"))?;
    let computed = checksum(text);
    if stated != computed {
        return Err(format_error(number, Some(69), format!("checksum is {stated}, computed {computed}")));
    }
    Ok(())
}

impl Tle {
    /// Parse a TLE from 2 lines, or 3 with a leading name line. Blank lines
    /// and surrounding whitespace are ignored.
    pub fn parse(text: &str) -> Result<Tle, OrbitPropError> {
        let lines: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
        let (name, l1, l2) = match lines.as_slice() {
            [l1, l2] => (None, *l1, *l2),
            [n, l1, l2] => {
                let n = n.strip_prefix("0 ").unwrap_or(n).trim();
                (Some(n.to_string()), *l1, *l2)
            }
            _ => return Err(format_error(0, None, format!("expected 2 or 3 lines, got {}", lines.len()))),
        };
        check_line(l1, 1)?;
        check_line(l2, 2)?;
        let elements = sgp4::Elements::from_tle(name.clone(), l1.as_bytes(), l2.as_bytes())
            .map_err(|e| format_error(0, None, e.to_string()))?;
        // Epoch from the TLE text itself (columns 19-32, YYDDD.DDDDDDDD).
        let yy: i32 = l1[18..20].trim().parse().map_err(|_| format_error(1, Some(19), "epoch year is not a number"))?;
        let day: f64 = l1[20..32].trim().parse().map_err(|_| format_error(1, Some(21), "epoch day is not a number"))?;
        let year = if yy < 57 { 2000 + yy } else { 1900 + yy };
        let epoch = Epoch::from_year_day(year, day).map_err(|e| format_error(1, Some(19), e.to_string()))?;
        // Negated so NaN is rejected too.
        if !(elements.mean_motion > 0.0) {
            return Err(format_error(2, Some(53), "mean motion must be greater than zero"));
        }
        Ok(Tle {
            name,
            line1: l1.to_string(),
            line2: l2.to_string(),
            norad_id: elements.norad_id,
            epoch,
            mean_motion_rev_day: elements.mean_motion,
            elements,
        })
    }
}
~~~~

`crates/orbit-prop/src/sgp4_propagator.rs`:

~~~~rust
//! SGP4 propagation of TLEs, via the `sgp4` crate.
//!
//! Uses the crate's AFSPC compatibility mode, which reproduces the
//! reference implementation (Vallado et al. 2006) that the catalog's TLEs
//! are generated with. The crate's default "improved" mode differs from it
//! by up to tens of metres.

use crate::error::OrbitPropError;
use crate::propagator::Propagator;
use crate::state::StateVector;
use crate::time::Epoch;
use crate::tle::Tle;

pub struct Sgp4Propagator {
    tle: Tle,
    constants: sgp4::Constants,
    label: String,
}

impl Sgp4Propagator {
    pub fn new(tle: &Tle) -> Result<Sgp4Propagator, OrbitPropError> {
        let constants = sgp4::Constants::from_elements_afspc_compatibility_mode(&tle.elements)
            .map_err(|e| OrbitPropError::Sgp4(e.to_string()))?;
        let label = tle.name.clone().unwrap_or_else(|| format!("NORAD {}", tle.norad_id));
        Ok(Sgp4Propagator { tle: tle.clone(), constants, label })
    }

    pub fn tle(&self) -> &Tle {
        &self.tle
    }
}

impl Propagator for Sgp4Propagator {
    fn propagate(&self, t: Epoch) -> Result<StateVector, OrbitPropError> {
        let minutes = t.seconds_since(&self.tle.epoch) / 60.0;
        let p = self
            .constants
            .propagate_afspc_compatibility_mode(sgp4::MinutesSinceEpoch(minutes))
            .map_err(|e| OrbitPropError::Sgp4(e.to_string()))?;
        Ok(StateVector { epoch: t, r_km: p.position, v_km_s: p.velocity })
    }

    fn period_s(&self) -> f64 {
        86_400.0 / self.tle.mean_motion_rev_day
    }

    fn label(&self) -> &str {
        &self.label
    }
}
~~~~

- [ ] **Step 4: Run the tests and confirm they pass**

Run: `cargo test -p orbit-prop`
Expected: `test result: ok. 36 passed; 0 failed`, with no compiler warnings.

If `matches_vallado_verification_case_00005` fails by about 15 m at 360 min, the code is calling `from_elements` / `propagate` instead of the `_afspc_compatibility_mode` variants.

- [ ] **Step 5: Commit**

~~~~bash
git add Cargo.lock crates/orbit-prop
git commit -m "Add TLE parsing and SGP4 propagation

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
~~~~


---

### Task 5: Observation geometry

**Files:**
- Create: `crates/orbit-prop/src/observe.rs`
- Modify: `crates/orbit-prop/src/lib.rs`

**Interfaces:**
- Consumes: `frames::{teme_to_ecef, ecef_to_teme, site_ecef, ecef_to_sez, sez_to_az_el}`, `StateVector`, `GroundSite`, `EARTH_ROTATION_RAD_S`.
- Produces: `observe(&StateVector, &GroundSite) -> Observation`, and `Observation { epoch, az_deg, el_deg, range_km, range_rate_km_s, az_rate_deg_s, el_rate_deg_s, ra_deg, dec_deg, ha_rate_deg_s, dec_rate_deg_s, rate_vs_ground_deg_s, rate_vs_stars_deg_s }` (`Copy, PartialEq`).


- [ ] **Step 1: Write the failing tests**

Create each file below with **only its test module** for now. The implementation goes above it in Step 3.

`crates/orbit-prop/src/observe.rs`:

~~~~rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{EARTH_RADIUS_KM, MU_EARTH};

    const ARCSEC_PER_DEG: f64 = 3600.0;

    fn t0() -> Epoch {
        Epoch::from_utc(2026, 10, 4, 3, 0, 0.0).unwrap()
    }

    /// A satellite in an equatorial prograde circular orbit at `alt_km`,
    /// placed directly above (lat 0, lon 0) at `t0`.
    fn overhead_equatorial(alt_km: f64) -> StateVector {
        let a = EARTH_RADIUS_KM + alt_km;
        let vc = (MU_EARTH / a).sqrt();
        let theta = t0().gmst_rad();
        StateVector {
            epoch: t0(),
            r_km: [a * theta.cos(), a * theta.sin(), 0.0],
            v_km_s: [-vc * theta.sin(), vc * theta.cos(), 0.0],
        }
    }

    #[test]
    fn overhead_pass_rate_matches_scope_eval() {
        // scope-eval's overhead_rate_arcsec_s(500) = v / h = 3140 arcsec/s
        // ignores Earth rotation. At the equator the site moves eastward with
        // the satellite at omega * R, so against the stars the rate is
        // (v - omega R) / h, and v / h is recovered by adding omega R / h back.
        let s = overhead_equatorial(500.0);
        let o = observe(&s, &GroundSite::new(0.0, 0.0, 0.0).unwrap());
        assert!((o.el_deg - 90.0).abs() < 1e-6);
        assert!((o.range_km - 500.0).abs() < 1e-6);
        let site_speed_rate = (EARTH_ROTATION_RAD_S * EARTH_RADIUS_KM / 500.0).to_degrees() * ARCSEC_PER_DEG;
        let v_over_h = o.rate_vs_stars_deg_s * ARCSEC_PER_DEG + site_speed_rate;
        assert!((v_over_h - 3140.0).abs() / 3140.0 < 0.005, "{v_over_h}");
    }

    #[test]
    fn ground_rate_subtracts_earth_rotation_at_satellite_radius() {
        // Relative to the ground the satellite moves at v - omega * a.
        let s = overhead_equatorial(500.0);
        let o = observe(&s, &GroundSite::new(0.0, 0.0, 0.0).unwrap());
        let a = EARTH_RADIUS_KM + 500.0;
        let expected = ((MU_EARTH / a).sqrt() - EARTH_ROTATION_RAD_S * a) / 500.0;
        assert!((o.rate_vs_ground_deg_s - expected.to_degrees()).abs() < 1e-9);
        assert!(o.range_rate_km_s.abs() < 1e-9);
    }

    #[test]
    fn geostationary_satellite_is_still_against_the_ground() {
        let a: f64 = (MU_EARTH / EARTH_ROTATION_RAD_S.powi(2)).cbrt();
        let (r, v) = ecef_to_teme([a, 0.0, 0.0], [0.0; 3], t0());
        let o = observe(&StateVector { epoch: t0(), r_km: r, v_km_s: v }, &GroundSite::new(0.0, 10.0, 0.0).unwrap());
        assert!(o.rate_vs_ground_deg_s < 1e-12 && o.az_rate_deg_s.abs() < 1e-12 && o.el_rate_deg_s.abs() < 1e-12);
        // Against the stars it moves at the sidereal rate, 15.04 arcsec/s
        // times cos(dec), and dec is 0 from the equator, so an equatorial mount's hour-angle axis is stationary.
        assert!((o.rate_vs_stars_deg_s * ARCSEC_PER_DEG - 15.041).abs() < 0.01);
        assert!(o.ha_rate_deg_s.abs() < 1e-9 && o.dec_rate_deg_s.abs() < 1e-12);
    }

    #[test]
    fn axis_rates_agree_with_finite_differences() {
        // A Molniya-like orbit viewed from mid-latitude.
        use crate::keplerian::{KeplerElements, KeplerJ2};
        use crate::propagator::Propagator;
        let el = KeplerElements::from_altitudes(t0(), 600.0, 39_000.0, 63.4, 300.0, 270.0, 330.0).unwrap();
        let prop = KeplerJ2::new(el, "x").unwrap();
        let site = GroundSite::new(45.0, 30.0, 0.0).unwrap();
        let at = |dt: f64| observe(&prop.propagate(t0().add_seconds(dt)).unwrap(), &site);
        let (a, o, b) = (at(-0.5), at(0.0), at(0.5));
        let wrap = |d: f64| (d + 540.0).rem_euclid(360.0) - 180.0;
        assert!((wrap(b.az_deg - a.az_deg) - o.az_rate_deg_s).abs() < 1e-6);
        assert!(((b.el_deg - a.el_deg) - o.el_rate_deg_s).abs() < 1e-6);
        assert!(((b.dec_deg - a.dec_deg) - o.dec_rate_deg_s).abs() < 1e-6);
        assert!(((b.range_km - a.range_km) - o.range_rate_km_s).abs() < 1e-6);
    }
}
~~~~

Register the modules in `crates/orbit-prop/src/lib.rs` (full file):

~~~~rust
//! Satellite propagation and ground-site observation geometry.
//!
//! Assessment-grade (about 0.01 deg); see README.md for accuracy and limits.

pub mod constants;
pub mod error;
pub mod frames;
pub mod keplerian;
pub mod observe;
pub mod propagator;
pub mod sgp4_propagator;
pub mod site;
pub mod state;
pub mod time;
pub mod tle;
// Some helpers are first used by observe and illumination (Tasks 5-6).
#[allow(dead_code)]
mod vec3;

pub use error::OrbitPropError;
pub use keplerian::{KeplerElements, KeplerJ2};
pub use observe::{observe, Observation};
pub use propagator::Propagator;
pub use sgp4_propagator::Sgp4Propagator;
pub use site::GroundSite;
pub use state::StateVector;
pub use time::{Epoch, UtcParts};
pub use tle::Tle;
~~~~

- [ ] **Step 2: Run the tests and confirm they fail**

Run: `cargo test -p orbit-prop observe`
Expected: compile errors (`cannot find ... in this scope`, unresolved imports), because nothing is implemented yet.

- [ ] **Step 3: Write the implementation**

Put each implementation **above** the test module already in the file (files without tests are created whole).

`crates/orbit-prop/src/observe.rs`:

~~~~rust
//! What a ground site sees of a satellite at one instant.
//!
//! All angular rates are computed analytically from the relative position
//! and velocity, not by differencing, so they are exact for the given state.

use crate::constants::EARTH_ROTATION_RAD_S;
use crate::frames::{ecef_to_sez, ecef_to_teme, sez_to_az_el, site_ecef, teme_to_ecef};
use crate::site::GroundSite;
use crate::state::StateVector;
use crate::time::Epoch;
use crate::vec3::{dot, norm, sub, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Observation {
    pub epoch: Epoch,
    /// Azimuth from north through east, `[0, 360)`.
    pub az_deg: f64,
    pub el_deg: f64,
    pub range_km: f64,
    /// Positive when the satellite is moving away.
    pub range_rate_km_s: f64,
    /// Alt-az mount axis rates. The azimuth rate grows without bound as the
    /// satellite approaches the zenith (the alt-az keyhole).
    pub az_rate_deg_s: f64,
    pub el_rate_deg_s: f64,
    /// Topocentric right ascension and declination, true-of-date.
    // TODO(astrometric): aberration, light time, and GCRS rather than TEME.
    pub ra_deg: f64,
    pub dec_deg: f64,
    /// Equatorial mount axis rates (hour angle increases westward).
    pub ha_rate_deg_s: f64,
    pub dec_rate_deg_s: f64,
    /// Total angular rate an Earth-fixed mount must follow.
    pub rate_vs_ground_deg_s: f64,
    /// Total angular rate against the background stars.
    pub rate_vs_stars_deg_s: f64,
}

/// Rate of change of `atan2(y, x)`, rad/s.
fn atan2_rate(y: f64, x: f64, y_dot: f64, x_dot: f64) -> f64 {
    let d = x * x + y * y;
    if d < 1e-18 {
        0.0
    } else {
        (x * y_dot - y * x_dot) / d
    }
}

/// Angular rate of a line of sight `rho` changing at `rho_dot`, rad/s:
/// the component of `rho_dot` perpendicular to `rho`, divided by `|rho|`.
fn transverse_rate(rho: Vec3, rho_dot: Vec3) -> f64 {
    let r = norm(rho);
    let radial = dot(rho, rho_dot) / r;
    (dot(rho_dot, rho_dot) - radial * radial).max(0.0).sqrt() / r
}

pub fn observe(state: &StateVector, site: &GroundSite) -> Observation {
    let t = state.epoch;
    // Earth-fixed geometry: what the mount follows.
    let (r_ecef, v_ecef) = teme_to_ecef(state.r_km, state.v_km_s, t);
    let site_r = site_ecef(site);
    let rho = sub(r_ecef, site_r);
    let sez = ecef_to_sez(rho, site);
    let sez_dot = ecef_to_sez(v_ecef, site);
    let range = norm(rho);
    let range_rate = dot(rho, v_ecef) / range;
    let (az, el) = sez_to_az_el(sez);
    let (north, east, up) = (-sez[0], sez[1], sez[2]);
    let (north_dot, east_dot, up_dot) = (-sez_dot[0], sez_dot[1], sez_dot[2]);
    let horizontal = (north * north + east * east).sqrt();
    let horizontal_dot = if horizontal > 1e-9 { (north * north_dot + east * east_dot) / horizontal } else { 0.0 };
    let az_rate = atan2_rate(east, north, east_dot, north_dot);
    let el_rate = atan2_rate(up, horizontal, up_dot, horizontal_dot);

    // Inertial geometry: motion against the stars.
    let (site_r_teme, site_v_teme) = ecef_to_teme(site_r, [0.0; 3], t);
    let rho_i = sub(state.r_km, site_r_teme);
    let rho_i_dot = sub(state.v_km_s, site_v_teme);
    let (x, y, z) = (rho_i[0], rho_i[1], rho_i[2]);
    let (xd, yd, zd) = (rho_i_dot[0], rho_i_dot[1], rho_i_dot[2]);
    let p = (x * x + y * y).sqrt();
    let p_dot = if p > 1e-9 { (x * xd + y * yd) / p } else { 0.0 };
    let ra_rate = atan2_rate(y, x, yd, xd);
    let dec_rate = atan2_rate(z, p, zd, p_dot);

    Observation {
        epoch: t,
        az_deg: az,
        el_deg: el,
        range_km: range,
        range_rate_km_s: range_rate,
        az_rate_deg_s: az_rate.to_degrees(),
        el_rate_deg_s: el_rate.to_degrees(),
        ra_deg: y.atan2(x).to_degrees().rem_euclid(360.0),
        dec_deg: z.atan2(p).to_degrees(),
        ha_rate_deg_s: (EARTH_ROTATION_RAD_S - ra_rate).to_degrees(),
        dec_rate_deg_s: dec_rate.to_degrees(),
        rate_vs_ground_deg_s: transverse_rate(rho, v_ecef).to_degrees(),
        rate_vs_stars_deg_s: transverse_rate(rho_i, rho_i_dot).to_degrees(),
    }
}
~~~~

- [ ] **Step 4: Run the tests and confirm they pass**

Run: `cargo test -p orbit-prop`
Expected: `test result: ok. 40 passed; 0 failed`, with no compiler warnings.

`overhead_pass_rate_matches_scope_eval` ties the library to `scope-eval`'s existing `overhead_rate_arcsec_s(500) = 3140"/s`.

- [ ] **Step 5: Commit**

~~~~bash
git add crates/orbit-prop
git commit -m "Add observation geometry with analytic axis rates

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
~~~~


---

### Task 6: Sun, Moon and illumination

**Files:**
- Create: `crates/orbit-prop/src/{sun_moon,illumination}.rs`
- Modify: `crates/orbit-prop/src/lib.rs`

**Interfaces:**
- Consumes: `Epoch::days_since_j2000`, `frames`, `vec3::{angle_between, dot, norm, scale, sub}`, constants `AU_KM`, `SUN_RADIUS_KM`, `EARTH_RADIUS_KM`.
- Produces:
  - `sun_moon::{sun_position_km(Epoch) -> [f64; 3], moon_position_km(Epoch) -> [f64; 3]}` (geocentric, true-of-date, treated as TEME).
  - `Lighting { Sunlit, Penumbra, Umbra }`, `illumination::{lighting(sat_r, sun_r) -> Lighting, phase_angle_deg(sat_r, observer_r, sun_r) -> f64, sun_elevation_deg(&GroundSite, Epoch) -> f64}`.


- [ ] **Step 1: Write the failing tests**

Create each file below with **only its test module** for now. The implementation goes above it in Step 3.

`crates/orbit-prop/src/sun_moon.rs`:

~~~~rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::vec3::norm;

    fn ra_dec(r: Vec3) -> (f64, f64) {
        (r[1].atan2(r[0]).to_degrees().rem_euclid(360.0), (r[2] / norm(r)).asin().to_degrees())
    }

    #[test]
    fn sun_matches_meeus_example_25a() {
        // Meeus, Astronomical Algorithms, Example 25.a: 1992 October 13.0 TD,
        // apparent RA 13h13m31.4s = 198.380833 deg, Dec -7d47m01s = -7.783611 deg,
        // distance 0.99760775 AU.
        let r = sun_position_km(Epoch::from_utc(1992, 10, 13, 0, 0, 0.0).unwrap());
        let (ra, dec) = ra_dec(r);
        assert!((ra - 198.380_833).abs() < 0.01, "ra {ra}");
        assert!((dec - (-7.783_611)).abs() < 0.01, "dec {dec}");
        assert!((norm(r) / AU_KM - 0.997_607_75).abs() < 1e-4);
    }

    #[test]
    fn moon_matches_meeus_example_47a() {
        // Meeus Example 47.a: 1992 April 12.0 TD, apparent RA 134.688470 deg,
        // Dec 13.768368 deg, distance 368409.7 km.
        let r = moon_position_km(Epoch::from_utc(1992, 4, 12, 0, 0, 0.0).unwrap());
        let (ra, dec) = ra_dec(r);
        assert!((ra - 134.688_470).abs() < 0.3, "ra {ra}");
        assert!((dec - 13.768_368).abs() < 0.3, "dec {dec}");
        assert!((norm(r) - 368_409.7).abs() < 1000.0, "dist {}", norm(r));
    }
}
~~~~

`crates/orbit-prop/src/illumination.rs`:

~~~~rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::AU_KM;

    const SUN: Vec3 = [AU_KM, 0.0, 0.0];

    #[test]
    fn shadow_classification() {
        assert_eq!(lighting([7000.0, 0.0, 0.0], SUN), Lighting::Sunlit, "day side");
        assert_eq!(lighting([-7000.0, 0.0, 0.0], SUN), Lighting::Umbra, "directly behind Earth");
        assert_eq!(lighting([-7000.0, 7000.0, 0.0], SUN), Lighting::Sunlit, "beside Earth, outside the shadow");
        // GEO near equinox midnight: 42164 km behind, on the axis -> umbra.
        assert_eq!(lighting([-42_164.0, 0.0, 0.0], SUN), Lighting::Umbra);
        // Just outside the umbra edge at GEO distance but inside the penumbra.
        let alpha_u = ((SUN_RADIUS_KM - EARTH_RADIUS_KM) / AU_KM).asin();
        let edge = EARTH_RADIUS_KM - 42_164.0 * alpha_u.tan();
        assert_eq!(lighting([-42_164.0, edge + 20.0, 0.0], SUN), Lighting::Penumbra);
    }

    #[test]
    fn phase_angle_extremes() {
        let observer = [6378.0, 0.0, 0.0];
        assert!(phase_angle_deg([7000.0, 0.0, 0.0], observer, SUN) > 179.9, "sat between observer and Sun");
        assert!(phase_angle_deg([5000.0, 0.0, 0.0], observer, SUN) < 0.1, "observer between sat and Sun");
        assert!((phase_angle_deg([6378.0, 1000.0, 0.0], observer, SUN) - 90.0).abs() < 0.1);
    }

    #[test]
    fn sun_is_up_at_noon_and_down_at_midnight() {
        // Greenwich on the 2026 June solstice: high at noon, well below at midnight.
        let site = GroundSite::new(51.48, 0.0, 0.0).unwrap();
        let noon = sun_elevation_deg(&site, Epoch::from_utc(2026, 6, 21, 12, 0, 0.0).unwrap());
        let midnight = sun_elevation_deg(&site, Epoch::from_utc(2026, 6, 21, 0, 0, 0.0).unwrap());
        // Expected noon elevation 90 - 51.48 + 23.44 = 61.96 deg; midnight -(51.48 - ... ) about -15 deg.
        assert!((noon - 61.96).abs() < 0.3, "noon {noon}");
        assert!((midnight - (-15.1)).abs() < 0.5, "midnight {midnight}");
    }
}
~~~~

Register the modules in `crates/orbit-prop/src/lib.rs` (full file):

~~~~rust
//! Satellite propagation and ground-site observation geometry.
//!
//! Assessment-grade (about 0.01 deg); see README.md for accuracy and limits.

pub mod constants;
pub mod error;
pub mod frames;
pub mod illumination;
pub mod keplerian;
pub mod observe;
pub mod propagator;
pub mod sgp4_propagator;
pub mod site;
pub mod state;
pub mod sun_moon;
pub mod time;
pub mod tle;
// Some helpers are first used by observe and illumination (Tasks 5-6).
#[allow(dead_code)]
mod vec3;

pub use error::OrbitPropError;
pub use illumination::Lighting;
pub use keplerian::{KeplerElements, KeplerJ2};
pub use observe::{observe, Observation};
pub use propagator::Propagator;
pub use sgp4_propagator::Sgp4Propagator;
pub use site::GroundSite;
pub use state::StateVector;
pub use time::{Epoch, UtcParts};
pub use tle::Tle;
~~~~

- [ ] **Step 2: Run the tests and confirm they fail**

Run: `cargo test -p orbit-prop `
Expected: compile errors (`cannot find ... in this scope`, unresolved imports), because nothing is implemented yet.

- [ ] **Step 3: Write the implementation**

Put each implementation **above** the test module already in the file (files without tests are created whole).

`crates/orbit-prop/src/sun_moon.rs`:

~~~~rust
//! Low-precision Sun and Moon positions.
//!
//! **Accuracy: assessment grade.** The Sun is good to about 0.01 deg and the
//! Moon to about 0.3 deg between 1950 and 2050, from the low-precision
//! formulas in the Astronomical Almanac (section C for the Sun, section D
//! for the Moon). That is plenty for shadow, phase angle, twilight and Moon
//! avoidance, but not for astrometry.
//!
//! Both return geocentric position vectors in km, in the mean-equator,
//! true-of-date frame, which is treated as TEME.
// TODO(astrometric): replace with a JPL ephemeris (e.g. DE440) in GCRS.

use crate::constants::{AU_KM, EARTH_RADIUS_KM, J2000_JD};
use crate::time::Epoch;
use crate::vec3::Vec3;

fn sin_d(deg: f64) -> f64 {
    deg.to_radians().sin()
}

fn cos_d(deg: f64) -> f64 {
    deg.to_radians().cos()
}

/// Ecliptic longitude and latitude (deg) and distance (km) to an equatorial vector.
fn ecliptic_to_equatorial(lon_deg: f64, lat_deg: f64, dist_km: f64, obliquity_deg: f64) -> Vec3 {
    let (x, y, z) = (cos_d(lat_deg) * cos_d(lon_deg), cos_d(lat_deg) * sin_d(lon_deg), sin_d(lat_deg));
    let (se, ce) = (sin_d(obliquity_deg), cos_d(obliquity_deg));
    [dist_km * x, dist_km * (ce * y - se * z), dist_km * (se * y + ce * z)]
}

/// Geocentric Sun position, km.
pub fn sun_position_km(t: Epoch) -> Vec3 {
    let n = t.days_since_j2000();
    let l = 280.460 + 0.985_647_4 * n;
    let g = 357.528 + 0.985_600_3 * n;
    let lambda = l + 1.915 * sin_d(g) + 0.020 * sin_d(2.0 * g);
    let eps = 23.439 - 0.000_000_4 * n;
    let r_au = 1.000_14 - 0.016_71 * cos_d(g) - 0.000_14 * cos_d(2.0 * g);
    ecliptic_to_equatorial(lambda, 0.0, r_au * AU_KM, eps)
}

/// Geocentric Moon position, km.
pub fn moon_position_km(t: Epoch) -> Vec3 {
    let tc = t.days_since_j2000() / 36_525.0;
    let lambda = 218.32 + 481_267.881 * tc + 6.29 * sin_d(135.0 + 477_198.87 * tc)
        - 1.27 * sin_d(259.3 - 413_335.36 * tc)
        + 0.66 * sin_d(235.7 + 890_534.22 * tc)
        + 0.21 * sin_d(269.9 + 954_397.74 * tc)
        - 0.19 * sin_d(357.5 + 35_999.05 * tc)
        - 0.11 * sin_d(186.5 + 966_404.03 * tc);
    let beta = 5.13 * sin_d(93.3 + 483_202.02 * tc) + 0.28 * sin_d(228.2 + 960_400.89 * tc)
        - 0.28 * sin_d(318.3 + 6_003.15 * tc)
        - 0.17 * sin_d(217.6 - 407_332.21 * tc);
    let parallax = 0.9508
        + 0.0518 * cos_d(135.0 + 477_198.87 * tc)
        + 0.0095 * cos_d(259.3 - 413_335.36 * tc)
        + 0.0078 * cos_d(235.7 + 890_534.22 * tc)
        + 0.0028 * cos_d(269.9 + 954_397.74 * tc);
    let dist = EARTH_RADIUS_KM / sin_d(parallax);
    let eps = 23.439 - 0.000_000_4 * (t.jd() - J2000_JD);
    ecliptic_to_equatorial(lambda, beta, dist, eps)
}
~~~~

`crates/orbit-prop/src/illumination.rs`:

~~~~rust
//! Lighting: Earth's shadow, phase angle, and how dark the site is.

use crate::constants::{EARTH_RADIUS_KM, SUN_RADIUS_KM};
use crate::frames::{ecef_to_sez, sez_to_az_el, site_ecef, teme_to_ecef};
use crate::site::GroundSite;
use crate::sun_moon::sun_position_km;
use crate::time::Epoch;
use crate::vec3::{angle_between, dot, norm, scale, sub, Vec3};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lighting {
    Sunlit,
    /// Partly shadowed: the Earth covers part of the Sun's disk.
    Penumbra,
    /// Fully shadowed.
    Umbra,
}

/// Shadow state of a satellite at `sat_r` (km, geocentric) with the Sun at
/// `sun_r` (km, geocentric), using a conical shadow model (Vallado
/// algorithm 34). Atmospheric refraction into the shadow is ignored.
pub fn lighting(sat_r: Vec3, sun_r: Vec3) -> Lighting {
    let sun_dist = norm(sun_r);
    let sun_hat = scale(sun_r, 1.0 / sun_dist);
    let along = dot(sat_r, sun_hat);
    if along >= 0.0 {
        return Lighting::Sunlit; // on the day side of the terminator plane
    }
    let behind = -along; // distance behind the Earth along the shadow axis
    let off_axis = norm(sub(sat_r, scale(sun_hat, along)));
    let alpha_umbra = ((SUN_RADIUS_KM - EARTH_RADIUS_KM) / sun_dist).asin();
    let alpha_penumbra = ((SUN_RADIUS_KM + EARTH_RADIUS_KM) / sun_dist).asin();
    let umbra_radius = EARTH_RADIUS_KM - behind * alpha_umbra.tan();
    let penumbra_radius = EARTH_RADIUS_KM + behind * alpha_penumbra.tan();
    if off_axis < umbra_radius {
        Lighting::Umbra
    } else if off_axis < penumbra_radius {
        Lighting::Penumbra
    } else {
        Lighting::Sunlit
    }
}

/// Sun-satellite-observer angle, degrees. 0 is fully lit as seen from the
/// site (Sun behind the observer); 180 is looking at the unlit side.
/// All vectors geocentric, km, same frame.
pub fn phase_angle_deg(sat_r: Vec3, observer_r: Vec3, sun_r: Vec3) -> f64 {
    angle_between(sub(sun_r, sat_r), sub(observer_r, sat_r)).to_degrees()
}

/// Elevation of the Sun's centre above the site's horizon, degrees.
pub fn sun_elevation_deg(site: &GroundSite, t: Epoch) -> f64 {
    let (sun_ecef, _) = teme_to_ecef(sun_position_km(t), [0.0; 3], t);
    let (_, el) = sez_to_az_el(ecef_to_sez(sub(sun_ecef, site_ecef(site)), site));
    el
}
~~~~

- [ ] **Step 4: Run the tests and confirm they pass**

Run: `cargo test -p orbit-prop`
Expected: `test result: ok. 45 passed; 0 failed`, with no compiler warnings.

- [ ] **Step 5: Commit**

~~~~bash
git add crates/orbit-prop
git commit -m "Add low-precision Sun and Moon and illumination

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
~~~~


---

### Task 7: Pass finding and the crate README

**Files:**
- Create: `crates/orbit-prop/src/passes.rs`, `crates/orbit-prop/README.md`
- Modify: `crates/orbit-prop/src/lib.rs`

**Interfaces:**
- Consumes: `Propagator`, `observe`, `lighting`, `sun_elevation_deg`, `sun_position_km`, `DARK_SUN_ELEVATION_DEG`, `MAX_SEARCH_WINDOW_S`; test-only `frames::ecef_to_teme`, `tle::tests::ISS`, `Sgp4Propagator`, `KeplerJ2`.
- Produces:
  - `PassSearch { start: Epoch, end: Epoch, min_el_deg: f64 }`
  - `PassLighting { Sunlit, Partial, Eclipsed }` and `PassDarkness { Dark, Partial, Daylight }`
  - `Pass { rise, culmination, set, clipped_start, clipped_end, max_el_deg, peak_az_rate_deg_s, peak_el_rate_deg_s, peak_az_accel_deg_s2, peak_el_accel_deg_s2, peak_ha_rate_deg_s, peak_dec_rate_deg_s, peak_ha_accel_deg_s2, peak_dec_accel_deg_s2, peak_rate_vs_ground_deg_s, lighting: PassLighting, site_dark: PassDarkness }` (`Clone, PartialEq`), plus `Pass::duration_s(&self) -> f64`
  - `PassResult { passes: Vec<Pass>, error: Option<OrbitPropError> }`
  - `find_passes(&dyn Propagator, &GroundSite, &PassSearch) -> PassResult`


- [ ] **Step 1: Write the failing tests**

Create each file below with **only its test module** for now. The implementation goes above it in Step 3.

`crates/orbit-prop/src/passes.rs`:

~~~~rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::EARTH_RADIUS_KM;
    use crate::frames::ecef_to_teme;
    use crate::state::StateVector;
    use crate::tle::Tle;
    use crate::Sgp4Propagator;

    fn t0() -> Epoch {
        Epoch::from_utc(2026, 10, 4, 0, 0, 0.0).unwrap()
    }

    /// An equatorial satellite whose Earth-fixed longitude moves at a
    /// constant rate, so pass geometry over an equatorial site is analytic.
    /// Fails with an SGP4-style error after `fail_after_s`, if set.
    struct EarthFixedCircle {
        radius_km: f64,
        lon0_deg: f64,
        rate_deg_s: f64,
        fail_after_s: Option<f64>,
    }

    impl Propagator for EarthFixedCircle {
        fn propagate(&self, t: Epoch) -> Result<StateVector, OrbitPropError> {
            let dt = t.seconds_since(&t0());
            if self.fail_after_s.is_some_and(|f| dt > f) {
                return Err(OrbitPropError::Sgp4("decayed".into()));
            }
            let lon = (self.lon0_deg + self.rate_deg_s * dt).to_radians();
            let w = self.rate_deg_s.to_radians();
            let r = [self.radius_km * lon.cos(), self.radius_km * lon.sin(), 0.0];
            let v = [-self.radius_km * w * lon.sin(), self.radius_km * w * lon.cos(), 0.0];
            let (rt, vt) = ecef_to_teme(r, v, t);
            Ok(StateVector { epoch: t, r_km: rt, v_km_s: vt })
        }
        fn period_s(&self) -> f64 {
            if self.rate_deg_s == 0.0 { 86_164.0 } else { 360.0 / self.rate_deg_s.abs() }
        }
        fn label(&self) -> &str {
            "test"
        }
    }

    fn equator() -> GroundSite {
        GroundSite::new(0.0, 0.0, 0.0).unwrap()
    }

    fn search(hours: f64, min_el: f64) -> PassSearch {
        PassSearch { start: t0(), end: t0().add_seconds(hours * 3600.0), min_el_deg: min_el }
    }

    /// Geocentric half-angle of the arc above `min_el` (Vallado eq. 11-4 rearranged).
    fn half_arc_deg(radius_km: f64, min_el_deg: f64) -> f64 {
        let e = min_el_deg.to_radians();
        ((EARTH_RADIUS_KM * e.cos() / radius_km).acos() - e).to_degrees()
    }

    #[test]
    fn overhead_pass_rise_and_set_match_geometry() {
        let sat = EarthFixedCircle { radius_km: EARTH_RADIUS_KM + 500.0, lon0_deg: -30.0, rate_deg_s: 0.06, fail_after_s: None };
        let r = find_passes(&sat, &equator(), &search(1.0, 10.0));
        assert_eq!(r.error, None);
        assert_eq!(r.passes.len(), 1);
        let p = &r.passes[0];
        let psi = half_arc_deg(EARTH_RADIUS_KM + 500.0, 10.0);
        assert!((p.rise.seconds_since(&t0()) - (30.0 - psi) / 0.06).abs() < 1.0);
        assert!((p.set.seconds_since(&t0()) - (30.0 + psi) / 0.06).abs() < 1.0);
        assert!((p.culmination.seconds_since(&t0()) - 500.0).abs() < 0.5);
        assert!(p.max_el_deg > 89.99);
        assert!(!p.clipped_start && !p.clipped_end);
        // At the zenith the ground rate is (a * omega) / h.
        let expected = 0.06 * (EARTH_RADIUS_KM + 500.0) / 500.0;
        assert!((p.peak_rate_vs_ground_deg_s - expected).abs() / expected < 1e-3, "{}", p.peak_rate_vs_ground_deg_s);
    }

    #[test]
    fn near_zenith_pass_spins_the_azimuth_axis() {
        // Seen from half a degree north of the track the pass culminates near 84 deg,
        // and the azimuth axis must turn far faster than the target moves.
        let sat = EarthFixedCircle { radius_km: EARTH_RADIUS_KM + 500.0, lon0_deg: -30.0, rate_deg_s: 0.06, fail_after_s: None };
        let site = GroundSite::new(0.5, 0.0, 0.0).unwrap();
        let p = &find_passes(&sat, &site, &search(1.0, 10.0)).passes[0];
        assert!(p.max_el_deg > 80.0 && p.max_el_deg < 89.0, "{}", p.max_el_deg);
        assert!(p.peak_az_rate_deg_s > 5.0 * p.peak_rate_vs_ground_deg_s);
        assert!(p.peak_az_accel_deg_s2 > p.peak_el_accel_deg_s2);
    }

    #[test]
    fn search_starting_mid_pass_is_clipped_at_the_start() {
        // At t0 the satellite is 5 deg west of the site, already well up.
        let sat = EarthFixedCircle { radius_km: EARTH_RADIUS_KM + 500.0, lon0_deg: -5.0, rate_deg_s: 0.06, fail_after_s: None };
        let r = find_passes(&sat, &equator(), &search(1.0, 10.0));
        assert_eq!(r.passes.len(), 1);
        let p = &r.passes[0];
        assert!(p.clipped_start && !p.clipped_end);
        assert_eq!(p.rise, t0());
        let psi = half_arc_deg(EARTH_RADIUS_KM + 500.0, 10.0);
        assert!((p.set.seconds_since(&t0()) - (5.0 + psi) / 0.06).abs() < 1.0);
    }

    #[test]
    fn polar_orbit_over_a_polar_site_passes_every_revolution() {
        use crate::keplerian::{KeplerElements, KeplerJ2};
        let el = KeplerElements::from_altitudes(t0(), 800.0, 800.0, 90.0, 0.0, 0.0, 0.0).unwrap();
        let prop = KeplerJ2::new(el, "polar").unwrap();
        let site = GroundSite::new(89.9, 0.0, 0.0).unwrap();
        let r = find_passes(&prop, &site, &search(24.0, 10.0));
        assert_eq!(r.error, None);
        let revolutions = 86_400.0 / prop.period_s();
        assert!((r.passes.len() as f64 - revolutions).abs() <= 1.0, "{} passes, {revolutions:.1} revs", r.passes.len());
        assert!(r.passes.iter().all(|p| p.max_el_deg > 60.0));
    }

    #[test]
    fn stationary_satellite_overhead_is_one_clipped_pass() {
        let geo = EarthFixedCircle { radius_km: 42_164.0, lon0_deg: 0.0, rate_deg_s: 0.0, fail_after_s: None };
        let r = find_passes(&geo, &equator(), &search(6.0, 10.0));
        assert_eq!(r.error, None);
        assert_eq!(r.passes.len(), 1);
        let p = &r.passes[0];
        assert!(p.clipped_start && p.clipped_end);
        assert_eq!(p.rise, t0());
        assert!((p.duration_s() - 6.0 * 3600.0).abs() < 1e-6);
        assert!(p.max_el_deg > 89.99 && p.peak_rate_vs_ground_deg_s < 1e-9);
    }

    #[test]
    fn satellite_that_never_rises_gives_no_passes() {
        let geo = EarthFixedCircle { radius_km: 42_164.0, lon0_deg: 180.0, rate_deg_s: 0.0, fail_after_s: None };
        assert_eq!(find_passes(&geo, &equator(), &search(24.0, 10.0)), PassResult { passes: vec![], error: None });
    }

    #[test]
    fn propagation_failure_keeps_earlier_passes() {
        // A pass every 3000 s; failure at 5000 s, after the second pass.
        let sat = EarthFixedCircle { radius_km: EARTH_RADIUS_KM + 500.0, lon0_deg: -30.0, rate_deg_s: 0.12, fail_after_s: Some(5000.0) };
        let r = find_passes(&sat, &equator(), &search(3.0, 10.0));
        assert_eq!(r.passes.len(), 2);
        assert!(matches!(r.error, Some(OrbitPropError::Sgp4(_))));
    }

    #[test]
    fn decaying_tle_reports_an_sgp4_error() {
        let tle = Tle::parse(
            "1 28350U 04020A   06167.21788666  .16154492  76267-5  18678-3 0  8894
2 28350  64.9977 345.6130 0024870 260.7578  99.9590 16.47856722116490",
        )
        .unwrap();
        let prop = Sgp4Propagator::new(&tle).unwrap();
        let s = PassSearch { start: tle.epoch, end: tle.epoch.add_seconds(2.0 * 86_400.0), min_el_deg: 10.0 };
        let r = find_passes(&prop, &GroundSite::new(40.0, -75.0, 0.0).unwrap(), &s);
        assert!(matches!(r.error, Some(OrbitPropError::Sgp4(_))));
        assert!(r.passes.iter().all(|p| p.set.seconds_since(&tle.epoch) < 1500.0 * 60.0));
    }

    #[test]
    fn iss_passes_are_plausible() {
        let tle = Tle::parse(crate::tle::tests::ISS).unwrap();
        let prop = Sgp4Propagator::new(&tle).unwrap();
        let s = PassSearch { start: tle.epoch, end: tle.epoch.add_seconds(86_400.0), min_el_deg: 10.0 };
        let r = find_passes(&prop, &GroundSite::new(40.0, -75.0, 0.0).unwrap(), &s);
        assert_eq!(r.error, None);
        assert!(r.passes.len() >= 2, "{} passes", r.passes.len());
        for p in &r.passes {
            assert!(p.rise.seconds_since(&p.culmination) < 0.0 && p.culmination.seconds_since(&p.set) < 0.0);
            assert!(p.duration_s() > 30.0 && p.duration_s() < 15.0 * 60.0, "{}", p.duration_s());
            assert!(p.max_el_deg >= 10.0 && p.peak_rate_vs_ground_deg_s < 2.0);
        }
    }

    #[test]
    fn invalid_windows_are_rejected() {
        let sat = EarthFixedCircle { radius_km: 42_164.0, lon0_deg: 0.0, rate_deg_s: 0.0, fail_after_s: None };
        let bad = |s: PassSearch| matches!(find_passes(&sat, &equator(), &s).error, Some(OrbitPropError::InvalidTime(_)));
        assert!(bad(PassSearch { start: t0(), end: t0(), min_el_deg: 10.0 }), "empty window");
        assert!(bad(search(31.0 * 24.0, 10.0)), "over 30 days");
        assert!(bad(search(1.0, 90.0)), "min elevation 90");
        assert!(bad(search(1.0, f64::NAN)), "NaN min elevation");
    }

    #[test]
    fn summary_of_flags() {
        assert_eq!(summarize(&[true, true], 'a', 'n', 'm'), 'a');
        assert_eq!(summarize(&[false, false], 'a', 'n', 'm'), 'n');
        assert_eq!(summarize(&[true, false], 'a', 'n', 'm'), 'm');
    }
}
~~~~

Create `crates/orbit-prop/README.md` (it becomes the crate documentation, and its example runs as a doctest):

~~~~markdown
# orbit-prop

Satellite propagation and ground-site observation geometry for
`scope-eval` and the simulator that will build on it.

* **Two orbit models behind one interface.** `Sgp4Propagator` propagates
  catalog TLEs with SGP4, and `KeplerJ2` propagates what-if orbits
  (Keplerian with secular J2 drift). Both implement the `Propagator`
  trait, and everything downstream depends only on that trait.
* **Observer geometry.** `observe` gives azimuth, elevation, range, range
  rate, alt-az and equatorial axis rates, and the angular rate against the
  ground and against the stars, as seen from a `GroundSite`.
* **Lighting.** Low-precision Sun and Moon positions, Earth shadow
  (umbra/penumbra), phase angle, and Sun elevation at the site.
* **Passes.** `find_passes` returns rise, culmination and set times, peak
  axis rates and accelerations, and lighting and darkness for each pass.

The only dependency is the pure-Rust [`sgp4`](https://crates.io/crates/sgp4)
crate.

## Example

```rust
use orbit_prop::{find_passes, Epoch, GroundSite, PassSearch, Sgp4Propagator, Tle};

let tle = Tle::parse("ISS (ZARYA)
1 25544U 98067A   08264.51782528 -.00002182  00000-0 -11606-4 0  2927
2 25544  51.6416 247.4627 0006703 130.5360 325.0288 15.72125391563537")?;
let prop = Sgp4Propagator::new(&tle)?;
let site = GroundSite::new(40.0, -75.0, 100.0)?;
let search = PassSearch { start: tle.epoch, end: tle.epoch.add_seconds(86_400.0), min_el_deg: 10.0 };
let result = find_passes(&prop, &site, &search);
for p in &result.passes {
    println!("{} max el {:.1} deg, peak az rate {:.2} deg/s", p.rise, p.max_el_deg, p.peak_az_rate_deg_s);
}
# Ok::<(), orbit_prop::OrbitPropError>(())
```

## Models

| Module | Model | Reference |
|---|---|---|
| `time` | Split Julian date, UTC; GMST by the IAU-1982 expression | Vallado, *Fundamentals of Astrodynamics*, eq. 3-47 |
| `frames` | TEME to ECEF by GMST rotation; WGS-84 geodetic to ECEF; topocentric SEZ | Vallado sec. 3.4 and 3.7 |
| `sgp4_propagator` | SGP4/SDP4 via the `sgp4` crate, AFSPC compatibility mode | Vallado et al. 2006, "Revisiting Spacetrack Report #3" |
| `keplerian` | Two-body orbit with secular J2 rates of node, perigee and mean anomaly | Vallado eq. 9-41 |
| `sun_moon` | Low-precision Sun (about 0.01 deg) and Moon (about 0.3 deg) | Astronomical Almanac, sections C and D |
| `illumination` | Conical umbra and penumbra | Vallado algorithm 34 |
| `passes` | Coarse scan, bisection to 0.1 s, dense sampling for peaks | this crate |

SGP4 runs in AFSPC compatibility mode because catalog TLEs are fitted with
that implementation. The `sgp4` crate's default "improved" mode differs
from it by up to tens of metres.

`KeplerJ2` drifts from a real satellite by kilometres per day: it has no
drag, no short-period J2 terms and no third bodies. Use it for "what would
this kind of orbit look like from my site", and a TLE for real objects.

## Units and conventions

Units are in the field names: `_km`, `_km_s`, `_deg`, `_deg_s`, `_deg_s2`,
`_m`, `_s`. Azimuth runs from north through east. Longitude is positive
east. Times are UTC, and leap seconds are ignored.

## Errors

No function panics on user input. Everything fallible returns
`OrbitPropError`: `TleFormat` (with line and column), `Sgp4` (for example
a decayed orbit), `InvalidElements`, `InvalidSite`, `InvalidTime`, and
`NoConvergence`. `find_passes` returns a `PassResult` that keeps the passes
found before an error, plus the error.

## Accuracy

This crate is **assessment grade, about 0.01 deg** in azimuth and
elevation. That is good enough for pass timing, rates, visibility and
lighting, and smaller than typical TLE error (about 1 km, several
arcminutes for a LEO object seen from the ground).

## Known limitations and future work

**Upgrade to astrometric grade before using this for astrometry, pointing
models or orbit determination.** Each simplification is marked
`TODO(astrometric)` in the source. The upgrade needs:

* IAU-2006/2000A precession and nutation (TEME to GCRS) instead of the
  GMST-only rotation,
* UT1 - UTC and polar motion from IERS Earth-orientation data,
* annual and diurnal aberration and light-time correction,
* atmospheric refraction in elevation (not modelled; about 0.5 deg at the
  horizon, which also shifts rise and set times),
* a higher-precision Sun and Moon ephemeris, for example JPL DE440.

Other limits:

* The pass search steps at `period / 60`, clamped to 10-300 s, so a pass
  shorter than that step can be missed.
* Lighting ignores refraction into the shadow, and the Moon's light.
~~~~

Register the module in `crates/orbit-prop/src/lib.rs` (full file). The `vec3` `allow(dead_code)` is gone because every helper is now used.

~~~~rust
// The crate documentation is the README, so its example is compiled and run by `cargo test`.
#![doc = include_str!("../README.md")]

pub mod constants;
pub mod error;
pub mod frames;
pub mod illumination;
pub mod keplerian;
pub mod observe;
pub mod passes;
pub mod propagator;
pub mod sgp4_propagator;
pub mod site;
pub mod state;
pub mod sun_moon;
pub mod time;
pub mod tle;
mod vec3;

pub use error::OrbitPropError;
pub use illumination::Lighting;
pub use keplerian::{KeplerElements, KeplerJ2};
pub use observe::{observe, Observation};
pub use passes::{find_passes, Pass, PassDarkness, PassLighting, PassResult, PassSearch};
pub use propagator::Propagator;
pub use sgp4_propagator::Sgp4Propagator;
pub use site::GroundSite;
pub use state::StateVector;
pub use time::{Epoch, UtcParts};
pub use tle::Tle;
~~~~

- [ ] **Step 2: Run the tests and confirm they fail**

Run: `cargo test -p orbit-prop passes`
Expected: compile errors (`cannot find ... in this scope`, unresolved imports), because nothing is implemented yet.

- [ ] **Step 3: Write the implementation**

Put each implementation **above** the test module already in the file (files without tests are created whole).

`crates/orbit-prop/src/passes.rs`:

~~~~rust
//! Finding passes of a satellite over a ground site.
//!
//! 1. Scan elevation minus the minimum elevation at a coarse step of
//!    `period / 60`, clamped to 10-300 s.
//! 2. Refine each sign change by bisection to 0.1 s.
//! 3. Sample the pass densely (1 s, or `duration / 7200` for passes longer
//!    than two hours) and at 0.1 s within 30 s of culmination, recording
//!    peak axis rates, accelerations (differences of the analytic rates),
//!    lighting and site darkness.
//! 4. Refine culmination by golden-section search on elevation.
//!
//! A pass shorter than the coarse step can be missed. For LEO that means
//! grazing passes that never get far above the minimum elevation.

use crate::constants::{DARK_SUN_ELEVATION_DEG, MAX_SEARCH_WINDOW_S};
use crate::error::OrbitPropError;
use crate::illumination::{lighting, sun_elevation_deg, Lighting};
use crate::observe::{observe, Observation};
use crate::propagator::Propagator;
use crate::site::GroundSite;
use crate::sun_moon::sun_position_km;
use crate::time::Epoch;

const REFINE_TOLERANCE_S: f64 = 0.1;
const DENSE_STEP_S: f64 = 1.0;
const MAX_DENSE_SAMPLES: f64 = 7200.0;
const FINE_HALF_WINDOW_S: f64 = 30.0;
const FINE_STEP_S: f64 = 0.1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PassSearch {
    pub start: Epoch,
    pub end: Epoch,
    /// Elevation above which the satellite counts as "in the pass", degrees.
    pub min_el_deg: f64,
}

/// Whether the satellite is sunlit during the pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassLighting {
    /// Sunlit for the whole pass.
    Sunlit,
    /// Enters or leaves the Earth's shadow during the pass.
    Partial,
    /// In shadow (umbra or penumbra) for the whole pass.
    Eclipsed,
}

/// Whether the site is dark (Sun below `DARK_SUN_ELEVATION_DEG`) during the pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassDarkness {
    Dark,
    Partial,
    /// The Sun is above the dark threshold for the whole pass. Includes twilight.
    Daylight,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pass {
    pub rise: Epoch,
    pub culmination: Epoch,
    pub set: Epoch,
    /// Already above the minimum elevation at the start of the search.
    pub clipped_start: bool,
    /// Still above the minimum elevation at the end of the search.
    pub clipped_end: bool,
    pub max_el_deg: f64,
    pub peak_az_rate_deg_s: f64,
    pub peak_el_rate_deg_s: f64,
    pub peak_az_accel_deg_s2: f64,
    pub peak_el_accel_deg_s2: f64,
    pub peak_ha_rate_deg_s: f64,
    pub peak_dec_rate_deg_s: f64,
    pub peak_ha_accel_deg_s2: f64,
    pub peak_dec_accel_deg_s2: f64,
    pub peak_rate_vs_ground_deg_s: f64,
    pub lighting: PassLighting,
    pub site_dark: PassDarkness,
}

impl Pass {
    pub fn duration_s(&self) -> f64 {
        self.set.seconds_since(&self.rise)
    }
}

/// Passes found, plus the error that stopped the search early, if any.
/// Passes completed before the error are kept.
#[derive(Debug, Clone, PartialEq)]
pub struct PassResult {
    pub passes: Vec<Pass>,
    pub error: Option<OrbitPropError>,
}

pub fn find_passes(prop: &dyn Propagator, site: &GroundSite, search: &PassSearch) -> PassResult {
    let mut passes = Vec::new();
    let error = validate(search).and_then(|_| scan(prop, site, search, &mut passes)).err();
    PassResult { passes, error }
}

fn validate(search: &PassSearch) -> Result<(), OrbitPropError> {
    let span = search.end.seconds_since(&search.start);
    // Negated so a NaN span is rejected too.
    if !(span > 0.0) {
        return Err(OrbitPropError::InvalidTime("the search must end after it starts".into()));
    }
    if span > MAX_SEARCH_WINDOW_S {
        return Err(OrbitPropError::InvalidTime(format!(
            "search window of {:.1} days exceeds the {:.0}-day limit",
            span / 86_400.0,
            MAX_SEARCH_WINDOW_S / 86_400.0
        )));
    }
    if !search.min_el_deg.is_finite() || !(-90.0..90.0).contains(&search.min_el_deg) {
        return Err(OrbitPropError::InvalidTime(format!(
            "minimum elevation {} deg must be at least -90 and below 90",
            search.min_el_deg
        )));
    }
    Ok(())
}

fn look(prop: &dyn Propagator, site: &GroundSite, t: Epoch) -> Result<Observation, OrbitPropError> {
    Ok(observe(&prop.propagate(t)?, site))
}

fn coarse_step_s(prop: &dyn Propagator) -> f64 {
    let period = prop.period_s();
    if period.is_finite() && period > 0.0 {
        (period / 60.0).clamp(10.0, 300.0)
    } else {
        60.0
    }
}

fn scan(prop: &dyn Propagator, site: &GroundSite, search: &PassSearch, out: &mut Vec<Pass>) -> Result<(), OrbitPropError> {
    let above = |t: Epoch| -> Result<f64, OrbitPropError> { Ok(look(prop, site, t)?.el_deg - search.min_el_deg) };
    let step = coarse_step_s(prop);
    let mut t0 = search.start;
    let mut f0 = above(t0)?;
    let mut rise: Option<(Epoch, bool)> = if f0 > 0.0 { Some((t0, true)) } else { None };
    while search.end.seconds_since(&t0) > 0.0 {
        let t1 = if search.end.seconds_since(&t0) > step { t0.add_seconds(step) } else { search.end };
        let f1 = above(t1)?;
        if f0 <= 0.0 && f1 > 0.0 {
            rise = Some((bisect(&above, t0, t1)?, false));
        } else if f0 > 0.0 && f1 <= 0.0 {
            if let Some((r, clipped)) = rise.take() {
                let set = bisect(&above, t0, t1)?;
                out.push(describe(prop, site, r, set, clipped, false)?);
            }
        }
        t0 = t1;
        f0 = f1;
    }
    if let Some((r, clipped)) = rise {
        out.push(describe(prop, site, r, search.end, clipped, true)?);
    }
    Ok(())
}

/// Find where `f` changes sign between `lo` and `hi`, to `REFINE_TOLERANCE_S`.
fn bisect(f: &dyn Fn(Epoch) -> Result<f64, OrbitPropError>, mut lo: Epoch, mut hi: Epoch) -> Result<Epoch, OrbitPropError> {
    let lo_positive = f(lo)? > 0.0;
    while hi.seconds_since(&lo) > REFINE_TOLERANCE_S {
        let mid = lo.add_seconds(hi.seconds_since(&lo) / 2.0);
        if (f(mid)? > 0.0) == lo_positive {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Ok(lo.add_seconds(hi.seconds_since(&lo) / 2.0))
}

/// Peak absolute values of rates and of rate differences over a sample run.
#[derive(Default)]
struct Peaks {
    az: f64,
    el: f64,
    ha: f64,
    dec: f64,
    ground: f64,
    az_acc: f64,
    el_acc: f64,
    ha_acc: f64,
    dec_acc: f64,
}

impl Peaks {
    fn add_run(&mut self, obs: &[Observation], dt: f64) {
        for o in obs {
            self.az = self.az.max(o.az_rate_deg_s.abs());
            self.el = self.el.max(o.el_rate_deg_s.abs());
            self.ha = self.ha.max(o.ha_rate_deg_s.abs());
            self.dec = self.dec.max(o.dec_rate_deg_s.abs());
            self.ground = self.ground.max(o.rate_vs_ground_deg_s);
        }
        for w in obs.windows(2) {
            let d = |f: fn(&Observation) -> f64| ((f(&w[1]) - f(&w[0])) / dt).abs();
            self.az_acc = self.az_acc.max(d(|o| o.az_rate_deg_s));
            self.el_acc = self.el_acc.max(d(|o| o.el_rate_deg_s));
            self.ha_acc = self.ha_acc.max(d(|o| o.ha_rate_deg_s));
            self.dec_acc = self.dec_acc.max(d(|o| o.dec_rate_deg_s));
        }
    }
}

/// Evenly spaced times from `a` to `b` inclusive, no further apart than `max_step`.
fn sample_times(a: Epoch, b: Epoch, max_step: f64) -> (Vec<Epoch>, f64) {
    let span = b.seconds_since(&a).max(0.0);
    let n = ((span / max_step).ceil() as usize).max(1);
    let dt = span / n as f64;
    ((0..=n).map(|k| a.add_seconds(k as f64 * dt)).collect(), dt)
}

fn summarize<T>(flags: &[bool], all: T, none: T, mixed: T) -> T {
    if flags.iter().all(|&f| f) {
        all
    } else if flags.iter().any(|&f| f) {
        mixed
    } else {
        none
    }
}

fn describe(
    prop: &dyn Propagator,
    site: &GroundSite,
    rise: Epoch,
    set: Epoch,
    clipped_start: bool,
    clipped_end: bool,
) -> Result<Pass, OrbitPropError> {
    let duration = set.seconds_since(&rise);
    let dense_step = DENSE_STEP_S.max(duration / MAX_DENSE_SAMPLES);
    let (times, dt) = sample_times(rise, set, dense_step);
    let mut obs = Vec::with_capacity(times.len());
    let mut sunlit = Vec::with_capacity(times.len());
    let mut dark = Vec::with_capacity(times.len());
    for &t in &times {
        let state = prop.propagate(t)?;
        obs.push(observe(&state, site));
        sunlit.push(lighting(state.r_km, sun_position_km(t)) == Lighting::Sunlit);
        dark.push(sun_elevation_deg(site, t) < DARK_SUN_ELEVATION_DEG);
    }
    let mut peaks = Peaks::default();
    peaks.add_run(&obs, dt.max(f64::MIN_POSITIVE));

    // Culmination: best dense sample, then golden-section refinement.
    let best = obs.iter().enumerate().fold(0, |b, (k, o)| if o.el_deg > obs[b].el_deg { k } else { b });
    let lo = times[best.saturating_sub(1)];
    let hi = times[(best + 1).min(times.len() - 1)];
    let (culmination, max_el) = golden_max(&|t| Ok(look(prop, site, t)?.el_deg), lo, hi)?;

    // Fine sampling near culmination, where the alt-az keyhole bites.
    let fine_lo = culmination.add_seconds(-FINE_HALF_WINDOW_S).max_of(rise);
    let fine_hi = culmination.add_seconds(FINE_HALF_WINDOW_S).min_of(set);
    let (fine_times, fine_dt) = sample_times(fine_lo, fine_hi, FINE_STEP_S);
    if fine_dt > 0.0 {
        let fine: Result<Vec<Observation>, OrbitPropError> = fine_times.iter().map(|&t| look(prop, site, t)).collect();
        peaks.add_run(&fine?, fine_dt);
    }

    Ok(Pass {
        rise,
        culmination,
        set,
        clipped_start,
        clipped_end,
        max_el_deg: max_el,
        peak_az_rate_deg_s: peaks.az,
        peak_el_rate_deg_s: peaks.el,
        peak_az_accel_deg_s2: peaks.az_acc,
        peak_el_accel_deg_s2: peaks.el_acc,
        peak_ha_rate_deg_s: peaks.ha,
        peak_dec_rate_deg_s: peaks.dec,
        peak_ha_accel_deg_s2: peaks.ha_acc,
        peak_dec_accel_deg_s2: peaks.dec_acc,
        peak_rate_vs_ground_deg_s: peaks.ground,
        lighting: summarize(&sunlit, PassLighting::Sunlit, PassLighting::Eclipsed, PassLighting::Partial),
        site_dark: summarize(&dark, PassDarkness::Dark, PassDarkness::Daylight, PassDarkness::Partial),
    })
}

/// Maximise `f` on `[lo, hi]` by golden-section search to 0.01 s.
fn golden_max(
    f: &dyn Fn(Epoch) -> Result<f64, OrbitPropError>,
    lo: Epoch,
    hi: Epoch,
) -> Result<(Epoch, f64), OrbitPropError> {
    const INV_PHI: f64 = 0.618_033_988_749_895;
    let (mut a, mut b) = (lo, hi);
    while b.seconds_since(&a) > 0.01 {
        let span = b.seconds_since(&a);
        let c = b.add_seconds(-span * INV_PHI);
        let d = a.add_seconds(span * INV_PHI);
        if f(c)? > f(d)? {
            b = d;
        } else {
            a = c;
        }
    }
    let t = a.add_seconds(b.seconds_since(&a) / 2.0);
    Ok((t, f(t)?))
}

trait EpochOrd {
    fn max_of(self, other: Epoch) -> Epoch;
    fn min_of(self, other: Epoch) -> Epoch;
}

impl EpochOrd for Epoch {
    fn max_of(self, other: Epoch) -> Epoch {
        if self.seconds_since(&other) >= 0.0 { self } else { other }
    }
    fn min_of(self, other: Epoch) -> Epoch {
        if self.seconds_since(&other) <= 0.0 { self } else { other }
    }
}
~~~~

- [ ] **Step 4: Run the tests and confirm they pass**

Run: `cargo test -p orbit-prop`
Expected: `test result: ok. 56 passed; 0 failed`, with no compiler warnings. `cargo test -p orbit-prop --doc` reports `1 passed` (the README example).

- [ ] **Step 5: Commit**

~~~~bash
git add crates/orbit-prop
git commit -m "Add pass finding and the orbit-prop README

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
~~~~


---

### Task 8: scope-eval site location, pass defaults and mount judgment

**Files:**
- Modify: `src/model/site.rs`, `src/constants.rs`, `src/main.rs` (two `Site` literals and the module list), `src/regimes.rs:921`, `src/photometry.rs:366`
- Create: `src/passes_report.rs`

**Interfaces:**
- Consumes: `orbit_prop::{GroundSite, Epoch, Pass, PassDarkness, PassLighting, PassResult}`, existing `checks::Status` (ordered `Info < Pass < Warn < Fail`), `model::{plausible, Config, Mount, MountType}`, `constants::regimes_limits::{RATE_PASS_HEADROOM, RATE_WARN_HEADROOM, RATE_MATTERS_DEG_S, ACCEL_PASS_HEADROOM, ACCEL_WARN_HEADROOM, ACCEL_MATTERS_DEG_S2}`, `constants::plausible_ranges::{SLEW_RATE_MIN_DEG_S, SLEW_RATE_MAX_DEG_S, ACCEL_MIN_DEG_S2, ACCEL_MAX_DEG_S2}`.
- Produces:
  - `Site.location: Option<GroundSite>`.
  - `constants::{DEFAULT_MIN_ELEVATION_DEG = 10.0, MAX_PASS_SEARCH_HOURS = 720.0, STALE_TLE_DAYS = 14.0}`.
  - `passes_report::judge_mount_for_pass(&Mount, &Pass) -> (Status, String)`, `passes_report::stale_tle_note(&Epoch, &Epoch) -> Option<String>`, `passes_report::print_passes(label: &str, &PassResult, &[Config])`.

- [ ] **Step 1: Add the site location**

In `src/model/site.rs`, below the `//! Site and observing assumptions.` line, add:

~~~~rust

use orbit_prop::GroundSite;
~~~~

and add the field after `sky_mag_arcsec2`:

~~~~rust
    /// Where the site is on Earth. Only pass prediction uses it; `None`
    /// until the user enters it.
    pub location: Option<GroundSite>,
~~~~

Add `location: None` to the four existing `Site { ... }` literals:
- `src/regimes.rs:921`, which becomes `let site = Site { seeing_arcsec: 2.5, wavelength_um: 0.55, sky_mag_arcsec2: None, location: None };`
- `src/photometry.rs:366`, which becomes `Site { seeing_arcsec: 2.5, wavelength_um: 0.55, sky_mag_arcsec2: None, location: None }`
- `src/main.rs`, in `run_interactive` (after the `sky_mag_arcsec2: input::ask_optional_hint(...)` field) and in `run_demo` (after `sky_mag_arcsec2: None,`): add the line `location: None,`

Run: `cargo test`
Expected: all green (`119 passed` for scope-eval).

- [ ] **Step 2: Add the pass-prediction constants**

In `src/constants.rs`, directly after `pub const DEFAULT_SEEING_ARCSEC: f64 = 2.5;`, add:

~~~~rust
/// Pass prediction: default minimum elevation for a usable pass, degrees.
/// Below about 10 deg, extinction, seeing and horizon obstructions dominate.
pub const DEFAULT_MIN_ELEVATION_DEG: f64 = 10.0;
/// Pass prediction: longest search window, hours (the library's 30-day limit).
pub const MAX_PASS_SEARCH_HOURS: f64 = 720.0;
/// Pass prediction: TLE age beyond which pass times may be off by minutes, days.
pub const STALE_TLE_DAYS: f64 = 14.0;
~~~~

- [ ] **Step 3: Write the failing tests**

Create `src/passes_report.rs` with only its test module:

~~~~rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Capability;

    fn pass(az_rate: f64, el_rate: f64, az_acc: f64, el_acc: f64) -> Pass {
        let t = Epoch::from_utc(2026, 10, 4, 0, 0, 0.0).unwrap();
        Pass {
            rise: t,
            culmination: t.add_seconds(300.0),
            set: t.add_seconds(600.0),
            clipped_start: false,
            clipped_end: false,
            max_el_deg: 60.0,
            peak_az_rate_deg_s: az_rate,
            peak_el_rate_deg_s: el_rate,
            peak_az_accel_deg_s2: az_acc,
            peak_el_accel_deg_s2: el_acc,
            peak_ha_rate_deg_s: 0.5,
            peak_dec_rate_deg_s: 0.2,
            peak_ha_accel_deg_s2: 0.001,
            peak_dec_accel_deg_s2: 0.002,
            peak_rate_vs_ground_deg_s: 0.8,
            lighting: PassLighting::Sunlit,
            site_dark: PassDarkness::Dark,
        }
    }

    fn mount(kind: MountType, slew: Option<f64>, accel: Option<f64>) -> Mount {
        Mount {
            name: "test".into(),
            mount_type: kind,
            capacity_lb: None,
            max_slew_deg_s: slew,
            max_accel_deg_s2: accel,
            settle_time_s: None,
            pointing_rms_arcsec: None,
            non_sidereal_tracking: Capability::Unknown,
            source: String::new(),
        }
    }

    #[test]
    fn ample_headroom_passes_and_names_the_binding_axis() {
        let (s, note) = judge_mount_for_pass(&mount(MountType::AltAz, Some(10.0), Some(10.0)), &pass(1.0, 0.5, 0.1, 0.05));
        assert_eq!(s, Status::Pass);
        assert_eq!(note, "az rate 10.0x");
    }

    #[test]
    fn rate_headroom_between_1x_and_3x_warns_below_1x_fails() {
        let m = mount(MountType::AltAz, Some(2.0), Some(10.0));
        assert_eq!(judge_mount_for_pass(&m, &pass(1.0, 0.5, 0.1, 0.05)).0, Status::Warn);
        assert_eq!(judge_mount_for_pass(&m, &pass(4.0, 0.5, 0.1, 0.05)).0, Status::Fail);
    }

    #[test]
    fn acceleration_can_be_the_binding_limit() {
        // Near-zenith pass: rate fine, azimuth acceleration exceeds the rating.
        let (s, note) = judge_mount_for_pass(&mount(MountType::AltAz, Some(10.0), Some(1.0)), &pass(1.0, 0.5, 2.0, 0.05));
        assert_eq!(s, Status::Fail);
        assert_eq!(note, "az accel 0.5x");
    }

    #[test]
    fn unknown_ratings_warn_only_when_the_pass_demands_it() {
        let m = mount(MountType::AltAz, None, None);
        assert_eq!(judge_mount_for_pass(&m, &pass(1.0, 0.5, 0.1, 0.05)).0, Status::Warn);
        // Slow pass: below RATE_MATTERS_DEG_S and ACCEL_MATTERS_DEG_S2.
        assert_eq!(judge_mount_for_pass(&m, &pass(0.01, 0.01, 0.0001, 0.0001)).0, Status::Info);
    }

    #[test]
    fn implausible_ratings_are_treated_as_unknown() {
        let m = mount(MountType::AltAz, Some(0.0), Some(f64::NAN));
        let (s, note) = judge_mount_for_pass(&m, &pass(1.0, 0.5, 0.1, 0.05));
        assert_eq!(s, Status::Warn);
        assert!(note.contains("unknown"), "{note}");
    }

    #[test]
    fn equatorial_mount_uses_hour_angle_and_declination_axes() {
        // Huge az rate (near-zenith) is irrelevant to an equatorial mount.
        let m = mount(MountType::Equatorial, Some(2.0), Some(1.0));
        let (s, note) = judge_mount_for_pass(&m, &pass(50.0, 0.5, 20.0, 0.05));
        assert_eq!(s, Status::Pass);
        assert_eq!(note, "HA rate 4.0x");
    }

    #[test]
    fn unknown_mount_type_takes_the_worse_axis_set() {
        let m = mount(MountType::Unknown, Some(10.0), Some(10.0));
        let (s, note) = judge_mount_for_pass(&m, &pass(50.0, 0.5, 0.1, 0.05));
        assert_eq!(s, Status::Fail);
        assert_eq!(note, "az rate 0.2x");
    }

    #[test]
    fn stationary_target_needs_nothing() {
        let (s, _) = judge_mount_for_pass(&mount(MountType::AltAz, Some(5.0), Some(5.0)), &pass(0.0, 0.0, 0.0, 0.0));
        assert_eq!(s, Status::Pass);
    }

    #[test]
    fn stale_tle_is_flagged_after_two_weeks() {
        let epoch = Epoch::from_utc(2026, 10, 1, 0, 0, 0.0).unwrap();
        assert_eq!(stale_tle_note(&epoch, &epoch.add_seconds(13.0 * 86_400.0)), None);
        let note = stale_tle_note(&epoch, &epoch.add_seconds(40.0 * 86_400.0)).unwrap();
        assert!(note.contains("40 days"), "{note}");
        // A search far before the epoch is just as stale.
        assert!(stale_tle_note(&epoch, &epoch.add_seconds(-20.0 * 86_400.0)).is_some());
    }

    #[test]
    fn duration_formatting() {
        assert_eq!(duration_text(425.4), "7m05s");
        assert_eq!(duration_text(6.0 * 3600.0 + 61.0), "6h01m");
    }
}
~~~~

In `src/main.rs`, add `mod passes_report;` after `mod model;`.

- [ ] **Step 4: Run the tests and confirm they fail**

Run: `cargo test -p scope-eval passes_report`
Expected: compile errors (`cannot find function judge_mount_for_pass`, and so on).

- [ ] **Step 5: Write the implementation**

Put this above the test module in `src/passes_report.rs`:

~~~~rust
//! Pass-prediction table and the per-pass "Mount can follow?" judgment.
//!
//! The geometry comes from the `orbit-prop` library. This module only
//! applies the mount thresholds already used by the regime checks and
//! prints the result.

use orbit_prop::{Epoch, Pass, PassDarkness, PassLighting, PassResult};

use crate::checks::Status;
use crate::constants::plausible_ranges as ranges;
use crate::constants::regimes_limits as limits;
use crate::constants::STALE_TLE_DAYS;
use crate::model::{plausible, Config, Mount, MountType};

/// One axis requirement: the axis name and the pass's peak value on it.
struct AxisNeed {
    axis: &'static str,
    value: f64,
}

/// The larger of two axis requirements.
fn worst(a: AxisNeed, b: AxisNeed) -> AxisNeed {
    if b.value > a.value { b } else { a }
}

/// Peak rate and acceleration the mount's own axes must reach on this pass.
fn axis_needs(mount_type: MountType, p: &Pass) -> (AxisNeed, AxisNeed) {
    let altaz = (
        worst(AxisNeed { axis: "az", value: p.peak_az_rate_deg_s }, AxisNeed { axis: "el", value: p.peak_el_rate_deg_s }),
        worst(AxisNeed { axis: "az", value: p.peak_az_accel_deg_s2 }, AxisNeed { axis: "el", value: p.peak_el_accel_deg_s2 }),
    );
    let equatorial = (
        worst(AxisNeed { axis: "HA", value: p.peak_ha_rate_deg_s }, AxisNeed { axis: "dec", value: p.peak_dec_rate_deg_s }),
        worst(AxisNeed { axis: "HA", value: p.peak_ha_accel_deg_s2 }, AxisNeed { axis: "dec", value: p.peak_dec_accel_deg_s2 }),
    );
    match mount_type {
        MountType::AltAz => altaz,
        MountType::Equatorial => equatorial,
        MountType::Unknown => (worst(altaz.0, equatorial.0), worst(altaz.1, equatorial.1)),
    }
}

/// Judge one requirement against a rating, with the regime-check thresholds.
fn judge_axis(
    need: &AxisNeed,
    quantity: &str,
    rating: Option<f64>,
    pass_headroom: f64,
    warn_headroom: f64,
    matters: f64,
) -> (Status, String) {
    match rating {
        None if need.value > matters => (Status::Warn, format!("{} {quantity} unknown", need.axis)),
        None => (Status::Info, format!("{quantity} unknown")),
        Some(_) if need.value <= 0.0 => (Status::Pass, format!("{quantity} not needed")),
        Some(max) => {
            let headroom = max / need.value;
            let status = if headroom >= pass_headroom {
                Status::Pass
            } else if headroom >= warn_headroom {
                Status::Warn
            } else {
                Status::Fail
            };
            (status, format!("{} {quantity} {headroom:.1}x", need.axis))
        }
    }
}

/// Can this mount follow this pass? Compares the pass's peak axis rate and
/// acceleration with the mount's ratings, using the same headroom thresholds
/// as the regime checks. For an alt-az mount a near-zenith pass shows up
/// directly as a large azimuth rate and acceleration, so the keyhole needs
/// no separate formula. Returns the worse of the two judgments and a short
/// note naming the binding axis.
pub fn judge_mount_for_pass(m: &Mount, p: &Pass) -> (Status, String) {
    let (rate_need, accel_need) = axis_needs(m.mount_type, p);
    let rate = judge_axis(
        &rate_need,
        "rate",
        plausible(m.max_slew_deg_s, ranges::SLEW_RATE_MIN_DEG_S, ranges::SLEW_RATE_MAX_DEG_S),
        limits::RATE_PASS_HEADROOM,
        limits::RATE_WARN_HEADROOM,
        limits::RATE_MATTERS_DEG_S,
    );
    let accel = judge_axis(
        &accel_need,
        "accel",
        plausible(m.max_accel_deg_s2, ranges::ACCEL_MIN_DEG_S2, ranges::ACCEL_MAX_DEG_S2),
        limits::ACCEL_PASS_HEADROOM,
        limits::ACCEL_WARN_HEADROOM,
        limits::ACCEL_MATTERS_DEG_S2,
    );
    if accel.0 > rate.0 { accel } else { rate }
}

/// A warning when the TLE's epoch is far from the search start, because
/// SGP4 errors grow quickly with element age.
pub fn stale_tle_note(tle_epoch: &Epoch, start: &Epoch) -> Option<String> {
    let age_days = start.seconds_since(tle_epoch).abs() / 86_400.0;
    (age_days > STALE_TLE_DAYS).then(|| {
        format!(
            "Note: this TLE's epoch ({tle_epoch}) is {age_days:.0} days from the search start.\n  SGP4 errors grow quickly with age; pass times may be off by minutes."
        )
    })
}

fn lighting_text(l: PassLighting) -> &'static str {
    match l {
        PassLighting::Sunlit => "yes",
        PassLighting::Partial => "partial",
        PassLighting::Eclipsed => "no",
    }
}

fn dark_text(d: PassDarkness) -> &'static str {
    match d {
        PassDarkness::Dark => "yes",
        PassDarkness::Partial => "twilight",
        PassDarkness::Daylight => "no",
    }
}

fn hms(t: &Epoch) -> String {
    let u = t.to_utc();
    format!("{:02}:{:02}:{:02}", u.hour, u.minute, u.second.floor() as u32)
}

fn date_hms(t: &Epoch) -> String {
    let u = t.to_utc();
    format!("{:04}-{:02}-{:02} {}", u.year, u.month, u.day, hms(t))
}

fn duration_text(seconds: f64) -> String {
    let s = seconds.round() as u64;
    if s >= 3600 {
        format!("{}h{:02}m", s / 3600, s % 3600 / 60)
    } else {
        format!("{}m{:02}s", s / 60, s % 60)
    }
}

/// Print the pass table. One "Mount can follow?" column is added for each
/// evaluated configuration that has a mount.
pub fn print_passes(label: &str, result: &PassResult, configs: &[Config]) {
    let mounted: Vec<(&str, &Mount)> =
        configs.iter().filter_map(|c| c.payload.mount.as_ref().map(|m| (c.label.as_str(), m))).collect();
    println!("\n==========================================================================");
    println!(" Passes of {label}");
    println!("==========================================================================");
    if result.passes.is_empty() {
        println!(" No passes in this window.");
    } else {
        print!(" #  Rise (UTC)            Set (UTC)  Duration MaxEl  Az rate El rate Sunlit  Dark    ");
        for (i, _) in mounted.iter().enumerate() {
            print!(" | {:<22}", format!("Mount {}", i + 1));
        }
        println!();
        for (n, p) in result.passes.iter().enumerate() {
            let start_mark = if p.clipped_start { "<" } else { " " };
            let end_mark = if p.clipped_end { ">" } else { " " };
            print!(
                "{:>2} {start_mark}{} {}{end_mark} {:>8} {:>5.1} {:>7.3} {:>7.3} {:<7} {:<8}",
                n + 1,
                date_hms(&p.rise),
                hms(&p.set),
                duration_text(p.duration_s()),
                p.max_el_deg,
                p.peak_az_rate_deg_s,
                p.peak_el_rate_deg_s,
                lighting_text(p.lighting),
                dark_text(p.site_dark),
            );
            for (_, m) in &mounted {
                let (status, note) = judge_mount_for_pass(m, p);
                print!(" | {} {:<15}", status.tag(), note);
            }
            println!();
        }
        println!("\n Rates are peak axis rates in deg/s for an alt-az mount. '<' = already up at");
        println!(" the start of the window, '>' = still up at the end. Sunlit: is the satellite");
        println!(" in sunlight. Dark: is the Sun more than 12 deg below the site's horizon.");
        if !mounted.is_empty() {
            println!("\n Mount columns compare the pass's peak axis rate and acceleration with the");
            println!(" mount's ratings (PASS at 3x headroom, WARN at 1x, FAIL below; INFO or WARN");
            println!(" when a rating is unknown). The note names the binding axis and its headroom.");
            for (i, (cfg_label, m)) in mounted.iter().enumerate() {
                println!("   Mount {}: {} ({cfg_label})", i + 1, m.name);
            }
        }
    }
    if let Some(e) = &result.error {
        println!("\n The search stopped early: {e}");
    }
}
~~~~

- [ ] **Step 6: Run the tests and confirm they pass**

Run: `cargo test`
Expected: `scope-eval` reports `129 passed`, and `orbit-prop` reports `56 passed`. Dead-code warnings for `print_passes` and its formatting helpers (`hms`, `date_hms`, `lighting_text`, `dark_text`), `stale_tle_note`, `DEFAULT_MIN_ELEVATION_DEG` and `MAX_PASS_SEARCH_HOURS` are expected until Task 9 wires them in. Do not silence them.

- [ ] **Step 7: Commit**

~~~~bash
git add src/model/site.rs src/constants.rs src/main.rs src/regimes.rs src/photometry.rs src/passes_report.rs
git commit -m "Add site location and per-pass mount judgment to scope-eval

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
~~~~

---

### Task 9: Pass-prediction menu item and README

**Files:**
- Modify: `src/main.rs` (imports, menu, help text, new prompt functions)
- Modify: `README.md`

**Interfaces:**
- Consumes: everything `orbit_prop` exports; `passes_report::{print_passes, stale_tle_note}`; `input::{read_line, ask_menu, ask_positive, ask_nonnegative, ask_optional_signed}`; `constants::{DEFAULT_MIN_ELEVATION_DEG, MAX_PASS_SEARCH_HOURS}`.
- Produces: main-menu item 5, "Predict passes for a satellite". Formula summary, clear and quit move to 6, 7 and 8.

This task is terminal I/O wiring, so its tests are the scripted end-to-end runs in Step 5. The logic it calls is already unit-tested.

- [ ] **Step 1: Imports**

In `src/main.rs`, change the `use constants::{` list to begin with `DEFAULT_MIN_ELEVATION_DEG, DEFAULT_SEEING_ARCSEC, MAX_PASS_SEARCH_HOURS,` (keep the rest), and add after the `use model::{...};` block:

~~~~rust
use orbit_prop::{
    find_passes, Epoch, GroundSite, KeplerElements, KeplerJ2, OrbitPropError, PassSearch, Propagator, Sgp4Propagator, Tle,
};
~~~~

- [ ] **Step 2: Prompt functions**

Insert directly above `fn build_config() -> Config {`:

~~~~rust
fn describe_location(site: &Site) -> String {
    match site.location {
        Some(l) => format!("site {:.4}, {:.4}, {:.0} m", l.lat_deg, l.lon_deg, l.alt_m),
        None => "location not set".to_string(),
    }
}

/// Ask for the site's location. A blank latitude keeps `current`.
fn ask_location(current: Option<GroundSite>) -> Option<GroundSite> {
    let hint = if current.is_some() { "blank to keep the current location" } else { "blank to skip" };
    loop {
        let Some(lat) = input::ask_optional_signed("Site latitude, deg (north positive)", hint) else {
            return current;
        };
        let lon = loop {
            if let Some(v) = input::ask_optional_signed("Site longitude, deg (east positive)", "required") {
                break v;
            }
            println!("  A longitude is required.");
        };
        let alt = input::ask_optional_signed("Site altitude, m", "blank for 0").unwrap_or(0.0);
        match GroundSite::new(lat, lon, alt) {
            Ok(s) => return Some(s),
            Err(e) => println!("  {e}"),
        }
    }
}

/// An optional UTC time. Blank returns `None`.
fn ask_epoch(prompt: &str, hint: &str) -> Option<Epoch> {
    loop {
        let s = input::read_line(&format!("{prompt} ({hint}): "));
        if s.is_empty() {
            return None;
        }
        match Epoch::parse_iso8601(&s) {
            Ok(t) => return Some(t),
            Err(e) => println!("  {e}"),
        }
    }
}

/// Read a pasted TLE line by line until it parses.
fn ask_tle() -> Tle {
    println!("Paste the TLE (2 lines, or 3 with a name line first):");
    let mut lines: Vec<String> = Vec::new();
    loop {
        let line = input::read_line("> ");
        if line.is_empty() {
            continue;
        }
        let is_line2 = line.starts_with("2 ");
        lines.push(line);
        if is_line2 {
            match Tle::parse(&lines.join("\n")) {
                Ok(t) => return t,
                Err(e) => println!("  {e}\n  Please paste the TLE again."),
            }
            lines.clear();
        } else if lines.len() > 2 {
            println!("  That doesn't look like a TLE. Please paste it again.");
            lines.clear();
        }
    }
}

/// Where the orbit comes from. A what-if orbit's epoch defaults to the
/// search start, which is asked for afterwards.
enum OrbitSource {
    Tle(Tle),
    WhatIf {
        perigee_km: f64,
        apogee_km: f64,
        i_deg: f64,
        raan_deg: f64,
        argp_deg: f64,
        mean_anomaly_deg: f64,
        epoch: Option<Epoch>,
    },
}

fn ask_orbit_source() -> OrbitSource {
    let options = ["Paste a TLE".to_string(), "Define a what-if orbit".to_string()];
    if input::ask_menu("Orbit source", &options) == 0 {
        return OrbitSource::Tle(ask_tle());
    }
    let perigee_km = input::ask_positive("Perigee altitude, km", None);
    let apogee_km = loop {
        let a = input::ask_positive("Apogee altitude, km", Some(perigee_km));
        if a >= perigee_km {
            break a;
        }
        println!("  Apogee must be at least the perigee altitude.");
    };
    let i_deg = loop {
        let i = input::ask_nonnegative("Inclination, deg", 0.0);
        if i <= 180.0 {
            break i;
        }
        println!("  Inclination must be between 0 and 180 degrees.");
    };
    OrbitSource::WhatIf {
        perigee_km,
        apogee_km,
        i_deg,
        raan_deg: input::ask_nonnegative("Right ascension of the ascending node, deg", 0.0),
        argp_deg: input::ask_nonnegative("Argument of perigee, deg", 0.0),
        mean_anomaly_deg: input::ask_nonnegative("Mean anomaly at epoch, deg", 0.0),
        epoch: ask_epoch("Element epoch, UTC YYYY-MM-DDTHH:MM", "blank for the search start"),
    }
}

fn build_propagator(source: OrbitSource, start: Epoch) -> Result<Box<dyn Propagator>, OrbitPropError> {
    match source {
        OrbitSource::Tle(tle) => {
            if let Some(note) = passes_report::stale_tle_note(&tle.epoch, &start) {
                println!("\n  {note}");
            }
            Ok(Box::new(Sgp4Propagator::new(&tle)?))
        }
        OrbitSource::WhatIf { perigee_km, apogee_km, i_deg, raan_deg, argp_deg, mean_anomaly_deg, epoch } => {
            let el = KeplerElements::from_altitudes(
                epoch.unwrap_or(start),
                perigee_km,
                apogee_km,
                i_deg,
                raan_deg,
                argp_deg,
                mean_anomaly_deg,
            )?;
            let label = format!("what-if orbit {perigee_km:.0} x {apogee_km:.0} km, {i_deg:.1} deg");
            Ok(Box::new(KeplerJ2::new(el, &label)?))
        }
    }
}

/// Menu item: predict passes over the site and judge each evaluated mount.
fn run_pass_prediction(site: &mut Site, configs: &[Config]) {
    if site.location.is_none() {
        println!("\nPass prediction needs the site's location.");
        while site.location.is_none() {
            site.location = ask_location(None);
        }
    }
    let Some(location) = site.location else { return };
    let source = ask_orbit_source();
    let start = ask_epoch("Search start, UTC YYYY-MM-DDTHH:MM", "blank for now").unwrap_or_else(Epoch::now);
    let hours = loop {
        let h = input::ask_positive("Search length, hours", Some(24.0));
        if h <= MAX_PASS_SEARCH_HOURS {
            break h;
        }
        println!("  The longest search is {MAX_PASS_SEARCH_HOURS:.0} hours.");
    };
    let min_el_deg = loop {
        let e = input::ask_nonnegative("Minimum elevation, deg", DEFAULT_MIN_ELEVATION_DEG);
        if e < 90.0 {
            break e;
        }
        println!("  Enter an elevation below 90 degrees.");
    };
    let prop = match build_propagator(source, start) {
        Ok(p) => p,
        Err(e) => {
            println!("\n  {e}");
            return;
        }
    };
    let search = PassSearch { start, end: start.add_seconds(hours * 3600.0), min_el_deg };
    println!("\nSearching {hours:.0} h from {start} ...");
    let result = find_passes(prop.as_ref(), &location, &search);
    passes_report::print_passes(prop.label(), &result, configs);
}
~~~~

- [ ] **Step 3: Menu and help**

In `run_interactive`, replace the site-conditions menu entry and the `"Show formula summary"` line with:

~~~~rust
            format!(
                "Change site conditions (seeing {:.2}\", sky {}, {})",
                site.seeing_arcsec,
                match site.sky_mag_arcsec2 {
                    Some(s) => format!("{s:.2}"),
                    None => "assumed".to_string(),
                },
                describe_location(&site)
            ),
            "Predict passes for a satellite".to_string(),
            "Show formula summary".to_string(),
~~~~

In the `match input::ask_menu(...)`, add a location prompt at the end of arm `3` and renumber the arms after it:

~~~~rust
                site.sky_mag_arcsec2 = input::resolve_keep(answer, site.sky_mag_arcsec2);
                site.location = ask_location(site.location);
                println!("Site updated. All configurations will be re-evaluated.");
            }
            4 => run_pass_prediction(&mut site, &configs),
            5 => report::print_formulas(),
            6 => {
~~~~

(The old `4 => report::print_formulas(),` and `5 => {` become the lines shown. The `_ => break` arm stays as it is.)

In `print_help`, insert before `USAGE:`:

~~~~text
The interactive menu can also predict passes of a satellite (from a TLE or
a what-if orbit) over your site, and judge whether each evaluated mount can
follow each pass.
~~~~

Run: `cargo build`
Expected: builds with only the existing `f_ratio` dead-code warning.

- [ ] **Step 4: README**

Make these edits to `README.md`:

**Edit 1.** Replace:

~~~~markdown
The tool has no external dependencies. It uses only the Rust standard library, so it builds offline and is easy to audit.
~~~~

with:

~~~~markdown
The tool has three external dependencies, all pure Rust: `serde` and `serde_yaml` read `presets.yaml`, and `sgp4` propagates satellite orbits inside the bundled `orbit-prop` library (`crates/orbit-prop`). The code is small enough to audit.
~~~~

**Edit 2.** Replace:

~~~~markdown
9. [Target brightness and detection](#target-brightness-and-detection)
10. [The comparison table](#the-comparison-table)
11. [Thresholds and how to tune them](#thresholds-and-how-to-tune-them)
12. [Assumptions and limitations](#assumptions-and-limitations)
13. [Spec-sheet red flags](#spec-sheet-red-flags)
14. [Presets and their sources](#presets-and-their-sources)
15. [Code structure and extending the tool](#code-structure-and-extending-the-tool)
~~~~

with:

~~~~markdown
9. [Target brightness and detection](#target-brightness-and-detection)
10. [Pass prediction](#pass-prediction)
11. [The comparison table](#the-comparison-table)
12. [Thresholds and how to tune them](#thresholds-and-how-to-tune-them)
13. [Assumptions and limitations](#assumptions-and-limitations)
14. [Spec-sheet red flags](#spec-sheet-red-flags)
15. [Presets and their sources](#presets-and-their-sources)
16. [Code structure and extending the tool](#code-structure-and-extending-the-tool)
~~~~

**Edit 3.** Replace:

~~~~markdown
cargo test                     # run the worked examples in this README as unit tests
~~~~

with:

~~~~markdown
cargo test                     # run the worked examples in this README, and the orbit-prop library's tests
~~~~

**Edit 4.** Replace:

~~~~markdown
## The comparison table
~~~~

with:

~~~~markdown
## Pass prediction

The regime checks above judge one representative geometry per regime. **Predict passes for a satellite** in the main menu uses the real path of one orbit over your site instead: it lists every pass in a time window and judges, pass by pass, whether each evaluated mount can follow it.

You enter:

1. **Your site's location**: latitude (north positive), longitude (east positive) and altitude in metres. It is asked for once and can be changed under **Change site conditions**.
2. **The orbit**, one of:
   * *Paste a TLE* from CelesTrak or Space-Track (two lines, or three with a name line first). It is propagated with SGP4, the model TLEs are fitted with. If the TLE's epoch is more than 14 days from the search start, the tool warns that pass times may be off by minutes.
   * *Define a what-if orbit*: perigee and apogee altitude, inclination, right ascension of the ascending node, argument of perigee and mean anomaly. It is propagated as a Keplerian orbit with J2 drift, which is right for "what would a 550 km, 53 degree orbit look like from here" but not for tracking a particular object.
3. **The window**: start time in UTC (blank for now), length (24 hours by default, 720 at most) and minimum elevation (10 degrees by default).

Example, the what-if orbit 550 km at 53 degrees from 40 N 75 W with a PlaneWave L-350 whose acceleration was entered as 2 deg/s^2:

```
 #  Rise (UTC)            Set (UTC)  Duration MaxEl  Az rate El rate Sunlit  Dark     | Mount 1
 1  2026-10-04 04:56:04 04:58:50     2m45s  11.4   0.245   0.032 no      yes      | [PASS] az rate 203.8x
 2  2026-10-04 06:32:15 06:40:37     8m22s  86.3  11.603   0.686 no      yes      | [WARN] az accel 1.3x
 3  2026-10-04 08:13:07 08:19:47     6m40s  22.3   0.366   0.080 partial yes      | [PASS] az rate 136.5x
```

* **Az rate, El rate** are the peak axis rates an alt-az mount needs during the pass, in deg/s. Pass 2 culminates at 86 degrees, so its azimuth axis has to swing 11.6 deg/s near the zenith: the alt-az keyhole, computed from the real pass rather than from a formula.
* **Sunlit** says whether the satellite is in sunlight (yes, partial, no). An optical sensor sees only sunlit satellites.
* **Dark** says whether the Sun is more than 12 degrees below your horizon (yes, twilight, no).
* **Mount N** compares the pass's peak axis rate and acceleration with the mount's ratings, using the thresholds of the regime mount checks (`RATE_PASS_HEADROOM`, `ACCEL_PASS_HEADROOM` and the rest in `src/constants.rs`). The note names the axis and quantity that bind and their headroom. An equatorial mount is judged on its hour-angle and declination axes. A rating that is unknown gives WARN when the pass needs real speed and INFO when it doesn't.
* `<` before a rise time means the satellite was already up when the window started; `>` after a set time means it was still up when the window ended.

The geometry comes from the `orbit-prop` library in `crates/orbit-prop`, whose README documents its models, accuracy and limits.

## The comparison table
~~~~

**Edit 5.** Replace:

~~~~markdown
* **Preset data can age.**
~~~~

with:

~~~~markdown
* **Pass prediction is assessment grade.** Earth orientation uses mean sidereal time only, the Sun and Moon use low-precision formulas, and atmospheric refraction is ignored. Positions are good to about 0.01 degrees, well inside TLE error, but this is not astrometry. The upgrade path is listed in `crates/orbit-prop/README.md` under "Known limitations and future work".
* **Short grazing passes can be missed.** The pass search steps at one sixtieth of the orbital period (90 seconds for the ISS), so a pass that stays above the minimum elevation for less than that may not be found.
* **What-if orbits drift.** They include J2's slow drift but no drag and no short-period terms, so they stand for a kind of orbit, not a specific satellite.
* **Preset data can age.**
~~~~

**Edit 6.** Replace:

~~~~markdown
```
src/
  main.rs      command-line entry point, interactive menus, --demo, --help
~~~~

with:

~~~~markdown
```
Cargo.toml     workspace: scope-eval (this directory) and crates/orbit-prop
src/
  main.rs      command-line entry point, interactive menus, pass-prediction prompts, --demo, --help
~~~~

**Edit 7.** Replace:

~~~~markdown
  report.rs    printing evaluations, regime summaries and details, comparison tables, formula summary
  input.rs     validated terminal input helpers
```
~~~~

with:

~~~~markdown
  report.rs    printing evaluations, regime summaries and details, comparison tables, formula summary
  passes_report.rs  pass table and the per-pass "Mount can follow?" judgment
  input.rs     validated terminal input helpers
crates/orbit-prop/  satellite propagation (SGP4, Keplerian + J2), observer geometry, lighting, pass finding
```
~~~~

**Edit 8.** Replace:

~~~~markdown
Molniya apogee rate, the L-350 keyhole and the GEO timing requirement.
~~~~

with:

~~~~markdown
Molniya apogee rate, the L-350 keyhole and the GEO timing requirement. It also runs the `orbit-prop` tests, which check the library against published references: Vallado's GMST, site-vector and SGP4 verification cases, and Meeus's Sun and Moon examples.
~~~~

- [ ] **Step 5: Verify end to end**

Run: `cargo test`
Expected: `orbit-prop 56 passed` (+1 doctest), `scope-eval 129 passed`.

Run: `cargo run -q -- --demo > target/demo-after.txt && cmp target/demo-before.txt target/demo-after.txt && echo SAME`
Expected: `SAME`.

Run (ISS, no configurations evaluated):

~~~~bash
printf '\n\n5\n40\n-75\n100\n1\nISS (ZARYA)\n1 25544U 98067A   08264.51782528 -.00002182  00000-0 -11606-4 0  2927\n2 25544  51.6416 247.4627 0006703 130.5360 325.0288 15.72125391563537\n2008-09-20T12:00\n24\n\n8\n' | cargo run -q 2>/dev/null | sed -n '/Passes of/,/Main menu/p'
~~~~

Expected: five passes, the first two being

~~~~text
 1  2008-09-20 22:51:52 22:55:11     3m20s  14.1   0.396   0.066 yes     no
 2  2008-09-21 00:25:43 00:31:27     5m44s  48.1   1.358   0.355 partial yes
~~~~

with no mount column, and the footnote text.

Run (one L-350 configuration with an acceleration of 2 deg/s^2, then a 550 km, 53 deg what-if orbit):

~~~~bash
printf '\n\n1\n1\n1\n1\n\n2\n3\n\n\n\n\n\n\n5\n40\n-75\n100\n2\n550\n\n53\n\n\n\n\n2026-10-04T00:00\n12\n\n8\n' | cargo run -q 2>/dev/null | sed -n '/Passes of/,/Main menu/p'
~~~~

Expected: a `Mount 1` column, with pass 2 (max elevation 86.3) showing `[WARN] az accel 1.3x` and the others `[PASS] az rate ...x`.

Run (stale TLE): the ISS command above with `2008-09-20T12:00` replaced by `2008-11-01T00:00`.
Expected: the output contains `Note: this TLE's epoch (2008-09-20T12:25:40.104Z) is 41 days from the search start.`

- [ ] **Step 6: Commit**

~~~~bash
git add src/main.rs README.md
git commit -m "Add pass-prediction menu item and document it

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
~~~~

