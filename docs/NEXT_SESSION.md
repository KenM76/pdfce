# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. Overwrite it once acted on.

**Written 2026-08-18**, replacing the earlier 2026-08-18 handoff whose task
(`Pass 75.0`) is now shipped.

---

## ★★★ THE TASK: reply to `pdfceGUI`, then build what the reply promises

`D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\` contains **one unanswered
request**, and it is about code that shipped the same day:

```
request_insert_pages_leaves_orphaned_widgets_and_has_no_route_back_for_outlines.md
```

**★ THE PREVIOUS HANDOFF SAID THAT CHANNEL WAS EMPTY. It was not, and I
repeated the claim to the librarian before checking.** The failure mode is
worth more than the correction: **that directory is outside the repository, so
no gate will ever contradict a stale sentence about it.** The two `iccce`
requests have been demonstrating the same thing for three filings. `ls` both
channels at session start — it costs nothing and it is the only thing that
works.

**Filed as `Pass 102.0` / `102.1` / `103.0` / `103.1` / `103.2` / `103.3`.**
`102.0` (the orphan count) is the one to do first: it is small, it is ruled
**permanent rather than interim**, and it turns a consuming project's
currently-false disclosure into a true one.

### What it says, in one paragraph

`insert_pages` (`Pass 99.0`, `38c0ef2`) copies everything reachable from a
page, and a page's `/Annots` reaches its widgets. So **widgets DO arrive while
`/AcroForm` does not** — they measured 13 widgets with `fields=None`. The
result is not "the form fields did not come across". It is **boxes that draw
exactly like form fields, that an operator will click, and that nothing can
fill**. A visible control that is silently inert, arriving through a document
instead of through a ribbon. They wrote their disclosure from my reply without
checking, which they own; the disclosure named the wrong failure, and *a
disclosure that names the wrong failure is worse than none, because it is
believed.*

### The four options they offer, and ★ my reading, which sharpens theirs

1. carry field definitions for fields whose widgets are all on inserted pages
2. strip the orphaned widgets
3. **report a count** of orphans so a shell can be precise
4. refuse a source page with widgets unless a flag is passed

Their preference is *(3) now, (1) later*, treating (3) as the cheap interim.

**I think (3) is permanent and (1) is a layer on top of it.** A field's widgets
can be **split** across inserted and non-inserted pages, and (1) cannot carry
such a field without either fracturing it or dragging in widgets nobody
inserted. **A residue of orphans survives (1) by construction**, so the count
has to exist forever.

⇒ **(3) unconditionally, (1) as the follow-on, never (2) or (4).** (2)
destroys content the operator can see in the source; (4) turns a disclosable
condition into a refusal.

### The four API asks in part 2

- `EditSession::add_outline_item(parent, title, dest)` — **no verb exists**
- **adopt an existing widget into a field** — today's `add_text_field` and
  friends author a *new* widget; they need to register geometry already correct
- **page labels for inserted pages** — they explicitly ask this NOT be answered
  by the existing `/PageLabels`-stays-stale ruling at `edit.rs:16339`
- **named destinations**, so a carried bookmark resolves

### ★ The part worth carrying into our own rules

Their draft declined to ask for the last two, citing **R151** and my own reason
for not shipping `markup_rects`. Their operator overruled it:

> *"not adding such things just because they weren't explicitly asked for i
> think is how we end up with partially finished features."*

The distinction they had collapsed is a good one. **`markup_rects` was a
convenience query duplicating something already reachable**, so no caller meant
no value and declining was right. **Page labels and named destinations are not
a separate feature — they are the missing members of ONE feature.** A
document's pages carry labels and are pointed at by destinations. Shipping
Insert with two of four handled makes Insert permanently partial, and the
partiality gets found by a user rather than by us.

**No reply has been written into the channel.** That is owed, and it is owed
deliberately rather than forgotten — it was not worth doing badly at the end of
a long session. `ROADMAP.md` carries a standing box under *Next up*
enumerating the five things the reply must cover.

### What was fixed on our side already, so the reply can say so

- `edit.rs`'s `insert_pages` doc comment — which was **the source of their
  wrong disclosure** — now states the widget/field split in full, with the
  measured counts and an explicit instruction not to paraphrase it as "form
  fields did not come across".
- `docs/core-api/02-editing-and-saving.md` **never mentioned `insert_pages` at
  all**, which is why a chat reply was their only source. It was **eight verbs
  behind**; all eight are now documented and the stated count is corrected
  108 → 116.
- `tools/check-core-api-verbs.py` (new) re-derives the verb list from
  `edit.rs` and fails if the doc omits one or states a wrong count.

---

## ★★ TWO MORE REQUESTS LANDED AT 19:22 AND 19:23 — three minutes after the
## commit that fixed the first one

```
request_a_placed_ce_dimension_cannot_be_moved_to_another_group.md
request_a_dimension_group_can_be_created_and_never_renamed_or_deleted.md
```

Found by `ls`-ing the channel per `R196`, **not** by inheriting the previous
filing's "one open request". That is the rule earning its keep within hours of
being minted, and it is the reason the count in any handoff — including this
one — is hearsay until re-listed.

**Both verified against the source rather than taken on trust:**
`grep -cE "pub fn (rename_dimension_group|delete_dimension_group|set_dimension_group|move_dimension_to_group)"`
returns **0**. `add_dimension_group` exists at `edit.rs:17692`,
`add_dimension` takes its `GroupId` at creation (`17549`), and nothing ever
changes it.

### ★ They are the SAME SHAPE as the `insert_pages` request, and that is the story

All three are *"missing members of a shipped cluster"* — the requester's own
phrase, and precisely the `R151` qualifier their operator articulated. Not
speculative features; the absent members of clusters pdfce already ships and
`FEATURES.md` already ticks. Three in one day suggests the pattern is
systematic rather than coincidental, and the honest question for next session
is **whether other shipped clusters have the same holes and nobody has hit
them yet.** A sweep for verbs that create a thing without a verb to rename,
re-parent or delete it would answer that cheaply.

### My parse, for whoever scopes them

- **`set_dimension_group(DimensionId, GroupId)`** — not a field assignment. A
  ce dimension's *appearance* derives from its group's scale, precision, units
  and standard, so re-parenting must **re-measure and re-bake the `/AP`**, and
  it must be ONE undo entry. The refusal cases are the interesting part: a
  target group on a different page, and a group whose unit is incompatible.
- **`rename_dimension_group(GroupId, &str)`** — small, and the only question
  is whether names must stay unique. If they do, that is a refusal with a
  name; if not, say so, because a UI listing two identical groups is a
  discoverability defect the shell cannot fix.
- **`delete_dimension_group(GroupId, …)`** — ★ **this is the orphan question
  again, in a second place.** What becomes of the dimensions in the group?
  Reassign to a default, delete with it, or refuse while non-empty. The
  `insert_pages` answer generalises: **report and refuse to guess**, with the
  count, rather than picking a fate silently. Whatever is chosen here should
  match `Pass 102.0`'s shape so a shell learns one convention rather than two.

The librarian filed all three **unscoped, with no Pass ID claimed** — parsing
them into Passes is the engineer's act and has not been done.

---

## §1 — WHAT SHIPPED 2026-08-18 (this session)

| commit | |
|---|---|
| `e13f8ed` | `Canvas` plumbing — 16 + 2 signatures, provably transparent |
| `6af5655` | **`Pass 75.0`** — the display list |
| `6b797db` | poster printing fixed (it failed outright above ~2× magnification) |
| `2aa1066` | **`Pass 101.0`** build stamp · string-gap gate · guard discharge |
| `85c9cb4` | librarian filings 182–183 · the widened ledger anchor |
| `e194b46` | three sweep survivors · `docs/core-api` was 8 verbs behind · new gate |

### `Pass 75.0` — the numbers, so nobody re-derives them

Three runs, medians, release, `examples/region_bench.rs`, A3 CAD sheet
(148,517 paints · 24,128 clip ops → **127,267 ops, 40 clips, 29.5 MiB**):

| case | from stream | replayed | ratio |
|---|---:|---:|---:|
| **FLOOR** 1×1 pt, 2 px | **636 ms** | **1.06 ms** | **600×** |
| region 400×300 pt, scale 1 | 680 ms | 83.5 ms | 8.1× |
| region 400×300 pt, scale 8 | 819 ms | 10.5 ms | **78×** |
| recording the page | — | 618 ms | — |

Read the FLOOR row first: almost no fill in it, so it measures interpretation
and nothing else, and interpretation is **gone** from the second render. Full
write-up in `docs/render-region-measurements.md`.

**The key is `(page, epoch, scale)`** — the consumer asked for `(page, epoch)`.
Scale was added because half the interpreter's decisions are device-dependent
(hairline, image minification, edge anti-aliasing, mask size) and because
composing the transform in a different order is not associative in f32.
Panning at fixed zoom is fully served; a zoom step costs one rebuild.
**Decision 071.**

**~2.4 % of pages refuse to record** (shading, overprint composite, soft mask)
and fall back — measured over 3,222 loadable files, so roughly one page in
forty.

---

## §2 — THE QUEUE, in the order I would take it

1. **The `pdfceGUI` reply + `(3)` orphan count** above. Small, unblocks a
   consumer, and corrects a false disclosure now in their shipping UI.
2. **`Pass 97.0 / 97.1 / 97.2`** — the colorant compositor. **~16 of the 18
   remaining Ghent failures**, still the highest-impact item in the project.
   Plan of record: `docs/compositor-plan.md`; collapse model sourced in
   `docs/collapse-model-survey.md`.
3. **`Pass 80.0`** (note text on markup) and **`Pass 81.1`** (markup opacity,
   write half) — both `pdfceGUI` requests, both already scoped.
4. **`Pass 98.0`** — read a foreign `/BE` back into `MarkupSpec`.
5. **`Pass 101.1`** — iccce provenance, **BLOCKED** until pdfce actually
   depends on iccce. See §4.

**Ghent standing unchanged: 25 pass / 18 FAIL / 8 UNRESOLVED of 51.** The GWG
Reference file is still not on this machine.

---

## §3 — THE LEDGER ANCHOR: DONE, and what it found on the way

`tools/check-ledger-numbers.py`'s Pass-heading anchor was widened from
`(?:★ )?` to `(?:★+ )?` and is **committed** (`85c9cb4`). Nothing is held back.

Widening it immediately surfaced a **real, pre-existing `Pass 85.5`
duplicate** the single-star anchor had never been able to see. Ruled by the
librarian as **one Pass in two stages** — now `Pass 85.5 (compositing)` and
`Pass 85.5 (Table 149 logic)` — on evidence that predates the gate being able
to read those lines: `bd9d5ef` already calls itself *"slice 1 of the 85.5
simulation half"*, and the two commits are 29 minutes apart. Unlike the
`Pass 75.0` ruling, this one moves no number and creates no immutable-history
debt.

**The transferable finding:** this is the *second* time this anchor's blind
spot was found, and **both times it was found by somebody predicting the gate's
output and disagreeing with it** — never by the gate reporting anything. **A
gate that under-reports is byte-indistinguishable from a green one**, so the
only detector is an independent forecast, which is the exact labour a gate
exists to remove. The first fix repaired the one spelling that had been seen
(`★`) rather than the class, so a convention that uses one to three stars by
weight stayed half invisible.

★ **And the same shape turned up in a document the same day**, which is why it
is worth carrying rather than filing under "regex". `docs/core-api/` asserted
`**Count: 108.**` *and showed its derivation* while being eight verbs behind.
**A stated derivation reads exactly like a maintained derivation** — a precise,
sourced, stale number is more durable than a vague one, because it deters the
check that would catch it. Both cases: something that looked audited, wasn't,
and nothing could tell the difference from outside.

---

## §4 — iccce

- **pdfce does not depend on iccce.** Measured, not assumed: zero matches in
  any `Cargo.toml` and zero in any source file. Decision 064 records the
  *boundary*; a boundary is not a dependency edge. `--version` therefore prints
  `iccce: not-linked` **with the reason**, because the operator asked for that
  revision by name and an omitted line reads as an oversight.
- iccce is at `D:\Dev\iccce`, `v0.1.0-19-g400179b`.
- **`D:\Dev\FeatureRequests\iccce_FeatureRequests\open\` holds FIVE files, not
  the two the last handoff named** — three `note_*` and two `request_*`. I did
  not read any of them and am not claiming to know which are owed by whom.
  **Establish that rather than inheriting my count.**

---

## §5 — TRAPS THAT COST TIME TODAY

- **★ I ran `git checkout -- crates/` to undo a partial script run.** My own
  agent-memory note forbids exactly that. It destroyed three uncommitted edits
  to **tracked** files while leaving the new **untracked** files alone — which
  is what makes the damage easy to miss, because most of the work was still
  there. Undo by editing.
- **A `\` continuation inside a string literal loses its backslash to patch
  tooling and `rustfmt` bakes the padding in as literal spaces.** This bit
  twice today, once in shipped code. There is now a gate:
  `tools/check-string-gaps.sh` (`--self-test` included). **A long single-line
  literal is the safe form** — rustfmt leaves it alone.
- **A cache's win and its cost do not live in the same case.** The first
  recorder was 88× faster at scale 8 and **2.5× slower than no cache** at
  scale 1, because at deep zoom almost every op culls before its clip is
  requested, so the expensive path was never taken. **Measure the case where
  the cache does the most work, not the case that motivated it.**
- **A test that guesses where content is tests the fixture, not the code.**
  Two wrong guesses (tile 0 = page margin; mid-grid = line leading) before the
  poster test was changed to *locate* the inked tile from a cheap render.
- **Run every new regression test against sabotaged code.** Three sabotages
  today, three different tests caught them — and the broad byte-identity test
  did **not** catch a too-tight cull, which is exactly why the dedicated case
  exists.

---

## §6 — STATE AT HANDOFF

- Gates **all clean**: `string-gaps` (new), `core-api-verbs` (new),
  `ui-strings`, `ledger-numbers`, `disclosure-channel`, `passes-filed`,
  `commits-filed`, `one-commit-per-command`. `cargo fmt --check`,
  `clippy --all-targets --workspace -D warnings`, `cargo test --workspace`,
  and the wasm32 cross-check are green.
- **Two new gates landed today**, both because a defect was found by hand that
  a grep could have found: `tools/check-string-gaps.sh` (ported from
  `pdfceGUI`) and `tools/check-core-api-verbs.py`. Both have been verified
  against sabotaged input; neither has ever been seen only to pass.
- Working tree: clean apart from whatever the librarian is holding in `docs/`.
- **v0.7.0 is bumped but NOT tagged.** The operator gave a standing go-ahead
  for builds/releases on 2026-08-17. Verify CI green on `HEAD`, then
  `verify-release.py` → tag → portable package → GitHub release → librarian
  release record. **A release build now stamps its own provenance**; note that
  a CI-built release would report `revision: unknown` unless that workflow
  gets `fetch-depth: 0` (depth-1 checkout has no tags).
- **`MAX_DISPLAY_LIST_BYTES` veraPDF discharge: DONE this session** —
  `examples/guard_probe.rs`, 3,245 files, 0 firings. The largest observed list
  is **41.9 MiB** (a §6.1.12 conformance file, **not** the CAD sheet), so real
  headroom is **6.1×**, correcting a claim of 8.5×. The librarian **widened the
  standing rule** rather than waiving it: the "fire on a real file" half is
  undischargeable for a ceiling no real input reaches, and a **measured
  non-zero maximum** defeats the reading that half exists to defeat. Successor
  obligation recorded: re-sweep whenever per-op cost changes, and **re-derive
  headroom from the maximum, never from a reference document.**
- **The `iccce` channel holds 16 files, not the 2 or the 5 previously claimed**
  — 5 requests / 6 replies / 5 notes, direction read from each file's own
  header. The two owed requests were the right count with the wrong
  description. **★ `note_gray_black_routing_is_yours.md` (newest, 13.5 KB) is a
  boundary ruling handing pdfce the four-way gray/CMYK/Separation/DeviceN black
  equivalence — the highest-value unread file in that channel, and not a
  request.** Also: one stray duplicate reply to delete, three answered
  exchanges awaiting archival.
- Ledger: next free Pass family **104**, decision **072**, standing rule
  **R197**, filing ordinal **185**. **Re-measure with
  `tools/check-ledger-numbers.py` rather than trusting this line** — that is
  what it is for, and this line has been wrong before.
