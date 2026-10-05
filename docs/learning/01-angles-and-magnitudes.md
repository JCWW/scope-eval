# Lesson 1: Angles and magnitudes

Almost every number `scope-eval` prints is an angle on the sky or a brightness on the magnitude scale. This lesson covers both units and the one approximation, the small-angle rule, that turns lengths on a sensor into angles on the sky.

**Prerequisites:** none.

## You will be able to

* Convert between degrees, arcminutes, arcseconds and radians, and explain where 206,265 comes from.
* Use the small-angle rule to turn a size and a distance into an angle, and back again.
* Convert a brightness ratio into a magnitude difference and back again.
* Explain why a *larger* magnitude means a *fainter* object.

## Key terms

| Term | Meaning |
|---|---|
| **Degree (deg)** | 1/360 of a full circle. |
| **Arcminute (')** | 1/60 of a degree. |
| **Arcsecond (")** | 1/60 of an arcminute, so 1/3600 of a degree and 1/1,296,000 of a circle. |
| **Radian (rad)** | The angle at which the arc length equals the radius. A circle is 2 pi radians. |
| **Small-angle rule** | For small angles, angle (in radians) = size / distance. |
| **Magnitude (mag)** | A logarithmic brightness scale. Larger numbers are fainter. 5 magnitudes is a factor of exactly 100. |
| **Flux** | The amount of light arriving per unit area per unit time. Magnitudes compare fluxes. |

---

## 1.1 Angles on the sky

Telescopes see angles, not distances. The tool reports pixel sizes, star sizes, fields of view and satellite motions as angles on the sky, mostly in arcseconds.

```
1 circle  = 360 deg
1 deg     = 60'  = 3600"
1 circle  = 360 x 3600 = 1,296,000"
1 circle  = 2 pi rad
```

**Arcseconds in a radian.** Divide the arcseconds in a circle by the radians in a circle:

```
1 rad = 1,296,000" / (2 pi) = 206,264.8"   (usually written 206,265)
1 rad = 360 / (2 pi) deg     = 57.2958 deg
```

You will see 206,265 and 57.2958 throughout these lessons. Both are just "radians to arcseconds" and "radians to degrees".

**For scale.** The full Moon is about 0.5 deg = 1,800" across. Typical seeing blurs a star to about 2.5". One pixel on the running example is about 0.74".

## 1.2 The small-angle rule

For an object of size `s` at distance `d`, the angle it covers is

```
angle (rad) = s / d                (when s is much smaller than d)
angle (")   = s / d x 206,265
```

**Why it works.** The exact angle is `2 x atan(s / 2d)`. For small arguments `atan(x)` is very close to `x`, so the exact answer collapses to `s / d`. At 2 degrees the error is about 0.01%, far smaller than anything else in the tool.

**Units must match.** `s` and `d` must be in the same unit, or you must account for the conversion. That is why the plate-scale formula in [Lesson 2](02-seeing-and-sampling.md) uses 206.265 instead of 206,265: pixel size is in micrometers and focal length in millimeters, and the factor of 1000 between them is folded into the constant.

### Worked example: a coin at 4 km

A coin 2.5 cm across seen from 4 km away:

```
angle = 0.025 m / 4,000 m x 206,265 = 1.29"
```

So "one arcsecond is roughly a coin at 4 km" is right to within the size of the coin.

### Worked example: a satellite's position error

A satellite whose predicted position is off by 2 km along its track looks, from 500 km away, like an angle of

```
2 / 500 x 206,265 = 825"   (about 14 arcminutes)
```

From GEO range (about 37,000 km) the same 2 km is only 11". The same physical error can be a large or a small angle depending on range. This idea comes back in [Lesson 6](06-orbits-and-angular-rates.md).

## 1.3 The magnitude scale

Astronomers measure brightness on a logarithmic scale that runs backwards: the brightest stars are around magnitude 0 or 1, the faintest the eye can see are about 6, and large telescopes reach 20 and beyond.

**Definition.** Two objects with fluxes `F1` and `F2` differ in magnitude by

```
m1 - m2 = -2.5 x log10(F1 / F2)
```

and, turned around,

```
F1 / F2 = 10^(-0.4 x (m1 - m2))
```

**Why it runs backwards.** The scale was inherited from ancient Greek catalogs, where the brightest stars were "of the first magnitude" (first rank) and dimmer ones were second, third and so on. The modern definition kept that order and fixed the step: 5 magnitudes is exactly a factor of 100, so one magnitude is 100^(1/5) = 2.512.

**Why a logarithm.** Brightnesses of interest span a factor of more than a billion. A logarithmic scale turns multiplying into adding: two factors of 2 in light (say twice the area and twice the exposure) add up to 0.75 + 0.75 = 1.5 magnitudes.

| Brightness ratio | Magnitude difference |
|---|---|
| 1.3x | 0.28 |
| 2x | 0.75 |
| 2.512x | 1 |
| 10x | 2.5 |
| 100x | 5 |

**Sign convention used by the tool.** Depth is reported so that a *positive* number means you can see *fainter* objects. A telescope collecting 1.3x more light than the reference reports `+0.28 mag`.

### Worked example: twice the light

```
2.5 x log10(2) = 2.5 x 0.301 = 0.75 mag
```

Doubling the collecting area lets you see objects 0.75 magnitude fainter, all else equal.

## 1.4 Surface brightness

The night sky is not a point; it is a glow spread over the whole field. Its brightness is quoted per unit of solid angle, in **magnitudes per square arcsecond** (mag/arcsec^2). A dark rural site is about 21.5 to 22, a suburban site about 19 to 20. Larger is darker. [Lesson 9](09-brightness-and-detection.md) turns this into electrons per pixel.

---

## Validate it yourself

* **By hand.** Compute `180 / pi x 3600` and confirm 206,264.8. Compute `10^(0.4 x 5)` and confirm 100.
* **Script.** `python3 docs/learning/check_examples.py` checks every number in this lesson under the heading `01-angles-and-magnitudes.md`.
* **Unit tests.**

  ```bash
  cargo test five_magnitudes_is_a_factor_of_one_hundred
  cargo test four_times_the_range_is_three_magnitudes_fainter
  ```

* **In the code.** Look up `ARCSEC_PER_RADIAN` and `DEG_PER_RADIAN` in `src/constants.rs` and check them against your own calculation.

## Self-check

1. How many arcseconds are in 1.5 degrees?
2. A pixel subtends 0.000005 radians. How many arcseconds is that?
3. Object A is magnitude 12, object B is magnitude 17. Which is brighter, and by what factor?
4. A telescope collects 4 times more light than another. How many magnitudes deeper can it go?
5. Why does moving the same object 4 times further away make it 3 magnitudes fainter? (Hint: light spreads over the area of a sphere.)

<details>
<summary>Answers</summary>

1. 1.5 x 3600 = 5,400".
2. 0.000005 x 206,265 = 1.03".
3. A is brighter (smaller magnitude). The difference is 5 magnitudes, so A is 100 times brighter.
4. 2.5 x log10(4) = 1.51 mag.
5. Flux falls as 1 / distance^2, so 4 times the distance is 1/16 of the flux. 2.5 x log10(16) = 3.01 mag. This is the test `four_times_the_range_is_three_magnitudes_fainter`.

</details>

## Where it lives in the code

* `src/constants.rs`: `ARCSEC_PER_RADIAN`, `DEG_PER_RADIAN`, `ARCSEC_PER_CIRCLE`.
* `src/calculations/optics.rs`: `OpticsCalculator::delta_mag` (magnitude difference from an area ratio).

**Next:** [Lesson 2: Seeing and sampling](02-seeing-and-sampling.md)
