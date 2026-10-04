//! What a ground site sees of a satellite at one instant.
//!
//! All angular rates are computed analytically from the relative position
//! and velocity, not by differencing, so they are exact for the given state.

use crate::constants::EARTH_ROTATION_RAD_S;
use crate::frames::{ecef_to_sez, ecef_to_teme, sez_to_az_el, site_ecef, teme_to_ecef};
use crate::site::GroundSite;
use crate::state::StateVector;
use crate::time::Epoch;
use crate::vec3::{dot, norm, sub, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Observation {
    pub epoch: Epoch,
    /// Azimuth from north through east, `[0, 360)`.
    pub az_deg: f64,
    pub el_deg: f64,
    pub range_km: f64,
    /// Positive when the satellite is moving away.
    pub range_rate_km_s: f64,
    /// Alt-az mount axis rates. The azimuth rate grows without bound as the
    /// satellite approaches the zenith (the alt-az keyhole).
    pub az_rate_deg_s: f64,
    pub el_rate_deg_s: f64,
    /// Topocentric right ascension and declination, true-of-date.
    // TODO(astrometric): aberration, light time, and GCRS rather than TEME.
    pub ra_deg: f64,
    pub dec_deg: f64,
    /// Equatorial mount axis rates (hour angle increases westward).
    pub ha_rate_deg_s: f64,
    pub dec_rate_deg_s: f64,
    /// Total angular rate an Earth-fixed mount must follow.
    pub rate_vs_ground_deg_s: f64,
    /// Total angular rate against the background stars.
    pub rate_vs_stars_deg_s: f64,
}

/// Rate of change of `atan2(y, x)`, rad/s.
fn atan2_rate(y: f64, x: f64, y_dot: f64, x_dot: f64) -> f64 {
    let d = x * x + y * y;
    if d < 1e-18 {
        0.0
    } else {
        (x * y_dot - y * x_dot) / d
    }
}

/// Angular rate of a line of sight `rho` changing at `rho_dot`, rad/s:
/// the component of `rho_dot` perpendicular to `rho`, divided by `|rho|`.
fn transverse_rate(rho: Vec3, rho_dot: Vec3) -> f64 {
    let r = norm(rho);
    let radial = dot(rho, rho_dot) / r;
    (dot(rho_dot, rho_dot) - radial * radial).max(0.0).sqrt() / r
}

pub fn observe(state: &StateVector, site: &GroundSite) -> Observation {
    let t = state.epoch;
    // Earth-fixed geometry: what the mount follows.
    let (r_ecef, v_ecef) = teme_to_ecef(state.r_km, state.v_km_s, t);
    let site_r = site_ecef(site);
    let rho = sub(r_ecef, site_r);
    let sez = ecef_to_sez(rho, site);
    let sez_dot = ecef_to_sez(v_ecef, site);
    let range = norm(rho);
    let range_rate = dot(rho, v_ecef) / range;
    let (az, el) = sez_to_az_el(sez);
    let (north, east, up) = (-sez[0], sez[1], sez[2]);
    let (north_dot, east_dot, up_dot) = (-sez_dot[0], sez_dot[1], sez_dot[2]);
    let horizontal = (north * north + east * east).sqrt();
    let horizontal_dot = if horizontal > 1e-9 { (north * north_dot + east * east_dot) / horizontal } else { 0.0 };
    let az_rate = atan2_rate(east, north, east_dot, north_dot);
    let el_rate = atan2_rate(up, horizontal, up_dot, horizontal_dot);

    // Inertial geometry: motion against the stars.
    let (site_r_teme, site_v_teme) = ecef_to_teme(site_r, [0.0; 3], t);
    let rho_i = sub(state.r_km, site_r_teme);
    let rho_i_dot = sub(state.v_km_s, site_v_teme);
    let (x, y, z) = (rho_i[0], rho_i[1], rho_i[2]);
    let (xd, yd, zd) = (rho_i_dot[0], rho_i_dot[1], rho_i_dot[2]);
    let p = (x * x + y * y).sqrt();
    let p_dot = if p > 1e-9 { (x * xd + y * yd) / p } else { 0.0 };
    let ra_rate = atan2_rate(y, x, yd, xd);
    let dec_rate = atan2_rate(z, p, zd, p_dot);

    Observation {
        epoch: t,
        az_deg: az,
        el_deg: el,
        range_km: range,
        range_rate_km_s: range_rate,
        az_rate_deg_s: az_rate.to_degrees(),
        el_rate_deg_s: el_rate.to_degrees(),
        ra_deg: y.atan2(x).to_degrees().rem_euclid(360.0),
        dec_deg: z.atan2(p).to_degrees(),
        ha_rate_deg_s: (EARTH_ROTATION_RAD_S - ra_rate).to_degrees(),
        dec_rate_deg_s: dec_rate.to_degrees(),
        rate_vs_ground_deg_s: transverse_rate(rho, v_ecef).to_degrees(),
        rate_vs_stars_deg_s: transverse_rate(rho_i, rho_i_dot).to_degrees(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{EARTH_RADIUS_KM, MU_EARTH};

    const ARCSEC_PER_DEG: f64 = 3600.0;

    fn t0() -> Epoch {
        Epoch::from_utc(2026, 10, 4, 3, 0, 0.0).unwrap()
    }

    /// A satellite in an equatorial prograde circular orbit at `alt_km`,
    /// placed directly above (lat 0, lon 0) at `t0`.
    fn overhead_equatorial(alt_km: f64) -> StateVector {
        let a = EARTH_RADIUS_KM + alt_km;
        let vc = (MU_EARTH / a).sqrt();
        let theta = t0().gmst_rad();
        StateVector {
            epoch: t0(),
            r_km: [a * theta.cos(), a * theta.sin(), 0.0],
            v_km_s: [-vc * theta.sin(), vc * theta.cos(), 0.0],
        }
    }

    #[test]
    fn overhead_pass_rate_matches_scope_eval() {
        // scope-eval's overhead_rate_arcsec_s(500) = v / h = 3140 arcsec/s
        // ignores Earth rotation. At the equator the site moves eastward with
        // the satellite at omega * R, so against the stars the rate is
        // (v - omega R) / h, and v / h is recovered by adding omega R / h back.
        let s = overhead_equatorial(500.0);
        let o = observe(&s, &GroundSite::new(0.0, 0.0, 0.0).unwrap());
        assert!((o.el_deg - 90.0).abs() < 1e-6);
        assert!((o.range_km - 500.0).abs() < 1e-6);
        let site_speed_rate = (EARTH_ROTATION_RAD_S * EARTH_RADIUS_KM / 500.0).to_degrees() * ARCSEC_PER_DEG;
        let v_over_h = o.rate_vs_stars_deg_s * ARCSEC_PER_DEG + site_speed_rate;
        assert!((v_over_h - 3140.0).abs() / 3140.0 < 0.005, "{v_over_h}");
    }

    #[test]
    fn ground_rate_subtracts_earth_rotation_at_satellite_radius() {
        // Relative to the ground the satellite moves at v - omega * a.
        let s = overhead_equatorial(500.0);
        let o = observe(&s, &GroundSite::new(0.0, 0.0, 0.0).unwrap());
        let a = EARTH_RADIUS_KM + 500.0;
        let expected = ((MU_EARTH / a).sqrt() - EARTH_ROTATION_RAD_S * a) / 500.0;
        assert!((o.rate_vs_ground_deg_s - expected.to_degrees()).abs() < 1e-9);
        assert!(o.range_rate_km_s.abs() < 1e-9);
    }

    #[test]
    fn geostationary_satellite_is_still_against_the_ground() {
        let a: f64 = (MU_EARTH / EARTH_ROTATION_RAD_S.powi(2)).cbrt();
        let (r, v) = ecef_to_teme([a, 0.0, 0.0], [0.0; 3], t0());
        let o = observe(&StateVector { epoch: t0(), r_km: r, v_km_s: v }, &GroundSite::new(0.0, 10.0, 0.0).unwrap());
        assert!(o.rate_vs_ground_deg_s < 1e-12 && o.az_rate_deg_s.abs() < 1e-12 && o.el_rate_deg_s.abs() < 1e-12);
        // Against the stars it moves at the sidereal rate, 15.04 arcsec/s
        // times cos(dec), and dec is 0 from the equator, so an equatorial mount's hour-angle axis is stationary.
        assert!((o.rate_vs_stars_deg_s * ARCSEC_PER_DEG - 15.041).abs() < 0.01);
        assert!(o.ha_rate_deg_s.abs() < 1e-9 && o.dec_rate_deg_s.abs() < 1e-12);
    }

    #[test]
    fn axis_rates_agree_with_finite_differences() {
        // A Molniya-like orbit viewed from mid-latitude.
        use crate::keplerian::{KeplerElements, KeplerJ2};
        use crate::propagator::Propagator;
        let el = KeplerElements::from_altitudes(t0(), 600.0, 39_000.0, 63.4, 300.0, 270.0, 330.0).unwrap();
        let prop = KeplerJ2::new(el, "x").unwrap();
        let site = GroundSite::new(45.0, 30.0, 0.0).unwrap();
        let at = |dt: f64| observe(&prop.propagate(t0().add_seconds(dt)).unwrap(), &site);
        let (a, o, b) = (at(-0.5), at(0.0), at(0.5));
        let wrap = |d: f64| (d + 540.0).rem_euclid(360.0) - 180.0;
        assert!((wrap(b.az_deg - a.az_deg) - o.az_rate_deg_s).abs() < 1e-6);
        assert!(((b.el_deg - a.el_deg) - o.el_rate_deg_s).abs() < 1e-6);
        assert!(((b.dec_deg - a.dec_deg) - o.dec_rate_deg_s).abs() < 1e-6);
        assert!(((b.range_km - a.range_km) - o.range_rate_km_s).abs() < 1e-6);
    }
}
