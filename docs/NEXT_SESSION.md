# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. **Overwrite it once acted on.**

Per standing rule `R216` this file carries **no edit-history layer**. What is
true now, plus a pointer. Corrections and their prior wording live in the
**append-only** record — `ROADMAP.md` and `SESSION_LOG.md`.

Written 2026-08-30, end of the button-actions session. **For the ledger — Pass
ceiling, standing-rule ceiling, decision ceiling, filing count — run
`python tools/check-ledger-numbers.py`.** It derives all four and is the only
thing that cannot be stale.

---

## §0 ★★ ONE THING IS OWED, AND IT IS A REFACTOR, NOT A CORRECTION

**`Pass 184.0` is PARTIALLY DELIVERED.** Criterion A shipped (`92b9f99`); the
rest did not, and the rest is the substance.

| criterion | state |
|---|---|
| **A — a categorical disclosure on rename** | **SHIPPED.** `FieldRename::actions_not_retargeted`, an upper bound, labelled categorical in three places. |
| **B — generalise the action-carrier walk to a visitor** | **OWED.** |
| **C — `rename_field` REPAIRS the name strings and reports the real count** | **OWED**, needs B. |
| **D — `delete_field` DISCLOSES without repairing** | **OWED**, needs B. Discharges the `Pass 38.5` **C8** debt. |

**The defect, stated once so it does not have to be re-derived.** Passes
182.0/183.0/183.1 write `/ResetForm` and `/SubmitForm` targets in `/Fields`,
and `/Hide` targets in `/T`, as **fully-qualified name strings** — deliberately,
because a name survives a field being renumbered or copied between documents
where an indirect reference does not. **A rename is the one operation that
breaks that choice**, and pdfce repairs nothing. A button reading "Reset"
quietly stops resetting the field it was drawn for.

★ **And `census_dangling` is STRUCTURALLY blind to it.** A name string leaves
**no dangling object reference**, so the graph census that `Pass 183.0`
correctly widened from links to every annotation subtype cannot see this at
all. Two different invisibilities in one feature; that Pass fixed the one a
graph walk can see. `personal_rag/pdf` carries the lesson.

**Why B is a refactor and not a line.** Finding the affected buttons means
walking **all seventeen action carrier sites with `/Next` chains** — the walk
`forms::scan_javascript` owns, which threads `&mut FormJavaScript` throughout.
Generalising it to a visitor is a real change to the crate's most
defect-prone function (`Pass 133.0`'s history is attached to it), and writing
a **second, narrower walker** instead is the exact defect class this project
keeps recording. Keep `scan_javascript`'s behaviour byte-identical and
sabotage-check that it is.

**Why repair is defensible on rename and nowhere else:** pdfce knows both the
old and the new name exactly, so rewriting the string is not a guess. A
*deleted* target has no answer to "what did the author mean", which is why D
discloses and does not repair — and why C9 (`/StructParent` / `/OBJR`) is
scoped out: different graph, different carrier, no name-string component.

---

## §A — Candidates, ordered by my read of value. None is a commitment.

1. **★ CHECK BOTH FeatureRequests CHANNELS FIRST.** They live outside the repo
   so no gate can contradict a stale "it's empty":
   - `D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\`
   - `D:\Dev\FeatureRequests\iccce_FeatureRequests\open\`

   Two inbound requests were **answered by shipped work** today and are still
   sitting in `open/`: the push-button one and the double-decomposition one.
   One outbound reply was left, with an addendum — it tells `pdfceGUI` that
   the submit they asked us to **refuse** shipped anyway on an operator
   override, and names the two Table 210 traps a UI will hit. Nothing is owed
   back.

2. **`Pass 184.0` B/C/D** — §0. The largest owed item and the only one with a
   named defect behind it.

3. **`Pass 142.0`** — a font face outside the standard 14. Still the largest
   remaining *named* feature, still **de-prioritised by the consuming
   project's own use report**, not by us: *"Synthetic is enough. Drop `142.0`
   down the queue."* Not closed, not declined.
   ★ `bind_font_resource` (`text_edit/addtext.rs`) is the single implementation
   of "add a `/Font` entry"; `142.0` extends it and does not write a second
   one. And there are **THREE save paths** — `EditSession::format_text`, its
   form twin, and the one-shot `text_edit::set_format`, **which is the one the
   CLI uses**. `Pass 162.0` wired two, every unit test passed, and the binary
   printed a disclosure about a resource it had not written.

4. **A CI job named for one of its steps.** The job **"verify pdfce-gui
   strings live in ui_text.rs"** also runs `check-commits-filed.py`, so when
   the filing gate fails, CI reports a red job named after a string check that
   is perfectly clean. It cost a diagnostic cycle today and it impugns the
   local gate runner, which is the expensive part. `.github/workflows/` was
   not touched; raised with the librarian.

5. **The n-channel (per-spot-colorant) buffer** — the only path to the print
   suite's remaining overprint/spot FAILs. **Operator's call; do not scope it
   without him.**

6. **`CmykIntent::Calibrated`'s cool greys** — three independent lines of
   evidence now, and **still not ours to fix** (decision 064 puts the
   conversion in `iccce`'s domain; the operator ruled the default 2026-08-28).
   ★ **The black end of that same table is a FALSE-DEFECT TRAP** — pdfce is the
   *closer* answer there and `iccce` said so unprompted. Read
   `settings/mod.rs`'s doc comment before touching anything CMYK.

---

## §B — What is deliberately NOT being worked, and why

- **A toggling show/hide button.** Not unbuilt — **out of scope**. Table 210
  works *"by setting or clearing"* with `/H` as a value, and the standard owns
  a toggle (`/SetOCGState`'s `/State`) and did not use it here, so a genuine
  toggle requires ECMAScript.
- **Four of the eight script-free, reach-nothing action types** —
  `SetOCGState`, `Trans`, `GoTo3DView`, `GoToDp`. Each is **sized** in
  `ButtonAction`'s doc block; none has been asked for.
- **`/AP` `/D` (the pressed appearance) and `/MK` icon/label layout** — the
  remainder of `Pass 131.0`. ★ It is **appearance work** (`R43`'s
  neighbourhood), not a continuation of the action work; a session scoping it
  as "the rest of the button actions" would scope the wrong thing.
- **ce-dimension tolerance, the ISO 286 fit classes** — needs a sourced
  class/table lookup this project does not have.

---

## §C — ★★ READ BEFORE WRITING CODE. Traps this session walked into.

1. **★★★ A CORRECTION THAT REACHES THE STRUCTURE AND STOPS SHORT OF THE
   PROSE.** Twice today, hours apart, both found by the librarian and neither
   by a gate:
   - the plan doc got a "PHASE 1 IS SHIPPED" banner at its head while §0's
     opening sentence still said pdfce authors no action;
   - `docs/core-api` got a six-row table and an updated heading with
     *"Five variants:"* left **on the line between them**.

   **Attention follows structure.** After amending a document, grep it for the
   claim you just falsified — not for the section you just edited.

2. **A plan's enumerated list is a snapshot; the operator's sentence is the
   ruling.** `Pass 183.0` shipped from a four-day-old plan's Phase-1 bullet
   list and omitted `/Hide`, which his actual words plainly covered. Cost a
   whole second Pass. Where the standard defines a **closed set**, enumerate
   it and say which members are in and out — that is what surfaces the
   omission *before* shipping.

3. **`pathlib.Path.write_text()` rewrites the whole file to CRLF on Windows.**
   Default `newline=None` translates every newline to `os.linesep`. A
   sabotage-and-restore script flipped all 40,699 lines of `edit.rs`, and its
   own identity assertion **passed**, because `read_text` compares after
   translation. Always pass `newline` explicitly, and check the byte count of
   CRLFs before committing.

4. **A sabotage LOOP leaves the previous case applied when a later anchor
   fails.** The restore was after the loop; an assertion on case 3 raised, and
   case 2's sabotage sat on disk with the tests still passing for everything
   else. **Restore in a `finally`, and validate every anchor before touching
   the file.**

5. **Three counts of the same enum are three different facts.** `ButtonAction`
   has **six** variants; pdfce authors **four** of the **eight** script-free,
   reach-nothing action types the standard defines. A sweep that "reconciles"
   those numbers breaks two correct sentences. They are filed side by side in
   the Pass entry and in decision 107 for exactly that reason.

---

## §D — State of the tree

Run these rather than trusting the sentence:

```
python tools/check-ledger-numbers.py      # all four ceilings
bash tools/run-gates.sh                   # the full sweep, 29 commands
git rev-list --count origin/main..main    # how far ahead
ls -lt D:/Dev/pdfce-backups/              # newest bundle
gh run list --limit 3                     # CI's colour, from GitHub
```

- **Pushing is standing-authorized** (rule 8, decision 090 — *"always push"*);
  cutting a tag or release is **not**, and neither is a force push or a
  non-`main` branch. Scrub `check-suite-name-absent.py` green **before**
  pushing regardless — the repository is public, so a push publishes.
- **★ "Always push" and `R217` (hold the push until the filing commits) pull
  against each other, and it is not theoretical.** Three code commits went out
  ahead of their filings today and **CI was red on each until the filing
  landed** — a known, expected, self-healing red. Raised with the librarian;
  read `ROADMAP.md` for whatever it decided before repeating the pattern.
- **Read CI's colour from GitHub**, not from a sentence in a document — and
  see §A item 4 before trusting a failing job's *name*.
- The backup bundle drifts about one commit per Pass. A fresh one is cheap:
  `git bundle create <path> --all`.
