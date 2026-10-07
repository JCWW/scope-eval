//! Hardware presets, read from scope-eval's `presets.yaml`, and the
//! configurations the dashboard builds from them.
//!
//! Only the fields the simulation needs are read; everything else in the
//! file is ignored, so the file stays the single source of hardware numbers.

use serde::{Deserialize, Serialize};

use scope_eval::presets::PresetSet;

use crate::error::SimError;
use crate::geometry::MountKind;
use crate::star::{star_image, Conditions, StarImage};
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
    /// The same file in scope-eval's full model types, for the star image
    /// (spot sizes, obstruction, MTF and the rest). Not sent to the dashboard.
    #[serde(skip)]
    full: Option<PresetSet>,
}

/// Values the user enters on top of a mount preset. Each one, when present,
/// replaces the preset's figure (or the assumed default).
///
/// Rate and acceleration apply to both axes; the `_by_axis` forms set one
/// axis (azimuth then elevation, or hour angle then declination) and win
/// over the both-axes figure for that axis.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MountOverrides {
    pub max_rate_deg_s: Option<f64>,
    pub max_accel_deg_s2: Option<f64>,
    pub max_rate_deg_s_by_axis: [Option<f64>; 2],
    pub max_accel_deg_s2_by_axis: [Option<f64>; 2],
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

/// scope-eval's `presets.yaml`, compiled in so every front end (the
/// dashboard, the command-line runner, the tests) reads the same specs.
pub const PRESETS_YAML: &str = include_str!("../../../presets.yaml");

impl Presets {
    /// The compiled-in presets, `PRESETS_YAML`.
    pub fn builtin() -> Result<Presets, SimError> {
        Presets::from_yaml(PRESETS_YAML)
    }

    pub fn from_yaml(text: &str) -> Result<Presets, SimError> {
        let mut presets: Presets =
            serde_yaml::from_str(text).map_err(|e| SimError::new(format!("presets file: {e}")))?;
        presets.full = Some(PresetSet::from_yaml(text).map_err(|e| SimError::new(format!("presets file: {e}")))?);
        Ok(presets)
    }

    /// What a star looks like on this configuration's sensor under the given
    /// conditions: scope-eval's point spread function plus tracking jitter.
    pub fn star_image(&self, spec: &ConfigSpec, conditions: &Conditions) -> Result<StarImage, SimError> {
        let hardware = self.resolve(spec)?;
        let full = self.full.as_ref().ok_or_else(|| SimError::new("presets were not read with from_yaml"))?;
        let t = find(&full.telescopes, |p| &p.name, &spec.telescope, "telescope")?;
        let c = find(&full.cameras, |p| &p.name, &spec.camera, "camera")?;
        star_image(t, c, &hardware, conditions)
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
        let rate_by_axis = [
            positive("Maximum axis 1 rate", o.max_rate_deg_s_by_axis[0])?,
            positive("Maximum axis 2 rate", o.max_rate_deg_s_by_axis[1])?,
        ];
        let accel_by_axis = [
            positive("Maximum axis 1 acceleration", o.max_accel_deg_s2_by_axis[0])?,
            positive("Maximum axis 2 acceleration", o.max_accel_deg_s2_by_axis[1])?,
        ];
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
                max_rate_deg_s: [0, 1]
                    .map(|i| Param::first_or(&[rate_by_axis[i], rate, m.max_slew_deg_s], DEFAULT_MAX_RATE_DEG_S)),
                max_accel_deg_s2: [0, 1]
                    .map(|i| Param::first_or(&[accel_by_axis[i], accel, m.max_accel_deg_s2], DEFAULT_MAX_ACCEL_DEG_S2)),
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

    const YAML: &str = PRESETS_YAML;

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
        assert_eq!(p.telescopes.len(), 23);
        assert_eq!(p.cameras.len(), 9);
        assert!(p.mounts.len() >= 3);
    }

    #[test]
    fn missing_mount_figures_are_assumed_and_overrides_win() {
        let p = Presets::from_yaml(YAML).unwrap();
        let hw = p.resolve(&spec("PlaneWave L-350 (direct drive)", MountOverrides::default())).unwrap();
        let m = &hw.mount_model;
        assert_eq!(m.kind, MountKind::AltAz);
        assert_eq!(m.max_rate_deg_s, [Param::entered(50.0); 2]);
        assert_eq!(m.max_accel_deg_s2, [Param::assumed(DEFAULT_MAX_ACCEL_DEG_S2); 2]);
        assert_eq!(m.pointing_rms_arcsec, Param::assumed(DEFAULT_POINTING_RMS_ARCSEC));

        let o = MountOverrides { max_accel_deg_s2: Some(2.0), pointing_rms_arcsec: Some(30.0), ..Default::default() };
        let hw = p.resolve(&spec("PlaneWave L-350 (direct drive)", o)).unwrap();
        assert_eq!(hw.mount_model.max_accel_deg_s2, [Param::entered(2.0); 2]);
        assert_eq!(hw.mount_model.pointing_rms_arcsec, Param::entered(30.0));
    }

    #[test]
    fn per_axis_overrides_win_for_their_axis() {
        let p = Presets::from_yaml(YAML).unwrap();
        let o = MountOverrides {
            max_rate_deg_s: Some(10.0),
            max_rate_deg_s_by_axis: [None, Some(4.0)],
            max_accel_deg_s2_by_axis: [Some(3.0), None],
            ..Default::default()
        };
        let m = p.resolve(&spec("PlaneWave L-350 (direct drive)", o)).unwrap().mount_model;
        assert_eq!(m.max_rate_deg_s, [Param::entered(10.0), Param::entered(4.0)]);
        assert_eq!(m.max_accel_deg_s2, [Param::entered(3.0), Param::assumed(DEFAULT_MAX_ACCEL_DEG_S2)]);

        let bad = MountOverrides { max_rate_deg_s_by_axis: [Some(0.0), None], ..Default::default() };
        assert!(p.resolve(&spec("PlaneWave L-350 (direct drive)", bad)).is_err());
    }

    #[test]
    fn overrides_without_per_axis_fields_still_parse() {
        // What the dashboard sent before per-axis limits existed.
        let o: MountOverrides = serde_json::from_str(r#"{"max_rate_deg_s":2,"pointing_rms_arcsec":null}"#).unwrap();
        assert_eq!(o.max_rate_deg_s, Some(2.0));
        assert_eq!(o.max_rate_deg_s_by_axis, [None, None]);
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

    #[test]
    fn star_image_matches_scope_eval() {
        // The running example: scope-eval's recorded star is 3.030" at 2.5"
        // seeing (docs/17-point-spread-function.md).
        let p = Presets::from_yaml(YAML).unwrap();
        let s = p.star_image(&spec("PlaneWave L-350 (direct drive)", MountOverrides::default()), &Conditions::default()).unwrap();
        assert!((s.recorded_fwhm.center - 3.030).abs() < 0.001, "recorded {}", s.recorded_fwhm.center);
        assert!((s.sampled_fwhm.center - 2.988).abs() < 0.001);
        assert!((s.pixels_across - 4.05).abs() < 0.01);
        assert!(s.sampled_fwhm_if_diameter.is_some());
        // Default jitter 1" RMS smears by 1.665": sqrt(3.030^2 + 1.665^2) = 3.457".
        assert!(s.jitter_assumed);
        assert!((s.tracked_fwhm.center - 3.457).abs() < 0.002, "tracked {}", s.tracked_fwhm.center);
        assert!(s.assumed.iter().any(|a| a == "tracking jitter"));
    }

    #[test]
    fn star_image_follows_seeing_and_jitter_overrides() {
        let p = Presets::from_yaml(YAML).unwrap();
        let still = MountOverrides { jitter_rms_arcsec: Some(0.0), ..MountOverrides::default() };
        let spec = spec("PlaneWave L-350 (direct drive)", still);
        let good = p.star_image(&spec, &Conditions { seeing_arcsec: 1.0, wavelength_um: 0.55 }).unwrap();
        let bad = p.star_image(&spec, &Conditions { seeing_arcsec: 4.0, wavelength_um: 0.55 }).unwrap();
        assert!(good.recorded_fwhm.center < bad.recorded_fwhm.center);
        // No jitter: the tracked star is the recorded star, and nothing is assumed about jitter.
        assert_eq!(good.tracked_fwhm, good.recorded_fwhm);
        assert!(!good.jitter_assumed);
        // At 1" seeing the optics, not the air, are the largest term.
        assert_eq!(good.largest_term, "optics");
    }

    #[test]
    fn star_image_rejects_bad_conditions() {
        let p = Presets::from_yaml(YAML).unwrap();
        let spec = spec("PlaneWave L-350 (direct drive)", MountOverrides::default());
        assert!(p.star_image(&spec, &Conditions { seeing_arcsec: 0.0, wavelength_um: 0.55 }).is_err());
        assert!(p.star_image(&spec, &Conditions { seeing_arcsec: 2.5, wavelength_um: f64::NAN }).is_err());
    }
}
