# Decision 027 — Refuse what has no good reading; disclose what has one (clipping-path edits, and the disclosure return channel)

**Status:** Decided and **already implemented** — filed post-hoc against
shipped code (Pass 30.0, commit `a56bdd7`, 2026-08-05).
**Date:** 2026-08-05
**Requested by:** pdfce-engineer (referral at Pass 30.0's ship: *"This may
warrant an `ARCHITECTURE.md` §12 decision-log entry; use your judgment on
whether it rises to that, and assign the next free decision number if
so."*)
**Filed by:** `pdfce-librarian`.

**★ Provenance, stated up front because it deviates from this
directory's protocol.** `docs/decisions/README.md` describes these files
as the Markdown output of the `autonomous-builder` (KenAgent) decision
consultant. **This record is not consultant output.** No consultant was
dispatched; the decision was made by the engineer during Pass 30.0 and
referred to the librarian for filing. The file exists here anyway for two
concrete reasons:

1. `tools/check-ledger-numbers.py` derives the live decision ceiling from
   `docs/decisions/NNN-*.md` **only**. A decision recorded solely in
   `ARCHITECTURE.md` §12 is **invisible to the checker**, so the next
   session would be told `next free is 027` and could mint a colliding
   number. The checker is the R133 guard against exactly this.
2. `ARCHITECTURE.md` §12 is the canonical *index*; per this directory's
   README, entries there *"should cross-reference the `NNN-*.md` file
   here when one exists."*

**Decision number 027** — verified against the live ceiling, not assumed.
`python tools/check-ledger-numbers.py` at filing time reported
`decision records: 026 -> next free is 027`. Same run:
`standing rules: R145` (after this filing's R143–R145) and
`Pass families with headings: up to 32`. Re-run the checker before
minting anything after this file.

---

## 0. Summary

Pass 30.0 made `re` rectangle corners draggable. In real PDFs, an `re`
rectangle is very often a **clipping path** (ISO 32000-1 §8.5.4's
canonical `re W n` idiom), so the same change made **clip geometry
editable** — and editing a clip changes **which other content on the page
is visible**, in a place other than where the operator is looking.

Two questions had to be answered together:

1. **Posture** — is editing a clipping path refused, confirmed, or
   disclosed-and-proceeded? And why does *deleting part of* a clip
   (refused since Pass 25.2) get a different answer from *moving* one?
2. **Mechanism** — where does a disclosure even go? Five `EditSession`
   verbs returned `Result<(), EditError>`, a shape with no channel for
   "this worked, and here is a consequence you could not see."

**Answers:** moving a clipping path is **disclosed**, deleting part of
one **stays refused**; and the return type changes so a disclosure has
somewhere to live.

---

## 1. The posture decision, and the test that produces it

**Rejected test — rank by danger.** Both gestures are dangerous in
exactly the same way and to the same degree: a `W`/`W*` clip governs what
*other* content is visible, so either edit changes the page somewhere
other than where the pointer is. Danger gives **no separation** between
the two cases, so it cannot be the discriminator.

**Adopted test — does a legitimate operator intent exist?**

| Gesture | Is there an intent pdfce can honour? | Posture |
|---|---|---|
| **Move / resize** a clip rectangle | **Yes.** Resizing a crop region is a real drafting task with an unambiguous meaning: the visible window changes to the new rectangle. | **Disclose and proceed** |
| **Delete part of** a clip | **No.** There is no operation the operator could have meant that pdfce could then perform correctly — a clip with one edge removed is not a smaller clip, it is an ill-defined region. | **Refuse** (`VectorEditError::ClippingPath`, Pass 25.2, unchanged) |

**Why refusing the movable case would be the wrong kind of safe.** Clip
geometry would then be **permanently uneditable** — a refusal with no
path to "yes" is a capability hole wearing safety's clothes. pdfce aims
at Acrobat-Pro parity; "you can never adjust a crop region" is not a
defensible boundary. Refuse the case with no good reading; disclose the
case that has one.

## 2. How this sits with CLAUDE.md rule 4 — the tension is real and is NOT resolved by fiat

The narrowed rule 4 (decision 024 §4.4) removes the confirm step from
**direct manipulations the operator performed** whose result is *fully
visible on the canvas* and reversible in one undo. A clip drag is a
direct manipulation, so the narrowing applies **on its face**.

But this is the case where the narrowing's own premise is **weakest**:
the *visible* result (the rectangle moved) and the *material* result
(content elsewhere appeared or vanished) are in **different places on the
screen**. The narrowing assumed those coincide.

This is not papered over. It is filed as **open operator question (av)**
in `ROADMAP.md`:

- *Shipped default:* disclose and proceed.
- *Named alternative if the operator disagrees:* promote to a
  **fixed-anchor** confirm (R121) for clip geometry specifically.
- *Explicitly NOT among the alternatives:* a refusal — that reinstates
  the permanent-uneditability problem §1 rejected.

## 3. The mechanism — `PlannedEdit::disclosures` and five changed signatures

**The problem.** `Result<(), E>` encodes success-or-failure and nothing
else. With no third slot, every caller drops advisory output **by default
rather than by decision**, and the drop is **invisible at the call site**
— no `let _`, no unused-variable warning, nothing for review to catch.

**The change.**

- `vector::PlannedEdit` gained **`disclosures: Vec<String>`**, so the
  information originates where it is discovered rather than being
  reconstructed by a caller.
- Five `EditSession` verbs changed from `Result<(), EditError>` to
  **`Result<Vec<String>, EditError>`**: `move_object`, `delete_object`,
  `move_subpath`, `delete_subpath`, `move_node`.
- **The whole family changed at once**, deliberately. A family where two
  of five verbs return disclosures teaches callers that an empty result
  means "nothing to disclose" — false for the other three.

**Routing, both halves load-bearing.**

- **CLI → stderr.** stdout stays the machine-parseable record, **pinned
  by a test**. A disclosure on stdout breaks every script that parses the
  command's output, and it would be found by a user's pipeline, not by
  the suite.
- **GUI → `pending_note`.** Same source of truth, different surface.

**The tell that this was an API-shape problem and not a discipline
problem:** the GUI layer had **already** hit this gap and patched it
ad-hoc with `pending_note`. When the same "we know something and have
nowhere to put it" appears in two layers, the missing thing is a return
channel, not another notification widget.

## 4. Consequences on the error surface

`VectorEditError::RectangleNode` and `VectorEditError::ImplicitNode` are
**removed** — no remaining producers. Pass 30.0 materializes the operands
whose absence they reported:

- `re x y w h` → `x y m` / `x+w y l` / `x+w y+h l` / `x y+h l` / `h`
  (the equivalence **ISO 32000-1 §8.5.2.1 Table 59 states itself**). The
  **trailing `h` is load-bearing**: `re` appends a *closed* subpath, and
  four segments without it leave the subpath *open* — invisible on a
  fill, wrong on a stroke (two line caps where a corner join belongs).
- An inherited subpath start (a subpath reopened after `h`, §8.5.2.1)
  → an explicit `m`, inserted **before** the segment that inherited it.

Their tests were **rewritten, not deleted**. The rectangle one became an
**undo** test and is the stronger case: undo must restore a stream
**shorter** than the one it undoes (five operators back to one), a length
change a same-length rewrite never exercises.

## 5. Standing rules originating here

- **R143** — a refusal's stated reason is re-verified before it is used
  to scope work. (From Pass 29.0's self-justifying R-INV-4, filed in the
  same session; recorded here because R143–R145 are one family.)
- **R144** — removing a refusal can remove an unrelated **protection**
  the refusal was incidentally providing. **This decision's own origin
  story.** Corollary: a fixture cannot surface this class, so a
  newly-lifted gesture is run against a **real file** before shipping.
- **R145** — a planner that can produce operator-visible information
  **returns** it; `Result<(), E>` is a shape that drops it by default.
  (§3 above.)

## 6. Acceptance criteria — all met at filing (the code shipped first)

- **C1** — moving an object/subpath/node that carries a `W`/`W*` clip
  succeeds and emits a disclosure. ✅
- **C2** — deleting part of a `W`/`W*` clip still refuses by name
  (`VectorEditError::ClippingPath`). ✅ (unchanged from Pass 25.2)
- **C3** — CLI prints disclosures to **stderr**; stdout remains
  machine-parseable, **asserted by a test**. ✅
- **C4** — GUI routes disclosures through `pending_note`. ✅
- **C5** — `re` expansion emits a **closed** subpath; a dedicated test
  fails if the trailing `h` is dropped (verified by planting the bug). ✅
- **C6** — `undo_identical=1` on a synthetic fixture **and** on a real
  linearized PDF, including the shorter-stream undo case. ✅
- **C7** — `RectangleNode` / `ImplicitNode` have no producers and are
  removed; their tests are rewritten, not deleted. ✅

**Owed, not met by this decision:** `ARCHITECTURE.md` §4's API contract
does not yet describe `PlannedEdit::disclosures`, the five changed
signatures, or the two removed variants. §4 is now three filings behind
on shipped core surface (Pass 25.x's vector surface and decision 026's
ce-dimension model are the other two). Named again here so the §3/§4 sync
is scheduled as work rather than assumed.

## 7. References

- ISO 32000-1 §8.5.2.1 Table 59 — path construction operators; `re`'s
  stated equivalence; the implicit control points of `v`/`y`.
- ISO 32000-1 §8.5.4 — clipping-path operators `W` / `W*`; the `re W n`
  idiom.
- `ROADMAP.md` — Pass 30.0 (Shipped), Pass 25.2 (the refusal kept),
  standing rules R143–R145, open operator question (av).
- `ARCHITECTURE.md` §12 — the dated index entry for this decision.
- `C:\personal_rag\pdf\lesson_20260805_clip_paths_are_re_rectangles_and_lifting_a_refusal_removed_the_guard.md`
  — the empirical half (why clip geometry is met on the first try).
- `D:\dev\rag\rust\result_unit_ok_drops_operator_visible_information_by_default.md`
  — the ecosystem-general half of §3.
- Decision 024 §4.4 — the rule-4 narrowing this decision tests against.

---

## Amendment, 2026-08-05, same day — Pass 30.1 is the first consumer of §3's channel beyond clipping paths, and it surfaced a cost of §3's change

Appended, not edited into the body above (this directory is append-only).

**1. The disclosure channel generalized immediately, which is the
evidence §3 was an API-shape fix and not a one-off.** Pass 30.1
(`d025c1a`, Bézier handles) drags a `v` or `y` handle by **re-spelling
the segment as the `c` that states both control points** — §8.5.2.1
Table 59's implicit control points cannot both stay implicit and move.
That is §4's materialize-rather-than-refuse move one level down, from
anchors to control points, and its disclosure rides
`PlannedEdit::disclosures` unchanged. **Two independent features, one
channel, no second mechanism** — which is exactly what did *not* happen
before the change, when the GUI grew an ad-hoc `pending_note` because
there was nowhere else to put the same class of information.

**2. §3's signature change had a cost, recorded so it is not read later
as unrelated churn.** Inserting the `# Returns` doc block on the five
`EditSession` vector methods put it **above** each method's summary line,
which makes *"# Returns"* the one-line summary rustdoc renders for all
five. Fixed in `d025c1a` by moving the block to the end. Small, but it is
a **direct consequence of this decision** and belongs on this record: a
mechanical signature change across a verb family touches five doc
comments at once, and the failure was uniform across all five precisely
because the edit was uniform.

**3. Pass 30.1 also refuses on the same test as §1.** A straight segment
has no handle, and `VectorEditError::NoHandleHere` refuses rather than
converting the line to a curve — *"turning a line into a curve is a
different operation with a different name."* Same discriminator as §1's:
there is a legitimate intent for reshaping a curve (disclose), and none
for silently promoting a line the operator never drew as a curve
(refuse). The decision generalizes as intended.
