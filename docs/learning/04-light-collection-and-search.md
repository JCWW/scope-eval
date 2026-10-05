# Lesson 4: Light collection and search speed

How much light does a telescope collect, and how fast can it sweep the sky? This lesson covers checks 5 and 6. Both are always graded INFO, because depth and coverage are trade-offs you weigh against each other, not pass/fail properties.

**Prerequisites:** [Lesson 1](01-angles-and-magnitudes.md) (magnitudes), [Lesson 3](03-optics-and-focus.md) (aperture, focal length).

## You will be able to

* Compute a telescope's effective collecting area, correctly handling a central obstruction quoted by diameter or by area.
* Turn an area ratio into a depth difference in magnitudes.
* Compute field of view and field area from sensor size and focal length.
* Explain etendue and use it to compare how fast two configurations survey the sky.

## Key terms

| Term | Meaning |
|---|---|
| **Central obstruction** | Anything in the middle of the incoming beam (secondary mirror, corrector, camera) that blocks light. |
| **By diameter / by area** | Two ways of quoting the obstruction: as a fraction of the aperture's diameter or of its area. |
| **Effective area** | Collecting area after subtracting the obstruction. |
| **Equivalent aperture** | The diameter of an unobstructed telescope with the same effective area. |
| **Depth** | How faint an object the system can detect, compared here in magnitudes against a reference. |
| **Field of view (FOV)** | The sky angle covered by the sensor, per axis. |
| **Etendue** | Effective area x field area. The standard figure of merit for survey speed. |
| **Reference configuration** | The first configuration evaluated in a session. Depth and search speed are reported relative to it. |

---

## 4.1 Effective collecting area (check 5)

**Formulas.**

```
geometric area      = pi/4 x D^2
blocked fraction    = obstruction_by_diameter^2      (or obstruction_by_area, used as-is)
effective area      = geometric area x (1 - blocked fraction)
equivalent aperture = 2 x sqrt(effective area / pi)
```

**Why square the obstruction.** Area goes with diameter squared. An obstruction 56% of the aperture's *diameter* covers 0.56^2 = 31.4% of its *area*. If the spec sheet already quotes it by area, using it as-is is correct; squaring it again would be wrong.

This is one of the most common spec-sheet traps. A 49% obstruction by diameter is only 24% by area. Quoting by area makes a telescope look better, so always check which one you're reading.

**Worked example.**

```
DeltaRho 350:  pi/4 x 0.350^2 = 0.0962 m^2   blocked 0.56^2       = 0.314   effective 0.0660 m^2
RASA 11:       pi/4 x 0.279^2 = 0.0611 m^2   blocked (114/279)^2  = 0.167   effective 0.0509 m^2
```

The DeltaRho 350's equivalent aperture is 2 x sqrt(0.0660 / pi) = 0.290 m: it collects as much light as an unobstructed 290 mm telescope.

## 4.2 Depth vs reference

**Formula.**

```
depth vs reference (mag) = 2.5 x log10(effective area / reference area)
```

**Why.** At equal exposure time, the signal is proportional to collecting area. Lesson 1's definition of the magnitude scale turns the area ratio into magnitudes.

**Worked example.** DeltaRho 350 against RASA 11:

```
2.5 x log10(0.0660 / 0.0509) = 2.5 x log10(1.30) = +0.28 mag
```

A 14-inch telescope reaches only about a quarter magnitude deeper than an 11-inch one, because its bigger central obstruction eats most of its advantage.

**Caveat.** "Same area means same depth" assumes everything else is equal: exposure time, sampling, optical throughput, camera QE. [Lesson 9](09-brightness-and-detection.md) computes an absolute limiting magnitude that accounts for all of them.

## 4.3 Field of view (check 6)

**Formula.** The small-angle rule applied to the whole sensor:

```
FOV (deg)         = sensor size (mm) / FL_mm x 57.2958        (for each axis)
field area (deg^2) = FOV_width x FOV_height
```

**Worked example.**

```
DeltaRho 350:  36.0 / 1050 x 57.3 = 1.96 deg,   24.0 / 1050 x 57.3 = 1.31 deg   ->  2.58 deg^2
RASA 11:       36.0 / 620  x 57.3 = 3.33 deg,   24.0 / 620  x 57.3 = 2.22 deg   ->  7.39 deg^2
```

The tool also reports how many non-overlapping fields tile 100 deg^2: 100 / 2.58 = about 39 for the DeltaRho.

## 4.4 Etendue and search speed

**Formula.**

```
etendue                     = effective area (m^2) x field area (deg^2)
search speed vs reference   = etendue / reference etendue
```

**Why it measures survey speed.** To survey a patch of sky to a given depth, you need to cover it (field) and you need to collect enough light per look (area). Doubling either halves the time. An analogy: mowing a lawn, field of view is the width of the mower and aperture is the power of the engine.

**Worked example.**

```
DeltaRho 350:  0.0660 x 2.58 = 0.170
RASA 11:       0.0509 x 7.39 = 0.376

RASA search speed vs DeltaRho = 0.376 / 0.170 = 2.2x
```

The RASA sweeps sky 2.2 times faster, but each look is 0.28 mag shallower. "Search speed" compares each configuration at its own depth. Neither is better in general: a search mission favors the RASA, a characterization mission favors depth.

## 4.5 The reference rule

Depth and search speed only make sense relative to something. The tool uses the **first configuration you evaluate** as the reference. If you change the seeing, every stored configuration is re-evaluated in order, so the reference doesn't change under you. Evaluate your current or baseline system first.

---

## Validate it yourself

* **By hand.** Compute the effective area of the CDK14 (356 mm, 48.5% obstruction by diameter). Then its depth against the DeltaRho 350. You should get 0.0761 m^2 and +0.15 mag, matching the `--demo` comparison table.
* **Script.** `python3 docs/learning/check_examples.py` (section `04-light-collection-and-search.md`).
* **Unit tests.**

  ```bash
  cargo test effective_area_and_depth
  cargo test obstruction_by_area_matches_by_diameter
  cargo test field_of_view_example
  cargo test area_equivalent_aperture_and_etendue
  ```

* **Tool output.** In `--demo`, checks 5 and 6 for the RASA 11 + IMX455 print "Depth vs reference -0.28 mag" and "Search speed vs reference x2.21".

## Self-check

1. A spec sheet says "obstruction: 20% by area". What is it by diameter?
2. Telescope A has three times the effective area of telescope B. How much deeper does it go?
3. Two configurations have the same etendue, but one has twice the area and half the field. When would you prefer each?
4. Why are checks 5 and 6 never PASS or FAIL?

<details>
<summary>Answers</summary>

1. sqrt(0.20) = 44.7% by diameter.
2. 2.5 x log10(3) = 1.19 mag.
3. Same survey speed. The bigger aperture is better for faint targets you must detect in one look (cislunar, small debris). The wider field is better for catching targets with uncertain positions (LEO acquisition) and for covering large areas quickly.
4. More depth or more field is never wrong in itself. Whether it matters depends on the mission, so the tool reports the numbers and leaves the judgment to you.

</details>

## Where it lives in the code

* `src/calculations/optics.rs`: `geometric_area_m2`, `effective_area_m2`, `equivalent_aperture_mm`, `delta_mag`, `fov_deg`, `field_area_deg2`, `etendue_m2_deg2`.
* `src/checks.rs`: `check_area_depth`, `check_field_and_search`, and `evaluate_all` (the reference rule).
* `src/model/optics.rs`: `Obstruction::ByDiameter` and `Obstruction::ByArea`.

**Next:** [Lesson 5: Practical fit](05-practical-fit.md)
