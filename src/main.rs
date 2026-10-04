//! scope-eval: interactive telescope + camera evaluation calculator.
//!
//! Run with no arguments for the interactive menu, `--demo` for a canned
//! comparison of the presets, or `--help` for usage. See README.md for the
//! full explanation of every calculation.

mod checks;
mod constants;
mod dynamics;
mod input;
mod model;
mod photometry;
mod presets;
mod regimes;
mod report;

use constants::{
    DEFAULT_SEEING_ARCSEC, DEFAULT_TIMESTAMP_MS, DEFAULT_WAVELENGTH_UM, DEMO_ASSUMED_POINTING_RMS_ARCSEC,
    GPS_TIMESTAMP_MS,
};
use model::{
    Camera, Capability, Config, Mount, MountType, Obstruction, Payload, Shutter, Site, SpotConvention, SpotPoint,
    SpotSpec, Telescope,
};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return;
    }
    if args.iter().any(|a| a == "--demo") {
        run_demo();
        return;
    }
    run_interactive();
}

fn print_help() {
    println!(
        "scope-eval {}

Evaluates a telescope + camera + mount configuration against eight checks:
  1 sensor fit   2 sampling   3 ideal pixel   4 optics vs seeing
  5 area/depth   6 field/search speed   7 focus tolerance   8 practical fit

Then evaluates the telescope, camera, mount and the configuration as a whole
against five orbital regimes: LEO, MEO, GEO, HEO (Molniya) and cislunar,
covering tracking rate, axis acceleration, slew-and-settle timing, timing
accuracy, shutter skew, acquisition and whether the target is bright enough
to detect.

USAGE:
  scope-eval            interactive menu
  scope-eval --demo     evaluate the built-in presets and print a comparison
  scope-eval --help     this message

See README.md for the formulas and the algorithm.",
        env!("CARGO_PKG_VERSION")
    );
}

fn run_interactive() {
    println!("==========================================================================");
    println!(" scope-eval: telescope + camera evaluation calculator");
    println!("==========================================================================");
    println!("Press Enter to accept a [default]. The first configuration you evaluate");
    println!("becomes the reference for depth and search-speed comparisons.");

    println!("\nSite conditions. Sky brightness is V magnitudes per square arcsecond:");
    println!("about 21.9 at a dark rural site, 21.0 rural, 18.5 suburban. Larger is darker.");
    let mut site = Site {
        seeing_arcsec: input::ask_positive("Typical seeing FWHM at your site, arcsec", Some(DEFAULT_SEEING_ARCSEC)),
        wavelength_um: DEFAULT_WAVELENGTH_UM,
        sky_mag_arcsec2: input::ask_optional_hint(
            "Sky brightness, mag/arcsec^2",
            "blank to assume 21.0 and have detection capped at WARN",
            false,
        ),
    };
    let mut configs: Vec<Config> = Vec::new();

    loop {
        let menu = [
            "Evaluate a telescope + camera configuration".to_string(),
            "Compare all evaluated configurations".to_string(),
            "Show detailed orbital-regime evaluation for a configuration".to_string(),
            format!(
                "Change site conditions (seeing {:.2}\", sky {})",
                site.seeing_arcsec,
                match site.sky_mag_arcsec2 {
                    Some(s) => format!("{s:.2}"),
                    None => "assumed".to_string(),
                }
            ),
            "Show formula summary".to_string(),
            "Remove all configurations and start over".to_string(),
            "Quit".to_string(),
        ];
        match input::ask_menu("Main menu", &menu) {
            0 => {
                let cfg = build_config();
                configs.push(cfg);
                let evals = checks::evaluate_all(&configs, &site);
                let last = configs.len() - 1;
                report::print_evaluation(&configs[last], &evals[last], &site);
            }
            1 => report::print_comparison(&checks::evaluate_all(&configs, &site)),
            2 => {
                if configs.is_empty() {
                    println!("\nNo configurations evaluated yet.");
                } else {
                    let labels: Vec<String> = configs.iter().map(|c| c.label.clone()).collect();
                    let k = input::ask_menu("Which configuration?", &labels);
                    let evals = checks::evaluate_all(&configs, &site);
                    report::print_regime_details(&evals[k]);
                }
            }
            3 => {
                site.seeing_arcsec = input::ask_positive("New seeing FWHM, arcsec", Some(site.seeing_arcsec));
                site.sky_mag_arcsec2 = input::ask_optional_hint(
                    "Sky brightness, mag/arcsec^2",
                    "blank to assume 21.0",
                    false,
                );
                println!("Site updated. All configurations will be re-evaluated.");
            }
            4 => report::print_formulas(),
            5 => {
                configs.clear();
                println!("Cleared.");
            }
            _ => break,
        }
    }
}

fn build_config() -> Config {
    let scopes = presets::telescopes();
    let mut options: Vec<String> = scopes.iter().map(|t| t.name.clone()).collect();
    options.push("Custom telescope (enter specs)".into());
    let i = input::ask_menu("Choose a telescope", &options);
    let telescope = if i < scopes.len() {
        println!("  Source: {}", scopes[i].source);
        scopes[i].clone()
    } else {
        custom_telescope()
    };

    let cams = presets::cameras();
    let mut options: Vec<String> = cams.iter().map(|c| c.name.clone()).collect();
    options.push("Custom camera (enter specs)".into());
    let j = input::ask_menu("Choose a camera", &options);
    let camera = if j < cams.len() {
        println!("  Source: {}", cams[j].source);
        cams[j].clone()
    } else {
        custom_camera()
    };

    let mounts = presets::mounts();
    let mut options: Vec<String> = mounts.iter().map(|m| m.name.clone()).collect();
    options.push("Custom mount (enter specs)".into());
    options.push("No mount / skip".into());
    let k = input::ask_menu("Choose a mount", &options);
    let mount = if k < mounts.len() {
        println!("  Source: {}", mounts[k].source);
        let mut m = mounts[k].clone();
        if m.pointing_rms_arcsec.is_none() {
            m.pointing_rms_arcsec = input::ask_optional("Pointing accuracy after modeling, arcsec RMS", false);
        }
        if m.max_accel_deg_s2.is_none() {
            m.max_accel_deg_s2 = input::ask_optional("Maximum axis acceleration, deg/s^2", false);
        }
        if m.non_sidereal_tracking == Capability::Unknown {
            m.non_sidereal_tracking = ask_capability("Can its software track a satellite from a TLE (non-sidereal)?");
        }
        Some(m)
    } else if k == mounts.len() {
        Some(custom_mount())
    } else {
        None
    };

    println!("\nPractical-fit inputs. Leave blank anything you don't know.");
    let payload = Payload {
        mount,
        accessories_lb: input::ask_nonnegative(
            "Accessories weight (focuser, dew heaters, cables, filters), lb",
            0.0,
        ),
        back_focus_required_mm: input::ask_optional("Back focus your camera train needs, mm", false),
    };

    println!("\nImage timestamp accuracy. Roughly 20 ms for a PC clock plus USB latency,");
    println!("0.1 ms or better for GPS hardware timestamping.");
    let timestamp_accuracy_ms = input::ask_positive("Timestamp accuracy, ms", Some(DEFAULT_TIMESTAMP_MS));

    println!("\nDetection inputs. Leave both blank to use values derived per regime:");
    println!("a 10 m^2 target at 0.2 albedo and full phase, exposed until its trail");
    println!("reaches one seeing disk (capped at 30 s).");
    let target_mag_override = input::ask_optional_hint(
        "Target apparent magnitude",
        "blank for the derived value",
        false,
    );
    let exposure_override_s = input::ask_optional_hint(
        "Exposure time, s",
        "blank for the trail-limited value",
        false,
    );

    let default_label = format!("{} + {}", short_name(&telescope.name), short_name(&camera.name));
    let label = input::ask_text("Label for this configuration", &default_label);
    Config {
        label,
        telescope,
        camera,
        payload,
        timestamp_accuracy_ms,
        target_mag_override,
        exposure_override_s,
    }
}

fn ask_capability(prompt: &str) -> Capability {
    match input::ask_menu(prompt, &["Yes".to_string(), "No".to_string(), "Not sure".to_string()]) {
        0 => Capability::Yes,
        1 => Capability::No,
        _ => Capability::Unknown,
    }
}

fn custom_mount() -> Mount {
    println!("\nEnter mount specs from the spec sheet.");
    let name = input::ask_text("Mount name", "Custom mount");
    let mount_type = match input::ask_menu(
        "Mount configuration?",
        &["Alt-azimuth".to_string(), "Equatorial".to_string(), "Not sure".to_string()],
    ) {
        0 => MountType::AltAz,
        1 => MountType::Equatorial,
        _ => MountType::Unknown,
    };
    Mount {
        name,
        mount_type,
        capacity_lb: input::ask_optional("Rated payload, lb", false),
        max_slew_deg_s: input::ask_optional("Maximum slew rate, deg/s", false),
        max_accel_deg_s2: input::ask_optional("Maximum axis acceleration, deg/s^2", false),
        settle_time_s: input::ask_optional_hint("Settle time after a slew, s", "blank to assume 2.0", true),
        pointing_rms_arcsec: input::ask_optional("Pointing accuracy after modeling, arcsec RMS", false),
        non_sidereal_tracking: ask_capability("Can its software track a satellite from a TLE (non-sidereal)?"),
        source: "User-entered".into(),
    }
}

/// Text before the first " (", without a leading vendor name, so labels stay short.
fn short_name(name: &str) -> String {
    let base = name.split(" (").next().unwrap_or(name).trim();
    for vendor in ["PlaneWave ", "Celestron ", "Sony "] {
        if let Some(rest) = base.strip_prefix(vendor) {
            return rest.to_string();
        }
    }
    base.to_string()
}

fn custom_telescope() -> Telescope {
    println!("\nEnter telescope specs from the spec sheet.");
    let name = input::ask_text("Telescope name", "Custom telescope");
    let aperture_mm = input::ask_positive("Clear aperture D, mm", None);
    let focal_length_mm = match input::ask_optional_hint("Focal length, mm", "blank to enter the f-ratio instead", false) {
        Some(fl) => fl,
        None => aperture_mm * input::ask_positive("f-ratio N", None),
    };
    let kind = input::ask_menu(
        "How does the spec sheet quote the central obstruction?",
        &[
            "Percent of diameter (most common)".to_string(),
            "Percent of area".to_string(),
            "No central obstruction (refractor)".to_string(),
        ],
    );
    let obstruction = match kind {
        0 => Obstruction::ByDiameter(input::ask_fraction("Obstruction, % of diameter")),
        1 => Obstruction::ByArea(input::ask_fraction("Obstruction, % of area")),
        _ => Obstruction::None,
    };
    let image_circle_mm = input::ask_positive("Corrected image circle diameter, mm", None);
    let back_focus_mm = input::ask_optional("Back focus available, mm", false);
    let weight_lb = input::ask_optional("Optical tube weight, lb", false);
    let throughput = input::ask_optional_hint(
        "Optical throughput (fraction, e.g. 0.85)",
        "blank to assume 0.85",
        false,
    );

    let spot = if input::ask_yes_no("Enter RMS spot sizes from the spec sheet?", false) {
        let conv = input::ask_menu(
            "How is the RMS spot size quoted?",
            &[
                "RMS radius".to_string(),
                "RMS diameter".to_string(),
                "Not stated / unsure (evaluate both)".to_string(),
            ],
        );
        let convention = match conv {
            0 => SpotConvention::RmsRadius,
            1 => SpotConvention::RmsDiameter,
            _ => SpotConvention::Unknown,
        };
        let mut points = Vec::new();
        println!("Enter one point per field position. 0 mm = on-axis.");
        while let Some(r) = input::ask_optional_hint("Field radius, mm off-axis", "blank when done", true) {
            let rms = input::ask_positive("  RMS spot at that radius, um", None);
            points.push(SpotPoint { field_radius_mm: r, rms_um: rms });
        }
        if points.is_empty() {
            None
        } else {
            Some(SpotSpec::new(convention, points))
        }
    } else {
        None
    };

    Telescope {
        name,
        aperture_mm,
        focal_length_mm,
        obstruction,
        image_circle_mm,
        back_focus_mm,
        weight_lb,
        throughput,
        spot,
        source: "User-entered".into(),
    }
}

fn custom_camera() -> Camera {
    println!("\nEnter camera specs from the spec sheet.");
    let name = input::ask_text("Camera name", "Custom camera");
    let pixel_um = input::ask_positive("Pixel size, um", None);
    let width_px = input::ask_count("Sensor width, pixels");
    let height_px = input::ask_count("Sensor height, pixels");
    let read_noise_e = input::ask_optional("Read noise, e- RMS", false);
    let qe = input::ask_optional_hint("Peak quantum efficiency (fraction, e.g. 0.80)", "blank to assume 0.80", false);
    let shutter = match input::ask_menu(
        "Shutter type?",
        &["Rolling shutter (most CMOS)".to_string(), "Global shutter".to_string()],
    ) {
        0 => Shutter::Rolling { line_time_us: input::ask_optional("Line time (row-to-row delay), us", false) },
        _ => Shutter::Global,
    };
    let weight_lb = input::ask_optional("Camera weight, lb", false);
    Camera {
        name,
        pixel_um,
        width_px,
        height_px,
        read_noise_e,
        qe,
        shutter,
        weight_lb,
        source: "User-entered".into(),
    }
}

/// Non-interactive comparison of the presets, useful as a worked example.
fn run_demo() {
    let site = Site {
        seeing_arcsec: DEFAULT_SEEING_ARCSEC,
        wavelength_um: DEFAULT_WAVELENGTH_UM,
        sky_mag_arcsec2: None,
    };
    let scopes = presets::telescopes();
    let cams = presets::cameras();
    let find = |s: &str| scopes.iter().find(|t| t.name.contains(s)).unwrap().clone();
    let imx455 = cams[0].clone();
    let imx461 = cams[2].clone();
    let imx174 = cams[3].clone();
    let all_mounts = presets::mounts();
    let mount = |s: &str| {
        let mut m = all_mounts.iter().find(|m| m.name.contains(s)).unwrap().clone();
        // Demo assumptions: a modeled mount points to ~30" RMS. TLE tracking left as "unknown"
        // so the demo shows the vendor question it raises.
        m.pointing_rms_arcsec = Some(DEMO_ASSUMED_POINTING_RMS_ARCSEC);
        m
    };
    let payload = |m: Mount, acc: f64| Payload { mount: Some(m), accessories_lb: acc, back_focus_required_mm: None };

    let cfg = |label: &str, telescope: Telescope, camera: Camera, payload: Payload| Config {
        label: label.into(),
        telescope,
        camera,
        payload,
        timestamp_accuracy_ms: GPS_TIMESTAMP_MS,
        target_mag_override: None,
        exposure_override_s: None,
    };

    let configs = vec![
        cfg("DeltaRho 350 + IMX455", find("DeltaRho 350"), imx455.clone(), payload(mount("L-350"), 10.0)),
        cfg("RASA 11 + IMX455", find("RASA 11"), imx455.clone(), payload(mount("HAE69"), 5.0)),
        cfg("CDK14 + IMX455", find("CDK14"), imx455.clone(), payload(mount("L-350"), 10.0)),
        cfg("CDK17 + IMX455", find("CDK17"), imx455.clone(), payload(mount("L-500"), 15.0)),
        cfg("DeltaRho 500 + IMX461", find("DeltaRho 500"), imx461, payload(mount("L-500"), 15.0)),
        cfg("RASA 11 + IMX174 (global)", find("RASA 11"), imx174, payload(mount("L-350"), 5.0)),
    ];

    println!("scope-eval demo: built-in presets, seeing {:.1}\"", site.seeing_arcsec);
    println!("No QE, throughput, sky brightness or mount dynamics are entered, so");
    println!("detection is capped at WARN throughout. That is the point of the demo.");
    let evals = checks::evaluate_all(&configs, &site);
    for (cfg, ev) in configs.iter().zip(evals.iter()) {
        report::print_evaluation(cfg, ev, &site);
    }
    report::print_regime_details(&evals[0]);
    report::print_comparison(&evals);
}
