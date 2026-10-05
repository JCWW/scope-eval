# Lesson 6: Orbits and angular rates

Everything so far described the optical system on its own. Whether it can observe a satellite depends on where the satellite is: how far away, how fast it moves across the sky and how well its position is predicted. This lesson derives those numbers for the tool's five orbital regimes and uses them in the telescope's regime checks.

**Prerequisites:** [Lesson 1](01-angles-and-magnitudes.md) (small-angle rule), [Lesson 4](04-light-collection-and-search.md) (field of view).

## You will be able to

* Describe the five orbital regimes and what makes each hard to observe.
* Compute a circular orbit's speed and the angular rate of an overhead pass.
* Use the vis-viva equation for an elliptical orbit.
* Compute rates from an orbital period, and explain the difference between rate against the stars and rate against the ground.
* Work out whether a target will land in the field when you point at its predicted position.

## Key terms

| Term | Meaning |
|---|---|
| **Altitude (h)** | Height above Earth's surface. |
| **Orbital radius (r)** | Distance from Earth's center: r = R + h, with R = 6,378 km. |
| **mu** | Earth's gravitational parameter, G x M = 398,600 km^3/s^2. |
| **Semi-major axis (a)** | Half the long axis of an ellipse; the "average" radius of an elliptical orbit. |
| **Eccentricity (e)** | How elongated an orbit is: 0 is a circle, close to 1 is very stretched. |
| **Perigee / apogee** | The lowest / highest point of an elliptical orbit. |
| **Range** | Distance from the observer to the target. |
| **Sidereal day** | One rotation of Earth relative to the stars: 86,164.09 s (about 4 minutes shorter than a solar day). |
| **Rate vs stars** | How fast the target moves against the background stars. Drives timing, shutter skew and trailing. |
| **Rate vs ground** | How fast the target moves across the sky as seen from the site. What the mount must follow. |
| **TLE / ephemeris** | A prediction of where a satellite will be. Its errors are mostly *along-track* (ahead of or behind the predicted position). |

---

## 6.1 The five regimes

| Regime | Representative case | Range (km) | Rate vs stars ("/s) | Rate vs ground ("/s) | Prediction error (km) | Usual mode |
|---|---|---|---|---|---|---|
| LEO | 500 km circular, overhead pass | 500 | ~3,140 (0.87 deg/s) | ~3,140 | 2 | rate track |
| MEO | GPS-like 20,200 km, overhead | 20,200 | ~40 | ~40 | 2 | rate track |
| GEO | 35,786 km altitude | ~37,000 | 15.04 | 0 | 2 | stare |
| HEO | Molniya near apogee | ~39,800 | ~7.8 | ~7.3 | 5 | rate track |
| Cislunar | Lunar distance | ~384,400 | ~0.55 | ~14.5 | 50 | sidereal |

The rest of this lesson derives every number in the rate and range columns. The prediction errors are assumptions (see 6.6).

Notice the spread: LEO moves 5,700 times faster across the sky than cislunar targets, and cislunar targets are 770 times further away. No single telescope is ideal for both.

## 6.2 LEO and MEO: circular speed and overhead rate

**Circular speed.** For a circular orbit, gravity provides exactly the centripetal force: `mu / r^2 = v^2 / r`, so

```
v = sqrt(mu / r) = sqrt(mu / (R + h))
```

**Overhead angular rate.** Seen from directly below, a satellite at altitude `h` moving at `v` sweeps an angle at the rate

```
omega (rad/s) ~ v / h          (x 206,265 for "/s)
```

This is the small-angle rule applied to motion: in one second it moves `v` km at a distance of `h` km.

**Worked example (LEO, 500 km).**

```
v     = sqrt(398,600 / 6,878) = 7.61 km/s
omega = 7.61 / 500 = 0.0152 rad/s = 3,140 "/s = 0.87 deg/s
```

**Why overhead.** An overhead pass is the closest the satellite ever gets, so it is the fastest it ever moves across the sky. That makes it the right *worst case* for mount and timing requirements.

**Simplifications.** Earth's rotation is ignored (a small error for LEO, about 10% for MEO). At these rates the stars' own motion, 15"/s, is negligible, so rate vs ground is taken equal to rate vs stars.

Try MEO yourself (h = 20,200 km). You should get about 40 "/s.

## 6.3 HEO: the vis-viva equation

An elliptical orbit's speed changes all the way round: fast at perigee, slow at apogee. The vis-viva equation gives the speed at any radius `r`:

```
v = sqrt( mu x (2/r - 1/a) )
```

It comes from conservation of energy: kinetic energy `v^2 / 2` plus potential energy `-mu / r` is a constant for the orbit, equal to `-mu / (2a)`. Solve for `v` and you get vis-viva. For a circle, r = a, and it reduces to `sqrt(mu / r)`, matching 6.2.

**Worked example (Molniya, a = 26,560 km, e = 0.74).**

```
apogee radius   r = a x (1 + e) = 46,214 km
apogee altitude   = 46,214 - 6,378 = 39,836 km
speed at apogee v = sqrt(398,600 x (2/46,214 - 1/26,560)) = 1.50 km/s
rate vs stars     = 1.50 / 39,836 x 206,265 = 7.8 "/s
rate vs ground    = |15.04 - 7.8| = 7.3 "/s
```

The ground rate is a rough approximation: the difference from the sidereal rate, ignoring directions.

## 6.4 GEO and cislunar: rates from the period

For distant objects, the observer's own motion (Earth turning under them) matters as much as the target's, so `v / h` is the wrong tool. Instead, use the period: an object going once round the sky (1,296,000") in period `P` moves against the stars at

```
rate vs stars = 1,296,000" / P
```

**GEO.** P = one sidereal day:

```
1,296,000 / 86,164.09 = 15.04 "/s     (the sidereal rate)
```

The ground rotates at exactly the same rate, so **rate vs ground = 0**. A GEO satellite hangs still in the sky while the stars drift past it.

**Cislunar.** Using the Moon's period, 27.32 days:

```
rate vs stars  = 1,296,000 / (27.32 x 86,400) = 0.55 "/s
rate vs ground = 15.04 - 0.55 = 14.5 "/s
```

Cislunar objects barely move against the stars, but the ground turns under them at almost the full sidereal rate. Ordinary sidereal tracking keeps them nearly still.

**Rate vs stars vs rate vs ground.** Positions are measured *against the stars* in an image, so the rate vs stars sets timing and shutter requirements ([Lesson 7](07-timing-and-shutters.md)). The mount sits *on the ground*, so the rate vs ground sets what it must follow ([Lesson 8](08-mount-dynamics.md)).

## 6.5 Tracking modes

| Mode | The mount follows | Target looks like | Stars look like | Used for |
|---|---|---|---|---|
| **Stare** | nothing (stopped) | a point, if it is Earth-fixed | streaks | GEO |
| **Sidereal** | the stars | a short streak, if it moves against the stars | points | cislunar |
| **Rate track** | the target's predicted path | a point | streaks | LEO, MEO, HEO |

Rate tracking needs the mount's software to follow an arbitrary path from a TLE or ephemeris, which is called **non-sidereal tracking**. Not every mount supports it.

## 6.6 Acquisition: will the target land in the field? (graded)

**Idea.** You point at the predicted position. The real position is off by the prediction error (mostly along-track) plus the mount's own pointing error. The target must still be in the field.

**Formulas.**

```
prediction error (") = prediction error (km) / range (km) x 206,265
needed (")           = prediction error + mount pointing error
margin               = (short side of the field / 2) / needed
```

**Why the short side.** The error can lie in any direction, so the worst case is along the narrow axis of the field. Half of it, because you aim at the center.

**Worked example (LEO, DeltaRho 350 + IMX455, 30" mount pointing).**

```
prediction error  = 2 / 500 x 206,265     = 825"
needed            = 825 + 30              = 855"
half short side   = 1.31 deg x 3600 / 2   = 2,359"
margin            = 2,359 / 855           = 2.8x    -> PASS
```

A CDK17 with the same camera has a half short side of only 843" (24.0 mm / 2939 mm), so its margin is 843 / 855 = 0.99: **FAIL**.

**The lesson.** Wide fields matter for LEO because a small along-track error is a large angle at short range. The same 2 km at GEO range is only 11".

**Grading.** PASS if margin >= 2, WARN if >= 1, FAIL below 1. Mount pointing comes from your input, or 60" if left blank.

**The prediction errors are assumptions.** 2 km for LEO, MEO and GEO, 5 km for HEO and 50 km for cislunar are placeholders for a reasonably fresh public element set. Real errors depend heavily on the catalog and on how old the prediction is. Edit them in `regimes()` in `src/regimes.rs`.

## 6.7 Field dwell and depth relevance (INFO)

**Field dwell.** How long an untracked target stays in the field:

```
dwell (s) = short side of the field (") / rate vs ground ("/s)
```

For LEO on the DeltaRho 350: 4,718" / 3,140 "/s = 1.5 s. That's the window for a "stare and catch" approach where the telescope waits for a satellite to cross. A GEO target never leaves a stopped telescope's field.

**Depth relevance.** The tool reports the effective area and depth against the reference (Lesson 4), alongside what usually limits detection in the regime. LEO targets are bright, so tracking and timing dominate. GEO and especially cislunar targets are faint, so collecting area dominates. [Lesson 9](09-brightness-and-detection.md) puts absolute numbers on that.

---

## Validate it yourself

* **By hand.** Derive the MEO overhead rate. Then compute the acquisition margin for GEO on the DeltaRho 350 (range 37,000 km, 2 km error, 30" pointing). You should get about 57x, matching the `--demo` output.
* **Script.** `python3 docs/learning/check_examples.py` (section `06-orbits-and-angular-rates.md`).
* **Unit tests.**

  ```bash
  cargo test circular_orbit_speed_and_overhead_rates
  cargo test period_rate_and_vis_viva_speed
  cargo test leo_overhead_rate
  cargo test meo_overhead_rate
  cargo test geo_and_lunar_rates_from_period
  cargo test molniya_apogee_rate
  ```

* **Independent physics check.** The `orbit-prop` library propagates real orbits and measures the same rates from simulated geometry. `cargo test -p orbit-prop overhead_pass_rate_matches_scope_eval` checks that a simulated overhead 500 km pass agrees with the formula above.
* **Tool output.** The detailed regime breakdown in `--demo` prints each acquisition calculation, for example `2.0 km / 500 km = 13.8' (825")`.

## Self-check

1. What is the circular speed at 1,000 km altitude? The overhead rate in deg/s?
2. Why does a GEO satellite have a rate vs stars of 15.04"/s but a rate vs ground of zero?
3. A 1 km prediction error at 1,000 km range: how many arcseconds?
4. A camera on a telescope gives a 0.5 x 0.3 deg field. With a 60" pointing error, will a LEO target at 500 km with a 2 km error reliably land in the field?
5. Using vis-viva, what is the Molniya orbit's speed at perigee (r = a x (1 - e))?

<details>
<summary>Answers</summary>

1. v = sqrt(398,600 / 7,378) = 7.35 km/s. omega = 7.35 / 1,000 rad/s = 0.42 deg/s. Note that it is slower than the 500 km pass.
2. It orbits in exactly one sidereal day, the same time Earth takes to turn under it, so it stays fixed over one point on the ground while the stars appear to move past it.
3. 1 / 1,000 x 206,265 = 206".
4. Half short side = 0.3 x 3600 / 2 = 540". Needed = 825 + 60 = 885". Margin 0.61: FAIL. You'd need a search pattern or a wider finder.
5. r = 6,906 km. v = sqrt(398,600 x (2/6,906 - 1/26,560)) = 10.0 km/s, nearly seven times faster than at apogee.

</details>

## Where it lives in the code

* `src/calculations/orbit.rs`: `circular_speed_km_s`, `overhead_rate_arcsec_s`, `rate_from_period_arcsec_s`, `vis_viva_km_s`, `position_uncertainty_arcsec`.
* `src/regimes.rs`: `regimes()` (the table in 6.1), `telescope_acquisition`, `telescope_dwell`, `telescope_depth`.
* `src/constants.rs`: `MU_EARTH`, `EARTH_RADIUS_KM`, `SIDEREAL_RATE_ARCSEC_PER_S`; module `regimes_limits`: `ACQ_PASS_MARGIN`, `ACQ_WARN_MARGIN`.

**Next:** [Lesson 7: Timing and shutters](07-timing-and-shutters.md)
