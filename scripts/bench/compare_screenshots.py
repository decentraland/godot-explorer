#!/usr/bin/env python3
# Requires Pillow + imagehash. On macOS with PEP 668 system Python, use a venv:
#   python3 -m venv .bench-venv && .bench-venv/bin/pip install pillow imagehash
#   .bench-venv/bin/python scripts/bench/compare_screenshots.py ref.png cand.png
# launch_devices.sh resolves this automatically when --pull-results is set.
"""Sanity-check benchmark screenshots — flag runs whose final frame diverges
too far from a reference baseline (different scene loaded, character drifted,
asset failed to render).

Compares two PNGs via perceptual hash (pHash). Two screenshots of the same
viewpoint typically score >0.95 similarity even with minor lighting differences;
a different scene or wrong avatar drops below 0.7.

Usage:
    compare_screenshots.py <reference.png> <candidate.png> [--threshold 0.80]

Exits 0 if similarity >= threshold, 2 if below (with the run-flagged warning),
1 on missing files / decode errors.

The threshold default is 0.80 (matches user spec for "80% coincidence").
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

try:
    from PIL import Image
    import imagehash
except ImportError:
    sys.stderr.write(
        "Missing dependencies. Install with: pip install pillow imagehash\n"
    )
    sys.exit(1)


def phash_similarity(a: Path, b: Path, hash_size: int = 16) -> float:
    """Return similarity in [0.0, 1.0] based on pHash Hamming distance."""
    img_a = Image.open(a).convert("RGB")
    img_b = Image.open(b).convert("RGB")
    h_a = imagehash.phash(img_a, hash_size=hash_size)
    h_b = imagehash.phash(img_b, hash_size=hash_size)
    distance = h_a - h_b
    bits = hash_size * hash_size
    return 1.0 - (distance / bits)


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("reference", type=Path)
    p.add_argument("candidate", type=Path)
    p.add_argument("--threshold", type=float, default=0.80)
    args = p.parse_args()

    if not args.reference.is_file():
        sys.stderr.write(f"reference not found: {args.reference}\n")
        return 1
    if not args.candidate.is_file():
        sys.stderr.write(f"candidate not found: {args.candidate}\n")
        return 1

    similarity = phash_similarity(args.reference, args.candidate)
    print(
        f"similarity={similarity:.4f} threshold={args.threshold:.2f} "
        f"reference={args.reference.name} candidate={args.candidate.name}"
    )
    if similarity < args.threshold:
        print(
            f"WARN: candidate diverges from reference (similarity={similarity:.4f} "
            f"< threshold={args.threshold:.2f}); discard the run's metrics",
            file=sys.stderr,
        )
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
