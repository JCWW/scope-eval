# 4. How an evaluation works

**What you'll learn:** the fixed pipeline the tool runs for every configuration, why one configuration is the "reference," and how individual results roll up into a component and overall status. Pages 5 to 10 then open up each step.

## The pipeline

For each configuration the tool runs the same sequence:

```
1. Read site assumptions        seeing FWHM, reference wavelength (0.55 um)
2. Derive basic geometry        f-ratio = FL / D
                                sensor width, height, diagonal (mm) = pixels x pixel size
3. Run the eight checks         each returns: status, detail lines, verdict
     1  Sensor fit              image circle vs sensor diagonal
     2  Sampling                plate scale vs star FWHM, recommended bin
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
     Mount      tracking rate, keyhole, acceleration, slew and settle,
                non-sidereal tracking
     System     target detection (signal-to-noise and limiting magnitude)
     -> worst status per component, and overall
6. Store headline metrics for the comparison tables
```

| Step | Explained on |
|---|---|
| 3, checks 1 to 4 | [Image-quality checks](05-image-quality-checks.md) |
| 3, checks 5 to 8 | [Light, field, focus and fit checks](06-light-field-focus-fit-checks.md) |
| 4 | [Motion and timing](07-motion-and-timing.md) |
| 5, telescope, camera, mount rate | [Orbital regimes](08-orbital-regimes.md) |
| 5, mount acceleration and slew | [Mount dynamics](09-mount-dynamics.md) |
| 5, system | [Target brightness and detection](10-target-brightness-and-detection.md) |
| 6 | [Comparing configurations](12-comparing-configurations.md) |

**Later steps reuse earlier results.** The regime checks in step 5 use the plate scale and recommended bin from check 2, the field of view from check 6 and the effective area from check 5. That's why understanding the eight checks first makes the regime results easy to follow.

## The reference configuration

Checks 5 and 6 report depth and search speed as ratios, because "0.066 m^2" means little on its own while "+0.28 magnitudes deeper than the RASA" is immediately useful. Ratios need something to compare against, so the **reference is always the first configuration in the session**. When you change the seeing, every stored configuration is re-evaluated in order, so the reference rule stays consistent. The rule is enforced in one place, `evaluate_all` in `src/checks.rs`.

## What the statuses mean

| Status | Meaning |
|---|---|
| PASS | Meets the rule of thumb for this check. |
| WARN | Usable, but there is a trade-off or a question to resolve with the vendor. |
| FAIL | A real mismatch for this application. |
| INFO | A measurement for comparison, not a pass/fail judgment (checks 5 and 6), or not enough data to judge. |

## How statuses roll up

A component's status in a regime is its **worst graded check**. The ordering is PASS < WARN < FAIL, and INFO never makes a status worse. The regime's overall status is the worst of its components.

For example, if the camera's checks for LEO are timestamp PASS, shutter skew FAIL and exposure INFO, the camera's LEO status is FAIL. If the telescope is PASS, the mount WARN and the system WARN, the LEO overall status is FAIL.

The thresholds behind PASS, WARN and FAIL are engineering rules of thumb, not physical laws. They are collected in one place so you can change them ([Thresholds](13-thresholds.md)).

## Physics versus judgment

The code keeps two concerns apart, and it helps to keep them apart when reading these docs too:

* **Calculations** (`src/calculations/`) contain physics and math only: plate scale, rates, signal-to-noise. Every page shows these as formulas you can redo by hand.
* **Judgments** (`src/checks.rs`, `src/regimes.rs`) compare those numbers with thresholds and write the verdict.

If you disagree with a verdict, the question is usually whether the threshold suits your mission, not whether the arithmetic is right. Run with `--equations` (or choose **Report style** in the menu) to see both under every check: the equations with the values substituted, then a `Rule:` line naming the threshold bands and which one the result fell in.

## Check it yourself

1. **Roll-up.** Run `cargo run --release -- --demo` and look at the compact regime table for DeltaRho 350 + IMX455. For each row, confirm that the Overall column equals the worst of the Telescope, Camera, Mount and System columns.
2. **The reference rule.** In the interactive menu, evaluate the RASA 11 + IMX455 first and the DeltaRho 350 + IMX455 second. The DeltaRho's depth should read about +0.28 mag. Restart and evaluate them in the opposite order: now the RASA should read about -0.28 mag. Same physics, different reference.
3. **INFO doesn't count.** In the demo's comparison table, checks 5 and 6 show `i` for every configuration. They are measurements for comparison, and the regime statuses never use them as grades.
4. **System is a component.** `cargo test system_is_a_component` checks that the detection result rolls up like the other three.

Next: [Image-quality checks](05-image-quality-checks.md).
