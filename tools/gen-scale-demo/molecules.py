"""The ten most abundant molecules in a banana cell, at 1:1, in PICOMETRES.

WHY THIS FILE EXISTS
====================
The page already claims that nothing on it is enlarged for clarity. That
claim is easy to make at 300 um and easy to keep at 10 nm. This box is
what happens when it is taken all the way down: a water molecule is
**0.28 nm = 7.9e-7 pt**, which is a millionth of a point, and it is drawn
at that size beside a banana that is drawn at 15.3 cm.

    banana                      153 mm
    water molecule                0.28 nm
    ratio                    546 000 000 : 1

Both are on one page, in one coordinate system, with no scale break
anywhere between them. Reading the label under the water molecule takes
roughly **1 300 000 000 %** — a billion three hundred million percent —
which is still three orders of magnitude inside the ceiling `Pass 74.1`
pushed out to.

UNITS, and why they are picometres
==================================
Because bond lengths and atomic radii are quoted in picometres, and using
their published units means the numbers in this file can be checked
against a reference table without arithmetic:

    O-H bond                     96 pm
    C-C bond                    154 pm
    C-O bond                    143 pm
    H van der Waals radius      110 pm
    C van der Waals radius      170 pm
    O van der Waals radius      152 pm
    K+ ionic radius             138 pm

A picometre is 2.835e-9 pt. As with `mitochondrion.py`, that magnitude
never appears in a coordinate: every molecule is a Form XObject authored
about its own origin in picometres, and one placement matrix carries the
scale. Here the argument is far stronger than it was there — an atom
written as an absolute page coordinate would need **fifteen** significant
figures, and IEEE 754 double gives about sixteen.

SPACE-FILLING, NOT BALL-AND-STICK, and that is a correctness decision
=====================================================================
Ball-and-stick is prettier and would show the connectivity better. It is
also a lie about size: a stick model of glucose is mostly empty space,
and this box exists to say how big these things are. The van der Waals
envelope IS the molecule's size, so every atom is drawn at its full vdW
radius and the label quotes the envelope, not the bond skeleton.

Consequence, stated rather than hidden: the rings come out as lumpy
blobs. Connectivity is sacrificed on purpose. Bonds are still drawn
underneath as dark sticks, so they show in the gaps.

THE CHEMISTRY, and where it is a projection rather than a structure
===================================================================
These are 2-D projections with idealised geometry — regular rings, ideal
bond lengths, substituents pointing radially outward. They are not
computed conformations, and a real sugar in solution is a puckered chair
rather than a flat hexagon. What IS accurate: the bond lengths, the
atomic radii, the atom counts, the connectivity, and therefore the
overall SIZE, which is the only quantity this box is making a claim
about.

The four polymers (starch, cellulose, pectin, protein) have no single
size at all — they are chains and assemblies. Each is drawn as a
representative segment or cross-section, and its label says which
dimension the number refers to ("1.3 nm helix", "3.5 nm fibril") rather
than pretending a polymer has a diameter.

THE RANKING, and the one thing about it that is genuinely misleading
====================================================================
Ordered by share of FRESH MASS in banana pulp parenchyma, which is the
sense in which they "make up" the cell:

    water          ~75 %      starch  ~20 % (unripe; becomes sugars)
    sucrose/glucose/fructose   the ripening products of that starch
    pectin, cellulose, hemicellulose   the wall, ~2.6 % as dietary fibre
    protein        ~1.1 %
    malic acid     ~0.5 %, the dominant organic acid
    potassium      ~0.36 %, and the reason bananas are famous for it

★ **By NUMBER of molecules the ranking is not close and not interesting:
water is about 99.5 % of them.** Everything else on this list competes
for the remaining half percent. Mass share is the ranking that says
something about the cell; molecule count says only that cells are wet.
The box states which one it is using, because a "top ten" with no unit
is a claim with no content.
"""

import math

# Points per picometre. 1 pt = 25.4/72 mm = 3.527778e8 pm.
#
# Written as a reciprocal for the same reason `mitochondrion.PT_PER_NM`
# is: the division form is millimetres-per-point scaled down, looks
# identical, and is wrong by a factor of 8.
PT_PER_PM = 1.0 / (25.4 / 72.0 * 1_000_000_000.0)

# --- atomic data, picometres ----------------------------------------------
VDW = {"H": 110.0, "C": 170.0, "N": 155.0, "O": 152.0, "P": 180.0, "S": 180.0}
K_IONIC = 138.0

BOND_CC = 154.0
BOND_CO = 143.0
BOND_OH = 96.0

# --- CPK colours -----------------------------------------------------------
CPK = {
    "H": (0.94, 0.94, 0.93),
    "C": (0.28, 0.28, 0.30),
    "N": (0.20, 0.34, 0.78),
    "O": (0.84, 0.20, 0.16),
    "P": (0.95, 0.55, 0.15),
    "S": (0.88, 0.78, 0.20),
    "K": (0.56, 0.30, 0.76),
}
C_BOND = (0.20, 0.20, 0.22)
C_OUTLINE = (0.14, 0.14, 0.16)
C_STARCH = (0.86, 0.76, 0.46)
C_PECTIN = (0.72, 0.62, 0.30)
C_CELL = (0.62, 0.72, 0.52)
C_PROTEIN = (0.62, 0.62, 0.78)

_K = 0.5522847498


# ---------------------------------------------------------------------------
# Primitives, all in picometres about the origin
# ---------------------------------------------------------------------------

def _circle(o, cx, cy, r):
    pts = [
        (-r, 0), (-r, r * _K), (-r * _K, r), (0, r),
        (r * _K, r), (r, r * _K), (r, 0),
        (r, -r * _K), (r * _K, -r), (0, -r),
        (-r * _K, -r), (-r, -r * _K), (-r, 0),
    ]
    p = [(cx + x, cy + y) for x, y in pts]
    o.append(f"{p[0][0]:.1f} {p[0][1]:.1f} m")
    for i in range(1, 13, 3):
        o.append(
            f"{p[i][0]:.1f} {p[i][1]:.1f} {p[i+1][0]:.1f} {p[i+1][1]:.1f} "
            f"{p[i+2][0]:.1f} {p[i+2][1]:.1f} c"
        )
    o.append("h")


def _blob(o, cx, cy, r, wob, seed, squash=1.0):
    """A closed irregular loop — used where the real object has no crisp
    outline (a protein globule, a microfibril envelope)."""
    rng = _Rng(seed)
    ph1, ph2 = rng.uniform(0, 6.28), rng.uniform(0, 6.28)
    pts = []
    for i in range(49):
        th = 2.0 * math.pi * i / 48.0
        rr = r * (1.0 + wob * math.sin(3 * th + ph1) + wob * 0.6 * math.sin(5 * th + ph2))
        pts.append((cx + math.cos(th) * rr, cy + math.sin(th) * rr * squash))
    o.append(f"{pts[0][0]:.1f} {pts[0][1]:.1f} m")
    for x, y in pts[1:]:
        o.append(f"{x:.1f} {y:.1f} l")
    o.append("h")


class _Rng:
    """The same deterministic LCG `mitochondrion` uses, and for the same
    reason: byte-identical output across machines and Python versions."""

    def __init__(self, seed):
        self.s = (seed * 1103515245 + 12345) & 0x7FFFFFFF

    def next(self):
        self.s = (self.s * 1103515245 + 12345) & 0x7FFFFFFF
        return self.s / float(0x7FFFFFFF)

    def uniform(self, lo, hi):
        return lo + (hi - lo) * self.next()


def _draw_atoms(o, atoms, bonds):
    """Sticks first, then space-filling spheres over them.

    Order matters and is the whole trick: drawn the other way round the
    bonds would score lines across every atom. Underneath, they show only
    in the gaps between spheres, which is exactly where connectivity is
    still readable.
    """
    if bonds:
        o.append(f"{C_BOND[0]} {C_BOND[1]} {C_BOND[2]} RG 34 w 1 J")
        for i, j in bonds:
            o.append(f"{atoms[i][1]:.1f} {atoms[i][2]:.1f} m "
                     f"{atoms[j][1]:.1f} {atoms[j][2]:.1f} l")
        o.append("S")
    o.append(f"{C_OUTLINE[0]} {C_OUTLINE[1]} {C_OUTLINE[2]} RG 9 w")
    for el, x, y in atoms:
        c = CPK[el]
        r = K_IONIC if el == "K" else VDW[el]
        o.append(f"{c[0]} {c[1]} {c[2]} rg")
        _circle(o, x, y, r)
        o.append("B")


def _extent(atoms, pad=0.0):
    """Bounding box of a space-filling model, radii included."""
    xs, ys = [], []
    for el, x, y in atoms:
        r = K_IONIC if el == "K" else VDW[el]
        xs += [x - r, x + r]
        ys += [y - r, y + r]
    return [min(xs) - pad, min(ys) - pad, max(xs) + pad, max(ys) + pad]


# ---------------------------------------------------------------------------
# The molecules
# ---------------------------------------------------------------------------

def _water():
    """H2O. O-H 96 pm, H-O-H 104.5 degrees. The only one of the ten whose
    every number here is exact rather than idealised."""
    half = math.radians(104.5 / 2.0)
    atoms = [
        ("O", 0.0, 0.0),
        ("H", BOND_OH * math.sin(half), BOND_OH * math.cos(half)),
        ("H", -BOND_OH * math.sin(half), BOND_OH * math.cos(half)),
    ]
    return atoms, [(0, 1), (0, 2)]


def _potassium():
    """K+, a bare ion. Drawn at the IONIC radius (138 pm), not the van der
    Waals radius of neutral potassium (275 pm) — a potassium atom that had
    kept its outer electron would be four times the volume, and there are
    none of those in a cell."""
    return [("K", 0.0, 0.0)], []


def _pyranose(cx, cy, rot, anomeric_oh=True):
    """A six-membered sugar ring: five carbons and the ring oxygen, with
    hydroxyls radiating outward and a CH2OH arm off C5.

    Ring radius from the bond length: for a regular hexagon the
    circumradius equals the side, so r = 150 pm.
    """
    r = 150.0
    pos = [(cx + r * math.cos(rot + i * math.pi / 3.0),
            cy + r * math.sin(rot + i * math.pi / 3.0)) for i in range(6)]
    atoms = [("O", pos[0][0], pos[0][1])]                      # ring O5
    atoms += [("C", p[0], p[1]) for p in pos[1:]]              # C1..C5
    bonds = [(i, (i + 1) % 6) for i in range(6)]
    # hydroxyls on C1..C4, radiating out from the ring centre
    for k in (1, 2, 3, 4):
        if k == 1 and not anomeric_oh:
            continue
        a = rot + k * math.pi / 3.0
        ox = cx + (r + BOND_CO) * math.cos(a)
        oy = cy + (r + BOND_CO) * math.sin(a)
        atoms.append(("O", ox, oy))
        bonds.append((k, len(atoms) - 1))
    # the C6H2OH arm off C5
    a = rot + 5 * math.pi / 3.0
    c6 = (cx + (r + BOND_CC) * math.cos(a - 0.30), cy + (r + BOND_CC) * math.sin(a - 0.30))
    o6 = (cx + (r + BOND_CC + BOND_CO) * math.cos(a - 0.52),
          cy + (r + BOND_CC + BOND_CO) * math.sin(a - 0.52))
    atoms.append(("C", c6[0], c6[1]))
    bonds.append((5, len(atoms) - 1))
    atoms.append(("O", o6[0], o6[1]))
    bonds.append((len(atoms) - 2, len(atoms) - 1))
    return atoms, bonds


def _furanose(cx, cy, rot):
    """A five-membered sugar ring (fructose in its furanose form), plus the
    two CH2OH arms that make it a ketose rather than an aldose."""
    r = BOND_CC / (2.0 * math.sin(math.pi / 5.0))
    pos = [(cx + r * math.cos(rot + i * 2 * math.pi / 5.0),
            cy + r * math.sin(rot + i * 2 * math.pi / 5.0)) for i in range(5)]
    atoms = [("O", pos[0][0], pos[0][1])]
    atoms += [("C", p[0], p[1]) for p in pos[1:]]
    bonds = [(i, (i + 1) % 5) for i in range(5)]
    for k in (2, 3):
        a = rot + k * 2 * math.pi / 5.0
        atoms.append(("O", cx + (r + BOND_CO) * math.cos(a), cy + (r + BOND_CO) * math.sin(a)))
        bonds.append((k, len(atoms) - 1))
    for k, tilt in ((1, -0.35), (4, 0.35)):
        a = rot + k * 2 * math.pi / 5.0
        atoms.append(("C", cx + (r + BOND_CC) * math.cos(a + tilt),
                      cy + (r + BOND_CC) * math.sin(a + tilt)))
        bonds.append((k, len(atoms) - 1))
        atoms.append(("O", cx + (r + BOND_CC + BOND_CO) * math.cos(a + tilt * 1.7),
                      cy + (r + BOND_CC + BOND_CO) * math.sin(a + tilt * 1.7)))
        bonds.append((len(atoms) - 2, len(atoms) - 1))
    return atoms, bonds


def _sucrose():
    """Glucose + fructose joined head to head through a glycosidic oxygen.

    That linkage is why sucrose is a NON-reducing sugar: the bond ties up
    both anomeric carbons, so neither ring can open. It is also why a
    banana tastes of sucrose before it tastes of glucose — invertase has
    to cut this one bond first.
    """
    ga, gb = _pyranose(-430.0, 40.0, math.radians(90), anomeric_oh=False)
    fa, fb = _furanose(430.0, -40.0, math.radians(-90))
    n = len(ga)
    atoms = ga + fa
    bonds = gb + [(i + n, j + n) for i, j in fb]
    atoms.append(("O", 0.0, 30.0))          # the glycosidic bridge
    bridge = len(atoms) - 1
    bonds += [(1, bridge), (n + 1, bridge)]
    return atoms, bonds


def _malic():
    """HOOC-CH(OH)-CH2-COOH: the dominant organic acid in banana pulp, and
    most of why an unripe one tastes sharp."""
    step, rise = BOND_CC * math.cos(math.radians(30)), BOND_CC * math.sin(math.radians(30))
    c = [(-1.5 * step, -rise / 2), (-0.5 * step, rise / 2),
         (0.5 * step, -rise / 2), (1.5 * step, rise / 2)]
    atoms = [("C", x, y) for x, y in c]
    bonds = [(0, 1), (1, 2), (2, 3)]
    for idx, sign in ((0, -1), (3, 1)):       # the two carboxyl groups
        x, y = c[idx]
        atoms.append(("O", x + sign * BOND_CO * 0.95, y + BOND_CO * 0.75))
        bonds.append((idx, len(atoms) - 1))
        atoms.append(("O", x + sign * BOND_CO * 0.95, y - BOND_CO * 0.75))
        bonds.append((idx, len(atoms) - 1))
    atoms.append(("O", c[1][0] - 20, c[1][1] + BOND_CO))   # the hydroxyl
    bonds.append((1, len(atoms) - 1))
    return atoms, bonds


def _amylose(o):
    """Starch, as a segment of the amylose helix.

    Amylose is a left-handed helix of about six glucose units per turn,
    ~1.3 nm across the coil and 2.1 nm per turn. Its LENGTH is not a
    property worth drawing: a chain runs to hundreds of nanometres, and
    amylopectin branches on top of that. What is drawn is two and a half
    turns.

    Drawn as glucose-sized beads rather than as atoms on purpose. At this
    zoom the individual sugars are what read; drawing 1 400 atoms would be
    honest and illegible.
    """
    o.append(f"{C_STARCH[0]} {C_STARCH[1]} {C_STARCH[2]} rg")
    o.append(f"{C_OUTLINE[0]} {C_OUTLINE[1]} {C_OUTLINE[2]} RG 12 w")
    x = -2700.0
    while x <= 2700.0:
        y = 520.0 * math.sin(2.0 * math.pi * x / 2100.0)
        _circle(o, x, y, 330.0)
        o.append("B")
        x += 300.0
    return [-3100, -1000, 3100, 1000]


def _pectin(o):
    """Pectin: a chain of galacturonic acid, the glue in the middle
    lamella. Its carboxyl groups (drawn red) are what calcium bridges
    crosslink — and what a ripening banana's pectinase cuts, which is
    most of why it goes soft."""
    o.append(f"{C_OUTLINE[0]} {C_OUTLINE[1]} {C_OUTLINE[2]} RG 40 w 1 J")
    o.append("-2400 0 m 2400 0 l S")
    for i in range(6):
        cx = -2000.0 + i * 800.0
        cy = 90.0 if i % 2 == 0 else -90.0
        o.append(f"{C_PECTIN[0]} {C_PECTIN[1]} {C_PECTIN[2]} rg")
        o.append(f"{C_OUTLINE[0]} {C_OUTLINE[1]} {C_OUTLINE[2]} RG 12 w")
        _circle(o, cx, cy, 300.0)
        o.append("B")
        o.append(f"{CPK['O'][0]} {CPK['O'][1]} {CPK['O'][2]} rg")
        _circle(o, cx, cy + 430.0, 130.0)
        o.append("f")
    return [-2800, -700, 2800, 800]


def _cellulose(o):
    """A cellulose microfibril, in CROSS SECTION.

    About 36 glucan chains hydrogen-bonded into a crystalline rod ~3.5 nm
    across and micrometres long. Sectioning it is the only way to state a
    size for it at all — lengthwise it would run off the box, off the
    page, and out of the building.
    """
    o.append(f"{C_CELL[0]} {C_CELL[1]} {C_CELL[2]} rg")
    o.append(f"{C_OUTLINE[0]} {C_OUTLINE[1]} {C_OUTLINE[2]} RG 14 w")
    _circle(o, 0, 0, 1750.0)
    o.append("B")
    o.append("0.42 0.52 0.34 rg 8 w")
    pitch = 520.0
    for row in range(-3, 4):
        for col in range(-3, 4):
            x = col * pitch + (pitch / 2 if row % 2 else 0)
            y = row * pitch * 0.87
            if math.hypot(x, y) < 1500.0:
                _circle(o, x, y, 200.0)
                o.append("B")
    return [-1900, -1900, 1900, 1900]


def _protein(o):
    """A globular protein, at the size of banana beta-amylase (~56 kDa,
    ~5.5 nm across) — the enzyme that chews the starch grains into maltose
    as the fruit ripens, so the largest molecule in the box is also the
    one doing the ripening."""
    o.append(f"{C_PROTEIN[0]} {C_PROTEIN[1]} {C_PROTEIN[2]} rg")
    o.append(f"{C_OUTLINE[0]} {C_OUTLINE[1]} {C_OUTLINE[2]} RG 16 w")
    _blob(o, 0, 0, 2600.0, 0.06, 7, 0.94)
    o.append("B")
    # a few alpha helices, as coils
    o.append("0.42 0.42 0.60 RG 90 w 1 J")
    rng = _Rng(3)
    for _ in range(4):
        x0, y0 = rng.uniform(-1500, 1100), rng.uniform(-1500, 1500)
        ang = rng.uniform(0, math.pi)
        ln = rng.uniform(1100, 1900)
        pts = []
        for i in range(25):
            t = ln * i / 24.0
            off = 260.0 * math.sin(2.0 * math.pi * t / 540.0)
            pts.append((x0 + math.cos(ang) * t - math.sin(ang) * off,
                        y0 + math.sin(ang) * t + math.cos(ang) * off))
        o.append(f"{pts[0][0]:.1f} {pts[0][1]:.1f} m")
        for x, y in pts[1:]:
            o.append(f"{x:.1f} {y:.1f} l")
        o.append("S")
    return [-2900, -2800, 2900, 2800]


# ---------------------------------------------------------------------------
# The ten, in order of share of fresh mass
# ---------------------------------------------------------------------------
# (key, name line, size line, builder). The size line quotes the DRAWN
# envelope, and for the polymers it names which dimension it is quoting
# rather than implying a chain has a diameter.

TEN = [
    ("water", "1 water", "74.9 % of the fruit", "0.37 nm"),
    ("starch", "2 starch", "5.4 % of the fruit", "1.3 nm helix"),
    ("glucose", "3 glucose", "5.0 % of the fruit", "1.00 nm"),
    ("fructose", "4 fructose", "4.9 % of the fruit", "0.83 nm"),
    ("sucrose", "5 sucrose", "2.4 % of the fruit", "1.68 nm"),
    ("cellulose", "6 cellulose", "1.2 % of the fruit", "3.5 nm fibril"),
    ("protein", "7 protein", "1.1 % of the fruit", "5.5 nm globule"),
    ("pectin", "8 pectin", "0.7 % of the fruit", "1.5 nm chain"),
    ("malic", "9 malic acid", "0.4 % of the fruit", "0.98 nm"),
    ("potassium", "10 potassium ion", "0.36 % of the fruit", "0.28 nm"),
]

# The ten sum to 96.4 % of a banana's fresh mass. The missing ~3.6 % is
# fat, ash, hemicellulose, the other organic acids, and everything present
# in milligrams.
TEN_TOTAL = 96.4

# ★ THESE PERCENTAGES ARE FOR A RIPE BANANA, and the ranking follows from
# them rather than the other way round -- which is why this list is in a
# different order than it was when the numbers were absent.
#
# Composition per 100 g of raw banana flesh: water 74.9, starch ~5.4,
# glucose 4.98, fructose 4.85, sucrose 2.39, dietary fibre 2.6 (of which
# cellulose ~1.2, pectin ~0.7, hemicellulose ~0.7), protein 1.09, malic
# acid ~0.4, potassium 0.358.
#
# ★★ AN UNRIPE BANANA IS A COMPLETELY DIFFERENT LIST, and the difference
# is the single largest fact about this fruit. Green pulp is roughly
# 20-25 % STARCH and 1-2 % sugars; ripening converts almost all of it, so
# starch falls by a factor of four while glucose and fructose rise by a
# factor of ten. Rows 2 through 5 are the same carbon, before and after.
#
# That leaves a tension inside this document, and it is disclosed rather
# than tidied away: the banana at the top of the page is drawn YELLOW, so
# these ripe figures match it -- but the two cells are drawn with green
# chloroplasts and starch grains, which is an UNRIPE section. Both are
# deliberate (the starch grains are the thing that makes the pulp cell
# recognisably a banana), and the box's subtitle says which state its
# numbers describe so a reader can see the seam rather than trip over it.

# ★ EVERY SIZE HERE IS THE DRAWN VAN DER WAALS ENVELOPE, and that is why
# water reads 0.37 nm rather than the 0.28 nm everybody quotes.
#
# Both numbers are correct and they measure different things. 0.28 nm is
# water's KINETIC diameter — the width of the hole it can squeeze
# through, which is the figure membrane and zeolite work cares about and
# therefore the one in every table. 0.37 nm is how much space the
# molecule's electron cloud actually occupies, oxygen plus both hydrogens.
#
# This box draws space-filling models at 1:1 and invites the operator to
# measure them, so a label quoting a number the drawing does not show
# would be a lie the page itself disproves. The envelope wins; the
# subtitle says which measure is being used, because "0.37 nm" against
# the familiar 0.28 will otherwise read as an error.


def molecule_form(key):
    """Content stream and bbox for one molecule, in picometre space about
    its own centre."""
    o = []
    if key == "starch":
        return _finish(o, _amylose(o))
    if key == "pectin":
        return _finish(o, _pectin(o))
    if key == "cellulose":
        return _finish(o, _cellulose(o))
    if key == "protein":
        return _finish(o, _protein(o))
    if key == "glucose":
        atoms, bonds = _pyranose(0.0, 0.0, math.radians(90))
    elif key == "fructose":
        atoms, bonds = _furanose(0.0, 0.0, math.radians(90))
    else:
        atoms, bonds = dict(
            water=_water, sucrose=_sucrose, malic=_malic, potassium=_potassium
        )[key]()
    _draw_atoms(o, atoms, bonds)
    return "\n".join(o), _extent(atoms, pad=20.0)


def _finish(o, bbox):
    return "\n".join(o), bbox


# ---------------------------------------------------------------------------
# The box
# ---------------------------------------------------------------------------
# Geometry, in picometres. The column pitch is set by the LABELS, not by
# the molecules — "10 potassium ion" is 6.3 nm of text over a 0.28 nm ion,
# a 22:1 mismatch. That is the same finding the page's arrow already
# records at a different scale (README section 3), arrived at
# independently four orders of magnitude further down.

COL_PITCH = 9400.0
NAME_SIZE = 780.0
SIZE_SIZE = 700.0
TITLE_SIZE = 1450.0
SUB_SIZE = 980.0
MARGIN = 1700.0

# The vertical layout, stated as a budget rather than derived from a
# guessed total. The first version computed `BOX_H` from a rounded
# multiple of the row pitch and then laid rows out inside it; the bottom
# row's second label line landed 235 pm BELOW the frame and was clipped by
# it, which is the failure mode of every layout whose height is asserted
# before its contents are measured.
MOL_ZONE = 6400.0                     # tallest molecule is the 5.8 nm protein
LABEL_GAP = 900.0                     # molecule underside to first label baseline
LABEL_LINES = 3                       # name, share of the fruit, size
LABEL_BLOCK = NAME_SIZE * 1.45 * LABEL_LINES
ROW_PITCH = MOL_ZONE + LABEL_GAP + LABEL_BLOCK + 1500.0
HEADER = MARGIN + TITLE_SIZE * 1.25 + SUB_SIZE * 1.5 * 2 + 1300.0
FOOTER = MARGIN + SIZE_SIZE * 2.0     # the scale bar sits in here

BOX_W = 5 * COL_PITCH + 2 * MARGIN
BOX_H = HEADER + 2 * ROW_PITCH + FOOTER

# The banana's drawn centreline, in points. Set by `gen_banana` before the
# forms are serialised, so the box's "the banana above is N of these
# across" caption is a measurement of the banana actually on the page
# rather than a number typed in beside it. Defaults to the current value
# so this module still renders standalone.
BANANA_PT = 433.0


def box_content(form_names):
    """The box: frame, title, ten placed molecules, and a two-line label
    under each. Authored in picometres; `form_names[key]` is the XObject
    name to invoke for each molecule.

    Everything is one Form XObject so that the whole box is placed by a
    single matrix. That also means the molecules are forms INSIDE a form,
    which the renderer has to nest — a free extra test of a path that the
    banana did not previously exercise.
    """
    o = []
    x0, y0 = -BOX_W / 2.0, -BOX_H / 2.0

    # frame
    o.append("0.995 0.985 0.96 rg 0.42 0.38 0.30 RG 34 w")
    o.append(f"{x0:.0f} {y0:.0f} {BOX_W:.0f} {BOX_H:.0f} re B")

    # title
    o.append("0.16 0.16 0.19 rg")
    top = y0 + BOX_H - MARGIN - TITLE_SIZE
    o.append(f"BT /F2 {TITLE_SIZE:.0f} Tf {x0 + MARGIN:.0f} {top:.0f} Td "
             f"(the ten most abundant molecules in these cells) Tj ET")
    o.append("0.36 0.36 0.40 rg")
    o.append(f"BT /F1 {SUB_SIZE:.0f} Tf {x0 + MARGIN:.0f} {top - SUB_SIZE * 1.6:.0f} Td "
             f"(drawn 1:1, like the banana. sizes are the van der Waals envelope. shares are) Tj ET")
    o.append(f"BT /F1 {SUB_SIZE:.0f} Tf {x0 + MARGIN:.0f} {top - SUB_SIZE * 3.1:.0f} Td "
             f"(% of fresh mass in RIPE pulp - green pulp is ~22 % starch and ~1 % sugar) Tj ET")

    # The grid, laid out from the top of the box downward, so that every
    # row's label block is placed inside a budget that was reserved for it
    # rather than wherever the previous row happened to end.
    grid_top = y0 + BOX_H - HEADER

    for n, (key, name, share, size) in enumerate(TEN):
        col, row = n % 5, n // 5
        cx = x0 + MARGIN + COL_PITCH * (col + 0.5)
        cy = grid_top - ROW_PITCH * row - MOL_ZONE / 2.0
        o.append("q")
        o.append(f"1 0 0 1 {cx:.0f} {cy:.0f} cm /{form_names[key]} Do")
        o.append("Q")
        # Three label lines, centred under the molecule: what it is, how
        # much of the fruit it is, and how big it is. The SHARE sits
        # second, above the size, because it is what orders the grid --
        # a reader who wonders why cellulose comes before protein finds
        # the answer on the line under both of them.
        base = cy - MOL_ZONE / 2.0 - LABEL_GAP
        for k, (txt, sz, col_rgb) in enumerate((
            (name, NAME_SIZE, (0.16, 0.16, 0.19)),
            (share, SIZE_SIZE, (0.30, 0.34, 0.44)),
            (size, SIZE_SIZE, (0.46, 0.46, 0.50)),
        )):
            w = len(txt) * sz * 0.53
            ly = base - k * NAME_SIZE * 1.45
            o.append(f"{col_rgb[0]} {col_rgb[1]} {col_rgb[2]} rg")
            o.append(f"BT /F1 {sz:.0f} Tf {cx - w / 2.0:.0f} {ly:.0f} Td ({txt}) Tj ET")

    # A 1 nm scale bar in the footer band, so the box substantiates its own
    # claim instead of asserting it. It lives BELOW the grid rather than
    # inside it: the first version put it at the box's bottom-left corner,
    # which is under column one, and it printed straight through the
    # "6 pectin" label.
    sb_x = x0 + MARGIN
    sb_y = y0 + MARGIN * 0.75
    o.append("0.16 0.16 0.19 rg")
    o.append(f"{sb_x:.0f} {sb_y:.0f} 1000 90 re f")
    # The box's width is DERIVED into this string, not typed into it. The
    # first version said "46 nm" as a literal and was wrong within the
    # hour, because widening a column changed `BOX_W` and nothing changed
    # the sentence -- the same two-copies-of-one-fact shape that `R212`
    # names, reproduced inside a caption.
    o.append(f"BT /F1 {SIZE_SIZE:.0f} Tf {sb_x + 1300:.0f} {sb_y - 40:.0f} Td "
             f"(1 nm. box is {BOX_W / 1000.0:.0f} nm wide. these ten are "
             f"{TEN_TOTAL:.1f} % of a banana) Tj ET")
    # Also derived, and for a sharper reason than the width above: the
    # first version said 546 000 000, which is the banana measured in
    # 0.28 nm water molecules -- correct against the KINETIC diameter and
    # wrong against the 0.37 nm envelope this box actually draws and
    # labels. Two numbers for water, two ratios, and the caption quietly
    # used the other one. Deriving it from `BANANA_PT` makes the box
    # internally consistent by construction.
    n = int(round(BANANA_PT / (370.0 * PT_PER_PM)))
    right = f"the banana above is {n:,} of these across".replace(",", " ")
    o.append("0.42 0.42 0.46 rg")
    o.append(
        f"BT /F1 {SIZE_SIZE:.0f} Tf "
        f"{x0 + BOX_W - MARGIN - len(right) * SIZE_SIZE * 0.53:.0f} {sb_y - 40:.0f} Td "
        f"({right}) Tj ET"
    )

    return "\n".join(o), [x0 - 40, y0 - 40, x0 + BOX_W + 40, y0 + BOX_H + 40]


def instance_matrix(px, py):
    """Place the picometre-space box at page point (px, py), unrotated."""
    s = PT_PER_PM
    return f"{s:.16f} 0 0 {s:.16f} {px:.7f} {py:.7f} cm"


# ---------------------------------------------------------------------------
# The form library
# ---------------------------------------------------------------------------
# Eleven forms: ten molecules plus the box that invokes them. The nesting
# is deliberate — the box is placed by ONE matrix, so the ten molecules
# inside it inherit their 1:1 scale rather than each carrying a copy of
# it, and a mistake in the scale cannot land on nine of ten.

_NAMES = {}
_ORDER = []


def reset_library():
    _NAMES.clear()
    del _ORDER[:]


def register():
    """Intern every molecule form plus the box, and return the box's name."""
    reset_library()
    for key, _n, _s, _b in TEN:
        _NAMES[key] = "Mol%d" % len(_ORDER)
        _ORDER.append(key)
    _NAMES["__box__"] = "MolBox"
    return "MolBox"


def forms():
    """(name, content, bbox, resource_dict_bytes) for every form.

    The molecules need no resources at all; the box needs the two page
    fonts and the ten molecule XObjects. `resource_dict_bytes` is a
    callable taking a name→object-number map, because the box cannot be
    serialised until the writer has assigned object numbers to the
    molecules it references.
    """
    out = []
    for key in _ORDER:
        body, bbox = molecule_form(key)
        out.append((_NAMES[key], body, bbox, None))
    body, bbox = box_content(_NAMES)
    out.append((_NAMES["__box__"], body, bbox, [_NAMES[k] for k in _ORDER]))
    return out
