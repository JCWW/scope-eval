//! Pass-prediction table and the per-pass "Mount can follow?" judgment.
//!
//! The geometry comes from the `orbit-prop` library. This module only
//! applies the mount thresholds already used by the regime checks and
//! prints the result.

use orbit_prop::{Epoch, Pass, PassDarkness, PassLighting, PassResult};

use crate::checks::Status;
use crate::constants::plausible_ranges as ranges;
use crate::constants::regimes_limits as limits;
use crate::constants::STALE_TLE_DAYS;
use crate::model::{plausible, Config, Mount, MountType};

/// One axis requirement: the axis name and the pass's peak value on it.
struct AxisNeed {
    axis: &'static str,
    value: f64,
}

/// The larger of two axis requirements.
fn worst(a: AxisNeed, b: AxisNeed) -> AxisNeed {
    if b.value > a.value { b } else { a }
}

/// Peak rate and acceleration the mount's own axes must reach on this pass.
fn axis_needs(mount_type: MountType, p: &Pass) -> (AxisNeed, AxisNeed) {
    let altaz = (
        worst(AxisNeed { axis: "az", value: p.peak_az_rate_deg_s }, AxisNeed { axis: "el", value: p.peak_el_rate_deg_s }),
        worst(AxisNeed { axis: "az", value: p.peak_az_accel_deg_s2 }, AxisNeed { axis: "el", value: p.peak_el_accel_deg_s2 }),
    );
    let equatorial = (
        worst(AxisNeed { axis: "HA", value: p.peak_ha_rate_deg_s }, AxisNeed { axis: "dec", value: p.peak_dec_rate_deg_s }),
        worst(AxisNeed { axis: "HA", value: p.peak_ha_accel_deg_s2 }, AxisNeed { axis: "dec", value: p.peak_dec_accel_deg_s2 }),
    );
    match mount_type {
        MountType::AltAz => altaz,
        MountType::Equatorial => equatorial,
        MountType::Unknown => (worst(altaz.0, equatorial.0), worst(altaz.1, equatorial.1)),
    }
}

/// Judge one requirement against a rating, with the regime-check thresholds.
fn judge_axis(
    need: &AxisNeed,
    quantity: &str,
    rating: Option<f64>,
    pass_headroom: f64,
    warn_headroom: f64,
    matters: f64,
) -> (Status, String) {
    match rating {
        None if need.value > matters => (Status::Warn, format!("{} {quantity} unknown", need.axis)),
        None => (Status::Info, format!("{quantity} unknown")),
        Some(_) if need.value <= 0.0 => (Status::Pass, format!("{quantity} not needed")),
        Some(max) => {
            let headroom = max / need.value;
            let status = if headroom >= pass_headroom {
                Status::Pass
            } else if headroom >= warn_headroom {
                Status::Warn
            } else {
                Status::Fail
            };
            (status, format!("{} {quantity} {headroom:.1}x", need.axis))
        }
    }
}

/// Can this mount follow this pass? Compares the pass's peak axis rate and
/// acceleration with the mount's ratings, using the same headroom thresholds
/// as the regime checks. For an alt-az mount a near-zenith pass shows up
/// directly as a large azimuth rate and acceleration, so the keyhole needs
/// no separate formula. Returns the worse of the two judgments and a short
/// note naming the binding axis.
pub fn judge_mount_for_pass(m: &Mount, p: &Pass) -> (Status, String) {
    let (rate_need, accel_need) = axis_needs(m.mount_type, p);
    let rate = judge_axis(
        &rate_need,
        "rate",
        plausible(m.max_slew_deg_s, ranges::SLEW_RATE_MIN_DEG_S, ranges::SLEW_RATE_MAX_DEG_S),
        limits::RATE_PASS_HEADROOM,
        limits::RATE_WARN_HEADROOM,
        limits::RATE_MATTERS_DEG_S,
    );
    let accel = judge_axis(
        &accel_need,
        "accel",
        plausible(m.max_accel_deg_s2, ranges::ACCEL_MIN_DEG_S2, ranges::ACCEL_MAX_DEG_S2),
        limits::ACCEL_PASS_HEADROOM,
        limits::ACCEL_WARN_HEADROOM,
        limits::ACCEL_MATTERS_DEG_S2,
    );
    if accel.0 > rate.0 { accel } else { rate }
}

/// A warning when any part of the search window is far from the TLE's
/// epoch, because SGP4 errors grow quickly with element age.
pub fn stale_tle_note(tle_epoch: &Epoch, start: &Epoch, end: &Epoch) -> Option<String> {
    let age_days = start.seconds_since(tle_epoch).abs().max(end.seconds_since(tle_epoch).abs()) / 86_400.0;
    (age_days > STALE_TLE_DAYS).then(|| {
        format!(
            "Note: parts of the search window are up to {age_days:.0} days from this TLE's epoch ({tle_epoch}).\n  SGP4 errors grow quickly with age; pass times may be off by minutes."
        )
    })
}

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

/// Print the pass table. One "Mount can follow?" column is added for each
/// evaluated configuration that has a mount.
pub fn print_passes(label: &str, result: &PassResult, configs: &[Config]) {
    let mounted: Vec<(&str, &Mount)> =
        configs.iter().filter_map(|c| c.payload.mount.as_ref().map(|m| (c.label.as_str(), m))).collect();
    println!("\n==========================================================================");
    println!(" Passes of {label}");
    println!("==========================================================================");
    if result.passes.is_empty() {
        println!(" No passes in this window.");
    } else {
        print!(" #  Rise (UTC)            Set (UTC)  Duration MaxEl  Az rate El rate Sunlit  Dark    ");
        for (i, _) in mounted.iter().enumerate() {
            print!(" | {:<22}", format!("Mount {}", i + 1));
        }
        println!();
        for (n, p) in result.passes.iter().enumerate() {
            let start_mark = if p.clipped_start { "<" } else { " " };
            let end_mark = if p.clipped_end { ">" } else { " " };
            print!(
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
            );
            for (_, m) in &mounted {
                let (status, note) = judge_mount_for_pass(m, p);
                print!(" | {} {:<15}", status.tag(), note);
            }
            println!();
        }
        println!("\n Rates are peak axis rates in deg/s for an alt-az mount. '<' = already up at");
        println!(" the start of the window, '>' = still up at the end. Sunlit: is the satellite");
        println!(" in sunlight. Dark: is the Sun more than 12 deg below the site's horizon.");
        if !mounted.is_empty() {
            println!("\n Mount columns compare the pass's peak axis rate and acceleration with the");
            println!(" mount's ratings (PASS at 3x headroom, WARN at 1x, FAIL below; INFO or WARN");
            println!(" when a rating is unknown). The note names the binding axis and its headroom.");
            for (i, (cfg_label, m)) in mounted.iter().enumerate() {
                println!("   Mount {}: {} ({cfg_label})", i + 1, m.name);
            }
        }
    }
    if let Some(e) = &result.error {
        println!("\n The search stopped early: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Capability;

    fn pass(az_rate: f64, el_rate: f64, az_acc: f64, el_acc: f64) -> Pass {
        let t = Epoch::from_utc(2026, 10, 4, 0, 0, 0.0).unwrap();
        Pass {
            rise: t,
            culmination: t.add_seconds(300.0),
            set: t.add_seconds(600.0),
            clipped_start: false,
            clipped_end: false,
            max_el_deg: 60.0,
            peak_az_rate_deg_s: az_rate,
            peak_el_rate_deg_s: el_rate,
            peak_az_accel_deg_s2: az_acc,
            peak_el_accel_deg_s2: el_acc,
            peak_ha_rate_deg_s: 0.5,
            peak_dec_rate_deg_s: 0.2,
            peak_ha_accel_deg_s2: 0.001,
            peak_dec_accel_deg_s2: 0.002,
            peak_rate_vs_ground_deg_s: 0.8,
            lighting: PassLighting::Sunlit,
            site_dark: PassDarkness::Dark,
        }
    }

    fn mount(kind: MountType, slew: Option<f64>, accel: Option<f64>) -> Mount {
        Mount {
            name: "test".into(),
            mount_type: kind,
            capacity_lb: None,
            max_slew_deg_s: slew,
            max_accel_deg_s2: accel,
            settle_time_s: None,
            pointing_rms_arcsec: None,
            non_sidereal_tracking: Capability::Unknown,
            source: String::new(),
        }
    }

    #[test]
    fn ample_headroom_passes_and_names_the_binding_axis() {
        let (s, note) = judge_mount_for_pass(&mount(MountType::AltAz, Some(10.0), Some(10.0)), &pass(1.0, 0.5, 0.1, 0.05));
        assert_eq!(s, Status::Pass);
        assert_eq!(note, "az rate 10.0x");
    }

    #[test]
    fn rate_headroom_between_1x_and_3x_warns_below_1x_fails() {
        let m = mount(MountType::AltAz, Some(2.0), Some(10.0));
        assert_eq!(judge_mount_for_pass(&m, &pass(1.0, 0.5, 0.1, 0.05)).0, Status::Warn);
        assert_eq!(judge_mount_for_pass(&m, &pass(4.0, 0.5, 0.1, 0.05)).0, Status::Fail);
    }

    #[test]
    fn acceleration_can_be_the_binding_limit() {
        // Near-zenith pass: rate fine, azimuth acceleration exceeds the rating.
        let (s, note) = judge_mount_for_pass(&mount(MountType::AltAz, Some(10.0), Some(1.0)), &pass(1.0, 0.5, 2.0, 0.05));
        assert_eq!(s, Status::Fail);
        assert_eq!(note, "az accel 0.5x");
    }

    #[test]
    fn unknown_ratings_warn_only_when_the_pass_demands_it() {
        let m = mount(MountType::AltAz, None, None);
        assert_eq!(judge_mount_for_pass(&m, &pass(1.0, 0.5, 0.1, 0.05)).0, Status::Warn);
        // Slow pass: below RATE_MATTERS_DEG_S and ACCEL_MATTERS_DEG_S2.
        assert_eq!(judge_mount_for_pass(&m, &pass(0.01, 0.01, 0.0001, 0.0001)).0, Status::Info);
    }

    #[test]
    fn implausible_ratings_are_treated_as_unknown() {
        let m = mount(MountType::AltAz, Some(0.0), Some(f64::NAN));
        let (s, note) = judge_mount_for_pass(&m, &pass(1.0, 0.5, 0.1, 0.05));
        assert_eq!(s, Status::Warn);
        assert!(note.contains("unknown"), "{note}");
    }

    #[test]
    fn equatorial_mount_uses_hour_angle_and_declination_axes() {
        // Huge az rate (near-zenith) is irrelevant to an equatorial mount.
        let m = mount(MountType::Equatorial, Some(2.0), Some(1.0));
        let (s, note) = judge_mount_for_pass(&m, &pass(50.0, 0.5, 20.0, 0.05));
        assert_eq!(s, Status::Pass);
        assert_eq!(note, "HA rate 4.0x");
    }

    #[test]
    fn unknown_mount_type_takes_the_worse_axis_set() {
        let m = mount(MountType::Unknown, Some(10.0), Some(10.0));
        let (s, note) = judge_mount_for_pass(&m, &pass(50.0, 0.5, 0.1, 0.05));
        assert_eq!(s, Status::Fail);
        assert_eq!(note, "az rate 0.2x");
    }

    #[test]
    fn stationary_target_needs_nothing() {
        let (s, _) = judge_mount_for_pass(&mount(MountType::AltAz, Some(5.0), Some(5.0)), &pass(0.0, 0.0, 0.0, 0.0));
        assert_eq!(s, Status::Pass);
    }

    #[test]
    fn stale_tle_is_flagged_after_two_weeks() {
        let epoch = Epoch::from_utc(2026, 10, 1, 0, 0, 0.0).unwrap();
        assert_eq!(stale_tle_note(&epoch, &epoch.add_seconds(13.0 * 86_400.0), &epoch.add_seconds(13.0 * 86_400.0)), None);
        let note = stale_tle_note(&epoch, &epoch.add_seconds(40.0 * 86_400.0), &epoch.add_seconds(40.0 * 86_400.0)).unwrap();
        assert!(note.contains("40 days"), "{note}");
        // A search far before the epoch is just as stale.
        assert!(stale_tle_note(&epoch, &epoch.add_seconds(-20.0 * 86_400.0), &epoch).is_some());
    }

    #[test]
    fn long_search_from_a_fresh_tle_is_flagged_by_its_far_end() {
        // Fresh at the start, 30 days old by the end: the late passes are stale.
        let epoch = Epoch::from_utc(2026, 10, 1, 0, 0, 0.0).unwrap();
        let note = stale_tle_note(&epoch, &epoch, &epoch.add_seconds(30.0 * 86_400.0)).unwrap();
        assert!(note.contains("30 days"), "{note}");
    }

    #[test]
    fn duration_formatting() {
        assert_eq!(duration_text(425.4), "7m05s");
        assert_eq!(duration_text(6.0 * 3600.0 + 61.0), "6h01m");
    }
}
