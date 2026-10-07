//! Statistics and the verdict for a run.
//!
//! The evaluator sees only what any setup can provide: the measurements
//! and the mount's reported state. It never looks at the simulated truth,
//! so the same grading applies when the hardware is real.

use serde::{Deserialize, Serialize};

use crate::clock::Tick;
use crate::hardware::{MountModel, Param};
use crate::mount::MountState;
use crate::sensor::Measurement;
use crate::servo::AxisLimits;

/// Fraction of the pass in the field needed for PASS and for WARN.
pub const FOV_PASS_FRACTION: f64 = 0.99;
pub const FOV_WARN_FRACTION: f64 = 0.90;

const ARCSEC_PER_RAD: f64 = 206_264.806;

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

/// The mount figures that were assumed rather than entered, named as the
/// verdict names them.
pub fn assumed_figures(m: &MountModel) -> Vec<&'static str> {
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
    assumed
}

#[derive(Debug, Clone)]
pub struct Evaluator {
    limits: [AxisLimits; 2],
    assumed: Vec<&'static str>,
    /// Seconds with a measurement, which the error statistics cover.
    measured_s: f64,
    sum_sq: f64,
    max: f64,
    max_at: f64,
    in_fov_s: f64,
    exits: u32,
    was_in_fov: bool,
    rate_limited_s: f64,
    accel_limited_s: f64,
    peak_rate: [f64; 2],
    peak_accel: [f64; 2],
}

impl Evaluator {
    /// `limits` are the mount's, for utilization; `assumed` names the
    /// figures that keep a run from grading PASS; `initial` is the
    /// measurement before the first step, which decides whether the first
    /// step out of the field counts as an exit.
    pub fn new(limits: [AxisLimits; 2], assumed: Vec<&'static str>, initial: Option<&Measurement>) -> Evaluator {
        Evaluator {
            limits,
            assumed,
            measured_s: 0.0,
            sum_sq: 0.0,
            max: 0.0,
            max_at: 0.0,
            in_fov_s: 0.0,
            exits: 0,
            was_in_fov: initial.map_or(true, |m| m.in_fov),
            rate_limited_s: 0.0,
            accel_limited_s: 0.0,
            peak_rate: [0.0; 2],
            peak_accel: [0.0; 2],
        }
    }

    /// Account for one step. A step without a measurement adds to the
    /// mount statistics only.
    pub fn add(&mut self, tick: Tick, mount: &MountState, measurement: Option<&Measurement>) {
        let dt = tick.dt_s;
        if let Some(m) = measurement {
            let err = m.err_rad.0.hypot(m.err_rad.1);
            self.measured_s += dt;
            self.sum_sq += err * err * dt;
            if err > self.max {
                self.max = err;
                self.max_at = tick.t_s;
            }
            if m.in_fov {
                self.in_fov_s += dt;
            } else if self.was_in_fov {
                self.exits += 1;
            }
            self.was_in_fov = m.in_fov;
        }
        if mount.rate_limited() {
            self.rate_limited_s += dt;
        }
        if mount.accel_limited() {
            self.accel_limited_s += dt;
        }
        for (i, axis) in mount.axes.iter().enumerate() {
            self.peak_rate[i] = self.peak_rate[i].max(axis.rate_deg_s.abs());
            self.peak_accel[i] = self.peak_accel[i].max(axis.accel_deg_s2.abs());
        }
    }

    /// The summary after `t_s` seconds of the run.
    pub fn summary(&self, t_s: f64, complete: bool) -> Summary {
        let measured = self.measured_s.max(1e-9);
        let in_fov_fraction = if self.measured_s > 0.0 { (self.in_fov_s / measured).min(1.0) } else { 1.0 };
        let pct = in_fov_fraction * 100.0;
        let (mut verdict, mut verdict_reason) = if in_fov_fraction >= FOV_PASS_FRACTION {
            (Verdict::Pass, format!("The target stayed in the field for {pct:.1}% of the pass."))
        } else if in_fov_fraction >= FOV_WARN_FRACTION {
            (Verdict::Warn, format!("The target left the field {} time(s); in the field {pct:.1}% of the pass.", self.exits))
        } else {
            (Verdict::Fail, format!("The target was in the field for only {pct:.1}% of the pass."))
        };
        // As in scope-eval: never PASS on numbers nobody entered.
        if verdict == Verdict::Pass && !self.assumed.is_empty() {
            verdict = Verdict::Warn;
            verdict_reason.push_str(&format!(" Not graded PASS because these were assumed: {}.", self.assumed.join(", ")));
        }
        let l = &self.limits;
        Summary {
            t_s,
            complete,
            rms_err_arcsec: (self.sum_sq / measured).sqrt() * ARCSEC_PER_RAD,
            max_err_arcsec: self.max * ARCSEC_PER_RAD,
            max_err_at_s: self.max_at,
            in_fov_fraction,
            fov_exits: self.exits,
            rate_limited_s: self.rate_limited_s,
            accel_limited_s: self.accel_limited_s,
            peak_rate_utilization: [self.peak_rate[0] / l[0].max_rate, self.peak_rate[1] / l[1].max_rate],
            peak_accel_utilization: [self.peak_accel[0] / l[0].max_accel, self.peak_accel[1] / l[1].max_accel],
            verdict,
            verdict_reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mount::AxisState;

    const LIM: AxisLimits = AxisLimits { max_rate: 2.0, max_accel: 1.0 };

    fn mount(rate: f64) -> MountState {
        MountState { t_s: 0.0, axes: [AxisState { rate_deg_s: rate, ..Default::default() }, AxisState::default()] }
    }

    fn seen(t_s: f64, err_arcsec: f64, in_fov: bool) -> Measurement {
        Measurement { t_s, err_rad: (err_arcsec / ARCSEC_PER_RAD, 0.0), in_fov, truth: None }
    }

    #[test]
    fn steps_without_a_measurement_do_not_count_against_the_field() {
        let mut e = Evaluator::new([LIM; 2], vec![], None);
        for k in 1..=10 {
            let t = k as f64;
            // Every other step has no measurement, as with a slow camera.
            let m = (k % 2 == 0).then(|| seen(t, 10.0, true));
            e.add(Tick { t_s: t, dt_s: 1.0 }, &mount(1.0), m.as_ref());
        }
        let s = e.summary(10.0, true);
        assert_eq!(s.in_fov_fraction, 1.0);
        assert!((s.rms_err_arcsec - 10.0).abs() < 1e-9);
        assert_eq!(s.verdict, Verdict::Pass);
        assert!((s.peak_rate_utilization[0] - 0.5).abs() < 1e-12);
    }

    #[test]
    fn counts_exits_and_grades_by_time_in_field() {
        let mut e = Evaluator::new([LIM; 2], vec!["pointing RMS"], Some(&seen(0.0, 0.0, true)));
        let pattern = [true, false, false, true, false, true, true, true, true, true];
        for (k, &in_fov) in pattern.iter().enumerate() {
            e.add(Tick { t_s: k as f64 + 1.0, dt_s: 1.0 }, &mount(0.0), Some(&seen(0.0, 1.0, in_fov)));
        }
        let s = e.summary(10.0, true);
        assert_eq!(s.fov_exits, 2);
        assert!((s.in_fov_fraction - 0.7).abs() < 1e-12);
        assert_eq!(s.verdict, Verdict::Fail);
    }
}
