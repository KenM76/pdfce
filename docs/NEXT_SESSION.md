# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. **Overwrite it once acted on.**

Per standing rule `R216` this file carries **no edit-history layer**. What is
true now, plus a pointer. Corrections and their prior wording live in the
**append-only** record — `ROADMAP.md` and `SESSION_LOG.md`.

Written 2026-08-30. **For the ledger — Pass ceiling, standing-rule ceiling,
decision ceiling, filing count — run `python tools/check-ledger-numbers.py`.**
It derives all four and is the only thing that cannot be stale.

---

## §0 ★★★ THE OWED LIST FROM THE LAST HANDOFF IS EMPTY

Every item the previous `§A` carried is discharged or is the operator's call.
This is the first handoff in weeks with nothing carried forward.

| was | now |
|---|---|
| **1.** five survivors of `Pass 174.9`'s withdrawn claim | discharged by the 334th filing |
| **2.** the **ce-dimension text override**, decision 097 — the only item Ken had personally specified | **SHIPPED**, `Pass 175.0` |
| **3.** `Pass 142.0`, a font face outside the standard 14 | **still open, still de-prioritised** — see §B |
| **4.** `rotate_widget` | **SHIPPED**, `Pass 177.0` |
| **5.** the n-channel buffer | **operator's call, unchanged** — see §B |

**Both phantom verbs are discharged.** `tools/check-cited-verbs-exist.py` now
reports 7/7 resolving with **none marked unbuilt** — the first time that gate
has been clean in that sense since it was written.

---

## §A — THERE IS NO PICK-FROM LIST. Pick work, then read §C first.

Nothing is owed. What follows are candidates, ordered by my read of value, not
by obligation. **None of them is a commitment made to anyone.**

1. **★ CHECK BOTH FeatureRequests CHANNELS FIRST.** They live outside the repo
   so no gate can contradict a stale "it's empty":
   - `D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\`
   - `D:\Dev\FeatureRequests\iccce_FeatureRequests\open\`

   **Two outbound notes were left there on 2026-08-30 and one carries a
   question I want answered**, in
   `note_widgets_rotate_now_and_three_verbs_you_could_not_reach.md` §"What
   we'd find useful back": *does Acrobat actually honour `/MK /R` when it
   disagrees with a baked `/AP`?* The standard says a conforming PDF 2.0
   reader ignores it (erratum #56). **What shipping readers do is unmeasured
   and this project asserts nothing either way.** Acrobat Reader is installed
   here; that measurement is takeable without waiting for a reply.

2. **★★ A DEFECT CLASS THIS SESSION FOUND TWICE AND DID NOT SWEEP FOR.**
   `Pass 176.0` fixed a rule enforced in a pure-model helper and **not** in the
   session verb that ships through it (`delete_dimension_group_with` vs
   `DimensionModel::delete_group`). It was latent for months because nothing
   called the verb with the argument that reached it.

   The librarian declined to mint a rule at `n=1` and argued the decline well.
   **But nobody has looked for a second instance** — the decline was a judgement
   about the two candidates offered, not a search. `DimensionModel` and
   `forms::` have several `pub(crate)` helpers with guards; the session verbs
   that wrap them are the shipping surface. A sweep is cheap and would either
   mint a rule on evidence or close the question.

3. **`Pass 142.0`** — a font face outside the standard 14. The largest
   remaining named item, **de-prioritised by the consuming project's own use
   report, not by us**: *"Synthetic is enough. Drop `142.0` down the queue."*
   Their reasoning is a use report and worth re-reading before re-weighing it
   (CAD exports, 8 pt notes, *"a fabricator reading a print, not a
   typographer"*). It is **not closed and not declined** — a different consumer
   setting body text would weigh it differently.
   ★ `bind_font_resource` (`text_edit/addtext.rs`) is the single implementation
   of "add a `/Font` entry"; `142.0` extends it and does not write a second one.
   And **there are THREE save paths**: `EditSession::format_text`, its form
   twin, and the one-shot `text_edit::set_format` — **which is the one the CLI
   uses.** `Pass 162.0` wired two, every unit test passed, and the binary
   printed a disclosure about a resource it had not written.

4. **The n-channel (per-spot-colorant) buffer** — the only path to the print
   suite's remaining overprint/spot FAILs. **Operator's call; do not scope it
   without him.** It is a large architectural change to the compositor.

5. **`CmykIntent::Calibrated`'s cool greys** — now carrying **three**
   independent lines of evidence after `Pass 175.1` (the reference's own
   neutrality, pdfce's measured spread, and `iccce`'s real ICC transform
   corroborated against `lcms2` to 0.22 counts). **Still not ours to fix** —
   decision 064 puts the conversion in `iccce`'s domain and the operator ruled
   the default on 2026-08-28. What changed is the strength of the evidence, and
   `settings/mod.rs`'s doc comment now records it.
   ★ **And the black end is a FALSE-DEFECT TRAP in the same table**: pdfce is
   the *closer* answer there and `iccce` said so unprompted. Read that doc
   comment before touching anything CMYK.

---

## §B — What is deliberately NOT being worked, and why

- **`Pass 142.0`** — §A item 3. De-prioritised by the consumer, not declined.
- **The n-channel buffer** — §A item 4. Operator's call.
- **ce-dimension tolerance, the ISO 286 fit classes** — needs a sourced
  class/table lookup this project does not have. Every other tolerance form
  ships.
- **A manual dimension tool** (decision 097 branch 2) — **not built and not
  needed.** Branch 1 shipped and is reversible, which was the condition. If a
  consumer wants an object with a typed value and no measurement underneath, it
  becomes a real request rather than a contingency.

---

## §C — ★★ READ THIS BEFORE WRITING CODE. Four traps this session walked into.

Each cost real time on 2026-08-30 and each is cheap to avoid.

1. **★★★ Anchor a splice on the FIRST LINE OF THE DOC BLOCK, never on the
   `fn` / `pub struct` / variant line.** Anchoring on the item lands the new
   code **between that item's `///` block and its `#[derive]`s and the item
   itself**. This happened **three times in one session**, in three shapes: a
   clap variant (would have shipped wrong `--help`), a CLI handler `fn` (caught
   by clippy's `doc_lazy_continuation` and `check-cli-help-leads.py`), and a
   `pub struct` with derives (caught by `E0119`). Two of three were caught by
   gates; do not rely on that.

2. **A byte scan over an incrementally-saved PDF sees EVERY revision.** It can
   prove *presence* and can never prove *absence*, and `.last()` on such a scan
   reads as "the current value" and is not — an earlier revision's `/Matrix`
   stays in the bytes forever. Assert absence by **rendering**, or by reopening
   and resolving.

3. **Measure the crop before reporting a render defect.** A ce dimension
   rendered with the dimension line apparently running through its caption
   turned out to be the *fixture's own page content* at the same coordinates —
   the object being measured. A freshly authored dimension breaks correctly. A
   false renderer-defect report was one step away, twice in one day.

4. **`git add -A` is unsafe here.** Subagents write their own memory and doc
   files while you work; three of them were live in this session. Stage your
   own paths by name.

---

## §D — State of the tree

Run these rather than trusting the sentence:

```
python tools/check-ledger-numbers.py      # all four ceilings
bash tools/run-gates.sh                   # the full sweep, 29 commands
git rev-list --count origin/main..main    # how far ahead
ls -lt D:/Dev/pdfce-backups/              # newest bundle
```

- **`main` was ahead of `origin/main` when this was written.** Pushing is
  **standing-authorized** (rule 8, decision 090 — *"always push"*); cutting a
  tag or release is **not**, and neither is a force push or a non-`main`
  branch. Scrub `check-suite-name-absent.py` green **before** pushing
  regardless — the repository is public, so a push publishes.
- **The filing gates defer only the TIP commit.** A code commit that is not the
  tip and not filed turns them red, which is normal between a Pass and its
  filing and is what `R217` (hold the push until the filing commits) exists for.
- The backup bundle drifts about one commit per Pass. A fresh one is cheap.
