---
name: spec-source-extraction-toolchain
description: How to get text out of staged spec PDFs on this machine (no pdftotext/mutool/qpdf; use Python 3.11 + pypdf), plus verified free URLs for ISO 32000-1 and TIFF 6.0.
metadata:
  type: reference
---

Staged spec PDFs live in `D:\Dev\Rag-Specialized\PDF_Spec\_sources\`.

**This machine has NO `pdftotext`, `mutool`, `qpdf`, or `pdftk`.** It does have
`C:\Users\Ken\AppData\Local\Programs\Python\Python311\python.exe` with **`pypdf`**
and `pdfminer` installed. Extraction recipe that works:

1. Dump every page to one UTF-8 text file with `=== PDFPAGE n ===` markers
   (`PdfReader(src).pages[i].extract_text()`), into the session scratchpad —
   **not** into `_sources\` or the pdfce repo.
2. Locate clauses by `grep -n '^<clause> '` on that dump, then `sed -n 'a,bp'`
   the range. ISO 32000-1's clause headings sit at line-start, so this is exact.
3. Spec tables often extract **column-misaligned or split across page breaks**
   (rows appear after the *following* subclause's prose). **RE-EXTRACT WITH THE
   FULL `=== PDFPAGE n ===` DUMP BEFORE CONCLUDING A TABLE CAN'T BE READ —
   the score is now 5 for 5.** Tables 6, 11, 13 (2026-07-30) and Tables 5, 16,
   17 (2026-07-31) were all recorded as "misaligned/unreadable" on first contact
   and every one came out **row-aligned** on a second pass over the whole-document
   dump. Misalignment is an artifact of extracting a *page range*, not a property
   of the table. Reconstruct-from-self-naming-descriptions only as a last resort,
   and when a reconstruction is later verified, record the **outcome** — in all
   five cases the reconstruction had been correct, which is worth knowing but is
   *not* a licence to skip the re-extraction.

3b. **`python.exe` invoked from the Bash tool resolves a `/tmp/...` output path
   to `C:\tmp\...`, not to git-bash's `/tmp`.** Write scratch dumps to an
   explicit absolute Windows path, or `find` for the file afterwards. Cost this
   1 wasted command on 2026-07-31. Full ISO 32000-1 dump = 756 pages, ~2.1 MB,
   ~90 s.
4. For big data tables (e.g. Annex D encodings), extract **programmatically with
   a regex + a cross-check on row/column counts**, not by hand. Annex D.2 was
   validated this way: 229 rows, 149/207/216/229 codes per encoding column,
   matching the published sizes of those encodings.

4b. **The strongest cross-check is against a SIBLING table already in the
   corpus** — two independently extracted datasets that must reconcile
   arithmetically. Annex D.3 (2026-07-31): 256 codes, 24 marked undefined ⇒ 232
   defined; minus the 3 controls (TAB/LF/CR) that have Unicode values but no
   Latin-set glyph name ⇒ **229**, exactly Annex D.2's independently extracted
   `PDF` column count. Also cheap and worth doing: **assert the identity ranges
   and enumerate the divergences** rather than eyeballing (0x20–0x7E and
   0xA1–0xFF are identity with Unicode; the script found exactly two exceptions,
   `0xA0` = EURO and `0xAD` = undefined — both then confirmed against other
   clauses). And **check the value column for duplicates**: the one duplicate
   found (U+0017 at both 0x16 and 0x17) turned out to be a *source typo*, which
   is itself a finding worth recording rather than silently repairing.

4c. **Table extraction interleaves the PREVIOUS annex's continuation rows.**
   Annex D.3's rows for 0x05, 0x86–0x8A and 0x9E/0x9F absorbed fragments of
   Annex D.2's continuation block and of running headers. Detect by asserting
   the row count and looking for anomalously long "name" fields; repair by
   reading those rows directly off the source text. This is the same page-break
   artifact as memory item 3, appearing inside a *single* table rather than
   across one.

**Verified-free source URLs (all re-confirmed HTTP 200 on 2026-07-30):**

- ISO 32000-1:2008 —
  `https://opensource.adobe.com/dc-acrobat-sdk-docs/standards/pdfstandards/pdf/PDF32000_2008.pdf`
- **TIFF 6.0** (normatively referenced by ISO 32000-1 §7.4.4 LZW and §7.4.4.4
  Predictor 2, and **not listed in `LEGAL.md` §2's table**) —
  `https://www.itu.int/itudoc/itu-t/com16/tiff-fx/docs/tiff6.pdf`.
  Adobe's own TIFF6.pdf links are dead after site restructuring; ITU-T's copy of
  the TIFF-FX working documents is the stable free mirror and qualifies as
  free_primary under the open-publication-body rule.
- **Adobe Glyph List / AGLFN** — `https://raw.githubusercontent.com/adobe-type-tools/agl-aglfn/master/`
  (`glyphlist.txt`, `aglfn.txt`, `zapfdingbats.txt`, `LICENSE.md`). AGL
  *Specification* prose: `adobe-type-tools/agl-specification`'s `README.md`
  (`adobe-type-tools.github.io/agl-specification/` **404s** — use the repo).
- **ITU-T Recommendations (verified 2026-07-30, five staged).** Two-step recipe:
  scrape item IDs from `https://www.itu.int/rec/T-REC-<rec>/en` (they appear as
  `parent=T-REC-<rec>-<YYYYMM>-<X>`), then fetch
  `https://www.itu.int/rec/dologin_pub.asp?lang=e&id=<ITEM-ID>!!PDF-E&type=items`
  — that endpoint returns `application/pdf` directly, no cookie/session needed.
  **All three failure modes are per-EDITION, so try another edition before
  concluding a document is unavailable:** TIES **login-form HTML** (T.81 every
  attempt; T.800 07/2024 — but T.800 11/2015 served fine), **HTTP 500** (T.88
  08/2018 — but T.88 02/2000 served fine), and a **BIG-IP "Request Rejected"
  WAF block** on `recommendation.asp` item pages (landing pages are fine; don't
  bother with item pages). Staged OK: T.4 07/2003, T.6 11/1988, T.88 02/2000,
  T.800 11/2015.
- **T.81 (JPEG) is NOT obtainable from itu.int** — gated behind TIES login on
  every attempt, with and without browser UA/referer. Use **W3C's reference
  copy**: `https://www.w3.org/Graphics/JPEG/itu-t81.pdf` (HTTP 200,
  `application/pdf`, 1 058 883 B, SHA-256 `631031d4…768bf0`). `free_primary`
  under the same open-publication-body mirror reasoning as the ITU-hosted
  TIFF 6.0 copy.
- **Internet Archive is the working route when the first-party URL is dead —
  and it TRUNCATES SILENTLY.** Established 2026-07-31 fetching the **Adobe
  Supplement to ISO 32000, ExtensionLevel 3** (the only free AES-256 source).
  Recipe:
  1. `https://archive.org/wayback/available?url=<url-without-scheme>` → JSON with
     the closest snapshot and its timestamp. Fast and reliable.
  2. Fetch `https://web.archive.org/web/<timestamp>if_/<original-url>` — the
     **`if_`** suffix serves the raw asset, not the wrapper page.
  3. **The first attempt stopped at exactly `1 048 576` bytes (1 MiB) with
     `http=200`, and the truncated PDF still opened as a plausible 5-page
     document** — `file` reported it without complaint; only `pypdf` failed, with
     an unrelated-looking "EOF marker not found". **Always
     `tail -c 200 f.pdf | grep -q '%%EOF'` before extracting.** A single
     `curl -sL -C - -o same-file <url>` resume completed it.
  4. `archive.org/cdx/search/cdx` **504s** — don't rely on it for enumeration.
- **Dead/blocked, 2026-07-31:** `www.adobe.com/content/dam/...` PDF paths **hang**
  (no response in 120 s; `curl` exit 92 on HTTP/2, then a 2-minute timeout on
  `--http1.1`) · `opensource.adobe.com/dc-acrobat-sdk-docs/standards/pdfstandards/pdf/adobe_supplement_iso32000*.pdf`
  → **404** on 4 filename variants (only `PDF32000_2008.pdf` lives there; the
  directory listing itself 404s) · **`pdfa.org` returns HTTP 403** to both
  `WebFetch` and `curl` with a full browser UA — the PDF Association's
  `/extensions/` and `/resource/pdf-specification-archive/` indexes are **not
  machine-reachable**, so plan on `WebSearch` + Wayback instead.
- **Microsoft OpenType specification (verified 2026-08-03).** `learn.microsoft.com`
  HTML pages work with `WebFetch`: `https://learn.microsoft.com/en-us/typography/opentype/spec/<page>`
  (`os2`, `otff`, `head`, `hmtx`, `cmap`, `post`, `glyf`, `loca`, `maxp`, `name`,
  `cff`, `cff2`, …). Big pages exceed the inline cap and land in
  `C:\Users\Ken\.claude\projects\<proj>\<session>\tool-results\toolu_*.txt` —
  `sed`/`grep` that file rather than re-fetching. **The GitHub raw route is DEAD:**
  `raw.githubusercontent.com/MicrosoftDocs/typography/{live,main,<the page's own
  pinned gitcommit>}/typographydocs/opentype/spec/<page>.md` all return **404**, so
  the docs repo's `LICENSE` cannot be read ⇒ record `free_primary` per `LEGAL.md`
  §2 but attach a `NEEDS VERIFICATION` on the *redistribution* grant and hold
  quotation to sentence/table-row level (memory item 6, availability ≠ licence).
- **Vendored Rust crate sources are a legitimate, offline verification route** —
  `~/.cargo/registry/src/index.crates.io-*/​<crate>-<version>/src/`. Used
  2026-08-03 to check `subsetter 0.2.6`'s emitted table set against a decision
  record's claim. Cheap (`grep -n "Tag::" src/lib.rs`), no network, and the crate
  version is pinned by the workspace lock so the reading is reproducible. This is
  *verification of a dependency claim*, **not** sourcing a normative algorithm
  from code — that remains the thing to put to the user first (memory item 16).
- **Adobe font technical notes** live at
  `https://adobe-type-tools.github.io/font-tech-notes/pdfs/<NNNN>.<Name>.pdf`
  (e.g. `5004.AFM_Spec.pdf`). **All `partners.adobe.com` TN URLs are dead.**

**Dead, do not retry (HTTP 404 confirmed 2026-07-30):** Adobe's Core 14 AFM zip
at both `www.adobe.com/devnet/font/pdfs/Core14_AFMs.zip` and
`opensource.adobe.com/dc-acrobat-sdk-docs/.../Core14_AFMs.zip`. **There is no
live first-party Adobe download for the Core 14 AFMs** — ISO 32000-1 §9.6.2.2's
own NOTE points at a source that no longer exists. Working mirrors:
`raw.githubusercontent.com/tecnickcom/tc-font-core14-afms/main/` (bare mirror,
ships Adobe's `LICENSE`) and `apache/pdfbox`'s
`pdfbox/src/main/resources/org/apache/pdfbox/resources/afm/`.

**Technique — cross-mirror integrity check when no first-party source exists.**
Fetch the same file from two independently-maintained mirrors and compare. Raw
SHA-256 will often differ from line-ending mangling alone (PDFBox's stored blobs
have CR rewritten to LF, giving the *same byte count* but a different hash — do
not read that as tampering). Normalize first: `tr -d '\r' | sed 's/[[:space:]]*$//' | grep -v '^$'`
then hash. Record both the raw hashes and the fact that the normalized ones
matched, in the RAG file's Provenance section.

**Font/software LICENSE verification — read the file, never recall it.** A
license claim written from training-data recall was **wrong in the corpus** and
had to be retracted 2026-07-30 (URW/Nimbus: recalled as "AFPL, relicensed 2015,
dual GPL-with-exception + AGPL"; actually `AGPL-3.0-only WITH
PS-or-PDF-font-exception-20170817`, single-licensed). Recalled license facts are
plausible-sounding and specific — exactly the shape the claim-bearing-copy rule
targets. Two cheap machine-checkable verifications, both used to confirm that fix:

- **The upstream `LICENSE` file, raw.** e.g.
  `https://raw.githubusercontent.com/ArtifexSoftware/urw-base35-fonts/master/LICENSE`.
  Fetch by two independent methods (`WebFetch` + `curl`) and compare — `WebFetch`
  summarizes through a small model, so it can paraphrase a clause you need verbatim.
- **SPDX's machine-readable lists**, which carry the authoritative id *and its
  date stamp*: `https://spdx.org/licenses/licenses.json` and
  `.../exceptions.json`. Filter with Python for the id; check
  `isDeprecatedLicenseId`. The exception id's own `YYYYMMDD` suffix is the
  reliable date (`PS-or-PDF-font-exception-20170817` → 2017, settling the
  2015-vs-2017 question by itself).

Verified 2026-07-30, font-licensing only (no font bytes staged):
Artifex `urw-base35-fonts` LICENSE · `https://spdx.org/licenses/exceptions.json` ·
pdfium `LICENSE` + `core/fxge/fontdata/chromefontdata/` listing via
`https://pdfium.googlesource.com/pdfium/+/refs/heads/main/<path>` (append
`?format=TEXT` for base64 raw; the plain URL returns browsable HTML).

Re-verify URLs each session before fetching (agent hard rule 4). See
[[pdf-spec-corpus-state]] and [[pdf-spec-embeddable-data-licensing]].
