#!/usr/bin/env python3
"""Compare the numeric probe to Python's independent arbitrary-precision ints.

This is supplementary validation, not a substitute for the required proptest,
Rust big-integer oracle, or the bounded Miri checks documented in the handoff.
"""

import argparse
import random
import subprocess
import tempfile


def oracle(a, sa, b, sb, target, mode):
    """Integer-only reference, independent of qlrs-num's wide arithmetic."""
    product = a * b
    exponent = sa + sb - target
    if exponent <= 0:
        value = product * 10 ** (-exponent)
    else:
        divisor = 10**exponent
        quotient, remainder = divmod(abs(product), divisor)
        if mode == "Exact" and remainder:
            # The wide path checks the final quotient range before rounding.
            limit = (1 << 127) if product < 0 else (1 << 127) - 1
            if quotient > limit:
                return "numeric-overflow"
            return "rounding-policy-required"
        if mode in ("HalfEven", "HalfUp"):
            doubled = remainder * 2
            if doubled > divisor or (
                doubled == divisor and (mode == "HalfUp" or quotient % 2)
            ):
                quotient += 1
        value = -quotient if product < 0 else quotient
    if not -(1 << 127) <= value < (1 << 127):
        return "numeric-overflow"
    return f"{value} {target}"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--probe", default="target/debug/examples/numeric_probe")
    parser.add_argument("--cases", type=int, default=100_000)
    args = parser.parse_args()
    rng = random.Random(0x514C5253)
    edges = [0, 1, -1, (1 << 127) - 1, -(1 << 127), 10**18, -(10**18)]
    count = 0
    # File-backed subprocess I/O avoids pipe deadlocks and unbounded memory.
    with tempfile.TemporaryFile(mode="w+") as inputs, tempfile.TemporaryFile(
        mode="w+"
    ) as expected, tempfile.TemporaryFile(mode="w+") as actual:
        for _ in range(args.cases):
            a = rng.choice(edges) if rng.randrange(4) == 0 else rng.randrange(-(1 << 127), 1 << 127)
            b = rng.choice(edges) if rng.randrange(4) == 0 else rng.randrange(-(1 << 127), 1 << 127)
            sa, sb, target = (rng.randrange(19) for _ in range(3))
            for mode in ("Exact", "HalfEven", "HalfUp", "Down"):
                inputs.write(f"{a} {sa} {b} {sb} {target} {mode}\n")
                expected.write(oracle(a, sa, b, sb, target, mode) + "\n")
                count += 1
        inputs.seek(0)
        expected.seek(0)
        subprocess.run([args.probe], stdin=inputs, stdout=actual, check=True)
        actual.seek(0)
        for index, reference in enumerate(expected, start=1):
            result = actual.readline()
            if reference != result:
                raise AssertionError(f"case {index}: expected {reference!r}, got {result!r}")
        if actual.readline():
            raise AssertionError("probe emitted extra results")
    print(f"PASS: {args.cases} seeded operands x 4 modes = {count} oracle comparisons")


if __name__ == "__main__":
    main()
