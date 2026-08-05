# Decision 031 — Where implicit commit stops: operator manipulation, pdfce inference, and a third axis neither one names

**Status:** Decided (engineer's line, confirmed by `pdfce-ui-specialist`, with
one addition from the specialist). Recorded by `pdfce-librarian` per the
engineer's explicit dispatch, 2026-08-05.
**Date:** 2026-08-05
**Requested by:** the operator, verbatim, 2026-08-05 (quoted in full in the
Pass 34.x `ROADMAP.md` entry this decision accompanies): *"There should be
no separate accept/reject when editing the text - if I click out of where I
am editing that should just accept the edits... this goes the same with all
tools."*
**Filed by:** `pdfce-engineer`'s classification, confirmed and extended by
`pdfce-ui-specialist` (`docs/ui_specs/tool-options-dock-and-ce-dimension-properties.md`
§B.7's new finding), recorded by `pdfce-librarian`.
**Amends:** nothing. **Extends** decision 024 §4.4 (the CLAUDE.md rule 4
narrowing) by applying its already-settled line to a new gesture inventory,
and adds one classification decision 024 did not need to make (§3 below).
**Implements, alongside Pass 34.0:** the `GestureInterrupt::Commit` wiring
this record's §2 line governs.

**Decision number 031 — verified against the live ledger at write time, not
assumed (R106).** `docs/decisions/` contains `001`–`030` with no gaps at
write time (`030-preserving-the-option-of-a-future-plugin-system.md`, filed
by `autonomous-builder` earlier the same day), so **031 is the next free
number**. It is not free by accident, and the chain is worth recording in
full because it has already moved twice: decision **029** reserved **030**
for a possible Pass 33.0 fix record; decision **030**'s own §9 item 2 then
spent 030 on itself and moved the reservation to **031**; this record now
spends **031** on the question below, per the engineer's explicit dispatch
of `pdfce-librarian` to open it here rather than hold it for Pass 33.0. **A
Pass 33.0 fix record, if one is ever written, now takes 032.** See the
corresponding `ROADMAP.md` R148 amendment (the second `[SUPERSEDED...]`
bracket) for the ledger-side half of this correction.

**Terminology (`CLAUDE.md` rule 15, binding throughout).** Every
dimensioning object this record discusses is a **ce dimension** — the
`/Line`+`/IT /LineDimension` annotation family pdfce itself authors
(`crates/pdfce-core/src/dimension/`), never a **pdf dimension** (a
pre-existing CAD-exported callout). `MeasureScale`, `MeasureLinear` and
`MeasureCircular` are the tools that author ce dimensions; nothing here
concerns pdf dimensions.

**Cross-references:** `CLAUDE.md` rule 4 (fuzzy, never sneaky) as narrowed
2026-08-05 by decision 024 §4.4; `ARCHITECTURE.md` §12 (this record's log
entry); `ROADMAP.md` Pass 34.0 (the `GestureInterrupt::Commit` wiring this
record governs the boundary of), Pass 34.1 (the docked Tool Options pane
this record's mitigating argument depends on), standing rule **R150**
(the gesture-asymmetry process lesson filed alongside, distinct from this
decision's content); decisions **017 Amendment A** (`egui_tiles` adoption —
the dock mechanism §4 below builds on), **024** (the ribbon/confirm record
this decision's §1/§2 directly extend, especially §4.1's diagnosis that the
operator's complaint was about *placement*, not about the existence of a
confirm step, and §4.4's exact narrowed rule-4 wording); `docs/ui_specs/tool-options-dock-and-ce-dimension-properties.md`
§B (the full per-tool/gesture table this record's §2 summarizes) and its
§A.1 (the dock architecture question §4 below records).

---

## 1. What is decided

1. **Operator-authored direct manipulation commits implicitly, on
   click-out (or tool-swap, or Enter where that convention already
   exists).** Typing a text replacement, dragging a node, placing a ce
   dimension by two literal picks — none of these has anything in it for
   the operator to "review" that they did not put there themselves. Rule
   4 as narrowed by decision 024 §4.4 already says so; this record applies
   that line to the full gesture inventory the operator's 2026-08-05
   request touches (§2).
2. **A pdfce-INFERRED result keeps an explicit, disclosed review step.**
   Reflow's engine-computed line breaks, `MeasureCircular`'s best-fit
   residual, and a derived-centerline confirm all keep their Accept/Reject
   pair, unchanged. These are exactly the cases rule 4's obligation was
   always about — a value pdfce guessed, not a value the operator typed
   or dragged.
3. **A third axis exists, distinct from both of the above, and it
   produces one named exception: `MeasureScale`.** A back-calculated
   scale is authored, not inferred — by the letter of rule 4 alone it
   would be case (a), implicit-commit. It is recommended to keep its
   explicit confirm anyway, because of **blast radius**, not inference.
   §3.
4. **The dock implementation is one `DockPanel` enum reused by TWO
   `Tree<DockPanel>` instances** (one per side of the window), sharing one
   `DockBehavior`, rather than genericizing `DockBehavior` over a pane
   type or writing a second, sibling `Behavior` impl. §4.

**What is deliberately NOT decided here:** the exact pixel geometry of the
new left dock (an engineer/ui-specialist judgment call against the running
app, per `docs/ui_specs/tool-options-dock-and-ce-dimension-properties.md`'s
own closing note); whether `MeasureScale`'s exception is final — that is
open operator question **(aw)**, not resolved by this record.

---

## 2. The line, restated for this gesture inventory

Rule 4, as narrowed 2026-08-05 (decision 024 §4.4, `CLAUDE.md` rule 4):

> "Where an inference is *inherently* uncertain... the uncertainty is
> stated in the disclosure... It is **not** satisfied by a control whose
> position is derived from the document. And it does **not** require a
> two-click confirmation for a direct manipulation whose result is fully
> visible on the canvas and reversible in one undo."

Applied to the operator's 2026-08-05 request, per tool
(`docs/ui_specs/tool-options-dock-and-ce-dimension-properties.md` §B.7's
table, reproduced here as the decision record of the classification
itself, not merely a design note):

| Tool / gesture | Class | Commit posture |
|---|---|---|
| TextEdit plain find/replace | operator-authored | implicit, on click-out / tool-swap / Enter |
| AddText authored run/box | operator-authored | implicit, on click-out (extending the existing Enter/Ctrl+Enter convention) |
| MeasureLinear plain two-point pick | operator-authored | implicit, on click-out, once both points are picked |
| VectorEdit node/subpath/object drag | operator-authored | **already shipped this way** — cited as the working precedent, not a hypothetical |
| ce-dimension position drag (Pass 27.x) | operator-authored | **already shipped this way** — the second working precedent |
| TextEdit Reflow draft | pdfce-inferred (engine-computed line breaks) | explicit Accept, unchanged |
| MeasureCircular best-fit | pdfce-inferred (Taubin fit, real residual) | explicit Accept, unchanged |
| Derived-centerline confirm | pdfce-inferred (fuzzy, in-code tagged) | explicit Accept, unchanged |
| **MeasureScale back-calculation** | **authored, but blast-radius exception (§3)** | **explicit Accept, recommended kept** |

**Mid-gesture states that have not yet formed a complete, committable
draft are not a commit decision at all** — one of two `MeasureLinear`
picks taken has nothing for undo to protect and nothing rule 4 governs.
`current_gesture_interrupt`'s existing `pending.is_some()` check is
already the right branch condition; this is Pass 34.0's implementation
detail, not a separate classification this record needs to make.

**One gesture where implicit commit is affirmatively wrong, named so it is
not accidentally swept into "commits like everything else":** an empty
`AddText` draft (a point placed, nothing typed, click-away). Auto-committing
would add a real but invisible, zero-content object to the document for no
operator-visible reason. `current_gesture_interrupt`'s `AddText` branch
checks for non-empty content before returning `Commit`; empty content stays
`Discard`. This is not a rule-4 case either — nothing was authored into a
state worth protecting — but it is worth naming because "click-out commits"
read carelessly could produce exactly this foot-gun.

---

## 3. The blast-radius axis — why `MeasureScale` is the one named exception

`pdfce-ui-specialist` identified this classification; it was not in the
engineer's original two-way split and is the one genuine addition this
record makes to decision 024's already-settled line.

**The test decision 024 §4.4 gives is authored-vs-inferred, and
`MeasureScale` passes it as authored.** A back-calculated scale
(`real_length / drawn_pdf_length`, or a typed ratio) is deterministic and
exactly reproducible from what the operator typed. By the letter of the
narrowed rule alone, it reads as an ordinary case-(a) direct manipulation,
no different from a text edit.

**It fails a third test, distinct from "is it inferred": whose state does
it touch, and how much of it, at once?** Every other case-(a) commit in §2
changes exactly the one ce dimension the operator is looking at, at the
position they are looking at it. A `MeasureScale` commit changes the
**displayed value of every other member of the group simultaneously** —
ce dimensions elsewhere on the page, possibly off-screen, that the
operator is not looking at when the click-out lands. The existing code
comment at `main.rs:13413` already states this consequence in its own
words: *"a calibration silently rescales every dimension in the group."*

**Why this is not covered by rule 4 as written, and is not meant to be.**
Rule 4 (as narrowed) is about whether pdfce GUESSED something, and blast
radius is orthogonal to that question — nothing about `MeasureScale` is
guessed. This record does not propose widening rule 4 to cover blast
radius generally; it names `MeasureScale` as a **specific, bounded
exception**, argued on its own facts, not as a reinterpretation of the
rule for every wide-effect operation. Whether a future wide-effect gesture
deserves the same treatment is a case-by-case judgment, not something this
record generalizes in advance.

**Recommendation: keep `MeasureScale`'s explicit Accept/Reject, named
explicitly as a deliberate exception to the case-(a) default, not left as
an unresolved case-(b) classification.** The distinction matters for
whoever reads this later: `MeasureScale` is not miscategorized as
"probably inferred, needs more thought" — it is correctly categorized as
authored, and kept explicit anyway, for a stated and different reason.

**This is a DEVIATION from the operator's literal instruction** — "this
goes the same with all tools" — and it is being surfaced to him rather
than decided silently on the engineer's or the specialist's authority
alone. See open operator question **(aw)**, filed alongside this record in
`ROADMAP.md`. **Default posture pending his answer: keep the explicit
confirm.**

**The mitigating argument, which is why this exception is a smaller ask
than it would have been a day earlier.** Decision 024 §4.1 diagnosed the
operator's original complaint as being about *placement* — a confirm box
floating at a position derived from the page image, drifting on zoom/
scroll/page-change — not about the existence of a confirm step itself.
Once Pass 34.1 lands, `MeasureScale`'s confirm lives in the fixed,
always-visible Tool Options dock pane, not a floating `egui::Area` pinned
to the canvas. It is no longer the "accept/reject box somewhere on the
screen... I've never seen any other software operate that way" complaint
that motivated rule 4's narrowing in the first place — it is a fixed
control in a fixed panel, the same shape as the reflow and best-fit
confirms the operator has not objected to. That does not make the
exception free, but it changes what it costs him.

---

## 4. Dock implementation shape — one `DockPanel` enum, two `Tree` instances, one `DockBehavior`

`docs/ui_specs/tool-options-dock-and-ce-dimension-properties.md` §A.1
raised, and left to the engineer, the question of whether the new
left-hand dock (Pages | Tool Options) shares its `Behavior` mechanism with
the existing right-hand dock (Objects/Properties/BatchTools/Redact) by
genericizing `DockBehavior<'a>` over the pane type, or by writing a
sibling `Behavior` implementation for a new pane enum.

**Decided: a new, small `LeftPanel` enum (`{Pages, ToolOptions}`), a
SEPARATE `Tree<LeftPanel>` instance for the left side, and the SAME
`DockBehavior` mechanism reused as-is** — not genericized, not
duplicated as an independent trait implementation. The reasoning:

- **`egui_tiles::Tree` instances are independent** — the library gives no
  mechanism for panes to move between two `Tree`s, and none is wanted
  here (Pages and Tool Options are not meant to migrate to the right-hand
  dock or vice versa). So there is no cross-tree behavior to unify in the
  first place; genericizing `DockBehavior<'a>` over the pane type would
  buy nothing a `match` in `panel_body` does not already buy for free.
- **The existing `panel_body(...)` dispatcher's `match` on `DockPanel`
  already survived one prior architecture change verbatim** (decision
  017 Amendment A's own record: `enum DockPanel` + one `panel_body(...)`
  dispatcher "survives verbatim as originally designed — it becomes the
  `egui_tiles` pane content" — `ARCHITECTURE.md` §12, continuation-57
  entry). Reusing the same shape for `LeftPanel`/its own dispatcher
  extends a pattern already proven to survive a library-adoption change,
  rather than introducing a generic parameter whose interaction with
  `egui_tiles`'s own trait bounds (`Behavior<Pane>`) is unverified.
- **Two small `Behavior` impls, both thin dispatchers, cost less than one
  generic one.** `DockBehavior` is not a large type; duplicating its
  mechanical parts (tab title lookup with R84's bold-on-active rule,
  `simplification_options`'s `all_panes_must_have_tabs: true` gotcha) into
  a second small module is cheap at this size, and it avoids a generic
  bound that egui_tiles's own trait may or may not accommodate cleanly —
  exactly the kind of implementation-shape uncertainty the ui-spec
  explicitly declined to resolve from reading source alone (§A.1's own
  text: "an implementation-shape call best made against `egui_tiles`'s
  actual trait bounds, not from reading source alone").

**What this decision does NOT do:** merge the two dock trees into one,
widen `DockPanel` to cover `Pages`/`Tool Options`, or introduce any
mechanism for a pane to move between the left and right docks. Both
docks stay architecturally independent; only the *implementation
mechanism* (the `Behavior` shape) is shared.

---

## 5. What this record does not decide

- **Does not** resolve open operator question (aw) — the `MeasureScale`
  exception is a recommendation, not final, until the operator answers.
- **Does not** change any shipped gesture's behavior by itself — Pass
  34.0 is the code change; this record is the classification governing
  its acceptance criteria.
- **Does not** widen rule 4 to cover blast radius as a general concept —
  §3's exception is bounded to `MeasureScale`, argued on its own facts.
- **Does not** authorize genericizing `DockBehavior` — §4 decides the
  opposite (two small, independent `Behavior` impls).

## 6. References

- `CLAUDE.md` rule 4 (fuzzy, never sneaky), rule 15 (ce vs pdf dimension
  terminology).
- `ARCHITECTURE.md` §12 (this record's dated log entry).
- `ROADMAP.md` Pass 34.0 (`GestureInterrupt::Commit` wiring), Pass 34.1
  (the docked Tool Options pane), standing rule R150 (the sibling process
  finding filed alongside, on the *code* side of the same Pass 34.0
  defect — distinct content from this record).
- Decision **024** §4.1 (placement, not existence, was the original
  complaint), §4.4 (the narrowed rule-4 text this record applies).
- Decision **017 Amendment A** (`egui_tiles` adoption; `DockPanel` +
  `panel_body` surviving verbatim as precedent for §4's reuse argument).
- `docs/ui_specs/tool-options-dock-and-ce-dimension-properties.md` §A.1
  (the dock-implementation question), §B.6–§B.10 (the full gesture
  classification and the empty-AddText-draft exception), §C (out of
  scope for this record — the ce-dimension property surface is Pass
  34.2/35.0/35.1's territory, not a commit-boundary question).
