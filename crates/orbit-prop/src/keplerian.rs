//! Keplerian orbit with secular J2 drift, for what-if orbits.
//!
//! The elements are treated as mean elements. J2 makes the node regress,
//! the perigee rotate and the mean motion change (Vallado eqs. 9-41):
//!
//! ```text
//! k      = 1.5 J2 (R / p)^2 n,   p = a (1 - e^2)
//! dRAAN  = -k cos i
//! dargp  =  k (2 - 2.5 sin^2 i)
//! dM     =  n + k sqrt(1 - e^2) (1 - 1.5 sin^2 i)
//! ```
//!
//! Short-period J2 terms, drag and third bodies are ignored, so this
//! drifts from a real satellite by kilometres per day. It is meant for
//! "what would a 550 km, 53 deg orbit look like from my site", not for
//! tracking a specific object; use a TLE and SGP4 for that.

use std::f64::consts::TAU;

use crate::constants::{EARTH_RADIUS_KM, J2, MU_EARTH};
use crate::error::OrbitPropError;
use crate::propagator::Propagator;
use crate::state::StateVector;
use crate::time::Epoch;
use crate::vec3::{add, cross, scale};

/// Classical orbital elements at `epoch`. Angles in degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeplerElements {
    pub epoch: Epoch,
    pub a_km: f64,
    pub e: f64,
    pub i_deg: f64,
    pub raan_deg: f64,
    pub argp_deg: f64,
    pub mean_anomaly_deg: f64,
}

impl KeplerElements {
    /// Elements from perigee and apogee altitudes above the equatorial radius, km.
    pub fn from_altitudes(
        epoch: Epoch,
        perigee_alt_km: f64,
        apogee_alt_km: f64,
        i_deg: f64,
        raan_deg: f64,
        argp_deg: f64,
        mean_anomaly_deg: f64,
    ) -> Result<KeplerElements, OrbitPropError> {
        // Negated comparisons here and below also reject NaN.
        if !(apogee_alt_km >= perigee_alt_km) {
            return Err(OrbitPropError::InvalidElements(format!(
                "apogee altitude {apogee_alt_km} km is below perigee altitude {perigee_alt_km} km"
            )));
        }
        let rp = EARTH_RADIUS_KM + perigee_alt_km;
        let ra = EARTH_RADIUS_KM + apogee_alt_km;
        Ok(KeplerElements {
            epoch,
            a_km: (rp + ra) / 2.0,
            e: (ra - rp) / (ra + rp),
            i_deg,
            raan_deg,
            argp_deg,
            mean_anomaly_deg,
        })
    }
}

/// Keplerian + secular J2 propagator.
#[derive(Debug, Clone)]
pub struct KeplerJ2 {
    el: KeplerElements,
    label: String,
    n: f64,
    raan_dot: f64,
    argp_dot: f64,
    m_dot: f64,
}

impl KeplerJ2 {
    pub fn new(el: KeplerElements, label: &str) -> Result<KeplerJ2, OrbitPropError> {
        let finite = [el.a_km, el.e, el.i_deg, el.raan_deg, el.argp_deg, el.mean_anomaly_deg]
            .iter()
            .all(|x| x.is_finite());
        if !finite {
            return Err(OrbitPropError::InvalidElements("elements must be finite numbers".into()));
        }
        if !(0.0..1.0).contains(&el.e) {
            return Err(OrbitPropError::InvalidElements(format!(
                "eccentricity {} must be at least 0 and below 1",
                el.e
            )));
        }
        if !(0.0..=180.0).contains(&el.i_deg) {
            return Err(OrbitPropError::InvalidElements(format!(
                "inclination {} deg must be between 0 and 180",
                el.i_deg
            )));
        }
        if el.a_km * (1.0 - el.e) <= EARTH_RADIUS_KM {
            return Err(OrbitPropError::InvalidElements(format!(
                "perigee radius {:.1} km is inside the Earth",
                el.a_km * (1.0 - el.e)
            )));
        }
        let n = (MU_EARTH / el.a_km.powi(3)).sqrt();
        let p = el.a_km * (1.0 - el.e * el.e);
        let k = 1.5 * J2 * (EARTH_RADIUS_KM / p).powi(2) * n;
        let (si, ci) = el.i_deg.to_radians().sin_cos();
        Ok(KeplerJ2 {
            el,
            label: label.to_string(),
            n,
            raan_dot: -k * ci,
            argp_dot: k * (2.0 - 2.5 * si * si),
            m_dot: n + k * (1.0 - el.e * el.e).sqrt() * (1.0 - 1.5 * si * si),
        })
    }

    pub fn elements(&self) -> &KeplerElements {
        &self.el
    }
}

/// Solve Kepler's equation `E - e sin E = M` for the eccentric anomaly.
fn eccentric_anomaly(m: f64, e: f64) -> Result<f64, OrbitPropError> {
    let m = m.rem_euclid(TAU);
    let mut ea = if e < 0.8 { m } else { std::f64::consts::PI };
    for _ in 0..50 {
        let step = (ea - e * ea.sin() - m) / (1.0 - e * ea.cos());
        ea -= step;
        if step.abs() < 1e-12 {
            return Ok(ea);
        }
    }
    Err(OrbitPropError::NoConvergence(format!("Kepler's equation for M = {m}, e = {e}")))
}

impl Propagator for KeplerJ2 {
    fn propagate(&self, t: Epoch) -> Result<StateVector, OrbitPropError> {
        let dt = t.seconds_since(&self.el.epoch);
        let raan = self.el.raan_deg.to_radians() + self.raan_dot * dt;
        let argp = self.el.argp_deg.to_radians() + self.argp_dot * dt;
        let m = self.el.mean_anomaly_deg.to_radians() + self.m_dot * dt;
        let (a, e) = (self.el.a_km, self.el.e);
        let ea = eccentric_anomaly(m, e)?;
        let (se, ce) = ea.sin_cos();
        let root = (1.0 - e * e).sqrt();
        let r = a * (1.0 - e * ce);
        // Perifocal position and velocity.
        let (xp, yp) = (a * (ce - e), a * root * se);
        let k = (MU_EARTH * a).sqrt() / r;
        let (vxp, vyp) = (-k * se, k * root * ce);
        // Perifocal -> inertial: R3(-raan) R1(-i) R3(-argp).
        let (so, co) = raan.sin_cos();
        let (sw, cw) = argp.sin_cos();
        let (si, ci) = self.el.i_deg.to_radians().sin_cos();
        let p = [co * cw - so * sw * ci, so * cw + co * sw * ci, sw * si];
        let q = [-co * sw - so * cw * ci, -so * sw + co * cw * ci, cw * si];
        let rv = |x: f64, y: f64| [x * p[0] + y * q[0], x * p[1] + y * q[1], x * p[2] + y * q[2]];
        let r_vec = rv(xp, yp);
        // The velocity must be the time derivative of the drifting position,
        // or angular rates computed from it disagree with the path. Scale the
        // two-body velocity by M_dot / n, then add the in-plane rotation of
        // the perigee (about the orbit normal) and the nodal regression
        // (about z).
        let h_hat = cross(p, q);
        let v_vec = add(
            add(scale(rv(vxp, vyp), self.m_dot / self.n), scale(cross(h_hat, r_vec), self.argp_dot)),
            scale(cross([0.0, 0.0, 1.0], r_vec), self.raan_dot),
        );
        Ok(StateVector { epoch: t, r_km: r_vec, v_km_s: v_vec })
    }

    fn period_s(&self) -> f64 {
        TAU / self.n
    }

    fn label(&self) -> &str {
        &self.label
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vec3::{cross, norm};

    fn epoch() -> Epoch {
        Epoch::from_utc(2026, 10, 4, 0, 0, 0.0).unwrap()
    }

    fn circular(alt_km: f64, i_deg: f64) -> KeplerJ2 {
        let el = KeplerElements::from_altitudes(epoch(), alt_km, alt_km, i_deg, 0.0, 0.0, 0.0).unwrap();
        KeplerJ2::new(el, "test").unwrap()
    }

    #[test]
    fn period_is_two_pi_root_a_cubed_over_mu() {
        let a: f64 = 6378.137 + 500.0;
        let expected = TAU * (a.powi(3) / MU_EARTH).sqrt();
        assert!((circular(500.0, 0.0).period_s() - expected).abs() < 1e-9);
        assert!((expected - 5676.98).abs() < 0.01);
    }

    #[test]
    fn equatorial_circular_orbit_starts_on_x_axis() {
        let prop = circular(500.0, 0.0);
        let s = prop.propagate(epoch()).unwrap();
        let a = 6878.137;
        assert!((s.r_km[0] - a).abs() < 1e-9 && s.r_km[1].abs() < 1e-9 && s.r_km[2].abs() < 1e-9);
        // Speed is a times the rate of the argument of latitude, which J2
        // makes slightly faster than the two-body mean motion.
        let u_dot = prop.m_dot + prop.argp_dot + prop.raan_dot;
        assert!(s.v_km_s[0].abs() < 1e-12 && (s.v_km_s[1] - a * u_dot).abs() < 1e-12);
        assert!(u_dot > prop.n && (u_dot - prop.n) / prop.n < 3e-3);
    }

    #[test]
    fn velocity_is_the_time_derivative_of_position() {
        let el = KeplerElements::from_altitudes(epoch(), 400.0, 39_000.0, 63.4, 40.0, 270.0, 10.0).unwrap();
        let prop = KeplerJ2::new(el, "molniya").unwrap();
        for hours in [0.0, 1.0, 5.5, 13.0, 100.0] {
            let t = epoch().add_seconds(hours * 3600.0);
            let a = prop.propagate(t.add_seconds(-0.5)).unwrap();
            let o = prop.propagate(t).unwrap();
            let b = prop.propagate(t.add_seconds(0.5)).unwrap();
            for k in 0..3 {
                assert!(((b.r_km[k] - a.r_km[k]) - o.v_km_s[k]).abs() < 1e-5, "hours {hours} axis {k}");
            }
        }
    }

    #[test]
    fn propagates_backwards_from_epoch() {
        // A search window that starts before the element epoch must work.
        let prop = circular(550.0, 53.0);
        let back = prop.propagate(epoch().add_seconds(-86_400.0)).unwrap();
        assert!((norm(back.r_km) - (6378.137 + 550.0)).abs() < 1e-6);
    }

    #[test]
    fn perigee_and_apogee_radii() {
        let at_m = |m: f64| {
            let el = KeplerElements::from_altitudes(epoch(), 400.0, 39_000.0, 63.4, 40.0, 270.0, m).unwrap();
            norm(KeplerJ2::new(el, "x").unwrap().propagate(epoch()).unwrap().r_km)
        };
        assert!((at_m(0.0) - (6378.137 + 400.0)).abs() < 1e-6);
        assert!((at_m(180.0) - (6378.137 + 39_000.0)).abs() < 1e-6);
    }

    #[test]
    fn sun_synchronous_orbit_regresses_about_one_degree_per_day() {
        // An 800 km orbit at 98.6 deg is sun-synchronous: the node advances
        // 360 deg per year, 0.9856 deg/day.
        let prop = circular(800.0, 98.6);
        let node = |t: Epoch| {
            let s = prop.propagate(t).unwrap();
            let h = cross(s.r_km, s.v_km_s);
            h[0].atan2(-h[1]).to_degrees()
        };
        let drift = node(epoch().add_seconds(86_400.0)) - node(epoch());
        assert!((drift - 0.9856).abs() < 0.01, "drift {drift}");
    }

    #[test]
    fn rejects_impossible_orbits() {
        let bad = |e: f64, a: f64, i: f64| {
            let el = KeplerElements { epoch: epoch(), a_km: a, e, i_deg: i, raan_deg: 0.0, argp_deg: 0.0, mean_anomaly_deg: 0.0 };
            KeplerJ2::new(el, "x").is_err()
        };
        assert!(bad(1.0, 10_000.0, 0.0), "parabolic");
        assert!(bad(-0.1, 10_000.0, 0.0), "negative e");
        assert!(bad(0.5, 10_000.0, 0.0), "perigee 5000 km radius is underground");
        assert!(bad(0.0, 7000.0, 200.0), "inclination over 180");
        assert!(bad(0.0, f64::NAN, 0.0), "NaN");
        assert!(KeplerElements::from_altitudes(epoch(), 1000.0, 500.0, 0.0, 0.0, 0.0, 0.0).is_err());
    }

    #[test]
    fn highly_eccentric_orbit_converges() {
        let el = KeplerElements { epoch: epoch(), a_km: 200_000.0, e: 0.96, i_deg: 10.0, raan_deg: 0.0, argp_deg: 0.0, mean_anomaly_deg: 0.0 };
        let prop = KeplerJ2::new(el, "x").unwrap();
        for k in 0..200 {
            assert!(prop.propagate(epoch().add_seconds(k as f64 * 3_000.0)).is_ok());
        }
    }
}
