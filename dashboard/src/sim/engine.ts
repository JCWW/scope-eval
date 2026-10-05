// Typed access to the WebAssembly simulation engine.
//
// The engine speaks JSON strings; this module turns them into the types in
// ./types.ts and turns engine errors into ordinary Error objects carrying
// the engine's message.

import init, {
  SimHandle,
  findPasses as findPassesJson,
  presets as presetsJson,
  resolveConfig as resolveConfigJson,
  runToEnd as runToEndJson,
  starImage as starImageJson,
  type InitInput,
} from '../wasm/scope_sim_wasm.js';
import type {
  AdvanceResult,
  Conditions,
  ConfigSpec,
  Hardware,
  PassList,
  Presets,
  Sample,
  ScenarioSpec,
  SimInfo,
  StarImage,
  Summary,
  TrackPoint,
} from './types';

let ready: Promise<unknown> | null = null;

/** Load the engine once. Pass the module bytes when not in a browser (tests). */
export function loadEngine(source?: InitInput): Promise<unknown> {
  ready ??= source === undefined ? init() : init({ module_or_path: source });
  return ready;
}

function call<T>(f: () => string): T {
  try {
    return JSON.parse(f()) as T;
  } catch (e) {
    throw e instanceof Error ? e : new Error(String(e));
  }
}

export const getPresets = (): Presets => call(() => presetsJson());

export const resolveConfig = (config: ConfigSpec): Hardware => call(() => resolveConfigJson(JSON.stringify(config)));

export const starImage = (config: ConfigSpec, conditions: Conditions): StarImage =>
  call(() => starImageJson(JSON.stringify(config), JSON.stringify(conditions)));

export const findPasses = (scenario: ScenarioSpec): PassList => call(() => findPassesJson(JSON.stringify(scenario)));

export const runToEnd = (config: ConfigSpec, scenario: ScenarioSpec, passIndex: number): Summary =>
  call(() => runToEndJson(JSON.stringify(config), JSON.stringify(scenario), passIndex));

/** One running simulation. Call `free()` when done with it. */
export class Simulation {
  private handle: SimHandle;
  readonly info: SimInfo;

  constructor(config: ConfigSpec, scenario: ScenarioSpec, passIndex: number) {
    try {
      this.handle = new SimHandle(JSON.stringify(config), JSON.stringify(scenario), passIndex);
    } catch (e) {
      throw e instanceof Error ? e : new Error(String(e));
    }
    this.info = call(() => this.handle.info());
  }

  track(stepS: number): TrackPoint[] {
    return call(() => this.handle.track(stepS));
  }

  samples(): Sample[] {
    return call(() => this.handle.samples());
  }

  advance(seconds: number): AdvanceResult {
    return call(() => this.handle.advance(seconds));
  }

  free(): void {
    this.handle.free();
  }
}
