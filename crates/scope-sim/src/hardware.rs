//! The hardware a simulation runs: the optics that set the field of view,
//! and the mount that has to keep the target in it.

use serde::Serialize;

use crate::geometry::MountKind;

const ARCSEC_PER_RADIAN: f64 = 206_264.806;

/// Axis rate assumed when a mount publishes none, deg/s.
pub const DEFAULT_MAX_RATE_DEG_S: f64 = 5.0;
/// Axis acceleration assumed when a mount publishes none, deg/s^2. No
/// preset publishes one; vendors quote it on request.
pub const DEFAULT_MAX_ACCEL_DEG_S2: f64 = 5.0;
/// Pointing-model error assumed when none is entered, arcsec RMS. Matches
/// scope-eval's `DEFAULT_POINTING_RMS_ARCSEC`.
pub const DEFAULT_POINTING_RMS_ARCSEC: f64 = 60.0;
/// Short-term tracking jitter assumed when none is entered, arcsec RMS.
pub const DEFAULT_JITTER_RMS_ARCSEC: f64 = 1.0;
/// Position-loop gain of the axis servo, 1/s. A 4/s loop corrects a small
/// error with a 0.25 s time constant.
pub const SERVO_GAIN_PER_S: f64 = 4.0;

/// A number that is either entered (from a preset or an override) or
/// assumed. The dashboard shows which, as scope-eval does, so a result
/// resting on an assumption is never mistaken for one resting on a spec.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Param {
    pub value: f64,
    pub assumed: bool,
}

impl Param {
    pub fn entered(value: f64) -> Param {
        Param { value, assumed: false }
    }
    pub fn assumed(value: f64) -> Param {
        Param { value, assumed: true }
    }
    /// The first of `values` that is present, else `default`, assumed.
    pub fn first_or(values: &[Option<f64>], default: f64) -> Param {
        values.iter().flatten().next().map_or(Param::assumed(default), |&v| Param::entered(v))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Optics {
    pub aperture_mm: f64,
    pub focal_length_mm: f64,
    pub pixel_um: f64,
    pub width_px: u32,
    pub height_px: u32,
    /// Native plate scale, arcsec per pixel.
    pub plate_scale_arcsec: f64,
    pub fov_w_deg: f64,
    pub fov_h_deg: f64,
}

impl Optics {
    pub fn new(aperture_mm: f64, focal_length_mm: f64, pixel_um: f64, width_px: u32, height_px: u32) -> Optics {
        let size_deg = |px: u32| (px as f64 * pixel_um / 1000.0 / focal_length_mm).to_degrees();
        Optics {
            aperture_mm,
            focal_length_mm,
            pixel_um,
            width_px,
            height_px,
            plate_scale_arcsec: ARCSEC_PER_RADIAN * (pixel_um / 1000.0) / focal_length_mm,
            fov_w_deg: size_deg(width_px),
            fov_h_deg: size_deg(height_px),
        }
    }

    /// Half the field's width and height, radians.
    pub fn half_fov_rad(&self) -> (f64, f64) {
        (self.fov_w_deg.to_radians() / 2.0, self.fov_h_deg.to_radians() / 2.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MountModel {
    pub kind: MountKind,
    /// True when the preset's mount type was unknown and alt-az was assumed.
    pub kind_assumed: bool,
    pub max_rate_deg_s: Param,
    pub max_accel_deg_s2: Param,
    /// Static pointing-model error, total RMS on the sky.
    pub pointing_rms_arcsec: Param,
    /// Short-term tracking jitter, total RMS on the sky.
    pub jitter_rms_arcsec: Param,
    pub servo_gain_per_s: f64,
}

/// A fully resolved configuration: everything the simulation needs.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Hardware {
    pub name: String,
    pub telescope: String,
    pub camera: String,
    pub mount: String,
    pub optics: Optics,
    pub mount_model: MountModel,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deltarho350_imx455_matches_scope_eval() {
        // Lesson 2 and 4 worked examples: 0.739 "/px, 1.96 x 1.31 deg.
        let o = Optics::new(350.0, 1050.0, 3.76, 9576, 6388);
        assert!((o.plate_scale_arcsec - 0.7386).abs() < 1e-3);
        assert!((o.fov_w_deg - 1.964).abs() < 2e-3);
        assert!((o.fov_h_deg - 1.310).abs() < 2e-3);
    }

    #[test]
    fn param_prefers_the_first_entered_value() {
        assert_eq!(Param::first_or(&[None, Some(3.0), Some(4.0)], 9.0), Param::entered(3.0));
        assert_eq!(Param::first_or(&[None, None], 9.0), Param::assumed(9.0));
    }
}
