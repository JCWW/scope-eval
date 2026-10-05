// Targets and the default site.
//
// A TLE is only good near its epoch, so each target carries the start of
// the pass search that goes with it.

import type { ScenarioSpec, SiteSpec, TargetSpec } from '../sim/types';

export interface TargetPreset {
  id: string;
  label: string;
  description: string;
  target: TargetSpec;
  /** ISO-8601 UTC start of the pass search. */
  start: string;
  hours: number;
}

/** The ISS TLE used by orbit-prop's README and tests. */
export const ISS_TLE = `ISS (ZARYA)
1 25544U 98067A   08264.51782528 -.00002182  00000-0 -11606-4 0  2927
2 25544  51.6416 247.4627 0006703 130.5360 325.0288 15.72125391563537`;

const WHAT_IF_EPOCH = '2026-10-05T00:00:00Z';

export const TARGETS: TargetPreset[] = [
  {
    id: 'iss',
    label: 'ISS (TLE, September 2008)',
    description: 'A real TLE propagated with SGP4. From the default site its window includes an 83 deg pass.',
    target: { kind: 'tle', text: ISS_TLE },
    start: '2008-09-20T12:00:00Z',
    hours: 72,
  },
  {
    id: 'leo-550',
    label: 'What-if: 550 km, 53 deg',
    description: 'A Starlink-like shell, propagated as Keplerian with J2 drift.',
    target: {
      kind: 'kepler',
      name: '550 km, 53 deg',
      epoch: WHAT_IF_EPOCH,
      perigee_km: 550,
      apogee_km: 550,
      inclination_deg: 53,
      raan_deg: 0,
      arg_perigee_deg: 0,
      mean_anomaly_deg: 0,
    },
    start: WHAT_IF_EPOCH,
    hours: 48,
  },
  {
    id: 'sso-800',
    label: 'What-if: 800 km sun-synchronous',
    description: 'A polar 98.6 deg orbit, typical of Earth-observation satellites.',
    target: {
      kind: 'kepler',
      name: '800 km sun-synchronous',
      epoch: WHAT_IF_EPOCH,
      perigee_km: 800,
      apogee_km: 800,
      inclination_deg: 98.6,
      raan_deg: 0,
      arg_perigee_deg: 0,
      mean_anomaly_deg: 0,
    },
    start: WHAT_IF_EPOCH,
    hours: 48,
  },
  {
    id: 'meo-gps',
    label: 'What-if: GPS-like MEO',
    description: 'A 20,200 km, 55 deg orbit. Passes last hours; up to four are simulated.',
    target: {
      kind: 'kepler',
      name: 'GPS-like 20,200 km',
      epoch: WHAT_IF_EPOCH,
      perigee_km: 20_200,
      apogee_km: 20_200,
      inclination_deg: 55,
      raan_deg: 0,
      arg_perigee_deg: 0,
      mean_anomaly_deg: 0,
    },
    start: WHAT_IF_EPOCH,
    hours: 24,
  },
];

export const CUSTOM_TLE_ID = 'custom-tle';

/** 35 N, 100 W: chosen so the ISS window has a near-zenith pass. */
export const DEFAULT_SITE: SiteSpec = { lat_deg: 35, lon_deg: -100, alt_m: 100 };

export const DEFAULT_SCENARIO_SETTINGS = {
  min_el_deg: 10,
  ephemeris_error_km: 0,
  seed: 1,
};

export function scenarioFor(
  preset: TargetPreset,
  site: SiteSpec,
  settings: { min_el_deg: number; ephemeris_error_km: number; seed: number },
): ScenarioSpec {
  return { site, target: preset.target, start: preset.start, hours: preset.hours, ...settings };
}
