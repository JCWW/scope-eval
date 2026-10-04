//! Mount and payload information for the practical-fit check.

/// How the mount's axes are arranged.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MountType {
    /// Altitude-azimuth: one axis turns horizontally, one vertically. Cannot follow
    /// objects passing exactly overhead (the zenith "keyhole").
    AltAz,
    /// Equatorial: one axis parallel to Earth's axis. Its keyhole is near the celestial pole.
    Equatorial,
    Unknown,
}

/// A yes / no / not-sure answer for mount capabilities.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Capability {
    Yes,
    No,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mount {
    pub name: String,
    pub mount_type: MountType,
    /// Rated payload, lb.
    pub capacity_lb: Option<f64>,
    /// Maximum slew (and tracking) rate per axis, degrees per second.
    pub max_slew_deg_s: Option<f64>,
    /// Pointing accuracy after a pointing model, arcsec RMS.
    pub pointing_rms_arcsec: Option<f64>,
    /// Can the control software track at arbitrary (non-sidereal) rates, e.g. from a TLE?
    pub non_sidereal_tracking: Capability,
    pub source: String,
}

/// Mount and optical-train information for the practical-fit check.
#[derive(Debug, Clone, PartialEq)]
pub struct Payload {
    pub mount: Option<Mount>,
    /// Focuser, dew heaters, cables, filter wheel, dovetail, and so on.
    pub accessories_lb: f64,
    /// Back focus the camera train needs, mm.
    pub back_focus_required_mm: Option<f64>,
}

impl Payload {
    pub fn mount_capacity_lb(&self) -> Option<f64> {
        self.mount.as_ref().and_then(|m| m.capacity_lb)
    }
}
