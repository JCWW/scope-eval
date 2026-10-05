// React state for one running simulation.
//
// The engine runs in WebAssembly on the main thread. Each animation frame
// advances it by the frame time multiplied by the playback speed; React
// state is published at most ~15 times a second, which is plenty for the
// charts and keeps rendering cheap.

import { useCallback, useEffect, useRef, useState } from 'react';
import { Simulation } from './engine';
import { simSecondsForFrame, type Speed } from './playback';
import type { ConfigSpec, Sample, ScenarioSpec, SimInfo, Summary, TrackPoint } from './types';

export type Status = 'loading' | 'ready' | 'running' | 'paused' | 'done' | 'error';

export interface SimView {
  error: string | null;
  info: SimInfo | null;
  track: TrackPoint[];
  samples: Sample[];
  current: Sample | null;
  summary: Summary | null;
  done: boolean;
}

const EMPTY: SimView = { error: null, info: null, track: [], samples: [], current: null, summary: null, done: false };
const PUBLISH_INTERVAL_MS = 66;
const TRACK_POINTS = 400;

export function useSimulation(
  engineReady: boolean,
  config: ConfigSpec | null,
  scenario: ScenarioSpec | null,
  passIndex: number | null,
) {
  const simRef = useRef<Simulation | null>(null);
  const samplesRef = useRef<Sample[]>([]);
  const placeRef = useRef<{ scenarioKey: string; t: number }>({ scenarioKey: '', t: 0 });
  const speedRef = useRef<Speed>(10);
  const [view, setView] = useState<SimView>(EMPTY);
  const [running, setRunning] = useState(false);
  const [speed, setSpeedState] = useState<Speed>(10);
  const [resetCount, setResetCount] = useState(0);
  const resetSeen = useRef(0);

  const configKey = JSON.stringify(config);
  const scenarioKey = JSON.stringify([scenario, passIndex]);

  // Build a simulation whenever the inputs change. Switching configuration
  // keeps the playback position, so two configurations can be compared at
  // the same moment of the same pass; a new target, pass or a reset starts
  // from the beginning.
  useEffect(() => {
    if (!engineReady || !config || !scenario || passIndex === null) return;
    const place = placeRef.current;
    const keepPlace = place.scenarioKey === scenarioKey && resetSeen.current === resetCount;
    const resumeAt = keepPlace ? place.t : 0;
    resetSeen.current = resetCount;

    let sim: Simulation;
    try {
      sim = new Simulation(config, scenario, passIndex);
    } catch (e) {
      simRef.current?.free();
      simRef.current = null;
      setRunning(false);
      setView({ ...EMPTY, error: e instanceof Error ? e.message : String(e) });
      return;
    }
    simRef.current?.free();
    simRef.current = sim;
    placeRef.current = { scenarioKey, t: resumeAt };

    const step = resumeAt > 0 ? sim.advance(resumeAt) : null;
    samplesRef.current = sim.samples();
    const samples = samplesRef.current;
    setView({
      error: null,
      info: sim.info,
      track: sim.track(Math.max(1, sim.info.duration_s / TRACK_POINTS)),
      samples,
      current: step?.current ?? samples[samples.length - 1] ?? null,
      summary: step?.summary ?? null,
      done: step?.done ?? false,
    });
    if (step?.done) setRunning(false);
    // configKey and scenarioKey stand in for config, scenario and passIndex,
    // so a new object with the same contents doesn't rebuild the simulation.
  }, [engineReady, configKey, scenarioKey, resetCount]);

  useEffect(
    () => () => {
      simRef.current?.free();
      simRef.current = null;
    },
    [],
  );

  // The animation loop.
  useEffect(() => {
    if (!running) return;
    let frame = 0;
    let prev = performance.now();
    let lastPublish = 0;
    const tick = (now: number) => {
      const sim = simRef.current;
      if (!sim) return;
      const dt = simSecondsForFrame((now - prev) / 1000, speedRef.current);
      prev = now;
      let res;
      try {
        res = sim.advance(dt);
      } catch (e) {
        setRunning(false);
        setView((v) => ({ ...v, error: e instanceof Error ? e.message : String(e) }));
        return;
      }
      if (res.samples.length > 0) samplesRef.current = samplesRef.current.concat(res.samples);
      placeRef.current.t = res.current.t_s;
      if (now - lastPublish >= PUBLISH_INTERVAL_MS || res.done) {
        lastPublish = now;
        const samples = samplesRef.current;
        setView((v) => ({ ...v, samples, current: res.current, summary: res.summary, done: res.done }));
      }
      if (res.done) {
        setRunning(false);
        return;
      }
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [running]);

  const start = useCallback(() => {
    if (view.done) {
      placeRef.current.t = 0;
      setResetCount((n) => n + 1);
    }
    setRunning(true);
  }, [view.done]);

  const stop = useCallback(() => setRunning(false), []);

  const reset = useCallback(() => {
    setRunning(false);
    placeRef.current.t = 0;
    setResetCount((n) => n + 1);
  }, []);

  const setSpeed = useCallback((s: Speed) => {
    speedRef.current = s;
    setSpeedState(s);
  }, []);

  const status: Status = view.error
    ? 'error'
    : !view.info
      ? 'loading'
      : view.done
        ? 'done'
        : running
          ? 'running'
          : (view.current?.t_s ?? 0) > 0
            ? 'paused'
            : 'ready';

  return { view, status, speed, setSpeed, start, stop, reset };
}
