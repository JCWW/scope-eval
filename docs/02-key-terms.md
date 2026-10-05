# 2. Key terms

**What you'll learn:** the vocabulary used on every later page. The terms are grouped so each one builds on the last: angles first, then the telescope, then the sensor, then brightness, then orbits and mounts.

## Angles

**Arcsecond (").** A unit of angle. A full circle is 360 degrees, each degree is 60 arcminutes ('), and each arcminute is 60 arcseconds. One arcsecond is roughly the width of a coin seen from 4 km away.

**Radian.** The angle at which an arc's length equals its radius. One radian is 206,265 arcseconds (180 / pi x 3600). This number appears in many formulas, because it converts the natural "size / distance" angle into arcseconds.

**The small-angle rule.** For small angles, angle (radians) = size / distance. Most geometry in the tool is this one rule applied to different things: a pixel at the end of a focal length, a sensor, a prediction error at a satellite's range.

## The atmosphere

**Seeing and FWHM.** Turbulence in the atmosphere blurs every star into a small fuzzy disk. "Seeing" is the size of that disk, measured as its **full width at half maximum (FWHM)**: the width of the blur where brightness has fallen to half its peak. Typical seeing at a decent mid-elevation site is 2 to 3 arcseconds. Seeing is set by the site and the weather, not by the telescope, and it is the starting point for most checks.

## The telescope

**Aperture (D).** The diameter of the telescope's main light-collecting opening.

**Focal length (FL).** The effective distance over which the telescope brings light to a focus. Longer focal length means a more magnified image on the sensor.

**Focal ratio (N, written f/N).** Focal length divided by aperture. A low number (f/2 or f/3) is called "fast" and gives a wide, bright, small-scale image. A high number (f/7 or f/8) is "slow" and gives a narrow, magnified image.

**Central obstruction.** Many telescopes have a secondary mirror, lens group or camera sitting in the middle of the incoming light, blocking part of the aperture. Spec sheets quote it either as a fraction of the diameter or as a fraction of the area, and the difference matters ([page 6](06-light-field-focus-fit-checks.md#check-5-collecting-area-and-depth)).

**Image circle.** The diameter of the region at the focal plane where the image is sharp and flat. The camera sensor must fit inside it.

**Critical focus zone (CFZ).** How far the sensor can sit from perfect focus before the image visibly degrades.

**Etendue.** Collecting area multiplied by field of view. It measures how fast a telescope can survey the sky to a given depth.

## The sensor

**Plate scale.** How much sky one pixel sees, in arcseconds per pixel.

**Binning.** Combining a square block of neighboring pixels (2x2, 3x3 and so on) into one larger "super-pixel."

**Read noise.** Electronic noise added every time a pixel is read out, quoted in electrons (e-) RMS.

**Rolling shutter.** Most CMOS sensors expose and read out the image one row at a time rather than all at once. The bottom row is captured slightly later than the top row. A **global shutter** sensor captures every row at the same instant.

## Brightness

**Magnitude.** The astronomical brightness scale. It is logarithmic and runs backwards: larger numbers are fainter. A difference of 5 magnitudes is a factor of 100 in brightness, so 1 magnitude is a factor of 100^(1/5), about 2.512. A brightness ratio r corresponds to 2.5 x log10(r) magnitudes.

## Orbits

**Orbital regimes.** Satellites are grouped by altitude and orbit shape:

* **LEO (low Earth orbit)**, roughly 200 to 2,000 km. Fast-moving across the sky, usually bright, visible for only minutes per pass.
* **MEO (medium Earth orbit)**, roughly 2,000 to 35,000 km. Navigation constellations such as GPS (about 20,200 km) live here.
* **GEO (geosynchronous orbit)**, about 35,786 km. Orbits once per sidereal day, so it appears nearly fixed in the sky.
* **HEO (highly elliptical orbit)**, such as the Molniya orbit. Very elongated: fast near the low point, slow and distant near the high point (apogee).
* **Cislunar space**, out to and around the Moon's distance (about 384,000 km). Very distant and therefore very faint.

**Sidereal day.** The time Earth takes to turn once relative to the stars: 86,164 seconds, about 4 minutes shorter than a 24-hour solar day.

**TLE and ephemeris.** A two-line element set (TLE) or ephemeris is a prediction of where a satellite will be. Predictions have errors, mostly along the direction of travel, so the telescope must have a wide enough field to catch the target anyway.

## Mounts and tracking

**Tracking modes.** *Stare*: the mount is stopped. *Sidereal tracking*: the mount follows the stars. *Rate tracking*: the mount follows the satellite's predicted path, so the satellite stays a point and the stars streak. Rate tracking requires the mount's software to support **non-sidereal tracking**.

**GEO and stare mode.** A geosynchronous satellite turns with Earth, so if the telescope stops tracking, the GEO object appears as a point while the background stars drift past and leave streaks.

**Keyhole.** Every two-axis mount has a direction where one axis would have to spin infinitely fast to follow an object. For an alt-azimuth mount it is straight overhead (the zenith). For an equatorial mount it is near the celestial pole.

## Check it yourself

1. **The radian constant.** Compute 180 / pi x 3600. You should get 206,264.8, which the tool rounds to 206,265 in prose (`ARCSEC_PER_RADIAN` in `src/constants.rs` holds 206,264.806).
2. **The coin.** Using the small-angle rule, what size subtends 1" at 4 km? 4,000 m / 206,265 = 0.0194 m, about 19 mm: the width of a small coin.
3. **The magnitude step.** Compute 100^(1/5). You should get 2.512. Then check 2.5 x log10(100) = 5. The test `five_magnitudes_is_a_factor_of_one_hundred` checks the same relationship in code: `cargo test five_magnitudes`.
4. **Focal ratio.** The DeltaRho 350 has D = 350 mm and FL = 1050 mm. Its focal ratio is 1050 / 350 = 3, so it is an f/3 ("fast") telescope. The CDK14 (356 mm, 2563 mm) is 2563 / 356 = f/7.2 ("slow").
5. **Sidereal versus solar day.** 86,400 - 86,164 = 236 seconds, about 3 minutes 56 seconds. That's why a star rises about 4 minutes earlier each night.

Next: [Inputs](03-inputs.md).
