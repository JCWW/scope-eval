//! Camera sensor and shutter characteristics.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shutter {
    /// Rows are read one after another. `line_time_us` is the delay between rows.
    Rolling { line_time_us: Option<f64> },
    /// All rows are exposed at the same instant.
    Global,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Camera {
    pub name: String,
    /// Pixel pitch, micrometers.
    pub pixel_um: f64,
    pub width_px: u32,
    pub height_px: u32,
    /// Read noise per pixel readout, electrons RMS. `None` means not entered.
    /// Used by the detection check in `regimes.rs`.
    pub read_noise_e: Option<f64>,
    /// Peak quantum efficiency, as a fraction from 0 to 1. `None` means not entered.
    pub qe: Option<f64>,
    /// Measured detector MTF at the Nyquist frequency (half a cycle per
    /// pixel), as a fraction from 0 to 1. It includes the pixel aperture, so
    /// an ideal square pixel reads 2/pi = 0.64 and anything lower is charge
    /// diffusion or crosstalk. `None` means not entered: the point spread
    /// function then counts the pixel aperture only. See `psf.rs`.
    pub mtf_nyquist: Option<f64>,
    pub shutter: Shutter,
    /// Camera weight, lb.
    pub weight_lb: Option<f64>,
    pub source: String,
}

impl Camera {
    pub fn width_mm(&self) -> f64 {
        self.width_px as f64 * self.pixel_um / 1000.0
    }
    pub fn height_mm(&self) -> f64 {
        self.height_px as f64 * self.pixel_um / 1000.0
    }
    pub fn diagonal_mm(&self) -> f64 {
        self.width_mm().hypot(self.height_mm())
    }
}
