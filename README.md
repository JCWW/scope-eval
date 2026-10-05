# scope-eval

An interactive command-line calculator, written in Rust, that evaluates a **telescope + camera + mount** configuration for ground-based satellite observation. It was built for engineers who need to choose optical hardware for space domain awareness but whose background is software rather than optics.

The tool works in two layers:

1. **Eight general checks** on the optical system: sensor fit, sampling, ideal pixel size, optics vs seeing, collecting area, field and search speed, focus tolerance and practical fit. These don't depend on what you are looking at.
2. **Orbital-regime evaluations.** The telescope, camera, mount and overall detection system are each graded against five orbital regimes: low Earth orbit (LEO), medium Earth orbit (MEO), geosynchronous orbit (GEO), highly elliptical orbit (HEO) and cislunar space. A configuration that is excellent for GEO can be unusable for LEO, and these checks show which component is the reason.

You pick hardware from built-in presets or type in numbers from any spec sheet. Every result is graded PASS, WARN, FAIL or INFO with a plain-English explanation. Evaluate several configurations and the tool prints a side-by-side comparison. It can also predict real passes of a satellite over your site and judge whether each mount can follow them.

The tool has three external dependencies, all pure Rust: `serde` and `serde_yaml` read `presets.yaml`, and `sgp4` propagates satellite orbits inside the bundled `orbit-prop` library (`crates/orbit-prop`). The code is small enough to audit.

## Quick start

You need a Rust toolchain, version 1.70 or newer (tested with 1.75). Install it from <https://rustup.rs> if you don't have it.

```bash
cargo build --release          # build
cargo run --release            # interactive menu
cargo run --release -- --demo  # evaluate the presets, show one full regime breakdown, compare all
cargo run --release -- --help  # usage
cargo test                     # run the worked examples from the docs, and the orbit-prop tests
```

[Getting started](docs/01-getting-started.md) walks through a first interactive session.

## Documentation

The [docs/](docs/README.md) folder teaches the terms, formulas, algorithms and thresholds behind every number the tool prints, one step at a time. Each page ends with a **Check it yourself** section: hand calculations with answers, the unit tests that encode the same examples, and where to find each number in the tool's output.

| # | Page | Covers |
|---|---|---|
| 1 | [Getting started](docs/01-getting-started.md) | Build, run and read a first report |
| 2 | [Key terms](docs/02-key-terms.md) | The vocabulary used everywhere else |
| 3 | [Inputs](docs/03-inputs.md) | Every input and where to find it on a spec sheet |
| 4 | [How an evaluation works](docs/04-how-an-evaluation-works.md) | The pipeline, the reference configuration and the statuses |
| 5 | [Image-quality checks](docs/05-image-quality-checks.md) | Checks 1 to 4: sensor fit, sampling, ideal pixel, optics vs seeing |
| 6 | [Light, field, focus and fit checks](docs/06-light-field-focus-fit-checks.md) | Checks 5 to 8: area, search speed, focus tolerance, payload |
| 7 | [Motion and timing](docs/07-motion-and-timing.md) | Sidereal drift, timing error and rolling-shutter skew |
| 8 | [Orbital regimes](docs/08-orbital-regimes.md) | Regime rates and the telescope, camera and mount checks |
| 9 | [Mount dynamics](docs/09-mount-dynamics.md) | Acceleration, the keyhole and slew-and-settle |
| 10 | [Target brightness and detection](docs/10-target-brightness-and-detection.md) | Magnitudes, signal-to-noise and limiting magnitude |
| 11 | [Pass prediction](docs/11-pass-prediction.md) | Real passes over your site |
| 12 | [Comparing configurations](docs/12-comparing-configurations.md) | The comparison tables and choosing by mission |
| 13 | [Thresholds](docs/13-thresholds.md) | Every tunable limit and default |
| 14 | [Assumptions and red flags](docs/14-assumptions-and-red-flags.md) | Model limits and spec-sheet traps |
| 15 | [Presets](docs/15-presets.md) | Built-in hardware and its sources |
| 16 | [Code structure](docs/16-code-structure.md) | Where things live and how to extend the tool |

The orbit propagation library is documented in [crates/orbit-prop/README.md](crates/orbit-prop/README.md).

## License

MIT.
