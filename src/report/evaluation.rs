//! The report for one evaluated configuration.

use std::fmt;

use super::{word, RULE};
use crate::checks::{Evaluation, Status};
use crate::constants::plausible_ranges as ranges;
use crate::constants::DEFAULT_SKY_MAG_ARCSEC2;
use crate::model::plausible;
use crate::model::{Config, Shutter, Site};
use crate::regimes::Component;

/// The full report for one configuration: the eight checks, the GEO timing
/// reference and the regime summary.
pub struct EvaluationReport<'a>(pub &'a Config, pub &'a Evaluation, pub &'a Site);

impl fmt::Display for EvaluationReport<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let EvaluationReport(cfg, ev, site) = *self;

        let t = &cfg.telescope;
        let c = &cfg.camera;
        writeln!(f, "\n{RULE}")?;
        writeln!(f, " {}", ev.label)?;
        writeln!(f, "{RULE}")?;
        writeln!(f,
            " Telescope: {}  (D = {:.0} mm, FL = {:.0} mm, f/{:.2})",
            t.name,
            t.aperture_mm,
            t.focal_length_mm,
            t.f_ratio()
        )?;
        let shutter = match c.shutter {
            Shutter::Global => "global shutter",
            Shutter::Rolling { .. } => "rolling shutter",
        };
        writeln!(f,
            " Camera:    {}  ({:.2} um pixels, {} x {} px, {shutter})",
            c.name, c.pixel_um, c.width_px, c.height_px
        )?;
        writeln!(f,
            " Site:      seeing {:.2}\" FWHM, wavelength {:.2} um, sky {}",
            site.seeing_arcsec,
            site.wavelength_um,
            sky_label(site.sky_mag_arcsec2)
        )?;

        for chk in &ev.checks {
            writeln!(f, "\n{} {}. {}", chk.status.tag(), chk.number, chk.title)?;
            for d in &chk.details {
                writeln!(f, "       {d}")?;
            }
            writeln!(f, "       -> {}", chk.verdict)?;
        }

        if let Some(m) = &cfg.payload.mount {
            writeln!(f, "\n Mount:     {}", m.name)?;
        }
        writeln!(f, " Timestamp accuracy: {} ms", cfg.timestamp_accuracy_ms)?;

        writeln!(f, "\n[----] Timing reference, GEO stare mode (supplementary, not scored)")?;
        for s in &ev.supplementary {
            writeln!(f, "       {s}")?;
        }

        write!(f, "{}", RegimeSummary(ev))?;

        let count = |s: Status| ev.checks.iter().filter(|c| c.status == s).count();
        writeln!(f,
            "\n Summary of the eight checks: {} pass, {} warn, {} fail, {} info",
            count(Status::Pass),
            count(Status::Warn),
            count(Status::Fail),
            count(Status::Info)
        )?;
        Ok(())
    }
}

/// Compact component-by-regime table printed with every evaluation.
pub struct RegimeSummary<'a>(pub &'a Evaluation);

impl fmt::Display for RegimeSummary<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let RegimeSummary(ev) = *self;

        writeln!(f, "\n[----] Orbital regimes (worst status per component; details via the menu or --demo)")?;
        writeln!(f,
            "       {:<31} {:<10} {:<8} {:<8} {:<8} {}",
            "Regime", "Telescope", "Camera", "Mount", "System", "Overall"
        )?;
        for r in &ev.regimes {
            writeln!(f,
                "       {:<31} {:<10} {:<8} {:<8} {:<8} {}",
                format!("{} ({})", r.regime.key, r.regime.name),
                word(r.component_status(Component::Telescope)),
                word(r.component_status(Component::Camera)),
                word(r.component_status(Component::Mount)),
                word(r.component_status(Component::System)),
                word(r.overall())
            )?;
        }
        Ok(())
    }
}

/// How the site's sky brightness should read in the report header.
///
/// A value outside the plausible range is not echoed back as fact: the
/// detection check substituted a default for it, and a header that confirmed
/// the original would contradict the check in the same report.
fn sky_label(sky_mag_arcsec2: Option<f64>) -> String {
    match plausible(sky_mag_arcsec2, ranges::SKY_MAG_MIN, ranges::SKY_MAG_MAX) {
        Some(s) => format!("{s:.2} mag/arcsec^2"),
        None => match sky_mag_arcsec2 {
            Some(bad) => format!(
                "{DEFAULT_SKY_MAG_ARCSEC2:.2} mag/arcsec^2 (assumed; {bad:.2} is outside {:.0}-{:.0})",
                ranges::SKY_MAG_MIN,
                ranges::SKY_MAG_MAX
            ),
            None => format!("{DEFAULT_SKY_MAG_ARCSEC2:.2} mag/arcsec^2 (assumed)"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sky_label_reports_an_entered_value_plainly() {
        assert_eq!(sky_label(Some(21.9)), "21.90 mag/arcsec^2");
    }

    #[test]
    fn sky_label_marks_an_absent_value_as_assumed() {
        assert_eq!(sky_label(None), "21.00 mag/arcsec^2 (assumed)");
    }

    #[test]
    fn sky_label_does_not_echo_an_implausible_value_back_as_fact() {
        // 2.1 for 21.0 is a plausible typo, and brighter than daylight. The
        // model substitutes 21.0 and names it; the header must not contradict
        // that by confirming the typo to the reader.
        let label = sky_label(Some(2.1));
        assert!(label.starts_with("21.00 mag/arcsec^2 (assumed"), "got {label}");
        assert!(label.contains("2.10"), "the rejected value should still be shown: {label}");
        assert!(label.contains("outside"), "say why it was rejected: {label}");
    }

    #[test]
    fn sky_label_rejects_an_impossibly_dark_sky() {
        assert!(sky_label(Some(30.0)).starts_with("21.00 mag/arcsec^2 (assumed"));
    }
}
