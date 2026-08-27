#!/usr/bin/env python3
"""Regenerate fixtures/synthetic/shading/ — a shading and a flat fill of the
SAME authored ink, side by side.

WHY THIS EXISTS
---------------
A shading's colour used to be resolved to three-channel sRGB when its colour
ramp was BUILT, so by the time anything composited there were no colorants left.
On a page that composites in ink, that meant a `CMYK -> sRGB -> CMYK` round
trip, and the return leg is a DIFFERENT function from the outbound one (a
calibrated table out, a naive formula back). The ink that arrived was not the
ink that left.

For a long time this was invisible, because EVERYTHING on the page took the same
round trip and so everything was consistently slightly wrong together.

★ IT BECAME VISIBLE WHEN THE OTHER HALF WAS FIXED. `Pass 130.1` gave a
`DeviceCMYK` image its authored ink, so images stopped round-tripping. From then
on the same colour drawn as a shading and as an image came out DIFFERENT — and
the operator found it on a conformance sheet whose shading boxes print a live
shading beside a reference IMAGE of what it should look like, captioned "the
shadings should look like the reference image". Two of four pairs visibly
disagreed. That box carries no trap cross, so nothing automated could see it.

⇒ Fixing one half of a two-halves-agree situation turns a silent shared error
into a visible disagreement. That is an argument FOR fixing halves, not against
— the disagreement is information — but it means the second half becomes urgent
in a way it was not before.

WHAT THE FIXTURES ARE
---------------------
Two single-page PDFs, 200 x 100 pt, each drawing the SAME `DeviceCMYK` colour
twice: once as a flat filled rectangle, once as an axial shading whose function
is CONSTANT (its two ends are the same colour).

| file | page group | what it pins |
|---|---|---|
| `shading-vs-fill-cmyk.pdf` | `/DeviceCMYK` | on an ink page the two must be the SAME colour |
| `shading-vs-fill-rgb.pdf` | *(none)* | the additive control: they must match there too |

★ THE CONSTANT SHADING IS THE POINT. A gradient cannot be compared against a
flat fill without picking a parametric position and arguing about it. A shading
whose ramp is constant is the same colour everywhere, so ANY pixel of it is
comparable to ANY pixel of the fill, and the assertion needs no geometry.

★★ AND THE COLOUR IS DELIBERATELY NOT A PRIMARY. `0.42 0.87 0.13 0.06` has all
four colorants non-zero and is nowhere near a `CMYK <-> sRGB` fixed point. A
round trip through sRGB moves it measurably; a colour like pure cyan or black
survives the trip nearly intact and would let the defect pass unnoticed.

PROVENANCE
----------
Authored here, byte by byte, from ISO 32000-1's own object syntax. No
third-party PDF is copied, adapted or consulted (`docs/LEGAL.md` §5), and in
particular nothing from the licensed conformance suite that exposed the defect.

USAGE
-----
    python tools/gen-shading-ink-fixtures.py
"""

from __future__ import annotations

import pathlib

OUT = pathlib.Path(__file__).resolve().parent.parent / "fixtures" / "synthetic" / "shading"

# All four colorants non-zero, far from any CMYK<->sRGB fixed point.
INK = b"0.42 0.87 0.13 0.06"


def assemble(objects: list[bytes]) -> bytes:
    out = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
    offsets = [0]
    for i, body in enumerate(objects, start=1):
        offsets.append(len(out))
        out += str(i).encode() + b" 0 obj\n" + body + b"\nendobj\n"
    startxref = len(out)
    n = len(objects) + 1
    out += b"xref\n0 " + str(n).encode() + b"\n0000000000 65535 f \n"
    for off in offsets[1:]:
        out += f"{off:010d} 00000 n \n".encode()
    out += (
        b"trailer\n<< /Size " + str(n).encode() + b" /Root 1 0 R >>\n"
        b"startxref\n" + str(startxref).encode() + b"\n%%EOF\n"
    )
    return bytes(out)


def stream(dict_body: bytes, data: bytes) -> bytes:
    return (
        b"<< " + dict_body + b" /Length " + str(len(data)).encode() + b" >>\nstream\n"
        + data
        + b"\nendstream"
    )


def build(subtractive: bool) -> bytes:
    # Left: a flat fill. Right: an axial shading clipped to a rectangle, whose
    # function returns the SAME colour at both ends.
    content = (
        b"q " + INK + b" k\n10 20 80 60 re f\nQ\n"
        b"q\n110 20 80 60 re W n\n/Sh0 sh\nQ\n"
    )
    group = b"/Group << /S /Transparency /CS /DeviceCMYK >> " if subtractive else b""
    return assemble([
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] " + group
        + b"/Resources << /Shading << /Sh0 5 0 R >> >> /Contents 4 0 R >>",
        stream(b"", content),
        # ShadingType 2 (axial), DeviceCMYK, across the right-hand rectangle.
        b"<< /ShadingType 2 /ColorSpace /DeviceCMYK /Coords [110 0 190 0] "
        b"/Function 6 0 R /Extend [true true] >>",
        # Type 2 exponential interpolation with C0 == C1: constant colour.
        b"<< /FunctionType 2 /Domain [0 1] /C0 [" + INK + b"] /C1 [" + INK
        + b"] /N 1 >>",
    ])


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    for name, data in {
        "shading-vs-fill-cmyk.pdf": build(subtractive=True),
        "shading-vs-fill-rgb.pdf": build(subtractive=False),
    }.items():
        (OUT / name).write_bytes(data)
        print(f"wrote {OUT / name}  ({len(data)} bytes)")


if __name__ == "__main__":
    main()
