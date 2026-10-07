# scope-eval

An interactive command-line calculator, written in Rust, that evaluates a **telescope + camera + mount** configuration for ground-based satellite observation. It was built for engineers who need to choose optical hardware for space domain awareness but whose background is software rather than optics.

The tool works in two layers:

1. **Eight general checks** on the optical system: sensor fit, sampling, ideal pixel size, optics vs seeing, collecting area, field and search speed, focus tolerance and practical fit. These don't depend on what you are looking at.
2. **Orbital-regime evaluations.** The telescope, camera, mount and overall detection system are each graded against five orbital regimes: low Earth orbit (LEO), medium Earth orbit (MEO), geosynchronous orbit (GEO), highly elliptical orbit (HEO) and cislunar space. A configuration that is excellent for GEO can be unusable for LEO, and these checks show which component is the reason.

You pick hardware from built-in presets or type in numbers from any spec sheet. Every result is graded PASS, WARN, FAIL or INFO with a plain-English explanation. Evaluate several configurations and the tool prints a side-by-side comparison. It can also predict real passes of a satellite over your site and judge whether each mount can follow them.

The tool has three external dependencies, all pure Rust: `serde` and `serde_yaml` read `presets.yaml`, and `sgp4` propagates satellite orbits inside the bundled `orbit-prop` library (`crates/orbit-prop`). The code is small enough to audit.

---

## Contents

1. [Quick start](#quick-start)
2. [Learning the concepts](#learning-the-concepts)
3. [Inputs and where to find them](#inputs-and-where-to-find-them)
4. [How an evaluation works](#how-an-evaluation-works)
5. [The eight checks](#the-eight-checks)
6. [Orbital-regime evaluations](#orbital-regime-evaluations)
7. [Pass prediction](#pass-prediction)
8. [The comparison table](#the-comparison-table)
9. [Thresholds and how to tune them](#thresholds-and-how-to-tune-them)
10. [Assumptions and limitations](#assumptions-and-limitations)
11. [Spec-sheet red flags](#spec-sheet-red-flags)
12. [Presets and their sources](#presets-and-their-sources)
13. [Code structure and extending the tool](#code-structure-and-extending-the-tool)

---

## Quick start

You need a Rust toolchain, version 1.70 or newer (tested with 1.75). Install it from <https://rustup.rs> if you don't have it.

```bash
cargo build --release          # build
cargo run --release            # interactive menu
cargo run --release -- --demo  # evaluate the presets, show one full regime breakdown, compare all
cargo run --release -- --demo --equations  # the same, with every check's equations and rule
cargo run --release -- --help  # usage
cargo test                     # run the worked examples from docs/learning, and the orbit-prop library's tests
```

A typical interactive session:

1. Enter your site's typical seeing (press Enter to accept the 2.5 arcsecond default).
2. Choose **Evaluate a telescope + camera configuration**.
3. Pick a telescope, a camera and a mount, or choose *Custom* and type in spec-sheet values. For preset mounts the tool asks for anything the preset doesn't know (pointing accuracy, satellite-tracking support).
4. Enter accessory weight, back-focus requirement and how accurately your images are timestamped. Leave anything you don't know blank.
5. Read the report: the eight checks, then a compact table of telescope, camera and mount status for each orbital regime.
6. Choose **Show detailed orbital-regime evaluation** to see every regime check with its numbers.
7. Repeat for other configurations, then choose **Compare all evaluated configurations** for the side-by-side tables.

The **first configuration you evaluate becomes the reference**. Depth and search speed for every later configuration are reported relative to it.

Output is plain ASCII (for example `um` for micrometers and `"` for arcseconds) so it displays correctly in any terminal, including older Windows consoles.

**Simulation dashboard.** [`dashboard/`](dashboard/README.md) is a separate React app that runs a time-stepped simulation of a configuration tracking a real pass. You can start and stop it, watch the pointing error on the sensor and the sky, and switch between configurations mid-pass. It uses the same presets and orbit library as this tool.

---

## Learning the concepts

This README covers how to run the tool and read its output. The terms, formulas, derivations and worked examples behind every check are in a set of lessons in [`docs/learning/`](docs/learning/README.md), written for engineers without an optics or astrodynamics background:

| # | Lesson | Covers |
|---|---|---|
| 1 | [Angles and magnitudes](docs/learning/01-angles-and-magnitudes.md) | Arcseconds, the small-angle rule, the magnitude scale |
| 2 | [Seeing and sampling](docs/learning/02-seeing-and-sampling.md) | Checks 2 and 3: plate scale, binning, CMOS read noise, ideal pixel |
| 3 | [Optics and focus](docs/learning/03-optics-and-focus.md) | Checks 1, 4 and 7: image circle, spot size vs seeing, critical focus zone |
| 4 | [Light collection and search speed](docs/learning/04-light-collection-and-search.md) | Checks 5 and 6: obstruction, effective area, depth, etendue |
| 5 | [Practical fit](docs/learning/05-practical-fit.md) | Check 8: payload and back focus |
| 6 | [Orbits and angular rates](docs/learning/06-orbits-and-angular-rates.md) | The regimes, vis-viva, rate vs stars and ground, acquisition |
| 7 | [Timing and shutters](docs/learning/07-timing-and-shutters.md) | Timestamp accuracy, rolling-shutter skew, trailing |
| 8 | [Mount dynamics](docs/learning/08-mount-dynamics.md) | Tracking rate, keyholes, peak acceleration, slew and settle |
| 9 | [Brightness and detection](docs/learning/09-brightness-and-detection.md) | Target magnitude, photon flux, SNR, limiting magnitude |
| 10 | [Pass prediction](docs/learning/10-pass-prediction.md) | TLEs, SGP4, J2, reference frames, pass search, lighting |
| 11 | [How the tool judges](docs/learning/11-how-the-tool-judges.md) | The pipeline, statuses, and which numbers are physics vs rules of thumb |

Each lesson ends with ways to check its numbers yourself. [`docs/learning/check_examples.py`](docs/learning/check_examples.py) recomputes every worked example using only the Python standard library, independently of the Rust code:

```bash
python3 docs/learning/check_examples.py
```

---

## Inputs and where to find them

| Input | Units | Where it comes from | Used by checks |
|---|---|---|---|
| Seeing FWHM | arcsec | Your site (2.5" is a reasonable default for a mid-elevation suburban site) | 2, 3, 4 |
| Aperture D | mm | Telescope spec sheet | 5, 7 |
| Focal length FL | mm | Spec sheet, or aperture x f-ratio | 2, 3, 4, 6, 7 |
| Central obstruction | % | Spec sheet. **Note whether it is quoted by diameter or by area.** | 5 |
| Image circle | mm | Spec sheet ("corrected field," "image circle") | 1 |
| RMS spot size(s) | um at a field radius (mm) | Spec sheet optical performance section (optional) | 4 |
| Back focus | mm | Spec sheet (optional) | 8 |
| OTA weight | lb | Spec sheet (optional) | 8 |
| Pixel size | um | Camera spec sheet | 2, 3 |
| Sensor width and height | pixels | Camera spec sheet | 1, 6 |
| Read noise | e- RMS | Camera spec sheet (optional, 3 e- assumed if blank) | 2, Regime: detection |
| Shutter type and line time | rolling/global, us | Camera manual (optional) | Supplementary |
| Camera weight | lb | Camera spec sheet (optional) | 8 |
| Mount type | alt-az / equatorial | Mount spec sheet | Regime: mount |
| Mount payload rating | lb | Mount spec sheet (optional) | 8 |
| Mount maximum slew rate | deg/s | Mount spec sheet (optional) | Regime: mount |
| Mount pointing accuracy | arcsec RMS | Mount spec sheet, after a pointing model (optional, 60" assumed if blank) | Regime: telescope |
| Non-sidereal (TLE) tracking | yes / no / unsure | Mount control software documentation | Regime: mount |
| Accessories weight | lb | Your estimate: focuser, dew heaters, cables, filter wheel, dovetail | 8 |
| Back focus required | mm | Sum of your camera train's optical path lengths (optional) | 8 |
| Timestamp accuracy | ms | Your timing chain: roughly 20 ms for a PC clock plus USB latency, 0.1 ms or better for GPS hardware timestamping | Regime: camera |
| Sky background brightness | mag/arcsec^2 | Site measurement or a dark-sky map (optional, 21.0 assumed if blank) | Regime: detection |
| Optical throughput | fraction 0..1 | Vendor, or measured (optional, 0.85 assumed if blank) | Regime: detection |
| Quantum efficiency | fraction 0..1 | Camera QE curve (optional, 0.80 assumed if blank) | Regime: detection |
| Mount maximum acceleration | deg/s^2 | Mount spec sheet (optional) | Regime: acceleration, keyhole, slew |
| Mount settle time | s | Mount spec sheet or measured (optional, 2.0 s assumed if blank) | Regime: slew and settle |
| Target apparent magnitude | mag | Your own catalog (optional, derived per regime if blank) | Regime: detection |
| Exposure time | s | Your choice (optional, trail-limited value if blank) | Regime: detection |

Obstruction can be typed as a percent (56) or a decimal (0.56). The tool treats any value above 1 as a percent.

---

## How an evaluation works

For each configuration the tool runs the eight general checks, prints an ungraded GEO stare-mode timing reference, then evaluates the telescope, camera, mount and system (detection) against each of the five orbital regimes. The regime checks reuse the plate scale, recommended bin, field of view and effective area from the general checks. [Lesson 11](docs/learning/11-how-the-tool-judges.md) walks through the full pipeline.

The **reference** for depth and search speed is always the first configuration in the session. When you change the seeing, every stored configuration is re-evaluated in order, so the reference doesn't change.

**Two report styles.** The standard report gives each check's numbers and verdict. The equations style (`--equations` on the command line, or **Report style** in the menu) prints the same report with, under each check, the equations behind its numbers with the actual values substituted, ending with the rule that turned the result into PASS, WARN or FAIL. A reviewer can redo every step by hand and see which threshold decided the status:

```
[WARN] 2. Sampling (plate scale vs seeing)
       Plate scale (native) ....................... 0.739"/px
       ...
       Equations:
         Plate scale: 206264.806 x (pixel (um) / 1000) / FL (mm) = 206264.806 x (3.7600 / 1000) / 1050.0 = 0.73862"/px
         Star FWHM (center): sqrt(seeing^2 + diffraction^2 + optics^2 + diffusion^2) = sqrt(2.5000^2 + 0.33353^2 + 1.6028^2 + 0^2) = 2.9883"
         Pixels across a star p: star FWHM / plate scale = 2.9883 / 0.73862 = 4.0458
         ...
         Rule: FAIL < 1 <= WARN < 1.5 <= PASS <= 2.5 (well sampled) < PASS <= 4 (mildly oversampled) < WARN <= 6 < FAIL; p = 4.0458 -> WARN
       -> Oversampled. ...
```

The equations are built from the same variables the check uses, so they cannot drift from what was judged, and a test confirms that every check's rule names the status it reported. [`docs/learning/check_equations.py`](docs/learning/check_equations.py) recomputes every equation in a report with Python's own math:

```bash
cargo run --release -- --demo --equations | python3 docs/learning/check_equations.py
```

### What the statuses mean

| Status | Meaning |
|---|---|
| PASS | Meets the rule of thumb for this check. |
| WARN | Usable, but there is a trade-off or a question to resolve with the vendor. |
| FAIL | A real mismatch for this application. |
| INFO | A measurement for comparison, not a pass/fail judgment (checks 5 and 6), or not enough data to judge. |

A component's status in a regime is its worst graded check. INFO never makes a status worse.

The thresholds behind PASS, WARN and FAIL are engineering rules of thumb, not physical laws. They are collected in one place so you can change them (see [Thresholds](#thresholds-and-how-to-tune-them)).

---

## The eight checks

These checks describe the optical system in general. They don't depend on the target.

| # | Check | Question | Graded | Lesson |
|---|---|---|---|---|
| 1 | Sensor fit | Does the sensor's diagonal fit inside the image circle? | PASS / WARN / FAIL | [3](docs/learning/03-optics-and-focus.md) |
| 2 | Sampling | Are there about 2 pixels across a star (seeing plus the telescope's and detector's blur)? Which bin gets closest? | PASS / WARN / FAIL | [2](docs/learning/02-seeing-and-sampling.md) |
| 3 | Ideal pixel | What pixel size does this focal length want, and does the camera match it natively or after binning? | PASS / WARN | [2](docs/learning/02-seeing-and-sampling.md) |
| 4 | Optics vs seeing | Does the optical blur enlarge stars noticeably beyond the seeing, at the center and at the sensor's corner? | PASS / WARN / FAIL (INFO without spot data) | [3](docs/learning/03-optics-and-focus.md) |
| 5 | Area and depth | Effective collecting area after the obstruction, and depth in magnitudes vs the reference | INFO | [4](docs/learning/04-light-collection-and-search.md) |
| 6 | Field and search | True field of view by two methods (exact trigonometric and small-angle), and etendue (search speed) vs the reference | INFO | [4](docs/learning/04-light-collection-and-search.md) |
| 7 | Focus tolerance | How wide is the critical focus zone? | PASS / WARN | [3](docs/learning/03-optics-and-focus.md) |
| 8 | Practical fit | Is the payload within 70% of the mount's rating? Is there enough back focus? | PASS / WARN / FAIL | [5](docs/learning/05-practical-fit.md) |

If your spec sheet doesn't say whether its RMS spot size is a radius or a diameter, choose "not stated". Check 4 then evaluates both readings and returns WARN if they disagree, so you know to ask the vendor.

After the eight checks, every evaluation prints a **timing reference for GEO stare mode**: star drift past a GEO target, position error per 10 ms of timing error, and rolling-shutter skew with the per-row time correction. It is not graded. [Lesson 7](docs/learning/07-timing-and-shutters.md) explains each line.

---

## Orbital-regime evaluations

Whether a system can observe a target depends on where the target is: how far away, how fast it moves and how well its position is predicted. The tool evaluates each component against five representative regimes:

| Regime | Representative case | Range (km) | Rate vs stars ("/s) | Rate vs ground ("/s) | Prediction error (km) | Usual mode |
|---|---|---|---|---|---|---|
| LEO | 500 km circular, overhead pass | 500 | ~3,140 (0.87 deg/s) | ~3,140 | 2 | rate track |
| MEO | GPS-like 20,200 km, overhead | 20,200 | ~40 | ~40 | 2 | rate track |
| GEO | 35,786 km altitude | ~37,000 | 15.04 | 0 | 2 | stare |
| HEO | Molniya near apogee | ~39,800 | ~7.8 | ~7.3 | 5 | rate track |
| Cislunar | Lunar distance | ~384,400 | ~0.55 | ~14.5 | 50 | sidereal |

[Lesson 6](docs/learning/06-orbits-and-angular-rates.md) derives the rates from orbital mechanics. The prediction errors are placeholder assumptions for a reasonably fresh public element set; edit them in `regimes()` in `src/regimes.rs` to match your data.

| Component | Check | Graded | Lesson |
|---|---|---|---|
| Telescope | Acquisition field: does half the field's short side cover prediction error plus mount pointing error? | PASS / WARN / FAIL | [6](docs/learning/06-orbits-and-angular-rates.md) |
| Telescope | Field dwell: how long an untracked target stays in the field | INFO | [6](docs/learning/06-orbits-and-angular-rates.md) |
| Telescope | Depth relevance: area and depth vs reference, and what limits the regime | INFO | [6](docs/learning/06-orbits-and-angular-rates.md) |
| Camera | Timestamp accuracy: is the timing error under a quarter binned pixel of motion? | PASS / WARN / FAIL | [7](docs/learning/07-timing-and-shutters.md) |
| Camera | Shutter skew: how far the target moves during a rolling-shutter readout | PASS / WARN / FAIL | [7](docs/learning/07-timing-and-shutters.md) |
| Camera | Exposure vs trailing: how long before the target or the stars smear | INFO | [7](docs/learning/07-timing-and-shutters.md) |
| Mount | Tracking rate and keyhole: axis-rate headroom, and the highest pass an alt-az mount can follow | PASS / WARN / FAIL | [8](docs/learning/08-mount-dynamics.md) |
| Mount | Acceleration: peak tracking acceleration and the acceleration-limited keyhole | PASS / WARN / FAIL | [8](docs/learning/08-mount-dynamics.md) |
| Mount | Slew and settle: time to get on target vs the regime's usable window | PASS / WARN / FAIL for LEO, INFO elsewhere | [8](docs/learning/08-mount-dynamics.md) |
| Mount | Non-sidereal tracking: can the control software follow a TLE? | Graded where the regime requires it | [8](docs/learning/08-mount-dynamics.md) |
| System | Detection: SNR and limiting magnitude for a representative target | PASS / WARN / FAIL | [9](docs/learning/09-brightness-and-detection.md) |

### Reading the regime results

The compact table printed with every evaluation shows the worst status per component:

```
[----] Orbital regimes (worst status per component; details via the menu or --demo)
       Regime                          Telescope  Camera   Mount    System   Overall
       LEO (Low Earth orbit)           PASS       FAIL     WARN     WARN     FAIL
       MEO (Medium Earth orbit)        PASS       WARN     WARN     WARN     WARN
       GEO (Geosynchronous orbit)      PASS       WARN     PASS     WARN     WARN
       HEO (Highly elliptical orbit)   PASS       WARN     WARN     WARN     WARN
       CIS (Cislunar space)            PASS       PASS     PASS     WARN     WARN
```

This is the demo's DeltaRho 350 + IMX455 on an L-350 with GPS timestamps. Reading it: the telescope is fine everywhere. The camera fails LEO because of rolling-shutter skew, and only needs per-row timestamps for MEO, GEO and HEO. The mount's warnings come from unanswered vendor questions: whether its software supports TLE tracking, and what its axis acceleration is. The fix for LEO is a global-shutter camera, which the demo's RASA 11 + IMX174 row confirms.

The System column is WARN in every regime, and that is not a finding about the hardware. The demo enters no quantum efficiency, throughput, sky brightness or read noise, so the detection check substitutes generic defaults and refuses to grade a PASS on them. Enter real values and the column grades normally.

Choose **Show detailed orbital-regime evaluation** in the menu, or run `--demo`, to see every regime check with its numbers.

---

## Pass prediction

The regime checks above judge one representative geometry per regime. **Predict passes for a satellite** in the main menu uses the real path of one orbit over your site instead: it lists every pass in a time window and judges, pass by pass, whether each evaluated mount can follow it.

You enter:

1. **Your site's location**: latitude (north positive), longitude (east positive) and altitude in metres. It is asked for once and can be changed under **Change site conditions**.
2. **The orbit**, one of:
   * *Paste a TLE* from CelesTrak or Space-Track (two lines, or three with a name line first). It is propagated with SGP4, the model TLEs are fitted with. If any part of the search window is more than 14 days from the TLE's epoch, the tool warns that pass times may be off by minutes.
   * *Define a what-if orbit*: perigee and apogee altitude, inclination, right ascension of the ascending node, argument of perigee and mean anomaly. It is propagated as a Keplerian orbit with J2 drift, which is right for "what would a 550 km, 53 degree orbit look like from here" but not for tracking a particular object.
3. **The window**: start time in UTC (blank for now), length (24 hours by default, 720 at most) and minimum elevation (10 degrees by default).

Example, the what-if orbit 550 km at 53 degrees from 40 N 75 W with a PlaneWave L-350 whose acceleration was entered as 2 deg/s^2:

```
 #  Rise (UTC)            Set (UTC)  Duration MaxEl  Az rate El rate Sunlit  Dark     | Mount 1
 1  2026-10-04 04:56:04 04:58:50     2m45s  11.4   0.245   0.032 no      yes      | [PASS] az rate 203.8x
 2  2026-10-04 06:32:15 06:40:37     8m22s  86.3  11.604   0.686 no      yes      | [WARN] az accel 1.3x
 3  2026-10-04 08:13:07 08:19:47     6m40s  22.3   0.366   0.080 partial yes      | [PASS] az rate 136.5x
```

* **Az rate, El rate** are the peak axis rates an alt-az mount needs during the pass, in deg/s. Pass 2 culminates at 86 degrees, so its azimuth axis has to swing 11.6 deg/s near the zenith: the alt-az keyhole, computed from the real pass rather than from a formula. A pass straight through the zenith needs an instantaneous 180-degree azimuth flip; the table reports it as the flip divided by the 0.1 s sampling step (about 1800 deg/s), which fails every alt-az mount. An equatorial mount has the same problem at the celestial pole on its hour-angle axis.
* **Sunlit** says whether the satellite is in sunlight (yes, partial, no). An optical sensor sees only sunlit satellites.
* **Dark** says whether the Sun is more than 12 degrees below your horizon (yes, twilight, no).
* **Mount N** compares the pass's peak axis rate and acceleration with the mount's ratings, using the thresholds of the regime mount checks (`RATE_PASS_HEADROOM`, `ACCEL_PASS_HEADROOM` and the rest in `src/constants.rs`). The note names the axis and quantity that bind and their headroom. An equatorial mount is judged on its hour-angle and declination axes. A rating that is unknown gives WARN when the pass needs real speed and INFO when it doesn't.
* `<` before a rise time means the satellite was already up when the window started; `>` after a set time means it was still up when the window ended.

The geometry comes from the `orbit-prop` library in `crates/orbit-prop`, whose README documents its models, accuracy and limits. [Lesson 10](docs/learning/10-pass-prediction.md) explains TLEs, SGP4, J2 drift, the reference frames, the pass search and the lighting model.

## The comparison table

Choose **Compare all evaluated configurations**, or run `--demo`. Example from the demo at 2.5" seeing, with GPS timestamps (0.1 ms) and 30" mount pointing assumed:

```
Configuration                  "/px  FWHM"   px/*  bin FOVdeg2 Area m2   dMag Search CFZ+/-  Load%
DeltaRho 350 + IMX455          0.74   3.03    4.0  2x2    2.58  0.0660  +0.00  1.00x   12.1     58
RASA 11 + IMX455               1.25   2.67    2.0  1x1    7.39  0.0509  -0.28  2.21x    6.6     72
CDK14 + IMX455                 0.30   2.56    8.4  4x4    0.43  0.0761  +0.15  0.19x   69.6     60
CDK17 + IMX455                 0.26   2.63   10.0  4x4    0.33  0.1120  +0.57  0.22x   62.1     62
DeltaRho 500 + IMX461          0.50   2.68    5.3  3x3    2.01  0.1321  +0.75  1.56x   12.3     90
RASA 11 + IMX174 (global)      1.95   2.86    1.3  1x1    0.69  0.0509  -0.28  0.21x    6.6     48

 Status by check (P=pass W=warn F=fail i=info):
                              1 2 3 4 5 6 7 8
DeltaRho 350 + IMX455         P W P W i i W P
RASA 11 + IMX455              P P P i i i W W
CDK14 + IMX455                P F W P i i P P
CDK17 + IMX455                P F W P i i P P
DeltaRho 500 + IMX461         P W W P i i W W
RASA 11 + IMX174 (global)     P W W i i i W P

 Orbital regimes, telescope/camera/mount/system (P=pass W=warn F=fail i=info):
                              LEO       MEO       GEO       HEO       CIS
DeltaRho 350 + IMX455         P/F/W/W   P/W/W/W   P/W/P/W   P/W/W/W   P/P/P/W
RASA 11 + IMX455              P/W/W/W   P/W/W/W   P/W/P/W   P/W/W/W   P/P/i/W
CDK14 + IMX455                W/F/W/W   P/W/W/W   P/W/P/W   P/W/W/W   P/P/P/W
CDK17 + IMX455                F/F/W/W   P/W/W/W   P/W/P/W   P/W/W/W   P/P/P/W
DeltaRho 500 + IMX461         P/W/W/W   P/W/W/W   P/W/P/W   P/W/W/W   P/W/P/W
RASA 11 + IMX174 (global)     W/P/W/W   P/P/W/W   P/P/P/W   P/P/W/W   P/P/P/W
```

In the regime matrix each cell is telescope/camera/mount/system. The last row shows the trade a small global-shutter camera makes: it fixes the camera for LEO, but its small field drops the telescope to WARN for LEO acquisition.

| Column | Meaning |
|---|---|
| "/px | Native plate scale |
| FWHM" | Recorded star FWHM at the sensor centre: seeing, diffraction, optics, detector diffusion and pixel aperture ([PSF budget](docs/17-point-spread-function.md)) |
| px/* | Pixels across the sampled star at native resolution |
| bin | Recommended square bin |
| FOVdeg2 | Field area in square degrees |
| Area m2 | Effective collecting area |
| dMag | Depth vs the reference (positive means fainter) |
| Search | Etendue vs the reference |
| CFZ+/- | Critical focus zone half-width, um |
| Load% | Payload as a percent of mount rating ("-" if unknown) |

### Reading the results by mission

No configuration wins every column. Decide which metric matters most for the role before comparing:

| Role | Primary metrics |
|---|---|
| LEO tracking | Regime checks: mount rate and keyhole, global shutter, GPS timing, acquisition field |
| Wide-area search (for example the GEO belt) | Search speed (etendue), then sampling |
| Precision astrometry and tracking | Sampling, optics vs seeing, timing |
| Characterization and photometry | Depth, focus stability, back focus for filters |
| Field portability | Weight, power, setup effort |

In the demo, the RASA 11 is the fastest searcher, the DeltaRho 350 balances depth, sampling and search speed, and the CDK telescopes are deep but narrow and badly oversampled with modern small-pixel CMOS sensors. That makes the CDKs better suited to characterization than to search.

---

## Thresholds and how to tune them

All judgment thresholds live in `src/constants.rs`, in the `checks_limits` and `regimes_limits` modules. [Lesson 11](docs/learning/11-how-the-tool-judges.md) explains which constants are physics and which are rules of thumb.

| Constant | Default | Check | Meaning |
|---|---|---|---|
| `FIT_WARN_FRACTION` | 0.90 | 1 | Image circle may be this fraction of the diagonal before failing |
| `SAMPLING_TARGET` | 2.0 | 2, 3 | Ideal pixels across a star |
| `SAMPLING_UNDER_FAIL` | 1.0 | 2 | Below this, undersampled (FAIL) |
| `SAMPLING_GOOD_MIN` / `MAX` | 1.5 / 2.5 | 2 | Well-sampled range |
| `SAMPLING_BIN2_MAX` | 4.0 | 2 | Upper end of "mildly oversampled" |
| `SAMPLING_OVER_FAIL` | 6.0 | 2 | Above this, heavily oversampled (FAIL) |
| `MAX_BIN` | 4 | 2, 3 | Largest bin considered |
| `PIXEL_MATCH_LOW` / `HIGH` | 0.75 / 1.33 | 3 | Effective/ideal pixel ratio counted as a match |
| `OPTICS_PASS_GROWTH` | 0.15 | 4 | Max star growth for PASS |
| `OPTICS_WARN_GROWTH` | 0.35 | 4 | Max star growth for WARN |
| `CFZ_FORGIVING_UM` | 40 | 7 | At or above this, focus is forgiving |
| `CFZ_DEMANDING_UM` | 15 | 7 | Below this, focus is demanding (WARN) |
| `PAYLOAD_PASS_FRACTION` | 0.70 | 8 | Max load fraction for PASS |
| `PAYLOAD_WARN_FRACTION` | 0.90 | 8 | Max load fraction for WARN |

Orbital-regime thresholds are in the `regimes_limits` module:

| Constant | Default | Component | Meaning |
|---|---|---|---|
| `ACQ_PASS_MARGIN` / `ACQ_WARN_MARGIN` | 2.0 / 1.0 | Telescope | Half short side of field / acquisition uncertainty |
| `TIMING_PIXEL_FRACTION` | 0.25 | Camera | Timing budget as a fraction of one binned pixel of motion |
| `TIMING_WARN_MULTIPLE` | 4.0 | Camera | Timing error up to this multiple of the budget is WARN |
| `SKEW_NEGLIGIBLE_PX` | 0.25 | Camera | Rolling-shutter skew below this needs no correction |
| `SKEW_FAIL_FRAME_FRACTION` | 0.10 | Camera | Skew above this fraction of frame height is FAIL |
| `RATE_PASS_HEADROOM` / `RATE_WARN_HEADROOM` | 3.0 / 1.0 | Mount | Max axis rate / required rate |
| `RATE_MATTERS_DEG_S` | 0.1 | Mount | Above this required rate, an unknown slew rate is WARN |
| `KEYHOLE_PASS_ELEV_DEG` / `KEYHOLE_WARN_ELEV_DEG` | 85 / 70 | Mount | Highest followable pass elevation (alt-az) |
| `ACCEL_PASS_HEADROOM` / `ACCEL_WARN_HEADROOM` | 3.0 / 1.0 | Mount | Max axis acceleration / required acceleration |
| `ACCEL_MATTERS_DEG_S2` | 0.005 | Mount | Above this required acceleration, an unknown rating is WARN |
| `SLEW_PASS_WINDOW_FRACTION` / `SLEW_WARN_WINDOW_FRACTION` | 0.10 / 0.25 | Mount | Slew + settle as a fraction of the usable window |
| `DETECT_SNR_THRESHOLD` | 5.0 | System | SNR at which a target counts as detected |
| `SNR_PASS` | 10.0 | System | SNR for comfortable detection |
| `SNR_TRIVIAL` | 100.0 | System | Above this, detection is simply not what limits the regime |

The photometric and dynamics assumptions live beside them: `REFERENCE_TARGET_CROSS_SECTION_M2` (10 m^2), `REFERENCE_TARGET_ALBEDO` (0.2), `DEFAULT_PHASE_FACTOR` (1.0, full phase), `DEFAULT_QE` (0.80), `DEFAULT_THROUGHPUT` (0.85), `DEFAULT_SKY_MAG_ARCSEC2` (21.0), `DEFAULT_READ_NOISE_E` (3.0), `MAX_EXPOSURE_S` (30 s), `DEFAULT_SLEW_DISTANCE_DEG` (90) and `DEFAULT_SETTLE_TIME_S` (2.0). `PEAK_ACCEL_COEFF` (3*sqrt(3)/8) and `PHOTONS_M2_S_MAG0` (8.9e9) are derived constants, not thresholds, and should not be tuned.

A `plausible_ranges` module sets the bounds outside which a hand-entered value is treated as not entered: QE and throughput 0.01 to 1.0, sky brightness 15 to 24 mag/arcsec^2, read noise 0.1 to 100 e-, axis acceleration 1e-4 to 1000 deg/s^2, axis rate 1e-3 to 1000 deg/s, settle time 0 to 600 s. NaN and the infinities always fail.

`DEFAULT_POINTING_RMS_ARCSEC` (60") in the same file is the pointing error assumed when a mount's figure isn't entered. The regime parameters themselves (ranges, rates, prediction errors) are in `regimes()`.

The default seeing (`DEFAULT_SEEING_ARCSEC`, 2.5") and reference wavelength (`DEFAULT_WAVELENGTH_UM`, 0.55 um, green light near the eye's and many sensors' peak sensitivity) are in `src/constants.rs` too.

---

## Assumptions and limitations

* **Seeing is a single number.** Real seeing varies by night, by elevation angle and through the night. Run the tool at your best, typical and worst seeing to see how sensitive a choice is.
* **Small-angle approximation.** Check 6 reports the true field of view two ways: exactly, as 2 atan(size / 2 FL), and by the small-angle approximation, size / FL. The small-angle value, which overstates the field by about (size / FL)^2 / 12, is the one everything downstream uses. The difference is printed: under 0.03% for every preset.
* **Obstruction ignores support vanes.** Spider vanes and cables in front of the aperture block a few more percent of the light and are not included.
* **Gaussian blur model.** Converting RMS spot to FWHM and adding blurs in quadrature both assume roughly Gaussian blurs. Real optical blur is often not Gaussian, so treat check 4 as an approximate screen, not a performance prediction.
* **Linear spot interpolation.** Spot size between and beyond quoted field points is estimated linearly.
* **Diffraction-based focus criterion.** The CFZ formula uses a standard diffraction criterion at 0.55 um. With seeing-limited images, practical tolerance can be somewhat looser, but fast systems remain demanding.
* **Footprint estimate.** "Pixels in a star's footprint" is approximated as (pixels across)^2. A photometric aperture is typically larger, but the ratio between configurations is what matters.
* **Gaussian point spread function.** Seeing, diffraction, optics, detector diffusion and the pixel aperture are each treated as a Gaussian and added in quadrature. Real seeing has broader wings and diffraction has rings. An unlabelled RMS spot is read as a radius, the larger blur. See [the PSF budget](docs/17-point-spread-function.md).
* **Sidereal drift at the equator.** GEO motion figures assume declination near zero.
* **Regimes are single representative cases.** Each regime is one geometry (for example a 500 km overhead LEO pass). Real targets span wide ranges of altitude, pass geometry and brightness. The overhead pass is deliberately the worst case for rates.
* **Earth rotation simplified.** LEO and MEO rates ignore Earth's rotation, and the HEO and cislunar ground rates are simple differences from the sidereal rate. Directions of motion are ignored.
* **Prediction errors are placeholders.** The along-track errors used for acquisition are assumptions, not catalog statistics.
* **One representative target.** Derived magnitudes assume a 10 m^2 object at 0.2 albedo, so the figures vary between regimes only through range. Real objects span orders of magnitude in size and brightness. Enter a target magnitude to override it.
* **Full phase assumed.** The derived magnitude uses a phase factor of 1.0, the brightest case. A target near quadrature is roughly a magnitude fainter.
* **Sky brightness is a single number.** No dependence on elevation, moon phase or airmass, and no extinction term.
* **Simple saturation model.** The saturation check compares the brightest pixel (star centred on a pixel, light spread along any trail) with 80% of the full well. It ignores bias, dark current and an ADC that clips before the well fills, and the detection SNR itself is still computed as if the detector were linear. See [step 7 of page 10](docs/10-target-brightness-and-detection.md#step-7-saturation).
* **Servo behaviour is still not modeled.** The acceleration model covers peak axis acceleration, the acceleration-limited keyhole and slew timing. Servo bandwidth, closed-loop following error and path-following smoothness are not included, because vendors do not publish the inputs.
* **Slew distance is assumed.** The slew-and-settle check uses a 90-degree acquisition slew and, where the mount does not publish one, a 2-second settle.
* **Photometric defaults are generic.** When QE, throughput, sky brightness or read noise are not entered, documented generic values are substituted and the detection check is capped at WARN. It will never report PASS on a quantum efficiency it assumed.
* **Hand-entered values are range-checked.** A photometric or dynamics value outside a plausible range (a QE above 1, a NaN, a sky brightness of 2.1 where 21.0 was meant) is treated as not entered rather than trusted, so a typo degrades the report instead of corrupting it.
* **Timing is a single figure.** The timestamp accuracy input lumps clock error, exposure-start latency and jitter together.
* **Pass prediction is assessment grade.** Earth orientation uses mean sidereal time only, the Sun and Moon use low-precision formulas, and atmospheric refraction is ignored. Positions are good to about 0.01 degrees, well inside TLE error, but this is not astrometry. The upgrade path is listed in `crates/orbit-prop/README.md` under "Known limitations and future work".
* **Short grazing passes can be missed.** The pass search steps at one sixtieth of the orbital period (90 seconds for the ISS), so a pass that stays above the minimum elevation for less than that may not be found.
* **What-if orbits drift.** They include J2's slow drift but no drag and no short-period terms, so they stand for a kind of orbit, not a specific satellite.
* **Preset data can age.** Specs and weights come from listings at the time of writing and may change. Always confirm against current vendor documentation before buying.

---

## Spec-sheet red flags

* **Obstruction quoted by area.** It looks much smaller than the by-diameter number, which flatters the telescope. A 49% obstruction by diameter is about 24% by area. Always check which one you're reading.
* **"Image circle" vs "fully corrected field."** These are not always the same, and sheets are not always consistent. One PlaneWave CDK14 product page lists both a 70 mm and a 52 mm image circle. Ask which is sharp at the edge.
* **RMS spot without radius or diameter stated, or without off-axis values.** A sharp center says little about the corners of a large sensor.
* **OTA weight alone.** Camera, focuser, dew heaters and cables can add 10 to 20 lb against the mount's rating.
* **Back focus reference point.** "From the mounting surface" and "with focuser installed" can differ by tens of millimeters.
* **Camera pixel count.** Effective pixel counts differ slightly between camera vendors using the same sensor.

---

## Presets and their sources

**Telescopes**

No preset carries an optical throughput figure: no vendor publishes one for these tubes, and every value in `presets.yaml` has a `source`. The detection check assumes 0.85 and says so.

PlaneWave publishes no corrected image circle for the RC20, RC24, RC700 or the IRDK tubes. Those presets leave it blank, and the sensor-fit check warns and tells you to ask the vendor.


| Preset | D (mm) | FL (mm) | Obstruction | Image circle (mm) | Notes |
|---|---|---|---|---|---|
| PlaneWave DeltaRho 280 | 280 | 770.6 | 63% (entered as diameter) | 55 | Spot 4.0 / 4.1 / 4.5 um RMS at 0 / 21 / 27.5 mm. The vendor page does not say whether 63% is by diameter or area. 32 lb. |
| PlaneWave DeltaRho 350 | 350 | 1050 | 56% diameter | 60 | Spot 4.9 / 6.2 / 7.6 um RMS at 0 / 23 / 30 mm. Some listings, including the current PlaneWave page, show 5.6 / 6.4 um off-axis. The conservative values are used. 46 lb. |
| PlaneWave DeltaRho 500 | 508 | 1537 | 59% diameter | 70 | Spot 3.86 / 4.04 / 6.04 um RMS at 0 / 22 / 35 mm. 165 lb. Also sold on an L-500 as the DRL500 system. |
| PlaneWave FSCT8 | 203.2 | 568 | 60% (entered as diameter) | 42 | Spot 3.6 / 6.0 / 6.4 um RMS at 0 / 16 / 21 mm. The vendor page does not say whether 60% is by diameter or area. 20 lb. |
| PlaneWave CDK12.5 | 318 | 2541 | 42% diameter | 52 | Spot 2.8 / 3.8 / 8.0 um RMS at 0 / 11 / 21 mm. 42 lb. Also sold on an L-350 as the CDK300 system. |
| PlaneWave CDK14 | 356 | 2563 | 48.5% diameter | 52 | Spot 3.1 / 6.0 um RMS at 13 / 35 mm. Vendor page lists both 70 and 52 mm image circles. 52 mm is used. 48 lb. Also sold on an L-350 as the CDK350 system. |
| PlaneWave CDK17 | 432 | 2939 | 48.6% diameter | 70 | Spot 6.5 / 9.6 um RMS at 21 / 26 mm. 106 lb. Also sold as the CDK400 (L-500) and CDK450 (L-550) systems. |
| PlaneWave CDK20 f/6.8 | 508 | 3454 | 39% diameter | 52 | Spot 1.5 / 3.8 / 6.0 um RMS at 0 / 12 / 21 mm. 140 lb. Also sold as the CDK500 (L-500) and CDK550 (L-550) systems. |
| PlaneWave CDK20 f/7.7 | 508 | 3951 | 39% diameter | 52 | Vendor page repeats the f/6.8 spot sizes. 140 lb. Also sold as the CDK550 f/7.77 system. |
| PlaneWave CDK24 | 610 | 3974 | 47% diameter | 70 | Spot 2.4 / 4.0 / 4.8 um RMS at 0 / 26 / 35 mm. 240 lb. Also sold on an L-600 as the CDK600 system. |
| PlaneWave CDK700 | 700 | 4540 | 47% diameter | 70 | Complete system on its own mount: no OTA weight. Pair with the 0.7 m system mount. |
| PlaneWave CDK1000 | 1000 | 6000 | 47% diameter | 100 | Complete system on its own mount: no OTA weight. Pair with the 1.0 m system mount. |
| PlaneWave PF1000 | 1000 | 2251 | 60% diameter | 115 | Prime focus. Complete system on its own mount: no OTA weight. Pair with the 1.0 m system mount. |
| PlaneWave RC20 | 508 | 3556 | 39% diameter | not published | Back focus 147 mm from the racked-in focuser. 140 lb. |
| PlaneWave RC24 | 610 | 6469 | 32% diameter | not published | 240 lb. |
| PlaneWave RC700 | 700 | 8410 | < 30% (30% entered) | not published | Complete system on its own mount: no OTA weight. Pair with the 0.7 m system mount. |
| PlaneWave RC1000 | 1000 | 12200 | 27% diameter | 64 | Complete system on its own mount: no OTA weight. Pair with the 1.0 m system mount. |
| PlaneWave IRDK12.5, IRDK14, IRDK17, IRDK20, IRDK24 | as the matching CDK | as the matching CDK | as the matching CDK | not published | Infrared-optimized, gold-coated, with no corrector lens group listed, so the CDK image circles and spot sizes are not assumed. Weights as the matching CDK. |
| Celestron RASA 11 V2 | 279 | 620 | 114 mm (41% diameter) | 43.3 | The camera sits in front of the aperture, so a large camera body adds obstruction. 43 lb (listings vary from 35 to 43). |

**Cameras**

No preset carries a quantum efficiency, read-noise or full-well figure. All three vary with gain, mode and vendor binning for the same sensor, so entering a single number would be inventing data. The detection check assumes 0.80 and 3 e-, the saturation check assumes a 20,000 e- well, and both say so. No preset carries a detector MTF either, so the point spread function counts the pixel aperture but no charge diffusion, and the report says so. Enter `mtf_nyquist` from a measured MTF curve if you have one ([page 17](docs/17-point-spread-function.md)).


| Preset | Pixel (um) | Pixels | Shutter | Notes |
|---|---|---|---|---|
| Sony IMX455 full frame (Moravian C3-61000 PRO, QHY600 PRO) | 3.76 | 9576 x 6388 | Rolling, 39.028 us/line | Line time from the Moravian C3 manual. 2.0 lb (QHY600 PRO). |
| ZWO ASI6200MM Pro / MC Pro | 3.76 | 9576 x 6388 | Rolling, line time unknown | 1.54 lb (0.7 kg). |
| QHYCCD QHY600M / QHY600C PH | 3.76 | 9576 x 6388 | Rolling, line time unknown | Weight not published for the PH. |
| Atik Apx60 | 3.76 | 9576 x 6380 | Rolling, line time unknown | Atik's pages disagree on 6380 vs 6388 rows. About 2.2 lb (1 kg). |
| Moravian Instruments C3-61000 PRO | 3.76 | 9576 x 6388 | Rolling, 39.028 us/line | From the Moravian C3 manual. 3.5 lb (1.6 kg) with standard cooling. |
| FLI Aurora AR455 | 3.76 | 9568 x 6380 | Rolling, line time unknown | Preliminary spec sheet. About 2 lb (0.9 kg). |
| Sony IMX571 APS-C (Moravian C3-26000 PRO) | 3.76 | 6252 x 4176 | Rolling, 34.667 us/line | Line time from the Moravian C3 manual. |
| Sony IMX461 medium format | 3.76 | 11664 x 8750 | Rolling, line time unknown | Exact pixel count varies by vendor. |
| Sony IMX174 global shutter (e.g. QHY174M-GPS) | 5.86 | 1936 x 1216 | Global | Small sensor often used for low-orbit timing work. |

**Mounts**

No preset carries an axis-acceleration or settle-time figure. Only the PlaneWave T-600 page gives an acceleration, and only as a best case that depends on payload, so it is not entered. Ask, and enter what you are told. Until then the keyhole is reported on the rate limit alone and the acceleration check asks you to confirm with the vendor.


| Preset | Type | Payload (lb) | Max slew (deg/s) | Notes |
|---|---|---|---|---|
| PlaneWave L-350 (direct drive) | Alt-az (equatorial with wedge) | 100 | 50 | Pointing accuracy and TLE tracking asked at run time. Confirm with the vendor. |
| PlaneWave L-500 (direct drive) | Alt-az or equatorial | 200 | 50 | Same as above. |
| PlaneWave L-550 (direct drive) | Alt-az or equatorial | 300 | 50 | Same as above. |
| PlaneWave L-600 (direct drive) | Alt-az or equatorial | 300 | 50 | Same as above. Longer swing-through, for 24" tubes. |
| PlaneWave T-600 (direct-drive gimbal) | Alt-az or equatorial | 600 | 100 | Vendor quotes up to 100 deg/s and up to 100 deg/s^2, both depending on payload. The acceleration is not entered. |
| PlaneWave 700 Series Gimbal (direct drive) | Alt-az | 800 | 50 | Vendor quotes "> 50 deg/s". |
| PlaneWave 1000 Series Gimbal (direct drive) | Alt-az | 1200 | 50 | Vendor quotes "> 50 deg/s". |
| PlaneWave 0.7 m system mount (CDK700, RC700) | Alt-az | 300 | 50 | The built-in mount of those systems. Capacity is the instrument payload per port, not the tube. |
| PlaneWave 1.0 m system mount (CDK1000, RC1000, PF1000) | Alt-az | 300 | 50 | The built-in mount of those systems. Capacity is the instrument payload, not the tube. |
| iOptron HAE69C-EC (strain-wave) | Entered as equatorial (can run alt-az) | 69 (79 with counterweight) | not entered | Slew rate, pointing and TLE tracking not entered. |

Preset mounts leave pointing accuracy and TLE-tracking support blank on purpose. The tool asks you for them, because they are exactly the questions to put to a vendor. The `--demo` run assumes 30" pointing and leaves TLE tracking as "unknown" so its WARN results show which question is outstanding.

---

## Code structure and extending the tool

```
Cargo.toml     workspace: scope-eval (this directory), crates/orbit-prop, crates/scope-sim and crates/scope-sim-wasm
src/
  lib.rs       the scope_eval library: everything below except cli/ and main.rs
  model/       data types: Telescope, Camera, Mount, Site, Payload, Config, Obstruction, SpotSpec,
               plus the *_dto.rs wire formats that presets.yaml is parsed into
  calculations/
    optics.rs    telescope geometry, plate scale, sampling, focus and collecting-area calculations
    camera.rs    pixel scale, timestamp accuracy and rolling-shutter calculations
    orbit.rs     circular speed, vis-viva speed and apparent orbital-rate calculations
    mount.rs     mount dynamics, payload capacity and back-focus calculations
    detection.rs target brightness, exposure, signal, noise and limiting-magnitude calculations
    psf.rs       point spread function terms: diffraction, diffusion from MTF, pixel aperture,
                 brightest-pixel fraction and centroid precision
  constants.rs every named constant: physical constants, default assumptions, judgment thresholds
  checks.rs    the eight general checks and their PASS/WARN/FAIL judgments
  regimes.rs   orbital-regime definitions and telescope/camera/mount/system judgments
  equations.rs formatting for the substituted equations and status rules each check records
  photometry.rs  photometric inputs with defaults substituted, and which defaults were assumed
  psf.rs       the system point spread function budget behind checks 2 and 3 and detection
  passes.rs    orbit source to propagator, stale-TLE note, per-pass "Mount can follow?" judgment
  presets.rs   loads presets.yaml
  report/      the text reports, as Display types; nothing here prints
    evaluation.rs      one configuration: the eight checks, PSF budget, GEO timing, regime summary
    comparison.rs      the side-by-side comparison tables
    regime_details.rs  every regime check with its numbers
    passes.rs          the pass table
    formulas.rs        the formula summary
  main.rs      the scope-eval binary: arguments (--demo, --equations, --help), and dispatch to cli/
  cli/         the command-line front end; the only code that reads input or prints
    interactive.rs     the main menu loop and pass prediction
    prompts.rs         prompts that build configurations, sites and orbits
    demo.rs            --demo
    input.rs           validated terminal input helpers
crates/orbit-prop/  satellite propagation (SGP4, Keplerian + J2), observer geometry, lighting, pass finding
crates/scope-sim/   time-stepped simulation of a mount tracking a pass, with pointing error
crates/scope-sim-wasm/  WebAssembly bindings for scope-sim
dashboard/          React + Material UI dashboard that runs and visualizes the simulation
docs/learning/      lessons on the concepts, plus check_examples.py (an independent check of every worked
                    example) and check_equations.py (recomputes every equation in a --equations report)
```

**Design notes**

* The code is layered. The `scope_eval` library (`src/lib.rs`) holds the model, the physics, the judgments and the report text, and never reads input or prints. Reports are `Display` types such as `EvaluationReport` and `ComparisonReport`, so any front end can call `.to_string()` on them. The binary (`src/main.rs` and `src/cli/`) only parses arguments, prompts, and prints those reports.
* Each `calculations/` module groups related equations in a small calculator type. For example, `OpticsCalculator` contains plate-scale and field-of-view equations, while `MountDynamicsCalculator` contains tracking and slew equations. Calculator methods are pure and covered by worked-example tests.
* The calculators contain physics and math only. `checks.rs` and `regimes.rs` apply engineering thresholds to those results and return human-readable check results.
* Each general `check_*` function returns a `CheckResult` (status, detail lines, verdict) and is independent of how results are displayed.
* `evaluate_all` enforces the "first configuration is the reference" rule in one place.
* `evaluate` runs the eight checks, then calls `evaluate_regimes` in `regimes.rs`, which reuses the computed plate scale, bin, field and area. Regime results are stored on the `Evaluation`.

**Adding a preset.** Add a `Telescope` or `Camera` entry to `src/presets.rs`. Fill in the `source` field with where the numbers came from. Use `Obstruction::ByDiameter` or `Obstruction::ByArea` to match how the spec sheet quotes it.

**Changing a rule of thumb.** Edit the relevant limit in `src/constants.rs`. Calculator tests check the equations and worked examples; evaluation checks apply the judgment thresholds.

**Adding a check.** Write a `check_*` function in `checks.rs` returning a `CheckResult`, call the appropriate calculator for its physical quantities, add it to the `checks` vector in `evaluate`, and give it the next number. The report and comparison table pick it up automatically.

**Adding or editing a regime.** Add a `Regime` to `regimes()` in `src/regimes.rs`. Use `OrbitCalculator` methods (`overhead_rate_arcsec_s`, `rate_from_period_arcsec_s`, `vis_viva_km_s`) to derive rates, and set the range, prediction error, usual tracking mode and whether non-sidereal tracking is required. Every report and table picks it up automatically.

**Adding a regime check.** Write a function returning a `RegimeCheck` tagged with its `Component`, and add it to the list in `evaluate_regimes`. Component and overall statuses are recomputed automatically.

**Tests.** `cargo test` runs the worked examples from the [lessons](docs/learning/README.md): plate scale, field of view, effective area and depth, by-area versus by-diameter obstruction, CFZ, ideal pixel, best bin, spot interpolation, the point spread function budget (diffraction, diffusion from MTF, pixel aperture, brightest-pixel fraction), rolling-shutter skew, LEO and MEO overhead rates, GEO and lunar rates from period, Molniya apogee rate, the L-350 keyhole and the GEO timing requirement. It also runs the `orbit-prop` tests, which check the library against published references: Vallado's GMST, site-vector and SGP4 verification cases, and Meeus's Sun and Moon examples.

---

## License

MIT.
