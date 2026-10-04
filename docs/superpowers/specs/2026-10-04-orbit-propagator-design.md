# Satellite propagator library (`orbit-prop`) and pass prediction

Date: 2026-10-04
Status: approved design, awaiting written-spec review

## 1. Purpose

`scope-eval` judges each orbital regime from one representative case: a
500 km LEO pass straight overhead, a GPS-like MEO orbit, GEO and so on.
These use closed-form formulas in `src/calculations/orbit.rs`. Nothing in
the project propagates an orbit over time, and nothing knows where the site
is on Earth.

The long-term goal is a simulator that improves the equipment assessment by
replacing those representative numbers with the real geometry of real
passes over the user's site. This design delivers the foundation for that:

* a separate, reusable library crate, `orbit-prop`, that propagates
  satellites and computes what a ground site sees, and
* a small integration in `scope-eval`, a pass-prediction menu item with a
  per-pass "Mount can follow?" judgment, which proves that the library fits
  the project end to end.

The simulator itself is a later, separate project.

## 2. Decisions made during brainstorming

| Question | Decision |
|---|---|
| Orbit sources | Both: SGP4 for catalog objects (TLEs), Keplerian + J2 for what-if orbits, behind one common interface |
| Library boundary | Propagation plus observer geometry (az/el, rates, Sun/Moon, shadow, phase angle, pass finding). The simulation loop is not included. |
| SGP4 implementation | Use the `sgp4` crate (pure Rust, verified against Vallado's test cases). Everything else is written in-house. |
| Accuracy | Assessment-grade, about 0.01 deg or better in az/el. The need for an astrometric-grade upgrade is recorded in code and in the README (section 9). |
| `scope-eval` changes | Library plus a small integration: pass prediction with a mount column. No changes to `regimes.rs`. |
| CLI flag for passes | Not now. Interactive menu only. |

## 3. Scope

In scope:

* Converting the repo to a Cargo workspace with the new crate at
  `crates/orbit-prop`.
* The library: time, frames, TLE + SGP4, Keplerian + J2, ground site,
  observation geometry, Sun and Moon, illumination, pass finding.
* `scope-eval`: optional site location, the "Predict passes for a
  satellite" menu item, the pass table, the mount column, and README
  updates.

Out of scope (left for the simulator or later work):

* Per-pass brightness and SNR, field-of-view and acquisition checks.
* Changes to `regimes.rs` or to any existing check result.
* A non-interactive `--passes` CLI flag.
* Astrometric-grade frames and ephemerides (section 9).
* Downloading TLEs. The user pastes them in.

## 4. Architecture

```
scope-eval/                  repo root, becomes a Cargo workspace
  Cargo.toml                 [workspace] members = [".", "crates/orbit-prop"]
                             [dependencies] orbit-prop = { path = "crates/orbit-prop" }
  src/                       scope-eval
  crates/orbit-prop/
    Cargo.toml               dependency: sgp4 (only)
    README.md                models, frames, accuracy, future-work section
    src/
      lib.rs                 public API re-exports
      constants.rs           mu, J2, WGS-84 Earth radius and flattening, Earth rotation rate
      error.rs               OrbitPropError
      time.rs                Epoch (UTC, split Julian date), GMST, ISO-8601 parsing, now()
      state.rs               StateVector { epoch, r_km: [f64;3], v_km_s: [f64;3] } (TEME)
      propagator.rs          trait Propagator
      keplerian.rs           KeplerJ2 propagator built from classical elements
      tle.rs                 Tle: parse + checksum validation
      sgp4.rs                Sgp4Propagator wrapping the sgp4 crate
      frames.rs              TEME <-> ECEF via GMST; geodetic <-> ECEF; ECEF -> SEZ/topocentric
      site.rs                GroundSite { lat_deg, lon_deg, alt_m }
      observe.rs             observe(state, site) -> Observation
      sun_moon.rs            low-precision Sun and Moon positions
      illumination.rs        Earth shadow, phase angle, Sun elevation at site
      passes.rs              find_passes -> Vec<Pass>
```

Design rules, matching the existing `scope-eval` conventions:

* Calculations are pure functions. Each module has worked-example tests
  against published references.
* Everything downstream of propagation depends only on the `Propagator`
  trait, so callers never need to know whether an orbit came from a TLE or
  from elements.
* The library never panics on bad input. All failures are typed errors
  (section 7).
* Units are part of the field names: `_km`, `_km_s`, `_deg`, `_deg_s`,
  `_deg_s2`, `_m`, `_s`. Angles are degrees in public types and radians
  internally.
* `scope-eval`'s `MU_EARTH` and `EARTH_RADIUS_KM` in `src/constants.rs`
  become re-exports of `orbit_prop::constants`, so the two crates cannot
  drift apart. The values are identical (398600.4418 and 6378.137), so no
  existing test changes.

## 5. Library components

### 5.1 `time.rs`

`Epoch` is a UTC instant stored as a split Julian date (`jd_whole`,
`jd_fraction`), to keep sub-millisecond precision.

* `Epoch::from_utc(y, mo, d, h, mi, s: f64)`, `Epoch::parse_iso8601(&str)`
  (accepts `YYYY-MM-DDTHH:MM:SS[.sss][Z]`), and `Epoch::now()` from
  `std::time::SystemTime`.
* `add_seconds`, `seconds_since`, and `Display` as ISO-8601 UTC.
* `gmst_rad()`: IAU-1982 GMST polynomial, treating UTC as UT1
  (`TODO(astrometric)`).

### 5.2 `propagator.rs`, `state.rs`

```rust
pub trait Propagator {
    fn propagate(&self, t: Epoch) -> Result<StateVector, OrbitPropError>;
    /// Nominal orbital period, seconds. Used to choose pass-search step sizes.
    fn period_s(&self) -> f64;
    /// Human-readable label (TLE name / "what-if orbit").
    fn label(&self) -> &str;
}
```

The output frame is TEME for both implementations. The `KeplerJ2` output
is treated as TEME as well, which is consistent at assessment grade.

### 5.3 `keplerian.rs`: `KeplerJ2`

The input is `KeplerElements { epoch, a_km, e, i_deg, raan_deg, argp_deg,
mean_anomaly_deg }`, with the convenience constructor
`KeplerElements::from_altitudes(perigee_alt_km, apogee_alt_km, ...)`.
Propagation applies the first-order secular J2 rates to RAAN, argument of
perigee and mean anomaly, solves Kepler's equation by Newton iteration
(tolerance 1e-12 rad, iteration cap of 50, error if it doesn't converge),
then converts to position and velocity. Validation requires 0 <= e < 1,
a > 0 and perigee radius > Earth's equatorial radius.

### 5.4 `tle.rs`, `sgp4.rs`

`Tle::parse(text)` accepts 2 or 3 lines (an optional name line), trims
whitespace, and validates line numbers, line length (69) and both
checksums. Error messages give the line and column. The parse uses the
`sgp4` crate's `Elements::from_tle`.

`Sgp4Propagator::new(&Tle)` builds the `sgp4` crate's `Constants` (WGS-72,
as SGP4 requires). `propagate(t)` converts `t` to minutes since the TLE
epoch and maps the crate's errors to `OrbitPropError::Sgp4`.

### 5.5 `frames.rs`, `site.rs`

* TEME -> ECEF: a rotation about z by GMST, with the velocity corrected
  for Earth rotation (omega x r). Polar motion is ignored
  (`TODO(astrometric)`).
* `GroundSite::new(lat_deg, lon_deg, alt_m)` validates the inputs.
  Geodetic -> ECEF uses WGS-84. ECEF -> geodetic (for tests) is iterative.
* ECEF -> topocentric SEZ -> az/el. Azimuth runs from north through east,
  0 to 360 deg.
* The module doc comment states the accuracy grade and points to the
  README future-work section.

### 5.6 `observe.rs`

`observe(&StateVector, &GroundSite) -> Observation`:

```rust
pub struct Observation {
    pub epoch: Epoch,
    pub az_deg: f64, pub el_deg: f64,
    pub range_km: f64, pub range_rate_km_s: f64,
    pub az_rate_deg_s: f64, pub el_rate_deg_s: f64,   // alt-az axis rates
    pub ra_deg: f64, pub dec_deg: f64,                // topocentric, true-of-date
    pub ha_rate_deg_s: f64, pub dec_rate_deg_s: f64,  // equatorial axis rates
    pub rate_vs_ground_deg_s: f64,                    // total angular rate an Earth-fixed mount follows
    pub rate_vs_stars_deg_s: f64,                     // total angular rate against the stars
}
```

Rates are computed analytically from the topocentric relative position and
velocity, not by differencing.

### 5.7 `sun_moon.rs`, `illumination.rs`

* Sun: the low-precision formula from the Astronomical Almanac (about
  0.01 deg). Moon: the low-precision series (about 0.3 deg), which is
  enough for Moon elevation and separation. Both return a geocentric
  position vector in km in the true-of-date frame. `TODO(astrometric)`.
* `sunlit(sat_r, sun_r) -> Lighting { Sunlit, Penumbra, Umbra }` uses a
  conical shadow model.
* `phase_angle_deg(sat_r, site_r, sun_r)` is the Sun–satellite–observer
  angle.
* `sun_elevation_deg(site, t)`. The site counts as dark below -12 deg
  (nautical twilight). The threshold is a named constant.

### 5.8 `passes.rs`

```rust
pub struct PassSearch { pub start: Epoch, pub end: Epoch, pub min_el_deg: f64 }

pub struct Pass {
    pub rise: Epoch, pub culmination: Epoch, pub set: Epoch,
    pub clipped_start: bool, pub clipped_end: bool,   // already up at start / still up at end
    pub max_el_deg: f64,
    pub peak_az_rate_deg_s: f64, pub peak_el_rate_deg_s: f64,
    pub peak_az_accel_deg_s2: f64, pub peak_el_accel_deg_s2: f64,
    pub peak_ha_rate_deg_s: f64, pub peak_dec_rate_deg_s: f64,
    pub peak_rate_vs_ground_deg_s: f64,
    pub lighting: PassLighting,          // Sunlit / Partial / Eclipsed for the whole pass
    pub site_dark: PassDarkness,         // Dark / Partial / Daylight (Sun elevation vs threshold)
}

pub fn find_passes(prop: &dyn Propagator, site: &GroundSite, search: &PassSearch)
    -> PassResult;

pub struct PassResult { pub passes: Vec<Pass>, pub error: Option<OrbitPropError> }
```

Algorithm:

1. Coarse scan of elevation minus `min_el` with step
   `clamp(period / 60, 10 s, 300 s)`.
2. Each sign change is refined by bisection to 0.1 s.
3. Culmination is found by golden-section search on elevation between
   rise and set.
4. Peak axis rates and accelerations come from dense sampling over the
   pass: a 1 s step, plus 0.1 s within ±30 s of culmination. Accelerations
   are central differences of the analytic rates.
5. Lighting and darkness are sampled on the same dense grid.

`PassResult` keeps the passes found before any propagation error, together
with that error, so a TLE that decays partway through the window still
reports its earlier passes. Search windows are limited to 30 days
(`InvalidTime` otherwise) to bound the run time.

## 6. `scope-eval` integration

### 6.1 Site location

`model/site.rs`: `Site` gains `location: Option<orbit_prop::GroundSite>`,
defaulting to `None`. No existing behaviour depends on it. The **Change
site conditions** menu item also asks for latitude, longitude and altitude.
A blank answer keeps the current value. The menu label shows the location
when it is set.

### 6.2 Menu item "Predict passes for a satellite"

This item is inserted before "Show formula summary".

1. If no site location is set, it asks for one and stores it.
2. **Orbit source** menu:
   * *Paste a TLE*: reads lines until two valid TLE lines are collected,
     with an optional name line. On a parse error it shows the library's
     message and asks again.
   * *Define a what-if orbit*: perigee altitude (km), apogee altitude (km,
     default = perigee), inclination (deg), RAAN (default 0), argument of
     perigee (default 0), mean anomaly (default 0) and epoch (default =
     search start).
3. **Window**: start (ISO-8601 UTC, blank = now), duration in hours
   (default 24, maximum 720), minimum elevation (default 10 deg).
4. It runs `find_passes` and prints the table described in 6.3. If
   `PassResult.error` is set, it prints the passes and then the error.

### 6.3 Pass table

New module `src/passes_report.rs`. It handles printing only; all the math
lives in the library. Output is plain ASCII, like the rest of the tool.

Columns: rise (UTC), set (UTC), duration, peak elevation, peak az rate,
peak el rate (deg/s), sunlit (yes / partial / no), site dark (yes /
twilight / no), plus one **Mount can follow?** column for each evaluated
configuration that has a mount. Clipped passes are marked `<` (already up
at the start) or `>` (still up at the end). "No passes in this window" is
printed when the list is empty. A footnote explains the mount-column
statuses.

### 6.4 Mount judgment

Function `judge_mount_for_pass(&Mount, &Pass) -> (Status, String)` in
`src/passes_report.rs`, using the existing thresholds in
`constants::regimes_limits` and the existing `plausible` range guards:

* Axis rates and accelerations by mount type: alt-az uses az/el, and
  equatorial uses HA/Dec. If the type is unknown, it uses the worse of the
  two.
* Rate: headroom = `max_slew_deg_s` / peak axis rate. PASS at
  `>= RATE_PASS_HEADROOM`, WARN at `>= RATE_WARN_HEADROOM`, otherwise FAIL.
  If the slew rate is unknown: WARN if the peak rate is above
  `RATE_MATTERS_DEG_S`, otherwise INFO.
* Acceleration: the same pattern with `max_accel_deg_s2`,
  `ACCEL_PASS_HEADROOM` / `ACCEL_WARN_HEADROOM` and `ACCEL_MATTERS_DEG_S2`.
* The column shows the worse of the two. For an alt-az mount, a
  high-culmination pass's actual azimuth rate and acceleration are part of
  the propagated data, so the keyhole falls out of this check directly. The
  generic keyhole formula is not needed.
* The returned string names the binding limit, for example "az rate 4.1x".

### 6.5 Docs

* `scope-eval` README: a new "Pass prediction" section, the
  code-structure tree updated for the workspace and `passes_report.rs`,
  and an assumptions entry pointing to the library's accuracy grade.
* `crates/orbit-prop/README.md`: models, frames, units, accuracy, error
  handling, and "Known limitations and future work" (section 9).
* The README's "no external dependencies" claim is already out of date
  (serde, serde_yaml). It is corrected to list the actual dependencies,
  `sgp4` included.

## 7. Error handling

```rust
pub enum OrbitPropError {
    TleFormat { line: u8, column: Option<usize>, message: String },
    Sgp4(String),
    InvalidElements(String),
    InvalidSite(String),
    InvalidTime(String),
    NoConvergence(String),
}
```

It implements `Display` and `std::error::Error`. There are no panics on
user-supplied input. In `scope-eval`, invalid input asks again using the
existing `input.rs` helpers. If no configuration is evaluated, the mount
columns are left out. A configuration without a mount gets no column.

## 8. Testing

All tests are offline, and fixtures are hard-coded.

| Module | Reference case | Tolerance |
|---|---|---|
| `time` | Vallado Ex. 3-5: GMST at 1992-08-20 12:14 UT1 = 152.578788 deg | 1e-5 deg |
| `time` | ISO-8601 parse/format round trip; `add_seconds` across a day boundary | exact to 1 us |
| `frames` | Vallado Ex. 3-3 geodetic -> ECEF; geodetic -> ECEF -> geodetic round trip | 1 m |
| `tle` | Valid TLE parses; bad checksum, short line and wrong line number are rejected with the correct line | — |
| `sgp4` | Vallado verification TLE 00005 at t = 0 and t = 360 min against the published TEME positions | 1 m |
| `keplerian` | Circular-orbit period = 2 pi sqrt(a^3/mu); sun-synchronous 800 km, 98.6 deg drifts RAAN about +0.9856 deg/day; e >= 1 and sub-surface perigee are rejected | 1e-3 relative |
| `sun_moon` | Meeus Ex. 25.a (Sun, 1992-10-13) and 47.a (Moon, 1992-04-12) | 0.01 deg Sun, 0.3 deg Moon |
| `illumination` | Satellite directly anti-Sun behind Earth is in umbra; beside Earth it is sunlit; phase angle is 0 and 180 deg at the extremes | exact classification |
| `observe` | Circular 500 km orbit passing through the zenith: rate vs stars at zenith matches `scope-eval`'s 3140 arcsec/s, with Earth rotation removed in the test setup | 0.5 % |
| `passes` | Equatorial circular orbit over an equatorial site: rise and set times match the analytic geometry; a GEO object gives one pass clipped at both ends; an object that never rises gives an empty list; a decaying TLE returns earlier passes plus an error | 1 s |
| `scope-eval` | `judge_mount_for_pass`: PASS/WARN/FAIL/INFO branches for rate and acceleration, alt-az vs equatorial axis selection, implausible ratings ignored | exact |

`cargo test` at the workspace root runs both crates. `--demo` output is
unchanged.

## 9. Known limitations and future work: astrometric grade

This must be recorded in `crates/orbit-prop/README.md`, in the module doc
comment of `frames.rs` and `sun_moon.rs`, and as `TODO(astrometric)` at
each simplification. The library is **assessment-grade, about 0.01 deg**.
That is good enough for pass timing, rates, visibility and lighting, and
smaller than typical TLE error. Astrometric-grade work, such as reducing
your own observations against the catalog, building pointing models, or
orbit determination, needs:

* IAU-2006/2000A precession and nutation (TEME -> GCRS) instead of the
  GMST-only rotation,
* UT1 - UTC and polar motion from IERS Earth-orientation data,
* annual and diurnal aberration and light-time correction,
* atmospheric refraction in elevation (currently not modelled; about
  0.5 deg at the horizon, which also affects rise and set times),
* a higher-precision Sun and Moon ephemeris (for example JPL DE440).

## 10. Phasing

Each phase leaves `cargo test` green and `--demo` unchanged.

1. Workspace conversion, crate skeleton, `constants`, `error`, `time`,
   `frames`, `site`, with the `scope-eval` constants re-exported.
2. `Propagator`, `KeplerJ2`, `tle`, `sgp4`.
3. `observe`, `sun_moon`, `illumination`.
4. `passes`.
5. `scope-eval` integration: site location, menu item, pass table, mount
   judgment, README updates.
