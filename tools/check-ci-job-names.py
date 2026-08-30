#!/usr/bin/env python3
"""check-ci-job-names.py — a CI job that runs many checks must SAY so, and the
number must be right.

WHY THIS EXISTS
===============

CI reports failures by **job**. The log shows the failing step, the summary
and the run list show the job. So a job's name is the first — and often the
only — thing a person reads when something goes red.

On 2026-08-30 a filing gate failed and CI reported a red job called
**"verify pdfce-gui strings live in ui_text.rs"**. That job ran **nineteen**
`run:` steps; the name described **one** of them, and `check-ui-strings.sh`
itself was perfectly clean. The engineer ran the named check locally, got
`clean`, and briefly believed the local runner and CI disagreed — which is the
expensive part, because it impugns the tool everybody uses before pushing.

★ **THE FINDING WAS ALREADY WRITTEN DOWN AND HAD BEEN FOR EIGHTEEN DAYS.**
`D:/dev/rag/rust/a_ci_job_name_describes_its_first_step_not_the_gate_that_failed.md`
(2026-08-12) names that exact job, quotes its YAML, and ends *"Never leave a
multi-gate job named after one of its gates."* It was correct, complete,
actionable — and **inert, because nothing was scheduled against it**. In the
interval the job grew from 3 steps to 19 and misattributed red runs went from
5 to 7.

⇒ **A lesson nobody scheduled is a lesson that does not act.** This file is the
scheduling.

WHAT IT CHECKS, and the one thing it deliberately does not
=========================================================

Two rules, both decidable from the YAML alone:

1. **A job with more than ``DECLARE_THRESHOLD`` ``run:`` steps must declare a
   count** in its ``name:``, as ``(N checks)``.
2. **Any declared count must equal the actual number of ``run:`` steps.**

Rule 2 is what keeps rule 1 from decaying. A collective name is honest on the
day it is written; the count is the part that goes stale, and a stale count is
a claim this file can falsify.

**What it does NOT check, and cannot: whether a name is a fair description.**
"Is this name honest?" is not decidable from the text — a two-step job named
after one of its steps may be perfectly fair, because the second step is a
detail of the first. That judgement stays with the author. What this gate
removes is the case where nobody made the judgement at all, which is how the
19-step job got there: it was named when it had one step and nobody renamed it
nineteen times.

THE THRESHOLD IS A JUDGEMENT AND IS STATED AS ONE
=================================================

At the time of writing, the workflow's jobs ran 1, 1, 2, 2, 2, 3, 3, 6 and 19
`run:` steps. **The gap between 6 and 19 is where the misdirection actually
bites**, and every job at 6 or below has a name that fairly covers all of its
steps (six `cargo tree` invocations under "verify … zero GUI deps" is one
subject, not six). So the threshold sits in that gap.

It is not a law about CI in general. Raising it silences this gate; lowering it
forces collective names onto jobs that do not need them. Change it
deliberately, and say why here.

USAGE
=====

    python tools/check-ci-job-names.py [--self-test]

Exit 0 clean, 1 on a finding, 2 if the workflow cannot be parsed — reported
rather than skipped, because a gate that silently checks nothing is the failure
mode this whole family exists to prevent.
"""

from __future__ import annotations

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
WORKFLOWS = ROOT / ".github" / "workflows"

# See the module docstring: a judgement, sitting in the measured gap between
# the largest single-subject job (6) and the one that went wrong (19).
DECLARE_THRESHOLD = 8

JOB_RE = re.compile(r"^  ([A-Za-z0-9_-]+):\s*$")
NAME_RE = re.compile(r"^    name:\s*(.+?)\s*$")
RUN_RE = re.compile(r"^\s+-?\s*run:\s")
COUNT_RE = re.compile(r"\((\d+)\s+checks?\)")


class Job:
    """One workflow job: its id, its declared name, and how many commands it
    actually runs."""

    def __init__(self, job_id: str) -> None:
        self.id = job_id
        self.name: str | None = None
        self.runs = 0

    @property
    def label(self) -> str:
        return self.name if self.name else self.id


def parse_jobs(source: str) -> list[Job]:
    """Every job in one workflow file, with its `run:` step count.

    Line-oriented rather than a YAML parse, deliberately and for the same
    reason the sibling gates are: this repository has no YAML dependency, and
    adding one to read nine job headers would be a dependency added for a
    gate. The shapes matched are the ones this workflow uses; a job header is
    two-space-indented and a step's `run:` is deeper, so the two cannot be
    confused.
    """
    jobs: list[Job] = []
    current: Job | None = None
    in_jobs = False
    for line in source.splitlines():
        if line.startswith("jobs:"):
            in_jobs = True
            continue
        if not in_jobs:
            continue
        m = JOB_RE.match(line)
        if m:
            current = Job(m.group(1))
            jobs.append(current)
            continue
        if current is None:
            continue
        m = NAME_RE.match(line)
        if m and current.name is None:
            current.name = m.group(1)
            continue
        if RUN_RE.match(line):
            current.runs += 1
    return jobs


def findings(jobs: list[Job]) -> list[str]:
    """Every rule violation, as an operator-readable sentence."""
    out: list[str] = []
    for job in jobs:
        declared = COUNT_RE.search(job.label)
        if declared:
            n = int(declared.group(1))
            if n != job.runs:
                out.append(
                    f"job {job.id!r} says '({n} checks)' and runs {job.runs}. "
                    f"The count in a job name is a claim; update it or drop it."
                )
        elif job.runs > DECLARE_THRESHOLD:
            out.append(
                f"job {job.id!r} runs {job.runs} commands and its name "
                f"({job.label!r}) declares no count. CI reports failures BY "
                f"JOB, so a name describing one of many steps sends the "
                f"reader to the wrong check. Add '(N checks)'."
            )
    return out


def self_test() -> int:
    """Prove the two rules fire, and that a compliant job passes.

    Written because a gate whose own logic is untested is a gate that can
    silently check nothing — the exact class the module docstring is about.
    """
    sample = """jobs:
  tiny:
    name: cargo fmt --check
    steps:
      - run: cargo fmt --all --check
  big:
    name: verify pdfce-gui strings live in ui_text.rs
    steps:
"""
    sample += "".join(f"      - run: echo {i}\n" for i in range(19))
    sample += """  counted:
    name: repository audits (2 checks)
    steps:
      - run: echo a
      - run: echo b
  miscounted:
    name: repository audits (5 checks)
    steps:
      - run: echo a
"""
    jobs = {j.id: j for j in parse_jobs(sample)}
    assert jobs["tiny"].runs == 1, jobs["tiny"].runs
    assert jobs["big"].runs == 19, jobs["big"].runs
    assert jobs["counted"].runs == 2, jobs["counted"].runs

    found = findings(list(jobs.values()))
    joined = " ".join(found)
    assert "'tiny'" not in joined, "a one-step job needs no declaration"
    assert "'big'" in joined, "an undeclared 19-step job must be reported"
    assert "'counted'" not in joined, "a correct declaration must pass"
    assert "'miscounted'" in joined, "a wrong count must be reported"
    print("check-ci-job-names: self-test PASS (4 cases)")
    return 0


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        return self_test()

    files = sorted(WORKFLOWS.glob("*.yml")) + sorted(WORKFLOWS.glob("*.yaml"))
    if not files:
        print(f"ci-job-names: no workflow files under {WORKFLOWS}", file=sys.stderr)
        return 2

    all_jobs: list[Job] = []
    for path in files:
        all_jobs.extend(parse_jobs(path.read_text(encoding="utf-8")))
    if not all_jobs:
        print("ci-job-names: parsed no jobs at all — the workflow shape changed", file=sys.stderr)
        return 2

    problems = findings(all_jobs)
    if problems:
        print("ci-job-names: FAIL")
        for p in problems:
            print(f"  {p}")
        print()
        print("CI reports a failure by JOB name. A job named after one of its")
        print("steps sends whoever reads the red run to a check that is clean,")
        print("which costs a diagnostic cycle and discredits the local runner.")
        return 1

    print(
        f"ci-job-names: clean — {len(all_jobs)} job(s); "
        f"every job over {DECLARE_THRESHOLD} command(s) declares a count, and every "
        f"declared count is right."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
