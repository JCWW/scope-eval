# scope-sim

A time-stepped simulation of a telescope mount tracking a satellite pass. `orbit-prop` says where the satellite is; `scope-sim` adds what the hardware does about it, and measures the result as a pointing error on the sky.

It is the engine behind the [dashboard](../../dashboard/README.md), which runs it in the browser through the WebAssembly bindings in [`crates/scope-sim-wasm`](../scope-sim-wasm), and behind the `scope-sim` command-line runner.

## The parts of a run

A run is assembled from four parts, each behind a trait, so any of them can be simulated or real. Hardware in the loop means replacing some of them with drivers for real equipment; nothing else in the loop changes.

| Part | Trait | Simulated version | Real counterpart (not yet written) |
|---|---|---|---|
| Time | `Clock` (`clock.rs`) | `SimClock`: every step arrives at once, exactly when due | A real-time clock that sleeps until the step is due, optionally disciplined by GPS/PPS |
| Tracking software | `Tracker` (`tracker.rs`) | `OpenLoopTracker`: follows the prediction | The same tracker, since it is the software under test; a closed-loop tracker would also steer by the measurement it is given |
| Mount | `MountDriver` (`mount.rs`) | `SimMount`: the controller in `servo.rs` driving ideal axes | A driver for the mount's own interface (for example PlaneWave's PWI4 or ASCOM Alpaca), which takes positions and rates and reports encoders |
| Sensor | `Sensor` (`sensor.rs`) | `SyntheticSensor`: a simulated sky, with the real satellite, pointing-model error and jitter | The same synthetic sky fed a real mount's encoder angles, or a camera measuring centroids |

The mount interface sits where a commercial mount's own does. Such a mount closes its servo loop in firmware, so the controller and the axes are swapped together.

`Simulation::new` assembles the simulated set. `Simulation::from_parts(RunSetup, Parts)` takes any other: start from `Parts::simulated` and replace what you need. The tests in `sim.rs` do this with a sensor that withholds its truth, one that misses frames, and a clock that wakes late.

The `Evaluator` (`evaluator.rs`) computes the statistics and the verdict. It sees only what every setup can provide: the measurements and the mount's reported state, never the simulated truth. A step without a measurement adds to the mount statistics only, and time in the field is a fraction of the time measured.

## What one step does

Every `STEP_S` (0.02 s):

1. **Command.** The tracker reads the *predicted* direction of the satellite from the propagator and converts it to axis angles: azimuth and elevation for an alt-az mount, hour angle and declination for an equatorial one. The commanded rate is the change in that angle over the step. If a step starts later than the last one ended (a real-time clock woke late), the tracker predicts afresh rather than send a stale position.
2. **Servo.** The mount steps each axis toward the command (`servo.rs`): it feeds the commanded rate forward, adds a correction proportional to the position error (gain 4/s), caps that correction at the fastest approach from which the axis can still stop, and never exceeds that axis's rate or acceleration limit. A mount that can't keep up falls behind; near a keyhole it falls far behind.
3. **Time.** The clock waits for the step to be due. A simulated clock arrives at once.
4. **Truth.** The sensor's *real* satellite runs `lead_s` ahead of the prediction, where `lead_s = ephemeris error (km) / orbital speed`. This models along-track error in a TLE, the dominant kind.
5. **Pointing.** The telescope's real boresight is where the axes point, displaced by a **pointing-model error** drawn once per run (Gaussian, the entered RMS split equally between the two camera axes) and by **jitter** (first-order Gauss-Markov noise with a 0.5 s correlation time).
6. **Measure.** The real satellite's position in the camera's tangent plane at the boresight gives the error along x (axis 1) and y (axis 2). The target is *in the field* when both components fit inside half the sensor's width and height.

Samples are recorded every 0.5 s and passed to any `TelemetrySink` (`telemetry.rs`) added with `Simulation::add_sink`, for streaming. Summary statistics (RMS and largest error, time in field, field exits, time rate- or acceleration-limited, peak axis utilization) are accumulated every step.

A sample's `target_*` and `boresight_*` fields come from the sensor's simulated truth. When a sensor has none, the target is taken to be the prediction and the boresight to be where the axes point.

## Running from the command line

```bash
cargo run -p scope-sim -- passes crates/scope-sim/runs/iss-high-pass-l350.yaml
cargo run -p scope-sim -- run crates/scope-sim/runs/iss-high-pass-l350.yaml --format summary
cargo run -p scope-sim -- run RUN.yaml --pass highest --seed 4 --format csv --out trace.csv
```

A **run file** is YAML with a `config` (telescope, camera and mount preset names plus `mount_overrides`, exactly as the dashboard sends them), a `scenario` (site, target, search window, ephemeris error, seed) and a `pass`: an index from 0 as `passes` lists them, `highest` or `lowest`. The examples in [`runs/`](runs) cover a high ISS pass on a PlaneWave L-350, the same pass on a slow mount with different limits per axis, and a what-if orbit on an equatorial mount.

`run` writes the **trace** as JSON (default), the samples as CSV, or just the summary, and prints a one-line verdict on standard error. `--presets FILE` reads another presets file instead of the compiled-in `presets.yaml`.

## Traces and the schema version

A trace is the run's `info`, `summary` and every `samples` entry. `Trace::from_json` reads one back exactly: every telemetry type deserializes, and floats round-trip bit for bit. `info.schema_version` is `SCHEMA_VERSION`; it goes up when a field is renamed, removed or changes meaning, not when one is added, and `Trace::from_json` refuses a trace from any other version.

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

Hardware comes from scope-eval's `presets.yaml`, compiled in once as `PRESETS_YAML`, so the dashboard and the command-line tools never disagree about a spec. The crate depends on scope-eval's library for the star image, so the point spread function model also exists once. Configurations name a telescope, camera and mount preset, plus optional mount overrides.

Rate and acceleration limits are per axis. `max_rate_deg_s` and `max_accel_deg_s2` set both axes; `max_rate_deg_s_by_axis` and `max_accel_deg_s2_by_axis` (two entries, `null` for "not set") set one axis and win over the both-axes figure.

Every figure is a `Param` that records its `source`: `entered` (a preset or an override), `measured` (fitted from recordings of the real hardware, for when there are some) or `assumed`. Where neither the preset nor an override gives a figure, these are assumed and flagged:

| Figure | Assumed | Why |
|---|---|---|
| Maximum axis rate | 5 deg/s | Only when the preset has none (the iOptron HAE69C) |
| Maximum axis acceleration | 5 deg/s^2 | No preset publishes one |
| Pointing RMS | 60" | scope-eval's `DEFAULT_POINTING_RMS_ARCSEC` |
| Jitter RMS | 1" | A typical well-tuned servo |

## Limitations

* **The servo is idealized.** No backlash, no structural resonance, no wind, no encoder quantization, no command or readback latency, and the same gain on both axes. Real following error near the limits will be worse. Treat a PASS as necessary, not sufficient.
* **The pointing model is a single fixed offset.** A real model's residuals vary across the sky.
* **The ephemeris error is purely along-track and constant.** Real errors grow with the TLE's age and have smaller radial and cross-track parts.
* **The sensor's long side lies along axis 1.** Field rotation on an alt-az mount without a derotator is ignored; it doesn't change whether the target is in a centered field much, but it changes which edge it leaves by.
* **Long passes are cut at four hours.** MEO and higher passes can last most of a day.
* **Geometry is assessment grade** (about 0.01 deg), from `orbit-prop`. Its README lists the upgrade path.

## Tests

```bash
cargo test -p scope-sim
```

`tests/golden.rs` runs every file in `runs/` and compares the trace with the one recorded in `tests/golden/` (one sample in ten, numbers to a relative 1e-9). A change to anything the simulation computes fails it; after an intended change, record new traces with `UPDATE_GOLDEN=1 cargo test -p scope-sim --test golden` and review the diff.

The unit tests also check the star image against scope-eval's running example (3.03" recorded, 3.46" with the default 1" jitter). They check the geometry (round trips, orthonormal camera bases), the servo (zero lag on a constant rate, no overshoot on a step, limits never exceeded, azimuth taking the short way round) and whole runs: a perfect mount has no error, a pointing offset appears as a constant offset, a 2 km ephemeris error appears as about 2 km / range, a slow mount loses an 83 deg pass at the zenith keyhole while a fast one doesn't, and the same seed gives the same run.
