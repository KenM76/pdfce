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

3a. **CHECK `C:\tmp\iso32000_dump.txt` BEFORE RE-DUMPING.** The full 756-page
   `pypdf` dump persists across sessions (2 124 253 B, 37 491 lines, written
   2026-07-31; still present and correct 2026-08-07). Verify with
   `grep -c "=== PDFPAGE" → 756`, then go straight to
   `grep -n '^<clause> ' `. Saves the ~90 s re-dump every session. If it is ever
   missing or the page count is wrong, re-dump per item 1.

3b. **`python.exe` invoked from the Bash tool resolves a `/tmp/...` output path
   to `C:\tmp\...`, not to git-bash's `/tmp`.** Write scratch dumps to an
   explicit absolute Windows path, or `find` for the file afterwards. Cost this
   1 wasted command on 2026-07-31. Full ISO 32000-1 dump = 756 pages, ~2.1 MB,
   ~90 s.
4. For big data tables (e.g. Annex D encodings), extract **programmatically with
   a regex + a cross-check on row/column counts**, not by hand. Annex D.2 was
   validated this way: 229 rows, 149/207/216/229 codes per encoding column,
   matching the published sizes of those encodings.

4a. **A spec EQUATION extracts as a scrambled glyph run — recover it by
   CHARACTER X-POSITION with `pdfminer`.** Established 2026-08-08 on ISO 32000-1
   §11.6.5.3's `/Matte` preblend formula. `pypdf` returned
   `c' m α cm–()×+ =` (draw order, not reading order) — plausible-looking and
   unusable. The fix, two commands, no re-staging:

   ```python
   from pdfminer.high_level import extract_pages
   from pdfminer.layout import LTChar, LAParams
   for page in extract_pages(src, page_numbers=[N], laparams=LAParams()):
       chars=[]                      # recurse: LTChar leaves are nested
       def walk(o):
           for e in o:
               if isinstance(e, LTChar): chars.append(e)
               elif hasattr(e,'__iter__'): walk(e)
       walk(page)
       rows={}                       # bucket by y, then sort each row by x0
       for c in chars: rows.setdefault(round(c.y0/3)*3, []).append(c)
       for y in sorted(rows, reverse=True):
           print(''.join(c.get_text() for c in sorted(rows[y], key=lambda c:c.x0)))
   ```
   → `c'=m+α×(c–m)`, unambiguous. Same technique read §11.3.3 and all nine of
   §11.3.8's summary formulas. This is item 25's "a figure is readable as
   geometry" applied to *type* instead of *paths* — and it is the **only**
   reliable way to transcribe a normative formula. Label the result *derived
   transcription of a normative formula object* in the RAG file, and say which
   page/index it came from.

   **4a-bis. Row-bucketing is NOT enough for superscripts, fractions and
   subscripts stacked in the same equation — add a SECOND, per-glyph pass that
   prints `(x0, y0, size)`.** Established 2026-08-10 on ISO 32000-1 §8.6.5.4's
   `g(x)`. The row pass returned `36` on one line and `1084` on another; those
   are **not numbers**. The per-glyph pass resolves them instantly:
   `3` at `x=135.9, size=9.0` (a superscript: smaller font, raised baseline) is
   the exponent of `x³`, while `6` at `x=274.8, size=10.5` sits above a run of
   `-` glyphs at `x=272–279` with `2 9` beneath ⇒ the fraction `6/29`. Likewise
   `1084` is `108` (numerator, `x=130–141`) and `4` (`x=183.7`, a different
   fraction's numerator). **Heuristics that work: a fraction is
   numerator-row / a horizontal run of `-` at the same x-span / denominator-row;
   a superscript is same-ish x but SMALLER `size` and a raised `y0`; a subscript
   is smaller `size` and a lowered `y0`.** Variant script kept as `eq2.py`/
   `eq3.py` in the session scratchpad — 15 lines, filters a y-window and prints
   every glyph with coordinates.

   **4a-ter. Cross-check every transcribed formula against a property the spec
   never states.** `g(x)`'s branches are C¹-continuous at `6/29`
   (`3·(6/29)² = 108/841`); `CalRGB`'s `(1−x)/y − 1 ≡ (1−x−y)/y` is the standard
   CIE `z` relation; a 3×3 colour matrix's per-axis sums reproduce its own
   `/WhitePoint`. Each is a decisive, cheap check that the superscript /
   fraction / major-order reading was right. **A transcription with no
   independent check is not finished.**

   **4a-quater. Expect GLYPH DROPOUTS in the source's own text layer.** In
   §8.6.5.3 the third gamma exponent's `B` subscript is present on the Z row and
   absent on the X and Y rows; two `y_G` subscripts vanish from the chromaticity
   block. Verify by selecting the exact y-band and confirming no glyph exists at
   the expected x — then record it as an erratum with the reading you adopted
   and what forces it, never silently normalise it.

   Three operational gotchas, all cost time on 2026-08-08:
   - **`pdfminer`'s `page_numbers` is 0-based**; the `=== PDFPAGE n ===` marker
     in the cached `pypdf` dump is **1-based** ⇒ `page_numbers=[n-1]`.
   - **Set `PYTHONIOENCODING=utf-8`** — printing `α`/`×` to a cp1252 console
     raises `UnicodeEncodeError` *mid-loop*, after partial output, which reads
     like a data problem and is not.
   - `pdfminer` prints "contains a metadata field indicating that it should not
     allow text extraction. Ignoring this field" on `PDF32000_2008.pdf` — noise,
     not a failure; `logging.disable(logging.WARNING)` silences it.
   - Subscripts land on their **own y-row** (`αr` → `α` then `r`), so bucket
     rows loosely (`round(y/3)*3`) and expect to reassemble subscripts by eye.

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
- **PDF Association — the HOST SPLIT (verified 2026-08-09).** `pdfa.org` still
  **403**s automated fetches, but two sibling hosts serve fine (both HTTP 200,
  no auth, plain `curl` with a browser UA):
  - **`https://pdf-issues.pdfa.org/32000-2-2020/clause<NN>.html` — `<NN>` IS
    ZERO-PADDED.** `clause07.html` works; **`clause7.html` returns 404**
    (confirmed 2026-08-10; the earlier note read `clause<N>` and only looked
    right because clause 12 is two digits). Enumerate the real page set once
    with `curl -s <base>/ | grep -oE 'href="[^"]*"'` — it also reveals the
    sibling standards (`19005-4-2020`, `14289-1-2014`, `21757-1-2020`, …) and
    the `clauseAnnex*.html` / `clauseBibliography.html` pages. Strip tags but
    **preserve `<ins>`/`<del>` as `[INS]`/`[/INS]`/`[DEL]`/`[/DEL]` markers
    before the generic `re.sub(r'<[^>]+>',' ')`** — the whole value of the page
    is which side of the edit a sentence is on. Second proven use beyond
    erratum-confirmation: **proving a known ambiguity was NOT fixed in 2.0**
    (ISO 32000-1 Table 46 `/CheckSum` is self-contradictory; 2.0 changes the
    key's type and adds a NOTE but leaves both contradictory sentences ⇒
    `PERMANENT`, evidenced rather than assumed). Do this before labelling any
    ambiguity PERMANENT.
  - **`https://pdf-issues.pdfa.org/32000-2-2020/clause<N>.html`** — the public
    **errata for ISO 32000-2:2020**, per clause. Quotes 2.0 clause text in
    strike-through/insertion form ⇒ it **confirms ISO 32000-1 errata** *and* is a
    legitimate **narrow 1.7→2.0 delta source** (`free_secondary_paraphrase`).
    Strip tags with a 3-line `re.sub` and grep the flat text; `clause12.html` is
    ~135 kB → ~57 kB of text.
  - **`https://www.pdfa-inc.org/product/iso-32000-2-pdf-2-0-bundle-sponsored-access/`**
    — states ISO 32000-2:2020 + ISO/TS 32001–32005 are **$0.00 since 2023-04-05**
    under sponsored access. **Acquisition needs an account + a $0 cart checkout =
    a side effect outside the working tree ⇒ escalate to the operator, do not
    perform it.** And **zero cost ≠ redistributable** — an acquired copy is
    `user_provided_paywalled_copy`, never `free_primary`.
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
- **PLRM3 — PostScript Language Reference, 3rd ed. (staged 2026-08-10 as
  `_sources/Adobe_PLRM3_1999.pdf`, 7 771 729 B, 912 pp).** The **semantics
  authority for ISO 32000-1's type-4 operator set** (§7.10.5.1: "the semantics
  are those of the corresponding PostScript operators"), though ISO lists it only
  as **Bibliography [15] = informative**. Every first-party route is dead:
  `www.adobe.com/jp/print/postscript/pdfs/PLRM.pdf` **hangs** (curl exit 92 on
  HTTP/2, then timeout on `--http1.1` — the known adobe.com failure mode),
  `www-cdf.fnal.gov` **403**, two other mirrors **404**, and
  `archive.org/wayback/available` **429**s. **Working route: skip the availability
  API and hit `https://web.archive.org/web/2018if_/<url>` directly** — it 302s to
  the nearest snapshot (here `20200722143236`) and serves `application/pdf`.
  Worth remembering generally: **the `/web/<YYYY>if_/` form is a usable
  substitute when the availability API is rate-limited.** `%%EOF` verified.
  Operator entries are **Chapter 8 §8.2 "Operator Details"**; **Appendix B is
  *Implementation Limits*, NOT operators** — which is what makes ISO 32000-1
  §7.10.5.1's "see Appendix B … for these operators" an erratum.
- **★ TWO PLRM-class extraction artifacts, both silent, both cost time:**
  - **LIGATURES.** FrameMaker-set Adobe books store `fl`/`fi` as **U+FB02/U+FB01**,
    so **`grep floor` returns 0 hits in 912 pages** while `grep ﬂoor` finds the
    entry. Same for `ﬁle`, `closeﬁle`, `speciﬁed`, `inﬁll`. **A 0-hit result on a
    common word containing `fl`/`fi`/`ff` is a ligature artifact, not evidence of
    absence** — re-grep with the ligature before recording a NEGATIVE RESULT.
    This is the one failure mode that can turn an extraction bug into a false
    negative in the corpus.
  - **PER-PAGE FRAGMENTATION.** Some pages extract **one token per line**
    (`or\n\nbool\n\n1\n\nbool\n\n2`) while their neighbours extract normally, so a
    regex anchored on a whole stack-effect line finds 40 of 42 entries and misses
    two. Detect by "the entry head is missing but the index says the page is
    right"; repair by `' '.join(x.strip() for x in lines[a:b] if x.strip())`.
    Cheaper than the pdfminer x-position route (item 4a) and sufficient for prose.
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

---

## EMPIRICAL VERIFICATION ROUTE — render a synthetic fixture (added 2026-08-10)

For a "does the implementation actually honour clause C?" question, code-reading
plus grep gives a suspicion; **rendering settles it**, and a spec librarian can do
it read-only without touching the repo.

1. **A built CLI usually already exists** — `ls -la D:/Dev/pdfce/target/debug/pdfce-cli.exe`
   (and `target/release/`). Check its mtime against the commit under test. No
   `cargo build` needed, so no repo mutation and no wait.
2. **Hand-write the PDF in Python** into the scratchpad — catalog + pages + one
   page + one content stream + the feature's objects, then a real `xref` table
   (offsets captured while appending, `%010d 00000 n `, object 0 as
   `0000000000 65535 f `) and a `trailer`/`startxref`. ~40 lines. Keep the page
   small (`/MediaBox [0 0 200 200]`) and paint in **black on white** so a single
   pixel probe is decisive.
3. **Put a KNOWN-GOOD control in the same file.** The 8.11 test put an image AND
   a filled rectangle inside the same hidden `/OC` section: the rectangle came out
   white (suppressed, as claimed) and the image came out black (the defect). One
   file proves the mechanism works *and* localises where it does not — a fixture
   with only the failing case cannot distinguish "unimplemented" from "my fixture
   is malformed".
4. `pdfce-cli render-page <in> -o <out.png>` — and **read the result line**, not
   only the raster. It prints the disclosure counters (`oc_hidden=1`,
   `images=1`), which independently confirm the feature *fired* while the pixels
   show what it did.
5. **Probe the PNG with pure Python** (no Pillow dependency): walk the chunks for
   `IHDR`/`IDAT`, `zlib.decompress`, then un-filter row by row (filter types 0–4,
   Paeth included) and index `rows[y][x*ch:(x+1)*ch]`. ~20 lines, reusable.
   Remember **device y is flipped** from user space: user `y=10..60` on a 200-tall
   page is device row `140..190`.

Also verified this session: `pypdf` is not needed for this route at all — it is a
*source-extraction* tool; this is an *implementation-behaviour* tool. See
[[pdf-spec-corpus-state]] § "a STATUS-CORRECTION dispatch" for when to reach for it.
