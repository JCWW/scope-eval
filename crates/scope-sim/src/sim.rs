//! The simulation loop.
//!
//! A run is assembled from four parts, each behind a trait so that any of
//! them can be simulated or real:
//!
//! * a `Clock` (`clock.rs`) that says when each step arrives,
//! * a `Tracker` (`tracker.rs`) that turns the prediction into axis commands,
//! * a `MountDriver` (`mount.rs`) that follows them and reports its axes,
//! * a `Sensor` (`sensor.rs`) that says where the target appears on the camera.
//!
//! Every `STEP_S` the tracker commands the mount, the clock waits for the
//! step, the mount reports, the sensor measures and the `Evaluator`
//! (`evaluator.rs`) accounts for it. `Simulation::new` assembles the fully
//! simulated set; `Simulation::from_parts` takes any other.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use orbit_prop::illumination::{lighting, Lighting};
use orbit_prop::sun_moon::sun_position_km;
use orbit_prop::{observe, Epoch, GroundSite, Observation, Propagator};

use crate::clock::{Clock, SimClock, Tick};
use crate::error::SimError;
use crate::evaluator::{assumed_figures, Evaluator};
use crate::geometry::{az_el_from_enu, enu_from_axes, MountKind, Vec3};
use crate::hardware::Hardware;
use crate::mount::{MountCommand, MountDriver, MountState, SimMount};
use crate::scenario::{passes_for, summarize, PassSummary, ScenarioSpec};
use crate::sensor::{Measurement, Sensor, SyntheticSensor};
use crate::telemetry::TelemetrySink;
use crate::tracker::{OpenLoopTracker, Tracker};

pub use crate::evaluator::{Summary, Verdict, FOV_PASS_FRACTION, FOV_WARN_FRACTION};
pub use crate::sensor::JITTER_CORRELATION_S;

/// Integration step, seconds. Short enough for a 4/s servo loop and for
/// the azimuth swing of a near-zenith LEO pass.
pub const STEP_S: f64 = 0.02;
/// Interval between recorded samples, seconds.
pub const RECORD_INTERVAL_S: f64 = 0.5;
/// Longest stretch of a pass that is simulated, seconds. Distant targets
/// can stay up for a day; four hours shows everything a mount will do.
pub const MAX_SIM_S: f64 = 4.0 * 3600.0;

/// Version of the telemetry this crate writes: `SimInfo`, `Sample`,
/// `Summary` and the trace file built from them. Raise it whenever a field
/// is renamed, removed or changes meaning; adding a field does not need it.
pub const SCHEMA_VERSION: u32 = 1;

const ARCSEC_PER_RAD: f64 = 206_264.806;

/// Whether the target is in sunlight, as `orbit_prop` judges it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetLighting {
    Sunlit,
    Penumbra,
    Umbra,
}

impl From<Lighting> for TargetLighting {
    fn from(l: Lighting) -> TargetLighting {
        match l {
            Lighting::Sunlit => TargetLighting::Sunlit,
            Lighting::Penumbra => TargetLighting::Penumbra,
            Lighting::Umbra => TargetLighting::Umbra,
        }
    }
}

/// One recorded moment of a run.
///
/// Where each field comes from: `axis*` from the mount, `axis*_cmd_rate`
/// from the tracker, `err_*` and `in_fov` from the sensor. `target_*` and
/// `boresight_*` come from the sensor's simulated truth when it has one;
/// otherwise the target is the prediction and the boresight is where the
/// axes point.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sample {
    /// Seconds since the start of the pass.
    pub t_s: f64,
    pub utc: String,
    /// Where the satellite really is.
    pub target_az_deg: f64,
    pub target_el_deg: f64,
    /// Where the telescope really points, pointing error included.
    pub boresight_az_deg: f64,
    pub boresight_el_deg: f64,
    /// Target position in the camera frame relative to the boresight.
    pub err_x_arcsec: f64,
    pub err_y_arcsec: f64,
    pub err_arcsec: f64,
    pub in_fov: bool,
    pub axis1_deg: f64,
    pub axis2_deg: f64,
    pub axis1_rate_deg_s: f64,
    pub axis2_rate_deg_s: f64,
    pub axis1_cmd_rate_deg_s: f64,
    pub axis2_cmd_rate_deg_s: f64,
    pub axis1_accel_deg_s2: f64,
    pub axis2_accel_deg_s2: f64,
    pub rate_limited: bool,
    pub accel_limited: bool,
    pub lighting: TargetLighting,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TrackPoint {
    pub t_s: f64,
    pub az_deg: f64,
    pub el_deg: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimInfo {
    /// `SCHEMA_VERSION` of the crate that wrote this.
    pub schema_version: u32,
    pub hardware: Hardware,
    pub target: String,
    pub pass: PassSummary,
    pub duration_s: f64,
    /// The pass is longer than `MAX_SIM_S` and only its start is simulated.
    pub truncated: bool,
    /// How far the true target runs ahead of the prediction, seconds.
    pub lead_s: f64,
    pub axis_names: [String; 2],
    pub step_s: f64,
    pub record_interval_s: f64,
    /// The pointing-model error drawn for this run, camera x and y. Only a
    /// simulated sky knows it.
    pub pointing_offset_arcsec: Option<[f64; 2]>,
    pub seed: u64,
}

/// Everything about a run that is the same whichever parts run it: the
/// hardware description, the prediction, the site and the pass.
pub struct RunSetup {
    pub hw: Hardware,
    /// The prediction the tracker follows.
    pub prop: Arc<dyn Propagator>,
    pub site: GroundSite,
    pub target_label: String,
    pub pass: PassSummary,
    pub rise: Epoch,
    pub duration_s: f64,
    /// The pass is longer than `MAX_SIM_S` and only its start is run.
    pub truncated: bool,
    /// How far the true target runs ahead of the prediction, seconds:
    /// the along-track ephemeris error over the orbital speed.
    pub lead_s: f64,
    pub seed: u64,
}

impl RunSetup {
    pub fn prepare(hw: Hardware, scenario: &ScenarioSpec, pass_index: usize) -> Result<RunSetup, SimError> {
        let crate::scenario::Prepared { prop, site, passes, .. } = passes_for(scenario)?;
        let pass = passes
            .get(pass_index)
            .ok_or_else(|| SimError::new(format!("there is no pass {} in the search window", pass_index + 1)))?;
        let speed = {
            let v = prop.propagate(pass.culmination)?.v_km_s;
            (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
        };
        Ok(RunSetup {
            target_label: prop.label().to_string(),
            pass: summarize(pass_index, pass),
            rise: pass.rise,
            duration_s: pass.duration_s().min(MAX_SIM_S),
            truncated: pass.duration_s() > MAX_SIM_S,
            lead_s: scenario.ephemeris_error_km / speed,
            seed: scenario.seed,
            prop: Arc::from(prop),
            site,
            hw,
        })
    }
}

/// The four swappable parts of a run.
pub struct Parts {
    pub clock: Box<dyn Clock>,
    pub tracker: Box<dyn Tracker>,
    pub mount: Box<dyn MountDriver>,
    pub sensor: Box<dyn Sensor>,
}

impl Parts {
    /// Everything simulated: simulated time, the open-loop tracker, the
    /// servo model of the mount and a synthetic sky.
    pub fn simulated(setup: &RunSetup) -> Parts {
        Parts {
            clock: Box::new(SimClock),
            tracker: Box::new(OpenLoopTracker::new(setup)),
            mount: Box::new(SimMount::new(&setup.hw.mount_model)),
            sensor: Box::new(SyntheticSensor::new(setup)),
        }
    }
}

/// State at the end of the latest step, enough to build a `Sample`.
#[derive(Debug, Clone, Copy)]
struct Latest {
    epoch: Epoch,
    target: Observation,
    target_r: Vec3,
    boresight: Vec3,
    mount: MountState,
    command: Option<MountCommand>,
    /// The latest measurement, which may be from an earlier step.
    measurement: Measurement,
}

pub struct Simulation {
    setup: RunSetup,
    parts: Parts,
    evaluator: Evaluator,
    sinks: Vec<Box<dyn TelemetrySink>>,
    t: f64,
    next_record: f64,
    samples: Vec<Sample>,
    latest: Latest,
}

impl Simulation {
    /// A fully simulated run of one pass.
    pub fn new(hw: Hardware, scenario: &ScenarioSpec, pass_index: usize) -> Result<Simulation, SimError> {
        let setup = RunSetup::prepare(hw, scenario, pass_index)?;
        let parts = Parts::simulated(&setup);
        Simulation::from_parts(setup, parts)
    }

    /// A run with the given parts. The tracker chooses where the run
    /// starts, the mount is put there at rest, and the sensor takes the
    /// first measurement.
    pub fn from_parts(setup: RunSetup, mut parts: Parts) -> Result<Simulation, SimError> {
        let start = parts.tracker.start()?;
        let mount = parts.mount.prepare(start)?;
        let measurement = parts
            .sensor
            .measure(Tick { t_s: 0.0, dt_s: 0.0 }, &mount)?
            .ok_or_else(|| SimError::new("the sensor gave no first measurement"))?;
        let evaluator = Evaluator::new(
            parts.mount.capabilities().limits,
            assumed_figures(&setup.hw.mount_model),
            Some(&measurement),
        );
        let latest = latest(&setup, 0.0, mount, None, measurement)?;
        let mut sim = Simulation {
            setup,
            parts,
            evaluator,
            sinks: Vec::new(),
            t: 0.0,
            next_record: 0.0,
            samples: Vec::new(),
            latest,
        };
        sim.record()?;
        Ok(sim)
    }

    /// Send every sample, those recorded so far and each new one, to `sink`.
    pub fn add_sink(&mut self, mut sink: Box<dyn TelemetrySink>) -> Result<(), SimError> {
        for s in &self.samples {
            sink.record(s)?;
        }
        self.sinks.push(sink);
        Ok(())
    }

    pub fn info(&self) -> SimInfo {
        let s = &self.setup;
        SimInfo {
            schema_version: SCHEMA_VERSION,
            hardware: s.hw.clone(),
            target: s.target_label.clone(),
            pass: s.pass.clone(),
            duration_s: s.duration_s,
            truncated: s.truncated,
            lead_s: s.lead_s,
            axis_names: match s.hw.mount_model.kind {
                MountKind::AltAz => ["Azimuth", "Elevation"],
                MountKind::Equatorial => ["Hour angle", "Declination"],
            }
            .map(String::from),
            step_s: STEP_S,
            record_interval_s: RECORD_INTERVAL_S,
            pointing_offset_arcsec: self.parts.sensor.pointing_offset_arcsec(),
            seed: s.seed,
        }
    }

    pub fn t_s(&self) -> f64 {
        self.t
    }

    pub fn duration_s(&self) -> f64 {
        self.setup.duration_s
    }

    pub fn is_done(&self) -> bool {
        self.t >= self.setup.duration_s - 1e-9
    }

    /// Every recorded sample so far.
    pub fn samples(&self) -> &[Sample] {
        &self.samples
    }

    /// The state right now, which may fall between recorded samples.
    pub fn current(&self) -> Sample {
        self.sample()
    }

    /// The predicted path across the sky, for drawing.
    pub fn track(&self, step_s: f64) -> Result<Vec<TrackPoint>, SimError> {
        let n = (self.setup.duration_s / step_s.max(0.1)).ceil() as usize;
        (0..=n)
            .map(|k| {
                let t = (k as f64 * step_s).min(self.setup.duration_s);
                let o = predicted(&self.setup, t)?;
                Ok(TrackPoint { t_s: t, az_deg: o.az_deg, el_deg: o.el_deg })
            })
            .collect()
    }

    /// Simulate `seconds` more (or to the end of the pass) and return the
    /// samples recorded on the way.
    pub fn advance(&mut self, seconds: f64) -> Result<&[Sample], SimError> {
        let first_new = self.samples.len();
        let until = (self.t + seconds.max(0.0)).min(self.setup.duration_s);
        while self.t < until - 1e-9 {
            self.step((until - self.t).min(STEP_S))?;
        }
        Ok(&self.samples[first_new..])
    }

    pub fn run_to_end(&mut self) -> Result<Summary, SimError> {
        self.advance(f64::INFINITY)?;
        Ok(self.summary())
    }

    pub fn summary(&self) -> Summary {
        self.evaluator.summary(self.t, self.is_done())
    }

    fn step(&mut self, dt: f64) -> Result<(), SimError> {
        let p = &mut self.parts;
        let command = p.tracker.command(self.t, dt, Some(&self.latest.measurement))?;
        p.mount.command(&command)?;
        let due = self.t + dt;
        let reached = p.clock.wait_until(due)?;
        // A simulated clock arrives exactly when due; keep the scheduled
        // step then, so simulated runs are free of rounding from `due - t`.
        let dt = if reached == due { dt } else { reached - self.t };
        self.t = reached;
        let tick = Tick { t_s: self.t, dt_s: dt };
        let mount = p.mount.read(tick)?;
        let measurement = p.sensor.measure(tick, &mount)?;
        self.evaluator.add(tick, &mount, measurement.as_ref());
        let measurement = measurement.unwrap_or(self.latest.measurement);
        self.latest = latest(&self.setup, self.t, mount, Some(command), measurement)?;

        if self.t >= self.next_record - 1e-9 || self.is_done() {
            self.record()?;
        }
        Ok(())
    }

    fn record(&mut self) -> Result<(), SimError> {
        let s = self.sample();
        for sink in &mut self.sinks {
            sink.record(&s)?;
        }
        self.samples.push(s);
        self.next_record += RECORD_INTERVAL_S;
        Ok(())
    }

    fn sample(&self) -> Sample {
        let l = &self.latest;
        let (bore_az, bore_el) = az_el_from_enu(l.boresight);
        let [a1, a2] = l.mount.axes;
        let cmd_rate = |i: usize| l.command.map_or(0.0, |c| c.axes[i].rate_deg_s);
        let err = l.measurement.err_rad;
        Sample {
            t_s: self.t,
            utc: l.epoch.to_string(),
            target_az_deg: l.target.az_deg,
            target_el_deg: l.target.el_deg,
            boresight_az_deg: bore_az,
            boresight_el_deg: bore_el,
            err_x_arcsec: err.0 * ARCSEC_PER_RAD,
            err_y_arcsec: err.1 * ARCSEC_PER_RAD,
            err_arcsec: err.0.hypot(err.1) * ARCSEC_PER_RAD,
            in_fov: l.measurement.in_fov,
            axis1_deg: a1.pos_deg,
            axis2_deg: a2.pos_deg,
            axis1_rate_deg_s: a1.rate_deg_s,
            axis2_rate_deg_s: a2.rate_deg_s,
            axis1_cmd_rate_deg_s: cmd_rate(0),
            axis2_cmd_rate_deg_s: cmd_rate(1),
            axis1_accel_deg_s2: a1.accel_deg_s2,
            axis2_accel_deg_s2: a2.accel_deg_s2,
            rate_limited: l.mount.rate_limited(),
            accel_limited: l.mount.accel_limited(),
            lighting: lighting(l.target_r, sun_position_km(l.epoch)).into(),
        }
    }
}

/// Where the prediction puts the target at `t_s`, and its TEME position.
fn predicted_state(setup: &RunSetup, t_s: f64) -> Result<(Observation, Vec3), SimError> {
    let state = setup.prop.propagate(setup.rise.add_seconds(t_s))?;
    Ok((observe(&state, &setup.site), state.r_km))
}

fn predicted(setup: &RunSetup, t_s: f64) -> Result<Observation, SimError> {
    Ok(predicted_state(setup, t_s)?.0)
}

/// The state at `t_s` for sampling. Without a simulated truth the target is
/// taken to be where it was predicted, and the boresight where the axes point.
fn latest(
    setup: &RunSetup,
    t_s: f64,
    mount: MountState,
    command: Option<MountCommand>,
    measurement: Measurement,
) -> Result<Latest, SimError> {
    let (target, target_r, boresight) = match measurement.truth {
        Some(truth) => (truth.target, truth.target_r_km, truth.boresight_enu),
        None => {
            let (target, r) = predicted_state(setup, t_s)?;
            let kind = setup.hw.mount_model.kind;
            let axes = enu_from_axes(kind, mount.axes[0].pos_deg, mount.axes[1].pos_deg, setup.site.lat_deg);
            (target, r, axes)
        }
    };
    Ok(Latest { epoch: setup.rise.add_seconds(t_s), target, target_r, boresight, mount, command, measurement })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware::{MountModel, Optics, Param};
    use crate::telemetry::JsonLinesSink;
    use std::cell::RefCell;
    use std::rc::Rc;
    use crate::scenario::tests::iss_scenario;
    use crate::scenario::find_passes;

    fn hardware(kind: MountKind, rate: f64, accel: f64, pointing: f64, jitter: f64) -> Hardware {
        Hardware {
            name: "test".into(),
            telescope: "DeltaRho 350".into(),
            camera: "IMX455".into(),
            mount: "test mount".into(),
            optics: Optics::new(350.0, 1050.0, 3.76, 9576, 6388),
            mount_model: MountModel {
                kind,
                kind_assumed: false,
                max_rate_deg_s: [Param::entered(rate); 2],
                max_accel_deg_s2: [Param::entered(accel); 2],
                pointing_rms_arcsec: Param::entered(pointing),
                jitter_rms_arcsec: Param::entered(jitter),
                servo_gain_per_s: crate::hardware::SERVO_GAIN_PER_S,
            },
        }
    }

    /// Index of the highest pass in the ISS scenario's window.
    fn highest_pass() -> usize {
        let list = find_passes(&iss_scenario()).unwrap();
        list.passes.iter().max_by(|a, b| a.max_el_deg.total_cmp(&b.max_el_deg)).unwrap().index
    }

    fn lowest_pass() -> usize {
        let list = find_passes(&iss_scenario()).unwrap();
        list.passes.iter().min_by(|a, b| a.max_el_deg.total_cmp(&b.max_el_deg)).unwrap().index
    }

    #[test]
    fn a_perfect_mount_has_no_error() {
        for kind in [MountKind::AltAz, MountKind::Equatorial] {
            let mut sim = Simulation::new(hardware(kind, 1000.0, 1000.0, 0.0, 0.0), &iss_scenario(), lowest_pass()).unwrap();
            let s = sim.run_to_end().unwrap();
            assert!(s.complete);
            assert!(s.max_err_arcsec < 1.0, "{kind:?}: max error {}", s.max_err_arcsec);
            assert_eq!(s.in_fov_fraction, 1.0);
            assert_eq!(s.verdict, Verdict::Pass);
        }
    }

    #[test]
    fn pointing_error_is_a_constant_offset() {
        let mut sim = Simulation::new(hardware(MountKind::AltAz, 1000.0, 1000.0, 60.0, 0.0), &iss_scenario(), lowest_pass()).unwrap();
        let [ox, oy] = sim.info().pointing_offset_arcsec.unwrap();
        let s = sim.run_to_end().unwrap();
        let expected = ox.hypot(oy);
        assert!(expected > 1.0, "drew a tiny offset: {expected}");
        assert!((s.rms_err_arcsec - expected).abs() < 1.0, "rms {} vs offset {expected}", s.rms_err_arcsec);
        for k in sim.samples() {
            assert!((k.err_x_arcsec + ox).abs() < 1.0 && (k.err_y_arcsec + oy).abs() < 1.0);
        }
    }

    #[test]
    fn along_track_error_shows_up_as_range_dependent_offset() {
        let mut scenario = iss_scenario();
        scenario.ephemeris_error_km = 2.0;
        let mut sim = Simulation::new(hardware(MountKind::AltAz, 1000.0, 1000.0, 0.0, 0.0), &scenario, highest_pass()).unwrap();
        assert!((sim.info().lead_s - 2.0 / 7.7).abs() < 0.02, "lead {}", sim.info().lead_s);
        let s = sim.run_to_end().unwrap();
        // The same 2 km is a bigger angle the closer the satellite is: at
        // most 2 / 350 km (about 1,180") overhead. At rise it is far away
        // and coming nearly straight at the site, so most of the error lies
        // along the line of sight and the angle is far smaller.
        assert!(s.max_err_arcsec > 900.0 && s.max_err_arcsec < 1300.0, "{}", s.max_err_arcsec);
        let samples = sim.samples();
        let first = samples[0].err_arcsec;
        let peak = samples.iter().map(|k| k.err_arcsec).fold(0.0, f64::max);
        assert!(first > 20.0 && first < 0.25 * peak, "first {first}, peak {peak}");
    }

    #[test]
    fn a_slow_mount_loses_a_high_pass_at_the_keyhole() {
        let pass = highest_pass();
        let list = find_passes(&iss_scenario()).unwrap();
        assert!(list.passes[pass].max_el_deg > 60.0, "need a high pass, got {}", list.passes[pass].max_el_deg);

        let mut fast = Simulation::new(hardware(MountKind::AltAz, 50.0, 10.0, 0.0, 0.0), &iss_scenario(), pass).unwrap();
        let fast = fast.run_to_end().unwrap();
        let mut slow = Simulation::new(hardware(MountKind::AltAz, 1.0, 0.5, 0.0, 0.0), &iss_scenario(), pass).unwrap();
        let slow = slow.run_to_end().unwrap();
        assert!(slow.max_err_arcsec > 10.0 * fast.max_err_arcsec.max(1.0), "slow {} fast {}", slow.max_err_arcsec, fast.max_err_arcsec);
        assert!(slow.rate_limited_s > 0.0);
        assert!(slow.in_fov_fraction < fast.in_fov_fraction);
    }

    #[test]
    fn the_same_seed_gives_the_same_run() {
        let run = |seed: u64| {
            let mut scenario = iss_scenario();
            scenario.seed = seed;
            let mut sim = Simulation::new(hardware(MountKind::AltAz, 50.0, 5.0, 30.0, 2.0), &scenario, lowest_pass()).unwrap();
            sim.run_to_end().unwrap()
        };
        assert_eq!(run(5), run(5));
        assert_ne!(run(5), run(6));
    }

    #[test]
    fn advance_records_samples_at_the_interval_and_stops_at_the_end() {
        let mut sim = Simulation::new(hardware(MountKind::AltAz, 50.0, 5.0, 0.0, 0.0), &iss_scenario(), lowest_pass()).unwrap();
        assert_eq!(sim.samples().len(), 1);
        let n = sim.advance(10.0).unwrap().len();
        assert_eq!(n, (10.0 / RECORD_INTERVAL_S) as usize);
        assert!((sim.t_s() - 10.0).abs() < 1e-9);
        sim.advance(1e6).unwrap();
        assert!(sim.is_done());
        assert!((sim.t_s() - sim.duration_s()).abs() < 1e-9);
        assert!(sim.advance(10.0).unwrap().is_empty());
        let track = sim.track(5.0).unwrap();
        assert!(track.iter().all(|p| p.el_deg > 9.0));
    }

    #[test]
    fn assumed_figures_cap_the_verdict_at_warn() {
        let mut hw = hardware(MountKind::AltAz, 1000.0, 1000.0, 0.0, 0.0);
        hw.mount_model.max_accel_deg_s2[1] = Param::assumed(1000.0);
        let mut sim = Simulation::new(hw, &iss_scenario(), lowest_pass()).unwrap();
        let s = sim.run_to_end().unwrap();
        assert_eq!(s.verdict, Verdict::Warn);
        assert!(s.verdict_reason.contains("maximum axis acceleration"));
    }

    #[test]
    fn a_missing_pass_is_an_error() {
        assert!(Simulation::new(hardware(MountKind::AltAz, 50.0, 5.0, 0.0, 0.0), &iss_scenario(), 99).is_err());
    }

    /// The synthetic sensor with its truth withheld, as a camera would be.
    struct NoTruth(SyntheticSensor);

    impl Sensor for NoTruth {
        fn measure(&mut self, tick: Tick, mount: &MountState) -> Result<Option<Measurement>, SimError> {
            Ok(self.0.measure(tick, mount)?.map(|m| Measurement { truth: None, ..m }))
        }
    }

    /// Measures only every `every`th tick after the first.
    struct Sometimes {
        inner: SyntheticSensor,
        every: u32,
        ticks: u32,
    }

    impl Sensor for Sometimes {
        fn measure(&mut self, tick: Tick, mount: &MountState) -> Result<Option<Measurement>, SimError> {
            let m = self.inner.measure(tick, mount)?;
            self.ticks += 1;
            Ok(if self.ticks == 1 || self.ticks % self.every == 0 { m } else { None })
        }
    }

    /// Wakes a millisecond late every step, as a real-time clock might.
    struct LateClock;

    impl Clock for LateClock {
        fn wait_until(&mut self, t_s: f64) -> Result<f64, SimError> {
            Ok(t_s + 0.001)
        }
    }

    /// Hands every recorded sample to a shared list.
    struct Shared(Rc<RefCell<Vec<f64>>>);

    impl TelemetrySink for Shared {
        fn record(&mut self, sample: &Sample) -> Result<(), SimError> {
            self.0.borrow_mut().push(sample.t_s);
            Ok(())
        }
    }

    fn with_parts(hw: Hardware, change: impl FnOnce(&RunSetup, &mut Parts)) -> Simulation {
        let setup = RunSetup::prepare(hw, &iss_scenario(), lowest_pass()).unwrap();
        let mut parts = Parts::simulated(&setup);
        change(&setup, &mut parts);
        Simulation::from_parts(setup, parts).unwrap()
    }

    #[test]
    fn new_is_from_parts_with_the_simulated_parts() {
        let hw = hardware(MountKind::Equatorial, 3.0, 2.0, 30.0, 2.0);
        let mut a = Simulation::new(hw.clone(), &iss_scenario(), lowest_pass()).unwrap();
        let mut b = with_parts(hw, |_, _| {});
        assert_eq!(a.run_to_end().unwrap(), b.run_to_end().unwrap());
        assert_eq!(a.samples(), b.samples());
    }

    #[test]
    fn without_truth_the_grade_is_the_same_and_the_target_is_the_prediction() {
        // No pointing error, jitter or ephemeris error: the prediction is
        // the truth and the axes point where the boresight does.
        let hw = hardware(MountKind::AltAz, 50.0, 10.0, 0.0, 0.0);
        let mut truth = Simulation::new(hw.clone(), &iss_scenario(), lowest_pass()).unwrap();
        let mut blind = with_parts(hw, |setup, parts| parts.sensor = Box::new(NoTruth(SyntheticSensor::new(setup))));
        assert_eq!(truth.run_to_end().unwrap(), blind.run_to_end().unwrap());
        assert_eq!(blind.info().pointing_offset_arcsec, None);
        for (a, b) in truth.samples().iter().zip(blind.samples()) {
            assert!((a.target_az_deg - b.target_az_deg).abs() < 1e-9 && (a.target_el_deg - b.target_el_deg).abs() < 1e-9);
            assert!((a.boresight_el_deg - b.boresight_el_deg).abs() < 1e-6, "{} vs {}", a.boresight_el_deg, b.boresight_el_deg);
            assert_eq!(a.err_arcsec, b.err_arcsec);
        }
    }

    #[test]
    fn missing_measurements_count_only_the_time_measured() {
        let hw = hardware(MountKind::AltAz, 50.0, 10.0, 30.0, 1.0);
        let mut sim = with_parts(hw, |setup, parts| {
            parts.sensor = Box::new(Sometimes { inner: SyntheticSensor::new(setup), every: 7, ticks: 0 })
        });
        // Between measurements the state repeats the latest one: of 14
        // steps, two bring a new measurement.
        let errs: Vec<f64> = (0..14).map(|_| {
            sim.advance(STEP_S).unwrap();
            sim.current().err_arcsec
        }).collect();
        let changes = errs.windows(2).filter(|w| w[0] != w[1]).count();
        assert_eq!(changes, 2, "{errs:?}");
        let s = sim.run_to_end().unwrap();
        assert!(s.complete);
        assert_eq!(s.in_fov_fraction, 1.0);
        assert!(s.rms_err_arcsec > 1.0);
    }

    #[test]
    fn a_late_clock_steps_by_the_time_that_really_passed() {
        let hw = hardware(MountKind::AltAz, 50.0, 10.0, 0.0, 0.0);
        let mut sim = with_parts(hw, |_, parts| parts.clock = Box::new(LateClock));
        sim.advance(1.0).unwrap();
        // Each 0.02 s step lands 1 ms late, so a second takes 20 steps, not 50.
        assert!(sim.t_s() >= 1.0 && sim.t_s() < 1.03, "{}", sim.t_s());
        let s = sim.run_to_end().unwrap();
        assert!(s.complete && s.max_err_arcsec < 1.0, "{s:?}");
    }

    #[test]
    fn a_sink_gets_every_sample_including_those_before_it_was_added() {
        let hw = hardware(MountKind::AltAz, 50.0, 10.0, 0.0, 0.0);
        let mut sim = Simulation::new(hw, &iss_scenario(), lowest_pass()).unwrap();
        sim.advance(3.0).unwrap();
        let seen = Rc::new(RefCell::new(Vec::new()));
        sim.add_sink(Box::new(Shared(Rc::clone(&seen)))).unwrap();
        sim.run_to_end().unwrap();
        let times: Vec<f64> = sim.samples().iter().map(|s| s.t_s).collect();
        assert_eq!(*seen.borrow(), times);
    }

    #[test]
    fn json_lines_sink_writes_one_sample_per_line() {
        let hw = hardware(MountKind::AltAz, 50.0, 10.0, 0.0, 0.0);
        let sim = Simulation::new(hw, &iss_scenario(), lowest_pass()).unwrap();
        let mut sink = JsonLinesSink::new(Vec::new());
        sink.record(&sim.current()).unwrap();
        let text = String::from_utf8(sink.into_inner()).unwrap();
        let back: Sample = serde_json::from_str(text.trim_end()).unwrap();
        assert_eq!(back, sim.current());
    }
}
