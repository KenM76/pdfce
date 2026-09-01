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


# ---------------------------------------------------------------------------
# Pass 201.0 -- an overprinting MIXED /DeviceN shading must not write channels
# it never claims.
# ---------------------------------------------------------------------------
#
# `Pass 195.0` fixed a spot colorant's ink being DROPPED by widening a mixed
# `/DeviceN` to write all four components. That wrote channels the source never
# claimed, and its own comment said so while concluding it was safe: "no patch
# in the conformance corpus detects that". One does -- a `1 0 1 .5 k` mark under
# such a shading lost its `K = 0.5` and vanished, sixteen times on one page.
# **K is a plane pdfce HAS**, so this was ink being ERASED by a fix for ink
# being DROPPED, not the missing per-spot-colorant plane.
#
# The tint transform: stack in `spot cyan`.  exch -> cyan spot | 0 -> cyan spot
# 0 | exch -> cyan 0 spot | 0 -> cyan 0 spot 0.  Out: C=cyan, M=0, Y=spot, K=0.
# The simplest program that reaches two channels and NEVER K, which is what
# makes the fixture discriminating.
TINT = b"{ exch 0 exch 0 }"


def overprint_mixed(overprint: bool) -> bytes:
    """A `1 0 1 0.5 k` rectangle under a mixed-`/DeviceN` axial shading.

    ★ TWO THINGS HERE ARE LOAD-BEARING AND WERE EACH GOT WRONG ONCE WHILE
    WRITING THIS, recorded so the next author does not repeat them:

    1. **The page needs a `/Group` with `/CS /DeviceCMYK`.** Without a
       subtractive blending space pdfce opens no colorant buffer, Table 149
       never runs, and the overprint-on and overprint-off pages render
       IDENTICALLY -- the fixture asserts nothing while looking fine.
       `cmyk_buffer=0` was the tell.
    2. **The shading's `/Function` and the colour space's tint transform are
       DIFFERENT functions.** The shading's maps the parametric `t` (ONE input)
       to the space's two components; the tint transform maps those two to
       DeviceCMYK. Handing the tint transform to the shading gives a two-input
       function one input and the ramp comes out FLAT -- while still reporting
       `shadings_painted=1`.
    """
    op = b"true" if overprint else b"false"
    content = (
        b"1 0 1 0.5 k 20 20 160 160 re f\n"
        b"q /GS0 gs /Sh0 sh Q\n"
    )
    return assemble([
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] "
        b"/Group << /S /Transparency /CS /DeviceCMYK /I false /K false >> "
        b"/Resources << /ExtGState << /GS0 5 0 R >> /Shading << /Sh0 6 0 R >> >> "
        b"/Contents 4 0 R >>",
        stream(b"", content),
        b"<< /Type /ExtGState /OP " + op + b" /op " + op + b" /OPM 1 >>",
        b"<< /ShadingType 2 /ColorSpace 7 0 R /Coords [20 0 180 0] "
        b"/Function 8 0 R /Extend [true true] >>",
        b"[ /DeviceN [/Spot#20Green /Cyan] /DeviceCMYK 9 0 R ]",
        # The SHADING's function: t -> (spot, cyan).
        b"<< /FunctionType 2 /Domain [0 1] /C0 [1 0] /C1 [0 1] /N 1 >>",
        # The COLOUR SPACE's tint transform: (spot, cyan) -> CMYK.
        # NOTE the shape: this file's `stream()` supplies the surrounding
        # `<< ... /Length N >>` itself, so the body passed in must be the
        # ENTRIES ONLY. Passing a complete dictionary produces `<< << ... >> >>`
        # and the object fails to parse with `DictKeyNotName`.
        stream(b"/FunctionType 4 /Domain [0 1 0 1] /Range [0 1 0 1 0 1 0 1]", TINT),
    ])


# ---------------------------------------------------------------------------
# Pass 202.0 -- a SPOT-ONLY /DeviceN shading under overprint must still paint.
# ---------------------------------------------------------------------------
#
# A `/DeviceN` naming ONLY spot colorants puts all four of the group's process
# components in Table 149's "not named in the source space" column, which under
# `/OP true` is `c_b` -- the backdrop. Composited literally that is correct for
# a press, where the spot ink sits on its own plate, and a VANISHED MARK for a
# renderer with four process planes and no spot plane.
#
# ★ THE DEFECT THIS PINS WAS A COMMENT WITHOUT A GUARD. `interpret.rs` carried
# a detailed block -- naming the patch, the shape, and the exact "451 x 29
# device pixels of bare white paper" it produced -- announcing a refusal that
# routed such a shading to the flattening bridge instead. The comment shipped;
# the condition did not. Prose describing a safeguard is indistinguishable from
# a safeguard to every tool in this project, which is why the regression test
# is a RENDER rather than a code inspection.
#
# The tint transform: inputs (s1, s2) are on the stack; `{ 0 0 }` appends two
# zeros, giving C=s1, M=s2, Y=0, K=0. Deliberately reaching two process
# channels through the TRANSFORM while naming no process colorant in the SPACE
# -- that combination is exactly what makes `names_a_process_colorant` false
# while there is still real ink to paint.
SPOT_ONLY_TINT = b"{ 0 0 }"


def overprint_spot_only() -> bytes:
    """A two-spot `/DeviceN` axial shading over white paper, `/OP true`.

    No process colorant is NAMED, so the native-ink route must refuse and let
    the bridge paint. The page needs a `/Group` with `/CS /DeviceCMYK` for the
    same reason the fixture above does: without a subtractive blending space
    pdfce opens no colorant buffer, Table 149 never runs, and the fixture
    asserts nothing while looking fine.
    """
    content = b"q /GS0 gs /Sh0 sh Q\n"
    return assemble([
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] "
        b"/Group << /S /Transparency /CS /DeviceCMYK /I false /K false >> "
        b"/Resources << /ExtGState << /GS0 5 0 R >> /Shading << /Sh0 6 0 R >> >> "
        b"/Contents 4 0 R >>",
        stream(b"", content),
        b"<< /Type /ExtGState /OP true /op true /OPM 1 >>",
        b"<< /ShadingType 2 /ColorSpace 7 0 R /Coords [20 0 180 0] "
        b"/Function 8 0 R /Extend [true true] >>",
        b"[ /DeviceN [/SpotAlpha /SpotBeta] /DeviceCMYK 9 0 R ]",
        # The SHADING's function: t -> (s1, s2), a real ramp between the two.
        b"<< /FunctionType 2 /Domain [0 1] /C0 [1 0] /C1 [0 1] /N 1 >>",
        # The COLOUR SPACE's tint transform: (s1, s2) -> CMYK.
        stream(b"/FunctionType 4 /Domain [0 1 0 1] /Range [0 1 0 1 0 1 0 1]",
               SPOT_ONLY_TINT),
    ])


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    for name, data in {
        "shading-vs-fill-cmyk.pdf": build(subtractive=True),
        "shading-vs-fill-rgb.pdf": build(subtractive=False),
        "shading-overprint-mixed-spot-keeps-k.pdf": overprint_mixed(True),
        # ★ The control is not optional: with overprint OFF the source correctly
        # DOES replace the backdrop, so K goes to 0. Without it, "the band is
        # dark" is equally satisfied by a build where the shading paints nothing
        # -- which is what the spot-only refusal produces, one branch away in
        # the same function.
        "shading-overprint-off-control.pdf": overprint_mixed(False),
        "shading-overprint-spot-only.pdf": overprint_spot_only(),
    }.items():
        (OUT / name).write_bytes(data)
        print(f"wrote {OUT / name}  ({len(data)} bytes)")


if __name__ == "__main__":
    main()
