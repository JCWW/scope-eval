# Lesson 10: Pass prediction

The regime checks in Lessons 6 to 9 judge one representative geometry per regime. Pass prediction uses the real path of one real (or what-if) orbit over your site instead: when the satellite rises and sets, how high it climbs, whether it's sunlit, whether your sky is dark, and whether your mount can follow each pass. This lesson explains the models underneath, which live in the `orbit-prop` library.

**Prerequisites:** [Lesson 6](06-orbits-and-angular-rates.md) (orbits, vis-viva), [Lesson 8](08-mount-dynamics.md) (axis rates, keyhole).

## You will be able to

* Read a two-line element set (TLE) and validate its checksum by hand.
* Get an orbit's period and semi-major axis from a TLE's mean motion.
* Explain the difference between SGP4 and a Keplerian orbit with J2 drift, and when to use each.
* Compute how fast J2 turns an orbit's plane, and explain what makes an orbit sun-synchronous.
* Describe the chain of reference frames from an orbit to an azimuth and elevation at your site.
* Explain how the pass search finds rise, set and culmination, and what it can miss.
* Explain the "Sunlit" and "Dark" columns.

## Key terms

| Term | Meaning |
|---|---|
| **TLE** | Two-line element set: a standard text format for a satellite's mean orbital elements at one time (its *epoch*). |
| **Epoch** | The instant the elements describe. Predictions get worse the further you are from it. |
| **Mean motion (n)** | Orbits per day (in a TLE) or radians per second (in formulas). |
| **SGP4** | Simplified General Perturbations 4: the orbit model TLEs are fitted with. Includes drag and Earth's oblateness. |
| **J2** | The main term describing Earth's equatorial bulge. It makes orbits precess slowly. |
| **RAAN** | Right ascension of the ascending node: where the orbit crosses the equator going north, measured against the stars. |
| **Argument of perigee** | Where in the orbital plane the low point sits. |
| **Mean anomaly** | Where along the orbit the satellite is, as a fraction of the period, in degrees. |
| **TEME / ECEF / SEZ** | Reference frames: inertial (SGP4's output), Earth-fixed and local horizon (south, east, zenith). |
| **GMST** | Greenwich mean sidereal time: Earth's rotation angle relative to the stars. |
| **Culmination** | The highest point of a pass. |
| **Umbra / penumbra** | Earth's full shadow / partial shadow. |

---

## 10.1 Reading a TLE

```
ISS (ZARYA)
1 25544U 98067A   08264.51782528 -.00002182  00000-0 -11606-4 0  2927
2 25544  51.6416 247.4627 0006703 130.5360 325.0288 15.72125391563537
```

The fields you need most:

| Line | Columns | Field | Value here |
|---|---|---|---|
| 1 | 19-32 | Epoch: two-digit year, then day of year with fraction | 2008, day 264.5178 |
| 2 | 9-16 | Inclination, deg | 51.6416 |
| 2 | 18-25 | RAAN, deg | 247.4627 |
| 2 | 27-33 | Eccentricity, with an implied leading "0." | 0.0006703 |
| 2 | 35-42 | Argument of perigee, deg | 130.5360 |
| 2 | 44-51 | Mean anomaly, deg | 325.0288 |
| 2 | 53-63 | Mean motion, revolutions per day | 15.72125391 |
| both | 69 | Checksum | 7 |

**Checksum.** Add every digit on the line (ignoring the last column), counting each minus sign as 1 and ignoring everything else. The last digit of the sum is the checksum. It catches a typo or a paste that lost a character. The tool rejects a TLE whose checksum fails and tells you the line and column.

**Epoch and age.** TLE predictions are best near the epoch and drift as drag and other unmodeled forces accumulate, especially in LEO. If any part of your search window is more than **14 days** from the epoch, the tool warns that pass times may be off by minutes. Fetch a fresh TLE from CelesTrak or Space-Track before relying on a prediction.

## 10.2 From mean motion to the orbit's size

**Period.**

```
period (s) = 86,400 / mean motion (rev/day)
```

ISS: 86,400 / 15.72125 = 5,496 s = 91.6 min.

**Semi-major axis (Kepler's third law).** Convert the mean motion to radians per second, `n = rev/day x 2 pi / 86,400`, then

```
a = (mu / n^2)^(1/3)
```

ISS: n = 0.0011433 rad/s, a = 6,731 km, so an average altitude of 6,731 - 6,378 = 353 km (the ISS's altitude in 2008).

Kepler's third law comes from the same balance of gravity and motion as the circular speed in Lesson 6: `n^2 a^3 = mu`.

## 10.3 Two orbit models

You can give the tool either a TLE or a what-if orbit. They use different models.

**SGP4, for TLEs.** A TLE's elements are not a simple snapshot of position and velocity. They are *mean* elements fitted to tracking data using SGP4, a model that includes atmospheric drag, Earth's oblateness and (for distant orbits, as "SDP4") the Sun and Moon. To get positions back out of a TLE correctly you **must** use SGP4, in the same compatibility mode the catalog used to fit it. Using a plain Keplerian orbit with TLE elements gives errors of many kilometers.

**Keplerian with J2, for what-if orbits.** A two-body ellipse that never changes, plus the slow drift caused by Earth's equatorial bulge. Good for "what would a 550 km, 53 degree orbit look like from here?". It has no drag and no short-period terms, so it drifts from any real satellite by kilometers per day.

## 10.4 J2: why orbits precess

Earth bulges at the equator. That extra mass pulls on an inclined orbit and twists its plane slowly around Earth's axis, like a spinning top that precesses. The main effects (Vallado eq. 9-41):

```
k      = 1.5 x J2 x (R / p)^2 x n,       p = a (1 - e^2),  J2 = 1.0826e-3
dRAAN/dt = -k cos i                       (the node drifts)
dargp/dt =  k (2 - 2.5 sin^2 i)           (perigee rotates)
```

**Worked example (550 km circular, 53 deg).** a = 6,928 km, n = 0.001094 rad/s, k = 1.5 x 1.0826e-3 x (6,378 / 6,928)^2 x 0.001094 = 1.50e-6 rad/s. dRAAN/dt = -1.50e-6 x cos(53 deg) = -9.04e-7 rad/s = **-4.49 deg/day**. The orbit plane swings westward by about 4.5 degrees every day, which is why the pass times of a satellite shift from night to night.

**Sun-synchronous orbits.** For an orbit tilted past 90 degrees (retrograde), cos i is negative, so the node drifts eastward. Choose the altitude and inclination so the drift is exactly 360 degrees per year (0.9856 deg/day), and the orbit keeps the same angle to the Sun all year. An 800 km orbit at 98.6 degrees does this. Check it with the formula: you should get about 0.985 deg/day.

## 10.5 From orbit to azimuth and elevation

The propagator gives the satellite's position in an inertial frame. To know where to point, the library converts in steps:

1. **TEME to ECEF.** SGP4 outputs positions in TEME, a frame fixed against the stars. Earth turns under that frame. Rotating by the Greenwich mean sidereal time (GMST), Earth's current rotation angle, gives an Earth-fixed (ECEF) position.
2. **Site position.** Your latitude, longitude and altitude become an ECEF vector using the WGS-84 ellipsoid (Earth is slightly flattened, so latitude is measured from the local vertical, not the center).
3. **ECEF to SEZ.** Subtract the site from the satellite, then rotate into the local south-east-zenith frame.
4. **SEZ to azimuth and elevation.** Elevation is the angle above the horizon plane; azimuth is the compass bearing, measured from north through east.

**GMST.** The IAU-1982 expression, with T in Julian centuries since J2000.0 (2000-01-01 12:00 UT):

```
GMST (s) = 67,310.54841 + (876,600 h + 8,640,184.812866) T + 0.093104 T^2 - 6.2e-6 T^3
```

Take the result modulo 86,400 s and scale to 360 degrees. Vallado's Example 3-5: 1992 August 20, 12:14 UT gives GMST = 152.578788 deg.

**Axis rates.** The library computes azimuth and elevation rates analytically from the relative position and velocity, not by differencing positions, so they are exact for the given state. It also gives the equivalent hour-angle and declination rates for an equatorial mount.

## 10.6 Finding passes

The pass search is a classic root-finding problem: find the times where `elevation - minimum elevation` changes sign.

1. **Coarse scan.** Step through the window at `period / 60`, clamped to 10-300 s (91.6 s for the ISS). Record every sign change.
2. **Bisection.** Narrow each sign change down to 0.1 s by repeatedly halving the interval. These are the rise and set times.
3. **Dense sampling.** Sample the pass every second (and every 0.1 s within 30 s of culmination) to record peak axis rates, accelerations, lighting and darkness.
4. **Golden-section search.** Refine the culmination time, the maximum of elevation.

**What it can miss.** A pass that stays above the minimum elevation for less than one coarse step can fall entirely between two samples, so short grazing passes may be missed.

**The zenith flip.** A pass straight through the zenith needs an instantaneous 180-degree azimuth flip (Lesson 8). The table reports it as the flip divided by the 0.1 s sampling step, about 1,800 deg/s, which fails every alt-az mount. An equatorial mount has the same problem at the celestial pole on its hour-angle axis.

## 10.7 Lighting: Sunlit and Dark

An optical telescope sees a satellite only when two things are true at once:

* **The satellite is sunlit.** It reflects sunlight; in Earth's shadow it's invisible. The library models the shadow as a cone: the **umbra** (no direct sunlight at all) and the **penumbra** (Sun partly hidden). The table reports yes, partial or no over the pass.
* **Your sky is dark.** The Sun must be more than **12 degrees** below your horizon (the end of nautical twilight). The table reports yes, twilight or no.

The best passes are when both hold: shortly after dusk or before dawn, when the ground is dark but a satellite a few hundred kilometers up is still in sunlight. That's why LEO satellites are rarely seen in the middle of a summer night.

## 10.8 Judging the mount on each pass

For each pass, the tool compares the peak axis rate and acceleration with each mount's ratings, using the same headroom thresholds as the regime checks in Lesson 8 (3x for PASS, 1x for WARN). It names the binding axis and quantity. An equatorial mount is judged on its hour-angle and declination axes instead of azimuth and elevation.

---

## Validate it yourself

* **Checksum by hand.** Add the digits of line 1 of the ISS TLE above, counting the four minus signs as 1 each. The digits add to 153, plus 4, for 157: the checksum is 7.
* **Period and altitude.** Compute the ISS's period and average altitude from its mean motion.
* **J2.** Compute the node drift for 800 km at 98.6 deg. Check it against the test `sun_synchronous_orbit_regresses_about_one_degree_per_day`.
* **GMST.** The Julian date of 1992-08-20 12:14 UT is 2,448,855.00972. Compute T and GMST and compare with 152.578788 deg.
* **Script.** `python3 docs/learning/check_examples.py` (section `10-pass-prediction.md`) checks the checksum, ISS period and altitude, both J2 drifts and the GMST example.
* **Against an outside source.** Paste a current ISS TLE from CelesTrak into the tool's pass-prediction menu with your site, then compare rise times and maximum elevations with an independent pass predictor such as Heavens-Above. Expect agreement to within a few seconds and a few tenths of a degree (the library ignores atmospheric refraction, which raises objects near the horizon by up to 0.5 deg, so rise and set times differ slightly).
* **Unit tests.** The `orbit-prop` tests compare the library with published references:

  ```bash
  cargo test -p orbit-prop gmst_matches_vallado_example_3_5
  cargo test -p orbit-prop site_ecef_matches_vallado_example_3_3
  cargo test -p orbit-prop matches_vallado_verification_case_00005
  cargo test -p orbit-prop sun_matches_meeus_example_25a
  cargo test -p orbit-prop moon_matches_meeus_example_47a
  cargo test -p orbit-prop bad_checksum_names_line_and_column
  cargo test -p orbit-prop sun_synchronous_orbit_regresses_about_one_degree_per_day
  cargo test -p orbit-prop exact_zenith_pass_reports_the_azimuth_flip
  ```

## Self-check

1. A TLE has a mean motion of 2.00563 rev/day. What kind of orbit is it?
2. Why must you use SGP4, and not a Keplerian orbit, to propagate a TLE?
3. Why do the pass times of a satellite in a 53 deg orbit shift from day to day?
4. A pass culminates at 88 deg elevation at 02:00 local time in midsummer. Will you see it?

<details>
<summary>Answers</summary>

1. Period 86,400 / 2.00563 = 43,079 s = 11.97 h. a = 26,560 km. It's a half-sidereal-day orbit: GPS (MEO), or Molniya if the eccentricity is high.
2. TLE elements are SGP4 mean elements, fitted with SGP4's drag and perturbation models. Interpreted as a plain ellipse, they give a different orbit, wrong by kilometers.
3. J2 turns the orbit plane by about 4.5 deg per day, and the orbital period doesn't divide evenly into a day, so the satellite comes over at a different time each day.
4. Probably not. At 02:00 in midsummer the Sun is far enough below the horizon that a LEO satellite overhead is likely in Earth's shadow ("Sunlit: no"). Your mount would also need to handle a near-zenith keyhole.

</details>

## Where it lives in the code

* `crates/orbit-prop/src/tle.rs`: TLE parsing and checksum.
* `crates/orbit-prop/src/sgp4_propagator.rs` and `keplerian.rs`: the two propagators behind the `Propagator` trait.
* `crates/orbit-prop/src/time.rs`: Julian dates and `gmst_rad`.
* `crates/orbit-prop/src/frames.rs`: TEME, ECEF, SEZ and azimuth/elevation.
* `crates/orbit-prop/src/observe.rs`: analytic axis rates.
* `crates/orbit-prop/src/passes.rs`: `find_passes`.
* `crates/orbit-prop/src/illumination.rs` and `sun_moon.rs`: shadow, phase and Sun elevation.
* `src/passes_report.rs`: the pass table and the per-pass mount judgment.
* [`crates/orbit-prop/README.md`](../../crates/orbit-prop/README.md): the library's models, references, accuracy and known limitations.

**Next:** [Lesson 11: How the tool judges](11-how-the-tool-judges.md)
