// The pixel-grid shading uses its own erf; check it against the values the
// engine's Rust implementation is tested with (calculations/psf.rs).

import { describe, expect, it } from 'vitest';
import { erf, pixelShare } from './StarImagePanel';

describe('pixel grid', () => {
  it('matches tabulated erf values', () => {
    expect(erf(0)).toBeCloseTo(0, 6);
    expect(erf(0.5)).toBeCloseTo(0.5204999, 6);
    expect(erf(-1)).toBeCloseTo(-0.8427008, 6);
  });

  it('shares light across pixels the way the engine does', () => {
    // FWHM 2 px: 0.1972 of the light in the central pixel (psf.rs peak_pixel_fraction_limits).
    const sigma = 2 / 2.35482;
    expect(pixelShare(sigma, 0) ** 2).toBeCloseTo(0.1972, 3);
    const total = Array.from({ length: 21 }, (_, k) => pixelShare(sigma, k - 10)).reduce((a, b) => a + b, 0);
    expect(total).toBeCloseTo(1, 6);
  });
});
