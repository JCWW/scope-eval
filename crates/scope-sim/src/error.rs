use std::fmt;

use orbit_prop::OrbitPropError;

/// Anything that stops a simulation from being set up or advanced. The
/// message is written for the person at the dashboard, not for a log.
#[derive(Debug, Clone, PartialEq)]
pub struct SimError(pub String);

impl SimError {
    pub fn new(message: impl Into<String>) -> SimError {
        SimError(message.into())
    }
}

impl fmt::Display for SimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SimError {}

impl From<OrbitPropError> for SimError {
    fn from(e: OrbitPropError) -> SimError {
        SimError(e.to_string())
    }
}
