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
