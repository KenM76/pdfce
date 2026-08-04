#!/usr/bin/env python3
"""Generate a synthetic PDF reproducing the CAD-export structure that makes
individual lines unselectable: MANY subpaths inside ONE path object.

Why this fixture exists
-----------------------
A SolidWorks export the operator supplied has, on page 1, a single stroked
path object with 1194 subpaths and 6681 anchors spanning a 550x500 pt
isometric view. Every visible line of that view is one object as far as
per-object hit testing is concerned, so clicking any line selects all of
them. That is the whole reason `hit_test_subpaths` exists.

That export is proprietary work product and is not in this repository
(docs/LEGAL.md section 5). Only the STRUCTURE it exposed is reproduced here,
from first principles: one `m`/`l` run per line, all between a single `q` and
a single `S`, so the decomposer sees one object with many subpaths. Six lines
are enough - the defect never needed 1194, it needed more than one.

Layout (PDF user space, origin bottom-left), a 400x400 page:

    y=340  ---------------- horizontal A (a long line)
    y=280  ---------------- horizontal B
    y=220  ---------------- horizontal C
    x=100  |                vertical D
    x=200  |                vertical E
    x=300  |                vertical F

The horizontals are 60 pt apart, comfortably more than any sane click
tolerance, so a click near one of them is unambiguous about WHICH subpath is
nearest - which is the property the ordering assertion needs.

Usage:  python tools/gen-multi-subpath-fixture.py
Writes: fixtures/synthetic/vector/multi-subpath-one-object.pdf
"""

import os
import pathlib

# One line per subpath. Each is (x0, y0, x1, y1) in PDF user space.
LINES = [
    (50.0, 340.0, 350.0, 340.0),  # subpath 0 - horizontal A
    (50.0, 280.0, 350.0, 280.0),  # subpath 1 - horizontal B
    (50.0, 220.0, 350.0, 220.0),  # subpath 2 - horizontal C
    (100.0, 50.0, 100.0, 180.0),  # subpath 3 - vertical D
    (200.0, 50.0, 200.0, 180.0),  # subpath 4 - vertical E
    (300.0, 50.0, 300.0, 180.0),  # subpath 5 - vertical F
]


def content_stream() -> bytes:
    """One `q` ... `S` group containing every line as its own subpath.

    The single trailing `S` is the load-bearing part: it makes all six
    subpaths one painting operation, hence ONE object in the decomposed
    model. Emitting `S` after each `m`/`l` pair would produce six separate
    objects and reproduce nothing.
    """
    parts = ["q", "1 w", "0 0 0 RG"]
    for x0, y0, x1, y1 in LINES:
        parts.append(f"{x0} {y0} m {x1} {y1} l")
    parts.append("S")
    parts.append("Q")
    return ("\n".join(parts) + "\n").encode("ascii")


def build() -> bytes:
    stream = content_stream()
    objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] "
        b"/Resources << >> /Contents 4 0 R >>",
        b"<< /Length " + str(len(stream)).encode("ascii") + b" >>\nstream\n" + stream + b"endstream",
    ]

    out = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
    offsets = [0]
    for i, body in enumerate(objects, start=1):
        offsets.append(len(out))
        out += f"{i} 0 obj\n".encode("ascii") + body + b"\nendobj\n"

    xref_at = len(out)
    n = len(objects) + 1
    out += f"xref\n0 {n}\n".encode("ascii")
    out += b"0000000000 65535 f \n"
    for off in offsets[1:]:
        out += f"{off:010d} 00000 n \n".encode("ascii")
    out += (
        f"trailer\n<< /Size {n} /Root 1 0 R >>\nstartxref\n{xref_at}\n".encode("ascii")
        + b"%%EOF\n"
    )
    return bytes(out)


def main() -> None:
    root = pathlib.Path(__file__).resolve().parent.parent
    dest = root / "fixtures" / "synthetic" / "vector" / "multi-subpath-one-object.pdf"
    os.makedirs(dest.parent, exist_ok=True)
    data = build()
    dest.write_bytes(data)
    print(f"wrote {dest} ({len(data)} bytes, {len(LINES)} subpaths in one object)")


if __name__ == "__main__":
    main()
