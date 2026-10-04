# Mount Acceleration and Target Brightness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give `scope-eval` a mount-acceleration model and a photometric target-brightness model, retiring three documented limitations in README.md.

**Architecture:** Two new pure-math modules, `src/photometry.rs` and `src/dynamics.rs`, as peers to the existing `src/checks.rs`. They contain only numbers-in/numbers-out functions. `src/regimes.rs` consumes them to add three new regime checks (mount acceleration, slew-and-settle, system detection) and to change one existing check (the alt-az keyhole in `mount_rate` becomes the binding constraint of the rate and acceleration limits rather than the rate alone). Every new model field is an `Option`, and the distinction between "entered" and "not entered" is what stops the detection check from ever reporting PASS on an assumed quantum efficiency.

**Tech Stack:** Rust 2021, `serde` 1 + `serde_yaml` 0.9. No new dependencies.

**Spec:** `docs/superpowers/specs/2026-10-04-acceleration-and-brightness-design.md`

## Global Constraints

- Rust edition 2021, `rust-version = "1.70"`. No language feature newer than 1.70.
- No new dependencies. Only `serde` (derive) and `serde_yaml` are available.
- Every named constant goes in `src/constants.rs`, in the section matching its kind. That file's module doc claims it holds "Every named constant in the project"; keep that true.
- Tests are in-file `#[cfg(test)] mod tests` blocks using the existing `fn close(a: f64, b: f64, tol: f64) -> bool` helper. There is no `tests/` directory and this plan does not create one.
- No invented vendor data in `presets.yaml`. Every preset carries a `source:` field; a fabricated QE or read-noise value would break the guarantee that makes the file trustworthy. New preset keys are `null`.
- Pure functions in `photometry.rs` and `dynamics.rs` take and return numbers only. No `Status`, no formatted strings. This mirrors `checks.rs`, which separates its calculations (lines 18-117) from its judgments.
- `Status` is ordered `Info < Pass < Warn < Fail` (`src/checks.rs:125`). Never cap a status with `.max(Status::Warn)` — that also promotes `Info`. Always use an explicit `== Status::Pass` test.
- `presets.yaml` must parse unchanged. Every new DTO field is `#[serde(default)]`.
- `cargo build` must introduce no new warnings. The baseline is one pre-existing `dead_code` warning on `Metrics`.

## Review Focus

These are input classes the spec does not address. `presets.yaml` is hand-edited and nothing validates it, and the interactive prompts guard only some of these. Each line names the input and the behaviour a reasonable person would expect; each has a test in the task that owns the code.

1. **Out-of-range photometric values in `presets.yaml`** (`qe: 1.5`, `qe: 0`, `throughput: -0.2`). A QE above 1 is unphysical and silently shifts every magnitude the tool reports. Expected: treated as not entered, so a default is substituted and named, and the check cannot report PASS. — Task 6.
2. **`max_accel_deg_s2: 0.0` or negative in `presets.yaml`.** The value divides into `sqrt(C/a)`, producing an infinite keyhole and an elevation of negative infinity, so a typo turns into a confident FAIL. Expected: treated as not entered, taking the `Info` branch. — Task 10.
3. **`.nan` in `presets.yaml`,** which is legal YAML. Every `f64` comparison against NaN is false, so grading falls through every branch to the final `else` and silently reports FAIL. Expected: treated as not entered. — Task 6.
4. **An exposure override long enough that the trail runs off the sensor.** The model will otherwise report a confident SNR for a target that streaked out of the field mid-exposure. Expected: the check says so and does not report PASS. — Task 13.
5. **Sky brightness entered at the wrong scale** (`2.1` for `21.0` — a plausible typo, and 2.1 mag/arcsec^2 is brighter than daylight). Sky swamps the signal and every regime FAILs with no hint that the input is at fault. Expected: implausible value treated as not entered and named. — Task 6.

Items 1, 3 and 5 are all handled by one range filter, `model::plausible`, introduced in Task 6 and reused by Task 10 and Task 12.

---

## File Structure

| File | Responsibility | Task |
|---|---|---|
| `src/model/site.rs` | +`sky_mag_arcsec2` | 1 |
| `src/model/optics.rs` | +`Telescope::throughput` | 1 |
| `src/model/camera.rs` | +`Camera::qe`; correct the `read_noise_e` doc comment | 1 |
| `src/model/mount.rs` | +`max_accel_deg_s2`, +`settle_time_s` | 1 |
| `src/model/config.rs` | +`target_mag_override`, +`exposure_override_s` | 1 |
| `src/model/optics_dto.rs`, `camera_dto.rs`, `mount_dto.rs` | matching `#[serde(default)]` fields | 1 |
| `src/constants.rs` | every new constant | 1 |
| `presets.yaml` | new keys as `null`, documented | 1 |
| `src/presets.rs` | +parse test | 1 |
| `src/model/mod.rs` | +`plausible` input filter | 6 |
| `src/photometry.rs` | **new.** Magnitudes, fluxes, trailing, SNR, limiting magnitude | 2-6 |
| `src/dynamics.rs` | **new.** Peak acceleration, keyhole, slew time | 7-8 |
| `src/regimes.rs` | `Component::System`, `Regime::usable_window_s`, three new checks, one changed check | 9-13 |
| `src/checks.rs` | `evaluate_regimes` call site | 9 |
| `src/report.rs` | `System` column, sky-brightness line, new formulas | 14 |
| `src/main.rs` | prompts, demo, help | 15 |
| `README.md` | retire three bullets, add formulas and worked examples | 16 |

---

## Task 1: Model state, constants, and wire format

Pure plumbing. No behaviour changes; the deliverable is that the project still builds, the presets still parse, and the new fields exist and default to `None`.

**Files:**
- Modify: `src/model/site.rs`, `src/model/optics.rs:79-96`, `src/model/camera.rs:11-24`, `src/model/mount.rs:22-35`, `src/model/config.rs`
- Modify: `src/model/optics_dto.rs:71-99`, `src/model/camera_dto.rs:24-50`, `src/model/mount_dto.rs:26-56`
- Modify: `src/constants.rs`, `presets.yaml`
- Modify: `src/main.rs` (struct literals only, so it compiles)
- Test: `src/presets.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `Site::sky_mag_arcsec2: Option<f64>`, `Telescope::throughput: Option<f64>`, `Camera::qe: Option<f64>`, `Mount::max_accel_deg_s2: Option<f64>`, `Mount::settle_time_s: Option<f64>`, `Config::target_mag_override: Option<f64>`, `Config::exposure_override_s: Option<f64>`, and every constant named in Step 5.

- [ ] **Step 0: Capture the demo baseline**

Several later tasks diff `--demo` output against the behaviour before this
work started. Capture it now, from a clean tree, before touching anything:

```bash
git stash list  # confirm nothing of yours is stashed
cargo run -- --demo > target/demo-baseline.txt
wc -l target/demo-baseline.txt
```

`target/` is gitignored, so this file cannot be committed by accident. Do not
regenerate it later — its value is that it predates every change in this plan.

- [ ] **Step 1: Add the `Site` field**

In `src/model/site.rs`, inside `pub struct Site`:

```rust
    /// Sky background surface brightness, V magnitudes per square arcsecond.
    /// `None` means not entered: the photometry model substitutes a default
    /// and reports that it did. Roughly 21.9 at a dark rural site, 21.0
    /// rural, 18.5 suburban. Larger numbers are darker.
    pub sky_mag_arcsec2: Option<f64>,
```

`Option<f64>` is `Copy`, so `Site: Copy` is preserved.

- [ ] **Step 2: Add the `Telescope`, `Camera`, `Mount` and `Config` fields**

In `src/model/optics.rs`, in `pub struct Telescope` after `weight_lb`:

```rust
    /// Optical throughput of the whole train (coatings, corrector, window),
    /// as a fraction from 0 to 1. `None` means not entered.
    pub throughput: Option<f64>,
```

In `src/model/camera.rs`, in `pub struct Camera` after `read_noise_e`, and correct the line above it — this field stops being illustrative:

```rust
    /// Read noise per pixel readout, electrons RMS. `None` means not entered.
    /// Used by the detection check in `regimes.rs`.
    pub read_noise_e: Option<f64>,
    /// Peak quantum efficiency, as a fraction from 0 to 1. `None` means not entered.
    pub qe: Option<f64>,
```

In `src/model/mount.rs`, in `pub struct Mount` after `max_slew_deg_s`:

```rust
    /// Maximum axis acceleration, degrees per second squared.
    pub max_accel_deg_s2: Option<f64>,
    /// Time from the end of a slew until the mount is steady enough to image, seconds.
    pub settle_time_s: Option<f64>,
```

In `src/model/config.rs`, in `pub struct Config`:

```rust
    /// Apparent magnitude of the target, if the user entered one.
    /// `None` uses the magnitude derived per regime from the reference target.
    pub target_mag_override: Option<f64>,
    /// Exposure time, if the user entered one. `None` uses the trail-limited exposure.
    pub exposure_override_s: Option<f64>,
```

- [ ] **Step 3: Add the matching DTO fields**

In `src/model/optics_dto.rs`, in `struct TelescopeDto` after `weight_lb`, and in its `From` impl:

```rust
    #[serde(default)]
    throughput: Option<f64>,
```
```rust
            throughput: dto.throughput,
```

In `src/model/camera_dto.rs`, in `struct CameraDto` after `read_noise_e`, and in its `From` impl:

```rust
    #[serde(default)]
    qe: Option<f64>,
```
```rust
            qe: dto.qe,
```

In `src/model/mount_dto.rs`, in `struct MountDto` after `max_slew_deg_s`, and in its `From` impl:

```rust
    #[serde(default)]
    max_accel_deg_s2: Option<f64>,
    #[serde(default)]
    settle_time_s: Option<f64>,
```
```rust
            max_accel_deg_s2: dto.max_accel_deg_s2,
            settle_time_s: dto.settle_time_s,
```

- [ ] **Step 4: Add the keys to `presets.yaml`**

Extend the header comment block, after the `non_sidereal_tracking` line:

```yaml
# throughput: optical throughput 0..1, or omit/null if not measured.
# qe: peak quantum efficiency 0..1, or omit/null if not entered.
# max_accel_deg_s2, settle_time_s: mount dynamics, or omit/null if not published.
```

Add `throughput: null` to each of the five telescopes, `qe: null` to each of the four cameras, and `max_accel_deg_s2: null` plus `settle_time_s: null` to each of the three mounts. Do not invent values. Extend each mount's `source:` string in the file's existing voice, for example:

```yaml
    source: "PlaneWave listing: 100 lb payload, 50 deg/s slew, alt-az (equatorial with wedge). Pointing accuracy, TLE tracking, axis acceleration and settle time not entered: confirm with the vendor."
```

- [ ] **Step 5: Add the constants**

In `src/constants.rs`, under "Physical constants":

```rust
/// Peak of the second derivative of `atan(v t / h)`, in units of `(v/h)^2`.
///
/// An overhead pass has `theta(t) = atan(v t / h)`, so with `u = v t / h`,
/// `theta'' = -2 (v/h)^2 u / (1 + u^2)^2`. That peaks at `u = 1/sqrt(3)`,
/// giving `(2/sqrt(3)) / (4/3)^2 = 3 sqrt(3) / 8`. A derived constant, not a
/// tuned threshold. See README.md.
pub const PEAK_ACCEL_COEFF: f64 = 0.649_519_052_838_329;
/// Apparent V magnitude of the Sun.
pub const SUN_APPARENT_MAG: f64 = -26.74;
/// Photons per square metre per second from a magnitude-zero source in V band.
///
/// From the V-band zero point 3.64e-23 W/m^2/Hz over a 550 nm band of width
/// 89 nm (8.82e13 Hz), giving 3.21e-9 W/m^2, divided by the 3.61e-19 J energy
/// of a 550 nm photon.
pub const PHOTONS_M2_S_MAG0: f64 = 8.9e9;
```

Under "Unit conversions":

```rust
/// Metres in one kilometre.
pub const M_PER_KM: f64 = 1000.0;
```

Under "Default assumptions, used when a spec sheet doesn't say":

```rust
/// Cross-sectional area of the representative target, m^2.
pub const REFERENCE_TARGET_CROSS_SECTION_M2: f64 = 10.0;
/// Albedo of the representative target.
pub const REFERENCE_TARGET_ALBEDO: f64 = 0.2;
/// Phase factor assumed for the representative target: full phase, phi = 0.
pub const DEFAULT_PHASE_FACTOR: f64 = 1.0;
/// Peak quantum efficiency assumed when not entered: generic back-illuminated CMOS.
pub const DEFAULT_QE: f64 = 0.80;
/// Optical throughput assumed when not entered: generic coated two-mirror train.
pub const DEFAULT_THROUGHPUT: f64 = 0.85;
/// Sky background assumed when not entered, V mag per square arcsec: rural site.
pub const DEFAULT_SKY_MAG_ARCSEC2: f64 = 21.0;
/// Read noise assumed when not entered, electrons RMS: generic CMOS.
pub const DEFAULT_READ_NOISE_E: f64 = 3.0;
/// Longest exposure the tool will derive, seconds. Caps the stationary-target case.
pub const MAX_EXPOSURE_S: f64 = 30.0;
/// Acquisition slew distance assumed by the slew-and-settle check, degrees.
pub const DEFAULT_SLEW_DISTANCE_DEG: f64 = 90.0;
/// Settle time assumed when not entered, seconds.
pub const DEFAULT_SETTLE_TIME_S: f64 = 2.0;
```

Add to the existing `pub mod regimes_limits`:

```rust
    /// System: SNR at which a target counts as detected.
    pub const DETECT_SNR_THRESHOLD: f64 = 5.0;
    /// System: SNR for comfortable detection.
    pub const SNR_PASS: f64 = 10.0;
    /// System: above this SNR, detection is simply not what limits the regime.
    pub const SNR_TRIVIAL: f64 = 100.0;
    /// Mount: required accelerations above this (deg/s^2) need a known rating to judge.
    ///
    /// Set between LEO (0.008627 deg/s^2) and MEO (1.37e-6 deg/s^2) so that LEO
    /// alone trips the Warn branch. A value of 0.01 would sit above LEO's own
    /// requirement and the branch would be unreachable.
    pub const ACCEL_MATTERS_DEG_S2: f64 = 0.005;
    /// Mount: max axis acceleration / required acceleration.
    pub const ACCEL_PASS_HEADROOM: f64 = 3.0;
    pub const ACCEL_WARN_HEADROOM: f64 = 1.0;
    /// Mount: slew + settle as a fraction of the regime's usable window.
    pub const SLEW_PASS_WINDOW_FRACTION: f64 = 0.10;
    pub const SLEW_WARN_WINDOW_FRACTION: f64 = 0.25;
```

Add a new module at the end of `src/constants.rs`:

```rust
/// Plausible ranges for hand-entered inputs.
///
/// `presets.yaml` is edited by hand and nothing else validates it. A value
/// outside these ranges is treated as not entered rather than trusted, so a
/// typo degrades the report instead of corrupting it. See `model::plausible`.
pub mod plausible_ranges {
    /// Quantum efficiency and optical throughput are fractions of 1.
    pub const QE_MIN: f64 = 0.01;
    pub const QE_MAX: f64 = 1.0;
    pub const THROUGHPUT_MIN: f64 = 0.01;
    pub const THROUGHPUT_MAX: f64 = 1.0;
    /// Sky surface brightness, V mag per square arcsec. Below 15 is daylight,
    /// above 24 is darker than any real sky.
    pub const SKY_MAG_MIN: f64 = 15.0;
    pub const SKY_MAG_MAX: f64 = 24.0;
    /// Read noise, electrons RMS.
    pub const READ_NOISE_MIN: f64 = 0.1;
    pub const READ_NOISE_MAX: f64 = 100.0;
    /// Mount axis acceleration, deg/s^2.
    pub const ACCEL_MIN_DEG_S2: f64 = 1e-4;
    pub const ACCEL_MAX_DEG_S2: f64 = 1000.0;
    /// Mount axis rate, deg/s.
    pub const SLEW_RATE_MIN_DEG_S: f64 = 1e-3;
    pub const SLEW_RATE_MAX_DEG_S: f64 = 1000.0;
    /// Settle time, seconds. Zero is allowed: it means no settle.
    pub const SETTLE_MIN_S: f64 = 0.0;
    pub const SETTLE_MAX_S: f64 = 600.0;
}
```

- [ ] **Step 6: Make `main.rs` compile again**

Adding fields breaks every struct literal. In `src/main.rs`:

- `run_interactive` (around line 65): add `sky_mag_arcsec2: None,` to the `Site` literal.
- `run_demo` (around line 305): add `sky_mag_arcsec2: None,` to the `Site` literal.
- `custom_telescope` (around line 272): add `throughput: None,` to the `Telescope` literal.
- `custom_camera` (around line 300): add `qe: None,` to the `Camera` literal.
- `custom_mount` (around line 196): add `max_accel_deg_s2: None,` and `settle_time_s: None,` to the `Mount` literal.
- `build_config` (around line 174): add `target_mag_override: None,` and `exposure_override_s: None,` to the `Config` literal.
- `run_demo`: all six `Config` literals need the same two fields. Replace the repetition with a local helper directly above `let configs = vec![`:

```rust
    let cfg = |label: &str, telescope: Telescope, camera: Camera, payload: Payload| Config {
        label: label.into(),
        telescope,
        camera,
        payload,
        timestamp_accuracy_ms: GPS_TIMESTAMP_MS,
        target_mag_override: None,
        exposure_override_s: None,
    };
```

and rewrite the six entries as, for example:

```rust
        cfg("DeltaRho 350 + IMX455", find("DeltaRho 350"), imx455.clone(), payload(mount("L-350"), 10.0)),
```

Task 15 fills these prompts in with real input. For now `None` everywhere keeps the demo's behaviour identical.

- [ ] **Step 7: Write the failing presets test**

Append to `src/presets.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_parse_with_new_fields_absent() {
        let scopes = telescopes();
        let cams = cameras();
        let ms = mounts();
        assert!(!scopes.is_empty() && !cams.is_empty() && !ms.is_empty());
        // presets.yaml carries no vendor QE, throughput or dynamics data, and
        // must not: every value in that file has a `source`.
        assert!(scopes.iter().all(|t| t.throughput.is_none()));
        assert!(cams.iter().all(|c| c.qe.is_none()));
        assert!(ms.iter().all(|m| m.max_accel_deg_s2.is_none()));
        assert!(ms.iter().all(|m| m.settle_time_s.is_none()));
    }
}
```

- [ ] **Step 8: Run the test suite**

Run: `cargo test`
Expected: PASS, 17 tests. The 16 existing tests must still pass; nothing in this task changes a calculation.

- [ ] **Step 9: Verify the demo is unchanged**

Run: `cargo run -- --demo > target/demo-task1.txt && diff target/demo-baseline.txt target/demo-task1.txt`
Expected: no differences. This task adds state, not behaviour.

- [ ] **Step 10: Commit**

```bash
git add src/model src/constants.rs src/presets.rs src/main.rs presets.yaml
git commit -m "Add model state and constants for acceleration and brightness

Every new field is Option so that 'not entered' stays distinguishable from
a value; that distinction is what stops the detection check reporting PASS
on an assumed quantum efficiency. No behaviour change yet.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 2: Derived target magnitude

**Files:**
- Create: `src/photometry.rs`
- Modify: `src/main.rs` (add `mod photometry;`)

**Interfaces:**
- Consumes: `SUN_APPARENT_MAG`, `M_PER_KM` from Task 1.
- Produces: `photometry::derived_target_mag(cross_section_m2: f64, albedo: f64, range_km: f64, phase: f64) -> f64`.

- [ ] **Step 1: Write the failing test**

Create `src/photometry.rs`:

```rust
//! Photometry: how bright a target is, and whether it can be detected.
//!
//! Every function here is pure (numbers in, number out) so it can be unit
//! tested without the interactive front end, mirroring the calculation
//! section of `checks.rs`. The judgments live in `regimes.rs`.
//!
//! Formulas and worked examples are in README.md.

use std::f64::consts::PI;

use crate::constants::{M_PER_KM, SUN_APPARENT_MAG};

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    /// The reference target: 10 m^2 at albedo 0.2, full phase.
    fn mag_at(range_km: f64) -> f64 {
        derived_target_mag(10.0, 0.2, range_km, 1.0)
    }

    #[test]
    fn derived_mag_matches_worked_examples() {
        assert!(close(mag_at(500.0), 2.25, 0.01)); // LEO
        assert!(close(mag_at(20_200.0), 10.28, 0.01)); // MEO
        assert!(close(mag_at(37_000.0), 11.59, 0.01)); // GEO
        assert!(close(mag_at(39_836.0), 11.75, 0.01)); // HEO, Molniya apogee
        assert!(close(mag_at(384_400.0), 16.67, 0.01)); // cislunar
    }

    #[test]
    fn derived_mag_lands_where_real_objects_do() {
        // The absolute scale is the thing most easily wrong by a constant
        // factor. Real GEO objects run 11-15, real cislunar 16-20.
        let geo = mag_at(37_000.0);
        let cis = mag_at(384_400.0);
        assert!((11.0..=15.0).contains(&geo), "GEO magnitude {geo} outside the observed range");
        assert!((16.0..=20.0).contains(&cis), "cislunar magnitude {cis} outside the observed range");
    }

    #[test]
    fn four_times_the_range_is_three_magnitudes_fainter() {
        // Inverse square: a factor of 4 in range is 2.5*log10(16) = 3.01 mag.
        assert!(close(mag_at(40_000.0) - mag_at(10_000.0), 3.01, 0.01));
    }
}
```

Add `mod photometry;` to the module list at the top of `src/main.rs` (after `mod model;`, keeping the list alphabetical).

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test photometry`
Expected: FAIL to compile, `cannot find function derived_target_mag in this scope`.

- [ ] **Step 3: Write the implementation**

Add to `src/photometry.rs`, above the `tests` module:

```rust
/// Apparent magnitude of a diffuse (Lambertian) target.
///
/// `m = m_sun - 2.5 log10(albedo * area * phase / (pi * d^2))`, with the area
/// in square metres and the range converted to metres. The `1/pi` is the
/// Lambertian scattering factor.
pub fn derived_target_mag(cross_section_m2: f64, albedo: f64, range_km: f64, phase: f64) -> f64 {
    let d_m = range_km * M_PER_KM;
    SUN_APPARENT_MAG - 2.5 * (albedo * cross_section_m2 * phase / (PI * d_m * d_m)).log10()
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test`
Expected: PASS, 20 tests. If `derived_mag_matches_worked_examples` fails while the others pass, suspect `SUN_APPARENT_MAG` or `M_PER_KM` from Task 1 Step 5 before touching the formula.

- [ ] **Step 5: Commit**

```bash
git add src/photometry.rs src/main.rs
git commit -m "Add derived target magnitude

m = m_sun - 2.5 log10(albedo * area * phase / (pi d^2)), the diffuse-sphere
relation. Tested against the five regime ranges, and against the magnitude
ranges real GEO and cislunar objects actually occupy.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 3: Signal and sky electron rates

**Files:**
- Modify: `src/photometry.rs`

**Interfaces:**
- Consumes: `PHOTONS_M2_S_MAG0` from Task 1.
- Produces: `photometry::signal_e_per_s(mag, eff_area_m2, qe, throughput) -> f64`, `photometry::sky_e_per_px_s(sky_mag_arcsec2, plate_scale, eff_area_m2, qe, throughput) -> f64`, `photometry::signal_coefficient(eff_area_m2, qe, throughput, exposure_s) -> f64`.

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module in `src/photometry.rs`:

```rust
    // The worked configuration from README.md: DeltaRho 350 (0.0660 m^2
    // effective area, 0.7386 "/px with 3.76 um pixels), QE 0.80,
    // throughput 0.85, sky 21.0 mag/arcsec^2.
    const AREA: f64 = 0.0660;
    const SCALE: f64 = 0.7386;
    const QE: f64 = 0.80;
    const THRU: f64 = 0.85;

    #[test]
    fn geo_signal_rate() {
        assert!(close(signal_e_per_s(11.5914, AREA, QE, THRU), 9222.4, 1.0));
    }

    #[test]
    fn sky_rate_per_pixel() {
        assert!(close(sky_e_per_px_s(21.0, SCALE, AREA, QE, THRU), 0.8674, 0.0005));
    }

    #[test]
    fn five_magnitudes_is_a_factor_of_one_hundred() {
        let bright = signal_e_per_s(10.0, AREA, QE, THRU);
        let faint = signal_e_per_s(15.0, AREA, QE, THRU);
        assert!(close(bright / faint, 100.0, 0.01));
    }

    #[test]
    fn signal_coefficient_is_the_magnitude_zero_signal() {
        // The coefficient is what a magnitude-zero target would deposit over
        // the whole exposure, so dividing the two must give 10^(-0.4 m).
        let k = signal_coefficient(AREA, QE, THRU, 30.0);
        let s = signal_e_per_s(11.5914, AREA, QE, THRU) * 30.0;
        assert!(close(s / k, 10f64.powf(-0.4 * 11.5914), 1e-12));
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test photometry`
Expected: FAIL to compile, `cannot find function signal_e_per_s in this scope`.

- [ ] **Step 3: Write the implementation**

Add to `src/photometry.rs`, above the `tests` module, and extend the `use crate::constants::{...}` line with `PHOTONS_M2_S_MAG0`:

```rust
/// Electrons per second from a point source of the given magnitude.
pub fn signal_e_per_s(mag: f64, eff_area_m2: f64, qe: f64, throughput: f64) -> f64 {
    PHOTONS_M2_S_MAG0 * 10f64.powf(-0.4 * mag) * eff_area_m2 * qe * throughput
}

/// Electrons per second per pixel from the sky background.
///
/// The sky is quoted per square arcsecond, so this is the point-source rate
/// for that surface brightness scaled by the solid angle one pixel covers.
pub fn sky_e_per_px_s(
    sky_mag_arcsec2: f64,
    plate_scale: f64,
    eff_area_m2: f64,
    qe: f64,
    throughput: f64,
) -> f64 {
    signal_e_per_s(sky_mag_arcsec2, eff_area_m2, qe, throughput) * plate_scale * plate_scale
}

/// Electrons a magnitude-zero target would deposit over the whole exposure.
/// Inverting the SNR equation for a magnitude needs this; see `limiting_mag`.
pub fn signal_coefficient(eff_area_m2: f64, qe: f64, throughput: f64, exposure_s: f64) -> f64 {
    PHOTONS_M2_S_MAG0 * eff_area_m2 * qe * throughput * exposure_s
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test photometry`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/photometry.rs
git commit -m "Add signal and sky electron rates

Sky is the point-source rate for its surface brightness scaled by the solid
angle of one pixel, so it reuses signal_e_per_s rather than repeating the
zero point.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 4: Trailing, exposure, and footprint

A moving target smears its light into a streak, which spreads the signal over more pixels and so collects more sky and read noise. This task models the streak and the exposure that keeps it to one seeing disk.

**Architecture note:** `residual_rate_arcsec_s` is *not* in this module. It switches on `TrackingMode`, which `regimes.rs` owns, and `regimes.rs` already consumes `photometry`. Rust permits module cycles within a crate, but keeping `photometry.rs` free of regime concepts is the point of the split. That function lands in Task 13.

**Files:**
- Modify: `src/photometry.rs`

**Interfaces:**
- Consumes: `MAX_EXPOSURE_S` from Task 1.
- Produces: `photometry::trail_arcsec(residual_rate_arcsec_s, exposure_s) -> f64`, `photometry::trail_limited_exposure_s(seeing_arcsec, residual_rate_arcsec_s) -> f64`, `photometry::footprint_px(seeing_arcsec, trail_arcsec, plate_scale) -> f64`.

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module in `src/photometry.rs`:

```rust
    /// The Moon's rate against the stars: 1,296,000" per sidereal month.
    const LUNAR_RATE: f64 = 1_296_000.0 / (27.321_661 * 86_400.0);

    #[test]
    fn lunar_rate_is_half_an_arcsecond_per_second() {
        assert!(close(LUNAR_RATE, 0.549_017, 0.000_01));
    }

    #[test]
    fn trail_limited_exposure_cislunar() {
        // 2.5" of seeing at 0.549"/s gives 4.554 s before the trail exceeds
        // one seeing disk.
        assert!(close(trail_limited_exposure_s(2.5, LUNAR_RATE), 4.5536, 0.001));
    }

    #[test]
    fn trail_limited_exposure_caps_a_stationary_target() {
        // A rate-tracked or stared target does not trail, so nothing bounds
        // the exposure except the cap.
        assert!(close(trail_limited_exposure_s(2.5, 0.0), 30.0, 1e-12));
    }

    #[test]
    fn trail_limited_exposure_caps_a_very_slow_target() {
        // 0.001"/s would allow a 2500 s exposure; the cap must still bind.
        assert!(close(trail_limited_exposure_s(2.5, 0.001), 30.0, 1e-12));
    }

    #[test]
    fn trail_limited_exposure_rejects_a_negative_rate() {
        // Guards against a sign slip in a caller's rate difference.
        assert!(close(trail_limited_exposure_s(2.5, -1.0), 30.0, 1e-12));
    }

    #[test]
    fn trail_equals_seeing_at_the_trail_limited_exposure() {
        let t = trail_limited_exposure_s(2.5, LUNAR_RATE);
        assert!(close(trail_arcsec(LUNAR_RATE, t), 2.5, 1e-9));
    }

    #[test]
    fn footprint_cislunar() {
        // 2.5" across, 5.0" long at 0.7386"/px: 3.385 x 6.771 = 22.92 px.
        assert!(close(footprint_px(2.5, 2.5, SCALE), 22.921, 0.005));
    }

    #[test]
    fn footprint_untrailed_is_the_seeing_disk_squared() {
        let across = 2.5 / SCALE;
        assert!(close(footprint_px(2.5, 0.0, SCALE), across * across, 1e-9));
    }

    #[test]
    fn footprint_grows_with_trail() {
        let a = footprint_px(2.5, 0.0, SCALE);
        let b = footprint_px(2.5, 5.0, SCALE);
        let c = footprint_px(2.5, 50.0, SCALE);
        assert!(a < b && b < c);
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test photometry`
Expected: FAIL to compile, `cannot find function trail_limited_exposure_s in this scope`.

- [ ] **Step 3: Write the implementation**

Add to `src/photometry.rs` and extend the `use crate::constants::{...}` line with `MAX_EXPOSURE_S`:

```rust
/// How far the target moves across the sensor during the exposure, arcsec.
pub fn trail_arcsec(residual_rate_arcsec_s: f64, exposure_s: f64) -> f64 {
    residual_rate_arcsec_s * exposure_s
}

/// Longest exposure that keeps the target's trail inside one seeing disk.
///
/// A target the mount holds still has no residual rate and so nothing to
/// trail, which would imply an unbounded exposure; that case and any
/// non-positive rate return `MAX_EXPOSURE_S`.
pub fn trail_limited_exposure_s(seeing_arcsec: f64, residual_rate_arcsec_s: f64) -> f64 {
    if residual_rate_arcsec_s <= 0.0 {
        MAX_EXPOSURE_S
    } else {
        (seeing_arcsec / residual_rate_arcsec_s).min(MAX_EXPOSURE_S)
    }
}

/// Pixels the target's light lands on: a seeing disk smeared along the trail.
///
/// Approximated as a rectangle, matching the `(pixels across)^2` footprint
/// approximation already used by check 5. The ratio between configurations is
/// what matters, not the absolute pixel count.
pub fn footprint_px(seeing_arcsec: f64, trail_arcsec: f64, plate_scale: f64) -> f64 {
    let across = seeing_arcsec / plate_scale;
    let along = (seeing_arcsec + trail_arcsec) / plate_scale;
    across * along
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test photometry`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/photometry.rs
git commit -m "Add trailing, trail-limited exposure, and footprint

Exposure derives from quantities the regimes already carry rather than
becoming a new required input: the trail-limited exposure is seeing divided
by the residual rate, capped for targets the mount holds still.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 5: SNR and limiting magnitude

**Files:**
- Modify: `src/photometry.rs`

**Interfaces:**
- Consumes: `signal_coefficient` from Task 3.
- Produces: `photometry::snr(signal_e, sky_e_total, read_noise_e, n_px) -> f64`, `photometry::limiting_mag(threshold, noise_variance_e2, signal_coefficient) -> f64`.

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module in `src/photometry.rs`:

```rust
    #[test]
    fn geo_snr_deltarho350() {
        // 30 s on a 11.59-mag target: 276,671 e- of signal against 298 e- of
        // sky over an 11.46 px footprint and a 103 e- read term.
        let signal = signal_e_per_s(11.5914, AREA, QE, THRU) * 30.0;
        let n_px = footprint_px(2.5, 0.0, SCALE);
        let sky = sky_e_per_px_s(21.0, SCALE, AREA, QE, THRU) * 30.0 * n_px;
        assert!(close(snr(signal, sky, 3.0, n_px), 525.6, 1.0));
    }

    #[test]
    fn cislunar_snr_deltarho350() {
        let t = trail_limited_exposure_s(2.5, LUNAR_RATE);
        let n_px = footprint_px(2.5, trail_arcsec(LUNAR_RATE, t), SCALE);
        let signal = signal_e_per_s(16.6739, AREA, QE, THRU) * t;
        let sky = sky_e_per_px_s(21.0, SCALE, AREA, QE, THRU) * t * n_px;
        assert!(close(snr(signal, sky, 3.0, n_px), 14.86, 0.05));
    }

    #[test]
    fn cislunar_limiting_mag() {
        let t = trail_limited_exposure_s(2.5, LUNAR_RATE);
        let n_px = footprint_px(2.5, trail_arcsec(LUNAR_RATE, t), SCALE);
        let sky = sky_e_per_px_s(21.0, SCALE, AREA, QE, THRU) * t * n_px;
        let noise = sky + 9.0 * n_px;
        let k = signal_coefficient(AREA, QE, THRU, t);
        assert!(close(limiting_mag(5.0, noise, k), 18.155, 0.01));
    }

    #[test]
    fn limiting_mag_round_trip() {
        // A target at the limiting magnitude must come back out at exactly
        // the threshold. This is the check that the quadratic inversion is
        // the true inverse of snr(), not an approximation of it.
        let t = 10.0;
        let n_px = footprint_px(2.5, 0.0, SCALE);
        let sky = sky_e_per_px_s(21.0, SCALE, AREA, QE, THRU) * t * n_px;
        let noise = sky + 9.0 * n_px;
        let k = signal_coefficient(AREA, QE, THRU, t);
        let m = limiting_mag(5.0, noise, k);
        let signal = signal_e_per_s(m, AREA, QE, THRU) * t;
        assert!(close(snr(signal, sky, 3.0, n_px), 5.0, 1e-6));
    }

    #[test]
    fn limiting_mag_round_trip_at_several_thresholds() {
        let t = 10.0;
        let n_px = footprint_px(2.5, 0.0, SCALE);
        let sky = sky_e_per_px_s(21.0, SCALE, AREA, QE, THRU) * t * n_px;
        let noise = sky + 9.0 * n_px;
        let k = signal_coefficient(AREA, QE, THRU, t);
        for threshold in [3.0, 5.0, 10.0, 50.0] {
            let m = limiting_mag(threshold, noise, k);
            let signal = signal_e_per_s(m, AREA, QE, THRU) * t;
            assert!(
                close(snr(signal, sky, 3.0, n_px), threshold, 1e-6),
                "round trip failed at threshold {threshold}"
            );
        }
    }

    #[test]
    fn limiting_mag_survives_a_noiseless_detector() {
        // Zero sky and zero read noise is the signal-limited case: the
        // inversion must not divide by zero. S = T^2 at N = 0.
        let k = signal_coefficient(AREA, QE, THRU, 10.0);
        let m = limiting_mag(5.0, 0.0, k);
        assert!(m.is_finite());
        assert!(close(10f64.powf(-0.4 * m) * k, 25.0, 1e-6));
    }

    #[test]
    fn snr_of_no_signal_is_zero() {
        assert!(close(snr(0.0, 0.0, 0.0, 0.0), 0.0, 1e-12));
    }

    #[test]
    fn a_longer_exposure_detects_a_fainter_target() {
        let faint = |t: f64| {
            let n_px = footprint_px(2.5, 0.0, SCALE);
            let sky = sky_e_per_px_s(21.0, SCALE, AREA, QE, THRU) * t * n_px;
            limiting_mag(5.0, sky + 9.0 * n_px, signal_coefficient(AREA, QE, THRU, t))
        };
        assert!(faint(60.0) > faint(10.0));
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test photometry`
Expected: FAIL to compile, `cannot find function snr in this scope`.

- [ ] **Step 3: Write the implementation**

Add to `src/photometry.rs`:

```rust
/// Signal-to-noise ratio for a target, against shot, sky and read noise.
///
/// `SNR = S / sqrt(S + B + R^2 n)`. The signal appears in the noise term
/// because photon arrival is Poisson: its own shot noise is `sqrt(S)`.
pub fn snr(signal_e: f64, sky_e_total: f64, read_noise_e: f64, n_px: f64) -> f64 {
    let variance = signal_e + sky_e_total + read_noise_e * read_noise_e * n_px;
    if variance <= 0.0 {
        0.0
    } else {
        signal_e / variance.sqrt()
    }
}

/// Faintest magnitude that reaches `threshold`.
///
/// `noise_variance_e2` is the non-signal variance `B + R^2 n`, and
/// `signal_coefficient` is what a magnitude-zero target would deposit over
/// the exposure (see [`signal_coefficient`]).
///
/// Setting `S / sqrt(S + N) = T` gives the quadratic `S^2 - T^2 S - T^2 N = 0`,
/// whose positive root is `S = (T^2 + sqrt(T^4 + 4 T^2 N)) / 2`. Exact, so no
/// iteration, and it inverts `snr` exactly rather than approximately.
pub fn limiting_mag(threshold: f64, noise_variance_e2: f64, signal_coefficient: f64) -> f64 {
    let t2 = threshold * threshold;
    let s_min = (t2 + (t2 * t2 + 4.0 * t2 * noise_variance_e2).sqrt()) / 2.0;
    -2.5 * (s_min / signal_coefficient).log10()
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test photometry`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/photometry.rs
git commit -m "Add SNR and limiting magnitude

Inverting the SNR equation is an exact quadratic rather than a search, and
the round-trip tests hold it to being the true inverse of snr().

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 6: Resolving photometric inputs, with range validation

The one place that substitutes defaults, the one place that reports what it substituted, and the one place that decides a value is not worth trusting.

**Review Focus items 1, 3 and 5 are covered here.** `presets.yaml` is hand-edited and nothing validates it. A `qe: 1.5` is unphysical, a `qe: .nan` is legal YAML that makes every subsequent comparison false, and a `sky_mag_arcsec2: 2.1` is a plausible mis-scaling of `21.0` that is brighter than daylight. None may be silently trusted. The chosen behaviour reuses the mechanism the spec already relies on: an implausible value is treated as *not entered*, so the default is substituted, the input is named in the report, and the detection check cannot grade PASS.

**Files:**
- Modify: `src/model/mod.rs`
- Modify: `src/photometry.rs`

**Interfaces:**
- Consumes: `constants::plausible_ranges::*` and the `DEFAULT_*` constants from Task 1.
- Produces: `model::plausible(value: Option<f64>, lo: f64, hi: f64) -> Option<f64>`; `photometry::Photometry { qe: f64, throughput: f64, sky_mag_arcsec2: f64, read_noise_e: f64, assumed: Vec<&'static str> }` with `Photometry::resolve(t: &Telescope, c: &Camera, site: &Site) -> Photometry` and `Photometry::any_assumed(&self) -> bool`.

- [ ] **Step 1: Write the failing tests for `plausible`**

Append to `src/model/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plausible_accepts_a_value_in_range() {
        assert_eq!(plausible(Some(0.8), 0.01, 1.0), Some(0.8));
        assert_eq!(plausible(Some(0.01), 0.01, 1.0), Some(0.01));
        assert_eq!(plausible(Some(1.0), 0.01, 1.0), Some(1.0));
    }

    #[test]
    fn plausible_rejects_out_of_range() {
        assert_eq!(plausible(Some(1.5), 0.01, 1.0), None);
        assert_eq!(plausible(Some(0.0), 0.01, 1.0), None);
        assert_eq!(plausible(Some(-0.2), 0.01, 1.0), None);
    }

    #[test]
    fn plausible_rejects_nan_and_infinity() {
        // `.nan` and `.inf` are both legal YAML scalars, and every f64
        // comparison against NaN is false, so an unguarded NaN would fall
        // through every grading branch to the final else.
        assert_eq!(plausible(Some(f64::NAN), 0.01, 1.0), None);
        assert_eq!(plausible(Some(f64::INFINITY), 0.01, 1.0), None);
        assert_eq!(plausible(Some(f64::NEG_INFINITY), 0.01, 1.0), None);
    }

    #[test]
    fn plausible_passes_none_through() {
        assert_eq!(plausible(None, 0.01, 1.0), None);
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test model::tests`
Expected: FAIL to compile, `cannot find function plausible in this scope`.

- [ ] **Step 3: Implement `plausible`**

Add to `src/model/mod.rs`, above the `tests` module:

```rust
/// Keep a hand-entered value only if it is finite and inside a plausible range.
///
/// `presets.yaml` is edited by hand and nothing else validates it, and the
/// interactive prompts guard only some fields. A value outside its range is
/// treated as *not entered*: callers then substitute a documented default and
/// report that they did, so a typo degrades the report instead of corrupting
/// it. `None`, NaN and the infinities all fail.
///
/// Ranges live in [`crate::constants::plausible_ranges`].
pub fn plausible(value: Option<f64>, lo: f64, hi: f64) -> Option<f64> {
    value.filter(|v| v.is_finite() && *v >= lo && *v <= hi)
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test model::tests`
Expected: PASS, 4 tests.

- [ ] **Step 5: Write the failing tests for `Photometry::resolve`**

Add to the `tests` module in `src/photometry.rs`:

```rust
    use crate::model::{Camera, Obstruction, Shutter, Site, Telescope};

    fn bare_telescope() -> Telescope {
        Telescope {
            name: "test".into(),
            aperture_mm: 350.0,
            focal_length_mm: 1050.0,
            obstruction: Obstruction::ByDiameter(0.56),
            image_circle_mm: 60.0,
            back_focus_mm: None,
            weight_lb: None,
            throughput: None,
            spot: None,
            source: "test".into(),
        }
    }

    fn bare_camera() -> Camera {
        Camera {
            name: "test".into(),
            pixel_um: 3.76,
            width_px: 9576,
            height_px: 6388,
            read_noise_e: None,
            qe: None,
            shutter: Shutter::Global,
            weight_lb: None,
            source: "test".into(),
        }
    }

    fn bare_site() -> Site {
        Site { seeing_arcsec: 2.5, wavelength_um: 0.55, sky_mag_arcsec2: None }
    }

    #[test]
    fn resolve_names_every_assumed_input() {
        let p = Photometry::resolve(&bare_telescope(), &bare_camera(), &bare_site());
        assert!(p.any_assumed());
        assert_eq!(p.assumed.len(), 4);
        assert!(close(p.qe, 0.80, 1e-12));
        assert!(close(p.throughput, 0.85, 1e-12));
        assert!(close(p.sky_mag_arcsec2, 21.0, 1e-12));
        assert!(close(p.read_noise_e, 3.0, 1e-12));
    }

    #[test]
    fn resolve_names_nothing_when_everything_is_entered() {
        let mut t = bare_telescope();
        let mut c = bare_camera();
        let mut s = bare_site();
        t.throughput = Some(0.9);
        c.qe = Some(0.7);
        c.read_noise_e = Some(1.5);
        s.sky_mag_arcsec2 = Some(21.9);
        let p = Photometry::resolve(&t, &c, &s);
        assert!(!p.any_assumed());
        assert!(p.assumed.is_empty());
        assert!(close(p.qe, 0.7, 1e-12));
        assert!(close(p.throughput, 0.9, 1e-12));
        assert!(close(p.sky_mag_arcsec2, 21.9, 1e-12));
        assert!(close(p.read_noise_e, 1.5, 1e-12));
    }

    #[test]
    fn resolve_rejects_a_quantum_efficiency_above_one() {
        // Review Focus 1: unphysical, and it would silently shift every
        // magnitude the tool reports.
        let mut c = bare_camera();
        c.qe = Some(1.5);
        let p = Photometry::resolve(&bare_telescope(), &c, &bare_site());
        assert!(close(p.qe, 0.80, 1e-12));
        assert!(p.assumed.contains(&"quantum efficiency"));
    }

    #[test]
    fn resolve_rejects_a_zero_or_negative_quantum_efficiency() {
        for bad in [0.0, -0.2] {
            let mut c = bare_camera();
            c.qe = Some(bad);
            let p = Photometry::resolve(&bare_telescope(), &c, &bare_site());
            assert!(close(p.qe, 0.80, 1e-12), "qe {bad} was trusted");
            assert!(p.assumed.contains(&"quantum efficiency"));
        }
    }

    #[test]
    fn resolve_rejects_nan() {
        // Review Focus 3: `.nan` is legal YAML, and NaN compares false
        // against everything, so grading would fall through to FAIL.
        let mut c = bare_camera();
        c.qe = Some(f64::NAN);
        let p = Photometry::resolve(&bare_telescope(), &c, &bare_site());
        assert!(p.qe.is_finite());
        assert!(close(p.qe, 0.80, 1e-12));
        assert!(p.assumed.contains(&"quantum efficiency"));
    }

    #[test]
    fn resolve_rejects_sky_brightness_at_the_wrong_scale() {
        // Review Focus 5: 2.1 for 21.0 is a plausible typo, and 2.1
        // mag/arcsec^2 is brighter than daylight. Trusting it would swamp
        // the signal and FAIL every regime with no hint of the real cause.
        let mut s = bare_site();
        s.sky_mag_arcsec2 = Some(2.1);
        let p = Photometry::resolve(&bare_telescope(), &bare_camera(), &s);
        assert!(close(p.sky_mag_arcsec2, 21.0, 1e-12));
        assert!(p.assumed.contains(&"sky brightness"));
    }

    #[test]
    fn resolve_rejects_an_impossibly_dark_sky() {
        let mut s = bare_site();
        s.sky_mag_arcsec2 = Some(30.0);
        let p = Photometry::resolve(&bare_telescope(), &bare_camera(), &s);
        assert!(close(p.sky_mag_arcsec2, 21.0, 1e-12));
        assert!(p.assumed.contains(&"sky brightness"));
    }

    #[test]
    fn resolve_rejects_one_bad_input_without_discarding_the_others() {
        let mut t = bare_telescope();
        let mut c = bare_camera();
        t.throughput = Some(0.9);
        c.qe = Some(99.0);
        c.read_noise_e = Some(1.5);
        let p = Photometry::resolve(&t, &c, &bare_site());
        assert!(close(p.throughput, 0.9, 1e-12));
        assert!(close(p.read_noise_e, 1.5, 1e-12));
        assert!(close(p.qe, 0.80, 1e-12));
        assert_eq!(p.assumed, vec!["quantum efficiency", "sky brightness"]);
    }
```

- [ ] **Step 6: Run the tests to verify they fail**

Run: `cargo test photometry`
Expected: FAIL to compile, `cannot find type Photometry in this scope`.

- [ ] **Step 7: Implement `Photometry`**

Add to `src/photometry.rs`. Extend the `use crate::constants::{...}` line with `DEFAULT_QE, DEFAULT_READ_NOISE_E, DEFAULT_SKY_MAG_ARCSEC2, DEFAULT_THROUGHPUT`, and add two more `use` lines:

```rust
use crate::constants::plausible_ranges as ranges;
use crate::model::{plausible, Camera, Site, Telescope};
```

```rust
/// Photometric inputs with defaults substituted, and the names of whatever
/// was assumed rather than entered.
///
/// `assumed` is what stops the detection check reporting PASS on numbers the
/// user never supplied. A value present but outside its plausible range is
/// treated as absent; see [`crate::model::plausible`].
#[derive(Debug, Clone, PartialEq)]
pub struct Photometry {
    pub qe: f64,
    pub throughput: f64,
    pub sky_mag_arcsec2: f64,
    pub read_noise_e: f64,
    /// Human-readable names of the inputs that fell back to a default.
    /// Ordered as resolved, so the report reads consistently.
    pub assumed: Vec<&'static str>,
}

impl Photometry {
    pub fn resolve(t: &Telescope, c: &Camera, site: &Site) -> Self {
        let mut assumed: Vec<&'static str> = Vec::new();
        let mut take = |value: Option<f64>, lo: f64, hi: f64, default: f64, name: &'static str| {
            match plausible(value, lo, hi) {
                Some(v) => v,
                None => {
                    assumed.push(name);
                    default
                }
            }
        };
        let qe = take(c.qe, ranges::QE_MIN, ranges::QE_MAX, DEFAULT_QE, "quantum efficiency");
        let throughput = take(
            t.throughput,
            ranges::THROUGHPUT_MIN,
            ranges::THROUGHPUT_MAX,
            DEFAULT_THROUGHPUT,
            "throughput",
        );
        let sky_mag_arcsec2 = take(
            site.sky_mag_arcsec2,
            ranges::SKY_MAG_MIN,
            ranges::SKY_MAG_MAX,
            DEFAULT_SKY_MAG_ARCSEC2,
            "sky brightness",
        );
        let read_noise_e = take(
            c.read_noise_e,
            ranges::READ_NOISE_MIN,
            ranges::READ_NOISE_MAX,
            DEFAULT_READ_NOISE_E,
            "read noise",
        );
        Photometry { qe, throughput, sky_mag_arcsec2, read_noise_e, assumed }
    }

    /// True when any input fell back to a default. The detection check must
    /// not report PASS when this holds.
    pub fn any_assumed(&self) -> bool {
        !self.assumed.is_empty()
    }
}
```

The closure takes `&mut assumed` and is called four times; the borrow ends after the last call, so moving `assumed` into the struct afterwards is accepted. If the borrow checker objects, replace the closure with four explicit `match` blocks rather than wrapping `assumed` in a cell.

- [ ] **Step 8: Run the tests to verify they pass**

Run: `cargo test`
Expected: PASS. The `resolve_rejects_one_bad_input_without_discarding_the_others` assertion on exact ordering also pins the resolution order to QE, throughput, sky, read noise.

- [ ] **Step 9: Commit**

```bash
git add src/model/mod.rs src/photometry.rs
git commit -m "Resolve photometric inputs, treating implausible values as absent

presets.yaml is hand-edited and nothing validated it. A qe of 1.5, a .nan
(legal YAML, and false against every comparison), or a sky brightness of 2.1
for 21.0 would all have been trusted silently. They now fall back to the
documented default and are named in the report, which also means the
detection check cannot grade them PASS.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 7: Peak acceleration and the keyhole

**Files:**
- Create: `src/dynamics.rs`
- Modify: `src/main.rs` (add `mod dynamics;`)

**Interfaces:**
- Consumes: `PEAK_ACCEL_COEFF` from Task 1.
- Produces: `dynamics::peak_tracking_accel_rad_s2(omega_rad_s) -> f64`, `dynamics::accel_limited_keyhole_rad(omega_rad_s, max_accel_rad_s2) -> f64`, `dynamics::keyhole_rad(omega_rad_s, max_rate_rad_s, max_accel_rad_s2: Option<f64>) -> (f64, &'static str)`.

`keyhole_rad` exists so that Task 12's compatibility guarantee can be tested on a function rather than by parsing a formatted check detail.

- [ ] **Step 1: Write the failing tests**

Create `src/dynamics.rs`:

```rust
//! Mount axis dynamics: acceleration, the alt-az keyhole, and slew timing.
//!
//! Pure functions (numbers in, number out), like the calculation section of
//! `checks.rs`. The judgments live in `regimes.rs`.
//!
//! Angles are radians and rates are per second unless a name says otherwise.
//! Formulas and worked examples are in README.md.

use crate::constants::PEAK_ACCEL_COEFF;

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    /// A 500 km overhead LEO pass: omega = v/h = 0.87234 deg/s.
    const LEO_OMEGA_DEG_S: f64 = 0.872_343;

    #[test]
    fn peak_accel_coeff_matches_its_closed_form() {
        // 3 sqrt(3) / 8, the peak of d2/dt2 atan(vt/h) in units of (v/h)^2.
        assert!(close(PEAK_ACCEL_COEFF, 3.0 * 3.0f64.sqrt() / 8.0, 1e-12));
    }

    #[test]
    fn leo_peak_tracking_accel() {
        let omega = LEO_OMEGA_DEG_S.to_radians();
        let accel_deg_s2 = peak_tracking_accel_rad_s2(omega).to_degrees();
        // 0.0086 deg/s^2: negligible for any real mount, which is why the
        // acceleration model cannot stop at this number.
        assert!(close(accel_deg_s2, 0.008_627, 1e-6));
    }

    #[test]
    fn peak_accel_scales_as_omega_squared() {
        let a = peak_tracking_accel_rad_s2(0.01);
        let b = peak_tracking_accel_rad_s2(0.02);
        assert!(close(b / a, 4.0, 1e-9));
    }

    #[test]
    fn keyhole_is_rate_limited_when_acceleration_is_unknown() {
        // The compatibility guarantee for Task 12: z = omega / v_max
        // = 0.87234 / 50 rad = 1.0 deg, so elevation 89.0 deg.
        let (z, binding) = keyhole_rad(LEO_OMEGA_DEG_S.to_radians(), 50.0f64.to_radians(), None);
        assert!(close(90.0 - z.to_degrees(), 89.0, 0.01));
        assert_eq!(binding, "rate");
    }

    #[test]
    fn acceleration_tightens_the_keyhole() {
        let omega = LEO_OMEGA_DEG_S.to_radians();
        let rate = 50.0f64.to_radians();
        for (accel_deg_s2, expected_elev) in [(10.0, 88.32), (2.0, 86.24), (0.5, 82.48)] {
            let (z, binding) = keyhole_rad(omega, rate, Some(accel_deg_s2.to_radians()));
            let elev = 90.0 - z.to_degrees();
            assert!(
                close(elev, expected_elev, 0.02),
                "at {accel_deg_s2} deg/s^2 got {elev} deg, wanted {expected_elev}"
            );
            assert_eq!(binding, "acceleration");
        }
    }

    #[test]
    fn a_very_fast_axis_leaves_the_rate_limit_binding() {
        // With enormous acceleration the rate limit is what remains.
        let omega = LEO_OMEGA_DEG_S.to_radians();
        let (z, binding) = keyhole_rad(omega, 50.0f64.to_radians(), Some(1e6f64.to_radians()));
        assert!(close(90.0 - z.to_degrees(), 89.0, 0.01));
        assert_eq!(binding, "rate");
    }

    #[test]
    fn keyhole_is_total_for_a_zero_acceleration_rating() {
        // Review Focus 2 is handled by the caller, but the pure function must
        // still be total rather than producing a NaN.
        let z = accel_limited_keyhole_rad(0.015, 0.0);
        assert!(z.is_infinite());
    }
}
```

Add `mod dynamics;` to the module list at the top of `src/main.rs`, before `mod input;`.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test dynamics`
Expected: FAIL to compile, `cannot find function peak_tracking_accel_rad_s2 in this scope`.

- [ ] **Step 3: Write the implementation**

Add to `src/dynamics.rs`, above the `tests` module:

```rust
/// Peak angular acceleration of an overhead pass, rad/s^2.
///
/// `alpha = PEAK_ACCEL_COEFF * omega^2`, where `omega = v/h` is the peak rate.
/// See the `PEAK_ACCEL_COEFF` doc comment for the derivation.
pub fn peak_tracking_accel_rad_s2(omega_rad_s: f64) -> f64 {
    PEAK_ACCEL_COEFF * omega_rad_s * omega_rad_s
}

/// Smallest zenith distance an alt-az mount can follow within its azimuth
/// acceleration limit, radians.
///
/// Near the zenith the azimuth angle sweeps through the same `atan` form as
/// the pass itself, with the minimum zenith distance `z` in place of the
/// altitude, so peak azimuth acceleration is `PEAK_ACCEL_COEFF * (omega/z)^2`.
/// Requiring that to stay within `max_accel` gives
/// `z >= omega * sqrt(PEAK_ACCEL_COEFF / max_accel)`.
///
/// A non-positive rating yields infinity: no pass is followable. Callers
/// should filter the rating with [`crate::model::plausible`] first so that a
/// typo reads as "not entered" rather than as an unfollowable mount.
pub fn accel_limited_keyhole_rad(omega_rad_s: f64, max_accel_rad_s2: f64) -> f64 {
    if max_accel_rad_s2 <= 0.0 {
        f64::INFINITY
    } else {
        omega_rad_s * (PEAK_ACCEL_COEFF / max_accel_rad_s2).sqrt()
    }
}

/// The alt-az keyhole, radians, and which limit sets it.
///
/// The rate limit gives `z >= omega / max_rate` and the acceleration limit
/// gives the value above. These are two constraints on one physical keyhole,
/// so the binding (larger) one is what the mount actually suffers.
/// `max_accel_rad_s2` of `None` means not entered, leaving the rate limit
/// alone — which is exactly the behaviour this tool had before acceleration
/// was modelled.
pub fn keyhole_rad(
    omega_rad_s: f64,
    max_rate_rad_s: f64,
    max_accel_rad_s2: Option<f64>,
) -> (f64, &'static str) {
    let z_rate = omega_rad_s / max_rate_rad_s;
    match max_accel_rad_s2 {
        Some(a) => {
            let z_accel = accel_limited_keyhole_rad(omega_rad_s, a);
            if z_accel > z_rate {
                (z_accel, "acceleration")
            } else {
                (z_rate, "rate")
            }
        }
        None => (z_rate, "rate"),
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test dynamics`
Expected: PASS, 7 tests.

- [ ] **Step 5: Commit**

```bash
git add src/dynamics.rs src/main.rs
git commit -m "Add peak tracking acceleration and the alt-az keyhole

The keyhole is one physical constraint with two limits on it, so keyhole_rad
reports the binding one. With acceleration unknown it returns the rate-only
figure, which is what the tool reported before.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 8: Slew time

**Files:**
- Modify: `src/dynamics.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces: `dynamics::slew_time_s(distance_deg, max_rate_deg_s, max_accel_deg_s2) -> f64`.

Units here are degrees, not radians, because every input is a spec-sheet figure in degrees and the output is compared against a window in seconds. The function is unit-agnostic as long as the three inputs agree; the names say degrees to stop a caller mixing them.

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module in `src/dynamics.rs`:

```rust
    #[test]
    fn slew_time_triangular_when_the_rate_limit_is_never_reached() {
        // 90 deg at 10 deg/s^2 would need 250 deg to reach 50 deg/s, so the
        // profile is accelerate-then-decelerate: t = 2 sqrt(D/a) = 6.0 s.
        assert!(close(slew_time_s(90.0, 50.0, 10.0), 6.0, 0.01));
    }

    #[test]
    fn slew_time_trapezoidal_when_the_rate_limit_is_reached() {
        // Ramp distance 50 deg < 90 deg, so it cruises: t = v/a + D/v.
        assert!(close(slew_time_s(90.0, 50.0, 50.0), 2.8, 0.01));
        assert!(close(slew_time_s(90.0, 6.0, 1.0), 21.0, 0.01));
    }

    #[test]
    fn slew_time_is_continuous_at_the_profile_boundary() {
        // At D = v^2/a the two branches must agree, or the check's output
        // would jump for a one-degree change in the assumed distance.
        let (v, a) = (50.0, 10.0);
        let boundary = v * v / a;
        let below = slew_time_s(boundary - 1e-6, v, a);
        let above = slew_time_s(boundary + 1e-6, v, a);
        assert!(close(below, above, 1e-4));
    }

    #[test]
    fn slew_time_grows_with_distance_and_shrinks_with_capability() {
        assert!(slew_time_s(180.0, 50.0, 10.0) > slew_time_s(90.0, 50.0, 10.0));
        assert!(slew_time_s(90.0, 50.0, 20.0) < slew_time_s(90.0, 50.0, 10.0));
        assert!(slew_time_s(90.0, 10.0, 10.0) > slew_time_s(90.0, 50.0, 10.0));
    }

    #[test]
    fn slew_time_is_total_for_zero_capability() {
        assert!(slew_time_s(90.0, 0.0, 10.0).is_infinite());
        assert!(slew_time_s(90.0, 50.0, 0.0).is_infinite());
        assert!(slew_time_s(90.0, -1.0, -1.0).is_infinite());
    }

    #[test]
    fn a_zero_distance_slew_takes_no_time() {
        assert!(close(slew_time_s(0.0, 50.0, 10.0), 0.0, 1e-12));
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test dynamics::tests::slew`
Expected: FAIL to compile, `cannot find function slew_time_s in this scope`.

- [ ] **Step 3: Write the implementation**

Add to `src/dynamics.rs`:

```rust
/// Time to slew a given distance under rate and acceleration limits, seconds.
///
/// A trapezoidal velocity profile: accelerate to the rate limit, cruise,
/// decelerate. If the distance is too short to reach the rate limit the
/// profile is triangular instead. The boundary is at `D = v^2 / a`, where
/// both branches agree.
///
/// ```text
/// trapezoidal (D >= v^2/a):  t = v/a + D/v
/// triangular  (D <  v^2/a):  t = 2 sqrt(D/a)
/// ```
///
/// A non-positive rate or acceleration yields infinity.
pub fn slew_time_s(distance_deg: f64, max_rate_deg_s: f64, max_accel_deg_s2: f64) -> f64 {
    if max_rate_deg_s <= 0.0 || max_accel_deg_s2 <= 0.0 {
        return f64::INFINITY;
    }
    let ramp_distance = max_rate_deg_s * max_rate_deg_s / max_accel_deg_s2;
    if distance_deg >= ramp_distance {
        max_rate_deg_s / max_accel_deg_s2 + distance_deg / max_rate_deg_s
    } else {
        2.0 * (distance_deg / max_accel_deg_s2).sqrt()
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test dynamics`
Expected: PASS, 13 tests.

- [ ] **Step 5: Commit**

```bash
git add src/dynamics.rs
git commit -m "Add trapezoidal slew timing

Falls back to a triangular profile when the distance is too short to reach
the rate limit, with a test pinning continuity at the boundary so the check's
output cannot jump for a small change in the assumed distance.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 9: `Component::System`, the usable window, and the `&Site` signature

Scaffolding for the three new checks. No new check yet; the deliverable is that `regimes.rs` can express a system-level result, every regime carries its window, and the regime layer can see the site's sky brightness.

**The signature change is load-bearing.** `evaluate_regimes` currently takes `seeing: f64` (`src/checks.rs:693`), so nothing below it can reach `site.sky_mag_arcsec2`. Task 13 needs it, so the parameter becomes `&Site`.

**Files:**
- Modify: `src/regimes.rs:40-59` (`Regime`), `:165-280` (`regimes()`), `:200-225` (`Component`), `:566-583` (`evaluate_regimes`)
- Modify: `src/checks.rs:693`

**Interfaces:**
- Consumes: nothing new.
- Produces: `Component::System`; `Regime::usable_window_s: Option<f64>`; `evaluate_regimes(cfg: &Config, ev: &Evaluation, site: &Site, reference_area: Option<f64>) -> Vec<RegimeEvaluation>`.

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module in `src/regimes.rs`:

```rust
    #[test]
    fn leo_is_the_only_window_constrained_regime() {
        // LEO passes are over in minutes; everything else is available for
        // hours, so only LEO grades the slew-and-settle check.
        for r in regimes() {
            match r.key {
                "LEO" => assert_eq!(r.usable_window_s, Some(300.0)),
                _ => assert_eq!(r.usable_window_s, None, "{} should be unconstrained", r.key),
            }
        }
    }

    #[test]
    fn system_is_a_component() {
        assert_eq!(Component::System.name(), "System");
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test regimes`
Expected: FAIL to compile, `no variant named System found for enum Component`.

- [ ] **Step 3: Add the `System` component**

In `src/regimes.rs`, extend the enum and its `name`:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Component {
    Telescope,
    Camera,
    Mount,
    /// The configuration as a whole. Detection needs collecting area, plate
    /// scale, quantum efficiency and sky background together, so it cannot be
    /// attributed to any single component.
    System,
}
```
```rust
            Component::System => "System",
```

- [ ] **Step 4: Add the usable window**

In `src/regimes.rs`, add to `pub struct Regime` after `mode`:

```rust
    /// How long the target stays usable per opportunity, seconds. `None` means
    /// effectively unlimited, so slew time does not compete with the window.
    pub usable_window_s: Option<f64>,
```

In `regimes()`, add to each of the five literals: `usable_window_s: Some(300.0),` for LEO (a 500 km pass is above useful elevation for roughly five minutes) and `usable_window_s: None,` for MEO, GEO, HEO and CIS.

- [ ] **Step 5: Change the `evaluate_regimes` signature**

In `src/regimes.rs`, replace the function with:

```rust
/// Evaluate one configuration against every regime.
pub fn evaluate_regimes(
    cfg: &Config,
    ev: &Evaluation,
    site: &Site,
    reference_area: Option<f64>,
) -> Vec<RegimeEvaluation> {
    regimes()
        .into_iter()
        .map(|r| {
            let checks = vec![
                telescope_acquisition(cfg, ev, &r),
                telescope_dwell(ev, &r),
                telescope_depth(ev, &r, reference_area),
                camera_timing(cfg, ev, &r),
                camera_shutter(cfg, ev, &r),
                camera_trailing(cfg, ev, &r, site.seeing_arcsec),
                mount_rate(cfg, &r),
                mount_non_sidereal(cfg, &r),
            ];
            RegimeEvaluation { regime: r, checks }
        })
        .collect()
}
```

Add `Site` to the `use crate::model::{...}` line at the top of `src/regimes.rs`.

In `src/checks.rs`, change the call at line 693 from:

```rust
    ev.regimes = evaluate_regimes(cfg, &ev, site.seeing_arcsec, reference.map(|r| r.effective_area_m2));
```

to:

```rust
    ev.regimes = evaluate_regimes(cfg, &ev, site, reference.map(|r| r.effective_area_m2));
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test`
Expected: PASS. `cargo build` will warn that `Component::System` is never constructed; that warning disappears in Task 13. Note it and move on.

- [ ] **Step 7: Verify the demo is unchanged**

Run: `cargo run -- --demo > target/demo-task9.txt && diff target/demo-baseline.txt target/demo-task9.txt`
Expected: no differences. This task adds capability, not output.

- [ ] **Step 8: Commit**

```bash
git add src/regimes.rs src/checks.rs
git commit -m "Add Component::System, regime usable windows, and &Site plumbing

evaluate_regimes took only the seeing figure, so nothing below it could
reach the site's sky brightness. It now takes &Site, which the detection
check needs.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 10: The mount acceleration check

**Review Focus item 2 is covered here.** A `max_accel_deg_s2: 0.0` or negative value in `presets.yaml` divides into `sqrt(C/a)` and yields an infinite keyhole, turning a typo into a confident FAIL. Filtering the rating through `model::plausible` makes it read as "not entered" and take the `Info` branch instead.

**Files:**
- Modify: `src/regimes.rs`

**Interfaces:**
- Consumes: `dynamics::peak_tracking_accel_rad_s2`, `model::plausible`, `ACCEL_MATTERS_DEG_S2`, `ACCEL_PASS_HEADROOM`, `ACCEL_WARN_HEADROOM`, `ARCSEC_PER_RADIAN`, `DEG_PER_RADIAN`.
- Produces: `fn mount_acceleration(cfg: &Config, r: &Regime) -> RegimeCheck`, wired into `evaluate_regimes`.

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module in `src/regimes.rs`. First a shared fixture, used by Tasks 10, 11, 12 and 13:

```rust
    use crate::model::{Payload, Site};

    /// DeltaRho 350 + IMX455 on an L-350, the configuration every worked
    /// example in README.md uses.
    fn fixture(mount: Option<Mount>) -> (Config, Site) {
        let telescope = crate::presets::telescopes()
            .into_iter()
            .find(|t| t.name.contains("DeltaRho 350"))
            .expect("DeltaRho 350 preset");
        let camera = crate::presets::cameras()
            .into_iter()
            .find(|c| c.name.contains("IMX455"))
            .expect("IMX455 preset");
        let cfg = Config {
            label: "fixture".into(),
            telescope,
            camera,
            payload: Payload { mount, accessories_lb: 10.0, back_focus_required_mm: None },
            timestamp_accuracy_ms: 0.1,
            target_mag_override: None,
            exposure_override_s: None,
        };
        let site = Site { seeing_arcsec: 2.5, wavelength_um: 0.55, sky_mag_arcsec2: None };
        (cfg, site)
    }

    fn l350(max_accel_deg_s2: Option<f64>) -> Mount {
        Mount {
            name: "L-350".into(),
            mount_type: MountType::AltAz,
            capacity_lb: Some(100.0),
            max_slew_deg_s: Some(50.0),
            max_accel_deg_s2,
            settle_time_s: None,
            pointing_rms_arcsec: Some(30.0),
            non_sidereal_tracking: Capability::Yes,
            source: "test".into(),
        }
    }

    /// The named check for one regime of one configuration.
    fn check_for(key: &str, title: &str, cfg: &Config, site: &Site) -> RegimeCheck {
        let ev = crate::checks::evaluate(cfg, site, None);
        ev.regimes
            .iter()
            .find(|r| r.regime.key == key)
            .unwrap_or_else(|| panic!("no regime {key}"))
            .checks
            .iter()
            .find(|c| c.title == title)
            .unwrap_or_else(|| panic!("no check {title} in regime {key}"))
            .clone()
    }
```

Then the tests:

```rust
    #[test]
    fn acceleration_passes_with_a_known_rating() {
        // LEO needs 0.0086 deg/s^2; 10 deg/s^2 is over a thousand times that.
        let (cfg, site) = fixture(Some(l350(Some(10.0))));
        let c = check_for("LEO", "Acceleration", &cfg, &site);
        assert_eq!(c.status, Status::Pass);
    }

    #[test]
    fn acceleration_fails_a_mount_that_cannot_keep_up() {
        let (cfg, site) = fixture(Some(l350(Some(0.004))));
        let c = check_for("LEO", "Acceleration", &cfg, &site);
        assert_eq!(c.status, Status::Fail);
    }

    #[test]
    fn acceleration_warns_when_the_rating_is_unknown_and_leo_needs_it() {
        // 0.008627 deg/s^2 is above ACCEL_MATTERS_DEG_S2, so an unknown
        // rating is a question for the vendor rather than a non-issue.
        let (cfg, site) = fixture(Some(l350(None)));
        let c = check_for("LEO", "Acceleration", &cfg, &site);
        assert_eq!(c.status, Status::Warn);
        assert!(c.verdict.contains("unknown"));
    }

    #[test]
    fn acceleration_is_info_when_the_rating_is_unknown_and_irrelevant() {
        // MEO needs 1.4e-6 deg/s^2. Nobody needs to ask the vendor about that.
        let (cfg, site) = fixture(Some(l350(None)));
        let c = check_for("MEO", "Acceleration", &cfg, &site);
        assert_eq!(c.status, Status::Info);
    }

    #[test]
    fn acceleration_passes_in_stare_mode() {
        let (cfg, site) = fixture(Some(l350(None)));
        let c = check_for("GEO", "Acceleration", &cfg, &site);
        assert_eq!(c.status, Status::Pass);
    }

    #[test]
    fn acceleration_treats_a_nonsense_rating_as_unknown() {
        // Review Focus 2: a zero, negative or NaN rating in presets.yaml must
        // not become an unfollowable mount.
        for bad in [0.0, -5.0, f64::NAN, f64::INFINITY] {
            let (cfg, site) = fixture(Some(l350(Some(bad))));
            let c = check_for("LEO", "Acceleration", &cfg, &site);
            assert_eq!(c.status, Status::Warn, "rating {bad} was trusted");
            assert!(c.verdict.contains("unknown"), "rating {bad} was trusted");
        }
    }

    #[test]
    fn acceleration_is_info_with_no_mount() {
        let (cfg, site) = fixture(None);
        let c = check_for("LEO", "Acceleration", &cfg, &site);
        assert_eq!(c.status, Status::Info);
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test regimes::tests::acceleration`
Expected: FAIL, panicking with `no check Acceleration in regime LEO`.

- [ ] **Step 3: Write the implementation**

Add to `src/regimes.rs` in the "Mount checks" section. Extend the imports: add `ACCEL_MATTERS_DEG_S2` etc. come from `limits`, so only add `use crate::dynamics;` and `plausible` plus `plausible_ranges as ranges` to the existing `use` lines.

```rust
/// Can the mount accelerate fast enough to follow the pass?
fn mount_acceleration(cfg: &Config, r: &Regime) -> RegimeCheck {
    let title = "Acceleration";
    let omega_rad_s = r.rate_vs_ground / ARCSEC_PER_RADIAN;
    let required = dynamics::peak_tracking_accel_rad_s2(omega_rad_s) * DEG_PER_RADIAN;
    let mut details = vec![kv("Required peak acceleration", format!("{required:.5} deg/s^2"))];

    if r.mode == TrackingMode::Stare {
        details.push(kv("Mode", "stare: tracking off".to_string()));
        return RegimeCheck {
            component: Component::Mount,
            title,
            status: Status::Pass,
            details,
            verdict: "The target is Earth-fixed. The mount never has to accelerate to follow it."
                .to_string(),
        };
    }

    let Some(m) = cfg.payload.mount.as_ref() else {
        return RegimeCheck {
            component: Component::Mount,
            title,
            status: Status::Info,
            details,
            verdict: "No mount selected.".to_string(),
        };
    };

    let Some(max) = plausible(m.max_accel_deg_s2, ranges::ACCEL_MIN_DEG_S2, ranges::ACCEL_MAX_DEG_S2)
    else {
        let status = if required > limits::ACCEL_MATTERS_DEG_S2 { Status::Warn } else { Status::Info };
        return RegimeCheck {
            component: Component::Mount,
            title,
            status,
            details,
            verdict: format!("Maximum axis acceleration for {} is unknown. Ask the vendor.", m.name),
        };
    };

    let headroom = max / required;
    details.push(kv("Mount maximum acceleration", format!("{max:.3} deg/s^2  (headroom {headroom:.0}x)")));
    let status = if headroom >= limits::ACCEL_PASS_HEADROOM {
        Status::Pass
    } else if headroom >= limits::ACCEL_WARN_HEADROOM {
        Status::Warn
    } else {
        Status::Fail
    };
    let verdict = match status {
        Status::Pass => "Ample acceleration headroom.".to_string(),
        Status::Warn => "The mount can just accelerate fast enough, with little margin for corrections.".to_string(),
        _ => "The mount cannot accelerate fast enough to follow the pass.".to_string(),
    };
    RegimeCheck { component: Component::Mount, title, status, details, verdict }
}
```

Add `mount_acceleration(cfg, &r),` to the `checks` vector in `evaluate_regimes`, after `mount_rate(cfg, &r),`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/regimes.rs
git commit -m "Add the mount acceleration check

An unknown rating is Warn only where the requirement is large enough to
matter, which in practice means LEO alone. A zero, negative or NaN rating in
presets.yaml reads as unknown rather than as an unfollowable mount.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 11: The slew-and-settle check

**Files:**
- Modify: `src/regimes.rs`

**Interfaces:**
- Consumes: `dynamics::slew_time_s`, `model::plausible`, `DEFAULT_SLEW_DISTANCE_DEG`, `DEFAULT_SETTLE_TIME_S`, `SLEW_PASS_WINDOW_FRACTION`, `SLEW_WARN_WINDOW_FRACTION`.
- Produces: `fn mount_slew_settle(cfg: &Config, r: &Regime) -> RegimeCheck`, wired into `evaluate_regimes`.

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module in `src/regimes.rs`:

```rust
    #[test]
    fn slew_and_settle_passes_a_direct_drive_mount_on_leo() {
        // 90 deg at 50 deg/s and 10 deg/s^2 is 6.0 s, plus 2 s of assumed
        // settle: 8 s of a 300 s window, under 3%.
        let (cfg, site) = fixture(Some(l350(Some(10.0))));
        let c = check_for("LEO", "Slew and settle", &cfg, &site);
        assert_eq!(c.status, Status::Pass);
    }

    #[test]
    fn slew_and_settle_fails_a_mount_that_cannot_get_there_in_time() {
        let mut m = l350(Some(0.05));
        m.max_slew_deg_s = Some(1.0);
        let (cfg, site) = fixture(Some(m));
        let c = check_for("LEO", "Slew and settle", &cfg, &site);
        assert_eq!(c.status, Status::Fail);
    }

    #[test]
    fn slew_and_settle_is_info_where_the_window_is_unlimited() {
        for key in ["MEO", "GEO", "HEO", "CIS"] {
            let (cfg, site) = fixture(Some(l350(Some(10.0))));
            let c = check_for(key, "Slew and settle", &cfg, &site);
            assert_eq!(c.status, Status::Info, "{key} should not be window-constrained");
        }
    }

    #[test]
    fn slew_and_settle_reports_a_lower_bound_when_acceleration_is_unknown() {
        let (cfg, site) = fixture(Some(l350(None)));
        let c = check_for("LEO", "Slew and settle", &cfg, &site);
        assert_eq!(c.status, Status::Info);
        assert!(c.verdict.contains("lower bound"));
    }

    #[test]
    fn slew_and_settle_names_the_settle_time_as_assumed() {
        let (cfg, site) = fixture(Some(l350(Some(10.0))));
        let c = check_for("LEO", "Slew and settle", &cfg, &site);
        assert!(c.details.iter().any(|d| d.contains("assumed")));
    }

    #[test]
    fn slew_and_settle_uses_an_entered_settle_time() {
        let mut m = l350(Some(10.0));
        m.settle_time_s = Some(45.0);
        let (cfg, site) = fixture(Some(m));
        let c = check_for("LEO", "Slew and settle", &cfg, &site);
        // 6 s of slew plus 45 s of settle is 17% of a 300 s window: a WARN.
        assert_eq!(c.status, Status::Warn);
    }

    #[test]
    fn slew_and_settle_is_info_with_no_mount() {
        let (cfg, site) = fixture(None);
        let c = check_for("LEO", "Slew and settle", &cfg, &site);
        assert_eq!(c.status, Status::Info);
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test regimes::tests::slew_and_settle`
Expected: FAIL, panicking with `no check Slew and settle in regime LEO`.

- [ ] **Step 3: Write the implementation**

Add to `src/regimes.rs` after `mount_acceleration`:

```rust
/// Can the mount get on target in time to use the pass?
fn mount_slew_settle(cfg: &Config, r: &Regime) -> RegimeCheck {
    let title = "Slew and settle";
    let Some(window) = r.usable_window_s else {
        return RegimeCheck {
            component: Component::Mount,
            title,
            status: Status::Info,
            details: vec![kv("Usable window", "effectively unlimited".to_string())],
            verdict: "The target stays available long enough that slew time does not compete with it."
                .to_string(),
        };
    };
    let mut details = vec![
        kv("Usable window", format!("{window:.0} s")),
        kv("Assumed slew distance", format!("{:.0} deg", DEFAULT_SLEW_DISTANCE_DEG)),
    ];
    let Some(m) = cfg.payload.mount.as_ref() else {
        return RegimeCheck {
            component: Component::Mount,
            title,
            status: Status::Info,
            details,
            verdict: "No mount selected.".to_string(),
        };
    };
    let rate = plausible(m.max_slew_deg_s, ranges::SLEW_RATE_MIN_DEG_S, ranges::SLEW_RATE_MAX_DEG_S);
    let accel = plausible(m.max_accel_deg_s2, ranges::ACCEL_MIN_DEG_S2, ranges::ACCEL_MAX_DEG_S2);

    match (rate, accel) {
        (Some(v), Some(a)) => {
            let entered = plausible(m.settle_time_s, ranges::SETTLE_MIN_S, ranges::SETTLE_MAX_S);
            let settle = entered.unwrap_or(DEFAULT_SETTLE_TIME_S);
            details.push(kv(
                "Settle time",
                match entered {
                    Some(_) => format!("{settle:.1} s"),
                    None => format!("{settle:.1} s (assumed)"),
                },
            ));
            let slew = dynamics::slew_time_s(DEFAULT_SLEW_DISTANCE_DEG, v, a);
            let total = slew + settle;
            let fraction = total / window;
            details.push(kv("Slew time", format!("{slew:.1} s")));
            details.push(kv(
                "Slew + settle",
                format!("{total:.1} s  ({:.0}% of the window)", fraction * 100.0),
            ));
            let status = if fraction <= limits::SLEW_PASS_WINDOW_FRACTION {
                Status::Pass
            } else if fraction <= limits::SLEW_WARN_WINDOW_FRACTION {
                Status::Warn
            } else {
                Status::Fail
            };
            let verdict = match status {
                Status::Pass => "The mount is on target well inside the window.".to_string(),
                Status::Warn => "Getting on target eats a significant part of the window.".to_string(),
                _ => "The mount cannot get on target in time to make use of the pass.".to_string(),
            };
            RegimeCheck { component: Component::Mount, title, status, details, verdict }
        }
        (Some(v), None) => {
            let floor = DEFAULT_SLEW_DISTANCE_DEG / v;
            details.push(kv("Slew time", format!("at least {floor:.1} s, ignoring ramp-up")));
            RegimeCheck {
                component: Component::Mount,
                title,
                status: Status::Info,
                details,
                verdict: format!(
                    "Axis acceleration for {} is unknown, so this is a lower bound only. Ask the vendor.",
                    m.name
                ),
            }
        }
        _ => RegimeCheck {
            component: Component::Mount,
            title,
            status: Status::Info,
            details,
            verdict: format!("Maximum slew rate for {} is unknown. Ask the vendor.", m.name),
        },
    }
}
```

Add `mount_slew_settle(cfg, &r),` to the `checks` vector in `evaluate_regimes`, after `mount_acceleration(cfg, &r),`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/regimes.rs
git commit -m "Add the slew-and-settle check

Grades slew plus settle against the regime's usable window, which in
practice means LEO alone. With acceleration unknown it still reports the
rate-only time as an explicit lower bound rather than giving up.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 12: The keyhole becomes the binding constraint

The only change to existing behaviour in this plan. The alt-az keyhole in `mount_rate` is currently rate-only; it becomes whichever of the rate and acceleration limits binds.

**The compatibility guarantee:** when `max_accel_deg_s2` is absent or implausible, the reported keyhole must be identical to today's. Step 1 tests it directly, and Step 6 diffs the demo output to prove it at the whole-program level.

**Files:**
- Modify: `src/regimes.rs:500-517` (the `MountType::AltAz` arm of `mount_rate`)

**Interfaces:**
- Consumes: `dynamics::keyhole_rad` from Task 7.
- Produces: no new names.

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module in `src/regimes.rs`:

```rust
    /// The keyhole elevation a check reported, parsed back out of its details.
    fn keyhole_elev(cfg: &Config, site: &Site) -> f64 {
        let c = check_for("LEO", "Tracking rate", cfg, site);
        let line = c
            .details
            .iter()
            .find(|d| d.contains("keyhole"))
            .expect("keyhole detail");
        line.split_whitespace()
            .find_map(|w| w.parse::<f64>().ok())
            .expect("a number in the keyhole detail")
    }

    #[test]
    fn keyhole_unchanged_when_acceleration_is_unknown() {
        // The guarantee: 0.87234 deg/s against a 50 deg/s axis gives a 1.0 deg
        // keyhole, so 89.0 deg of elevation -- exactly what this tool reported
        // before acceleration was modelled.
        let (cfg, site) = fixture(Some(l350(None)));
        assert!(close(keyhole_elev(&cfg, &site), 89.0, 0.05));
        let c = check_for("LEO", "Tracking rate", &cfg, &site);
        assert!(c.details.iter().any(|d| d.contains("rate limit")));
    }

    #[test]
    fn keyhole_unchanged_when_acceleration_is_nonsense() {
        for bad in [0.0, -5.0, f64::NAN] {
            let (cfg, site) = fixture(Some(l350(Some(bad))));
            assert!(close(keyhole_elev(&cfg, &site), 89.0, 0.05), "rating {bad} moved the keyhole");
        }
    }

    #[test]
    fn keyhole_tightens_when_acceleration_is_known() {
        let (cfg, site) = fixture(Some(l350(Some(10.0))));
        assert!(close(keyhole_elev(&cfg, &site), 88.32, 0.05));
        let c = check_for("LEO", "Tracking rate", &cfg, &site);
        assert!(c.details.iter().any(|d| d.contains("acceleration limit")));
    }

    #[test]
    fn a_low_acceleration_rating_warns_on_the_keyhole() {
        // 0.5 deg/s^2 pushes the keyhole to 82.5 deg, below
        // KEYHOLE_WARN_ELEV_DEG, so the check can no longer pass.
        let (cfg, site) = fixture(Some(l350(Some(0.5))));
        assert!(close(keyhole_elev(&cfg, &site), 82.48, 0.05));
        let c = check_for("LEO", "Tracking rate", &cfg, &site);
        assert!(c.status >= Status::Warn);
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test regimes::tests::keyhole`
Expected: `keyhole_unchanged_when_acceleration_is_unknown` fails on the missing `"rate limit"` detail; `keyhole_tightens_when_acceleration_is_known` fails because the elevation is still 89.0.

- [ ] **Step 3: Write the implementation**

In `src/regimes.rs`, replace the `MountType::AltAz` arm of `mount_rate` with:

```rust
        MountType::AltAz => {
            // Near the zenith the azimuth axis must sweep through the same
            // atan form as the pass itself, so both the rate and the
            // acceleration limits constrain the same keyhole. Report whichever
            // binds; with acceleration unknown that is the rate limit, which
            // is what this check reported before acceleration was modelled.
            let omega_rad = required_deg_s.to_radians();
            let accel_rad = plausible(
                m.max_accel_deg_s2,
                ranges::ACCEL_MIN_DEG_S2,
                ranges::ACCEL_MAX_DEG_S2,
            )
            .map(f64::to_radians);
            let (z_min_rad, binding) = dynamics::keyhole_rad(omega_rad, max.to_radians(), accel_rad);
            let elev = 90.0 - z_min_rad.to_degrees();
            details.push(kv("Highest pass followable (alt-az keyhole)", format!("{elev:.1} deg elevation")));
            details.push(kv("Keyhole set by", format!("{binding} limit")));
            let k = if elev >= limits::KEYHOLE_PASS_ELEV_DEG {
                Status::Pass
            } else if elev >= limits::KEYHOLE_WARN_ELEV_DEG {
                Status::Warn
            } else {
                Status::Fail
            };
            if k > status {
                status = k;
            }
            verdict.push_str(&format!(
                " Passes peaking above {elev:.1} deg would outrun the azimuth axis near the zenith."
            ));
        }
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test`
Expected: PASS.

- [ ] **Step 5: Confirm the demo moved only where it should**

Run: `cargo run -- --demo > target/demo-task12.txt && diff target/demo-baseline.txt target/demo-task12.txt`
Expected: the only differences are the new `Acceleration` and `Slew and settle` checks from Tasks 10 and 11, plus a new `Keyhole set by ... rate limit` line. **No keyhole elevation may change.** Every preset mount has `max_accel_deg_s2: null`, so every keyhole figure must match the pre-change output exactly. If one moved, the `plausible` filter is not being applied.

- [ ] **Step 6: Commit**

```bash
git add src/regimes.rs
git commit -m "Report the binding alt-az keyhole, not the rate-only one

The rate and acceleration limits constrain one physical keyhole, so showing
two numbers would be misleading. With acceleration unknown the figure is
unchanged, which the tests and the demo diff both hold.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 13: The detection check

**Review Focus item 4 is covered here.** An exposure override long enough to trail the target off the sensor would otherwise produce a confident SNR for a target that left the field mid-exposure.

**Deviation from the spec, deliberate:** the spec's §10 asks for a `detection_info_not_promoted_by_cap` test. The detection check as designed has no `Info` path — every branch computes a number — so that test would be vacuous. It is replaced by `detection_fail_is_not_changed_by_cap`, which pins the same property (the cap is an `== Pass` test, not a `.max()`) on a status that actually occurs.

**Files:**
- Modify: `src/regimes.rs`

**Interfaces:**
- Consumes: everything from Tasks 2-6, plus `REFERENCE_TARGET_CROSS_SECTION_M2`, `REFERENCE_TARGET_ALBEDO`, `DEFAULT_PHASE_FACTOR`, `DETECT_SNR_THRESHOLD`, `SNR_PASS`, `SNR_TRIVIAL`, `ARCSEC_PER_DEGREE`.
- Produces: `fn residual_rate_arcsec_s(r: &Regime) -> f64` and `fn system_detection(cfg: &Config, ev: &Evaluation, r: &Regime, site: &Site) -> RegimeCheck`, wired into `evaluate_regimes`.

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module in `src/regimes.rs`:

```rust
    fn detection(key: &str, cfg: &Config, site: &Site) -> RegimeCheck {
        check_for(key, "Detection", cfg, site)
    }

    #[test]
    fn residual_rate_follows_the_tracking_mode() {
        for r in regimes() {
            let residual = residual_rate_arcsec_s(&r);
            match r.mode {
                // The mount holds a rate-tracked or stared target still.
                TrackingMode::RateTrack | TrackingMode::Stare => {
                    assert!(close(residual, 0.0, 1e-12), "{} should not trail", r.key)
                }
                // Under sidereal tracking the target drifts against the stars.
                TrackingMode::Sidereal => assert!(close(residual, r.rate_vs_stars, 1e-12)),
            }
        }
    }

    #[test]
    fn detection_caps_at_warn_when_inputs_are_assumed() {
        // Every preset carries null QE, throughput and read noise, and the
        // fixture's site carries no sky brightness, so nothing here is
        // entered. GEO would otherwise grade PASS on SNR 526.
        let (cfg, site) = fixture(Some(l350(Some(10.0))));
        let c = detection("GEO", &cfg, &site);
        assert_eq!(c.status, Status::Warn);
        assert!(c.details.iter().any(|d| d.contains("quantum efficiency")));
        assert!(c.verdict.contains("assumed"));
    }

    #[test]
    fn detection_passes_when_every_input_is_entered() {
        let (mut cfg, mut site) = fixture(Some(l350(Some(10.0))));
        cfg.telescope.throughput = Some(0.85);
        cfg.camera.qe = Some(0.80);
        cfg.camera.read_noise_e = Some(3.0);
        site.sky_mag_arcsec2 = Some(21.0);
        let c = detection("GEO", &cfg, &site);
        assert_eq!(c.status, Status::Pass);
        assert!(!c.verdict.contains("assumed"));
    }

    #[test]
    fn detection_fail_is_not_changed_by_cap() {
        // The cap must be an `== Pass` test, not `.max(Warn)`: a FAIL stays a
        // FAIL whether or not inputs were assumed.
        let (mut cfg, site) = fixture(Some(l350(Some(10.0))));
        cfg.target_mag_override = Some(30.0); // far beyond any limit
        let c = detection("GEO", &cfg, &site);
        assert_eq!(c.status, Status::Fail);
    }

    #[test]
    fn cislunar_is_the_only_regime_detection_actually_grades() {
        // Every regime nearer than the Moon exceeds SNR_TRIVIAL on a 14-inch
        // at 30 s, which is the point: brightness is not what limits them.
        let (mut cfg, mut site) = fixture(Some(l350(Some(10.0))));
        cfg.telescope.throughput = Some(0.85);
        cfg.camera.qe = Some(0.80);
        cfg.camera.read_noise_e = Some(3.0);
        site.sky_mag_arcsec2 = Some(21.0);
        for key in ["LEO", "MEO", "GEO", "HEO"] {
            let c = detection(key, &cfg, &site);
            assert!(
                c.verdict.contains("not the limiting factor"),
                "{key} should be trivially detectable"
            );
        }
        let cis = detection("CIS", &cfg, &site);
        assert!(!cis.verdict.contains("not the limiting factor"));
        assert_eq!(cis.status, Status::Pass); // SNR 14.9, above SNR_PASS
    }

    #[test]
    fn stationary_regimes_share_a_limiting_magnitude() {
        // LEO, MEO, GEO and HEO all hold the target still, so they share an
        // exposure, a zero trail, a footprint and a noise budget, and
        // therefore a limiting magnitude of 20.06. They differ only in target
        // magnitude. A guard against the noise terms picking up a spurious
        // range dependence.
        let (mut cfg, mut site) = fixture(Some(l350(Some(10.0))));
        cfg.telescope.throughput = Some(0.85);
        cfg.camera.qe = Some(0.80);
        cfg.camera.read_noise_e = Some(3.0);
        site.sky_mag_arcsec2 = Some(21.0);
        let limit_of = |key: &str| -> f64 {
            let c = detection(key, &cfg, &site);
            let line = c
                .details
                .iter()
                .find(|d| d.contains("Limiting magnitude"))
                .expect("limiting magnitude detail");
            line.split_whitespace().find_map(|w| w.parse::<f64>().ok()).expect("a number")
        };
        let leo = limit_of("LEO");
        assert!(close(leo, 20.06, 0.02));
        for key in ["MEO", "GEO", "HEO"] {
            assert!(close(limit_of(key), leo, 0.001), "{key} limiting magnitude differs from LEO");
        }
    }

    #[test]
    fn detection_warns_when_the_trail_runs_off_the_sensor() {
        // Review Focus 4: a long exposure on a sidereally tracked cislunar
        // target streaks it out of the field. Reporting a confident SNR for a
        // target that left the sensor would be worse than useless.
        let (mut cfg, mut site) = fixture(Some(l350(Some(10.0))));
        cfg.telescope.throughput = Some(0.85);
        cfg.camera.qe = Some(0.80);
        cfg.camera.read_noise_e = Some(3.0);
        site.sky_mag_arcsec2 = Some(21.0);
        // The short side of this field is 1.31 deg = 4716". At 0.549"/s that
        // takes 8,590 s to cross; 20,000 s is comfortably past it.
        cfg.exposure_override_s = Some(20_000.0);
        let c = detection("CIS", &cfg, &site);
        assert!(c.details.iter().any(|d| d.contains("longer than the short side")));
        assert!(c.verdict.contains("streaks off the sensor"));
        assert!(c.status >= Status::Warn, "a target off the sensor must not be a PASS");
    }

    #[test]
    fn detection_honours_an_entered_target_magnitude() {
        let (mut cfg, site) = fixture(Some(l350(Some(10.0))));
        cfg.target_mag_override = Some(14.0);
        let c = detection("GEO", &cfg, &site);
        assert!(c.details.iter().any(|d| d.contains("14.00") && d.contains("entered")));
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test regimes::tests::detection`
Expected: FAIL to compile, `cannot find function residual_rate_arcsec_s in this scope`.

- [ ] **Step 3: Write the implementation**

Add to `src/regimes.rs` a new "System checks" section. Extend the `use crate::constants::{...}` line with `ARCSEC_PER_DEGREE, DEFAULT_PHASE_FACTOR, REFERENCE_TARGET_ALBEDO, REFERENCE_TARGET_CROSS_SECTION_M2` and add `use crate::photometry::{self, Photometry};`.

```rust
// ---------------------------------------------------------------------------
// System checks
// ---------------------------------------------------------------------------

/// How fast the target moves across the sensor, arcsec/s, given how it is tracked.
///
/// A rate-tracked target is held still by the mount, and an Earth-fixed target
/// in stare mode is still by definition; the stars are what trail in both
/// cases. Under sidereal tracking it is the other way round: the target drifts
/// against the tracked stars at its rate against them.
fn residual_rate_arcsec_s(r: &Regime) -> f64 {
    match r.mode {
        TrackingMode::RateTrack | TrackingMode::Stare => 0.0,
        TrackingMode::Sidereal => r.rate_vs_stars,
    }
}

/// Is the target bright enough for this configuration to detect?
fn system_detection(cfg: &Config, ev: &Evaluation, r: &Regime, site: &Site) -> RegimeCheck {
    let title = "Detection";
    let p = Photometry::resolve(&cfg.telescope, &cfg.camera, site);
    let (target_mag, mag_from) = match cfg.target_mag_override {
        Some(m) => (m, "entered"),
        None => (
            photometry::derived_target_mag(
                REFERENCE_TARGET_CROSS_SECTION_M2,
                REFERENCE_TARGET_ALBEDO,
                r.range_km,
                DEFAULT_PHASE_FACTOR,
            ),
            "derived",
        ),
    };
    let residual = residual_rate_arcsec_s(r);
    let (exposure, exp_from) = match cfg.exposure_override_s {
        Some(t) => (t, "entered"),
        None => (photometry::trail_limited_exposure_s(site.seeing_arcsec, residual), "derived"),
    };

    let scale = ev.metrics.plate_scale;
    let area = ev.metrics.effective_area_m2;
    let trail = photometry::trail_arcsec(residual, exposure);
    let n_px = photometry::footprint_px(site.seeing_arcsec, trail, scale);
    let signal = photometry::signal_e_per_s(target_mag, area, p.qe, p.throughput) * exposure;
    let sky = photometry::sky_e_per_px_s(p.sky_mag_arcsec2, scale, area, p.qe, p.throughput)
        * exposure
        * n_px;
    let snr = photometry::snr(signal, sky, p.read_noise_e, n_px);
    let noise_variance = sky + p.read_noise_e * p.read_noise_e * n_px;
    let m_limit = photometry::limiting_mag(
        limits::DETECT_SNR_THRESHOLD,
        noise_variance,
        photometry::signal_coefficient(area, p.qe, p.throughput, exposure),
    );

    let mut details = vec![
        kv("Target magnitude", format!("{target_mag:.2} ({mag_from})")),
        kv("Exposure", format!("{exposure:.3} s ({exp_from})")),
        kv("Trail", format!("{trail:.2}\" ({:.1} px)", trail / scale)),
        kv("Footprint", format!("{n_px:.1} px")),
        kv("Signal / sky", format!("{signal:.0} e- / {sky:.0} e-")),
        kv("SNR", format!("{snr:.1}")),
        kv("Limiting magnitude", format!("{m_limit:.2}")),
        kv("Margin", format!("{:+.2} mag", m_limit - target_mag)),
    ];

    let trivial = snr >= limits::SNR_TRIVIAL;
    let mut status = if snr >= limits::SNR_PASS {
        Status::Pass
    } else if snr >= limits::DETECT_SNR_THRESHOLD {
        Status::Warn
    } else {
        Status::Fail
    };
    let mut verdict = if trivial {
        format!(
            "Detection is not the limiting factor here, so choose exposure for saturation and timing instead. What limits this regime is {}.",
            r.limiting_factor
        )
    } else if status == Status::Pass {
        format!("Detectable with margin: SNR {snr:.0} against a threshold of {:.0}.", limits::DETECT_SNR_THRESHOLD)
    } else if status == Status::Warn {
        "Marginal: detectable, but close enough to the threshold that conditions will decide it.".to_string()
    } else {
        "Too faint to detect in this configuration.".to_string()
    };

    // None of the above means anything if the target left the sensor.
    let short_side_arcsec = ev.metrics.fov_w_deg.min(ev.metrics.fov_h_deg) * ARCSEC_PER_DEGREE;
    if trail > short_side_arcsec {
        details.push(kv("Trail vs field", "longer than the short side of the field".to_string()));
        verdict.push_str(" The trail is longer than the field, so the target streaks off the sensor during the exposure: shorten it.");
        if status == Status::Pass {
            status = Status::Warn;
        }
    }

    // Never claim a PASS on numbers the user did not supply. Checked against
    // Pass explicitly: Status is ordered Info < Pass < Warn < Fail, so a
    // `.max(Warn)` here would also promote an Info.
    if p.any_assumed() {
        details.push(kv("Assumed inputs", p.assumed.join(", ")));
        if status == Status::Pass {
            status = Status::Warn;
            verdict.push_str(&format!(
                " Not graded PASS because these were assumed rather than entered: {}.",
                p.assumed.join(", ")
            ));
        }
    }

    RegimeCheck { component: Component::System, title, status, details, verdict }
}
```

Add `system_detection(cfg, ev, &r, site),` to the `checks` vector in `evaluate_regimes`, at the end.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test`
Expected: PASS. The `Component::System` dead-code warning from Task 9 is now gone.

- [ ] **Step 5: Commit**

```bash
git add src/regimes.rs
git commit -m "Add the system detection check

Derives target magnitude and exposure, computes trailed SNR and an absolute
limiting magnitude, and refuses to grade PASS on any input the user did not
supply. A trail longer than the field is called out rather than silently
reported as a detection.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 14: Report the new results

The checks exist but nothing prints the `System` component. `report.rs` iterates a hard-coded array of three components in two places and formats a three-letter status cell in a third.

**Files:**
- Modify: `src/report.rs:30-33` (site line), `:84-112` (comparison header), `:130-152` (regime grid), `:155-186` (formulas), `:207-220` (summary), `:238` (detail loop)

**Interfaces:**
- Consumes: `Component::System`.
- Produces: no new names.

- [ ] **Step 1: Add sky brightness to the site line**

In `print_evaluation`, replace the site line:

```rust
    println!(
        " Site:      seeing {:.2}\" FWHM, wavelength {:.2} um, sky {}",
        site.seeing_arcsec,
        site.wavelength_um,
        match site.sky_mag_arcsec2 {
            Some(s) => format!("{s:.2} mag/arcsec^2"),
            None => format!("{:.2} mag/arcsec^2 (assumed)", DEFAULT_SKY_MAG_ARCSEC2),
        }
    );
```

Add `use crate::constants::DEFAULT_SKY_MAG_ARCSEC2;` to the top of `src/report.rs`.

- [ ] **Step 2: Add `System` to the per-regime detail loop**

In `print_regime_details`, extend the component array:

```rust
        for comp in [Component::Telescope, Component::Camera, Component::Mount, Component::System] {
```

and the overall line below it:

```rust
        println!(
            "\n  {} overall: telescope {}, camera {}, mount {}, system {}",
            g.key,
            word(r.component_status(Component::Telescope)),
            word(r.component_status(Component::Camera)),
            word(r.component_status(Component::Mount)),
            word(r.component_status(Component::System)),
        );
```

Also extend the regime header line to show the window:

```rust
        println!(
            "    range {:.0} km | vs stars {:.2}\"/s | vs ground {:.2}\"/s | prediction error {:.0} km | usual mode: {} | window {}",
            g.range_km,
            g.rate_vs_stars,
            g.rate_vs_ground,
            g.ephemeris_uncertainty_km,
            g.mode.describe(),
            match g.usable_window_s {
                Some(w) => format!("{w:.0} s"),
                None => "unlimited".to_string(),
            }
        );
```

- [ ] **Step 3: Add a `System` column to the regime summary**

In `print_regime_summary`:

```rust
pub fn print_regime_summary(ev: &Evaluation) {
    println!("\n[----] Orbital regimes (worst status per component; details via the menu or --demo)");
    println!(
        "       {:<31} {:<10} {:<8} {:<8} {:<8} {}",
        "Regime", "Telescope", "Camera", "Mount", "System", "Overall"
    );
    for r in &ev.regimes {
        println!(
            "       {:<31} {:<10} {:<8} {:<8} {:<8} {}",
            format!("{} ({})", r.regime.key, r.regime.name),
            word(r.component_status(Component::Telescope)),
            word(r.component_status(Component::Camera)),
            word(r.component_status(Component::Mount)),
            word(r.component_status(Component::System)),
            word(r.overall())
        );
    }
}
```

- [ ] **Step 4: Widen the comparison grid to four letters**

In `print_comparison`, the regime grid becomes `T/C/M/S`. Widen the cells from `{:<7}` to `{:<9}`:

```rust
    if let Some(first) = evals.first() {
        println!("\n Orbital regimes, telescope/camera/mount/system (P=pass W=warn F=fail i=info):");
        let header: Vec<String> = first.regimes.iter().map(|r| format!("{:<9}", r.regime.key)).collect();
        println!("{:<28}  {}", "", header.join(" "));
        for e in evals {
            let cells: Vec<String> = e
                .regimes
                .iter()
                .map(|r| {
                    format!(
                        "{:<9}",
                        format!(
                            "{}/{}/{}/{}",
                            letter(r.component_status(Component::Telescope)),
                            letter(r.component_status(Component::Camera)),
                            letter(r.component_status(Component::Mount)),
                            letter(r.component_status(Component::System))
                        )
                    )
                })
                .collect();
            println!("{:<28}  {}", truncate(&e.label, 28), cells.join(" "));
        }
    }
```

- [ ] **Step 5: Add the new formulas to the summary**

In `print_formulas`, extend the "Orbital regimes" block with:

```text
   Peak accel        alpha = 0.6495 x omega^2        (0.6495 = 3 sqrt(3) / 8)
   Accel keyhole     z >= omega x sqrt(0.6495 / max accel)
   Keyhole reported  the larger of the rate and acceleration limits
   Slew time         t = v/a + D/v,  or 2 sqrt(D/a) if D < v^2/a
   Target magnitude  m = -26.74 - 2.5 x log10(albedo x area x phase / (pi x d^2))
   Signal            e-/s = 8.9e9 x 10^(-0.4 m) x area x QE x throughput
   Sky               e-/px/s = same, at the sky magnitude, x plate scale^2
   Trail-limited t   exposure = seeing / residual rate   (capped at 30 s)
   Footprint         (seeing / scale) x ((seeing + trail) / scale)
   SNR               S / sqrt(S + B + R^2 x n)
   Limiting mag      invert SNR = 5:  S = (T^2 + sqrt(T^4 + 4 T^2 N)) / 2
```

- [ ] **Step 6: Run the suite and look at the output**

Run: `cargo test && cargo run -- --demo | head -80`
Expected: tests PASS. In the output, confirm the site line names the assumed sky brightness, the regime summary has a System column, and the comparison grid shows four letters per regime.

- [ ] **Step 7: Check the comparison table still aligns**

Run: `cargo run -- --demo | tail -20`
Expected: the regime grid's header keys line up over their cells. Five regimes at 9 columns plus separators is 49 characters after a 28-character label; that fits an 80-column terminal at 77. If it overflows, do not narrow the cells — the four letters and three slashes need 7 characters plus a space.

- [ ] **Step 8: Commit**

```bash
git add src/report.rs
git commit -m "Report the system component and the new formulas

The regime grid goes from T/C/M to T/C/M/S, and the site line now says when
sky brightness was assumed rather than entered.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 15: Collect the new inputs

Everything works from presets and defaults; this task lets a user supply the real numbers. Follow the existing prompt idiom exactly: `ask_optional` for a spec-sheet figure, `ask_optional_hint` where a blank answer means something specific, and the preset-mount pattern at `main.rs:145` of asking only for what the preset lacks.

**Files:**
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: everything prior.
- Produces: no new names.

- [ ] **Step 1: Ask for sky brightness**

In `run_interactive`, replace the `Site` construction:

```rust
    println!("\nSite conditions. Sky brightness is V magnitudes per square arcsecond:");
    println!("about 21.9 at a dark rural site, 21.0 rural, 18.5 suburban. Larger is darker.");
    let mut site = Site {
        seeing_arcsec: input::ask_positive("Typical seeing FWHM at your site, arcsec", Some(DEFAULT_SEEING_ARCSEC)),
        wavelength_um: DEFAULT_WAVELENGTH_UM,
        sky_mag_arcsec2: input::ask_optional_hint(
            "Sky brightness, mag/arcsec^2",
            "blank to assume 21.0 and have detection capped at WARN",
            false,
        ),
    };
```

- [ ] **Step 2: Let the site menu item change it**

Replace menu entry 3 and its handler:

```rust
            format!(
                "Change site conditions (seeing {:.2}\", sky {})",
                site.seeing_arcsec,
                match site.sky_mag_arcsec2 {
                    Some(s) => format!("{s:.2}"),
                    None => "assumed".to_string(),
                }
            ),
```
```rust
            3 => {
                site.seeing_arcsec = input::ask_positive("New seeing FWHM, arcsec", Some(site.seeing_arcsec));
                site.sky_mag_arcsec2 = input::ask_optional_hint(
                    "Sky brightness, mag/arcsec^2",
                    "blank to assume 21.0",
                    false,
                );
                println!("Site updated. All configurations will be re-evaluated.");
            }
```

- [ ] **Step 3: Ask for throughput, QE, and mount dynamics**

In `custom_telescope`, before building the `Telescope`:

```rust
    let throughput = input::ask_optional_hint(
        "Optical throughput (fraction, e.g. 0.85)",
        "blank to assume 0.85",
        false,
    );
```
and set `throughput,` in the literal.

In `custom_camera`, after `read_noise_e`:

```rust
    let qe = input::ask_optional_hint("Peak quantum efficiency (fraction, e.g. 0.80)", "blank to assume 0.80", false);
```
and set `qe,` in the literal.

In `custom_mount`, replace the `Mount` literal's tail:

```rust
        max_slew_deg_s: input::ask_optional("Maximum slew rate, deg/s", false),
        max_accel_deg_s2: input::ask_optional("Maximum axis acceleration, deg/s^2", false),
        settle_time_s: input::ask_optional_hint("Settle time after a slew, s", "blank to assume 2.0", true),
        pointing_rms_arcsec: input::ask_optional("Pointing accuracy after modeling, arcsec RMS", false),
```

In `build_config`, extend the preset-mount branch to ask for what the preset lacks, following the pattern already there:

```rust
        if m.max_accel_deg_s2.is_none() {
            m.max_accel_deg_s2 = input::ask_optional("Maximum axis acceleration, deg/s^2", false);
        }
```

- [ ] **Step 4: Ask for the target and exposure overrides**

In `build_config`, after `timestamp_accuracy_ms`:

```rust
    println!("\nDetection inputs. Leave both blank to use values derived per regime:");
    println!("a 10 m^2 target at 0.2 albedo and full phase, exposed until its trail");
    println!("reaches one seeing disk (capped at 30 s).");
    let target_mag_override = input::ask_optional_hint(
        "Target apparent magnitude",
        "blank for the derived value",
        false,
    );
    let exposure_override_s = input::ask_optional_hint(
        "Exposure time, s",
        "blank for the trail-limited value",
        false,
    );
```

and set both in the `Config` literal, replacing the `None`s from Task 1.

- [ ] **Step 5: Update the help text**

In `print_help`, replace the regime sentence:

```rust
Then evaluates the telescope, camera, mount and the configuration as a whole
against five orbital regimes: LEO, MEO, GEO, HEO (Molniya) and cislunar,
covering tracking rate, axis acceleration, slew-and-settle timing, timing
accuracy, shutter skew, acquisition and whether the target is bright enough
to detect.
```

- [ ] **Step 6: Leave the demo on defaults, deliberately**

Do not give `run_demo` real photometric values. Every new field stays `None` so that `--demo` exercises the WARN caps and shows a reader exactly what the tool declines to claim without real data. Add a line to the demo banner:

```rust
    println!("scope-eval demo: built-in presets, seeing {:.1}\"", site.seeing_arcsec);
    println!("No QE, throughput, sky brightness or mount dynamics are entered, so");
    println!("detection is capped at WARN throughout. That is the point of the demo.");
```

- [ ] **Step 7: Run the suite and walk the prompts**

Run: `cargo test`
Expected: PASS.

Run: `printf '2.5\n21.0\n1\n1\n1\n\n10\n\n0.1\n\n\n\n' | cargo run`
Expected: it reaches the main menu, accepts a configuration from presets, and prints an evaluation whose detection check is graded (not capped) for sky brightness but still capped for QE, throughput and read noise. Press through to Quit.

- [ ] **Step 8: Commit**

```bash
git add src/main.rs
git commit -m "Collect sky brightness, throughput, QE, and mount dynamics

The demo deliberately enters none of them, so --demo shows the WARN caps
rather than hiding them behind invented numbers.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 16: Documentation

The three limitation bullets this work retires are the reason the work exists. Removing them without putting the replacements in their place would leave the README claiming less than the tool does, and the new assumptions undocumented.

**Files:**
- Modify: `README.md`

**Interfaces:**
- Consumes: the worked examples in §8 of the spec.
- Produces: nothing in code.

- [ ] **Step 1: Retire the three bullets**

In "Assumptions and limitations", delete these three lines exactly:

```
* **No absolute limiting magnitude.** Depth is reported relative to the reference only. Absolute limiting magnitude depends on sky brightness, exposure time, detector quantum efficiency, filters and detection thresholds, none of which are modeled.
* **No acceleration model.** The mount check compares maximum rates only. Real LEO tracking also depends on axis acceleration, servo bandwidth and how smoothly the software follows the path.
* **No brightness model.** Regime checks say what usually limits detection but don't estimate whether a specific target is bright enough to detect.
```

- [ ] **Step 2: Add the replacement limitations**

In their place:

```
* **One representative target.** Derived magnitudes assume a 10 m^2 object at 0.2 albedo, so the figures vary between regimes only through range. Real objects span orders of magnitude in size and brightness. Enter a target magnitude to override it.
* **Full phase assumed.** The derived magnitude uses a phase factor of 1.0, the brightest case. A target near quadrature is roughly a magnitude fainter.
* **Sky brightness is a single number.** No dependence on elevation, moon phase or airmass, and no extinction term.
* **No saturation model.** Detector full-well depth is not modeled, so bright LEO targets report implausibly high SNR. The check reports these as "detection is not the limiting factor" rather than as a number to act on.
* **Servo behaviour is still not modeled.** The acceleration model covers peak axis acceleration, the acceleration-limited keyhole and slew timing. Servo bandwidth, closed-loop following error and path-following smoothness are not included, because vendors do not publish the inputs.
* **Slew distance is assumed.** The slew-and-settle check uses a 90-degree acquisition slew and, where the mount does not publish one, a 2-second settle.
* **Photometric defaults are generic.** When QE, throughput, sky brightness or read noise are not entered, documented generic values are substituted and the detection check is capped at WARN. It will never report PASS on a quantum efficiency it assumed.
```

- [ ] **Step 3: Add the new inputs to the input table**

Add these rows to the inputs table (around line 129):

| Input | Unit | Source | Used by |
|---|---|---|---|
| Sky background brightness | mag/arcsec^2 | Site measurement or a dark-sky map (optional) | Regime: detection |
| Optical throughput | fraction | Vendor, or measured (optional) | Regime: detection |
| Quantum efficiency | fraction | Camera QE curve (optional) | Regime: detection |
| Read noise | e- RMS | Camera spec sheet (optional) | Regime: detection |
| Mount maximum acceleration | deg/s^2 | Mount spec sheet (optional) | Regime: mount acceleration, keyhole, slew |
| Mount settle time | s | Mount spec sheet or measured (optional) | Regime: slew and settle |
| Target apparent magnitude | mag | Your own catalog (optional) | Regime: detection |
| Exposure time | s | Your choice (optional) | Regime: detection |

- [ ] **Step 4: Add the formula sections**

Add a section documenting, with the derivations from spec §5.1 and §6.1 and the worked numbers from spec §8:

- the diffuse-sphere magnitude relation, with the table of five derived magnitudes (LEO 2.25, MEO 10.28, GEO 11.59, HEO 11.75, cislunar 16.67) and the note that GEO and cislunar land inside the 11-15 and 16-20 ranges real objects occupy;
- the photon zero point and its derivation from the V-band 3.64e-23 W/m^2/Hz;
- the trail-limited exposure and the footprint approximation;
- the SNR equation and the exact quadratic inversion for limiting magnitude;
- `PEAK_ACCEL_COEFF = 3 sqrt(3) / 8` with the `atan` derivation, and the LEO figure of 0.008627 deg/s^2 together with the explicit observation that this is negligible for any real mount, which is why the keyhole and slew-time results carry the acceleration model;
- the keyhole table (88.32 / 86.24 / 82.48 degrees at 10 / 2 / 0.5 deg/s^2 against the rate-only 89.00) and the statement that with acceleration unknown the figure is unchanged;
- the three slew-time worked examples (6.0 s, 2.8 s, 21.0 s);
- the detection table from spec §8.5, including that all four stationary-target regimes share a limiting magnitude of 20.06 and why.

- [ ] **Step 5: Add the new constants to the constants table**

Add a row per constant from spec §9 to the constants table (around line 754), including the `plausible_ranges` values and a sentence explaining that out-of-range inputs are treated as not entered rather than trusted.

- [ ] **Step 6: Update the preset tables**

In "Presets and their sources", note in each table that throughput, QE and mount dynamics are not entered for any preset, and why: every value in `presets.yaml` carries a `source`, and no vendor figure was available for these.

- [ ] **Step 7: Check the README against the tool**

Run: `cargo run -- --help && cargo run -- --demo | head -40`
Expected: nothing the README now claims is absent from the output, and nothing the output shows is undocumented. Specifically confirm the three deleted bullets describe nothing the tool still fails to do.

Run: `grep -n "No acceleration model\|No brightness model\|No absolute limiting magnitude" README.md`
Expected: no matches.

- [ ] **Step 8: Commit**

```bash
git add README.md
git commit -m "Document the acceleration and brightness models

Retires the three limitation bullets this work addresses and replaces them
with the assumptions the new models actually rest on: one representative
target, full phase, generic photometric defaults, an assumed slew distance,
and no saturation or servo model.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Final verification

Run every gate from spec §13 before calling this done:

- [ ] `cargo test` — all tests pass, including every test named in this plan
- [ ] `cargo build` — no new warnings beyond the pre-existing `dead_code` on `Metrics`
- [ ] `cargo run -- --demo` — runs end to end, shows the new checks, and shows detection capped at WARN throughout
- [ ] `cargo run -- --help` — mentions the new checks
- [ ] `git diff --stat main` — `presets.yaml` changed only by added `null` keys, comments and `source` text; no invented vendor values
- [ ] `grep -c "No acceleration model\|No brightness model\|No absolute limiting magnitude" README.md` returns 0
- [ ] Keyhole elevations in `--demo` match `target/demo-baseline.txt` exactly, since no preset mount publishes an acceleration figure
