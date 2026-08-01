# 002 — Internationalization/localization architecture: decide before the first UI strings

- **Status:** Accepted
- **Date:** 2026-07-30
- **Deciders:** KenAgent (autonomous-builder), on request of `pdfce-engineer`
- **Supersedes:** the "Internationalization/localization" bullet under
  `docs/ROADMAP.md`'s "Product-scope decisions — deliberately deferred"
  (2026-07-23)
- **Scope:** the *application's own UI chrome* in `pdfce-gui` and
  `pdfce-cli`, and the presentation status of `pdfce-core`'s error
  messages. **Does not** decide anything about non-Latin text *inside*
  PDF documents — see §7, which is the most important section of this
  record for anyone skimming.
- **Dependencies added:** none.

---

## 1. Context

`docs/ROADMAP.md` flagged this as a decision with a hard deadline:

> **Internationalization/localization.** No decision on whether v1 ships
> English-only or externalizes UI strings from the start. Cheap to bake
> in now (route every UI string through a translation layer even if only
> `en` is populated); expensive to retrofit into a GUI codebase later.
> Flag to the user before the Pass where the first real UI strings get
> written — this is the point of no return for "cheap to add."

Pass 1 is that Pass. It writes the viewer chrome (open dialog, page
navigation, zoom, thumbnail rail, error surfaces) into `pdfce-gui` and
subcommand help/error text into `pdfce-cli`. The deadline is real and
this record is the answer.

Two facts frame the question, and the second is easy to get wrong:

1. **pdfce targets feature-for-feature Acrobat Pro parity**, and Acrobat
   ships in roughly 35 locales. Localization is therefore inside the
   stated product ambition, not outside it.
2. **pdfce is a sole-operator tool today** with an undecided license
   (`LEGAL.md` §1) that has never been published. There is currently no
   non-English user, no translator, and no repository a translator could
   contribute to.

### 1.1 The constraints this decision has to serve

| # | Constraint | Source |
|---|---|---|
| L1 | **GUI-core separation.** No locale/i18n dependency may reach `pdfce-core` or `pdfce-render`. Those crates are presentation-free by construction. | `ARCHITECTURE.md` §3 |
| L2 | **Single-folder portable, no installer.** Binary size is a live budget; no external catalog files may be required at runtime. | `ARCHITECTURE.md` §6 |
| L3 | **No network calls of any kind by default.** No remote translation fetch, ever, under any design. | `ARCHITECTURE.md` §1.1 |
| L4 | **The WASM/web fork stays a shell-crate swap.** No C FFI; no filesystem-dependent catalog loading. | `ARCHITECTURE.md` §1, §3 |
| L5 | **`pdfce-cli` is genuinely scriptable**, with a documented exit-code contract and machine-parseable output. | `ARCHITECTURE.md` §7 |
| L6 | **Documentation-first.** Whatever discipline is chosen becomes a binding, documented standing rule, not a habit. | `CLAUDE.md` rule 6 |
| L7 | **Permissive licenses only; every dependency license-checked before it is added; attribution generated.** | `LEGAL.md` §6 |
| L8 | **pdfce's own UI aims at screen-reader accessibility**, not just accessible output files. | `.claude/agents/pdfce-ui-specialist.md` rule 6 |

---

## 2. Options considered

**(a) English-only, plain string literals.** Zero overhead now; retrofit
later if ever needed.

**(b) Externalized string layer from day one.** Every user-facing string
goes through a lookup (a `tr!()` macro over a compile-time table, or
`fluent` / `rust-i18n`), with only `en` populated.

**(c) Centralized string-constants module.** All user-facing strings in
one `strings.rs` per crate as plain `&str` consts, no runtime lookup.

**(d) — proposed here — Centralized string *catalog* module,
function-based.** Option (c) corrected on three points that turn out to
be decisive, plus the discipline rules that carry the actual retrofit
cost. Described fully in §6.

---

## 3. Evidence

Everything in §3.1–§3.3 was verified first-hand against pdfce's own
pinned `Cargo.lock` and the vendored crate sources in the local cargo
registry — not asserted from training data — and cross-checked against
upstream issues. Versions are the ones pdfce actually builds against
today.

### 3.1 What egui/epaint 0.35 can and cannot do with text

This is the load-bearing evidence, and it is more favorable in one
direction and less favorable in another than the naive assumption.

**Shaping: present, and recently so.** epaint 0.35.0 shapes with
`harfrust` 0.7.0 — a Rust port of HarfBuzz — over `skrifa` 0.42.1 /
`read-fonts` 0.39.2. This landed in 0.35 (emilk/egui#8031, merged
2026-04-06), immediately after 0.34 replaced `ab_glyph` with
skrifa + vello_cpu (#7694). So GSUB/GPOS substitution and positioning —
ligatures, contextual alternates, kerning, combining-mark anchoring —
work. Arabic joining and Indic reordering are, in principle, reachable
within a homogeneous run.

**Bidirectional layout: absent.** There is no `unicode-bidi` (or
equivalent) anywhere in pdfce's lockfile, and epaint's own source states
the gap in two places. `text/font.rs` carries a literal
`// TODO(emilk): heed bidi characters` immediately above an
`invisible_char()` function that swallows **every** bidi control —
RLM, LRE/RLE, LRO/RLO, PDF, all four isolates — as zero-width.
`text/text_layout.rs`'s `segment_into_runs` documents that segmentation
is by **font face, not Unicode script**, and notes it "would need
script-aware splitting once RTL/bidi support is added." Upstream issue
emilk/egui#1016 has been open since **2021-12-29** with no assignee and
no milestone.

**Bidi absence is a correctness bug, not a cosmetic one.** epaint's
glyph-emission loop assumes monotonically increasing cluster indices;
RTL shaping emits clusters in *decreasing* order, inverting the range,
returning `None` from the text slice, and breaking the documented
`glyphs.len() == char_count` invariant that — per epaint's own comment —
"all cursor and selection code relies on." So Arabic/Hebrew text may
*render* plausibly while cursor placement, selection, and hit-testing
corrupt. For a PDF editor with form-field and annotation text entry,
that is a data-integrity hazard, not a polish item.

**Line breaking: Latin + CJK only.** epaint's break-candidate logic
recognizes whitespace, CJK, and pre-CJK only. There is no UAX #14
implementation and no dictionary-based segmentation, so Thai, Lao,
Khmer, and Burmese wrap only at explicit spaces.

**Maintainer position:** emilk has been consistent since 2021 that he
will not hand-implement bidi — the plan has always been to swap in a
real text stack (#1016, #3378). As of 0.35 that swap is *half* done:
skrifa + vello_cpu + harfrust are in; the bidi/script layer on top is
not, and the one attempt to finish it (PR #5784, Parley) is a stalled,
unfunded draft.

**Consequence for pdfce's locale reach:** Latin/Greek/Cyrillic locales —
roughly 25 of Acrobat's 35 — are technically shippable today and gated
only on a string layer. CJK is gated on fonts (§3.2). Arabic and Hebrew
are gated on upstream work nobody is currently driving. **No choice of
string layer changes any of this.**

### 3.2 Bundled fonts, and a bug that exists right now

`epaint_default_fonts` 0.35.0 ships exactly four faces —
`Ubuntu-Light.ttf`, `Hack-Regular.ttf`, `NotoEmoji-Regular.ttf`,
`emoji-icon-font.ttf`, ~1.4 MB total. Coverage is Latin, Greek, Cyrillic,
IPA, combining diacritics, and emoji. **No CJK, Arabic, Hebrew, Indic, or
Thai glyphs at all** — and bundling CJK is explicitly out of scope
upstream (emilk/egui#3060, closed as not planned).

epaint does per-character fallback across the faces *you registered*
(`find_face_for_char`, memoized), plus a second NOTDEF-time resolution
pass. But there is no automatic system-font discovery (emilk/egui#5233,
open since 2024-10-08). When no registered face has the character,
epaint renders `'◻'` (U+25FB), not U+FFFD.

**This produces a live Pass-1 bug that has nothing to do with
localization.** A user with a Japanese, Chinese, Korean, Arabic, or
Hebrew *filename* — or a document whose `/Title` metadata is in one of
those scripts — sees tofu in pdfce's status bar and canvas, with an
entirely English UI. Bundling `NotoSansCJKsc-Regular.otf` costs
**15.7 MB** against `ARCHITECTURE.md` §6's single-folder budget; runtime
discovery of the system CJK face (Windows ships Yu Gothic, Microsoft
YaHei, Malgun Gothic) costs nothing and fits the portable-folder
constraint. This is filed as a separate Backlog item (§10) because it is
a rendering-correctness issue independent of every other decision here.

### 3.3 clap 4.6's localization ceiling

pdfce pins `clap` 4.6.4 / `clap_builder` 4.6.2.

**Our own text is localizable.** `Command::about`, `long_about`,
`Arg::help`, `long_help`, and `help_template` all take
`impl IntoResettable<StyledStr>`, and `StyledStr` — which is literally
`StyledStr(String)` — has an **ungated** `From<String>`. Runtime-computed
help text is a move, with no leak, no feature flag, no caveat. (The
identifier-ish setters — `Arg::long`, `value_name`, `help_heading` —
take `clap::builder::Str`, whose `From<String>` sits behind the
non-default `string` feature; internally it is
`enum Inner { Static(&'static str), Owned(Box<str>) }`, so the widespread
`Box::leak` folklore is a clap-3-era workaround and is unnecessary in
4.x.)

**clap's own text is not.** Verified directly in the vendored source:
`output/help_template.rs` hardcodes `"Commands"`, `"Arguments"`,
`"Options"`, and `error/kind.rs` hardcodes the failure prose —
`"unexpected argument found"`, `"unrecognized subcommand"`,
`"one or more required arguments were not provided"`,
`"invalid UTF-8 was detected in one or more arguments"`, and the rest.
There is no i18n API. The tracking issue, **clap-rs/clap#380**, was
opened **2016-01-12** and remains open, labeled *waiting on user-facing
design to be resolved*; the sole implementation attempt (PR #5853,
2024-12-22) is an unreviewed draft.

Partial workarounds exist — `help_template`, `next_help_heading`,
`subcommand_help_heading`, `disable_help_flag` plus a custom
`ArgAction::Help` arg, and forking `RichFormatter` via the
`ErrorFormatter` trait — and one crate wraps them
(`clap-i18n-richformatter` 0.3.2, MIT). But #380 itself concedes that
suggestion text, adaptive usage strings, and several bracketed
placeholders stay unreachable. A "localized" pdfce-cli would be a
half-English chimera.

### 3.4 The i18n crate landscape, and one licensing landmine

| Crate | Version | SPDX | Note |
|---|---|---|---|
| `rust-i18n` | 4.2.1 | MIT | compile-time codegen, WASM-clean, actively maintained (2026-07-16) |
| `fluent-static` | 0.5.4 | MIT | compiles `.ftl` to plain Rust fns, no runtime parser |
| `fluent` / `fluent-bundle` | 0.17 / 0.16 | Apache-2.0 OR MIT | runtime bundle lookup |
| `i18n-embed` / `-fl` | 0.16 / 0.10.1 | MIT | |
| `unic-langid` | 0.9.6 | MIT OR Apache-2.0 | maintenance mode |
| `icu` (ICU4X) | 2.2.0 | Unicode-3.0 | formatting primitives, not a message catalog; heavy |
| `rosetta-i18n` | 0.1.3 | ISC | truly zero-dep, but last release 2023-06-23 |
| **`gettext-rs` / `gettext-sys`** | 0.7.7 / 0.26 | MIT (crate) | ⚠️ **disqualified** — see below |

**`gettext-rs` is categorically excluded.** `gettext-sys` statically
links GNU **gettext (LGPL)** on any platform without a native
implementation, and its own README says so explicitly: *"you have to
abide by LGPL."* Windows — pdfce's first target — is precisely such a
platform. It is also a confirmed non-starter on
`wasm32-unknown-unknown`. That is `LEGAL.md` §6.1 (weak copyleft,
statically linked) and §6.2 step 5 (FFI to a non-Rust library, reopening
the single-binary portability question) failing simultaneously, plus
`ARCHITECTURE.md` §1/§3's web-fork constraint. Recorded so no future
session re-derives it.

One benign flag worth pre-empting: `fluent-bundle`'s transitive
`self_cell` is `Apache-2.0 OR GPL-2.0-only`, which trips naive scanners.
**It is already in pdfce's shipping tree** at `self_cell` 1.3.0 (via
epaint's font cache), and `cargo-about` already resolves the disjunction
to Apache-2.0 against the current `about.toml` accept-list — it appears
cleanly in the generated `THIRD_PARTY_LICENSES.md`. Adopting Fluent
later would introduce no *new* license problem from that quarter.

### 3.5 The measured size of the thing being decided

`crates/pdfce-gui/src/main.rs` today contains **7** user-facing strings.
The whole GUI crate is 162 lines. Three of those 7 are not at a `ui.*`
call site at all — they live in a `status_summary()` helper and in an
`rfd` file-filter label — which is itself a useful finding: a call-site
grep would have missed nearly half of them, and so would a mental model
of "the strings are the `ui.label()` calls."

---

## 4. Decision

**Option (d): a centralized, zero-dependency string *catalog* module per
front-end crate, populated with English only — plus eight binding
discipline rules and one CI job.**

Concretely:

1. **`crates/pdfce-gui/src/ui_text.rs`** becomes the single home of every
   user-facing string in the GUI, from Pass 1 onward.
2. **Entries are `pub fn`, never `pub const`.** This is the pivotal
   detail — see §5.2.
3. **Content is English only.** No locale detection, no language menu,
   no catalog file, no `LANG` handling.
4. **No dependency is added.** Not now, and not until a §9 trigger fires.
5. **`pdfce-cli` is English-only permanently, by design** (§5.4), with a
   binding locale-invariant-stdout contract.
6. **`pdfce-core` errors are never localized in-crate**, and in exchange
   carry structured data rather than pre-formatted prose (§5.5).
7. **The discipline rules R1–R8 (§6.1) are the actual deliverable.**
   The catalog module is the cheap part; the rules are what preserve the
   option.

---

## 5. Rationale

### 5.1 The ROADMAP's framing is half right, and the half it gets wrong is the expensive half

The 2026-07-23 note said routing strings through a layer is "cheap now,
expensive to retrofit." That is true of the *layer* and false about
*where the cost lives*.

In a Rust codebase, converting `ui.label("Open a PDF…")` into
`ui.label(tr!("open-hint"))` is a mechanical, greppable, compiler-checked
edit. Painful at 2,000 sites, but linear, safe, and delegable. It is not
where retrofits die.

Retrofits die on things a string layer does not touch at all:

- **Sentence assembly.** `format!("{} {} pages", verb, n)` bakes English
  word order and English pluralization into control flow. No catalog
  fixes it; the code must be restructured.
- **Layout sized to English.** A 120-point column that fits "Page" does
  not fit "Seite von" or "Sivunumero." German and Finnish routinely run
  30–40% longer. Finding every such site later means re-testing the whole
  UI in a pseudo-locale.
- **Pre-formatted prose inside error types.** Once an error variant
  carries `String` instead of the *data* the message was built from, the
  original values are gone and the message can never be re-rendered in
  another language. This is genuinely irreversible without changing a
  public API.
- **Numbers, dates, sizes formatted inline.** Separator and ordering
  conventions get scattered across hundreds of call sites.

Every one of those costs **zero** to avoid today and is expensive later.
None of them requires a translation layer, a dependency, or a catalog
format. **That is the real answer to the ROADMAP's question:** adopt the
discipline now, defer the machinery. Option (b) buys the cheap half at
full price while leaving the expensive half unaddressed.

### 5.2 Functions, not consts — the one correction that makes option (c) actually work

The middle path as posed ("plain `&str` consts") has three defects, and
fixing them is what produces option (d):

**Defect 1 — consts cannot express the interesting strings.** Most real
UI strings are parameterized: `format!("PDF {version} — {}", name)`.
A `&'static str` const cannot hold that, so under option (c) as written
the parameterized majority stays scattered — which is most of the
problem, unsolved.

**Defect 2 — a const-to-function retrofit touches every call site.**
`ui_text::OPEN_BUTTON` → `ui_text::open_button()` is a one-character
change repeated everywhere. Mechanical, yes, but it is precisely the
"touch every UI line" cost the decision exists to avoid. Starting with
functions costs *the same keystrokes today* and reduces the retrofit to
**one file, zero call sites**. This is the entire value of the decision
and it is available for free.

**Defect 3 — the assumed objection to `&'static str` does not hold.**
One might think a function returning `&'static str` cannot be backed by a
runtime-loaded catalog. It can: pdfce's locale would be fixed at startup
(as Acrobat's is, requiring a restart to change), so the loaded catalog
is interned once into a `OnceLock` and leaked — a bounded, one-time
allocation proportional to catalog size, which is the idiomatic answer
for startup-fixed configuration. So `&'static str` returns for static
entries and `String` for parameterized ones are **both retrofit-safe**,
and both are ergonomic at the call site because egui's `WidgetText`
converts from each (already demonstrated by the current code, which
passes both forms to `ui.label`).

Shape:

```rust
/// Label of the toolbar's file-open button.
pub fn open_button() -> &'static str {
    "📂  Open…"
}

/// Toolbar summary after a successful header probe.
///
/// One complete sentence per R2 — never assembled from fragments,
/// so a translation can reorder the version and the file name freely.
pub fn status_probed(version: PdfVersion, file_name: &str) -> String {
    format!("PDF {version} — {file_name}")
}
```

### 5.3 Why not the full externalized layer now (option b)

Not on principle — on cost/benefit, with three concrete costs and no
present benefit:

- **It does not unblock anything.** Per §3.1, the binding constraints on
  pdfce's locale reach are *fonts* (CJK) and *upstream bidi* (Arabic,
  Hebrew). A string layer relieves neither. Shipping one now buys a
  capability that cannot be exercised.
- **Runtime cost is real in an immediate-mode GUI.** egui rebuilds every
  label every frame. A `fluent` bundle lookup plus message formatting,
  per label, per frame, at 60 Hz, across a many-panel editor, is a cost
  paid continuously for a feature nobody is using. (`rust-i18n` and
  `fluent-static`'s compile-time codegen largely avoid this, which is why
  they are the recorded front-runners for a future trigger.)
- **It creates a second source of truth with no compile-time checking.**
  A `.ftl`/YAML catalog drifts from the code silently unless an extra
  macro layer enforces key existence — more machinery, still zero
  translations.

And there is a governance cost: adding a dependency invokes `LEGAL.md`
§6.2 for a crate whose only user is a hypothetical future translator,
while `LEGAL.md` §1 (pdfce's own license) is still open. Deferring costs
nothing and keeps the tree clean.

### 5.4 Why `pdfce-cli` is English-only *permanently*, not "deferred"

This is a positive decision and deserves to be stated as one, because
"deferred" would invite a future session to revisit it wastefully.

1. **The ceiling is external and immovable** (§3.3). clap's own
   `Options:` / `Commands:` / `error:` / `unexpected argument found` are
   hardcoded, with an upstream issue that has been open for over a
   decade. A partly-translated CLI is worse than a consistently English
   one — it reads as broken rather than as unlocalized.
2. **Localizing a scripting interface is a hazard.** `pdfce-cli`'s reason
   to exist is that it is scriptable, with a documented exit-code
   contract (`ARCHITECTURE.md` §7). The GNU convention of `LC_ALL=C` for
   machine parsing exists precisely because localized tool output breaks
   callers. pdfce should not create the hazard it would then need an
   escape hatch from.
3. **There is no parity obligation.** Acrobat Pro has no CLI. Nothing is
   being conceded.

The binding contract that follows (**R5**) is the valuable part: **stdout
is locale-invariant machine output, permanently.** If human-facing
stderr diagnostics are ever localized, no script breaks — because the
separation was designed in from Pass 0 rather than discovered later.

No `ui_text` module is required for `pdfce-cli` at Pass 1: clap's derive
doc-comments already centralize help text on the `Command` enum, and
there are two runtime message sites. Adopt one if runtime message sites
exceed roughly ten.

### 5.5 `pdfce-core` errors: not localized — and the obligation that creates

The engineer's framing offered this as possibly out of scope. **It is out
of scope for localization, and that is a ruling, not an omission** — but
it is emphatically *in* scope for one design obligation, which is the
only genuinely irreversible item in this whole record.

**Why core errors stay English.** Three independent reasons:

- **L1, GUI-core separation.** A locale or catalog dependency inside
  `pdfce-core` would put presentation logic in the crate whose entire
  purpose is to have none. It would not trip the `cargo tree` GUI-crate
  grep, which makes it *more* dangerous, not less — an invariant violated
  in spirit while passing its automated check.
- **Stability.** `Display` output is consumed by logs, CI assertions,
  `pdfce-cli`'s stderr, and the differential test oracle of decision 001.
  Locale-dependent error text makes all of those non-deterministic.
- **Convention.** The Rust API Guidelines' error-message conventions
  (C-GOOD-ERR) describe English, lowercase, sentence-fragment `Display`
  impls. `CLAUDE.md` rule 10 binds pdfce to those guidelines.

**The obligation this creates (R4).** Because the core will not localize
its own messages, the *front end* must be able to. That is only possible
if error variants carry the **structured data** the message was built
from — never a pre-formatted prose `String`. `PdfError` gets this right
today (`MalformedVersion { found: String }`, `MissingHeader { searched }`),
so the rule is "keep doing this." It matters now because decision 001's
six Pass-1 obligations add a substantial set of new error variants
(`FilterError` and its per-filter fail-clean contract, xref and
object-model parse failures), and each is a chance to bake prose into a
variant and lose the underlying values forever. Public-API changes to a
`#[non_exhaustive]` error enum are cheap to get right now and awkward to
correct after `pdfce-cli`, the GUI, and the fuzz harness all match on it.

### 5.6 The web-fork angle favors the light option too

`ARCHITECTURE.md` §1/§3 make the eventual WASM fork a design constraint
on today's code. A `ui_text` module is trivially WASM-portable: it is
plain Rust, compiled in, with no filesystem catalog to load and no C FFI.
A runtime-file-loading i18n layer would need `i18n-embed` /
`include_str!` machinery specifically to work on WASM at all. Choosing
the light option is the choice that keeps the fork a shell-crate swap.

---

## 6. What this decision produces

### 6.1 Standing rules (binding; add verbatim to `ROADMAP.md`)

- **R1 — Single catalog.** Every user-facing string in `pdfce-gui` lives
  in `crates/pdfce-gui/src/ui_text.rs` and nowhere else. Entries are
  `pub fn`, never `pub const`.
- **R2 — No sentence assembly.** Never build a message by concatenating
  fragments, and never `format!` a sub-phrase from one catalog entry into
  a wrapper phrase from another. One entry = one complete, grammatically
  self-contained message with inline named placeholders.
- **R3 — No English-width layout.** Never size a panel, column, grid
  cell, or `add_sized` widget to fit an English string. Prefer egui's
  intrinsic sizing; where a fixed extent is unavoidable, budget +50% over
  the English measurement and document the number in a comment.
- **R4 — Structured errors in core.** `pdfce-core` / `pdfce-render` error
  variants carry the data needed to render a message, never
  pre-formatted prose. Their `Display` is English, diagnostic, stable,
  and never localized. Front ends own presentation.
- **R5 — Locale-invariant machine output.** `pdfce-cli` stdout is
  machine-readable and locale-invariant permanently; it never varies with
  `LANG`/`LC_ALL`. Human diagnostics go to stderr. The exit-code contract
  is likewise locale-invariant.
- **R6 — Formatting helpers.** Page counts, byte sizes, timestamps, and
  Bates ranges shown in the GUI are produced by helper functions in
  `ui_text.rs`, not by inline `format!("{}", n)` at the call site.
- **R7 — Document text is not deferred.** See §7.
- **R8 — No i18n dependency without a trigger.** No i18n crate enters any
  `Cargo.toml` until a §9 trigger fires. `LEGAL.md` §6.2 applies as
  normal when one does; `gettext-rs` is pre-disqualified (§3.4).

### 6.2 The CI job

A new `ui-strings` job in `.github/workflows/ci.yml`, modeled on the
existing `gui-core-separation` job (grep, `::error::`, non-zero exit):

> In `crates/pdfce-gui/src/**` excluding `ui_text.rs`: any string literal
> containing a whitespace character, on a non-comment line, fails the
> build unless the line carries a trailing `// ui-text-exempt: <reason>`.

**Validated empirically before adoption.** Run against the current
`main.rs`, this heuristic flags all 7 user-facing strings — including the
3 that a `ui.*` call-site grep misses — with **zero** false positives.
The whitespace test is what makes it clean: egui `Id` strings
(`"toolbar"`), file extensions (`"pdf"`), `cfg` values (`"windows"`), and
the product name (`"pdfce"`) are single tokens, while human prose has
spaces.

**Honest limitation, recorded so nobody over-corrects later:** the lint
catches *literals*, not helper functions returning English prose
assembled elsewhere. That residue is caught by R1 plus the fact that a
single-file catalog makes drift obvious in `git diff`. Do not escalate to
a heavier mechanism unless drift is actually observed.

### 6.3 What the module looks like

`ui_text.rs` opens with a full module docstring (documentation-first)
covering: purpose; the R1–R3/R6 contract; **why entries are functions
rather than consts**, with the retrofit argument spelled out so a future
reader does not "simplify" them into consts and silently destroy the
property the module exists for; and the §6.4 retrofit recipe. Every entry
carries a doc comment naming *where in the UI it appears* — the context a
translator would otherwise have to reverse-engineer, and which is free to
write while the code is being authored and expensive to reconstruct
later.

### 6.4 The retrofit recipe (write it down now, execute it if a trigger fires)

1. Add the chosen crate (`rust-i18n` or `fluent-static` per §3.4), with a
   `LEGAL.md` §6.2 license check.
2. Extract `ui_text.rs`'s English strings into the catalog format,
   keyed by function name.
3. Rewrite each function body as a catalog lookup. **Signatures do not
   change. Call sites do not change. No other file in the crate is
   touched.**
4. Add startup locale selection; intern static entries into a
   `OnceLock` (§5.2, defect 3).
5. Add a pseudo-locale (`en-XA`-style: accented, +40% length) as a test
   fixture and walk the UI once. This is what catches the R3 violations
   that slipped through review.
6. If `pdfce-web` exists by then, promote `ui_text` to a shared crate
   first (trigger T5).

---

## 7. What this decision explicitly does NOT defer

**Non-Latin text *inside* PDF documents is not deferred, not deprioritized,
and not affected by anything above.** This section exists because
conflating the two would be the single most damaging misreading of this
record.

Two entirely separate text stacks:

| | UI chrome | Document content |
|---|---|---|
| Owner | `pdfce-gui` (egui/epaint) | `pdfce-render` (pdfce's own) |
| Scope of this decision | **deferred to English** | **not deferred at all** |
| Shaping/bidi source | epaint's, with epaint's limits | pdfce's own `harfrust` / `unicode-bidi` path |

A PDF containing Japanese, Arabic, or Devanagari must render, extract,
search, and eventually edit correctly from Pass 1 onward, using embedded
fonts and pdfce's own content-stream interpreter. That is the file
format's job and the product's core competence. `pdfce-render` must **not**
inherit epaint's bidi gap or its Latin-only bundled fonts — the two
stacks share nothing, by construction, which is exactly what the
GUI-core separation invariant buys.

Codified as **R7** so it is enforceable, not merely stated here.

---

## 8. Consequences

**Positive**

- Zero dependencies added; `THIRD_PARTY_LICENSES.md`, the `cargo tree`
  invariants, and the WASM-fork constraint are all untouched.
- Zero runtime cost in the frame loop.
- The retrofit is bounded, estimable, and confined to one file — the
  property the ROADMAP was trying to buy, obtained without the machinery.
- The genuinely irreversible items — pre-formatted prose in error types,
  fragment concatenation, English-width layout — are prevented from
  Pass 1 rather than discovered in a pseudo-locale audit two years out.
- Every UI string acquires a doc comment describing where it appears, at
  the moment that context is free to capture.
- One file becomes the readable inventory of pdfce's entire user-facing
  vocabulary — useful for tone/terminology consistency across an
  Acrobat-parity surface long before it is useful for translation.
- The CLI's machine/human output separation becomes a stated contract at
  Pass 0/1 rather than an accident.

**Negative**

- If a translator appears tomorrow, pdfce is roughly a day of work from
  `en` + one locale, not zero. Accepted: that day is cheap, and it is
  scheduled by trigger T2/T3 rather than left to chance.
- R1–R3 and R6 are discipline, and discipline decays. Mitigated by the
  CI job for R1 and by review for the rest — imperfectly, and honestly so.
- The catalog adds one indirection between reading a UI call site and
  seeing its text. Real, small, and the standard trade.

**Neutral**

- Independent of `LEGAL.md` §1 in both directions — no dependency, no
  license implication.
- Independent of the Windows-first cross-platform scope question, which
  remains separately deferred in `ROADMAP.md`.
- Nothing here is a criticism of egui. Its 0.34/0.35 text work (skrifa,
  vello_cpu, harfrust) is substantial and moving in the right direction;
  the bidi layer is simply not finished, and pdfce is planning around
  where the stack actually is rather than where it is heading.

---

## 9. Revisit triggers

Re-open this record if any of the following becomes true:

1. **egui/epaint gains bidi + script-aware run segmentation**
   (emilk/egui#1016), or `pdfce-gui` adopts an external text stack for
   its chrome. This is the hard structural blocker on `ar`/`he`; until it
   lifts, no string layer can deliver them.
2. **First concrete external demand** — a contributor offers a
   translation, or a non-English user files a request. Translations are
   the most common first contribution to an open-source desktop app;
   watch this from the moment the repository goes public.
3. **Immediately before the first public v1.0 release announcement.**
   Decide then whether `en` + one locale is part of the launch story. The
   catalog makes that a scoped, estimable piece of work.
4. **`ui_text.rs` passes ~300 entries** in any crate — the point where a
   flat module wants per-feature submodules, and the natural moment to
   reconsider a real catalog format rather than restructuring twice.
5. **The `pdfce-web` WASM fork is started** — promote `ui_text` to a
   shared crate at fork time, when it is free.
6. **pdfce ships GUI text *entry* for document content in a non-Latin
   script** (form-field editing, annotation text, search). This fires
   *earlier* than trigger 1 and for a different reason: epaint's
   cluster-index assumption breaks on RTL shaping, corrupting cursor,
   selection, and hit-testing even where rendering looks plausible
   (§3.1). That is a correctness bug in an editor, not a localization
   nicety.

---

## 10. Follow-up actions

**Engineering (Pass 1):**

1. Create `crates/pdfce-gui/src/ui_text.rs` per §6.3; add `mod ui_text;`.
2. Move all 7 existing user-facing strings into it — including the `rfd`
   file-filter label `"PDF documents"` and the three `Status` summaries,
   which are easy to miss because they are not at `ui.*` call sites.
   Author every new Pass-1 viewer string there from the first line.
3. Add the `ui-strings` CI job (§6.2).
4. Add the English-only-by-design + locale-invariant-stdout paragraph
   (R5) to `crates/pdfce-cli/src/main.rs`'s module docs, beside the
   existing exit-code contract.
5. Honor R4 explicitly when decision 001's obligations add error variants
   — structured fields, no pre-formatted prose — and say so in the error
   type's docstring, at the point of temptation.

**Librarian (`pdfce-librarian`):**

6. Archive this record to `docs/decisions/002-i18n-timing.md`.
7. Add R1–R8 to `ROADMAP.md`'s *Standing rules*.
8. **Strike** the "Internationalization/localization" bullet from
   `ROADMAP.md`'s "Product-scope decisions — deliberately deferred" list
   and record it RESOLVED with a pointer here. It is no longer deferred.
9. Add a dated `ARCHITECTURE.md` §12 entry cross-referencing this record.
10. File a **new** `ROADMAP.md` Backlog entry, **"UI font coverage for
    non-Latin file paths and document metadata"** (§3.2). Not a Pass-1
    blocker; it is a live rendering-correctness bug today, independent of
    localization. Options to weigh at scoping: bundle a subsetted CJK
    face (full Noto Sans CJK is 15.7 MB, material against
    `ARCHITECTURE.md` §6) versus runtime system-font discovery (Windows
    ships Yu Gothic / Microsoft YaHei / Malgun Gothic), which fits the
    portable-folder constraint far better.
11. Write a finding to `D:\dev\rag\egui\` capturing the version-stamped
    epaint 0.35 text-stack picture — harfrust shaping present; bidi
    absent (#1016) with the cluster-index hazard; no CJK in bundled fonts
    (#3060 closed as not-planned); no system-font discovery (#5233);
    tofu glyph is U+25FB. This generalizes to any egui project on this
    machine and is exactly the RAG's stated scope.

**Operator check-in (informational, not a decision request):** pdfce's UI
is English-only by decision, not by omission, and — independently of that
decision — cannot support an Arabic or Hebrew interface on egui as it
stands today. Roughly 25 of Acrobat's ~35 locales are reachable once a
translator exists; CJK needs a font strategy; RTL needs upstream work
nobody is currently driving. None of this affects pdfce's ability to
*handle* non-Latin PDFs, which remains a Pass-1-onward requirement (§7).

---

## 11. References

- `docs/ARCHITECTURE.md` §1/§1.1 (goal, privacy posture), §3 (workspace +
  GUI-core invariant), §6 (single-folder packaging), §7 (CLI contract),
  §8 (Rust API Guidelines), §12 (decision log)
- `docs/ROADMAP.md` — "Product-scope decisions — deliberately deferred"
  (superseded by this record); Standing rules
- `docs/decisions/001-oxidize-pdf-adopt-vs-build.md` §6.1 — the six
  Pass-1 obligations whose new error variants R4 binds
- `docs/LEGAL.md` §1 (license undecided), §6.1 (permissive/copyleft
  split), §6.2 (per-dependency check), §6.3 (generated attribution)
- `.claude/agents/pdfce-ui-specialist.md` rule 6 (accessibility)
- `CLAUDE.md` rules 2 (GUI-core separation), 6 (documentation-first),
  10 (Rust style/API), 13 (dependency licensing)
- Verified in-tree: `clap` 4.6.4 / `clap_builder` 4.6.2
  (`output/help_template.rs`, `error/kind.rs`, `builder/str.rs`);
  `epaint` 0.35.0 (`text/font.rs`, `text/text_layout.rs`,
  `text/fonts.rs`); `epaint_default_fonts` 0.35.0; `harfrust` 0.7.0;
  `skrifa` 0.42.1; `self_cell` 1.3.0
- Upstream: [clap-rs/clap#380](https://github.com/clap-rs/clap/issues/380)
  (localization, open since 2016-01-12) ·
  [clap-rs/clap#5853](https://github.com/clap-rs/clap/pull/5853) (draft) ·
  [emilk/egui#1016](https://github.com/emilk/egui/issues/1016) (bidi,
  open since 2021-12-29) ·
  [emilk/egui#2517](https://github.com/emilk/egui/issues/2517) ·
  [emilk/egui#3060](https://github.com/emilk/egui/issues/3060) (CJK,
  closed not-planned) ·
  [emilk/egui#3378](https://github.com/emilk/egui/issues/3378) ·
  [emilk/egui#5233](https://github.com/emilk/egui/issues/5233)
  (system fonts) ·
  [emilk/egui#5784](https://github.com/emilk/egui/pull/5784) (Parley,
  stalled) ·
  [emilk/egui#8031](https://github.com/emilk/egui/pull/8031) (harfrust) ·
  [emilk/egui#7694](https://github.com/emilk/egui/pull/7694) (skrifa)
- [gettext-rs README](https://github.com/gettext-rs/gettext-rs) — the
  LGPL static-link statement quoted in §3.4
