//! Turning evaluations into the text the CLI prints.
//!
//! Every report is a value that implements `Display`: it builds text and
//! never writes to the terminal itself. The CLI prints them with
//! `print!("{}", ...)`; tests and other front ends can call `.to_string()`.

mod comparison;
mod evaluation;
mod formulas;
mod passes;
mod regime_details;

pub use comparison::ComparisonReport;
pub use evaluation::{EvaluationReport, RegimeSummary};
pub use formulas::FORMULAS;
pub use passes::PassTable;
pub use regime_details::RegimeDetails;

use std::fmt;

use crate::checks::Status;

/// How much a report shows. The two styles print the same checks, numbers
/// and verdicts; `WithEquations` adds, under each check, the equations
/// behind its numbers with the values substituted and the rule that set
/// its status, so a reviewer can recompute and confirm it by hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReportStyle {
    #[default]
    Standard,
    WithEquations,
}

impl ReportStyle {
    pub fn describe(&self) -> &'static str {
        match self {
            ReportStyle::Standard => "standard (no equations)",
            ReportStyle::WithEquations => "with equations",
        }
    }
}

/// One check's details, then (in the equations style) its equations, then
/// its verdict. `indent` is the margin for the details.
pub(crate) fn write_check_body(
    f: &mut fmt::Formatter<'_>,
    indent: &str,
    details: &[String],
    equations: &[String],
    verdict: &str,
    style: ReportStyle,
) -> fmt::Result {
    for d in details {
        writeln!(f, "{indent}{d}")?;
    }
    if style == ReportStyle::WithEquations && !equations.is_empty() {
        writeln!(f, "{indent}Equations:")?;
        for e in equations {
            writeln!(f, "{indent}  {e}")?;
        }
    }
    writeln!(f, "{indent}-> {verdict}")
}

pub(crate) const RULE: &str = "==========================================================================";

pub(crate) fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(n - 1).collect();
        out.push('~');
        out
    }
}

pub(crate) fn word(s: Status) -> &'static str {
    match s {
        Status::Pass => "PASS",
        Status::Warn => "WARN",
        Status::Fail => "FAIL",
        Status::Info => "info",
    }
}

pub(crate) fn letter(s: Status) -> &'static str {
    match s {
        Status::Pass => "P",
        Status::Warn => "W",
        Status::Fail => "F",
        Status::Info => "i",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checks::evaluate_all;
    use crate::constants::{DEFAULT_SEEING_ARCSEC, DEFAULT_WAVELENGTH_UM};
    use crate::model::{Config, Payload, Site};
    use crate::presets;

    fn site() -> Site {
        Site { seeing_arcsec: DEFAULT_SEEING_ARCSEC, wavelength_um: DEFAULT_WAVELENGTH_UM, sky_mag_arcsec2: None, location: None }
    }

    fn config() -> Config {
        Config {
            label: "Test configuration".into(),
            telescope: presets::telescopes()[0].clone(),
            camera: presets::cameras()[0].clone(),
            payload: Payload { mount: Some(presets::mounts()[0].clone()), accessories_lb: 10.0, back_focus_required_mm: None },
            timestamp_accuracy_ms: 0.1,
            target_mag_override: None,
            exposure_override_s: None,
        }
    }

    #[test]
    fn reports_render_to_text_without_a_terminal() {
        let (cfg, site) = (config(), site());
        let evals = evaluate_all(std::slice::from_ref(&cfg), &site);
        let text = EvaluationReport(&cfg, &evals[0], &site, ReportStyle::Standard).to_string();
        assert!(text.contains(" Test configuration\n"), "{text}");
        assert!(text.contains("1. Sensor fit") && text.contains("8. Practical fit"), "{text}");
        assert!(text.contains("Orbital regimes"), "the regime summary is part of the report: {text}");
        assert!(RegimeDetails(&evals[0], ReportStyle::Standard).to_string().contains("--- LEO"));
        assert!(ComparisonReport(&evals).to_string().contains("Test configuration"));
    }

    #[test]
    fn only_the_equations_style_prints_equations() {
        let (cfg, site) = (config(), site());
        let evals = evaluate_all(std::slice::from_ref(&cfg), &site);
        let plain = EvaluationReport(&cfg, &evals[0], &site, ReportStyle::Standard).to_string();
        let full = EvaluationReport(&cfg, &evals[0], &site, ReportStyle::WithEquations).to_string();
        assert!(!plain.contains("Equations:") && !plain.contains("Rule:"), "{plain}");
        assert!(full.contains("Equations:") && full.contains("Rule:"), "{full}");
        assert!(full.contains("Plate scale: 206264.806 x (pixel (um) / 1000) / FL (mm)"), "{full}");
        // Same checks and verdicts either way: the equations style only adds lines.
        let without_equations: Vec<&str> = {
            let mut skipping = false;
            full.lines()
                .filter(|l| {
                    if l.trim() == "Equations:" {
                        skipping = true;
                        return false;
                    }
                    if skipping && l.trim_start().starts_with("->") {
                        skipping = false;
                    }
                    !skipping
                })
                .collect()
        };
        assert_eq!(without_equations, plain.lines().collect::<Vec<_>>());

        let regime_plain = RegimeDetails(&evals[0], ReportStyle::Standard).to_string();
        let regime_full = RegimeDetails(&evals[0], ReportStyle::WithEquations).to_string();
        assert!(!regime_plain.contains("Rule:") && regime_full.contains("Rule:"));
    }

    /// Every check, in every regime, for every telescope and camera on a
    /// mount and with none: each has equations, the last one is its rule,
    /// and the rule names the status the check reported.
    #[test]
    fn every_check_ends_with_the_rule_that_set_its_status() {
        let site = site();
        let mut configs = Vec::new();
        for (i, t) in presets::telescopes().into_iter().enumerate() {
            for (j, c) in presets::cameras().into_iter().enumerate() {
                let mut cfg = config();
                cfg.label = format!("{} + {}", t.name, c.name);
                cfg.telescope = t.clone();
                cfg.camera = c;
                // Cycle through the mounts, with every (len + 1)th
                // configuration on none.
                let mounts = presets::mounts();
                cfg.payload.mount = mounts.get((i + j) % (mounts.len() + 1)).cloned();
                cfg.payload.back_focus_required_mm = (j % 2 == 0).then_some(150.0);
                configs.push(cfg);
            }
        }
        let mut checked = 0;
        for (cfg, ev) in configs.iter().zip(evaluate_all(&configs, &site)) {
            let all = ev
                .checks
                .iter()
                .map(|k| (k.title, k.status, &k.equations))
                .chain(ev.regimes.iter().flat_map(|r| r.checks.iter().map(|k| (k.title, k.status, &k.equations))));
            for (title, status, equations) in all {
                let last = equations.last().unwrap_or_else(|| panic!("{}: {title} has no equations", cfg.label));
                assert!(last.starts_with("Rule: "), "{}: {title} ends with {last}", cfg.label);
                assert!(
                    last.ends_with(&format!("-> {}", status.word())),
                    "{}: {title} is {} but its rule says {last}",
                    cfg.label,
                    status.word()
                );
                checked += 1;
            }
        }
        assert!(checked > 5000, "only {checked} checks");
    }

    #[test]
    fn comparing_nothing_says_so() {
        assert_eq!(ComparisonReport(&[]).to_string(), "\nNo configurations evaluated yet.\n");
    }
}
