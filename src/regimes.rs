//! Component evaluations (telescope, camera, mount) against orbital regimes.
//!
//! Each regime is described by a few representative numbers: how far away the
//! target is, how fast it moves against the stars and against the ground, how
//! uncertain its predicted position typically is, and how it is usually
//! observed. The checks then ask, component by component, whether a
//! configuration can acquire, time-tag and follow that kind of target.
//!
//! All regime numbers are representative worst-to-typical cases, documented
//! in README.md. Change them in `regimes()` to match your own catalog.

use crate::checks::{kv, Evaluation, Status};
use crate::constants::{
    regimes_limits as limits, ARCSEC_PER_CIRCLE, ARCSEC_PER_DEGREE, ARCSEC_PER_RADIAN, DEFAULT_POINTING_RMS_ARCSEC,
    EARTH_RADIUS_KM, MS_PER_S, MU_EARTH, S_PER_US, SIDEREAL_RATE_ARCSEC_PER_S,
};
use crate::model::{Capability, Config, MountType, Shutter, Site};

/// How a regime's targets are usually observed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrackingMode {
    /// Mount stopped. Earth-fixed targets stay put while stars streak.
    Stare,
    /// Mount follows the stars. Slow targets drift slightly.
    Sidereal,
    /// Mount follows the target's predicted path. Stars streak.
    RateTrack,
}

impl TrackingMode {
    pub fn describe(&self) -> &'static str {
        match self {
            TrackingMode::Stare => "stare (tracking off)",
            TrackingMode::Sidereal => "sidereal tracking",
            TrackingMode::RateTrack => "rate-track the target",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Regime {
    pub key: &'static str,
    pub name: &'static str,
    /// The representative case the numbers describe.
    pub case: &'static str,
    /// Observer-to-target distance, km.
    pub range_km: f64,
    /// Target's apparent motion relative to the background stars, arcsec/s.
    pub rate_vs_stars: f64,
    /// Target's apparent motion relative to the ground (what the mount must follow), arcsec/s.
    pub rate_vs_ground: f64,
    /// Typical along-track uncertainty of the predicted position (e.g. TLE), km. An assumption.
    pub ephemeris_uncertainty_km: f64,
    pub mode: TrackingMode,
    /// How long the target stays usable per opportunity, seconds. `None` means
    /// effectively unlimited, so slew time does not compete with the window.
    pub usable_window_s: Option<f64>,
    /// Whether following the target requires non-sidereal tracking.
    pub needs_non_sidereal: bool,
    /// What usually limits detection in this regime.
    pub limiting_factor: &'static str,
}

/// Circular-orbit speed at altitude h, km/s:  v = sqrt(mu / (R + h)).
pub fn circular_speed_km_s(altitude_km: f64) -> f64 {
    (MU_EARTH / (EARTH_RADIUS_KM + altitude_km)).sqrt()
}

/// Angular rate of a satellite passing directly overhead, arcsec/s:  omega ~ v / h.
/// Ignores Earth's rotation (small for LEO, roughly 10% for MEO).
pub fn overhead_rate_arcsec_s(altitude_km: f64) -> f64 {
    circular_speed_km_s(altitude_km) / altitude_km * ARCSEC_PER_RADIAN
}

/// Rate against the stars for an orbit with the given period, arcsec/s:  1,296,000 / period.
pub fn rate_from_period_arcsec_s(period_s: f64) -> f64 {
    ARCSEC_PER_CIRCLE / period_s
}

/// Speed anywhere on an orbit, km/s (vis-viva):  v = sqrt(mu * (2/r - 1/a)).
pub fn vis_viva_km_s(radius_km: f64, semi_major_axis_km: f64) -> f64 {
    (MU_EARTH * (2.0 / radius_km - 1.0 / semi_major_axis_km)).sqrt()
}

/// The representative regimes. Edit here to match your own targets.
pub fn regimes() -> Vec<Regime> {
    // LEO: 500 km circular orbit passing overhead (the fastest geometry).
    let leo_alt = 500.0;
    let leo_rate = overhead_rate_arcsec_s(leo_alt);

    // MEO: GPS-like 20,200 km circular orbit passing overhead.
    let meo_alt = 20_200.0;
    let meo_rate = overhead_rate_arcsec_s(meo_alt);

    // HEO: Molniya orbit (a = 26,560 km, e = 0.74) observed near apogee.
    let molniya_a = 26_560.0;
    let molniya_apogee_r = molniya_a * (1.0 + 0.74);
    let molniya_apogee_alt = molniya_apogee_r - EARTH_RADIUS_KM;
    let heo_rate = vis_viva_km_s(molniya_apogee_r, molniya_a) / molniya_apogee_alt * ARCSEC_PER_RADIAN;

    // Cislunar: lunar distance, moving with roughly the Moon's motion against the stars.
    let lunar_rate = rate_from_period_arcsec_s(27.321_661 * 86_400.0);

    vec![
        Regime {
            key: "LEO",
            name: "Low Earth orbit",
            case: "500 km circular orbit, overhead pass",
            range_km: leo_alt,
            rate_vs_stars: leo_rate,
            rate_vs_ground: leo_rate,
            ephemeris_uncertainty_km: 2.0,
            mode: TrackingMode::RateTrack,
            usable_window_s: Some(300.0),
            needs_non_sidereal: true,
            limiting_factor: "tracking speed, timing and acquisition (targets are usually bright)",
        },
        Regime {
            key: "MEO",
            name: "Medium Earth orbit",
            case: "GPS-like 20,200 km orbit, overhead pass",
            range_km: meo_alt,
            rate_vs_stars: meo_rate,
            rate_vs_ground: meo_rate,
            ephemeris_uncertainty_km: 2.0,
            mode: TrackingMode::RateTrack,
            usable_window_s: None,
            needs_non_sidereal: true,
            limiting_factor: "brightness and exposure time (moderate rates)",
        },
        Regime {
            key: "GEO",
            name: "Geosynchronous orbit",
            case: "35,786 km altitude, ~37,000 km slant range from mid-latitudes",
            range_km: 37_000.0,
            rate_vs_stars: SIDEREAL_RATE_ARCSEC_PER_S,
            rate_vs_ground: 0.0,
            ephemeris_uncertainty_km: 2.0,
            mode: TrackingMode::Stare,
            usable_window_s: None,
            needs_non_sidereal: false,
            limiting_factor: "brightness and search speed (targets are faint and Earth-fixed)",
        },
        Regime {
            key: "HEO",
            name: "Highly elliptical orbit",
            case: "Molniya orbit near apogee (~39,800 km)",
            range_km: molniya_apogee_alt,
            rate_vs_stars: heo_rate,
            rate_vs_ground: (SIDEREAL_RATE_ARCSEC_PER_S - heo_rate).abs(),
            ephemeris_uncertainty_km: 5.0,
            mode: TrackingMode::RateTrack,
            usable_window_s: None,
            needs_non_sidereal: true,
            limiting_factor: "brightness, plus orbit-prediction uncertainty",
        },
        Regime {
            key: "CIS",
            name: "Cislunar space",
            case: "lunar distance (~384,400 km)",
            range_km: 384_400.0,
            rate_vs_stars: lunar_rate,
            rate_vs_ground: SIDEREAL_RATE_ARCSEC_PER_S - lunar_rate,
            ephemeris_uncertainty_km: 50.0,
            mode: TrackingMode::Sidereal,
            usable_window_s: None,
            needs_non_sidereal: false,
            limiting_factor: "brightness above all (targets are roughly 10x farther than GEO)",
        },
    ]
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Component {
    Telescope,
    Camera,
    Mount,
    /// The configuration as a whole. Detection needs collecting area, plate
    /// scale, quantum efficiency and sky background together, so it cannot be
    /// attributed to any single component.
    System,
}

impl Component {
    pub fn name(&self) -> &'static str {
        match self {
            Component::Telescope => "Telescope",
            Component::Camera => "Camera",
            Component::Mount => "Mount",
            Component::System => "System",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RegimeCheck {
    pub component: Component,
    pub title: &'static str,
    pub status: Status,
    pub details: Vec<String>,
    pub verdict: String,
}

#[derive(Debug, Clone)]
pub struct RegimeEvaluation {
    pub regime: Regime,
    pub checks: Vec<RegimeCheck>,
}

impl RegimeEvaluation {
    /// Worst status for one component (Info if it has no graded checks).
    pub fn component_status(&self, c: Component) -> Status {
        self.checks
            .iter()
            .filter(|k| k.component == c)
            .map(|k| k.status)
            .max()
            .unwrap_or(Status::Info)
    }
    pub fn overall(&self) -> Status {
        self.checks.iter().map(|k| k.status).max().unwrap_or(Status::Info)
    }
}

/// Seconds for an angular rate, formatted readably.
fn fmt_duration(seconds: f64) -> String {
    if !seconds.is_finite() {
        "indefinitely".to_string()
    } else if seconds < 1e-3 {
        format!("{:.0} us", seconds * 1e6)
    } else if seconds < 1.0 {
        format!("{:.1} ms", seconds * 1e3)
    } else if seconds < 120.0 {
        format!("{seconds:.1} s")
    } else if seconds < 7200.0 {
        format!("{:.1} min", seconds / 60.0)
    } else {
        format!("{:.1} h", seconds / 3600.0)
    }
}

fn fmt_rate(arcsec_s: f64) -> String {
    if arcsec_s >= 360.0 {
        format!("{:.0}\"/s ({:.2} deg/s)", arcsec_s, arcsec_s / 3600.0)
    } else {
        format!("{arcsec_s:.2}\"/s")
    }
}

// ---------------------------------------------------------------------------
// Telescope checks
// ---------------------------------------------------------------------------

/// Is the field wide enough to catch the target despite orbit and pointing errors?
fn telescope_acquisition(cfg: &Config, ev: &Evaluation, r: &Regime) -> RegimeCheck {
    let ephem_arcsec = r.ephemeris_uncertainty_km / r.range_km * ARCSEC_PER_RADIAN;
    let (pointing, assumed) = match cfg.payload.mount.as_ref().and_then(|m| m.pointing_rms_arcsec) {
        Some(p) => (p, false),
        None => (DEFAULT_POINTING_RMS_ARCSEC, true),
    };
    let needed = ephem_arcsec + pointing;
    let half_field = ev.metrics.fov_w_deg.min(ev.metrics.fov_h_deg) * ARCSEC_PER_DEGREE / 2.0;
    let margin = half_field / needed;

    let details = vec![
        kv(
            "Prediction error at target range",
            format!("{:.1} km / {:.0} km = {}", r.ephemeris_uncertainty_km, r.range_km, fmt_angle(ephem_arcsec)),
        ),
        kv(
            "Mount pointing error",
            format!("{}{}", fmt_angle(pointing), if assumed { " (assumed, not entered)" } else { "" }),
        ),
        kv("Half of the field's short side", fmt_angle(half_field)),
        kv("Acquisition margin", format!("{margin:.1}x  (target {:.0}x)", limits::ACQ_PASS_MARGIN)),
    ];
    let (status, verdict) = if margin >= limits::ACQ_PASS_MARGIN {
        (Status::Pass, "The field comfortably covers the expected position error.".to_string())
    } else if margin >= limits::ACQ_WARN_MARGIN {
        (
            Status::Warn,
            "The target should land in the field, but with little margin. Expect occasional misses or a search pattern.".to_string(),
        )
    } else {
        (
            Status::Fail,
            "The expected position error is larger than the field. Plan on searching, or use a wider-field finder.".to_string(),
        )
    };
    RegimeCheck { component: Component::Telescope, title: "Acquisition field", status, details, verdict }
}

/// How long an untracked target stays in the field (informational).
fn telescope_dwell(ev: &Evaluation, r: &Regime) -> RegimeCheck {
    let short_side = ev.metrics.fov_w_deg.min(ev.metrics.fov_h_deg) * ARCSEC_PER_DEGREE;
    let dwell = if r.rate_vs_ground > 0.0 { short_side / r.rate_vs_ground } else { f64::INFINITY };
    let verdict = if r.rate_vs_ground == 0.0 {
        "Earth-fixed target: it stays in a stopped telescope's field indefinitely.".to_string()
    } else {
        format!(
            "Without tracking, the target crosses the short side of the field in {}.",
            fmt_duration(dwell)
        )
    };
    RegimeCheck {
        component: Component::Telescope,
        title: "Field dwell (untracked)",
        status: Status::Info,
        details: vec![kv("Target rate vs ground", fmt_rate(r.rate_vs_ground)), kv("Time to cross the field", fmt_duration(dwell))],
        verdict,
    }
}

/// What limits detection in this regime, with this telescope's depth (informational).
fn telescope_depth(ev: &Evaluation, r: &Regime, reference_area: Option<f64>) -> RegimeCheck {
    let mut details = vec![kv("Effective collecting area", format!("{:.4} m^2", ev.metrics.effective_area_m2))];
    if let Some(ra) = reference_area {
        details.push(kv("Depth vs reference", format!("{:+.2} mag", 2.5 * (ev.metrics.effective_area_m2 / ra).log10())));
    }
    RegimeCheck {
        component: Component::Telescope,
        title: "Depth relevance",
        status: Status::Info,
        details,
        verdict: format!("Detection in this regime is usually limited by {}.", r.limiting_factor),
    }
}

fn fmt_angle(arcsec: f64) -> String {
    if arcsec >= 3600.0 {
        format!("{:.2} deg", arcsec / 3600.0)
    } else if arcsec >= 60.0 {
        format!("{:.1}' ({arcsec:.0}\")", arcsec / 60.0)
    } else if arcsec >= 1.0 {
        format!("{arcsec:.1}\"")
    } else {
        format!("{arcsec:.3}\"")
    }
}

// ---------------------------------------------------------------------------
// Camera checks
// ---------------------------------------------------------------------------

/// Is the timestamp accurate enough for this regime's motion?
fn camera_timing(cfg: &Config, ev: &Evaluation, r: &Regime) -> RegimeCheck {
    let binned = ev.metrics.plate_scale * ev.metrics.recommended_bin as f64;
    let required_s = limits::TIMING_PIXEL_FRACTION * binned / r.rate_vs_stars;
    let actual_s = cfg.timestamp_accuracy_ms / MS_PER_S;
    let error_arcsec = r.rate_vs_stars * actual_s;

    let details = vec![
        kv("Target motion vs stars", fmt_rate(r.rate_vs_stars)),
        kv(
            "Timing needed",
            format!(
                "{}  ({:.2} of a {:.2}\" binned pixel)",
                fmt_duration(required_s),
                limits::TIMING_PIXEL_FRACTION,
                binned
            ),
        ),
        kv("Timestamp accuracy entered", fmt_duration(actual_s)),
        kv("Resulting position error", format!("{} = {:.3} px", fmt_angle(error_arcsec), error_arcsec / binned)),
    ];
    let (status, verdict) = if actual_s <= required_s {
        (Status::Pass, "Timing error is within the budget of a quarter binned pixel.".to_string())
    } else if actual_s <= limits::TIMING_WARN_MULTIPLE * required_s {
        (
            Status::Warn,
            "Timing error is a noticeable fraction of a pixel. Positions are usable but timing limits accuracy.".to_string(),
        )
    } else {
        (
            Status::Fail,
            format!(
                "Timing error dominates the position error. Needs timestamps good to about {}, which usually means GPS hardware timestamping.",
                fmt_duration(required_s)
            ),
        )
    };
    RegimeCheck { component: Component::Camera, title: "Timestamp accuracy", status, details, verdict }
}

/// How much does row-by-row readout distort positions at this regime's rate?
fn camera_shutter(cfg: &Config, ev: &Evaluation, r: &Regime) -> RegimeCheck {
    let binned = ev.metrics.plate_scale * ev.metrics.recommended_bin as f64;
    let c = &cfg.camera;
    match c.shutter {
        Shutter::Global => RegimeCheck {
            component: Component::Camera,
            title: "Shutter skew",
            status: Status::Pass,
            details: vec![kv("Shutter", "global".to_string())],
            verdict: "Every row is exposed at once, so there is no readout skew at any rate.".to_string(),
        },
        Shutter::Rolling { line_time_us: None } => RegimeCheck {
            component: Component::Camera,
            title: "Shutter skew",
            status: Status::Warn,
            details: vec![kv("Shutter", "rolling, line time unknown".to_string())],
            verdict: "Ask the vendor for the line time to size the readout skew (skew = rows x line time).".to_string(),
        },
        Shutter::Rolling { line_time_us: Some(lt) } => {
            let readout_s = c.height_px as f64 * lt * S_PER_US;
            let skew = r.rate_vs_stars * readout_s;
            let skew_px = skew / binned;
            let frame_h = ev.metrics.fov_h_deg * ARCSEC_PER_DEGREE;
            let frac = skew / frame_h;
            let details = vec![
                kv("Readout time, top to bottom", fmt_duration(readout_s)),
                kv(
                    "Target-vs-star skew across the frame",
                    format!("{} = {:.1} px ({:.1}% of frame height)", fmt_angle(skew), skew_px, frac * 100.0),
                ),
            ];
            let (status, verdict) = if skew_px <= limits::SKEW_NEGLIGIBLE_PX {
                (Status::Pass, "Skew is negligible at this rate.".to_string())
            } else if frac <= limits::SKEW_FAIL_FRAME_FRACTION {
                (
                    Status::Warn,
                    "Correctable: the software must assign each row its own timestamp (t = t_first_row + row x line time).".to_string(),
                )
            } else {
                (
                    Status::Fail,
                    "The target moves a large fraction of the frame during readout. Use a global-shutter camera or a small region-of-interest readout.".to_string(),
                )
            };
            RegimeCheck { component: Component::Camera, title: "Shutter skew", status, details, verdict }
        }
    }
}

/// How long can an exposure be before relative motion smears something (informational).
fn camera_trailing(cfg: &Config, ev: &Evaluation, r: &Regime, seeing: f64) -> RegimeCheck {
    let _ = cfg;
    let binned = ev.metrics.plate_scale * ev.metrics.recommended_bin as f64;
    let t_cross = seeing / r.rate_vs_stars;
    let streak_px = r.rate_vs_stars / binned;
    RegimeCheck {
        component: Component::Camera,
        title: "Exposure vs trailing",
        status: Status::Info,
        details: vec![
            kv("Relative motion crosses one seeing FWHM in", fmt_duration(t_cross)),
            kv("Streak length per second of exposure", format!("{streak_px:.0} px (binned)")),
            kv("Usual observing mode", r.mode.describe().to_string()),
        ],
        verdict: format!(
            "Exposures longer than {} smear either the target (if tracking the stars) or the stars (if tracking the target).",
            fmt_duration(t_cross)
        ),
    }
}

// ---------------------------------------------------------------------------
// Mount checks
// ---------------------------------------------------------------------------

/// Can the mount move fast enough, including the alt-az zenith keyhole?
fn mount_rate(cfg: &Config, r: &Regime) -> RegimeCheck {
    let required_deg_s = r.rate_vs_ground / ARCSEC_PER_DEGREE;
    let mount = cfg.payload.mount.as_ref();
    let mut details = vec![kv("Required rate (vs ground)", fmt_rate(r.rate_vs_ground))];

    if r.mode == TrackingMode::Stare {
        details.push(kv("Mode", "stare: tracking off".to_string()));
        return RegimeCheck {
            component: Component::Mount,
            title: "Tracking rate",
            status: Status::Pass,
            details,
            verdict: "The target is Earth-fixed. The mount only needs to point and hold still.".to_string(),
        };
    }

    let Some(m) = mount else {
        return RegimeCheck {
            component: Component::Mount,
            title: "Tracking rate",
            status: Status::Info,
            details,
            verdict: "No mount selected.".to_string(),
        };
    };
    let Some(max) = m.max_slew_deg_s else {
        let status = if required_deg_s > limits::RATE_MATTERS_DEG_S { Status::Warn } else { Status::Info };
        return RegimeCheck {
            component: Component::Mount,
            title: "Tracking rate",
            status,
            details,
            verdict: format!("Maximum axis rate for {} is unknown. Ask the vendor.", m.name),
        };
    };

    let headroom = max / required_deg_s;
    details.push(kv("Mount maximum axis rate", format!("{max:.1} deg/s  (headroom {headroom:.0}x)")));
    let mut status = if headroom >= limits::RATE_PASS_HEADROOM {
        Status::Pass
    } else if headroom >= limits::RATE_WARN_HEADROOM {
        Status::Warn
    } else {
        Status::Fail
    };
    let mut verdict = match status {
        Status::Pass => "Ample rate headroom.".to_string(),
        Status::Warn => "The mount can keep up, but with little margin for acceleration and corrections.".to_string(),
        _ => "The mount cannot move as fast as the target.".to_string(),
    };

    match m.mount_type {
        MountType::AltAz => {
            // Near the zenith the azimuth rate is about omega / (zenith distance in radians).
            let omega_rad = required_deg_s.to_radians();
            let max_rad = max.to_radians();
            let z_min_deg = (omega_rad / max_rad).to_degrees();
            let elev = 90.0 - z_min_deg;
            details.push(kv("Highest pass followable (alt-az keyhole)", format!("{elev:.1} deg elevation")));
            let k = if elev >= limits::KEYHOLE_PASS_ELEV_DEG {
                Status::Pass
            } else if elev >= limits::KEYHOLE_WARN_ELEV_DEG {
                Status::Warn
            } else {
                Status::Fail
            };
            if k > status {
                status = k;
            }
            verdict.push_str(&format!(
                " Passes peaking above {elev:.1} deg would outrun the azimuth axis near the zenith."
            ));
        }
        MountType::Equatorial => details.push(kv(
            "Keyhole",
            "near the celestial pole (affects polar-orbit passes)".to_string(),
        )),
        MountType::Unknown => details.push(kv("Keyhole", "mount type unknown".to_string())),
    }
    RegimeCheck { component: Component::Mount, title: "Tracking rate", status, details, verdict }
}

/// Can the control software follow a predicted path?
fn mount_non_sidereal(cfg: &Config, r: &Regime) -> RegimeCheck {
    let cap = cfg.payload.mount.as_ref().map(|m| m.non_sidereal_tracking);
    let answer = match cap {
        Some(Capability::Yes) => "yes",
        Some(Capability::No) => "no",
        Some(Capability::Unknown) => "unknown",
        None => "no mount selected",
    };
    let details = vec![
        kv("Non-sidereal / TLE tracking", answer.to_string()),
        kv("Required in this regime", if r.needs_non_sidereal { "yes" } else { "no" }.to_string()),
    ];
    let (status, verdict) = match (r.needs_non_sidereal, cap) {
        (false, _) => (
            Status::Info,
            match r.mode {
                TrackingMode::Stare => "Not needed: stare mode uses tracking off.".to_string(),
                _ => "Not strictly needed, but it allows longer exposures on slowly moving targets.".to_string(),
            },
        ),
        (true, Some(Capability::Yes)) => (Status::Pass, "The mount can follow the target's predicted path.".to_string()),
        (true, Some(Capability::No)) => (
            Status::Fail,
            "This regime needs the mount to follow a predicted path, which this mount's software cannot do.".to_string(),
        ),
        (true, _) => (
            Status::Warn,
            "Confirm with the vendor that the control software can track from a TLE or ephemeris at arbitrary rates.".to_string(),
        ),
    };
    RegimeCheck { component: Component::Mount, title: "Non-sidereal tracking", status, details, verdict }
}

/// Evaluate one configuration against every regime.
pub fn evaluate_regimes(
    cfg: &Config,
    ev: &Evaluation,
    site: &Site,
    reference_area: Option<f64>,
) -> Vec<RegimeEvaluation> {
    regimes()
        .into_iter()
        .map(|r| {
            let checks = vec![
                telescope_acquisition(cfg, ev, &r),
                telescope_dwell(ev, &r),
                telescope_depth(ev, &r, reference_area),
                camera_timing(cfg, ev, &r),
                camera_shutter(cfg, ev, &r),
                camera_trailing(cfg, ev, &r, site.seeing_arcsec),
                mount_rate(cfg, &r),
                mount_non_sidereal(cfg, &r),
            ];
            RegimeEvaluation { regime: r, checks }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn leo_is_the_only_window_constrained_regime() {
        // LEO passes are over in minutes; everything else is available for
        // hours, so only LEO grades the slew-and-settle check.
        for r in regimes() {
            match r.key {
                "LEO" => assert_eq!(r.usable_window_s, Some(300.0)),
                _ => assert_eq!(r.usable_window_s, None, "{} should be unconstrained", r.key),
            }
        }
    }

    #[test]
    fn system_is_a_component() {
        assert_eq!(Component::System.name(), "System");
    }

    #[test]
    fn leo_overhead_rate() {
        // 500 km: v = 7.61 km/s, omega = v / h = 0.0152 rad/s = ~3,140"/s = 0.87 deg/s.
        assert!(close(circular_speed_km_s(500.0), 7.613, 0.002));
        assert!(close(overhead_rate_arcsec_s(500.0), 3140.0, 2.0));
    }

    #[test]
    fn meo_overhead_rate() {
        assert!(close(overhead_rate_arcsec_s(20_200.0), 39.6, 0.2));
    }

    #[test]
    fn geo_and_lunar_rates_from_period() {
        assert!(close(rate_from_period_arcsec_s(86_164.0905), 15.041, 0.001));
        assert!(close(rate_from_period_arcsec_s(27.321_661 * 86_400.0), 0.549, 0.001));
    }

    #[test]
    fn molniya_apogee_rate() {
        let a = 26_560.0;
        let r = a * 1.74;
        let v = vis_viva_km_s(r, a);
        assert!(close(v, 1.497, 0.005));
        assert!(close(v / (r - EARTH_RADIUS_KM) * ARCSEC_PER_RADIAN, 7.75, 0.05));
    }

    #[test]
    fn keyhole_l350_leo() {
        // 0.872 deg/s target, 50 deg/s mount: zenith distance 0.872/50 rad = 1.0 deg -> 89.0 deg elevation.
        let omega = 3140.5_f64 / 3600.0;
        let z = (omega.to_radians() / 50f64.to_radians()).to_degrees();
        assert!(close(90.0 - z, 89.0, 0.01));
    }

    #[test]
    fn geo_timing_requirement() {
        // 0.25 x 1.477"/px / 15.04"/s = 24.6 ms
        let req = limits::TIMING_PIXEL_FRACTION * 1.477 / SIDEREAL_RATE_ARCSEC_PER_S;
        assert!(close(req * 1000.0, 24.6, 0.1));
    }
}
