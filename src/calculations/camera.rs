//! Detector pixel scale, timestamp accuracy, and rolling-shutter calculations.

use crate::constants::{MS_PER_S, S_PER_US};

/// Independent calculations for camera sampling and timing.
pub struct CameraTimingCalculator;

impl CameraTimingCalculator {
    /// Convert milliseconds to seconds.
    pub fn milliseconds_to_seconds(milliseconds: f64) -> f64 {
        milliseconds / MS_PER_S
    }

    /// Sky angle covered by a binned pixel, arcsec/px.
    pub fn binned_plate_scale_arcsec_per_px(plate_scale: f64, bin: u32) -> f64 {
        plate_scale * bin as f64
    }

    /// Timestamp-error budget for a fraction of one moving pixel, seconds.
    pub fn timing_budget_s(
        pixel_scale_arcsec: f64,
        rate_arcsec_s: f64,
        pixel_fraction: f64,
    ) -> f64 {
        pixel_fraction * pixel_scale_arcsec / rate_arcsec_s
    }

    /// Apparent position error from timestamp uncertainty, arcsec.
    pub fn position_error_arcsec(rate_arcsec_s: f64, timestamp_error_s: f64) -> f64 {
        rate_arcsec_s * timestamp_error_s
    }

    /// Top-to-bottom rolling-shutter readout time, seconds.
    pub fn rolling_readout_time_s(rows: u32, line_time_us: f64) -> f64 {
        rows as f64 * line_time_us * S_PER_US
    }

    /// Motion across the frame during rolling-shutter readout, arcsec.
    pub fn rolling_shutter_skew_arcsec(rate_arcsec_s: f64, readout_s: f64) -> f64 {
        rate_arcsec_s * readout_s
    }

    /// Pixels spanned by an angular displacement.
    pub fn pixels_for_angle(angle_arcsec: f64, pixel_scale_arcsec: f64) -> f64 {
        angle_arcsec / pixel_scale_arcsec
    }

    /// Apparent motion across detector pixels per second.
    pub fn pixels_per_second(rate_arcsec_s: f64, pixel_scale_arcsec: f64) -> f64 {
        rate_arcsec_s / pixel_scale_arcsec
    }

    /// Time for a moving target to cross an angular width, seconds.
    pub fn crossing_time_s(width_arcsec: f64, rate_arcsec_s: f64) -> f64 {
        width_arcsec / rate_arcsec_s
    }
}

#[cfg(test)]
mod tests {
    use super::CameraTimingCalculator;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn timing_budget_is_fraction_of_a_moving_pixel() {
        let scale = CameraTimingCalculator::binned_plate_scale_arcsec_per_px(0.7386, 2);
        let budget = CameraTimingCalculator::timing_budget_s(scale, 15.041, 0.25);
        assert!(close(budget * 1000.0, 24.6, 0.1));
        assert!(close(
            CameraTimingCalculator::position_error_arcsec(15.041, 0.010),
            0.15041,
            1e-12
        ));
    }

    #[test]
    fn rolling_shutter_skew_matches_sensor_spec() {
        let readout = CameraTimingCalculator::rolling_readout_time_s(6388, 39.028);
        let skew = CameraTimingCalculator::rolling_shutter_skew_arcsec(15.041, readout);
        assert!(close(readout, 0.2493, 0.0005));
        assert!(close(skew, 3.75, 0.01));
        assert!(close(
            CameraTimingCalculator::pixels_for_angle(skew, 0.7386),
            5.08,
            0.02
        ));
    }
}
