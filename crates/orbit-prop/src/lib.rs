//! Satellite propagation and ground-site observation geometry.
//!
//! Assessment-grade (about 0.01 deg); see README.md for accuracy and limits.

pub mod constants;
pub mod error;
pub mod time;

pub use error::OrbitPropError;
pub use time::{Epoch, UtcParts};
