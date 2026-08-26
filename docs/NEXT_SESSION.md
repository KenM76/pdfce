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

**The previous session shipped `Pass 132.0`** — the CMYK compositing ceiling
became readable (four `pub` items in `pdfce-render`) and settable
(`Settings::max_cmyk_buffer_bytes`, uncapped), because a sibling project could
neither size a raster to stay inside it nor let the operator move it. It also
committed the *preceding* session's plan-only tree, unchanged, as its own
commit.

### ★ Verified from a shell at write time — do not copy forward without re-running

| fact | value | command |
|---|---|---|
| `HEAD` | `76eb04c` | `git rev-parse HEAD` |
| `git describe` | `v0.13.0-2-g76eb04c` | `git describe` |
| `origin/main` | `9a4fb18` — **`main` is 2 AHEAD** | `git rev-list --count origin/main..main` |
| tag at `HEAD` | **none**; highest tag on disk is `v0.13.0` | `git tag --points-at HEAD` |
| working tree | clean at read time, **dirty with the 270th filing when you get it** | `git status --porcelain` |
| newest backup bundle | `pdfce-20260826-0958-9a4fb18-full.bundle`, **2 commits behind `HEAD`** | `ls -lt D:\Dev\pdfce-backups\` |
| gates on disk | **18**; **17 run with no arguments and all 17 were green after the 270th filing**, the 18th (`check-image-colorspace-truth.py`) needs a fixture dir | `ls tools/check-*`, then run them |

---

## §B — ★★★ DO THIS FIRST: `v0.14.0` WAS INSTRUCTED AND HAS NOT BEEN CUT, AND A FILE ALREADY SAYS IT WAS

The operator explicitly instructed a release for `Pass 132.0`. **No `v0.14.0`
tag exists** and `main` is 2 commits ahead of `origin/main`. ★ **The version
bump itself was in the working tree, uncommitted, when the 270th filing was
written** (`Cargo.toml` `0.13.0` → `0.14.0`, both lock files) — so the bump may
already be committed by the time you read this and the **tag** still may not
exist. **Check both separately.**

★★ **The outbound reply at
`D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\reply_cmyk_buffer_ceiling.md`
already states *"released as `v0.14.0`"*.** That sentence is **ahead of the
repository**, in a file another project reads. Either cut the release and make
it true, or correct the file — **do not leave it standing while the tag does
not exist.**

**Order matters and `§H` is why.** `check-commits-filed.py` counts commits no
filing names, and the tip-deferral only excuses the tip:

> **Commit the 270th filing FIRST. Then bump, tag, push, publish, and run
> `verify-release.py`.**

**The version call.** `Pass 132.0` adds **new public items in `pdfce-render`**
(`CMYK_BYTES_PER_PIXEL`, `DEFAULT_MAX_CMYK_BUFFER_BYTES`,
`max_cmyk_composite_pixels`, `will_composite_in_cmyk`) and **three in
`pdfce-core::settings`** (`parse_byte_size`, `format_byte_size`,
`ByteSizeError`) plus a **new `Settings` field**. All additive; no behaviour
changes at the default ceiling. **`docs/core-api/`'s verb count is unchanged**
— the `768e934` precedent's minor case rested on that file, and the `1f66eae`
entry corrected an identical "new public core items" misclassification a day
earlier, so **check which crate before writing the bump message**.

★ **Also still unanswered from the last handoff, and it bites at tag time:**
`Pass 129.1` changed a shipped default's *value* (`ocr --dpi` 300 → 150), which
a scripted caller can observe while adding no public core item. `768e934`
settles the additive case and does **not** settle this one. **Decide before
tagging, not after.**

---

## §C — ★★★ THE ONE CODE DEFECT, STILL UNFIXED, STILL THE RIGHT FIRST TASK

**pdfce cannot see that a push button submits to a website.** Unchanged from
the previous handoff — nothing in `Pass 132.0` touched it.

`scan_javascript` / `FormJavaScript` in `crates/pdfce-core/src/forms.rs` exist
— in their own doc comment's words — *"to disclose what a document would run in
Acrobat/Reader"*, and `network_action_count` is documented as flagging *"`/AA`
actions referencing the network"*.

**It scans `/AA` only. A widget's PRIMARY action lives in `/A`.** A submit, a
URI, a launch on a push button all sit in `/A`; `/AA` carries only the
*additional* triggers (`/E` `/X` `/D` `/U` `/Fo` `/Bl`).

**Measured on a hand-built file whose button Acrobat then actually submitted to
`http://127.0.0.1:8765/declared-http`:**

| surface | output |
|---|---|
| `list-fields` | `aa=0 … js_network_actions=0 js_launch_actions=0` |
| `inspect` | no action / submit / network / URI / launch line at all |
| `list-annotations` | `subtype=Widget … author="Go"` — no action |

**Three surfaces, none of them disclosing it.** Ask pdfce whether a document
phones home and it says no about a file that demonstrably does.

**Fix shape.** Walk widget `/A` alongside `/AA` — and because `/A` also appears
on **link annotations, outline items and `/OpenAction`**, decide deliberately
which carriers are in scope rather than patching the one that was noticed. This
is **recognition only**: it fires no trigger, so it needs neither the `R54`
amendment (decision `088`) nor any new rule. Owes fixture tests and a
`FEATURES.md` touch (the disclosure rows currently over-claim).

★ **Why it ranks above everything in §E:** it is the read half of the exact
disclosure the whole submit plan rests on, it is shipped and wrong today, and it
is the same *"a check that under-reports reads as a clean bill of health"* shape
this project keeps meeting.

---

## §D — ★★ THE "A4" NUMBERS WERE ARITHMETIC ON A DIFFERENT SHEET. REPAIR WAS IN FLIGHT WHEN THIS WAS WRITTEN — VERIFY, DO NOT TRUST THIS SECTION

**Every "A4" percentage shipped with `Pass 132.0` was computed on a 596 × 791 pt
page** — the print-conformance file named in the incoming request — **and
labelled A4.** A4 is 595 × 842 pt.

| page | ceiling reached at | `MAX_PIXMAP_EDGE` tier ends at |
|---|---:|---:|
| **A4, 595 × 842 pt** | **517.6 %** | **1946 %** |
| **596 × 791 pt** (what the prose computed) | **533.6 %** | **2071.3 %** |
| US-Letter, 612 × 792 pt | 526.2 % (≈ 379 DPI) | 2069 % |

**The figures were right and the sheet was wrong** — every number survived an
internal consistency check and only the label failed against the world. The
270th filing found **nine sites**; full list with line numbers is in the
`Pass 132.0` entry's hard-rule-11 sweep in `ROADMAP.md` (**append-only — that
list is a record of `76eb04c`, not of your tree**).

★★ **The engineer began repairing them in the working tree while the filing was
being written, taking the *recompute-for-A4* branch** (`534 %` → `518 %`,
`2071 %` → `1946 %`, `375 DPI` → `379 DPI`). **So this section is the section
most likely to be stale.** Re-derive rather than re-read:

```
grep -rn "534\|530%\|2071\|5\.33x\|5\.34x" crates/ docs/core-api/
```

**Known still-open at write time** (both in `crates/pdfce-render/`):

1. **The doctest comments** at `src/lib.rs:257`/`:259` — *"A4 at 5.33x"* over
   `will_composite_in_cmyk(3177, 4216, None)`. ★★ **These are the proof, not
   two more instances:** `3177 × 4216` px **is** 596 × 791 pt at 5.33×; true A4
   at 5.33× is 14.23 M px, **past** the ceiling — so the assertion would be
   **false** if its own comment were true. **The compiler runs the literal;
   nothing runs the label.** Either relabel the comment or move the literals.
2. **`tests/ambiguity_settings_reach_the_pixels.rs:459`/`:463`** still assert
   `"about 530% zoom"`, which **the settings-file prose no longer says** — so
   `cargo test` should be red on that until it moves.

★★★ **And fix that test properly rather than swapping its literal, because it
is the interesting half.**
`the_settings_file_describes_the_ceiling_the_renderer_actually_enforces` was
written *in the same commit* to prevent exactly this class, and it passed
anyway. Its two halves check different things and never meet: one is
`text.contains("about 530% zoom")` — a **string** check that compares the prose
to nothing — and the other recomputes **517.6 %** on true A4 and asserts
`(500.0..560.0)`. **A ±30-point band holds 517.6, 530 and 534 at once.**
Replace the band with an equality between the recomputed figure and the number
**parsed out of** the prose. ⇢ *A tolerance band chosen to survive rounding will
also survive being wrong.* Recorded as a named candidate under `R212`; a
**second** instance found by a different filing should mint it as an `R212`
clause.

★ **One consequence of the repair worth carrying:** the ceiling and
`MAX_PIXMAP_EDGE` are **very nearly** a factor of four apart on A4 (3.76×), not
exactly — the *"factor of four"* phrasing survives, but do not restate it as
exact.

---

## §E — THE NON-JAVASCRIPT WORK QUEUE

The operator's standing instruction: **continue the other, non-JavaScript
work.** The submit/scripting plan is **parked by his own ruling** — see §G. Do
not re-open it.

Ordered by engineering judgement, not by Pass number:

1. **§C — the `/A` disclosure defect.** Do it first.
2. **§D — finish the A4 repair and fix the documentation test properly.**
   Mostly done in the working tree at write time; two doctest comments and the
   test's own literal were still open, and the test's ±30-point band is the
   durable item. Cheap.
3. **`Pass 130.2` — per-sample image overprint for `Separation`/`DeviceN`
   images.** Re-scoped 2026-08-26 and **smaller than it looks**: Table 149
   excludes a sampled image from row 1 by name, so painting a **process** image
   normally is conforming and must NOT be "fixed". What is owed is row 3 — a
   process component takes the backdrop under `OP true`, so an overprinting
   `DeviceN` image must preserve it. Blocker: `Pass 130.1` captures colorants
   only for a `DeviceCMYK` base. Candidates: `PCS1_190`/`191`/`192`.
4. **`Pass 122.7` — the undiagnosed blue-channel residual.** Green matches
   Acrobat to ~1 level (187.5 vs 186.6); blue does not (55.7 vs 2.6, down from
   209.2). A large improvement, not a completed one, and nobody knows why.
5. **`Pass 127.2` — `redact-mark`'s stdout carries no diagnostics field.**
   `find-text` prints unreadable-font counts to stdout; `redact-mark` puts them
   on stderr only, so a batch caller parsing stdout still cannot tell a clean
   run from an unreadable one. Small, closes a real asymmetry.
6. **The per-colorant (spot) compositing buffer.** One plate per spot colorant.
   Every remaining print-suite FAIL is now an overprint, spot or ICC patch —
   **not one is a blending-space failure**. ★ The cheap page-sized spot-ink
   multiplier was built, ablated and reverted: it flipped no patch and regressed
   one. **Do not re-attempt it.**
7. **`Pass 122.3` — band a large page render.** Two of its three acceptance
   clauses were discharged by `Pass 132.0` (the refusal note now names the way
   out with numbers; the ceiling's doc comment now carries the arithmetic
   argument it demanded). **Banding is the third and it is untouched.**
   ⇢ *An operator-set ceiling trades memory for correctness; banding removes the
   trade* — peak usage is a multiple of nominal (parent + child + spare), so a
   ceiling admitting one large page admits three.
8. **CLI surface for the four ce-dimension group management verbs** — rename,
   delete, delete-with-policy, re-parent. Core ships all four; no subcommand
   reaches any of them, so from a script a group is still create-only (`R151`
   shape: callable-and-uncalled).

**A `D:/dev/rag/rust/` escalation is OWED and was deliberately not written.**
The general form — *an infallible allocation is a bound's silent partner; a
ceiling removed without `try_reserve` converts a typo into an abort* — is a real
ecosystem finding, but **no OOM was actually induced**, so it would be filed on
reasoning rather than measurement. Induce one (a 64 GiB ceiling on a machine
that cannot honour it, and check the process survives and discloses) and the
finding is worth writing. The pdfce-visible half is already in
`ARCHITECTURE.md` §10.1a and decision `089`.

---

## §F — WHAT `Pass 132.0` DECIDED, IN CASE YOU TOUCH THE SAME GROUND

**Decision `089` has two halves and they are one decision.**

1. **`ARCHITECTURE.md` §10's allocation-ceiling rule is about UNTRUSTED
   INPUT.** Every guard in §10.1 bounds a file-supplied quantity. A number the
   operator typed is not that, so an operator-set ceiling is **uncapped** — no
   guard, no warning, no preflight — on his own `max_zoom_percent` ruling:
   *"it is up to the user to determine how much of a performance hit they want
   to take."*
2. **That is only safe because the allocation became fallible in the same
   commit.** `vec![0.0; n]` aborts the process on allocation failure — no
   unwind, no `Err`, no page, **no disclosure**. `CmykBuffer::try_planes` uses
   `try_reserve_exact`.

⇢ **Binding: a session that raises or removes an operator-settable allocation
bound must check that the allocation behind it is fallible, in the same
change.** Full reasoning, alternatives weighed and the scope limits are in
`ARCHITECTURE.md` §12 decision `089` and the new §10.1a.

---

## §G — THE SUBMIT / SCRIPTING PLAN IS PARKED. READ THIS BEFORE RE-OPENING IT

Full detail: **`docs/plan-scripting-submit-and-plugins.md`**. Eight operator
rulings are quoted verbatim there; do not paraphrase them from this file.

- **A push button that does anything was blocked by `R54`** (*"no trigger event
  ever fires"*), **not** by the JavaScript rule. `R54`'s text outran its
  motivation and bit a plain, script-free Reset button. **The operator ruled
  "change the rule"; decision `088` amends it to a dispatch allow-list.**
- **The JavaScript half is deferred by the operator** (*"defer for now"*).
  `R53` stands. Phases 1–3 must each work with **no scripting engine present at
  all** — a design constraint, not a description.
- **The plugin boundary is a versioned MESSAGE FORMAT, not a binary**, ruled for
  the stated reason that it makes a web version easier. It may name no pipe, no
  path and no host-language type.
- **Submission is permitted, destination open by default, destination always
  disclosed, whitelist mode and payload disclosure available.**

**Still owed before any submit code:** a decision record for `R12`'s new
destination class (sending the operator's data where a *file's* author said is
a different class from fetching where *we* said), the `R13` clause 5 ruling
(deliberately not forced — Phase 3 is hand-install only), the transport question
(unencrypted destinations), and a `pdfce-ui-specialist` dispatch before any GUI
surface.

**Measured against the local Acrobat, and it corrected the research:** the
Security Warning names **scheme + host only** — not the port, not the path —
while the button's own hover tooltip shows the *full* URL. The
remember-this-site box is **ticked by default** and one Allow grants the host
for every PDF permanently. **Not established:** HTTP-vs-HTTPS,
scripted-vs-declared, and the raw FDF body (so the spec's claim that a baseline
FDF submission carries your file's path and document ID is **spec-sourced, not
observed** — do not upgrade it without the bytes).

★ **If you rebuild that probe harness** (it lived in `%TEMP%\pdfce-submit-probe\`
and is disposable): Acrobat ignores synthetic keystrokes and `BM_CLICK`
entirely; UI Automation can **read** its dialog but not press it. The working
technique is **read the true control rect via UI Automation, click it with the
mouse**. Two modals swallow clicks aimed at the page — sign-in, and crash
recovery — and **a swallowed click is indistinguishable from a refusal**, which
cost several runs and nearly produced a false security finding. And an `Allow`
with the box ticked **writes into the operator's profile**: a prior session's
entry was removed and `version:2|ikea.com:2` restored and verified. Any future
run owes the same.

---

## §H — PRE-FLIGHT, EVERY SESSION

**1. `ls` BOTH FeatureRequests channels.** They are outside this repository, so
**no gate will ever contradict a stale sentence about them — including this
one.**

```
D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\
D:\Dev\FeatureRequests\iccce_FeatureRequests\open\
```

At write time the pdfce channel's two newest files are
`reply_cmyk_buffer_ceiling.md` (**the one claiming `v0.14.0` is released — see
§B**) and the `request_cmyk_buffer_ceiling_is_invisible_to_the_gui.md` it
answers. **Four `request_*` notes remain unanswered:** `adopt_widget`
pre-flight, markup-opacity-in-two-verbs, `insert_pages` orphaned widgets, and
restyle-an-existing-text-run. The iccce channel's newest is
`note_your_name_gate_has_the_two_defects_mine_had.md` — **unanswered, and it is
about a gate this project also owns.**

**2. Run the gates — `ls tools/check-*`, do not trust any list**, including §A's
count. `R209`: *"all gates green" names a set, and the set somebody runs is not
the set CI runs.*

**3. ★★ RUN `cargo +nightly fuzz build` ON WINDOWS.** CI cannot do it for you
and `cargo check --bins` is not a substitute. Two traps, each defeating the
obvious escape from the other: CI's fuzz job runs on `ubuntu-latest` and was
green throughout a window in which the harness was completely unbuildable here
(`rten` declares a `cdylib`, and only Windows hands that libFuzzer's
`/include:main`); and the documented local stand-in **passes in both states**,
because `cargo check` never links and the break was a link break. **A cheap
proxy for a gate is a proxy for the part of the gate that is cheap.**

**4. Read `docs/ARCHITECTURE.md` §12** — cross-project boundaries live there and
no gate can catch a violation of them. **New since the last handoff: `089`, plus
§10.1a.**

**5. Read `docs/compositor-plan.md`** before scoping anything in `97.x`, and
before items 3, 4, 6 or 7 of §E.

---

## §I — ★★ A DISPATCH IS A SET OF CLAIMS, AND YOURS WILL BE WRONG

Carried forward because it keeps earning its place. The 263rd filing's dispatch
carried three factual premises and **all three were false** — a path the other
project had already consumed and renamed, a `FEATURES.md` box asserted `[ ]`
that had been wired six hours earlier, and a feature credited to the wrong
commit. None was careless; each was a reasonable inference from what the
engineer remembered doing. **Memory of one's own session is exactly the kind of
source that feels like a fact.**

★ **Fresh instance, 2026-08-26: the `Pass 132.0` dispatch carried "about 534 %
zoom on A4" as an established fact.** It was arithmetic on a different sheet,
repeated from the crate's own doc comments into the dispatch and then into a
decision. **It was caught by division**, not by review — which is hard rule
10's whole point. Nine sites survive; see §D.

**Write dispatches so a premise is checkable, and expect the agent to check.** A
dispatch that says *"X is at path P"* invites verification; *"as we discussed"*
does not. **Finish the code, then dispatch, then commit the filing last.**

---

## §J — THE FILING COMMIT MUST BE THE LAST COMMIT BEFORE ANY TAG

`check-commits-filed.py` counts commits that no filing names. The tip-deferral
excuses a commit that cannot cite its own hash — **but only while it is the
tip**. The instant a filing lands on top, the excuse evaporates and the gate
flips red without anything about that commit changing.

> **Dispatch the librarian LAST, and commit its filing LAST.** Any code commit
> made after the dispatch has, by construction, no filing that can name it.

★ **Both `c29f5bd` and `76eb04c` are named by filings on disk** — the
268th/269th name the first, the 270th names the second — so the gate has
nothing outstanding *provided the 270th's commit is the tip when you tag*.

Recovery if it goes wrong anyway (precedent: `v0.8.0`, `v0.10.0`, `v0.12.0`):
file the orphan, re-tag at the filing commit, force-push the tag, **rebuild the
package** so `BUILD-INFO.txt` names the tagged commit, replace the asset with
`--clobber`, and **re-run the smoke test on the new artefact** — a re-cut
release is a new artefact and does not inherit the old one's test.
