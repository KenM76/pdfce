# Decision 038 — `/BaseState` propagation: the redundant array cannot override, so Table 101 and §8.11.4.5 b) do not actually disagree

- **Date:** 2026-08-10
- **Status:** DECIDED, with one named contingency (§4.4) that would reopen
  it as a setting. Ratifies the currently shipped resolution rule **on
  reasoning independent of the fact that it shipped** — see §3.6, which
  states the test that would have overturned it.
- **Claimed by:** `ARCHITECTURE.md` §12, 2026-08-10 (seventy-sixth filing).
- **Authored by:** `autonomous-builder` / KenAgent, per
  `docs/decisions/README.md`.
- **Clauses:** ISO 32000-1:2008 §8.11.4.3 Table 101 rows `BaseState`, `ON`,
  `OFF` (= ISO 32000-2:2020 Table 99); §8.11.4.5 a) and b).
- **Spec RAG:** `iso32000__s__8.11.md` DA-A10, DA.14 item 3;
  `iso32000__ref__optional_content_order.md` §5;
  `index.md`'s DA-A10 register row. **All three carry a recommendation this
  record overrules — see §6.**
- **Code radius:** `pdfce_core::annot::optional_content_default_off`
  (unchanged in behaviour), `pdfce_core::layers::LayerDiagnostics` (**one
  new counter**), the CLI and GUI diagnostics surfaces.
- **Related:** decision 037 (same clause, same day, shares the
  `OcDefaultState` refactor), decision 027 (refuse what has no good
  reading, disclose what has one).

---

## 1. The question

Two sentences of ISO 32000-1 describe how `/BaseState` and the
`/ON`//`/OFF` arrays combine, and they appear to disagree.

**Table 101, `BaseState` row:**

> After this initialization, the contents of the `ON` **and** `OFF` arrays
> shall be processed, overriding the state of the groups included in the
> arrays.

**§8.11.4.5 b):**

> The groups listed in **either** the `ON` or `OFF` array (depending on
> which one is **opposite** to `BaseState`) shall have their states
> adjusted.

`pdfce_core::annot::optional_content_default_off` implements §8.11.4.5 b):
under `/BaseState` `ON` it processes only `/OFF`; under `/BaseState /OFF`
it seeds from the registry and processes only `/ON`.

The two readings can only diverge for a group named in **both** arrays.
`ARCHITECTURE.md` recorded this as *"a genuine contradiction WITHIN the
standard's own two clauses, not merely an area the standard leaves
silent"* — a different kind of problem from the project's usual ambiguity
cases, and the reason a ruling was owed rather than a default.

---

## 2. The ruling

**The contradiction is apparent, not genuine, and it dissolves as soon as
Table 101 is read as a whole table instead of as one row.**

Table 101 does not contain only the `BaseState` row. It also contains the
`ON` and `OFF` rows, and each of those states that the array **matching**
the base state is *redundant*:

| Row | What it says about redundancy |
|---|---|
| `ON` | groups set ON when this config applies — **redundant if `BaseState` is `ON`** |
| `OFF` | groups set OFF when this config applies — **redundant if `BaseState` is `OFF`** |

Redundant means *carries no information*. An array that carries no
information cannot override anything. So:

- under `/BaseState` `ON`, `/ON` is redundant ⇒ only `/OFF` can change a
  state ⇒ a group in both arrays is **OFF**;
- under `/BaseState /OFF`, `/OFF` is redundant ⇒ only `/ON` can change a
  state ⇒ a group in both arrays is **ON**.

That is **exactly** §8.11.4.5 b)'s "the array opposite to `BaseState`".
Table 101's `BaseState` sentence remains true as written — both arrays
*are* processed; processing the matching one is simply a no-op — and
§8.11.4.5 b) is the shorthand that says so. One function, two descriptions,
no conflict.

**Stated as the rule an implementation follows:**

> The array that restates the base state is inert. The array that
> contradicts the base state decides. A group named in both is decided by
> the array opposite `BaseState`.

Equivalently, and this is the formulation worth carrying because it
generalises: **the listing that does work wins over the listing that
restates the default.**

**pdfce's shipped resolution is therefore correct and does not change.**
What *does* change is that a both-listed group is a self-contradictory file
and is currently invisible to the operator — see §5.

---

## 3. Why — the reasoning that actually drives it

### 3.1 The reconciliation is preferred to picking a winner, and that is a substantive choice

The obvious move is to declare one clause normative and the other
descriptive. That move was available and was **rejected**, because a
reading that makes every sentence true is strictly better evidence about
what the committee meant than a reading that has to discard one. Nothing in
either sentence is contradicted by the redundancy rule; both survive
intact. A "one clause wins" ruling would have left a live claim that ISO
32000-1 contradicts itself here, which would then propagate into the
corpus, into doc comments, and into every future reading of §8.11.

### 3.2 Table 101's `BaseState` sentence never specified an order, so it cannot be the operative rule on its own

"The contents of the `ON` and `OFF` arrays shall be processed" does not say
which is processed first. For a group in both arrays that sentence, read
alone, has **no answer** — it is not a competing rule, it is an incomplete
one. Any implementation claiming to follow it must invent an order, and an
invented order wearing a clause's name is worse than an honest choice.

This is why "process both arrays, it is a superset and cannot lose
information" — the corpus's own recommendation (§6) — does not work as a
semantics. A superset of *what*? The union of the two arrays is not a
state; it is a set of groups whose state is then decided by an ordering the
standard never gives. "Cannot lose information" is a property of a set
operation, and the thing being computed is a function.

### 3.3 §8.11.4.5 is the clause whose job is the algorithm

Independently of §3.1: §8.11.4.5 is a lettered procedure — a), b), c) —
whose entire purpose is to specify how a configuration is applied. Table
101 is an entry-description table; its `BaseState` cell exists to tell an
*author* what setting the key does. Where a table cell summarises an
algorithm that a procedural clause specifies, the procedural clause governs
the algorithm. That is the ordinary reading rule for a standard containing
both, and it is the same rule the corpus already applies elsewhere (e.g.
treating Annex H's worked examples as oracles for prose that under-specifies).

So the reconciliation and the clause-priority argument point the same way.
The ruling holds under either, which is what makes it safe to state
without waiting on §6's verbatim re-read.

### 3.4 The rule assigns no unstated role to either array

Under the ruling, `/ON` means "these groups are ON despite the base saying
OFF" and `/OFF` means "these are OFF despite the base saying ON". Each
array is an exception list to the base — exactly what its Table 101 row
describes, and nothing more.

The two alternatives both hand one array a second job no sentence gives it:

- **"process `/OFF` then `/ON`" (ON always wins)** makes `/ON`
  simultaneously an exception list *and* a tiebreaker that defeats an
  explicit `/OFF`. Under `/BaseState` `ON` this means a group listed in
  `/OFF` is **not** turned off — `/OFF` becomes conditionally inert, which
  is a surprising thing for the array whose only purpose is hiding.
- **"process `/ON` then `/OFF`" (OFF always wins)** hands `/OFF` the same
  extra role in the mirror case.

Neither extra role is stated anywhere. The ruling's rule needs no extra
role for anything.

### 3.5 Consequence check against pdfce's disclosure posture

pdfce's standing tolerance direction is *shown-by-mistake is arguable;
hidden-by-mistake is invisible*. The ruling is not uniform on that axis and
should not be defended as if it were:

| `/BaseState` | Group in both arrays | Ruling | Direction |
|---|---|---|---|
| `ON` (the only value legal in `/D`) | in `/ON` and `/OFF` | **OFF** — hidden | hides |
| `OFF` (legal only in `/Configs`) | in `/ON` and `/OFF` | **ON** — shown | shows |

The hiding case is the reachable one, so it needs a defence rather than a
shrug. The defence: the file listed the group in `/OFF`, and `/OFF`'s whole
purpose is to hide. Honouring an explicit `/OFF` is not a guess and not an
inference — it is the file's own instruction. What makes it acceptable
under rule 4 is not that it errs toward visible (it does not) but that the
contradiction is **disclosed** and the result is **reversible** in the
Layers panel. §5 makes the disclosure real; today it does not exist, which
is the actual defect this decision fixes.

### 3.6 The independence test — what would have overturned the shipped behaviour

R15 forbids settling an ambiguity by whichever implementation shipped
first, so this ruling has to be checkable against that charge. The test:
**had `optional_content_default_off` shipped the "process `/OFF` then
`/ON`" rule instead — the rule the spec corpus itself recommends — would
this record have ratified it?**

**No.** That rule fails §3.4 (it gives `/ON` an unstated tiebreaker role)
and fails §3.2 (its ordering is invented, not sourced), and it contradicts
Table 101's own `ON` row, which says `/ON` is redundant under `/BaseState`
`ON` — a rule under which `/ON` decides a both-listed group is a rule under
which `/ON` is not redundant. The record would have called for the change.
The reasoning is therefore load-bearing and not decorative; that it lands
on the shipped answer is a result, not a premise.

### 3.7 pdfce's own operator-facing surface already applies the sibling principle

`pdfce-cli`'s `resolve_layer_override` refuses a layer name passed to both
`--show-layer` and `--hide-layer`:

> That is refused rather than resolved by flag order: the operator asked
> for two contradictory things and there is no reading of the command line
> that says which one they meant. Order-dependence would make the same two
> flags mean different things depending on how a script assembled them.

The same contradiction, in a file, is **not** refused — and the difference
is decision 027's exactly. On the command line there is no good reading, so
pdfce refuses. In a file there **is** a good reading (the redundancy rule),
so pdfce resolves it and discloses. Note also the shared instinct: neither
surface resolves the contradiction by *order of appearance*, which is the
one resolution both would find easiest and both reject.

---

## 4. Setting, or defended default? — **DEFENDED DEFAULT, with a named contingency**

### 4.1 Why not a setting

A setting is the right answer when the standard genuinely forks and a
competent reader could land either side. After §2 there is no fork: three
of the four relevant sentences (§8.11.4.5 b), Table 101's `ON` row, Table
101's `OFF` row) describe one function, and the fourth (Table 101's
`BaseState` sentence) is consistent with it and under-determined on its
own. Filing a resolved reading in the settings register as contested would
misrepresent the corpus and would hand the operator a choice with no
observable to base it on: the divergent case is a **self-contradictory
file**, and no operator preference makes a file mean two things.

The reachability seals it. The rule differs from the alternatives only for
a group in **both** arrays; `/D` — the only configuration pdfce reads — is
required to have `/BaseState` `ON`; and no fixture in
`fixtures/synthetic/layers/` exercises it (`basestate-off.pdf` has
`/ON [5 0 R] /OFF [6 0 R]`, disjoint by construction). A persisted global
key for that is a knob whose maintenance cost exceeds its use.

### 4.2 What replaces the knob

A **counter and a disclosure**, not a policy. §5.2 adds
`LayerDiagnostics::contradictory_on_off_groups`. The operator sees *"this
document lists 1 layer in both `/ON` and `/OFF`; pdfce resolved it per
§8.11.4.5 b)"* and, if they disagree, toggles that layer — a per-document
control that already exists on both surfaces. That is a strictly better
instrument than a global setting, because the contradiction is a property
of one file.

### 4.3 The residue that is genuinely undefined — `/BaseState /Unchanged`

`/Unchanged` is the one place the reconciliation does **not** reach, and it
is recorded here rather than buried, because a later session will otherwise
rediscover it.

Under `/Unchanged`, neither array restates the base — there is no base to
restate — so **neither is redundant**, §8.11.4.5 b)'s "opposite" has no
referent, and a both-listed group has no defined answer at all. DA-A10
already flags this.

The ruling for pdfce, in two parts:

1. **At document open, `/Unchanged` is treated as `ON`** — which is what
   the code does today (`base_off` is true only for the literal name `OFF`,
   so `/Unchanged` falls to the else branch). The justification is not "it
   falls in the else branch": at open there is no prior state for a group
   to remain unchanged *from*, Table 101's own default for `BaseState` is
   `ON`, and a group about which the configuration says nothing is visible.
   Under that base, `/ON` becomes redundant again and `/OFF` decides —
   i.e. the general rule applies unchanged.
2. **It becomes genuinely undefined only if pdfce ever applies a
   configuration to an already-open document** — switching to a
   `/Configs[i]`, or honouring a `SetOCGState` action. There "unchanged"
   has a real referent (the current state), both arrays carry information,
   and a both-listed group is a coin-flip the standard does not call.
   **That** is the point at which this question would deserve a setting,
   and it is not reachable today because the renderer reads only `/D`.

Recorded as an owed sub-decision on the `/Configs` work, not as an open
item now.

### 4.4 The contingency — the one finding that converts this to a setting

The reconciliation in §2 rests on Table 101's `ON` and `OFF` rows
containing the redundancy statements. **That is currently sourced only from
the spec corpus's own summary table**
(`iso32000__s__8.11.md` lines 138–139: *"Redundant if `BaseState` is
`ON`"* / *"…is `OFF`"*), which is a paraphrase — the corpus quotes the
`BaseState`, `Order`, `RBGroups` and `Locked` rows verbatim but **not** the
`ON` and `OFF` rows. A `pdfce-spec-librarian` verbatim pass on those two
rows is the discharging action.

**Stated explicitly, per the dispatch's instruction, under each reading:**

| If the verbatim `ON`//`OFF` rows… | Then |
|---|---|
| **contain the redundancy statements** (expected) | This ruling stands as written, at its strongest — the reconciliation holds and §3.3 is a second, independent support. |
| **contain them only as ISO NOTEs** (informative) | Ruling stands. A NOTE is not normative, but it is direct evidence of the committee's own reading of the array's role, and §3.3 (procedural clause governs) carries the normative weight by itself. Record the tier as: normative §8.11.4.5 b) + informative confirmation ×2. |
| **do not contain them at all** | The reconciliation collapses and DA-A10's "genuine contradiction" characterisation is restored. **Then it becomes a setting: `OC-A1`, `BaseStateArrayPropagation`, variants `OppositeArrayOnly` (default, §3.3's clause-priority argument survives the collapse untouched) and `BothArraysOffWins`; evidence tier (d).** `ON`-wins is deliberately *not* offered as a variant under any reading — it makes `/OFF` conditionally inert (§3.4), which no reading of any of the four sentences supports. |

`OC-A1` is **reserved** for that contingency; decision 037 reserves `OC-A2`
for its own. Recorded so a later session does not mint a colliding id.

---

## 5. Implementation

### 5.1 The resolution rule — **no change**

`optional_content_default_off`'s branch structure already implements the
ruling and stays as it is (modulo decision 037's conversion of the same
function to a per-group resolver, which preserves this logic exactly:
`if base_off { !on.contains(g) } else { off.contains(g) }` is the ruling,
written as one line).

What must change is the **documentation** at that site. The function's doc
comment currently says only *"`/BaseState` (default `ON`) sets all groups,
then `/ON`//`/OFF` override"*, which describes neither reading precisely and
would let a future editor "fix" it toward the corpus's recommendation. It
should state the redundancy rule, cite Table 101's `ON`//`OFF` rows
alongside §8.11.4.5 b), and point here.

### 5.2 The disclosure — **the actual change this decision requires**

A group in both `/ON` and `/OFF` is a self-contradictory file, and pdfce
currently resolves it silently. Under rule 4 an inference this load-bearing
must be visible.

- `pdfce_core::layers::LayerDiagnostics` gains
  `pub contradictory_on_off_groups: usize` — the count of groups named in
  both the active configuration's `/ON` and `/OFF` arrays. Computed as
  `on ∩ off` while the configuration is parsed, so it costs one set
  intersection over two arrays that are already read.
- It joins the existing `is_clean()`-style rollup that
  `unregistered_groups` and `base_state_off_in_default` participate in
  (`layers.rs` ≈ line 773), so a document with the contradiction does not
  report a clean layer structure.
- Surfaced by the CLI's layer diagnostics output and the GUI's Layers panel
  diagnostics, in the same place and phrasing as the existing
  `unregistered_groups` / `base_state_off_with_unregistered` disclosures —
  a new row, not a new mechanism.
- The message names the resolution, not just the fault: *"N group(s) listed
  in both `/ON` and `/OFF`; resolved per §8.11.4.5 b) — the array opposite
  `/BaseState` decides."* A disclosure that says only "this file is
  contradictory" leaves the operator unable to predict what they are
  looking at.

### 5.3 Tests and fixtures

- **New fixture** `fixtures/synthetic/layers/on-off-contradiction.pdf`:
  `/D << /BaseState /ON /ON [7 0 R] /OFF [7 0 R] >>` with group 7 painting
  visible content, plus a control group in neither array. Asserts the group
  is OFF (`visible_by_default == false`), the control is ON, and
  `contradictory_on_off_groups == 1`. Rule 7: synthetic.
- **A second fixture, or a `/Configs` unit test, for the mirror case** —
  `/BaseState /OFF` with a both-listed group resolving **ON**. This is the
  arm no shipped code path reaches today (the renderer reads only `/D`),
  which is exactly why it needs a test: it is the arm a future `/Configs`
  implementation will silently inherit, and an untested inherited rule is
  how the mirror case gets "simplified" into the wrong one.
- A render-level assertion on the first fixture: the both-listed group's
  content is not painted, `oc_hidden` increments.

### 5.4 Documentation to correct in the same change

`crates/pdfce-core/src/layers.rs` §"A contradiction inside the standard,
and which side pdfce takes" (≈ lines 303–319) states the divergence as
unresolved and endorses the superset reading as *"the safer of the two — it
cannot lose information"*. Both halves are superseded: there is no genuine
contradiction (§2), and the superset reading is not a semantics (§3.2).
Replace with the ruling and a pointer here; keep the section, because
naming the apparent conflict is still valuable to a reader coming from
DA-A10.

---

## 6. Where this record overrules the spec corpus (routed to `pdfce-spec-librarian`, not edited here)

Three places in `D:\Dev\Rag-Specialized\PDF_Spec\` recommend the reading
this decision rejects. They are listed for the librarian; **this record has
not modified the RAG**, because `pdfce-spec-librarian` owns it and was
dispatched concurrently.

1. **`iso32000__s__8.11.md` DA-A10** (≈ line 775): *"**Process both
   arrays** (Table 101's reading) — it is a superset and cannot lose
   information."* → §3.2. A superset of a set is not a resolution of a
   function.
2. **`iso32000__ref__optional_content_order.md` §5** — the initial-state
   pseudocode applies `/OFF` then `/ON`, making **`/ON` always win**, and
   labels the order *"a pdfce choice, disclosed"*. It is not pdfce's
   choice: pdfce ships the opposite answer for `/BaseState` `ON`, which is
   the only base state legal in `/D`. **This is a live disagreement between
   the corpus's recommended algorithm and shipped behaviour, in the
   configuration pdfce actually reads** — not a `/Configs`-only edge, and
   the more urgent of the three to correct.
3. **`index.md`'s DA-A10 register row** (≈ line 2877) — same "Process
   **both** (superset)" recommendation, carried into the top-level index
   where it is most likely to be read without the surrounding argument.

Additionally, and this is the finding worth carrying beyond §8.11:
**DA-A10 should be re-graded.** It is currently listed in DA.12 as a
genuine ambiguity. If the §4.4 verbatim check confirms the `ON`//`OFF`
redundancy statements, DA-A10 is not an ambiguity at all — it is a
**partial-reading artifact**: an apparent conflict produced by comparing
one table row against a clause instead of reading the table. That is a
distinct and reusable category, and the corpus has no id class for it. It
is worth one, because the same failure shape will recur wherever a table
row summarises an algorithm specified elsewhere — and the shape is
expensive precisely because a partial reading looks like a finding.

---

## 7. What would falsify this ruling

1. **★ The §4.4 verbatim check** — Table 101's `ON` and `OFF` rows do not
   contain the redundancy statements. Converts this to setting `OC-A1`
   (§4.4). Cheapest and most decisive; should be done before §5.2 lands.
2. **Acrobat Reader resolves a both-listed group the other way.** Build the
   §5.3 fixture, open it in Reader, observe whether the layer paints. If
   Reader shows it under `/BaseState` `ON`, then the dominant
   implementation applies `/OFF`-then-`/ON` and the corpus's recommendation
   was right about behaviour if not about reasoning — that is tier-(a)
   evidence and would justify either flipping the default or installing
   `OC-A1` with `BothArraysOffWins`… note the naming: an Acrobat result
   showing the group **visible** under `/BaseState` `ON` means *`/ON`
   wins*, which §3.4 argues no reading supports, so that observation would
   be a genuinely surprising result and should be re-run before being
   believed.
3. **ISO 32000-2 amends either sentence.** The corpus's 2.0 sweep (DA.9)
   reports Table 99 (2.0's Table 101) amended only in its `RBGroups` row,
   and §8.11.4.5 not amended at all — but that rests on the PDF
   Association's public change record, a secondary source. A primary read
   finding an edit to the `BaseState`, `ON`, or `OFF` rows reopens this.
4. **pdfce implements `/Configs` switching or `SetOCGState`.** Does not
   falsify the ruling, but activates §4.3's `/Unchanged` residue, which
   *is* genuinely undefined and would need its own call at that point.

---

## 8. References

- ISO 32000-1:2008 §8.11.4.3 Table 101 rows `BaseState`, `ON`, `OFF`
  (= ISO 32000-2:2020 Table 99); §8.11.4.5 a), b).
- `D:\Dev\Rag-Specialized\PDF_Spec\iso32000\iso32000__s__8.11.md` — DA-A10
  (both sentences quoted), DA.14 item 3 (the recorded divergence notice),
  Table 101 summary rows 137–139.
- `D:\Dev\Rag-Specialized\PDF_Spec\iso32000\iso32000__ref__optional_content_order.md`
  §5 — the initial-state pseudocode this record overrules.
- `docs/ARCHITECTURE.md` §12, seventy-sixth filing — the claim this record
  discharges, including its statement that the divergence "has shipped
  without a visible defect on any fixture built so far", which §5.3
  converts from an observation into a covered case.
- `docs/decisions/027-refuse-what-has-no-good-reading-disclose-what-has-one.md`
  — why the file contradiction is resolved-and-disclosed while the
  equivalent CLI contradiction is refused (§3.7).
- `docs/decisions/037-base-state-off-covers-unregistered-groups.md` — same
  clause, same day; its `OcDefaultState` refactor is the natural carrier
  for this ruling's doc comment and for §5.2's intersection.
- `CLAUDE.md` rules 1 (spec fidelity — the ISO edition is cited at every
  table reference, per DA.9), 4 (fuzzy never sneaky — §5.2 is the rule-4
  obligation this decision creates), 6 (documentation-first), 15 (no
  dimension terminology arises in this record).

---

## ADDENDUM 2026-08-10 — the contingency is discharged, and the ruling stands at full strength

Appended, not edited: the ruling above is unchanged and this records what
was measured after it was written. (`docs/decisions/README.md`: records are
append-only history.)

The ruling was made **contingent** on Table 101's verbatim `ON` and `OFF`
rows containing the "redundant if `BaseState` is X" statements — the
corpus had them only in a summary table, and if they were absent the
reconciliation collapsed and this became setting `OC-A1`.

A `pdfce-spec-librarian` dispatch re-extracted both loci from the staged
`PDF32000_2008.pdf` and **cross-verified with a second extractor**
(`pdfminer.six` against `pypdf`, agreeing sentence-for-sentence). The rows
are present, verbatim:

> `ON` — array — (Optional) An array of optional content groups whose
> state shall be set to ON when this configuration is applied. **If the
> `BaseState` entry is `ON`, this entry is redundant.**
>
> `OFF` — array — (Optional) An array of optional content groups whose
> state shall be set to OFF when this configuration is applied. **If the
> `BaseState` entry is `OFF`, this entry is redundant.**

**Contingency discharged. No setting is minted. `OC-A1` stays unused.**

Three further measurements, each of which could have weakened the ruling
and did not:

- **No ISO drafting convention settles table-vs-clause precedence.**
  `precedence` occurs **0 times** in ISO/IEC Directives Part 2 (9th ed.).
  The ruling correctly did not lean on one.
- **No erratum or PDF 2.0 change touches this.** The PDF Association's
  ISO 32000-2:2020 clause-8 change record contains **0 occurrences** of
  `BaseState`, `opposite`, or `8.11.4.5`.
- **ISO 32000-1 states no internal precedence rule**: `in case of
  conflict` — 0 hits in 756 pages.

### ★ The independent derivation, which is stronger than the one above

The librarian reached the same conclusion by a different and better route:
treat "this entry is redundant" as a **testable** claim and check it
against each candidate order. For a group listed in both arrays:

| `BaseState` | Order applied | Result | Result with the "redundant" array deleted | Redundancy claim |
|---|---|---|---|---|
| `ON` | `ON` then `OFF` | OFF | OFF | **holds** |
| `ON` | `OFF` then `ON` | ON | OFF | **FALSIFIED** |
| `OFF` | `OFF` then `ON` | ON | ON | **holds** |
| `OFF` | `ON` then `OFF` | OFF | ON | **FALSIFIED** |

**Table 101 admits exactly one processing order** — matching array first
(a no-op, since `BaseState` just set every group to that value), opposite
array last. Which is §8.11.4.5 b) verbatim. The two loci are one function
with a redundant no-op prepended, identical on every input.

That argument needs no clause-priority principle at all, which matters
because the priority principle turned out to be unsourceable. The ruling
above reached the right answer partly by an argument that could not have
been checked; this one can be, and was.

**One asymmetry points the other way and is recorded rather than
suppressed.** §8.11.4.5 self-declares as recapitulative — *"This
sub-clause summarizes the rules…"*, a phrase occurring exactly once in
the whole standard. Read naively that says the table is where the rule
lives, i.e. process both arrays, i.e. **against** this ruling. It is
defeated on content by the falsification table, not dismissed.

### Two corrections to this record's own framing

- **The divergence is one cell, not four.** Only `/BaseState /OFF` with a
  both-listed group divides the readings, and only under an order Table
  101 forbids. Under `/BaseState ON` all three candidate readings agree
  on OFF.
- **"They diverge only for a both-listed group" is false for
  `/Unchanged`**, where §8.11.4.5 b)'s "opposite" selector has no
  referent, so it processes *neither* array while Table 101 processes
  both — a divergence for **every** listed group, not only both-listed
  ones.

### `/BaseState /Unchanged` — the record was right about the answer and wrong about why

This record justified pdfce's ON-branch treatment by clause reasoning.
The clause read shows there is **no clause to reason from**:

1. `/Unchanged` in `/D` **violates a `shall`** outright — Table 101: *"If
   `BaseState` is present in the document's default configuration
   dictionary, its value shall be `ON`."*
2. It is also semantically empty there. §8.11.2.1: *"States themselves are
   not part of the PDF document"*, and at first open there is no prior
   state to leave unchanged. `/Unchanged` exists for the *other* consumer
   of Table 101 — a `/Configs` configuration applied to an already-open
   document, which is why the row is scoped *"when this configuration is
   applied"* rather than "when the document is opened".
3. No reader behaviour is specified for the violation.

So the ON branch is a **recovery policy for non-conforming input**, not an
application of §8.11.4.5. Still the right recovery, on two grounds that
should be cited instead of the clause: Table 101 gives `ON` as the default
*and* requires `/D` to be `ON`, so it is the only value `/D` was ever
allowed to have; and it is the safe direction — the rival repair ("leave
everything as found, process no arrays") makes `/D /OFF` inert and paints
every layer the author turned off.

**Implementation consequence:** the code comment must say *recovery from a
non-conforming value*, not cite the clause, and the case wants a
disclosure under rule 4 exactly as `/Intent` got in `b6a9ca0`.

**Residual ambiguity, narrowed and PERMANENT:** `/Unchanged` **plus** a
both-listed group is genuinely undetermined — no "opposite" array exists
and neither array is declared redundant, so no order is forced. Reachable
only through `/Configs`, which pdfce does not apply. Tracked in the spec
corpus as `DA-A10′`.

### What this changes in the required work

Nothing in the resolution rule; `if base_off { !on.contains(g) } else {
off.contains(g) }` stands. The disclosure work
(`contradictory_on_off_groups`) is unchanged and still owed, and gains a
sibling: the `/Unchanged` arm needs its own comment and disclosure.
