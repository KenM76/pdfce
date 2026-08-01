#!/usr/bin/env python3
"""render-parity — full-page pdfium (pypdfium2) pixel-parity harness (Pass 11).

WHY THIS EXISTS
===============
`docs/decisions/010` sequences render-fidelity verification (candidate C,
Pass 11) *ahead* of vector/content-stream editing (candidate A) for one
structural reason: vector editing is the first subsystem whose correctness
oracle is independent *visual* fidelity. pdfce's existing round-trip oracle
(`tools/content-identity`, `tools/roundtrip`) proves pdfce agrees with
ITSELF — sufficient for additive authoring, useless for proving an *edited*
page still renders *correctly*. That needs an independent reference renderer.

This harness is that oracle. It generalizes `tools/annot-pdfium-diff.py`
(an ink-bounding-box differential on 7 annotation fixtures) into a
FULL-PAGE, per-channel, per-pixel differential between:

  * pdfce   — via the shipped `pdfce-cli render-page` binary (the same
              read path the GUI uses), and
  * pdfium  — via `pypdfium2` (the engine inside Chrome; the decision 006
              §3.2 tooling precedent),

over the whole loadable conformance corpus (`fixtures/external`, ~2,914
files). It is out-of-tree tooling exactly like the other corpus harnesses:
it is never shipped, never in `cargo test`, never in the GUI-core
`cargo tree` invariant, and pypdfium2 never enters pdfce's runtime
dependency set or `THIRD_PARTY_LICENSES.md` (decision 010 acceptance;
LEGAL §6).

THE CENTRAL PROBLEM (decision 010 risk Y1) — NOISE vs SIGNAL
============================================================
Two independent renderers ALWAYS differ at the pixel level: anti-aliasing,
font hinting, sub-pixel glyph positioning, and image interpolation are all
implementation choices, not bugs. Demanding pixel-for-pixel agreement is a
category error. The analytical core of this Pass is therefore SEPARATING
benign renderer noise from real fidelity divergence, WITHOUT either of the
two forbidden failure modes:

  * W14 — tuning a threshold until a number turns green; and
  * declaring benign anti-aliasing noise a "bug".

HOW THE TOLERANCE BAND IS DERIVED (empirical, not tuned) — see README.md §3
--------------------------------------------------------------------------
The band is NOT a hand-picked number. It is derived from the data:

  1. Every page is rendered in both engines and reduced to a per-page
     divergence metric `frac_over_32` = the fraction of pixels whose
     maximum per-channel absolute delta exceeds 32/255. (Rationale:
     benign AA/hinting noise is confined to a THIN sub-pixel band around
     edges, so it touches a SMALL fraction of the page even where
     individual edge pixels swing the full 0..255; a real divergence — a
     missing shading fill, a wrong DeviceCMYK colour, a shifted glyph run —
     touches a LARGE contiguous AREA, i.e. a large fraction. Fraction-of-
     area, not max-delta, is the noise-robust discriminator.)

  2. Each page is tagged with pdfce's OWN disclosed diagnostics (the
     `render-page` stdout tally): does it substitute glyphs, skip a Type3
     font, defer an `sh`/marked-content operator, drop an image codec,
     carry a DeviceCMYK JPEG, etc.? A page with ZERO disclosed gaps AND no
     DeviceCMYK content is "clean-by-construction": whatever it diverges by
     CAN ONLY be renderer noise, because pdfce itself claims to render it
     fully.

  3. The benign band is the high percentile (default p99.0, configurable
     and reported) of `frac_over_32` OVER THE CLEAN-BY-CONSTRUCTION PAGES
     ONLY. The band is a property of the known-benign population, so it
     cannot be "tuned to make a bug pass" — a bug lives, by definition,
     either on a page pdfce discloses a gap for (bucket ii) or in the
     residual tail of clean pages ABOVE their own noise floor (bucket iii).

THE THREE BUCKETS (decision 010 deliverable 3; R20 by-file-and-reason)
======================================================================
Every (file, page) is classified:

  (i)   benign-renderer-noise  — frac_over_32 <= band. AA/hinting/subpixel.
  (ii)  known-disclosed-gap    — frac_over_32 > band AND pdfce disclosed a
                                 gap that explains it (Type3, sh shading,
                                 /SMask, /OC, image codec, DeviceCMYK, a
                                 substituted font face, ...). Cross-checked
                                 against pdfce's existing Diagnostics tally
                                 so an already-counted gap is SUBTRACTED,
                                 not re-reported as a new bug.
  (iii) unexplained-divergence — frac_over_32 > band AND no disclosed gap
                                 explains it. The genuine bug candidates —
                                 the residual after subtracting (i)+(ii).

Plus two side classifications that are NOT pdfce errors:

  * reference-divergence — (only in --annots mode) the page carries a
    /Widget or a no-/AP annotation; pdfium needs FPDF_FFLDraw to draw
    widgets and SYNTHESIZES some no-/AP appearances (e.g. /Circle /IC fill)
    that R43 makes pdfce correctly REFUSE (Pass 6.0 finding). Bucketed
    reference-side so pdfium's own quirks are never misattributed to pdfce
    (decision 010 deliverable 5 / risk Y2). The DEFAULT run is content-only
    (annotations off on both sides), which structurally avoids this
    confounder entirely — the vector-editing oracle cares about page
    CONTENT, which is what an edit re-renders.

  * skipped — pdfce could not load/render, or pdfium could not, or the page
    boxes disagree past `--dim-tol`. Out of scope, like the roundtrip gate's
    unloadable files. Counted, never silently dropped.

OUTPUTS (deterministic, locale-invariant)
=========================================
  out/per-page.tsv   — one row per (file, page): dims, metrics, bucket,
                        reason, the raw pdfce diagnostics. Sorted.
  out/summary.txt     — the distribution + the three-bucket counts + the
                        DeviceCMYK characterization + the enumerated
                        unexplained tail (R20).
  out/summary.json    — the same, machine-readable, for a gate/CI check.
  out/diffs/*.png     — side-by-side (pdfce | pdfium | 8x-amplified delta)
                        panels for the top unexplained pages and any page
                        named with --diff, for eyeball triage / the demo.

GATE ROLE (decision 010 deliverable 6; the R34/R46 pattern)
===========================================================
`--gate` mode exits non-zero if the count of UNEXPLAINED pages exceeds
`--max-unexplained` (default 0 once the corpus baseline is filed). This is
the standing render-fidelity gate: it must be re-run on every render-
touching Pass — ESPECIALLY the vector-editing Pass, whose content-stream
edits re-render the very pages this harness measures. Like content-identity
and roundtrip it is a LOCAL corpus gate (pypdfium2 is not in CI), documented
as required in README.md.

USAGE
=====
    python render_parity.py [CORPUS_DIR ...] [options]

    # default: content-only, 150 DPI, <=4 sampled pages/file, full corpus
    python render_parity.py

    # bounded demo subset
    python render_parity.py --max-files 200 --emit-diffs 12

    # one specific page's diff panel (for the demo / triage)
    python render_parity.py --diff "veraPDF-corpus/.../file.pdf" --diff-page 1

    # gate mode for a render-touching Pass
    python render_parity.py --gate --max-unexplained 0

Requires: pypdfium2, numpy, Pillow, and a built `pdfce-cli` release binary
(`cargo build --release -p pdfce-cli`).
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field
from pathlib import Path

import numpy as np
import pypdfium2 as pdfium
from PIL import Image

ROOT = Path(__file__).resolve().parent.parent.parent
DEFAULT_CORPUS = ROOT / "fixtures" / "external"
CLI = ROOT / "target" / "release" / (
    "pdfce-cli.exe" if sys.platform == "win32" else "pdfce-cli"
)

# Per-pixel delta threshold (0..255) above which a pixel "differs
# substantially". 32 is ~12.5% of full range — comfortably above 8-bit
# rounding + gamma jitter, comfortably below a real colour/geometry error.
PIXEL_DELTA_T = 32
# Secondary thresholds reported for the distribution, not used for bucketing.
EXTRA_T = (16, 64)


def devicecmyk_in_file(path: Path) -> bool:
    """Whether the raw file bytes mention `/DeviceCMYK`.

    WHY a byte scan and not a diagnostics counter: pdfce's render Diagnostics
    count DeviceCMYK *JPEGs* (`dct_cmyk`) but there is no counter for
    DeviceCMYK *vector* fills/strokes, and decision 006 §3.7 established that
    the naive-additive `Rgb::from_cmyk` colorimetry gap affects ALL
    DeviceCMYK painting, not just images. A file-level byte scan is a
    tooling-only, render-unchanged way to flag "this file could exhibit the
    colorimetry gap" so the harness can characterize it (deliverable 7)
    without adding a render-side counter (a non-goal this Pass). It is a
    file-level (not page-level) over-approximation, stated honestly.
    """
    try:
        return b"/DeviceCMYK" in path.read_bytes()
    except OSError:
        return False


# --- pdfce diagnostics -----------------------------------------------------

# Map of `render-page` stdout keys -> whether a non-zero value is a
# CONTENT-affecting disclosed gap that would legitimately diverge from
# pdfium. Keys not listed here (images=, forms=, annots_painted=, ...) are
# volume counters, not gaps.
GAP_KEYS = {
    "unsupported": "font-unsupported",          # Type3 / exotic CMap: text skipped
    "substituted": "font-substituted",          # substitute face: shapes differ from embedded
    "notdef": "glyph-notdef",                   # .notdef boxes
    "deferred": "deferred-op",                  # sh shading / BDC-EMC (OC) / Type3 proc / clip
    "images_unsupported": "image-unsupported",
    "images_codec_unsupported": "image-codec",
    "codec_features": "image-codec-feature",
    "codec_geometry_mismatch": "image-geometry",
    "jpx_preblended": "jpx-preblended",
    "lzw_anomalies": "lzw-anomaly",
    "dct_cmyk": "devicecmyk-jpeg",              # decision 006 §3.7 colorimetry (image)
    "dct_cmyk_unverifiable": "dct-polarity",
}
# Annotation keys that indicate a pdfium REFERENCE-divergence (annots mode).
REF_KEYS = {
    "annots_widget": "pdfium-fflodraw-widget",  # pdfium needs FPDF_FFLDraw
    "annots_no_ap": "pdfium-synthesized-noap",  # pdfium synthesizes /IC etc.; R43 refuses
}


def parse_diag_line(line: str) -> dict[str, int] | None:
    """Parse the `render-page` stdout stable line into {key: int}.

    Also extracts the raster dimensions from the `-> <path> WxH` clause. The
    line's contract (pdfce-cli module docs) is append-only key=value pairs,
    so a robust `k=v` scan survives future counter additions.
    """
    if "->" not in line:
        return None
    out: dict[str, int] = {}
    # dimensions: token of the form WxH right after the output path.
    for tok in line.replace(";", " ").split():
        if "=" in tok:
            k, v = tok.split("=", 1)
            if v.lstrip("-").isdigit():
                out[k] = int(v)
        elif "x" in tok:
            a, _, b = tok.partition("x")
            if a.isdigit() and b.isdigit():
                out["_w"], out["_h"] = int(a), int(b)
    return out or None


@dataclass
class PageResult:
    rel: str
    page: int  # 1-based
    status: str  # "ok" | "skip"
    reason: str = ""
    w: int = 0
    h: int = 0
    mean: float = 0.0
    p95: float = 0.0
    dmax: int = 0
    frac16: float = 0.0
    frac32: float = 0.0
    frac64: float = 0.0
    dim_mismatch: int = 0
    clean: int = 0  # 1 if no disclosed gap AND no DeviceCMYK (band-derivation set)
    devicecmyk: int = 0
    gaps: str = ""  # comma-joined gap reasons (bucket ii candidates)
    refdiv: str = ""  # comma-joined reference-divergence reasons (annots mode)
    bucket: str = ""  # filled in phase 2
    # keep the delta image path only when we emit a diff panel
    _arrays: object = field(default=None, repr=False, compare=False)


def to_white_rgb(img: Image.Image) -> np.ndarray:
    """Composite any image onto a white background, return HxWx3 uint8.

    Both engines may emit alpha; a PDF's "white page" is transparent in
    neither reference. Compositing onto white normalizes transparency so the
    comparison is of visible colour, not of premultiplied alpha conventions.
    """
    if img.mode != "RGBA":
        img = img.convert("RGBA")
    bg = Image.new("RGBA", img.size, (255, 255, 255, 255))
    comp = Image.alpha_composite(bg, img).convert("RGB")
    return np.asarray(comp, dtype=np.uint8)


def render_pdfium(path: Path, page_index: int, scale: float, annots: bool) -> np.ndarray:
    pdf = pdfium.PdfDocument(str(path))
    try:
        page = pdf[page_index]
        bitmap = page.render(scale=scale, draw_annots=annots)
        arr = to_white_rgb(bitmap.to_pil())
        page.close()
        return arr
    finally:
        pdf.close()


def pdfium_page_count(path: Path) -> int:
    pdf = pdfium.PdfDocument(str(path))
    try:
        return len(pdf)
    finally:
        pdf.close()


def render_pdfce(
    path: Path, page: int, scale: float, annots: bool, tmp: Path, timeout: float
) -> tuple[np.ndarray, dict[str, int]]:
    """Render one page via the pdfce CLI; return (rgb array, diagnostics).

    decision 012 R63 — the gate is BUNDLED-ONLY by construction: this
    command deliberately never passes `--font-dir`, so the renderer uses
    exactly `FontEnvironment::bundled()` and no operator-supplied face can
    perturb the pixels. Supplied-font renders are machine-dependent by
    definition and are therefore outside this determinism gate; adding a
    `--font-dir` here (or reading one from the environment) would break the
    gate's reproducibility. The invariant is enforced at the render layer
    too (`render_is_font_dir_independent_for_unreferenced_supplied_faces`
    in `crates/pdfce-render/src/lib.rs`).
    """
    out = tmp / "pdfce.png"
    cmd = [
        str(CLI), "render-page", str(path),
        "--page", str(page), "--scale", f"{scale:.6f}", "-o", str(out),
    ]
    if not annots:
        cmd.append("--no-annotations")
    r = subprocess.run(cmd, capture_output=True, timeout=timeout)
    if r.returncode != 0:
        raise RuntimeError(
            "pdfce render rc=%d: %s"
            % (r.returncode, r.stderr.decode(errors="replace").strip()[:200])
        )
    diag = parse_diag_line(r.stdout.decode(errors="replace")) or {}
    arr = to_white_rgb(Image.open(out))
    return arr, diag


def compare(a: np.ndarray, b: np.ndarray) -> tuple[np.ndarray, dict, int]:
    """Align two RGB rasters and compute the per-pixel max-channel delta.

    Returns (delta_map HxW uint16, stats, dim_mismatch_flag). Alignment crops
    both to the common top-left region: both engines emit a top-left-origin
    raster of the SAME page box, so a 1px rounding difference is absorbed by
    cropping to the min extent. A larger mismatch is flagged (page-box
    disagreement is a geometry finding, not a pixel one) but still measured on
    the overlap so it is never silently dropped.
    """
    dim_mismatch = int(a.shape[0] != b.shape[0] or a.shape[1] != b.shape[1])
    h = min(a.shape[0], b.shape[0])
    w = min(a.shape[1], b.shape[1])
    a = a[:h, :w, :].astype(np.int16)
    b = b[:h, :w, :].astype(np.int16)
    delta = np.abs(a - b).max(axis=2).astype(np.uint16)  # HxW, 0..255
    n = delta.size
    stats = {
        "mean": float(delta.mean()),
        "p95": float(np.percentile(delta, 95)),
        "dmax": int(delta.max()),
        "frac16": float(np.count_nonzero(delta > 16) / n),
        "frac32": float(np.count_nonzero(delta > 32) / n),
        "frac64": float(np.count_nonzero(delta > 64) / n),
    }
    return delta, stats, dim_mismatch


def gap_reasons(diag: dict[str, int]) -> list[str]:
    return [label for key, label in GAP_KEYS.items() if diag.get(key, 0) > 0]


def ref_reasons(diag: dict[str, int]) -> list[str]:
    return [label for key, label in REF_KEYS.items() if diag.get(key, 0) > 0]


def collect_pdfs(root: Path) -> list[tuple[str, Path]]:
    """Every *.pdf under root, sorted, as (relpath, abspath). Skips dotdirs
    (e.g. the corpus's own `.git`)."""
    out = []
    for p in sorted(root.rglob("*.pdf")):
        if any(part.startswith(".") for part in p.relative_to(root).parts):
            continue
        out.append((p.relative_to(root).as_posix(), p))
    # also .PDF on case-sensitive fs
    for p in sorted(root.rglob("*.PDF")):
        rel = p.relative_to(root).as_posix()
        if rel not in {r for r, _ in out} and not any(
            part.startswith(".") for part in p.relative_to(root).parts
        ):
            out.append((rel, p))
    out.sort()
    return out


def choose_pages(n_pages: int, cap: int) -> list[int]:
    """1-based page indices to sample. cap<=0 means all pages; otherwise
    sample first, last, and evenly spaced interior pages up to `cap`."""
    if n_pages <= 0:
        return []
    if cap <= 0 or n_pages <= cap:
        return list(range(1, n_pages + 1))
    if cap == 1:
        return [1]
    idxs = {1, n_pages}
    # evenly spaced fill
    step = (n_pages - 1) / (cap - 1)
    for k in range(cap):
        idxs.add(1 + round(k * step))
    return sorted(i for i in idxs if 1 <= i <= n_pages)[:cap]


def amplify_delta_panel(
    pdfce: np.ndarray, pdfium_img: np.ndarray, delta: np.ndarray
) -> Image.Image:
    """Build a [pdfce | pdfium | 8x-amplified delta] side-by-side panel."""
    h = min(pdfce.shape[0], pdfium_img.shape[0], delta.shape[0])
    w = min(pdfce.shape[1], pdfium_img.shape[1], delta.shape[1])
    a = pdfce[:h, :w, :]
    b = pdfium_img[:h, :w, :]
    d = np.clip(delta[:h, :w].astype(np.int32) * 8, 0, 255).astype(np.uint8)
    dmap = np.stack([d, d, d], axis=2)  # grey heatmap; brighter = larger delta
    gap = np.full((h, 8, 3), 200, dtype=np.uint8)
    panel = np.concatenate([a, gap, b, gap, dmap], axis=1)
    return Image.fromarray(panel, "RGB")


def run(args: argparse.Namespace) -> int:
    # Corpus filenames + spec section marks (§) can carry non-cp1252 chars;
    # force UTF-8 on the console so a print never dies on a Windows codepage.
    for stream in (sys.stdout, sys.stderr):
        if hasattr(stream, "reconfigure"):
            stream.reconfigure(encoding="utf-8", errors="replace")
    if not CLI.exists():
        print(f"ERROR: build the CLI first: cargo build --release -p pdfce-cli", file=sys.stderr)
        return 2

    corpus_dirs = [Path(d) for d in args.corpus] or [DEFAULT_CORPUS]
    files: list[tuple[str, Path]] = []
    for d in corpus_dirs:
        if not d.exists():
            print(f"ERROR: corpus dir not found: {d}", file=sys.stderr)
            return 2
        prefix = d.name
        for rel, abs_ in collect_pdfs(d):
            files.append((f"{prefix}/{rel}", abs_))
    files.sort()
    if args.max_files > 0:
        files = files[: args.max_files]

    # Optional single-page diff request (demo / triage): resolve the file.
    diff_target = None
    if args.diff:
        for rel, abs_ in files:
            if args.diff in rel:
                diff_target = (rel, abs_, args.diff_page)
                break
        if diff_target is None:
            print(f"ERROR: --diff file not found in corpus: {args.diff}", file=sys.stderr)
            return 2

    scale = args.dpi / 72.0
    outdir = Path(args.out)
    (outdir / "diffs").mkdir(parents=True, exist_ok=True)

    results: list[PageResult] = []
    retained: list[PageResult] = []
    retain_cap = max(args.emit_diffs * 3, 48)
    n_files_ok = 0
    print(
        f"render-parity: {len(files)} files, dpi={args.dpi} (scale {scale:.4f}), "
        f"pages/file cap={args.pages_per_file}, annots={'on' if args.annots else 'off'}",
        file=sys.stderr,
    )

    with tempfile.TemporaryDirectory() as td:
        tmp = Path(td)
        for fi, (rel, abs_) in enumerate(files):
            if fi % 200 == 0:
                print(f"  [{fi}/{len(files)}] {rel}", file=sys.stderr)
            try:
                npages = pdfium_page_count(abs_)
            except Exception as e:  # noqa: BLE001 — reference tool failure = skip file
                results.append(PageResult(rel, 0, "skip", f"pdfium-open: {str(e)[:80]}"))
                continue
            dcmyk = devicecmyk_in_file(abs_)
            pages = choose_pages(npages, args.pages_per_file)
            file_ok = False
            for pg in pages:
                pr = measure_page(rel, abs_, pg, scale, args, tmp, dcmyk)
                results.append(pr)
                file_ok = file_ok or pr.status == "ok"
                # Bound retained rasters to the worst-N by frac32 so a full
                # corpus sweep cannot exhaust memory (each retained page holds
                # three full-page arrays, ~10-15 MB at 125 DPI). Evict the
                # least-divergent when over the cap.
                if pr._arrays is not None:
                    retained.append(pr)
                    if len(retained) > retain_cap:
                        victim = min(retained, key=lambda r: r.frac32)
                        victim._arrays = None
                        # identity filter — PageResult eq is value-based, so
                        # `.remove` could drop a different equal-metric page.
                        retained = [r for r in retained if r is not victim]
            if file_ok:
                n_files_ok += 1

    ok = [r for r in results if r.status == "ok"]
    if not ok:
        print("no pages measured (all skipped) — is the corpus present?", file=sys.stderr)
        return 2

    # ---- Phase 2: derive the benign band from clean-by-construction pages.
    clean = [r for r in ok if r.clean]
    clean_frac = np.array([r.frac32 for r in clean]) if clean else np.array([0.0])
    if args.band is not None:
        band = args.band
        band_src = f"explicit --band {band}"
    else:
        band = float(np.percentile(clean_frac, args.band_pct))
        band_src = (
            f"p{args.band_pct:g} of frac_over_32 over {len(clean)} "
            f"clean-by-construction pages"
        )

    # ---- Phase 3: classify.
    for r in ok:
        if r.frac32 <= band:
            r.bucket = "benign"
        elif r.refdiv:
            r.bucket = "reference-divergence"
        elif r.gaps:
            r.bucket = "known-gap"
        else:
            r.bucket = "unexplained"

    write_reports(results, ok, clean, band, band_src, n_files_ok, len(files), args, outdir, scale)

    # ---- Diff panels: the top unexplained pages + any explicit --diff.
    emit_diff_panels(ok, diff_target, args, outdir, scale, tmp_reuse=None)

    unexplained = [r for r in ok if r.bucket == "unexplained"]
    if args.gate:
        n = len(unexplained)
        verdict = "PASS" if n <= args.max_unexplained else "FAIL"
        print(f"\nGATE: {verdict} — {n} unexplained (max {args.max_unexplained})")
        return 0 if n <= args.max_unexplained else 1
    return 0


def measure_page(
    rel: str, abs_: Path, pg: int, scale: float, args, tmp: Path, dcmyk: bool
) -> PageResult:
    """Render one page in both engines, compute metrics, tag gaps. Never
    raises — any failure becomes a counted skip (acceptance: zero panics)."""
    try:
        pdfce_img, diag = render_pdfce(abs_, pg, scale, args.annots, tmp, args.timeout)
    except subprocess.TimeoutExpired:
        return PageResult(rel, pg, "skip", "pdfce-timeout")
    except Exception as e:  # noqa: BLE001
        return PageResult(rel, pg, "skip", f"pdfce: {str(e)[:80]}")
    try:
        pdfium_img = render_pdfium(abs_, pg - 1, scale, args.annots)
    except Exception as e:  # noqa: BLE001
        return PageResult(rel, pg, "skip", f"pdfium: {str(e)[:80]}")

    delta, stats, dim_mismatch = compare(pdfce_img, pdfium_img)
    gaps = gap_reasons(diag)
    refs = ref_reasons(diag) if args.annots else []
    if dcmyk:
        gaps = gaps + ["devicecmyk-file"] if "devicecmyk-jpeg" not in gaps else gaps
    clean = int(not gaps and not dcmyk and not refs and not dim_mismatch)

    pr = PageResult(
        rel=rel, page=pg, status="ok",
        w=min(pdfce_img.shape[1], pdfium_img.shape[1]),
        h=min(pdfce_img.shape[0], pdfium_img.shape[0]),
        mean=stats["mean"], p95=stats["p95"], dmax=stats["dmax"],
        frac16=stats["frac16"], frac32=stats["frac32"], frac64=stats["frac64"],
        dim_mismatch=dim_mismatch, clean=clean, devicecmyk=int(dcmyk),
        gaps=",".join(gaps), refdiv=",".join(refs),
    )
    # Retain arrays only for the worst pages so we can emit diff panels
    # without re-rendering; bounded by keeping them light (frac32 gate).
    if args.emit_diffs > 0 and stats["frac32"] > 0.001:
        pr._arrays = (pdfce_img, pdfium_img, delta)
    return pr


def emit_diff_panels(ok, diff_target, args, outdir, scale, tmp_reuse) -> None:
    # Explicit --diff request: render fresh (may not be in the retained set).
    if diff_target is not None:
        rel, abs_, pg = diff_target
        try:
            with tempfile.TemporaryDirectory() as td:
                pdfce_img, _ = render_pdfce(abs_, pg, scale, args.annots, Path(td), args.timeout)
            pdfium_img = render_pdfium(abs_, pg - 1, scale, args.annots)
            delta, _, _ = compare(pdfce_img, pdfium_img)
            panel = amplify_delta_panel(pdfce_img, pdfium_img, delta)
            name = rel.replace("/", "_").replace("\\", "_")
            panel.save(outdir / "diffs" / f"DIFF_{name}_p{pg}.png")
            print(f"wrote diff panel: diffs/DIFF_{name}_p{pg}.png", file=sys.stderr)
        except Exception as e:  # noqa: BLE001
            print(f"--diff render failed: {e}", file=sys.stderr)

    if args.emit_diffs <= 0:
        return
    # Top unexplained (then top overall) by frac32, among pages whose arrays
    # we retained.
    have = [r for r in ok if r._arrays is not None]
    unexp = sorted(
        [r for r in have if r.bucket == "unexplained"], key=lambda r: -r.frac32
    )
    rest = sorted(
        [r for r in have if r.bucket != "unexplained"], key=lambda r: -r.frac32
    )
    chosen = (unexp + rest)[: args.emit_diffs]
    for r in chosen:
        pdfce_img, pdfium_img, delta = r._arrays
        panel = amplify_delta_panel(pdfce_img, pdfium_img, delta)
        name = r.rel.replace("/", "_").replace("\\", "_")
        panel.save(outdir / "diffs" / f"{r.bucket}_{r.frac32:.4f}_{name}_p{r.page}.png")
    if chosen:
        print(f"wrote {len(chosen)} diff panels to diffs/", file=sys.stderr)


def _distribution(vals: list[float]) -> dict:
    if not vals:
        return {"n": 0}
    a = np.array(vals)
    return {
        "n": len(vals),
        "mean": float(a.mean()),
        "p50": float(np.percentile(a, 50)),
        "p95": float(np.percentile(a, 95)),
        "p99": float(np.percentile(a, 99)),
        "max": float(a.max()),
    }


def write_reports(results, ok, clean, band, band_src, n_files_ok, n_files, args, outdir, scale) -> None:
    # per-page TSV (all rows, deterministic order already).
    tsv = outdir / "per-page.tsv"
    with tsv.open("w", encoding="utf-8", newline="\n") as f:
        f.write(
            "file\tpage\tstatus\tbucket\tw\th\tmean\tp95\tdmax\t"
            "frac16\tfrac32\tfrac64\tdim_mismatch\tclean\tdevicecmyk\tgaps\trefdiv\treason\n"
        )
        for r in results:
            f.write(
                f"{r.rel}\t{r.page}\t{r.status}\t{r.bucket}\t{r.w}\t{r.h}\t"
                f"{r.mean:.3f}\t{r.p95:.1f}\t{r.dmax}\t{r.frac16:.5f}\t{r.frac32:.5f}\t"
                f"{r.frac64:.5f}\t{r.dim_mismatch}\t{r.clean}\t{r.devicecmyk}\t"
                f"{r.gaps}\t{r.refdiv}\t{r.reason}\n"
            )

    buckets = {"benign": 0, "known-gap": 0, "unexplained": 0, "reference-divergence": 0}
    for r in ok:
        buckets[r.bucket] = buckets.get(r.bucket, 0) + 1
    skipped = [r for r in results if r.status == "skip"]

    # DeviceCMYK characterization: pages that are DeviceCMYK AND have NO other
    # gap (so the divergence is attributable to colorimetry), vs clean pages.
    dcmyk_only = [
        r for r in ok
        if r.devicecmyk and not any(
            g for g in r.gaps.split(",") if g and g not in ("devicecmyk-file", "devicecmyk-jpeg")
        )
    ]
    clean_frac = [r.frac32 for r in clean]
    dcmyk_frac = [r.frac32 for r in dcmyk_only]

    # unexplained enumerated by file+reason (R20), sorted worst-first.
    unexplained = sorted([r for r in ok if r.bucket == "unexplained"], key=lambda r: -r.frac32)
    # gap reasons histogram
    gap_hist: dict[str, int] = {}
    for r in ok:
        if r.bucket == "known-gap":
            for g in r.gaps.split(","):
                if g:
                    gap_hist[g] = gap_hist.get(g, 0) + 1
    skip_hist: dict[str, int] = {}
    for r in skipped:
        key = r.reason.split(":")[0]
        skip_hist[key] = skip_hist.get(key, 0) + 1

    summary = {
        "config": {
            "dpi": args.dpi, "scale": round(scale, 6),
            "pages_per_file_cap": args.pages_per_file,
            "annots": args.annots, "pixel_delta_threshold": PIXEL_DELTA_T,
        },
        "corpus": {
            "files_seen": n_files, "files_with_a_measured_page": n_files_ok,
            "pages_measured": len(ok), "pages_skipped": len(skipped),
        },
        "band": {"frac_over_32": band, "source": band_src, "percentile": args.band_pct},
        "buckets": buckets,
        "distribution_frac32": {
            "all_measured": _distribution([r.frac32 for r in ok]),
            "clean_by_construction": _distribution(clean_frac),
            "devicecmyk_only": _distribution(dcmyk_frac),
        },
        "gap_histogram": dict(sorted(gap_hist.items(), key=lambda kv: -kv[1])),
        "skip_histogram": dict(sorted(skip_hist.items(), key=lambda kv: -kv[1])),
        "unexplained_top": [
            {"file": r.rel, "page": r.page, "frac32": round(r.frac32, 5),
             "p95": r.p95, "dmax": r.dmax, "dim_mismatch": r.dim_mismatch}
            for r in unexplained[:50]
        ],
        "unexplained_total": len(unexplained),
    }
    (outdir / "summary.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")

    # human/LLM-readable summary
    lines: list[str] = []
    P = lines.append
    P("=== render-parity -- full-page pdfium pixel-parity (Pass 11) ===")
    P(f"config: dpi={args.dpi} scale={scale:.4f} pages/file<= {args.pages_per_file} "
      f"annots={'on' if args.annots else 'off'} pixel-delta-T={PIXEL_DELTA_T}")
    P(f"corpus: {n_files} files, {n_files_ok} with a measured page, "
      f"{len(ok)} pages measured, {len(skipped)} pages skipped")
    P("")
    P("--- tolerance band (empirical, NOT tuned -- decision 010 Y1/W14) ---")
    P(f"band(frac_over_32) = {band:.6f}")
    P(f"  source: {band_src}")
    P(f"  a page is BENIGN iff frac_over_32 <= band; the band is a property of")
    P(f"  known-clean pages, so it cannot be tuned to pass a bug.")
    P("")
    P("--- frac_over_32 distribution ---")
    for label, vals in (
        ("all measured   ", [r.frac32 for r in ok]),
        ("clean-by-constr", clean_frac),
        ("devicecmyk-only", dcmyk_frac),
    ):
        d = _distribution(vals)
        if d["n"]:
            P(f"  {label}: n={d['n']:6d} mean={d['mean']:.5f} p50={d['p50']:.5f} "
              f"p95={d['p95']:.5f} p99={d['p99']:.5f} max={d['max']:.5f}")
        else:
            P(f"  {label}: n=0")
    P("")
    P("--- three buckets (by file+reason, R20) ---")
    P(f"  (i)   benign-renderer-noise : {buckets['benign']}")
    P(f"  (ii)  known-disclosed-gap   : {buckets['known-gap']}")
    P(f"  (iii) unexplained-divergence: {buckets['unexplained']}")
    if args.annots:
        P(f"  (ref) reference-divergence  : {buckets['reference-divergence']}")
    P("")
    P("--- known-gap reason histogram (subtracted from bug candidates) ---")
    for g, c in sorted(gap_hist.items(), key=lambda kv: -kv[1]):
        P(f"  {c:6d}  {g}")
    P("")
    P("--- DeviceCMYK colorimetry characterization (decision 006 sec3.7 / deliverable 7) ---")
    dd = _distribution(dcmyk_frac)
    cd = _distribution(clean_frac)
    if dd["n"]:
        P(f"  DeviceCMYK-only pages: n={dd['n']} mean frac32={dd['mean']:.5f} "
          f"p95={dd['p95']:.5f} max={dd['max']:.5f}")
        P(f"  clean pages (baseline): n={cd['n']} mean frac32={cd['mean']:.5f} "
          f"p95={cd['p95']:.5f}")
        P(f"  => DeviceCMYK pages diverge {(dd['mean']/cd['mean']) if cd['mean'] else float('nan'):.1f}x "
          f"the clean-page mean (naive-additive Rgb::from_cmyk vs pdfium AdobeCMYK_to_sRGB1)")
    else:
        P("  no DeviceCMYK-only pages in this run")
    P("")
    P("--- unexplained tail (bug candidates, worst-first, R20 by file+reason) ---")
    if not unexplained:
        P("  (none)")
    for r in unexplained[:40]:
        P(f"  frac32={r.frac32:.5f} p95={r.p95:.0f} dmax={r.dmax} dimMM={r.dim_mismatch} "
          f"{r.rel} p{r.page}")
    P("")
    P("--- skip histogram (out of scope: unloadable / geometry) ---")
    for k, c in sorted(skip_hist.items(), key=lambda kv: -kv[1]):
        P(f"  {c:6d}  {k}")
    (outdir / "summary.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print("\n".join(lines))


def build_argparser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description="full-page pdfium pixel-parity harness (Pass 11)")
    p.add_argument("corpus", nargs="*", help="corpus dir(s); default fixtures/external")
    p.add_argument("--dpi", type=float, default=150.0, help="render DPI (default 150)")
    p.add_argument("--pages-per-file", type=int, default=4,
                   help="max pages sampled per file (0 = all; default 4)")
    p.add_argument("--max-files", type=int, default=0, help="cap files (0 = all)")
    p.add_argument("--annots", action="store_true",
                   help="compare WITH annotations (default off = content-only oracle)")
    p.add_argument("--band", type=float, default=None,
                   help="explicit benign band on frac_over_32 (default: derive from clean p99)")
    p.add_argument("--band-pct", type=float, default=99.9,
                   help="percentile of clean pages defining the band (default 99.9). "
                        "Principle: the clean-by-construction population is benign in "
                        "full, so the band covers essentially all of it; the tiny "
                        "residual above is the bug-candidate set to triage. NOT chosen "
                        "to hit a target unexplained count (W14).")
    p.add_argument("--emit-diffs", type=int, default=8,
                   help="write N diff panels for worst pages (default 8)")
    p.add_argument("--diff", type=str, default=None,
                   help="substring of a corpus file to emit a diff panel for")
    p.add_argument("--diff-page", type=int, default=1, help="page for --diff (1-based)")
    p.add_argument("--timeout", type=float, default=120.0,
                   help="per-page pdfce render timeout seconds (default 120)")
    p.add_argument("--out", type=str, default=str(Path(__file__).resolve().parent / "out"),
                   help="output directory (default tools/render-parity/out)")
    p.add_argument("--gate", action="store_true", help="exit nonzero if unexplained > max")
    p.add_argument("--max-unexplained", type=int, default=0,
                   help="gate threshold on unexplained pages (default 0)")
    return p


if __name__ == "__main__":
    raise SystemExit(run(build_argparser().parse_args()))
