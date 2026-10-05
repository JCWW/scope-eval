# 1. Getting started

**What you'll learn:** how to build scope-eval, run it, and recognize the parts of a report. Later pages explain where every number comes from.

## What the tool is for

scope-eval is an interactive command-line calculator, written in Rust, that evaluates a **telescope + camera + mount** configuration for ground-based satellite observation. It was built for engineers choosing optical hardware for space domain awareness whose background is software rather than optics.

It works in two layers:

1. **Eight general checks** on the optical system: sensor fit, sampling, ideal pixel size, optics vs seeing, collecting area, field and search speed, focus tolerance and practical fit. These don't depend on what you are looking at.
2. **Orbital-regime evaluations.** The telescope, camera, mount and overall detection system are each graded against five orbital regimes: low Earth orbit (LEO), medium Earth orbit (MEO), geosynchronous orbit (GEO), highly elliptical orbit (HEO) and cislunar space. A configuration that is excellent for GEO can be unusable for LEO, and these checks show which component is the reason.

You pick hardware from built-in presets or type in numbers from any spec sheet. Every result is graded PASS, WARN, FAIL or INFO with a plain-English explanation. Evaluate several configurations and the tool prints a side-by-side comparison.

The tool has three external dependencies, all pure Rust: `serde` and `serde_yaml` read `presets.yaml`, and `sgp4` propagates satellite orbits inside the bundled `orbit-prop` library (`crates/orbit-prop`). The code is small enough to audit.

## Build and run

You need a Rust toolchain, version 1.70 or newer (tested with 1.75). Install it from <https://rustup.rs> if you don't have it.

```bash
cargo build --release          # build
cargo run --release            # interactive menu
cargo run --release -- --demo  # evaluate the presets, show one full regime breakdown, compare all
cargo run --release -- --help  # usage
cargo test                     # run the worked examples from these docs, and the orbit-prop tests
```

## A typical interactive session

1. Enter your site's typical seeing (press Enter to accept the 2.5 arcsecond default), then its sky brightness (press Enter to assume 21.0 mag/arcsec^2).
2. Choose **Evaluate a telescope + camera configuration**.
3. Pick a telescope, a camera and a mount, or choose *Custom* and type in spec-sheet values. For preset mounts the tool asks for anything the preset doesn't know (pointing accuracy, axis acceleration, satellite-tracking support).
4. Enter accessory weight, back-focus requirement and how accurately your images are timestamped. Leave anything you don't know blank.
5. Read the report: the eight checks, then a compact table of telescope, camera, mount and system status for each orbital regime.
6. Choose **Show detailed orbital-regime evaluation** to see every regime check with its numbers.
7. Repeat for other configurations, then choose **Compare all evaluated configurations** for the side-by-side tables.

The **first configuration you evaluate becomes the reference**. Depth and search speed for every later configuration are reported relative to it ([page 4](04-how-an-evaluation-works.md) explains why).

Output is plain ASCII (for example `um` for micrometers and `"` for arcseconds) so it displays correctly in any terminal, including older Windows consoles.

## Check it yourself

1. Run `cargo test`. Every test should pass. Many of those tests are the worked examples in these docs, so a passing run means the formulas in the code still produce the numbers printed here.
2. Run `cargo run --release -- --demo` and find the line for **DeltaRho 350 + IMX455** in the comparison table near the end. It should read a plate scale of `0.74`, a recommended bin of `2x2` and a field of `2.58` square degrees. Pages 5 and 6 derive each of those by hand.
3. Run the interactive menu, evaluate the DeltaRho 350 with the IMX455, and change only the seeing (menu: **Change site conditions**) from 2.5 to 1.5. The sampling check (check 2) should drop from about 4.0 to about 3.0 pixels across a star and move from WARN to PASS. It doesn't fall in proportion to the seeing, because the telescope's own blur stays the same while the seeing shrinks. [Page 5](05-image-quality-checks.md) and [page 17](17-point-spread-function.md) explain why.

Next: [Key terms](02-key-terms.md).
