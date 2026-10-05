# 7. Motion and timing

**What you'll learn:** how motion on the sky turns timing errors and rolling-shutter readout into position errors. The ideas are introduced with the simplest case, a GEO satellite in stare mode, where only one rate matters. [Orbital regimes](08-orbital-regimes.md) then applies the same ideas to every regime and grades them.

**Before you start:** [Key terms](02-key-terms.md) (sidereal day, stare mode, rolling shutter) and [check 2](05-image-quality-checks.md#check-2-sampling-plate-scale-vs-seeing) (plate scale and binning).

The tool prints these numbers after the eight checks as a **timing reference**. They are not graded.

## Step 1: the sidereal rate

Earth turns 360 degrees (1,296,000 arcseconds) once per sidereal day (86,164.09 seconds):

```
sidereal rate = 1,296,000" / 86,164.09 s = 15.04 "/s
```

A GEO satellite turns with Earth, so in stare mode the stars slide past it at this rate. The GEO belt lies close to the celestial equator, so the cos(declination) factor that would slow the stars' apparent motion is close to 1 and is ignored.

## Step 2: star streaks

Divide the rate by the plate scale to get motion in pixels:

```
star streak per second (px) = 15.04 / binned plate scale
```

For the DeltaRho 350 binned 2x2 (1.48 "/px): 15.04 / 1.48 = **10 pixels per second** of exposure.

## Step 3: timing error becomes position error

A satellite's reported position is only as good as the timestamp on the image. If the target moves at a rate relative to the stars, an error in time is an error in position:

```
position error (") = rate (") x timing error (s)
```

For GEO, a 10 ms timing error gives 15.04 x 0.010 = 0.15" of along-track error. A computer clock plus USB latency can easily be off by tens of milliseconds, which is why hardware GPS timestamping matters. For comparison, a low Earth orbit object moving about 1 degree per second (3,600 "/s) picks up 3.6" of error per millisecond.

## Step 4: rolling-shutter skew

A rolling shutter reads rows one after another, so the bottom row is captured later than the top row by the full readout time:

```
readout skew (s) = number of rows x line time
star skew (")    = 15.04 x readout skew
per-row time     = t(first row) + row index x line time
```

For the IMX455 in a Moravian C3-61000: 6,388 rows x 39.028 us = 0.249 s. In stare mode that's a **3.75"** systematic skew between the top and bottom of the frame. If the reduction software uses a single timestamp for the whole frame, the plate solution absorbs this skew as a false distortion. The fix is to assign each row its own time using the per-row formula. Global-shutter sensors don't have this effect.

## The general pattern

Every result on this page has the same shape: **a rate multiplied by a time gives an angle**, and dividing by the plate scale turns the angle into pixels. The only thing that changes between regimes is the rate. GEO's 15"/s is gentle. LEO's roughly 3,140"/s makes every millisecond count, as the next page shows.

## Check it yourself

**By hand.**

1. **Sidereal rate.** 1,296,000 / 86,164.09 = **15.041 "/s**.
2. **RASA streaks.** The RASA 11 + IMX455 is recommended at 1x1 (1.251 "/px). Star streak = 15.04 / 1.251 = **12.0 px per second**.
3. **PC clock.** A 20 ms timing error at GEO gives 15.04 x 0.020 = **0.30"**. That's 0.2 of a binned DeltaRho pixel (1.48 "/px), so already a noticeable fraction.
4. **A smaller sensor.** The IMX571 has 4,176 rows at 34.667 us. Readout = 4,176 x 34.667e-6 = 0.145 s, so stare-mode skew = 15.04 x 0.145 = **2.18"**. Fewer rows means less skew.

**Against the tests.**

| Concept | Command |
|---|---|
| Rolling-shutter skew for the IMX455 | `cargo test rolling_shutter_skew_matches_sensor_spec` |
| Timing budget as a fraction of a moving pixel | `cargo test timing_budget_is_fraction_of_a_moving_pixel` |

**In the tool.** Evaluate the DeltaRho 350 + IMX455 and read the timing reference printed after the eight checks. It should show a star drift of 15.04 "/s, a streak of about 10 px per second at 2x2, and a rolling-shutter skew of about 3.75".

Next: [Orbital regimes](08-orbital-regimes.md).
