#!/usr/bin/env python3
"""Judge pdfce's OWN OUTPUT with an independent PDF parser (veraPDF).

WHY THIS EXISTS
---------------
Every test pdfce has reads pdfce's output with **pdfce's own parser**.
``round-trip`` reloads through ``pdfce-core``; the forms tests assert
through ``parse_acroform``; the redaction tests read back with the same
lexer that wrote the bytes. That is a closed loop, and a closed loop
cannot see a defect that both halves share.

This is not hypothetical. **R159** was minted on 2026-08-07 after
exactly that failure: ``flatten`` left ``/AcroForm /Fields`` naming
objects it had deleted, and *every* forms test passed — because
``parse_acroform`` silently drops entries that no longer resolve. The
model looked right while the file was wrong. No amount of in-house
discipline closes that gap, because the discipline and the defect live
in the same codebase.

So this gate hands pdfce's bytes to a **completely independent
implementation** — veraPDF, a Java PDF parser written by people who
have never seen this repository — and asks one question: *can you read
it at all?*

WHAT IT DOES **NOT** DO, AND WHY
--------------------------------
It does **not** check PDF/A conformance, even though PDF/A conformance
is veraPDF's entire reason for existing. pdfce does not write PDF/A yet
(``to-pdfa`` is unimplemented), and running a PDF/A profile against
ordinary PDF output reports a wall of failures that are **not defects**
— no XMP metadata, no ``/OutputIntent``, unembedded fonts. Every one of
those is correct behaviour for a file that never claimed to be PDF/A.

``--off`` turns validation off and runs the parser alone, which is
precisely the question worth asking today. When ``to-pdfa`` ships, the
conformance gate is a *separate* tool, not a flag on this one: the two
answer different questions and a failure in each means something
different.

THE TRAP THIS TOOL EXISTS TO AVOID (read before "simplifying" it)
-----------------------------------------------------------------
The obvious implementation is ``verapdf --off <file>`` plus a check on
the exit code. **That gate passes everything, forever, including a file
with no xref table.** Measured 2026-08-07:

    verapdf --off <valid.pdf>    -> exit 0
    verapdf --off <garbage.pdf>  -> exit 0     <-- both zero

``--off`` suppresses the failure signal along with the validation.
The parse verdict lives **only in the XML report body**, never in the
exit status. This is **R162** ("an assertion that something is ABSENT
proves nothing until the container has been shown capable of holding
it") in the wild, on the day the rule was written.

``self_test()`` below exists so that trap cannot silently reopen: it
feeds the tool a deliberately broken file and **fails if the gate does
not fail**. Run it with ``--self-test``.

WHY ``--mode full`` IS THE DEFAULT (the second trap)
-----------------------------------------------------
``round-trip --mode incremental`` with an empty dirty set promises
**whole-file byte identity** — the output *is* the input, byte for
byte. Validating that output tells you the INPUT parses. It says
nothing whatsoever about pdfce.

``full`` is a complete rewrite: every object definition, the xref
table, the trailer, all emitted by pdfce. That is the only mode where
a veraPDF verdict is a verdict on *pdfce's writer*. ``append-identity``
is also meaningful (it exercises the real append writer), and is
offered. ``incremental`` is offered too, but see ``MODE_NOTES`` — the
tool says out loud when the mode it was given cannot prove anything.

LICENSING — WHY THIS IS A SEPARATE PROCESS AND WHY IT SKIPS
------------------------------------------------------------
veraPDF is dual-licensed **GPLv3+ / MPLv2+** (verified against every
component repo's ``README`` and the presence of ``LICENSE.MPL`` in
``veraPDF-apps``). pdfce **elects MPL-2.0** — see ``docs/LEGAL.md``.
The CLI's own startup banner names only GPL v3 and is misleading; it
states one branch of a dual licence.

pdfce therefore:

* invokes veraPDF as a **separate process** over its documented CLI —
  never links, embeds, or vendors it;
* never redistributes it (hence the out-of-tree install path);
* keeps it **dev-time only** — it appears in no ``Cargo.toml`` and
  correctly never appears in ``THIRD_PARTY_LICENSES.md``, which
  ``cargo-about`` generates from Cargo dependencies alone. Do not
  "fix" that by adding it.

**And this gate SKIPS — never fails — when veraPDF is absent.** That is
a licensing requirement, not a convenience: a gate that *required*
veraPDF would make it a de facto build dependency of pdfce, which
muddies the arms-length position and breaks anyone who clones the repo
without installing a GPL/MPL Java application. A skip is reported
loudly on stderr so it cannot be mistaken for a pass.

USAGE
-----
    python tools/verapdf-parse-gate.py <pdf-or-dir> [...] [options]

    --mode {full,append-identity,incremental}   default: full
    --limit N              stop after N inputs
    --batch N              files per veraPDF invocation (default 32)
    --keep                 keep the produced PDFs for inspection
    --self-test            prove the gate can fail, then exit
    --verapdf PATH         override veraPDF discovery

Discovery order for veraPDF: ``--verapdf``, then ``$PDFCE_VERAPDF``,
then ``D:\\tools\\verapdf\\verapdf.bat``, then ``verapdf`` on ``PATH``.

EXIT CODES
----------
0   every produced file parsed, **or** veraPDF is not installed (skip)
1   at least one file pdfce wrote could not be parsed by veraPDF
2   the harness itself failed (pdfce-cli missing, bad arguments)

A parse failure is printed with the file, the mode that produced it,
and veraPDF's own exception message — counted, never rounded away.
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET
from dataclasses import dataclass
from pathlib import Path

# Where veraPDF is expected when nothing overrides it. Deliberately
# OUT of the repository tree: pdfce must never redistribute it.
DEFAULT_VERAPDF = Path(r"D:\tools\verapdf\verapdf.bat")

# What each round-trip mode proves about pdfce's WRITER, which is the
# only thing this gate is trying to judge.
MODE_NOTES = {
    "full": None,  # the meaningful default; every byte is pdfce's
    "append-identity": None,  # exercises the real append writer
    "incremental": (
        "MODE WARNING: 'incremental' with no edits promises whole-file "
        "byte identity, so the file handed to veraPDF IS the input. A "
        "pass proves the INPUT parses and says nothing about pdfce's "
        "writer. Use --mode full for a verdict on pdfce."
    ),
}


@dataclass
class ParseFailure:
    """One file pdfce wrote that an independent parser could not read."""

    source: Path
    mode: str
    message: str


def find_verapdf(override: str | None) -> Path | None:
    """Locate the veraPDF CLI, or return None so the caller can SKIP.

    Returning None rather than raising is the whole point — see the
    licensing note in the module docstring. Absence is a legitimate,
    non-failing state.
    """
    for candidate in (
        override,
        os.environ.get("PDFCE_VERAPDF"),
        str(DEFAULT_VERAPDF),
    ):
        if candidate and Path(candidate).is_file():
            return Path(candidate)
    found = shutil.which("verapdf") or shutil.which("verapdf.bat")
    return Path(found) if found else None


def collect_inputs(paths: list[str], limit: int | None) -> list[Path]:
    """Expand the given files and directories into a list of PDFs."""
    out: list[Path] = []
    for raw in paths:
        p = Path(raw)
        if p.is_dir():
            out.extend(sorted(q for q in p.rglob("*.pdf") if q.is_file()))
        elif p.is_file():
            out.append(p)
        else:
            print(f"warn  not found, skipped: {p}", file=sys.stderr)
    return out[:limit] if limit else out


def build_cli(workdir: Path) -> Path:
    """Build `pdfce-cli` once and return a PRIVATE COPY of the binary.

    # Why a copy, and why not `cargo run` per file

    The obvious implementation calls ``cargo run -p pdfce-cli`` for each
    input. That is wrong in two compounding ways, both measured on
    2026-08-07 rather than predicted:

    1. **It holds the build artifact hostage.** A sweep of a few hundred
       files keeps ``target/debug/pdfce-cli.exe`` in use for many
       minutes, and any concurrent ``cargo test`` in the same repository
       dies with ``failed to remove file ... Access is denied
       (os error 5)`` on Windows, because it cannot relink a running
       binary. That failure names a *file permission* problem and gives
       no hint that another job is the cause — a genuinely confusing
       error for anyone who did not start the sweep.
    2. **It re-runs cargo's dependency resolution every single time**,
       which dominates the runtime of the actual work.

    Building once and running a copy out of the sweep's own temp
    directory fixes both: ``target/`` is untouched for the whole run, so
    a developer can build and test normally while a long sweep is in
    flight.
    """
    proc = subprocess.run(
        ["cargo", "build", "-q", "-p", "pdfce-cli"],
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0:
        raise RuntimeError(
            f"cargo build -p pdfce-cli failed:\n{proc.stderr or proc.stdout}"
        )
    exe = "pdfce-cli.exe" if os.name == "nt" else "pdfce-cli"
    built = Path("target") / "debug" / exe
    if not built.is_file():
        raise RuntimeError(f"built pdfce-cli not found at {built}")
    private = workdir / exe
    shutil.copy2(built, private)
    return private


def produce(cli: Path, src: Path, mode: str, dest: Path) -> str | None:
    """Have pdfce WRITE `src` to `dest` in `mode`.

    Returns None on success, or a reason string. A pdfce refusal is
    NOT a gate failure: refusing by name is correct behaviour (R27),
    and a sweep that counted refusals as failures would push the
    implementation toward guessing rather than refusing.
    """
    proc = subprocess.run(
        [
            str(cli),
            "round-trip", "--mode", mode, "-o", str(dest), str(src),
        ],
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0 or not dest.is_file():
        detail = (proc.stderr or proc.stdout or "").strip().splitlines()
        return detail[-1] if detail else f"exit {proc.returncode}"
    return None


def verapdf_parse_report(verapdf: Path, files: list[Path]) -> dict[str, str]:
    """Run veraPDF in parse-only mode; return {file: parse-error}.

    Only files that FAILED to parse appear in the returned mapping, so
    an empty result means every file was readable.

    The verdict is read from the XML body and never from the exit
    status — `--off` returns 0 for a file with no xref table. See the
    module docstring; this is the tool's central hazard.
    """
    proc = subprocess.run(
        [str(verapdf), "--off", *[str(f) for f in files]],
        capture_output=True,
        text=True,
    )
    try:
        report = ET.fromstring(proc.stdout)
    except ET.ParseError as exc:
        # veraPDF produced something that is not a report at all. That
        # is a harness problem, not a verdict, and must not be silently
        # read as "nothing failed to parse".
        raise RuntimeError(
            f"veraPDF did not return a parseable report ({exc}). "
            f"stderr: {(proc.stderr or '').strip()[:300]}"
        ) from exc

    failures: dict[str, str] = {}
    for job in report.iter("job"):
        item = job.find("item")
        name_el = item.find("name") if item is not None else None
        name = (name_el.text or "").strip() if name_el is not None else "<unknown>"
        for task in job.findall("taskException"):
            if task.get("type") == "PARSE":
                msg_el = task.find("exceptionMessage")
                msg = (msg_el.text or "").strip() if msg_el is not None else "parse failed"
                failures[name] = msg

    # Cross-check against veraPDF's own count. If the summary says N
    # files failed to parse and we extracted a different number, our
    # extraction is wrong and reporting "clean" would be a lie.
    summary = report.find("batchSummary")
    if summary is not None:
        claimed = int(summary.get("failedToParse", "0"))
        if claimed != len(failures):
            raise RuntimeError(
                f"extraction disagrees with veraPDF: batchSummary says "
                f"failedToParse={claimed}, extracted {len(failures)}. "
                f"The report format changed; fix the parser rather than "
                f"trusting either number."
            )
    return failures


def self_test(verapdf: Path) -> int:
    """Prove the gate can FAIL. Exits non-zero if it cannot.

    R162: an assertion that something is absent proves nothing until
    the container has been shown capable of holding it. This gate's
    whole output is "no parse failures", which is exactly the shape
    that passes vacuously — so the tool ships with the proof attached.
    """
    with tempfile.TemporaryDirectory(prefix="verapdf-selftest-") as tmp:
        broken = Path(tmp) / "deliberately-broken.pdf"
        broken.write_bytes(b"%PDF-1.7\nthis is not a pdf\n")
        failures = verapdf_parse_report(verapdf, [broken])
        if not failures:
            print(
                "SELF-TEST FAILED: veraPDF reported no parse failure for a "
                "file with no xref table. The gate cannot detect anything "
                "and every 'clean' result it has ever printed is vacuous.",
                file=sys.stderr,
            )
            return 1
        (name, msg), = failures.items()
        print(f"self-test ok — gate detects a broken file: {msg}")
        return 0


def main() -> int:
    ap = argparse.ArgumentParser(
        description="Validate pdfce's own output with an independent PDF parser.",
    )
    ap.add_argument("paths", nargs="*", help="PDF files or directories")
    ap.add_argument("--mode", default="full", choices=sorted(MODE_NOTES))
    ap.add_argument("--limit", type=int, default=None)
    ap.add_argument("--batch", type=int, default=32)
    ap.add_argument("--keep", action="store_true")
    ap.add_argument("--self-test", action="store_true")
    ap.add_argument("--verapdf", default=None)
    args = ap.parse_args()

    verapdf = find_verapdf(args.verapdf)
    if verapdf is None:
        # SKIP, never fail. See the licensing note: a required gate
        # would make veraPDF a build dependency of pdfce.
        print(
            "SKIP  veraPDF not found — this gate is optional by design "
            "(dev-time only, never a pdfce dependency). Install it and "
            "set PDFCE_VERAPDF, or pass --verapdf PATH.",
            file=sys.stderr,
        )
        return 0

    if args.self_test:
        return self_test(verapdf)

    if not args.paths:
        ap.error("give at least one PDF or directory (or use --self-test)")

    note = MODE_NOTES[args.mode]
    if note:
        print(note, file=sys.stderr)

    inputs = collect_inputs(args.paths, args.limit)
    if not inputs:
        print("no input PDFs found", file=sys.stderr)
        return 2

    workdir = Path(tempfile.mkdtemp(prefix="verapdf-gate-"))
    produced: dict[Path, Path] = {}
    refused = 0
    try:
        try:
            cli = build_cli(workdir)
        except RuntimeError as exc:
            print(f"harness error: {exc}", file=sys.stderr)
            return 2
        for i, src in enumerate(inputs):
            dest = workdir / f"{i:05d}-{src.name}"
            reason = produce(cli, src, args.mode, dest)
            if reason is not None:
                # A refusal is a correct outcome, not a gate failure.
                refused += 1
                continue
            produced[dest.resolve()] = src

        failures: list[ParseFailure] = []
        batch: list[Path] = list(produced)
        for start in range(0, len(batch), args.batch):
            chunk = batch[start : start + args.batch]
            for name, msg in verapdf_parse_report(verapdf, chunk).items():
                src = produced.get(Path(name).resolve(), Path(name))
                failures.append(ParseFailure(src, args.mode, msg))

        for f in failures:
            print(f"FAIL  {f.source}  [--mode {f.mode}]\n      {f.message}")

        print(
            f"\nverapdf-parse-gate: {len(produced)} file(s) written by pdfce "
            f"and parsed by veraPDF {verapdf.name}, "
            f"{len(failures)} unreadable, {refused} refused by pdfce "
            f"(refusals are not failures)."
        )
        return 1 if failures else 0
    finally:
        if args.keep:
            print(f"produced files kept in {workdir}", file=sys.stderr)
        else:
            shutil.rmtree(workdir, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main())
