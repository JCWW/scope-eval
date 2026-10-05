// Pure helpers for playback and charting, kept free of React so they can
// be unit-tested.

import type { Sample } from './types';

export const SPEEDS = [1, 2, 5, 10, 30, 100] as const;
export type Speed = (typeof SPEEDS)[number];

/**
 * Longest wall-clock gap one animation frame may cover, seconds. A tab
 * that was in the background for a minute resumes where it paused instead
 * of jumping ahead.
 */
export const MAX_FRAME_S = 0.25;

/** Simulated seconds to advance for a frame that took `frameS` of wall time. */
export function simSecondsForFrame(frameS: number, speed: number): number {
  if (!(frameS > 0)) return 0;
  return Math.min(frameS, MAX_FRAME_S) * speed;
}

/**
 * Thin `samples` to at most `maxPoints`, keeping from each bucket the
 * sample with the largest `key`. Peaks survive, so a brief loss of the
 * target is never thinned away.
 */
export function decimate(samples: readonly Sample[], maxPoints: number, key: (s: Sample) => number): Sample[] {
  if (samples.length <= maxPoints || maxPoints < 1) return samples.slice();
  const bucket = samples.length / maxPoints;
  const out: Sample[] = [];
  for (let b = 0; b < maxPoints; b++) {
    const from = Math.floor(b * bucket);
    const to = Math.min(samples.length, Math.floor((b + 1) * bucket));
    let best = samples[from]!;
    for (let i = from + 1; i < to; i++) {
      const s = samples[i]!;
      if (key(s) > key(best)) best = s;
    }
    out.push(best);
  }
  return out;
}

/** Samples from the last `windowS` seconds before `t`, for a trail. */
export function trail(samples: readonly Sample[], t: number, windowS: number): Sample[] {
  let i = samples.length;
  while (i > 0 && samples[i - 1]!.t_s >= t - windowS) i--;
  return samples.slice(i);
}
