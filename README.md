# scope-eval

An interactive command-line calculator, written in Rust, that evaluates a **telescope + camera + mount** configuration for ground-based satellite observation. It was built for engineers who need to choose optical hardware for space domain awareness but whose background is software rather than optics.

The tool works in two layers:

1. **Eight general checks** on the optical system: plate scale, sampling, field of view, collecting area, search speed, focus tolerance and payload margin. These don't depend on what you are looking at.
2. **Orbital-regime evaluations.** The telescope, camera and mount are each evaluated separately against five orbital regimes: low Earth orbit (LEO), medium Earth orbit (MEO), geosynchronous orbit (GEO), highly elliptical orbit (HEO) and cislunar space. A configuration that is excellent for GEO can be unusable for LEO, and these checks show which component is the reason.

You pick hardware from built-in presets or type in numbers from any spec sheet. Every result is graded PASS, WARN, FAIL or INFO with a plain-English explanation. Evaluate several configurations and the tool prints a side-by-side comparison, including a regime-by-component matrix.

The tool has no external dependencies. It uses only the Rust standard library, so it builds offline and is easy to audit.

---

## Contents

1. [Quick start](#quick-start)
2. [A short primer on the terms](#a-short-primer-on-the-terms)
3. [Inputs and where to find them](#inputs-and-where-to-find-them)
4. [The algorithm](#the-algorithm)
5. [The eight checks, with formulas and worked examples](#the-eight-checks)
6. [Supplementary: timing reference for GEO stare mode](#supplementary-timing-reference-for-geo-stare-mode)
7. [Orbital-regime evaluations](#orbital-regime-evaluations)
8. [Mount acceleration](#mount-acceleration)
9. [Target brightness and detection](#target-brightness-and-detection)
10. [The comparison table](#the-comparison-table)
11. [Thresholds and how to tune them](#thresholds-and-how-to-tune-them)
12. [Assumptions and limitations](#assumptions-and-limitations)
13. [Spec-sheet red flags](#spec-sheet-red-flags)
14. [Presets and their sources](#presets-and-their-sources)
15. [Code structure and extending the tool](#code-structure-and-extending-the-tool)

---

## Quick start

You need a Rust toolchain, version 1.70 or newer (tested with 1.75). Install it from <https://rustup.rs> if you don't have it.

```bash
cargo build --release          # build
cargo run --release            # interactive menu
cargo run --release -- --demo  # evaluate the presets, show one full regime breakdown, compare all
cargo run --release -- --help  # usage
cargo test                     # run the worked examples in this README as unit tests
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

---

## A short primer on the terms

You don't need an optics background to use this tool, but these terms appear throughout.

**Arcsecond (").** A unit of angle. A full circle is 360 degrees, each degree is 60 arcminutes ('), and each arcminute is 60 arcseconds. One arcsecond is roughly the width of a coin seen from 4 km away. A radian is 206,265 arcseconds, a number that appears in several formulas below.

**Seeing and FWHM.** Turbulence in the atmosphere blurs every star into a small fuzzy disk. "Seeing" is the size of that disk, measured as its **full width at half maximum (FWHM)**: the width of the blur where brightness has fallen to half its peak. Typical seeing at a decent mid-elevation site is 2 to 3 arcseconds. Seeing is set by the site and the weather, not by the telescope, and it's the starting point for most checks.

**Aperture (D).** The diameter of the telescope's main light-collecting opening.

**Focal length (FL).** The effective distance over which the telescope brings light to a focus. Longer focal length means a more magnified image on the sensor.

**Focal ratio (N, written f/N).** Focal length divided by aperture. A low number (f/2 or f/3) is called "fast" and gives a wide, bright, small-scale image. A high number (f/7 or f/8) is "slow" and gives a narrow, magnified image.

**Central obstruction.** Many telescopes have a secondary mirror, lens group or camera sitting in the middle of the incoming light, blocking part of the aperture.

**Image circle.** The diameter of the region at the focal plane where the image is sharp and flat. The camera sensor must fit inside it.

**Plate scale.** How much sky one pixel sees, in arcseconds per pixel.

**Binning.** Combining a square block of neighboring pixels (2x2, 3x3 and so on) into one larger "super-pixel."

**Magnitude.** The astronomical brightness scale. It is logarithmic and runs backwards: larger numbers are fainter. A difference of 5 magnitudes is a factor of 100 in brightness, so 1 magnitude is a factor of about 2.512.

**Etendue.** Collecting area multiplied by field of view. It measures how fast a telescope can survey the sky to a given depth.

**Critical focus zone (CFZ).** How far the sensor can sit from perfect focus before the image visibly degrades.

**GEO and stare mode.** A geosynchronous satellite orbits once per sidereal day, so it stays nearly fixed relative to the ground. If the telescope stops tracking (called stare mode), the GEO object appears as a point while the background stars drift past and leave streaks.

**Rolling shutter.** Most CMOS sensors expose and read out the image one row at a time rather than all at once. The bottom row is captured slightly later than the top row. A **global shutter** sensor captures every row at the same instant.

**Orbital regimes.** Satellites are grouped by altitude and orbit shape:

* **LEO (low Earth orbit)**, roughly 200 to 2,000 km. Fast-moving across the sky, usually bright, visible for only minutes per pass.
* **MEO (medium Earth orbit)**, roughly 2,000 to 35,000 km. Navigation constellations such as GPS (about 20,200 km) live here.
* **GEO (geosynchronous orbit)**, about 35,786 km. Orbits once per sidereal day, so it appears nearly fixed in the sky.
* **HEO (highly elliptical orbit)**, such as the Molniya orbit. Very elongated: fast near the low point, slow and distant near the high point (apogee).
* **Cislunar space**, out to and around the Moon's distance (about 384,000 km). Very distant and therefore very faint.

**TLE and ephemeris.** A two-line element set (TLE) or ephemeris is a prediction of where a satellite will be. Predictions have errors, mostly along the direction of travel, so the telescope must have a wide enough field to catch the target anyway.

**Tracking modes.** *Stare*: the mount is stopped. *Sidereal tracking*: the mount follows the stars. *Rate tracking*: the mount follows the satellite's predicted path, so the satellite stays a point and the stars streak. Rate tracking requires the mount's software to support **non-sidereal tracking**.

**Keyhole.** Every two-axis mount has a direction where one axis would have to spin infinitely fast to follow an object. For an alt-azimuth mount it is straight overhead (the zenith). For an equatorial mount it is near the celestial pole.

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

## The algorithm

For each configuration the tool runs the same fixed pipeline:

```
1. Read site assumptions        seeing FWHM, reference wavelength (0.55 um)
2. Derive basic geometry        f-ratio = FL / D
                                sensor width, height, diagonal (mm) = pixels x pixel size
3. Run the eight checks         each returns: status, detail lines, verdict
     1  Sensor fit              image circle vs sensor diagonal
     2  Sampling                plate scale vs seeing, recommended bin
     3  Ideal pixel             the pixel size this telescope wants
     4  Optics vs seeing        does the glass or the air limit the image?
     5  Area and depth          effective collecting area, magnitudes vs reference
     6  Field and search        field of view, etendue vs reference
     7  Focus tolerance         critical focus zone
     8  Practical fit           payload vs mount rating, back focus
4. Compute the GEO stare-mode timing reference (not graded)
     star drift, timing sensitivity, rolling-shutter skew
5. For each orbital regime (LEO, MEO, GEO, HEO, cislunar):
     Telescope  acquisition field, field dwell, depth relevance
     Camera     timestamp accuracy, shutter skew, exposure vs trailing
     Mount      tracking rate (with keyhole), non-sidereal tracking
     -> worst status per component, and overall
6. Store headline metrics for the comparison tables
```

Step 5 reuses results from step 3. The regime checks use the plate scale and recommended bin from check 2, the field of view from check 6 and the effective area from check 5.

The **reference** for checks 5 and 6 is always the first configuration in the session. When you change the seeing, every stored configuration is re-evaluated in order, so the reference rule stays consistent.

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

Every worked example below uses a **PlaneWave DeltaRho 350** (350 mm aperture, 1050 mm focal length, f/3, 56% obstruction by diameter, 60 mm image circle) with a **Sony IMX455** full-frame sensor (9576 x 6388 pixels of 3.76 um, so 36.0 x 24.0 mm), at **2.5" seeing**. These same numbers are checked by the unit tests.

### Check 1: Sensor fit

**Question.** Does the whole sensor fit inside the telescope's sharp, flat image circle?

**Formula.**

```
sensor width  (mm) = width_px  x pixel_um / 1000
sensor height (mm) = height_px x pixel_um / 1000
sensor diagonal    = sqrt(width^2 + height^2)

PASS if image circle >= diagonal
```

**Why.** The sensor is a rectangle, and its corners are the points farthest from the center. Those corners sit at half the diagonal from the optical axis. If the image circle is smaller than the diagonal, the corners of every frame are dim or blurry.

**Worked example.** Diagonal = sqrt(36.0^2 + 24.0^2) = 43.3 mm. The image circle is 60 mm, so it passes, with headroom for a larger sensor later.

**Thresholds.** PASS if the image circle covers the diagonal. WARN if it covers at least 90% of the diagonal (soft corners). FAIL below that.

### Check 2: Sampling (plate scale vs seeing)

**Question.** Is each star spread across the right number of pixels?

**Formula.**

```
plate scale ("/px)    = 206.265 x pixel_um / FL_mm
pixels across a star  = seeing FWHM (") / plate scale
footprint (pixels)    ~ (pixels across)^2
```

**Why the formula has this shape.** It's the small-angle rule: angle = size / distance. The telescope projects the sky onto the sensor at a distance equal to the focal length, so a pixel of size p subtends p / FL radians. Multiply by 206,265 to convert radians to arcseconds. The 206.265 (instead of 206,265) absorbs the factor of 1000 from mixing micrometers and millimeters.

**Why about 2 pixels across a star is the target.** The atmosphere already blurs each star to the seeing FWHM, so finer pixels can't record real detail.

* **Too few pixels (undersampled).** A star lands in one or two pixels, so its position (centroid) can only be measured coarsely. That hurts astrometry, which is measuring precise sky positions.
* **Too many pixels (oversampled).** A faint target's light is smeared thinly over many pixels. Each pixel adds its own electronic read noise when it is read out, so more pixels means more noise for the same light.

About 2 pixels across the FWHM is the standard compromise.

**Worked example.**

```
plate scale         = 206.265 x 3.76 / 1050 = 0.739 "/px
pixels across star  = 2.5 / 0.739           = 3.38
footprint           ~ 3.38^2                = ~11 pixels
```

That's mildly oversampled. Binning 2x2 gives 1.48 "/px and 1.69 pixels across, which is right on target.

**Recommended bin.** The tool tries square bins from 1x1 to 4x4 and picks the one that brings pixels-across closest to 2.

**Binning and read noise (important for CMOS cameras).** The standard signal-to-noise equation for measuring a faint object is:

```
SNR = S / sqrt( S + n x (sky + R^2) )

S   = photons from the target
n   = number of pixels added together to measure it
sky = sky photons per pixel
R   = read noise per pixel readout (electrons)
```

Every pixel readout adds R^2 of noise variance, so the read-noise term grows with the number of pixels. Consider a faint target delivering 400 photons, with R = 2 electrons, and ignore sky for a moment:

| Situation | Pixels read | Noise | SNR |
|---|---|---|---|
| Star spread over 69 pixels (heavily oversampled) | 69 | sqrt(400 + 69 x 4) = 26.0 | 15.4 |
| Same, CCD hardware binning 4x4 (charge combined before readout) | ~4 | sqrt(400 + 4 x 4) = 20.4 | 19.6 |
| Same, CMOS digital binning 4x4 (each pixel read, then added) | 69 | sqrt(400 + 69 x 4) = 26.0 | 15.4 |
| Star spread over 11 pixels (well matched) | 11 | sqrt(400 + 11 x 4) = 21.1 | 19.0 |

CMOS sensors bin *after* readout, so every native pixel has already contributed its read noise. Digital binning improves SNR **per super-pixel**, but it does not improve the SNR **of the object as a whole**. **You cannot bin your way out of oversampling on a CMOS camera.** That's why the tool reports a "read-noise penalty vs ideal," which is footprint / 4.

Digital binning is still useful. It shrinks data volume (2x2 cuts it by 4), raises per-pixel SNR so simple detection thresholds work better, and makes centroiding better behaved.

The sky term caveat: sky brightness per square arcsecond is fixed by the site, so binning never changes total sky noise. When exposures are long enough that sky noise dominates read noise, the oversampling penalty shrinks. It matters most for short exposures and dark skies.

**Thresholds (pixels across a star).**

| Range | Status | Meaning |
|---|---|---|
| below 1.0 | FAIL | Undersampled |
| 1.0 to 1.5 | WARN | Slightly undersampled |
| 1.5 to 2.5 | PASS | Well sampled |
| 2.5 to 4.0 | PASS | Mildly oversampled, bin 2x2 if you like |
| 4.0 to 6.0 | WARN | Oversampled, read-noise penalty on CMOS |
| above 6.0 | FAIL | Heavily oversampled |

### Check 3: Ideal pixel size (camera match)

**Question.** What pixel size does this telescope want, and does this camera provide it, natively or after binning?

**Formula.** The plate-scale formula solved for pixel size, with the plate scale set to half the seeing:

```
ideal pixel (um) = (seeing / 2) x FL_mm / 206.265
```

**Why it's useful.** This turns the question around. Instead of asking "is this camera OK on this telescope?" it tells you what camera the telescope is asking for. That makes it a fast filter when comparing scopes.

**Worked examples at 2.5" seeing.**

| Telescope | Focal length | Ideal pixel | IMX455 (3.76 um) match |
|---|---|---|---|
| Celestron RASA 11 | 620 mm | 3.8 um | Natural match |
| PlaneWave DeltaRho 350 | 1050 mm | 6.4 um | Good after 2x2 (7.5 um) |
| PlaneWave CDK17 | 2939 mm | 17.8 um | Needs 4x4, read-noise penalty |

**Algorithm.** The tool finds the bin factor b (1 to 4) whose effective pixel (b x pixel) is closest to ideal on a ratio scale, then computes the match ratio m = effective / ideal.

**Thresholds.** PASS if 0.75 <= m <= 1.33 with no binning or 2x2. WARN if a match needs 3x3 or 4x4 binning (CMOS read-noise penalty), or if no bin gets within range.

### Check 4: Optical quality vs seeing

**Question.** Are the optics sharp enough that the atmosphere, not the glass, sets the image quality, both at the center and at the sensor's corners?

**Formulas.**

```
seeing blur at focal plane (um) = seeing (") x FL_mm / 206.265

optics FWHM (um) ~ 1.665 x RMS spot radius
                 ~ 0.833 x RMS spot diameter

combined blur    = sqrt(seeing_blur^2 + optics_FWHM^2)
star growth      = combined / seeing_blur - 1
```

**Why each step.**

1. **Seeing blur in micrometers.** Same small-angle rule as plate scale, run in reverse. It converts the seeing angle into a physical size on the sensor, so it can be compared directly with the vendor's spot size.
2. **RMS spot to FWHM.** Vendors quote optical performance as an RMS (root-mean-square) spot size. To compare it with seeing, which is a FWHM, the tool assumes the optical blur is roughly a round Gaussian. For a round Gaussian with per-axis standard deviation sigma, FWHM = 2.355 x sigma and RMS radius = 1.414 x sigma, so FWHM = 1.665 x RMS radius. If the vendor's figure is a diameter, divide by 2 first.
3. **Adding in quadrature.** Two independent blurs (atmosphere and optics) combine by convolution. For Gaussian blurs, their variances add, so their widths add as the square root of the sum of squares. A small optical blur barely enlarges a large seeing blur.

**Field position.** Spot size usually grows away from the optical axis. The tool evaluates the center (the quoted point closest to the axis) and the sensor corner (radius = half the sensor diagonal). It interpolates linearly between quoted points and extrapolates linearly from the last two points if the corner lies beyond them (flagged as "extrapolated").

**The radius-or-diameter ambiguity.** Spec sheets often don't say whether their RMS figure is a radius or a diameter, and the answer changes the result by a factor of 2. If you choose "not stated," the tool evaluates both readings. If they lead to different statuses, the check returns WARN and tells you to ask the vendor.

**Worked example (DeltaRho 350).** Seeing blur = 2.5 x 1050 / 206.265 = 12.7 um. The quoted spot is 4.9 um RMS on-axis, interpolated to about 6.1 um at the sensor corner (21.6 mm off-axis).

| Reading | Optics FWHM, center / corner | Star growth, center / corner | Status |
|---|---|---|---|
| RMS radius | 8.2 / 10.2 um | 19% / 28% | WARN |
| RMS diameter | 4.1 / 5.1 um | 5% / 8% | PASS |

The readings disagree, so the result is WARN: ask the vendor which convention they use.

**Thresholds (worst-case star growth).** PASS up to 15%. WARN up to 35%. FAIL above that. If no spot data is entered, the result is INFO with a reminder of what to request.

### Check 5: Collecting area and depth

**Question.** How much light does the telescope actually collect, and how much fainter can it see than the reference?

**Formulas.**

```
geometric area      = pi/4 x D^2
blocked fraction    = obstruction_by_diameter^2      (or obstruction_by_area, used as-is)
effective area      = geometric area x (1 - blocked fraction)
equivalent aperture = 2 x sqrt(effective area / pi)

depth vs reference (mag) = 2.5 x log10(effective area / reference area)
```

**Why square the obstruction.** Area goes with diameter squared. When the obstruction is quoted as a fraction of the diameter, it must be squared too to become a fraction of the area. A "56% obstruction by diameter" blocks only 31% of the light. If the spec sheet quotes it by area, do **not** square it.

**Why 2.5 x log10.** That's the definition of the magnitude scale: a brightness ratio r corresponds to 2.5 x log10(r) magnitudes. A rule of thumb: 30% more light is about a quarter magnitude deeper, and twice the light is about 0.75 magnitude.

**Worked example.**

```
DeltaRho 350:  pi/4 x 0.350^2 = 0.0962 m^2,  blocked 0.56^2 = 0.314,  effective 0.0660 m^2
RASA 11:       pi/4 x 0.279^2 = 0.0611 m^2,  blocked (114/279)^2 = 0.167,  effective 0.0509 m^2

depth difference = 2.5 x log10(0.0660 / 0.0509) = 2.5 x log10(1.30) = +0.28 mag
```

Despite being 14 inches against 11, the DeltaRho reaches only about 0.28 magnitude deeper, because of its larger central obstruction.

**Status.** Always INFO. Depth is a trade-off to weigh, not a pass/fail property.

### Check 6: Field of view and search speed

**Question.** How much sky does each frame cover, and how fast can this configuration survey the sky compared to the reference?

**Formulas.**

```
FOV (deg) = sensor size (mm) / FL_mm x 57.2958        (for each axis)
field area (deg^2) = FOV_width x FOV_height
etendue = effective area (m^2) x field area (deg^2)
search speed vs reference = etendue / reference etendue
```

**Why.** FOV is the small-angle rule applied to the whole sensor, with 57.2958 converting radians to degrees. Etendue multiplies how deep each look goes (area) by how much sky each look covers (field). A useful analogy is mowing a lawn: field of view is the width of the mower, and aperture is the power of the engine.

**Worked example.**

```
DeltaRho 350:  36.0 / 1050 x 57.3 = 1.96 deg,  24.0 / 1050 x 57.3 = 1.31 deg  ->  2.58 deg^2
               etendue = 0.0660 x 2.58 = 0.170
RASA 11:       36.0 / 620 x 57.3 = 3.33 deg,   24.0 / 620 x 57.3 = 2.22 deg   ->  7.39 deg^2
               etendue = 0.0509 x 7.39 = 0.376

RASA search speed vs DeltaRho = 0.376 / 0.170 = 2.2x
```

The RASA sweeps sky about 2.2 times faster, but each look is about 0.28 magnitude shallower. "Search speed" compares each configuration at its own depth.

The tool also reports how many non-overlapping fields are needed to tile 100 square degrees, which gives a feel for survey cadence.

**Status.** Always INFO.

### Check 7: Focus tolerance

**Question.** How precisely must the sensor sit at focus?

**Formula.**

```
CFZ (um) = +/- 2.44 x wavelength_um x N^2        (wavelength = 0.55 um, N = FL / D)
```

**Why N squared.** Two effects each contribute a factor of N.

1. The smallest spot the optics can form (the diffraction-limited Airy disk) is about 2.44 x wavelength x N wide. Slower optics have a larger acceptable blur.
2. The cone of light converging to focus has a slope of about 1/N. Slower optics have a gentler cone, so moving the sensor along the axis enlarges the blur more slowly.

Tolerance = acceptable blur / cone slope = (wavelength x N) x N, hence N squared.

**Worked examples.**

| Telescope | f-ratio | CFZ |
|---|---|---|
| RASA 11 | f/2.2 | +/- 6.5 um |
| DeltaRho 350 | f/3.0 | +/- 12.1 um |
| CDK14 | f/7.2 | +/- 69.6 um |

For scale, a human hair is about 70 um thick. Fast systems drift out of focus with small temperature changes, and a sensor tilted by a few micrometers across its width will be sharp on one side and soft on the other.

**Thresholds.** PASS if 40 um or more (forgiving). PASS with a note if between 15 and 40 um (motorized focuser recommended). WARN below 15 um (needs a motorized focuser, temperature-compensated autofocus and a sensor tilt adjuster). This check never fails, because tight focus is a requirement to plan for, not a disqualifier.

### Check 8: Practical fit

**Question.** Can the mount carry the payload with margin, and is there enough back focus for the camera train?

**Formulas.**

```
payload = OTA weight + camera weight + accessories weight
load fraction = payload / mount rating

back focus OK if available >= required
```

**Why 70%.** A mount rated for a given payload is rarely stiff at that limit. Wind, vibration and fast slews all perform better with margin. Keeping the payload at or below about 70% of the rating is a common rule of thumb.

**Worked example.** DeltaRho 350 (46 lb) + IMX455 camera (2 lb) + 10 lb of accessories = 58 lb on a 100 lb mount = 58% of rating. PASS.

**Thresholds.** PASS at 70% or less. WARN up to 90%. FAIL above 90%. Back focus is PASS if available meets the requirement, FAIL if short. Any piece left blank is skipped and noted. If nothing can be evaluated, the status is INFO.

**Back-focus caution.** Vendors measure back focus from different reference points, for example "from the mounting surface without focuser" versus "with focuser installed at mid-travel." Make sure the requirement you enter is measured from the same reference as the telescope's figure.

---

## Supplementary: timing reference for GEO stare mode

These numbers are printed after the eight checks but are not graded. They show, for the GEO case, how timing errors and rolling-shutter readout turn into position errors. The [orbital-regime evaluations](#orbital-regime-evaluations) apply the same ideas to every regime and grade them.

**Star drift past a GEO target.**

```
sidereal rate = 1,296,000" / 86,164.09 s = 15.04 "/s
```

Earth turns 360 degrees (1,296,000 arcseconds) once per sidereal day (86,164 seconds). A GEO satellite turns with Earth, so in stare mode the stars slide past it at this rate. The GEO belt lies close to the celestial equator, so the cos(declination) factor is close to 1 and is ignored.

```
star streak per second (px) = 15.04 / binned plate scale
```

For the DeltaRho 350 binned 2x2: 15.04 / 1.48 = 10 pixels per second of exposure.

**Timing sensitivity.**

```
position error (") = 15.04 x timing error (s)
```

A satellite's reported position is only as good as the timestamp on the image. A 10 ms timing error gives 0.15" of along-track error for a GEO object. A computer clock plus USB latency can easily be off by tens of milliseconds, which is why hardware GPS timestamping matters. For comparison, a low Earth orbit object moving about 1 degree per second (3,600 "/s) picks up 3.6" of error per millisecond.

**Rolling-shutter skew.**

```
readout skew (s) = number of rows x line time
star skew (")    = 15.04 x readout skew
per-row time     = t(first row) + row index x line time
```

For the IMX455 in a Moravian C3-61000: 6,388 rows x 39.028 us = 0.249 s. In stare mode that's a 3.75" systematic skew between the top and bottom of the frame. If the reduction software uses a single timestamp for the whole frame, the plate solution absorbs this skew as a false distortion. The fix is to assign each row its own time using the per-row formula. Global-shutter sensors don't have this effect.

---

## Orbital-regime evaluations

The eight checks describe the optical system in general. Whether that system can actually observe a target depends on where the target is: how far away, how fast it moves and how well its position is predicted. This section evaluates the **telescope**, **camera** and **mount** separately against five representative regimes, so you can see which component limits which mission.

### The regimes and how their numbers are derived

Each regime is described by six numbers. Two are rates, measured two different ways:

* **Rate vs stars.** How fast the target moves against the background stars. This drives timing requirements, rolling-shutter skew and trailing, because positions are measured against the stars.
* **Rate vs ground.** How fast the target moves across the sky as seen from the site. This is what the mount must follow.

| Regime | Representative case | Range (km) | Rate vs stars ("/s) | Rate vs ground ("/s) | Prediction error (km) | Usual mode |
|---|---|---|---|---|---|---|
| LEO | 500 km circular, overhead pass | 500 | ~3,140 (0.87 deg/s) | ~3,140 | 2 | rate track |
| MEO | GPS-like 20,200 km, overhead | 20,200 | ~40 | ~40 | 2 | rate track |
| GEO | 35,786 km altitude | ~37,000 | 15.04 | 0 | 2 | stare |
| HEO | Molniya near apogee | ~39,800 | ~7.8 | ~7.3 | 5 | rate track |
| Cislunar | Lunar distance | ~384,400 | ~0.55 | ~14.5 | 50 | sidereal |

**LEO and MEO rates (overhead pass).** A satellite in a circular orbit at altitude h moves at

```
v = sqrt(mu / (R + h))          mu = 398,600 km^3/s^2 (Earth's gravity), R = 6,378 km
```

Seen from directly below, its angular rate is speed divided by distance:

```
omega (rad/s) ~ v / h           x 206,265 for arcsec/s
```

For 500 km: v = 7.61 km/s, so omega = 7.61 / 500 = 0.0152 rad/s = 3,140 "/s = 0.87 deg/s. An overhead pass is the fastest geometry, which makes it the right worst case for mount and timing requirements. Lower passes are slower. Earth's rotation is ignored here, which is a small error for LEO and roughly 10% for MEO. At these rates the stars' own motion (15"/s) is negligible, so rate vs ground is taken equal to rate vs stars.

**GEO and cislunar rates (from the orbital period).** For distant objects the observer's own motion matters, so the v / h shortcut no longer works. Instead, use the period: an object completing one circle (1,296,000") per period P moves against the stars at

```
rate vs stars = 1,296,000" / P
```

GEO: P = one sidereal day (86,164 s), giving 15.04 "/s. The ground rotates at the same rate, so rate vs ground is 0. Cislunar: using the Moon's period (27.32 days) gives about 0.55 "/s against the stars. The ground turns under it at nearly the full 15.04 "/s, leaving about 14.5 "/s vs ground.

**HEO rate (near apogee).** Speed anywhere on an elliptical orbit comes from the vis-viva equation:

```
v = sqrt( mu x (2/r - 1/a) )
```

For a Molniya orbit (semi-major axis a = 26,560 km, eccentricity 0.74), apogee is at r = a x (1 + e) = 46,214 km from Earth's center, where v = 1.50 km/s. Divided by the apogee altitude (about 39,800 km) that is about 7.8 "/s against the stars. Rate vs ground is approximated as the difference from the sidereal rate, about 7.3 "/s.

**Prediction errors are assumptions.** The along-track prediction errors (2 km for LEO, MEO and GEO, 5 km for HEO, 50 km for cislunar) are placeholder values for a reasonably fresh public element set. Real errors depend heavily on the catalog source and the age of the prediction. Edit them in `regimes()` in `src/regimes.rs` to match your data.

### Telescope checks

**Acquisition field (graded).** Will the target land in the field when the telescope points at the predicted position?

```
prediction error (") = prediction error (km) / range (km) x 206,265
needed (")           = prediction error + mount pointing error
margin               = (short side of the field / 2) / needed
```

The short side is used because the error can lie in any direction. Mount pointing error comes from the mount input, or 60" RMS if not entered. PASS if margin >= 2, WARN if >= 1, FAIL below 1.

Example, DeltaRho 350 + IMX455 for LEO: 2 km / 500 km x 206,265 = 825". Add 30" of pointing error for 855". Half the short side is 1.31 deg / 2 = 2,359". Margin = 2,359 / 855 = 2.8x, PASS. A CDK17 with the same camera has a half short side of only 846", margin 0.99, FAIL. **Wide fields matter for LEO because a small along-track error is a large angle at short range.** The same 2 km at GEO range is only 11".

**Field dwell (INFO).** How long an untracked target stays in the field:

```
dwell (s) = short side of the field (") / rate vs ground ("/s)
```

For LEO on the DeltaRho 350: 4,716" / 3,140 "/s = 1.5 s. This matters for a "stare and catch" approach, where the telescope waits for a satellite to cross. A GEO target never leaves a stopped telescope's field.

**Depth relevance (INFO).** Reports the effective collecting area and depth vs the reference, plus what usually limits detection in the regime. LEO targets are usually bright, so tracking and timing dominate. GEO and especially cislunar targets are faint, so collecting area and search speed dominate. This tool does not compute an absolute limiting magnitude (see [limitations](#assumptions-and-limitations)).

### Camera checks

**Timestamp accuracy (graded).** Is each image timestamped accurately enough that timing error doesn't dominate the measured position?

```
required timing (s) = 0.25 x binned plate scale (") / rate vs stars ("/s)
position error (")  = rate vs stars x timestamp accuracy
```

The budget is a quarter of a binned pixel of target motion. PASS if the entered accuracy meets the requirement. WARN if within 4x. FAIL beyond that.

| Regime | Required timing (DeltaRho 350, 1.48 "/px binned) |
|---|---|
| LEO | 0.12 ms |
| MEO | 9.3 ms |
| GEO | 24.6 ms |
| HEO | 48 ms |
| Cislunar | 670 ms |

A PC clock with USB latency (tens of ms) is borderline even for GEO and hopeless for LEO. LEO needs GPS hardware timestamping.

**Shutter skew (graded).** How far does the target move against the stars while a rolling-shutter sensor reads from top to bottom?

```
readout time (s) = rows x line time
skew (")         = rate vs stars x readout time
```

PASS for a global shutter, or if the skew is under 0.25 binned pixel. WARN if larger but under 10% of the frame height: correctable, but the software must timestamp each row separately (t = t_first_row + row x line time). FAIL above 10% of the frame height, where the frame geometry is badly distorted and a global-shutter camera or a small region-of-interest readout is the better answer. WARN if the line time is unknown.

Example with the IMX455 (6,388 rows x 39.028 us = 0.249 s readout):

| Regime | Skew | DeltaRho 350 frame height (4,716") | Status |
|---|---|---|---|
| LEO | 783" | 16.6% | FAIL |
| MEO | 9.9" | 0.2% | WARN (correctable) |
| GEO | 3.75" | 0.1% | WARN (correctable) |
| Cislunar | 0.14" | negligible | PASS |

**Exposure vs trailing (INFO).** How long can an exposure be before relative motion smears something?

```
crossing time (s)     = seeing FWHM (") / rate vs stars ("/s)
streak per second (px) = rate vs stars / binned plate scale
```

The target and the stars move relative to each other, so one of them always smears in long exposures: the target if the mount follows the stars, the stars if the mount follows the target. For LEO the crossing time is under a millisecond (2.5 / 3,140 = 0.8 ms), which is why LEO observing means rate tracking with streaked stars. For cislunar it is about 4.5 s.

### Mount checks

**Tracking rate (graded).** Can the mount move as fast as the target?

```
required rate (deg/s) = rate vs ground / 3600
headroom              = mount maximum axis rate / required rate
```

PASS if headroom >= 3x (margin for acceleration and corrections), WARN if >= 1x, FAIL below. In stare mode (GEO) the mount only has to point and hold, so it passes. If the mount's slew rate is unknown, the result is WARN when the required rate is significant (above 0.1 deg/s, which in practice means LEO), otherwise INFO.

**Alt-az zenith keyhole.** For an alt-azimuth mount, a pass that goes nearly overhead forces the azimuth axis to swing around quickly. Near the zenith the sky is locally flat, so for a target moving at angular rate omega and passing at a closest zenith distance z (in radians), the peak azimuth rate is about

```
azimuth rate ~ omega / z
```

Setting the azimuth rate equal to the mount's maximum gives the closest followable zenith distance, z = omega / max rate, and therefore the highest followable pass:

```
highest pass elevation (deg) = 90 - degrees(omega / max axis rate)
```

Example, LEO on a PlaneWave L-350 (50 deg/s): z = 0.87 / 50 rad = 1.0 deg, so passes up to 89 deg elevation can be followed. A mount limited to 3 deg/s could only follow passes up to about 73 deg. PASS if >= 85 deg, WARN if >= 70 deg, FAIL below. For equatorial mounts the keyhole is near the celestial pole and is noted but not graded. Acceleration limits are not modeled (see limitations).

**Non-sidereal tracking (graded where required).** Rate tracking needs the mount's control software to follow a predicted path, for example from a TLE. LEO, MEO and HEO require it. PASS if supported, FAIL if not, WARN if unknown (ask the vendor). For GEO (stare) and cislunar (sidereal is usually adequate) it is reported as INFO.

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

---

## Mount acceleration

### Peak tracking acceleration

An overhead pass has `theta(t) = atan(v t / h)`. Differentiating twice, with `u = v t / h`:

```
theta'  = (v/h) / (1 + u^2)
theta'' = -2 (v/h)^2 u / (1 + u^2)^2
```

`|theta''|` peaks at `u = 1/sqrt(3)`, which gives

```
peak acceleration = PEAK_ACCEL_COEFF x omega^2,   PEAK_ACCEL_COEFF = 3 sqrt(3) / 8 = 0.6495
```

where `omega = v/h` is the peak rate. This is a derived constant, not a tuned threshold.

For the LEO regime, `omega = 0.87234 deg/s`, so the peak tracking acceleration is **0.008627 deg/s^2**. That is negligible for any mount in the preset list, and it is why the acceleration model does not stop here: compared against a spec sheet, this check would pass unconditionally and tell you nothing. The equivalent MEO figure is 1.37e-6 deg/s^2, nearly four orders smaller, because the requirement scales as `omega^2`. `ACCEL_MATTERS_DEG_S2` is set at 0.005 so that LEO alone trips the "ask the vendor" branch when a rating is missing.

### The acceleration-limited keyhole

Acceleration bites where the azimuth axis has to whip around near the zenith. There the azimuth angle sweeps through the same `atan` form as the pass itself, with the minimum zenith distance `z` in place of the altitude, so peak azimuth acceleration is `PEAK_ACCEL_COEFF x (omega/z)^2`. Requiring that to stay inside the mount's rating gives

```
z >= omega x sqrt(PEAK_ACCEL_COEFF / max acceleration)
```

The rate limit already gave `z >= omega / max rate`. These are two constraints on one physical keyhole, so the tool reports whichever binds and names which one it was:

| Mount acceleration | Accel-limited keyhole | Rate-limited (50 deg/s) | Binding |
|---|---|---|---|
| 10 deg/s^2 | 88.3 deg elevation | 89.0 deg elevation | acceleration |
| 2 deg/s^2 | 86.2 deg elevation | 89.0 deg elevation | acceleration |
| 0.5 deg/s^2 | 82.5 deg elevation (WARN) | 89.0 deg elevation | acceleration |

Acceleration binds in every realistic case, and by 0.5 deg/s^2 it has pushed the keyhole below `KEYHOLE_WARN_ELEV_DEG`. **When a mount publishes no acceleration figure the reported keyhole is the rate-only number, exactly as before this model existed.** None of the presets publish one, so none of their keyhole figures moved.

### Slew and settle

Getting on target is a trapezoidal move: accelerate to the rate limit, cruise, decelerate. If the distance is too short to reach the rate limit the profile is triangular instead.

```
trapezoidal (D >= v^2/a):  t = v/a + D/v
triangular  (D <  v^2/a):  t = 2 sqrt(D/a)
```

Both branches agree at `D = v^2/a`, so the reported time cannot jump for a small change in the assumed distance.

| Distance | Max rate | Max acceleration | Profile | Time |
|---|---|---|---|---|
| 90 deg | 50 deg/s | 10 deg/s^2 | triangular | 6.0 s |
| 90 deg | 50 deg/s | 50 deg/s^2 | trapezoidal | 2.8 s |
| 90 deg | 6 deg/s | 1 deg/s^2 | trapezoidal | 21.0 s |

Slew plus settle is then graded against the regime's usable window. Only LEO has one (300 s, roughly how long a 500 km pass stays above useful elevation); every other regime stays available for hours, so the check reports INFO there. A direct-drive mount needs 6 s of slew and 2 s of settle, under 3% of a LEO pass. If the acceleration rating is missing but the slew rate is known, the tool still reports `D / v` as an explicit lower bound rather than giving up.

---

## Target brightness and detection

### How bright the target is

For a diffuse (Lambertian) target, the apparent magnitude follows from its cross-section, albedo, range and phase. The `1/pi` is the Lambertian scattering factor:

```
m = -26.74 - 2.5 x log10(albedo x area x phase / (pi x d^2))
```

with the area in square metres and the range in metres. The tool assumes one representative target, a 10 m^2 object at 0.2 albedo at full phase, so magnitudes differ between regimes only through range: the same object, moved further away.

| Regime | Range | Derived magnitude |
|---|---|---|
| LEO | 500 km | 2.25 |
| MEO | 20,200 km | 10.28 |
| GEO | 37,000 km | 11.59 |
| HEO | 39,836 km | 11.75 |
| Cislunar | 384,400 km | 16.67 |

The check on the absolute scale is that these land where real objects do: GEO objects run 11 to 15, cislunar 16 to 20. You can override the magnitude with your own figure.

### Signal, sky and trailing

A magnitude-zero source delivers about `8.9e9` photons per square metre per second in V band. (From the V-band zero point 3.64e-23 W/m^2/Hz over a 550 nm band 89 nm wide, giving 3.21e-9 W/m^2, divided by the 3.61e-19 J energy of a 550 nm photon.) So:

```
signal (e-/s)      = 8.9e9 x 10^(-0.4 m) x effective area x QE x throughput
sky (e-/px/s)      = the same, at the sky magnitude, x plate scale^2
```

The sky term is just the point-source rate for that surface brightness scaled by the solid angle one pixel covers.

Exposure is not a separate input. A target the mount holds still does not trail, so nothing bounds the exposure but a 30 s cap. A target tracked sidereally drifts at its rate against the stars, and the natural exposure is the one that keeps its trail inside a single seeing disk:

```
exposure = seeing / residual rate     (capped at MAX_EXPOSURE_S = 30 s)
trail    = residual rate x exposure
footprint (px) = (seeing / scale) x ((seeing + trail) / scale)
```

For cislunar at 2.5" seeing: the Moon's rate against the stars is 0.549"/s, so the exposure is 2.5 / 0.549 = **4.554 s**, the trail is 2.5" by construction, and the footprint is 2.5 x 5.0 / 0.7386^2 = **22.914 px**. You can override the exposure; if your choice trails the target off the sensor, the tool says so and will not report a PASS.

### Signal-to-noise and limiting magnitude

```
SNR = S / sqrt(S + B + R^2 x n)
```

The signal appears inside the noise term because photon arrival is Poisson: its own shot noise is `sqrt(S)`.

Setting `SNR = T` and solving for the signal gives a quadratic with one positive root, so the faintest detectable magnitude needs no search:

```
S_min = (T^2 + sqrt(T^4 + 4 T^2 N)) / 2,    N = B + R^2 x n
m_limit = -2.5 x log10(S_min / K),          K = 8.9e9 x area x QE x throughput x exposure
```

For the DeltaRho 350 (0.0660 m^2, 0.7386 "/px) with an IMX455 at 2.5" seeing, 21.0 mag/arcsec^2 sky, QE 0.80, throughput 0.85 and 3 e- read noise:

| Regime | Mode | Exposure | Target mag | SNR | Limiting mag | Margin | Verdict |
|---|---|---|---|---|---|---|---|
| LEO | rate-track | 30 s (capped) | 2.25 | ~38,900 | 20.06 | +17.8 | trivial |
| MEO | rate-track | 30 s (capped) | 10.28 | 964 | 20.06 | +9.8 | trivial |
| GEO | stare | 30 s (capped) | 11.59 | 526 | 20.06 | +8.5 | trivial |
| HEO | rate-track | 30 s (capped) | 11.75 | 488 | 20.06 | +8.3 | trivial |
| Cislunar | sidereal | 4.554 s (trail-limited) | 16.67 | 14.9 | 18.15 | +1.5 | graded |

Two things in that table are worth reading twice.

The limiting magnitude is **identical at 20.06 for all four stationary-target regimes**. That is not a coincidence: they share an exposure (the 30 s cap), a zero trail, and therefore the same footprint and noise budget. They differ only in how bright the target is.

And four of the five regimes sit above `SNR_TRIVIAL`, so the tool reports "detection is not the limiting factor" instead of a graded margin. That is the right answer rather than a mis-set threshold: a 14-inch aperture at 30 seconds genuinely does not struggle with anything nearer than the Moon. Cislunar is the only regime where detection is close, and it is the only regime whose limiting factor reads "brightness above all". The model reproduces the tool's own editorial judgment from computed numbers.

Where the brightness figures come from generic defaults rather than entered values, the check is capped at WARN and names what it assumed. It will not tell you a configuration will detect something on the strength of a quantum efficiency it invented.

## The comparison table

Choose **Compare all evaluated configurations**, or run `--demo`. Example from the demo at 2.5" seeing, with GPS timestamps (0.1 ms) and 30" mount pointing assumed:

```
Configuration                  "/px   px/*  bin FOVdeg2 Area m2   dMag Search CFZ+/-  Load%
DeltaRho 350 + IMX455          0.74    3.4  2x2    2.58  0.0660  +0.00  1.00x   12.1     58
RASA 11 + IMX455               1.25    2.0  1x1    7.39  0.0509  -0.28  2.21x    6.6     72
CDK14 + IMX455                 0.30    8.3  4x4    0.43  0.0761  +0.15  0.19x   69.6     60
CDK17 + IMX455                 0.26    9.5  4x4    0.33  0.1114  +0.57  0.22x   62.1      -
DeltaRho 500 + IMX461          0.50    5.0  3x3    2.01  0.1321  +0.75  1.56x   12.3     90
RASA 11 + IMX174 (global)      1.95    1.3  1x1    0.69  0.0509  -0.28  0.21x    6.6     48

 Status by check (P=pass W=warn F=fail i=info):
                              1 2 3 4 5 6 7 8
DeltaRho 350 + IMX455         P P P W i i W P
RASA 11 + IMX455              P P P i i i W W
CDK14 + IMX455                P F W P i i P P
CDK17 + IMX455                P F W i i i P i
DeltaRho 500 + IMX461         P W W i i i W W
RASA 11 + IMX174 (global)     P W W i i i W P

 Orbital regimes, telescope/camera/mount (P=pass W=warn F=fail i=info):
                              LEO     MEO     GEO     HEO     CIS
DeltaRho 350 + IMX455         P/F/W   P/W/W   P/W/P   P/W/W   P/P/P
RASA 11 + IMX455              P/W/W   P/W/W   P/W/P   P/W/W   P/P/i
CDK14 + IMX455                W/F/W   P/W/W   P/W/P   P/W/W   P/P/P
CDK17 + IMX455                F/F/W   P/W/W   P/W/P   P/W/W   P/P/P
DeltaRho 500 + IMX461         P/W/W   P/W/W   P/W/P   P/W/W   P/W/P
RASA 11 + IMX174 (global)     W/P/W   P/P/W   P/P/P   P/P/W   P/P/P
```

In the regime matrix each cell is telescope/camera/mount. The last row shows the trade a small global-shutter camera makes: it fixes the camera for LEO, but its small field drops the telescope to WARN for LEO acquisition.

| Column | Meaning |
|---|---|
| "/px | Native plate scale |
| px/* | Pixels across a star at native resolution |
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

All judgment thresholds live in the `limits` module at the top of `src/checks.rs`.

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

Orbital-regime thresholds live in the `limits` module of `src/regimes.rs`:

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

The default seeing (2.5") and reference wavelength (0.55 um, green light near the eye's and many sensors' peak sensitivity) are constants at the top of `src/main.rs`.

---

## Assumptions and limitations

* **Seeing is a single number.** Real seeing varies by night, by elevation angle and through the night. Run the tool at your best, typical and worst seeing to see how sensitive a choice is.
* **Small-angle approximation.** Field of view uses size / focal length. The error is well under 0.1% for fields of a few degrees.
* **Obstruction ignores support vanes.** Spider vanes and cables in front of the aperture block a few more percent of the light and are not included.
* **Gaussian blur model.** Converting RMS spot to FWHM and adding blurs in quadrature both assume roughly Gaussian blurs. Real optical blur is often not Gaussian, so treat check 4 as an approximate screen, not a performance prediction.
* **Linear spot interpolation.** Spot size between and beyond quoted field points is estimated linearly.
* **Diffraction-based focus criterion.** The CFZ formula uses a standard diffraction criterion at 0.55 um. With seeing-limited images, practical tolerance can be somewhat looser, but fast systems remain demanding.
* **Footprint estimate.** "Pixels in a star's footprint" is approximated as (pixels across)^2. A photometric aperture is typically larger, but the ratio between configurations is what matters.
* **Sidereal drift at the equator.** GEO motion figures assume declination near zero.
* **Regimes are single representative cases.** Each regime is one geometry (for example a 500 km overhead LEO pass). Real targets span wide ranges of altitude, pass geometry and brightness. The overhead pass is deliberately the worst case for rates.
* **Earth rotation simplified.** LEO and MEO rates ignore Earth's rotation, and the HEO and cislunar ground rates are simple differences from the sidereal rate. Directions of motion are ignored.
* **Prediction errors are placeholders.** The along-track errors used for acquisition are assumptions, not catalog statistics.
* **One representative target.** Derived magnitudes assume a 10 m^2 object at 0.2 albedo, so the figures vary between regimes only through range. Real objects span orders of magnitude in size and brightness. Enter a target magnitude to override it.
* **Full phase assumed.** The derived magnitude uses a phase factor of 1.0, the brightest case. A target near quadrature is roughly a magnitude fainter.
* **Sky brightness is a single number.** No dependence on elevation, moon phase or airmass, and no extinction term.
* **No saturation model.** Detector full-well depth is not modeled, so bright LEO targets report implausibly high SNR. The check reports these as "detection is not the limiting factor" rather than as a number to act on.
* **Servo behaviour is still not modeled.** The acceleration model covers peak axis acceleration, the acceleration-limited keyhole and slew timing. Servo bandwidth, closed-loop following error and path-following smoothness are not included, because vendors do not publish the inputs.
* **Slew distance is assumed.** The slew-and-settle check uses a 90-degree acquisition slew and, where the mount does not publish one, a 2-second settle.
* **Photometric defaults are generic.** When QE, throughput, sky brightness or read noise are not entered, documented generic values are substituted and the detection check is capped at WARN. It will never report PASS on a quantum efficiency it assumed.
* **Hand-entered values are range-checked.** A photometric or dynamics value outside a plausible range (a QE above 1, a NaN, a sky brightness of 2.1 where 21.0 was meant) is treated as not entered rather than trusted, so a typo degrades the report instead of corrupting it.
* **Timing is a single figure.** The timestamp accuracy input lumps clock error, exposure-start latency and jitter together.
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


| Preset | D (mm) | FL (mm) | Obstruction | Image circle (mm) | Notes |
|---|---|---|---|---|---|
| PlaneWave DeltaRho 350 | 350 | 1050 | 56% diameter | 60 | Spot 4.9 / 6.2 / 7.6 um RMS at 0 / 23 / 30 mm. Some listings show 5.6 / 6.4 um off-axis. The conservative values are used. 46 lb. |
| PlaneWave DeltaRho 500 | 508 | 1537 | 59% diameter | 70 | 165 lb. No spot data. |
| PlaneWave CDK14 | 356 | 2563 | 48.5% diameter | 52 | Spot 3.1 / 6.0 um RMS at 13 / 35 mm. Vendor page lists both 70 and 52 mm image circles. 52 mm is used. 48 lb. |
| PlaneWave CDK17 | 432 | 2939 | 49% diameter | 70 | Weight not entered. |
| Celestron RASA 11 V2 | 279 | 620 | 114 mm (41% diameter) | 43.3 | The camera sits in front of the aperture, so a large camera body adds obstruction. 43 lb (listings vary from 35 to 43). |

**Cameras**

No preset carries a quantum efficiency or read-noise figure. Both vary with gain, mode and vendor binning for the same sensor, so entering a single number from a QE curve would be inventing data. The detection check assumes 0.80 and 3 e- and says so.


| Preset | Pixel (um) | Pixels | Shutter | Notes |
|---|---|---|---|---|
| Sony IMX455 full frame (Moravian C3-61000 PRO, QHY600 PRO) | 3.76 | 9576 x 6388 | Rolling, 39.028 us/line | Line time from the Moravian C3 manual. 2.0 lb (QHY600 PRO). |
| Sony IMX571 APS-C (Moravian C3-26000 PRO) | 3.76 | 6252 x 4176 | Rolling, 34.667 us/line | Line time from the Moravian C3 manual. |
| Sony IMX461 medium format | 3.76 | 11664 x 8750 | Rolling, line time unknown | Exact pixel count varies by vendor. |
| Sony IMX174 global shutter (e.g. QHY174M-GPS) | 5.86 | 1936 x 1216 | Global | Small sensor often used for low-orbit timing work. |

**Mounts**

No preset carries an axis-acceleration or settle-time figure: none of these vendors publish them. Ask, and enter what you are told. Until then the keyhole is reported on the rate limit alone and the acceleration check asks you to confirm with the vendor.


| Preset | Type | Payload (lb) | Max slew (deg/s) | Notes |
|---|---|---|---|---|
| PlaneWave L-350 (direct drive) | Alt-az (equatorial with wedge) | 100 | 50 | Pointing accuracy and TLE tracking asked at run time. Confirm with the vendor. |
| PlaneWave L-500 (direct drive) | Alt-az or equatorial | 200 | 50 | Same as above. |
| iOptron HAE69C-EC (strain-wave) | Entered as equatorial (can run alt-az) | 69 (79 with counterweight) | not entered | Slew rate, pointing and TLE tracking not entered. |

Preset mounts leave pointing accuracy and TLE-tracking support blank on purpose. The tool asks you for them, because they are exactly the questions to put to a vendor. The `--demo` run assumes 30" pointing and leaves TLE tracking as "unknown" so its WARN results show which question is outstanding.

---

## Code structure and extending the tool

```
src/
  main.rs      command-line entry point, interactive menus, --demo, --help
  model.rs     data types: Telescope, Camera, Mount, Site, Payload, Config, Obstruction, SpotSpec
  checks.rs    constants, thresholds, pure calculation functions, the eight checks, unit tests
  regimes.rs   orbital regimes, orbital-rate formulas, telescope/camera/mount regime checks, unit tests
  presets.rs   built-in telescopes, cameras and mounts, with sources
  report.rs    printing evaluations, regime summaries and details, comparison tables, formula summary
  input.rs     validated terminal input helpers
```

**Design notes**

* The calculation functions at the top of `checks.rs` (plate scale, ideal pixel, effective area, CFZ and so on) are pure functions with no input/output. They can be reused from other code or wrapped in a different front end, such as a GUI or web service.
* Each `check_*` function returns a `CheckResult` (status, detail lines, verdict) and is independent of how results are displayed.
* `evaluate_all` enforces the "first configuration is the reference" rule in one place.
* `evaluate` runs the eight checks, then calls `evaluate_regimes` in `regimes.rs`, which reuses the computed plate scale, bin, field and area. Regime results are stored on the `Evaluation`.

**Adding a preset.** Add a `Telescope` or `Camera` entry to `src/presets.rs`. Fill in the `source` field with where the numbers came from. Use `Obstruction::ByDiameter` or `Obstruction::ByArea` to match how the spec sheet quotes it.

**Changing a rule of thumb.** Edit the constant in the `limits` module of `src/checks.rs`. The unit tests check the physics, not the thresholds, so they keep passing.

**Adding a check.** Write a `check_*` function in `checks.rs` returning a `CheckResult`, add it to the `checks` vector in `evaluate`, and give it the next number. The report and comparison table pick it up automatically.

**Adding or editing a regime.** Add a `Regime` to `regimes()` in `src/regimes.rs`. Use the helper functions (`overhead_rate_arcsec_s`, `rate_from_period_arcsec_s`, `vis_viva_km_s`) to derive rates, and set the range, prediction error, usual tracking mode and whether non-sidereal tracking is required. Every report and table picks it up automatically.

**Adding a regime check.** Write a function returning a `RegimeCheck` tagged with its `Component`, and add it to the list in `evaluate_regimes`. Component and overall statuses are recomputed automatically.

**Tests.** `cargo test` runs the worked examples from this README: plate scale, field of view, effective area and depth, by-area versus by-diameter obstruction, CFZ, ideal pixel, best bin, spot interpolation, rolling-shutter skew, LEO and MEO overhead rates, GEO and lunar rates from period, Molniya apogee rate, the L-350 keyhole and the GEO timing requirement.

---

## License

MIT.
