//! Printing evaluations and the comparison table.

use crate::checks::{Evaluation, Status};
use crate::model::{Config, Shutter, Site};
use crate::regimes::Component;

const RULE: &str = "==========================================================================";

pub fn print_evaluation(cfg: &Config, ev: &Evaluation, site: &Site) {
    let t = &cfg.telescope;
    let c = &cfg.camera;
    println!("\n{RULE}");
    println!(" {}", ev.label);
    println!("{RULE}");
    println!(
        " Telescope: {}  (D = {:.0} mm, FL = {:.0} mm, f/{:.2})",
        t.name,
        t.aperture_mm,
        t.focal_length_mm,
        t.f_ratio()
    );
    let shutter = match c.shutter {
        Shutter::Global => "global shutter",
        Shutter::Rolling { .. } => "rolling shutter",
    };
    println!(
        " Camera:    {}  ({:.2} um pixels, {} x {} px, {shutter})",
        c.name, c.pixel_um, c.width_px, c.height_px
    );
    println!(
        " Site:      seeing {:.2}\" FWHM, wavelength {:.2} um",
        site.seeing_arcsec, site.wavelength_um
    );

    for chk in &ev.checks {
        println!("\n{} {}. {}", chk.status.tag(), chk.number, chk.title);
        for d in &chk.details {
            println!("       {d}");
        }
        println!("       -> {}", chk.verdict);
    }

    if let Some(m) = &cfg.payload.mount {
        println!("\n Mount:     {}", m.name);
    }
    println!(" Timestamp accuracy: {} ms", cfg.timestamp_accuracy_ms);

    println!("\n[----] Timing reference, GEO stare mode (supplementary, not scored)");
    for s in &ev.supplementary {
        println!("       {s}");
    }

    print_regime_summary(ev);

    let count = |s: Status| ev.checks.iter().filter(|c| c.status == s).count();
    println!(
        "\n Summary of the eight checks: {} pass, {} warn, {} fail, {} info",
        count(Status::Pass),
        count(Status::Warn),
        count(Status::Fail),
        count(Status::Info)
    );
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(n - 1).collect();
        out.push('~');
        out
    }
}

pub fn print_comparison(evals: &[Evaluation]) {
    if evals.is_empty() {
        println!("\nNo configurations evaluated yet.");
        return;
    }
    let reference = &evals[0];
    println!("\n{RULE}");
    println!(" Comparison (reference = first configuration: {})", reference.label);
    println!("{RULE}");
    println!(
        "{:<28} {:>6} {:>6} {:>4} {:>7} {:>7} {:>6} {:>6} {:>6} {:>6}",
        "Configuration", "\"/px", "px/*", "bin", "FOVdeg2", "Area m2", "dMag", "Search", "CFZ+/-", "Load%"
    );
    for e in evals {
        let m = &e.metrics;
        let dmag = 2.5 * (m.effective_area_m2 / reference.metrics.effective_area_m2).log10();
        let search = m.etendue / reference.metrics.etendue;
        let load = m
            .payload_fraction
            .map(|f| format!("{:.0}", f * 100.0))
            .unwrap_or_else(|| "-".to_string());
        println!(
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
        );
    }
    println!("\n Columns: plate scale, pixels across a star (native), best bin, field area,");
    println!(" effective collecting area, depth vs reference (+ is fainter), search speed");
    println!(" (etendue) vs reference, critical focus zone in um, payload as % of mount rating.");

    println!("\n Status by check (P=pass W=warn F=fail i=info):");
    println!("{:<28}  1 2 3 4 5 6 7 8", "");
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
        println!("{:<28}  {}", truncate(&e.label, 28), row.join(" "));
    }

    if let Some(first) = evals.first() {
        println!("\n Orbital regimes, telescope/camera/mount (P=pass W=warn F=fail i=info):");
        let header: Vec<String> = first.regimes.iter().map(|r| format!("{:<7}", r.regime.key)).collect();
        println!("{:<28}  {}", "", header.join(" "));
        for e in evals {
            let cells: Vec<String> = e
                .regimes
                .iter()
                .map(|r| {
                    format!(
                        "{:<7}",
                        format!(
                            "{}/{}/{}",
                            letter(r.component_status(Component::Telescope)),
                            letter(r.component_status(Component::Camera)),
                            letter(r.component_status(Component::Mount))
                        )
                    )
                })
                .collect();
            println!("{:<28}  {}", truncate(&e.label, 28), cells.join(" "));
        }
    }
}

pub fn print_formulas() {
    println!(
        r#"
Formula summary (full explanations in README.md)

 1 Sensor fit        image circle >= sensor diagonal = sqrt(w^2 + h^2)
 2 Plate scale       scale ("/px) = 206.265 x pixel (um) / focal length (mm)
   Sampling          pixels across a star = seeing FWHM (") / scale   (target ~2)
 3 Ideal pixel       pixel (um) = (seeing / 2) x focal length (mm) / 206.265
 4 Seeing blur       blur (um) = seeing (") x focal length (mm) / 206.265
   Optics FWHM       ~1.665 x RMS radius  (or ~0.833 x RMS diameter)
   Star growth       sqrt(seeing^2 + optics^2) / seeing - 1
 5 Effective area    pi/4 x D^2 x (1 - obstruction_by_diameter^2)
   Depth             dMag = 2.5 x log10(area / reference area)
 6 Field of view     FOV (deg) = sensor (mm) / focal length (mm) x 57.3
   Search speed      etendue = effective area x field area
 7 Focus tolerance   CFZ = +/- 2.44 x wavelength x N^2,  N = FL / D
 8 Payload           (OTA + camera + accessories) / mount rating  (keep <= 70%)
   GEO timing        star drift = 15.04"/s,  rolling skew = rows x line time

Orbital regimes (component checks per regime)
   LEO/MEO rate      omega ~ v / h,  v = sqrt(mu / (R + h))   (overhead pass)
   GEO/lunar rate    rate vs stars = 1,296,000" / orbital period
   Acquisition       margin = (short side of field / 2) / (ephemeris_km / range_km x 206265 + pointing)
   Timing needed     dt = 0.25 x binned scale / rate vs stars
   Shutter skew      skew = rate vs stars x rows x line time
   Trailing          t = seeing / rate vs stars
   Mount headroom    max axis rate / rate vs ground   (keep >= 3x)
   Alt-az keyhole    highest followable pass = 90 deg - (omega / max azimuth rate) in degrees
"#
    );
}

fn word(s: Status) -> &'static str {
    match s {
        Status::Pass => "PASS",
        Status::Warn => "WARN",
        Status::Fail => "FAIL",
        Status::Info => "info",
    }
}

fn letter(s: Status) -> &'static str {
    match s {
        Status::Pass => "P",
        Status::Warn => "W",
        Status::Fail => "F",
        Status::Info => "i",
    }
}

/// Compact component-by-regime table printed with every evaluation.
pub fn print_regime_summary(ev: &Evaluation) {
    println!("\n[----] Orbital regimes (worst status per component; details via the menu or --demo)");
    println!("       {:<31} {:<10} {:<8} {:<8} {}", "Regime", "Telescope", "Camera", "Mount", "Overall");
    for r in &ev.regimes {
        println!(
            "       {:<31} {:<10} {:<8} {:<8} {}",
            format!("{} ({})", r.regime.key, r.regime.name),
            word(r.component_status(Component::Telescope)),
            word(r.component_status(Component::Camera)),
            word(r.component_status(Component::Mount)),
            word(r.overall())
        );
    }
}

/// Full detail for every regime check of one configuration.
pub fn print_regime_details(ev: &Evaluation) {
    println!("\n{RULE}");
    println!(" Orbital-regime evaluation: {}", ev.label);
    println!("{RULE}");
    for r in &ev.regimes {
        let g = &r.regime;
        println!("\n--- {} : {} -- {} ---", g.key, g.name, g.case);
        println!(
            "    range {:.0} km | vs stars {:.2}\"/s | vs ground {:.2}\"/s | prediction error {:.0} km | usual mode: {}",
            g.range_km,
            g.rate_vs_stars,
            g.rate_vs_ground,
            g.ephemeris_uncertainty_km,
            g.mode.describe()
        );
        for comp in [Component::Telescope, Component::Camera, Component::Mount] {
            for k in r.checks.iter().filter(|k| k.component == comp) {
                println!("\n  {} {}: {}", k.status.tag(), comp.name(), k.title);
                for d in &k.details {
                    println!("         {d}");
                }
                println!("         -> {}", k.verdict);
            }
        }
        println!(
            "\n  {} overall: telescope {}, camera {}, mount {}",
            g.key,
            word(r.component_status(Component::Telescope)),
            word(r.component_status(Component::Camera)),
            word(r.component_status(Component::Mount))
        );
    }
}
