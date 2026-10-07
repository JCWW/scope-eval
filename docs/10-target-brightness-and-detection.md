# 10. Target brightness and detection

**What you'll learn:** how bright a satellite appears from its size, reflectivity and range, how many photons that delivers to the sensor, how long an exposure can be, and how signal-to-noise turns into a limiting magnitude and a detection verdict, and how bright a target can get before it saturates the sensor. This is the **System** component in the regime tables.

**Before you start:** [Key terms](02-key-terms.md#brightness) (magnitudes), [check 2](05-image-quality-checks.md#why-you-cant-bin-your-way-out-of-oversampling-on-cmos) (the SNR equation and read noise), [check 5](06-light-field-focus-fit-checks.md#check-5-collecting-area-and-depth) (effective area) and [exposure vs trailing](08-orbital-regimes.md#exposure-vs-trailing-info).

The page runs as one chain: **magnitude -> photons -> exposure -> SNR -> limiting magnitude -> verdict**, then turns round in step 7 to ask the opposite question: is the target too bright?

---

## Step 1: how bright the target is

For a diffuse (Lambertian) target, the apparent magnitude follows from its cross-section, albedo, range and phase, measured relative to the Sun's apparent magnitude of -26.74. The `1/pi` is the Lambertian scattering factor:

```
m = -26.74 - 2.5 x log10(albedo x area x phase / (pi x d^2))
```

with the area in square metres and the range d in metres. The tool assumes one representative target, a 10 m^2 object at 0.2 albedo at full phase, so magnitudes differ between regimes only through range: the same object, moved further away. Because brightness falls as 1/d^2, four times the range is 2.5 x log10(16) = 3 magnitudes fainter.

| Regime | Range | Derived magnitude |
|---|---|---|
| LEO | 500 km | 2.25 |
| MEO | 20,200 km | 10.28 |
| GEO | 37,000 km | 11.59 |
| HEO | 39,836 km | 11.75 |
| Cislunar | 384,400 km | 16.67 |

The sanity check on the absolute scale is that these land where real objects do: GEO objects run 11 to 15, cislunar 16 to 20. You can override the magnitude with your own figure.

---

## Step 2: photons from the target and from the sky

A magnitude-zero source delivers about `8.9e9` photons per square metre per second in V band. That comes from the V-band zero point 3.64e-23 W/m^2/Hz over a 550 nm band 89 nm wide, giving 3.21e-9 W/m^2, divided by the 3.61e-19 J energy of a 550 nm photon. So:

```
signal (e-/s)  = 8.9e9 x 10^(-0.4 m) x effective area x QE x throughput
sky (e-/px/s)  = the same, at the sky magnitude, x plate scale^2
```

The `10^(-0.4 m)` term is the magnitude definition run backwards: each magnitude divides the photon rate by 2.512. QE (quantum efficiency) is the fraction of photons that become electrons, and throughput is the fraction that survive the optics. The sky term is the point-source rate for that surface brightness scaled by the solid angle one pixel covers.

---

## Step 3: choosing the exposure

Exposure is not a separate input. It follows from how the target moves relative to the mount (its **residual rate**):

* A target the mount holds still (rate-tracked LEO, MEO and HEO, or GEO in stare mode) does not trail, so nothing bounds the exposure but a 30 s cap.
* A target tracked sidereally (cislunar) drifts at its rate against the stars, and the natural exposure is the one that keeps its trail inside a single star width.

The star width is the recorded star FWHM from the [point spread function budget](17-point-spread-function.md): seeing, diffraction, optics, detector diffusion and the pixel aperture, added in quadrature. For the DeltaRho 350 + IMX455 at 2.5" seeing it is **3.03"**.

```
exposure       = star / residual rate     (capped at MAX_EXPOSURE_S = 30 s)
trail          = residual rate x exposure
footprint (px) = (star / scale) x ((star + trail) / scale)
```

**Worked example (cislunar, 3.03" star, native 0.7386 "/px).** The Moon's rate against the stars is 0.549 "/s, so the exposure is 3.03 / 0.549 = **5.519 s**, the trail is 3.03" by construction, and the footprint is 3.03 x 6.06 / 0.7386^2 = **33.66 px**. With the 2.5" seeing alone as the star, these would be 4.554 s and 22.91 px.

You can override the exposure. If your choice trails the target off the sensor, the tool says so and will not report a PASS.

---

## Step 4: signal-to-noise

```
SNR = S / sqrt(S + B + R^2 x n)

S = target electrons in the exposure
B = sky electrons in the footprint
R = read noise per pixel, n = footprint in pixels
```

The signal appears inside the noise term because photon arrival is Poisson: its own shot noise is `sqrt(S)`.

---

## Step 5: limiting magnitude

Setting `SNR = T` (the detection threshold, 5) and solving for the signal gives a quadratic with one positive root, so the faintest detectable magnitude needs no search:

```
S_min   = (T^2 + sqrt(T^4 + 4 T^2 N)) / 2,    N = B + R^2 x n
m_limit = -2.5 x log10(S_min / K),            K = 8.9e9 x area x QE x throughput x exposure
```

K is the signal a magnitude-zero target would give in this exposure, so `S_min / K` is a brightness ratio and the log turns it back into a magnitude.

---

## Step 6: the verdict

For the DeltaRho 350 (0.0660 m^2, 0.7386 "/px) with an IMX455 at 2.5" seeing, 21.0 mag/arcsec^2 sky, QE 0.80, throughput 0.85 and 3 e- read noise:

| Regime | Mode | Exposure | Target mag | SNR | Limiting mag | Margin | Verdict |
|---|---|---|---|---|---|---|---|
| LEO | rate-track | 30 s (capped) | 2.25 | ~38,900 | 19.87 | +17.6 | trivial |
| MEO | rate-track | 30 s (capped) | 10.28 | 964 | 19.87 | +9.6 | trivial |
| GEO | stare | 30 s (capped) | 11.59 | 526 | 19.87 | +8.3 | trivial |
| HEO | rate-track | 30 s (capped) | 11.75 | 488 | 19.87 | +8.1 | trivial |
| Cislunar | sidereal | 5.519 s (trail-limited) | 16.67 | 15.4 | 18.15 | +1.5 | graded |

Two things in that table are worth reading twice.

The limiting magnitude is **identical at 19.87 for all four stationary-target regimes**. That is not a coincidence: they share an exposure (the 30 s cap), a zero trail, and therefore the same footprint and noise budget. They differ only in how bright the target is.

And four of the five regimes sit above `SNR_TRIVIAL` (100), so the tool reports "detection is not the limiting factor" instead of a graded margin. That is the right answer rather than a mis-set threshold: a 14-inch aperture at 30 seconds genuinely does not struggle with anything nearer than the Moon. Cislunar is the only regime where detection is close.

**Centroid precision.** Each detection check also prints the photon-limited centroid precision, sigma = star FWHM / 2.355 / SNR per axis: 2.4 milliarcseconds at GEO, 83 at cislunar. Sky, read noise and coarse pixels all make the real figure larger, so read it as a best case.

**Grading.** PASS at SNR 10 or more (`SNR_PASS`), WARN at 5 or more (`DETECT_SNR_THRESHOLD`) and FAIL below. At `SNR_TRIVIAL` or above the status is still PASS, but the verdict says detection is not what limits the regime and names what does. A PASS also drops to WARN if the exposure trails the target off the sensor. Where QE, throughput, sky brightness or read noise come from generic defaults rather than entered values, the check is capped at WARN and names what it assumed. It will not tell you a configuration will detect something on the strength of a quantum efficiency it invented.

---

## Step 7: saturation

Detection asks whether a target is bright enough. Saturation asks whether it is too bright. A pixel holds only so much charge, its **full well** (tens of thousands of electrons on modern CMOS, depending on the gain mode). Well before it fills, the response stops being linear, and a star whose brightest pixel is clipped or bent has a biased centroid and a wrong brightness. That matters for metric tracking, where the centroid is the product.

The check follows the light into the single brightest pixel:

```
peak_e = (S_rate x f_peak + B_px) x t
```

* `S_rate` is the target signal in e-/s (step 2) and `B_px` the sky per pixel per second.
* `f_peak` is the fraction of the star's light in its brightest pixel, with the star centred on a pixel (the brightest case). It comes from the system point spread function before the pixel aperture, integrated over one pixel ([page 17](17-point-spread-function.md#light-in-the-brightest-pixel)). For a target that trails during the exposure, the light along the trail is spread over its length L, which lowers the along-trail share from the untrailed value towards 1/L.

The check compares `peak_e` with 80% of the full well (`SATURATION_WARN_FRACTION`, the usual onset of nonlinearity) and reports three numbers:

* **Longest linear exposure**, the exposure at which `peak_e` reaches 80% of the well. For a held target this is `0.8 x full_well / (S_rate x f_peak + B_px)`; for a trailed one it is found numerically, and a fast-trailing target may never get there, because it never dwells on one pixel long enough.
* **Brightest linear magnitude** at the exposure in use, the bright end of the linear range: `m_bright = -2.5 log10((0.8 x full_well - B_px x t) / (K x f_peak))`, with K from step 5.
* **Saturates at**, the exposure that fills the well completely.

With the worked example's inputs and a 50,000 e- full well entered:

| Regime | Exposure | Target mag | f_peak | Peak pixel | Longest linear exposure | Brightest linear mag | Status |
|---|---|---|---|---|---|---|---|
| LEO | 30 s | 2.25 | 5.24% | 1,590 times full well | 15.1 ms | 10.49 | WARN |
| MEO | 30 s | 10.28 | 5.24% | 97% | 24.6 s | 10.49 | WARN |
| GEO | 30 s | 11.59 | 5.24% | 29% | over 30 s | 10.49 | PASS |
| HEO | 30 s | 11.75 | 5.24% | 25% | over 30 s | 10.49 | PASS |
| Cislunar | 5.519 s | 16.67 | 4.25% | under 1% | over 30 s | 8.42 | PASS |

This is the other half of step 6's "detection is not the limiting factor". The 30 s exposure that buries a LEO target in signal floods its brightest pixel 1,590 times over, so the exposure for LEO is set by saturation and timing, not detection: about 15 ms here. Brightness and detection bracket the useful range from both ends: 10.49 to 19.87 mag for the stationary regimes at 30 s, about nine magnitudes.

**Grading.** PASS when the peak pixel stays under 80% of the full well. Saturating at the tool's derived exposure is a WARN, because the exposure is a choice; the verdict gives the longest one that stays linear. Saturating at an exposure you entered is a FAIL. So is a target that saturates even in 1 ms (`MIN_PRACTICAL_EXPOSURE_S`), because then a shorter exposure is no longer a practical fix: it needs a neutral density filter, a smaller aperture or a deeper well. As with detection, a PASS is capped at WARN when QE, throughput, sky brightness or the full well were assumed (`DEFAULT_FULL_WELL_E`, 20,000 e-, deliberately at the low end so an unknown well flags saturation early). Read noise plays no part, so it is not named.

**What it leaves out.** Bias offset, dark current and the camera's ADC ceiling, which on some cameras clips before the well fills. Read the full well at the gain you will actually use.

---

## Check it yourself

**By hand.** Use the DeltaRho 350 values above.

1. **GEO magnitude.** d = 3.7e7 m. albedo x area x phase / (pi x d^2) = 2 / (pi x 1.369e15) = 4.65e-16. log10 = -15.33. m = -26.74 + 2.5 x 15.33 = **11.59**.
2. **Range scaling.** Cislunar is 384,400 / 37,000 = 10.4 times further than GEO. 5 x log10(10.4) = 5.08 mag, and 11.59 + 5.08 = **16.67**.
3. **Photon constant.** 3.21e-9 / 3.61e-19 = **8.9e9** photons per m^2 per s.
4. **Sky per pixel.** 8.9e9 x 10^(-0.4 x 21) x 0.0660 x 0.80 x 0.85 x 0.7386^2 = **0.87 e- per pixel per second**.
5. **GEO SNR.** Footprint = (3.03 / 0.7386)^2 = 16.83 px. S = 8.9e9 x 10^(-0.4 x 11.59) x 0.0660 x 0.80 x 0.85 x 30 = 277,000 e-. B = 0.867 x 30 x 16.83 = 438 e-. Read term = 9 x 16.83 = 151. SNR = 277,000 / sqrt(277,000 + 438 + 151) = **526**. The bigger star barely matters here: the target's own shot noise dominates.
6. **Cislunar limiting magnitude.** With the footprint of 33.66 px and exposure 5.519 s: B = 0.867 x 5.519 x 33.66 = 161.1, N = 161.1 + 9 x 33.66 = 464.0. S_min = (25 + sqrt(625 + 100 x 464.0)) / 2 = 120.9 e-. K = 8.9e9 x 0.0660 x 0.80 x 0.85 x 5.519 = 2.204e9. m_limit = -2.5 x log10(120.9 / 2.204e9) = **18.15**.
7. **GEO peak pixel.** S_rate = 277,000 / 30 = 9,222 e-/s. With 5.24% in the brightest pixel and 0.867 e-/s of sky: (9,222 x 0.0524 + 0.867) x 30 = **14,500 e-**, 29% of a 50,000 e- well.
8. **LEO longest linear exposure.** S_rate at 2.25 mag is 9,222 x 10^(0.4 x (11.59 - 2.25)) = 5.0e7 e-/s. 0.8 x 50,000 / (5.0e7 x 0.0524) = **15 ms**.

**Against the tests.**

| Concept | Command |
|---|---|
| Derived magnitudes and where they land | `cargo test derived_mag` and `cargo test target_magnitude_matches` |
| Four times the range is three magnitudes | `cargo test four_times_the_range` |
| Magnitude-zero signal and sky rates | `cargo test signal_coefficient` and `cargo test sky_rate_per_pixel` |
| Trail-limited exposure and footprint | `cargo test trail_limited_exposure` and `cargo test footprint` |
| GEO and cislunar SNR | `cargo test snr_deltarho350` |
| Limiting magnitude inverts the SNR equation | `cargo test limiting_mag` |
| Stationary regimes share a limit; only cislunar grades | `cargo test stationary_regimes_share` and `cargo test cislunar_is_the_only_regime` |
| Footprint and centroid use the system-PSF star | `cargo test detection_footprint_is_the_recorded_star` and `cargo test detection_reports_centroid_precision` |
| Trailed peak-pixel fraction | `cargo test trail` (in `calculations::psf`) |
| Saturation: peak pixel, grading and the fix | `cargo test saturation` and `cargo test a_deeper_well` |
| The WARN cap on assumed inputs | `cargo test detection_caps_at_warn` and `cargo test detection_passes_when_every_input_is_entered` |

**In the tool.** Run `cargo run --release -- --demo` and read the System: Detection check in each regime block for DeltaRho 350 + IMX455. LEO through HEO should show a footprint of 16.8 px and a limiting magnitude of 19.87; cislunar should show an exposure of 5.519 s, a footprint of 33.7 px, SNR 15.4 and limiting magnitude 18.15, capped at WARN with the assumed inputs listed. The test `detection_passes_when_every_input_is_entered` shows the other side: with every input entered, the same configuration's GEO detection grades PASS and names nothing as assumed.

Next: [Pass prediction](11-pass-prediction.md).
