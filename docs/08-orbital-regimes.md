# 8. Orbital regimes

**What you'll learn:** why the same hardware can be excellent for one orbit and useless for another, how the tool derives a representative rate for each orbital regime, and how it grades the telescope, camera and mount against each one. The fourth component, the system's ability to detect the target, has its own page: [Target brightness and detection](10-target-brightness-and-detection.md).

**Before you start:** [Motion and timing](07-motion-and-timing.md). This page generalizes its "rate x time = angle" idea from GEO to every regime.

The eight checks describe the optical system in general. Whether that system can actually observe a target depends on where the target is: how far away, how fast it moves and how well its position is predicted.

---

## Part 1: describing a regime

Each regime is described by a handful of numbers. Two of them are rates, measured two different ways:

* **Rate vs stars.** How fast the target moves against the background stars. This drives timing requirements, rolling-shutter skew and trailing, because positions are measured against the stars.
* **Rate vs ground.** How fast the target moves across the sky as seen from the site. This is what the mount must follow.

| Regime | Representative case | Range (km) | Rate vs stars ("/s) | Rate vs ground ("/s) | Prediction error (km) | Usual mode |
|---|---|---|---|---|---|---|
| LEO | 500 km circular, overhead pass | 500 | ~3,140 (0.87 deg/s) | ~3,140 | 2 | rate track |
| MEO | GPS-like 20,200 km, overhead | 20,200 | ~40 | ~40 | 2 | rate track |
| GEO | 35,786 km altitude | ~37,000 | 15.04 | 0 | 2 | stare |
| HEO | Molniya near apogee | ~39,800 | ~7.8 | ~7.3 | 5 | rate track |
| Cislunar | Lunar distance | ~384,400 | ~0.55 | ~14.5 | 50 | sidereal |

The rates come from three different formulas, each suited to a different kind of orbit.

### LEO and MEO: speed over distance

A satellite in a circular orbit at altitude h moves at

```
v = sqrt(mu / (R + h))          mu = 398,600 km^3/s^2 (Earth's gravity), R = 6,378 km
```

Seen from directly below, its angular rate is speed divided by distance (the small-angle rule again):

```
omega (rad/s) ~ v / h           x 206,265 for arcsec/s
```

For 500 km: v = sqrt(398,600 / 6,878) = 7.61 km/s, so omega = 7.61 / 500 = 0.0152 rad/s = 3,140 "/s = 0.87 deg/s.

An overhead pass is the fastest geometry, which makes it the right worst case for mount and timing requirements. Lower passes are slower. Earth's rotation is ignored here, which is a small error for LEO and roughly 10% for MEO. At these rates the stars' own motion (15"/s) is negligible, so rate vs ground is taken equal to rate vs stars.

### GEO and cislunar: one circle per period

For distant objects the observer's own motion matters, so the v / h shortcut no longer works. Instead, use the period: an object completing one circle (1,296,000") per period P moves against the stars at

```
rate vs stars = 1,296,000" / P
```

* **GEO:** P = one sidereal day (86,164 s), giving 15.04 "/s. The ground rotates at the same rate, so rate vs ground is 0.
* **Cislunar:** the Moon's period (27.32 days) gives about 0.55 "/s against the stars. The ground turns under it at nearly the full 15.04 "/s, leaving about 14.5 "/s vs ground.

### HEO: vis-viva at apogee

Speed anywhere on an elliptical orbit comes from the vis-viva equation, where r is the current distance from Earth's center and a is the semi-major axis:

```
v = sqrt( mu x (2/r - 1/a) )
```

For a Molniya orbit (a = 26,560 km, eccentricity 0.74), apogee is at r = a x (1 + e) = 46,214 km from Earth's center, where v = 1.50 km/s. Divided by the apogee altitude (about 39,800 km) that is about 7.8 "/s against the stars. Rate vs ground is approximated as the difference from the sidereal rate, about 7.3 "/s.

### Prediction errors are assumptions

The along-track prediction errors (2 km for LEO, MEO and GEO, 5 km for HEO, 50 km for cislunar) are placeholder values for a reasonably fresh public element set. Real errors depend heavily on the catalog source and the age of the prediction. Edit them in `regimes()` in `src/regimes.rs` to match your data.

---

## Part 2: telescope checks

### Acquisition field (graded)

**Question.** Will the target land in the field when the telescope points at the predicted position?

```
prediction error (") = prediction error (km) / range (km) x 206,265
needed (")           = prediction error + mount pointing error
margin               = (short side of the field / 2) / needed
```

The short side is used because the error can lie in any direction. Mount pointing error comes from the mount input, or 60" RMS if not entered. PASS if margin >= 2, WARN if >= 1, FAIL below 1.

**Worked example,** DeltaRho 350 + IMX455 for LEO: 2 km / 500 km x 206,265 = 825". Add 30" of pointing error for 855". Half the short side is 1.31 deg / 2 = 2,359". Margin = 2,359 / 855 = 2.8x, PASS. A CDK17 with the same camera has a half short side of only about 843", margin 0.99, FAIL.

**Wide fields matter for LEO because a small along-track error is a large angle at short range.** The same 2 km at GEO range is only 11".

### Field dwell (INFO)

How long an untracked target stays in the field:

```
dwell (s) = short side of the field (") / rate vs ground ("/s)
```

For LEO on the DeltaRho 350: 4,716" / 3,140 "/s = 1.5 s. This matters for a "stare and catch" approach, where the telescope waits for a satellite to cross. A GEO target never leaves a stopped telescope's field.

### Depth relevance (INFO)

Reports the effective collecting area, depth vs the reference, and what usually limits detection in the regime. LEO targets are usually bright, so tracking and timing dominate. GEO and especially cislunar targets are faint, so collecting area and search speed dominate. The absolute answer (signal-to-noise and limiting magnitude for a representative target) comes from the system detection check on [page 10](10-target-brightness-and-detection.md).

---

## Part 3: camera checks

### Timestamp accuracy (graded)

**Question.** Is each image timestamped accurately enough that timing error doesn't dominate the measured position?

```
required timing (s) = 0.25 x binned plate scale (") / rate vs stars ("/s)
position error (")  = rate vs stars x timestamp accuracy
```

The budget is a quarter of a binned pixel of target motion. PASS if the entered accuracy meets the requirement. WARN if within 4x. FAIL beyond that.

| Regime | Required timing (DeltaRho 350, 1.48 "/px binned) |
|---|---|
| LEO | 0.12 ms |
| MEO | 9.3 ms |
| GEO | 24.6 ms |
| HEO | 48 ms |
| Cislunar | 670 ms |

A PC clock with USB latency (tens of ms) is borderline even for GEO and hopeless for LEO. LEO needs GPS hardware timestamping.

### Shutter skew (graded)

**Question.** How far does the target move against the stars while a rolling-shutter sensor reads from top to bottom?

```
readout time (s) = rows x line time
skew (")         = rate vs stars x readout time
```

PASS for a global shutter, or if the skew is under 0.25 binned pixel. WARN if larger but under 10% of the frame height: correctable, but the software must timestamp each row separately (t = t_first_row + row x line time). FAIL above 10% of the frame height, where the frame geometry is badly distorted and a global-shutter camera or a small region-of-interest readout is the better answer. WARN if the line time is unknown.

Example with the IMX455 (6,388 rows x 39.028 us = 0.249 s readout):

| Regime | Skew | Fraction of DeltaRho 350 frame height (4,716") | Status |
|---|---|---|---|
| LEO | 783" | 16.6% | FAIL |
| MEO | 9.9" | 0.2% | WARN (correctable) |
| GEO | 3.75" | 0.1% | WARN (correctable) |
| Cislunar | 0.14" | negligible | PASS |

### Exposure vs trailing (INFO)

**Question.** How long can an exposure be before relative motion smears something?

```
crossing time (s)      = seeing FWHM (") / rate vs stars ("/s)
streak per second (px) = rate vs stars / binned plate scale
```

The target and the stars move relative to each other, so one of them always smears in long exposures: the target if the mount follows the stars, the stars if the mount follows the target. For LEO the crossing time is under a millisecond (2.5 / 3,140 = 0.8 ms), which is why LEO observing means rate tracking with streaked stars. For cislunar it is 2.5 / 0.549 = 4.6 s.

---

## Part 4: mount checks

### Tracking rate (graded)

**Question.** Can the mount move as fast as the target?

```
required rate (deg/s) = rate vs ground / 3600
headroom              = mount maximum axis rate / required rate
```

PASS if headroom >= 3x (margin for acceleration and corrections), WARN if >= 1x, FAIL below. In stare mode (GEO) the mount only has to point and hold, so it passes. If the mount's slew rate is unknown, the result is WARN when the required rate is significant (above 0.1 deg/s, which in practice means LEO), otherwise INFO.

### The alt-az zenith keyhole

For an alt-azimuth mount, a pass that goes nearly overhead forces the azimuth axis to swing around quickly. Near the zenith the sky is locally flat, so for a target moving at angular rate omega and passing at a closest zenith distance z (in radians), the peak azimuth rate is about

```
azimuth rate ~ omega / z
```

Setting the azimuth rate equal to the mount's maximum gives the closest followable zenith distance, z = omega / max rate, and therefore the highest followable pass:

```
highest pass elevation (deg) = 90 - degrees(omega / max axis rate)
```

**Worked example,** LEO on a PlaneWave L-350 (50 deg/s): z = 0.87 / 50 rad = 1.0 deg, so passes up to 89 deg elevation can be followed. A mount limited to 3 deg/s could only follow passes up to about 73 deg. PASS if >= 85 deg, WARN if >= 70 deg, FAIL below.

For equatorial mounts the keyhole is near the celestial pole and is noted but not graded. When the mount's acceleration rating is entered, acceleration usually tightens the keyhole further; [Mount dynamics](09-mount-dynamics.md#step-2-the-acceleration-limited-keyhole) explains how.

### Acceleration and slew-and-settle (graded where it matters)

Two more mount checks use the mount's acceleration and settle time. They have their own page: [Mount dynamics](09-mount-dynamics.md).

### Non-sidereal tracking (graded where required)

Rate tracking needs the mount's control software to follow a predicted path, for example from a TLE. LEO, MEO and HEO require it. PASS if supported, FAIL if not, WARN if unknown (ask the vendor). For GEO (stare) and cislunar (sidereal is usually adequate) it is reported as INFO.

---

## Part 5: reading the regime results

The compact table printed with every evaluation shows the worst status per component:

```
[----] Orbital regimes (worst status per component; details via the menu or --demo)
       Regime                          Telescope  Camera   Mount    System   Overall
       LEO (Low Earth orbit)           PASS       FAIL     WARN     WARN     FAIL
       MEO (Medium Earth orbit)        PASS       WARN     WARN     WARN     WARN
       GEO (Geosynchronous orbit)      PASS       WARN     PASS     WARN     WARN
       HEO (Highly elliptical orbit)   PASS       WARN     WARN     WARN     WARN
       CIS (Cislunar space)            PASS       PASS     PASS     WARN     WARN
```

This is the demo's DeltaRho 350 + IMX455 on an L-350 with GPS timestamps. Reading it:

* The telescope is fine everywhere.
* The camera fails LEO because of rolling-shutter skew, and only needs per-row timestamps for MEO, GEO and HEO. The fix for LEO is a global-shutter camera, which the demo's RASA 11 + IMX174 row confirms.
* The mount's warnings come from unanswered vendor questions: whether its software supports TLE tracking, and what its axis acceleration is.
* The System column is WARN in every regime, and that is not a finding about the hardware. The demo enters no quantum efficiency, throughput, sky brightness or read noise, so the detection check substitutes generic defaults and refuses to grade a PASS on them. Enter real values and the column grades normally.

---

## Check it yourself

**By hand.** Work these for **MEO** on the DeltaRho 350 + IMX455 (binned plate scale 1.477 "/px, short side 4,716", 30" pointing).

1. **Speed.** v = sqrt(398,600 / (6,378 + 20,200)) = **3.873 km/s**.
2. **Rate.** 3.873 / 20,200 x 206,265 = **39.5 "/s**.
3. **Acquisition.** 2 / 20,200 x 206,265 = 20.4". Needed = 20.4 + 30 = 50.4". Margin = 2,359 / 50.4 = **46.8x**, PASS.
4. **Dwell.** 4,716 / 39.54 = **119 s**.
5. **Timing.** 0.25 x 1.477 / 39.54 = **9.3 ms**.
6. **Skew.** 39.54 x 0.249 = **9.9"**, about 6.7 binned pixels and 0.2% of the frame height, so WARN (correctable).
7. **Crossing time.** 2.5 / 39.54 = **63 ms**.
8. **Molniya.** sqrt(398,600 x (2 / 46,214 - 1 / 26,560)) = 1.497 km/s, then 1.497 / 39,836 x 206,265 = **7.75 "/s**.
9. **Moon.** 1,296,000 / (27.32 x 86,400) = **0.549 "/s**.
10. **Keyhole.** For a 3 deg/s mount on LEO: z = 0.872 / 3 = 0.291 rad = 16.7 deg, so the highest pass is 90 - 16.7 = **73.3 deg**, WARN.

**Against the tests.**

| Concept | Command |
|---|---|
| LEO and MEO overhead rates | `cargo test overhead_rate` and `cargo test circular_orbit_speed` |
| GEO and lunar rates from period | `cargo test geo_and_lunar_rates_from_period` |
| Molniya apogee rate and vis-viva | `cargo test molniya_apogee_rate` and `cargo test period_rate_and_vis_viva_speed` |
| L-350 keyhole on LEO | `cargo test keyhole_l350_leo` |
| GEO timing requirement | `cargo test geo_timing_requirement` |

**In the tool.** Run `cargo run --release -- --demo` and find the MEO block of the detailed breakdown for DeltaRho 350 + IMX455. Its header should read `vs stars 39.54"/s`, and the checks should show an acquisition margin of 46.8x, a dwell of 119.3 s, a timing requirement of 9.3 ms, a skew of 9.9" and a crossing time of 63.2 ms. In the comparison's regime matrix, the CDK17 row should start with `F` for LEO (its telescope fails acquisition).

Next: [Mount dynamics](09-mount-dynamics.md).
