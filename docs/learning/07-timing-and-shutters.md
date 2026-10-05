# Lesson 7: Timing and shutters

A satellite's measured position is only as good as the time attached to it. A moving target that is 10 ms late is in the wrong place. This lesson turns timing errors, rolling-shutter readout and long exposures into position errors, first for the GEO case (the tool's ungraded "timing reference") and then for every regime (the graded camera checks).

**Prerequisites:** [Lesson 2](02-seeing-and-sampling.md) (binned plate scale), [Lesson 6](06-orbits-and-angular-rates.md) (rate vs stars).

## You will be able to

* Convert a timestamp error into a position error for any regime.
* Work out how accurately each regime needs its images timestamped.
* Explain what a rolling shutter is, compute its skew, and know when it can be corrected.
* Compute how long an exposure can be before something trails.

## Key terms

| Term | Meaning |
|---|---|
| **Sidereal rate** | The rate at which the stars appear to move across the sky: 15.04"/s. |
| **Timestamp accuracy** | How far the time recorded for an exposure can be from the true time. Includes clock error and latency. |
| **Along-track error** | Position error along the target's direction of motion. Timing errors produce only this kind. |
| **Rolling shutter** | A sensor that exposes and reads out one row at a time, so the bottom row is captured later than the top. |
| **Global shutter** | A sensor that captures every row at the same instant. |
| **Line time** | The delay between reading one row and the next, in microseconds. |
| **Skew** | The distortion a rolling shutter adds to a moving scene: the top and bottom of the frame see it at different times. |
| **Trailing** | Smearing of a point source into a streak during an exposure because it moves relative to the mount. |

---

## 7.1 The core relationship

Every calculation in this lesson is the same idea:

```
position error (") = rate vs stars ("/s) x time error (s)
```

A target moving against the stars at a given rate is in the wrong place by that rate times however wrong your clock is. Only the rate vs stars matters, because the position is measured against the stars in the image.

## 7.2 The GEO stare-mode timing reference (ungraded)

The tool prints these numbers after the eight checks for every configuration. They illustrate the ideas with GEO, where the mount is stopped and the stars drift past at the sidereal rate.

**Star drift.**

```
sidereal rate = 1,296,000" / 86,164.09 s = 15.04 "/s
star streak per second (px) = 15.04 / binned plate scale
```

For the DeltaRho 350 binned 2x2: 15.04 / 1.48 = **10 pixels per second of exposure**. The GEO belt lies near the celestial equator, so the cos(declination) factor that slows the stars at other declinations is close to 1 and ignored.

**Timing sensitivity.**

```
position error = 15.04 x timing error
```

A 10 ms timing error gives 15.04 x 0.010 = **0.15"** of along-track error. That's a tenth of a binned pixel: fine for GEO. Now compare LEO, moving about 3,140"/s: the same 10 ms gives 31", or 21 binned pixels.

A PC clock plus USB latency can easily be off by tens of milliseconds. Hardware GPS timestamping is typically 0.1 ms or better.

## 7.3 Required timestamp accuracy (graded)

**Idea.** Set a budget: the target shouldn't move more than a quarter of a binned pixel in the time uncertainty.

**Formulas.**

```
required timing (s) = 0.25 x binned plate scale (") / rate vs stars ("/s)
position error (")  = rate vs stars x timestamp accuracy
```

**Why a quarter pixel.** Centroiding a well-sampled star routinely finds its center to a fraction of a pixel. A timing error larger than that would swamp the measurement you worked to make precise.

**Worked example (DeltaRho 350, binned 1.48 "/px).**

| Regime | Rate vs stars | Required timing |
|---|---|---|
| LEO | 3,140 "/s | 0.12 ms |
| MEO | 40 "/s | 9.3 ms |
| GEO | 15.04 "/s | 24.6 ms |
| HEO | 7.8 "/s | 48 ms |
| Cislunar | 0.55 "/s | 670 ms |

Check GEO yourself: 0.25 x 1.48 / 15.04 = 0.0246 s.

A PC clock with USB latency (tens of ms) is borderline even for GEO and hopeless for LEO, which needs GPS hardware timestamping.

**Grading.** PASS if your timestamp accuracy meets the requirement. WARN if within 4x of it. FAIL beyond that.

## 7.4 Rolling-shutter skew (graded)

**Idea.** Most CMOS sensors don't capture the whole frame at once. They start and read each row a fixed *line time* after the one before. The bottom row sees the sky later than the top row. If something is moving against the stars, it is in a different place for each row.

**Formulas.**

```
readout time (s) = rows x line time
skew (")         = rate vs stars x readout time
per-row time     = t(first row) + row index x line time
```

**Worked example (IMX455 in a Moravian C3-61000).**

```
readout = 6,388 rows x 39.028 us = 0.249 s
GEO skew = 15.04 x 0.249 = 3.75"
```

In GEO stare mode, the stars at the bottom of the frame are recorded a quarter second after those at the top, so they're displaced by 3.75" (2.5 binned pixels). If the reduction software uses one timestamp for the whole frame, the astrometric solution soaks up this skew as a false distortion.

**The fix** is to give every row its own time with the per-row formula. That works as long as the skew is a small part of the frame. When the skew is a large fraction of the frame, the frame geometry is badly distorted and a global-shutter camera, or reading only a small region of the sensor, is the better answer.

| Regime | Skew | As a fraction of the DeltaRho 350's 4,718" frame height | Status |
|---|---|---|---|
| LEO | 783" | 16.6% | FAIL |
| MEO | 9.9" | 0.2% | WARN (correctable) |
| GEO | 3.75" | 0.1% | WARN (correctable) |
| Cislunar | 0.14" | negligible | PASS |

**Grading.** PASS for a global shutter, or if the skew is under a quarter of a binned pixel. WARN if larger but under 10% of the frame height: correctable with per-row timestamps. FAIL above 10%. WARN if the line time is unknown.

## 7.5 Exposure vs trailing (INFO)

**Idea.** The target and the stars move relative to each other, so one of them always smears in a long exposure: the target if the mount follows the stars, the stars if the mount follows the target.

**Formulas.**

```
crossing time (s)       = seeing FWHM (") / rate vs stars ("/s)
streak per second (px)  = rate vs stars / binned plate scale
```

The crossing time is how long it takes the moving object to move by one seeing disk: the longest exposure before it starts to look like a streak rather than a star.

**Worked examples.**

* **LEO:** 2.5 / 3,140 = 0.8 ms. No useful exposure is that short, which is why LEO observing means rate tracking with streaked stars.
* **Cislunar:** 2.5 / 0.55 = 4.5 s. A sidereally tracked exposure of a few seconds keeps the target point-like. [Lesson 9](09-brightness-and-detection.md) uses this as its exposure time.

---

## Validate it yourself

* **By hand.** Compute the required timing and the rolling-shutter skew for HEO (7.8"/s). Grade the shutter: is the skew under a quarter of a binned pixel?
* **Script.** `python3 docs/learning/check_examples.py` (section `07-timing-and-shutters.md`).
* **Unit tests.**

  ```bash
  cargo test timing_budget_is_fraction_of_a_moving_pixel
  cargo test rolling_shutter_skew_matches_sensor_spec
  cargo test geo_timing_requirement
  ```

* **Tool output.** Every evaluation prints a `Timing reference, GEO stare mode` block with the numbers from 7.2 and 7.4. The detailed regime breakdown in `--demo` prints the timing, shutter and trailing checks for each regime. Compare the RASA 11 + IMX174 (global shutter) with the IMX455 rows.

## Self-check

1. Your timestamps are good to 20 ms. What position error does that cause for a MEO target? For LEO?
2. A sensor has 3,000 rows at 10 us per row. What is the LEO skew? What fraction of a 1-degree frame is that?
3. Why is the IMX455's rolling-shutter skew a FAIL for LEO but only a WARN for GEO, when it's the same sensor?
4. Why do LEO observers accept streaked stars?

<details>
<summary>Answers</summary>

1. MEO: 40 x 0.020 = 0.8". LEO: 3,140 x 0.020 = 63".
2. Readout 0.03 s. Skew 3,140 x 0.03 = 94". A 1-degree frame is 3,600", so 2.6%: correctable (WARN).
3. The skew is rate x readout time. LEO moves about 200 times faster against the stars than GEO, so the same readout time gives 200 times the skew, enough to distort the frame beyond correcting.
4. A LEO target crosses a seeing disk in under a millisecond, so either it or the stars must streak. Rate tracking keeps the target, which is what you're measuring, point-like and bright.

</details>

## Where it lives in the code

* `src/calculations/camera.rs`: `timing_budget_s`, `position_error_arcsec`, `rolling_readout_time_s`, `rolling_shutter_skew_arcsec`, `crossing_time_s`, `pixels_per_second`.
* `src/checks.rs`: `geo_motion_and_timing` (the timing reference block).
* `src/regimes.rs`: `camera_timing`, `camera_shutter`, `camera_trailing`.
* `src/constants.rs`, module `regimes_limits`: `TIMING_PIXEL_FRACTION`, `TIMING_WARN_MULTIPLE`, `SKEW_NEGLIGIBLE_PX`, `SKEW_FAIL_FRAME_FRACTION`.

**Next:** [Lesson 8: Mount dynamics](08-mount-dynamics.md)
