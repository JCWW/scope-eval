//! Telescope optics: aperture, obstruction, and spot-size data.

/// How a spec sheet quotes the central obstruction.
///
/// Vendors quote this two different ways, and mixing them up is the most
/// common spec-sheet error. A 56% obstruction *by diameter* blocks only
/// 0.56^2 = 31% of the light.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Obstruction {
    /// Fraction of the aperture diameter that is blocked (0.56 for "56% by diameter").
    ByDiameter(f64),
    /// Fraction of the aperture area that is blocked (0.237 for "23.7% by area").
    ByArea(f64),
    /// No central obstruction (for example a refractor).
    None,
}

impl Obstruction {
    /// Fraction of the collecting area that is blocked.
    pub fn area_fraction(&self) -> f64 {
        match *self {
            Obstruction::ByDiameter(d) => d * d,
            Obstruction::ByArea(a) => a,
            Obstruction::None => 0.0,
        }
    }

    /// Human-readable description of how the obstruction was quoted.
    pub fn describe(&self) -> String {
        match *self {
            Obstruction::ByDiameter(d) => format!(
                "{:.1}% by diameter -> {:.1}% by area",
                d * 100.0,
                d * d * 100.0
            ),
            Obstruction::ByArea(a) => format!(
                "{:.1}% by area (= {:.1}% by diameter)",
                a * 100.0,
                a.sqrt() * 100.0
            ),
            Obstruction::None => "none".to_string(),
        }
    }
}

/// Whether a vendor's RMS spot figure is a radius or a diameter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpotConvention {
    RmsRadius,
    RmsDiameter,
    /// The spec sheet does not say. The tool evaluates both readings.
    Unknown,
}

/// One quoted spot-size value: the RMS spot at a given distance from the optical axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpotPoint {
    /// Distance from the center of the field on the focal plane, mm (0 = on-axis).
    pub field_radius_mm: f64,
    /// Quoted RMS spot size, micrometers.
    pub rms_um: f64,
}

/// The optical spot-size data from a spec sheet.
#[derive(Debug, Clone, PartialEq)]
pub struct SpotSpec {
    pub convention: SpotConvention,
    /// Always kept sorted by field radius, ascending.
    pub points: Vec<SpotPoint>,
}

impl SpotSpec {
    pub fn new(convention: SpotConvention, mut points: Vec<SpotPoint>) -> Self {
        points.sort_by(|a, b| a.field_radius_mm.partial_cmp(&b.field_radius_mm).unwrap());
        SpotSpec { convention, points }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Telescope {
    pub name: String,
    /// Clear aperture D, mm.
    pub aperture_mm: f64,
    /// Effective focal length, mm.
    pub focal_length_mm: f64,
    pub obstruction: Obstruction,
    /// Diameter of the corrected (sharp, flat) image circle, mm.
    pub image_circle_mm: f64,
    /// Back focus available, mm (how the vendor measures it varies, see README).
    pub back_focus_mm: Option<f64>,
    /// Optical tube weight, lb.
    pub weight_lb: Option<f64>,
    pub spot: Option<SpotSpec>,
    /// Where the preset numbers came from, and any caveats.
    pub source: String,
}

impl Telescope {
    /// Focal ratio N = focal length / aperture.
    pub fn f_ratio(&self) -> f64 {
        self.focal_length_mm / self.aperture_mm
    }
}
