# 5. Image-quality checks (checks 1 to 4)

**What you'll learn:** whether the sensor fits the telescope's sharp field, how many pixels a star covers and why that matters, what pixel size a telescope "wants," and whether the optics or the atmosphere limits image sharpness.

**Before you start:** [Key terms](02-key-terms.md), especially the small-angle rule, seeing, plate scale and binning.

All four checks use the running example: a **PlaneWave DeltaRho 350** (1050 mm focal length, 60 mm image circle) with a **Sony IMX455** (9576 x 6388 pixels of 3.76 um) at **2.5" seeing**. The same numbers are checked by the unit tests named at the end.

---

## Check 1: Sensor fit

**Question.** Does the whole sensor fit inside the telescope's sharp, flat image circle?

**Step 1: sensor size in millimeters.** Multiply pixel count by pixel size:

```
sensor width  (mm) = width_px  x pixel_um / 1000
sensor height (mm) = height_px x pixel_um / 1000
```

**Step 2: the diagonal.** The sensor is a rectangle, and its corners are the points farthest from the center. Those corners sit at half the diagonal from the optical axis, so the diagonal is what must fit:

```
sensor diagonal = sqrt(width^2 + height^2)

PASS if image circle >= diagonal
```

If the image circle is smaller than the diagonal, the corners of every frame are dim or blurry.

**Worked example.** 9576 x 3.76 / 1000 = 36.0 mm and 6388 x 3.76 / 1000 = 24.0 mm. Diagonal = sqrt(36.0^2 + 24.0^2) = 43.3 mm. The image circle is 60 mm, so it passes, with headroom for a larger sensor later.

**Thresholds.** PASS if the image circle covers the diagonal. WARN if it covers at least 90% of the diagonal (soft corners). FAIL below that. When the image circle exceeds 1.15 times the diagonal (`FIT_HEADROOM_FACTOR`), the report adds a note that there is room for a larger sensor.

---

## Check 2: Sampling (plate scale vs seeing)

**Question.** Is each star spread across the right number of pixels?

**Step 1: plate scale.** By the small-angle rule, a pixel of size p at the end of a focal length FL subtends p / FL radians. Multiply by 206,265 to get arcseconds:

```
plate scale ("/px) = 206.265 x pixel_um / FL_mm
```

The constant is 206.265 instead of 206,265 because it absorbs the factor of 1000 from mixing micrometers and millimeters.

**Step 2: pixels across a star.** The atmosphere blurs each star to the seeing FWHM. Divide by the plate scale to get that blur in pixels:

```
pixels across a star = seeing FWHM (") / plate scale
footprint (pixels)   ~ (pixels across)^2
```

**Step 3: compare with the target of about 2.** Finer pixels can't record detail the atmosphere has already erased, so there is a sweet spot:

* **Too few pixels (undersampled).** A star lands in one or two pixels, so its position (centroid) can only be measured coarsely. That hurts astrometry, which is measuring precise sky positions.
* **Too many pixels (oversampled).** A faint target's light is smeared thinly over many pixels. Each pixel adds its own read noise when it is read out, so more pixels means more noise for the same light.

About 2 pixels across the FWHM is the standard compromise.

**Worked example.**

```
plate scale         = 206.265 x 3.76 / 1050 = 0.739 "/px
pixels across star  = 2.5 / 0.739           = 3.38
footprint           ~ 3.38^2                = ~11 pixels
```

That's mildly oversampled. Binning 2x2 gives 1.48 "/px and 1.69 pixels across, which is right on target.

**Recommended bin.** The tool tries square bins from 1x1 to 4x4 and picks the one that brings pixels-across closest to 2.

### Why you can't bin your way out of oversampling on CMOS

The standard signal-to-noise equation for measuring a faint object is:

```
SNR = S / sqrt( S + n x (sky + R^2) )

S   = photons from the target
n   = number of pixels added together to measure it
sky = sky photons per pixel
R   = read noise per pixel readout (electrons)
```

Every pixel readout adds R^2 of noise variance, so the read-noise term grows with the number of pixels. Take a faint target delivering 400 photons, with R = 2 electrons, and ignore sky for a moment:

| Situation | Pixels read | Noise | SNR |
|---|---|---|---|
| Star spread over 69 pixels (heavily oversampled) | 69 | sqrt(400 + 69 x 4) = 26.0 | 15.4 |
| Same, CCD hardware binning 4x4 (charge combined before readout) | ~4 | sqrt(400 + 4 x 4) = 20.4 | 19.6 |
| Same, CMOS digital binning 4x4 (each pixel read, then added) | 69 | sqrt(400 + 69 x 4) = 26.0 | 15.4 |
| Star spread over 11 pixels (well matched) | 11 | sqrt(400 + 11 x 4) = 21.1 | 19.0 |

CMOS sensors bin *after* readout, so every native pixel has already contributed its read noise. Digital binning improves SNR **per super-pixel**, but it does not improve the SNR **of the object as a whole**. That's why the tool reports a "read-noise penalty vs ideal," which is footprint / 4 (a well-sampled star covers about 2 x 2 = 4 pixels), never less than 1.

Digital binning is still useful. It shrinks data volume (2x2 cuts it by 4), raises per-pixel SNR so simple detection thresholds work better, and makes centroiding better behaved.

The sky caveat: sky brightness per square arcsecond is fixed by the site, so binning never changes total sky noise. When exposures are long enough that sky noise dominates read noise, the oversampling penalty shrinks. It matters most for short exposures and dark skies.

**Thresholds (pixels across a star).**

| Range | Status | Meaning |
|---|---|---|
| below 1.0 | FAIL | Undersampled |
| 1.0 to 1.5 | WARN | Slightly undersampled |
| 1.5 to 2.5 | PASS | Well sampled |
| 2.5 to 4.0 | PASS | Mildly oversampled, bin 2x2 if you like |
| 4.0 to 6.0 | WARN | Oversampled, read-noise penalty on CMOS |
| above 6.0 | FAIL | Heavily oversampled |

---

## Check 3: Ideal pixel size (camera match)

**Question.** What pixel size does this telescope want, and does this camera provide it, natively or after binning?

**Step 1: turn the plate-scale formula around.** Set the plate scale to half the seeing (2 pixels across a star) and solve for pixel size:

```
ideal pixel (um) = (seeing / 2) x FL_mm / 206.265
```

Instead of asking "is this camera OK on this telescope?" this tells you what camera the telescope is asking for, which makes it a fast filter when comparing scopes.

**Step 2: find the best bin.** The tool finds the bin factor b (1 to 4) whose effective pixel (b x pixel) is closest to ideal on a ratio scale, then computes the match ratio m = effective / ideal. "Ratio scale" means 0.5x and 2x count as equally far from 1x.

**Worked examples at 2.5" seeing.**

| Telescope | Focal length | Ideal pixel | IMX455 (3.76 um) match |
|---|---|---|---|
| Celestron RASA 11 | 620 mm | 3.8 um | Natural match |
| PlaneWave DeltaRho 350 | 1050 mm | 6.4 um | Good after 2x2 (7.5 um) |
| PlaneWave CDK17 | 2939 mm | 17.8 um | Needs 4x4, read-noise penalty |

**Thresholds.** PASS if 0.75 <= m <= 1.33 with no binning or 2x2. WARN if a match needs 3x3 or 4x4 binning (CMOS read-noise penalty), or if no bin gets within range.

---

## Check 4: Optical quality vs seeing

**Question.** Are the optics sharp enough that the atmosphere, not the glass, sets the image quality, both at the center and at the sensor's corners?

**Step 1: seeing blur in micrometers.** The plate-scale rule in reverse converts the seeing angle into a physical size on the sensor, so it can be compared directly with the vendor's spot size:

```
seeing blur at focal plane (um) = seeing (") x FL_mm / 206.265
```

**Step 2: convert the vendor's RMS spot to a FWHM.** Vendors quote optical performance as an RMS (root-mean-square) spot size, but seeing is a FWHM. Assume the optical blur is roughly a round Gaussian. For a round Gaussian with per-axis standard deviation sigma, FWHM = 2.355 x sigma and RMS radius = 1.414 x sigma, so:

```
optics FWHM (um) ~ 1.665 x RMS spot radius
                 ~ 0.833 x RMS spot diameter
```

**Step 3: add the two blurs in quadrature.** Two independent blurs (atmosphere and optics) combine by convolution. For Gaussian blurs, their variances add, so their widths add as the square root of the sum of squares:

```
combined blur = sqrt(seeing_blur^2 + optics_FWHM^2)
star growth   = combined / seeing_blur - 1
```

A small optical blur barely enlarges a large seeing blur. That's the key intuition: optics only need to be "good enough" relative to the air.

**Step 4: evaluate center and corner.** Spot size usually grows away from the optical axis. The tool evaluates the center (the quoted point closest to the axis) and the sensor corner (radius = half the sensor diagonal). It interpolates linearly between quoted points and extrapolates linearly from the last two points if the corner lies beyond them (flagged as "extrapolated"). An extrapolated value is never allowed to fall below the last quoted spot size, so a spec sheet whose last two points happen to shrink can't make the corner look sharper than the data.

**The radius-or-diameter ambiguity.** Spec sheets often don't say whether their RMS figure is a radius or a diameter, and the answer changes the result by a factor of 2. If you choose "not stated," the tool evaluates both readings. If they lead to different statuses, the check returns WARN and tells you to ask the vendor.

**Worked example (DeltaRho 350).** Seeing blur = 2.5 x 1050 / 206.265 = 12.7 um. The quoted spot is 4.9 um RMS on-axis, interpolated to about 6.1 um at the sensor corner (21.6 mm off-axis).

| Reading | Optics FWHM, center / corner | Star growth, center / corner | Status |
|---|---|---|---|
| RMS radius | 8.2 / 10.2 um | 19% / 28% | WARN |
| RMS diameter | 4.1 / 5.1 um | 5% / 8% | PASS |

The readings disagree, so the result is WARN: ask the vendor which convention they use.

**Thresholds (worst-case star growth).** PASS up to 15%. WARN up to 35%. FAIL above that. If no spot data is entered, the result is INFO with a reminder of what to request.

---

## Check it yourself

**By hand.** Work these with a calculator, then compare.

1. **RASA 11 sensor fit.** The RASA 11's image circle is 43.3 mm. Is the IMX455 (36.0 x 24.0 mm) inside it?
   *Answer:* the diagonal is 43.28 mm, so it passes by about 0.02 mm. The tool reports PASS, but a sensor even slightly larger would not fit.
2. **RASA 11 sampling.** Plate scale = 206.265 x 3.76 / 620 = **1.251 "/px**. Pixels across = 2.5 / 1.251 = **2.0**, which is well sampled at 1x1.
3. **CDK14 sampling.** Plate scale = 206.265 x 3.76 / 2563 = **0.303 "/px**. Pixels across = 2.5 / 0.303 = **8.3**, above 6.0, so FAIL (heavily oversampled).
4. **DeltaRho ideal pixel.** (2.5 / 2) x 1050 / 206.265 = **6.36 um**. The 2x2-binned IMX455 gives 7.52 um, a match ratio of 7.52 / 6.36 = 1.18, inside 0.75 to 1.33, so PASS.
5. **CDK14 optics vs seeing.** The CDK14 quotes 3.1 um RMS at 13 mm and 6.0 um at 35 mm off-axis. The IMX455 corner is 21.6 mm off-axis.
   * Corner spot by interpolation: 3.1 + (21.6 - 13) / (35 - 13) x (6.0 - 3.1) = **4.2 um**.
   * Seeing blur: 2.5 x 2563 / 206.265 = **31.1 um**.
   * Worst case (radius reading): FWHM = 1.665 x 4.2 = 7.0 um; growth = sqrt(31.1^2 + 7.0^2) / 31.1 - 1 = **2.5%**. PASS on either reading. Long focal lengths enlarge the seeing blur in micrometers, which makes the same spot size matter less.
6. **The CMOS binning table.** Recompute one row, for example sqrt(400 + 11 x 4) = 21.07 and 400 / 21.07 = 19.0.

**Against the tests.** Each command runs the test that encodes the matching worked example:

| Concept | Command |
|---|---|
| Plate scale, DeltaRho and RASA | `cargo test plate_scale` |
| Footprint and read-noise penalty | `cargo test sampling_footprint` |
| Recommended bin | `cargo test best_bin_choices` |
| Ideal pixel and ratio-scale matching | `cargo test ideal_pixel_values` and `cargo test closest_binned_pixel` |
| Blurs in quadrature | `cargo test blur_growth` |
| Spot interpolation | `cargo test spot_interpolation` |

**In the tool.** Run `cargo run --release -- --demo`. In the comparison table, the `"/px` and `px/*` columns should show 0.74 and 3.4 for DeltaRho 350 + IMX455, 1.25 and 2.0 for the RASA 11, and 0.30 and 8.3 for the CDK14. In the status-by-check grid, check 2 should be `P` for the first two and `F` for the CDK14, and check 4 should be `W` for the DeltaRho 350.

Next: [Light, field, focus and fit checks](06-light-field-focus-fit-checks.md).
