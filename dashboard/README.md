# Scope Sim dashboard

A React dashboard for watching the [`scope-sim`](../crates/scope-sim/README.md) simulation: a telescope mount tracking a satellite pass, and where the satellite lands on the camera sensor.

* **Start, stop and reset** the simulation, at 1x to 100x real time.
* **Watch pointing accuracy** four ways: the camera field (where the satellite is on the sensor, with a 30-second trail and the pointing-RMS circle), a sky plot of the pass, the pointing error over time against the edge of the field, and the axis rates against the mount's limit.
* **Switch configurations** at any moment. Switching keeps the playback position, so you can watch two configurations at the same instant of the same pass. Configurations can be edited, duplicated and deleted; edits are kept in the browser.
* **Compare** every configuration through the selected pass in one table.

It is built with React, TypeScript, Vite and [Material UI](https://mui.com/) (MUI X Charts for the time series).

## How it fits together

The simulation itself is Rust, compiled to WebAssembly, so the dashboard uses the same orbit propagation (`orbit-prop`) and the same hardware presets (`presets.yaml`) as the `scope-eval` command-line tool. The dashboard owns only the clock and the display:

```
presets.yaml ──┐
orbit-prop ────┼─> scope-sim (Rust) ─> scope-sim-wasm ─> src/wasm/ (generated) ─> React
               │                          JSON strings     src/sim/engine.ts
```

Each animation frame, `useSimulation` advances the engine by the frame time multiplied by the playback speed and publishes the result to React about 15 times a second.

## Prerequisites

* Node.js 22 (Vite 8 needs 20.19 or newer).
* A Rust toolchain with the WebAssembly target, and `wasm-bindgen-cli` at the same version as the `wasm-bindgen` crate:

  ```bash
  rustup target add wasm32-unknown-unknown
  cargo install wasm-bindgen-cli --version 0.2.129 --locked
  ```

## Running it

```bash
cd dashboard
npm install
npm run dev        # builds the engine, then serves http://localhost:5173
```

Other scripts:

| Script | What it does |
|---|---|
| `npm run wasm` | Rebuild the engine into `src/wasm/` (after changing any Rust code) |
| `npm run build` | Engine, typecheck and production build into `dist/` |
| `npm test` | Unit tests, plus tests that run the real engine from Node (needs `npm run wasm` first) |
| `npm run typecheck` | TypeScript only |

## Using it

1. **Pick a configuration** at the top left. The defaults pair scope-eval's presets: a DeltaRho 350 on an L-350, a RASA 11 with a small global-shutter camera, a narrow-field CDK17, the same DeltaRho on a deliberately slow mount, and on an equatorial mount.
2. **Pick a target and pass.** The ISS (a real TLE from 2008) over the default site includes an 83-degree pass, which shows the alt-az keyhole. The what-if orbits are propagated as Keplerian with J2. Paste your own TLE with a search start near its epoch.
3. **Press Start.** Change the speed at any time. Stop pauses; Reset goes back to the rise.
4. **Read the results.** The tiles summarize the run so far. "Assumed" chips and the verdict's reason say which figures were defaults rather than specs; as in scope-eval, those cap the verdict at WARN.

Things to try:

* Run the ISS's 83-degree pass with **DeltaRho 350 on a slow alt-az mount**. The azimuth axis hits its 2 deg/s limit near culmination and the satellite leaves the field for over a minute. Switch to **DeltaRho 350 on L-350** mid-pass to see the same moment with a 50 deg/s axis.
* Set the **ephemeris error** to 2 km and compare the **CDK17** (0.33 deg field) with the **RASA 11 + IMX174**: the same error is a much bigger fraction of a narrow field, and biggest overhead where the range is shortest.
* Raise **pointing RMS** until the satellite starts landing near the edge of a small sensor.

## Project layout

```
src/
  main.tsx, App.tsx      theme provider and page layout; configuration and scenario state
  theme.ts               MUI light/dark themes and the chart palette
  format.ts              angle, duration and time formatting
  config/
    configurations.ts    default configurations (preset names + mount overrides)
    scenarios.ts         target orbits and the default site
  sim/
    types.ts             TypeScript mirrors of the engine's JSON
    engine.ts            loads the WebAssembly engine; typed wrappers
    useSimulation.ts     the playback loop as a React hook
    playback.ts          speeds, frame timing, chart decimation
  components/            transport bar, stat tiles, sky plot, camera field, charts, panels, comparison
scripts/build-wasm.mjs   builds crates/scope-sim-wasm and runs wasm-bindgen
```

`src/wasm/` is generated and not committed.

## Moving it to its own repository

The dashboard depends on the repository only through `scripts/build-wasm.mjs`, which builds `crates/scope-sim-wasm`. To split it out, either keep the Rust crates as a git submodule and point the script at them, or publish the generated `src/wasm/` package and depend on it from npm.
