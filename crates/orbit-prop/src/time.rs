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
