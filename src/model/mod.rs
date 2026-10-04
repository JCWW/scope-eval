//! Core data types, organized by domain: optics, camera, site, mount, and a
//! full configuration tying them together.
//!
//! These are plain data. All of the math lives in `checks.rs`.

pub mod camera;
pub(crate) mod camera_dto;
pub mod config;
pub mod mount;
pub(crate) mod mount_dto;
pub mod optics;
pub(crate) mod optics_dto;
pub mod site;

pub use camera::{Camera, Shutter};
pub use config::Config;
pub use mount::{Capability, Mount, MountType, Payload};
pub use optics::{Obstruction, SpotConvention, SpotPoint, SpotSpec, Telescope};
pub use site::Site;

/// Keep a hand-entered value only if it is finite and inside a plausible range.
///
/// `presets.yaml` is edited by hand and nothing else validates it, and the
/// interactive prompts guard only some fields. A value outside its range is
/// treated as *not entered*: callers then substitute a documented default and
/// report that they did, so a typo degrades the report instead of corrupting
/// it. `None`, NaN and the infinities all fail.
///
/// Ranges live in [`crate::constants::plausible_ranges`].
pub fn plausible(value: Option<f64>, lo: f64, hi: f64) -> Option<f64> {
    value.filter(|v| v.is_finite() && *v >= lo && *v <= hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plausible_accepts_a_value_in_range() {
        assert_eq!(plausible(Some(0.8), 0.01, 1.0), Some(0.8));
        assert_eq!(plausible(Some(0.01), 0.01, 1.0), Some(0.01));
        assert_eq!(plausible(Some(1.0), 0.01, 1.0), Some(1.0));
    }

    #[test]
    fn plausible_rejects_out_of_range() {
        assert_eq!(plausible(Some(1.5), 0.01, 1.0), None);
        assert_eq!(plausible(Some(0.0), 0.01, 1.0), None);
        assert_eq!(plausible(Some(-0.2), 0.01, 1.0), None);
    }

    #[test]
    fn plausible_rejects_nan_and_infinity() {
        // `.nan` and `.inf` are both legal YAML scalars, and every f64
        // comparison against NaN is false, so an unguarded NaN would fall
        // through every grading branch to the final else.
        assert_eq!(plausible(Some(f64::NAN), 0.01, 1.0), None);
        assert_eq!(plausible(Some(f64::INFINITY), 0.01, 1.0), None);
        assert_eq!(plausible(Some(f64::NEG_INFINITY), 0.01, 1.0), None);
    }

    #[test]
    fn plausible_passes_none_through() {
        assert_eq!(plausible(None, 0.01, 1.0), None);
    }
}
