#!/usr/bin/env python3
"""Recompute every equation in a scope-eval report printed with --equations.

Each equation line reads `name: formula = substituted = result`. This
script evaluates the substituted expression with Python's own math and
checks that it gives the printed result, so the report's arithmetic is
confirmed independently of the Rust code, the way check_examples.py
confirms the worked examples.

Usage:
    cargo run -- --demo --equations | python3 docs/learning/check_equations.py

It exits non-zero if any equation does not reproduce. Lines whose
substitution is not plain arithmetic (a choice between bins, a numerical
solve) are counted as skipped, not checked.
"""

import math
import re
import sys

# Printed numbers keep about five significant figures, so a result
# recomputed from printed inputs can differ in the fifth figure, more after
# a subtraction such as "growth - 1".
REL_TOL = 2e-3
ABS_TOL = 2e-4

NUMBER = re.compile(r"[-+]?(?:\d+\.?\d*|\.\d+)(?:e[-+]?\d+)?")
ALLOWED = re.compile(r"^[\d\s.eE+\-*/(),]*$")


def to_python(expr):
    """The report's arithmetic in Python syntax, or None if it is not arithmetic."""
    e = expr.strip().rstrip('"').replace('"', "")
    e = e.replace(" x ", " * ").replace("^", "**")
    names = {"sqrt": "math.sqrt", "atan": "math.atan", "log10": "math.log10", "min": "min"}
    plain = e.replace("**", "^")
    for word in names:
        plain = plain.replace(word, "")
    plain = plain.replace("pi", "")
    if not ALLOWED.match(plain.replace("^", "")):
        return None
    for word, py in names.items():
        e = re.sub(rf"\b{word}\(", f"{py}(", e)
    return re.sub(r"\bpi\b", "math.pi", e)


def numbers(text):
    t = text.strip()
    if t.startswith("+/-"):
        t = t[3:]
    if t.startswith("x"):
        t = t[1:]
    return [float(n) for n in NUMBER.findall(t)]


def check_line(line):
    """'ok', 'skip' or an error message for one equation line."""
    parts = line.split(" = ")
    for i in range(1, len(parts) - 1):
        py = to_python(parts[i])
        if py is None:
            continue
        try:
            value = eval(py, {"math": math, "min": min})  # noqa: S307 - our own report text
        except (SyntaxError, ValueError, ZeroDivisionError, NameError, TypeError):
            continue
        result_text = parts[i + 1]
        printed = numbers(result_text)
        if not printed:
            return "skip"
        values = list(value) if isinstance(value, tuple) else [value]
        if result_text.strip().endswith("%") or "%" in result_text.split()[0]:
            values = [v * 100.0 for v in values]
        for v, p in zip(values, printed):
            if not math.isclose(v, p, rel_tol=REL_TOL, abs_tol=ABS_TOL):
                return f"computes {v:.6g}, printed {p:.6g}"
        return "ok"
    return "skip"


def main():
    ok = skipped = 0
    failures = []
    for raw in sys.stdin:
        line = raw.strip()
        if " = " not in line or line.startswith("Rule:") or ":" not in line:
            continue
        if line.startswith(("Payload (", "Prediction error at")):  # detail lines, not equations
            continue
        result = check_line(line)
        if result == "ok":
            ok += 1
        elif result == "skip":
            skipped += 1
        else:
            failures.append(f"{line}\n    -> {result}")
    for f in failures:
        print(f"MISMATCH {f}")
    print(f"{ok} equations reproduced, {skipped} skipped (not plain arithmetic), {len(failures)} mismatched")
    return 1 if failures or ok == 0 else 0


if __name__ == "__main__":
    sys.exit(main())
