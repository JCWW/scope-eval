# 15. Presets and their sources

**Reference page.** The built-in hardware offered by the menus. Every value lives in `presets.yaml` at the project root, each with a `source` field saying where it came from. Always confirm against the current spec sheet before a purchase.

## Telescopes

No preset carries an optical throughput figure: no vendor publishes one for these tubes, and every value in `presets.yaml` has a source. The detection check assumes 0.85 and says so.

| Preset | D (mm) | FL (mm) | Obstruction | Image circle (mm) | Notes |
|---|---|---|---|---|---|
| PlaneWave DeltaRho 350 | 350 | 1050 | 56% diameter | 60 | Spot 4.9 / 6.2 / 7.6 um RMS at 0 / 23 / 30 mm. Some listings show 5.6 / 6.4 um off-axis. The conservative values are used. 46 lb. |
| PlaneWave DeltaRho 500 | 508 | 1537 | 59% diameter | 70 | 165 lb. No spot data. |
| PlaneWave CDK14 | 356 | 2563 | 48.5% diameter | 52 | Spot 3.1 / 6.0 um RMS at 13 / 35 mm. Vendor page lists both 70 and 52 mm image circles. 52 mm is used. 48 lb. |
| PlaneWave CDK17 | 432 | 2939 | 49% diameter | 70 | Weight not entered. |
| Celestron RASA 11 V2 | 279 | 620 | 114 mm (41% diameter) | 43.3 | The camera sits in front of the aperture, so a large camera body adds obstruction. 43 lb (listings vary from 35 to 43). |

## Cameras

No preset carries a quantum efficiency or read-noise figure. Both vary with gain, mode and vendor binning for the same sensor, so entering a single number from a QE curve would be inventing data. The detection check assumes 0.80 and 3 e- and says so.

| Preset | Pixel (um) | Pixels | Shutter | Notes |
|---|---|---|---|---|
| Sony IMX455 full frame (Moravian C3-61000 PRO, QHY600 PRO) | 3.76 | 9576 x 6388 | Rolling, 39.028 us/line | Line time from the Moravian C3 manual. 2.0 lb (QHY600 PRO). |
| Sony IMX571 APS-C (Moravian C3-26000 PRO) | 3.76 | 6252 x 4176 | Rolling, 34.667 us/line | Line time from the Moravian C3 manual. |
| Sony IMX461 medium format | 3.76 | 11664 x 8750 | Rolling, line time unknown | Exact pixel count varies by vendor. |
| Sony IMX174 global shutter (e.g. QHY174M-GPS) | 5.86 | 1936 x 1216 | Global | Small sensor often used for low-orbit timing work. |

## Mounts

No preset carries an axis-acceleration or settle-time figure: none of these vendors publish them. Ask, and enter what you are told. Until then the keyhole is reported on the rate limit alone and the acceleration check asks you to confirm with the vendor.

| Preset | Type | Payload (lb) | Max slew (deg/s) | Notes |
|---|---|---|---|---|
| PlaneWave L-350 (direct drive) | Alt-az (equatorial with wedge) | 100 | 50 | Pointing accuracy and TLE tracking asked at run time. Confirm with the vendor. |
| PlaneWave L-500 (direct drive) | Alt-az or equatorial | 200 | 50 | Same as above. |
| iOptron HAE69C-EC (strain-wave) | Entered as equatorial (can run alt-az) | 69 (79 with counterweight) | not entered | Slew rate, pointing and TLE tracking not entered. |

Preset mounts leave pointing accuracy and TLE-tracking support blank on purpose. The tool asks you for them (and for axis acceleration), because they are exactly the questions to put to a vendor. The `--demo` run assumes 30" pointing and leaves TLE tracking as "unknown" so its WARN results show which question is outstanding.

## Check it yourself

1. Open `presets.yaml` and confirm one row of each table, including its `source` field.
2. Confirm the obstruction conversion for the RASA: 114 / 279 = 0.409, so "114 mm" is 41% by diameter and 0.409^2 = 17% by area, which matches the 0.167 blocked fraction on [page 6](06-light-field-focus-fit-checks.md#check-5-collecting-area-and-depth).
3. Run `cargo test presets_parse` to confirm the file parses, including entries that omit the optional fields.

To add your own hardware, see [Code structure](16-code-structure.md#adding-a-preset).
