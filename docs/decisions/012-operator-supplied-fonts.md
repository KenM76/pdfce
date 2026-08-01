# 012 — Operator-supplied fonts for non-embedded text

**Date:** 2026-07-31
**Status:** Decided
**Decided by:** KenAgent (`autonomous-builder`), per `docs/decisions/README.md`
**Requested by:** `pdfce-engineer` (a real drawing rendered with missing text)
**Supersedes:** nothing
**Amends:** `docs/ROADMAP.md` standing rules (adds R61–R65); the R20
diagnostics contract (three trust levels, not two); the R19 determinism
scope note (bundled-only)
**Does not touch:** `LEGAL.md` §1 (license stays open); decision 003's
D2/R10/R11 invariants (preserved, see §5.1); R17 (no shaping, ever)

---

## 1. Context

An operator's real drawing rendered with a visible text gap. Two
production cases produce this today:

- **(a)** a non-embedded, non-Base-14 **simple** font — a document that
  references `Calibri` or `Consolas` without embedding a program. Today
  it substitutes to a bundled Foxit Base-14 face by descriptor class
  (decision 004 §4.2): shapes are plausible, positions exact, but the
  face is Helvetica/Times/Courier, not Calibri.
- **(b)** a non-embedded **composite** font (Type0 / CIDFontType2 /
  CIDFontType0 — CJK and friends). Today it is a hard skip
  (`UnsupportedFont::CompositeNotEmbedded`, `text.rs:657`): there is no
  defined mapping from an arbitrary CID to a substitute face's glyphs,
  so pdfce refuses rather than paint confident nonsense (§9.7.5.2
  forbids `Identity-H` with a non-embedded font).

The operator asked for a way to **supply the missing fonts** — point
pdfce at the actual Calibri, and have it drawn. The whole task is to do
that without breaking the load-bearing invariants it is in tension with:
decision 003's portable / no-system-dependency posture, decision 004's
deterministic-by-default renderer (R19), and the fuzzy-never-sneaky
disclosure rule (CLAUDE.md rule 4, ROADMAP R20).

### 1.1 The seam already exists

Decision 004 §5.3 built the `FontEnvironment` override seam for exactly
this future, and said so in as many words: *"Users will eventually want
to point pdfce at a specific font for a specific document… an override
seam is the difference between a renderer that can be told things and
one that cannot."* `FontEnvironment::insert_named(base_font, data)`
(`font/mod.rs:126`) already exists, and `substitute_face` already
consults `env.named(base_font)` **before** the bundled descriptor
fallback (`text.rs:773`). This decision is mostly about **who fills that
map, how the match is made, and how the result is disclosed** — not
about inventing a new mechanism.

### 1.2 Constraints this decision must serve

1. **R19 — deterministic by default.** `pdfce-render` never discovers,
   opens, or reads a font from the filesystem, environment, or OS. Bytes
   arrive only through the API. This is what makes the golden-pixel and
   differential (R59) tests meaningful and the WASM fork work with no
   shell support.
2. **Decision 003 D2 / R10 / R11.** Single-folder portable, no installer,
   no system-wide runtime dependency; no `cfg(target_os)` in
   `pdfce-core`/`pdfce-render`; the wasm32 cross-check stays green.
3. **Fuzzy, never sneaky (rule 4 / R20).** Every substitute is a
   disclosed, reviewable fact — the operator must always know a shape is
   not the document's own, which face drew it, and that positions still
   come from the PDF's widths.
4. **R21 — one font parser.** No second parser enters the read path.
5. **Rule 13 — no copyleft font dependency** for loading/enumeration;
   any dependency is classified permissive-only and escalated.
6. **GUI-core separation.** Font *loading* lives in core/render; the
   font-folder *setting* lives in the GUI/CLI shell.

---

## 2. Options considered

**Supply mechanism.**
- **S1 — folder only.** Operator points pdfce at a directory; the shell
  walks it and registers faces.
- **S2 — folder + opt-in OS-font directory** (e.g. Windows Fonts).
- **S3 — OS fonts on by default** (like a normal desktop renderer).

**Matching.**
- **M1 — exact BaseFont name (subset-tag-stripped) only.**
- **M2 — name, then descriptor-class auto-routing to a supplied face.**

**Composite (CID) substitution.**
- **C1 — never** (keep the hard skip).
- **C2 — via the Unicode route** (CID→ToUnicode/predefined-CMap→Unicode→
  supplied-face cmap→GID), disclosed lossy, skip when no mapping.
- **C3 — GID-guess** (map CID→GID by index/heuristic).

**Disclosure.**
- **D-a — reuse the existing single 'substituted' bucket** (two levels:
  embedded vs substituted).
- **D-b — three trust levels** (embedded / bundled / supplied), counted
  and surfaced separately.

---

## 3. Decision

**Supply mechanism — S1 now, S2 as a named fast-follow, S3 rejected.**
The first cut is **folder-only**: a GUI "Font folders" preference list
and a CLI `--font-dir <DIR>` (repeatable). The **shell**
(`pdfce-gui`/`pdfce-cli`) walks the folder with `std::fs`, reads each
file, asks `pdfce-render` for the face's advertised name(s), and
registers each into a `FontEnvironment` via `insert_named`. The renderer
receives bytes through `RenderOptions.fonts` and never touches the disk —
R19 stays literally true.

**OS-font-directory access is explicit opt-in only, off by default,
never a build-time dependency, and deferred to fast-follow FF1.** The
portable build ships **zero external font dependencies**; the bundled
Foxit 14 are the deterministic floor with or without OS access (R65).
S3 is rejected outright: OS fonts on by default is a system dependency
and a determinism hazard (two machines → two renders) that decisions 003
and 004 exist to prevent.

**Matching — M1.** Reuse `select.rs`. The named-face lookup precedence is
`env.named(BaseFont verbatim)` → `env.named(strip_subset_tag(BaseFont))`
→ existing bundled descriptor fallback. The shell registers each supplied
face under both its internal PostScript/family name and its filename
stem. **Descriptor-based auto-routing to a supplied face is NOT done**
(fuzzy, style-mismatch risk); auto-matching is by name. Manual slot
mapping via the existing `insert_fallback` is a possible advanced
fast-follow (FF3).

**Composite (CID) substitution — C1 now, C2 as fast-follow FF2, C3
rejected permanently.** The first cut leaves `load_composite`'s hard skip
unchanged; operator-supplied fonts do **not** attempt composite
substitution. It is a **named non-goal**. If ever added (FF2), it goes
**strictly** via the Unicode route, is disclosed as lossy, and **skips
(never guesses)** when no Unicode mapping exists — `Identity-H` without
`ToUnicode` stays a hard skip forever (R64). C3 (GID-guessing) is the
"sneaky" failure mode rule 4 forbids and is rejected outright.

**Disclosure — D-b, three trust levels.** Replace `LoadedFont.substituted:
bool` with `GlyphSource { Embedded, Bundled, Supplied }`. `Diagnostics`
gains `glyphs_supplied` and `supplied_fonts`, **distinct** from the
existing bundled `glyphs_substituted`/`substituted_fonts`. The GUI R20
panel and the CLI summary show all three separately (R62).

---

## 4. Rationale

### 4.1 Why folder-first and OS-opt-in-only

The whole point of decisions 003 and 004 is that pdfce's output does not
depend on the machine it runs on. A folder the operator explicitly points
at is a *deliberate, disclosed* input — it does not make pdfce depend on
what happens to be installed. **OS-font enumeration does**: it silently
makes the same document render differently on two machines, it is a
genuine platform dependency (a `cfg(target_os)` path to `%WINDIR%\Fonts`
or its peers), and it invites a "why do my pixels differ from my
colleague's?" question with no visible cause. So OS access is opt-in,
off by default, never a build-time dep, and even when enabled it lives
entirely in the shell (R61) and is disclosed as determinism-breaking
(R63). Folder-first captures ~all of the operator value (the missing
Calibri) with none of the determinism cost.

### 4.2 Why the renderer must stay bytes-in (R19/R61)

If `pdfce-render` grew a folder walk, three things break at once: the
WASM fork (no filesystem) stops matching the native renderer; the R59
differential gate stops being reproducible; and `cfg(target_os)` enters
a crate decision 003 R10 forbids it in. Keeping the walk in the shell and
the renderer bytes-in preserves all three. The seam was designed for this
(004 §6.3): the renderer "never *obtains* bytes — it only ever receives
them."

### 4.3 Why composite substitution is deferred, not solved

A substitute face's glyph IDs do not correspond to arbitrary CIDs — that
is the whole reason `CompositeNotEmbedded` is a hard skip. The only sound
bridge is **through Unicode**: `CID → (ToUnicode ▸ predefined CMap) →
Unicode → supplied-face (3,1) cmap → GID`. It is lossy (it recovers a
character, not the producer's exact glyph), and it is **impossible** for
`Identity-H` with no `ToUnicode`, which carries no character semantics at
all. Worse, it requires `ToUnicode` in the **render** path, which
decision 004 §7 deliberately kept in extraction/core — a real new
coupling. For the first cut the value is overwhelmingly in the simple-font
case (the Calibri gap), so composites are a named non-goal with the
correct algorithm recorded for FF2. GID-guessing (C3) is rejected because
it paints confident nonsense — precisely the "sneaky" behavior rule 4
exists to forbid.

### 4.4 Why three trust levels, not two

Today `substituted: bool` collapses "pdfce guessed a Base-14 face" and
"the operator supplied their own Calibri" into one bucket. Those mean
different things to an operator reading the page: a bundled substitute is
*pdfce's* plausible shape; a supplied face is *the operator's own*
shape, chosen deliberately. Both are still substitutes — neither is the
document's embedded program — and neither may ever be presented as
embedded. Three counters (`glyphs_notdef` unchanged; bundled
`glyphs_substituted`; new `glyphs_supplied`) plus two name lists make the
distinction visible. Crucially, the decision-004 §3.6 fact still holds
for supplied faces: **positions come from the PDF's own `/Widths`**, so
inter-glyph layout is exact regardless of which face draws — a supplied
face improves *shapes*, not *positions*, and the disclosure copy must say
so, so nobody mistakes "I supplied the font" for "the layout is now
authoritative."

### 4.5 Why this costs nothing structurally

Folder mode adds **no dependency** (`std::fs` + the one skrifa parser
already in `program.rs`, R21), so rule 13 has nothing to classify and
`THIRD_PARTY_LICENSES.md` is untouched. `pdfce-core` is untouched.
`pdfce-render` gains an enum, a subset-strip on an existing lookup, a
`face_names()` helper on the existing parser, and two diagnostics
counters — no filesystem, no OS, no `cfg`. The wasm32 check and the
`cargo tree` invariants stay green.

---

## 5. What this decision produces

### 5.1 The invariant it preserves (decision 003 tension, resolved)

D2 (single-folder portable, no system dependency) is **not** violated:
a user-pointed font folder is optional and opt-in — pdfce runs
folder-clean with only the bundled 14, requires nothing system-wide,
writes no registry, needs no installer. The one genuine tension is
OS-font enumeration, and it is quarantined: opt-in, off by default,
zero build-time dep, shell-only, disclosed as determinism-breaking. R10
(no `cfg(target_os)` in core/render) and R11 (wasm32 clean) are held by
keeping every filesystem/OS path in the shell and the renderer bytes-in.

### 5.2 Standing rules (binding; add to `ROADMAP.md`, continuing R60)

- **R61 — Supplied faces are shell-sourced, never renderer-discovered.**
  The folder walk (and any future OS enumeration) lives in
  `pdfce-gui`/`pdfce-cli`; `pdfce-render` only ever *receives* bytes
  through the `FontEnvironment` seam. No filesystem access and no
  `cfg(target_os)` enters `pdfce-core`/`pdfce-render` under this feature.
- **R62 — Three glyph trust levels, always disclosed distinctly.**
  Embedded (document's own program, exact) / Bundled (Foxit Base-14,
  plausible) / Supplied (operator's own face). Counted and surfaced
  separately; a supplied glyph is never shown as embedded.
- **R63 — Supplied faces are outside the determinism guarantee and the
  R59 gate.** The gate constructs a bundled-only `FontEnvironment`;
  supplied-font renders are machine-dependent by definition, and the UI
  discloses when supplied fonts are active. R19's "same input → same
  pixels" is scoped to the bundled set.
- **R64 — Composite (CID) substitution, if ever added, is Unicode-route
  only.** `CID → (ToUnicode ▸ predefined CMap) → Unicode → supplied-face
  cmap → GID`, disclosed lossy, skipping (never GID-guessing) when no
  Unicode mapping exists. `Identity-H` without `ToUnicode` stays a hard
  skip permanently.
- **R65 — OS-font access is explicit opt-in, never default, never a
  build-time dependency.** The portable build ships zero external font
  dependencies; the bundled 14 are the deterministic floor.

### 5.3 Code changes (first cut)

- `font/mod.rs` / `text.rs`: `LoadedFont.substituted: bool` →
  `GlyphSource { Embedded, Bundled, Supplied }`; `substitute_face`
  returns which slot it drew from; `load_simple` threads it through.
- `text.rs` `substitute_face`: named lookup also tries
  `strip_subset_tag(base_font)`; precedence documented in the doc comment.
- `font/program.rs`: `face_names(&[u8]) -> Vec<String>` on the existing
  parser (R21) so the shell registers faces without a second parser.
- `interpret.rs` / `annot.rs` `Diagnostics`: add `glyphs_supplied` and
  `supplied_fonts`, distinct from the bundled counters; update `merge()`
  and the counting sites.
- `pdfce-cli`: `--font-dir <DIR>` (repeatable) → walk, parse names, build
  `FontEnvironment`, pass via `RenderOptions`; summary prints the
  three-way disclosure.
- `pdfce-gui`: "Font folders" preference (persisted in the R15
  user-state partition, not the replaceable payload); R20 panel shows
  bundled vs supplied; a visible "supplied fonts active" indicator.
- R59 harness: build a bundled-only `FontEnvironment` explicitly; assert
  font-dir-independence.

### 5.4 Acceptance

A non-embedded `Calibri` PDF draws with a supplied `Calibri.ttf` under
`--font-dir` and with bundled Helvetica without it; both disclose the
correct trust level and the actual face name; **glyph positions are
identical** across both runs. Subset-tagged and style-variant references
resolve to matching supplied variants, else fall to bundled (disclosed).
A corrupt supplied file fails clean to bundled, never errors the page.
Composite non-embedded still returns `CompositeNotEmbedded`. The R59
corpus gate is byte-identical regardless of any font-dir config.
`cargo tree` adds zero packages; wasm32 green; no `cfg(target_os)` in
core/render; fmt/clippy clean.

---

## 6. What this decision explicitly does NOT decide

- **Composite / CID substitution** — named non-goal; FF2 only, via R64's
  Unicode route.
- **OS-font enumeration** — FF1; explicit opt-in, off by default, zero
  build-time dep; the platform path and variant-selection logic are the
  reason it is deferred rather than shipped now.
- **Descriptor-based auto-routing** of unknown fonts to a supplied face —
  matching is by name; FF3 (manual slot mapping) is the fuzzier, opt-in
  successor.
- **Any shaping / GSUB / bidi** — R17, never in the render path.
- **The write side** (font embedding/subsetting) — unrelated.

---

## 7. Revisit triggers

1. **A corpus measurement shows composite non-embedded fonts are a
   material share of the missing-text cases.** Schedule FF2 (R64
   Unicode-route), which also depends on `ToUnicode` reaching the render
   path.
2. **Operators ask for OS fonts.** Ship FF1 behind an explicit toggle
   with the R63 determinism disclosure; keep the platform path shell-side.
3. **Name-matching misses at a material rate** (producers writing exotic
   `BaseFont` spellings). Enrich the shell's registration keys before
   loosening the render-side match toward descriptor auto-routing (FF3).
4. **The WASM fork gains a file-picker.** It populates named faces from an
   `ArrayBuffer` — same seam, no renderer change; confirm R61 still holds.

---

## 8. References

- **Code:** `font/mod.rs` (`FontEnvironment`, `insert_named`/`named`,
  `RenderOptions`); `font/select.rs` (`by_name`/`by_descriptor`/
  `strip_subset_tag`); `font/program.rs` (the one skrifa parser, R21);
  `text.rs:334` (`substituted`), `:765` (`substitute_face`), `:626`
  (`load_composite` → `CompositeNotEmbedded`); `interpret.rs:199/225/452/
  492/1140/1300` (diagnostics + merge + counting); `annot.rs:813`.
- **Decisions/rules:** decision 003 §D2/R10/R11 (portable, platform-clean,
  wasm invariant); decision 004 §4.2/§5.3/§6.3 (the seam), §3.6
  (positions-from-widths), §7 (ToUnicode in extraction), R17–R21; ROADMAP
  R19/R20/R59; CLAUDE.md rule 4 (fuzzy-never-sneaky), rule 13 (no copyleft
  font dep).

## Appendix A — JSON decision block

```json
{
  "decision_id": "012-operator-supplied-fonts",
  "title": "Operator-supplied fonts: folder-based supply for non-embedded fonts, three-way substitution disclosure",
  "status": "Decided",
  "requested_by": "pdfce-engineer",
  "decided_by": "KenAgent (autonomous-builder)",
  "date": "2026-07-31",
  "confidence": "high",
  "one_line": "Ship folder-based operator font supply for non-embedded SIMPLE fonts, riding the existing FontEnvironment.named seam; disclose embedded/bundled/supplied as three distinct trust levels; keep the renderer I/O-free (shell owns the folder walk); defer composite/CID substitution and OS-font enumeration as named fast-follows.",

  "design": {
    "supply_mechanism": {
      "first_cut": "Folder-only. The operator points pdfce at one or more directories (GUI 'Font folders' preference list; CLI `--font-dir <DIR>`, repeatable). The SHELL (pdfce-gui / pdfce-cli) walks the folder with std::fs, reads each file, asks pdfce-render for the face's advertised name(s), and registers each face into a FontEnvironment via `insert_named`, which the renderer already consults first in `substitute_face` (text.rs:773). The face bytes reach the renderer only through RenderOptions.fonts — R19 stays literally true.",
      "os_font_directory": "Explicit opt-in ONLY, never default, and a FAST-FOLLOW (FF1), not the first cut. When enabled it reads a fixed known location (e.g. %WINDIR%\\Fonts) in the SHELL, with any platform path guarded by a cfg comment in pdfce-gui/pdfce-cli — never in core/render (R10). Never a build-time dependency. The portable build ships ZERO external font dependencies; the bundled Foxit 14 remain the deterministic floor with or without OS access.",
      "wasm_interaction": "Folder/OS supply is a native-shell capability. pdfce-render's seam is filesystem-agnostic (it receives bytes, never obtains them), so the WASM fork simply populates no named faces — or populates from a browser file-picker → ArrayBuffer, still bytes-in. No filesystem assumption or cfg(target_os) leaks into core/render; R10/R11 hold; the fork stays a shell-crate swap."
    },
    "tension_with_decision_003_resolved": "D2 (single-folder portable, no system dependency) is NOT violated: a user-pointed font folder is optional and opt-in — pdfce still runs folder-clean with only the bundled 14, so nothing is REQUIRED system-wide, no registry, no installer. The real tension is OS-font enumeration (a genuine platform/system dependency and a determinism hazard), which is exactly why it is opt-in-only, off by default, never a build-time dep, and deferred to FF1. R10/R11/R19 are preserved by keeping ALL filesystem/OS logic in the shell and the renderer bytes-in.",
    "matching": {
      "reuse": "select.rs unchanged in spirit; the named-face path is extended. Precedence in substitute_face: (1) env.named(BaseFont verbatim) → (2) env.named(strip_subset_tag(BaseFont)) → (3) existing bundled descriptor fallback. The shell registers each supplied face under BOTH its internal PostScript/family name AND its filename stem, so 'Calibri.ttf' matches a PDF BaseFont 'Calibri'.",
      "subset_tag": "strip_subset_tag (select.rs:17) already handles 'ABCDEF+Calibri'→'Calibri'; the named lookup must apply it (currently only the verbatim string is tried at text.rs:773).",
      "descriptor_to_supplied": "NOT auto-routed in the first cut (fuzzy, risks style mismatch). Auto-matching to a supplied face is by NAME only. Manual descriptor→face slot mapping via the existing insert_fallback is a possible advanced fast-follow (FF3)."
    },
    "composite_cid_hard_case": {
      "first_cut_behavior": "UNCHANGED hard skip. load_composite (text.rs:626) continues to return UnsupportedFont::CompositeNotEmbedded when a Type0 font has no embedded program. Operator-supplied fonts do NOT attempt composite substitution in the first cut. Named non-goal.",
      "why": "A substitute face's glyph IDs do not correspond to arbitrary CIDs. The ONLY sound route is CID → (ToUnicode ▸ predefined CMap) → Unicode → supplied-face (3,1) cmap → GID. It is lossy, works only when a Unicode mapping exists, and CANNOT work for Identity-H without ToUnicode. It also requires ToUnicode in the RENDER path, which decision 004 §7 deliberately left in extraction/core — a real new coupling not worth taking for the first cut.",
      "fast_follow_FF2": "If added, strictly via the Unicode route above; SKIP (never GID-guess) when no Unicode mapping exists; disclose as supplied-AND-lossy (a distinct sub-state). Identity-H without ToUnicode stays a hard skip forever. This is the only route that is 'fuzzy, never sneaky' for composites."
    },
    "three_trust_levels": {
      "model": "Replace LoadedFont.substituted: bool (text.rs:334) with an enum GlyphSource { Embedded, Bundled, Supplied }. Embedded = the document's own program (exact). Bundled = a Foxit Base-14 face (plausible-Base-14). Supplied = an operator's own face (operator's-own-face).",
      "disclosure": "Diagnostics gains glyphs_supplied: usize and supplied_fonts: Vec<String>, DISTINCT from the existing bundled glyphs_substituted / substituted_fonts (which now mean bundled-only). GUI R20 panel and CLI summary show all three distinctly. A supplied glyph is NEVER presented as embedded.",
      "positions_still_from_pdf": "The decision-004 §3.6 invariant holds for supplied faces too: text is positioned from the PDF's own /Widths, so inter-glyph positions are exact regardless of which face draws. A supplied face improves SHAPES, not positions; the only artifact is sidebearing. Disclosure must say this — supplied is 'closer shapes,' not 'more correct layout.'"
    },
    "where_code_lives": {
      "pdfce-core": "Untouched. No font-folder logic.",
      "pdfce-render": "GlyphSource enum + the named-lookup subset-strip + a public helper `face_names(&[u8]) -> Vec<String>` (reusing the ONE skrifa parser in program.rs — R21, no second parser) + the two new Diagnostics counters and their merge/count wiring. Still no filesystem, no env, no OS, no cfg(target_os).",
      "pdfce-gui / pdfce-cli": "Own the std::fs directory walk, the font-folder SETTING (GUI preference; CLI flag), any future OS-path logic, and the three-way disclosure surface."
    }
  },

  "shippable_first_cut": {
    "deliverables": [
      "pdfce-render: LoadedFont.substituted:bool → GlyphSource{Embedded,Bundled,Supplied}; substitute_face returns which, and load_simple threads it into LoadedFont.",
      "pdfce-render: named lookup in substitute_face also tries strip_subset_tag(base_font); document the exact precedence.",
      "pdfce-render: public `face_names(bytes)->Vec<String>` helper on the existing parser (R21) so the shell registers faces without a second parser.",
      "pdfce-render Diagnostics: add glyphs_supplied + supplied_fonts, distinct from glyphs_substituted/substituted_fonts; update merge() and the counting sites in interpret.rs and annot.rs.",
      "pdfce-cli: `--font-dir <DIR>` (repeatable) → walk, parse names, build FontEnvironment, pass via RenderOptions; render/inspect summary prints three-way disclosure (bundled count+names vs supplied count+names).",
      "pdfce-gui: 'Font folders' preference list (persisted in the R15 user-state partition, not among the replaceable payload); R20 diagnostics panel distinguishes bundled vs supplied; a visible indicator when supplied fonts are active.",
      "R59 gate: harness constructs a bundled-only FontEnvironment explicitly and ignores any ambient font-dir config; add an assertion that the gate is font-dir-independent.",
      "Docs: archive decision 012; file standing rules R61–R65; README font-supply + determinism-caveat copy; ARCHITECTURE §12 dated entry."
    ],
    "acceptance": [
      "A PDF referencing non-embedded 'Calibri' (simple TrueType, no program) renders with an operator-supplied Calibri.ttf when --font-dir points at it, and with bundled Helvetica when it does not; both are disclosed with the correct trust level and the actual face name; glyph POSITIONS are identical in both runs (from /Widths).",
      "'ABCDEF+Calibri' and 'Calibri,Bold' resolve to the supplied face when a matching supplied variant is registered; unmatched variants fall to bundled and are disclosed as bundled.",
      "A corrupt/oversized/misnamed supplied file fails clean: the page renders with bundled fallback, never errors; the shell notes the skipped file.",
      "Composite non-embedded still returns CompositeNotEmbedded; no supplied-composite code path exists.",
      "The R59 corpus gate yields byte-identical output regardless of any font-dir env/config/preference (R19 determinism preserved for the gate).",
      "cargo tree -p pdfce-render adds zero new packages; -p pdfce-core unchanged; wasm32 check green; no cfg(target_os) in core/render; cargo fmt --check and clippy -D warnings clean."
    ],
    "non_goals": [
      "Composite / CID (Type0) substitution via supplied fonts — named non-goal; FF2 only, strictly via the Unicode route, disclosed lossy.",
      "OS-font-directory enumeration — FF1, explicit opt-in, off by default, zero build-time dep.",
      "Descriptor-based AUTO-routing of unknown fonts to a supplied face — matching is by name only; manual slot mapping is FF3.",
      "Any shaping / GSUB / GPOS / bidi — R17, never in the render path.",
      "Identity-V, non-Identity CMaps — already deferred (unchanged)."
    ],
    "prereqs": [
      "None blocking. The FontEnvironment.named seam, the single skrifa parser, and the Diagnostics scaffold all already exist.",
      "Rule 13 (no copyleft font dep): folder mode adds NO dependency (std::fs + the existing skrifa parser). Nothing to classify. FF1's OS mode, if ever built, also needs no crate (a known OS path + std::fs); if a discovery crate is ever proposed it must be permissive-only and escalated per LEGAL §6."
    ],
    "risks": [
      "GlyphSource migration is wide but mechanical: LoadedFont construction sites (several test fixtures in text.rs), interpret.rs counting + merge, annot.rs, and the CLI/GUI display all touch it. Existing tests (lib.rs:484-485 assert bundled Helvetica) keep their meaning because bundled counters are unchanged.",
      "Name-match false positive: a supplied 'Arial.ttf' registered as 'Arial' shadows the bundled Helvetica substitute for every 'Arial' reference. This is DESIRED behavior, made safe by disclosure — the operator sees it became supplied.",
      "Determinism divergence: two machines with different folders render differently. Mitigated by R63 (gate runs bundled-only), the GUI 'supplied fonts active' indicator, and README copy. This is inherent to the feature, accepted and disclosed.",
      "A supplied file that parses but advertises a misleading internal name mis-matches. Low severity — disclosure always names the face actually used, so the operator can see and correct it.",
      "FF1 (OS mode) variant-selection ambiguity (which weight/style the OS face maps to) and the platform-path cfg — contained to the shell, and the reason OS mode is deferred rather than shipped in the first cut."
    ]
  },

  "proposed_standing_rules": {
    "R61": "Operator-supplied faces are SHELL-sourced, never renderer-discovered. The folder walk (and any future OS enumeration) lives in pdfce-gui/pdfce-cli; pdfce-render still only RECEIVES bytes through the FontEnvironment seam (R19 intact). No filesystem access and no cfg(target_os) enters pdfce-core or pdfce-render under this feature.",
    "R62": "Three glyph trust levels, always disclosed distinctly: Embedded (the document's own program, exact), Bundled (a Foxit Base-14 face, plausible), Supplied (an operator-supplied face, the operator's own shapes). Counted and surfaced separately in Diagnostics, the GUI panel, and the CLI summary. A supplied glyph is never presented as embedded; a bundled glyph is never presented as supplied.",
    "R63": "Supplied faces are OUTSIDE the determinism guarantee and the R59 corpus gate. The gate always constructs a bundled-only FontEnvironment; supplied-font renders are machine-dependent by definition, and the UI discloses when supplied fonts are active. R19's 'same input → same pixels' is scoped to the bundled set.",
    "R64": "Composite (CID) substitution, if ever added, goes STRICTLY via CID → (ToUnicode ▸ predefined CMap) → Unicode → supplied-face cmap → GID, is disclosed as lossy, and SKIPS (never GID-guesses) when no Unicode mapping exists. Identity-H without ToUnicode stays a hard skip permanently.",
    "R65": "OS-font-directory access is explicit opt-in, never default, and never a build-time dependency. The portable build ships zero external font dependencies; the bundled Foxit 14 are the deterministic floor with or without OS access."
  },

  "references": {
    "code": [
      "crates/pdfce-render/src/font/mod.rs (FontEnvironment, insert_named/named, RenderOptions)",
      "crates/pdfce-render/src/font/select.rs (by_name/by_descriptor/strip_subset_tag)",
      "crates/pdfce-render/src/font/program.rs (the one skrifa parser — R21 — where face_names() belongs)",
      "crates/pdfce-render/src/text.rs:334 (LoadedFont.substituted), :765 substitute_face, :626 load_composite (CompositeNotEmbedded)",
      "crates/pdfce-render/src/interpret.rs:199,225,452,492,1140,1300 (Diagnostics counters + merge + counting)"
    ],
    "decisions_and_rules": [
      "decision 003 §D2/R10/R11 (portable, platform-clean, wasm invariant) — the tension resolved here",
      "decision 004 §4.2/§5.3/§6.3 (the FontEnvironment seam built for exactly this), §3.6 (positions-from-widths), §7 (ToUnicode left in extraction), R17/R18/R19/R20/R21",
      "ROADMAP standing rules R19, R20, R59; CLAUDE.md rule 4 (fuzzy-never-sneaky), rule 13 (no copyleft font dep), GUI-core separation"
    ]
  }
}
```

## Appendix B — Engineer handoff notes

### Handoff notes for the engineer (not part of the archived record)

- The **one behavioral subtlety** to get right: `substitute_face` (`text.rs:773`) currently only tries `env.named(base_font)` with the verbatim string, so a supplied `Calibri` will miss `ABCDEF+Calibri` and `Calibri,Bold`. Add the `strip_subset_tag` retry, and have the shell register each face under its internal name *and* filename stem.
- The **widest mechanical change** is `substituted: bool → GlyphSource`. Existing tests (`lib.rs:484-485`, the `substituted: false` fixtures in `text.rs`) keep their meaning because the *bundled* counters are unchanged — only the *supplied* level is new. Map old `substituted:false` → `GlyphSource::Embedded`, old bundled path → `GlyphSource::Bundled`.
- `load_composite` (`text.rs:657`) stays exactly as is — do **not** wire `env` into it this pass. That is the R64/FF2 boundary.
- Per the roadmap-discipline rule, dispatch `pdfce-acrobat-librarian` before scoping this into a Pass (Acrobat's "add fonts" behavior for non-embedded text is the parity reference), then `pdfce-librarian` to file R61–R65 and the ROADMAP/ARCHITECTURE entries. Nothing here is blocked on the license decision.

## Orchestrator note (2026-08-01, at archival)

Decision 012 archived (operator-supplied fonts). Requested by the operator after a real drawing rendered with missing text; the root-cause font bug (subset CIDFontType2 no-cmap TrueType misroute) was separately fixed. Outcome: folder-based supply for non-embedded SIMPLE fonts via the existing FontEnvironment seam (decision 004 §5.3 built it for this); three glyph trust levels disclosed distinctly (Embedded/Bundled/Supplied); renderer stays bytes-in so portability/determinism/wasm invariants (decision 003 R10/R11/R19) hold; OS-font-directory access is explicit opt-in fast-follow (FF1), composite/CID substitution deferred (FF2, Unicode-route-only), descriptor auto-routing deferred (FF3). Adds standing rules R61–R65. Zero new dependencies. STATUS at archival: the design was shown to the operator for confirmation before building; the ROADMAP standing-rules filing (R61–R65) + the implementation await the operator's go-ahead. Also owed per the decision's handoff note: dispatch pdfce-acrobat-librarian for Acrobat's 'add fonts / substitute non-embedded' parity behavior before scoping the implementation Pass.
