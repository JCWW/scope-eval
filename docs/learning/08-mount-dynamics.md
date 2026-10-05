# Lesson 8: Mount dynamics

Can the mount keep up with the target? This lesson covers the speed it must reach, the acceleration it needs, the "keyhole" where every two-axis mount struggles, and how long it takes to get onto a target in the first place.

**Prerequisites:** [Lesson 6](06-orbits-and-angular-rates.md) (rate vs ground, overhead pass), basic calculus (one derivative) for section 8.4.

## You will be able to

* Compute the axis rate a mount needs for each regime and grade its headroom.
* Explain the alt-az zenith keyhole and compute the highest pass a mount can follow.
* Derive the peak tracking acceleration of an overhead pass, including the constant 3 sqrt(3) / 8.
* Combine the rate and acceleration limits on the keyhole and say which one binds.
* Compute slew time with a trapezoidal or triangular motion profile.

## Key terms

| Term | Meaning |
|---|---|
| **Alt-azimuth (alt-az) mount** | Two axes: azimuth (compass direction) and altitude/elevation (angle above the horizon). |
| **Equatorial mount** | Two axes aligned with Earth's spin: hour angle (or right ascension) and declination. |
| **Axis rate** | How fast one axis turns, deg/s. |
| **Axis acceleration** | How fast the axis rate can change, deg/s^2. |
| **Headroom** | Mount capability / requirement. 3x means the mount can do three times what's needed. |
| **Zenith** | The point directly overhead. |
| **Zenith distance (z)** | Angle from the zenith: 90 deg - elevation. |
| **Keyhole** | The direction where one axis would have to turn infinitely fast to follow a target. For alt-az, the zenith; for equatorial, the celestial pole. |
| **Slew** | A fast move to a new target. |
| **Settle time** | How long the mount takes to stop vibrating after a slew. |

---

## 8.1 Tracking rate (graded)

**Formulas.**

```
required rate (deg/s) = rate vs ground ("/s) / 3600
headroom              = mount maximum axis rate / required rate
```

**Worked example.** LEO needs 3,140 / 3600 = 0.87 deg/s. An L-350 (50 deg/s) has 57x headroom.

**Why 3x for PASS.** The maximum slew rate is a ceiling, not something a mount can track smoothly at while also accelerating and correcting. 3x leaves room for that.

**Grading.** PASS if headroom >= 3x, WARN if >= 1x, FAIL below. Stare mode (GEO) only needs to point and hold, so it passes. If the slew rate is unknown: WARN when the required rate is above 0.1 deg/s (in practice, LEO), otherwise INFO.

## 8.2 The alt-az zenith keyhole

**Idea.** Picture a satellite passing almost, but not quite, straight overhead. Near the zenith, the azimuth axis must swing from one side of the sky to the other in the few seconds it takes the satellite to pass closest. The closer to the zenith, the faster the swing. A pass exactly through the zenith needs an instantaneous 180-degree flip.

**Small-angle model.** Near the zenith the sky is locally flat. A target moving in a straight line at angular rate `omega` passes the zenith at a closest distance `z` (in radians). Its azimuth is the angle of its position seen from the zenith, which turns fastest at closest approach:

```
peak azimuth rate ~ omega / z
```

(The same `v / h` idea as an overhead pass, with the zenith as the observer and `z` as the closest distance.)

**Highest followable pass.** Set the peak azimuth rate equal to the mount's maximum and solve for z:

```
z = omega / max rate
highest pass elevation (deg) = 90 - degrees(omega / max axis rate)
```

**Worked example.** LEO on an L-350: z = 0.87 / 50 = 0.0174 rad = 1.0 deg, so passes up to **89 deg** elevation can be followed. A mount limited to 3 deg/s: z = 0.87 / 3 = 0.29 rad = 16.6 deg, so only passes up to about **73 deg**.

**Grading.** PASS if the highest followable pass is >= 85 deg, WARN if >= 70 deg, FAIL below. For equatorial mounts the keyhole is near the celestial pole, which is noted but not graded here (pass prediction in [Lesson 10](10-pass-prediction.md) judges it pass by pass).

## 8.3 Non-sidereal tracking (graded where required)

Rate tracking (Lesson 6) needs the mount's control software to follow an arbitrary predicted path. LEO, MEO and HEO require it. PASS if supported, FAIL if not, WARN if unknown (a question for the vendor). INFO for GEO and cislunar, where it isn't needed.

## 8.4 Peak tracking acceleration

**Setting up.** On an overhead pass at altitude `h` and speed `v`, measure time `t` from closest approach. The angle from the zenith is

```
theta(t) = atan(v t / h)
```

**Differentiating.** With `u = v t / h` (so `du/dt = v/h`):

```
theta'  = (v/h) / (1 + u^2)
theta'' = -2 (v/h)^2 u / (1 + u^2)^2
```

Check: the first derivative of `atan(u)` is `1 / (1 + u^2)`; the chain rule gives the `v/h` factor. Differentiating again gives the second line.

**Finding the peak.** Maximize `f(u) = u / (1 + u^2)^2`. Setting `f'(u) = 0`:

```
(1 + u^2) - 4u^2 = 0   ->   u = 1 / sqrt(3)
f(1/sqrt(3)) = (1/sqrt(3)) / (4/3)^2 = 9 / (16 sqrt(3)) = 3 sqrt(3) / 16
```

So, with `omega = v/h` (the peak rate):

```
peak acceleration = 2 x 3 sqrt(3) / 16 x omega^2 = (3 sqrt(3) / 8) x omega^2 = 0.6495 x omega^2
```

`PEAK_ACCEL_COEFF = 3 sqrt(3) / 8` is a derived constant, not a tuned threshold.

**Worked example.** LEO: omega = 0.87234 deg/s = 0.015225 rad/s.

```
peak acceleration = 0.6495 x 0.015225^2 = 1.506e-4 rad/s^2 = 0.008627 deg/s^2
```

(Careful with units: square omega in rad/s, then convert back to deg/s^2. Squaring deg/s directly gives the wrong answer.)

That's negligible for any real mount, which is why the acceleration model doesn't stop here. MEO's figure is 1.37e-6 deg/s^2, nearly four orders of magnitude smaller, because the requirement scales as `omega^2`. `ACCEL_MATTERS_DEG_S2` is set to 0.005 so that only LEO triggers the "ask the vendor" branch when the mount's acceleration rating is missing.

## 8.5 The acceleration-limited keyhole

**Idea.** Acceleration matters where the azimuth axis whips around near the zenith. The azimuth sweeps through the same `atan` form as 8.4, with the zenith distance `z` in place of the altitude and `omega / z` in place of `v / h`. So the peak azimuth acceleration is

```
peak azimuth acceleration = 0.6495 x (omega / z)^2
```

Requiring that to stay within the mount's rating:

```
z >= omega x sqrt(0.6495 / max acceleration)          (all in radians)
```

The rate limit (8.2) gave `z >= omega / max rate`. These are two constraints on one keyhole, so the tool reports whichever is larger and names which one binds:

| Mount acceleration | Accel-limited keyhole | Rate-limited (50 deg/s) | Binding |
|---|---|---|---|
| 10 deg/s^2 | 88.3 deg elevation | 89.0 deg elevation | acceleration |
| 2 deg/s^2 | 86.2 deg elevation | 89.0 deg elevation | acceleration |
| 0.5 deg/s^2 | 82.5 deg elevation (WARN) | 89.0 deg elevation | acceleration |

Work the 2 deg/s^2 row: omega = 0.015225 rad/s, max acceleration = 2 x pi/180 = 0.0349 rad/s^2. z = 0.015225 x sqrt(0.6495 / 0.0349) = 0.0657 rad = 3.76 deg, so 86.2 deg elevation.

Acceleration binds in every realistic case. When a mount publishes no acceleration figure, the tool falls back to the rate-only keyhole. None of the presets publish one.

## 8.6 Slew and settle

**Idea.** Getting onto a target is a three-part move: accelerate up to the rate limit, cruise, decelerate. If the distance is too short to reach the rate limit, it's accelerate-then-decelerate with no cruise.

**Formulas.**

```
trapezoidal (D >= v^2/a):  t = v/a + D/v
triangular  (D <  v^2/a):  t = 2 sqrt(D/a)
```

**Derive them.**

* *Triangular:* accelerate over D/2 then decelerate over D/2. Covering D/2 from rest at acceleration a takes sqrt(2 (D/2) / a) = sqrt(D/a). Twice that is 2 sqrt(D/a).
* *Trapezoidal:* accelerating to v takes v/a and covers v^2 / (2a). Decelerating is the same. The cruise covers the remaining D - v^2/a at speed v, taking D/v - v/a. Total: 2(v/a) + D/v - v/a = v/a + D/v.

At D = v^2/a both give 2v/a, so the time doesn't jump at the boundary.

| Distance | Max rate | Max acceleration | Profile | Time |
|---|---|---|---|---|
| 90 deg | 50 deg/s | 10 deg/s^2 | triangular (v^2/a = 250) | 6.0 s |
| 90 deg | 50 deg/s | 50 deg/s^2 | trapezoidal (v^2/a = 50) | 2.8 s |
| 90 deg | 6 deg/s | 1 deg/s^2 | trapezoidal (v^2/a = 36) | 21.0 s |

**Grading.** Slew time plus settle time (2 s if not entered) is compared with the regime's usable window. Only LEO has a window (300 s, roughly how long a 500 km pass stays above useful elevation): PASS under 10% of it, WARN under 25%, FAIL above. Every other regime is visible for hours, so the check is INFO there. If the acceleration rating is missing but the slew rate is known, the tool reports `D / v` as a lower bound.

---

## Validate it yourself

* **By hand.** Repeat the 0.5 deg/s^2 keyhole row. Then compute the slew time for 90 deg at 6 deg/s and 1 deg/s^2.
* **Numerically.** Check the 3 sqrt(3) / 8 result without calculus: evaluate `u / (1 + u^2)^2` for u from 0 to 2 in steps of 0.01 in a spreadsheet and find the peak. It should peak near u = 0.58 (exactly 1/sqrt(3) = 0.577) with value 0.3248, and 2 x 0.3248 = 0.6495.
* **Script.** `python3 docs/learning/check_examples.py` (section `08-mount-dynamics.md`).
* **Unit tests.**

  ```bash
  cargo test peak_accel_coefficient_matches_closed_form
  cargo test leo_peak_tracking_accel
  cargo test peak_accel_scales_as_omega_squared
  cargo test acceleration_tightens_the_keyhole
  cargo test keyhole_l350_leo
  cargo test slew_time_is_continuous_at_profile_boundary
  ```

* **Independent geometry check.** `cargo test -p orbit-prop near_zenith_pass_spins_the_azimuth_axis` propagates a real near-zenith pass and measures the azimuth rate, instead of using the small-angle formula.

## Self-check

1. A mount's maximum rate is 5 deg/s. What is its headroom for LEO? Its highest followable LEO pass?
2. Why does the required acceleration for MEO come out so much smaller than for LEO when the rates differ by only about 80x?
3. A mount has 20 deg/s and 4 deg/s^2. How long does a 30 deg slew take? Which profile?
4. Why does an exactly overhead pass need an infinite azimuth rate, but not an infinite elevation rate?

<details>
<summary>Answers</summary>

1. Headroom 5 / 0.87 = 5.7x (PASS). z = 0.87 / 5 = 0.174 rad = 10 deg, so passes up to 80 deg: WARN.
2. Acceleration scales as omega^2, so an 80x rate difference becomes 80^2 = 6,400x.
3. v^2/a = 100 deg, more than 30, so triangular: 2 x sqrt(30 / 4) = 5.5 s.
4. The elevation follows the smooth `atan` curve through 90 deg. But the azimuth is the direction of the target seen from the zenith, which jumps by 180 deg as it passes through the zenith, like a line through the center of a clock face.

</details>

## Where it lives in the code

* `src/calculations/mount.rs`: `peak_tracking_accel_rad_s2`, `accel_limited_keyhole_rad`, `keyhole_rad`, `slew_time_s`.
* `src/regimes.rs`: `mount_rate`, `mount_acceleration`, `mount_slew_settle`, `mount_non_sidereal`.
* `src/constants.rs`: `PEAK_ACCEL_COEFF`, `DEFAULT_SLEW_DISTANCE_DEG`, `DEFAULT_SETTLE_TIME_S`; module `regimes_limits`: `RATE_*`, `KEYHOLE_*`, `ACCEL_*`, `SLEW_*`.

**Next:** [Lesson 9: Brightness and detection](09-brightness-and-detection.md)
