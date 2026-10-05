//! Where the target appears on the camera.
//!
//! A sensor turns the mount's state into a measurement: the target's offset
//! from the boresight in the camera frame, and whether it is in the field.
//! `SyntheticSensor` builds the measurement from a simulated sky: the real
//! satellite (the prediction plus the ephemeris error), the pointing-model
//! error and tracking jitter. Fed the encoder angles of a real mount it
//! still works, which is the cheapest hardware-in-the-loop setup. A camera
//! would measure the same offset from centroids instead.

use std::sync::Arc;

use orbit_prop::{observe, Epoch, GroundSite, Observation, Propagator};

use crate::clock::Tick;
use crate::error::SimError;
use crate::geometry::{camera_offset, enu_from_axes, enu_from_az_el, normalize, tangent_basis, MountKind, Vec3};
use crate::mount::MountState;
use crate::rng::Rng;
use crate::sim::RunSetup;

const ARCSEC_PER_RAD: f64 = 206_264.806;
/// Correlation time of the tracking jitter, seconds.
pub const JITTER_CORRELATION_S: f64 = 0.5;

/// What only a simulated sky knows: where the satellite really is and where
/// the telescope really points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Truth {
    pub target: Observation,
    /// The satellite's position, TEME, km.
    pub target_r_km: Vec3,
    /// The real boresight, pointing-model error and jitter included, as an
    /// east-north-up unit vector.
    pub boresight_enu: Vec3,
}

/// The target relative to the boresight at `t_s`, in the camera frame:
/// x along axis 1 and y along axis 2, radians.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Measurement {
    pub t_s: f64,
    pub err_rad: (f64, f64),
    pub in_fov: bool,
    /// Present when the sky is simulated; a camera cannot know it.
    pub truth: Option<Truth>,
}

pub trait Sensor {
    /// The target as seen at the end of `tick`, with the mount in `mount`.
    /// `None` when there is no measurement this tick, for instance between
    /// camera frames.
    fn measure(&mut self, tick: Tick, mount: &MountState) -> Result<Option<Measurement>, SimError>;
    /// The pointing-model error drawn for this run, camera x and y, arcsec.
    /// Only a sensor that simulates it knows it.
    fn pointing_offset_arcsec(&self) -> Option<[f64; 2]> {
        None
    }
}

/// A simulated sky: the true target, a fixed pointing-model error drawn
/// once per run and first-order Gauss-Markov jitter.
pub struct SyntheticSensor {
    prop: Arc<dyn Propagator>,
    site: GroundSite,
    rise: Epoch,
    lead_s: f64,
    kind: MountKind,
    half_fov_rad: (f64, f64),
    offset: (f64, f64),
    jitter: (f64, f64),
    jitter_sigma: f64,
    rng: Rng,
}

impl SyntheticSensor {
    /// Draws the pointing-model error and the starting jitter from the
    /// run's seed.
    pub fn new(setup: &RunSetup) -> SyntheticSensor {
        let m = &setup.hw.mount_model;
        let mut rng = Rng::new(setup.seed);
        let per_axis = |rms_arcsec: f64| rms_arcsec / ARCSEC_PER_RAD / std::f64::consts::SQRT_2;
        let pointing_sigma = per_axis(m.pointing_rms_arcsec.value);
        let jitter_sigma = per_axis(m.jitter_rms_arcsec.value);
        let offset = (pointing_sigma * rng.gaussian(), pointing_sigma * rng.gaussian());
        let jitter = (jitter_sigma * rng.gaussian(), jitter_sigma * rng.gaussian());
        SyntheticSensor {
            prop: Arc::clone(&setup.prop),
            site: setup.site,
            rise: setup.rise,
            lead_s: setup.lead_s,
            kind: m.kind,
            half_fov_rad: setup.hw.optics.half_fov_rad(),
            offset,
            jitter,
            jitter_sigma,
            rng,
        }
    }
}

impl Sensor for SyntheticSensor {
    fn measure(&mut self, tick: Tick, mount: &MountState) -> Result<Option<Measurement>, SimError> {
        if tick.dt_s > 0.0 {
            // First-order Gauss-Markov jitter with the requested RMS.
            let phi = (-tick.dt_s / JITTER_CORRELATION_S).exp();
            let kick = self.jitter_sigma * (1.0 - phi * phi).sqrt();
            self.jitter = (
                self.jitter.0 * phi + kick * self.rng.gaussian(),
                self.jitter.1 * phi + kick * self.rng.gaussian(),
            );
        }
        let lat = self.site.lat_deg;
        let state = self.prop.propagate(self.rise.add_seconds(tick.t_s).add_seconds(self.lead_s))?;
        let target = observe(&state, &self.site);
        let target_dir = enu_from_az_el(target.az_deg, target.el_deg);
        let (a1, a2) = (mount.axes[0].pos_deg, mount.axes[1].pos_deg);
        let axis_dir = enu_from_axes(self.kind, a1, a2, lat);
        let (x, y) = tangent_basis(self.kind, a1, a2, lat);
        let (ox, oy) = (self.offset.0 + self.jitter.0, self.offset.1 + self.jitter.1);
        let (tx, ty) = camera_offset(axis_dir, x, y, target_dir);
        let err = (tx - ox, ty - oy);
        let boresight = normalize([
            axis_dir[0] + ox * x[0] + oy * y[0],
            axis_dir[1] + ox * x[1] + oy * y[1],
            axis_dir[2] + ox * x[2] + oy * y[2],
        ]);
        let (half_w, half_h) = self.half_fov_rad;
        // `camera_offset` puts a target behind the boresight near +/-180
        // deg, so it can never pass this test.
        let in_fov = err.0.abs() <= half_w && err.1.abs() <= half_h;
        Ok(Some(Measurement {
            t_s: tick.t_s,
            err_rad: err,
            in_fov,
            truth: Some(Truth { target, target_r_km: state.r_km, boresight_enu: boresight }),
        }))
    }

    fn pointing_offset_arcsec(&self) -> Option<[f64; 2]> {
        Some([self.offset.0 * ARCSEC_PER_RAD, self.offset.1 * ARCSEC_PER_RAD])
    }
}
