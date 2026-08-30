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

## §0 ONE THING IS OWED: the third fuzz finding. The button-action arc is closed.

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

**★ THE OWED ITEM: finding #3 from the form-edit fuzz target**, a
`debug_assert_eq!` in `delete_field_group` where the emptied-node cascade and
its prediction disagree. **Sized honestly: it is `debug_assert`, so in release
it is a wrong `nodes_removed` in a disclosure, not corruption** — materially
less severe than the two page-tree defects already fixed. §C item 5 has the
location, the replay seed, the harness traps and a stated guess to start from.

`Pass 185.1` (the target + the first guard) and `Pass 185.2` (the catalog,
which the first guard did not cover) are both shipped and pushed.

**★★ AND `R236` ARRIVED WITH THIS PASS, SO ITS DENOMINATOR IS MEASURED HERE
RATHER THAN LEFT AS "SEVERAL".** The rule: *a `debug_assert` postcondition over
state derived from untrusted input is a tripwire for a fuzzer, not a guard for
an operator*, so it owes a `cargo-fuzz` target over the verbs it guards or a
written exemption at the site.

The filing's ledger counted **named helpers** (2 of 2). The unit was then
widened to include bare inline assertions — finding #3 is one — and **the wider
denominator was never taken**.

**It is 24** — 12 in `pdfce-core`, 12 in `pdfce-render` — and getting there
took three wrong answers, which is worth more than the number:

| published | actual | why |
|---|---|---|
| 34 (349th filing) | — | never right at any commit |
| 27 (`7ac98da`, mine) | — | counted grep **hits**, three of which are **comment lines** |
| **24** | ✓ | independently derived twice, agreeing |

★★ **A source grep over a documentation-first codebase counts the codebase's
own prose about the construct.** The three lines that inflated my count are
`edit.rs:12085`, `12092` and `xref_out.rs:282` — the careful reasoning about
*why* the guard is a `debug_assert!` and not a `panic!`. **The best-written
lines in the region are the ones that corrupted the census.**

And the shape is `7ac98da`'s own headline turned back on it: *sourcing a figure
makes it auditable, not durable* — that commit named its exact command and
still published the wrong **noun** (hits, not invocations).

⇒ **Publish the decomposition, not the total**, so a reader checks by addition:
`grep -c debug_assert` = 56 mentions = **24 invocations** + 6
`cfg(debug_assertions)` + 2 `fn` definitions + 24 lines of prose about them.

The sites this rule actually reaches:

| site | verdict |
|---|---|
| `edit.rs:12095` page-tree postcondition | **covered** — `form_edit_sequence`, `pageops_sequence` |
| `edit.rs:17486` group cascade vs prediction | **open finding #3** — the target reaches it |
| **`edit.rs:21669`** *"the target was located on some page, so at least that page must be patched"* | ★ **UNCOVERED. No fuzz target drives annotation DELETION** — `annot_walk` reads, `annot_author` writes. This is the concrete work `R236` creates. |
| `ccitt.rs:236`, `lexer.rs:400`, `parser.rs:699` | covered by `image_codec_ccitt` / `parse_object` / `load_document` |
| `cmyk_buffer.rs` ×10, `mesh.rs` ×2 | render side; `mesh_shading` covers the mesh pair — **the cmyk_buffer ten are unaudited and are the second thing to look at** |
| `text_edit/edit.rs:2998`, `addtext.rs:682`, `xref_out.rs:292`, `writer/content.rs` | **exempt**: caller-convention or pdfce-constructed state, not untrusted-derived — the same reasoning `writer/content.rs:649` already wrote for itself |

⇒ **One named uncovered site, one unaudited group of ten.** That is a finite
work item rather than a rule with an open-ended obligation, which is the shape
it needs to survive contact with a busy session.

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
   ★ Guarded on all three deletion routes, each on its own removal set, and
   reproduced in hand-built fixtures
   (`crates/pdfce-core/tests/form_delete_page_tree.rs`).

   ★★ **It has now found THREE things, two fixed and one open** — see §C item
   5 for the table, the replay seed and the harness traps. The fuzz gate is
   the one this role skips, twice recorded and twice recurred, and the first
   target written after saying so out loud found a months-old defect in
   fifteen minutes and two more behind it.

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

5. **★★ THE FUZZ TARGET HAS FOUND THREE THINGS AND ONE IS STILL OPEN.**
   `fuzz/fuzz_targets/form_edit_sequence.rs`, in its first afternoon:

   | # | verb | what | state |
   |---|---|---|---|
   | 1 | `delete_field` | a field that is also a **page** → no page tree | **FIXED**, `Pass 185.1` |
   | 2 | `delete_field` | a field that is the **catalog** → `NoPageTreeRoot` | **FIXED**, `Pass 185.2` |
   | 3 | `delete_field_group` | `debug_assert_eq!` at `edit.rs:17486` — the emptied-node **cascade and prediction disagree** | **OPEN** |

   ★ **#2 is the one to learn from**: #1's fix was built from "the page tree",
   and the catalog — the object that *points at* the tree — is not in it. The
   error said `NoPageTreeRoot`, not `NoPages`, from the very first crash.

   **On #3, what is known and what is a guess.** Known: it is
   `delete_field_group`, it is the assertion that `remove_fields_from_form`'s
   `emptied` fixed point agrees with `group_deletion_preflight`'s tree walk,
   and it is **`debug_assert_eq!` — so in release it is a WRONG
   `nodes_removed` in a disclosure, not corruption.** Materially less severe
   than #1 and #2 and it should be sized accordingly.
   Guess, not measured: the preview filters `form.groups` by
   `fully_qualified_name != fqn` and then appends `fqn`, so **two grouping
   nodes sharing one FQN** — trivial on a malformed file, and `""` for any
   `/T`-less node, which is exactly what the fuzzer reaches — would make the
   two derivations count differently. Start there, but measure it.

   **How to run it, because the harness fights you:**
   - `-seed=1` makes a crash **replayable**; without it the same defect
     appears and vanishes across runs.
   - **libFuzzer writes NO artifact here.** Rust's abort on Windows exits
     `0xc0000409` before its crash handler saves one, so there is nothing to
     reduce — consider making the target print its own input on panic.
   - **Grep for the panic HEAD, never `tail` the output.** A `tail -40` keeps
     only libFuzzer's internal frames and drops both the message and the verb.
   - ASan DLL path and invocation:
     `.claude/agent-memory/pdfce-engineer/reference_fuzz_asan_dll.md`.

   ⇒ **The absence of a reproduction is not evidence of a fix.** #2 was filed
   as "seen once, maybe a different class" and turned out to be the same crash
   walking through an incomplete guard.

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
- `.tmp_bench.py` has been untracked for seven filings. It is deliberately not
  committed — the repository is public — so **stage by path, never `git add
  -A`**.
