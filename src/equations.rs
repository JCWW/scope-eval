//! Equation lines for the reports.
//!
//! Every graded check records, next to its details, the equations behind
//! its numbers with the actual values substituted, and the rule that turned
//! the result into PASS, WARN or FAIL. A reviewer can then recompute each
//! number by hand and see which threshold decided the status. The report
//! prints them only when asked for (see `report::ReportStyle`).
//!
//! The lines are built from the same variables the check uses, never
//! recomputed separately, so they cannot drift from what was judged.

use crate::checks::Status;

/// A number to about five significant figures, enough to recompute the
/// next step by hand. Very large or small values use scientific notation.
pub fn num(x: f64) -> String {
    if x == 0.0 || !x.is_finite() {
        return format!("{x}");
    }
    let a = x.abs();
    if !(1e-3..1e7).contains(&a) {
        return format!("{x:.4e}");
    }
    let decimals = (4 - a.log10().floor() as i32).max(0) as usize;
    format!("{x:.decimals$}")
}

/// A threshold or constant as written, without padding zeros: 15 rather
/// than 15.000, 0.001 rather than 0.0010000.
pub fn short(x: f64) -> String {
    let n = num(x);
    if n.contains('e') || !n.contains('.') {
        return n;
    }
    n.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// `name: formula = substituted = result`.
pub fn eq(name: &str, formula: &str, substituted: String, result: String) -> String {
    format!("{name}: {formula} = {substituted} = {result}")
}

/// `Rule: <bands>; <value> -> STATUS`: which threshold decided the status.
pub fn rule(bands: String, value: String, status: Status) -> String {
    format!("Rule: {bands}; {value} -> {}", status.word())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_keep_about_five_significant_figures() {
        assert_eq!(num(0.738_624), "0.73862");
        assert_eq!(num(1050.0), "1050.0");
        assert_eq!(num(206_264.806), "206265");
        assert_eq!(num(2.5), "2.5000");
        assert_eq!(num(1.37e-6), "1.3700e-6");
        assert_eq!(num(8.9e9), "8.9000e9");
        assert_eq!(num(0.0), "0");
        assert_eq!(num(f64::INFINITY), "inf");
    }

    #[test]
    fn thresholds_drop_padding_zeros() {
        assert_eq!(short(0.15 * 100.0), "15");
        assert_eq!(short(0.001), "0.001");
        assert_eq!(short(2.0), "2");
        assert_eq!(short(0.25), "0.25");
        assert_eq!(short(1.37e-6), "1.3700e-6");
    }

    #[test]
    fn rule_names_the_status() {
        assert_eq!(rule("PASS if x >= 1".into(), "x = 2".into(), Status::Pass), "Rule: PASS if x >= 1; x = 2 -> PASS");
    }
}
