# 3. Inputs and where to find them

**What you'll learn:** every number the tool asks for, its units, where it comes from and which checks use it. Read [Key terms](02-key-terms.md) first if any name below is unfamiliar.

Only a handful of inputs are required: seeing, aperture, focal length, obstruction, image circle, pixel size and sensor dimensions. Everything else can be left blank. A blank input either skips the part of a check that needs it or substitutes a documented default, and the report says which.

## Site

| Input | Units | Where it comes from | Used by |
|---|---|---|---|
| Seeing FWHM | arcsec | Your site (2.5" is a reasonable default for a mid-elevation suburban site) | Checks 2, 3, 4 |
| Sky background brightness | mag/arcsec^2 | Site measurement or a dark-sky map (optional, 21.0 assumed if blank) | Regime: detection |

## Telescope

| Input | Units | Where it comes from | Used by |
|---|---|---|---|
| Aperture D | mm | Telescope spec sheet | Checks 5, 7 |
| Focal length FL | mm | Spec sheet, or aperture x f-ratio | Checks 2, 3, 4, 6, 7 |
| Central obstruction | % | Spec sheet. **Note whether it is quoted by diameter or by area.** | Check 5 |
| Image circle | mm | Spec sheet ("corrected field," "image circle") | Check 1 |
| RMS spot size(s) | um at a field radius (mm) | Spec sheet optical performance section (optional) | Check 4 |
| Back focus | mm | Spec sheet (optional) | Check 8 |
| OTA weight | lb | Spec sheet (optional) | Check 8 |
| Optical throughput | fraction 0..1 | Vendor, or measured (optional, 0.85 assumed if blank) | Regime: detection |

Obstruction can be typed as a percent (56) or a decimal (0.56). The tool treats any value above 1 as a percent.

## Camera

| Input | Units | Where it comes from | Used by |
|---|---|---|---|
| Pixel size | um | Camera spec sheet | Checks 2, 3 |
| Sensor width and height | pixels | Camera spec sheet | Checks 1, 6 |
| Read noise | e- RMS | Camera spec sheet (optional, 3 e- assumed if blank) | Check 2, regime: detection |
| Quantum efficiency | fraction 0..1 | Camera QE curve (optional, 0.80 assumed if blank) | Regime: detection |
| Shutter type and line time | rolling/global, us | Camera manual (optional) | Timing reference, regime: camera |
| Camera weight | lb | Camera spec sheet (optional) | Check 8 |

## Mount

| Input | Units | Where it comes from | Used by |
|---|---|---|---|
| Mount type | alt-az / equatorial | Mount spec sheet | Regime: mount |
| Payload rating | lb | Mount spec sheet (optional) | Check 8 |
| Maximum slew rate | deg/s | Mount spec sheet (optional) | Regime: mount |
| Maximum acceleration | deg/s^2 | Mount spec sheet (optional) | Regime: acceleration, keyhole, slew |
| Settle time | s | Mount spec sheet or measured (optional, 2.0 s assumed if blank) | Regime: slew and settle |
| Pointing accuracy | arcsec RMS | Mount spec sheet, after a pointing model (optional, 60" assumed if blank) | Regime: telescope |
| Non-sidereal (TLE) tracking | yes / no / unsure | Mount control software documentation | Regime: mount |

## Your setup and target

| Input | Units | Where it comes from | Used by |
|---|---|---|---|
| Accessories weight | lb | Your estimate: focuser, dew heaters, cables, filter wheel, dovetail | Check 8 |
| Back focus required | mm | Sum of your camera train's optical path lengths (optional) | Check 8 |
| Timestamp accuracy | ms | Your timing chain: roughly 20 ms for a PC clock plus USB latency, 0.1 ms or better for GPS hardware timestamping | Regime: camera |
| Target apparent magnitude | mag | Your own catalog (optional, derived per regime if blank) | Regime: detection |
| Exposure time | s | Your choice (optional, trail-limited value if blank) | Regime: detection |

## Values that are range-checked

A hand-entered photometric or dynamics value outside a plausible range is treated as not entered rather than trusted, so a typo degrades the report instead of corrupting it. The ranges are QE and throughput 0.01 to 1.0, sky brightness 15 to 24 mag/arcsec^2, read noise 0.1 to 100 e-, axis acceleration 1e-4 to 1000 deg/s^2, axis rate 1e-3 to 1000 deg/s and settle time 0 to 600 s. NaN and the infinities always fail. The bounds live in the `plausible_ranges` module of `src/constants.rs`.

## Check it yourself

1. **Obstruction as percent or decimal.** Evaluate a custom telescope twice, once with obstruction `56` and once with `0.56`. Check 5 should report the same effective area both times.
2. **Focal length from f-ratio.** A spec sheet that gives only "356 mm f/7.2" implies FL = 356 x 7.2 = 2563 mm, which is the CDK14 preset's value.
3. **Range checks.** Run `cargo test plausible` to see the range-check tests (`plausible_accepts_a_value_in_range`, `plausible_rejects_out_of_range`, `plausible_rejects_nan_and_infinity`). Then run `cargo test resolve_rejects` for the tests that show a QE above 1, a sky brightness of 2.1 instead of 21.0 and a NaN are all rejected without discarding the other inputs.
4. **Defaults are named.** Evaluate any configuration leaving QE, throughput, sky and read noise blank, then open the detailed regime evaluation. The detection check should list each assumed value by name. `cargo test resolve_names_every_assumed_input` checks this in code.

Next: [How an evaluation works](04-how-an-evaluation-works.md).
