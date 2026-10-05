// The configurations the dashboard starts with. Each one names presets from
// scope-eval's presets.yaml (the engine resolves the names, so specs live in
// one place) plus any mount figures the presets don't publish.
//
// Edit, add or remove entries freely. Edits made in the dashboard are kept
// in the browser's local storage; "Restore defaults" brings these back.

import type { ConfigSpec } from '../sim/types';

const DELTARHO_350 = 'PlaneWave DeltaRho 350 (14" f/3)';
const CDK17 = 'PlaneWave CDK17 (17" f/6.8)';
const RASA_11 = 'Celestron RASA 11 V2 (11" f/2.2)';
const IMX455 = 'Sony IMX455 full frame (Moravian C3-61000 PRO, QHY600 PRO)';
const IMX174 = 'Sony IMX174 global shutter (e.g. QHY174M-GPS)';
const L350 = 'PlaneWave L-350 (direct drive)';
const L500 = 'PlaneWave L-500 (direct drive)';
const HAE69 = 'iOptron HAE69C-EC (strain-wave)';

export const DEFAULT_CONFIGURATIONS: ConfigSpec[] = [
  {
    name: 'DeltaRho 350 + IMX455 on L-350',
    telescope: DELTARHO_350,
    camera: IMX455,
    mount: L350,
    // scope-eval's --demo assumes 30" pointing for this mount.
    mount_overrides: { pointing_rms_arcsec: 30 },
  },
  {
    name: 'RASA 11 + IMX174 on L-350',
    telescope: RASA_11,
    camera: IMX174,
    mount: L350,
    mount_overrides: { pointing_rms_arcsec: 30 },
  },
  {
    name: 'CDK17 + IMX455 on L-500',
    telescope: CDK17,
    camera: IMX455,
    mount: L500,
    mount_overrides: { pointing_rms_arcsec: 30 },
  },
  {
    name: 'DeltaRho 350 + IMX455 on a slow alt-az mount',
    telescope: DELTARHO_350,
    camera: IMX455,
    mount: L350,
    // The same tube on an axis limited to 2 deg/s and 1 deg/s^2, to show
    // the keyhole on a high pass.
    mount_overrides: { max_rate_deg_s: 2, max_accel_deg_s2: 1, pointing_rms_arcsec: 30 },
  },
  {
    name: 'DeltaRho 350 + IMX455 on HAE69C (equatorial)',
    telescope: DELTARHO_350,
    camera: IMX455,
    mount: HAE69,
    mount_overrides: {},
  },
];
