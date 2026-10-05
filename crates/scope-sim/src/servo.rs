//! One mount axis following a commanded trajectory within its limits.
//!
//! The controller is the usual one for a tracking mount: feed the
//! commanded rate forward, add a correction proportional to the position
//! error, and never ask for more rate or acceleration than the axis has.
//! The correction is also capped at about `sqrt(2 a e)`, the fastest
//! approach from which the axis can still stop at the target, so a large
//! error is closed without overshoot.

use crate::geometry::wrap180;

/// How an axis angle wraps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wrap {
    /// Elevation and declination: no wrap.
    None,
    /// Azimuth: `[0, 360)`.
    Full,
    /// Hour angle: `[-180, 180)`.
    Half,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisLimits {
    pub max_rate: f64,
    pub max_accel: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Axis {
    /// Angle, degrees.
    pub pos: f64,
    /// Rate, deg/s.
    pub vel: f64,
    pub wrap: Wrap,
}

/// What limited the axis during one step.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AxisStep {
    pub accel: f64,
    pub rate_limited: bool,
    pub accel_limited: bool,
}

impl Axis {
    pub fn at_rest(pos: f64, wrap: Wrap) -> Axis {
        Axis { pos, vel: 0.0, wrap }
    }

    /// Commanded minus actual, the short way round for a wrapping axis.
    pub fn error_to(&self, cmd_pos: f64) -> f64 {
        match self.wrap {
            Wrap::None => cmd_pos - self.pos,
            Wrap::Full | Wrap::Half => wrap180(cmd_pos - self.pos),
        }
    }

    /// Advance one step of `dt` seconds. `cmd_pos` is the commanded angle
    /// now (at the start of the step) and `cmd_vel` the commanded rate over
    /// the step, so a constant-rate command is followed with zero error.
    pub fn step(&mut self, cmd_pos: f64, cmd_vel: f64, dt: f64, lim: AxisLimits, gain: f64) -> AxisStep {
        let err = self.error_to(cmd_pos);
        // Fastest approach that can still stop at the target. For a
        // continuous axis that is sqrt(2 a e); stepping in time with the
        // updated rate covers an extra half step while braking, so the
        // discrete limit is the positive root of v^2 / 2a + v dt / 2 = e.
        let half_step = lim.max_accel * dt / 2.0;
        let stopping_vel = (half_step * half_step + 2.0 * lim.max_accel * err.abs()).sqrt() - half_step;
        let correction = err.signum() * (gain * err.abs()).min(stopping_vel);
        let wanted_vel = cmd_vel + correction;
        let rate_limited = wanted_vel.abs() > lim.max_rate;
        let target_vel = wanted_vel.clamp(-lim.max_rate, lim.max_rate);
        let wanted_accel = (target_vel - self.vel) / dt;
        let accel_limited = wanted_accel.abs() > lim.max_accel * (1.0 + 1e-9);
        let accel = wanted_accel.clamp(-lim.max_accel, lim.max_accel);
        self.vel += accel * dt;
        self.pos += self.vel * dt;
        self.pos = match self.wrap {
            Wrap::None => self.pos,
            Wrap::Full => self.pos.rem_euclid(360.0),
            Wrap::Half => wrap180(self.pos),
        };
        AxisStep { accel, rate_limited, accel_limited }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f64 = 0.02;
    const LIM: AxisLimits = AxisLimits { max_rate: 10.0, max_accel: 5.0 };

    #[test]
    fn follows_a_constant_rate_with_no_lag() {
        let mut a = Axis { pos: 0.0, vel: 1.0, wrap: Wrap::None };
        for k in 0..500 {
            let cmd = k as f64 * DT;
            let s = a.step(cmd, 1.0, DT, LIM, 4.0);
            assert!(!s.rate_limited && !s.accel_limited);
        }
        assert!((a.pos - 10.0).abs() < 1e-9);
    }

    #[test]
    fn closes_a_step_error_without_overshoot() {
        let mut a = Axis::at_rest(0.0, Wrap::None);
        let mut max = 0.0f64;
        for _ in 0..2000 {
            a.step(30.0, 0.0, DT, LIM, 4.0);
            max = max.max(a.pos);
        }
        assert!((a.pos - 30.0).abs() < 1e-6, "{}", a.pos);
        assert!(max <= 30.0 + 0.01, "overshoot to {max}");
    }

    #[test]
    fn never_exceeds_its_limits() {
        let mut a = Axis::at_rest(0.0, Wrap::None);
        let mut prev = a.vel;
        for k in 0..1000 {
            a.step(1000.0 * (k as f64 * 0.01).sin(), 100.0, DT, LIM, 4.0);
            assert!(a.vel.abs() <= LIM.max_rate + 1e-9);
            assert!(((a.vel - prev) / DT).abs() <= LIM.max_accel + 1e-6);
            prev = a.vel;
        }
    }

    #[test]
    fn azimuth_takes_the_short_way_round() {
        let mut a = Axis::at_rest(355.0, Wrap::Full);
        for _ in 0..500 {
            a.step(5.0, 0.0, DT, LIM, 4.0);
        }
        assert!((a.pos - 5.0).abs() < 1e-6, "{}", a.pos);
    }
}
