import { describe, expect, it } from 'vitest';
import { angle, clock, duration, percent, utc } from '../format';
import { MAX_FRAME_S, decimate, simSecondsForFrame, trail } from './playback';
import type { Sample } from './types';

function sample(t_s: number, err_arcsec: number): Sample {
  return { t_s, err_arcsec } as Sample;
}

describe('simSecondsForFrame', () => {
  it('scales the frame by the speed', () => {
    expect(simSecondsForFrame(0.016, 10)).toBeCloseTo(0.16, 12);
  });
  it('caps a long gap, such as a tab returning from the background', () => {
    expect(simSecondsForFrame(60, 2)).toBe(MAX_FRAME_S * 2);
  });
  it('ignores non-positive and NaN gaps', () => {
    expect(simSecondsForFrame(-1, 5)).toBe(0);
    expect(simSecondsForFrame(Number.NaN, 5)).toBe(0);
  });
});

describe('decimate', () => {
  it('returns short series unchanged', () => {
    const s = [sample(0, 1), sample(1, 2)];
    expect(decimate(s, 10, (x) => x.err_arcsec)).toEqual(s);
  });
  it('keeps the peak of each bucket', () => {
    const s = Array.from({ length: 1000 }, (_, i) => sample(i, i === 517 ? 999 : 1));
    const d = decimate(s, 50, (x) => x.err_arcsec);
    expect(d).toHaveLength(50);
    expect(d.some((x) => x.err_arcsec === 999)).toBe(true);
    expect(d.map((x) => x.t_s)).toEqual([...d.map((x) => x.t_s)].sort((a, b) => a - b));
  });
});

describe('trail', () => {
  it('returns the samples within the window', () => {
    const s = Array.from({ length: 21 }, (_, i) => sample(i * 0.5, 0));
    expect(trail(s, 10, 2).map((x) => x.t_s)).toEqual([8, 8.5, 9, 9.5, 10]);
  });
});

describe('format', () => {
  it('chooses arcseconds, arcminutes or degrees', () => {
    expect(angle(4.25)).toBe('4.3"');
    expect(angle(42)).toBe('42"');
    expect(angle(825)).toBe("13.8'");
    expect(angle(7200)).toBe('2.00 deg');
  });
  it('formats durations and clocks', () => {
    expect(duration(45)).toBe('45s');
    expect(duration(402)).toBe('6m 42s');
    expect(duration(9000)).toBe('2h 30m');
    expect(clock(65)).toBe('1:05');
    expect(clock(3725)).toBe('1:02:05');
  });
  it('formats percentages and UTC', () => {
    expect(percent(0.9876)).toBe('98.8%');
    expect(utc('2008-09-22T00:47:17.209Z')).toBe('2008-09-22 00:47:17 UTC');
  });
});
