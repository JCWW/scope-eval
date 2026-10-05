# Lesson 3: Optics and focus

This lesson asks three questions about the telescope's glass: does the sensor fit inside the sharp part of the image (check 1), is the glass sharp enough that the atmosphere and not the optics limits the image (check 4), and how precisely must the sensor sit at focus (check 7)?

**Prerequisites:** [Lesson 2](02-seeing-and-sampling.md) (seeing, FWHM, focal length).

## You will be able to

* Compute a sensor's diagonal and compare it with a telescope's image circle.
* Convert a vendor's RMS spot size into a FWHM you can compare with seeing.
* Combine two independent blurs in quadrature and explain why that works.
* Interpolate a spot size to the corner of your sensor.
* Derive the critical focus zone and explain why it scales with the square of the focal ratio.

## Key terms

| Term | Meaning |
|---|---|
| **Aperture (D)** | Diameter of the main light-collecting opening. |
| **Focal ratio (N, f/N)** | Focal length / aperture. Low N is "fast" (wide, bright); high N is "slow" (narrow, magnified). |
| **Image circle** | Diameter of the region at the focal plane where the image is sharp and flat. |
| **Optical axis** | The line through the center of the optics. "Off-axis" means toward the edge of the field. |
| **RMS spot size** | The root-mean-square spread of rays from a point source, from the vendor's ray-trace. May be quoted as a radius or a diameter. |
| **Gaussian** | The bell-shaped profile often used to approximate a blur. |
| **Quadrature** | Combining independent quantities as the square root of the sum of their squares. |
| **Airy disk** | The smallest spot a perfect optic can form, set by diffraction. |
| **Critical focus zone (CFZ)** | How far the sensor can move along the axis from best focus before the image visibly degrades. |

---

## 3.1 Sensor fit (check 1)

**Question.** Does the whole sensor fit inside the image circle?

**Formula.**

```
sensor width  (mm) = width_px  x pixel_um / 1000
sensor height (mm) = height_px x pixel_um / 1000
sensor diagonal    = sqrt(width^2 + height^2)

PASS if image circle >= diagonal
```

**Why the diagonal.** The sensor is a rectangle centered on the optical axis. The points farthest from the axis are its corners, which sit half a diagonal away. If the image circle is smaller than the diagonal, the corners fall outside the sharp zone.

**Worked example.** IMX455: 9576 x 3.76 / 1000 = 36.0 mm, 6388 x 3.76 / 1000 = 24.0 mm. Diagonal = sqrt(36.0^2 + 24.0^2) = 43.3 mm. The DeltaRho 350's 60 mm image circle covers it: PASS, with room for a bigger sensor.

**Grading.** PASS if the image circle covers the diagonal. WARN if it covers at least 90% of it (soft corners). FAIL below that.

## 3.2 From RMS spot to FWHM

Vendors describe sharpness with an **RMS spot size** from a ray-trace. Seeing is a **FWHM**. To compare them, you need to convert one into the other.

**Assumption.** The optical blur is roughly a round Gaussian with standard deviation sigma in each axis. For that shape:

```
FWHM          = 2 sqrt(2 ln 2) x sigma = 2.355 x sigma
RMS radius    = sqrt(2) x sigma        = 1.414 x sigma      (radius in 2D: sigma_x and sigma_y add in quadrature)

so FWHM       = (2.355 / 1.414) x RMS radius = 1.665 x RMS radius
              = 0.833 x RMS diameter
```

**Derive it yourself.** A 1D Gaussian is `exp(-x^2 / (2 sigma^2))`. Set it equal to 1/2, solve for x, and double it: x = sigma sqrt(2 ln 2), so the FWHM is 2 sigma sqrt(2 ln 2) = 2.355 sigma.

**The radius-or-diameter trap.** Spec sheets often don't say which they mean, and the answer changes the result by a factor of 2. When you tell the tool "not stated", it evaluates both readings. If they lead to different grades, it reports WARN and tells you to ask the vendor.

## 3.3 Seeing blur on the sensor

To compare the optics' blur (in micrometers) with the seeing (in arcseconds), convert the seeing into a physical size on the sensor. It is the plate-scale formula run in reverse:

```
seeing blur (um) = seeing (") x FL_mm / 206.265
```

**Worked example.** 2.5 x 1050 / 206.265 = **12.7 um**.

## 3.4 Adding blurs in quadrature

**Idea.** The atmosphere blurs the star, then the optics blurs it again. Mathematically, the final image is the first blur *convolved* with the second.

**Key fact.** When you convolve two Gaussians, their variances (sigma^2) add. Because FWHM is proportional to sigma, FWHMs add the same way:

```
combined = sqrt(seeing_blur^2 + optics_FWHM^2)
star growth = combined / seeing_blur - 1
```

**Why this is good news.** A small blur barely enlarges a large one. An optical blur half the size of the seeing makes stars only sqrt(1 + 0.25) - 1 = 12% bigger. That's why a telescope doesn't need to be perfect, just good enough compared with the air.

**This is the same rule as noise.** In Lesson 2, independent noise sources added as variances too. "Independent things add in quadrature" comes up throughout these lessons.

## 3.5 Spot size across the field

Spot size usually grows away from the axis. The tool evaluates two points:

* the **center**: the quoted point closest to the axis
* the **corner**: radius = half the sensor diagonal

Between quoted points it interpolates linearly. Beyond the last point it extrapolates linearly from the last two, never going below the last value, and flags the result as "extrapolated".

**Worked example.** The DeltaRho 350 preset quotes 4.9 / 6.2 / 7.6 um RMS at 0 / 23 / 30 mm. The IMX455's corner is at 43.3 / 2 = 21.6 mm, between the first two points:

```
spot at 21.6 mm = 4.9 + (6.2 - 4.9) x 21.6 / 23 = 6.1 um
```

## 3.6 Putting check 4 together

| Reading | Optics FWHM, center / corner | Combined, center / corner | Star growth, center / corner | Status |
|---|---|---|---|---|
| RMS radius (x 1.665) | 8.2 / 10.2 um | 15.1 / 16.3 um | 19% / 28% | WARN |
| RMS diameter (x 0.833) | 4.1 / 5.1 um | 13.4 / 13.7 um | 5% / 8% | PASS |

The two readings disagree, so the check returns **WARN: ask the vendor**.

Work one cell yourself: radius reading at the corner, 1.665 x 6.1 = 10.2 um; sqrt(12.7^2 + 10.2^2) = 16.3 um; 16.3 / 12.7 - 1 = 0.28.

**Grading (worst-case star growth).** PASS up to 15%, WARN up to 35%, FAIL above. INFO if no spot data is entered.

**Caveat.** Real optical blur is often not Gaussian (it can have a sharp core and wide wings). Treat check 4 as a screen, not a prediction.

## 3.7 Critical focus zone (check 7)

**Question.** How far can the sensor drift along the axis before stars grow noticeably?

**Formula.**

```
CFZ (um) = +/- 2.44 x wavelength_um x N^2        (wavelength = 0.55 um, N = FL / D)
```

**Why N squared.** Two effects each contribute one factor of N.

1. **Acceptable blur grows with N.** The diffraction-limited Airy disk is about 2.44 x wavelength x N across. Slower optics form a bigger smallest spot, so they can tolerate a bigger blur.
2. **The light cone gets gentler with N.** Light converges to focus in a cone whose half-width shrinks by about 1 / (2N) for every unit of distance along the axis. Move the sensor by a distance `x` off focus and the blur grows to about `x / N`.

Setting the defocus blur `x / N` equal to the acceptable blur `2.44 x wavelength x N` and solving for `x`:

```
x = 2.44 x wavelength x N x N = 2.44 x wavelength x N^2
```

**Worked examples.**

| Telescope | f-ratio | CFZ |
|---|---|---|
| RASA 11 | f/2.22 (620 / 279) | +/- 6.6 um |
| DeltaRho 350 | f/3.0 | +/- 12.1 um |
| CDK14 | f/7.2 | +/- 69.6 um |

Check the DeltaRho yourself: 2.44 x 0.55 x 3^2 = 12.08 um.

**A note on rounding.** The RASA 11 is sold as "f/2.2". With N = 2.2 exactly the CFZ is 6.5 um, but the tool computes N from the focal length and aperture (2.222) and prints 6.6 um. Because N is squared, a 1% change in N moves the answer 2%. When your hand calculation is a few percent off the tool's, check what you rounded.

**For scale.** A human hair is about 70 um thick. On an f/3 system the whole in-focus zone is about a third of a hair. Temperature changes move the focal point by more than that over a night, and a sensor tilted by a few micrometers will be sharp on one side and soft on the other.

**Grading.** PASS if 40 um or more. PASS with a note between 15 and 40 um (motorized focuser recommended). WARN below 15 um (motorized focuser, temperature-compensated autofocus and a tilt adjuster). This check never FAILs: tight focus is a requirement to plan for, not a disqualifier.

---

## Validate it yourself

* **By hand.** Repeat the check 4 table for the reading you didn't do above. Derive 1.665 from the Gaussian formula.
* **Script.** `python3 docs/learning/check_examples.py` (section `03-optics-and-focus.md`).
* **Unit tests.**

  ```bash
  cargo test blur_growth_uses_independent_blurs_in_quadrature
  cargo test spot_interpolation
  cargo test critical_focus_zone_values
  ```

* **Tool output.** In `--demo`, check 4 for the DeltaRho 350 prints both readings with their growths. Check 7 prints the CFZ for each telescope.

## Self-check

1. A camera has 4144 x 2822 pixels of 4.63 um. What is its diagonal? Does it fit in a 22 mm image circle?
2. A vendor quotes "8 um RMS". The seeing blur at the focal plane is 20 um. What is the star growth if 8 um is a radius? A diameter?
3. Why can't you just add the seeing blur and the optical blur directly?
4. How does the CFZ of an f/4 system compare with that of an f/8 system?

<details>
<summary>Answers</summary>

1. 19.2 x 13.1 mm, diagonal 23.2 mm. A 22 mm circle covers 95% of it: WARN (soft corners).
2. Radius: optics FWHM 13.3 um, combined 24.0 um, growth 20% (WARN). Diameter: 6.7 um, combined 21.1 um, growth 5% (PASS).
3. They are independent random blurs. Their variances add, not their widths, so the combined width is the square root of the sum of squares. Adding the widths directly would overstate the blur.
4. The CFZ scales with N^2, so f/8 has (8/4)^2 = 4 times the tolerance of f/4.

</details>

## Where it lives in the code

* `src/calculations/optics.rs`: `seeing_blur_um`, `spot_fwhm_um`, `quadrature`, `blur_growth_fraction`, `spot_rms_at`, `critical_focus_zone_um`.
* `src/checks.rs`: `check_sensor_fit`, `check_optics`, `check_focus`.
* `src/constants.rs`: `FWHM_PER_RMS_RADIUS`; module `checks_limits`: `FIT_WARN_FRACTION`, `OPTICS_*_GROWTH`, `CFZ_*_UM`.

**Next:** [Lesson 4: Light collection and search speed](04-light-collection-and-search.md)
