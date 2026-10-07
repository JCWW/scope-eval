# Lesson 9: Brightness and detection

Can this configuration actually see the target? This lesson builds the tool's detection model from first principles: how bright a satellite is, how many electrons that puts in the camera, how much noise comes with them, and the faintest magnitude that can be detected.

**Prerequisites:** [Lesson 1](01-angles-and-magnitudes.md) (magnitudes, surface brightness), [Lesson 2](02-seeing-and-sampling.md) (SNR equation, read noise), [Lesson 4](04-light-collection-and-search.md) (effective area), [Lesson 7](07-timing-and-shutters.md) (trailing).

## You will be able to

* Estimate a satellite's apparent magnitude from its size, reflectivity and range.
* Convert a magnitude into electrons per second in a given camera on a given telescope.
* Do the same for the sky background, per pixel.
* Choose an exposure from the target's motion and compute the pixels its light lands in.
* Compute the SNR of a detection, and solve the SNR equation backwards for the limiting magnitude.

## Key terms

| Term | Meaning |
|---|---|
| **Apparent magnitude (m)** | How bright the target looks from the site. |
| **Albedo** | The fraction of incoming sunlight a surface reflects. |
| **Cross-section** | The area of the target facing the Sun and observer, m^2. |
| **Lambertian** | A perfectly diffuse (matte) reflector that looks equally bright from all directions. |
| **Phase** | The Sun–target–observer angle. "Full phase" (phase factor 1) means fully lit as seen from the site. |
| **Zero point** | The photon rate from a magnitude-0 source, per m^2 per second. |
| **Quantum efficiency (QE)** | The fraction of photons that hit the sensor and become electrons. |
| **Throughput** | The fraction of photons entering the telescope that reach the sensor (mirror and lens losses). |
| **Shot noise** | The random fluctuation in a count of photons, sqrt(N) for N photons (Poisson statistics). |
| **SNR** | Signal-to-noise ratio. 5 is a common detection threshold. |
| **Limiting magnitude** | The faintest magnitude that reaches the detection SNR. |

---

## 9.1 How bright is the target?

**Formula.** For a diffuse target lit by the Sun:

```
m = -26.74 - 2.5 x log10( albedo x area x phase / (pi x d^2) )
```

with area in m^2 and range `d` in meters.

**Read it piece by piece.**

* **-26.74** is the Sun's apparent magnitude. The target can only reflect sunlight, so its brightness is the Sun's times a (tiny) fraction.
* **albedo x area** is how much sunlight the target catches and sends back.
* **1 / (pi x d^2)** is how that reflected light spreads out. The `pi` is the Lambertian scattering factor; the `d^2` is the inverse-square law.
* **phase** dims a target that is partly in shadow as seen from the site. The tool assumes 1 (full phase), the brightest case.

**The representative target.** The tool assumes one object, 10 m^2 at 0.2 albedo, so the regimes differ only through range.

| Regime | Range | Derived magnitude |
|---|---|---|
| LEO | 500 km | 2.25 |
| MEO | 20,200 km | 10.28 |
| GEO | 37,000 km | 11.59 |
| HEO | 39,836 km | 11.75 |
| Cislunar | 384,400 km | 16.67 |

Work GEO yourself: 0.2 x 10 x 1 / (pi x (3.7e7)^2) = 4.65e-16. log10 = -15.33. -26.74 + 2.5 x 15.33 = 11.59.

**Sanity check against reality.** Real GEO objects run from magnitude 11 to 15, cislunar ones 16 to 20. The representative target lands at the bright end of each, which is consistent with assuming full phase. You can enter a target magnitude to override it.

## 9.2 From magnitude to electrons

**The zero point.** A magnitude-0 source delivers about **8.9e9 photons per m^2 per second** in the V (visual) band. To derive it:

```
V-band zero point     f_nu = 3.64e-23 W/m^2/Hz
convert to per meter  f_lambda = f_nu x c / lambda^2        (lambda = 550 nm)
over an 89 nm band    flux = f_lambda x 89e-9 m = 3.21e-9 W/m^2
energy of one photon  E = h c / lambda = 3.61e-19 J
photon rate           3.21e-9 / 3.61e-19 = 8.9e9 photons/m^2/s
```

**Signal.**

```
signal (e-/s) = 8.9e9 x 10^(-0.4 m) x effective area x QE x throughput
```

The `10^(-0.4 m)` is Lesson 1's magnitude-to-flux conversion: how much fainter than magnitude 0 the target is.

**Sky.** The sky is a surface brightness (mag/arcsec^2). One pixel sees `plate scale^2` square arcseconds of it:

```
sky (e-/px/s) = 8.9e9 x 10^(-0.4 x sky_mag) x effective area x QE x throughput x plate scale^2
```

**Defaults.** QE 0.80, throughput 0.85, sky 21.0 mag/arcsec^2, read noise 3 e-, unless you enter your own.

## 9.3 Exposure, trail and footprint

**What trails depends on tracking mode** (Lesson 6). If the mount holds the target still (rate track or stare), the target doesn't trail, and only the 30 s cap limits the exposure. If the mount tracks the stars (cislunar), the target drifts at its rate against the stars.

**Trail-limited exposure.** Keep the target's trail within one star width. The star is the recorded star from Lesson 2, section 2.7: the seeing plus diffraction, optics, detector diffusion and the pixel's own shape, added in quadrature. For the running example it is **3.03"**, not the 2.5" seeing.

```
exposure        = star / residual rate        (capped at 30 s)
trail           = residual rate x exposure
footprint (px)  = (star / scale) x ((star + trail) / scale)
```

The footprint is a star image stretched along the trail: one star width across, star plus trail along. Note that the detection model uses the **native** plate scale, not the binned one, because read noise is paid per native pixel on CMOS (Lesson 2).

**Worked example (cislunar, 2.5" seeing, 3.03" star).**

```
exposure  = 3.03 / 0.549 = 5.519 s
trail     = 0.549 x 5.519 = 3.03"           (one star width, by construction)
footprint = (3.03 / 0.7386) x (6.06 / 0.7386) = 33.7 px
```

For the stationary regimes: exposure 30 s, trail 0, footprint (3.03 / 0.7386)^2 = 16.8 px. With the seeing alone as the star, these would be 4.554 s, 22.9 px and 11.5 px: the bigger star costs some sky and read noise.

## 9.4 Signal-to-noise

```
SNR = S / sqrt(S + B + R^2 x n)

S = signal electrons in the exposure
B = sky electrons in the footprint = sky rate x exposure x n
R = read noise per pixel
n = footprint in pixels
```

This is Lesson 2's SNR equation with the sky term written as a total. Each term is a variance: the target's own shot noise (S), the sky's shot noise (B) and the read noise of every pixel read (R^2 x n). Independent variances add.

## 9.5 Limiting magnitude

**Idea.** Set the SNR to a threshold `T` (5) and solve for the signal `S` that just reaches it. Write `N = B + R^2 x n` for the noise that doesn't depend on the target:

```
T = S / sqrt(S + N)
T^2 (S + N) = S^2
S^2 - T^2 S - T^2 N = 0
```

A quadratic in S, with one positive root:

```
S_min = (T^2 + sqrt(T^4 + 4 T^2 N)) / 2
```

Then turn electrons back into a magnitude, dividing by the electrons a magnitude-0 source would give in the same exposure:

```
K       = 8.9e9 x area x QE x throughput x exposure
m_limit = -2.5 x log10(S_min / K)
```

No search or iteration is needed.

## 9.6 Putting it together

For the DeltaRho 350 (0.0660 m^2, 0.7386 "/px) with an IMX455 at 2.5" seeing, 21.0 mag/arcsec^2 sky, QE 0.80, throughput 0.85 and 3 e- read noise:

| Regime | Mode | Exposure | Target mag | SNR | Limiting mag | Margin | Verdict |
|---|---|---|---|---|---|---|---|
| LEO | rate-track | 30 s (capped) | 2.25 | ~38,900 | 19.87 | +17.6 | trivial |
| MEO | rate-track | 30 s (capped) | 10.28 | 964 | 19.87 | +9.6 | trivial |
| GEO | stare | 30 s (capped) | 11.59 | 526 | 19.87 | +8.3 | trivial |
| HEO | rate-track | 30 s (capped) | 11.75 | 488 | 19.87 | +8.1 | trivial |
| Cislunar | sidereal | 5.519 s (trail-limited) | 16.67 | 15.4 | 18.15 | +1.5 | graded |

**Two things worth reading twice.**

1. The limiting magnitude is **identical at 19.87 for all four stationary-target regimes**. They share an exposure (the cap), a zero trail and therefore the same footprint and noise. Only the target's brightness differs.
2. Four of five regimes have SNR above 100, where the tool says "detection is not the limiting factor" rather than grading it. A 14-inch telescope at 30 s genuinely doesn't struggle with anything nearer than the Moon. Cislunar is the only regime where detection is close.

**Grading.** PASS at SNR >= 10, WARN at >= 5, FAIL below. If QE, throughput, sky or read noise were assumed rather than entered, the check is capped at WARN and lists what it assumed. If an entered exposure trails the target off the sensor, the check won't PASS either.

**Too bright is a problem too.** A LEO target at SNR 38,900 floods its brightest pixel many times over in a 30 s exposure. That's why "trivial" means "choose your exposure for timing and saturation instead", and the separate saturation check reports the longest exposure that stays linear ([step 7 of page 10](../10-target-brightness-and-detection.md#step-7-saturation)).

**Centroid precision.** The check also prints how precisely the target's position can be measured: sigma = star FWHM / 2.355 / SNR per axis. At GEO that is 3.03 / 2.355 / 526 = 2.4 milliarcseconds. It is the photon-limited best case; sky, read noise and coarse pixels all make the real figure worse.

---

## Validate it yourself

* **By hand.** Reproduce the GEO row: signal rate, sky rate per pixel, S and B at 30 s, SNR and limiting magnitude. Keep 4 significant figures. Intermediate values for 30 s: S = 2.769e5 e-, sky = 26.0 e- per pixel, footprint 16.83 px, so B = 438 e- and R^2 n = 151 e-^2.
* **Round trip.** Put the limiting magnitude back into the SNR equation as the target magnitude. You should get exactly SNR = 5. The test `limiting_mag_round_trip` does this.
* **Script.** `python3 docs/learning/check_examples.py` (section `09-brightness-and-detection.md`) recomputes the derived magnitudes, the zero point, the cislunar exposure and footprint, both SNRs and both limiting magnitudes.
* **Unit tests.**

  ```bash
  cargo test derived_mag_matches_worked_examples
  cargo test signal_coefficient_is_the_magnitude_zero_signal
  cargo test geo_snr_deltarho350
  cargo test cislunar_snr_deltarho350
  cargo test cislunar_limiting_mag
  cargo test limiting_mag_round_trip
  cargo test stationary_regimes_share_a_limiting_magnitude
  ```

* **Tool output.** Enter QE, throughput, sky and read noise in a custom configuration and read the System row of the detailed regime breakdown. It prints signal, sky, SNR, limiting magnitude and margin.

## Self-check

1. Halve the target's cross-section. How much fainter is it?
2. Why is the cislunar exposure 5.519 s and not 30 s?
3. A darker sky (22 instead of 21 mag/arcsec^2) reduces the sky rate by what factor?
4. Why does the limiting-magnitude formula need a quadratic, rather than just `S = T x sqrt(N)`?
5. At what SNR does the tool stop grading detection and why?

<details>
<summary>Answers</summary>

1. 2.5 x log10(2) = 0.75 mag fainter.
2. The mount tracks the stars, and the target drifts against them at 0.549"/s. After 5.519 s it has moved one star width (3.03"); longer and it streaks, spreading its light over more pixels and more read noise.
3. 10^0.4 = 2.512 times fewer sky electrons.
4. The target's own shot noise is part of the noise, so S appears on both sides. `T x sqrt(N)` is the faint-target limit where S is much smaller than N.
5. At SNR >= 100 (`SNR_TRIVIAL`). Beyond that, detection is certain and what limits the regime is something else: tracking, timing, acquisition or saturation.

</details>

## Where it lives in the code

* `src/calculations/detection.rs`: `derived_target_mag`, `signal_e_per_s`, `sky_e_per_px_s`, `signal_coefficient`, `trail_limited_exposure_s`, `footprint_px`, `snr`, `limiting_mag`.
* `src/photometry.rs`: `Photometry::resolve` decides which inputs were entered and which were assumed.
* `src/regimes.rs`: `residual_rate_arcsec_s`, `system_detection`.
* `src/constants.rs`: `SUN_APPARENT_MAG`, `PHOTONS_M2_S_MAG0`, `REFERENCE_TARGET_*`, `DEFAULT_*`, `MAX_EXPOSURE_S`; module `regimes_limits`: `DETECT_SNR_THRESHOLD`, `SNR_PASS`, `SNR_TRIVIAL`.

**Next:** [Lesson 10: Pass prediction](10-pass-prediction.md)
