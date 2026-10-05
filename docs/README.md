# scope-eval documentation

These pages teach the ideas behind scope-eval one step at a time: the terms, the formulas, the algorithm that strings them together, and the thresholds that turn numbers into PASS, WARN and FAIL. You don't need an optics or orbital-mechanics background. Each page builds on the ones before it.

Every page ends with a **Check it yourself** section. Its exercises let you confirm each idea independently of this text, in three ways:

1. **By hand.** Redo a calculation with a pocket calculator and compare with the answer given.
2. **Against the tests.** Run the named unit test (`cargo test <name>`), which encodes the same worked example in code.
3. **In the tool.** Run `cargo run --release -- --demo` or the interactive menu and find the same number in the report.

If all three agree, you understand the concept and the tool computes it the way the docs say.

## Learning path

Read these in order the first time. Later, jump straight to the page you need.

| # | Page | What you'll learn |
|---|---|---|
| 1 | [Getting started](01-getting-started.md) | Build, run and read a first report |
| 2 | [Key terms](02-key-terms.md) | Arcseconds, seeing, focal ratio, magnitudes, orbital regimes and the rest of the vocabulary |
| 3 | [Inputs](03-inputs.md) | Every number the tool asks for and where to find it on a spec sheet |
| 4 | [How an evaluation works](04-how-an-evaluation-works.md) | The fixed pipeline, the reference configuration and what each status means |
| 5 | [Image-quality checks (1 to 4)](05-image-quality-checks.md) | Sensor fit, sampling, ideal pixel size and optics vs seeing |
| 6 | [Light, field, focus and fit checks (5 to 8)](06-light-field-focus-fit-checks.md) | Collecting area, search speed, focus tolerance and payload |
| 7 | [Motion and timing](07-motion-and-timing.md) | Sidereal drift, timestamp error and rolling-shutter skew, using GEO stare mode |
| 8 | [Orbital regimes](08-orbital-regimes.md) | How target rates are derived and how telescope, camera and mount are graded per regime |
| 9 | [Mount dynamics](09-mount-dynamics.md) | Peak acceleration, the keyhole and slew-and-settle time |
| 10 | [Target brightness and detection](10-target-brightness-and-detection.md) | Apparent magnitude, photon counts, signal-to-noise and limiting magnitude |
| 11 | [Pass prediction](11-pass-prediction.md) | Real passes over your site and whether a mount can follow each one |
| 12 | [Comparing configurations](12-comparing-configurations.md) | Reading the comparison tables and choosing by mission |

## Reference

| Page | Contents |
|---|---|
| [Thresholds](13-thresholds.md) | Every tunable limit, its default and where it lives |
| [Assumptions, limitations and red flags](14-assumptions-and-red-flags.md) | What the models leave out, and spec-sheet traps |
| [Presets](15-presets.md) | Built-in hardware and where its numbers came from |
| [Code structure](16-code-structure.md) | Where each calculation lives and how to extend the tool |

The orbit propagation library has its own documentation in [`crates/orbit-prop/README.md`](../crates/orbit-prop/README.md).

## The running example

Most worked examples use one configuration, so you can follow a single set of numbers from page to page:

* **Telescope:** PlaneWave DeltaRho 350. 350 mm aperture, 1050 mm focal length (f/3), 56% central obstruction by diameter, 60 mm image circle, 46 lb.
* **Camera:** Sony IMX455 full-frame sensor. 9576 x 6388 pixels of 3.76 um (36.0 x 24.0 mm), rolling shutter at 39.028 us per line, 2 lb.
* **Mount:** PlaneWave L-350. Alt-azimuth, 100 lb payload, 50 deg/s maximum slew.
* **Site:** 2.5 arcseconds of seeing.

Comparison examples use the Celestron RASA 11 (279 mm, 620 mm focal length) and the PlaneWave CDK14 (356 mm, 2563 mm focal length) with the same camera.
