//! Wire format for [`super::mount`] types, as read from a preset config file.

use serde::Deserialize;

use super::mount::{Capability, Mount, MountType};

/// Wire format for [`MountType`].
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MountTypeDto {
    AltAz,
    Equatorial,
    Unknown,
}

impl From<MountTypeDto> for MountType {
    fn from(dto: MountTypeDto) -> Self {
        match dto {
            MountTypeDto::AltAz => MountType::AltAz,
            MountTypeDto::Equatorial => MountType::Equatorial,
            MountTypeDto::Unknown => MountType::Unknown,
        }
    }
}

/// Wire format for [`Mount`].
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MountDto {
    name: String,
    mount_type: MountTypeDto,
    capacity_lb: Option<f64>,
    max_slew_deg_s: Option<f64>,
    pointing_rms_arcsec: Option<f64>,
    /// `true`/`false` if known, otherwise omitted or `null`.
    #[serde(default)]
    non_sidereal_tracking: Option<bool>,
    source: String,
}

impl From<MountDto> for Mount {
    fn from(dto: MountDto) -> Self {
        Mount {
            name: dto.name,
            mount_type: dto.mount_type.into(),
            capacity_lb: dto.capacity_lb,
            max_slew_deg_s: dto.max_slew_deg_s,
            pointing_rms_arcsec: dto.pointing_rms_arcsec,
            non_sidereal_tracking: match dto.non_sidereal_tracking {
                Some(true) => Capability::Yes,
                Some(false) => Capability::No,
                None => Capability::Unknown,
            },
            source: dto.source,
        }
    }
}
