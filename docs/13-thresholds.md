# 13. Thresholds and how to tune them

**Reference page.** Every judgment threshold, its default, the check that uses it and what it means. All of them live in `src/constants.rs`, in two modules: `checks_limits` for the eight general checks and `regimes_limits` for the orbital-regime checks. Physical constants and default assumptions sit at the top of the same file.

These are engineering rules of thumb, not physical laws. If your mission needs a stricter or looser rule, change the constant and rebuild; the calculator tests will still confirm the physics, and the evaluation tests show which verdicts moved.

## General checks (`checks_limits`)

| Constant | Default | Check | Meaning |
|---|---|---|---|
| `FIT_WARN_FRACTION` | 0.90 | 1 | Image circle may be this fraction of the diagonal before failing |
| `FIT_HEADROOM_FACTOR` | 1.15 | 1 | Above this ratio of image circle to diagonal, the report notes room for a larger sensor |
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

## Orbital-regime checks (`regimes_limits`)

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
| `SATURATION_WARN_FRACTION` | 0.8 | System | Peak pixel above this fraction of full well is nonlinear |
| `MIN_PRACTICAL_EXPOSURE_S` | 0.001 s | System | Saturating even at this exposure is a FAIL |

## Default assumptions

These are substituted when an input is left blank. QE, throughput, sky brightness, read noise, full well, pointing and settle time are named in the report when assumed. Seeing and timestamp accuracy are only prompt defaults, the wavelength is never flagged, and the reference target shows up only as a magnitude marked "(derived)".

| Constant | Default | Used for |
|---|---|---|
| `DEFAULT_SEEING_ARCSEC` | 2.5" | Seeing prompt default |
| `DEFAULT_WAVELENGTH_UM` | 0.55 um | Focus tolerance (green light, near the eye's and many sensors' peak sensitivity) |
| `DEFAULT_POINTING_RMS_ARCSEC` | 60" | Mount pointing error for acquisition |
| `DEFAULT_TIMESTAMP_MS` | 20 ms | Timestamp prompt default (PC clock plus USB) |
| `REFERENCE_TARGET_CROSS_SECTION_M2` | 10 m^2 | Derived target magnitude |
| `REFERENCE_TARGET_ALBEDO` | 0.2 | Derived target magnitude |
| `DEFAULT_PHASE_FACTOR` | 1.0 (full phase) | Derived target magnitude |
| `DEFAULT_QE` | 0.80 | Detection |
| `DEFAULT_THROUGHPUT` | 0.85 | Detection |
| `DEFAULT_SKY_MAG_ARCSEC2` | 21.0 | Detection |
| `DEFAULT_READ_NOISE_E` | 3.0 e- | Detection |
| `DEFAULT_FULL_WELL_E` | 20,000 e- | Saturation |
| `MAX_EXPOSURE_S` | 30 s | Exposure cap |
| `DEFAULT_SLEW_DISTANCE_DEG` | 90 deg | Slew and settle |
| `DEFAULT_SETTLE_TIME_S` | 2.0 s | Slew and settle |

`PEAK_ACCEL_COEFF` (3 sqrt(3) / 8) and `PHOTONS_M2_S_MAG0` (8.9e9) are derived constants, not thresholds, and should not be tuned. [Mount dynamics](09-mount-dynamics.md#step-1-peak-tracking-acceleration) and [detection](10-target-brightness-and-detection.md#step-2-photons-from-the-target-and-from-the-sky) derive them.

The `plausible_ranges` module sets the bounds outside which a hand-entered value is treated as not entered ([Inputs](03-inputs.md#values-that-are-range-checked)).

The regime parameters themselves (ranges, rates, prediction errors) are not thresholds; they are in `regimes()` in `src/regimes.rs`.

## Check it yourself

1. Open `src/constants.rs` and confirm each default above matches the file.
2. Change `SAMPLING_OVER_FAIL` from 6.0 to 10.0, rebuild and rerun `--demo`. The CDK14 (8.4 px across) and CDK17 (9.95 px across) should move from `F` to `W` on check 2. Run `cargo test` too: the calculator tests should still pass, because only a judgment changed. Revert the change afterwards.
