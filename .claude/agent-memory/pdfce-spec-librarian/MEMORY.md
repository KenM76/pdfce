# Memory index — pdfce-spec-librarian

- [Spec source extraction toolchain](reference_spec_source_extraction.md) — how to GET a spec and get text out of it: 17 numbered routes (4a–4q), verified free URLs, paywall workarounds, errata-scan recipes. Read before any acquisition or extraction.
- [PDF_Spec corpus conventions + dispatch-shape playbook](project_corpus_state.md) — 64 numbered items, one per past dispatch. **Find the item matching your dispatch's SHAPE and read it first.** Naming, banners, grading vocabulary, negative-result evidencing, index-filing mechanics.
- [Font + spec-data licensing patterns](project_embeddable_data_licensing.md) — what may cross into pdfce's MIT tree; data-vs-document, widths-vs-shapes, availability ≠ redistribution licence.

## Where to look, by what you were asked to do

| Dispatch shape | Read |
|---|---|
| "get me spec S" / a URL 403s / text extracts wrong | extraction 4a–4q; **4d** (`r.jina.ai`), **4h** (iTeh ISO previews), **4m-i** (Wayback `if_/`), **4m-ii** (etsi.org UA gate) |
| a table extracts misaligned, split, or fused with a neighbour | extraction **4i** (caption-index-then-slice), **4c-bis** (per-page `pdfminer` layout pass) |
| a phrase/term count is going into a file | extraction **4b** (whitespace-stripped counting — a raw `grep -c` on a multi-word phrase is a lower bound) |
| **a TABLE's labels extract but its VALUES are blank** | extraction **4r** — **symbol font (`CambriaMath`), NOT a deletion**; grepping the value returns 0 hits and "the row was deleted" is the false conclusion |
| a formula, figure or annex looks empty or scrambled | extraction **4a**/**4a-bis**/**4a-sexies** (glyph x-positions, path-drawn symbols), **4c**/**4f** (the figure is a raster) |
| "is this amended? is there an erratum?" | extraction **4j**/**4k**/**4n**; corpus **63d** (three channels + a positive control) |
| **"what makes the OTHER implementation right? — here are its pixels"** | corpus **66** (whole item). **66a: THE ORACLE IS A HYPOTHESIS TOO — ask what MODE the reference viewer was in (Acrobat's `Use Overprint Preview` defaults to *Only for PDF/X files*), and name the one-minute falsification. 66d: read the clause that DEFINES the objects, not only the one that USES them. 66e: if the rule is determinate but the pixels disagree, the answer IS a ranked list of ways the rule was not REACHED.** |
| **"which of these two readings is right? — I narrowed it to one clause"** | corpus **64** (whole item). **64a: transcribe the clause's EXAMPLEs and its summary TABLE — an omitted EXAMPLE is what makes a determinate clause look ambiguous. 64e: grep the ambiguity register FIRST — the question may be a setting pdfce already shipped.** 64c: also answer whether the rule is even REACHED |
| "does feature F apply inside context C?" | corpus **64d** — three places: C's own clause, a whole-document co-occurrence count, and **C's entry-RESET list read as a CLOSED set** (absence from it = affirmative inheritance) |
| "the spec is silent on X" | corpus **62c** (an erratum can answer it), **63a** (grade the silence by counting the sentence elsewhere), **63b** (grep the feature name document-wide) |
| ingest clause C in full before a Pass is written from it | corpus **63** (whole item), **58** (submit-form precedent) |
| **"is clause C advisory or mandatory? — don't soften it either way"** | corpus **65** (whole item). **65a: SPLIT the obligation from the output — a `shall` with no measurable predicate is BOTH real and untestable, and reporting one half is how the answer gets acted on wrongly. Build an `M1`…`Mn` ladder, one row per SENTENCE. 65b: FOLLOW THE DELEGATION OUT OF PDF — the answer was in ICC.1:2010's Introduction, free at color.org, a 5-minute targeted read. 65c: an ISO-approved erratum may have DELETED the permissive sentence — search the errata by the FEATURE PHRASE, not the key name.** |
| "where can X be set / which carrier wins?" | corpus **65d** (count the CONCEPT phrase document-wide — the clause's own carrier list is a hypothesis; 50/50 hits surfaced 4 consumption sites nobody would dispatch you to), **65e** (grep for a `Restrictions on the entries in a <type>` table — one word there can kill the feature) |
| "enumerate every X" / build a census or closed set | corpus **59** (semantic sweep beats key-name sweep), **63e** (state each predicate separately) |
| verify or refute the dispatch's own recall / premise | corpus **62**, **55**, **61f** — everything a dispatch asserts is a hypothesis, including claims about pdfce's own code |
| licensing: may this text/data cross into pdfce or a public repo? | `project_embeddable_data_licensing.md`; corpus **45**, **60**; extraction **4m-iii**/**4m-iv** (check the DATA repo separately; read NOTICE, and the manual inside a bundled corpus) |
| a corpus file and another document disagree | corpus **50** (the corpus's own one-line compression is usually the ancestor — quote the row, never compress it) |
| filing mechanics / `index.md` upkeep | corpus **61h**, **63j**. **Recount `ls <subdir>/<prefix>*.md \| wc -l` before touching a count cell** — it was stale five sessions running. **Run every search recipe you add.** |

## Standing cautions

- **Bash heredocs break on spec punctuation at file size.** Use `Write` for corpus files and for any multi-line script, then run it.
- **A cross-edition table shift proven over a RANGE is a measurement with a DOMAIN, not a rule.** `−1` through Table 70, **`−2` by Table 89 and Table 145** — the offset GROWS down the document. Corpus **65f**.
- **A deleted TABLE whose rule survives verbatim in prose is a RESTATEMENT removed, not a rule removed** (2.0 deleted Table 148; §8.6.7's prose is word-for-word identical across editions and carries the opaque model alone). Say which of the two you measured. Corpus **66f**.
- **A re-dispatch of an ALREADY-ANSWERED question is still worth real work if it carries a new measurement — PROMOTE the existing entry, don't write a parallel one.** A promotion costs 4 edits and **zero recounts**; a new file costs 5 and moves count cells. Corpus **66b**, **66g**.
- **A 1→0 phrase count proves the SENTENCE was deleted, never that the RULE was.** 2.0 relocated a rule to another clause in different words while a corpus file wrote "and nothing replaces it" — and its OWN §0 cross-referenced the sibling holding the answer (4th instance of the stranded-finding failure). Before writing "nothing replaces it", grep the CONCEPT in the target edition. Corpus **65g**.
- **A clause number is not a key across editions** (1.7 §12.6.4.10 = Hide, 2.0 §12.6.4.10 = Movie). Neither is a table number. Corpus **63c**, **52**, **62h**.
- **A scope-exclusion banner is an untested claim about material its author deliberately did not read.** Corpus **58**.
- **A "no free route exists" negative has a scope: the method you tried, not the resource.** It can expire in a day. Corpus **57**; extraction **4g** vs **4h**.
- **A shipped SETTING has a scope (model / edition / device class), and a shared code path leaks it.** A register entry citing two tables jointly (“Table 148/149”) is a scope smell — one may be a real choice while the other is determinate. Corpus **64e**, **61e**, **54**.
- **A clause’s EXAMPLEs are normative-adjacent; dropping them is the accurate-but-incomplete failure.** Corpus **64a**. And the corresponding edition probe: 2.0 can WEAKEN an example by swapping one word rather than deleting the sentence — grep the discriminating TOKEN (`CalRGB`), not the sentence. Corpus **64b**.
