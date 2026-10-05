//! Component evaluations (telescope, camera, mount) against orbital regimes.
//!
//! Each regime is described by a few representative numbers: how far away the
//! target is, how fast it moves against the stars and against the ground, how
//! uncertain its predicted position typically is, and how it is usually
//! observed. The checks then ask, component by component, whether a
//! configuration can acquire, time-tag and follow that kind of target.
//!
//! All regime numbers are representative worst-to-typical cases, documented
//! in docs/08-orbital-regimes.md. Change them in `regimes()` to match your own catalog.

use crate::checks::{kv, Evaluation, Status};
use crate::calculations::camera::CameraTimingCalculator;
use crate::calculations::detection::DetectionCalculator;
use crate::calculations::mount::MountDynamicsCalculator;
use crate::calculations::optics::OpticsCalculator;
use crate::calculations::orbit::OrbitCalculator;
use crate::calculations::psf::PsfCalculator;
use crate::constants::{
    regimes_limits as limits, ARCSEC_PER_DEGREE, ARCSEC_PER_RADIAN, DEFAULT_POINTING_RMS_ARCSEC,
    DEFAULT_PHASE_FACTOR, DEFAULT_SETTLE_TIME_S, DEFAULT_SLEW_DISTANCE_DEG, DEG_PER_RADIAN,
    EARTH_RADIUS_KM, MAS_PER_ARCSEC, REFERENCE_TARGET_ALBEDO, REFERENCE_TARGET_CROSS_SECTION_M2,
    SIDEREAL_RATE_ARCSEC_PER_S,
};
use crate::constants::plausible_ranges as ranges;
use crate::photometry::Photometry;
use crate::model::{plausible, Capability, Config, MountType, Shutter, Site};

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

/// The representative regimes. Edit here to match your own targets.
pub fn regimes() -> Vec<Regime> {
    // LEO: 500 km circular orbit passing overhead (the fastest geometry).
    let leo_alt = 500.0;
    let leo_rate = OrbitCalculator::overhead_rate_arcsec_s(leo_alt);

    // MEO: GPS-like 20,200 km circular orbit passing overhead.
    let meo_alt = 20_200.0;
    let meo_rate = OrbitCalculator::overhead_rate_arcsec_s(meo_alt);

    // HEO: Molniya orbit (a = 26,560 km, e = 0.74) observed near apogee.
    let molniya_a = 26_560.0;
    let molniya_apogee_r = molniya_a * (1.0 + 0.74);
    let molniya_apogee_alt = molniya_apogee_r - EARTH_RADIUS_KM;
    let heo_rate =
        OrbitCalculator::vis_viva_km_s(molniya_apogee_r, molniya_a) / molniya_apogee_alt * ARCSEC_PER_RADIAN;

    // Cislunar: lunar distance, moving with roughly the Moon's motion against the stars.
    let lunar_rate = OrbitCalculator::rate_from_period_arcsec_s(27.321_661 * 86_400.0);

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
    let ephem_arcsec =
        OrbitCalculator::position_uncertainty_arcsec(r.ephemeris_uncertainty_km, r.range_km);
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
    let dwell = if r.rate_vs_ground > 0.0 {
        CameraTimingCalculator::crossing_time_s(short_side, r.rate_vs_ground)
    } else {
        f64::INFINITY
    };
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
        details.push(kv(
            "Depth vs reference",
            format!("{:+.2} mag", OpticsCalculator::delta_mag(ev.metrics.effective_area_m2, ra)),
        ));
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
    let binned = CameraTimingCalculator::binned_plate_scale_arcsec_per_px(
        ev.metrics.plate_scale,
        ev.metrics.recommended_bin,
    );
    let required_s =
        CameraTimingCalculator::timing_budget_s(binned, r.rate_vs_stars, limits::TIMING_PIXEL_FRACTION);
    let actual_s = CameraTimingCalculator::milliseconds_to_seconds(cfg.timestamp_accuracy_ms);
    let error_arcsec = CameraTimingCalculator::position_error_arcsec(r.rate_vs_stars, actual_s);

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
        kv(
            "Resulting position error",
            format!(
                "{} = {:.3} px",
                fmt_angle(error_arcsec),
                CameraTimingCalculator::pixels_for_angle(error_arcsec, binned)
            ),
        ),
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
    let binned = CameraTimingCalculator::binned_plate_scale_arcsec_per_px(
        ev.metrics.plate_scale,
        ev.metrics.recommended_bin,
    );
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
            let readout_s = CameraTimingCalculator::rolling_readout_time_s(c.height_px, lt);
            let skew = CameraTimingCalculator::rolling_shutter_skew_arcsec(r.rate_vs_stars, readout_s);
            let skew_px = CameraTimingCalculator::pixels_for_angle(skew, binned);
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
fn camera_trailing(ev: &Evaluation, r: &Regime) -> RegimeCheck {
    let star = ev.metrics.star_fwhm_arcsec;
    let binned = CameraTimingCalculator::binned_plate_scale_arcsec_per_px(
        ev.metrics.plate_scale,
        ev.metrics.recommended_bin,
    );
    let t_cross = CameraTimingCalculator::crossing_time_s(star, r.rate_vs_stars);
    let streak_px = CameraTimingCalculator::pixels_per_second(r.rate_vs_stars, binned);
    RegimeCheck {
        component: Component::Camera,
        title: "Exposure vs trailing",
        status: Status::Info,
        details: vec![
            kv("Relative motion crosses one star FWHM in", fmt_duration(t_cross)),
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
    let Some(max) = plausible(m.max_slew_deg_s, ranges::SLEW_RATE_MIN_DEG_S, ranges::SLEW_RATE_MAX_DEG_S)
    else {
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
            // Near the zenith the azimuth axis must sweep through the same
            // atan form as the pass itself, so both the rate and the
            // acceleration limits constrain the same keyhole. Report whichever
            // binds; with acceleration unknown that is the rate limit, which
            // is what this check reported before acceleration was modelled.
            let omega_rad = required_deg_s.to_radians();
            let accel_rad = plausible(
                m.max_accel_deg_s2,
                ranges::ACCEL_MIN_DEG_S2,
                ranges::ACCEL_MAX_DEG_S2,
            )
            .map(f64::to_radians);
            let (z_min_rad, binding) = MountDynamicsCalculator::keyhole_rad(omega_rad, max.to_radians(), accel_rad);
            let elev = 90.0 - z_min_rad.to_degrees();
            details.push(kv("Highest pass followable (alt-az keyhole)", format!("{elev:.1} deg elevation")));
            details.push(kv("Keyhole set by", format!("{binding} limit")));
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

/// Can the mount accelerate fast enough to follow the pass?
fn mount_acceleration(cfg: &Config, r: &Regime) -> RegimeCheck {
    let title = "Acceleration";
    let omega_rad_s = r.rate_vs_ground / ARCSEC_PER_RADIAN;
    let required = MountDynamicsCalculator::peak_tracking_accel_rad_s2(omega_rad_s) * DEG_PER_RADIAN;
    let mut details = vec![kv("Required peak acceleration", format!("{required:.5} deg/s^2"))];

    if r.mode == TrackingMode::Stare {
        details.push(kv("Mode", "stare: tracking off".to_string()));
        return RegimeCheck {
            component: Component::Mount,
            title,
            status: Status::Pass,
            details,
            verdict: "The target is Earth-fixed. The mount never has to accelerate to follow it."
                .to_string(),
        };
    }

    let Some(m) = cfg.payload.mount.as_ref() else {
        return RegimeCheck {
            component: Component::Mount,
            title,
            status: Status::Info,
            details,
            verdict: "No mount selected.".to_string(),
        };
    };

    let Some(max) = plausible(m.max_accel_deg_s2, ranges::ACCEL_MIN_DEG_S2, ranges::ACCEL_MAX_DEG_S2)
    else {
        let status = if required > limits::ACCEL_MATTERS_DEG_S2 { Status::Warn } else { Status::Info };
        return RegimeCheck {
            component: Component::Mount,
            title,
            status,
            details,
            verdict: format!("Maximum axis acceleration for {} is unknown. Ask the vendor.", m.name),
        };
    };

    let headroom = max / required;
    details.push(kv("Mount maximum acceleration", format!("{max:.3} deg/s^2  (headroom {headroom:.0}x)")));
    let status = if headroom >= limits::ACCEL_PASS_HEADROOM {
        Status::Pass
    } else if headroom >= limits::ACCEL_WARN_HEADROOM {
        Status::Warn
    } else {
        Status::Fail
    };
    let verdict = match status {
        Status::Pass => "Ample acceleration headroom.".to_string(),
        Status::Warn => "The mount can just accelerate fast enough, with little margin for corrections.".to_string(),
        _ => "The mount cannot accelerate fast enough to follow the pass.".to_string(),
    };
    RegimeCheck { component: Component::Mount, title, status, details, verdict }
}

/// Can the mount get on target in time to use the pass?
fn mount_slew_settle(cfg: &Config, r: &Regime) -> RegimeCheck {
    let title = "Slew and settle";
    let Some(window) = r.usable_window_s else {
        return RegimeCheck {
            component: Component::Mount,
            title,
            status: Status::Info,
            details: vec![kv("Usable window", "effectively unlimited".to_string())],
            verdict: "The target stays available long enough that slew time does not compete with it."
                .to_string(),
        };
    };
    let mut details = vec![
        kv("Usable window", format!("{window:.0} s")),
        kv("Assumed slew distance", format!("{:.0} deg", DEFAULT_SLEW_DISTANCE_DEG)),
    ];
    let Some(m) = cfg.payload.mount.as_ref() else {
        return RegimeCheck {
            component: Component::Mount,
            title,
            status: Status::Info,
            details,
            verdict: "No mount selected.".to_string(),
        };
    };
    let rate = plausible(m.max_slew_deg_s, ranges::SLEW_RATE_MIN_DEG_S, ranges::SLEW_RATE_MAX_DEG_S);
    let accel = plausible(m.max_accel_deg_s2, ranges::ACCEL_MIN_DEG_S2, ranges::ACCEL_MAX_DEG_S2);

    match (rate, accel) {
        (Some(v), Some(a)) => {
            let entered = plausible(m.settle_time_s, ranges::SETTLE_MIN_S, ranges::SETTLE_MAX_S);
            let settle = entered.unwrap_or(DEFAULT_SETTLE_TIME_S);
            details.push(kv(
                "Settle time",
                match entered {
                    Some(_) => format!("{settle:.1} s"),
                    None => format!("{settle:.1} s (assumed)"),
                },
            ));
            let slew = MountDynamicsCalculator::slew_time_s(DEFAULT_SLEW_DISTANCE_DEG, v, a);
            let total = slew + settle;
            let fraction = total / window;
            details.push(kv("Slew time", format!("{slew:.1} s")));
            details.push(kv(
                "Slew + settle",
                format!("{total:.1} s  ({:.0}% of the window)", fraction * 100.0),
            ));
            let status = if fraction <= limits::SLEW_PASS_WINDOW_FRACTION {
                Status::Pass
            } else if fraction <= limits::SLEW_WARN_WINDOW_FRACTION {
                Status::Warn
            } else {
                Status::Fail
            };
            let verdict = match status {
                Status::Pass => "The mount is on target well inside the window.".to_string(),
                Status::Warn => "Getting on target eats a significant part of the window.".to_string(),
                _ => "The mount cannot get on target in time to make use of the pass.".to_string(),
            };
            RegimeCheck { component: Component::Mount, title, status, details, verdict }
        }
        (Some(v), None) => {
            let floor = DEFAULT_SLEW_DISTANCE_DEG / v;
            details.push(kv("Slew time", format!("at least {floor:.1} s, ignoring ramp-up")));
            RegimeCheck {
                component: Component::Mount,
                title,
                status: Status::Info,
                details,
                verdict: format!(
                    "Axis acceleration for {} is unknown, so this is a lower bound only. Ask the vendor.",
                    m.name
                ),
            }
        }
        _ => RegimeCheck {
            component: Component::Mount,
            title,
            status: Status::Info,
            details,
            verdict: format!("Maximum slew rate for {} is unknown. Ask the vendor.", m.name),
        },
    }
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

// ---------------------------------------------------------------------------
// System checks
// ---------------------------------------------------------------------------

/// How fast the target moves across the sensor, arcsec/s, given how it is tracked.
///
/// A rate-tracked target is held still by the mount, and an Earth-fixed target
/// in stare mode is still by definition; the stars are what trail in both
/// cases. Under sidereal tracking it is the other way round: the target drifts
/// against the tracked stars at its rate against them.
fn residual_rate_arcsec_s(r: &Regime) -> f64 {
    match r.mode {
        TrackingMode::RateTrack | TrackingMode::Stare => 0.0,
        TrackingMode::Sidereal => r.rate_vs_stars,
    }
}

/// Is the target bright enough for this configuration to detect?
fn system_detection(cfg: &Config, ev: &Evaluation, r: &Regime, site: &Site) -> RegimeCheck {
    let title = "Detection";
    let p = Photometry::resolve(&cfg.telescope, &cfg.camera, site);
    let (target_mag, mag_from) = match cfg.target_mag_override {
        Some(m) => (m, "entered"),
        None => (
            DetectionCalculator::derived_target_mag(
                REFERENCE_TARGET_CROSS_SECTION_M2,
                REFERENCE_TARGET_ALBEDO,
                r.range_km,
                DEFAULT_PHASE_FACTOR,
            ),
            "derived",
        ),
    };
    let residual = residual_rate_arcsec_s(r);
    // The recorded star (system PSF at the sensor centre), not the seeing alone.
    let star = ev.metrics.star_fwhm_arcsec;
    let (exposure, exp_from) = match cfg.exposure_override_s {
        Some(t) => (t, "entered"),
        None => (DetectionCalculator::trail_limited_exposure_s(star, residual), "derived"),
    };

    let scale = ev.metrics.plate_scale;
    let area = ev.metrics.effective_area_m2;
    let trail = DetectionCalculator::trail_arcsec(residual, exposure);
    let n_px = DetectionCalculator::footprint_px(star, trail, scale);
    let signal = DetectionCalculator::signal_e_per_s(target_mag, area, p.qe, p.throughput) * exposure;
    let sky = DetectionCalculator::sky_e_per_px_s(p.sky_mag_arcsec2, scale, area, p.qe, p.throughput)
        * exposure
        * n_px;
    let snr = DetectionCalculator::snr(signal, sky, p.read_noise_e, n_px);
    let noise_variance = DetectionCalculator::noise_variance_e2(0.0, sky, p.read_noise_e, n_px);
    let m_limit = DetectionCalculator::limiting_mag(
        limits::DETECT_SNR_THRESHOLD,
        noise_variance,
        DetectionCalculator::signal_coefficient(area, p.qe, p.throughput, exposure),
    );

    let mut details = vec![
        kv("Target magnitude", format!("{target_mag:.2} ({mag_from})")),
        kv("Exposure", format!("{exposure:.3} s ({exp_from})")),
        kv("Trail", format!("{trail:.2}\" ({:.1} px)", trail / scale)),
        kv("Footprint", format!("{n_px:.1} px")),
        kv("Signal / sky", format!("{signal:.0} e- / {sky:.0} e-")),
        kv("SNR", format!("{snr:.1}")),
        kv(
            "Centroid precision (photon-limited)",
            if snr > 0.0 {
                let sigma = PsfCalculator::centroid_sigma(star, snr);
                format!("{:.2} mas ({:.4} px) per axis, 1 sigma", sigma * MAS_PER_ARCSEC, sigma / scale)
            } else {
                "no signal".to_string()
            },
        ),
        kv("Limiting magnitude", format!("{m_limit:.2}")),
        kv("Margin", format!("{:+.2} mag", m_limit - target_mag)),
    ];

    let trivial = snr >= limits::SNR_TRIVIAL;
    let mut status = if snr >= limits::SNR_PASS {
        Status::Pass
    } else if snr >= limits::DETECT_SNR_THRESHOLD {
        Status::Warn
    } else {
        Status::Fail
    };
    let mut verdict = if trivial {
        format!(
            "Detection is not the limiting factor here, so choose exposure for saturation and timing instead. What limits this regime is {}.",
            r.limiting_factor
        )
    } else if status == Status::Pass {
        format!("Detectable with margin: SNR {snr:.0} against a threshold of {:.0}.", limits::DETECT_SNR_THRESHOLD)
    } else if status == Status::Warn {
        "Marginal: detectable, but close enough to the threshold that conditions will decide it.".to_string()
    } else {
        "Too faint to detect in this configuration.".to_string()
    };

    // None of the above means anything if the target left the sensor.
    let short_side_arcsec = ev.metrics.fov_w_deg.min(ev.metrics.fov_h_deg) * ARCSEC_PER_DEGREE;
    if trail > short_side_arcsec {
        details.push(kv("Trail vs field", "longer than the short side of the field".to_string()));
        verdict.push_str(" The trail is longer than the field, so the target streaks off the sensor during the exposure: shorten it.");
        if status == Status::Pass {
            status = Status::Warn;
        }
    }

    // Never claim a PASS on numbers the user did not supply. Checked against
    // Pass explicitly: Status is ordered Info < Pass < Warn < Fail, so a
    // `.max(Warn)` here would also promote an Info.
    if p.any_assumed() {
        details.push(kv("Assumed inputs", p.assumed.join(", ")));
        if status == Status::Pass {
            status = Status::Warn;
            verdict.push_str(&format!(
                " Not graded PASS because these were assumed rather than entered: {}.",
                p.assumed.join(", ")
            ));
        }
    }

    RegimeCheck { component: Component::System, title, status, details, verdict }
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
                camera_trailing(ev, &r),
                mount_rate(cfg, &r),
                mount_acceleration(cfg, &r),
                mount_slew_settle(cfg, &r),
                mount_non_sidereal(cfg, &r),
                system_detection(cfg, ev, &r, site),
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

    // Site arrives via `use super::*`; Mount and Payload are not imported at
    // module level and the fixtures need both (pre-flight ruling).
    use crate::model::{Mount, Payload};

    /// DeltaRho 350 + IMX455 on an L-350, the configuration every worked
    /// example in docs/ uses.
    fn fixture(mount: Option<Mount>) -> (Config, Site) {
        let telescope = crate::presets::telescopes()
            .into_iter()
            .find(|t| t.name.contains("DeltaRho 350"))
            .expect("DeltaRho 350 preset");
        let camera = crate::presets::cameras()
            .into_iter()
            .find(|c| c.name.contains("IMX455"))
            .expect("IMX455 preset");
        let cfg = Config {
            label: "fixture".into(),
            telescope,
            camera,
            payload: Payload { mount, accessories_lb: 10.0, back_focus_required_mm: None },
            timestamp_accuracy_ms: 0.1,
            target_mag_override: None,
            exposure_override_s: None,
        };
        let site = Site { seeing_arcsec: 2.5, wavelength_um: 0.55, sky_mag_arcsec2: None, location: None };
        (cfg, site)
    }

    fn l350(max_accel_deg_s2: Option<f64>) -> Mount {
        Mount {
            name: "L-350".into(),
            mount_type: MountType::AltAz,
            capacity_lb: Some(100.0),
            max_slew_deg_s: Some(50.0),
            max_accel_deg_s2,
            settle_time_s: None,
            pointing_rms_arcsec: Some(30.0),
            non_sidereal_tracking: Capability::Yes,
            source: "test".into(),
        }
    }

    /// The named check for one regime of one configuration.
    fn check_for(key: &str, title: &str, cfg: &Config, site: &Site) -> RegimeCheck {
        let ev = crate::checks::evaluate(cfg, site, None);
        ev.regimes
            .iter()
            .find(|r| r.regime.key == key)
            .unwrap_or_else(|| panic!("no regime {key}"))
            .checks
            .iter()
            .find(|c| c.title == title)
            .unwrap_or_else(|| panic!("no check {title} in regime {key}"))
            .clone()
    }

    #[test]
    fn acceleration_passes_with_a_known_rating() {
        // LEO needs 0.0086 deg/s^2; 10 deg/s^2 is over a thousand times that.
        let (cfg, site) = fixture(Some(l350(Some(10.0))));
        let c = check_for("LEO", "Acceleration", &cfg, &site);
        assert_eq!(c.status, Status::Pass);
    }

    #[test]
    fn acceleration_fails_a_mount_that_cannot_keep_up() {
        let (cfg, site) = fixture(Some(l350(Some(0.004))));
        let c = check_for("LEO", "Acceleration", &cfg, &site);
        assert_eq!(c.status, Status::Fail);
    }

    #[test]
    fn acceleration_warns_when_the_rating_is_unknown_and_leo_needs_it() {
        // 0.008627 deg/s^2 is above ACCEL_MATTERS_DEG_S2, so an unknown
        // rating is a question for the vendor rather than a non-issue.
        let (cfg, site) = fixture(Some(l350(None)));
        let c = check_for("LEO", "Acceleration", &cfg, &site);
        assert_eq!(c.status, Status::Warn);
        assert!(c.verdict.contains("unknown"));
    }

    #[test]
    fn acceleration_is_info_when_the_rating_is_unknown_and_irrelevant() {
        // MEO needs 1.4e-6 deg/s^2. Nobody needs to ask the vendor about that.
        let (cfg, site) = fixture(Some(l350(None)));
        let c = check_for("MEO", "Acceleration", &cfg, &site);
        assert_eq!(c.status, Status::Info);
    }

    #[test]
    fn acceleration_passes_in_stare_mode() {
        let (cfg, site) = fixture(Some(l350(None)));
        let c = check_for("GEO", "Acceleration", &cfg, &site);
        assert_eq!(c.status, Status::Pass);
    }

    #[test]
    fn acceleration_treats_a_nonsense_rating_as_unknown() {
        // Review Focus 2: a zero, negative or NaN rating in presets.yaml must
        // not become an unfollowable mount.
        for bad in [0.0, -5.0, f64::NAN, f64::INFINITY] {
            let (cfg, site) = fixture(Some(l350(Some(bad))));
            let c = check_for("LEO", "Acceleration", &cfg, &site);
            assert_eq!(c.status, Status::Warn, "rating {bad} was trusted");
            assert!(c.verdict.contains("unknown"), "rating {bad} was trusted");
        }
    }

    #[test]
    fn slew_and_settle_passes_a_direct_drive_mount_on_leo() {
        // 90 deg at 50 deg/s and 10 deg/s^2 is 6.0 s, plus 2 s of assumed
        // settle: 8 s of a 300 s window, under 3%.
        let (cfg, site) = fixture(Some(l350(Some(10.0))));
        let c = check_for("LEO", "Slew and settle", &cfg, &site);
        assert_eq!(c.status, Status::Pass);
    }

    #[test]
    fn slew_and_settle_fails_a_mount_that_cannot_get_there_in_time() {
        let mut m = l350(Some(0.05));
        m.max_slew_deg_s = Some(1.0);
        let (cfg, site) = fixture(Some(m));
        let c = check_for("LEO", "Slew and settle", &cfg, &site);
        assert_eq!(c.status, Status::Fail);
    }

    #[test]
    fn slew_and_settle_is_info_where_the_window_is_unlimited() {
        for key in ["MEO", "GEO", "HEO", "CIS"] {
            let (cfg, site) = fixture(Some(l350(Some(10.0))));
            let c = check_for(key, "Slew and settle", &cfg, &site);
            assert_eq!(c.status, Status::Info, "{key} should not be window-constrained");
        }
    }

    #[test]
    fn slew_and_settle_reports_a_lower_bound_when_acceleration_is_unknown() {
        let (cfg, site) = fixture(Some(l350(None)));
        let c = check_for("LEO", "Slew and settle", &cfg, &site);
        assert_eq!(c.status, Status::Info);
        assert!(c.verdict.contains("lower bound"));
    }

    #[test]
    fn slew_and_settle_names_the_settle_time_as_assumed() {
        let (cfg, site) = fixture(Some(l350(Some(10.0))));
        let c = check_for("LEO", "Slew and settle", &cfg, &site);
        assert!(c.details.iter().any(|d| d.contains("assumed")));
    }

    #[test]
    fn slew_and_settle_uses_an_entered_settle_time() {
        let mut m = l350(Some(10.0));
        m.settle_time_s = Some(45.0);
        let (cfg, site) = fixture(Some(m));
        let c = check_for("LEO", "Slew and settle", &cfg, &site);
        // 6 s of slew plus 45 s of settle is 17% of a 300 s window: a WARN.
        assert_eq!(c.status, Status::Warn);
    }

    #[test]
    fn slew_and_settle_is_info_with_no_mount() {
        let (cfg, site) = fixture(None);
        let c = check_for("LEO", "Slew and settle", &cfg, &site);
        assert_eq!(c.status, Status::Info);
    }

    /// The keyhole elevation a check reported, parsed back out of its details.
    fn keyhole_elev(cfg: &Config, site: &Site) -> f64 {
        let c = check_for("LEO", "Tracking rate", cfg, site);
        let line = c
            .details
            .iter()
            .find(|d| d.contains("keyhole"))
            .expect("keyhole detail");
        line.split_whitespace()
            .find_map(|w| w.parse::<f64>().ok())
            .expect("a number in the keyhole detail")
    }

    #[test]
    fn keyhole_unchanged_when_acceleration_is_unknown() {
        // The guarantee: 0.87234 deg/s against a 50 deg/s axis gives a 1.0 deg
        // keyhole, so 89.0 deg of elevation -- exactly what this tool reported
        // before acceleration was modelled.
        let (cfg, site) = fixture(Some(l350(None)));
        assert!(close(keyhole_elev(&cfg, &site), 89.0, 0.05));
        let c = check_for("LEO", "Tracking rate", &cfg, &site);
        assert!(c.details.iter().any(|d| d.contains("rate limit")));
    }

    #[test]
    fn keyhole_unchanged_when_acceleration_is_nonsense() {
        for bad in [0.0, -5.0, f64::NAN] {
            let (cfg, site) = fixture(Some(l350(Some(bad))));
            assert!(close(keyhole_elev(&cfg, &site), 89.0, 0.05), "rating {bad} moved the keyhole");
        }
    }

    #[test]
    fn tracking_rate_treats_a_nonsense_slew_rate_as_unknown() {
        // A 0.0 or NaN max_slew_deg_s in presets.yaml must not become a
        // confident FAIL with "-inf deg elevation" in it. mount_slew_settle
        // already filters this field; mount_rate must too.
        for bad in [0.0, -5.0, f64::NAN, f64::INFINITY] {
            let mut m = l350(Some(10.0));
            m.max_slew_deg_s = Some(bad);
            let (cfg, site) = fixture(Some(m));
            let c = check_for("LEO", "Tracking rate", &cfg, &site);
            assert!(c.verdict.contains("unknown"), "slew rate {bad} was trusted: {}", c.verdict);
            assert!(
                !c.details.iter().any(|d| d.contains("inf") || d.contains("NaN")),
                "slew rate {bad} leaked a non-finite figure into the report: {:?}",
                c.details
            );
        }
    }

    #[test]
    fn keyhole_tightens_when_acceleration_is_known() {
        let (cfg, site) = fixture(Some(l350(Some(10.0))));
        assert!(close(keyhole_elev(&cfg, &site), 88.32, 0.05));
        let c = check_for("LEO", "Tracking rate", &cfg, &site);
        assert!(c.details.iter().any(|d| d.contains("acceleration limit")));
    }

    #[test]
    fn a_low_acceleration_rating_warns_on_the_keyhole() {
        // 0.5 deg/s^2 pushes the keyhole to 82.5 deg, below
        // KEYHOLE_WARN_ELEV_DEG, so the check can no longer pass.
        let (cfg, site) = fixture(Some(l350(Some(0.5))));
        assert!(close(keyhole_elev(&cfg, &site), 82.48, 0.05));
        let c = check_for("LEO", "Tracking rate", &cfg, &site);
        assert!(c.status >= Status::Warn);
    }

    fn detection(key: &str, cfg: &Config, site: &Site) -> RegimeCheck {
        check_for(key, "Detection", cfg, site)
    }

    /// A fixture with every photometric input entered, so nothing is capped.
    fn fully_specified() -> (Config, Site) {
        let (mut cfg, mut site) = fixture(Some(l350(Some(10.0))));
        cfg.telescope.throughput = Some(0.85);
        cfg.camera.qe = Some(0.80);
        cfg.camera.read_noise_e = Some(3.0);
        site.sky_mag_arcsec2 = Some(21.0);
        (cfg, site)
    }

    /// A numeric value out of a named detail line.
    fn detail_number(c: &RegimeCheck, label: &str) -> f64 {
        let line = c
            .details
            .iter()
            .find(|d| d.contains(label))
            .unwrap_or_else(|| panic!("no detail {label}"));
        line.split_whitespace()
            .find_map(|w| w.parse::<f64>().ok())
            .unwrap_or_else(|| panic!("no number in detail {label}"))
    }

    #[test]
    fn residual_rate_follows_the_tracking_mode() {
        for r in regimes() {
            let residual = residual_rate_arcsec_s(&r);
            match r.mode {
                // The mount holds a rate-tracked or stared target still.
                TrackingMode::RateTrack | TrackingMode::Stare => {
                    assert!(close(residual, 0.0, 1e-12), "{} should not trail", r.key)
                }
                // Under sidereal tracking the target drifts against the stars.
                TrackingMode::Sidereal => assert!(close(residual, r.rate_vs_stars, 1e-12)),
            }
        }
    }

    #[test]
    fn detection_caps_at_warn_when_inputs_are_assumed() {
        // Every preset carries null QE, throughput and read noise, and the
        // fixture's site carries no sky brightness, so nothing here is
        // entered. GEO would otherwise grade PASS on SNR 526.
        let (cfg, site) = fixture(Some(l350(Some(10.0))));
        let c = detection("GEO", &cfg, &site);
        assert_eq!(c.status, Status::Warn);
        assert!(c.details.iter().any(|d| d.contains("quantum efficiency")));
        assert!(c.verdict.contains("assumed"));
    }

    #[test]
    fn detection_passes_when_every_input_is_entered() {
        let (cfg, site) = fully_specified();
        let c = detection("GEO", &cfg, &site);
        assert_eq!(c.status, Status::Pass);
        assert!(!c.verdict.contains("assumed"));
    }

    #[test]
    fn detection_fail_is_not_changed_by_cap() {
        // The cap must be an `== Pass` test, not `.max(Warn)`: a FAIL stays a
        // FAIL whether or not inputs were assumed. Replaces the spec's
        // Info-based test, since the detection check has no Info path.
        let (mut cfg, site) = fixture(Some(l350(Some(10.0))));
        cfg.target_mag_override = Some(30.0); // far beyond any limit
        let c = detection("GEO", &cfg, &site);
        assert_eq!(c.status, Status::Fail);
    }

    #[test]
    fn cislunar_is_the_only_regime_detection_actually_grades() {
        // Every regime nearer than the Moon exceeds SNR_TRIVIAL on a 14-inch
        // at 30 s, which is the point: brightness is not what limits them.
        let (cfg, site) = fully_specified();
        for key in ["LEO", "MEO", "GEO", "HEO"] {
            let c = detection(key, &cfg, &site);
            assert!(
                c.verdict.contains("not the limiting factor"),
                "{key} should be trivially detectable"
            );
        }
        let cis = detection("CIS", &cfg, &site);
        assert!(!cis.verdict.contains("not the limiting factor"));
        assert_eq!(cis.status, Status::Pass);
    }

    #[test]
    fn detection_footprint_is_the_recorded_star() {
        // The system PSF, not the seeing alone: (3.030 / 0.7386)^2 = 16.83 px
        // against (2.5 / 0.7386)^2 = 11.46 px with seeing only.
        let (cfg, site) = fully_specified();
        let c = detection("GEO", &cfg, &site);
        assert!(close(detail_number(&c, "Footprint"), 16.8, 0.05));
    }

    #[test]
    fn detection_reports_centroid_precision() {
        // sigma = FWHM / 2.355 / SNR, so a better SNR gives a tighter centroid.
        let (cfg, site) = fully_specified();
        let geo = detection("GEO", &cfg, &site);
        let snr = detail_number(&geo, "SNR");
        let centroid = detail_number(&geo, "Centroid precision");
        assert!(close(centroid, 3030.0 / 2.3548 / snr, 0.1), "centroid {centroid} mas at SNR {snr}");
    }

    #[test]
    fn stationary_regimes_share_a_limiting_magnitude() {
        // LEO, MEO, GEO and HEO all hold the target still, so they share an
        // exposure, a zero trail, a footprint and a noise budget, and
        // therefore a limiting magnitude of 19.87 (the footprint is the
        // 3.03" recorded star, 16.8 px). They differ only in target
        // magnitude. A guard against the noise terms picking up a spurious
        // range dependence.
        let (cfg, site) = fully_specified();
        let leo = detail_number(&detection("LEO", &cfg, &site), "Limiting magnitude");
        assert!(close(leo, 19.87, 0.02), "LEO limiting magnitude {leo}");
        for key in ["MEO", "GEO", "HEO"] {
            let m = detail_number(&detection(key, &cfg, &site), "Limiting magnitude");
            assert!(close(m, leo, 0.001), "{key} limiting magnitude {m} differs from LEO {leo}");
        }
    }

    #[test]
    fn detection_warns_when_the_trail_runs_off_the_sensor() {
        // Review Focus 4: a long exposure on a sidereally tracked cislunar
        // target streaks it out of the field. Reporting a confident SNR for a
        // target that left the sensor would be worse than useless. The short
        // side of this field is 1.31 deg = 4716"; at 0.549"/s that takes
        // 8,590 s to cross, so 20,000 s is comfortably past it.
        let (mut cfg, site) = fully_specified();
        cfg.exposure_override_s = Some(20_000.0);
        let c = detection("CIS", &cfg, &site);
        assert!(c.details.iter().any(|d| d.contains("longer than the short side")));
        assert!(c.verdict.contains("streaks off the sensor"));
        assert!(c.status >= Status::Warn, "a target off the sensor must not be a PASS");
    }

    #[test]
    fn detection_honours_an_entered_target_magnitude() {
        let (mut cfg, site) = fixture(Some(l350(Some(10.0))));
        cfg.target_mag_override = Some(14.0);
        let c = detection("GEO", &cfg, &site);
        assert!(c.details.iter().any(|d| d.contains("14.00") && d.contains("entered")));
    }

    #[test]
    fn acceleration_is_info_with_no_mount() {
        let (cfg, site) = fixture(None);
        let c = check_for("LEO", "Acceleration", &cfg, &site);
        assert_eq!(c.status, Status::Info);
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
        assert!(close(OrbitCalculator::circular_speed_km_s(500.0), 7.613, 0.002));
        assert!(close(OrbitCalculator::overhead_rate_arcsec_s(500.0), 3140.0, 2.0));
    }

    #[test]
    fn meo_overhead_rate() {
        assert!(close(OrbitCalculator::overhead_rate_arcsec_s(20_200.0), 39.6, 0.2));
    }

    #[test]
    fn geo_and_lunar_rates_from_period() {
        assert!(close(OrbitCalculator::rate_from_period_arcsec_s(86_164.0905), 15.041, 0.001));
        assert!(close(OrbitCalculator::rate_from_period_arcsec_s(27.321_661 * 86_400.0), 0.549, 0.001));
    }

    #[test]
    fn molniya_apogee_rate() {
        let a = 26_560.0;
        let r = a * 1.74;
        let v = OrbitCalculator::vis_viva_km_s(r, a);
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
