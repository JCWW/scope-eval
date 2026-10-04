//! A ground observing site.

use crate::error::OrbitPropError;

/// A site on the WGS-84 ellipsoid. Latitude is geodetic, positive north;
/// longitude is positive east; altitude is above the ellipsoid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroundSite {
    pub lat_deg: f64,
    pub lon_deg: f64,
    pub alt_m: f64,
}

impl GroundSite {
    /// Validates the inputs. Longitude is wrapped into `(-180, 180]`.
    pub fn new(lat_deg: f64, lon_deg: f64, alt_m: f64) -> Result<GroundSite, OrbitPropError> {
        if !lat_deg.is_finite() || !(-90.0..=90.0).contains(&lat_deg) {
            return Err(OrbitPropError::InvalidSite(format!("latitude {lat_deg} must be between -90 and 90 degrees")));
        }
        if !lon_deg.is_finite() {
            return Err(OrbitPropError::InvalidSite(format!("longitude {lon_deg} is not a number")));
        }
        if !alt_m.is_finite() || !(-500.0..=10_000.0).contains(&alt_m) {
            return Err(OrbitPropError::InvalidSite(format!("altitude {alt_m} m must be between -500 and 10000 m")));
        }
        let mut lon = lon_deg.rem_euclid(360.0);
        if lon > 180.0 {
            lon -= 360.0;
        }
        Ok(GroundSite { lat_deg, lon_deg: lon, alt_m })
    }
}

#[cfg(test)]
mod tests {
    use super::GroundSite;

    #[test]
    fn accepts_a_normal_site_and_wraps_longitude() {
        let s = GroundSite::new(39.007, 255.117, 2194.56).unwrap();
        assert!((s.lon_deg - (-104.883)).abs() < 1e-9);
    }

    #[test]
    fn rejects_bad_latitude_and_nan() {
        assert!(GroundSite::new(90.5, 0.0, 0.0).is_err());
        assert!(GroundSite::new(f64::NAN, 0.0, 0.0).is_err());
        assert!(GroundSite::new(0.0, f64::INFINITY, 0.0).is_err());
        assert!(GroundSite::new(0.0, 0.0, 50_000.0).is_err());
    }
}
