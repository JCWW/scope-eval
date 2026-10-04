//! Domain-focused calculators for the physical calculations used by the
//! evaluation rules.
//!
//! Keep equations here and keep PASS/WARN/FAIL decisions in `checks` and
//! `regimes`, so each calculation can be verified independently.

pub mod camera;
pub mod detection;
pub mod mount;
pub mod optics;
pub mod orbit;
