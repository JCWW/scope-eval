//! Site and observing assumptions.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Site {
    /// Typical atmospheric seeing, full width at half maximum, arcseconds.
    pub seeing_arcsec: f64,
    /// Reference wavelength for the focus calculation, micrometers.
    pub wavelength_um: f64,
}
