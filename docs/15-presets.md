# 15. Presets and their sources

**Reference page.** The built-in hardware offered by the menus. Every value lives in `presets.yaml` at the project root, each with a `source` field saying where it came from. Always confirm against the current spec sheet before a purchase.

## Telescopes

No preset carries an optical throughput figure: no vendor publishes one for these tubes, and every value in `presets.yaml` has a source. The detection check assumes 0.85 and says so.

PlaneWave publishes no corrected image circle for the RC20, RC24, RC700 or the IRDK tubes. Those presets leave it blank, and the sensor-fit check warns and tells you to ask the vendor.

| Preset | D (mm) | FL (mm) | Obstruction | Image circle (mm) | Notes |
|---|---|---|---|---|---|
| PlaneWave DeltaRho 280 | 280 | 770.6 | 63% (entered as diameter) | 55 | Spot 4.0 / 4.1 / 4.5 um RMS at 0 / 21 / 27.5 mm. The vendor page does not say whether 63% is by diameter or area. 32 lb. |
| PlaneWave DeltaRho 350 | 350 | 1050 | 56% diameter | 60 | Spot 4.9 / 6.2 / 7.6 um RMS at 0 / 23 / 30 mm. Some listings, including the current PlaneWave page, show 5.6 / 6.4 um off-axis. The conservative values are used. 46 lb. |
| PlaneWave DeltaRho 500 | 508 | 1537 | 59% diameter | 70 | Spot 3.86 / 4.04 / 6.04 um RMS at 0 / 22 / 35 mm. 165 lb. Also sold on an L-500 as the DRL500 system. |
| PlaneWave FSCT8 | 203.2 | 568 | 60% (entered as diameter) | 42 | Spot 3.6 / 6.0 / 6.4 um RMS at 0 / 16 / 21 mm. The vendor page does not say whether 60% is by diameter or area. 20 lb. |
| PlaneWave CDK12.5 | 318 | 2541 | 42% diameter | 52 | Spot 2.8 / 3.8 / 8.0 um RMS at 0 / 11 / 21 mm. 42 lb. Also sold on an L-350 as the CDK300 system. |
| PlaneWave CDK14 | 356 | 2563 | 48.5% diameter | 52 | Spot 3.1 / 6.0 um RMS at 13 / 35 mm. Vendor page lists both 70 and 52 mm image circles. 52 mm is used. 48 lb. Also sold on an L-350 as the CDK350 system. |
| PlaneWave CDK17 | 432 | 2939 | 48.6% diameter | 70 | Spot 6.5 / 9.6 um RMS at 21 / 26 mm. 106 lb. Also sold as the CDK400 (L-500) and CDK450 (L-550) systems. |
| PlaneWave CDK20 f/6.8 | 508 | 3454 | 39% diameter | 52 | Spot 1.5 / 3.8 / 6.0 um RMS at 0 / 12 / 21 mm. 140 lb. Also sold as the CDK500 (L-500) and CDK550 (L-550) systems. |
| PlaneWave CDK20 f/7.7 | 508 | 3951 | 39% diameter | 52 | Vendor page repeats the f/6.8 spot sizes. 140 lb. Also sold as the CDK550 f/7.77 system. |
| PlaneWave CDK24 | 610 | 3974 | 47% diameter | 70 | Spot 2.4 / 4.0 / 4.8 um RMS at 0 / 26 / 35 mm. 240 lb. Also sold on an L-600 as the CDK600 system. |
| PlaneWave CDK700 | 700 | 4540 | 47% diameter | 70 | Complete system on its own mount: no OTA weight. Pair with the 0.7 m system mount. |
| PlaneWave CDK1000 | 1000 | 6000 | 47% diameter | 100 | Complete system on its own mount: no OTA weight. Pair with the 1.0 m system mount. |
| PlaneWave PF1000 | 1000 | 2251 | 60% diameter | 115 | Prime focus. Complete system on its own mount: no OTA weight. Pair with the 1.0 m system mount. |
| PlaneWave RC20 | 508 | 3556 | 39% diameter | not published | Back focus 147 mm from the racked-in focuser. 140 lb. |
| PlaneWave RC24 | 610 | 6469 | 32% diameter | not published | 240 lb. |
| PlaneWave RC700 | 700 | 8410 | < 30% (30% entered) | not published | Complete system on its own mount: no OTA weight. Pair with the 0.7 m system mount. |
| PlaneWave RC1000 | 1000 | 12200 | 27% diameter | 64 | Complete system on its own mount: no OTA weight. Pair with the 1.0 m system mount. |
| PlaneWave IRDK12.5, IRDK14, IRDK17, IRDK20, IRDK24 | as the matching CDK | as the matching CDK | as the matching CDK | not published | Infrared-optimized, gold-coated, with no corrector lens group listed, so the CDK image circles and spot sizes are not assumed. Weights as the matching CDK. |
| Celestron RASA 11 V2 | 279 | 620 | 114 mm (41% diameter) | 43.3 | The camera sits in front of the aperture, so a large camera body adds obstruction. 43 lb (listings vary from 35 to 43). |

## Cameras

No preset carries a quantum efficiency or read-noise figure. Both vary with gain, mode and vendor binning for the same sensor, so entering a single number from a QE curve would be inventing data. The detection check assumes 0.80 and 3 e- and says so. No preset carries a detector MTF either, so the point spread function counts the pixel aperture but no charge diffusion, and the report says so. Enter `mtf_nyquist` from a measured MTF curve if you have one ([page 17](17-point-spread-function.md)).

| Preset | Pixel (um) | Pixels | Shutter | Notes |
|---|---|---|---|---|
| Sony IMX455 full frame (Moravian C3-61000 PRO, QHY600 PRO) | 3.76 | 9576 x 6388 | Rolling, 39.028 us/line | Line time from the Moravian C3 manual. 2.0 lb (QHY600 PRO). |
| Sony IMX571 APS-C (Moravian C3-26000 PRO) | 3.76 | 6252 x 4176 | Rolling, 34.667 us/line | Line time from the Moravian C3 manual. |
| Sony IMX461 medium format | 3.76 | 11664 x 8750 | Rolling, line time unknown | Exact pixel count varies by vendor. |
| Sony IMX174 global shutter (e.g. QHY174M-GPS) | 5.86 | 1936 x 1216 | Global | Small sensor often used for low-orbit timing work. |

## Mounts

No preset carries an axis-acceleration or settle-time figure. Only the PlaneWave T-600 page gives an acceleration, and only as a best case that depends on payload, so it is not entered. Ask, and enter what you are told. Until then the keyhole is reported on the rate limit alone and the acceleration check asks you to confirm with the vendor.

| Preset | Type | Payload (lb) | Max slew (deg/s) | Notes |
|---|---|---|---|---|
| PlaneWave L-350 (direct drive) | Alt-az (equatorial with wedge) | 100 | 50 | Pointing accuracy and TLE tracking asked at run time. Confirm with the vendor. |
| PlaneWave L-500 (direct drive) | Alt-az or equatorial | 200 | 50 | Same as above. |
| PlaneWave L-550 (direct drive) | Alt-az or equatorial | 300 | 50 | Same as above. |
| PlaneWave L-600 (direct drive) | Alt-az or equatorial | 300 | 50 | Same as above. Longer swing-through, for 24" tubes. |
| PlaneWave T-600 (direct-drive gimbal) | Alt-az or equatorial | 600 | 100 | Vendor quotes up to 100 deg/s and up to 100 deg/s^2, both depending on payload. The acceleration is not entered. |
| PlaneWave 700 Series Gimbal (direct drive) | Alt-az | 800 | 50 | Vendor quotes "> 50 deg/s". |
| PlaneWave 1000 Series Gimbal (direct drive) | Alt-az | 1200 | 50 | Vendor quotes "> 50 deg/s". |
| PlaneWave 0.7 m system mount (CDK700, RC700) | Alt-az | 300 | 50 | The built-in mount of those systems. Capacity is the instrument payload per port, not the tube. |
| PlaneWave 1.0 m system mount (CDK1000, RC1000, PF1000) | Alt-az | 300 | 50 | The built-in mount of those systems. Capacity is the instrument payload, not the tube. |
| iOptron HAE69C-EC (strain-wave) | Entered as equatorial (can run alt-az) | 69 (79 with counterweight) | not entered | Slew rate, pointing and TLE tracking not entered. |

Preset mounts leave pointing accuracy and TLE-tracking support blank on purpose. The tool asks you for them (and for axis acceleration), because they are exactly the questions to put to a vendor. The `--demo` run assumes 30" pointing and leaves TLE tracking as "unknown" so its WARN results show which question is outstanding.

## Check it yourself

1. Open `presets.yaml` and confirm one row of each table, including its `source` field.
2. Confirm the obstruction conversion for the RASA: 114 / 279 = 0.409, so "114 mm" is 41% by diameter and 0.409^2 = 17% by area, which matches the 0.167 blocked fraction on [page 6](06-light-field-focus-fit-checks.md#check-5-collecting-area-and-depth).
3. Run `cargo test presets_parse` to confirm the file parses, including entries that omit the optional fields.

To add your own hardware, see [Code structure](16-code-structure.md#adding-a-preset).
