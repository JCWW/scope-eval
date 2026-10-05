#!/usr/bin/env python3
"""Recompute every worked example in docs/learning with nothing but the
Python standard library.

This is a second, independent implementation of the formulas. It shares no
code with the Rust tool, so if both agree, you can trust the number more
than if either stood alone. Run it with:

    python3 docs/learning/check_examples.py

Each line prints the value computed here next to the value quoted in the
docs. Read the code beside the matching lesson: every block is labelled
with the lesson file it checks.
"""

from math import pi, sqrt, log, log10, cos, degrees, radians, erf, hypot

ARCSEC_PER_RAD = 206_265.0
DEG_PER_RAD = 57.2958
SIDEREAL_DAY_S = 86_164.09
MU_EARTH = 398_600.0  # km^3/s^2
R_EARTH = 6_378.0     # km

failures = 0


def show(label, computed, quoted, tol):
    """Print one comparison. `tol` is the allowed absolute difference."""
    global failures
    ok = abs(computed - quoted) <= tol
    if not ok:
        failures += 1
    mark = "ok " if ok else "BAD"
    print(f"  [{mark}] {label:<52} computed {computed:>14.6g}   docs {quoted:>12g}")


def section(name):
    print(f"\n{name}")


# ---------------------------------------------------------------- lesson 01
section("01-angles-and-magnitudes.md")
show("arcseconds in a radian", 180 / pi * 3600, 206_265, 1)
show("coin 2.5 cm wide at 4 km, arcsec", 0.025 / 4000 * ARCSEC_PER_RAD, 1.29, 0.01)
show("5 magnitudes as a flux ratio", 10 ** (0.4 * 5), 100, 1e-9)
show("1 magnitude as a flux ratio", 10 ** 0.4, 2.512, 0.001)
show("2x the light, in magnitudes", 2.5 * log10(2), 0.75, 0.01)

# Shared example configuration: DeltaRho 350 + IMX455 at 2.5" seeing.
D_MM, FL_MM, OBS = 350.0, 1050.0, 0.56
PIX_UM, W_PX, H_PX = 3.76, 9576, 6388
SEEING = 2.5

# ---------------------------------------------------------------- lesson 02
section("02-seeing-and-sampling.md")
scale = 206.265 * PIX_UM / FL_MM
show("plate scale, \"/px", scale, 0.739, 0.001)
across = SEEING / scale
show("pixels across a star", across, 3.38, 0.01)
show("footprint, px", across ** 2, 11.4, 0.1)
show("2x2 binned plate scale", 2 * scale, 1.48, 0.01)
show("2x2 binned pixels across", across / 2, 1.69, 0.01)
# SNR table: 400 photons, read noise 2 e-, no sky.
for n, quoted in [(69, 15.4), (4, 19.6), (11, 19.0)]:
    show(f"SNR, {n} pixels read", 400 / sqrt(400 + n * 4), quoted, 0.05)
# Ideal pixel.
for name, fl, quoted in [("RASA 11", 620, 3.8), ("DeltaRho 350", 1050, 6.4), ("CDK17", 2939, 17.8)]:
    show(f"ideal pixel {name}, um", (SEEING / 2) * fl / 206.265, quoted, 0.05)
# Section 2.7 and reference page 17: the system point spread function.
FWHM_PER_SIGMA = 2 * sqrt(2 * log(2))
diffraction = 1.029 * 0.55e-6 / (D_MM / 1000) * ARCSEC_PER_RAD
show("diffraction FWHM, arcsec", diffraction, 0.33, 0.005)
optics_r = 1.665 * 4.9 * 206.265 / FL_MM
show("optics FWHM, 4.9 um read as RMS radius, arcsec", optics_r, 1.60, 0.005)
show("optics FWHM, read as RMS diameter, arcsec", optics_r / 2, 0.80, 0.005)
STAR_SAMPLED = sqrt(SEEING ** 2 + diffraction ** 2 + optics_r ** 2)
show("sampled star FWHM, arcsec", STAR_SAMPLED, 2.99, 0.005)
show("system-PSF pixels across a star", STAR_SAMPLED / scale, 4.05, 0.01)
star_d = sqrt(SEEING ** 2 + diffraction ** 2 + (optics_r / 2) ** 2)
show("sampled star, diameter reading, arcsec", star_d, 2.65, 0.005)
show("pixels across, diameter reading", star_d / scale, 3.58, 0.01)
show("system-PSF 2x2 binned pixels across", STAR_SAMPLED / scale / 2, 2.02, 0.01)
show("system-PSF ideal pixel DeltaRho, um", (STAR_SAMPLED / 2) * FL_MM / 206.265, 7.61, 0.01)
pixel_ap = FWHM_PER_SIGMA * PIX_UM / sqrt(12) * 206.265 / FL_MM
show("pixel aperture FWHM, arcsec", pixel_ap, 0.50, 0.005)
STAR = sqrt(STAR_SAMPLED ** 2 + pixel_ap ** 2)
show("recorded star FWHM, arcsec", STAR, 3.03, 0.005)
# Diffusion from MTF 0.5 at Nyquist: sigma = (p / pi) sqrt(-2 ln(0.5 / (2/pi))).
sigma_diff = PIX_UM / pi * sqrt(-2 * log(0.5 / (2 / pi)))
show("diffusion FWHM at MTF 0.5, um", FWHM_PER_SIGMA * sigma_diff, 1.96, 0.005)
# Peak pixel fraction of the sampled star, centred and on a corner.
sig_px = STAR_SAMPLED / scale / FWHM_PER_SIGMA
share = lambda d: 0.5 * (erf((0.5 - d) / (sqrt(2) * sig_px)) - erf((-0.5 - d) / (sqrt(2) * sig_px)))
show("peak pixel fraction, centred, %", 100 * share(0) ** 2, 5.2, 0.05)
show("peak pixel fraction, on a corner, %", 100 * share(0.5) ** 2, 4.8, 0.05)

# ---------------------------------------------------------------- lesson 03
section("03-optics-and-focus.md")
w_mm, h_mm = W_PX * PIX_UM / 1000, H_PX * PIX_UM / 1000
show("sensor width, mm", w_mm, 36.0, 0.05)
show("sensor height, mm", h_mm, 24.0, 0.05)
diag = sqrt(w_mm ** 2 + h_mm ** 2)
show("sensor diagonal, mm", diag, 43.3, 0.05)
seeing_um = SEEING * FL_MM / 206.265
show("seeing blur at focal plane, um", seeing_um, 12.7, 0.05)
show("Gaussian FWHM / RMS radius", 2 * sqrt(2 * log(2)) / sqrt(2), 1.665, 0.001)
# Spot interpolation: 4.9 um at 0 mm, 6.2 um at 23 mm; corner at diag/2.
corner_r = diag / 2
spot_corner = 4.9 + (6.2 - 4.9) * corner_r / 23
show("RMS spot at the corner, um", spot_corner, 6.1, 0.05)
for reading, factor, q_c, q_k in [("radius", 1.665, 0.19, 0.28), ("diameter", 0.833, 0.05, 0.08)]:
    for where, spot, quoted in [("center", 4.9, q_c), ("corner", spot_corner, q_k)]:
        optics = factor * spot
        growth = sqrt(seeing_um ** 2 + optics ** 2) / seeing_um - 1
        show(f"star growth, RMS {reading}, {where}", growth, quoted, 0.01)
for name, n, quoted in [("RASA 11", 620 / 279, 6.6), ("DeltaRho 350", 3.0, 12.1), ("CDK14", 2563 / 356, 69.6)]:
    show(f"CFZ {name}, +/- um", 2.44 * 0.55 * n ** 2, quoted, 0.05)
show("CFZ RASA 11 with N rounded to 2.2, +/- um", 2.44 * 0.55 * 2.2 ** 2, 6.5, 0.05)

# ---------------------------------------------------------------- lesson 04
section("04-light-collection-and-search.md")
def eff_area(d_m, blocked):
    return pi / 4 * d_m ** 2 * (1 - blocked)
a_dr = eff_area(0.350, 0.56 ** 2)
a_rasa = eff_area(0.279, (114 / 279) ** 2)
show("DeltaRho 350 effective area, m^2", a_dr, 0.0660, 0.0001)
show("RASA 11 effective area, m^2", a_rasa, 0.0509, 0.0001)
show("56% by diameter, as fraction of area", 0.56 ** 2, 0.314, 0.001)
show("depth difference, mag", 2.5 * log10(a_dr / a_rasa), 0.28, 0.01)
fov_dr = (w_mm / FL_MM * DEG_PER_RAD) * (h_mm / FL_MM * DEG_PER_RAD)
fov_rasa = (w_mm / 620 * DEG_PER_RAD) * (h_mm / 620 * DEG_PER_RAD)
show("DeltaRho 350 field area, deg^2", fov_dr, 2.58, 0.01)
show("RASA 11 field area, deg^2", fov_rasa, 7.39, 0.02)
show("RASA search speed vs DeltaRho", (a_rasa * fov_rasa) / (a_dr * fov_dr), 2.2, 0.05)

# ---------------------------------------------------------------- lesson 05
section("05-practical-fit.md")
show("payload load fraction, %", (46 + 2 + 10) / 100 * 100, 58, 1e-9)
show("49% by diameter, as % of area", 0.49 ** 2 * 100, 24, 0.1)

# ---------------------------------------------------------------- lesson 06
section("06-orbits-and-angular-rates.md")
v_leo = sqrt(MU_EARTH / (R_EARTH + 500))
show("LEO 500 km circular speed, km/s", v_leo, 7.61, 0.01)
show("LEO overhead rate, \"/s", v_leo / 500 * ARCSEC_PER_RAD, 3140, 5)
show("LEO overhead rate, deg/s", v_leo / 500 * DEG_PER_RAD, 0.87, 0.01)
v_meo = sqrt(MU_EARTH / (R_EARTH + 20_200))
show("MEO overhead rate, \"/s", v_meo / 20_200 * ARCSEC_PER_RAD, 40, 1)
sid = 1_296_000 / SIDEREAL_DAY_S
show("sidereal rate, \"/s", sid, 15.04, 0.01)
lunar = 1_296_000 / (27.32 * 86_400)
show("Moon vs stars, \"/s", lunar, 0.55, 0.01)
show("Moon vs ground, \"/s", sid - lunar, 14.5, 0.05)
a, e = 26_560.0, 0.74
r_apo = a * (1 + e)
show("Molniya apogee radius, km", r_apo, 46_214, 1)
v_apo = sqrt(MU_EARTH * (2 / r_apo - 1 / a))
show("Molniya apogee speed, km/s", v_apo, 1.50, 0.01)
show("Molniya apogee rate, \"/s", v_apo / (r_apo - R_EARTH) * ARCSEC_PER_RAD, 7.8, 0.05)
show("2 km at LEO range, arcsec", 2 / 500 * ARCSEC_PER_RAD, 825, 1)
show("2 km at GEO range, arcsec", 2 / 37_000 * ARCSEC_PER_RAD, 11, 0.2)
half_short = h_mm / FL_MM * DEG_PER_RAD * 3600 / 2
show("DeltaRho half short side, arcsec", half_short, 2359, 2)
show("acquisition margin, LEO", half_short / (825 + 30), 2.8, 0.05)
cdk17_half = h_mm / 2939 * DEG_PER_RAD * 3600 / 2
show("CDK17 half short side, arcsec", cdk17_half, 843, 1)
show("CDK17 acquisition margin, LEO", cdk17_half / (2 / 500 * ARCSEC_PER_RAD + 30), 0.99, 0.01)
show("LEO field dwell, s", 2 * half_short / 3140, 1.5, 0.05)

# ---------------------------------------------------------------- lesson 07
section("07-timing-and-shutters.md")
binned = 2 * scale
show("GEO star streak, px per s (2x2)", sid / binned, 10, 0.2)
show("GEO error for 10 ms timing, arcsec", sid * 0.010, 0.15, 0.005)
for name, rate, quoted in [("LEO", 3140, 0.12e-3), ("MEO", 40, 9.3e-3), ("GEO", sid, 24.6e-3)]:
    show(f"required timing {name}, ms", 0.25 * binned / rate * 1000, quoted * 1000, 0.3)
readout = 6388 * 39.028e-6
show("IMX455 readout time, s", readout, 0.249, 0.001)
show("GEO rolling-shutter skew, arcsec", sid * readout, 3.75, 0.01)
show("LEO rolling-shutter skew, arcsec", 3140 * readout, 783, 1)
show("LEO skew as % of frame height", 3140 * readout / (2 * half_short) * 100, 16.6, 0.1)
show("LEO crossing time, ms", SEEING / 3140 * 1000, 0.8, 0.01)

# ---------------------------------------------------------------- lesson 08
section("08-mount-dynamics.md")
omega_leo = 0.87234  # deg/s, as computed by the tool
z = radians(omega_leo / 50)  # rate-limited keyhole: z = omega / max rate (both in deg/s gives rad)
show("rate-limited keyhole, L-350, elevation deg", 90 - degrees(omega_leo / 50), 89.0, 0.05)
show("keyhole for a 3 deg/s mount, elevation deg", 90 - degrees(0.87 / 3), 73, 0.5)
coeff = 3 * sqrt(3) / 8
show("PEAK_ACCEL_COEFF", coeff, 0.6495, 0.0001)
show("LEO peak tracking accel, deg/s^2", coeff * omega_leo ** 2 * pi / 180, 0.008627, 1e-6)
for acc, quoted in [(10, 88.3), (2, 86.2), (0.5, 82.5)]:
    # omega and acc in rad units: z = omega * sqrt(coeff / acc)
    om = radians(omega_leo)
    z_acc = om * sqrt(coeff / radians(acc))
    show(f"accel-limited keyhole at {acc} deg/s^2, elev deg", 90 - degrees(z_acc), quoted, 0.05)
def slew(d, v, a):
    return v / a + d / v if d >= v * v / a else 2 * sqrt(d / a)
show("slew 90 deg, 50 deg/s, 10 deg/s^2, s", slew(90, 50, 10), 6.0, 0.01)
show("slew 90 deg, 50 deg/s, 50 deg/s^2, s", slew(90, 50, 50), 2.8, 0.01)
show("slew 90 deg, 6 deg/s, 1 deg/s^2, s", slew(90, 6, 1), 21.0, 0.01)

# ---------------------------------------------------------------- lesson 09
section("09-brightness-and-detection.md")
def target_mag(range_km, area=10.0, albedo=0.2, phase=1.0):
    d = range_km * 1000
    return -26.74 - 2.5 * log10(albedo * area * phase / (pi * d * d))
for name, rng, quoted in [("LEO", 500, 2.25), ("MEO", 20_200, 10.28), ("GEO", 37_000, 11.59),
                          ("HEO", 39_836, 11.75), ("Cislunar", 384_400, 16.67)]:
    show(f"derived magnitude {name}", target_mag(rng), quoted, 0.01)
watts = 3.64e-23 * (3e8 / 550e-9 ** 2) * 89e-9  # f_nu * (c / lambda^2) * bandwidth
show("V-band mag-0 flux, W/m^2", watts, 3.21e-9, 0.02e-9)
photon_j = 6.626e-34 * 3e8 / 550e-9
show("energy of a 550 nm photon, J", photon_j, 3.61e-19, 0.01e-19)
show("mag-0 photons per m^2 per s", watts / photon_j, 8.9e9, 0.05e9)
K0 = 8.9e9
QE, TP, SKY, RN = 0.80, 0.85, 21.0, 3.0
# The detection check sizes everything on the recorded star (lesson 2, section 2.7).
show("cislunar exposure, seeing alone, s", SEEING / lunar, 4.554, 0.01)
exp_cis = STAR / lunar
show("cislunar trail-limited exposure, s", exp_cis, 5.519, 0.01)
fp_cis = (STAR / scale) * ((STAR + lunar * exp_cis) / scale)
show("cislunar footprint, px", fp_cis, 33.7, 0.05)


def snr_and_limit(mag, exposure, footprint, threshold=5.0):
    sig_rate = K0 * 10 ** (-0.4 * mag) * a_dr * QE * TP
    sky_rate = K0 * 10 ** (-0.4 * SKY) * a_dr * QE * TP * scale ** 2
    s = sig_rate * exposure
    noise = sky_rate * exposure * footprint + RN ** 2 * footprint
    snr = s / sqrt(s + noise)
    t = threshold
    s_min = (t * t + sqrt(t ** 4 + 4 * t * t * noise)) / 2
    k = K0 * a_dr * QE * TP * exposure
    return snr, -2.5 * log10(s_min / k)


fp_still = (STAR / scale) ** 2
show("stationary footprint, px", fp_still, 16.83, 0.01)
snr_geo, lim_still = snr_and_limit(target_mag(37_000), 30, fp_still)
show("GEO SNR (30 s, stare)", snr_geo, 526, 3)
show("limiting mag, stationary regimes", lim_still, 19.87, 0.02)
show("GEO centroid precision, mas", STAR / FWHM_PER_SIGMA / snr_geo * 1000, 2.4, 0.05)
sig_geo = K0 * 10 ** (-0.4 * target_mag(37_000)) * a_dr * QE * TP * 30
sky_geo_px = K0 * 10 ** (-0.4 * SKY) * a_dr * QE * TP * scale ** 2 * 30
show("GEO signal in 30 s, e-", sig_geo, 2.769e5, 0.001e5)
show("GEO sky per pixel in 30 s, e-", sky_geo_px, 26.0, 0.05)
show("GEO sky in footprint B, e-", sky_geo_px * fp_still, 438, 1)
show("GEO read-noise variance R^2 n, e-^2", RN ** 2 * fp_still, 151, 0.5)
snr_cis, lim_cis = snr_and_limit(target_mag(384_400), exp_cis, fp_cis)
show("cislunar SNR", snr_cis, 15.4, 0.1)
show("cislunar limiting mag", lim_cis, 18.15, 0.02)

# ---------------------------------------------------------------- lesson 10
section("10-pass-prediction.md")
MU_PRECISE, R_PRECISE, J2 = 398_600.4418, 6_378.137, 1.08262668e-3


def tle_checksum(line):
    total = sum(int(c) for c in line[:68] if c.isdigit()) + line[:68].count("-")
    return total % 10


ISS_1 = "1 25544U 98067A   08264.51782528 -.00002182  00000-0 -11606-4 0  2927"
ISS_2 = "2 25544  51.6416 247.4627 0006703 130.5360 325.0288 15.72125391563537"
show("ISS line 1 checksum", tle_checksum(ISS_1), int(ISS_1[68]), 0)
show("ISS line 2 checksum", tle_checksum(ISS_2), int(ISS_2[68]), 0)
rev_per_day = float(ISS_2[52:63])
show("ISS period, min", 86_400 / rev_per_day / 60, 91.6, 0.05)
n_iss = rev_per_day * 2 * pi / 86_400
a_iss = (MU_PRECISE / n_iss ** 2) ** (1 / 3)
show("ISS semi-major axis, km", a_iss, 6_731, 1)
show("ISS mean altitude, km", a_iss - R_PRECISE, 353, 1)


def node_drift_deg_per_day(alt_km, inc_deg, e=0.0):
    a = R_PRECISE + alt_km
    n = sqrt(MU_PRECISE / a ** 3)
    p = a * (1 - e * e)
    k = 1.5 * J2 * (R_PRECISE / p) ** 2 * n
    return degrees(-k * cos(radians(inc_deg))) * 86_400


show("J2 node drift, 550 km at 53 deg, deg/day", node_drift_deg_per_day(550, 53), -4.49, 0.01)
show("J2 node drift, 800 km at 98.6 deg, deg/day", node_drift_deg_per_day(800, 98.6), 0.985, 0.005)
jd = 2_448_855.009722222  # 1992-08-20 12:14 UT
T = (jd - 2_451_545.0) / 36_525
gmst_s = 67_310.54841 + (876_600 * 3600 + 8_640_184.812866) * T + 0.093104 * T ** 2 - 6.2e-6 * T ** 3
show("GMST, Vallado example 3-5, deg", (gmst_s % 86_400) / 86_400 * 360, 152.578788, 1e-5)
n_gps = 2.00563 * 2 * pi / 86_400
show("a for 2.00563 rev/day, km", (MU_PRECISE / n_gps ** 2) ** (1 / 3), 26_560, 1)

print()
if failures:
    print(f"{failures} value(s) disagree with the docs.")
    raise SystemExit(1)
print("Every worked example agrees with the docs.")
