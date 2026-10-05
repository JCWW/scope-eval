# 17. The point spread function budget

**Reference page.** How the tool sizes a star image: every blur between the star and the recorded pixels, which checks use the result, and what the model leaves out.

## Why a star is bigger than the seeing

A point of light reaches the sensor as a blur called the **point spread function (PSF)**. The atmosphere is the largest part of it, which is why the early pages treat a star as exactly the seeing FWHM. Four more things add to it:

| Term | Where it comes from | Input |
|---|---|---|
| Seeing | Turbulence in the atmosphere | Site seeing, arcsec FWHM |
| Diffraction | The aperture's finite size | Aperture and site wavelength |
| Optics | The telescope's aberrations across the field | Vendor RMS spot sizes |
| Detector diffusion | Charge spreading into neighbouring pixels before it is collected | Camera MTF at Nyquist (optional) |
| Pixel aperture | Each pixel adds up all the light that lands on its square | Pixel pitch |

The tool treats each term as a round Gaussian. Independent Gaussian blurs convolve into another Gaussian whose variance is the sum of theirs, so the FWHMs add **in quadrature**:

```
total = sqrt(a^2 + b^2 + c^2 + ...)
```

That is the same rule check 4 uses for optics against seeing, extended to every term. The budget is kept term by term, so the report can show which one dominates.

## The terms

**Diffraction.** The core of the Airy pattern of a circular aperture has

```
FWHM = 1.029 x wavelength / D     (radians; x 206,265 for arcsec)
```

For the DeltaRho 350 at 0.55 um: 1.029 x 0.55e-6 / 0.35 = 1.62e-6 rad = **0.33"**. That is small against 2.5" seeing, and smaller still for bigger apertures (0.12" at 1 m). It matters on nights of very good seeing.

**Optics.** The vendor's RMS spot size at the sensor centre and corner, converted to a FWHM as in check 4 (1.665 x RMS radius, or 0.833 x RMS diameter) and then to arcseconds with the focal length. Spec sheets rarely say which convention they use. When the preset or your entry says "not stated", the budget reads the figure as an **RMS radius**. That's what Zemax spot diagrams report, and it is the larger blur, so the tool doesn't flatter the telescope. Check 2 also prints the star size under the diameter reading so you can see what hangs on it. Without spot data, the optics term is left out and the report says so.

**Detector diffusion.** Charge generated deep in the silicon wanders sideways before a pixel collects it, and some leaks across pixel boundaries. Camera makers rarely publish this. It is measured as the detector's **MTF (modulation transfer function) at the Nyquist frequency**, half a cycle per pixel. A perfect square pixel already has an MTF of sinc(1/2) = 2/pi = 0.637 there, so the tool treats a measured value below 0.637 as a Gaussian diffusion kernel:

```
MTF_measured = (2/pi) x exp(-2 pi^2 sigma^2 f^2),   f = 1 / (2 x pixel)
sigma        = (pixel / pi) x sqrt(-2 ln(MTF_measured / 0.637))
FWHM         = 2.355 x sigma
```

For example, an IMX455 measured at MTF 0.50 would have sigma = (3.76 / pi) x sqrt(-2 ln 0.785) = 0.83 um, a FWHM of 1.96 um. A value of 0.637 or more means no diffusion. Enter it as `mtf_nyquist` in `presets.yaml` or at the custom-camera prompt. With nothing entered, the term is left out and the report says so.

**Pixel aperture.** A pixel integrates over its square, so a recorded star is the image convolved with a box one pixel wide. A box of width p has the same RMS width as a Gaussian with sigma = p / sqrt(12), so

```
pixel FWHM = 2.355 x 0.2887 x p = 0.680 x p
```

For the IMX455 on the DeltaRho 350 that is 0.68 x 0.739" = **0.50"**.

## Three sizes of the same star

The report gives the star at three stages, because each answers a different question:

| Size | Terms | Used for |
|---|---|---|
| Focal-plane image | seeing, diffraction, optics | The telescope's delivered image |
| Sampled image | + detector diffusion | Check 2 (sampling), check 3 (ideal pixel), light in the brightest pixel |
| Recorded star | + pixel aperture | Detection footprint, trail-limited exposure, crossing time, centroid precision, the comparison table's `FWHM"` column |

Sampling uses the image *before* the pixel aperture, because the question is how finely the pixels sample the light that falls on them. Detection uses the recorded star, because it counts the pixels the star's light actually ends up in. It is also the size you would measure on a real frame.

## Worked example: DeltaRho 350 + IMX455

At 2.5" seeing and 0.55 um, with the 4.9 um RMS spot read as a radius and no MTF entered:

```
seeing          2.50"
diffraction     0.33"
optics          1.665 x 4.9 = 8.16 um at 1050 mm = 1.60"   (2.00" at the 21.6 mm corner)
sampled image   sqrt(2.50^2 + 0.33^2 + 1.60^2) = 2.99"     (4.05 px at 0.739"/px)
pixel aperture  0.50"
recorded star   sqrt(2.99^2 + 0.50^2) = 3.03"              (4.10 px)
```

The star is 20% wider than the seeing alone. As a result, check 2 moves from 3.38 to 4.05 pixels across, and from PASS to WARN. The detection footprint grows from 11.5 to 16.8 pixels. That costs about 0.2 mag of limiting magnitude in the stationary regimes (20.06 to 19.87). Read as an RMS diameter, the spot would give a 2.65" sampled star and 3.58 pixels across, back to PASS.

## Light in the brightest pixel

For a Gaussian image of sigma s pixels whose centre sits d pixels from a pixel's centre, the fraction of the light in that pixel along one axis is

```
share(d) = [erf((0.5 - d) / (sqrt 2 x s)) - erf((-0.5 - d) / (sqrt 2 x s))] / 2
```

and the two axes multiply. The report gives the best case (star centred on a pixel, d = 0) and the worst (on a corner shared by four pixels, d = 0.5). In the worked example both are about 5%, because a 4-pixel star is spread evenly whatever its position. On an undersampled pairing such as the RASA 11 with an IMX174 (1.3 pixels across) the two cases differ much more: 40% centred against 22% on a corner. That difference is what makes undersampled photometry and saturation depend on where the star lands.

## Centroid precision

Each detection check prints the photon-limited centroid precision per axis:

```
sigma_centroid = recorded FWHM / 2.355 / SNR
```

At GEO the running example gives 3.03 / 2.355 / 526 = **2.4 milliarcseconds**. This is a lower bound. Sky noise, read noise and coarse pixels all make the real figure larger.

## What the model leaves out

* **Gaussian shapes.** Real seeing has broader wings than a Gaussian (a Moffat profile fits better), and a diffraction pattern has rings. Both put light outside the core, so the model slightly overstates the light in the brightest pixel and understates the footprint a photometric aperture needs.
* **The central obstruction's rings.** An obstruction narrows the Airy core slightly but moves light into the rings. The diffraction term uses the unobstructed core. That makes little difference while seeing dominates, but the DeltaRho tubes' 56 to 63% obstructions move a noticeable share of light out of the core.
* **One wavelength.** Diffraction and diffusion both depend on wavelength. The tool uses the site's single reference wavelength.
* **Spot shape.** A real off-axis spot is rarely round. The tool uses the RMS figure as if it were.
* **Tracking jitter and wind shake.** Not included. If you know them, add them to the seeing in quadrature before entering it.

A fuller model would build the star on a fine 2-D grid from a Moffat seeing profile, a computed diffraction pattern for the actual obstructed aperture and the vendor's encircled-energy data, then integrate it over pixels at random positions. Measured PSFs from real frames are the best check on either.

## Check it yourself

**By hand.** Reproduce the worked example above: the four terms, the sampled image, the recorded star, and pixels across. Then redo it with the spot read as a diameter (0.80" optics term) and confirm 3.58 pixels across.

**Against the tests.**

| Concept | Command |
|---|---|
| Diffraction, pixel aperture, diffusion from MTF | `cargo test calculations::psf` |
| The running example's budget and both spot readings | `cargo test running_example_budget` and `cargo test unknown_convention` |
| Detection footprint and centroid | `cargo test detection_footprint_is_the_recorded_star` and `cargo test detection_reports_centroid_precision` |

**Script.** `python3 docs/learning/check_examples.py` recomputes every number on this page independently (section `02-seeing-and-sampling.md`).

**In the tool.** Run `cargo run --release -- --demo` and read the "Point spread function budget" block for DeltaRho 350 + IMX455. It should show the terms above, a 2.99" / 3.22" sampled image (centre / corner) and a 3.03" recorded star. The comparison table's `FWHM"` column should read 3.03 for that row.
