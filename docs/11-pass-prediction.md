# 11. Pass prediction

**What you'll learn:** how to replace the single representative geometry of the regime checks with the real path of one orbit over your site, and how the tool decides, pass by pass, whether each mount can follow.

**Before you start:** [Orbital regimes](08-orbital-regimes.md) for the keyhole and [Mount dynamics](09-mount-dynamics.md) for acceleration. This page is where those formulas meet a real orbit.

The regime checks judge one worst-case pass per regime. **Predict passes for a satellite** in the main menu instead lists every pass in a time window and judges each one against every evaluated mount.

## What you enter

1. **Your site's location**: latitude (north positive), longitude (east positive) and altitude in metres. It is asked for once and can be changed under **Change site conditions**.
2. **The orbit**, one of:
   * *Paste a TLE* from CelesTrak or Space-Track (two lines, or three with a name line first). It is propagated with SGP4, the model TLEs are fitted with. If any part of the search window is more than 14 days from the TLE's epoch, the tool warns that pass times may be off by minutes.
   * *Define a what-if orbit*: perigee and apogee altitude, inclination, right ascension of the ascending node, argument of perigee, mean anomaly and element epoch. It is propagated as a Keplerian orbit with J2 drift, which is right for "what would a 550 km, 53 degree orbit look like from here" but not for tracking a particular object.
3. **The window**: start time in UTC (blank for now), length (24 hours by default, 720 at most) and minimum elevation (10 degrees by default).

## How passes are found

The search runs in four steps (`crates/orbit-prop/src/passes.rs`):

1. **Coarse scan.** Step through the window at one sixtieth of the orbital period, clamped to between 10 and 300 seconds (about 93 seconds for the ISS), watching for the elevation crossing the minimum.
2. **Refine rise and set.** Narrow each crossing by bisection to 0.1 s.
3. **Sample the pass.** Every 1 s (and every 0.1 s within 30 s of culmination), record the axis rates, accelerations, satellite lighting and site darkness, keeping the peaks.
4. **Refine culmination.** Find the exact peak elevation by golden-section search.

A pass that stays above the minimum elevation for less than one coarse step can be missed. For LEO that means grazing passes that never get far above the minimum elevation.

## A worked example

A what-if orbit at 550 km and 53 degrees, seen from 40 N 75 W, with a PlaneWave L-350 whose acceleration was entered as 2 deg/s^2. The inputs are: perigee 550 km, apogee blank (same as perigee, so circular), inclination 53 deg, right ascension of the ascending node, argument of perigee and mean anomaly all 0, element epoch blank (so it equals the search start), search start **2026-10-04 00:00 UTC**, 24 hours, 10 degrees minimum elevation. The first three passes:

```
 #  Rise (UTC)            Set (UTC)  Duration MaxEl  Az rate El rate Sunlit  Dark     | Mount 1
 1  2026-10-04 04:56:04 04:58:50     2m46s  11.4   0.245   0.032 no      yes      | [PASS] az rate 203.8x
 2  2026-10-04 06:32:15 06:40:37     8m22s  86.3  11.604   0.686 no      yes      | [WARN] az accel 1.3x
 3  2026-10-04 08:13:07 08:19:47     6m40s  22.3   0.366   0.080 partial yes      | [PASS] az rate 136.5x
```

## Reading the columns

* **Az rate, El rate** are the peak axis rates an alt-az mount needs during the pass, in deg/s. Pass 2 culminates at 86 degrees, so its azimuth axis has to swing 11.6 deg/s near the zenith: the alt-az keyhole, computed from the real pass rather than from a formula. A pass straight through the zenith needs an instantaneous 180-degree azimuth flip; the table reports it as the flip divided by the 0.1 s sampling step (about 1800 deg/s), which fails every alt-az mount. An equatorial mount has the same problem at the celestial pole on its hour-angle axis.
* **Sunlit** says whether the satellite is in sunlight for the whole pass (yes), part of it (partial) or none of it (no). An optical sensor sees only sunlit satellites.
* **Dark** says whether your sky is dark, meaning the Sun is more than 12 degrees below your horizon. **yes** means dark for the whole pass. **twilight** means dark for only part of the pass. **no** means never dark during the pass; a pass that sits entirely in twilight, with the Sun between 0 and 12 degrees below the horizon, also reads **no**.
* **Mount N** compares the pass's peak axis rate and acceleration with the mount's ratings, using the thresholds of the regime mount checks (`RATE_PASS_HEADROOM`, `ACCEL_PASS_HEADROOM` and the rest in `src/constants.rs`). The note names the axis and quantity that bind and their headroom. An equatorial mount is judged on its hour-angle and declination axes. A rating that is unknown gives WARN when the pass needs real speed and INFO when it doesn't.
* `<` before a rise time means the satellite was already up when the window started; `>` after a set time means it was still up when the window ended.

The best pass for an optical observer is sunlit **and** dark: the satellite in sunlight while your sky is dark. In the example none of the first three qualifies fully; pass 3 is partly sunlit in a dark sky.

The geometry comes from the `orbit-prop` library in `crates/orbit-prop`, whose README documents its models, accuracy and limits.

## Check it yourself

**Reproduce the table exactly.** Build with `cargo build --release`, then pipe the answers to the prompts into the program. This evaluates the DeltaRho 350 + IMX455 on an L-350 with 2 deg/s^2 acceleration, then predicts passes with the inputs above:

```bash
printf '\n\n1\n1\n1\n1\n\n2\n3\n\n\n\n\n\n\n5\n40\n-75\n\n2\n550\n\n53\n\n\n\n\n2026-10-04T00:00\n\n\n8\n' \
  | ./target/release/scope-eval | grep -A8 "Rise (UTC)"
```

The first three rows should match the example. (If you change the preset lists, the menu numbers in that string may need adjusting.)

**Check pass 2 against the keyhole formula, by hand.** The formula from [page 8](08-orbital-regimes.md#the-alt-az-zenith-keyhole) should land near the measured 11.6 deg/s.

1. Speed at 550 km: v = sqrt(398,600 / 6,928) = 7.59 km/s. At 86.3 deg elevation the range is close to the altitude, about 551 km, so omega = 7.59 / 551 = 0.0138 rad/s = 0.79 deg/s.
2. Closest zenith distance: z = 90 - 86.3 = 3.7 deg = 0.0646 rad.
3. Peak azimuth rate: omega / z = 0.79 / 0.0646 = **about 12 deg/s**, close to the 11.6 the tool measured from the real path. The formula ignores Earth's rotation and the exact slant geometry, so a few percent difference is expected.
4. Peak azimuth acceleration, in radians: 0.6495 x (0.0138 / 0.0646)^2 = 0.0296 rad/s^2 = **about 1.7 deg/s^2**. Against the 2 deg/s^2 rating that is a headroom of about 1.2x, between 1x and 3x, so WARN. The tool reports 1.3x from the real path.

**Against the tests.**

| Concept | Command |
|---|---|
| Per-pass mount judgment (headroom, binding axis, unknown ratings) | `cargo test --bin scope-eval passes_report` |
| Equatorial mounts use hour-angle and declination axes | `cargo test equatorial_mount_uses_hour_angle` |
| Stale-TLE warning across the whole window | `cargo test stale_tle` and `cargo test long_search_from_a_fresh_tle` |
| The orbit-prop library against published references | `cargo test -p orbit-prop` |

Next: [Comparing configurations](12-comparing-configurations.md).
