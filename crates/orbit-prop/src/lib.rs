//! Satellite propagation and ground-site observation geometry.
//!
//! Assessment-grade (about 0.01 deg); see README.md for accuracy and limits.

pub mod constants;
pub mod error;
pub mod frames;
pub mod keplerian;
pub mod observe;
pub mod propagator;
pub mod sgp4_propagator;
pub mod site;
pub mod state;
pub mod time;
pub mod tle;
// Some helpers are first used by observe and illumination (Tasks 5-6).
#[allow(dead_code)]
mod vec3;

pub use error::OrbitPropError;
pub use keplerian::{KeplerElements, KeplerJ2};
pub use observe::{observe, Observation};
pub use propagator::Propagator;
pub use sgp4_propagator::Sgp4Propagator;
pub use site::GroundSite;
pub use state::StateVector;
pub use time::{Epoch, UtcParts};
pub use tle::Tle;
