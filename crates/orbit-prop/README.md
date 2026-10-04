# orbit-prop

Satellite propagation and ground-site observation geometry for
`scope-eval` and the simulator that will build on it.

* **Two orbit models behind one interface.** `Sgp4Propagator` propagates
  catalog TLEs with SGP4, and `KeplerJ2` propagates what-if orbits
  (Keplerian with secular J2 drift). Both implement the `Propagator`
  trait, and everything downstream depends only on that trait.
* **Observer geometry.** `observe` gives azimuth, elevation, range, range
  rate, alt-az and equatorial axis rates, and the angular rate against the
  ground and against the stars, as seen from a `GroundSite`.
* **Lighting.** Low-precision Sun and Moon positions, Earth shadow
  (umbra/penumbra), phase angle, and Sun elevation at the site.
* **Passes.** `find_passes` returns rise, culmination and set times, peak
  axis rates and accelerations, and lighting and darkness for each pass.

The only dependency is the pure-Rust [`sgp4`](https://crates.io/crates/sgp4)
crate.

## Example

```rust
use orbit_prop::{find_passes, Epoch, GroundSite, PassSearch, Sgp4Propagator, Tle};

let tle = Tle::parse("ISS (ZARYA)
1 25544U 98067A   08264.51782528 -.00002182  00000-0 -11606-4 0  2927
2 25544  51.6416 247.4627 0006703 130.5360 325.0288 15.72125391563537")?;
let prop = Sgp4Propagator::new(&tle)?;
let site = GroundSite::new(40.0, -75.0, 100.0)?;
let search = PassSearch { start: tle.epoch, end: tle.epoch.add_seconds(86_400.0), min_el_deg: 10.0 };
let result = find_passes(&prop, &site, &search);
for p in &result.passes {
    println!("{} max el {:.1} deg, peak az rate {:.2} deg/s", p.rise, p.max_el_deg, p.peak_az_rate_deg_s);
}
# Ok::<(), orbit_prop::OrbitPropError>(())
```

## Models

| Module | Model | Reference |
|---|---|---|
| `time` | Split Julian date, UTC; GMST by the IAU-1982 expression | Vallado, *Fundamentals of Astrodynamics*, eq. 3-47 |
| `frames` | TEME to ECEF by GMST rotation; WGS-84 geodetic to ECEF; topocentric SEZ | Vallado sec. 3.4 and 3.7 |
| `sgp4_propagator` | SGP4/SDP4 via the `sgp4` crate, AFSPC compatibility mode | Vallado et al. 2006, "Revisiting Spacetrack Report #3" |
| `keplerian` | Two-body orbit with secular J2 rates of node, perigee and mean anomaly | Vallado eq. 9-41 |
| `sun_moon` | Low-precision Sun (about 0.01 deg) and Moon (about 0.3 deg) | Astronomical Almanac, sections C and D |
| `illumination` | Conical umbra and penumbra | Vallado algorithm 34 |
| `passes` | Coarse scan, bisection to 0.1 s, dense sampling for peaks | this crate |

SGP4 runs in AFSPC compatibility mode because catalog TLEs are fitted with
that implementation. The `sgp4` crate's default "improved" mode differs
from it by up to tens of metres.

`KeplerJ2` drifts from a real satellite by kilometres per day: it has no
drag, no short-period J2 terms and no third bodies. Use it for "what would
this kind of orbit look like from my site", and a TLE for real objects.

## Units and conventions

Units are in the field names: `_km`, `_km_s`, `_deg`, `_deg_s`, `_deg_s2`,
`_m`, `_s`. Azimuth runs from north through east. Longitude is positive
east. Times are UTC, and leap seconds are ignored.

## Errors

No function panics on user input. Everything fallible returns
`OrbitPropError`: `TleFormat` (with line and column), `Sgp4` (for example
a decayed orbit), `InvalidElements`, `InvalidSite`, `InvalidTime`, and
`NoConvergence`. `find_passes` returns a `PassResult` that keeps the passes
found before an error, plus the error.

## Accuracy

This crate is **assessment grade, about 0.01 deg** in azimuth and
elevation. That is good enough for pass timing, rates, visibility and
lighting, and smaller than typical TLE error (about 1 km, several
arcminutes for a LEO object seen from the ground).

## Known limitations and future work

**Upgrade to astrometric grade before using this for astrometry, pointing
models or orbit determination.** Each simplification is marked
`TODO(astrometric)` in the source. The upgrade needs:

* IAU-2006/2000A precession and nutation (TEME to GCRS) instead of the
  GMST-only rotation,
* UT1 - UTC and polar motion from IERS Earth-orientation data,
* annual and diurnal aberration and light-time correction,
* atmospheric refraction in elevation (not modelled; about 0.5 deg at the
  horizon, which also shifts rise and set times),
* a higher-precision Sun and Moon ephemeris, for example JPL DE440.

Other limits:

* The pass search steps at `period / 60`, clamped to 10-300 s, so a pass
  shorter than that step can be missed.
* Lighting ignores refraction into the shadow, and the Moon's light.
