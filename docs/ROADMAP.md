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

## Shipped

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

### Beta — Scaled measurement / dimensioning tool (decision 011 ARCHIVED; prerequisites COMPLETE; awaits operator go-ahead)

**Promoted to In progress 2026-08-01 by operator REPRIORITIZATION.** Ken
has requested a **measurement / dimensioning BETA** as his first usable
deliverable: **scaled dimensions + vector selection/snapping + basic
vector editing**. Its architecture is now DECIDED and ARCHIVED at
`docs/decisions/011-first-beta-scaled-measurement-dimensioning-tool.md`
(five slices — **12.0 / 9a / 12.M1 / 12.M2 / 9c-min**). **Do NOT invent the
beta's Pass IDs / slices here — decision 011 defines them.**

**READY-TO-START status (2026-08-01):** the beta's research prerequisites
are **COMPLETE** — the spec slices (§12.9 measurement / §14.5 optional
content / §8.11 measurement dictionaries), the Acrobat measuring-tools
feature bucket, and the Inkscape selection+snapping capability bucket are
all sourced. **The beta build now awaits operator go-ahead** — Ken is
reviewing the plan; the engineer starts on his confirmation.

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
- **Forms (AcroForm)** — field creation/editing, appearance-stream
  generation, form-field auto-detection (as a *hint*, per
  fuzzy-never-sneaky), flatten-to-static.
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
  gone from the saved bytes, not just hidden.
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
  **Explicitly BLOCKED ON `LEGAL.md` §1** (Scoop requires a `license`
  manifest property; WinGet requires `License` in the defaultLocale
  manifest; no public repo/release until the license is decided). The
  work, in order:
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
  AWAITING OPERATOR DECISION — DO NOT SCHEDULE TO A PASS.** Filed
  2026-08-01 by decision 016 §10 (`docs/decisions/016-ffd-add-new-page-text.md`),
  ranked **#2 by value** in decision 016's text-parity fast-follow
  prioritization (§2) — it lifts the single most common wall in real
  editing, the embedded-subset "can't originate that glyph" refusal
  from `text_edit__font_handling_on_edit.md` (R71) — but it **cannot
  start solo**: adding a font subsetter/embedder is a new Cargo
  dependency, which triggers **rule 13** (copyleft classification —
  must be flagged to and approved by the operator, never decided solo)
  and is gated by **rule 8** (pdfce's own OSS license is still
  undecided, `LEGAL.md` §1 — which gates what dependency posture is
  even usable). **Recommendation surfaced by decision 016:** unblock in
  parallel with the now-scoped Pass 16.x (FF-D) build — operator
  approves a permissive-only subsetter path and, ideally, settles
  `LEGAL.md` §1, so FF-C can follow FF-D directly; the
  `pdfce-spec-librarian` font-subsetting dispatch (named at decision
  014) is queued to run meanwhile so the spec grounding is ready when
  the operator unblocks it. No Pass number invented here.

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
  faked, never silently substituted. Font-subsetting/glyph-embedding (FF-C)
  is a deferred writer subsystem, permissive-only (rule 13).
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
- **R79 — New text uses a bundled/supplied face by name+code, no embedding,
  with disclosed provenance (decision 016, 2026-08-01; librarian-assigned
  number).** A newly-added run defaults to a bundled Standard-14 permissive
  face (§9.6.2.2 — no embedding, sidestepping the FF-C embedded-subset wall),
  operator-configurable via decision 012's `GlyphSource`; a glyph the chosen
  face lacks is refused-and-disclosed (R71), never faked; the run's font
  source is disclosed (`Bundled`/`Supplied`), never presented as the
  document's own. This is why FF-D needs no FF-C to ship.

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
