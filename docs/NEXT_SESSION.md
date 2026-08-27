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

**The previous session shipped TWO Passes**, both minted straight into
*Shipped*, both requested and delivered the same session:

- **`Pass 133.0`** (`afd8da8`) — pdfce can now say **what a document would run
  in Acrobat/Reader, and whether it reaches outside itself**. It previously
  said `0` about a file Acrobat submits to a live endpoint, because the scan
  walked `/AA` and a widget's primary action lives in `/A`. Fixed from the
  **carrier set** — 17 sites, 10 container types — with `/Next` chains
  followed.
- **`Pass 134.0`** (`fd71e4f`) — **a field's properties are editable after it
  is placed, including its size.** Two verbs, `edit_field` (field scope) and
  `edit_widget` (widget scope), plus `edit-field` / `edit-widget` on the CLI.

**Everything about both Passes is finished** — tests green, gates green,
invariants verified, CLI verified live on real files — **and both are committed
and pushed**, along with their filing (`e0a432e`). ★ **Two things remain
before the tree is tidy:** the filing's own late corrections are still
uncommitted in three `docs/` files, and **no backup bundle contains any of it**.
See `§B`, then start the work at `§C`.

### ★ Verified from a shell at write time — do not copy forward without re-running

| fact | value | command |
|---|---|---|
| `HEAD` | `e0a432e` — the 272nd filing's commit | `git rev-parse HEAD` |
| `git describe --tags` | `v0.14.0-4-ge0a432e` — **4 commits past the tag** | `git describe --tags` |
| `origin/main` | **level, 0 ahead** — `afd8da8`, `fd71e4f` and `e0a432e` are all **PUSHED** | `git rev-list --count origin/main..main` |
| tag at `HEAD` | **none.** Highest tag `v0.14.0`, at `4bea7fe` — **no release covers either new Pass** | `git tag --points-at HEAD`, `git describe --tags` |
| working tree | **NOT clean** — three `docs/` files carry the filing's late corrections, uncommitted | `git status --porcelain` |
| newest backup bundle | `pdfce-20260826-1740-4bea7fe-full.bundle`, 44,030,408 B — at `4bea7fe`, **4 commits behind `HEAD`**; ★★ **no bundle on disk contains either new Pass — the one release-plumbing item still genuinely owed** | `ls -lt D:\Dev\pdfce-backups\`, then `git rev-list --count 4bea7fe..HEAD` |
| gates on disk | **18**; **17 run with no arguments**, the 18th (`check-image-colorspace-truth.py`) needs a fixture dir | `ls tools/check-*`, then run them |
| CI at `HEAD` | **PUSHED, COLOUR UNKNOWN — not queried here.** The last colour anyone recorded is green at `4bea7fe`/`v0.14.0`. ★ **`e0a432e` is a different tree; ask GitHub, do not infer** | `gh run list --branch main --limit 3` |
| `docs/core-api` verbs | **144** (was 142) | `grep -n 144 docs/core-api/index.md` |
| `crates/pdfce-core/src/edit.rs` | **31,655** lines | `grep -c "" crates/pdfce-core/src/edit.rs` |

★ **The CI row is deliberately not a colour, and that is the point.** Four
releases have gone out on a tag whose CI run was red (`§J`'s precedent list),
and the wrong belief came each time from a **carried-forward sentence** rather
than from a query. `HEAD` has been pushed, so a run exists — **run the command
and read it.** Copying the `4bea7fe` green forward is exactly the move that has
cost four re-cuts.

---

## §B — ★★ THE RELEASE PLUMBING IS THE FIRST THING

★★ **Most of this is already DONE — the engineer committed the 272nd filing as
`e0a432e` and pushed it while the filing was still being corrected.** `afd8da8`,
`fd71e4f` and `e0a432e` are all on `origin`, and `origin/main` is level. **What
is left is small and is listed below.**

★ **`check-commits-filed.py` is GREEN — measured, not inferred:**

```
$ python tools/check-commits-filed.py
commits-filed: clean — 597 code commit(s) checked (whole history);
               5 known-unfiled carried in the baseline
```

**It reads the WORKING TREE, not `HEAD`**, so it went green the moment the
272nd filing's text landed on disk, before any commit. ⇢ *This role's own
first draft asserted the gate was "already red" from `§J`'s reasoning and was
wrong — the reasoning was sound and the gate reads a different tree than the
reasoning assumed.* **`§J`'s rule still governs what happens next:** the
tip-deferral excuses **exactly one** trailing unfiled code commit and there are
**two**, so the green survives **only if the filing is committed and nothing is
committed after it.**

**What is actually left:**

1. **Commit the three modified `docs/` files** — the filing's late corrections
   (the `R218` ceiling, the `open/` path, this section). **A docs commit needs
   no filing of its own**, so it is safe as the new tip; a *code* commit there
   would not be.
2. **Re-run the 17 argument-free gates on THAT tree** — ★ not on the one
   already swept. *A gate sweep certifies the tree it ran on*, and this tree has
   changed twice since the engineer's sweep.
3. **★★ TAKE A BACKUP BUNDLE. This is the one release-plumbing item genuinely
   outstanding.** The newest bundle is at `4bea7fe`, **four commits back**, and
   **nothing on disk contains either new Pass.**
4. **Read CI's colour at `main`** rather than assuming it (see `§A`).
5. Then decide about a version bump **separately** — below.

**On a version bump: not obviously owed, no tag has been cut, and the call is
the operator's.**
`Pass 134.0` adds two public `pdfce-core` verbs (**142 → 144**) and two
`EditError` variants; `Pass 133.0` adds nine public `FormJavaScript` fields and
makes that struct `#[non_exhaustive]`. That is the **additive-public-items**
case `768e934` settles as **minor** — but `v0.14.0` shipped hours ago, and
whether two Passes justify `v0.15.0` on the same day is a product judgement, not
a mechanical one. **Do not cut a tag without asking** (`CLAUDE.md` rule 8).

---

## §C — ★★ THREE DOC SURVIVORS, OWED, ALL ONE-SENTENCE REPAIRS

The 272nd filing's hard-rule-11 sweep found four; it fixed one and left three
that are not a librarian's to make. **None blocks anything. All three are
places where a stale sentence will be believed.**

**1–2. `docs/plan-scripting-submit-and-plugins.md`, two survivors.** This is the
document a session re-opening the parked submit plan (`§G`) reads first, and
**its description of pdfce's own read side is now wrong in two places**:

- **`:196`** — the three-row *"no action"* table, copied from the old `§C`.
  `list-annotations` now prints `action=<Type|Type+next|none>`. The table is
  **false**.
- **`:119`** — *"`forms.rs` classifies **`/AA`** action subtypes and counts
  hazards"*, sitting inside a section headed *"Action **reading**: partially
  exists, **and is good**"*. It now classifies **17 carrier sites**. ⇢ *A
  section that **grades** the read side is the worst place for a stale
  **description** of it — the grade travels, the description is not
  re-checked.*

★ **Why the librarian did not edit them:** that document quotes **eight
operator rulings verbatim** and its value is its verbatimness. Re-wording
tables inside it is the engineer's hand, not a filing agent's.

**3. `docs/ARCHITECTURE.md:8798`** — decision 009's Pass record: *"`scan_javascript`
+ the `FormJavaScript` histogram COUNT all **field-level** JS actions … with a
loud stderr flag on any network/launch **`/AA`** action."* **Both qualifiers are
now wrong.** ⇢ *It was accurate when written, which is exactly what makes an
append-only Pass record dangerous when read in the present tense.* The right
repair is a **dated forward pointer** beside it, not a rewrite of history — and
whether decision 009's own framing needs amending is a call the librarian
declined to make alone.

★ **Already fixed, do not redo:** `docs/core-api/03-capabilities.md:651` cited
`scan_javascript` at `forms.rs:1653`; it is now at `forms.rs:1813` and the
filing corrected it.

---

## §D — ★ A NAMING DIVERGENCE, REPORTED AS AN OBSERVATION, NOT A DEFECT

`Pass 133.0`'s nine counters have **two** name sets, and **three of seven
diverge**:

| `FormJavaScript` field (`forms.rs`) | CLI stable-line key (`main.rs`) |
|---|---|
| `annotation_actions` | **`annot_actions`** |
| `javascript_actions` | **`js_actions_anywhere`** |
| `scan_truncated` | **`action_scan_truncated`** |

Neither set is wrong — one is the API a consuming crate binds, the other the key
a script parses. But **the mapping is written down nowhere outside the
`println!` itself**, `check-metrics-line-contract.py` governs **`render-page`
only**, and the commit message uses the CLI spelling while the dispatch used the
Rust one. **Both sets are recorded in the `Pass 133.0` ROADMAP entry** so a
future grep that finds one does not conclude the other is stale. Decide whether
to document the mapping in `docs/core-api/` or to converge the names; either is
fine, **drifting silently is not.**

Related, smaller: `list-fields` **appended** its new keys (per its own
append-never-reorder note) while `list-annotations` **inserted** `action=`
mid-line before `author=`. No published key-order contract exists for
`list-annotations` and no gate covers it, so nothing is broken — but a
**positional** parser would break, and one commit used two conventions.

---

## §E — THE WORK QUEUE

The operator's standing instruction: **continue the other, non-JavaScript
work.** The submit/scripting plan is **parked by his own ruling** — see `§G`.
Do not re-open it. `Pass 133.0` is **recognition only** and did not touch it.

Ordered by engineering judgement, not by Pass number:

0. **`§B` — commit the filing, run the gates on THAT tree, push, back up.**
   Ahead of everything: two trailing code commits already have the gate red,
   and nothing on disk backs them up.
1. **`§C` — the three doc survivors.** Cheap, and two of them sit in the
   document the next submit-plan session reads first.
2. **The sibling-gate check.** ★ Carried forward unchanged from the previous
   handoff and **still owed**. `check-string-gaps.sh` was widened in `ffe9d4c`
   after it proved blind to a defect whose shape `rustfmt` could not fold onto
   one line. The question: **do `check-ui-strings.sh` and
   `check-theme-colors.sh` carry the same post-formatting-shape assumption?**
   ★★ **These are the two line-scanners that EXIST** — earlier handoffs named a
   `check-strong-text.sh` that has never existed, a name read out of
   `check-string-gaps.sh`'s own header. A sibling carrying the assumption is
   what would earn the general form a `D:/dev/rag/rust/` file.
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
   clauses were discharged by `Pass 132.0`. **Banding is the third and it is
   untouched.** ⇢ *An operator-set ceiling trades memory for correctness;
   banding removes the trade* — peak usage is a multiple of nominal (parent +
   child + spare), so a ceiling admitting one large page admits three.
8. **CLI surface for the four ce-dimension group management verbs** — rename,
   delete, delete-with-policy, re-parent. Core ships all four; no subcommand
   reaches any of them, so from a script a group is still create-only (`R151`
   shape: callable-and-uncalled).

**Two `D:/dev/rag/` escalations are OWED and were deliberately not written**,
both at n = 1, both with a named second-occurrence trigger:

- ***In-crate tests are privileged in exactly the ways a consumer is not***
  (`#[non_exhaustive]` construction, private items) — **so an API can be green
  in CI and unusable from outside.** Found in `Pass 134.0` by writing the
  out-of-crate integration test. **Trigger:** the next `pdfceGUI` construction-wall
  report is the second occurrence.
- ***An infallible allocation is a bound's silent partner; a ceiling removed
  without `try_reserve` converts a typo into an abort.*** Carried forward from
  `Pass 132.0`. **Trigger:** actually induce an OOM (a 64 GiB ceiling on a
  machine that cannot honour it; check the process survives and discloses). No
  OOM has been induced, so the finding would rest on reasoning rather than
  measurement. The pdfce-visible half is already in `ARCHITECTURE.md` §10.1a
  and decision `089`.

---

## §F — WHAT `Pass 133.0` AND `Pass 134.0` DECIDED, IN CASE YOU TOUCH THE SAME GROUND

**No architectural decision was minted and no standing rule was minted** — that
is itself the finding, not an omission. Ceiling stays: decisions **089** (next
free **090**), rules **`R218`** (next free **`R219`**). ★ Both figures are
from `python tools/check-ledger-numbers.py`, run at filing time — **not** from a
prior entry: several filings between the 265th and 268th recorded the rule
ceiling as `R219`, which the gate and the last two filings both say is one too
high.

Four rulings that will bite a session working nearby:

1. **★★★ Recognise-and-report is a pdfce PRODUCT decision, not conformance.**
   The standard has **no threat model** — `malicious`, `privacy`, `untrusted`
   are **0 hits across both editions** (756 and 1023 pages) — and its posture is
   ***"shall execute"***. `R12`/`R13`/`R53`/`R54` are where this lives. ⇢ *A
   behaviour that exceeds the standard must be filed as a choice, or the next
   reader files it as a requirement and cannot tell what would be permissible
   to change.*
2. **★★★ An edit verb validates the POST-IMAGE, not the request.** A creation
   verb sees the whole field, so validating the request validates the file. An
   edit verb sees a **delta**, and a producer gate is a property of the field
   **after** it lands — which the request need never mention. Clearing
   `/MaxLen` breaks Table 228's comb gate without the word *comb*; clearing
   `combo` breaks Table 230's `Edit` gate without the word *editable*. **Both
   are among the four producer gates the standard gives no reader recovery rule
   for.** Recorded as a **named candidate at n = 1**, not minted.
3. **★★ When an edit outdates stored data, pdfce neither refuses nor repairs —
   it discloses.** Shortening a limit **is** a legitimate authoring act
   (refusing just moves the loss); truncating the value or re-pointing a
   selection is **inventing document state** (rules 3 and 4). The vehicle is
   `FieldEditOutcome::value_no_longer_fits`, which carries **a ready-made
   sentence**, not a boolean the caller must phrase.
4. **★★ A guard keyed on IDENTITY is disarmed by anything that normalises the
   identity away — and resolution is exactly such a normalisation.** The first
   `/Next` walk resolved before recursing, so the cycle guard never engaged and
   a `5 → 6 → 5` loop counted one `/URI` **sixteen times**. Every line was
   individually correct; the defect was the **order**, and it produced a wrong
   **number** rather than a crash.

**Where to read the spec, not your memory:**
`D:/Dev/Rag-Specialized/PDF_Spec/iso32000/iso32000__ref__action_carriers.md`
is **the catalogue and the one to grep** — 17 carrier sites, 10 container
types, 7 key names. `iso32000__s__12.6.md` beside it. Both written for
`Pass 133.0`.

---

## §G — THE SUBMIT / SCRIPTING PLAN IS PARKED. READ THIS BEFORE RE-OPENING IT

Full detail: **`docs/plan-scripting-submit-and-plugins.md`**. Eight operator
rulings are quoted verbatim there; do not paraphrase them from this file.
★ **Two of its sentences about pdfce's read side are now stale — see `§C`.**

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

★ **`Pass 133.0` shipped the READ half and nothing else.** It fires no trigger,
so it needed neither the `R54` amendment nor any new rule. **Do not read it as
progress on the plan.**

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
D:\Dev\FeatureRequests\pdfce_FeatureRequests\
D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\
D:\Dev\FeatureRequests\iccce_FeatureRequests\open\
```

★ **`ls` the channel ROOT as well as `open/`, and do it as its own command.**
`request_extraction_drops_the_writing_direction.md` (2026-08-26 16:29) sits at
the **root** and is **unanswered** — a session that lists only `open/` never
sees it. (`note_the_cmyk_ceiling_is_consumed.md` is also at the root, and is a
reply, not a request.)

★★ **A caution earned inside the 272nd filing itself:** `ls dirA dirB` in one
command prints two blocks with no obvious seam, and this role read an `open/`
entry as a root entry and then *"confirmed"* it. **Two commands, not one.**

**Both new Passes are already answered on the channel** — do not re-answer:
`open/reply_field_property_edit.md` (20:52) and
`open/note_the_action_scan_was_lying_and_now_is_not.md` (20:53).
★ **One thing the `Pass 134.0` reply should be checked for**, if it does not
already say it: **`move_widget` already existed**, public and CLI-wired since
`fd6eadd`, and the request's survey of `EditSession` verbs missed it **while
presenting itself as complete**.

Under `open/`, **four `request_*` notes remain unanswered**: `adopt_widget`
pre-flight, markup-opacity-in-two-verbs, `insert_pages` orphaned widgets, and
restyle-an-existing-text-run. The iccce channel's newest is
`note_your_name_gate_has_the_two_defects_mine_had.md` — **unanswered, and it is
about a gate this project also owns.**

**2. Run the gates — `ls tools/check-*`, do not trust any list**, including
`§A`'s count. `R209`: *"all gates green" names a set, and the set somebody runs
is not the set CI runs.*

**3. ★★ RUN `cargo +nightly fuzz build` ON WINDOWS.** CI cannot do it for you
and `cargo check --bins` is not a substitute. Two traps, each defeating the
obvious escape from the other: CI's fuzz job runs on `ubuntu-latest` and was
green throughout a window in which the harness was completely unbuildable here
(`rten` declares a `cdylib`, and only Windows hands that libFuzzer's
`/include:main`); and the documented local stand-in **passes in both states**,
because `cargo check` never links and the break was a link break. **A cheap
proxy for a gate is a proxy for the part of the gate that is cheap.**

**4. Read `docs/ARCHITECTURE.md` §12** — cross-project boundaries live there and
no gate can catch a violation of them. Newest is **`089`**, plus **§10.1a**;
nothing was added by `Pass 133.0` or `134.0`.

**5. Read `docs/compositor-plan.md`** before scoping anything in `97.x`, and
before items 3, 4, 6 or 7 of `§E`.

---

## §I — ★★ A DISPATCH IS A SET OF CLAIMS, AND YOURS WILL BE WRONG

Carried forward because it keeps earning its place, **and it earned it again in
the 272nd filing.** The 263rd filing's dispatch carried three factual premises
and **all three were false**. The `Pass 132.0` dispatch carried *"about 534 %
zoom on A4"* as an established fact; it was arithmetic on a different sheet, and
**it was caught by division, not by review.**

★★ **Fresh instance, and it is the smallest one yet, which is the point.** The
`Pass 133.0` commit message says ***"Fourteen tests"***, the dispatch to the
librarian repeated it, and **the real number is sixteen**:

```
git show afd8da8:crates/pdfce-core/src/forms.rs  | grep -c '#[test]'   →  34
git show afd8da8^:crates/pdfce-core/src/forms.rs | grep -c '#[test]'   →  18
```

The two uncounted tests are `an_action_on_a_nav_nodes_next_is_still_classified`
and `a_nav_node_chain_still_walks_as_nodes` — **the `/Next` × navigation-node
cross-product**, precisely the pair a round number omits. ⇢ ***The count was
written before the last two tests were added, and nothing re-derived it.***
Hard rule 10's whole point: **the correction required no new work, only
`grep -c`.**

**Write dispatches so a premise is checkable, and expect the agent to check.** A
dispatch that says *"X is at path P"* invites verification; *"as we discussed"*
does not. **Finish the code, then dispatch, then commit the filing last.**

---

## §J — THE FILING COMMIT MUST BE THE LAST COMMIT BEFORE ANY TAG — AND ★ THE DEFERRAL IS EXACTLY ONE COMMIT WIDE

`check-commits-filed.py` counts commits that no filing names. The tip-deferral
excuses a commit that cannot cite its own hash — **but only while it is the
tip**. The instant a filing lands on top, the excuse evaporates and the gate
flips red without anything about that commit changing.

> **Dispatch the librarian LAST, and commit its filing LAST.** Any code commit
> made after the dispatch has, by construction, no filing that can name it.

★★ **This was live an hour ago and is now DISCHARGED, which is why the
mechanism is worth keeping in view.** `afd8da8` and `fd71e4f` were **two**
trailing unfiled code commits against a **one**-commit deferral; `e0a432e` names
both, so the gate is green and `e0a432e` is the tip. ★ **The green is
conditional from here:** the uncommitted `docs/` corrections will become the new
tip, which is safe because **a docs commit needs no filing** — but **a code
commit landing on top of `e0a432e` before a tag would not be.**

⇢ ***A one-commit deferral tolerates one trailing code commit. The second is
not deferred — it is merely no longer the tip.*** Recorded as a **named
candidate under `R217`**, **still n = 1**, not minted; the mint is the
operator's act. ★ **The present state is NOT a second occurrence and must not
be counted as one** — `R217`'s candidate is about a *filing landing and then
more code being committed on top*, which is a **failure**; two code commits
followed by their filing is the **normal, correct** order and goes green the
moment the filing lands. ⇢ *Counting a routine state as an instance of a
failure shape is how a candidate gets promoted on evidence it does not have.*
The orders that work are **(file → code → file)** or **(code → file, then
stop)**.

★ **`afd8da8` and `fd71e4f` are both named by the 272nd filing (`e0a432e`)** —
in `ROADMAP.md`'s two new *Shipped* entries and in `SESSION_LOG.md` — **verified
green here** (`python tools/check-commits-filed.py`). Nothing is outstanding.

Recovery if a tag goes out on a red run anyway (precedent: `v0.8.0`, `v0.10.0`,
`v0.12.0`, `v0.14.0` — **four times**): file the orphan, re-tag at the filing
commit, force-push the tag, **rebuild the package** so `BUILD-INFO.txt` names
the tagged commit, replace the asset with `--clobber`, and **re-run the smoke
test on the new artefact** — a re-cut release is a new artefact and does not
inherit the old one's test.
