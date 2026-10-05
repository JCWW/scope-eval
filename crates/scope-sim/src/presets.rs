//! Hardware presets, read from scope-eval's `presets.yaml`, and the
//! configurations the dashboard builds from them.
//!
//! Only the fields the simulation needs are read; everything else in the
//! file is ignored, so the file stays the single source of hardware numbers.

use serde::{Deserialize, Serialize};

use crate::error::SimError;
use crate::geometry::MountKind;
use crate::hardware::{
    Hardware, MountModel, Optics, Param, DEFAULT_JITTER_RMS_ARCSEC, DEFAULT_MAX_ACCEL_DEG_S2,
    DEFAULT_MAX_RATE_DEG_S, DEFAULT_POINTING_RMS_ARCSEC, SERVO_GAIN_PER_S,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TelescopePreset {
    pub name: String,
    pub aperture_mm: f64,
    pub focal_length_mm: f64,
    #[serde(default)]
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CameraPreset {
    pub name: String,
    pub pixel_um: f64,
    pub width_px: u32,
    pub height_px: u32,
    #[serde(default)]
    pub source: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresetMountType {
    AltAz,
    Equatorial,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MountPreset {
    pub name: String,
    pub mount_type: PresetMountType,
    pub max_slew_deg_s: Option<f64>,
    #[serde(default)]
    pub max_accel_deg_s2: Option<f64>,
    #[serde(default)]
    pub pointing_rms_arcsec: Option<f64>,
    #[serde(default)]
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Presets {
    pub telescopes: Vec<TelescopePreset>,
    pub cameras: Vec<CameraPreset>,
    pub mounts: Vec<MountPreset>,
}

/// Values the user enters on top of a mount preset. Each one, when present,
/// replaces the preset's figure (or the assumed default).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MountOverrides {
    pub max_rate_deg_s: Option<f64>,
    pub max_accel_deg_s2: Option<f64>,
    pub pointing_rms_arcsec: Option<f64>,
    pub jitter_rms_arcsec: Option<f64>,
}

/// A named configuration: which presets to combine, plus overrides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigSpec {
    pub name: String,
    pub telescope: String,
    pub camera: String,
    pub mount: String,
    #[serde(default)]
    pub mount_overrides: MountOverrides,
}

impl Presets {
    pub fn from_yaml(text: &str) -> Result<Presets, SimError> {
        serde_yaml::from_str(text).map_err(|e| SimError::new(format!("presets file: {e}")))
    }

    pub fn resolve(&self, spec: &ConfigSpec) -> Result<Hardware, SimError> {
        let t = find(&self.telescopes, |p| &p.name, &spec.telescope, "telescope")?;
        let c = find(&self.cameras, |p| &p.name, &spec.camera, "camera")?;
        let m = find(&self.mounts, |p| &p.name, &spec.mount, "mount")?;
        let o = &spec.mount_overrides;

        let positive = |name: &str, v: Option<f64>| match v {
            Some(x) if !(x.is_finite() && x > 0.0) => Err(SimError::new(format!("{name} must be a positive number, not {x}"))),
            _ => Ok(v),
        };
        let non_negative = |name: &str, v: Option<f64>| match v {
            Some(x) if !(x.is_finite() && x >= 0.0) => Err(SimError::new(format!("{name} must be zero or more, not {x}"))),
            _ => Ok(v),
        };
        let rate = positive("Maximum axis rate", o.max_rate_deg_s)?;
        let accel = positive("Maximum axis acceleration", o.max_accel_deg_s2)?;
        let pointing = non_negative("Pointing RMS", o.pointing_rms_arcsec)?;
        let jitter = non_negative("Tracking jitter RMS", o.jitter_rms_arcsec)?;

        let (kind, kind_assumed) = match m.mount_type {
            PresetMountType::AltAz => (MountKind::AltAz, false),
            PresetMountType::Equatorial => (MountKind::Equatorial, false),
            PresetMountType::Unknown => (MountKind::AltAz, true),
        };
        Ok(Hardware {
            name: spec.name.clone(),
            telescope: t.name.clone(),
            camera: c.name.clone(),
            mount: m.name.clone(),
            optics: Optics::new(t.aperture_mm, t.focal_length_mm, c.pixel_um, c.width_px, c.height_px),
            mount_model: MountModel {
                kind,
                kind_assumed,
                max_rate_deg_s: Param::first_or(&[rate, m.max_slew_deg_s], DEFAULT_MAX_RATE_DEG_S),
                max_accel_deg_s2: Param::first_or(&[accel, m.max_accel_deg_s2], DEFAULT_MAX_ACCEL_DEG_S2),
                pointing_rms_arcsec: Param::first_or(&[pointing, m.pointing_rms_arcsec], DEFAULT_POINTING_RMS_ARCSEC),
                jitter_rms_arcsec: Param::first_or(&[jitter], DEFAULT_JITTER_RMS_ARCSEC),
                servo_gain_per_s: SERVO_GAIN_PER_S,
            },
        })
    }
}

fn find<'a, T>(items: &'a [T], name: impl Fn(&T) -> &String, wanted: &str, what: &str) -> Result<&'a T, SimError> {
    items
        .iter()
        .find(|p| name(p) == wanted)
        .ok_or_else(|| SimError::new(format!("no {what} preset named \"{wanted}\"")))
}

#[cfg(test)]
mod tests {
    use super::*;

    const YAML: &str = include_str!("../../../presets.yaml");

    fn spec(mount: &str, o: MountOverrides) -> ConfigSpec {
        ConfigSpec {
            name: "test".into(),
            telescope: "PlaneWave DeltaRho 350 (14\" f/3)".into(),
            camera: "Sony IMX455 full frame (Moravian C3-61000 PRO, QHY600 PRO)".into(),
            mount: mount.into(),
            mount_overrides: o,
        }
    }

    #[test]
    fn reads_scope_evals_presets_file() {
        let p = Presets::from_yaml(YAML).unwrap();
        assert_eq!(p.telescopes.len(), 5);
        assert_eq!(p.cameras.len(), 4);
        assert!(p.mounts.len() >= 3);
    }

    #[test]
    fn missing_mount_figures_are_assumed_and_overrides_win() {
        let p = Presets::from_yaml(YAML).unwrap();
        let hw = p.resolve(&spec("PlaneWave L-350 (direct drive)", MountOverrides::default())).unwrap();
        let m = &hw.mount_model;
        assert_eq!(m.kind, MountKind::AltAz);
        assert_eq!(m.max_rate_deg_s, Param::entered(50.0));
        assert_eq!(m.max_accel_deg_s2, Param::assumed(DEFAULT_MAX_ACCEL_DEG_S2));
        assert_eq!(m.pointing_rms_arcsec, Param::assumed(DEFAULT_POINTING_RMS_ARCSEC));

        let o = MountOverrides { max_accel_deg_s2: Some(2.0), pointing_rms_arcsec: Some(30.0), ..Default::default() };
        let hw = p.resolve(&spec("PlaneWave L-350 (direct drive)", o)).unwrap();
        assert_eq!(hw.mount_model.max_accel_deg_s2, Param::entered(2.0));
        assert_eq!(hw.mount_model.pointing_rms_arcsec, Param::entered(30.0));
    }

    #[test]
    fn rejects_unknown_names_and_bad_overrides() {
        let p = Presets::from_yaml(YAML).unwrap();
        assert!(p.resolve(&spec("No Such Mount", MountOverrides::default())).is_err());
        let bad = MountOverrides { max_accel_deg_s2: Some(-1.0), ..Default::default() };
        assert!(p.resolve(&spec("PlaneWave L-350 (direct drive)", bad)).is_err());
        let nan = MountOverrides { pointing_rms_arcsec: Some(f64::NAN), ..Default::default() };
        assert!(p.resolve(&spec("PlaneWave L-350 (direct drive)", nan)).is_err());
    }
}
