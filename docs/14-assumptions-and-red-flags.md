# 14. Assumptions, limitations and spec-sheet red flags

**Reference page.** What the models leave out, so you know how far to trust each number, and the spec-sheet traps that most often produce wrong inputs.

## Assumptions and limitations

### Optics and the atmosphere

* **Seeing is a single number.** Real seeing varies by night, by elevation angle and through the night. Run the tool at your best, typical and worst seeing to see how sensitive a choice is.
* **Small-angle approximation.** Field of view uses size / focal length. The error is well under 0.1% for fields of a few degrees.
* **Obstruction ignores support vanes.** Spider vanes and cables in front of the aperture block a few more percent of the light and are not included.
* **Gaussian blur model.** Converting RMS spot to FWHM and adding blurs in quadrature both assume roughly Gaussian blurs. Real optical blur is often not Gaussian, so treat check 4 as an approximate screen, not a performance prediction.
* **Linear spot interpolation.** Spot size between and beyond quoted field points is estimated linearly.
* **Diffraction-based focus criterion.** The CFZ formula uses a standard diffraction criterion at 0.55 um. With seeing-limited images, practical tolerance can be somewhat looser, but fast systems remain demanding.
* **Footprint estimate.** "Pixels in a star's footprint" is approximated as (pixels across)^2. A photometric aperture is typically larger, but the ratio between configurations is what matters.
* **Gaussian point spread function.** Every blur (seeing, diffraction, optics, detector diffusion, pixel aperture) is treated as a Gaussian and added in quadrature. Real seeing has broader wings and diffraction has rings, so the brightest-pixel fraction is somewhat optimistic. An unlabelled RMS spot is read as a radius, the larger blur. See [page 17](17-point-spread-function.md#what-the-model-leaves-out).

### Motion and regimes

* **Sidereal drift at the equator.** GEO motion figures assume declination near zero.
* **Regimes are single representative cases.** Each regime is one geometry (for example a 500 km overhead LEO pass). Real targets span wide ranges of altitude, pass geometry and brightness. The overhead pass is deliberately the worst case for rates.
* **Earth rotation simplified.** LEO and MEO rates ignore Earth's rotation, and the HEO and cislunar ground rates are simple differences from the sidereal rate. Directions of motion are ignored.
* **Prediction errors are placeholders.** The along-track errors used for acquisition are assumptions, not catalog statistics.
* **Timing is a single figure.** The timestamp accuracy input lumps clock error, exposure-start latency and jitter together.

### Mount

* **Servo behaviour is not modeled.** The acceleration model covers peak axis acceleration, the acceleration-limited keyhole and slew timing. Servo bandwidth, closed-loop following error and path-following smoothness are not included, because vendors do not publish the inputs.
* **Slew distance is assumed.** The slew-and-settle check uses a 90-degree acquisition slew and, where the mount does not publish one, a 2-second settle.

### Brightness and detection

* **One representative target.** Derived magnitudes assume a 10 m^2 object at 0.2 albedo, so the figures vary between regimes only through range. Real objects span orders of magnitude in size and brightness. Enter a target magnitude to override it.
* **Full phase assumed.** The derived magnitude uses a phase factor of 1.0, the brightest case. A target near quadrature is roughly a magnitude fainter.
* **Sky brightness is a single number.** No dependence on elevation, moon phase or airmass, and no extinction term.
* **No saturation model.** Detector full-well depth is not modeled, so bright LEO targets report implausibly high SNR. The check reports these as "detection is not the limiting factor" rather than as a number to act on.
* **Photometric defaults are generic.** When QE, throughput, sky brightness or read noise are not entered, documented generic values are substituted and the detection check is capped at WARN. It will never report PASS on a quantum efficiency it assumed.
* **Hand-entered values are range-checked.** A photometric or dynamics value outside a plausible range (a QE above 1, a NaN, a sky brightness of 2.1 where 21.0 was meant) is treated as not entered rather than trusted, so a typo degrades the report instead of corrupting it.

### Pass prediction

* **Assessment grade.** Earth orientation uses mean sidereal time only, the Sun and Moon use low-precision formulas, and atmospheric refraction is ignored. Positions are good to about 0.01 degrees, well inside TLE error, but this is not astrometry. The upgrade path is listed in `crates/orbit-prop/README.md` under "Known limitations and future work".
* **Short grazing passes can be missed.** The pass search steps at one sixtieth of the orbital period, clamped to 10 to 300 seconds (about 93 seconds for the ISS), so a pass that stays above the minimum elevation for less than that may not be found.
* **What-if orbits drift.** They include J2's slow drift but no drag and no short-period terms, so they stand for a kind of orbit, not a specific satellite.

### Data

* **Preset data can age.** Specs and weights come from listings at the time of writing and may change. Always confirm against current vendor documentation before buying.

## Spec-sheet red flags

* **Obstruction quoted by area.** It looks much smaller than the by-diameter number, which flatters the telescope. A 49% obstruction by diameter is about 24% by area (0.49^2 = 0.24). Always check which one you're reading.
* **"Image circle" vs "fully corrected field."** These are not always the same, and sheets are not always consistent. One PlaneWave CDK14 product page lists both a 70 mm and a 52 mm image circle. Ask which is sharp at the edge.
* **RMS spot without radius or diameter stated, or without off-axis values.** A sharp center says little about the corners of a large sensor ([check 4](05-image-quality-checks.md#check-4-optical-quality-vs-seeing)).
* **OTA weight alone.** Camera, focuser, dew heaters and cables can add 10 to 20 lb against the mount's rating.
* **Back focus reference point.** "From the mounting surface" and "with focuser installed" can differ by tens of millimeters.
* **Camera pixel count.** Effective pixel counts differ slightly between camera vendors using the same sensor.

## Check it yourself

1. **Seeing sensitivity.** Evaluate the DeltaRho 350 + IMX455 at 1.5", 2.5" and 3.5" seeing (**Change site conditions**). Check 2 should move from about 3.0 to 4.0 to 5.2 pixels across (the optics' fixed blur keeps the good-seeing figure from falling further), and check 4's star growth should shrink as seeing worsens, because a larger seeing blur hides the optics.
2. **Small-angle error.** For the RASA 11's 3.33 deg field, compare 2 x atan(18.0 / 620) in degrees (3.327) with 36.0 / 620 x 57.2958 (3.327). The difference is under 0.1%.
3. **Obstruction red flag.** 0.49^2 = 0.24, so "49% by diameter" and "24% by area" describe the same telescope.
