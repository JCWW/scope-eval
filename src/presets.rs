//! Preset telescopes, cameras, and mounts, loaded from `presets.yaml` at the
//! project root.
//!
//! Values come from vendor and dealer listings as noted in each `source`
//! field. Always confirm against the current spec sheet before a purchase.

use std::sync::OnceLock;

use serde::Deserialize;

use crate::model::camera_dto::CameraDto;
use crate::model::mount_dto::MountDto;
use crate::model::optics_dto::TelescopeDto;
use crate::model::{Camera, Mount, Telescope};

const PRESETS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/presets.yaml");

#[derive(Debug, Deserialize)]
struct PresetsFile {
    telescopes: Vec<TelescopeDto>,
    cameras: Vec<CameraDto>,
    mounts: Vec<MountDto>,
}

/// Every preset in a presets file, in the model types.
///
/// `scope-sim` parses the same file from text compiled into the dashboard,
/// so this is public: it lets the simulation reuse the point spread
/// function model instead of a copy of it.
#[derive(Debug, Clone, PartialEq)]
pub struct PresetSet {
    pub telescopes: Vec<Telescope>,
    pub cameras: Vec<Camera>,
    pub mounts: Vec<Mount>,
}

impl PresetSet {
    pub fn from_yaml(text: &str) -> Result<PresetSet, String> {
        let file: PresetsFile = serde_yaml::from_str(text).map_err(|e| e.to_string())?;
        Ok(PresetSet {
            telescopes: file.telescopes.into_iter().map(Into::into).collect(),
            cameras: file.cameras.into_iter().map(Into::into).collect(),
            mounts: file.mounts.into_iter().map(Into::into).collect(),
        })
    }
}

fn presets_file() -> &'static PresetSet {
    static PRESETS: OnceLock<PresetSet> = OnceLock::new();
    PRESETS.get_or_init(|| {
        let text = std::fs::read_to_string(PRESETS_PATH)
            .unwrap_or_else(|e| panic!("failed to read presets file at {PRESETS_PATH}: {e}"));
        PresetSet::from_yaml(&text)
            .unwrap_or_else(|e| panic!("failed to parse presets file at {PRESETS_PATH}: {e}"))
    })
}

pub fn telescopes() -> Vec<Telescope> {
    presets_file().telescopes.clone()
}

pub fn cameras() -> Vec<Camera> {
    presets_file().cameras.clone()
}

pub fn mounts() -> Vec<Mount> {
    presets_file().mounts.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_parse_with_new_fields_absent() {
        let scopes = telescopes();
        let cams = cameras();
        let ms = mounts();
        assert!(!scopes.is_empty() && !cams.is_empty() && !ms.is_empty());
        // presets.yaml carries no vendor QE, full well, throughput or dynamics data, and
        // must not: every value in that file has a `source`.
        assert!(scopes.iter().all(|t| t.throughput.is_none()));
        assert!(cams.iter().all(|c| c.qe.is_none()));
        assert!(cams.iter().all(|c| c.full_well_e.is_none()));
        assert!(ms.iter().all(|m| m.max_accel_deg_s2.is_none()));
        assert!(ms.iter().all(|m| m.settle_time_s.is_none()));
    }
}
