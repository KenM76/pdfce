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

## §0 ONE THING IS IN FLIGHT: `Pass 185.1`. The button-action arc is closed.

`Pass 182.0` → `183.0` → `183.1` → `184.0` ran end to end today, and the last
of them repaired a defect the first three created. Concretely, a push button
can now be given a **reset**, a **submit**, an in-document **jump**, one of the
four **named navigations**, a **URI**, or a **show/hide** — and renaming a
field repoints every button that named it while deleting one reports what it
orphaned.

**Two things that arc left standing, deliberately, and neither is a gap:**

- **A toggling show/hide button is OUT OF SCOPE, not unbuilt.** Table 210
  assigns (`/H` is a value, not a verb) and the standard owns a toggle
  elsewhere (`/SetOCGState`'s `/State`) and did not use it here. A real toggle
  needs ECMAScript.
- **Four of the eight script-free, reach-nothing action types are unauthored**
  — `SetOCGState`, `Trans`, `GoTo3DView`, `GoToDp`. Each is **sized** in
  `ButtonAction`'s own doc block. None has been asked for.

**And one that is:** `/AP` `/D` (the pressed appearance) and `/MK` icon/label
layout, the remainder of `Pass 131.0`. ★ It is **appearance work** (`R43`'s
neighbourhood), not a continuation of the action work — a session scoping it as
"the rest of the button actions" would scope the wrong thing.

**★ `Pass 185.1` is IN FLIGHT and may be uncommitted when you read this.** Run
`git status` first. It is the form-edit fuzz target plus the page-tree guard it
found — see §A item 2 — and it carries **one unresolved item**, §C item 6.

---

## §A — Candidates, ordered by my read of value. None is a commitment.

1. **★ CHECK BOTH FeatureRequests CHANNELS FIRST.** They live outside the repo
   so no gate can contradict a stale "it's empty":
   - `D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\`
   - `D:\Dev\FeatureRequests\iccce_FeatureRequests\open\`

   Two inbound requests were **answered by shipped work** today and still sit
   in `open/`. One outbound reply is there with **two addenda** — it tells
   `pdfceGUI` that the submit they asked us to *refuse* shipped on an operator
   override, names the two Table 210 traps a UI will hit, and warns that
   `rename_field`/`delete_field` **changed behaviour** on verbs they already
   call. Nothing is owed back.

2. **`Pass 185.1` — the form-edit fuzz target and what it found in two
   minutes.** `fuzz/fuzz_targets/form_edit_sequence.rs` drives
   `set_button_action`, `rename_field`, `delete_field` and
   `delete_field_group` over mutated documents, and immediately hit a
   **release-silent page-tree corruption**: an `/AcroForm` whose `/Fields`
   names an object that is also a `/Page`. `parse_acroform` models it as a
   field (correctly — the form dictionary says it is one), the deletion
   removed the page, and the only thing that complained was a
   `#[cfg(debug_assertions)]` postcondition that is **compiled out of the
   build operators run**.
   ★ Guarded on all three deletion routes and reproduced in a hand-built
   fixture (`crates/pdfce-core/tests/form_delete_page_tree.rs`). **A second,
   different page-tree crash was seen once and has not been reproduced** —
   see §C item 6 before assuming the class is closed.

   ★★ **`Pass 185.0` shipped this morning and this target is the reason to
   care about it**: the fuzz gate is the one this role skips, twice recorded
   and twice recurred, and the first target written after saying so found a
   real defect in an existing verb within two minutes.

3. **`Pass 142.0`** — a font face outside the standard 14. The largest
   remaining *named* feature, **de-prioritised by the consuming project's own
   use report**, not by us: *"Synthetic is enough. Drop `142.0` down the
   queue."* Not closed, not declined.
   ★ `bind_font_resource` (`text_edit/addtext.rs`) is the single
   implementation of "add a `/Font` entry"; `142.0` extends it and does not
   write a second one. And there are **THREE save paths** —
   `EditSession::format_text`, its form twin, and the one-shot
   `text_edit::set_format`, **which is the one the CLI uses**. `Pass 162.0`
   wired two, every unit test passed, and the binary printed a disclosure
   about a resource it had not written.

4. **The n-channel (per-spot-colorant) buffer** — the only path to the print
   suite's remaining overprint/spot FAILs. **Operator's call; do not scope it
   without him.**

5. **`CmykIntent::Calibrated`'s cool greys** — three independent lines of
   evidence, and **still not ours to fix** (decision 064 puts the conversion in
   `iccce`'s domain; the operator ruled the default 2026-08-28).
   ★ **The black end of that same table is a FALSE-DEFECT TRAP** — pdfce is the
   *closer* answer there and `iccce` said so unprompted. Read
   `settings/mod.rs`'s doc comment before touching anything CMYK.

---

## §B — What is deliberately NOT being worked, and why

- **`census_dangling` will never see a field-name target.** That is a boundary,
  not a bug: a name is not a reference, and the census answers a question about
  the object graph. The companion numbers live on `delete_field` and
  `rename_field`. Both halves are now documented at both ends — do not
  "fix" the census by teaching it names; it would then need the object sweep,
  and that already exists.
- **C9 — `/StructParent` / `/OBJR` orphaned by an annotation delete.** Owed
  since `Pass 38.5` and scoped out again: different graph, different carrier,
  no name-string component, so it shares none of the sweep's machinery.
- **ce-dimension tolerance, the ISO 286 fit classes** — needs a sourced
  class/table lookup this project does not have.

---

## §C — ★★ READ BEFORE WRITING CODE. Six items from this session.

1. **★★★ A CORRECTION THAT REACHES THE STRUCTURE AND STOPS SHORT OF THE
   PROSE.** Four instances today, every one found by a reader rather than a
   gate: a plan doc's banner updated while its opening sentence still said the
   opposite; *"Five variants:"* left on the line between an updated heading and
   a six-row table; a rename disclosure saying it broke *"submit mappings"*
   hours after it started repairing them; and the census's own blindness
   documented in every file the fix touched and **in none of the files the
   checker lives in**.
   ⇒ **After amending a document, grep for the CLAIM you just falsified, not
   for the section you just edited.** And when a sweep comes back clean, ask
   what its keyword set could not have matched — two of today's sweeps were
   disjoint and each would have missed the other's survivors.

2. **A plan's enumerated list is a snapshot; the operator's sentence is the
   ruling.** `Pass 183.0` shipped from a four-day-old plan's bullet list and
   omitted `/Hide`, which his words plainly covered. Cost a whole second Pass.

3. **★ The difficulty of the obvious implementation is not the difficulty of
   the problem.** I wrote this file's own previous §0 saying `Pass 184.0`
   needed a visitor refactor of `scan_javascript` — the crate's most
   defect-prone function. It did not. The two walks answer different questions,
   and an **object sweep** is a strict superset of the carrier walk. The
   refactor was correctly assessed as risky and was never the work.

4. **A surviving sabotage has three causes and only one is a weak test:** an
   assertion that cannot see the change (`saved()` returns the base bytes
   **plus** the update — this fired twice in one file), a guarantee enforced
   elsewhere (the call site discards the writes, so no argument can defeat it),
   and a mutation that is semantically a no-op. Ask in that order.

5. **★★ A SECOND PAGE-TREE CRASH, SEEN ONCE, NOT REPRODUCED — do not assume
   `Pass 185.1` closed the class.** After the three deletion routes were
   guarded, `cargo +nightly fuzz run form_edit_sequence` hit the same
   `debug_assert_page_tree_still_walks` postcondition again, at a different
   line, and I could not get it back: two subsequent runs (50k and 90k
   iterations) were clean, and **libFuzzer wrote no artifact**, because Rust's
   abort on Windows exits `0xc0000409` before its crash handler saves one.

   Two things to carry:
   - **Run it with `-seed=N` fixed** so a crash is replayable, and consider
     making the target print the input itself on panic — the harness cannot be
     relied on to.
   - **The absence of a reproduction is not evidence of a fix.** The first
     crash was reproduced deterministically in a hand-built fixture *before*
     being fixed; this one was not, so it is open. The ASan DLL path and the
     invocation are in `.claude/agent-memory/pdfce-engineer/reference_fuzz_asan_dll.md`.

6. **Patch-script hazards, both of which destroy work silently.**
   `pathlib.write_text()` rewrites a whole file to CRLF unless you pass
   `newline`; and a sabotage loop that asserts on a *later* case leaves an
   *earlier* case's sabotage on disk if the restore is after the loop rather
   than in a `finally`. Validate every anchor before touching the file.
   ★ And a new string-gap mechanism: a Rust line continuation can survive
   transit and be **flattened by `cargo fmt`**, which rejoins the literal and
   leaves the eaten indentation as a run of spaces.

---

## §D — State of the tree

Run these rather than trusting the sentence:

```
python tools/check-ledger-numbers.py      # all four ceilings
bash tools/run-gates.sh                   # the full sweep; it derives its
                                          # own list, so do not memorise a count
git rev-list --count origin/main..main    # how far ahead
ls -lt D:/Dev/pdfce-backups/              # newest bundle
gh run list --limit 3                     # CI's colour, from GitHub
```

- **Pushing is standing-authorized** (rule 8, decision 090 — *"always push"*);
  cutting a tag or release is **not**, and neither is a force push or a
  non-`main` branch. Scrub `check-suite-name-absent.py` green **before**
  pushing regardless — the repository is public, so a push publishes.
- **★ `R217` does NOT constrain pushing, and I claimed it did.** I asserted
  that three commits went out ahead of their filings and turned CI red each
  time; the librarian measured GitHub and **one of the three was green** —
  pushed alone, as the tip, before its filing existed, which is exactly the act
  I called forbidden. `R217` constrains what may land **on top of** an unfiled
  commit. Read its third amendment note before repeating my mistake.
- **Read CI's colour from GitHub**, not from a sentence in a document, and see
  §A item 2 before trusting a failing job's *name*.
- The backup bundle drifts about one commit per Pass. A fresh one is cheap:
  `git bundle create <path> --all`.
- `.tmp_bench.py` has been untracked for four filings. It is deliberately not
  committed — the repository is public — so **stage by path, never `git add
  -A`**.
