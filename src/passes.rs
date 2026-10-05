//! Pass prediction for the CLI: turning an orbit source into a propagator,
//! and judging whether a mount can follow each pass.
//!
//! The geometry comes from the `orbit-prop` library. This module applies
//! the mount thresholds already used by the regime checks; rendering the
//! pass table is `report::passes`.

use orbit_prop::{Epoch, KeplerElements, KeplerJ2, OrbitPropError, Pass, Propagator, Sgp4Propagator, Tle};

use crate::checks::Status;
use crate::constants::plausible_ranges as ranges;
use crate::constants::regimes_limits as limits;
use crate::constants::STALE_TLE_DAYS;
use crate::model::{plausible, Mount, MountType};

/// Where an orbit comes from. A what-if orbit with no epoch takes the
/// search start as its epoch.
pub enum OrbitSource {
    Tle(Tle),
    WhatIf {
        perigee_km: f64,
        apogee_km: f64,
        i_deg: f64,
        raan_deg: f64,
        argp_deg: f64,
        mean_anomaly_deg: f64,
        epoch: Option<Epoch>,
    },
}

/// A propagator for `source`, plus a note when a TLE is too old for the
/// search window from `start` to `end`.
pub fn build_propagator(
    source: OrbitSource,
    start: Epoch,
    end: Epoch,
) -> Result<(Box<dyn Propagator>, Option<String>), OrbitPropError> {
    match source {
        OrbitSource::Tle(tle) => {
            let note = stale_tle_note(&tle.epoch, &start, &end);
            Ok((Box::new(Sgp4Propagator::new(&tle)?), note))
        }
        OrbitSource::WhatIf { perigee_km, apogee_km, i_deg, raan_deg, argp_deg, mean_anomaly_deg, epoch } => {
            let el = KeplerElements::from_altitudes(
                epoch.unwrap_or(start),
                perigee_km,
                apogee_km,
                i_deg,
                raan_deg,
                argp_deg,
                mean_anomaly_deg,
            )?;
            let label = format!("what-if orbit {perigee_km:.0} x {apogee_km:.0} km, {i_deg:.1} deg");
            Ok((Box::new(KeplerJ2::new(el, &label)?), None))
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Capability;
    use orbit_prop::{PassDarkness, PassLighting};

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
}
