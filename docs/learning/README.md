# Learning the concepts behind scope-eval

These lessons teach the terms, formulas, algorithms and engineering judgment that `scope-eval` uses. The aim is that you finish able to **work out every number the tool prints by yourself**, and to argue with it when you disagree.

The main [README](../../README.md) covers how to install and run the tool. These pages cover *why* it says what it says.

## How to use these lessons

Every lesson uses the same layout, so you always know where to look:

| Section | What it is for |
|---|---|
| **You will be able to** | The outcomes. Come back after the lesson and check that you can do each one. |
| **Key terms** | Short definitions. Read them first, then come back to them when a term shows up. |
| **Concept sections** | The idea in words, then the formula, then *why it takes that shape*, then a worked example. |
| **Validate it yourself** | Ways to confirm the result without trusting this text: a hand calculation, an independent script, the matching Rust unit test and the line in the tool's output that shows it. |
| **Self-check** | Questions to answer before you open the answers (they are folded under *Answer*). |
| **Where it lives in the code** | The file and function that implement it, if you want to read the source. |

Try each worked example before you read the answer. A calculator is enough for almost all of them.

## Learning path

The lessons build on each other. Lessons 1 and 2 are needed for everything else. After those you can follow the order below, or jump to the topic you need.

| # | Lesson | You will learn | Tool feature |
|---|---|---|---|
| 1 | [Angles and magnitudes](01-angles-and-magnitudes.md) | Arcseconds, radians, the small-angle rule, the logarithmic magnitude scale | The units in every check |
| 2 | [Seeing and sampling](02-seeing-and-sampling.md) | Seeing, FWHM, plate scale, sampling, binning, read noise, ideal pixel size | Checks 2 and 3 |
| 3 | [Optics and focus](03-optics-and-focus.md) | Image circle, RMS spot vs FWHM, blurs added in quadrature, critical focus zone | Checks 1, 4 and 7 |
| 4 | [Light collection and search speed](04-light-collection-and-search.md) | Central obstruction, effective area, depth, field of view, etendue | Checks 5 and 6 |
| 5 | [Practical fit](05-practical-fit.md) | Payload margin, back focus, how to read a spec sheet | Check 8 |
| 6 | [Orbits and angular rates](06-orbits-and-angular-rates.md) | Orbital regimes, circular speed, vis-viva, rate vs stars vs ground, acquisition | Regime telescope checks |
| 7 | [Timing and shutters](07-timing-and-shutters.md) | Sidereal rate, timestamp error, rolling-shutter skew, trailing | Regime camera checks, GEO timing reference |
| 8 | [Mount dynamics](08-mount-dynamics.md) | Tracking modes, keyholes, peak acceleration, slew and settle | Regime mount checks |
| 9 | [Brightness and detection](09-brightness-and-detection.md) | Target magnitude, photon flux, sky background, SNR, limiting magnitude | Regime system (detection) check |
| 10 | [Pass prediction](10-pass-prediction.md) | TLEs, SGP4, Keplerian orbits with J2, topocentric geometry, lighting | Pass-prediction menu |
| 11 | [How the tool judges](11-how-the-tool-judges.md) | The evaluation pipeline, PASS/WARN/FAIL, reference configuration, rules of thumb | Reports and comparison table |

## The running example

Unless a lesson says otherwise, every worked example uses the same configuration, so you can carry numbers from one lesson to the next:

* **Telescope:** PlaneWave DeltaRho 350: 350 mm aperture, 1050 mm focal length (f/3), 56% central obstruction by diameter, 60 mm image circle.
* **Camera:** Sony IMX455 full-frame sensor: 9576 x 6388 pixels of 3.76 um (36.0 x 24.0 mm), rolling shutter at 39.028 us per row.
* **Mount:** PlaneWave L-350, 100 lb payload, 50 deg/s maximum slew.
* **Site:** 2.5" seeing.

The tool's `--demo` run evaluates this configuration first, so every number in the lessons can also be found in its output:

```bash
cargo run --release -- --demo | less
```

## Three ways to check a number

You don't have to take any number in these lessons on trust.

1. **By hand.** Every formula is given in full, with its units. Each worked example shows every step.
2. **With an independent script.** [`check_examples.py`](check_examples.py) recomputes every worked example using only Python's standard library. It shares no code with the Rust tool. Run it from the repository root:

   ```bash
   python3 docs/learning/check_examples.py
   ```

   Each line shows the value it computed next to the value quoted in the lesson, and the script exits with an error if any of them disagree. Edit it to try your own numbers.
3. **With the tool's own tests.** Each lesson names the Rust unit tests that assert the same examples. Run a single test by name, for example:

   ```bash
   cargo test plate_scale_deltarho350_imx455
   cargo test --workspace            # everything, including the orbit-prop library
   ```

If the three disagree, one of them has a bug, and that is worth knowing.
