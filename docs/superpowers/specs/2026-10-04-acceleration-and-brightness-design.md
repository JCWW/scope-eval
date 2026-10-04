# Mount acceleration and target brightness models

Date: 2026-10-04
Status: approved design, ready for implementation planning

## 1. Purpose

`scope-eval` currently documents two things it does not model
(README.md, "Assumptions and limitations"):

> **No acceleration model.** The mount check compares maximum rates only.
> Real LEO tracking also depends on axis acceleration, servo bandwidth and
> how smoothly the software follows the path.

> **No brightness model.** Regime checks say what usually limits detection
> but don't estimate whether a specific target is bright enough to detect.

A third bullet, "No absolute limiting magnitude", falls out with them once
there is an SNR model to invert.

This design retires all three by adding the models, so the tool can answer
two questions it cannot answer today:

* Can this mount actually catch a LEO pass, not just match its peak rate?
* Can this configuration detect a target of a given apparent magnitude in a
  given regime, and how faint can it go?

## 2. Scope

In scope: a photometric detection model (signal, sky, read noise, trailing,
SNR, limiting magnitude), a derived target-brightness model, mount axis
acceleration, an acceleration-limited alt-az keyhole, and a slew-and-settle
time budget.

The work decomposes into three phases that can land independently:
`photometry.rs` with its tests, `dynamics.rs` with its tests, then the
wiring into `regimes.rs` and the input/report/README surface. Each phase
leaves `cargo test` green and `--demo` working.

Out of scope, and to remain documented limitations: servo bandwidth and
closed-loop following error (vendors do not publish the inputs); detector
full-well and saturation; filters and colour terms; atmospheric extinction
as a function of elevation; real ephemeris catalogues.

## 3. Decisions

1. **Target brightness is user-entered, defaulting to a derived value.** The
   tool derives an apparent magnitude per regime from a representative
   target and the regime's range; the user may override it. A full SNR model
   sits behind it either way.
2. **Acceleration is modeled in three places:** peak tracking acceleration,
   an acceleration-limited alt-az keyhole, and a slew-and-settle budget.
   A pure tracking-acceleration check alone would pass unconditionally
   (see 8.2) and tell the operator nothing; the keyhole is where
   acceleration actually binds.
3. **Missing photometric inputs are substituted with documented defaults,
   but the check may then never report PASS.** The status is capped at WARN
   and the assumed inputs are named in the output. This tool informs
   purchasing decisions; it must not report a confident PASS resting on an
   assumed quantum efficiency.
4. **An absolute limiting magnitude is reported per regime.** The
   "No absolute limiting magnitude" bullet is also retired. The WARN cap in
   decision 3 is what makes this safe to publish.
5. **The representative target is two constants, not per-regime fields.**
   Every regime shares one 10 m^2 / 0.2-albedo target, so the derived
   magnitude varies per regime only through `range_km`. This states the
   model's claim plainly -- the same object, moved further away -- and avoids
   three fields where two constants do.

## 4. Model state

Every new field is `Option`, so "not entered" stays distinguishable from a
value. That distinction is what drives the WARN cap, and it is the only
mechanism that does.

| Type | New field | Rationale |
|---|---|---|
| `Site` | `sky_mag_arcsec2: Option<f64>` | Site property, like seeing. `Option<f64>` is `Copy`, so `Site: Copy` is preserved. |
| `Telescope` | `throughput: Option<f64>` | Mirror coatings and corrector belong to the telescope, not the camera. |
| `Camera` | `qe: Option<f64>` | Sensor property. |
| `Mount` | `max_accel_deg_s2: Option<f64>` | Spec sheet, where published. |
| `Mount` | `settle_time_s: Option<f64>` | Spec sheet or measured. |
| `Regime` | `usable_window_s: Option<f64>` | Acquisition window; `None` means effectively unlimited. Values in 4.1. |
| `Config` | `target_mag_override: Option<f64>` | `None` selects the derived magnitude. |
| `Config` | `exposure_override_s: Option<f64>` | `None` selects the trail-limited exposure. |

`Camera::read_noise_e` already exists but is documented at
`src/model/camera.rs:18` as "optional, illustrative only". This design makes
it load-bearing; that doc comment must be corrected rather than left to
mislead.

### 4.1 Values for `usable_window_s`

| Regime | Value | Rationale |
|---|---|---|
| LEO | `Some(300.0)` | A 500 km pass is above useful elevation for roughly five minutes. |
| MEO | `None` | Hours above the horizon. |
| GEO | `None` | Earth-fixed; always available. |
| HEO | `None` | Near apogee the target moves slowly for hours. |
| Cislunar | `None` | Available all night. |

LEO is therefore the only regime where the slew-and-settle check grades
rather than reporting `Info`, which is the intended behaviour: it is the
only regime with real window pressure.

### 4.2 Provenance

One resolution type, rather than an `assumed` flag smeared across structs:

```rust
/// Photometric inputs with defaults substituted, and the names of whatever
/// was assumed rather than entered.
pub struct Photometry {
    pub qe: f64,
    pub throughput: f64,
    pub sky_mag_arcsec2: f64,
    pub read_noise_e: f64,
    pub assumed: Vec<&'static str>,
}

impl Photometry {
    pub fn resolve(t: &Telescope, c: &Camera, site: &Site) -> Self;
    pub fn any_assumed(&self) -> bool;
}
```

One place substitutes defaults, one place reports what it substituted, one
place caps the grade.

### 4.3 Wire format

`optics_dto`, `camera_dto` and `mount_dto` gain matching fields marked
`#[serde(default)]`, so `presets.yaml` parses identically before and after
this change.

## 5. New module: `src/photometry.rs`

A pure-math peer to `checks.rs`, following that file's separation of
calculation from judgment. Numbers in, number out; no `Status`, no strings.

```
derived_target_mag(cross_section_m2, albedo, range_km, phase) -> f64
    d_m = range_km * 1000
    m = SUN_APPARENT_MAG - 2.5 * log10(albedo * cross_section * phase / (PI * d_m^2))

signal_e_per_s(mag, eff_area_m2, qe, throughput) -> f64
    PHOTONS_M2_S_MAG0 * 10^(-0.4 * mag) * eff_area_m2 * qe * throughput

sky_e_per_px_s(sky_mag_arcsec2, plate_scale, eff_area_m2, qe, throughput) -> f64
    PHOTONS_M2_S_MAG0 * 10^(-0.4 * sky_mag_arcsec2)
        * eff_area_m2 * qe * throughput * plate_scale^2

residual_rate_arcsec_s(regime) -> f64
    RateTrack | Stare -> 0.0            // target held still by the mount
    Sidereal          -> rate_vs_stars  // target trails against tracked stars

trail_arcsec(residual_rate, exposure_s) -> f64
    residual_rate * exposure_s

trail_limited_exposure_s(seeing_arcsec, residual_rate) -> f64
    if residual_rate <= 0 { MAX_EXPOSURE_S }
    else { (seeing_arcsec / residual_rate).min(MAX_EXPOSURE_S) }

footprint_px(seeing_arcsec, trail_arcsec, plate_scale) -> f64
    across = seeing / plate_scale
    along  = (seeing + trail) / plate_scale
    across * along                      // seeing disk smeared into a streak

snr(signal_e, sky_e_total, read_noise_e, n_px) -> f64
    signal_e / sqrt(signal_e + sky_e_total + read_noise_e^2 * n_px)

limiting_mag(threshold, noise_variance_e2, signal_coefficient) -> f64
```

### 5.1 Inverting SNR

`limiting_mag` must not iterate. Setting `SNR = T` and solving
`S / sqrt(S + N) = T` for the signal `S`, where `N` is the non-signal
variance `sky + read^2 * n_px`, gives a quadratic `S^2 - T^2 S - T^2 N = 0`
with the positive root:

```
S_min = (T^2 + sqrt(T^4 + 4 * T^2 * N)) / 2
```

Then, with `K = PHOTONS_M2_S_MAG0 * eff_area * qe * throughput * exposure_s`:

```
m_limit = -2.5 * log10(S_min / K)
```

This is exact, and it unit-tests as a round trip: feeding `m_limit` back
through `snr` must return `DETECT_SNR_THRESHOLD`.

## 6. New module: `src/dynamics.rs`

A second pure-math peer, so that `regimes.rs` (already 634 lines) grows only
by its check functions.

```
peak_tracking_accel_rad_s2(omega_rad_s) -> f64
    PEAK_ACCEL_COEFF * omega_rad_s^2

accel_limited_keyhole_rad(omega_rad_s, max_accel_rad_s2) -> f64
    omega_rad_s * sqrt(PEAK_ACCEL_COEFF / max_accel_rad_s2)

slew_time_s(distance_deg, max_rate_deg_s, max_accel_deg_s2) -> f64
    if distance_deg >= max_rate^2 / max_accel {
        max_rate / max_accel + distance_deg / max_rate   // trapezoidal
    } else {
        2.0 * sqrt(distance_deg / max_accel)             // triangular
    }
```

### 6.1 Where `PEAK_ACCEL_COEFF` comes from

An overhead pass at constant altitude `h` and speed `v` has angular position
`theta(t) = atan(v t / h)`. Differentiating twice and writing `u = v t / h`:

```
theta'  = (v/h) / (1 + u^2)
theta'' = -2 (v/h)^2 * u / (1 + u^2)^2
```

`|theta''|` peaks at `u = 1/sqrt(3)`, giving

```
PEAK_ACCEL_COEFF = (2/sqrt(3)) / (4/3)^2 = 3*sqrt(3)/8 = 0.6495190528...
```

so `alpha_peak = PEAK_ACCEL_COEFF * omega_max^2` where `omega_max = v/h`.

This is a derived constant, not a tuned threshold, and its closed form
belongs in the doc comment. It therefore lives in the physical-constants
section of `constants.rs`, not in `regimes_limits`.

### 6.2 The acceleration-limited keyhole

Near the zenith an alt-az azimuth axis sweeps through the same `atan` form,
with the minimum zenith distance `z0` (radians) in place of `h`. Peak
azimuth acceleration is therefore `PEAK_ACCEL_COEFF * (omega / z0)^2`, and
requiring it to stay within `a_max` gives

```
z0 >= omega * sqrt(PEAK_ACCEL_COEFF / a_max)
```

with `omega` in rad/s and `a_max` in rad/s^2.

The rate-limited keyhole already computed at `src/regimes.rs:508` is
`z0 >= omega / v_max`. These are two constraints on one physical keyhole, so
the reported keyhole is whichever binds:

```
z0 = max(omega / v_max, omega * sqrt(PEAK_ACCEL_COEFF / a_max))
```

## 7. Checks and grading

### 7.1 `Component::System`

Detection is not separable into telescope, camera and mount: it needs
collecting area, plate scale, quantum efficiency and sky background
together. `Component` therefore gains a `System` variant. This reuses all
existing machinery -- `RegimeCheck`, `component_status`, and `overall()`,
which already takes the worst status across checks, so detection flows into
the regime verdict without new code.

Cost: the per-regime summary gains a column, and the comparison grid at
`src/report.rs:143` goes from `T/C/M` to `T/C/M/S`.

### 7.2 New check: mount acceleration

Component `Mount`, title `"Acceleration"`.

* `Stare` mode: PASS. Nothing is being tracked.
* `max_accel_deg_s2` is `None`: `Info` when the required acceleration is
  below `ACCEL_MATTERS_DEG_S2`, `Warn` above it. This mirrors how
  `mount_rate` handles an unknown slew rate today. The threshold is set so
  that LEO alone trips it; see 8.2 for why it cannot be 0.01.
* Otherwise grade `headroom = a_max / required` against
  `ACCEL_PASS_HEADROOM` and `ACCEL_WARN_HEADROOM`.

Details report the required peak acceleration, the mount's rating, and the
headroom.

### 7.3 New check: slew and settle

Component `Mount`, title `"Slew and settle"`.

* `usable_window_s` is `None`: `Info`. There is no window pressure.
* `max_accel_deg_s2` is `None` but `max_slew_deg_s` is known: report
  `distance / max_rate` as a lower bound on slew time, status `Info`.
* Otherwise `total = slew_time_s(...) + settle_time_s`, and grade
  `total / usable_window_s` against `SLEW_PASS_WINDOW_FRACTION` and
  `SLEW_WARN_WINDOW_FRACTION`.

`DEFAULT_SLEW_DISTANCE_DEG` and `DEFAULT_SETTLE_TIME_S` are assumptions and
must be named as such in the details.

### 7.4 Change to the existing `mount_rate` check

The keyhole detail becomes the binding constraint of section 6.2 instead of
the rate-only figure, with a detail line naming which constraint binds.

**Backward compatibility is a requirement, not a hope.** When
`max_accel_deg_s2` is `None` the accel term is absent and the reported
keyhole is identical to today's. Section 10 requires a test asserting this.

### 7.5 New check: detection

Component `System`, title `"Detection"`.

```
photometry   = Photometry::resolve(telescope, camera, site)
target_mag   = config.target_mag_override
                 .unwrap_or(derived_target_mag(REFERENCE_TARGET_CROSS_SECTION_M2,
                                               REFERENCE_TARGET_ALBEDO,
                                               regime.range_km,
                                               DEFAULT_PHASE_FACTOR))
exposure_s   = config.exposure_override_s
                 .unwrap_or(trail_limited_exposure_s(site.seeing_arcsec,
                                                     residual_rate(regime)))
```

then signal, sky, footprint, SNR and limiting magnitude per section 5.

Grading:

* `SNR >= SNR_TRIVIAL`: PASS, and the verdict states that detection is not
  the limiting factor in this regime -- exposure should instead be chosen
  for saturation and timing. This reproduces, from computed numbers, the
  editorial claim already carried in each regime's `limiting_factor`.
* `SNR >= SNR_PASS`: PASS.
* `SNR >= DETECT_SNR_THRESHOLD`: WARN.
* Otherwise: FAIL.

Then the provenance cap. `Status` is ordered `Info < Pass < Warn < Fail`
(`src/checks.rs:125`), so a naive `status.max(Status::Warn)` would also
promote `Info` to `Warn` and misreport "could not compute" as a graded
warning. The rule is exactly:

```rust
if photometry.any_assumed() && status == Status::Pass {
    status = Status::Warn;
}
```

Details must report target magnitude (and whether it was derived or
entered), exposure (derived or entered), trail length, SNR, limiting
magnitude, the margin, and the names of any assumed inputs.

## 8. Worked examples

These values are the acceptance criteria. Following this project's existing
convention -- `src/checks.rs:707`, "Tests: the worked examples from
README.md" -- each becomes both a README example and a test.

Common configuration: PlaneWave DeltaRho 350 (effective area 0.0660 m^2,
focal length 1050 mm) with an IMX455 (3.76 um pixels, plate scale
0.7386 "/px); seeing 2.5"; sky 21.0 mag/arcsec^2; QE 0.80; throughput 0.85;
read noise 3.0 e-; target 10 m^2 at albedo 0.2 at full phase.

### 8.1 Derived target magnitude

| Regime | Range (km) | Derived mag |
|---|---|---|
| LEO | 500 | 2.25 |
| MEO | 20,200 | 10.28 |
| GEO | 37,000 | 11.59 |
| HEO | 39,836 | 11.75 |
| Cislunar | 384,400 | 16.67 |

GEO at 11.59 and cislunar at 16.67 fall inside the ranges real objects
occupy (GEO 11-15, cislunar 16-20), which is the sanity check on the
absolute scale.

### 8.2 Mount acceleration, LEO

```
v     = sqrt(MU_EARTH / (EARTH_RADIUS_KM + 500)) = 7.6126 km/s
omega = v / 500 = 0.01522526 rad/s = 0.87234 deg/s
alpha = PEAK_ACCEL_COEFF * omega^2 = 0.008627 deg/s^2
```

0.0086 deg/s^2 is negligible for any mount in `presets.yaml`. This is why
decision 2 does not stop at a tracking-acceleration check.

It also fixes `ACCEL_MATTERS_DEG_S2`. The equivalent figure for MEO is
1.37e-6 deg/s^2 -- nearly four orders of magnitude smaller, because the
requirement scales as `omega^2`. The threshold must fall between the two for
the `Info`/`Warn` split of 7.2 to mean anything, so it is 0.005: LEO's
0.008627 trips it, MEO's 1.37e-6 does not. A threshold of 0.01 would sit
above LEO's own requirement and the `Warn` branch would be unreachable.

### 8.3 Keyhole, LEO overhead pass

| Mount `a_max` | Accel keyhole | Rate keyhole (`v_max` = 50 deg/s) | Binding |
|---|---|---|---|
| 10 deg/s^2 | 88.32 deg elev | 89.00 deg elev | acceleration |
| 2 deg/s^2 | 86.24 deg elev | 89.00 deg elev | acceleration |
| 0.5 deg/s^2 | 82.48 deg elev | 89.00 deg elev | acceleration (WARN) |

The acceleration constraint binds in every case, and at 0.5 deg/s^2 it
crosses `KEYHOLE_WARN_ELEV_DEG`. This check discriminates between mounts;
the one in 8.2 does not.

### 8.4 Slew time

| Distance | `v_max` | `a_max` | Profile | Time |
|---|---|---|---|---|
| 90 deg | 50 deg/s | 10 deg/s^2 | triangular | 6.0 s |
| 90 deg | 50 deg/s | 50 deg/s^2 | trapezoidal | 2.8 s |
| 90 deg | 6 deg/s | 1 deg/s^2 | trapezoidal | 21.0 s |

### 8.5 Detection

| Regime | Mode | Exposure | Target mag | SNR | Limiting mag | Margin | Verdict |
|---|---|---|---|---|---|---|---|
| LEO | RateTrack | 30 s (capped) | 2.25 | ~38,900 | 20.06 | +17.81 | trivial |
| MEO | RateTrack | 30 s (capped) | 10.28 | 964 | 20.06 | +9.78 | trivial |
| GEO | Stare | 30 s (capped) | 11.59 | 526 | 20.06 | +8.47 | trivial |
| HEO | RateTrack | 30 s (capped) | 11.75 | 488 | 20.06 | +8.31 | trivial |
| Cislunar | Sidereal | 4.554 s (trail-limited) | 16.67 | 14.86 | 18.15 | +1.48 | graded |

Two things to note, both of which the implementer must preserve.

First, the limiting magnitude is **identical at 20.06 for all four
stationary-target regimes**, and this is not a coincidence to be optimised
away: they share an exposure (the `MAX_EXPOSURE_S` cap), a zero trail, and
therefore an identical footprint and noise budget. They differ only in
target magnitude. A test asserting all four agree is a cheap guard against
the noise terms accidentally picking up a range dependence.

Second, four of the five regimes exceed `SNR_TRIVIAL`, so they report the
"detection is not the limiting factor" verdict of 7.5 rather than a graded
margin. That is the correct outcome, not a mis-set threshold: a 14-inch
aperture at 30 seconds genuinely does not struggle with anything nearer than
the Moon. Cislunar is the only regime where detection is close -- and it is
the only regime whose `limiting_factor` already reads "brightness above
all". The model reproduces the tool's existing editorial judgment from
computed numbers, which is the strongest available check that it is wired up
correctly.

Cislunar intermediate values, for the test:

```
residual rate = 0.549017 "/s   (lunar: 1,296,000 / (27.321661 * 86,400))
exposure      = 2.5 / 0.549017 = 4.5536 s
trail         = 2.5"            (equal to seeing, by construction)
footprint     = 3.3853 * 6.7706 = 22.921 px
signal        = 389.31 e-
sky           = 90.55 e-
read term     = 206.29 e-
SNR           = 389.31 / sqrt(389.31 + 296.83) = 14.86
limiting mag  = 18.155
```

## 9. New constants

In `constants.rs`, placed by kind, following that file's existing sections.

Physical constants:

| Constant | Value | Note |
|---|---|---|
| `PEAK_ACCEL_COEFF` | 0.6495190528 | `3*sqrt(3)/8`, derived in 6.1 |
| `SUN_APPARENT_MAG` | -26.74 | Apparent V magnitude of the Sun |
| `PHOTONS_M2_S_MAG0` | 8.9e9 | V-band photon rate for `m = 0`; derivation in the doc comment |

Default assumptions:

| Constant | Value | Note |
|---|---|---|
| `REFERENCE_TARGET_CROSS_SECTION_M2` | 10.0 | Representative target |
| `REFERENCE_TARGET_ALBEDO` | 0.2 | Representative target |
| `DEFAULT_PHASE_FACTOR` | 1.0 | Full phase, phi = 0 |
| `DEFAULT_QE` | 0.80 | Generic back-illuminated CMOS |
| `DEFAULT_THROUGHPUT` | 0.85 | Generic coated two-mirror train |
| `DEFAULT_SKY_MAG_ARCSEC2` | 21.0 | Rural site |
| `DEFAULT_READ_NOISE_E` | 3.0 | Generic CMOS |
| `MAX_EXPOSURE_S` | 30.0 | Cap when the target is held still |
| `DEFAULT_SLEW_DISTANCE_DEG` | 90.0 | Assumed acquisition slew |
| `DEFAULT_SETTLE_TIME_S` | 2.0 | Assumed settle |

In `regimes_limits`:

| Constant | Value | Note |
|---|---|---|
| `DETECT_SNR_THRESHOLD` | 5.0 | Detection threshold |
| `SNR_PASS` | 10.0 | Comfortable detection |
| `SNR_TRIVIAL` | 100.0 | Above this, detection is not the limiting factor |
| `ACCEL_MATTERS_DEG_S2` | 0.005 | Below this, an unknown rating is `Info`. See 8.2. |
| `ACCEL_PASS_HEADROOM` | 3.0 | |
| `ACCEL_WARN_HEADROOM` | 1.0 | |
| `SLEW_PASS_WINDOW_FRACTION` | 0.10 | |
| `SLEW_WARN_WINDOW_FRACTION` | 0.25 | |

None of these values is generated by the code; all are either derived
(section 6.1, the photon zero point) or declared engineering assumptions,
consistent with the existing header of the judgment-thresholds section.

## 10. Test plan

Test-driven: each pure function gets its test before its implementation.
Tests live in in-file `#[cfg(test)] mod tests` blocks with the existing
`close(a, b, tol)` helper, matching `src/checks.rs:710` and
`src/regimes.rs:585`.

`photometry.rs`:

* `derived_mag_leo` / `_meo` / `_geo` / `_heo` / `_cislunar` -- table 8.1
* `geo_snr_deltarho350` -- 526, tolerance 1.0
* `cislunar_snr_deltarho350` -- 14.86, tolerance 0.05
* `cislunar_limiting_mag` -- 18.155, tolerance 0.01
* `limiting_mag_round_trip` -- `snr` at `limiting_mag` equals
  `DETECT_SNR_THRESHOLD`
* `stationary_regimes_share_limiting_mag` -- LEO, MEO, GEO and HEO all give
  20.06, per 8.5
* `trail_limited_exposure_cislunar` -- 4.5536 s
* `trail_limited_exposure_capped_when_stationary` -- `MAX_EXPOSURE_S`
* `footprint_grows_with_trail` -- monotonic in trail length
* `resolve_names_assumed_inputs` -- all four names present when nothing is
  entered, empty when everything is

`dynamics.rs`:

* `peak_accel_coeff_matches_closed_form` -- equals `3*sqrt(3)/8`
* `leo_peak_tracking_accel` -- 0.008627 deg/s^2
* `accel_keyhole_tighter_than_rate_keyhole` -- 88.32 against 89.00
* `accel_keyhole_warns_at_low_accel` -- 82.48 deg, below
  `KEYHOLE_WARN_ELEV_DEG`
* `slew_time_triangular` -- 6.0 s
* `slew_time_trapezoidal` -- 2.8 s and 21.0 s
* `slew_time_picks_triangular_when_vmax_unreached`

`regimes.rs`:

* `detection_caps_at_warn_when_inputs_assumed` -- a configuration that
  would grade PASS reports WARN when QE is `None`
* `detection_info_not_promoted_by_cap` -- an `Info` detection result stays
  `Info` (guards the `Ord` trap in 7.5)
* `keyhole_unchanged_when_accel_unknown` -- the 7.4 compatibility guarantee

## 11. Surface changes

`main.rs`:

* site prompt gains sky brightness (`ask_optional`, blank -> default)
* `build_config` gains the target-magnitude and exposure overrides, both
  `ask_optional_hint` with "blank for the derived value"
* `custom_telescope` gains throughput; `custom_camera` gains QE;
  `custom_mount` gains max acceleration and settle time
* the preset-mount path follows the pattern at `main.rs:145`: if
  `max_accel_deg_s2.is_none()`, ask for it
* `run_demo` leaves every new field `None`, so the demo exercises the WARN
  caps rather than hiding them
* `print_help` mentions the new checks

`presets.yaml`: new keys present and `null`, documented in the header
comment, with each `source:` note extended in the file's existing voice
("... not entered: confirm with the vendor"). **No invented vendor
values.** Every preset in this file carries a `source:`; fabricating a
quantum efficiency or read noise would break the guarantee that makes the
file trustworthy.

`report.rs`: site line gains sky brightness; the component loop at
`report.rs:238` and both status tables gain `System`; `print_formulas` gains
the new formulas; the regime detail header gains the usable window.

## 12. README changes

* Delete the two bullets this design retires, and the "No absolute limiting
  magnitude" bullet.
* Add the replacement limitations: full-phase assumption; a single
  representative 10 m^2 / 0.2 target; sky brightness as one number with no
  elevation dependence; assumed slew distance and settle time; servo
  bandwidth and following error still unmodeled; no detector saturation or
  full-well model, which is why very bright LEO targets report an
  implausibly high SNR and are reported as "trivial" instead.
* Add input-table rows for every field in section 4.
* Add constants-table rows for every constant in section 9.
* Add formula sections carrying the derivations in 5.1 and 6.1 and the
  worked examples in section 8.
* Update the preset tables to show the new columns as not entered.

## 13. Verification gate

* `cargo test` green, including every test in section 10.
* `cargo build` with no new warnings.
* `./target/debug/scope-eval --demo` runs end to end and shows the new
  checks with their WARN caps.
* `presets.yaml` parses unchanged (covered by `--demo`).
* The three README bullets are gone and their replacements are present.
