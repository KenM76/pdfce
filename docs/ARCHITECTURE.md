# pdfce — Architecture

This document is the logic. The Rust code is the syntax that enacts it.
Per the user's standing global rule: a competent engineer (human or LLM)
should be able to reconstruct pdfce's design from this file (plus
`ROADMAP.md`, `LEGAL.md`, the PDF-spec RAG, and the Acrobat feature-
parity RAG) without reading a line of code.

## 1. Project goal (verbatim framing from the founding conversation, 2026-07-23)

An open-source, non-monetized, full-feature-for-feature replacement for
**Adobe Acrobat Pro**. The initial application is a native desktop GUI
that does **not** rely on running a web server, a browser runtime, or
any local network listener — everything happens in one native process.
It must run from a single folder, including all of its dependencies
(no installer, no registry writes, no system-wide runtime dependency).

pdfce also ships **CLI capabilities** from the start (`pdfce-cli`, see
§3 and §7) — batch/scriptable operations (merge, split, stamp, convert,
sign, validate) invokable without opening the GUI at all. This is
addressed by the user (2026-07-23) as an explicit project requirement,
not just a developer convenience: Acrobat Pro itself has no equivalent
first-class CLI (only in-GUI Action Wizard batch sequences), so a real
CLI is a genuine parity-plus feature for anyone scripting document
workflows.

A **later fork** (not this codebase's job yet, but a design constraint
on this codebase **today**) will turn the same core logic into a web
application. Every architectural decision below is chosen to keep that
fork cheap when the time comes, without over-building for it now.

**Competitive/prior-art landscape confirmed clear** (see
`docs/PRIOR_ART.md`, researched 2026-07-23): no existing open-source
project, web or desktop, currently combines pdfce's full target
feature breadth in one native application. The closest attempts (Open
PDF Studio, KillerPDF) each have confirmed major gaps and neither uses
a native Rust PDF engine. This validates the project's premise.

### 1.1 Privacy posture (explicit, binding — not merely implicit in "no web server")

pdfce makes **no network calls of any kind by default**: no telemetry,
no usage analytics, no crash reporting, no update-check phone-home, no
license-verification callback. Every document a user opens is
processed entirely locally, in-process, with no data ever leaving the
machine unless the user explicitly initiates it themselves (e.g.
emailing a file — an action pdfce doesn't perform on their behalf
anyway). If a future feature ever genuinely needs network access (an
opt-in update checker, say), it must be **off by default and
explicitly opted into**, disclosed plainly in the UI and in
`README.md`, never silently enabled. This is a load-bearing part of
the project's value proposition ("not a web app" is a promise about
data handling, not just deployment topology) and is treated with the
same weight as the GUI-core-separation and round-trip invariants —
don't add a network call without flagging it to the user first.

**Precision clause (2026-07-30, decision 003 §3.4 — a correction of
wording, not a weakening):** pdfce makes no network requests and
contains **no HTTP client and no TLS stack** — verifiable by any
reader of the generated `THIRD_PARTY_LICENSES.md` — but the shipped
GUI binary does link the `webbrowser` crate (and its `url` parser
dependency), because eframe 0.35 hardcodes egui-winit's `links`
feature and it cannot be disabled downstream. That code opens the OS
default browser and makes no request itself; it is inert unless pdfce
emits an `OpenUrl` event. When it fires, the request belongs to the
user's browser, not to pdfce. State the posture in exactly these terms
(decision 003 §6.3's copy) — "no network code at all" would be false.
Enforcement is the fail-closed `no-network` CI job (decision 003 R12):
no HTTP/TLS/socket client crate may enter any pdfce crate without a
new decision record.

## 2. Language & toolkit decision (made 2026-07-23, by the user)

| Decision | Choice | Why |
|---|---|---|
| Systems language | **Rust** | Single self-contained native binary (no runtime to bundle), memory safety for a file-format parser that will be fed adversarial/malformed input from the public internet, first-class WASM target for the future web fork, mature crate ecosystem for compression (`flate2`, `weezl` for LZW), fonts (`ttf-parser`, `allsorts`), image codecs, and crypto (`rsa`, `aes`, `sha2`, `rustls`-adjacent primitives) that pdfce will need anyway. |
| GUI toolkit | **egui + eframe** (recommended default — see §2.1) | `eframe` is egui's application shell and already targets **both native (winit+wgpu/glow) and WASM+canvas from the same codebase** — this is the single biggest lever for making the later web fork cheap. Immediate-mode fits a tool-heavy, many-panel editor (canvas + thumbnails + inspector + toolbars) well; prior art includes rerun.io and many CAD-adjacent Rust tools built the same way. |
| Rendering backend | `wgpu` (falls back to `glow`/OpenGL if needed) | Cross-platform, matches eframe's default, no separate native-toolkit dependency to bundle. |

### 2.1a — Toolchain pin & lockfile policy (Pass 0 task)

- **Toolchain**: pin a specific stable Rust release via `rust-toolchain.toml`
  at the workspace root, created at Pass 0. Don't float on "whatever
  stable is installed" — reproducibility matters for a project other
  people will eventually build. Bump deliberately (dated decision-log
  entry), not silently.
- **MSRV** (minimum supported Rust version): not yet decided — set it
  at Pass 0 once the toolchain is pinned, document it in `Cargo.toml`'s
  `rust-version` field, and re-check it against `docs/PRIOR_ART.md`'s
  candidate dependencies (some crates there have their own MSRV floors
  that could force pdfce's own MSRV higher than expected).
- **`Cargo.lock`**: **commit it.** This is an application workspace
  (produces `pdfce-gui`/`pdfce-cli` binaries), not a pure library —
  the Rust ecosystem convention for binaries is to commit the lockfile
  for reproducible builds, unlike libraries which typically don't.
  Don't `.gitignore` it.

### 2.1 — egui vs iced: still confirm at Pass 0

The user's decision was "Rust core + native GUI (egui/iced)" — the
specific pick between the two was left to engineering judgment. This
document recommends **egui/eframe** for the WASM-parity reason above.
**pdfce-engineer**: treat this as a strong default, not yet a closed
decision — confirm it explicitly with the user at the start of Pass 0
(the first real coding session) before the workspace is scaffolded,
since reversing it later means rewriting the entire GUI crate.

## 3. Workspace layout (Cargo workspace, to be created at Pass 0)

```
D:\Dev\pdfce\
  Cargo.toml                  <- workspace root, [workspace] members below
  crates\
    pdfce-core\                <- COS object model, tokenizer, xref (table + stream),
                                   object streams, incremental-update writer, filters,
                                   fonts, color spaces, encryption/decryption, digital
                                   signature verification, content-stream interpreter
                                   (produces a display-list / draw-op stream, NOT pixels).
                                   ZERO windowing/GUI/rendering-backend dependencies.
                                   THIS is the crate that forks to WASM later.
    pdfce-render\               <- Takes pdfce-core's draw-op stream + resources
                                   (fonts, images, color spaces) and rasterizes to an
                                   in-memory pixel buffer via `tiny-skia` (CPU-only,
                                   pure Rust, no GPU/windowing context — see
                                   docs/PRIOR_ART.md, resolved 2026-07-23).
                                   Depends on pdfce-core. Still GUI-framework-agnostic
                                   (no egui/eframe dependency) — a headless render
                                   (e.g. "render page 3 to PNG") must work with zero
                                   windowing system present, which is also what makes
                                   the eventual web fork (canvas-based rendering) and
                                   any future CLI/batch tooling possible.
                                   **Implementation note (Pass 1, amended 2026-07-30):**
                                   the content-stream *interpreter* (`gstate`/
                                   `interpret` modules, incl. §8.10.1 Form-XObject
                                   execution and §8.9 image drawing) lives HERE, not
                                   in pdfce-core as this diagram's original wording
                                   ("content-stream interpreter... produces a
                                   display-list/draw-op stream") implied — pdfce-core
                                   supplies the lossless content-token model only
                                   (`content.rs`); pdfce-render walks those tokens and
                                   paints directly, with no separate draw-op IR
                                   in between. Recursive `Do` dispatch into nested
                                   Form XObjects is therefore a pdfce-render-time
                                   concern (`MAX_XOBJECT_DEPTH`, §10.1), distinct from
                                   pdfce-core's parse-time recursion guards (page-tree
                                   depth, xref/ObjStm cycles).
    pdfce-gui\                  <- The native desktop shell. egui/eframe application,
                                   window chrome, file dialogs (rfd crate), menus,
                                   docking layout (egui_dock or hand-rolled), the
                                   `fn main()` entry point and packaged executable.
                                   Depends on pdfce-core + pdfce-render.
    pdfce-cli\                  <- The command-line batch shell. Subcommand parsing
                                   (clap crate), one subcommand per batch operation
                                   (merge/split/rotate/extract, Bates stamp, convert
                                   to PDF/A, sign, validate PDF/A or PDF/UA conformance
                                   and print a report, render-page-to-PNG for scripted
                                   thumbnailing). `fn main()` entry point, packaged as
                                   its own executable alongside pdfce-gui in the same
                                   single-folder distribution. Depends on pdfce-core +
                                   pdfce-render, same as pdfce-gui — ZERO GUI/windowing
                                   dependencies of its own, see §7. Doubles as a fast,
                                   windowless way to exercise pdfce-core in tests.
    pdfce-web\ (future, not     <- The web fork. Same pdfce-core + pdfce-render,
      built in this phase)         compiled to wasm32-unknown-unknown, eframe's
                                   web target, served as static files (no server-side
                                   PDF processing — everything still runs in-browser,
                                   preserving the "not a web app in spirit" privacy
                                   posture even in the fork).
  docs\                        <- This file, ROADMAP.md, LEGAL.md, SESSION_LOG.md
  .claude\agents\               <- pdfce-engineer, pdfce-librarian,
                                   pdfce-spec-librarian, pdfce-ui-specialist
  tests\                       <- Integration tests: parse→render→compare fixture PDFs
  fixtures\                    <- ONLY synthetic or clearly-licensed-for-redistribution
                                   test PDFs (see LEGAL.md §Test corpus sourcing).
                                   Never a scanned/downloaded real-world PDF of unknown
                                   provenance.
```

**Invariant (do not violate):** `pdfce-core` and `pdfce-render` must
compile with zero GUI/windowing crates in their dependency tree. This
is checked, not just hoped for — `cargo tree -p pdfce-core` and
`cargo tree -p pdfce-render` should never show `egui`, `eframe`,
`winit`, `wgpu` (a headless CPU rasterizer like `tiny_skia` in
`pdfce-render` is fine; a *windowing* dependency is not). This is the
single invariant that keeps the future web fork a "swap the shell
crate" job instead of a rewrite.

## 4. Core data model (target contract — implemented incrementally per ROADMAP)

This is what `pdfce-core` will expose once Pass 1+ lands. Written now
as the target so early implementation work has a north star; update
this section the moment the real API diverges (the doc is the logic —
if code and doc disagree, that's a bug in one of them, fix it same-day).

**Current state as of Pass 0 (2026-07-23):** `pdfce-core` exposes ONLY
the header-probe surface — `PdfVersion { major, minor }`, `PdfError`
(`thiserror`, `#[non_exhaustive]`), `probe_header(&[u8])`,
`probe_file(&Path)`, and the `HEADER_SCAN_WINDOW` const (1024, the
byte window the `%PDF-` marker is scanned within). None of the
`Document`/`Object`/`Page`/`StreamData` model below exists yet; it is
the Pass 1+ target. The user deliberately kept Pass 0's core this thin
to defer the from-scratch-vs-`oxidize-pdf` foundation decision (§12
entry (b), 2026-07-23) — do not treat the contract below as implemented.

**Forward pointer (2026-07-30):** that decision is now CLOSED — build
from scratch (§12 entry 2026-07-30,
`docs/decisions/001-oxidize-pdf-adopt-vs-build.md`) — and it binds six
Pass-1 obligations on this model: `ByteSpan` provenance, a lossless
content-stream token model, the ONE-object-model invariant, fail-clean
filter contract, unwrap-deny lints, and no output fingerprint. The
engineer integrates the full design text here at Pass 1.

- `Document` — owns the COS object graph, trailer, xref, and the
  original byte buffer (for lazy/unmodified-object passthrough on
  write — see the round-trip invariant below).
- `ObjectId(u32 /* number */, u16 /* generation */)`
- `Object` — enum: `Null | Bool | Integer | Real | String(Str) | Name |
  Array(Vec<Object>) | Dict(Dictionary) | Stream(Dictionary, StreamData) |
  Reference(ObjectId)`
- `StreamData` — lazy: holds the raw (still-encoded) bytes plus the
  filter chain; decoding happens on demand and is cached, never eager
  for every stream in a large document.
- `Page` — resolved view over a page dictionary: `MediaBox`, `Resources`,
  content stream(s) concatenated, inherited attributes resolved per
  §7.7.3.4 of the spec (page tree attribute inheritance).
- `Document::open(path) -> Result<Document, PdfError>`
- `Document::save(path) -> Result<(), PdfError>` — full rewrite.
- `Document::save_incremental(path) -> Result<(), PdfError>` — **the
  default save mode.** Appends a new xref section + updated/new objects
  only; every object pdfce did not touch is left byte-identical in the
  file. This is not an optimization, it's a correctness requirement:
  Acrobat's own digital-signature model depends on incremental updates
  (a signature covers a byte range; anything after that range is a
  later revision). pdfce must support this from day one, not bolt it on
  after signatures are implemented.
- `Document::render_page(index, dpi) -> Pixmap` (in `pdfce-render`,
  takes a `&Document`).

## 5. Round-trip / non-destructive-editing invariant

Analogous to the tail-bytes / lazy-round-trip discipline the user's
other format-RE project (SWFormat) established for SOLIDWORKS files —
same principle, different format:

- Any object pdfce did not logically modify is re-emitted **byte
  identical** (for full rewrite) or **omitted entirely** (for
  incremental save, since the old bytes are simply not touched).
- Never "normalize" a PDF's internal structure as a side effect of an
  unrelated edit (e.g. don't silently rewrite every xref table to xref
  streams just because pdfce opened the file). Minimal-diff editing is
  a hard requirement — Acrobat users expect that adding one comment to
  a 400-page contract does not perturb the other 399 pages' bytes,
  and forensic/signature-validity expectations depend on it.
- Corollary: **redaction is the one deliberate exception.** True
  redaction must actually remove the covered content from the object
  stream (not just draw a black box on top) — see ROADMAP backlog
  item "Redaction — true content removal". This is a documented,
  intentional violation of the minimal-diff rule for exactly the
  objects the user asked to redact, and only those.
- **Forward pointer (2026-07-30):** the mechanical enactment of this
  invariant — `ByteSpan` provenance on every parsed object and a
  lossless, span-provenanced content-stream token model — is specified
  by the six Pass-1 obligations in the §12 entry of 2026-07-30
  (decision record `docs/decisions/001-oxidize-pdf-adopt-vs-build.md`
  §6.1); full design text lands in this document with Pass 1.
- **PDF-1.5 extension (2026-07-30, Pass 1.1 item 1 — continuation
  8):** objects parsed out of object streams (§7.5.7) carry
  `Provenance::ObjectStream { container, index }` rather than a file
  `ByteSpan` — for these, byte-identical passthrough is
  **expressible-or-consciously-absent**: a compressed object has no
  contiguous file bytes to re-emit, so any writer that touches one
  must either promote it to an uncompressed object or rewrite its
  container stream. The contract is documented on the `Provenance`
  type itself in `pdfce-core`; `file_span()` returns `Some` only for
  `Provenance::File`. See the §12 continuation-8 entry of 2026-07-30.

### 5.1 The invariant stated precisely — three contracts, never one

*(Added 2026-07-31, Pass 3.0. Before this Pass §5 was prose; it is now
a measured gate. Decision 007 W1/R32 names conflating these "the single
likeliest source of a false green or a false red".)*

The invariant is **not** one claim. It is three, and each save mode
promises exactly one of them:

| Save mode | What is byte-identical | Assertion shape |
|---|---|---|
| `save_incremental`, **empty dirty set** | the **whole file** — output *is* input | `output == input` |
| `save_incremental`, non-empty dirty set | **every byte below the original EOF** | `output.starts_with(input)` |
| `save_full` | **every object definition** of a `Provenance::File` object | per object, **never** per file |

A full rewrite **cannot** be byte-identical file-wide: object offsets
move, so the cross-reference section must differ. A test asserting
file-level identity for `save_full` fails universally; a test asserting
only reloadability passes vacuously. Both mistakes look like diligence.

Two corollaries that are easy to get wrong and expensive to discover
late:

- **Zero edits means zero bytes.** An empty dirty set produces the
  input file, not "the input plus an empty revision". Appending a
  revision to a document the operator did not change is itself a §5
  violation.
- **The dirty set is a save-time diff against the base revision**,
  never the union of every command run (§11.1). §7.5.6 requirement 1
  is the spec-side reason: an update section *"shall contain entries
  **only for** objects that have been changed, replaced, or deleted"*
  — a restriction, not merely permission to omit.

**Measured, not asserted** (Pass 3.0, 2,914-file corpus): whole-file
identity 2,898/2,898 loadable files (100%); prior-bytes-intact on
append 2,898/2,898 (100%); per-object verbatim on full rewrite
2,897/2,898, the one exception being a hybrid file pdfce **refuses by
name** (see below). `tools/roundtrip` is the executable gate; it
re-runs on every writer-touching Pass.

### 5.2 Redaction forbids incremental save

*(Added 2026-07-31, Pass 3.0, closing decision 007 W2. This was
trust-critical and undocumented, and incremental is the DEFAULT mode.)*

**Incremental save structurally preserves superseded content.** §7.5.6
requires that *"changes shall be appended to the end of the file,
leaving its original contents intact"* — so the old bytes of every
replaced object remain in the file **by construction**. A redaction
saved incrementally therefore leaves the redacted content trivially
recoverable by anyone who reads the earlier revision.

Binding rule (**R35**): redaction — and any operation whose contract is
*removal* — **must force a full rewrite and must refuse incremental
save.** This is enforced in the writer, not left to the Redaction Pass
to remember, and the Redaction Pass owes a test that greps the saved
bytes for the removed content.

See also §11.2, which covers the *undo* half of the same exception:
once a redaction is written, no later session can undo it, because
there is no data left in the file to restore.

**Correction (2026-07-31, Pass 3.1):** this section's original framing
implied that forcing a full rewrite closes the stale-copy path for
**promoted compressed objects**. It does not — object streams carry
through **verbatim in both save modes** (§5.6), so a promoted object's
superseded value survives inside its untouched container even after a
full rewrite. R35's refusal of incremental save is necessary but NOT
sufficient for redaction; see §5.7 for the full amendment and the
binding consequence for the Redaction Pass (container
rewrite/decomposition).

### 5.3 `/ID` discipline on save

*(Added 2026-07-31, Pass 3.0, closing decision 007 W6/R39.)*

§14.4 says `ID[0]` *"shall not change when the file is incrementally
updated"* and `ID[1]` is *"a changing identifier based on the file's
contents at the time it was last updated."* Read naively, the second
half conflicts head-on with byte-identical round-tripping.

It does not actually conflict, and the reasoning matters because it
will be re-litigated:

1. **If nothing changed, nothing was "updated"** — §14.4's trigger
   never fired.
2. `/ID` is `should`-strength for unencrypted files, and **no `shall`
   anywhere requires regeneration**. §14.4 states what `ID[1]` *is*,
   not when a writer must recompute it.

Binding rule: pdfce regenerates `ID[1]` **exactly when a save writes at
least one changed object**, and never otherwise. `ID[0]` changes only
when pdfce creates a document it regards as new (a from-scratch write,
or an explicit "Save As new document") — never on incremental save or
plain full rewrite. This is also an R41 matter: a gratuitously
regenerated `/ID` is an observable *pdfce touched this file* signal on
a file pdfce did not change.

Load-bearing beyond tidiness: `/ID[0]` is an input to §7.6.3.3's
encryption-key derivation, so an error here becomes a Pass 5 decryption
failure that presents as a crypto bug.

### 5.4 Linearization is invalidated by any save, and never repaired

*(Added 2026-07-31, Pass 3.0, closing decision 007 W5. Citation
corrected 2026-07-31, Pass 3.2 filing: this section's rule is
**R42** — the original "R36" citation collided with decision 007's
R36, "save mode is chosen by contract and disclosed," which the
writer/`document.rs`/`linearization.rs` code comments cite and keep.
See the dated reconciliation note at R42 in `ROADMAP.md` Standing
rules. The warn-before-save behavior below remains ALSO covered by
R36's disclosure clause; the never-repair/never-strip/never-patch-`L`
rule is R42.)*

Annex F.1 is normative and blunt: *"Incremental update shall still be
permitted, but the resulting PDF is **no longer linearized** and
subsequently shall be treated as ordinary PDF."* An append lands past
the first-page cross-reference table and the hint streams, so the
linearization is stale afterwards.

That is spec-sanctioned and unavoidable — but it is an observable
property change the operator did not ask for (the file opens more
slowly over a network). Under the *fuzzy, never sneaky* rule, pdfce:

- **detects** linearization on load (Annex F.3.3's 1024-byte parameter
  dictionary, with `L`-versus-file-length as the liveness check);
- **warns** before a save that would spend a live Fast Web View
  property;
- **never strips** a stale `/Linearized` dictionary (that would be a
  normalization, and Annex G.7's reader-side revalidation depends on
  it being present);
- **never patches `L`.** `L` is not the property — the object ordering
  and hint validity are. A file whose `L` was "fixed" after an append
  *claims* to be linearized while its hints point into a stale layout,
  which is strictly worse for a network reader than an honestly
  de-linearized one.

Re-linearization belongs to the Optimization backlog bucket, not to any
save path.

### 5.5 Signatures and the redaction conflict

*(Added 2026-07-31, Pass 3.0, closing decision 007 W7.)*

§12.8.1 NOTE 1: *"If a signed document is modified and saved by
incremental update, the data corresponding to the byte range of the
original signature is preserved."* A **full rewrite destroys every
existing signature**, because a signature covers a byte range that a
full rewrite necessarily disturbs.

So signature presence forces incremental — which collides head-on with
§5.2's rule that redaction forces a full rewrite. **"Redact a signed
document" is a genuine either/or, not an oversight**, and it must be
surfaced to the operator as an explicit choice, never resolved
silently. Naming it here means neither the Redaction Pass nor the
Signatures Pass can claim surprise.

Structural consequence already in force: pdfce never re-serializes a
signature dictionary, *even identically*. Its `/Contents` is a
fixed-width placeholder referenced by byte offsets, so re-emitting it
is a hazard regardless of whether the bytes come out the same. The
answer is structural rather than a special case — signed objects are
`Provenance::File` objects and ride the verbatim copy path like any
other.

### 5.6 Never normalize — the rule that has no spec backing, and needs none

*(Added 2026-07-31, Pass 3.0. Decision 007 R33/W4.)*

§7.5.6 contains **no requirement** that an appended update section
match the form of the section it supersedes. That is a recorded
NEGATIVE RESULT in the spec RAG, not an oversight — which is precisely
why the rule has to be pdfce's own.

pdfce emits whatever the base file's **newest** cross-reference section
already used, and never chooses:

- a classic §7.5.4 table stays a classic table;
- a §7.5.8 cross-reference stream stays a stream;
- a §7.5.8.4 **hybrid** file is appended to as a classic section
  carrying `/XRefStm` **forward** (form A — the only shape that
  satisfies §7.5.6 requirement 3's *"all the entries except the `Prev`
  entry … whether modified or not"*, since `/XRefStm` is such an
  entry);
- object streams are carried through a full rewrite **intact, with
  zero promotions**: a type-2 entry names a container and an index,
  neither of which is a byte offset, so re-emitting the container
  verbatim leaves every type-2 entry still correct;
- the `%PDF-M.N` header line and its §7.5.2 binary-comment line are
  copied byte-for-byte, so no save can raise a file's version.

**A full rewrite of a hybrid file is refused by name** rather than
flattened. §7.5.8.4 describes a hybrid as a three-part unit a writer
creates *"at the same time"*; rebuilding it from a merged view requires
re-deriving the hidden-object set and re-checking the clause's
recursive visibility rule. Normalizing it to a single section instead
would silently destroy the file's pre-1.5 readability. Refusing is the
R27 fail-clean posture applied to the write side: name it, count it,
do not guess.

### 5.7 The mutation writer, promotion, and the stale-copy reality

*(Added 2026-07-31, Pass 3.1 — the first Pass with real mutations.
Records both the mutation-writer design and a CRITICAL correction to
§5.2's original framing and decision 007 W3's mitigation. Corrections
are recorded forward; the archived 007 record is not edited.)*

**Design: one writer path, dirty set as an argument.** Pass 3.1
extended the Pass 3.0 writer rather than adding a mutation sibling:
`save_full` (like `save_incremental`) now takes a `&DirtySet` —
replacements (object number → new definition) plus a trailer patch,
with `changes_content()` distinguishing content-bearing edits from
metadata-only ones. `DirtySet::empty()` reproduces Pass 3.0's identity
behavior exactly, making identity a **strict pinned subset** of the
mutation writer, not a parallel code path that could drift. The dirty
set itself is produced by `EditSession` (§11.5) as a save-time diff
against the base revision, per §11.1. `/ID[1]` is derived per §14.4 in
`writer/fileid.rs`, exactly when a save writes at least one changed
object (§5.3); `/ID` is **never synthesised when absent**, in either
mode — the spec RAG's synthesise-on-full-rewrite recommendation was
declined (R41: stamping an `/ID` into a file that never had one is an
observable "pdfce touched this" signal); a real Save-As path may
revisit.

**Promotion (R38) in practice.** A touched
`Provenance::ObjectStream` object is promoted to an uncompressed
object superseded by a type-1 xref entry; its container is left
byte-untouched. Coverage honesty: promotion is **fixture-covered, not
corpus-covered** — 75 corpus files hold 2,197 compressed objects, but
page objects are uncompressed in all of them, so the corpus rotation
gate never exercises promotion; the round-trip harness reports both
numbers so the gap cannot silently pass for coverage.

**The stale-copy reality — CORRECTION.** Decision 007 W3's mitigation
and §5.2's original framing claimed a full rewrite "closes the
stale-copy path" for promoted compressed objects. **FALSE.** Object
streams carry through **verbatim in BOTH save modes** — incremental
save never touches them by construction, and `save_full` re-emits
containers intact with zero promotions (§5.6, deliberately, because
rewriting a container perturbs every other object inside it). So a
promoted object's old value survives inside its untouched container
under *either* save mode. Binding consequence (documented at the
creating code as well): **the Redaction Pass must rewrite or decompose
every container stream that holds a redacted object.** R35 (refuse
incremental) is necessary but not sufficient; the redaction test that
greps saved bytes for removed content (§5.2) is what will hold this
honest, provided its fixtures include object-stream-compressed
content.

**Object creation and `/Size` suppression.** The Pass 3.1 fuzzer found
a real bug class here: creating a new object by raising `/Size`
**resurrected** xref entries that the base trailer's `/Size` was
suppressing (§7.5.4/§7.5.8: entries beyond `/Size` shall be ignored —
and real chains carry such entries, which then fail to parse when
exposed). Fix: `next_object_number` allocates above the **unfiltered**
chain maximum (never reusing a suppressed number), and creation is
refused by name when `/Size` suppresses entries
(`EditError::ObjectCreationWouldExposeHiddenObjects`, CLI exit 9);
editing existing objects still works on such files. Lesson:
`C:\personal_rag\pdf\lesson_20260731_xref_size_suppresses_trailing_entries_raising_resurrects.md`.

### 5.8 Flatten burns in by overlay-APPEND, not content-stream surgery

*(Added 2026-08-01, Pass 7.1 — the first operation that makes an
authored appearance part of a page's rendered content. Records the
design and why it is MORE minimal-diff than the in-place rewrite the
Pass scope anticipated.)*

**The problem.** Flattening a form field removes the interactive widget
and bakes its current appearance into the page so it renders identically
in a non-form-aware viewer. The obvious implementation — splice the
widget's appearance operators into the existing page content stream —
would rewrite that stream, which under §5.6 (never normalize) and the R46
identity discipline is exactly the destructive re-emission pdfce avoids on
every object it did not logically change.

**The design pdfce adopted.** Flatten does NOT touch the existing page
content stream. It:

1. builds a one-line overlay content stream that sets the widget's
   placement matrix (the §12.5.5 `fit_matrix_for` `/Rect`→`/BBox`
   transform) and `Do`-invokes the widget's existing `/AP` `/N` form
   XObject by name (`ContentBuilder::invoke_xobject` — `/Name Do`);
2. APPENDS that new stream to the page's `/Contents` array (promoting a
   single-stream `/Contents` to an array as needed);
3. registers the `/AP` `/N` XObject under the page's `/Resources`
   `/XObject` (`add_page_xobjects`, merging into the page's effective
   resources); and
4. removes the widget from `/Annots` and the field from `/AcroForm`
   `/Fields` (`remove_from_annots` / `remove_fields_from_form`), clearing
   `/NeedAppearances` if it was set.

The pre-existing page content bytes are never re-serialized. The only new
bytes are the appended overlay stream and the dict edits.

**Consequence — R46 keeps ZERO flattened-page exceptions.** Because the
existing content stream passes through byte-verbatim (§5.6 span
re-emission), the R46 re-emit-everything identity gate finds no new
divergence on a flattened page: GATE PASS over `fixtures/synthetic` +
`fixtures/external`, all divergences the known value-preserving `-0`→`0`
number re-spellings, zero corruptions. In-place surgery would have put
every flattened page's content stream through the canonical serializer,
surfacing (harmlessly, but noisily) the number-respelling class §5.6/R46
document — and, worse, would have been a genuine rewrite of content the
operator did not ask to reformat.

**R48 (flatten discloses its destructiveness) is still honored.** Flatten
is destructive in the sense R48 means — the interactive field is gone. But
under incremental save the field dict survives in the PRIOR revision
(recoverable), which flatten discloses; a `--full-rewrite` save produces a
file with no `/FT`/`/Tx` that still renders the burned value. Flatten uses
the STRICT certification gate (refused on any enforced `/DocMDP`, including
`/P 2` certified — proven by test), NOT the fill path's `/P >= 2` permit,
because flatten is a STRUCTURAL change to the page/annotation/field
structure, not a value fill.

**General pattern (recorded for future Passes).** Overlay-APPEND beats
content-stream-surgery whenever the goal is ADDITIVE burn-in (make
something already-authored part of the rendered page). Reserve true
in-place content-stream surgery for the one operation whose goal is
REMOVAL, not addition: **Redaction (Pass 8)** — the R46 named exception,
where covered operators must actually be deleted from the content stream
(and containers decomposed, §5.7), because visual masking is not removal.
The two operations are mirror images: flatten adds without rewriting;
redaction removes and must rewrite. This finding is escalated as a
`personal_rag/pdf` lesson
(`lesson_20260801_flatten_overlay_append_beats_content_stream_surgery.md`).

### 5.9 Every removal/scrub operation forces a full rewrite (R58 — generalizes §5.2's R35)

*(Added 2026-08-01, Pass 8.0 — Redaction landed, and the
`pdfce-ui-specialist` review generalized R35 into a standing rule that
binds every future scrub operation, not just redaction-apply.)*

§5.2 established **R35** for redaction specifically: because incremental
save structurally preserves superseded content (§7.5.6 requires the
original contents be left intact and changes appended), a removal saved
incrementally leaves the removed content trivially recoverable in the
prior revision. The remedy — force a full rewrite, refuse incremental
save, drop `/Prev` so prior revisions are gone — is not unique to
redaction. It is the correct posture for **any** operation whose contract
is *removal or scrubbing of content*.

**Binding rule (R58):** every removal/scrub operation rides the same
forced full rewrite as redaction-apply. This includes, prospectively, any
**Sanitize / Remove-Hidden-Information / metadata-scrub** Pass pdfce may
add. Three obligations travel with the rule:

1. **Force full rewrite, refuse incremental save.** The R35 mechanism,
   enforced in the writer, not left to each scrub Pass to remember.
2. **Decompose every object-stream container holding a scrubbed object**
   (§5.7). Refusing incremental save (R35) is necessary but NOT sufficient:
   object streams carry through verbatim in BOTH save modes, so a scrubbed
   object's old value survives inside its untouched container unless the
   container is rewritten/decomposed. Pass 8.0 proved this concretely — a
   redacted `/Info` compressed in an `/ObjStm` survives without §7.5.7
   Strategy B decomposition (`containers_decomposed >= 1`).
3. **Owe an absence test.** The scrub Pass greps the whole saved output —
   raw bytes AND every decoded content stream — for the removed content
   and asserts zero occurrences. This is R46 inverted: R46 proves presence
   (untouched content re-emitted byte-identical); the absence test proves
   deletion (removed content gone from the entire file). Pass 8.0's
   headline embodied it: `redact-apply` on `demo-secret.pdf` →
   `grep "SECRET" redacted.pdf` = 0 (control `marked.pdf` = 3).

The general framing (§5.8): flatten and redaction are mirror images —
flatten ADDS without rewriting (overlay-append), redaction/scrub REMOVES
and must rewrite (content-stream surgery + container decomposition). R58
is the standing-rule form of "removal is never additive, and never
incremental."

### 5.10 A cross-reference-recovered document forces a full rewrite (R67 — third sibling of §5.2/R35 and §5.9/R58)

*(Added 2026-07-31, decision 013. **FLIPPED TO SHIPPED/ACTIVE 2026-08-01**
— Pass 13b (rebuild-by-scan recovery) shipped this session; the contract
below is now enforced code, not a forward-looking design note. R67 is now
IN FORCE. See `ROADMAP.md` Shipped, Pass 13b, for the acceptance numbers:
566 previously-failing real-world files now open (1,109-file corpus), zero
regression on the 2,907-file veraPDF corpus, `*-fail-*` reconciliation
complete.)*

§5.2 (R35, redaction) and §5.9 (R58, all removal/scrub) force a full rewrite
because incremental save structurally *preserves* superseded content.
Cross-reference recovery forces a full rewrite for a **different but equally
structural** reason: a document loaded via rebuild-by-scan had an **invalid
base cross-reference table**. An incremental append onto it would write a new
section whose `/Prev` points at a cross-reference section that does not
correctly exist — the appended file would be self-inconsistent and would fail
to reload. **Incremental-append onto a broken base is structurally
impossible, not merely undesirable.**

**Binding rule (R67):** a recovered document's save is a **mandatory full
rewrite** (`save_full`) emitting a fresh valid classic xref/trailer/
`startxref`. `save_incremental` on a recovered document is **refused by
name** (`WriteError::RecoveredBaseForbidsIncremental`). The recovered/rebuilt
status is flagged on the `Document` (a `recovery: Option<RecoveryReport>`
field), disclosed in the CLI + GUI, and counted (R20) — recovery is a
reviewable fact, never a silent repair (fuzzy-never-sneaky).

**Interaction with §5.6 "never normalize" (stated explicitly so a future
reader does not think recovery breaks R33):** §5.6 governs *clean
passthrough* objects — it forbids reformatting a file the operator loaded
intact. It does **not** bind a recovered file: the base was invalid, so
emitting a fresh normalized classic xref (`SectionShape::Classic { xref_stm:
None }` — the most compatible form) is the correct, honest output, not a
normalization violation.

**Why this never perturbs a clean file:** recovery triggers **exclusively on
the strict-load error path** (`document.rs::from_bytes` only invokes it when
`load_xref_chain` / `probe_header` returned `Err`). A file that loads cleanly
never enters recovery code, so the round-trip/minimal-diff invariant for
clean files (§5.1) is preserved **by construction**, not by policy. Full
record: `docs/decisions/013-xref-recovery.md`; standing rule R67.

### 5.11 In-place text editing is surgery-under-incremental-save, NOT a fourth forced-full-rewrite sibling (decision 014 — SHIPPED 2026-08-01, Pass 14.0–14.3 all COMPLETE)

*(Added 2026-08-01 as a forward-looking design note ahead of Pass 14.1;
FLIPPED to shipped/active 2026-08-01 on decision 015's filing — all four
Pass 14.x slices are now shipped (see `ROADMAP.md` Shipped). This section
records the actual module layout, mirroring how §5.10 was rewritten on
Pass 13b's ship.)*

§§5.2/5.9/5.10 (R35/R58/R67) form a **forced-full-rewrite family**: every
member exists because incremental save structurally *preserves* superseded
content, which is disqualifying for redaction, scrub, and a recovered-base
save alike. **In-place text editing is confirmed NOT a fourth member of
that family.** Editing is a content *change*, not a removal or a
recovery — it uses the project's **default** incremental save (R36/R70),
and prior text surviving in history is a disclosed, accepted consequence,
not a defect. Truly removing text remains Redaction's job (§5.2/R35);
conflating the two would either weaken redaction's absence guarantee or
force every keystroke through a full rewrite that drops revision history
for no security reason.

**Shipped module layout.** `crates/pdfce-core/src/text_edit/`:

- **`model.rs`** — the derived Run→Line→Block hierarchy over Pass 4's
  extraction (`Block`/`Line` with union `bbox`, `line_indices`, `column`;
  `BlockRecognitionOptions` — `column_overlap_ratio`,
  `paragraph_leading_ratio`, `indent_ratio`, `line_baseline_ratio`;
  `BlockDiagnostics` counting every inference, R72). `line_at`/
  `word_range_at`/`line_range_at`/`word_bounds` accessors (added Pass 14.3
  for caret/selection navigation).
- **`edit.rs`** — the advance-preserving REMOVE→REPLACE content-stream
  surgery (extends Pass 8.0's `redact.rs` interpreter, R69/R47), the
  inverse-encoding builder (Unicode→code, inverting Pass 4's §9.10.2 decode
  ladder), `FollowerDisposition` (same-line relayout past the original
  margin, disclosed), `EditReport.disclosures` (verbatim-surfaced
  refusals/warnings), and the R-INV-1..8 font-on-edit gate (R71) keyed on
  `GlyphSource` + glyph presence (decision 012). Also owns `EditSession` —
  the undo/redo command log — split as `plan_edit(...) -> EditPlan` /
  `plan_format(...) -> FormatPlan` (shared by both the free-function path
  and the session path) + `write_incremental`; `CommandKind::{EditText,
  FormatText}` (Pass 14.3 addition; `ReflowBlock` is Pass 15.1's addition,
  see §12's decision-015 entry) apply as ONE undo-able command each over
  the session's in-memory object graph, proven byte-identical to the free
  function for a single edit.
- **`format.rs`** — formatting-on-selection (Pass 14.2): size (`Tf`), fill
  colour (`rg`/`g`/`k`, storing the operator's actual chosen colour space —
  RGB/CMYK/gray — unlike Acrobat, which always stores `DeviceRGB`
  regardless of the picker mode shown), gated font-family/style change
  (re-encode into an available covering face, else refuse-and-disclose).
- **`vartext.rs`** — reused verbatim for reflow line-breaking (Pass 15.x);
  not itself part of the 14.x edit path but the shared line-breaking
  substrate.

**GUI (`pdfce-gui`, Pass 14.3):** `CanvasTool::TextEdit` — click→caret,
Shift-click→extend, double-click→word, drag→select; `TextEditState`/
`PendingEdit` in `main.rs`; live preview (mask + draft text + a dashed
"PREVIEW — not yet applied" tag), Accept/Reject buttons, the verbatim
disclosure/refusal strips, a read-only block-boundary review overlay, and
the property bar (size / colour-model / font, trust-labelled per R63).
`ui_text.rs` carries the ~30 new user-facing strings. Deferred, named
non-goals: triple-click/arrow-Home-End caret nav (accessor plumbing already
shipped, wiring deferred), split/merge/reorder of recognized blocks,
commit-on-focus-loss for the property bar (an explicit Apply button is used
instead).

The edit mechanism **is** content-stream surgery — the second sanctioned
page-content-rewriting operation after Pass 8.0's redaction interpreter
(R47's surgery-vs-overlay line), extended from REMOVE to REPLACE. It reuses
Pass 8.0's §9.4.4 advance-preservation machinery so un-edited same-line
text does not slide. The crux design call is **font-on-edit**: a keystroke
is applied only when the run's font can already supply the glyph (an
embedded program's existing glyphs, or a non-embedded font's full
bundled/supplied coverage per decision 012); a glyph an embedded *subset*
lacks is refused-and-disclosed by name, never faked or silently substituted
(R71). Block recognition is derived, counted, reviewable structure over
Pass 4's extraction output — never authoritative, never a silent re-layout
(R72; reflow itself is Pass 15.x, see below). An edit inside a
marked-content sequence preserves its BDC/EMC + MCID wrapper and discloses
staleness rather than corrupting the structure tree the way Acrobat's own
in-place edit is known to (R73) — minimal-diff turned into an
accessibility guarantee.

**Not a fourth forced-full-rewrite sibling, confirmed by the shipped
gates.** Every Pass 14.x ship re-verified `cargo tree -p pdfce-core` /
`-p pdfce-render` zero egui/eframe/winit/wgpu/glow (GUI-core separation
intact); the round-trip/R46 gate stays green for untouched objects; only
the edited content stream(s) (+ changed resource/font dict) are re-emitted.

**Forward pointer — reflow (FF-A) is a separate Pass family, not an
extension of 14.x's module boundary.** Decision 015 (2026-08-01) scopes
within-block offline reflow as `ROADMAP.md`'s ★ Pass 15.x, building a
`ReflowEngine`/`ReflowPreview` on top of this same `text_edit::model`/
`edit` substrate (15.0 read-only engine, 15.1 surgery +
`CommandKind::ReflowBlock`, 15.2 canvas UI). Reflow remains an *opt-in*
beside — never a replacement for — the default single-line relayout
described above (R75). Full design: `docs/decisions/015-ffa-within-block-offline-reflow.md`;
decision-log entry below.

Full design, the four-case font-on-edit matrix, the fast-follow ladder
(FF-A offline reflow ladder through FF-H spacing/synthetic-styles — FF-A/
FF-B boundary amended by decision 015, see below), and the six standing
rules (R69–R74) are in `docs/decisions/014-acrobat-text-editing.md`; Pass
slicing (14.0 read-only model → 14.1 edit+relayout+font-gate → 14.2
formatting → 14.3 canvas UI) and its Shipped records are in `ROADMAP.md`.

## 6. Packaging: single-folder portable

- **Platform scope (decided 2026-07-30, decision 003 §4.1 — no longer
  a default):** v1 ships **Windows 10/11 x86_64 only**, as a
  deliberate scope decision. The codebase stays platform-clean at all
  times (no `#[cfg(target_os)]` in `pdfce-core`/`pdfce-render`, rule
  R10), verified continuously by cross-target `cargo check` CI for
  macOS-arm64 and wasm32 — a compile signal, never a support claim
  (rule R9). See `docs/decisions/003-distribution-posture.md` for the
  full reasoning, the macOS/Linux gating triggers, and the
  CLI-first-via-musl rule if Linux ever ships.
- No installer. Build produces `pdfce.exe` (Windows first target) plus
  whatever DLLs/assets are needed, all in one output folder.
- **Payload/user-state partition (decision 003 R15, binding from the
  first Pass that persists anything):** the distribution folder is
  split into replaceable payload (binaries, assets,
  `THIRD_PARTY_LICENSES.md`, README) and user state (settings,
  recents, later OCR data) in a clearly named location — because the
  documented update procedure is "replace the folder," and replacing a
  folder destroys whatever the user kept in it. User state never sits
  loose among the binaries; the update instructions name exactly which
  files to keep. The packaging smoke test verifies the partition.
- No registry writes, no `%APPDATA%` requirement for the app to run
  (per-user settings/recents may still use a conventional config dir,
  but the app must run read-only-folder-clean with no config present).
- Verify every packaging pass with a **real smoke test**: zip the
  output folder, unzip it to an unrelated path (e.g. a fresh temp
  dir), launch from there with no prior install step, confirm it
  opens and renders a fixture PDF. This is the packaging equivalent of
  MatExtractor's "smoke-import MainWindow" rule — don't claim a
  packaging pass done without actually running the copied folder.

## 7. CLI capabilities (`pdfce-cli`)

pdfce ships a real command-line interface alongside the GUI, not as a
debug afterthought. Design points:

- **Same crate-separation discipline as the GUI.** `pdfce-cli` depends
  on `pdfce-core` + `pdfce-render` exactly like `pdfce-gui` does, and
  is held to the same zero-GUI-dependency-in-core invariant (§3) — the
  CLI's existence is itself proof that invariant is doing its job:
  two completely different front ends, one shared core, no logic
  duplicated.
- **Subcommand shape** (`clap`-based, final surface scoped alongside
  each feature's own Pass — see `docs/ROADMAP.md`): one subcommand per
  batch operation, e.g. `pdfce-cli merge a.pdf b.pdf -o out.pdf`,
  `pdfce-cli extract-pages in.pdf 3-7 -o out.pdf`, `pdfce-cli
  bates-stamp *.pdf --start 1 --format "DOC-{:06}"`, `pdfce-cli
  to-pdfa in.pdf --level 2b -o out.pdf`, `pdfce-cli validate-pdfa
  in.pdf` (prints a conformance report, non-zero exit on failure —
  scriptable in CI/document-pipeline contexts), `pdfce-cli sign in.pdf
  --cert cert.p12 -o out.pdf`, `pdfce-cli render-page in.pdf 3 -o
  page3.png --dpi 150`.
- **Exit codes matter.** Since this is meant to be genuinely scriptable
  (unlike Acrobat, which has no real CLI), follow normal Unix
  conventions: `0` success, non-zero on any failure, with a specific,
  documented meaning per non-zero code where it's useful for a calling
  script to distinguish failure modes (e.g. "input not found" vs
  "encrypted, no password given" vs "PDF/A validation failed").
- **Same round-trip / redaction / fuzzy-never-sneaky invariants apply.**
  A CLI redact command must truly remove content, same as the GUI
  path (§5); a CLI OCR command's output is still a hint the caller
  chooses to apply, not silently baked into the saved file, unless an
  explicit `--apply` (or similarly unambiguous) flag says otherwise.
- **Packaging**: `pdfce-cli.exe` ships in the same single output
  folder as `pdfce-gui`'s executable — one portable folder, two
  entry-point binaries, both zero-install. The packaging smoke test
  (§6) covers both.

## 8. Code style & public API design

`pdfce-core`'s public API (and, downstream of it, `pdfce-cli`'s
argument/output design) follows the official Rust ecosystem
conventions, not an invented house style:

- **Formatting** — the Rust Style Guide, enforced via `cargo fmt`.
- **API design** — the Rust API Guidelines checklist (naming
  conventions, trait derives, error-type design, documentation,
  predictability, type safety).

Full condensed reference, kept up to date as a cross-project resource
(useful to any future Rust project, not just pdfce):
`D:\dev\rag\rust\rust-style-guide-and-api-guidelines.md`. This is a
binding engineering discipline, not a style preference — see
`.claude\agents\pdfce-engineer.md` §"Code style & API design
discipline" for the enforcement mechanics (`cargo fmt --check` and
`cargo clippy -- -D warnings` clean before any Pass ships).

## 9. Open-source dependencies & attribution

pdfce builds on the existing Rust/OSS ecosystem rather than
reinventing every primitive — see `docs/PRIOR_ART.md` for the
survey/decision record and `docs/LEGAL.md` §6 for the binding
licensing discipline (permissive-vs-copyleft classification, the
mandatory per-dependency license check, and why pdfce's own license
gates which prior art is even usable). Attribution for whatever's
actually adopted is **generated**, not hand-maintained — `cargo-about`
produces `THIRD_PARTY_LICENSES.md` from the real `Cargo.lock`,
regenerated at every packaging pass (§6).

**pdfce's own license is MIT (decided 2026-08-01, `LEGAL.md` §1; see
§12 decision log).** `LICENSE` (repo root) + `license = "MIT"` in
`Cargo.toml` `[workspace.package]`, inherited by all four member
crates via `license.workspace = true`. Every current dependency is
permissive (verified against `THIRD_PARTY_LICENSES.md`), so this
decision required no dependency rework. **Consequence: GPL/AGPL prior
art (MuPDF, Poppler, Ghostscript) is now categorically, permanently
excluded as a real dependency** — reference-only (architecture/
algorithms studied, never linked or copied), per `LEGAL.md` §6.1.

## 10. Adversarial input hardening & fuzzing

`pdfce-core` parses files from the public internet by design — every
PDF it opens must be treated as **untrusted, potentially adversarial
input**, not just "possibly malformed." This is a real gap identified
2026-07-23: the project justified choosing Rust partly on this basis
(§2) but had never written down what that actually requires structurally.

### 10.1 Resource-limit guards (decompression-bomb defense)

Every filter decoder (`FlateDecode`, `LZWDecode`, `CCITTFaxDecode`,
`JBIG2Decode`, `DCTDecode`, `JPXDecode`) **must** enforce a maximum
decoded-output-size cap before/while decoding, not just check the
result afterward — a few KB of compressed input can expand to
gigabytes (classic zip-bomb pattern), and PDF's filter chaining
(e.g. `ASCII85Decode` → `FlateDecode` → raw image data) can compound
this. Concretely:

- Every decoder takes an explicit output-size ceiling (a sane default,
  overridable) and returns an error rather than continuing once
  exceeded — never silently truncate, never allocate unbounded.
- Object/dictionary nesting (page tree, `Kids` arrays, `Resources`
  inheritance chains, annotation appearance-stream references) needs
  cycle detection and a depth cap — a maliciously crafted circular
  reference must fail cleanly, not hang or stack-overflow.
- Content-stream interpretation (path construction, clipping, nested
  `Form XObject`/`q`/`Q` graphics-state pairs) needs an operation-count
  or time budget per page — pathological but syntactically valid
  content streams (e.g. millions of degenerate path segments) must not
  be able to hang the renderer indefinitely.
- Object counts / xref table size get a sanity ceiling too (a
  100 MB file claiming 500 million objects is lying).
- **Concrete instance (added 2026-07-30, Pass 1.1 slice): recursive
  Form-XObject execution (`Do`) gets `MAX_XOBJECT_DEPTH` = 64,
  corpus-measured, not guessed.** An initial guard of 16 (intuition)
  overflowed on exactly one of 2,914 veraPDF/PDF-Association corpus
  files — a **conformant** 32-deep chain
  (`veraPDF-corpus/PDF_A-1b/6.1 File structure/6.1.12 Implementation
  limits/veraPDF test suite 6-1-12-t08-pass-c.pdf`, objects 19–50).
  Annex C sets no form-nesting limit and PDF/A §6.1.12 forbids a
  reader from imposing Annex C limits anyway. Raised to 64 (2× the
  deepest conformant structure measured); corpus-wide overflows are
  now 0. This is the SECOND guard in this project caught by the
  veraPDF §6.1.12 implementation-limits suite (the first was
  `MAX_TOKEN_LEN`) — see the `ROADMAP.md` standing rule requiring
  every new resource guard to be run against that suite specifically
  before shipping.

### 10.2 Fuzz-testing (required, not optional, before Pass 1 ships)

Set up a `cargo-fuzz` target against the tokenizer/object-parser as
part of Pass 1 (add explicitly to its acceptance criteria in
`docs/ROADMAP.md` if not already there by the time Pass 1 starts).
Minimum scope for the first fuzz target: raw byte-stream → tokenizer
→ COS object parser, asserting only "never panics, never hangs past a
bounded timeout, never allocates past a bounded ceiling" — not
semantic correctness (that's what the fixture-based tests in §5/§9
cover). Expand fuzz targets to each filter decoder as they're
implemented. Treat any fuzz-discovered crash as a release blocker for
the Pass that introduced the vulnerable code path, not a "file it and
move on" backlog item.

### 10.3 Where this lives in the codebase

Guard logic (size ceilings, depth counters, timeouts) belongs in
`pdfce-core` itself — not bolted on as a wrapper in `pdfce-gui`/
`pdfce-cli` — so both front ends (and the future WASM fork) inherit
the same hardening automatically. Document each guard's default limit
and rationale in the doc comment of the function it guards, per the
documentation-first rule; a reader should understand *why* the number
is what it is (e.g. "1 GiB default output cap — larger than any
legitimate single decoded PDF stream this project has seen, small
enough that hitting it can't exhaust a typical machine's memory").

**Amendment (2026-07-30, Pass 1.1 slice):** the principle stated above
("guards belong in `pdfce-core`") is precise for **parse-time**
recursion (page-tree walk, xref/ObjStm cycles) but not for
**render-time** recursion — a Form XObject's recursive `Do` execution
happens inside `pdfce-render`'s content-stream interpreter (§3's
implementation note), which `pdfce-core` has no visibility into (it
only ever sees one content stream's tokens at a time, never resolves
`Do` itself). `MAX_XOBJECT_DEPTH` therefore lives in `pdfce-render`.
Both front ends (and the WASM fork) still inherit it automatically,
because both depend on `pdfce-render` for any rendering at all — the
"automatically inherited by every front end" property is what actually
matters, not which of the two GUI-agnostic crates holds the constant.
General rule going forward: a guard against adversarial input lives in
whichever of `pdfce-core`/`pdfce-render` actually performs the
recursive/expanding operation being guarded.

## 11. Undo/redo architecture

Identified as a real design gap 2026-07-23: the UI standing rule
"every edit is undoable" (see `pdfce-ui-specialist.md`) was never
reconciled with the round-trip/minimal-diff invariant (§5) — the two
interact in a way that needs an explicit mechanism, not just a UX
promise.

### 11.1 The core design: command log over the in-memory object graph, diffed at save time

- Every edit the user makes is represented as a small command object
  (`PendingEdit` or similar) with `apply()` and an inverse/`revert()`,
  operating on `pdfce-core`'s **in-memory** `Document` object graph —
  never on file bytes directly, and never on the saved file at all.
  The undo stack holds these commands.
- The **original loaded byte buffer / object graph is retained
  unmodified** as the "base revision" for the life of the open
  document (this is already required by §5 for lazy round-trip
  passthrough — undo reuses the same retained state, doesn't add a
  new one).
- Undo/redo operates **entirely pre-save**: hitting Undo reverts the
  in-memory graph via the command's inverse. It has no relationship to
  what's on disk until the user actually invokes Save.
- **Critical rule**: the "dirty set" (which objects actually differ
  from the base revision, i.e. what an incremental save must include)
  is computed as a **structural diff against the base revision at save
  time** — it is *not* the union of every object any command ever
  touched during the session. If a user edits an object and then
  undoes that specific edit before saving, that object must **not**
  appear in the incremental update, because compared to the base
  revision nothing net changed. Tracking "was this object touched
  by history" instead of "does this object currently differ from
  base" would silently violate the minimal-diff promise the moment
  undo is involved — this is exactly the subtle bug this section
  exists to prevent someone from introducing.
- Redo stack is invalidated (cleared) the instant a new edit is made
  after an undo — standard editor behavior, stated here for
  documentation-first completeness, not because it's subtle.
- Bound the undo history (a configurable max operation count) rather
  than keeping it unbounded — large documents with long editing
  sessions shouldn't accumulate unbounded command-object memory.
  Acrobat itself bounds undo; matching that expectation is fine.

### 11.2 Redaction is the deliberate exception, and only after save

Redaction's true-content-removal behavior (§5 corollary) is
undo-able **like any other edit, right up until the document is
saved**. Once a redaction has actually been written to disk (the
underlying content genuinely gone from the saved bytes), that save is
not reversible by "Undo" in a later session — there is no data left
in the file to restore. This matches real-world expectation (redact +
save = permanent) and is exactly why the UI standing rule requires an
explicit, honest confirmation dialog for redaction specifically
(`pdfce-ui-specialist.md`) — the operator needs to understand *before*
saving that this is the one edit type Undo can't rescue them from
after the fact.

**Cross-reference added 2026-07-31 (Pass 3.0, decision 007 W2/R35):**
the *save-side* half of this exception is now specified in **§5.2**.
Redaction must force a **full rewrite** and must **refuse incremental
save**, because incremental save structurally preserves superseded
content — the old bytes of every replaced object stay in the file by
construction (§7.5.6). Without that rule, a redaction saved in pdfce's
*default* mode would leave the redacted content trivially recoverable,
and the confirmation dialog this section describes would be promising
something the writer did not deliver. §5.5 records the resulting
conflict with signed documents, which is a genuine operator either/or.

**Correction cross-reference added 2026-07-31 (Pass 3.1):** forcing a
full rewrite is NOT by itself enough to make redacted content
"genuinely gone from the saved bytes" when the redacted object lives
in an object stream — containers carry through verbatim in both save
modes, so the Redaction Pass must also rewrite/decompose the
containers holding redacted objects. See §5.7.

### 11.3 Snapshot fallback for bulk structural edits

The command-pattern model is the default for content-level edits (text,
annotations, form fields, single-page operations). For bulk structural
operations where per-item commands would be awkward (e.g. reordering
50 pages in one drag operation), a coarser "before/after page-order
snapshot" command is an acceptable specialization of the same pattern
— still one undo-stack entry, still diffed against the base revision
at save time via the same mechanism. Don't invent a second, parallel
undo system for this case.

### 11.4 Scope for Pass planning

Read-only Passes (Pass 1) need none of this. **The first Pass that
introduces any editing capability must build the command-log/undo-
stack mechanism as part of that Pass, not after** — retrofitting undo
onto edit code that was written assuming direct mutation is
significantly more expensive than designing it in from the first edit
feature. Flag this explicitly when `docs/ROADMAP.md` scopes the first
editing Pass.

### 11.5 Implementation record — the overlay design (Pass 3.1, 2026-07-31)

*(§11.4's obligation bound at Pass 3.1 — the first editing Pass — and
was honored: the mechanism below shipped in that Pass, not after.
This section records the shape actually built, so §11.1's design
prose and the code stay reconcilable.)*

- **`EditSession` command log** (`crates/pdfce-core/src/edit.rs`,
  1,608 lines): every edit is a command with apply/revert, exactly as
  §11.1 specifies — operating on an **overlay** above the base
  revision, never on the base object graph and never on file bytes.
  The base revision (buffer + parse) stays untouched for the life of
  the open document; the overlay holds only the objects that
  currently differ.
- **The dirty set is derived, not accumulated:** at save time the
  overlay yields a `DirtySet` (replacements + trailer patch +
  `changes_content`) as a diff against the base revision. An
  edit that has been undone leaves no trace in the overlay, so it
  cannot appear in the save — §11.1's "union of every command ever
  run" bug is structurally unexpressible, and executably pinned:
  **edit → undo → save is byte-identical, 2,897/2,897 corpus files
  (100%)**, plus dedicated fixture tests including a 12-command
  history and undo → redo → save.
- **One writer path:** both save modes take `&DirtySet`;
  `DirtySet::empty()` is Pass 3.0's identity writer as a strict
  pinned subset (§5.7).
- **Undo granularity matches operator intent:** the GUI applies edits
  on button press, not per keystroke — one undo step per intent, so
  the stack holds meaningful operations (a deliberate Pass 3.1
  decision, §12 continuation-18 entry).
- Redo invalidation on new-edit-after-undo behaves as §11.1 states.

## 12. Decision log

Append-dated entries here whenever an architectural decision is made
or revised. Don't rewrite history — if a decision changes, add a new
dated entry noting the change and why; leave the old entry in place
with a forward pointer.

- **2026-07-23** — Project bootstrap. Language: Rust. GUI: egui/eframe
  recommended (confirm at Pass 0). Target parity product: Adobe
  Acrobat Pro. PDF-spec RAG location: `D:\Dev\Rag-Specialized\PDF_Spec\`.
  Agents created: pdfce-engineer, pdfce-librarian, pdfce-spec-librarian,
  pdfce-ui-specialist.
- **2026-07-23 (same-session amendment)** — Added `pdfce-cli` as a
  first-class crate (§3, §7) per explicit user request: pdfce ships
  CLI batch capabilities from the start, not just a GUI. Added Rust
  Style Guide + API Guidelines as a binding, cross-project-referenced
  engineering discipline (§8), backed by a new reference file at
  `D:\dev\rag\rust\rust-style-guide-and-api-guidelines.md`. Corrected
  the cross-project-knowledge-base plan: Rust/egui/wgpu findings
  belong in the existing `D:\dev\rag\rust\` / `D:\dev\rag\egui\`
  Cross-project Tool RAG (registered in the user's global CLAUDE.md),
  not a new `personal_rag/rust` subject — `personal_rag/pdf` remains
  correct as-is for PDF-domain-specific empirical findings.
- **2026-07-23 (same-session amendment 2)** — Added a second reference
  RAG, `D:\Dev\Rag-Specialized\Acrobat_Features\`, cataloging Adobe
  Acrobat Pro's feature set (capability/behavior/edge-cases/limits)
  to ground `ROADMAP.md` acceptance criteria — explicitly excludes
  Acrobat's GUI mechanics, since pdfce's UI is designed independently
  by `pdfce-ui-specialist`. New agent `pdfce-acrobat-librarian` owns
  it. Also established a binding cross-RAG format rule: every RAG
  this project builds or writes to (`PDF_Spec`, `Acrobat_Features`,
  `D:/dev/rag/rust`, `D:/dev/rag/egui`, the future `personal_rag/pdf`)
  is written for LLM consumption only, not human reading — dense,
  schema-consistent, no narrative padding.
- **2026-07-23 (same-session amendment 3)** — Added §9, open-source
  dependencies & attribution, per user request to survey existing OSS
  prior art and ensure proper crediting. New `docs/PRIOR_ART.md`
  (research/decision record) + `LEGAL.md` §6 (binding licensing
  discipline: permissive-vs-copyleft classification, per-dependency
  license check before any `Cargo.toml` addition, generated
  `THIRD_PARTY_LICENSES.md` via `cargo-about` rather than hand-
  maintained attribution). Three research passes launched to survey
  core PDF crates, supporting codec/font/crypto crates, and existing
  full OSS PDF tools; findings pending synthesis into `PRIOR_ART.md`.
- **2026-07-23 (same-session amendment 4)** — Research synthesized
  into `docs/PRIOR_ART.md`. **Open question flagged, not yet decided:**
  `oxidize-pdf` (MIT) may already cover most of pdfce-core's target
  scope — needs a dedicated audit (round-trip fidelity, signature-safe
  incremental saves, PAdES signing) before Pass 1 locks in a
  from-scratch `pdfce-core`. Confirmed: pure-Rust filter answers now
  exist for JBIG2/CCITT/JPX (`hayro-*` crate family) — the "problem
  filter" set from §7's plan is resolved. Confirmed gap: no Rust crate
  does signature-safe incremental saves or PAdES signing — build from
  `cms`+`x509-cert`+RustCrypto. `tiny-skia` selected as the
  `pdfce-render` rasterizer. MuPDF and Ghostscript flagged as AGPL-3.0
  licensing landmines — never link without a deliberate, user-
  confirmed decision. Competitive landscape confirmed clear (§1).
- **2026-07-23 (Pass 0 — workspace bootstrap)** — Pass 0 shipped (see
  `ROADMAP.md` Shipped, `SESSION_LOG.md` 2026-07-23 Pass 0). Decisions
  recorded this Pass:
  - **(a) GUI toolkit CONFIRMED: egui/eframe over iced** (user
    decision). Closes the §2.1 open question — egui/eframe was a strong
    default there, now a closed decision. Reversal cost (rewriting the
    GUI crate) no longer looms over subsequent Passes.
  - **(b) `oxidize-pdf`: decision DEFERRED, gate unchanged.** The user
    chose a thin, header-probe-only `pdfce-core` for Pass 0 (just the
    `%PDF-` version probe, no COS parser), specifically to defer the
    build-from-scratch-vs-adopt-`oxidize-pdf` foundation decision. That
    audit remains the gate BEFORE Pass 1 (see `ROADMAP.md` Pass 1 GATE
    note and `PRIOR_ART.md`'s OPEN QUESTION) — Pass 0 did not resolve it
    and did not lock in a from-scratch core.
  - **(c) Rendering backend = GLOW, not wgpu.** eframe 0.35's default
    backend is now wgpu, but the current wgpu 29.0.4 stack FAILS TO
    COMPILE on `x86_64-pc-windows-msvc`: `wgpu-hal` 29.0.4 depends on the
    `windows` crate 0.61.2 while `gpu-allocator` 0.28.0 depends on
    `windows` 0.62.2, and their D3D12 `ID3D12Heap` types are mutually
    incompatible across the two `windows-core` versions
    (`windows_core::imp::CanInto` trait mismatch in
    `CreatePlacedResource`). §2's stack table already specified "`wgpu`,
    falls back to `glow`/OpenGL if needed" — choosing glow here
    exercises that documented fallback, so this is a **routine
    engineering call within the pre-authorized §2 design, not a
    reversal**. glow is also lighter for single-folder packaging.
    Revisit wgpu when the upstream `windows`-crate versions realign.
    (Full version-stamped finding: `D:\dev\rag\egui\`.)
  - **(d) Toolchain pinned: 1.97.1; edition 2024; resolver 3; MSRV
    (`rust-version`) = 1.92; `Cargo.lock` committed.** The MSRV of 1.92
    is driven up from the edition-2024 language floor of 1.85 by
    `eframe` 0.35, which requires rustc 1.92 — exactly the "candidate
    dependencies may force pdfce's MSRV higher than the edition floor"
    re-check that §2.1a anticipated. Sets the §2.1a toolchain/lockfile
    policy into concrete values.
  - **(e) Static CRT on Windows.** `.cargo/config.toml` sets
    `[target.x86_64-pc-windows-msvc] rustflags =
    ["-C","target-feature=+crt-static"]` so the binaries statically link
    the MSVC CRT and need no VC++ redistributable — directly serves §6's
    single-folder, no-system-wide-runtime-dependency requirement.
    Verified via `dumpbin /dependents` (only OS DLLs remain). (Full
    finding: `D:\dev\rag\rust\`.)
  - **(f) `.gitattributes` hardened for binary fixtures.** `*.pdf`,
    `*.bin`, `*.png`, etc. marked `binary` so Git's EOL normalization
    cannot corrupt byte-offset-sensitive binary PDF fixtures on
    checkout — a real risk for the §5 round-trip / byte-exact invariant
    once fixtures land. (Full finding: `D:\dev\rag\rust\`.)
- **2026-07-30 — `oxidize-pdf` gate CLOSED: decision (c)
  reference-only; `pdfce-core` is built from scratch.** Decided via
  the KenAgent decision protocol; full record at
  `docs/decisions/001-oxidize-pdf-adopt-vs-build.md` (audit of
  `bzsanti/oxidizePdf` HEAD `5f3e8b3`, v4.2.1, MIT). Closes the §12
  entry (b) of 2026-07-23 ("decision DEFERRED") and `ROADMAP.md`
  Pass 1's GATE. `oxidize-pdf` becomes MIT prior art plus an
  out-of-tree differential test oracle (`tools/difftest/`,
  `[workspace]`-excluded, pinned version, advisory-never-authoritative,
  fixtures never sourced from its repo) — never a shipping dependency
  (`cargo tree` on all four crates must never show it), no fork or
  vendor, zero literal ports planned. Maintained permissive crates are
  preferred over any vendoring: `hayro-jbig2`, `hayro-ccitt`,
  `subsetter` (fallback `allsorts`), RustCrypto stack; `flate2`
  (`miniz_oxide`/`zlib-rs` backend only) + `tiny-skia` adopted at
  Pass 1 (decision 001 §6.2). The decision creates **six Pass-1
  architecture obligations** (decision 001 §6.1 — all land in Pass 1
  even though Pass 1 is read-only, because each is
  expensive-to-impossible to retrofit):
  1. **`ByteSpan` provenance is a first-class field** on every parsed
     indirect object (retained source buffer; a span-backed object
     structurally equal to the base revision re-emits its source bytes
     verbatim on full rewrite, or is omitted on incremental save).
  2. **Lossless content-stream token model with per-token byte
     spans**; the semantic operator view is a *projection* over the
     tokens, never the primary representation.
  3. **ONE object model** — exactly one `Document` type that is
     simultaneously the parse result and the write source, to be
     recorded in §4 as a named invariant.
  4. **Fail-clean filters as a type-level contract**: every decoder
     returns `Result<Vec<u8>, FilterError>`; no code path returns
     undecoded or partial bytes on failure; one
     corrupted-stream→`Err` regression test per filter.
  5. **Lint policy**: `#![deny(clippy::unwrap_used,
     clippy::expect_used, clippy::panic, clippy::indexing_slicing)]`
     in `pdfce-core` (allowed under `#[cfg(test)]`).
  6. **No output fingerprint**: `/Info` untouched on incremental save
     unless the operator actually changed metadata; on full rewrite
     only, `/Producer` is set to `pdfce <version>`, documented, and
     overridable via API and CLI flag.
  The engineer integrates the full design text into §4/§5 when
  implementing Pass 1; until then the decision record is the
  authoritative design source for these obligations.
- **2026-07-30 — i18n/l10n architecture decided (decision 002; second
  use of the KenAgent protocol).** Full record:
  `docs/decisions/002-i18n-timing.md`. Resolves (and supersedes) the
  "Internationalization/localization" deferred bullet from
  `ROADMAP.md`'s product-scope list (2026-07-23), before the first
  real UI strings are written (Pass 1 — the record's stated point of
  no return). Outcome:
  - **Centralized, zero-dependency, function-based string catalog**:
    `crates/pdfce-gui/src/ui_text.rs` is the single home of every
    user-facing GUI string from Pass 1 onward; entries are `pub fn`
    (never `pub const` — the function signature is what makes a future
    catalog-backed retrofit a one-file, zero-call-site change).
    English-only; no locale detection, no i18n crate. Enforced by a
    new `ui-strings` CI job (whitespace-bearing string literals in
    `pdfce-gui` outside `ui_text.rs` fail the build unless
    `// ui-text-exempt: <reason>`).
  - **Eight standing rules R1–R8** added to `ROADMAP.md`'s Standing
    rules — the discipline (no sentence assembly, no English-width
    layout, formatting helpers, no i18n dep without a §9 trigger,
    `gettext-rs` pre-disqualified for LGPL-static-link-on-Windows) is
    the actual deliverable; the catalog module is the cheap part.
  - **`pdfce-cli` is English-only PERMANENTLY, by design — not
    deferred** (R5): clap 4.6 hardcodes its own headings/error prose
    with no i18n API (clap-rs/clap#380, open since 2016), and a
    localized scripting interface is a hazard. The binding contract:
    **stdout is locale-invariant machine output, permanently** — never
    varies with `LANG`/`LC_ALL`; human diagnostics go to stderr; the
    §7 exit-code contract is likewise locale-invariant.
  - **`pdfce-core`/`pdfce-render` errors are never localized (R4)** —
    and in exchange every error variant must carry the **structured
    data** the message is rendered from, never pre-formatted prose
    `String`s. This is the record's one genuinely irreversible item
    and binds the substantial new error variants decision 001's §6.1
    obligations add in Pass 1 (`FilterError`, xref/object-model parse
    failures). `Display` stays English/diagnostic/stable per
    C-GOOD-ERR (§8); front ends own presentation.
  - **Non-Latin text *inside* PDF documents is NOT deferred (R7)** —
    `pdfce-render`'s own text stack, entirely separate from epaint's
    (which lacks bidi and CJK fonts; see the related Backlog entry
    "UI font coverage for non-Latin file paths and document metadata"
    and `D:\dev\rag\egui\epaint_0.35_text_stack_i18n_limits.md`).
  The engineer implements the record's §10 items 1–5 in Pass 1
  (ui_text.rs, CI job, CLI module-doc paragraph, R4 adherence); until
  §4/§7 body text is updated the decision record is the authoritative
  design source, same convention as decision 001 above.
- **2026-07-30 — Distribution posture decided (decision 003; third use
  of the KenAgent protocol).** Full record:
  `docs/decisions/003-distribution-posture.md`. Resolves (and
  supersedes) the final two `ROADMAP.md` deferred bullets
  ("Cross-platform scope beyond Windows first" and "Update/release
  mechanism") — that list is now **empty**. Outcome:
  - **v1 ships Windows 10/11 x86_64 and nothing else — a deliberate
    scope decision, not an accident of where the project started**
    (§6's "Windows first" parenthetical is now a decision with this
    record as authority, not a default). The codebase stays
    platform-clean at all times (R10: no `#[cfg(target_os)]` in
    `pdfce-core`/`pdfce-render`, ever), verified continuously by
    **cross-target `cargo check` CI on the existing ubuntu runner
    instead of new runners**: `aarch64-apple-darwin` (32 s, no SDK
    needed — `check` type-checks, never links) plus a **positive
    web-fork invariant check** — `pdfce-core` + `pdfce-render` must
    compile for `wasm32-unknown-unknown` (6.5 s; §3's crate split
    checked positively for the first time, not just by absence of GUI
    crates). No macOS runner (unactionable red CI with no Mac
    hardware; macOS's real cost is Gatekeeper/notarization/$99-year/
    hardware, not the build). If Linux ever ships: `pdfce-cli` first
    as static musl, `pdfce-gui` separately as glibc-dynamic — Linux is
    the *heaviest* dependency target (237 crates vs Windows 147) and
    the GUI cannot be musl-static (all Linux windowing bindings are
    `dlopen`-based; musl has no `dlopen` in static builds).
  - **R12 — no network client in the tree, fail-closed.** New
    `no-network` CI job (cargo-tree denylist against the SHIPPED
    Windows target); unlocking requires a new decision record;
    `pdfce-core`/`pdfce-render` may never contain network code under
    any future decision. §1.1's privacy posture stops being a promise
    and becomes a build gate, verifiable by a skeptical reader via
    `THIRD_PARTY_LICENSES.md`.
  - **R13 — pdfce never self-updates. Permanent.** Manual
    replace-the-folder is the only update mechanism; discovery is
    delegated to Scoop-then-WinGet manifests (gated on `LEGAL.md` §1;
    see `ROADMAP.md`'s "Release & distribution channel" Backlog
    entry). An in-app *checker* is deferred behind the record's
    complete §6.4 spec, requiring a new decision record (needs an
    HTTP client, which R12 forbids).
  - **R15 — the distribution folder is partitioned** (replaceable
    payload vs user state) from the first Pass that persists anything,
    because folder-replace destroys everything in the folder — decided
    now while it costs zero, expensive to retrofit after users have
    state. Amends §6's packaging contract + smoke-test procedure
    (engineering item 6, pending).
  - **`webbrowser`-in-eframe finding (record §3.4):** eframe 0.35
    hardcodes egui-winit `features = ["clipboard", "links"]`, so
    `webbrowser` 1.2.1 (+ its `url` dep — the lockfile's ONLY hit on a
    network-crate grep) is unconditionally linked in the shipping
    Windows binary and cannot be feature-disabled downstream. §1.1
    stays true — `webbrowser` makes no request itself, it hands a URL
    to the OS default browser, and pdfce never emits `OpenUrl` today —
    but **§1.1 needs a precision correction** ("no HTTP client/TLS
    stack", not "no code that can reach the network") — engineering
    item 5, pending. Free dividend: a zero-dependency Help-menu
    "open releases page" item is available at first release.
  - **Two latent CI defects found (record §3.5), fixes pending
    engineering:** (1) the `gui-core-separation` job runs `cargo tree`
    with no `--target` on ubuntu, so it checks the 237-crate **Linux**
    graph, not the 147-crate shipped **Windows** graph — a
    Windows-only GUI dep creeping into `pdfce-core` would pass CI; fix
    is one `--target x86_64-pc-windows-msvc` flag (`cargo tree
    --target` is metadata-only, needs no installed target). (2) every
    job uses `dtolnay/rust-toolchain@stable` while
    `rust-toolchain.toml` pins 1.97.1 — rustup honors the file, so a
    second toolchain downloads silently; pin the action to 1.97.1.
  - Eight standing rules **R9–R16** added to `ROADMAP.md` (condensed;
    the record's §6.1 is the full-text authority). The engineer
    implements the record's §10 items 1–7 (two new CI jobs, two CI
    fixes, §1.1 + §6 amendments, README copy per §6.3 verbatim) in the
    next Pass touching CI or packaging; until §1.1/§6 body text is
    amended the decision record is the authoritative source, same
    convention as decisions 001/002 above.
- **2026-07-30 — Pass 1 text-rendering font strategy decided (decision
  004; fourth use of the KenAgent protocol).** Full record:
  `docs/decisions/004-text-rendering-fonts.md`. Resolves the three
  referred sub-questions — read-path font parser, standard-14 glyph
  shapes, Pass 1 composite/shaping scope. Outcome:
  - **`skrifa` 0.42, pinned to epaint's resolved version, is the sole
    read-path font parser** for `pdfce-render` (never `pdfce-core`).
    Already in `Cargo.lock` via `epaint` 0.35 and `vello_common`, so
    the dependency adds **zero new lock packages** and zero
    `THIRD_PARTY_LICENSES.md` entries. Its `raw` re-export of
    `read-fonts` covers all four PDF font-program cases from the one
    dependency — including **bare Type 1 and bare CFF via
    `raw::ps::{type1, cff}`**, which eliminates `PRIOR_ART.md`'s
    "Type1 is the weakest link ecosystem-wide" risk outright (PFB
    segment tags, PFA hex `eexec`, raw binary `eexec` and `lenIV` all
    verified at source in `read-fonts` 0.39.2). The version pin is
    load-bearing: declaring upstream 0.45 would link a second
    semver-incompatible fontations stack beside epaint's — guarded by
    a `cargo tree --duplicates` CI check (R21).
  - **The Foxit base-14 faces are bundled** (BSD-3-Clause via Google's
    pdfium grant, 264,741 bytes, all 14 including byte-exact-metric
    Symbol and ZapfDingbats; provenance-verified per R22 —
    source URL, upstream commit, SHA-256, extraction method, license
    text). The obvious alternative, URW/Nimbus, was identified as
    **`AGPL-3.0-only WITH PS-or-PDF-font-exception-20170817`** — the
    exception covers embedding in a PS/PDF *document* only, not
    application bundling — i.e. a copyleft trap. Rejected on the
    merits and **never escalated to Ken because it was unneeded**:
    Foxit is better on every remaining axis, so the `LEGAL.md` §6.2
    copyleft-escalation protocol was never triggered and the §1
    license decision stays untouched and unconstrained.
  - **`FontEnvironment`/`RenderOptions` API seam** (record §6.3): the
    renderer never touches the filesystem, environment, or OS font
    store — the shell supplies any additional/replacement faces
    through the public API. R10 platform-cleanliness holds unchanged;
    the WASM fork supplies nothing and gets the bundled set.
  - **The render path never shapes (R17)** — content streams carry
    already-positioned glyphs; shaping belongs only to a future
    text-authoring path (`harfrust`) and reading-order recovery only
    to text extraction (`unicode-bidi`). **No hinting, ever (R18).**
    **Rendering is font-deterministic by default (R19).**
    **Substitution is always disclosed in `Diagnostics` (R20).**
  - **Pass 1 scope grows to `Identity-H`/`Identity-V` composite fonts**
    (`CIDFontType2` + `CIDFontType0`), Type 3, and the full simple-font
    §9.6.6 chain — a documented departure from the spec RAG's ladder
    (steps 4–6 were "the natural Pass 2" only while the parsing
    question was open; with `skrifa` in they collapse to "pick a GID,
    hand an outline to a pen", and steps 1–3 alone would render no
    text in most modern subsetted PDFs). Non-Identity CMaps, vertical
    metrics beyond `Identity-V` decoding, and `Tr` 4–7 clipping defer
    with diagnostics.
  - Six standing rules **R17–R22** added to `ROADMAP.md` (condensed;
    the record's §6.1 is the full-text authority). The engineer
    implements the record's §10 items 1–8 (skrifa dep + duplicates
    guard, `tools/extract-base14/`, `.gitattributes` font formats,
    manual attribution entry, `pdfce-core` std-14 data vs
    `pdfce-render` outline split, `src/font/` modules + seam +
    diagnostics, the four test families) in the Pass 1 font slice;
    until §4/§10 body text absorbs the design, the decision record is
    the authoritative source, same convention as decisions 001–003
    above.
- **2026-07-30 (continuation 8) — Pass 1.1 item 1 shipped
  (xref/object/hybrid streams); decision 001 §6.3 harvest gate CLOSED
  BY MEASUREMENT; `Provenance` API evolution; encryption-refusal
  scope addition.**
  - **(a) The conditional oxidize-pdf xref-recovery-harvest gate
    (decision 001 §6.3) is now CLEARED BY MEASUREMENT, permanently
    closed.** The gate's trigger was pdfce's own parser scoring <~95%
    on the veraPDF / PDF-Association corpora. Continuation 7's
    baseline measurement (2,914 files, `tools/corpus-report`) was
    82.4%, with `RefusedXrefStream` (489 files) accounting for 97.8%
    of all failures — i.e. the shortfall was one unimplemented
    feature, not parser weakness. After implementing cross-reference
    streams (§7.5.8, Tables 17/18, W defaults, entry types 0/1/2),
    object streams (§7.5.7, new `objstm.rs`), and hybrid files
    (§7.5.8.4, `/XRefStm` before `/Prev`), the re-run measured
    veraPDF Ok **2,395 → 2,884 = 99.2%**, `RefusedXrefStream` 489 →
    0, with ALL 24 remaining non-Ok files across both corpora being
    deliberate `*-fail-*` conformance files (correct rejections),
    zero panics/timeouts, 12,927 pages rendered. 99.2% actual vs the
    <~95% trigger: **no oxidize-pdf harvest was ever needed.** The
    conditional-harvest lesson
    (`C:\personal_rag\pdf\lesson_20260730_oxidize_pdf_xref_recovery_conditional_harvest.md`)
    stands as methodology; its pdfce-specific condition is resolved.
  - **(b) `Provenance` API evolution — the §5 round-trip contract
    extended to PDF-1.5 compressed objects.**
    `IndirectObject.span: ByteSpan` (decision 001 §6.1 obligation 1)
    became `provenance: Provenance`, an enum of `File(ByteSpan)` |
    `ObjectStream { container, index }`, with a `file_span()`
    accessor (`Some` only for `File`). Rationale: an object parsed
    out of an object stream has no contiguous file bytes, so
    byte-identical passthrough is not merely unimplemented for it —
    it is *inexpressible*, and the type now says so. The §5 contract
    is thereby **expressible-or-consciously-absent**: any future
    writer touching a compressed object must either promote it to an
    uncompressed object or rewrite its container stream; that
    obligation is documented on the `Provenance` type itself.
    `XrefEntry` is now `#[non_exhaustive]` with a new `InStream`
    variant. §5 body amended same-day (see the dated bullet there).
  - **(c) Scope addition, engineer judgment, flagged to the
    operator:** `XrefErrorKind::EncryptionUnsupported` — encrypted
    PDFs (§7.6) are refused up front rather than silently rendering
    ciphertext (an honesty call in the same spirit as the render
    path's "unsupported ≠ broken" state). GUI `Status::Unsupported`
    repointed; 4 corpus files reclassified `RefusedEncrypted`.
    Supersedes nothing — encryption support proper remains the
    Backlog "Encryption" bucket.
  - **(d) Tolerance postures chosen** (auditable, each with its spec
    anchor): `/Type` absent tolerated on XRef/ObjStm streams,
    present-but-wrong refused; a malformed individual xref-stream
    row is skipped per §7.5.8.3's unknown-type posture; a broken
    `/XRefStm` degrades to the classic xref view (safe by
    §7.5.8.4's completeness guarantee for pre-1.5 readers); ObjStm
    `/N` and `/First` must be direct objects. Corpus-verified facts
    behind these:
    `C:\personal_rag\pdf\lesson_20260730_xref_stream_w_default_hybrid_fallback_objstm_drop.md`.
- **2026-07-30 (continuation 9) — Pass 1.1 slice shipped: Form and
  Image XObjects (`Do`) + inline images. Five decisions; §3/§10 body
  amended same-day (see the dated notes there).**
  - **(a) Nested form execution uses a fresh `Interpreter` over a
    clone of the current `GraphicsState`, not `q`/`Q` on the shared
    stack.** Rationale: this makes §8.10.1's steps (a) ("save the
    current graphics state") and (e) ("restore the graphics state to
    what it was before step a") **structural, not conventional** — an
    unbalanced `Q` inside a form's own content stream provably cannot
    pop the caller's graphics state, because the caller's state was
    never pushed onto the same stack the form is mutating. A second,
    non-optional consequence: each nested interpreter gets its own
    font cache, which is a **correctness** requirement, not a
    performance optimization — a font cache keyed only by resource
    name would silently satisfy a form's `/F1` lookup with the page's
    `/F1` glyph program if the two happen to share a cache, and
    `/F1` in a form's own `/Resources` dictionary is legitimately a
    *different* font object than `/F1` on the page (§7.8.3 — resource
    names are scoped to the dictionary that defines them, not global).
  - **(b) The XObject cycle guard is keyed on the XObject's object
    number, not its resource name.** Rationale: the same stream object
    can legitimately be referenced under different resource names in
    different `/Resources` dictionaries (e.g. `/X1` on the page and
    `/Im1` inside a form both pointing at object 42) — a name-keyed
    guard would miss a real cycle reachable only through the second
    name, and could also false-positive on two different objects that
    happen to share a name in unrelated resource dictionaries.
  - **(c) Text objects do not cross the form boundary — the nested
    interpreter's text state starts as `text: None` — while
    text-relevant graphics state (current font, size, character/word
    spacing, `Tz`/`Tr`/leading) IS inherited from the caller.**
    Rationale: §9.4.1 defines `BT`/`ET` as delimiting exactly one text
    object with its own `Tm`/`Tlm`; a form boundary is not a `BT`/`ET`
    pair, so a `BT` that never closes before `Do` (or a form that
    itself never opens one) must not silently inherit or leak a text
    matrix across the boundary. But §9.3 defines font/size/spacing/
    render-mode as **graphics state**, not text-object state, and
    graphics state IS supposed to flow into a form per step (a)'s
    state-save semantics — so these two are deliberately handled
    differently rather than uniformly "reset everything" or
    "inherit everything." Pinned by a regression test.
  - **(d) Images are painted via a tiny-skia `Pattern` shader over the
    user-space unit square, never `Pixmap::draw_pixmap`.** Rationale:
    `draw_pixmap` takes an integer `(x, y)` destination origin plus a
    transform — it is fundamentally a *blit*, unable to express
    §8.9.4's requirement that an image map onto an **arbitrary
    affine** region of the page (rotation, skew, deliberate aspect
    distortion, all legal under the image-space-to-user-space CTM).
    The `Pattern`'s own transform is set to image-space→user-space
    (`[1/w 0 0 −1/h 0 1]`, carrying the mandatory y-flip since PDF
    image space has the origin at the top-left row while user space
    doesn't); `fill_path`'s transform argument (the CTM) is then
    POST-concatenated by tiny-skia into that pattern transform
    (`Shader::transform` semantics: `a.post_concat(b)` = "apply a,
    then b"), yielding image→user→device in one composed matrix. The
    filled geometry is the same unit square under the same CTM, so
    the painted region and the sampled region coincide by
    construction rather than by two independently-computed transforms
    needing to agree. Full write-up (ecosystem-wide tiny-skia finding,
    not pdfce-specific):
    `D:\dev\rag\rust\tiny_skia_0.11_pattern_shader_arbitrary_affine_image_placement.md`.
  - **(e) `MAX_XOBJECT_DEPTH` raised 16 → 64, corpus-corrected mid-slice.**
    See §10.1's dated amendment for the full rationale — briefly: 16
    was intuition and overflowed on one of 2,914 corpus files, a
    **conformant** 32-deep form-XObject chain (veraPDF
    `6-1-12-t08-pass-c.pdf`, objects 19–50); Annex C has no
    form-nesting limit and PDF/A §6.1.12 forbids imposing one. Raised
    to 64 (2× measured depth); corpus-wide overflows now 0. **Second
    incident of the identical bug shape** (first: `MAX_TOKEN_LEN`,
    continuation 7) — prompted a new `ROADMAP.md` Standing rule
    requiring every new resource guard to be run against veraPDF's
    §6.1.12 suite specifically before shipping, not just the general
    corpus.
  - **Corpus delta** (same 2,914-file corpus, isolated by reverting
    only the `Do`/inline-image arms): deferred ops 7,347 → 6,079
    (−17.3%); images rendered 0 → 76; images unsupported 0 → 137;
    forms rendered 0 → 1,168; glyphs substituted +37 (text inside
    forms now paints); xobject depth overflows 0 (was 1 at the old
    depth-16 guard); zero panics/timeouts/hangs. `images_unsupported`
    (137) now EXCEEDS `images_rendered` (76) — makes `DCTDecode`
    (baseline JPEG) the corpus-measured next priority, recorded in
    `ROADMAP.md` Pass 1.1 item 6.
  - New crates/modules: `pdfce-core/src/filters/ascii.rs`
    (`ASCIIHexDecode`/`ASCII85Decode`, §7.4.2/§7.4.3 — required by this
    slice, not deferred, because they're the only two filters that
    make an inline image's data length unambiguous per §8.9.7);
    `pdfce-render/src/image.rs` (image XObjects + inline images → RGBA
    pixmaps, full §8.9.5.2 pipeline, `MAX_IMAGE_PIXELS` = 32 Mpx
    guard); `pdfce-render/src/interpret.rs` gained `Do` dispatch and
    inline-image routing.
  - RAG escalations: `C:\personal_rag\pdf\lesson_20260730_max_xobject_depth_verapdf_32_deep_conformant_chain.md`,
    `C:\personal_rag\pdf\lesson_20260730_corpus_image_codec_priority_dct_first.md`,
    `D:\dev\rag\rust\tiny_skia_0.11_pattern_shader_arbitrary_affine_image_placement.md`.
- **2026-07-30 — Image-codec strategy decided (decision 005; fifth use
  of the KenAgent protocol).** Full record:
  `docs/decisions/005-image-codecs.md`. Resolves `ROADMAP.md` Pass 1.1
  item 6's deferred sub-order for the remaining unimplemented PDF
  filters (DCT/LZW/CCITT/JBIG2/JPX). Outcome:
  - **Two-tier codec architecture, both tiers in `pdfce-core` (R23,
    record §4.6/§6.3).** Image codecs (DCT, CCITT, JBIG2, JPX) are a
    **terminal stage**, not byte-stream filters:
    `filters::decode_stream` never decodes them — it returns a new
    `FilterError::ImageCodec(String)` variant, and a new
    `pdfce_core::image_codec` module
    (`decode_image(doc, dict, raw) -> Result<CodedImage,
    ImageCodecError>`) runs the byte-stream *prefix* of the `/Filter`
    chain through tier 1 and dispatches the single terminal codec.
    Codec output crosses the API as a `CodedImage` — samples plus
    **codec-declared** geometry and colour model — never a bare
    `Vec<u8>`, because §8.9.5 Table 89 (JPX codestream overrides the
    dictionary) and §7.4.8 (DCT colour model depends on the JPEG's own
    APP14 marker) make that declaration unrecoverable otherwise.
    `LZWDecode` alone stays a byte-stream filter in the cascade
    (bytes-in/bytes-out, `/Predictor` composes over it unchanged).
    Placement in `pdfce-core` (not `pdfce-render`) is set by the
    consumer set — `pdfce-cli extract-images`/optimize and the
    round-trip writer need to understand image streams without a
    rasterizer: **core decodes and models, render paints** (R26: only
    `pdfce-render` applies `/Decode` and resolves `/ColorSpace`).
  - **Five crate selections, all permissive, all pure-Rust, all
    `forbid(unsafe_code)` in the configuration pdfce builds:**
    DCT = `zune-jpeg 0.5` (`default-features = false` drops the
    `x86`/`neon` SIMD features, which is what activates its
    `cfg_attr` `forbid(unsafe_code)` — all 96 unsafe occurrences live
    in SIMD files that don't compile in this configuration);
    LZW = `weezl 0.2`; CCITT = `hayro-ccitt 0.3` (`fax` the named
    fallback/differential oracle); JBIG2 = `hayro-jbig2 0.3` and
    JPX = `hayro-jpeg2000 0.4` (both `default-features = false,
    features = ["std"]` — `simd` off drops `fearless_simd`, `image`
    off keeps the `image` crate/`moxcms` out of `pdfce-core`).
    The SIMD-off posture is a standing rule (R24), and CI asserts the
    **feature state** (`cargo tree -e features`), not just the
    dependency set, because feature unification is transitive and
    silent. The GPL C alternatives the obvious answers would have
    required (`jbig2dec`, OpenJPEG wrappers, mozjpeg) were made moot —
    zero `LEGAL.md` §6.2 copyleft escalations needed; §1 stays open
    and unconstrained. Named risks: three of five codecs are one
    author's project; `hayro-*` MSRV = 1.92 = pdfce's exactly, zero
    headroom.
  - **A correction worth recording (record §3.6): `Cargo.lock`
    presence ≠ build-graph presence.** The lockfile already listed
    `zune-jpeg`/`weezl`/`fax`/`tiff` as **unenabled optional
    dependencies** of `image` (whose `jpeg`/`tiff` features are off),
    so the tempting "zero-cost like `skrifa` in decision 004"
    conclusion was WRONG — verified by `cargo tree -i` ("nothing to
    print") and by `THIRD_PARTY_LICENSES.md` (generated from the real
    build graph, no entries). Honest cost: six new packages, six new
    attribution entries. Generalized to
    `D:\dev\rag\rust\cargo_lock_unenabled_optional_deps_not_build_graph.md`.
  - **Priority order, set by measurement** (record §3.1/§3.2; 2,914
    corpus files): **Pass 2.1 = DCT + LZW** (82.3% and 10.4% of
    unimplemented-filter occurrences), **Pass 2.2 = CCITT + JBIG2**
    (zero corpus presence *by corpus construction* — conformance
    corpora contain no scanned documents; priority set by the OCR/
    scanned-document Backlog dependency; one vendor, `hayro-jbig2`
    depends on `hayro-ccitt`), **Pass 2.3 = JPX** (rarest, largest
    codec surface, most unverified spec surface). Full measurement
    tables recorded in the `ROADMAP.md` Pass 2.1 entry.
  - **Three BLOCKING spec verifications dispatched to
    `pdfce-spec-librarian`** — each gates its Pass, none is
    ceremonial: §7.4.8 **Table 13** `/ColorTransform` wording +
    defaults (blocks Pass 2.1; `filter__dct.md` marks it unverified
    and the colour-routing table rests on it); **Table 11**
    `/Columns`/`/EndOfBlock`/`/BlackIs1` defaults (blocks Pass 2.2;
    `BlackIs1` is a polarity flag — a wrong default inverts every fax
    image plausibly); §8.9.5 **`/SMaskInData` + Table 89** JPX
    overrides, then an audit of `pdfce-render`'s image path for hard
    requirements Table 89 makes optional (blocks Pass 2.3).
  - Six standing rules **R23–R28** added to `ROADMAP.md` (condensed;
    the record's §6.1 is the full-text authority). The engineer
    implements the record's §10 engineering items 5–14 across Passes
    2.1–2.3; until §4/§10 body text absorbs the design, the decision
    record is the authoritative source, same convention as decisions
    001–004 above.
- **2026-07-30 (continuation 12) — Pass 2.1 shipped (DCT + LZW +
  RunLength; two-tier `image_codec` landed); three additive API
  deviations from decision 005 §6.3; decision 005 §3.2 measurement
  corrected; decision 006 dispatched.**
  - **(a) Three engineer deviations from decision 005's §6.3 API
    sketch, all additive, none contradicting a standing rule:**
    (1) `CodedImage::codec` is an `Option<Codec>` with an
    `Unspecified` variant rather than a bare enum — a codestream can
    fail to declare what the sketch assumed it always would;
    (2) `decode_image` takes an `inline: bool` parameter — the
    inline-image path has different legality rules (e.g. JBIG2
    forbidden per §7.4.7/§8.9.7) and the seam is where that belongs;
    (3) `RunLengthDecode` truncation (data ends mid-run, no EOD) is a
    strict `Err`, consistent with the fail-clean filter contract
    (decision 001 §6.1 obligation 4), not a tolerance.
  - **(b) CORRECTION to decision 005 §3.2 — the "0 four-component
    JPEGs in the corpus" measurement was WRONG: 12 exist**, in
    veraPDF's "6.2.4.3 Uncalibrated -Device colour spaces" section
    (the scan missed them; the record's method caveats anticipated
    the failure mode). **Revisit trigger 2 (§9) is LIVE** —
    `6-2-4-3-t02-pass-a.pdf` is `/DeviceCMYK` `/DCTDecode`, Adobe
    APP14 transform 2 (YCCK), NO `/Decode` array (relies on the bare
    Adobe convention); pdfce passes raw samples per §5.5's deliberate
    no-guess posture, so these 12 likely render inverted today.
    Filed as a dated addendum at the END of
    `docs/decisions/005-image-codecs.md` (the record is not
    rewritten). **Decision 006 dispatched** for the sourced
    inversion rule, per §5.5's file-the-answer-then-implement order.
  - **(c) Pass 1 bugfix (content.rs):** `ID` followed by CRLF is ONE
    white-space character (§8.9.7 + §7.2.2's CRLF-is-one-EOL);
    consuming only the CR left a stray `\n` corrupting 4 corpus
    inline DCT images. Lesson:
    `C:\personal_rag\pdf\lesson_20260730_inline_image_id_crlf_single_whitespace.md`.
  - Ship stats: corpus Ok 99.2% → 99.3% (2,886); images rendered
    74 → 201; unsupported 135 → 8 (7 JPX for Pass 2.3 + 1 deliberate
    `/Lzw`-misspelling fail-file); 412 workspace tests; 4 fuzz
    targets zero crashes; `THIRD_PARTY_LICENSES.md` +3 permissive
    entries; all gates green incl. the new R24 feature assertion.
    Full entry: `ROADMAP.md` Shipped, Pass 2.1.
- **2026-07-31 — CMYK/YCCK JPEG inversion rule decided (decision 006;
  sixth use of the KenAgent protocol).** Full record:
  `docs/decisions/006-cmyk-jpeg-inversion.md`. Closes decision 005
  §5.5's deliberately-open question and its §9 revisit trigger 2;
  corrects the 005 Addendum of 2026-07-30 (second dated addendum
  appended there). Outcome:
  - **The rule is the null rule: pdfce NEVER applies an "Adobe CMYK
    inversion" (R29).** Not on APP14 presence, not on transform-byte
    value, not on component count, not on producer sniffing. The APP14
    transform byte is consumed for exactly one purpose, already
    correctly implemented: selecting the ISO 32000-1 Table 13 colour
    transform. `/Decode` is the sole polarity control —
    `/Decode [1 0 1 0 1 0 1 0]` IS the sanctioned mechanism by which a
    producer declares inverted storage. **No behavioral change was
    needed**: the 005 Addendum's "these files likely render inverted
    today" premise was FALSIFIED — pdfce pixel-matches pdfium on all 9
    real four-component corpus JPEGs (count corrected from 12 by two
    independent scans), and matches on all six controlled variants
    (transform 2/0/no-marker × with/without `/Decode`).
  - **Sourced by four-engine consensus + a revert trail:** pdf.js,
    pdfium, MuPDF (PDF path) and Poppler all implement exactly
    never-invert (actual conditions read at source, not paraphrased);
    marker-gated inversion was shipped and reverted twice upstream
    (cairo issue 156, Firefox bug 674619). ImageMagick/libvips/
    standalone pdf.js DO invert unconditionally — recorded so they are
    never mistaken for PDF-reader precedent.
  - **Adobe TN #5116 negative result:** the normative-by-reference
    primary (ISO 32000-1 §7.4.8 footnote a) was obtained and read. The
    word "invert" appears zero times; §13.1's only `255 −` is the
    reversible CMYK→YCCK definition (forward transform defined in
    terms of TRUE ink values, so the inverse recovers true ink
    directly — no further step exists). §18's APP14 layout does NOT
    enumerate transform values 0/1/2 (those are de facto, from libjpeg
    `jdapimin.c` + Table 13). The inverted-CMYK storage convention is
    undocumented Photoshop behavior, absent from the canonical source;
    Adobe's own products compensate out of band via the container's
    decode array.
  - **The Pillow trap → R31.** The first reference consulted (Pillow)
    reported the exact complement of libjpeg's answer — because
    `PIL.JpegImagePlugin` applies rawmode `CMYK;I` to EVERY
    four-component JPEG unconditionally ("assume adobe conventions",
    no marker test). Trusting it would have "fixed" a non-bug, broken
    all 9 files, and produced a green test suite (fixtures built
    against the same wrong reference). Hence R31: a reference decoder
    is evidence only after its own conventions are verified; prefer a
    production-engine page render (pdfium/`pypdfium2`), and a
    source-level read of the condition, over a bare image-library
    decode.
  - **R26 clarified, not changed: observing is not applying.** The
    codec adapter may OBSERVE the image dictionary to classify
    diagnostics (`dct::decode` already receives `dict`) while
    remaining forbidden to APPLY `/Decode` or any polarity flip. R26's
    anti-inversion clause graduates from provisional to
    permanent-and-sourced. Diagnostics split accordingly:
    `dct_cmyk_images` (benign YCCK census — 9 in corpus, verified
    correct, no warning) vs `dct_cmyk_polarity_unverifiable` (R30 —
    4-component AND effective transform 0 AND no `/Decode`, the one
    genuinely ambiguous shape; 0 in corpus; named warning, and any
    future repair is an operator-reviewable per-image toggle, never a
    default).
  - **Separate colorimetry gap found in passing, deliberately NOT
    decided here (006 §3.7):** pdfce and pdfium agree on polarity but
    disagree on colour — `Rgb::from_cmyk` (`gstate.rs:112`) is naive
    additive vs pdfium's calibrated `AdobeCMYK_to_sRGB1` table (37.4%
    of pixels >8 Δ on the corpus CMYK image; max Δ `[11,37,30]`).
    Affects every `DeviceCMYK` fill/stroke, not just images. Filed as
    its own `ROADMAP.md` Backlog entry ("DeviceCMYK→RGB colorimetry"),
    to be scoped via `pdfce-acrobat-librarian`.
  - Three standing rules **R29–R31** + the R26 clarification added to
    `ROADMAP.md` (condensed; the record's §6.1 is the full-text
    authority). Engineering follow-ups (006 §10 items 2–6: dct.rs doc
    rewrite, counter split, CLI/GUI note text, six §6.4 regression
    fixtures asserting sample values at named pixels) are docs/
    diagnostics/fixtures only — no behavioral change, per the record.
- **2026-07-31 (continuation 15) — Pass 2.3 shipped (JPXDecode via
  `hayro-jpeg2000 0.4`); Pass 2 / decision 005 COMPLETE as planned;
  six engineer deviations recorded; a new guard CLASS
  (declared-work amplification, `jpx::MAX_TILES`) from a live fuzz
  find.**
  - **(a) Table 89 precedence — the dispatch brief stated it
    BACKWARDS; the verified rule is implemented:** a PRESENT
    `/ColorSpace` **wins** over the codestream ("any colour space
    specifications in the JPEG2000 data shall be ignored"); the
    codestream wins only when `/ColorSpace` is absent.
    `/BitsPerComponent` and `/Decode` are ignored as briefed. Pinned
    by test `jpx_present_colour_space_still_wins`.
  - **(b) `/Width`/`/Height` are NOT a Table 89 override** — the
    dict-for-placement / codestream-for-stride split is retained,
    divergence counted; a per-filter dimension-policy contrast table
    added to the `image_codec` `mod.rs` docs.
  - **(c) Bit-depth normalization is full-range scale to 8**
    (`round(v/(2^d−1)×255)`), not high-byte truncation — Table 89
    leaves depth handling to the conforming reader; the 16-bit
    fixture's `0x00FF` discriminator pixel distinguishes the two
    choices.
  - **(d) `/SMaskInData` 2 is recognize-and-defer** — preblended
    colour returned as stored, alpha not exposed; new counter
    `jpx_smask_in_data_preblended` → CLI key `jpx_preblended`
    (appended; stable-line contract kept) + a GUI line.
  - **(e) EXTRA Table 89 gap found in the audit and closed:**
    `decode_stencil` hard-required 1-bit data and would have sheared
    a JPX `/ImageMask`'s 8-bit samples 8× — the stencil path now
    takes stride/depth from the codec and thresholds against zero,
    `/Decode` still honoured (the §7.4.9 exemption).
  - **(f) hayro `data_u8()` deliberately unused** — it interleaves
    alpha AND computes `1 << bit_depth` on a palette-box depth that
    may be 128, a shift-overflow panic reachable from fuzzed input;
    pdfce interleaves itself and refuses depths outside `1..=31`
    (named diagnostic `JPX/bit-depth`). Lesson:
    `C:\personal_rag\pdf\lesson_20260731_hayro_jpeg2000_data_u8_shift_overflow_palette_depth.md`.
  - **(g) `jpx::MAX_TILES = 4096` — a new guard class:
    declared-work amplification.** A 310-byte codestream declaring a
    65,536-tile grid over 512×1024 pixels cost 32 s to decode; the
    tile grid is declared independently of image size, so no
    pixel/byte ceiling saw it. Third guard-by-intuition encounter
    (after MAX_TOKEN_LEN, MAX_XOBJECT_DEPTH) but the FIRST found by
    the fuzzer rather than a rejected real file. 8× the most
    aggressive real tiling; 32 Mpx can still tile 91×91; same input
    now 3 ms; the input kept as fuzz corpus seed + an accept-side
    test pins the ceiling from below. Lesson:
    `C:\personal_rag\pdf\lesson_20260731_jpx_max_tiles_declared_work_amplification.md`.
  - Ship stats: corpus Ok holds 2,892 (99.2%); images rendered
    204 → 210, unsupported 9 → 3; codec-unsupported 7 → 0;
    codec-FEATURE-unsupported 0 → 1 (NAMED:
    JPX/enumerated-colour-space — CIEJab, space 19,
    §7.4.9-permitted, unimplemented upstream; was a generic
    corrupt-file error). 487 workspace tests (was 457); final fuzz
    campaign 15,694 runs / 60 s / zero crashes;
    `THIRD_PARTY_LICENSES.md` +1 permissive entry (Apache-2.0 OR
    MIT); all gates green incl. MSRV 1.92 core+render (no bump —
    decision 005 §3.7's zero-headroom risk did not bite). GUI
    launched on `jpx-rgba-smaskindata1.pdf`. Full entry:
    `ROADMAP.md` Shipped, Pass 2.3.
- **2026-07-31 (continuation 16) — Next subsystem decided (decision
  007; seventh use of the KenAgent protocol): the incremental-save
  writer, sliced 3.0 → 3.1 → 3.2, the first slice with NO editing
  capability.** Full record:
  `docs/decisions/007-next-subsystem-after-read-stack.md` (the
  effective JSON is its Appendix A — base block plus the
  final-message patch; a reconciliation note at archival grounds its
  housekeeping items against Continuation 15). Candidate ranking
  A ≫ D > B > C. Pass 3.0 ships a serializer plus
  `save_full`/`save_incremental` whose entire acceptance bar is a
  corpus-wide executable proof of the §5 round-trip/minimal-diff
  invariant — per-object-definition byte identity for full rewrite,
  whole-file identity for empty-dirty-set incremental (R32) — BEFORE
  any mutation code exists, so §11.4's undo obligation does not bind
  until Pass 3.1. Adds standing rules **R32–R41** (condensed in
  `ROADMAP.md`; the record's §6 is the authority): never normalize;
  the round-trip gate guards every writer Pass; redaction forbids
  incremental save; save-mode disclosure; the object-encoder seam
  for the Pass-5 crypt stage; compressed-object promote-not-rewrite;
  `/ID` discipline; fuzz + differential coverage; no output
  fingerprint. Pass 3.0 also owes THIS document an amendment —
  §5 gains R35/R36/R39 and §11.2 a cross-reference to R35 (decision
  007 deliverable 9); the body-section update is deferred to that
  Pass, when the writer's actual shape is known, and this entry is
  the audit-trail pointer until then. Blockers live now: the
  `pdfce-spec-librarian` write-direction audit of §7.5.4/.5/.6/.8 +
  §14.4 (dispatched, in flight) and the engineer's first-action
  re-check of `hayro-write`'s changelog for byte-preserving
  incremental append (decision 001 §9 trigger 2).
- **2026-07-31 (continuation 17) — Pass 3.0 shipped (identity writer
  + round-trip proof harness): the §5 invariant is now an executable,
  green, corpus-wide gate; the §5.1–5.6 + §11.2 body amendments
  LANDED in-Pass (deliverable 9 — closing the deferral the
  continuation-16 entry carried); six engineer deviations recorded;
  the `/Encrypt` census returned — promotion trigger NOT met, Pass 5
  stays behind Pass 4.**
  - **Blocker (b) resolved first, NEGATIVE:** `hayro-write` 0.7.0
    (2026-05-27) self-describes as an internal `pdf-writer`
    converter, ~580 LoC, no incremental append — decision 001 §9
    trigger 2 does not fire; depend-or-contribute stays closed.
  - **Gate results (2,898 loadable of 2,914; the 16 NotLoadable are
    deliberate `*-fail-*` files):** empty-dirty-set
    `save_incremental` whole-file byte identity **2,898/2,898 =
    100.00%**; append identity (prior bytes intact) 2,898/2,898;
    `save_full` per-object-definition verbatim 2,897/2,898 = 99.97%,
    the single miss a CORRECT named refusal (hybrid "Isartor test
    suite manual.pdf" → `WriteError::HybridFullRewrite`, CLI exit 8,
    R33/R27 posture; incremental works on it via form A); raster
    self-oracle 5,783/5,783; 0 objects re-serialized under
    `SaveOptions::identity()`; 0 panics/timeouts; W14's ~98% STOP
    threshold never approached. The two identity assertions were kept
    distinct per R32 (W1's named confusion did not occur).
    Structural census byproduct: 2,410 classic / 487 xref-stream /
    1 hybrid / 36 live-linearized.
  - **(a) `ProducerPolicy::Set` never CREATES a missing `/Info`** —
    stamping a producer into a file that had no `/Info` would be the
    exact fingerprinting behavior R41 / decision 001 §6.1 obligation
    6 exists to prevent; `Set` only rewrites an `/Info` that already
    exists.
  - **(b) `save_full` carries object streams intact — zero
    promotions.** Type-2 xref entries name container+index, not byte
    offsets, so verbatim re-emission of the container keeps them
    valid; W3 (compressed-object offset drift) is structurally
    avoided rather than handled.
  - **(c) Hybrid full-rewrite refused BY NAME**
    (`WriteError::HybridFullRewrite`, CLI exit 8) — a full rewrite of
    a §7.5.8.4 hybrid cannot preserve both xref views without
    normalizing one away (forbidden by R33); incremental save remains
    available on hybrids via form A.
  - **(d) No predictor on emitted xref streams** — §7.5.8 never
    mentions predictors on the write side (negative result from the
    write-direction audit); reading predictored streams is
    unaffected.
  - **(e) No wildcard match arms anywhere in the writer** —
    `#[non_exhaustive]` does not bind inside the defining crate, so
    wildcard-free matches make a future `Object` variant a compile
    error at every serializer decision point instead of a silent
    null/fallback emission. Finding escalated:
    `D:\dev\rag\rust\non_exhaustive_no_effect_defining_crate_wildcard_free_match.md`.
  - **(f) A NUL-bearing Name emits `#00` and fails reload
    deliberately** — §7.3.5 forbids NUL in a name; emitting the
    escape and letting the strict reader refuse it is the honest
    posture (never silently dropping or mangling the byte).
  - **`/Encrypt` census (decision 007 parallel cheap task, run by a
    parallel agent):** 19,940 organic PDFs (20k cap, read-only,
    aggregates only — LEGAL §5): 134 = 0.67% carry `/Encrypt`;
    26 R2 / 30 R3 / 67 R4 / 10 R6 / 1 undetermined-R (FOPN FileOpen
    DRM, non-Standard handler); 92.5% legacy R≤4; empty-vs-real
    password not determinable pre-Pass-5. Promotion trigger NOT met.
  - Ship stats: new `pdfce-core` writer module
    (`mod`/`serialize`/`encoder`/`xref_out`/`save`),
    `linearization.rs`, `equivalent_across_buffers` on `object.rs`
    (lesson:
    `C:\personal_rag\pdf\lesson_20260731_span_backed_stream_derived_partialeq_cross_buffer.md`),
    `SectionShape` + `LoadedXref.startxref` on `xref.rs`,
    `tests/writer_roundtrip.rs`, `tools/roundtrip`, fuzz target
    `writer_roundtrip` (661,190 ASan execs / 61 s, zero crashes),
    CLI `round-trip` subcommand with documented exit-code contract.
    585 workspace tests (was 487); veraPDF §6.1.12 suite 44/44
    against the new writer-side guards; dependency set UNCHANGED (no
    `cargo-about` regeneration owed); all other standing gates green.
    GUI opened blank — `pdfce-gui` still lacks a file argument (open
    Pass 1.1 remainder); rendering verified via CLI `render-page`.
    Full entry: `ROADMAP.md` Shipped, Pass 3.0. Pass 3.1 engineer
    dispatched same day, in flight.
- **2026-07-31 (continuation 18) — Pass 3.1 shipped (mutation writer
  + dirty-set diff + undo/redo command log): §11.4's undo obligation
  bound and was honored in-Pass; §11.1's union-bug is now an
  executable gate (edit → undo → save byte-identical 2,897/2,897);
  §5.7 + §11.5 body amendments landed; and a CRITICAL correction to
  decision 007 W3 / §5.2 is filed forward.**
  - **CRITICAL correction (recorded forward — the archived 007
    decision file is NOT edited):** decision 007 W3's mitigation and
    §5.2's original framing claimed R35's full rewrite "closes the
    stale-copy path" for promoted compressed objects. **FALSE** —
    object streams carry through verbatim in BOTH save modes (§5.6),
    so a promoted object's old value survives inside its untouched
    container. Documented at the creating code; §5.2 carries a dated
    correction footer, §5.7 the full amendment, `ROADMAP.md` a dated
    note at R38. Binding consequence: **the Redaction Pass must
    rewrite/decompose every container stream holding a redacted
    object** — R35 is necessary but not sufficient.
  - **(a) One writer path — `save_full` takes `&DirtySet`**
    (deviation 1): `DirtySet::empty()` makes Pass 3.0's identity
    behavior a strict pinned subset of the mutation writer, not a
    parallel path that could drift.
  - **(b) `/ID` never synthesised when absent, either mode**
    (deviation 2, R41): the spec RAG's synthesise-on-full-rewrite
    recommendation was DECLINED — stamping an `/ID` into a file that
    never had one is an observable fingerprint; deferred to a real
    Save-As path.
  - **(c) Rotate-to-base-value writes nothing** (deviation 3, R33):
    the exact base spelling is restored, 4 quarter-turns net to
    zero, and `/Rotate 450` is NOT normalised.
  - **(d) Text-string encoding is ASCII-or-UTF-16BE+BOM only**
    (deviation 4): §7.9.2/Annex D.3 PDFDocEncoding is a RECORDED RAG
    GAP (a `pdfce-spec-librarian` item); undecodable bytes decode to
    U+FFFD with `exact: false` surfaced in the GUI — fuzzy, never
    sneaky.
  - **(e) GUI applies on button press, not per keystroke**
    (deviation 5): one undo step per operator intent; the undo stack
    holds meaningful operations.
  - **Fuzz find + fix (real bug):** object creation raised `/Size`
    and RESURRECTED xref entries the base `/Size` was suppressing
    (they then failed to parse). Fix: `next_object_number` allocates
    above the UNFILTERED chain maximum (was reusing live numbers) +
    creation refused by name when `/Size` suppresses entries
    (`EditError::ObjectCreationWouldExposeHiddenObjects`, CLI exit
    9; editing existing objects still works). Post-fix 408,886
    runs / 91 s zero crashes; `load_document` 681,645 / 61 s clean.
    Lesson:
    `C:\personal_rag\pdf\lesson_20260731_xref_size_suppresses_trailing_entries_raising_resurrects.md`.
  - **R38 coverage honesty:** promotion is fixture-covered, NOT
    corpus-covered — 75 corpus files hold 2,197 compressed objects
    but page objects are uncompressed in all (corpus rotation never
    promotes); the harness reports both numbers.
  - Ship stats: `EditSession` command log
    (`crates/pdfce-core/src/edit.rs`, 1,608 lines),
    `writer/fileid.rs` (§14.4 `/ID[1]` derivation), `DirtySet`
    (replacements + trailer patch + `changes_content`), CLI
    `set-info` / `rotate-page` / `--verify-undo` / exit 9 /
    appended `promoted=` key, GUI properties panel + rotate +
    undo/redo + "Save a copy…", `tools/roundtrip` mutation mode,
    fuzz edit-history extension. Key test edit → undo → save
    byte-identical 2,897/2,897 (100%) + 6 fixture tests (incl.
    object-stream file, 12-command history, undo → redo → save).
    Pass 3.0 identity gate UNPERTURBED per R34 (2,892/2,892 + 6/6;
    full-rewrite 2,891/2,892, the miss the same correct hybrid named
    refusal; raster 5,783/5,783; 0 re-serialized). Mutation gate:
    edit applied + reloaded 100%; all other objects byte-verbatim
    100%. 52 new tests (32 core + 20 CLI) over the 585 baseline;
    fmt/clippy clean; GUI-core separation verified; dependency set
    UNCHANGED; nothing committed. UI follow-ups with
    `pdfce-ui-specialist` (in flight); Pass 3.2 promoted to In
    progress, blocked on the `pdfce-acrobat-librarian` "Core
    document ops" dispatch (in flight). Full entry: `ROADMAP.md`
    Shipped, Pass 3.1.
- **2026-07-31 (continuation 19) — Pass 3.2 shipped (structural page
  operations — the first operator-visible editing feature): seven
  ops in two shapes; deletion writes a real free list (decision 007
  W9); signature awareness shipped as a real API with a
  DocMDP-grounded rename; the R36 rule-number collision reconciled
  (R42); the Tools-dock/toolbar-cap UI conventions adopted as
  standing decisions.**
  - **UI-surface conventions ADOPTED as architectural decisions**
    (authored by `pdfce-ui-specialist` in
    `docs/ui_specs/pass-3.2-page-ops.md` §1–2, which remains the
    living spec; this entry is the audit trail):
    (1) **The Tools dock is pdfce's ONE "more tools" secondary
    surface** — a persistent right-side panel toggled from the
    toolbar. Future advanced buckets (Bates stamping, OCR,
    redaction, forms, portfolios, PDF/A conversion) become entries
    in this dock's tool list, never new floating windows;
    Properties (Pass 3.1) stays the single legacy floating
    exception, never to be joined by a second.
    (2) **The toolbar is CAPPED at its 6 groups + the Tools
    toggle.** Any future feature fits an existing group, becomes a
    rail-contextual control, or becomes a Tools-dock entry; no 7th
    toolbar group without a fresh review.
    (3) **The rail-vs-dock rule:** if the operator's argument is a
    set of pages already visible in the open document, the control
    lives on the thumbnail rail (selection + contextual action
    bar); if the argument comes from outside the open document (a
    file, a folder, a set of files), it lives in the Tools dock.
    Panel add-order stays load-bearing: toolbar, status, rail,
    dock, canvas, floating-last.
    *(Extended 2026-08-01, continuation 20 — the placement taxonomy
    is now THREE-way: a snapshot action on the whole open document
    (copy-text) fits neither rail nor dock and lives as a toolbar
    menu button. See the continuation-20 entry, item (b). This is an
    extension, not a rewrite — rules (1)–(3) stand unchanged.)*
  - **R36 collision reconciled (record defect flagged in the UI
    spec's header):** §5.4's linearization-never-repaired rule is
    now **R42**; decision 007's R36 ("save mode is chosen by
    contract and disclosed" — the signature either-or) keeps the
    number, as the code comments already use it. §5.4's citation
    line corrected; full dated note at R42 in `ROADMAP.md` Standing
    rules; no rule content changed.
  - **(a) `SignatureImpact::ByteRangePreserved`** — renamed from the
    UI spec's `PreservedIncremental` per the mid-Pass DocMDP relay
    from `pdfce-spec-librarian`: §12.8.1 NOTE 1 guarantees only
    that the signed BYTE RANGE is preserved; whether the signature
    remains *valid* in the DocMDP sense is a separate verdict, and
    the variant name no longer overclaims. Classification walks
    `/Reference` → `/TransformMethod` (`/DocMDP` lives in the
    signature's reference array, never `/Perms`; `/P` defaults 2);
    a certification whose `/P` forbids the change is a NAMED
    refusal, `EditError::CertificationForbidsChange` (Table 258);
    `/FieldMDP` recognized. `signature_impact_of_save(mode)` takes
    the save mode as a parameter (deviation from the spec's
    zero-arg sketch). Spec closure: `PDF_Spec`
    `iso32000__s__12.8.md` now 689 lines, a/b/c verdicts + the
    ByteRangePreserved-never-reported-alone rule.
  - **(b) Insert is a producer, not an `EditSession` command**
    (deviation from the UI spec §3.6/§3.9): in-place overlay insert
    requires per-object SOURCE buffers plus an overlay-aware
    renderer — deferred rather than half-built. GUI Insert
    deferred; the Tools dock names the CLI `insert` command; the
    producer path (`assemble()`) ships it complete.
  - **(c) One shared `assemble()` for all four producers**
    (extract/merge/split/insert) over one shared `ObjectGraph`
    walk (`graph.rs` — works over the loaded file OR the
    `EditSession` overlay; `edit.rs`'s Pass-3.1 comment predicted
    the need). Carryover policy table documented + cited in
    `pageops/`: outline subset+repoint / per-source-top-level
    merge / target-only insert; `/Dests` never carried (carried
    bookmarks rewritten explicit); `/PageLabels` stale-for-insert
    with named diagnostic, dropped-for-subsets; `/StructTreeRoot`
    dropped + counted; form fields `Doc<N>_` auto-rename,
    straddling fields dropped whole + counted; barrier hits
    counted.
  - **(d) Deletion free-list (W9):** `DirtySet::delete` +
    `apply_free_list` — type-0 entries, generation+1 saturating at
    65,535, front-spliced onto the existing free chain;
    pre-existing detached free entries untouched (R33); a
    two-closure sweep proves shared objects are never freed.
  - **(e) Remaining spec deviations:** rail checkbox is one
    interaction + position test (not two overlapping `interact`s);
    `egui::Window` not `egui::Modal` (the spec's own named
    fallback); split's file-size criterion deferred + named.
  - **Two bugs caught by tests (both filed as `personal_rag/pdf`
    lessons):** reorder lost inherited rotation (`materialize_for`
    was one-directional; `preserve_inherited` now writes
    §7.7.3.4's default when the NEW parent chain supplies a value
    the old chain didn't); extract left `/Dest [null /Fit]`
    (reference barriers must propagate through WHOLE arrays, not
    per-element).
  - **Open residuals (named):** `/Info` edits not
    certification-gated (`/P 1` strict reading — owed decision,
    recorded at `check_certification`);
    `PermissionGate::NotApplicableYet` awaits Pass 5; delete corpus
    coverage thin (23 multi-page files — fixtures + fuzz carry
    it); `qpdf` not on PATH (R40's external oracle unused —
    operator-installable).
  - Ship stats: `graph.rs`, `signature.rs` (810 lines), `pageops/`
    (2,833), `tests/page_ops.rs` (967). **769 workspace tests (was
    707)**; 3.0/3.1 gates UNMOVED per R34 (identity 2,892/2,892;
    full-rewrite 2,891/2,892 same hybrid refusal; edit → undo →
    save 2,891/2,891; raster 5,771/5,771); corpus page-op sweep
    2,892 extract-ok + 23/23 delete-ok, 0 failures; §6.1.12 40
    files clean with guard headroom MEASURED (outlines 10 vs 200k,
    dests 62 vs 100k, depth 3 vs 64, pages 10k vs 1M); fuzz
    `pageops_sequence` 130,400 / 61 s zero crashes +
    `writer_roundtrip` clean; GUI-free both targets; wasm32 +
    aarch64 clean; `--duplicates` + R24 clean; **`ui-strings` R1
    gate clean for the FIRST time** (3 pre-existing false positives
    fixed — evidence CI has never run, W15); no new dependencies.
    Carried items applied: Apply/Revert grey-out, per-field lossy
    marking, command-named undo tooltips, **GUI file argument
    (Pass 1.1 remainder CLOSED)**, rotate shortcut `[`/`]`. GUI
    launched (PID 23332); CLI demo split-by-bookmark →
    reverse-merge → render. Pass 4 (text extraction) promoted to
    In progress; `pdfce-spec-librarian` §9.10 sourcing dispatched,
    in flight. Full entry: `ROADMAP.md` Shipped, Pass 3.2.
- **2026-08-01 (continuation 20) — Pass 4 shipped (text extraction /
  structured content): the §9.10.2 ladder verbatim with rung 3
  structural+named, PDFDocEncoding built from Annex D.3's structural
  rules (4 source-table typos caught), the plain/sourced dual API
  adopted as the extraction-feature pattern, and the UI placement
  taxonomy extended to three-way.**
  - **(a) Confirmation-dialog convention is now a STANDING pattern**
    — two independent uses exist (Pass 3.2's signature-impact
    confirmation; Pass 4's pre-copy reliability gate, firing on
    `identity_fonts_without_to_unicode > 0 || sourced < 50%` —
    deliberately not a low threshold): a single centre-anchored,
    one-question, input-blocking `egui::Window`, resolved before any
    other pending question is posed. Enforcement lives in the action
    dispatcher, not the window code — the Ctrl+S bug (below) is why
    that placement is load-bearing.
  - **(b) Placement taxonomy is now THREE-way** (dated extension of
    the continuation-19 conventions record, which stands unchanged):
    **rail** — argument is pages in the open document; **Tools
    dock** — argument comes from outside the open document; **toolbar
    menu button** — a snapshot action over the whole open document
    (copy-text is the first instance). The rail-vs-dock binary didn't
    cover the third case; copy-text is deliberately NOT a Tools-dock
    entry. Designed in `docs/ui_specs/pass-4-text-extraction.md`
    (573 lines), which remains the living spec.
    *(Superseded/extended 2026-08-01, continuation 23 — the Pass 6.0
    ui-specialist delivered a five-way placement taxonomy that resolves
    the X14 drift and subsumes this three-way rule. See the
    continuation-23 entry, item (b). This bullet stays as the audit
    trail of the intermediate three-way form.)*
  - **(c) `plain_text()` vs `sourced_text()` dual API adopted as THE
    fuzzy-never-sneaky pattern for extraction-like features** (OCR is
    next): spec-sourced characters and derived judgments (spaces,
    line breaks, ordering) are separate API surfaces, the derived
    layer isolated (`text_extract/layout.rs`) and every derived
    insertion labelled. The Drucker `/ActualText` example is the
    pinned verification: sourced "Drucker", plain "Druc\nker" with
    one labelled derived break.
  - **(d) Deviations, both additive and counted:** per-code
    fallthrough (§9.10.3 NOTE 4 — universal practice, unsourced,
    counted); glyph-name extension for fonts failing method 2's
    whole-array precondition. `FontNote::BuiltinEncodingUnreadable`
    names the one R21-unreachable case (embedded symbolic built-in
    encoding → StandardEncoding fallback — counted as extension,
    never sourced). Rung-3 gaps are structural+named
    (`Rung3Gap::{IdentityNoToUnicode, Ucs2NotBundled,
    PredefinedCmapNotBundled}`), never silently skipped.
  - **(e) Bidi deferred-not-half-done:** RTL presence detected +
    counted; `unicode-bidi` NOT added (B1–B3 would make reordering
    wholly derived). Extraction diagnostics are a SNAPSHOT surface,
    separate from the per-frame render header (merging would lie on
    navigation).
  - **Real pre-existing GUI bug found by the ui-specialist's
    verification and fixed:** Ctrl+S fired through a live signature
    confirmation — the doc comment claimed a guard that didn't
    exist; Pass 4's second centre-anchored window made the collision
    reachable. Fix: one-question gate at the top of `apply()`, doc
    comments corrected; `status_is_open()` now requires a page
    (`/Count 0` nit). Escalated as an egui-tier RAG lesson
    (pending-state gates belong in the action dispatcher).
  - **Open residuals (named):** `/Alt`/`/E` counted-not-substituted;
    nested `/ActualText` outermost-wins; artifacts
    excluded-by-policy but present in runs; structure-tree order
    recognition-only; derived layout assumes axis-aligned text
    (rotated text over-produces line breaks — cannot affect sourced
    chars); canvas text-selection deferred WITH its spec written
    (verified needing no core addition — `ExtractedGlyph` already
    carries per-glyph `LadderRung` + geometry).
  - Ship stats: 5,469 new `pdfce-core` lines (`textstring.rs`,
    `text_extract/{cmap,font,page,layout,mod}.rs`). Corpus
    measurement: 2,907 files, 281,516 codes, 0 panics/timeouts —
    rung 1 78,101 (27.74%), rung 2 202,793 (72.04%), rung 3 zero,
    extension 39 (0.01%, almost all Isartor 6-3-7), failed 583
    (0.21%); **sourced total 99.78%**; derived 752 spaces / 1,905
    line breaks. **875 workspace tests (was 769)**; Pass 3.x gates
    UNMOVED; §6.1.12 44/44 with measured headroom (1,674 CMap
    singles vs 500k, 2,044 ranges vs 100k); fuzz `text_extract`
    50,215 / 61 s zero crashes, 10 targets build; NO new deps;
    `cargo-about` byte-identical. Demo: 20-page tagged manual 34,037
    codes 100% sourced in 66 ms (background-extraction concern
    measured-and-unneeded); GUI PID 41588. Pass 5 (encryption)
    promoted to In progress by decision-007 sequence;
    `pdfce-spec-librarian` §7.6 corpus session dispatched, in
    flight. Full entry: `ROADMAP.md` Shipped, Pass 4.
- **2026-08-01 (continuation 21) — Decision 008: next subsystem after
  the decision-007 read→write→edit→extract stack = Annotations &
  markup (candidate A), sliced.** Full record:
  `docs/decisions/008-next-subsystem-after-extract.md` (archived in
  parallel by another agent). Ranking across candidates
  **A ≫ B > C > E > D > F** (A = Annotations & markup,
  B = Forms/AcroForm, C = Redaction, E = Vector/Inkscape-parity,
  D = Text-&-object editing, F = Signatures/PAdES).
  - **The slice:** Pass 6.0 = annotation & widget appearance rendering
    (read-side) — IN PROGRESS, blocked on the `pdfce-spec-librarian`
    §12.5 dispatch (in flight); Pass 6.1 = authored streams + the
    project's first content-stream serializer + geometric markup
    authoring (Ink/Square/Circle/Line/Polygon/quad-point); Pass 6.2 =
    text-bearing annotations + §12.7.3.3 variable text (no `harfrust`
    per R17 — Base-14 + embedded widget widths); then Pass 7 (Forms,
    B, second overall), Pass 8 (Redaction, C), Pass 9+ (Vector, E,
    sliced a–g), Pass 5 (Encryption — repositioned to the
    fallback/interleave track AFTER Pass 7, retaining its
    decision-007 ID), Pass 10 (Signatures, F, last). Sequence recorded
    in `ROADMAP.md` "Next up → Decision 008 sequence."
  - **Census (read-only, pypdf, aggregates only, LEGAL §5 posture):**
    conformance corpus 338/2,914 files (11.6%) have annotations,
    228 with `/AP`, 127 `/AcroForm`, 4 `/XFA`; organic sample
    2,500/25,203 Dropbox files — 814 (32.6%) annotations, 753 (30.1%)
    `/AcroForm`, 43,508/55,545 annots have `/AP` (78.3%), `/Widget`
    87.8% of annots, `/Tx` 99.8% of 47,868 fields, `/SigFlags` 16
    (0.64%), `/XFA` 2 (0.08%). **Per-file figures are robust; the
    per-annotation figures are concentration-skewed and must be
    re-measured with pdfce's own tooling before any becomes a gate
    denominator** (decision 008 caveat W16). The 0.64% `/SigFlags` and
    0.08% `/XFA` shares are recorded against the Signatures and XFA
    Backlog buckets respectively; the XFA measurement answers the
    demand half only and does NOT close the standing "verify Adobe's
    XFA deprecation status" open item.
  - **Structural findings F1–F4:** **F1 — pdfce renders NO annotations
    and does not even COUNT them: an UNDISCLOSED shortfall, unique in
    the project** (everything else unsupported is R20/R27-counted;
    annotations are the one gap filed nowhere — a new "Annotation
    display (read-side)" Backlog bucket was created for it, exactly as
    text extraction was unfiled pre-decision-007). **F2 — the §8.10.1
    form-XObject execution path already shipped in Pass 1.1, and an
    `/AP` `/N` IS a form XObject** — the rendering primitive for 6.0
    already exists. **F3 — there is no content-stream writer yet and
    `Stream` cannot hold authored bytes** — Pass 6.1 builds the first
    one. **F4 — the pageops/assemble staging-buffer pattern is the
    model for authored bytes, and `DocumentView` carries a written
    assertion "a Pass that authors stream bytes must revisit this
    type"** — discharged in 6.1 by deliberately amending the type
    (R45).
  - **Standing rules added: R43–R52** (see `ROADMAP.md` Standing
    rules): R43 render-from-`/AP`-or-not-at-all (display sibling of
    R29); R44 generated appearances are written to the file, never
    rendered from a private buffer; R45 authored bytes in a session
    staging buffer, `Stream` keeps its span model, the `DocumentView`
    assertion discharged by amending the type; R46 content-stream
    serializer proven by a corpus identity gate before it authors;
    R47 an annotation edit never touches the page content stream;
    R48 flatten is destructive and discloses incremental-save
    recoverability (R35 sibling); R49 a widget is an annotation first
    (one appearance pipeline); R50 hidden annotations honored AND
    counted (forensics — the F1 fix); R51 `/NeedAppearances` disclosed,
    never silent auto-generate; R52 redaction mark and apply are
    separate operations with separate confirmations.
  - **Pass-5 repositioning:** the continuation-20 promotion of Pass 5
    (Encryption) to In progress was by the decision-007 SEQUENCE;
    decision 008 supersedes that sequencing and moves Pass 5 to the
    fallback/interleave track after Pass 7. Pass 5 keeps its 007 ID
    (never renumbered); its scope and the 0.67% `/Encrypt` census are
    unchanged. A dated append-only correction note is at the
    `ROADMAP.md` In-progress (Pass 6.0) entry; the Pass 4 Shipped entry
    is NOT rewritten.
  - **Owed (logged, NOT fixed this filing) — two §4 staleness items
    surfaced by decision 008:** (1) `ARCHITECTURE.md` §4 still
    describes the Pass-0 header-probe state ("Current state as of
    Pass 0") — decisions 001, 004, and 005 owe their §4 core-data-model
    integration (the real `Document`/`Object`/`StreamData`/font/codec
    surface shipped across Passes 1–2 is not yet reflected in the §4
    body). (2) A consolidation session is owed to integrate that
    accumulated reality into §4 **before** the annotation data model is
    documented there — so the annotation types (§12.5 annotation dict,
    `/AP` appearance selection, the flag set, the authored-stream
    staging buffer of R45) land in a §4 that already reflects Passes
    1–5, not on top of a Pass-0 stub. Neither is fixed here; both are
    recorded as owed.
- **2026-08-01 (continuation 22) — §7.6 encryption spec-corpus session
  complete; Pass 5 spec-unblocked (still queue-deferred behind Pass 7
  per decision 008).** Spec-corpus work, not a code Pass — pointer only.
  `pdfce-spec-librarian` built the §7.6 corpus at
  `D:\Dev\Rag-Specialized\PDF_Spec\` (7 new + 2 updated files;
  `iso32000__s__7.6.1`–`7.6.5`, new `security__aes256_r5_r6.md` under a
  new `security__` prefix, `iso32000__ref__encryption_impl.md` derived
  checklist, `filter__crypt.md` de-stubbed; Adobe ExtensionLevel 3
  supplement staged). Closes the "§7.6 largest spec gap" prerequisite
  the Encryption Backlog bucket named. **`/R 6` (AES-256, Acrobat X+)
  could NOT be sourced** (ISO 32000-2 paywalled; no public
  ExtensionLevel 8; pdfa.org 403) — the agent correctly REFUSED to
  reconstruct Algorithm 2.B from memory. Consequence + the three
  AES-256-write options are a Pass-5 open sub-decision, recorded at the
  `ROADMAP.md` Encryption Backlog bucket; two operator decisions
  (LEGAL.md §2 Adobe-supplement copyright contradiction; `/R 6` sourcing
  method) at SESSION_LOG continuation 22's operator-items list. Full
  record: SESSION_LOG continuation 22.
- **2026-08-01 (continuation 23) — Pass 6.0 shipped (annotation &
  widget appearance rendering, read-side): render every existing
  `/AP` `/N`, count every annotation, author NOTHING (R43).** The
  direct remedy for decision 008 finding F1 (annotations were the
  project's one undisclosed, uncounted shortfall). F2 confirmed in
  code: an `/AP` `/N` IS a form XObject, so §12.5.5 placement routes
  through the EXISTING Pass 1.1 `interpret::run_form_at` → `do_form`
  path — X8 resource scoping, cycle guard, `MAX_XOBJECT_DEPTH`, and the
  per-form font cache all inherited unchanged, pinned by
  `appearance_uses_its_own_resources_not_the_page_font`. New surface:
  `pdfce-core` `annot.rs` (§12.5 walk/model/select, `AnnotFlags`
  §12.5.3, the `/AP`→`/N` + `/AS` `Appearance` taxonomy, the
  document-scoped `need_appearances` query); `pdfce-render` `annot.rs`;
  `RenderOptions.annotations` + `RenderOptions::with_annotations`;
  Diagnostics +8 keys; CLI `render-page --no-annotations` +
  `list-annotations`; GUI toolbar visibility toggle + status-bar
  disclosure; fuzz target 11 `annot_walk.rs` (1.1M runs, 0 crashes).
  - **(a) Census baseline PINNED (pdfce-native; supersedes decision
    008's pypdf conformance figures per W16, which is now DISCHARGED
    for the conformance corpus):** 2,914 files, ZERO panics — 338 with
    annotations / 429 annotations / 224 USABLE `/AP` `/N` / 127
    `/AcroForm` / 34 `/Popup` / 87 `/Widget`. The per-file 338 and 127
    match pypdf exactly. **The 224-vs-228 (pypdf) `/AP` gap is
    DEFINITIONAL, not an error:** pdfce counts a *usable* `/AP` `/N`
    (resolvable stream / selectable `/AS` state), pypdf counts raw
    `/AP`-key presence; pdfce's predicate is stronger. Filed as a
    `personal_rag/pdf` finding.
  - **(b) Durable FIVE-way GUI placement taxonomy (ui-specialist
    deliverable — THE settled convention, resolves the X14 drift;
    supersedes/extends the continuation-20 three-way rule):**
    **view-state → toolbar view group; edit → toolbar/window;
    selection-scoped → rail; advanced → Tools dock; disclosure →
    status bar.** All future GUI placement decisions follow this. The
    Pass-6.0 GUI (visibility toggle = view-state → toolbar view group;
    annotation diagnostics = disclosure → status bar) is the first
    instance built to it.
  - **(c) Deviations, all named/counted (fuzzy-never-sneaky):**
    (1) `/NoZoom`/`/NoRotate` post-annotation-matrix transform DEFERRED
    — counted + named (`annotation_notes`); rare, near-exclusively on
    icon subtypes lacking `/AP` anyway; a wrong post-transform is worse
    than a disclosed omission. (2) `/OC` optional-content visibility
    test not implemented — consistent with the renderer implementing NO
    optional content anywhere (BDC/EMC deferred; §8.11 is a RAG GAP);
    an OC-off annotation currently paints, named. (3)
    `need_appearances_documents` is a document-scoped query, not folded
    into per-page render `Diagnostics` (inherently document-level). (4)
    GUI diagnostics are a separate always-evaluated status line below
    the content-diagnostics header, NOT folded into the content
    unsupported-tally (chosen to avoid destabilizing the tested content
    clean-return path; still honest R50/R27/R51; flagged for future
    ui-specialist refinement).
  - **(d) Placement correctness (X2), NOT a pixel-parity close:**
    `tools/annot-pdfium-diff.py` (pypdfium2, decision 006 §3.2
    precedent) — 7/7 pure-geometry placement fixtures agree with pdfium
    within 4 px, 0 mismatches; 6 blank-expected cases correctly blank.
    This is an ink-bbox differential on the annotation subset ONLY; the
    Pass 1.1 full-page pixel-parity remainder stays OWED — explicitly
    NOT claimed closed.
  - **(e) Guards:** new `MAX_ANNOTS_PER_PAGE = 1_000_000` (pure memory
    backstop — Annex C imposes no limit, §6.1.12 forbids imposing one;
    busiest corpus page ≪100, >10,000× headroom); `/AP` recursion
    reuses `MAX_XOBJECT_DEPTH = 64` unchanged. **R34 holds
    STRUCTURALLY** — no pinned reference raster exists; the round-trip
    oracle is a runtime self-comparison, so painting annotations
    perturbs nothing the Pass 3.x/4 gates measure.
  - **RAG escalations (`C:\personal_rag\pdf\`):** (a) pdfium requires
    `FPDF_FFLDraw` to render `/Widget` appearances — a differential-
    harness gotcha (the two apparent pdfium diffs were REFERENCE
    divergences, not pdfce errors: pdfium SYNTHESIZES the no-`/AP`
    `/Circle` `/IC` fill that R43 makes pdfce refuse); (b) QuadPoints
    CCW-vs-Z-order unresolved (§12.5.6 says CCW, real producers/Acrobat
    emit Z/reading order — only bites Pass 6.1 generation).
  - Ship stats: **901 workspace tests (was 875)**; fmt/clippy clean;
    GUI-free host + msvc; wasm32; `--duplicates`; `ui-strings`;
    `no-network` all clean. Pass 6.1 (authored streams + content-stream
    serializer + geometric markup) promoted to In progress, blocked on
    the §8.10.2 form-XObject WRITE-direction audit
    (`pdfce-spec-librarian` dispatched, in flight); the "Comments &
    markup" acrobat bucket is complete. Full entry: `ROADMAP.md`
    Shipped, Pass 6.0.
- **2026-08-01 (continuation 24) — Pass 6.1 shipped (authored streams +
  content-stream serializer + geometric markup authoring): the
  project's FIRST content-stream authoring path.** Discharges decision
  008 findings **F3** (there was no content-stream serializer, and
  `Stream` could not hold authored bytes) and **F4/R45** (authored
  bytes are staged, not stored by mutating the span-provenanced `Stream`
  type). Authors the pure-geometry markup annotations (Ink, Square,
  Circle, Line, Polygon, PolyLine + the quad-point family
  Highlight/Underline/StrikeOut/Squiggly); text-bearing annotations and
  §12.7.3.3 variable text are deferred to Pass 6.2 (one appearance
  pipeline). New surface: `writer/content.rs` (`ContentBuilder` — the
  §8.2 path/paint/graphics-state/colour operator set + the §8.10 WF6
  form-XObject ordering from the unblocking WRITE-direction audit),
  `annot_author.rs` (`MarkupSpec`/`Color`/`Quad`/`LineEnding`/
  `TextMarkupKind`; `build_appearance` → `AuthoredAppearance` =
  annotation dict + `/AP` `/N` form XObject + content bytes). Modified:
  `writer/serialize.rs` primitives promoted to `pub(crate)`;
  `DirtySet` gains the R45 staging buffer + `combined_source()`;
  `writer/save.rs` serializes against base++staging; `edit.rs` gains
  `EditSession::add_markup` + `authored_source()` + COW `/Annots`
  patching + `AnnotKind` + `CommandKind::AddAnnotation` + three named
  `EditError`s; `pageops/assemble.rs`'s `DocumentView` doc comment is
  **amended to discharge — not delete —** the R45 written assertion.
  CLI `annotate`; GUI minimal "Markup ▾" menu; fuzz target 12.
  - **(a) The content-stream serializer is proven before it authors
    (R46) — this is the load-bearing architectural fact of the Pass.**
    The R46 corpus identity gate re-serializes EVERY existing content
    stream and requires byte-faithful reproduction before the writer is
    trusted to author: **12,936 streams / 2,898 files → 12,854
    byte-identical (99.37%) / 82 non-identical (0.63%) / 0 corrupted →
    PASS.** The 82 are all spec-legal, VALUE-PRESERVED number
    re-spellings, enumerated by file+reason (R20): `.05`→`0.05` (20×),
    `-0`→`0` (18×), one 300-digit pathological real, `1.`→`1.0`.
    **Architectural framing (records why this is NOT a §5 round-trip
    violation):** R46 is a SERIALIZER-correctness test that deliberately
    re-emits every stream. §5's minimal-diff invariant means pdfce
    **never re-serializes untouched page content in normal save** —
    span re-emission passes it through byte-verbatim — and authoring
    writes only NEW streams. So these 82 divergences are structurally
    unreachable in production save; X6 (silent normalization of content
    pdfce claims to preserve) is caught mechanically.
  - **(b) R45 staging-buffer reality — the F4 assertion discharged by
    amendment, not deletion.** `Stream` keeps its (offset, len) span
    model; authored bytes accumulate in a per-`DirtySet` staging buffer
    (the `pageops/assemble` pattern generalized), and `save` serializes
    replacement/created objects against `combined_source()` =
    base++staging. The `DocumentView` "a Pass that authors stream bytes
    must revisit this type" assertion (F4) is discharged by a named,
    reviewed doc-comment amendment — the deliberate change R45
    anticipated, never a silent widening of `Stream` into a
    bytes-owning type.
  - **(c) R44 authored-appearance identity holds end-to-end.** Author →
    save → reload → paint round-trips: an authored square/highlight/ink
    reloads and renders through Pass 6.0's read path (annots=3/painted=3/
    forms=3; red square paints red), `undo_identical=1` on the first
    author (minimal-diff), and extract-from-session (X5) resolves the
    authored appearance BYTE-EXACT via `authored_source()`. Every
    authored look is a real baked `/AP` `/N`; there is no second private
    render path (R44).
  - **(d) QuadPoints authoring convention DECIDED — Z / reading order
    (UL, UR, LL, LR).** Closes the continuation-23 carried open item
    (§12.5.6 spec's CCW vs the Z/reading order real producers/Acrobat
    emit). pdfce authors in Z order for maximum third-party interop
    (Acrobat/PDFBox/pdf.js), documented in `annot_author.rs`. Because
    pdfce's own render paints the baked `/AP` and never re-derives
    geometry from QuadPoints, the choice is an interop decision, not a
    correctness one — render is convention-independent.
  - **(e) Deviations/residuals (fuzzy-never-sneaky, all named):** X11
    certification gating is CONSERVATIVE (reuses `check_certification()`,
    over-refuses annotation-add that `/DocMDP` `/P 3` permits;
    fail-clean-safe; per-`/P` refinement scoped, §12.8 already sourced);
    X10 encryption guard is a forward-compat R37 seam (encrypted files
    refused at LOAD, so `DocumentEncrypted` in `add_markup` is
    unreachable until Pass 5); no `/M`//`CreationDate` on authored
    annotations (avoids clock non-determinism in byte-compare tests);
    line-ending set limited to None/OpenArrow/ClosedArrow; default
    colours are pdfce's own except the sourced Highlight yellow+Multiply.
  - **Ship stats: 939 workspace tests (was 901)**; fmt/clippy clean;
    **R34 re-runs GREEN** (identity + raster unperturbed — authoring
    touches no existing object); GUI-free host + msvc; wasm32;
    `--duplicates`; `ui-strings`; `no-network` all clean; fuzz target 12
    696,098 runs / 61 s, 0 crashes; **ZERO new dependencies** (hand-
    rolled, no `harfrust`; `THIRD_PARTY_LICENSES.md` unchanged). Pass
    6.2 (text-bearing annotations + §12.7.3.3 variable text) promoted to
    In progress, blocked on the §12.7.3.3 variable-text spec
    (`pdfce-spec-librarian` dispatched, in flight). Full entry:
    `ROADMAP.md` Shipped, Pass 6.1.
- **2026-08-01 (continuation 25) — Pass 6.2 shipped (text-bearing
  annotations + §12.7.3.3 variable-text appearance generation): the
  decision-008 6.x annotation arc is COMPLETE.** 6.0 (display) → 6.1
  (geometry) → 6.2 (text) are all shipped; In progress advances to Pass 7
  (Forms/AcroForm). Adds the text-bearing annotation subtypes Pass 6.1
  deferred — FreeText, Text (sticky note), Stamp — plus the shared
  §12.7.3.3 variable-text pipeline. New surface: **`vartext.rs`** — the
  §12.7.3.3 variable-text pipeline (`/DA` default-appearance parsing, the
  auto-font-size `0` rule, field-value → appearance-stream layout with
  line breaking / `/Q` quadding / baseline placement). Modified:
  `writer/content.rs` (`ContentBuilder` gains the text/marked-content/
  clip/matrix operator set BT/ET/Tf/Td/TD/TL/Tj/Tc/Tw/Tz/q/Q/BMC/EMC/W/cm
  + `emit_literal_string`); `annot_author.rs` (`TextAnnotSpec` +
  `StickyIcon`/`StampName`/`AuthoredTextAnnot`/`build_text_annotation`);
  `edit.rs` (`EditSession::add_text_annotation` + `AnnotKind::{FreeText,
  Text,Stamp}` + `EditError::VariableText`). CLI `annotate --type
  freetext|text|stamp`; GUI "Text ▾" menu + modeless text-entry popup.
  - **(a) `vartext.rs` is the ONE appearance generator Pass 7 reuses —
    the load-bearing architectural fact of the Pass.** The §12.7.3.3
    variable-text procedure is written once, here, as the shared FreeText
    + widget-field appearance generator. Pass 7 (Forms) wires it to the
    `/AcroForm` field model rather than re-implementing appearance
    generation (R49 — a widget is an annotation first; one appearance
    pipeline for widgets and annotations alike). This is why 6.2 precedes
    7 in the decision-008 sequence: the appearance half is earned before
    the field model needs it.
  - **(b) The content-stream operator additions are PURELY ADDITIVE —
    the R46 identity result is preserved by construction.**
    `ContentBuilder` gains text/clip/matrix/marked-content emit methods,
    but the R46 re-emission path (`reemit_canonical` /
    `emit_token_canonical` / `number_divergence_reason` / `emit_number`)
    is byte-unchanged. The orchestrator's full-corpus R46 re-run
    (2026-08-01, over `fixtures/external` — 3,020 files, veraPDF-corpus +
    pdf20examples) is **GATE PASS, zero corruptions**, all divergences the
    same value-preserving number re-spellings Pass 6.1 catalogued — so the
    additive-only claim is confirmed BY MEASUREMENT, not merely by
    inspection. (The engineer's earlier "corpus not present" was a
    path-resolution miss; the corpus is present and runnable at
    `fixtures/external`, a standing note for future Passes.)
  - **(c) The bare-Base-14 modality choice.** A FreeText appearance is
    authored against a Base-14 font dict with **no embedded font program**
    — no `/FontDescriptor`, no `/Widths` — relying on §9.6.2.1's
    reader-shall-supply-standard-metrics rule (the PDF-1.5 should-embed
    deprecation is a *should*, honoured as a named modality choice, not a
    *shall*). The one deviation from a literal 3-key dict is `+/Encoding
    /WinAnsiEncoding` (4-key), added for deterministic Latin byte→glyph so
    the pipeline can assert real glyph pixels
    (`authored_freetext_paints_glyph_pixels_after_reload_r44`: >100 dark
    glyph pixels through the Pass 6.0 read path) and measure `/Q` against
    AFM widths ("AV" Helvetica = 13.34 pt). The dict stays program-free —
    the gate's real meaning. Base-14 is LATIN-only (no `harfrust`, R17;
    non-WinAnsi chars → "?" counted as `unencodable_chars`).
  - **(d) The auto-size VT1 heuristic is implementation-defined, counted,
    never presented as spec-mandated.** §12.7.3.3's auto-font-size (`/DA`
    text size `0`) has no spec formula (S-class spec silence). pdfce uses
    `auto_size(rect_h) = ((rect_h − 2·PAD)/1.15).clamp(4.0,12.0)`
    (`PAD = 2`, line-factor 1.15); every generated appearance reports
    `applied_autosize` so the operator sees the derived value. This is the
    general pattern for spec-silent layout parameters — pick a reviewable
    heuristic, count it, surface it (fuzzy, never sneaky).
  - **(e) Deviations/residuals (all named):** text specs live in a
    SEPARATE `TextAnnotSpec` enum, NOT folded into `MarkupSpec`, so the
    R46/R34-proven geometric `add_markup` path + its exhaustive match arms
    stay byte-unchanged (text needs `/DA`, popup, `/NoZoom`/`/NoRotate`);
    `/M`//`/CreationDate` still omitted (clock non-determinism in
    byte-compare tests); `/RC` rich text recognition-only (VT3 non-goal);
    no comb fields (Pass 7; comb = field-flag bit 25 = 16777216); X11
    certification gating still conservative; X10 encryption refusal still
    the load-time R37 seam.
  - **Ship stats: 971 workspace tests (was 939)**; fmt/clippy clean;
    GUI-free core+render (zero egui/eframe/winit/wgpu); wasm32;
    `--duplicates`; `ui-strings`; `no-network` all clean; fuzz
    `annot_author` extended (`/DA` parsing + text-appearance gen) 13,871
    runs / 61 s, 0 crashes; **no new §6.1.12 guards**; **ZERO new
    dependencies** (Base-14 only, no `harfrust`; `THIRD_PARTY_LICENSES.md`
    unchanged). **Pass 7 (Forms/AcroForm) promoted to In progress**,
    blocked on two prerequisites both dispatched in parallel (the
    §12.7.1–12.7.4 form-field spec via `pdfce-spec-librarian` + the "Forms
    (AcroForm)" acrobat parity bucket); the embedded-JavaScript posture is
    an open Pass-7 security sub-decision (recommend never-execute —
    recognize + disclose). Full entry: `ROADMAP.md` Shipped, Pass 6.2.
- **2026-08-01 (continuation 26) — Pass 7.0 shipped (AcroForm field model
  + text/checkbox fill: the forms FOUNDATIONAL SLICE) AND decision 009
  (embedded form/document JavaScript posture) filed.** Pass 7 was split on
  ship: 7.0 = the field-model read path + the dominant fill path; the
  residuals become Pass 7.1 ("completes the forms subsystem", now In
  progress). This is NOT "Forms shipped."
  - **(a) `forms.rs` — the `/AcroForm` field model (~1,050 lines, 13
    tests).** `parse_acroform(graph)` walks `/AcroForm` → `/Fields` DFS
    with §12.7.3.1 inheritance of `/FT`//`/V`//`/DV`//`/Ff`//`/DA`//`/Q`
    down `/Kids` via `/Parent`, building the dotted fully-qualified field
    name (§12.7.3.2). **Generic over `ObjectGraph`** so it runs against
    both a loaded `Document` and an `EditSession` overlay — the same
    graph-abstraction Pass 3.2 introduced.
  - **(b) The field-vs-widget MERGE is the load-bearing model fact
    (R49).** *Shape A* — a terminal field with a single associated widget
    merges field dict + widget dict into ONE dictionary (empirically ~88%
    of real fields). *Shape B* — a field carrying a `/Kids` array of
    widget annotations keeps field and widgets separate. A reader that
    always expects `/Kids` widgets breaks on the Shape-A common case; this
    is escalated as a `personal_rag/pdf` parsing lesson. `FieldFlags` bits
    are pinned verbatim by test (§12.7.4.2 Table 226 / §12.7.4.2.1: Radio
    32768, Pushbutton 65536, NoToggleToOff 16384, RadiosInUnison
    33554432; Multiline 4096, Comb 16777216; Combo 131072, MultiSelect
    2097152). XFA is **detect-only** (`XfaPresence` — recognized, never
    parsed).
  - **(c) Fill reuses the ONE §12.7.3.3 appearance generator (R49).**
    `fill_text_field` sets `/V` and regenerates `/AP` for every widget via
    Pass 6.2's `vartext.rs`, wrapped by
    `annot_author::build_field_text_appearance` (the `/DA` font resolved
    from `/DR` via `basefont_to_std14`). `set_button_state` selects
    checkbox/radio `/V` + `/AS` with no regen (on/off appearances already
    exist in the widget `/AP` sub-dict), honoring RadiosInUnison and the
    `/Off` convention. There is no second widget-only appearance path —
    the appearance half was earned in 6.2 before the field model needed
    it. R44 form-fill proof: reload → `render-page` paints 11 real glyphs,
    `annots_painted=2 forms=2`.
  - **(d) The `/P`-aware fill certification gate.**
    `check_certification_for_fill` permits fill at `/DocMDP` `/P >= 2`
    (including absent = 2 by §12.8.1 default), refuses by name at `/P 1`,
    and refuses on any `/FieldMDP` — the structural gate stays STRICT.
    Proven by `certification_p2_permits_fill_p1_refuses`. This is the
    per-`/P` refinement the Pass 6.1/6.2 X11 residual scoped, now applied
    to the fill path (annotation-add gating stays conservative until its
    own refinement Pass).
  - **(e) Decision 009 honored structurally — fill never touches the
    AcroForm dict.** Fill mutates only `/V`//`/AP`//`/AS`, so `/CO`//`/AA`
    //`/Names /JavaScript` re-emit byte-verbatim under incremental save.
    `has_additional_actions` + `calc_order_count` are surfaced
    recognition-only; the full JS-disclosure histogram is Pass 7.1.
  - **(f) Additivity preserves R34/R46.** A new module + additive
    methods/variants + one new `pub fn`; the re-emission path and
    `add_markup`/`add_text_annotation` are byte-unchanged. Full-corpus R46
    re-run 2026-08-01 post-7.0 over `fixtures/external` = **GATE PASS,
    additivity confirmed by measurement** (fill authors new `/AP` streams
    via the proven §12.7.3.3 generator; the re-emission path is
    byte-unchanged), discharging the "R46 re-run owed" residual. R34 (Pass
    3.0 roundtrip) accepted as additivity-preserved, not separately re-run.
  - **DECISION 009 (embedded form/document JavaScript) — the security
    posture, filed this continuation.** Archived at
    `docs/decisions/009-forms-javascript-posture.md`; discharges the
    decision-008 §5.1 embedded-JS scope trap and the Pass 6.2 open
    sub-decision. **Outcome: NEVER execute embedded PDF JavaScript** —
    field `/AA`, document `/AA`, `/OpenAction`, `/Names /JavaScript`,
    built-in or custom, on load or interaction. This is fully
    ISO-conformant: §12.6.4.16 is a **"hollow shall"** — it mandates
    execution but defines no JS semantics/API/DOM/security model
    (deferring to two external non-ISO documents), specifying only the
    carrier (Table 217) and hook points (§12.6.3, `/AA`, `/CO`, `/Names
    /JavaScript`); there is no normative JS behavior to conform to.
    **Phased hybrid:** posture A (recognize + classify + disclose +
    byte-exact round-trip, zero execution) is the mandatory floor and
    Pass 7's entire JS scope; posture B (native Rust recompute of an
    exact-match whitelist — `AFSimple_Calculate` SUM/AVG/PRD/MIN/MAX
    changes `/V`, `AF*_Format` changes display only — opt-in,
    off-by-default per document, every recompute a reviewable/undoable
    `EditSession` edit leaving the source script in place) is deferred to
    a demand-driven Pass 7.x; posture C (a sandboxed JS engine) is
    REJECTED and made a standing prohibition (re-imports the attack
    surface Adobe's broker process contains; hook points reference
    `/URI`//`/SubmitForm`//`/ImportData`//`/Launch` which R12/R13 forbid;
    nothing to conform to). Adds **standing rules R53–R57** (decision
    009's R-JS-1…R-JS-5, renumbered next-free after R52). Spec
    prerequisites (verify §12.6 carrier/hook coverage + formalize the
    hollow-shall finding via `pdfce-spec-librarian`; source the `AF*`
    helper shapes via `pdfce-acrobat-librarian`; confirm PDF/A forbids JS
    actions) are queued for Pass 7.x, non-blocking for posture A.
  - **Ship stats: 601 `pdfce-core` lib tests (was 582; +13 model +6
    fill)** + integration green; fmt/clippy clean (core + cli); GUI-free
    core+render (zero egui/eframe/winit/wgpu); **ZERO new dependencies**;
    fuzz target 13 `form_model` 1,306,476 runs / 61 s, 0 crashes;
    real-corpus `list-fields` clean on all `/AcroForm` files; veraPDF
    §6.1.12 two new guards (`MAX_FORM_FIELDS = 500_000` /
    `MAX_FIELD_TREE_DEPTH = 64`) are pure memory backstops (corpus max ≈
    63 fields/file). **Pass 7.1 promoted to In progress.** Full entries:
    `ROADMAP.md` Shipped Pass 7.0 + Standing rules R53–R57.
- **2026-08-01 (continuation 27) — Pass 7.1 shipped (form flatten +
  FDF/XFDF + choice fields + regenerate-all): the AcroForm subsystem CORE
  is COMPLETE.** 7.0 (field model + text/checkbox fill) + 7.1 (flatten +
  data interchange + choice fields + regenerate-all) finish the forms
  core; the remaining forms items (GUI form-fill slice, field
  auto-detection, posture-B native recompute) are FOLLOW-UP SLICES tracked
  in Backlog, not core. New surface: **`fdf.rs`** (~700 lines — FDF §12.7.7
  reusing `crate::parser::Parser`; XFDF via a hand-rolled ~200-line scoped
  XML reader, ZERO new deps per rule 13); `edit.rs`
  (`set_choice_value`/`regenerate_appearances`/`flatten_fields`/
  `export_form_data`/`import_form_data` + flatten helpers + §12.5.5
  `fit_matrix_for`; `RegenOutcome`/`ImportOutcome`/`FlattenOutcome`);
  `forms.rs` (`scan_javascript` + `FormJavaScript` histogram);
  `writer/content.rs` (`ContentBuilder::invoke_xobject`, `/Name Do`); CLI
  `regenerate-appearances`/`flatten`/`export-data`/`import-data` + choice
  routing + `|`-multi-select fill.
  - **(a) Flatten burns in by overlay-APPEND, not content-stream surgery —
    the load-bearing design fact of the Pass (full record §5.8).** Flatten
    appends a NEW overlay content stream to the page `/Contents` array that
    `Do`-invokes the widget's existing `/AP` `/N` as a page XObject
    (`invoke_xobject`), rather than rewriting the existing page content
    stream. The pre-existing content bytes are never re-serialized, so
    **the R46 re-emit-everything gate finds ZERO flattened-page
    exceptions** (GATE PASS over `fixtures/synthetic` + `fixtures/external`,
    all divergences the known value-preserving `-0`→`0` re-spellings, 0
    corruptions). This is MORE minimal-diff than the in-place surgery the
    Pass scope anticipated. **General pattern:** overlay-append beats
    content-stream-surgery for ADDITIVE burn-in; reserve in-place surgery
    for REMOVAL (Redaction, Pass 8 — the R46 named exception). The two are
    mirror images: flatten adds without rewriting; redaction removes and
    must rewrite (and decompose containers, §5.7).
  - **(b) R48 honored, STRICT cert gate for flatten.** Incremental flatten
    leaves the field dict recoverable in the prior revision (disclosed);
    `--full-rewrite` output has no `/FT`/`/Tx` yet renders the burned
    value. Flatten refuses on ANY enforced `/DocMDP` (incl. `/P 2`
    certified, by test) — the STRICT gate, NOT the fill path's `/P >= 2`
    permit, because flatten is a STRUCTURAL change, not a value fill.
  - **(c) FDF/XFDF interchange with ZERO new dependencies.** FDF is
    PDF-syntax, so the reader reuses `crate::parser::Parser`; XFDF gets a
    hand-rolled ~200-line scoped XML reader (5 predefined entities, numeric
    char refs, comments, `<?xml?>`/DOCTYPE skip, MAX_XML_DEPTH-guarded) —
    classified per rule 13, `quick-xml`/`roxmltree` declined for a reader
    this small and scoped. Round-trip: fill → export FDF+XFDF → re-import →
    identical `/V` + regenerated appearances; import SKIPS fields the doc
    lacks (counted, never an error).
  - **(d) Choice-field value matrix.** Single-select combo → `/V` = EXPORT
    value, `/I=[idx]`, appearance shows DISPLAY value; multi-select list →
    `/V` array + `/I` array; single-value on a multiselect-required field →
    `ChoiceRequiresMultiSelect` refusal; unknown value on a non-editable
    field → `ChoiceValueNotInOptions`; editable combo (`Combo|Edit`)
    accepts free text with no `/I`.
  - **(e) JS-disclosure histogram = posture A only (decision 009).**
    `scan_javascript` + the `FormJavaScript` histogram COUNT all
    field-level JS actions (recognition-only) with a loud stderr flag on
    any network/launch `/AA` action; NO whitelist recompute — posture B
    stays a demand-driven Pass 7.x follow-up.
  - **(f) Deviations/residuals (named):** flatten overlay-append is a
    POSITIVE deviation (more minimal-diff than scoped); list-box
    multi-select appearance is a simplified display-text newline-join, not
    the §12.7.4.4 highlight-rectangle rendering; corpus flatten-burn
    coverage thin (synthetic fixtures + unit tests carry it); import
    applies as per-field undoable commands, not one atomic
    `ImportFormData`.
  - **Windows toolchain gotcha (RAG-escalated `D:\dev\rag\rust\`):** adding
    the CLI subcommands overflowed the DEBUG `pdfce-cli` main-thread stack
    (clap's `debug_assert` command-tree recursion vs the MSVC ~1 MB main
    stack), surfacing as `TryFromIntError(NegOverflow)` in integration
    tests; fixed by running `main()` on a 16 MB worker thread.
  - **Ship stats: 1,010 workspace tests (core lib 620, was 601)**;
    fmt/clippy clean; GUI-free core+render (zero egui/eframe/winit/wgpu);
    wasm32; `--duplicates`; `no-network` clean; `ui-strings` N/A (no GUI
    changes); R34 (Pass 3.0 identity) green + R46 re-emit-everything GATE
    PASS (additive — flatten appends); fuzz target 14 `fdf_parse` 624,202
    runs / 61 s, 0 crashes; veraPDF §6.1.12 N/A (MAX_XML_DEPTH is
    XFDF-only, outside PDF-conformance scope); **ZERO new dependencies**,
    `THIRD_PARTY_LICENSES.md` unchanged. **AcroForm CORE COMPLETE; Pass 8
    (Redaction) promoted to In progress** — the standing R35 obligation and
    the one truly destructive op, blocked on two prerequisites dispatched
    in parallel (Redaction acrobat-parity bucket + a redaction spec
    dispatch for container-decomposition + `/Redact`-apply semantics). Full
    entry: `ROADMAP.md` Shipped Pass 7.1; the flatten design: §5.8.
- **2026-08-01 (continuation 28) — Pass 8.0 shipped (Redaction — mark +
  apply, text + region): the highest-stakes Pass, and the cardinal rule
  held — never claim redacted what isn't.** This discharges the standing
  **R35** obligation and is the ONE operation whose contract is genuine
  REMOVAL (§5's sole deliberate exception; R46's one named content-stream-
  surgery exception). New surface: **`redact.rs`** (the self-contained
  advance-preserving content-stream surgery interpreter + apply
  orchestration + carrier sweep + container decomposition + `RedactionReport`
  + `count_redaction_marks`); `edit.rs` (`add_redaction`,
  `mark_redactions_by_search`/`_by_pattern`, `find_matches`/
  `find_pattern_matches`); `annot_author.rs` (`RedactSpec` +
  `build_redact_mark` — RED-OUTLINE preview, never a solid fill);
  `text_extract/font.rs` (exposed per-code width/codes/to_unicode as
  `pub(crate)` for the advance computation); CLI `redact-mark` / `redact-apply`
  / `list-redactions`; fuzz target 15 `redact_apply`.
  - **(a) Redaction is the MIRROR IMAGE of Pass 7.1's flatten (§5.8/§5.9).**
    Flatten ADDS by overlay-append and never touches page content; redaction
    REMOVES and is the one op that DOES rewrite existing page content (R46's
    named exception). The surgery interpreter is a NEW code path — the R34/R46
    identity/re-emission paths (`writer/` + `content.rs`) are byte-unchanged,
    so additive preservation holds.
  - **(b) Advance-preserving content-stream surgery — the load-bearing
    correctness fact.** Deleting a text-showing run does NOT shift surviving
    same-line text: the removed `Tj` is replaced by a `TJ` offset consuming
    the exact advance `N = −Σtx·1000/(Tfs·Th)`. Proven visually (redacted
    "SECRET" is a baked black box; "dossier"/"PUBLIC" sit exactly where they
    were) AND numerically (survivor x-origin moved <1.0 pt). Escalated as a
    `personal_rag/pdf` lesson.
  - **(c) The absence-proof acceptance gate (R46 INVERTED).** Redaction's
    four-shalls are embodied as an executable gate: grep the WHOLE saved
    output — raw bytes AND every decoded content stream — for the redacted
    bytes → zero. Demo on `demo-secret.pdf`: `redact-apply` →
    `glyphs_removed=21 info_strings_scrubbed=1`; `grep "SECRET" redacted.pdf`
    = **0** (control `marked.pdf` = 3). R46 proves presence for untouched
    content; the absence test proves deletion for redacted content. Codified
    as standing rule R58 obligation 3 and §5.9. Escalated as a
    `personal_rag/pdf` lesson.
  - **(d) Container decomposition (§7.5.7 Strategy B) is necessary — R35 is
    not sufficient (§5.7).** A redacted `/Info` compressed in an `/ObjStm`
    survives verbatim in BOTH save modes unless its container is decomposed;
    the test asserts absence AND `containers_decomposed >= 1` (promote
    survivors, drop the container). Forced full rewrite (R35): output has no
    `/Prev`, prior revisions dropped, every carrier scrub rides `save_full`.
  - **(e) Refuse-not-false-redact posture.** Images in a redaction region are
    REFUSED by name (`RedactError::ImageRegion`, NO output written) rather
    than overlay-and-leave-pixels — never falsely claim a raster region
    redacted when only a masking box covers intact pixels. Form-XObject
    content in-region is NOT surgically redacted and is disclosed loudly
    (`form_intersect`), never claimed removed. XFA / structure-tree
    `/ActualText` / attachments are detect + disclose (`DISCLOSED_NOT_SCRUBBED`)
    this cut, gated by `--acknowledge-residuals` (CLI exit 10
    `REDACTION_RESIDUALS` otherwise — ui-spec §4.4).
  - **(f) Carrier sweep / report.** `/Info` + XMP SCRUBBED (asserted absent);
    object-streams + prior-revisions DROPPED-BY-REWRITE; OCG
    REDACTED-BY-GEOMETRY (ignores `/OC` visibility); XFA/struct-tree/
    attachments DETECTED + DISCLOSED. The "diligence carriers" a naive
    region-redact misses are escalated as a `personal_rag/pdf` lesson.
  - **(g) GUI — the ONE non-negotiable item shipped:** a persistent status-bar
    disclosure of unapplied `/Redact` marks, computed from the document's own
    annotations — targeting the #1 real-world redaction failure (saving a
    marked-but-not-applied document believing it is redacted). The GUI
    apply-button + canvas marking are DEFERRED to the named GUI follow-up
    (they depend on the Pass 6.1 canvas tool-mode that never shipped; the
    engineer correctly did NOT build a parallel drag tool).
  - **NEW STANDING RULE R58 (generalizes R35 — the ui-specialist finding).**
    Every removal/scrub operation rides R35's forced FULL REWRITE, including
    any future Sanitize / Remove-Hidden-Information Pass, because an
    incremental save leaves the "removed" content recoverable in the prior
    revision. Full text: §5.9 (which generalizes §5.2's R35); ROADMAP Standing
    rules R58.
  - **(h) Deviations/residuals (all disclosed, none silent):** image
    refuse-not-clear (named safe choice); `/RO`+`/OverlayText` burn-in
    deferred — apply draws the `/IC`/default-black fill (Acrobat default),
    overlay-text LABEL not drawn (COSMETIC only, content removed regardless,
    disclosed at mark time); form-XObject in-region not surgically redacted
    (disclosed); XFA/struct-tree/attachments detect+disclose not scrubbed;
    GUI apply-button + canvas marking deferred.
  - **Ship stats: 1,018 workspace tests (+8)**; fmt/clippy clean workspace-
    wide; GUI-free core+render (zero egui/eframe/winit/wgpu); wasm32;
    `--duplicates`; `no-network`; `ui-strings` clean; R34/R46 additive-
    preserved (re-emission paths + gates byte-unchanged); fuzz target 15
    `redact_apply` 9,262 runs / 61 s, 0 crashes (multi-byte CID, nested q/Q,
    overlapping/degenerate quads, all/none covered); **ZERO new
    dependencies**, `THIRD_PARTY_LICENSES.md` unchanged; GUI PID 40828.
    **MILESTONE: read → write → edit → extract → annotations → forms →
    redaction ALL shipped. In progress advances to decision 010** (post-
    redaction priority — KenAgent consultation IN FLIGHT: vector/Inkscape
    editing vs GUI-editing consolidation vs render-fidelity verification vs
    encryption). Full entry: `ROADMAP.md` Shipped Pass 8.0; the surgery/scrub
    design: §5.9; the mirror-image framing: §5.8.
- **2026-08-01 — Post-redaction priority decided (decision 010; the
  KenAgent consultation of continuation-28 RETURNED).** Full record:
  `docs/decisions/010-highest-value-investment-after-the-editing-arc.md`.
  Consulted + archived; scopes Pass 11 (dispatched) and the forward
  sequence.
  - **(a) DESTINATION UNCHANGED, PATH AMENDED — the framing.**
    Vector/Inkscape editing (decision 008's candidate **E** / **Pass 9**)
    remains the highest-value major investment AND pdfce's distinctive
    purpose — that destination does not move. What the accumulated
    GUI-editing + render-verification debt changes is the PATH to it. The
    build order becomes the three-Pass sequence **C → B → A**: **Pass 11**
    (render-fidelity verification) → **Pass 12** (canvas-interaction
    foundation + editing-GUI consolidation) → **Pass 9** (vector editing,
    repositioned onto C+B, keeping its decision-008 Pass ID). Decision
    010's candidate letters **A–E are LOCAL to that record and DIFFER from
    decision 008's A–F** — do not conflate (010-A = 008-E = vector; 010-B =
    GUI consolidation; 010-C = render-fidelity; 010-D = encryption; 010-E =
    signatures).
  - **(b) AMENDS decision 008's revisit-trigger-7.** Decision 008 named a
    clean jump straight to Pass 9 after Pass 6.1; decision 010 amends that
    trigger into the C→B→A sequence, because the render-verification and
    GUI-editing debt must be discharged before the vector-editing surface
    is built on top of it. **Decision 008's ranking and Pass IDs are
    otherwise intact.**
  - **(c) Pass 11 = render-fidelity verification (candidate C), DISPATCHED
    — no blocking spec, pure measurement.** Generalize
    `tools/annot-pdfium-diff.py` to full-page pdfium/pypdfium2 pixel-parity
    over the loadable corpus; a DOCUMENTED justified tolerance band
    reporting DISTRIBUTIONS (never a bare pass/fail); a three-bucket
    classification (benign-renderer-noise / known-disclosed-gap
    [cross-ref the Diagnostics unsupported-tally — Type3 // `sh` // SMask //
    OC // DeviceCMYK — SUBTRACT, do not re-report] / unexplained),
    enumerated by file + reason (R20); triage+fix cheap bucket-(iii) pdfce
    bugs, file the rest as counted named render-gaps; encode the known
    pdfium reference-divergences (`FPDF_FFLDraw` widgets + pdfium's
    synthesized no-`/AP` appearances that R43 makes pdfce refuse); WIRE
    into the standing gate set (re-run on every render-touching Pass);
    DeviceCMYK colorimetry characterized corpus-wide, fixed only if bounded
    (else the first named residual — re-pin decision 006 §3.4's polarity
    matrix before any colour change). **DISCHARGES the long-owed full-page
    pixel-parity remainder (Pass 1.1)** — conditionally, only if the
    harness genuinely generalizes to full-page corpus scale (not a
    "pixel-perfect" claim).
  - **(d) Pass 12 = one canvas-interaction substrate (candidate B).** The
    three accumulated named GUI follow-up slices (Pass-6.1 markup-drawing
    state machine; Pass-7 form-fill GUI; Pass-8 redaction-marking GUI) are
    RECONCILED as SLICES on ONE shared substrate — focusable canvas +
    screen↔page transform + tool-mode dispatch + hit-test/selection +
    live-preview overlay, built once, resolving `main.rs`'s Pass-1
    focusable-canvas caveat — NOT three independent buckets. Pass 9 vector
    editing later layers on the same substrate.
  - **(e) THREE new standing rules added to `ROADMAP.md` (R59/R60/R61):**
    **R59** render-fidelity gate (prove against an independent renderer at
    corpus scale before any subsystem edits content it re-renders;
    self-comparison proves agreement-with-self, not correctness; re-run on
    every render-touching Pass; residual enumerated by file+reason, never a
    threshold tuned to pass — W14); **R60** one-canvas-interaction-substrate
    (exactly one focusable-canvas/transform/tool-mode/hit-test/selection/
    overlay; markup/form-fill/redaction/vector all layer on it; a second
    parallel path forbidden — R49 applied to interaction); **R61**
    Inkscape-is-behavioral-reference-only (GPL-2.0-or-later, never a
    dependency/code-source/GUI-mimicry; `pdfce-inkscape-librarian` catalogs
    capability/behavior/limits, `pdfce-ui-specialist` designs the UI
    independently — formalizes the prior binding ROADMAP note).
  - **(f) `pdfce-inkscape-librarian` + `Inkscape_Features` RAG
    COMMISSIONED 2026-08-01** (in parallel, another agent creating the
    agent file + scaffold at `D:\Dev\Rag-Specialized\Inkscape_Features\`) —
    closes decision 008 §11.4's previously-unowned Inkscape-catalog item,
    so the capability catalog exists before Pass 9 is scoped. Registered in
    the project agent roster (`CLAUDE.md`'s "Project agents" table). It is a
    private development-reference corpus (same posture as the Acrobat
    Features RAG) — never shipped, never committed to the pdfce repo.
  - **(g) Encryption (Pass 5) = candidate D**, stays fallback/interleave
    (unchanged by decision 010, retains its decision-007 ID);
    **signatures (Pass 10) = candidate E, unchanged-last.** Full entry:
    `ROADMAP.md` In progress (Pass 11) + Next up (Pass 12 → Pass 9);
    standing rules R59–R61.

- **2026-08-01 — Pass 11 SHIPPED (render-fidelity verification harness) +
  operator reprioritization to a measurement/dimensioning beta (decision
  011 in flight).** Pass 11 (decision 010's candidate C) shipped as PURE
  MEASUREMENT — zero Rust touched, zero new pdfce dependency (pypdfium2
  dev-tooling only, out-of-tree, not vendored, absent from
  `THIRD_PARTY_LICENSES.md`). Full record: `ROADMAP.md` Shipped (Pass 11).
  - **(a) The harness.** `tools/render-parity/` (out-of-tree, mirroring
    `tools/content-identity/`) drives `pdfce-cli render-page` + pypdfium2,
    aligns rasters, computes per-channel per-pixel deltas over the full
    loadable corpus (2,914 files → 2,890 pages at 125 DPI, content-only;
    ZERO panics/timeouts; 24 skips = unloadable `fail-*` files). Replaces
    the self-comparison round-trip oracle (which proves pdfce agrees with
    *itself*, not that it matches an independent renderer) with a measured,
    bucketed, by-file/by-reason fidelity report — the correctness oracle
    Pass 9 vector editing newly requires (first subsystem whose acceptance
    test is independent *visual* fidelity).
  - **(b) The area-fraction tolerance band (the analytical core).** Metric
    `frac_over_32` = fraction of pixels with max-channel |delta| > 32/255.
    Benign AA/hinting/sub-pixel noise is confined to a thin edge band
    (small AREA) even where edge pixels swing full-range, so the
    noise-robust discriminator is AREA-fraction, not max per-pixel delta.
    Band = p99.9 of `frac_over_32` over the 1,728 clean-by-construction
    pages (zero disclosed gaps + no DeviceCMYK) = a property of the
    known-benign population, so it CANNOT be tuned to make a bug pass (W14
    structurally satisfied). This run: band 0.0294; clean-floor mean
    0.00096 / p95 0.0022 / p99 0.0098. The report prints the distribution,
    never a bare pass/fail.
  - **(c) Three buckets.** (i) benign-renderer-noise 2,840; (ii)
    known-disclosed-gap 49 (cross-referenced against the EXISTING
    Diagnostics tally so already-counted gaps are SUBTRACTED, not
    re-reported); (iii) **unexplained-divergence 1** = `A019-pdfa2-pass-a.pdf`
    (a form-XObject triangle vertex at x ~= `f32::MAX` under the CTM ->
    pdfce rasterizes a spurious cyan bar, pdfium clips it). Filed as a
    named counted render-gap (R20/R27), NOT fixed — the clamp/reject-policy
    call is a `pdfce-render` R34-risk decision, Pass-9-adjacent; the
    measurement-only non-goal binds.
  - **(d) DeviceCMYK = FIRST named residual (NOT fixed).** DeviceCMYK-only
    pages diverge 3.0x the clean-page mean; the delta lights the whole
    filled area uniformly with polarity IDENTICAL (§3.4 / R29 holds) — the
    naive additive `Rgb::from_cmyk` vs pdfium `AdobeCMYK_to_sRGB1` gap.
    Filed as a follow-up colour Pass; decision 006 §3.4 polarity matrix
    must be re-pinned BEFORE any colour change (006 revisit-trigger 7;
    don't confound colour with the harness build). Both residuals filed as
    Backlog items.
  - **(e) R59 discharged for the first time.** `--gate --max-unexplained
    <baseline>` returns non-zero when the unexplained count rises; baseline
    = 1 (the A019 file), verified PASS. Documented as a REQUIRED re-run on
    every render-touching Pass (the R34/R46 pattern), especially Pass 9.
    Local-corpus gate (pypdfium2 not in CI, like content-identity /
    roundtrip). Reference-side pdfium quirks (`--annots` mode:
    `FPDF_FFLDraw` widgets + synthesized no-`/AP` looks R43 refuses) are
    bucketed reference-side (Y2), never charged against pdfce.
  - **(f) Pass 1.1 pixel-parity remainder DISCHARGED.** The harness
    genuinely generalizes to full-page corpus scale (per-channel per-pixel;
    full loadable corpus; first-page coverage of every file; multi-page via
    `--pages-per-file 0`, demonstrated) — meeting decision 010's exact bar.
    Scope named precisely (first-page corpus coverage + a multi-page knob),
    NOT overclaimed as exhaustive-multi-page or pixel-perfect. Struck from
    the SESSION_LOG "still open" lists going forward.
  - **(g) Reprioritization — measurement/dimensioning beta (decision 011,
    IN FLIGHT).** Operator requested a beta (scaled dimensions + vector
    selection/snapping + basic vector editing) as his first usable
    deliverable; its architecture is being decided via KenAgent as decision
    011. The beta PULLS FORWARD decision 010's Pass 12 (candidate B) + the
    first slices of Pass 9 (candidate A) and adds a new dimensioning
    subsystem — the mechanism is decision 010 revisit-trigger 3 (operator
    wants vector editing sooner, now on a *corpus-measured* render rather
    than merely spot-checked). Decision 010's C -> B -> A sequence CONTINUES
    after the beta; Pass 11 (C) is now shipped so the render is verified for
    the editing work. The beta's Pass IDs/slices are defined by decision
    011, not here.
  - **Gates:** `cargo fmt --check` clean; `cargo tree` core+render GUI-free;
    ZERO Rust delta -> clippy/test/R34/R46 unmoved by construction; no
    `Cargo.toml` change -> `THIRD_PARTY_LICENSES.md` unchanged;
    deterministic/locale-invariant (sorted files, fixed DPI, no clocks).
    RAG escalations: `C:\personal_rag\pdf\` (area-fraction-not-max-delta
    tolerance-band methodology); `D:\dev\rag\rust\` (`nohup`-detach
    background-sweep gotcha).
- **2026-08-01 (GUI-polish interlude + launcher)** — An operator-requested
  GUI polish + launcher interlude shipped (see `ROADMAP.md` Shipped,
  `SESSION_LOG.md` continuation 31). NOT a feature Pass — `pdfce-gui` +
  `ui_text.rs` only, ZERO new deps. Two items with lasting architectural
  weight: **(a) canonical run entrypoint** — `D:\Dev\pdfce\pdfce.bat` +
  `pdfce.ps1` at the repo root are now the double-clickable / drag-a-PDF /
  `pdfce.bat [file]` launchers (each `cd`s to repo root, `cargo build
  --release -p pdfce-gui` as a freshness check, then `Start-Process` the exe
  detached). **(b) A named data-safety relationship, now formally tracked as
  a standing-UX-rule gap (Backlog):** true **in-place Save** stays
  deliberately GATED on an **autosave / crash-recovery scratch file**
  existing; until that lands, "Save a copy" is the only save affordance —
  the conservative, non-destructive-by-default posture (§5's spirit applied
  at the GUI layer: never overwrite the source without a recovery net). This
  is NOT cosmetic polish; it is an open crash-safety obligation surfaced,
  named, and filed rather than silently deferred.
- **2026-07-31 — Root-cause font fix (NUL-misroute) + operator-supplied
  fonts (decision 012) SHIPPED.** Two coupled font-layer changes. Full
  records: `ROADMAP.md` Shipped (Font-fix; Operator-supplied fonts);
  `docs/decisions/012-operator-supplied-fonts.md`.
  - **(a) The root-cause fix.** A subset CIDFontType2 (embedded TrueType, no
    `cmap`, legal per §9.7.4.2) was misrouted to the CFF parser because
    font-program format detection trimmed leading whitespace **including
    NUL** before magic-sniffing — stripping the leading NUL of the sfnt
    magic `0x00010000` so `01 00 …` matched bare-CFF magic. Fix: match binary
    magics on RAW bytes; trim only on the Type 1 `%!` text path; never NUL.
    Class impact: all embedded TrueType from SolidWorks/AutoCAD/Office CAD.
    **skrifa stays 0.42.1 pinned — the bug was pdfce-side routing, no bump.**
    Verified corpus-wide by the R59 render-parity gate: font-unsupported gap
    **7→0**, unexplained **1→1** (no regression), band re-derived
    0.02942→0.02963. New `Diagnostics::fonts_unsupported_by_reason`
    (Type3/NonIdentityCmap/VerticalWriting/CompositeNotEmbedded/
    UnknownSubtype/UnusableProgram). This graduates the R68 standing rule
    (embedded font programs route to the correct parser or fail clean; a
    magic/variant disagreement is a gate failure) + the `tools/font-parity/`
    harness that guards it.
  - **(b) Operator-supplied fonts (decision 012 first cut).** Non-embedded,
    non-Base-14 SIMPLE fonts render from an operator-supplied folder, riding
    the `FontEnvironment.named` seam decision 004 §5.3 built for exactly
    this. `LoadedFont.substituted: bool` → `GlyphSource {Embedded, Bundled,
    Supplied}` (three trust levels, R63); `Diagnostics.glyphs_supplied` /
    `supplied_fonts` distinct from the bundled counters; `substitute_face`
    retries after `strip_subset_tag`; `face_names()` on the one skrifa parser
    (R21). The **shell** (`pdfce-gui`/`pdfce-cli`) owns the `std::fs` folder
    walk and the setting; `pdfce-render` stays **bytes-in** (R62) so R10
    (platform-clean core/render), R11 (wasm32), and R19 (deterministic-by-
    default) all hold; `pdfce-core` untouched. Positions still come from
    `/Widths` — supplied improves *shapes*, not *positions* (R63). Adds
    standing rules **R62–R66** (renumbered from the record's proposed R61–R65;
    R61 was taken by decision-010's Inkscape rule). Named fast-follows: FF1
    OS-font enumeration (R66 opt-in), FF2 composite/CID via the Unicode route
    (R65), FF3 descriptor auto-routing. Composite non-embedded stays a hard
    skip (`CompositeNotEmbedded`). ZERO new deps. **Recorded connection:**
    decision 012 is the enabler for the ★ NEXT MAJOR FOCUS Acrobat
    text-editing subsystem — a typed/edited glyph run needs the font
    available to draw it.
  - **OWED code follow-up:** the operator-supplied-fonts `pdfce-render` doc
    comments cite the record's proposed R61/R62/R63; they must be updated to
    the assigned R62/R63/R64 (recorded in ROADMAP Standing rules + SESSION_LOG).
- **2026-07-31 — Cross-reference recovery decided (decision 013); Pass 13a
  SHIPPED (negative result), Pass 13b IN PROGRESS.** The #1 real-world
  robustness fix: 605/712 (85%) of real-file load failures are a missing
  rebuild-by-scan xref-recovery path. Full record:
  `docs/decisions/013-xref-recovery.md`; `ROADMAP.md` Shipped (Pass 13a) +
  In progress (Pass 13b).
  - **(a) Headline finding (Pass 13a, negative result).** pdfce's classic
    xref-table parser is already CRLF-correct for all three §7.5.4 EOL forms
    (SP CR / SP LF / CR LF); the strong CRLF failure correlation is
    **offset-shift corruption** (LF→CRLF text-mode conversion invalidating
    every stored byte offset incl. `startxref`), **NOT a parser bug**. 9
    synthetic legal-EOL fixtures all parse; 547/567 sampled real failures are
    offset-shift; 0 genuine parser bugs. So rebuild-by-scan (Pass 13b) carries
    essentially all the recovery, and Pass 13a is a cheap disambiguation. No
    parser code changed (tests + fixtures + tools only).
  - **(b) Pass 13b design (in progress).** A `pdfce-core` recovery module
    firing ONLY on the strict-load error path (clean files untouched by
    construction); two-phase scan (file-level `N G obj` last-wins, then ObjStm
    pair-table); trailer from last `trailer` or synthesized from `/Type
    /Catalog`; re-checks `/Encrypt` after rebuild and still refuses; subsumes
    the offset-start header case (decision 007 §10 item 6) for free. Non-
    normative reader-robustness policy (no ISO clause defines a recovery
    algorithm) grounded in universal reader behavior; bounded (R25), fail-
    clean (R27), disclosed + counted (R20). NO new dependency (reuses
    `parser.rs` + existing filters/objstm).
  - **(c) The §5 interaction → R67.** A recovered document forces a full-
    rewrite save (`save_incremental` refused by name); §5.10 records the full
    contract, marked pending Pass-13b ship. Standing rule **R67** (renumbered
    from the record's proposed R59, which was taken by decision-010's render-
    fidelity gate) — the third sibling of R35 (redaction) and R58
    (removal/scrub) in the forced-full-rewrite family.
- **2026-07-31 — Acrobat-style in-place text editing decided (decision
  014); Pass 13.x renumbered to Pass 14.x.** The operator's ★ NEXT MAJOR
  FOCUS directive (Backlog, filed 2026-08-01) is now a full architecture
  decision, archived `docs/decisions/014-acrobat-text-editing.md`. Full
  record: `ROADMAP.md` "Next up" (Pass 14.x) + Standing rules (R69–R74);
  `ARCHITECTURE.md` §5.11 (new).
  - **Renumber, recorded explicitly.** The record proposes "Pass 13.x
    (13.0–13.3)"; 13.x was already assigned to xref recovery (decision 013,
    Pass 13a + 13b) by the time this decision was filed. Librarian assigned
    the next free MAJOR number, **Pass 14.x** (14.0 read-only model + block
    recognition; 14.1 in-place edit + single-line relayout + font-on-edit
    gate + CLI `edit-text`; 14.2 formatting on selection; 14.3 edit UI on
    the Pass 12.0 canvas).
  - **Model — M-hier (Run→Line→Block), derived and reviewable.** A NEW
    `pdfce-core` module clusters Pass 4's positioned-glyph extraction into a
    hierarchy (baseline-Y lines, x-band columns, indent/leading
    paragraphs), reusing `layout.rs`'s three ratios. Every Run/Glyph gains
    provenance (source show-operator identity, byte span, full text-state)
    — the substrate the surgery needs. Everything is DERIVED (§14.8
    S1-S9), counted, and reviewable (rule 4) — never silently
    authoritative.
  - **Edit mechanism — E-surgery, not overlay.** Extends Pass 8.0's
    advance-preserving REMOVE interpreter to REPLACE: locate the show
    operator(s), re-encode via an inverse of the §9.10.2 decode ladder,
    re-emit, preserve the §9.4.4 advance. Only edited content stream(s) (+
    changed resource/font dict) are re-emitted; R47's surgery-vs-overlay
    line gets its second sanctioned member (redaction was the first).
  - **Font-on-edit — F-refuse primary; F-substitute only as an explicit
    disclosed choice; F-embed (subsetting) DEFERRED as FF-C.** A keystroke
    applies only when the run's font can already provide the glyph:
    embedded-full (free edit), embedded-subset (edit within existing
    glyphs, refuse-and-disclose a missing one), non-embedded named simple
    (edit bounded by bundled/supplied coverage — decision 012's
    `--font-dir` is pdfce's "local font," reused verbatim), non-embedded
    composite/CID (deferred, FF-E). Ships real editing for three
    high-coverage cases WITHOUT a font subsetter; names the one refusal
    case precisely instead of faking a glyph.
  - **Relayout — RL-line first cut; reflow (FF-A/FF-B) is the
    exceed-Acrobat play.** Acrobat's offline reflow is within-block only
    and its cross-block reflow is cloud-gated + English-only; pdfce's
    offline cross-block reflow (FF-B) is a genuine capability lead, not
    parity. First cut ships single-line advance-preserving relayout only
    (line may overflow the margin, disclosed).
  - **Save mode — default INCREMENTAL (R36), explicitly NOT a fourth
    forced-full-rewrite sibling.** See `ARCHITECTURE.md` §5.11 (new) — this
    is the key structural distinction from R35/R58/R67. Truly removing
    text stays Redaction's job.
  - **Tagged PDFs — T-disclose, the Acrobat-beating property.** Preserves
    BDC/EMC + MCID wrappers around edited operators (structure-tree
    references stay valid) and discloses `/ActualText`/reading-order
    staleness, rather than corrupting the tree the way Acrobat's own
    in-place edit is known to.
  - **Standing rules R69–R74 filed** (decision 014 §5.1's six proposed
    rules, in order, no collisions against R68): text-edit-is-surgery-not-
    overlay; text-edit-is-incremental-not-a-scrub; font-on-edit-trust-
    ladder; recognized-blocks-and-reflow-are-reviewable-hints;
    tagged-edits-disclose-never-corrupt; text-model-in-core-edit-UI-in-gui.
  - **Zero new dependency for 14.0–14.2** (reuses Pass 4, Pass 8.0,
    `vartext.rs`, decision 012's `GlyphSource`, the one skrifa parser
    R21). Only FF-C (font subsetting) would add a crate, gated
    permissive-only (rule 13) with its own dependency-licensing
    escalation, flagged early.
  - **Timing.** All four gating items the operator's directive named
    (font-supply/decision 012, Pass 12.0 canvas, xref-recovery/decision
    013, the beta's Pass-12.0 foundation) are now SHIPPED — see the
    following entry. Starting Pass 14.0 is an engineering scheduling call,
    not a blocked prerequisite.
- **2026-08-01 — Pass 13b (rebuild-by-scan xref recovery) SHIPPED;
  decision 013 CLOSED.** The #1 real-world robustness fix lands: 566
  previously-strict-failing real-world files now open (1,109-file corpus:
  qpdf 639 / pdfium 331 / PDFBox 139), reason-bucketed
  (`NotAnXrefSection` 417 / `TrailerParse` 99 / `BadEntry` 20 /
  `BadXrefStream` 13 / `StartxrefNotFound` 7 / `BadStartxrefOffset` 7 /
  `MissingHeader` 3); **zero regression** on the 2,907-file veraPDF corpus
  (0 clean files diverted into recovery, verified by object-outcome
  tally); the `*-fail-*` reconciliation gate is COMPLETE (all 5 veraPDF
  status changes are PDF/A-conformance files failing a header/colour-space
  rule, never an xref-parse bug — defensible reader recovery, qpdf/pdfium
  agree). 53 real-world files with object-level corruption after a clean
  xref recovery are a named non-goal (new Backlog item, `ROADMAP.md`).
  Fuzz 21,595 runs / 0 crashes; ZERO new dependency; full record:
  `ROADMAP.md` Shipped (Pass 13b). **`ARCHITECTURE.md` §5.10 FLIPPED from
  "pending Pass-13b ship" to shipped/active** (see §5.10 above) — R67 is
  now IN FORCE, not merely filed. Two engineer-flagged deviations recorded
  at the Shipped entry: a code-comment number lag (R59→R67 in
  `recover.rs`, being discharged this session) and a deliberate,
  defensible `gen-65536` deviation (rebuild-by-scan opens some gen-65536
  files via the `BadEntry` trigger — a decision-013 target bucket, NOT the
  separate strict-parser gen-65536 tolerance question Pass 13a flagged,
  which remains open and unaffected).
- **2026-08-01 — FF-A within-block offline reflow decided (decision 015);
  AMENDS decision 014; `ARCHITECTURE.md` §5.11 FLIPPED to shipped.** Full
  record: `docs/decisions/015-ffa-within-block-offline-reflow.md`;
  `ROADMAP.md` "Next up" (★ Pass 15.x) + Standing rules (R75–R77);
  `docs/decisions/014-acrobat-text-editing.md` (amended §3/§5.3/§6, see its
  dated footnotes).
  - **Trigger.** Decision 014's Pass 14.0–14.3 in-place-editing family
    shipped complete (2026-08-01); FF-A (the reflow ladder's first rung)
    is the active thread, and `pdfce-acrobat-librarian`'s scoping surfaced
    one genuinely open question — see next bullet.
  - **The settled call: justified alignment relocates FF-B → FF-A.**
    Acrobat exposes **Justify** on its BASE (non-cloud) Edit-Text panel —
    proof it is a classic-engine, single-block capability, not a
    cross-block/cloud one. Justified is therefore a within-block alignment
    mode, a peer of left/center/right (all three already FF-A), not a
    reflow *scope*; shipping 3-of-4 alignment modes in FF-A while gating
    the fourth behind an unrelated cross-block engine would be incoherent.
    FF-B's headline narrows to cross-block + cross-page reflow only — the
    genuine exceed-Acrobat axis (Acrobat's cross-block reflow is
    cloud-gated + English-only).
  - **Line-breaking — greedy/first-fit, `vartext.rs`'s packing core
    factored (not reused as-is).** Acrobat publishes no line-breaking
    algorithm, so greedy is a free, honest, low-cost choice (Knuth-Plass
    deferred, named non-goal). The greedy packing core is factored into a
    shared breaker taking a width-measuring closure: `vartext.rs` keeps its
    Std14-AFM-width Std14 path; FF-A supplies a provenance-§9.4.4-advance
    measurer over the block model's runs. Break opportunities are
    whitespace-only — no hyphenation, no CJK breaking (FF-E).
  - **Trigger + scope — explicit operator action, exactly one recognized
    `Block`.** Reflow is derived layout the file never stated (§14.8
    S1-S9), so rule 4 requires an accept/reject step; it never fires
    automatically on edit. It coexists with, and never supersedes, Pass
    14.1's single-line relayout (the default post-edit behavior). Reflow
    never crosses into a sibling block or column band; wrap width is the
    block's own detected `bbox` width, operator-adjustable.
  - **One derived preview, one undo-able command.** A `ReflowPreview`
    (new break points, per-line `Tm`/`TD` origins, alignment, new `bbox`,
    disclosures) is an accept/reject overlay; the operator adjusts
    width/alignment/leading, re-previewing live; on accept, 14.1's
    advance-preserving surgery re-emits the block's show operators and the
    whole thing lands as ONE `CommandKind::ReflowBlock` on `EditSession`
    (undo/redo atomic, sibling to `EditText`/`FormatText`). Reject mutates
    nothing.
  - **Page overflow — disclose-and-allow, never silent-disappear, never
    hard-refuse.** A block grows top-anchored downward as lines are
    added/removed; content pushed past the page cropbox is disclosed
    ("reflow grows the block N pt past the page bottom; M line(s) fall
    outside the visible page") and, on accept, emitted as real, recoverable
    off-page content — never clipped-to-invisible, never dropped. This is
    a deliberate divergence from Acrobat, whose own documentation says
    overflow "disappears" — reproducing silent loss is exactly what rule 4
    forbids; a hard refuse would lose legitimate operator work.
  - **Alignment auto-detect + preserve — the differentiator.** A block's
    left/center/right/justified alignment is inferred from glyph
    x-positions (reusing the Pass 14.0 x-band/column geometry) and
    preserved by default through re-wrap; every inference is counted
    (`BlockDiagnostics`) and operator-overridable; a single-line block
    defaults to left + disclosed ambiguity. Acrobat has no documented
    auto-detect/preserve — a re-wrap there risks a silent left-align. This
    is a named, evidenced exceed-Acrobat property, not incidental.
  - **Minimal-diff confirmed.** Reflow re-emits only the reflowed block's
    own content-stream object via the 14.1 surgery machinery; unchanged
    lines byte-identical where provable; default save stays INCREMENTAL
    (R34/R36) — not a forced full rewrite (redaction's R35 alone keeps
    that). Tagged-block MCID/BDC/EMC wrapper preserved, staleness
    disclosed (R72), exactly as 14.1 does.
  - **Pass numbering — assigned fresh Pass 15.x (librarian's call, per
    decision 015 §6's explicit delegation).** Rather than folding into
    14.4–14.6, keeps "Pass 14.x = in-place editing" and "Pass 15.x =
    reflow" as two coherent, separately-citable families — the same
    precedent as 14.x itself being assigned fresh once 13a/13b had already
    taken 13.x. 15.0 (engine, read-only) is DISPATCHED TO BUILD NOW; 15.1
    (surgery + `ReflowBlock` + CLI) and 15.2 (canvas UI,
    `pdfce-ui-specialist` first) follow.
  - **Standing rules R75–R77 filed** (decision 015 §5, in order, no
    collisions against R74): reflow-is-explicit-reviewable-single-block-
    one-undo-command; reflow-overflow-discloses-never-disappears;
    alignment-auto-detected-and-preserved-through-rewrap. Kept as three
    rules (not folded per the decision's discretion note) — matches the
    granularity of decision 014's six rules for the same family.
  - **`ARCHITECTURE.md` §5.11 FLIPPED** from "pending Pass 14.0–14.3 ship"
    to the shipped module layout (see §5.11 above), since all four Pass
    14.x slices are complete; §5.11 also gained a forward pointer to this
    Pass 15.x reflow family.
  - **Zero new dependency.** Reuses Pass 14.0's model/geometry, Pass 14.1's
    surgery, `vartext.rs`'s packing core (factored), decision 012's
    `GlyphSource`. `pdfce-spec-librarian` is dispatched for §9.4.3 `TJ` /
    §9.3.3 `Tw` ahead of 15.1.
- **2026-08-01 — License = MIT (operator decision).** `docs/LEGAL.md`
  §1 flipped from "OPEN DECISION" to **DECIDED: MIT**, as part of a
  combined operator instruction that also set the project's next work
  focus (dimensioning tool → GUI-complete; ScripTree-style icons for
  all GUI features; finish text-handling fast-follows; form-building
  tools after — see `ROADMAP.md` "In progress"/"Next up" for the full
  sequencing record). **Rationale:** MIT is maximally permissive
  (easiest third-party adoption/embedding) and is fully compatible with
  the existing dependency set — a per-dependency audit against
  `THIRD_PARTY_LICENSES.md` found every current dependency permissive
  (MIT/Apache-2.0/BSD/ISC/Zlib/Unicode), zero copyleft, so the decision
  requires no dependency rework. Implemented same-session: `LICENSE`
  file at repo root, `license = "MIT"` in `Cargo.toml`
  `[workspace.package]`, `license.workspace = true` on all four member
  crates (`cargo metadata`-confirmed). **Consequence (§9 below,
  amended):** GPL/AGPL prior art — MuPDF, Poppler, Ghostscript (see
  `docs/PRIOR_ART.md`) — is now categorically and permanently excluded
  as a real dependency; it was already reference-only in practice, but
  the license decision now forecloses the alternative (an AGPL pdfce
  unlocking them) for good. Project rule 8's license precondition for a
  public-facing commit posture is now satisfied, but pushing the
  existing local commit (`d8b3903`) or publishing a release still
  requires its own, separate operator go-ahead — not implied by this
  decision. Full record: `docs/LEGAL.md` §1/§6.1/§7.
