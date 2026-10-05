# Lesson 2: Seeing and sampling

This lesson covers the most important match in any imaging system: the size of a pixel on the sky against the size of a star blurred by the atmosphere. It is checks 2 and 3 in the tool, and the binned plate scale it produces feeds almost every orbital-regime check later.

**Prerequisites:** [Lesson 1](01-angles-and-magnitudes.md) (arcseconds, small-angle rule).

## You will be able to

* Explain what seeing is and why it is measured as a FWHM.
* Calculate the plate scale of any telescope and camera pair.
* Decide whether a system is undersampled, well sampled or oversampled, and pick a bin factor.
* Explain, with the signal-to-noise equation, why digital binning on a CMOS camera does not undo oversampling.
* Work backwards from a telescope's focal length to the pixel size it wants.

## Key terms

| Term | Meaning |
|---|---|
| **Seeing** | The blur the atmosphere adds to every star. Set by the site and the night, not the telescope. |
| **FWHM** | Full width at half maximum: the width of a blur profile at the height where it has fallen to half its peak. |
| **Focal length (FL)** | The effective distance over which the telescope forms its image. It sets how big the sky looks on the sensor. |
| **Plate scale** | The sky angle covered by one pixel, in arcseconds per pixel ("/px). |
| **Sampling** | How many pixels span a star's FWHM. About 2 is the target. |
| **Undersampled / oversampled** | Too few / too many pixels across a star. |
| **Binning** | Combining a b x b block of pixels into one super-pixel. Multiplies the plate scale by b. |
| **Read noise (R)** | Electronic noise added each time a pixel is read, in electrons RMS. |
| **Footprint** | The number of pixels a star's light lands in, roughly (pixels across)^2. |

---

## 2.1 Seeing and FWHM

Light from a star passes through tens of kilometers of turbulent air. Pockets of slightly different temperature bend the light by slightly different amounts, so in an exposure longer than a fraction of a second, a point-like star smears into a fuzzy disk.

The size of that disk is the **seeing**. It is quoted as a FWHM in arcseconds because a FWHM is easy to measure on an image and doesn't depend on the faint wings of the profile. Typical values:

| Site | Seeing |
|---|---|
| World-class mountain observatory | 0.5" to 1" |
| Good mid-elevation site | 1.5" to 2.5" |
| Suburban backyard | 2.5" to 4" |

The tool's default is 2.5". Seeing changes night to night, so it is worth re-running the tool at your best and worst seeing.

**Key point.** Seeing is fixed by the site. A bigger telescope does not reduce it. That makes it the natural yardstick for every check in this lesson.

## 2.2 Plate scale

**Idea.** The telescope projects the sky onto the sensor. A pixel of size `p` at focal distance `FL` covers an angle `p / FL` on the sky (the small-angle rule from Lesson 1).

**Formula.**

```
plate scale ("/px) = 206.265 x pixel_um / FL_mm
```

**Why 206.265 and not 206,265.** The pixel is in micrometers and the focal length in millimeters. Converting micrometers to millimeters divides by 1000, and 206,265 / 1000 = 206.265.

**Worked example.** DeltaRho 350 (FL 1050 mm) with the IMX455 (3.76 um pixels):

```
plate scale = 206.265 x 3.76 / 1050 = 0.739 "/px
```

Try it yourself for the RASA 11 (FL 620 mm) with the same camera before reading on. You should get 1.25 "/px.

## 2.3 Sampling: pixels across a star

**Formula.**

```
pixels across a star = seeing FWHM (") / plate scale ("/px)
footprint (pixels)   ~ (pixels across)^2
```

**Worked example.**

```
pixels across = 2.5 / 0.739 = 3.38
footprint     ~ 3.38^2      = 11.4 pixels
```

**Why about 2 pixels across is the target.** The atmosphere has already blurred away any detail finer than the seeing, so there is nothing to gain from very small pixels. There are costs on both sides:

* **Undersampled (fewer than about 1.5 pixels across).** The whole star falls in one or two pixels. You can't tell where in the pixel it sits, so position measurements (astrometry) are coarse.
* **Oversampled (more than about 2.5 pixels across).** The star's light is spread thin over many pixels. Each pixel adds its own read noise, so the faint target is buried in more noise than necessary.

Two pixels across is the usual compromise: enough to locate the center precisely, few enough to keep the noise down. It is related to the Nyquist sampling theorem from signal processing, which says you need at least two samples per cycle of the finest detail you want to record.

The tool grades it this way:

| Pixels across | Status | Meaning |
|---|---|---|
| below 1.0 | FAIL | Undersampled |
| 1.0 to 1.5 | WARN | Slightly undersampled |
| 1.5 to 2.5 | PASS | Well sampled |
| 2.5 to 4.0 | PASS | Mildly oversampled, bin 2x2 if you like |
| 4.0 to 6.0 | WARN | Oversampled, read-noise penalty on CMOS |
| above 6.0 | FAIL | Heavily oversampled |

The running example, at 3.38 pixels across, is mildly oversampled: PASS.

## 2.4 Binning

Binning combines a b x b block of pixels into one. The plate scale grows by b and the pixels across a star shrink by b.

**Algorithm (recommended bin).** Try b = 1, 2, 3 and 4. Keep the one for which `pixels across / b` is closest to 2.

**Worked example.**

| Bin | Plate scale | Pixels across | Distance from 2 |
|---|---|---|---|
| 1x1 | 0.739 | 3.38 | 1.38 |
| 2x2 | 1.477 | 1.69 | **0.31** |
| 3x3 | 2.216 | 1.13 | 0.87 |
| 4x4 | 2.954 | 0.85 | 1.15 |

2x2 wins. The **binned plate scale of 1.48 "/px** is the number the orbital-regime checks use from here on.

## 2.5 Why you can't bin your way out of oversampling on CMOS

This is the most counter-intuitive idea in the tool, so it's worth working through slowly.

**The signal-to-noise equation.** When you measure a faint object by adding up the pixels it falls in:

```
SNR = S / sqrt( S + n x (sky + R^2) )

S   = electrons from the target (its own shot noise is sqrt(S))
n   = number of pixels *read out* and added together
sky = sky electrons per pixel
R   = read noise per readout, electrons RMS
```

Each readout adds noise *variance* R^2. Variances of independent noise sources add, so n readouts add n x R^2.

**CCD vs CMOS binning.**

* A **CCD** can bin in hardware: it moves the charge from a 4x4 block into one well *before* reading it, so the block is read once and pays R^2 once.
* A **CMOS** sensor reads every pixel individually, then the software adds them. Each of the 16 pixels has already paid its R^2.

**Worked example.** A faint target gives 400 electrons, read noise R = 2 e-, and ignore the sky for now:

| Situation | Pixels read (n) | Noise = sqrt(400 + n x 4) | SNR |
|---|---|---|---|
| Star spread over 69 pixels (heavily oversampled) | 69 | 26.0 | 15.4 |
| Same star, CCD hardware binning 4x4 | ~4 | 20.4 | 19.6 |
| Same star, CMOS digital binning 4x4 | 69 | 26.0 | 15.4 |
| Star spread over 11 pixels (well matched) | 11 | 21.1 | 19.0 |

Compute one row yourself. For the first: 400 + 69 x 4 = 676, sqrt(676) = 26.0, 400 / 26.0 = 15.4.

On CMOS, binning raises the SNR of each super-pixel (which helps simple thresholds) and cuts the data volume by b^2, but it cannot recover read noise that was already added. That is why the tool reports a **read-noise penalty vs ideal = footprint / 4**. For the running example, 11.4 / 4 = **x2.9**.

**The sky caveat.** Sky brightness per square arcsecond is fixed by the site. However you slice it into pixels, the total sky under the star is the same, so the sky noise is unchanged by sampling. When exposures are long enough that sky noise dominates read noise, the oversampling penalty fades. It matters most for short exposures on dark sites, which is exactly the LEO case.

## 2.6 The ideal pixel (check 3)

**Idea.** Turn the question around: instead of "is this camera OK?", ask "what pixel size does this telescope want?". Set the plate scale to half the seeing (2 pixels across) and solve the plate-scale formula for pixel size:

```
ideal pixel (um) = (seeing / 2) x FL_mm / 206.265
```

**Worked examples at 2.5" seeing.**

| Telescope | Focal length | Ideal pixel | IMX455 (3.76 um) match |
|---|---|---|---|
| Celestron RASA 11 | 620 mm | 3.8 um | Natural match |
| PlaneWave DeltaRho 350 | 1050 mm | 6.4 um | Good after 2x2 (7.5 um) |
| PlaneWave CDK17 | 2939 mm | 17.8 um | Needs 4x4, read-noise penalty |

**Algorithm.** For bin b from 1 to 4, compute the effective pixel `b x pixel`. Pick the b whose effective pixel is closest to ideal *on a ratio scale* (the smallest `|ln(effective / ideal)|`), so being 2x too big counts the same as being 2x too small. Then the match ratio is `m = effective / ideal`.

For the DeltaRho: 1x1 gives 3.76 / 6.36 = 0.59, 2x2 gives 7.52 / 6.36 = 1.18. 2x2 is closer on the ratio scale, so m = 1.18.

**Grading.** PASS if 0.75 <= m <= 1.33 with no bin or 2x2. WARN if the match needs 3x3 or 4x4 (CMOS read-noise penalty, section 2.5), or if no bin gets within range.

---

## Validate it yourself

* **By hand.** Reproduce the plate scale, pixels across, best bin and ideal pixel for the RASA 11 (620 mm) with the IMX455. You should get 1.251 "/px, 2.00 pixels across, 1x1 and 3.8 um.
* **Script.** `python3 docs/learning/check_examples.py` (section `02-seeing-and-sampling.md`) checks every number above, including the SNR table.
* **Unit tests.**

  ```bash
  cargo test plate_scale_deltarho350_imx455
  cargo test plate_scale_rasa11_imx455
  cargo test best_bin_choices
  cargo test ideal_pixel_values
  cargo test closest_binned_pixel_matches_on_ratio_scale
  cargo test sampling_footprint_and_read_noise_penalty
  ```

* **Tool output.** Run `cargo run --release -- --demo` and read checks 2 and 3 for the DeltaRho 350 + IMX455. Then change the seeing in the interactive menu (*Change site conditions*) to 1.5" and watch the sampling verdict change.

## Self-check

1. A telescope has a 2000 mm focal length and a camera with 9 um pixels. What is the plate scale?
2. At 3" seeing, how many pixels across a star is that? Is it well sampled?
3. Why is the target about 2 pixels across, not 10?
4. A star covers 36 pixels on a CMOS camera with R = 3 e-, and gives 900 electrons. Ignoring sky, what is the SNR? What would it be if the star covered only 4 pixels?
5. What pixel size does a 400 mm focal length want at 2" seeing?

<details>
<summary>Answers</summary>

1. 206.265 x 9 / 2000 = 0.928 "/px.
2. 3 / 0.928 = 3.23 pixels across. Mildly oversampled: PASS, with 2x2 suggested (1.62 across).
3. Finer pixels record no extra detail, because the atmosphere has already blurred it away, but each extra pixel adds read noise.
4. 900 / sqrt(900 + 36 x 9) = 900 / 35.0 = 25.7. With 4 pixels: 900 / sqrt(900 + 36) = 900 / 30.6 = 29.4.
5. (2 / 2) x 400 / 206.265 = 1.94 um. Very few cameras have pixels that small, so short focal lengths tend to be undersampled in good seeing.

</details>

## Where it lives in the code

* `src/calculations/optics.rs`: `plate_scale_arcsec_per_px`, `pixels_across_star`, `best_bin`, `closest_binned_pixel_um`, `star_footprint_px`, `read_noise_variance_penalty`, `ideal_pixel_um`.
* `src/checks.rs`: `check_sampling` and `check_ideal_pixel` apply the thresholds.
* `src/constants.rs`, module `checks_limits`: `SAMPLING_*`, `MAX_BIN`, `PIXEL_MATCH_*`.

**Next:** [Lesson 3: Optics and focus](03-optics-and-focus.md)
