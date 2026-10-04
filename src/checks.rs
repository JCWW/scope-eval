//! The eight checks, plus supplementary GEO motion and timing figures.
//!
//! Every formula here is explained in README.md. The calculation functions
//! at the top of the file are pure (numbers in, number out) so they can be
//! unit tested and reused without the interactive front end. The `check_*`
//! functions turn those numbers into a pass/warn/fail judgment with an
//! explanation.

use crate::model::{Camera, Config, Shutter, Site, SpotConvention, SpotPoint, Telescope};
use crate::regimes::{evaluate_regimes, RegimeEvaluation};
use std::f64::consts::PI;

// ---------------------------------------------------------------------------
// Physical constants
// ---------------------------------------------------------------------------

/// Arcseconds in one radian (180 / pi * 3600).
pub const ARCSEC_PER_RADIAN: f64 = 206_264.806;
/// Degrees in one radian.
pub const DEG_PER_RADIAN: f64 = 180.0 / PI;
/// Apparent drift of the stars past an Earth-fixed (GEO) object, arcsec per second.
/// One full turn (1,296,000") per sidereal day (86,164.09 s).
pub const SIDEREAL_RATE_ARCSEC_PER_S: f64 = 1_296_000.0 / 86_164.0905;
/// For a round Gaussian blur, FWHM = 2*sqrt(ln 2) * (RMS radius) = 1.665 * RMS radius.
pub const FWHM_PER_RMS_RADIUS: f64 = 1.665_109;

// ---------------------------------------------------------------------------
// Judgment thresholds. These are engineering rules of thumb, not physics.
// Change them here to tune the tool to your program's standards.
// ---------------------------------------------------------------------------
pub mod limits {
    /// Check 1: image circle may be this fraction of the sensor diagonal before failing.
    pub const FIT_WARN_FRACTION: f64 = 0.90;
    /// Check 2: ideal number of pixels across a star's FWHM.
    pub const SAMPLING_TARGET: f64 = 2.0;
    pub const SAMPLING_UNDER_FAIL: f64 = 1.0;
    pub const SAMPLING_GOOD_MIN: f64 = 1.5;
    pub const SAMPLING_GOOD_MAX: f64 = 2.5;
    pub const SAMPLING_BIN2_MAX: f64 = 4.0;
    pub const SAMPLING_OVER_FAIL: f64 = 6.0;
    /// Largest square bin factor the tool will recommend.
    pub const MAX_BIN: u32 = 4;
    /// Check 3: effective pixel within this ratio of ideal counts as a match.
    pub const PIXEL_MATCH_LOW: f64 = 0.75;
    pub const PIXEL_MATCH_HIGH: f64 = 1.33;
    /// Check 4: allowed growth of the star image caused by the optics.
    pub const OPTICS_PASS_GROWTH: f64 = 0.15;
    pub const OPTICS_WARN_GROWTH: f64 = 0.35;
    /// Check 7: critical focus zone half-widths, micrometers.
    pub const CFZ_FORGIVING_UM: f64 = 40.0;
    pub const CFZ_DEMANDING_UM: f64 = 15.0;
    /// Check 8: payload as a fraction of mount capacity.
    pub const PAYLOAD_PASS_FRACTION: f64 = 0.70;
    pub const PAYLOAD_WARN_FRACTION: f64 = 0.90;
}

// ---------------------------------------------------------------------------
// Pure calculations
// ---------------------------------------------------------------------------

/// Plate scale: sky angle seen by one pixel, arcsec/px.
/// Small-angle rule: angle = size / distance, with the focal length as the distance.
pub fn plate_scale_arcsec_per_px(pixel_um: f64, focal_length_mm: f64) -> f64 {
    ARCSEC_PER_RADIAN * (pixel_um / 1000.0) / focal_length_mm
}

/// How many pixels span a star's FWHM.
pub fn pixels_across_star(seeing_arcsec: f64, plate_scale: f64) -> f64 {
    seeing_arcsec / plate_scale
}

/// Square bin factor (1..=MAX_BIN) that brings pixels-across-a-star closest to the target.
pub fn best_bin(pixels_across: f64) -> u32 {
    (1..=limits::MAX_BIN)
        .min_by(|a, b| {
            let da = (pixels_across / *a as f64 - limits::SAMPLING_TARGET).abs();
            let db = (pixels_across / *b as f64 - limits::SAMPLING_TARGET).abs();
            da.partial_cmp(&db).unwrap()
        })
        .unwrap()
}

/// Pixel size (um) that would put SAMPLING_TARGET pixels across a star at this focal length.
/// This is the plate-scale formula solved for pixel size.
pub fn ideal_pixel_um(seeing_arcsec: f64, focal_length_mm: f64) -> f64 {
    (seeing_arcsec / limits::SAMPLING_TARGET) / ARCSEC_PER_RADIAN * focal_length_mm * 1000.0
}

/// Physical size of the seeing blur on the focal plane, um.
pub fn seeing_blur_um(seeing_arcsec: f64, focal_length_mm: f64) -> f64 {
    seeing_arcsec / ARCSEC_PER_RADIAN * focal_length_mm * 1000.0
}

/// Convert a quoted RMS spot figure to an approximate FWHM, assuming a round Gaussian blur.
pub fn spot_fwhm_um(rms_um: f64, is_radius: bool) -> f64 {
    if is_radius {
        FWHM_PER_RMS_RADIUS * rms_um
    } else {
        FWHM_PER_RMS_RADIUS * rms_um / 2.0
    }
}

/// Combine two independent blurs. Widths of independent Gaussian blurs add in quadrature.
pub fn quadrature(a: f64, b: f64) -> f64 {
    a.hypot(b)
}

/// RMS spot at a field radius, by linear interpolation between quoted points.
/// Returns (value, extrapolated?). Beyond the last point it extrapolates linearly
/// from the last two points (never below the last value), and flags it.
pub fn spot_rms_at(points: &[SpotPoint], radius_mm: f64) -> Option<(f64, bool)> {
    let first = points.first()?;
    if points.len() == 1 {
        return Some((first.rms_um, radius_mm > first.field_radius_mm));
    }
    if radius_mm <= first.field_radius_mm {
        return Some((first.rms_um, false));
    }
    for w in points.windows(2) {
        let (a, b) = (w[0], w[1]);
        if radius_mm <= b.field_radius_mm {
            let span = b.field_radius_mm - a.field_radius_mm;
            if span <= 0.0 {
                return Some((b.rms_um, false));
            }
            let t = (radius_mm - a.field_radius_mm) / span;
            return Some((a.rms_um + t * (b.rms_um - a.rms_um), false));
        }
    }
    let n = points.len();
    let (a, b) = (points[n - 2], points[n - 1]);
    let span = b.field_radius_mm - a.field_radius_mm;
    let slope = if span > 0.0 { (b.rms_um - a.rms_um) / span } else { 0.0 };
    let value = b.rms_um + slope * (radius_mm - b.field_radius_mm);
    Some((value.max(b.rms_um), true))
}

/// Light-collecting area after subtracting the central obstruction, m^2.
pub fn effective_area_m2(aperture_mm: f64, blocked_area_fraction: f64) -> f64 {
    let d_m = aperture_mm / 1000.0;
    PI / 4.0 * d_m * d_m * (1.0 - blocked_area_fraction)
}

/// Depth gain in magnitudes from collecting more light. Positive = fainter limit.
pub fn delta_mag(area_m2: f64, reference_area_m2: f64) -> f64 {
    2.5 * (area_m2 / reference_area_m2).log10()
}

/// Field of view along one sensor axis, degrees (small-angle approximation).
pub fn fov_deg(sensor_mm: f64, focal_length_mm: f64) -> f64 {
    sensor_mm / focal_length_mm * DEG_PER_RADIAN
}

/// Critical focus zone half-width, um: +/- 2.44 * lambda * N^2.
pub fn critical_focus_zone_um(wavelength_um: f64, f_ratio: f64) -> f64 {
    2.44 * wavelength_um * f_ratio * f_ratio
}

// ---------------------------------------------------------------------------
// Results
// ---------------------------------------------------------------------------

/// Ordered so that the "worst" status compares greatest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    Info,
    Pass,
    Warn,
    Fail,
}

impl Status {
    pub fn tag(&self) -> &'static str {
        match self {
            Status::Info => "[INFO]",
            Status::Pass => "[PASS]",
            Status::Warn => "[WARN]",
            Status::Fail => "[FAIL]",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CheckResult {
    pub number: u8,
    pub title: &'static str,
    pub status: Status,
    pub details: Vec<String>,
    pub verdict: String,
}

/// Headline numbers kept for the comparison table.
#[derive(Debug, Clone, Copy)]
pub struct Metrics {
    pub f_ratio: f64,
    pub plate_scale: f64,
    pub pixels_across: f64,
    pub recommended_bin: u32,
    pub fov_w_deg: f64,
    pub fov_h_deg: f64,
    pub fov_area_deg2: f64,
    pub effective_area_m2: f64,
    pub etendue: f64,
    pub cfz_um: f64,
    pub payload_fraction: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct Evaluation {
    pub label: String,
    pub metrics: Metrics,
    pub checks: Vec<CheckResult>,
    pub supplementary: Vec<String>,
    /// Component evaluations against each orbital regime (see regimes.rs).
    pub regimes: Vec<RegimeEvaluation>,
}

/// The configuration that relative numbers (depth, search speed) are compared against.
#[derive(Debug, Clone)]
pub struct Reference {
    pub label: String,
    pub effective_area_m2: f64,
    pub etendue: f64,
}

impl Reference {
    pub fn from_evaluation(e: &Evaluation) -> Self {
        Reference {
            label: e.label.clone(),
            effective_area_m2: e.metrics.effective_area_m2,
            etendue: e.metrics.etendue,
        }
    }
}

/// Format a "label ........ value" detail line.
pub(crate) fn kv(label: &str, value: String) -> String {
    format!("{:.<44} {}", format!("{label} "), value)
}

// ---------------------------------------------------------------------------
// The eight checks
// ---------------------------------------------------------------------------

/// Check 1: does the whole sensor fit inside the corrected image circle?
pub fn check_sensor_fit(t: &Telescope, c: &Camera) -> CheckResult {
    let diag = c.diagonal_mm();
    let ic = t.image_circle_mm;
    let details = vec![
        kv("Sensor size", format!("{:.1} x {:.1} mm", c.width_mm(), c.height_mm())),
        kv("Sensor diagonal", format!("{diag:.1} mm")),
        kv("Corrected image circle", format!("{ic:.1} mm")),
        kv("Image circle / diagonal", format!("{:.2}", ic / diag)),
    ];
    let (status, verdict) = if ic >= diag {
        let headroom = if ic > 1.15 * diag {
            format!(" Headroom for a sensor up to about {ic:.0} mm diagonal.")
        } else {
            String::new()
        };
        (Status::Pass, format!("The whole sensor sits inside the corrected field.{headroom}"))
    } else if ic >= limits::FIT_WARN_FRACTION * diag {
        (
            Status::Warn,
            format!(
                "The sensor corners fall outside the corrected field (outer {:.0}% of the diagonal). Expect soft or dim corners.",
                (1.0 - ic / diag) * 100.0
            ),
        )
    } else {
        (
            Status::Fail,
            "A large part of the sensor is outside the corrected field. Use a smaller sensor or crop.".to_string(),
        )
    };
    CheckResult { number: 1, title: "Sensor fit", status, details, verdict }
}

/// Check 2: is a star spread across the right number of pixels?
/// Returns the check plus (plate scale, pixels across, recommended bin).
pub fn check_sampling(t: &Telescope, c: &Camera, site: &Site) -> (CheckResult, f64, f64, u32) {
    let scale = plate_scale_arcsec_per_px(c.pixel_um, t.focal_length_mm);
    let p = pixels_across_star(site.seeing_arcsec, scale);
    let bin = best_bin(p);
    let footprint = p * p;
    let ideal_footprint = limits::SAMPLING_TARGET * limits::SAMPLING_TARGET;

    let mut details = vec![
        kv("Plate scale (native)", format!("{scale:.3}\"/px")),
        kv("Pixels across a star (native)", format!("{p:.2}  (target {:.1})", limits::SAMPLING_TARGET)),
        kv("Best square bin", format!("{bin}x{bin}")),
        kv(
            "After binning",
            format!("{:.3}\"/px, {:.2} px across a star", scale * bin as f64, p / bin as f64),
        ),
        kv("Pixels in a star's footprint (native)", format!("~{footprint:.0}  (ideal ~{ideal_footprint:.0})")),
        kv(
            "Read-noise penalty vs ideal (CMOS)",
            format!("x{:.1} read-noise variance", (footprint / ideal_footprint).max(1.0)),
        ),
    ];
    if let Some(r) = c.read_noise_e {
        details.push(kv(
            "Read-noise variance per star",
            format!("{:.0} e-^2  (= footprint x {r:.1}^2)", footprint * r * r),
        ));
    }

    let (status, verdict) = if p < limits::SAMPLING_UNDER_FAIL {
        (
            Status::Fail,
            format!("Undersampled: a star spans only {p:.1} px. Centroids and astrometry suffer. Use smaller pixels or a longer focal length."),
        )
    } else if p < limits::SAMPLING_GOOD_MIN {
        (
            Status::Warn,
            "Slightly undersampled. Usable for detection, but pixel size limits astrometric precision.".to_string(),
        )
    } else if p <= limits::SAMPLING_GOOD_MAX {
        (Status::Pass, "Well sampled at native resolution.".to_string())
    } else if p <= limits::SAMPLING_BIN2_MAX {
        (
            Status::Pass,
            format!(
                "Mildly oversampled. Fine as-is, or bin 2x2 ({:.1} px across) to cut data volume 4x.",
                p / 2.0
            ),
        )
    } else if p <= limits::SAMPLING_OVER_FAIL {
        (
            Status::Warn,
            format!(
                "Oversampled. Binning {bin}x{bin} fixes the image geometry, but on a CMOS sensor digital binning cannot remove the extra read noise."
            ),
        )
    } else {
        (
            Status::Fail,
            format!(
                "Heavily oversampled: each star is smeared over ~{footprint:.0} pixels. On CMOS this is a permanent sensitivity penalty for faint targets."
            ),
        )
    };
    (CheckResult { number: 2, title: "Sampling (plate scale vs seeing)", status, details, verdict }, scale, p, bin)
}

/// Check 3: what pixel size does this telescope want, and does the camera provide it?
pub fn check_ideal_pixel(t: &Telescope, c: &Camera, site: &Site) -> CheckResult {
    let ideal = ideal_pixel_um(site.seeing_arcsec, t.focal_length_mm);
    // Bin factor whose effective pixel is closest to ideal on a ratio (log) scale.
    let b = (1..=limits::MAX_BIN)
        .min_by(|x, y| {
            let ex = (c.pixel_um * *x as f64 / ideal).ln().abs();
            let ey = (c.pixel_um * *y as f64 / ideal).ln().abs();
            ex.partial_cmp(&ey).unwrap()
        })
        .unwrap();
    let effective = c.pixel_um * b as f64;
    let m = effective / ideal;
    let in_range = (limits::PIXEL_MATCH_LOW..=limits::PIXEL_MATCH_HIGH).contains(&m);

    let details = vec![
        kv("Ideal pixel for this focal length", format!("{ideal:.1} um")),
        kv("Camera pixel", format!("{:.2} um  (ratio {:.2})", c.pixel_um, c.pixel_um / ideal)),
        kv("Closest binned match", format!("{b}x{b} -> {effective:.2} um  (ratio {m:.2})")),
    ];

    let (status, verdict) = if in_range && b == 1 {
        (Status::Pass, "Natural match: the camera samples this telescope correctly without binning.".to_string())
    } else if in_range && b == 2 {
        (Status::Pass, "Good match after 2x2 binning.".to_string())
    } else if in_range {
        (
            Status::Warn,
            format!(
                "Only matches with {b}x{b} binning. Digital (CMOS) binning cannot remove read noise, so this pairing pays a sensitivity penalty. A camera with ~{ideal:.0} um pixels would suit this telescope."
            ),
        )
    } else if m > limits::PIXEL_MATCH_HIGH {
        (
            Status::Warn,
            format!("Camera pixels are larger than this telescope wants (undersampled). Look for pixels near {ideal:.1} um."),
        )
    } else {
        (
            Status::Warn,
            format!("Camera pixels are far smaller than this telescope wants, even at {b}x{b} binning. Look for pixels near {ideal:.0} um."),
        )
    };
    CheckResult { number: 3, title: "Ideal pixel size (camera match)", status, details, verdict }
}

fn optics_status(growth: f64) -> Status {
    if growth <= limits::OPTICS_PASS_GROWTH {
        Status::Pass
    } else if growth <= limits::OPTICS_WARN_GROWTH {
        Status::Warn
    } else {
        Status::Fail
    }
}

/// Check 4: are the optics sharp enough that the atmosphere, not the glass, limits the image?
pub fn check_optics(t: &Telescope, c: &Camera, site: &Site) -> CheckResult {
    let seeing_um = seeing_blur_um(site.seeing_arcsec, t.focal_length_mm);
    let mut details = vec![kv("Seeing blur at the focal plane", format!("{seeing_um:.1} um FWHM"))];

    let spot = match &t.spot {
        Some(s) if !s.points.is_empty() => s,
        _ => {
            return CheckResult {
                number: 4,
                title: "Optical quality vs seeing",
                status: Status::Info,
                details,
                verdict: "No spot-size data entered. Ask the vendor for the RMS spot size on-axis and at your sensor's corner, and whether it is a radius or a diameter.".to_string(),
            }
        }
    };

    let corner_r = c.diagonal_mm() / 2.0;
    let (center_rms, _) = spot_rms_at(&spot.points, 0.0).unwrap();
    let (corner_rms, extrapolated) = spot_rms_at(&spot.points, corner_r).unwrap();
    let first_r = spot.points[0].field_radius_mm;
    let center_note = if first_r > 0.0 {
        format!("  (closest quoted point is {first_r:.0} mm off-axis)")
    } else {
        String::new()
    };
    details.push(kv("Quoted RMS spot, center", format!("{center_rms:.1} um{center_note}")));
    details.push(kv(
        &format!("Quoted RMS spot, sensor corner (r={corner_r:.1} mm)"),
        format!("{corner_rms:.1} um{}", if extrapolated { "  (extrapolated)" } else { "" }),
    ));

    let readings: Vec<(&str, bool)> = match spot.convention {
        SpotConvention::RmsRadius => vec![("Read as RMS radius", true)],
        SpotConvention::RmsDiameter => vec![("Read as RMS diameter", false)],
        SpotConvention::Unknown => vec![("If RMS radius", true), ("If RMS diameter", false)],
    };

    let mut results: Vec<(Status, f64)> = Vec::new();
    for (name, is_radius) in readings {
        let growth = |rms: f64| {
            let optics = spot_fwhm_um(rms, is_radius);
            (optics, quadrature(seeing_um, optics) / seeing_um - 1.0)
        };
        let (fc, gc) = growth(center_rms);
        let (fe, ge) = growth(corner_rms);
        details.push(kv(
            name,
            format!(
                "optics FWHM {fc:.1}/{fe:.1} um -> stars grow {:.0}% center, {:.0}% corner",
                gc * 100.0,
                ge * 100.0
            ),
        ));
        let worst = gc.max(ge);
        results.push((optics_status(worst), worst));
    }

    let consistent = results.iter().all(|(s, _)| *s == results[0].0);
    let worst_growth = results.iter().map(|(_, g)| *g).fold(0.0, f64::max);
    let (status, verdict) = if !consistent {
        (
            Status::Warn,
            "The answer depends on whether the vendor's RMS figure is a radius or a diameter. Ask the vendor before relying on this check.".to_string(),
        )
    } else {
        match results[0].0 {
            Status::Pass => (
                Status::Pass,
                "Seeing-limited: the atmosphere, not the optics, sets image quality across the sensor.".to_string(),
            ),
            Status::Warn => (
                Status::Warn,
                format!("The optics noticeably enlarge stars (up to {:.0}%), mostly toward the corners.", worst_growth * 100.0),
            ),
            _ => (
                Status::Fail,
                format!("Over part of the sensor the optics, not the atmosphere, dominate the blur (stars up to {:.0}% larger).", worst_growth * 100.0),
            ),
        }
    };
    CheckResult { number: 4, title: "Optical quality vs seeing", status, details, verdict }
}

/// Check 5: effective collecting area and depth relative to the reference.
pub fn check_area_depth(t: &Telescope, reference: Option<&Reference>) -> (CheckResult, f64) {
    let d_m = t.aperture_mm / 1000.0;
    let geometric = PI / 4.0 * d_m * d_m;
    let blocked = t.obstruction.area_fraction();
    let area = effective_area_m2(t.aperture_mm, blocked);
    let equivalent_d = 2.0 * (area / PI).sqrt() * 1000.0;

    let mut details = vec![
        kv("Geometric area (pi/4 x D^2)", format!("{geometric:.4} m^2")),
        kv("Central obstruction", t.obstruction.describe()),
        kv("Effective collecting area", format!("{area:.4} m^2")),
        kv("Equivalent unobstructed aperture", format!("{equivalent_d:.0} mm")),
    ];
    let verdict = match reference {
        Some(r) => {
            let ratio = area / r.effective_area_m2;
            let dm = delta_mag(area, r.effective_area_m2);
            details.push(kv("Reference configuration", r.label.clone()));
            details.push(kv("Light vs reference", format!("x{ratio:.2}")));
            details.push(kv("Depth vs reference", format!("{dm:+.2} mag")));
            if dm >= 0.0 {
                format!("Reaches about {dm:.2} mag fainter than the reference at equal exposure.")
            } else {
                format!("About {:.2} mag shallower than the reference at equal exposure.", -dm)
            }
        }
        None => "This configuration is the reference. Later configurations are compared against it.".to_string(),
    };
    (CheckResult { number: 5, title: "Collecting area and depth", status: Status::Info, details, verdict }, area)
}

/// Check 6: field of view and search speed (etendue).
/// Returns the check plus (fov width deg, fov height deg, etendue).
pub fn check_field_and_search(
    t: &Telescope,
    c: &Camera,
    area_m2: f64,
    reference: Option<&Reference>,
) -> (CheckResult, f64, f64, f64) {
    let w = fov_deg(c.width_mm(), t.focal_length_mm);
    let h = fov_deg(c.height_mm(), t.focal_length_mm);
    let fov_area = w * h;
    let etendue = area_m2 * fov_area;

    let mut details = vec![
        kv("Field of view", format!("{w:.2} x {h:.2} deg  ({:.0}' x {:.0}')", w * 60.0, h * 60.0)),
        kv("Field area", format!("{fov_area:.2} deg^2")),
        kv("Fields to tile 100 deg^2 (no overlap)", format!("~{:.0}", (100.0 / fov_area).ceil())),
        kv("Etendue (area x field)", format!("{etendue:.3} m^2 deg^2")),
    ];
    let verdict = match reference {
        Some(r) => {
            let ratio = etendue / r.etendue;
            details.push(kv("Search speed vs reference", format!("x{ratio:.2}")));
            if ratio >= 1.0 {
                format!("Surveys sky {ratio:.1}x faster than the reference (each at its own depth).")
            } else {
                format!("Surveys sky {:.1}x slower than the reference (each at its own depth).", 1.0 / ratio)
            }
        }
        None => "This configuration is the reference for search-speed comparisons.".to_string(),
    };
    (
        CheckResult { number: 6, title: "Field of view and search speed", status: Status::Info, details, verdict },
        w,
        h,
        etendue,
    )
}

/// Check 7: how tight is focus?
pub fn check_focus(t: &Telescope, site: &Site) -> (CheckResult, f64) {
    let n = t.f_ratio();
    let cfz = critical_focus_zone_um(site.wavelength_um, n);
    let details = vec![
        kv("Focal ratio N", format!("f/{n:.2}")),
        kv("Critical focus zone", format!("+/-{cfz:.1} um  (total depth {:.1} um)", 2.0 * cfz)),
    ];
    let (status, verdict) = if cfz >= limits::CFZ_FORGIVING_UM {
        (Status::Pass, "Forgiving focus. Temperature drift and minor tilt are easy to manage.".to_string())
    } else if cfz >= limits::CFZ_DEMANDING_UM {
        (
            Status::Pass,
            "Moderate. A motorized focuser with periodic autofocus is recommended.".to_string(),
        )
    } else {
        (
            Status::Warn,
            "Demanding. Requires a motorized focuser, temperature-compensated autofocus, and a sensor tilt adjuster.".to_string(),
        )
    };
    (CheckResult { number: 7, title: "Focus tolerance", status, details, verdict }, cfz)
}

/// Check 8: payload against mount capacity, and back focus.
pub fn check_practical_fit(cfg: &Config) -> (CheckResult, Option<f64>) {
    let t = &cfg.telescope;
    let c = &cfg.camera;
    let p = &cfg.payload;
    let mut details = Vec::new();
    let mut statuses: Vec<Status> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    let mut fraction = None;

    let mut unknown = Vec::new();
    let ota = t.weight_lb.unwrap_or_else(|| {
        unknown.push("OTA");
        0.0
    });
    let cam = c.weight_lb.unwrap_or_else(|| {
        unknown.push("camera");
        0.0
    });
    let total = ota + cam + p.accessories_lb;
    details.push(kv(
        "Payload (OTA + camera + accessories)",
        format!("{ota:.1} + {cam:.1} + {:.1} = {total:.1} lb ({:.1} kg)", p.accessories_lb, total * 0.4536),
    ));
    if !unknown.is_empty() {
        details.push(kv("Not included (weight unknown)", unknown.join(", ")));
    }

    match p.mount_capacity_lb() {
        Some(cap) if t.weight_lb.is_some() => {
            let f = total / cap;
            fraction = Some(f);
            let name = p.mount.as_ref().map(|m| m.name.as_str()).unwrap_or("Mount");
            details.push(kv("Mount", name.to_string()));
            details.push(kv("Mount capacity", format!("{cap:.0} lb -> payload is {:.0}% of rating", f * 100.0)));
            if f <= limits::PAYLOAD_PASS_FRACTION {
                statuses.push(Status::Pass);
                notes.push("Payload is within the comfortable range of the mount.".to_string());
            } else if f <= limits::PAYLOAD_WARN_FRACTION {
                statuses.push(Status::Warn);
                notes.push("Payload is near the mount's rating. Expect reduced stiffness in wind.".to_string());
            } else {
                statuses.push(Status::Fail);
                notes.push("Payload exceeds a safe fraction of the mount's rating.".to_string());
            }
        }
        Some(_) => notes.push("OTA weight unknown, so payload margin was not evaluated.".to_string()),
        None => notes.push("No mount capacity known.".to_string()),
    }

    match (t.back_focus_mm, p.back_focus_required_mm) {
        (Some(avail), Some(req)) => {
            details.push(kv("Back focus available / required", format!("{avail:.1} / {req:.1} mm")));
            if avail >= req {
                statuses.push(Status::Pass);
                notes.push(format!("Back focus has {:.1} mm to spare.", avail - req));
            } else {
                statuses.push(Status::Fail);
                notes.push(format!("Back focus is {:.1} mm short.", req - avail));
            }
        }
        (Some(avail), None) => details.push(kv("Back focus available", format!("{avail:.1} mm (no requirement entered)"))),
        _ => {}
    }

    let status = statuses.iter().copied().max().unwrap_or(Status::Info);
    (
        CheckResult { number: 8, title: "Practical fit (payload, back focus)", status, details, verdict: notes.join(" ") },
        fraction,
    )
}

/// Supplementary timing reference for GEO stare mode: star drift, timing sensitivity,
/// rolling-shutter skew. The regime evaluations generalize these to other orbits.
pub fn geo_motion_and_timing(c: &Camera, plate_scale: f64, bin: u32) -> Vec<String> {
    let binned = plate_scale * bin as f64;
    let rate = SIDEREAL_RATE_ARCSEC_PER_S;
    let mut out = vec![
        kv("Star drift past a GEO target (stare mode)", format!("{rate:.2}\"/s")),
        kv(
            &format!("Star streak per 1 s exposure ({bin}x{bin} bin)"),
            format!("{:.1} px", rate / binned),
        ),
        kv(
            "Position error per 10 ms of timing error",
            format!("{:.3}\" = {:.2} px ({bin}x{bin} bin)", rate * 0.010, rate * 0.010 / binned),
        ),
    ];
    match c.shutter {
        Shutter::Global => out.push(kv("Shutter", "global: every row exposed at once, no row correction".to_string())),
        Shutter::Rolling { line_time_us: Some(lt) } => {
            let skew_s = c.height_px as f64 * lt * 1e-6;
            out.push(kv(
                "Rolling-shutter readout skew",
                format!("{} rows x {lt:.3} us = {:.3} s", c.height_px, skew_s),
            ));
            out.push(kv(
                "Star position skew, top to bottom",
                format!("{:.2}\" = {:.1} px ({bin}x{bin} bin)", rate * skew_s, rate * skew_s / binned),
            ));
            out.push(kv("Per-row time correction", format!("t(row) = t(first row) + row x {lt:.3} us")));
        }
        Shutter::Rolling { line_time_us: None } => out.push(kv(
            "Shutter",
            "rolling, line time unknown: ask the vendor (skew = rows x line time)".to_string(),
        )),
    }
    out
}

/// Run all eight checks on one configuration.
pub fn evaluate(cfg: &Config, site: &Site, reference: Option<&Reference>) -> Evaluation {
    let t = &cfg.telescope;
    let c = &cfg.camera;
    let fit = check_sensor_fit(t, c);
    let (sampling, scale, px_across, bin) = check_sampling(t, c, site);
    let ideal = check_ideal_pixel(t, c, site);
    let optics = check_optics(t, c, site);
    let (area_check, area) = check_area_depth(t, reference);
    let (fov_check, fov_w, fov_h, etendue) = check_field_and_search(t, c, area, reference);
    let (focus, cfz) = check_focus(t, site);
    let (practical, payload_fraction) = check_practical_fit(cfg);
    let supplementary = geo_motion_and_timing(c, scale, bin);

    let mut ev = Evaluation {
        label: cfg.label.clone(),
        metrics: Metrics {
            f_ratio: t.f_ratio(),
            plate_scale: scale,
            pixels_across: px_across,
            recommended_bin: bin,
            fov_w_deg: fov_w,
            fov_h_deg: fov_h,
            fov_area_deg2: fov_w * fov_h,
            effective_area_m2: area,
            etendue,
            cfz_um: cfz,
            payload_fraction,
        },
        checks: vec![fit, sampling, ideal, optics, area_check, fov_check, focus, practical],
        supplementary,
        regimes: Vec::new(),
    };
    ev.regimes = evaluate_regimes(cfg, &ev, site.seeing_arcsec, reference.map(|r| r.effective_area_m2));
    ev
}

/// Evaluate a list of configurations. The first one is the reference for the rest.
pub fn evaluate_all(configs: &[Config], site: &Site) -> Vec<Evaluation> {
    let mut out: Vec<Evaluation> = Vec::with_capacity(configs.len());
    for cfg in configs {
        let reference = out.first().map(Reference::from_evaluation);
        out.push(evaluate(cfg, site, reference.as_ref()));
    }
    out
}

// ---------------------------------------------------------------------------
// Tests: the worked examples from README.md
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Obstruction;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn plate_scale_deltarho350_imx455() {
        assert!(close(plate_scale_arcsec_per_px(3.76, 1050.0), 0.7386, 0.0005));
    }

    #[test]
    fn plate_scale_rasa11_imx455() {
        assert!(close(plate_scale_arcsec_per_px(3.76, 620.0), 1.2509, 0.0005));
    }

    #[test]
    fn field_of_view_deltarho350() {
        assert!(close(fov_deg(36.0, 1050.0), 1.964, 0.002));
        assert!(close(fov_deg(24.0, 1050.0), 1.310, 0.002));
    }

    #[test]
    fn effective_area_and_depth() {
        let dr = effective_area_m2(350.0, Obstruction::ByDiameter(0.56).area_fraction());
        let rasa = effective_area_m2(279.0, Obstruction::ByDiameter(114.0 / 279.0).area_fraction());
        assert!(close(dr, 0.0660, 0.0005));
        assert!(close(rasa, 0.0509, 0.0005));
        assert!(close(delta_mag(dr, rasa), 0.28, 0.01));
    }

    #[test]
    fn obstruction_by_area_matches_by_diameter() {
        // CDK17: 49% by diameter is quoted as 23.7% by area.
        let a = Obstruction::ByDiameter(0.49).area_fraction();
        let b = Obstruction::ByArea(0.237).area_fraction();
        assert!(close(a, b, 0.004));
    }

    #[test]
    fn critical_focus_zone_values() {
        assert!(close(critical_focus_zone_um(0.55, 3.0), 12.08, 0.01));
        assert!(close(critical_focus_zone_um(0.55, 2.2), 6.50, 0.01));
    }

    #[test]
    fn ideal_pixel_values() {
        assert!(close(ideal_pixel_um(2.5, 1050.0), 6.36, 0.01));
        assert!(close(ideal_pixel_um(2.5, 620.0), 3.76, 0.01));
    }

    #[test]
    fn best_bin_choices() {
        assert_eq!(best_bin(2.0), 1);
        assert_eq!(best_bin(3.38), 2);
        assert_eq!(best_bin(8.25), 4);
    }

    #[test]
    fn spot_interpolation() {
        let pts = vec![
            SpotPoint { field_radius_mm: 0.0, rms_um: 4.9 },
            SpotPoint { field_radius_mm: 23.0, rms_um: 6.2 },
            SpotPoint { field_radius_mm: 30.0, rms_um: 7.6 },
        ];
        let (mid, ex) = spot_rms_at(&pts, 11.5).unwrap();
        assert!(close(mid, 5.55, 0.001) && !ex);
        let (beyond, ex) = spot_rms_at(&pts, 32.0).unwrap();
        assert!(close(beyond, 8.0, 0.001) && ex);
    }

    #[test]
    fn rolling_shutter_skew_imx455() {
        let skew = 6388.0 * 39.028e-6;
        assert!(close(skew, 0.2493, 0.0005));
        assert!(close(SIDEREAL_RATE_ARCSEC_PER_S * skew, 3.75, 0.01));
    }
}
