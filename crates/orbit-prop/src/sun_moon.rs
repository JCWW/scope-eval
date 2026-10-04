//! Low-precision Sun and Moon positions.
//!
//! **Accuracy: assessment grade.** The Sun is good to about 0.01 deg and the
//! Moon to about 0.3 deg between 1950 and 2050, from the low-precision
//! formulas in the Astronomical Almanac (section C for the Sun, section D
//! for the Moon). That is plenty for shadow, phase angle, twilight and Moon
//! avoidance, but not for astrometry.
//!
//! Both return geocentric position vectors in km, in the mean-equator,
//! true-of-date frame, which is treated as TEME.
// TODO(astrometric): replace with a JPL ephemeris (e.g. DE440) in GCRS.

use crate::constants::{AU_KM, EARTH_RADIUS_KM, J2000_JD};
use crate::time::Epoch;
use crate::vec3::Vec3;

fn sin_d(deg: f64) -> f64 {
    deg.to_radians().sin()
}

fn cos_d(deg: f64) -> f64 {
    deg.to_radians().cos()
}

/// Ecliptic longitude and latitude (deg) and distance (km) to an equatorial vector.
fn ecliptic_to_equatorial(lon_deg: f64, lat_deg: f64, dist_km: f64, obliquity_deg: f64) -> Vec3 {
    let (x, y, z) = (cos_d(lat_deg) * cos_d(lon_deg), cos_d(lat_deg) * sin_d(lon_deg), sin_d(lat_deg));
    let (se, ce) = (sin_d(obliquity_deg), cos_d(obliquity_deg));
    [dist_km * x, dist_km * (ce * y - se * z), dist_km * (se * y + ce * z)]
}

/// Geocentric Sun position, km.
pub fn sun_position_km(t: Epoch) -> Vec3 {
    let n = t.days_since_j2000();
    let l = 280.460 + 0.985_647_4 * n;
    let g = 357.528 + 0.985_600_3 * n;
    let lambda = l + 1.915 * sin_d(g) + 0.020 * sin_d(2.0 * g);
    let eps = 23.439 - 0.000_000_4 * n;
    let r_au = 1.000_14 - 0.016_71 * cos_d(g) - 0.000_14 * cos_d(2.0 * g);
    ecliptic_to_equatorial(lambda, 0.0, r_au * AU_KM, eps)
}

/// Geocentric Moon position, km.
pub fn moon_position_km(t: Epoch) -> Vec3 {
    let tc = t.days_since_j2000() / 36_525.0;
    let lambda = 218.32 + 481_267.881 * tc + 6.29 * sin_d(135.0 + 477_198.87 * tc)
        - 1.27 * sin_d(259.3 - 413_335.36 * tc)
        + 0.66 * sin_d(235.7 + 890_534.22 * tc)
        + 0.21 * sin_d(269.9 + 954_397.74 * tc)
        - 0.19 * sin_d(357.5 + 35_999.05 * tc)
        - 0.11 * sin_d(186.5 + 966_404.03 * tc);
    let beta = 5.13 * sin_d(93.3 + 483_202.02 * tc) + 0.28 * sin_d(228.2 + 960_400.89 * tc)
        - 0.28 * sin_d(318.3 + 6_003.15 * tc)
        - 0.17 * sin_d(217.6 - 407_332.21 * tc);
    let parallax = 0.9508
        + 0.0518 * cos_d(135.0 + 477_198.87 * tc)
        + 0.0095 * cos_d(259.3 - 413_335.36 * tc)
        + 0.0078 * cos_d(235.7 + 890_534.22 * tc)
        + 0.0028 * cos_d(269.9 + 954_397.74 * tc);
    let dist = EARTH_RADIUS_KM / sin_d(parallax);
    let eps = 23.439 - 0.000_000_4 * (t.jd() - J2000_JD);
    ecliptic_to_equatorial(lambda, beta, dist, eps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vec3::norm;

    fn ra_dec(r: Vec3) -> (f64, f64) {
        (r[1].atan2(r[0]).to_degrees().rem_euclid(360.0), (r[2] / norm(r)).asin().to_degrees())
    }

    #[test]
    fn sun_matches_meeus_example_25a() {
        // Meeus, Astronomical Algorithms, Example 25.a: 1992 October 13.0 TD,
        // apparent RA 13h13m31.4s = 198.380833 deg, Dec -7d47m01s = -7.783611 deg,
        // distance 0.99760775 AU.
        let r = sun_position_km(Epoch::from_utc(1992, 10, 13, 0, 0, 0.0).unwrap());
        let (ra, dec) = ra_dec(r);
        assert!((ra - 198.380_833).abs() < 0.01, "ra {ra}");
        assert!((dec - (-7.783_611)).abs() < 0.01, "dec {dec}");
        assert!((norm(r) / AU_KM - 0.997_607_75).abs() < 1e-4);
    }

    #[test]
    fn moon_matches_meeus_example_47a() {
        // Meeus Example 47.a: 1992 April 12.0 TD, apparent RA 134.688470 deg,
        // Dec 13.768368 deg, distance 368409.7 km.
        let r = moon_position_km(Epoch::from_utc(1992, 4, 12, 0, 0, 0.0).unwrap());
        let (ra, dec) = ra_dec(r);
        assert!((ra - 134.688_470).abs() < 0.3, "ra {ra}");
        assert!((dec - 13.768_368).abs() < 0.3, "dec {dec}");
        assert!((norm(r) - 368_409.7).abs() < 1000.0, "dist {}", norm(r));
    }
}
