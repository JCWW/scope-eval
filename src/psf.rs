//! The system point spread function: every blur between a star and the
//! recorded image, kept as a budget so the report can show which term
//! dominates.
//!
//! Terms, in the order light meets them: atmospheric seeing, diffraction,
//! the optics' aberrations (from the vendor's spot sizes), charge diffusion
//! in the detector (from a measured MTF), and the pixel aperture. Each is
//! treated as a Gaussian and combined in quadrature; the equations live in
//! `calculations::psf`. Like `photometry.rs`, this module records which
//! inputs were assumed rather than entered.
//!
//! Formulas and worked examples are in docs/17-point-spread-function.md.

use crate::calculations::optics::OpticsCalculator;
use crate::calculations::psf::PsfCalculator;
use crate::constants::plausible_ranges as ranges;
use crate::model::{plausible, Camera, Site, SpotConvention, Telescope};

/// Where on the sensor a figure applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldPoint {
    Center,
    /// The sensor corner, at half the sensor diagonal from the optical axis.
    Corner,
}

/// Every term of the point spread function, as FWHM in arcseconds.
#[derive(Debug, Clone, PartialEq)]
pub struct PsfBudget {
    pub seeing_arcsec: f64,
    pub diffraction_arcsec: f64,
    /// Optical aberrations at the centre and at the sensor corner. `None`
    /// when the telescope has no spot data, which leaves the term out.
    pub optics_arcsec: Option<(f64, f64)>,
    /// The optics term had the spot figure been an RMS diameter. Only set when
    /// the convention is unknown, so the report can show what hangs on it.
    pub optics_if_diameter_arcsec: Option<(f64, f64)>,
    /// True when the corner figure was extrapolated past the last quoted spot.
    pub optics_extrapolated: bool,
    /// How the vendor's RMS spot figure was read.
    pub spot_reading: Option<&'static str>,
    /// Charge diffusion, from the camera's MTF at Nyquist. `None` when no
    /// MTF was entered.
    pub diffusion_arcsec: Option<f64>,
    pub pixel_arcsec: f64,
    /// Native plate scale, arcsec per pixel.
    pub plate_scale: f64,
    /// Field radius of the sensor corner, mm.
    pub corner_radius_mm: f64,
    /// Human-readable names of the inputs that were assumed or missing.
    pub assumed: Vec<&'static str>,
}

impl PsfBudget {
    pub fn resolve(t: &Telescope, c: &Camera, site: &Site) -> Self {
        let fl = t.focal_length_mm;
        let to_arcsec = |um: f64| OpticsCalculator::focal_plane_to_sky_arcsec(um, fl);
        let mut assumed = Vec::new();
        let corner_radius_mm = c.diagonal_mm() / 2.0;

        let (optics_arcsec, optics_if_diameter_arcsec, optics_extrapolated, spot_reading) = match &t.spot {
            Some(s) if !s.points.is_empty() => {
                // An unknown convention is read as an RMS radius: that is what
                // Zemax spot diagrams report, and it is the larger blur.
                let (is_radius, reading) = match s.convention {
                    SpotConvention::RmsRadius => (true, "RMS radius"),
                    SpotConvention::RmsDiameter => (false, "RMS diameter"),
                    SpotConvention::Unknown => {
                        assumed.push("spot convention (read as RMS radius)");
                        (true, "RMS radius, assumed")
                    }
                };
                let at = |r: f64| OpticsCalculator::spot_rms_at(&s.points, r).unwrap();
                let (center, _) = at(0.0);
                let (corner, extrapolated) = at(corner_radius_mm);
                let fwhm = |rms, radius| to_arcsec(OpticsCalculator::spot_fwhm_um(rms, radius));
                let if_diameter = (s.convention == SpotConvention::Unknown)
                    .then(|| (fwhm(center, false), fwhm(corner, false)));
                (Some((fwhm(center, is_radius), fwhm(corner, is_radius))), if_diameter, extrapolated, Some(reading))
            }
            _ => {
                assumed.push("spot sizes (optics left out)");
                (None, None, false, None)
            }
        };

        let diffusion_arcsec = match plausible(c.mtf_nyquist, ranges::MTF_NYQUIST_MIN, ranges::MTF_NYQUIST_MAX) {
            Some(mtf) => Some(to_arcsec(PsfCalculator::diffusion_fwhm_um(c.pixel_um, mtf))),
            None => {
                assumed.push("detector MTF (pixel aperture only)");
                None
            }
        };

        PsfBudget {
            seeing_arcsec: site.seeing_arcsec,
            diffraction_arcsec: PsfCalculator::diffraction_fwhm_arcsec(site.wavelength_um, t.aperture_mm),
            optics_arcsec,
            optics_if_diameter_arcsec,
            optics_extrapolated,
            spot_reading,
            diffusion_arcsec,
            pixel_arcsec: to_arcsec(PsfCalculator::pixel_aperture_fwhm_um(c.pixel_um)),
            plate_scale: OpticsCalculator::plate_scale_arcsec_per_px(c.pixel_um, fl),
            corner_radius_mm,
            assumed,
        }
    }

    /// The optics term at a field point, zero when there is no spot data.
    pub fn optics_at(&self, at: FieldPoint) -> f64 {
        match (self.optics_arcsec, at) {
            (Some((center, _)), FieldPoint::Center) => center,
            (Some((_, corner)), FieldPoint::Corner) => corner,
            (None, _) => 0.0,
        }
    }

    /// Atmosphere, diffraction and optics: the image the telescope delivers
    /// to the focal plane.
    pub fn focal_plane_fwhm(&self, at: FieldPoint) -> f64 {
        PsfCalculator::quadrature_sum(&[self.seeing_arcsec, self.diffraction_arcsec, self.optics_at(at)])
    }

    /// `sampled_fwhm` had an unlabelled spot figure been an RMS diameter.
    /// `None` when the convention is known or there is no spot data.
    pub fn sampled_fwhm_if_diameter(&self, at: FieldPoint) -> Option<f64> {
        let (center, corner) = self.optics_if_diameter_arcsec?;
        let optics = if at == FieldPoint::Center { center } else { corner };
        let focal_plane = PsfCalculator::quadrature_sum(&[self.seeing_arcsec, self.diffraction_arcsec, optics]);
        Some(PsfCalculator::quadrature_sum(&[focal_plane, self.diffusion_arcsec.unwrap_or(0.0)]))
    }

    /// The focal-plane image plus charge diffusion: the light distribution
    /// the pixels sample. Sampling judgments use this.
    pub fn sampled_fwhm(&self, at: FieldPoint) -> f64 {
        PsfCalculator::quadrature_sum(&[self.focal_plane_fwhm(at), self.diffusion_arcsec.unwrap_or(0.0)])
    }

    /// The sampled image plus the pixel aperture: the star as recorded, and
    /// what a FWHM measured on a real frame would show.
    pub fn recorded_fwhm(&self, at: FieldPoint) -> f64 {
        PsfCalculator::quadrature_sum(&[self.sampled_fwhm(at), self.pixel_arcsec])
    }

    /// Fraction of a star's light in its brightest native pixel, with the star
    /// centred on a pixel (`centred`) or on a corner shared by four.
    pub fn peak_pixel_fraction(&self, at: FieldPoint, centred: bool) -> f64 {
        let offset = if centred { 0.0 } else { 0.5 };
        PsfCalculator::peak_pixel_fraction(self.sampled_fwhm(at) / self.plate_scale, offset, offset)
    }

    /// Name of the largest term at a field point.
    pub fn largest_term(&self, at: FieldPoint) -> &'static str {
        [
            ("seeing", self.seeing_arcsec),
            ("diffraction", self.diffraction_arcsec),
            ("optics", self.optics_at(at)),
            ("detector diffusion", self.diffusion_arcsec.unwrap_or(0.0)),
            ("pixel aperture", self.pixel_arcsec),
        ]
        .into_iter()
        .fold(("seeing", f64::MIN), |best, term| if term.1 > best.1 { term } else { best })
        .0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Obstruction, Shutter, SpotPoint, SpotSpec};

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    /// The running example: DeltaRho 350 + IMX455 at 2.5" seeing, 550 nm.
    fn deltarho350(spot: Option<SpotSpec>) -> Telescope {
        Telescope {
            name: "DeltaRho 350".into(),
            aperture_mm: 350.0,
            focal_length_mm: 1050.0,
            obstruction: Obstruction::ByDiameter(0.56),
            image_circle_mm: Some(60.0),
            back_focus_mm: None,
            weight_lb: None,
            throughput: None,
            spot,
            source: "test".into(),
        }
    }

    fn imx455(mtf_nyquist: Option<f64>) -> Camera {
        Camera {
            name: "IMX455".into(),
            pixel_um: 3.76,
            width_px: 9576,
            height_px: 6388,
            read_noise_e: None,
            qe: None,
            mtf_nyquist,
            shutter: Shutter::Global,
            weight_lb: None,
            source: "test".into(),
        }
    }

    fn site() -> Site {
        Site { seeing_arcsec: 2.5, wavelength_um: 0.55, sky_mag_arcsec2: None, location: None }
    }

    fn spot(convention: SpotConvention) -> SpotSpec {
        let p = |r, rms| SpotPoint { field_radius_mm: r, rms_um: rms };
        SpotSpec::new(convention, vec![p(0.0, 4.9), p(23.0, 6.2), p(30.0, 7.6)])
    }

    #[test]
    fn running_example_budget() {
        let b = PsfBudget::resolve(&deltarho350(Some(spot(SpotConvention::Unknown))), &imx455(None), &site());
        // Optics: 4.9 um RMS radius -> 8.159 um FWHM -> 1.603".
        assert!(close(b.optics_at(FieldPoint::Center), 1.603, 0.001));
        // Diffraction 0.334", pixel 0.68 x 0.7386" = 0.502".
        assert!(close(b.diffraction_arcsec, 0.334, 0.001));
        assert!(close(b.pixel_arcsec, 0.502, 0.001));
        // sqrt(2.5^2 + 0.334^2 + 1.603^2) = 2.988".
        assert!(close(b.sampled_fwhm(FieldPoint::Center), 2.988, 0.001));
        // With the pixel: sqrt(2.988^2 + 0.502^2) = 3.030".
        assert!(close(b.recorded_fwhm(FieldPoint::Center), 3.030, 0.001));
        assert_eq!(b.largest_term(FieldPoint::Center), "seeing");
        assert!(b.assumed.contains(&"spot convention (read as RMS radius)"));
        assert!(b.assumed.contains(&"detector MTF (pixel aperture only)"));
    }

    #[test]
    fn a_known_diameter_convention_halves_the_optics_term() {
        let radius = PsfBudget::resolve(&deltarho350(Some(spot(SpotConvention::RmsRadius))), &imx455(None), &site());
        let diameter = PsfBudget::resolve(&deltarho350(Some(spot(SpotConvention::RmsDiameter))), &imx455(None), &site());
        let ratio = diameter.optics_at(FieldPoint::Center) / radius.optics_at(FieldPoint::Center);
        assert!(close(ratio, 0.5, 1e-12));
        assert!(!radius.assumed.iter().any(|a| a.starts_with("spot convention")));
        assert_eq!(radius.sampled_fwhm_if_diameter(FieldPoint::Center), None);
    }

    #[test]
    fn an_unknown_convention_also_reports_the_diameter_reading() {
        let b = PsfBudget::resolve(&deltarho350(Some(spot(SpotConvention::Unknown))), &imx455(None), &site());
        // As a diameter the optics term is 0.802": sqrt(2.5^2 + 0.334^2 + 0.802^2) = 2.646".
        let alt = b.sampled_fwhm_if_diameter(FieldPoint::Center).unwrap();
        assert!(close(alt, 2.646, 0.001));
        assert!(alt < b.sampled_fwhm(FieldPoint::Center));
    }

    #[test]
    fn corner_uses_the_spot_at_half_the_sensor_diagonal() {
        let b = PsfBudget::resolve(&deltarho350(Some(spot(SpotConvention::RmsRadius))), &imx455(None), &site());
        // Half-diagonal 21.64 mm: 4.9 + (21.64/23) x 1.3 = 6.123 um -> 2.003".
        assert!(close(b.corner_radius_mm, 21.64, 0.005));
        assert!(close(b.optics_at(FieldPoint::Corner), 2.003, 0.002));
        assert!(!b.optics_extrapolated);
        assert!(b.sampled_fwhm(FieldPoint::Corner) > b.sampled_fwhm(FieldPoint::Center));
    }

    #[test]
    fn missing_spot_data_leaves_optics_out_and_says_so() {
        let b = PsfBudget::resolve(&deltarho350(None), &imx455(None), &site());
        assert_eq!(b.optics_arcsec, None);
        assert!(close(b.focal_plane_fwhm(FieldPoint::Center), 2.5f64.hypot(0.334), 0.001));
        assert!(b.assumed.contains(&"spot sizes (optics left out)"));
    }

    #[test]
    fn a_measured_mtf_adds_diffusion() {
        let without = PsfBudget::resolve(&deltarho350(None), &imx455(None), &site());
        let with = PsfBudget::resolve(&deltarho350(None), &imx455(Some(0.5)), &site());
        // 1.959 um at 1050 mm = 0.385".
        assert!(close(with.diffusion_arcsec.unwrap(), 0.385, 0.001));
        assert!(with.sampled_fwhm(FieldPoint::Center) > without.sampled_fwhm(FieldPoint::Center));
        assert!(!with.assumed.iter().any(|a| a.starts_with("detector MTF")));
    }

    #[test]
    fn an_implausible_mtf_is_treated_as_not_entered() {
        let b = PsfBudget::resolve(&deltarho350(None), &imx455(Some(55.0)), &site());
        assert_eq!(b.diffusion_arcsec, None);
        assert!(b.assumed.contains(&"detector MTF (pixel aperture only)"));
    }

    #[test]
    fn peak_pixel_fraction_is_higher_when_centred() {
        let b = PsfBudget::resolve(&deltarho350(None), &imx455(None), &site());
        let centred = b.peak_pixel_fraction(FieldPoint::Center, true);
        let corner = b.peak_pixel_fraction(FieldPoint::Center, false);
        assert!(centred > corner && corner > 0.0 && centred < 1.0);
    }
}
