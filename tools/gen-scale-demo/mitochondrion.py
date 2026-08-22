"""One mitochondrion, drawn to its real internal anatomy, in NANOMETRES.

WHY THIS FILE EXISTS
====================
Every mitochondrion on the banana page used to be an ellipse with one or
three straight lines ruled across it. That reads correctly at the zoom
where a mitochondrion is a few pixels wide and is a lie at every zoom
below it: cristae are not chords, they are invaginations OF the inner
membrane, joined to it through narrow necks, and the space inside a
crista is continuous with the space between the two membranes rather
than with the matrix. Drawing them as chords gets the topology backwards,
which is the one thing a section view is for.

It exists as a separate module because all four populations of
mitochondria on the page -- the 14 in the pulp cytoplasm, the 3 in the
skin cell, and the 325 beaded into the easter egg heart and letters --
now share it. Before this module they were three independent copy-pastes
that had drifted: the pulp cell drew three cristae, the easter egg drew
one or two, and the skin cell drew NONE.

UNITS, and why they are nanometres
==================================
The features here span three decades:

    outer membrane                  5 nm
    inner membrane                4.5 nm
    ATP synthase F1 head           10 nm
    intermembrane space            24 nm
    crista junction (the neck)     18 nm
    crista lumen                   26 nm
    mitoribosome                   26 nm
    matrix granule                 36 nm
    mtDNA nucleoid                210 nm
    whole organelle         1240-3000 nm

The numbers live in the constants below, not here; this table is the
shape of the problem, and the constants are the answer.

A 10 nm feature is 2.8e-5 pt. Authoring that in absolute page points
would put six significant figures of the number into the page offset and
leave two for the feature, so this module authors in NANOMETRES at the
origin and lets the placement matrix (`instance_matrix`) carry the
position. That is also why every instance is a Form XObject: 342 copies
of ~1400 path operators inline would be megabytes of content stream, and
as a handful of shared forms it is a fraction of that.

THE ANATOMY, outside in, and the confidence in each figure
==========================================================
Sizes are the standard textbook/EM ranges for plant mitochondria; they
are representative, not measurements of a banana.

1.  OUTER MEMBRANE -- smooth, unfolded, a plain closed ellipse. ~6-7 nm.
2.  INTERMEMBRANE SPACE -- ~10-20 nm in life. Drawn 24 nm, which is the
    one figure here knowingly stretched past its range: at 18 nm the two
    membrane strokes very nearly meet and the compartment stops reading
    as a compartment, which defeats the point of drawing it. Continuous
    with every crista lumen; that continuity is the topological fact this
    whole drawing exists to show.
3.  INNER BOUNDARY MEMBRANE -- the part of the inner membrane that runs
    parallel to the outer one. Protein-poor compared to the cristae.
4.  CRISTAE -- lamellar (plate-like) folds of the inner membrane pushing
    into the matrix. Plant mitochondria are typically lamellar rather
    than tubular, so they appear in section as flattened slots. Each
    connects to the boundary membrane through a
5.  CRISTA JUNCTION -- a narrow neck, ~20-25 nm, which is what makes the
    crista lumen a separate compartment rather than just "more IMS". The
    junctions are the reason cytochrome c can be sequestered. Drawn 18 nm
    against a 26 nm lumen: in life the two are nearly equal, but a neck
    that is not narrower than what it opens into does not READ as a neck,
    and the constriction is the entire anatomical point of the feature.
6.  ATP SYNTHASE (complex V) -- the "elementary particles" of the classic
    EM literature: a ~10 nm F1 head on a short stalk, projecting into the
    MATRIX from the inner membrane. Dimer rows concentrate along the
    highly curved crista RIMS, which is what bends the rim in the first
    place, and are sparse on the flat boundary membrane. This module
    honours that gradient: dense on crista faces and tips, sparse on the
    boundary.
7.  MATRIX -- the enclosed space, drawn in the same colour the whole
    organelle used to be, so that at the zoom where this is still a
    two-pixel blob the page looks exactly as it did before.
8.  MITORIBOSOMES -- ~25-30 nm, free in the matrix. Plant mitoribosomes
    are large; 26 nm is used.
9.  mtDNA NUCLEOID -- a protein-packaged loop of the mitochondrial
    genome, ~100-200 nm as a fibrous patch. Plant mtDNA is much larger
    than animal mtDNA and there are usually several nucleoids per
    organelle; one or two are drawn.
10. MATRIX GRANULES -- dense calcium/phosphate deposits, ~30-50 nm.

WHAT IS DELIBERATELY NOT DRAWN
==============================
Individual bilayer leaflets (a membrane is one stroke, not two), the
respiratory complexes I-IV (they do not project far enough to have a
silhouette at this scale), and cristae in true 3-D -- these are sections,
so a lamella is a slot, not a plate.
"""

import math

# Points per nanometre.
#
#   1 pt = 25.4/72 mm = 0.352 777 8 mm = 352 777.8 nm
#   so 1 nm = 1 / 352 777.8 pt = 2.834 645 7e-6 pt
#
# Written as a reciprocal deliberately. The first draft of this line was
# `25.4 / 72.0 / 1_000_000.0`, which is millimetres-per-point scaled down
# rather than points-per-nanometre, and is wrong by a factor of 8.03. It
# did not fail loudly: every mitochondrion simply came out an eighth of
# its size, still in the right place and still correctly shaped, which at
# a glance looks like a drawing choice rather than a unit error. The sibling
# constant `UM` in `cells_detail` is already written in this reciprocal
# form; matching it is the cheapest guard against making the mistake again.
PT_PER_NM = 1.0 / (25.4 / 72.0 * 1_000_000.0)

# --- the anatomy, in nanometres -------------------------------------------
OM_THICK = 5.0        # outer membrane stroke
IM_THICK = 4.5        # inner membrane stroke
IMS = 24.0            # intermembrane space, outer -> inner boundary membrane
LUMEN_HALF = 13.0     # half the intracristal space, so a 26 nm lumen
JUNCTION = 18.0       # crista junction opening, along the boundary
NECK_DEPTH = 26.0     # how far in the neck runs before the lamella proper
F1_R = 5.0            # ATP synthase F1 head radius -> 10 nm particle
F1_STALK = 3.5        # stalk from membrane surface to the head
F1_PITCH = 11.5       # spacing of F1 particles along a crista face
F1_PITCH_IBM = 62.0   # ...and along the boundary membrane, much sparser
RIBO_R = 13.0         # mitoribosome
GRANULE_R = 18.0      # matrix granule
NUCLEOID_R = 105.0    # mtDNA nucleoid patch

# --- colours ---------------------------------------------------------------
# The matrix keeps the exact fill the whole organelle used to have, so
# nothing about the page changes at the zoom where this is a blob.
C_MATRIX = (0.72, 0.55, 0.50)
C_IMS = (0.94, 0.86, 0.83)
C_OM = (0.44, 0.27, 0.24)
C_IM = (0.50, 0.32, 0.29)
C_F1 = (0.40, 0.24, 0.22)
C_RIBO = (0.35, 0.21, 0.19)
C_GRANULE = (0.26, 0.15, 0.14)
C_NUCLEOID = (0.93, 0.87, 0.74)
C_DNA = (0.58, 0.44, 0.32)


class _Rng:
    """A tiny deterministic LCG.

    `random` is avoided on purpose: this file must produce byte-identical
    output on every machine and every Python version, because the PDF it
    feeds is diffed against previous runs to check the writer, and a
    seeded `random.Random` is only guaranteed stable within one Python
    release line.
    """

    def __init__(self, seed):
        self.s = (seed * 1103515245 + 12345) & 0x7FFFFFFF

    def next(self):
        self.s = (self.s * 1103515245 + 12345) & 0x7FFFFFFF
        return self.s / float(0x7FFFFFFF)

    def uniform(self, lo, hi):
        return lo + (hi - lo) * self.next()


# ---------------------------------------------------------------------------
# Path helpers, all in nanometres about the origin
# ---------------------------------------------------------------------------

_K = 0.5522847498  # kappa: cubic control-point offset for a quarter circle


def _ellipse(o, cx, cy, rx, ry):
    """Append a kappa-approximated ellipse as four cubics."""
    pts = [
        (-rx, 0), (-rx, ry * _K), (-rx * _K, ry), (0, ry),
        (rx * _K, ry), (rx, ry * _K), (rx, 0),
        (rx, -ry * _K), (rx * _K, -ry), (0, -ry),
        (-rx * _K, -ry), (-rx, -ry * _K), (-rx, 0),
    ]
    p = [(cx + x, cy + y) for x, y in pts]
    o.append(f"{p[0][0]:.3f} {p[0][1]:.3f} m")
    for i in range(1, 13, 3):
        o.append(
            f"{p[i][0]:.3f} {p[i][1]:.3f} {p[i+1][0]:.3f} {p[i+1][1]:.3f} "
            f"{p[i+2][0]:.3f} {p[i+2][1]:.3f} c"
        )
    o.append("h")


def _polyline(o, pts, close=True):
    o.append(f"{pts[0][0]:.3f} {pts[0][1]:.3f} m")
    for x, y in pts[1:]:
        o.append(f"{x:.3f} {y:.3f} l")
    if close:
        o.append("h")


def _boundary_point(a, b, th):
    return (a * math.cos(th), b * math.sin(th))


def _boundary_normal(a, b, th):
    """Inward unit normal of the ellipse at parameter `th`.

    The outward normal of x^2/a^2 + y^2/b^2 = 1 at (a cos t, b sin t) is
    parallel to (b cos t, a sin t) -- note the axes SWAP, which is the
    step it is easy to get wrong and which makes cristae lean the wrong
    way on an eccentric organelle.
    """
    nx, ny = b * math.cos(th), a * math.sin(th)
    n = math.hypot(nx, ny) or 1.0
    return (-nx / n, -ny / n)


# ---------------------------------------------------------------------------
# Crista placement
# ---------------------------------------------------------------------------

def _crista_sites(a, b, n, rng):
    """Alternating top/bottom crista positions, staggered along the long axis.

    Returned as (theta, half_angular_width, depth). Cristae are placed by
    their x position rather than by angle so they stay evenly spread on an
    elongated organelle, where equal angular steps would bunch them at the
    ends.
    """
    sites = []
    if n <= 0:
        return sites
    for i in range(n):
        # spread across the middle 84% of the long axis
        f = (i + 0.5) / n * 2.0 - 1.0
        x = f * a * 0.84
        x += rng.uniform(-0.035, 0.035) * a
        x = max(-a * 0.93, min(a * 0.93, x))
        top = (i % 2) == 0
        th = math.acos(max(-1.0, min(1.0, x / a)))
        if not top:
            th = -th
        # the junction subtends this much of the boundary
        speed = math.hypot(a * math.sin(th), b * math.cos(th)) or 1.0
        half_w = (JUNCTION * 0.5) / speed
        depth = b * rng.uniform(0.52, 0.78)
        sites.append((th % (2 * math.pi), half_w, depth))
    sites.sort()
    return sites


def _crista_excursion(th, half_w, depth, a, b):
    """The inner membrane path from one lip of a crista junction, in around
    the lamella, and back out to the other lip.

    Returns (points, faces, tip) where `faces` says where ATP synthase
    particles go along the two flat faces, and `tip` describes the curved
    rim at the blind end.
    """
    e1 = _boundary_point(a, b, th - half_w)
    e2 = _boundary_point(a, b, th + half_w)
    m = _boundary_point(a, b, th)
    ux, uy = _boundary_normal(a, b, th)          # inward
    vx, vy = -uy, ux                             # tangent

    t = LUMEN_HALF

    def at(s, lat):
        """(depth inward, lateral along the boundary) -> a point."""
        return (m[0] + ux * s + vx * lat, m[1] + uy * s + vy * lat)

    # WHICH LATERAL SIDE e1 IS ON, and why getting this wrong is invisible
    # in the geometry and glaring in the fill.
    #
    # `v` is the tangent obtained by rotating the inward normal, and for
    # this parametrisation INCREASING theta runs in the -v direction. So
    # the lip at `th - half_w` sits on the +v side and the lip at
    # `th + half_w` on the -v side. The first draft dived in on the -v
    # side first, i.e. it left e1 (+v), crossed the mouth of its own
    # junction, ran down the far wall and came back to e2 (-v) crossing
    # again -- a bow tie.
    #
    # A self-crossing costs nothing structurally: every membrane is still
    # the right shape and the stroke looks perfect. What it destroys is
    # the FILL. `B` fills by nonzero winding, and the two crossings wind
    # the lumen the same way as the matrix, so every crista filled solid
    # with matrix colour -- which is exactly the compartment error this
    # whole module exists to correct, reintroduced one layer down and
    # much harder to see. Verified by rendering one crista at 160 000 000
    # %; at anything less the lumen is too narrow to read a colour from.
    #
    # Hence `side`: +1 puts the first wall on e1's own side, so the path
    # is a fjord (a simple closed curve with a deep narrow inlet) and the
    # inlet is unambiguously OUTSIDE it under any fill rule.
    side = 1.0

    tip = depth
    pts = [e1]
    pts.append(at(NECK_DEPTH, side * t))
    pts.append(at(tip - t, side * t))
    # The blind end: a semicircle of radius t centred at depth (tip - t),
    # swept from lateral +side*t round to -side*t, so its apex is the
    # deepest point of the crista at exactly `tip`.
    for k in range(1, 6):
        ang = math.pi * k / 6.0
        pts.append(at(tip - t + math.sin(ang) * t, side * math.cos(ang) * t))
    pts.append(at(tip - t, -side * t))
    pts.append(at(NECK_DEPTH, -side * t))
    pts.append(e2)

    # The two flat faces, with the direction the ATP synthase heads point:
    # AWAY from the lumen, into the matrix. That is the outward normal of
    # each wall, which is why the two entries have opposite lateral signs.
    face_len = max(0.0, (tip - t) - NECK_DEPTH)
    faces = [
        (at(NECK_DEPTH, side * t), (ux, uy), (side * vx, side * vy), face_len),
        (at(NECK_DEPTH, -side * t), (ux, uy), (-side * vx, -side * vy), face_len),
    ]
    return pts, faces, (at(tip - t, 0.0), (ux, uy), t)


def _inner_membrane(a, b, sites, steps=180):
    """The whole inner membrane as ONE closed polyline: boundary arcs and
    crista excursions in sequence.

    This is the heart of the module. Because it is one path, filling it
    with the matrix colour automatically leaves each crista lumen unfilled
    -- the slot shows the intermembrane colour behind it -- and that is
    exactly the compartment topology, obtained for free instead of by
    drawing the lumen as a separate object that could drift out of
    register with its own membrane.
    """
    pts = []
    faces = []
    tips = []
    if not sites:
        for i in range(steps):
            pts.append(_boundary_point(a, b, 2 * math.pi * i / steps))
        return pts, faces, tips

    def arc(t0, t1):
        span = t1 - t0
        n = max(2, int(abs(span) / (2 * math.pi) * steps))
        for i in range(n + 1):
            pts.append(_boundary_point(a, b, t0 + span * i / n))

    prev = sites[-1][0] + sites[-1][1] - 2 * math.pi
    for th, hw, depth in sites:
        arc(prev, th - hw)
        ex, fc, tp = _crista_excursion(th, hw, depth, a, b)
        pts.extend(ex)
        faces.extend(fc)
        tips.append(tp)
        prev = th + hw
    return pts, faces, tips


# ---------------------------------------------------------------------------
# ATP synthase
# ---------------------------------------------------------------------------

def _f1_particles(faces, tips, a, b, sites):
    """Positions of the F1 heads, with the stalk foot each one sits on.

    Density follows the real gradient: tightly packed along the crista
    faces and around the curved rim, sparse on the flat boundary membrane.
    """
    out = []
    off = F1_STALK + F1_R

    for (ox, oy), (ux, uy), (wx, wy), length in faces:
        n = int(length / F1_PITCH)
        for i in range(n):
            s = (i + 0.5) * F1_PITCH
            fx, fy = ox + ux * s, oy + uy * s
            out.append(((fx, fy), (fx + wx * off, fy + wy * off)))

    # the rim: the tightly curved blind end, where dimer rows sit
    for (tx, ty), (ux, uy), t in tips:
        vx, vy = -uy, ux
        for k in range(5):
            ang = -math.pi / 2 + math.pi * (k + 0.5) / 5.0
            rx, ry = math.sin(ang), math.cos(ang)
            dx, dy = ux * rx + vx * ry, uy * rx + vy * ry
            fx, fy = tx + dx * t, ty + dy * t
            out.append(((fx, fy), (fx + dx * off, fy + dy * off)))

    # the boundary membrane, sparse, and never inside a junction
    circ = math.pi * (3 * (a + b) - math.sqrt((3 * a + b) * (a + 3 * b)))
    n = max(4, int(circ / F1_PITCH_IBM))
    for i in range(n):
        th = 2 * math.pi * (i + 0.5) / n
        if any(abs(((th - s[0] + math.pi) % (2 * math.pi)) - math.pi) < s[1] * 2.4
               for s in sites):
            continue
        px, py = _boundary_point(a, b, th)
        ux, uy = _boundary_normal(a, b, th)
        out.append(((px, py), (px + ux * off, py + uy * off)))
    return out


# ---------------------------------------------------------------------------
# The whole organelle
# ---------------------------------------------------------------------------

def content(half_len_nm, half_wid_nm, cristae, seed=1):
    """The content stream of ONE mitochondrion, in nanometre space, centred
    on the origin with its long axis along +x.

    Returns (content_string, bbox, f1_count) where bbox is [x0 y0 x1 y1] in
    the same nanometre space, sized to include the outer membrane stroke.
    """
    rng = _Rng(seed)
    o = []
    A, B = float(half_len_nm), float(half_wid_nm)
    a, b = A - IMS, B - IMS

    sites = _crista_sites(a, b, cristae, rng)
    im_pts, faces, tips = _inner_membrane(a, b, sites)

    # 1. outer membrane, filled with the intermembrane space colour so
    #    that colour is what shows through every crista slot
    o.append(f"{C_IMS[0]} {C_IMS[1]} {C_IMS[2]} rg")
    o.append(f"{C_OM[0]} {C_OM[1]} {C_OM[2]} RG")
    o.append(f"{OM_THICK:.2f} w 1 j 1 J")
    _ellipse(o, 0.0, 0.0, A - OM_THICK / 2, B - OM_THICK / 2)
    o.append("B")

    # 2. the inner membrane: one closed path, filled as the matrix. The
    #    crista lumens are notches in this path, so they stay the IMS
    #    colour laid down above.
    o.append(f"{C_MATRIX[0]} {C_MATRIX[1]} {C_MATRIX[2]} rg")
    o.append(f"{C_IM[0]} {C_IM[1]} {C_IM[2]} RG")
    o.append(f"{IM_THICK:.2f} w")
    _polyline(o, im_pts)
    o.append("B")

    # 3. matrix contents, clipped to the matrix so nothing spills into a
    #    crista lumen or across a membrane
    o.append("q")
    _polyline(o, im_pts)
    o.append("W n")

    #    mtDNA nucleoid(s): a fibrous patch, drawn as a pale blob with a
    #    few strands of packaged genome across it
    # Placed ON the long axis and spread along it, not scattered: a
    # nucleoid dropped at a random angle lands on a crista as often as
    # not, and a genome patch overlapping a membrane fold is the one
    # arrangement a section cannot show, since the crista is a wall.
    n_nuc = 1 if A < 900 else 3
    nr = NUCLEOID_R * (0.62 if A < 900 else 1.0)
    for k in range(n_nuc):
        f = 0.0 if n_nuc == 1 else ((k + 0.5) / n_nuc * 2.0 - 1.0)
        nx = f * A * 0.44 + rng.uniform(-0.03, 0.03) * A
        ny = rng.uniform(-0.20, 0.20) * B
        rr = nr * rng.uniform(0.85, 1.15)
        o.append(f"{C_NUCLEOID[0]} {C_NUCLEOID[1]} {C_NUCLEOID[2]} rg")
        _ellipse(o, nx, ny, rr, rr * rng.uniform(0.66, 0.84))
        o.append("f")
        # the packaged genome inside it: long shallow loops rather than
        # short scribbles, because mtDNA is a circle wound onto protein
        # and short strokes read as texture instead of as a molecule
        o.append(f"{C_DNA[0]} {C_DNA[1]} {C_DNA[2]} RG 1.8 w")
        for _k in range(4):
            # A closed, wobbly loop. Sampled densely as a polyline rather
            # than emitted as a few cubics: an earlier version used cubics
            # with duplicated control points, which is a straight line
            # written the long way, and every nucleoid came out a polygon.
            ph = rng.uniform(0, 2 * math.pi)
            r0 = rr * rng.uniform(0.34, 0.80)
            w3 = rng.uniform(0.18, 0.38)
            w5 = rng.uniform(0.08, 0.22)
            pts = []
            for i in range(41):
                th = 2.0 * math.pi * i / 40.0
                rad = r0 * (1.0 + w3 * math.sin(3 * th + ph)
                            + w5 * math.sin(5 * th + ph * 1.7))
                pts.append((nx + math.cos(th) * rad,
                            ny + math.sin(th) * rad * 0.74))
            _polyline(o, pts, close=False)
            o.append("S")

    #    mitoribosomes, scattered through the matrix
    o.append(f"{C_RIBO[0]} {C_RIBO[1]} {C_RIBO[2]} rg")
    n_ribo = int(max(7, (A * B) / 18000.0))
    for _ in range(n_ribo):
        ang = rng.uniform(0, 2 * math.pi)
        rad = math.sqrt(rng.next()) * 0.88
        rx, ry = math.cos(ang) * rad * a, math.sin(ang) * rad * b
        _ellipse(o, rx, ry, RIBO_R, RIBO_R * rng.uniform(0.82, 1.0))
        o.append("f")

    #    dense matrix granules
    o.append(f"{C_GRANULE[0]} {C_GRANULE[1]} {C_GRANULE[2]} rg")
    for _ in range(2 if A < 900 else 3):
        ang = rng.uniform(0, 2 * math.pi)
        rad = math.sqrt(rng.next()) * 0.7
        gx, gy = math.cos(ang) * rad * a, math.sin(ang) * rad * b
        _ellipse(o, gx, gy, GRANULE_R, GRANULE_R * 0.9)
        o.append("f")

    #    ATP synthase: stalks first as one stroked path, then the heads
    parts = _f1_particles(faces, tips, a, b, sites)
    o.append(f"{C_F1[0]} {C_F1[1]} {C_F1[2]} RG 2.4 w")
    for (fx, fy), (hx, hy) in parts:
        o.append(f"{fx:.3f} {fy:.3f} m {hx:.3f} {hy:.3f} l")
    o.append("S")
    o.append(f"{C_F1[0]} {C_F1[1]} {C_F1[2]} rg")
    for _f, (hx, hy) in parts:
        _ellipse(o, hx, hy, F1_R, F1_R)
    o.append("f")

    o.append("Q")

    pad = OM_THICK
    bbox = [-(A + pad), -(B + pad), A + pad, B + pad]
    return "\n".join(o), bbox, len(parts)


def instance_matrix(px, py, rot, flip=False):
    """The `cm` matrix that places a nanometre-space form at page point
    (px, py), rotated by `rot` radians.

    `flip` mirrors across the long axis, which doubles the apparent variety
    of a small form library for free -- a mitochondrion has no handedness,
    so a mirrored one is just as true as the original.
    """
    s = PT_PER_NM
    c, si = math.cos(rot), math.sin(rot)
    m = -1.0 if flip else 1.0
    return (
        f"{s*c:.12f} {s*si:.12f} {-s*si*m:.12f} {s*c*m:.12f} "
        f"{px:.7f} {py:.7f} cm"
    )


# ---------------------------------------------------------------------------
# The form library
# ---------------------------------------------------------------------------
# Every mitochondrion on the page is one of a handful of shared Form
# XObjects, placed with a `cm` matrix. Two reasons, and the second is the
# one that made it non-optional:
#
#   1. SIZE. 342 mitochondria x ~1000-3000 path operators inline would be
#      several megabytes of content stream for a page whose entire prior
#      content was 53 kB.
#   2. PRECISION. A 10 nm ATP synthase head is 2.8e-5 pt. Written as an
#      absolute page coordinate near x=540 that needs eleven significant
#      figures, which is past the 5-digit precision ISO 32000-1 Annex C
#      says a conforming reader need only honour. Inside a form it is the
#      literal `5.000`, and the placement matrix -- ONE number per
#      instance -- carries the magnitude.
#
# A side effect worth having: this makes the page a deep-zoom stress test
# of the renderer's Form XObject path under an extreme CTM, not just of
# its path rasteriser.

_LIB = {}     # (A, B, cristae, variant) -> form name
_ORDER = []   # insertion order, so object numbering is deterministic


def reset_library():
    """Clear the form registry, so a second `build_content()` in the same
    process does not accumulate duplicate forms."""
    _LIB.clear()
    del _ORDER[:]


def form_name(half_len_nm, half_wid_nm, cristae, variant):
    """Name of the shared form for this shape, registering it on first use."""
    key = (float(half_len_nm), float(half_wid_nm), int(cristae), int(variant))
    if key not in _LIB:
        _LIB[key] = "Mt%d" % len(_ORDER)
        _ORDER.append(key)
    return _LIB[key]


def place(px, py, rot, half_len_nm, half_wid_nm, cristae, variant=0, flip=False):
    """A content-stream fragment placing one mitochondrion at page point
    (px, py). `variant` picks a different pseudo-random interior; `flip`
    mirrors it. Together they give 2N distinct-looking organelles from N
    forms, which matters because 325 identical ones in a row would read as
    a repeating pattern rather than as tissue."""
    name = form_name(half_len_nm, half_wid_nm, cristae, variant)
    return "q " + instance_matrix(px, py, rot, flip) + " /" + name + " Do Q"


def forms():
    """Every registered form, in registration order, as
    (name, content_string, bbox). Call after all placements are emitted."""
    out = []
    for key in _ORDER:
        A, B, n, variant = key
        body, bbox, _f1 = content(A, B, n, seed=variant + 1)
        out.append((_LIB[key], body, bbox))
    return out


def library_stats():
    """(form count, instance-shape count, total F1 particles per form set)
    for the generator's own report."""
    total = 0
    for key in _ORDER:
        A, B, n, variant = key
        _b, _bb, f1 = content(A, B, n, seed=variant + 1)
        total += f1
    return len(_ORDER), total
