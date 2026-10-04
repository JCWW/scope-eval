//! Core data types, organized by domain: optics, camera, site, mount, and a
//! full configuration tying them together.
//!
//! These are plain data. All of the math lives in `checks.rs`.

pub mod camera;
pub(crate) mod camera_dto;
pub mod config;
pub mod mount;
pub(crate) mod mount_dto;
pub mod optics;
pub(crate) mod optics_dto;
pub mod site;

pub use camera::{Camera, Shutter};
pub use config::Config;
pub use mount::{Capability, Mount, MountType, Payload};
pub use optics::{Obstruction, SpotConvention, SpotPoint, SpotSpec, Telescope};
pub use site::Site;
