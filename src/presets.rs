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

fn presets_file() -> &'static PresetsFile {
    static PRESETS: OnceLock<PresetsFile> = OnceLock::new();
    PRESETS.get_or_init(|| {
        let text = std::fs::read_to_string(PRESETS_PATH)
            .unwrap_or_else(|e| panic!("failed to read presets file at {PRESETS_PATH}: {e}"));
        serde_yaml::from_str(&text)
            .unwrap_or_else(|e| panic!("failed to parse presets file at {PRESETS_PATH}: {e}"))
    })
}

pub fn telescopes() -> Vec<Telescope> {
    presets_file().telescopes.iter().cloned().map(Into::into).collect()
}

pub fn cameras() -> Vec<Camera> {
    presets_file().cameras.iter().cloned().map(Into::into).collect()
}

pub fn mounts() -> Vec<Mount> {
    presets_file().mounts.iter().cloned().map(Into::into).collect()
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
        // presets.yaml carries no vendor QE, throughput or dynamics data, and
        // must not: every value in that file has a `source`.
        assert!(scopes.iter().all(|t| t.throughput.is_none()));
        assert!(cams.iter().all(|c| c.qe.is_none()));
        assert!(ms.iter().all(|m| m.max_accel_deg_s2.is_none()));
        assert!(ms.iter().all(|m| m.settle_time_s.is_none()));
    }
}
