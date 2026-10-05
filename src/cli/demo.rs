//! `--demo`: evaluate the presets and print the reports, without prompting.

use scope_eval::checks;
use scope_eval::constants::{DEFAULT_SEEING_ARCSEC, DEFAULT_WAVELENGTH_UM, DEMO_ASSUMED_POINTING_RMS_ARCSEC, GPS_TIMESTAMP_MS};
use scope_eval::model::{Camera, Config, Mount, Payload, Site, Telescope};
use scope_eval::presets;
use scope_eval::report::{ComparisonReport, EvaluationReport, RegimeDetails};

/// Non-interactive comparison of the presets, useful as a worked example.
pub fn run_demo() {
    let site = Site {
        seeing_arcsec: DEFAULT_SEEING_ARCSEC,
        wavelength_um: DEFAULT_WAVELENGTH_UM,
        sky_mag_arcsec2: None,
        location: None,
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
        print!("{}", EvaluationReport(cfg, ev, &site));
    }
    print!("{}", RegimeDetails(&evals[0]));
    print!("{}", ComparisonReport(&evals));
}
