#!/usr/bin/env python3
# =============================================================================
# realdrawings_smoke.py  --  PRIVATE, READ-ONLY, RESULTS-ONLY regression
#                            smoke-test over the operator's real CAD drawings.
# =============================================================================
#
# PURPOSE
# -------
# A "does the fix hold on the files the operator actually uses?" guard. It was
# built the day the subset-CIDFontType2 no-cmap TrueType font bug was fixed
# (the class that surfaced on real TOP Steel drawings): text that pdfce could
# not route through the correct glyph program was being SKIPPED rather than
# drawn. `pdfce-render` now attributes every unsupported font to a REASON
# (`UnusableProgram`, `Type3`, `NonIdentityCmap`, `VerticalWriting`,
# `CompositeNotEmbedded`, `UnknownSubtype`) and `pdfce-cli render-page` emits
# those reason counts on its stable stdout diagnostic line. This harness runs
# `render-page --page 1` over every drawing, parses that line, and aggregates
# the counts so we can answer:
#
#   1. Is the just-fixed class truly gone?  (unsupported_unusable_program == 0
#      across the whole corpus, and unsupported == 0 for the CIDFontType2
#      no-cmap files specifically.)
#   2. What OTHER real-world gaps exist now?  (Type3 fonts, non-Identity CMaps,
#      notdef glyphs, unsupported image codecs, load/render failures, tolerated
#      oddities.)  -> a prioritized "next real-world gap" list.
#
# HARD RULES  (LEGAL.md §5 + the project's test-corpus rule)
# ----------------------------------------------------------
# The drawings under R:\Products  (R: == D:\Stanley Dropbox\Resource\products)
# are PROPRIETARY (TOP Steel confidential). This harness:
#   * reads them IN PLACE -- never copies a proprietary file anywhere;
#   * renders each page 1 to a SINGLE throwaway temp PNG that is OVERWRITTEN
#     every file and DELETED at the end -- the pixels never enter the repo;
#   * emits ONLY diagnostics: counts, filenames, and the CLI's own diagnostic
#     line. No file content, no rendered image, ever reaches git.
# The report lands in ./out/ which is gitignored. NOTHING here is to be
# committed, and no proprietary path is hard-coded into anything tracked.
#
# INPUT / OUTPUT CONTRACT
# -----------------------
#   Input : a root dir of *.pdf (default R:\Products, recursive) + the RELEASE
#           pdfce-cli.exe (default D:\Dev\pdfce\target\release\pdfce-cli.exe).
#           It uses the ALREADY-BUILT exe; it never triggers a build (other
#           agents may be compiling -- rebuilding under them is forbidden).
#   Output: out/report.txt   -- human-readable aggregate + prioritized outliers
#           out/results.json  -- per-file records (machine-readable)
#           stdout            -- the same aggregate, for the dispatching agent.
#   Exit  : 0 always on a completed scan (this is a REPORT tool, not a gate --
#           a dirty corpus is a finding, not a failure). Non-zero only when the
#           harness itself cannot run (missing exe, missing root).
#
# PER-FILE ALGORITHM
# ------------------
#   render-page --page 1 -o <temp.png> <file>
#     exit 0  -> LOADED+RENDERED. Parse the stdout diagnostic line's trailing
#                "key=value key=value ..." run into a dict; record every count.
#     exit !0 -> FAILED. Classify from stderr: an open/parse error ("load
#                failure") vs a page-render error, plus subprocess timeouts.
#   (optional) extract-text --pages 1  -> cross-check: does a text-bearing file
#                actually yield characters? Records char count + U+FFFD count.
#
# The diagnostic line is parsed GENERICALLY (all key=int pairs), so if the CLI
# appends new counters later (its contract is append-never-reorder) this
# harness picks them up without edits.
#
# RE-RUN
# ------
#   python tools/realdrawings-smoke/realdrawings_smoke.py
#   # options:
#   #   --root <dir>        corpus root (default R:\Products)
#   #   --cli  <exe>        pdfce-cli.exe (default target/release build)
#   #   --cap  <N>          scan at most N files (0 = no cap; logged if hit)
#   #   --timeout <sec>     per-file subprocess timeout (default 120)
#   #   --no-extract-text   skip the extract-text cross-check (faster)
# =============================================================================

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
import time
from collections import Counter, defaultdict
from pathlib import Path

# Default corpus root. R: is a subst alias for D:\Stanley Dropbox\Resource, so
# the products live at R:\Products == D:\Stanley Dropbox\Resource\products.
DEFAULT_ROOT = r"R:\Products"
DEFAULT_CLI = r"D:\Dev\pdfce\target\release\pdfce-cli.exe"

# The trailing metrics run of the render-page stdout line is a space-separated
# sequence of key=<integer> tokens. This matches each token; we build a dict.
KV_RE = re.compile(r"(\w+)=(\d+)")

# The six by-reason unsupported-font counters (the new diagnostic). Their SUM
# equals `unsupported`. `unsupported_unusable_program` is the bucket that hid
# the CIDFontType2 no-cmap misroute -- watching it go to zero is the whole
# point of this run.
REASON_KEYS = [
    "unsupported_type3",
    "unsupported_noncmap",
    "unsupported_vertical",
    "unsupported_composite_not_embedded",
    "unsupported_unknown_subtype",
    "unsupported_unusable_program",
]

# Counters that mean "pdfce noticed something odd but tolerated it" (rendered
# anyway, possibly imperfectly). High counts here are a soft signal, not a bug.
TOLERATED_KEYS = [
    "codec_geometry_mismatch",
    "lzw_anomalies",
    "dct_cmyk_unverifiable",
    "jpx_preblended",
    "unknown",
    "deferred",
]


def parse_line(line: str) -> dict | None:
    """Extract the key=int metrics dict from a render-page stdout line.

    Returns None if the line isn't a result line (no 'rendered ... ->'). The
    WxH token also matches KV? No -- WxH is '200x120', not key=value, so it is
    ignored by KV_RE. Only genuine key=value pairs are captured.
    """
    if "rendered " not in line or "->" not in line:
        return None
    # Parse only the half after the '; ' separator so the path (which can
    # contain '=' in theory) never pollutes the metrics.
    tail = line.split(";", 1)[1] if ";" in line else line
    d = {k: int(v) for k, v in KV_RE.findall(tail)}
    return d or None


def run_one(cli: str, pdf: Path, tmp_png: str, timeout: int):
    """Render page 1 of one PDF. Returns a per-file record dict.

    status ∈ {'clean','dirty','load_fail','render_fail','timeout','no_line'}.
    """
    rec = {"file": str(pdf), "name": pdf.name}
    try:
        p = subprocess.run(
            [cli, "render-page", "--page", "1", "-o", tmp_png, str(pdf)],
            capture_output=True, text=True, timeout=timeout,
        )
    except subprocess.TimeoutExpired:
        rec["status"] = "timeout"
        return rec
    except OSError as e:
        rec["status"] = "load_fail"
        rec["error"] = f"harness-oserror: {e}"
        return rec

    rec["exit"] = p.returncode
    if p.returncode != 0:
        # stderr classifies: 'page N:' => render error; else doc open/parse.
        err = (p.stderr or "").strip().splitlines()
        msg = err[-1] if err else "(no stderr)"
        rec["error"] = msg
        rec["status"] = "render_fail" if re.search(r"page \d+:", msg) else "load_fail"
        return rec

    d = parse_line(p.stdout or "")
    if d is None:
        rec["status"] = "no_line"
        rec["error"] = "exit0 but no parsable diagnostic line"
        return rec

    rec["counts"] = d
    # 'clean' == every problem counter zero. We define the problem set as all
    # counters EXCEPT the benign informational ones (images/forms rendered,
    # annotation totals, substituted -- substitution is a faithful fallback,
    # not a gap; need_appearances is a doc property). Everything else at zero
    # means a faithful page-1 render.
    benign = {"images", "forms", "annots", "annots_painted", "annots_widget",
              "need_appearances", "substituted"}
    problem = {k: v for k, v in d.items()
               if k not in benign and not k.startswith("unsupported_") and v > 0}
    # unsupported_* are a breakdown of 'unsupported'; counting 'unsupported'
    # once (above) avoids double-flagging.
    rec["status"] = "clean" if not problem else "dirty"
    return rec


def run_extract(cli: str, pdf: Path, timeout: int):
    """extract-text --pages 1 cross-check. Returns (chars, replacement_chars)
    or None on any failure. A text-bearing drawing that yields 0 chars while
    render reported no unsupported fonts is itself a finding."""
    try:
        p = subprocess.run(
            [cli, "extract-text", "--pages", "1", str(pdf)],
            capture_output=True, text=True, timeout=timeout,
        )
    except (subprocess.TimeoutExpired, OSError):
        return None
    if p.returncode != 0:
        return None
    txt = p.stdout or ""
    return (len(txt), txt.count("\ufffd"))


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--root", default=DEFAULT_ROOT)
    ap.add_argument("--cli", default=DEFAULT_CLI)
    ap.add_argument("--cap", type=int, default=0, help="max files (0=all)")
    ap.add_argument("--timeout", type=int, default=120)
    ap.add_argument("--no-extract-text", action="store_true")
    args = ap.parse_args()

    here = Path(__file__).resolve().parent
    out_dir = here / "out"
    out_dir.mkdir(exist_ok=True)

    cli = args.cli
    if not Path(cli).exists():
        print(f"FATAL: pdfce-cli not found: {cli}", file=sys.stderr)
        return 2
    root = Path(args.root)
    if not root.exists():
        print(f"FATAL: corpus root not found: {root}", file=sys.stderr)
        return 2

    # Enumerate *.pdf recursively, case-insensitive (real files use .pdf/.PDF).
    all_pdfs = sorted(p for p in root.rglob("*") if p.suffix.lower() == ".pdf")
    total_found = len(all_pdfs)
    capped = False
    if args.cap and total_found > args.cap:
        all_pdfs = all_pdfs[: args.cap]
        capped = True

    # ONE throwaway PNG in the system temp dir -- overwritten every file,
    # deleted at the end. Proprietary pixels never touch the repo.
    fd, tmp_png = tempfile.mkstemp(suffix=".png", prefix="pdfce_smoke_")
    os.close(fd)

    records = []
    t0 = time.time()
    print(f"Scanning {len(all_pdfs)} PDF(s) under {root} "
          f"(found {total_found}{', CAPPED' if capped else ''}) ...",
          file=sys.stderr)
    for i, pdf in enumerate(all_pdfs, 1):
        rec = run_one(cli, pdf, tmp_png, args.timeout)
        if not args.no_extract_text and rec.get("status") in ("clean", "dirty"):
            et = run_extract(cli, pdf, args.timeout)
            if et is not None:
                rec["extract_chars"], rec["extract_replacement"] = et
        records.append(rec)
        if i % 25 == 0:
            print(f"  ... {i}/{len(all_pdfs)}", file=sys.stderr)
    elapsed = time.time() - t0

    try:
        os.remove(tmp_png)
    except OSError:
        pass

    # ---- Aggregate -----------------------------------------------------------
    by_status = Counter(r["status"] for r in records)
    ok = [r for r in records if r["status"] in ("clean", "dirty")]

    # Sum every counter across all rendered files.
    totals = defaultdict(int)
    for r in ok:
        for k, v in r.get("counts", {}).items():
            totals[k] += v

    # Outlier buckets (prioritized).
    unsupported_files = [r for r in ok if r["counts"].get("unsupported", 0) > 0]
    notdef_files = [r for r in ok if r["counts"].get("notdef", 0) > 0]
    imgcodec_files = [r for r in ok if r["counts"].get("images_codec_unsupported", 0) > 0]
    imgunsup_files = [r for r in ok if r["counts"].get("images_unsupported", 0) > 0]
    codecfeat_files = [r for r in ok if r["counts"].get("codec_features", 0) > 0]
    tolerated_files = [
        r for r in ok
        if any(r["counts"].get(k, 0) > 0 for k in TOLERATED_KEYS)
    ]
    fail_files = [r for r in records if r["status"] in
                  ("load_fail", "render_fail", "timeout", "no_line")]
    # extract-text cross-check: rendered without unsupported fonts yet 0 chars.
    empty_text = [r for r in ok
                  if r.get("extract_chars") == 0
                  and r["counts"].get("unsupported", 0) == 0]

    # Reason breakdown across the whole corpus.
    reason_totals = {k: totals.get(k, 0) for k in REASON_KEYS}

    # ---- Emit report ---------------------------------------------------------
    L = []
    L.append("=" * 78)
    L.append("pdfce real-drawings smoke-test  --  PRIVATE / READ-ONLY / RESULTS-ONLY")
    L.append("=" * 78)
    L.append(f"root            : {root}")
    L.append(f"cli             : {cli}")
    L.append(f"scanned         : {len(records)} of {total_found} found"
             + (f"  (CAPPED at {args.cap})" if capped else ""))
    L.append(f"coverage        : page 1 only (fast smoke; not a full-doc scan)")
    L.append(f"elapsed         : {elapsed:.0f}s")
    L.append("")
    L.append("-- LOAD / RENDER STATUS " + "-" * 54)
    for s in ("clean", "dirty", "load_fail", "render_fail", "timeout", "no_line"):
        if by_status.get(s):
            L.append(f"  {s:12s}: {by_status[s]}")
    clean_n = by_status.get("clean", 0)
    L.append(f"  => {clean_n}/{len(ok)} rendered files are CLEAN (all problem counters zero)")
    L.append("")
    L.append("-- FONT FIX VERIFICATION (the reason this harness exists) " + "-" * 20)
    L.append(f"  unsupported (fonts skipped, total across corpus) : {totals.get('unsupported',0)}")
    for k in REASON_KEYS:
        tag = "  <-- the just-fixed class" if k == "unsupported_unusable_program" else ""
        L.append(f"    {k:38s}: {reason_totals[k]}{tag}")
    if reason_totals["unsupported_unusable_program"] == 0:
        L.append("  RESULT: unsupported_unusable_program == 0 across the corpus.")
        L.append("          The subset-CIDFontType2 no-cmap TrueType class is GONE. [PASS]")
    else:
        L.append("  RESULT: unusable_program STILL NONZERO -- the fix did NOT fully hold. [FAIL]")
    L.append("")
    L.append("-- AGGREGATE COUNTERS (summed over all rendered pages) " + "-" * 23)
    for k in sorted(totals):
        if totals[k]:
            L.append(f"  {k:34s}: {totals[k]}")
    L.append("")
    L.append("-- OUTLIERS, PRIORITIZED " + "-" * 53)

    def dump(title, rows, count_keys):
        L.append(f"[{title}]  ({len(rows)} file(s))")
        if not rows:
            L.append("    none")
            return
        # Sort worst-first by the primary count key.
        pk = count_keys[0]
        rows = sorted(rows, key=lambda r: -r["counts"].get(pk, 0))
        for r in rows[:40]:
            cs = " ".join(f"{k}={r['counts'].get(k,0)}" for k in count_keys)
            L.append(f"    {cs:44s} {r['name']}")
        if len(rows) > 40:
            L.append(f"    ... and {len(rows)-40} more")

    # 1. Fonts skipped -- highest priority (data loss on the operator's files).
    dump("1. UNSUPPORTED FONTS (text SKIPPED)", unsupported_files,
         ["unsupported"] + REASON_KEYS)
    L.append("")
    # 2. Load / render failures -- files pdfce can't open at all.
    L.append(f"[2. LOAD / RENDER FAILURES]  ({len(fail_files)} file(s))")
    if not fail_files:
        L.append("    none")
    else:
        for r in fail_files:
            L.append(f"    [{r['status']}] {r['name']}: {r.get('error','')[:110]}")
    L.append("")
    # 3. notdef glyphs -- font present but a glyph had no mapping.
    dump("3. NOTDEF GLYPHS (unmapped glyphs)", notdef_files, ["notdef"])
    L.append("")
    # 4. Unsupported / unsupported-codec images.
    dump("4. IMAGE CODEC UNSUPPORTED", imgcodec_files, ["images_codec_unsupported"])
    L.append("")
    dump("4b. IMAGES UNSUPPORTED (other)", imgunsup_files, ["images_unsupported"])
    L.append("")
    dump("4c. CODEC FEATURE UNSUPPORTED", codecfeat_files, ["codec_features"])
    L.append("")
    # 5. Tolerated oddities -- rendered, but pdfce flagged something.
    dump("5. TOLERATED ODDITIES (rendered, soft-flagged)", tolerated_files, TOLERATED_KEYS)
    L.append("")
    # 6. extract-text cross-check anomalies.
    L.append(f"[6. TEXT CROSS-CHECK: rendered clean-of-fonts but 0 chars]  ({len(empty_text)} file(s))")
    if not empty_text:
        L.append("    none")
    else:
        for r in empty_text[:40]:
            L.append(f"    {r['name']}")
    L.append("")
    L.append("=" * 78)

    report = "\n".join(L)
    (out_dir / "report.txt").write_text(report, encoding="utf-8")
    (out_dir / "results.json").write_text(
        json.dumps({"root": str(root), "cli": cli, "total_found": total_found,
                    "capped": capped, "cap": args.cap, "elapsed_s": elapsed,
                    "records": records}, indent=1),
        encoding="utf-8")
    print(report)
    print(f"\n[report written to {out_dir/'report.txt'} and results.json]", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
