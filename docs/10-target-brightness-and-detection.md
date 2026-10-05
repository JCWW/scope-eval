# 10. Target brightness and detection

**What you'll learn:** how bright a satellite appears from its size, reflectivity and range, how many photons that delivers to the sensor, how long an exposure can be, and how signal-to-noise turns into a limiting magnitude and a detection verdict. This is the **System** component in the regime tables.

**Before you start:** [Key terms](02-key-terms.md#brightness) (magnitudes), [check 2](05-image-quality-checks.md#why-you-cant-bin-your-way-out-of-oversampling-on-cmos) (the SNR equation and read noise), [check 5](06-light-field-focus-fit-checks.md#check-5-collecting-area-and-depth) (effective area) and [exposure vs trailing](08-orbital-regimes.md#exposure-vs-trailing-info).

The page runs as one chain: **magnitude -> photons -> exposure -> SNR -> limiting magnitude -> verdict**.

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

## Check it yourself

**By hand.** Use the DeltaRho 350 values above.

1. **GEO magnitude.** d = 3.7e7 m. albedo x area x phase / (pi x d^2) = 2 / (pi x 1.369e15) = 4.65e-16. log10 = -15.33. m = -26.74 + 2.5 x 15.33 = **11.59**.
2. **Range scaling.** Cislunar is 384,400 / 37,000 = 10.4 times further than GEO. 5 x log10(10.4) = 5.08 mag, and 11.59 + 5.08 = **16.67**.
3. **Photon constant.** 3.21e-9 / 3.61e-19 = **8.9e9** photons per m^2 per s.
4. **Sky per pixel.** 8.9e9 x 10^(-0.4 x 21) x 0.0660 x 0.80 x 0.85 x 0.7386^2 = **0.87 e- per pixel per second**.
5. **GEO SNR.** Footprint = (3.03 / 0.7386)^2 = 16.83 px. S = 8.9e9 x 10^(-0.4 x 11.59) x 0.0660 x 0.80 x 0.85 x 30 = 277,000 e-. B = 0.867 x 30 x 16.83 = 438 e-. Read term = 9 x 16.83 = 151. SNR = 277,000 / sqrt(277,000 + 438 + 151) = **526**. The bigger star barely matters here: the target's own shot noise dominates.
6. **Cislunar limiting magnitude.** With the footprint of 33.66 px and exposure 5.519 s: B = 0.867 x 5.519 x 33.66 = 161.1, N = 161.1 + 9 x 33.66 = 464.0. S_min = (25 + sqrt(625 + 100 x 464.0)) / 2 = 120.9 e-. K = 8.9e9 x 0.0660 x 0.80 x 0.85 x 5.519 = 2.204e9. m_limit = -2.5 x log10(120.9 / 2.204e9) = **18.15**.

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
| The WARN cap on assumed inputs | `cargo test detection_caps_at_warn` and `cargo test detection_passes_when_every_input_is_entered` |

**In the tool.** Run `cargo run --release -- --demo` and read the System: Detection check in each regime block for DeltaRho 350 + IMX455. LEO through HEO should show a footprint of 16.8 px and a limiting magnitude of 19.87; cislunar should show an exposure of 5.519 s, a footprint of 33.7 px, SNR 15.4 and limiting magnitude 18.15, capped at WARN with the assumed inputs listed. The test `detection_passes_when_every_input_is_entered` shows the other side: with every input entered, the same configuration's GEO detection grades PASS and names nothing as assumed.

Next: [Pass prediction](11-pass-prediction.md).
