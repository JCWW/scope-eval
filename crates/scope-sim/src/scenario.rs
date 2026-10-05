//! What is being observed, from where, and when: the target orbit, the
//! site, the search window and the prediction error.

use serde::{Deserialize, Serialize};

use orbit_prop::{
    find_passes as search_passes, Epoch, GroundSite, KeplerElements, KeplerJ2, Pass, PassDarkness, PassLighting,
    PassSearch, Propagator, Sgp4Propagator, Tle,
};

use crate::error::SimError;

/// TLE age beyond which pass times may be off by minutes. Matches
/// scope-eval's `STALE_TLE_DAYS`.
pub const STALE_TLE_DAYS: f64 = 14.0;
/// Longest search window accepted, hours.
pub const MAX_SEARCH_HOURS: f64 = 72.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TargetSpec {
    /// A pasted two- or three-line element set, propagated with SGP4.
    Tle { text: String },
    /// A what-if orbit, propagated as Keplerian with J2 drift.
    Kepler {
        name: String,
        /// ISO-8601 UTC epoch of the elements.
        epoch: String,
        perigee_km: f64,
        apogee_km: f64,
        inclination_deg: f64,
        raan_deg: f64,
        arg_perigee_deg: f64,
        mean_anomaly_deg: f64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SiteSpec {
    pub lat_deg: f64,
    pub lon_deg: f64,
    pub alt_m: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenarioSpec {
    pub site: SiteSpec,
    pub target: TargetSpec,
    /// ISO-8601 UTC start of the pass search.
    pub start: String,
    pub hours: f64,
    pub min_el_deg: f64,
    /// Along-track error of the prediction, km. Positive means the real
    /// satellite runs ahead of the prediction the mount follows.
    pub ephemeris_error_km: f64,
    /// Seed for the pointing-model and jitter draws.
    pub seed: u64,
}

/// One pass, as the dashboard lists it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PassSummary {
    pub index: usize,
    pub rise: String,
    pub culmination: String,
    pub set: String,
    pub duration_s: f64,
    pub max_el_deg: f64,
    pub peak_az_rate_deg_s: f64,
    pub peak_el_rate_deg_s: f64,
    pub peak_ha_rate_deg_s: f64,
    pub peak_dec_rate_deg_s: f64,
    pub lighting: &'static str,
    pub site_dark: &'static str,
    pub clipped_start: bool,
    pub clipped_end: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PassList {
    pub target: String,
    pub passes: Vec<PassSummary>,
    /// Problems worth showing next to the list: a stale TLE, or a search
    /// that stopped early.
    pub warnings: Vec<String>,
}

pub fn parse_epoch(text: &str, what: &str) -> Result<Epoch, SimError> {
    Epoch::parse_iso8601(text).map_err(|e| SimError::new(format!("{what}: {e}")))
}

pub fn build_site(s: &SiteSpec) -> Result<GroundSite, SimError> {
    Ok(GroundSite::new(s.lat_deg, s.lon_deg, s.alt_m)?)
}

/// The propagator for a target, and the epoch its elements describe.
pub fn build_propagator(target: &TargetSpec) -> Result<(Box<dyn Propagator>, Epoch), SimError> {
    match target {
        TargetSpec::Tle { text } => {
            let tle = Tle::parse(text)?;
            let epoch = tle.epoch;
            Ok((Box::new(Sgp4Propagator::new(&tle)?), epoch))
        }
        TargetSpec::Kepler {
            name,
            epoch,
            perigee_km,
            apogee_km,
            inclination_deg,
            raan_deg,
            arg_perigee_deg,
            mean_anomaly_deg,
        } => {
            let epoch = parse_epoch(epoch, "orbit epoch")?;
            let el = KeplerElements::from_altitudes(
                epoch,
                *perigee_km,
                *apogee_km,
                *inclination_deg,
                *raan_deg,
                *arg_perigee_deg,
                *mean_anomaly_deg,
            )?;
            Ok((Box::new(KeplerJ2::new(el, name)?), epoch))
        }
    }
}

/// The search window for a scenario, validated.
pub fn search_for(s: &ScenarioSpec) -> Result<PassSearch, SimError> {
    if !(s.hours > 0.0 && s.hours <= MAX_SEARCH_HOURS) {
        return Err(SimError::new(format!("the search window must be between 0 and {MAX_SEARCH_HOURS} hours")));
    }
    if !s.ephemeris_error_km.is_finite() || s.ephemeris_error_km.abs() > 1000.0 {
        return Err(SimError::new("the ephemeris error must be within +/-1000 km"));
    }
    let start = parse_epoch(&s.start, "search start")?;
    Ok(PassSearch { start, end: start.add_seconds(s.hours * 3600.0), min_el_deg: s.min_el_deg })
}

/// A scenario made concrete: its propagator, site, passes and warnings.
pub(crate) struct Prepared {
    pub prop: Box<dyn Propagator>,
    pub site: GroundSite,
    pub passes: Vec<Pass>,
    pub warnings: Vec<String>,
}

pub(crate) fn passes_for(s: &ScenarioSpec) -> Result<Prepared, SimError> {
    let site = build_site(&s.site)?;
    let (prop, epoch) = build_propagator(&s.target)?;
    let search = search_for(s)?;
    let mut warnings = Vec::new();
    if matches!(s.target, TargetSpec::Tle { .. }) {
        let age = |t: Epoch| t.seconds_since(&epoch).abs() / 86_400.0;
        let worst = age(search.start).max(age(search.end));
        if worst > STALE_TLE_DAYS {
            warnings.push(format!(
                "The search reaches {worst:.0} days from the TLE's epoch ({epoch}). Beyond {STALE_TLE_DAYS:.0} days, pass times may be off by minutes."
            ));
        }
    }
    let result = search_passes(prop.as_ref(), &site, &search);
    if let Some(e) = result.error {
        warnings.push(format!("The search stopped early: {e}"));
    }
    Ok(Prepared { prop, site, passes: result.passes, warnings })
}

pub fn find_passes(s: &ScenarioSpec) -> Result<PassList, SimError> {
    let p = passes_for(s)?;
    Ok(PassList {
        target: p.prop.label().to_string(),
        passes: p.passes.iter().enumerate().map(|(i, pass)| summarize(i, pass)).collect(),
        warnings: p.warnings,
    })
}

pub(crate) fn summarize(index: usize, p: &Pass) -> PassSummary {
    PassSummary {
        index,
        rise: p.rise.to_string(),
        culmination: p.culmination.to_string(),
        set: p.set.to_string(),
        duration_s: p.duration_s(),
        max_el_deg: p.max_el_deg,
        peak_az_rate_deg_s: p.peak_az_rate_deg_s,
        peak_el_rate_deg_s: p.peak_el_rate_deg_s,
        peak_ha_rate_deg_s: p.peak_ha_rate_deg_s,
        peak_dec_rate_deg_s: p.peak_dec_rate_deg_s,
        lighting: match p.lighting {
            PassLighting::Sunlit => "sunlit",
            PassLighting::Partial => "partial",
            PassLighting::Eclipsed => "eclipsed",
        },
        site_dark: match p.site_dark {
            PassDarkness::Dark => "dark",
            PassDarkness::Partial => "twilight",
            PassDarkness::Daylight => "daylight",
        },
        clipped_start: p.clipped_start,
        clipped_end: p.clipped_end,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The ISS TLE from the orbit-prop README.
    pub const ISS: &str = "ISS (ZARYA)
1 25544U 98067A   08264.51782528 -.00002182  00000-0 -11606-4 0  2927
2 25544  51.6416 247.4627 0006703 130.5360 325.0288 15.72125391563537";

    pub fn iss_scenario() -> ScenarioSpec {
        ScenarioSpec {
            // This site and window include an 83 deg pass, to exercise the keyhole.
            site: SiteSpec { lat_deg: 35.0, lon_deg: -100.0, alt_m: 100.0 },
            target: TargetSpec::Tle { text: ISS.into() },
            start: "2008-09-20T12:00:00Z".into(),
            hours: 72.0,
            min_el_deg: 10.0,
            ephemeris_error_km: 0.0,
            seed: 1,
        }
    }

    #[test]
    fn finds_iss_passes_from_a_fresh_tle() {
        let list = find_passes(&iss_scenario()).unwrap();
        assert_eq!(list.target, "ISS (ZARYA)");
        assert!(list.passes.len() >= 3, "{} passes", list.passes.len());
        assert!(list.warnings.is_empty(), "{:?}", list.warnings);
        for p in &list.passes {
            assert!(p.duration_s > 0.0 && p.duration_s < 900.0);
            assert!(p.max_el_deg >= 10.0 && p.max_el_deg <= 90.0);
        }
    }

    #[test]
    fn stale_tle_is_flagged() {
        let mut s = iss_scenario();
        s.start = "2008-11-01T00:00:00Z".into();
        let list = find_passes(&s).unwrap();
        assert!(list.warnings.iter().any(|w| w.contains("days from the TLE")));
    }

    #[test]
    fn rejects_bad_windows() {
        let mut s = iss_scenario();
        s.hours = 0.0;
        assert!(find_passes(&s).is_err());
        s.hours = 1000.0;
        assert!(find_passes(&s).is_err());
        let mut s = iss_scenario();
        s.start = "yesterday".into();
        assert!(find_passes(&s).is_err());
    }

    #[test]
    fn what_if_orbit_has_passes() {
        let mut s = iss_scenario();
        s.target = TargetSpec::Kepler {
            name: "550 km, 53 deg".into(),
            epoch: "2008-09-20T12:00:00Z".into(),
            perigee_km: 550.0,
            apogee_km: 550.0,
            inclination_deg: 53.0,
            raan_deg: 0.0,
            arg_perigee_deg: 0.0,
            mean_anomaly_deg: 0.0,
        };
        let list = find_passes(&s).unwrap();
        assert_eq!(list.target, "550 km, 53 deg");
        assert!(!list.passes.is_empty());
    }
}
