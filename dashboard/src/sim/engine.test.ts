// Runs the real WebAssembly engine from Node, to check that the JSON it
// produces matches ./types.ts and that the default configurations and
// targets all resolve. Needs `npm run wasm` first.

import { readFileSync } from 'node:fs';
import { beforeAll, describe, expect, it } from 'vitest';
import { DEFAULT_CONFIGURATIONS } from '../config/configurations';
import { DEFAULT_CONDITIONS, DEFAULT_SCENARIO_SETTINGS, DEFAULT_SITE, TARGETS, scenarioFor } from '../config/scenarios';
import { Simulation, findPasses, getPresets, loadEngine, resolveConfig, runToEnd, starImage } from './engine';

const iss = scenarioFor(TARGETS[0]!, DEFAULT_SITE, DEFAULT_SCENARIO_SETTINGS);

beforeAll(async () => {
  await loadEngine(readFileSync(new URL('../wasm/scope_sim_wasm_bg.wasm', import.meta.url)));
});

describe('engine', () => {
  it('reads the presets', () => {
    const p = getPresets();
    expect(p.telescopes.length).toBeGreaterThan(0);
    expect(p.mounts.map((m) => m.mount_type)).toContain('equatorial');
  });

  it('resolves every default configuration', () => {
    for (const c of DEFAULT_CONFIGURATIONS) {
      const hw = resolveConfig(c);
      expect(hw.optics.fov_w_deg).toBeGreaterThan(0);
      expect(hw.mount_model.max_rate_deg_s.value).toBeGreaterThan(0);
    }
  });

  it('reports a bad configuration as an Error with the engine message', () => {
    expect(() => resolveConfig({ ...DEFAULT_CONFIGURATIONS[0]!, mount: 'nope' })).toThrow(/no mount preset named "nope"/);
  });

  it('finds passes for every target', () => {
    for (const t of TARGETS) {
      const list = findPasses(scenarioFor(t, DEFAULT_SITE, DEFAULT_SCENARIO_SETTINGS));
      expect(list.passes.length, t.label).toBeGreaterThan(0);
    }
  });

  it('steps a simulation and returns typed samples', () => {
    const passes = findPasses(iss).passes;
    const highest = passes.reduce((a, b) => (b.max_el_deg > a.max_el_deg ? b : a));
    const sim = new Simulation(DEFAULT_CONFIGURATIONS[0]!, iss, highest.index);
    try {
      expect(sim.info.pass.max_el_deg).toBeCloseTo(highest.max_el_deg, 6);
      expect(sim.samples()).toHaveLength(1);
      const step = sim.advance(10);
      expect(step.samples).toHaveLength(20);
      expect(step.current.t_s).toBeCloseTo(10, 9);
      expect(typeof step.current.in_fov).toBe('boolean');
      expect(step.done).toBe(false);
      expect(sim.track(5).length).toBeGreaterThan(10);
      const end = sim.advance(1e6);
      expect(end.done).toBe(true);
      expect(end.summary.complete).toBe(true);
    } finally {
      sim.free();
    }
  });

  it('shows the slow mount losing the target on the high pass', () => {
    const passes = findPasses(iss).passes;
    const highest = passes.reduce((a, b) => (b.max_el_deg > a.max_el_deg ? b : a));
    const fast = runToEnd(DEFAULT_CONFIGURATIONS[0]!, iss, highest.index);
    const slow = runToEnd(DEFAULT_CONFIGURATIONS[3]!, iss, highest.index);
    expect(slow.in_fov_fraction).toBeLessThan(fast.in_fov_fraction);
    expect(slow.rate_limited_s).toBeGreaterThan(0);
  });

  it('sizes the star with scope-eval\'s point spread function', () => {
    // DeltaRho 350 + IMX455 at 2.5": scope-eval's recorded star is 3.03"
    // (docs/17-point-spread-function.md), and the default 1" RMS jitter
    // smears it to 3.46" in a tracked exposure.
    const star = starImage(DEFAULT_CONFIGURATIONS[0]!, DEFAULT_CONDITIONS);
    expect(star.recorded_fwhm.center).toBeCloseTo(3.03, 2);
    expect(star.pixels_across).toBeCloseTo(4.05, 2);
    expect(star.tracked_fwhm.center).toBeCloseTo(3.46, 2);
    expect(star.assumed).toContain('tracking jitter');
    expect(star.sampled_fwhm_if_diameter?.center).toBeCloseTo(2.65, 2);
  });

  it('follows the seeing and the jitter override', () => {
    const c = DEFAULT_CONFIGURATIONS[0]!;
    const good = starImage(c, { ...DEFAULT_CONDITIONS, seeing_arcsec: 1.0 });
    const bad = starImage(c, { ...DEFAULT_CONDITIONS, seeing_arcsec: 4.0 });
    expect(good.recorded_fwhm.center).toBeLessThan(bad.recorded_fwhm.center);
    const still = starImage({ ...c, mount_overrides: { ...c.mount_overrides, jitter_rms_arcsec: 0 } }, DEFAULT_CONDITIONS);
    expect(still.tracked_fwhm).toEqual(still.recorded_fwhm);
    expect(() => starImage(c, { ...DEFAULT_CONDITIONS, seeing_arcsec: 0 })).toThrow(/Seeing must be a positive number/);
  });

  it('sizes a star for every default configuration', () => {
    for (const c of DEFAULT_CONFIGURATIONS) {
      const star = starImage(c, DEFAULT_CONDITIONS);
      expect(star.tracked_fwhm.center, c.name).toBeGreaterThanOrEqual(star.recorded_fwhm.center);
      expect(star.peak_pixel_fraction.center, c.name).toBeGreaterThanOrEqual(star.peak_pixel_fraction.corner);
    }
  });
});
