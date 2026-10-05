//! The mount as the simulation loop sees it: something that takes axis
//! commands and reports where its axes are.
//!
//! The interface sits where a commercial mount's own does. Such a mount
//! closes its servo loop in firmware; the software tracking a satellite
//! sends it positions and rates and reads its encoders back. So the part
//! that swaps between simulation and hardware is the controller and the
//! axes together: `SimMount` is the controller of `servo.rs` driving ideal
//! axes, and a hardware driver would talk to a real mount.

use crate::clock::Tick;
use crate::error::SimError;
use crate::geometry::MountKind;
use crate::hardware::MountModel;
use crate::servo::{Axis, AxisLimits, AxisStep, Wrap};

/// What one axis is asked to do over the next interval: be at `pos_deg`
/// at its start, and turn at `rate_deg_s` through it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisCommand {
    pub pos_deg: f64,
    pub rate_deg_s: f64,
}

/// A command for both axes, for the interval starting at `t_s`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MountCommand {
    pub t_s: f64,
    pub axes: [AxisCommand; 2],
}

/// Where one axis is and what it is doing. The acceleration and the two
/// limit flags are what a simulated mount knows about itself; a hardware
/// driver reports what its controller does, or leaves them at zero and
/// false.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AxisState {
    pub pos_deg: f64,
    pub rate_deg_s: f64,
    pub accel_deg_s2: f64,
    pub rate_limited: bool,
    pub accel_limited: bool,
}

/// Both axes at `t_s`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MountState {
    pub t_s: f64,
    pub axes: [AxisState; 2],
}

impl MountState {
    pub fn rate_limited(&self) -> bool {
        self.axes.iter().any(|a| a.rate_limited)
    }
    pub fn accel_limited(&self) -> bool {
        self.axes.iter().any(|a| a.accel_limited)
    }
}

/// What a mount can do: how its axes are arranged and how they wrap, and
/// each axis's limits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MountCapabilities {
    pub kind: MountKind,
    pub wraps: [Wrap; 2],
    pub limits: [AxisLimits; 2],
}

/// How each axis of a mount of this kind wraps: azimuth all the way round,
/// hour angle at +/-180 deg, elevation and declination not at all.
pub fn axis_wraps(kind: MountKind) -> [Wrap; 2] {
    match kind {
        MountKind::AltAz => [Wrap::Full, Wrap::None],
        MountKind::Equatorial => [Wrap::Half, Wrap::None],
    }
}

pub trait MountDriver {
    fn capabilities(&self) -> MountCapabilities;
    /// Put both axes at `axes_deg`, at rest, ready for the run.
    fn prepare(&mut self, axes_deg: [f64; 2]) -> Result<MountState, SimError>;
    /// Start following `cmd`.
    fn command(&mut self, cmd: &MountCommand) -> Result<(), SimError>;
    /// The axes at the end of `tick`. A simulated mount moves through the
    /// tick under its latest command; a real one reads its encoders.
    fn read(&mut self, tick: Tick) -> Result<MountState, SimError>;
}

/// A simulated mount: each axis runs the controller in `servo.rs` within
/// its limits, with no backlash, resonance or quantization.
#[derive(Debug, Clone)]
pub struct SimMount {
    caps: MountCapabilities,
    gain_per_s: f64,
    axes: [Axis; 2],
    pending: Option<MountCommand>,
}

impl SimMount {
    pub fn new(model: &MountModel) -> SimMount {
        let wraps = axis_wraps(model.kind);
        SimMount {
            caps: MountCapabilities {
                kind: model.kind,
                wraps,
                limits: [0, 1].map(|i| AxisLimits {
                    max_rate: model.max_rate_deg_s[i].value,
                    max_accel: model.max_accel_deg_s2[i].value,
                }),
            },
            gain_per_s: model.servo_gain_per_s,
            axes: [Axis::at_rest(0.0, wraps[0]), Axis::at_rest(0.0, wraps[1])],
            pending: None,
        }
    }

    fn state(&self, t_s: f64, steps: [AxisStep; 2]) -> MountState {
        let axis = |i: usize| AxisState {
            pos_deg: self.axes[i].pos,
            rate_deg_s: self.axes[i].vel,
            accel_deg_s2: steps[i].accel,
            rate_limited: steps[i].rate_limited,
            accel_limited: steps[i].accel_limited,
        };
        MountState { t_s, axes: [axis(0), axis(1)] }
    }
}

impl MountDriver for SimMount {
    fn capabilities(&self) -> MountCapabilities {
        self.caps
    }

    fn prepare(&mut self, axes_deg: [f64; 2]) -> Result<MountState, SimError> {
        for (axis, pos) in self.axes.iter_mut().zip(axes_deg) {
            axis.pos = pos;
            axis.vel = 0.0;
        }
        self.pending = None;
        Ok(self.state(0.0, [AxisStep::default(); 2]))
    }

    fn command(&mut self, cmd: &MountCommand) -> Result<(), SimError> {
        self.pending = Some(*cmd);
        Ok(())
    }

    /// Steps each axis through `tick.dt_s` under the latest command. With
    /// no new command since the last read, the axes hold where they are.
    /// A tick with no duration moves nothing.
    fn read(&mut self, tick: Tick) -> Result<MountState, SimError> {
        if tick.dt_s.is_nan() || tick.dt_s <= 0.0 {
            return Ok(self.state(tick.t_s, [AxisStep::default(); 2]));
        }
        let hold = |a: &Axis| AxisCommand { pos_deg: a.pos, rate_deg_s: 0.0 };
        let cmd = self.pending.take().map_or([hold(&self.axes[0]), hold(&self.axes[1])], |c| c.axes);
        let mut steps = [AxisStep::default(); 2];
        for i in 0..2 {
            steps[i] = self.axes[i].step(cmd[i].pos_deg, cmd[i].rate_deg_s, tick.dt_s, self.caps.limits[i], self.gain_per_s);
        }
        Ok(self.state(tick.t_s, steps))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware::{Param, SERVO_GAIN_PER_S};

    fn model(rate: [f64; 2], accel: [f64; 2]) -> MountModel {
        MountModel {
            kind: MountKind::AltAz,
            kind_assumed: false,
            max_rate_deg_s: rate.map(Param::entered),
            max_accel_deg_s2: accel.map(Param::entered),
            pointing_rms_arcsec: Param::entered(0.0),
            jitter_rms_arcsec: Param::entered(0.0),
            servo_gain_per_s: SERVO_GAIN_PER_S,
        }
    }

    #[test]
    fn follows_a_constant_rate_command() {
        let mut m = SimMount::new(&model([10.0; 2], [10.0; 2]));
        m.prepare([100.0, 30.0]).unwrap();
        // Starting from rest, the axes catch up within a few 0.25 s time
        // constants and then follow with no lag.
        let dt = 0.02;
        let mut s = None;
        for k in 0..150 {
            let t = k as f64 * dt;
            let pos = |p0: f64, r: f64| AxisCommand { pos_deg: p0 + r * t, rate_deg_s: r };
            m.command(&MountCommand { t_s: t, axes: [pos(100.0, 0.5), pos(30.0, -0.2)] }).unwrap();
            s = Some(m.read(Tick { t_s: t + dt, dt_s: dt }).unwrap());
        }
        let s = s.unwrap();
        assert!((s.t_s - 3.0).abs() < 1e-9);
        assert!((s.axes[0].pos_deg - 101.5).abs() < 1e-4, "{:?}", s.axes[0]);
        assert!((s.axes[1].pos_deg - 29.4).abs() < 1e-4, "{:?}", s.axes[1]);
        assert!((s.axes[0].rate_deg_s - 0.5).abs() < 1e-4 && !s.rate_limited());
    }

    #[test]
    fn each_axis_keeps_its_own_limit() {
        let mut m = SimMount::new(&model([2.0, 0.5], [100.0; 2]));
        m.prepare([0.0, 0.0]).unwrap();
        let far = AxisCommand { pos_deg: 45.0, rate_deg_s: 0.0 };
        let mut s = None;
        for k in 0..200 {
            m.command(&MountCommand { t_s: k as f64 * 0.02, axes: [far, far] }).unwrap();
            s = Some(m.read(Tick { t_s: (k + 1) as f64 * 0.02, dt_s: 0.02 }).unwrap());
        }
        let s = s.unwrap();
        assert!((s.axes[0].rate_deg_s - 2.0).abs() < 1e-9 && (s.axes[1].rate_deg_s - 0.5).abs() < 1e-9, "{s:?}");
        assert!(s.rate_limited());
    }

    #[test]
    fn without_a_command_the_axes_hold() {
        let mut m = SimMount::new(&model([5.0; 2], [5.0; 2]));
        m.prepare([12.0, 34.0]).unwrap();
        let s = m.read(Tick { t_s: 0.02, dt_s: 0.02 }).unwrap();
        assert_eq!([s.axes[0].pos_deg, s.axes[1].pos_deg], [12.0, 34.0]);
        assert_eq!(m.capabilities().wraps, [Wrap::Full, Wrap::None]);
    }
}
