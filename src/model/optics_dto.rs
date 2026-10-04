//! Wire format for [`super::optics`] types, as read from a preset config file.

use serde::Deserialize;

use super::optics::{Obstruction, SpotConvention, SpotPoint, SpotSpec, Telescope};

/// Wire format for [`Obstruction`].
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ObstructionDto {
    ByDiameter(f64),
    ByArea(f64),
    None,
}

impl From<ObstructionDto> for Obstruction {
    fn from(dto: ObstructionDto) -> Self {
        match dto {
            ObstructionDto::ByDiameter(v) => Obstruction::ByDiameter(v),
            ObstructionDto::ByArea(v) => Obstruction::ByArea(v),
            ObstructionDto::None => Obstruction::None,
        }
    }
}

/// Wire format for [`SpotConvention`].
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SpotConventionDto {
    RmsRadius,
    RmsDiameter,
    Unknown,
}

impl From<SpotConventionDto> for SpotConvention {
    fn from(dto: SpotConventionDto) -> Self {
        match dto {
            SpotConventionDto::RmsRadius => SpotConvention::RmsRadius,
            SpotConventionDto::RmsDiameter => SpotConvention::RmsDiameter,
            SpotConventionDto::Unknown => SpotConvention::Unknown,
        }
    }
}

/// Wire format for [`SpotPoint`].
#[derive(Debug, Clone, Copy, Deserialize)]
pub(crate) struct SpotPointDto {
    field_radius_mm: f64,
    rms_um: f64,
}

impl From<SpotPointDto> for SpotPoint {
    fn from(dto: SpotPointDto) -> Self {
        SpotPoint { field_radius_mm: dto.field_radius_mm, rms_um: dto.rms_um }
    }
}

/// Wire format for [`SpotSpec`].
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SpotSpecDto {
    convention: SpotConventionDto,
    points: Vec<SpotPointDto>,
}

impl From<SpotSpecDto> for SpotSpec {
    fn from(dto: SpotSpecDto) -> Self {
        SpotSpec::new(dto.convention.into(), dto.points.into_iter().map(Into::into).collect())
    }
}

/// Wire format for [`Telescope`].
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct TelescopeDto {
    name: String,
    aperture_mm: f64,
    focal_length_mm: f64,
    obstruction: ObstructionDto,
    image_circle_mm: f64,
    back_focus_mm: Option<f64>,
    weight_lb: Option<f64>,
    spot: Option<SpotSpecDto>,
    source: String,
}

impl From<TelescopeDto> for Telescope {
    fn from(dto: TelescopeDto) -> Self {
        Telescope {
            name: dto.name,
            aperture_mm: dto.aperture_mm,
            focal_length_mm: dto.focal_length_mm,
            obstruction: dto.obstruction.into(),
            image_circle_mm: dto.image_circle_mm,
            back_focus_mm: dto.back_focus_mm,
            weight_lb: dto.weight_lb,
            spot: dto.spot.map(Into::into),
            source: dto.source,
        }
    }
}
