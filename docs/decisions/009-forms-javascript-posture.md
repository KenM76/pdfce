# Decision 009 — pdfce's posture on embedded form/document JavaScript

- **Date:** 2026-07-31
- **Status:** Recommended (for engineer ratification when Pass 7 is scoped)
- **Decider:** KenAgent (autonomous-builder / decision-consultant), per the
  ROADMAP standing rule "KenAgent decision routing (operator process rule,
  2026-07-30)", answering the Pass 7 open sub-decision flagged in
  decision 008 §5.1 and in the Pass 6.2 ROADMAP entry.
- **Question:** What is pdfce's posture on embedded PDF JavaScript —
  AcroForm calculation (`C`), validation (`V`), formatting (`F`),
  keystroke (`K`) scripts, and document-level scripts — for Pass 7
  (Interactive forms / AcroForm)? This is a **security + feature-parity**
  decision.
- **Outcome:** **Never execute anything.** Ship a phased hybrid: posture A
  (recognize + disclose + byte-exact round-trip, zero execution) as the
  mandatory floor and the entirety of Pass 7's first JavaScript scope;
  posture B (native reimplementation of an exact-match whitelist of
  well-known Acrobat built-ins) as a bounded, opt-in, off-by-default,
  demand-driven enhancement deferred to Pass 7.x; posture C (a sandboxed
  JS engine) rejected outright and prohibited by standing rule.
- **Adds standing rules:** R-JS-1 … R-JS-5 (librarian assigns the next
  numbers in the R43–R52 sequence; provisionally R53–R57).
- **Supersedes / amends:** nothing. **Discharges** the "embedded
  JavaScript scope trap" decision 008 §5.1 routed to a decision record,
  and the Pass 6.2 ROADMAP entry's "embedded-JavaScript posture is a
  Pass-7 open sub-decision" note.

---

## 0. CORRECTION (2026-08-10) — §1's Bibliography claim was FALSE; the conclusion survives on different, weaker footing

**Filed by `pdfce-librarian` at the engineer's request, sourced from a
`pdfce-spec-librarian` measurement against the staged ISO 32000-1 source
this session.** Read this section BEFORE §1 — it corrects the load-bearing
claim §1 is built on, without deleting §1 (append-only discipline; the
retraction is recorded here rather than the original wording erased).

**The false claim.** §1 said ISO 32000-1 §12.6.4.16 "defers entirely to
two external NON-ISO documents (Mozilla Client-Side JS Reference + Adobe
JavaScript for Acrobat API Reference)" which are "in the Bibliography"
(informative, non-normative). **This is false.** Both documents are listed
in ISO 32000-1 **clause 3, "Normative references"** — *JavaScript for
Acrobat API Reference, Version 8.0 (April 2007)* at clause 3 line 367,
*Client-Side JavaScript Reference (May 1999), Mozilla Foundation* at line
478. Neither appears in the Bibliography (which starts at line 37395+ and
contains zero JavaScript/RFC/Unicode entries).

**Why the error happened, and why it is not isolated to this decision.**
§12.6.4.16's own text carries a parenthetical "(see the Bibliography)"
pointing at these two references. That parenthetical is **itself a
systematic erratum in the standard** — the same wrong pointer recurs at
≥8 sites across ISO 32000-1 (Unicode Standard, RFC 1321, RFC 2045, RFC
3161, Adobe Glyph List, UAX #29, XDP, and these two JavaScript
references), all of which are in fact clause-3 Normative references.
**Never treat "(see the Bibliography)" in ISO 32000-1 as evidence a
reference is informative** — this is now recorded corpus-wide by
`pdfce-spec-librarian`.

**The conclusion survives, on a different and weaker argument.** ISO
32000-1 has a formula for binding an external document normatively —
"shall conform to", used on Adobe Technical Note #5014 (§9.7.5.3), XFA 2.0
(§12.7.3.4), and RFC 2315 (§12.8.1). **§12.6.4.16 uses none of that
formula.** The two documents are invoked only to "give details on the
contents and effects of JavaScript scripts" — a weaker, descriptive
invocation — and the clause's own obligation is phrased permissively
("**may** update their values"), imposing no obligation on any processor
to produce a particular computed value.

**The accurate verdict, replacing §1's "there is no normative JS behavior
to conform to" and §5 bullet 4's "there is nothing to conform to":**

> Non-execution is a **deliberate, disclosed decision not to implement
> one clause whose content the standard did not itself fully specify** —
> **not** "there is no normative behaviour to conform to." The stronger
> phrasing rested on the retracted Bibliography claim. **Everywhere in
> this document that says "nothing to conform to" or "no normative JS
> behavior," read it through this correction** — the practical outcome
> (never execute) is UNCHANGED; only the strength of the ISO-conformance
> argument for it is revised downward.

**A second execute-`shall` this decision did not count.** Also newly
found: the catalog `/Names /JavaScript` name tree carries its own
unconditional shall — *"When the document is opened, all of the actions
in this name tree **shall** be executed."* Document-level, on-open,
unconditional (unlike the per-action, user-triggered `/AA`/`/OpenAction`
shalls this decision already discussed). Hollow for the same corrected
reason (§12.6.4.16's descriptive-not-conformance-binding invocation
applies equally here), but **any statement that "the only execute-shall
is per-action and user-triggered" is now known inaccurate** and must not
be repeated.

**ISO 32000-2 delta — recorded as an OPEN QUESTION, not a finding.**
§12.6.4.16 becomes **§12.6.4.17 in ISO 32000-2**, retitled "JavaScript
actions" → "**ECMAScript** actions." 32000-2's Introduction states
"ISO/DIS 21757-1 replaces several Adobe, ECMA and ISO publications
related to ECMAScript in PDF 2.0." **Whether ISO 21757-1 is invoked with
a "shall conform to"-class formula (this decision's whole hinge) or a
descriptive one, as in 1.7, is PAYWALLED and UNVERIFIED.** Do not cite a
32000-2 posture for this decision until that verb is confirmed.

**Every NF4-style spec-finding citation from this decision should say
"ISO 32000-1," never "PDF" or "the spec"** — the edition matters, per the
32000-2 delta above.

---

## 1. The finding that settles this before the ranking begins

**Read §0 above first — this section's central claim (the Bibliography
sentence, and by extension "no normative JS behavior to conform to") was
CORRECTED 2026-08-10. The surviving argument is the invocation-verb one
(no "shall conform to"-class formula used), not the Bibliography-location
one. The text below is left as originally written, per this project's
append-only discipline for decision records; do not re-derive the old
Bibliography argument from it.**

The brief's most important input is a **spec finding**, not a preference:
ISO 32000-1 contains a **"hollow shall."**

§12.6.4.16 says a conforming processor *"shall execute a script … in the
JavaScript programming language."* But ISO 32000 defines **no JavaScript
semantics, no API, no DOM, and no security model.** It defers entirely to
two **external, non-ISO** documents — the Mozilla Client-Side JavaScript
Reference and the Adobe *JavaScript for Acrobat API Reference*. What ISO
32000 actually specifies is only:

- the **carrier** — Table 217: an action dictionary with `S = /JavaScript`
  and `JS = <string | stream>`; and
- the **hook points** — §12.6.3's trigger events (`K` keystroke, `F`
  format, `V` validate, `C` calculate; page `O`/`C`; document `WC`/`WS`/
  `DS`/`WP`/`DP`), field and document `/AA` dictionaries, the `/CO`
  calculation-order array, and the `/Names /JavaScript` document-level
  name tree.

Plus §12.6.4.16 **NOTE 2**, which warns that a triggered action *"can
occur outside the described scope of the event"* — i.e. the spec itself
flags that JS execution is not bounded by the document model.

**Consequence, and it is load-bearing:** there is **no normative JS
behavior to conform to.** An ISO 32000 conformance claim depends on
honoring the *carrier and hook structure*, not on running a script.
**Non-execution forfeits nothing an ISO conformance claim requires.**
Whatever else follows, "pdfce does not run embedded JavaScript" is a
fully conformant posture. This removes the only argument that could have
forced execution.

Corroborating: **PDF/A forbids JavaScript actions** at several conformance
levels outright, so a recognize-and-disclose posture is not merely
tolerated by the archival profiles pdfce cares about — it is *aligned*
with them (verify exact levels via `pdfce-spec-librarian` and cite).

---

## 2. The reframe — A, B, C are not three doors

The brief poses A / B / C as mutually exclusive. **They are not**, and
seeing why is most of the decision:

- **B cannot exist without A.** Posture B reimplements a *whitelist* of
  well-known helpers. Every script that is *not* on the whitelist —
  every genuinely custom calculation, every validation script, every
  document-level script — must fall back to "recognized but not
  executed," which **is** posture A. So B is literally "A, plus a
  native fast-path for a recognized subset."
- **C is the only real alternative to (A or A+B),** and it is the one the
  project's invariants forbid.

So the real question is two smaller ones:

1. **A alone, or A + an opt-in B layer on top of it?** → Both, phased.
2. **Is C ever acceptable?** → No. Prohibited.

The ranking, correctly stated: **A (floor) > B (opt-in, on top of A) ≫ C
(rejected).** A and B are not competitors; only C is.

---

## 3. Why C is rejected outright (and prohibited, not merely deferred)

Posture C — embed a sandboxed Rust JS engine (boa, quickjs-rs, …) and run
the scripts — is ranked last and made a **standing prohibition**, for
reasons that are structural to this project rather than matters of taste:

1. **It re-imports the exact problem Adobe built a broker process to
   contain.** Adobe's own Application Security Guide documents a
   sandbox/broker architecture *because* embedded PDF JS is a real,
   actively-exploited attack surface. §12.6.4.16 NOTE 2 says the quiet
   part out loud: a triggered action can act outside the event's scope.
   pdfce would have to reproduce a security boundary that a company with
   Adobe's resources treats as a dedicated subsystem.
2. **The hook points reference actions R12 and R13 hard-prohibit.** A
   trigger's action can be `/URI` or `/SubmitForm` or `/ImportData`
   (network — **R12** forbids it) or `/Launch` (process — **R13** forbids
   it). Executing the JS layer means either honoring those (impossible
   under R12/R13) or building a second, ad-hoc "execute-but-not-really"
   sandbox — the worst of both worlds.
3. **It contradicts the whole distribution posture** (decision 003 / the
   no-network, minimal-surface single-folder portable): a JS interpreter
   is a large dependency with its own MSRV, wasm portability, CVE stream,
   and license-classification burden (rule 13), added to a project whose
   defining virtue is a small, auditable, offline surface.
4. ~~**There is nothing to conform to** (§1's hollow shall).~~
   **CORRECTED 2026-08-10 (§0): the underlying Bibliography claim was
   false; the accurate statement is that §12.6.4.16 invokes its two
   external references descriptively, never with a "shall conform to"
   -class formula, so ISO conformance does not oblige a particular
   computed value.** C still pays the entire cost of an interpreter for
   a clause whose content the standard did not itself fully specify.

C is therefore not "deferred until we have time" — it is **out of scope
permanently** (R-JS-1). Recording it as a prohibition, not a backlog
item, is deliberate: it stops future Passes from drifting into it under
feature pressure, the same way R17 fenced `harfrust` out of everything
but a future authoring path.

---

## 4. Why A is the floor, and ships first and alone

Posture A — recognize every JS-driven field, **disclose** that its value
is script-computed and not run by pdfce, show the stored value as-is, and
round-trip the script **byte-exact** — is the mandatory floor, and it is
the *entire* JavaScript scope of Pass 7's first cut. Five reasons:

1. **Zero attack surface, zero new dependency, fully conformant** (§1).
2. **It is honest and self-correcting downstream.** A disclosed
   uncomputed value re-computes *correctly* the instant any
   JS-executing reader (Acrobat, pdf.js with scripting, …) opens the
   file. The cost — a stale total until such a tool opens it — is a
   **disclosed** cost.
3. **A disclosed-uncomputed value is categorically better than a
   silently-baked WRONG one** — and this is the decisive argument for the
   whole decision. pdfce is an **editor whose output other tools
   consume.** This is the R43/R44 failure mode, one level over from
   pixels into semantics: R44 exists so pdfce never writes an appearance
   that "looks right in pdfce and blank everywhere else." A silently
   baked computed value is the same disease — a number that looks
   authoritative in pdfce and is wrong for every downstream consumer,
   with no signal that pdfce invented it. A stale-but-disclosed value can
   never do that.
4. **It closes a silence, the way Pass 6.0 did for annotations.** Under
   R20/R27 this project counts and names everything it cannot do. Today
   pdfce cannot execute JS *and does not say so.* Posture A makes the
   non-execution a counted, disclosed, per-field fact — the same "fix the
   gap and fix the silence in the same Pass" move decision 008 §3.2 made
   for annotations.
5. **It is the read-and-model side, and it measures demand.** Pass 7's
   recognition histogram (how many real fields use whitelisted built-ins
   vs custom scripts, across the organic + conformance corpora) is the
   demand signal that tells us whether posture B is even worth building —
   exactly as Pass 6.0's `annotations_without_ap` histogram drove Pass
   6.1's authoring priorities. Build the measurement before the
   generator.

### 4.1 Pass 7's first JavaScript scope (posture A) — concretely

- **Model every carrier** in `pdfce-core`: field `/AA` (`K`/`F`/`V`/`C`),
  document `/AA` (`WC`/`WS`/`DS`/`WP`/`DP`), `/OpenAction` when it is or
  contains a JS action, the `/CO` calculation-order array, and the
  `/Names /JavaScript` document-level name tree.
- **Resolve** each `S = /JavaScript` action's `/JS` (string **or** stream)
  to script *text*, without evaluating it.
- **Classify** each script by pure text matching: `RecognizedBuiltin`
  (exact-shape whitelist match, §6), `Custom`, or `Unparseable`.
- **Disclose** per field and at document level (§7's contract).
- **Byte-preserve** every carrier on save (§8).
- **Count** everything (Diagnostics keys appended, never reordered):
  `fields_with_{calculate,format,validate,keystroke}_script`,
  `recognized_builtin_calc` (by helper), `recognized_builtin_format`,
  `custom_scripts`, `unparseable_scripts`, `doc_level_scripts`,
  `open_action_is_javascript`, `aa_actions_by_type` — with any
  network/launch-referencing `/AA` action (`/URI`, `/SubmitForm`,
  `/ImportData`, `/Launch`) counted and flagged loudly (R50/R27
  tradition).
- **CLI** `list-scripts` (or `forms --scripts`): the locale-invariant
  stable-line inventory.
- **GUI** (dispatch `pdfce-ui-specialist`): computed-field badge +
  disclosure on inspection; a read-only "Document scripts" inspector;
  **no execution controls.** Placement per the continuation-23 taxonomy
  (disclosure → status bar / inspector).
- **Non-goals, binding:** no recompute of any value; no trigger fires, on
  load or on interaction; no `/NeedAppearances`-driven regeneration
  beyond counting (R51 already binds this); no `/SubmitForm` / `/ResetForm`
  / `/ImportData` dispatch (recognized + counted, never run — two of
  those are R12 violations regardless).

---

## 5. Why B is built, but opt-in, off by default, and deferred to Pass 7.x

Posture B — natively reimplement, in Rust, an exact-match whitelist of the
well-known Acrobat built-ins, so common totals and formatting work
without any interpreter — is worth building, because the common case is
common: `AFSimple_Calculate` totals and `AFNumber`/`AFDate` formatting are
how ordinary forms do arithmetic and presentation. B restores functional
parity on that case while running **no** script.

But B is deliberately **opt-in, off by default per-document, and deferred
to a later sub-Pass (Pass 7.x)**, for reasons that are all
project-invariant, not convenience:

1. **Pattern recognition is brittle.** A script that *looks like*
   `AFSimple_Calculate` but has been edited, wrapped, or conditionalized
   is not it. The whole safety of B rests on **erring hard toward
   `Custom`**: a false-negative (a real built-in treated as custom) is a
   disclosed stale value (safe); a false-positive (a custom script
   mis-recognized and computed) is a wrong bake (unsafe). Making B
   operator-invoked keeps every computed value a **reviewable hint**
   (rule 4 — fuzzy, never sneaky), never a silent authority.
2. **An editor must never bake a computed value as a side effect of
   opening a file** (§4, reason 3). Off-by-default guarantees the mere
   act of loading + saving a form never changes a computed `/V`.
3. **Demand-driven, measured first.** B is scoped only after Pass 7's
   histogram shows the whitelist carries a materially large share of real
   computation — the decision-008 "measure before you build the
   generator" discipline (6.0 read-side → 6.1 authoring), applied one
   subsystem over.

### 5.1 How a B recompute behaves (when built, and enabled)

- It is an **`EditSession` command** — undoable, one command per operator
  intent, visible in the edit diff. **Never** a load-time or save-time
  side effect.
- The recomputed `/V` is written; the **source script is left in place.**
- **Why leave the script in place:** if pdfce's native recompute ever
  diverges from Acrobat's real JS semantics on an edge case (hidden or
  read-only operands, non-numeric values, locale differences), a
  downstream JS-executing reader recomputes and *corrects* pdfce's value
  on the next dependent-field change. Removing the script would freeze
  pdfce's possibly-divergent value as authoritative — the opposite of
  fail-safe.
- **Format vs value stay separate.** `AF*_Format` helpers change
  **display only**, never `/V`; reimplementing them means choosing the
  appearance string fed to the Pass 6.2 / Pass 7 variable-text generator,
  while `/V` is always stored raw. The two code paths must not merge — a
  formatted string must never be baked into `/V`.

---

## 6. The recognized-helper whitelist (posture B)

Sourced (behavior/capability only, never Adobe code) from the Adobe
*JavaScript for Acrobat API Reference* via `pdfce-acrobat-librarian`
(the `forms__calculation_validation_javascript.md` bucket), and cited in
code doc comments.

**Calculation — changes `/V`:**

| Helper | Operation |
|---|---|
| `AFSimple_Calculate("SUM", <field array>)` | sum of named fields |
| `AFSimple_Calculate("AVG", <field array>)` | average |
| `AFSimple_Calculate("PRD", <field array>)` | product |
| `AFSimple_Calculate("MIN", <field array>)` | minimum |
| `AFSimple_Calculate("MAX", <field array>)` | maximum |

**Formatting — changes display only, never `/V`:**

| Helper |
|---|
| `AFNumber_Format(nDec, sepStyle, negStyle, currStyle, strCurrency, bCurrencyPrepend)` |
| `AFPercent_Format(nDec, sepStyle)` |
| `AFDate_Format(pdfFormat)` / `AFDate_FormatEx(cFormat)` |
| `AFTime_Format(pdfFormat)` |
| `AFSpecial_Format(psf)` (zip, zip+4, phone, SSN) |

**Recognized but NOT reimplemented in the first B cut (disclosed only):**

- `AF*_Keystroke` input filters — advisory; pdfce fills are
  operator-reviewed, so keystroke validation is disclosed, not enforced
  by execution.
- `AFRange_Validate(...)` — disclosed as a constraint, not enforced by
  execution.
- **Simplified Field Notation** arithmetic — Acrobat compiles it to a
  recognizable script, but with higher parse risk than the fixed
  `AFSimple` shapes; a separate, later addition, never the first B cut.

**Matching discipline:** exact structural/shape match to Acrobat's
canonical generated form and its argument grammar. Any deviation
(wrapped, edited, concatenated, conditionalized) ⇒ `Custom` ⇒ posture A.
The whitelist is drawn **conservatively**, biased toward `Custom`, for the
false-negative-is-safe / false-positive-is-unsafe reason in §5.

---

## 7. Disclosure UX contract

Every disclosure names three things: **what** computes the value,
**whether pdfce ran it** (always: no), and **whether the shown value may
be stale.** This is R51's disclosed-never-silent pattern applied to
semantics.

- **Recognized built-in calc, B enabled + operator-accepted:**
  > "Total computed by pdfce: SUM of {FieldA, FieldB, FieldC} = {value}.
  > Recognized Acrobat built-in (`AFSimple_Calculate`). Source script
  > preserved; downstream readers recompute independently."
- **Recognized built-in calc, B disabled (the default):**
  > "This field's value is computed by a recognized Acrobat built-in
  > (AFSimple SUM of {…}) that pdfce does not execute. Showing the stored
  > value as last saved: {value}. It may be stale if you changed its
  > inputs. (Enable Recompute to update it.)"
- **Custom script (always posture A):**
  > "This field's value is computed by a document script pdfce does not
  > run. Showing the stored value as last saved: {value}. It may be stale
  > if you changed its inputs."
- **Format-only script:**
  > "This field's DISPLAYED value is formatted by a script pdfce does not
  > run; the raw stored value is {value}."
- **Document-level and `/AA` scripts** (read-only "Document scripts"
  inspector):
  > "This document carries {n} document-level script(s) and {m} action
  > trigger(s) that run automatically in Adobe Acrobat/Reader. pdfce does
  > not execute any of them."
  > Any `/AA` action referencing the network (`/URI`, `/SubmitForm`,
  > `/ImportData`) or a launch (`/Launch`) is flagged explicitly as a
  > **blocked capability** (R12/R13).

Nothing is presented as authoritative pdfce computation unless the
operator explicitly opted in **and** accepted it.

---

## 8. Save-time guarantee

- **Byte preservation.** The `/JS` strings/streams, all `/AA`
  dictionaries, the `/CO` calculation-order array, the `/Names
  /JavaScript` name tree, and `/OpenAction` re-emit **byte-identical**
  (untouched under incremental save; verbatim under full rewrite) per the
  round-trip / minimal-diff invariant. **pdfce never strips a script** —
  removing one silently changes document semantics and corrupts the
  document for every downstream JS-executing consumer.
- **Never execute-and-bake.** pdfce never runs a script and writes the
  result as a load- or save-time side effect. A recomputed value (posture
  B) is written **only** as an explicit, operator-accepted, undoable
  `EditSession` edit to `/V`, with the source script **left in place**
  (§5.1).

---

## 9. `/AA` and document-open-script posture — never auto-run

**Rule:** never auto-run on load or on any interaction — the direct
semantic sibling of **R51** (`/NeedAppearances` is a disclosed condition,
never a silent auto-generate).

- **Covers:** field triggers (`K`/`F`/`V`/`C`), page triggers (`O`/`C`),
  document triggers (`WC`/`WS`/`DS`/`WP`/`DP`), `/OpenAction` JS, and the
  `/Names /JavaScript` document-level tree (Acrobat's on-open scripts).
- **Enforcement:** **R12** (no network) + **R13** (no process launch).
  Trigger actions can be `/URI` / `/SubmitForm` / `/ImportData` (network)
  or `/Launch` (process); all are hard-prohibited. Recognition is pure
  data modeling — there is **no JS action dispatcher** in pdfce and none
  is added.
- **Disclosure:** counted and surfaced. A document that runs scripts on
  open is a fact the operator is entitled to know — the R50
  hidden-annotation logic applied to scripts; auto-run document scripts
  are a recognized document-forensics / malware vector.

---

## 10. FieldMDP / certification interaction

- **A recompute is a form-fill edit.** A posture-B recompute that changes
  `/V` is a form-field-value modification and routes through the
  **existing** DocMDP certification gate (`signature.rs` /
  `SignatureImpact`, Pass 3.2) and the conservative certification gating
  inherited from Passes 6.1/6.2 (X11). No new signature logic — it is a
  machine-suggested form fill.
- **`/FieldMDP` lock.** A recompute that would change a `/FieldMDP`-locked
  field is **refused by name** — never silently applied, never silently
  skipped (the fuzzy-never-sneaky refusal shape).
- **`/DocMDP` permission.** `P ≥ 2` permits form filling (recompute
  allowed; may affect signature validity per `SignatureImpact`); `P = 1`
  forbids changes (recompute refused by name). Same either/or
  classification pdfce already ships.
- **Read side is unaffected.** Posture A (Pass 7's first scope) changes no
  `/V` and raises no certification question at all — another reason A
  ships first and clean.

---

## 11. Standing rules added (R-JS-1 … R-JS-5)

Binding, in the R43–R52 tradition; the librarian assigns the actual next
numbers (provisionally R53–R57).

- **R-JS-1 — pdfce never executes embedded PDF JavaScript.** Not field
  `/AA`, not document `/AA`, not `/OpenAction`, not `/Names /JavaScript`,
  not built-in, not custom. There is no JS interpreter in pdfce, and
  adding one (posture C) is prohibited scope, not deferred scope.
- **R-JS-2 — no trigger event ever fires.** On load or on any
  interaction. The semantic sibling of R51, enforced by R12 (no network)
  and R13 (no process launch) because trigger actions can reference
  `/URI`, `/SubmitForm`, `/ImportData`, `/Launch`.
- **R-JS-3 — JS carriers are byte-preserved; never stripped, never
  silently baked.** All `/JS`, `/AA`, `/CO`, `/Names /JavaScript`, and
  `/OpenAction` round-trip byte-identical. A recomputed value is only ever
  an explicit, reviewable, undoable `EditSession` edit that leaves the
  source script in place.
- **R-JS-4 — recognize + disclose; recompute is opt-in, whitelisted,
  fuzzy-never-sneaky.** JS-driven fields are recognized, classified,
  counted, disclosed. Native recompute is limited to an exact-match
  built-in whitelist, OFF by default per-document, and every recomputed
  value is a reviewable hint the operator accepts or overrides — never
  silent, never authoritative, always leaving the source script in place
  as the downstream authority.
- **R-JS-5 — a recompute changing `/V` is DocMDP/FieldMDP-gated.**
  Subject to the existing certification gate and refused by name if it
  would alter a `/FieldMDP`-locked field.

---

## 12. Risks & mitigations

1. **Pattern-recognition brittleness.** → Exact-canonical-shape match
   only; any deviation ⇒ `Custom` ⇒ posture A; matcher biased toward
   `Custom` (false-negative safe, false-positive unsafe); B off by
   default; every recompute operator-reviewed.
2. **Stale-value confusion.** → Explicit staleness disclosure; a
   Pass-4-style reliability gate warns before export/flatten/extract when
   un-recomputed computed fields are present.
3. **Scope creep toward a JS engine.** → C is a standing prohibition
   (R-JS-1), not a backlog item.
4. **Recompute diverging from Acrobat's JS semantics on edge cases.** →
   Leave the source script in place (downstream self-corrects); B is
   opt-in + reviewable + undoable, so a wrong recompute is caught, never
   authoritative.
5. **Format-vs-value conflation.** → Distinct code paths; `AF*_Format`
   influences appearance-string generation only, `/V` stored raw.
6. **`/AA` actions referencing network/launch accidentally dispatched.**
   → R-JS-2 + R12/R13; recognition is pure modeling, no JS action
   dispatcher exists.
7. **A baked wrong value corrupting the document for every downstream
   consumer** (R43/R44 "looks right in pdfce, wrong everywhere else", one
   level over into semantics). → The decisive argument for A-as-floor and
   B-opt-in-leaving-script-in-place: a disclosed uncomputed value is
   honest and self-correcting; a silent wrong bake is neither.

---

## 13. Spec prerequisites

- **`pdfce-spec-librarian`** — confirm the RAG covers the full
  carrier + hook model: §12.6.4.16 (JavaScript action) incl. NOTE 2,
  Table 217 (`S=/JavaScript`, `JS`), §12.6.3 trigger-event table, field
  triggers §12.7.5.2, the `/CO` calculation-order array, the
  `/Names /JavaScript` document-level name tree (§7.7.4 + §12.6.4.16), and
  `/OpenAction`. The brief indicates §12.7.4 NF4 and §12.6 are gathered —
  verify `/CO` and the name-tree carrier are explicit.
- **`pdfce-spec-librarian`** — record the **hollow-shall** finding
  formally (§1): §12.6.4.16 defers JS semantics/API/security entirely to
  two non-ISO external documents, so ISO conformance imposes no
  executable-JS obligation; non-execution is fully conformant. Cite in
  code doc comments.
- **`pdfce-acrobat-librarian`** — source the exact canonical shapes +
  behavioral semantics of `AFSimple_Calculate(SUM/AVG/PRD/MIN/MAX)` and
  the `AFNumber`/`AFPercent`/`AFDate`/`AFTime`/`AFSpecial` `_Format`
  helpers from the Adobe *JavaScript for Acrobat API Reference* —
  behavior/capability only, never copying Adobe code — so the whitelist
  matcher and the native reimplementation are grounded in real Acrobat
  behavior (the `forms__calculation_validation_javascript.md` bucket).
- Confirm **PDF/A and PDF/UA** impose no JS-execution requirement (PDF/A
  forbids JS actions at several levels) — recognition + disclosure aligns;
  verify exact conformance levels and cite.

---

## 14. Documentation-first obligations

- Code doc comments cite §12.6.4.16 + NOTE 2 and the hollow-shall finding
  at the non-execution site.
- Each recognized-builtin's native reimplementation documents its
  Adobe-API-Reference behavioral citation and its exact-match grammar.
- The §7 disclosure strings are the binding UX contract (this record +
  the `pdfce-ui-specialist` spec).
- This decision archived verbatim as
  `docs/decisions/009-forms-javascript-posture.md`.

## References

- `docs/decisions/008-next-subsystem-after-extract.md` §5.1 — the
  embedded-JavaScript scope trap routed here; the "recommend never-execute"
  seed.
- ROADMAP standing rules **R12** (no network), **R13** (no process
  launch), **R43/R44** (never a private/invented render — the semantic
  sibling of a silent bake), **R50** (hidden content honored AND counted),
  **R51** (`/NeedAppearances` disclosed, never silent auto-generate),
  and project rule **4** (fuzzy, never sneaky).
- `docs/decisions/003` — no-network distribution posture.
- ISO 32000-1 §12.6.3, §12.6.4.16 (+NOTE 2), Table 217, §12.7.4/§12.7.5,
  §7.7.4.
- Adobe *JavaScript for Acrobat API Reference* (behavioral reference only,
  via the Acrobat parity RAG).

## Appendix A — JSON decision block

```json
{
  "decision_id": "009-forms-javascript-posture",
  "title": "pdfce's posture on embedded form/document JavaScript (AcroForm calculation, validation, formatting, and document-level scripts)",
  "date": "2026-07-31",
  "status": "Recommended (for engineer ratification when Pass 7 is scoped)",
  "decision": "NEVER EXECUTE any embedded PDF JavaScript — not field /AA, not document /AA, not /OpenAction, not the /Names/JavaScript document-level tree, not recognized built-ins, not custom scripts, not on load, not on any interaction. The three candidate postures are NOT mutually exclusive as the brief frames them: B is impossible without A (custom scripts always fall back to A), so the real choice is 'A alone' vs 'A + an opt-in B layer strictly on top of A', and when to build B. Adopt a PHASED HYBRID: (1) Posture A (recognize + disclose + byte-exact round-trip, zero execution) is the MANDATORY FLOOR and is the entirety of Pass 7's first JavaScript scope. (2) Posture B (native Rust reimplementation of an exact-match whitelist of well-known Acrobat built-ins) is a BOUNDED, OPT-IN, OFF-BY-DEFAULT, per-document enhancement deferred to a later sub-Pass (Pass 7.x), demand-driven by the recognition histogram Pass 7 measures. Every B recompute is a reviewable, undoable EditSession edit that leaves the source script in place as the downstream authority — never silent, never authoritative, never a strip-and-bake. (3) Posture C (a sandboxed JS engine) is REJECTED OUTRIGHT and made a standing prohibition.",
  "ranking": {
    "reframe": "A and B are not alternatives; A is the floor, B is an optional layer on top of A. Only C is genuinely rejected.",
    "order": "A (floor, ship first) > B (opt-in, deferred, demand-driven) >> C (rejected, prohibited)",
    "A_rationale": "Zero attack surface, zero new dependency, honest, self-correcting downstream (a disclosed-uncomputed value re-computes correctly the moment any JS-executing reader opens the file). Directly satisfies R12/R13 and the 'output other tools consume' constraint. The one cost — stale computed values until a JS-executing tool opens the file — is a DISCLOSED cost, which is categorically better than a silently-baked WRONG value.",
    "B_rationale": "Restores functional parity on the common case (totals, number/date/percent/special formatting) WITHOUT running an interpreter, by pattern-matching a conservative whitelist and computing natively. It is the fuzzy-never-sneaky shape (rule 4): a reviewable value the operator sees, verifies, accepts or overrides. Deferred and opt-in because pattern-recognition is brittle and an editor must never bake a computed value as a side effect of opening a file. Built only after Pass 7 measures how many real forms actually use the whitelisted built-ins vs custom scripts — the same 'measure before you build the generator' discipline decision 008 used (6.0 read-side before 6.1 authoring).",
    "C_rationale": "REJECTED. Re-imports the exact security problem Adobe built a broker/sandbox process to contain (ISO 32000-1 §12.6.4.16 NOTE 2: a triggered action 'can occur outside the described scope of the event'); /AA hook points can reference /URI, /SubmitForm, /Launch, /ImportData actions that directly violate R12 (no network) and R13 (no process launch); adds a large dependency with its own MSRV/wasm/licensing (rule 13) and attack surface; and the spec defines NO JS security model to conform to (the 'hollow shall'). Wrong for a minimal-surface, no-network, no-process-launch editor. Prohibited by new standing rule."
  },
  "spec_finding_hollow_shall": "ISO 32000-1 §12.6.4.16 says a conforming processor 'shall execute a script … in the JavaScript programming language' but the standard defines NO JavaScript semantics, API, DOM, or security model. CORRECTED 2026-08-10 (see correction_2026_08_10 below): the two referenced documents (Mozilla Client-Side JS Reference + Adobe JavaScript for Acrobat API Reference) are NOT in the Bibliography — both are ISO 32000-1 clause-3 Normative References, and the clause's own '(see the Bibliography)' pointer is a systematic erratum recurring at 8+ sites in the standard. The surviving argument is the INVOCATION VERB, not the reference's informative/normative status: §12.6.4.16 never uses the 'shall conform to'-class formula the standard uses elsewhere (Adobe TN #5014, XFA 2.0, RFC 2315) to bind an external document normatively — it only says the two documents 'give details on the contents and effects,' and the clause's own consequence is permissive ('may update their values'). ISO 32000 specifies only the CARRIER (Table 217: S=/JavaScript, JS=<string|stream>) and the HOOK POINTS (§12.6.3 trigger events; field/doc /AA; /CO calculation order; /Names/JavaScript name tree, which carries its own separate unconditional on-open execute-shall), plus NOTE 2's out-of-scope warning. Non-execution is therefore a deliberate, disclosed decision not to implement a clause whose content the standard did not itself fully specify -- NOT 'there is no normative behavior to conform to' (that stronger phrasing is retracted). Practical outcome UNCHANGED: 'never execute' remains fully ISO-conformant.",
  "correction_2026_08_10": {
    "retracted": "The claim that the Mozilla Client-Side JS Reference and the Adobe JavaScript for Acrobat API Reference are in ISO 32000-1's (informative) Bibliography.",
    "measured_fact": "Both are in ISO 32000-1 clause 3, Normative References (JavaScript for Acrobat API Reference V8.0 at clause-3 line 367; Client-Side JavaScript Reference May 1999 at line 478). The Bibliography (line 37395+) contains zero JavaScript/RFC/Unicode entries. Measured by pdfce-spec-librarian against the staged ISO 32000-1 source.",
    "erratum_found": "Section 12.6.4.16's own '(see the Bibliography)' parenthetical is wrong, and is one of >=8 such wrong pointers in the standard (also: Unicode Standard, RFC 1321, RFC 2045, RFC 3161, Adobe Glyph List, UAX #29, XDP). Never treat '(see the Bibliography)' in ISO 32000-1 as evidence of informative status.",
    "surviving_argument": "The invocation-verb argument: ISO 32000-1 binds an external document NORMATIVELY only via a 'shall conform to'-class formula (used on Adobe TN #5014 section 9.7.5.3, XFA 2.0 section 12.7.3.4, RFC 2315 section 12.8.1). Section 12.6.4.16 uses none of that formula -- the two documents are invoked only to 'give details on the contents and effects of JavaScript scripts,' and the clause's own consequence is permissive ('may update their values').",
    "second_execute_shall_found": "The /Names/JavaScript document-level name tree carries its OWN unconditional shall, separate from the field/document /AA and /OpenAction shalls already discussed in this decision: 'When the document is opened, all of the actions in this name tree shall be executed.' Any statement in this decision or elsewhere that the only execute-shall is per-action/user-triggered is inaccurate.",
    "iso_32000_2_delta_UNVERIFIED": "Section 12.6.4.16 becomes section 12.6.4.17 in ISO 32000-2, retitled 'JavaScript actions' to 'ECMAScript actions'; the 32000-2 Introduction states ISO/DIS 21757-1 replaces several Adobe/ECMA/ISO ECMAScript-in-PDF publications. Whether ISO 21757-1 is invoked with a 'shall conform to'-class formula (this decision's hinge) or descriptively as in 1.7 is PAYWALLED and NOT YET VERIFIED -- do not cite a 32000-2 posture for this decision until confirmed. Every NF4-style citation from this decision should say 'ISO 32000-1', never 'PDF' or 'the spec'."
  },
  "first_implementation_scope_pass_7": {
    "summary": "Pass 7's ENTIRE JavaScript scope is posture A: recognize, classify, count, disclose, byte-preserve. ZERO execution, ZERO recompute. Native recompute (B) is explicitly NOT in Pass 7's first scope.",
    "core_pdfce_core": [
      "Model every JS carrier: field /AA (K keystroke, F format, V validate, C calculate), document /AA (WC/WS/DS/WP/DP), /OpenAction when it is or contains a JS action, the /CO calculation-order array, and the /Names/JavaScript document-level name tree (the scripts Acrobat runs on document open).",
      "Resolve each S=/JavaScript action's /JS (string OR stream) to its script text WITHOUT evaluating it.",
      "Classify each script: RecognizedBuiltin (exact-shape match to the whitelist), Custom (arbitrary script), or Unparseable (could not extract /JS text). Classification is pure text pattern-matching — never execution.",
      "Expose a per-field and document-level JavaScript inventory API (which fields have calc/format/validate/keystroke scripts; the /CO order; the doc-level scripts; the /AA action types present, including any that reference network/launch actions).",
      "NO trigger ever fires; there is no action dispatcher for JS in the codebase (R-JS-1/R-JS-2)."
    ],
    "diagnostics_counters_appended": [
      "fields_with_calculate_script", "fields_with_format_script", "fields_with_validate_script", "fields_with_keystroke_script",
      "recognized_builtin_calc (by helper: SUM/AVG/PRD/MIN/MAX)", "recognized_builtin_format (by helper family)",
      "custom_scripts", "unparseable_scripts", "doc_level_scripts (Names/JavaScript entries)",
      "open_action_is_javascript", "aa_actions_by_type (incl. any URI/SubmitForm/Launch/ImportData carriers, counted and named per R50/R27 tradition)"
    ],
    "cli": "A `list-scripts` (or `forms --scripts`) subcommand emitting the locale-invariant stable-line inventory: per field, script kind + classification + (for recognized built-ins) the parsed operation and operand field list; document-level scripts listed separately; any network/launch-referencing /AA action flagged loudly.",
    "gui": "Disclosure surface (dispatch pdfce-ui-specialist): computed fields show a badge and, on inspection, the disclosure string (see disclosure_contract). Document-level and /AA scripts surfaced in a read-only 'Document scripts' inspector. No execution controls exist. Follow the settled continuation-23 placement taxonomy (disclosure -> status bar / inspector).",
    "save_guarantee": "All JS carriers round-trip byte-identical under the minimal-diff invariant. Nothing is stripped, nothing is executed-and-baked.",
    "explicit_non_goals_pass_7": [
      "No native recompute of ANY value (that is Pass 7.x / posture B).",
      "No execution of any trigger, on load or on interaction.",
      "No /NeedAppearances-driven appearance regeneration for widget fields beyond counting (R51 already binds this).",
      "No form SUBMIT / RESET / import-export action handling (those are /SubmitForm, /ResetForm, /ImportData actions — recognized + counted, never dispatched; SubmitForm/ImportData are also R12 network violations)."
    ]
  },
  "deferred_scope_pass_7x_posture_B": {
    "trigger_to_build": "Pass 7's measured recognition histogram (recognized_builtin_calc / recognized_builtin_format vs custom_scripts across the organic + conformance corpora) shows the whitelisted built-ins carry a materially large share of real form computation. This is the exact demand-signal-drives-generator pattern of decision 008 (annotations_without_ap histogram drove 6.1).",
    "default_state": "OFF per-document. Default behavior remains posture A (disclose-only). The operator explicitly opts in per document ('Recompute recognized totals / re-format recognized fields').",
    "mechanism": "Recompute is an EditSession command (undoable, appears in the edit diff, one command per operator intent) — never a load-time or save-time side effect. The recomputed /V is written; the source calc script is LEFT IN PLACE so a downstream JS-executing reader remains authoritative and self-corrects any pdfce edge-case divergence (fail-safe).",
    "format_vs_value_separation": "AF*_Format helpers change DISPLAY only, never /V. Reimplementing them means choosing the appearance string fed to the Pass 6.2/7 variable-text generator; the raw /V is always stored unformatted. Calc helpers (SUM/AVG/PRD/MIN/MAX) change /V. These two code paths MUST stay distinct — a formatted string must never be baked into /V."
  },
  "recognized_helper_whitelist": {
    "calculation_changes_V": [
      "AFSimple_Calculate(\"SUM\", <field-name array>)",
      "AFSimple_Calculate(\"AVG\", <field-name array>)",
      "AFSimple_Calculate(\"PRD\", <field-name array>)  (product)",
      "AFSimple_Calculate(\"MIN\", <field-name array>)",
      "AFSimple_Calculate(\"MAX\", <field-name array>)"
    ],
    "formatting_changes_display_only": [
      "AFNumber_Format(nDec, sepStyle, negStyle, currStyle, strCurrency, bCurrencyPrepend)",
      "AFPercent_Format(nDec, sepStyle)",
      "AFDate_Format(pdfFormat) and AFDate_FormatEx(cFormat)",
      "AFTime_Format(pdfFormat)",
      "AFSpecial_Format(psf)  (zip, zip+4, phone, SSN)"
    ],
    "recognized_but_NOT_reimplemented_in_first_B_cut": [
      "AFNumber_Keystroke / AFPercent_Keystroke / AFDate_Keystroke / AFTime_Keystroke / AFSpecial_Keystroke (input filters — advisory only; pdfce fills are operator-reviewed, so keystroke validation is disclosed, not enforced)",
      "AFRange_Validate(bGreaterThan, nGreaterThan, bLessThan, nLessThan) (range validation — disclosed as a constraint, not enforced-by-execution)",
      "Simplified Field Notation arithmetic (Acrobat compiles it to a recognizable script; higher parse risk than the fixed AFSimple shapes — a SEPARATE, later addition, not the first B cut)"
    ],
    "matching_discipline": "EXACT structural/shape match only. Acrobat generates these helpers from its UI in a canonical, stable textual form; pdfce matches that canonical form and its argument grammar precisely. ANY deviation (wrapped, edited, concatenated, conditionalized, or otherwise non-canonical) => classified Custom => posture A (disclosed-not-computed). The whitelist is drawn CONSERVATIVELY: a false-negative (a real built-in treated as custom) is a disclosed stale value (safe); a false-positive (a custom script mis-recognized) is a wrong bake (unsafe) — so the matcher errs hard toward Custom. The behavioral semantics of each helper are sourced from the Adobe JavaScript for Acrobat API Reference via pdfce-acrobat-librarian (behavior/capability only — never copying Adobe code), cited in code doc comments."
  },
  "disclosure_contract": {
    "recognized_builtin_calc_field_B_enabled": "'Total computed by pdfce: SUM of {FieldA, FieldB, FieldC} = {value}. Recognized Acrobat built-in (AFSimple_Calculate). Source script preserved; downstream readers recompute independently. [Recompute is off by default — this value was operator-accepted.]'",
    "recognized_builtin_calc_field_B_disabled_default": "'This field's value is computed by a recognized Acrobat built-in (AFSimple SUM of {…}) that pdfce does not execute. Showing the stored value as last saved: {value}. It may be stale if you changed its inputs. (Enable Recompute to update it.)'",
    "custom_script_field": "'This field's value is computed by a document script pdfce does not run. Showing the stored value as last saved: {value}. It may be stale if you changed its inputs.'",
    "format_only_field": "'This field's DISPLAYED value is formatted by a script pdfce does not run; the raw stored value is {value}.'",
    "document_level_and_AA_scripts": "Surfaced in a read-only 'Document scripts' inspector: 'This document carries {n} document-level script(s) and {m} action trigger(s) that run automatically in Adobe Acrobat/Reader. pdfce does not execute any of them.' Any /AA action referencing the network (/URI, /SubmitForm, /ImportData) or a launch (/Launch) is flagged explicitly as a blocked capability (R12/R13).",
    "principle": "Every disclosure names WHAT computes the value, WHETHER pdfce ran it (always: no), and WHETHER the shown value may be stale. Nothing is presented as authoritative pdfce computation unless the operator explicitly opted in AND accepted it. This is R51's disclosed-never-silent pattern applied to semantics."
  },
  "save_time_guarantee": {
    "byte_preservation": "The /JS strings/streams, all /AA dictionaries, the /CO calculation-order array, the /Names/JavaScript name tree, and /OpenAction are re-emitted byte-identical (untouched under incremental save; verbatim under full rewrite) per the round-trip/minimal-diff invariant. pdfce NEVER strips a script — removing one silently changes document semantics and would corrupt the document for every downstream JS-executing consumer.",
    "never_execute_and_bake": "pdfce NEVER executes a script and writes the result as a load- or save-time side effect. A recomputed value (posture B) is written ONLY as an explicit, operator-accepted, undoable EditSession edit to /V, and the source script is LEFT IN PLACE.",
    "why_leave_script_in_place": "If pdfce's native recompute ever diverges from Acrobat's actual JS semantics on an edge case (hidden/readonly operands, non-numeric values, locale), leaving the calc script present means the next JS-executing reader recomputes and corrects the value on the next dependent-field change. Removing the script would freeze pdfce's possibly-divergent value as authoritative — the opposite of fail-safe."
  },
  "aa_and_document_open_script_posture": {
    "rule": "NEVER auto-run on load or on any interaction — the direct semantic sibling of R51 (/NeedAppearances is disclosed, never silently auto-generated).",
    "covers": "Field triggers (§12.6.3 K/F/V/C), page triggers (O/C), document triggers (WC willClose, WS willSave, DS didSave, WP willPrint, DP didPrint), /OpenAction JS, and the /Names/JavaScript document-level tree (Acrobat's on-open scripts).",
    "enforcement": "Enforced by R12 (no network) + R13 (no process launch): trigger actions can be /URI, /SubmitForm, /ImportData (network) or /Launch (process) — all hard-prohibited. Recognition is pure data modeling; there is no JS action dispatcher in pdfce and none is added.",
    "disclosure": "Counted and surfaced (a document that runs scripts on open is a fact the operator is entitled to know — the R50 hidden-annotation logic applied to scripts; auto-run scripts are a recognized document-forensics/attack vector)."
  },
  "fieldmdp_certification_interaction": {
    "recompute_is_a_form_fill_edit": "A posture-B recompute that changes /V is a form-field-value modification and routes through the EXISTING DocMDP certification gate (signature.rs / SignatureImpact, Pass 3.2) and the conservative certification gating inherited from Passes 6.1/6.2 (X11). No new signature logic — it is a machine-suggested form fill.",
    "fieldmdp_lock": "A recompute that would change a /FieldMDP-locked field is REFUSED BY NAME (never silently applied, never silently skipped) — the fuzzy-never-sneaky refusal shape.",
    "docmdp_permission": "Under /DocMDP, P>=2 permits form filling (recompute allowed, may affect signature validity per SignatureImpact); P=1 forbids changes (recompute refused by name). Same either/or classification pdfce already ships.",
    "read_side_unaffected": "Posture A (Pass 7's first scope) changes no /V and therefore raises no certification question at all — another reason A ships first and clean."
  },
  "proposed_standing_rules": {
    "note": "Numbering continues the R43–R52 tradition from decision 008; the librarian assigns the actual next numbers (provisionally R53–R57).",
    "R-JS-1_never_execute": "pdfce never executes embedded PDF JavaScript — field /AA, document /AA, /OpenAction, /Names/JavaScript, built-in or custom. There is no JS interpreter in pdfce and adding one (posture C) is prohibited scope.",
    "R-JS-2_never_auto_run": "No trigger event ever fires (semantic sibling of R51), enforced by R12 (no network) + R13 (no process launch) because trigger actions can reference /URI, /SubmitForm, /ImportData, /Launch.",
    "R-JS-3_byte_preserve_never_bake": "All JS carriers round-trip byte-identical; pdfce never strips a script and never executes-and-bakes a value as a load/save side effect. A recomputed value is only ever an explicit, reviewable, undoable EditSession edit that leaves the source script in place.",
    "R-JS-4_recognize_disclose_optin": "JS-driven fields are recognized, classified, counted, disclosed. Native recompute is limited to an exact-match built-in whitelist, OFF by default per-document, and every recomputed value is a reviewable hint the operator accepts or overrides (rule 4) — never silent, never authoritative.",
    "R-JS-5_fieldmdp_gating": "A recompute changing /V is subject to the existing DocMDP gate and refused by name if it would alter a /FieldMDP-locked field."
  },
  "risks": [
    "Pattern-recognition brittleness (a script textually resembling AFSimple_Calculate but edited/wrapped). MITIGATION: exact-canonical-shape match only; any deviation => Custom => posture A; matcher errs hard toward Custom (false-negative = safe disclosed stale value; false-positive = unsafe wrong bake). B off by default; every recompute operator-reviewed.",
    "Stale-value confusion (operator does not notice the shown value is stale). MITIGATION: explicit staleness disclosure string; a Pass-4-style reliability gate warns before export/flatten/extract when un-recomputed computed fields are present.",
    "Scope creep toward a JS engine over successive Passes. MITIGATION: posture C made a standing prohibition (R-JS-1), not just 'not now'.",
    "Recompute diverging from Acrobat's real JS semantics on edge cases (hidden/readonly operands, non-numeric values, locale). MITIGATION: leave the source script in place (downstream self-corrects); B is opt-in + reviewable + undoable so a wrong recompute is caught, never authoritative.",
    "Format-vs-value conflation baking a formatted string into /V. MITIGATION: distinct code paths; AF*_Format influences appearance-string generation ONLY, /V stored raw.",
    "/AA actions referencing network/launch being accidentally dispatched. MITIGATION: R-JS-2 + R12/R13; recognition is pure modeling, no JS action dispatcher exists.",
    "Baked wrong value corrupts the document for EVERY downstream consumer (the R43/R44 'looks right in pdfce, wrong everywhere else' failure, one level over into semantics). MITIGATION: this is the decisive argument for A-as-floor and B-opt-in-leaving-script-in-place; a disclosed uncomputed value is honest and self-correcting, a silent wrong bake is not."
  ],
  "spec_prerequisites": [
    "pdfce-spec-librarian: confirm the RAG fully covers the CARRIER + HOOK model — §12.6.4.16 (JavaScript action) incl. NOTE 2, Table 217 (S=/JavaScript, /JS string|stream), §12.6.3 (trigger events table K/F/V/C/O/C/WC/WS/DS/WP/DP), field triggers §12.7.5.2, the /CO calculation-order array (§12.7.4.x), the /Names/JavaScript document-level name tree (§7.7.4 + §12.6.4.16), and /OpenAction (§12.3.2 / §12.6). Brief indicates 12.7.4 NF4 and 12.6 already gathered — verify /CO and the name-tree carrier are explicit.",
    "pdfce-spec-librarian: record the 'hollow shall' finding formally — §12.6.4.16 defers JS semantics/API/security entirely to two non-ISO external documents, so ISO conformance imposes NO executable-JS obligation; non-execution is fully conformant. Cite in code doc comments.",
    "pdfce-acrobat-librarian: source the EXACT canonical shapes + behavioral semantics of AFSimple_Calculate(SUM/AVG/PRD/MIN/MAX) and the AFNumber/AFPercent/AFDate/AFTime/AFSpecial Format helpers from the Adobe JavaScript for Acrobat API Reference — behavior/capability ONLY (never copying Adobe code), so the whitelist matcher and the native reimplementation are grounded in real Acrobat behavior. This is the same forms JS parity bucket the brief cites (forms__calculation_validation_javascript.md).",
    "Confirm veraPDF / PDF/A and PDF/UA impose no JS-execution requirement (PDF/A forbids JS actions outright in several conformance levels — recognition + disclosure aligns; verify and cite)."
  ],
  "doc_first_obligations": [
    "Code doc comments cite §12.6.4.16 + NOTE 2 (out-of-scope warning), Table 217, and the hollow-shall finding at the non-execution site.",
    "Each recognized-builtin's native reimplementation documents its Adobe-API-Reference behavioral citation and its exact-match grammar.",
    "The disclosure strings are documented as the binding UX contract (this decision record + the ui-specialist spec).",
    "This decision archived verbatim as docs/decisions/009-forms-javascript-posture.md."
  ]
}
```

## Orchestrator note (2026-08-01, at archival)

**★ CORRECTED 2026-08-10 — see §0 above. The "hollow shall" / "fully
ISO-conformant, nothing to conform to" phrasing below rested on a false
Bibliography claim; the practical outcome (never execute) is unchanged,
but the ISO-conformance argument for it is the weaker invocation-verb
one, not "there is no normative JS behavior at all."**

Decision 009 archived from the KenAgent consultation, discharging the Pass-7 embedded-JavaScript open sub-decision flagged in decision 008 §5.1 and the Pass 6.2 ROADMAP entry. Outcome: NEVER execute embedded PDF JavaScript (ISO-conformant on the invocation-verb argument — see §0's correction of the original "hollow shall" claim). Posture A (recognize + classify + disclose + byte-exact round-trip, zero execution) is Pass 7's entire JavaScript scope and is already the posture the in-flight Pass 7 engineer was dispatched with. Posture B (native reimplementation of the exact-match AFSimple_Calculate + AF*_Format whitelist) is opt-in, off-by-default per-document, deferred to Pass 7.x, demand-driven by Pass 7's recognition histogram; every B recompute is a reviewable/undoable EditSession edit that leaves the source script in place. Posture C (a sandboxed JS engine) is REJECTED and prohibited by standing rule. Adds standing rules provisionally R-JS-1..R-JS-5 — the pdfce-librarian assigns the actual next numbers (expected R53—R57) when it files decision 009 alongside the Pass 7 ship; this record is the authority for their content. Spec prerequisites (verify §12.6 carrier/hook coverage + record the hollow-shall finding formally; source the AF* helper canonical shapes via pdfce-acrobat-librarian; confirm PDF/A forbids JS actions) are queued for when Pass 7.x/posture-B is scoped, non-blocking for Pass 7's posture-A floor.
