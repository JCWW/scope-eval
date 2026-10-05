# 16. Code structure and extending the tool

**Reference page.** Where each calculation lives, how the pieces fit, and the recipes for adding presets, checks and regimes.

## Layout

```
Cargo.toml     workspace: scope-eval (this directory) and crates/orbit-prop
presets.yaml   built-in telescopes, cameras and mounts, each with its source
src/
  main.rs      command-line entry point, interactive menus, pass-prediction prompts, --demo, --help
  model/       data types: Telescope, Camera, Mount, Site, Payload, Config, Obstruction, SpotSpec,
               plus the *_dto.rs wire formats that presets.yaml is parsed into
  calculations/
    optics.rs    telescope geometry, plate scale, sampling, focus and collecting-area calculations
    camera.rs    pixel scale, timestamp accuracy and rolling-shutter calculations
    orbit.rs     circular speed, vis-viva speed and apparent orbital-rate calculations
    mount.rs     mount dynamics, payload capacity and back-focus calculations
    detection.rs target brightness, exposure, signal, noise and limiting-magnitude calculations
  constants.rs every named constant: physical constants, default assumptions, judgment thresholds
  checks.rs    the eight general checks and their PASS/WARN/FAIL judgments
  regimes.rs   orbital-regime definitions and telescope/camera/mount/system judgments
  photometry.rs  photometric inputs with defaults substituted, and which defaults were assumed
  presets.rs   loads presets.yaml
  report.rs    printing evaluations, regime summaries and details, comparison tables, formula summary
  passes_report.rs  pass table and the per-pass "Mount can follow?" judgment
  input.rs     validated terminal input helpers
crates/orbit-prop/  satellite propagation (SGP4, Keplerian + J2), observer geometry, lighting, pass finding
```

## Where each docs page lives in the code

| Page | Calculations | Judgments |
|---|---|---|
| [5](05-image-quality-checks.md), [6](06-light-field-focus-fit-checks.md) | `calculations/optics.rs`, `calculations/mount.rs` (payload) | `checks.rs` |
| [7](07-motion-and-timing.md) | `calculations/camera.rs` | `checks.rs` (`geo_motion_and_timing`, not graded) |
| [8](08-orbital-regimes.md) | `calculations/orbit.rs`, `calculations/camera.rs` | `regimes.rs` |
| [9](09-mount-dynamics.md) | `calculations/mount.rs` (`MountDynamicsCalculator`) | `regimes.rs` |
| [10](10-target-brightness-and-detection.md) | `calculations/detection.rs`, `photometry.rs` | `regimes.rs` (`system_detection`) |
| [11](11-pass-prediction.md) | `crates/orbit-prop` | `passes_report.rs` |
| [13](13-thresholds.md) | | `constants.rs` |

## Design notes

* Each `calculations/` module groups related equations in a small calculator type. For example, `OpticsCalculator` contains plate-scale and field-of-view equations, while `MountDynamicsCalculator` contains tracking and slew equations. Calculator methods are pure and covered by worked-example tests.
* The calculators contain physics and math only. `checks.rs` and `regimes.rs` apply engineering thresholds to those results and return human-readable check results ([page 4](04-how-an-evaluation-works.md#physics-versus-judgment)).
* Each general `check_*` function returns a `CheckResult` (status, detail lines, verdict) and is independent of how results are displayed.
* `evaluate_all` enforces the "first configuration is the reference" rule in one place.
* `evaluate` runs the eight checks, then calls `evaluate_regimes` in `regimes.rs`, which reuses the computed plate scale, bin, field and area. Regime results are stored on the `Evaluation`.

## Recipes

### Adding a preset

Add an entry under `telescopes`, `cameras` or `mounts` in `presets.yaml`; the comment at the top of the file lists the fields and their allowed values. Fill in the `source` field with where the numbers came from. Use `!by_diameter` or `!by_area` to match how the spec sheet quotes the obstruction, and leave anything the vendor doesn't publish as `null` rather than guessing. The file is read at run time, so no rebuild is needed; its path is fixed at build time, so the binary only works while the checkout it was built from is still in place.

### Changing a rule of thumb

Edit the relevant limit in `src/constants.rs` ([Thresholds](13-thresholds.md)). Calculator tests check the equations and worked examples; evaluation checks apply the judgment thresholds.

### Adding a check

Write a `check_*` function in `checks.rs` returning a `CheckResult`, call the appropriate calculator for its physical quantities, add it to the `checks` vector in `evaluate`, and give it the next number. The report and comparison table pick it up automatically.

### Adding or editing a regime

Add a `Regime` to `regimes()` in `src/regimes.rs`. Use `OrbitCalculator` methods (`overhead_rate_arcsec_s`, `rate_from_period_arcsec_s`, `vis_viva_km_s`) to derive rates, and set the range, prediction error, usual tracking mode and whether non-sidereal tracking is required. Every report and table picks it up automatically.

### Adding a regime check

Write a function returning a `RegimeCheck` tagged with its `Component`, and add it to the list in `evaluate_regimes`. Component and overall statuses are recomputed automatically.

## Tests

`cargo test` runs the worked examples from these docs. Each page's **Check it yourself** section names the tests that encode its examples. Together they cover plate scale, field of view, effective area and depth, by-area versus by-diameter obstruction, CFZ, ideal pixel, best bin, spot interpolation, rolling-shutter skew, the regime rates, the keyhole, timing requirements, mount acceleration and slew time, target brightness, signal-to-noise and limiting magnitude, the range checks on hand-entered values, the per-pass mount judgment and the stale-TLE warning.

It also runs the `orbit-prop` tests, which check the library against published references: Vallado's GMST, site-vector and SGP4 verification cases, and Meeus's Sun and Moon examples.

When you change a formula or a worked example, update the matching docs page and its test together, so the three ways of checking a concept (by hand, by test, in the tool) keep agreeing.
