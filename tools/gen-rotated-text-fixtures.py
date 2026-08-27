#!/usr/bin/env python3
"""Byte-author the fixture `Pass 139.1`'s direction-aware segmentation needs.

WHY THIS EXISTS
---------------
Every derived-layout threshold in `text_extract::layout` was stated in
PAGE axes until `Pass 139.1`:

    |dy| > line_gap_ratio * size          -> a new line
    dx - advance < -backward_jump * size  -> a new line
    dx - advance >  word_gap * size       -> a word space

That is correct for text running along +x and **catastrophic for text
running any other way**, in two independent ways:

    90 / 270 deg   the whole advance lands in dy, so the baseline
                   "moves" at every glyph          -> the baseline clause
    180 deg        the step is in -x while `advance` is published as a
                   positive magnitude, so dx - advance ~= -2*advance
                                                       -> the jump clause

The result either way is ONE DERIVED LINE BREAK BETWEEN EVERY LETTER.
Measured on a real SOLIDWORKS drawing set whose title block stamps the
source path vertically: 82 glyphs, 72 runs, 71 derived breaks, for what
is one line of text. Copy-and-paste gave one character per line; Acrobat
gave one line.

★ THE TWO FAILURES GO THROUGH DIFFERENT CLAUSES, so a fix for one need
not fix the other. Both orientations are therefore in the fixture, and
so is a horizontal control -- without the control, a regression that
broke ordinary text while fixing rotated text would still pass.

``text/rotated-text.pdf``
    Four ``BT``...``ET`` blocks in one content stream, one page, a
    non-embedded standard-14 Helvetica:

    ======== ================== ==================================
    Block    ``Tm``             What it pins
    ======== ================== ==================================
    HORIZON  ``1 0 0 1``        The CONTROL. direction (1, 0); one
                                run, no derived breaks. A change
                                that "fixes" rotation by loosening
                                a threshold breaks this first.
    UPWARD   ``0 1 -1 0``       direction (0, 1). Fails the
                                BASELINE clause when measured in
                                page axes.
    INVERTED ``-1 0 0 -1``      direction (-1, 0). Fails the
                                BACKWARD-JUMP clause -- a different
                                code path from UPWARD.
    DOWNWARD ``0 -1 1 0``       direction (0, -1). The other
                                quarter turn, so a fix that
                                special-cased one sign is caught.
    ======== ================== ==================================

★ CAPITALS DELIBERATELY, and this is the whole reason the fixture is
worth authoring rather than reusing an existing one. At 12 pt Helvetica
every capital's advance exceeds ``line_gap_ratio * size = 0.30 * 12 =
3.6 pt``, so under the old page-axis rule NO RUN HOLDS TWO GLYPHS. A
fixture full of narrow lowercase letters hides the defect: ``i``, ``n``,
``c`` and the space are narrow enough that consecutive pairs survive,
and a reader looking at the output sees fragmentation rather than total
fragmentation and may conclude the segmentation is merely imperfect.

The consuming shell found this the expensive way: it built a workaround
that recovered the writing direction from the vector between a run's
first and last glyph -- sound reasoning, and useless, because on the
real file only 10 of the 72 runs held two glyphs at all. A vertical
label set in wide capitals, which is exactly what a title block carries,
yields no multi-glyph run whatsoever. The segmentation was so thorough
on rotated text that it destroyed the evidence needed to undo it.

WHAT THIS IS NOT
----------------
Not sec 9.7.4.3 VERTICAL WRITING MODE (``/WMode 1``), which is a
different feature with different metrics and is not implemented. This is
ordinary horizontal-mode text placed by a rotated text matrix -- what
every CAD exporter and every rotated word-processor text box emits.

PROVENANCE
----------
100% byte-authored here, no PDF library involved, so the fixture cannot
inherit a bug from the code it tests. Project rule 7 / ``LEGAL.md`` sec 5:
synthetic or rights-cleared only. Non-embedded Helvetica (sec 9.6.2.2
permits omitting ``/Widths`` and ``/FontDescriptor`` for the standard 14).

USAGE
-----
    python tools/gen-rotated-text-fixtures.py
"""

import sys
from pathlib import Path

OUT = Path("fixtures/synthetic/text")
PAGE_W, PAGE_H = 612, 792


def serialize(objects: dict[int, bytes]) -> bytes:
    """A classic sec 7.5.4 cross-reference table over a 1-based object map."""
    out = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
    offsets: dict[int, int] = {}
    highest = max(objects)
    for num in sorted(objects):
        body = objects[num]
        offsets[num] = len(out)
        out += f"{num} 0 obj\n".encode("ascii") + body + b"\nendobj\n"
    xref_at = len(out)
    out += f"xref\n0 {highest + 1}\n".encode("ascii")
    out += b"0000000000 65535 f \n"
    for num in range(1, highest + 1):
        out += (
            f"{offsets[num]:010d} 00000 n \n".encode("ascii")
            if num in offsets
            else b"0000000000 65535 f \n"
        )
    out += (
        f"trailer\n<< /Size {highest + 1} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    ).encode("ascii")
    return bytes(out)


def raw_stream(body: bytes) -> bytes:
    return f"<< /Length {len(body)} >>\nstream\n".encode("ascii") + body + b"\nendstream"


def page(content: bytes) -> bytes:
    objects: dict[int, bytes] = {
        1: b"<< /Type /Catalog /Pages 2 0 R >>",
        2: (
            f"<< /Type /Pages /Kids [3 0 R] /Count 1 "
            f"/MediaBox [0 0 {PAGE_W} {PAGE_H}] "
            f"/Resources << /Font << /F1 5 0 R >> >> >>"
        ).encode("ascii"),
        3: b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>",
        4: raw_stream(content),
        5: (
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica "
            b"/Encoding /WinAnsiEncoding >>"
        ),
    }
    return serialize(objects)


def rotated_text() -> bytes:
    """Four blocks: 0, 90, 180 and 270 degrees, all capitals.

    The four strings are DISTINCT and single-word so a test can assert
    which block it is looking at by decoding, and so a byte span can be
    checked against a literal that appears exactly once in the stream.

    Coordinates are chosen so each block runs well clear of the page
    edges in the direction it travels: UPWARD from y=300 rises about
    46 pt, DOWNWARD from y=700 falls about 60 pt, INVERTED from x=520
    travels about 62 pt back toward the origin.
    """
    return page(
        b"BT\n/F1 12 Tf\n"
        b"1 0 0 1 72 700 Tm\n(HORIZONTAL) Tj\n"    # dir (1, 0)   -- the control
        b"ET\n"
        b"BT\n/F1 12 Tf\n"
        b"0 1 -1 0 100 300 Tm\n(UPWARD) Tj\n"      # dir (0, 1)   -- baseline clause
        b"ET\n"
        b"BT\n/F1 12 Tf\n"
        b"-1 0 0 -1 520 200 Tm\n(INVERTED) Tj\n"   # dir (-1, 0)  -- jump clause
        b"ET\n"
        b"BT\n/F1 12 Tf\n"
        b"0 -1 1 0 300 700 Tm\n(DOWNWARD) Tj\n"    # dir (0, -1)  -- the other turn
        b"ET\n"
    )


def rotated_text_abutting() -> bytes:
    """A horizontal run whose END is exactly where a vertical run BEGINS.

    ★ THIS FIXTURE EXISTS BECAUSE ``rotated-text.pdf`` DOES NOT COVER THE
    DIRECTION-CHANGE RULE, and a sabotage run proved it.

    In ``rotated-text.pdf`` the four blocks sit far apart on the page, so
    the perpendicular-displacement clause fires between them regardless of
    what the direction rule does. Deleting the rule entirely left all nine
    of that fixture's tests green. The tests EXERCISED the rule and did not
    COVER it -- the failure mode this project has now found repeatedly.

    Here the two runs abut. ``AB`` is set at 12 pt Helvetica from
    ``(200, 500)``; both capitals are 667/1000 em, so the pen finishes at
    ``200 + 2 * 0.667 * 12 = 216.008``. The vertical run starts at exactly
    that point. So between the ``B`` and the ``C``:

        perp  = 0     (no baseline displacement at all)
        along = 0     (no gap at all)

    Every geometric clause says ``Break::None`` and the two runs MERGE --
    into one run holding glyphs that do not share a direction, which is
    precisely the guarantee ``TextRun::direction()`` is published on.

    Only the direction rule separates them. Delete it and this fixture's
    test fails; that is the whole job of the file.
    """
    return page(
        b"BT\n/F1 12 Tf\n"
        b"1 0 0 1 200 500 Tm\n(AB) Tj\n"           # dir (1, 0), ends at x=216.008
        b"ET\n"
        b"BT\n/F1 12 Tf\n"
        b"0 1 -1 0 216.008 500 Tm\n(CD) Tj\n"      # dir (0, 1), starts THERE
        b"ET\n"
    )


def rotated_text_columns() -> bytes:
    """A vertical WORD GAP and a vertical LINE BREAK, both invisible to a
    page-axis reader.

    ★ THIS FIXTURE EXISTS BECAUSE THE OTHER TWO DO NOT COVER THE
    ``perp``/``gap`` RESOLUTION ITSELF, and a sabotage run proved it.

    ``rotated-text.pdf`` and ``rotated-text-abutting.pdf`` between them
    cover the frame-aware CURSOR (the end point advanced along the writing
    direction) and the DIRECTION-CHANGE rule. Neither covers the two dot
    products, because in both of those fixtures the glyphs abut exactly --
    ``dy`` is zero within a run, so the correct ``perp = d x dir`` and the
    old page-axis ``-dy`` both come out zero and agree.

    They only disagree when there is a real gap. Two cases, both here, both
    at 12 pt Helvetica (``word_gap 0.20 -> 2.4 pt``,
    ``line_gap 0.30 -> 3.6 pt``):

    ``UP`` at (100, 100), running up
        U=722 + P=667 = 1389/1000 em = 16.668 pt, so the pen ends at
        y = 116.668.

    ``ON`` at (100, 120) -- a WORD GAP
        The gap ALONG the writing direction is 3.332 pt, over the 2.4 pt
        word threshold and under the 3.6 pt line threshold: a derived word
        space. A page-axis reader computes ``gap = dx = 0`` and inserts
        NOTHING, silently running the two words together. It also computes
        ``perp = -dy = -3.332``, which is under its own line threshold, so
        it does not even break -- the failure is a missing space, not a
        visible fragmentation.
        O=778 + N=722 = 1500/1000 em = 18 pt, so the pen ends at y = 138.

    ``SIDE`` at (130, 138) -- a LINE BREAK
        A SECOND COLUMN, 30 pt to the right, deliberately aligned so that
        ``dy = 0`` exactly. The correct ``perp = dx = 30`` is a baseline
        displacement eight times the line threshold: a new line. A
        page-axis reader computes ``perp = -dy = 0`` -- no baseline change
        at all -- and then ``gap = dx = 30``, which it reads as a WORD
        SPACE. So it turns a second column into a continuation of the
        first, which is exactly the two-column failure the backward-jump
        clause exists to prevent in horizontal text.

    Expected: ``UP ON\\nSIDE`` -- one derived space, one derived break.
    A page-axis reader gives ``UPON SIDE`` -- one derived space in the
    wrong place, no break, and two columns run together.

    ★ Note that both wrong answers are QUIET. Neither produces the
    one-letter-per-line fragmentation that made the original defect
    visible; a reader would have to know what the file said to notice.
    """
    return page(
        b"BT\n/F1 12 Tf\n"
        b"0 1 -1 0 100 100 Tm\n(UP) Tj\n"      # ends at y = 116.668
        b"ET\n"
        b"BT\n/F1 12 Tf\n"
        b"0 1 -1 0 100 120 Tm\n(ON) Tj\n"      # gap 3.332 along dir -> WORD
        b"ET\n"
        b"BT\n/F1 12 Tf\n"
        b"0 1 -1 0 130 138 Tm\n(SIDE) Tj\n"    # perp 30, dy 0 -> LINE
        b"ET\n"
    )


def main() -> int:
    out_dir = Path(sys.argv[1]) if len(sys.argv) > 1 else OUT
    out_dir.mkdir(parents=True, exist_ok=True)
    for name, data in (
        ("rotated-text.pdf", rotated_text()),
        ("rotated-text-abutting.pdf", rotated_text_abutting()),
        ("rotated-text-columns.pdf", rotated_text_columns()),
    ):
        p = out_dir / name
        p.write_bytes(data)
        print(f"wrote {p} ({len(data)} bytes)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
