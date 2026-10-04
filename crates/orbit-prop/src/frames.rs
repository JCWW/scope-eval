//! Reference-frame conversions.
//!
//! **Accuracy: assessment grade, about 0.01 deg.** TEME is rotated to the
//! Earth-fixed frame by Greenwich mean sidereal time alone. Polar motion,
//! UT1 - UTC, and the equation of the equinoxes are ignored, and TEME is
//! treated as true-of-date. That is good enough for pass timing, rates and
//! lighting, and smaller than typical TLE error. Astrometric work needs the
//! upgrades listed under "Known limitations and future work" in the crate
//! README.
//!
//! Frames used:
//! * TEME: the inertial frame SGP4 outputs. x toward the mean equinox.
//! * ECEF: Earth-fixed, x through 0 deg latitude / 0 deg longitude.
//! * SEZ: topocentric south-east-zenith at a ground site.

use crate::constants::{EARTH_FLATTENING, EARTH_RADIUS_KM, EARTH_ROTATION_RAD_S};
use crate::site::GroundSite;
use crate::time::Epoch;
use crate::vec3::{rot_z, Vec3};

/// TEME position and velocity to ECEF.
// TODO(astrometric): add polar motion and use GCRS -> ITRS (IAU-2006/2000A).
pub fn teme_to_ecef(r_teme: Vec3, v_teme: Vec3, t: Epoch) -> (Vec3, Vec3) {
    let theta = t.gmst_rad();
    let r = rot_z(r_teme, theta);
    let v_rot = rot_z(v_teme, theta);
    // Subtract Earth rotation: v_ecef = R v_teme - omega x r_ecef.
    let w = EARTH_ROTATION_RAD_S;
    let v = [v_rot[0] + w * r[1], v_rot[1] - w * r[0], v_rot[2]];
    (r, v)
}

/// ECEF position and velocity to TEME (inverse of [`teme_to_ecef`]).
pub fn ecef_to_teme(r_ecef: Vec3, v_ecef: Vec3, t: Epoch) -> (Vec3, Vec3) {
    let w = EARTH_ROTATION_RAD_S;
    let v_rot = [v_ecef[0] - w * r_ecef[1], v_ecef[1] + w * r_ecef[0], v_ecef[2]];
    let theta = t.gmst_rad();
    (rot_z(r_ecef, -theta), rot_z(v_rot, -theta))
}

fn eccentricity_squared() -> f64 {
    EARTH_FLATTENING * (2.0 - EARTH_FLATTENING)
}

/// Geodetic site to ECEF position, km (WGS-84).
pub fn site_ecef(site: &GroundSite) -> Vec3 {
    let (lat, lon) = (site.lat_deg.to_radians(), site.lon_deg.to_radians());
    let h = site.alt_m / 1000.0;
    let e2 = eccentricity_squared();
    let n = EARTH_RADIUS_KM / (1.0 - e2 * lat.sin().powi(2)).sqrt();
    [
        (n + h) * lat.cos() * lon.cos(),
        (n + h) * lat.cos() * lon.sin(),
        (n * (1.0 - e2) + h) * lat.sin(),
    ]
}

/// ECEF position, km, to geodetic latitude and longitude (deg) and altitude (m).
/// Iterative; converges to well under a millimetre in a few steps.
pub fn ecef_to_geodetic(r: Vec3) -> (f64, f64, f64) {
    let e2 = eccentricity_squared();
    let p = (r[0] * r[0] + r[1] * r[1]).sqrt();
    let lon = r[1].atan2(r[0]);
    let mut lat = r[2].atan2(p * (1.0 - e2));
    let mut h = 0.0;
    for _ in 0..10 {
        let n = EARTH_RADIUS_KM / (1.0 - e2 * lat.sin().powi(2)).sqrt();
        h = if lat.cos().abs() > 1e-10 { p / lat.cos() - n } else { r[2].abs() - n * (1.0 - e2) };
        lat = r[2].atan2(p * (1.0 - e2 * n / (n + h)));
    }
    (lat.to_degrees(), lon.to_degrees(), h * 1000.0)
}

/// Rotate an ECEF vector into the site's south-east-zenith frame.
/// Uses geodetic latitude, so zenith is along the local vertical.
pub fn ecef_to_sez(v: Vec3, site: &GroundSite) -> Vec3 {
    let (lat, lon) = (site.lat_deg.to_radians(), site.lon_deg.to_radians());
    let (sl, cl) = lat.sin_cos();
    let (so, co) = lon.sin_cos();
    [
        sl * co * v[0] + sl * so * v[1] - cl * v[2],
        -so * v[0] + co * v[1],
        cl * co * v[0] + cl * so * v[1] + sl * v[2],
    ]
}

/// Azimuth (deg, from north through east, `[0, 360)`) and elevation (deg)
/// of a south-east-zenith vector.
pub fn sez_to_az_el(sez: Vec3) -> (f64, f64) {
    let north = -sez[0];
    let east = sez[1];
    let horizontal = (north * north + east * east).sqrt();
    let az = east.atan2(north).to_degrees().rem_euclid(360.0);
    let el = sez[2].atan2(horizontal).to_degrees();
    (az, el)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vec3::{norm, sub};

    #[test]
    fn site_ecef_matches_vallado_example_3_3() {
        // Vallado Example 3-3: lat 39.007 deg, lon -104.883 deg, alt 2187 m
        // -> r = (-1275.1219, -4797.9890, 3994.2975) km.
        let s = GroundSite::new(39.007, -104.883, 2187.0).unwrap();
        let r = site_ecef(&s);
        let expected = [-1275.1219, -4797.9890, 3994.2975];
        assert!(norm(sub(r, expected)) < 0.001, "{r:?}");
    }

    #[test]
    fn geodetic_round_trip() {
        for (lat, lon, alt) in [(39.007, -104.883, 2194.56), (-33.9, 18.4, 10.0), (89.99, 45.0, 0.0), (0.0, 180.0, 5000.0)] {
            let s = GroundSite::new(lat, lon, alt).unwrap();
            let (la, lo, h) = ecef_to_geodetic(site_ecef(&s));
            assert!((la - lat).abs() < 1e-9 && (h - alt).abs() < 1e-3, "{lat} {lon} {alt} -> {la} {lo} {h}");
            assert!(((lo - s.lon_deg + 540.0).rem_euclid(360.0) - 180.0).abs() < 1e-9);
        }
    }

    #[test]
    fn teme_ecef_round_trip() {
        let t = Epoch::from_utc(2026, 10, 4, 3, 0, 0.0).unwrap();
        let (r, v) = ([7000.0, -1200.0, 300.0], [1.0, 7.0, 2.0]);
        let (re, ve) = teme_to_ecef(r, v, t);
        let (r2, v2) = ecef_to_teme(re, ve, t);
        assert!(norm(sub(r, r2)) < 1e-9 && norm(sub(v, v2)) < 1e-12);
    }

    #[test]
    fn earth_fixed_point_has_zero_ecef_velocity() {
        // A point riding with the Earth: TEME velocity = omega x r.
        let t = Epoch::from_utc(2026, 10, 4, 3, 0, 0.0).unwrap();
        let r = [6378.137, 0.0, 0.0];
        let (rt, vt) = ecef_to_teme(r, [0.0; 3], t);
        assert!((norm(vt) - EARTH_ROTATION_RAD_S * 6378.137).abs() < 1e-12);
        let (_, ve) = teme_to_ecef(rt, vt, t);
        assert!(norm(ve) < 1e-12);
    }

    #[test]
    fn az_el_of_cardinal_directions() {
        let s = GroundSite::new(0.0, 0.0, 0.0).unwrap();
        let site = site_ecef(&s);
        let look = |target: Vec3| sez_to_az_el(ecef_to_sez(sub(target, site), &s));
        let (az, el) = look([6378.137, 0.0, 1000.0]); // north, on the horizon
        assert!(az.abs() < 1e-9 && el.abs() < 1e-9);
        let (az, el) = look([6378.137, 1000.0, 0.0]); // east, on the horizon
        assert!((az - 90.0).abs() < 1e-9 && el.abs() < 1e-9);
        let (_, el) = look([6878.137, 0.0, 0.0]); // straight up
        assert!((el - 90.0).abs() < 1e-9);
    }
}
