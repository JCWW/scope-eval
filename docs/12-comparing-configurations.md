# 12. Comparing configurations

**What you'll learn:** how to read the three comparison tables, and how to decide which column matters for your mission. No configuration wins every column.

**Before you start:** pages 5 to 10. Every column here is a number derived on one of those pages.

## The comparison output

Choose **Compare all evaluated configurations**, or run `--demo`. Example from the demo at 2.5" seeing, with GPS timestamps (0.1 ms) and 30" mount pointing assumed:

```
Configuration                  "/px   px/*  bin FOVdeg2 Area m2   dMag Search CFZ+/-  Load%
DeltaRho 350 + IMX455          0.74    3.4  2x2    2.58  0.0660  +0.00  1.00x   12.1     58
RASA 11 + IMX455               1.25    2.0  1x1    7.39  0.0509  -0.28  2.21x    6.6     72
CDK14 + IMX455                 0.30    8.3  4x4    0.43  0.0761  +0.15  0.19x   69.6     60
CDK17 + IMX455                 0.26    9.5  4x4    0.33  0.1120  +0.57  0.22x   62.1     62
DeltaRho 500 + IMX461          0.50    5.0  3x3    2.01  0.1321  +0.75  1.56x   12.3     90
RASA 11 + IMX174 (global)      1.95    1.3  1x1    0.69  0.0509  -0.28  0.21x    6.6     48

 Status by check (P=pass W=warn F=fail i=info):
                              1 2 3 4 5 6 7 8
DeltaRho 350 + IMX455         P P P W i i W P
RASA 11 + IMX455              P P P i i i W W
CDK14 + IMX455                P F W P i i P P
CDK17 + IMX455                P F W P i i P P
DeltaRho 500 + IMX461         P W W P i i W W
RASA 11 + IMX174 (global)     P W W i i i W P

 Orbital regimes, telescope/camera/mount/system (P=pass W=warn F=fail i=info):
                              LEO       MEO       GEO       HEO       CIS
DeltaRho 350 + IMX455         P/F/W/W   P/W/W/W   P/W/P/W   P/W/W/W   P/P/P/W
RASA 11 + IMX455              P/W/W/W   P/W/W/W   P/W/P/W   P/W/W/W   P/P/i/W
CDK14 + IMX455                W/F/W/W   P/W/W/W   P/W/P/W   P/W/W/W   P/P/P/W
CDK17 + IMX455                F/F/W/W   P/W/W/W   P/W/P/W   P/W/W/W   P/P/P/W
DeltaRho 500 + IMX461         P/W/W/W   P/W/W/W   P/W/P/W   P/W/W/W   P/W/P/W
RASA 11 + IMX174 (global)     W/P/W/W   P/P/W/W   P/P/P/W   P/P/W/W   P/P/P/W
```

### Table 1: headline metrics

| Column | Meaning | Derived on |
|---|---|---|
| "/px | Native plate scale | [Check 2](05-image-quality-checks.md#check-2-sampling-plate-scale-vs-seeing) |
| px/* | Pixels across a star at native resolution | [Check 2](05-image-quality-checks.md#check-2-sampling-plate-scale-vs-seeing) |
| bin | Recommended square bin | [Check 2](05-image-quality-checks.md#check-2-sampling-plate-scale-vs-seeing) |
| FOVdeg2 | Field area in square degrees | [Check 6](06-light-field-focus-fit-checks.md#check-6-field-of-view-and-search-speed) |
| Area m2 | Effective collecting area | [Check 5](06-light-field-focus-fit-checks.md#check-5-collecting-area-and-depth) |
| dMag | Depth vs the reference (positive means fainter) | [Check 5](06-light-field-focus-fit-checks.md#check-5-collecting-area-and-depth) |
| Search | Etendue vs the reference | [Check 6](06-light-field-focus-fit-checks.md#check-6-field-of-view-and-search-speed) |
| CFZ+/- | Critical focus zone half-width, um | [Check 7](06-light-field-focus-fit-checks.md#check-7-focus-tolerance) |
| Load% | Payload as a percent of mount rating ("-" if unknown) | [Check 8](06-light-field-focus-fit-checks.md#check-8-practical-fit) |

### Table 2: status by check

One letter per general check, in order 1 to 8. Checks 5 and 6 are always `i` because they are comparisons, not judgments.

### Table 3: regime matrix

Each cell is **telescope/camera/mount/system** for that regime. The last row shows the trade a small global-shutter camera makes: it fixes the camera for LEO, but its small field drops the telescope to WARN for LEO acquisition. The system column is `W` everywhere because the demo enters no QE, throughput, sky brightness or read noise ([page 10](10-target-brightness-and-detection.md#step-6-the-verdict)).

## Reading the results by mission

Decide which metric matters most for the role before comparing:

| Role | Primary metrics |
|---|---|
| LEO tracking | Regime checks: mount rate and keyhole, global shutter, GPS timing, acquisition field |
| Wide-area search (for example the GEO belt) | Search speed (etendue), then sampling |
| Precision astrometry and tracking | Sampling, optics vs seeing, timing |
| Characterization and photometry | Depth, focus stability, back focus for filters |
| Field portability | Weight, power, setup effort |

In the demo, the RASA 11 is the fastest searcher, the DeltaRho 350 balances depth, sampling and search speed, and the CDK telescopes are deep but narrow and badly oversampled with modern small-pixel CMOS sensors. That makes the CDKs better suited to characterization than to search.

## Check it yourself

1. **Recompute a row.** Take the RASA 11 + IMX174 row. The IMX174 has 1936 x 1216 pixels of 5.86 um. Plate scale = 206.265 x 5.86 / 620 = **1.95 "/px**. Pixels across = 2.5 / 1.95 = **1.3** (WARN, slightly undersampled). Sensor = 11.3 x 7.1 mm, so the field is 11.3 / 620 x 57.3 = 1.05 deg by 7.1 / 620 x 57.3 = 0.66 deg = **0.69 deg^2**. Search = 0.0509 x 0.69 / 0.170 = **0.21x**.
2. **Why the IMX174 drops LEO acquisition to WARN.** Half the short side = 0.66 / 2 x 3600 = 1,185". Needed = 825 + 30 = 855". Margin = 1,185 / 855 = **1.4x**, between 1 and 2, so WARN.
3. **Why its camera passes LEO.** A global shutter has no skew, which removes the FAIL the IMX455 gets for LEO.
4. **Run it.** `cargo run --release -- --demo` should print the tables above exactly, apart from trailing spaces.

Next: the reference pages, starting with [Thresholds](13-thresholds.md).
