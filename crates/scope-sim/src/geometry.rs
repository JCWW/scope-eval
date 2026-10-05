//! Directions on the sky and the two kinds of mount axes.
//!
//! Directions are unit vectors in the site's local east-north-up (ENU)
//! frame. Azimuth runs from north through east, as in `orbit-prop`.
//!
//! The camera frame is the tangent plane at the boresight, with x along
//! increasing axis 1 (azimuth, or hour angle) and y along increasing axis 2
//! (elevation, or declination). The sensor's long side is assumed to lie
//! along x.

use serde::{Deserialize, Serialize};

pub type Vec3 = [f64; 3];

/// How the mount's two axes are arranged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MountKind {
    /// Axis 1 is azimuth, axis 2 is elevation. Keyhole at the zenith.
    AltAz,
    /// Axis 1 is hour angle (increasing westward), axis 2 is declination.
    /// Keyhole at the celestial pole.
    Equatorial,
}

pub fn dot(a: Vec3, b: Vec3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn normalize(a: Vec3) -> Vec3 {
    let n = dot(a, a).sqrt();
    [a[0] / n, a[1] / n, a[2] / n]
}

/// Angle between two directions, radians. Stable for tiny angles, where
/// `acos(dot)` loses precision.
pub fn angle_between(a: Vec3, b: Vec3) -> f64 {
    let c = [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    dot(c, c).sqrt().atan2(dot(a, b))
}

/// Wrap an angle into `[-180, 180)` degrees.
pub fn wrap180(deg: f64) -> f64 {
    (deg + 180.0).rem_euclid(360.0) - 180.0
}

pub fn enu_from_az_el(az_deg: f64, el_deg: f64) -> Vec3 {
    let (a, e) = (az_deg.to_radians(), el_deg.to_radians());
    [e.cos() * a.sin(), e.cos() * a.cos(), e.sin()]
}

pub fn az_el_from_enu(v: Vec3) -> (f64, f64) {
    let az = v[0].atan2(v[1]).to_degrees().rem_euclid(360.0);
    let el = v[2].atan2(v[0].hypot(v[1])).to_degrees();
    (az, el)
}

/// Direction of hour angle `ha_deg` and declination `dec_deg` from a site at
/// geodetic latitude `lat_deg`.
pub fn enu_from_ha_dec(ha_deg: f64, dec_deg: f64, lat_deg: f64) -> Vec3 {
    let (h, d, p) = (ha_deg.to_radians(), dec_deg.to_radians(), lat_deg.to_radians());
    [
        -d.cos() * h.sin(),
        p.cos() * d.sin() - p.sin() * d.cos() * h.cos(),
        p.sin() * d.sin() + p.cos() * d.cos() * h.cos(),
    ]
}

/// Inverse of [`enu_from_ha_dec`]. Hour angle in `[-180, 180)`.
pub fn ha_dec_from_enu(v: Vec3, lat_deg: f64) -> (f64, f64) {
    let p = lat_deg.to_radians();
    let sin_dec = (p.cos() * v[1] + p.sin() * v[2]).clamp(-1.0, 1.0);
    let ha = (-v[0]).atan2(p.cos() * v[2] - p.sin() * v[1]).to_degrees();
    (wrap180(ha), sin_dec.asin().to_degrees())
}

/// Mount axis angles (degrees) that point along `v`.
pub fn axes_from_enu(kind: MountKind, v: Vec3, lat_deg: f64) -> (f64, f64) {
    match kind {
        MountKind::AltAz => az_el_from_enu(v),
        MountKind::Equatorial => ha_dec_from_enu(v, lat_deg),
    }
}

/// Direction the mount points with its axes at `a1`, `a2` (degrees).
pub fn enu_from_axes(kind: MountKind, a1: f64, a2: f64, lat_deg: f64) -> Vec3 {
    match kind {
        MountKind::AltAz => enu_from_az_el(a1, a2),
        MountKind::Equatorial => enu_from_ha_dec(a1, a2, lat_deg),
    }
}

/// Unit vectors along increasing axis 1 and increasing axis 2 at the given
/// pointing: the camera's x and y directions. Both stay well defined at the
/// keyhole, where the axis-1 direction is set by the axis angle alone.
pub fn tangent_basis(kind: MountKind, a1: f64, a2: f64, lat_deg: f64) -> (Vec3, Vec3) {
    match kind {
        MountKind::AltAz => {
            let (a, e) = (a1.to_radians(), a2.to_radians());
            (
                [a.cos(), -a.sin(), 0.0],
                [-e.sin() * a.sin(), -e.sin() * a.cos(), e.cos()],
            )
        }
        MountKind::Equatorial => {
            let (h, d, p) = (a1.to_radians(), a2.to_radians(), lat_deg.to_radians());
            (
                [-h.cos(), p.sin() * h.sin(), -p.cos() * h.sin()],
                [
                    d.sin() * h.sin(),
                    p.cos() * d.cos() + p.sin() * d.sin() * h.cos(),
                    p.sin() * d.cos() - p.cos() * d.sin() * h.cos(),
                ],
            )
        }
    }
}

/// Where `target` appears in the camera frame at `boresight`, radians,
/// along x and y. Equal to the gnomonic (flat-sensor) projection for small
/// offsets, and still finite for a target far off or behind the boresight.
pub fn camera_offset(boresight: Vec3, x: Vec3, y: Vec3, target: Vec3) -> (f64, f64) {
    let along = dot(target, boresight);
    (dot(target, x).atan2(along), dot(target, y).atan2(along))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAT: f64 = 40.0;

    fn close(a: Vec3, b: Vec3) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 1e-12)
    }

    #[test]
    fn az_el_round_trip_and_cardinal_directions() {
        assert!(close(enu_from_az_el(0.0, 0.0), [0.0, 1.0, 0.0]));
        assert!(close(enu_from_az_el(90.0, 0.0), [1.0, 0.0, 0.0]));
        let (az, el) = az_el_from_enu(enu_from_az_el(231.5, 37.25));
        assert!((az - 231.5).abs() < 1e-9 && (el - 37.25).abs() < 1e-9);
    }

    #[test]
    fn ha_dec_matches_known_directions() {
        // On the meridian at dec = latitude is the zenith.
        assert!(close(enu_from_ha_dec(0.0, LAT, LAT), [0.0, 0.0, 1.0]));
        // Dec 90 is the celestial pole: due north at elevation = latitude.
        let (az, el) = az_el_from_enu(enu_from_ha_dec(0.0, 90.0, LAT));
        assert!(az.abs() < 1e-9 && (el - LAT).abs() < 1e-9);
        // Positive hour angle is west of the meridian.
        assert!(enu_from_ha_dec(30.0, 0.0, LAT)[0] < 0.0);
    }

    #[test]
    fn ha_dec_round_trip() {
        for &(ha, dec) in &[(0.0, 0.0), (-75.0, 12.0), (150.0, -30.0), (10.0, 85.0)] {
            let (h, d) = ha_dec_from_enu(enu_from_ha_dec(ha, dec, LAT), LAT);
            assert!((h - ha).abs() < 1e-9 && (d - dec).abs() < 1e-9, "{ha} {dec} -> {h} {d}");
        }
    }

    #[test]
    fn tangent_basis_is_orthonormal_and_follows_the_axes() {
        for kind in [MountKind::AltAz, MountKind::Equatorial] {
            let (a1, a2) = (37.0, 52.0);
            let b = enu_from_axes(kind, a1, a2, LAT);
            let (x, y) = tangent_basis(kind, a1, a2, LAT);
            for (u, v) in [(b, x), (b, y), (x, y)] {
                assert!(dot(u, v).abs() < 1e-12);
            }
            for u in [x, y] {
                assert!((dot(u, u) - 1.0).abs() < 1e-12);
            }
            // A small step in each axis moves the boresight along x or y.
            let h = 1e-6;
            let (dx, _) = camera_offset(b, x, y, enu_from_axes(kind, a1 + h, a2, LAT));
            let (_, dy) = camera_offset(b, x, y, enu_from_axes(kind, a1, a2 + h, LAT));
            assert!(dx > 0.0 && dy > 0.0);
            assert!((dy - h.to_radians()).abs() < 1e-12);
        }
    }

    #[test]
    fn camera_offset_is_the_angle_for_small_offsets() {
        let b = enu_from_az_el(100.0, 30.0);
        let (x, y) = tangent_basis(MountKind::AltAz, 100.0, 30.0, LAT);
        let t = enu_from_az_el(100.0, 30.0 + 10.0 / 3600.0);
        let (ox, oy) = camera_offset(b, x, y, t);
        assert!(ox.abs() < 1e-12);
        assert!((oy.to_degrees() * 3600.0 - 10.0).abs() < 1e-6);
        assert!((angle_between(b, t).to_degrees() * 3600.0 - 10.0).abs() < 1e-6);
    }

    #[test]
    fn wrap180_range() {
        assert_eq!(wrap180(190.0), -170.0);
        assert_eq!(wrap180(-190.0), 170.0);
        assert_eq!(wrap180(180.0), -180.0);
    }
}
