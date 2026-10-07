//! Point spread function terms and what a sampled star looks like.
//!
//! Every blur is treated as a Gaussian, so independent terms add in
//! quadrature. That is a first-order model: real seeing has broader wings
//! (Moffat) and a diffraction pattern has rings. See
//! docs/17-point-spread-function.md for the derivations and the limits.

use crate::constants::{
    AIRY_FWHM_PER_LAMBDA_OVER_D, ARCSEC_PER_RADIAN, FWHM_PER_SIGMA, PIXEL_BOX_MTF_AT_NYQUIST,
    PIXEL_BOX_SIGMA_PER_PITCH, UM_PER_MM,
};
use std::f64::consts::{PI, SQRT_2};

/// Independent calculations for the terms of the system point spread function.
pub struct PsfCalculator;

impl PsfCalculator {
    /// FWHM of the diffraction core, arcsec: 1.029 x wavelength / aperture.
    pub fn diffraction_fwhm_arcsec(wavelength_um: f64, aperture_mm: f64) -> f64 {
        AIRY_FWHM_PER_LAMBDA_OVER_D * (wavelength_um / UM_PER_MM) / aperture_mm * ARCSEC_PER_RADIAN
    }

    /// Gaussian-equivalent FWHM of a square pixel's response, um.
    /// A box of width p has the same RMS width as a Gaussian of sigma p / sqrt(12),
    /// so FWHM = 2.355 x 0.2887 x p = 0.680 x p.
    pub fn pixel_aperture_fwhm_um(pixel_um: f64) -> f64 {
        FWHM_PER_SIGMA * PIXEL_BOX_SIGMA_PER_PITCH * pixel_um
    }

    /// FWHM of the charge-diffusion blur implied by a measured detector MTF at
    /// Nyquist, um.
    ///
    /// The measured MTF is the pixel aperture's 2/pi times the diffusion
    /// kernel's exp(-2 pi^2 sigma^2 f^2) at f = 1 / (2 p). Solving for sigma
    /// gives sigma = (p / pi) x sqrt(-2 ln(MTF / (2/pi))). An MTF at or above
    /// 2/pi leaves nothing for diffusion, so the result is zero.
    pub fn diffusion_fwhm_um(pixel_um: f64, mtf_nyquist: f64) -> f64 {
        let diffusion_mtf = mtf_nyquist / PIXEL_BOX_MTF_AT_NYQUIST;
        if diffusion_mtf >= 1.0 {
            return 0.0;
        }
        FWHM_PER_SIGMA * pixel_um / PI * (-2.0 * diffusion_mtf.ln()).sqrt()
    }

    /// Combine independent Gaussian blur widths in quadrature.
    pub fn quadrature_sum(terms: &[f64]) -> f64 {
        terms.iter().map(|t| t * t).sum::<f64>().sqrt()
    }

    /// Fraction of a 1-D Gaussian (sigma in pixels) that lands in one pixel,
    /// when the Gaussian's centre is `offset_px` from that pixel's centre.
    pub fn pixel_fraction_1d(sigma_px: f64, offset_px: f64) -> f64 {
        let edge = |x: f64| 0.5 * (1.0 + erf(x / (SQRT_2 * sigma_px)));
        edge(0.5 - offset_px) - edge(-0.5 - offset_px)
    }

    /// Fraction of a star's light in one pixel, for a round Gaussian image of
    /// the given FWHM (pixels) centred (dx, dy) pixels from that pixel's centre.
    ///
    /// The FWHM must be the image *before* the pixel aperture: integrating
    /// over the pixel is what applies the aperture here.
    pub fn peak_pixel_fraction(fwhm_px: f64, dx_px: f64, dy_px: f64) -> f64 {
        let sigma = fwhm_px / FWHM_PER_SIGMA;
        Self::pixel_fraction_1d(sigma, dx_px) * Self::pixel_fraction_1d(sigma, dy_px)
    }

    /// Fraction of a 1-D Gaussian (sigma in pixels) that lands in the brightest
    /// pixel when the image is smeared uniformly along a trail `trail_px` long,
    /// with the trail centred on that pixel.
    ///
    /// The trailed profile is (1/L) [Phi((x + L/2)/sigma) - Phi((x - L/2)/sigma)].
    /// Integrating it over the pixel uses the antiderivative of the Gaussian
    /// CDF, G(x) = x Phi(x/sigma) + sigma phi(x/sigma). A trail shorter than a
    /// thousandth of a pixel is treated as no trail.
    pub fn trailed_fraction_1d(sigma_px: f64, trail_px: f64) -> f64 {
        if trail_px < 1e-3 {
            return Self::pixel_fraction_1d(sigma_px, 0.0);
        }
        let g = |x: f64| {
            let u = x / sigma_px;
            x * normal_cdf(u) + sigma_px * normal_pdf(u)
        };
        let h = trail_px / 2.0;
        (g(0.5 + h) - g(-0.5 + h) - g(0.5 - h) + g(-0.5 - h)) / trail_px
    }

    /// Fraction of a star's light in its brightest pixel when the star is
    /// centred on a pixel and trailed along a pixel row by `trail_px`. With no
    /// trail this is `peak_pixel_fraction(fwhm_px, 0, 0)`.
    ///
    /// As with `peak_pixel_fraction`, the FWHM is the image before the pixel
    /// aperture.
    pub fn trailed_peak_pixel_fraction(fwhm_px: f64, trail_px: f64) -> f64 {
        let sigma = fwhm_px / FWHM_PER_SIGMA;
        Self::pixel_fraction_1d(sigma, 0.0) * Self::trailed_fraction_1d(sigma, trail_px)
    }

    /// One-sigma centroid precision per axis, in the units of `fwhm`, for a
    /// Gaussian star measured at the given SNR: sigma_psf / SNR.
    ///
    /// This is the photon-limited bound. Sky noise, read noise and coarse
    /// pixels all make the real figure larger.
    pub fn centroid_sigma(fwhm: f64, snr: f64) -> f64 {
        fwhm / FWHM_PER_SIGMA / snr
    }
}

/// Standard normal cumulative distribution.
fn normal_cdf(u: f64) -> f64 {
    0.5 * (1.0 + erf(u / SQRT_2))
}

/// Standard normal density.
fn normal_pdf(u: f64) -> f64 {
    (-0.5 * u * u).exp() / (2.0 * PI).sqrt()
}

/// Error function, Abramowitz and Stegun 7.1.26 (absolute error below 1.5e-7).
fn erf(x: f64) -> f64 {
    const P: f64 = 0.327_591_1;
    const A: [f64; 5] = [0.254_829_592, -0.284_496_736, 1.421_413_741, -1.453_152_027, 1.061_405_429];
    let sign = x.signum();
    let x = x.abs();
    let t = 1.0 / (1.0 + P * x);
    let poly = t * (A[0] + t * (A[1] + t * (A[2] + t * (A[3] + t * A[4]))));
    sign * (1.0 - poly * (-x * x).exp())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn erf_matches_tabulated_values() {
        assert!(close(erf(0.0), 0.0, 1e-7));
        assert!(close(erf(0.5), 0.520_499_9, 2e-7));
        assert!(close(erf(1.0), 0.842_700_8, 2e-7));
        assert!(close(erf(-1.0), -0.842_700_8, 2e-7));
        assert!(close(erf(3.0), 0.999_977_9, 2e-7));
    }

    #[test]
    fn diffraction_deltarho350_at_550nm() {
        // 1.029 x 0.55e-6 m / 0.35 m = 1.617e-6 rad = 0.334".
        assert!(close(PsfCalculator::diffraction_fwhm_arcsec(0.55, 350.0), 0.334, 0.001));
    }

    #[test]
    fn pixel_aperture_is_068_of_the_pitch() {
        assert!(close(PsfCalculator::pixel_aperture_fwhm_um(3.76), 2.556, 0.001));
    }

    #[test]
    fn diffusion_from_mtf() {
        // An ideal pixel (MTF 2/pi) has no diffusion, and neither does a
        // reading above it.
        assert_eq!(PsfCalculator::diffusion_fwhm_um(3.76, PIXEL_BOX_MTF_AT_NYQUIST), 0.0);
        assert_eq!(PsfCalculator::diffusion_fwhm_um(3.76, 0.70), 0.0);
        // MTF 0.5 at Nyquist: diffusion MTF 0.7854, sigma = 3.76/pi x sqrt(-2 ln 0.7854)
        // = 1.1968 x 0.6952 = 0.832 um, FWHM 1.959 um.
        assert!(close(PsfCalculator::diffusion_fwhm_um(3.76, 0.5), 1.959, 0.002));
        // Round trip: the Gaussian's own MTF at Nyquist gives the reading back.
        let fwhm = PsfCalculator::diffusion_fwhm_um(3.76, 0.5);
        let sigma = fwhm / FWHM_PER_SIGMA;
        let f = 1.0 / (2.0 * 3.76);
        let mtf = PIXEL_BOX_MTF_AT_NYQUIST * (-2.0 * PI * PI * sigma * sigma * f * f).exp();
        assert!(close(mtf, 0.5, 1e-9));
    }

    #[test]
    fn quadrature_of_three_four_twelve() {
        assert!(close(PsfCalculator::quadrature_sum(&[3.0, 4.0, 12.0]), 13.0, 1e-12));
    }

    #[test]
    fn peak_pixel_fraction_limits() {
        // A very sharp star centred on a pixel puts everything in it, and one
        // on a corner shares it four ways.
        assert!(close(PsfCalculator::peak_pixel_fraction(0.01, 0.0, 0.0), 1.0, 1e-9));
        assert!(close(PsfCalculator::peak_pixel_fraction(0.01, 0.5, 0.5), 0.25, 1e-9));
        // FWHM 2 px: sigma 0.8493 px, 1-D fraction erf(0.5 / (sqrt 2 x 0.8493)) = 0.4441,
        // so 0.1972 centred. On a corner each 1-D share is erf(1 / (sqrt 2 x sigma)) / 2 = 0.3805.
        assert!(close(PsfCalculator::peak_pixel_fraction(2.0, 0.0, 0.0), 0.1972, 0.0005));
        assert!(close(PsfCalculator::peak_pixel_fraction(2.0, 0.5, 0.5), 0.1449, 0.0005));
    }

    #[test]
    fn an_untrailed_star_matches_the_peak_pixel_fraction() {
        for fwhm in [0.5, 2.0, 4.0] {
            assert!(close(
                PsfCalculator::trailed_peak_pixel_fraction(fwhm, 0.0),
                PsfCalculator::peak_pixel_fraction(fwhm, 0.0, 0.0),
                1e-12
            ));
        }
    }

    #[test]
    fn a_tiny_trail_is_continuous_with_no_trail() {
        // Either side of the 1e-3 px cut-over must agree, or the closed form
        // is losing precision to cancellation.
        let sigma = 2.0 / FWHM_PER_SIGMA;
        let none = PsfCalculator::pixel_fraction_1d(sigma, 0.0);
        let tiny = PsfCalculator::trailed_fraction_1d(sigma, 1.01e-3);
        assert!(close(tiny, none, 1e-5), "{tiny} vs {none}");
    }

    #[test]
    fn a_long_trail_spreads_the_light_over_its_length() {
        // Far longer than the star, each pixel along the trail gets 1/L of
        // the light along the trail: 100 px gives 0.01 of the 1-D share.
        let sigma = 2.0 / FWHM_PER_SIGMA;
        assert!(close(PsfCalculator::trailed_fraction_1d(sigma, 100.0), 0.01, 1e-6));
        let across = PsfCalculator::pixel_fraction_1d(sigma, 0.0);
        assert!(close(PsfCalculator::trailed_peak_pixel_fraction(2.0, 100.0), across * 0.01, 1e-7));
    }

    #[test]
    fn a_sharp_star_trailed_two_pixels_puts_half_in_the_centre_pixel() {
        // A point trailed over [-1, 1] spends half its time in [-0.5, 0.5].
        assert!(close(PsfCalculator::trailed_fraction_1d(1e-4, 2.0), 0.5, 1e-6));
    }

    #[test]
    fn trailing_only_ever_lowers_the_peak() {
        let mut last = PsfCalculator::trailed_peak_pixel_fraction(3.0, 0.0);
        for trail in [0.01, 0.1, 0.5, 1.0, 3.0, 10.0, 1000.0] {
            let f = PsfCalculator::trailed_peak_pixel_fraction(3.0, trail);
            assert!(f < last, "trail {trail}: {f} not below {last}");
            last = f;
        }
    }

    #[test]
    fn centroid_sigma_is_psf_sigma_over_snr() {
        assert!(close(PsfCalculator::centroid_sigma(2.355, 10.0), 0.1, 1e-4));
    }
}
