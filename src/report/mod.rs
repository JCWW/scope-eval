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

use crate::checks::Status;

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
        let text = EvaluationReport(&cfg, &evals[0], &site).to_string();
        assert!(text.contains(" Test configuration\n"), "{text}");
        assert!(text.contains("1. Sensor fit") && text.contains("8. Practical fit"), "{text}");
        assert!(text.contains("Orbital regimes"), "the regime summary is part of the report: {text}");
        assert!(RegimeDetails(&evals[0]).to_string().contains("--- LEO"));
        assert!(ComparisonReport(&evals).to_string().contains("Test configuration"));
    }

    #[test]
    fn comparing_nothing_says_so() {
        assert_eq!(ComparisonReport(&[]).to_string(), "\nNo configurations evaluated yet.\n");
    }
}
