//! Lighting: Earth's shadow, phase angle, and how dark the site is.

use crate::constants::{EARTH_RADIUS_KM, SUN_RADIUS_KM};
use crate::frames::{ecef_to_sez, sez_to_az_el, site_ecef, teme_to_ecef};
use crate::site::GroundSite;
use crate::sun_moon::sun_position_km;
use crate::time::Epoch;
use crate::vec3::{angle_between, dot, norm, scale, sub, Vec3};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lighting {
    Sunlit,
    /// Partly shadowed: the Earth covers part of the Sun's disk.
    Penumbra,
    /// Fully shadowed.
    Umbra,
}

/// Shadow state of a satellite at `sat_r` (km, geocentric) with the Sun at
/// `sun_r` (km, geocentric), using a conical shadow model (Vallado
/// algorithm 34). Atmospheric refraction into the shadow is ignored.
pub fn lighting(sat_r: Vec3, sun_r: Vec3) -> Lighting {
    let sun_dist = norm(sun_r);
    let sun_hat = scale(sun_r, 1.0 / sun_dist);
    let along = dot(sat_r, sun_hat);
    if along >= 0.0 {
        return Lighting::Sunlit; // on the day side of the terminator plane
    }
    let behind = -along; // distance behind the Earth along the shadow axis
    let off_axis = norm(sub(sat_r, scale(sun_hat, along)));
    let alpha_umbra = ((SUN_RADIUS_KM - EARTH_RADIUS_KM) / sun_dist).asin();
    let alpha_penumbra = ((SUN_RADIUS_KM + EARTH_RADIUS_KM) / sun_dist).asin();
    let umbra_radius = EARTH_RADIUS_KM - behind * alpha_umbra.tan();
    let penumbra_radius = EARTH_RADIUS_KM + behind * alpha_penumbra.tan();
    if off_axis < umbra_radius {
        Lighting::Umbra
    } else if off_axis < penumbra_radius {
        Lighting::Penumbra
    } else {
        Lighting::Sunlit
    }
}

/// Sun-satellite-observer angle, degrees. 0 is fully lit as seen from the
/// site (Sun behind the observer); 180 is looking at the unlit side.
/// All vectors geocentric, km, same frame.
pub fn phase_angle_deg(sat_r: Vec3, observer_r: Vec3, sun_r: Vec3) -> f64 {
    angle_between(sub(sun_r, sat_r), sub(observer_r, sat_r)).to_degrees()
}

/// Elevation of the Sun's centre above the site's horizon, degrees.
pub fn sun_elevation_deg(site: &GroundSite, t: Epoch) -> f64 {
    let (sun_ecef, _) = teme_to_ecef(sun_position_km(t), [0.0; 3], t);
    let (_, el) = sez_to_az_el(ecef_to_sez(sub(sun_ecef, site_ecef(site)), site));
    el
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::AU_KM;

    const SUN: Vec3 = [AU_KM, 0.0, 0.0];

    #[test]
    fn shadow_classification() {
        assert_eq!(lighting([7000.0, 0.0, 0.0], SUN), Lighting::Sunlit, "day side");
        assert_eq!(lighting([-7000.0, 0.0, 0.0], SUN), Lighting::Umbra, "directly behind Earth");
        assert_eq!(lighting([-7000.0, 7000.0, 0.0], SUN), Lighting::Sunlit, "beside Earth, outside the shadow");
        // GEO near equinox midnight: 42164 km behind, on the axis -> umbra.
        assert_eq!(lighting([-42_164.0, 0.0, 0.0], SUN), Lighting::Umbra);
        // Just outside the umbra edge at GEO distance but inside the penumbra.
        let alpha_u = ((SUN_RADIUS_KM - EARTH_RADIUS_KM) / AU_KM).asin();
        let edge = EARTH_RADIUS_KM - 42_164.0 * alpha_u.tan();
        assert_eq!(lighting([-42_164.0, edge + 20.0, 0.0], SUN), Lighting::Penumbra);
    }

    #[test]
    fn phase_angle_extremes() {
        let observer = [6378.0, 0.0, 0.0];
        assert!(phase_angle_deg([7000.0, 0.0, 0.0], observer, SUN) > 179.9, "sat between observer and Sun");
        assert!(phase_angle_deg([5000.0, 0.0, 0.0], observer, SUN) < 0.1, "observer between sat and Sun");
        assert!((phase_angle_deg([6378.0, 1000.0, 0.0], observer, SUN) - 90.0).abs() < 0.1);
    }

    #[test]
    fn sun_is_up_at_noon_and_down_at_midnight() {
        // Greenwich on the 2026 June solstice: high at noon, well below at midnight.
        let site = GroundSite::new(51.48, 0.0, 0.0).unwrap();
        let noon = sun_elevation_deg(&site, Epoch::from_utc(2026, 6, 21, 12, 0, 0.0).unwrap());
        let midnight = sun_elevation_deg(&site, Epoch::from_utc(2026, 6, 21, 0, 0, 0.0).unwrap());
        // Expected noon elevation 90 - 51.48 + 23.44 = 61.96 deg; midnight -(51.48 - ... ) about -15 deg.
        assert!((noon - 61.96).abs() < 0.3, "noon {noon}");
        assert!((midnight - (-15.1)).abs() < 0.5, "midnight {midnight}");
    }
}
