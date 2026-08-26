# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. **Overwrite it once acted on.**

Per standing rule `R216` this file carries **no edit-history layer**: no
*"this paragraph read X until…"*. What is true now, plus a pointer.
Corrections and their prior wording live in the **append-only** record —
`ROADMAP.md` and `SESSION_LOG.md` — where a claim is dated and no later edit
can falsify it.

---

## §A — COLD START: everything you need, in one screen

**The previous session was a PLANNING session. No code shipped. No commit was
made.** It answered *"what do we need to make form push buttons work?"*,
produced eight operator rulings, one standing-rule amendment, and an
engineer-owned plan — and then found a **shipped defect** while probing
Acrobat, which is the first thing you should fix.

### ★ Verified from a shell at write time — do not copy these forward without re-running

| fact | value |
|---|---|
| `HEAD` | `9a4fb18` |
| `origin/main` | `9a4fb18` — **level, NOT ahead** |
| tag at `HEAD` | `v0.13.0` |
| working tree | **dirty — see §B, nothing from that session is committed** |
| gates on disk | `ls tools/check-*` → **18** (one is not a bare gate; count, don't quote) |

**The previous handoff said `main` was AHEAD of `origin/main` and that a push
needed a go-ahead. That is no longer true** — `v0.13.0` is tagged and pushed
and `HEAD == origin/main`. Re-check with `git rev-list --count origin/main..main`
rather than trusting this table an hour from now.

---

## §B — ★★★ DO THIS FIRST: an uncommitted working tree from a session that
made no commits

The planning session left everything on disk and committed **nothing**,
deliberately — it was plan-only. You inherit:

| path | what it is | disposition |
|---|---|---|
| `docs/plan-scripting-submit-and-plugins.md` | **NEW.** The whole submit/scripting plan. Engineer-owned. | keep |
| `docs/ROADMAP.md`, `FEATURES.md`, `SESSION_LOG.md`, `ARCHITECTURE.md` | librarian's 268th + 269th filings: `R54` amended (decision `088`), `R56` citation corrected, `Pass 131.0`–`131.4` minted, capability rows added | keep |
| `tools/check-ledger-numbers.py` | **a gate fix** — see §D | keep |
| `.claude/agent-memory/**` | engineer + spec-librarian memory updates | keep |

**Decide the commit shape before you write code**, because `check-commits-filed.py`
and `check-passes-filed.py` both care. These are docs + one tool fix, already
filed by the librarian — so they can go as one commit that the 268th/269th
filings already name. **Do not bundle new code into it** (that mistake has its
own standing entry).

---

## §C — ★★★ THE ONE CODE DEFECT, AND IT IS THE RIGHT FIRST TASK

**pdfce cannot see that a push button submits to a website.**

`scan_javascript` / `FormJavaScript` in `crates/pdfce-core/src/forms.rs` exist
— in their own doc comment's words — *"to disclose what a document would run
in Acrobat/Reader"*, and `network_action_count` is documented as flagging
*"`/AA` actions referencing the network"*.

**It scans `/AA` only. A widget's PRIMARY action lives in `/A`.** A submit, a
URI, a launch on a push button all sit in `/A`; `/AA` carries only the
*additional* triggers (`/E` `/X` `/D` `/U` `/Fo` `/Bl`).

**Measured on a hand-built file whose button Acrobat then actually submitted
to `http://127.0.0.1:8765/declared-http`:**

| surface | output |
|---|---|
| `list-fields` | `aa=0 … js_network_actions=0 js_launch_actions=0` |
| `inspect` | no action / submit / network / URI / launch line at all |
| `list-annotations` | `subtype=Widget … author="Go"` — no action |

**Three surfaces, none of them disclosing it.** Ask pdfce whether a document
phones home and it says no about a file that demonstrably does.

**Fix shape.** Walk widget `/A` alongside `/AA` — and because `/A` also
appears on **link annotations, outline items and `/OpenAction`**, decide
deliberately which carriers are in scope rather than patching the one that was
noticed. This is **recognition only**: it fires no trigger, so it needs
neither the `R54` amendment nor any new rule. Owes fixture tests and a
`FEATURES.md` touch (the disclosure rows currently over-claim).

★ **Why it ranks above everything else in §E:** it is the read half of the
exact disclosure the whole submit plan rests on, it is shipped and wrong
today, and it is the same *"a check that under-reports reads as a clean bill
of health"* shape this project keeps meeting.

---

## §D — A GATE WAS UNDER-REPORTING; IT IS FIXED, AND THE LESSON IS THE POINT

`tools/check-ledger-numbers.py` printed `SESSION_LOG filings : 267 -> next
free is 268` **while filing 268 was already in the document**, and still
summarised `ledger-numbers: clean`.

**Cause.** Its `FILING_HEADING` regex demanded an *alphabetic* ordinal. The
268th filing wrote `(268th filing)` — a numeral. The line therefore never
matched, so it was never a "filing heading", so it never reached
`ordinal_to_int()`, so it never landed in `unparsed`, so the `UNCHECKED`
report that exists **precisely to make such a hole loud** had nothing to say.

★ **A strict RECOGNISER upstream of a reporting PARSER turns every novel
spelling into a silent skip.** The safety net was already correct and simply
unreachable for that input class. **Third instance of this exact shape in that
one file.**

**Fixed as a class, not a spelling**: the recogniser now matches
`(<anything> filing)` and decides nothing; `ordinal_to_int()` decides and
returns `None` if it cannot; every `None` is reported and fails. Verified —
`CCLXVIII` and `sixty-eleventh` are now *reported* rather than skipped, and
the gate reads `268 -> next free is 269`.

**The librarian ruled numerals acceptable and preferred going forward**, and
recorded it under `ROADMAP.md`'s *Update protocol*. The 268th heading was
deliberately **not** renormalised (rewriting a filed heading to match a
convention decided afterwards is what the append-only rule forbids).

---

## §E — THE NON-JAVASCRIPT WORK QUEUE

The operator's instruction closing the planning session: **continue the other,
non-JavaScript work.** The submit/scripting plan is **parked by his own
ruling** — see §F. Do not re-open it.

Ordered by this engineer's judgement, not by Pass number:

1. **§C — the `/A` disclosure defect.** Above. Do it first.
2. **`Pass 130.2` — per-sample image overprint for `Separation`/`DeviceN`
   images.** Re-scoped 2026-08-26 and now much *smaller* than it looks:
   Table 149 excludes a sampled image from row 1 by name, so painting a
   **process** image normally is the conforming behaviour and must NOT be
   "fixed". What is genuinely owed is row 3 — a process component takes the
   backdrop under `OP true`, so an overprinting `DeviceN` image must preserve
   it. Blocker: `Pass 130.1` captures colorants only for a `DeviceCMYK` base.
   Measured candidates: `PCS1_190`/`191`/`192`.
3. **`Pass 122.7` — the undiagnosed blue-channel residual.** Green now matches
   Acrobat to ~1 level (187.5 vs 186.6); blue does not (55.7 vs 2.6, down from
   209.2). A large improvement, not a completed one, and nobody knows why.
4. **`Pass 127.2` — `redact-mark`'s stdout carries no diagnostics field.**
   `find-text` prints unreadable-font counts to stdout; `redact-mark` puts
   them on stderr only, so a batch caller parsing stdout still cannot tell a
   clean run from an unreadable one. Small, and it closes a real asymmetry.
5. **The per-colorant (spot) compositing buffer.** One plate per spot
   colorant. Every remaining print-suite FAIL is now an overprint, spot or ICC
   patch — **not one is a blending-space failure**. ★ The cheap page-sized
   spot-ink multiplier was built, ablated and reverted: it flipped no patch
   and regressed one. **Do not re-attempt it.**
6. **CLI surface for the four ce-dimension group management verbs** — rename,
   delete, delete-with-policy, re-parent. Core ships all four; no subcommand
   reaches any of them, so from a script a group is still create-only (`R151`
   shape: callable-and-uncalled).

**An open call you may have to make before tagging anything:** `Pass 129.1`
changed a shipped default's *value* (`ocr --dpi` 300 → 150), which a scripted
caller can observe, while adding no public core item. The `768e934` precedent
(*"a new callable verb is not a patch"*) settles the additive case and does
**not** settle this one. **Decide before tagging, not after.**

---

## §F — THE SUBMIT / SCRIPTING PLAN IS PARKED. READ THIS BEFORE RE-OPENING IT

Full detail: **`docs/plan-scripting-submit-and-plugins.md`**. Eight operator
rulings are quoted verbatim there; do not paraphrase them from this file.

The four-line version:

- **A push button that does anything is blocked by `R54`** (*"no trigger event
  ever fires"*), **not** by the JavaScript rule. `R54` was written as a
  companion to `R53` and its text outran its motivation — it bites a plain,
  script-free Reset button. **The operator ruled "change the rule"; decision
  `088` amends it to a dispatch allow-list.**
- **The JavaScript half is deferred by the operator** (*"defer for now"*).
  `R53` stands. Phases 1–3 must each work with **no scripting engine present
  at all** — that is a design constraint, not a description.
- **The plugin boundary is a versioned MESSAGE FORMAT, not a binary**, ruled
  for the stated reason that it makes a web version easier. It may name no
  pipe, no path and no host-language type.
- **Submission is permitted, destination open by default, destination always
  disclosed, whitelist mode and payload disclosure available.**

**Still owed to the operator before any submit code:** a decision record for
`R12`'s new destination class (sending the operator's data where a *file's*
author said is a different class from fetching where *we* said), the
`R13` clause 5 ruling (deliberately not forced — Phase 3 is hand-install
only), the transport question (unencrypted destinations), and a
`pdfce-ui-specialist` dispatch before any GUI surface.

**Measured against the local Acrobat, and it corrected the research:** the
Security Warning names **scheme + host only** — not the port, not the path —
while the button's own hover tooltip shows the *full* URL. The remember-this-
site box is **ticked by default** and one Allow grants the host for every PDF
permanently. **Not established:** HTTP-vs-HTTPS, scripted-vs-declared, and the
raw FDF body (so the spec's claim that a baseline FDF submission carries your
file's path and document ID is **spec-sourced, not observed** — do not
upgrade it without the bytes).

★ **If you rebuild that probe harness** (it lived in `%TEMP%\pdfce-submit-probe\`
and is disposable): Acrobat ignores synthetic keystrokes and `BM_CLICK`
entirely; UI Automation can **read** its dialog but not press it. The working
technique is **read the true control rect via UI Automation, click it with the
mouse**. Two modals swallow clicks aimed at the page — sign-in, and crash
recovery — and **a swallowed click is indistinguishable from a refusal**,
which cost several runs and nearly produced a false security finding. And an
`Allow` with the box ticked **writes into the operator's profile**: this
session's entry was removed and `version:2|ikea.com:2` restored and verified.
Any future run owes the same.

---

## §G — PRE-FLIGHT, EVERY SESSION

**1. `ls` BOTH FeatureRequests channels.** They are outside this repository,
so **no gate will ever contradict a stale sentence about them — including
this one.**

```
D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\
D:\Dev\FeatureRequests\iccce_FeatureRequests\open\
```

At write time the pdfce channel's three newest are all **outbound** notes
dated 2026-08-26 (CMYK images / the per-image-overprint correction / the
corpus reply). The iccce channel's newest is
`note_your_name_gate_has_the_two_defects_mine_had.md` — **unanswered, and it
is about a gate this project also owns.** Worth reading early.

**2. Run the gates — `ls tools/check-*`, do not trust any list**, including
the count in §A. `R209`: *"all gates green" names a set, and the set somebody
runs is not the set CI runs.*

**3. ★★ RUN `cargo +nightly fuzz build` ON WINDOWS.** CI cannot do it for you
and `cargo check --bins` is not a substitute. Two traps, each defeating the
obvious escape from the other: CI's fuzz job runs on `ubuntu-latest` and was
green throughout a window in which the harness was completely unbuildable here
(`rten` declares a `cdylib`, and only Windows hands that libFuzzer's
`/include:main`); and the documented local stand-in **passes in both states**,
because `cargo check` never links and the break was a link break. **A cheap
proxy for a gate is a proxy for the part of the gate that is cheap.**

**4. Read `docs/ARCHITECTURE.md` §12** — cross-project boundaries live there
and no gate can catch a violation of them.

**5. Read `docs/compositor-plan.md`** before scoping anything in `97.x`, and
before items 2, 3 or 5 of §E.

---

## §H — ★★ A DISPATCH IS A SET OF CLAIMS, AND YOURS WILL BE WRONG

Carried forward because it keeps earning its place. The 263rd filing's
dispatch carried three factual premises and **all three were false** — a path
the other project had already consumed and renamed, a `FEATURES.md` box
asserted `[ ]` that had been wired six hours earlier, and a feature credited
to the wrong commit. None was careless; each was a reasonable inference from
what the engineer remembered doing. **Memory of one's own session is exactly
the kind of source that feels like a fact.**

It happened again this session, twice, in miniature: a dispatch repeated a
key list from the spec corpus's own *"NOT ingested"* notice, and **two of the
keys did not belong to that table at all** — one was a page-break interleave
from the neighbouring table, one a name collision with a font-descriptor key.
**A gap notice describing what it does not contain is an unverified claim like
any other**, written by someone who had by definition not read the clause. And
a `reg query` that failed on quoting returned nothing, which was read as *"the
key is empty"* and reported to the operator as fact; it was not empty.

**Write dispatches so a premise is checkable, and expect the agent to check.**
A dispatch that says *"X is at path P"* invites verification; *"as we
discussed"* does not. **Finish the code, then dispatch, then commit the filing
last.**

---

## §I — THE FILING COMMIT MUST BE THE LAST COMMIT BEFORE ANY TAG

`check-commits-filed.py` counts commits that no filing names. The tip-deferral
excuses a commit that cannot cite its own hash — **but only while it is the
tip**. The instant a filing lands on top, the excuse evaporates and the gate
flips red without anything about that commit changing.

> **Dispatch the librarian LAST, and commit its filing LAST.** Any code commit
> made after the dispatch has, by construction, no filing that can name it.

Recovery if it happens anyway (precedent: `v0.8.0`, `v0.10.0`, `v0.12.0`):
file the orphan, re-tag at the filing commit, force-push the tag, **rebuild
the package** so `BUILD-INFO.txt` names the tagged commit, replace the asset
with `--clobber`, and **re-run the smoke test on the new artefact** — a re-cut
release is a new artefact and does not inherit the old one's test.
