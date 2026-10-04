//! Wire format for [`super::camera`] types, as read from a preset config file.

use serde::Deserialize;

use super::camera::{Camera, Shutter};

/// Wire format for [`Shutter`].
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ShutterDto {
    Rolling { line_time_us: Option<f64> },
    Global,
}

impl From<ShutterDto> for Shutter {
    fn from(dto: ShutterDto) -> Self {
        match dto {
            ShutterDto::Rolling { line_time_us } => Shutter::Rolling { line_time_us },
            ShutterDto::Global => Shutter::Global,
        }
    }
}

/// Wire format for [`Camera`].
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CameraDto {
    name: String,
    pixel_um: f64,
    width_px: u32,
    height_px: u32,
    read_noise_e: Option<f64>,
    #[serde(default)]
    qe: Option<f64>,
    shutter: ShutterDto,
    weight_lb: Option<f64>,
    source: String,
}

impl From<CameraDto> for Camera {
    fn from(dto: CameraDto) -> Self {
        Camera {
            name: dto.name,
            pixel_um: dto.pixel_um,
            width_px: dto.width_px,
            height_px: dto.height_px,
            read_noise_e: dto.read_noise_e,
            qe: dto.qe,
            shutter: dto.shutter.into(),
            weight_lb: dto.weight_lb,
            source: dto.source,
        }
    }
}
