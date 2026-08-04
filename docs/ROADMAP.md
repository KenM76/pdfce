# pdfce — Roadmap

**The contract.** Every operator (Ken) request gets parsed into a Pass
entry here; every completion gets recorded here. Read this file at the
start of every session. Maintained by `pdfce-librarian`, dispatched by
`pdfce-engineer` — the engineer does not edit this file directly (see
`.claude/agents/pdfce-engineer.md`).

## Glossary

- **Pass** — a scoped unit of engineering work with acceptance criteria,
  numbered `Pass N[a-z]` (sub-letter when a feature needs splitting for
  shippability). IDs are stable and never reused.
- **pdfce-core** — the GUI-agnostic Rust crate: object model, parser,
  writer, filters, fonts, crypto, content-stream interpretation.
- **pdfce-render** — headless rasterizer (draw-ops -> pixels), no GUI deps.
- **pdfce-gui** — the native egui/eframe desktop shell.
- **pdfce-cli** — the command-line batch shell (merge/split/stamp/
  convert/sign/validate subcommands). Depends on pdfce-core +
  pdfce-render only, same zero-GUI-deps discipline as pdfce-gui. See
  `ARCHITECTURE.md` §7.
- **Incremental save** — appending a new xref section + changed objects
  only, leaving untouched bytes in place. Default save mode (see
  `ARCHITECTURE.md` §5). Required for signature-validity semantics.
- **Spec RAG** — the canonical PDF-standard reference corpus at
  `D:\Dev\Rag-Specialized\PDF_Spec\`, owned by `pdfce-spec-librarian`.
  Consult it before implementing any spec-governed behavior; don't
  guess byte layouts from training-data memory.
- **Feature RAG** — the Acrobat Pro feature-parity reference corpus at
  `D:\Dev\Rag-Specialized\Acrobat_Features\`, owned by
  `pdfce-acrobat-librarian`. Catalogs capability/behavior/edge-cases/
  limits per feature — never GUI mechanics. Consult it before scoping
  a Backlog bucket into a real Pass, so acceptance criteria reflect
  actual Acrobat behavior.
- **Prior art** — `docs/PRIOR_ART.md`, the survey/decision record of
  existing open-source crates and tools pdfce can depend on or learn
  from. See `docs/LEGAL.md` §6 for the binding license-classification
  and attribution rules that govern adding any dependency.
- **Fuzzy, never sneaky** — a UX principle inherited from the user's
  other projects (MatExtractor): algorithmic suggestions (OCR text,
  auto-detected form fields, suggested Bates ranges) are always
  reviewable hints, never silent auto-applies.
- **pdf dimension** — a dimension already present in the PDF, exported
  by CAD or another authoring tool: existing page content, or a
  foreign (non-pdfce) annotation. pdfce reads it and may measure
  against it, but must never silently alter it. The `55 5/8"` printed
  on a CAD drawing is a *pdf dimension*.
- **ce dimension** — a dimension object **pdfce itself authors**:
  `/Line` + `/IT /LineDimension` annotations with a baked `/AP`, plus
  their groups, scale, `/Measure` dict and `/PieceInfo` sidecar.
  Everything under `crates/pdfce-core/src/dimension/`. Authored,
  editable, deletable, re-measurable — pdfce's own.
  **Bare "dimension(s)" is banned project-wide — always qualify as
  one of these two.** The distinction is *provenance, not
  representation*: a ce dimension is still a ce dimension after
  save-and-reopen; a pdf dimension does not become a ce dimension
  because pdfce can see and render it. Binding statement:
  `CLAUDE.md` rule 15 (operator ruling, 2026-08-04, commit `89c5837`).
  When a report or request uses "dimension" unqualified, infer from
  context and **echo back the qualified term** so a mismatch surfaces
  before work starts, not after.
- **Obj-tool universality is a selection-layer property, not a
  verb-layer one** — decision 023 §5.2's reconciliation of the
  operator's "the Obj tool is for everything" instruction against
  decision 022 §4.2's anti-silent-re-measure argument, recorded here
  because it is a durable project-wide design principle, not a
  one-off Pass detail. The Obj tool selects **everything** on a page —
  content objects, annotations, ce dimensions, pdf dimensions alike —
  but it does not thereby inherit every verb for every kind it can
  select. A ce dimension's re-measure gesture stays owned by the
  Measure tool (where the operator's mental model is "I am measuring,"
  and where a two-stage disclosed old→new preview belongs); the Obj
  tool's role is to select the ce dimension and offer a **`Re-measure`
  verb that hands off** to the Measure tool with that ce dimension
  loaded, never to perform the geometry edit itself. Stated as the
  reusable rule: a tool
  that is instructed to be "universal" is universal at whichever layer
  the instruction was actually about — confirm which layer before
  concluding two requirements conflict. See decisions
  022 §4.2/023 §5.1–§5.2 for the full worked derivation.

## Shipped

### Pass 21.0 — FF-C P0 floor: font subsetting/embedding for `add-text`, `glyf`/TrueType donors only (decision 021 §§3–4, narrowed by the 2026-08-03 spec-review amendment) — **PDFCE CAN NOW ADD NON-LATIN TEXT TO A PDF** — 2026-08-04, chain `88b9487`→`0c4f490`→`d4e7355`→`5b7bed3`→`eb0bde5`→`48c6b77` (six commits, all independently `git cat-file -t` verified)

**Before this Pass, `add-text` could not write ANY character outside WinAnsi/Symbol/ZapfDingbats — no Greek, Cyrillic, CJK, Hebrew, or Devanagari, at all.** That was the single widest wall in the product (named as such at decision 021's filing, `ROADMAP.md`'s ★ Pass 21.x entry under Next up). This Pass takes it down for scripts that do not require shaping. Per R17 (no shaping, ever), Arabic/Devanagari/Thai remain out of scope — see Honest limits, below.

**Build, six commits in the order decision 021 §"Slices" required:**

- **`88b9487`** — `subsetter` (Typst; MIT OR Apache-2.0) dependency + `pdfce-core::font_embed` emitter + **the R107 object-id-disjointness test, written FIRST**, while the emitter was still trivially correct rather than deferred to 21.2 when `set-font`'s temptation to "just widen the existing font" exists. Verified non-vacuous by planting an id-reuse violation and watching it fail, naming the offending object. Deliberately a TEST, not a runtime guard: a guard inside a function that can only allocate fresh ids is unreachable by construction (R96's dead-code shape). Dependency re-verified rather than inherited (R87): one new package, `read-fonts 0.39.2` only (no `write-fonts`), `default-features = false` — `pdfce-core` gains zero new dependencies. `/Type0` + `Identity-H` turned out forced TWICE, not once: `subsetter` strips `cmap` by design, and §9.9 independently `shall`s both the CIDFont's `cmap` absence and `Identity-H` — decision 021 §3.4 understated its own case; corrected in the record here.
- **`0c4f490`** — `tools/fontfile-census` (new corpus tool). **Its negative result matters more than its numbers.** 4,023 files, 1,563 embedded programs, p50 11 KB, max 1,195,688 bytes — a 2 MiB ceiling would refuse none of them. **These numbers do NOT set FF-C's donor byte ceiling**, because ISO 32000-1 §9.9's opening paragraph (decision 021 §10 C-8, spec-review) means an existing document's `/FontFile*` is never an admissible FF-C donor — the donor is always an operator-supplied font file, and no corpus of PDFs contains those. The tool prints this caveat in its OWN output, not only in the commit message, because the number is what gets copied forward and the caveat is what stops it being copied into the wrong constant.
- **`d4e7355`** — the `pdfce-render` producer. `MAX_DONOR_BYTES = 64 MiB`; the constant's doc comment states the census result, states that it does not apply, and calls the number a JUDGEMENT, not a derivation. Errors separate *"your font"* from *"our bug"* — three `subsetter`-documented self-bug variants get a message saying so and asking to be reported, rather than letting an operator spend an afternoon proving their own font is valid. CFF donors are refused BY NAME (`DonorUnsupported`), distinct from "not a font" (`DonorNotSfnt`) — *"the font is fine, pdfce's support for this kind is not."* **A structural test-suite gap caught here:** every test was an error-path test until the fixture generator gained a standalone `.ttf` — because §9.9 rules out extracting a donor from a PDF, the happy path is UNREACHABLE without a real font file on disk, so a complete-looking refusal-test suite could have shipped beside entirely untested working code.
- **`5b7bed3`** — `add-text` widened: `base_font: Std14` → `face: NewTextFace { Std14 | Embedded }` (an enum, not an additive field — the additive alternative breaks zero call sites but admits a meaningless both-set state; the enum cost one struct literal across all 57 pre-existing references). The bug the enum shape prevents: an `Identity-H` run shows 2-byte CIDs, so reusing the literal-string content-stream builder would still RENDER — onto whichever glyphs those bytes happen to address. Wrong text that looks like right text. New `build_content_embedded` path; a test asserts the ABSENCE of `(AB) Tj`-shaped literal-string show operators on an embedded run. **A test name corrected before it could mislead:** `embedding_rewrites_no_pre_existing_object_except_the_page` was renamed to `embedded_add_allocates_at_least_six_new_objects` (an incremental save re-parses as the merged view, so the original name claimed more than its assertion proved) and now points at the `font_embed` unit test as the authoritative R107 check.
- **`eb0bde5`** — `pdfce-cli add-text --embed-font` (rule 11: extends the existing `add-text` subcommand, same reasoning as Pass 19.3/8.1 — recorded so the absence of a NEW subcommand reads as reasoned, not missed). R108 respected structurally: embedding is never inferred (not from `--font-dir`, not from the text needing it); the real computed subset size is printed before anything is written (*"456 byte(s) of font program"*, a measurement, R98 applied — never *"will add roughly N KB,"* a prediction). **Running it once found a lie no test caught:** the first successful embed still printed `base_font=Helvetica` and disclosed *"no glyph embedding (R79)"* — on a run that had JUST embedded a font. The disclosure predated FF-C, when it was the only true answer, and stayed true-looking after a second answer existed (R93 exactly — no test asserts a disclosure's TEXT against the branch that produced it; flagged below as an owed test-shape). Three more defects found the same way, same commit: a duplicated disclosure line; mangled whitespace from a `\` line-continuation in two format strings; and `EmbeddedBoxedUnsupported` exiting **1 (generic runtime error) instead of 9 (its own named exit code)** because it fell into a `_ =>` catch-all arm in the exit-code mapping — telling a calling script pdfce had CRASHED when it had DECLINED. Flagged below as a second owed rule-shaped finding (a `_ =>` arm in an exit-code map is a standing decision that every future variant is a crash, made once and never revisited).
- **`48c6b77`** — fuzz target #20 (`font_subset`) + a synthetic composite-glyph-CYCLE fixture. Targets pdfce's own glue, not `subsetter`'s internals: the donor-byte ceiling's ordering relative to parsing, the `u16` narrowing of a GID sourced from a hostile `cmap` (truncation silently selects a DIFFERENT VALID glyph — the worst outcome, because it still renders), the units-per-em division, and total error-mapping coverage. **The cycle fixture is what decision 021 specifies INSTEAD of a depth guard, and it passes.** `subsetter`'s own `closure()` walk is iteratively bounded by construction, so a depth cap in pdfce's glue would be unreachable dead code dressed as a defence (R96 shape) — the fixture asserts the termination property directly instead. **fontTools cannot even write this fixture** — building the component cycle directly with fontTools dies with a Python `RecursionError`, because its own bounds-recalculation walk is recursive. That a mature, widely-used font library takes the recursive route is the strongest evidence the fixture's non-recursive property is worth asserting rather than assuming. Built acyclic, then the component-glyph indices are patched directly in the compiled `glyf` bytes, then RE-READ from the finished bytes to confirm the cycle survived the patch (R22).

**Gates.** `cargo test --workspace` **1779 → 1790, 0 failed** (measured from a fresh checkout, not quoted). fmt clean; `clippy -D warnings` clean; `check-ui-strings.sh` clean; `check-ledger-numbers.py` clean. `cargo tree -p pdfce-core -p pdfce-render` confirmed GUI-free (R107's crate-boundary invariant). The fuzz crate gaining a `pdfce-render` dependency does NOT weaken this check — `fuzz/` sits outside the shipping workspace.

**Honest limits — do not let the non-Latin headline overstate them.** Point text only: boxed layout with an embedded face is refused BY NAME, because `layout_boxed` measures glyph advances through the Standard-14 inverse-encoding table, which an embedded CID font does not populate. TrueType (`glyf`) donors only — CFF is refused by name (spec-review C-3: `subsetter` wraps CFF in an OTTO sfnt that cannot conformantly satisfy either `FontFile3 /OpenType`'s `cmap` requirement or `/CIDFontType0C`'s bare-CFF requirement). **No shaping, ever (R17):** CJK, Cyrillic, Greek, and Hebrew-without-points work correctly; **Arabic, Devanagari, and Thai will embed and RENDER WRONG** (glyphs placed by advance, no GSUB/GPOS) — Open operator question (s) (below) still governs whether pdfce should refuse these by name.

**NOT yet implemented, despite being named in decision 021's original 21.0 slice bullet — flagged here, not silently dropped.** **R109's `fsType` donor-permission read did NOT ship in this Pass.** `add-text --embed-font` currently embeds a donor face without reading or disclosing its `OS/2` `fsType` bits — meaning pdfce can silently embed a font whose license forbids subsetting (bit 8) or forbids embedding outright (bit 9), with no refusal and no disclosure. This is a real gap against R108/R109's own design intent (rule 4, fuzzy-never-sneaky) and against decision 021's own P0 scope, not mere polish deferred to a later slice — see "Still owed," below, and the dated amendment added to R109's Standing-rules bullet (below) recording the gap. **GAP CLOSED (continuation 76, 2026-08-04, `58fe3f6`) — see R109's Standing-rules bullet (below) for the full shipped design (three refusals, ordering before subsetting, two interim non-refusal defaults flagged against Open operator question (r)).**

**Two rule-shaped findings from `eb0bde5`'s bug hunt, recorded here per the engineer's explicit request — NOT adopted as new numbered standing rules this filing (rule-adoption for the numbered ledger is not this librarian's call to make solo; see the R107–R110 precedent of librarian-assigned numbers only against an existing decision record).** (1) A disclosure/hint string needs a test that asserts its TEXT against the specific branch that produced it, not merely a test that the string exists — `has-glyph-embedded` staying `false`-shaped prose on a build that had just embedded a font is R93's exact failure mode, and no existing gate catches it. (2) An exit-code mapping's `_ =>` catch-all arm is a standing decision that every future error variant is an unhandled crash, made once at write time and never revisited as variants are added — worth an exhaustiveness discipline (parallel to the existing `non_exhaustive_no_effect_...` Rust-RAG finding) rather than a wildcard arm. Both written to `D:\dev\rag\rust\` this filing (see RAG escalations, below) as generalizable Rust findings; neither is a PDF-domain finding, so neither goes to `personal_rag/pdf`.

**Still owed from decision 021 — named explicitly so 21.0 does not read as "FF-C done":**
- **Pass 21.1 (composite-run editability under verified-injective `/ToUnicode`, R110) is NOT optional polish.** Shipping 21.0 alone means pdfce can add text (e.g. Japanese) it can never afterward edit — a capability REGRESSION against the already-shipped Std-14 add-text path. Invisible to every existing gate including the R85 raster oracle, which will show the glyphs correctly and say nothing about editability (the `flatten_fields`-failure shape: correct counters, wrong artifact). **Promoted to In progress, below** (was Next-up-only pending 21.0's ship). **UPDATE (continuation 76, 2026-08-04): substantial progress, still NOT shipped** — the R110 primitive (`injective_inverse`) landed, and the composite-run refusal that was silently unreachable in `edit.rs` is now fixed (a shipped-but-dead R-INV-4 gate, same shape as the Pass 19.4 `Tw` finding). Composite runs are now correctly LOCATED and REFUSED with the right reason — still not EDITABLE (`ShowSlot::code` widening + multi-byte operand writer remain owed). See Pass 21.1's In-progress entry and R110's Standing-rules bullet (below) for the full build record.
- **R109's `fsType` read** (see "NOT yet implemented," above) — the spec librarian has already sourced the full bit table (decision 021 §10, `ROADMAP.md` Standing rules R109 bullet), so this is an implementation gap, not a research gap. **CLOSED (continuation 76, 2026-08-04, `58fe3f6`)** — see R109's Standing-rules bullet, below, for the shipped design.
- **The `/W`, `/CIDSet`, and subset-tag clause citations** decision 021 §4.2's dispatch table now carries correctly (§9.8.3 Table 124 for `/CIDSet`, §9.6.4 for the subset-tag prefix — both fixed by the spec-review amendment, continuation 74) are available for whoever picks up 21.2's `set-font` widening.

**RAG escalations, this filing:**
- `D:\dev\rag\rust\assert_termination_property_instead_of_unreachable_depth_guard.md` — when upstream already bounds a walk structurally, assert the termination PROPERTY with a fixture that can fail, rather than adding a depth guard that becomes unreachable dead code dressed as a defence (R96 shape). Worked example: `subsetter`'s `closure()` is iteratively bounded by construction, so pdfce's composite-glyph-cycle fixture proves termination directly; a fontTools cross-check shows a MATURE library cannot even construct the adversarial input by the recursive route (`RecursionError`), which is the evidence that makes the argument concrete.
- `D:\dev\rag\rust\disclosure_text_must_be_tested_against_producing_branch.md` — a disclosure/hint string needs a test asserting its exact TEXT against the specific code branch that produced it; a test that only asserts the string's existence lets it go stale silently when a new branch is added beside it (found: `add-text --embed-font`'s `base_font=Helvetica`/"no glyph embedding (R79)" disclosure survived unchanged onto the first successful embed run).
- `D:\dev\rag\rust\exit_code_catchall_reclassifies_future_variants_as_crash.md` — an exit-code mapping's `_ =>` arm silently reclassifies every future error variant as an unhandled crash (exit 1) instead of its own named code; found via `EmbeddedBoxedUnsupported` (intended exit 9) exiting 1.
- All three indexed in `D:\dev\rag\rust\index.md` this filing. No `personal_rag/pdf` entry from this Pass's engineering findings (none are PDF-producer-divergence-from-spec; the fontfile-census corpus finding is filed to `personal_rag/pdf` separately, see its own lesson file, this same session).

### Pass 18.7 — Glyph-coverage gate + tofu-glyph fixes — a headless test checking every operator-visible character in `ui_text.rs` against the font stack the app actually runs on, plus the twelve breakages it found — 2026-08-03, committed `09be28d`

**Pass-number note (flag, per hard rule "Pass IDs are stable, never
reused"; CORRECTION filed by `pdfce-librarian`, same day):** the
implementation commit's own subject line reads **"Pass 19.4:
glyph-coverage gate; fix tofu Accept/Reject, info markers, arrows."**
That number was already taken — `a1638f4` is Pass 19.4 (`Tw` word
spacing, decision 019/FF-H, see its own Shipped entry below) and is
referenced by name in three other places in this file. This entry had
also, briefly, been filed with **no Pass ID at all** rather than a
number — also incorrect: this work has clear acceptance criteria (closes
§8.3/§4.4 of `docs/ui_specs/menu-affordance-and-glyph-coverage.md`) and
belongs in the numbered ledger like any other Pass. **Assigned here as
Pass 18.7** — the next free slot in the 18.x family, which is the
correct lineage: this work is not part of decision 019's `Tw`/spacing
family at all, it is a continuation of the 18.x UI-quality line (same
family as Pass 18.6's text-hit-target fix and the "Menu-affordance &
glyph-coverage audit" Shipped entry, above) that closes the two items
that audit left explicitly open (§8.3 `✓`/`✕`, §4.4 `ⓘ`/arrows).
**This roadmap entry is the canonical Pass-ID record; the commit
message itself is not rewritten** (history stays as committed) — same
convention already used twice before on this project for the identical
error class: Pass 18.4's commit called itself "Pass 18.2" (already
taken by the `object-list` CLI Pass, see that Shipped entry's own
Pass-number note), and decision 014's original design proposed "Pass
13.x" before discovering Pass 13a/13b were already shipped under that
number (renumbered to 14.x, see the "PASS-NUMBER RENUMBER" note under
Next up). **This is now the THIRD documented instance on this
project** — see the new standing rule R106 (Standing rules, below;
ceiling was R105, now R106 — R100–R105 were claimed the SAME real day
by decision 020's renumbering, an independent, same-shape collision on
the standing-rule ledger rather than the Pass-ID ledger) for why a
fourth instance is worth actively preventing rather than merely
correcting each time it happens. A dedicated empty correction commit,
`1111652` ("docs: correct Pass number on `09be28d` — glyph-coverage
gate is 18.7, not 19.4"), records this on the branch; a second commit,
`d9960cd`, also verified via `git cat-file -t` alongside the other
three hashes, is the SEPARATE decision-020 (form-field-authoring)
filing commit — unrelated to this Pass's own content beyond landing on
the same branch the same day, named here only because it was handed
to the librarian as part of the same hash-verification set.

**What was broken, and for how long.** `"✓ Accept"` / `"✕ Reject"` (U+2713/U+2715) and their reflow/add-text siblings had **no glyph anywhere in egui's default font chain**, and neither did U+24D8 `ⓘ` on twelve disclosure/"Now:" strings. They rendered as empty boxes on **every edit in three already-shipped features** — in-place text editing (Pass 14.x), reflow (Pass 15.x), add-text (Pass 16.x) — not once per session, once per operation. `docs/ui_specs/menu-affordance-and-glyph-coverage.md` named the check/cross glyph *"the single highest-priority item in this whole audit to verify"* and left U+24D8 as *"no basis in hand to call this safe OR broken."* **Both are now answered: broken, and fixed.** Mark those ui-spec items resolved.

**The gate.** `scan_string_literal_chars()` reads `ui_text.rs`'s own **bytes** rather than calling its functions or maintaining a hand-written glyph list — a list is a duplicated predicate that drifts (R92), and calling every `pub fn` misses whatever nobody remembers to call. Scanning the file is complete by construction, because R1 already requires every operator-visible string to live there. Comments are excluded (the file's own prose names the very glyphs it removed, which would make the gate fire on its own documentation); `\u{...}` escapes are decoded, since the escape form is *more* likely to hide a coverage bug (chosen exactly when a glyph is hard to see in an editor); raw strings **panic** rather than being silently skipped.

**★ The methodology finding, worth generalizing past this Pass.** The first implementation used the API named for exactly this job — `epaint::Fonts::has_glyph` — and it was wrong in the dangerous direction: **15 characters reported missing, including U+26A0 and U+2714, both demonstrably painted on screen today.** `has_glyph` is `resolve_face(c) != replacement_face_key` — "did resolution land anywhere other than the fallback face?" — and `CachedFamily::new` picks the fallback face by searching the chain for `◻` U+25FB, which Ubuntu-Light lacks. So **the fallback face is an emoji face**, and every symbol sharing that face reads as "missing" whether it is or not. Without a positive control (a set of characters *known* to render, checked against the same oracle) this commit would have "fixed" fifteen defects, twelve real and three imaginary, rewriting working strings on the authority of a green-looking test. The corrected oracle is `Font::glyph_width(c, size) > 0.0`, which asks the resolved face directly rather than comparing it to a second, independently-broken lookup. The gate was also verified by planting a violation and watching it fail, not only by watching it pass (the `check-ui-strings.sh` gate-verification discipline, applied to a new checker).

**Fixes.** U+2713/U+2715 → U+2714/U+2716 (heavy variants, covered by the emoji-recommended subset). U+24D8 → U+2139. Arrow glyphs in keyboard hints → **words** (`Alt+Up`) — better for screen readers regardless of glyph coverage. Menu-path separators → U+203A. The measurement readout's arrow → `=`, which is what it means at the group's own scale.

**`snap_glyph()` DELETED rather than repaired.** Zero call sites, carrying `#[allow(dead_code, reason = "drawn by the Pass 12.M2 measure tools' overlay")]` — what actually draws the overlay is vector art in `canvas.rs`. **Seven of its eight marks were uncovered by the fixed oracle** — so had anyone trusted that `reason` comment and wired the function up, seven of eight snap candidates would have shown a box, *with an inline comment vouching for the display*. R93's exact shape. Its live sibling `snap_indicator_label()` loses its now-false `allow`.

**The second defect, found only by looking (R86).** The screenshot taken to confirm the glyph fix showed **"Place point" rendered as four stacked fragments — `Pla`/`ce`/`poi`/`nt` — in a column the width of a scrollbar.** Six word-labelled buttons used `add_sized(ICON_BUTTON_SIZE, ..)`, which allocates *exactly* 28 pt and forces egui to wrap the label per character once it overflows. **Two of the six were "Accept reflow" and "Reject reflow"** — so fixing the glyph alone would have shipped a readable check mark on an unreadable button. Fixed by switching to `.min_size(ICON_BUTTON_SIZE)` — the accessibility floor without the layout cap. `ICON_BUTTON_SIZE`'s doc comment now states which of the two to use and why. **Stated crisply because it generalizes: a test can prove a character has a glyph; only looking proves the operator can read the button.** Considered filing this as a new standing rule; decided against it — it sharpens R86's existing rationale rather than adding new behavior, so no new rule number is assigned for it.

**Gates.** `cargo test --workspace` **1768 → 1770, 0 failed**; fmt clean; `clippy -D warnings` clean; `check-ui-strings.sh` clean; `cargo tree -p pdfce-core` / `cargo tree -p pdfce-render` confirmed GUI-free. Observed in the running **release** build across five captures: U+26A0 in the status bar, `✔ Add` / `✖ Cancel` with real check and cross glyphs, **four U+2139 spacing hints where boxes used to be**, and "Place point" rendered on one line. The capture guard refused a uniform frame once before a real click woke eframe — the documented blank-until-first-input behavior, not a failure.

**RAG escalations (filed by `pdfce-librarian`, this filing — the names below are canonical; an earlier draft of this entry cited two of these three under different filenames before any file existed at those paths, corrected here rather than left dangling):** `D:\dev\rag\egui\epaint_035_has_glyph_false_positive_via_replacement_face_fallback.md` (the `has_glyph` false-positive finding); `D:\dev\rag\egui\egui_add_sized_is_a_layout_cap_not_an_accessibility_floor.md` (the `add_sized` per-character-wrap finding); a dated 2026-08-03 footer added to the ALREADY-EXISTING `D:\dev\rag\rust\ci_gate_red_at_baseline_enforces_nothing.md` (a checker needs a positive control, not only a negative one — this Pass's gate needed the positive-control half specifically). All three indexed in their subject's `index.md` this filing. None reached `C:\personal_rag\pdf\` — this is an egui/rendering finding, not a PDF-domain one.

### Pass 8.1 — GUI redaction-apply flow (the GUI half of Pass 8.0's redaction feature; decision 018's live-edit-rendering framing; `docs/ui_specs/pass-8-redaction.md` §§3–4) — **THE HALF-SHIPPED SECURITY FEATURE IS NOW WHOLE** — 2026-08-03, committed `9a68999`

**Before this Pass, `grep -c "apply_redactions\|RedactApply" crates/pdfce-gui/src/main.rs` returned 0.** pdfce could mark redactions in the GUI and disclose the mark count (the status bar warned *"⚠ N UNAPPLIED redaction mark(s) — this document is NOT redacted"*), but applying — the operation that actually removes covered content — was CLI-only (`pdfce-cli redact-apply`). The app named the danger and withheld the remedy. This Pass closes that gap end-to-end: mark, review, apply, and a runtime-verified confirmation, all reachable from the running GUI.

**Build.** New `crates/pdfce-gui/src/redact_apply.rs` (640 lines) — the whole pipeline as a **free function over `&EditSession`**, deliberately, so the security assertion can be a TEST rather than a manual inspection (see the standing-rule-adjacent finding below). New `DockPanel::Redact`, reached through the same `panel_body` dispatcher as every other dock surface (R80). `Icon::Redact` — the icon set's only solid-filled glyph (per its own documented §8.1 exception, the one place an outline icon would misrepresent what the operation does) — un-reserved and wired to the new panel. Core gained `RedactionMark`/`redaction_marks()`; `count_redaction_marks` now delegates to it, so the status-bar count and the panel's list walk the SAME data — a "3 marks" banner beside a 2-row list is now structurally impossible, not merely untested-for. Core also gained `EditSession::delete_redaction_mark` + `CommandKind::DeleteRedactionMark` — needed because bulk mark-by-search must stay reviewable (rule 4): undo cannot reject the 3rd of 40 marks without discarding the 38 good ones. Deliberately NOT a general `delete_annotation` — it refuses any non-`/Redact` subtype by construction, so it cannot become a back door for deleting widgets or reply chains.

**Gates:** `cargo test --workspace` **1756 → 1768, 0 failed** (baseline measured from a fresh checkout, not quoted from a prior filing); fmt/clippy clean; `check-ui-strings.sh` clean; `cargo tree -p pdfce-core` / `cargo tree -p pdfce-render` confirmed free of GUI dependencies; **R85 21/21**.

**The design decision worth recording above the feature itself: there is no incremental-save fallback because the code path does not exist to be taken — an absence, not a check that could be bypassed.** An incremental save would leave the un-redacted content sitting in the file's previous revision, inside a file the operator has just been told is redacted. Engineer-verified: the only two occurrences of `to_incremental_bytes` anywhere in `redact_apply.rs` are comments EXPLAINING the absence; a precise grep for a call to it returns nothing. (The librarian's own first grep of this claim was too coarse and appeared to contradict the builder — re-run precisely, the claim held. Recorded because the correction is itself the lesson: verify a "this can't happen" claim by grepping for the call, not for the word.)

**The security proof proves ABSENCE, not invisibility — this is the finding worth generalizing.** `applied_redaction_leaves_no_recoverable_trace_in_the_saved_bytes` drives the EXACT GUI pipeline and asserts three independent absences of the secret: (1) `text_extract::extract_document` on the saved bytes — the same extractor the CLI and Copy-text use; (2) **every** stream in the file, decoded — not just page content, but form XObjects, metadata, and object-stream containers, so a stale compressed copy is caught; (3) the raw file bytes. Plus a **negative control** (`KEEPTHIS`, never marked, must still extract) so a build that emits a blank page would FAIL the test rather than pass it vacuously. **Deliberately no raster assertion** — a black box drawn over live text is precisely the §12.5.6.23 false-redaction failure mode, so "the region looks blank" is the wrong question to ask.

**The same proof runs at RUNTIME on the real output, before the confirmation dialog even opens** — that is what licenses the word "verified" in the confirmation UI. A survivor found in a decoded stream refuses the apply and writes nothing; a survivor found in raw bytes only is disclosed as an acknowledgement-gated residual, worded to claim only what pdfce actually knows (*"It may be an unrelated coincidence, or a copy in a carrier pdfce does not recognise — pdfce cannot tell which"*). Strings under 4 characters are excluded from the raw-byte half of the check and **counted in the report** rather than silently dropped.

**Two defects found only by looking (R86), fixed in the same commit:**
- The ui-spec's `max_height(240.0)` marks list pushed **"Review & Apply Redactions…" below the fold** in a ~250 pt dock pane — a panel that shows the marks but hides the way to finish them is an active nudge toward "marking is redacting," the exact misconception the status-bar warning exists to prevent. Reordered to **state → action → detail** (mark count and warning first, Review & Apply next, the scrollable mark list last).
- The confirmation report attributed the ENTIRE `annotations_removed` count to *overlapping* annotations; core counts the marks themselves plus true overlaps together and does not separate them. On the fixture this read "3 annotations overlapping a marked region will also be removed" when all three WERE the marks. Reworded to state an accurate total instead of a false attribution.

All four GUI states (marks-present-not-applied, review dialog open, applying, applied-and-confirmed) were observed on a running build via `tools/observe-gui.ps1`/`gui-click.ps1` with `-ProcessId`; the blank-capture guard refused a uniform frame twice before a real click woke eframe, exactly as it is documented to. Applied output independently confirmed post-hoc: `list-redactions` reports 0, a raw grep for the secret returns 0, no `/Prev` chain, `/Info` `/Title` scrubbed.

**Where the spec no longer fits current reality — recorded, not silently deviated from.** Six items: §3.1's dedicated `SidePanel` design is now an R80 violation on paper (the shipped build correctly used `DockPanel::Redact` instead, mounted in the upper tab group — A.3 caps a default tab group at two labels and `egui_tiles` hides overflow behind scroll arrows, now enforced by a new test `no_default_tab_group_holds_more_than_two_panes`); §3.1's icon-only button design is superseded by the shipped icon set; **§4.3's permanence wording was factually WRONG for this build** ("cannot be undone once you save this" assumes apply mutates the open document — it does not; apply writes a NEW file and leaves the open session untouched); §4.3/§7 assumed a *predicted* report, but because `apply_redactions` is a pure function the apply now runs BEFORE the modal opens and the report states MEASUREMENTS — strictly better, and it changes §5.1's calculus (see R98, Standing rules); §4.4's `could_not_remove` field does not exist in core — derived instead in one `residual_lines()` function so the pass/fail gate and the printed report section cannot drift apart; §3.2's `✕` glyph was replaced with the word "Remove," citing the project's already-shipped tofu finding on that glyph.

**Not built — scope-called and named, not silently dropped.** §2.2/§2.6 canvas drag-marking and its transient property bar (the §1.1 canvas-substrate dependency this needed has since landed, so this is a scope call, not a block — filed as a named follow-up under Backlog, recommended as a `CanvasTool::Redact` variant rather than a parallel drag implementation); §6 Sanitize (filed under Backlog's Redaction bucket, unchanged).

**Rule 11 does not apply.** `redact-mark`/`redact-apply`/`list-redactions` shipped as CLI in Pass 8.0. This Pass adds no new CLI surface because the CLI already had the whole feature — this Pass was the missing GUI half.

**Standing-rule-adjacent facts, three of which are now filed as new standing rules (R97–R99, below; ceiling was R96, now R99):** the free-function-over-data-so-the-proof-can-be-a-test pattern (R97); the apply-computes-before-confirming pattern that turns a predicted report into a measured one (R98); the ~250 pt dock-pane-height finding that a panel's primary action must precede its detail list (R99). Also recorded, not promoted to a numbered rule: the third confirmation-dialog convention (resizable 760×560, scrollable body) as the shared shape for any future report-bearing destructive confirmation; A.3's two-panes-per-default-group cap is now test-enforced.

### Pass 19.4 — `Tw` (word spacing) direct-authoring control (core + CLI + GUI; decision 019, closes FF-H — **decision 019 / FF-H COMPLETE end-to-end, all five slices 19.0–19.4 shipped**) — 2026-08-03, committed `a1638f4`

**MILESTONE: decision 019 / FF-H is COMPLETE, and with it the operator's
priority-#3 item ("finish off all the text handling stuff") is DONE as
far as FF-H's own scope goes** — FF-C (font subsetting/embedding) and
FF-B (cross-block/cross-page reflow) remain unscheduled, per this
decision's own Q3 build order (FF-H → FF-C → FF-B), unaffected by this
milestone beyond clearing FF-H's own slot. See the ★★★ Operator priority
sequence entry (Next up) for the updated status.

`Tw` rides the existing `push_state_param` four-rung restore ladder and
the `pre | set_ops | mid | restore_ops | post` splice — no new authoring
path. `FormatRequest::set_word_spacing` shares `Tc`'s
`MetricSpec::{Absolute, Relative}` model, resolved against the BASE
font size per Amendment B item B.3. `FormatError::WordSpacingComposite`.
`FormatReport::word_spacing_change` + `word_spacing_affected_codes`.
`Tw` enters the §9.4.4 advance via `eff_tw` and joins the existing
justify-invalidation trigger set (Pass 19.1's
`disclosure_justify_invalidated`, not a second path). CLI
`--word-spacing V[pt|em]`; `parse_char_spacing` generalized into
`parse_text_metric` so `Tc`/`Tw`/`Ts` share one grammar and one error
voice. GUI row live for simple-font runs; the composite strip stays the
existing read-only R83 presentation.

**Gates:** `cargo test --workspace` 1738 → 1756, 0 failed; fmt/clippy
clean; `check-ui-strings.sh` exit 0; `cargo tree` clean; **zero new
Cargo dependencies**; R85 (preview-equals-saved oracle) 21/21; the
pre-existing `reflow_apply` justify tripwire (Pass 19.0) still passes,
and that file was untouched by this slice. The 1738 baseline was
**measured, not quoted** — built from a `git archive HEAD` export and
run, not assumed from the previous filing. Round-trip proven
non-vacuous by two binaries differing in both MD5 **and size**
(3,396,096 vs 3,394,048 bytes), with identical decoded output.

**★★★★ THE SHARPEST FINDING — a standing rule that would have compiled,
read correctly, and NEVER FIRED.** The composite-run refusal (R91,
§9.3.3's structural void for `Tw` on 2-byte CID runs) was **unreachable
as `plan_format` was originally ordered.** `Walk::record_show` does not
decode a composite run's string, so `ShowData::text` is empty for every
composite run, so `match_run` returns `NoMatch` on every composite run
**before any font-aware gate can speak.** Left alone, R91 would have
existed in code, been referenced in three documents (this decision's
text, `ARCHITECTURE.md` §5.11, and `ROADMAP.md`'s own R91 wording), and
never once executed — an untestable branch claiming to honour a
standing rule. **Fixed by hoisting font resolution above `match_run`.**
Two new tests prove it: one proves the gate now fires, a second
(`the_composite_gate_fires_only_for_word_spacing`) proves the OTHER
three controls stay live on the same composite run — establishing this
is a specific capability gate, not "composite disables the whole
panel." **Neither decision 019 nor Amendments A–E anticipated this** —
§3.3 and R91 both described the gate as if the composite run were
addressable. Filed as decision 019 **Amendment F**; generalized to
`D:\dev\rag\rust\dead_guard_clause_behind_a_filter_the_guarded_case_cannot_pass.md`
(a guard clause placed after a filter the guarded case cannot pass is
dead code that looks live — detect it by writing the test that asserts
the guard FIRES, not merely that the happy path works, the same
"prove it by making it fail" discipline as Pass 19.2's mutation
testing).

**A named limit that came with it, recorded not papered over.** The
fixed R91 refusal is reachable through the **pinned-span** path (the
GUI's, and the core tests') but **not** through CLI `--find`:
searching for text inside a composite run finds nothing, so
`format-text --find` returns *"text to format … was not found in an
editable run on the page"* — not a silent no-op, not a false success,
but a **less specific refusal than the decision describes.** Closing
it needs composite decoding in the authoring walk (FF-E's scope, not
this slice's). Recorded as a named limit in decision 019 Amendment F.

**Three more findings, all filed to Amendment F:**
- **Amendment A.1's fourth restore rung was written for `TD`/`"` in the
  abstract; `Tw` is its concrete headline.** `"` sets `Tw` AND `Tc`
  while showing a string (§9.4.3 Tables 108/109), so `Tw` is the
  parameter where replaying a producer's own bytes for restore
  actually repaints text. A new test,
  `word_spacing_rung_three_indirect_ambient_is_respelled_not_replayed`,
  asserts `(lead) "` appears exactly once in the appended revision.
  **The rung needed no code change to handle this correctly — worth
  recording that the abstract design was right**, not only recording
  amendments that correct something.
- **The `Th` coupling needed a disclosure the decision never asked
  for.** §9.4.4 multiplies `Tw` by `Th` on the same basis as `Tc`, so
  `--word-spacing 2 --h-scale 50` delivers a **1-unit** gap, not 2.
  Decision 019 mentions the multiplicative interaction only as a
  *reason `Tw` is awkward to expose as a control*, never as something
  needing disclosure. The disclosure now quotes the effective
  delivered figure whenever `Th ≠ 1`. Filed to
  `C:\personal_rag\pdf\lesson_20260803_word_spacing_multiplied_by_horizontal_scaling.md`.
- **`Some(0)` affected spaces is a real answer, stated as one.** A
  `Tw` set on a space-free run is genuine state with no visible
  effect; pdfce emits it, restores it, and discloses `0` explicitly —
  rather than suppressing the operation as a no-op, which would be a
  silent no-op in the one slice whose entire point is that `Tw`'s
  effect is conditional on content the operator does not directly see.
- **Amendment E's falsification held under implementation, and the
  "growing" caveat was honoured.** Nothing in code, GUI disclosure
  copy, or CLI help text added this slice asserts a trend in composite
  adoption in either direction — checked against Amendment E's own
  caveat that the "growing" half of §3.2 reason 2 is untestable on its
  corpus.

**R86 (operator-visible-definition-of-done) observed with `-ProcessId`**
on a purpose-built fixture carrying one simple and one Type0/Identity-H
run. Live case: property strip read *"Now: 0.0‰ of size (0.0000 pt) —
the PDF default, never set on this run,"* dragged to 57.0‰, applied;
the canvas visibly widened the gaps, and the strip carried `Tw 0 ->
0.912` and *"It applies to ALL 3 spaces inside the formatted run."*
Refused case: the row collapsed to grey read-only with the §9.3.3
explanation — no spinner, no unit toggle, no Apply (R83). The capture
guard fired twice on uniform frames during this observation and the
builder sent real clicks until it passed rather than defeating the
guard.

### `/Contents`-defect fix — a dangling `/Contents` array element no longer condemns the whole document; 289 previously-unopenable documents now read (no Pass ID — a correctness fix, per the ★ pdfce defect In-progress entry's own framing, RESOLVED) — 2026-08-03, committed `409a6b5`

**289 previously-unopenable documents now read**, engineer-verified by
independently re-running `tools/tw-census` over the same 4,023-file
corpus:

| | before | after |
|---|---|---|
| text-bearing documents | 1,224 | **1,513** |
| page-tree load failures | 497 | **163** |
| parse failures (strict path) | 130 | **130** (untouched — confirms the fix did not loosen normal parsing) |

`BadContents` went **341 → 1**. Zero regressions — no file that opened
before now fails. The 341 resolved as: 289 measured (above), 45
no-text, 6 a *different* pre-existing page-tree defect that
`BadContents` had been masking (5 cycles, 1 missing `/Resources`), 1
still correctly `BadContents`. Newly-opening files carry 32,729 glyphs
between them, none zero-glyph. `fixtures/external/qpdf/qpdf/qtest/
qpdf/add-contents.pdf` — the file hand-verified last continuation as
legal-but-refused — now extracts `Baked / Potato / Mashed`.

**The diagnosis filed last continuation was WRONG IN MECHANISM, not
just incomplete — recording the correction, not merely the fix.** The
prior filing reported that rebuild-by-scan recovery *misses an
object*. It does not: the scan correctly proposes all 8 `N G obj`
headers; object 5 is dropped at the **confirmation** step, which
parses each candidate strictly and fails with "endstream not found
where `/Length` points." The real cause: `add-contents.pdf` is an
**LF file that was converted to CRLF**. Every `/Length` was measured
on the LF form, so each stream is now one byte longer per internal
line than declared, and the declared extent lands mid-content. **One
damage event, two symptoms** — the same CRLF shift is why `startxref`
misses `xref` in the first place (the reason recovery engages at all),
and the second symptom silently ate the content stream that the
first symptom's recovery existed to save. No off-by-one, no
terminator bug: the scan was correct, the strictness applied to its
output was not.

**The inferred SHAPE was also wrong.** Last continuation described an
array of references; ~300 of the 341 are actually a **single**
indirect `/Contents N 0 R` resolving to null, and only ~41 are the
array form. Per dangling element, classified this continuation rather
than generalized from the one hand-verified file: 340
`StreamExtentMismatch`, 12 `BadStreamLength`, 3 lexical, 2 missing
`endobj`, 4 genuinely absent, 1 resolving to a dictionary. **337 of 341
had the missing object's header physically present**, dropped only at
confirmation — the scan itself was never the problem.

**Two fixes, kept deliberately separate:**
1. **`StreamLengthPolicy`** — an explicit opt-in parser policy.
   `Strict` (default) is unchanged. `RecoverFromEndstream` re-derives
   a stream's extent from the `endstream` keyword — **not a
   heuristic landmark**: ISO 32000-1 §7.3.8.2 *defines* `/Length` as
   the byte count "to the last byte just before the keyword
   `endstream`," so the keyword is the other half of the same
   normative statement. Reachable only from the existing recovery
   paths; both must agree, or an object one path accepts the other
   would reject.
2. **`/Contents` degradation, per element.** A reference resolving to
   null contributes nothing (§7.3.10 dangling-reference rule + Table
   30's "if absent, the page shall be empty" — degrade that one
   element, not the document). A *type* error (a number, a dict, a
   non-reference array element) is still `BadContents` — unchanged. A
   *direct* `null` (not an unresolved reference) is treated as absent
   per §7.3.9 and deliberately **not counted** in the
   `contents_unresolved` tally, which is reserved for content that
   should have been present and was not.

**Disclosure — counted, through existing channels, never silent.**
`RecoveryReport.stream_lengths_recovered` → CLI
`stream-lengths-recovered=N` + note, GUI recovery banner.
`Page.contents_unresolved` → `render::Diagnostics.
contents_streams_unresolved` (CLI `contents_unresolved=N` on the
stable output line, joins the GUI "unsupported items" headline and
leads its detail list) and `TextDiagnostics.contents_unresolved`.

**★★★★ THE ROUND-TRIP GATE CAUGHT A BUG IN THE FIX ITSELF.** The first
attempt repaired the recovered object's `data_span` but left its
**stale `/Length` beside it unmodified**. Because the writer re-emits
`Provenance::File` objects verbatim, `save_full` produced a file
**pdfce itself could not reload** — a self-inflicted violation of
§5.10's own round-trip contract, caught by the gate that contract
exists to enforce, before it left the worktree. Resolved with a third
`Provenance::RecoveredFile` variant on the already-`#[non_exhaustive]`
`Provenance` enum, meaning "bytes exist but contradict the value" —
the serializer re-serializes those objects and always recomputes
`/Length`, rather than copying stale bytes forward. **§5.10 is not
weakened by this**: pdfce genuinely did touch that object's length,
deliberately, and it is disclosed via the same `RecoveryReport`
channel as every other recovery action. The two existing sites that
assert verbatim-passthrough already skipped non-`File` provenance via
`let-else`, so both were correct by construction against the new
variant; their comments now say so explicitly rather than leaving it
implicit.

Round-trip verified **non-vacuously** — the pre-change harness was
built from `git archive HEAD` (pre-fix), confirmed to contain zero
occurrences of `StreamLengthPolicy`, and produced a binary with a
different hash from the post-fix build. Every §5 round-trip metric is
identical pre/post change; the raster oracle went **174 → 178
compared, all identical**. On `xref-recover/` alone: pre-change
compares **0/0** rasters (nothing in that fixture set loaded far
enough to compare), post-change **4/4**.

**Gates:** `cargo test --workspace` **1722 → 1738, 0 failed**; fmt/
clippy clean; `tools/check-ui-strings.sh` exit 0; `cargo tree -p
pdfce-core -p pdfce-render` clean (no GUI dependency); **zero new
Cargo dependencies** (`Cargo.lock` and every manifest untouched). New
fixtures `xref-recover/{crlf-shifted-lengths,dangling-contents,
dangling-contents-array}.pdf`, generator + `PROVENANCE` updated; the 7
pre-existing fixtures in that set regenerate byte-identically.

**Two follow-ups flagged, not built:** 9 `load-failed` corpus files
hit `StreamExtentMismatch` on the **strict** (default) parse path and
are correctly untouched by this fix — a `--repair` opt-in flag could
reach them, not scoped here. The +5 page-tree cycle failures uncovered
by `BadContents` no longer masking them are **pre-existing defects
newly exposed**, not new breakage introduced by this fix.

**Chain-completeness correction filed the same continuation, committed
`0395177`:** the audit that runs before each of these filings is
committed found `fb97abb` (the continuation-66 filing commit,
recording Pass 19.3 and standing rule R93) referenced nowhere in
`docs/`. This is the **second** time the missing commit has been a
*filing* commit rather than a code one (`7274fdd` was the first, per
R87's own note) — a structural blind spot, not bad luck: a
continuation records the commits it is filing ABOUT, and the commit
that lands the filing has no later entry to mention it. The audit
catches it only because it compares against `git rev-list`, not
against the previous entry. **Standing rule R87 amended** with this
finding — see Standing rules, below.

**New standing rules R94–R95 added** (see Standing rules, below): R94
generalizes the `Provenance::RecoveredFile` fix (a repair that mutates
a value must invalidate any "these bytes are verbatim" assertion
attached to it, or a downstream verbatim-copy path re-emits stale
bytes beside a corrected value); R95 states the `/Contents`
per-element-degrade rule as binding, not merely this fix's rationale.

**Branch `pass-8-redaction`, 56 commits** (`git rev-list --count
HEAD`), hashes `0395177`/`409a6b5` engineer-verified via `git cat-file
-t` per R87; still no git remote configured.

**RAG escalations this continuation:**
`C:\personal_rag\pdf\lesson_20260803_crlf_conversion_invalidates_every_length.md`
— a CRLF-converted PDF invalidates every `/Length` in one damage
event, producing two independent symptoms (broken `startxref` AND
overrunning stream extents), where recovering from the first still
leaves the second silently eating content; the remedy is normative,
not heuristic (§7.3.8.2 defines `/Length` in terms of the `endstream`
keyword). And
`D:\dev\rag\rust\repair_that_mutates_a_value_must_invalidate_verbatim_provenance.md`
— the general shape of the round-trip-gate-catches-itself bug, for any
system carrying a "these bytes are original, copy them verbatim" flag
alongside a value a repair path can mutate. Both indexed in their
subject's `index.md` this same continuation.

### Pass 19.3 — GUI: the spacing/style property surface, AND a project-wide correctness fix — the shipped Edit-Text property bar had never applied a single edit (decision 019, closes the FF-H formatting-slice family) — 2026-08-03, committed `74052d3`

**★★★★ HEADLINE FINDING — every property-bar "Apply" in the shipped GUI, from Pass 14.3 through Pass 19.2, failed silently with `NoMatch` before reaching the surgery.** `GlyphProvenance::operator_span` publishes the span of the operator token ALONE (`Tj`, e.g. `37..39`); the authoring walk's `OpRec` records the OPERAND-INCLUSIVE extent (`(hello) Tj`, `23..39`). `find_anchor`'s pinned-request path compared the two for EXACT EQUALITY. The GUI pins every formatting request from provenance, so **every Pass 14.3/15.x/19.1/19.2 property-bar Apply on an ordinary one-`Tj` page refused** with "text to format was not found in an editable run on the page" — confirmed live in the running application before the fix. **It survived because two doc comments, on both the publisher and the consumer, independently asserted the two conventions already agreed** — `EditRequest::pinned_span`'s "matches the same span" and `page.rs:518`'s "the surgery locates the operator by exactly this span" (both present before the fix, both corrected after) — so nothing prompted a check. It was found only because this Pass stopped discarding failed pin queries with `.ok()`; rendering them as visible errors is what exposed a bug that had survived two prior GUI-observation rounds unnoticed.

Fix, in `pin_names_operator`: accept EITHER convention — `pin.end() == r.end && pin.start >= r.start` — since two operations sharing one content stream cannot share an end offset. Neither publisher's span semantics changed; both wrong doc comments were corrected in place. Two new regression tests, one of which proves the pin still DISCRIMINATES (a near-miss span is still refused) — the fix traded a false refusal for a correct match, not for the strictly worse failure of silently editing the wrong run. **Engineer-verified by mutation:** reverting `pin_names_operator` to the shipped exact-equality behavior makes `a_pin_taken_from_provenance_locates_the_same_operator` FAIL; restoring it passes. The defect was proven, not inferred.

**Third instance this project of a confident, wrong doc comment being the reason nobody looked** — after decision 018's `refresh_pages` ("the base revision has not changed," true through Pass 3.1, false since Pass 6.1) and the `.gitattributes` ordering incident (the file's own `*.pdf binary` rule was silently overridden by a catch-all placed below it). Recorded as new standing rule **R93** (below) — the pattern is now load-bearing evidence of a project-wide failure mode, not a one-off worth noting only in an individual Pass entry.

**The slice itself.** Option-B wrapper in `pdfce-core`: `StyleOutcome`, `StyleResolution`, `probe_synthesis`, `preview_style_resolution` — read-only, side-effect-free, and calls `gate_synthesis` up to three times rather than re-deriving anything, so the preview cannot say something the commit path would not do (a byte-equality test proves a commit made after previewing is identical to one made without previewing first). Exposed as an `EditSession` method — reads session-current content, not a free function over a bare `Document`. GUI: `MetricUnit`, `BaselineChoice`, `AmbientSnapshot`, 11 new `TextEditState` fields, five `FormatOp` variants, a `CollapsingHeader` property tree, five refusal hints, ~45 new `ui_text.rs` entries.

**Design answers, per `pdfce-ui-specialist`'s `docs/ui_specs/pass-19.3-text-formatting-surface.md` (committed `e883e26`).** Placement answered by PRECEDENT — the TextEdit tool already has a floating tool-scoped property bar, and AddText and Measure independently converged on the identical pattern. Mutual exclusion is structural twice over: one four-way selector with a single live member, plus the free-rise field HIDDEN rather than disabled (R83 — a greyed spinner is still an affordance); `FormatOp` has no way to *spell* both at once. Absolute vs. relative is labelled by behavior (`scales with size (‰)` / `fixed (pt)`); switching RE-DERIVES from ambient so the displayed number always means what its visible unit says. Word spacing: no control at all, a greyed value plus a composite-aware reason (pending-census for simple fonts, the permanent §9.3.3 void for composites). **The ui-spec's own §3 and §3.1 contradicted each other on unit-switching behavior** (re-derive from ambient vs. convert the typed value) — the builder followed §3.1, since re-deriving from ambient would silently discard what the operator had just typed; recorded as a spec-document self-contradiction found and resolved, not a design choice made freely. `StyleResolution` had to be a per-axis verdict, not a single one, to model the mixed real-bold/synthetic-italic case. `preview_style_resolution` cannot model a PENDING family change without re-running font re-encoding every frame — documented as a named limit on the function itself. **The spec also documents a design reversal with its reasoning kept in the written record**: the specialist first assumed the synthesis offer needed a declinable-confirmation widget, then found by reading `gate_synthesis` that the shipped refusal strip already IS an honest, non-modal, declinable offer — and left the abandoned assumption in the spec rather than silently deleting it. Rule 11 (CLI parity) does not apply here — the CLI shipped with 19.1/19.2 — stated explicitly so the omission reads as reasoned, not missed.

**Gates.** `cargo test --workspace` 1708 → 1722, 0 failed; fmt/clippy clean; `tools/check-ui-strings.sh` exit 0; `cargo tree -p pdfce-core -p pdfce-render` clean (no GUI dependency); zero new Cargo dependencies; **R85 20/20** (preview-equals-saved). **R86 observed**, against a purpose-built non-default fixture (a default-valued document would make a correctly-seeded panel indistinguishable from a blindly-seeded one — generated in scratch, never committed): ambient seeded as `Now: 31.2‰ of size (0.7500 pt)` / `Now: 92.0%` / `Now: raised 2.5000 pt`, none defaulted; synthesis pre-resolution naming the real Bold resource before submission; the mixed-case refusal explaining which axis has a real face and what two-step path to take; R84's bold-on-selected pairing rendering correctly.

**Owed, not built:** partial-axis synthesis (a real face for the covered axis plus synthesis for the uncovered one); `Ctrl+B`/`Ctrl+I` accelerators; a persistent "synthesized" badge.

**Correction to the builder's own report, engineer-verified by observation.** The builder had reported that both `ⓘ` and `⚠` render as tofu. **`⚠` (U+26A0) does NOT** — captured at 4× magnification as a proper warning-triangle glyph, consistent with an earlier 3× observation; the two codepoints were conflated in the original report. `ⓘ` (U+24D8, CIRCLED LATIN SMALL LETTER I — used 12×, the single most-used symbol in `ui_text.rs`, Enclosed Alphanumerics block, not emoji-recommended) is PLAUSIBLY tofu but remains UNVERIFIED — no reachable UI state displayed it this session. Filed accordingly: a font-coverage pass should target U+24D8 specifically, not `⚠`. Usage tally for the record: U+24D8 ×12, U+26A0 ×10, U+2715 ×4, U+2714/U+2716/U+2713 ×3 each.

**Carry forward, unchanged:** operator questions (g)–(k); no GUI redaction-apply flow (R85-uncoverable by design, not an oracle gap); `✓`/`✕` (U+2713/U+2715) glyph verification still owed; status-bar/fit-zoom feedback loop; letter badges pending real icons; `egui_kittest` harness gap. Branch still `pass-8-redaction`, now spanning Passes 9–19.3. Fresh-checkout integrity re-verified this session at 49 commits (1708 tests green, fixtures byte-identical) — the `.gitattributes` ordering fix (above) is holding under accumulation. Backup bundle `pdfce-20260803-1400.bundle` now two commits stale.

**RAG escalation.** The pinned-span-convention defect generalizes beyond PDF/pdfce — filed to `D:\dev\rag\rust\` (see this session's RAG-escalation note in `SESSION_LOG.md`), not `personal_rag/pdf`: the lesson is about byte-span-provenance API design for ANY editor that publishes spans for later re-location, not about PDF-domain producer behavior.

### Pass 19.2 — Free-form `Ts` + synthetic bold/italic, the deliberate exceed (decision 019, extends the Pass 19.1 formatting slice) — 2026-08-03, committed `ebe35d8`

**New `crates/pdfce-core/src/text_edit/synth.rs` — one shared policy type.**
`StyleSynthesis` (provenance/decision type), `SynthesisPath` (the *only*
asymmetry between Add-Text and in-place edit is remedy *order*, per decision
019 §3.6), `SynthesisOffer` (one wording, both paths), `OBLIQUE_TAN`/
`BOLD_STROKE_RATIO` constants, `shear_into` (a genuine matrix
premultiplication — its test uses a pre-rotated matrix, where a naive
`tm[2] = tan` overwrite loses the lean entirely), `matrix_scale`
(determinant-based, so a shear does not perturb the derived bold stroke
width), and `detect`, the reload-time re-detector (pdfce's own synthesis and
other producers'). CLI gained `--rise`, `--bold-synthetic`,
`--italic-synthetic` (rule 11, CLI parity, shipped in-Pass).

**Gates:** `cargo test --workspace` 1663 → 1708, 0 failed; fmt/clippy clean;
`bash tools/check-ui-strings.sh` exit 0; `cargo tree -p pdfce-core`/`-p
pdfce-render` GUI-dep-free; **zero new Cargo dependencies**; **R85
(preview-equals-saved) 20/20** (four new cases: rise, synthetic bold,
synthetic italic, bold+italic+rise combined); roundtrip byte-identical over
`fixtures/synthetic`, proven **non-vacuous** by `md5sum`-distinct harness
binaries built from a genuine `git archive` export of the base commit
(`49d285ad…` vs. `9ccb6e0c…`, differing sizes — not a repeat of the Pass
19.0 vacuous-stash mistake).

**THE RENDER-HONOURS-MODE-2/SHEARED-`Tm` PREREQUISITE WAS CONFIRMED BY
MUTATION TESTING, not by inspection — this is the standard the by-reading
check should have met.** New `crates/pdfce-render/tests/
synthetic_style_render.rs` builds PDFs, rasterizes through the shipped
`render_page`, and interrogates pixels. All 5 tests passed on the first run
— so the builder then **deliberately broke the renderer three separate ways
and re-ran** to prove the pass was not vacuous: dropping mode 2 from
`strokes()` failed 2 tests; building `Tm` with `c = 0.0` (i.e. no shear)
failed 2; zeroing the rise failed 1. With the renderer intact, the tests
additionally established real facts about pdfce's own rendering, not just
that it compiles: `2 Tr` + `2 w` paints strictly more ink than a plain fill,
with **more than 20 of the new pixels OUTSIDE the filled glyph silhouette**
(a true outline, not merely a darker fill); `1 0 0 RG` on black-filled text
produces genuinely red pixels, proving §9.3.6's stroking-colour rule is
**both implemented and load-bearing** (the "coloured text acquires black
outlines" hazard named in decision 019 §3.6 is real, not hypothetical, and
is closed); a sheared `Tm` moves the glyph TOP rightward by more than 3 px
while the baseline row moves less than half that amount, with the vertical
ink band unchanged (a shear, not a translation).

**ENGINEER-VERIFIED emitted content stream** (blue text, bold + italic +
rise-5 combined):

```
BT /F1 12 Tf 0 0 1 rg 72 700 Td 5 Ts 2 Tr 0.264 w 0 0 1 RG 1 0 0.212557 1 72 700 Tm (hello) Tj 0 Ts 0 Tr 1 w 0 G 1 0 0 1 102 700 Tm ( world) Tj ET
```

Stroking colour matches the blue fill and restores to `0 G`; stroke width
`0.264` = 2.2% × 12 pt, restoring to Table 52's default `1 w`; the shear
sits in the `Tm` matrix's `c` term, bracketed by absolute `Tm`s on both
sides so it cannot reach the following run; no `q`/`Q` appears inside
`BT…ET` (would be illegal — §8.2 Table 51/Figure 9).

**SIX CORRECTIONS TO DECISION 019 — decision 019 Amendment C filed** (full
record `docs/decisions/019-ffh-spacing-scaling-synthetic-styles.md`
Amendment C; `ARCHITECTURE.md` §5.11/§12 updated to match; standing rules
R88/R90 amended, below):

1. **§3.6 named the WRONG restore set.** Stroking colour and stroke line
   width are graphics state **shared with path painting** — a synthetic-
   bold run leaving `0.264 w` in force changes the weight of every later
   stroked *path* on the page. §3.6 treated both only as things to *set*
   correctly, never as things to *restore*. Two restore obligations the
   decision omitted; both now tracked by two new `Walk` trackers alongside
   the existing six text-state parameters.
2. **§3.6's "followers must be re-emitted with an absolute `Tm`" is
   narrower in practice than written — deliberately.** The builder did
   **not** convert a producer's own `Td`/`T*` into an absolute `Tm`
   (that rewrites the producer's own line structure past minimal-diff and
   cascades). pdfce instead **requires** the follower already be absolute
   and **refuses, disclosed, otherwise** — a twin test proves the refusal
   is not unconditional (the same run succeeds once the next line opens
   its own independent `BT…ET`).
3. **§3.6's bold-width formula (`Tfs × |Tm| × |CTM|`) ships two of its
   three factors.** The authoring walk models `Tfs` and `Tm` scale but has
   no `cm` model, so a page-level CTM scale is not compensated. **Disclosed
   verbatim in the builder's report text** ("LIMIT, disclosed rather than
   hidden"), not silently dropped.
4. **Neither the decision nor Amendment A anticipated that synthetic
   italic needs `Tm`/`Tlm` tracking in the authoring walk at all.**
   Amendment A.3 carefully excluded `Tf`/`Tfs` from the shared text-state
   hoist and said nothing about `Tm` — but item 2's refusal gate cannot be
   evaluated without knowing whether a follower is already absolute. The
   walk had no matrix tracking; 19.2 built it (`BT` reset, `Td`/`TD`/`T*`
   derivation, §9.4.4 advance accumulation, a `matrix_known` honesty flag,
   a new `Rec::EndText` variant).
5. **Two conflicts the decision never names, both refused rather than
   silently merged:** free-form rise vs. the superscript/subscript toggle
   (both write `Ts`); synthetic italic vs. `--pin` (the closing absolute
   `Tm` and `--pin`'s compensating `TJ` adjustment would each consume the
   same positional delta twice).
6. **Add-Text synthesis is NOT wired.** The shared type, gate, and wording
   exist in `pdfce-core` with `SynthesisPath::AddText` implemented and
   tested, but `addtext.rs` has no bold/italic request surface, so the
   offer cannot currently be reached from that path. The decision predicts
   the gate "will rarely even open here" (R79 defaults Add-Text to a
   bundled Standard-14 face with real Bolds), but "rarely opens" is not
   "cannot be reached." **Flagged as not delivered, explicitly.**

**ARCHITECTURE §5.11's "exactly one definition" claim is narrowed
accordingly.** It is true of the six §9.3 text-state parameters
specifically — the `Tm` matrix, the stroke line width, and the stroking
colour are tracked separately and deliberately, not folded into
`TextStateParams`.

**R86, stated plainly.** No GUI code was added and the builder made no
on-screen claim — the GUI surface is slice 19.3. Verification was the CLI
oracle plus R85. **Slice 19.3 is now IN DESIGN** — `pdfce-ui-specialist` is
writing `docs/ui_specs/pass-19.3-text-formatting-surface.md` concurrently
with this filing.

**RAG escalations this Pass:**
`C:\personal_rag\pdf\lesson_20260803_mode2_faux_bold_re_detectable_by_stroke_ratio.md`
(mode-2 faux bold is re-detectable across producers from `Tr` + stroke-ratio
+ `/BaseFont` alone; the false-positive guard is that a deliberate outline
display style strokes at 5–10% of size versus a synthesized ~2%) and
`D:\dev\rag\rust\prove_test_suite_non_vacuous_by_deliberately_breaking_the_thing_it_tests.md`
(the mutation-testing method above, generalized — pairs with the existing
`git_stash_on_clean_tree_makes_before_after_comparison_vacuous.md` finding;
both are about verifying a green result was actually earned).

### Pass 19.1 — `Tc`/`Tz`/super-subscript direct text-state authoring, the Acrobat-parity slice (decision 019, extends the Pass 19.0 consolidation) — 2026-08-03, committed `603b051`

**Extends `format.rs`'s existing `pre | set_ops | mid | restore_ops |
post` splice, no parallel path.** New types: `MetricSpec`
(`Absolute | Relative` per R89), `ScriptPosition`, `ScriptMetrics`,
`SUPERSCRIPT`/`SUBSCRIPT` constants; three new `FormatRequest` fields;
seven new `FormatReport` fields; two new `FormatError` variants
(`AmbientUnrestorable`, `BadHorizScale`); `push_state_param` (the R88
restore-ladder application point); `derived_operand` (see the float-
noise finding, below); five new disclosures. CLI: `--char-spacing
V[pt|em]`, `--h-scale PCT`, `--superscript`/`--subscript`/`--no-script`.

**Gates:** `cargo test --workspace` 1643 → 1663, 0 failed; fmt/clippy
clean; `bash tools/check-ui-strings.sh` exit 0; `cargo tree -p
pdfce-core`/`-p pdfce-render` GUI-dep-free; **zero new Cargo
dependencies**; **R85 (preview-equals-saved) 16/16**, including a new
`format_text_spacing_preview_equals_saved` case; roundtrip unchanged
and proven **non-vacuous** by `md5sum`-distinct binaries built from a
genuine pre-change `git worktree` checkout (directly addresses the
vacuous-comparison failure mode flagged at Pass 19.0's ship — see
`D:\dev\rag\rust\git_stash_on_clean_tree_makes_before_after_comparison_vacuous.md`).

**ENGINEER-VERIFIED end-to-end**, on a real fixture — the emitted
content stream:

```
BT /F1 12 Tf 72 700 Td /F1 7.2 Tf 0.24 Tc 85 Tz 4.08 Ts (hello) Tj /F1 12 Tf 0 Tc 100 Tz 0 Ts ( world) Tj ET
```

Set before the run, restored immediately after, inside the same text
object, no `q`/`Q` (illegal there per §8.2 Table 51/Figure 9).
`7.2 = 0.6 × 12`; `4.08 = 0.34 × 12`.

**Superscript/subscript ratios: 0.60× size, +0.34×/−0.18× rise, both of
the BASE size** (decision 019 Amendment B item B.3 makes this explicit
— the decision text left "which `Tfs`" ambiguous). Kept from decision
019 §3.2 rather than re-picked; the Acrobat parity RAG records
Acrobat's own values as an explicit GAP, so this is pdfce's own choice,
not a parity claim, and sits inside ordinary typographic practice —
the asymmetry (larger rise than drop) is deliberate: a lowered glyph
hits its own line's descenders while a raised one has the ascender
band to work with. Every emitted value is disclosed by number in both
the report and the CLI output — no hidden magic number.

**Restore-ladder rungs exercised: 1 (spec default), 2 (observed raw
bytes), and 3 (`ObservedIndirect`, the important one) end-to-end.**
Rung 3 tested on `2 0.25 (lead) "` — asserts the restore re-spells
`0.25 Tc` in its own dedicated operator AND that `(lead) "` appears
**exactly once** in the appended revision (a raw-bytes replay would
have re-painted the string; see decision 019 Amendment A item A.1 /
`C:\personal_rag\pdf\lesson_20260803_quote_operator_side_effect_poisons_raw_byte_restore.md`
for why). **Rung 4 (refuse-and-disclose) is tested at the planner but
is UNREACHABLE end-to-end**: `text_edit::edit::Walk` has no `b"Do"`
arm, so a run inside a Form XObject is never located as a format
anchor in the first place. The builder followed Amendment A.2's
precedent (multi-stream `/Contents` refuse is architecturally
unreachable today) and declined to manufacture an untestable trigger —
filed as a known limit, not a test gap.

**FOUR CORRECTIONS from building this slice — decision 019 Amendment B
filed** (`docs/decisions/019-ffh-spacing-scaling-synthetic-styles.md`
Amendment B; `ARCHITECTURE.md` §12/§5.11 updated to match):
1. **The `Tz` × justify MECHANISM was misstated**, in decision 019
   and in this file's own ★ Pass 19.x entry (now corrected, see
   below). `Th` genuinely rescales `TJ` adjustments per §9.3.4 — but
   the `TJ` numbers carrying a justified line's slack sit OUTSIDE the
   edit's set/restore wrap (in the `pre`/`post` splice) and run at
   ambient `Th`, unrescaled. The conclusion (a `Tz` edit invalidates a
   justified line) survives; the cause is the run's changed rendered
   WIDTH (`ΔA`), not a `TJ`-value rescale. Filed as a general finding:
   `C:\personal_rag\pdf\lesson_20260803_tz_th_rescales_tj_adjustments_not_slack_outside_wrap.md`.
2. **A flagged spec-citation error (`Ts` cited as §9.3.6, text
   rendering mode, instead of §9.3.7, text rise) was verified NOT to
   exist in the decision document** — only in `text_state.rs` (three
   comment citations), already fixed. §9.3.7 is used throughout the
   new code.
3. **R89 now states explicitly that ratios resolve against the BASE
   size** — decision 019 left "`Tfs`" ambiguous; the implementation
   had to choose (chose base) and the record now says so.
4. **R88's four-rung wording in this file's own Standing Rules was
   checked and found already correct** — no edit needed; recorded so
   the item closes rather than silently drops.

**A LIVE DEFECT, found by the R85 oracle — its SECOND real catch.**
`EditSession::format_text` hand-listed its own no-op predicate
(`set_size.is_none() && set_fill.is_none() && set_font.is_none()`).
Pass 19.1's new `FormatRequest` fields bypassed it entirely, making a
spacing-only request a phantom `NoOp` on the **GUI-facing
`EditSession` path specifically** — the CLI's `set_format` path, which
consumes the real `FormatRequest` directly, was unaffected. Fixed by
replacing the hand-list with `req.is_empty()`. Same "duplicated
predicate drifts from what it duplicates" bug shape as decision 019
Amendment A.4 (missing `q`/`Q` arms) — now seen twice, recorded as
**new standing rule R92** (Standing rules, below).

**Two more findings:**
- **Float noise nobody anticipated:** `12.0 × 0.60` is
  `7.199999999999999` in Rust's shortest-round-trip formatting, and
  that noise was headed straight into the emitted stream. New
  `derived_operand` rounds to 6 dp, applied **only to values pdfce
  itself derives** — an operator-supplied absolute value passes
  through untouched, because rounding a typed number would be a silent
  modification of the caller's own input. Filed:
  `D:\dev\rag\rust\shortest_roundtrip_float_format_needs_derived_value_rounding.md`.
- **A latent follower-mispositioning bug, fixed in passing:** the `ΔA`
  computation evaluated `a_new` at the AMBIENT `Tc`/`Th` on both sides
  of the delta — correct only while neither could change, which is
  exactly the slice that makes them changeable. Now evaluated at the
  NEW values. Also: `Tz ≤ 0` is refused by name rather than clamped,
  since it collapses or mirrors the run.

**Rule 11 (CLI parity) — shipped in-Pass.** **No GUI code and no GUI
verification this Pass** — the property surface is slice 19.3, and
this entry says so plainly rather than implying an on-screen check
occurred. Verification was the CLI oracle plus the new R85 case, which
drives the same `EditSession` the GUI drives — so the phantom-`NoOp`
defect above was caught on the exact code path the GUI will use, even
though no GUI frame was rendered this Pass.

### Pass 19.0 — shared text-state model: consolidation + ambient publication (decision 019, correctness-only, no new operator surface) — 2026-08-03, committed `38fffad`

**Consolidates three private ambient-text-state trackers into one.** New
`pdfce-core/src/text_state.rs` in two layers:
`TextStateParam`/`TextStateParams` (parameter identity + resolved
values, for arithmetic-only consumers) and
`AmbientValue`/`AmbientOrigin`/`AmbientTextState`/`AmbientRestoreError`
(values plus restore provenance). One `apply_operator` update rule is
now shared by all three walks (`text_extract::page::TextState`,
`text_edit::edit::Walk`/`reflow_apply::BlockTextState`,
`vector::decompose::GState`). `GlyphProvenance` gains `text_state` and
`composite` fields, published for the first time — previously dropped
at provenance-construction time (`text_edit::edit::Walk` had no
`b"Ts"`/`b"Tr"` arm at all, so pdfce could not yet restore an ambient
rise it never observed).

**Gates:** `cargo test --workspace` 1613 → 1643 passed, 0 failed;
`cargo fmt --check`/`cargo clippy --workspace --all-targets -D
warnings` clean; `bash tools/check-ui-strings.sh` exit 0; `cargo tree -p
pdfce-core`/`-p pdfce-render` GUI-dep-free; zero new Cargo dependencies;
`fixtures/synthetic` roundtrip byte-identical.

**Verification-method finding, worth recording on its own:** the first
roundtrip comparison attempt was VACUOUS — it ran `git stash`
immediately before the "before" build, but the tree was already clean
at that point (a prior commit had landed the change), so `git stash`
silently no-op'd and both "before" and "after" builds used the
identical binary. "Byte-identical" was therefore true but proved
nothing. Redone by building the harness from a real pre-change
`git worktree` checkout instead. **General rule: a before/after
comparison must be demonstrated to actually compare two different
artifacts, not merely asserted to** — escalated to
`D:\dev\rag\rust\git_stash_on_clean_tree_makes_before_after_comparison_vacuous.md`.

**Three deviations from decision 019/R88-R89 as originally written, all
recorded in decision 019 Amendment A (`1a2e265`) and reflected in
`ARCHITECTURE.md` §5/§12 — file any future reference to this slice
against the amendment, not the original decision text:**
1. **The restore ladder needed a FOURTH rung.** R88's "raw operand
   bytes where available" assumed bytes are either available or
   absent; there is a third case, **available and poisonous**. `TD`
   sets `TL` as a documented side effect of moving the line, and `"`
   sets `Tw`/`Tc` **while showing a string** — replaying a captured
   `"`'s raw bytes as a spacing-only restore *repaints the text*.
   Resolved with `AmbientOrigin::ObservedIndirect { setter }`, which
   re-spells the value in its own operator and reports
   `is_byte_faithful() == false` for disclosure. **R88's wording is
   corrected below** (Standing rules) from "restore from raw bytes
   where available" to "restore from raw bytes where they are a
   faithful and side-effect-free record; re-spell where the value is
   known but its source operator did more than set it; refuse where
   unobservable." Also filed as a general PDF-editing finding:
   `C:\personal_rag\pdf\lesson_20260803_quote_operator_side_effect_poisons_raw_byte_restore.md`.
2. **§3.4's tier-3 case (i) (multi-stream `/Contents`) is
   architecturally UNREACHABLE today**, not merely rare —
   `ContentStream::from_page` concatenates the whole `/Contents` array
   before any walk, and a decode failure fails the whole page rather
   than yielding a partial prefix. Recorded with the condition that
   would make it reachable again (lazy/per-element concatenation)
   rather than manufacturing an untestable trigger.
3. **`Tf`/`Tfs` cannot be hoisted into the shared model.** The
   extraction walk narrows `Tfs` to `f32` to publish
   `GlyphProvenance::tf_size` and re-widens it for the §9.4.4 advance
   computation; unifying to `f64` would perturb already-published glyph
   positions bit-for-bit (same narrow-then-divide-vs-divide-then-narrow
   trap applies to `Tz`). The shared type is exactly R88's six
   single-operand parameters. Also: **"exactly one definition" is true
   of `pdfce-core` only** — `pdfce-render::text::TextState` remains a
   deliberate FOURTH tracker, kept independent on purpose because
   render-parity wants an implementation that cannot share a bug with
   the authoring-side model.

**A live defect in already-shipped Pass 14.2 code, found by this slice
and worth its own record:** `text_edit::edit::Walk` had **no `q` and no
`Q` arm at all** (engineer-verified 0 → 1 occurrences before/after).
Text state AND fill colour leaked past a `Q` in the in-place-edit
model — shipped Pass 14.2 behavior could re-emit a fill colour a `Q`
had already discarded. Decision 019 §1.2's own audit of missing arms
reported the missing `Ts`/`Tr` cases and missed this one — recorded
alongside the fix as a meta-point: that audit was otherwise the
strongest part of the decision, and "the audit was thorough" is exactly
the belief that let this gap through. Fixed with two new regression
tests in the same Pass; the justify-gate mechanism (`reflow_apply`'s
`Tc`/`Tz`/`Tw` preamble leak, decision 019's other named correctness
item) is also closed — `restore_ops` now emits only on divergence and
emits nothing on any current fixture (why roundtrip is unchanged), with
a dedicated tripwire test (`reflow_leaves_the_following_text_state_unchanged`)
that fails the moment 19.1 relaxes the justify gate without a restore.

**Rule 11 (CLI parity) — deliberately NOT extended this Pass.** No CLI
surface added. Recommendation recorded for 19.1: `extract-text --json`
should carry the published ambient state, not `object-list` (the wrong
home — `object-list` is a paint-order/hit-test inventory keyed on
vector objects; ambient text state is per-run) — and not yet, since
shipping a flag now would fix its output shape before the
`MetricSpec::{Absolute,Relative}` decision is made in 19.1.

### Observation scripts refuse an ambiguous target — `-ProcessId` disambiguation on `observe-gui.ps1`/`gui-click.ps1`, closing the Backlog item filed at Pass 18.5's ship — 2026-08-03, committed `f45d8d6`

**Both scripts previously selected their target process with
`Select-Object -First 1` over the process name.** With two `pdfce-gui`
instances running simultaneously that silently picks one of them — and
a synthesized click aimed at the wrong window is an unintended action
on some other running application, not merely a failed observation.
Not hypothetical: a build agent had to kill a stale instance because it
could not say which one a script's clicks were actually landing on.

**Both scripts now take an optional `-ProcessId` parameter and REFUSE
outright when several candidate processes exist and none was named**,
listing the running candidate ids so the caller can disambiguate.
Falls back to the prior MRU-first behavior when only one instance is
running or `-ProcessId` is supplied. Deliberately **not** named `-Pid`
— `$Pid` is a PowerShell automatic variable, and a parameter that
shadows it fails confusingly rather than cleanly.

**Verified all three paths live** (single instance / `-ProcessId`
supplied / ambiguous-and-refused). Incidentally, the first verification
attempt demonstrated the client-area blank-frame guard (`6a6a48f`,
Pass 18.5's Shipped entry) catching a case that had previously slipped
through — a fully WHITE client area under a painted title bar — and
confirmed that a mouse *move* does not wake eframe where a real click
does. **First time that failure mode was observed being caught by the
harness rather than reasoned about after the fact.**

Gates: no test-count change (tooling, not product code); no new Cargo
dependency.

### Pass 18.6 — text hit-target geometry now derived from font metrics, not glyph-origin inflation (ui-spec §E, closing the FOURTH and last named cause of "can't click on objects") — 2026-08-03, committed `1b38e34`

**Closes Backlog's "ui-spec §B.4/§C follow-ons" item 1 for real** — Pass
18.4 (`be62e48`) and its `d296666` correction fixed the DISCLOSURE text
about the wrong bbox model; this Pass fixes the bbox itself. Of the four
named contributing causes of the operator's 2026-08-02 "I don't seem to
be able to click on objects" report — Pass 18.0's zoom-inverted select
tolerance (`9a68d6f`), the Obj tool's missing selection outline
(`c998521`), the page-centring coordinate offset (`3f6f5ae`), and this
text-bbox approximation — **all four are now fixed**, not just explained.

`TextObject`'s bbox was the pen-START point of each show operator
inflated symmetrically by the largest `Tf` size in the run (a square
centred on the run's start, for the common single-`Tj` case). It is now
the summed advance widths across the run for the horizontal extent and
`/FontDescriptor` ascent/descent for the vertical. Measured on
`fixtures/synthetic/vector/mixed.pdf`: `before: bbox=16,136,44,164` →
`after: bbox=30,147.102,70.46,160.052` — the old box left ~14 pt of
blank paper before the first glyph and stopped ~26 pt short of the last
(see Pass 18.4's Finding 1 and its `d296666` correction footer, both
above, for the historical/wrong values — left as-is, append-only, now
cross-referenced here as superseded).

**Implementation.** New `Vertical { ascent, descent, nominal }` with a
four-rung ladder: `/Ascent`+`/Descent` (§9.8 Table 122) → `/FontBBox`
`ury`/`lly` (§7.9.5) → compiled-in standard-14 descriptor (§9.6.2.2
no-descriptor case) → nominal 1.0/−0.25 em, flagged. Composite fonts
read the descendant's descriptor per §9.8.1 (descriptors shall not be
used with Type 0 itself). Type 3 deliberately takes the nominal rung —
its descriptor numbers, if any, live in `/FontMatrix` glyph space, not
text space. Reuses `text_extract::font::ExtractFont`'s existing
dictionary-only resolver — no `skrifa`, no glyph shaping added to
`pdfce-core` (rule R21; a hit-test runs per click). New
`advance_tx(w0, tfs, tc, tw, th)` is now the ONE copy of §9.4.4's
displacement formula, shared by `text_extract::page::show_code`,
`redact::glyph`, and this Pass's decompose walk (previously drifting
toward a third, independent implementation).

**Four bbox bases shipped, not the two the ui-spec §E asked for**
(`TextBoundsBasis::{FontMetrics, MetricAdvancesNominalHeight,
EstimatedAdvances, EmBox}`) — deliberate, not scope creep: a Type 3 or
descriptor-less CIDFont has real advances but a guessed height; a
non-standard-14 font with no `/Widths` has estimated advances. Collapsing
either into `FontMetrics` would silently reproduce the exact "sentence
that no longer matches the box" failure §E exists to prevent — and the
third case is not hypothetical, the existing `text/identity-h-no-tounicode.pdf`
fixture exercises it. `approximate` stays `true` in all four bases; only
`EmBox` keeps the "can MISS it" warning. `NoFonts`/unresolvable-font/
non-finite-`Tf`-size objects fall back to the *original* geometry,
basis `EmBox`, pinned by a test — never silently upgraded to a basis the
data doesn't support.

**Two latent bugs found and fixed in passing, both filed as a general
lesson (making geometry honest exposed correctness bugs the sloppy
geometry had been masking):**
1. `'` and `"` did not perform their `T*` line move, and `"` did not set
   `Tw`/`Tc` (§9.4.3 Table 109).
2. `Tc`/`Tw`/`Tz`/`Ts` were not tracked in the decomposer at all —
   `GState` now carries them with Table 105 initial values, saved/
   restored across `q`/`Q`.
Both were invisible under a ±1 em inflation and are not invisible under
a box that claims to be where the text actually is.

**Not modelled, stated in doc comments rather than silently
mis-measured:** vertical writing mode, `Tr` 3/7 (invisible text is
deliberately still bounded), clipping.

**ENGINEER-VERIFIED (R86), via the CLI oracle (same code path as the
GUI) and on screen, on `mixed.pdf`:**
```
ON the visible glyphs   --hit 65,152 --tolerance 0  -> index=1 kind=text   (was a MISS)
blank paper to the left --hit 20,152 --tolerance 0  -> index=none          (was a FALSE HIT)
```
On screen the dashed outline now hugs the word; readout:
`Selected: Text · "Vector" · Helvetica 14 pt · bounds from metrics —
40.5 × 13.0 pt at (30.0, 147.1).`

**CLI output contract (rule 11) — additive, not breaking.** Text rows in
`object-list` gain `bounds=font-metrics|metric-advances-nominal-height|
estimated-advances|em-box`; bbox coordinates now print at 4 dp with
trailing zeros trimmed (the raw `f64` was printing seventeen digits of
f32-widening artefact — four orders below the hit tolerance). Both
changes are additive/lossless for any `key=`-matching consumer.

**Stale values this Pass created, marked historical rather than deleted
(append-only) — cross-referenced here, not rewritten in place:** the
Pass 18.4 entry's `bbox=16,136,44,164` / `28.0 × 28.0 pt at (16.0,
136.0)` figures (above) are now historical, pre-fix values — the old
geometry is exactly why four separate bugs were reported by the operator
as one. `docs/ui_specs/pass-17-dock-and-layer-tree.md` §0.2/§B.3 still
describe the old (wrong) model in the present tense; that file is
`pdfce-ui-specialist`'s territory, not corrected here — flagged for a
`pdfce-ui-specialist` pass to reconcile §0.2/§B.3 against its own
already-written §E.

Gates: `cargo test --workspace` 1599 → 1613 passed, 0 failed; `cargo fmt
--check` / `cargo clippy --workspace --all-targets -D warnings` clean;
`bash tools/check-ui-strings.sh` exit 0; `cargo tree -p pdfce-core` /
`-p pdfce-render` GUI-dep-free; zero new Cargo dependencies. 11 new core
unit tests + 2 CLI integration tests + a bbox assertion added to the
GUI↔`object-list` oracle-agreement test.

### GUI observation-harness fix — blank-capture guard now samples the CLIENT rect, not the window rect; `gui-click.ps1` gained modifier-key support — 2026-08-03, committed `6a6a48f`

**The blank-capture guard added at `d15c360` (continuation 58) sampled the whole WINDOW, not the client area — a guard that can pass on a real failure is worse than no guard, because it earns trust it hasn't earned.** The window rect includes the title bar, which the OS shell paints independently of the application (icon, caption text, min/max/close buttons) — so it always supplies pixel variation, even on a frame the app itself never presented. A capture with a painted title bar and a completely WHITE client area (eframe not yet drawn) therefore PASSED the guard, because the *sampled* pixel set was never uniform, even though the region that matters — what the application actually drew — was blank. The guard's original target case (a fully black, sleeping-display capture) still fired correctly, which is exactly the trap: **a guard that catches the obvious failure and misses the subtle one is more dangerous than no guard, because it earns unearned trust** — a partially-correct guard reads, at a glance, as a working one.

Fixed by resolving the CLIENT rect via `GetClientRect` + `ClientToScreen` and sampling only inside it, excluding the shell-painted chrome entirely.

**Same session, `tools/gui-click.ps1` gained `-Modifiers Shift|Ctrl|Alt`** (held via `keybd_event`, released in a `finally` block so a mid-script exception can't leave a modifier key latched across the whole desktop). Without it, Alt+click (Pass 18.5's click-through-cycling gesture, below) was unverifiable in the running app — and R86 (a Pass does not ship until observed working) makes that verification a ship condition, not an optional nicety.

**Harness gap still owed, not fixed this session:** both `observe-gui.ps1` and `gui-click.ps1` select their target process via `Select-Object -First 1` over the process name, with no way to disambiguate two simultaneously-running instances. A worktree-isolated agent session this cycle had to kill a stale `pdfce-gui.exe` left over from a previous session to stop its synthesized clicks from landing in the wrong window. A `-Pid` parameter on both scripts would close this — filed to Backlog below.

Escalated to `D:\dev\rag\egui\` (see below): screen-capture blank-frame detection must sample the CLIENT rect, not the window rect, on Windows.

Gates: no test-count change (tooling, not product code); `cargo fmt --check`/`cargo clippy` unaffected; no new Cargo dependency.

### Pass 18.5 — `hit_test_point_all` + Alt+click click-through cycling; text/image object detail in decomposition (ui-spec §B.4 core additions + §C's deferred Alt+click cycling, both now SHIPPED) — 2026-08-03, committed `9998a6b`

Delivers the two explicitly-owed core APIs named at Pass 18.4's ship (see that Shipped entry's "Deferred and explicitly owed" paragraph, below) and at the Backlog's "ui-spec §B.4/§C follow-ons" entry (below).

**`hit_test_point_all` (`pdfce-core/src/vector/hit.rs`).** A private `hits_front_to_back()` iterator is the one definition; `hit_test_point` is `.next()` on it, `hit_test_point_all` is `.collect()`. **`hit_test_point(..) == hit_test_point_all(..).first().copied()` is now structural, not conventional** — the two provably cannot disagree, because there is only one hit-testing implementation underneath both. `hit_test_point` still allocates nothing (it never materializes the `Vec`). The same shape is applied at the GUI boundary: `CanvasTargetProvider::hit_test_all` is now the REQUIRED trait method and `hit_test` a PROVIDED method defined as its head (`.first()`), so no future provider implementation can make the singular and plural queries disagree.

**Click-through cycling (Alt+click, per ui-spec §C.3 — chosen specifically to avoid colliding with Shift's existing additive-select binding).** Before this Pass, an object occluded by another object at the same point was structurally UNREACHABLE by pointer — the topmost-only `hit_test_point` gave no path to anything beneath it. Alt+click now steps through the full stack at that point. `ClickCycle` state is **derived-live, not explicitly torn down**: it remains valid only while (a) the same page, (b) the pointer stays within `CYCLE_SAME_POINT_CANVAS = 4.0` canvas units of the originating click, and (c) the current selection is still the object THIS cycle produced. Any of those three failing resets the cycle silently on the next click. There is exactly one explicit clear, in `prune_canvas_selection` (which every edit/undo/redo already funnels through) — because after a content rewrite the same `TargetId` can resolve to a semantically different object, and a stale cycle must not silently continue against it.

**Disclosed in the status readout on every click, not only when cycling is active** — `1 of 3 at this point — Alt+click for the next` — a deliberate choice: this is how an operator discovers the capability exists at all, so it's shown even on a plain single-object click. Suppressed for the trivial `1 of 1` case (no ambiguity, nothing to disclose).

**Text/image object detail (ui-spec §B.4, the core-data-model addition Pass 18.1 deferred).** `TextObject` gains `preview: TextPreview` and `font: Option<TextFont>`; `ImageObject` gains `pixel_size`. New `FontResolver` seam with two implementations: `NoFonts` (the prior behavior, preserved) and `DocumentFonts` (memoizing — one `ExtractFont::resolve` call per distinct font resource per page, not per text object). `decompose(...)`'s public signature is UNCHANGED and internally delegates to `NoFonts`, so every existing geometry-only caller is a no-diff. Decoding reuses `text_extract::ExtractFont::{codes, to_unicode}` directly — the SAME §9.10.2 simple/composite-font ladder `extract-text` already climbs, not a second, parallel decoder for the same problem.

**Rule 11 (CLI/GUI parity) honoured:** `object-list --all-hits` now emits one `hit-candidate … ordinal=N` line per hit, front-most first; text rows gain `font=`/`resource=`/`size=`/`text=`/`truncated=`/`lossy=`; image rows gain `pixels=WxH`. New fixture `fixtures/synthetic/vector/overlap.pdf` (three concentric filled squares, generated via `tools/gen-vector-fixtures.py`, documented in `PROVENANCE.md`); the other five existing synthetic vector fixtures regenerate byte-identically, confirming the generator itself is untouched.

**Memory/work-bound decision, recorded because it was a genuine judgment call, not an obvious default:** the text preview is capped AT DECOMPOSITION TIME, not at display time. `MAX_TEXT_PREVIEW_CHARS = 64`, and the decode loop physically STOPS at that count — a 10 kB `Tj` string is never fully decoded and then discarded, so the cap is a work bound on decode, not merely a memory bound on the result. Worst case ≈450 bytes per text object (≈100 bytes realistic); a 50,000-text-object page tops out ≈22 MB worst case / ≈5 MB realistic. **Owned, truncated `String`s were chosen over borrowed spans deliberately:** a span-based design would require keeping the source `ContentStream` alive for the `VectorObject`'s entire lifetime AND re-running the font-decode ladder on every row redraw — and Objects-tree rows redraw every frame, so that cost would be paid continuously, not once. The separate, smaller display cap (`ROW_TEXT_CHARS = 32`, applied in the GUI layer on top of the already-64-char-capped core value) exists specifically so a future memory-tuning decision on ONE cap can never silently retypeset the other.

**Three honest limitations, disclosed rather than hidden — all deliberate, all reversible:**
1. **The preview is sourced-only text, with no derived spacing.** `simple-winansi.pdf` previews as `"HelloworldSecond line"` — its word gap is a `TJ` kerning offset with no literal space glyph, and its line break is a bare `Td` move (§14.8.2.5 layout modes S3/S5), neither of which the content stream states as a character. `text_extract`'s `plain_text` mode *derives* both a space and a line break for exactly this case; `sourced_text` mode omits both, and a preview uses the latter. Reusing the derived layer here would mean re-implementing `text_extract/layout.rs`'s heuristics over a second interpreter pass. Judged: an odd-but-literally-true string is a more honest preview than a reader-supplied guess presented as if it were the document's own content. **One call site (`Decomposer::decode_show_string`) — reversible in an afternoon if this judgment is revisited.**
2. **`TextFont::size` is the `Tf` operand exactly as the content stream states it, not the rendered glyph size.** `/F1 1 Tf` followed by `12 0 0 12 tx ty Tm` renders at 12 pt but reports `size: 1.0` — folding the text matrix's scale into this field would produce a number that disagrees with what the content stream literally says, and `pdfce-core` has no glyph-metrics access at this layer to defend a "measured" alternative anyway. Documented on the field itself and in the relevant CLI help text.
3. **`TextPreview` is a four-variant enum (`Decoded{text,truncated,lossy}` / `Undecodable` / `Unavailable` / `Empty`), deliberately not `Option<String>`.** "No text to show" has four semantically distinct causes, and only one of the four (`Empty`) is actually a fact about the document itself — collapsing all four into `None` would silently discard that distinction from every future caller.

Gates: `cargo test --workspace` 1559 → 1599 passed, 0 failed; doc-tests 69 passed; `cargo fmt --check` / `cargo clippy --workspace --all-targets -D warnings` clean; `bash tools/check-ui-strings.sh` exit 0; `cargo tree -p pdfce-core`/`-p pdfce-render` GUI-dep-free; zero new Cargo dependencies; `fuzz/` `cargo check --all-targets` clean.

**ENGINEER-VERIFIED ON SCREEN (R86):** on `overlap.pdf`, a plain click at the centre reads `Selected: Path · filled #33B34D · 4 node(s) — 60.0 × 60.0 pt at (120.0, 120.0). 1 of 3 at this point — Alt+click for the next.`; Alt+click advances to `#E69933 · 160.0 × 160.0 pt at (70.0, 70.0). 2 of 3`, with both the selection outline and the `P` corner badge moving to the newly-cycled object. On `mixed.pdf`: `Selected: Text · "Vector" · Helvetica 14 pt · approximate bounds` and `Selected: Image · 2 × 2 px`.

### `ui-strings` CI gate — was RED AT BASELINE (140 hits) and hiding a real R1 violation; FIXED and moved to a local script — 2026-08-03, committed `a5d1d18`

**The job enforcing decision 002 R1 (single string catalog, `ui_text.rs`)
had been red at baseline on 140 hits for some unknown prior span — the
rule was not actually being enforced.** A gate that cannot pass trains
everyone who sees it red to ignore it; this is worse than having no gate
at all, because it looks like coverage that doesn't exist.

**It was concealing a genuine violation.** The Measure sub-tool names
`"Linear"` / `"Radius/Diameter"` / `"Set Scale"` are drawn directly on
the toolbar and lived as bare string literals in `main.rs` — a real R1
breach. Moved into `ui_text.rs` as `measure_tool_name_linear` /
`_measure_tool_name_circular` / `_measure_tool_name_scale`. Two of the
three would **not** have been caught even by a gate that was green at
baseline (the whitespace-literal heuristic only flags literals
containing a space character, and `"Linear"` has none) — they were
moved anyway because the rule is about operator-visible text living in
one place, not about what a regex can see.

**Breakdown of the 140 baseline hits:** 125 were test-assertion
messages (prose, but read only by whoever is debugging a failing test,
never rendered to an operator); 14 were an `impl Display` writing an
error's own description (`pdfce-core`/`pdfce-render` diagnostic text —
different audience and lifecycle than GUI chrome, R4's domain not R1's);
3 were the **detector regex itself misreading Rust** (see the RAG
finding below — `"svg" | "?xml"` was parsed as one literal spanning
`" | "`); 1 was a genuine stderr diagnostic, now explicitly exempted
with its reasoning recorded inline.

**The rule moved out of the inline CI grep into `tools/check-ui-strings.sh`**
so it can be run locally before pushing, not only discovered by CI after
the fact. Every exclusion category is justified in the script's own
header, and the character scanner tracks which quotes open/close a
literal rather than matching a regex against a whole line. Exemptions
(`// ui-text-exempt: <reason>`) may now sit in the comment block ABOVE
the offending line, not only trailing it, because a real reason
sometimes doesn't fit past column 100.

**Verified by making the gate FAIL on purpose, not only by making it
pass.** That check immediately exposed a second limit, now recorded in
the script itself: the first planted violation was appended to
end-of-file — i.e. after `#[cfg(test)]`, where the checker truncates —
so the gate stayed green and briefly looked like a check that could
only ever pass. Re-planting the violation above the test module caught
it correctly. **This is the standing methodology lesson, escalated to
`D:\dev\rag\rust\` below: verify any gate by making it fail, never only
by making it pass.**

Gates: the script itself is the gate (no separate Pass test-count
change); `cargo fmt --check` / `cargo clippy --workspace --all-targets
-D warnings` clean; `tools/check-ui-strings.sh` clean against the
current tree (0 hits after the fix, all 140 baseline hits accounted
for above); no new Cargo dependency.

### Pass 18.4 — Selection legibility (ui-spec §C, closing the deviation flagged at Pass 18.1's ship) — 2026-08-03, committed `be62e48`

**Pass-number note (flag, per hard rule "Pass IDs are stable, never
reused"):** the shipped commit's own message says "Pass 18.2:
selection legibility" — that collides with the ALREADY-SHIPPED Pass
18.2 (`object-list` CLI subcommand + headless hit-test query,
committed `dae0139`, 2026-08-02 — see that Shipped entry above). This
is a naming slip in the commit message, not a real reuse of the ID:
the feature this entry describes is the "ui-spec §B.4/§C follow-ons"
Backlog item filed at Pass 18.1's ship (2026-08-03, no Pass number
assigned at filing time). **Filed here as Pass 18.4** — the next free
slot in the 18.x family — so the ledger stays unambiguous; the commit
message itself is not rewritten (history stays as committed), but the
roadmap entry is the canonical Pass-ID record per the Update protocol.

Delivers the P1 half of the Backlog item filed at Pass 18.1's ship:
(1) `crates/pdfce-gui/src/object_summary.rs`'s `describe_object(&VectorObject)
-> ObjectSummary` — a **prose-free fact record** (kind, paint style,
visible colour, node count, line width, bounds, disclosure notes) with
no strings in it at all, so decision 002/R1 is satisfied structurally
(nothing here can leak an untranslated literal, because there is no
literal to leak) and the module is headlessly unit-testable without a
running GUI. `ObjectKind` is deliberately finer-grained than
`VectorObject` — inline image / image XObject / form XObject are three
different answers to "what did I select?", not one "image" bucket.
`ObjectNote` carries five disclosable facts: `ApproximateTextBounds`,
`PaintsNothing`, `DegenerateBounds(VerticalRule|HorizontalRule|Point)`,
`NoBounds`, `FormNotDecomposed`.

(2) **This is now the ONE source of truth shared by the Objects tree
row and the canvas status readout** — a test asserts the tree row's
detail clause appears verbatim inside the status readout, making the
"these two must never disagree" guarantee structural rather than a
convention two future edits could silently drift apart on (the exact
divergence pattern `object_provider.rs`'s own doc comment cites
decision 011 warning about).

(3) `canvas.rs`'s `selection_outline_bounds` now returns
`Vec<(TargetId, Rect)>` instead of a bare `Vec<Rect>` — the overlay
cannot choose a per-kind treatment (solid vs. dashed outline, badge
letter) from a bare rect list because `filter_map` breaks positional
correspondence between objects and rects. New `visible_outline_rect` +
`MIN_OUTLINE_EXTENT_PX = 6.0`, applied in SCREEN space (zoom-invariant,
symmetric about the rect's centre, non-finite values passed through
unmodified) so a degenerate (zero-height/zero-width/point) selection
still paints a visible, inflated box instead of nothing.

(4) `main.rs`: solid outline for a measured bounds, **dashed** outline
for an approximate one (a SHAPE cue, never colour alone — R84);
degenerate rects get the inflation above; a corner chip carries a
`P`/`T`/`I`/`F` letter badge (Path/Text/Image/Form). New
`selection_readout` in `status_bar_body`, placed ABOVE the
`page_texture` early-return so the readout survives a pre-raster frame
and doesn't flash away on the very frame it would be most useful.

**ENGINEER-VERIFIED ON SCREEN (R86, not merely headless):** clicking
blank paper immediately left of the word "Vector" in
`fixtures/synthetic/vector/mixed.pdf` selects the text object and shows
a dashed box over mostly-empty paper, a `T` badge, and the status line
`Selected: Text · approximate bounds — 28.0 × 28.0 pt at (16.0, 136.0).`
**This is the operator's original "a box highlighting on the screen
that doesn't seem to correspond to anything" complaint, now explained
rather than merely fixed** — the box IS on the right object, it's just
that the text bbox approximation undershoots the glyph ink (see
Finding 1 below).

**Three findings this Pass, all escalated below:**

1. **The `docs/ui_specs/pass-17-dock-and-layer-tree.md` ui-spec's own
   model of the text-bbox approximation is WRONG and needs correcting
   by `pdfce-ui-specialist`.** §0.2 and §B.3 both describe the
   approximation as "wider and taller than the ink." Empirically, for
   `mixed.pdf`'s text object, the bbox is inflated from the glyph
   ORIGINS by the largest `Tf` size in the run, giving `bbox=16,136,44,164`
   (28×28 pt) — while the rendered glyphs actually run ~40 pt wide,
   starting further right than the box's left edge. **The box is
   narrower than the glyphs and offset from them, not merely oversized**
   — so clicking directly ON visible text can MISS the hit region
   entirely. This is a FOURTH contributing cause of the operator's
   "can't click on objects" report (alongside Pass 18.0's zoom-inverted
   tolerance, `c998521`'s missing selection outline, and `3f6f5ae`'s
   centring-margin coordinate offset) — filed to Backlog below, needs a
   `pdfce-ui-specialist` re-dispatch to correct the spec text, not a
   RAG finding (this is project-internal documentation accuracy, not a
   generalizable ecosystem or PDF-domain fact).
2. **A status-bar height change retriggers `Fit page` zoom — a
   pre-existing egui feedback loop, not new to this Pass.** The first
   cut of the status readout put full explanation sentences inline.
   Because the status bar is a bottom panel, every line it grows
   shrinks the canvas viewport, and under `Fit page` zoom mode that
   re-fits the page smaller on the very next frame: the page visibly
   jumped and shrank (230% → 224% → 215%) as lines accumulated across
   frames, which ALSO invalidated the click coordinates the operator
   had just used, since the page moved between the click frame and the
   next render. Fixed here with a one-line headline plus a
   `CollapsingHeader` for detail — but **the loop itself is
   pre-existing and applies to any future status-bar content growth**
   (save notes, edit disclosures, warnings). Filed as a standing hazard
   to Backlog below, not a Pass-18.4-specific bug, and escalated to
   `D:\dev\rag\egui\` below since it generalizes to any egui app
   combining a dynamic bottom panel with a fit-to-viewport zoom mode.
3. **`icons::Icon` (the Pass 18.3 drawn-vector icon set) cannot supply
   object-kind badges.** No glyph exists for path/image/form XObject,
   and `Icon::Text` already denotes the TEXT TOOL, not "this is a text
   object." Reusing it for the badge would assert an affordance that
   doesn't exist (R83). Letter badges (`P`/`T`/`I`/`F`) are the honest
   interim choice, not a placeholder to feel bad about.

**Deferred and explicitly owed, not built:** Alt+click cycling through
overlapping objects. `pdfce_core::vector::hit::hit_test_point` is
`objects.iter().enumerate().rev().find(...)` — topmost-only, no
all-hits query exists anywhere in `pdfce-core`. `hit_test_rect`
answers a different question (bbox enclosure, no tolerance, no
nearest-first ordering) and can't substitute. Needs a new core API,
tentatively `hit_test_point_all -> Vec<usize>` (nearest/topmost-first
ordering) — filed to Backlog below as an owed core API, not
half-built.

Gates: `cargo test --workspace` 1538 → 1559 passed, 0 failed; `cargo fmt
--check` / `cargo clippy --workspace --all-targets -D warnings` clean;
`cargo tree -p pdfce-core` / `-p pdfce-render` free of egui/eframe/
winit/wgpu/glow/egui_tiles (GUI-core separation intact); zero new Cargo
dependencies.

**CORRECTION (2026-08-03, committed `d296666`) — the `ApproximateTextBounds`
disclosure text shipped by THIS Pass was itself inaccurate, in the
dangerous direction.** The copy written here explained why a selection
box can sit over blank paper by repeating the same "normally wider and
taller than the ink" model Finding 1 (above) had already identified as
WRONG in the ui-spec, and concluded "the selection is correct even
though the box looks empty." Traced to `decompose.rs`
(`record_text_origin`/`end_text`) and `geometry.rs` (`Bounds::inflate`):
only the pen-START point of each show operator is recorded, and the box
is that single point inflated symmetrically by the largest `Tf` size in
the run — for the common single-`Tj` case this is a SQUARE centred on
the run's start, not a padded region around the ink. It reaches
backward into blank paper before the text begins and stops roughly one
em in, short of most strings' actual extent. **The shipped copy
reassured the operator that a surprising selection was correct, while
saying nothing about the opposite and strictly worse failure mode:
clicking directly on visible glyphs can MISS the text object entirely.**
An operator who read that note, then failed to select some visible
text, would reasonably conclude the tool was broken — having just been
told selection was reliable by the same feature. Rewritten to state the
actual box construction, BOTH failure directions (surprise-hit and
surprise-miss), and a workaround (click nearer the start of the line).
**This is disclosure only — the underlying hit-target geometry is still
wrong; see the Backlog "ui-spec §B.4/§C follow-ons" entry's item 1, now
IN PROGRESS** (`pdfce-ui-specialist` has written the corrected geometry
spec, ui-spec §E, and a builder is implementing the fix as of this
filing).

**FURTHER CORRECTION (2026-08-03, committed `1b38e34`) — the underlying
geometry itself is now fixed, not merely disclosed. See the Pass 18.6
Shipped entry (top of Shipped) for the full build record.** All of this
entry's `bbox=16,136,44,164` / `28.0 × 28.0 pt at (16.0, 136.0)` figures,
above, are historical (pre-fix) values, left in place per the append-only
rule — they are the reason the operator's complaint was originally
reported as one bug instead of four.

### Menu-affordance & glyph-coverage audit — tofu-glyph class CLOSED (pdfce-ui-specialist audit + engineer fixes) — 2026-08-03, committed `85a6cac` / `a1badc1` / `eeadbcb` / `869d891`

**Closes the open defect flagged at continuation 57's Pass 18.3 entry**
(`▾` U+25BE tofu on 4 menu-affordance controls) and, in the course of
verifying the fix by direct observation, finds and closes a SECOND tofu
pair the original audit's four-controls list missed.

- `85a6cac` — `pdfce-ui-specialist` delivers `docs/ui_specs/
  menu-affordance-and-glyph-coverage.md`, dispatched at continuation 57
  to adjudicate the `▾` tofu on `Markup □`/`Text □`/`Measure □`/Copy's
  `⧉ □` and to sweep `ui_text.rs` project-wide for other unrenderable
  codepoints. **Root cause:** pdfce sets no custom fonts; egui's default
  Proportional font chain (Ubuntu-Light → NotoEmoji → emoji-icon-font)
  covers none of `▾`/`▲`/`▼`.
- `a1badc1` — engineer implements the audit's fix: menu-affordance
  controls now draw a real drawn CHEVRON (vector, same tinted-mask style
  as the Pass-18.3 icon set, not a font glyph) instead of `▾`, AND carry
  an AccessKit accessible name of the form "{label}, opens a menu." The
  audit's finding driving the SECOND half of this fix: egui's
  `WidgetType` has no menu/has-popup role and `menu_button` sets no
  `WidgetInfo` at all, so "opens a menu" only ever reaches assistive
  tech as literal text — deleting the tofu glyph WITHOUT adding that
  text would have made the control LESS accessible than the bug (a tofu
  box at least carries a Unicode character name some screen readers
  speak). Both cues are applied through one wrapper so they cannot drift
  apart in a future edit.
- `d15c360` — see the harness-hardening entry below; built because
  visually verifying the chevron fix needed a screenshot the harness
  could be trusted not to falsely bless as evidence.
- `eeadbcb` — engineer VERIFIES the `a1badc1` fix by direct observation
  of the running build (not headless-only) and, doing so, finds a
  SECOND tofu pair the original four-controls audit list missed: `▲`/`▼`
  (U+25B2/U+25BC) on the thumbnail rail's page-reorder controls and the
  Combine-files dialog's reorder buttons — GLYPH-ONLY controls (no
  accompanying text label at all), so before this fix they had NO
  visible identity whatsoever, a strictly worse case than the tofu-next-
  to-a-label controls the original audit named.
- `f963895` interleaves chronologically with this thread (Pass 18.1, own
  Shipped entry below) but is unrelated to it.
- `869d891` — engineer draws real chevrons for the rail and Combine-
  files reorder arrows, closing the second pair. Also fixed in the same
  commit: `copy_text_button()` returned the bare glyph with no "Copy"
  word in its accessible name at all; a tooltip on a sibling control
  cited U+FFFC where its own sibling control and `pdfce-core` both use
  U+FFFD (the standard REPLACEMENT CHARACTER) — corrected to U+FFFD for
  consistency; the Combine-files reorder buttons had never been migrated
  onto the shared accessible-name wrapper used everywhere else and now
  are.

**Milestone: `glyph_button` (the helper that rendered raw Unicode
glyphs as button faces) is deleted — it has no remaining callers.**
pdfce now has **no text-glyph buttons anywhere**; every icon-only
control is a drawn vector icon (`icons.rs`, Pass 18.3) whose appearance
does not depend on whether the host font stack happens to contain an
unverified codepoint.

**Explicitly NOT verified, not claimed fixed:** `✓`/`✕` (U+2713/U+2715)
on three tools' Accept/Reject buttons remain unverified — reaching them
requires an in-progress tool gesture the observation harness hasn't yet
driven. The rail checkbox's own tick is egui's vector-drawn `Checkbox`,
not a font glyph, so observing it proves nothing about this class.
Filed as an explicit Backlog follow-up (see below) rather than silently
assumed clean by extrapolation from the rest of the audit.

Gates: rolled into the Pass 17.1/17.2 and Pass 18.1 test counts (below);
fmt/clippy clean throughout; no standalone Pass number assigned (these
are `pdfce-ui-specialist`-directed fixes, not a numbered feature Pass).

### Canvas hit-testing coordinate-mapping fix — the THIRD root cause of "can't click objects" — 2026-08-03, committed `3f6f5ae`

`canvas()` allocated the page image inside `ui.centered_and_justified
(...)` and used `image_response.rect` as the page↔screen mapping origin
for every `screen_to_page`/`page_to_screen` call. `centered_and_justified`
returns the JUSTIFIED CONTAINER rect while drawing the child image
CENTRED inside it — so whenever the rendered page was smaller than the
viewport (any zoom short of "fills the window"), every hit-test and
every selection-outline draw was offset by the centring margin.
Selection outlines drew roughly 105px away from the object they marked;
a click directly ON a visible object missed. **The error scaled with
the centring margin, not the zoom** — it vanished at high zoom (margin
→ 0) and was worst at exactly the zoom an operator would use to see a
whole page, the single most common working zoom level. Fixed by
reserving `max(page_size, viewport_size)` and placing the image at an
explicit centred rect via `Ui::put`/`allocate_exact_size`, so the same
rect used to draw is the same rect used for coordinate mapping.

**Correction to the record, stated explicitly rather than silently
absorbed, per this file's own discipline:** after the selection-outline
fix (`c998521`, continuation 56), the engineer attributed still-failing
synthetic canvas clicks in the observation harness to egui's
`Response::clicked()` not being satisfiable by OS-level synthetic input
(filed continuation 56 as `D:\dev\rag\egui\
synthetic_os_pointer_input_not_response_clicked.md`). **That RAG finding
remains true in isolation, but it was not the actual explanation for the
specific failures being diagnosed at the time** — the harness was fine;
the app was mapping coordinates incorrectly, and the harness was
correctly reporting no-hit because there genuinely was no hit at the
coordinates it clicked. The earlier reasoning missed a bigger tell:
toolbar clicks worked (no coordinate mapping involved) and canvas clicks
did not (coordinate-mapped) — read at the time as an egui synthetic-
input subtlety, when the simpler and ultimately correct explanation was
that only the mapped path was broken. The `object-list --hit` CLI query
proving core hit-testing geometrically correct (Pass 18.0/18.2) was true
and simultaneously misleading: the underlying geometry math was right,
but the screen coordinates fed into the screen→page conversion ahead of
that math were wrong. **Net: the operator's original single-sentence
complaint had THREE independent, now all-fixed, causes** — the
zoom-inverted selection tolerance (Pass 18.0, `9a68d6f`), the missing
selection-outline draw in the Obj tool (`c998521`), and this centring-
margin coordinate bug (`3f6f5ae`) — not one, and not two.

Filed to `D:\dev\rag\egui\
centered_and_justified_returns_container_rect_not_child_rect.md`. Gates:
rolled into Pass 18.1's test count (below); fmt/clippy clean.

### Pass 17.1 + Pass 17.2 — `session.document()` audit finishes + R85 preview-equals-saved oracle harness (decision 018, slices 2/3 of 3 — decision 018 now COMPLETE) — 2026-08-03, committed `437a6f7`

**Pass 17.1** finished the `session.document()` audit named at Pass
17.0's ship: `count_redaction_marks` (`main.rs:4606`), `need_appearances`
(`main.rs:4598`), and `page_font_entries` (`main.rs:6078`) were all
reading the base revision instead of `session.view()`; all three fixed.
The remaining named-for-triage sites (`main.rs:1377/2300/2391/4953`)
triaged individually; `main.rs:1779` `recovery()` and `:4491` `version()`
confirmed as legitimate base reads, left alone.

**Pass 17.2** built the R85 preview-equals-saved oracle as a `tools/`
harness (no new public CLI surface — rule 11's CLI parity is satisfied
trivially, since the CLI's one-shot parse→edit→save model never renders
an unsaved session, so there is no CLI behavior change to expose). It
renders a live `EditSession`'s current view and compares it pixel-for-
pixel against the raster of the same session saved-then-reloaded,
reusing the Pass 11 raster oracle. Covers **11 of the 12** named R85
operations (`add-text`, `annotate`, `dimension-add`, `object-move`,
`object-delete`, `node-move`, `edit-text`, `format-text`, `reflow`,
`flatten`, `fill-field`); `redact-apply` is **structurally**
uncoverable, not merely unimplemented — applying redaction is not an
`EditSession` operation, it consumes a `Document` and emits a file
directly, so "preview equals saved" has no live-session left-hand side
to compare against for that one operation (full architectural framing:
`ARCHITECTURE.md` §12's continuation-58 decision-018 entry).

**On its very first run, the oracle found real, silent data loss — the
headline finding of this session.** `flatten_fields` issued THREE
whole-dictionary `ObjectWrite`s to the SAME page object within one
command (`/Contents`, `/Resources /XObject`, `/Annots`), each cloned
from the pre-command page state. `EditSession` applies a command's
writes in sequence against that pre-command state and nothing commits
mid-command, so the three writes OVERWROTE rather than composed, and
`/Annots` (written last) won: flatten created the burn-in appearance
stream and the new page content, then discarded both by re-writing the
page dict back to its pre-flatten `/Contents`/`/Resources` with only the
`/Annots` deletion applied. **Every flattened form silently lost its
visible burned-in field values**, while `fields_flattened`/
`widgets_burned`/`pages_touched` all still reported correct counts — the
operation reported success and was wrong. Every existing flatten test
passed throughout the feature's life (Pass 7.1 onward) because none of
them rendered the result; they all asserted on the returned counters and
the AcroForm structure, never on the rendered page. Reproduced
independently against the pre-fix binary: identical command, burned
value ABSENT from the rendered page pre-fix, PRESENT post-fix. See
`ARCHITECTURE.md` §11.1's continuation-58 addendum for the general
architectural rule this establishes (at most one `ObjectWrite` per
object id per command) — other multi-write commands are owed the same
audit, not yet performed exhaustively.

Two further silent-wrong-answer bugs, found by the same audit sweep,
distinct in KIND from the `ObjectWrite`-overwrite bug above:
- **Search-redaction resolved against the wrong page after any
  delete/reorder.** `author_text_matches` extracted match geometry
  using BASE page indices, then fed that geometry straight to
  `add_redaction`, which resolves page numbers through SESSION
  `page_slots`. After any page delete/reorder earlier in the same
  session, a search-driven redaction mark silently lands on the WRONG
  page, with fully plausible-looking geometry — nothing about the
  result looks wrong on inspection. Fixed.
- **Content authored this session could be extracted as empty.**
  `extract_selection` paired a SESSION object graph with BASE bytes. A
  stream authored during the current session has no corresponding bytes
  in `base`, so extraction silently returned empty content instead of
  erroring or reading the session's own staged bytes. Fixed.

**Consequence for the GUI, worth naming as a real feature gap, not
merely an oracle-coverage gap:** there is currently **no GUI flow for
redaction apply at all** — mark-and-disclose only; applying redaction
(the operation that actually removes content) is CLI-only,
`pdfce-cli redact-apply`. This predates this session but is newly
notable for being exactly the one operation R85 cannot cover. Filed to
Backlog below.

Gates: `cargo test --workspace` **1521 passed, 0 failed** (from
continuation-57's 1504 baseline); `cargo fmt --all --check` clean;
`cargo clippy --workspace --all-targets -D warnings` clean; `cargo tree
-p pdfce-core`/`-p pdfce-render`/`-p pdfce-cli` free of egui/eframe/
winit/wgpu/glow (GUI-core separation intact); zero new Cargo
dependencies. **Decision 018 (live-edit rendering) is now COMPLETE
end-to-end** — Pass 17.0 (canvas renders the edited view), 17.1 (every
remaining base-read site triaged and fixed), and 17.2 (the oracle that
proves it, 11/12 operations) have all shipped. Full architecture:
`docs/decisions/018-edited-state-is-what-the-canvas-renders.md`;
`ARCHITECTURE.md` §12 continuation-58 entry.

### Pass 18.1 — `egui_tiles` dock shell + object/layer tree panel (decision 017 Amendment A, BUILT) — 2026-08-03, committed `f963895`

Builds the shell decided at decision 017 Amendment A (continuation 57):
`egui_tiles` 0.16.0, `pdfce-gui`-only, **`default-features = false`**
(see `ARCHITECTURE.md` §12's continuation-58 entry for why that flag
mattered — the crate's default features include `serde`, unmentioned in
the original vetting table, and would have silently contradicted the
continuation-57 instruction not to enable it yet). Exactly **1** new
package; `Cargo.lock` +13 lines; MIT OR Apache-2.0; `THIRD_PARTY_
LICENSES.md` regenerated via `cargo-about 0.9.1` (generated, not
hand-edited, per rule 13); `cargo tree -p pdfce-core`/`-p pdfce-render`/
`-p pdfce-cli` verified clean.

New `crates/pdfce-gui/src/dock.rs` (~510 lines): `enum DockPanel` + ONE
`panel_body` dispatcher (R80) survives verbatim from the original
decision as the `egui_tiles` pane payload. Default layout ships Objects
ABOVE Properties as a vertical split, BOTH simultaneously visible —
pinned by a unit test asserting both panels are present in
`active_tiles()` so a future "fold these into one tab group" regression
fails loudly instead of silently reintroducing the exact simultaneity
gap the operator originally complained about. Both engine gotchas
pre-paid at continuation 57 were real and handled as predicted:
`Tree<Pane>` derives `Clone, PartialEq` but not `Default` (used
`std::mem::replace`, not `std::mem::take`); `SimplificationOptions::
default()` needed `all_panes_must_have_tabs: true` overridden, or the
tab bar vanishes when only one pane is open. `properties_open` (a
second source of truth for panel visibility) is deleted;
`properties_window()` is retired — no more float-or-dock dual mode, per
R80/R81. New `Action::ResetPanelLayout`. Dock default width 320 →
380pt. Inner "Tools" row renamed **"Batch Tools"** to disambiguate from
the new Tools dock generally.

**Mandatory bugfix, caught before ship, not after:** `open_path()` did
not invalidate `properties_draft` (the in-progress edit buffer for the
Document Properties form). For a now-persistently-mounted panel — which
the dock makes Properties, for the first time, since it no longer needs
to be opened/closed to appear — the failure mode would have been an
operator opening a NEW document, seeing a stale (or EMPTY) metadata form
left over from the PREVIOUS document, and clicking Apply, silently
overwriting the new document's real `/Info` dictionary with leftover or
blank values. Fixed with two regression tests covering both the
stale-value and the empty-value cases.

Object tree: flat list, paint order, front-most object first,
`ScrollArea::show_rows` virtualized (no item-count cap). Bidirectional
selection sync (tree click ↔ canvas click) reuses
`canvas::selection_after_click` verbatim rather than reimplementing
selection logic a second time. **Pinned against drift from the canvas
itself:** a regression test asserts the object tree agrees with the
`pdfce-cli object-list` oracle (Pass 18.2) — same indices, same object
kinds — so this diagnostic/navigation surface cannot silently diverge
from what the canvas actually hit-tests.

**Accessibility, recorded honestly rather than left implicit:**
`egui_tiles` 0.16.0 falls on the UNFIXED side of the AccessKit
tab-naming gap continuation-57 flagged as still-open (zero
`widget_info`/`accesskit` hits in the pinned release's source) — tab
names are supplied via `Behavior::on_tab_button` rather than by forking
`tab_ui`. **A gap that cannot be closed downstream at all:** egui 0.35's
`WidgetType` enum has no `Tab`/`TabList` member, so a tab announces with
a correct name and correct selected state as a `SelectableLabel`, but
the correct semantic ROLE is unavailable short of an upstream egui
change. Filed to `D:\dev\rag\egui\egui_035_no_tab_tablist_widgettype.md`.

**Deliberately NOT done, and said so in code:** the dock still starts
CLOSED by default. The original justification for that default —
Properties being pdfce's sole legacy floating exception, per the now-
superseded R80/R81 framing — is now false, but flipping a startup
default is a product call left to the operator, not taken unilaterally.

**Deviation from the ui-spec §B.4/§C "binding asks" named in this
family's own Next-up entry — flagged, not silently dropped:** the
ui-spec's §B.4 core additions (`TextObject` extracted-string preview +
resolved font-name/size; `ImageObject` pixel width/height) and §C's full
selection-legibility asks (type badge, invisible/approximate-hit
disclosure, status readout) were **NOT** delivered as part of this
Pass, despite §B.4 being framed as a "binding core ask" in the original
entry. See Backlog below ("ui-spec §B.4/§C follow-ons") for the
consolidated remaining scope, including a newly-found case (a
zero-height path/horizontal-rule object selects correctly but its
outline is a zero-height rect that paints nothing visible).

Gates: `cargo test --workspace` **1538 passed, 0 failed** (from 1521);
fmt/clippy clean; `cargo tree` invariant intact; exactly one new
dependency, license-classified and attributed. Full architecture:
`ARCHITECTURE.md` §12 continuation-58 entry; `docs/decisions/
017-tabbed-dockable-panel-system.md` ("AMENDMENT A").

### GUI observation-harness hardening — refuse a uniform (blank/black) capture — 2026-08-03, committed `d15c360`

`tools/observe-gui.ps1` now refuses to return a capture whose pixels are
ALL the same colour, after three distinct real causes each
independently produced one during this session's diagnostic work: a
sleeping monitor (solid black), eframe not yet having presented a frame
(solid white — eframe repaints only on receiving real input, per
`eframe_blank_until_first_input_reactive_repaint.md`, filed
continuation 57), and a window-animation frame caught mid-flight. Under
proposed standing rule R86 ("observed working in the running
application"), an image containing no evidence at all is exactly what a
hurried check would wrongly accept as evidence — this is the THIRD
guard of its kind added to this harness, after "refuse when the target
window isn't foreground" and "refuse to click outside the target
window's bounds." Gates: rolled into the counts above; no dedicated Rust
test suite (this is a PowerShell tool).

### Pass 18.3 — ScripTree-style SVG icon set + toolbar overflow wrapping (icon-set entry RESOLVED, operator priority #2) — 2026-08-02, committed `c59b0c4`

**Closes the long-queued ★ Icon set Next-up entry (design complete
2026-08-01, build unblocked 2026-08-02 once Pass 17.x's tiny-skia
pipeline decision landed) and Pass 18.3 of the ★ Pass 18.x family
(Measure ▾ affordance fix, ui-spec §D — see below).** Docs commit
`f9bb560` landed first this session (continuation-56 filing + decision
017 Amendment A + a `pass-17-dock-and-layer-tree.md` status notice +
the `tools/roundtrip` HashMap-sampling determinism fix — see the
commit-chain update below and the ARCHITECTURE §12 continuation-57
entry); Pass 18.3 is the feature commit that followed it.

**Pipeline — new `crates/pdfce-gui/src/icons.rs` (~2000 lines, ~55%
doc comment), zero new Cargo dependencies, `Cargo.lock` byte-identical.**
SVG-path `d`-attribute → `tiny-skia` → egui-texture, reached via
`pdfce_render::tiny_skia` (already a pdfce dependency) — the tiny-skia
path chosen at Pass 17.x/continuation-56 over the original
pre-rasterize-to-PNG plan, which could not execute on this machine (no
Inkscape/ImageMagick, `cairosvg` libcairo load failure). Supports
`<path>`/`<rect>`/`<circle>`, full path-data command grammar
(`M m L l H h V v C c S s Q q T t A a Z z`), elliptical arcs converted
to cubic Béziers per SVG 1.1 §F.6.5/§F.6.6. **Refuses** unknown
elements, unknown path commands, malformed numbers, and bad arc flags
rather than silently drawing a wrong glyph — same refuse-not-guess
discipline as the PDF parser. Mask-plus-tint rendering per
`docs/ui_specs/icon-set-and-toolbar.md` §6: one white-on-transparent
raster per icon, tinted at draw time (tint is deliberately NOT part of
the cache key, so the same raster serves every color state). Cache
keyed `(icon, physical px, weight)`, with physical px derived from
`pixels_per_point()` so icons stay crisp under HiDPI scaling.

**Gotcha with a pinned regression test:** SVG arc-command flags must be
lexed as single CHARACTERS, not via the general number lexer —
ScripTree's own `link.svg` is written `a6 6 0 008 8`, where a naive
number-grabber reads `008` as the single number `8` and silently
misparses the whole arc. Filed generalizably to
`D:\dev\rag\egui\svg_arc_flag_single_char_lexing_not_number.md`.

**New `crates/pdfce-gui/assets/icons/`** — 35 SVGs + `PROVENANCE.md`
recording the confirmation Ken gave 2026-08-01 (continuation 54).
**8 copied verbatim from ScripTree** (the `folder` icon is reused for
both the Open and Font-folder controls per ui-spec §3.5 — document,
edit, tool, ruler, link, scissors, upload), **2 derived** (zoom-in/
zoom-out reuse icon-search's circle+handle shape), **25 authored new**
in the same 48×48 viewBox / `stroke="currentColor"` / stroke-width
2.5 / round-cap-join contract the ScripTree originals use. `redact.svg`
remains the ONE deliberately solid-filled glyph in an otherwise
all-outline set (ui-spec §8.1's rule-based exception — an outline-only
icon would understate that redaction irreversibly removes content, not
just masks it), with a test asserting it stays the only solid icon so
a future "style cleanup" pass cannot quietly outline it away.

**`main.rs` / `ui_text.rs`:** `icon_button` now takes an `Icon`;
`glyph_button`/`icon_toggle`/`icon_text_toggle`/`icon_text`/
`selected_icon_ring` all now share ONE `labeled_icon_button` body, so
the AccessKit accessible-name-override logic exists in exactly one
place instead of five near-duplicates. Emoji/glyph prefixes stripped
from 7 `ui_text.rs` labels; 11 now-surfaceless glyph-only functions
deleted, each with a block comment recording what was removed and why
(audit trail for "why did this function disappear").

**Toolbar overflow — solved by WRAPPING, not an overflow menu, decision
recorded in-code.** An overflow ("...") menu needs to know what didn't
fit BEFORE layout runs, which in egui's immediate-mode model means
either a frame of visible lag on every resize, or a hard-coded control
priority list that silently rots as future Passes add toolbar controls
— and even done perfectly, it still hides controls behind an extra
click, which is itself a discoverability regression (this is part of
why the operator originally couldn't find the dimensioning tool's
scale-entry controls — see the ENGINEER-VERIFIED OBSERVATION below).
Wrapping is the only option where **nothing is ever hidden**, so
standing rule R83 ("no affordance without the capability" / "never
unreachable without a visible cue") holds structurally rather than by
convention. **A second, self-inflicted bug found by running the app
and fixed in the same commit:** plain wrapping hands each widget only
the width remaining on its current line, so at 640pt window width the
`Measure` button's label rendered ONE CHARACTER PER LINE as a tall
column — inflating the toolbar's height and pushing the History and
utility control groups off the bottom of the panel, which is WORSE
than the clipping it was meant to replace. Fixed with `wrap_mode =
Extend` on the toolbar row so egui takes the wrap decision at control
boundaries, not mid-label.

**Pass 18.3 proper — Measure ▾ affordance fix (ui-spec §D), shipped as
part of the same commit.** The dimensioning tool was already confirmed
functional end-to-end (see the Pass 12.M2c Backlog cluster and the
Pass 17.0 diagnosis); this closes the *discoverability* gap: a real
icon (`icon-ruler.svg`) now reaches the Measure ▾ control per the
icon-set mapping, resolving the sequencing note left open in the ★
Pass 18.x Next-up entry.

**Gates (engineer re-ran ALL of these in the main tree, not just the
autonomous builder's worktree):** `cargo test --workspace` **1504
passed, 0 failed** (up from 1474 baseline; `pdfce-gui`'s own bin-test
count went 94 → 124, no test was removed to get there); `cargo fmt
--all --check` clean; `cargo clippy --workspace --all-targets -D
warnings` clean; `cargo tree -p pdfce-core` / `-p pdfce-render`
verified free of `egui`/`eframe`/`winit`/`wgpu`/`glow`/`accesskit`
(GUI-core separation invariant intact); **zero new Cargo dependencies**
(icon rendering rides the tiny-skia dependency pdfce already had).

**ENGINEER-VERIFIED OBSERVATION (standing rule R86 satisfied at the
toolbar level, first time by direct observation rather than headless
proof alone).** A screenshot of the running release build at the
default 1116pt window width shows the toolbar wrapped to two rows
with **all 27 controls visible and reachable, each with an icon**.
Before this change, at the same window width the toolbar was CLIPPED
after `□ Aa` and `Obj`, with `Measure ▾`, `Tools`, and print entirely
off-screen and **no affordance indicating anything was missing** —
which is part of why the operator originally reported the
dimensioning tool "didn't seem to have a way to actually set the
dimensions": the scale-entry control was never rendered on screen at
all, not merely hard to notice.

**UI-spec defects found during the build (recorded so the spec gets
corrected, not silently deviated from — `docs/ui_specs/
icon-set-and-toolbar.md` is `pdfce-ui-specialist`'s document, flagged
here for its next pass, not edited by the engineer or librarian):**
1. §4.1's icon-size guidance is internally self-contradictory — 18–20px
   inside a 28×24pt button with egui's `(4,1)` `button_padding` leaves
   ~1pt of padding, not the "few px of padding on every side" the same
   sentence asks for. Built at 16pt instead (`icons::ICON_PTS`), with
   the deviation recorded in both the code and `PROVENANCE.md` §5.
2. §1.2's heading claims "two files that do not match the contract —
   flagged, not silently used," but its own body calls the variance
   "trivial, immaterial." Direct inspection of every reused ScripTree
   source found no file that actually deviates from the contract —
   nothing was excluded on §1.2's account; the heading appears to
   overstate what the body found.
3. §2 predates the toolbar's Pass 9c-min "Obj" vector-edit toggle,
   which shipped after the icon spec and had no icon mapping.
   `edit-objects.svg` was authored in-contract rather than leaving one
   toolbar control on a bare glyph forever — recorded as a deviation
   from, not an execution of, §2's original mapping.
4. §3 assigns no icon to the object rail's ▲/▼ reorder arrows; left as
   plain glyphs routed through `glyph_button` so their accessible names
   are unaffected either way.
5. An autonomous builder working in a worktree branched BEFORE `f9bb560`
   reported that `docs/decisions/017` and standing rule R84 "do not
   exist." They DO exist in the main tree — the worktree simply
   predated the commit that added them (see
   `D:\dev\rag\rust\autonomous_builder_worktree_isolation_uncommitted_substrate.md`,
   a recurring pattern on this project). No action needed beyond noting
   it; the icon build's substance was implemented from ui-spec §5.3
   regardless of the stale worktree's confusion.

**Open defect DISPATCHED, not yet fixed — filed here so it isn't
lost:** the down-caret glyph `▾` (U+25BE) has NO glyph in egui's
bundled fonts and renders as a tofu box on the `Markup □`, `Text □`,
`Measure □`, and Copy's `⧉ □` controls. **Pre-existing, not a
regression** — confirmed by stashing this Pass's change and rebuilding
the baseline, which shows the same tofu box. `pdfce-ui-specialist` has
been dispatched to adjudicate the fix and to audit `ui_text.rs` for
other unrenderable codepoints project-wide; it will produce
`docs/ui_specs/menu-affordance-and-glyph-coverage.md`. Track as an open
item until that spec lands and is built.

**RAG filings this session (both ecosystem-wide, `D:\dev\rag\egui\`):**
`eframe_blank_until_first_input_reactive_repaint.md` (eframe presents
nothing until it receives real input — a screenshot harness must drive
a synthetic input event first, or it photographs an unpresented frame
and false-alarms a broken UI; cost the builder several diagnostic
cycles and the engineer one wasted black-screenshot capture) and
`svg_arc_flag_single_char_lexing_not_number.md` (see the gotcha above).
A third finding — `HashMap` iteration order drifting between separate
runs of the same binary, root cause of the `tools/roundtrip` R38
census-drift bug fixed in `f9bb560` — was filed to
`D:\dev\rag\rust\hashmap_iteration_order_drifts_between_runs_of_same_binary.md`.

### `.gitattributes` ordering fix — CRLF corruption of binary PDF fixtures (repo-integrity fix) — 2026-08-02, committed `b73604d`

**Not a feature Pass — a repo-integrity incident found and fixed this
session.** `.gitattributes` applies the LAST matching pattern, and
`* text=auto` sat BELOW `*.pdf binary`, so the catch-all won —
`git check-attr text -- foo.pdf` reported `text: auto` despite the
specific `binary` rule existing above it. Two corruption surfaces, not
one: **69 fixtures** were CRLF-inflated on any checkout with
`autocrlf=true` (recoverable — just re-checkout after the fix), but
**four were damaged in the index itself** because git stripped CRs at
`git add` time, baking the corruption into the committed blob:
`hello.pdf` (703→709 bytes), `minimal.pdf` (331→335 bytes),
`xref-eol/entry-crlf.pdf` (551→556 bytes), `xref-eol/struct-crlf.pdf`
(551→563 bytes). The last two exist SPECIFICALLY to test CRLF handling
in cross-reference sections, where ISO 32000-1 §7.5.4 mandates exactly
20-byte xref entries — normalization turned six entries into 19 bytes
each, so the bytes under test were exactly the bytes destroyed, and the
tests asserting CRLF handling were unknowingly passing against a file
that no longer contained CRLF. Neither state actually worked (the index
form failed `BadEntry`, the checkout-smudged form failed
`NotAnXrefSection`). **The main working tree only kept passing because
it still held the original on-disk bytes and had never been
re-checked-out** — a fresh clone of the prior commit fails 2 integration
tests + 2 doctests. Fixed by moving `* text=auto` to the top of
`.gitattributes` (incident recorded in the file itself), then
`git add --renormalize .` (5 files restaged, now byte-identical to
disk). **Verified by creating a fresh `git worktree`:** fixtures check
out byte-identical to the known-good source and
`cargo test -p pdfce-core --test xref_eol` passes 9/9 there. Filed a
generalizable finding to `D:\dev\rag\rust\
gitattributes_last_match_wins_ordering_corrupts_index.md` (distinct from
the existing `gitattributes_binary_fixtures.md` finding — that one is a
missing-rule/NUL-misdetection failure mode, this one is an
ordering/index-corruption failure mode).

### Pass 18.2 — `object-list` CLI subcommand + headless hit-test query (`--hit`/`--tolerance`) — 2026-08-02, committed `dae0139`

Closes a real, operator-facing gap: `object-move`'s own help text told
operators to get object indices from `object-list`, which had never
existed — leaving three subcommands (`object-move`, `object-delete`,
`node-move`) taking indices nobody could actually discover. Uses the
same `decompose_page` walk the GUI's `object_provider.rs` uses, with a
test pinning that the index `object-list` prints for a given object is
the SAME index `object-move` consumes for it (no drift between the two
call sites' walk order). Surfaces `dropped_objects`/`dropped_nodes`
counts explicitly, because a silent cap would otherwise shift every
later index past the drop point without any way to notice. Also adds
the `--hit`/`--tolerance` headless hit-test query used to produce the
diagnosis below. **Convention note:** `--page` is **1-based**, matching
every existing subcommand's convention — the task brief that spawned
this build said 0-based and was wrong; the implementation correctly
followed the established CLI convention instead of the (incorrect)
brief.

### Selection-outline feedback for the Obj (vector-edit) tool — second independent cause of "I don't seem to be able to click on objects" — 2026-08-02, committed `c998521`

**The object-edit tool drew NO selection feedback at all — a SECOND,
independent root cause of the operator's original complaint, on top of
the zoom-inverted tolerance bug fixed at Pass 18.0.** The
selection-outline drawing code lived inline in the plain-selection
`else` branch of the canvas paint routine; `run_vector_edit_tool`
returns before that branch runs, so the outline was structurally
unreachable whenever the Obj tool owned the canvas. Concretely: clicking
DID select the object, DID arm Delete, and DID arm drag-to-move — it
just drew nothing, so from the operator's chair every click looked like
a miss. Extracted to a shared `draw_selection_outlines` function, called
from both the plain-selection path and the vector-edit path (drawn last
in the vector-edit path, so it isn't painted under other overlay
content). **Record explicitly: the operator's one sentence had TWO
distinct, independently-fixed causes** — the zoom-inverted screen/page
tolerance (Pass 18.0, `9a68d6f`) and this missing-feedback bug
(`c998521`). Fixing either alone would have left the complaint only
partially resolved.

### GUI observation harness — `tools/observe-gui.ps1` + `tools/gui-click.ps1` — 2026-08-02, committed `f2d5fae`

Screenshot-the-live-window (`observe-gui.ps1`) and synthetic-pointer-
input-in-window-relative-coordinates (`gui-click.ps1`) tooling, built to
let the engineer visually verify GUI fixes without depending on the
operator's own eyes for every iteration. **Both REFUSE to act unless the
target is verifiably the FOREGROUND window** — added after an unrelated
terminal window came to the front mid-sequence and the capture
photographed IT while the synthetic clicks landed in IT, silently
producing a "successful" but meaningless observation. Also gitignored
`tools/observations/` (screenshots of arbitrary open documents would
bypass `LEGAL.md` §5's fixture-provenance rules — an ad hoc screenshot
is not a rights-cleared synthetic fixture) and
`fixtures/external-roundtrip.tsv`. **Known limitation found using this
harness, not fixed by it:** synthetic OS-level pointer input reaches
toolbar buttons (verified: the Obj toggle visibly highlights) but does
NOT satisfy egui's `Response::clicked()` on the canvas `Image` widget —
see the diagnosis note below and the new
`D:\dev\rag\egui\synthetic_os_pointer_input_not_response_clicked.md`
finding. Consequence: this harness can observe toolbar-level state but
cannot yet drive or verify canvas gestures; an `egui_kittest`-based
harness is the recommended follow-up (new Backlog entry, below).

### Pass 17.0 — The canvas renders the edited document (decision 018, live-edit rendering) — 2026-08-02, committed `3a56b55`

**Fixes the ★★★★ HEADLINE FINDING below: the GUI now renders
`EditSession`'s live overlay, not just the base revision.** Promoted
`pdfce_core::pageops::assemble::DocumentView` to a new top-level
`pdfce_core::view` module (re-exported from `pageops` for the existing
call sites); added `StreamSource { Contiguous | Split { base, staged } }`
for zero-copy dispatch of a `ByteSpan` against either a plain buffer or
an `EditSession`'s base+staging pair, with a proven (tested)
non-straddling invariant — a span can never cross the base/staged
boundary, by construction of the staging offset scheme. `impl
ObjectGraph for DocumentView` is the delegating implementation that kept
45 of `pdfce-render`'s 50 `Document`-surface call sites compiling
UNCHANGED. Added `EditSession::view()` / `Document::view()`; generalized
`ContentStream::from_page` and `pdfce_core::vector::decompose_page` over
`&DocumentView`. In `pdfce-render`: 27 `&Document` params became
`&DocumentView`. **`render_page`/`render_page_with` kept as thin
`&Document` wrappers** (unchanged signatures) so `pdfce-cli`,
`tools/roundtrip`, `tools/font-parity`, and `tools/render-parity` needed
NO changes. In `pdfce-gui`: two call sites now pass `&self.session.view()`
(`OpenDoc::rasterize_current`, `ensure_object_provider`). New
integration test `edited_view_is_what_renders.rs` (4 tests) proves an
added object is present via the session view and absent via the base,
and that rasters differ once staging is non-empty.

**Two deviations from decision 018's original plan, both handled in
this Pass, not deferred:**
1. `image_codec::decode_image` also threads a `&Document` parameter and
   had to be generalized the same way (same delegating-wrapper trick,
   zero external call-site churn) — the decision record's "3 methods /
   50 call sites" count did not enumerate this call path separately.
2. `DocumentView::bytes()` had to become `Option<&[u8]>` rather than
   `&[u8]`, because a `Split` view has no single contiguous buffer to
   return — returning either half under the old `&[u8]` signature would
   be a plausible-looking but silently WRONG slice (a caller reading
   "the document's bytes" would get only the base OR only the staged
   half, never both, with no compiler signal that anything changed).

**Decision 018 §10 hazard 2 confirmed REAL, not merely theoretical:**
the `refresh_pages` commit-site audit required by this Pass found THREE
canvas call sites doing genuine content-stream surgery WITHOUT calling
it — vector `Commit::Move`, `Commit::Node`, and `delete_selected_object`.
They called `ensure_object_provider` instead, which early-returns while
`provider_page` still equals the current page, so the provider was never
rebuilt and `page_texture` was never dropped. This was invisible before
Pass 17.0 shipped, because the canvas drew the base revision regardless
of whether the provider was stale. **Fixed as part of this Pass** — had
it not been caught here, Pass 17.0 would have shipped looking broken
specifically on the canvas, the one place it was supposed to fix.

**Gates:** workspace **1474 tests passing, 0 failed**; `cargo fmt
--check` clean; `cargo clippy --workspace --all-targets -D warnings`
clean; `cargo tree -p pdfce-core` / `-p pdfce-render` verified
GUI-dependency-free (invariant intact); **zero new Cargo dependencies**.
Roundtrip corpus sweep over **4,023 files**, identical to baseline on
every verdict count and the §5 gate — proving the read-path change
perturbed no writer behavior; raster oracle **6566/6566**.

**Diagnosis closing the operator's original complaint (headless proof,
this session):** hit-testing on `fixtures/synthetic/dimension/
linear-base.pdf` (content `1 w 100 200 m 300 200 l S`, a zero-height
bounding box — the degenerate case most likely to break) succeeds even
at **tolerance 0** (the 1pt stroke's own half-width IS the hittable
band) via the new `object-list --hit --tolerance` CLI query (Pass 18.2),
and correctly MISSES a point 3pt off the line at tolerance 0.5. Core's
`hit.rs` thresholds a stroke-only path at `stroke_half_width +
tolerance` against the flattened polyline, so correctness never depended
on a fill rule or a non-degenerate bbox. **No core hit-testing bug —
hit-testing was always correct.** Combined with Pass 18.0 (tolerance
units) and the selection-outline fix above (missing visual feedback),
the operator's original "I don't seem to be able to click on objects"
report is now FULLY explained: it had **three** contributing factors —
wrong tolerance units, missing feedback, AND the base-vs-edited read
path — and all three are now fixed.

### Pass 18.0 — Zoom-invariant selection tolerance + gesture-preserving zoom (GUI bug-fix; root-caused by `pdfce-ui-specialist`'s `docs/ui_specs/pass-17-dock-and-layer-tree.md` §0.1) — 2026-08-02, committed `9a68d6f`

**Direct fix for the operator's verbatim complaint — "If I click the one
to edit objects, I don't seem to be able to click on objects."** First
shipped slice of the Pass 18.x family (tabbed dock + layer tree, see
the ★ Pass 18.x entry under Next up) — the P0 item from the ui-spec's
own priority table, pulled forward and shipped standalone because it
was small, root-caused, and blocking every other troubleshooting fix
in that spec. **CORRECTION (this session, continuation 56): this work
was previously recorded here as "uncommitted" — that was stale. It is
committed as `9a68d6f`, on top of HEAD `0569373` on branch
`pass-8-redaction`, as is everything else shipped this session (see the
five entries immediately above, newest first).**

**Fix 1 — zoom-invariant selection tolerance.** `object_provider.rs`'s
`SELECT_TOLERANCE = 3.0` was a fixed **canvas-space** (device-space at
zoom 1.0) value, but the click point reaching it had already been
divided by `zoom` (`viewer::screen_to_page`'s documented
1/zoom scaling law) — so the real on-screen catch radius was
`3.0 × zoom` px: 1.5 px at 50% zoom, 0.75 px at 25%, i.e. **zooming
OUT to see a whole page before clicking something made clicking
HARDER**, the inverse of every other viewer's behavior. Root cause
confirmed by reading `object_provider.rs:219` + `canvas.rs`'s own
`screen_to_page_distance_scales_as_one_over_zoom` law. Fix: reuse the
already-shipped, already-tested snap-engine solution
(`canvas.rs:715/732`'s `SNAP_SCREEN_TOLERANCE_PX` /
`screen_tolerance_to_page`) rather than reinventing it —
`CanvasTargetProvider::hit_test` now takes a `tolerance` parameter;
callers pass `screen_tolerance_to_page(SELECT_SCREEN_TOLERANCE_PX,
zoom)` (new constant, 6.0 px — deliberately tighter than the snap
engine's 10.0 px, because a wrong snap is visible and cyclable while a
wrong selection is a silent wrong answer). A degenerate tolerance
(non-finite/negative zoom) falls back to the old fixed 3.0 rather than
disabling selection outright. Files: `canvas.rs`, `object_provider.rs`,
`main.rs` (4 call sites).

**Fix 2 — zooming no longer discards an in-progress tool gesture.**
`resolve_gesture_interrupt` discarded the in-progress gesture for
every action except tool-select/cancel — so picking point A of a
dimension, then Ctrl+scrolling to zoom in for an accurate point B,
silently threw point A away. New `PdfceApp::action_preserves_gesture`
allow-lists pure camera changes (ZoomIn/ZoomOut/ZoomBy/
ZoomActualSize/Fit) on the rule that changing HOW the page is viewed
must not disturb WHAT is being authored. Page navigation deliberately
still discards a gesture (`MeasureState` is per-page).

**Gates:** workspace 33 test suites green (pdfce-gui 87 → 94, pdfce-core
826); `cargo fmt --all` applied; `cargo clippy --workspace --all-targets
-D warnings` clean; `cargo tree -p pdfce-core` / `-p pdfce-render`
verified GUI-dependency-free (invariant intact); **zero new Cargo
dependencies**; release GUI rebuilt and relaunched with
`fixtures/synthetic/vector/mixed.pdf` loaded. New tests: a
selection-tolerance regression (near-miss hairline misses at tight
tolerance, hits at forgiving), zoom-invariance across 0.25×–4×, and
both halves of the gesture-preservation contract.

**What this does NOT fix (see the ★★★★ HEADLINE FINDING above "In
progress"):** even with a correctly-sized catch radius, the object
being clicked was, until decision 018 lands (Pass 17.x), always the
BASE-revision geometry — any object authored via an editing feature in
the current session (a dimension, a moved/deleted vector object, added
text, a markup annotation) is invisible AND unclickable regardless of
this fix, because the canvas rasterizes and hit-tests
`session.document()`, not the edited view. This Pass fixes the
*tolerance*; decision 018 fixes the *read path*. Both were needed for
the operator's full complaint to be resolved.

**Filed by:** `pdfce-librarian`, this session, per explicit engineer
dispatch (§6 of the dispatch) — Pass ID `18.0` librarian-assigned (see
the ★ Pass 18.x entry under Next up for the numbering rationale: the
ui-spec's own filename, `pass-17-dock-and-layer-tree.md`, predates
decision 018 claiming "Pass 17" for the live-edit-rendering fix — the
whole ui-spec family is therefore renumbered to Pass 18.x, this being
its first shipped slice).

### Pass 12.M2b — On-canvas dimension authoring gesture (decision 011 slice 5 of 6 in practice — the deferred GUI slice from Pass 12.M2) — 2026-08-01

**Deferred GUI-authoring gesture from Pass 12.M2's judgment call 1, closing
the "completely functional in the GUI" requirement — SHIPPED and COMMITTED
as `7c93cc3`** (on top of `c7c1744` Pass 12.M2 / `6150e1a` docs / `801a748`
Pass 12.M1 / `19ed865` docs / `e13f3e6` Pass 9a / `79d1c6f` MIT-license
commit / `d8b3903` first implementation commit). Independently re-verified
green in the main tree: gui **87/87** tests, core **811/811** tests
passing; `cargo fmt --all --check` clean; `cargo clippy --workspace
--all-targets -D warnings` clean; `cargo tree -p pdfce-core` /
`-p pdfce-render` GUI-dep-free (invariant intact); **zero new Cargo
dependencies**; GUI release build launches (pid 46476).

This is the on-canvas click-point-A-then-click-point-B authoring gesture
deferred at Pass 12.M2's ship — closing the last gap between "dimensions
fully authorable via CLI" and "completely functional in the GUI."

- **NEW `crates/pdfce-gui/src/measure_tool.rs`** — pure, headless-tested
  authoring state machines (19 tests). **CHANGED** `main.rs`
  (`run_measure_tool`/`run_dimension_groups_panel`/`scale_entry_widget`,
  tool entry/teardown, Escape/gesture-interrupt handling, "Manage
  Dimension Groups…" menu item), `object_provider.rs`
  (`object_sample_points` accessor — the one per-page geometry
  decomposition now feeds selection + snap + circle-fit), `ui_text.rs`.
- **Three gestures, each reusing the shipped 12.M2 engine + 12.M1 snap
  indicator + the 14.3/15.2/16.2 preview/Accept idiom:**
  - **MeasureLinear** — click point A (snapped) → live axis/free-
    constrained preview with scaled readout → click point B → Accept →
    `EditSession::add_dimension`.
  - **MeasureCircular** — toggle pick-set of points on an arc → live
    `fit_circle_taubin` preview with fit residual shown → Accept →
    radius/diameter dimension (display-only kind toggle, per 12.M2's
    judgment call 3).
  - **MeasureScale** — reference-line pick → scale-entry dialog
    (real-length or ratio) → `set_group_scale` re-propagates to the
    active group's existing dimensions.
  - **Dimension-groups panel** — create group / set scale+units+format /
    toggle per-group layer visibility / select active group, all via the
    shipped `EditSession` methods (`add_dimension_group`,
    `set_group_scale`, `toggle_dimension_layer`).
- **Canvas==CLI equivalence PROVEN, not merely asserted:**
  `gui_linear_kind_equals_cli_linear_kind` and
  `gui_circular_kind_equals_cli_circular_kind` confirm GUI-authored
  gestures produce the identical `DimensionKind` the CLI's `dimension-add`
  builds — the additive `/Line`+`/IT`+`/PieceInfo`+OCG bytes therefore
  match by construction, not by two independent code paths that merely
  happen to agree today.

**Engineer judgment calls made this Pass (recorded):**
1. **Raw second point stored for the linear gesture** (the constrained/
   snapped line shown during preview is display-only) — required for
   CLI byte-equivalence; storing the constrained point instead would
   diverge from what `dimension-add --points` records.
2. **Group rename/delete NOT implemented this Pass** — needs core
   sidecar-rewrite support; implementing it GUI-side only would push
   storage logic into the GUI, violating the GUI-core separation
   invariant. Named explicitly here as a follow-up, not silently
   dropped.
3. **Ctrl+Shift+D chord left unbound** — unverified-unclaimed; no
   keybinding-conflict check has been run against it, so it is left
   unwired rather than wired speculatively.
4. **Fixed a pre-existing doc-comment/`#[allow]` misattachment** between
   `run_measure_tool` and `run_add_text_tool` (left over from Pass 16.2)
   while wiring the new tool into the same region of `main.rs`.

**Gates (re-verified in the main tree):** gui **87/87** tests; core
**811/811** tests; `cargo fmt --all --check` clean; `cargo clippy
--workspace --all-targets -D warnings` clean; `cargo tree -p pdfce-core`
/ `-p pdfce-render` GUI-dep-free; **zero new Cargo dependencies**; GUI
release build launches (pid 46476). **Committed as `7c93cc3`**, on top
of `6150e1a` (docs), `c7c1744` (Pass 12.M2), `801a748` (Pass 12.M1),
`19ed865` (docs), `e13f3e6` (Pass 9a), `79d1c6f` (MIT license artifacts),
and `d8b3903` (first implementation commit) — still **local-only**, same
unpushed posture as all prior commits (push authorization remains a
separate, not-yet-granted operator item).

**MILESTONE — the operator's directed #1 priority, "get the dimensioning
tool completely functional in the gui interface," is now SUBSTANTIALLY
MET.** Decision 011's dimensioning capability is complete end-to-end in
the GUI: the Pass 12.0 canvas substrate, Pass 9a's object model, Pass
12.M1's snapping engine, Pass 12.M2's dimensioning/scale/storage engine,
and this Pass's on-canvas authoring gesture combine so an operator can
draw linear/radius/diameter/scale dimensions by clicking (snapped),
manage named groups with per-group scale/units (including architectural
feet-inches), and toggle per-group layer visibility — all on the canvas,
with several capabilities that exceed Acrobat (see the Pass 12.M2 Shipped
entry above).

**Pass 9c-min — basic vector editing (move/delete/drag-node) — SHIPPED and
COMMITTED as `76485b5`, 2026-08-01.** Content-stream SURGERY (the R46/§5.7
named exception, mirror of redaction) on Pass 9a's object model, riding
Pass 11's independent render-fidelity oracle (R59): `vector::edit`
plan_move/plan_delete/plan_move_node reuse the Pass 8.0 REPLACE substrate
(splice + emit_number + make_raw_stream), rewriting/removing path
operands and re-emitting ONLY the edited stream; move uses the linear CTM
inverse (`Matrix::inverse`/`map_vector`) for the page→user delta; node
ordering replays decompose's exact anchor bookkeeping (agree-by-
construction). `EditSession::{move_object,delete_object,move_node}` +
CommandKinds; CLI `object-move`/`object-delete`/`node-move` (--verify-undo);
`CanvasTool::VectorEdit`. Gates: 33 suites green; content-identity = exactly
one changed stream; §5.7 objstm promotion (no stale copy); R59 faithful;
undo byte-identical; fuzz 400k/0 crashes; cargo tree core+render
GUI-dep-free; zero new deps. Named limits: single edit per GUI session
(base-content decomposition — a second same-session edit is refused with
`VectorEditNeedsReopen`, never misindexed); rect-corner / control-handle /
text-image node editing stay full Pass 9.

**★ MILESTONE — decision 011's measurement/editing beta is COMPLETE
(2026-08-01).** All six slices shipped and committed
(`12.0 → 9a → 12.M1 → 12.M2 → 12.M2b → 9c-min`). The operator's #1 directed
priority ("get the dimensioning tool completely functional in the GUI") is
fully met: draw/measure/scale/group/layer dimensions AND move/delete/
drag-node edit existing vector objects — all on the canvas. Commit chain:
`d8b3903 → 79d1c6f MIT → e13f3e6 9a → 19ed865 docs → 801a748 12.M1 →
c7c1744 12.M2 → 6150e1a docs → 7c93cc3 12.M2b → 2abbd75 test-flake fix →
dd3a8b8 §12 backfill → 76485b5 9c-min`. **NOTE: the session subagent budget
(200) is exhausted — subsequent work is done directly, not delegated,
unless the operator raises `CLAUDE_CODE_MAX_SUBAGENTS_PER_SESSION`.**

### Pass 12.M2 — Dimensioning + scale/group + hybrid storage + OCG layer (decision 011 slice 4 of 5, THE HEADLINE CAPABILITY) — 2026-08-01

**Fourth slice of decision 011's five-slice dimensioning-tool architecture
(`12.0 → 9a → 12.M1 → 12.M2 → 9c-min`) — SHIPPED and COMMITTED as
`c7c1744`** (on top of `801a748` Pass 12.M1 / `19ed865` docs / `e13f3e6`
Pass 9a / `79d1c6f` MIT-license commit / `d8b3903` first implementation
commit). Independently re-verified green in the main tree: core
`dimension` module **39 unit tests + 6 round-trip tests**, all green;
full workspace `cargo test` — **1389** tests passing; `cargo fmt --all
--check` clean; `cargo clippy --workspace --all-targets -D warnings`
clean; `cargo tree -p pdfce-core` / `-p pdfce-render` GUI-dep-free
(invariant intact); **zero new Cargo dependencies**; R59 render-fidelity
check agrees with pdfium (the one deliberate, documented OCG-honoring
divergence is noted below — NOT a fidelity defect); additive
existing-content byte-verbatim; undo byte-identical. A live CLI smoke
test (`dimension-add`) authored a linear dimension with round-trip
`identical=1, raster_identical=1`.

This is the headline capability of the beta — the actual measurement/
dimensioning authoring engine (fit, units/scale, storage, layer
visibility), not just the substrate (12.0/9a/12.M1) it's built on.

- **NEW `crates/pdfce-core/src/dimension/`** (`mod`, `fit` [Taubin],
  `units`, `group`, `measure_dict`, `author`, `sidecar`). **CHANGED**
  `edit.rs` (`add_dimension`/`add_dimension_group`/`set_group_scale`/
  `toggle_dimension_layer`/`dimension_model` + 3 new `CommandKind`s),
  `annot.rs` (`Annotation.oc` + `optional_content_default_off`/
  `oc_is_hidden`), `render/annot.rs` (OCG visibility gate in
  `survey_page_annotations`), `cli/main.rs` (6 subcommands),
  `gui/{canvas, ui_text, main}` (3 Measure `CanvasTool` variants +
  "Measure ▾" menu + status overlay), `lib.rs` (`pub mod dimension`).
  Tests: `dimension_roundtrip.rs` (6) + 39 unit. Fixtures
  `fixtures/synthetic/dimension/` + generator.
- **Taubin best-fit circle** (hand-rolled Chernov variant, chosen for
  the short-arc regime pdfce's dimensioning tool actually hits):
  `taubin_beats_kasa_on_short_arcs` (1200 trials, 90° arc, r=100,
  σ=1.5) proves Taubin bias <1.5% AND less than Kåsa's; a real-file fit
  recovered r=100.00 exactly on the 12-segment short-arc fixture.
  **Radius/diameter dimensioning EXCEEDS Acrobat** — Acrobat has no
  equivalent baseline to match against.
- **Units/scale:** 6 units including architectural **feet-inches**
  (144pt @ 12.5ft → 12'-6", spec §12.9 Table 263) — EXCEEDS Acrobat.
  **Tri-state `ScaleState`** (`NeverSet`/`OneToOne`/`Calibrated` —
  deliberately never collapsed to `Option<f64>`, so "explicitly 1:1"
  and "never set" stay distinguishable states). Both entry paths
  supported: real-length (`L`/`D`) and ratio (`N:M × basis`). Scale is
  authoritative from the `/X` array's first `/C` entry. **Named
  per-group scale/units EXCEEDS Acrobat's per-viewport-only geometric
  scoping.**
- **Hybrid storage** (three layers, each serving a different
  consumer): native `/Line` + `/IT /LineDimension` + baked `/AP`
  (renders correctly in any PDF viewer); per-annotation `/Measure`
  mirror (interop convenience — NOT spec-guaranteed to survive a
  round-trip through another tool, since `/PieceInfo` cross-tool
  survival is likewise not spec-guaranteed); authoritative §14.5
  `/PieceInfo /pdfce` sidecar (pdfce's own source of truth for
  group/scale/id). Foreign `/PieceInfo` keys and existing OCGs are
  preserved untouched. All of this is additive — existing content
  bytes are re-emitted byte-verbatim. Per-group §8.11 OCG registered in
  `/OCProperties /D` (default-hidden via `/D /OFF`); each dimension
  annotation's `/OC` points at its group's OCG; render honors
  annotation-level `/OC` (content-stream BDC/EMC-level OCG honoring is
  deferred — out of scope for annotation-only dimensioning).
- **Public API (rule-10 trail):** `dimension::{fit_circle_taubin,
  fit_circle_taubin_refined, FitCircle, Unit, NumberFormat,
  FractionMode, ScaleState, ScaleEntry, ScalePreview,
  MeasurementDisplay, preview_group_scale, format_measurement, Group,
  GroupId, DimensionId, DimensionKind, DimensionRecord, DimensionModel,
  DEFAULT_GROUP_ID, AuthoredDimension, author_dimension,
  build_measure_dict, build_ocg, build_ocproperties, serialize_model,
  deserialize_model}`; `EditSession::{add_dimension,
  add_dimension_group, set_group_scale, toggle_dimension_layer,
  dimension_model}`; `annot::{optional_content_default_off,
  oc_is_hidden}` + `Annotation.oc`. CLI: `dimension-add` (--kind
  linear/radius/diameter --points --group), `dimension-list`,
  `group-add`, `group-set-scale`, `layer-toggle`.

**Engineer judgment calls made this Pass (recorded):**
1. **GUI scope capped at menu + tools + disclosure this Pass — the
   on-canvas snap-pick AUTHORING GESTURE (click point A, click point
   B, on the actual canvas, consuming 12.M1's snap engine) is DEFERRED
   to a follow-up GUI slice, now being built as "Pass 12.M2b —
   on-canvas dimension authoring."** Dimensions are fully authorable
   today via the CLI, and the GUI discloses existing dimensions/
   groups/layers even though it can't yet author new ones on-canvas —
   a disclosed gap, not a silent one (fuzzy-never-sneaky).
2. Per-annotation `/Measure` (not page-level `/Viewport`) — sidesteps
   the overlapping-different-scale-groups geometric-partition problem
   a `/Viewport`-based design would hit; the sidecar remains
   authoritative regardless of which mirror a downstream tool reads.
3. Radius/diameter modeled as one underlying geometry with a
   display-only kind toggle (3 `CanvasTool` variants, not 4) — per
   `pdfce-ui-specialist`'s dimension-tool UX spec §1.1.
4. `/LastModified` uses a fixed placeholder (`D:20260801000000Z`) so
   an unchanged sidecar re-serializes byte-stable; wiring a real clock
   is a trivial follow-up, not a substantively deferred item.
5. Reused the `AddDimension` `CommandKind` for group-add rather than
   inventing a fourth `CommandKind` — group-add is modeled as a
   variant of the same undo-able mutation family.

**Important R59 note (recorded here explicitly so a future session
doesn't "reconcile" it away as a bug):** on `ocg-hidden.pdf`, pdfce
correctly HIDES the OFF-layer dimension (renders only the base line),
while pdfium with `draw_annots=True` paints it regardless of OCG
state. This is pdfce being MORE correct — honoring §8.11.3.3
optional-content visibility — not a fidelity defect against the
pdfium baseline. Documented in the fixture's `PROVENANCE.md`.

**Gates (re-verified in the main tree):** core `dimension` module 39
unit + 6 round-trip tests green; full workspace `cargo test` **1389**
passing; `cargo fmt --all --check` clean; `cargo clippy --workspace
--all-targets -D warnings` clean; `cargo tree -p pdfce-core` /
`-p pdfce-render` GUI-dep-free; **zero new Cargo dependencies**; R59
agrees with pdfium (the one documented, correct OCG-honoring
divergence above); additive existing-content byte-verbatim; undo
byte-identical. Live CLI smoke test: `dimension-add` authored a linear
dimension, round-trip `identical=1, raster_identical=1`. **Committed
as `c7c1744`**, on top of `801a748` (Pass 12.M1), `19ed865` (docs),
`e13f3e6` (Pass 9a), `79d1c6f` (MIT license artifacts), and `d8b3903`
(first implementation commit) — still **local-only**, same unpushed
posture as all prior commits (push authorization remains a separate,
not-yet-granted operator item).

**With this shipped, decision 011's dependency chain is 4 of 5 done
(`12.0 → 9a → 12.M1 → 12.M2`). "Pass 12.M2b" (on-canvas dimension
authoring — the deferred gesture from judgment call 1 above) is
dispatched to build next, ahead of 9c-min, so the dimensioning tool
reaches "completely functional in the GUI" per the operator's
requirement. This effectively splits decision 011's originally-planned
5th-slice gap into two GUI slices in practice (12.M2b then 9c-min) —
a judgment call recorded here, not a librarian-invented resequencing;
decision 011's own document is unchanged. 9c-min (basic vector editing:
move/delete/drag-node) remains the last of decision 011's originally-
named five slices, after 12.M2b — see "In progress" (below) for the
updated Beta state.**

### Pass 12.M1 — Snapping engine + fuzzy snap indicator (decision 011 slice 3 of 5) — 2026-08-01

**Third slice of decision 011's five-slice dimensioning-tool architecture
(`12.0 → 9a → 12.M1 → 12.M2 → 9c-min`) — SHIPPED and COMMITTED as
`801a748`** (on top of `19ed865` docs / `e13f3e6` Pass 9a / `79d1c6f`
MIT-license commit / `d8b3903` first implementation commit).
Independently re-verified green in the main tree: full workspace
`cargo test` — core lib **772** tests (up from 749 at Pass 9a's ship,
**+23 new snap tests**), 70 GUI tests, all passing; `cargo fmt --all
--check` clean; `cargo clippy --workspace --all-targets -D warnings`
clean; `cargo tree -p pdfce-core` / `-p pdfce-render` GUI-dep-free
(invariant intact); **zero new Cargo dependencies**; GUI release
builds and launches.

This is the tool-agnostic snapping service that 12.M2's dimension
picks (next slice) and the eventual 9c-min node-drag both consume,
built directly on Pass 9a's object geometry.

- **NEW `crates/pdfce-core/src/vector/snap.rs`** — `snap_candidates(query,
  &SnapConfig, &PageObjects) -> Vec<SnapCandidate>`. Seven-level
  priority order, deterministic:
  `Node < Endpoint < Center < Midpoint < Intersection <
  DerivedCenterline < SegmentCenterline < Axis` (8 `SnapKind`s
  including the derived filled-quad midline). Tie-break is
  `(priority, distance, x, y, source)`, with coincident-point dedup at
  `1e-3`. H/V axis constraint (`constrained_second_point`/
  `measured_length`) verified correct at 0/90/180/270°. Tolerance is
  zoom-invariant (`px / zoom`, screen-space tolerance converted to
  page-space per current zoom).
  **Intersection-snap defaults OFF and is neighbourhood-bounded**
  (`near_query_segments` bbox pre-filter,
  `MAX_NEIGHBOURHOOD_SEGMENTS = 256`, explicitly no global all-pairs
  intersection search) — the Inkscape-freeze precedent surfaced by
  `pdfce-inkscape-librarian`'s 12.M1 grounding (Z4 risk mitigation,
  cited in-code).
- **pdfce-gui: fuzzy snap indicator.** Per-`SnapKind` marker glyph +
  type label shown pre-commit (never silently applied — fuzzy-never-
  sneaky); Tab cycles ties; Alt overrides/suppresses snapping; a
  master on/off toggle. The derived-centerline candidate gets a
  distinct glyph AND a two-click confirm (higher scrutiny for the one
  candidate kind that's inferred rather than literally-present
  geometry).
- **`ObjectModelProvider::page_objects()`** — the ONE per-page object
  decomposition, now exposed to both selection (Pass 9a) and snapping
  (this Pass). Closes a latent double-decompose risk (Z2-adjacent):
  swapped `OpenDoc`'s boxed `dyn` target-provider field for a concrete
  `object_model` field plus an on-demand `target_provider()` accessor,
  so both consumers share the same decomposition instance instead of
  each holding/rebuilding their own.
- **Marquee-vs-pan UX flag (owed since Pass 9a) — RESOLVED, KEPT.**
  `pdfce-ui-specialist` reviewed the Pass-9a drag-to-marquee-select
  change (replacing drag-to-pan in no-tool selection mode; pan moved
  to wheel/scrollbars, the Inkscape/Illustrator convention, R61) in
  this Pass's dispatch and found no conflict with the dimensioning
  tool: Measure/dimension tools use a click-point-A-then-click-point-B
  interaction, not drag, so marquee-drag and dimension-picking never
  contend for the same gesture. No behavior change from Pass 9a's
  shipped default.
- **Fuzz target** `vector_snap` added.
- **Public API added to `pdfce-core`** (rule-10 trail):
  `pdfce_core::vector::{SnapKind, SnapCandidate, SnapConfig,
  AxisConstraint, snap_candidates, constrained_second_point,
  measured_length, SNAP_FLATTEN_STEPS, MAX_NEIGHBOURHOOD_SEGMENTS,
  MAX_CANDIDATES}`; gui `ObjectModelProvider::page_objects` plus
  canvas snap-indicator helpers.

**Engineer judgment calls made this Pass (recorded):**
1. **Node vs. Endpoint semantics** — `Endpoint` names the free
   terminus of an OPEN subpath; every other subpath vertex is `Node`
   (closed-subpath vertices are always `Node`, never `Endpoint`).
2. **Center** = bbox-centre of a closed, all-cubic subpath — exact for
   circles/ellipses built from the standard 4-cubic kappa
   approximation; a Taubin best-fit circle/ellipse center is the
   12.M2-stage upgrade, same `SnapKind`, not a new one.
3. **`SnapConfig` struct, not a bare function-signature's worth of
   parameters** — carries intersection/grid/axis knobs as one unit.
   The ui-spec (12.M1's `pdfce-ui-specialist` grounding) left the
   exact shape to the engineer; a config struct was chosen over
   positional args for API-guidelines ergonomics (rule 10) and to
   leave room for 12.M2's additional knobs without signature churn.
4. **Bbox-corner snapping is OUT of this Pass** — decision 011 does
   not name it as a required candidate kind; documented here as a
   scoped fast-follow candidate, not silently folded in as a ninth
   `SnapKind`.
5. **Concrete `object_model` field replacing the boxed `dyn`
   target-provider** (see `ObjectModelProvider::page_objects()`
   above) — the only way to honor "one decomposition per page" for
   both consumers; the literal "just add a second provider field"
   reading of the brief would have re-introduced the double-decompose
   risk Pass 9a's cross-check discipline exists to prevent.

**Gates (re-verified in the main tree):** core lib **772** passed / 0
failed (+23 vs. Pass 9a's 749); GUI **70** tests passed; `cargo fmt
--all --check` clean; `cargo clippy --workspace --all-targets -D
warnings` clean; `cargo tree -p pdfce-core` / `-p pdfce-render`
GUI-dep-free (core also confirmed free of `tiny-skia`); GUI release
build launches; **zero new Cargo dependencies** (no `Cargo.toml`/
`Cargo.lock` change). **Committed as `801a748`**, on top of `19ed865`
(docs), `e13f3e6` (Pass 9a), `79d1c6f` (MIT license artifacts), and
`d8b3903` (first implementation commit) — still **local-only**, same
unpushed posture as all prior commits (push authorization remains a
separate, not-yet-granted operator item).

**With this shipped, decision 011's dependency chain is 3 of 5 done
(`12.0 → 9a → 12.M1`); `12.M2` (dimensioning + scale/group + hybrid
storage + OCG layer) is next, dispatched to build this same
continuation — see "In progress" (below) for the updated Beta state.**

### Pass 9a — Read-only vector object/selection model + centerline derivation (decision 011 slice 2 of 5; first BUILDABLE slice atop the Pass 12.0 canvas substrate) — 2026-08-01

**Second slice of decision 011's five-slice dimensioning-tool architecture
(`12.0 → 9a → 12.M1 → 12.M2 → 9c-min`) — SHIPPED and COMMITTED as
`e13f3e6`** (on top of `79d1c6f` MIT-license commit / `d8b3903` first
implementation commit). Independently re-verified green in the main
tree: full workspace `cargo test` all green — core lib **749** tests
(cross-check + provider + fuzz all pass, including **36 new vector**
tests); `cargo fmt --all --check` clean; `cargo clippy --workspace
--all-targets -D warnings` clean; `cargo tree -p pdfce-core` /
`-p pdfce-render` GUI-dep-free (invariant intact); **zero new Cargo
dependencies**.

This is the first BUILDABLE slice on top of Pass 12.0's uninhabited
canvas substrate: a read-only decomposition of a page's content-token
stream into selectable vector/text/image objects, point + marquee
hit-testing, and a thin-filled-bar centerline derivation hint — the
object/selection model that the remaining three decision-011 slices
(12.M1 snapping, 12.M2 dimensioning UI, 9c-min basic editing) plug into.

- **NEW `crates/pdfce-core/src/vector/` module** (`mod.rs`,
  `geometry.rs`, `decompose.rs` (~1000 lines), `hit.rs`,
  `centerline.rs`): read-only decomposition of a page's content-token
  stream into selectable `PathObject`/`TextObject`/`ImageObject` nodes
  in user+page space, carrying the effective graphics state and a
  content-token `TokenRange`/`ByteSpan` editing handle (CAPTURED now,
  not yet consumed — reserved for the 9c-min editing slice); point +
  marquee hit-testing; thin-filled-bar centerline derivation
  (`CENTERLINE_ASPECT_THRESHOLD = 8.0`, midline of the two short
  edges, rotation-correct) — a confirmable dimensioning hint, never
  auto-applied (fuzzy-never-sneaky).
- **CHANGED** `crates/pdfce-core/src/lib.rs` (`pub mod vector`);
  `crates/pdfce-render/src/interpret.rs` (additive
  `trace_paths`/`TracedPath`/`TracedNode` cross-check hook — returns
  `None` on every render/save path, so rendered/written output bytes
  are unchanged); `crates/pdfce-gui/src/{object_provider.rs (new),
  main.rs}` (`ObjectModelProvider: CanvasTargetProvider` + selection
  wiring onto the Pass 12.0 canvas).
- **Fixtures** `fixtures/synthetic/vector/{paths,curves,mixed,
  centerline}.pdf` + generator + `PROVENANCE.md`. **Fuzz** target
  `vector_decompose` (686k execs, 0 crashes).
- **Agree-by-construction (Z2 mitigation).** Object geometry and the
  renderer's actual walk share the same construction/CTM primitives,
  and a cross-check test asserts object geometry matches the render
  point-for-point (including a kappa-approximated circle and a
  `cm`-rotated bar) — the render is the oracle, not a second
  independently-reasoned geometry path.
- **Public API added to `pdfce-core`** (rule-10 API-guidelines trail;
  `decompose` is total — no error type, forwards the existing
  `ContentError`): `pdfce_core::vector::{Point, Matrix, Bounds, Rgb,
  cubic_from_v, cubic_from_y, rect_corners, Segment, Subpath,
  PaintStyle, FillRule, TokenRange, PathObject, ImageSource,
  ImageObject, TextObject, VectorObject, DecomposeDiagnostics,
  PageObjects, XObjectShape, XObjectResolver, NoXObjects,
  DocumentXObjects, decompose, decompose_page, MAX_OBJECTS, MAX_NODES,
  MarqueeMode, FLATTEN_STEPS, hit_test_point, hit_test_rect,
  CENTERLINE_ASPECT_THRESHOLD, CenterlineCandidate, page_candidates,
  derive_from_path}`; render `interpret::{trace_paths, TracedPath,
  TracedNode}`; gui `object_provider::ObjectModelProvider`.

**Engineer judgment calls made this Pass (recorded, some owed follow-up):**
1. Shared-primitives-not-forked-walk agree-by-construction (see above),
   chosen over an independently-reasoned second geometry path, to make
   render/object-model drift structurally impossible rather than
   merely tested-against.
2. **Marquee-vs-pan: canvas drag now means rubber-band marquee select
   in no-tool selection mode; pan moved to wheel/scrollbars.** This is
   a CHANGE to the Pass 12.0 shipped viewer's drag-to-pan default
   (Inkscape/Illustrator convention, standing rule R61). **OWED a
   `pdfce-ui-specialist` review at the 12.M1 stage** — see the new
   Backlog flag below; not actioned this Pass.
3. Marquee selection is fully-contained-by-default (a `Touched` mode
   is available but not the default), matching the same
   Inkscape/Illustrator convention.
4. Text object bounding boxes are approximate (no glyph metrics
   consulted) — bbox-selectable, not node-editable; disclosed as a
   limit, not silently glossed over.
5. A Form XObject `Do` decomposes to ONE opaque bounding-box object
   (no recursion into the form's own content stream) — a scoped-down
   fast-follow, not full nested-XObject decomposition.
6. `XObjectResolver` is a trait (not a concrete resolver), chosen for
   testability — `NoXObjects` and `DocumentXObjects` are the two
   current implementations.
7. Content-stream `n` (clip-only/no-op path-painting) operators
   produce selectable invisible objects rather than being dropped —
   consistent with "never silently discard operator-authored
   content."
8. **No `pdfce-cli` subcommand this Pass** — the model is read-only and
   GUI-internal for now; a `vector-list` inspect subcommand is a
   fast-follow, tracked against the 12.M2/9c-min stage per rule 11.
9. `gs` (ExtGState) line-width is not tracked into the derived
   geometry — a minor, documented stroke-tolerance approximation, not
   a correctness bug for the selection/centerline use case.
10. The GUI provider decomposes the CURRENT page only, and rebuilds
    lazily (not eagerly across the whole document) — a scoped
    performance choice for an interactive canvas, not a document-wide
    cache.

**Gates (re-verified in the main tree):** core lib 749 passed / 0
failed (up from 713 at the FF-D-hardening ship, +36 new vector tests);
cross-check + provider + fuzz all pass; `cargo fmt --all --check`
clean; `cargo clippy --workspace --all-targets -D warnings` clean;
`cargo tree -p pdfce-core` / `-p pdfce-render` GUI-dep-free (invariant
intact); **zero new dependencies** (no `Cargo.toml`/`Cargo.lock`
change). **Committed as `e13f3e6`**, on top of `79d1c6f` (MIT license
artifacts) and `d8b3903` (first implementation commit) — still
**local-only**, same unpushed posture as the two prior commits (push
authorization remains a separate, not-yet-granted operator item).

**With this shipped, decision 011's dependency chain is 2 of 5 done
(`12.0 → 9a`); `12.M1` (snapping engine) is next — see "In progress"
(below) for the updated Beta state.**

### FF-D follow-up hardening — certification-signature guard on `add_text`/`EditSession::add_text` (closes the Pass 16.0 flagged gap) — 2026-08-01

**Correctness hardening, not a new Pass — CLOSES the Backlog "FF-D
follow-up — certification-signature guard gap" entry flagged at Pass
16.0's ship.** Independently re-verified green in the main tree: 15 CLI
`add_text` tests (incl. the certified-refusal cases) pass, core lib 713
pass, and a live `add-text` against the certified fixture is refused
with the verbatim §12.8.4 DocMDP message.

Adding page content to a certified-signed PDF whose enforced
`/Perms /DocMDP` forbids structural changes is now REFUSED, mirroring
`EditSession::add_markup`'s existing guard. Previously `add_text`/
`EditSession::add_text` checked encryption and suppressed-objects but
not certification — a certified (MDP-locked) PDF could accept a
page-text add that a certification signature should have blocked. That
gap is now closed.

- `crates/pdfce-core/src/text_edit/addtext.rs` — new
  `AddTextError::CertificationForbidsChange { permission: u8 }`, its
  `#[error]` message a VERBATIM copy of
  `EditError::CertificationForbidsChange`'s (same wording, same ISO
  32000-1 §12.8.4 `/Perms /DocMDP P=` citation — reused, not
  reinvented; asserted by a message-parity unit test). New shared
  `pub(crate) fn refuse_if_certification_forbids<G: ObjectGraph>(graph)`
  uses the SAME machinery as `EditSession::check_certification`
  (`crate::signature::census` + `SignatureCensus::forbids_structural_change()`
  + the `/P`-absent-defaults-to-2 rule), producing an `AddTextError`.
  Wired into the free `add_text` engine between the encryption and
  suppressed-objects guards (matching `add_markup`'s
  encryption→certification→suppressed order).
- `crates/pdfce-core/src/edit.rs` — `EditSession::add_text` calls the
  same shared guard in the same position; the boxed add shares the
  planner so it is covered automatically (tested).
- `crates/pdfce-cli/src/main.rs` — `cmd_add_text` maps the new variant
  to `exit::EDIT_REFUSED`.
- **Free-function guard posture chosen: (a).** `census`/
  `forbids_structural_change` are already `pub` in `signature.rs` and
  reachable from the free function (`Document: ObjectGraph`), so the
  free `add_text` engine itself refuses via the shared helper — both
  entry points (GUI `EditSession::add_text` and the free `pub`
  function, which the CLI calls) call the SAME helper. This guards
  every operator-reachable path with zero drift; no operator-reachable
  unguarded entry remains. This also discharges Backlog item (2)'s
  "expose a guard hook for other free-function engines" — the shared
  helper IS that hook, positioned so the 15.x reflow engine or any
  future free-function engine can reach it the same way.
- **Fixture** `fixtures/synthetic/addtext/certified-locked.pdf`
  (`plain.pdf` + an enforced `/Perms /DocMDP` P=1 cert sig), added to
  `tools/gen-addtext-fixtures.py` (byte-stable/idempotent,
  md5-confirmed) + `PROVENANCE.md`.
- **Tests:** core (point/box/free-function refused with
  `CertificationForbidsChange { permission: 1 }`, session left
  unmodified; uncertified doc still adds — regression guard;
  message-parity assertion) + CLI (point/box → `EDIT_REFUSED`, stderr
  cites §12.8.4, no output produced).

**Gates (re-verified main tree):** core lib 713 passed / 0 failed; CLI
`add_text` 15 passed; `cargo test --workspace` all green; `cargo fmt
--all --check` clean; `cargo clippy --workspace --all-targets -D
warnings` clean; `cargo tree -p pdfce-core` / `-p pdfce-render`
GUI-dep-free (invariant intact); **NO new dependency** (only
intra-crate references; no `Cargo.toml`/`Cargo.lock` touched); Pass
14.x/15.x/16.x/`vartext` tests unchanged.

**With this closed, the FF-D/text-parity-arc milestone (decisions 014 +
015 + 016) has no known loose threads.** See the ★ Pass 16.x entry
(Next up, below) for the closing amendment, and "In progress" (above
in file order, below in this section) for the updated milestone
framing.

### Pass 16.2 — On-canvas Add-Text UI (decision 016, FF-D slice 3 of 3, FINAL SLICE — decision 016 / FF-D COMPLETE end-to-end) — 2026-08-01

**The third and FINAL slice of the FF-D add-new-page-text subsystem
(decision 016) — SHIPPED. Decision 016 / FF-D is now COMPLETE
end-to-end (16.0 point-text engine → 16.1 boxed/wrap engine → 16.2
canvas UI).** Independently re-verified green in the main tree: 18 core
`add_text` tests (incl. the `preview_wrap`↔`add_text` parity proof) +
59 GUI tests pass; release GUI build succeeds and launches; relaunched
live for the operator (pid 25280) with the Add-Text tool available.

- **P0 pure wrap-preview (core).** NEW
  `pdfce_core::text_edit::preview_wrap(text, wrap_box, page_crop, font:
  Std14, size, alignment, leading) -> Result<AddTextWrapPreview,
  AddTextError>` — GENUINELY factored out of 16.1's boxed layout:
  `layout_boxed` now takes explicit inputs and carries each line's
  original text alongside its emission codes, so `add_text`'s boxed
  path and `preview_wrap` share ONE `layout_boxed` pass (no duplicated
  wrap/origin/overflow math). Pure/read-only — no `&Document`, no
  mutation, no GUI dependency. CHANGED `crates/pdfce-core/src/text_edit/addtext.rs`
  + `mod.rs` (re-exports `preview_wrap`/`AddTextWrapPreview`/
  `WrapPreviewLine`) + `fontdata/mod.rs` (`Std14::ALL`).
- **GUI.** `CanvasTool::AddText` — a SECOND real tool variant, mutually
  exclusive with `CanvasTool::TextEdit` (the opposite call from 15.2's
  reflow-sub-mode approach, reasoned in the `pdfce-ui-specialist` spec
  §0.1) — plus pure placement helpers (`resolve_drag_placement`) and
  `run_add_text_tool` (click→point / drag→box rubber-band / typing→live
  wrap-preview ghost / property bar size+colour+font+alignment /
  Accept→`EditSession::add_text` landing one `CommandKind::AddText` /
  Reject+Esc discards), a toolbar button, and a keyboard chord
  (Ctrl+Shift+E). CHANGED `crates/pdfce-gui/src/canvas.rs`,
  `ui_text.rs`, `main.rs`.
- **Tooltip disambiguation (required companion, R78 bidirectional).**
  `text_menu_tooltip()` now states the FreeText/markup tool is "a
  removable annotation, not page content … use Add Text instead";
  `edit_text_tool_tooltip()` names Add Text; a new
  `add_text_tool_tooltip()` is the three-sentence disambiguator. This
  fixes the LIVE tooltip collision (the shipped FreeText tooltip
  previously read "Add a text box…", ambiguous against the new tool).

**Public API added to `pdfce-core`** (rule-10 API-guidelines trail; all
`#[non_exhaustive]`, non-breaking): `fn preview_wrap(...) ->
Result<AddTextWrapPreview, AddTextError>`; `struct AddTextWrapPreview {
lines, wrapped_lines, box_overflow_lines, page_overflow_pt, alignment,
disclosures }`; `struct WrapPreviewLine { text, origin_x, baseline_y
}`; `Std14::ALL: [Std14; 14]`. Doc-commented with spec §§ + R79/R76
cites, runnable doc examples.

**Headless-tested:**
`preview_wrap_lines_match_committed_boxed_add_for_identical_inputs`
(parses the `Tm` operands from `add_text`'s ACTUAL emitted stream,
asserts they equal the preview's per-line origins ±1e-4, across L/C/R/
Justified, two faces, explicit+derived leading, and an R76 overflow
case) + `preview_wrap_refuses_where_the_commit_would_refuse` (refusal
parity); the pure GUI helpers `resolve_drag_placement` (drag→
normalized box any direction; degenerate/NaN drag→point at drag start;
click→point) + `tool_builds_text_edit`/`tool_builds_add_text` (mutual-
exclusion invariant against the ACTUAL `SelectCanvasTool` dispatch, not
dead code). The egui wiring itself is compile-and-launch-verified, per
this project's established GUI-testing posture.

**Gates (re-verified in the main tree):** `cargo fmt --all --check`
clean; `cargo clippy --workspace --all-targets --all-features -D
warnings` clean (panic-free); `cargo tree -p pdfce-core` /
`-p pdfce-render` zero GUI deps (GUI-core separation intact through the
UI Pass); full workspace `cargo test` 0 failed (708 core lib + 18 core
`add_text` [up from 16 at 16.1] + 59 GUI unit, plus CLI/doctests all
unchanged — Pass 14.x/15.x/16.0/16.1/`vartext` tests all unchanged);
release GUI build succeeds and launches without panic; **ZERO new
dependency**.

**Engineer judgment calls (all defensible, none blocking — recorded
for the trail):**
1. Box mode was BUILT, not blocked — the `pdfce-ui-specialist` spec
   predated 16.1's ship; since 16.0 and 16.1 are both shipped, the full
   box-drag path plus the new P0 `preview_wrap` accessor were built for
   16.2 rather than deferred.
2. Enter semantics: point mode's plain Enter Accepts; box mode's plain
   Enter inserts a paragraph break (`\n`), Ctrl+Enter Accepts —
   resolves "Enter = Accept" against box mode's need to type newlines
   without a hidden mode toggle.
3. `preview_wrap` returns the existing `AddTextError` (no new error
   type invented); the GUI stores and surfaces its `Display` string
   verbatim rather than re-wording it.
4. The colour surface uses `color_edit_button_srgba` →
   `NewTextColor::Black|Rgb` (no Gray/CMYK widget), since core
   Add-Text's colour model can't express those spaces either — the GUI
   surface matches the core surface exactly, no phantom capability.
5. `#[allow(clippy::too_many_arguments, clippy::type_complexity)]`
   with an inline reason comment on the 9-field `layout_boxed` —
   matches the codebase's existing convention for justified, spec-
   driven parameter lists rather than inventing a config-struct wrapper
   for a `pub(crate)` function.

**MILESTONE — FF-D (add-new-text) is COMPLETE end-to-end.** pdfce now
adds new page text — point and boxed, bundled Std-14 no-embed by
default, byte-identical-original, immediately editable/formattable/
reflowable through the existing 14.x/15.x pipeline — reaching Acrobat's
Add-Text baseline and exceeding it on minimal-diff (append-a-stream vs.
Acrobat's rewrite), tagged-honesty (disclosed-untagged, never a silent
structure-tree corruption), a first-class scriptable `pdfce-cli text
add` (Acrobat has none), and a documented, deterministic default-font
policy (Acrobat's is undocumented) — decision 016 §9's exceed-Acrobat
list, now all delivered. See "In progress" (below) for the broader
text-parity-arc-complete framing and the ★ Pass 16.x entry (Next up)
for the closing amendment.

### Pass 16.1 — Boxed add-new-text: multi-line wrap via the 15.x reflow engine (core + CLI; decision 016, FF-D slice 2 of 3) — 2026-08-01

**The second slice of the FF-D add-new-page-text subsystem (decision
016) — SHIPPED.** Independently re-verified green in the main tree: 16
core `add_text` tests + 13 CLI `add_text` tests pass (up from 9+8 at
Pass 16.0); a live boxed justified add wrapped to 2 lines with the
derived-layout disclosure, and round-trip reports `identical=1,
raster_identical=1` (original page byte-untouched).

**Modules changed:** CHANGED `crates/pdfce-core/src/text_edit/addtext.rs`
(the boxed engine extends 16.0 — the boxed branch lives entirely inside
the SHARED `plan_add_text` planner, so 16.0's single→array `/Contents`
append, inheritance-safe `/Resources`/`/Font` merge, Std-14 no-embed
dict, F-refuse encode, and both call sites (free `add_text` +
`EditSession::add_text`) are inherited verbatim — boxed session
integration was FREE, no `edit.rs` change, same `CommandKind::AddText`,
same byte-identical-original guarantee). CHANGED `reflow.rs`
(`align_origin_x` + `line_natural_width` hoisted to `pub(crate)` for
reuse). CHANGED `crates/pdfce-cli/src/main.rs` (`add-text
--box/--align/--leading`). Tests: +7 core, +5 CLI. Reused
`fixtures/synthetic/addtext/plain.pdf` (Courier monospace makes breaks
hand-computable; no new fixture).

**Reuse (decision 016 §3.1/§3.5 continuity):** 15.x breaker/origin math
reused, not duplicated (`linebreak::greedy_pack` + the two hoisted
reflow helpers); the only difference from a reflow is the MEASURER — a
fresh box has no glyphs, so it measures the operator's UTF-8 by the
chosen face's §9.4.4 AFM `/Widths` (like `vartext`) instead of
provenance advances. 15.1's emission recipe reused (negative-`TJ`
justified slack `N = −(slack/G)·1000/size`, space kept inside the
preceding string — sign-mirror of `reflow_apply::emit_justified_line`).

**Public API added to `pdfce-core`** (rule-10 API-guidelines trail;
all `#[non_exhaustive]`, non-breaking): `AddTextRequest` fields
`wrap_box: Option<Rect>`, `alignment: BlockAlignment`, `leading:
Option<f64>` + builders `with_box(x, y, w, h)` / `with_alignment` /
`with_leading` (`#[must_use]`, doc examples); `AddTextReport` fields
`wrapped_lines: Option<usize>`, `box_overflow_lines: usize`,
`page_overflow_pt: f64`, `alignment: Option<BlockAlignment>`;
`AddTextError` variants `InvalidBox(f64, f64)`, `NoWordsToWrap`. Reused
existing public `BlockAlignment`/`Rect` (no new types beyond the two
error variants).

**Acceptance (all tested):** wrap-matches-hand-computed (Courier
6×"aa"@12pt/6pt-spaces/box-width-30→2-per-line→3 lines); left/center/
right place correctly (origin_x 72/116/160); justified right-flush +
last-line-unstretched (negative-`TJ` slack −600, last line plain
`Tj`); original-byte-identical (prefix assertions); overflow emitted-
not-clipped per R76 (`box_overflow_lines > 0`, `page_overflow_pt > 0`,
all `Tm` runs emitted, reloads clean); undo restores original (one
`AddText` command, empty dirty set, redo re-applies); routes into the
14.0 model (re-extraction recognizes the new block); R71 missing-glyph
refusal; `InvalidBox`/`NoWordsToWrap` clean refusals; CLI `--at`/
`--box` mutual exclusion enforced + R59 render clean.

**Gates (re-verified in the main tree):** `cargo fmt --all --check`
clean; `cargo clippy --workspace --all-targets --all-features -D
warnings` clean (panic-free); `cargo tree -p pdfce-core` /
`-p pdfce-render` zero GUI deps (GUI-core separation intact); full
workspace `cargo test` 0 failed (16 core + 13 CLI `add_text`, up from
9+8; Pass 14.x/15.x/16.0/`vartext` all unchanged); R59 render check
notdef=0; round-trip original byte-verbatim; **ZERO new dependency**.

**Engineer judgment calls (all defensible, none blocking — recorded
for the trail):**
1. Emission recipe C — absolute `Tm` per line (immune to relative-`Td`
   accumulation); the larger-diff cost is moot since the whole added
   stream is new content anyway.
2. `\n` honored as hard paragraph breaks — each paragraph wraps
   independently (so it gets its own un-justified last line); a blank
   paragraph advances a baseline; words split on ASCII whitespace.
3. Wrap width = the FULL box width, left origin = the box's `llx`, NO
   padding inset — breaks match "box width" literally (unlike
   `vartext`'s `TEXT_PAD` convention).
4. First baseline = `box_top − 0.75·size`, descent budget `0.25·size`
   — matches the existing 14.0/15.x line-box convention so the
   re-recognized block agrees with the rest of the model.
5. Box is `(x, y, w, h)` with `(x, y)` = lower-left, matching the PDF
   `Rect` convention (not top-left).
6. Alignment is an explicit input, default `Left` — deliberately NOT
   15.0's auto-detect (a fresh box has no glyphs to auto-detect
   alignment from).
7. Added `--leading`/`with_leading`, default `1.2·size`, disclosed as
   derived when not supplied, operator-overridable.
8. `NoWordsToWrap` (whitespace-only text) and `InvalidBox`
   (non-positive/non-finite `w`/`h`) are clean refusals — CLI exits
   `EDIT_REFUSED`.
9. Justified single-word/last/overflowing-word lines are left
   un-stretched; space width comes from the chosen face's space glyph,
   falling back to `0.25·size` (disclosed) when the face has none.

**16.2 (add-text canvas UI) is the final slice of decision 016/FF-D —
its UI design has shipped** (a dedicated `CanvasTool::AddText` variant,
a required FreeText-tooltip disambiguation, and a new pure read-only
wrap-preview accessor for box-mode live feedback), **build now
dispatched** — see the ★ Pass 16.x entry under Next up for status.

### Pass 16.0 — Add-new-text engine + point-text insert (core + CLI; decision 016, FF-D slice 1 of 3) — 2026-08-01

**The first slice of the FF-D add-new-page-text subsystem (decision
016) — SHIPPED. The "RECOMMENDED NEXT BUILD" status named at
continuation 44 is now discharged.** Full record:
`docs/decisions/016-ffd-add-new-page-text.md`. Independently
re-verified green in the main tree: 9 core `add_text` tests + 8 CLI
`add_text` tests pass; a live add ("Added by pdfce" at 100,700)
produced exactly two new objects (content_object=6, font_object=7),
bundled Helvetica disclosed, and round-trip reports `identical=1,
raster_identical=1, reloaded=1` (original page byte-untouched).

**Modules changed:** NEW `crates/pdfce-core/src/text_edit/addtext.rs`
(the engine); CHANGED `mod.rs` (re-exports), `fontdata/mod.rs` (added
`std14_base_font_name(Std14) -> &'static str`, the inverse of
`std14_by_base_font`, to write `/BaseFont`), `edit.rs`
(`CommandKind::AddText` + `EditSession::add_text`),
`crates/pdfce-cli/src/main.rs` (`add-text` subcommand + `cmd_add_text`
+ `parse_at_pair`/`parse_rgb_triple`). NEW tests
`crates/pdfce-core/tests/add_text.rs` (9),
`crates/pdfce-cli/tests/add_text.rs` (8). NEW fixtures
`fixtures/synthetic/addtext/{plain,inherited-resources,tagged}.pdf` +
generator + `PROVENANCE.md`.

**The three load-bearing recipes (decision 016 §3.3/§3.5):**
1. `/Contents` single→array append (`R_orig` → `[R_orig R_new]`;
   array → append; absent → `R_new`) — the incremental update
   re-emits ONLY the page dict + the 2 new objects; the original
   content stream is NEVER re-emitted (byte-identical, R32/R46).
2. Standard-14 Type1 font dict (`/Type/Font /Subtype/Type1
   /BaseFont/<one-of-14> [/Encoding/WinAnsiEncoding] /FirstChar 32
   /LastChar 255 /Widths[…]`, NO `/FontFile` — R79; `/Encoding`
   omitted for Symbol/ZapfDingbats; `/FontDescriptor` deliberately
   omitted to keep exactly 2 new objects).
3. Inheritance-safe `/Font` add: rebuilds the page's `/Resources`
   INLINE from the effective (own-or-inherited) resources with a
   merged `/Font` subdict + a collision-free `/pdfceF{n}` name — never
   mutates the shared ancestor `/Pages` dict (verified: the
   inherited-resources fixture keeps ancestor object-2's font count at
   1). The run emitted is `\n q BT /pdfceF1 <size> Tf <0 g|rg> 1 0 0 1
   x y Tm (codes) Tj ET Q` — self-q/Q-balanced (§8.4.2), explicit
   Tf/colour/Tm, leading `\n` separator.

**Public API surface added to `pdfce-core`** (rule-10 API-guidelines
trail): `text_edit::add_text(&Document, &AddTextRequest) ->
Result<AddTextOutcome, AddTextError>`; `AddTextRequest` (`new` +
`with_font`/`with_provenance`/`with_size`/`with_color` builders);
`AddTextReport`; `AddTextOutcome`; `AddTextError` (`thiserror`,
`#[non_exhaustive]`); `FontProvenance { Bundled, Supplied }`;
`NewTextColor { Black, Rgb }`. `edit::CommandKind::AddText`,
`EditSession::add_text`. `fontdata::std14_base_font_name`. All
`#[non_exhaustive]`, doc-commented with runnable examples + ISO cites
(§7.7.3.3 / §8.4.2 / §7.8.3 / §9.6.2.2 / §9.4.2 / §9.4.3 + R78 / R79 /
R71 / R73).

**Acceptance (all tested):** original byte-identical
(`plain_add_keeps_original_byte_identical_and_renders_new_run` + the
CLI's byte-prefix check); re-extracts as an editable block
(`added_run_is_editable_by_the_14_1_surgery`); inheritance-safe
(`inherited_resources_add_does_not_mutate_the_shared_ancestor`);
tagged-page disclosure per R73
(`tagged_page_add_emits_the_untagged_disclosure`); missing-glyph
refusal per R71 (core returns no output + CLI exits 9); undo restores
byte-identical (`session_add_text_is_one_undoable_command`); font
provenance both `Bundled` and `Supplied` disclosed; R59 render check
(notdef glyph count 0, both the original and the new run rasterize).

**Gates (re-verified in the main tree):** `cargo fmt --all --check`
clean; `cargo clippy --workspace --all-targets --all-features -D
warnings` clean (panic-free); `cargo tree -p pdfce-core` /
`-p pdfce-render` zero GUI deps (GUI-core separation intact); full
workspace `cargo test` 0 failed (708 core unit + integration tests
including the 9+8 new, plus 47 doctests); Pass 14.x/15.x/`vartext`
tests all unchanged; **ZERO new dependency** (no `Cargo.toml`
touched).

**Engineer judgment calls (all defensible, none blocking — recorded
for the trail):**
1. CLI subcommand named `add-text` (flat kebab, matching the
   already-shipped `edit-text`/`format-text`/`reflow`), NOT decision
   016's literal "text add" group name — internal CLI-surface
   consistency won; migrating the whole family to a `text` subcommand
   group is left as a separate, cosmetic future pass.
2. `/FontDescriptor` omitted (full `/Widths` array form kept instead)
   — §9.6.2.1 would force it indirect for zero metric benefit at
   Standard-14; keeps the add at exactly 2 new objects.
3. `/Resources` is always rebuilt inline (uniform handling across
   own/indirect/inherited resource dicts) — it references the shared
   sub-dicts rather than mutating or duplicating them.
4. A space character emits an R-INV-5 "ambiguous" disclosure (WinAnsi
   maps space at both code 32 and code 160) — this is the shared 14.1
   gate behaving exactly as designed; pdfce picks 32 and discloses
   honestly. Left as-is for cross-Pass consistency, but flagged as
   mildly noisy (a multi-word add can emit two near-identical space
   disclosures) — a candidate future polish to de-dup R-INV-5 space
   disclosures within one add.

**Flagged follow-up, NOT actioned this Pass — no
certification-signature guard on the `add_text`/`EditSession::add_text`
path.** Unlike `add_markup`, this path checks encryption and
suppressed-objects but not `check_certification` (a private
`EditSession` method the free function can't reach). Recorded in
Backlog below (see "FF-D follow-up — certification-signature guard
gap"); do not action without an explicit dispatch.

**16.1 (boxed add + wrap via the 15.x reflow engine) and 16.2 (add-text
canvas UI) remain the next slices of decision 016 / FF-D** — see the
★ Pass 16.x entry under Next up for the promoted status.

### Pass 14.4 — Text-edit GUI refinements: selection-replace-on-type, triple-click line-select, drag-select, arrow/Home/End caret navigation (completing the four Pass-14.3 deferred interactions) — 2026-08-01

**The four GUI interactions deferred at Pass 14.3's ship — SHIPPED,
closing out the on-canvas text-editing beta interaction set.** All
four ride the existing `CanvasTool::TextEdit` — no new tool variant,
no new dependency, everything a reviewable `PendingEdit`. Independently
re-verified green in the main tree: 4 new core caret-navigation tests +
56 GUI tests pass; release GUI build launches; relaunched live for the
operator (pid 40764).

- **Selection-replace-on-type:** typing over a single-run selection
  now replaces it in one step (previously insert-then-backspace);
  Backspace/Delete delete the selection outright; stays a reviewable
  `PendingEdit`; the font-on-edit refusal-and-disclosure gate at Accept
  is untouched.
- **Triple-click → line select:** inlined over the already-shipped
  `line_range_at` (Pass 14.3 §0.2 accessor).
- **Drag-select:** press sets anchor + caret; each dragged frame moves
  the focus caret via a per-move hit-test; selection resolves
  anchor..focus through `resolve_range`.
- **Arrow / Home / End caret navigation:** Left/Right cross run/line
  boundaries; Up/Down land at the nearest x-position on the adjacent
  line; Home/End via `line_range_at`; Shift extends the active
  selection.

**Modules changed:** `crates/pdfce-core/src/text_edit/model.rs` (new
caret-nav accessors + tests); `crates/pdfce-gui/src/canvas.rs`
(selection-replace pure helpers + tests); `crates/pdfce-gui/src/main.rs`
(gesture/nav/Delete-key/keyboard-gating/selection-replace wiring).

**Public API surface added to `pdfce-core`** (rule-10 API-guidelines
trail): `EditableTextModel::{caret_x(pos) -> Option<f32>,
caret_on_line_nearest_x(line_index, x) -> Option<TextPosition>,
caret_left(pos), caret_right(pos), caret_up(pos, desired_x),
caret_down(pos, desired_x)}`. (`pdfce-gui`'s `canvas.rs` also gained
`single_run_selection_range` + `selection_after_type` pure helpers —
GUI-crate, not core API surface.)

**Notable fix (recorded as a side-effect discovery, not a separately-
scoped bug hunt):** `collect_keyboard_actions` now yields Home/End/
Delete/Backspace to the canvas whenever a tool is active. This
un-swallowed the text-edit Backspace key that the global
`DeleteSelection` keybinding was silently eating in shipped Pass
14.3 — a latent bug, fixed as a side effect of this Pass's
keyboard-gating reconciliation.

**Engineer judgment calls (all defensible, none blocking — recorded
for the trail):**
1. Model-dependent caret navigation lives in `pdfce-core`, not
   `pdfce-gui` — `PageText`/`TextRun`/`ExtractedGlyph` are
   `#[non_exhaustive]`, so they're only constructible/headless-testable
   from inside core. Matches the Pass 14.3 §4.3 precedent ("core owns
   the derived structure"); only model-FREE string helpers stayed in
   `canvas.rs`.
2. Keyboard-gating reconciled per the Pass 14.3 §4.5 spec (which keys a
   tool is allowed to consume vs. the global bindings).
3. Up/Down use a per-press `desired_x` from `caret_x` — there is no
   sticky goal-column carried across repeated Up/Down presses. Named
   as a deferred nicety, not a defect.
4. Arrow-key navigation is gated to `pending.is_none()` — inside an
   open `PendingEdit`, the caret is the draft's own text-editing
   cursor, so model-space arrow-nav while actively composing an edit is
   a named first-cut limitation. Typing/Backspace/Delete still work
   normally inside a pending edit.
5. Multi-run selection refusal is NOT regressed by this Pass:
   `cross_run` still suppresses typing and disables Accept;
   `single_run_selection_range` returns `None` for a cross-run span, so
   the new selection-replace path only ever fires on a same-run
   selection.

**Gates (re-verified in the main tree):** `cargo fmt --all --check`
clean; `cargo clippy --workspace --all-targets --all-features -D
warnings` clean; `cargo test --workspace` all green (708 core lib tests
+ 56 GUI + integration suites; the 1 ignored test is pre-existing,
unrelated); `cargo tree -p pdfce-core` / `-p pdfce-render` zero GUI
deps (GUI-core separation intact); release GUI build succeeds and
launches without panic, relaunched live for the operator (pid 40764);
**ZERO new dependency**; Pass 14.x/15.x/`vartext` tests all unchanged.

**MILESTONE — the text-editing beta's interaction set named at Pass
14.3's ship is now COMPLETE.** All four interactions deferred at that
Pass (selection-replace-on-type, triple-click line-select, drag-select,
arrow/Home/End caret navigation) are shipped. What remains open in the
text-parity space: **FF-B** (cross-block/cross-page reflow — the
genuine exceed-Acrobat headline), **FF-D** (add new page text),
**FF-H** (`Tc`/`Tw`/`Tz`/`Ts` spacing + synthetic styles), and the
still-open list-authoring scope question (no operator answer yet).
None of these are yet scoped to a Pass.

### Pass 15.2 — On-canvas within-block reflow UI (decision 015, FF-A slice 3 of 3, FINAL SLICE — decision 015 / FF-A COMPLETE end-to-end) — 2026-08-01

**The third and FINAL slice of the FF-A within-block offline reflow
subsystem (decision 015) — SHIPPED. Decision 015 / FF-A is now COMPLETE
end-to-end (15.0 engine → 15.1 surgery → 15.2 UI).** Full record:
`docs/decisions/015-ffa-within-block-offline-reflow.md`. Independently
re-verified green in the main tree: 60 core `text_edit` tests; CLI
reflow intact after the P0 dedup (5 `reflow` + 11
`inspect_reflow_preview`); 53 GUI tests; release GUI builds + launches;
the GUI runs live against the reflow fixture with the reflow sub-mode
of the Edit Text tool functioning.

- **P0 consolidation (paid down before the UI landed):**
  `reflow_recognition_options()` — the relaxed block-recognition that
  keeps ragged-left justified/right/center paragraphs whole (R77) — is
  now ONE `pub fn` in `pdfce-core::text_edit::reflow` (re-exported at
  `pdfce_core::text_edit::reflow_recognition_options`). The
  `pub(crate)` duplicate previously in `reflow_apply.rs` and the
  private duplicate previously in `pdfce-cli/src/main.rs` are DELETED;
  every consumer (CLI `inspect`, the apply/session path, engine tests,
  and all three GUI call sites) now calls the single source. CLI
  reflow tests stay green across the dedup.
- **NEW `EditableTextModel::block_at(pos: TextPosition) -> Option<usize>`**
  (`#[must_use]`) — sugar over `line_at` + `Line::block`, no GUI type
  in its signature; lets the GUI resolve "which block is the caret in"
  without duplicating Pass 14.0's line/block-lookup logic.
- **GUI (`crates/pdfce-gui/src/main.rs` + `ui_text.rs`): reflow is a
  SUB-MODE of `CanvasTool::TextEdit`, NOT a new `CanvasTool` variant**
  (R60 — no proliferation of top-level tools for what is really one
  editing mode's alternate action). `TextEditState.reflow:
  Option<ReflowState>` is mutually exclusive with `pending` (the 14.3
  in-place-edit state) — a block is either being retyped or being
  reflowed, never both at once. A "Reflow paragraph…" button targets
  the caret's block via `reflow_recognition_options()`'s relaxed
  recognition (so a ragged-left justified paragraph reflows as one
  block, matching 15.0/15.1's recognition). The property bar offers:
  width (a `DragValue` AND a canvas drag-handle, so width can be set
  numerically or by dragging on the page), alignment (pre-filled with
  the 15.0-DETECTED value, switchable to L/C/R/Justify), and leading.
  Ghost preview (the proposed re-wrap) and a solid targeted-block
  highlight render non-destructively, reusing Pass 14.3's
  preview/mask rendering language so the visual grammar is consistent
  across both edit modes. **Accept** commits exactly the one
  undo-able `EditSession::reflow_block` (`CommandKind::ReflowBlock`,
  Pass 15.1's command) the operator is previewing; **Reject** discards
  with nothing written. Overflow (R76), the tagged/trust-level
  disclosures (R72/R73), and the already-edited-this-session refusal
  (Pass 15.1 judgment call 6) surface VERBATIM via Pass 14.3's
  disclosure rendering — no re-wording, no silent swallowing. Two-stage
  Esc rejects (matching 14.3's existing Esc convention for the
  in-place-edit pending state).
- **Pure, headless-tested helper functions**: `reflow_button_enabled`,
  `reflow_alignment_is_override`, `reflow_refusal_hint` — the
  decision-logic slivers of the reflow UI are factored out as plain
  functions with their own unit tests, independent of egui, so the
  underlying logic is verifiable without a running GUI. The egui
  wiring itself (widget layout, drag handling, paint calls) is
  compile-and-launch-verified rather than unit-tested, consistent with
  this project's established GUI-testing posture (immediate-mode code
  is cheap to visually/launch-verify, expensive to unit-test
  meaningfully).

**Public API surface added to `pdfce-core` (rule-10 API-guidelines
trail):** `pub fn reflow_recognition_options() -> BlockRecognitionOptions`
(`#[must_use]`; doc comment states the WHY/trade-off — relaxed
recognition keeps ragged-edge justified/right/center paragraphs whole
at the cost of being less strict than the default recognizer — with
R77/§0.3 cites); `pub fn EditableTextModel::block_at(&self, pos:
TextPosition) -> Option<usize>` (`#[must_use]`).

**Gates (re-verified in the main tree):** `cargo fmt --all --check`
clean; `cargo clippy --workspace --all-targets --all-features -D
warnings` clean; `cargo tree -p pdfce-core` / `-p pdfce-render` zero
GUI deps (no egui/eframe/winit/wgpu/glow/accesskit — GUI-core
separation invariant holds through the reflow-UI Pass); `cargo test
--workspace` — 1198 passed, 0 failed (60 core `text_edit` + 5 CLI
`reflow` + 11 CLI `inspect_reflow_preview` + 53 GUI, among others);
release GUI build succeeds and launches without panic against the
reflow fixture; **ZERO new dependency**; Pass 14.x, Pass 15.0/15.1, and
`vartext` tests all unchanged.

**Engineer judgment calls (all defensible, none blocking — recorded
for the trail):**
1. Wired the panel's disclosure/refusal surfacing to 15.1's REAL
   shipped types (`ReflowApplyReport`/`ReflowApplyError`,
   `report.disclosures`), not the original decision-015 spec's
   hypothesized `ReflowReport`/`ReflowSessionError` names — the spec
   predates 15.1's actual implementation; the UI must speak to what
   actually shipped, not the pre-build sketch.
2. **Fixed a bug found in the spec's own §4.3 override-detection
   snippet** while implementing it: the spec's `else if val ==
   detected` pattern reset the override flag every frame (any frame
   where the current value happened to equal the auto-detected value
   would silently clear a deliberate override). The shipped logic
   instead decides "is this an override" on the CLICK event of the
   clicked alignment value, not by re-comparing on every frame.
3. Accept/Reject are buttons + Esc rejects — matched the ALREADY
   -SHIPPED 14.3 `PendingEdit` convention (button-only Accept, no
   dedicated keyboard Accept action) rather than inventing a new
   keybinding for reflow specifically. Consistency with the sibling
   edit mode won over adding a shortcut.
4. **`ReflowApplyError::Unsupported` covers BOTH** the
   already-edited-this-session refusal (15.1 judgment call 6) AND
   rotated/shared/non-contiguous blocks (15.1 judgment call 3) — one
   variant, two triggers. The GUI shows one hint per refusal; core's
   `Display` impl names which specific condition triggered it, so the
   operator-facing message stays precise without the GUI needing to
   duplicate that classification logic.
5. Live preview is recomputed every frame from the current
   width/alignment/leading inputs (pure + cheap relative to 15.0's
   engine cost on a single block) rather than cached-and-invalidated —
   simpler code, no staleness-tracking bugs, and the block size involved
   makes the recompute cost immaterial.
6. Added `block_at` (a P1 nice-to-have from 15.0/15.1's design notes)
   as a clean, testable, `#[must_use]` accessor rather than inlining
   the line/block lookup at each GUI call site.
7. The width drag-handle is painted with a faint fill (not just a bare
   hit-region) purely for discoverability — an invisible drag target
   is a known egui usability trap; a faint visual affordance costs
   nothing and prevents "how do I resize this" confusion.

**Milestone — FF-A (within-block offline reflow) is now COMPLETE
end-to-end (15.0 + 15.1 + 15.2 all shipped).** pdfce does reviewable,
undo-able within-block reflow: greedy re-wrap, alignment auto-detect/
preserve across all four modes (left/center/right/justified), and
working justified alignment via `TJ` slack distribution — entirely
offline. This reaches, and on justify-reliability/alignment-detection/
overflow-honesty exceeds, Acrobat's own offline (non-cloud) reflow
capability (decision 015 §9's exceed-Acrobat list, now all delivered).
Combined with the already-shipped Pass 14.x in-place-editing family,
pdfce's Acrobat text-handling parity is now broad and deep at the P0
level. See "In progress" (below) for what remains open in the
text-parity space (FF-B onward) and the ★ Pass 15.x entry (Next up)
for the closing amendment.

### Pass 15.1 — Reflow surgery + one undo-able `CommandKind::ReflowBlock` + CLI `reflow` (decision 015, FF-A slice 2 of 3) — 2026-08-01

**The second slice of the FF-A within-block offline reflow subsystem
(decision 015) — SHIPPED.** Reflow now APPLIES, not just previews. Full
record: `docs/decisions/015-ffa-within-block-offline-reflow.md`.
Independently re-verified green in the main tree: 6 core `reflow_apply`
+ 5 CLI `reflow` + 2 render tests pass; a live justified reflow (page
4, width 180) re-wrapped 4→5 lines with 4 justified lines + an
un-stretched last line, only the block's content object changed, and
round-trip reports `identical=1, raster_identical=1, reloaded=1`.

- **NEW `crates/pdfce-core/src/text_edit/reflow_apply.rs`** (~660 LoC +
  tests) — the reflow-apply surgery: takes a Pass 15.0 `ReflowPreview`
  for one block and re-emits that block's show operators at the new
  line origins/breaks via the Pass 14.1 advance-preserving machinery
  (`emit_tm`/`splice`/`write_incremental`/`make_raw_stream`/§9.4.4
  advance). **CHANGED** `reflow.rs` (`WordTok` now carries the source
  `codes`; `tokenise_block` made `pub(crate)`, returning the
  representative space code — 15.0's preview behaviour is byte-
  unchanged by this), `mod.rs` (re-exports `apply_reflow`/
  `ReflowApplyError`/`ReflowApplyReport`/`ReflowOutcome`), `edit.rs`
  (`CommandKind::ReflowBlock { lines_before, lines_after }` +
  `EditSession::reflow_block`, mirroring Pass 14.3's edit_text/
  format_text plan/effect split), `crates/pdfce-cli/src/main.rs`
  (`reflow` subcommand + `cmd_reflow`; flags are `--page` singular +
  `--block/--width/--align/--leading`), `crates/pdfce-render/src/lib.rs`
  (2 R59 tests on reflowed output), `crates/pdfce-gui/src/ui_text.rs`
  (`ReflowBlock` undo label). **NEW**
  `crates/pdfce-cli/tests/reflow.rs` (5 tests). Reused
  `fixtures/synthetic/reflow/reflow.pdf` (`PROVENANCE.md` updated, no
  new fixture needed).
- **Justify (`TJ` general path):** per full non-last line, `N_gap =
  −(S/G)·1000/emit_scale`, where `emit_scale = Tfs·Th·a·ca`, emitted as
  one `[ (w0 SP) N (w1 SP) … (wlast) ] TJ` with the original code-32
  spaces kept + `0 Tw` set once — a sign-mirror of Pass 14.1's
  compensating-`TJ` pin. Slack `S`/gap-count `G` come from the 15.0
  preview. The last line and any single-word line are never stretched
  (`justified_slack == None` → plain `Tj`). Justify with non-zero
  `Tc`/`Tw` is refused-and-disclosed (the slack arithmetic assumes
  spaces carry only `w0`). The `Tw`-word-spacing alternative is
  documented as the non-goal path (can't serve composite fonts; leaks
  into the last line).
- **Line origin = recipe C (absolute `Tm` per line):** the whole block
  is re-emitted as one fresh `BT…ET` (per-line `Tm` costs nothing extra
  in minimal-diff terms this way), immune to the §3.1 relative-`Td`
  re-basing bug identified during design, and drives left/center/right/
  justified uniformly. `(a,b,c,d)` come from the block's provenance
  text-matrix (preserving size/orientation); `(e,f)` come from the
  preview's line origin through the axis-aligned CTM.
- **Codes carried, never re-encoded** — reflow re-wraps, it does not
  change text, so no InverseEncoding round-trip is needed; only
  R-INV-4 (composite) applies. **Page-overflow (R76):** all lines are
  emitted at their true (possibly negative-baseline) positions, never
  clipped or dropped, and disclosed. **Tagged (R72):** the block's own
  `BT…ET` is re-emitted preserving the enclosing `BDC`/`EMC` + `MCID`
  by construction. **Incremental save (R34):** only the block's own
  content-stream object is re-emitted.

**Public API surface added to `pdfce-core` (rule-10 API-guidelines
trail):** `apply_reflow(&Document, page_index, block_index,
&ReflowRequest) -> Result<ReflowOutcome, ReflowApplyError>`;
`ReflowOutcome { bytes, report }`; `ReflowApplyReport { block_index,
lines_before, lines_after, alignment, justified_lines, base_font,
glyph_source, tagged_mcid, height_delta, overflow, content_object,
extra_objects_emptied, disclosures }`; `ReflowApplyError` (`thiserror`,
`#[non_exhaustive]`: `Preview`/`Refused`/`NoProvenance`/`PageIndex`/
`Unsupported`/`Encrypted`/`Extract`/`Content`/`PageTree`/`Write`);
`EditSession::reflow_block(page_index, block_index, &ReflowRequest)`;
`CommandKind::ReflowBlock { lines_before, lines_after }`. ISO-cited doc
comments throughout (§9.4.3, §9.3.3, §9.4.2, §9.3.5, plus the
`reflow_emission` module note and R-INV/R76/R72 cross-references).

**Gates (re-verified in the main tree):** `cargo fmt --all --check`
clean; `cargo clippy --workspace --all-targets --all-features -D
warnings` clean; `cargo tree -p pdfce-core` / `-p pdfce-render` zero
GUI deps; full workspace green across 25 ok-blocks, 0 failures (core
lib 702 including the 6 new `reflow_apply` tests; CLI `reflow` 5;
render 2 new R59 reflow tests); R59 on reflowed output confirms real
glyphs, `unknown_ops = 0`, and justified ink reaching the box's right
margin; round-trip reports `identical=1, raster_identical=1`; **ZERO
new dependency**; Pass 14.x, Pass 15.0, and `vartext` tests all
unchanged.

**Engineer judgment calls (all defensible, none blocking — recorded
for the trail):**
1. Codes are carried through, never re-encoded (reflow doesn't change
   text, only its layout).
2. Recipe C (absolute per-line `Tm`) chosen over a more compact `Td`/
   `T*` scheme — see the §3.1 relative-repositioning bug it avoids.
3. **Region = the block's own `BT…ET`, re-emitted as one fresh
   `BT…ET`; refused if the block shares a text object with other
   content, is non-contiguous, or has a show operator outside `BT`/
   `ET`.** This keeps the surgery provably safe and preserves the
   MCID wrapper by construction, at the cost of narrower applicability
   — a deliberate trade recorded here, not an oversight.
4. **Axis-aligned scope only** — rotated/skewed/multi-transform text
   and form-XObject text are refused by name. Recipe C's rotation
   support is left as a documented future extension, not a claimed-but-
   untested path.
5. Justify requires `Tc = Tw = 0`, else refused-and-disclosed (see the
   slack-arithmetic assumption above).
6. **`reflow_block` plans against BASE content and refuses if the page
   was already edited earlier in the same session** (its offsets are
   base-relative; a clean named refusal beats a silent mis-splice).
   Recorded explicitly as a **known first-cut limitation** to lift in a
   later Pass, not a permanent design constraint.
7. Filtered out the 15.0 preview's carried-through disclosures whose
   wording asserted "nothing written / READ-ONLY" and re-emitted
   apply-stage-equivalent disclosures instead, so nothing in the
   disclosure list contradicts the fact that a write just happened.
8. Text state (`Tf`/`Tz`/`Tc`/`Tw`) is read from the content walk;
   geometry comes from provenance — the two sources are deliberately
   kept separate rather than merged into one struct.

### Pass 15.0 — Within-block greedy reflow engine + alignment auto-detect (read-only; decision 015, FF-A slice 1 of 3) — 2026-08-01

**The first slice of the FF-A within-block offline reflow subsystem
(decision 015) — SHIPPED, READ-ONLY.** Full record:
`docs/decisions/015-ffa-within-block-offline-reflow.md`. Independently
re-verified green in the main tree: 11 core `reflow::tests` + 11 CLI
`inspect_reflow_preview` tests pass; **vartext's 17 tests pass
unchanged** (the greedy-core factoring preserved behavior); the demo
detected left/right/center/justified correctly across the 4 fixture
pages with new bboxes/height-deltas computed.

- **NEW `crates/pdfce-core/src/linebreak.rs`** — the factored shared
  greedy first-fit breaker `greedy_pack(word_count, max_width,
  line_width_closure) -> Vec<Range<usize>>` (pure index arithmetic + a
  width-measuring closure, no font/text-state coupling of its own).
  **NEW `crates/pdfce-core/src/text_edit/reflow.rs`** (~1030 lines, 11
  tests) — the `ReflowEngine` + alignment auto-detect + `ReflowPreview`,
  built on Pass 14.0's `Block`/`Line`/x-band geometry. `vartext.rs`'s
  `wrap_lines` now calls `greedy_pack` with an Std14-AFM measurer —
  **byte-for-byte identical output, all 17 vartext tests pass
  verbatim** — proving the factoring was a pure refactor, not a
  behavior change. `lib.rs` gains `pub mod linebreak`;
  `text_edit/mod.rs` re-exports the new reflow types.
- **CLI:** `inspect --reflow-preview --block N --width W [--align
  L|R|C|J] [--leading pt] [--json]` (`crates/pdfce-cli/src/main.rs`).
  **Fixture:** `fixtures/synthetic/reflow/reflow.pdf` (5-page Courier
  synthetic: left/right/center/justified + a small page proving
  computed page-overflow) + `tools/gen-reflow-fixtures.py` +
  PROVENANCE.md. **Tests:**
  `crates/pdfce-cli/tests/inspect_reflow_preview.rs` (11 tests).
- **ONE greedy breaker, two callers** (`vartext` = AFM measurer;
  `reflow` = provenance §9.4.4-advance measurer via
  `ExtractedGlyph::advance` — no font re-measurement). Whitespace-only
  breaks, no hyphenation (Knuth-Plass/hyphenation stay deferred to
  FF-B/later). **READ-ONLY**: no surgery/session/save path exists in
  15.0 — that's 15.1.
- **Alignment auto-detect** from the 14.0 line boxes: per-line left/
  right/mid edges, `tol = max(2.0, 0.5·size)` pt; priority
  Justified(n≥3, left-flush + all-but-last right-flush + short last) →
  Left → Right → Center → Left/Ambiguous; single-line → Left/
  SingleLineDefault. Counted + disclosed + overridable (R72).
  Justified preview computes per-line slack; the last line is never
  justified (matches the eventual 15.1 surgery's own rule, and
  Acrobat's own base-panel behavior).
- **Page-overflow COMPUTED + disclosed** here (all lines still
  computed with negative baselines past the box), applied/enforced in
  15.1 (disclose-and-allow, R76 — never silently clipped).

**Public API surface added to `pdfce-core` (rule-10 API-guidelines
trail):** `ReflowEngine<'m, 'a>` (new/detect_alignment/preview);
`enum BlockAlignment { Left, Right, Center, Justified }`
(as_str/parse/is_justified); `enum AlignmentSource { Detected,
SingleLineDefault, AmbiguousDefault, Overridden }`;
`DetectedAlignment`; `ReflowLine`; `PageOverflow`;
`ReflowDiagnostics`; `ReflowPreview` (+ `height_delta()`);
`ReflowRequest` (builders `new`/`with_wrap_width[_opt]`/
`with_alignment[_opt]`/`with_leading[_opt]`/`with_page_cropbox`);
`enum ReflowError { BlockIndexOutOfRange, EmptyBlock, BadWidth }`
(`thiserror`). `#[non_exhaustive]` on options/outputs; builders exist
specifically because `#[non_exhaustive]` blocks cross-crate struct
literals.

**Engineer judgment call recorded for the trail:** a right/center/
justified paragraph has ragged LEFT edges, which Pass 14.0's
first-line-indent recognizer rule would fragment into single-line
blocks (each ragged-left line misread as its own indented paragraph
start); reflow therefore recognizes with **indent-splitting
relaxed** — `indent_ratio` pushed out of practical reach while
leading-gap splitting is kept unchanged. Exposed as
`reflow_recognition_options()` in the CLI and `recognise_relaxed` in
tests; documented in both call sites. Left/justified (flush-left)
paragraphs are unaffected by the relaxation. The threshold constants
are named + documented as corpus-tunable (decision 015 §10 revisit
trigger 2 — re-derive from a larger corpus if false-splits/false-joins
show up).

**Gates (re-verified in the main tree):** `cargo fmt --all --check`
clean; `cargo clippy --workspace --all-targets --all-features -D
warnings` clean; `cargo test --workspace` all green (core lib 694; CLI
reflow-preview 11; text-blocks 5 UNCHANGED; edit/format/undo/render all
pass; doctests incl. `greedy_pack` + `ReflowRequest::new`); `cargo tree
-p pdfce-core` / `-p pdfce-render` zero GUI deps; **ZERO new
dependency** (no `Cargo.toml`/`Cargo.lock` touched); Pass 14.x + all
vartext tests unchanged.

### Pass 14.0 — Editable text model + block recognition (read-only; decision 014 Pass 1 of 4) — 2026-08-01

**The first slice of the Acrobat in-place-text-editing subsystem (decision
014) — SHIPPED, READ-ONLY.** Full record:
`docs/decisions/014-acrobat-text-editing.md`. Independently re-verified
green in the main tree: all 10 new tests pass (5 core `text_edit.rs` + 5
CLI `inspect_text_blocks.rs`), including
`sourced_view_is_unchanged_by_provenance_capture`, which pins Pass 4's
output as byte-identical when provenance capture is off.

- **New `pdfce-core` module `text_edit`** (`mod.rs` + `model.rs`): a
  Run→Line→Block recognition pipeline built as a SECOND clustering pass
  over Pass 4's `PageText.runs` — no re-extraction. Lines split at Pass
  4's `DerivedLineBreak` plus a defensive baseline-jump check; columns
  cluster by horizontal overlap then order left-to-right (derived
  §14.8.2.3.1 reading order); blocks/paragraphs break on leading-gap or
  first-line indent. All four thresholds exposed in
  `BlockRecognitionOptions`; every inference counted in
  `BlockDiagnostics`; the sourced-only view is always available via
  `EditableTextModel::sourced_view()`. Everything DERIVED/COUNTED/
  REVIEWABLE (§14.8 S1–S9, R72).
- **Provenance linkage added to the read path** — the substrate Pass 14.1
  surgery needs — gated behind `ExtractOptions::capture_provenance`
  (default OFF, so Pass 4 output is byte-for-byte unchanged). New
  per-glyph fields: show-operator byte span, content-stream ref (page vs.
  form object), font resource name, `Tf` size, fill colour (g/rg/k
  decoded; sc/scn → `Other`, never guessed), text matrix, CTM.
- **CLI:** `inspect --text-blocks [--pages …] [--json]` (plain `inspect`
  unchanged, pinned by a regression test). Derived-structure disclosures
  go to stderr; `--json` carries full structure + per-line provenance.
- **Fixture:** `fixtures/synthetic/textblocks/multi-column.pdf` (CC0
  synthetic, 1,154 bytes; 2 columns × 2 paragraphs × 10 lines; content
  emitted left-then-right to prove geometric ordering, one paragraph in
  blue to exercise colour provenance) + `tools/gen-textblocks-fixtures.py`
  + PROVENANCE.md.

**Gates (re-verified in main tree):** `cargo fmt --check` clean; `clippy
--workspace --all-targets -D warnings` clean (new code uses checked
`.get()` per the crate's panic-free `#![deny(clippy::indexing_slicing)]`
policy); `cargo tree -p pdfce-core` / `-p pdfce-render` GUI-dep-free;
**ZERO new dependency**; full workspace tests green (core lib 645,
`text_extract` integration 26 UNCHANGED, `text_edit` 5,
`inspect_text_blocks` 5, render/gui green, 6 doctests).

**Public API surface added to `pdfce-core` (rule-10 API-guidelines
trail):**
- `text_extract`: `enum ContentStreamRef { Page, Form { object: u32 } }`
  (#[non_exhaustive]); `enum TextColor { Gray(f32), Rgb(..), Cmyk(..),
  Other }` (#[non_exhaustive]); `struct GlyphProvenance { content_stream,
  operator_span: ByteSpan, font_resource: Option<Vec<u8>>, tf_size: f32,
  fill_color: Option<TextColor>, text_matrix: [f32; 6], ctm: [f32; 6] }`
  (#[non_exhaustive]); `ExtractedGlyph.provenance: Option<GlyphProvenance>`
  (new field); `ExtractOptions.capture_provenance: bool` + `const fn
  with_provenance`.
- `text_edit` (new): `EditableTextModel<'a>` (recognize / blocks / lines /
  columns / diagnostics / sourced_view / glyph / provenance / line_text /
  block_text / hit_test / resolve_range); `GlyphRef { run, glyph }`;
  `TextPosition { run, byte_offset }`; `Line`; `Block`; `enum BlockKind {
  Paragraph }` (#[non_exhaustive]); `BlockDiagnostics { .. }`;
  `BlockRecognitionOptions` (4 ratios).

**Engineer judgment calls (all defensible, none blocking — recorded for
the API-guidelines trail):**
1. `ExtractedGlyph` dropped `Copy` (now owns a `Vec` via the provenance
   `Option`), kept `Clone` — a technically breaking change to that type,
   but zero external consumers exist and every workspace consumer
   accesses glyphs by reference.
2. `TextPosition` uses a **byte** offset (glyph-boundary) into the run's
   UTF-8 text, not the decision record's literal "char-offset" wording —
   because Pass 4 already keys glyphs by byte offsets (`text_start`/
   `len`). Documented; a UI layer converts to char index if it needs one.
3. Fill-colour is deliberately partial: device g/rg/k decoded; named-space
   sc/scn → `TextColor::Other`, never guessed to black (rule 4,
   fuzzy-never-sneaky).
4. `ActualText` runs left atomic (counted, not glyph-split — §14.9.4 N4);
   artifact runs excluded + counted from the hierarchy. Documented as a
   14.0 limit.

**Sequencing (superseded by the Shipped entry immediately below — Pass
14.1 has now shipped in turn):** originally promoted to In progress on
this Pass's ship; see `### Pass 14.1` below for the completed record.
14.2/14.3 remain in Next up, scope unchanged (see the ★ NEXT MAJOR FOCUS
entry there for the amendment note).

### Pass 14.1 — In-place text editing (content-stream surgery + font-on-edit refusal gate + CLI `edit-text`; decision 014 Pass 2 of 4) — 2026-08-01

**The second slice of the Acrobat in-place-text-editing subsystem
(decision 014) — SHIPPED.** Full record:
`docs/decisions/014-acrobat-text-editing.md`. Independently re-verified
green in the main tree: 19 core `text_edit` unit tests + 6 CLI
`edit_text` integration tests all pass; a live edit ("Hello"→"Hi")
produced `advance_delta = -16.008` with the Tm-follower repositioned and
all three disclosures surfaced; a subset-missing glyph was refused BY
NAME (R-INV-1, exit 9, verbatim Acrobat "embedded-but-not-local"
framing).

- **REMOVE→REPLACE content-stream surgery**, extending Pass 8.0's
  machinery (REMOVE is the `A_new = 0` case). New modules
  `crates/pdfce-core/src/text_edit/encoding.rs` (inverse-encoding
  builder + the R-INV refusal gate) and `text_edit/edit.rs` (REPLACE
  surgery, single-line relayout, font-on-edit gate, incremental save).
  Accessors added to `text_extract/font.rs` (`glyph_names()`,
  `is_simple()`). CLI `edit-text` subcommand
  (`crates/pdfce-cli/src/main.rs`). Fixtures
  `fixtures/synthetic/textedit/{nonembedded,embedded_full,
  subset_missing,tagged,tm_follower}.pdf` + `tools/gen-textedit-
  fixtures.py` + PROVENANCE.md (two embed the in-repo rights-cleared
  FoxitSans.cff; three wholly synthetic). Tests
  `crates/pdfce-cli/tests/edit_text.rs`.
- **Inverse encoding** inverts the font's OWN resolved `/Encoding` (via
  AGL), NEVER `/ToUnicode` (documented non-injective/lossy — a decode
  map is not safely invertible). **Advance-delta relayout** is
  REFLOW-default: `A = ((Σw0/1000·Tfs) − (ΣTj/1000·Tfs) + n·Tc +
  n_sp·Tw)·Th`; `ΔA` is added to every absolute `Tm` up to the next
  `Td`/`TD`/`T*`/`'`/`"` boundary; `--pin` keeps the Pass-8.0
  compensating-`TJ` path instead. Widths come from the same
  `ExtractFont::width` the render path already uses. Line-overflow is
  disclosed, not reflowed (FF-A deferred).
- **Font-on-edit gate = all R-INV-1..8:** font-classification triggers
  (R-INV-2 symbolic-no-encoding, R-INV-3 ToUnicode-only, R-INV-4
  composite) live in `classify_font`; per-character triggers (R-INV-1
  absent, R-INV-5 ambiguous→disclose+choose-reused-else-lowest, R-INV-6
  ligature-only, R-INV-7 code-occupied, R-INV-8 beyond-BMP) live in the
  inverse map. The embedded-subset floor (the one refusal case
  independently re-verified this session) = a code not in the page's
  already-used set → a named R-INV-1 refusal. A refusal never reaches
  the writer (rule 4 / R71).
- **Save = incremental (R34/R70)**; prior-text-survives is disclosed;
  edit ≠ redaction. **Tagged runs (R72):** BDC/MCID/EMC wrapper
  preserved, `/ActualText` staleness disclosed, not regenerated.

**Gates (re-verified main tree):** `cargo fmt --check` clean; `clippy
--workspace --all-targets -D warnings` clean (panic-free); `cargo tree
-p pdfce-core` / `-p pdfce-render` zero GUI deps; full workspace green
(core lib 657 incl. the 19 new; 6 new CLI; Pass 4 + Pass 14.0 tests
UNCHANGED); R59 on the edited `embedded_full.pdf` = substituted=0
notdef=0 unsupported=0; round-trip = the edited output is a
byte-identical prefix (untouched objects verbatim) across all five
test flows.

**Public API surface added to `pdfce-core` (rule-10 API-guidelines
trail):**
- `text_edit::encoding`: `RInvTrigger` (8 variants, `id()`/`is_hard()`);
  `Refusal` (Display+Error); `CharEncoding`; `EncodeResult`;
  `InverseEncoding` (`build`/`base_font`/`has_char`/`encode_char`/
  `encode_str`).
- `text_edit::edit`: `FollowerDisposition { Reflow, Pin }`;
  `EditRequest` + `find_replace`; `EditOptions` +
  `with_disposition`; `EditGlyphSource { Embedded, NonEmbedded }`;
  `EditOutcome`; `EditReport`; `EditError` (`thiserror`, 8 variants);
  `fn edit_text(doc, &EditRequest, &EditOptions) -> Result<EditOutcome,
  EditError>`.
- All new enums/structs are `#[non_exhaustive]`; errors are
  `thiserror`-derived, never stringly-typed (rule 10).

**Engineer judgment calls (all defensible, none blocking — recorded
for the API-guidelines trail):**
1. **Trust levels split across the crate seam** — core reports
   `Embedded`/`NonEmbedded` only; the CLI refines `NonEmbedded` into
   `Bundled`/`Supplied` via its own `FontEnvironment` (keeps core
   rasterizer-free, R21).
2. `subset = "ABCDEF+"` tag; "carried" = codes already used on the
   page — a safe under-approximation, disclosed as such, never an
   overclaim of what the embedded subset actually contains.
3. Anchor = find-in-operator with an optional `pinned_span` from Pass
   14.0's `GlyphProvenance`; Form-XObject content, `'`/`"` anchors, and
   cross-`TJ`-element matches are refused BY NAME (first-cut
   non-goals, not silent gaps).
4. Multi-stream pages collapse into the first content object + empty
   extras (disclosed) — a documented Pass-14.1 simplification.
5. Reflow applies `ΔA` to ALL absolute `Tm` operators on the line, not
   just the edited run's own.

**Follow-up flagged (Backlog candidate, not yet filed as a Pass):**
R-INV-2/3/4 are logic-covered but not fixture-tested — no
symbolic/ToUnicode-only/composite-font fixture exists yet to exercise
them end-to-end. Clean, scoped follow-up: build the three missing
fixtures + integration tests before FF-E/FF-F (composite/CJK/RTL
editing) is attempted, since those slices depend on the same gate
paths.

**Sequencing (superseded by the Shipped entry immediately below — Pass
14.2 has now shipped in turn):** originally left 14.2/14.3 in Next up
per decision 014's scope; see `### Pass 14.2` below for the completed
record. 14.3 (edit UI on the Pass 12.0 canvas) remains in Next up,
scope unchanged (see the ★ NEXT MAJOR FOCUS entry there for the
amendment note).

### Pass 14.2 — Formatting on a selection (size / fill-colour / font-family-style; decision 014 Pass 3 of 4) — 2026-08-01

**The third slice of the Acrobat in-place-text-editing subsystem
(decision 014) — SHIPPED.** Full record:
`docs/decisions/014-acrobat-text-editing.md`. Independently
re-verified green in the main tree: 10 core `text_edit::format` unit
tests + 11 CLI `format_text` integration tests all pass; a live CMYK
colour change stored the `k` operator (not DeviceRGB) with the
parity-plus disclosure surfaced; full workspace **1134 passed / 0
failed**.

- **New `pdfce-core` module `crates/pdfce-core/src/text_edit/format.rs`**
  (the three ops + `set_format`). **Changed** `edit.rs` (added
  fill-colour graphics-state tracking `FillState`/`DeviceSpace` to the
  shared walk with `g`/`rg`/`k`/`cs`/`sc`/`scn` arms; exposed walk/
  record/match/classify/emit/save helpers `pub(crate)`;
  `glyph_advance_with`; `emit_show` — **14.1's `edit_text` output bytes
  unchanged**, its tests pass verbatim); **changed** `mod.rs`
  (re-exports); **changed** `crates/pdfce-cli/src/main.rs`
  (`format-text` subcommand + `cmd_format_text` + `parse_set_color`);
  **new** `crates/pdfce-cli/tests/format_text.rs` (11 tests); **new**
  fixtures `fixtures/synthetic/textedit/{format_color,format_other,
  format_family}.pdf` + generator update + PROVENANCE.md (the 5
  existing 14.1 fixtures regenerated byte-identical).
- **State-wrap-and-restore emission** (the mechanism all three ops
  reuse): the anchor operator is split at the matched code-range into
  `pre | mid | post` and re-emitted as `[pre] <state-set> [mid]
  <state-restore> [post]`, so only the anchor operator's bytes change
  and every following operator stays byte-verbatim. Size → `/F newsize
  Tf … /F origsize Tf` wrap (fill operator never touched). Colour →
  chosen device operator (`rg`/`g`/`k`) + byte-verbatim restore of the
  recorded prior `FillState` (advance unaffected). Family → `/Ftarget
  Tf` swap + re-encode via 14.1's `InverseEncoding` against the
  target's `/Encoding`, gated by 14.1's `classify_font` + the
  embedded-subset carried-codes floor. All three reuse 14.1's
  locate→recompute-advance→relayout→incremental-save pipeline.
- **Fill-colour parity-PLUS (the differentiator):** the operator picks
  RGB/CMYK/gray and pdfce STORES the actual space (`rg`/`k`/`g`), NOT
  force-converted to DeviceRGB like Acrobat — disclosed. A run whose
  original space is non-device (`Other`: ICCBased/Separation/DeviceN/
  Indexed) has its tail restored byte-verbatim and the edited `mid`'s
  narrowing to device is DISCLOSED, never silent. Size-only edits
  never touch the colour operator (minimal-diff).
- **Family target restricted to an existing page font resource**
  (content-stream-only change, no resource-dict edit, no embedding — a
  missing/new target is a clean named refusal pointing at FF-C).
  Coverage gate is encoding-level (rasterizer-free, R21); Embedded/
  Bundled/Supplied is the shell's disclosure layer (decision 012);
  `--font-dir` completing coverage surfaces `glyph_source=Supplied`.
  Outlined-text target refused with 14.1's existing "no font resource"
  reason. Faux bold/italic, `Tc`/`Tw`/`Tz`/`Ts`, reflow, subsetting,
  lists/alignment all correctly out of scope.

**Gates (re-verified main tree):** `cargo fmt --all --check` clean;
`clippy -p pdfce-core -p pdfce-cli --all-targets -D warnings` clean
(panic-free); `cargo tree -p pdfce-core` / `-p pdfce-render` zero GUI
deps; **ZERO new dependency**; full workspace **1134 passed / 0
failed** (14.1/14.0/Pass-4 unchanged); R59 render (notdef=0,
unsupported=0) + round-trip (reloaded=1) green on all three formatted
outputs.

**Public API surface added to `pdfce_core::text_edit` (rule-10
API-guidelines trail):** `set_format`; `FormatRequest` (new/size/fill/
font); `FormatOptions` (`with_disposition`); `FormatOutcome`;
`FormatReport`; `FormatError` (`thiserror`: `Refused`/
`CoverageFailure`/`NoOp`/`BadColor`/`TargetFontMissing`/`PageIndex`/
`NoMatch`/`Unsupported`/`Encrypted`/`Content`/`PageTree`/`Write`);
`FillModel` (`Gray`/`Rgb`/`Cmyk`; operator/arity/space_name);
`NewFill` (`new`, validated); `FontSelector` (`new`). All
`#[non_exhaustive]`, `thiserror` errors, ISO-cited doc comments.

**Acceptance (all tested):** size always works minimal-diff
colour-untouched relayout-applied; CMYK stored as `k`
(`cmyk_color_change_stores_k_not_devicergb`); non-device narrowing
disclosed + tail restored (`other_space_color_change_discloses_
narrowing`); family covering succeeds + re-encodes; partial-coverage
refused nothing-applied (`family_coverage_failure_is_refused`, exit 9,
names U+006F); `--font-dir`→`Supplied`; outlined refused;
**tagged-run colour change keeps `/MCID` 0 + discloses staleness
(`tagged_run_color_change_keeps_mcid_and_discloses`) — the
anti-Acrobat-tag-corruption test.**

**Engineer judgment calls (all defensible, none blocking — recorded
for the API-guidelines trail):**
1. State-wrap-and-restore emission (robust minimal-diff, handles
   substring + `TJ`-array matches).
2. Coverage gate is encoding-level; trust-level (Embedded/Bundled/
   Supplied) stays in the shell (CLI), not core.
3. Other-space restore via recorded raw operator bytes (not
   re-derived from a decoded model).
4. Size/colour-only edits are NOT gated by R-INV-2/3/4 (a symbolic/
   ToUnicode-only font can still be resized/recoloured) — only a
   family CHANGE runs the full classifier against the target.
5. Family target = existing page resource only (no new embedding, no
   resource-dict edit).
6. Pin = trailing compensating `TJ`; Reflow (default) adjusts
   absolute-`Tm` followers by `ΔA`; colour-only edits (`ΔA = 0`) never
   relayout.

**Honest note (not a defect):** a Calibri-Bold family change discloses
the R-INV-5 ambiguity for the space character (WinAnsi maps space at
two codes) — the inverse map picks the lowest code + discloses, the
established fuzzy-never-sneaky behavior.

**Sequencing (superseded by the Shipped entry immediately below — Pass
14.3 has now shipped in turn, and decision 014's text-editing family is
COMPLETE):** originally left Pass 14.3 as the sole remaining slice,
`pdfce-ui-specialist`'s interaction-design dispatch running in parallel;
see `### Pass 14.3` below for the completed record.

### Pass 14.3 — On-canvas text-editing UI + `EditSession` undo integration (decision 014 Pass 4 of 4, FINAL SLICE — decision 014 COMPLETE) — 2026-08-01

**The fourth and final slice of the Acrobat in-place-text-editing
subsystem (decision 014) — SHIPPED. Decision 014 is now COMPLETE
end-to-end (14.0 model + 14.1 edit + 14.2 format + 14.3 GUI).** Full
record: `docs/decisions/014-acrobat-text-editing.md`. Independently
re-verified green in the main tree: 39 core `text_edit` tests (incl. new
`EditSession` edit/format commands, undo/redo, and a
byte-identical-to-free-function minimal-diff proof) + 50 GUI tests all
pass; release GUI builds and launches without panic; the GUI was
launched live with the multi-column fixture, Edit Text tool / `Ctrl+E`
operable.

- **Core — the blocking §0.2 `EditSession` undo-integration
  prerequisite, discharged this Pass:**
  - `crates/pdfce-core/src/text_edit/edit.rs` — extracted
    `pub(crate) plan_edit(...) -> EditPlan { new_content, report }` (the
    surgery split out of `edit_text`'s save step); `edit_text` is now
    `plan_edit` + `write_incremental`; `make_raw_stream` made
    `pub(crate)`.
  - `crates/pdfce-core/src/text_edit/format.rs` — matching
    `plan_format(...) -> FormatPlan`; `set_format` delegates to it.
  - `crates/pdfce-core/src/text_edit/model.rs` — new `line_at`/
    `word_range_at`/`line_range_at` (+ `word_bounds`) accessors + 4
    tests — the substrate the deferred caret/Home/End/word-select
    refinements below will wire into.
  - `crates/pdfce-core/src/edit.rs` — `EditSession::edit_text`/
    `format_text`, `current_page_content`, `text_edit_command`, new
    `CommandKind::{EditText, FormatText}` + 6 session tests. Each edit
    applies as ONE undo-able command over the session's in-memory
    object graph; multi-edit accumulation walks the session's staged
    content (composes N sequential edits, each an independent undo
    entry); **proven byte-identical to the free function for a single
    edit** (`session_edit_output_matches_the_free_function`). The free
    functions are UNCHANGED behaviorally — 14.1's and 14.2's tests pass
    verbatim; the shared surgery was factored into `plan_edit`/
    `plan_format` so the free-function path and the session path share
    one code path rather than diverging.
- **Render/CLI hoist (de-duplication, no behavior change):**
  `crates/pdfce-render/src/font/mod.rs` — new
  `FontEnvironment::subset_stem` + `classify_nonembedded(&self, base) ->
  GlyphSource`; `crates/pdfce-cli/src/main.rs` deleted its private
  `font_subset_stem`, both subcommands now use the shared classifier.
- **GUI (`pdfce-gui`) — the first slice with a real `CanvasTool`
  variant:**
  - `crates/pdfce-gui/src/canvas.rs` — `CanvasTool::TextEdit`, the
    FIRST real variant (the previously-synthetic `resolve_escape`/
    `canvas_suppresses_pan`/gesture-interrupt branches now actually
    fire) + `text_caret_after_click` + `selection_spans_multiple_runs`
    pure functions + 3 tests.
  - `crates/pdfce-gui/src/main.rs` — `TextEditState`/`PendingEdit`,
    `OpenDoc.text_edit`, tool build/teardown, "Edit Text" toolbar
    toggle, `run_text_edit_tool` handler + helpers, two-stage `Escape`,
    `Ctrl+E` shortcut.
  - `crates/pdfce-gui/src/ui_text.rs` — ~30 Pass-14.3 strings (the §11
    UI-string list + the §8.2 hint table + strip titles).
  - Shipped the full P0 interaction spine: click → caret; Shift-click →
    extend selection; double-click → word select; rotation/zoom-correct
    caret+selection rendering (the first live consumers of the
    Pass-12.0 `canvas_to_pdf_space`/`pdf_space_to_canvas` bridges); live
    preview (mask + draft text drawn in an egui font + a dashed
    "PREVIEW — not yet applied" tag; real glyphs only render after
    commit); real Accept/Reject buttons; the verbatim disclosure strip
    + refusal strip (with the §8.2 "what would lift it" table);
    cross-run refusal; a read-only block-boundary review overlay
    (split/merge/reorder explicitly named as a deferred non-goal, in
    code); the property bar (size / colour-model RGB-CMYK-Gray / font
    `ComboBox`, trust-labelled via the shared `classify_nonembedded`).

**Named simplifications (deferred; each has its core substrate already
shipped, so they're not lost, just not yet wired):**
selection-replace-on-type (currently insert-then-backspace at the
caret, not a single replace op); triple-click (line-select),
drag-select, and arrow/Home/End caret navigation (the `line_at`/
`line_range_at` accessors Home/End needs are shipped and tested, ready
to wire); property-bar edits apply via an explicit "Apply" button
rather than commit-on-focus-loss. GUI wiring is compile-and-launch-
verified (not headless-unit-tested); the pure state-machine functions
and all core/model commands ARE headless-tested.

**Gates (re-verified main tree):** `cargo fmt --all --check` clean;
`clippy --workspace --all-targets -D warnings` clean; `cargo tree -p
pdfce-core` / `-p pdfce-render` still zero egui/eframe/winit/wgpu/glow
(GUI-core separation intact); `cargo test --workspace` — 23/23
binaries, 0 failures (677 core tests incl. the §0.2 tests, 42 gui incl.
the new canvas tests per the build report; independently re-run this
session at 39 core `text_edit` + 50 gui, all green); R59 + round-trip
green (the byte-identical session-vs-free-function test is itself the
direct minimal-diff proof); GUI release build launches, no startup
panic; **ZERO new dependency**.

**Public API surface added (rule-10 API-guidelines trail):**
`pdfce_core`: `EditSession::edit_text`/`format_text` (return the
`text_edit::EditError`/`FormatError` types directly, not a
session-local error); `CommandKind::{EditText, FormatText}`;
`EditableTextModel::{line_at, word_range_at, line_range_at}` (+
`word_bounds`). `pdfce_render`:
`FontEnvironment::{subset_stem, classify_nonembedded}` (reuses the
existing `GlyphSource` enum, no new type). All snake_case,
`#[non_exhaustive]` where applicable, ISO-cited doc comments.

**Engineer judgment calls (all defensible, none blocking — recorded
for the API-guidelines trail):**
1. "Free functions unchanged" is read as *behaviorally* unchanged — the
   `plan_edit`/`plan_format` split is a mechanical extraction; 14.1's
   and 14.2's tests pass verbatim, unmodified.
2. Multi-edit accumulation walks the session's staged content; a
   first-edit-gated extra-stream-emptying step keeps undo history clean
   across repeated edits.
3. Session methods surface `text_edit::EditError`/`FormatError`
   directly rather than wrapping them in a new session-local error
   type.
4. The live preview draws draft text in an egui font (no new
   font-shaping dependency) — a deliberate visual-only approximation;
   the committed glyphs are the real, spec-correct ones.
5. The delegated GUI sub-fork returned 0 tool-uses this session, so the
   builder implemented the GUI slice directly rather than through a
   further sub-dispatch.

**MILESTONE — decision-014's Acrobat in-place-text-editing subsystem is
now COMPLETE end-to-end** (core model → in-place edit → formatting →
GUI edit tool, 14.0 through 14.3, all four slices shipped). The
operator's directed "Acrobat text-handling parity" focus (Backlog's ★
NEXT MAJOR FOCUS, filed 2026-08-01) is **substantially achieved at the
P0 level**. The GUI was launched for the operator with the
multi-column fixture, Edit Text tool / `Ctrl+E` live. Remaining
text-parity work is the reflow ladder (**FF-A** within-block reflow
next, **FF-B** cross-block after) plus the named GUI refinements above
(selection-replace-on-type, triple-click/drag-select/arrow-key nav,
commit-on-focus-loss) — none of these are core-substrate gaps; all are
GUI-wiring or reflow-architecture follow-ups over already-shipped,
tested primitives.

### Pass 13b — Rebuild-by-scan cross-reference recovery (decision 013 Pass B — SHIPPED) — 2026-08-01

**The #1 real-world robustness fix — SHIPPED, closes decision 013.** Full
design + acceptance: `docs/decisions/013-xref-recovery.md` §3.3–§5.
Librarian-assigned **Pass 13b** (after Pass 13a).

**Headline (real-world corpus, 1,109 files: qpdf Apache 639 / pdfium
BSD 331 / PDFBox Apache 139):** **566 previously-strict-failing files
now open.** Reason-bucket breakdown (counted, not rounded):
`NotAnXrefSection` 417, `TrailerParse` 99, `BadEntry` 20,
`BadXrefStream` 13, `StartxrefNotFound` 7, `BadStartxrefOffset` 7,
`MissingHeader`/offset-start 3.

**Zero regression (veraPDF, 2,907 files):** 2,892 clean files load via
the STRICT path completely unchanged; **0 clean files were diverted
into recovery** (verified empirically by an object-outcome tally, not
assumed). 6 still-failing files (`BadObject`) unchanged.

**`*-fail-*` reconciliation — COMPLETE (the hardest gate).** All 5
veraPDF status changes (refused → opens under recovery) are
PDF/A-conformance `*-fail-*` files that fail a File-header or
colour-space CONFORMANCE rule — never an xref-parse bug. Defensible
reader recovery: qpdf and pdfium open these same files too. Recovery
fires because the file's deliberate header manipulation (the thing the
conformance test is checking for) also happened to invalidate the
stored xref offsets — an honest side effect, not a masked bug.
Enumerated individually and justified in the build report per decision
013's hardest-gate requirement.

**Named non-goal (unchanged from scope):** 53 still-failing real-world
files carry OBJECT-LEVEL corruption AFTER a clean xref recovery — a
documented non-goal, filed to Backlog as a future Pass (object-level
lenient loading), not silently absorbed into this Pass's scope.
Encrypted-and-refused: 58 (unchanged capability gap, Pass 5's
territory). Recovery-refused: 9 (`NoCatalog` 2, `NoObjects` 7).

**Gates:** fuzz green (21,595 runs, 0 crashes); `cargo fmt --check` /
`clippy -D warnings` clean; `cargo tree -p pdfce-core` /
`-p pdfce-render` GUI-dep-free; **ZERO new dependency**; full workspace
tests green (638 `pdfce-core` lib tests + integration suites).

**Demo:** `add-contents.pdf` (a Pass-13a canonical offset-shift example)
opens `(recovered)`; `round-trip --mode full` on it produces a clean,
independently-reloadable PDF; `save_incremental` on the recovered
document is refused BY NAME (CLI exit 8); the CLI's own recovery-load
path reports exit 11 (recovery occurred, disclosed).

**Files shipped:** `crates/pdfce-core/src/recover.rs` (new); edits to
`document.rs` / `objstm.rs` / `xref.rs` / `writer/{mod,save}.rs`;
`crates/pdfce-cli/src/main.rs` (new exit code 11); `crates/pdfce-gui/
src/main.rs` (recovery banner disclosure); fixtures
`fixtures/synthetic/xref-recover/` +
`tools/gen-xref-recover-fixtures.py`; tests
`crates/pdfce-core/tests/xref_recover.rs`; fuzz
`fuzz/fuzz_targets/recover_roundtrip.rs`; `tools/recover-sweep/`.

**`ARCHITECTURE.md` §5.10 FLIPPED from "pending Pass-13b ship" to
shipped/active** — the recovered-base-forces-full-rewrite contract is
now enforced code, not a forward-looking design note. **Standing rule
R67 is now IN FORCE** (previously filed against not-yet-shipped code).

**Deviations (disclosed, both flagged by the engineer, neither actioned
by the librarian):**
1. **Code-comment number lag (being discharged this session).** Pass-13b
   code comments in `recover.rs` cite the recovered-base rule
   descriptively as "~R62/R59" — the canonical number is **R67**. The
   engineer is fixing this in-session (R59→R67 in `recover.rs`);
   recorded here as being discharged, not as an outstanding owed item.
2. **`gen-65536` deviation (deliberate, flagged, NOT a scope
   violation).** Rebuild-by-scan opens some recoverable gen-65536 files
   via the `BadEntry` trigger — this IS one of decision 013's target
   buckets (a malformed generation number is exactly the kind of entry
   corruption recovery is meant to route around). This is **NOT** the
   separate strict-parser gen-65536 TOLERANCE question flagged by Pass
   13a (Backlog) — the strict parser still correctly REJECTS gen 65536
   today; only the recovery path (which never consults the original
   malformed entry) opens these files. A defensible, named, flagged
   deviation that does not foreclose the future tolerance decision.

### Operator-supplied fonts (decision 012 first cut) — 2026-08-01

**Non-embedded, non-Base-14 SIMPLE fonts can now be drawn from an
operator-supplied font folder** — the fix for the operator's real drawing
that rendered `Calibri` as a bundled Helvetica substitute. Rides the
`FontEnvironment.named` seam decision 004 §5.3 built for exactly this;
**ZERO new dependencies** (`std::fs` + the one skrifa parser, R21). Adds
standing rules **R62–R66** (renumbered from the record's proposed R61–R65 —
R61 was taken; see Standing rules). Full record:
`docs/decisions/012-operator-supplied-fonts.md`.

- **`pdfce-render`:** replaced `LoadedFont.substituted: bool` with
  `GlyphSource { Embedded, Bundled, Supplied }`; `substitute_face` returns
  the source it drew from and now retries the named lookup after
  `strip_subset_tag` (so `ABCDEF+Calibri` / `Calibri,Bold` resolve to a
  supplied `Calibri`); new `face_names(&[u8]) -> Vec<String>` on the ONE
  skrifa parser (R21 — no second parser); `Diagnostics` gains
  `glyphs_supplied` + `supplied_fonts`, DISTINCT from the bundled
  `glyphs_substituted` / `substituted_fonts` (three trust levels, R63).
- **`pdfce-cli`:** `--font-dir <DIR>` (repeatable) on `render-page` — the
  shell walks the folder, parses face names, builds a `FontEnvironment`,
  passes it via `RenderOptions`; the summary prints the three-way
  disclosure (bundled count+names vs supplied count+names).
- **`pdfce-gui`:** a "Font folders" tool feeding the same seam.
- **`pdfce-core`:** untouched (no font-folder logic).

**Acceptance (all met):** a non-embedded `Calibri` PDF renders bundled
Helvetica WITHOUT `--font-dir` and the supplied `Calibri` WITH it; glyph
**positions are BYTE-IDENTICAL** across both runs (proof that positions
come from `/Widths`, not the face — R63); subset-tag + style-variant
references resolve to matching supplied variants else fall to bundled
(disclosed); a corrupt supplied file fails clean to bundled (skip-and-note,
never errors the page); composite non-embedded still returns
`CompositeNotEmbedded` (unchanged hard skip); the **R64-equivalent
font-dir-independence gate** confirms the R59 corpus output is byte-
identical regardless of any font-dir config. **1,045 tests, all gates
green, ZERO new deps, release rebuilt.** `cargo tree -p pdfce-render` adds
zero packages; wasm32 green; no `cfg(target_os)` in core/render.

**Deviations (disclosed):** (1) `--font-dir` is `render-page`-only this cut;
(2) the GUI "Font folders" setting is **session-state, NOT persisted** — the
R15 user-state partition does not exist yet, so persistence is DEFERRED with
it (recorded against the crash-safety/user-state Backlog bucket); (3) the
inline "supply this font" GUI link is deferred. **Fast-follows (named):**
FF1 OS-font-directory enumeration (opt-in, R66); FF2 composite/CID
substitution via the Unicode route (R65); FF3 descriptor-based auto-routing
to a supplied face.

**FONT-ON-EDIT CONNECTION (recorded):** decision 012 is the **enabler for
the upcoming Acrobat text-editing subsystem** (the ★ NEXT MAJOR FOCUS
Backlog bucket) — a typed/edited glyph run needs the font available to draw
it, and font-handling-on-edit couples directly to this supply path plus the
R17 Base-14-Latin-only limit.

### Pass 12.0 — Canvas-interaction substrate (decision 010 candidate B / decision 011 beta foundation) — 2026-08-01

**The single shared canvas-interaction substrate R60 mandates — shipped as
a FOUNDATION with ZERO document-mutating tools.** This is decision 010's
Pass 12 / candidate-B foundation slice, **doubly justified** (continuation
32): it is the substrate BOTH the Acrobat text-editing arc AND the
measurement/vector work consume. Ships UNINHABITED so viewer behavior is
unchanged. **ZERO new deps, release rebuilt.**

- **New `crates/pdfce-gui/src/canvas.rs`:** `CanvasTool` (ships
  **UNINHABITED** — no tool variants this Pass), a `CanvasTargetProvider`
  trait + `EmptyTargetProvider`, the selection-set model, and pure
  state-machine functions with tests.
- **`viewer.rs` — four geometry bridges:** the existing
  `screen_to_page` / `page_to_screen` PLUS the **new
  `canvas_to_pdf_space` / `pdf_space_to_canvas`**, built by inverting
  `page_device_geometry`'s `Transform`. **A genuine finding beyond decision
  011's literal 12.0 deliverables:** correct device-Y-down ↔ PDF-Y-up
  coordinate-space handling. Transforms **proven correct at 0/90/180/270°**
  rotation and under 1/zoom invariance by test.
- **`main.rs` wiring:** a focusable canvas (`Sense::click_and_drag`),
  pan-suppression, four-way `Escape` precedence, and the preview overlay.
- **Rename `MarkupTool` → `CanvasTool`** (permanent; noted against the
  pass-6.1 / pass-8 UI specs so a future reader isn't confused).

**Gates:** 47 gui tests; full-workspace gates green; **GUI-core separation
intact** (`cargo tree` core+render still egui/eframe/winit/wgpu-free);
wasm32 clean. Release rebuilt — canvas is focusable but tool-less, so
viewer behavior is unchanged.

**Deviations / notes:** (1) image drag-sense is gated on `suppress_pan`
(egui 0.35 pans content before the widget sees the drag); (2)
`target_provider = Some(EmptyTargetProvider)` rather than `None`
(observably identical, cleaner). **Out-of-scope pre-existing fix folded
in:** a doc-comment clippy error in `pdfce-core/document.rs` (zero
functional impact). **Residuals (who plugs what):** Pass 9a plugs the real
target provider + marquee-vs-pan; Passes 6.1/8/12.M2/9c-min plug real
`CanvasTool` variants; Pass 7 reconciles global-vs-focused keyboard
handling.

### Pass 13a — Cross-reference EOL/CRLF audit (decision 013 Pass A — NEGATIVE RESULT, filed) — 2026-08-01

**Decision 013's Pass A: a measurement that confirmed the classic
xref-table parser is EOL/CRLF-correct — the expected negative result,
filed.** No parser code changed (tests + fixtures + tools only). Confirms
the CRLF failure correlation is **offset-shift corruption, not a parser
bug**, so the entire recovery burden falls on Pass 13b (rebuild-by-scan).
Full record: `docs/decisions/013-xref-recovery.md` §3.2. Librarian assigned
**Pass 13a** (decision 013 delegated the number).

- **Finding:** 9 synthetic legal-EOL fixtures (SP CR / SP LF / CR LF on
  entry lines AND subsection-header/`xref`/`trailer`/`startxref` lines;
  multi-subsection; trailing-space; bare-CR; mixed-EOL) all parse. The
  **547 of 567 sampled real failures are OFFSET-SHIFT corruption** (LF→CRLF
  text-mode byte-growth invalidating both `startxref` and every in-table
  offset), **0 genuine parser bugs**. The canonical example:
  `qpdf/add-contents.pdf` stores `startxref 685` but the real `xref` is at
  byte 724 (a 39-byte forward shift) — a file whose stored offsets cannot
  be trusted at all, which a parser tweak cannot fix and rebuild-by-scan
  solves head-on.
- **Artifacts (all out-of-tree / test-only):** `fixtures/synthetic/xref-eol/`,
  `tools/gen-xref-eol-fixtures.py`, `tools/xref-crlf-classify.py`,
  `tests/xref_eol.rs`.
- **Gates:** `cargo fmt --check` / `clippy -D warnings` clean; the 2,890
  clean conformance files show zero change in load outcome (the hot-path
  guard); no `pdfce-core` source touched.
- **`gen-65536` tolerance candidate surfaced (separate future decision):**
  17 files carry an out-of-spec generation number > 65535; this is **NOT
  CRLF-related**, and pdfce correctly rejects them strict. Filed as a
  separate future tolerance decision + a `personal_rag/pdf` finding — not
  part of xref recovery.

### Test infrastructure — font-parity harness + real-drawings smoke + OSS-corpus expansion — 2026-08-01

**Three standing test gates / tools shipped, all corpus-scale, none adding
a shipped dependency.**

- **(a) Font-parse regression harness — `tools/font-parity/`.** Parses
  every embedded font program in the corpus and asserts routing-or-clean-
  fail (0 misroutes). Guards the NUL-misroute bug permanently; its standing
  rule is **R68** (see Standing rules). Re-runs on any `font/program.rs` /
  font-layer change.
- **(b) Real-drawings smoke — `tools/realdrawings-smoke/`.** The operator's
  private **read-only** `R:\Products` render smoke test — **results are
  gitignored, nothing proprietary is committed**. Confirmed the font fix
  holds across **339 real drawings, `unsupported=0`**.
- **(c) OSS-corpus expansion (`fixtures/external/`, gitignored).** +1,109
  real-world PDFs added — pdfium (BSD, 331), qpdf (Apache, 639), PDFBox
  (Apache, 139) — each with per-source `PROVENANCE`. **pdf.js was SKIPPED**
  (unclear per-file provenance); GPL/AGPL-project corpora were avoided
  (rule 7 / LEGAL §5). The corpus is now **~4,000 files**. Sweep tooling:
  `fixtures/external/realworld-sweep.sh`. This sweep is what surfaced the
  ranked real-world gaps (see Backlog) and the 85%-xref-recovery finding
  (decision 013).

### Font-fix — NUL-misroute of no-cmap CIDFontType2 embedded TrueType (render-parity footer IN, COMPLETE) — 2026-08-01

**The root-cause font bug behind the operator's real drawing rendering with
missing text — fixed, verified corpus-wide, COMPLETE.** A subset
CIDFontType2 (embedded TrueType, no `cmap`, legal per §9.7.4.2) was
misrouted to the CFF parser and failed. Fixed pdfce-side; **skrifa stays
0.42.1 pinned — the root cause was pdfce-side routing, no bump needed.**
**ZERO dependency change.**

- **Root cause:** font-program format detection trimmed leading whitespace
  **including NUL (0x00)** before magic-sniffing → the leading NUL of the
  sfnt magic `0x00010000` was stripped → the remaining `01 00 …` matched
  bare-CFF magic → TrueType bytes were handed to the CFF parser, surfacing
  as an "offset out of bounds" that *looked* like a read-fonts objection
  but was a **caller-side misroute**. **Fix:** match binary magics on RAW
  bytes; trim only on the Type 1 `%!` text path; never treat NUL as
  whitespace. `extract-text` was unaffected (the ToUnicode path never
  parses the program). Class impact: **all embedded TrueType from
  SolidWorks / AutoCAD / Office CAD.**
- **Render-parity footer — GATE PASS (the R59 gate, --max-unexplained 1,
  exit 0) over the full 2,914-file / 2,922-page corpus:** unexplained
  **1→1** (NO regression — the 1 is the pre-existing A019 f32-coordinate
  case, unrelated); font-unsupported gap histogram **7→0** (the fix
  converted every no-cmap CIDFontType2 page out of the shortfall — the whole
  SolidWorks/AutoCAD/Office CAD class); benign **2840→2868**; known-gap
  **49→53** (a CORRECT net rise — text now renders on those pages, revealing
  already-disclosed shading/marked-content gaps previously MASKED by the
  whole-font skip; not new divergence); band re-derived data-driven
  **0.02942→0.02963**.
- **Diagnostics:** new `Diagnostics::fonts_unsupported_by_reason` keyed
  Type3 / NonIdentityCmap / VerticalWriting / CompositeNotEmbedded /
  UnknownSubtype / UnusableProgram (+6 appended CLI stdout tokens).
- **Regression fixture (synthetic CC0, never the proprietary file):**
  `fixtures/synthetic/text/cidfonttype2-nocmap-embedded.pdf` +
  `tools/gen-cidfont-nocmap-fixtures.py` + `tests/cidfont_nocmap_render.rs`.
- **Gates:** 1,018+ tests, all gates green, ZERO dep change, release
  rebuilt. **STATUS: COMPLETE** (the earlier "corpus-regression footer
  owed" residual is now discharged by the GATE PASS above).
- **Residual (named future item):** no dedicated `font_program` fuzz target
  exists yet — filed to Backlog.

### GUI polish (current feature set) + launcher — 2026-08-01

**Not a feature Pass — an operator-requested polish/usability interlude**
(Ken: "get the GUI polished up for the current feature set, then give me a
way to launch it from `D:\Dev\pdfce`"). Deliberately filed WITHOUT a Pass
number: it ships no new document capability, touches only `pdfce-gui` +
`ui_text.rs`, and adds ZERO dependencies. The measurement/dimensioning beta
(decision 011) remains the queued next major work — this interlude does not
displace it. Executed against `docs/ui_specs/gui-polish-current-featureset.md`.

**Scope (pdfce-gui + `ui_text.rs` only, zero new deps):**
- **All 6 P0 items:**
  1. `open_path()` resets stale per-document narration (`edit_note` /
     `copy_result` / `copy_detail_expanded` / `pending_text_kind` /
     `text_input`) — no bleed-through from the previously open file.
  2. Properties panel reseeds when a second file is opened (no empty grid).
  3. Window title reflects the open file (`ViewportCommand::Title`; new
     `ui_text` `window_title_idle` / `window_title_open`).
  4. Status-bar height cap (`ScrollArea` `max_height=220`) — no disclosure
     suppressed.
  5. Real empty state (pdfce heading + inline Open button + drop hint) +
     working drag-and-drop (`dropped_files`, `.pdf`, restricted to
     Idle/Failed/Unsupported so unsaved edits can't be silently discarded).
  6. Annotation-visibility toggle uses `ICON_BUTTON_SIZE`.
- **All P1 items:** colour-is-not-the-sole-signal on the four toggles (bold
  active label); a keyboard-shortcuts reference window (⌨ button,
  `ui_text::shortcuts_reference`, doc-commented to stay in step with
  `collect_keyboard_actions`); text-menu wording + colour note + per-add
  jitter (`author_jitter` mod-6×12pt, so repeated author-at-center adds
  don't stack invisibly); utility-cluster spacing; Revert-disabled tooltip;
  **accessible names on every glyph-only icon button** via a new
  `Self::icon_button()` helper (egui 0.35 `Response::widget_info` +
  `WidgetInfo::labeled`/`selected` — API verified available in 0.35).
- **Launcher (repo root, NEW):** `D:\Dev\pdfce\pdfce.bat` and
  `D:\Dev\pdfce\pdfce.ps1` — double-clickable / drag-a-PDF-onto-it /
  `pdfce.bat [file]`. Both `cd` to the repo root, run `cargo build --release
  -p pdfce-gui` (fast freshness check → always launches the latest build),
  then `Start-Process` the exe detached with an optional file arg.
  Smoke-tested end-to-end (release GUI launches, no startup crash).

**Gates:** `cargo fmt --check` / `clippy -D warnings` clean; **31 pdfce-gui
tests pass**; GUI-core-separation invariant confirmed (`cargo tree -p
pdfce-core` / `-p pdfce-render` still egui/eframe/winit/wgpu/glow/rfd-free);
`ui-strings` R1 gate clean. Release rebuilt + smoke-tested. **ZERO new
dependencies.**

**Deferred polish residuals (named follow-ups, NOT built — filed to
Backlog):** P2-1 recent-files list (needs settings persistence); P2-2
window/taskbar app-icon asset (needs artwork); P2-3 light-mode visual QA
pass (no hardcoded colours added — stays OS-theme-driven); P2-4 markup
colour-picker tooltip; P2-5 screenshot-driven spacing QA.

**TWO DATA-SAFETY items flagged as NOT-polish — real, still-open (the
crash-safety / non-destructive-by-default standing UX rule, ui-specialist's
territory):** (1) **no autosave / crash-recovery scratch file exists** — an
unsaved editing session is lost on a crash; (2) **true in-place Save remains
(correctly) gated on that autosave existing** — "Save a copy" is still the
only save affordance. These two are a standing-UX-rule GAP, not cosmetic
polish, and carry their own tracked Backlog item (below). Recorded
prominently so a future session does not mistake them for done.

### Pass 11 — Render-fidelity verification harness (full-page pdfium pixel-parity, corpus-scale) — 2026-08-01

**PURE MEASUREMENT — decision 010's candidate C, the first Pass of the
C → B → A sequence, shipped with ZERO Rust touched and ZERO new pdfce
dependency.** Proves pdfce's render stack against an *independent* reference
renderer (pdfium via pypdfium2) at corpus scale, replacing "pdfce agrees with
pdfce" (the self-comparison round-trip oracle, which structurally cannot judge
visual correctness) with a measured, bucketed, by-file/by-reason fidelity
report. This is the correctness oracle Pass 9 (vector editing) newly requires —
vector editing is the first subsystem whose acceptance test is independent
*visual* fidelity, and this Pass is the only thing that provides it. Wires the
new **R59** render-fidelity gate to a concrete baseline. pypdfium2 is
dev-tooling ONLY (out-of-tree, NOT vendored, absent from
`THIRD_PARTY_LICENSES.md`).

- **New files (all out-of-tree, mirroring `tools/content-identity/`):**
  `tools/render-parity/render_parity.py` (the harness — drives `pdfce-cli
  render-page` + pypdfium2, aligns rasters, computes per-channel per-pixel
  deltas); `tools/render-parity/README.md` (band derivation, bucket
  definitions, gate role, LEGAL/invariant notes, non-goals);
  `tools/render-parity/out/{summary.txt,summary.json,per-page.tsv,diffs/}`
  (the full-corpus report). No `pdfce-core`/`pdfce-render`/`pdfce-gui`/
  `pdfce-cli` source changed.

**Tolerance band — EMPIRICAL, not tuned (the analytical core, Y1/W14):** the
metric is `frac_over_32` = fraction of pixels whose max-channel absolute delta
exceeds 32/255. Rationale: benign anti-aliasing / hinting / sub-pixel noise is
confined to a thin *edge band* (small AREA) even where individual edge pixels
swing full-range — so the noise-robust discriminator is the AREA fraction, not
the max per-pixel delta. The band = **p99.9 of `frac_over_32` over the 1,728
clean-by-construction pages** (pages for which pdfce discloses zero gaps AND
which carry no DeviceCMYK) — a property of the *known-benign* population, so it
**cannot be tuned to make a bug pass** (W14 structurally satisfied). This run:
**band = 0.0294**; clean-floor mean 0.00096 / p95 0.0022 / p99 0.0098 (tight,
well-separated from the band). The report always prints the DISTRIBUTION, never
a bare pass/fail.

**Three-bucket classification — full loadable corpus (2,914 files → 2,890 pages
at 125 DPI, content-only; ZERO panics / ZERO timeouts; 24 skips = unloadable
`fail-*` conformance files):**
- **(i) benign-renderer-noise — 2,840 pages.**
- **(ii) known-disclosed-gap — 49 pages,** cross-referenced against pdfce's
  EXISTING Diagnostics tally so already-counted gaps are SUBTRACTED, not
  re-reported (48 deferred-op sh/OC/Type3; 7 font-unsupported; 6 DeviceCMYK
  file; 2 substituted; 2 image-unsupported; 1 codec-feature — a page may carry
  more than one reason).
- **(iii) unexplained-divergence — 1 page.**

**Bucket-(iii) triage — the single unexplained page:** TWG test-suite
`A019-pdfa2-pass-a.pdf` — a form XObject fills a triangle with a vertex at
x ≈ 3.4028e38 (≈ `f32::MAX`); pdfium rejects/clips the out-of-range path,
pdfce rasterizes a spurious cyan bar. **FILED as a named, counted render-gap
(R20/R27), NOT fixed** — a render-robustness edge case (a path coordinate
overflowing under the CTM) on one deliberate torture file; the fix is a
clamp/reject-policy decision in `pdfce-render` (R34 risk), not a cheap clear,
and the measurement-only non-goal (Y3) binds.

**Reference-divergences encoded (Y2):** the default run is content-only, which
structurally avoids pdfium's annotation-rendering quirks; a `--annots` mode
buckets REFERENCE-SIDE (3 pages verified) the pdfium behaviors pdfce must never
be blamed for — `/Widget` appearances needing `FPDF_FFLDraw`, and pdfium's
synthesized no-`/AP` looks that R43 makes pdfce correctly REFUSE.

**DeviceCMYK — characterized, filed as the FIRST NAMED RESIDUAL (NOT fixed):**
DeviceCMYK-only pages diverge at **3.0× the clean-page mean** corpus-wide;
anchored to decision 006's file, the delta lights the ENTIRE filled area
uniformly with POLARITY IDENTICAL (R29 holds) — the naive additive
`Rgb::from_cmyk` vs pdfium's `AdobeCMYK_to_sRGB1` gap. NOT fixed here (Y5:
don't confound a colour question with the harness build; decision 006
revisit-trigger 7 requires re-pinning the §3.4 polarity matrix FIRST). Filed as
a follow-up COLOUR PASS, scopeable via `pdfce-acrobat-librarian` (the
already-filed uncalibrated-DeviceCMYK question). Cross-refs decision 010
revisit-trigger 6.

**Standing gate wired (R59 now has a concrete baseline):** `--gate
--max-unexplained <baseline>` returns non-zero when the unexplained count
rises; **baseline = 1** (the A019 file); verified PASS at baseline. Documented
as a REQUIRED re-run on every render-touching Pass — especially Pass 9 vector
editing (the R34/R46 re-run pattern). Local-corpus gate (pypdfium2 is not in
CI, exactly like content-identity / roundtrip).

**Pass 1.1 pixel-parity remainder — CLOSED (discharged), stated exactly:** the
harness genuinely generalizes to full-page corpus scale (per-channel
per-pixel; full loadable corpus; first-page coverage of every file; multi-page
via `--pages-per-file 0`, demonstrated). This meets decision 010's exact bar,
so the long-owed **full-page pixel-parity remainder (Pass 1.1) is
DISCHARGED** — scope named precisely (first-page corpus coverage + a multi-page
knob), NOT overclaimed as exhaustive-multi-page or pixel-perfect. **Struck from
the SESSION_LOG "still open" lists going forward** (the discharge is noted; prior
entries are not rewritten).

**Gates:** `cargo fmt --check` clean; `cargo tree` core+render GUI-free; **ZERO
Rust delta → clippy/test/R34/R46 unmoved BY CONSTRUCTION** (no source changed);
no `Cargo.toml` change → `THIRD_PARTY_LICENSES.md` unchanged;
deterministic/locale-invariant (sorted file order, fixed DPI, no clocks).

**Deviations / residuals (ALL disclosed):**
1. **A019 `f32`-max path-coordinate render-gap** — filed, not fixed
   (render-robustness; the clamp/reject-policy call is Pass-9-adjacent, R34
   risk). New Backlog item filed.
2. **DeviceCMYK→sRGB colorimetry** — the FIRST named residual → a follow-up
   colour Pass (adopt a calibrated table à la `AdobeCMYK_to_sRGB1`; re-pin
   decision 006 §3.4 polarity matrix FIRST; scope via
   `pdfce-acrobat-librarian`). New Backlog item filed.
3. **Full sweep uses first-page coverage for breadth** — multi-page is a
   demonstrated knob (`--pages-per-file 0`), a stated parameter, not a hidden
   limit.
4. A transient `nohup`-detach Bash-backgrounding gotcha was hit and resolved
   during the corpus sweep — escalated to `D:\dev\rag\rust\`.

**RAG escalations:** `C:\personal_rag\pdf\` — the **area-fraction-not-max-delta**
tolerance-band methodology (separating benign independent-renderer noise from
real divergence: area-confined AA vs area-wide systematic gap; band derived from
the clean-by-construction population so it cannot be tuned to pass).
`D:\dev\rag\rust\` — the `nohup`-detach background-sweep gotcha. Subject +
master + subdir indexes updated.

**MILESTONE / sequence:** the render stack is now VERIFIED against an
independent reference at corpus scale — the ground under the editing arc is
proven. **Decision 010's C → B → A sequence CONTINUES after the reprioritized
beta** (see In progress): Pass 12 (canvas-interaction foundation) → Pass 9
(vector editing), promoted onto this now-trustworthy render. R59 is discharged
for the first time.

**Still open (unchanged MINUS the now-discharged Pass 1.1 pixel-parity item,
ordered oldest-first):** encryption-refusal operator sign-off (oldest owed);
LEGAL.md §2 Adobe-supplement copyright contradiction; `/R 6` sourcing method;
license decision; commit authorization (**Passes 0–8.0 + the
`tools/render-parity` additions ALL uncommitted in git**); W15 (no remote/CI).

### Pass 8.0 — Redaction (mark + apply, text + region) — 2026-08-01

**The highest-stakes Pass in the project — and the cardinal rule held:
never claim redacted what isn't.** This discharges the standing **R35**
obligation and is the one operation whose contract is genuine REMOVAL, not
minimal-diff preservation (§5's sole deliberate exception, R46's one named
content-stream-surgery exception). Redaction MARK and APPLY are separate
operator actions (R52): a mark is a reviewable, reversible `/Redact`
annotation drawn as a RED OUTLINE preview (never a solid fill — the
mark-vs-apply distinction made visible); apply is the destructive act that
proves the covered bytes are GONE from the entire saved file.

- **New `pdfce-core` `redact.rs`** — the self-contained,
  **advance-preserving content-stream surgery interpreter** + the apply
  orchestration, the carrier sweep, the container decomposition, the
  `RedactionReport`, and `count_redaction_marks`. This is the one op that
  DOES rewrite existing page content (the R46 named exception), the mirror
  image of Pass 7.1's overlay-append flatten.
- **`edit.rs`** — `add_redaction`, `mark_redactions_by_search` /
  `_by_pattern`, `find_matches` / `find_pattern_matches`.
- **`annot_author.rs`** — `RedactSpec` + `build_redact_mark` (RED-OUTLINE
  preview, never a solid fill — the mark is a proposal, not a result).
- **`text_extract/font.rs`** — exposed `codes` / `width` / `to_unicode` /
  `bytes_per_code` / `width_estimated` / `base_font_name` as `pub(crate)`
  (the surgery interpreter needs per-code widths to compute the exact
  advance to preserve).
- **CLI:** `redact-mark` (`--rect` / `--search` / `--pattern`),
  `redact-apply` (emits the `RedactionReport` + `--acknowledge-residuals`
  gate; **exit 10 `REDACTION_RESIDUALS`** when undisclosed-not-scrubbed
  carriers are present and not acknowledged), `list-redactions`.
- **GUI — the ONE non-negotiable item:** a persistent status-bar
  disclosure of unapplied `/Redact` marks, computed from the document's
  own annotations. This targets the #1 real-world redaction failure mode —
  saving a marked-but-not-applied document believing it is redacted. The
  GUI apply-button + canvas marking are deferred (see deviations).
- **Fuzz target 15 `redact_apply`**; `tools/gen-redact-fixtures.py` +
  `fixtures/synthetic/redact/`.

**THE HEADLINE — ABSENCE PROOF PASSES (R46 INVERTED):** demo on
`demo-secret.pdf` ("SECRET" in heading + body, "PUBLIC" surrounding text,
`/Info /Title (SECRET dossier)`): `redact-mark --search SECRET` → **3
marks, document NOT yet redacted**; `redact-apply` → `glyphs_removed=21
info_strings_scrubbed=1`; `grep "SECRET" redacted.pdf` → **0** (control
`marked.pdf` → 3). ZERO occurrences in the ENTIRE saved file — raw bytes
AND every decoded content stream. Redaction's four-shalls are embodied as
an executable acceptance gate: grep the whole output for the redacted
bytes → zero (**R46 inverted** — absence proven for redacted content,
presence preserved for everything else).

**The proofs:**
1. **Advance preservation.** The redacted-page render shows "SECRET" as a
   baked black box while "dossier" / "PUBLIC text" sit EXACTLY where they
   were — not shifted left. Proven visually AND numerically (survivor
   x-origin moved <1.0 pt). The removed run is replaced by a `TJ` advance
   `N = −Σtx·1000/(Tfs·Th)` so surviving same-line text stays positioned.
2. **Container decomposition (§7.5.7 Strategy B).** A redacted `/Info`
   compressed inside an `/ObjStm` would survive verbatim without
   decomposition (§5.7 — object streams carry through in BOTH save modes);
   the test proves absence AND `containers_decomposed >= 1` (promote the
   surviving objects, drop the container).
3. **Forced full rewrite (R35).** The output has no `/Prev`; prior
   revisions (which hold the un-redacted content) are dropped; every
   carrier scrub rides `save_full`.
4. **Image handling — REFUSE by name** (`RedactError::ImageRegion`, NO
   output written) rather than overlay-and-leave-pixels. The documented
   choice: never falsely claim a raster region redacted when only a masking
   box was drawn over intact pixels.
5. **Carrier sweep / report.** `/Info` + XMP SCRUBBED (asserted absent);
   object-streams + prior-revisions DROPPED-BY-REWRITE; OCG
   REDACTED-BY-GEOMETRY (ignores `/OC` visibility); XFA / structure-tree /
   attachments DETECTED + DISCLOSED (`DISCLOSED_NOT_SCRUBBED`), gated by
   the refusal-acknowledgement gate (`--acknowledge-residuals`; exit 10
   otherwise — ui-spec §4.4).

**Gates:** **1,018 workspace tests (+8)**; `cargo fmt --check` /
`clippy -D warnings` clean workspace-wide; GUI-free core+render invariant
(zero egui/eframe/winit/wgpu); wasm32; `--duplicates`; `no-network`;
`ui-strings` all clean; **R34/R46 additive-preserved** (the `writer/` +
`content.rs` re-emission paths and gates are byte-unchanged — surgery is a
NEW code path, the identity path does not move); fuzz **target 15
`redact_apply`** 9,262 runs / 61 s, 0 crashes (multi-byte CID, nested
q/Q, overlapping/degenerate quads, all-covered / none-covered — the
security assert held); **ZERO new dependencies**, `THIRD_PARTY_LICENSES.md`
unchanged. GUI launched PID 40828.

**NEW STANDING RULE R58 (the ui-specialist finding — generalizes R35):**
every removal/scrub operation must ride R35's forced FULL REWRITE —
including any future Sanitize / Remove-Hidden-Information — because an
incremental save leaves the "removed" content recoverable in the prior
revision, defeating removal. R35 covered redaction-apply specifically;
R58 generalizes it to ALL scrub operations. See Standing rules R58,
`ARCHITECTURE.md` §5.9, and `docs/ui_specs/` (the ui-spec that surfaced
it).

**Deviations / residuals (ALL disclosed, none silent):**
1. **Image pixels REFUSE-not-clear** (`RedactError::ImageRegion`, no output
   written) — a named, safe, disclosed choice; partial raster clear was
   rejected as more error-prone than an honest refusal.
2. **`/RO` + `/OverlayText` burn-in DEFERRED** — apply draws the
   `/IC`/default-black fill (Acrobat default); the overlay-text LABEL is
   NOT drawn. This is COSMETIC only (content is removed regardless);
   disclosed at mark time.
3. **Form-XObject content in-region is NOT surgically redacted** —
   disclosed loudly (`form_intersect` note), never claimed removed.
4. **XFA / structure-tree `/ActualText` / attachments** are detect +
   disclose this cut, not scrubbed.
5. **GUI apply-button + canvas marking DEFERRED** to the named GUI
   follow-up — it depends on the Pass 6.1 canvas tool-mode that never
   shipped; the engineer correctly did NOT build a parallel drag tool.

**RAG escalations (`C:\personal_rag\pdf\`):** (a) the **advance-preserving
text-removal** pattern (delete a `Tj` → substitute a `TJ` offset consuming
the exact `tx`, so surviving same-line text stays positioned — the
content-stream-surgery correctness lesson); (b) the **absence-proof-as-
acceptance-gate** pattern (redaction's four-shalls as "grep the whole
output for the redacted bytes → zero" — R46 inverted); (c) redaction's
**"diligence carriers"** list (the carriers a naive region-redact misses —
ObjStm survivors, prior revisions, `/Info`, XMP, XFA, overlapping annots,
attachments, OCG, StructTree `/ActualText`). Subject + master indexes
updated.

**MILESTONE:** read → write → edit → extract → annotations → forms →
redaction are ALL shipped. **In progress advances to decision 010**
(post-redaction priority — KenAgent consultation IN FLIGHT: vector/Inkscape
editing vs GUI-editing consolidation vs render-fidelity verification vs
encryption).

**Still open (unchanged, ordered oldest-first):** encryption-refusal
operator sign-off (oldest owed); LEGAL.md §2 Adobe-supplement copyright
contradiction; `/R 6` sourcing method; license decision; commit
authorization (**Passes 0–8.0 ALL uncommitted in git**); W15 (no
remote/CI); the full-page pixel-parity remainder (Pass 1.1); the
accumulated GUI-editing follow-up slices (canvas markup drawing /
form-fill / redaction-marking).

### Pass 7.1 — Form flatten + FDF/XFDF + choice fields + regenerate-all (COMPLETES the AcroForm subsystem CORE) — 2026-08-01

**Functionally COMPLETES the AcroForm subsystem CORE.** With Pass 7.0's
field model + text/checkbox fill and Pass 7.1's flatten + data interchange
+ choice fields + regenerate-all, the forms core is done; the remaining
forms items (GUI form-fill slice, field auto-detection, posture-B native
recompute) are FOLLOW-UP SLICES, not core (tracked in Backlog). Every
deliverable honors the round-trip and fuzzy-never-sneaky invariants;
decision 009 posture A (recognize + disclose, ZERO execution) is preserved
and extended with the JS-disclosure histogram.

- **New `pdfce-core` `fdf.rs` (~700 lines)** — FDF (§12.7.7) + XFDF
  import/export. The FDF reader REUSES `crate::parser::Parser` (FDF is a
  PDF-syntax file, so the existing tokenizer/object parser applies
  directly). XFDF uses a HAND-ROLLED ~200-line scoped XML reader
  (element/attribute/text events, the 5 predefined entities, numeric
  character references, comments, `<?xml?>`/DOCTYPE skip, MAX_XML_DEPTH-
  guarded). **ZERO new dependencies** (rule 13; the brief's stated
  preference — no `quick-xml`/`roxmltree` pulled in for a reader this
  small and this scoped).
- **`edit.rs` additions:** `set_choice_value`, `regenerate_appearances`,
  `flatten_fields`, `export_form_data`/`import_form_data`,
  `regen_field_appearance`, and the flatten helpers (`burn_target`,
  `page_of_widget`, `append_page_content`, `add_page_xobjects`,
  `effective_resources`, `remove_from_annots`, `remove_fields_from_form`,
  `clear_need_appearances_write`), the §12.5.5 `fit_matrix_for`,
  `match_option`, `choice_display_text`. New outcome types
  `RegenOutcome` / `ImportOutcome` / `FlattenOutcome` (+9 tests).
- **`forms.rs`:** `scan_javascript` + the `FormJavaScript` histogram
  (decision 009 posture A, recognition-only) (+2 tests).
- **`writer/content.rs`:** `ContentBuilder::invoke_xobject` (`/Name Do`).
- **CLI:** `regenerate-appearances` / `flatten` / `export-data` /
  `import-data` subcommands + choice routing + `|`-multi-select in
  `fill-field` + the JS histogram on `list-fields`.

**Key design win — FLATTEN is APPEND-not-rewrite (the ADOPTED design,
supersedes the brief's in-place-rewrite anticipation):** flatten appends a
NEW overlay content stream to the page `/Contents` array and invokes the
widget's existing `/AP` `/N` as a page XObject (`ContentBuilder::
invoke_xobject`), rather than rewriting the existing page content stream.
Consequence: **existing content streams stay byte-verbatim, so the R46
re-emit-everything gate has ZERO flattened-page exceptions** (GATE PASS
over `fixtures/synthetic` + `fixtures/external`; all divergences the known
value-preserving `-0`→`0` number re-spellings, 0 corruptions). This is
MORE minimal-diff than the in-place content-stream surgery Pass 7.1's
scope anticipated. Recorded as a general pattern: **overlay-append beats
content-stream-surgery when the goal is additive burn-in** (see
`ARCHITECTURE.md` §5.8 + a `personal_rag/pdf` lesson). **R48 verified:**
incremental flatten leaves the field dict recoverable in the prior
revision; `--full-rewrite` output has no `/FT` / `/Tx` yet renders the
burned value. Flatten uses the STRICT certification gate (refused on `/P
2` certified, by test — correct: flatten is a STRUCTURAL change, distinct
from the fill path's `/P >= 2` permit).

**Choice-field matrix (recorded):** single-select combo → `/V` stores the
EXPORT value, `/I = [idx]`, appearance shows the DISPLAY value; multi-
select list → `/V` array + `/I` array; single-value-given-a-multiselect-
required field → `ChoiceRequiresMultiSelect` refusal; unknown-value on a
non-editable field → `ChoiceValueNotInOptions`; editable combo
(`Combo|Edit`) accepts free text with no `/I`. **FDF/XFDF round-trip:**
fill → export FDF + XFDF → re-import → identical `/V` + regenerated
appearances; import SKIPS fields the target doc lacks (counted, never an
error).

**Gates:** **1,010 workspace tests** (core lib 620, was 601); `cargo fmt
--check` / `clippy -D warnings` clean; GUI-free core+render invariant
(zero egui/eframe/winit/wgpu); wasm32; `--duplicates`; `no-network` clean;
`ui-strings` N/A (no GUI changes this Pass); **R34** (Pass 3.0 identity)
green + **R46** re-emit-everything GATE PASS (Pass 7.1 additive — flatten
appends, never rewrites); fuzz **target 14 `fdf_parse`** 624,202 runs /
61 s, 0 crashes (malformed FDF/XFDF, huge arrays, entity edges); veraPDF
§6.1.12 N/A (MAX_XML_DEPTH is XFDF-only, outside PDF-conformance scope);
**ZERO new dependencies**, `THIRD_PARTY_LICENSES.md` unchanged.

**Gotcha found + fixed (RAG-escalated to `D:\dev\rag\rust\`):** adding the
CLI subcommands overflowed the DEBUG `pdfce-cli` main-thread stack on
Windows — clap's `debug_assert` recursion vs the MSVC ~1 MB main-thread
stack — surfacing as `TryFromIntError(NegOverflow)` in the CLI integration
tests. Fixed by running `main()` on a 16 MB worker thread. Filed as a
rust-tier lesson; the engineer had already noted it to agent memory
(`reference_clap_windows_stack.md`) — promoted to `D:\dev\rag\rust\` +
index.

**Deviations:** (1) flatten APPEND-not-rewrite (a POSITIVE deviation —
more minimal-diff than scoped; see above); (2) the JS histogram is
posture-A ONLY (`custom_scripts` counts all field-level JS actions, no
whitelist recompute — posture B stays a Pass 7.x follow-up per decision
009), surfaced on `list-fields` + a loud stderr flag when network/launch
`/AA` actions are present.

**Residuals (named):** list-box multi-select appearance is a simplified
display-text newline-join, NOT the §12.7.4.4 highlight-rectangle rendering
(named simplification); corpus flatten-burn coverage is thin (the sampled
external forms were certified / pushbutton / no-`/AP` per the 6.0 census,
so synthetic fixtures + unit tests carry the burn path); import applies as
per-field commands (each independently undoable), NOT one atomic
`ImportFormData` command. **STILL OPEN from Pass 7 — the forms FOLLOW-UP
slices (NOT core):** GUI form-fill slice
(`docs/ui_specs/pass-7-form-fill.md`), field auto-detection ("Prepare
Form", the fuzzy-never-sneaky HINT surface), posture-B native recompute
(decision 009, opt-in / demand-driven), X10/encryption, full-page
pixel-parity.

**RAG escalations:** `C:\personal_rag\pdf\` — the overlay-append-beats-
content-surgery flatten pattern (minimal-diff burn-in);
`D:\dev\rag\rust\` — the clap-Windows-debug-stack lesson. Subject +
master + subdir indexes updated.

**Demo:** CLI `fill` (text + checkbox) → `export-data` FDF + XFDF →
re-import round-trip → `flatten` (`fields_flattened=1 widgets_burned=1`) →
`list-fields` shows the field gone → `render-page` renders the burned
appearance (`forms=1`) → `--full-rewrite` removes it. GUI launched
(PID 42444) on the flattened file.

**AcroForm CORE COMPLETE (milestone):** 7.0 (field model + fill) + 7.1
(flatten + data + choice + regenerate) = the AcroForm subsystem core is
done. Remaining forms work is the named follow-up slices tracked in
Backlog, not core. **In progress advances to Pass 8 (Redaction)** — the
standing R35 obligation and the one truly destructive op — blocked on two
prerequisites both DISPATCHED in parallel: the Redaction acrobat-parity
bucket (`pdfce-acrobat-librarian`) + a redaction spec dispatch for
container-decomposition + `/Redact`-apply semantics
(`pdfce-spec-librarian`).

**Still open (unchanged, ordered oldest-first):** encryption-refusal
operator sign-off (oldest owed); LEGAL.md §2 Adobe-supplement copyright
contradiction; `/R 6` sourcing method; license decision; commit
authorization (**Passes 0–7.1 ALL uncommitted in git**); W15 (no
remote/CI); the full-page pixel-parity remainder (Pass 1.1).

### Pass 7.0 — AcroForm field model + text/checkbox fill (forms FOUNDATIONAL SLICE) — 2026-08-01

**The FOUNDATIONAL SLICE of the forms subsystem — NOT all of Forms.**
Pass 7 was split on ship: the engineer delivered the field-model read
path plus the dominant fill path (text + checkbox/radio) and honestly
named the rest as residuals. Those residuals are filed as **Pass 7.1
(Next up) — "completes the forms subsystem"**; do not read this entry as
"Forms shipped." What DID ship: the `/AcroForm` field-model parser, the
field↔widget merge, per-type value decode, the `/P`-aware certification
gate for fill, and text/checkbox fill through the SAME §12.7.3.3
appearance generator Pass 6.2 built (R49 — one appearance pipeline for
widgets and annotations alike). Decision 009's posture A (recognize +
disclose + byte-exact JS round-trip, ZERO execution) is honored: fill
touches only `/V` // `/AP` // `/AS`, never the `/AcroForm` dict, so every
JS carrier (`/CO`, `/AA`, `/Names /JavaScript`) re-emits verbatim.

- **New `pdfce-core` `forms.rs` (~1,050 lines, 13 model tests)** —
  `parse_acroform(graph)` walks `/AcroForm` → `/Fields` depth-first,
  resolving field-tree inheritance of `/FT` // `/V` // `/DV` // `/Ff` //
  `/DA` // `/Q` down `/Kids` via `/Parent`, building the dotted
  **fully-qualified field name** (§12.7.3.2). Implements the
  **field-vs-widget MERGE (R49):** *Shape A* — a terminal field with a
  single widget merges field + widget into ONE dictionary (the ~88%
  common case); *Shape B* — a field with a `/Kids` array of widget
  annotations keeps field and widgets separate. Generic over
  `ObjectGraph` so it runs against both a loaded `Document` and an
  `EditSession` overlay. **`FieldFlags` verbatim bits pinned by test**
  (§12.7.4.2 Table 226 / §12.7.4.2.1): Radio `32768`, Pushbutton
  `65536`, NoToggleToOff `16384`, RadiosInUnison `33554432`; Multiline
  `4096`, Comb `16777216`; Combo `131072`, MultiSelect `2097152`.
  Per-type `/V` decode; `/Opt` export/display pairing; `/I` // `/TI`;
  **XFA detect-only** (`XfaPresence` — recognized, never parsed);
  `/SigFlags`; `/CO` count. **Cycle-guarded** (visited set +
  `MAX_FIELD_TREE_DEPTH = 64`) and **bounded** (`MAX_FORM_FIELDS =
  500_000`) — both pure memory backstops (see veraPDF note below).
- **Fill (`edit.rs`, 6 fill tests):**
  - `fill_text_field` — sets `/V` and **regenerates `/AP` for every
    widget** of the field via the shared §12.7.3.3 generator (R49 reuses
    Pass 6.2's `vartext.rs`, wrapped by
    `annot_author::build_field_text_appearance`; the `/DA` font is
    resolved from `/DR` via `basefont_to_std14`).
  - `set_button_state` — checkbox / radio `/V` + `/AS` state selection
    (no appearance regen — the on/off appearances already exist in the
    widget's `/AP` sub-dictionary), honoring RadiosInUnison and the
    `/Off` convention.
  - **`/P`-aware certification gate** (`check_certification_for_fill`,
    built per the orchestrator's mid-Pass correction): permits fill at
    `/DocMDP` `/P >= 2` (including **absent = 2** by §12.8.1 default),
    refuses **by name** at `/P 1`, and refuses on **any** `/FieldMDP`.
    The structural gate stays STRICT. Proven by
    `certification_p2_permits_fill_p1_refuses`. This is the per-`/P`
    refinement the Pass 6.1/6.2 X11 residual asked for, applied to the
    fill path.
  - One command per fill (undo inherited from the Pass 3.1 command log);
    encryption + `/Size` guards inherited.
- **Decision 009 honored structurally:** fill mutates only `/V` // `/AP`
  // `/AS`, never the AcroForm dict — so `/CO` // `/AA` // `/Names
  /JavaScript` re-emit **byte-verbatim** under incremental save.
  `has_additional_actions` + `calc_order_count` are surfaced
  **recognition-only** (the full JS-disclosure histogram is deferred to
  Pass 7.1 per decision 009's posture-A scope).
- **CLI:** `list-fields` (locale-invariant stable-line field inventory:
  FQN, type, flags, value, widget count) + `fill-field` (`--set
  Name=value`, text and checkbox/radio; auto-size disclosed).

**Verified (R44 form-fill round-trip with GLYPH PIXELS — HEADLINE):**
the demo authors a text fill + a checkbox on-state, saves incremental
(`undo_identical=1` — minimal-diff holds on the first command), reloads a
fresh `Document`, and `render-page` **paints 11 real glyphs for the
filled text**, reporting `annots_painted=2 / forms=2`. This is the
form-fill analogue of Pass 6.1/6.2's baked-`/AP` proof: an authored field
value is real glyph pixels through the SAME Pass-6.0 read path, never a
private "what I just filled" render.

**Gates:** `cargo fmt --check` / `clippy -D warnings` clean (core + cli);
GUI-free `cargo tree` invariant verified core + render (zero egui / eframe
/ winit / wgpu); **ZERO new dependencies.** **601 `pdfce-core` lib tests
green (was 582; +13 model +6 fill)** + integration green. **Fuzz target
13 `form_model`** — 1,306,476 runs / 61 s, 0 crashes (cyclic `/Parent` //
`/Kids`, huge `/Kids` arrays, merge-shape edge cases, malformed field
values). Real-corpus `list-fields` clean on all `/AcroForm` files
(pushbutton `flags = 0x10000`, no panics). **veraPDF §6.1.12:** the two
new guards `MAX_FORM_FIELDS` / `MAX_FIELD_TREE_DEPTH = 64` are pure memory
backstops (corpus max ≈ 63 fields/file — >7,900× headroom on the field
count, 1× on depth but no corpus file nests fields at all).

**R34 / R46 preserved BY ADDITIVITY** (a new module + additive
methods/variants + one new `pub fn`; the re-emission path and
`add_markup` / `add_text_annotation` byte-unchanged). R34 (Pass 3.0
roundtrip) is accepted as additivity-preserved — fill authors NEW `/AP`
streams, never re-serializes untouched objects — and is not separately
re-run this Pass.

> **Footer (2026-08-01) — full-corpus R46 re-run post-7.0 = GATE PASS.**
> Re-run by the orchestrator over `fixtures/external` (3,020 files):
> every content stream semantically preserved, identical to the post-6.2
> result (same value-preserving number-respelling divergences, zero
> corruptions). Additivity confirmed BY MEASUREMENT — fill authors new
> `/AP` streams via the proven §12.7.3.3 generator; the re-emission path
> (`reemit_canonical` / `emit_token_canonical` / `number_divergence_reason`
> / `emit_number`) is byte-unchanged. This DISCHARGES the "full-corpus
> R46 re-run owed" residual from the engineer's completion report.

**Demo:** `fixtures/synthetic/forms/demo-form.pdf` (+ `PROVENANCE.md` +
`tools/gen-form-fixtures.py`): `list-fields` → `fill-field --set
FullName="Ada Lovelace" --set Subscribe=on` (auto-size disclosed) →
incremental save `undo_identical=1` (minimal-diff) → reload shows "Ada
Lovelace" + the "Yes" checkbox state with regenerated `/AP` →
`render-page` paints 11 glyphs for the filled text, `annots_painted=2
forms=2` (the R44 form-fill proof — authored text is real glyph pixels
through the Pass 6.0 read path).

**Residuals → Pass 7.1 (Next up, "completes the forms subsystem"):**
regenerate-all + clear `/NeedAppearances` save-side op (R51); **Flatten**
(destructive R48 — page-content-stream APPEND + byte-grep test; the FIRST
controlled modification of EXISTING page content, distinct from 6.1's
new-stream-only authoring); FDF / XFDF import/export (must-have, data
round-trip — XFDF needs a minimal XML reader, classify per rule 13);
choice-field multi-select (array `/V`, `/I` maintenance, `/Opt`
display↔export); the JS-disclosure histogram (per-hook counters +
network/process `/AA` flags, decision 009 posture A); GUI form-fill
(`docs/ui_specs/pass-7-form-fill.md` P0 — text + checkbox fill, both
disclosures, direct-click no-mode, real egui widgets, flatten toolbar
button, export/import in the Tools dock). Field **AUTO-DETECTION**
("Prepare Form") stays a SEPARATE later Pass/backlog (fuzzy-never-sneaky
HINT). Posture-B native recompute (`AFSimple_Calculate` SUM/AVG/PRD/MIN/
MAX + `AF*_Format`, opt-in / off-by-default) is a distinct demand-driven
Pass 7.x follow-up per decision 009 — NOT part of 7.1.

**RAG escalation (`C:\personal_rag\pdf\`):** the field-vs-widget merge
Shape-A / Shape-B distinction as a parsing lesson — single-widget
terminal fields merge field + widget into one dict (~88% of real fields);
a reader that always expects a `/Kids` widget array breaks on the common
case. Subject + master indexes updated.

**Decision 009 (embedded form/document JavaScript) filed this session** —
outcome NEVER execute embedded PDF JavaScript (ISO-conformant, §12.6.4.16
is a "hollow shall"). Adds standing rules **R53–R57** (see Standing rules
below). Posture A is Pass 7's entire JS scope; posture B is deferred Pass
7.x; posture C (a JS engine) is REJECTED + prohibited. Full record:
`docs/decisions/009-forms-javascript-posture.md`.

**Still open (unchanged, ordered oldest-first):** encryption-refusal
operator sign-off (oldest owed); LEGAL.md §2 Adobe-supplement copyright
contradiction; `/R 6` sourcing method; license decision; commit
authorization (Passes 0–7.0 ALL uncommitted in git); W15 (no remote/CI);
the full-page pixel-parity remainder (Pass 1.1).

### Pass 6.2 — Text-bearing annotations + §12.7.3.3 variable-text appearance generation — 2026-08-01

**COMPLETES the decision-008 6.x annotation arc** (6.0 display →
6.1 geometry → 6.2 text; all three SHIPPED). Adds the text-bearing
annotation subtypes Pass 6.1 deliberately deferred — **FreeText, Text
(sticky note), Stamp** — and the shared §12.7.3.3 variable-text
appearance-generation pipeline. That pipeline (`vartext.rs`) is the
appearance half **Pass 7 (Forms) REUSES** for widget-field appearances
(R49 — one appearance pipeline for widgets and annotations alike). Next
up is **Pass 7 (Forms/AcroForm)**. R43/R44/R47 honored throughout:
every authored appearance is a real baked `/AP` `/N`, displayed by the
SAME Pass-6.0 read path — never a private "what I just drew" render.

- **New `pdfce-core` `vartext.rs`** — the §12.7.3.3 variable-text
  pipeline: `/DA` default-appearance parsing (font, size, colour), the
  auto-size `0` rule, and the field-value → appearance-stream layout
  procedure (line breaking, `/Q` quadding, baseline placement). **This
  is the shared FreeText + widget generator Pass 7 reuses** — built once
  here, wired to the form-field model there.
- **Modified `pdfce-core` `writer/content.rs`** — `ContentBuilder`
  extended with the text/marked-content/clip/matrix operator set
  (BT/ET/Tf/Td/TD/TL/Tj/Tc/Tw/Tz/q/Q/BMC/EMC/W/cm + `emit_literal_string`).
  **PURELY ADDITIVE** — the R46 re-emission path
  (`reemit_canonical` / `emit_token_canonical` / `number_divergence_reason`
  / `emit_number`) is NOT touched, so the Pass 6.1 R46 identity result
  carries forward unperturbed.
- **Modified `annot_author.rs`** — `TextAnnotSpec` (FreeText / Sticky /
  Stamp), `StickyIcon`, `StampName`, `AuthoredTextAnnot`,
  `build_text_annotation`, R44 icon/stamp look. Kept a **SEPARATE**
  enum from `MarkupSpec` (deviation 1 below) so the R46/R34-proven
  geometric `add_markup` path and its exhaustive match arms stay
  byte-unchanged.
- **Modified `edit.rs`** — `EditSession::add_text_annotation` + guards
  (X10/X11 inherited-conservative from 6.1) + R45 staging + X7 `/Annots`
  multi-append + one-command undo (including the `/Popup` companion for
  sticky notes); `AnnotKind::{FreeText, Text, Stamp}`;
  `EditError::VariableText`.
- **CLI** `annotate --type freetext|text|stamp` (`--text` / `--font` /
  `--size` / `--quad` / `--multiline` / `--icon` / `--stamp-name`;
  `--fill` doubles as the optional FreeText border colour). **GUI**
  "Text ▾" menu + a modeless text-entry popup.

**Verified (R44 text round-trip with GLYPH PIXELS asserted — HEADLINE):**
`authored_freetext_paints_glyph_pixels_after_reload_r44` authors a
FreeText → save incremental → reload a fresh `Document` → render via the
Pass 6.0 read path → **>100 dark glyph pixels, `annotations_painted=1`**.
The demo run reports `annots_painted=3 / forms=3`, substituted 35→55 =
**20 authored glyphs** via the bundled Foxit substitute fonts. This is
the text analogue of Pass 6.1's baked-`/AP` proof: authored text is real
glyph pixels through the same read path, never a private render.

- **`/Q` quadding measured vs AFM widths** (`quadding_places_lines_by_afm_width`):
  "AV" in Helvetica = 13.34 pt; the `Tm` x-origin matches left / centre /
  right exactly.
- **Bare standard-14 font dict renders with NO embedded program**
  (`bare_standard14_font_dict_renders_with_no_embedded_program`): no
  `/FontDescriptor`, no `/Widths`. Modality choice — authored the bare
  form, reader-shall-honour §9.6.2.1, PDF-1.5 should-embed deprecation
  noted; `+/Encoding /WinAnsiEncoding` added for deterministic Latin
  byte→glyph.
- **Auto-size VT1 heuristic (no spec formula):**
  `auto_size(rect_h) = ((rect_h − 2·PAD) / 1.15).clamp(4.0, 12.0)`,
  `PAD = 2`, line-factor 1.15 — reviewable, every appearance reports
  `applied_autosize`, never presented as spec-mandated (S-class spec
  silence → counted, reviewable heuristic).

**Deviations (recorded as decisions):**
1. Text specs live in a SEPARATE `TextAnnotSpec` enum, NOT folded into
   `MarkupSpec` — keeps the R46/R34-proven geometric `add_markup` path +
   its exhaustive match arms byte-unchanged (text needs different wiring:
   `/DA`, popup, `/NoZoom`/`/NoRotate`).
2. FreeText font dict is 4-key (adds `/Encoding /WinAnsiEncoding`), not
   literally 3-key — deterministic Latin byte→glyph for the glyph-pixel
   proof + `/Q` measurement; still program-free (the gate's real
   meaning).
3. CLI `--fill` doubles as the optional FreeText border colour.
4. `/M` // `/CreationDate` still omitted (inherited 6.1 residual — clock
   non-determinism breaks byte-compare).

**Residuals (named):** Base-14 **LATIN ONLY** (no complex-script / RTL
shaping — R17; non-WinAnsi chars → "?" counted as `unencodable_chars`);
`/RC` rich text recognition-only (VT3 non-goal); no comb fields (Pass 7;
comb = field-flag bit 25 = 16777216); X11 certification gating still
conservative (over-refuses `/P 3` — scoped fix = check
`certification_permission == Some(3)` for annotation-adding, §12.8
already sourced); X10 encryption refusal still the load-time R37 seam.
**GUI:** full in-canvas text editing + the sticky-note marker's exact
artwork join the already-named Pass-6.1-followup GUI slice
(ui-specialist refinement) — NOT built here.

**Tests: 971 workspace tests green (was 939).**

**Gates:** `cargo fmt --check` / `clippy -D warnings` clean; GUI-free
core+render invariant verified (zero egui/eframe/winit/wgpu); wasm32;
`--duplicates`; `ui-strings`; `no-network` all clean; fuzz `annot_author`
extended (`/DA` parsing + text-appearance generation: malformed `/DA`,
unresolvable font, symbolic font, huge text, size 0) — **13,871 runs /
61 s, 0 crashes**; **no new §6.1.12 guards**. **ZERO new dependencies**
(Base-14 only, **NO `harfrust`** — R17 upheld; the text-authoring path
reserved by decision 004 for a future `harfrust` built WITHOUT it);
`THIRD_PARTY_LICENSES.md` unchanged.

**Full-corpus R46 — GATE PASS (measured, superseding the fixture-only
run).** At filing time the engineer's completion report had run the R46
content-identity gate over synthetic fixtures only (46/46 byte-identical)
plus proof-by-inspection that the content-stream re-emission path is
additive-only / untouched, having reported the conformance corpus "not
present on this machine." **That was a path-resolution miss — the corpus
IS present (3,020 files under `fixtures/external` — veraPDF-corpus +
pdf20examples).**

> **Footer (2026-08-01) — full-corpus R46 re-run by the orchestrator =
> GATE PASS.** Every content stream semantically preserved, zero
> corruptions. All divergent streams are value-preserving number
> re-spellings — the same class Pass 6.1 enumerated: `-0`→`0` (the
> majority), `.050003`→`0.050003` (leading-zero insertion, the Isartor
> file), `-.001`→`-0.001`, and the one 300-digit pathological real
> (`50.111…`→`50.111111111111114`, value-preserved within f64). This
> CONFIRMS Pass 6.2's additive-only claim BY MEASUREMENT: the re-emission
> path (`reemit_canonical` / `emit_token_canonical` /
> `number_divergence_reason` / `emit_number`) is byte-unchanged and the
> gate did not regress. The earlier fixture-only run (46/46) is
> **superseded** by this full-corpus measurement.
> **For the record:** the corpus IS present and runnable at
> `fixtures/external` — future Passes must NOT accept a "corpus absent"
> caveat without checking `fixtures/external` first.

**RAG escalations (`C:\personal_rag\pdf\`):** (a) the
bare-Base-14-font-dict-renders-with-no-embedded-program finding +
the WinAnsiEncoding-for-determinism note (authoring lesson); (b) the
auto-size VT1-is-implementation-defined heuristic pattern (S-class spec
silence → reviewable, counted heuristic). Subject + master indexes
updated.

**ARC-COMPLETE:** the decision-008 6.x arc is now COMPLETE (6.0 / 6.1 /
6.2 all SHIPPED). In progress advances to **Pass 7 (Forms/AcroForm)**,
blocked on two prerequisites both DISPATCHED in parallel: the
§12.7.1–12.7.4 form-field spec (`pdfce-spec-librarian`) and the "Forms
(AcroForm)" acrobat parity bucket (`pdfce-acrobat-librarian`). The
embedded-JavaScript posture is a Pass-7 open sub-decision (decision 008:
recommend **never-execute** — recognize + disclose; a security decision
needing its own record when Pass 7 is scoped).

**Still open (unchanged, ordered oldest-first):** encryption-refusal
operator sign-off (oldest owed); LEGAL.md §2 Adobe-supplement copyright
contradiction; `/R 6` sourcing method; license decision; commit
authorization (Passes 0–6.2 ALL uncommitted in git); W15 (no remote/CI);
the full-page pixel-parity remainder (Pass 1.1).

### Pass 6.1 — Authored streams + content-stream serializer + geometric markup authoring — 2026-08-01

The project's **FIRST content-stream authoring Pass** — decision 008
findings **F3** (no content-stream serializer; `Stream` couldn't hold
authored bytes) and **F4/R45** (authored bytes need a session staging
buffer, not a mutated `Stream` type) both discharged here. Authors the
geometric markup annotations whose appearance is **pure geometry**:
Ink, Square, Circle, Line, Polygon, PolyLine, plus the quad-point
text-markup family (Highlight/Underline/StrikeOut/Squiggly). **No
text-bearing annotations** — FreeText/Text/Stamp + §12.7.3.3 variable
text are deferred to Pass 6.2 (one appearance pipeline). R43/R44/R47
honored throughout: every authored appearance is a real baked `/AP`
`/N` written into the document, then displayed by the SAME read-side
path Pass 6.0 uses — never a private "what I just drew" render.

- **New `pdfce-core` `writer/content.rs`** — `ContentBuilder`: the
  §8.2 path/paint/graphics-state/colour operator set plus the §8.10
  WF6 form-XObject ordering (from the `pdfce-spec-librarian` §8.10.2
  WRITE-direction audit that unblocked the Pass). Carries
  `reemit_canonical` + `number_divergence_reason` — the machinery the
  R46 identity gate uses to re-serialize existing streams and classify
  any divergence.
- **New `pdfce-core` `annot_author.rs`** — `MarkupSpec` / `Color` /
  `Quad` / `LineEnding` / `TextMarkupKind`; `build_appearance` →
  `AuthoredAppearance` = the annotation dict + its `/AP` `/N` form
  XObject + the content bytes. The one place a markup annotation's
  geometry becomes on-disk bytes.
- **Modified core surface:** `writer/serialize.rs`
  (`write_real`/`write_name`/`write_string` now `pub(crate)` — reused
  by the content serializer); `writer/mod.rs` (`DirtySet` gains the R45
  staging buffer + `combined_source()`); `writer/save.rs` (serializes
  replacement/created objects against base++staging); `edit.rs`
  (`EditSession::add_markup` + staging + guards + `authored_source()` +
  COW `/Annots` patching + `AnnotKind` + `CommandKind::AddAnnotation` +
  `EditError::{DocumentEncrypted, EmptyGeometry, AnnotsNotAnArray}`);
  `pageops/assemble.rs` (`DocumentView` doc comment **amended to
  discharge — not delete —** the R45 written assertion).
- **CLI** `annotate` subcommand (unified `--type`, per-subtype geometry
  flags, stable append-only stdout contract). **GUI** toolbar
  "Markup ▾" menu (minimal — see the named follow-up slice below).
- **`tools/content-identity/`** — the R46 corpus identity gate
  (out-of-tree, like the other corpus harnesses). **Fuzz target 12**
  `annot_author.rs`.

**R46 content-stream identity gate (HEADLINE — the serializer is
proven before it is trusted to author):** over the full corpus —
**12,936 content streams across 2,898 loadable files; 12,854
byte-identical (99.37%); 82 non-identical (0.63%); 0 CORRUPTED → GATE
PASS.** The 82 are **all spec-legal, VALUE-PRESERVED number
re-spellings**, enumerated by file+reason (R20 discipline): `.05`→`0.05`
leading-zero insertion (20×), `-0`→`0` (18×), one 300-digit
pathological real, `1.`→`1.0`. **Framing (recorded so it is never
misread as a fidelity loss):** this is a **serializer-correctness**
test — the gate deliberately re-emits every stream. pdfce **never
re-serializes untouched page content in normal save** (span re-emission
keeps it byte-verbatim, §5); authoring writes only NEW streams — so
these divergences **never occur in production save**. X6 (silent
normalization of a stream pdfce claims to preserve) is caught
mechanically.

**Verified invariants:**
- **R44 round-trip** author→save→reload→paint: CLI authored a
  square+highlight+ink incrementally; `undo_identical=1` on the first
  (minimal-diff holds); `render-page` reports annots=3 / painted=3 /
  forms=3 through Pass 6.0's read path; a render test confirms the red
  square paints red after reload — every appearance a real baked `/AP`,
  never a private render.
- **X5** extract-from-authoring-session resolves via
  `authored_source()` (base++staging); the authored appearance survives
  **BYTE-EXACT** (the `DocumentView` assertion discharged by
  amendment).
- **X7** `/Annots` create / append-direct / COW-shared-indirect-array
  (sibling page provably untouched, tested) + a compressed-page fixture
  promotes out (R38, reload verified).

**QuadPoints policy DECIDED (closes the Pass 6.0 carried open item):**
pdfce authors quads in **Z / reading order (UL, UR, LL, LR)** — the
dominant Acrobat/PDFBox/pdf.js convention, chosen for maximum
third-party interop and documented in `annot_author.rs`. pdfce's own
render is convention-independent (it paints the baked `/AP`), so the
choice is an interop decision, not a correctness one. The
`personal_rag/pdf` QuadPoints lesson is updated OPEN → RESOLVED-for-
authoring (Z order) with the render-independence note.

**Deviations / residuals (recorded as decisions):**
1. **X11 certification gating is deliberately CONSERVATIVE** — reuses
   `check_certification()`, which refuses on ANY enforced `/DocMDP`,
   so it over-refuses annotation addition that `/DocMDP` `/P 3`
   actually permits. Fail-clean-safe. Per-`/P` refinement is a
   spec-verified follow-up (the fix is scoped: check
   `certification_permission == Some(3)` for annotation-adding — §12.8
   already sourced, no new spec research).
2. **X10 encryption guard is a forward-compat R37 seam** — encrypted
   files are refused at LOAD today, so `DocumentEncrypted` in
   `add_markup` is unreachable until Pass 5; the test pins the
   load-time refusal.
3. **No `/M` mod-date / `/CreationDate`** on authored annotations —
   avoids clock non-determinism breaking the byte-compare tests. Named
   residual; revisit when a deterministic-clock-injection or metadata
   policy exists.
4. **Line-ending set = None/OpenArrow/ClosedArrow** (Acrobat default
   Open honored) — the full §12.5.6.7 Table 176 set is not authored.
5. **Square/Circle/Underline/StrikeOut/Squiggly default colours are
   pdfce's own** (the Acrobat RAG marks them a GAP); **Highlight
   yellow + Multiply** is the sourced default, locked.

**Named GUI follow-up slice (decision-log-worthy, filed to Backlog
below):** the full canvas markup drawing state machine (drag/marquee/
multi-click/ink-freehand + live preview + the screen↔page transform,
planned for a pass-4-only slice + the ten-tool set + keyboard map) and
P1 glyph-accurate text-selection markup are a FOLLOW-UP SLICE.
`docs/ui_specs/pass-6.1-markup-tools.md` is the design; the shipped GUI
is a **minimal menu affordance** that authors at a default page-centred
rect through the same `EditSession::add_markup` path.

**Tests: 939 workspace tests green (was 901).**

**Gates:** `cargo fmt --check` / `clippy -D warnings` clean;
**R34 re-runs GREEN** (Pass 3.0 identity identical=1 raster_identical=1
— authoring never perturbs untouched objects); GUI-free invariant
verified core+render host + `x86_64-pc-windows-msvc`; wasm32;
`--duplicates`; `ui-strings`; `no-network` all clean; fuzz target 12
`annot_author.rs` 696,098 runs / 61 s, 0 crashes. **ZERO new
dependencies** — the content serializer and markup authoring are
hand-rolled (no `harfrust`, consistent with R17); `THIRD_PARTY_LICENSES.md`
unchanged.

**RAG escalations (`C:\personal_rag\pdf\`):**
(a) QuadPoints lesson updated OPEN → RESOLVED-for-authoring (Z /
reading order is the de-facto convention; render is convention-
independent);
(b) new serializer-authoring lesson — the R46-gate number-respelling
catalogue: which real-world number spellings a canonical serializer
diverges from (leading-zero-absent `.05`, `-0`, trailing-dot `1.`,
pathological-length reals — all value-preserving), and why a
re-emit-everything gate surfaces them while production span-re-emission
never does.

**Still open (unchanged, ordered oldest-first):** encryption-refusal
operator sign-off (oldest owed); LEGAL.md §2 Adobe-supplement copyright
contradiction; `/R 6` sourcing method; license decision; commit
authorization (Passes 0–6.1 ALL uncommitted in git); W15 (no
remote/CI); the full-page pixel-parity remainder (Pass 1.1).

### Pass 6.0 — Annotation & widget appearance rendering (read-side) — 2026-08-01

Scoped by decision 008
(`docs/decisions/008-next-subsystem-after-extract.md`) as the
read-side display half of Annotations & markup (candidate
A ≫ B > C > E > D > F). The direct remedy for decision 008's finding
**F1 — pdfce rendered NO annotations and did not even COUNT them, the
one undisclosed shortfall unique in the project.** ZERO authoring: R43
honored throughout — pdfce paints an existing `/AP` `/N` or
counts-by-name, and never synthesizes an appearance.

- **`pdfce-core` `annot.rs`** — §12.5 annotation walk / model /
  selection: the annotation dictionary, `AnnotFlags` (§12.5.3 —
  `/Hidden`, `/NoView`, `/Print`, `/NoZoom`, `/NoRotate`, … each
  HONORED and COUNTED per R50), the `Appearance` taxonomy (`/AP` →
  `/N`, appearance sub-dictionaries keyed by `/AS`), and
  `need_appearances` (a document-scoped `/NeedAppearances` query —
  disclosed, never silently auto-generated, per R51).
- **`pdfce-render` `annot.rs`** — §12.5.5 appearance placement, routed
  through the EXISTING `interpret::run_form_at` → `do_form` path
  (F2 confirmed: an `/AP` `/N` IS a form XObject). Everything the
  Pass 1.1 form path already earned is inherited unchanged — X8
  resource scoping, the cycle guard, `MAX_XOBJECT_DEPTH`, the font
  cache — pinned by the test
  `appearance_uses_its_own_resources_not_the_page_font`.
- **Diagnostics +8** appended keys; **CLI** `render-page
  --no-annotations` + a new `list-annotations` subcommand; **GUI**
  toolbar visibility toggle + a status-bar disclosure line.
- **Tooling:** `tools/corpus-report` annotation census;
  `tools/gen-annot-fixtures.py` + `fixtures/synthetic/annot/PROVENANCE.md`
  (16 fixtures); `tools/annot-pdfium-diff.py` (placement differential,
  pypdfium2 — decision 006 §3.2 precedent).
- **Fuzz target 11** `annot_walk.rs` — 1.1M runs / 46 s, 0 crashes
  (cyclic `/AP`, degenerate/inverted `/Rect`, missing `/AS` state,
  `/N` neither stream nor dict).
- **New public API:** `RenderOptions.annotations` +
  `RenderOptions::with_annotations`.

**Census baseline — PINNED (pdfce-native; replaces decision 008's
pypdf conformance figures per W16):** across all **2,914 corpus files,
ZERO panics** — **338 files with annotations / 429 annotations
total / 224 with a USABLE `/AP` `/N` / 127 `/AcroForm` / 34 `/Popup` /
87 `/Widget`.** The per-file 338 and 127 match pypdf exactly (tooling
agreement); W16's per-annotation re-measurement obligation is
DISCHARGED for the conformance corpus. **The 224-vs-228 (pypdf) `/AP`
gap is a DEFINITIONAL finding, not an error:** pdfce counts a *usable*
`/AP` `/N` (a resolvable stream / selectable `/AS` state), pypdf counts
raw `/AP`-key presence; the 4-file difference = ~2 `/AS`-unresolved
state subdicts + ~2 absent/dangling/non-stream `/N`. pdfce's predicate
is deliberately stronger. Filed as a `personal_rag/pdf` finding.

**Placement correctness (X2):** `tools/annot-pdfium-diff.py` — **7/7
pure-geometry placement fixtures agree with pdfium within 4 px, 0
mismatches** (identity, non-origin BBox, BBox-larger, BBox-smaller,
Matrix-scale, Matrix-rotate, inverted `/Rect`); 6 blank-expected cases
(hidden / noview / popup / no-AP / state-missing / degenerate)
correctly blank. **This is NOT a claim that the Pass 1.1 pixel-parity
remainder is CLOSED** — it is an ink-bbox differential on the
annotation subset only; full-page pixel parity over real corpus pages
remains OWED (unchanged open item).

**Deviations (recorded as decisions — `ARCHITECTURE.md` §12
continuation-23):**
1. **`/NoZoom` / `/NoRotate` post-annotation-matrix transform
   DEFERRED** — counted + named (`annotation_notes`); rare,
   near-exclusively on icon subtypes that lack `/AP` anyway. A wrong
   post-transform is worse than a disclosed omission.
2. **`/OC` optional-content visibility test not implemented** —
   consistent with the renderer implementing NO optional content
   anywhere (BDC/EMC deferred; §8.11 is a RAG GAP). An OC-off
   annotation currently paints — named.
3. `need_appearances_documents` is a document-scoped query, not folded
   into per-page render `Diagnostics` (inherently document-level).
4. GUI diagnostics placed as a separate always-evaluated status line
   below the content-diagnostics header, NOT folded into the content
   unsupported-tally (chosen to avoid destabilizing the tested content
   clean-return path; still honest R50/R27/R51; flagged for future
   ui-specialist refinement).

**Durable GUI placement taxonomy** (ui-specialist deliverable —
resolves the X14 drift; recorded at `ARCHITECTURE.md` §12
continuation-23; supersedes/extends the continuation-20 three-way
taxonomy): view-state → toolbar view group; edit → toolbar/window;
selection-scoped → rail; advanced → Tools dock; disclosure → status
bar. This is the settled convention for all future GUI placement.

**Tests: 901 workspace tests green (was 875).**

**Gates:** `cargo fmt --check` / `clippy -D warnings` clean; GUI-free
invariant verified host + `x86_64-pc-windows-msvc`; wasm32;
`--duplicates`; `ui-strings`; `no-network` all clean. **R34 holds
STRUCTURALLY** — no pinned reference raster exists; the round-trip
oracle is a runtime self-comparison, so painting annotations perturbs
nothing the Pass 3.x/4 gates measure. **veraPDF §6.1.12:** new
`MAX_ANNOTS_PER_PAGE = 1_000_000` (a pure memory backstop — Annex C
imposes no limit and §6.1.12 forbids imposing one; the busiest corpus
page carries ≪100, >10,000× headroom); `/AP` recursion reuses
`MAX_XOBJECT_DEPTH = 64` unchanged (2× headroom vs veraPDF's 32-deep
conformant chain).

**Demand signals for 6.1/6.2/7 (recorded):** the
`annotations_without_ap` by-subtype histogram is corpus-dominated by
no-`/AP` `/Link`, `/Widget`, `/Circle`; `/NoZoom`-`/NoRotate` and
`/OC` are the two named display deferrals.

**RAG escalations (`C:\personal_rag\pdf\`):**
(a) pdfium requires `FPDF_FFLDraw` to render `/Widget` appearances — a
differential-harness gotcha (the two apparent diffs in the pdfium
comparison were REFERENCE divergences, not pdfce errors: pdfium also
SYNTHESIZES the no-`/AP` `/Circle` `/IC` fill that R43 makes pdfce
refuse);
(b) QuadPoints CCW-vs-Z-order unresolved (carried from §12.5.6 — the
spec says CCW but real producers / Acrobat emit Z / reading order;
only bites 6.1 generation).

**Still open (unchanged):** the full-page pixel-parity remainder;
encryption-refusal operator sign-off (oldest owed); LEGAL.md §2
Adobe-supplement copyright contradiction; `/R 6` sourcing method;
license decision; commit authorization; W15 (no remote/CI).

### Pass 4 — Text extraction / structured content — 2026-08-01

Sourced text extraction: the §9.10.2 character-mapping ladder
implemented VERBATIM, every derived judgment isolated and labelled,
every gap counted by name. **5,469 new `pdfce-core` lines**, no new
dependencies, acceptance grounded in the `pdfce-spec-librarian` §9.10
corpus (dispatched at promotion, returned before the engineer ran).

- **`textstring.rs`** — §7.9.2 text-string decoding + Annex D.3
  PDFDocEncoding, built from the annex's FOUR STRUCTURAL RULES rather
  than 256 transcribed rows — the transcription-resistant construction
  caught **4 typos in the source table** (0xA0 = EURO SIGN, 0xAD
  undefined, 0x18–0x1F are modifier letters; 232 defined code points
  cross-checked against D.2's 229 + 3). Lesson filed (see RAG
  escalations in the SESSION_LOG continuation-20 entry).
- **`text_extract/{cmap,font,page,layout,mod}.rs`** — the §9.10.3
  ToUnicode CMap parser and the §9.10.2 ladder with rung 3
  structural+named: `Rung3Gap::{IdentityNoToUnicode, Ucs2NotBundled,
  PredefinedCmapNotBundled}` — a rung-3 miss is never silently
  skipped. The derived layer (spaces, line breaks, ordering) is
  isolated in `layout.rs`, never mixed into sourced decoding.
- **API — the fuzzy-never-sneaky split, made structural:**
  `ExtractedText` exposes `plain_text()` (with derived layout) vs
  `sourced_text()` (spec-sourced characters only). The Drucker
  `/ActualText` example verifies both directions: sourced "Drucker",
  plain "Druc\nker" with the one break labelled derived. This dual
  API is the recorded pattern for all future extraction-like features
  (OCR next) — `ARCHITECTURE.md` §12 continuation-20 entry (c).
- **Two additive counted deviations (both named, never silent):**
  per-code fallthrough (§9.10.3 NOTE 4 — unsourced universal
  practice, counted) and a glyph-name extension for fonts failing
  method 2's whole-array precondition. `FontNote::BuiltinEncodingUnreadable`
  names the one R21-unreachable case (embedded symbolic built-in
  encoding → StandardEncoding fallback — counted as extension, never
  reported as sourced).
- **Measurements (2,907 files, 281,516 codes, 0 panics/timeouts):**
  rung 1 78,101 (27.74%); rung 2 202,793 (72.04%); rung 3 zero;
  extension 39 (0.01% — almost all on the deliberately non-conforming
  Isartor 6-3-7 encodings file); failed 583 (0.21%). **SOURCED TOTAL
  99.78%.** Derived layout: 752 spaces, 1,905 line breaks.
- **GUI** (per the ui-specialist design,
  `docs/ui_specs/pass-4-text-extraction.md`, 573 lines): copy-text is
  an ungrouped **toolbar menu button**, NOT a Tools-dock entry — the
  dock is for outside-the-document arguments; this is a THIRD
  placement pattern the Pass 3.2 rail-vs-dock binary didn't cover
  (decision-log extension, §12 continuation-20 (b)). Extraction
  diagnostics are a **snapshot surface** separate from the per-frame
  render header (merging would lie on navigation). Pre-copy
  reliability gate fires on `identity_fonts_without_to_unicode > 0 ||
  sourced < 50%` — deliberately not a low threshold.
- **Real pre-existing GUI bug found by the specialist's verification
  and FIXED:** Ctrl+S fired straight through a live signature
  confirmation — the doc comment claimed a guard that didn't exist,
  and Pass 4's second centre-anchored window made the collision
  reachable. Fix: a one-question gate at the top of `apply()`, doc
  comments corrected; `status_is_open()` now requires a page
  (`/Count 0` nit). Filed as an egui-tier RAG lesson (pending-state
  gates live in the action dispatcher, not the window code).

**Tests:** **875** workspace tests green (was 769).

**Gates:** `cargo fmt --check` / `clippy -D warnings` clean; GUI-free
invariant verified on both targets; wasm32 / `--duplicates` / R24 /
`no-network` / `ui-strings` all clean; **Pass 3.x gates UNMOVED**;
veraPDF §6.1.12 **44/44** with headroom MEASURED (1,674 CMap singles
vs the 500k ceiling, 2,044 ranges vs 100k); fuzz `text_extract`
50,215 runs / 61 s zero crashes, all 10 targets build. **NO new
dependencies** — bidi is deferred-not-half-done: RTL presence
detected + counted, `unicode-bidi` NOT added (B1–B3 would make
reordering wholly derived); `cargo-about` output byte-identical.

**Residuals (open, named):**
- Bidi reordering deferred (named, counted — see above).
- `/Alt` and `/E` expansions counted, not substituted.
- Nested `/ActualText`: outermost wins.
- Artifacts excluded-by-policy but still present in runs.
- Structure-tree reading order is recognition-only.
- Derived layout assumes axis-aligned text — rotated text
  over-produces line breaks (cannot affect sourced chars).
- Canvas text-selection deferred WITH its real spec written
  (`docs/ui_specs/pass-4-text-extraction.md`) — verified to need no
  core addition (`ExtractedGlyph` already carries per-glyph
  `LadderRung` + geometry).

**Demo run:** CLI on `hello.pdf` + the CID fixtures, both text
directions; GUI relaunched (PID 41588); a 20-page tagged manual —
34,037 codes, 100% sourced, **66 ms** (the specialist's
background-extraction concern measured-and-unneeded).

### Pass 3.2 — Structural page operations (Core document ops bucket) — 2026-07-31

First operator-visible editing feature: the seven page operations,
against acceptance criteria grounded in the `pdfce-acrobat-librarian`
"Core document ops" bucket and the `pdfce-ui-specialist` spec
(`docs/ui_specs/pass-3.2-page-ops.md` — implemented with recorded
deviations, below).

- **New `pdfce-core` surface:** `graph.rs` (`ObjectGraph` — ONE
  page-tree walk shared by every op, over either the loaded file or
  the `EditSession` overlay; `edit.rs`'s Pass-3.1 comment predicted
  exactly this need), `signature.rs` (810 lines), `pageops/` (2,833
  lines), `tests/page_ops.rs` (967 lines).
- **Seven ops, two shapes:** *in-place* `EditSession` commands
  `delete_pages` / `reorder_pages` / `rotate_pages` (one undo entry
  each, per the spec's §3.9 table); *producers* `extract` / `merge` /
  `split` / `insert` through one shared `assemble()` (each writes a
  brand-new document — no undo entry, sources untouched).
- **Deletion writes a real free list (decision 007 W9):**
  `DirtySet::delete` + `apply_free_list` — type-0 entries,
  generation+1 saturating at 65,535, spliced onto the FRONT of the
  existing free chain; pre-existing detached free entries left
  byte-untouched (R33). A two-closure reachability sweep proves
  objects shared with surviving pages are never freed.
- **Signature awareness shipped as a real API, not the spec's
  fallback:** `EditSession::signature_impact_of_save(mode)` (takes
  the save mode — deviation, see below) returning
  `SignatureImpact::{None, ByteRangePreserved, Invalidated}` —
  `ByteRangePreserved` renamed from the spec's `PreservedIncremental`
  per the mid-Pass DocMDP relay from `pdfce-spec-librarian`
  (§12.8.1 NOTE 1 preserves the BYTE RANGE; DocMDP validity is a
  separate verdict — the name no longer overclaims). Classification
  walks `/Reference` → `/TransformMethod`: `/DocMDP` lives in the
  signature's reference array, never `/Perms`; `/P` defaults to 2;
  a `/Perms`→`/DocMDP` certification with P forbidding the change ⇒
  `EditError::CertificationForbidsChange` — a NAMED refusal (Table
  258), never a silent proceed. `/FieldMDP` recognized. Spec closure:
  `PDF_Spec` `iso32000__s__12.8.md` now 689 lines with the a/b/c
  verdicts and the ByteRangePreserved-never-reported-alone rule.
- **Carryover policy table (documented + cited in `pageops/`):**
  outlines — subset + repoint for extract/split,
  per-source-top-level merge, target-only for insert; `/Dests`
  name trees never carried — carried bookmarks are rewritten to
  explicit destinations; `/PageLabels` — kept-but-stale for insert
  WITH a named diagnostic, dropped for subsets; `/StructTreeRoot`
  dropped + counted; AcroForm fields auto-renamed `Doc<N>_` on
  merge collisions, fields straddling the extraction boundary
  dropped WHOLE + counted; every reference-barrier hit counted.
  All fuzzy-never-sneaky: counted and disclosed, never silent.
- **Two real bugs caught by this Pass's own tests** (both filed as
  `personal_rag/pdf` lessons):
  (1) reorder LOST INHERITED ROTATION — `materialize_for` was
  one-directional (copied inherited values down, never wrote the
  *default* when the new parent chain supplies a value the old chain
  didn't); `preserve_inherited` now writes §7.7.3.4's default
  explicitly.
  (2) extract left `/Dest [null /Fit]` — nulling one element of a
  destination array produces a present-but-malformed `/Dest`; the
  reference barrier now propagates through the WHOLE array.
- **Engineer deviations from the UI spec (recorded):**
  (1) **Insert is a producer, not an `EditSession` command** —
  overlay insert needs per-object source buffers plus an
  overlay-aware renderer; GUI Insert deferred, the Tools dock names
  the CLI command instead;
  (2) rail checkbox is one interaction with a position test, not two
  overlapping `interact`s;
  (3) `egui::Window` (non-collapsible, input-blocking, added last)
  instead of `egui::Modal` — the spec's own named fallback;
  (4) `signature_impact_of_save(mode)` takes the save mode as a
  parameter;
  (5) split's file-size criterion deferred + named in the dock.
- **Spec priority ledger:** P0 ALL shipped (rail multi-select +
  Delete/Reorder/Rotate-batch/Extract, Tools-dock scaffold + toggle,
  the REAL SignatureImpact gate — not the fallback wording — and the
  dangling-reference count shipping WITH Delete); P1 — signature API
  + dangling count shipped, GUI Insert deferred (CLI `insert`
  complete); P2 — **Merge GUI SHIPPED**, Split GUI deferred (CLI
  `split` complete).
- **Carried small items applied:** properties Apply/Revert grey-out,
  per-field lossy marking, command-named undo/redo tooltips,
  **`pdfce-gui` file argument (the Pass 1.1 remainder — CLOSED)**,
  rotate keyboard shortcut `[` / `]`.

**Tests:** **769** workspace tests green (was 707).

**Gates:** `cargo fmt --check` / `clippy -D warnings` clean.
**Pass 3.0/3.1 gates UNMOVED (R34):** identity 2,892/2,892;
full-rewrite 2,891/2,892 (the miss the same correct hybrid named
refusal); edit → undo → save 2,891/2,891; raster 5,771/5,771. **New
corpus page-op sweep:** 2,892 extract-ok, 23/23 delete-ok, 0
failures. **veraPDF §6.1.12:** 40 files clean, and the new guards'
headroom MEASURED, not asserted (outlines 10 vs 200k ceiling, dests
62 vs 100k, depth 3 vs 64, pages 10k vs 1M). **Fuzz:** new
`pageops_sequence` target 130,400 runs / 61 s zero crashes;
`writer_roundtrip` re-run clean. GUI-free invariant verified on both
targets; wasm32 + aarch64 clean; `--duplicates` + R24 assertions
clean. **`ui-strings` R1 gate clean for the FIRST time** — 3
pre-existing false positives fixed; evidence CI has never run (W15).
**No new dependencies** (no `cargo-about` regeneration owed).

**Record reconciliation (librarian, this filing):** the R36
rule-number collision flagged in the UI spec's header is resolved —
the §5.4 linearization-never-repaired rule is now **R42**; see the
dated note at R42 under Standing rules.

**Residuals (open, named):**
- `/Info` edits are NOT certification-gated — a `/DocMDP` `/P 1`
  strict reading arguably forbids them; owed decision, recorded at
  `check_certification`.
- `PermissionGate::NotApplicableYet` — encrypted-file permission
  gating awaits Pass 5.
- Delete corpus coverage is thin (only 23 multi-page corpus files) —
  fixtures + fuzz carry the weight; re-measure on an organic corpus.
- `qpdf` not on PATH — the R40 external structural oracle went
  unused this Pass; operator-installable improvement.

**Demo run:** GUI launched (PID 23332) on a real file via the new
file argument; CLI demo chained `split` (by bookmark) →
reverse-order `merge` → `render-page` on the result.

### Pass 3.1 — Mutation writer + dirty-set diff + undo/redo command log — 2026-07-31

First real mutation. `ARCHITECTURE.md` §11.4 bound here and was
honored: the command-log undo stack shipped **in** this Pass, not
retrofitted. Mutation surface deliberately minimal per the scoping
(document `/Info` metadata + page `/Rotate` — dictionary values only),
so the Pass tested the dirty-set machinery, not content re-emission.

- **New surface:** `EditSession` command log
  (`crates/pdfce-core/src/edit.rs`, 1,608 lines — commands with
  apply/revert over an overlay above the untouched base revision);
  `writer/fileid.rs` (§14.4 `/ID[1]` derivation); `DirtySet`
  (replacements + trailer patch + `changes_content`); **`save_full`
  now takes `&DirtySet`** (deviation 1 — ONE writer path;
  `DirtySet::empty()` makes the Pass 3.0 identity behavior a strict
  pinned subset, not a parallel code path); CLI `set-info` /
  `rotate-page` / `--verify-undo` / exit code 9 / appended `promoted=`
  key (stable-line contract kept); GUI properties panel + rotate +
  undo/redo + "Save a copy…"; `tools/roundtrip` mutation mode; fuzz
  edit-history extension.
- **Key test — §11.1's "union of every command ever run" bug, made
  executable: edit → undo → save is byte-identical 2,897/2,897
  (100%)**, plus 6 dedicated fixture tests (incl. an object-stream
  file, a 12-command history, and undo → redo → save).
- **Pass 3.0 identity gate UNPERTURBED (R34):** 2,892/2,892 + 6/6
  fixture files; full-rewrite 2,891/2,892 with the single miss the
  same CORRECT hybrid named refusal; raster self-oracle 5,783/5,783;
  0 objects re-serialized. **Mutation gate:** edit applied + reloaded
  100%; all other objects byte-verbatim 100%.
- **Fuzzer found AND fixed a real bug:** creating an object raised
  `/Size` and thereby **RESURRECTED xref entries the base `/Size` was
  suppressing** (entries beyond `/Size` must stay hidden; the
  resurrected ones then failed to parse). Fix: `next_object_number`
  allocates above the **unfiltered** chain maximum (it was reusing
  live numbers), and creation is refused by name when `/Size`
  suppresses entries (`EditError::ObjectCreationWouldExposeHiddenObjects`,
  CLI exit 9 — editing existing objects still works). Post-fix:
  408,886 runs / 91 s zero crashes; `load_document` 681,645 / 61 s
  clean. Lesson:
  `C:\personal_rag\pdf\lesson_20260731_xref_size_suppresses_trailing_entries_raising_resurrects.md`.
- **Engineer deviations 2–5 (recorded as decisions —
  `ARCHITECTURE.md` §12, continuation-18 entry):**
  (2) `/ID` is never synthesised when absent, in either save mode
  (R41 — the spec RAG's synthesise-on-full-rewrite recommendation
  DECLINED; deferred to a real Save-As path);
  (3) rotate-to-base-value writes nothing — exact base spelling
  restored, 4 quarter-turns net to zero, `/Rotate 450` is NOT
  normalised (R33);
  (4) text-string encoding is ASCII-or-UTF-16BE+BOM only —
  §7.9.2/Annex D.3 PDFDocEncoding is a RECORDED RAG GAP; undecodable
  bytes → U+FFFD with `exact: false` surfaced in the GUI
  (fuzzy-never-sneaky);
  (5) the GUI applies edits on button press, not per keystroke — one
  undo step per operator intent.

**CRITICAL correction (2026-07-31, filed forward — the archived 007
decision file is NOT edited):** decision 007 W3's mitigation and
`ARCHITECTURE.md` §5.2's framing claimed R35's full rewrite "closes
the stale-copy path" for promoted compressed objects. **FALSE** —
object streams carry through **verbatim in BOTH save modes** (§5.6:
`save_full` carries containers intact, zero promotions), so a
promoted object's old value survives inside its untouched container.
Documented at the creating code; **the Redaction Pass must
rewrite/decompose any container stream holding a redacted object.**
Full amendment: `ARCHITECTURE.md` §5.7; dated note at R38 below.

**R38 coverage honesty:** promotion is **fixture-covered, NOT
corpus-covered** — 75 corpus files hold 2,197 compressed objects, but
page objects are uncompressed in ALL of them, so rotation never
promotes on the corpus; the harness reports both numbers so the gap
stays visible.

**Tests:** 52 new (32 pdfce-core + 20 pdfce-cli) over Pass 3.0's 585
workspace baseline, all green.

**Gates:** `cargo fmt --check` / `clippy -D warnings` clean;
GUI-core separation verified; dependency set UNCHANGED (no
`cargo-about` regeneration owed). Nothing committed to git.

**In flight at filing (parallel):** UI follow-up items handed to
`pdfce-ui-specialist` (review in flight); **Pass 3.2 remains blocked
on `pdfce-acrobat-librarian`'s "Core document ops" bucket — dispatch
in flight.**

### Pass 3.0 — Identity writer + round-trip proof harness — 2026-07-31

Scoped by decision 007; the first writer slice, deliberately with
**no editing capability** — its entire acceptance bar was a
corpus-wide executable proof of the `ARCHITECTURE.md` §5
round-trip/minimal-diff invariant, and that proof is now green.

- **Blocker (b) resolved first, NEGATIVE:** `hayro-write` 0.7.0
  (2026-05-27) self-describes as an internal `pdf-writer` converter,
  ~580 LoC, no incremental append — decision 001 §9 trigger 2 does
  **not** fire; the depend-or-contribute question stays closed.
- **Round-trip gate, over 2,898 loadable of 2,914 corpus files** (the
  16 NotLoadable are deliberate `*-fail-*` conformance files):
  - `save_incremental`, empty dirty set: whole-file byte identity
    **2,898/2,898 = 100.00%**.
  - Append identity (prior bytes intact under an appended revision):
    **2,898/2,898**.
  - `save_full` per-object-definition verbatim: **2,897/2,898 =
    99.97%** — the single miss is a CORRECT named refusal (R20-style,
    counted not rounded away): hybrid file "Isartor test suite
    manual.pdf" → `WriteError::HybridFullRewrite`, CLI exit 8
    (R33/R27 posture); incremental save works on it via form A.
  - Raster self-oracle: **5,783/5,783** pages identical.
  - **0 objects re-serialized** under `SaveOptions::identity()`; 0
    panics/timeouts; W14's ~98% STOP threshold never approached.
- **Structural census** (byproduct of the gate): 2,410 classic xref /
  487 xref-stream / 1 hybrid / 36 live-linearized.
- **New surface:** `pdfce-core` writer module
  (`mod`/`serialize`/`encoder`/`xref_out`/`save`),
  `linearization.rs`, `equivalent_across_buffers` on `object.rs`,
  `SectionShape` + `LoadedXref.startxref` on `xref.rs`,
  `tests/writer_roundtrip.rs`, `tools/roundtrip`, fuzz target
  `writer_roundtrip` (661,190 ASan execs / 61 s, zero crashes), CLI
  `round-trip` subcommand with the documented exit-code contract, and
  the `ARCHITECTURE.md` §5.1–5.6 + §11.2 amendments (deliverable 9
  done in-Pass, closing the deferral in §12's continuation-16 entry).
- **Engineer deviations (all recorded as decisions —
  `ARCHITECTURE.md` §12, continuation-17 entry):**
  (1) `ProducerPolicy::Set` never CREATES a missing `/Info` (R41
  anti-stamping posture);
  (2) `save_full` carries object streams intact, zero promotions —
  structurally avoids W3 (type-2 entries name container+index, not
  offsets);
  (3) hybrid full-rewrite refused BY NAME
  (`WriteError::HybridFullRewrite`), never a silent flattening;
  (4) no predictor on emitted xref streams — §7.5.8 never mentions
  predictors on the write side (negative result from the
  write-direction audit);
  (5) no wildcard match arms anywhere in the writer —
  `#[non_exhaustive]` does not bind in the defining crate, so a new
  `Object` variant is a compile error at every decision point, not a
  silent null;
  (6) a NUL-bearing Name emits `#00` and fails reload deliberately
  (§7.3.5).

**`/Encrypt` census (decision 007 parallel cheap task) — RETURNED**
(run by a parallel agent; an earlier engineer residual saying it "was
not run" is stale): 19,940 organic PDFs scanned (20k cap hit,
Dropbox-dominated; read-only, aggregate counts only, nothing copied —
LEGAL §5): **134 = 0.67% carry `/Encrypt`**; revision mix 26 R2 /
30 R3 / 67 R4 / 10 R6 / 1 undetermined-R (FOPN FileOpen DRM — a
non-Standard handler, never silently openable); 92.5% legacy R≤4.
Empty-vs-real user password NOT determinable pre-Pass-5. **Promotion
trigger NOT met — Pass 5 stays behind Pass 4** (dated result recorded
at the Encryption Backlog entry).

**Tests:** 585 workspace tests green (was 487).

**Gates:** `cargo fmt --check` / `clippy -D warnings` clean;
GUI-free invariant verified on 3 targets; wasm32; `--duplicates`
guard; veraPDF §6.1.12 implementation-limits suite **44/44** against
the new writer-side guards; all 8 fuzz targets build; dependency set
UNCHANGED (no `cargo-about` regeneration owed).

**Demo run:** `round-trip` identical=1 (709 → 709 bytes);
append-identity `/Prev`=base-startxref with 20-byte SP-LF entries;
producer preserve-vs-set both ways; hybrid refusal exit 8;
linearization warning surfaced. GUI launched but opened BLANK —
`pdfce-gui` still lacks a file argument (open Pass 1.1 remainder);
the engineer verified rendering via the CLI `render-page` PNG
instead.

**Carried items (corrected — the engineer's carried list items 1 and
5 predated same-day closures: the `/Encrypt` census WAS run, see
above; `filter__jbig2` Table 12 was CLOSED 2026-07-31; the
`filter__dct` sourcing gap closed earlier):** genuinely open are the
Pass 1.1 pdfium pixel-parity harness (NOT closed by the
self-comparison oracle — do not overclaim, per the raster-oracle
note), the encrypted-refusal operator sign-off, W15 (no remote/CI —
CI has never run; operator's call per LEGAL §1), the license
decision, and commit authorization. Everything remains uncommitted in
git. **Pass 3.1 engineer DISPATCHED 2026-07-31, in flight** — see In
progress.

### Pass 2.3 — JPXDecode (JPEG 2000) — image-codec slice 3 — 2026-07-31

Scoped by decision 005; last of the three Pass 2.x codec slices.
Shipped in full. **Pass 2 (image codecs, decision 005) is COMPLETE as
planned** — all five standard image-filter families now either decode
or fail with a named diagnostic.

- **JPXDecode** via `hayro-jpeg2000 0.4` (`default-features = false,
  features = ["std"]` — `simd`/`image` off per R24; **Apache-2.0 OR
  MIT, permissive, no license escalation**). New
  `image_codec/jpx.rs` + `fixtures_jpx.rs` (12 generated fixtures via
  `tools/gen-jpx-fixtures.py` — OpenJPEG/Pillow 12.1.0,
  lossless-round-trip-asserted) + 6 demo PDFs in
  `fixtures/synthetic/jpx/` + new fuzz target `image_codec_jpx`.
- **Fuzz bug found AND fixed:** a 310-byte codestream declaring a
  **65,536-tile grid** over a mere 512×1024 image took **32 s** to
  decode — the tile grid is declared independently of image size, so
  NO existing pixel/byte ceiling saw it. Fix: `jpx::MAX_TILES = 4096`
  (8× the most aggressive real-world tiling; a 32 Mpx image can still
  tile 91×91); the same input now fails in **3 ms**. The input is
  kept as a fuzz corpus seed AND an accept-side test pins the ceiling
  from below, so it can't silently over-tighten. Final campaign:
  15,694 runs / 60 s / zero crashes.
- **Engineer deviations (all recorded as decisions —
  `ARCHITECTURE.md` §12):**
  (1) **The dispatch brief stated Table 89 precedence BACKWARDS**;
  the verified rule is implemented: a PRESENT `/ColorSpace` **wins**
  ("any colour space specifications in the JPEG2000 data shall be
  ignored") — the codestream wins only when `/ColorSpace` is absent.
  `/BitsPerComponent` + `/Decode` are ignored as briefed. Pinned by
  test `jpx_present_colour_space_still_wins`.
  (2) `/Width`/`/Height` are NOT a Table 89 override — the
  dict-for-placement / codestream-for-stride split is retained,
  divergence counted; a per-filter contrast table added to the
  `image_codec` `mod.rs` docs.
  (3) Bit depth is normalized by **full-range scale to 8**
  (`round(v/(2^d−1)×255)`), not high-byte truncation — Table 89
  leaves depth handling to the conforming reader; the 16-bit fixture
  carries a `0x00FF` discriminator pixel that catches the wrong
  choice.
  (4) `/SMaskInData` 2 is **recognize-and-defer**: preblended colour
  returned as stored, alpha not exposed; new counter
  `jpx_smask_in_data_preblended` → CLI key `jpx_preblended`
  (appended — stable-line contract kept) + a GUI line.
  (5) An EXTRA Table 89 gap found in the audit and closed:
  `decode_stencil` hard-required 1-bit data and would have sheared a
  JPX `/ImageMask`'s 8-bit samples 8× — the stencil path now takes
  stride/depth from the codec, thresholds against zero, with
  `/Decode` still honoured (the §7.4.9 exemption).
  (6) hayro's `data_u8()` convenience is **deliberately unused**: it
  interleaves alpha AND computes `1 << bit_depth` on a palette-box
  depth that may be 128 — a shift-overflow panic reachable from
  fuzzed input. pdfce interleaves itself and refuses depths outside
  `1..=31` (named diagnostic `JPX/bit-depth`).

**Corpus** (same 2,914 files): Ok holds **2,892 (99.2%)**; images
rendered **204 → 210**; images unsupported **9 → 3**;
codec-unsupported **7 → 0**; codec-FEATURE-unsupported **0 → 1** —
and that 1 is **named**: JPX/enumerated-colour-space (CIEJab,
space 19, §7.4.9-permitted, unimplemented upstream) — previously a
generic corrupt-file error. Zero panics/timeouts.

**Tests:** 487 workspace tests green (was 457).

**Gates:** all standing gates clean, incl. **MSRV 1.92 builds
core+render (no bump — decision 005 §3.7's zero-headroom hayro MSRV
risk did not bite)**, `cargo-about` regeneration (**+1 entry**), the
R24 feature assertion on 4 targets, the `--duplicates` guard, and
wasm32. GUI launched on `jpx-rgba-smaskindata1.pdf`.

**Carried items:** Pass 2.2's open Table 12 spec item is now a
`pdfce-spec-librarian` dispatch **IN FLIGHT** (running in parallel
with this filing), no longer pending. The next subsystem Pass awaits
**decision 007** (next-subsystem priority; KenAgent consultation in
flight). Everything remains uncommitted in git.

### Pass 2.2 — CCITTFaxDecode + JBIG2Decode + shared bilevel sink — image-codec slice 2 — 2026-07-31

Scoped by decision 005; second of the three Pass 2.x codec slices.
Shipped in full. **Ships on zero corpus pressure, deliberately and
honestly** — see the corpus paragraph below.

- **CCITTFaxDecode** via `hayro-ccitt 0.3` (zero deps, `no_std`,
  `forbid(unsafe_code)`). `DecodeSettings` maps **1:1 onto Table 11**;
  the blocking Table 11 verification came back from
  `pdfce-spec-librarian` and the implemented defaults are verified
  against it. The `/K` trichotomy is implemented (K < 0 pure
  two-dimensional G4/T.6; K = 0 pure one-dimensional G3; K > 0 mixed).
  **`/Rows` 0 → `/Height` fallback is load-bearing, not cosmetic:**
  `hayro-ccitt` decodes ZERO rows when given `rows: 0`, so the
  dictionary fallback is what makes real streams decode at all
  (lesson filed, see below).
- **JBIG2Decode** via `hayro-jbig2 0.3` (`default-features = false,
  features = ["std"]` — `image`/`simd` OFF per R24);
  `Image::new_embedded()` is the `/JBIG2Globals` path. JBIG2 stays
  forbidden in inline images per the scoping (§7.4.7/§8.9.7) — the
  inline path rejects rather than routes.
- **Shared bilevel sink** used by both codecs: §8.9.3 sample packing
  (1 bpc, per-row byte budget) — one packing implementation, two
  codec front-ends.
- **Polarity chain PROVEN, not assumed.** `/BlackIs1` maps by DIRECT
  assignment to hayro's `invert_black` (hayro XORs internally; the
  sink writes 1-for-white; PDF's default is 0 = black). JBIG2's T.88
  1 = black convention is inverted unconditionally. The two routes
  are **byte-identical for the same Group-4 payload** (a G4 stream
  decoded via CCITTFaxDecode vs the same payload as a JBIG2 MMR
  generic region) — asserted in `pdfce-core` tests AND
  pixel-identical renders. This closes the decision 005 §10 item 2
  hazard ("a wrong `BlackIs1` default inverts every fax image
  plausibly and silently") with an executable identity, not a code
  read.
- **Engineer deviations (all additive, recorded here):**
  (1) an EXTRA named diagnostic for CCITT damaged rows beyond the
  scoped set (R27); (2) `pdfce-render` needed **no code change** —
  `SampleLayout` was already generic over 1-bpc data, so the render
  side of the slice is 6 new tests, zero new code; (3) new
  `fixtures/synthetic/bilevel/` (5 demo PDFs + `PROVENANCE`);
  (4) one **honest test relaxation**: `[0xFF; 32]` is VALID Group 4
  data — T.4/T.6 carry no checksum, so fail-clean cannot detect
  undetectable garbage; the test documents this in a comment instead
  of asserting a rejection that would be false.

**Corpus** (same 2,914 files): **honestly UNCHANGED at 99.3%** — zero
CCITT and zero JBIG2 streams exist in the conformance corpora, the
decision 005 §5.1 prediction ("zero by corpus construction —
conformance corpora contain no scanned documents") now **confirmed by
direct scan: 0 files / 0 occurrences** for both filters. The
demand signal remains the OCR and scanned-document Backlog buckets,
for which these codecs are the precondition; re-measure the moment an
organic (non-conformance) corpus exists.

**Tests:** 457 workspace tests green (was 420) — ~45 new including
17 `pdfce-core` + 6 `pdfce-render`.

**Fuzzing:** 6 targets (per-codec targets extended per decision 005
§6.5), 60 s each, zero crashes.

**Guard validation (R25):** veraPDF §6.1.12 implementation-limits
suite **44/44** — the standing per-guard corpus gate run before ship.

**Attribution:** `THIRD_PARTY_LICENSES.md` regenerated — **+2
entries** (`hayro-ccitt`/`hayro-jbig2`), 169 total, zero copyleft.

**Binary size:** +306.5 KiB (**+10.28%**), measured.

**Open spec item (carried, not blocking):**
`PDF_Spec\filter__jbig2.md` still marks **Table 12's exact contents
unverified** — Pass 2.2 implemented against §7.4.7's quoted prose; a
future `pdfce-spec-librarian` dispatch closes it. T.4/T.6/T.88 source
specs are staged in the spec RAG but their code tables remain
unextracted.

**Amendment footer (2026-07-31, same day) — gate results from the
engineer's completion report, all run before ship, all clean** (this
footer completes the entry's invariant-check record; the entry body
above is unchanged):
- `cargo fmt --all --check` clean workspace-wide; `cargo clippy --
  -D warnings` clean workspace-wide; `cargo test --workspace`
  457 green.
- **GUI-core separation** verified via `cargo tree -p pdfce-core` /
  `-p pdfce-render` on native AND `x86_64-pc-windows-msvc` — zero
  windowing deps.
- `wasm32-unknown-unknown` cross-check builds.
- **R24 configuration assertions hold:** `hayro-jbig2`
  `default-features = false` with `image`+`simd` off; `hayro-ccitt`
  `no_std`/`forbid(unsafe_code)`.
- `cargo tree --duplicates` fontations guard clean.

### Pass 2.1 — DCTDecode (JPEG) + LZWDecode (+ RunLengthDecode) — image-codec slice 1 — 2026-07-30

Scoped by decision 005 (`docs/decisions/005-image-codecs.md`); first of
the three Pass 2.x codec slices. Shipped in full, plus one scope
addition and one Pass 1 bugfix found along the way.

- **DCTDecode** via `zune-jpeg 0.5.15` (`default-features = false` —
  SIMD off, compiler-enforced `forbid(unsafe_code)`, R24). The
  Table 13 blocking verification came back from `pdfce-spec-librarian`
  and the implemented precedence is verified against it: **the JPEG's
  own APP14 marker overrides the dictionary's `/ColorTransform`;
  default is 1 for 3-component, 0 otherwise.** APP14 pre-sniff done in
  pdfce's own adapter (decision 005 §5.5's thirty-line marker walk);
  **YCCK→CMYK conversion is in-house** (zune-jpeg has no YCCK→CMYK
  arm — it gets a YCCK-passthrough request).
- **LZWDecode** via `weezl 0.2.1` — a byte-stream filter in the
  `filters::decode_stream` cascade per R23, both `/EarlyChange` modes
  (1 → `with_tiff_size_switch`, 0 → plain; MSB packing).
- **RunLengthDecode** in-house, ~130 lines (§7.4.5) — small scope
  addition alongside the two scoped filters; a truncated stream is a
  strict `Err` (see deviations below).
- **The two-tier image-codec architecture landed** (R23, decision 005
  §4.6/§6.3): `pdfce_core::image_codec` with the `CodedImage` seam
  and `terminal_codec` dispatch; `filters::decode_stream` returns
  `FilterError::ImageCodec` for terminal codecs. **Ceilings are
  explicit pdfce constants** (`MAX_IMAGE_PIXELS` /
  `MAX_IMAGE_DIMENSION` / `MAX_IMAGE_SAMPLE_BYTES`), never vendor
  defaults — `zune-jpeg`'s 16,384-pixel default cap overridden per
  R25.
- **Engineer deviations from decision 005's §6.3 API sketch (all
  additive, recorded here + `ARCHITECTURE.md` §12):**
  (1) `CodedImage::codec` is an `Option` with an `Unspecified`
  variant; (2) `decode_image` takes an `inline: bool` parameter;
  (3) RunLength truncation is a strict `Err`, not a tolerance.
- **Pass 1 bugfix (content.rs inline images):** `ID` followed by CRLF
  is ONE white-space character (§8.9.7 "single white-space character"
  read with §7.2.2's CRLF-is-one-EOL rule). The Pass 1 code consumed
  only the CR, leaving a stray `\n` prepended to the image data —
  silently corrupting 4 corpus inline DCT images (caught by their SOI
  check failing). Lesson:
  `C:\personal_rag\pdf\lesson_20260730_inline_image_id_crlf_single_whitespace.md`.

**Corpus** (same 2,914 files): Ok 99.2% → **99.3% (2,886)**; images
rendered **74 → 201**; images unsupported **135 → 8** — the 8 being
7 JPX files (Pass 2.3's scope) + 1 deliberate `/Lzw`-misspelling
fail-file (correct rejection). Zero panics/timeouts/hangs.

**Decision 005 §3.2 correction (filed as a dated addendum at the end
of the decision record — the record itself is not rewritten):** the
"0 four-component JPEGs in the corpus" measurement was WRONG — **12
exist**, in veraPDF's "6.2.4.3 Uncalibrated -Device colour spaces"
section; the §3.2 scan missed them. **Revisit trigger 2 (§9) is
LIVE:** `6-2-4-3-t02-pass-a.pdf` is `/DeviceCMYK` `/DCTDecode` with
Adobe APP14 transform 2 (YCCK) and NO `/Decode` array — it relies on
the bare Adobe convention, and pdfce currently passes raw samples
through (per §5.5's deliberate no-guess posture), so these 12 likely
render inverted today. **Decision 006 dispatched** to settle the
sourced inversion rule.

**Tests:** 412 workspace tests green (was 338). `cargo fmt --all
--check` + `cargo clippy --workspace --all-targets --all-features --
-D warnings` clean.

**Fuzzing:** 4 targets (per-codec targets added per decision 005
§6.5; `content_and_filters.rs` extended with both LZW `EarlyChange`
modes), zero crashes.

**Attribution:** `THIRD_PARTY_LICENSES.md` regenerated same session —
**+3 entries** (`zune-jpeg`/`zune-core`/`weezl`), all permissive,
zero copyleft (the hayro crates arrive with Passes 2.2/2.3).

**Invariant checks:** all gates green — GUI-core separation
(`cargo tree`, host + `x86_64-pc-windows-msvc` +
`wasm32-unknown-unknown`), `--duplicates` clean (codec crates added
to the guard), wasm32 `cargo check` clean, and the **new R24 feature
assertion** (`cargo tree -e features` shows no `x86`/`neon`/`simd`
anywhere in the graph).

### Pass 1.1 (slice) — Form and Image XObjects (`Do`) + inline images — 2026-07-30

A scoped slice of Pass 1.1 — Corpus validation & hardening (see Next
up; that Pass stays open for pixel-parity measurement, the GUI file
argument, the R20 diagnostics panel, and the next image-codec slice
below). This slice closes the single biggest measured render-fidelity
gap identified by the Pass 1.1 corpus run (continuation 7's "3,387
deferred ops, XObjects/shading" fidelity note).

- **New `crates/pdfce-core/src/filters/ascii.rs`** — `ASCIIHexDecode`
  (§7.4.2) and `ASCII85Decode` (§7.4.3), wired into `filters::apply_one`
  under both full and inline-abbreviated names (`AHx`/`A85`). Landed
  with this slice, not later, because they are the only two filters
  that make an inline image's data length unambiguous (§8.9.7).
  Handles whitespace-anywhere, the odd-final-hex-digit rule, `z`
  shorthand plus the three §7.4.3 "impossible combination" refusals,
  pad-with-`u`-and-truncate for partial groups; tolerates a missing
  EOD and a leading `<~` prefix (both documented as non-conformance
  tolerances, not spec requirements).
- **New `crates/pdfce-render/src/image.rs`** — image XObjects and
  inline images to RGBA pixmaps, following the §8.9.5.2 pipeline order
  (filter chain → BitsPerComponent unpack, per-row `ceil` stride →
  `Decode` transform → colour conversion). Colour spaces: DeviceGray/
  CalGray, DeviceRGB/CalRGB, DeviceCMYK (shared naive conversion with
  `k`/`K`), ICCBased via the `/N` fallback (1/3/4), Indexed over any of
  those (string or stream lookup, palette pre-converted once, 0..hival
  clamp). bpc 1/2/4/8/16. Table 90 defaults resolved from the colour
  space (Indexed's `[0 2^n−1]` identity, not `[0 1]`). Stencil masks
  (`/ImageMask true`) are a separate code path with `Decode` as a
  polarity switch. `MAX_IMAGE_PIXELS` = 32 Mpx guard, checked before
  any decode or allocation.
- **`crates/pdfce-render/src/interpret.rs`** — `Do` dispatch on
  `/Subtype` (Image / Form / PS-silently-ignored / missing-Subtype
  repair heuristic), the §8.10.1 five-step form-execution procedure,
  inline images routed through the same image path.

**Decisions (full text: `ARCHITECTURE.md` §12, 2026-07-30 continuation
9 entry):**
1. Nested form execution uses a fresh `Interpreter` over a clone of the
   current `GraphicsState`, not `q`/`Q` on the shared stack — makes
   §8.10.1 steps (a)/(e) structural (an unbalanced `Q` inside a form
   provably cannot pop the caller's state) and gives each form its own
   font cache (correctness requirement: `/F1` in a form's own
   `/Resources` is a different font than the page's `/F1`).
2. The XObject cycle guard is keyed on object number, not resource
   name (the same stream can be reached under different names).
3. Text objects do not cross the form boundary (§9.4.1's BT/ET
   scoping holds structurally); text *state* (font/size/spacing) is
   inherited because §9.3 makes it graphics state. Pinned by a test.
4. Images are drawn with a tiny-skia `Pattern` shader over the
   user-space unit square, not `Pixmap::draw_pixmap` — `draw_pixmap`'s
   integer x/y origin makes it a blit that cannot express §8.9.4's
   arbitrary-affine placement (rotation, skew, deliberate distortion).
5. **`MAX_XOBJECT_DEPTH` = 64** (was 16, corpus-corrected — see below).

**Corpus finding that changed a guard mid-slice:** the initial
`MAX_XOBJECT_DEPTH` = 16 (chosen from intuition) overflowed on exactly
one of 2,914 corpus files — `veraPDF-corpus/PDF_A-1b/6.1 File
structure/6.1.12 Implementation limits/veraPDF test suite
6-1-12-t08-pass-c.pdf`, a **conformant** file with a deliberate 32-deep
form-XObject chain (objects 19–50, `/X1`…`/X32`). Annex C sets no
form-nesting limit; PDF/A §6.1.12 forbids imposing Annex C limits on
readers. Raised to 64 (2× the deepest conformant structure measured);
overflows are now 0 corpus-wide. Two regression tests pin it (32-deep
must render clean; `MAX_XOBJECT_DEPTH + 3` must be refused). **This is
the second guard in this project caught by the same veraPDF §6.1.12
suite** (the first was `MAX_TOKEN_LEN`, Pass 1.1 item 2) — see the new
standing rule below.

**Corpus before/after** (same 2,914 files; "before" = a scratch build
with only the `Do`/inline-image arms reverted to deferral, isolating
this slice's delta):

| counter | before | after | delta |
|---|---|---|---|
| Ok (parse+render page 1) | 2,890 (99.2%) | 2,890 (99.2%) | unchanged |
| deferred ops | 7,347 | 6,079 | −1,268 (−17.3%) |
| images rendered | 0 | 76 | +76 |
| images unsupported | 0 | 137 | +137 |
| forms rendered | 0 | 1,168 | +1,168 |
| xobject depth overflows | 0 | 0 | (1 at depth-16, fixed to 0 at depth-64) |
| glyphs substituted | 1,139 | 1,176 | +37 (text inside forms now paints) |
| unknown ops | 3 | 5 | +2 (ops inside forms, reached for the first time) |
| tolerated | 4,752 | 4,867 | +115 |

The deferred-ops drop (1,268) is smaller than the 1,381 `Do`/`BI`
operations consumed because executing 1,168 forms newly exposes their
own contents' deferred operators (`sh`, `scn`, `BDC`), never counted
before. Zero panics, zero timeouts, zero hangs.

**Next-slice signal, straight from the data:** `images_unsupported`
(137) now EXCEEDS `images_rendered` (76) — dominated by codecs pdfce
doesn't implement yet (`DCTDecode` first, then `CCITTFaxDecode`/
`JBIG2Decode`/`JPXDecode`/`LZWDecode`). See Next up item 6.

**Diagnostics/contract changes:** `Diagnostics` gained
`images_rendered`, `images_unsupported`, `image_notes: Vec<String>`,
`forms_rendered`, `xobject_depth_overflows`, plus a `merge` that folds
a nested form's counters into the page's. `pdfce-cli render-page`'s
stdout contract gained three APPENDED keys (`images`,
`images_unsupported`, `forms`) after the existing five, which did not
move — contract test updated to assert key order. GUI: the two
shortfall counters fold into the "unsupported items" headline; three
new `ui_text` detail strings; the deferred-ops string reworded (no
longer covers images). `tools/corpus-report` gained the four new
counters.

**Fuzzing (§10.2 — "expand fuzz targets to each filter decoder as
they're implemented"):** `fuzz/fuzz_targets/content_and_filters.rs`
extended to call `pdfce_core::filters::ascii::decode_hex` and
`decode_85` directly on the raw fuzz input, not through
`decode_stream` — no dictionary shape stands between libFuzzer and the
byte loops. Rationale (recorded in the target's module docs): ASCII85
has genuine overflow surface — a five-digit group accumulates to a
value that can legitimately exceed `u32::MAX` (`uuuuu` = 85^5 − 1),
partial final groups index a fixed-size array by a running count, and
`z`/`~>` are position-sensitive. **Campaign: 588,048 ASan-instrumented
executions in 120 s, ZERO crashes** (nightly-x86_64-pc-windows-msvc,
`clang_rt.asan_dynamic-x86_64.dll` PATH workaround per
`D:\dev\rag\rust\cargo_fuzz_windows_msvc_asan_dll_path.md` — lesson
followed, confirmed still accurate, no update needed).

**Post-slice refinements:** a regression test pinning §8.10's "a
`/BBox` with zero width or height is legal and means paint nothing"
(guards against the failure mode of treating the degenerate rectangle
as *absent* and painting the form unclipped — the exact opposite of
the spec); removed the unused `pub` helper `ImageNotes::any` from
`pdfce-render`'s public surface, per the Rust API Guidelines'
don't-ship-unused-public-items posture.

**Tests:** 338 workspace tests green (was 245 at Pass 1; 74 in
pdfce-render alone; count corrected from an initial 337 after the
post-slice refinements above). `cargo fmt --all --check` clean;
`cargo clippy --workspace --all-targets --all-features -- -D
warnings` clean.

**Invariant checks:** `cargo tree` for pdfce-core/pdfce-render/pdfce-cli
on host, `x86_64-pc-windows-msvc`, and `wasm32-unknown-unknown` — no
egui/eframe/winit/wgpu; `cargo tree --duplicates` clean (no second
skrifa/read-fonts/font-types major); `cargo check -p pdfce-core -p
pdfce-render --target wasm32-unknown-unknown` clean. One new
dev-dependency (`flate2` on pdfce-render, test-only, already resolved
via pdfce-core, adds zero packages — no `THIRD_PARTY_LICENSES.md`
regeneration needed).

**Packaging:** untouched this slice, no packaging smoke test owed; per
the launch-on-completion rule the GUI was rebuilt and launched, and
`pdfce-cli render-page` run against real corpus files (a gradient
image, a 4-image page, the 32-deep form chain) with visually verified
PNG output.

### Pass 1 — Minimal parse + render (read-only viewer) — 2026-07-30

Shipped the same day as decisions 001–004 — the entire Pass, spec RAG
to launched GUI, in one operator-initiated `/loop` autonomous session.

- **`pdfce-core`** — complete Pass 1 read stack: `span` / `lexer` /
  `object` / `parser` / `xref` / `document` / `page_tree` / `content` /
  `filters` / `fontdata` (197+ unit tests). ByteSpan provenance is
  first-class; ONE object model (decision 001 §6.1.3); fail-clean
  filters (`Result<_, FilterError>`) with decompression-bomb guards;
  lossless span-provenanced content token model; standard-14 metrics /
  Annex-D encoding / AGL tables generated from sourced data (APAFML +
  BSD-3-Clause AGL, per the spec RAG font-licensing verdicts).
- **`pdfce-render`** — full path rendering (all Table 59/60/61
  operators, deferred-`W` clipping, §8.3 CTM math), device colours,
  dash/caps/joins, `gs` subset, `BX`/`EX` — AND full text rendering:
  simple fonts through the complete §9.6.6 encoding chain (all four
  `FontFile` flavors via `skrifa`), `Identity-H` composites
  (CIDFontType0 + CIDFontType2), §9.4.4 Trm/advance math, standard-14
  substitution via the 14 bundled Foxit faces (BSD-3-Clause,
  provenance-verified per R22). R17–R22 enforced throughout.
- **`pdfce-gui`** — read-only viewer: page canvas with pan/zoom
  (re-raster debounced), thumbnail rail, page navigation with keyboard
  shortcuts, three-way render-failure distinction including an honest
  "unsupported ≠ broken" state, R20 substituted-glyph diagnostics in
  the status bar. ui-specialist-reviewed; HiDPI-correct.
- **`pdfce-cli`** — `render-page` implemented, with a documented
  machine-parseable stdout contract (R5-conformant).
- **Fuzzing (§10.2 requirement — met):** 3 `cargo-fuzz` targets, ~4M
  ASan-instrumented executions, ZERO crashes.
- **Attribution:** `THIRD_PARTY_LICENSES.md` regenerated — 164 crates,
  zero copyleft — plus the manual embedded-data epilogue (APAFML AFM
  data / Adobe Glyph List / Foxit faces), discharging the Pass 1
  manual-attribution obligation. Along the way the template's
  empty-license-text bug was found and fixed
  (`D:\dev\rag\rust\cargo_about_template_text_var_and_epilogue.md`).
- **Fixtures:** new `fixtures/synthetic/hello.pdf` (loadable,
  text + graphics).

**Tests:** 245 workspace tests green; `cargo fmt --check` +
`cargo clippy -- -D warnings` clean workspace-wide.

**Invariant checks:** ALL CI guards verified locally —
`gui-core-separation` (native AND shipped-target
`cargo tree --target x86_64-pc-windows-msvc`), `cargo tree
--duplicates` (no second `skrifa`/`read-fonts`, R21), `ui-strings`,
`no-network`, wasm32 cross-check of `pdfce-core` + `pdfce-render`.

**Packaging smoke test — PASSED:** fresh-folder run with no install
step — `pdfce-cli render-page` produced correct output; `pdfce-gui`
launched and rendered.

**Honest scope statement — what Pass 1 did NOT demonstrate:** the
original acceptance bar's "pixel parity (or documented near-parity)
with a reference renderer" is NOT yet demonstrated — only synthetic
fixtures were rendered; no real-world corpus has been measured.
Known coverage gaps carried forward: xref streams / object streams /
hybrid files are clean-refused (most modern PDFs use them); Type 3
fonts are diagnostics-only; `Tr` 4–7 text clipping unimplemented;
`/Resources`-missing files strict-refused (including
`fixtures/synthetic/minimal.pdf` itself). All filed as **Pass 1.1**
under Next up.

### Pass 0 — Workspace bootstrap — 2026-07-23

Cargo workspace created with the four crates specified in
`ARCHITECTURE.md` §3/§7, each meeting its Pass 0 acceptance bar:

- **`pdfce-core`** (dep: `thiserror` 2.0.19) — implements the Pass 0
  `%PDF-` header probe: `probe_header(&[u8])` and `probe_file(&Path)`
  returning `PdfVersion { major, minor }`; error type `PdfError`
  (`thiserror`, `#[non_exhaustive]`, per the C-GOOD-ERR API guideline).
  Scans the first `HEADER_SCAN_WINDOW` (1024) bytes for the marker,
  tolerating a leading BOM/whitespace, then parses `M.N`. Cites
  ISO 32000-2:2020 §7.5.2. Zero GUI/windowing dependencies.
- **`pdfce-render`** — Pass 0 stub only: re-exports `pdfce-core`'s
  `PdfVersion`/`PdfError` and exposes a `PLANNED_RASTERIZER` const.
  `tiny-skia` deferred to Pass 1 (no rendering yet). Zero GUI deps.
- **`pdfce-gui`** — `eframe` 0.35.0 (GLOW backend, not wgpu — see
  decision (c) below) + `rfd` 0.17.2. Opens a native window, blank
  canvas, with a working Open… dialog that runs the header probe and
  displays the detected version or a clear error message.
- **`pdfce-cli`** — `clap` 4.6.4. `inspect <file>` fully implemented;
  nine stub subcommands (`merge`/`split`/`extract-pages`/`rotate`/
  `bates-stamp`/`to-pdfa`/`validate-pdfa`/`sign`/`render-page`) print a
  "not implemented, later Pass" message. Documented exit-code contract:
  `0` ok, `1` generic, `3` IO, `4` not-a-PDF, `64` unimplemented (2
  reserved by clap). Zero GUI deps.

**Tests:** 13 passing (8 `pdfce-core` unit + 2 `pdfce-core` doctests +
3 `pdfce-cli`).

**Invariant checks (GUI-core separation):** `cargo tree -p pdfce-core`,
`-p pdfce-render`, and `-p pdfce-cli` show ZERO
`egui`/`eframe`/`winit`/`wgpu`/`glow`/`glutin`/`rfd` — verified
explicitly, not assumed. `pdfce-gui` correctly HAS `glow` and does NOT
have `wgpu`.

**Style/lint:** `cargo fmt --all --check` clean; `cargo clippy
--workspace --all-targets --all-features -- -D warnings` clean.

**Packaging smoke test — PASSED:** release build with static CRT; both
binaries copied to a fresh temp folder and run with no install step —
`pdfce-cli inspect` returned "PDF 1.7" exit 0; `pdfce-gui` opened a
window (title "pdfce", valid window handle, glow/OpenGL initialized, no
crash). `dumpbin /dependents` confirms NO VC++ redistributable
dependency (only OS DLLs: kernel32/user32/opengl32/etc.). Binary sizes:
`pdfce-cli.exe` 0.83 MB, `pdfce-gui.exe` 7.27 MB.

**Attribution:** `cargo-about` set up (`about.toml` permissive-only
accept-list + `about.hbs` template); `THIRD_PARTY_LICENSES.md`
generated — 158 crates, all permissive
(Apache-2.0/MIT/Unicode-3.0/Boost/ISC/0BSD/zlib) + 2 bundled-font
licenses (OFL-1.1, Ubuntu-font-1.0 via `epaint_default_fonts`). ZERO
copyleft.

**Toolchain:** `rust-toolchain.toml` pins 1.97.1; edition 2024;
resolver 3; MSRV `rust-version` = 1.92; `Cargo.lock` committed (not
ignored).

**Two Pass-0 acceptance items resolved as user decisions** (see
`ARCHITECTURE.md` §12 entries (a) and (b), 2026-07-23): egui/eframe
CONFIRMED over iced; and the build-from-scratch-vs-`oxidize-pdf`
question was NOT resolved this Pass — the user chose a thin,
header-probe-only `pdfce-core` stub for Pass 0, deferring that decision
to a dedicated `oxidize-pdf` audit that remains the gate before Pass 1.

## In progress

### Pass 21.1 — FF-C composite-run editability under verified-injective `/ToUnicode` (decision 021 R110; promoted from ★ Pass 21.x's Next-up slice list, 2026-08-04, on Pass 21.0's ship)

**Promoted the same session Pass 21.0 shipped (`48c6b77`), per decision 021's own instruction not to call FF-C done without it.** See the Pass 21.0 Shipped entry (top of Shipped) and the ★ Pass 21.x entry (Next up, below) for full framing. Scope: core + CLI. `/ToUnicode` verified injective (every CID maps to exactly one scalar, no two CIDs share a scalar) per font, per session, before a composite run FF-C authored becomes editable (R110); conditionally lifts R-INV-4 for that font only. Non-injective, absent, or partial `/ToUnicode` keeps refusing — `Identity-H` with no `/ToUnicode` remains R65's permanent hard skip, untouched by this Pass. **Also owed alongside 21.1, named at Pass 21.0's ship and not yet assigned its own slice number:** R109's `fsType` donor-permission read, which decision 021 scoped into 21.0 but did not ship there (see the Pass 21.0 Shipped entry's "NOT yet implemented" note) — whoever picks up 21.1 should confirm with the engineer whether to fold the fsType read in here or open it as a small standalone slice before 21.2.

**Continuation 76 (2026-08-04) build log — three landed commits, this Pass is closer to shippable but NOT there yet.**

- **`58fe3f6`** — R109's fsType read, folded into this Pass's build rather than opened as a separate slice (the engineer's call, resolving the "fold in or standalone" question left open above). Full design recorded on R109's own Standing-rules bullet — three named refusals (`SubsettingNotPermitted`, `EmbeddingNotPermitted`, and the correct non-firing of either on `nosubset-v1`'s v1 `OS/2`), read before subsetting because `subsetter` strips `OS/2`, seven fixtures. Two non-refusal cases (absent/unparseable `OS/2`; `fsType == 4` Preview & Print) ship as an interim disclose-and-proceed default — Open operator question (r), below, stays open for either to be overridden.
- **`c0ed638`** — `ToUnicodeCMap::injective_inverse()`, the R110 primitive. Full design on R110's own Standing-rules bullet — three named disqualifying obstructions (ligature, many-to-one collision, empty map), ranges materialised (not lazily resolved, unlike ordinary lookup) so a range/single collision can't hide, and a non-committal test against the standard's own §9.10.3 EXAMPLE 2 (asserts the check runs and decides, not that the standard's example inverts).
- **`8e08e80` + `87d3cb0` + `6b69956`** — **the headline finding.** A composite run's R-INV-4 refusal was UNREACHABLE from `edit-text` — `edit.rs` carried a comment claiming the refusal fired later; it never did, because the text-match stage failed first (`NoMatch`) on every composite run, present-and-locatable or not, so `classify_font` (R-INV-4's home) was never reached. Same shape as the Pass 19.4 `Tw`/R91 dead-guard finding, this time on R-INV-4 — filed as a SECOND occurrence of the existing RAG finding, not a new file (see RAG escalations, below). Fix: classify the font BEFORE matching, since the font-level refusal is a property of the run, not of whether the sought text is inside it. Composite runs now decode far enough to be findable and no further (`ShowSlot::code` stays `u8`, can't hold a CID). `6b69956` is a regression test asserting the ERROR VARIANT, verified non-vacuous by reverting the fix and watching the wrong diagnosis come back.

**What this Pass has NOT yet delivered: actual editability.** Composite runs are now correctly located and refused for the right, disclosed reason — they are not yet rewritable. `ShowSlot::code` (currently `u8`) must widen to hold multi-byte CIDs, and the content-stream operand writer must learn to emit multi-byte show operators, before R110's conditional lift has anything to attach to. Until that lands, Pass 21.0's own capability-regression warning (pdfce can add composite text it cannot edit) stays live, unchanged by this continuation's work.

**Honest limit, carried forward from the reachability fix:** the widened decode assumes `Identity-H` specifically (what pdfce itself writes, and what real composite text overwhelmingly uses in practice) — other CMap encodings on a composite run remain invisible to this decode path, same as before the fix. Narrowed, not regressed.

**RAG escalations, continuation 76:**
- `D:\dev\rag\rust\dead_guard_clause_behind_a_filter_the_guarded_case_cannot_pass.md` — extended with a SECOND occurrence (R-INV-4/`edit-text`, distinct code path from the original R91/`Tw` finding, same exact shape) and a generalized framing: a precondition check (a property of the OBJECT) placed after a search step (a property of the QUERY) only ever fires for objects the search can already handle — so the cases it exists for are exactly the cases that never reach it.
- `D:\dev\rag\rust\trust_but_verify_doc_comments_are_not_evidence.md` — extended with a fourth occurrence (the `edit.rs` comment asserting R-INV-4 fires "later" when it structurally could not) — now four confirmed instances of a confidently-worded comment asserting untrue runtime behavior on this project alone. Flagged to the engineer as a pattern frequent enough to be worth judging for standing-rule elevation — that adoption call is not this librarian's to make solo (same discipline as continuations 74/75).
- Both indexed already (edits to existing files, not new ones) — no `index.md` change needed this filing.
- No `personal_rag/pdf` entry this continuation — the fsType semantics are canonical OpenType-spec content (spec-librarian's territory, already sourced), and the Identity-H-prevalence honest-limit note above is a restated existing observation, not a new empirically-verified finding from this session (no fresh census was run to back it).

**Continuation 78 (2026-08-04) build log — substrate for editability landed (three commits), WIRING DELIBERATELY NOT STARTED. Still In progress, still not shippable.**

- **`31d2fdc`** — `ShowSlot` widened: `code: u8` → `code: u32`, plus a per-slot `width: u8` (1 = simple, 2 = Identity-H). This is the specific type that made a composite run UNREPRESENTABLE, not merely unimplemented — `match_run`'s `+ 1` advance became `+ width`, the same number for every code that could reach it before the widening, which is exactly why the old constant looked correct. Three narrowings back to `u8` (`prefer`, `carried_codes`, `MatchRun::old_codes`) all go through `filter_map`, never a bare cast — a truncated code is a DIFFERENT, VALID code (not an error value), so a silent truncation would splice confidently wrong text or tell R-INV-1 the page carries a glyph it does not. Landed ALONE and re-verified: all 1801 tests passed unchanged, which is the entire claim for this commit — the type widened, nothing downstream yet reads the new range.
  - **Near-miss worth recording as its own finding, not a footnote (see RAG escalation, below):** with the type widened, the obvious next move is to start pushing slots for composite runs. It compiles and every existing test passes — and it would have SILENTLY DISARMED `tests/composite_refusal_reachable.rs` (the continuation-76 regression test). That test catches someone moving font classification back below `match_run`, and it works BECAUSE composite runs currently produce no slots (the match fails, the wrong `NoMatch` surfaces, the test's assertion holds for the wrong reason). Give composite runs slots and the match starts succeeding — `classify_font` still refuses correctly, so the test's assertion (an error occurs) still passes, but now on the CORRECT ordering, meaning the test would stay green even if a future edit silently moved classification back below matching. The guard goes quiet at the exact moment the defect it guards against becomes reachable again.
- **`b98589a`** — `CompositeEncoding`: character → CID, constructed ONLY from a verified-injective `/ToUnicode` (goes through `injective_inverse()`, the R110 primitive shipped continuation 76) — a ligature table or a colliding map never yields an encoder at all, so the refusal happens where the evidence already lives (R110), not later at encode time when the caller has already committed to an edit. A SEPARATE type from `InverseEncoding` (the existing simple-font encoder), not a mode flag on it — the simple encoder reasons about glyph names, `/Differences`, ligature components, and code-occupancy, none of which exist for a CIDFont; forcing one type to cover both would mean half its fields are `None` for every composite call and vice versa. **The load-bearing test is byte order:** `Identity-H` codes are big-endian per §9.7.6.2 — reversing the two bytes yields a DIFFERENT, VALID code pointing at a different glyph; nothing errors, the page just silently says something else. `to_bytes()` therefore lives on the encoder's result type, giving exactly one place in the codebase that has to get this right. A CID above 16 bits is REFUSED, not truncated — same reasoning as the `u32`→`u8` narrowings above, a truncated CID is a different valid CID, not an error sentinel.
- **HEAD (unhashed at filing time — see note below)** — `composite-editable.pdf` fixture (`/Type0`, three CIDs, injective `/ToUnicode`, extracts "ABC") built BEFORE the wiring code that will need it, deliberately, so the fixture is not shaped around what that code happens to do. Then the wiring survey (see below) was written directly into the code as specific, actionable notes — not a bare "TODO: wire it up" — and the session stopped there.

**Why the session stopped here, recorded because it is a decision, not a stall:** wiring `ShowSlot`/`CompositeEncoding` into the actual edit path surveyed as FOUR coupled changes, not one, all touching the shipped in-place-editing path every existing document's edits rely on:
  1. `glyph_names()` returns `None` for a composite font, so the composite branch must be checked and handled BEFORE the existing `Unsupported` bail, not folded into it.
  2. `glyph_advance` currently reads simple-font widths (`/Widths`); composite advances come from `/W`/`/DW` per §9.7.4.3 — a DIFFERENT table, keyed differently, not a wider argument to the same lookup.
  3. `emit_edited_operator` currently writes a literal `( … )` PDF string; a composite run needs a HEX string (`< … >`) with `CompositeEncoding::to_bytes()`'s big-endian pairs inside it — a different operand syntax, not a different byte source into the same syntax.
  4. `carried_codes`' subset-floor accounting assumes single-byte codes; it needs to become width-aware or it will misreport which codes a re-subset must retain.
  A half-applied version of these four is worse than none — an operator could accept an edit that types correctly but writes the wrong operand syntax, or advances glyphs using the wrong table, and nothing would visibly fail. Substrate complete and tested; wiring is the entire remaining scope of Pass 21.1.
  **Discriminator recorded for whoever resumes, needed once slots exist:** the composite-refusal regression test must be rewritten to ask `edit-text` for text that is NOT present on the page. Correct ordering (classify-before-match) still returns the R-INV-4 composite refusal even for absent text, because the refusal is a property of the FONT, never of whether the sought text is findable. Broken ordering (match-before-classify) returns `NoMatch` instead. This discriminator survives the slot-pushing change that disarms the current test's mechanism — see the near-miss note on `31d2fdc`, above.

**RAG escalation, continuation 78:**
- New file: `D:\dev\rag\rust\regression_test_guard_via_incidental_property_disarms_silently.md` — the `composite_refusal_reachable.rs` near-miss, generalized: a regression test that detects a fault via a SECOND, incidental property (here: "composite runs currently produce no slots") silently stops detecting the fault the moment that incidental property changes for an unrelated reason (here: adding slot support for a legitimate feature) — the test keeps passing throughout, so nothing reports the loss of coverage. Fix is to assert the SUBJECT directly (here: that font classification precedes text matching, provable with a search for absent text), not a symptom that happens to correlate with it today. Indexed in `D:\dev\rag\rust\index.md` this continuation.
- No `personal_rag/pdf` entry — nothing PDF-domain empirical this continuation; the `Identity-H` byte-order requirement is canonical §9.7.6.2 content, already the spec-librarian's territory.

### GUI redaction-apply flow — **SHIPPED as Pass 8.1 (`9a68999`), 2026-08-03. See the Pass 8.1 Shipped entry (top of Shipped) for the full build record.** Retained below as the historical framing (append-only discipline).

**Promoted from Backlog the same continuation decision 019/FF-H
completed (Pass 19.4, `a1638f4`).** The GUI can mark redactions and
disclose the mark count (the status bar already warns *"⚠ N UNAPPLIED
redaction mark(s) — this document is NOT redacted"*), but has never had
a way to actually APPLY a redaction — the operation that removes
covered content — from the running application; applying is CLI-only
(`pdfce-cli redact-apply`). `grep -c "apply_redactions\|RedactApply"
crates/pdfce-gui/src/main.rs` returns **0**. The app tells the operator
their document is not redacted and gives them no way to make it so.
`docs/ui_specs/pass-8-redaction.md` §§3–4 specified this GUI flow fully
(including §4.1's deliberately-heavier confirmation convention — this
is the one operation in the app where an extra-heavy confirm step is
the honest design, not friction) and it was never built. Filed as a
Backlog item at Pass 17.1/17.2's ship (2026-08-03) with the structural
note that applying redaction consumes a `Document` and emits a file
directly — it is not an `EditSession` operation, so there is no live
session state for a preview-then-apply surface to render; any GUI apply
flow needs its own design (a distinct confirm/apply modal, most likely,
not a live-preview surface).

**Engineer sequencing call, flagged for operator awareness — not itself
a blocking question.** The engineer dispatched this work AHEAD of item
#4 (form-building tools) in the operator's ★★★ four-item priority
sequence, on the grounds that completing a half-shipped **security**
feature (redaction — the app currently claims a document "is NOT
redacted" with no in-app remedy) outranks starting a new feature family
form-building represents. This is a sequencing judgment call made
without a fresh, explicit operator instruction to reorder — recorded
here, and as new Open operator question (l), so it doesn't read as
silent scope drift if the operator would have preferred item #4 dispatched
next per the standing order. See the Backlog entry (below, now marked
PROMOTED) for the full prior framing.

**RESOLVED by shipping (2026-08-03) — see the Pass 8.1 Shipped entry
(top of Shipped).** The sequencing call itself is still unratified by
the operator (Open operator question (l), below, remains open as a
retrospective flag, not a live blocker), but the work it authorized is
now done. Item #4 (form-building tools) is next per the ★★★ priority
sequence — its Acrobat-parity research is already teed up, see the
updated Forms Backlog entry below.

### ★★★★ HEADLINE FINDING (2026-08-02) — THE GUI NEVER RENDERS UNSAVED EDITS. READ THIS BEFORE TRUSTING ANY "the operator can't verify feature X" report.

Ken ran the GUI and reported it felt non-functional: can't click objects,
a highlighted box that "doesn't correspond to anything," no tab-docking,
no layer tree, and "the dimensioning tool didn't seem to have a way to
actually set the dimensions." **The underlying diagnosis (decision 018,
`docs/decisions/018-edited-state-is-what-the-canvas-renders.md`)
reframes project status: this is ONE shared read-path bug, not fourteen
broken features.**

`OpenDoc::rasterize_current` (`pdfce-gui/src/main.rs:1555`) and
`ensure_object_provider` (`main.rs:1473`) both call
`self.session.document()` — and `edit.rs:962`'s own doc comment says,
verbatim, *"This is the base revision, not the edited state."* Every
editing feature shipped from **Pass 3.1 through Pass 16.2** — dimensions
(12.M2/12.M2b), add-text (16.0–16.2), in-place text edit (14.0–14.4),
reflow (15.0–15.2), markup annotations (6.1/6.2), vector move/delete/
node-drag (9c-min) — writes into `EditSession`'s in-memory overlay, and
the canvas rasterizes and hit-tests only the **base** revision. So every
one of those features is authored correctly and is **invisible and
unclickable in the running GUI.**

This is not a cache-invalidation bug. `refresh_pages` already nulls
`page_texture`/`ThumbnailCache`/`provider_page` on every edit, undo, and
redo — its own doc comment is the fossil: *"the document is not
reloaded, because the base revision ... has not changed."* **True
through Pass 3.1. False since Pass 6.1** introduced staged appearance
streams. The GUI faithfully re-rasterizes every frame and faithfully
reproduces the base. Fix the parameter type (`&Document` →
`&DocumentView`, decision 018) and the pixels change.

**Consequence for how to read every Pass 3.1–16.2 Shipped entry above:**
each one met its stated gates (headless tests, CLI round-trip, R46/R59
oracles) and each one shipped a feature the operator could not see. This
is a **GATE defect** — "done" never required "observed working in the
running application" — not an engineering defect in any of those Passes.

**UPDATE (this session, continuation 56) — Pass 17.0 SHIPPED, the read
path is FIXED (see the Pass 17.0 Shipped entry, top of Shipped above,
committed `3a56b55`).** The canvas now renders `self.session.view()`
(base + staged overlay), not `self.session.document()` (base only) —
every feature from Pass 3.1 through 16.2 is now visible and clickable
in the running GUI, proven by the new `edited_view_is_what_renders.rs`
integration tests and an unchanged-and-green `tools/roundtrip` corpus
sweep (4,023 files) + raster oracle (6566/6566), confirming the
read-path change perturbed no writer behavior. **Pass 17.1** (finish
triaging the remaining `session.document()` call sites — a second,
independent instance of this same bug class was confirmed live on the
redaction-mark-count path, `main.rs:4606`) **and Pass 17.2** (CLI parity
+ headless preview-equals-saved oracle harness) **remain open** — see
the ★ Pass 17.x entry under Next up. Operator confirmed (this session)
that Pass 17 work should land before further new-feature work — see the
★★★★★ reordering entry and the (now-resolved) Open operator questions
item (f), both under Next up.

**Also this session (continuation 56):** a second, independent cause of
"I don't seem to be able to click on objects" was found and fixed — the
Obj (vector-edit) tool drew no selection-outline feedback at all
(`c998521`, Shipped above). Combined with Pass 18.0's tolerance fix and
Pass 17.0's read-path fix, the operator's original complaint is now
**fully explained and fixed on all three contributing causes** — see
the Pass 17.0 Shipped entry's closing diagnosis paragraph for the
headless proof.

**Filed 2026-08-02, all work now COMMITTED on `pass-8-redaction`:** two
architecture decisions (`docs/decisions/017-tabbed-dockable-panel-system.md`,
`docs/decisions/018-edited-state-is-what-the-canvas-renders.md`), a UI
spec (`docs/ui_specs/pass-17-dock-and-layer-tree.md`), and six shipped
Passes/fixes this session (Pass 18.0, Pass 17.0, the GUI observation
harness, the selection-outline fix, Pass 18.2, and the `.gitattributes`
repo-integrity fix — see Shipped above, newest first). The remaining
Pass 17.1/17.2 and Pass 18.1/18.3 work is not yet built — see Next up.
See `SESSION_LOG.md`'s 2026-08-02 entry (continuation 56) for the full
session record. **UPDATE (continuation 57, same day): Pass 18.3 has
since SHIPPED** (`c59b0c4`, ScripTree-style SVG icon set + toolbar
overflow wrapping — see the Pass 18.3 Shipped entry, top of Shipped),
alongside decision 017 AMENDMENT A (`egui_tiles` adopted — see
`ARCHITECTURE.md` §12) and a docs-plus-fix commit `f9bb560`. **Pass
17.1/17.2 and Pass 18.1 remain unbuilt** — see Next up.

**UPDATE (continuation 58, same-day, real date 2026-08-03) — Pass
17.1, Pass 17.2, AND Pass 18.1 have ALL now SHIPPED; decision 018 is
COMPLETE end-to-end; the ★★★★★ REORDERING gate below is now genuinely
CLEARED, not merely deviated around.** Eight more commits landed on top
of `c59b0c4` (continuation-57 HEAD) — see the commit-chain UPDATE
paragraph below for the full list. **The R85 preview-equals-saved
oracle (Pass 17.2) found real, silent data loss on its FIRST run** — see
the Pass 17.1/17.2 Shipped entry (top of Shipped) for the full account:
`flatten_fields` silently discarded every burned-in form value it wrote
(a multi-`ObjectWrite`-per-object-id overwrite bug, now a named
architectural rule at `ARCHITECTURE.md` §11.1); search-redaction could
mark the wrong page after a delete/reorder; session-authored content
could extract as empty. A third, independent root cause of "can't click
objects" was also found and fixed this continuation — a coordinate-
mapping bug in `canvas()`'s use of `ui.centered_and_justified` (see the
Canvas hit-testing Shipped entry) — bringing that operator complaint's
total explained-and-fixed cause count to three. Pass 18.1 shipped the
`egui_tiles` dock + object/layer tree (see its own Shipped entry). The
menu-affordance/glyph-coverage tofu class (flagged open at continuation
57) is also now fully closed. **Remaining open items, none of them
gating the next dispatch:** the GUI has no redaction-apply flow at all
(R85-uncoverable by design, not oracle gap — see the Pass 17.1/17.2
entry); `✓`/`✕` glyph verification on three tools' Accept/Reject
buttons; ui-spec §B.4/§C follow-ons (TextObject/ImageObject core
additions, full selection-legibility asks, a newly-found zero-height-
path selection-outline case) — all filed to Backlog below. See
`SESSION_LOG.md`'s 2026-08-02 entry, same-day continuation 58, for the
full session record.

**UPDATE (continuation 59, same real date 2026-08-03): Pass 18.4
SHIPPED** (selection legibility, ui-spec §C — see its own Shipped entry
above) **and the `ui-strings` CI gate, found red at baseline on 140
hits, was fixed.** §C's full selection-legibility asks are now
delivered end-to-end; §B.4's core `pdfce-core` additions and the
`hit_test_point_all` Alt+click-cycling API remained open at this point.

**UPDATE (continuation 60, same real date 2026-08-03): Pass 18.5
SHIPPED** (`hit_test_point_all` + Alt+click click-through cycling +
text/image object detail — see its own Shipped entry above), **closing
BOTH remaining items from the ui-spec §B.4/§C follow-ons Backlog entry.
All six numbered Pass 18.x slices (18.0–18.5) are now SHIPPED.** The
Pass 18.4 `ApproximateTextBounds` disclosure text, itself found to be
inaccurate (repeating the same wrong bbox model the ui-spec carries),
was also corrected this continuation (`d296666`). The one item left
open from the whole Pass 18.x / decision-017 family is the ui-spec's
own text-bbox-model wording (§0.2/§B.3) — now IN PROGRESS, not merely
filed: `pdfce-ui-specialist` has written the corrected geometry spec
(ui-spec §E) and a builder is implementing the underlying hit-target
fix. See `SESSION_LOG.md`'s 2026-08-03 entry, same-day continuation 60,
for the full session record.

**UPDATE (continuation 61, same real date 2026-08-03): Pass 18.6
SHIPPED** (text hit-target geometry now derived from font metrics — see
its own Shipped entry, top of Shipped). **This closes the fourth and
last named contributing cause of the operator's original "can't click
on objects" complaint** — all four (Pass 18.0's zoom-inverted tolerance,
the Obj tool's missing selection outline, the page-centring coordinate
offset, and the origin-inflated text bbox) are now fixed, not merely
explained. All six numbered Pass 18.x slices plus this follow-on fix are
now SHIPPED; the only remaining loose end is reconciling
`docs/ui_specs/pass-17-dock-and-layer-tree.md` §0.2/§B.3's own wording
against its own already-written §E — `pdfce-ui-specialist` territory,
not built by this fix. See `SESSION_LOG.md`'s 2026-08-03 entry, same-day
continuation 61, for the full session record.


ONLY.** The operator authorized "commit all work"; the engineer
committed the entire working tree as **`d8b3903`** on branch
**`pass-8-redaction`** (373 files changed, 168,217 insertions) on top
of the 2026-07-23 bootstrap commit `67967b2`. This is a **local commit
only — NOT pushed to any remote.** **`LEGAL.md` §1 (OSS license
choice) is now DECIDED — MIT (2026-08-01)** — see `LICENSE` (repo
root) + `Cargo.toml` `[workspace.package] license = "MIT"` +
`license.workspace = true` on all four member crates. Project rule 8's
license precondition for a public-facing commit posture is therefore
now **satisfied**, but pushing is still a **separate, not-yet-granted
authorization**: the operator asked for the license decision and the
new work focus (below), not a push. Do not push without an explicit
go-ahead. The working tree is now clean (nothing uncommitted except
gitignored build/scratch/corpus artifacts); every "Passes 0–N ALL
uncommitted in git" note embedded in the older Shipped entries below
(Pass 2.2 through Pass 8.0) describes the true state **at the time
each of those Passes shipped** and is left as-is (append-only
history) — as of this commit, none of it is uncommitted any longer.
Full record: `docs/SESSION_LOG.md`, same-day continuation 49
(2026-08-01) for the commit, continuation 50 for the license decision
and new focus. **UPDATE (continuation 51):** a second logical commit,
**`e13f3e6`**, has since landed on top of `79d1c6f` (the MIT-license
artifacts commit) for Pass 9a — see the Pass 9a Shipped entry (above)
for gates/content. Both `79d1c6f` and `e13f3e6` remain **local-only**,
same not-yet-pushed posture as `d8b3903`. The engineer is now
committing shipped work in logical per-Pass/per-decision chunks rather
than one large tree-wide commit, going forward. **UPDATE (continuation
52):** a docs commit **`19ed865`** and a fourth logical commit
**`801a748`** (Pass 12.M1, snapping engine) have since landed, giving
the chain **`d8b3903` → `79d1c6f` (MIT) → `e13f3e6` (Pass 9a) →
`19ed865` (docs) → `801a748` (Pass 12.M1)** — see the Pass 12.M1
Shipped entry (above) for gates/content. **UPDATE (continuation 53):**
a sixth logical commit, **`c7c1744`** (Pass 12.M2, dimensioning + scale/
group + hybrid storage + OCG layer), has since landed on top of
`801a748`, giving the chain **`d8b3903` → `79d1c6f` (MIT) → `e13f3e6`
(Pass 9a) → `19ed865` (docs) → `801a748` (Pass 12.M1) → `c7c1744` (Pass
12.M2)** — see the Pass 12.M2 Shipped entry (above) for gates/content.
**UPDATE (continuation 54):** a docs commit **`6150e1a`** and a seventh
logical commit **`7c93cc3`** (Pass 12.M2b, on-canvas dimension authoring
gesture) have since landed on top of `c7c1744`, giving the chain
**`d8b3903` → `79d1c6f` (MIT) → `e13f3e6` (Pass 9a) → `19ed865` (docs) →
`801a748` (Pass 12.M1) → `c7c1744` (Pass 12.M2) → `6150e1a` (docs) →
`7c93cc3` (Pass 12.M2b)** — see the Pass 12.M2b Shipped entry (top of
Shipped, above) for gates/content. All eight commits remain
**local-only**; push authorization is still a separate, not-yet-granted
operator item. **UPDATE (2026-08-01, SESSION_LOG same-day continuation
55):** `2abbd75` (test-hygiene: globally-unique integration-test temp
paths) and `dd3a8b8` (docs: backfill §12 decision-016 entry) landed,
followed by `76485b5` (Pass 9c-min, closing decision 011's beta) and a
docs commit `0569373` (Pass 12.M2b + 9c-min filing) — chain now
**`d8b3903` → `79d1c6f` → `e13f3e6` → `19ed865` → `801a748` →
`c7c1744` → `6150e1a` → `7c93cc3` → `2abbd75` → `dd3a8b8` → `76485b5` →
`0569373`**. **UPDATE (2026-08-02, this session, continuation 56):** six
more commits landed — `9a68d6f` (Pass 18.0, zoom-invariant selection
tolerance + gesture-preserving zoom) → `3a56b55` (Pass 17.0, live-edit
rendering) → `f2d5fae` (GUI observation harness) → `c998521` (selection-
outline feedback, second cause of the click-tracking complaint) →
`dae0139` (Pass 18.2, `object-list` CLI + hit-test query) → `b73604d`
(`.gitattributes` ordering repo-integrity fix). Full chain, 18 commits:
**`d8b3903` → `79d1c6f` → `e13f3e6` → `19ed865` → `801a748` →
`c7c1744` → `6150e1a` → `7c93cc3` → `2abbd75` → `dd3a8b8` → `76485b5` →
`0569373` → `9a68d6f` → `3a56b55` → `f2d5fae` → `c998521` → `dae0139` →
`b73604d`**. All 18 remain **local-only**; push authorization is still
a separate, not-yet-granted operator item. **UPDATE (2026-08-02, this
session, continuation 57):** two more commits landed —
**`f9bb560`** (docs: continuation-56 librarian filing + decision 017
AMENDMENT A + a status notice atop `docs/ui_specs/
pass-17-dock-and-layer-tree.md` + a `tools/roundtrip` determinism fix,
sorting `ObjId`s by key before truncating the R38 promotion-object
sample so it no longer drifts between runs of the same binary — see
`D:\dev\rag\rust\hashmap_iteration_order_drifts_between_runs_of_same_binary.md`)
→ **`c59b0c4`** (Pass 18.3, ScripTree-style SVG icon set + toolbar
overflow wrapping — see Shipped above, top of Shipped). Full chain, 20
commits: **`d8b3903` → `79d1c6f` → `e13f3e6` → `19ed865` → `801a748` →
`c7c1744` → `6150e1a` → `7c93cc3` → `2abbd75` → `dd3a8b8` → `76485b5` →
`0569373` → `9a68d6f` → `3a56b55` → `f2d5fae` → `c998521` → `dae0139`
→ `b73604d` → `f9bb560` → `c59b0c4`**. All 20 remain **local-only**;
push authorization is still a separate, not-yet-granted operator item.
**UPDATE (continuation 58, real date 2026-08-03):** eight more commits
landed — `85a6cac` (docs: `pdfce-ui-specialist`'s menu-affordance-and-
glyph-coverage audit) → `437a6f7` (Pass 17.1 + Pass 17.2) → `a1badc1`
(fix: real chevron + "opens a menu" accessible name for menu-affordance
buttons) → `d15c360` (tools: `observe-gui.ps1` refuses a uniform blank/
black capture) → `eeadbcb` (docs: glyph-fix verified by observation,
second tofu pair found on the rail/Combine-files reorder arrows) →
`f963895` (Pass 18.1, `egui_tiles` dock + object/layer tree) →
`3f6f5ae` (fix: canvas hit-testing was offset by the page-centring
margin) → `869d891` (fix: chevrons for the reorder arrows, closing the
glyph-tofu class). See the five new Shipped entries above (top of
Shipped) for full content. Full chain, 28 commits: **`d8b3903` →
`79d1c6f` → `e13f3e6` → `19ed865` → `801a748` → `c7c1744` → `6150e1a` →
`7c93cc3` → `2abbd75` → `dd3a8b8` → `76485b5` → `0569373` → `9a68d6f` →
`3a56b55` → `f2d5fae` → `c998521` → `dae0139` → `b73604d` → `f9bb560` →
`c59b0c4` → `85a6cac` → `437a6f7` → `a1badc1` → `d15c360` → `eeadbcb` →
`f963895` → `3f6f5ae` → `869d891`**. All 28 remain **local-only**; push
authorization is still a separate, not-yet-granted operator item.
**UPDATE (continuation 59, same real date 2026-08-03):** two more
commits landed on top of `869d891` — **`be62e48`** (Pass 18.4, selection
legibility / ui-spec §C — see its own Shipped entry above; note that
entry's own "Pass-number note" flag: its commit message says "Pass
18.2", but that ID was already taken by the object-list CLI Pass) →
**`a5d1d18`** (the `ui-strings` CI gate was red at baseline on 140 hits
and was hiding a real R1 violation — fixed and moved to
`tools/check-ui-strings.sh`, see its own Shipped entry above). Full
chain, 31 commits from the first implementation commit: **`d8b3903` →
`79d1c6f` → `e13f3e6` → `19ed865` → `801a748` → `c7c1744` → `6150e1a` →
`7c93cc3` → `2abbd75` → `dd3a8b8` → `76485b5` → `0569373` → `9a68d6f` →
`3a56b55` → `f2d5fae` → `c998521` → `dae0139` → `b73604d` → `f9bb560` →
`c59b0c4` → `7274fdd` → `85a6cac` → `437a6f7` → `a1badc1` → `d15c360` →
`eeadbcb` → `f963895` → `3f6f5ae` → `869d891` → `be62e48` → `a5d1d18`**.
Plus `67967b2` (the bootstrap commit, which predates `d8b3903`) for a
branch total of **32**. All of them remain **local-only**; push
authorization is still a separate, not-yet-granted operator item.

**Correction, 2026-08-03 (engineer):** this chain previously listed 30
hashes and claimed 30. It was missing exactly one — **`7274fdd`, the
commit that corrected a fabricated hash in the Pass 18.3 filing**. The
record of repairing a record-keeping error was itself dropped from the
record, which is a tidy demonstration of why the chain is now verified
against `git rev-list` rather than assembled by hand. The counts also
conflated "commits in the chain" with "commits on the branch"; both are
stated explicitly above.
**Repository risk, now more precise than prior entries stated it:**
there is NO git remote configured at all (`git remote -v` empty, no
upstream) — the project's entire history exists solely on this
machine, 32 commits deep. A verified full-history backup bundle was
created as a decision-free stopgap:
`D:\Dev\pdfce-backups\pdfce-20260803.bundle` (3.4 MB; `git bundle
verify` reports a complete history). Also flag for the operator: the
working branch is still named `pass-8-redaction` but now carries
Passes 9 through 18.4, the full icon set, the `egui_tiles` dock shell,
and three independent click-tracking root-cause fixes — worth a rename
whenever a push is authorized.

**UPDATE (continuation 60, real date 2026-08-03):** four more commits
landed on top of `a5d1d18` (continuation-59 HEAD) — **`25b4783`** (docs:
correct the commit chain itself — it had been missing `7274fdd`, the
commit that repairs a fabricated hash in the Pass 18.3 filing, and had
conflated "commits in the chain" with "commits on the branch"; both
fixed and re-verified against `git rev-list`, not re-assembled by hand
— see the correction note above, this is now the record of that
correction being folded back into the librarian-owned chain) →
**`d296666`** (fix: the Pass 18.4 `ApproximateTextBounds` disclosure
text was itself wrong — see the dated correction footer on the Pass
18.4 Shipped entry, above) → **`9998a6b`** (Pass 18.5:
`hit_test_point_all` + Alt+click click-through cycling + text/image
object detail — see its own Shipped entry, above) → **`6a6a48f`**
(tools: the blank-capture guard now samples the CLIENT area, not the
whole window — see the GUI observation-harness Shipped entry, above).
Full chain, **35 commits** from the first implementation commit:
**`d8b3903` → `79d1c6f` → `e13f3e6` → `19ed865` → `801a748` → `c7c1744`
→ `6150e1a` → `7c93cc3` → `2abbd75` → `dd3a8b8` → `76485b5` → `0569373`
→ `9a68d6f` → `3a56b55` → `f2d5fae` → `c998521` → `dae0139` → `b73604d`
→ `f9bb560` → `c59b0c4` → `7274fdd` → `85a6cac` → `437a6f7` → `a1badc1`
→ `d15c360` → `eeadbcb` → `f963895` → `3f6f5ae` → `869d891` → `be62e48`
→ `a5d1d18` → `25b4783` → `d296666` → `9998a6b` → `6a6a48f`**. Plus
`67967b2` (the bootstrap commit, predating `d8b3903`) for a **branch
total of 36**. All 36 remain **local-only**; no git remote is configured
at all; push authorization is still a separate, not-yet-granted
operator item; the verified backup bundle remains
`D:\Dev\pdfce-backups\pdfce-20260803.bundle` (not yet re-generated
against these four new commits — regenerate before treating it as
current). **Standing methodology, now confirmed twice in one project:**
the doc-writing agents (`pdfce-librarian` included) have no shell of
their own — any hash or count handed to them is filed as fact, verbatim,
with no independent means to check it. This is the SECOND filing error
this exact audit habit has caught (the first being `7274fdd` itself,
folded into the continuation-59 filing before this correction). The
standing rule this justifies: **hashes and commit/test counts must be
produced by the engineer directly from `git`/`cargo test` output and
spot-checked (`git cat-file -t`, `git rev-list --count`) after filing,
not assembled from memory or from a prior summary** — see the new
Standing rule R87, below.

**UPDATE (continuation 63, real date 2026-08-03):** three more commits
reported this filing — **`f45d8d6`** (tools: observation scripts refuse
an ambiguous target, `-ProcessId` disambiguation) → **`38fffad`** (Pass
19.0, shared text-state model — see the Pass 19.0 Shipped entry, top of
Shipped) → **`1a2e265`** (docs: decision 019 Amendment A). All three
hashes were engineer-verified via `git cat-file -t` per R87. Branch
`pass-8-redaction`, engineer-reported total **43 commits**, still no git
remote configured. **Arithmetic flag, not silently reconciled:** the
continuation-60 chain (36, including the bootstrap commit) plus
continuation 61's `1b38e34` plus continuation 62's two commits
(`67f49bb`, `743e463`) sums to 39 — matching the "39 commits" figure
recorded at continuation 62 — and **this continuation's three new
commits would bring that to 42, one short of the reported 43.** Per this
project's own standing discipline (R87 — hash/count figures are
produced by the engineer directly from `git`, not assembled from
memory, and are spot-checked after filing), this discrepancy is
recorded rather than quietly absorbed into "43"; whoever next runs
`git rev-list --count HEAD` on this branch should resolve it (either a
fourth commit landed that wasn't reported to this filing, or the
running total drifted by one somewhere in continuations 60–62). The
backup bundle (`D:\Dev\pdfce-backups\pdfce-20260803-0830.bundle`) is
unchanged this continuation and does not yet cover these three commits.

**UPDATE (continuation 64, real date 2026-08-03) — the 39→43
commit-count flag from continuation 63 is RESOLVED, and Pass 19.1
SHIPPED.** Two new commits this filing — **`5c1f5dc`** (docs: the
39→43 arithmetic resolution + backup-bundle regeneration) → **`603b051`**
(Pass 19.1, `Tc`/`Tz`/super-subscript authoring — see the Pass 19.1
Shipped entry, top of Shipped). Both hashes engineer-verified via
`git cat-file -t` per R87. Branch `pass-8-redaction`, **45 commits**
confirmed by `git rev-list --count HEAD` (not merely engineer-reported
this time — an actual recount), still no git remote configured.
**The 39→43 gap was an ANCHORING error, not an arithmetic one:** 39 was
the commit count at `743e463`, but the filing immediately preceding the
"43" figure was actually `0c385a9`, where the count was **40** — so
40 + 3 (`f45d8d6`, `38fffad`, `1a2e265`) = 43, and the flag raised at
continuation 63 was correct to raise even though 43 itself was also
correct. **General lesson kept (not just the corrected number): a
running total is only as good as the anchor it is added to, and "the
last number I filed" is not necessarily "the number at the last commit
I am counting from"** — the remedy is what R87 already prescribes
(produce the figure from `git` at filing time), applied here for real.
This is the THIRD filing-integrity issue this audit habit has caught in
this project (see R87's own note for the first two) — the habit is
earning its keep, keep raising these rather than silently reconciling
them. The stale-bundle flag from continuation 63 is also closed:
regenerated at `D:\Dev\pdfce-backups\pdfce-20260803-1145.bundle`
(verified, covered all 43 commits at the time of its own creation; is
itself now two commits behind again as of this filing — regeneration
is a point-in-time action, not a standing guarantee).

**Pass 19.0 shipped 2026-08-03 (`38fffad`) — see Shipped above; no
longer listed here. Pass 19.1 (`Tc`/`Tz`/super-subscript authoring)
SHIPPED 2026-08-03 (`603b051`) — see Shipped above; no longer listed
here. Pass 19.2 (free-form `Ts` + synthetic bold/italic) SHIPPED
2026-08-03 (`ebe35d8`)** — see the Pass 19.2 Shipped entry (top of
Shipped) for the full slice record, and decision 019 Amendment A
(`1a2e265`). **Decision 019 Amendment B was filed as part of the
continuation-64 librarian pass** (three corrections found while
building 19.1 — mechanism fix, citation-flag closure, R89 base-size
clarification — plus new standing rule R92) and **committed as
`450a44b`** (engineer backfill, 2026-08-03 — the filing correctly
recorded the hash as pending rather than predicting one, which is R87
working as intended: the librarian has no shell and must not invent a
hash it cannot verify).

**UPDATE (continuation 65, real date 2026-08-03) — Pass 19.2 SHIPPED
and decision 019 Amendment C filed as part of THIS librarian pass.**
Amendment C records six corrections found while building 19.2: the
wrong restore set named for stroking colour/line width (they leak into
later stroked *paths*, not just text, if left unrestored); a narrower-
than-written absolute-`Tm`-required-for-followers refusal (deliberately
not converting a producer's `Td`/`T*` to absolute, to avoid a minimal-
diff-violating cascade); a disclosed two-of-three-factor bold-width
formula (no `cm` model in the authoring walk); unanticipated `Tm`/`Tlm`
tracking needed in the authoring walk (neither the original decision
nor Amendment A foresaw this); two named conflicts refused rather than
silently merged (rise vs. superscript/subscript toggle; synthetic
italic vs. `--pin`); and Add-Text synthesis flagged as **not wired**
despite the shared `StyleSynthesis` type and gate existing and being
tested. Full account: the Pass 19.2 Shipped entry (top of Shipped),
`docs/decisions/019-ffh-spacing-scaling-synthetic-styles.md` Amendment
C, and `ARCHITECTURE.md` §5.11/§12. **R88 and R90 amended** (no new
rule number; ceiling remains R92). **Engineer-reported hashes this
continuation, verified by `git cat-file -t`:** `450a44b` (Amendment
B's own filing commit — its hash was correctly recorded as pending at
continuation 64 rather than predicted, and is now confirmed rather
than backfilled-and-guessed), `ebe35d8` (Pass 19.2), `8664912` (a
docs commit recording the `450a44b` confirmation). Branch
`pass-8-redaction`, **48 commits** (`git rev-list --count HEAD`),
all still local-only, no git remote configured. **Slice 19.3 is now IN
DESIGN** — a `pdfce-ui-specialist` dispatch is concurrently writing
`docs/ui_specs/pass-19.3-text-formatting-surface.md` as a new file
only, not touching any file this librarian pass edits.
**RAG escalations filed this continuation:**
`C:\personal_rag\pdf\lesson_20260803_mode2_faux_bold_re_detectable_by_stroke_ratio.md`
and
`D:\dev\rag\rust\prove_test_suite_non_vacuous_by_deliberately_breaking_the_thing_it_tests.md`
— both indexed in their subject's `index.md` this same continuation.

**UPDATE (continuation 66, real date 2026-08-03) — Pass 19.3 SHIPPED
(`74052d3`), closing the FF-H formatting-slice family down to the
conditional Pass 19.4. Only Pass 19.4 (`Tw`, gated behind the census)
remains open in the decision-019 family.** Three engineer-reported
hashes this continuation, verified by `git cat-file -t`: `25b2d0e`
(docs: "file Pass 19.2 and decision-019 Amendment C" — touching
ROADMAP/SESSION_LOG/ARCHITECTURE plus the decision-019 record.
**CONFIRMED by the engineer, 2026-08-03.** The librarian had inferred
this from the established per-continuation pattern and correctly
flagged it as unconfirmed rather than asserting it; the inference was
right. Worth recording that the flag was raised at all — applying R87
to your OWN inference, not only to figures handed to you, is the
harder half of the rule), `e883e26`
(docs: the Pass 19.3 UI spec,
`docs/ui_specs/pass-19.3-text-formatting-surface.md`), `74052d3` (Pass
19.3 SHIPPED — see its own Shipped entry, top of Shipped, for the full
build record). **Headline: the shipped Edit-Text property bar had never
successfully applied a single edit** — `find_anchor`'s pinned-span
match used exact equality against a span convention (`GlyphProvenance::
operator_span`, operator-token-only) that never matched what the
authoring walk's `OpRec` actually published (operand-inclusive) — every
property-bar Apply since Pass 14.3 refused with `NoMatch`, masked by
two independently-wrong doc comments asserting the conventions agreed.
Fixed in the same commit (`pin_names_operator` now accepts either
convention), engineer-verified by mutation (revert → regression test
fails; restore → passes). Third instance this project of a confident,
wrong doc comment being the reason nobody looked (after decision 018's
`refresh_pages` comment and the `.gitattributes` ordering incident) —
recorded as new standing rule **R93** (Standing rules, below; ceiling
was R92). Branch `pass-8-redaction`, **51 commits**
(`git rev-list --count HEAD`), all still local-only, no git remote
configured. **RAG escalation this continuation, filed to
`D:\dev\rag\rust\` (a deliberate deviation from the engineer's
suggested `personal_rag/pdf` location — see the Pass 19.3 Shipped
entry's own RAG-escalation note for the reasoning: the lesson
generalizes to any editor publishing byte spans for later re-location,
not to PDF-domain producer behavior specifically):**
`D:\dev\rag\rust\byte_span_convention_must_live_in_the_type_not_matching_doc_comments.md`
and a companion methodology finding,
`D:\dev\rag\rust\trust_but_verify_doc_comments_are_not_evidence.md`
(the three-instance "confident wrong comment" pattern) — both indexed
in `D:\dev\rag\rust\index.md` this same continuation.

**UPDATE (continuation 67, real date 2026-08-03) — the `Tw` census
(Pass 19.4's gating measurement) has been RUN: BUILD band cleared
(91.6% of show operators / 97.4% of glyphs across 4,012 real PDFs),
but Pass 19.4 itself has NOT started — the engineer paused to fix a
newly-found pdfce defect first.** New out-of-workspace crate
`tools/tw-census` (zero new Cargo dependencies, added to the root
`exclude` list per the established `tools/font-parity`/
`tools/render-parity` convention); two commits, `359d486` and
`5387699`, both verified by `git cat-file -t`. Branch
`pass-8-redaction`, **54 commits** (`git rev-list --count HEAD`),
still no git remote.

**Method** (load-bearing — the number is meaningless without it): unit
of measurement is the **show operator**, keyed by
`(ContentStreamRef, ByteSpan)` from `GlyphProvenance` — the literal
unit decision 019 §3.3 names, deliberately NOT pdfce's `TextRun`
(which splits on geometry/marked-content and would over-report). Keys
pooled per page (a form XObject invoked twice counts once).
Deterministic (sorted path order); the one aggregating `HashMap` is
summed over exhaustively, never sampled — the exact bug class
`D:\dev\rag\rust\hashmap_iteration_order_drifts_between_runs_of_same_binary.md`
warns about. Two full runs produced byte-identical aggregates.
**Ground-truth calibration is a TEST**, not a spot-check — a
known-simple and a known-composite fixture must classify correctly or
the corpus number is meaningless.

Denominators stated exactly, excluding 627 files that would not load
and 2,172 that loaded with zero show operators: **text-bearing
denominator = 1,224 documents / 23,144 show operators / 620,858 shown
character codes.**

| denominator | loose (simple font) | strict (simple AND contains code 32) |
|---|---|---|
| by document (n=1,224) | 86.7% | 43.9% |
| **by show operator (n=23,144)** | **91.6%** | 36.9% |
| by glyph (n=620,858) | **97.4%** | 55.7% |
| median per-document glyph share | 100.0% | 0.0% |

Sub-corpus (loose, by run): pdf20examples 100% · qpdf 99.6% · pdfbox
89.2% · veraPDF 87.6% · pdfium 42.1% (sole outlier, smallest sample,
30 text-bearing docs). Font mix across the 1,224 docs: all-simple 994
(81.2%) · all-composite 163 (13.3%) · mixed 67 (5.5%). Operator
prevalence: `Tc` 19.6% · **`Tw` 10.9%** · `Tz` 1.2% · `TL` 17.6% · `Ts`
0.1% · `Tr` 7.1%.

**VERDICT (R91's decision bands): 91.6% → BUILD (≥60%). Slice 19.4 is
cleared to build**, not marginal — every loose denominator clears 60%,
the median document is 100% simple, and the figure survives the most
adversarial robustness check at 87.3% (removing the four
most-glyph-heavy files).

**The finding that matters more than the verdict: decision 019's
premise 2 is NOT SUPPORTED by this corpus.** The decision partly
justified withholding `Tw` on producers now defaulting to
Type0/Identity-H even for pure-Latin text, "a large and growing
share" — but **81.2% of text-bearing documents contain no composite
run at all.** The census can prove the "large" half wrong; it
**cannot test "growing" at all** — Isartor dates to 2008, qpdf's qtest
files are older, and answering the modern-producer-defaults question
needs a corpus of recently-produced documents (Word/LibreOffice/Chrome
print-to-PDF) that `fixtures/external/` does not contain. **Record
both halves**, not just the falsified one. Corpus-bias caveat, also
load-bearing: this corpus is PDF-tooling test suites (72% veraPDF,
2,053/2,896 loadable files have no text at all) deliberately full of
edge cases, not a random sample of documents an operator would edit;
`pdfbox`'s corpus (real user-submitted bug attachments) is the closest
thing here to real documents and is the MOST favourable to `Tw` (95.9%
loose / 89.7% strict by glyph) — the blended figure UNDER-states it,
strengthening rather than weakening the BUILD reading.

**The strict metric is flagged untrustworthy, not acted on.** It lands
in the escalate band but moves 12 points on the removal of four files
(the top file alone is 18.6% of all glyphs, top 10 are 62%; the three
biggest veraPDF contributors are implementation-limit conformance
probes showing 32k–65k glyphs with ZERO code-32). It is also
asymmetric — an equivalent "has a space" test cannot be applied to
composite runs, since in an Identity-H subset the space is a CID
rather than code 32 (corpus-wide composite code-32 occurrences total
73). Reported as context; **the decision's band is written against
the loose metric, not this one** — Open operator question (g), below,
is closed as moot for this same reason.

**Chain-completeness correction (engineer, 2026-08-03).** The audit that
runs before each of these filings is committed found `fb97abb` — the
continuation-66 filing commit, which recorded Pass 19.3 and standing
rule R93 — absent from every hash reference in `docs/`. Added here.
This is the **second** time the missing commit has been a *filing*
commit rather than a code one (`7274fdd`, itself the fix for a
fabricated hash, was the first), which points at a specific blind spot
rather than bad luck: a continuation records the commits it is filing
ABOUT, and the commit that lands the filing itself has no later entry
to mention it. The audit catches it precisely because it compares
against `git rev-list` rather than against the previous entry.

**Other honest limits recorded:** `/ActualText`-carried text has no
glyph provenance and is invisible to the census — cross-checked
against the independent text-extraction harness at 99.6% agreement on
the text/no-text predicate over 2,892 files, all 11 disagreements
being `/ActualText`/Unicode-CMap conformance probes. Five show
operators had glyphs disagreeing about the composite flag (2 pdfbox, 3
veraPDF) — impossible in principle (one `Tf` governs one show
operator), immaterial to the aggregate, unchased. The "text-free"
bucket conflates genuinely blank pages with content streams that fail
to decode; the tool cannot separate them. **A defect the builder found
and fixed in its own tool, disclosed rather than hidden**: the TSV
header and the failure-row shape were written separately and
disagreed by one tab (all 627 failure rows had 29 fields against a
30-field header); aggregates were unaffected (computed in memory, not
re-parsed from the TSV), and both shapes now derive from one list with
an assertion and two tests — filed as an instance of R92's
duplicated-definition pattern.

**THE MORE VALUABLE FIND — a pdfce defect, engineer-verified: 341
corpus files (8.5%) are unopenable** with *"page /Contents is neither
a stream nor an array of streams"* (226 qpdf, 114 pdfium). Hand-
verified: `fixtures/external/qpdf/qpdf/qtest/qpdf/add-contents.pdf` is
a LEGAL file — `/Contents` is `[ 4 0 R 5 0 R 6 0 R ]`, objects `1 0
obj`–`8 0 obj` are all present, objects 4/5/6 are intact streams with
real text (`(Baked) Tj`, `(Mashed) Tj`) — and pdfce refuses the whole
document, CLI and GUI alike. Two separable problems: (1) rebuild-by-
scan recovery reports `file-level-objects=7` on a file containing 8,
so a `/Contents` element resolves to Null; (2) a single unresolvable
element condemns the ENTIRE document, when ISO 32000-1 §7.3.10 makes a
dangling reference the null object and Table 30 makes `/Contents`
optional ("if this entry is absent, the page shall be empty") — a
fail-clean violation, a damaged part costing the whole file. **A
builder is fixing both now** — instructed to keep the two problems
separate, disclose rather than silently swallow, distinguish "resolved
to null" (degrade the one element) from "genuinely wrong type" (still
an error), and prove newly-opening files have REAL CONTENT rather than
opening as blank pages. **The engineer prioritized this fix above
building slice 19.4** — a control reaching 91% of text matters less
than 341 real files that cannot be opened at all. See the new "★
pdfce defect" entry below for the tracked item.

**RAG escalations filed this continuation:**
`C:\personal_rag\pdf\lesson_20260803_tw_reachability_census_show_operator_91pct.md`
(the reachability finding, with the vintage/corpus-bias caveats
prominent) and
`D:\dev\rag\rust\state_every_denominator_a_census_could_report.md`
(methodology: this census's three denominators — document/operator/
glyph — differ by 11 points; a single headline figure would have been
actionable-looking and wrong) — both indexed in their subject's
`index.md` this same continuation.

### ★ pdfce defect — a single unresolvable `/Contents` array element condemns the WHOLE document (found 2026-08-03 via the `Tw` census corpus sweep; **FIXED 2026-08-03, committed `409a6b5` — see the new Shipped entry at the top of Shipped for the full diagnosis, numbers, and gates**)

**RESOLVED. 289 previously-unopenable documents now read** (`BadContents`
341 → 1, zero regressions). The diagnosis originally filed here
(rebuild-by-scan *missing* an object) was **wrong in mechanism** — the
real cause was an LF-to-CRLF-converted file invalidating every
`/Length` in the stream, dropped only at strict-confirmation, not at
the scan. See the Shipped entry for the corrected mechanism, the two
kept-separate fixes (`StreamLengthPolicy` + per-element `/Contents`
degradation), the round-trip-gate-caught-its-own-bug finding
(`Provenance::RecoveredFile`), and new standing rules R94–R95. This
paragraph is retained as the historical record of what was believed at
the time the fix was dispatched — the "why this took priority" reasoning
below held and does not need correcting.

**Why this took priority over Pass 19.4** (engineer's own call,
recorded so it doesn't read as scope drift): a control reaching 91% of
text-bearing documents matters less than 341 real files — leaning
qpdf/pdfium, closer to organic malformed-in-the-wild files than
veraPDF's deliberately-adversarial conformance probes — that cannot be
opened by pdfce at all. **UPDATE (2026-08-03): Pass 19.4 has since
SHIPPED** (`a1638f4`) — see the Pass 19.4 Shipped entry (top of
Shipped) and the now-retired ★ Pass 19.x entry under Next up. This
whole In-progress entry (the `/Contents` defect and its sequencing
rationale) is fully historical as of this update — nothing from this
thread remains open. **UPDATE (continuation 70, real date 2026-08-03):
the GUI redaction-apply flow has since SHIPPED, as Pass 8.1** (`9a68999`,
engineer-reported hash verified by `git cat-file -t`, alongside
`24bdbc6` also verified this continuation) — see the Pass 8.1 Shipped
entry (top of Shipped). Branch `pass-8-redaction`, **60 commits** as
reported this continuation (the librarian has no shell of its own and
files the count as handed, per the standing methodology noted above;
not independently re-run via `git rev-list --count HEAD`), still no
git remote configured. **Nothing is currently in progress** — the next
dispatch per the ★★★ operator priority sequence is item #4
(form-building tools), whose Acrobat-parity research
(`pdfce-acrobat-librarian`'s form-authoring extension session) is
already teed up — see the updated Forms Backlog entry, below.

**Pass 16.0, Pass 16.1, AND Pass 16.2 all shipped 2026-08-01 — see
Shipped above; no longer listed here. Decision 016 / FF-D (add NEW page
text as real page content) is now COMPLETE end-to-end.** 16.0
(add-new-text engine + point-text insert, core + CLI), 16.1 (boxed add
+ wrap via the 15.x reflow engine, core + CLI), and 16.2 (on-canvas
Add-Text UI — click/drag/type/property-bar/Accept-Reject, plus the
required FreeText-tooltip disambiguation) have all shipped. See the ★
Pass 16.x entry under Next up for the closing amendment. **The
certification-signature-guard gap flagged at 16.0's ship is now CLOSED
(2026-08-01)** — see the "FF-D follow-up hardening" Shipped entry
(top of Shipped, above) and the Backlog entry (marked RESOLVED). No
follow-up remains open from Pass 16.x.

**Pass 14.3 shipped 2026-08-01 — see Shipped above; no longer listed
here. Decision 014 (Acrobat in-place text-editing, Pass 14.0–14.3) is
now COMPLETE end-to-end.** **Pass 14.4 shipped 2026-08-01 — see
Shipped above; no longer listed here.** The four GUI interactions
deferred at Pass 14.3's ship (selection-replace-on-type, triple-click
line-select, drag-select, arrow/Home/End caret navigation) are now all
shipped — the on-canvas text-editing UI's interaction set is COMPLETE,
not just its P0 slice. **Pass 15.2 shipped 2026-08-01 — see Shipped
above; no longer listed here. Decision 015 (FF-A within-block offline
reflow, Pass 15.0–15.2) is now COMPLETE end-to-end** (15.0 read-only
greedy reflow engine + alignment auto-detect → 15.1 reflow-apply
surgery + `CommandKind::ReflowBlock` + CLI `reflow` → 15.2 on-canvas
reflow UI, all three shipped 2026-08-01).

**MILESTONE — FF-A COMPLETE end-to-end, AND the Pass-14.3 GUI-
interaction deferrals are fully discharged (Pass 14.4).** pdfce now
does reviewable, undo-able within-block reflow: greedy re-wrap,
alignment auto-detect/preserve across all four modes (left/center/
right/justified), and working justified alignment (`TJ` slack
distribution) — entirely offline. It also has a complete on-canvas
text-editing interaction set: click-to-caret, Shift-click/drag-select/
triple-click selection, selection-replace-on-type, and full
arrow/Home/End caret navigation. This reaches, and on
justify-reliability/alignment-detection/overflow-honesty exceeds,
Acrobat's own offline reflow (decision 015 §9). Combined, pdfce's
Acrobat text-handling parity is now broad and deep at the P0 level.
**What remains open in the text-parity space (AMENDED 2026-08-01 by
decision 016):** **FF-D is now DECIDED and SCOPED as ★ Pass 16.x** (see
Next up below) — 16.0 (point-text engine + CLI) is the recommended next
build, with its spec grounding being sourced by `pdfce-spec-librarian`
in parallel. Still open, unscheduled: FF-B (cross-block/cross-page
reflow — the genuine exceed-Acrobat headline, Acrobat's own cross-block
reflow is cloud-gated + English-only), FF-H (`Tc`/`Tw`/`Tz`/`Ts`
spacing + synthetic styles), and two items now explicitly recorded
**operator-gated** in Backlog (decision 016 §10, do not schedule
without an operator call): **FF-C** (font subsetting/glyph embedding —
Cargo-dependency copyleft/license gate) and **list-authoring** (scope
call). See the ★ Pass 15.x entry under Next up (below) for the closing
FF-A amendment record, and the ★ Pass 16.x entry for FF-D.

**MILESTONE — FF-D COMPLETE end-to-end (2026-08-01), AND the Acrobat
text-handling parity arc is now COMPLETE at the P0 level.** Pass 16.2
(on-canvas Add-Text UI) was the final slice of decision 016 — 16.0
(point-text engine) → 16.1 (boxed/wrap engine) → 16.2 (canvas UI) are
all shipped. pdfce now adds new page text — point and boxed, bundled
Std-14 no-embed by default, byte-identical-original, immediately
editable/formattable/reflowable through the existing 14.x/15.x
pipeline — reaching Acrobat's Add-Text baseline and exceeding it on
minimal-diff (append-a-stream vs. Acrobat's rewrite), tagged-honesty
(disclosed-untagged vs. silent structure corruption), a first-class
scriptable CLI (Acrobat has none), and a documented default-font policy
(Acrobat's is undocumented) — decision 016 §9's exceed-Acrobat list,
now all delivered. **Broader milestone: with FF-D done, pdfce's Acrobat
text-handling parity arc is complete at the P0 level** — in-place
editing (decision 014, Pass 14.0–14.4), within-block reflow including
justified alignment (decision 015 / FF-A, Pass 15.0–15.2), and
add-new-text (decision 016 / FF-D, Pass 16.0–16.2) are ALL shipped, on
top of the earlier root-cause font fix (SESSION_LOG continuation 33)
and the xref-recovery work (Pass 13.x). **What remains open in the
text-parity space is now a clean decision point, not open engineering
work:** FF-B (cross-block/cross-page reflow — the genuine
exceed-Acrobat headline) and FF-H (`Tc`/`Tw`/`Tz`/`Ts` spacing +
synthetic styles + minimal StructTree update) are lower-priority-
deferred, unscheduled; FF-C (font subsetting/embedding — Cargo-
dependency copyleft/license gate, rule 13) and list-authoring (scope
call) are both explicitly **operator-gated** — see Backlog. No Pass
number is invented for any of these. **The certification-signature-
guard follow-up flagged at Pass 16.0's ship has since SHIPPED
(2026-08-01, see the "FF-D follow-up hardening" Shipped entry, top of
Shipped) and the Backlog entry is marked RESOLVED — this closes the
last known loose thread in the text-parity arc.** See the ★ Pass 16.x
entry (Next up) for the closing amendment record. With FF-C and
list-authoring both operator-gated and everything else either shipped
or deliberately deferred, **the text-parity arc now has no open
engineering work awaiting dispatch** — the next move on this arc is an
operator decision (FF-C unblock, list-authoring scope call), not
further autonomous engineering.

**AMENDMENT (2026-08-01, SESSION_LOG continuation 50) — the awaited
operator move arrived: "finish off all the text handling stuff."**
Combined with the same-continuation MIT license decision (which lifts
FF-C's rule-8/rule-13 license gate — see `LEGAL.md` §1, `ARCHITECTURE.md`
§12), **FF-B, FF-H, and FF-C are now all schedulable** — this directive
is the operator go-ahead the milestone above was waiting on. **List-
authoring is explicitly NOT resolved by this instruction** — "text
handling" and "list authoring" are two different open items in this
file (see Backlog), and the operator did not answer the list-authoring
scope question; do not schedule it without a separate, explicit
answer. Sequencing note: per the ★★★ Operator priority sequence (top
of Next up), this text-handling work is priority **#3**, queued behind
the dimensioning tool (#1) and the icon set (#2) — do not jump the
queue.

### Beta — Scaled measurement / dimensioning tool (decision 011 ARCHIVED; NOW ACTIVE — operator go-ahead received 2026-08-01)

**Promoted to In progress 2026-08-01 by operator REPRIORITIZATION;
GO-AHEAD RECEIVED same day (2026-08-01, SESSION_LOG continuation 50).**
Ken's directed instruction — "get the dimensioning tool completely
functional in the gui interface" — is the **#1 item in a four-item
priority sequence** he set this continuation (icons → text-handling
completion → form-building tools follow it; see the three new/updated
entries below and under Backlog). This is now the project's **ACTIVE
engineering focus**, superseding the prior "awaits operator go-ahead"
posture.

Its architecture is DECIDED and ARCHIVED at
`docs/decisions/011-first-beta-scaled-measurement-dimensioning-tool.md`
(five slices — **12.0 / 9a / 12.M1 / 12.M2 / 9c-min**). **Do NOT invent the
beta's Pass IDs / slices here — decision 011 defines them.** (The
"12.M2b" split below is an engineer-assigned Pass ID for a scope split
within slice 4/5, not a librarian- or engineer-invented resequencing of
decision 011's own architecture — see the Pass 12.M2 Shipped entry's
judgment call 1.)

**Current state (2026-08-01): slices 1–5 of decision 011's originally-
named five are shipped; only 9c-min remains.** Pass **12.0**
(canvas-interaction substrate), Pass **9a** (read-only vector
object/selection model + centerline), Pass **12.M1** (snapping engine +
fuzzy snap indicator), Pass **12.M2** (dimensioning + scale/group +
hybrid storage + OCG layer — the headline capability), and now Pass
**12.M2b** (on-canvas dimension authoring gesture) have all shipped —
see Shipped above. **Dimensions are now fully authorable BOTH via the
CLI (`dimension-add`/`dimension-list`/`group-add`/`group-set-scale`/
`layer-toggle`) AND on the canvas** (click-point-A-then-click-point-B
for linear, toggle pick-set + live fit for radius/diameter, reference-
line pick + dialog for scale, plus a dimension-groups management
panel) — see the Pass 12.M2b Shipped entry (top of Shipped) for the
full gesture set and the canvas==CLI equivalence proof.
**MILESTONE — the operator's #1 directed priority, "get the
dimensioning tool completely functional in the gui interface," is now
SUBSTANTIALLY MET** (see the Pass 12.M2b Shipped entry's milestone
paragraph for the full record). **9c-min** (basic vector editing:
move/delete/drag-node — the R59/Pass-11-gated surgery slice, a distinct
capability: editing EXISTING vector objects rather than authoring
dimensions) is the only decision-011 beta slice remaining, and is now
**IN PROGRESS**.

**IN PROGRESS NOW (2026-08-02) — CORRECTION: nothing from decision
011's beta remains in progress; it is COMPLETE.** `9c-min` SHIPPED and
COMMITTED as `76485b5` (2026-08-01; see Shipped above) — the "9c-min
in progress" status previously recorded in this section was never
updated after that ship. Corrected here. All six slices of decision
011's beta are shipped: `12.0 → 9a → 12.M1 → 12.M2 → 12.M2b → 9c-min`.
**What IS in progress now:** the two threads opened 2026-08-02 —
**decision 018** (Pass 17.x, live-edit rendering) and **decision 017**
(Pass 18.x, tabbed dock + layer tree). **UPDATE (continuation 56): both
threads have shipped further slices this session.** Decision 018's
**Pass 17.0** (the core+render generalization + the canvas read-path
fix itself) is now SHIPPED — see the Pass 17.0 Shipped entry (top of
Shipped) and the ★★★★ HEADLINE FINDING's update above; **17.1**
(finish the `session.document()` audit) and **17.2** (CLI parity +
headless oracle) remain. Decision 017's Pass 18.0 (zoom tolerance,
committed `9a68d6f`) and **Pass 18.2** (`object-list` CLI, committed
`dae0139`) are SHIPPED; a related fix (selection-outline feedback for
the Obj tool, `c998521`) also shipped this session — see Shipped above.
**UPDATE (continuation 57): Pass 18.3** (Measure ▾ affordance fix +
icon set + toolbar wrapping) has since SHIPPED (`c59b0c4`) — see the
Pass 18.3 Shipped entry (top of Shipped). **Pass 18.1** (tabbed/panel
shell + Objects tree + Properties panel, now built atop `egui_tiles`
per decision 017 Amendment A) remains unbuilt — see the ★ Pass 17.x /
★ Pass 18.x entries under Next up for what's left.
**UPDATE (continuation 58, real date 2026-08-03): both threads are now
FULLY SHIPPED.** Decision 018's **Pass 17.1** (finished the
`session.document()` audit; found and fixed two further silent-
correctness bugs plus a third, distinct `ObjectWrite`-overwrite bug in
`flatten_fields`) and **Pass 17.2** (R85 preview-equals-saved oracle,
11/12 operations) both SHIPPED `437a6f7` — **decision 018 is COMPLETE
end-to-end.** Decision 017's **Pass 18.1** (the `egui_tiles` dock shell
+ object/layer tree) SHIPPED `f963895` — **all four numbered Pass 18.x
engineering slices (18.0/18.1/18.2/18.3) are now shipped**, though the
ui-spec's §B.4 (core data-model additions) and §C (full selection-
legibility asks) were NOT fully delivered as part of 18.1 and remain
open follow-on work (see Backlog: "ui-spec §B.4/§C follow-ons"). A
third, independent root cause of "can't click objects" (a canvas
coordinate-mapping bug) was also found and fixed this continuation. See
the five new Shipped entries (top of Shipped) for full content; the ★
Pass 17.x entry below is now retired (fully shipped) and the ★ Pass
18.x entry below is updated in place.
- **Marquee-vs-pan UX flag (owed since Pass 9a) — RESOLVED, KEPT, no
  further action.** `pdfce-ui-specialist` reviewed it during the
  12.M1 dispatch and found no conflict with dimension-picking (which
  is click-A-then-click-B, not drag) — see the Pass 12.M1 Shipped
  entry above for the full record. This flag is now closed.

**Icon design (operator priority #2) is COMPLETE, and its BUILD is now
UNBLOCKED (both gated decisions RESOLVED 2026-08-01, SESSION_LOG
continuation 54) — still queued behind 9c-min per the operator's
sequence, not yet dispatched.** `docs/ui_specs/icon-set-and-toolbar.md`
(authored by `pdfce-ui-specialist`) maps all 27 current/near-term GUI
controls to a ScripTree-styled icon (reuse or new-draw), including a
deliberate **solid-filled exception for redaction's icon** (the one
place in an otherwise all-outline set where a solid mark is the honest
depiction of what redaction actually does). The two decisions
previously gating the BUILD's scoping are now both answered by the
operator — see the "★ Icon set" entry under Next up (below) for the
full resolved record:
(a) **SVG rendering: PRE-RASTERIZE to PNG at build time** — no new
Cargo dependency; the runtime `resvg`/`usvg`-style crate alternative
was explicitly rejected.
(b) **Provenance/style: USE the ScripTree SVGs (operator's own) where
they fit; CREATE new icons in that same flat style when needed; make
new icons resemble Inkscape/Adobe VISUAL CONVENTIONS for equivalent
commands (the recognizable metaphor — e.g. a hand for pan, a magnifier
for zoom) WITHOUT copying their actual icon artwork** (no copyright
issues, since the resemblance is at the metaphor level, not the asset
level).

**READY-TO-START status (2026-08-01, unchanged):** the beta's research
prerequisites (spec slices, Acrobat measuring-tools bucket, Inkscape
selection+snapping bucket) were already sourced before this
continuation — that is why 9a could be dispatched immediately on the
operator's go-ahead rather than waiting on a fresh research round.

**GUI-polish interlude (2026-08-01, does NOT displace the beta):** an
operator-requested GUI polish + launcher interlude shipped ahead of the
beta (see Shipped above) — it added no document capability, only usability
on the current feature set plus `pdfce.bat` / `pdfce.ps1` launchers. The
operator is now **actively / interactively using the GUI** (the `/loop`
autonomous mode is stopped; work is interactive from here).

**What the beta PULLS FORWARD (from the decision-010 C → B → A sequence):**
decision 010's **Pass 12** (canvas-interaction foundation / candidate B)
and the **first slices of Pass 9** (vector editing / candidate A), plus a
NEW dimensioning subsystem layered on top. The mechanism is **decision
010 revisit-trigger 3** (operator wants vector editing sooner, accepting
that visual correctness is now corpus-*measured* by Pass 11 rather than
merely spot-checked — the fuzzy-never-sneaky scheduling posture).

**Sequencing — decision 010's C → B → A CONTINUES after the beta.** Pass
11 (candidate C, render-fidelity verification) is now **SHIPPED** (see
Shipped above), so the render is VERIFIED for the editing work the beta
begins — exactly the ground decision 010 required before A. After the
beta, the remaining C → B → A work proceeds: Pass 12 (the full
canvas-interaction foundation + the three deferred editing-GUI slices) →
Pass 9 (full vector/Inkscape parity). The beta does not cancel that
sequence; it front-loads a usable vertical slice of it around a new
dimensioning capability.

**Carried Pass-5 reconciliation (append-only pointer — decisions
007/008/010):** Pass 5 (Encryption) is decision 010's candidate **D** —
it stays on the **fallback/interleave** track, NOT the In-progress slot
and NOT next. It retains its decision-007 ID (never renumbered). It is
spec-unblocked (continuation-22 §7.6 corpus session) but queue-deferred;
its `/R 6` open sub-decision and the two operator decisions (LEGAL.md §2
Adobe-supplement contradiction; `/R 6` sourcing method) gate its scoping
when it activates. Scope, constraints, and the 0.67% `/Encrypt` census
(92.5% legacy R≤4, promotion trigger NOT met) are unchanged — recorded
at the Encryption Backlog bucket and in SESSION_LOG continuations 20 and
22. Only its queue position changed.

## Next up

### ★★★ OPERATOR PRIORITY SEQUENCE (set 2026-08-01, SESSION_LOG continuation 50) — READ THIS FIRST

Ken's directive, same continuation as the MIT license decision: **"get
the dimensioning tool completely functional in the gui interface. add
d:/dev/scriptree style icons for all gui features. finish off all the
text handling stuff. work on form building tools after if that makes
sense."** This is a four-item priority order, superseding whatever
"next" framing any older entry below implies. Do not resequence
without a new operator instruction.

1. **Dimensioning tool → completely functional in the GUI.**
   SUBSTANTIALLY MET (2026-08-01) — see the Beta entry under "In
   progress" (above) and the Pass 12.M2b Shipped entry's milestone
   paragraph for the full record (12.0, 9a, 12.M1, 12.M2, AND 12.M2b
   all shipped — dimensions fully authorable both via CLI and on the
   canvas, disclosed in GUI). **9c-min** (basic vector editing, a
   distinct capability) is the only decision-011 slice remaining and
   is now IN PROGRESS.
2. **ScripTree-style icons for all GUI features.** **SHIPPED
   2026-08-02** (`c59b0c4`, Pass 18.3) — see the Pass 18.3 Shipped
   entry (top of Shipped) and the "Icon set" entry below (now carrying
   a SHIPPED banner). Shipped AHEAD of the ★★★★★ REORDERING's stated
   gate (Pass 17.1/17.2 still unbuilt at the time) — see that entry's
   "DEVIATION RECORDED" note. **Pass 17.1/17.2 have since shipped
   2026-08-03 (continuation 58)** — the gate this deviation stepped
   ahead of is now satisfied retroactively; the deviation itself stays
   recorded as history (not retracted), but nothing about it needs
   operator resolution to unblock items 3/4 below any longer.
3. **Finish all text-handling.** FF-B (cross-block/cross-page reflow),
   FF-H (`Tc`/`Tw`/`Tz`/`Ts` spacing + synthetic styles + minimal
   StructTree update), and FF-C (font subsetting/glyph embedding — its
   license/rule-8 gate is now LIFTED by the MIT decision) are all now
   schedulable per this directive — see the ★ Pass 16.x entry's Backlog
   pointers (FF-C bullet, amended) for detail. **List-authoring remains
   a separate, still-unanswered scope question** (see Backlog) — this
   directive does not resolve it; do not fold list-authoring into
   "text-handling" without a further, explicit operator answer to that
   specific question.
   **UPDATE (2026-08-03) — FF-H is now DONE.** Pass 19.4 (`Tw`) SHIPPED
   (`a1638f4`), closing decision 019 / FF-H end-to-end (all five slices
   19.0–19.4) — see the Pass 19.4 Shipped entry (top of Shipped) and the
   ★ Pass 19.x entry below. **Item 3 is therefore PARTIALLY DONE, not
   fully DONE**: FF-H's own scope is complete; FF-C and FF-C's rule-13
   dependency-classification gate (Open operator question (h)) and FF-B
   remain unscheduled, per decision 019's own Q3 build order (FF-H → FF-C
   → FF-B) — do not treat "text-handling" as closed until those two ship
   or are explicitly deferred by the operator.
   **Namesake-collision note, not to be mistaken for progress on FF-H
   (added 2026-08-03, Pass 18.6):** Pass 18.6 added `Tc`/`Tw`/`Tz`/`Ts`
   tracking to `pdfce-core`'s vector-DECOMPOSE walk's `GState`
   (`crates/pdfce-core/src/vector/decompose.rs`) — a bug fix for
   accurately measuring the bbox of EXISTING content during hit-testing,
   nothing to do with authoring. FF-H is about the add-new-text/
   in-place-text-edit authoring engines APPLYING these parameters when
   writing new content. Same four operator names, two unrelated code
   paths — FF-H's own scope is UNCHANGED and NOT advanced by Pass 18.6.
   **AMENDMENT (2026-08-03, decision 019) — item 3 now has a concrete
   scoping for FF-H.** FF-H is DECIDED and sliced as ★ Pass 19.x (above):
   `Tc`/`Tz` + super/subscript ship as parity, free-form `Ts` + synthetic
   bold/italic ship as a deliberate exceed, `Tw` is evidence-gated
   (corpus census), and the minimal StructTree/`/ActualText` piece is
   CUT from FF-H entirely (re-filed as FF-I, Backlog). Build order is
   FF-H → FF-C → FF-B (★ Pass 19.x's Q3) — FF-B and FF-C remain
   unscheduled/gated exactly as before, unchanged by this amendment
   beyond the ordering call. Pass 19.0 (correctness consolidation) is
   IN PROGRESS. Full record:
   `docs/decisions/019-ffh-spacing-scaling-synthetic-styles.md`.
4. **Form-building tools, after — "if that makes sense."** Queued
   behind items 1–3. This is form field CREATION/authoring (adding new
   AcroForm fields to a document), distinct from the already-shipped
   Pass 7.0/7.1 form FILL/flatten subsystem — see the "Forms (AcroForm)"
   Backlog bucket (amended) for detail. The "if that makes sense"
   qualifier is the operator's own hedge — re-evaluate scope/priority
   against whatever's shipped by the time items 1–3 are done, don't
   treat it as an unconditional commitment.

### ★★★★★ REORDERING — CONFIRMED by the operator 2026-08-02 (continuation 56), now supersedes the ★★★ priority sequence below until Pass 17 finishes — GATE CLEARED 2026-08-03 (continuation 58)

**GATE CLEARED (continuation 58, 2026-08-03) — Pass 17.1 AND Pass 17.2
have both SHIPPED (`437a6f7`); decision 018 is COMPLETE end-to-end.**
This entry's stated condition ("do not start the icon build,
text-handling fast-follows, or form-building until 17.1/17.2 are also
shipped") is now genuinely satisfied — not merely deviated around. The
icon build (item #2) already shipped ahead of this gate (see the
DEVIATION RECORDED note below, kept as history, not retracted); items
#3 (text-handling fast-follows) and #4 (form-building) were never
started and are now unblocked for real, no further operator sign-off
needed on this specific gate. See the Pass 17.1/17.2 Shipped entry (top
of Shipped) for what those two slices found and fixed — including real,
previously-silent data loss in `flatten_fields` — before treating "the
gate is clear" as "nothing more to check" for related work.

**RESOLVED — was "proposed, awaiting operator answer" earlier this
session; the operator confirmed it this session** (see "Operator
answers to record" and Open operator questions item (f), now marked
resolved). **Pass 17.x (live-edit rendering, decision 018) lands before
any further item in the ★★★ priority sequence below** — the icon set
(#2), text-handling fast-follows (#3), and form-building (#4) all wait
on Pass 17.x finishing. **Pass 17.0 itself is now SHIPPED** (top of
Shipped above, `3a56b55`) — what's left gating the rest of the sequence
is **Pass 17.1** (finish the `session.document()` audit) and **Pass
17.2** (CLI parity + headless preview-equals-saved oracle), both still
open. Do not start the icon build, text-handling fast-follows, or
form-building until 17.1/17.2 are also shipped.

**DEVIATION RECORDED (continuation 57, 2026-08-02) — this directive was
NOT observed for the icon build.** Pass 18.3 (ScripTree-style icon set
+ toolbar overflow wrapping + Measure ▾ affordance fix) SHIPPED
(`c59b0c4`) this session, WITHOUT waiting for Pass 17.1/17.2, which
remain unbuilt as of this entry. Flagged here explicitly, not silently
folded in, per the same "don't let a contradiction stand unrecorded"
discipline this file uses for its own decision history. The engineer's
rationale (from the commit sequence, not a re-litigated operator
answer): Pass 18.3 was scoped and dispatched together with Pass 18.0/
18.2's discoverability fixes for the SAME operator complaint ("I don't
seem to be able to click on objects" / the dimensioning tool "didn't
seem to have a way to set dimensions"), and the toolbar-wrapping fix in
particular was a direct, verified fix for that complaint (see the Pass
18.3 Shipped entry's ENGINEER-VERIFIED OBSERVATION). Whether this
justifies stepping ahead of the confirmed sequence is a judgment call
the operator has not yet been asked to bless — **flag to the operator
at next contact**, do not treat it as retroactively authorized. Pass
17.1/17.2 remain the gating items for text-handling fast-follows (#3)
and form-building (#4), which have NOT been started.

### ★ Pass 17.x — Live-edit rendering: the canvas renders the edited document (decision 018 — RETIRED, all 3 slices SHIPPED, decision 018 COMPLETE 2026-08-03)

**All three librarian-assigned slices are now SHIPPED — this entry is
retired; nothing here is still to build.** Full architecture:
`docs/decisions/018-edited-state-is-what-the-canvas-renders.md`.

- **Pass 17.0** — "the canvas renders the edited document." SHIPPED
  2026-08-02, `3a56b55`. See the Pass 17.0 Shipped entry for the full
  build record (two implementation deviations, the confirmed-real §10
  hazard-2 bug fixed in the same Pass).
- **Pass 17.1** — finished the `session.document()` audit. SHIPPED
  2026-08-03, `437a6f7`. Fixed three remaining base-read sites
  (`count_redaction_marks`, `need_appearances`, `page_font_entries`)
  plus two further silent-correctness bugs of a different kind (search-
  redaction resolving through the wrong page index; session-authored
  content extracting as empty) — see the Pass 17.1/17.2 Shipped entry
  (top of Shipped) for the full account.
- **Pass 17.2** — R85 preview-equals-saved oracle harness. SHIPPED
  2026-08-03, `437a6f7` (same commit as 17.1). Covers 11/12 named R85
  operations; `redact-apply` is structurally uncoverable (see the
  Shipped entry). **Found real, silent data loss on its first run** — a
  multi-`ObjectWrite`-per-object-id overwrite bug in `flatten_fields`
  that silently discarded every burned-in form value — now a named
  architectural rule (`ARCHITECTURE.md` §11.1).

**Two operator decisions this Pass was gated on (decision 018 §11):**
(a) adopt the operator-visible definition of done (provisionally R86)
as a standing rule — **still PENDING, not answered — see Open operator
questions item (e); R86 remains proposed-not-in-force**, and is now
arguably strengthened by this session's own findings (R85's headless
oracle caught bugs R86-style manual observation might also have caught,
which cuts both ways as an argument — record the fact, not a
recommendation on how the operator should weigh it). (b) sequence Pass
17 before new feature work — RESOLVED (continuation 56): confirmed yes,
and the sequence has since fully discharged (continuation 58) — see the
★★★★★ reordering entry above.

### ★ Pass 18.x — Tabbed dock, layer/object tree, selection legibility, Measure ▾ affordance fix (decision 017 + Amendment A + `docs/ui_specs/pass-17-dock-and-layer-tree.md`, ALL 6 numbered slices 18.0–18.5 PLUS the follow-on Pass 18.6 text-bbox-geometry fix SHIPPED 2026-08-03 — only the ui-spec's own §0.2/§B.3 wording reconciliation remains open, `pdfce-ui-specialist` territory, see Backlog)

**Pass-number note (same pattern as decision 014's Pass 13→14
renumber — see that entry above, "PASS-NUMBER RENUMBER"):** the UI
spec's own filename, `docs/ui_specs/pass-17-dock-and-layer-tree.md`,
predates decision 018 claiming Pass 17 for the live-edit-rendering fix.
**Pass 17 is decision 018's; this whole family is renumbered to Pass
18.x.** Do not rename the spec file for this alone — the renumbering
note here is the canonical record, same convention decision 014's
archived record uses for its own superseded "13.x" self-reference.

- **Pass 18.0 — zoom-invariant selection tolerance + gesture-preserving
  zoom.** **SHIPPED 2026-08-02, committed `9a68d6f` — see Shipped
  above.** The ui-spec's own §0.1 P0 item, pulled forward and shipped
  standalone. **Related, separately-shipped fix, same session:** the Obj
  tool's missing selection-outline feedback was a SECOND, independent
  cause of the same operator complaint — fixed as its own commit
  (`c998521`, Shipped above), not part of this slice's scope but closing
  the same complaint together with it.
- **Pass 18.1 — tabbed/panel dock shell + Objects tree + Properties
  selection panel + canvas selection feedback. SHIPPED 2026-08-03,
  committed `f963895` — see the Pass 18.1 Shipped entry (top of
  Shipped) for the full build record.** Built on `egui_tiles` 0.16.0
  per decision 017 AMENDMENT A (superseding both the ui-spec's original
  §A horizontal-tab-strip design AND decision 017's own hand-rolled
  two-compartment design — see `ARCHITECTURE.md` §12 continuation-57
  entry for that history). `enum DockPanel` + one `panel_body(...)`
  dispatcher (decision 017 §8.1) survives verbatim as the `egui_tiles`
  pane payload; the underlying simultaneity requirement (Layers/Objects
  and Properties visible together) ships as the DEFAULT vertical-split
  layout. The ui-spec's §D (Measure ▾ affordance fix) shipped separately
  as Pass 18.3 (below).
  **DEVIATION FROM THIS BULLET'S OWN "BINDING" FRAMING, flagged not
  silently dropped:** the ui-spec's §A.2 must-ship-together rename
  (`properties_window` → "Document Properties" in the same slice) and
  §B.4's "binding core asks" (`TextObject` extracted-string preview +
  resolved font-name/size; `ImageObject` pixel width/height, both
  zero-GUI-dependency additions to `pdfce-core`) plus §C's full
  selection-legibility asks (type badge, invisible/approximate-hit
  disclosure, status readout) were **NOT** all delivered as part of
  this Pass. Consolidated as an explicit Backlog follow-on below
  ("ui-spec §B.4/§C follow-ons") rather than left to be rediscovered —
  do not assume this Pass closed §B/§C in full just because it shipped
  the dock shell and object tree. **UPDATE — Pass 18.4 (below) has since
  delivered §C's full selection-legibility asks** (type badge, invisible/
  approximate-hit disclosure, status readout) **end-to-end.** **FURTHER
  UPDATE — Pass 18.5 (below) has since delivered §B.4's core additions
  to `pdfce-core` as well** (`TextObject` string/font preview,
  `ImageObject` pixel dimensions) — see that entry and the Backlog entry
  (amended) for the full record. Nothing from §B/§C remains undelivered
  except the ui-spec's own text-bbox-model wording, tracked separately
  below (Pass 18.4's finding 1 / the Backlog item's item 1).
- **Pass 18.4 — Selection legibility (ui-spec §C). SHIPPED 2026-08-03,
  committed `be62e48` — see the Pass 18.4 Shipped entry (above) for the
  full build record, including its own "Pass-number note" flag (the
  commit message calls itself "Pass 18.2," an ID already taken — this
  roadmap entry is the canonical record, filed as 18.4).** Delivers a
  new `object_summary.rs` fact-record module shared verbatim by the
  Objects tree row and the canvas status readout (structurally
  guaranteed not to disagree, by a test), a per-kind selection-outline
  treatment (solid vs. dashed, letter badge, degenerate-rect inflation),
  and a one-line-plus-`CollapsingHeader` status readout. **Engineer-
  verified on screen (R86):** the operator's original "box that doesn't
  correspond to anything" complaint reproduces as a real, now-explained
  case — an approximate text bbox that is narrower than and offset from
  the glyph ink it describes (see the Backlog entry's ui-spec-correction
  item). Deferred, not built at THIS Pass: Alt+click cycling through
  overlapping objects (needed a new `pdfce-core` `hit_test_point_all`
  API) — **SHIPPED since, as Pass 18.5 (below).**
- **Pass 18.5 — `hit_test_point_all` + Alt+click click-through cycling;
  text/image object detail in decomposition. SHIPPED 2026-08-03,
  committed `9998a6b` — see the Pass 18.5 Shipped entry (top of Shipped)
  for the full build record.** Closes BOTH items Pass 18.4 deferred:
  Alt+click cycling through overlapping objects (a new
  `hit_test_point_all` core API, with `hit_test_point` now structurally
  defined as its head so the two provably cannot disagree) and §B.4's
  core-data-model additions (`TextObject` string/font preview via a new
  `FontResolver` seam, `ImageObject` pixel dimensions). Also fixes the
  Pass 18.4 `ApproximateTextBounds` disclosure text (`d296666`, same
  continuation — see the Pass 18.4 Shipped entry's dated correction
  footer), which had repeated the same wrong bbox model the ui-spec
  itself carries. **All numbered Pass 18.x slices are now SHIPPED.** The
  one item still open from this whole family is the ui-spec's own
  text-bbox-model wording correction (§0.2/§B.3) — now IN PROGRESS, not
  merely filed (`pdfce-ui-specialist` has written the corrected
  geometry spec as ui-spec §E; a builder is implementing the underlying
  hit-target fix) — see the Backlog entry (amended) for status.
  **UPDATE — SHIPPED since, as Pass 18.6 (below).**
- **Pass 18.6 — text hit-target geometry now derived from font metrics,
  not glyph-origin inflation (ui-spec §E). SHIPPED 2026-08-03, committed
  `1b38e34` — see the Pass 18.6 Shipped entry (top of Shipped) for the
  full build record.** Fixes the geometry Pass 18.4 only diagnosed and
  disclosed: `TextObject`'s bbox is now the summed advance widths across
  the run plus `/FontDescriptor` ascent/descent, via a four-rung metrics
  ladder, reusing `text_extract`'s existing font resolver (no new
  dependency). **This is the fourth and last named contributing cause of
  the operator's original "can't click on objects" complaint, now fixed**
  — alongside Pass 18.0's tolerance fix, the Obj tool's selection-outline
  fix, and the page-centring coordinate-offset fix. The only remaining
  loose end from the whole Pass 18.x / decision-017 family is reconciling
  the ui-spec's own §0.2/§B.3 wording against its already-written §E —
  `pdfce-ui-specialist` territory, not built by this fix.
- **Pass 18.2 — `object-list` CLI subcommand. SHIPPED 2026-08-02,
  committed `dae0139` — see Shipped above.** Closed the gap found this
  session: `object-move`'s own help text told operators to get object
  indices from `object-list` — no such command existed. Also shipped a
  `--hit`/`--tolerance` headless hit-test query beyond the original
  scope, used to produce the diagnosis in the Pass 17.0 Shipped entry.
  Serves as the Objects tree's headless companion (rule 11, CLI
  parity) for Pass 18.1 (now shipped, above) — the Pass 18.1 Shipped
  entry's object-tree regression test pins agreement against this
  command directly.
- **Pass 18.3 — Measure ▾ affordance fix (ui-spec §D) + ScripTree-style
  icon set + toolbar overflow wrapping. SHIPPED 2026-08-02, committed
  `c59b0c4` — see Shipped above (top of Shipped).** The dimensioning
  tool was already confirmed FUNCTIONAL end-to-end (`run_measure_tool`/
  `scale_entry_widget`) — this was a discoverability fix, not a
  capability gap, and shipped bundled with the icon-set build (the two
  were sequenced together because the Measure ▾ icon assignment was
  tracked in the icon-set entry). A real `icon-ruler.svg`-style
  affordance now reaches the Measure ▾ control; the toolbar itself now
  wraps (not an overflow menu — see Shipped entry for the rationale)
  so every control including Measure ▾ stays visible and reachable at
  any window width, closing the discoverability half of the operator's
  original dimensioning-tool complaint (the capability half was already
  closed).

**Not v1, explicitly flagged for the operator (ui-spec §A.6 / decision
017 §10 Q2–Q3):** drag-to-reorder panels is an `egui_tiles` native
capability, now available since Pass 18.1 shipped; multi-monitor
OS-window undocking (§10 Q2) is explicitly NOT granted by `egui_tiles`
(no `Surface::Window` equivalent, rerun-io/egui_tiles issue #30) —
default stands: docked-only, its own Backlog entry, still unanswered.
§10 Q1 (the `egui_tiles`-vs-hand-rolled question this note originally
tracked) is ANSWERED and BUILT — see Pass 18.1 above.

### ★ Pass 19.x — FF-H: direct text-state formatting (`Tc`/`Tz` + free-form `Ts` + synthetic bold/italic + `Tw`), decision 019 + Amendments A–F — **COMPLETE 2026-08-03, ALL FIVE SLICES SHIPPED (19.0 `38fffad`, 19.1 `603b051`, 19.2 `ebe35d8`, 19.3 `74052d3`, 19.4 `a1638f4`) — decision 019 / FF-H RETIRED, no further work on this entry**

**Decision 019 ACCEPTED via the KenAgent protocol.** Full record:
`docs/decisions/019-ffh-spacing-scaling-synthetic-styles.md`. Filed
against the ★★★ operator priority sequence's item 3 ("finish off all
the text handling stuff") — the third fast-follow decision this
directive has produced, after decisions 014/015/016.

**The parity premise for two of FF-H's four named operators collapsed
before any code was written.**
`D:\Dev\Rag-Specialized\Acrobat_Features\text_edit__spacing_and_scaling_controls.md`
(sourced to Dov Isaacs, former Adobe Principal Scientist, Adobe
Community 2019-01-03/04) establishes that Acrobat itself dropped word
spacing (`Tw`) and free-form baseline offset when text editing was
consolidated into the single Edit Text & Images tool; Isaacs' own
retained list is "adding, deleting, bold, italic, font size, leading,
kerning, and horizontal scaling." FF-H's name lists four operators
(`Tc`/`Tw`/`Tz`/`Ts`); **parity covers only two** (`Tc`, `Tz`, plus the
coarse superscript/subscript toggle Acrobat exposes as a hack on top of
`Ts`). A removal during a UI consolidation is strong evidence about the
consolidation and weak evidence about the feature on its own — but for
`Tw` a SECOND, independent signal points the same way (§9.3.3 makes it
structurally void for 2-byte composite/CID runs, and its one honest job
already collapsed into the 15.1 reflow layer's `TJ`-based design). No
such second signal exists for `Ts`.

**Q1 — the two operators SPLIT, on font-model universality vs. marginal
cost given what parity already forces, not the parity-vs-exceed axis
originally assumed:**
- **Free-form `Ts` SHIPS as a deliberate exceed.** Superscript/subscript
  is a parity must-have and `Ts` is the only PDF-native baseline
  mechanism — the emission/restore/tracking/test path is forced
  regardless of whether the raw number is exposed, so withholding it
  means building the mechanism and hiding it. Works identically on
  every font model — no void case, no refusal class.
- **`Tw` does NOT ship as a direct authoring control in the core
  slices.** Its one honest job (inter-word distribution) already
  belongs to the 15.1 reflow layer, which deliberately chose `TJ` over
  `Tw` for exactly the properties that make `Tw` a poor *control*
  (decision 015); re-adding it reintroduces what that decision rejected
  and gives the operator two dials that both look like "space between
  words," interacting multiplicatively through `Th`. Engine-first
  (read/track/publish/preserve/display-read-only, Pass 19.0); whether
  it ever becomes a direct control is gated behind a **corpus census
  with explicit decision bands** (≥60% of sampled real-world documents
  → build; ≤25% → close the item; 25–60% → escalate to the operator —
  see Open operator questions, below).

**Q2 — synthetic bold/italic: one shared policy across in-place edit
(14.x) and add-text (16.x); the asymmetry is emergent, not designed
in.** Add-Text defaults to a bundled Standard-14 whose family has real
Bold/Italic faces, so the synthesis gate rarely opens there — the
remedy-offer ORDER differs (Add-Text offers a real face first;
in-place editing offers synthesis first) but the gate, the mechanism,
and the disclosure are identical. Mechanism is `Tr 2` (stroke+fill) +
`Tm` shear, **rejecting double-strike** (doubles glyph count, breaks
the byte↔glyph correspondence provenance depends on). Two §9.3.6
correctness traps a naive implementation would hit, both bugs if
missed: stroking must use the STROKING colour matched to the fill
(otherwise coloured text acquires a black outline); stroked line width
is in USER space and must be derived from `Tfs × |Tm| × |CTM|`, never a
device-space constant. Two hazards beyond the original brief: a `Tm`
shear is **not** text state, so it survives `Td`/`TD`/`T*` into every
later line; shear composed with `Ts` displaces a raised run by
`Trise × tan θ`. Persistence is self-evident emission with no private
marker — the same bytes that make pdfce's own synthesis detectable on
reload also let it recognize other producers' faux styles.

**Q3 — build order FF-H → FF-C → FF-B.** Not on value — FF-H is judged
the least valuable of the three named fast-follows — but because Pass
19.0 (below) is a shared correctness prerequisite both FF-C and FF-B
inherit. **Pass 18.6's `Tc`/`Tw`/`Tz`/`Ts` tracking
(`vector::decompose::GState`) is explicitly NOT groundwork for this
decision, worth roughly 5%:** it is a private *reading*-path walk
feeding an approximate hit-test bbox, not callable from any authoring
path; its real contribution is being the THIRD private ambient-state
tracker in the codebase, which is what makes the Pass 19.0
consolidation argument unarguable rather than merely tidy. See the
★★★ Operator priority sequence entry's namesake-collision note (above)
— unchanged and unaffected by this decision.

**CUT from FF-H, re-filed as its own Backlog item (FF-I):** the minimal
StructTree/`/ActualText` update named in decision 014 §5.3's original
FF-H bundle is removed entirely — a *partial* structure-tree writer is
judged worse than none. Decision 016 §2 had already reached this
conclusion on the merits (ranked FF-H's StructTree piece "premature,"
deferred it); decision 019 acts on that finding by cutting it rather
than carrying it forward unbuilt. See the new "FF-I — minimal
StructTree/ActualText update" Backlog entry, below.

**Slices (Pass 19.x; no new `CommandKind` — every slice rides the
existing `FormatText`/`AddText` one-command-per-accepted-edit path):**
- **19.0 — Text-state consolidation + ambient publication (core + CLI;
  CORRECTNESS ONLY, no new operator surface). SHIPPED 2026-08-03,
  committed `38fffad` — see the Pass 19.0 Shipped entry (top of
  Shipped) for the full build record, including three deviations from
  this decision as originally written (now decision 019 Amendment A,
  `1a2e265`) and a live `q`/`Q` state-leak defect found and fixed in
  already-shipped Pass 14.2 code.** Consolidated the three private
  ambient-spacing trackers (`text_extract::page::TextState`,
  `text_edit::edit::Walk`/`reflow_apply::BlockTextState`,
  `vector::decompose::GState`) and publishes `Ts`/`Tr` through
  `GlyphProvenance` for the first time — previously dropped at
  provenance-construction time. Also fixed the **live, previously-
  masked state leak** in `reflow_apply.rs` (its preamble emitted
  `Tc`/`Tz`/`Tw` before `BT…ET` with no restore and no `q`/`Q`, illegal
  there regardless — §8.2 Table 51/Figure 9) — `restore_ops` now emits
  only on divergence, with a dedicated tripwire test guarding the gate
  19.1 will relax.
- **19.1 — `Tc` + `Tz` + superscript/subscript authoring (core + CLI).
  The Acrobat-parity slice. SHIPPED 2026-08-03, committed `603b051`**
  — see the Pass 19.1 Shipped entry (top of Shipped) for the full
  build record. `MetricSpec::{Absolute, Relative}` per R89 (ratios
  resolve against the BASE font size — decision 019 Amendment B item
  B.3). Includes a `Tz` × justify disclosure — **CORRECTED wording,
  Amendment B item B.1:** `Th` genuinely rescales every `TJ` numeric
  adjustment (§9.3.4) in general, but the specific `TJ` adjustments
  carrying a 15.1-justified line's slack sit OUTSIDE this edit's
  `set_ops`/`restore_ops` wrap (in the `pre`/`post` splice segments) and
  run at ambient, unchanged `Th` — they are NOT rescaled. The real
  cause is the formatted run's changed rendered WIDTH (`ΔA`, §9.4.4),
  which makes slack computed for the run's OLD width wrong for its NEW
  width. Same practical consequence (re-justify offered), corrected
  mechanism. Superscript/subscript ratios are pdfce's own choice — size
  **0.60×** base size, rise **+0.34×**/**−0.18×** of base size — not an
  Acrobat parity claim (Acrobat's own values are an unsourced gap in
  the parity catalog); every emitted value is disclosed by number.
- **19.2 — Free-form `Ts` + synthetic bold/italic (core + CLI). The
  deliberate exceed. SHIPPED 2026-08-03, committed `ebe35d8`** — see
  the Pass 19.2 Shipped entry (top of Shipped) for the full build
  record. `StyleSynthesis`/`SynthesisPath`/`SynthesisOffer` in new
  `text_edit/synth.rs`; R90's gate wired behind the same coverage check
  Pass 14.2 already uses for family/style changes. **The render-
  honours-`Tr 2`-and-sheared-`Tm` prerequisite was confirmed by
  MUTATION TESTING** (build the fixture-rendering tests, confirm they
  pass, then deliberately break the renderer three ways and confirm
  each mutation fails exactly the tests it should) — not merely by the
  by-inspection check this filing originally recorded, which is now
  superseded as a verification standard (see decision 019 Amendment C).
  **Amendment C filed** — six corrections found while building this
  slice, including a wrong restore-set omission (stroking colour/line
  width leak into later stroked paths), a narrower-than-written
  absolute-`Tm`-for-followers refusal gate, a disclosed two-of-three-
  factor bold-width formula, unanticipated `Tm`/`Tlm` authoring-walk
  tracking, two named conflicts refused rather than merged, and
  Add-Text synthesis flagged as not wired despite the shared type
  existing. Full account in the Pass 19.2 Shipped entry and
  `docs/decisions/019-ffh-spacing-scaling-synthetic-styles.md`
  Amendment C.
- **19.3 — GUI: the spacing/style property surface. SHIPPED 2026-08-03,
  committed `74052d3`** — see the Pass 19.3 Shipped entry (top of
  Shipped) for the full build record, including the design record
  (`docs/ui_specs/pass-19.3-text-formatting-surface.md`, committed
  `e883e26`) and the ★★★★ headline correctness finding this slice
  surfaced: `find_anchor`'s pinned-span match had used exact equality
  against a span convention no published provenance actually used, so
  every property-bar Apply shipped since Pass 14.3 had silently
  refused. Fixed in the same commit; see the Shipped entry and new
  standing rule R93.
- **19.4 — `Tw` direct-authoring control (core + CLI + GUI). SHIPPED
  2026-08-03, committed `a1638f4`** — see the Pass 19.4 Shipped entry
  (top of Shipped) for the full build record, including the
  unreachable-composite-gate finding (decision 019 Amendment F), the
  named `--find` limit, and the `Tw`×`Th` disclosure addition. Census
  method/numbers (91.6% of show operators / 97.4% of glyphs, n=4,012
  real PDFs, 1,224 text-bearing) remain recorded in decision 019
  Amendment E. The blocking `/Contents` defect (341 corpus files, found
  via this same census sweep) was fixed first, committed `409a6b5` —
  see the `/Contents`-defect-fix Shipped entry.

**MILESTONE — decision 019 / FF-H is COMPLETE end-to-end, and with it
the operator's priority-#3 item ("finish off all the text handling
stuff") is DONE as far as FF-H's own scope goes.** All five slices
(19.0 consolidation → 19.1 `Tc`/`Tz`/super-subscript → 19.2 free-form
`Ts`/synthetic bold-italic → 19.3 GUI property surface → 19.4 `Tw`)
shipped 2026-08-03. **This entry is now RETIRED — no further Pass 19.x
work is scheduled.** FF-C (font subsetting/embedding) and FF-B
(cross-block/cross-page reflow) remain unscheduled, per this decision's
own Q3 build order (FF-H → FF-C → FF-B); see the ★★★ Operator priority
sequence entry's item-3 update (above) and the "GUI redaction-apply
flow" In-progress entry (below) for what the engineer dispatched next.

**Standing rules R88–R91 added** (see Standing rules, below) — the
ceiling was R87. **R92 added 2026-08-03** (decision 019 Amendment B,
methodology: hand-duplicated no-op/arm-list predicates drift silently
— see Standing rules, below). **R88 and R90 AMENDED 2026-08-03** by
decision 019 Amendment C (Pass 19.2) — R88 gains the shared-graphics-
state restore obligation (stroking colour + line width); R90 gains the
narrower absolute-`Tm`-for-followers refusal, the disclosed bold-width
limit, and the two named unhandled-conflict refusals. **R96 added
2026-08-03** (decision 019 Amendment F, methodology: a guard clause
behind a filter the guarded case cannot pass is dead code that looks
live — see Standing rules, below). Ceiling is now **R96**.

**Open items for the operator** — see Open operator questions, below:
FF-C's rule-13 dependency classification (MIT lifted rule 8, it did NOT
pre-approve any crate); the FF-I StructTree cut (a scoping call Ken may
have counted inside "finish off all the text handling stuff");
list-authoring (re-surfaced, still unanswered); a newly-found parity
gap this decision did NOT scope — **kerning**: Isaacs lists kerning
among Acrobat's retained text-editing controls, and pdfce currently has
no kerning surface distinct from `Tc`; and the named Pass 19.4 `--find`
limit on composite runs (Amendment F, not itself a blocking question,
recorded for awareness — closing it is FF-E's scope). The `Tw` census
middle-band judgement call (former item (g)) is CLOSED AS MOOT, see
Open operator questions.

### ★ Pass 21.x — FF-C: font subsetting and glyph embedding (decision 021, 2026-08-03, DECIDED — **21.0 SHIPPED 2026-08-04, `48c6b77` — see the Pass 21.0 Shipped entry, top of Shipped, for the full build record. 21.1 promoted to In progress, below. 21.2/21.3 remain here, NOT STARTED.**)

**21.0 shipped narrower than this entry's original P0-floor bullet promised — flagged, not silently corrected in place.** Per the Pass 21.0 Shipped entry's own "NOT yet implemented" note: R109's `fsType` donor-permission read (named in the 21.0 slice bullet below) did NOT ship with 21.0. `add-text --embed-font` today embeds a donor face without reading or disclosing its embedding-permission bits. This is tracked as owed alongside 21.1, not deferred to 21.2/21.3.

**Full record:**
`docs/decisions/021-ffc-font-subsetting-and-glyph-embedding.md`.
Requested by `pdfce-engineer` per operator priority #3 ("finish off all
the text handling stuff") and decision 019 §3.8's build order FF-H →
**FF-C** → FF-B, with FF-H already complete (`a1638f4`). Filed against
`ROADMAP.md`'s FF-D-fast-follow-FF-C Backlog entry (amended above) and
Open operator questions (h) (already cleared — rule-13 licensing) and
new (r)/(s) (below).

**THE HEADLINE — FF-C as previously described anywhere in this project
(R71, decision 014 §5.3, the spec-RAG queue stub) is not
implementable, and is re-scoped ADD-ONLY.** A subset font, by
definition, does not contain the glyph being added — there is no
operation on the document's own `FontFile2` that produces missing
outline data, only a donor face on disk or nothing. Verified at
source, not relayed: `subsetter 0.2.6`'s `src/lib.rs:20–21` states
embedding forces `/Type0`+`Identity-H` because it strips `cmap`
entirely; the spec-RAG stub's own line 42 names the now-corrected-away
"add outline to `glyf`" mechanism. **FF-C adds a NEW, subsetted font
resource from a donor face (decision 012's `--font-dir`); it never
modifies an existing font program or font dictionary — R107.** The
spec-RAG stub rewrite is dispatched to `pdfce-spec-librarian` **before**
any Pass 21.0 code — it would actively mislead an implementer as
written.

**Crate boundary.** `subsetter` (Typst; MIT OR Apache-2.0; verified
transitively all-permissive) lands in `pdfce-render::font::subset`
(parses the donor via the existing skrifa parser — R21's own escape
clause, "no second parser without a new decision record," is what this
decision discharges; the `cargo tree --duplicates` guard is unchanged).
`pdfce-core::font_embed` (new module) defines the plain-data contract
and emits the PDF objects (`/Type0`+CIDFont dict+`FontFile2`/`3`+`/W`+
`/ToUnicode`); **zero new `pdfce-core` dependencies.**
`default-features = false` (the `variable-fonts` feature, unneeded at
P0, is what pulls `write-fonts`/`kurbo`) — net **1** new package, not
2; see `PRIOR_ART.md`'s amended "FF-C dependency classification"
section.

**Round-trip.** No new §5 exception needed — see R107. Original
content streams stay byte-identical; incremental save stays the
default (R36/R70); FF-C joins neither the §5.9/R58 nor §5.10/R67
forced-full-rewrite family. Enforced by an object-id-disjointness
corpus test (R97 shape) written in **21.0**, before the 21.2
`set-font` temptation to "just widen the existing font" exists — not a
runtime guard (would be R96's unreachable dead code, since the emitter
can only allocate fresh ids).

**Disclosure (rule 4, fuzzy-never-sneaky).** R108 — embedding is an
explicit, per-action operator choice, offered at the point of refusal,
never a default or a silent upgrade of the existing R79 no-embed path;
because subsetting is pure, the confirmation shows the REAL computed
subset byte count and covered/uncovered character list (R98 applied),
never an estimate. R109 — font-embedding permission (OpenType `OS/2`
`fsType`, which `subsetter` strips) is read from the donor *before*
subsetting and disclosed; absent/unparseable data is disclosed as
unknown, never silently treated as "permitted." R110 — a composite run
FF-C authors is editable only where its `/ToUnicode` is VERIFIED
injective, per font, per session; `Identity-H` with no `/ToUnicode`
stays a permanent hard skip (**R65 untouched**).

**SPEC-REVIEW AMENDMENT (2026-08-03) — narrows 21.0's P0 floor, see
`docs/decisions/021-ffc-font-subsetting-and-glyph-embedding.md` §10 for
the full eight-finding record.** `pdfce-spec-librarian`'s dispatch (§9,
above) found `subsetter`'s CFF output is an `OTTO`-wrapped sfnt that
cannot conformantly satisfy ISO 32000-1 §9.9 Table 126 (`FontFile3`
`/OpenType` requires `cmap` for CFF-outline programs; `subsetter` strips
`cmap` unconditionally; `/CIDFontType0C` needs a bare CFF program, not an
OTTO container) — decision 021 §3.4/M2's own claim that *"`subsetter`
absorbs the TrueType/CFF split entirely"* is true for simple-vs-composite,
false for the descriptor key. **21.0's P0 floor is narrowed to `glyf`
(TrueType-outline) donors only; CFF donors are refused by name
(`DonorUnsupported`) until a later slice.** L1 (the headline capability)
survives intact — Noto Sans JP/CJK, DejaVu, and most Google Fonts are
TrueType `glyf`. R109 is also amended: fsType's bit 8 (*No subsetting*,
the only thing FF-C does) and bit 9 (*Bitmap embedding only* — the spec's
own "unembeddable" case) are now two distinct named refusals
(`SubsettingNotPermitted` / `EmbeddingNotPermitted`), not one. The bit
semantics that gate Open operator question (r) are now spec-sourced (see
that item's update, below).

**Slices:**
- **21.0 — SHIPPED 2026-08-04, `48c6b77`. See the Pass 21.0 Shipped
  entry (top of Shipped) for the full build record.** Original plan
  (retained for audit trail): core + CLI, THE P0 FLOOR (glyf/TrueType
  donors only — see the spec-review amendment above; CFF donors
  refused by name). `SubsetPlan` producer; `font_embed` emitter; wire
  into `addtext.rs` (`base_font: Std14` → `NewTextFace { Std14 |
  Embedded }`); hostile-input guards (§3.5: input-side byte ceiling,
  corpus-measured not guessed; a synthetic composite-glyph-cycle
  fixture, not a depth guard — `subsetter`'s `closure()` is
  iteratively bounded by construction; a `cargo-fuzz` target); `fsType`
  read (two distinct refusal diagnostics, per the amendment above);
  disclosure. CLI: `add-text --embed-font`. Lifts the single widest
  wall in the product: pdfce today cannot add ANY text outside
  WinAnsi/Symbol/ZapfDingbats — no Greek, Cyrillic, CJK, Hebrew, or
  Devanagari, at all. `add-text` already exists as a CLI subcommand
  (rule 11 note, same reasoning Pass 19.3/8.1 recorded) — 21.0 extends
  its flags, adds no new subcommand. **AS SHIPPED, the `fsType` read
  sub-item above did NOT land** — see the Pass 21.0 Shipped entry's
  "NOT yet implemented" note; tracked as owed, not silently dropped.
  **Owed fixture, filed 2026-08-03, STILL OWED at 21.0's ship:** a
  synthetic PDF with an embedded **subset** font plus a character
  outside its repertoire — `fixtures/synthetic` currently has none
  suitable, and both `format_coverage_hint()` and `r_inv_1_hint()`
  still need it to be observed on screen for the first time (see the
  `0893191` hint-fix note in the 2026-08-03 SESSION_LOG entry).
- **21.1 — PROMOTED TO IN PROGRESS 2026-08-04 (see In progress,
  above/below).** core + CLI. Composite-run edit/format where
  `/ToUnicode` is verified injective (R110); conditionally lifts
  R-INV-4. **Makes 21.0's output editable — do not call FF-C done
  until this ships.** Left alone, 21.0 lets pdfce add text (e.g.
  Japanese) it can never afterward edit: a capability *regression*
  against the already-shipped Std-14 add-text path, invisible to every
  existing gate including the R85 raster oracle (which will show the
  glyphs, not that they're uneditable — the `flatten_fields` failure
  shape, correct counters/wrong artifact). Needs a deliberate
  acceptance criterion at ship, not merely a gate.
- **21.2 — core + CLI.** `set-font` to a newly embedded face;
  `format.rs` composite-target emission. Makes the shipped
  `format_coverage_hint()` GUI text honest for the first time — it
  currently tells the operator to supply a font via Tools › Font
  folders, and doing so does **not** fix the saved document today
  (decision 012 is read-side only); this is a current honesty gap in
  the shipped product independent of when FF-C lands.
- **21.3 — GUI. FINAL SLICE.** Face picker over `--font-dir` faces;
  refusal→remedy flow; embed confirmation showing the real subset
  size/coverage (R108); trust and licence disclosure. **Dispatch
  `pdfce-ui-specialist` first.**

**Honest limits, named up front for the ship notes:** **no shaping,
ever (R17)** — an embedded Devanagari or Arabic face places glyphs by
advance with no GSUB/GPOS, so FF-C's non-Latin headline is "CJK,
Cyrillic, Greek, Hebrew without vowel points — yes; Arabic, Devanagari,
Thai — the glyphs embed and the text is WRONG." Do not let the L1
headline imply otherwise. Variable-font donors embed at their default
instance only (feature off); bare Type 1 donors and **all CFF-outline
donors (not just CFF2 — narrowed 2026-08-03, see the spec-review
amendment above)** are unsupported at P0 (`DonorNotSfnt`/
`DonorUnsupported`); the emitted font is PDF-only, will not install
elsewhere.

**Standing rules R107–R110 added** (see Standing rules, below) — the
ceiling was R106. **Ceiling is now R110.**

**Open items for the operator** — see Open operator questions (r) and
(s), below: font-EULA policy for a donor whose `OS/2` `fsType`
forbids/is unparseable (a legal call, Ken's per
`docs/decisions/README.md`); whether Pass 21.0 refuses complex scripts
(Arabic/Devanagari/Thai) by name given R17, or discloses loudly and
lets the operator decide (recommendation: refuse by name).

### Ce-dimension-tool bug-fix cluster — Pass 12.M2c (Backlog, code-trace found 2026-08-02, distinct from Pass 18.x)

**Terminology note (added 2026-08-04, `pdfce-librarian`, per `CLAUDE.md`
rule 15 — see the ★ Terminology ruling entry under Standing rules and
the Glossary's `pdf dimension` / `ce dimension` entries):** every
"dimension" below is a **ce dimension** — pdfce's own authored
dimensioning tool (Pass 12.M2/12.M2b family) — not a pdf dimension
read from a CAD export. Retitled from "Dimension-tool bug-fix cluster"
for consistency; no bug content changed.

Five named bugs found by an engineer code trace this session (file:line
cited), sibling of Pass 12.M2/12.M2b in the same ce-dimensioning
subsystem — NOT part of the ui-spec/dock family above:

1. Ratio scale entry silently overwrites the group's display unit with
   the paper-basis unit. **Also named in decision 023 §6.5/§10.9 as a
   coupled bug that must be fixed in the same slice as Pass 23.0
   (format & units GUI surface), or that new control ships looking
   broken from day one — see the "Object-tool selection, level
   navigation & ce-dimension authoring controls" Backlog bucket
   (filed continuation 80, 2026-08-04), below, for the filed Pass
   entry.**
2. The circular (radius/diameter) tool renders NO status feedback
   below the 3-point fit threshold — the whole status body lives
   inside `if let Some(fit)`, so a partial pick shows nothing.
3. "+ New Group" is a silent no-op on an empty name, with the button
   left enabled (should disable, or disclose why nothing happened).
4. Post-second-click linear-ce-dimension clicks are dead with no hint
   that Accept/Reject is the required next action.
5. Measure tools and the group panel are entirely unreachable on a
   page whose render failed (no fallback/disclosure path).
6. Stale comment at `main.rs:5286` still claims on-canvas ce-dimension
   authoring is unbuilt — false since Pass 12.M2b (`7c93cc3`,
   2026-08-01); correct or remove it in the same Pass that touches
   this code next.

Not yet scoped into a build plan — flagged here so it isn't lost, per
the same "name it even if not building it yet" discipline as every
other Backlog bucket in this file.

### ★ Icon set — ScripTree-style SVG icons for all GUI features (operator priority #2, set 2026-08-01, SHIPPED 2026-08-02)

**SHIPPED 2026-08-02 (SESSION_LOG continuation 57), committed `c59b0c4`
as Pass 18.3 — see the Pass 18.3 Shipped entry (top of Shipped) for
full content/gates/gotchas.** This entry's history below (design
completion, the two gated-decision amendments, the pipeline switch) is
retained as the audit trail for how the build got scoped and unblocked
— nothing below needs re-reading to pick up new work, but it explains
WHY the shipped build looks the way it does (tiny-skia pipeline, not
pre-rasterized PNG; 8 verbatim + 2 derived + 25 new SVGs; the
redaction-icon style exception). **Item flagged during the build is now
RESOLVED (2026-08-03):** `▾` (U+25BE) had no glyph in egui's bundled
fonts and tofu-boxed on 4 controls, pre-existing, not introduced by this
Pass — `pdfce-ui-specialist` audited and the engineer fixed the whole
class, including a second tofu pair (`▲`/`▼` on the rail/Combine-files
reorder arrows) the original audit missed; see the "Menu-affordance &
glyph-coverage audit" Shipped entry (top of Shipped). `glyph_button` is
now deleted — pdfce has no text-glyph buttons left anywhere. `✓`/`✕` on
three tools' Accept/Reject buttons remain unverified — see Backlog.
**AMENDED 2026-08-03 — RESOLVED, as Pass 18.7** (`09be28d`, corrected
from a wrong self-reported "Pass 19.4" by `1111652` — see the Pass
18.7 Shipped entry's own Pass-number note, top of Shipped). `✓`/`✕`
(U+2713/U+2715) were confirmed genuinely tofu (not merely unverified)
and replaced with `✔`/`✖` (U+2714/U+2716); the same Pass also closed
the sibling `ⓘ`/arrow items §4.4 of the same audit had left
unverified. See the Backlog entry below (amended) for the full record.

**AMENDED 2026-08-01 (SESSION_LOG continuation 54) — BOTH gated decisions
RESOLVED by the operator; the icon BUILD is now UNBLOCKED (still queued
behind 9c-min per the priority sequence, not yet dispatched):**
(a) **SVG rendering: PRE-RASTERIZE to PNG at build time.** No new Cargo
    dependency — keeps pdfce's zero-copyleft-dependency posture. The
    runtime `resvg`/`usvg`-style crate alternative is explicitly
    REJECTED (not merely deferred).
(b) **Provenance/style, Ken's own words:** *"Scriptree icons are mine,
    use from it what makes sense and create new ones in its style when
    necessary, try to make them close to what inkscape and Adobe use
    for similar commands without running into copyright issues."*
    Concretely: USE the ScripTree SVGs (operator's own art) where they
    fit an existing pdfce control; CREATE new icons in that same flat
    48×48/`stroke="currentColor"`/outline style when no ScripTree icon
    fits; for those new icons, aim for the recognizable VISUAL
    CONVENTION Inkscape/Adobe use for the equivalent command (e.g. a
    hand for pan, a magnifier for zoom, crosshairs for a measure tool)
    WITHOUT copying their actual icon artwork — metaphor-level
    resemblance is fine, asset-level copying is not. This resolves
    gated decision (b) below without needing a separate provenance
    confirmation pass (Ken's own statement of ownership + intent IS the
    confirmation).

**Prior state (2026-08-01, SESSION_LOG continuation 53) — the DESIGN
was COMPLETE; the BUILD had not started.** `pdfce-ui-specialist` authored
`docs/ui_specs/icon-set-and-toolbar.md` while Pass 12.M2 was building
in parallel: full audit of pdfce's current icon-less/inconsistent
toolbar (emoji glyph+text / bare Unicode dingbat / plain text — three
inconsistent kinds, none an actual image), a reverse-engineered style
contract from `D:\Dev\ScripTree\icons\*.svg` (48×48 viewBox,
`stroke="currentColor"`, outline-only), and an icon→feature mapping
covering all 27 controls audited (shipped + the not-yet-built Measure
▾ menu from `pass-12.M2-dimension-tools.md`) plus lighter-depth
reservations for unbuilt features (§8). **One deliberate style
exception:** redaction's icon is the ONE solid-filled glyph in an
otherwise all-outline set — a solid black bar over a faint outlined
rect, because an outline-only icon would visually understate what
redaction actually does (irreversibly removes content, not just masks
it) — recorded as a rule-based exception, not a style drift. **Two
decisions were named in the spec (§7) as operator/KenAgent-gated before
the BUILD could be scoped — this design doc did NOT decide either; BOTH
are now RESOLVED per the continuation-54 amendment above:**
(a) **SVG-in-egui rendering pipeline** — pre-rasterize to PNG at build
    time (`include_bytes!`, zero new Cargo dependency, but fixed
    resolution unless multiple DPI variants are baked) vs. a runtime
    SVG-rasterizing crate such as `resvg`/`usvg` (crisp at any DPI/zoom,
    but MPL-2.0 — a real rule-13 dependency classification even though
    pdfce is now MIT, needing an explicit `docs/PRIOR_ART.md` check and
    operator go/no-go before it enters `Cargo.toml`). **RESOLVED:
    pre-rasterize to PNG chosen; `resvg`/`usvg` rejected.**
(b) **ScripTree icon provenance/licensing confirmation** — every
    inspected SVG carries a "generic … placeholder, not a vendor
    trademark logo" comment suggesting original-art authorship, and
    Ken owns both projects, so this is LIKELY a non-issue — but per
    rule 13 / `LEGAL.md` §5's spirit (applied to icon art, not test
    corpus) it must be **confirmed, not assumed**, before any SVG is
    copied into pdfce's asset tree; the spec recommends a
    `PROVENANCE.md` in the new asset directory recording the
    confirmation. **RESOLVED: Ken confirmed ownership/intent directly
    (see the continuation-54 quote above) — use ScripTree SVGs where
    they fit, create new ones in-style otherwise, resemble Inkscape/
    Adobe VISUAL CONVENTIONS (not their artwork) for new icons. A
    `PROVENANCE.md` recording this confirmation is still owed in the
    new asset directory when the BUILD lands, per the spec's own
    recommendation.**

**AMENDED 2026-08-02 — the "queued behind 9c-min" gate below is now
MOOT (9c-min shipped 2026-08-01, `76485b5`) and is superseded by a
NEW, larger gate.** Two things changed the queue position:
(1) **the SVG-rasterization pipeline decision (pre-rasterize to PNG at
build time, resolved continuation 54) is NOT EXECUTABLE on this
machine** — no Inkscape, no ImageMagick, and `cairosvg`'s libcairo
fails to load. The engineer has proposed an alternative to the
operator that needs no new dependency and no external tool: a minimal
SVG-path-`d` parser feeding `tiny-skia` (already a pdfce dependency,
reachable from `pdfce-gui` via `pdfce_render::tiny_skia`), giving crisp
icons at any DPI/zoom. (2) **The dispatch order itself changed:**
decision 018 recommends Pass 17.x (live-edit rendering) land before ANY
new feature work, including the icon build.

**BOTH RESOLVED 2026-08-02 (this session, continuation 56):**
(1) **Pipeline switch CONFIRMED by the operator** — minimal SVG-path-`d`
parser feeding `tiny-skia` (already a pdfce dependency, no new Cargo
dependency), NOT the pre-rasterize-to-PNG plan (which cannot execute on
this machine — no Inkscape/ImageMagick, `cairosvg` libcairo load
failure). Open operator question (a) is now closed; the pre-rasterize
plan above is superseded, retained for history only.
(2) **Sequencing CONFIRMED — Pass 17 before new feature work**, per the
same operator answer. Open operator question (f) is now closed; the
★★★★★ reordering entry (above) is CONFIRMED, not merely proposed.
**SHIPPED 2026-08-02 (continuation 57), ahead of Pass 17.1/17.2** — see
the SHIPPED banner at the top of this entry and the Pass 18.3 Shipped
entry (top of Shipped) for what actually landed. (Historical note,
retained: at the time this paragraph was written, the plan was to wait
for Pass 17.1/17.2 before starting the icon build; in practice it
shipped as part of the same Pass 18.3 commit as the Measure ▾
affordance fix, with Pass 17.1/17.2 still unbuilt — see Next up.)

Ken wants pdfce's GUI toolbar/tool icons styled after
`D:\Dev\ScripTree\icons\*.svg` — the existing coherent SVG icon set
used elsewhere in Ken's tooling (flat, simple, single-purpose glyphs
per tool). Applies across **every** GUI feature/tool currently
exposed without a proper icon (viewer navigation, the Pass 6.x/8.0
markup/redaction tools, the Pass 12.0+ canvas tools including the
dimensioning tool once built, Pass 14.x/15.x/16.x text tools, etc.) —
a coherent icon language for the whole toolbar, not a one-off asset
drop.

**Scoping notes for whoever picks this up (original, filed 2026-08-01
— DONE per the design-complete amendment above, retained for history):**
- ~~Not yet audited: which of pdfce's current GUI affordances are
  glyph-only vs. text-labeled...~~ **DONE — `icon-set-and-toolbar.md`
  §0 is exactly this audit** (three inconsistent existing kinds
  cataloged, `icon_button`'s accessible-name wrapper confirmed as
  must-preserve).
- ~~`pdfce-ui-specialist` is the natural dispatch for the icon→feature
  mapping...~~ **DONE — same spec, §§1-6.**
- ~~Licensing check owed before adoption...~~ **RESOLVED 2026-08-01
  (continuation 54) — see gated decision (b) above; Ken confirmed
  provenance/style directly.**
- ~~Format/pipeline question, unresolved...~~ **RESOLVED 2026-08-01
  (continuation 54) — see gated decision (a) above; pre-rasterize to
  PNG chosen, `resvg`/`usvg` rejected.**
- ~~Queued behind 9c-min...~~ **MOOT — 9c-min shipped 2026-08-01
  (`76485b5`), and the icon build itself has since SHIPPED 2026-08-02
  (`c59b0c4`, Pass 18.3) — see the SHIPPED banner at the top of this
  entry.**

### ★ NEXT MAJOR FOCUS — Pass 14.x: Acrobat text-handling parity (decision 014, DECIDED 2026-07-31)

**Decision 014 ACCEPTED via the KenAgent protocol.** Full record:
`docs/decisions/014-acrobat-text-editing.md`. This is the operator's
explicitly-directed NEXT MAJOR FOCUS (Backlog's "★ NEXT MAJOR FOCUS"
bucket, filed 2026-08-01) — Acrobat-style in-place text editing
(select/re-edit page-content glyph runs, paragraph/block recognition,
reflow, formatting), sequenced AHEAD of the further Inkscape/vector-
editing breadth (Pass 9 slices (b)–(g) and the "Vector graphics editing
(Inkscape-parity)" Backlog bucket — see the dated amendment above).
Distinct from the shipped text EXTRACTION path (Pass 4) and the
text-bearing ANNOTATIONS path (Pass 6.2 overlay authoring) — this edits
the page's OWN content.

**AMENDMENT (2026-08-01) — ALL FOUR SLICES SHIPPED; DECISION 014
COMPLETE.** Pass 14.0 (read-only editable text model + block
recognition), Pass 14.1 (in-place text editing via content-stream
surgery + font-on-edit refusal gate + CLI `edit-text`), Pass 14.2
(formatting on a selection — size/fill-colour/font-family-style), and
now **Pass 14.3 (on-canvas text-editing UI + the `EditSession`
undo-integration prerequisite — FINAL SLICE)** are all shipped — full
records in Shipped (above). **Decision 014's Acrobat in-place
text-editing subsystem is COMPLETE end-to-end.** The bullet list
immediately below is decision 014's original scope record, retained
for history — all four bullets are now superseded by their Shipped
entries. The active thread going forward is the reflow ladder (FF-A
next); see the new "FF-A scoping + decision 015" note below.

**PASS-NUMBER RENUMBER (recorded explicitly — do not lose this):**
decision 014's archived record proposes this family as "Pass 13.x
(13.0–13.3)". **13.x is already taken** — Pass 13a (xref EOL audit) and
Pass 13b (rebuild-by-scan recovery, decision 013) both SHIPPED under
that number before decision 014 was filed (see Shipped). The
text-editing family is therefore assigned the next free MAJOR number,
**Pass 14.x**:

- **14.0 — Editable text model + block recognition (READ-ONLY, core +
  CLI inspect). SHIPPED 2026-08-01 — see Shipped above.** Run→Line→Block
  hierarchy derived from Pass 4's
  extraction (baseline-Y clustering, x-band columns, indent/leading
  paragraph segmentation — reusing `layout.rs`'s three ratios);
  provenance linkage (source show-operator identity, byte span, full
  text-state) on every Run/Glyph; hit-test + caret/selection API;
  `inspect --text-blocks`. Acceptance: blocks recognized/counted on a
  multi-paragraph/multi-column fixture; NO write; core stays GUI-dep-
  free; Pass 4 tests unchanged; fmt/clippy clean.
- **14.1 — In-place edit + single-line relayout + the font-on-edit gate
  (core surgery + CLI `edit-text`). SHIPPED 2026-08-01 — see Shipped
  above.** Extended Pass 8.0's advance-preserving interpreter from
  REMOVE to REPLACE; an inverse-encoding builder (Unicode→code,
  inverting Pass 4's §9.10.2 decode, never `/ToUnicode`); advance-
  preserving same-line relayout (REFLOW-default, `--pin` fallback); the
  8-trigger (R-INV-1..8) font-on-edit gate keyed on `GlyphSource` +
  glyph presence (decision 012); default INCREMENTAL save (R36) with
  prior-text disclosure — NOT redaction's forced full rewrite;
  tagged-run MCID/BDC/EMC preservation + staleness disclosure.
  Non-goals (unchanged, still deferred): reflow across lines (FF-A/
  FF-B), family/style change (14.2), subsetting (FF-C), composite/
  CJK/RTL (FF-E/FF-F), add-new-text (FF-D).
- **14.2 — Formatting on a selection (core + CLI). SHIPPED 2026-08-01
  — see Shipped above.** Size (`Tf`), fill colour (`rg`/`g`/`k`), gated
  family/style change (re-encode into an available covering face, else
  refuse-and-disclose). Non-goals (unchanged, still deferred):
  spacing/scale/rise, synthetic styles (FF-H). The forward-looking
  fill-colour parity-plus design decision recorded ahead of this
  Pass's ship (unlike Acrobat, which always stores `DeviceRGB`
  regardless of the colour-picker mode shown, pdfce lets the operator
  choose RGB/CMYK/gray and STORES the actual chosen space) shipped
  exactly as designed — verified by the Shipped entry's
  `cmyk_color_change_stores_k_not_devicergb` test.
- **14.3 — Edit UI on the Pass 12.0 canvas (gui). SHIPPED 2026-08-01 —
  see Shipped above.** CanvasTool::TextEdit: click→caret, Shift-click→
  extend, double-click→word, drag→select (P0; triple-click/arrow-nav
  deferred, named), live preview accepted/rejected by the operator,
  read-only block-boundary review overlay, the three-trust-level +
  tagged disclosures surfaced. Also shipped the blocking §0.2
  `EditSession` undo-integration prerequisite (edit/format as undoable
  commands over the session's in-memory graph).

**Standing rules R69–R74 added** (decision 014 §5.1's six proposed
rules, filed in order — see Standing rules below): **R69** text-edit-
is-surgery-not-overlay; **R70** text-edit-is-incremental-not-a-scrub;
**R71** font-on-edit-trust-ladder; **R72**
recognized-blocks-and-reflow-are-reviewable-hints; **R73**
tagged-edits-disclose-never-corrupt; **R74**
text-model-in-core-edit-UI-in-gui.

**Alignment cross-reference — adjudicated, NOT a scope gap (engineer
decision, 2026-08-01).** The same Pass-14.2 scoping pass that surfaced
the list-authoring gap (see Backlog) also flagged text ALIGNMENT (left/
center/right/justified) as possibly unscoped. It is not: decision 014
already covers it — FF-A handles left/center/right (within-block
reflow) and FF-B adds justified. Alignment only meaningfully applies
when a block re-wraps, so it correctly lives in the reflow ladder
below, not in Pass 14.2 (which only formats size/colour/family on an
existing, non-reflowed selection). Recorded here so this was
adjudicated, not overlooked.

**AMENDED 2026-08-01 by decision 015 §3.1:** "FF-B adds justified" above
is superseded — decision 015 relocates justified alignment OUT of FF-B
INTO FF-A (it is a within-block alignment mode, a peer of left/center/
right, orthogonal to FF-B's cross-block/cross-page scope). FF-A now
ships all four alignment modes; FF-B's headline narrows to cross-block +
cross-page reflow only. See the ★ Pass 15.x entry below.

**FF-A scoping + decision 015 (filed 2026-08-01, following Pass 14.3's
ship).** `pdfce-acrobat-librarian` has scoped FF-A (within-block
offline reflow) into
`D:\Dev\Rag-Specialized\Acrobat_Features\text_edit__paragraph_reflow_and_auto_adjust_layout.md`.
A KenAgent decision (**015**) is now being taken to settle FF-A's
architecture before any build — per rule 12 (parity reference → scope
→ KenAgent decision → build). **Two FF-A differentiators the scoping
surfaced** (recorded now so they aren't lost before decision 015
lands): (1) alignment auto-detection — a block's existing left/center/
right alignment must be DETECTED and PRESERVED through reflow, not
reset to a default; (2) never silently drop page-overflowed content —
a reflow that would push text past the page/box boundary must be
disclosed (fuzzy-never-sneaky), never silently truncated.

**OPEN QUESTION flagged for decision 015 — RESOLVED 2026-08-01 (justified
alignment vs. FF-B).** Acrobat exposes a **Justify** button on its
BASE (non-cloud) Edit-Text panel, in direct tension with decision 014's
working assumption that justified alignment requires FF-B (cross-block
reflow, the exceed-Acrobat capability). **Decision 015 §3.1 settles
this: justified moves into FF-A.** The base-panel placement proves
Justify is a classic-engine (offline, single-block) capability —
within-block inter-word slack distribution over an already-computed
greedy wrap — not an Auto-Adjust/cloud one. Justified ships alongside
FF-A (all four alignment modes: left/center/right/justified); FF-B's
scope narrows to cross-block + cross-page offline reflow only. See the
★ Pass 15.x entry below and `docs/decisions/015-ffa-within-block-offline-reflow.md`
§3.1.

**Fast-follow ladder (named, not scheduled) — AMENDED 2026-08-01 by
decision 015 (justified relocated FF-B → FF-A; see the ★ Pass 15.x
entry below for the now-DECIDED FF-A architecture):** FF-A within-block
offline reflow (greedy line-break, breaker factored out of `vartext.rs`;
left/center/right/**justified** — all four alignment modes; Acrobat
offline parity plus a reliable-Justify lead) · FF-B cross-block/
cross-page offline reflow ONLY (justified removed — the exceed-Acrobat
headline — Acrobat's cross-block reflow is cloud-gated + English-only;
pdfce's is offline + script-agnostic) ·
FF-C font-subsetting/glyph-embedding writer capability (lifts the
embedded-subset refusal; permissive-only, rule 13; needs a dependency-
licensing escalation before it is built) · FF-D add NEW page text ·
FF-E CJK/composite in-place editing (couples to decision 012 FF2) ·
FF-F RTL/bidi editing (R17 / `layout.rs` bidi deferral) · FF-G
OCR-gated scanned-text editing (edits the OCR layer, never the raster)
· FF-H `Tc`/`Tw`/`Tz`/`Ts` + synthetic styles + minimal StructTree/
`ActualText` update.

**Honest limits named up front:** outlined/vectorized text is
permanently unreachable (detect + disclose "vector art"); scanned/
raster text needs OCR first (FF-G); embedded-subset fonts can't gain
NEW glyphs until FF-C; composite/CJK (FF-E) and RTL/bidi (FF-F)
deferred; reflow is derived and always reviewable, never authoritative.

**Where pdfce exceeds Acrobat:** offline cross-block reflow (FF-B) ·
minimal-diff tag preservation — Acrobat's own in-place edit is known to
corrupt the accessibility structure tree; pdfce discloses instead
(R73) · first-class scriptable CLI `edit-text` (Acrobat has none).

**Zero new dependency for 14.0–14.2** (reuses Pass 4 extraction, Pass
8.0 surgery, `vartext.rs` line-breaking, decision 012's `GlyphSource`,
the one skrifa parser R21). Only FF-C would add a crate — gated
permissive-only (rule 13) and flagged early so it never blocks the
shipping slices.

**Timing — the four gating items decision 014 named are now ALL
complete.** Font-supply (decision 012) SHIPPED; the Pass 12.0 canvas
substrate SHIPPED; xref-recovery (decision 013, Pass 13a + 13b) SHIPPED
(see Shipped above); the measurement/dimensioning beta's foundation IS
the same shipped Pass 12.0 canvas. Starting 14.0 is now an engineering
scheduling call, not blocked on any further prerequisite. The beta's
remaining vector-selection/basic-editing slices (9a/9c-min) vs. Pass
14.x ordering remains the flagged operator sequencing question
(continuations 32/33, unchanged).

**Prereqs / dispatch order (decision 014 Appendix B):** 14.0's
provenance-linkage extension of Pass 4 first (confirm what Pass 4
already carries per run vs. what must be added); `pdfce-spec-librarian`
for 14.1 (§9.4.x advance + inverse encoding) and a QUEUED
font-subsetting spec dispatch for FF-C; `pdfce-ui-specialist` before
14.3.

### ★ Pass 15.x — FF-A: within-block offline reflow (decision 015, DECIDED 2026-08-01)

**Decision 015 ACCEPTED via the KenAgent protocol.** Full record:
`docs/decisions/015-ffa-within-block-offline-reflow.md`. Scopes FF-A
(decision 014's fast-follow ladder) — within-block, offline, explicit/
reviewable text reflow — as the active thread now that decision 014's
Pass 14.0–14.3 in-place-editing family is COMPLETE (see Shipped).
**AMENDS decision 014** §3 (Reflow), §5.3 (fast-follow ladder), §6:
**justified alignment moves OUT of FF-B and INTO FF-A** as a fourth
within-block alignment mode (a peer of left/center/right); FF-B's
headline narrows to cross-block + cross-page offline reflow only. This
**RESOLVES** the justified-alignment open question flagged at Pass
14.3's ship (SESSION_LOG continuation 38) — Acrobat's base-panel
Justify sits on the classic (offline, single-block) engine, so it
belongs in FF-A, not gated behind FF-B's cross-block engine.

**PASS-NUMBER CALL (librarian, per decision 015 §6's delegated choice):**
assigned a fresh **Pass 15.x** rather than folding into 14.4–14.6 —
keeps "Pass 14.x = in-place editing" and "Pass 15.x = reflow" as two
coherent, separately-referenceable families, the same precedent set
when 14.x itself was assigned fresh after 13a/13b had already taken
13.x for xref recovery.

**AMENDMENT (2026-08-01) — 15.0 SHIPPED.** Pass 15.0 (within-block
greedy reflow engine + alignment auto-detect, read-only) has shipped —
full record in Shipped (above). **15.1 (reflow surgery) is promoted to
In progress** — see In progress (above) — with its §9.4.3 `TJ` /
§9.3.3 `Tw` spec grounding being sourced by `pdfce-spec-librarian` in
parallel. 15.2 (reflow UI) remains in Next up, unscheduled until
15.0–15.1 + Pass 14.3 (already shipped) are all consumed by it.

**AMENDMENT (2026-08-01) — 15.1 SHIPPED.** Pass 15.1 (reflow-apply
surgery + one undo-able `CommandKind::ReflowBlock` + CLI `reflow`) has
shipped — full record in Shipped (above); reflow now APPLIES, not just
previews (a live justified reflow demonstrated correct right-flush with
an un-stretched last line, undo byte-identical, overflow emitted-not-
clipped, composite refused-and-disclosed). **15.2 (reflow UI) is
promoted to In progress** — see In progress (above). Once 15.2 ships,
decision 015 / FF-A is COMPLETE end-to-end.

**AMENDMENT (2026-08-01) — 15.2 SHIPPED. DECISION 015 / FF-A COMPLETE
END-TO-END.** Pass 15.2 (on-canvas within-block reflow UI) has
shipped — full record in Shipped (above): reflow lives as a sub-mode of
`CanvasTool::TextEdit` (no new tool variant, per R60), with a ghost
preview, a targeted-block highlight, width/alignment/leading controls,
and Accept/Reject landing exactly Pass 15.1's one undo-able
`CommandKind::ReflowBlock`. A P0 consolidation also collapsed three
duplicate copies of the relaxed block-recognition helper into one
`pub fn reflow_recognition_options()` in `pdfce-core`. **All three
slices (15.0 engine, 15.1 surgery, 15.2 UI) are now shipped — decision
015 and FF-A are COMPLETE end-to-end.** The fast-follow ladder's next
scoping question is FF-B (cross-block/cross-page offline reflow — the
genuine exceed-Acrobat headline, since Acrobat's own cross-block reflow
is cloud-gated + English-only); it is NOT yet scoped to a Pass. This
entry (★ Pass 15.x) is now historical record, same as the ★ Pass 14.x
entry above it once decision 014 completed.

- **15.0 — Alignment inference + within-block greedy re-wrap engine
  (core, READ-ONLY, CLI inspect). SHIPPED 2026-08-01 — see Shipped
  above.** The
  derived `ReflowEngine`: `Block` + width + alignment + leading →
  `ReflowPreview` via a greedy breaker factored out of `vartext.rs`'s
  packing core (taking a width-measuring closure — `vartext` keeps its
  Std14-AFM path; 15.0 supplies a provenance-§9.4.4-advance measurer,
  one breaker, two callers); alignment (left/center/right/justified)
  auto-detected from glyph x-positions via the Pass 14.0 x-band
  geometry; `pdfce-cli inspect --reflow-preview`. **Acceptance:** greedy
  wrap matches hand-computed break points at a given width on a
  multi-line fixture; L/C/R/justified inferred correctly on aligned
  fixtures; single-line block → left + disclosed ambiguity note;
  oversized word → one overflowing line + disclosure; NO write; core
  stays GUI-dep-free (`cargo tree -p pdfce-core`); Pass 14.0 tests
  unchanged; fmt/clippy clean. **Non-goals:** any content-stream
  mutation, UI, cross-block. **Prereqs:** Pass 14.0; factor the greedy
  core out of `vartext.rs`.
- **15.1 — Reflow surgery + one undo-able `CommandKind::ReflowBlock`
  (core + CLI). SHIPPED 2026-08-01 — see Shipped above.** Apply an accepted `ReflowPreview` via the 14.1
  advance-preserving surgery; justified slack distributed via `TJ`
  (§9.4.3) numeric position adjustments / `Tw` (§9.3.3) word spacing
  (the single-byte-code-32 simple-font path); the last line of each
  paragraph stays un-stretched at base alignment; unchanged lines
  byte-identical; default incremental save (R34/R36); lands as ONE
  `CommandKind::ReflowBlock` on `EditSession`; tagged-block MCID
  preservation + staleness disclosure (R72); page-overflow
  disclose-and-allow (R76, below). `pdfce-cli edit-text --reflow` / a
  `reflow` subcommand. **Acceptance:** re-wrap an embedded-full and a
  non-embedded block correctly; only the block's own content-stream
  object changed (R32/R46); incremental-save-safe (R34) verified;
  justified distributes slack correctly (last line un-justified);
  page-overflow disclosed + content emitted, never clipped; undo
  restores the byte-identical pre-reflow stream; R59 + round-trip
  green; fmt/clippy clean. **Non-goals:** cross-block/cross-page (FF-B),
  Knuth-Plass, hyphenation, composite/CJK/RTL, UI. **Prereqs:** 15.0;
  Pass 14.1 surgery; dispatch `pdfce-spec-librarian` for §9.4.3 `TJ`
  distribution + the §9.3.3 `Tw` single-byte-code-32 caveat.
- **15.2 — Reflow UI: preview overlay + width/alignment adjust +
  accept/reject (gui). SHIPPED 2026-08-01 — see Shipped above.** Canvas:
  invoke reflow on a block; ghost preview vs current; drag-adjust block
  width; alignment picker (L/C/R/justify, pre-filled with the
  auto-detected value); leading; accept → one command, reject →
  nothing; overflow and disclosures surfaced. **Prereqs:** 15.0–15.1 +
  Pass 14.3; `pdfce-ui-specialist` dispatched first, per the standing
  rule for non-trivial UI changes.

**What stays in FF-B:** cross-block reflow (content moving between
sibling blocks) and cross-page reflow (content to/from adjacent pages)
— the genuine exceed-Acrobat headline (Acrobat's cross-block reflow is
cloud-gated + English-only). **Justified is REMOVED from FF-B**
(relocated to FF-A above, decision 015 §3.1).

**Standing rules R75–R77 added** (decision 015 §5, filed in order — see
Standing rules below): **R75** reflow-is-explicit-reviewable-single-
block-one-undo-command; **R76** reflow-overflow-discloses-never-
disappears; **R77** alignment-auto-detected-and-preserved-through-
rewrap. (Kept as three distinct rules rather than folding R77 into R75
— each names a genuinely separate invariant: operation shape/scope,
overflow disclosure, and alignment fidelity — matching the granularity
decision 014's six rules R69–R74 already established for this family.)

**Where pdfce exceeds Acrobat (decision 015 §9):** offline (no cloud,
no qualification gate, no English-only limit) within-block reflow ·
reliable offline Justify (Acrobat's own is community-reported buggy) ·
alignment auto-detect + preserve (Acrobat has none — risks a silent
left-align on re-wrap) · page-overflow disclosed, never silently
disappeared (Acrobat's own documentation says overflow "disappears") ·
reflow as an explicit one-undo reviewable command, not a silent black
box · first-class scriptable CLI `reflow` (Acrobat has none) ·
minimal-diff — only the block's own content stream changes and tags
are preserved.

**Honest limits named up front (decision 015 §8):**
single-detected-block scope only (cross-block/cross-page is FF-B) ·
greedy first-fit, no Knuth-Plass · no hyphenation — an oversized word
overflows unbroken and is disclosed · simple fonts only (composite/CJK
is FF-E; `Tw` word spacing is single-byte-code-32-only per §9.3.3) ·
LTR only (RTL/bidi is FF-F) · alignment auto-detect is a reviewable,
overridable heuristic, corpus-tuned · reflow is always derived and
reviewable, never silent.

### ★ Pass 16.x — FF-D: add NEW page text as real page content (decision 016, DECIDED 2026-08-01)

**Decision 016 ACCEPTED via the KenAgent protocol.** Full record:
`docs/decisions/016-ffd-add-new-page-text.md`. Decision 016 does two
jobs: (1) **prioritizes** the remaining named text-parity fast-follows
now that decision 014 (in-place editing, Pass 14.x) and decision 015
(FF-A within-block reflow, Pass 15.x) are both COMPLETE end-to-end —
ranking **FF-D (add new page text) #1, solo-startable now**; FF-C
(font subsetting/embedding) #2 on value but **operator-gated** (rule 13
copyleft + rule 8 license); FF-B (cross-block/cross-page reflow)
deferred (rarest daily action, largest new subsystem); FF-H (spacing +
synthetic styles + StructTree) deferred; list-authoring **operator-
gated** (scope call) — and (2) **scopes FF-D** concretely into a Pass
family (below). **RECOMMENDED NEXT BUILD.**

**PASS-NUMBER CALL (librarian, per decision 016 §6's delegated choice):**
assigned a fresh **Pass 16.x** — "text authoring" — rather than folding
into 14.5–14.7. This keeps three coherent, separately-referenceable
families intact: Pass 14.x = in-place editing, Pass 15.x = reflow,
Pass 16.x = authoring NEW text. Same precedent as 15.x's own
fresh-family assignment (continuation 39) rather than folding into
14.4–14.6.

**The load-bearing scoping call (decision 016 §3.1): real page content,
NEVER a FreeText annotation.** FF-D synthesizes a **new `BT…ET` text
object** (§9.4) appended to the page's own content — genuine, editable
page text, routed through the SAME 14.x edit/format + 15.x reflow
pipeline as any other run. It is explicitly, structurally distinct from
the already-shipped Pass 6.2 **FreeText annotation** path — the catalog
documents a real Acrobat naming collision between "Edit PDF → Add Text"
(page content) and "Fill & Sign → Add Text" (typewriter-descended
`/FreeText`) with different removal/flatten/permission semantics; pdfce
must not conflate them (→ R78 below).

**Default font — bundled Standard-14, no embedding (why FF-D needs no
FF-C, decision 016 §3.3).** New runs default to a bundled Standard-14
permissive face (§9.6.2.2 — available without embedding), operator-
configurable via decision 012's `GlyphSource`. Because the new run is
saved by name+code with no embedding, it is decision 014's *most-
editable* font case and never touches the embedded-subset refusal wall
FF-C exists to lift — this is the reason FF-D is solo-startable today
and does not depend on FF-C landing first.

**Minimal-diff save — append a content stream, don't rewrite (§3.5).**
The new run is appended as an **additional stream in the page
`/Contents` array** (§7.7.3.3 — `/Contents` may be an array of
concatenated streams), leaving the original content stream byte-
identical; one `/Font` entry is added to page `/Resources` (§7.8.3, a
plain Type1 dict, no `FontFile`). Default **incremental save**
(R34/R36), lands as one undo-able `CommandKind::AddText` on
`EditSession` — same posture as `EditText`/`FormatText`/`ReflowBlock`.

**Tagged pages — disclose untagged, never corrupt (§3.6, governed by
the existing R73, no new rule needed).** New content on a tagged page
is emitted as untagged page content (or a minimal marked-content/
artifact wrapper) and pdfce discloses that the structure tree's reading
order was not updated — never a silently fabricated structure element.

**Standing rules R78–R79 added** (decision 016 §5, filed in order — see
Standing rules below): **R78** add-new-text-is-page-content-surgery-
never-freetext; **R79** new-text-uses-bundled-supplied-face-no-
embedding-disclosed-provenance.

**AMENDMENT (2026-08-01) — 16.0 SHIPPED.** Pass 16.0 (add-new-text
engine + point-text insert, core + CLI `add-text`) has shipped — full
record in Shipped (above): a live add ("Added by pdfce" at 100,700)
produced exactly two new objects (content stream + Standard-14 font
dict), the original page byte-untouched, round-trip green
(`identical=1, raster_identical=1, reloaded=1`), inheritance-safe
`/Resources` rebuild verified, tagged-page R73 disclosure verified, and
the R71 missing-glyph refusal verified both core-side and CLI-side
(exit 9). A flagged follow-up (NOT yet actioned): `add_text`/
`EditSession::add_text` has no certification-signature guard, unlike
`add_markup` — see the Backlog bullet "FF-D follow-up —
certification-signature guard gap."

**AMENDMENT (2026-08-01) — 16.1 SHIPPED.** Pass 16.1 (boxed add-new-text
+ wrap via the 15.x reflow engine, core + CLI `add-text --box`) has
shipped — full record in Shipped (above): 16 core + 13 CLI `add_text`
tests pass (up from 9+8), a live boxed justified add wrapped to 2 lines
with the derived-layout disclosure, round-trip `identical=1,
raster_identical=1` (original byte-untouched). Boxed session
integration was FREE (shared `plan_add_text` planner, no `edit.rs`
change). **16.2 (add-text canvas UI) is the sole remaining slice of
decision 016/FF-D, now In progress** — its UI design (from
`pdfce-ui-specialist`) has shipped, the build is dispatched — see
below.

**AMENDMENT (2026-08-01) — 16.2 SHIPPED. DECISION 016 / FF-D COMPLETE
END-TO-END.** Pass 16.2 (on-canvas Add-Text UI) has shipped — full
record in Shipped (above): a dedicated `CanvasTool::AddText` variant
(mutually exclusive with `TextEdit`), click→point / drag→box
rubber-band / typing→live wrap-preview ghost (via the new pure
`preview_wrap` core accessor) / property bar (size, colour, font,
alignment) / Accept→one `CommandKind::AddText` / Reject+Esc→discard,
plus the required FreeText-tooltip disambiguation across all three
text-tool tooltips. A P0 pure factoring also gave `preview_wrap` and
`add_text`'s boxed path ONE shared `layout_boxed` pass (no duplicated
wrap/origin/overflow math), proven by a parity test that parses
`add_text`'s actual emitted `Tm` operands and asserts they equal the
preview's origins. **All three slices (16.0 engine, 16.1 boxed/wrap,
16.2 UI) are now shipped — decision 016 and FF-D are COMPLETE
end-to-end.** This entry (★ Pass 16.x) is now historical record, same
as the ★ Pass 15.x and ★ Pass 14.x entries above it once their
respective decisions completed. See "In progress" (above) for the
broader text-parity-arc-complete milestone and the clean decision point
that remains open (FF-B/FF-H lower-priority-deferred; FF-C/list-
authoring operator-gated).

**AMENDMENT (2026-08-01) — FF-D FOLLOW-UP HARDENING SHIPPED; BACKLOG
GAP CLOSED.** The certification-signature-guard gap flagged at Pass
16.0's ship (`add_text`/`EditSession::add_text` had no
`check_certification` guard, unlike `add_markup`) is now CLOSED — see
the "FF-D follow-up hardening" Shipped entry (top of Shipped, above).
A certified (MDP-locked) PDF now refuses an `add_text` that its
`/Perms /DocMDP` forbids, via a new shared `pub(crate)
refuse_if_certification_forbids` helper both `EditSession::add_text`
and the free `add_text` function call (posture (a) — the free function
guards itself, since `signature::census`/`forbids_structural_change`
were already reachable without going through `EditSession`). The
Backlog "FF-D follow-up" entry is marked RESOLVED (see Backlog, below).
With this closed, decision 016/FF-D and the broader text-parity arc
(decisions 014+015+016) have no known open loose threads — the
remaining items (FF-B, FF-H, FF-C, list-authoring) are all either
lower-priority-deferred or explicitly operator-gated, none of them a
dangling engineering gap.

- **16.0 — Add-new-text engine + point-text insert (core + CLI).
  SHIPPED 2026-08-01 — see Shipped above.** Synthesize a new `BT…ET`
  object at operator coordinates (`Tm`, §9.4.2); append as a new content
  stream in `/Contents` (§7.7.3.3), original byte-identical; add one
  `/Font` resource entry (§7.8.3); default-font policy (bundled Std-14,
  §9.6.2.2, decision 012 `GlyphSource`, configurable); route the new run
  into the Pass 14.0 model as a first-class `Run`/`Line`/`Block`;
  F-refuse gate reuse (R71); 14.2 size/colour formatting; incremental
  save; tagged-page untagged disclosure (R73); one
  `CommandKind::AddText`; `pdfce-cli text add --at "x,y" --text "…"
  [--font NAME|auto] [--size N]`. **Acceptance:** new run added, original
  content stream byte-verbatim (R32/R46); the new run is a recognized,
  editable (14.1) and formattable (14.2) block; a glyph the face lacks
  is refused-and-disclosed; a tagged-page add emits the untagged
  disclosure; a supplied face lifts coverage and is disclosed
  `Supplied`; incremental-save-safe (R34); R59 + round-trip green; ZERO
  new dependency; core stays GUI-dep-free (`cargo tree -p pdfce-core`);
  Pass 14.x/15.x tests unchanged; fmt/clippy clean. **Non-goals:** boxed
  wrap (16.1), lists, StructTree insertion (FF-H), FreeText conflation,
  composite/CJK/RTL new text (FF-E/FF-F). **Prereqs:** Pass 14.0–14.2,
  decision 012; dispatch `pdfce-spec-librarian` for §7.7.3.3 (`/Contents`
  array concatenation), §7.8.3 (resource dicts), §9.4/§9.4.2 (text
  objects/`Tm`), §9.6.2.2 (Standard-14 no-embed), new-content encoding.
- **16.1 — Boxed text add + wrap via the 15.x reflow engine (core +
  CLI). SHIPPED 2026-08-01 — see Shipped above.** Operator-dragged box;
  multi-line new text wraps through the already-shipped 15.x greedy
  reflow engine (alignment auto/override, top-anchored growth,
  page-overflow disclose-and-allow per R76); `pdfce-cli add-text --box
  "x,y,w,h" --align … --leading …`. **Acceptance:** multi-line new text
  wraps to the box width; alignment applies; overflow disclosed (R76)
  and emitted as real recoverable content; the whole add lands as one
  `CommandKind::AddText`; gates green; fmt/clippy clean. **Non-goals:**
  lists, cross-block flow (FF-B), StructTree insertion (FF-H).
  **Prereqs:** 16.0 + Pass 15.0–15.1.
- **16.2 — Add-text UI on the Pass 12.0 canvas (gui). SHIPPED
  2026-08-01 — see Shipped above.** Click → place point text; drag →
  place a wrap box; type → live preview (via the new pure
  `preview_wrap` core accessor); commit → one `CommandKind::AddText`,
  cancel → nothing. Unambiguous labels distinguishing "add page text"
  from "add FreeText annotation" (the catalog's naming-collision
  finding), delivered as a required tooltip disambiguation across all
  three text-tool tooltips. Disclosures (font provenance, tagged-
  untagged, overflow) surfaced verbatim via 14.3's disclosure channel.
  **`pdfce-ui-specialist`'s design calls, all delivered:** a dedicated
  `CanvasTool::AddText` variant (not overloaded onto the FreeText tool);
  a required tooltip/label disambiguating page-text-add from
  FreeText-annotation-add at the point of interaction, not just in docs;
  a new pure, read-only wrap-preview accessor so box-mode dragging shows
  live wrap feedback without mutating document state ahead of commit.
  **Acceptance:** point-place, box-drag-place, live wrap preview,
  property-bar size/colour/font/alignment, Accept lands one undo-able
  `CommandKind::AddText`, Reject/Esc discards; mutual exclusion with
  `CanvasTool::TextEdit` verified; gates green; fmt/clippy clean.
  **Prereqs:** 16.0–16.1 + Pass 12.0/14.3; `pdfce-ui-specialist` design
  DISPATCHED and SHIPPED.

**Fast-follows beyond FF-D (named, not scheduled):** FF-B cross-block/
cross-page reflow (the exceed-Acrobat headline) · FF-C font-subsetting/
glyph-embedding (**operator-gated**, see Backlog) · FF-H `Tc`/`Tw`/`Tz`/
`Ts` spacing + synthetic bold/italic + minimal StructTree/`ActualText`
update (including a structure element for FF-D's new content) ·
list-authoring (**operator-gated**, see Backlog) · FF-E composite/CJK
new text · FF-F RTL/bidi new text.

**Honest limits named up front (decision 016 §8):** point + single-box
add only (auto-flow across boxes/pages is FF-B) · new content is
UNtagged, disclosed (structure insertion is FF-H) · simple fonts only —
new composite/CJK/RTL runs are FF-E/FF-F · preview fidelity depends on
the bundled/supplied face, disclosed as such · default-font policy is
pdfce's own documented choice (Acrobat's is a GAP) · new text is always
an explicit, reviewable, one-undo operator action, never silent.

**Where pdfce reaches/exceeds Acrobat (decision 016 §9):** reaches
Acrobat's Add-Text baseline (real page content, immediately re-editable)
· **exceeds** on minimal diff (append-a-stream keeps the original
content stream byte-identical; Acrobat rewrites) · **exceeds** on tagged
honesty (disclosed-untagged, never a silent structure-tree corruption)
· **exceeds** with a first-class scriptable `pdfce-cli text add`
(Acrobat has none) · **exceeds** with a documented deterministic
default-font policy where Acrobat's is opaque/undocumented.

**Operator-gated items surfaced by this decision — NOT scheduled here,
see Backlog:** FF-C (font subsetting/embedding) and list-authoring both
require an explicit operator decision before entering a Pass (decision
016 §10). Recommendation on FF-C: unblock in parallel with FF-D — the
operator approves a permissive-only subsetter path and, ideally, settles
`LEGAL.md` §1, so FF-C can follow FF-D; the `pdfce-spec-librarian`
font-subsetting dispatch (named at decision 014) is queued meanwhile.

### Decision 008 sequence — Annotations & markup, sliced (2026-08-01)

**The 6.x annotation arc is COMPLETE (2026-08-01): Pass 6.0 (display) →
6.1 (geometry) → 6.2 (text) are ALL SHIPPED.** The build order advanced
to **Pass 7 (Forms/AcroForm)**, which was split on ship: **Pass 7.0
(field model + text/checkbox fill) SHIPPED**; **Pass 7.1 (flatten +
FDF/XFDF + choice fields + regenerate-all) SHIPPED (2026-08-01) —
completing the AcroForm subsystem CORE.** **Pass 8.0 (Redaction — mark +
apply) SHIPPED 2026-08-01**, completing the decision-008 ranked arc.
Ranking across all candidates was **A ≫ B > C > E > D > F**
(A = Annotations & markup, B = Forms/AcroForm, C = Redaction,
E = Vector/Inkscape-parity, D = Text-&-object editing, F =
Signatures/PAdES). Full reasoning, the read-only pypdf census, and the
structural findings F1–F4 are archived in
`docs/decisions/008-next-subsystem-after-extract.md`. Each Pass below
is scoped into full acceptance criteria (via `pdfce-acrobat-librarian`
+ `pdfce-spec-librarian`) at the moment the engineer reaches it.

**AMENDMENT — decision 010 (2026-08-01, the C→B→A sequence).** With the
whole read → write → edit → extract → annotations → forms → redaction arc
SHIPPED, decision 010 concluded the post-redaction priority call. It
**AMENDS decision 008's revisit-trigger-7** (the clean jump straight to
Pass 9 vector editing after Pass 6.1) into a three-Pass PATH — **Pass 11
(render-fidelity verification, candidate C) → Pass 12 (canvas-interaction
foundation + editing-GUI consolidation, candidate B) → Pass 9 (vector
editing, candidate A, repositioned onto C+B)** — because the accumulated
render-verification and GUI-editing debt must be discharged before the
vector-editing surface is built on top of it. **Decision 008's ranking and
Pass IDs are otherwise intact**; the DESTINATION (Pass 9 vector/Inkscape
editing) is unchanged, only the path to it. Decision-010's candidate
letters A–E are LOCAL to that record and differ from decision 008's A–F.
The build order now runs **Pass 11 (IN PROGRESS) → Pass 12 → Pass 9**; the
Pass 6.0–8 bullets below are the shipped decision-008 arc, retained as the
audit trail.

- **Pass 6.0 — Annotation & widget appearance rendering (read-side).**
  SHIPPED 2026-08-01 — full record in Shipped above.
- **Pass 6.1 — Authored streams + content-stream serializer +
  geometric markup authoring.** SHIPPED 2026-08-01 — full record in
  Shipped above. The project's first content-stream serializer
  (`writer/content.rs`), the R45 staging buffer, `add_markup`, the
  R46 identity gate (12,854/12,936 streams byte-identical, 82
  value-preserved re-spellings, 0 corrupted → PASS), and the
  QuadPoints-Z-order decision all landed.
- **Pass 6.2 — Text-bearing annotations + §12.7.3.3 variable text.**
  SHIPPED 2026-08-01 — full record in Shipped above. FreeText, Text
  (sticky note), Stamp; the shared `vartext.rs` §12.7.3.3 variable-text
  appearance pipeline (the appearance half Pass 7 reuses — R49); the
  bare-Base-14 modality choice and the counted auto-size VT1 heuristic.
  Full-corpus R46 re-run = GATE PASS (all divergences value-preserving
  number re-spellings, 0 corruptions). **No `harfrust`** (R17); Base-14
  LATIN-only. **COMPLETES the decision-008 6.x annotation arc.**
- **Pass 7 — Forms (AcroForm).** Candidate **B, second overall** (30.1%
  organic `/AcroForm` share — the largest real-world demand signal after
  annotations). **Split on ship into 7.0 + 7.1:**
  - **Pass 7.0 — AcroForm field model + text/checkbox fill.** SHIPPED
    2026-08-01 — full record in Shipped above. The `/AcroForm` field-model
    parser (`forms.rs`, field-tree inheritance, dotted FQN §12.7.3.2, the
    field↔widget Shape-A/Shape-B merge R49, `FieldFlags` pinned by test),
    per-type `/V` decode, XFA detect-only, the `/P`-aware fill
    certification gate, and text + checkbox/radio fill through the shared
    §12.7.3.3 appearance generator (R49). CLI `list-fields` + `fill-field`.
    Decision 009 posture A honored (fill touches only `/V`//`/AP`//`/AS`;
    JS carriers re-emit verbatim).
  - **Pass 7.1 — Form flatten + FDF/XFDF + choice fields + regenerate-all
    (COMPLETES the AcroForm subsystem CORE).** SHIPPED 2026-08-01 — full
    record in Shipped above. regenerate-all + clear `/NeedAppearances`
    (R51); Flatten (destructive R48 — the FIRST controlled EXISTING-page-
    content edit, delivered as overlay-APPEND-not-rewrite so R46 keeps ZERO
    flattened-page exceptions); FDF/XFDF import/export (hand-rolled scoped
    XML reader, ZERO new deps); choice-field multi-select; the JS-
    disclosure histogram (decision 009 posture A). Field auto-detection
    ("Prepare Form"), posture-B native recompute (Pass 7.x), and the GUI
    form-fill slice remain the named forms FOLLOW-UP slices (Backlog), NOT
    core.
  - The DISPLAY half of forms IS Pass 6.0 (a widget is an annotation
    first — R49); the APPEARANCE-generation half IS Pass 6.2's
    `vartext.rs` (one appearance pipeline). **Embedded-JavaScript posture
    RESOLVED by decision 009 (2026-07-31): never-execute (posture A =
    Pass 7's whole JS scope; posture B deferred Pass 7.x; posture C
    rejected + prohibited). Standing rules R53–R57 added.**
- **Pass 8 — Redaction.** Candidate **C**. **SHIPPED 2026-08-01 — full
  record in Shipped above (Pass 8.0).** The one truly destructive op:
  advance-preserving content-stream surgery (the R46 named exception, the
  OPPOSITE discipline from Pass 7.1's overlay-append flatten) + container
  decomposition (§5.7) + the absence-proof acceptance gate (R46 inverted).
  Discharged the standing R35 obligation; added R58. GUI apply-button +
  canvas marking DEFERRED to the Pass-12 canvas substrate.

**Decision-010 forward sequence (2026-08-01) — the C→B→A path to the
unchanged Pass-9 destination.** *(REPRIORITIZED 2026-08-01: an operator-
requested measurement/dimensioning BETA — scaled dimensions + vector
selection/snapping + basic vector editing — now holds the In-progress
slot, pulling FORWARD Pass 12 + the first Pass 9 slices under a new
dimensioning subsystem, its architecture in flight as decision 011. The
C→B→A sequence CONTINUES after the beta; Pass 11 (C) is SHIPPED so the
render is verified for the editing work. See the In-progress "Beta" entry.)*

**AMENDMENT — operator prioritization directive (2026-08-01): Acrobat
TEXT-handling parity is inserted AHEAD of the further Inkscape/vector-editing
BREADTH.** Ken (verbatim intent): *"…focus on bringing the software to parity
with Adobe Acrobat's text-handling capabilities such as paragraphs, etc.
Focus on bringing parity with Acrobat first before continuing to build what
Inkscape is better at."* Effect on this sequence:
- **Candidate A's DESTINATION-RANKING is amended, not its ID or its
  survival.** Decision 010 made vector/Inkscape editing (Pass 9) candidate
  **A** — the highest-value post-foundation investment. The operator now
  places **Acrobat text-handling parity ahead of the Inkscape-vector
  breadth** — specifically ahead of Pass 9 slices **(b)–(g)** (boolean ops;
  gradients/shading/transparency; node/Bézier beyond basic; text-to-path;
  OCG layers) and the "Vector graphics editing (Inkscape-parity)" Backlog
  bucket. Pass 9's ID and destination are retained; a new **Acrobat
  text-handling parity** major subsystem is sequenced before the Inkscape
  breadth (full entry + capability list: Backlog, top, "★ NEXT MAJOR
  FOCUS"). Formal record will be KenAgent decision ~014 once the in-flight
  decided work + the Acrobat parity catalog land. **(2026-08-01 update:
  decision 014 is now DECIDED — see the ★ NEXT MAJOR FOCUS — Pass 14.x
  entry at the top of "Next up".)**
- **C and B UNCHANGED; the canvas is SHARED and doubly justified.** Pass 11
  (C) stays SHIPPED; Pass 12 (B) proceeds unchanged — Acrobat-style in-place
  text editing consumes the SAME R60 canvas substrate as the Pass-9 vector
  work, so the canvas foundation is needed either way.
- **Timing.** This focus begins only AFTER the currently-decided / in-flight
  work completes: font-supply (decision 012), Pass 12.0 canvas substrate,
  xref-recovery (decision 013), and the measurement/dimensioning beta
  foundation (decision 011). `pdfce-acrobat-librarian` is cataloging the
  Acrobat "Edit PDF" text-handling parity reference NOW.
- **Beta sequencing FLAGGED (not cancelled):** the beta's Pass-12.0 canvas
  foundation proceeds; the ordering of the beta's remaining
  vector-selection / basic-editing slices (9a / 9c-min) relative to
  Acrobat-text parity is an operator sequencing question to confirm.

- **Pass 11 — Render-fidelity verification harness.** Candidate **C**
  (decision-010-local). **SHIPPED 2026-08-01 — full record in Shipped
  above.** Full-page pdfium/pypdfium2 pixel-parity over the loadable corpus
  (2,890 pages, 0 panics/timeouts); the area-fraction (`frac_over_32`)
  tolerance band derived from the clean-by-construction population
  (band 0.0294); three buckets (2,840 benign / 49 known-gap / **1
  unexplained** = the A019 `f32`-max path-coord render-gap, filed not
  fixed); DeviceCMYK colorimetry the FIRST named residual (→ colour Pass);
  the **R59** gate wired at baseline = 1. **DISCHARGES the long-owed
  Pass-1.1 full-page pixel-parity remainder** (harness generalizes to
  full-page corpus scale; first-page coverage + `--pages-per-file 0`
  multi-page knob). ZERO Rust touched, ZERO new dependency.
- **Pass 12 — Canvas-interaction foundation + editing-GUI consolidation.**
  Candidate **B** (decision-010-local). **NEXT.** RECONCILES the three
  existing named GUI follow-up slices — the **Pass-6.1-followup markup-
  drawing state machine** (`docs/ui_specs/pass-6.1-markup-tools.md`), the
  **Pass-7 form-fill GUI** (`docs/ui_specs/pass-7-form-fill.md`), and the
  **Pass-8 redaction-marking GUI** (`docs/ui_specs/pass-8-redaction.md`) —
  as SLICES on ONE shared canvas-interaction substrate, **NOT three
  independent buckets**. The substrate (built ONCE): a focusable canvas +
  screen↔page transform + tool-mode dispatch + hit-test/selection +
  live-preview overlay — resolving `main.rs`'s Pass-1 focusable-canvas
  caveat. Governed by new standing rule **R60** (exactly one
  canvas-interaction substrate; a second parallel path forbidden). Scope
  into full acceptance criteria via `pdfce-ui-specialist` when reached.
- **Pass 9 — Vector graphics editing (Inkscape parity).** Candidate **A**
  (decision-010-local) = decision-008's candidate **E** — **keeps its
  decision-008 Pass ID (9)**, repositioned AFTER Pass 12. Sliced (a)–(g)
  per decision 008 §5.3 (node/Bézier editing, boolean path ops, stroke/
  fill+gradient editing, numeric transforms, align/distribute, z-order,
  group/ungroup, text-to-path, OCG layers). Promoted onto **Pass 11's
  render-fidelity gate (C) + Pass 12's canvas substrate (B)** plus the
  Pass-6.1 content-stream serializer and the Pass-8.0 surgery interpreter.
  Inkscape is GPL-2.0-or-later, **behavioral reference ONLY** (never a
  dependency, code source, or GUI-mimicry — new standing rule **R61**);
  the **`pdfce-inkscape-librarian` agent + `Inkscape_Features` RAG**
  (`D:\Dev\Rag-Specialized\Inkscape_Features\`) are being COMMISSIONED
  2026-08-01 (closes the previously-unowned Inkscape-catalog FLAG), so the
  capability catalog exists before Pass 9 is scoped into real slices.
- **Pass 5 — Encryption (fallback/interleave track).** Decision-010
  candidate **D**; retains its **decision-007 ID** (Pass 5, never
  renumbered). Stays fallback/interleave (unchanged by decision 010).
  Census payoff is low (0.67% `/Encrypt`, 92.5% legacy R≤4 — Pass 3.0
  organic census, promotion trigger NOT met). Scope unchanged (decrypt ALL
  handlers; encrypt-on-save AES-128/256 ONLY, RC4 never written; crypt
  stage slots into Pass 3.0's serializer seam R37; synthetic
  encrypted-fixture generator a prerequisite, LEGAL §5). Full scoping
  record: the Encryption Backlog bucket entry.
- **Pass 10 — Digital signatures / PAdES.** Decision-010 candidate **E**,
  **unchanged-LAST** (= decision-008's candidate F; 0.64% `/SigFlags`
  organic share — the lowest real-world demand of the ranked set). The
  READ half is already far along (Pass 3.2's `SignatureImpact`
  classification, `/DocMDP`/`/FieldMDP` awareness, the §12.8 spec closure);
  Pass 10 adds PKCS#7 signing + verification, the PAdES B-B/B-T/B-LT/B-LTA
  profiles, and RFC 3161 timestamping. Full scoping record: the "Digital
  signatures" Backlog bucket entry.

### Pass 4 — Text extraction / structured content

**SHIPPED 2026-08-01 — see the Shipped entry at top.** The promotion
record below is retained as the auditable trail; the scoping record
is the Backlog bucket entry ("Text extraction / structured
content"). Promoted from Backlog 2026-07-31 on Pass 3.2's ship
(decision 007 sequence: Pass 4 follows the writer slices; the
`/Encrypt` census promotion trigger for Pass 5 was NOT met). The
`pdfce-spec-librarian` §9.10 sourcing dispatch (`ToUnicode` CMaps,
`/ActualText`, reading order) returned before the engineer ran.
Pre-constrained by R17 (`unicode-bidi` permitted in a
text-extraction reading-order path, forbidden in `pdfce-render`) and
R7; CMap machinery already existed from the CID/`Identity-H` work.
Writer-independent by construction — and shipped without touching
the writer.

*(Restructured 2026-07-31 per decision 007: Pass 3.0 is the ACTIVE
next Pass; 3.1/3.2 are queued behind it. Pass 1.1's open remainders
and the Pass 2.x remainder below stay open in parallel; the Pass
2.1–2.3 scoping texts are retained as shipped audit records.
Updated 2026-07-31 on Pass 3.0's ship: Pass 3.0's scoping text below
is now also a shipped audit record; Pass 3.1 moved to In progress;
Pass 3.2 is next in the queue. Updated again 2026-07-31 on Pass
3.1's ship: Pass 3.1's scoping text is retained below as a shipped
audit record; Pass 3.2 moved to In progress, blocked on the
`pdfce-acrobat-librarian` "Core document ops" dispatch.)*

### Pass 3.0 — Identity writer + round-trip proof harness

**SHIPPED 2026-07-31 — see the Shipped entry at top.** The scoping
text below is retained in place as the auditable record. Both
blockers resolved before code: the write-direction audit returned,
and the hayro-write re-check came back NEGATIVE (0.7.0, 2026-05-27,
self-described internal `pdf-writer` converter, ~580 LoC, no
incremental append — decision 001 §9 trigger 2 does not fire). The
gate closed at **2,898/2,898 = 100.00%** empty-dirty-set whole-file
byte identity over the loadable corpus (denominator re-pinned at
2,898 loadable of 2,914, exactly as the baseline note below
required); `save_full` 2,897/2,898 with the single miss a CORRECT
named hybrid refusal. The parallel `/Encrypt` census RETURNED:
0.67% `/Encrypt` share, 92.5% legacy R≤4 — **promotion trigger NOT
met, Pass 5 stays behind Pass 4** (dated result at the Encryption
Backlog entry).

Scoped by **decision 007** (2026-07-31,
`docs/decisions/007-next-subsystem-after-read-stack.md` — the
full-text authority for everything condensed here; the effective JSON
block in its Appendix A drives implementation). Decision: the next
subsystem is the **incremental-save writer**, sliced 3.0 → 3.1 → 3.2;
candidate ranking A ≫ D > B > C. Pass 3.0 deliberately introduces
**no editing capability**: its entire acceptance bar is a corpus-wide
executable proof of the `ARCHITECTURE.md` §5 round-trip/minimal-diff
invariant, so §11.4's undo obligation does not bind (no mutation) and
every later editing Pass lands against a measured gate instead of a
promise.

**Blockers (both live):**

1. **`pdfce-spec-librarian` write-direction audit — DISPATCHED, in
   flight now.** The §7.5.4 / .5 / .6 / .8 spec-RAG files exist but
   were built for the READ path in Pass 1; confirm they cover
   emission: the exactly-20-byte xref entry rule + permitted EOL
   forms, subsection header syntax, `startxref`/`%%EOF` placement,
   `/Prev` chaining on an appended section, `/Size` semantics on an
   incremental update, xref-stream `/W` `/Index` `/Filter`
   `/Predictor` emission constraints, type-0 free-chain construction,
   the §7.5.8.4 hybrid-file write side, and §14.4 `/ID` (absent from
   the RAG entirely).
2. **Engineer first action: re-check the `hayro-write` changelog** for
   byte-preserving incremental append (decision 001 §9 trigger 2 is
   live). If it has landed, the depend-or-contribute question reopens
   BEFORE this Pass — decision 007 reopens on its own terms.

**Deliverables (10 — decision 007 `first_pass_scope`):**

1. `pdfce-core::writer` — a serializer for every `Object` variant per
   §7.3, byte-exact on the forms that matter: string escaping +
   hex-string form, name `#`-escaping, real-number formatting, stream
   `/Length` agreement.
2. `Document::save_full(path)` — full rewrite. Every
   `Provenance::File` object re-emitted **from its retained source
   bytes verbatim**; only xref, trailer, `startxref` and object
   offsets newly generated.
3. `Document::save_incremental(path)` — the default save mode. With a
   structurally-empty dirty set the output is **byte-identical to the
   input** (not "input plus an empty revision"). Zero edits means
   zero bytes.
4. Xref emission in **both** forms — table (§7.5.4) and stream
   (§7.5.8) — selected to match the input's newest section, never
   normalized (R33).
5. An **object-encoder seam** in the serializer, identity
   implementation this Pass, so the Pass-5 crypt stage plugs in
   rather than being retrofitted (R37).
6. `tools/corpus-report` extended with a round-trip mode; a
   `tools/roundtrip` harness or subcommand.
7. A `parse → write → parse → compare` fuzz target (R40).
8. `pdfce-cli`: a `round-trip` / `save` subcommand exposing both modes
   plus the verification result, with a documented exit-code contract
   distinguishing *not byte-identical* from *failed to reload* from
   *raster differs*.
9. **`ARCHITECTURE.md` §5 amendment** closing the three gaps decision
   007 identified — redaction-forbids-incremental (R35), `/ID`
   discipline (R39), linearization invalidation (R36) — plus the
   §11.2 cross-reference to R35.
10. **No-output-fingerprint enforcement** (decision 001 §6.1
    obligation 6, whose enforcement point IS the writer):
    `save_incremental` NEVER rewrites `/Info` unless the operator
    actually changed metadata; `save_full` sets
    `/Producer = 'pdfce <version>'`, documented and overridable via
    both the pdfce-core API and a pdfce-cli flag. No build hash, no
    edition marker, no non-suppressible producer id anywhere — the
    structural prevention of the exact behavior that disqualified
    oxidize-pdf as a foundation.

**Acceptance criteria (6):**

- Across all 2,892 currently-Ok corpus files: `save_incremental` with
  no edits produces a **byte-identical** file. Target 100%; any
  shortfall enumerated **by file and by reason** — an R20-style
  counted shortfall, never rounded away.
- Across the same 2,892: `save_full` produces a file that (a) reloads,
  (b) contains every `File`-provenance object's definition bytes
  verbatim, and (c) **re-renders to a raster identical to the input's
  render** at fixed DPI — the semantic oracle that is available only
  because the render stack shipped first.
- Byte identity asserted **per object definition** for `save_full` and
  **whole-file** for empty-dirty-set `save_incremental`. Two
  different assertions, never conflated (R32 — decision 007 W1 names
  this the likeliest source of a false green or false red).
- veraPDF §6.1.12 implementation-limits suite run against any new
  writer-side guard, per the existing standing rule (two prior
  intuition-guard incidents: `MAX_TOKEN_LEN`, `MAX_XOBJECT_DEPTH`).
- `cargo fmt --check` and `cargo clippy -- -D warnings` clean;
  `cargo tree -p pdfce-core` shows no GUI dependency.
- A test asserts that `save_incremental` on an unmodified document
  leaves `/Info` byte-untouched, and that `save_full`'s `/Producer`
  is suppressible through both front ends.

**Explicit non-goals — binding, because "while we are in the writer"
is how this Pass doubles:**

- No editing capability of any kind, therefore no undo stack.
- No object deletion, no free-list writing, no generation increments
  (those arrive with Pass 3.1/3.2, tested against real mutations).
- No linearization writing. No optimization. **No normalization of
  anything, ever.**
- No encryption. The seam is built (R37); the implementation is
  Pass 5.

**Parallel cheap task:** the organic-corpus **`/Encrypt` census** — a
read-only scan over a real-world local PDF collection (Ken's own
files) reporting the `/Encrypt` share split by empty-vs-real user
password, by handler revision (R2/R3/R4/R6) and by `/V`. Measurement
only; nothing committed to `fixtures/` (LEGAL §5) — aggregate counts
emitted, files read in place. Its result is the promotion trigger
deciding whether Pass 5 runs ahead of Pass 4 (see the Encryption
Backlog entry). Costs under an hour.

**Raster-oracle note (decision 007 `note_on_candidate_C`):** Pass 3.0
needs a **self-comparison** raster oracle (pdfce-before vs
pdfce-after) — no reference renderer required, roughly a day of work.
That is deliberately NOT Pass 1.1's outstanding reference-renderer
pixel-parity harness (pdfce vs pdfium) and must not be reported as
closing it — but it is most of the same plumbing, so that harness
gets materially cheaper as a side effect. Do not overclaim this as
"candidate C done."

**Baseline note:** the confirmed post-Pass-2.3 corpus baseline is
**Ok 2,892/2,914 (99.2%)** (decision 007 W16's ambiguity resolved by
the record's archival reconciliation note). Re-run
`tools/corpus-report` and re-pin before the round-trip gate is
written if anything ships in between — a gate whose denominator is
uncertain cannot report an honest shortfall.

### Pass 3.1 — Mutation writer + dirty-set diff + undo/redo command log

**SHIPPED 2026-07-31 — see the Shipped entry at top.** The scoping
text below (carried in In progress while the Pass ran) is retained
here as the auditable record. The key acceptance test closed at
**edit → undo → save byte-identical 2,897/2,897 (100%)**; R34's
round-trip gate re-ran unperturbed. One structural deviation:
`save_full` now takes `&DirtySet` — one writer path, with
`DirtySet::empty()` making Pass 3.0's identity behavior a strict
pinned subset. See also the Shipped entry's CRITICAL stale-copy
correction (promoted compressed objects; `ARCHITECTURE.md` §5.7).

First real mutation — `ARCHITECTURE.md` §11.4 binds here: the
command-log undo stack is built in this Pass, not retrofitted.
Mutation surface deliberately the smallest possible: document `/Info`
metadata and page `/Rotate` — dictionary values only, touching no
content stream, no appearance stream, no font — so the Pass tests the
dirty-set machinery, not content re-emission. **Key test: edit → undo
→ save produces a byte-identical file** — §11.1's "union of every
command ever run" bug, made executable. R34's corpus round-trip gate
re-runs; regressions block the Pass. Acrobat RAG not required (still
§5 infrastructure, not an Acrobat-parity feature bucket).

### Pass 3.2 — Structural page operations (Core document ops bucket)

**SHIPPED 2026-07-31 — see the Shipped entry at top.** The scoping
text below is retained in place as the auditable record. The
acrobat-librarian "Core document ops" bucket returned before the
engineer dispatch (unblocking the promotion note below); acceptance
criteria were grounded in it plus the ui-specialist spec
(`docs/ui_specs/pass-3.2-page-ops.md`). One structural deviation of
note: insert shipped as a producer (new-document `assemble()` path),
not an in-place `EditSession` command — GUI Insert deferred, CLI
`insert` complete. Earlier history: **moved to In progress
2026-07-31 on Pass 3.1's ship** (blocked there on the
`pdfce-acrobat-librarian` "Core document ops" dispatch, then in
flight). Original queue note retained:
queued behind Pass 3.1 (decision 007 sequence). **BLOCKED on
`pdfce-acrobat-librarian`: the Acrobat_Features RAG was EMPTY at
promotion** (`LEGAL_NOTE.md`, `_TEMPLATE.md`, `index.md` only) —
dispatch for the "Core document ops" bucket before acceptance
criteria are written. (The feature-fidelity standing rule binds HERE,
not at Passes 3.0/3.1, which are `ARCHITECTURE.md` §5 infrastructure
with no Acrobat behavior for a serializer to match — stated
explicitly so the empty RAG is not read as blocking the whole
sequence.)

### Pass 2.x remainder — JBIG2 robustness follow-up (filed 2026-07-31)

From the Table 12 spec verification (returned 2026-07-31, closing
Pass 2.2's carried open spec item — one key, `/JBIG2Globals`, no code
defects found): §7.4.7 rule 3a (segment page association = 1) is
**should**-only — determine `hayro-jbig2`'s behavior on non-1 page
association (blank-page risk) and on Annex D.2 random-access-organized
input. The `jbig2.rs` doc-comment paraphrase was already corrected the
same day (fmt/check clean).

### Pass 1.1 — Corpus validation & hardening

The honest remainders from Pass 1 (shipped 2026-07-30 — see Shipped).
Not a new feature Pass: this closes Pass 1's ORIGINAL acceptance bar
("renders a legally-clean fixture corpus at pixel parity or documented
near-parity with a reference renderer"), which Pass 1 did not
demonstrate — only synthetic fixtures were rendered.

**Data-driven reprioritization (2026-07-30, continuation 7).** The
corpus measurement ran: `tools/corpus-report` (workspace-excluded
tool; corpora fetched to gitignored `fixtures/external/`) over 2,907
veraPDF + 7 PDF-Association files. ZERO panics, ZERO timeouts; 82.4%
full load + render of page 1. `RefusedXrefStream` = 489 files (16.8%)
= **97.8% of ALL failures** (concentrated in PDF/UA and PDF/A-2/4
subcorpora); `RefusedHybrid` = exactly 1; `MissingResources` = ZERO;
all 11 LoadErrors + 10/11 RenderErrors are deliberate `*-fail-*`
conformance files being correctly rejected. The item order below is
now priority order, set by that data:

1. **xref streams + object streams + hybrid files — DONE
   2026-07-30 (continuation 8).** Cross-reference streams (§7.5.8,
   Tables 17/18 incl. the W field-1 zero-width default, entry types
   0/1/2), object streams (§7.5.7, new `objstm.rs`, per-container
   caching, `/Extends` inert-by-design) and hybrid files (§7.5.8.4,
   `/XRefStm` before `/Prev`, classic-view fallback on breakage).
   **Corpus re-run: veraPDF Ok 2,395 → 2,884 (99.2%)**;
   `RefusedXrefStream` 489 → 0; ALL 24 remaining non-Ok files
   across both corpora are deliberate `*-fail-*` conformance files;
   zero panics/timeouts; 12,927 pages rendered. 280 workspace tests
   (17 new end-to-end PDF-1.5); fuzz smoke 879k execs/60 s, zero
   crashes. **Decision 001 §6.3 harvest gate CLEARED BY MEASUREMENT
   (82.4% → 99.2% actual, vs the <~95% trigger) — no oxidize-pdf
   harvest ever needed.** Scope addition (engineer judgment, flagged
   to operator): encrypted PDFs (§7.6) now refused up front
   (`XrefErrorKind::EncryptionUnsupported`) instead of silently
   rendering ciphertext; 4 corpus files reclassified
   `RefusedEncrypted`. API evolution (`Provenance` enum extending
   the §5 round-trip contract to compressed objects; `XrefEntry`
   `#[non_exhaustive]` + `InStream`) recorded in `ARCHITECTURE.md`
   §12, 2026-07-30 continuation-8 entry.
2. **MAX_TOKEN_LEN lexer guard — DONE 2026-07-30.** The 8 KiB guard
   rejected veraPDF `6-1-12-t02-pass-k.pdf`, a VALID file (PDF/A
   §6.1.12: readers must not impose Annex C limits). Raised to 1 MiB
   with a corpus-cited doc comment; file verified rendering (exit 0).
   The only pass-classified corpus file pdfce mishandled.
3. **The `/Resources`-missing tolerance question — DEPRIORITIZED.**
   Corpus pressure is zero: 0 omissions in 2,914 files. But
   conformance corpora are hand-built to spec, so this does NOT close
   the real-world question — strict mode is corpus-cost-free today;
   the decision waits for organic non-conformance files (Word/
   Chrome/scanner output). OPEN-question lesson (amended with the
   corpus datum):
   `C:\personal_rag\pdf\lesson_20260730_resources_required_but_omitted_open_question.md`.
   `fixtures/synthetic/minimal.pdf` remains unloadable for this
   reason — fixture decision deferred with the policy decision.
4. **Type 3 font rendering / `Tr` 4–7 text-clipping — LOW.**
   Near-zero corpus presence.
5. **Form and Image XObjects (`Do`) + inline images — DONE
   2026-07-30.** Closed the biggest measured fidelity gap (was the
   "Fidelity note" below — now folded into this item). See
   `ROADMAP.md` Shipped, "Pass 1.1 (slice) — Form and Image XObjects
   (`Do`) + inline images": deferred ops 7,347 → 6,079 (−17.3%), 1,168
   forms + 76 images now rendered. Also fixed `MAX_XOBJECT_DEPTH`
   (16 → 64) against a conformant 32-deep veraPDF chain — the second
   guard caught by the §6.1.12 suite, see the new Standing rule below.
6. **Next fidelity slice, corpus-measured priority order — NOT YET
   STARTED.** Item 5's corpus run measured `images_unsupported` (137)
   EXCEEDING `images_rendered` (76) — data, not guesswork, on what's
   next:
   1. **`DCTDecode` (baseline JPEG) — highest-value single item.**
   2. `CCITTFaxDecode` / `JBIG2Decode` / `JPXDecode` / `LZWDecode` —
      remaining unimplemented image filters; re-measure corpus
      frequency before committing to a sub-order.
      **Sub-order SET BY MEASUREMENT 2026-07-30 (decision 005 §3.1)**
      — the re-measure this item deferred has run; scoped as Pass
      2.1/2.2/2.3 below (sub-items 1–2 of this list are superseded by
      those Pass entries; sub-items 3–5 remain unscoped).
   3. `/SMask` + `/Mask` (soft/explicit/colour-key transparency) —
      recognized today, base image drawn opaque with a diagnostic
      note; needs PDF_Spec clause 11, a flagged GAP —
      **dispatch `pdfce-spec-librarian` before starting.**
   4. `Lab`/`Separation`/`DeviceN` colour spaces for images (currently
      DeviceGray/RGB/CMYK/CalGray/CalRGB/ICCBased/Indexed only).
   5. Optional-content (`/OC`) visibility on XObjects.
- **Pixel-parity measurement against a reference renderer** — still
  owed (the run above measured load/render success + honesty
  counters, not pixel parity).
- ~~**`pdfce-gui` file argument** — open a path passed on the command
  line (open-with / drag-onto-exe support).~~ **CLOSED 2026-07-31 —
  shipped as a Pass 3.2 carried item** (see the Pass 3.2 Shipped
  entry; the Pass 3.2 GUI demo launched on a real file through it).
- **R20 GUI diagnostics surface is half done** — the status-bar count
  shipped; revisit the fuller diagnostics panel when editing lands.

**Fidelity note — RESOLVED for XObjects/images by item 5 above
(2026-07-30).** The original note ("3,387 deferred ops, XObjects/
shading = the biggest fidelity gap") is superseded: item 5 closed the
Form/Image XObject and inline-image share of that gap (deferred ops
now 6,079, see item 5's corpus table). Shading (`sh`) operators remain
deferred — not yet scoped into a Pass 1.1 item; candidate for the next
rendering-fidelity slice after item 6's image-codec work. Text-fidelity
counters unchanged in kind: 732 → 1,176 substituted glyphs (the
increase is text inside forms now painting, not a new problem), 303
unsupported fonts, 7 notdefs. `LZWDecode` seen in actual use (item 6.2
above tracks implementing it).

### Pass 2.1 — DCTDecode (JPEG) + LZWDecode — image-codec slice 1

**SHIPPED 2026-07-30 — see the Shipped entry at top.** The scoping
text below is retained in place as the auditable measurement record
(Pass 2.2 references its tables). One figure was later corrected:
the "0 four-component (CMYK/YCCK)" measurement was WRONG — 12 exist
in veraPDF's 6.2.4.3 section; see the Shipped entry and the dated
addendum at the end of `docs/decisions/005-image-codecs.md`.

Scoped by **decision 005** (2026-07-30,
`docs/decisions/005-image-codecs.md` — the full-text authority for
everything condensed here; standing rules R23–R28 bind all three
Pass 2.x slices).

**Priority is set by measurement, recorded here so the order stays
auditable** (decision 005 §3.1/§3.2; all 2,914 `.pdf` files under
`fixtures/external/` — veraPDF-corpus + pdf20examples — scanned
2026-07-30):

- Filter frequency: **DCTDecode 70 files / 79 stream occurrences /
  82.3% of unimplemented-filter occurrences**; **LZWDecode 10**;
  **JPXDecode 7**; **CCITTFaxDecode 0**; **JBIG2Decode 0**. The two
  zeroes are **zero by corpus construction** (conformance corpora
  contain no scanned documents — CCITT/JBIG2 are exclusively
  scanned-document codecs), NOT evidence those codecs don't matter.
- JPEG codestream shape (all 70 DCT codestreams walked
  marker-by-marker): **14% progressive (SOF2)**, **74% carry restart
  intervals (DRI)**, **80% carry an Adobe APP14 marker**
  (transform=1 ×50, transform=0 ×6), **100% 8-bit**, **0%
  4-component (CMYK/YCCK)**, 0 arithmetic/lossless. Consequences:
  "baseline-only is enough" is FALSE (1 in 7 is progressive);
  restart handling is not optional; APP14/`/ColorTransform` default
  rules are the hot path; CMYK rarity is UNPROVEN (conformance
  corpora can't measure it), so CMYK is designed for without
  guessing the inversion rule (R26, decision 005 §5.5).

Scope:

- `zune-jpeg = "0.5"` (`default-features = false, features = ["std"]`
  — dropping `x86`/`neon` makes the crate compiler-enforced
  `forbid(unsafe_code)`, R24) for DCTDecode; `weezl = "0.2"`
  (`forbid(unsafe_code)`, zero required deps) for LZWDecode.
- The **two-tier architecture lands here** (R23, decision 005 §4.6/
  §6.3): `FilterError::ImageCodec(String)` variant; new
  `pdfce_core::image_codec` module (`decode_image` →
  `CodedImage`); LZW joins `filters::decode_stream`'s byte-stream
  cascade (`/EarlyChange` → `weezl` constructor choice, MSB packing).
- §4.1 colour-contract table + APP14 pre-sniff; ceilings set
  explicitly from `MAX_IMAGE_PIXELS`, never inherited — `zune-jpeg`'s
  16,384-pixel default cap MUST be overridden (R25), veraPDF §6.1.12
  run before shipping.
- CI: R24 feature assertion (`cargo tree -p pdfce-core -e features`
  shows no `x86`/`neon`/`simd`); codec crates added to the
  `--duplicates` guard. Fuzz targets per codec;
  `content_and_filters.rs` extended with both LZW `EarlyChange`
  modes. `THIRD_PARTY_LICENSES.md` regenerated in the same commit
  (six new packages — decision 005 §3.6; all permissive, zero
  copyleft). `pdfce-cli` flow ships alongside per standing rule.

**BLOCKING (before code): dispatch `pdfce-spec-librarian` to read
§7.4.8 Table 13 from the source PDF** — `/ColorTransform` exact
wording and defaults; `filter__dct.md` marks Table 13 unverified, and
the colour-routing table rests on it (decision 005 §10 item 1).

### Pass 2.2 — CCITTFaxDecode + JBIG2Decode — image-codec slice 2

**SHIPPED 2026-07-31 — see the Shipped entry at top.** The scoping
text below is retained in place as the auditable record. The blocking
Table 11 defaults verification returned and the implemented defaults
are verified against it (including the `BlackIs1` polarity hazard,
closed by an executable byte-identity proof — see Shipped). One spec
item remains open (Table 12's exact contents — implemented against
§7.4.7's quoted prose; see the Shipped entry's "Open spec item").

`hayro-ccitt = "0.3"` (zero deps, `no_std`, `forbid(unsafe_code)`;
`DecodeSettings` maps 1:1 onto Table 11) + `hayro-jbig2 = "0.3"`
(`default-features = false, features = ["std"]` — `image`/`simd` OFF
per R24; `Image::new_embedded()` is the `/JBIG2Globals` path). One
Pass because one vendor supplies both and `hayro-jbig2` depends on
`hayro-ccitt`. `fax` stays the named fallback / differential oracle.

**Ships on zero corpus pressure, deliberately** (decision 005 §5.1):
both codecs measured 0 in the conformance corpus *by construction*
(see Pass 2.1's table); the demand signal is the OCR and
scanned-document Backlog buckets, for which these are the
precondition. Re-check the moment an organic (non-conformance)
corpus exists. JBIG2 is forbidden in inline images (§7.4.7/§8.9.7) —
the inline path rejects rather than routes.

**BLOCKING (before code): dispatch `pdfce-spec-librarian` to re-read
Table 11** and confirm the `/Columns`, `/EndOfBlock` and `/BlackIs1`
defaults — flagged NEEDS VERIFICATION in `filter__ccitt.md`;
`BlackIs1` is a polarity flag, and a wrong default inverts every fax
image plausibly and silently (decision 005 §10 item 2). Also dispatch
`pdfce-acrobat-librarian` for Acrobat's scanned-image failure
behaviour (missing globals, damaged rows, `DamagedRowsBeforeError`)
so fail-clean thresholds match real expectations (item 20).

### Pass 2.3 — JPXDecode (JPEG 2000) — image-codec slice 3

**SHIPPED 2026-07-31 — see the Shipped entry at top.** The scoping
text below is retained in place as the auditable record — with one
caution: its BLOCKING paragraph's Table 89 parenthetical ("codestream
overrides the image dictionary") states the precedence **BACKWARDS**.
The verified, implemented rule is the reverse: a PRESENT
`/ColorSpace` wins ("any colour space specifications in the JPEG2000
data shall be ignored"); the codestream wins only when `/ColorSpace`
is absent (see the Shipped entry, deviation 1, and test
`jpx_present_colour_space_still_wins`). The blocking §8.9.5/Table 89
verification had already returned before dispatch. With this Pass,
**Pass 2 (decision 005's three slices) is COMPLETE**; the next
subsystem Pass awaits decision 007 (KenAgent consultation in flight).

`hayro-jpeg2000 = "0.4"` (`default-features = false,
features = ["std"]` — `simd` off drops `fearless_simd` where the
unsafe lives; `image` off keeps the `image` crate and `moxcms` out of
`pdfce-core`). The only credible pure-Rust JPEG 2000 decoder; its
known gaps (e.g. progression-order changes in tile-parts) become
named diagnostics per R27.

Last of the three by measurement AND by risk (decision 005 §5.1):
rarest (7 corpus files), largest codec surface, most unverified spec
surface. Note `hayro-*` MSRV = 1.92 = pdfce's exactly, zero headroom
(decision 005 §3.7 — a hayro MSRV bump forces a pdfce MSRV bump).

**BLOCKING (before code): dispatch `pdfce-spec-librarian` to read
§8.9.5 `/SMaskInData` and Table 89's JPX overrides** (codestream
overrides the image dictionary; `/ColorSpace` optional,
`/BitsPerComponent` ignored if present) plus Table 6's
"JPXDecode takes no parameters" attribution — then audit
`pdfce-render`'s image path for any hard requirement on
`/ColorSpace`/`/BitsPerComponent` that Table 89 makes optional
(decision 005 §10 item 3).

## Backlog (Acrobat-parity feature buckets — not yet scoped to Passes)

Grouped by rough Acrobat Pro feature area. Each bucket gets scoped into
real Pass entries as the engineer reaches it — this list exists so
nothing gets forgotten, not as a commitment to build in this order.

- **GUI has no redaction-apply flow at all (filed 2026-08-03, Pass
  17.1/17.2, no Pass number assigned). PROMOTED TO IN PROGRESS
  2026-08-03, then SHIPPED 2026-08-03 as Pass 8.1 (`9a68999`) — see the
  Pass 8.1 Shipped entry (top of Shipped) for the full build record.
  This item is CLOSED.** Original framing retained below (append-only).
  See the new "canvas drag-marking + property bar" bullet immediately
  below for the one piece of the original ui-spec (§2.2/§2.6) this Pass
  deliberately did not build. The GUI can mark redactions and
  disclose the marks, but cannot APPLY a redaction (the operation that
  actually removes covered content) — applying is CLI-only,
  `pdfce-cli redact-apply`. Predates this session but newly notable
  because it's the one R85 operation the new preview-equals-saved oracle
  cannot cover for a structural reason, not an implementation gap:
  applying redaction consumes a `Document` and emits a file directly,
  it is not an `EditSession` operation, so there is no live-session
  state for a GUI "preview then apply" flow to render in the first
  place — any GUI apply flow would need its own design (a distinct
  confirm/apply modal, most likely, not a live-preview surface). See
  the Pass 17.1/17.2 Shipped entry and `ARCHITECTURE.md` §12's
  continuation-58 decision-018 entry for the full architectural framing.
- **Canvas drag-marking + transient property bar for redaction (filed
  2026-08-03, Pass 8.1's ship — `docs/ui_specs/pass-8-redaction.md`
  §§2.2/2.6, no Pass number assigned).** Scope-called out of Pass 8.1
  by name, not silently dropped: the ui-spec's canvas-drag mark gesture
  and its transient property bar were never built — Pass 8.1 shipped
  mark-by-search and the review/apply flow only. The §1.1 canvas-
  interaction-substrate dependency this needed has since landed (Pass
  12.0), so this is now a scope call rather than a block. **Recommended
  approach, per the Pass 8.1 build record:** add a `CanvasTool::Redact`
  variant to the existing canvas-tool enum rather than a parallel,
  bespoke drag implementation — reuses the marquee/drag machinery
  already shipped for selection and dimensioning.
- **`✓`/`✕` glyph verification still owed (filed 2026-08-03, sibling of
  the now-closed menu-affordance/glyph-coverage class).** `✓`/`✕`
  (U+2713/U+2715) on three tools' Accept/Reject buttons were never
  confirmed rendered/not-tofu by direct observation — reaching them
  needs an in-progress tool gesture the observation harness hasn't yet
  driven (dimension authoring, add-text, or reflow mid-flow). The rail
  checkbox's own tick mark is egui's vector-drawn `Checkbox`, not a font
  glyph, so observing it proves nothing about this specific class. Not
  assumed clean by extrapolation from the rest of the glyph-coverage
  audit — needs its own direct-observation check.
  **RESOLVED 2026-08-03, as Pass 18.7** (`09be28d` — see the Pass 18.7
  Shipped entry, top of Shipped, for the full build record and its own
  Pass-number-note: the commit's own subject line wrongly called this
  "Pass 19.4," already taken by the `Tw` Pass, `a1638f4`; corrected to
  18.7 by `pdfce-librarian`, recorded on the branch by commit
  `1111652`). `✓`/`✕` were confirmed genuinely tofu by the new
  automated glyph-coverage gate (a headless static-scan test over
  `ui_text.rs`, not a screenshot-only check) and replaced with the
  confirmed-safe emoji-recommended heavy variants `✔`/`✖`
  (U+2714/U+2716) across all three tools (Edit-Text accept/reject,
  reflow accept/reject, add-text add/cancel). The sibling ui-spec §4.4
  items (`ⓘ` U+24D8 → `ℹ` U+2139; `→`/`↑`/`↓` reworded to words or a
  safer punctuation mark) closed in the same Pass — this item and its
  §4.4 sibling are both fully closed, nothing remains open from the
  `menu-affordance-and-glyph-coverage.md` audit.
- **ui-spec §B.4 follow-on — core data-model additions (filed
  2026-08-03, deviation flagged at Pass 18.1's ship; §C's half of this
  entry SHIPPED 2026-08-03 as Pass 18.4; §B.4's core additions and the
  Alt+click-cycling API SHIPPED as Pass 18.5; the text-bbox-geometry
  item SHIPPED as Pass 18.6 — see amendments below. ALL THREE items this
  bullet ever named are now SHIPPED; only a documentation reconciliation
  in `pdfce-ui-specialist`'s own ui-spec file remains, see the item-1
  amendment below).** Pass
  18.1 shipped the `egui_tiles` dock shell and object/layer tree but did
  NOT deliver **§B.4's core additions to `pdfce-core`** (zero GUI
  dependency added) — extend `TextObject` with a short extracted-string
  preview + resolved font-name/size (P1, high value: Text is the object
  kind most likely to be the "box that doesn't correspond to anything"
  an operator reported, and Pass 18.4 confirmed exactly this case is
  live in `mixed.pdf`); extend `ImageObject` with pixel width/height
  (P1, lower urgency). Still not built as of Pass 18.4's ship. Also
  still owed from §A.2: the `properties_window` → "Document Properties"
  rename in the same slice as the new selection-scoped Properties
  surface (both now exist; confirm the rename actually landed before
  closing this item).
  **AMENDED 2026-08-03 (Pass 18.4 ship) — §C's full selection-legibility
  asks are now SHIPPED, not open.** Pass 18.4 (`be62e48`, its own
  Shipped entry above) delivered the type badge (`P`/`T`/`I`/`F` letter
  badge, since `icons::Icon` has no glyphs for these kinds and
  `Icon::Text` already denotes the text TOOL), invisible/approximate-hit
  disclosure (dashed vs. solid outline, `ObjectNote` variants), the
  status readout, AND the newly-found zero-height-path case (now
  handled by `visible_outline_rect` + `MIN_OUTLINE_EXTENT_PX` inflation
  in screen space). **Three new items surfaced by Pass 18.4, not in the
  original §B.4/§C list, still open:**
  1. **The ui-spec's own text-bbox model (§0.2, §B.3) needs correcting
     by `pdfce-ui-specialist`** — it currently claims the approximation
     is "wider and taller than the ink"; empirically (confirmed against
     `mixed.pdf`) it is narrower than and offset from the glyph ink,
     inflated from glyph ORIGINS by the largest `Tf` size rather than
     from the ink extent. This is a FOURTH contributing cause of "can't
     click on objects," alongside Pass 18.0's zoom-inverted tolerance,
     `c998521`'s missing outline, and `3f6f5ae`'s centring-margin
     coordinate offset. Needs a `pdfce-ui-specialist` re-dispatch to
     correct the spec text; the underlying `pdfce-core` bbox-computation
     behavior itself is not necessarily wrong, just under-disclosed and
     mis-described.
  2. **Status-bar-height / `Fit page`-zoom feedback loop (standing
     hazard, not a Pass 18.4-specific bug).** A bottom-panel height
     change (e.g. the new selection readout growing across frames)
     shrinks the canvas viewport, which under `Fit page` re-fits the
     page smaller on the next frame, which invalidates click coordinates
     the operator just used. Pass 18.4 worked around it locally (a
     one-line headline + `CollapsingHeader` instead of inline sentences)
     but the underlying loop is generic to any egui app combining a
     dynamic bottom panel with a fit-to-viewport zoom mode — applies to
     ANY future status-bar content growth (save notes, edit disclosures,
     warnings), not just this Pass. Escalated to
     `D:\dev\rag\egui\bottom_panel_height_change_retriggers_fit_to_viewport_zoom.md`.
  3. **Owed core API: `hit_test_point_all` (Alt+click cycling through
     overlapping objects).** `pdfce_core::vector::hit::hit_test_point` is
     topmost-only (`objects.iter().enumerate().rev().find(...)`);
     `hit_test_rect` answers a different question (bbox enclosure, no
     tolerance, no nearest-first ordering) and can't substitute. Needs a
     new `pdfce-core` API returning all hits, nearest/topmost-first
     ordered, before Alt+click cycling can be built — not started.

  **AMENDED 2026-08-03 (Pass 18.5 ship, committed `9998a6b`) — items 3
  and the original §B.4 core-data-model additions are now SHIPPED, not
  open; item 1 is now IN PROGRESS, not merely filed.**
  - **Item 3 (`hit_test_point_all` + Alt+click cycling): SHIPPED.** See
    the Pass 18.5 Shipped entry (top of Shipped) for the full build
    record — `hit_test_point` is now structurally defined as the head of
    `hit_test_point_all`'s iterator, not a separate implementation, and
    `CanvasTargetProvider::hit_test_all` mirrors the same shape at the
    GUI trait boundary.
  - **The original §B.4 core additions (`TextObject` extracted-string
    preview + resolved font-name/size; `ImageObject` pixel width/height):
    SHIPPED**, same commit — `TextObject::preview: TextPreview` +
    `TextObject::font: Option<TextFont>`, `ImageObject::pixel_size`, via
    a new `FontResolver` seam (`NoFonts`/`DocumentFonts`). See the Pass
    18.5 Shipped entry for the three disclosed limitations (preview is
    sourced-only text with no derived spacing; `TextFont::size` is the
    raw `Tf` operand, not the rendered size; `TextPreview` is a
    four-variant enum, not `Option<String>`).
  - **Item 1 (ui-spec text-bbox-model correction): SHIPPED, as Pass
    18.6.** **AMENDED 2026-08-03 (Pass 18.6 ship, committed `1b38e34`) —
    the underlying hit-target geometry itself is now fixed**, not merely
    disclosed. `TextObject`'s bbox is now the summed advance widths
    across the run plus `/FontDescriptor` ascent/descent (a four-rung
    metrics ladder), replacing the origin-inflated square. See the Pass
    18.6 Shipped entry (top of Shipped) for the full build record,
    including the two latent decompose-walk bugs (missing `T*` on `'`/
    `"`, untracked `Tc`/`Tw`/`Tz`/`Ts`) found and fixed in passing. **The
    only piece of this item still open is documentation-only:**
    `docs/ui_specs/pass-17-dock-and-layer-tree.md` §0.2/§B.3 still
    describe the OLD (wrong) "wider and taller than the ink" model in the
    present tense, even though the same file's own §E already carries
    the correct one — needs a `pdfce-ui-specialist` pass to reconcile the
    two, not a builder task.
    **RESOLVED 2026-08-03 (commit `67f49bb`).** §0.2/§B.3 were corrected
    earlier the same day to describe the em-box geometry accurately;
    Pass 18.6 then replaced that geometry, so the corrected sections
    became accurate descriptions of behavior that no longer exists —
    a second-order staleness, not the original bug. `67f49bb` marks
    §0.2/§B.3 **historical** (before/after bboxes both recorded,
    `bbox=16,136,44,164` → `bbox=30,147.102,70.46,160.052`) rather than
    deleting them: the em-box geometry is *why* four separate defects
    reached the operator as a single "can't click on objects" report,
    and the analysis that untangled it is what found the other three
    (Pass 18.0 tolerance, Obj-tool outline, coordinate-offset). This
    item is now fully closed — no `pdfce-ui-specialist` work remains
    owed on it.
  - **Item 2 (status-bar/`Fit page`-zoom feedback loop): unchanged,
    still a standing hazard**, not scheduled as its own Pass — see the
    original item 2 text above and the escalated RAG finding.

- **`egui_kittest`-based headless canvas-gesture testing harness (filed
  2026-08-02, no Pass number assigned).** Found while building the GUI
  observation harness (`tools/observe-gui.ps1`/`gui-click.ps1`, Shipped
  2026-08-02, `f2d5fae`): OS-level synthetic pointer input (`SendInput`-
  style automation) partially activates simple controls (toolbar-button
  hover/pressed styling updates) but does NOT satisfy egui's
  `Response::clicked()` on the canvas `Image` widget or other
  custom-`Sense` widgets — verified negative across long/short press
  (140ms/25ms) with and without preceding pointer motion. Root cause:
  egui's click detection is a multi-frame gesture state machine that
  expects input through its own per-frame polling cadence, not
  externally-injected OS messages — see
  `D:\dev\rag\egui\synthetic_os_pointer_input_not_response_clicked.md`
  for the full finding. **Consequence:** the current observation harness
  can verify toolbar-level reachability but cannot drive or verify
  canvas gestures (dimension picking, vector-object drag, node-move,
  marquee-select, etc.) — a real gap for any future "prove the fix
  headlessly" workflow on canvas interactions specifically.
  **Recommended fix:** an `egui_kittest`-based harness, which synthesizes
  `egui::Event`s directly into `RawInput` instead of simulating OS
  input, so the SAME interaction state machine egui uses in production
  evaluates the synthetic gesture as real. `egui_kittest` is already
  referenced as the accessibility-testing path in decision 017's UI
  architecture record (Pass 18.x) — this Backlog item is the concrete
  "also needed for canvas-gesture QA, not just accessibility" case for
  adopting it. Not yet scoped into a Pass; a new dependency (`egui_kittest`
  itself) would need the standard rule-13 license classification before
  landing in `Cargo.toml`, even though it would likely be a dev-only
  dependency (test/tooling surface, not shipped in the release binary).

- **`-Pid` parameter needed on `tools/gui-click.ps1` and
  `tools/observe-gui.ps1` (filed 2026-08-03, no Pass number assigned) —
  RESOLVED 2026-08-03, committed `f45d8d6`.** Both scripts previously
  selected their target process via `Select-Object -First 1` over the
  process name, with no way to disambiguate two simultaneously running
  instances. Shipped as an **`-ProcessId`** parameter (deliberately not
  `-Pid` — `$Pid` is a PowerShell automatic variable and shadowing it
  fails confusingly) that both scripts now REFUSE to proceed without
  when several candidate processes exist, listing the running ids. See
  the Shipped entry (top of Shipped) for the verification record.

- **★ UX flag — marquee-vs-pan canvas-drag default change at Pass 9a —
  RESOLVED, KEPT (reviewed at Pass 12.M1, 2026-08-01).** Pass 9a's
  shipped selection model repurposed the Pass 12.0 canvas's plain-drag
  gesture: in no-tool selection mode, drag is now a rubber-band
  **marquee select** (fully-contained-by-default, with a `Touched`
  mode available), and **pan moved to wheel/scrollbars** — the
  Inkscape/Illustrator convention. This was a real CHANGE from the
  behavior Pass 12.0 shipped (plain drag-to-pan, standing rule R61's
  original framing), made as an engineer judgment call during Pass
  9a's build. **`pdfce-ui-specialist` reviewed it during the Pass
  12.M1 dispatch and found no conflict with the dimensioning tool:**
  Measure/dimension tools use a click-point-A-then-click-point-B
  interaction, not drag, so marquee-select-drag and dimension-picking
  never contend for the same gesture. Verdict: keep the Pass-9a
  default as-is, no further change. See the Pass 12.M1 Shipped entry
  above for the full record. This flag is now closed — no outstanding
  action.
- **Test-hygiene — integration-test temp-path collision risk (low
  severity, non-blocking, filed 2026-08-01). FIX IN PROGRESS (as of
  2026-08-01, SESSION_LOG continuation 54).** Some integration tests
  build temp output paths from `std::env::temp_dir()` +
  `process::id()`; `process::id()` is only unique per OS process, not
  per-thread, so two tests racing inside the same `cargo test`
  process (the normal parallel-test default) can collide on the same
  path if their other disambiguating tag/counter also matches. **Now
  observed by TWO separate builders** — the Pass 9a ship (one
  transient, non-reproducing intermittent `RecoveredBaseForbidsIncremental`
  failure under a full parallel `cargo test --workspace` run, not
  reproducing on a clean main-tree run) and, independently, the Pass
  12.M2b build. **Still not a product bug, not blocking any ship** —
  a bounded test-hygiene fix (make integration-test temp paths globally
  unique, e.g. an `AtomicUsize`/`AtomicU64` counter appended alongside
  the PID) is now dispatched. No Pass number assigned; tracked as an
  opportunistic hardening item until it ships, at which point the
  librarian should record the fix here or promote it to a Pass entry
  if it turns out to need one.

- **★ NEXT MAJOR FOCUS — Acrobat text-handling parity (Edit PDF: in-place
  text edit, paragraphs, reflow, formatting, font-on-edit)** — filed by
  OPERATOR PRIORITIZATION DIRECTIVE (Ken, 2026-08-01). Verbatim intent:
  *"Continue the autonomous loop, but when you finish doing the decided
  work, focus on bringing the software to parity with Adobe Acrobat's
  text-handling capabilities such as paragraphs, etc. Focus on bringing
  parity with Acrobat first before continuing to build what Inkscape is
  better at."*
  - **WHEN it starts — after the currently-DECIDED / IN-FLIGHT work
    completes:** (i) operator-supplied **font-supply** (decision 012,
    building now); (ii) the **Pass 12.0 canvas-interaction substrate**
    (decision 010 candidate B / beta slice — being designed→built); (iii)
    **xref-recovery** (decision 013, in KenAgent consultation now); and (iv)
    the **measurement/dimensioning beta foundation** (decision 011). This
    directive does NOT interrupt that decided work — it defines the NEXT
    major subsystem after it.
  - **WHAT it is — a NEW major subsystem: editing the document's own page
    TEXT CONTENT.** Distinct from two shipped things it must NOT be confused
    with: the shipped text **EXTRACTION** path (Pass 4) and the text-bearing
    **ANNOTATIONS** path (Pass 6.2 — FreeText/Stamp overlays authored ON TOP
    of the page). This is Acrobat "Edit PDF": select and re-edit the glyph
    runs already in the page content stream. Capability set (to be grounded
    against the Acrobat parity RAG — see status below):
    - **In-place text editing** — select existing page text, change it, the
      page content stream is rewritten minimal-diff (§5 discipline; this is
      content-stream surgery, kin to the Pass-8.0 redaction interpreter, not
      the Pass-6.x overlay-append path).
    - **Paragraph / text-block recognition** — reconstruct logical
      paragraphs and text blocks from positioned glyph runs (fuzzy, never
      sneaky: a reviewable inferred structure, never a silent re-layout).
    - **Reflow** — re-wrap a paragraph's lines when its text or box changes.
    - **Text formatting** — font family / size / style / colour / spacing on
      edited runs.
    - **Font-handling-on-edit** — the hard sub-problem: an edited run needs
      glyphs the embedded subset may not contain; couples directly to
      decision 012's operator-supplied-font supply and the R17 base-14
      Latin-only limit. Limits (subset-missing glyphs, complex-script/RTL
      shaping, vertical text) get named up front, not discovered late.
  - **PRIORITIZATION — ahead of the further Inkscape/vector-editing
    BREADTH (amends decision 010's destination-ranking).** This LEAPFROGS
    decision 008's Pass 9 vector-editing slices **(b)–(g)** (boolean ops;
    gradients/shading/transparency; node/Bézier beyond basic; text-to-path;
    OCG layers) and the general **"Vector graphics editing
    (Inkscape-parity)"** Backlog bucket below. Decision 010 ranked
    vector/Inkscape editing candidate **A** (the highest-value
    post-foundation investment); the operator now places **Acrobat TEXT
    parity AHEAD of the Inkscape-vector breadth**. See the dated amendment
    note in the decision-010 forward-sequence block (Next up) — this is a
    ranking amendment, not a cancellation: the Pass-9 destination survives,
    repositioned behind Acrobat-text parity.
  - **Decision 010's C and B are UNAFFECTED — and the canvas is SHARED.**
    Candidate C (render-fidelity verification, **Pass 11**) is SHIPPED;
    candidate B (**Pass 12** canvas-interaction foundation) is in progress.
    Acrobat-style in-place text editing needs the **same interactive canvas
    substrate** (focusable canvas + screen↔page transform + hit-test /
    selection + live-preview overlay, R60), so Pass 12 is **doubly
    justified** and proceeds unchanged. Acrobat-text parity is a CONSUMER of
    that substrate, exactly like the Pass-9 vector work.
  - **Beta (decision 011) — foundation proceeds; remaining-slice ordering is
    an OPERATOR SEQUENCING QUESTION (flagged, NOT cancelled).** The beta's
    SHARED foundation (Pass 12.0 canvas) proceeds. The beta's own
    dimensioning slices are unaffected; but its **vector-selection /
    basic-editing slices (9a / 9c-min)** are Inkscape-adjacent, so their
    placement RELATIVE to Acrobat-text parity is a sequencing call for the
    operator to confirm (the beta is Ken's stated "first beta," which argues
    for finishing it; the directive argues Acrobat-text ahead of
    Inkscape-vector work). **Do NOT cancel the beta — flag the ordering.**
  - **STATUS — being teed up NOW (rule-12 discipline).**
    `pdfce-acrobat-librarian` is cataloging Acrobat's **"Edit PDF"
    text-handling** capabilities (in-place edit, paragraph/reflow,
    formatting, font-on-edit, limits) at `D:\Dev\Rag-Specialized\
    Acrobat_Features\` — the parity reference. Once that catalog **and** the
    in-flight decided work (decisions 011/012/013 + Pass 12.0) land, the
    Acrobat-text-editing **ARCHITECTURE routes through KenAgent** (a future
    decision, ~014) before any build — parity reference → scope → KenAgent
    decision → build, per rule 12.
  - **AMENDMENT (2026-08-01) — decision 014 DECIDED; Pass 13.x renumbered
    to Pass 14.x.** The KenAgent decision this bucket named as "~014" is
    now filed: `docs/decisions/014-acrobat-text-editing.md`. Full Pass
    slicing, the six standing rules (assigned **R69–R74**), and the
    fast-follow ladder are recorded in the new **★ NEXT MAJOR FOCUS —
    Pass 14.x** entry at the top of "Next up" (this Backlog bucket now
    serves only as the historical directive record — do not duplicate
    scope here). Decision 014's own proposed "Pass 13.x" label is
    superseded: 13a/13b were already assigned to xref recovery (decision
    013) by the time decision 014 was filed, so the text-editing family
    is **Pass 14.0–14.3**.
  - **AMENDMENT (2026-08-01) — Pass 14.3 SHIPPED; decision 014
    COMPLETE end-to-end.** All four slices (14.0 model, 14.1 edit, 14.2
    format, 14.3 GUI + the `EditSession` undo-integration prerequisite)
    have shipped — full records in `ROADMAP.md` Shipped. This bucket's
    directive is now **substantially achieved at the P0 level**; the
    active thread is the reflow ladder. `pdfce-acrobat-librarian` has
    scoped **FF-A** (within-block reflow) at
    `D:\Dev\Rag-Specialized\Acrobat_Features\text_edit__paragraph_reflow_and_auto_adjust_layout.md`;
    a KenAgent decision (**015**) is being taken to settle its
    architecture, including an unresolved question about whether
    Acrobat's base-panel Justify button implies justified alignment is
    achievable at FF-A rather than gated on FF-B — see the ★ NEXT MAJOR
    FOCUS entry under Next up for the full note. Do not duplicate that
    scope here.
  - **AMENDMENT (2026-08-01) — decision 015 DECIDED; the justified-
    alignment question is RESOLVED, not left open.** Decision 015
    (`docs/decisions/015-ffa-within-block-offline-reflow.md`) settles
    the question named directly above: Acrobat's base-panel Justify is
    a classic-engine (offline, single-block) capability, so justified
    alignment moves OUT of FF-B and INTO FF-A — it ships as a fourth
    within-block alignment mode alongside left/center/right, and
    amends decision 014 accordingly. FF-A's build is scoped as the new
    **★ Pass 15.x** entry (Next up); this bucket again serves only as
    the historical directive record — do not duplicate scope here.

- **OSS real-world sweep — ranked robustness gaps (filed 2026-07-31 from
  the +1,109-file / ~4,000-total corpus sweep).** The empirically-ranked
  real-world compatibility gaps, in priority order (counted, not rounded):
  1. **xref recovery — the dominant real-world gap, NOW SHIPPED
     (2026-08-01, decision 013 Pass 13b — see Shipped).** 566 previously
     -failing real-world files now open (corpus of 1,109: qpdf 639 /
     pdfium 331 / PDFBox 139); zero regression on the 2,907-file veraPDF
     corpus. **New Backlog sub-item filed by this Pass:** object-level
     lenient loading — 53 real-world files load a valid xref (strict or
     recovered) but carry corrupt/malformed OBJECT-level data downstream;
     a documented non-goal of Pass 13b, not yet scoped to a Pass.
  2. **Encryption — ~5% (empty-password permissions).** DOUBLY confirmed
     low-payoff: the corpus `/Encrypt` sweep (~5%, 92.5% legacy R≤4) PLUS
     the operator's stamped-drawings context both point the same way. This
     is the deferred **Pass 5**, awaiting operator go-ahead (the promotion
     trigger is still NOT met). Cross-refs the Encryption Backlog bucket.
  3. **`/Resources` omission tolerance** — real readers treat a missing
     page `/Resources` as empty rather than failing; a Pass-1.1 tolerance
     question, now quantified by the sweep.
  4. **LZW "invalid code" (EarlyChange handling)** + content-lexer
     strictness edge cases — a cluster of strict-vs-lenient decoder edges.
  5. **More font-subtype gaps** — Type3, vertical writing, CID-no-cmap, and
     a couple of real embedded fonts showing `notdef` glyphs. Closest kin
     to the just-fixed NUL-misroute font bug (R68) — worth a look.
  6. **Undecodable images** — CalRGB colour space, PNG-predictor cases.
  - **Separate future tolerance decision — `gen-65536`:** 17 files carry an
    out-of-spec generation number > 65535 (**NOT CRLF-related**; surfaced by
    Pass 13a). pdfce correctly rejects them strict today. A future tolerance
    decision + a `personal_rag/pdf` finding — distinct from xref recovery.
  - **`font_program` fuzz target** — no dedicated fuzz target exists for the
    font-program parser/router yet (residual named by the NUL-misroute font
    fix). Add one covering the magic-detection/routing path R68 guards.
- **Render-fidelity residuals (filed by Pass 11, 2026-08-01)** — the two
  named residuals the render-parity harness surfaced but deliberately did
  NOT fix in-Pass (measurement-only non-goal Y3):
  - **Out-of-range path-coordinate robustness (`pdfce-render`)** — a path
    vertex at ≈ `f32::MAX` (x ≈ 3.4028e38) under the CTM makes pdfce
    rasterize a spurious fill (the `A019-pdfa2-pass-a.pdf` cyan bar; the
    single unexplained Pass-11 divergence, counted render-gap R20/R27);
    pdfium rejects/clips it. Fix = a clamp/reject-policy decision in
    `pdfce-render` (R34 identity risk — the round-trip gate must stay
    green), Pass-9-adjacent (vector editing re-renders edited paths, so
    this robustness question resurfaces there). Not cheap-clear; scope
    deliberately.
  - **DeviceCMYK → sRGB colorimetry colour Pass** — DeviceCMYK-only pages
    diverge 3.0× the clean-page mean corpus-wide (naive additive
    `Rgb::from_cmyk` vs pdfium's `AdobeCMYK_to_sRGB1`; polarity IDENTICAL,
    R29 holds — a colour-accuracy gap, not an inversion bug). Adopt a
    calibrated CMYK→sRGB table. **Re-pin decision 006 §3.4's polarity
    matrix BEFORE any colour change lands** (006 revisit-trigger 7; don't
    confound colour with the harness/polarity work). Scope via
    `pdfce-acrobat-librarian`'s already-filed "what does Acrobat do for
    uncalibrated DeviceCMYK → screen" question. Graduation path is decision
    010 revisit-trigger 6 (if it dominates the corpus "unexplained" bucket
    it becomes its own scoped colour Pass — Pass 11's measurement puts it
    at the FIRST named residual, one file, so a bounded colour Pass, not an
    emergency).
- **GUI crash-safety / non-destructive-save gap (filed by the GUI-polish
  interlude, 2026-08-01) — NOT polish, a standing-UX-rule gap** (ui-specialist
  territory: crash-safe autosave + non-destructive-by-default). Two coupled
  items, both still open:
  - **Autosave / crash-recovery scratch file** — none exists today; an
    unsaved editing session is lost on a crash. Needs a periodic scratch
    write of the in-memory command log / edit session to a recovery file,
    detected + offered on next launch (fuzzy-never-sneaky: offer to recover,
    never silently apply). This is the PREREQUISITE for the next item.
  - **True in-place Save** — deliberately still GATED on the autosave/
    recovery mechanism existing; until then **"Save a copy" remains the only
    save affordance** (correct, conservative behavior — never overwrite the
    source without a recovery net). Unblocks once autosave lands.
- **GUI-polish residuals (filed by the GUI-polish interlude, 2026-08-01) —
  cosmetic/usability, low priority:** P2-1 recent-files list (needs settings
  persistence); P2-2 window/taskbar app-icon asset (needs artwork); P2-3
  light-mode visual QA pass (no hardcoded colours added — stays
  OS-theme-driven); P2-4 markup colour-picker tooltip; P2-5 screenshot-driven
  spacing QA.
- **Core document ops** — merge/split/extract/insert/delete/rotate/
  reorder pages; page-size & rotation normalization.
- **Text & object editing** — in-place text edit with font re-flow,
  image replace/move/resize, vector object edit. Basic object editing
  lives here; deep vector editing (node-level paths, booleans,
  gradients, layers) is the adjacent **"Vector graphics editing
  (Inkscape-parity)"** bucket below — scope them together when either
  is reached.
- **Vector graphics editing (Inkscape-parity)** — scope expansion
  filed 2026-07-30 (operator's words: "I want it to have all of the
  capabilities to edit pdfs that inkscape and acrobat pro does").
  Acrobat Pro parity was already the target; the NEW part is
  Inkscape-level vector editing of PDF page content. Bucket-level
  scope, not yet Pass-scoped:
  - Node/Bézier path editing of content-stream path objects.
  - Boolean path operations: union, difference, intersection,
    exclusion, division.
  - Stroke & fill property editing, including gradients (axial/radial
    shading) and patterns.
  - Object transforms (move/scale/rotate/skew) with numeric precision
    entry.
  - Alignment/distribution tools.
  - Z-order (raise/lower) manipulation within the content stream.
  - Grouping/ungrouping.
  - Text-to-path conversion.
  - Layer support via PDF Optional Content Groups (OCG).

  Binding notes for whoever scopes this into Passes:
  (a) **Inkscape is GPL-2.0-or-later — behavioral reference ONLY,
  never a dependency or code source** (same standing rule as
  MuPDF/Ghostscript in `PRIOR_ART.md`). (b) This scope raises the bar
  on `pdfce-core`'s content-stream model — it must support full
  round-trip decomposition of graphics operators into editable objects
  and minimal-diff re-emission per `ARCHITECTURE.md` §5. (c) Scoping
  this bucket into real Passes will need a feature-parity catalog of
  Inkscape's editing capabilities — capability/behavior only, never
  its GUI mechanics — same discipline as the Acrobat Features RAG.
  (d) **Decision 007 fold-in (2026-07-31, Pass 6+ decomposition):**
  shading patterns and transparency groups / soft masks — the unfiled
  remainder of decision 007's candidate C ("render completeness") —
  fold INTO this bucket rather than shipping as a standalone
  render-fidelity Pass: gradients ARE axial/radial shading, and
  implementing them twice is waste. The rest of "render completeness"
  is already filed elsewhere, not new work: Type 3 fonts + `Tr` 4–7
  text clipping = **Pass 1.1 item 4** (LOW, near-zero corpus
  presence); `/SMask` + `/Mask` = **Pass 1.1 item 6.3** (blocked on a
  clause-11 spec dispatch). Only general transparency groups and
  blend modes were scoped nowhere — they now live here. The
  reference-renderer pixel-parity harness stays a Pass 1.1 remainder,
  made materially cheaper by Pass 3.0's self-comparison oracle (see
  Pass 3.0's raster-oracle note).
- **Object-tool selection, level navigation & ce-dimension authoring
  controls — Pass 22.0 (decision 022) / Pass 23.0–23.3 (decision 023).
  Filed 2026-08-04 (continuation 80). Status: both DECIDED, SCOPED,
  NOT STARTED — no code has been written for either.** Archived at
  `docs/decisions/022-annotations-in-canvas-selection.md` and
  `docs/decisions/023-object-tool-level-navigation-and-dimension-authoring-controls.md`.
  Pass families **22** and **23** verified free at both decisions'
  own write time (`python tools/check-ledger-numbers.py --stats`,
  per each document's own preamble) and re-confirmed by this filing
  via a live Grep of `ROADMAP.md` for `Pass 22`/`Pass 23` — no
  `### Pass 22` / `### Pass 23` heading existed anywhere in this file
  before this entry. **Methodology note: this librarian has no
  Bash/shell tool and could not execute
  `tools/check-ledger-numbers.py` directly this filing** — the
  confirmation above is a Grep against the same source file the
  checker reads, not a script run; recorded so a future reader does
  not assume the checker itself was invoked.

  **Origin.** The operator reported box-select failing on "dimension
  lines and dimensions" in a CAD drawing (2026-08-03/04). Investigation
  found **two independent, structurally identical defects**, not one:
  1. **Decision 022** — ce dimensions (`/Line`+`/IT /LineDimension`
     annotations pdfce itself authors) are painted by the renderer but
     never enumerated by the selection model (`decompose_page` never
     reads `/Annots`), so they cannot be clicked, marqueed, **or
     deleted by any surface at all** — a second, more severe defect
     found during the investigation: there was no way to remove a ce
     dimension by any means whatsoever, canvas, Objects panel, or CLI.
  2. **Decision 023** — the SAME paint/select asymmetry exists a
     second time, independently, in **form XObjects**:
     `pdfce-render` recurses into a `Do` on a form and paints its
     contents individually; `decompose.rs` emits **one opaque object**
     for the whole form. Every line inside a placed CAD block (a title
     block, a hatch, a nested drawing block) is visible and
     unselectable.

  **★ Restated at the point of filing, because it changes what
  "closes the original report" means:** per Open operator question
  (t), below, the operator's original complaint was almost certainly
  describing a CAD-exported drawing — **pdf dimensions**, not ce
  ones. If so, **Pass 22.0 alone does not fix what was reported.**
  22.0 fixes a real, independently-confirmed defect (ce dimensions
  were never selectable or deletable, a genuine gap the operator had
  not yet hit) — but it is Pass 23.2 (level navigation, dependent on
  22.0), not Pass 22.0, that closes the form-XObject asymmetry most
  likely responsible for the literal report. Both are worth building
  regardless; only one is the literal fix. **Not yet confirmed** which
  the operator's drawing actually contained — see (t) for the
  recommended one-question/one-file-open confirmation step.

  **Sub-slice plan and dependency graph:**
  - **Pass 22.0** (decision 022) — three sub-slices: **22.0a** core
    (`pdfce_core::annot::is_visible_on_screen`/`selectable_annotations`
    extracted from the renderer's own predicate, one definition two
    consumers; `EditSession::delete_annotation`/`delete_dimension`,
    ce-dimension-aware — prunes the `/PieceInfo` sidecar in the same
    command) → **22.0b** CLI (`annot-list`, `annot-delete`,
    `dimension-delete`, `dimension-list` widened to print `annot=`,
    `object-list --enclose` for the marquee's first-ever headless
    oracle) → **22.0c** GUI (`TargetId` widened `u64` → a two-variant
    enum `{Content(u64), Annot(ObjId)}`; composite `ObjectModelProvider`;
    **select + delete only** — move/drag-node are structurally absent
    from the `Annot` arm, so there is no affordance and no unfireable
    refusal). Full acceptance criteria: decision 022 §6.4 (A1–A9).
  - **Pass 23.0** (decision 023, Q6) — format/units GUI surface. **Zero
    dependency on 22.0.** `EditSession::set_group_format` (decoupled
    from scale — today format can only change by re-drawing a scale
    line); CLI `group-set-format --style decimal|fraction --denominator
    N --reduce`; GUI group-row controls + a live sample of the
    operator's own data before Apply. Closes **two dead-capability
    defects** found during the same audit: `NumberFormat::inch_fraction`
    and `FractionMode::Fraction{reduce:true}` are both implemented,
    documented with runnable examples, spec-mirrored into `/Measure` —
    and constructed from **nowhere outside a unit test**. No operator,
    GUI or CLI, can produce a fraction-formatted ce dimension today. **Must
    also fix the already-filed Pass 12.M2c bug #1** (ratio-scale entry
    silently overwrites the group's display unit) in the same slice, or
    the new control ships looking broken from day one — see the
    Pass 12.M2c cluster entry, above, bug 1, which now cross-references
    this Pass.
  - **Pass 23.1** (decision 023, Q5) — ce-dimension re-measure +
    whole-dimension move. **Depends on 22.0** (reuses its
    sidecar-pruning discipline and guarded-`/AP`-write fix — building
    23.1 first would force both to be re-derived under pressure).
    `EditSession::set_dimension_geometry`; Measure-tool endpoint
    handles on a selected ce dimension; a two-stage disclosed gesture
    (old→new value + delta + group/scale visible live, mouse-up never
    commits, Accept/Reject). See the Glossary's new "Obj-tool
    universality" entry, above, for the reconciliation that makes this
    buildable without reopening decision 022 §4.2.
  - **Pass 23.2** (decision 023, Q1–Q3) — level navigation: containers
    (a form-XObject invocation OR a balanced marked-content sequence,
    taken as a union), descend one level per double-click, ascend via
    Escape (`LeaveGroupLevel`, a new precedence-chain slot between
    `CancelGesture` and `ExitTool`) or click-outside (nearest common
    ancestor), a breadcrumb that is simultaneously the level disclosure
    and the ascent affordance. **Depends on 22.0** (`TargetId` enum
    widening; payload grows to `ContentPath`, no further substrate
    change — a direct dividend of 022 choosing the enum over the
    tagged-integer alternative it rejected). **READ-ONLY — writes
    nothing**, the largest slice at zero round-trip risk. **This is the
    slice that closes the form-XObject paint/select asymmetry** (§0
    finding 2) — the candidate fix for the operator's literal original
    report, per question (t).
  - **Pass 23.3** (decision 023, Q4) — node selection set, multi-node
    move (**ONE** core plan, `plan_move_nodes`, not N sequential
    `move_node` calls), node delete (straight-join semantics, curvature
    disclosed-as-lost). Named, reachable, R96-tested refusals:
    `SubpathWouldDegenerate`, `RectangleNode`, `ImplicitNode`,
    `NotAPath`, `FormStreamIsShared`. `DegenerateCtm` must **NOT** be
    raised for node delete — deletion needs no coordinate transform, so
    no such failure exists; raising it anyway would be a refusal no
    test could honestly reach. A node-display ceiling with disclosure
    (never a silent first-N) is required — `MAX_NODES` is 4,000,000 and
    a plotted drawing routinely carries tens of thousands of anchors.
    **Depends on 23.2** (level 3 is the level below level 2).

  **Librarian's read on slice ordering, asked for explicitly by the
  operator, not just filed as a fact:** decision 023 §7.1 orders **23.0
  first**, and its stated reason is entirely an **engineering-risk**
  argument — zero hierarchy risk, no dependency on 022, a clean CLI
  oracle. That reasoning is sound on its own terms, but it is silent on
  **user impact**, and the terminology audit that produced Open
  operator question (t) changes the user-impact picture materially: the
  operator's actual reported pain (box-select failing on a CAD drawing)
  is very likely a **pdf-dimension**/form-XObject problem that **neither
  22.0 nor 23.0 touches** — only 23.2 (which depends on 22.0) does.
  Shipping 23.0 first means the first thing that ships from this whole
  body of decided work is a **ce-dimension** format-precision control,
  while the reported bug remains unfixed. I do not think that is the
  right ordering if the goal is "fix what was reported soonest" —
  **22.0 → 23.2 is the fix-oriented order**, and it is also the order
  decision 023's own dependency graph already requires for 23.2
  regardless of where 23.0 lands, so putting 23.0 first buys engineering
  safety at the cost of sequencing, not in exchange for it. My
  recommendation: resolve question (t)'s confirmation (open the
  operator's file, or ask — near-zero cost) before committing to a
  build order at all, since it is what actually determines whether
  22.0 alone or 22.0+23.2 is the fix; treat 23.0's zero-risk,
  zero-dependency shape as a reason it is a safe thing to build **in
  parallel or while waiting on that confirmation**, not as a reason to
  sequence it strictly ahead of the fix. This is a recommendation, not
  a decision — sequencing calls of this shape have consistently been
  left to the engineer/operator on this project (see the item (l)/(m)
  precedent, Open operator questions, below).

  **Standing rules assigned:** R111–R120 (see Standing rules, below,
  for full text and the two amendments 023 makes to 022's proposed
  rules). Ceiling before this filing was R110; **ceiling is now
  R120.** **New open operator questions filed:** (u)–(ab) — see Open
  operator questions, below.

  **Cross-references:** Open operator question (t) (the scoping
  question this whole bucket exists to answer); the "Ce-dimension-tool
  bug-fix cluster — Pass 12.M2c" entry, above (bug #1 coupling with
  Pass 23.0); the "Vector graphics editing (Inkscape-parity)" bucket,
  above (Pass 23.3's node selection/multi-move/delete is that bucket's
  "Node/Bézier path editing" line item, now Pass-scoped for the first
  time); `ARCHITECTURE.md` §12 (dated decision-022/023 entries) and new
  §5.12 (the R58-family-membership clarification these decisions
  surfaced, independent of whether either Pass ships).

  **No `ARCHITECTURE.md` §3/§4 body-section update this filing** — same
  disposition as decisions 020's and 021's own entries (`ARCHITECTURE.md`
  §12): nothing has shipped, so §4 (core data model) describes no new
  reality yet; §4 gets its `PageObjects.containers`/`Container`/
  `ContainerKind` row added when Pass 23.2 actually lands, not now. The
  one exception, by the same logic as decision 020's decision-009
  forward-pointer exception: `ARCHITECTURE.md` §5.9/R58's text is
  **already** stale against **already-shipped** code
  (`delete_object`/Pass 9c-min, `delete_redaction_mark`/Pass 8) — that
  is a correction to a currently-true statement, not a preview of a
  future one, so it is made now (see the new §5.12 note) rather than
  deferred to either Pass's ship.
- **Forms (AcroForm)** — field creation/editing, appearance-stream
  generation, form-field auto-detection (as a *hint*, per
  fuzzy-never-sneaky), flatten-to-static.
  **AMENDED 2026-08-01 (SESSION_LOG continuation 50) — "form-building
  tools" is operator priority #4, queued after the dimensioning tool /
  icon set / text-handling (see the ★★★ Operator priority sequence,
  top of Next up).** This is **field CREATION/authoring** — adding NEW
  AcroForm fields (text/checkbox/choice/etc.) to a document — which is
  a genuinely different subsystem from the already-shipped Pass 7.0
  (field model + fill) and Pass 7.1 (flatten + FDF/XFDF + choice fill +
  regenerate). Pass 7.x is the FILL/read/flatten side of AcroForm;
  field creation is the AUTHORING side, unscoped, unbuilt, no Pass
  number assigned. Ken's own framing — "after, if that makes sense" —
  is an explicit hedge, not a firm commitment; re-evaluate scope and
  priority against whatever's shipped by the time items 1–3 are
  reached, rather than treating this as an unconditionally queued Pass.
  When it is picked up: dispatch `pdfce-acrobat-librarian` for the
  Form-Field-creation capability bucket (widget appearance-stream
  generation rules, default field properties per type, Acrobat's own
  auto-detect heuristics) before scoping a Pass, same discipline as
  every other Backlog-to-Pass promotion.
  **AMENDMENT (2026-08-03) — the form-field-creation research is now
  DONE; scoping is IN FLIGHT via a KenAgent decision agent writing to
  `docs/decisions/` (form-building scope), concurrent with this
  filing.** `pdfce-acrobat-librarian` added 5 new files
  (`forms__field_creation_minimums.md`,
  `forms__field_naming_hierarchy_and_collisions.md`,
  `forms__radio_group_authoring.md`, `forms__tab_order.md`,
  `forms__authoring_limits_and_refusals.md`) plus dated addenda to 3
  existing files — bucket now 17 files total, index in
  `D:\Dev\Rag-Specialized\Acrobat_Features\index.md`. **Headline
  finding, recorded as the bucket's central must_have acceptance
  criterion:** a newly-typed field name colliding with an existing
  field's name is TYPE-BRANCHED — same name + same type MERGES into
  one logical field with additional `/Kids` widget members (the
  correct, intentional mechanism radio groups and legitimately-
  duplicated fields depend on); same name + a DIFFERENT type is
  REFUSED outright with a specific quoted error string. **Recommendation
  to whoever scopes this into a Pass: `pdfce-core`'s field model must
  be a `/Kids` object graph from day one, not a flat name-keyed list**
  — retrofitting this distinction later is expensive. Also produced:
  two search-confirmed-unfillable GAPs on radio-group member deletion
  (recommend empirical testing against a real Acrobat install once
  fixtures exist, rather than further web research); no source anywhere
  on tab-order insertion behavior (recommend pdfce adopt its own
  documented deterministic rule rather than claim parity); a sourced
  exceed-Acrobat opportunity (Acrobat's own newly-created fields ship
  UNTAGGED, even in Acrobat's own accessibility-forward workflow); and
  two unreconciled conflicts flagged rather than guessed — (a)
  `core_ops__merge_combine_files.md` (already built) documents
  Combine-Files as defaulting to auto-RENAMING duplicate field names
  across merged source files, while this session's separately-sourced
  finding describes same-named fields across merged documents as
  LINKING into one field by default (plausibly different Acrobat
  versions/entry points, not reconciled); (b) the encrypted-document
  permission workflow (two sourced accounts of "add fields before
  securing" vs. "unlock with owner password, then Prepare Form works,"
  not reconciled). See `D:\Dev\Rag-Specialized\Acrobat_Features\
  index.md`'s 2026-08-03 extension-session entry for the full record.
  **The librarian avoided `docs/decisions/` this filing per explicit
  instruction — the KenAgent decision agent's scope work there is not
  reflected above beyond noting it is in flight.**
  **AMENDMENT (2026-08-03, continuation 71) — decision 020 is now
  DECIDED, SCOPED, NOT STARTED.** Archived at
  `docs/decisions/020-form-field-authoring.md`; full record:
  `ARCHITECTURE.md` §12's 2026-08-03 continuation-71 entry. **Pass
  20.x family assigned and verified free**: **Pass 20.0** (F0,
  field-hierarchy correctness + authoring substrate, core-only,
  rule-11 exempt, no operator surface) → **Pass 20.1** (F1, text-field
  creation through the resolver with all four collision outcomes live —
  the P0 floor) → **Pass 20.2** (F2, checkbox/radio creation + field/
  widget deletion) → **Pass 20.3** (F3, choice fields + push buttons) →
  **Pass 20.4** (F4, tab order — **BLOCKED on a `pdfce-spec-librarian`
  dispatch** for Table 30's `/Tabs` row, §14.7 structure-order
  derivation, and the ISO 32000-2 `/Tabs` delta, all verified absent
  from the spec RAG this session) → **Pass 20.5** (F5, GUI authoring
  surface — **requires a `pdfce-ui-specialist` dispatch first**, rule:
  non-trivial UI). **Pass 20.6** (F6, `--defaults-from`/`rename-field`)
  and **Pass 20.7** (F7, `merge --on-field-collision`) are named
  fast-follows, non-gating, filed for numbering stability only — not
  scheduled ahead of F0–F5.
  **The headline correction to this bucket's own prior recommendation:**
  the "build `pdfce-core`'s field model as a `/Kids` object graph from
  day one" advice above is **superseded** — the shipped flat
  `AcroForm.fields: Vec<Field>` read projection turns out to already be
  correct (it retains `Field.widgets: Vec<Widget>`, the one-to-many that
  actually matters) and stays unchanged; what was missing is a
  **write-side-only** graph resolver, now designed (R100). Read this
  bucket's headline line as historical framing, not current guidance —
  decision 020 is now the authority for this bucket's data model.
  **The two unreconciled conflicts flagged above are now resolved**
  (Combine-Files: not a contradiction, an operator choice, filed F7;
  encrypted-document workflow: structurally inapplicable to pdfce, see
  R103). The two radio-deletion GAPs are also resolved — reframed as
  pdfce's own documented, test-provable design choice rather than
  something needing empirical verification against a real Acrobat
  install. **Five items filed for the operator, not decided solo** — see
  Open operator questions, below, items (m)–(q).
  **Whether to actually START F0 is itself one of those operator
  questions** (item (m)) — this amendment files the Pass IDs so they
  exist and are stable per project rule 2, not as an instruction to
  begin building.
- **XFA** — legacy Adobe forms tech. **Verify current status before
  scoping** — Adobe has been deprecating XFA in Acrobat; consult the
  spec RAG + a fresh web check before committing engineering time here.
  Likely low priority relative to AcroForm.
  **Demand measured 2026-08-01 (decision 008 census):** `/XFA`
  present in **2 of 2,500 organic Dropbox files (0.08%)** and 4 of
  2,914 conformance-corpus files. This confirms XFA is negligible in
  the wild and ranks it below AcroForm, as expected — but it answers
  ONLY the demand half of the open question. It does **NOT** close the
  standing `CLAUDE.md` / ARCHITECTURE open item "verify Adobe's current
  XFA support/deprecation status," which is a spec/vendor-status
  question the census cannot answer; that verification is still owed
  before any XFA engineering time is committed.
  **AMENDMENT (2026-08-03) — the standing open item is PARTIALLY
  answered, PARTIALLY still open.** `xfa__deprecation_status_and_
  hybrid_forms.md` (`D:\Dev\Rag-Specialized\Acrobat_Features\`)
  confirms dynamic XFA has NO AcroForm at all as of Acrobat 8.1+, so
  `out_of_scope` reads clean for dynamic XFA — but **static-XFA-hybrid
  new-field-creation permissibility (does Acrobat allow authoring a NEW
  AcroForm field into a document that also carries a static-XFA
  stream?) is an explicit, unresolved GAP**, surfaced by this session's
  forms-authoring research and not answered by the census or the prior
  deprecation-timeline lookup. Acrobat's own precise version-level
  deprecation date also remains a GAP (third-party-vendor-sourced
  approximate timing only, no Adobe-primary source found). **This is
  an operator-relevant call, not just an engineering one**: the
  `CLAUDE.md` open item asks to verify XFA's relevance before
  committing engineering time; this amendment narrows exactly what
  still needs verifying (hybrid-form authoring permissibility, exact
  deprecation date) rather than closing the item.
  **AMENDMENT (2026-08-03, continuation 71, decision 020 §3.2/§10.5) —
  the hybrid-authoring GAP is now DECIDED, not left open: static-XFA
  hybrid field creation is REFUSED BY NAME**, decided from pdfce's own
  capability boundary (it can write the AcroForm half of a hybrid but
  not the XFA half, so a one-sided add would make an XFA-aware viewer
  and a non-XFA viewer show different field counts for one document) —
  this closes the "does Acrobat allow this" GAP by making it
  not-load-bearing, not by resolving it empirically. Dynamic XFA stays
  `out_of_scope`, unchanged. **This narrows, but does not fully close,
  the standing `CLAUDE.md`/`ARCHITECTURE.md` "verify Adobe's current
  XFA support/deprecation status" open item** — decision 020
  recommends re-scoping it to "before any XFA *read/fill* work" rather
  than a general standing item, since both authoring branches are now
  decided without needing it answered. **This recommendation is filed
  as Open operator question (p), below — retiring or re-scoping the
  item is Ken's call, not the librarian's or engineer's to make solo.**
  Note for whoever eventually acts on it: this decision only updates
  this `ROADMAP.md` bucket; the mirror-image bullet in the project's
  own `CLAUDE.md` ("Outstanding open items") is a separate file outside
  `pdfce-librarian`'s owned tiers and was not edited by this filing —
  flagged so it doesn't silently drift out of sync with this entry.
- **Digital signatures** — PAdES profiles (B-B, B-T, B-LT, B-LTA),
  PKCS#7 signing + verification, incremental-update-based signing
  (see `ARCHITECTURE.md` §5), timestamp authority (RFC 3161) support.
  **Demand measured 2026-08-01 (decision 008 census):** `/SigFlags`
  present in **16 of 2,500 organic Dropbox files (0.64%)** — the
  lowest real-world share of the decision-008 ranked set, consistent
  with ranking Signatures LAST (candidate F, Pass 10). The READ half
  is already far along (Pass 3.2's `SignatureImpact` /
  `/DocMDP` / `/FieldMDP` classification and the §12.8 spec closure);
  this bucket's remaining work is the signing/verification/timestamp
  authoring side.
- **Encryption** — standard security handler, RC4 (legacy read-compat
  only, never write), AES-128/256, public-key (certificate) security
  handler. **Updated 2026-07-31 by decision 007 (Pass 5 in its
  sequence):** decrypt **ALL** handlers; encrypt-on-save
  **AES-128/256 ONLY** — RC4 is NEVER written (the standing Backlog
  posture R28 cites as its own precedent). Deliberately placed AFTER
  the writer: the crypt stage is a bidirectional encoder that slots
  into Pass 3.0's serializer seam (R37), its ground truth is a
  decrypt → re-encrypt → byte-compare round trip only the writer
  makes possible, and encrypting newly-appended objects during
  incremental save is designed in as a first-class requirement rather
  than a retrofit. Measured corpus payoff is **zero** (all 4
  `RefusedEncrypted` files are veraPDF/Isartor `*-fail-*` conformance
  files, non-conformant by design) — pdfce cannot validate an
  encryption implementation with the corpus it owns; a synthetic
  encrypted-fixture generator is a prerequisite (LEGAL §5,
  `tools/gen-*-fixtures.py` pattern). **Promotion trigger:** Pass 5
  runs AHEAD of Pass 4 if Pass 3.0's parallel organic `/Encrypt`
  census shows a materially non-trivial empty-user-password share
  (those files open silently in every other reader).
  **Census RESULT (2026-07-31 — decision 007 parallel cheap task, run
  alongside Pass 3.0 by a parallel agent):** 19,940 organic PDFs
  scanned (20k cap hit, Dropbox-dominated; read-only, aggregate
  counts only, nothing copied — LEGAL §5): **134 = 0.67% carry
  `/Encrypt`**; revision mix 26 R2 / 30 R3 / 67 R4 / 10 R6 /
  1 undetermined-R (FOPN FileOpen DRM — non-Standard security
  handler, never silently openable by any reader); **92.5% legacy
  R≤4**. Empty-vs-real user password is NOT determinable pre-Pass-5
  (it requires the §7.6 handshake itself). **Promotion trigger NOT
  met — Pass 5 stays behind Pass 4.**
  **PROMOTED to In progress as Pass 5, 2026-08-01, on Pass 4's ship —
  by decision-007 SEQUENCE, not by the (untriggered) promotion rule**
  (see In progress; `pdfce-spec-librarian` §7.6 spec-corpus session
  dispatched, in flight). Bucket text retained here as the scoping
  record. **§7.6 is the
  single largest spec gap across all decision-007 candidates** — the
  spec RAG has only `filters/filter__crypt.md` (the `/Crypt` FILTER,
  not the clause tree); a full `pdfce-spec-librarian` corpus-building
  session is required before this Pass. Dependencies pre-selected by
  decision 001 §6.2 (`aes`, `cbc`, `sha2`, `md-5`, `rc4`;
  `cms`/`x509-cert`/`x509-parser` for later PAdES).
  **§7.6 SPEC-CORPUS SESSION COMPLETE (2026-08-01) — Pass 5 is no
  longer spec-blocked (it remains queue-deferred behind Pass 7 per
  decision 008; only the spec prerequisite closed, not its position).**
  `pdfce-spec-librarian` built the §7.6 corpus at
  `D:\Dev\Rag-Specialized\PDF_Spec\` (7 new + 2 updated files:
  `iso32000__s__7.6.1`–`7.6.5.md`; new `security__aes256_r5_r6.md` under
  a new `security__` prefix; `iso32000__ref__encryption_impl.md` derived
  implementation checklist; `filter__crypt.md` de-stubbed; the Adobe
  ExtensionLevel 3 supplement staged). This closes the "§7.6 is the
  single largest spec gap" prerequisite named above.
  **Finding that changes the Pass 5 plan:** ISO 32000-1 contains **NO
  AES-256** — AESV3/AESV5, revisions R5/R6, SHA-256, Algorithms
  1.A/2.A/2.B and 8–13, and the `/OE` `/UE` `/Perms` entries in the
  `/Encrypt` dict are ALL sourced from **Adobe's ExtensionLevel 3
  supplement**, not the base ISO standard. Critically, **`/R 6` — the
  AES-256 revision Acrobat X+ actually WRITES — could NOT be sourced**:
  ISO 32000-2 is paywalled, no public ExtensionLevel 8 is locatable, and
  pdfa.org returns 403. The spec-librarian **correctly REFUSED to
  reconstruct Algorithm 2.B from memory** (the retracted-URW-claim /
  no-fabrication discipline). **Consequence for decision 007's
  "encrypt-on-save AES-128/256 only":** AES-256 is currently buildable
  **only at `/R 5`** (which has published weaknesses); `/R 6` cannot be
  implemented from sourced material today. **Three honest options,
  to settle as a Pass-5 OPEN SUB-DECISION BEFORE the Pass is scoped
  (a future KenAgent decision when Pass 5 activates):** (i) close the
  `/R 6` sourcing gap first; (ii) ship **AES-128 (`/V 4` AESV2) as the
  only write target**; (iii) **decrypt-only for AES-256** (never write
  it). Two operator decisions this session raised — the LEGAL.md §2
  Adobe-supplement copyright contradiction and the `/R 6` sourcing
  method — are on the SESSION_LOG operator-items list (continuation 22),
  Ken's calls, not resolvable autonomously.
- **Redaction** — true content removal (not visual-overlay-only), per
  `ARCHITECTURE.md` §5 corollary. This is a trust-critical feature;
  needs explicit test coverage proving removed content is actually
  gone from the saved bytes, not just hidden. **AMENDED 2026-08-03 —
  mark + apply + review, CLI and GUI, are all SHIPPED (Pass 8.0, Pass
  8.1) with the absence-proof test coverage this bullet asked for. Two
  named items remain in this bucket, both filed as their own Backlog
  bullets rather than duplicated here:** canvas drag-marking + the
  transient property bar (see the bullet immediately above the
  Forms/AcroForm bucket, below — a scope call out of Pass 8.1, not a
  block); and **Sanitize / Remove Hidden Information (§6 of the
  ui-spec)** — not yet scoped into any Pass, no builder dispatched.
  `redaction__sanitize_remove_hidden_information.md`
  (`D:\Dev\Rag-Specialized\Acrobat_Features\`) already grounds this
  capability's acceptance criteria from the 2026-07-31 Redaction-bucket
  research session — re-read it, don't re-research, when Sanitize is
  scoped.
- **Bates numbering / stamping** — header/footer stamps, sequential
  numbering across a batch, watermarks.
- **Annotation display (read-side)** — created 2026-08-01 by decision
  008 (finding **F1**). Rendering AND counting of every §12.5
  annotation and widget from its `/AP` `/N` appearance stream, with the
  annotation-flag set honored and disclosed. This bucket did not exist
  before decision 008 — the gap was **filed nowhere**, exactly as text
  extraction was pre-decision-007 (`ToUnicode` appeared in no ROADMAP,
  SESSION_LOG, or decision record until 007 created it). Distinct from
  "Comments & markup" below, which is the AUTHORING side (creating new
  annotations); this bucket is the DISPLAY side (showing what a file
  already contains). **Scoped into Pass 6.0 by decision 008 — see the
  In-progress and Next-up entries.** Retained here as the bucket of
  record for the read-side capability area.
- **Comments & markup** — annotations (text notes, highlights, ink,
  shapes, stamps), reply threads, markup summary/export. **This bucket
  is the AUTHORING side** (its display sibling is the "Annotation
  display (read-side)" bucket above). Scoped by decision 008 into
  Passes 6.1 (geometric markup: Ink/Square/Circle/Line/Polygon/
  quad-point) and 6.2 (text-bearing: FreeText + §12.7.3.3 variable
  text). `pdfce-acrobat-librarian` "Comments & markup" bucket
  dispatched 2026-08-01 to ground Pass 6.1's acceptance criteria.
  **OPEN GUI follow-up slice (Pass-6.1-followup, named on 6.1's ship
  2026-08-01):** the full canvas markup **drawing state machine** —
  drag/marquee/multi-click/ink-freehand authoring + live preview + the
  screen↔page transform (the pass-4-only-planned transform) + the
  ten-tool set + the keyboard map — plus **P1 glyph-accurate
  text-selection markup**. Design is
  `docs/ui_specs/pass-6.1-markup-tools.md`. Pass 6.1 shipped only the
  **minimal menu affordance** (the toolbar "Markup ▾" menu authoring at
  a default page-centred rect through `EditSession::add_markup`); the
  core authoring path is complete, so this slice is pure GUI/interaction
  work and can be promoted independently of 6.2.
- **Text extraction / structured content** — **SHIPPED 2026-08-01 as
  Pass 4 — see the Shipped entry at top** (sourced total 99.78% over
  281,516 codes; rung 3 zero; bidi deferred-not-half-done). Earlier
  history: **PROMOTED to In progress as Pass 4, 2026-07-31, on Pass
  3.2's ship** (`pdfce-spec-librarian` §9.10 sourcing dispatch, since
  returned). Bucket text retained below as the scoping record. Bucket
  CREATED 2026-07-31 by decision 007 (**Pass 4** in its sequence; previously
  unfiled — `ToUnicode` appeared nowhere in ROADMAP, SESSION_LOG, or
  any decision record). Scope: `ToUnicode` CMaps, `/ActualText`,
  reading order — search and select-copy are what separate a viewer
  from a demo; later feeds redaction *verification*, OCR quality
  comparison, and the Comparison bucket's text diff. Pre-constrained
  by **R17** (`unicode-bidi` is permitted in a text-extraction
  reading-order path and forbidden in `pdfce-render`) and **R7**
  (document text is a Pass-1-onward requirement); CMap machinery
  already exists from the CID/`Identity-H` work. **Ranked second
  overall (A ≫ D > B > C) and designated the FALLBACK track**: if a
  writer Pass hits the three-attempts wall, this is the switch target
  (writer-independent by construction). Also the interleave candidate
  if the operator wants a visible-value break between Pass 3.1
  and 3.2.
- **OCR** — recognize-text-in-scanned-page. Needs a decision on OCR
  engine binding (candidate: `tesseract` via a Rust binding, or a
  pure-Rust OCR crate if one is production-quality by the time this
  is scoped — check current state, don't assume 2026-era training
  data is current). Output is always reviewable hint text, never
  silently baked in without operator confirmation.
- **Accessibility (PDF/UA)** — tagged-PDF structure tree authoring +
  validation, reading-order tools, alt-text prompts for images. Also
  see `.claude/agents/pdfce-ui-specialist.md` — the *app's own UI*
  should aim for screen-reader accessibility too, not just its output
  files.
- **Comparison** — visual diff + text diff between two PDF revisions.
- **Portfolios (PDF Package)** — multi-file container support.
- **Optimization / linearization** — "Fast Web View" linearized output,
  image downsampling, font subsetting on save, size-reduction reports.
- **PDF/A conformance** — convert-to and validate-against PDF/A-1/2/3/4
  profiles; surface non-conformance reasons in a way a non-specialist
  operator can act on.
- **Print & prepress (PDF/X)** — lower priority unless the user
  signals otherwise; flag as backlog-only until requested.
- **UI font coverage for non-Latin file paths and document metadata**
  (filed 2026-07-30 from decision 002 §3.2 / §10 item 10). A **live
  rendering-correctness bug today**, independent of localization:
  epaint's bundled fonts (~1.4 MB: Ubuntu-Light, Hack, NotoEmoji,
  emoji-icon-font) cover Latin/Greek/Cyrillic/IPA/emoji only — **no
  CJK, Arabic, Hebrew, Indic, or Thai glyphs**. Bundling CJK is
  out of scope upstream (emilk/egui#3060, closed not-planned) and
  there is no system-font discovery (emilk/egui#5233, open). A user
  opening a file with a CJK/Arabic/Hebrew name — or a document whose
  `/Title` metadata is in one of those scripts — sees tofu (U+25FB)
  in pdfce's status bar right now, with an entirely English UI.
  Options to weigh at scoping time: (a) bundle a subsetted CJK face —
  full Noto Sans CJK is ≈15.7 MB, material against `ARCHITECTURE.md`
  §6's single-folder budget; (b) runtime system-font discovery —
  Windows ships Yu Gothic / Microsoft YaHei / Malgun Gothic, costs
  zero bytes and fits the portable-folder constraint far better.
  Not a Pass-1 blocker. Version-stamped text-stack detail:
  `D:\dev\rag\egui\epaint_0.35_text_stack_i18n_limits.md`.
- **DeviceCMYK→RGB colorimetry** — filed 2026-07-31 from decision 006
  §3.7 / §10 item 9 (found in passing while proving CMYK/JPEG polarity;
  deliberately excluded from that decision so polarity and colour
  never get confounded). `Rgb::from_cmyk`
  (`crates/pdfce-render/src/gstate.rs:112`) is the naive additive
  `1.0 − min(c + k, 1.0)`; pdfium uses its calibrated
  `AdobeCMYK_to_sRGB1` table. Measured on the 300×232 corpus CMYK JPEG
  at 1:1 against pdfium: **37.4% of pixels differ by >8 in some
  channel**, max abs Δ per channel `[11, 37, 30]`, 95th percentile
  `[5, 27, 18]` — a real, visible fidelity gap, though channel
  correlations stay 0.99+ and sign agreement 100% (which is why it
  never confounded the polarity result). **Affects ALL `DeviceCMYK`
  painting — every fill and stroke — not just images.** Scope via
  `pdfce-acrobat-librarian` (what does Acrobat actually do for
  uncalibrated DeviceCMYK→screen?) before committing engineering
  time. When addressed, re-run decision 006 §3.4's variant matrix and
  pin the polarity conclusions BEFORE the colour change lands
  (006 revisit trigger 7).
- **Product-scope decisions — deliberately deferred, not oversights**
  (identified 2026-07-23, flagged rather than silently skipped):
  - ~~**Internationalization/localization.**~~ **RESOLVED 2026-07-30**
    per `docs/decisions/002-i18n-timing.md` (KenAgent decision 002) —
    no longer deferred. Outcome: centralized zero-dependency
    function-based string catalog (`crates/pdfce-gui/src/ui_text.rs`),
    English-only; `pdfce-cli` English-only permanently by design with
    locale-invariant stdout; `pdfce-core` errors never localized but
    structured (R4). See standing rules R1–R8 above and the decision
    record's §9 revisit triggers.
  - ~~**Cross-platform scope beyond "Windows first."**~~ **RESOLVED
    2026-07-30** per `docs/decisions/003-distribution-posture.md`
    (KenAgent decision 003) — no longer deferred. Outcome: v1 ships
    Windows 10/11 x64 only, as a deliberate scope decision; the
    codebase stays platform-clean at all times, enforced by
    cross-target `cargo check` CI (macOS + wasm32) instead of new
    runners; macOS gated on decision 003 §9 T1+T2 jointly; if Linux is
    ever shipped, `pdfce-cli` goes first (static musl), `pdfce-gui`
    separately (glibc-dynamic). See standing rules R9–R11.
  - ~~**Update/release mechanism.**~~ **RESOLVED 2026-07-30** per
    `docs/decisions/003-distribution-posture.md` (KenAgent decision
    003) — no longer deferred. Outcome: manual download-and-replace is
    the only update mechanism pdfce implements, permanently; pdfce
    never self-updates (R13); update *discovery* is delegated to
    Scoop-then-WinGet manifests (gated on `LEGAL.md` §1 — see the
    "Release & distribution channel" Backlog entry below); no network
    client crate ever enters the tree without a new decision record
    (R12, fail-closed CI). See standing rules R12–R16.

  **This list is now EMPTY.** Every product-scope question flagged at
  the 2026-07-23 bootstrap (i18n → decision 002; cross-platform scope
  and update mechanism → decision 003) has been answered within one
  week, each with an archived decision record in `docs/decisions/`.
- **Release & distribution channel** — filed 2026-07-30 per
  `docs/decisions/003-distribution-posture.md` §10 item 12.
  **AMENDED 2026-08-01: the `LEGAL.md` §1 manifest-property blocker is
  LIFTED** — `license = "MIT"` now exists and can populate both Scoop's
  `license` manifest property and WinGet's `License` defaultLocale
  field. **Still blocked in practice, on a separate gate: no public
  repo/release exists yet** — the project's one implementation commit
  (`d8b3903`) is local-only, and pushing/publishing requires its own,
  not-yet-granted operator authorization (distinct from the license
  decision — see `docs/SESSION_LOG.md` continuations 49–50). Do not
  build or publish either manifest until that authorization arrives.
  The work, in order, once unblocked:
  - **Scoop manifest** first (natural fit for a portable app:
    user-scope, no UAC, no registry; `checkver: "github"` +
    `autoupdate`; use `persist` for the R15 user-state partition).
  - **WinGet portable manifest** second (broader reach, more friction:
    community-repo PR review; zip + nested-portable rough edges,
    winget-cli #3279/#2806/#6215). Chocolatey: skippable.
  - **SHA-256 checksums published for every artifact of every release,
    from the first release** (R16 — unrecoverable if skipped: a
    rebuild is not bit-identical, so hashes cannot be honestly
    reconstructed later).
  - **README privacy/platform/signing copy** — use decision 003 §6.3's
    wording VERBATIM; do not paraphrase it looser (claim-bearing copy).
    Covers: no network use (verifiable via `THIRD_PARTY_LICENSES.md`),
    link clicks go to the OS browser, manual updates + keep-user-state
    folder, Windows-x64-only support statement, unsigned-binary
    SmartScreen disclosure + checksum verification pointer.
  - Re-read the current Scoop/WinGet schema at the moment a manifest is
    actually written — manifest schemas drift (decision 003 §5.4).
- **CLI batch operations (`pdfce-cli`)** — a subcommand per feature
  bucket above, added *alongside* that feature's own GUI Pass rather
  than as a separate late-stage effort (e.g. the Bates-stamping Pass
  ships both the GUI flow and `pdfce-cli bates-stamp`, same session).
  See `ARCHITECTURE.md` §7 for the subcommand shape and exit-code
  convention. Pass 0 seeds the scaffold; this bucket tracks ongoing
  subcommand coverage as other features land.
- **Bulleted/numbered list authoring** — filed 2026-08-01, surfaced
  while scoping Pass 14.2 against the Acrobat feature-parity RAG (see
  `D:\Dev\Rag-Specialized\Acrobat_Features\text_edit__formatting_options.md`).
  Creating list items and auto-numbering is real Acrobat Edit-PDF
  behavior with **no home in decision 014's Pass 14.x family or the
  FF-A..FF-H fast-follow ladder** — not even as a named deferral. It is
  content AUTHORING (kin to FF-D add-new-text, but structured — an
  ordered/unordered list is a block-level construct, not a per-glyph
  edit), NOT in-place editing of existing runs, so it does not fit
  cleanly into 14.1/14.2/14.3's surgery model as scoped. **Operator
  scope question, flagged to Ken, not yet answered:** do we want list
  authoring as an Acrobat-parity target at all, and if so where in the
  Pass sequence? No Pass number or priority invented here — this entry
  exists solely so the gap isn't silently dropped. Scope it into a real
  Pass (candidate name: a future "14.4" or a standalone authoring Pass)
  only once the operator decides.

  **AMENDED 2026-08-01 by decision 016 §10:** re-confirmed operator-
  gated on prioritizing the text-parity fast-follow ladder (decision
  016 ranked it #5, "sequences after FF-D regardless"). **AWAITING
  OPERATOR DECISION — DO NOT SCHEDULE TO A PASS.** No new information
  changes the scope question above; decision 016 only confirms it is
  still open and notes list-authoring builds on FF-D (Pass 16.x) + the
  15.x reflow engine once/if the operator says yes.

- **FF-D fast-follow FF-C — font subsetting / glyph embedding.
  UNBLOCKED 2026-08-01 — schedulable, no Pass number assigned yet.**
  Filed 2026-08-01 by decision 016 §10
  (`docs/decisions/016-ffd-add-new-page-text.md`), ranked **#2 by
  value** in decision 016's text-parity fast-follow prioritization
  (§2) — it lifts the single most common wall in real editing, the
  embedded-subset "can't originate that glyph" refusal from
  `text_edit__font_handling_on_edit.md` (R71). Originally could not
  start solo because adding a font subsetter/embedder is a new Cargo
  dependency, gated by **rule 8** (pdfce's own OSS license — was
  undecided) and requiring **rule 13** flagging if the chosen crate
  turns out copyleft.
  **AMENDMENT (2026-08-01, SESSION_LOG continuation 50): both gates are
  now clear.** `LEGAL.md` §1 is DECIDED — MIT — closing the rule-8 gate
  outright, and the operator's same-continuation "finish off all the
  text handling stuff" directive is the explicit go-ahead decision 016
  recommended waiting for. **Rule 13 still applies to the specific
  crate chosen** — whoever scopes this into a Pass must pick a
  **permissive-only** font subsetter/embedder (MIT/Apache-2.0/BSD-class)
  and flag it in `PRIOR_ART.md`/the dependency audit same as any other
  new dependency; a copyleft candidate still needs an explicit
  operator check-in per rule 13, even though the *project* license
  question that used to gate this is now settled. The
  `pdfce-spec-librarian` font-subsetting dispatch (named at decision
  014) remains queued to run ahead of the actual build so spec grounding
  is ready. **No Pass number invented here** — assignment is the
  engineer's call when this is actually scoped, per the ★★★ Operator
  priority sequence (top of Next up), behind the dimensioning tool and
  icon set.
  **AMENDMENT (2026-08-03): rule 13's specific-crate classification is
  now DONE, ahead of the Pass, and clears without an operator
  decision.** `subsetter 0.2.6` (Typst) is `MIT OR Apache-2.0` with an
  all-permissive transitive graph (verified via `cargo metadata` on a
  scratch crate, not from crates.io pages or memory); `LEGAL.md` §6.2
  step 3 (proceed and log) applies, same as `egui_tiles` got. It also
  resolves the correct `read-fonts 0.39.2`/`font-types 0.11.3` pin by
  construction, matching what `pdfce-render`'s `skrifa 0.42`/epaint
  0.35 pin already requires — a bare `cargo add write-fonts` would
  instead select 0.51.0 and split the graph across two incompatible
  font-parser versions. Nothing was added to any `Cargo.toml` yet; this
  is the classification rule 13 requires *before* adding, done in
  advance. Full record: `PRIOR_ART.md` §Fonts "FF-C dependency
  classification (rule 13) — COMPLETE, 2026-08-03". A KenAgent decision
  agent is scoping FF-C into a Pass family concurrently (will land as
  decision 021, writing to `docs/decisions/`) — this amendment does not
  pre-empt that scoping, only closes the licensing sub-question.
  **AMENDMENT (2026-08-03) — decision 021 filed: DECIDED, SCOPED, NOT
  STARTED. FF-C now has an assigned Pass family: ★ Pass 21.x (Next
  up, below).** `docs/decisions/021-ffc-font-subsetting-and-glyph-
  embedding.md`. **Headline correction, propagating outward from this
  bullet, R71, and decision 014 §5.3 (all three amended below/
  elsewhere): FF-C as previously described — "extend the document's
  own embedded font" — is not implementable.** A subset font by
  definition does not contain the glyph being added; there is no
  operation on an existing `FontFile2` alone that produces it. FF-C is
  re-scoped **add-only**: it adds a new, subsetted font resource from a
  donor face (decision 012's `--font-dir`) and never touches an
  existing font program or font dictionary — new standing rule **R107**.
  Crate boundary: `subsetter` (Typst, MIT OR Apache-2.0) lands in
  `pdfce-render`, `default-features = false` (net **1** new package,
  refined from the 2-package figure above — see `PRIOR_ART.md`'s
  amended FF-C dependency-classification section); `pdfce-core` gains
  zero new dependencies. `pdfce-core::font_embed` (new module) emits
  the PDF objects. New standing rules **R108** (embedding is an
  explicit per-action operator choice, real subset size/coverage shown
  before confirmation, never a default) and **R109** (font-embedding
  permission is read from the donor's `OS/2` `fsType` and disclosed,
  never assumed) govern disclosure. New standing rule **R110** governs
  when an FF-C-authored composite run becomes editable (verified-
  injective `/ToUnicode` only — `Identity-H` with no `/ToUnicode` stays
  a permanent hard skip, R65 untouched). Sliced as Pass 21.0 (core+CLI,
  the P0 floor, lifts the widest wall — any character outside WinAnsi/
  Symbol/ZapfDingbats) → 21.1 (composite-run edit, makes 21.0's output
  editable — **do not ship 21.0 without 21.1 and call FF-C done**, see
  the ★ Pass 21.x entry) → 21.2 (`set-font` to an embedded face) → 21.3
  (GUI, `pdfce-ui-specialist` dispatched first). Full slice detail: the
  ★ Pass 21.x entry under Next up, below.
  **AMENDMENT (2026-08-04): Pass 21.0 SHIPPED (`48c6b77`) — see the
  Pass 21.0 Shipped entry, top of Shipped. pdfce can now add non-Latin
  (glyf/TrueType-donor) text via `add-text --embed-font`. Pass 21.1
  promoted to In progress (above/below). R109's fsType read, though
  named in 21.0's original scope, did NOT ship — tracked as an owed
  gap alongside 21.1, not FF-C-complete yet.**

- ~~**FF-D follow-up — certification-signature guard gap on
  `add_text`/`EditSession::add_text`. Flagged 2026-08-01 at Pass 16.0's
  ship; NOT actioned, do not schedule without an explicit dispatch.**
  Unlike `add_markup` (Pass 6.x), the `add_text` free function and
  `EditSession::add_text` both check encryption and suppressed-objects
  guards, but neither reaches `check_certification` — a private
  `EditSession` method the free-function engine has no access to. A
  certified (MDP-locked) PDF could therefore accept a page-text add
  that a certification signature should have blocked. Follow-up scope,
  when actioned: (1) add a certification-signature guard to the
  `add_text`/`EditSession::add_text` path, mirroring `add_markup`'s
  existing check; (2) consider exposing `check_certification` (or an
  equivalent guard hook) so other free-function engines (e.g. the 15.x
  reflow engine, if it has the same gap) can reach it without going
  through `EditSession`. No Pass number invented here — this is a gap
  record, not a scoped Pass.~~ **RESOLVED 2026-08-01 — see the "FF-D
  follow-up hardening" Shipped entry** (top of Shipped, above). Both
  scope items are discharged: (1) a new shared `pub(crate)
  refuse_if_certification_forbids` helper, reusing
  `EditSession::check_certification`'s exact machinery
  (`signature::census` + `SignatureCensus::forbids_structural_change()`),
  is now called by both `EditSession::add_text` and the free `add_text`
  function; (2) **posture (a) chosen** — the free function guards
  itself directly (`census`/`forbids_structural_change` were already
  `pub` and reachable without an `EditSession`), so the shared helper
  itself IS the guard hook other free-function engines (e.g. a future
  15.x-reflow-adjacent engine) can call the same way, without a
  separate exposure step. New `AddTextError::CertificationForbidsChange`
  (message VERBATIM-identical to `EditError`'s variant, message-parity
  tested); CLI maps it to `exit::EDIT_REFUSED`. New fixture
  `fixtures/synthetic/addtext/certified-locked.pdf`. Tests: core
  point/box/free-fn refusal + regression (uncertified doc still adds) +
  message-parity; CLI point/box refusal with §12.8.4 in stderr. Gates:
  core lib 713 passed, CLI `add_text` 15 passed, fmt/clippy clean,
  `cargo tree` GUI-dep-free, zero new dependency.

- **FF-I — minimal StructTree/`/ActualText` update on tagged-page text
  edits. Filed 2026-08-03 by decision 019 §3.7, CUT from FF-H's
  original bundle (decision 014 §5.3) rather than carried forward
  unbuilt.** Decision 016 §2 had already found FF-H's StructTree piece
  "premature"; decision 019 acts on that finding rather than merely
  repeating it. Rationale for the cut: a *partial* structure-tree
  writer (one that updates `/ActualText`/MCID linkage for the specific
  edits FF-H's own operators touch, but not the general case of a
  content-stream surgery moving marked-content boundaries) is judged
  worse than none — a document that looks tagged-and-consistent but
  silently drifts out of sync with its own structure tree is a harder
  failure to detect than one that visibly discloses "structure not
  updated" (the existing R73 posture, which pdfce already ships).
  **No Pass number assigned.** Scope, when actually picked up: a real
  StructTree writer needs its own decision record — this entry exists
  so the gap isn't silently dropped, not to pre-scope the eventual
  Pass. See the ★ Pass 19.x entry (Next up) for the full context this
  cut sits inside.

## Open operator questions (as of 2026-08-02 — answer any, all default to the stated fallback if not answered)

**RESOLVED this session (continuation 56) — no longer open:**
- **(a) Icon SVG pipeline switch — RESOLVED, tiny-skia SVG-path-`d`
  parser CONFIRMED** (no new Cargo dependency, crisp at any DPI/zoom;
  the pre-rasterize-to-PNG plan is superseded, retained for history in
  the "★ Icon set" entry above). **UPDATE:** the icon BUILD has since
  SHIPPED (Pass 18.3, continuation 57) — ahead of the Pass-17.1/17.2
  gate at the time (see the ★★★★★ REORDERING entry's "DEVIATION
  RECORDED" note), and that gate has itself since cleared for real
  (continuation 58) — nothing about this item remains open in any
  direction.
- **(f) Decision 018 / ★★★★★ reordering — RESOLVED, CONFIRMED yes.**
  Pass 17.x (live-edit rendering) lands before the rest of the ★★★
  operator priority sequence (icons, text-handling, forms). The
  ★★★★★ entry above is now a confirmed reordering, not a proposal.

**RESOLVED this session (continuation 57) — no longer open:**
- **(b) Decision 017 Q1 — RESOLVED, ANSWERED wide.** The operator chose
  the VS Code/Blender whole-content-area model: *"Use egui_tiles…
  has the flexibal docking that works as well as inkscape's."* This
  fires the §6.1 trigger — `egui_tiles` 0.16.0 is ADOPTED as decision
  017 AMENDMENT A (`ARCHITECTURE.md` §12 continuation-57 entry;
  `docs/decisions/017-tabbed-dockable-panel-system.md`'s "AMENDMENT A"
  section). Superseded: the hand-rolled two-compartment vertical list
  (§3/§8.2) and the ui-spec's original §A horizontal-strip design (both
  are now superseded shell mechanisms, not the binding one). Survives:
  the simultaneity requirement itself (Layers+Properties together),
  now realized as an `egui_tiles` vertical split rather than two fixed
  compartments. See the ★ Pass 18.x entry's Pass 18.1 bullet.

**Still open, new this session:**
- **(c) Decision 017 Q2 — should panels undock into separate OS
  windows (multi-monitor)?** Needs egui's multi-viewport machinery;
  interacts with crash-safe autosave and the (not-yet-built)
  persistence schema. **Default: no, docked-only** — its own Backlog
  entry if ever wanted.
- **(d) Decision 017 Q3 — confirm the panel-pairing assignment**
  (originally proposed as two fixed compartments: upper =
  Properties/Comments/Bookmarks; lower = Layers/OCGs/batch Tools —
  now, per Amendment A (b), realized as an `egui_tiles` DEFAULT layout
  with the same pairing rather than a fixed compartment boundary; the
  operator can drag to rearrange once built, lowering the stakes of
  this default). **Default: ship the proposed split as the initial
  `egui_tiles` layout.**
- **(e) Decision 018 — bless the operator-visible definition of done**
  (provisionally R86: a Pass that adds/changes operator-facing behavior
  does not ship until observed working in the running application, not
  merely tested headlessly)? **Engineer recommends yes** — every Pass
  3.1–16.2 met its stated gates and shipped an invisible feature; see
  the ★★★★ HEADLINE FINDING. No default stated — this is a "define
  done" call the engineer declines to default unilaterally. **Still
  open — NOT answered this session** (the operator confirmed the Pass-17
  sequencing question (f), a related but distinct question, without
  addressing this one). R86 remains PROPOSED, not in force.
  (Item (f), the sequencing question, is now RESOLVED — see above.)
  **Still unanswered as of continuation 58 (2026-08-03).** Worth noting
  for whenever the operator does weigh in: this session's R85 oracle
  (a headless mechanism, not R86's "observed in the running app"
  mechanism) is what caught the `flatten_fields` silent-data-loss bug,
  the wrong-page search-redaction bug, and the empty-extraction bug —
  R85 and R86 are complementary, not substitutes, and this session is
  evidence for BOTH being worth having, not an argument that R85 alone
  is sufficient. Recorded as a fact for the operator to weigh, not a
  renewed recommendation beyond the one already on record.

**New this session (2026-08-03, decision 019) — five items, none
blocking Pass 19.0's in-progress build:**
- **(g) The `Tw` census middle band (25–60%) — is a control that works
  on roughly half of documents worth permanent surface area? — CLOSED
  AS MOOT this continuation (continuation 67, 2026-08-03).** The census
  ran (`tools/tw-census`) and the loose-metric result (91.6% of show
  operators, 97.4% of glyphs), which is what R91/★ Pass 19.x's decision
  bands are written against, landed cleanly in the ≥60% BUILD band —
  the middle-band product judgement this item asked about never became
  live. The question is not answered, it never had to be. **Kept for
  context, not as a live question:** a strict variant (simple font AND
  contains code 32) does land in the 25–60% escalate band, but is
  flagged fragile (a 12-point swing on removing four outlier files) and
  is explicitly NOT what the decision's bands are written against —
  see decision 019 Amendment E §E.6 and the continuation-67 In-progress
  entry for the full reasoning. If the operator ever wants the strict
  reading revisited as its own question, it would need to be re-opened
  explicitly; it is not implicitly live by virtue of this item's
  original framing.
- ~~**(h) FF-C's rule-13 dependency classification.**~~ **CLEARED
  2026-08-03, no operator decision required.** The MIT license decision
  (2026-08-01) lifted rule 8's gate on FF-C outright but explicitly left
  rule 13 (copyleft always flagged, never decided solo) open against
  whichever specific crate got chosen. That classification has now been
  done — `subsetter 0.2.6` (Typst) is `MIT OR Apache-2.0` and every crate
  in its transitive dependency graph is permissive (no GPL/LGPL/AGPL/MPL
  anywhere); `LEGAL.md` §6.2 step 3 applies (proceed and log, same
  disposition `egui_tiles` got — no operator flag needed). Net cost 2
  new packages (`subsetter`, `write-fonts 0.48.1`); `write-fonts` pinned
  via `subsetter` to `read-fonts 0.39.2`/`font-types 0.11.3`, matching
  what `epaint 0.35`/`skrifa 0.42` already resolve to (a bare
  `cargo add write-fonts` would instead select 0.51.0 and put two
  incompatible font-parser versions in the graph). Full record:
  `PRIOR_ART.md` "FF-C dependency classification (rule 13) — COMPLETE,
  2026-08-03" under Fonts. **What remains for FF-C is scope/sequencing
  only (Q3 of ★ Pass 19.x: FF-H → FF-C → FF-B), not licensing** — do not
  keep describing this item as a licence gate.
- **(i) Cutting the minimal StructTree/`/ActualText` update out of FF-H
  (now filed separately as Backlog's FF-I).** A scoping call decision
  019 made on its own authority (§3.7, building on decision 016 §2's
  "premature" finding) — but it changes the shape of what "finish off
  all the text handling stuff" delivers, since the original FF-H bundle
  named it explicitly. Ken may have counted it inside that directive;
  flagging so the cut isn't discovered only after the fact.
- **(j) List-authoring — re-surfaced, still unanswered.** Unchanged in
  substance from the carried item below; called out here specifically
  because decision 019 touches adjacent territory (text authoring
  scope) without resolving this one — re-surfaced so it is not silently
  assumed either way by proximity.
- **(k) Kerning — a parity gap decision 019 found but did not scope.**
  Dov Isaacs' retained-controls list (the same source establishing
  Acrobat dropped `Tw`/free-form `Ts`, §1.1 of decision 019) names
  kerning among Acrobat's *retained* text-editing controls, alongside
  `Tc` (character spacing) — and pdfce currently has **no kerning
  surface distinct from `Tc`.** Whether kerning is a separate operator
  affordance (pair-kerning adjustment) or is considered subsumed by
  uniform `Tc` tracking is an open scope question, not answered by
  decision 019 or ★ Pass 19.x's slicing.

**New this session (2026-08-03, Pass 19.4 ship) — one item, a
sequencing flag rather than a blocking question:**
- **(l) Engineer sequencing call: GUI redaction-apply flow dispatched
  ahead of item #4 (forms) in the ★★★ operator priority sequence —
  the WORK is now done (Pass 8.1 SHIPPED 2026-08-03, `9a68999`); the
  RATIFICATION question stays open.** With decision 019/FF-H complete,
  the engineer chose to dispatch the GUI redaction-apply flow
  (Backlog → In progress → now Shipped) next, ahead of form-building
  tools, on the grounds that completing a half-shipped **security**
  feature (the GUI used to tell the operator their document "is NOT
  redacted" with no in-app remedy — no longer true, see the Pass 8.1
  Shipped entry) outranks starting a new feature family. This is a
  scope-sequencing judgment call, not itself required by any standing
  rule or prior operator instruction — flagged so it isn't discovered
  only after the fact. **Default: no objection assumed** — item #4
  (form-building tools) is next per the standing order, its
  Acrobat-parity research already done (see the updated Forms Backlog
  entry) — unless the operator retroactively objects to the
  reordering itself.

**New this session (2026-08-03, continuation 71, decision 020) — five
items, none blocking (decision 020 itself explicitly does not authorize
starting Pass 20.0):**
- **(m) Should item #4 (form-building tools) start at all, right now?**
  The real question, per decision 020 §10.1. Item #3 ("finish off all
  the text handling stuff") is only **partially** done — FF-H is
  complete, but **FF-C** (font subsetting/embedding) and **FF-B**
  (cross-block/cross-page reflow) remain unscheduled, and this
  `ROADMAP.md` already states not to treat text-handling as closed
  until they ship or are explicitly deferred. Starting Pass 20.0 while
  item #3 is open would be a **second** undirected resequencing this
  project cycle (after redaction-apply, item (l) above) — two in a row
  is where a priority list stops meaning anything. **Default: do NOT
  start Pass 20.0 until this is answered** — this is a deliberate
  departure from most other open items' "default to the stated
  fallback," because there is no safe fallback here; starting is not
  reversible the way declining a UI convenience is. Bundled sub-question:
  does the "form building tools... if that makes sense" hedge survive
  contact with a six-slice plan, or did Ken have something smaller in
  mind?
- **(n) Signature-field creation for someone else to sign.** Decision
  020 defers signature-field creation to Pass 10 (Signatures) because
  pdfce cannot itself sign — but "place a field so a *different* person
  can sign this document" needs no signing subsystem at all, and is a
  legitimate, small addition to F3 (an empty `/FT /Sig` widget) if
  wanted. **Default: not built** — F3 as scoped omits it; say so if you
  want it pulled forward.
- **(o) Barcode-field creation — confirm the parity subtraction.**
  Decision 020 cuts it outright: no sourcing exists on the creation
  floor, and a barcode field's content is populated by a JavaScript
  calculate action, which decision 009 permanently forbids executing.
  This is a genuine, deliberate gap against feature-for-feature parity,
  not an oversight. **Default: accepted as scoped** — confirm if this
  should be revisited.
- **(p) Retire or re-scope the standing XFA-deprecation open item?**
  Decision 020 makes it non-blocking for this family (both authoring
  branches are decided without it) and recommends re-scoping it to
  "before any XFA *read/fill* work" rather than leaving it a general
  standing item. **Default: left as-is, general** — Ken's call per
  decision 020 §10.5, not the librarian's or engineer's to narrow
  solo. Note: the mirror bullet in the project's own `CLAUDE.md`
  ("Outstanding open items") was not edited by this filing — it is
  outside `pdfce-librarian`'s owned tiers; whoever acts on this answer
  should update both files together.
- **(q) CLI surface migration — not an operator question, filed here
  only so it isn't lost.** Decision 020 §11 flags that its six new
  `forms <verb>` authoring subcommands sit awkwardly next to the six
  already-shipped top-level forms subcommands (`list-fields`,
  `fill-field`, …) and asks whether the shipped six should migrate
  under a `forms` parent for consistency. This is a CLI-surface
  question for the librarian/engineer to settle when F1 is actually
  built, not something needing Ken's input — recorded here only as a
  pointer so it isn't rediscovered from scratch at F1 time.

**New this session (2026-08-03, decision 021 filed) — two items, both
Ken's per `docs/decisions/README.md` (legal call / headline-capability
scope call), neither blocking ★ Pass 21.x's P0 slice (21.0/21.1 do not
need either answered — a face with no `OS/2` data or a Latin-only donor
exercises neither):**
- **(r) Font-EULA policy — NARROWED 2026-08-03 (spec review, decision
  021 §10). What should pdfce do when a donor face's `OS/2` is ABSENT or
  UNPARSEABLE?** The forbids-embedding/forbids-subsetting cases are no
  longer open — the bit semantics are now sourced (fsType `0x000F`
  usage sub-field valid values 0/2/4/8, bit 0 permanently reserved,
  `0x0100` No subsetting, `0x0200` Bitmap embedding only, `0x00F0`/
  `0xFC00` reserved; bits 8–9 MUST be ignored on `OS/2` v0/v1) and R109
  is amended to refuse each by name (`SubsettingNotPermitted` /
  `EmbeddingNotPermitted`) — that much is no longer Ken's call, it is
  spec-directed. **What remains genuinely open, and is narrower and
  sharper than this item's original framing:** the specification is
  silent on absent/unparseable `OS/2`, and on `fsType == 1` (an
  undefined usage-subfield value). **The asymmetry is the trap:**
  `fsType == 0` means *Installable*, the MOST permissive value, so
  "absent" cannot be modelled as `0` — a missing-data default of "most
  permissive" is exactly the silent-"permitted" failure R109 already
  forbids. Options per decision 021 §7.1, unchanged: refuse outright ·
  disclose and require an explicit operator acknowledgement · disclose
  and proceed. Deliberately not picked in the decision record — R109 is
  written to accept whichever policy is chosen. **Permanent finding,
  also recorded (decision 021 §10 C-7/C-8, `pdfce-spec-librarian`): the
  fsType↔PDF bridge exists in NEITHER specification** — ISO 32000-1
  names no field, OpenType never mentions PDF — which is why this stays
  an operator call and not a lookup.
  **INTERIM DEFAULT SHIPPED (continuation 76, 2026-08-04, `58fe3f6`) —
  this does NOT close the question.** Absent/unparseable `OS/2` now
  proceeds, disclosed as unknown. A second case also ships the same
  disclose-and-proceed default: `fsType == 4` (Preview & Print), which
  permits the embed itself but additionally obliges the *document*
  stay read-only thereafter — an obligation on every future reader
  that pdfce has no PDF field to express and cannot enforce, so
  "proceed" here is a pragmatic default, not a claim that the
  obligation is satisfied. Both were shipped as the engineering default
  needed to ship code at all, not as a resolution — Ken can still pick
  refuse-outright or disclose-and-acknowledge for either case, and
  R109 is written to accept whichever policy is chosen (unchanged from
  the narrowing above).
- **(s) Complex scripts (Arabic/Devanagari/Thai) — refuse by name at
  Pass 21.0, or disclose loudly and let the operator decide?** FF-C
  plus standing rule R17 (no shaping, ever) means these scripts would
  EMBED but RENDER WRONG (glyphs placed by advance, no GSUB/GPOS).
  Recommendation (decision 021 §7.2): refuse by name — painting
  confident nonsense is the rule-4 failure — but it caps a headline
  capability (FF-C's non-Latin story becomes "CJK/Cyrillic/Greek/Hebrew
  yes, Arabic/Devanagari/Thai no"), so it is worth Ken's call rather
  than a solo engineering decision.
- **(t) Does decision 022 (Pass 22.0) actually fix what was originally
  reported, or does decision 023 (Pass 23.2) — filed 2026-08-04, not
  yet promoted into a Pass entry above; both records exist only at
  `docs/decisions/022-annotations-in-canvas-selection.md` and
  `docs/decisions/023-object-tool-level-navigation-and-dimension-authoring-controls.md`.**
  Filed by `pdfce-librarian` per the 2026-08-04 terminology-ruling
  dispatch (see the ★ Terminology ruling entry, Standing rules, below)
  — a scoping question surfaced while qualifying "dimension" mentions
  across the record, not itself a terminology fix.
  **What the audit found:** decision 022 is scoped almost entirely
  around **ce dimensions** — pdfce's own authored `/Line`+
  `/IT /LineDimension` annotations. Its root-cause finding (§1) is that
  `decompose_page` never reads `/Annots`, so a ce dimension is painted
  but not selectable or deletable by any surface. Decision 023 inherits
  the same ce-dimension framing for re-measure (§5) and the format
  surface (§6), but its §0 finding 2 / §1.2 independently identifies a
  **second, structurally identical paint/select asymmetry — this time
  in form XObjects**: `pdfce-render`'s interpreter recurses into a
  `Do` on a form and paints its contents individually, while
  `decompose.rs` emits one opaque object for the same `Do`, so every
  line inside a placed CAD block (a title block, a hatch, a nested
  drawing block) is visible and unselectable. That is a **pdf
  dimension**-shaped defect (foreign, CAD-exported content), not a ce
  one, and it is Pass 23.2 (level navigation), not Pass 22.0, that
  closes it.
  **The scoping question:** the operator's original complaint that
  "some objects don't seem to box select, like dimension lines and
  dimensions in that drawing" was describing a CAD-exported drawing —
  almost certainly **pdf dimensions**, not pdfce-authored ce ones. If
  the unselectable geometry in that drawing was flattened paths or a
  placed form (the common CAD-export shape), **Pass 22.0 alone does
  not fix what was reported** — 22.0 fixes a real defect (ce dimensions
  were never selectable at all, a genuine gap the operator had not yet
  hit per decision 022 §0) but a different one. Pass 23.2 is the
  candidate fix for the literal original report.
  **Recommendation, not a decision:** confirm with the operator which
  the original drawing's unselectable "dimension lines and dimensions"
  actually were (open the file, or ask) before treating Pass 22.0's
  ship as closing the original complaint. Both Passes are worth
  building regardless — they close two independently real, differently
  scoped instances of the same paint/select-asymmetry class (decision
  022's own proposed standing rule §8) — but only one of them is the
  literal fix for what was reported, and it may not be 22.0.
  **Also flagged, not itself part of this question:** decisions 022
  and 023 are not yet filed into `ROADMAP.md` as Pass 22.0/23.0–23.3
  entries or into `ARCHITECTURE.md` §12 — they exist only as decision
  records (`docs/decisions/`). Filing them (including any standing-rule
  number assignment against decision 022 §8 / decision 023 §9) is owed
  as a separate `pdfce-librarian` dispatch ("roadmap update — new
  request" / "decision log entry"), deliberately not done in this same
  filing so that filing can happen with the qualified `pdf dimension`/
  `ce dimension` terminology from day one rather than needing its own
  correction pass.
  **FILED (continuation 80, 2026-08-04) — the deliberately-deferred
  filing named above is now done.** Decisions 022 and 023 are promoted
  into `ROADMAP.md` as the "Object-tool selection, level navigation &
  ce-dimension authoring controls — Pass 22.0 / Pass 23.0–23.3" Backlog
  bucket (below), with standing rules R111–R120 assigned and new open
  questions (u)–(ab) filed (below) and `ARCHITECTURE.md` §12 dated. **The
  scoping question itself — which kind of dimension the operator's
  original drawing actually contained — remains open**, unresolved by
  this filing; only the "file it" action item is discharged. **Librarian's
  ordering read, added at filing time:** the fix-oriented build order is
  **22.0 → 23.2** (23.2 is the slice that closes the form-XObject
  asymmetry most likely responsible for the original report); decision
  023 §7.1's 23.0-first ordering is a pure engineering-risk argument and
  does not fix what was reported — see the new Backlog bucket's own
  "Librarian's read on slice ordering" paragraph for the full reasoning.
  This does not resolve the confirmation step above; it only says what
  order to build in once that confirmation lands (or in parallel with
  seeking it).

**New this session (2026-08-04, continuation 80) — eight items, drawn
from decisions 022 §9 and 023 §10's "for the operator" sections, filed
here for the first time (both decisions themselves only listed them
inline; none had a ROADMAP letter until now). None block Pass 22.0/23.0
starting build — each is a scope/preference question about a LATER
slice or an already-decided default that can be revisited:**
- **(u) Widget (form-field) annotations — refuse deletion by name, or
  cascade into form surgery?** Decision 022 §6.2/§9 item 2 recommends
  refuse (`AnnotationIsWidget`) — deleting a widget without its
  `/AcroForm /Fields` entry orphans the field, which is form surgery
  (decision 020's R100–R106 family), not annotation surgery. **Default:
  refuse, as recommended** — confirm if cascading into form deletion is
  wanted instead.
- **(v) R58's literal wording is already stale — fix the rule text, or
  leave it and keep accumulating named exceptions?** Decision 022 §5.4/
  §9 item 3 found that R58's binding text ("every removal/scrub
  operation forces a full rewrite") is **already contradicted** by two
  shipped operations (`delete_object`, Pass 9c-min; `delete_redaction_mark`,
  Pass 8) that correctly stay under incremental save, using the same
  confidentiality-contract-vs-not distinction §5.11/R70 already
  established for in-place text editing. Decision 022 explicitly declines
  to narrow a standing rule's scope solo. **This filing adds a flagged
  staleness note to `ARCHITECTURE.md` §5.9/§5.12 and to R58's own
  Standing-rules bullet, below, WITHOUT rewriting R58's binding text** —
  the correction itself (narrowing "every removal/scrub" to
  "confidentiality-contract removals") is what needs your confirmation.
  **Default if unanswered: the flag stays, the text stays unchanged, and
  Pass 22.0's `delete_annotation` becomes a THIRD unreconciled exception**
  (see the §5.9/§5.12 note for exactly which shipped/proposed operations
  already sit outside R58's literal scope).
- **(w) Per-group vs. per-ce-dimension display format?** Decision 023 §6.3/
  §10 item 1 recommends **per-group**, with "draw a second group at the
  same scale" as the free escape hatch if a workflow needs two formats
  inside one scale context. **Default: per-group, as recommended** —
  confirm this fits, or the group model needs revisiting before Pass
  23.0 ships.
- **(x) A GUI toggle for `reduce` (`3/4"` vs. the shipped-default `6/8"`
  kept-denominators), or CLI-only?** Decision 023 §6.4/§10 item 2 ships
  `reduce` CLI-only, disclosed in the GUI rather than exposed as a
  checkbox (nobody asked for it). **Default: CLI-only, as scoped** —
  say so if a GUI toggle is wanted.
- **(y) Form-XObject "make this placement independent" un-sharing
  command — wanted, or is refuse-by-name (`FormStreamIsShared`) the
  whole feature?** Decision 023 §1.4/§10 item 3 — a form placed N times
  shares one content stream, so editing inside it is refused while
  N > 1 (Pass 23.3). An un-share command (duplicate the stream, rewrite
  the one `Do`) is a real, deliberate minimal-diff-breaking trade, not a
  freebie. **Default: refuse only, no un-share command** — say so if
  wanted as a fast-follow.
- **(z) Fixed three navigation levels, or as deep as the file's own
  structure goes?** Decision 023 §1.3/§10 item 4 recommends **as deep as
  the file goes**, terminating at nodes — a fixed-three design would
  either refuse to enter a nested block or lie about where the operator
  is standing. **Default: as deep as the file goes, as recommended** —
  this is the only item in this block where the alternative is
  materially worse, not just different, but confirming avoids
  discovering a disagreement mid-build.
- **(aa) Node delete between two curve segments → a straight join with
  disclosed curvature loss, or is a reviewable curve-refit wanted as a
  later feature?** Decision 023 §4.5/§10 item 5 ships the straight-join
  semantics now (deterministic, spec-trivial) and names curve-preserving
  refit as a deferred, larger, rule-4-governed (reviewable preview)
  feature, not built in 23.3. **Default: straight-join now, refit
  deferred, as scoped** — confirm if refit should be pulled forward.
- **(ab) Should snapping see inside form XObjects?** Decision 023 §2.4/
  §10 item 7 — today it cannot, which means **the operator cannot
  author a ce dimension referencing a line inside a placed CAD block**
  even after Pass 23.2
  makes that geometry navigable/selectable. Consuming it in the snap
  engine is a separate, real cost (a snap query over a deeply-nested
  page is materially larger than today's top-level-only query) — not a
  freebie alongside 23.2. **No default stated** — this is a genuine
  cost/benefit call, not a "ship the obviously-better option" item.

**Carried from prior sessions (unchanged, still open):**
- Push/publish the local commit chain to a remote — separate,
  not-yet-granted authorization (see "In progress" GIT STATUS above).
  **Now more precise (continuation 59, 2026-08-03): there is no git
  remote configured at all** — the commit chain exists solely on this
  machine (30 commits at continuation 59; 39 at continuation 61's
  filing; 43 as of continuation 63; **45 as of continuation 64**,
  confirmed by `git rev-list --count HEAD` directly, not
  engineer-reported-then-recounted this time).

  **The 39→43 arithmetic flag is RESOLVED (engineer, 2026-08-03): 43
  was correct, and the flag was right to be raised.** The apparent gap
  came from anchoring on a stale figure: 39 was the count at `743e463`,
  but the immediately preceding filing was `0c385a9`, where the count
  was **40**. So 40 + 3 (`f45d8d6`, `38fffad`, `1a2e265`) = 43.
  Recording the resolution rather than just the corrected number,
  because the failure mode is reusable — **a running total is only as
  good as the anchor it is added to, and "the last number I filed" is
  not necessarily "the number at the last commit I am counting from"**
  (this is now recorded as the reusable lesson behind the discipline,
  not a one-off correction). This is the THIRD filing-integrity issue
  this exact audit habit has caught in this project — see standing rule
  R87's own note for the first two. A verified backup bundle exists as
  a stopgap
  (`D:\Dev\pdfce-backups\pdfce-20260803-1145.bundle`, superseding the
  earlier `...0830.bundle`; covered all 43 commits at the time of its
  own creation and is **already two commits behind again** as of
  continuation 64's `5c1f5dc`/`603b051` — regeneration is a
  point-in-time action, not a standing guarantee, not a substitute for
  an actual push decision). Also flag: the branch is still named
  `pass-8-redaction` though it now carries Passes 9 through 19.4 plus
  this session's fixes — worth renaming whenever a push is authorized.
  **UPDATED (continuation 71, 2026-08-03): 62 commits, still no
  remote** (engineer-reported this filing, includes `09be28d` and
  `d9960cd`). **UPDATED again (continuation 73, 2026-08-03): 66
  commits, still no remote.** Engineer-reported and this time
  spot-verified independently with `git cat-file -t` against six
  hashes spanning the range (`d30842c`, `4dc8cf8`, `d738950`,
  `1111652`, `d9960cd`, `09be28d` — all confirmed `commit` objects),
  not merely relayed. Per-commit hash listing is no longer tracked
  exhaustively past continuation 62's count — verify the live count
  with `git rev-list --count HEAD` rather than trusting this number
  once further commits land. **The backup-bundle staleness flagged above is CLOSED as
  of continuation 70** — refreshed to
  `D:\Dev\pdfce-backups\pdfce-20260803-1936.bundle`, `git bundle
  verify` reports "records a complete history," current as of `d9960cd`
  at continuation 70's filing time; superseded `...1145.bundle`. Per
  the standing pattern above, this is still a point-in-time snapshot,
  not a substitute for an actual push decision, and will drift behind
  again with the next commit.
  **UPDATED (continuation 75, 2026-08-04): 74 commits, still no
  remote.** Six hashes spanning Pass 21.0's build independently
  verified with `git cat-file -t` (`88b9487`, `0c4f490`, `d4e7355`,
  `5b7bed3`, `eb0bde5`, `48c6b77` — all confirmed `commit` objects).
  Backup bundle refreshed to
  `D:\Dev\pdfce-backups\pdfce-20260804-0015.bundle`, `git bundle
  verify`-clean; supersedes `...1936.bundle`. Same standing caveat:
  point-in-time snapshot, will drift behind again with the next
  commit, not a substitute for an actual push decision.
  **UPDATED (continuation 76, 2026-08-04): 79 commits, still no
  remote.** Five hashes spanning this continuation's Pass 21.1 build
  independently verified with `git cat-file -t` by the operator
  (`58fe3f6`, `c0ed638`, `8e08e80`, `87d3cb0`, `6b69956` — all
  confirmed `commit` objects). **The backup bundle is now STALE, two
  commits behind** — `...0015.bundle` (continuation 75) does not cover
  any of the five hashes above; not yet refreshed this continuation.
  Flag carried forward rather than silently fixed: a fresh
  `git bundle create` + `git bundle verify` pass is owed before the
  next natural checkpoint.
  **UPDATED (continuation 77, 2026-08-04): 79 commits, unchanged count
  (librarian-only filing, no code shipped).** Backup bundle refreshed
  to `D:\Dev\pdfce-backups\pdfce-20260804-0325.bundle`, `git bundle
  verify`-clean, current to `6b69956` — discharges continuation 76's
  staleness flag.
  **UPDATED (continuation 78, 2026-08-04, SESSION-ENDING FILING): 82
  commits, still no remote.** Three new commits landed this
  continuation (`31d2fdc`, `b98589a`, and the fixture/survey commit at
  HEAD — the first two independently `git cat-file -t` verified by the
  operator as `commit` objects; the third is recorded as "HEAD at
  session end," not a specific hash string, per the operator's own
  instruction — the count was confirmed, a hash for that one commit
  was not separately verified this filing). Backup bundle refreshed to
  `D:\Dev\pdfce-backups\pdfce-20260804-final.bundle`, `git bundle
  verify`-clean, current to HEAD — supersedes `...0325.bundle`. Test
  suite: 1806 tests passing; `cargo fmt --check`, `cargo clippy -- -D
  warnings`, `tools/check-ui-strings.sh`, `tools/check-ledger-numbers.py`
  all clean; `cargo tree -p pdfce-core` / `-p pdfce-render` still
  GUI-free. Same standing caveat as every prior refresh: a point-in-time
  snapshot, will drift behind again with the next commit, not a
  substitute for an actual push decision — **this is the session-ending
  filing; the next session should re-verify the bundle is still current
  before assuming it covers whatever it finds on disk.**
- Encryption (Pass 5 / decision 007)'s `/R 6` sourcing method and the
  `LEGAL.md` §2 Adobe-supplement contradiction — both still gate its
  scoping when it activates.
- `LEGAL.md` §2 itself — still open (see LEGAL.md).
- List-authoring scope call — still explicitly unresolved; do not fold
  into "text-handling" (priority #3) without a further, explicit
  operator answer naming it specifically. **Re-surfaced, not resolved,
  by decision 019 — see item (j) above.**

## Standing rules

- **Documentation-first.** Every module gets a thorough header
  docstring (purpose, contracts, spec citations); every function gets
  a doc comment explaining WHY; every Pass gets a roadmap entry the
  same session it ships.
- **Spec-fidelity discipline.** Never implement spec-governed byte
  layout, filter, or structural behavior from memory — check
  `D:\Dev\Rag-Specialized\PDF_Spec\` first (via pdfce-spec-librarian
  if the answer isn't already cached in a prior session's notes).
- **Feature-fidelity discipline.** Before scoping a Backlog bucket
  into a real Pass, consult `D:\Dev\Rag-Specialized\Acrobat_Features\`
  (via `pdfce-acrobat-librarian` if the bucket isn't cataloged yet) so
  acceptance criteria reflect actual Acrobat Pro behavior, not
  assumption. That RAG describes capabilities only — never use it (or
  let it lead to) copying Acrobat's GUI structure; pdfce's UI is
  designed independently.
- **GUI-core separation is load-bearing, not a suggestion.** See
  `ARCHITECTURE.md` §3 invariant. Verify with `cargo tree` on any Pass
  that touches `pdfce-core` or `pdfce-render` dependencies.
- **Round-trip / minimal-diff editing** per `ARCHITECTURE.md` §5,
  with redaction as the sole deliberate, explicit exception.
- **Fuzzy, never sneaky** for every algorithmic suggestion (OCR,
  auto-detected fields, suggested Bates ranges, etc.).
- **Test-corpus sourcing discipline** per `LEGAL.md` — no
  unknown-provenance real-world PDFs in the repo.
- **Rust Style Guide + API Guidelines compliance.** `cargo fmt --check`
  and `cargo clippy -- -D warnings` clean before any Pass ships; any
  `pub` item added to `pdfce-core` checked against
  `D:\dev\rag\rust\rust-style-guide-and-api-guidelines.md`. See
  `ARCHITECTURE.md` §8.
  **AMENDED 2026-08-03 (continuation 59):** the decision-002 R1 `ui-strings`
  enforcement (see below) runs locally as `tools/check-ui-strings.sh`,
  not only as a CI job — it had been red at baseline on 140 hits (see
  the `ui-strings` CI gate Shipped entry above) before this continuation
  fixed it; run it locally before pushing, don't rely on CI alone to
  surface a violation.
- **Every feature Pass considers both `pdfce-gui` and `pdfce-cli`.**
  Not every feature needs a CLI subcommand on day one, but the default
  is to ship both together — see the "CLI batch operations" backlog
  entry above.
- **No dependency without a license check.** Every new `Cargo.toml`
  entry is classified permissive/weak-copyleft/strong-copyleft before
  it's added; copyleft is always flagged to the user, never decided
  solo. See `LEGAL.md` §6. Attribution is generated (`cargo-about` →
  `THIRD_PARTY_LICENSES.md`), never hand-maintained.
- **Adversarial-input hardening is not optional.** Every filter
  decoder gets an output-size ceiling; every recursive structure gets
  a depth/cycle guard; the parser gets a fuzz target before Pass 1
  ships. See `ARCHITECTURE.md` §10.
- **Every resource guard is validated against veraPDF's §6.1.12
  implementation-limits suite before it ships** (`veraPDF-corpus/*/6.1
  File structure/6.1.12 Implementation limits/`). That suite exists
  precisely to catch guards chosen by intuition rather than
  measurement — it deliberately contains conformant files that sit
  right at (or past) Annex C's non-binding suggested limits, and
  PDF/A §6.1.12 forbids a reader from imposing those limits anyway.
  Two incidents so far, same bug shape both times: `MAX_TOKEN_LEN` (8
  KiB, intuition) rejected a valid file and was raised to 1 MiB (Pass
  1.1 item 2); `MAX_XOBJECT_DEPTH` (16, intuition) rejected a
  conformant 32-deep form-XObject chain and was raised to 64 (Pass 1.1
  item 5, 2026-07-30). Any new depth/count/size guard added to
  `pdfce-core` or `pdfce-render` gets a corpus run against this suite
  specifically before it ships, not just the general corpus.
- **Undo/redo is command-log-based, built into the first editing
  Pass, not retrofitted.** The dirty-set for incremental save is
  computed as a diff against the base revision at save time, never as
  the union of every command ever run. See `ARCHITECTURE.md` §11.
- **No network calls without explicit user opt-in, ever.** No
  telemetry, no silent update-checks, no phone-home. See
  `ARCHITECTURE.md` §1.1.
- **Solo by default.** Workflow tool only when parallelism is genuine
  and the user has opted in (ultracode or explicit request).
- **KenAgent decision routing (operator process rule, 2026-07-30).**
  Non-trivial technical decisions are routed through the "KenAgent"
  autonomous-builder agent, which returns a decision with reasoning in
  two forms: JSON (consumed to implement the decision) + Markdown
  (archived to `docs/decisions/NNN-slug.md` for project history).
  Legal/license decisions remain Ken's directly — unchanged by this
  rule.
- **i18n/l10n discipline R1–R8 (decision 002, 2026-07-30).** Binding
  from Pass 1 onward, per `docs/decisions/002-i18n-timing.md` §6.1
  (the full-text authority if any condensation below is ambiguous):
  - **R1 — Single catalog.** Every user-facing string in `pdfce-gui`
    lives in `crates/pdfce-gui/src/ui_text.rs` and nowhere else.
    Entries are `pub fn`, never `pub const`. Enforced by the
    `ui-strings` CI job (whitespace-bearing string literals outside
    `ui_text.rs` fail the build unless `// ui-text-exempt: <reason>`).
  - **R2 — No sentence assembly.** Never concatenate message fragments
    or `format!` one catalog entry into another. One entry = one
    complete, grammatically self-contained message with inline named
    placeholders.
  - **R3 — No English-width layout.** Never size a panel/column/grid
    cell/`add_sized` widget to fit an English string. Prefer intrinsic
    sizing; where a fixed extent is unavoidable, budget +50% over the
    English measurement and document the number in a comment.
  - **R4 — Structured errors in core.** `pdfce-core`/`pdfce-render`
    error variants carry the data needed to render a message, never
    pre-formatted prose. Their `Display` is English, diagnostic,
    stable, never localized. Front ends own presentation.
  - **R5 — Locale-invariant machine output.** `pdfce-cli` stdout is
    machine-readable and locale-invariant **permanently** — it never
    varies with `LANG`/`LC_ALL`; the CLI is English-only by design
    (clap's own text is untranslatable, clap-rs/clap#380). Human
    diagnostics go to stderr. The exit-code contract is likewise
    locale-invariant.
  - **R6 — Formatting helpers.** Page counts, byte sizes, timestamps,
    and Bates ranges shown in the GUI come from helper functions in
    `ui_text.rs`, not inline `format!` at the call site.
  - **R7 — Document text is not deferred.** Non-Latin text *inside*
    PDF documents (render/extract/search/edit via `pdfce-render`'s own
    text stack) is a Pass-1-onward requirement, entirely separate from
    the English-only UI chrome. `pdfce-render` must never inherit
    epaint's bidi gap or Latin-only fonts (decision 002 §7).
  - **R8 — No i18n dependency without a trigger.** No i18n crate
    enters any `Cargo.toml` until a decision 002 §9 trigger fires;
    `LEGAL.md` §6.2 applies as normal when one does. `gettext-rs` is
    pre-disqualified (LGPL statically linked on Windows, WASM-broken —
    decision 002 §3.4).
- **Distribution-posture discipline R9–R16 (decision 003, 2026-07-30).**
  Binding, per `docs/decisions/003-distribution-posture.md` §6.1 (the
  full-text authority if any condensation below is ambiguous):
  - **R9 — One supported platform at a time.** pdfce claims support for
    exactly the platforms on which a human has run the
    `ARCHITECTURE.md` §6 packaging smoke test on real hardware. Today:
    Windows x64, nothing else. A green CI job is a compile signal,
    never a support claim, and never appears in user-facing copy as one.
  - **R10 — Platform-clean by construction.** No `#[cfg(target_os)]` /
    `#[cfg(windows)]` / `#[cfg(unix)]` in `pdfce-core` or
    `pdfce-render`, ever. Platform conditionals in
    `pdfce-gui`/`pdfce-cli` carry a comment naming why. Build-level
    platform specificity stays target-scoped in `.cargo/config.toml`.
    No path-separator, line-ending, or filesystem-case assumptions
    anywhere.
  - **R11 — CI runs natively only where pdfce ships**, or where the
    runner is free AND the failure is actionable. Every other
    platform's compile signal comes from cross-target `cargo check` on
    the ubuntu runner. The `wasm32-unknown-unknown` check of
    `pdfce-core` + `pdfce-render` is a first-class invariant guard
    (protects the web-fork premise of `ARCHITECTURE.md` §3) and ranks
    above macOS.
  - **R12 — No network client in the tree.** No HTTP/TLS/socket client
    crate may enter any pdfce crate. Enforced fail-closed by the
    `no-network` CI job. Unlocking requires a NEW decision record
    amending 003, naming the crate and the feature it serves.
    `pdfce-core` and `pdfce-render` may never contain network code
    under any future decision.
  - **R13 — pdfce never self-updates.** Never downloads a file the
    user did not ask for, never replaces its own binary, never
    executes anything it fetched, never launches an installer.
    Permanent.
  - **R14 — Update discovery is external by default.** Manual
    replace-the-folder plus package-manager manifests are the shipping
    answer. Any in-app checker obeys decision 003 §6.4 in full —
    opt-in, off by default, user-initiated per check, display-only, no
    identifiers, kill-switchable, disclosed in §6.3's exact terms.
  - **R15 — The distribution folder is partitioned.** Replaceable
    payload and user state are separate; user state never sits loose
    among the binaries; the documented update procedure names exactly
    which files to keep. Binding from the first Pass that persists
    anything.
  - **R16 — Release integrity and attribution track reality.** Every
    release publishes SHA-256 checksums for every artifact.
    `about.toml`'s `targets` lists exactly the platform triples
    actually shipped — no more, no fewer.
- **Text-rendering/font discipline R17–R22 (decision 004, 2026-07-30).**
  Binding, per `docs/decisions/004-text-rendering-fonts.md` §6.1 (the
  full-text authority if any condensation below is ambiguous):
  - **R17 — The render path never shapes.** No GSUB/GPOS, no bidi, no
    script itemization, no ligatures, no mark positioning, no kerning
    between a `Tj` and a painted glyph — PDF content streams carry
    already-positioned glyphs; shaping them corrupts correct output.
    `harfrust` may only ever enter a future text-**authoring** path;
    `unicode-bidi` only a text-**extraction** reading-order path.
    Neither may become a `pdfce-render` dependency.
  - **R18 — No hinting, ever, in `pdfce-render`.** Always
    `DrawSettings::unhinted(Size::unscaled(), ..)`; outlines are taken
    in font units and transformed by pdfce's own `Trm × CTM`.
  - **R19 — Rendering is font-deterministic by default.**
    `pdfce-render` never discovers, opens, or reads a font from the
    filesystem, the environment, or the OS. Its default
    `FontEnvironment` is the bundled 14 and nothing else. Same input →
    same pixels on every machine, in the CLI, and in the WASM fork.
    Additional faces arrive only through the public API, supplied by
    the shell.
  - **R20 — Substituted glyph shapes are always disclosed.** Any glyph
    painted from a substitute face rather than the document's own
    embedded program is counted in `Diagnostics` and surfaced in the
    GUI diagnostics panel and the CLI summary.
  - **R21 — One font parser in the read path.** `skrifa` (with its
    `raw` re-export of `read-fonts`) is the single font-program parser
    for `pdfce-render`. No second parser enters without a new decision
    record. Its version tracks whatever `epaint` resolves to;
    `cargo tree --duplicates` must never show two `skrifa` or
    `read-fonts` majors.
    **AMENDED 2026-08-03 (decision 021 §3.2, scope note).** R21 governs
    the READ path (its own title says so). FF-C's `subsetter` crate
    (write path, `pdfce-render`) contains an internal reader that
    parses the donor face to build a `SubsetPlan` — this is the escape
    clause ("no second parser without a new decision record")
    discharged, not evaded: that internal reader never renders a
    glyph, never chooses a substitute, never reaches `Diagnostics`;
    its output is re-read by skrifa if ever displayed. The
    `cargo tree --duplicates` guard is unchanged and verified to hold
    (`subsetter`, `default-features = false`, resolves to the SAME
    `skrifa 0.42.1`/`read-fonts 0.39.2` pdfce already pins).
  - **R22 — Bundled font provenance is verified, not asserted.** Every
    bundled face carries a recorded source URL, upstream commit,
    SHA-256, extraction method and license text, and a test asserts
    each face's advance widths against the APAFML-sourced width
    tables, with the known exceptions (`Euro`, guillemets) enumerated
    explicitly rather than tolerated silently.
- **Image-codec discipline R23–R28 (decision 005, 2026-07-30).**
  Binding, per `docs/decisions/005-image-codecs.md` §6.1 (the
  full-text authority if any condensation below is ambiguous):
  - **R23 — Image codecs are a terminal stage, not byte-stream
    filters.** `filters::decode_stream` never decodes
    DCT/CCITT/JBIG2/JPX — it returns `FilterError::ImageCodec(name)`,
    and codec output crosses the API as a `CodedImage` (samples +
    declared geometry + declared colour model), never as a bare
    `Vec<u8>`. `LZWDecode` is a byte-stream filter and stays in the
    cascade.
  - **R24 — Codec crates are built with SIMD and unsafe features
    OFF.** `zune-jpeg` without `x86`/`neon` is compiler-enforced
    `forbid(unsafe_code)`; `hayro-*` without `simd` pulls no
    `fearless_simd`. CI asserts the **feature state**, not just the
    dependency set, because feature unification is transitive and
    silent. Enabling any SIMD feature requires a NEW decision record,
    a measurement showing it matters, and fuzz coverage of the
    enabled path.
  - **R25 — Every codec ceiling is set explicitly by pdfce, never
    inherited from a vendor default.** (`zune-jpeg`'s default
    16,384-pixel dimension cap would reject legitimate wide scans.)
    Ceilings derive from `MAX_IMAGE_PIXELS` and get a veraPDF §6.1.12
    run before shipping.
  - **R26 — The codec layer never decides colour.** `pdfce-core`
    hands `pdfce-render` the codec's own samples and its declared
    colour model; only `pdfce-render` applies `/Decode` (§8.9.5.2),
    resolves `/ColorSpace`, and reconciles the two. No codec adapter
    may apply a `/Decode` array, an "Adobe CMYK inversion", or any
    polarity flip of its own — with one named exception: CCITT's
    `/BlackIs1`, which is a Table 11 *filter parameter* and therefore
    belongs to the adapter.
  - **R27 — Unsupported codec sub-features fail clean and are counted
    BY NAME.** Arithmetic-coded JPEG, 12-bit JPEG, lossless JPEG, an
    unknown Adobe transform byte, a JPX progression-order change in
    tile-parts: each is a distinct named diagnostic. Never a grey
    box, never a guessed pixel, never a generic "decode failed."
  - **R28 — Read-compat only: pdfce writes none of these codecs.** No
    image encoder enters any pdfce crate without a new decision
    record. LZW specifically is never written under any future
    decision (§7.4.4.1 NOTE 1 makes Flate strictly better on every
    axis but encode speed). Same posture as RC4 in the encryption
    bucket.
- **CMYK/JPEG-polarity discipline R29–R31 (decision 006, 2026-07-31).**
  Binding, per `docs/decisions/006-cmyk-jpeg-inversion.md` §6.1 (the
  full-text authority if any condensation below is ambiguous):
  - **R29 — pdfce never applies an "Adobe CMYK inversion."**
    Four-component DCTDecode samples reach `pdfce-render` exactly as
    the codec's own Table 13 / TN #5116 §13.1 mandated transform
    produced them. `/Decode` is the sole polarity control for every
    image, in every colour space, at every bit depth. No
    APP14-conditioned, transform-byte-conditioned,
    component-count-conditioned, or producer-sniffed polarity flip may
    be added to any layer without a NEW decision record citing a
    source the four-engine consensus does not already contradict.
    Sourced: pdf.js, pdfium, MuPDF (PDF path) and Poppler all
    implement exactly this; marker-gated inversion has been shipped
    and reverted twice upstream (cairo issue 156, Firefox bug 674619).
  - **R30 — The residual inversion risk is reported, never repaired.**
    A four-component DCT image with **no `/Decode`** and an
    **effective `ColorTransform` of 0** (APP14 transform byte 0, or no
    Adobe marker) is the one shape where the undocumented Photoshop
    polarity convention can still produce a photographic negative. It
    gets its own named diagnostic
    (`dct_cmyk_polarity_unverifiable`), distinct from the benign YCCK
    census (`dct_cmyk_images`). If a repair is ever offered it is an
    operator-facing, per-image, reviewable toggle — per `CLAUDE.md`
    rule 4 — never a silent auto-apply and never a default. All four
    reference engines share this gap and accept it; pdfce's
    differentiator is that it names it.
  - **R31 — A reference decoder is evidence only after its own
    conventions are verified.** Before treating any third-party render
    or decode as ground truth, establish what normalization the tool
    applies on its own initiative, and record it in the decision.
    Pillow applies rawmode `CMYK;I` to every four-component JPEG
    ("assume adobe conventions", no marker test), which produced a
    false positive for this exact investigation. Prefer a full-page
    render from a production PDF engine (pdfium via `pypdfium2` is the
    cheapest) over a bare image-library decode, and prefer a
    source-level read of the condition over both.
  - **R26 — status change, text unchanged (decision 006).** Its clause
    forbidding "an 'Adobe CMYK inversion', or any polarity flip of its
    own" was provisional pending decision 005 §5.5's open question. It
    is now **permanent and sourced** via R29. One clarification is
    added: the codec adapter **may observe** the image dictionary for
    the purpose of emitting diagnostics (`dct::decode` already
    receives `dict`). **Observing is not applying.**
- **Incremental-save-writer discipline R32–R41 (decision 007,
  2026-07-31).** Binding, per
  `docs/decisions/007-next-subsystem-after-read-stack.md` §6 (the
  condensed authority; the record's §5 and §8 govern if any
  condensation is ambiguous). *Librarian note (2026-07-31): the
  archived record's header says "Adds standing rules: R32–R40" —
  R41 was appended by the consultation's final-message patch (record
  Appendix B, merged into the effective JSON in Appendix A) and is
  part of the effective decision; the archived file is not edited.*
  - **R32 — Byte identity is per object, not per file.** §5's
    invariant is a per-object-definition contract. A full rewrite
    legitimately changes xref offsets, the trailer and `startxref`,
    and can never be file-identical. Therefore: `save_full` asserts
    per-object-definition byte identity **plus** raster identity;
    `save_incremental` with an empty dirty set asserts **whole-file**
    byte identity. Conflating the two produces a test that fails
    universally or passes vacuously.
  - **R33 — The writer never normalizes.** The xref form (table vs
    stream) matches the input's newest section. The PDF version
    header is never bumped. Object streams are never introduced,
    expanded, or reorganized as a side effect. Numbers, names and
    strings are never reformatted on a passthrough object.
    Normalization produces a plausible, working, wrong file — the
    hardest defect class to notice.
  - **R34 — The round-trip gate guards every writer Pass.** Any Pass
    touching the writer re-runs the corpus round-trip harness.
    Regressions block the Pass. The invariant stops depending on
    anyone remembering it.
  - **R35 — Incremental save structurally preserves superseded
    content; removal operations must therefore refuse it.** An
    incremental update leaves the prior bytes of every edited object
    in the file by construction. **Redaction — and any operation
    whose contract is removal — forces a full rewrite and must refuse
    incremental save.** A test greps the saved bytes for the redacted
    content. This closes a real, previously undocumented gap between
    §5, §11.2 and the fact that incremental is the *default* save
    mode.
  - **R36 — Save mode is chosen by contract and disclosed, never
    chosen silently.** Signatures present → incremental by default; a
    full rewrite must name what it destroys before proceeding.
    Linearized input → an incremental save warns that Fast Web View
    is invalidated (never silently re-linearizes; that is the
    Optimization bucket). Redaction → full rewrite, mandatory, per
    R35.
  - **R37 — The serializer takes an object-encoder seam from day
    one.** Identity implementation in Pass 3.0; the §7.6 crypt stage
    plugs into it in Pass 5. No layer below the seam writes bytes to
    the file. One page of design now; a cross-cutting rewrite avoided
    later.
  - **R38 — Compressed-object edits promote; they do not rewrite
    containers.** A touched `Provenance::ObjectStream` object is
    promoted to an uncompressed object and superseded by a type-1
    xref entry; the old container is left byte-untouched. Rewriting
    the container would perturb every *other* object inside it — a
    minimal-diff violation by proxy. Every promotion is a counted,
    named diagnostic. Note the interaction with R35: the stale
    compressed copy is exactly why redaction cannot use incremental
    save.
  - **CORRECTION (2026-07-31, Pass 3.1) to R35's rationale and R38's
    note — the stale-copy path is NOT closed by a full rewrite.**
    Decision 007 W3's mitigation and `ARCHITECTURE.md` §5.2's original
    framing implied that forcing a full rewrite purges a promoted
    compressed object's superseded value. **FALSE**: object streams
    carry through **verbatim in BOTH save modes** (§5.6 — `save_full`
    re-emits containers intact, zero promotions), so a promoted
    object's old value survives inside its untouched container in the
    full rewrite too. Consequence, binding on the Redaction Pass: **it
    must rewrite/decompose every container stream that holds a
    redacted object** — refusing incremental save (R35) is necessary
    but NOT sufficient. Documented at the creating code; full
    amendment `ARCHITECTURE.md` §5.7; the archived 007 record is not
    edited (corrections are recorded forward). Coverage honesty,
    same date: R38 promotion is **fixture-covered, not
    corpus-covered** — 75 corpus files hold 2,197 compressed objects
    but page objects are uncompressed in all of them (rotation never
    promotes on the corpus); the round-trip harness reports both
    numbers so the gap stays visible.
  - **R39 — `/ID` discipline.** `/ID[0]` is preserved for the life of
    the file; `/ID[1]` is regenerated on every save (§14.4). Tested
    explicitly. It is also an input to §7.6 key derivation, so an
    error here surfaces in Pass 5 as a decryption failure that looks
    like a crypto bug.
  - **R40 — Writer changes carry fuzz and differential coverage.**
    Every writer Pass extends the `parse → write → parse → compare`
    fuzz target and the `tools/difftest` oracle. `qpdf --check` is
    used as an external structural validator (Apache-2.0; PRIOR_ART
    line 227 clears direct reuse including test material, with
    attribution).
  - **R41 — No output fingerprint, enforced at the writer.** Restates
    decision 001 §6.1 obligation 6 at its actual enforcement point:
    incremental save never rewrites `/Info` absent a real operator
    metadata change; full rewrite's `/Producer` is documented and
    overridable from both front ends; no build hash, edition marker,
    or non-suppressible producer id is ever emitted.
- **R42 — Linearization is never repaired (rule renumbered
  2026-07-31; librarian reconciliation note below).** The
  `ARCHITECTURE.md` §5.4 rule in full: pdfce **detects** Annex F
  linearization on load (F.3.3 parameter dictionary,
  `L`-versus-file-length liveness check); **warns** before a save
  that would spend a live Fast Web View property; **never strips** a
  stale `/Linearized` dictionary (that would be a normalization —
  R33 — and Annex G.7's reader-side revalidation depends on its
  presence); **never patches `L`** (a "fixed" `L` claims
  linearization while the hints point into a stale layout — strictly
  worse than an honestly de-linearized file). Re-linearization
  belongs to the Optimization backlog bucket, never to any save
  path.
  *Reconciliation note (2026-07-31, librarian, Pass 3.2 filing —
  resolving the RECORD DEFECT flagged in
  `docs/ui_specs/pass-3.2-page-ops.md`'s header):* two different
  rules had come to share the number R36. Decision 007's R36 —
  "save mode is chosen by contract and disclosed" (the
  signature-forces-incremental / full-rewrite-names-what-it-destroys
  either-or, plus the warn-on-save clause) — is the meaning used by
  `writer/mod.rs`, `document.rs`, `linearization.rs` and
  `save.rs` comments, and it KEEPS R36 (code comments stay as-is;
  the warn-on-save citations are consistent with 007's R36 text).
  `ARCHITECTURE.md` §5.4 had additionally cited "R36" for the
  DISTINCT never-repaired/never-strip/never-patch-`L` rule above,
  which decision 007 never numbered separately. That rule is now
  **R42** (first free number after R41, verified against this list),
  and §5.4's citation line is corrected to cite it. No rule content
  changed on either side — numbering only.
- **Annotation & markup discipline R43–R52 (decision 008, 2026-08-01).**
  Binding, per `docs/decisions/008-next-subsystem-after-extract.md`
  (the full-text authority if any condensation below is ambiguous):
  - **R43 — Render from `/AP`, or not at all.** An annotation/widget is
    painted from its appearance stream (`/AP` `/N`, sub-selected by
    `/AS` where present) or it is not painted — pdfce never synthesizes
    a look for an annotation that has no appearance stream during
    normal display. This is the display sibling of R29
    (render-what-the-file-says, never invent).
  - **R44 — A generated appearance written to the file is never
    rendered from a private buffer.** When pdfce AUTHORS an appearance
    (6.1/6.2 markup, 6.2 variable text, form-field appearances), the
    generated stream is written into the document's `/AP` and then
    displayed by the SAME R43 path that displays any other annotation —
    there is no second, private "what I just drew" render path that
    could diverge from the bytes on disk.
  - **R45 — Authored stream bytes live in a session staging buffer;
    `Stream` keeps its span model.** Content pdfce authors is
    accumulated in a per-session staging buffer (the pageops/assemble
    pattern, finding F4), not by mutating the span-provenanced `Stream`
    type into a bytes-owning type. The `DocumentView` assertion "a Pass
    that authors stream bytes must revisit this type" (F4) is
    discharged by AMENDING the type deliberately in Pass 6.1 — a named,
    reviewed change, not a silent widening.
  - **R46 — The content-stream serializer is proven by a corpus
    identity gate before it authors anything.** Before Pass 6.1 trusts
    its new content-stream writer to author markup, that writer must
    re-serialize existing content streams byte-faithfully across the
    corpus (the R32/R34 discipline extended to content streams). A
    serializer that cannot reproduce is not allowed to author.
  - **R47 — An annotation edit never touches the page content stream.**
    Annotations live in the page `/Annots` array and their own
    appearance streams, NOT in the page's content stream. Creating,
    editing, or deleting an annotation must leave the page content
    stream byte-untouched (minimal-diff, §5). The one operation that
    DOES rewrite page content is redaction (Pass 8), which is the
    deliberate exception, not the rule.
  - **R48 — Flatten is destructive and discloses its
    incremental-save-recoverability.** Flattening an annotation/form
    field into page content is a destructive operation (the live
    annotation is gone); like redaction it must disclose that an
    incremental save leaves the pre-flatten annotation recoverable in
    the prior revision, and offer/force a full rewrite when the intent
    is true removal. Sibling of R35.
  - **R49 — A widget is an annotation first.** Form-field widgets
    (`/Subtype /Widget`) are annotations and share ONE appearance
    pipeline with every other annotation — Pass 6.0 displays them,
    Pass 6.2 generates their appearances, Pass 7 wires them to the
    field model. There is never a second widget-only rendering or
    appearance path.
  - **R50 — Hidden annotations are honored AND counted.** The §12.5.3
    flag set (`/Hidden`, `/NoView`, `/Print`, etc.) is obeyed for
    display, but every annotation is COUNTED whether or not it is
    painted — a hidden annotation is a forensic fact an operator may
    need (it is content in the file). This is the direct fix for
    finding F1 (annotations were neither rendered nor counted).
  - **R51 — `/NeedAppearances` is disclosed, never silently
    auto-generated.** When a form declares `/NeedAppearances true`
    (asking the reader to build field appearances), pdfce discloses the
    state and generates appearances only as a reviewable, operator-
    visible action — never a silent auto-apply on open (fuzzy, never
    sneaky).
  - **R52 — Redaction mark and redaction apply are separate operations
    with separate confirmations.** Marking content for redaction
    (authoring `/Redact` annotations, Pass 6.1 primitives) and applying
    the redaction (destroying the covered content, Pass 8) are two
    distinct operator actions, each with its own confirmation. A mark
    is reversible and non-destructive; an apply is neither. They are
    never fused into one click.
- **Embedded-JavaScript posture R53–R57 (decision 009, 2026-07-31).**
  Binding, per `docs/decisions/009-forms-javascript-posture.md` (the
  full-text authority if any condensation below is ambiguous; its JSON
  Appendix A is the effective machine form). These are decision 009's
  R-JS-1…R-JS-5, renumbered to the next-free slot after R52.
  **Foundational finding:** ISO 32000-1 §12.6.4.16 is a **"hollow
  shall"** — it says a processor *shall execute* a JavaScript script but
  defines NO JS semantics/API/DOM/security model (deferring entirely to
  two external, non-ISO documents). It specifies only the CARRIER
  (Table 217: `S=/JavaScript`, `JS=<string|stream>`) and the HOOK POINTS
  (§12.6.3 triggers; field/doc `/AA`; `/CO`; `/Names /JavaScript`). There
  is therefore NO normative JS behavior to conform to, so **non-execution
  forfeits nothing an ISO conformance claim depends on** — cite at the
  non-execution site.
  - **R53 (was R-JS-1) — pdfce never executes embedded PDF JavaScript.**
    Not field `/AA`, not document `/AA`, not `/OpenAction`, not
    `/Names /JavaScript`, not a recognized built-in, not a custom script.
    There is no JS interpreter in pdfce, and adding one (posture C — a
    sandboxed JS engine) is **prohibited scope, not deferred scope**
    (re-imports the exact attack surface Adobe's broker process contains;
    the hook points can reference `/URI` // `/SubmitForm` // `/ImportData`
    // `/Launch`, which R12/R13 forbid; and there is nothing to conform
    to).
  - **R54 (was R-JS-2) — no trigger event ever fires**, on load or on any
    interaction. The direct semantic sibling of R51, enforced by **R12**
    (no network) + **R13** (no process launch) because trigger actions
    can reference `/URI` // `/SubmitForm` // `/ImportData` // `/Launch`.
    Recognition is pure data modeling — there is no JS action dispatcher
    in pdfce and none is added.
  - **R55 (was R-JS-3) — JS carriers are byte-preserved; never stripped,
    never silently baked.** All `/JS` strings/streams, all `/AA` dicts,
    the `/CO` calculation-order array, the `/Names /JavaScript` name tree,
    and `/OpenAction` round-trip byte-identical (untouched under
    incremental save; verbatim under full rewrite). pdfce never strips a
    script (removal silently changes document semantics), and never
    executes-and-bakes a value as a load/save side effect. A recomputed
    value is only ever an explicit, reviewable, undoable `EditSession`
    edit that leaves the source script in place.
  - **R56 (was R-JS-4) — recognize + disclose; recompute is opt-in,
    whitelisted, fuzzy-never-sneaky.** JS-driven fields are recognized,
    classified, counted, disclosed (posture A — Pass 7's entire JS
    scope). Native recompute (posture B, deferred Pass 7.x) is limited to
    an exact-match built-in whitelist (`AFSimple_Calculate` SUM/AVG/PRD/
    MIN/MAX changes `/V`; the `AF*_Format` family changes display only,
    never `/V` — the two code paths must never merge), is **OFF by default
    per document**, and every recomputed value is a reviewable hint the
    operator accepts or overrides (rule 4) — never silent, never
    authoritative, always leaving the source script in place as the
    downstream authority. The matcher errs hard toward `Custom`
    (false-negative = safe disclosed stale value; false-positive = unsafe
    wrong bake).
  - **R57 (was R-JS-5) — a recompute changing `/V` is DocMDP/FieldMDP-
    gated.** Subject to the existing certification gate (`signature.rs` /
    `SignatureImpact`): `/P >= 2` permits, `/P 1` refuses by name, and a
    recompute that would alter a `/FieldMDP`-locked field is refused by
    name (never silently applied, never silently skipped).
- **R58 — Every removal/scrub operation rides R35's forced FULL REWRITE
  (generalizes R35 beyond redaction-apply).** Added 2026-08-01 on Pass
  8.0's ship (the `pdfce-ui-specialist` finding). R35 established that
  redaction-apply must force a full rewrite and refuse incremental save,
  because an incremental save leaves the "removed" content recoverable in
  the prior revision. R58 generalizes the same reasoning to **any**
  operation whose contract is removal or scrubbing of content —
  **including any future Sanitize / Remove-Hidden-Information / metadata-
  scrub Pass**: it must ride the forced full rewrite (and, per §5.7, must
  decompose every object-stream container holding a scrubbed object), and
  owes an absence test that greps the whole saved output for the removed
  bytes → zero. An incremental scrub is a contradiction in terms. See
  `ARCHITECTURE.md` §5.9 (which generalizes §5.2's R35), the Pass 8.0
  Shipped entry, and the ui-spec that surfaced it.
  **Staleness flagged, text NOT changed (decision 022 §5.4, filed
  continuation 80, 2026-08-04) — see Open operator question (v),
  above.** This rule's literal text ("every removal/scrub operation")
  is already contradicted by two shipped operations that correctly stay
  under incremental save: `EditSession::delete_object` (Pass 9c-min,
  content-stream surgery) and `delete_redaction_mark` (Pass 8). Both are
  removals whose contract is NOT confidentiality (§5.11/R70 already
  established this distinction for in-place text editing, the same
  reasoning applies here) — deleting page content or an annotation says
  "this is no longer in the current revision," not "this must be
  provably unrecoverable," and undo/version history remaining reachable
  is that working as intended, not a defect. Decision 022's own proposed
  `delete_annotation` (Pass 22.0) would be a **third** such exception if
  built without a wording fix. The correction this rule's text needs —
  narrowing "every removal/scrub operation" to "every operation whose
  contract is CONFIDENTIALITY (redaction, scrub, recovered-base save)" —
  is deliberately NOT made here; decision 022 explicitly declines to
  narrow a standing rule's scope solo, and this librarian is following
  that same restraint rather than unilaterally rewriting binding rule
  text. Full reasoning: `ARCHITECTURE.md` §5.9's new staleness note and
  §5.12.
- **R59 — Render-fidelity is proved against an INDEPENDENT renderer at
  corpus scale before any subsystem edits content it re-renders.** Added
  2026-08-01 (decision 010). A self-comparison (pdfce-before vs
  pdfce-after, e.g. Pass 3.0's round-trip oracle) proves
  agreement-with-self, NOT correctness — only a differential against an
  independent reference (pdfium via pypdfium2) proves the pixels are
  right. The harness re-runs on **every render-touching Pass** (the
  R34/R46 re-run pattern), so fidelity cannot silently regress. Any
  residual is **enumerated by file + reason** (R20), NEVER a single
  threshold tuned until the corpus passes — the tolerance band is
  argued and its distribution reported. This is decision 010's
  candidate-C rule; Pass 11 is its first discharge. Cross-references the
  W14 warning. See the Pass 11 In-progress entry and `ARCHITECTURE.md`
  §12 (decision-010 entry).
- **R60 — Exactly ONE canvas-interaction substrate.** Added 2026-08-01
  (decision 010; R49's one-pipeline principle applied to interaction).
  There is exactly one focusable-canvas / screen↔page-transform /
  tool-mode-dispatch / hit-test / selection / live-preview-overlay
  substrate in `pdfce-gui`. **Markup drawing, form-fill, redaction-
  marking, and vector editing all LAYER on it** — a second parallel
  canvas-interaction path is FORBIDDEN. This is why the three accumulated
  GUI follow-up slices (Pass-6.1 markup / Pass-7 form-fill / Pass-8
  redaction-marking) are reconciled as slices of one Pass 12, not three
  independent buckets. See the Pass 12 Next-up entry.
- **R61 — Inkscape is a BEHAVIORAL reference only.** Added 2026-08-01
  (decision 010; formalizes the prior binding ROADMAP note). Inkscape is
  **GPL-2.0-or-later** — it is NEVER a dependency, a code source, or a
  GUI-mimicry target. `pdfce-inkscape-librarian` catalogs its
  capability / behavior / limits ONLY (never its GUI mechanics — same
  discipline as the Acrobat Features RAG per rule 12); `pdfce-ui-specialist`
  designs pdfce's vector-editing UI INDEPENDENTLY. The
  `Inkscape_Features` RAG at `D:\Dev\Rag-Specialized\Inkscape_Features\`
  is a private development-reference corpus, never shipped with pdfce and
  never committed to its repository. See the Pass 9 Next-up entry.
- **Operator-supplied-font discipline R62–R66 (decision 012, 2026-07-31;
  librarian-assigned numbers).** Binding, per
  `docs/decisions/012-operator-supplied-fonts.md` §5.2 + Appendix A (the
  full-text authority if any condensation below is ambiguous).
  *Librarian reconciliation note (2026-07-31): decision 012's record
  proposes these five as "R61–R65", but **R61 was already taken** by the
  decision-010 Inkscape-behavioral-reference rule above. They are therefore
  filed at the next five free numbers, **R62–R66** (mapping: record-R61→R62,
  R62→R63, R63→R64, R64→R65, R65→R66). No rule content changed — numbering
  only. **OWED CODE FOLLOW-UP (next engineer):** the operator-supplied-fonts
  implementation in `pdfce-render` carries in-code rule-number doc comments
  written as R61/R62/R63 (the record's proposed numbers); those comments must
  be updated to the assigned R62/R63/R64 to match this list. Librarian does
  not edit code — recorded here and in SESSION_LOG as an owed item.*
  - **R62 — Supplied faces are shell-sourced, never renderer-discovered.**
    The folder walk (and any future OS enumeration) lives in
    `pdfce-gui`/`pdfce-cli`; `pdfce-render` only ever *receives* bytes
    through the `FontEnvironment` seam. No filesystem access and no
    `cfg(target_os)` enters `pdfce-core`/`pdfce-render` under this feature —
    so R10 (platform-clean core/render), R11 (wasm32 clean), and R19
    (deterministic-by-default renderer) all hold.
  - **R63 — Three glyph trust levels, always disclosed distinctly.**
    Embedded (the document's own program, exact) / Bundled (a Foxit Base-14
    face, plausible) / Supplied (an operator-supplied face, the operator's
    own shapes). Counted and surfaced separately in `Diagnostics`, the GUI
    panel, and the CLI summary; a supplied glyph is never presented as
    embedded, a bundled glyph never as supplied. Refines the R20
    diagnostics contract from two levels to three. Positions still come from
    the PDF's own `/Widths` (decision 004 §3.6) — a supplied face improves
    *shapes*, not *positions*, and the disclosure copy must say so.
  - **R64 — Supplied faces are outside the determinism guarantee and the
    R59 gate.** The R59 render-fidelity harness always constructs a
    bundled-only `FontEnvironment` and ignores any ambient font-dir config;
    supplied-font renders are machine-dependent by definition, and the UI
    discloses when supplied fonts are active. R19's "same input → same
    pixels" is scoped to the bundled set.
  - **R65 — Composite (CID) substitution, if ever added, is Unicode-route
    only.** `CID → (ToUnicode ▸ predefined CMap) → Unicode → supplied-face
    cmap → GID`, disclosed lossy, SKIPPING (never GID-guessing) when no
    Unicode mapping exists. `Identity-H` without `ToUnicode` stays a hard
    skip permanently. GID-guessing (option C3) is the "sneaky" failure mode
    rule 4 forbids and is rejected outright. This is a named non-goal for
    the first cut (fast-follow FF2).
  - **R66 — OS-font access is explicit opt-in, never default, never a
    build-time dependency.** The portable build ships zero external font
    dependencies; the bundled Foxit 14 are the deterministic floor with or
    without OS access (fast-follow FF1). Preserves decision 003's D2/R10
    single-folder-portable posture.
- **R67 — A cross-reference-recovered document forces a full-rewrite save
  (decision 013, 2026-07-31; librarian-assigned number).** Binding, per
  `docs/decisions/013-xref-recovery.md` §9. *Librarian reconciliation note:
  decision 013's record proposes this as "R59", which is **already taken**
  (decision-010 render-fidelity gate); it is filed at the next free number
  after decision 012's block, **R67**. **OWED CODE FOLLOW-UP:** any Pass-B
  (rebuild-by-scan) code doc comments that cite the recovered-base rule as
  "R59" must be updated to R67 when Pass 13b lands — recorded as an owed
  item.* **(2026-08-01 footer: Pass 13b SHIPPED — see Shipped above. The
  code comments in `recover.rs` still say "~R62/R59"; the engineer is
  discharging this in-session. R67 is now IN FORCE, not merely filed.)**
  A document loaded via cross-reference recovery had an **invalid
  base xref** and cannot be incrementally appended to (an incremental
  append would write a section whose `/Prev` points at a cross-reference
  section that does not correctly exist). Its save is therefore a
  **mandatory full rewrite** emitting a fresh valid classic xref; the
  recovered/rebuilt status is flagged on the `Document`, disclosed in
  CLI + GUI, and counted (R20). `save_incremental` on a recovered document
  is **refused by name** (`WriteError::RecoveredBaseForbidsIncremental`).
  Sibling of R35 (redaction) and R58 (removal/scrub) — the third member of
  the forced-full-rewrite family. The §5.6 "never normalize" rule does NOT
  bind a recovered file, because its base was already invalid, so emitting a
  fresh normalized classic xref is the correct, honest output. Recovery
  triggers exclusively on the strict-load error path, so the round-trip/
  minimal-diff invariant for cleanly-loading files is preserved by
  construction. See `ARCHITECTURE.md` §5.10 (pending Pass-13b ship).
- **R68 — Embedded font programs route to the correct parser or fail clean;
  a magic/variant disagreement is a gate failure (font-parity harness,
  2026-07-31; librarian-assigned number).** Added on the NUL-misroute font
  fix (see Shipped). Every embedded font program is dispatched to the parser
  matching its *actual* binary format (sfnt `0x00010000`/`OTTO`/`true`/`ttcf`
  → the TrueType/CFF-in-OpenType path via the one skrifa parser, R21; bare
  CFF → the CFF path; `%!`-prefixed Type 1 → the Type 1 path) or it fails
  clean with a named diagnostic — **never silently handed to the wrong
  parser**. The root-cause bug this rule guards: format detection trimmed
  leading whitespace *including NUL* before magic-sniffing, stripping the
  leading NUL of `0x00010000` so the remaining `01 00 …` matched bare-CFF
  magic and TrueType bytes were handed to the CFF parser (surfacing as an
  "offset out of bounds" that *looked* like a read-fonts objection but was a
  caller-side misroute). The `tools/font-parity/` harness parses every
  embedded font in the corpus and asserts routing-or-clean-fail; **a
  magic/variant disagreement is a gate failure**, and the harness re-runs on
  any `font/program.rs` or font-layer change (the R46/R59 re-run pattern).
- **R69 — Text edit is surgery, never overlay (decision 014, 2026-07-31;
  librarian-assigned number).** In-place text edits rewrite the page content
  stream via advance-preserving surgery (kin to Pass 8.0 redaction), NOT the
  Pass-6.x overlay-append path (sibling to R47). Only the edited content
  stream(s) (+ any changed resource/font dict) are re-emitted; everything
  else stays byte-verbatim.
- **R70 — Text edit is incremental, not a scrub (decision 014, 2026-07-31;
  librarian-assigned number).** Editing uses the DEFAULT incremental save
  (R36); prior text survives in history by design and this is disclosed to
  the operator. Truly removing text is REDACTION (Pass 8, R35) — never
  conflated. Distinguishes text editing from R58's forced-full-rewrite scrub
  family: editing is a content CHANGE, not a removal.
- **R71 — Font-on-edit trust ladder (decision 014, 2026-07-31;
  librarian-assigned number).** A keystroke is applied only when the run's
  font can already provide the glyph: an embedded program's existing glyphs,
  or a non-embedded font's full bundled/supplied coverage (decision 012). A
  glyph an embedded SUBSET lacks is REFUSED with a named disclosure — never
  faked, never silently substituted. ~~Font-subsetting/glyph-embedding (FF-C)
  is a deferred writer subsystem, permissive-only (rule 13).~~
  **AMENDED 2026-08-03 (decision 021) — FF-C is no longer "a deferred
  writer subsystem": it is DECIDED and SCOPED as ★ Pass 21.x (Next up).**
  The trust ladder gains a fourth rung: **refuse → offer embed (R108,
  an explicit per-action operator choice with the real computed subset
  size/coverage shown) → embed on accept.** FF-C never widens the
  document's OWN embedded subset (R107 — it only ever adds a new font
  resource from a donor face), so the original refusal's wording is
  correct as far as it goes; what changes is that the refusal now
  carries a remedy pointer where a donor face is available.
- **R72 — Recognized blocks and reflow are reviewable hints (decision 014,
  2026-07-31; librarian-assigned number).** Block recognition and reflow are
  DERIVED (ISO 32000-1 §14.8 S1-S9), counted, and presented as a reviewable
  structure the operator accepts/corrects — never a silent re-layout (rule
  4, fuzzy-never-sneaky).
- **R73 — Tagged edits disclose, never corrupt (decision 014, 2026-07-31;
  librarian-assigned number).** An edit inside a marked-content sequence
  preserves its BDC/EMC + MCID wrapper (the structure tree's references
  stay valid) and discloses that `/ActualText` and reading-order were not
  updated. pdfce never silently corrupts the accessibility tree — a
  documented Acrobat-beating property (Acrobat's own in-place edit is known
  to corrupt the structure tree).
- **R74 — Text model/edit/reflow in core; edit UI in gui (decision 014,
  2026-07-31; librarian-assigned number).** The text model, hit-test,
  caret/selection, surgery, and reflow live in `pdfce-core` (no GUI dep,
  verified by `cargo tree`); the canvas interaction lives in `pdfce-gui` on
  the R60 canvas; the CLI gets a scriptable `edit-text` subcommand.
- **R75 — Reflow is an explicit, reviewable, single-block, one-undo-command
  operation (decision 015, 2026-08-01; librarian-assigned number).**
  Within-block re-wrap is never automatic on edit; it is an operator-invoked
  action producing a DERIVED `ReflowPreview` accepted/rejected before any
  mutation, scoped to exactly one recognized `Block` (never crossing a
  sibling block or column band), applied as ONE undo-able
  `CommandKind::ReflowBlock`. Pass 14.1's single-line relayout + overflow
  disclosure remains the DEFAULT post-edit behavior; reflow (Pass 15.x) is an
  opt-in beside it, never a replacement.
- **R76 — Reflow overflow discloses, never disappears (decision 015,
  2026-08-01; librarian-assigned number).** A re-wrap that grows a block past
  the page cropbox never silently clips-to-invisible or drops content
  (Acrobat's own documentation says overflow "disappears" — an anti-pattern
  rule 4 forbids reproducing). The overflow is a disclosed, reviewable
  condition, and any accepted off-page content is emitted as real, recoverable
  content at its true position — never clipped-to-deleted. A hard refuse is
  also rejected: the content genuinely exists and must not be lost.
- **R77 — Alignment is auto-detected and preserved through re-wrap (decision
  015, 2026-08-01; librarian-assigned number).** A block's original alignment
  (left/center/right/justified) is inferred from its glyph x-positions via the
  Pass 14.0 x-band geometry and preserved by default through a reflow; the
  inference is counted (R20 diagnostics) and operator-overridable; a
  single-line block defaults to left with a disclosed ambiguity note. Acrobat
  has no documented alignment auto-detect/preserve — a re-wrap there risks a
  silent left-align; this rule is a named exceed-Acrobat property, not an
  incidental side effect. (Kept distinct from R75 rather than folded per
  decision 015 §5's discretion note — see the ★ Pass 15.x rationale above.)
- **R78 — Add-new-text is page-content surgery, never a FreeText annotation
  (decision 016, 2026-08-01; librarian-assigned number).** The operation
  synthesizes a new `BT…ET` text object appended to the page `/Contents`
  (original streams byte-verbatim), routed into the same 14.x model/edit/
  format + 15.x reflow pipeline as existing page text, applied as ONE
  undo-able `CommandKind::AddText`; it is NEVER implemented as, or conflated
  with, the Pass-6.2 FreeText annotation path (distinct removal/flatten/
  permission semantics — a real, sourced Acrobat naming collision the catalog
  documents between Edit-PDF "Add Text" and Fill&Sign's typewriter-descended
  `/FreeText`). Sibling of R69 (text edit is surgery, not overlay) for the
  add-new-content case.
- **R79 — New text uses a bundled/supplied face by name+code, no embedding
  BY DEFAULT, with disclosed provenance (decision 016, 2026-08-01;
  librarian-assigned number).** A newly-added run defaults to a bundled
  Standard-14 permissive face (§9.6.2.2 — no embedding, sidestepping the
  FF-C embedded-subset wall), operator-configurable via decision 012's
  `GlyphSource`; a glyph the chosen face lacks is refused-and-disclosed
  (R71), never faked; the run's font source is disclosed
  (`Bundled`/`Supplied`), never presented as the document's own. This is
  why FF-D needs no FF-C to ship. **AMENDED 2026-08-03 (decision 021) —
  "no embedding" corrected to "no embedding BY DEFAULT."** FF-C
  (★ Pass 21.x) makes embedding possible as an explicit, per-action
  operator choice (R108) offered at the point of refusal — it is never
  a default and never silently upgrades this rule's no-embed baseline;
  R79's own default behavior is unchanged by FF-C's existence.
- **R80 — The right-hand dock is a two-compartment, independently-
  selecting panel host (decision 017, 2026-08-02; librarian-assigned
  number).** Every dockable surface is a `DockPanel` variant reached
  through ONE `panel_body` dispatcher. No panel is reachable ONLY as a
  floating window.
- **R81 — Floating windows are for TRANSIENT surfaces only (decision
  017, 2026-08-02; librarian-assigned number).** Confirmations,
  blocking questions, and modeless references may float; anything an
  operator keeps open while working on the document is a dock panel.
  **Supersedes** `ARCHITECTURE.md` §12 continuation-19's "Properties
  stays the single legacy floating exception, never to be joined by a
  second" — see the dated §12 entry filed 2026-08-02 for the full
  correction (Dimension Groups, Pass 12.M2, already breached that rule
  before this one existed; named there as the remaining floating-window
  holdout for a follow-up migration into the dock).
- **R82 — Panel layout is user state, and user state rides R15 (decision
  017, 2026-08-02; librarian-assigned number).** Never persisted through
  eframe's platform-directory Storage; session-only, explicitly disclosed
  until a Pass lands R15's own persistence scheme (fail-soft: file
  missing/parse failure/unknown variant/mandatory panel absent →
  default layout, disclosed, never an error dialog and never a lost
  document session).
- **R83 — No affordance without the capability (decision 017, 2026-08-02;
  librarian-assigned number).** No drag cursor, drag-highlight, or
  resize handle for an interaction that is not implemented.
- **R84 — Selected state is never colour alone (decision 017, 2026-08-02;
  librarian-assigned number).** Pair it with a weight or glyph cue —
  the GUI-polish audit already flagged colour-fill-only selected state
  as a recurring blind spot (Fit Page/Width, the Properties toggle, the
  annotations toggle); new selection surfaces (tab strips, tree rows,
  canvas selection outlines) must not repeat it.
- **R85 — Preview-equals-saved (decision 018, 2026-08-02; librarian-
  assigned number).** For every editing operation, the raster of the
  edited session view must be pixel-identical to the raster of the
  saved-then-reloaded document — what the operator sees before saving is
  what they get after saving. Headlessly checkable, reusing the Pass 11
  raster oracle; covers `add-text`, `annotate`, `dimension-add`,
  `object-move`, `object-delete`, `node-move`, `edit-text`, `format-text`,
  `reflow`, `flatten`, `fill-field`, `redact-apply`. Inverts cleanly
  against R46 (proves presence) and §5.9's absence test (proves
  deletion); its absence is exactly what let the ★★★★ HEADLINE FINDING
  survive fourteen editing Passes unnoticed — every one of them proved
  *saved* output correct, none proved *displayed* output correct.
  **AMENDMENT (2026-08-03, Pass 17.2, continuation 58) — `redact-apply`
  is STRUCTURALLY uncoverable by this rule, not merely unimplemented.**
  The oracle built to satisfy this rule (`tools/` harness, Pass 17.2)
  covers the other 11 operations listed above; `redact-apply` consumes
  a `Document` and emits a file directly rather than operating on a
  live `EditSession`, so "preview equals saved" has no live-session
  left-hand side to compare against for that one operation. The
  operation listing above is left as originally written (append-only
  discipline) with this amendment recording the gap rather than
  editing the list silently — see the Pass 17.1/17.2 Shipped entry and
  `ARCHITECTURE.md` §12's continuation-58 decision-018 entry for the
  full account, and the "GUI has no redaction-apply flow at all"
  Backlog entry for the resulting product-gap consequence.
- **R86 — A Pass does not ship until observed working in the running
  application (decision 018, 2026-08-02; librarian-assigned number;
  OPERATOR SIGN-OFF PENDING — see Open operator questions above, item
  (e)).** A Pass that adds or changes operator-facing behavior is not
  "done" on headless-test-green alone; the behavior must have been
  observed working in the actual running GUI/CLI before the Pass is
  recorded as Shipped. Every Pass 3.1–16.2 met its stated gates and
  shipped a feature the operator could not see — a GATE defect, not an
  engineering defect, per decision 018 §11. **This rule is proposed,
  not yet in force** — do not cite it as binding until the operator
  answers item (e); it is recorded here now so the number is reserved
  and the text is ready the moment it is confirmed.
  **Scope note queued for activation (librarian-assigned, 2026-08-04,
  continuation 77 — not yet in force, same PENDING status as the rule
  itself):** "observed working in the running application" is not
  limited to success paths. A REFUSAL is operator-facing behavior too —
  the operator sees a message, or doesn't, exactly as much as they see
  a feature working — so once confirmed, R86 also requires observing a
  refusal actually fire in the running GUI/CLI before a Pass that adds
  or changes one is recorded as Shipped, not merely that the refusal
  compiles and its unit test passes. This is not a new rule: it is what
  actually caught continuation 76's R-INV-4/R96 finding above — the
  refusal had a green test and a confident comment, and was only proven
  unreachable by trying to make it fire in a real edit and failing
  twice. Filed as a scope clarification to fold in whenever item (e) is
  answered, not as new machinery.
- **R87 — Hashes and commit/test counts handed to a doc-writing agent
  must be engineer-verified against `git`/`cargo test` and spot-checked
  after filing, never filed on trust (methodology; no decision number;
  librarian-assigned; 2026-08-03, continuation 60).** `pdfce-librarian`
  (and every other doc-writing agent) has no shell of its own — any
  figure it's handed is filed verbatim as fact, with no independent way
  to check it. This is the **second** filing error this exact habit has
  caught in this project (the first: `7274fdd`, a hash-repair commit,
  went missing from its own chain listing at continuation 59; the
  second: that same chain listing's own hash/count, corrected at
  continuation 60 — see the commit-chain UPDATE paragraph above). Every
  hash and every commit/test count in this file and in `SESSION_LOG.md`
  is produced by the engineer running `git rev-list`/`git cat-file -t`/
  `cargo test --workspace` directly, not recalled or re-derived from a
  prior summary, and is spot-checked once filed.
  **AMENDED 2026-08-03 (continuation 68) — a structural blind spot in
  the audit itself, not just another instance of the same slip.** A
  continuation's own filing commit has now gone missing from every hash
  reference in `docs/` **twice, and both times the missing commit was
  itself a *filing* commit** (`fb97abb`, the continuation-66 filing,
  found missing at continuation 67; `7274fdd` was the first, above).
  The pattern is structural, not coincidental: a continuation records
  the commits it is filing ABOUT, and the commit that lands the filing
  itself has no later entry to mention it — there is no natural point
  in the process where a filing commit gets to cite itself. The audit
  catches it only because it compares against `git rev-list --count
  HEAD`, not against the previous entry's own listed total. **No new
  numbered rule filed for this** — it is R87's own mechanism (verify
  against `git`, never against memory of what was last written) working
  as designed on its own blind spot; recorded as an amendment so the
  discipline reads as one rule, not two.
- **R88 — Direct text-state formatting is scoped by explicit
  restore-by-value, never by `q`/`Q`, never by normalization (decision
  019, 2026-08-03; wording CORRECTED 2026-08-03 by decision 019
  Amendment A / Pass 19.0 — see the Pass 19.0 Shipped entry and
  `ARCHITECTURE.md` §12 for why a fourth rung was needed).** Any
  `Tc`/`Tw`/`Tz`/`Ts`/`Tr` pdfce emits to affect one run is followed by
  an explicit restore of the resolved ambient value, emitted **inside
  the same text object** — `q`/`Q` are "Special graphics state"
  operators and are **not permitted inside `BT…ET`** (ISO 32000-1 §8.2
  Table 51/Figure 9), and splitting the text object to use them would
  discard `Tm` (§9.4.1). The ambient value is resolved by a **four-rung**
  ladder (corrected from the original three-rung wording): spec default
  when provably unset → **restore from raw operand bytes where they are
  a faithful and side-effect-free record** → **re-spell the value in
  its own dedicated operator where it is known but its source operator
  did more than set it** (`AmbientOrigin::ObservedIndirect` — e.g. `"`
  sets `Tw`/`Tc` while showing a string, `TD` sets `TL` while moving the
  line; replaying either's raw bytes as a spacing-only restore would
  re-execute the side effect) → **refuse-and-disclose when unobservable**
  (Form-XObject-inherited state, unparseable prefix; the multi-stream
  `/Contents` case is currently architecturally unreachable — see
  Amendment A item 2). A guessed default restore is never emitted. New
  text authored inside a balanced `q … BT … ET … Q` envelope (the
  `addtext.rs` path, Pass 16.x) is exempt: `Q` performs the restore.
  **Restore set EXTENDED 2026-08-03 by decision 019 Amendment C / Pass
  19.2 (`ebe35d8`):** synthetic bold's stroking colour and derived
  stroke line width are **ordinary graphics state shared with path
  painting**, not text state — §3.6 originally described them only as
  values to *set* correctly and never named them as values to
  *restore*. Left unrestored, a synthetic-bold run's stroke settings
  leak into every later stroked *path* on the page, not only later
  text. Both are now tracked and restored by the same ladder, alongside
  the six text-state parameters.
- **R89 — Size-relative typographic quantities are stored as ratios and
  derived at emit time (decision 019, 2026-08-03).** `Tc` and `Ts` are
  in *unscaled text space units* and are **not** scaled by `Tfs` (§9.3)
  — a naive implementation that stores an absolute rise/tracking value
  silently mis-scales it on a font-size change (a 10 pt superscript
  resized to 20 pt keeps its absolute rise and lands wrong). pdfce's
  model stores these as a discriminated `Absolute | Relative` quantity;
  superscript/subscript are always `Relative`, and the absolute operand
  is re-derived whenever `Tfs` changes.
- **R90 — Synthetic bold/italic is per-use, declinable, fallback-only,
  and self-evident (decision 019, 2026-08-03).** Offered only when no
  real Bold/Italic resource resolves (the same coverage check that
  gates a real family/style change, Pass 14.2); never a global
  preference — a named, declinable, per-application choice on every use
  (deliberately stricter than Acrobat's set-and-forget "Enable
  Artificial Bold/Italic Styles" preference). Emitted by spec-native
  means only: text rendering mode 2 (`Tr 2`, stroke+fill) with a
  **user-space-derived** stroke width and the stroking colour matched
  to the fill (§9.3.6 — both are real bugs if missed: composite mode
  without a matched stroke colour puts a black outline on coloured
  text; a device-space-constant stroke width ignores
  `Tfs × |Tm| × |CTM|`), and a `Tm` shear for oblique — **never
  double-strike** (doubles glyph count, breaks the byte↔glyph
  correspondence provenance depends on). The result is re-detectable by
  byte inspection on reload — recorded in-session as `StyleSynthesis`
  provenance, **never written into the PDF as a private marker.** One
  shared policy across both the in-place-edit (14.x) and add-text
  (16.x) paths; only the *order* the real-face remedy is offered in
  differs, and that difference is disclosed. A `Tm` shear is **not**
  text state and is **not** covered by R88 — it propagates through
  `Td`/`TD`/`T*` into every later line, and a shear composed with `Ts`
  displaces a raised run by `Trise × tan θ` — both are hazards beyond
  the original brief, not hypothetical edge cases. **AMENDED 2026-08-03
  by decision 019 Amendment C / Pass 19.2 (`ebe35d8`):** the fix for
  the `Tm`-shear-propagation hazard is narrower than originally
  written — pdfce does **not** convert a producer's own `Td`/`T*` into
  an absolute `Tm` (that would rewrite the producer's own line
  structure past minimal-diff, R32/R46, and cascade to every later
  relative move); it instead **requires** the follower already be
  positioned by an absolute `Tm` and **refuses, disclosed, otherwise**
  — verified non-vacuous by a twin test where the same run succeeds
  once the next line opens its own `BT…ET`. This refusal gate needs
  `Tm`/`Tlm` tracking in the authoring walk, which neither the original
  decision nor Amendment A anticipated (Amendment A.3 scoped the shared
  hoist to the six text-state parameters only) — Pass 19.2 added it to
  `text_edit::edit::Walk` (`BT` reset, `Td`/`TD`/`T*` derivation,
  §9.4.4 advance accumulation, a `matrix_known` honesty flag, a new
  `Rec::EndText` variant). The bold-width formula
  (`Tfs × |Tm| × |CTM|`) ships **two of its three factors** — no
  page-level `cm` model exists in the authoring walk, so a stroke
  synthesized inside a scaled `cm` context is not compensated; this is
  a **named, disclosed limit**, not a silent gap. Two conflicts are
  refused by name rather than silently merged: free-form rise vs. the
  superscript/subscript toggle (both write `Ts`), and synthetic italic
  vs. a `--pin` follower-positioning mode (the closing absolute `Tm`
  and `--pin`'s compensating `TJ` adjustment would each consume the
  same positional delta). **Add-Text synthesis is not yet wired**: the
  shared type and `SynthesisPath::AddText` exist and are tested, but
  `addtext.rs` has no bold/italic request surface to reach them from —
  flagged as undelivered, not implied shipped by the type's existence.
- **R91 — `Tw` is capability-gated by font model, and inter-word
  distribution on composite runs is `TJ`-only (decision 019,
  2026-08-03).** Word spacing applies only to single-byte code 32
  (§9.3.3) and is structurally void for 2-byte composite/CID runs.
  pdfce never emits `Tw` on a composite run and never presents a
  word-spacing affordance for one (R83). Slack/inter-word distribution
  for composite runs uses `TJ` numeric adjustments — the decision-015
  path — never `Tw`. Whether `Tw` ships as a direct control AT ALL for
  simple-font runs is gated behind a corpus census with explicit
  decision bands (≥60% of sampled real-world documents → build; ≤25% →
  close the item; 25–60% → escalate to the operator) — see the ★ Pass
  19.x entry (Next up) and Open operator questions. **AMENDED 2026-08-03
  by decision 019 Amendment F (Pass 19.4, `a1638f4`):** the composite-run
  refusal this rule states was implemented but UNREACHABLE until this
  Pass — `match_run` filtered every composite run to `NoMatch` before
  the refusal gate could run, since composite runs are never decoded to
  text. Fixed by hoisting font resolution above `match_run`; see R96 and
  the Pass 19.4 Shipped entry. The rule's substance (word spacing is
  structurally void for composite runs) is unchanged — only the
  implementation's reachability was defective.
- **R92 — A predicate that hand-duplicates the shape of a data
  structure it inspects drifts silently the moment the structure gains
  a field or case (methodology; no decision number; librarian-assigned;
  2026-08-03, decision 019 Amendment B).** An exhaustive field-by-field
  no-op/emptiness check, or a hand-listed operator-arm list mirroring an
  operator set, is a duplication of information the structure itself
  already carries — the moment the structure grows, the duplicate is
  silently stale. **Second occurrence of this exact bug shape in this
  project:** the first was decision 019 Amendment A.4 (`text_edit::
  edit::Walk` had no `q`/`Q` arm at all, a hand-maintained operator-arm
  list that never grew to cover them); the second was Pass 19.1's
  discovery that `EditSession::format_text` hand-listed its own no-op
  predicate (`set_size.is_none() && set_fill.is_none() &&
  set_font.is_none()`), which Pass 19.1's new `FormatRequest` fields
  bypassed entirely — a spacing-only request became a phantom `NoOp` on
  the GUI-facing `EditSession` path specifically, while the CLI path
  (which used the real `FormatRequest` directly) was unaffected. Prefer
  deriving such a predicate from the structure itself (an `is_empty()`
  method on the type, an exhaustive match with no wildcard) over
  hand-maintaining a parallel check — same underlying discipline as
  `D:\dev\rag\rust\non_exhaustive_no_effect_defining_crate_wildcard_free_match.md`
  (keep in-crate matches wildcard-free so a new variant is a compile
  error at every decision point, never a silent `_`-arm fallback).
- **R93 — A code comment asserting a behavior is not evidence the
  behavior holds, even when two independent comments on both ends of a
  contract agree (methodology; no decision number; librarian-assigned;
  2026-08-03, Pass 19.3).** Fourth occurrence of this exact failure shape
  in this project: (1) decision 018's `refresh_pages` doc comment —
  "the document is not reloaded, because the base revision ... has not
  changed" — was true through Pass 3.1 and silently false from Pass 6.1
  onward, and every editing Pass shipped between them relied on it
  without re-checking; (2) the `.gitattributes` ordering incident — the
  file's own `*.pdf binary` rule LOOKED like it protected fixtures, and
  was silently overridden by a catch-all `* text=auto` placed below it;
  (3) Pass 19.3's pinned-span defect (see its Shipped entry, above) —
  `EditRequest::pinned_span`'s "matches the same span" and `page.rs`'s
  "the surgery locates the operator by exactly this span" independently
  asserted the SAME wrong claim, so cross-referencing the two doc
  comments against each other caught nothing; agreement between two
  confident assertions is not corroboration when both are unverified
  against the actual data; (4) `edit.rs`'s comment — *"a composite font
  is not decoded (edit is refused later, R-INV-4)"* — read as correct
  and was wired to a real refusal, but the refusal could never fire:
  `match_run`, the stage immediately before it, silently filtered every
  composite run to `NoMatch` before `classify_font` (R-INV-4's home)
  ever ran, so `edit-text` on a genuinely locatable composite run always
  returned the generic "text not found" refusal, never the R-INV-4
  font-limitation one, on any input, ever (continuation 76, 2026-08-04,
  `8e08e80`+`87d3cb0`+`6b69956` — full record under R96, below, and
  R110's Standing-rules bullet). **The generalizable rule:** a doc comment
  describing a cross-module contract (two conventions match, a cache is
  still valid, a rule fires before a later one) is a claim, not a
  guarantee, and needs the same standard of evidence R86 already
  requires for "done" — observed or tested, not merely asserted and
  left unchallenged because the assertion sounded authoritative. Prefer
  encoding the contract in the type system (see R92's same-shaped
  precedent) or a round-trip test over a prose promise on either side of
  a producer/consumer boundary.
- **R94 — A repair that mutates a value must invalidate any
  "these-bytes-are-verbatim" provenance attached to it (methodology;
  no decision number; librarian-assigned; 2026-08-03, `/Contents`-
  defect fix, committed `409a6b5`).** A writer that copies
  `Provenance::File` objects verbatim as a fast path assumes the
  stored bytes and the object's current value never disagree. A repair
  that fixes the *value* (here, a recovered stream's true extent) but
  leaves that assumption in place produces a file where the emitted
  bytes contradict the corrected value — round-trip-broken by the fix
  itself, self-inflicted. The `Provenance::RecoveredFile` variant
  fixes this instance by giving "value was mutated by a repair"
  its own provenance state, forcing re-serialization instead of
  verbatim copy. General form: any system carrying an
  original-bytes-are-authoritative fast path needs a companion state
  for "we know better than the original bytes now," or a later repair
  silently reintroduces the exact defect it was written to fix.
- **R95 — A dangling reference inside an optional, array-valued page
  entry degrades that one element; it never condemns the whole
  document (decision 013 / R67 family, extended; 2026-08-03,
  `/Contents`-defect fix, committed `409a6b5`).** ISO 32000-1 §7.3.10
  makes an indirect reference to a nonexistent object the null object,
  not an error; Table 30 makes `/Contents` itself optional ("if this
  entry is absent, the page shall be empty"). A single unresolvable
  `/Contents` array element therefore has an exact, spec-defined
  degraded reading (drop that element, keep the rest) — refusing the
  entire document over one bad element is a fail-clean violation
  (`ARCHITECTURE.md` §5.10's "reviewable fact, never a silent repair"
  framing), not conservatism. The degradation is counted
  (`Page.contents_unresolved`) and surfaced (CLI `contents_unresolved=N`,
  GUI "unsupported items" detail list) — never silent. Binding only for
  entries the spec itself marks optional/array-valued with a defined
  null-element reading; a reference resolving to a *wrong type*
  (§7.3.10's other failure mode) is unchanged and still an error.
- **R96 — A guard clause placed after a filter the guarded case cannot
  pass is dead code that looks live (methodology; no decision number;
  librarian-assigned; 2026-08-03, decision 019 Amendment F, Pass
  19.4).** A refusal/validation gate's position in a pipeline matters as
  much as its logic: R91's composite-run refusal was correctly written,
  correctly wired, and structurally unreachable, because the text-decode/
  match stage immediately before it (`match_run`, reading
  `ShowData::text`) silently filtered out every composite run to
  `NoMatch` before the font-aware gate could ever run — composite runs
  are never decoded to text by that walk. Detection requires writing the
  test that asserts the GATE fires (not merely that the happy path or
  even the refusal outcome occurs) — the same "prove it by making it
  fail" discipline R92/R93 and Pass 19.2's mutation testing already
  established, applied here to reachability rather than correctness.
  Before trusting any refusal/validation gate exists, trace every stage
  that runs before it and ask whether the input class the gate is meant
  to catch can even survive to reach it. See
  `D:\dev\rag\rust\dead_guard_clause_behind_a_filter_the_guarded_case_cannot_pass.md`.
  **Second occurrence (continuation 76, 2026-08-04,
  `8e08e80`+`87d3cb0`+`6b69956`), distinct code path, same exact
  shape:** R-INV-4's composite-run refusal in `edit.rs` was correctly
  written and correctly wired, and structurally unreachable, because
  `match_run` — the text-decode/match stage immediately before it — read
  `ShowData::text` and silently filtered every composite run to
  `NoMatch` before the font-aware gate could run; composite runs were
  never decoded far enough to be matched. Found by trying to reach the
  refusal message and failing TWICE — once against an undecodable
  fixture (where `NoMatch` was arguably honest) and again against a
  purpose-built `cidfonttype2-with-tounicode.pdf` whose text was
  genuinely findable, still getting `NoMatch`; the second failure is
  what proved the bug, not reading the code. Fix was ORDERING, not new
  machinery: classify the anchor's font BEFORE `match_run`, since a
  font-level refusal is a property of the run, never of whether the
  sought text sits inside it. **Generalized framing this occurrence
  adds:** a PRECONDITION check (a property of the OBJECT) placed after
  a SEARCH step (a property of the QUERY) only ever fires for objects
  the search can already handle — so the cases the guard exists for are
  exactly the cases that never reach it. The only reliable guard against
  this shape is a test that asserts the ERROR VARIANT the gate is meant
  to produce (not merely that some error occurs), because nothing in the
  type system requires a refusal to be reachable. Filed as the RAG
  file's second occurrence, not a new file — see
  `D:\dev\rag\rust\dead_guard_clause_behind_a_filter_the_guarded_case_cannot_pass.md`.
- **R97 — A security- or correctness-critical proof should be extracted
  to a free function over data, so the proof can be a TEST rather than
  a manual inspection (methodology; no decision number; librarian-
  assigned; 2026-08-03, Pass 8.1, `redact_apply.rs`).** Pass 8.1's
  redaction-apply pipeline was written as a free function taking
  `&EditSession`, deliberately, rather than as inline GUI event-handler
  code — the resulting `applied_redaction_leaves_no_recoverable_trace_
  in_the_saved_bytes` test can drive the EXACT production pipeline
  headlessly and assert absence across the extractor, every decoded
  stream, and the raw bytes, with a negative control. Had the same
  logic lived inline in an `egui` click handler, the same proof would
  have needed either a GUI-driving test harness or a human inspecting
  the code path by eye — a materially weaker guarantee for a
  trust-critical claim. General form: when a change's correctness
  claim is genuinely load-bearing (security, data-loss, round-trip),
  prefer shaping the implementation so the claim CAN be asserted by a
  test over the running production code, even where the natural first
  draft would embed the logic directly in a UI callback.
- **R98 — A confirmation dialog for a destructive, irreversible-in-
  effect operation should compute and disclose the REAL outcome before
  the operator confirms, not a prediction of it, whenever the
  underlying operation is a pure function (methodology; no decision
  number; librarian-assigned; 2026-08-03, Pass 8.1).** Because
  `apply_redactions` is pure (it does not mutate the open session; it
  produces a new file's bytes), Pass 8.1's confirmation flow runs the
  apply BEFORE opening the confirmation modal and shows the operator
  the actual measured report (glyphs removed, annotations removed,
  residuals found) rather than a prediction assembled from the marks
  alone. This is strictly more honest than a prediction, and it is
  what licenses the security proof (R97) to run before the operator
  ever sees the dialog, so a survivor can refuse the whole apply rather
  than be disclosed as a residual after the fact. General form: when a
  confirm-then-act flow's "act" step is pure/side-effect-free relative
  to the object being confirmed, prefer computing the real result first
  and presenting it as the confirmation content, over predicting what
  the result will probably be.
- **R99 — In a bounded dock-pane container, a panel's primary action
  must be reachable before its detail/list content, not after it (UI
  methodology, R86-family finding; librarian-assigned; 2026-08-03, Pass
  8.1, observed via `-ProcessId`).** Pass 8.1's Redact panel originally
  ordered mark-count → scrollable mark list → "Review & Apply
  Redactions…" button; in a default `egui_tiles` tab group (~250 pt
  tall), the list alone pushed the action below the fold. A panel that
  shows the danger (unapplied marks) and hides the remedy (the button
  to finish the job) is an active nudge toward the exact misconception
  the status-bar warning exists to prevent. Fixed by reordering to
  **state → action → detail**: summary/warning first, the primary
  action next, the scrollable/expandable detail list last. General
  form: for any dock panel whose content can grow past its typical
  visible height, order state-then-action-then-detail, not
  content-then-action — R86's "observed working in the running
  application" discipline is what catches this, since headless tests
  have no viewport height to push anything below.
- **R100 — Field identity is the fully-qualified name, and every
  authoring write resolves that name against the object graph before it
  writes (decision 020 §3.1.1/§5, filed as R97 in the decision document
  and renumbered by `pdfce-librarian`; 2026-08-03, Pass 20.x family, not
  yet built).** §12.7.3.2 derives the FQN from the field tree's shape
  rather than storing it, so only the tree can answer what a name
  currently denotes. All field-creation, rename, and widget-attachment
  writes pass through one resolver (`resolve_field_path`); none may
  append to `/AcroForm/Fields` without it. Two same-FQN sibling fields
  are a malformed document pdfce must never author — pdfce's own reader
  already treats that shape as damage to cope with (`fields_named()`'s
  fan-out over every match), not as an intended result.
- **R101 — A widget kid carries no field keys (decision 020 §3.1.3/§5,
  filed as R98 in the decision document; 2026-08-03, Pass 20.x family,
  not yet built).** A `/Kids` entry pdfce authors as a *widget* must not
  contain `/T`, `/FT`, or `/Kids`. `pdfce-core`'s own `kid_is_field`
  heuristic promotes any such kid to a separate terminal field, silently
  destroying the group semantics (radio mutual exclusivity, shared `/V`)
  the widget was created to have. Verified against the shipped parser,
  not inferred.
- **R102 — pdfce never normalizes field shape (decision 020 §3.1.6/§5,
  filed as R99 in the decision document; 2026-08-03, Pass 20.x family,
  not yet built).** Shape A→B promotion occurs **only** when a second
  widget makes the merged form illegal under Table 220; Shape B
  **never** collapses back to A when deletion leaves one widget.
  `ARCHITECTURE.md` §5.6 ("never normalize") applied to a shape it did
  not anticipate when written — cosmetic re-tidying of an object the
  operator did not logically change is a minimal-diff violation
  regardless of how much nicer the result looks.
- **R103 — A guard whose precondition is already refused by a coarser
  earlier gate is not built; the refinement is recorded as owed to the
  Pass that removes the coarser gate (decision 020 §3.6.2/§5, filed as
  R100 in the decision document; 2026-08-03, Pass 20.x family, not yet
  built).** The prospective form of R96. Verified instance: the `/P`
  bit-6 field-creation permission is unreachable while `/Encrypt`
  documents are refused outright by every authoring path, so it is
  filed against Pass 5 (Encryption) rather than written now as a gate
  that could never fire.
- **R104 — `/Tabs` is a mode, not a snapshot (decision 020 §3.4.1/§5,
  filed as R101 in the decision document; 2026-08-03, Pass 20.x family,
  not yet built).** Under `/Tabs /R`, `/C` or `/S`, pdfce reorders
  nothing on field insertion — the order is computed by the consumer
  and there is no stored sequence to maintain. Re-sorting `/Annots` to
  "realize" a computed order would rewrite references pdfce did not
  logically touch **and change annotation paint order**, a visible
  change caused by a non-visible feature. Under an explicit/manual
  order the new widget is appended to the end and that fact is
  disclosed. `/Tabs` is never written as a side effect of field
  creation.
- **R105 — Every field pdfce authors carries `/TU`, or an explicitly
  recorded operator declination (decision 020 §3.5.3/§5, filed as R102
  in the decision document; 2026-08-03, Pass 20.x family, not yet
  built).** For form fields, `/TU` — not the structure tree — is the
  accessible name assistive technology actually reads (WebAIM,
  sourced). It costs one optional string on an object pdfce is creating
  anyway, and its absence is invisible to the sighted operator who
  created the field. Omitting both `--tooltip` and `--no-tooltip` is an
  error, never a silent default.
  **Renumbering note (R100–R105 only):** decision 020 was drafted
  concurrently with continuation 70's Pass 8.1 filing and originally
  proposed these six rules as R97–R102 against a believed ceiling of
  R96. Continuation 70 had already claimed R97–R99 (above) for three
  unrelated Pass 8.1 findings by the time both landed. `pdfce-librarian`
  renumbered all six to R100–R105 in the decision document (prose +
  Appendix A JSON, with a machine-readable mapping added there) and
  here, so the two records agree; no rule's substance changed.
- **R106 — Every one of this project's numbered/lettered ledgers
  (Pass IDs, standing-rule `R`-numbers, decision numbers, Open-operator-
  question letters) has its NEXT free slot determined by reading the
  current ceiling in `ROADMAP.md`/`ARCHITECTURE.md` §12 at assignment
  time, not by guessing, and the assignment is verified — not merely
  intended — before the number is written into a commit message or a
  drafted document (methodology; librarian-assigned; 2026-08-03,
  triggered by the THIRD documented Pass-ID collision on this project
  and, independently, the R97–R105 standing-rule renumbering
  immediately above, both real incidents on the SAME day).** No test,
  lint, `cargo fmt`/`clippy` run, or CI job on this project has any
  concept of these ledgers — `grep -c "^### Pass "`/`"^- \*\*R\d"`
  would catch a literal duplicate heading but not two different Passes
  independently proposing the SAME free-looking number before either
  ships (exactly what happened to the standing-rule renumbering above:
  two authors, working concurrently, each correctly read a ceiling that
  was true when they started and stale by the time they finished).
  Concrete instances on this project, oldest first: decision 014's
  design proposed "Pass 13.x," already shipped by Pass 13a/13b
  (renumbered to 14.x before build); Pass 18.4's shipped commit called
  itself "Pass 18.2," already shipped as the `object-list` CLI Pass
  (corrected in the roadmap entry, commit left as-is); decision 020's
  draft proposed R97–R102 against a stale ceiling, colliding with Pass
  8.1's concurrently-filed R97–R99 (renumbered to R100–R105, immediately
  above); and this Pass's own commit message called itself "Pass 19.4,"
  already shipped as the `Tw` Pass (corrected to Pass 18.7 by a
  dedicated commit, `1111652`, plus this roadmap entry — see the Pass
  18.7 Shipped entry's own Pass-number note, top of Shipped). **Four
  instances, not one — this is a structural gap, not a fluke.**
  Practical mitigation, since no automated gate exists for this: (1)
  the LAST action before writing a Pass number or rule number into a
  commit message, a `docs/decisions/` draft, or a UI spec is re-reading
  the live ceiling in `ROADMAP.md` (not a remembered or session-cached
  one — the concurrent-authorship failure mode above defeats a
  remembered ceiling specifically); (2) when two threads of work may be
  proposing numbers concurrently (a builder committing while a decision
  document is independently drafted, as happened for both the
  standing-rule and this Pass-ID collision), the number is treated as
  PROVISIONAL until `pdfce-librarian` confirms it against `ROADMAP.md`
  at filing time and is the one who assigns the final, canonical
  number — never the number a commit message or draft guessed at
  mid-flight. A cheap mechanical check (a script asserting no two
  `### Pass N` headings or `**R\d+ —` bullets share a number) is
  flagged here as a worthwhile future addition, not built by this
  entry — it would have caught three of these four instances outright,
  though not the standing-rule renumbering, whose collision existed
  only across two DRAFT documents until the moment both were filed.

  **Amendment (2026-08-03, `pdfce-librarian`, mechanical check built —
  `tools/check-ledger-numbers.py`, commit `4dc8cf8`).** The flagged
  future addition above now exists and enforces this rule. It checks
  Pass-ID uniqueness per top-level ROADMAP section (not globally — a
  Pass legitimately appears twice, once as a planning entry and once
  Shipped, and nine such pairs exist today), standing-rule-number
  uniqueness (one allowlisted amendment-shaped exception, R26), and
  decision-file-number uniqueness, and it prints the live ceiling of
  every ledger on both success and failure — the preventive half this
  paragraph asked for, since reading the ceiling is now one command
  instead of a careful read of an 11,000-line file. It was written
  because a FIFTH numbering collision was found the same day this rule
  was drafted: a heading at the old ~line 8153 declared "### Pass 4 —
  Text extraction / structured content" twice, back to back, with
  nothing between the two headings — fixed in the same session that
  added the checker. `--stats` also caught a silent under-parse (5 of
  106 rules, `R53`–`R57`, use a `(was R-JS-1)`-style parenthetical the
  first pattern didn't allow for) before it could produce a false
  "clean". Run `python tools/check-ledger-numbers.py --stats` before
  assigning any new Pass/rule/decision number.

  **Amendment (2026-08-03, second, `pdfce-librarian`/decision 021,
  commit `d30842c`) — the ceiling report itself had the same blind
  spot as the collision it was built to prevent, and was fixed the
  same day it shipped.** Its ceiling report scanned only `### Pass N`
  *headings*. A Pass family is CLAIMED the moment a decision record or
  a Backlog entry names it — well before any heading exists — and this
  fired for real within an hour of the tool shipping: decision 020
  claimed Pass 20.0–20.7 in Backlog *prose* with no heading yet, the
  checker reported "highest Pass family: 19" (true, useless), and a
  concurrently-running FF-C scoping session — reading the same
  heading-only view — independently proposed Pass 20.x for an
  unrelated feature before the engineer caught and corrected it to
  21.x. **The scoping session's error and the checker's were the same
  error: both read only what was visible as a finished/headed record
  and missed what was merely claimed.** Fixed: the checker now scans
  every `Pass N` *mention* in the file (not only headings), reports
  "families with headings" and "families MENTIONED" separately, and
  names the difference explicitly as `CLAIMED BUT NOT YET HEADED —
  already spoken for; do NOT reuse`. Generalizes past this project's
  Pass IDs: **a ceiling computed only from completed/finished records
  under-reports, and does so specifically in the direction that causes
  collisions** — the things most likely to collide with a fresh
  proposal are exactly the other fresh, in-flight, not-yet-finished
  proposals a finished-only view excludes. Escalated to
  `D:\dev\rag\rust\ci_gate_red_at_baseline_enforces_nothing.md`
  (Amendment 2026-08-03 third) as a generalizable finding.

- **R107 — FF-C only ever ADDS font resources; it never modifies an
  existing font program or font dictionary (decision 021, 2026-08-03;
  numbering corrected by the engineer before filing, confirmed by the
  librarian).** Embedding allocates a new `/Type0` dict, CIDFont dict,
  `FontFile*` stream and `/ToUnicode` CMap, and merges one entry into
  the page's `/Font` sub-dict via the `addtext.rs` reference-not-mutate
  path. No `/FontFile`, `/FontFile2`, `/FontFile3`, `/FontDescriptor`,
  or existing `/Font`/CIDFont dictionary is ever rewritten. Keeps §5
  exception-free for the whole family, keeps incremental save the
  default (R70), and keeps FF-C out of the R35/R58/R67
  forced-full-rewrite family. Enforced by an object-id-disjointness
  test (R97 shape), NOT a runtime guard — a guard in an emitter that
  can only allocate fresh ids is unreachable by construction and would
  be exactly R96's dead code that looks live.
- **R108 — Embedding is an explicit, per-action operator choice whose
  real outcome is computed before confirmation (decision 021,
  2026-08-03; librarian-assigned number).** Never a default, never a
  global preference, never a silent upgrade of the R79 no-embed path.
  Offered at the point where the no-embed path would refuse, so the
  refusal becomes an actionable remedy. Because subsetting is pure, the
  confirmation shows the ACTUAL subset byte count and the ACTUAL
  covered/uncovered character list (R98 applied), never an estimate.
- **R109 — Font-embedding permission is read from the donor face and
  disclosed; never assumed, never guessed (decision 021, 2026-08-03;
  librarian-assigned number). AMENDED 2026-08-03 (spec review, decision
  021 §10 C-7): fsType is not one gate — TWO distinct named refusals,
  not one `EmbeddingNotPermitted`.** Read before subsetting (`subsetter`
  strips `OS/2`). **`SubsettingNotPermitted`** — bit 8, `0x0100`, "No
  subsetting": permits embedding the whole face but forbids the one
  thing FF-C ever does. **`EmbeddingNotPermitted`** — bit 9, `0x0200`,
  "Bitmap embedding only," the specification's own "unembeddable" case.
  Absent or unparseable permission data is disclosed as unknown; pdfce
  never silently treats "no data" as "permitted" — and never treats it
  as bit-0 `0x0000` ("Installable") either, since Installable is the
  MOST permissive value, not a safe stand-in for missing data. Bit
  semantics sourced from the OpenType specification via
  `pdfce-spec-librarian`, never from recall (rule 1). The accept/refuse
  policy itself is Open operator question (r), below — narrowed by this
  amendment to the absent/unparseable-`OS/2` case specifically, since
  the bit semantics themselves are no longer open.
  **GAP CLOSED (continuation 76, 2026-08-04, `58fe3f6`) — the read now
  exists in shipped code.** `add-text --embed-font` reads `fsType`
  from the donor's `OS/2` table BEFORE subsetting (`subsetter` strips
  `OS/2`, so this ordering is forced, not stylistic) and enforces the
  two named refusals above. **Three refusals, not two — worth stating
  precisely because the third is a non-refusal:** bit 8
  (`SubsettingNotPermitted`) fires because FF-C always subsets, so a
  face at `0x0108` (embeddable + no-subsetting) is correctly refused
  even though whole-face embedding would be legal — reporting this as
  a blanket "may not be embedded" would misdescribe the font's own
  licence, which is exactly why bit 8 and bit 9 are separate named
  variants rather than one `EmbeddingNotPermitted`. Bit 9
  (`EmbeddingNotPermitted`) is unconditionally unsatisfiable for FF-C,
  since pdfce embeds outlines, never bitmaps. Version-gating enforced:
  bits 8–9 are ignored on `OS/2` v0/v1 — the load-bearing fixture pair
  is `nosubset` (v4) vs. `nosubset-v1` (v1), **byte-identical bit
  patterns, different enforcement**, because a fixture that only varies
  the bits (not the version) cannot prove the version gate is even
  consulted. Seven fixtures total, one per outcome (`SubsettingNotPermitted`,
  `EmbeddingNotPermitted`, `nosubset-v1` no-op, absent `OS/2`, unparseable
  `OS/2`, `fsType==0` Installable proceeds, ordinary permitted proceeds).
  **Two non-refusal cases shipped as an INTERIM DEFAULT, not a
  resolution of Open operator question (r), below:** absent/unparseable
  `OS/2` proceeds (disclosed as unknown, never silently modelled as
  bit-0 `0x0000` Installable — see the trap already named above) and
  `fsType == 4` (Preview & Print) also proceeds, because that value
  permits the embed itself; what it additionally obliges is that the
  *document* stay read-only thereafter, an obligation on every future
  reader that pdfce has no PDF field to express and cannot enforce.
  Both are flagged, not decided — (r) remains open for Ken to pick a
  different policy for either case. Bit semantics re-confirmed sourced
  from `font__opentype_os2_fstype.md` (`pdfce-spec-librarian`), not
  recall (rule 1) — this governs redistributing a third party's font,
  exactly the class rule 1 exists to guard. Until this commit, any
  `add-text --embed-font` output from Pass 21.0 alone was UNVERIFIED
  against the donor's licence; that caveat is now retired for output
  produced from this commit forward.
- **R110 — A composite run is editable only where its `/ToUnicode` is
  VERIFIED injective, per font, per session (decision 021, 2026-08-03;
  librarian-assigned number).** Injectivity — every CID maps to exactly
  one scalar, no two CIDs map to the same scalar — is what makes the
  inverse a function and conditionally lifts R-INV-4. Checked against
  the data, never inferred from the fact that pdfce authored the font
  (R93). Non-injective, absent, or partial `/ToUnicode` keeps
  refusing. `Identity-H` with no `/ToUnicode` remains a permanent hard
  skip — **R65 untouched.**

  **Primitive SHIPPED (continuation 76, 2026-08-04, `c0ed638`):
  `ToUnicodeCMap::injective_inverse()`.** Three disqualifying
  obstructions, each independently checked and named in the refusal:
  a **ligature** (one code maps to a multi-character string, e.g.
  "ffi" — no code means "just the f," so the inverse has no answer for
  that scalar alone); **two codes mapping to one character** (the
  inverse becomes a relation, not a function — pdfce would have to
  CHOOSE which code an edit resolves to, and either choice silently
  changes the glyph; both colliding codes are named in the refusal,
  not just "non-injective"); and an **empty map**. Ranges are
  MATERIALISED for this check (unlike ordinary `/ToUnicode` lookup,
  which resolves lazily) — a collision between a range member and a
  separate `bfchar` single entry is invisible to any point lookup and
  would otherwise go undetected; bounded per range and in total so a
  hostile range cannot turn the check itself into a resource sink.
  Tested against the standard's own §9.10.3 EXAMPLE 2 CMap WITHOUT
  asserting whether it inverts — whether the standard's own example
  happens to be injective is a fact about the example, not about
  pdfce; the test asserts only that the check runs to completion on a
  FOREIGN CMap and reaches a decision with a stated reason, so R110
  cannot silently become a rule that only ever answers "yes" on
  pdfce's own output.

  **Composite-run refusal made REACHABLE (continuation 76, 2026-08-04,
  `8e08e80`+`87d3cb0`+`6b69956`) — the headline finding this
  continuation.** `edit.rs` carried a comment claiming composite fonts
  are refused later, by R-INV-4 — the comment was FALSE: a composite
  run's text is decoded only far enough to attempt `match_run`, and
  because that decode failed to locate the sought text, `classify_font`
  (where R-INV-4 actually lives) was never reached — `edit-text` on a
  genuinely present, genuinely locatable composite run returned the
  generic "text not found" refusal, never the R-INV-4 font-limitation
  refusal, on ANY input, ever. Same dead-guard-behind-a-filter shape as
  the Pass 19.4 `Tw` finding (`R-INV-4` this time, not `R91`) — see
  the RAG escalation, below, filed as a SECOND occurrence of that exact
  pattern rather than a new file. **Found by failing to reach the
  message twice**, not by reading code: once against the existing
  (undecodable) composite fixture, where `NoMatch` was arguably honest,
  and again against a purpose-built `cidfonttype2-with-tounicode.pdf`
  whose text IS genuinely findable — and still getting `NoMatch`. The
  second failure is what proved the bug. **Fix is ORDERING, not new
  machinery:** classify the anchor's font BEFORE `match_run`, since a
  font-level refusal is a property of the run, never of whether the
  sought text sits inside it — there was never a reason to match
  first. Composite runs are now decoded far enough to be **findable and
  no further** (`ShowSlot::code` is a `u8` and cannot hold a 2-byte
  CID, so decoding stops at "text populated, no code slots"). Verified
  all three arms by running them, not just testing them: injective-CMap
  composite → the real R-INV-4 refusal, naming pdfce's own limitation;
  no-CMap composite → still `NoMatch`, and that is HONEST (no character
  map means pdfce genuinely cannot say what text is there — narrowed,
  not eliminated); simple font → unchanged, still edits, exit 0.
  `6b69956` is a regression test asserting the ERROR VARIANT (not
  merely that an error occurs), verified non-vacuous by reverting the
  reorder and watching it fail with the wrong diagnosis, then
  restoring — its positive control matters because the fix reorders a
  path EVERY font shares, so the plausible break is refusing
  everything, which would satisfy a same-shaped composite assertion
  while silently destroying editing for every working document.
  **Honest limit, recorded not hidden:** the widened decode assumes
  `Identity-H` specifically; other CMap encodings on a composite run
  stay invisible to this decode path exactly as they were before this
  fix — narrowed, not regressed.

  **Substrate SHIPPED (continuation 78, 2026-08-04, `31d2fdc` +
  `b98589a`) — still NOT wired in, composite runs remain LOCATABLE-BUT-
  REFUSED, not yet EDITABLE.** `ShowSlot::code` widened `u8`→`u32` with
  a per-slot `width` (1 simple / 2 Identity-H) — landed alone, all 1801
  tests unchanged, since nothing downstream yet reads the new range.
  `CompositeEncoding` (character→CID via `injective_inverse()`, a
  SEPARATE type from the simple-font `InverseEncoding`, big-endian
  `to_bytes()`, CIDs above 16 bits refused rather than truncated) gives
  the encode side something to call once wiring lands. New fixture
  `composite-editable.pdf` (`/Type0`, 3 CIDs, injective `/ToUnicode`,
  "ABC") built ahead of the wiring code, deliberately. **A near-miss is
  on permanent record here, not just in the RAG:** widening the slot
  type made it POSSIBLE to push slots for composite runs before the
  rest of the wiring existed, and doing so would have silently disarmed
  `tests/composite_refusal_reachable.rs` (continuation 76) — that test
  currently passes BECAUSE composite runs produce no slots (match
  fails, wrong-but-caught `NoMatch` surfaces), not because it directly
  asserts the ordering it exists to guard. See the RAG escalation on
  the Pass 21.1 In-progress entry (continuation 78 build log, above)
  for the general framing and the discriminator (ask for ABSENT text —
  correct ordering still refuses, broken ordering returns `NoMatch`)
  that any future re-armed version of that test needs to use instead.

  **STILL OWED — the actual wiring, surveyed as FOUR coupled changes,
  not one** (continuation 78): `glyph_names()` returning `None` for
  composite must be checked before the existing `Unsupported` bail;
  `glyph_advance` must read `/W`/`/DW` (§9.7.4.3) for composite runs,
  a different table than `/Widths`, not a wider argument to the same
  lookup; `emit_edited_operator` must emit a hex string (`< … >`) with
  `CompositeEncoding::to_bytes()`'s bytes inside it for composite runs,
  not the literal `( … )` string it always writes today; `carried_codes`'
  subset-floor accounting must become width-aware. Deliberately NOT
  started this continuation — a half-applied version is worse than none,
  since all four touch the shipped in-place-editing path every existing
  edit relies on. R110's conditional lift is real but still has nothing
  to attach to. Pass 21.0 alone remains a capability regression (pdfce
  can add composite text it cannot edit) until all four land.

  **Ceiling is now R110** (was R106 before this entry).

- **★ Terminology ruling (operator, 2026-08-04) — "pdf dimension" vs
  "ce dimension," never bare "dimension." Codified as `CLAUDE.md` rule
  15, committed `89c5837`; cross-referenced from the Glossary, above
  (this project rule, like "Documentation-first" and the other
  unnumbered bullets at the top of this section, is not itself an
  R-numbered empirical/architectural rule).** Two unrelated things
  share the word "dimension" in this project with **opposite
  properties** — a **pdf dimension** is CAD/authoring-tool-exported
  content already in the file, which pdfce reads and measures against
  but must never alter; a **ce dimension** is a `/Line`+
  `/IT /LineDimension` annotation pdfce itself authors, fully editable
  and deletable. The distinction is provenance, not representation: a
  ce dimension stays one after save-and-reopen; a pdf dimension does
  not become one because pdfce can render it. **Why it is a rule, not
  a style note:** the operator could not decode analysis that used
  "dimension" throughout without saying which kind, and named the
  failure in both directions — ambiguous output from an agent is hard
  to act on, and an ambiguous report *from* him can send troubleshooting
  down the wrong path. Binding on every agent, every reply, every
  commit message, every decision record, every RAG entry, and every
  subagent dispatch. **Audited this filing (`pdfce-librarian`) against
  the ROADMAP Backlog dimensioning bullets, Pass 21.x, and the
  2026-08-04 SESSION_LOG continuations** — Pass 21.x (FF-C font
  subsetting) has no "dimension" mentions at all and needed no change;
  the 2026-08-04 SESSION_LOG continuations (75–78) likewise had none
  prior to this filing; the one existing bucket that did — the
  "Dimension-tool bug-fix cluster," Pass 12.M2c, Backlog above — was
  entirely about ce dimensions (pdfce's own dimensioning tool) and has
  been retitled/qualified in place (Backlog is a mutable current-state
  section, not append-only, so this was an in-place edit, not a dated
  footer). **The load-bearing finding is at the decision-record level,
  not the prose level** — see new Open operator question (t), above:
  decisions 022 and 023 (filed 2026-08-04, not yet promoted into
  ROADMAP Pass entries) are scoped almost entirely around ce
  dimensions, while the operator's original box-select complaint was
  very likely about pdf dimensions in a CAD export, which decision
  023's independently-found form-XObject paint/select asymmetry (Pass
  23.2), not decision 022 (Pass 22.0), actually addresses. **RAG
  escalation:** the generalizable finding — an ambiguous overloaded
  term in project documentation eventually becomes an ambiguous SCOPE,
  not merely an ambiguous sentence — is filed to
  `D:\dev\rag\rust\overloaded_term_ambiguity_becomes_scope_ambiguity.md`
  this session (worked example: this exact case), indexed in
  `D:\dev\rag\rust\index.md`.

- **R111 — Selection enumerates exactly what the renderer paints
  (decision 022 §8, filed continuation 80, 2026-08-04;
  librarian-assigned number).** Any object space the canvas paints must
  be hit-testable through the SAME visibility predicate the renderer
  uses, defined once in `pdfce-core`, never re-derived in `pdfce-gui`.
  A paint/hit-test asymmetry is a defect of the same class as decision
  011's Z2 two-decompositions divergence, one object space over. This
  rule's absence is the whole cause of decision 022 existing: ce
  dimensions were painted from Pass 6.1 onward and selectable from
  never, and no gate existed to notice. **Amended by decision 023 §8
  item 8 (2026-08-04): a SECOND, independent live violation of this
  exact rule was found in form XObjects** — `pdfce-render`'s
  interpreter recurses into a `Do` on a form (painting its contents
  individually) while `decompose.rs` emits one opaque object for the
  same `Do`, so this rule's first gate must cover both object spaces,
  not just annotations. Enforcement: decision 022's A3 (annotation
  visibility, Pass 22.0) and decision 023's C2 (form-content visibility,
  Pass 23.2).
- **R112 — A selectable kind carries its verb set in its type
  (decision 022 §8, filed continuation 80, 2026-08-04;
  librarian-assigned number).** Handles like `TargetId` are an enum
  over kinds, and every verb dispatch on that handle is an exhaustive
  match; a verb that does not apply to a kind is UNREPRESENTABLE, never
  a silent no-op. This is the structural form of R83: R83 forbids the
  affordance for an unavailable verb, this rule forbids the shape that
  makes adding that affordance by accident easy in the first place — a
  half-polymorphic verb set (select works, move silently does nothing)
  is worse than not selecting at all. **Amended by decision 023 §8
  item 7 (2026-08-04): STRENGTHENED — the handle must also express
  LEVEL** (which container/depth an object sits at), or level becomes a
  runtime convention of exactly the class decision 022 §2 option (d)
  (the rejected tagged-integer partition) already refused. Concretely:
  `TargetId::Content`'s payload grows from a bare `u64` to `ContentPath
  { stream, index }` in Pass 23.2 — payload only, no further substrate
  change, a direct dividend of this rule choosing the enum over a
  tagged integer in the first place.
- **R113 — Deleting a pdfce-authored annotation prunes its
  `/PieceInfo` record in the same command (decision 022 §5.3/§8, filed
  continuation 80, 2026-08-04; librarian-assigned number).** The
  sidecar and the document object graph must never diverge across one
  undoable command, in either direction. Derived from a concrete,
  found-not-hypothesized corruption path: `set_group_scale`
  (`edit.rs:6083-6130`) pushes an `ObjectWrite` replacing every
  member's `/AP` stream UNCONDITIONALLY, while the annotation-dict half
  of the same loop is guarded — so deleting an annotation generically
  while its `DimensionRecord` survives in the sidecar causes the next
  scale change to write a fresh `/AP` stream at a REMOVED object id,
  resurrecting an orphan appearance stream nothing references, and
  `dimension-list` keeps reporting a ce dimension that is not on the
  page (a disclosure lie, rule 4). `EditSession::delete_annotation` (Pass
  22.0) must use the guarded-write pattern and prune the sidecar in the
  SAME command, not a follow-up one. Also inherited by
  `set_dimension_geometry` (Pass 23.1, decision 023 §5.5 item 1) — the
  same corruption shape reintroduced from a re-measure direction if not
  guarded there too.
- **R114 — Every new selection or gesture surface ships a headless
  oracle in the same Pass (decision 022 §8, filed continuation 80,
  2026-08-04; librarian-assigned number).** Rule 11's logic (each
  feature ships its `pdfce-cli` surface) applied specifically to
  VERIFICATION surfaces, not just authoring ones. Concrete gap this
  closes: `hit_test` has had `object-list --hit` since Pass 9a;
  `hit_test_rect` (the marquee) had NOTHING, and the marquee gap this
  decision exists to close was found by a human clicking the app, not
  by any gate. Decision 023's `object-list --tree`/`--level N`
  (Pass 23.2) is the same rule applied to descent: descent is a pure
  function of (point, level, container forest), so it is fully
  assertable headlessly even though the marquee's visual reading is
  not.
- **R115 — A PDF page's structural hierarchy is a laminar interval
  family over paint order, plus a forest of content streams — never a
  re-parented object list (decision 023 §9 item 1, filed continuation
  80, 2026-08-04; librarian-assigned number).** A `BDC`…`EMC` sequence
  (§14.6) is by definition a contiguous token range, so the objects it
  encloses are a contiguous index range in paint order; nesting
  produces nested ranges, never crossing ones. A form's contents live
  in a DIFFERENT content stream and were never in the page's index
  space to begin with, so giving them their own flat list is the
  truth, not a workaround. Any container model preserves each stream's
  flat paint-order index space byte-for-byte; hierarchy is an INDEX
  over it, a view, never the canonical structure. Protects the exact
  agreement `EditSession::vector_surgery`'s content-only decompose and
  the GUI provider's `decompose_page` share BY CONSTRUCTION (decision
  022 §2 option (e)'s reasoning) — the agreement that makes
  `object-move --object 2` and a GUI drag mean the same thing — at the
  one moment (adding hierarchy) most likely to break it silently.
- **R116 — Editing inside a form XObject is refused while that form is
  invoked more than once, and the invocation count is MEASURED, not
  assumed (decision 023 §1.4/§9 item 2, filed continuation 80,
  2026-08-04; librarian-assigned number).** A form's content stream is
  ONE object; `Do` paints it N times. A correct, minimal, byte-perfect
  edit to that shared stream is still wrong for every placement but the
  one the operator meant — editing a path inside a title block placed
  on 12 pages changes it on all 12. Not a bug to fix; it is what a form
  XObject IS (§8.10 — reuse is the entire point). Named refusal:
  `FormStreamIsShared { form: ObjId, invocations: usize }`, reachable
  trivially (any repeated block) and owed a test asserting it FIRES
  (R96). When the count is exactly 1, this is DISCLOSED (not silent),
  so the operator learns the rule from the common case rather than only
  from the refusal.
- **R117 — A shipped capability reachable from no surface is a defect
  of the same class as an affordance with no capability — R83's
  inverse (decision 023 §9 item 3, filed continuation 80, 2026-08-04;
  librarian-assigned number).** Every constructor of an
  operator-visible formatting or behavior variant is reachable from at
  least one surface (GUI or CLI), or it is deleted — a capability that
  exists in code but that no operator can ever invoke is exactly as
  dishonest as an affordance that promises a capability that doesn't
  exist, just in the opposite direction. Sourced concretely:
  `NumberFormat::inch_fraction` and `FractionMode::Fraction{reduce:
  true}` are both live, documented with runnable examples, spec-mirrored
  into `/Measure`, unit-tested — and constructed from NOWHERE outside
  that one test. No operator, GUI or CLI, can ask pdfce to do something
  it already fully implements. Closed by Pass 23.0's `--style`/
  `--denominator`/`--reduce` CLI surface, which costs nothing beyond
  argument parsing (R96 governs the CLI: a name must be reachable).
- **R118 — A display-format preference never rides the gesture that
  first set it (decision 023 §9 item 4, filed continuation 80,
  2026-08-04; librarian-assigned number).** Format lives on the
  persistent entity (a ce-dimension group), is settable independently of
  whatever transient gesture originally set it (a scale-entry dialog),
  and shows a live sample of the operator's OWN data before Apply.
  Sourced from a concrete coupling defect: `set_group_scale(scale,
  format)` is the only setter that exists, so precision is only
  changeable today by re-drawing a scale line — an artifact of where
  the control happened to land, not a design. `set_group_format`
  (Pass 23.0) decouples the two.
- **R119 — A geometry change to a measurement is two-stage and
  disclosed: old → new visible before commit, and mouse-up is never
  the commit (decision 023 §9 item 5, filed continuation 80,
  2026-08-04; librarian-assigned number. Preserves decision 022 §4.2's
  principle as a rule now that the capability — re-measure — is
  wanted, rather than discarding the principle along with the original
  prohibition).** In a measurement tool specifically, a drag that
  silently changes a reported measurement is the single sneakiest thing
  the application could do (rule 4) — the argument was always about
  SILENCE, not about whether re-measure should exist at all. What must
  be on screen before Accept can be pressed: the old value, the new
  value, the delta, and the group/scale the new value was computed
  under. Reject or Escape reverts to the pre-drag geometry via the
  shipped `CancelGesture` mechanism. Governs `set_dimension_geometry`
  (Pass 23.1).
- **R120 — A precedence chain resolved from booleans takes a
  named-field context, not positional arguments (decision 023 §9 item
  6, filed continuation 80, 2026-08-04; librarian-assigned number,
  numbered despite the decision document's own hedge that it "may be
  judged too small for a number" — R106 is direct precedent that a
  methodology rule earns a number when it materially prevents a
  concrete collision risk, and this one does).** Concrete risk this
  closes: `resolve_escape(tool_active: bool, gesture_discardable: bool,
  canvas_selection_nonempty: bool)` is about to be edited by TWO
  concurrent Passes (the in-flight navigation Pass's context-menu-
  dismissal slot, and Pass 23.2's `LeaveGroupLevel` slot) — each
  appending a positional `bool` produces
  `resolve_escape(true, false, true, false, true)` at the call site, a
  shape where a transposed argument compiles, type-checks, and silently
  reorders Escape's precedence. Fixed by whichever Pass lands first
  changing the signature ONCE to a named-field `EscapeContext` struct;
  each subsequent Pass then ADDS a field, and a transposition becomes a
  name error instead of a silent behavior change.

  **Ceiling is now R120** (was R110 before this entry).

## Update protocol

- New operator request → engineer parses into Pass entry/entries →
  dispatches pdfce-librarian to add under *Backlog* or *Next up* →
  reports assigned Pass IDs back to the operator.
- Backlog bucket → real Pass (scoping) → engineer dispatches
  `pdfce-acrobat-librarian` for the matching feature area first, so
  the Pass's acceptance criteria are grounded in actual Acrobat
  behavior before they're written down.
- Pass completion → engineer dispatches pdfce-librarian with
  completion details (summary, test results, packaging-smoke-test
  result) → librarian moves the entry to *Shipped* (top, reverse
  chronological) and appends a `SESSION_LOG.md` entry.
- Shipped entries are never rewritten. A reverted Pass gets a new
  "Pass NN — revert of Pass MM" entry, not a deletion.
