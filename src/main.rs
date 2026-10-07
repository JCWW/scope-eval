//! scope-eval: interactive telescope + camera evaluation calculator.
//!
//! This binary is only the command-line front end: it reads arguments and
//! terminal input and prints reports. The evaluation and the report text
//! come from the `scope_eval` library (src/lib.rs).
//!
//! Run with no arguments for the interactive menu, `--demo` for a canned
//! comparison of the presets, or `--help` for usage. See docs/README.md for the
//! full explanation of every calculation.

mod cli;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return;
    }
    if args.iter().any(|a| a == "--demo") {
        cli::run_demo();
        return;
    }
    cli::run_interactive();
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
accuracy, shutter skew, acquisition, whether the target is bright enough
to detect and whether it is so bright it saturates.

The interactive menu can also predict passes of a satellite (from a TLE or
a what-if orbit) over your site, and judge whether each evaluated mount can
follow each pass.

USAGE:
  scope-eval            interactive menu
  scope-eval --demo     evaluate the built-in presets and print a comparison
  scope-eval --help     this message

See docs/README.md for the formulas and the algorithm.",
        env!("CARGO_PKG_VERSION")
    );
}
