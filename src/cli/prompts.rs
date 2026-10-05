//! Prompts that build configurations, sites and orbits from terminal input.

use orbit_prop::{Epoch, GroundSite, Tle};
use scope_eval::constants::DEFAULT_TIMESTAMP_MS;
use scope_eval::model::{
    Camera, Capability, Config, Mount, MountType, Obstruction, Payload, Shutter, SpotConvention, SpotPoint, SpotSpec,
    Telescope,
};
use scope_eval::passes::OrbitSource;
use scope_eval::presets;

use super::input;

/// Ask for the site's location. A blank latitude keeps `current`.
pub(super) fn ask_location(current: Option<GroundSite>) -> Option<GroundSite> {
    let hint = if current.is_some() { "blank to keep the current location" } else { "blank to skip" };
    loop {
        let Some(lat) = input::ask_optional_signed("Site latitude, deg (north positive)", hint) else {
            return current;
        };
        let lon = loop {
            if let Some(v) = input::ask_optional_signed("Site longitude, deg (east positive)", "required") {
                break v;
            }
            println!("  A longitude is required.");
        };
        let alt = input::ask_optional_signed("Site altitude, m", "blank for 0").unwrap_or(0.0);
        match GroundSite::new(lat, lon, alt) {
            Ok(s) => return Some(s),
            Err(e) => println!("  {e}"),
        }
    }
}

/// An optional UTC time. Blank returns `None`.
pub(super) fn ask_epoch(prompt: &str, hint: &str) -> Option<Epoch> {
    loop {
        let s = input::read_line(&format!("{prompt} ({hint}): "));
        if s.is_empty() {
            return None;
        }
        match Epoch::parse_iso8601(&s) {
            Ok(t) => return Some(t),
            Err(e) => println!("  {e}"),
        }
    }
}

/// Read a pasted TLE line by line until it parses.
fn ask_tle() -> Tle {
    println!("Paste the TLE (2 lines, or 3 with a name line first):");
    let mut lines: Vec<String> = Vec::new();
    loop {
        let line = input::read_line("> ");
        if line.is_empty() {
            continue;
        }
        let is_line2 = line.starts_with("2 ");
        lines.push(line);
        if is_line2 {
            match Tle::parse(&lines.join("\n")) {
                Ok(t) => return t,
                Err(e) => println!("  {e}\n  Please paste the TLE again."),
            }
            lines.clear();
        } else if lines.len() > 2 {
            println!("  That doesn't look like a TLE. Please paste it again.");
            lines.clear();
        }
    }
}

pub(super) fn ask_orbit_source() -> OrbitSource {
    let options = ["Paste a TLE".to_string(), "Define a what-if orbit".to_string()];
    if input::ask_menu("Orbit source", &options) == 0 {
        return OrbitSource::Tle(ask_tle());
    }
    let perigee_km = input::ask_positive("Perigee altitude, km", None);
    let apogee_km = loop {
        let a = input::ask_positive("Apogee altitude, km", Some(perigee_km));
        if a >= perigee_km {
            break a;
        }
        println!("  Apogee must be at least the perigee altitude.");
    };
    let i_deg = loop {
        let i = input::ask_nonnegative("Inclination, deg", 0.0);
        if i <= 180.0 {
            break i;
        }
        println!("  Inclination must be between 0 and 180 degrees.");
    };
    OrbitSource::WhatIf {
        perigee_km,
        apogee_km,
        i_deg,
        raan_deg: input::ask_nonnegative("Right ascension of the ascending node, deg", 0.0),
        argp_deg: input::ask_nonnegative("Argument of perigee, deg", 0.0),
        mean_anomaly_deg: input::ask_nonnegative("Mean anomaly at epoch, deg", 0.0),
        epoch: ask_epoch("Element epoch, UTC YYYY-MM-DDTHH:MM", "blank for the search start"),
    }
}

pub(super) fn build_config() -> Config {
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
    let target_mag_override = input::ask_optional_signed(
        "Target apparent magnitude",
        "blank for the derived value; negative is brighter",
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
