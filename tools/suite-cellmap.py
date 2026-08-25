#!/usr/bin/env python3
"""Map each suite transparency-patch cell to the blend mode that governs it.

WHY THIS EXISTS
---------------
`tools/suite-cell-probe.py` says a trap fired at device (204, 106) and what
colour it is. It cannot say WHICH BLEND MODE that cell tests, and without that
a trap count is not a diagnosis.

The obvious shortcut is cell-pitch arithmetic: the cells are 22.678 pt squares
on a 31.68 pt pitch, the render is at scale 2.0, so cell index = position / 63.
**That shortcut produced a wrong diagnosis on 2026-08-18** -- not wrong about
positions, which it got right, but wrong in the story built on top of them,
which is the combination that survives a sanity check. It also silently
assumes every patch lays its modes out in the same order, and the ICCBased
patches do not label them the same way.

So this tool resolves the mapping from the file instead:

  * inflate every stream and rebuild an object table, including objects that
    live inside object streams (the suite files put their resources there);
  * collect `/ExtGState` name -> `/BM`, and `/XObject` name -> `/BBox` + `/Matrix`;
  * walk the content stream tracking the CTM through `q` / `Q` / `cm`;
  * at each `/Xnn Do`, compose `/Matrix` with the CTM, push `/BBox` through it,
    and flip into device pixels at the harness's scale.

Output is one line per painted cell, sorted top-to-bottom then left-to-right:

    device~( 203, 105)   X10    GS39  Hue

Cross-reference those coordinates against `suite-cell-probe.py`'s trap
positions (they agree to within a pixel or two -- the probe reports the trap
mark's bounding box, this reports the XObject's).

KNOWN LIMITATION -- READ BEFORE TRUSTING THE OUTPUT
---------------------------------------------------
**Resource names are scoped per resource dictionary, and this tool flattens
them into one namespace.** Where a file has several resource dictionaries that
each define `/GS1`, later definitions overwrite earlier ones and the reported
blend mode for some cells will be wrong or `?`.

Measured 2026-08-18: `PCS1_160` and `PCS3_164` resolve cleanly and completely.
`PCS1_161` and `PCS1_162` do NOT -- they report `?` for most cells and repeat
device positions, which is the symptom. **Those two patches are unmapped.**
Fixing it means threading the resource dictionary through the content walk
rather than pre-flattening, which is the correct design and is not done here.

Do not read a `?` as "no blend mode set". Read it as "this tool could not tell".

USAGE
-----
    python tools/suite-cellmap.py <patch.pdf>

The corpus lives OUTSIDE the repository (test-corpus rules, `docs/LEGAL.md`
§5). Nothing here writes anything.
"""

import re, sys, zlib

path = sys.argv[1]
data = open(path, "rb").read()


def inflate_all(buf):
    out = []
    for m in re.finditer(rb"stream\r?\n", buf):
        s = m.end()
        e = buf.find(b"endstream", s)
        try:
            out.append(zlib.decompress(buf[s:e]))
        except Exception:
            pass
    return out


streams = inflate_all(data)

# --- object table: number -> raw dict text, from both plain objects and objstms
objs = {}
for m in re.finditer(rb"(\d+)\s+\d+\s+obj(.*?)(?:endobj)", data, re.S):
    objs[int(m.group(1))] = m.group(2)
for st in streams:
    # object stream: "N off N off ... <<dict>><<dict>>"
    head = re.match(rb"((?:\d+\s+\d+\s+)+)", st)
    if not head:
        continue
    nums = [int(x) for x in head.group(1).split()]
    body = st[head.end():]
    pairs = list(zip(nums[0::2], nums[1::2]))
    for i, (num, off) in enumerate(pairs):
        end = pairs[i + 1][1] if i + 1 < len(pairs) else len(body)
        objs.setdefault(num, body[off:end])

# --- resources: ExtGState name -> blend mode ; XObject name -> obj number
res_gs, res_xo = {}, {}
for num, body in objs.items():
    for m in re.finditer(rb"/ExtGState<<(.*?)>>", body, re.S):
        for name, ref in re.findall(rb"/(\w+)\s+(\d+)\s+0\s+R", m.group(1)):
            res_gs[name.decode()] = int(ref)
    for m in re.finditer(rb"/XObject<<(.*?)>>", body, re.S):
        for name, ref in re.findall(rb"/(\w+)\s+(\d+)\s+0\s+R", m.group(1)):
            res_xo[name.decode()] = int(ref)

gs_bm = {}
for name, ref in res_gs.items():
    b = objs.get(ref, b"")
    m = re.search(rb"/BM\s*/(\w+)", b)
    ca = re.search(rb"/ca\s+([\d.]+)", b)
    if m:
        gs_bm[name] = m.group(1).decode()
    elif ca:
        gs_bm[name] = f"ca={ca.group(1).decode()}"

# XObject BBox/Matrix
xo_geo = {}
for name, ref in res_xo.items():
    b = objs.get(ref, b"")
    bb = re.search(rb"/BBox\s*\[([-\d.\s]+)\]", b)
    mx = re.search(rb"/Matrix\s*\[([-\d.\s]+)\]", b)
    if bb:
        xo_geo[name] = (
            [float(x) for x in bb.group(1).split()],
            [float(x) for x in mx.group(1).split()] if mx else [1, 0, 0, 1, 0, 0],
        )


def mul(a, b):
    return [
        a[0] * b[0] + a[1] * b[2], a[0] * b[1] + a[1] * b[3],
        a[2] * b[0] + a[3] * b[2], a[2] * b[1] + a[3] * b[3],
        a[4] * b[0] + a[5] * b[2] + b[4], a[4] * b[1] + a[5] * b[3] + b[5],
    ]


def apply(m, x, y):
    return (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])


# page height for the y flip; scale 2.0 as the harness uses
mb = re.search(rb"/MediaBox\s*\[([-\d.\s]+)\]", data)
page_h = float(mb.group(1).split()[3]) if mb else 792.0
SCALE = 2.0

# --- walk every content-bearing stream
tokre = re.compile(rb"(/\w+|[-\d.]+|[A-Za-z'\"*]+)")
for st in streams:
    if b" Do" not in st or b" gs" not in st:
        continue
    stack, ctm, cur_gs = [], [1, 0, 0, 1, 0, 0], None
    ops = []
    toks = tokre.findall(st)
    for i, t in enumerate(toks):
        if t == b"q":
            stack.append((ctm[:], cur_gs))
        elif t == b"Q" and stack:
            ctm, cur_gs = stack.pop()
            ctm = ctm[:]
        elif t == b"cm" and i >= 6:
            nums = [float(x) for x in toks[i - 6:i]]
            ctm = mul(nums, ctm)
        elif t == b"gs" and i >= 1:
            cur_gs = toks[i - 1].decode().lstrip("/")
        elif t == b"Do" and i >= 1:
            name = toks[i - 1].decode().lstrip("/")
            if name in xo_geo:
                bbox, mtx = xo_geo[name]
                full = mul(mtx, ctm)
                pts = [apply(full, bbox[0], bbox[1]), apply(full, bbox[2], bbox[3])]
                xs = sorted(p[0] for p in pts)
                ys = sorted(p[1] for p in pts)
                dx = xs[0] * SCALE
                dy = (page_h - ys[1]) * SCALE
                ops.append((round(dx), round(dy), name, cur_gs,
                            gs_bm.get(cur_gs, "?")))
    if ops:
        for o in sorted(ops, key=lambda z: (z[1], z[0])):
            print(f"  device~({o[0]:4d},{o[1]:4d})  {o[2]:>4}  {str(o[3]):>6}  {o[4]}")
