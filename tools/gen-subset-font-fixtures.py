#!/usr/bin/env python3
"""Generate the embedded-SUBSET simple-font fixture (Pass 21.x / FF-C).

WHY THIS FIXTURE EXISTS
=======================
Two shipped refusals are reachable only with an embedded **subset** font:

  * **R-INV-1**, the embedded-subset floor — "this run's font is a subset
    and does not carry the character you just typed."
  * **`FormatError::CoverageFailure`** raised by `format.rs`'s subset branch,
    which refuses when a re-encoded code is not already carried on the page.

`fixtures/synthetic/` had three files and none of them could fire either.
That gap was not noticed until 2026-08-03, when two operator-facing hints for
these exact refusals turned out to have been telling the operator to do
something that could not work (`0893191`) — and the fix could not be observed
in the running app, because nothing in the corpus could reach the message.

A refusal no fixture can trigger is a refusal whose wording nobody has ever
read on screen. This closes that.

It is also a prerequisite for Pass 21.0 (decision 021): FF-C's whole purpose
is to turn these refusals into an actionable remedy, and "the refusal still
fires when it should" is half of that Pass's acceptance criteria.

WHY A *SIMPLE* FONT AND NOT A CIDFont
=====================================
`tools/gen-cidfont-nocmap-fixtures.py` already emits an embedded subset
`CIDFontType2`. It cannot reach these refusals, because a composite run is
refused *earlier* by **R-INV-4** (`/Type0` runs are not character-editable at
all) — so the subset branch is never consulted. The refusals being targeted
live on the **simple**-font path, so the fixture has to be a simple font.

That distinction is the entire reason this file exists as a sibling rather
than a flag on the existing generator.

WHAT MAKES IT A SUBSET, MECHANICALLY
====================================
`pdfce-core`'s `is_subset_tag` (`text_edit/edit.rs`) decides subset-ness
purely from the `/BaseFont` name: exactly six ASCII **uppercase** letters
followed by `+`. That is ISO 32000-1 §9.6.4's subset prefix. The font is
*also* genuinely reduced — it carries outlines for only the characters shown
— so the fixture is not merely lying in its name. Both halves matter: the tag
is what the code branches on, the real reduction is what makes the fixture
honest if the detection rule ever changes.

**Verify-don't-assume (R22):** the builder asserts the sfnt magic, the
presence of `glyf`, and that the shipped `/BaseFont` actually satisfies the
same six-uppercase-plus-`+` predicate `is_subset_tag` applies. A fixture that
silently stops being a subset would make every test built on it pass for the
wrong reason.

LICENSING
=========
Fully SYNTHETIC (`docs/LEGAL.md` §5). Every outline is drawn here by
`fontTools`; no byte is copied from any real-world font or document. This
matters more than usual for a *font* fixture — embedding someone else's face
is exactly the redistribution question decision 021's R109 is about, and a
test corpus is not the place to have it.

Usage:
    python tools/gen-subset-font-fixtures.py [OUT_DIR]
        OUT_DIR defaults to fixtures/synthetic/text/.
"""

from __future__ import annotations

import io
import sys
from pathlib import Path

PAGE_WIDTH = 612
PAGE_HEIGHT = 792

OUT = Path(__file__).resolve().parent.parent / "fixtures" / "synthetic" / "text"

# The subset prefix. Six ASCII uppercase letters + '+', per ISO 32000-1
# §9.6.4 and exactly what `is_subset_tag` tests for.
SUBSET_TAG = "SUBSET"
FAMILY = "pdfceSubsetDemo"
BASE_FONT = f"{SUBSET_TAG}+{FAMILY}"

# The characters the subset CARRIES. Deliberately a small, contiguous,
# obviously-incomplete set: an operator looking at the page can see at a
# glance which letters exist, which makes a refusal about any other letter
# self-explanatory rather than mysterious.
CARRIED = "ABC"
# Where the first absent character sits, for whoever writes the test:
# anything outside CARRIED. 'Z' is used in the doc comments because it is
# unambiguous and adjacent in nobody's mental model to A/B/C.

UPEM = 1000
ADVANCE = 600


def build_subset_truetype() -> bytes:
    """A minimal TrueType carrying outlines for `CARRIED` and nothing else.

    Each carried character gets a distinguishable outline — a bar whose
    height varies by index — so a rendering test can tell the glyphs apart
    without OCR, and so a wrong-glyph bug looks different from a
    missing-glyph bug.
    """
    from fontTools.fontBuilder import FontBuilder
    from fontTools.pens.ttGlyphPen import TTGlyphPen

    glyph_names = [".notdef"] + [f"g{ch}" for ch in CARRIED]

    fb = FontBuilder(UPEM, isTTF=True)
    fb.setupGlyphOrder(glyph_names)

    glyphs = {}
    notdef_pen = TTGlyphPen(None)
    glyphs[".notdef"] = notdef_pen.glyph()  # empty outline

    for i, ch in enumerate(CARRIED):
        pen = TTGlyphPen(None)
        # Height rises with index: A short, B taller, C tallest. Distinct
        # on sight, and trivially assertable from a rasterized bitmap.
        top = 300 + 200 * i
        pen.moveTo((80, 0))
        pen.lineTo((ADVANCE - 80, 0))
        pen.lineTo((ADVANCE - 80, top))
        pen.lineTo((80, top))
        pen.closePath()
        glyphs[f"g{ch}"] = pen.glyph()

    fb.setupGlyf(glyphs)
    fb.setupHorizontalMetrics(
        {name: (ADVANCE, 80) for name in glyph_names}
    )
    fb.setupHorizontalHeader(ascent=800, descent=-200)
    fb.setupNameTable({"familyName": FAMILY, "styleName": "Regular"})
    # A cmap covering ONLY the carried characters. This is what makes the
    # font a real subset rather than one that merely claims to be: a
    # consumer asking for 'Z' finds nothing, which is the truth the
    # /BaseFont tag is asserting.
    fb.setupCharacterMap({ord(ch): f"g{ch}" for ch in CARRIED})
    fb.setupOS2(sTypoAscender=800, sTypoDescender=-200)
    fb.setupPost()

    buf = io.BytesIO()
    fb.font.save(buf)
    data = buf.getvalue()

    # Verify-don't-assume (R22). A fixture that quietly stopped carrying a
    # glyf table, or quietly grew coverage for the absent characters, would
    # make every test built on it pass for the wrong reason.
    assert data[:4] == b"\x00\x01\x00\x00", f"sfnt magic was {data[:4]!r}"
    directory = data[: 12 + 16 * int.from_bytes(data[4:6], "big")]
    assert b"glyf" in directory, "no glyf table in the built font"
    cmap = fb.font["cmap"].getBestCmap()
    assert set(cmap) == {ord(c) for c in CARRIED}, (
        f"cmap coverage drifted: {sorted(cmap)} != {sorted(ord(c) for c in CARRIED)}"
    )
    return data


def serialize(objects: dict[int, bytes]) -> bytes:
    """Classic xref layout, exactly-20-byte entries (§7.5.4). Identical
    discipline to the sibling generators."""
    out = bytearray(b"%PDF-1.7\n")
    out += b"%\xe2\xe3\xcf\xd3\n"
    highest = max(objects)
    offsets: dict[int, int] = {}
    for num in range(1, highest + 1):
        body = objects.get(num)
        if body is None:
            continue
        offsets[num] = len(out)
        out += f"{num} 0 obj\n".encode("ascii")
        out += body
        out += b"\nendobj\n"
    xref_at = len(out)
    out += f"xref\n0 {highest + 1}\n".encode("ascii")
    out += b"0000000000 65535 f \n"
    for num in range(1, highest + 1):
        if num in offsets:
            out += f"{offsets[num]:010d} 00000 n \n".encode("ascii")
        else:
            out += b"0000000000 65535 f \n"
    out += (
        f"trailer\n<< /Size {highest + 1} /Root 1 0 R >>\n"
        f"startxref\n{xref_at}\n%%EOF\n"
    ).encode("ascii")
    return bytes(out)


def raw_stream(body: bytes, extra: str) -> bytes:
    """Uncompressed stream with a correct `/Length` (plus caller-supplied
    keys such as `/Length1`). Uncompressed so a failure can never be blamed
    on the filter."""
    return (
        f"<< /Length {len(body)}{extra} >>\nstream\n".encode("ascii")
        + body
        + b"\nendstream"
    )


def subset_simple_embedded() -> bytes:
    """`/TrueType` simple font, embedded SUBSET program, carrying only `CARRIED`.

    `/FirstChar`..`/LastChar` and `/Widths` span exactly the carried
    characters, so the width array agrees with the program's real coverage.
    Disagreement there is its own class of real-world bug and is NOT what
    this fixture is for — a fixture that tests two things at once tells you
    nothing when it fails.
    """
    ttf = build_subset_truetype()

    first = ord(CARRIED[0])
    last = ord(CARRIED[-1])
    widths = " ".join(str(ADVANCE) for _ in CARRIED)

    content = (
        b"BT\n"
        b"/F0 48 Tf\n"
        b"72 600 Td\n"
        b"(" + CARRIED.encode("ascii") + b") Tj\n"
        b"ET\n"
    )

    objects: dict[int, bytes] = {
        1: b"<< /Type /Catalog /Pages 2 0 R >>",
        2: (
            f"<< /Type /Pages /Kids [3 0 R] /Count 1 "
            f"/MediaBox [0 0 {PAGE_WIDTH} {PAGE_HEIGHT}] "
            f"/Resources << /Font << /F0 5 0 R >> >> >>"
        ).encode("ascii"),
        3: b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>",
        4: raw_stream(content, ""),
        5: (
            f"<< /Type /Font /Subtype /TrueType /BaseFont /{BASE_FONT} "
            f"/FirstChar {first} /LastChar {last} /Widths [{widths}] "
            f"/Encoding /WinAnsiEncoding /FontDescriptor 6 0 R >>"
        ).encode("ascii"),
        6: (
            f"<< /Type /FontDescriptor /FontName /{BASE_FONT} "
            f"/Flags 32 /FontBBox [0 -200 {ADVANCE} 800] /ItalicAngle 0 "
            f"/Ascent 800 /Descent -200 /CapHeight 700 /StemV 80 "
            f"/FontFile2 7 0 R >>"
        ).encode("ascii"),
        7: raw_stream(ttf, f" /Length1 {len(ttf)}"),
    }
    return serialize(objects)


def main() -> int:
    out_dir = Path(sys.argv[1]) if len(sys.argv) > 1 else OUT
    out_dir.mkdir(parents=True, exist_ok=True)

    # The shipped /BaseFont must satisfy the SAME predicate pdfce-core
    # applies (`is_subset_tag`), or the fixture is not testing the branch
    # it names. Reproduced here rather than assumed, because the fixture
    # and the code that classifies it live in different languages and
    # nothing else would notice them drifting apart.
    tag, _, rest = BASE_FONT.partition("+")
    assert len(tag) == 6 and tag.isascii() and tag.isupper() and rest, (
        f"/BaseFont {BASE_FONT!r} does not satisfy is_subset_tag's rule "
        "(exactly six ASCII uppercase letters, then '+', then a name)"
    )

    path = out_dir / "subset-simple-embedded.pdf"
    path.write_bytes(subset_simple_embedded())
    print(f"wrote {path} ({path.stat().st_size} bytes)")
    print(f"  /BaseFont  {BASE_FONT}   (subset tag: {tag})")
    print(f"  carries    {CARRIED!r}")
    print(f"  absent     any other character — 'Z' is the canonical probe")
    return 0


if __name__ == "__main__":
    sys.exit(main())
