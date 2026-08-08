#!/usr/bin/env python3
"""Fail if a persisted operator setting is stored but never read.

WHY THIS EXISTS
===============

Standing rule **R83 — no affordance without capability**. A setting that
round-trips through ``userdata/settings.txt``, is documented in that
file's own comments, and is then read by nothing is worse than no setting
at all: the operator changes it, sees no effect, and reasonably concludes
pdfce is broken rather than that the knob is decorative.

This is not hypothetical. The Pass that introduced the settings store
shipped **two** such fields in its very first commit — ``separations`` and
``word_gap_ratio``. Both parsed, both written back out, both described in
the generated settings file, both consumed by zero call sites. The third
field, ``cmyk_intent``, was threaded all the way to the pixels and had a
whole integration-test file defending it, which is precisely what makes
the other two instructive: the discipline was applied deliberately where
it was being thought about and forgotten where it was not.

Vigilance already failed once here. So it is a gate.

WHAT IT CHECKS
==============

For every ``pub`` field of ``Settings`` in
``crates/pdfce-core/src/settings/mod.rs``:

1. The field must be **parsed** — its key must appear in an ``apply`` arm,
   or it can never be set from the file.
2. The field must be **written** — it must appear in
   ``write_to_string``, or a saved file silently loses it.
3. The field must be **consumed** — read at least once from a
   ``settings.<field>`` / ``.settings.<field>`` expression somewhere
   OUTSIDE the settings module itself. A read inside ``settings/`` does
   not count: round-tripping a value through its own tests proves the
   parser works, not that the program does anything with it.

WHAT IT DELIBERATELY DOES NOT CHECK
===================================

That the consumer is *correct*, or that it reaches the pixels/bytes. That
is a test's job — see ``crates/pdfce-render/tests/cmyk_intent.rs``, which
proves the CMYK intent survives the whole distance from ``RenderOptions``
to a rendered pixel. This gate only catches the cheaper, dumber failure:
nobody wired it at all. A grep-based gate that tried to judge semantics
would produce false confidence, which is the one outcome worse than no
gate.

EXIT CODES
==========

``0`` clean, ``1`` at least one setting is unreachable, ``2`` the gate
could not run (missing files, unparseable struct) — never confused with
"clean", because a check that cannot run must not look like one that
passed.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SETTINGS = ROOT / "crates" / "pdfce-core" / "src" / "settings" / "mod.rs"

# Where a setting may legitimately be consumed. The settings module itself
# is excluded on purpose (see the module docstring).
CONSUMER_ROOTS = [
    ROOT / "crates" / "pdfce-gui" / "src",
    ROOT / "crates" / "pdfce-cli" / "src",
    ROOT / "crates" / "pdfce-render" / "src",
    ROOT / "crates" / "pdfce-core" / "src",
]


def fail(message: str) -> None:
    print(f"settings-consumed: {message}", file=sys.stderr)


def struct_fields(source: str) -> list[str]:
    """Every `pub` field name of the `Settings` struct, in order."""
    match = re.search(
        r"pub struct Settings \{(.*?)\n\}", source, re.DOTALL
    )
    if not match:
        return []
    body = match.group(1)
    # `pub name: Type,` — doc comments and attributes are skipped by the
    # anchor on `pub `.
    return re.findall(r"^\s*pub ([a-z_][a-z0-9_]*):", body, re.MULTILINE)


def section(source: str, signature: str) -> str:
    """The body of one function, by its signature line."""
    at = source.find(signature)
    if at < 0:
        return ""
    # Functions in this module are separated by a de-indented `}`; taking
    # everything up to the next `\n    }\n` is exact enough for a gate and
    # does not need a Rust parser.
    end = source.find("\n    }\n", at)
    return source[at : end if end > 0 else len(source)]


def main() -> int:
    if not SETTINGS.is_file():
        fail(f"cannot read {SETTINGS}")
        return 2

    source = SETTINGS.read_text(encoding="utf-8")
    fields = struct_fields(source)
    if not fields:
        fail(
            "no `pub` fields found on `Settings` — either the struct moved "
            "or this gate's parser is stale. Refusing to report clean."
        )
        return 2

    apply_body = section(source, "fn apply(")
    write_body = section(source, "pub fn write_to_string(")
    if not apply_body or not write_body:
        fail(
            "could not locate `apply` and/or `write_to_string` in the "
            "settings module. Refusing to report clean."
        )
        return 2

    # Gather every consumer file's text once.
    consumers: dict[Path, str] = {}
    for root in CONSUMER_ROOTS:
        if not root.is_dir():
            continue
        for path in root.rglob("*.rs"):
            if SETTINGS.parent in path.parents or path == SETTINGS:
                continue
            try:
                consumers[path] = path.read_text(encoding="utf-8")
            except OSError:
                continue

    problems: list[str] = []
    for field in fields:
        if f'"{field}"' not in apply_body:
            problems.append(
                f"`{field}` has no arm in `apply`, so it can never be set "
                f"from the settings file"
            )
        if field not in write_body:
            problems.append(
                f"`{field}` is not written by `write_to_string`, so saving "
                f"settings would silently drop it"
            )

        pattern = re.compile(rf"settings\s*\.\s*{re.escape(field)}\b")
        readers = sorted(
            str(path.relative_to(ROOT)).replace("\\", "/")
            for path, text in consumers.items()
            if pattern.search(text)
        )
        if not readers:
            problems.append(
                f"`{field}` is parsed and written but READ BY NOTHING. "
                f"R83: an operator who changes it will see no effect. "
                f"Either wire it to the behaviour it names, or take it out "
                f"of `Settings` and out of the generated file"
            )

    if problems:
        fail(f"{len(problems)} problem(s):")
        for problem in problems:
            print(f"  - {problem}", file=sys.stderr)
        print(
            "\n  A setting is a promise. Storing one that does nothing "
            "breaks it silently.",
            file=sys.stderr,
        )
        return 1

    print(
        f"settings-consumed: clean - all {len(fields)} setting(s) are "
        f"parsed, written, and read by at least one caller."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
