//! Site and observing assumptions.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Site {
    /// Typical atmospheric seeing, full width at half maximum, arcseconds.
    pub seeing_arcsec: f64,
    /// Reference wavelength for the focus calculation, micrometers.
    pub wavelength_um: f64,
    /// Sky background surface brightness, V magnitudes per square arcsecond.
    /// `None` means not entered: the photometry model substitutes a default
    /// and reports that it did. Roughly 21.9 at a dark rural site, 21.0
    /// rural, 18.5 suburban. Larger numbers are darker.
    pub sky_mag_arcsec2: Option<f64>,
}
