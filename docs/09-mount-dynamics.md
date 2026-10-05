# 9. Mount dynamics

**What you'll learn:** how much acceleration a mount needs to track a pass, why acceleration rather than top speed usually sets the alt-az keyhole, and how long a mount takes to slew onto a target and settle.

**Before you start:** [Orbital regimes](08-orbital-regimes.md#part-4-mount-checks), especially the LEO overhead rate and the rate-limited keyhole.

> **Work in radians.** Every formula on this page squares a rate or takes a square root of a ratio, so it only gives the right answer when angles are in radians: rates in rad/s and accelerations in rad/s^2. Convert to degrees at the end. The code does the same (`MountDynamicsCalculator` in `src/calculations/mount.rs`). Plugging in deg/s directly gives a wrong answer, not just a differently scaled one.

---

## Step 1: peak tracking acceleration

An overhead pass at altitude h and speed v is seen at an angle from the zenith of `theta(t) = atan(v t / h)`, where t is the time from closest approach. Differentiating twice, with `u = v t / h`:

```
theta'  = (v/h) / (1 + u^2)
theta'' = -2 (v/h)^2 u / (1 + u^2)^2
```

`|theta''|` peaks at `u = 1/sqrt(3)`, which gives

```
peak acceleration = PEAK_ACCEL_COEFF x omega^2,   PEAK_ACCEL_COEFF = 3 sqrt(3) / 8 = 0.6495
```

where `omega = v/h` is the peak rate. This is a derived constant, not a tuned threshold.

**Worked example (LEO).** omega = 0.87234 deg/s. First convert: 0.87234 x pi / 180 = **0.015225 rad/s**. Then 0.6495 x 0.015225^2 = **1.506e-4 rad/s^2**, which is **0.008627 deg/s^2**.

That is negligible for any mount in the preset list, and it is why the acceleration model doesn't stop here: compared against a spec sheet, this check would pass unconditionally and tell you nothing. The equivalent MEO figure is 1.37e-6 deg/s^2, nearly four orders of magnitude smaller, because the requirement scales as `omega^2`. `ACCEL_MATTERS_DEG_S2` is set at 0.005 so that LEO alone trips the "ask the vendor" branch when a rating is missing.

**Grading.** Headroom = mount maximum acceleration / required acceleration. PASS at 3x or more, WARN at 1x or more, FAIL below. With no rating entered: WARN if the requirement exceeds `ACCEL_MATTERS_DEG_S2`, otherwise INFO. Stare mode passes.

---

## Step 2: the acceleration-limited keyhole

Acceleration bites where the azimuth axis has to whip around near the zenith. There the azimuth angle sweeps through the same `atan` form as the pass itself, with the minimum zenith distance `z` in place of the altitude. So the peak azimuth rate is `omega / z` (the rate keyhole from page 8) and the peak azimuth acceleration is `PEAK_ACCEL_COEFF x (omega/z)^2`. Requiring that to stay inside the mount's rating gives

```
z >= omega x sqrt(PEAK_ACCEL_COEFF / max acceleration)        (all in radians)
```

The rate limit already gave `z >= omega / max rate`. These are two constraints on one physical keyhole, so the tool reports whichever binds (the larger z) and names which one it was.

**Worked example (LEO, 10 deg/s^2).** Convert the rating: 10 x pi / 180 = 0.17453 rad/s^2. Then z = 0.015225 x sqrt(0.6495 / 0.17453) = 0.02937 rad = 1.68 deg. The highest followable pass is 90 - 1.68 = **88.3 deg**. The rate limit at 50 deg/s gave 89.0 deg, so acceleration binds.

| Mount acceleration | Accel-limited keyhole | Rate-limited (50 deg/s) | Binding |
|---|---|---|---|
| 10 deg/s^2 | 88.3 deg elevation | 89.0 deg elevation | acceleration |
| 2 deg/s^2 | 86.2 deg elevation | 89.0 deg elevation | acceleration |
| 0.5 deg/s^2 | 82.5 deg elevation (WARN) | 89.0 deg elevation | acceleration |

Acceleration binds in every realistic case, and by 0.5 deg/s^2 it has pushed the keyhole below `KEYHOLE_PASS_ELEV_DEG` (85 deg), so the keyhole grades WARN. **When a mount publishes no acceleration figure the reported keyhole is the rate-only number.** None of the presets publish one, so none of their keyhole figures depend on this model until you enter a rating.

---

## Step 3: slew and settle

Getting on target is a trapezoidal move: accelerate to the rate limit, cruise, decelerate. If the distance is too short to reach the rate limit the profile is triangular instead. With distance D, maximum rate v and maximum acceleration a:

```
trapezoidal (D >= v^2/a):  t = v/a + D/v
triangular  (D <  v^2/a):  t = 2 sqrt(D/a)
```

Both branches agree at `D = v^2/a`, so the reported time cannot jump for a small change in the assumed distance. (These formulas are ratios of like units, so degrees work here as long as you're consistent.)

| Distance | Max rate | Max acceleration | Profile | Time |
|---|---|---|---|---|
| 90 deg | 50 deg/s | 10 deg/s^2 | triangular | 6.0 s |
| 90 deg | 50 deg/s | 50 deg/s^2 | trapezoidal | 2.8 s |
| 90 deg | 6 deg/s | 1 deg/s^2 | trapezoidal | 21.0 s |

**Grading.** Slew plus settle is graded against the regime's usable window. Only LEO has one (300 s, roughly how long a 500 km pass stays above useful elevation); every other regime stays available for hours, so the check reports INFO there. PASS if slew plus settle is at most 10% of the window, WARN up to 25%, FAIL beyond. A direct-drive mount needing 6 s of slew and 2 s of settle uses 8 / 300 = 2.7% of a LEO pass. If the acceleration rating is missing but the slew rate is known, the tool still reports `D / v` as an explicit lower bound rather than giving up. The slew distance is assumed to be 90 deg and, where none is entered, the settle time 2 s.

---

## Check it yourself

**By hand.**

1. **The coefficient.** Compute 3 x sqrt(3) / 8. You should get **0.6495**.
2. **The units trap.** Compute 0.6495 x 0.87234^2 using degrees directly. You get 0.494, which is wrong by a factor of about 57 (180/pi). Redo it in radians as in Step 1 and you get 0.008627 deg/s^2.
3. **The 2 deg/s^2 keyhole.** 2 x pi / 180 = 0.034907 rad/s^2. z = 0.015225 x sqrt(0.6495 / 0.034907) = 0.0657 rad = 3.76 deg. Highest pass = **86.2 deg**.
4. **Triangular slew.** For 90 deg at 50 deg/s and 10 deg/s^2: v^2/a = 250 deg, more than 90, so triangular. t = 2 x sqrt(90 / 10) = **6.0 s**.
5. **Trapezoidal slew.** For 90 deg at 6 deg/s and 1 deg/s^2: v^2/a = 36 deg, less than 90, so trapezoidal. t = 6 / 1 + 90 / 6 = **21.0 s**.
6. **Continuity.** For v = 50, a = 10, set D = v^2/a = 250. Triangular: 2 x sqrt(25) = 10 s. Trapezoidal: 5 + 5 = 10 s. They agree.
7. **Lower bound.** With no acceleration rating, the L-350 at 50 deg/s needs at least 90 / 50 = **1.8 s**.

**Against the tests.**

| Concept | Command |
|---|---|
| The coefficient matches 3 sqrt(3) / 8 | `cargo test peak_accel_coefficient` |
| LEO peak tracking acceleration and omega^2 scaling | `cargo test peak_accel` and `cargo test leo_peak_tracking_accel` |
| Keyhole with and without an acceleration rating | `cargo test keyhole` |
| Triangular, trapezoidal and continuity | `cargo test slew_time` |
| Grading of acceleration and slew-and-settle | `cargo test acceleration_` and `cargo test slew_and_settle` |

**In the tool.** Run `cargo run --release -- --demo` and look at the LEO block for DeltaRho 350 + IMX455. The acceleration check should show a required peak acceleration of about 0.00863 deg/s^2 and WARN because the L-350's rating is unknown, and the slew-and-settle check should report "at least 1.8 s". Then evaluate the same configuration interactively and enter 0.5 deg/s^2 when asked for the mount's acceleration: the keyhole should drop to 82.5 deg and name acceleration as the binding limit.

Next: [Target brightness and detection](10-target-brightness-and-detection.md).
