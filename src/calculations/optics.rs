//! Telescope and image-plane geometry calculations.

use crate::constants::{
    checks_limits as limits, ARCSEC_PER_RADIAN, DEG_PER_RADIAN, FWHM_PER_RMS_RADIUS, MM_PER_M,
    UM_PER_MM,
};
use crate::model::SpotPoint;
use std::f64::consts::PI;

/// Independent calculations for telescope optics, detector sampling, and
/// the geometry of their projected image on the sensor.
pub struct OpticsCalculator;

impl OpticsCalculator {
    /// Plate scale: sky angle seen by one pixel, arcsec/px.
    /// Small-angle rule: angle = size / distance, with focal length as distance.
    pub fn plate_scale_arcsec_per_px(pixel_um: f64, focal_length_mm: f64) -> f64 {
        ARCSEC_PER_RADIAN * (pixel_um / UM_PER_MM) / focal_length_mm
    }

    /// Number of pixels spanning a star's FWHM.
    pub fn pixels_across_star(fwhm_arcsec: f64, plate_scale: f64) -> f64 {
        fwhm_arcsec / plate_scale
    }

    /// Square bin factor (1..=MAX_BIN) that brings sampling closest to target.
    pub fn best_bin(pixels_across: f64) -> u32 {
        (1..=limits::MAX_BIN)
            .min_by(|a, b| {
                let da = (pixels_across / *a as f64 - limits::SAMPLING_TARGET).abs();
                let db = (pixels_across / *b as f64 - limits::SAMPLING_TARGET).abs();
                da.partial_cmp(&db).unwrap()
            })
            .unwrap()
    }

    /// Nearest square bin and effective pixel size for a desired pixel size.
    pub fn closest_binned_pixel_um(pixel_um: f64, ideal_pixel_um: f64) -> (u32, f64) {
        let bin = (1..=limits::MAX_BIN)
            .min_by(|x, y| {
                let error_x = (pixel_um * *x as f64 / ideal_pixel_um).ln().abs();
                let error_y = (pixel_um * *y as f64 / ideal_pixel_um).ln().abs();
                error_x.partial_cmp(&error_y).unwrap()
            })
            .unwrap();
        (bin, pixel_um * bin as f64)
    }

    /// Number of native detector pixels in a square seeing footprint.
    pub fn star_footprint_px(pixels_across: f64) -> f64 {
        pixels_across * pixels_across
    }

    /// Relative read-noise variance versus the ideal sampling footprint.
    pub fn read_noise_variance_penalty(footprint_px: f64, ideal_footprint_px: f64) -> f64 {
        (footprint_px / ideal_footprint_px).max(1.0)
    }

    /// Pixel size (um) that gives the target number of pixels across a star.
    pub fn ideal_pixel_um(fwhm_arcsec: f64, focal_length_mm: f64) -> f64 {
        (fwhm_arcsec / limits::SAMPLING_TARGET) / ARCSEC_PER_RADIAN * focal_length_mm * UM_PER_MM
    }

    /// Physical size of the seeing blur on the focal plane, um.
    pub fn seeing_blur_um(seeing_arcsec: f64, focal_length_mm: f64) -> f64 {
        seeing_arcsec / ARCSEC_PER_RADIAN * focal_length_mm * UM_PER_MM
    }

    /// Sky angle of a size on the focal plane, arcsec: the inverse of
    /// `seeing_blur_um`.
    pub fn focal_plane_to_sky_arcsec(size_um: f64, focal_length_mm: f64) -> f64 {
        size_um / UM_PER_MM / focal_length_mm * ARCSEC_PER_RADIAN
    }

    /// Convert a quoted RMS spot figure to approximate FWHM for a Gaussian blur.
    pub fn spot_fwhm_um(rms_um: f64, is_radius: bool) -> f64 {
        if is_radius {
            FWHM_PER_RMS_RADIUS * rms_um
        } else {
            FWHM_PER_RMS_RADIUS * rms_um / 2.0
        }
    }

    /// Combine two independent Gaussian blur widths in quadrature.
    pub fn quadrature(a: f64, b: f64) -> f64 {
        a.hypot(b)
    }

    /// Fractional increase in star width after combining independent blurs.
    pub fn blur_growth_fraction(seeing_fwhm: f64, optical_fwhm: f64) -> f64 {
        Self::quadrature(seeing_fwhm, optical_fwhm) / seeing_fwhm - 1.0
    }

    /// RMS spot at a field radius by linear interpolation between quoted points.
    /// Beyond the last point, extrapolates linearly from the last two points and
    /// flags the result.
    pub fn spot_rms_at(points: &[SpotPoint], radius_mm: f64) -> Option<(f64, bool)> {
        let first = points.first()?;
        if points.len() == 1 {
            return Some((first.rms_um, radius_mm > first.field_radius_mm));
        }
        if radius_mm <= first.field_radius_mm {
            return Some((first.rms_um, false));
        }
        for w in points.windows(2) {
            let (a, b) = (w[0], w[1]);
            if radius_mm <= b.field_radius_mm {
                let span = b.field_radius_mm - a.field_radius_mm;
                if span <= 0.0 {
                    return Some((b.rms_um, false));
                }
                let t = (radius_mm - a.field_radius_mm) / span;
                return Some((a.rms_um + t * (b.rms_um - a.rms_um), false));
            }
        }
        let n = points.len();
        let (a, b) = (points[n - 2], points[n - 1]);
        let span = b.field_radius_mm - a.field_radius_mm;
        let slope = if span > 0.0 {
            (b.rms_um - a.rms_um) / span
        } else {
            0.0
        };
        let value = b.rms_um + slope * (radius_mm - b.field_radius_mm);
        Some((value.max(b.rms_um), true))
    }

    /// Light-collecting area after subtracting the central obstruction, m^2.
    pub fn effective_area_m2(aperture_mm: f64, blocked_area_fraction: f64) -> f64 {
        Self::geometric_area_m2(aperture_mm) * (1.0 - blocked_area_fraction)
    }

    /// Unobstructed geometric aperture area, m^2.
    pub fn geometric_area_m2(aperture_mm: f64) -> f64 {
        let d_m = aperture_mm / MM_PER_M;
        PI / 4.0 * d_m * d_m
    }

    /// Unobstructed aperture diameter equivalent to a collecting area, mm.
    pub fn equivalent_aperture_mm(area_m2: f64) -> f64 {
        2.0 * (area_m2 / PI).sqrt() * MM_PER_M
    }

    /// Depth gain in magnitudes from collecting more light. Positive means fainter.
    pub fn delta_mag(area_m2: f64, reference_area_m2: f64) -> f64 {
        2.5 * (area_m2 / reference_area_m2).log10()
    }

    /// Field of view along one sensor axis, degrees (small-angle approximation).
    pub fn fov_deg(sensor_mm: f64, focal_length_mm: f64) -> f64 {
        sensor_mm / focal_length_mm * DEG_PER_RADIAN
    }

    /// Rectangular field area, square degrees.
    pub fn field_area_deg2(width_deg: f64, height_deg: f64) -> f64 {
        width_deg * height_deg
    }

    /// Etendue, collecting area multiplied by field area, m^2 deg^2.
    pub fn etendue_m2_deg2(area_m2: f64, field_area_deg2: f64) -> f64 {
        area_m2 * field_area_deg2
    }

    /// Critical focus zone half-width, um: +/- 2.44 * wavelength * focal_ratio^2.
    pub fn critical_focus_zone_um(wavelength_um: f64, f_ratio: f64) -> f64 {
        2.44 * wavelength_um * f_ratio * f_ratio
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn plate_scale_deltarho350_imx455() {
        assert!(close(
            OpticsCalculator::plate_scale_arcsec_per_px(3.76, 1050.0),
            0.7386,
            0.0005
        ));
    }

    #[test]
    fn plate_scale_rasa11_imx455() {
        assert!(close(
            OpticsCalculator::plate_scale_arcsec_per_px(3.76, 620.0),
            1.2509,
            0.0005
        ));
    }

    #[test]
    fn field_of_view_example() {
        assert!(close(OpticsCalculator::fov_deg(36.0, 1050.0), 1.964, 0.002));
        assert!(close(OpticsCalculator::fov_deg(24.0, 1050.0), 1.310, 0.002));
    }

    #[test]
    fn effective_area_and_depth() {
        let dr = OpticsCalculator::effective_area_m2(350.0, 0.56 * 0.56);
        let rasa = OpticsCalculator::effective_area_m2(279.0, (114.0_f64 / 279.0).powi(2));
        assert!(close(dr, 0.0660, 0.0005));
        assert!(close(rasa, 0.0509, 0.0005));
        assert!(close(OpticsCalculator::delta_mag(dr, rasa), 0.28, 0.01));
    }

    #[test]
    fn critical_focus_zone_values() {
        assert!(close(
            OpticsCalculator::critical_focus_zone_um(0.55, 3.0),
            12.08,
            0.01
        ));
        assert!(close(
            OpticsCalculator::critical_focus_zone_um(0.55, 2.2),
            6.50,
            0.01
        ));
    }

    #[test]
    fn ideal_pixel_values() {
        assert!(close(
            OpticsCalculator::ideal_pixel_um(2.5, 1050.0),
            6.36,
            0.01
        ));
        assert!(close(
            OpticsCalculator::ideal_pixel_um(2.5, 620.0),
            3.76,
            0.01
        ));
    }

    #[test]
    fn best_bin_choices() {
        assert_eq!(OpticsCalculator::best_bin(2.0), 1);
        assert_eq!(OpticsCalculator::best_bin(3.38), 2);
        assert_eq!(OpticsCalculator::best_bin(8.25), 4);
    }

    #[test]
    fn closest_binned_pixel_matches_on_ratio_scale() {
        assert_eq!(
            OpticsCalculator::closest_binned_pixel_um(3.76, 6.36),
            (2, 7.52)
        );
    }

    #[test]
    fn sampling_footprint_and_read_noise_penalty() {
        let footprint = OpticsCalculator::star_footprint_px(4.0);
        assert_eq!(footprint, 16.0);
        assert_eq!(
            OpticsCalculator::read_noise_variance_penalty(footprint, 4.0),
            4.0
        );
        assert_eq!(
            OpticsCalculator::read_noise_variance_penalty(footprint, 16.0),
            1.0
        );
    }

    #[test]
    fn blur_growth_uses_independent_blurs_in_quadrature() {
        assert!(close(
            OpticsCalculator::blur_growth_fraction(10.0, 10.0),
            2.0f64.sqrt() - 1.0,
            1e-12
        ));
    }

    #[test]
    fn area_equivalent_aperture_and_etendue() {
        let geometric = OpticsCalculator::geometric_area_m2(100.0);
        let effective = OpticsCalculator::effective_area_m2(100.0, 0.25);
        assert!(close(effective, geometric * 0.75, 1e-12));
        assert!(close(
            OpticsCalculator::equivalent_aperture_mm(geometric),
            100.0,
            1e-9
        ));
        let field = OpticsCalculator::field_area_deg2(2.0, 3.0);
        assert_eq!(field, 6.0);
        assert_eq!(OpticsCalculator::etendue_m2_deg2(0.5, field), 3.0);
    }

    #[test]
    fn spot_interpolation() {
        let pts = vec![
            SpotPoint {
                field_radius_mm: 0.0,
                rms_um: 4.9,
            },
            SpotPoint {
                field_radius_mm: 23.0,
                rms_um: 6.2,
            },
            SpotPoint {
                field_radius_mm: 30.0,
                rms_um: 7.6,
            },
        ];
        let (mid, ex) = OpticsCalculator::spot_rms_at(&pts, 11.5).unwrap();
        assert!(close(mid, 5.55, 0.001) && !ex);
        let (beyond, ex) = OpticsCalculator::spot_rms_at(&pts, 32.0).unwrap();
        assert!(close(beyond, 8.0, 0.001) && ex);
    }
}
