// Runs the real WebAssembly engine from Node, to check that the JSON it
// produces matches ./types.ts and that the default configurations and
// targets all resolve. Needs `npm run wasm` first.

import { readFileSync } from 'node:fs';
import { beforeAll, describe, expect, it } from 'vitest';
import { DEFAULT_CONFIGURATIONS } from '../config/configurations';
import { DEFAULT_SCENARIO_SETTINGS, DEFAULT_SITE, TARGETS, scenarioFor } from '../config/scenarios';
import { Simulation, findPasses, getPresets, loadEngine, resolveConfig, runToEnd } from './engine';

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
});
