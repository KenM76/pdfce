"""The detailed cell interiors for gen_banana.py.

WHY THIS IS A SEPARATE MODULE
=============================
The two cells carry more geometry than the rest of the page put together,
and all of it is authored in MICROMETRES rather than points. Keeping that
coordinate system in one file means the drawing code never sees a
conversion factor: `p(120, 40)` is "120 um right, 40 um up, inside this
cell", and the only place micrometres become points is `emit`.

THE SCALE CHAIN THIS BUILDS, which is the point of the exercise
===============================================================
Four tiers on one page, each needing roughly ten times the zoom of the one
above it:

    tier                        size        as points     visible at
    banana                      153 mm      433 pt        100 %
    cell outlines               300 um      0.85 pt       ~2 000 %
    cell labels                 30 um       0.085 pt      ~12 000 %
    starch grains, nucleus      25-45 um    0.07-0.13 pt  ~12 000 %
    organelle labels            8 um        0.023 pt      ~45 000 %
    chloroplasts, plasmodesmata 1-5 um      0.003-0.014   ~60 000 %
    mitochondria, cristae       0.7-1.5 um  0.002-0.004   ~250 000 %

THE BIOLOGY, and where it is a simplification
=============================================
* PULP is parenchyma: a thin primary wall, a vacuole occupying most of the
  volume, cytoplasm pressed into a peripheral band, the nucleus shoved
  against the wall, and — the signature of an unripe banana — amyloplasts
  crammed with large concentrically-layered starch grains with an
  ECCENTRIC hilum. Those grains are why the tissue is starchy and firm.
* PEEL epidermis is a brick-shaped cell under a waxy CUTICLE, with a much
  thicker outer wall than its side walls, and chloroplasts (green fruit)
  that become carotenoid-loaded chromoplasts as it ripens. Drawn green,
  i.e. unripe.
* Simplified deliberately: no endoplasmic-reticulum network beyond a few
  strands, no ribosomes, no accurate membrane bilayers. Organelle COUNTS
  are illustrative; a real section would show far more mitochondria.
"""

UM = 1.0 / (25.4 / 72.0 * 1000.0)  # points per micrometre


def um(v):
    return v * UM


class Pen:
    """Accumulates content-stream operators in micrometre space."""

    def __init__(self, ox, oy):
        self.ox, self.oy = ox, oy
        self.out = []

    def p(self, x, y):
        """Micrometres inside the cell -> absolute page points."""
        return (self.ox + um(x), self.oy + um(y))

    def op(self, s):
        self.out.append(s)

    def fill(self, r, g, b):
        self.op(f"{r} {g} {b} rg")

    def stroke(self, r, g, b):
        self.op(f"{r} {g} {b} RG")

    def width(self, w_um):
        self.op(f"{um(w_um):.7f} w")

    def moveto(self, x, y):
        a, b = self.p(x, y)
        self.op(f"{a:.7f} {b:.7f} m")

    def lineto(self, x, y):
        a, b = self.p(x, y)
        self.op(f"{a:.7f} {b:.7f} l")

    def curveto(self, x1, y1, x2, y2, x3, y3):
        a1, b1 = self.p(x1, y1)
        a2, b2 = self.p(x2, y2)
        a3, b3 = self.p(x3, y3)
        self.op(f"{a1:.7f} {b1:.7f} {a2:.7f} {b2:.7f} {a3:.7f} {b3:.7f} c")

    def poly(self, pts, close=True):
        self.moveto(*pts[0])
        for q in pts[1:]:
            self.lineto(*q)
        if close:
            self.op("h")

    def ellipse(self, cx, cy, rx, ry, rot=0.0):
        """Kappa-approximated ellipse, optionally rotated (radians)."""
        import math

        k = 0.5522847498
        pts = [
            (-rx, 0), (-rx, ry * k), (-rx * k, ry), (0, ry),
            (rx * k, ry), (rx, ry * k), (rx, 0),
            (rx, -ry * k), (rx * k, -ry), (0, -ry),
            (-rx * k, -ry), (-rx, -ry * k), (-rx, 0),
        ]
        c, s = math.cos(rot), math.sin(rot)
        r = [(cx + x * c - y * s, cy + x * s + y * c) for x, y in pts]
        self.moveto(*r[0])
        for i in range(1, 13, 3):
            self.curveto(*r[i], *r[i + 1], *r[i + 2])
        self.op("h")

    def text(self, x, y, size_um, s):
        a, b = self.p(x, y)
        self.op(f"BT /F1 {um(size_um):.7f} Tf {a:.7f} {b:.7f} Td ({s}) Tj ET")

    def leader(self, x1, y1, x2, y2, w_um=0.6):
        self.width(w_um)
        self.moveto(x1, y1)
        self.lineto(x2, y2)
        self.op("S")


# ---------------------------------------------------------------------------
# The pulp parenchyma cell: 300 x 300 um
# ---------------------------------------------------------------------------

# An irregular polygon, because parenchyma in section is polygonal — not
# the rounded blob a diagram usually shows.
PULP_OUTLINE = [
    (10, 46), (34, 12), (118, 3), (232, 9), (289, 44),
    (297, 138), (289, 252), (240, 293), (121, 297), (36, 284), (5, 198),
]

# Grains: (cx, cy, rx, ry, rotation, ring count). Eccentric hilum is drawn
# by offsetting the inner rings toward one end.
# Ringed around the periphery rather than scattered, to leave the vacuole's
# centre clear for the easter egg. Grains cluster where they cluster, so
# this costs nothing biologically.
STARCH = [
    (46, 74, 27, 19, 0.35, 5),
    (252, 76, 25, 18, -0.25, 4),
    (258, 228, 22, 16, 0.55, 4),
    (150, 272, 25, 17, -0.5, 4),
    (42, 168, 19, 14, 0.2, 3),
    (262, 156, 18, 13, 0.9, 3),
]

MITOS = [
    (30, 96, 0.35), (26, 130, -0.4), (33, 160, 0.15), (44, 226, 0.8),
    (74, 262, -0.2), (140, 274, 0.4), (206, 268, -0.6), (258, 236, 0.25),
    (274, 168, 0.7), (268, 96, -0.3), (232, 40, 0.5), (160, 26, -0.15),
    (92, 34, 0.3), (52, 62, -0.7),
]


def draw_pulp(ox, oy):
    d = Pen(ox, oy)

    # middle lamella + primary wall, drawn as two strokes of the same
    # polygon so the shared pectin layer reads as a separate structure
    d.stroke(0.62, 0.50, 0.24)
    d.width(7)
    d.poly(PULP_OUTLINE)
    d.op("S")
    d.fill(0.995, 0.975, 0.88)
    d.stroke(0.47, 0.37, 0.16)
    d.width(2.6)
    d.poly(PULP_OUTLINE)
    d.op("B")

    # wall stubs where neighbouring cells meet this one
    d.stroke(0.62, 0.50, 0.24)
    d.width(6)
    for (x1, y1, x2, y2) in ((34, 12, 22, -14), (289, 44, 314, 30),
                             (289, 252, 316, 268), (121, 297, 112, 322),
                             (5, 198, -20, 206)):
        d.moveto(x1, y1)
        d.lineto(x2, y2)
        d.op("S")

    # tonoplast: the vacuole membrane, inset from the wall by the width of
    # the peripheral cytoplasm
    d.fill(0.965, 0.975, 0.93)
    d.stroke(0.60, 0.68, 0.55)
    d.width(1.1)
    d.poly([(x * 0.86 + 21, y * 0.86 + 21) for x, y in PULP_OUTLINE])
    d.op("B")

    # starch grains inside amyloplasts, with growth rings and an
    # eccentric hilum — the thing that makes this a BANANA cell
    for cx, cy, rx, ry, rot, rings in STARCH:
        d.fill(0.86, 0.79, 0.55)
        d.stroke(0.60, 0.52, 0.28)
        d.width(0.9)
        d.ellipse(cx, cy, rx, ry, rot)
        d.op("B")
        import math

        hx = cx + math.cos(rot) * rx * 0.34   # hilum, off toward one end
        hy = cy + math.sin(rot) * rx * 0.34
        d.stroke(0.70, 0.62, 0.36)
        d.width(0.5)
        for i in range(1, rings):
            f = 1.0 - i / float(rings)
            d.ellipse(hx * (1 - f) + cx * f, hy * (1 - f) + cy * f,
                      rx * f, ry * f, rot)
            d.op("S")
        d.fill(0.55, 0.46, 0.24)
        d.ellipse(hx, hy, rx * 0.055, rx * 0.055, 0)
        d.op("f")

    # nucleus pressed against the wall by the vacuole, with envelope,
    # nucleolus and chromatin
    nx, ny, nr = 52, 246, 25
    d.fill(0.80, 0.74, 0.86)
    d.stroke(0.46, 0.38, 0.56)
    d.width(1.4)
    d.ellipse(nx, ny, nr, nr * 0.88, -0.3)
    d.op("B")
    d.stroke(0.46, 0.38, 0.56)
    d.width(0.5)
    d.ellipse(nx, ny, nr - 2.4, nr * 0.88 - 2.4, -0.3)
    d.op("S")
    d.fill(0.50, 0.40, 0.60)
    d.ellipse(nx + 5, ny - 3, 7.5, 7.0, 0)
    d.op("f")
    d.fill(0.66, 0.58, 0.76)
    for dx, dy, r in ((-12, 6, 3.0), (-6, -10, 2.4), (10, 9, 2.6),
                      (14, -8, 2.0), (-15, -5, 2.2)):
        d.ellipse(nx + dx, ny + dy, r, r * 0.85, 0)
        d.op("f")

    # mitochondria in the peripheral cytoplasm, each with a cristae line
    for cx, cy, rot in MITOS:
        d.fill(0.72, 0.55, 0.50)
        d.stroke(0.50, 0.34, 0.30)
        d.width(0.18)
        d.ellipse(cx, cy, 1.5, 0.72, rot)
        d.op("B")
        import math

        d.stroke(0.50, 0.34, 0.30)
        d.width(0.13)
        for t in (-0.6, 0.0, 0.6):
            x0 = cx + math.cos(rot) * t - math.sin(rot) * 0.55
            y0 = cy + math.sin(rot) * t + math.cos(rot) * 0.55
            x1 = cx + math.cos(rot) * t + math.sin(rot) * 0.55
            y1 = cy + math.sin(rot) * t - math.cos(rot) * 0.55
            d.moveto(x0, y0)
            d.lineto(x1, y1)
            d.op("S")

    # a Golgi stack and a few ER strands
    d.stroke(0.42, 0.55, 0.62)
    d.width(0.3)
    for i in range(4):
        d.moveto(86, 60 + i * 1.6)
        d.curveto(90, 63 + i * 1.6, 96, 63 + i * 1.6, 100, 60 + i * 1.6)
        d.op("S")
    d.width(0.35)
    d.moveto(36, 210)
    d.curveto(48, 222, 44, 236, 56, 244)
    d.op("S")
    d.moveto(30, 196)
    d.curveto(46, 204, 52, 220, 68, 228)
    d.op("S")

    # tannin deposit — bananas brown from phenolics like these
    d.fill(0.45, 0.33, 0.24)
    d.ellipse(244, 118, 5.5, 4.6, 0.4)
    d.op("f")

    # plasmodesmata: channels through the wall, in a pit field
    d.stroke(0.47, 0.37, 0.16)
    d.width(0.7)
    for i in range(6):
        y = 120 + i * 9
        d.moveto(292, y)
        d.lineto(300, y)
        d.op("S")

    # --- the easter egg, in the clear centre of the vacuole -------------
    import easter_egg

    draw_pulp.mito_count = easter_egg.draw(d)

    # --- organelle labels, 8 um: a tier below the 30 um cell labels -----
    d.fill(0.20, 0.20, 0.24)
    d.stroke(0.45, 0.45, 0.50)
    lab = [
        (46, 74, 96, 40, "starch grain in amyloplast"),
        (52, 246, -78, 300, "nucleus"),
        (57, 251, -78, 289, "nucleolus + chromatin"),
        (33, 160, -88, 168, "mitochondrion"),
        (93, 63, -74, 52, "Golgi stack"),
        (96, 250, 118, 262, "central vacuole"),
        (297, 138, 318, 132, "primary wall"),
        (300, 145, 318, 152, "middle lamella"),
        (296, 156, 318, 166, "plasmodesmata"),
        (244, 118, 262, 100, "tannin deposit"),
        (36, 208, -86, 196, "ER strand"),
    ]
    for tx, ty, lx, ly, s in lab:
        d.leader(tx, ty, lx, ly, 0.45)
        anchor = lx if lx > tx else lx
        d.text(anchor, ly + 2, 8, s)
    return "\n".join(d.out)


# ---------------------------------------------------------------------------
# The peel epidermal cell: 60 x 35 um
# ---------------------------------------------------------------------------

def draw_skin(ox, oy, w=60.0, h=35.0):
    d = Pen(ox, oy)

    # cuticle: the waxy layer outside the outer wall, slightly uneven
    d.fill(0.93, 0.90, 0.62)
    d.stroke(0.72, 0.68, 0.38)
    d.width(0.25)
    d.moveto(-1, h)
    d.curveto(10, h + 2.4, 22, h + 1.2, 32, h + 2.2)
    d.curveto(42, h + 3.0, 52, h + 1.4, w + 1, h + 2.0)
    d.lineto(w + 1, h)
    d.lineto(-1, h)
    d.op("h B")

    # walls: the outer one much thicker than the side and inner walls
    d.fill(0.90, 0.95, 0.87)
    d.stroke(0.33, 0.45, 0.30)
    d.width(0.9)
    d.poly([(0, 0), (w, 0), (w, h), (0, h)])
    d.op("B")
    d.fill(0.62, 0.72, 0.55)
    d.op(f"{d.p(0, h - 3.2)[0]:.7f} {d.p(0, h - 3.2)[1]:.7f} "
         f"{um(w):.7f} {um(3.2):.7f} re f")

    # neighbouring cells, so the brick reads as part of a sheet
    d.stroke(0.33, 0.45, 0.30)
    d.width(0.9)
    for x in (-9, w + 9):
        d.moveto(x, 0)
        d.lineto(x, h)
        d.op("S")
    d.moveto(-9, 0)
    d.lineto(w + 9, 0)
    d.op("S")
    d.moveto(-9, h)
    d.lineto(w + 9, h)
    d.op("S")

    # vacuole
    d.fill(0.95, 0.98, 0.94)
    d.stroke(0.55, 0.66, 0.50)
    d.width(0.3)
    d.ellipse(w * 0.55, h * 0.45, w * 0.33, h * 0.30, 0)
    d.op("B")

    # chloroplasts — green peel, i.e. unripe. Lens-shaped, with grana.
    import math

    for cx, cy, rot in ((11, 8, 0.3), (24, 26, -0.4), (44, 9, 0.15),
                        (52, 24, 0.8), (33, 6, -0.2), (18, 19, 0.6)):
        d.fill(0.42, 0.62, 0.34)
        d.stroke(0.26, 0.44, 0.22)
        d.width(0.16)
        d.ellipse(cx, cy, 2.6, 1.5, rot)
        d.op("B")
        d.stroke(0.26, 0.44, 0.22)
        d.width(0.12)
        for t in (-1.2, -0.4, 0.4, 1.2):
            x0 = cx + math.cos(rot) * t - math.sin(rot) * 1.0
            y0 = cy + math.sin(rot) * t + math.cos(rot) * 1.0
            x1 = cx + math.cos(rot) * t + math.sin(rot) * 1.0
            y1 = cy + math.sin(rot) * t - math.cos(rot) * 1.0
            d.moveto(x0, y0)
            d.lineto(x1, y1)
            d.op("S")

    # nucleus — proportionally far larger here than in the pulp cell
    d.fill(0.78, 0.72, 0.85)
    d.stroke(0.44, 0.36, 0.54)
    d.width(0.3)
    d.ellipse(w * 0.28, h * 0.62, 5.4, 4.6, -0.2)
    d.op("B")
    d.fill(0.50, 0.40, 0.60)
    d.ellipse(w * 0.28 + 1.2, h * 0.62 - 0.6, 1.7, 1.5, 0)
    d.op("f")

    # mitochondria
    for cx, cy, rot in ((38, 18, 0.4), (48, 13, -0.3), (8, 26, 0.2)):
        d.fill(0.72, 0.55, 0.50)
        d.stroke(0.50, 0.34, 0.30)
        d.width(0.12)
        d.ellipse(cx, cy, 1.1, 0.55, rot)
        d.op("B")

    # plasmodesmata through the side walls
    d.stroke(0.33, 0.45, 0.30)
    d.width(0.22)
    for y in (9, 17, 25):
        d.moveto(-2.5, y)
        d.lineto(2.5, y)
        d.op("S")
        d.moveto(w - 2.5, y)
        d.lineto(w + 2.5, y)
        d.op("S")

    # --- organelle labels, 3.5 um (this cell is five times smaller) -----
    d.fill(0.20, 0.20, 0.24)
    d.stroke(0.45, 0.45, 0.50)
    lab = [
        (30, h + 2.0, 30, h + 12, "cuticle (wax)"),
        (30, h - 1.6, 62, h + 6, "thick outer wall"),
        (24, 26, 66, 30, "chloroplast (grana)"),
        (w * 0.28, h * 0.62, -34, 30, "nucleus"),
        (38, 18, 66, 18, "mitochondrion"),
        (w * 0.55, h * 0.45, 66, 10, "vacuole"),
        (w + 2.5, 17, 66, 2, "plasmodesmata"),
        (0, 4, -34, 4, "anticlinal wall"),
    ]
    for tx, ty, lx, ly, s in lab:
        d.leader(tx, ty, lx, ly, 0.16)
        d.text(lx, ly + 0.8, 3.5, s)
    return "\n".join(d.out)
