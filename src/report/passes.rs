//! The pass-prediction table, with a "Mount can follow?" column per mount.

use std::fmt;

use orbit_prop::{Epoch, PassDarkness, PassLighting, PassResult};

use crate::model::{Config, Mount};
use crate::passes::judge_mount_for_pass;

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

/// The pass table for `label`'s passes. One "Mount N" column is added for
/// each configuration that has a mount, judged by `judge_mount_for_pass`.
pub struct PassTable<'a>(pub &'a str, pub &'a PassResult, pub &'a [Config]);

impl fmt::Display for PassTable<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let PassTable(label, result, configs) = *self;

        let mounted: Vec<(&str, &Mount)> =
            configs.iter().filter_map(|c| c.payload.mount.as_ref().map(|m| (c.label.as_str(), m))).collect();
        writeln!(f, "\n==========================================================================")?;
        writeln!(f, " Passes of {label}")?;
        writeln!(f, "==========================================================================")?;
        if result.passes.is_empty() {
            writeln!(f, " No passes in this window.")?;
        } else {
            write!(f, " #  Rise (UTC)            Set (UTC)  Duration MaxEl  Az rate El rate Sunlit  Dark    ")?;
            for (i, _) in mounted.iter().enumerate() {
                write!(f, " | {:<22}", format!("Mount {}", i + 1))?;
            }
            writeln!(f)?;
            for (n, p) in result.passes.iter().enumerate() {
                let start_mark = if p.clipped_start { "<" } else { " " };
                let end_mark = if p.clipped_end { ">" } else { " " };
                write!(f,
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
                )?;
                for (_, m) in &mounted {
                    let (status, note) = judge_mount_for_pass(m, p);
                    write!(f, " | {} {:<15}", status.tag(), note)?;
                }
                writeln!(f)?;
            }
            writeln!(f, "\n Rates are peak axis rates in deg/s for an alt-az mount. '<' = already up at")?;
            writeln!(f, " the start of the window, '>' = still up at the end. Sunlit: is the satellite")?;
            writeln!(f, " in sunlight. Dark: is the Sun more than 12 deg below the site's horizon.")?;
            if !mounted.is_empty() {
                writeln!(f, "\n Mount columns compare the pass's peak axis rate and acceleration with the")?;
                writeln!(f, " mount's ratings (PASS at 3x headroom, WARN at 1x, FAIL below; INFO or WARN")?;
                writeln!(f, " when a rating is unknown). The note names the binding axis and its headroom.")?;
                for (i, (cfg_label, m)) in mounted.iter().enumerate() {
                    writeln!(f, "   Mount {}: {} ({cfg_label})", i + 1, m.name)?;
                }
            }
        }
        if let Some(e) = &result.error {
            writeln!(f, "\n The search stopped early: {e}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_formatting() {
        assert_eq!(duration_text(425.4), "7m05s");
        assert_eq!(duration_text(6.0 * 3600.0 + 61.0), "6h01m");
    }
}
