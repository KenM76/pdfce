# 021 — FF-C: font subsetting and glyph embedding

**Date:** 2026-08-03
**Status:** Decided — **not started**
**Decided by:** KenAgent (`autonomous-builder`, decision-consultant mode)
**Requested by:** `pdfce-engineer` (operator priority #3, *"finish off all the
text handling stuff"*; decision 019 §3.8 build order FF-H → **FF-C** → FF-B,
with FF-H complete as of `a1638f4`)
**Proposed Pass family:** **Pass 21.x** — see the numbering correction below.

---

## NUMBERING CORRECTION — applied by the engineer before filing

This record was produced against a `docs/ROADMAP.md` that was **stale by the
time it was written**: three librarian filings landed on the same day while
the scoping ran. Two of its numbering claims were wrong and are corrected
here rather than filed and patched later.

| It claimed | Live value at filing | Corrected to |
|---|---|---|
| Standing-rule ceiling **R99**, so new rules R100–R103 | ceiling **R106** — R100–R105 were taken by decision 020's renumbering, R106 by the Pass-18.7 correction | **R107–R110** |
| **Pass 20.x** is free (*"no occurrence of 'Pass 20' anywhere in the file"*) | decision 020 claims **Pass 20.0–20.7** in ROADMAP's Backlog prose | **Pass 21.x** |
| Next decision number **021** | correct | 021 |

Both were caught by `tools/check-ledger-numbers.py`, which is the tool
standing rule R106 called for and which shipped hours earlier at `4dc8cf8`.
**It caught the rule collision immediately and missed the Pass collision** —
its ceiling report scanned only `### Pass N` headings, and decision 020 had
claimed Pass 20.x in prose with no heading yet, so it reported "highest Pass
family: 19", which was true and useless. That blind spot is now fixed: the
tool reports mentioned-but-unheaded families by name. The scoping agent's
error and the tool's were **the same error** — reading only what is visible
as a heading — which is why the fix belongs in the tool and not only in a
reminder to be careful.

The §10.3 recommendation this record makes — *"FF-C takes Pass 20.x and the
librarian amends decision 020's header to 21.x"* — is therefore **declined**.
It was premised on Pass 20.x being unclaimed. It is claimed. FF-C takes 21.x,
decision 020 keeps 20.x, and the chronological scheme is preserved without
renumbering already-filed work.

---

**Amends:**

- `D:\Dev\Rag-Specialized\PDF_Spec\fonts\font__subsetting_ffc_queue.md` —
  **its central premise is wrong** and it must be rewritten before it grounds
  any code (§1.2, §4.2).
- `docs/decisions/012-operator-supplied-fonts.md` §6 — *"The write side (font
  embedding/subsetting) — unrelated"* is now false.
- `ROADMAP.md` **R21** — scope note, discharging R21's own *"No second parser
  enters without a new decision record"* clause (§3.2).
- `ROADMAP.md` **R71** — FF-C ceases to be "a deferred writer subsystem".
- `ROADMAP.md` **R79** — "no embedding" becomes "no embedding **by default**".
- `docs/PRIOR_ART.md` "FF-C dependency classification" — refines the net-cost
  figure from 2 packages to **1** (§3.2).

**Builds on:** decision 004 (skrifa as the single read-path parser, R17–R22) ·
decision 012 (`GlyphSource`, `FontEnvironment` supply seam) · decision 014
(R-INV-1..8) · decision 016 (`addtext.rs` append envelope, R78–R79) ·
decision 019 (R88–R91 ambient text state) · Pass 3.x writer +
`signature::impact_of` · Pass 8.0/8.1

**Does not touch:** R35/R58/R67 (the forced-full-rewrite family) · R65
(`Identity-H` without `ToUnicode` stays a hard skip) · R17 (no shaping, ever) ·
the Pass-6.2 FreeText path

---

## 1. Context

### 1.1 The gate is clear; the remaining question is scope

The licence question was not re-litigated. The *resolution* was re-run,
because `PRIOR_ART.md`'s own closing note says the version-unification
finding *"is a property of **this** workspace's resolved graph rather than of
any crate on its own"* — and that property changes when a feature flag
changes. It does. See §3.2.

### 1.2 THE HEADLINE — FF-C as filed is not implementable

`ROADMAP.md`, `R71`, decision 014 §5.3 and the spec-RAG queue stub all
describe FF-C as the Pass that **lifts the embedded-subset refusal** —
R-INV-1: *"the run's embedded font is a subset and lacks the glyph you just
typed."* The queue stub is explicit about the mechanism:

> | TrueType tables | `glyf`/`loca`/`hmtx`/`cmap`/`post`/`head`/`maxp` | **Add outline to `glyf`, fix `loca` offsets, `hmtx`, `maxp.numGlyphs`** |

**Add *which* outline?** A subset font, by definition, does not contain the
glyph you want. The bytes are not in the file. There is no operation on
`FontFile2` alone that produces them.

The only sources of that outline are (a) a donor face on the operator's disk,
or (b) nothing. So the capability described in three places as "FF-C" is, as
described, **not implementable** — not because it is hard, but because the
input does not exist. Every downstream plan that assumed "FF-C lifts R-INV-1"
inherited that error.

**Engineer verification (2026-08-03).** Both halves checked directly rather
than relayed: `font__subsetting_ffc_queue.md:42` does contain the quoted
"Add outline to `glyf`…" row, and `subsetter 0.2.6`'s `src/lib.rs:20–21`
does state *"You must write your fonts as a CID font. This is because we
remove the `cmap` table from the font."* The reframe and the structural
forcing in §3.4 both hold.

Scoping FF-C by "what does a subsetter do" would have shipped a subsetter
that lifts nothing.

---

## 2. Options considered

**What FF-C is.** W1 — extend the document's own embedded font in place (the
stub's framing). **W2 — add a new, subsetted font resource from a donor face;
never touch existing fonts.** W3 — replace the document's embedded font with
a re-subset donor.

**Crate boundary.** B1 — `subsetter` in `pdfce-core`. **B2 — font-program
work in `pdfce-render`, PDF-object emission in `pdfce-core`.** B3 — a new
`pdfce-font` crate.

**Dependency.** D1 — `subsetter`, default features. **D2 — `subsetter`,
`default-features = false`.** D3 — `allsorts`. D4 — hand-rolled.

**Emitted font model.** M1 — simple font. **M2 — `/Type0` + `Identity-H` +
CIDFont.**

**Disclosure.** P1 — embed silently when it would help. **P2 — an explicit
per-action operator choice, offered at the point of refusal, with the real
outcome computed first.**

---

## 3. Decision

### 3.1 Q1 — what FF-C is for: **W2**, and the refusal set is the scope

**FF-C adds new, subsetted font resources from a donor face. It never
modifies an existing font program or font dictionary.**

**W1 is rejected as structurally impossible** (§1.2). **W3 is rejected**
because it buys nothing W2 does not: if you have a donor good enough to
re-subset, you have a donor good enough to embed as a *new* resource — and W2
does it without rewriting an object pdfce did not author. W3 trades the
round-trip invariant for zero capability, and is silently destructive in a way
W2 is not: a re-subset donor replaces the shapes of *every already-correct
glyph in that font*, document-wide, with a physically different font file that
merely claims the same name.

#### Refusals FF-C **lifts**

| # | Refusal | Site | How FF-C lifts it | Slice |
|---|---|---|---|---|
| **L1** | `AddTextError::Refused` — a character outside the chosen Standard-14 face's repertoire | `addtext.rs`, via `InverseEncoding`; R71/R79 | The operator may embed a supplied face that covers it. **The headline: pdfce today cannot add *any* text outside WinAnsi/Symbol/ZapfDingbats — no Greek, Cyrillic, CJK, Hebrew or Devanagari, at all.** | **21.0** |
| **L2** | `FormatError::TargetFontMissing` | `format.rs:1020`, raised `:1822`; GUI hint `ui_text.rs` names FF-C verbatim | `set-font` may name a supplied face | 21.2 |
| **L3** | `FormatError::CoverageFailure` | `format.rs`; GUI hint *"…or supply one via Tools › Font folders"* | The hint becomes true end-to-end | 21.2 |
| **L4** | *(not a refusal)* `FormatError::RealFaceAvailable` / synthetic bold-italic | decision 019 R90 | Decision 019's own revisit trigger fires | 21.2 |

L3 deserves emphasis: `format_coverage_hint()` currently tells the operator to
supply a font, and supplying one **does not fix the saved document** —
decision 012 is a read-side feature, and `addtext.rs`'s own header says pdfce
*"writes an identical named non-embedded dict either way."* **That hint is
today a promise the write path does not keep.** FF-C makes it honest.

#### Refusals that **survive** and must keep firing

| # | Refusal | Why it survives |
|---|---|---|
| **S1** | **R-INV-1** literal form | FF-C does not extend the document's font; it offers a *different* remedy the operator may decline or lack a donor for. **Narrowed from wall to wall-with-a-door, not deleted.** |
| **S2** | **R-INV-2** — symbolic, no usable `/Encoding` | A read-side inverse-mapping problem. Embedding a new font says nothing about what the old codes meant. |
| **S3** | **R-INV-3** — `/ToUnicode` only | Still not invertible. |
| **S4** | **R-INV-4** — composite run | Conditionally lifted **only** by 21.1, and only where `/ToUnicode` is *verifiably* injective. `Identity-H` with absent or ambiguous `ToUnicode` stays a hard refusal — **R65 untouched.** |
| **S5** | Coverage refusal when the *supplied* face genuinely lacks the glyph | The refusal moves; it does not disappear. |
| **S6** | `UnsupportedFont::CompositeNotEmbedded` (read side) | Decision 012 FF2 territory. |

#### New refusals FF-C introduces (all reachable, all owed a firing test — R96)

`FontEmbedError::` — `DonorNotSfnt` (bare Type 1 donor; `subsetter` is
sfnt-only → `Err(UnknownKind)`) · `DonorUnsupported` (CFF2 without
`variable-fonts` → `Err(Unimplemented)`; **also, per §10 C-3, any CFF-outline
donor at P0** — `subsetter`'s CFF output is `OTTO`-wrapped and cannot be
conformantly emitted as either `/FontFile3 /Subtype /OpenType` (Table 126
requires `cmap`, which `subsetter` strips) or `/CIDFontType0C` (which
requires a bare CFF program, not an OTTO container); narrowed to `glyf`
donors at 21.0, lifted in a later slice) · `DonorMalformed` · `DonorTooLarge`
(§3.5) · **`SubsettingNotPermitted` / `EmbeddingNotPermitted`** (§3.6, §10
C-7 — split into two distinct fsType refusals, not one) · `CoverageIncomplete`.

### 3.2 Q2 — crate boundary: **B2**, and it does **not** threaten the invariants

**Verified dependency direction** (from the four `Cargo.toml`s, not assumed):
`pdfce-core` ← `pdfce-render` ← {`pdfce-gui`, `pdfce-cli`}. `pdfce-core` has
**no** `pdfce-render` dependency, and `pdfce-core/src/fontdata/` is **metrics
only** — compiled AFM widths, Annex D encodings, AGL — with **no font-program
parsing anywhere in the crate**. The parser is
`pdfce-render/src/font/program.rs` (skrifa).

**Decision: `subsetter` goes in `pdfce-render`. `pdfce-core` gains zero new
dependencies.**

- `pdfce-render::font` gains a **`SubsetPlan` producer**: parse the donor
  (existing skrifa), map required characters → GIDs via the donor's `cmap`,
  read `hmtx` advances, `head.unitsPerEm`, descriptor metrics and the
  embedding-permission bits, call `subsetter::subset`, return plain data.
- `pdfce-core::font_embed` **defines** that plain-data type and **consumes**
  it: emit `/Type0` + `/CIDFontType2`(or `0`) + `/FontDescriptor` +
  `/FontFile2`(or `3`) + `/W` + `/CIDToGIDMap /Identity` + `/ToUnicode`, and
  wire the page `/Resources`. That is PDF writing, and belongs in core.

**B1 is rejected.** It contradicts decision 004's own words — *"the sole
read-path font parser for `pdfce-render` (**never `pdfce-core`**)"* — and buys
nothing, because the coverage lookup, advances, descriptor metrics and
permission bits **must be parsed anyway**, and that parse already lives in
`pdfce-render`. B1 would put two parsers in core to avoid a data seam.

**B3 is rejected for now** as a real refactor (moving `program.rs` changes
`pdfce-render`'s public surface) with no invariant benefit today. Named as a
revisit trigger if a second consumer appears.

**Does this contradict R21?** No, on three independent grounds — all three on
record, because the first alone would be lawyering:

1. **R21's own title and text scope it to the read path** — *"One font parser
   in the **read path**."* Subsetting is the write path.
2. **R21 contains its own escape clause** — *"No second parser enters without
   a new decision record."* This is that record. Discharged, not evaded.
3. **The spirit is preserved.** R21 exists so that what the operator *sees*
   comes from one parser. `subsetter`'s internal reader never renders a glyph,
   never chooses a substitute, never reaches `Diagnostics`. It reads to
   rewrite, and its output is re-read by skrifa if ever displayed.

R21's mechanical guard — *"`cargo tree --duplicates` must never show two
`skrifa` or `read-fonts` majors"* — is **strengthened** by the feature choice
below, not weakened.

**GUI-core separation: no risk.** The resolved graph is font and proc-macro
crates only; no windowing crate is reachable. The existing
`cargo tree -p pdfce-core -p pdfce-render` gate covers it.

**wasm32 / R11: no risk.** `subsetter` is pure Rust, `#![deny(unsafe_code)]`,
std-only, no filesystem/network/time/`cfg(target_os)`; MSRV 1.85 against
pdfce's 1.92. CI's existing wasm32 cross-check covers it unchanged.

#### The dependency, re-verified — and the number changes

`subsetter 0.2.6`'s manifest, read from the vendored registry copy:

```
license = "MIT OR Apache-2.0"          rust-version = "1.85"
default        = ["variable-fonts"]
variable-fonts = ["dep:skrifa", "dep:write-fonts", "dep:kurbo"]
[dependencies] rustc-hash = "2.1"      # the ONLY non-optional dependency
```

`variable-fonts` exists solely to instance variable fonts at non-default axis
coordinates. pdfce does not need it at P0.

**Decision: `default-features = false`** — the same R24 call `pdfce-core`
already applies to `zune-jpeg`, `hayro-ccitt`, `hayro-jbig2` and
`hayro-jpeg2000`.

Resolved live via `cargo metadata --offline` on a scratch crate carrying
`subsetter` (features off) **plus** `skrifa = "0.42"`, to prove co-resolution
rather than assert it — 11 packages, every one permissive, single
`read-fonts 0.39.2` / single `font-types 0.11.3` / single `skrifa 0.42.1`,
exactly pdfce's pinned set. No GPL/LGPL/AGPL/MPL. `LEGAL.md` §6.2 step 3:
proceed and log.

**Net cost, checked against pdfce's actual `Cargo.lock`:** `rustc-hash 2.1.3`
is already present (via `type-map` ← eframe); `write-fonts` is not.

| Configuration | New to the workspace lock | New to `pdfce-render`'s graph |
|---|---|---|
| default features | `subsetter` + `write-fonts` = **2** | 4 |
| **`default-features = false`** | **`subsetter` only = 1** | 2 |

`PRIOR_ART.md`'s "2 packages" is **correct for default features** and stays on
record as what a naive add costs. Turning the feature off halves it and makes
the version-unification hazard *moot for this dependency* — while leaving the
hazard itself real and the version map worth keeping, because it is still
exactly what bites anyone reaching for `write-fonts` directly.

**D3 (`allsorts`) rejected:** Apache-2.0 with no MIT arm, ~23 direct deps, no
`no_std` (decision 004 flagged the R11 exposure). Retained as the named
fallback if `subsetter` is abandoned.
**D4 (hand-rolled) rejected:** `glyf` closure + `loca` format selection + CFF
charstring and subroutine subsetting + SID→CID conversion is precisely the
*"massive undertaking"* `subsetter`'s own docs describe — and the spec RAG
carries a **named GAP** for OpenType and CFF table structure, so building it
would first require ingesting two external specs.

**Named trap:** the crate published as **`klippa` on crates.io is a geometry
clipper**, not fontations' subsetter. `subsetter`'s own docs point at
**`skera`** as the intended future general-purpose Rust subsetter — not yet
published; a revisit trigger, not an option.

### 3.3 Q3 — round-trip: **the exception is not needed, because FF-C never earns one**

**Binding rule (proposed R107): FF-C never writes to an existing `/FontFile`,
`/FontFile2`, `/FontFile3`, `/FontDescriptor`, `/Font` dictionary, or CIDFont
dictionary. It only ever allocates new objects.**

The question asks *when* FF-C legitimately touches an existing font stream.
**The answer is: never, in this family.** Not caution — the observation that
the add-only path delivers every lifted refusal in §3.1 at no invariant cost,
so there is nothing to trade for.

- **§5 needs no new exception.** The objects FF-C modifies are exactly those
  `addtext.rs` already modifies: the page dict (`/Contents` reference and/or
  `/Resources`), plus new objects. `addtext.rs`'s "five objects touched" table
  grows three rows — the CIDFont dict, the `FontFile2` stream, the
  `/ToUnicode` CMap — **all new**. Original content streams stay
  byte-identical (R32/R46).
- **Incremental save stays the default** (R36/R70). FF-C is a content
  *change*, not a removal — it joins neither R35's nor R58's nor R67's
  forced-full-rewrite family.
- **Signatures.** Reuse the shipped `pdfce_core::signature::{census,
  impact_of}`. Adding a font resource to a page's `/Resources` is the same
  class of page-dict change `add-text` already makes; FF-C passes the same
  `structural` assertion and inherits `SignatureImpact` unchanged. **FF-C
  introduces no new signature semantics — precisely *because* it does not
  rewrite an existing font stream.** W3 would have: replacing a font program a
  certification signature covers is a visible change to signed content.
- **The `/Resources` inheritance trap** (§7.7.3.4) is already solved in
  `addtext.rs` — give the page its own `/Resources` referencing the same
  indirect sub-dicts except a freshly merged `/Font`. Reuse verbatim.
- **The `ObjectWrite` correction** (`ARCHITECTURE.md` §11.1, 2026-08-03): a
  command touching the same page object's `/Contents` **and** `/Resources`
  must accumulate **one merged dictionary write per object id**. FF-C's
  add-text-with-embedding path is exactly that combination — the
  `flatten_fields` bug's shape, and the entry says other multi-write commands
  are owed the same audit. **FF-C is one of them. Audit it in 21.0.**

**Does R107 need a runtime refusal? No — and adding one would violate R96.**
If the emitter can only allocate fresh object ids, a guard asking "am I about
to rewrite an existing font?" sits after a filter the guarded case cannot
pass: dead code that looks live. The correct discipline is R97's — extract the
proof to a free function over data and **assert it**: a corpus test that the
set of object ids a `FontEmbedPlan` modifies intersects the set of
pre-existing font object ids in **∅**. That test can fail if someone later
"optimizes" the emitter; an unreachable guard cannot.

### 3.4 Q4 — slice plan and the P0 floor

**Spec-review correction (2026-08-03) — read before the emitted-table list
below, which predates it.** §10 C-3 found that `subsetter`'s CFF output
cannot be emitted conformantly under this section's original M2 plan (it
wraps CFF donors in an `OTTO` sfnt that Table 126 requires a `cmap` for,
and `subsetter` strips `cmap` unconditionally). **Pass 21.0's P0 floor is
narrowed to `glyf` (TrueType-outline) donors only; CFF donors are refused
by name (`DonorUnsupported`) until a later slice.** Full reasoning, and why
L1 survives intact despite the narrowing, in §10 C-3.

**The forced structural fact.** From `subsetter`'s own `lib.rs`, verified at
source by the engineer:

> **You must write your fonts as a CID font.** This is because we remove the
> `cmap` table from the font, so you must provide your own cmap table in the
> PDF.

The emitted table set confirms it: `GLYF`+`LOCA` (or `CFF`), `HEAD`, `HMTX`,
`MAXP`, `NAME`, `POST` — **`CMAP` and `OS/2` deliberately excluded.**

This settles the TrueType-vs-CFF-vs-Type0 tension: **`subsetter` absorbs the
TrueType/CFF split entirely** (glyf/loca closure *and* CFF charstring
subsetting *and* SID→CID conversion) and **forces** `/Type0` + `Identity-H` —
which is the case real-world PDFs actually use. pdfce writes no charstring
subsetter and makes no simple-vs-composite choice. **M2 by construction, not
preference.** M1 is rejected: hand-writing a `cmap` back in would re-open the
write-side-parser question and cap the result at 256 codes, defeating the
non-Latin case that motivates FF-C.

**The trap this creates.** A `/Type0` run is exactly what R-INV-4 refuses to
edit. Left alone, 21.0 would let pdfce *add* Japanese text it can never
afterwards *edit* — a capability regression against the shipped Std-14
add-text path, whose milestone entry boasts the run is *"immediately
editable/formattable/reflowable through the existing 14.x/15.x pipeline."*
**Shipping the adder without the editor is shipping a trap.**

The fix is not a pdfce-only special case. R-INV-4's stated blocker is *"no
Unicode→CID map without inverting `/ToUnicode` (lossy)."* Inverting is lossy
**when it is not injective** — and injectivity is a **checkable property of
the data**, not an assumption from authorship (R93: authorship is a claim, the
check is evidence). A pdfce-authored CMap is injective by construction *and
verified anyway*; plenty of real-world files qualify too, so the lift is
general.

| Slice | Scope | Lifts |
|---|---|---|
| **21.0** — core + CLI. **P0 floor.** | `pdfce-render::font::SubsetPlan` producer; `pdfce-core::font_embed` emitter; wire into `addtext.rs` (`base_font: Std14` → `NewTextFace { Std14 \| Embedded }`); §10 guards; fuzz target; fsType read; disclosure. CLI: `add-text --embed-font`. | **L1** |
| **21.1** — core + CLI | Composite-run edit/format where `/ToUnicode` is **verified injective**; conditional R-INV-4 lift. Makes 21.0's output editable. | **S4 (conditional)** |
| **21.2** — core + CLI | `set-font` to a newly embedded face; `format.rs` composite-target emission. | **L2, L3, L4** |
| **21.3** — GUI. **Final slice.** | Face picker over `--font-dir` faces; refusal→remedy flow; embed confirmation showing the *real* subset size and coverage; trust and licence disclosure. **Dispatch `pdfce-ui-specialist` first.** | (surfaces the above) |

**Why 21.0 is the smallest slice that lifts a real refusal:** `add-text` is
already a pure-append path (R78/R79), so the whole round-trip surface is
already solved and tested there; and L1 is the widest wall in the product.
Doing L2/L3 first would mean teaching `format.rs` composite emission before
anything can use it.

**Rule 11 note:** `add-text` already exists as a CLI subcommand, so 21.0
extends its flags rather than adding a subcommand — the same reasoning Pass
19.3 and Pass 8.1 recorded explicitly. **State it that way at ship**, so the
absence of a *new* subcommand reads as reasoned rather than missed.

**Gates every slice:** `cargo test --workspace` (measured baseline, not quoted
— R87) · fmt + `clippy -D warnings` · `tools/check-ui-strings.sh` ·
`tools/check-ledger-numbers.py` · `cargo tree -p pdfce-core -p pdfce-render`
GUI-dep-free · wasm32 cross-check · R85 preview-equals-saved N/N ·
`tools/roundtrip` corpus · veraPDF §6.1.12 for any new guard · `cargo-about`
regeneration (the dependency set changes at 21.0).

### 3.5 Q5 — hostile input

A donor face is untrusted **even when the operator supplied it**. Font files
are a classic exploit vector; "the operator chose it" is not a trust argument.

1. **`MAX_FONT_PROGRAM_BYTES`, on the *input*.** §10.1's ceilings are
   output-side because decoders expand; `subsetter` only ever *removes*, so
   `len(output) ≤ len(input)` and the input is the correct place.
   **Corpus-measure it, do not guess** — the three recorded guard-by-intuition
   failures on this project (`MAX_TOKEN_LEN` 8 KiB, `MAX_XOBJECT_DEPTH` 16,
   `jpx::MAX_TILES`) are the precedent. Measure the largest embedded font
   program across the 2,914-file corpus **and** a realistic supplied-face
   ceiling (a CJK `.ttc` can exceed 100 MB).
2. **Composite-glyph cycles: a *test*, not a guard.** Verified at source
   (`subsetter/src/glyf.rs`): `closure()` is an **iterative worklist**, not
   recursion, and pushes a component only when
   `glyph_remapper.get(component).is_none()`. The remapped set grows
   monotonically, bounded by `numGlyphs`, so self-referential and mutually
   referential composites **terminate and cannot stack-overflow** —
   structurally, upstream. A pdfce-side depth cap would be exactly R96's
   defect. **What is owed instead is a synthetic fixture** — a
   self-referencing composite and a two-glyph cycle — asserting bounded time
   and a clean result. That test fails if upstream ever rewrites `closure()`
   recursively; a redundant guard could not.
3. **A `cargo-fuzz` target — owed.** `fuzz/fuzz_targets/font_subset.rs`,
   matching the existing per-codec pattern: arbitrary bytes + arbitrary GID
   set → `subset()`, asserting never panics, never hangs past a bounded
   timeout, never allocates past a bounded ceiling. `subsetter`'s
   `#![deny(unsafe_code)]` means a finding is DoS, not memory corruption —
   still a release blocker per §10.2.
4. **`.ttc` collection index validation.** pdfce validates `index < numFonts`
   before calling, and passes `0` unless the operator chose a face from a
   collection.
5. **Error mapping is per-cause, never generic (R27).** `subsetter::Error`'s
   six variants map to distinct named diagnostics. `SubsetError` and
   `OverflowError` are documented upstream as *"indicates a logical bug in the
   subsetter"* — those get a diagnostic that says so, and an upstream report
   if ever seen on a real face.
6. **Where the guards live:** `pdfce-render`, per §10.3's rule that a guard
   lives in whichever crate actually performs the operation being guarded —
   the same reasoning that placed `MAX_XOBJECT_DEPTH` there.

### 3.6 Q6 — fuzzy-never-sneaky: **P2**

**A correction to the question's framing first.** The lossiness named — *"the
embedded font no longer contains glyphs the document does not use, so a later
edit may fail"* — **is already R-INV-1**, which already exists, already fires
by name, and already carries operator-facing copy. FF-C does not create a new
undisclosed trap; it creates a new *instance* of an existing, disclosed one. A
second disclosure would be noise. **What FF-C does owe is that the R-INV-1
message names an FF-C-authored font as pdfce-authored**, so the operator knows
the remedy is "re-embed with wider coverage", not "your document was always
like this".

**Embedding is never automatic (P1 rejected).** It changes file size and
creates font-redistribution exposure. `add-text` today silently defaults to
Helvetica; FF-C must not silently upgrade that to "embed whatever you
supplied".

**The shape (proposed R108), which is R98 applied.** R98 says a confirmation
for an irreversible-in-effect operation should compute and disclose the
**real** outcome whenever the operation is pure. **Subsetting is pure.** So
the offer is not *"embedding a font will increase file size"* — it is
**"Embed a subset of Noto Sans JP: 14 glyphs, 11.2 KB added. All 14 characters
covered. Licence: embedding permitted (installable)."** Computed, shown, then
confirmed.

**Three disclosures FF-C genuinely adds:**

1. **The trust-level flip, stated accurately.** After FF-C the run's glyphs
   are `GlyphSource::Embedded` — the document's own program for every reader,
   and **outside R64's determinism exclusion**, because the bytes now travel
   with the file. That is the real win. But it must not be oversold: pdfce
   embedded a subset of *the face the operator supplied*. If that face is not
   what the document's author used, the shapes differ from the original
   design — the write-side echo of decision 012's *"supplied improves shapes,
   not positions"*. R62/R63 need a write-side **sentence**, not a fourth level.
2. **Font-embedding permission.** Embedding redistributes someone else's font.
   This is a **font EULA** question, entirely separate from rule 13 (which is
   about *code* licences) and from pdfce's own MIT. OpenType encodes the
   author's intent in `OS/2`'s `fsType` — and **`subsetter` strips `OS/2`**,
   so pdfce must read it from the donor *before* subsetting. **The bit
   semantics are deliberately not stated here**: they are claim-bearing, they
   govern a refuse-or-proceed decision, and per rule 1 must be sourced from
   the OpenType specification by `pdfce-spec-librarian`, never recalled. The
   *policy* is Ken's (§7). **Update (2026-08-03): the bit semantics are now
   sourced — see §10 C-7/the fsType open-question narrowing below.** Bit 8
   (No subsetting) and bit 9 (Bitmap embedding only / "unembeddable") are
   *distinct* refusals, not one `EmbeddingNotPermitted` — R109 amended
   accordingly.
3. **Re-editability**, for 21.0 shipped before 21.1: an FF-C-authored run is
   composite; say so, and say that in-place editing of it arrives in 21.1.

---

## 4. What this decision produces

### 4.1 Proposed standing rules — **R107–R110** (corrected; the librarian confirms)

- **R107 — FF-C only ever ADDS font resources; it never modifies an existing
  font program or font dictionary.** Embedding allocates a new `/Type0` dict,
  CIDFont dict, `FontFile*` stream and `/ToUnicode` CMap, and merges one entry
  into the page's `/Font` sub-dict via the `addtext.rs` reference-not-mutate
  path. No `/FontFile`, `/FontFile2`, `/FontFile3`, `/FontDescriptor`, or
  existing `/Font`/CIDFont dictionary is ever rewritten. This keeps §5
  exception-free for the whole family, keeps incremental save the default
  (R70), and keeps FF-C out of the R35/R58/R67 forced-full-rewrite family.
  Enforced by an **object-id-disjointness test** (R97 shape), **not** by a
  runtime guard — a guard in an emitter that can only allocate fresh ids is
  unreachable by construction and would be exactly R96's dead code that looks
  live.
- **R108 — Embedding is an explicit, per-action operator choice whose real
  outcome is computed before confirmation.** Never a default, never a global
  preference, never a silent upgrade of the R79 no-embed path. Offered at the
  point where the no-embed path would refuse, so the refusal becomes an
  actionable remedy. Because subsetting is pure, the confirmation shows the
  **actual** subset byte count and the **actual** covered/uncovered character
  list (R98), never an estimate.
- **R109 — Font-embedding permission is read from the donor face and
  disclosed; never assumed, never guessed. AMENDED 2026-08-03 (§10 C-7):
  fsType is not one gate — the refusal is TWO distinct named diagnostics,
  not one `EmbeddingNotPermitted`.** Read before subsetting (`subsetter`
  strips `OS/2`). **`SubsettingNotPermitted`** — bit 8, `0x0100`, "No
  subsetting": permits embedding the whole face but forbids the one thing
  FF-C ever does. **`EmbeddingNotPermitted`** — bit 9, `0x0200`, "Bitmap
  embedding only," the specification's own "unembeddable" case for
  outline-program embedding. Absent or unparseable permission data is
  disclosed as unknown and follows the operator policy of §7 — pdfce never
  silently treats "no data" as "permitted," and never treats it as bit-0
  `0x0000` (*Installable*) either, since Installable is the **most**
  permissive value, not a safe stand-in for missing data. Bit semantics
  sourced from the OpenType specification via `pdfce-spec-librarian`, never
  from recall.
- **R110 — A composite run is editable only where its `/ToUnicode` is VERIFIED
  injective, per font, per session.** Injectivity — every CID maps to exactly
  one scalar, and no two CIDs map to the same scalar — is what makes the
  inverse a function and conditionally lifts R-INV-4. **Checked against the
  data**, never inferred from the fact that pdfce authored the font (R93).
  Non-injective, absent or partial `/ToUnicode` keeps refusing. `Identity-H`
  with no `/ToUnicode` remains a permanent hard skip — **R65 untouched.**

### 4.2 Amendments owed

| Target | Change |
|---|---|
| **R21** | Scope note: R21 governs the **read path**; `subsetter`'s internal write-side reader in `pdfce-render` is admitted by this record, discharging R21's own "no second parser without a new decision record" clause. The `cargo tree --duplicates` guard is unchanged and verified to hold. |
| **R71** | *"FF-C is a deferred writer subsystem"* → scoped as Pass 21.x. The trust ladder gains a fourth rung: **refuse → offer embed (R108) → embed on accept**. R-INV-1's wording gains the remedy pointer. |
| **R79** | *"no embedding"* → *"no embedding **by default**"*. |
| **decision 012 §6** | *"The write side — unrelated"* is now false. FF-C is the write-side consumer of decision 012's supply mechanism; `--font-dir` is FF-C's donor source. |
| **`font__subsetting_ffc_queue.md`** | **Rewrite, do not extend.** Dispatch `pdfce-spec-librarian` for **§9.8.3 Table 124** `/CIDSet` (corrected 2026-08-03; was miscited as §9.7.4.2) · §9.7.4.3 `/W`/`/DW`/`/CIDToGIDMap` · **§9.6.4** subset tag (`ABCDEF+`) (corrected 2026-08-03; was miscited as §9.8.1, which has no subset rule) · §9.9 Table 126 `FontFile3` `/Subtype` selection · §9.10.3 `/ToUnicode` CMap authoring · **OpenType `OS/2` `fsType`** (R109's dependency — sourced 2026-08-03, see §10 C-7). |
| **`PRIOR_ART.md`** | Net-cost refinement (1 package at `default-features = false`, 2 at default) plus the feature-flag finding. |

### 4.3 Code changes (21.0)

`pdfce-render/Cargo.toml`: `subsetter = { version = "0.2", default-features =
false }` with an R24-style load-bearing comment ·
`pdfce-render/src/font/subset.rs` (new) · `pdfce-core/src/font_embed.rs` (new)
· `pdfce-core/src/text_edit/addtext.rs`: `base_font: Std14` → `face:
NewTextFace`, plus the **merged-single-`ObjectWrite`** audit (§3.3) ·
`pdfce-cli`: `add-text --embed-font` · `fuzz/fuzz_targets/font_subset.rs` ·
`THIRD_PARTY_LICENSES.md` regenerated via `cargo-about`, never hand-edited.

---

## 5. Risks to the two load-bearing invariants

**GUI-core separation — LOW, verified.** Resolved graph is 11 crates, all
font/proc-macro, no windowing. `pdfce-core` gains nothing. Residual: someone
later enabling `variable-fonts` pulls `write-fonts`, fine at 0.48 and
**breaking the pin at 0.51** (→ `read-fonts 0.42.1`, two parsers). The
existing `cargo tree --duplicates` gate catches it; the Cargo.toml comment
must say why the feature is off, in R24's voice.

**Round-trip — LOW, *because of* R107.** The residual is a temptation, not a
mechanism: in 21.2 (`set-font`), "just widen the existing font" will look
cheaper than "add a second resource". The object-id-disjointness test is the
guard-rail, and **it must be written in 21.0** — before the temptation
exists — not in 21.2.

**A third risk, named because it is the likeliest way this goes wrong:**
shipping 21.0 without 21.1 and calling FF-C done. That produces a product that
adds text it cannot edit — a *regression* against Std-14 add-text — while
every counter reports success. That is the `flatten_fields` failure shape
(correct counters, wrong artifact) that R85 was created to catch, and R85
covers `add-text`, so the raster oracle **will** show the glyphs; it will
**not** show that they are uneditable. This needs a deliberate acceptance
criterion, not a gate.

---

## 6. Honest limits — name these up front

- Variable-font donors embed at their **default instance** (no axis instancing
  with `variable-fonts` off) — pick a static face for a specific weight.
- Bare Type 1 (`.pfb`/`.pfa`) donors cannot be subsetted (`subsetter` is
  sfnt-only).
- CFF2 donors are `Unimplemented`.
- The emitted font is **PDF-only** — `subsetter`'s docs state the output
  *"will most likely be unusable in any other context than PDF writing"*. A
  font extracted from a pdfce-authored PDF will not install.
- **No shaping, ever (R17).** An embedded Devanagari or Arabic face places
  glyphs by advance with no GSUB/GPOS. FF-C makes L1 *"pdfce can add non-Latin
  text **that does not require shaping**"*: CJK, Cyrillic, Greek, Hebrew
  without vowel points — yes. Arabic, Devanagari, Thai — the glyphs embed and
  the *text is wrong*. FF-E/FF-F territory. **Do not let the L1 headline imply
  otherwise; put this in the ship notes.**
- `NAME` is retained by `subsetter` (copyright preserved — good); `OS/2` is
  dropped, which is why R109 reads it first.

---

## 7. For Ken personally

1. **Font-EULA policy — a legal call, and `docs/decisions/README.md` says
   legal decisions are Ken's.** Embedding redistributes a third party's font.
   OpenType lets the author encode intent in `OS/2` `fsType`. What should
   pdfce do when the donor says *no embedding* / *no subsetting* /
   *preview-and-print only*, and — the common case — when `OS/2` is **absent
   or unparseable**? Options: refuse outright · disclose and require an
   explicit acknowledgement · disclose and proceed. Deliberately **not**
   picked, and the bit values deliberately not quoted from memory. R109 is
   written to accept whichever is chosen.
2. **Complex scripts.** FF-C plus R17 means Arabic/Devanagari/Thai embed but
   render wrong. Refuse those scripts by name at 21.0, or disclose loudly and
   let the operator decide? Recommendation: **refuse by name** — painting
   confident nonsense is the rule-4 failure — but it caps a headline
   capability, so it is worth Ken's call.
3. ~~Pass-number bookkeeping.~~ **Resolved before filing** — see the numbering
   correction at the top. Decision 020 already claims Pass 20.x, so FF-C takes
   21.x and nothing already filed is renumbered.

---

## 8. Revisit triggers

1. `skera` (fontations' general-purpose subsetter, named in `subsetter`'s own
   docs) publishes → re-evaluate; it would unify the write side onto the read
   side's stack.
2. Variable-font donors turn out common in operator font folders → enable
   `variable-fonts`, accepting `write-fonts 0.48` and the pin discipline.
3. A second consumer of the font-program layer appears → revisit B3.
4. The R-INV-4 injective-`/ToUnicode` lift measures well on the corpus →
   consider generalizing into decision 012's FF2 (composite *substitution* via
   the same Unicode route, R65).
5. FF-C makes embedding a real Bold face routine → decision 019 R90's own
   trigger fires: re-order the synthetic-bold-vs-embed remedy offer.
6. Operators ask to *replace* a document's embedded font wholesale → that is
   W3; own decision record; Optimization bucket; **not** FF-C.

---

## 9. Dispatches owed

| Agent | When | Topic |
|---|---|---|
| `pdfce-spec-librarian` | **before 21.0 code** | Rewrite `font__subsetting_ffc_queue.md` (its premise is wrong); ingest §9.7.4.2 `/CIDSet`, §9.7.4.3 `/W` + `/CIDToGIDMap`, §9.8.1 subset tag, §9.9 Table 126 `FontFile3` `/Subtype`, §9.10.3 `/ToUnicode` authoring, and OpenType `OS/2` `fsType` |
| `pdfce-acrobat-librarian` | before 21.2/21.3 acceptance criteria | Acrobat's embed-on-edit behaviour: when it embeds vs substitutes, what it discloses, how it handles a font whose `fsType` forbids embedding |
| `pdfce-ui-specialist` | **before 21.3** | The refusal→remedy flow, the R98 embed confirmation, the face picker, the licence disclosure surface |
| `pdfce-librarian` | on acceptance | File Pass 21.x under Next up; assign R107–R110; the R21/R71/R79 amendments; the decision-012 §6 correction; the `ARCHITECTURE.md` §12 dated entry; the `PRIOR_ART.md` net-cost refinement |

---

## 10. Spec review (2026-08-03) — amendment, eight findings against the record above

`pdfce-spec-librarian` completed the dispatch owed in §9. This record was
written from crate source and shipped code, **not from the standard** — the
standard has now been read and partly disagrees. All eight findings are
recorded, favourable and unfavourable alike: reversing a correct-but-
unsourced claim later would be worse than recording why it held.

**C-3 — CHANGES THE WORK. `subsetter`'s CFF output cannot be emitted
conformantly as §3.4/M2 assumed.** `subsetter` emits an **`OTTO`-wrapped
sfnt** for CFF donors — verified at source, `lib.rs:492`,
`FontFlavor::Cff => 0x4F54544F`, the `OTTO` tag. ISO 32000-1 §9.9 **Table
126**'s `OpenType` rows **require a `cmap` table for CFF-outline programs**
(the `glyf` row does not) — and `subsetter` strips `cmap` by design. So its
CFF output cannot conformantly be `/FontFile3 /Subtype /OpenType`, while
`/CIDFontType0C` requires a **bare** CFF program, not an OTTO container.
§3.4's *"`subsetter` absorbs the TrueType/CFF split entirely"* is **true for
simple-vs-composite, false for the descriptor key**.

**Scope call (librarian, recorded as a §3.4 amendment, not a new decision —
it narrows a decided Pass on a sourced constraint, and the decision already
names `DonorUnsupported` for CFF2, so this extends the same refusal):**
**Pass 21.0's P0 floor is restricted to `glyf` (TrueType-outline) donors.
CFF donors are refused by name (`DonorUnsupported`) at 21.0** and lifted in
a later slice once the OTTO-unwrap and the CID-keyed-CFF question are
settled. Rationale: **L1 survives intact** — Noto Sans JP/CJK, DejaVu and
most Google Fonts are TrueType `glyf`, so "pdfce can add non-Latin text" is
unaffected — and the alternative is emitting a non-conformant `/FontFile3`
on day one, which veraPDF would catch late and expensively. The refusal is
reachable and testable (R96). **This is a narrowing of the P0 floor, not a
silent cut — flag it to Ken at next contact.**

**C-7 — fsType is not one gate, and R109's original taxonomy was too
coarse.** Bit 8 (*No subsetting*) forbids **the only thing FF-C does**: a
face at `0x0108` permits *editable embedding* while forbidding subsetting.
Bit 9 makes a face "unembeddable" in the specification's own word. R109 is
amended (§4.1, above) to name these as **distinct** refusals —
`SubsettingNotPermitted` and `EmbeddingNotPermitted` — not one
`EmbeddingNotPermitted`.

**C-8 — an independent, sourced argument for the add-only (W2) call that
§3.1 did not cite.** ISO 32000-1 §9.9's opening paragraph is a licensing
rule: *"In the absence of explicit information to the contrary, embedded
font programs **shall** be used only to view and print the document,"* and
creating new text requires *"a licensed copy of the font program, **not a
copy extracted from the PDF file**."* ⇒ **an existing document's
`/FontFile*` is not an admissible FF-C donor.** This is a second,
independent reason W1/W3 were correctly rejected in §3.1 — the first was
"the bytes don't exist" (§1.2), this one is "and you may not use them even
where they do." **Modality check, so this is not overstated:** the
producer-side rule (*"should not be incorporated"*) is a **`should`**, so
§9.9 is **not** a blanket `shall not` on embedding a restricted face —
only the reuse-as-a-donor case above is a `shall`. Flagged for the
librarian/engineer to judge whether this deserves standing-rule status as
a constraint on donor provenance; no current rule states it.

**C-1 — favourable.** §3.4's emitted-table list omits `HHEA` (actually
written by `hmtx::subset`) and `CVT`/`FPGM`/`PREP`. ISO 32000-1 §9.9
requires exactly those if present in the original, so the **real** output
satisfies the rule the decision's list appeared to violate. Watch item: the
hinting tables are skipped under `interjector.is_skrifa()`.

**C-2 / C-6 — reframes that strengthen the decision.** `cmap` removal is
not merely a crate quirk, it is **mandated**: §9.9 says that under a
CIDFont dictionary *"the `cmap` table is not needed and **shall not** be
present."* And §9.9 carries a **`shall`** on conforming writers to use
`/Type0` + `Identity-H` for OpenType `glyf` programs. So **M2 (§3.4) is
spec-directed, not crate-forced** — the original text understated its own
case.

**C-4 / C-5 — citation fixes, applied verbatim (§4.2, above).** `/CIDSet`
is **§9.8.3 Table 124**, not §9.7.4.2. The subset prefix is **§9.6.4**, not
§9.8.1 (which has no subset rule). §4.2's dispatch table carried both
errors; corrected in place above so nobody re-derives from a wrong pointer.

---

## Appendix A — engineer handoff notes

- **Read §1.2 first and act on it before writing any code.** The spec-RAG stub
  will actively mislead an implementer: it describes adding an outline to the
  document's existing `FontFile2`. Dispatch `pdfce-spec-librarian` to rewrite
  it *before* 21.0, or someone will build toward a target that does not exist.
- **The single highest-leverage test to write first** is the
  object-id-disjointness assertion (R107). Write it in 21.0 while the emitter
  is trivially correct; it is the only thing standing between 21.2 and a quiet
  round-trip violation.
- **`AddTextRequest.base_font: Std14`** is a closed 14-value enum and is the
  exact widening point. `FontProvenance` is disclosure-only today
  (`addtext.rs` header: *"writes an identical named non-embedded dict either
  way"*) — that sentence becomes false at 21.0 and must be rewritten, not left
  as a stale comment (R93).
- **`format_coverage_hint()`** currently tells the operator to supply a font
  via Tools › Font folders, and supplying one does not fix the saved file.
  That is a shipped promise the write path does not keep — worth flagging to
  the operator as a *current* honesty gap independent of when FF-C lands.
- **Verify the `subsetter` facts yourself before relying on them.** The
  engineer has already re-verified the two load-bearing ones (cmap removal /
  CID forcing at `lib.rs:20–21`; the queue stub's "Add outline to `glyf`" row
  at line 42). Per R87, re-run the dependency probe rather than inheriting
  numbers.
- **`cargo-about` must be regenerated at 21.0**, not at packaging time — the
  dependency set changes there, and rule 13 says regenerate whenever it does.
- Decision 019's Q3 ordering (FF-H → FF-C → FF-B) **survives the §1.2
  correction intact.** Its stated reason — 19.0's ambient text-state model is
  a shared prerequisite because *"FF-C re-encodes runs into newly-subset
  fonts"* — is if anything more true under W2 than under W1, since 21.2
  re-encodes runs into a *different* font entirely.
