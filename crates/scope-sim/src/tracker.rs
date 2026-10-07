//! The tracking software: what the mount is told to do.
//!
//! This is the part under test in every setup, simulated or not.
//! `OpenLoopTracker` follows the prediction alone. A closed-loop tracker
//! would also steer by the latest measurement, which is why `command` is
//! given it.

use std::sync::Arc;

use orbit_prop::{observe, Epoch, GroundSite, Propagator};

use crate::error::SimError;
use crate::geometry::{axes_from_enu, enu_from_az_el, wrap180, MountKind};
use crate::mount::{axis_wraps, AxisCommand, MountCommand};
use crate::sensor::Measurement;
use crate::servo::Wrap;
use crate::sim::RunSetup;

pub trait Tracker {
    /// The axis angles to start the run at, `t_s = 0`.
    fn start(&mut self) -> Result<[f64; 2], SimError>;
    /// The command for the interval from `t_s` to `t_s + dt_s`. `feedback`
    /// is the latest measurement, if any.
    fn command(&mut self, t_s: f64, dt_s: f64, feedback: Option<&Measurement>) -> Result<MountCommand, SimError>;
}

/// Points at the predicted position: at the start of each interval the
/// command is the predicted axis angles, and the rate the one that reaches
/// the next prediction at the interval's end.
pub struct OpenLoopTracker {
    prop: Arc<dyn Propagator>,
    site: GroundSite,
    rise: Epoch,
    kind: MountKind,
    wrap1: Wrap,
    /// The predicted axis angles at `next_t_s`, the end of the last
    /// interval, which is normally where the next one starts.
    next: (f64, f64),
    next_t_s: f64,
}

impl OpenLoopTracker {
    pub fn new(setup: &RunSetup) -> OpenLoopTracker {
        let kind = setup.hw.mount_model.kind;
        OpenLoopTracker {
            prop: Arc::clone(&setup.prop),
            site: setup.site,
            rise: setup.rise,
            kind,
            wrap1: axis_wraps(kind)[0],
            next: (0.0, 0.0),
            next_t_s: 0.0,
        }
    }

    /// Axis angles that point at the predicted position at `t_s`.
    fn predicted_axes(&self, t_s: f64) -> Result<(f64, f64), SimError> {
        let o = observe(&self.prop.propagate(self.rise.add_seconds(t_s))?, &self.site);
        Ok(axes_from_enu(self.kind, enu_from_az_el(o.az_deg, o.el_deg), self.site.lat_deg))
    }
}

impl Tracker for OpenLoopTracker {
    fn start(&mut self) -> Result<[f64; 2], SimError> {
        self.next = self.predicted_axes(0.0)?;
        self.next_t_s = 0.0;
        Ok([self.next.0, self.next.1])
    }

    fn command(&mut self, t_s: f64, dt_s: f64, _feedback: Option<&Measurement>) -> Result<MountCommand, SimError> {
        // An interval normally starts where the last one ended. When it
        // doesn't (a real-time clock woke late), predict afresh rather than
        // command a position that is already stale.
        let now = if t_s == self.next_t_s { self.next } else { self.predicted_axes(t_s)? };
        let next_t_s = t_s + dt_s;
        let next = self.predicted_axes(next_t_s)?;
        let d1 = match self.wrap1 {
            Wrap::None => next.0 - now.0,
            Wrap::Full | Wrap::Half => wrap180(next.0 - now.0),
        };
        self.next = next;
        self.next_t_s = next_t_s;
        Ok(MountCommand {
            t_s,
            axes: [
                AxisCommand { pos_deg: now.0, rate_deg_s: d1 / dt_s },
                AxisCommand { pos_deg: now.1, rate_deg_s: (next.1 - now.1) / dt_s },
            ],
        })
    }
}
