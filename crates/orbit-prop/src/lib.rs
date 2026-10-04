//! Satellite propagation and ground-site observation geometry.
//!
//! Assessment-grade (about 0.01 deg); see README.md for accuracy and limits.

pub mod constants;
pub mod error;
pub mod frames;
pub mod site;
pub mod time;
// Some helpers are first used by observe and illumination (Tasks 5-6).
#[allow(dead_code)]
mod vec3;

pub use error::OrbitPropError;
pub use site::GroundSite;
pub use time::{Epoch, UtcParts};
