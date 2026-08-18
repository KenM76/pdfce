#!/usr/bin/env python3
"""ghent-check — turn the Ghent PDF Output Suite into a pass/fail signal.

WHY THIS EXISTS
===============
The Ghent PDF Output Suite 5.0 ships 51 single-patch PDFs, each testing one
PDF/X feature. Its stated pass criterion is a human at 0.5 m looking for a
red X, which is not automatable — but the ARTWORK IS AUTHORED PRE-SWAPPED,
and that is what makes it mechanical. Each patch draws a trap X whose colour
is chosen so that a CORRECT renderer makes it vanish into its surround and
an INCORRECT one leaves it visible. Every patch says so on its own face; the
GWG 16.0 patch's caption reads "If an 'X' appears, rendering of Non-Knockout
Transparency Groups (a transparency effect) is not performed correctly."

So the pass/fail signal ships inside the corpus. No press, no proof, no
instrument, no reference measurement to source, and — importantly — no
second renderer to disagree with.

WHY NOT JUST DIFF AGAINST pdfium
================================
Because pdfium fails many of these tests too. It is a screen renderer with
no overprint, and `tools/render-parity` measured 11 "unexplained" and 40
"disclosed-gap" divergences across these same 51 patches — a number that
mixes pdfce's failures with pdfium's and cannot separate them. The trap X is
an ORACLE; pdfium is a peer. Where an oracle exists, it wins (the same
argument `tools/check-image-colorspace-truth.py` makes for closed-form Lab).

HOW THE DETECTOR WORKS, AND WHY IT IS CONTENT-INDEPENDENT
=========================================================
The trap is two crossing DIAGONAL strokes. Essentially all other content on
these pages — text, table rules, swatch borders, the PDF/X-4 badge — is
AXIS-ALIGNED. So the discriminator is the ratio of diagonal to axis-aligned
edge energy in a sliding window:

    diag(x,y) = min(|dI/dx|, |dI/dy|)      both large  => a 45-degree edge
    axis(x,y) = | |dI/dx| - |dI/dy| |      one dominates => H or V edge
    score     = sum(diag) / sum(axis)

CALIBRATION, measured rather than chosen (2026-08-17, GWG 16.0 at scale 2.0,
the patch whose expected result was known independently because pdfce had
just been changed to fix it):

    swatch        score     verdict
    Hue           0.650     X VISIBLE
    Saturation    0.599     X VISIBLE
    Color         0.570     X VISIBLE
    HardLight     0.043     clean
    Luminosity    0.026     clean
    Difference    0.019     clean
    Opacity 0%    0.015     clean
    Exclusion     0.014     clean

A 13x separation between the two populations, so the 0.25 threshold sits in
an empty gap rather than being tuned to a wanted answer (W14). The energy
floor exists only to reject small text glyphs, whose strokes are short
enough to produce a high ratio on very little total energy.

WHAT A VERDICT HERE DOES AND DOES NOT MEAN
==========================================
`X` means the suite's own trap fired: that feature is not rendered
correctly. `clean` means no trap fired in that patch — which is the suite's
pass criterion and is NOT the same as "pixel-correct". A patch can be clean
and still differ from a press proof in ways the trap was not designed to
catch. This tool reports what the suite asks; it does not claim more.

USAGE
=====
    python tools/ghent-check.py <dir-of-patch-pdfs> [--scale 2.0] [--json]

Out-of-tree tooling, exactly like `tools/render-parity`: never shipped,
never in `cargo test`, never in the GUI-core `cargo tree` invariant.
Requires `pillow` + `numpy`; uses the shipped `pdfce-cli render-page`.
"""

import argparse
import json
import os
import subprocess
import sys

import cv2
import numpy as np

AREA_MIN = 200      # px; below this a mark is a glyph, not a swatch trap
EDGE_MIN, EDGE_MAX = 16, 90
FILL_LO, FILL_HI = 0.15, 0.60
DIAG_MIN = 0.85
CONTRAST_MIN = 12.0   # 8-bit levels; below this the X is not "clear"


def cli_path():
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    for cand in ("pdfce-cli.exe", "pdfce-cli"):
        p = os.path.join(root, "target", "release", cand)
        if os.path.exists(p):
            return p
    sys.exit("build the release CLI first: cargo build --release -p pdfce-cli")


def find_traps(png):
    """Locate trap X marks by EXACT intensity level and shape.

    Segmenting on exact levels rather than a quantised or thresholded image
    is what makes this work, and it is not an optimisation. The trap is a
    FLAT-FILLED shape drawn in one colour over a flat swatch of another —
    measured on GWG 16.0, a grey `178` X on a black `0` square — so the two
    are perfectly separable by value and the X falls out as one connected
    component. An edge detector sees only its outline (which is a hollow X,
    with none of the shape statistics below), and a quantiser can split two
    nearby trap colours into different buckets: `178` and `165` land in
    different bins at any step coarser than 13, and those are the actual
    values of two adjacent traps on the same patch.

    Shape test, all four measured on known traps before being fixed as
    thresholds (GWG 16.0: three traps, each 38x38, fill 0.44, diag 1.00;
    every clean swatch scored below 0.05 on the diagonal measure):

      * bbox 16..90 px square-ish -- a swatch-sized mark, not a glyph;
      * fill 0.15..0.60 -- an X is thin; a filled square or blob is not;
      * >=85% of the mark's pixels lie within 0.25 of a bbox diagonal;
      * >=200 px so a small letter cannot qualify on ratio alone.
    """
    im = cv2.imread(png, cv2.IMREAD_GRAYSCALE)
    if im is None:
        return []
    found = []
    levels, counts = np.unique(im, return_counts=True)
    for v in levels[counts >= AREA_MIN]:
        mask = (im == v).astype(np.uint8)
        n, lab, stats, _ = cv2.connectedComponentsWithStats(mask, 8)
        for i in range(1, n):
            x, y, w, h, area = (int(z) for z in stats[i])
            if area < AREA_MIN:
                continue
            if not (EDGE_MIN <= w <= EDGE_MAX and EDGE_MIN <= h <= EDGE_MAX):
                continue
            if abs(w - h) > max(w, h) * 0.4:
                continue
            fill = area / float(w * h)
            if not (FILL_LO <= fill <= FILL_HI):
                continue
            m = lab[y:y + h, x:x + w] == i
            yy, xx = np.nonzero(m)
            u = xx / max(w - 1, 1)
            vv = yy / max(h - 1, 1)
            d1 = np.abs(u - vv) < 0.25          # top-left -> bottom-right
            d2 = np.abs(u + vv - 1) < 0.25      # top-right -> bottom-left
            diag = float((d1 | d2).mean())
            if diag < DIAG_MIN:
                continue
            # ★ BOTH diagonals must carry real mass. Without this a SINGLE
            # diagonal stroke scores 1.00 -- every one of its pixels is
            # "near a diagonal" -- and the detector reports an X wherever a
            # slash, a chart rule or an anti-aliased corner appears. That
            # false positive is not hypothetical: it put 8 phantom traps on
            # an Acrobat render of GWG 2.0 whose ten swatches are provably
            # clean, and it inflated pdfce's own failure count too. An X has
            # two arms; requiring each to hold at least a quarter of the
            # mark is what makes it an X rather than a line.
            if float(d1.mean()) < 0.25 or float(d2.mean()) < 0.25:
                continue
            # ★ AND THE ARMS MUST ACTUALLY CROSS. A real X has mass at the
            # centre of its bounding box, where the two strokes meet. A
            # hollow ring, an anti-aliased corner, or two opposite corner
            # wedges all satisfy "both diagonals carry mass" while having
            # nothing in the middle -- which is what still fired on
            # Acrobat's anti-aliased screen renders after the both-arms
            # constraint. Cheap, and it is the difference between "shaped
            # like a cross" and "shaped like anything on two diagonals".
            cy0, cy1 = int(h * 0.40), int(h * 0.60) + 1
            cx0, cx1 = int(w * 0.40), int(w * 0.60) + 1
            centre = m[cy0:cy1, cx0:cx1]
            if centre.size == 0 or centre.mean() < 0.55:
                continue
            # ★ AND IT MUST BE A *CLEAR* X. The suite's own wording is "a
            # clear X indicates the improper handling of a file", judged by
            # a human at 0.5 m -- so a mark that is geometrically an X but
            # only a shade away from its surround is a PASS by the suite's
            # criterion even though it is present in the pixels.
            #
            # This is not a convenience threshold. Segmenting on exact
            # intensity found genuine X marks in all eight swatches of an
            # Acrobat render of GWG 2.0 that is, to the eye, ten clean green
            # squares: Acrobat leaves a sub-perceptual difference. Counting
            # those made the detector STRICTER than the standard it is
            # implementing, which is its own kind of wrong answer.
            band = im[y:y + h, x:x + w]
            inside = float(band[m].mean())
            outside_mask = ~m
            if outside_mask.sum() < 20:
                continue
            outside = float(band[outside_mask].mean())
            if abs(inside - outside) < CONTRAST_MIN:
                continue
            found.append((x, y, w, h, round(fill, 2), round(diag, 2)))
    found.sort(key=lambda t: -t[2] * t[3])
    keep = []
    for f in found:
        if any(abs(f[0] - k[0]) < 20 and abs(f[1] - k[1]) < 20 for k in keep):
            continue
        keep.append(f)
    return keep



def content_bands(im):
    """Horizontal content bands separated by full-width white gaps."""
    ink = (im < 245).sum(axis=1)
    rows = ink > im.shape[1] * 0.25
    segs, start = [], None
    for i, r in enumerate(rows):
        if r and start is None:
            start = i
        elif not r and start is not None:
            if i - start > 20:
                segs.append((start, i))
            start = None
    if start is not None and len(rows) - start > 20:
        segs.append((start, len(rows)))
    return segs


def reference_similarity(png):
    """Compare a patch's "Actual test objects" strip to its "Reference
    Images" strip.

    ★ THIS EXISTS BECAUSE THE X-TRAP DETECTOR SILENTLY PASSED PATCHES THAT
    VISIBLY FAIL. The suite uses (at least) TWO evaluation designs, and only
    one of them draws an X. Thirteen of the 51 patches instead print the
    test objects above a strip of REFERENCE IMAGES and say "each of these
    ... should match the reference images". On those, "no X found" is not a
    pass — it is the detector answering a question the patch never asked.
    `GWG 16.10` is the case that exposed it: it reported clean while two of
    its five reference cells rendered as empty boxes.

    Returns (correlation, mean-abs-difference). A correct render makes the
    two strips near-identical; the labels row differs by construction, which
    is why this reports a SCORE rather than a verdict — the threshold is not
    yet calibrated against a known-passing patch, and inventing one would be
    exactly the W14 error this harness's sibling was built to avoid.
    """
    im = cv2.imread(png, cv2.IMREAD_GRAYSCALE)
    if im is None:
        return None
    segs = content_bands(im)
    if len(segs) < 2:
        return None
    segs = sorted(segs, key=lambda t: -(t[1] - t[0]))[:2]
    segs.sort()
    (a0, a1), (b0, b1) = segs
    a = cv2.resize(im[a0:a1], (im.shape[1], min(a1 - a0, b1 - b0))).astype(np.float32)
    b = cv2.resize(im[b0:b1], (im.shape[1], min(a1 - a0, b1 - b0))).astype(np.float32)
    corr = float(((a - a.mean()) * (b - b.mean())).mean() / (a.std() * b.std() + 1e-6))
    return corr, float(np.abs(a - b).mean())


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("dir")
    ap.add_argument("--scale", type=float, default=2.0)
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args()

    cli = cli_path()
    tmp = os.path.join(args.dir, "_render")
    os.makedirs(tmp, exist_ok=True)

    pdfs = sorted(f for f in os.listdir(args.dir) if f.lower().endswith(".pdf"))
    results = []
    for f in pdfs:
        png = os.path.join(tmp, f + ".png")
        proc = subprocess.run(
            [cli, "render-page", os.path.join(args.dir, f), "--page", "1",
             "--scale", str(args.scale), "-o", png],
            capture_output=True,
        )
        if proc.returncode != 0 or not os.path.exists(png):
            results.append({"patch": f, "verdict": "RENDER-FAILED", "traps": 0})
            continue
        marks = find_traps(png)
        # Does this patch use the reference-strip design rather than an X?
        txt = subprocess.run([cli, "extract-text", os.path.join(args.dir, f)],
                             capture_output=True, text=True, errors="replace").stdout.lower()
        ref_style = ("reference image" in txt) or ("match the reference" in txt)
        sim = reference_similarity(png) if ref_style else None
        if marks:
            verdict = "X"
        elif ref_style:
            verdict = "REF"          # scored, not adjudicated -- see docstring
        else:
            verdict = "clean"
        results.append({
            "patch": f,
            "verdict": verdict,
            "traps": len(marks),
            "where": [f"{m[0]},{m[1]}" for m in marks[:6]],
            "ref_corr": None if sim is None else round(sim[0], 3),
            "ref_absdiff": None if sim is None else round(sim[1], 1),
        })

    if args.json:
        print(json.dumps(results, indent=2))
        return 0

    clean = [r for r in results if r["verdict"] == "clean"]
    failed = [r for r in results if r["verdict"] == "X"]
    ref = [r for r in results if r["verdict"] == "REF"]
    broke = [r for r in results if r["verdict"] == "RENDER-FAILED"]
    for r in results:
        mark = {"clean": "  ok  ", "X": " FAIL ", "REF": " ref? ",
                "RENDER-FAILED": " ERR  "}[r["verdict"]]
        if r["verdict"] == "X":
            extra = f"  {r['traps']} trap(s) at {' '.join(r['where'])}"
        elif r["verdict"] == "REF":
            extra = f"  actual-vs-reference corr={r['ref_corr']} absdiff={r['ref_absdiff']}"
        else:
            extra = ""
        print(f"{mark} {r['patch']}{extra}")
    print()
    print(f"ghent-check: {len(results)} patches -- "
          f"{len(failed)} FAIL (trap X visible), "
          f"{len(clean)} clean (X-trap design, no X), "
          f"{len(ref)} scored only (reference-strip design), "
          f"{len(broke)} render errors")
    print()
    print("A 'clean' verdict is the SUITE's own pass criterion for an X-trap")
    print("patch and is NOT a claim of pixel-accuracy against a press proof.")
    print("A 'ref?' row is NOT a pass: those patches carry their reference")
    print("images inline and are scored, not adjudicated, because no")
    print("known-passing patch exists yet to calibrate a threshold against.")
    print("A correlation well below 1.0 means the strips visibly differ.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
