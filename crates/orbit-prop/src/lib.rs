// The crate documentation is the README, so its example is compiled and run by `cargo test`.
#![doc = include_str!("../README.md")]

pub mod constants;
pub mod error;
pub mod frames;
pub mod illumination;
pub mod keplerian;
pub mod observe;
pub mod passes;
pub mod propagator;
pub mod sgp4_propagator;
pub mod site;
pub mod state;
pub mod sun_moon;
pub mod time;
pub mod tle;
mod vec3;

pub use error::OrbitPropError;
pub use illumination::Lighting;
pub use keplerian::{KeplerElements, KeplerJ2};
pub use observe::{observe, Observation};
pub use passes::{find_passes, Pass, PassDarkness, PassLighting, PassResult, PassSearch};
pub use propagator::Propagator;
pub use sgp4_propagator::Sgp4Propagator;
pub use site::GroundSite;
pub use state::StateVector;
pub use time::{Epoch, UtcParts};
pub use tle::Tle;
