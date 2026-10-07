# 6. Light, field, focus and fit checks (checks 5 to 8)

**What you'll learn:** how much light a telescope really collects once the central obstruction is subtracted, how to compare survey speed between configurations, why fast telescopes are hard to focus, and how to check that the mount and camera train physically fit.

**Before you start:** [Image-quality checks](05-image-quality-checks.md), for the small-angle rule and the sensor dimensions, and [How an evaluation works](04-how-an-evaluation-works.md#the-reference-configuration), for the reference configuration.

The running example is again the **DeltaRho 350 + IMX455**. Checks 5 and 6 compare it with the **RASA 11 + IMX455**.

---

## Check 5: Collecting area and depth

**Question.** How much light does the telescope actually collect, and how much fainter can it see than the reference?

**Step 1: geometric area.** The area of a circle of diameter D:

```
geometric area = pi/4 x D^2
```

**Step 2: subtract the obstruction.** Area goes with diameter squared. When the obstruction is quoted as a fraction of the diameter, it must be squared too to become a fraction of the area. A "56% obstruction by diameter" blocks only 0.56^2 = 31% of the light. If the spec sheet quotes it by area, do **not** square it.

```
blocked fraction    = obstruction_by_diameter^2      (or obstruction_by_area, used as-is)
effective area      = geometric area x (1 - blocked fraction)
equivalent aperture = 2 x sqrt(effective area / pi)
```

The equivalent aperture is the diameter of an unobstructed telescope with the same effective area.

**Step 3: convert the area ratio to magnitudes.** By the definition of the magnitude scale, a brightness ratio r corresponds to 2.5 x log10(r) magnitudes:

```
depth vs reference (mag) = 2.5 x log10(effective area / reference area)
```

A rule of thumb: 30% more light is about a quarter magnitude deeper, and twice the light is about 0.75 magnitude.

**Worked example.**

```
DeltaRho 350:  pi/4 x 0.350^2 = 0.0962 m^2,  blocked 0.56^2 = 0.314,  effective 0.0660 m^2
RASA 11:       pi/4 x 0.279^2 = 0.0611 m^2,  blocked (114/279)^2 = 0.167,  effective 0.0509 m^2

depth difference = 2.5 x log10(0.0660 / 0.0509) = 2.5 x log10(1.30) = +0.28 mag
```

Despite being 14 inches against 11, the DeltaRho reaches only about 0.28 magnitude deeper, because of its larger central obstruction.

**Status.** Always INFO. Depth is a trade-off to weigh, not a pass/fail property.

---

## Check 6: Field of view and search speed

**Question.** How much sky does each frame cover, and how fast can this configuration survey the sky compared to the reference?

**Step 1: true field of view.** The angle the sensor subtends on the sky, for each axis. The report gives it two ways:

```
method 1, exact:        TFOV (deg) = 2 x atan(sensor size (mm) / (2 x FL_mm)) x 57.2958
method 2, small-angle:  TFOV (deg) = sensor size (mm) / FL_mm x 57.2958
field area (deg^2)    = TFOV_width x TFOV_height                (method 2)
```

Method 1 is exact for a flat sensor at the focal plane. Method 2 replaces atan(x) with x, which overstates the field by about x^2 / 12 with x = size / FL. The rest of the evaluation uses method 2, and the report prints how far it is from method 1. For the DeltaRho 350 + IMX455 the width is 1.96456 deg exactly and 1.96475 deg by the approximation, 0.0098% apart; even the fast RASA 11 is only 0.03% apart.

**Step 2: etendue.** Multiply how deep each look goes (area) by how much sky each look covers (field):

```
etendue                   = effective area (m^2) x field area (deg^2)
search speed vs reference = etendue / reference etendue
```

A useful analogy is mowing a lawn: field of view is the width of the mower, and aperture is the power of the engine.

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

---

## Check 7: Focus tolerance

**Question.** How precisely must the sensor sit at focus?

**Formula.**

```
CFZ (um) = +/- 2.44 x wavelength_um x N^2        (wavelength = 0.55 um, N = FL / D)
```

**Why N squared.** Two effects each contribute a factor of N.

1. The smallest spot the optics can form (the diffraction-limited Airy disk) is about 2.44 x wavelength x N wide. Slower optics have a larger acceptable blur.
2. The cone of light converging to focus has a slope of about 1/N. Slower optics have a gentler cone, so moving the sensor along the axis enlarges the blur more slowly.

Tolerance = acceptable blur / cone slope = (wavelength x N) x N, hence N squared. Halving the focal ratio quarters the focus tolerance.

**Worked examples.**

| Telescope | f-ratio | CFZ |
|---|---|---|
| RASA 11 | f/2.22 | +/- 6.6 um |
| DeltaRho 350 | f/3.0 | +/- 12.1 um |
| CDK14 | f/7.2 | +/- 69.6 um |

For scale, a human hair is about 70 um thick. Fast systems drift out of focus with small temperature changes, and a sensor tilted by a few micrometers across its width will be sharp on one side and soft on the other.

**Thresholds.** PASS if 40 um or more (forgiving). PASS with a note if between 15 and 40 um (motorized focuser recommended). WARN below 15 um (needs a motorized focuser, temperature-compensated autofocus and a sensor tilt adjuster). This check never fails, because tight focus is a requirement to plan for, not a disqualifier.

---

## Check 8: Practical fit

**Question.** Can the mount carry the payload with margin, and is there enough back focus for the camera train?

**Formulas.**

```
payload       = OTA weight + camera weight + accessories weight
load fraction = payload / mount rating

back focus OK if available >= required
```

**Why 70%.** A mount rated for a given payload is rarely stiff at that limit. Wind, vibration and fast slews all behave better with margin. Keeping the payload at or below about 70% of the rating is a common rule of thumb.

**Worked example.** DeltaRho 350 (46 lb) + IMX455 camera (2 lb) + 10 lb of accessories = 58 lb on a 100 lb mount = 58% of rating. PASS.

**Thresholds.** PASS at 70% or less. WARN up to 90%. FAIL above 90%. Back focus is PASS if available meets the requirement, FAIL if short. If nothing can be evaluated, the status is INFO.

**Which blanks matter.** The OTA weight and the mount rating are both needed for the payload grade; if either is blank, the payload part is skipped and the report says so. A blank camera or accessory weight counts as zero, so the grade still runs, but it may read lighter than reality. Back focus is graded only when both the available and the required figure are entered.

**Back-focus caution.** Vendors measure back focus from different reference points, for example "from the mounting surface without focuser" versus "with focuser installed at mid-travel." Make sure the requirement you enter is measured from the same reference as the telescope's figure.

---

## Check it yourself

**By hand.** Work these for the **CDK14** (356 mm aperture, 2563 mm focal length, 48.5% obstruction by diameter) with the IMX455, using the DeltaRho 350 as the reference.

1. **Effective area.** pi/4 x 0.356^2 = 0.0995 m^2. Blocked 0.485^2 = 0.235. Effective = 0.0995 x (1 - 0.235) = **0.0761 m^2**.
2. **Depth vs the DeltaRho.** 2.5 x log10(0.0761 / 0.0660) = **+0.15 mag**.
3. **By area versus by diameter.** If a spec sheet said "23.5% by area" for the same telescope, you would get the same 0.0761 m^2 without squaring. Reading the 48.5% as if it were by area would wrongly give 0.0513 m^2.
4. **Field.** 36.0 / 2563 x 57.3 = 0.805 deg and 24.0 / 2563 x 57.3 = 0.537 deg, so **0.43 deg^2**.
5. **Search speed.** Etendue = 0.0761 x 0.432 = 0.0329. Versus the DeltaRho: 0.0329 / 0.170 = **0.19x**. Deeper per look, but over five times slower to survey.
6. **Focus.** N = 2563 / 356 = 7.2. CFZ = 2.44 x 0.55 x 7.2^2 = **+/- 69.6 um**, forgiving.
7. **Payload on a smaller mount.** The DeltaRho 350 payload (58 lb) on the iOptron HAE69C (69 lb rating) is 58 / 69 = **84%**, so WARN.

**Against the tests.**

| Concept | Command |
|---|---|
| Effective area and depth | `cargo test effective_area_and_depth` |
| By-area equals by-diameter when converted | `cargo test obstruction_by_area_matches_by_diameter` |
| Equivalent aperture and etendue | `cargo test area_equivalent_aperture_and_etendue` |
| Field of view | `cargo test field_of_view_example` |
| Critical focus zone | `cargo test critical_focus_zone_values` |
| Payload and back focus margins | `cargo test payload_and_back_focus_margins` |

**In the tool.** Run `cargo run --release -- --demo`. In the comparison table, the CDK14 + IMX455 row should read `FOVdeg2` 0.43, `Area m2` 0.0761, `dMag` +0.15, `Search` 0.19x and `CFZ+/-` 69.6. The DeltaRho 350 row should show `Load%` 58.

Next: [Motion and timing](07-motion-and-timing.md).
