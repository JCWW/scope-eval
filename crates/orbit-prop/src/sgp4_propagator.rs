//! SGP4 propagation of TLEs, via the `sgp4` crate.
//!
//! Uses the crate's AFSPC compatibility mode, which reproduces the
//! reference implementation (Vallado et al. 2006) that the catalog's TLEs
//! are generated with. The crate's default "improved" mode differs from it
//! by up to tens of metres.

use crate::error::OrbitPropError;
use crate::propagator::Propagator;
use crate::state::StateVector;
use crate::time::Epoch;
use crate::tle::Tle;

pub struct Sgp4Propagator {
    tle: Tle,
    constants: sgp4::Constants,
    label: String,
}

impl Sgp4Propagator {
    pub fn new(tle: &Tle) -> Result<Sgp4Propagator, OrbitPropError> {
        let constants = sgp4::Constants::from_elements_afspc_compatibility_mode(&tle.elements)
            .map_err(|e| OrbitPropError::Sgp4(e.to_string()))?;
        let label = tle.name.clone().unwrap_or_else(|| format!("NORAD {}", tle.norad_id));
        Ok(Sgp4Propagator { tle: tle.clone(), constants, label })
    }

    pub fn tle(&self) -> &Tle {
        &self.tle
    }
}

impl Propagator for Sgp4Propagator {
    fn propagate(&self, t: Epoch) -> Result<StateVector, OrbitPropError> {
        let minutes = t.seconds_since(&self.tle.epoch) / 60.0;
        let p = self
            .constants
            .propagate_afspc_compatibility_mode(sgp4::MinutesSinceEpoch(minutes))
            .map_err(|e| OrbitPropError::Sgp4(e.to_string()))?;
        Ok(StateVector { epoch: t, r_km: p.position, v_km_s: p.velocity })
    }

    fn period_s(&self) -> f64 {
        86_400.0 / self.tle.mean_motion_rev_day
    }

    fn label(&self) -> &str {
        &self.label
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tle::tests::{ISS, SAT_00005_L1, SAT_00005_L2};
    use crate::vec3::{norm, sub};

    fn sat_00005() -> Sgp4Propagator {
        Sgp4Propagator::new(&Tle::parse(&format!("{SAT_00005_L1}\n{SAT_00005_L2}")).unwrap()).unwrap()
    }

    #[test]
    fn matches_vallado_verification_case_00005() {
        // Vallado et al. 2006, "Revisiting Spacetrack Report #3", tcppver.out.
        let prop = sat_00005();
        let epoch = prop.tle().epoch;
        let cases = [
            (0.0, [7022.46529266, -1400.08296755, 0.03995155], [1.893841015, 6.405893759, 4.534807250]),
            (360.0, [-7154.03120202, -3783.17682504, -3536.19412294], [4.741887409, -4.151817765, -2.093935425]),
        ];
        for (minutes, r, v) in cases {
            let s = prop.propagate(epoch.add_seconds(minutes * 60.0)).unwrap();
            assert!(norm(sub(s.r_km, r)) < 0.001, "t={minutes} r={:?}", s.r_km);
            assert!(norm(sub(s.v_km_s, v)) < 1e-6, "t={minutes} v={:?}", s.v_km_s);
        }
    }

    #[test]
    fn label_and_period() {
        let iss = Sgp4Propagator::new(&Tle::parse(ISS).unwrap()).unwrap();
        assert_eq!(iss.label(), "ISS (ZARYA)");
        assert!((iss.period_s() - 86_400.0 / 15.721_253_91).abs() < 1e-6);
        assert_eq!(sat_00005().label(), "NORAD 5");
    }

    #[test]
    fn decayed_orbit_returns_an_error() {
        // Vallado's case 28350 decays roughly 1460 minutes after epoch.
        let tle = Tle::parse(
            "1 28350U 04020A   06167.21788666  .16154492  76267-5  18678-3 0  8894
2 28350  64.9977 345.6130 0024870 260.7578  99.9590 16.47856722116490",
        )
        .unwrap();
        let prop = Sgp4Propagator::new(&tle).unwrap();
        assert!(prop.propagate(tle.epoch.add_seconds(1400.0 * 60.0)).is_ok());
        assert!(matches!(prop.propagate(tle.epoch.add_seconds(1500.0 * 60.0)), Err(OrbitPropError::Sgp4(_))));
    }
}
