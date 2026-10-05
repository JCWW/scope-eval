//! The formula summary shown from the interactive menu.

/// Every formula the checks use, one line each. Full explanations are in
/// docs/learning/.
pub const FORMULAS: &str = r#"
Formula summary (full explanations in docs/README.md)

 1 Sensor fit        image circle >= sensor diagonal = sqrt(w^2 + h^2)
 2 Plate scale       scale ("/px) = 206.265 x pixel (um) / focal length (mm)
   Sampling          pixels across a star = star FWHM (") / scale   (target ~2)
 3 Ideal pixel       pixel (um) = (star FWHM / 2) x focal length (mm) / 206.265
 4 Seeing blur       blur (um) = seeing (") x focal length (mm) / 206.265
   Optics FWHM       ~1.665 x RMS radius  (or ~0.833 x RMS diameter)
   Star growth       sqrt(seeing^2 + optics^2) / seeing - 1

Point spread function (every term a Gaussian FWHM, added in quadrature)
   Diffraction       1.029 x wavelength / D  (x 206265 for arcsec)
   Diffusion         sigma = (pixel / pi) x sqrt(-2 ln(MTF_Nyquist / 0.6366))
   Pixel aperture    0.680 x pixel  (a box of width p has sigma p / sqrt(12))
   Star FWHM         sqrt(seeing^2 + diffraction^2 + optics^2 + diffusion^2)
   Recorded star     sqrt(star FWHM^2 + pixel aperture^2)
   Peak pixel        [erf((0.5 - d) / (sqrt 2 sigma)) - erf((-0.5 - d) / (sqrt 2 sigma))]^2 / 4
   Centroid          sigma = recorded FWHM / 2.355 / SNR  (photon-limited)
 5 Effective area    pi/4 x D^2 x (1 - obstruction_by_diameter^2)
   Depth             dMag = 2.5 x log10(area / reference area)
 6 Field of view     FOV (deg) = sensor (mm) / focal length (mm) x 57.3
   Search speed      etendue = effective area x field area
 7 Focus tolerance   CFZ = +/- 2.44 x wavelength x N^2,  N = FL / D
 8 Payload           (OTA + camera + accessories) / mount rating  (keep <= 70%)
   GEO timing        star drift = 15.04"/s,  rolling skew = rows x line time

Orbital regimes (component checks per regime)
   LEO/MEO rate      omega ~ v / h,  v = sqrt(mu / (R + h))   (overhead pass)
   GEO/lunar rate    rate vs stars = 1,296,000" / orbital period
   Acquisition       margin = (short side of field / 2) / (ephemeris_km / range_km x 206265 + pointing)
   Timing needed     dt = 0.25 x binned scale / rate vs stars
   Shutter skew      skew = rate vs stars x rows x line time
   Trailing          t = recorded star FWHM / rate vs stars
   Mount headroom    max axis rate / rate vs ground   (keep >= 3x)
   Alt-az keyhole    highest followable pass = 90 deg - (omega / max azimuth rate) in degrees
   Peak accel        alpha = 0.6495 x omega^2        (0.6495 = 3 sqrt(3) / 8)
   Accel keyhole     z >= omega x sqrt(0.6495 / max accel)
   Keyhole reported  the larger of the rate and acceleration limits
   Slew time         t = v/a + D/v,  or 2 sqrt(D/a) if D < v^2/a
   Target magnitude  m = -26.74 - 2.5 x log10(albedo x area x phase / (pi x d^2))
   Signal            e-/s = 8.9e9 x 10^(-0.4 m) x area x QE x throughput
   Sky               e-/px/s = same, at the sky magnitude, x plate scale^2
   Trail-limited t   exposure = recorded star FWHM / residual rate   (capped at 30 s)
   Footprint         (star / scale) x ((star + trail) / scale),  star = recorded FWHM
   SNR               S / sqrt(S + B + R^2 x n)
   Limiting mag      invert SNR = 5:  S = (T^2 + sqrt(T^4 + 4 T^2 N)) / 2
"#;
