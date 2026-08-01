# Decision 013 — Cross-reference recovery: rebuild-by-scan + a CRLF xref-table audit

- **Date:** 2026-07-31
- **Status:** Decided (consultation — the engineer implements from the JSON block)
- **Decider:** KenAgent (autonomous-builder), per the ROADMAP KenAgent-decision-routing rule
- **Question:** How should pdfce recover from a broken/mislocated cross-reference
  table so it can OPEN the large real-world class that a strict xref parse
  rejects — a rebuild-by-scan fallback plus a CRLF xref-table audit — WITHOUT
  compromising the round-trip/minimal-diff invariant (ARCHITECTURE §5) for files
  that already load cleanly?
- **Outcome:** Two sequenced Passes. Pass A: an EOL/CRLF classic-table audit
  (measurement + conditional strict-correctness fix). Pass B: a rebuild-by-scan
  recovery path that triggers only on cross-reference load failure, is flagged,
  disclosed, counted, and forces a full-rewrite save.
- **Adds standing rule:** R59 (recovered base forces full rewrite + disclosure;
  librarian confirms the number).
- **Supersedes:** nothing. **Relates to:** decision 007 (writer + §5), R35, R58,
  R20, R25, R27, R33, and §5.6.

---

## 1. Context — the quantified gap

A newly-acquired 1,109-file real-world corpus (pdfium/qpdf/PDFBox provenance,
permissive) was swept with pdfce. **605 of 712 load failures (85%) are one
missing capability:** pdfce hard-fails when the cross-reference table cannot be
parsed at the location `startxref` names, where every mature reader
(pdfium, qpdf, poppler, mupdf, pdf.js) recovers by brute-force scanning for
`N G obj` object headers and rebuilding the xref + trailer. pdfce has no such
path.

The failure breakdown:

| Count | Error | Meaning |
|---|---|---|
| 423 | `NotAnXrefSection` | `startxref` offset does not land on `xref` / an xref-stream object |
| 94 | malformed-indirect-header at xref | `startxref` lands mid-object; stream-classification fails |
| 22 | no `%PDF-` header | header probe fails (incl. leading-bytes / offset-start) |
| 20 | `BadEntry` | a 20-byte record deviates |
| 13 | `StartxrefNotFound` | no `startxref` in the trailing scan window |
| 7 | `BadStartxrefOffset` | `startxref` value out of range |

**Strong correlation with CRLF line endings:** 237/241 of the top bucket are
CRLF files, versus clean-loading files at 236/251 LF. The brief hypothesised
this could be BOTH a missing recovery path AND a real bug in the classic
xref-table parser's CRLF handling.

---

## 2. The headline finding — the CRLF correlation is offset-shift, not a parser bug

Reading `crates/pdfce-core/src/xref.rs` closely (`parse_section_at`,
`skip_one_eol`, `parse_entry`), pdfce's classic-table parser is **already
CRLF-correct** for all three §7.5.4 EOL forms:

- `parse_entry` accepts exactly `SP CR`, `SP LF`, and `CR LF` as the 2-byte EOL
  of a 20-byte record (`matches!(eol, b" \r" | b" \n" | b"\r\n")`).
- Entries are read as **exact 20-byte** records at absolute offsets.
- `entry_pos` is recomputed via `skip_one_eol` — which correctly consumes CR,
  LF, or CRLF — at the **start of every subsection**, so no per-line drift can
  accumulate across a CRLF table.
- `find_startxref` lexes the offset token, and the lexer skips CRLF like any
  whitespace.

So a *well-formed* CRLF table parses. The most plausible explanation for the
CRLF correlation is therefore an **offset-shift artifact**: a file authored with
LF, then converted to CRLF in text-mode transport (the classic FTP/email
corruption), gains one byte per line. Every byte offset computed for the LF
version — the `startxref` **value** AND every in-table entry offset — is now
wrong by the number of preceding lines.

The brief's own canonical example confirms this exactly:
`qpdf/add-contents.pdf` is a **valid qpdf-authored** file storing
`startxref 685`, but byte 685 lands inside `...endobj\r\n8 0 obj` — the real
`xref` is at byte 724, a **39-byte forward shift**. qpdf recovers by scanning;
pdfce dies at `NotAnXrefSection`.

**This is not a parser bug. It is a file whose stored offsets cannot be trusted
at all** — which a parser tweak cannot fix and rebuild-by-scan solves head-on
(the scan rebuilds every offset from physical object positions and ignores the
stored ones entirely).

**Consequence for slicing:** the CRLF audit (Pass A) is expected to be a cheap
**disambiguation**, quite possibly a documented negative result — not the thing
that recovers the 605. Do not bank the 605 on Pass A. This applies the project's
measurement-first discipline (decisions 005/006) to the one hypothesis that
would otherwise send the engineer hunting a phantom bug.

> Honesty caveat: this conclusion is from reading the code, not from running the
> 241 CRLF files. An edge case (a bare-CR-only file, a subsection header with an
> unusual terminator, mixed EOLs) could still surface a genuine rejection. That
> is precisely why Pass A **runs** the fixtures rather than asserting the result.

---

## 3. Decision

Ship xref recovery as **two sequenced Passes, A before B.**

### 3.1 Why two Passes, and why A first

- The two levels have **opposite risk profiles.** Pass A edits the hot
  clean-load path (tiny change, high blast radius). Pass B is pure error-path
  new code (large change, near-zero blast radius on clean files). Fusing them
  would hide a strict-correctness fix inside a large feature and muddle the
  regression story.
- **B's acceptance classification depends on A.** You cannot correctly label a
  recovered file as *"valid that pdfce wrongly rejected"* versus *"genuinely
  broken, needed rebuild"* until the parser is **confirmed correct on valid
  CRLF input**. A produces that confirmation.
- If A finds a bug, it ships on its own merits with its own regression test. If
  A finds nothing, that negative result is the deliverable that justifies
  putting all effort into B.

### 3.2 Pass A — Classic xref-table EOL/CRLF audit + conditional fix

**Goal:** determine empirically whether pdfce rejects any *well-formed*
CRLF-authored classic table (a strict §7.5.4 bug), separate from genuinely-broken
xref. Fix if found; document either way.

**Method:**
1. Build synthetic fixtures (`fixtures/synthetic`): valid classic tables using
   each legal EOL (SP CR, SP LF, CR LF) on both the entry lines and the
   `xref`/subsection-header/`trailer`/`startxref` lines; multi-subsection tables;
   trailing-space-before-EOL; a bare-CR-only (old-Mac) file; a mixed-EOL file.
   Assert every well-formed one loads.
2. Take a bounded sample (~25–40) of the 241 CRLF failures and hand-classify each
   root cause: **(a)** stored `startxref` lands off the real xref (offset-shift —
   Pass B territory) versus **(b)** `startxref` is correct and the parser still
   rejects a structurally-valid table (a real Pass-A bug).
3. Only if class (b) appears: fix the parser, add a regression fixture per
   distinct bug shape, verify the fix reopens those files with **zero change** to
   any currently-Ok file.

**Acceptance:** every synthetic well-formed table loads; the 2,890 clean
conformance files show zero change in load outcome and object counts; the
sampled-failure classification is recorded with counts;
`cargo fmt`/`clippy -D warnings` clean.

**Expected outcome:** most likely a **negative result** (no parser bug; the
correlation is offset-shift). Framed as cheap disambiguation.

**Risk:** this Pass touches the clean-load path. A careless "tolerance" change
could accept a malformed table (a false green — decision 007 W10). Keep strict
rejection of genuinely-malformed entries; broaden only to spec-permitted forms
proven by a valid fixture; assert exact entry length in a unit test, never rely
on round-trip reload to catch a bad entry.

### 3.3 Pass B — Rebuild-by-scan cross-reference recovery

**Goal:** open the large real-world class by reconstructing the xref + trailer
from a full-file object scan when the stored cross-reference machinery cannot be
parsed. Reader-robustness parity with pdfium/qpdf/poppler/mupdf/pdf.js.

#### 3.3.1 Trigger rule (load-bearing)

Recovery is a **fallback, never the default**. It runs **only after the strict
path has failed**. A file that returns `Ok(LoadedXref)` from `load_xref_chain`
takes the normal path unchanged — **the round-trip/minimal-diff invariant for
clean files is preserved by construction, not by policy**, because recovery code
never appears in a clean file's control flow.

Wire recovery into `document.rs::from_bytes` at the two current failure points:
`xref::load_xref_chain(&buf)` returning `Err(XrefError)`, and `probe_header`
returning `Err`.

**Triggers on** these `XrefErrorKind`s: `StartxrefNotFound`,
`BadStartxrefOffset`, `NotAnXrefSection`, `BadEntry`, `BadSubsectionHeader`,
`BadTrailer(_)`, `PrevChainCycle`, `TooManyEntries` (bounded re-attempt under the
scan cap), and `BadXrefStream(_)`/`XrefStreamDecode(_)` (the stream is
unrecoverable in place — scan the file-level objects instead).

**Must NOT trigger on:**
- `EncryptionUnsupported` — a deliberate **named capability gap**, not damage.
  Scanning would still surface encrypted strings/streams. Recovery **re-checks
  for `/Encrypt`** after rebuilding the trailer and refuses with the same error,
  so a broken-xref-AND-encrypted file fails clean for the right reason.
- Object-level failures **after** a clean xref (`BadObject`, `ObjectIdMismatch`,
  `ObjectStream*`). Those mean the xref parsed but the body is corrupt —
  object-level lenient loading is a separate future feature. **Documented
  limitation:** a file whose xref parses structurally but whose offsets are
  uniformly wrong (offset-shift that happened to spare `startxref`) hits
  `ObjectIdMismatch` and is not recovered this Pass. But the dominant
  offset-shift case *also* breaks `startxref` (→ `NotAnXrefSection` → recovery
  fires), so this covers the bulk. A second-tier "validate-then-rebuild"
  recovery is a counted, named follow-up — not this Pass.

**Header case:** `probe_header` failure ("no `%PDF-`", 22×) is a related recovery
— rebuild-by-scan is header-independent: it finds `N G obj` at **absolute** byte
positions and never trusts stored offsets, so it also solves the "leading bytes
before `%PDF-`" / offset-start case that decision 007 flagged as the one genuine
non-`*-fail-*` corpus gap (`PDF 2.0 with offset start.pdf`). Attempt
scan-recovery on header failure **only if** the scan locates a plausible object
structure and a `/Catalog`; otherwise fail clean as "not a PDF". Detect the
version by scanning for `%PDF-` within the first ~1 KiB, else default
conservatively. Keep the **primary framing on xref-rebuild**; the offset-start
gap is subsumed for free.

#### 3.3.2 The scan algorithm

**Primitive.** Reuse the existing object-header lexer /
`Parser::parse_indirect_object` (parser.rs already lexes `N G obj`). No new
tokenizer, no new dependency (rule 13) — the scan reuses the exact `N G obj`
acceptance the rest of the loader uses.

**Phase 1 — file-level.** A single linear pass locates every `N G obj` header;
record `(obj_num, generation, byte_offset)` and synthesize
`XrefEntry::InUse { offset, generation }`. Duplicate object numbers:
**last-wins** by file order (reader convention — incremental updates append, so a
later definition supersedes).

**Phase 2 — object streams.** *After* phase 1 (forced order, identical to
document.rs's existing two-phase load): parse each recovered file-level object;
for any stream with `/Type /ObjStm`, decode it and read its `/N` pair table,
synthesizing `XrefEntry::InStream { stream_num, index }` (type-2) for each
compressed member. Compressed members are **not** at file-level `N G obj`
offsets, so the scan cannot find them directly — the container's pair table is
the authority, exactly as a type-2 xref entry normally is. **Conflict rule
(documented limitation):** a file-level definition wins over an ObjStm member of
the same number (true newest-wins needs revision ordering the scan discards).
For a normal xref-stream file, most objects arrive via phase 2 — the design must
handle "few file-level objects, bulk from ObjStm".

**Trailer recovery.**
- Find the **last** `trailer` keyword and parse its dict → `/Root`, `/Info`,
  `/Size`, `/Encrypt`, `/ID` directly.
- If there is no `trailer` keyword (pure xref-stream files have none): synthesize
  a trailer by scanning recovered objects for a dict with `/Type /Catalog`,
  set `/Root` to its reference, `/Size` = max recovered number + 1, preserve
  `/ID` if any recovered dict carried one (needed for §7.6 key derivation and R39
  continuity).
- If `/Encrypt` is present → refuse (`EncryptionUnsupported`).
- If **no** `/Catalog` can be found by any route → recovery **fails clean** with
  a named error. No partial/garbage `Document` is ever returned.

#### 3.3.3 Resource guards (R25)

- The scan is O(n) single-pass — bounded by file size, no backtracking, must not
  be quadratic.
- Total synthesized entries capped by the existing `MAX_XREF_ENTRIES` (10M); an
  `obj`-token-dense adversarial file stops at the cap and fails clean.
- ObjStm decoding respects the §10.1 decompression-bomb guards.
- No recursion (§7.5.7's two-level guarantee holds for recovered containers).
- Fail-clean, never hang: linear scan + entry cap + "no catalog → refuse"
  together bound worst-case work.

#### 3.3.4 The honest interaction with §5

A file loaded via recovery had a **broken** cross-reference table, so its bytes
**cannot** be preserved byte-identically on save — the original xref was invalid,
and an incremental append would write a section whose `/Prev` points at a
cross-reference section that does not correctly exist. **Incremental-append onto
a broken base is structurally impossible, not merely undesirable.**

Therefore:
- A recovered document's save is **necessarily a clean full rebuild**
  (`save_full` — regenerating a fresh valid xref/trailer/startxref).
- `save_incremental` on a recovered document is **refused** with a named
  `WriteError` (recommend `RecoveredBaseForbidsIncremental`), mirroring the R35
  (redaction) / R58 (removal/scrub) full-rewrite-forcing pattern exactly.
- **Normalization note:** §5.6's "never normalize" governs *clean passthrough*
  objects — it forbids reformatting a file the operator loaded intact. It does
  **not** bind a recovered file: the base was invalid, so emitting a fresh
  normalized classic xref (`SectionShape::Classic { xref_stm: None }` — the most
  compatible form) is the correct, honest output, not a violation of R33/§5.6.
- The `Document` gains a recovery-state field (recommend
  `recovery: Option<RecoveryReport>`). The writer reads it to force full rewrite;
  the GUI/CLI read it to disclose.

#### 3.3.5 Fuzzy-never-sneaky and R20 disclosure

A recovered open is **disclosed and counted**, never a silent "repair". The load
result distinguishes *loaded normally* from *loaded via xref recovery* — the
reconstructed xref is a reviewable fact surfaced to the operator (project rule
4), not a silent auto-apply.

`RecoveryReport` counts: the originating failure reason; objects recovered by
file-level scan; objects recovered from ObjStm containers; duplicate object
numbers resolved (last-wins collisions); trailer source (parsed `trailer` vs
synthesized from `/Catalog`); whether offset-start rebasing was involved.

- **CLI:** reports recovery on load (a diagnostic line) and via a distinct,
  documented exit-code / status, so a batch script can tell "opened clean" from
  "opened via recovery" (R20 counted-diagnostics tradition).
- **GUI:** a non-blocking banner — *"This document had a damaged cross-reference
  table and was rebuilt in memory. Saving will rewrite (normalize) the file."*
  Dispatch pdfce-ui-specialist for the exact surface/wording.

#### 3.3.6 Spec posture

xref recovery is **not spec-normative**. §7.5.4 (classic table), §7.5.5 (load
algorithm / `startxref` / `%%EOF`), and Annex H.7 define the *well-formed*
structure; none defines a recovery procedure. Recovery is a deliberate
reader-robustness **policy** grounded in universal reader behavior (pdfium, qpdf,
poppler, mupdf, pdf.js all rebuild-by-scan) — the same outcome-over-method
pattern pdfce already uses (e.g. tolerating a missing `/Type` on an xref stream).
Cite §7.5.4 / §7.5.5 / Annex H.7 in code doc comments for the well-formed
structure being **reconstructed**, and mark the recovery **algorithm** as pdfce
policy (a peer of `STARTXREF_SCAN_WINDOW` / `MAX_XREF_SECTIONS`), not a spec
requirement. Fail-clean where recovery cannot succeed (R27).

---

## 4. Pass B deliverables

1. A pdfce-core recovery module (e.g. `src/recover.rs`): `&[u8] →
   Result<RecoveredXref, RecoverError>`, reusing the parser's `N G obj` lexing
   and the existing filter/objstm machinery.
2. `document.rs::from_bytes` wired to attempt recovery **only** on the triggering
   `XrefError` kinds (and gated header failure), populating the recovery flag +
   `RecoveryReport`.
3. A new fail-clean error variant for "recovery attempted but no catalog found".
4. Writer guard: `save_incremental` refuses a recovered document
   (`WriteError::RecoveredBaseForbidsIncremental`); `save_full` emits a fresh
   valid classic xref for it.
5. pdfce-cli: recovery disclosure on load + documented exit-code/status
   distinguishing clean vs recovered open; an `inspect`/diagnostic surfacing the
   `RecoveryReport` counts.
6. Fixtures (`fixtures/synthetic`): offset-shifted-startxref (the
   qpdf/add-contents.pdf shape), no-startxref, startxref-out-of-range, pure
   xref-stream with a corrupted stream (recover file-level + ObjStm),
   duplicate/superseded objects (last-wins), offset-start (leading bytes before
   `%PDF-`), and an unrecoverable file (no catalog → clean refusal).
7. A `parse → recover → re-load` fuzz target: recovery never panics, always
   terminates, always returns a valid `Document` or a clean error.
8. ARCHITECTURE §5 amendment adding the recovered-base-forces-full-rewrite rule
   as a sibling to §5.2/R35 and §5.9/R58; the new standing rule (R59).
9. personal_rag/pdf finding(s): real-world CRLF PDFs carry offset-shift
   corruption; rebuild-by-scan conventions (last-wins, ObjStm-from-pair-table,
   trailer-from-`/Catalog`).

---

## 5. Acceptance criteria (Pass B)

- **Real-world corpus (1,109 files):** rebuild-by-scan opens a substantial share
  of the ~605 previously-failing valid files — target the bulk of the 423
  `NotAnXrefSection` + 94 malformed-header + 13 no-startxref + 7 out-of-range +
  20 bad-entry buckets (and, via the header case, the 22 no-`%PDF-` files where a
  catalog is findable). Report converted vs still-failing **by file and by
  reason** (R20 counted shortfall, never rounded).
- **Clean conformance (2,890 files):** ZERO regressions. Recovery must never fire
  on a cleanly-loading file. Verify empirically (identical object counts / load
  outcomes), not merely by trusting trigger placement.
- **`*-fail-*` reconciliation (the hardest gate):** enumerate every deliberate
  veraPDF/Isartor `*-fail-*` file whose status **changes** under recovery
  (previously-refused → now-opens). For each, confirm the change is **defensible
  reader behavior** (the file is genuinely rebuild-by-scan-recoverable the way
  qpdf/pdfium would open it — pdfce is a reader/editor, not a conformance
  validator, so opening a damaged-but-recoverable file is correct), **not**
  recovery masking a real parse bug. A `*-fail-*` file that fails for a *non-xref*
  reason (font, metadata, a PDF/A rule) is structurally valid and will not
  trigger recovery at all; only xref-damage `*-fail-*` files are at risk and must
  be individually justified.
- Recovery always terminates and fail-cleans on adversarial input (fuzz green;
  `MAX_XREF_ENTRIES` + no-catalog-refusal enforced).
- A test proves `save_incremental` is refused on a recovered document and
  `save_full` produces a reloadable file with a valid fresh xref.
- A test proves the disclosure is present (`RecoveryReport` populated; CLI status
  distinct).
- veraPDF §6.1.12 implementation-limits suite run against any new recovery-side
  guard (standing rule).
- `cargo fmt --check` + `cargo clippy -- -D warnings` clean; `cargo tree -p
  pdfce-core` shows no GUI dep; **no new dependency** (rule 13).

---

## 6. Explicit non-goals (Pass B)

- No object-level lenient loading — a clean xref but corrupt object bodies still
  fails strict this Pass.
- No recovery of encrypted files — `EncryptionUnsupported` still refuses
  post-rebuild.
- No second-tier "validate-clean-xref-then-rebuild-if-offsets-wrong" recovery
  (only hard xref-parse **failure** triggers). A counted follow-up.
- No repair written back silently — recovery is in-memory + disclosed; the file
  on disk is unchanged until an explicit, disclosed full-rewrite save.
- No linearization repair; no normalization of anything on a clean file.

---

## 7. Risks

| id | Risk | Mitigation |
|---|---|---|
| B-W1 | Recovery masks a real parser bug — a `*-fail-*` file that should stay refused now "recovers". | The `*-fail-*` reconciliation gate: every status change enumerated and individually justified. |
| B-W2 | A cleanly-loading file is perturbed by recovery creeping onto the hot path. | Trigger strictly on the error path; assert zero regression on the 2,890 clean files by object-count identity. |
| B-W3 | last-wins picks the wrong revision where physical order ≠ revision order. | Documented limitation; matches universal reader convention; true ordering is unrecoverable once the chain is broken. |
| B-W4 | ObjStm-vs-file-level precedence chooses a stale compressed copy. | File-level (scanned) wins over ObjStm member of the same number; documented; counted. |
| B-W5 | Adversarial `obj`-dense file causes excessive work. | Single linear pass + `MAX_XREF_ENTRIES` cap + fail-clean-on-no-catalog; fuzz enforces termination. |
| B-W6 | Recovered doc saved incremental leaves a broken `/Prev` and silently corrupts. | `save_incremental` refused on recovered docs (WriteError), sibling to R35/R58; full rewrite mandatory and disclosed. |

Pass-A-specific risk: editing the hot clean-load path could accept a malformed
table (false green — decision 007 W10). Keep strict rejection; broaden only to
spec-permitted, fixture-proven forms; assert exact entry length in a unit test.

---

## 8. Spec prerequisites

**Light — no new spec ingestion expected.** §7.5.4 (incl. the exact 20-byte /
`SP CR | SP LF | CR LF` EOL rule), §7.5.5, §7.5.6, and Annex H.7 already exist in
`D:\Dev\Rag-Specialized\PDF_Spec\iso32000\` and were confirmed to carry the
EOL-exactness text. Because recovery is **non-normative**, there is no recovery
clause to ingest. Dispatch pdfce-spec-librarian only to **confirm** the §7.5.4
EOL/20-byte details for Pass A and to record, as a negative, that no ISO clause
defines a recovery algorithm — not to build new corpus. The scan reuses parser.rs
and the existing filters/objstm machinery, so **no new dependency** is required.

---

## 9. New standing rule

**R59 (librarian confirms the next available number; §5.9 already used R58):**
A document loaded via cross-reference recovery had an invalid base xref and
cannot be incrementally appended to; its save is a mandatory full rewrite
emitting a fresh valid cross-reference, and the recovered/rebuilt status is
flagged on the `Document`, disclosed in CLI + GUI, and counted (R20).
`save_incremental` on a recovered document is refused by name. Sibling of R35
(redaction forces full rewrite) and R58 (removal/scrub forces full rewrite); the
§5.6 "never normalize" rule does not bind a recovered file because its base was
already invalid. Recovery triggers exclusively on the strict-load error path, so
the round-trip/minimal-diff invariant for cleanly-loading files is preserved by
construction.

---

## 10. Follow-up items for the librarian

1. File Pass A (CRLF/EOL audit) and Pass B (rebuild-by-scan) under *Next up*,
   A before B; annotate the Backlog with the 605/712 (85%) real-world finding.
2. Add R59 to the ROADMAP standing rules; assign the concrete Pass numbers.
3. Amend ARCHITECTURE §5 with the recovered-base-forces-full-rewrite rule
   (sibling to §5.2/R35, §5.9/R58) once Pass B ships.
4. Record the CRLF-is-offset-shift finding (and Pass A's actual result) in
   `C:\personal_rag\pdf\`, creating the subject if it does not yet exist.
5. Append the §12 decision-log entry for decision 013.
6. Reconcile with decision 007 §10 item 6: `PDF 2.0 with offset start.pdf` is
   subsumed by Pass B's header/offset-start handling — close it as tracked here.

---

## Appendix A — JSON decision block

```json
{
  "decision_id": "013",
  "title": "Cross-reference recovery: rebuild-by-scan fallback + a CRLF xref-table-parser audit, without perturbing files that already load cleanly",
  "date": "2026-07-31",
  "status": "Decided (consultation — engineer implements from this block)",
  "decider": "KenAgent (autonomous-builder), per the ROADMAP KenAgent-decision-routing rule",
  "question": "How should pdfce recover from a broken/mislocated cross-reference table so it can OPEN the large real-world class that a strict xref parse rejects — a rebuild-by-scan fallback plus a CRLF xref-table audit — without compromising the round-trip/minimal-diff invariant (ARCHITECTURE.md §5) for files that already load cleanly?",

  "decision": "Ship xref recovery as TWO sequenced Passes. Pass A first: an EOL/CRLF classic-table audit — a MEASUREMENT that runs valid-CRLF fixtures plus a sample of the corpus's 241 CRLF files through the existing parser, fixes any genuinely-mishandled valid table (strict correctness bug, independent of recovery), and documents the result either way. Pass B second: a rebuild-by-scan recovery path that triggers ONLY on a hard cross-reference load failure, scans the whole buffer for `N G obj` headers to synthesize an xref + trailer, walks recovered /ObjStm containers for compressed members, and hands the document layer a fully-formed table. Recovery is FLAGGED on the Document, DISCLOSED (CLI + GUI), COUNTED (R20), and FORCES a full-rewrite save (incremental-append onto a broken base is refused, sibling to R35/R58). A cleanly-parsing file is byte-for-byte untouched: the trigger sits on the error path only, so §5 for clean files is structurally unaffected.",

  "headline_finding": "Reading xref.rs closely, pdfce's classic-table parser is CRLF-correct for all three §7.5.4 EOL forms (SP CR / SP LF / CR LF): parse_entry matches all three, entries are read as exact 20-byte records, and entry_pos is recomputed via skip_one_eol (which handles CR, LF, and CRLF) at the start of every subsection, so no per-line drift can accumulate. The strong CRLF correlation (237/241 of the top failure bucket) is therefore most plausibly an OFFSET-SHIFT artifact: files authored with LF, then converted to CRLF in text-mode transport, gain one byte per line before the xref — invalidating both the stored `startxref` value AND every in-table byte offset. The canonical example confirms this: qpdf/add-contents.pdf stores `startxref 685` but byte 685 lands inside `...endobj\\r\\n8 0 obj` (real xref at 724, a 39-byte forward shift). That is not a parser bug; it is a file whose stored offsets cannot be trusted at all — which is exactly what rebuild-by-scan fixes and a parser tweak cannot. CONSEQUENCE for slicing: Pass A is expected to be a cheap DISAMBIGUATION (possibly a documented negative result), NOT the thing that recovers the 605. Do not bank the 605 on Pass A. This is measurement-first discipline (decisions 005/006) applied to the one hypothesis that would otherwise send the engineer hunting a phantom bug.",

  "pass_slicing": {
    "recommendation": "Two Passes, A before B. A is a correctness audit with a conditional fix; B is the recovery subsystem. A must precede B because B's acceptance classification DEPENDS on A's result — you cannot correctly label a recovered file 'valid-that-pdfce-wrongly-rejected' vs 'genuinely-broken-needing-rebuild' until the parser is confirmed correct on valid CRLF input.",
    "why_not_one_pass": "Folding them hides the disambiguation. If A finds a real bug, it is a strict correctness fix that should ship on its own merits and its own regression test, not buried inside a large recovery Pass. If A finds nothing, that negative result is itself the deliverable that justifies putting all effort into B. Separating them keeps the risk profiles distinct: A touches the hot clean-load path (high blast radius, tiny change); B is pure error-path new code (low blast radius, large change)."
  },

  "pass_A": {
    "id": "Pass A (recommend the librarian assign the concrete number)",
    "name": "Classic xref-table EOL/CRLF audit + conditional strict-correctness fix",
    "goal": "Determine empirically whether pdfce rejects any WELL-FORMED CRLF-authored classic table (a strict §7.5.4 bug), separate from genuinely-broken xref. Fix if found; document either way.",
    "method": [
      "Construct synthetic fixtures under fixtures/synthetic: valid classic tables with each of the three legal EOLs (SP CR, SP LF, CR LF) on BOTH the entry lines AND the subsection-header/`xref`/`trailer`/`startxref` lines; multi-subsection tables; tables with trailing spaces before EOL; a bare-CR-only file (old-Mac EOL) and a mixed-EOL file. Assert every well-formed one loads.",
      "Take a bounded sample (e.g. 25-40) of the corpus's 241 CRLF-correlated failures, and for each classify the ACTUAL root cause by hand: (a) stored startxref lands off the real `xref`/xref-stream (offset-shift — NOT a parser bug, needs Pass B), vs (b) startxref is correct and the classic-table parser still rejects a structurally-valid table (a real Pass-A bug).",
      "If and only if class (b) appears: fix the parser, add a regression fixture per distinct bug shape, and verify the fix reopens those specific files WITHOUT changing any currently-Ok file."
    ],
    "deliverables": [
      "Synthetic valid-CRLF fixture set + tests proving all three EOL forms (and trailing-space / bare-CR variants) parse.",
      "A short classification tally of the sampled CRLF failures: how many are offset-shift (Pass B territory) vs genuine parser rejections (Pass A territory). Counted, R20-style.",
      "IF a bug is found: the fix + regression tests + a zero-regression proof over the 2,890 clean conformance files.",
      "IF no bug is found: a documented negative result (in the decision record + SESSION_LOG) recording that the CRLF correlation is offset-shift, so the recovery burden falls entirely on Pass B. A personal_rag/pdf finding is filed either way (real-world CRLF PDFs carry offset-shift corruption; the classic-table parser is / is-not EOL-correct)."
    ],
    "acceptance": [
      "Every synthetic well-formed CRLF/CR/LF classic table loads.",
      "The 2,890 clean conformance files show ZERO change in load outcome and object counts (this Pass touches the hot path — the guard against a regression is mandatory).",
      "cargo fmt --check + cargo clippy -- -D warnings clean; cargo tree -p pdfce-core shows no GUI dep.",
      "The sampled-failure classification is recorded with counts."
    ],
    "risk": "This Pass edits the clean-load path. A careless 'tolerance' change here could silently accept a malformed table (a false green — decision 007 W10). Mitigation: keep strict rejection of genuinely-malformed entries; only broaden to accept forms the spec explicitly permits and that a valid-fixture proves; assert exact entry length in unit tests, never rely on round-trip reload to catch a bad entry.",
    "expected_outcome": "Most likely a NEGATIVE result (no parser bug; correlation is offset-shift). Framed as cheap disambiguation, not as the 605-file fix."
  },

  "pass_B": {
    "id": "Pass B (librarian assigns number; sequenced after A)",
    "name": "Rebuild-by-scan cross-reference recovery",
    "goal": "Open the large real-world class (the ~605 failures, dominated by 423 NotAnXrefSection + 94 malformed-indirect-header-at-xref + 13 no-startxref + 7 out-of-range + 20 bad-entry) by reconstructing the xref and trailer from a full-file object scan when the stored cross-reference machinery cannot be parsed. Reader-robustness parity with pdfium/qpdf/poppler/mupdf/pdf.js.",

    "trigger_rule_LOAD_BEARING": {
      "principle": "Recovery is a FALLBACK, never the default. It runs ONLY after the strict path has FAILED. A file that returns Ok(LoadedXref) from load_xref_chain takes the normal path unchanged, with zero recovery code in its flow — the round-trip/minimal-diff invariant for clean files is therefore untouched by construction, not by policy.",
      "where": "In document.rs::from_bytes, wrap the two current failure points: (1) `xref::load_xref_chain(&buf)` returning Err(XrefError), and (2) probe_header returning Err (see header_case). Only on those Errs is recovery attempted.",
      "triggers_on": [
        "XrefErrorKind::StartxrefNotFound",
        "XrefErrorKind::BadStartxrefOffset",
        "XrefErrorKind::NotAnXrefSection",
        "XrefErrorKind::BadEntry",
        "XrefErrorKind::BadSubsectionHeader",
        "XrefErrorKind::BadTrailer(_)",
        "XrefErrorKind::PrevChainCycle",
        "XrefErrorKind::TooManyEntries (bounded re-attempt under the scan cap)",
        "XrefErrorKind::BadXrefStream(_) / XrefStreamDecode(_) (the stream is unrecoverable in place; scan the file-level objects instead)"
      ],
      "must_NOT_trigger_on": [
        "XrefErrorKind::EncryptionUnsupported — a deliberate NAMED capability gap, not damage. Scanning would still surface encrypted strings/streams; the correct behavior is the same up-front refusal. Recovery MUST re-check for /Encrypt after rebuilding the trailer and refuse with EncryptionUnsupported, so a broken-xref-AND-encrypted file still fails clean for the right reason.",
        "Object-level failures AFTER a clean xref (DocError::BadObject / ObjectIdMismatch / ObjectStream*). Those mean the xref parsed but the body is corrupt — object-level lenient loading is a separate future feature (the parser/document module docs already flag it). Scoping to xref-parse failure keeps this Pass tight. Documented limitation: a file whose xref parses structurally but whose offsets are uniformly wrong (offset-shift that spared startxref) would hit ObjectIdMismatch and is NOT recovered this Pass — but the dominant offset-shift case ALSO breaks startxref (→ NotAnXrefSection → recovery fires), so this covers the bulk. Second-tier 'validate-then-rebuild' recovery is a counted, named follow-up, not this Pass."
      ],
      "header_case": "probe_header failure ('no %PDF- header', 22×) is a RELATED recovery: rebuild-by-scan is header-independent (it finds `N G obj` at ABSOLUTE byte positions and never trusts stored offsets, so it also solves the 'leading bytes before %PDF-' / offset-start case that decision 007 §10 item 6 flagged as the one genuine non-*-fail-* corpus gap, `PDF 2.0 with offset start.pdf`). Attempt scan-recovery on header failure ONLY if the scan locates a plausible object structure AND a /Catalog; otherwise fail clean as 'not a PDF'. Detect the PDF version by scanning for `%PDF-` anywhere in the first ~1 KiB, else default conservatively. This elegantly subsumes the offset-start gap for free; keep the PRIMARY framing on xref-rebuild."
    },

    "scan_algorithm": {
      "primitive": "Reuse the existing object-header lexer / Parser::parse_indirect_object (parser.rs already lexes `N G obj`). Do NOT hand-roll a new tokenizer — no new deps (rule 13), and the scan must reuse the exact `N G obj` acceptance the rest of the loader uses.",
      "phase_1_file_level": "Single linear pass over the buffer locating every `N G obj` header. For each, record (obj_num, generation, byte_offset). Build synthetic XrefEntry::InUse { offset, generation }. Duplicate object numbers: LAST-WINS by file order (matches reader convention: incremental updates append, so the later definition supersedes). This mirrors how the merged view already works, but derived from physical position instead of the trusted xref.",
      "phase_2_objstm": "AFTER phase 1 (forced order, identical to document.rs's existing two-phase load): parse each recovered file-level object; for any stream with /Type /ObjStm, decode it and read its /N pair table, synthesizing an XrefEntry::InStream { stream_num, index } (type-2) for each compressed member. Compressed members are NOT at file-level `N G obj` offsets, so the scan cannot find them directly — the container's pair table is the authority, exactly as a type-2 xref entry normally is. Conflict rule (documented limitation): a file-level definition found by the scan takes precedence over an ObjStm member of the same number (true newest-wins needs revision ordering the scan discards; note it). For a normal xref-stream file most objects arrive via this phase — the design must handle 'few file-level objects, bulk from ObjStm'.",
      "trailer_recovery": [
        "Find the LAST `trailer` keyword and parse its dict → gives /Root, /Info, /Size, /Encrypt, /ID directly.",
        "If no `trailer` keyword (pure xref-stream files have none — the trailer is the xref-stream dict, which we could not parse): synthesize a trailer by scanning recovered objects for a dict with /Type /Catalog and setting /Root to that object's reference. /Size = max recovered object number + 1. /ID preserved if any recovered trailer/xref-stream dict carried one (needed for §7.6 key derivation and R39 continuity).",
        "If /Encrypt is present in the recovered trailer → refuse with EncryptionUnsupported (consistent with the clean path).",
        "If NO /Catalog can be found by any route → recovery FAILS clean with a named error (see fail_clean). No partial/garbage Document is ever returned."
      ]
    },

    "guards_R25": [
      "The scan is O(n) single-pass over the buffer — inherently bounded by file size, no backtracking, must not be quadratic.",
      "Total synthesized entries capped by the existing MAX_XREF_ENTRIES (10M); an adversarial file dense with `obj`-like tokens stops at the cap and fails clean rather than allocating unbounded.",
      "ObjStm decoding during phase 2 respects the same decompression-bomb guards (ARCHITECTURE §10.1) as the normal path.",
      "No recursion to guard — §7.5.7's two-level guarantee (an ObjStm cannot contain an ObjStm) holds for recovered containers too.",
      "A pathological file must fail-clean, never hang: the linear-scan + entry cap + 'no catalog → refuse' together bound worst-case work."
    ],

    "section5_reconciliation": {
      "core_fact": "A file loaded via recovery had a BROKEN cross-reference table, so its bytes CANNOT be preserved byte-identically on save — the original xref was invalid, and an incremental append would write a new section whose /Prev points at a cross-reference section that does not correctly exist. Incremental-append onto a broken base is therefore structurally impossible, not merely undesirable.",
      "rule": "A recovered document's save is NECESSARILY a clean full REBUILD (save_full — regenerating a fresh, valid xref/trailer/startxref). save_incremental on a recovered document is REFUSED with a named WriteError (recommend WriteError::RecoveredBaseForbidsIncremental), mirroring the R35 (redaction) / R58 (removal/scrub) full-rewrite-forcing pattern exactly.",
      "normalization_note": "ARCHITECTURE §5.6 'never normalize' governs CLEAN passthrough objects — it forbids reformatting a file the operator loaded intact. It does NOT bind a recovered file: the base was invalid, so emitting a fresh normalized classic xref (SectionShape::Classic { xref_stm: None } by default — the most compatible form) is the CORRECT and honest output, not a normalization violation. State this interaction explicitly so a future reader does not think recovery breaks R33/§5.6.",
      "flag": "Document gains a recovery-state field (recommend `recovery: Option<RecoveryReport>` or `loaded_via_recovery: bool` + a RecoveryReport with counts). The writer reads it to force full-rewrite; the GUI/CLI read it to disclose. section_shape() for a recovered doc returns the synthesized Classic form."
    },

    "disclosure_fuzzy_never_sneaky_R20": {
      "principle": "A recovered open is DISCLOSED and COUNTED, never a silent 'repair'. The load result distinguishes 'loaded normally' from 'loaded via xref recovery'. This is the fuzzy-never-sneaky rule (project rule 4): the reconstructed xref is a reviewable fact surfaced to the operator, not a silent auto-apply.",
      "RecoveryReport_counts": [
        "reason the strict path failed (the originating XrefError kind / header failure)",
        "objects recovered by file-level scan",
        "objects recovered from ObjStm containers",
        "duplicate object numbers resolved (last-wins collisions)",
        "trailer source: parsed `trailer` keyword vs synthesized from /Type /Catalog",
        "whether offset-start rebasing was involved (header not at byte 0)"
      ],
      "CLI": "pdfce-cli reports recovery on load (a diagnostic line) and via a distinct, documented exit-code / status so a batch script can tell 'opened clean' from 'opened via recovery'. Consistent with the R20 counted-diagnostics tradition.",
      "GUI": "A non-blocking banner/notice: 'This document had a damaged cross-reference table and was rebuilt in memory. Saving will rewrite (normalize) the file.' Dispatch pdfce-ui-specialist for the exact surface/wording."
    },

    "spec_posture": {
      "non_normative": "xref recovery is NOT spec-normative. §7.5.4 (classic table), §7.5.5 (load algorithm / startxref / %%EOF), and Annex H.7 define the WELL-FORMED structure; none defines a recovery procedure. Recovery is a deliberate reader-robustness POLICY grounded in universal reader behavior (pdfium, qpdf, poppler, mupdf, pdf.js all rebuild-by-scan) — the same outcome-over-method pattern pdfce already uses (e.g. tolerating a missing /Type on an xref stream).",
      "citations": "Cite §7.5.4 / §7.5.5 / Annex H.7 in code doc comments for the well-formed structure being RECONSTRUCTED, and mark the recovery ALGORITHM itself as pdfce policy (a peer of STARTXREF_SCAN_WINDOW / MAX_XREF_SECTIONS), not a spec requirement. Fail-clean where recovery cannot succeed."
    },

    "deliverables": [
      "pdfce-core recovery module (e.g. src/recover.rs) exposing a function that takes &[u8] and returns Result<RecoveredXref, RecoverError>, reusing the parser's `N G obj` lexing and the existing filter/objstm machinery.",
      "document.rs::from_bytes wired to attempt recovery ONLY on the triggering XrefError kinds (and gated header failure), populating the Document recovery flag + RecoveryReport.",
      "A new XrefErrorKind / DocError / RecoverError variant for 'recovery attempted but no catalog found' (fail-clean).",
      "writer guard: save_incremental refuses a recovered document (WriteError::RecoveredBaseForbidsIncremental); save_full emits a fresh valid classic xref for it.",
      "pdfce-cli: recovery disclosure on load + documented exit-code/status distinguishing clean vs recovered open (and an `inspect`/diagnostic surfacing the RecoveryReport counts).",
      "Fixture set under fixtures/synthetic: offset-shifted-startxref file (the qpdf/add-contents.pdf shape), no-startxref file, startxref-out-of-range file, pure xref-stream file with a corrupted stream (recover file-level + ObjStm), a file with duplicate/superseded objects (last-wins), an offset-start file (leading bytes before %PDF-), and an unrecoverable file (no catalog → clean refusal).",
      "A parse→recover→re-load fuzz target extension (recovery must never panic; must always terminate; must return a valid Document or a clean error).",
      "ARCHITECTURE §5 amendment adding the recovered-base-forces-full-rewrite rule as a sibling to §5.2/R35 and §5.9/R58; a new standing rule (recommend R59 — librarian confirms next number).",
      "personal_rag/pdf finding(s): real-world CRLF PDFs carry offset-shift corruption; rebuild-by-scan conventions (last-wins, ObjStm-from-pair-table, trailer-from-/Catalog)."
    ],

    "acceptance_criteria": [
      "RE-RUN over the 1,109-file real-world corpus: rebuild-by-scan OPENS a substantial share of the ~605 previously-failing valid files — target the bulk of the 423 NotAnXrefSection + 94 malformed-header + 13 no-startxref + 7 out-of-range + 20 bad-entry buckets (and, via the header case, the 22 no-%PDF- files where a catalog is findable). Report converted vs still-failing BY FILE AND BY REASON (R20 counted shortfall — never rounded).",
      "RE-RUN over the 2,890 clean conformance files: ZERO regressions. Recovery must never fire on a cleanly-loading file. Verify empirically (identical object counts / load outcomes), not just by trusting the trigger placement.",
      "*-fail-* reconciliation (the trickiest gate): enumerate EVERY deliberate veraPDF/Isartor *-fail-* file whose status CHANGES under recovery (previously-refused → now-opens). For each, confirm the change is DEFENSIBLE reader behavior (the file is genuinely rebuild-by-scan-recoverable the way qpdf/pdfium would open it — pdfce is a reader/editor, not a conformance validator, so opening a damaged-but-recoverable file is correct), NOT recovery masking a real parse bug that should have been fixed properly. A *-fail-* file that fails for a NON-xref reason (font, metadata, PDF/A rule) is structurally valid and will not trigger recovery at all; only the xref-damage *-fail- files are at risk and must be individually justified.",
      "Recovery always terminates and fail-cleans on adversarial input (fuzz target green; MAX_XREF_ENTRIES + no-catalog-refusal enforced).",
      "A test proves save_incremental is REFUSED on a recovered document and save_full produces a reloadable file with a valid fresh xref.",
      "A test proves a recovered-then-saved file's DISCLOSURE is present (RecoveryReport populated; CLI status distinct).",
      "veraPDF §6.1.12 implementation-limits suite run against any new recovery-side guard (standing rule; two prior intuition-guard incidents).",
      "cargo fmt --check + cargo clippy -- -D warnings clean; cargo tree -p pdfce-core shows no GUI dep; no new dependency added (rule 13)."
    ],

    "explicit_non_goals": [
      "No object-level lenient loading — a file with a clean xref but corrupt object bodies still fails strict this Pass.",
      "No recovery of encrypted files — EncryptionUnsupported still refuses post-rebuild.",
      "No second-tier 'validate-clean-xref-then-rebuild-if-offsets-wrong' recovery (only hard xref-parse FAILURE triggers). Documented as a counted follow-up.",
      "No repair written back silently — recovery is in-memory + disclosed; the file on disk is unchanged until an explicit, disclosed full-rewrite save.",
      "No linearization repair, no normalization of anything on a CLEAN file (unchanged by this Pass)."
    ],

    "risks": [
      { "id": "B-W1", "risk": "Recovery masks a real parser bug: a *-fail-* file that SHOULD stay refused now 'recovers', hiding a defect.", "mitigation": "The *-fail- reconciliation gate: every status change enumerated and individually justified as legitimate reader recovery vs bug-masking." },
      { "id": "B-W2", "risk": "A cleanly-loading file is perturbed by recovery code creeping onto the hot path.", "mitigation": "Trigger strictly on the error path; assert zero regression on the 2,890 clean files by object-count identity." },
      { "id": "B-W3", "risk": "last-wins duplicate resolution picks the wrong revision for a file where physical order != revision order (rare).", "mitigation": "Documented limitation; matches universal reader convention; the alternative (true revision ordering) is unrecoverable once the xref chain is broken." },
      { "id": "B-W4", "risk": "ObjStm-from-pair-table vs file-level precedence chooses a stale compressed copy.", "mitigation": "File-level (scanned) definition wins over ObjStm member of the same number; documented; counted in RecoveryReport." },
      { "id": "B-W5", "risk": "Adversarial `obj`-token-dense file causes excessive work.", "mitigation": "Single linear pass + MAX_XREF_ENTRIES cap + fail-clean-on-no-catalog; fuzz target enforces termination." },
      { "id": "B-W6", "risk": "Recovered doc saved incremental would leave a broken /Prev and silently corrupt.", "mitigation": "save_incremental refused on recovered docs (WriteError), sibling to R35/R58; full-rewrite mandatory and disclosed." }
    ]
  },

  "spec_prereqs": {
    "status": "LIGHT — no new spec ingestion expected.",
    "detail": "§7.5.4 (incl. the exact 20-byte / SP-CR|SP-LF|CR-LF EOL rule), §7.5.5 (load algorithm, startxref, %%EOF), §7.5.6 (incremental / most-recent-copy), and Annex H.7 already exist in D:\\Dev\\Rag-Specialized\\PDF_Spec\\iso32000\\ and were confirmed to contain the EOL-exactness text (7.5.4 'eol — 2-character EOL: SP CR, SP LF, or CR LF'). Because recovery is NON-normative, there is no recovery clause to ingest. Dispatch pdfce-spec-librarian only to CONFIRM the §7.5.4 EOL/20-byte details for Pass A and to note in the RAG that no ISO clause defines a recovery algorithm (a recorded negative), NOT to build new corpus.",
    "no_new_deps": "Confirmed feasible: the scan reuses parser.rs's `N G obj` header lexing (Parser::parse_indirect_object) and the existing filters/objstm machinery. Rule 13 satisfied."
  },

  "new_standing_rule": {
    "recommend": "R59 (librarian confirms next available; ARCHITECTURE §5.9 already used R58)",
    "text": "A document loaded via cross-reference recovery had an invalid base xref and CANNOT be incrementally appended to; its save is a mandatory full rewrite emitting a fresh valid cross-reference, and the recovered/rebuilt status is flagged on the Document, disclosed in CLI + GUI, and counted (R20). save_incremental on a recovered document is refused by name. This is a sibling of R35 (redaction forces full rewrite) and R58 (removal/scrub forces full rewrite); the §5.6 'never normalize' rule does not bind a recovered file because its base was already invalid.",
    "also_note": "The recovery trigger sits exclusively on the strict-load error path, so the round-trip/minimal-diff invariant for cleanly-loading files is preserved by construction — recovery can never perturb a file that loads fine."
  },

  "risks_gotchas_top": [
    "Do NOT expect Pass A to recover the 605 — the CRLF correlation is offset-shift, and rebuild-by-scan (Pass B) carries the load. Banking on a phantom CRLF parser bug would waste the Pass.",
    "Recovery must re-check /Encrypt after rebuilding the trailer and refuse — otherwise a broken-xref-and-encrypted file would slip past the deliberate capability gap.",
    "The single hardest acceptance gate is *-fail-* reconciliation: distinguish 'legitimate reader recovery of a damaged-but-recoverable file' from 'recovery masking a real bug'. Enumerate every status change.",
    "Per-file, per-reason counted reporting (R20) on BOTH corpora is the deliverable, not a rounded percentage — an honest counted shortfall is the whole point (decision 007 W14 discipline).",
    "A recovered-doc save that used incremental would silently corrupt via a dangling /Prev — the WriteError refusal is load-bearing, not cosmetic."
  ],

  "rationale_for_docs": "This is the highest-leverage real-world robustness fix in the project: 605 of 712 real-file load failures (85%) are this single missing capability, and every mature reader closes it by rebuild-by-scan. The decision splits into a cheap correctness audit (Pass A — likely a negative result that redirects effort correctly) and the real recovery subsystem (Pass B). The load-bearing design choice is that recovery lives ENTIRELY on the strict-load error path, so §5's round-trip/minimal-diff invariant for clean files is untouched by construction; and the honest interaction with §5 is that a recovered file — having had an invalid base — must save as a disclosed full rewrite, a natural sibling of the existing R35/R58 full-rewrite-forcing rules. Recovery is non-normative reader-robustness policy grounded in universal reader behavior, disclosed and counted per fuzzy-never-sneaky and R20, bounded per R25, and fail-clean per R27."
}
```

---

## Orchestrator note (2026-08-01, at archival)

Decision 013 archived — cross-reference recovery, the #1 real-world robustness fix (605 of 712 real-file load failures = 85% are the missing xref-recovery path). Key finding: pdfce's classic-table parser is already CRLF-correct for all three §7.5.4 EOL forms; the CRLF failure correlation is OFFSET-SHIFT corruption (LF→CRLF text-mode conversion invalidating every stored byte offset incl. startxref), NOT a parser bug — so rebuild-by-scan (Pass B) carries essentially all the recovery, and the CRLF audit (Pass A) is expected to be a cheap disambiguation / negative result (do NOT hunt a phantom parser bug). Two sequenced Passes: A (EOL/CRLF audit + conditional strict fix, core-only) then B (rebuild-by-scan recovery — scans N G obj headers to synthesize xref+trailer, walks recovered /ObjStm, triggers ONLY on xref-parse failure so clean files are untouched by construction, re-checks /Encrypt after rebuild and still refuses, forces full-rewrite save + disclosure). Standing rule proposed as 'R59' but R59/R60/R61 are ALREADY TAKEN (decision 010: render-fidelity/one-canvas/inkscape) — the librarian must assign the actual next-free number (and reconcile with decision 012's five rules + the font-parse harness rule, all of which also proposed clashing numbers). At archival: Pass A dispatched (core xref audit); Pass B queued after Pass A + the in-flight font-supply build (to avoid concurrent pdfce-cli edits). Subsumes decision 007 §10 item 6 (PDF-2.0-offset-start.pdf) via Pass B's header/offset-start handling.
