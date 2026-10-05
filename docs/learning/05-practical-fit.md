# Lesson 5: Practical fit

The last general check is the least glamorous and the one most often skipped: will the mount carry the payload with margin, and is there enough back focus for everything you want to put between the telescope and the sensor? This is check 8.

**Prerequisites:** none beyond the running example.

## You will be able to

* Build a payload budget and judge it against a mount's rating.
* Explain why mounts are run well below their rated payload.
* Check a back-focus budget and spot the most common way it goes wrong.

## Key terms

| Term | Meaning |
|---|---|
| **OTA** | Optical tube assembly: the telescope itself, without mount or camera. |
| **Payload rating** | The maximum load a mount's maker says it can carry. |
| **Load fraction** | Payload / rating. |
| **Back focus** | The distance behind the telescope's reference surface at which it comes to focus. Everything in the imaging train must fit inside it. |
| **Imaging train** | The parts between telescope and sensor: focuser, filter wheel, adapters, the camera's own internal distance to its sensor. |

---

## 5.1 Payload

**Formulas.**

```
payload       = OTA weight + camera weight + accessories weight
load fraction = payload / mount rating
```

**Why 70%.** A mount's rating is usually the most it can move, not the most it can move *well*. Near the limit, wind shake, vibration after a slew and tracking error all get worse. Satellite tracking makes it worse still, because the mount is accelerating and slewing rather than creeping at the sidereal rate. Keeping the payload at or below about 70% of the rating is a common rule of thumb.

**Don't forget the accessories.** Focuser, dew heaters, cables, filter wheel, dovetail bars and guide scope typically add 10 to 20 lb. Spec sheets quote the OTA alone.

**Worked example.**

```
DeltaRho 350 (46 lb) + IMX455 camera (2 lb) + accessories (10 lb) = 58 lb
58 / 100 lb (L-350 rating) = 58%   -> PASS
```

**Grading.** PASS at 70% or less, WARN up to 90%, FAIL above 90%.

## 5.2 Back focus

**Formula.**

```
back focus OK if available >= required
required = sum of the optical path lengths of everything in the imaging train
```

**The common trap: reference points.** Vendors measure back focus from different places: "from the mounting surface without focuser" versus "with focuser installed at mid-travel" can differ by tens of millimeters. The requirement you enter must be measured from the same reference surface as the telescope's figure.

**Grading.** PASS if available >= required, FAIL if short. Anything left blank is skipped and noted; if nothing can be evaluated the check is INFO.

## 5.3 Reading spec sheets critically

Check 8 depends entirely on numbers you take from spec sheets, and so do most of the other checks. The main README's [Spec-sheet red flags](../../README.md#spec-sheet-red-flags) section lists the traps worth checking before you trust a number. Each one connects to a lesson:

| Red flag | Lesson |
|---|---|
| Obstruction quoted by area | [Lesson 4](04-light-collection-and-search.md) |
| "Image circle" vs "fully corrected field" | [Lesson 3](03-optics-and-focus.md) |
| RMS spot without radius/diameter or without off-axis values | [Lesson 3](03-optics-and-focus.md) |
| OTA weight alone | This lesson |
| Back-focus reference point | This lesson |

---

## Validate it yourself

* **By hand.** Add your own accessories to the budget. At what accessory weight does the DeltaRho 350 + IMX455 on an L-350 reach WARN? (Answer below.)
* **Script.** `python3 docs/learning/check_examples.py` (section `05-practical-fit.md`).
* **Unit test.**

  ```bash
  cargo test payload_and_back_focus_margins
  ```

* **Tool output.** Check 8 in `--demo` shows the payload sum and percentage for each configuration. The comparison table's `Load%` column summarizes them.

## Self-check

1. At what accessory weight does the running example reach WARN? FAIL?
2. A DeltaRho 500 weighs 165 lb. With 15 lb of accessories and a camera whose weight you don't know, what load fraction does it put on a 200 lb mount? What does the unknown camera weight mean for the verdict?
3. Your camera needs 55 mm of back focus and the telescope offers 56 mm "from the focuser drawtube". Your focuser isn't installed yet. Is it OK?

<details>
<summary>Answers</summary>

1. WARN above 70 lb total, so above 22 lb of accessories. FAIL above 90 lb total, so above 42 lb.
2. 180 / 200 = 90%: right at the WARN/FAIL boundary. This is the DeltaRho 500 row in the demo (check 8 notes that the camera is not included). Any camera at all pushes it past 90% into FAIL, so the WARN is optimistic.
3. You can't tell yet. The 56 mm figure assumes the vendor's focuser is in place. Measure from the same reference surface, or ask the vendor.

</details>

## Where it lives in the code

* `src/calculations/mount.rs`: `total_weight_lb`, `capacity_fraction`, `back_focus_margin_mm`.
* `src/checks.rs`: `check_practical_fit`.
* `src/constants.rs`, module `checks_limits`: `PAYLOAD_PASS_FRACTION`, `PAYLOAD_WARN_FRACTION`.

**Next:** [Lesson 6: Orbits and angular rates](06-orbits-and-angular-rates.md)
