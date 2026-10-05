# Lesson 11: How the tool judges

The previous lessons covered the physics. This one covers the judgment: the order the tool computes things in, how a number becomes PASS, WARN, FAIL or INFO, how those grades roll up into the regime table, and which numbers are laws of physics versus rules of thumb you're free to change.

**Prerequisites:** Lessons 1 to 9. This lesson ties them together.

## You will be able to

* Trace the evaluation pipeline and say which later checks reuse which earlier results.
* Explain what each status means, and why INFO never makes a result worse.
* Read the regime table and the comparison table and name the component behind each warning.
* Tell a derived constant from a tunable threshold, and know which ones to change.
* Explain why the tool refuses to grade PASS on numbers it assumed.

## Key terms

| Term | Meaning |
|---|---|
| **Check** | One question with a status, detail lines and a one-sentence verdict. |
| **Component** | Telescope, camera, mount, or the system as a whole (detection). |
| **Worst-status rule** | A component's status in a regime is the worst of its graded checks. |
| **Reference configuration** | The first configuration in a session; depth and search speed are relative to it. |
| **Derived constant** | A number that follows from physics or geometry (206,265, 3 sqrt(3) / 8). Not to be tuned. |
| **Threshold** | A rule-of-thumb boundary between PASS, WARN and FAIL. Meant to be tuned. |
| **Plausible range** | Bounds outside which a hand-entered value is treated as a typo and ignored. |

---

## 11.1 The pipeline

For each configuration the tool runs the same fixed sequence:

```
1. Read site assumptions        seeing FWHM, reference wavelength (0.55 um)
2. Derive basic geometry        f-ratio = FL / D
                                sensor width, height, diagonal (mm) = pixels x pixel size
3. Run the eight checks         each returns: status, detail lines, verdict
     1  Sensor fit              image circle vs sensor diagonal            Lesson 3
     2  Sampling                plate scale vs seeing, recommended bin      Lesson 2
     3  Ideal pixel             the pixel size this telescope wants        Lesson 2
     4  Optics vs seeing        does the glass or the air limit the image? Lesson 3
     5  Area and depth          effective area, magnitudes vs reference    Lesson 4
     6  Field and search        field of view, etendue vs reference        Lesson 4
     7  Focus tolerance         critical focus zone                        Lesson 3
     8  Practical fit           payload vs mount rating, back focus        Lesson 5
4. GEO stare-mode timing reference (not graded)                            Lesson 7
5. For each orbital regime (LEO, MEO, GEO, HEO, cislunar):                 Lesson 6
     Telescope  acquisition field, field dwell, depth relevance            Lesson 6
     Camera     timestamp accuracy, shutter skew, exposure vs trailing     Lesson 7
     Mount      tracking rate and keyhole, acceleration, slew and settle,
                non-sidereal tracking                                      Lesson 8
     System     detection                                                  Lesson 9
     -> worst status per component, and overall
6. Store headline metrics for the comparison tables
```

**Reuse.** Step 5 reuses step 3: the plate scale and recommended bin from check 2, the field of view from check 6 and the effective area from check 5. If you change a check, every regime that depends on it changes too.

**Physics and judgment are separated.** The calculators in `src/calculations/` contain only equations and are tested against the worked examples. `checks.rs` and `regimes.rs` apply the thresholds. That's why you can check each formula on its own (as these lessons do) without worrying about how it's graded.

## 11.2 What the statuses mean

| Status | Meaning |
|---|---|
| PASS | Meets the rule of thumb for this check. |
| WARN | Usable, but there is a trade-off or a question to resolve with the vendor. |
| FAIL | A real mismatch for this application. |
| INFO | A measurement for comparison, not a pass/fail judgment (checks 5 and 6), or not enough data to judge. |

**Ordering.** Internally, INFO < PASS < WARN < FAIL. A component's status in a regime is the *maximum* (worst) of its checks, so INFO never makes a component look worse, and a single FAIL can't be hidden by passes elsewhere.

**Unknown is not the same as bad.** Many checks give WARN when an input is missing *and* it would matter (a LEO tracking rate with no slew rate entered), but INFO when it wouldn't (the same missing rate for GEO). The WARN means "ask the question", not "this will fail".

## 11.3 Never PASS on an assumption

Where the detection check had to substitute a generic QE, throughput, sky brightness or read noise, it caps its result at WARN and lists what it assumed. The reasoning: a PASS should mean "your numbers say this works", not "a number we made up says this works".

Similarly, a hand-entered value outside a **plausible range** (a QE above 1, a sky brightness of 2.1 where 21.0 was meant, a NaN) is treated as not entered, so a typo degrades the report to WARN instead of silently corrupting it.

## 11.4 Reading the regime table

```
       Regime                          Telescope  Camera   Mount    System   Overall
       LEO (Low Earth orbit)           PASS       FAIL     WARN     WARN     FAIL
       MEO (Medium Earth orbit)        PASS       WARN     WARN     WARN     WARN
       GEO (Geosynchronous orbit)      PASS       WARN     PASS     WARN     WARN
       HEO (Highly elliptical orbit)   PASS       WARN     WARN     WARN     WARN
       CIS (Cislunar space)            PASS       PASS     PASS     WARN     WARN
```

This is the running example (DeltaRho 350 + IMX455 on an L-350 with GPS timestamps). Practice reading it with what you've learned, then open the answers.

1. Why is the telescope PASS everywhere?
2. Why does the camera FAIL LEO, and WARN in MEO, GEO and HEO?
3. Where do the mount's warnings come from?
4. Why is the System column WARN everywhere?

<details>
<summary>Answers</summary>

1. Its 1.96 x 1.31 deg field gives an acquisition margin of 2.8x even for LEO (Lesson 6), and it has more than enough area for every regime.
2. Rolling-shutter skew (Lesson 7): 783" for LEO, 16.6% of the frame (FAIL); a few arcseconds elsewhere, correctable with per-row timestamps (WARN). Cislunar targets move too slowly to matter.
3. Unanswered vendor questions (Lesson 8): whether the control software supports TLE tracking, and what the axis acceleration is. Neither is a known deficiency.
4. The demo enters no QE, throughput, sky brightness or read noise, so the detection check uses defaults and won't grade PASS on them (11.3). Enter real values and it grades normally.

</details>

The fix for LEO is a global-shutter camera. In the demo's comparison table, the RASA 11 + IMX174 row shows the trade: the camera passes LEO, but the IMX174's small sensor shrinks the field enough to drop the telescope to WARN for LEO acquisition.

## 11.5 Derived constants vs thresholds

Some numbers in `src/constants.rs` are physics. Changing them would make the tool wrong:

| Constant | Value | Where it comes from |
|---|---|---|
| `ARCSEC_PER_RADIAN` | 206,264.8 | Lesson 1 |
| `SIDEREAL_RATE_ARCSEC_PER_S` | 15.04 | Lesson 6 |
| `FWHM_PER_RMS_RADIUS` | 1.665 | Lesson 3 |
| `PEAK_ACCEL_COEFF` | 3 sqrt(3) / 8 = 0.6495 | Lesson 8 |
| `SUN_APPARENT_MAG` | -26.74 | Lesson 9 |
| `PHOTONS_M2_S_MAG0` | 8.9e9 | Lesson 9 |

Others are engineering rules of thumb. They are the right thing to change if your mission or your experience disagrees: `SAMPLING_TARGET` (2 pixels across), `PAYLOAD_PASS_FRACTION` (70%), `TIMING_PIXEL_FRACTION` (a quarter pixel), `RATE_PASS_HEADROOM` (3x) and the rest. The main README's [Thresholds and how to tune them](../../README.md#thresholds-and-how-to-tune-them) lists every one with the check it affects.

A third group are assumptions about the world: the regime ranges and prediction errors, the 10 m^2 reference target, the default QE and sky. These are placeholders for your own data.

## 11.6 Choosing by mission

No configuration wins every column. Before comparing, decide which lessons matter for the job:

| Role | What decides it | Lessons |
|---|---|---|
| LEO tracking | Mount rate and keyhole, global shutter, GPS timing, acquisition field | 6, 7, 8 |
| Wide-area search (for example the GEO belt) | Search speed (etendue), then sampling | 4, 2 |
| Precision astrometry and tracking | Sampling, optics vs seeing, timing | 2, 3, 7 |
| Characterization and photometry | Depth, focus stability, back focus for filters | 4, 9, 3, 5 |
| Field portability | Weight, power, setup effort | 5 |

---

## Validate it yourself

* **Predict, then run.** Before running `cargo run --release -- --demo`, predict the regime table for the RASA 11 + IMX174 (global shutter, 5.86 um pixels, 1936 x 1216) using Lessons 6 to 8. Then compare with the demo's comparison table. Where you disagree, find the check in the detailed breakdown and work out which of you is wrong.
* **Change a threshold.** Set `SAMPLING_TARGET` to 1.5 in `src/constants.rs`, rebuild, and run the demo. Predict first which configurations change their recommended bin.
* **Unit tests.**

  ```bash
  cargo test system_is_a_component
  cargo test detection_caps_at_warn_when_inputs_are_assumed
  cargo test detection_passes_when_every_input_is_entered
  cargo test plausible_rejects_out_of_range
  cargo test cislunar_is_the_only_regime_detection_actually_grades
  ```

## Self-check

1. A component has checks graded INFO, PASS and WARN in a regime. What is its status?
2. You evaluate configuration B, then configuration A. Which one is the reference for depth?
3. Why doesn't the tool just let you set 206,265 in a config file?
4. A colleague's mount gives WARN for LEO acceleration. Does that mean it can't track LEO?

<details>
<summary>Answers</summary>

1. WARN, the worst graded status.
2. B, the first one evaluated.
3. It isn't an opinion: it's the number of arcseconds in a radian. Only thresholds and assumptions are meant to be tuned.
4. Not necessarily. Most likely the acceleration rating wasn't entered and LEO is the one regime where it matters (`ACCEL_MATTERS_DEG_S2`). It's a question for the vendor.

</details>

## Where it lives in the code

* `src/checks.rs`: `evaluate` (steps 1 to 4 and 6), `evaluate_all` (the reference rule).
* `src/regimes.rs`: `evaluate_regimes`, `RegimeEvaluation::component_status`, `RegimeEvaluation::overall`.
* `src/report/`: the reports and comparison tables, as text. `src/cli/` prints them.
* `src/constants.rs`: derived constants at the top; modules `checks_limits`, `regimes_limits` and `plausible_ranges`.

**Back to:** [the lesson index](README.md)
