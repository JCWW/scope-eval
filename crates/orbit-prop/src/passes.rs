//! Finding passes of a satellite over a ground site.
//!
//! 1. Scan elevation minus the minimum elevation at a coarse step of
//!    `period / 60`, clamped to 10-300 s.
//! 2. Refine each sign change by bisection to 0.1 s.
//! 3. Sample the pass densely (1 s, or `duration / 7200` for passes longer
//!    than two hours) and at 0.1 s within 30 s of culmination, recording
//!    peak axis rates, accelerations (differences of the analytic rates),
//!    lighting and site darkness.
//! 4. Refine culmination by golden-section search on elevation.
//!
//! A pass shorter than the coarse step can be missed. For LEO that means
//! grazing passes that never get far above the minimum elevation.

use crate::constants::{DARK_SUN_ELEVATION_DEG, EARTH_ROTATION_RAD_S, MAX_SEARCH_WINDOW_S};
use crate::error::OrbitPropError;
use crate::illumination::{lighting, sun_elevation_deg, Lighting};
use crate::observe::{observe, Observation};
use crate::propagator::Propagator;
use crate::site::GroundSite;
use crate::sun_moon::sun_position_km;
use crate::time::Epoch;

const REFINE_TOLERANCE_S: f64 = 0.1;
const DENSE_STEP_S: f64 = 1.0;
const MAX_DENSE_SAMPLES: f64 = 7200.0;
const FINE_HALF_WINDOW_S: f64 = 30.0;
const FINE_STEP_S: f64 = 0.1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PassSearch {
    pub start: Epoch,
    pub end: Epoch,
    /// Elevation above which the satellite counts as "in the pass", degrees.
    pub min_el_deg: f64,
}

/// Whether the satellite is sunlit during the pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassLighting {
    /// Sunlit for the whole pass.
    Sunlit,
    /// Enters or leaves the Earth's shadow during the pass.
    Partial,
    /// In shadow (umbra or penumbra) for the whole pass.
    Eclipsed,
}

/// Whether the site is dark (Sun below `DARK_SUN_ELEVATION_DEG`) during the pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassDarkness {
    Dark,
    Partial,
    /// The Sun is above the dark threshold for the whole pass. Includes twilight.
    Daylight,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pass {
    pub rise: Epoch,
    pub culmination: Epoch,
    pub set: Epoch,
    /// Already above the minimum elevation at the start of the search.
    pub clipped_start: bool,
    /// Still above the minimum elevation at the end of the search.
    pub clipped_end: bool,
    pub max_el_deg: f64,
    pub peak_az_rate_deg_s: f64,
    pub peak_el_rate_deg_s: f64,
    pub peak_az_accel_deg_s2: f64,
    pub peak_el_accel_deg_s2: f64,
    pub peak_ha_rate_deg_s: f64,
    pub peak_dec_rate_deg_s: f64,
    pub peak_ha_accel_deg_s2: f64,
    pub peak_dec_accel_deg_s2: f64,
    pub peak_rate_vs_ground_deg_s: f64,
    pub lighting: PassLighting,
    pub site_dark: PassDarkness,
}

impl Pass {
    pub fn duration_s(&self) -> f64 {
        self.set.seconds_since(&self.rise)
    }
}

/// Passes found, plus the error that stopped the search early, if any.
/// Passes completed before the error are kept.
#[derive(Debug, Clone, PartialEq)]
pub struct PassResult {
    pub passes: Vec<Pass>,
    pub error: Option<OrbitPropError>,
}

pub fn find_passes(prop: &dyn Propagator, site: &GroundSite, search: &PassSearch) -> PassResult {
    let mut passes = Vec::new();
    let error = validate(search).and_then(|_| scan(prop, site, search, &mut passes)).err();
    PassResult { passes, error }
}

fn validate(search: &PassSearch) -> Result<(), OrbitPropError> {
    let span = search.end.seconds_since(&search.start);
    // Negated so a NaN span is rejected too.
    if !(span > 0.0) {
        return Err(OrbitPropError::InvalidTime("the search must end after it starts".into()));
    }
    if span > MAX_SEARCH_WINDOW_S {
        return Err(OrbitPropError::InvalidTime(format!(
            "search window of {:.1} days exceeds the {:.0}-day limit",
            span / 86_400.0,
            MAX_SEARCH_WINDOW_S / 86_400.0
        )));
    }
    if !search.min_el_deg.is_finite() || !(-90.0..90.0).contains(&search.min_el_deg) {
        return Err(OrbitPropError::InvalidTime(format!(
            "minimum elevation {} deg must be at least -90 and below 90",
            search.min_el_deg
        )));
    }
    Ok(())
}

fn look(prop: &dyn Propagator, site: &GroundSite, t: Epoch) -> Result<Observation, OrbitPropError> {
    Ok(observe(&prop.propagate(t)?, site))
}

fn coarse_step_s(prop: &dyn Propagator) -> f64 {
    let period = prop.period_s();
    if period.is_finite() && period > 0.0 {
        (period / 60.0).clamp(10.0, 300.0)
    } else {
        60.0
    }
}

fn scan(prop: &dyn Propagator, site: &GroundSite, search: &PassSearch, out: &mut Vec<Pass>) -> Result<(), OrbitPropError> {
    let above = |t: Epoch| -> Result<f64, OrbitPropError> { Ok(look(prop, site, t)?.el_deg - search.min_el_deg) };
    let step = coarse_step_s(prop);
    let mut t0 = search.start;
    let mut f0 = above(t0)?;
    let mut rise: Option<(Epoch, bool)> = if f0 > 0.0 { Some((t0, true)) } else { None };
    while search.end.seconds_since(&t0) > 0.0 {
        let t1 = if search.end.seconds_since(&t0) > step { t0.add_seconds(step) } else { search.end };
        let f1 = above(t1)?;
        if f0 <= 0.0 && f1 > 0.0 {
            rise = Some((bisect(&above, t0, t1)?, false));
        } else if f0 > 0.0 && f1 <= 0.0 {
            if let Some((r, clipped)) = rise.take() {
                let set = bisect(&above, t0, t1)?;
                out.push(describe(prop, site, r, set, clipped, false)?);
            }
        }
        t0 = t1;
        f0 = f1;
    }
    if let Some((r, clipped)) = rise {
        out.push(describe(prop, site, r, search.end, clipped, true)?);
    }
    Ok(())
}

/// Find where `f` changes sign between `lo` and `hi`, to `REFINE_TOLERANCE_S`.
fn bisect(f: &dyn Fn(Epoch) -> Result<f64, OrbitPropError>, mut lo: Epoch, mut hi: Epoch) -> Result<Epoch, OrbitPropError> {
    let lo_positive = f(lo)? > 0.0;
    while hi.seconds_since(&lo) > REFINE_TOLERANCE_S {
        let mid = lo.add_seconds(hi.seconds_since(&lo) / 2.0);
        if (f(mid)? > 0.0) == lo_positive {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Ok(lo.add_seconds(hi.seconds_since(&lo) / 2.0))
}

/// Peak absolute values of rates and of rate differences over a sample run.
#[derive(Default)]
struct Peaks {
    az: f64,
    el: f64,
    ha: f64,
    dec: f64,
    ground: f64,
    az_acc: f64,
    el_acc: f64,
    ha_acc: f64,
    dec_acc: f64,
}

impl Peaks {
    fn add_run(&mut self, obs: &[Observation], dt: f64) {
        for o in obs {
            self.az = self.az.max(o.az_rate_deg_s.abs());
            self.el = self.el.max(o.el_rate_deg_s.abs());
            self.ha = self.ha.max(o.ha_rate_deg_s.abs());
            self.dec = self.dec.max(o.dec_rate_deg_s.abs());
            self.ground = self.ground.max(o.rate_vs_ground_deg_s);
        }
        for w in obs.windows(2) {
            let d = |f: fn(&Observation) -> f64| ((f(&w[1]) - f(&w[0])) / dt).abs();
            self.az_acc = self.az_acc.max(d(|o| o.az_rate_deg_s));
            self.el_acc = self.el_acc.max(d(|o| o.el_rate_deg_s));
            self.ha_acc = self.ha_acc.max(d(|o| o.ha_rate_deg_s));
            self.dec_acc = self.dec_acc.max(d(|o| o.dec_rate_deg_s));
        }
        // Through the zenith (alt-az) or the celestial pole (equatorial) the
        // analytic axis rate is near 0 on both sides of an instantaneous
        // 180 deg flip. Finite differences of the angles themselves catch
        // the flip, so the larger of the two is reported.
        let sidereal_deg_s = EARTH_ROTATION_RAD_S.to_degrees();
        let az_fd: Vec<f64> = obs.windows(2).map(|w| wrap_deg(w[1].az_deg - w[0].az_deg) / dt).collect();
        let ha_fd: Vec<f64> =
            obs.windows(2).map(|w| sidereal_deg_s - wrap_deg(w[1].ra_deg - w[0].ra_deg) / dt).collect();
        for (fd, rate, acc) in [(&az_fd, &mut self.az, &mut self.az_acc), (&ha_fd, &mut self.ha, &mut self.ha_acc)] {
            for r in fd {
                *rate = rate.max(r.abs());
            }
            for w in fd.windows(2) {
                *acc = acc.max(((w[1] - w[0]) / dt).abs());
            }
        }
    }
}

/// An angle difference wrapped into `[-180, 180)` degrees.
fn wrap_deg(d: f64) -> f64 {
    (d + 180.0).rem_euclid(360.0) - 180.0
}

/// Evenly spaced times from `a` to `b` inclusive, no further apart than `max_step`.
fn sample_times(a: Epoch, b: Epoch, max_step: f64) -> (Vec<Epoch>, f64) {
    let span = b.seconds_since(&a).max(0.0);
    let n = ((span / max_step).ceil() as usize).max(1);
    let dt = span / n as f64;
    ((0..=n).map(|k| a.add_seconds(k as f64 * dt)).collect(), dt)
}

fn summarize<T>(flags: &[bool], all: T, none: T, mixed: T) -> T {
    if flags.iter().all(|&f| f) {
        all
    } else if flags.iter().any(|&f| f) {
        mixed
    } else {
        none
    }
}

fn describe(
    prop: &dyn Propagator,
    site: &GroundSite,
    rise: Epoch,
    set: Epoch,
    clipped_start: bool,
    clipped_end: bool,
) -> Result<Pass, OrbitPropError> {
    let duration = set.seconds_since(&rise);
    let dense_step = DENSE_STEP_S.max(duration / MAX_DENSE_SAMPLES);
    let (times, dt) = sample_times(rise, set, dense_step);
    let mut obs = Vec::with_capacity(times.len());
    let mut sunlit = Vec::with_capacity(times.len());
    let mut dark = Vec::with_capacity(times.len());
    for &t in &times {
        let state = prop.propagate(t)?;
        obs.push(observe(&state, site));
        sunlit.push(lighting(state.r_km, sun_position_km(t)) == Lighting::Sunlit);
        dark.push(sun_elevation_deg(site, t) < DARK_SUN_ELEVATION_DEG);
    }
    let mut peaks = Peaks::default();
    peaks.add_run(&obs, dt.max(f64::MIN_POSITIVE));

    // Culmination: best dense sample, then golden-section refinement.
    let best = obs.iter().enumerate().fold(0, |b, (k, o)| if o.el_deg > obs[b].el_deg { k } else { b });
    let lo = times[best.saturating_sub(1)];
    let hi = times[(best + 1).min(times.len() - 1)];
    let (culmination, max_el) = golden_max(&|t| Ok(look(prop, site, t)?.el_deg), lo, hi)?;

    // Fine sampling near culmination, where the alt-az keyhole bites.
    let fine_lo = culmination.add_seconds(-FINE_HALF_WINDOW_S).max_of(rise);
    let fine_hi = culmination.add_seconds(FINE_HALF_WINDOW_S).min_of(set);
    let (fine_times, fine_dt) = sample_times(fine_lo, fine_hi, FINE_STEP_S);
    if fine_dt > 0.0 {
        let fine: Result<Vec<Observation>, OrbitPropError> = fine_times.iter().map(|&t| look(prop, site, t)).collect();
        peaks.add_run(&fine?, fine_dt);
    }

    Ok(Pass {
        rise,
        culmination,
        set,
        clipped_start,
        clipped_end,
        max_el_deg: max_el,
        peak_az_rate_deg_s: peaks.az,
        peak_el_rate_deg_s: peaks.el,
        peak_az_accel_deg_s2: peaks.az_acc,
        peak_el_accel_deg_s2: peaks.el_acc,
        peak_ha_rate_deg_s: peaks.ha,
        peak_dec_rate_deg_s: peaks.dec,
        peak_ha_accel_deg_s2: peaks.ha_acc,
        peak_dec_accel_deg_s2: peaks.dec_acc,
        peak_rate_vs_ground_deg_s: peaks.ground,
        lighting: summarize(&sunlit, PassLighting::Sunlit, PassLighting::Eclipsed, PassLighting::Partial),
        site_dark: summarize(&dark, PassDarkness::Dark, PassDarkness::Daylight, PassDarkness::Partial),
    })
}

/// Maximise `f` on `[lo, hi]` by golden-section search to 0.01 s.
fn golden_max(
    f: &dyn Fn(Epoch) -> Result<f64, OrbitPropError>,
    lo: Epoch,
    hi: Epoch,
) -> Result<(Epoch, f64), OrbitPropError> {
    const INV_PHI: f64 = 0.618_033_988_749_895;
    let (mut a, mut b) = (lo, hi);
    while b.seconds_since(&a) > 0.01 {
        let span = b.seconds_since(&a);
        let c = b.add_seconds(-span * INV_PHI);
        let d = a.add_seconds(span * INV_PHI);
        if f(c)? > f(d)? {
            b = d;
        } else {
            a = c;
        }
    }
    let t = a.add_seconds(b.seconds_since(&a) / 2.0);
    Ok((t, f(t)?))
}

trait EpochOrd {
    fn max_of(self, other: Epoch) -> Epoch;
    fn min_of(self, other: Epoch) -> Epoch;
}

impl EpochOrd for Epoch {
    fn max_of(self, other: Epoch) -> Epoch {
        if self.seconds_since(&other) >= 0.0 { self } else { other }
    }
    fn min_of(self, other: Epoch) -> Epoch {
        if self.seconds_since(&other) <= 0.0 { self } else { other }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::EARTH_RADIUS_KM;
    use crate::frames::ecef_to_teme;
    use crate::state::StateVector;
    use crate::tle::Tle;
    use crate::Sgp4Propagator;

    fn t0() -> Epoch {
        Epoch::from_utc(2026, 10, 4, 0, 0, 0.0).unwrap()
    }

    /// An equatorial satellite whose Earth-fixed longitude moves at a
    /// constant rate, so pass geometry over an equatorial site is analytic.
    /// Fails with an SGP4-style error after `fail_after_s`, if set.
    struct EarthFixedCircle {
        radius_km: f64,
        lon0_deg: f64,
        rate_deg_s: f64,
        fail_after_s: Option<f64>,
    }

    impl Propagator for EarthFixedCircle {
        fn propagate(&self, t: Epoch) -> Result<StateVector, OrbitPropError> {
            let dt = t.seconds_since(&t0());
            if self.fail_after_s.is_some_and(|f| dt > f) {
                return Err(OrbitPropError::Sgp4("decayed".into()));
            }
            let lon = (self.lon0_deg + self.rate_deg_s * dt).to_radians();
            let w = self.rate_deg_s.to_radians();
            let r = [self.radius_km * lon.cos(), self.radius_km * lon.sin(), 0.0];
            let v = [-self.radius_km * w * lon.sin(), self.radius_km * w * lon.cos(), 0.0];
            let (rt, vt) = ecef_to_teme(r, v, t);
            Ok(StateVector { epoch: t, r_km: rt, v_km_s: vt })
        }
        fn period_s(&self) -> f64 {
            if self.rate_deg_s == 0.0 { 86_164.0 } else { 360.0 / self.rate_deg_s.abs() }
        }
        fn label(&self) -> &str {
            "test"
        }
    }

    fn equator() -> GroundSite {
        GroundSite::new(0.0, 0.0, 0.0).unwrap()
    }

    fn search(hours: f64, min_el: f64) -> PassSearch {
        PassSearch { start: t0(), end: t0().add_seconds(hours * 3600.0), min_el_deg: min_el }
    }

    /// Geocentric half-angle of the arc above `min_el` (Vallado eq. 11-4 rearranged).
    fn half_arc_deg(radius_km: f64, min_el_deg: f64) -> f64 {
        let e = min_el_deg.to_radians();
        ((EARTH_RADIUS_KM * e.cos() / radius_km).acos() - e).to_degrees()
    }

    #[test]
    fn overhead_pass_rise_and_set_match_geometry() {
        let sat = EarthFixedCircle { radius_km: EARTH_RADIUS_KM + 500.0, lon0_deg: -30.0, rate_deg_s: 0.06, fail_after_s: None };
        let r = find_passes(&sat, &equator(), &search(1.0, 10.0));
        assert_eq!(r.error, None);
        assert_eq!(r.passes.len(), 1);
        let p = &r.passes[0];
        let psi = half_arc_deg(EARTH_RADIUS_KM + 500.0, 10.0);
        assert!((p.rise.seconds_since(&t0()) - (30.0 - psi) / 0.06).abs() < 1.0);
        assert!((p.set.seconds_since(&t0()) - (30.0 + psi) / 0.06).abs() < 1.0);
        assert!((p.culmination.seconds_since(&t0()) - 500.0).abs() < 0.5);
        assert!(p.max_el_deg > 89.99);
        assert!(!p.clipped_start && !p.clipped_end);
        // At the zenith the ground rate is (a * omega) / h.
        let expected = 0.06 * (EARTH_RADIUS_KM + 500.0) / 500.0;
        assert!((p.peak_rate_vs_ground_deg_s - expected).abs() / expected < 1e-3, "{}", p.peak_rate_vs_ground_deg_s);
    }

    #[test]
    fn exact_zenith_pass_reports_the_azimuth_flip() {
        // Straight through the zenith the analytic azimuth rate is 0 on both
        // sides, but the azimuth axis must flip 180 deg at culmination. The
        // peak must show that, or an alt-az mount gets a false PASS.
        let sat = EarthFixedCircle { radius_km: EARTH_RADIUS_KM + 500.0, lon0_deg: -30.0, rate_deg_s: 0.06, fail_after_s: None };
        let p = &find_passes(&sat, &equator(), &search(1.0, 10.0)).passes[0];
        assert!(p.peak_az_rate_deg_s > 100.0, "az rate {}", p.peak_az_rate_deg_s);
        assert!(p.peak_az_accel_deg_s2 > 100.0, "az accel {}", p.peak_az_accel_deg_s2);
    }

    #[test]
    fn pass_over_the_celestial_pole_reports_the_hour_angle_flip() {
        // From the South Pole a polar orbit crosses the zenith, which is the
        // celestial pole, so an equatorial mount's hour-angle axis flips.
        use crate::keplerian::{KeplerElements, KeplerJ2};
        let el = KeplerElements::from_altitudes(t0(), 800.0, 800.0, 90.0, 0.0, 0.0, 0.0).unwrap();
        let prop = KeplerJ2::new(el, "polar").unwrap();
        let site = GroundSite::new(-90.0, 0.0, 2835.0).unwrap();
        let r = find_passes(&prop, &site, &search(24.0, 10.0));
        let p = r.passes.iter().max_by(|a, b| a.max_el_deg.total_cmp(&b.max_el_deg)).unwrap();
        assert!(p.max_el_deg > 89.9, "max el {}", p.max_el_deg);
        assert!(p.peak_ha_rate_deg_s > 100.0, "HA rate {}", p.peak_ha_rate_deg_s);
    }

    #[test]
    fn near_zenith_pass_spins_the_azimuth_axis() {
        // Seen from half a degree north of the track the pass culminates near 84 deg,
        // and the azimuth axis must turn far faster than the target moves.
        let sat = EarthFixedCircle { radius_km: EARTH_RADIUS_KM + 500.0, lon0_deg: -30.0, rate_deg_s: 0.06, fail_after_s: None };
        let site = GroundSite::new(0.5, 0.0, 0.0).unwrap();
        let p = &find_passes(&sat, &site, &search(1.0, 10.0)).passes[0];
        assert!(p.max_el_deg > 80.0 && p.max_el_deg < 89.0, "{}", p.max_el_deg);
        assert!(p.peak_az_rate_deg_s > 5.0 * p.peak_rate_vs_ground_deg_s);
        assert!(p.peak_az_accel_deg_s2 > p.peak_el_accel_deg_s2);
    }

    #[test]
    fn search_starting_mid_pass_is_clipped_at_the_start() {
        // At t0 the satellite is 5 deg west of the site, already well up.
        let sat = EarthFixedCircle { radius_km: EARTH_RADIUS_KM + 500.0, lon0_deg: -5.0, rate_deg_s: 0.06, fail_after_s: None };
        let r = find_passes(&sat, &equator(), &search(1.0, 10.0));
        assert_eq!(r.passes.len(), 1);
        let p = &r.passes[0];
        assert!(p.clipped_start && !p.clipped_end);
        assert_eq!(p.rise, t0());
        let psi = half_arc_deg(EARTH_RADIUS_KM + 500.0, 10.0);
        assert!((p.set.seconds_since(&t0()) - (5.0 + psi) / 0.06).abs() < 1.0);
    }

    #[test]
    fn polar_orbit_over_a_polar_site_passes_every_revolution() {
        use crate::keplerian::{KeplerElements, KeplerJ2};
        let el = KeplerElements::from_altitudes(t0(), 800.0, 800.0, 90.0, 0.0, 0.0, 0.0).unwrap();
        let prop = KeplerJ2::new(el, "polar").unwrap();
        let site = GroundSite::new(89.9, 0.0, 0.0).unwrap();
        let r = find_passes(&prop, &site, &search(24.0, 10.0));
        assert_eq!(r.error, None);
        let revolutions = 86_400.0 / prop.period_s();
        assert!((r.passes.len() as f64 - revolutions).abs() <= 1.0, "{} passes, {revolutions:.1} revs", r.passes.len());
        assert!(r.passes.iter().all(|p| p.max_el_deg > 60.0));
    }

    #[test]
    fn stationary_satellite_overhead_is_one_clipped_pass() {
        let geo = EarthFixedCircle { radius_km: 42_164.0, lon0_deg: 0.0, rate_deg_s: 0.0, fail_after_s: None };
        let r = find_passes(&geo, &equator(), &search(6.0, 10.0));
        assert_eq!(r.error, None);
        assert_eq!(r.passes.len(), 1);
        let p = &r.passes[0];
        assert!(p.clipped_start && p.clipped_end);
        assert_eq!(p.rise, t0());
        assert!((p.duration_s() - 6.0 * 3600.0).abs() < 1e-6);
        assert!(p.max_el_deg > 89.99 && p.peak_rate_vs_ground_deg_s < 1e-9);
    }

    #[test]
    fn satellite_that_never_rises_gives_no_passes() {
        let geo = EarthFixedCircle { radius_km: 42_164.0, lon0_deg: 180.0, rate_deg_s: 0.0, fail_after_s: None };
        assert_eq!(find_passes(&geo, &equator(), &search(24.0, 10.0)), PassResult { passes: vec![], error: None });
    }

    #[test]
    fn propagation_failure_keeps_earlier_passes() {
        // A pass every 3000 s; failure at 5000 s, after the second pass.
        let sat = EarthFixedCircle { radius_km: EARTH_RADIUS_KM + 500.0, lon0_deg: -30.0, rate_deg_s: 0.12, fail_after_s: Some(5000.0) };
        let r = find_passes(&sat, &equator(), &search(3.0, 10.0));
        assert_eq!(r.passes.len(), 2);
        assert!(matches!(r.error, Some(OrbitPropError::Sgp4(_))));
    }

    #[test]
    fn decaying_tle_reports_an_sgp4_error() {
        let tle = Tle::parse(
            "1 28350U 04020A   06167.21788666  .16154492  76267-5  18678-3 0  8894
2 28350  64.9977 345.6130 0024870 260.7578  99.9590 16.47856722116490",
        )
        .unwrap();
        let prop = Sgp4Propagator::new(&tle).unwrap();
        let s = PassSearch { start: tle.epoch, end: tle.epoch.add_seconds(2.0 * 86_400.0), min_el_deg: 10.0 };
        let r = find_passes(&prop, &GroundSite::new(40.0, -75.0, 0.0).unwrap(), &s);
        assert!(matches!(r.error, Some(OrbitPropError::Sgp4(_))));
        assert!(r.passes.iter().all(|p| p.set.seconds_since(&tle.epoch) < 1500.0 * 60.0));
    }

    #[test]
    fn iss_passes_are_plausible() {
        let tle = Tle::parse(crate::tle::tests::ISS).unwrap();
        let prop = Sgp4Propagator::new(&tle).unwrap();
        let s = PassSearch { start: tle.epoch, end: tle.epoch.add_seconds(86_400.0), min_el_deg: 10.0 };
        let r = find_passes(&prop, &GroundSite::new(40.0, -75.0, 0.0).unwrap(), &s);
        assert_eq!(r.error, None);
        assert!(r.passes.len() >= 2, "{} passes", r.passes.len());
        for p in &r.passes {
            assert!(p.rise.seconds_since(&p.culmination) < 0.0 && p.culmination.seconds_since(&p.set) < 0.0);
            assert!(p.duration_s() > 30.0 && p.duration_s() < 15.0 * 60.0, "{}", p.duration_s());
            assert!(p.max_el_deg >= 10.0 && p.peak_rate_vs_ground_deg_s < 2.0);
        }
    }

    #[test]
    fn invalid_windows_are_rejected() {
        let sat = EarthFixedCircle { radius_km: 42_164.0, lon0_deg: 0.0, rate_deg_s: 0.0, fail_after_s: None };
        let bad = |s: PassSearch| matches!(find_passes(&sat, &equator(), &s).error, Some(OrbitPropError::InvalidTime(_)));
        assert!(bad(PassSearch { start: t0(), end: t0(), min_el_deg: 10.0 }), "empty window");
        assert!(bad(search(31.0 * 24.0, 10.0)), "over 30 days");
        assert!(bad(search(1.0, 90.0)), "min elevation 90");
        assert!(bad(search(1.0, f64::NAN)), "NaN min elevation");
    }

    #[test]
    fn summary_of_flags() {
        assert_eq!(summarize(&[true, true], 'a', 'n', 'm'), 'a');
        assert_eq!(summarize(&[false, false], 'a', 'n', 'm'), 'n');
        assert_eq!(summarize(&[true, false], 'a', 'n', 'm'), 'm');
    }
}
