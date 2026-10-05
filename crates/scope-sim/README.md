# scope-sim

A time-stepped simulation of a telescope mount tracking a satellite pass. `orbit-prop` says where the satellite is; `scope-sim` adds what the hardware does about it, and measures the result as a pointing error on the sky.

It is the engine behind the [dashboard](../../dashboard/README.md), which runs it in the browser through the WebAssembly bindings in [`crates/scope-sim-wasm`](../scope-sim-wasm).

## What one step does

Every `STEP_S` (0.02 s):

1. **Command.** The mount reads the *predicted* direction of the satellite from the propagator and converts it to axis angles: azimuth and elevation for an alt-az mount, hour angle and declination for an equatorial one. The commanded rate is the change in that angle over the step.
2. **Servo.** Each axis steps toward the command (`servo.rs`): it feeds the commanded rate forward, adds a correction proportional to the position error (gain 4/s), caps that correction at the fastest approach from which the axis can still stop, and never exceeds its rate or acceleration limit. A mount that can't keep up falls behind; near a keyhole it falls far behind.
3. **Truth.** The *real* satellite runs `lead_s` ahead of the prediction, where `lead_s = ephemeris error (km) / orbital speed`. This models along-track error in a TLE, the dominant kind.
4. **Pointing.** The telescope's real boresight is where the axes point, displaced by a **pointing-model error** drawn once per run (Gaussian, the entered RMS split equally between the two camera axes) and by **jitter** (first-order Gauss-Markov noise with a 0.5 s correlation time).
5. **Measure.** The real satellite's position in the camera's tangent plane at the boresight gives the error along x (axis 1) and y (axis 2). The target is *in the field* when both components fit inside half the sensor's width and height.

Samples are recorded every 0.5 s. Summary statistics (RMS and largest error, time in field, field exits, time rate- or acceleration-limited, peak axis utilization) are accumulated every step.

## Star image

`Presets::star_image` sizes a star on a configuration's sensor. It runs scope-eval's point spread function budget (`scope_eval::psf::PsfBudget`) on the full telescope and camera presets: seeing, diffraction, the optics' spot sizes, detector diffusion from a measured MTF and the pixel aperture, each a Gaussian added in quadrature ([docs/17](../../docs/17-point-spread-function.md)). It then adds one term only the simulation has: the smear from **tracking jitter** over an exposure, a Gaussian whose per-axis RMS is the jitter RMS / sqrt 2, so FWHM = 1.665 x jitter RMS. That assumes the exposure is much longer than the jitter's 0.5 s correlation time; shorter exposures smear less. Seeing and wavelength come in as `Conditions` (default 2.5" at 0.55 um), separate from the scenario, so changing them never restarts a run.

The star image does not change the verdict: whether the target stays in the field doesn't depend on its size.

## Verdict

| Time in field | Verdict |
|---|---|
| at least 99% | PASS |
| at least 90% | WARN |
| below 90% | FAIL |

As in scope-eval, a run never grades PASS on numbers nobody entered: if the axis rate, axis acceleration or pointing RMS was assumed, PASS becomes WARN and the reason names the assumption.

## Inputs and assumed values

Hardware comes from scope-eval's `presets.yaml`, so the dashboard and the command-line tool never disagree about a spec. The crate depends on scope-eval's library for the star image, so the point spread function model also exists once. Configurations name a telescope, camera and mount preset, plus optional mount overrides. Where neither the preset nor the override gives a figure, these are assumed and flagged:

| Figure | Assumed | Why |
|---|---|---|
| Maximum axis rate | 5 deg/s | Only when the preset has none (the iOptron HAE69C) |
| Maximum axis acceleration | 5 deg/s^2 | No preset publishes one |
| Pointing RMS | 60" | scope-eval's `DEFAULT_POINTING_RMS_ARCSEC` |
| Jitter RMS | 1" | A typical well-tuned servo |

## Limitations

* **The servo is idealized.** No backlash, no structural resonance, no wind, no encoder quantization, and the same gain on both axes. Real following error near the limits will be worse. Treat a PASS as necessary, not sufficient.
* **The pointing model is a single fixed offset.** A real model's residuals vary across the sky.
* **The ephemeris error is purely along-track and constant.** Real errors grow with the TLE's age and have smaller radial and cross-track parts.
* **The sensor's long side lies along axis 1.** Field rotation on an alt-az mount without a derotator is ignored; it doesn't change whether the target is in a centered field much, but it changes which edge it leaves by.
* **Long passes are cut at four hours.** MEO and higher passes can last most of a day.
* **Geometry is assessment grade** (about 0.01 deg), from `orbit-prop`. Its README lists the upgrade path.

## Tests

```bash
cargo test -p scope-sim
```

They also check the star image against scope-eval's running example (3.03" recorded, 3.46" with the default 1" jitter). They check the geometry (round trips, orthonormal camera bases), the servo (zero lag on a constant rate, no overshoot on a step, limits never exceeded, azimuth taking the short way round) and whole runs: a perfect mount has no error, a pointing offset appears as a constant offset, a 2 km ephemeris error appears as about 2 km / range, a slow mount loses an 83 deg pass at the zenith keyhole while a fast one doesn't, and the same seed gives the same run.
