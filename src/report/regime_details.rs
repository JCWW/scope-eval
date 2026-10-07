//! Every orbital-regime check for one configuration, with its numbers.

use std::fmt;

use super::{word, write_check_body, ReportStyle, RULE};
use crate::checks::Evaluation;
use crate::regimes::Component;

/// Full detail for every regime check of one configuration, with or without
/// each check's equations.
pub struct RegimeDetails<'a>(pub &'a Evaluation, pub ReportStyle);

impl fmt::Display for RegimeDetails<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let RegimeDetails(ev, style) = *self;

        writeln!(f, "\n{RULE}")?;
        writeln!(f, " Orbital-regime evaluation: {}", ev.label)?;
        writeln!(f, "{RULE}")?;
        for r in &ev.regimes {
            let g = &r.regime;
            writeln!(f, "\n--- {} : {} -- {} ---", g.key, g.name, g.case)?;
            writeln!(f,
                "    range {:.0} km | vs stars {:.2}\"/s | vs ground {:.2}\"/s | prediction error {:.0} km | usual mode: {} | window {}",
                g.range_km,
                g.rate_vs_stars,
                g.rate_vs_ground,
                g.ephemeris_uncertainty_km,
                g.mode.describe(),
                match g.usable_window_s {
                    Some(w) => format!("{w:.0} s"),
                    None => "unlimited".to_string(),
                }
            )?;
            for comp in [Component::Telescope, Component::Camera, Component::Mount, Component::System] {
                for k in r.checks.iter().filter(|k| k.component == comp) {
                    writeln!(f, "\n  {} {}: {}", k.status.tag(), comp.name(), k.title)?;
                    write_check_body(f, "         ", &k.details, &k.equations, &k.verdict, style)?;
                }
            }
            writeln!(f,
                "\n  {} overall: telescope {}, camera {}, mount {}, system {}",
                g.key,
                word(r.component_status(Component::Telescope)),
                word(r.component_status(Component::Camera)),
                word(r.component_status(Component::Mount)),
                word(r.component_status(Component::System))
            )?;
        }
        Ok(())
    }
}
