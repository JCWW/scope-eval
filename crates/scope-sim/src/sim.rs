//! The simulation loop.
//!
//! Every `STEP_S` the mount reads the *predicted* direction of the target
//! (where the ephemeris says it is), converts it to axis angles, and each
//! axis steps toward it within its limits (see `servo`). The *true* target
//! runs `lead_s` ahead of the prediction, to model along-track ephemeris
//! error. The boresight is wherever the axes point, displaced by a fixed
//! pointing-model error drawn once per run and by short-term jitter. The
//! pointing error is the true target's position in the camera frame.

use serde::{Deserialize, Serialize};

use orbit_prop::illumination::{lighting, Lighting};
use orbit_prop::sun_moon::sun_position_km;
use orbit_prop::{observe, Epoch, GroundSite, Observation, Propagator};

use crate::error::SimError;
use crate::geometry::{
    axes_from_enu, az_el_from_enu, camera_offset, enu_from_axes, enu_from_az_el, normalize, tangent_basis, wrap180,
    MountKind, Vec3,
};
use crate::hardware::{Hardware, Param};
use crate::rng::Rng;
use crate::scenario::{passes_for, summarize, PassSummary, ScenarioSpec};
use crate::servo::{Axis, AxisLimits, AxisStep, Wrap};

/// Integration step, seconds. Short enough for a 4/s servo loop and for
/// the azimuth swing of a near-zenith LEO pass.
pub const STEP_S: f64 = 0.02;
/// Interval between recorded samples, seconds.
pub const RECORD_INTERVAL_S: f64 = 0.5;
/// Longest stretch of a pass that is simulated, seconds. Distant targets
/// can stay up for a day; four hours shows everything a mount will do.
pub const MAX_SIM_S: f64 = 4.0 * 3600.0;
/// Correlation time of the tracking jitter, seconds.
pub const JITTER_CORRELATION_S: f64 = 0.5;
/// Fraction of the pass in the field needed for PASS and for WARN.
pub const FOV_PASS_FRACTION: f64 = 0.99;
pub const FOV_WARN_FRACTION: f64 = 0.90;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    /// Seconds simulated so far.
    pub t_s: f64,
    pub complete: bool,
    pub rms_err_arcsec: f64,
    pub max_err_arcsec: f64,
    pub max_err_at_s: f64,
    pub in_fov_fraction: f64,
    /// Times the target left the field.
    pub fov_exits: u32,
    pub rate_limited_s: f64,
    pub accel_limited_s: f64,
    /// Peak |rate| / maximum rate, per axis.
    pub peak_rate_utilization: [f64; 2],
    /// Peak |acceleration| / maximum acceleration, per axis.
    pub peak_accel_utilization: [f64; 2],
    pub verdict: Verdict,
    pub verdict_reason: String,
}

/// State at the end of the latest step, enough to build a `Sample`.
#[derive(Debug, Clone, Copy)]
struct Latest {
    epoch: Epoch,
    target: Observation,
    target_r: Vec3,
    boresight: Vec3,
    err: (f64, f64),
    in_fov: bool,
    cmd_vel: [f64; 2],
    steps: [AxisStep; 2],
}

#[derive(Debug, Clone, Default)]
struct Stats {
    sum_sq: f64,
    max: f64,
    max_at: f64,
    in_fov_s: f64,
    exits: u32,
    rate_limited_s: f64,
    accel_limited_s: f64,
    peak_rate: [f64; 2],
    peak_accel: [f64; 2],
}

pub struct Simulation {
    hw: Hardware,
    prop: Box<dyn Propagator>,
    site: GroundSite,
    target_label: String,
    pass: PassSummary,
    rise: Epoch,
    duration_s: f64,
    truncated: bool,
    lead_s: f64,
    seed: u64,
    limits: [AxisLimits; 2],
    axes: [Axis; 2],
    cmd: (f64, f64),
    offset: (f64, f64),
    jitter: (f64, f64),
    jitter_sigma: f64,
    rng: Rng,
    t: f64,
    next_record: f64,
    samples: Vec<Sample>,
    /// Always `Some` once `new` returns.
    latest: Option<Latest>,
    stats: Stats,
}

impl Simulation {
    pub fn new(hw: Hardware, scenario: &ScenarioSpec, pass_index: usize) -> Result<Simulation, SimError> {
        let crate::scenario::Prepared { prop, site, passes, .. } = passes_for(scenario)?;
        let pass = passes
            .get(pass_index)
            .ok_or_else(|| SimError::new(format!("there is no pass {} in the search window", pass_index + 1)))?;
        let rise = pass.rise;
        let duration_s = pass.duration_s().min(MAX_SIM_S);
        let speed = {
            let v = prop.propagate(pass.culmination)?.v_km_s;
            (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
        };
        let lead_s = scenario.ephemeris_error_km / speed;

        let m = &hw.mount_model;
        let limits = [0, 1].map(|i| AxisLimits { max_rate: m.max_rate_deg_s[i].value, max_accel: m.max_accel_deg_s2[i].value });
        let wrap1 = match m.kind {
            MountKind::AltAz => Wrap::Full,
            MountKind::Equatorial => Wrap::Half,
        };
        let mut rng = Rng::new(scenario.seed);
        let per_axis = |rms_arcsec: f64| rms_arcsec / ARCSEC_PER_RAD / std::f64::consts::SQRT_2;
        let pointing_sigma = per_axis(m.pointing_rms_arcsec.value);
        let jitter_sigma = per_axis(m.jitter_rms_arcsec.value);
        let offset = (pointing_sigma * rng.gaussian(), pointing_sigma * rng.gaussian());
        let jitter = (jitter_sigma * rng.gaussian(), jitter_sigma * rng.gaussian());

        let mut sim = Simulation {
            target_label: prop.label().to_string(),
            pass: summarize(pass_index, pass),
            hw,
            prop,
            site,
            rise,
            duration_s,
            truncated: pass.duration_s() > MAX_SIM_S,
            lead_s,
            seed: scenario.seed,
            limits,
            axes: [Axis::at_rest(0.0, wrap1), Axis::at_rest(0.0, Wrap::None)],
            cmd: (0.0, 0.0),
            offset,
            jitter,
            jitter_sigma,
            rng,
            t: 0.0,
            next_record: 0.0,
            samples: Vec::new(),
            latest: None,
            stats: Stats::default(),
        };
        // Start on target and at rest: the mount has slewed to the rise
        // point and is waiting.
        sim.cmd = sim.commanded_axes(0.0)?;
        sim.axes[0].pos = sim.cmd.0;
        sim.axes[1].pos = sim.cmd.1;
        sim.measure([0.0; 2], [AxisStep::default(); 2])?;
        sim.stats = Stats::default();
        sim.record();
        Ok(sim)
    }

    pub fn info(&self) -> SimInfo {
        SimInfo {
            schema_version: SCHEMA_VERSION,
            hardware: self.hw.clone(),
            target: self.target_label.clone(),
            pass: self.pass.clone(),
            duration_s: self.duration_s,
            truncated: self.truncated,
            lead_s: self.lead_s,
            axis_names: match self.hw.mount_model.kind {
                MountKind::AltAz => ["Azimuth", "Elevation"],
                MountKind::Equatorial => ["Hour angle", "Declination"],
            }
            .map(String::from),
            step_s: STEP_S,
            record_interval_s: RECORD_INTERVAL_S,
            pointing_offset_arcsec: Some([self.offset.0 * ARCSEC_PER_RAD, self.offset.1 * ARCSEC_PER_RAD]),
            seed: self.seed,
        }
    }

    pub fn t_s(&self) -> f64 {
        self.t
    }

    pub fn duration_s(&self) -> f64 {
        self.duration_s
    }

    pub fn is_done(&self) -> bool {
        self.t >= self.duration_s - 1e-9
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
        let n = (self.duration_s / step_s.max(0.1)).ceil() as usize;
        (0..=n)
            .map(|k| {
                let t = (k as f64 * step_s).min(self.duration_s);
                let o = self.look(t)?;
                Ok(TrackPoint { t_s: t, az_deg: o.az_deg, el_deg: o.el_deg })
            })
            .collect()
    }

    /// Simulate `seconds` more (or to the end of the pass) and return the
    /// samples recorded on the way.
    pub fn advance(&mut self, seconds: f64) -> Result<&[Sample], SimError> {
        let first_new = self.samples.len();
        let until = (self.t + seconds.max(0.0)).min(self.duration_s);
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
        let s = &self.stats;
        let t = self.t.max(1e-9);
        let in_fov_fraction = if self.t > 0.0 { (s.in_fov_s / t).min(1.0) } else { 1.0 };
        let m = &self.hw.mount_model;
        let mut assumed = Vec::new();
        if m.max_rate_deg_s.iter().any(Param::is_assumed) {
            assumed.push("maximum axis rate");
        }
        if m.max_accel_deg_s2.iter().any(Param::is_assumed) {
            assumed.push("maximum axis acceleration");
        }
        if m.pointing_rms_arcsec.is_assumed() {
            assumed.push("pointing RMS");
        }
        let pct = in_fov_fraction * 100.0;
        let (mut verdict, mut verdict_reason) = if in_fov_fraction >= FOV_PASS_FRACTION {
            (Verdict::Pass, format!("The target stayed in the field for {pct:.1}% of the pass."))
        } else if in_fov_fraction >= FOV_WARN_FRACTION {
            (Verdict::Warn, format!("The target left the field {} time(s); in the field {pct:.1}% of the pass.", s.exits))
        } else {
            (Verdict::Fail, format!("The target was in the field for only {pct:.1}% of the pass."))
        };
        // As in scope-eval: never PASS on numbers nobody entered.
        if verdict == Verdict::Pass && !assumed.is_empty() {
            verdict = Verdict::Warn;
            verdict_reason.push_str(&format!(" Not graded PASS because these were assumed: {}.", assumed.join(", ")));
        }
        Summary {
            t_s: self.t,
            complete: self.is_done(),
            rms_err_arcsec: (s.sum_sq / t).sqrt() * ARCSEC_PER_RAD,
            max_err_arcsec: s.max * ARCSEC_PER_RAD,
            max_err_at_s: s.max_at,
            in_fov_fraction,
            fov_exits: s.exits,
            rate_limited_s: s.rate_limited_s,
            accel_limited_s: s.accel_limited_s,
            peak_rate_utilization: [s.peak_rate[0] / self.limits[0].max_rate, s.peak_rate[1] / self.limits[1].max_rate],
            peak_accel_utilization: [
                s.peak_accel[0] / self.limits[0].max_accel,
                s.peak_accel[1] / self.limits[1].max_accel,
            ],
            verdict,
            verdict_reason,
        }
    }

    fn look(&self, t_s: f64) -> Result<Observation, SimError> {
        Ok(observe(&self.prop.propagate(self.rise.add_seconds(t_s))?, &self.site))
    }

    /// Axis angles that point at the predicted position at `t_s`.
    fn commanded_axes(&self, t_s: f64) -> Result<(f64, f64), SimError> {
        let o = self.look(t_s)?;
        Ok(axes_from_enu(self.hw.mount_model.kind, enu_from_az_el(o.az_deg, o.el_deg), self.site.lat_deg))
    }

    fn step(&mut self, dt: f64) -> Result<(), SimError> {
        let gain = self.hw.mount_model.servo_gain_per_s;
        let next = self.commanded_axes(self.t + dt)?;
        let d1 = match self.axes[0].wrap {
            Wrap::None => next.0 - self.cmd.0,
            Wrap::Full | Wrap::Half => wrap180(next.0 - self.cmd.0),
        };
        let cmd_vel = [d1 / dt, (next.1 - self.cmd.1) / dt];
        let steps = [
            self.axes[0].step(self.cmd.0, cmd_vel[0], dt, self.limits[0], gain),
            self.axes[1].step(self.cmd.1, cmd_vel[1], dt, self.limits[1], gain),
        ];
        self.cmd = next;
        self.t += dt;

        // First-order Gauss-Markov jitter with the requested RMS.
        let phi = (-dt / JITTER_CORRELATION_S).exp();
        let kick = self.jitter_sigma * (1.0 - phi * phi).sqrt();
        self.jitter = (
            self.jitter.0 * phi + kick * self.rng.gaussian(),
            self.jitter.1 * phi + kick * self.rng.gaussian(),
        );

        let was_in = self.latest.map_or(true, |l| l.in_fov);
        let l = self.measure(cmd_vel, steps)?;
        let s = &mut self.stats;
        let err = l.err.0.hypot(l.err.1);
        s.sum_sq += err * err * dt;
        if err > s.max {
            s.max = err;
            s.max_at = self.t;
        }
        if l.in_fov {
            s.in_fov_s += dt;
        } else if was_in {
            s.exits += 1;
        }
        if steps.iter().any(|k| k.rate_limited) {
            s.rate_limited_s += dt;
        }
        if steps.iter().any(|k| k.accel_limited) {
            s.accel_limited_s += dt;
        }
        for (i, (axis, step)) in self.axes.iter().zip(steps).enumerate() {
            s.peak_rate[i] = s.peak_rate[i].max(axis.vel.abs());
            s.peak_accel[i] = s.peak_accel[i].max(step.accel.abs());
        }

        if self.t >= self.next_record - 1e-9 || self.is_done() {
            self.record();
        }
        Ok(())
    }

    /// Where the true target appears relative to the boresight now.
    fn measure(&mut self, cmd_vel: [f64; 2], steps: [AxisStep; 2]) -> Result<Latest, SimError> {
        let kind = self.hw.mount_model.kind;
        let lat = self.site.lat_deg;
        let epoch = self.rise.add_seconds(self.t);
        let state = self.prop.propagate(epoch.add_seconds(self.lead_s))?;
        let target = observe(&state, &self.site);
        let target_dir = enu_from_az_el(target.az_deg, target.el_deg);
        let (a1, a2) = (self.axes[0].pos, self.axes[1].pos);
        let axis_dir = enu_from_axes(kind, a1, a2, lat);
        let (x, y) = tangent_basis(kind, a1, a2, lat);
        let (ox, oy) = (self.offset.0 + self.jitter.0, self.offset.1 + self.jitter.1);
        let (tx, ty) = camera_offset(axis_dir, x, y, target_dir);
        let err = (tx - ox, ty - oy);
        let boresight = normalize([
            axis_dir[0] + ox * x[0] + oy * y[0],
            axis_dir[1] + ox * x[1] + oy * y[1],
            axis_dir[2] + ox * x[2] + oy * y[2],
        ]);
        let (half_w, half_h) = self.hw.optics.half_fov_rad();
        // `camera_offset` puts a target behind the boresight near +/-180
        // deg, so it can never pass this test.
        let in_fov = err.0.abs() <= half_w && err.1.abs() <= half_h;
        let latest = Latest { epoch, target, target_r: state.r_km, boresight, err, in_fov, cmd_vel, steps };
        self.latest = Some(latest);
        Ok(latest)
    }

    fn record(&mut self) {
        let s = self.sample();
        self.samples.push(s);
        self.next_record += RECORD_INTERVAL_S;
    }

    fn sample(&self) -> Sample {
        let l = self.latest.as_ref().expect("measured in new");
        let (bore_az, bore_el) = az_el_from_enu(l.boresight);
        let lit = lighting(l.target_r, sun_position_km(l.epoch)).into();
        Sample {
            t_s: self.t,
            utc: l.epoch.to_string(),
            target_az_deg: l.target.az_deg,
            target_el_deg: l.target.el_deg,
            boresight_az_deg: bore_az,
            boresight_el_deg: bore_el,
            err_x_arcsec: l.err.0 * ARCSEC_PER_RAD,
            err_y_arcsec: l.err.1 * ARCSEC_PER_RAD,
            err_arcsec: l.err.0.hypot(l.err.1) * ARCSEC_PER_RAD,
            in_fov: l.in_fov,
            axis1_deg: self.axes[0].pos,
            axis2_deg: self.axes[1].pos,
            axis1_rate_deg_s: self.axes[0].vel,
            axis2_rate_deg_s: self.axes[1].vel,
            axis1_cmd_rate_deg_s: l.cmd_vel[0],
            axis2_cmd_rate_deg_s: l.cmd_vel[1],
            axis1_accel_deg_s2: l.steps[0].accel,
            axis2_accel_deg_s2: l.steps[1].accel,
            rate_limited: l.steps.iter().any(|k| k.rate_limited),
            accel_limited: l.steps.iter().any(|k| k.accel_limited),
            lighting: lit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware::{MountModel, Optics};
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
}
