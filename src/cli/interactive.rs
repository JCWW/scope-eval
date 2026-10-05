//! The interactive menu: the main loop and pass prediction.

use orbit_prop::{find_passes, Epoch, PassSearch};
use scope_eval::checks;
use scope_eval::constants::{DEFAULT_MIN_ELEVATION_DEG, DEFAULT_SEEING_ARCSEC, DEFAULT_WAVELENGTH_UM, MAX_PASS_SEARCH_HOURS};
use scope_eval::model::{Config, Site};
use scope_eval::passes::build_propagator;
use scope_eval::report::{ComparisonReport, EvaluationReport, PassTable, RegimeDetails, FORMULAS};

use super::input;
use super::prompts;

pub fn run_interactive() {
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
        location: None,
    };
    let mut configs: Vec<Config> = Vec::new();

    loop {
        let menu = [
            "Evaluate a telescope + camera configuration".to_string(),
            "Compare all evaluated configurations".to_string(),
            "Show detailed orbital-regime evaluation for a configuration".to_string(),
            format!(
                "Change site conditions (seeing {:.2}\", sky {}, {})",
                site.seeing_arcsec,
                match site.sky_mag_arcsec2 {
                    Some(s) => format!("{s:.2}"),
                    None => "assumed".to_string(),
                },
                describe_location(&site)
            ),
            "Predict passes for a satellite".to_string(),
            "Show formula summary".to_string(),
            "Remove all configurations and start over".to_string(),
            "Quit".to_string(),
        ];
        match input::ask_menu("Main menu", &menu) {
            0 => {
                let cfg = prompts::build_config();
                configs.push(cfg);
                let evals = checks::evaluate_all(&configs, &site);
                let last = configs.len() - 1;
                print!("{}", EvaluationReport(&configs[last], &evals[last], &site));
            }
            1 => print!("{}", ComparisonReport(&checks::evaluate_all(&configs, &site))),
            2 => {
                if configs.is_empty() {
                    println!("\nNo configurations evaluated yet.");
                } else {
                    let labels: Vec<String> = configs.iter().map(|c| c.label.clone()).collect();
                    let k = input::ask_menu("Which configuration?", &labels);
                    let evals = checks::evaluate_all(&configs, &site);
                    print!("{}", RegimeDetails(&evals[k]));
                }
            }
            3 => {
                site.seeing_arcsec = input::ask_positive("New seeing FWHM, arcsec", Some(site.seeing_arcsec));
                let answer = input::ask_optional_hint(
                    "Sky brightness, mag/arcsec^2",
                    match site.sky_mag_arcsec2 {
                        Some(_) => "blank to keep the current value",
                        None => "blank to assume 21.0",
                    },
                    false,
                );
                site.sky_mag_arcsec2 = input::resolve_keep(answer, site.sky_mag_arcsec2);
                site.location = prompts::ask_location(site.location);
                println!("Site updated. All configurations will be re-evaluated.");
            }
            4 => run_pass_prediction(&mut site, &configs),
            5 => println!("{FORMULAS}"),
            6 => {
                configs.clear();
                println!("Cleared.");
            }
            _ => break,
        }
    }
}

fn describe_location(site: &Site) -> String {
    match site.location {
        Some(l) => format!("site {:.4}, {:.4}, {:.0} m", l.lat_deg, l.lon_deg, l.alt_m),
        None => "location not set".to_string(),
    }
}

/// Menu item: predict passes over the site and judge each evaluated mount.
fn run_pass_prediction(site: &mut Site, configs: &[Config]) {
    if site.location.is_none() {
        println!("\nPass prediction needs the site's location.");
        while site.location.is_none() {
            site.location = prompts::ask_location(None);
        }
    }
    let Some(location) = site.location else { return };
    let source = prompts::ask_orbit_source();
    let start = prompts::ask_epoch("Search start, UTC YYYY-MM-DDTHH:MM", "blank for now").unwrap_or_else(Epoch::now);
    let hours = loop {
        let h = input::ask_positive("Search length, hours", Some(24.0));
        if h <= MAX_PASS_SEARCH_HOURS {
            break h;
        }
        println!("  The longest search is {MAX_PASS_SEARCH_HOURS:.0} hours.");
    };
    let min_el_deg = loop {
        let e = input::ask_nonnegative("Minimum elevation, deg", DEFAULT_MIN_ELEVATION_DEG);
        if e < 90.0 {
            break e;
        }
        println!("  Enter an elevation below 90 degrees.");
    };
    let end = start.add_seconds(hours * 3600.0);
    let prop = match build_propagator(source, start, end) {
        Ok((p, note)) => {
            if let Some(note) = note {
                println!("\n  {note}");
            }
            p
        }
        Err(e) => {
            println!("\n  {e}");
            return;
        }
    };
    let search = PassSearch { start, end, min_el_deg };
    println!("\nSearching {hours:.0} h from {start} ...");
    let result = find_passes(prop.as_ref(), &location, &search);
    print!("{}", PassTable(prop.label(), &result, configs));
}
