//! What a star (or a satellite) looks like on the sensor: scope-eval's point
//! spread function budget for a configuration, plus the smear the mount's
//! tracking jitter adds during an exposure.
//!
//! The optics, detector and seeing terms come from `scope_eval::psf`, so the
//! dashboard and the command-line tool size a star identically. Only the
//! jitter term is added here, because only the simulation has a jitter
//! figure. See docs/17-point-spread-function.md.

use serde::{Deserialize, Serialize};

use scope_eval::constants::{DEFAULT_SEEING_ARCSEC, DEFAULT_WAVELENGTH_UM, FWHM_PER_SIGMA};
use scope_eval::model::Site;
use scope_eval::psf::{FieldPoint, PsfBudget};

use crate::error::SimError;
use crate::hardware::Hardware;

/// Observing conditions that set the star's size. Absent fields take
/// scope-eval's defaults (2.5" seeing at 0.55 um).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Conditions {
    #[serde(default = "default_seeing")]
    pub seeing_arcsec: f64,
    #[serde(default = "default_wavelength")]
    pub wavelength_um: f64,
}

fn default_seeing() -> f64 {
    DEFAULT_SEEING_ARCSEC
}

fn default_wavelength() -> f64 {
    DEFAULT_WAVELENGTH_UM
}

impl Default for Conditions {
    fn default() -> Self {
        Conditions { seeing_arcsec: DEFAULT_SEEING_ARCSEC, wavelength_um: DEFAULT_WAVELENGTH_UM }
    }
}

/// A figure at the sensor centre and at its corner.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct CenterCorner {
    pub center: f64,
    pub corner: f64,
}

/// The star image for one configuration. Every width is a FWHM in arcsec.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StarImage {
    pub seeing_arcsec: f64,
    pub diffraction_arcsec: f64,
    /// `None` when the telescope has no spot data.
    pub optics_arcsec: Option<CenterCorner>,
    /// How the vendor's RMS spot was read, e.g. "RMS radius, assumed".
    pub spot_reading: Option<String>,
    pub optics_extrapolated: bool,
    /// `None` when the camera has no measured MTF.
    pub diffusion_arcsec: Option<f64>,
    pub pixel_arcsec: f64,
    /// Smear from the mount's tracking jitter over an exposure much longer
    /// than its correlation time: the per-axis RMS as a Gaussian FWHM.
    pub jitter_arcsec: f64,
    /// True when the jitter figure is the simulation's default.
    pub jitter_assumed: bool,
    /// The image the pixels sample (seeing, diffraction, optics, diffusion).
    pub sampled_fwhm: CenterCorner,
    /// The sampled image had an unlabelled spot been an RMS diameter.
    pub sampled_fwhm_if_diameter: Option<CenterCorner>,
    /// Plus the pixel aperture: the star as recorded.
    pub recorded_fwhm: CenterCorner,
    /// Plus tracking jitter: the star in a tracked exposure.
    pub tracked_fwhm: CenterCorner,
    pub plate_scale_arcsec: f64,
    /// Pixels across the sampled image at the centre.
    pub pixels_across: f64,
    /// Share of the light in the brightest pixel at the centre, with the
    /// star centred on a pixel and on a pixel corner.
    pub peak_pixel_fraction: CenterCorner,
    /// The largest single term at the centre.
    pub largest_term: String,
    /// Field radius of the sensor corner, mm.
    pub corner_radius_mm: f64,
    /// Inputs that were assumed or missing.
    pub assumed: Vec<String>,
}

/// Gaussian FWHM of the smear from jitter of the given total RMS on the
/// sky. The simulation splits the total equally between the two camera
/// axes, so each axis has sigma = total / sqrt(2).
pub fn jitter_smear_fwhm_arcsec(jitter_rms_arcsec: f64) -> f64 {
    FWHM_PER_SIGMA * jitter_rms_arcsec / std::f64::consts::SQRT_2
}

/// Build the star image from scope-eval's budget and the resolved hardware's
/// jitter figure.
pub fn star_image(
    telescope: &scope_eval::model::Telescope,
    camera: &scope_eval::model::Camera,
    hardware: &Hardware,
    conditions: &Conditions,
) -> Result<StarImage, SimError> {
    let positive = |name: &str, v: f64| {
        if v.is_finite() && v > 0.0 {
            Ok(v)
        } else {
            Err(SimError::new(format!("{name} must be a positive number, not {v}")))
        }
    };
    let site = Site {
        seeing_arcsec: positive("Seeing", conditions.seeing_arcsec)?,
        wavelength_um: positive("Wavelength", conditions.wavelength_um)?,
        sky_mag_arcsec2: None,
        location: None,
    };
    let b = PsfBudget::resolve(telescope, camera, &site);
    let both = |f: &dyn Fn(FieldPoint) -> f64| CenterCorner { center: f(FieldPoint::Center), corner: f(FieldPoint::Corner) };

    let jitter = hardware.mount_model.jitter_rms_arcsec;
    let jitter_arcsec = jitter_smear_fwhm_arcsec(jitter.value);
    let with_jitter = |recorded: f64| (recorded * recorded + jitter_arcsec * jitter_arcsec).sqrt();
    let recorded_fwhm = both(&|at| b.recorded_fwhm(at));

    let mut assumed: Vec<String> = b.assumed.iter().map(|s| s.to_string()).collect();
    if jitter.is_assumed() {
        assumed.push("tracking jitter".to_string());
    }

    Ok(StarImage {
        seeing_arcsec: b.seeing_arcsec,
        diffraction_arcsec: b.diffraction_arcsec,
        optics_arcsec: b.optics_arcsec.map(|(center, corner)| CenterCorner { center, corner }),
        spot_reading: b.spot_reading.map(str::to_string),
        optics_extrapolated: b.optics_extrapolated,
        diffusion_arcsec: b.diffusion_arcsec,
        pixel_arcsec: b.pixel_arcsec,
        jitter_arcsec,
        jitter_assumed: jitter.is_assumed(),
        sampled_fwhm: both(&|at| b.sampled_fwhm(at)),
        sampled_fwhm_if_diameter: b
            .sampled_fwhm_if_diameter(FieldPoint::Center)
            .zip(b.sampled_fwhm_if_diameter(FieldPoint::Corner))
            .map(|(center, corner)| CenterCorner { center, corner }),
        recorded_fwhm,
        tracked_fwhm: CenterCorner { center: with_jitter(recorded_fwhm.center), corner: with_jitter(recorded_fwhm.corner) },
        plate_scale_arcsec: b.plate_scale,
        pixels_across: b.sampled_fwhm(FieldPoint::Center) / b.plate_scale,
        peak_pixel_fraction: CenterCorner {
            center: b.peak_pixel_fraction(FieldPoint::Center, true),
            corner: b.peak_pixel_fraction(FieldPoint::Center, false),
        },
        largest_term: b.largest_term(FieldPoint::Center).to_string(),
        corner_radius_mm: b.corner_radius_mm,
        assumed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jitter_smear_is_1665_times_the_total_rms() {
        // sigma per axis = 1" / sqrt 2 = 0.7071", FWHM = 2.3548 x 0.7071 = 1.665".
        assert!((jitter_smear_fwhm_arcsec(1.0) - 1.665).abs() < 1e-3);
        assert_eq!(jitter_smear_fwhm_arcsec(0.0), 0.0);
    }

    #[test]
    fn default_conditions_are_scope_evals() {
        let c: Conditions = serde_yaml::from_str("{}").unwrap();
        assert_eq!(c, Conditions::default());
        assert_eq!(c.seeing_arcsec, 2.5);
        assert_eq!(c.wavelength_um, 0.55);
    }
}
