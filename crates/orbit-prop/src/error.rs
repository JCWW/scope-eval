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
