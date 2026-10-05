// TypeScript mirrors of the JSON that crates/scope-sim-wasm exchanges.
// Field names match the Rust structs in crates/scope-sim exactly; keep the
// two in step. src/sim/engine.test.ts runs the real engine against them.

export type MountKind = 'alt_az' | 'equatorial';

/** A figure that came from a preset or the user (`assumed: false`), or a default. */
export interface Param {
  value: number;
  assumed: boolean;
}

export interface Optics {
  aperture_mm: number;
  focal_length_mm: number;
  pixel_um: number;
  width_px: number;
  height_px: number;
  plate_scale_arcsec: number;
  fov_w_deg: number;
  fov_h_deg: number;
}

export interface MountModel {
  kind: MountKind;
  kind_assumed: boolean;
  max_rate_deg_s: Param;
  max_accel_deg_s2: Param;
  pointing_rms_arcsec: Param;
  jitter_rms_arcsec: Param;
  servo_gain_per_s: number;
}

export interface Hardware {
  name: string;
  telescope: string;
  camera: string;
  mount: string;
  optics: Optics;
  mount_model: MountModel;
}

export interface MountOverrides {
  max_rate_deg_s?: number | null;
  max_accel_deg_s2?: number | null;
  pointing_rms_arcsec?: number | null;
  jitter_rms_arcsec?: number | null;
}

export interface ConfigSpec {
  name: string;
  telescope: string;
  camera: string;
  mount: string;
  mount_overrides: MountOverrides;
}

export type TargetSpec =
  | { kind: 'tle'; text: string }
  | {
      kind: 'kepler';
      name: string;
      epoch: string;
      perigee_km: number;
      apogee_km: number;
      inclination_deg: number;
      raan_deg: number;
      arg_perigee_deg: number;
      mean_anomaly_deg: number;
    };

export interface SiteSpec {
  lat_deg: number;
  lon_deg: number;
  alt_m: number;
}

export interface ScenarioSpec {
  site: SiteSpec;
  target: TargetSpec;
  start: string;
  hours: number;
  min_el_deg: number;
  ephemeris_error_km: number;
  seed: number;
}

export interface PassSummary {
  index: number;
  rise: string;
  culmination: string;
  set: string;
  duration_s: number;
  max_el_deg: number;
  peak_az_rate_deg_s: number;
  peak_el_rate_deg_s: number;
  peak_ha_rate_deg_s: number;
  peak_dec_rate_deg_s: number;
  lighting: 'sunlit' | 'partial' | 'eclipsed';
  site_dark: 'dark' | 'twilight' | 'daylight';
  clipped_start: boolean;
  clipped_end: boolean;
}

export interface PassList {
  target: string;
  passes: PassSummary[];
  warnings: string[];
}

export interface Sample {
  t_s: number;
  utc: string;
  target_az_deg: number;
  target_el_deg: number;
  boresight_az_deg: number;
  boresight_el_deg: number;
  err_x_arcsec: number;
  err_y_arcsec: number;
  err_arcsec: number;
  in_fov: boolean;
  axis1_deg: number;
  axis2_deg: number;
  axis1_rate_deg_s: number;
  axis2_rate_deg_s: number;
  axis1_cmd_rate_deg_s: number;
  axis2_cmd_rate_deg_s: number;
  axis1_accel_deg_s2: number;
  axis2_accel_deg_s2: number;
  rate_limited: boolean;
  accel_limited: boolean;
  lighting: 'sunlit' | 'penumbra' | 'umbra';
}

export interface TrackPoint {
  t_s: number;
  az_deg: number;
  el_deg: number;
}

export interface SimInfo {
  hardware: Hardware;
  target: string;
  pass: PassSummary;
  duration_s: number;
  truncated: boolean;
  lead_s: number;
  axis_names: [string, string];
  step_s: number;
  record_interval_s: number;
  pointing_offset_arcsec: [number, number];
  seed: number;
}

export type Verdict = 'pass' | 'warn' | 'fail';

export interface Summary {
  t_s: number;
  complete: boolean;
  rms_err_arcsec: number;
  max_err_arcsec: number;
  max_err_at_s: number;
  in_fov_fraction: number;
  fov_exits: number;
  rate_limited_s: number;
  accel_limited_s: number;
  peak_rate_utilization: [number, number];
  peak_accel_utilization: [number, number];
  verdict: Verdict;
  verdict_reason: string;
}

export interface AdvanceResult {
  samples: Sample[];
  current: Sample;
  summary: Summary;
  done: boolean;
}

export interface Presets {
  telescopes: { name: string; aperture_mm: number; focal_length_mm: number; source: string }[];
  cameras: { name: string; pixel_um: number; width_px: number; height_px: number; source: string }[];
  mounts: {
    name: string;
    mount_type: 'alt_az' | 'equatorial' | 'unknown';
    max_slew_deg_s: number | null;
    max_accel_deg_s2: number | null;
    pointing_rms_arcsec: number | null;
    source: string;
  }[];
}
