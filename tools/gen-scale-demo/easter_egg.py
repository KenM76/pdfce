"""A heart drawn out of mitochondria, inside the banana's pulp cell.

DOES IT FIT? — the arithmetic, before any of it was drawn
========================================================
The pulp cell is 300 x 300 um. A mitochondrion is 1.5 x 0.7 um. So the
cell is 200 mitochondria wide, which is roughly the same ratio as a sheet
of A4 to a grain of rice — there is a lot of room.

    outer heart      140 x 125 um     ~ 190 mitochondria around its curve
    letter height    12 um            ~ 8 mitochondria per vertical stroke
    "KEN <3 EMILY"   9 glyphs, 81 um  ~ 210 mitochondria
    heart interior   ~110 um wide at the widest, so the line fits with room

The limit is not space, it is LEGIBILITY: a 12 um capital built from
1.5 um mitochondria has about eight of them per stroke, which is enough
to read and not enough to look smooth. Smaller letters would turn into
dotted lines.

WHERE IT LIVES
==============
In the cell's central vacuole, which is the only large clear region. The
starch grains were moved out to a ring around it — biologically harmless,
since grains cluster wherever they cluster, and it is the only structural
change the egg required.

VISIBLE FROM
============
The whole composition is ~150 um, so it appears at about 25 000 % — the
same zoom that makes the organelle labels readable. Individual
mitochondria in the strokes resolve at about 250 000 %.
"""

import math

import mitochondrion

UM = 1.0 / (25.4 / 72.0 * 1000.0)


def heart_points(cx, cy, w, h, n=220):
    """Points along the classic parametric heart, scaled into `w` x `h`."""
    pts = []
    for i in range(n):
        t = 2.0 * math.pi * i / n
        x = 16.0 * math.sin(t) ** 3
        y = (
            13.0 * math.cos(t)
            - 5.0 * math.cos(2 * t)
            - 2.0 * math.cos(3 * t)
            - math.cos(4 * t)
        )
        pts.append((cx + x * w / 32.0, cy + y * h / 30.0))
    pts.append(pts[0])
    return pts


# A stroke font: each glyph is a list of polylines in a 6 wide x 10 tall
# box. Capitals only — a stroke font's lower case needs curves, and curves
# built from 1.5 um beads stop reading as letters at this size.
FONT = {
    "K": [[(0, 0), (0, 10)], [(0, 5), (5, 10)], [(0, 5), (5, 0)]],
    "E": [[(5, 10), (0, 10), (0, 0), (5, 0)], [(0, 5), (4, 5)]],
    "N": [[(0, 0), (0, 10), (5, 0), (5, 10)]],
    "M": [[(0, 0), (0, 10), (2.5, 4), (5, 10), (5, 0)]],
    "I": [[(2.5, 0), (2.5, 10)], [(1, 10), (4, 10)], [(1, 0), (4, 0)]],
    "L": [[(0, 10), (0, 0), (5, 0)]],
    "Y": [[(0, 10), (2.5, 5), (5, 10)], [(2.5, 5), (2.5, 0)]],
    " ": [],
}
GLYPH_ADVANCE = 7.0  # in font units


def glyph_polylines(ch, x, y, size):
    """Polylines for one glyph, in micrometres, baseline at (x, y)."""
    s = size / 10.0
    return [[(x + px * s, y + py * s) for px, py in stroke] for stroke in FONT[ch]]


def string_polylines(text, x, y, size):
    out, cx = [], x
    for ch in text:
        if ch == "<":            # the little heart, written as "<3"
            continue
        if ch == "3":
            out.extend(
                [heart_points(cx + size * 0.30, y + size * 0.52,
                              size * 0.85, size * 0.85, 46)]
            )
            cx += GLYPH_ADVANCE * size / 10.0
            continue
        out.extend(glyph_polylines(ch, cx, y, size))
        cx += GLYPH_ADVANCE * size / 10.0
    return out, cx - x


def string_width(text, size):
    n = sum(0 if ch == "<" else 1 for ch in text)
    return n * GLYPH_ADVANCE * size / 10.0


def _resample(pts, spacing):
    """Even arc-length samples along a polyline, with a tangent angle."""
    out, carry = [], 0.0
    for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
        dx, dy = x1 - x0, y1 - y0
        seg = math.hypot(dx, dy)
        if seg <= 1e-12:
            continue
        ang = math.atan2(dy, dx)
        d = spacing - carry
        while d <= seg:
            out.append((x0 + dx * d / seg, y0 + dy * d / seg, ang))
            d += spacing
        carry = (carry + seg) % spacing
    return out


def draw_mito(d, cx, cy, rot, rx=0.75, ry=0.35, cristae=5, variant=0, flip=False):
    """Place one fully-detailed mitochondrion, centred at (cx, cy) in the
    cell's micrometre space.

    This used to draw the organelle inline as an ellipse plus one or two
    chords. It now places a shared Form XObject from `mitochondrion`, whose
    interior carries the real compartment topology -- two membranes, crista
    junctions, ATP synthase, ribosomes, a nucleoid. The beads in the heart
    and the letters get the SAME anatomy as the big ones in the cytoplasm;
    they are smaller organelles, not simpler ones.
    """
    px, py = d.p(cx, cy)
    d.op(mitochondrion.place(px, py, rot, rx * 1000.0, ry * 1000.0,
                             cristae, variant, flip))


def chain(d, polylines, spacing, **kw):
    """Bead mitochondria along polylines at even arc-length spacing.

    `variant`/`flip` cycle so consecutive beads are not identical. With
    three variants and a mirror that is six apparent organelles before the
    pattern repeats, which at this bead pitch is far enough apart that the
    eye reads tissue rather than a stamp.
    """
    n = 0
    for pl in polylines:
        for x, y, a in _resample(pl, spacing):
            draw_mito(d, x, y, a, variant=n % 3, flip=bool((n // 3) % 2), **kw)
            n += 1
    return n


def draw(d, cx=152.0, cy=158.0):
    """The whole easter egg. Returns a count of mitochondria drawn."""
    total = 0

    # the outer heart
    total += chain(d, [heart_points(cx, cy + 6, 140.0, 125.0, 260)], 2.3,
                   cristae=7)

    # KEN <3 EMILY, centred, 12 um capitals
    line = "KEN <3 EMILY"
    size = 12.0
    w = string_width(line, size)
    polys, _ = string_polylines(line, cx - w / 2.0, cy - size / 2.0 + 2.0, size)
    total += chain(d, polys, 1.75, rx=0.62, ry=0.30, cristae=6)

    # the anniversary line, in ordinary type below the heart. Ordinary
    # rather than beaded on purpose: at 7 um a mitochondria-built letter
    # would be four beads tall and stop being a letter.
    d.fill(0.42, 0.26, 0.30)
    for i, (s, sz) in enumerate((("HAPPY 7TH ANNIVERSARY", 7.0), ("2026", 7.0))):
        wpt = len(s) * sz * 0.56
        d.text(cx - wpt / 2.0, 74.0 - i * 9.5, sz, s)

    return total
