//! The side-by-side comparison of every evaluated configuration.

use std::fmt;

use super::{letter, truncate, RULE};
use crate::calculations::optics::OpticsCalculator;
use crate::checks::{Evaluation, Status};
use crate::regimes::Component;

/// The comparison table. The first evaluation is the reference for depth
/// and search speed.
pub struct ComparisonReport<'a>(pub &'a [Evaluation]);

impl fmt::Display for ComparisonReport<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ComparisonReport(evals) = *self;

        if evals.is_empty() {
            writeln!(f, "\nNo configurations evaluated yet.")?;
            return Ok(());
        }
        let reference = &evals[0];
        writeln!(f, "\n{RULE}")?;
        writeln!(f, " Comparison (reference = first configuration: {})", reference.label)?;
        writeln!(f, "{RULE}")?;
        writeln!(f,
            "{:<28} {:>6} {:>6} {:>4} {:>7} {:>7} {:>6} {:>6} {:>6} {:>6}",
            "Configuration", "\"/px", "px/*", "bin", "FOVdeg2", "Area m2", "dMag", "Search", "CFZ+/-", "Load%"
        )?;
        for e in evals {
            let m = &e.metrics;
            let dmag = OpticsCalculator::delta_mag(m.effective_area_m2, reference.metrics.effective_area_m2);
            let search = m.etendue / reference.metrics.etendue;
            let load = m
                .payload_fraction
                .map(|f| format!("{:.0}", f * 100.0))
                .unwrap_or_else(|| "-".to_string());
            writeln!(f,
                "{:<28} {:>6.2} {:>6.1} {:>4} {:>7.2} {:>7.4} {:>+6.2} {:>5.2}x {:>6.1} {:>6}",
                truncate(&e.label, 28),
                m.plate_scale,
                m.pixels_across,
                format!("{}x{}", m.recommended_bin, m.recommended_bin),
                m.fov_area_deg2,
                m.effective_area_m2,
                dmag,
                search,
                m.cfz_um,
                load
            )?;
        }
        writeln!(f, "\n Columns: plate scale, pixels across a star (native), best bin, field area,")?;
        writeln!(f, " effective collecting area, depth vs reference (+ is fainter), search speed")?;
        writeln!(f, " (etendue) vs reference, critical focus zone in um, payload as % of mount rating.")?;

        writeln!(f, "\n Status by check (P=pass W=warn F=fail i=info):")?;
        writeln!(f, "{:<28}  1 2 3 4 5 6 7 8", "")?;
        for e in evals {
            let row: Vec<&str> = e
                .checks
                .iter()
                .map(|c| match c.status {
                    Status::Pass => "P",
                    Status::Warn => "W",
                    Status::Fail => "F",
                    Status::Info => "i",
                })
                .collect();
            writeln!(f, "{:<28}  {}", truncate(&e.label, 28), row.join(" "))?;
        }

        if let Some(first) = evals.first() {
            writeln!(f, "\n Orbital regimes, telescope/camera/mount/system (P=pass W=warn F=fail i=info):")?;
            let header: Vec<String> = first.regimes.iter().map(|r| format!("{:<9}", r.regime.key)).collect();
            writeln!(f, "{:<28}  {}", "", header.join(" "))?;
            for e in evals {
                let cells: Vec<String> = e
                    .regimes
                    .iter()
                    .map(|r| {
                        format!(
                            "{:<9}",
                            format!(
                                "{}/{}/{}/{}",
                                letter(r.component_status(Component::Telescope)),
                                letter(r.component_status(Component::Camera)),
                                letter(r.component_status(Component::Mount)),
                                letter(r.component_status(Component::System))
                            )
                        )
                    })
                    .collect();
                writeln!(f, "{:<28}  {}", truncate(&e.label, 28), cells.join(" "))?;
            }
        }
        Ok(())
    }
}
