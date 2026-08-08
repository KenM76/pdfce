---
name: pdf-spec-corpus-state
description: PDF_Spec RAG conventions that are NOT recorded in its own index.md — the font__* vs iso32000__s__9.* prefix split, the two-tier gap vocabulary, retraction markers, shall/should modality tabulation, and targeted codec-spec cross-reference reads.
metadata:
  type: project
---

Two conventions established for `D:\Dev\Rag-Specialized\PDF_Spec\` that are easy
to violate on a later session because they are judgement calls, not rules stated
in the agent file.

**1. `font__*` is for EXTERNAL font data only.** PDF's *font dictionaries*,
encodings, metrics and descriptors **as specified** are ISO 32000-1 clause 9 and
live in `iso32000\` as `iso32000__s__9.*.md` + `iso32000__annex__d.md`. The
`font__*` prefix holds the external files clause 9 depends on but does not
contain. Populated 2026-07-30 with 8 files: the Core 14 AFM metrics
(`font__std14_afm_licensing.md`, `font__std14_descriptors.md`,
`font__std14_widths__{helvetica,times,courier,symbol,zapfdingbats}.md`) and the
Adobe Glyph List (`font__agl.md`). Still to come: TrueType/CFF/OpenType
internals — `glyf`, `loca`, `cmap`, `post`, charstrings.
Same logic for `color__*` (spaces beyond the device spaces) vs `iso32000__s__8.6.md`.

Working rule of thumb: **"what does the standard say" → `iso32000__s__9.*`;
"what is the actual number/mapping" → `font__*`.**

**Why:** the original prefix table said `font__*` = "OpenType/TrueType/CFF
structure, encoding, cmaps"; putting PDF clause-9 material there would have made
`iso32000__s__*` non-contiguous and broken the "one targeted Glob finds the
clause" property the whole naming scheme exists for.

**How to apply:** when adding a font/colour file, ask "is this ISO 32000-1 clause
text, or an external format?" Clause text → `iso32000__s__<clause>.md`.

**2. Gap vocabulary is two-tier and greppable.** Distinguish, in every file:
- **GAP** / **SOURCING GAP** — the corpus doesn't cover it yet (fixable by work).
- **NEEDS VERIFICATION** — a claim IS stated but its citation is unconfirmed.
- **genuine spec ambiguity** — the standard itself is unclear; no amount of
  further ingestion fixes it. `index.md` keeps a dedicated table of these.
- **GAP CLOSED <date>** — left in place as a dated update footer when a gap is
  filled, so the audit trail survives.

**2b. A gap can close on a NEGATIVE result, and a gap can be PERMANENT.**
Established 2026-07-31 closing `filter__dct.md`'s TN #5116 SOURCING GAP. Three
sub-rules, all easy to get wrong:

- **A negative result closes a gap.** "The cited document does not contain X" is
  a *stronger* closure than a positive citation, because it disarms every future
  appeal to that document. Write it as the answer, not as a failure to find one.
  Canonical instance: **"invert" appears zero times in Adobe TN #5116**, the
  document ISO 32000-1 §7.4.8 footnote *a* makes normative by reference — which
  is what settles the "Adobe CMYK inversion" folklore.
- **A gap that no document can close is PERMANENT, and must be labelled so** —
  otherwise a future session schedules an ingestion that cannot succeed. The
  APP14 transform-byte 0/1/2 value table is the instance: ISO defers to TN
  #5116, TN #5116 doesn't have it, so the de-facto source (libjpeg
  `jdapimin.c` + ExifTool) is the *end state*. Permanent gaps graduate into
  `index.md`'s **spec-ambiguity table**, not its Known-gaps table.
- **Split-sourcing within one topic must be tabulated, not prose'd.** When
  layout comes from source A and semantics from source B with different
  license bases, lead the section with a fact→source→basis table. Prose blends
  them and a later reader mis-cites the weaker one.

**How to apply:** when closing a gap, always ask "*could* any document close
the residue?" If no, say so explicitly and move the row.

**Why:** an engineer acting on a "GAP" schedules a librarian session; an engineer
acting on a "spec ambiguity" has to make a product decision and diagnose it.
Collapsing the two wastes sessions and produces silent wrong behaviour.

**How to apply:** never delete a gap marker — convert it to `GAP CLOSED <date>`
plus a pointer to the file that closed it. See [[spec-source-extraction-toolchain]].

**3. Retractions get a third marker class: `CORRECTION <date>` / `AMENDMENT <date>`,
with the WRONG text retained.** Established 2026-07-30 when two claims had to be
walked back (URW font license; the Pass-1 text-ladder scope). The convention:

- Fix the claim **in place** wherever it is stated (table row, scope line), so a
  reader who greps only that row gets the right answer.
- Add a dated `CORRECTION <date>` / `AMENDMENT <date>` block that **quotes the
  superseded wording in a blockquote** and names what was wrong about it.
- `index.md` carries a "Superseded-content discipline" note warning that **a
  blockquote under one of those headings is the OLD, WRONG text**, plus a search
  recipe: `CORRECTION [0-9]{4}-|AMENDMENT [0-9]{4}-|superseded|RECLASSIFIED`.

**Why:** deleting a wrong claim guarantees a later session re-derives it from the
same faulty recall or the same stale upstream summary that produced it the first
time. Retaining it, labelled, makes the error self-defeating. Same reasoning as
`GAP CLOSED` — the audit trail *is* the value. **Risk this manages:** a retained
wrong claim is only safe if it is unmistakably marked, hence the blockquote-plus-
heading convention and the index-level warning; never leave old text inline.

**How to apply:** when an edit contradicts something already on disk, also sweep
`index.md` for *derived* statements that quietly depended on it — the ladder
rescope silently invalidated a "not blocking Pass 1" row in the Known-gaps table
that nothing in the edit brief mentioned. Same sweep applies to *verifications*:
closing `filter__ccitt.md`'s `BlackIs1` flag (2026-07-30) also required editing
`iso32000__s__8.9.5.2.md`, which cited that flag by name.

**4. `NEEDS VERIFICATION` closures use `VERIFICATION CLOSED <date>`** — the same
retained-blockquote shape as `CORRECTION`/`AMENDMENT`, plus an explicit
**"outcome"** line saying whether the recalled claim was right. Both outcomes
have occurred and both are worth recording: CCITT's three recalled Table 11
defaults were **correct**, while DCT's recalled `ColorTransform` summary was
**materially incomplete** (missed that the APP14 marker *overrides* the
dictionary entry, and that the fallback default is 1-for-3-components /
0-otherwise). Recall being right once is not evidence it can be trusted.

**6. Modality (`shall` / `should` / `can`) is load-bearing — tabulate it, never
flatten it to "the spec says".** Established 2026-07-31 verifying ISO 32000-1
§7.4.7 (JBIG2). That clause's five embedding rules mix all three modalities, and
two of the splits change what an implementation must do:

- Rule 4 splits **mid-sentence**: page-0 segments **`shall`** go to a separate
  stream, but `/JBIG2Globals` only **`should`** point at it.
- The "one page per image XObject" constraint everyone quotes as a requirement is
  a **`should`** (segment page association set to 1) — so a decoder that filters
  on `page == 1` silently blanks a file that violated only a recommendation.

**Why:** a `should` violation produces files that are *non-conforming but
common*, and a reader that treats `should` as `shall` fails them silently — the
worst failure shape. A `shall`/`should` split within one sentence is invisible to
paraphrase and survives only if the file reproduces the modal verb per rule.

**How to apply:** when a clause states embedding/conformance rules, write them as
a numbered table with a **Modality column**, quoting the modal verb. Where a
`should` has no stated reader-side recovery rule, that is a **spec-ambiguity row**
for `index.md`, not a gap.

**7. Resolve ISO 32000-1's dangling cross-references with a TARGETED read of the
staged codec spec — this does not open full codec extraction.** §7.4.7 forbids
"the optional 2-byte combination (marker) mentioned in the specification"
**without naming the bytes**; one grep of the staged T.88 for its Annex D
headings, plus reading D.3's body, produced `0xFF 0xAA` / `0xFF 0xAB` and
confirmed D.3 "Embedded organisation" as the mandated one. Cost: two commands.
**Record in `index.md` exactly which sub-clause was read**, so the codec spec's
"not yet extracted" status stays accurate rather than becoming a lie.

**How to apply:** whenever a PDF clause says "mentioned in the specification",
"described in an annex of", or similar without a number, that is a resolvable
citation, not a gap — and resolving it is in scope even in a session otherwise
scoped to the PDF side.

**8. A clause file can be audited a SECOND time along a different AXIS — record
it as an appended `## WRITE DIRECTION` section, never a rewrite.** Established
2026-07-31 auditing §7.5.4/.5/.6/.7/.8 for emission constraints (they had been
built for the read path during Pass 1). The convention:

- Append `## WRITE DIRECTION — <scope> (audit <date>)` **before** the file's
  `## Cross-references` section, opening with `**GAP CLOSED <date>.**` plus a
  one-line statement that the read-path material above was re-verified.
- **Do not rewrite the original sections.** The read-path content is still
  correct; the axis is new, not the facts.
- Bump `date:` → keep, add `updated:` to frontmatter; extend `keywords` with
  write-side terms (`emission`, `writer`, plus the specific ones) and
  `pdfce_relevance` with `pdfce-core::writer`.
- Add a **derived `iso32000__ref__*` consolidator** when the audit spans ≥4
  clause files — `iso32000__ref__writer_emission.md` collects every writer-facing
  `shall`/`shall not`/`should` into modality tables keyed `A1…G9`, so the
  engineer greps one file instead of five. Same role as
  `iso32000__ref__operator_index.md` / `__text_pipeline.md`.

**Why:** the read/write asymmetry is real and large — §7.5.4 alone gained the
exact 20-byte offset table, the three legal EOL *byte pairs*, and a `shall not`
(no comments in an xref table) **that lives in §7.5.8.4, not §7.5.4**. A
reader-path file is not evidence about the write path, and must not be assumed
to be.

**How to apply:** whenever a Pass changes *direction* (read→write, parse→emit,
validate→produce), assume every existing file on that topic is half-covered and
re-audit rather than trusting it.

**9. Scope a file explicitly in its title + a bold SCOPED banner when ingesting
part of a large clause.** Two shapes now in use, both established 2026-07-31:
`iso32000__annex__f.md` is *"DETECTION SCOPE ONLY"* (Annex F minus all of F.4's
hint tables), `iso32000__s__12.8.md` is a *"SCOPED STUB"* (§12.8's `/ByteRange`
model only). Both carry an explicit **"NOT ingested:"** enumeration and a
"do not answer X from this file" line. Third instance of the pattern after
`iso32000__s__9.7.5.md` (CMaps, DEFERRED STUB).

**Why:** partial ingestion is usually the *right* call — decision 007 asked for
detection-level linearization and the ByteRange model only — but an unlabelled
partial file reads as complete and gets cited for things it never covered. The
`NOT ingested` list is what makes the partial safe.

**9b. When a SCOPED file's scope later WIDENS, the banner must be edited in
place — `NOT ingested` → `NOW ingested (was "NOT")` + a trimmed `STILL NOT`
list.** Established 2026-07-31 widening `iso32000__s__12.8.md` from the
`/ByteRange` stub to the full DocMDP validation model. This is the **one part of
a SCOPED file that append-only discipline does not protect**: an untouched
banner actively lies about the file's own coverage, and the lie is worse than a
missing section because it *stops* a future lookup from reading further. Also
sweep the file's earlier sections for one-line summaries the new depth
supersedes and mark them `(Imprecise — superseded by …)` in place rather than
rewriting (the §12.8 stub's "P=2/P=3 permit form-fill and comment workflows
respectively" was wrong twice: P=3 is P=2 ∪ annotations, and P=2 also permits
signing + page-template instantiation).

**11. A file can be re-audited along a COVERAGE-DEPTH axis, not just a
direction axis.** Item 8 covered read→write. 2026-07-31 added stage-1→stage-2:
`iso32000__s__12.8.md` had the cryptographic half (`/ByteRange` digest) and none
of the policy half (permitted-changes / MDP). Same mechanics as item 8 — append
`## <AXIS NAME>` before `## Cross-references`, open with `**GAP CLOSED
<date>.**`, state that the existing material was re-verified and is unchanged.

**How to apply:** when a question is phrased as "X or merely Y?" (here: "valid,
or merely byte-range-intact?"), that phrasing is itself the signal that an
existing file covers Y and not X. Check before assuming the file answers it.

**12. A negative result must NOT be closed with a recalled real-world fact —
mark it `NEEDS VERIFICATION` and route it to `personal_rag\pdf`.** Established
2026-07-31. ISO 32000-1 defines **no** permitted-changes analysis for a plain
approval signature; the obvious completion ("but real validators flag altered
documents anyway") is *empirical tool behaviour*, was not verified in-session,
and belongs to `pdfce-librarian`'s corpus, not this one. Recording the silence
is the deliverable; resolving it from recall would have re-created exactly the
URW-license failure mode in a new domain.

**How to apply:** when a sourced silence has a plausible-sounding practical
answer, write the silence as the finding, mark the practical answer
`NEEDS VERIFICATION / OUT OF SCOPE`, name which corpus owns it, and — this is
the useful part — note that the *unverified* status is itself an argument for
the conservative engineering choice, not a reason to ignore it.

**10. Negative results are now a first-class deliverable class with their own
grep marker: `NEGATIVE RESULT`.** The write-direction audit produced 19 of them,
consolidated into `iso32000__ref__writer_emission.md` § D. Highest-value ones are
the *absences* a writer would otherwise burn a session searching for: no
predictor requirement for xref streams, `/Columns == sum(W)` is convention not
spec, the spec never addresses appending to an existing hybrid file, no `shall
not` forbids rewriting a signed file. Extends memory item 2b (a gap can close on
a negative result) with a searchable marker.

**13. Direction changes come in TWO shapes — decide which before starting.**
Memory item 8 (append a `## WRITE DIRECTION` section) applies when the **same
clauses** are re-read on a new axis (§7.5.4 read→write). It does **not** apply
when the opposite direction is governed by **different clauses**. Established
2026-07-31 building text *extraction* (Pass 4) against the existing text
*rendering* pipeline (Pass 1):

- Rendering is §9.6.6 + Annex D.2 + §9.7 → `iso32000__ref__text_pipeline.md`.
- Extraction is **§9.10** + Annex D.3 + §14.6/.7/.8/.9 → new files and a new
  consolidator `iso32000__ref__text_extraction.md`.
- Same input (a font dictionary, a show string), **opposite output**, and they
  give *different answers for the same font* — `/ToUnicode` is irrelevant to
  rendering and decisive for extraction.

**How to apply:** when a new Pass reverses direction, first ask "**do the same
clauses govern both directions?**" Same clauses → append an axis section.
Different clauses → **new files + a mirror consolidator, and make the two
consolidators cross-warn against each other in both directions** ("do not answer
an extraction question from this file"). An LLM that greps "text pipeline" and
lands on the rendering file will otherwise answer confidently and wrongly.

**14. A dispatch can ASK for negative results — satisfy it with EVIDENCE, not
assertion.** Established 2026-07-31; the Pass-4 dispatch explicitly requested
"largely negative results — record them" for whitespace/segmentation/bidi.
"The spec doesn't say X" is only trustworthy if the file shows **how it was
determined**. The shape that works:

- **Term-frequency evidence**: exhaustive full-text search of the staged source,
  with the **hit count and every hit's location**. Canonical instances: "word
  break" = **7 hits, all inside §14.8/§14.9** (so clause 9 never mentions words);
  "bidirectional" = **2 hits**, one of them an unrelated sense; "shaping" /
  "GSUB" / "GPOS" / "grapheme cluster" / "normalization" = **0 hits**.
- **Quote the clause that DOES exist and show its scope is narrower than
  assumed** — §14.8.2.5's spacing `shall` binds a *writer* and only inside
  Tagged PDF; UAX #9 is cited once, in a *layout* attribute.
- **Give each negative an ID** (`S1`–`S9`, `B1`–`B5`, `A1`–`A3`) so the derived
  consolidator and `index.md` triggers can cite them by number.
- **Convert the negatives into a SOURCED-vs-DERIVED table** keyed on the
  condition that flips them (here: tagged vs untagged). That table is what the
  engineer actually consumes — it says which outputs may be presented as fact.

**Why:** these silences are load-bearing product constraints, not gaps. They tell
the engineer which parts of a feature must be labelled fuzzy/heuristic under the
project's fuzzy-never-sneaky rule. An unevidenced "the spec is silent" gets
re-litigated every session; a hit-counted one does not.

**15. When a dispatch asks for material by a numbering scheme, CHECK THE STAGED
SOURCE ACTUALLY CONTAINS IT — with term-frequency evidence — BEFORE writing
anything.** Established 2026-07-31 on the §7.6 encryption build. The dispatch
asked for "Algorithm 2.A (the R6 hash — the SHA-256/384/512 iterated
construction, **verbatim step by step**)", "Algorithms 8/9/10/11/12/13", `/OE`,
`/UE`, `AESV3`. **None of it exists in ISO 32000-1:2008.** Measured counts over
the 756-page staged source: `AESV3` **0**, `SHA-256` **0**, `revision 5` **0**,
`revision 6` **0**, `Algorithm 2.A`/`2.B`/`8`/`9`/`13` **0 each**; the single
`/OE` hit is the **glyph name** in Annex D.2 and all three `Perms` hits are the
**catalog** §12.8.4 key, not the encryption-dictionary one.

Three sub-rules:

- **A dispatch's clause/algorithm numbering is a *hypothesis*, not a citation.**
  ISO 32000-1's own §7.6.2 warns that its algorithms are "uniquely numbered …
  in a manner that maintains compatibility with previous documentation" — so
  **three** schemes are in circulation for the same algorithms (ISO 32000-1 `N`
  ≡ *PDF Reference 1.7* / Adobe supplement `3.N` ≡ ISO 32000-2 `N`/`N.A`/`2.B`).
  Build the mapping table early and mark the column you cannot verify
  `NEEDS VERIFICATION`.
- **The absence is itself the headline deliverable** — lead the file with it
  (`§0 READ THIS FIRST`), not with a caveat buried in Gotchas. Extends memory
  item 14 (evidencing negatives) to the case where the negative is about *the
  source*, not *the subject*.
- **When part of a requested topic turns out to live in a differently-licensed
  document, SPLIT IT INTO A SIBLING PREFIX** rather than mixing licence bases in
  one file. `iso32000__s__7.6.*` = ISO 32000-1, `free_primary`, quotable;
  `security__aes256_r5_r6.md` = Adobe supplement, `free_secondary_paraphrase`,
  paraphrase-only. **"Is it in ISO 32000-1?" is the whole test** — the same
  shape as the `font__*` vs `iso32000__s__9.*` split in item 1 and `color__*`
  vs §8.6, now applied on a **licensing** axis rather than a *content-origin*
  axis. See [[pdf-spec-embeddable-data-licensing]] item 6.

**16. REFUSING to write an algorithm is a valid deliverable — even when the
dispatch explicitly asked for it verbatim.** ISO 32000-2's `/R 6` Algorithm 2.B
was requested and is **not obtainable**: ISO 32000-2 paywalled, no public Adobe
ExtensionLevel 8 document locatable, `pdfa.org` 403. Recall could supply
plausible, specific detail (round counts, the SHA-384/512 selector, the inner
AES-128-CBC step) — which is **exactly** the shape of the URW-licence claim that
had to be retracted 2026-07-30. What was written instead:

- the gap, with **every acquisition route tried and its failure mode**;
- the **product consequence**, stated bluntly (`/R 6` is what Acrobat X+ writes,
  so this is the *common* AES-256 case; encrypt-on-save AES-256 is currently
  buildable only at the security-weakened `/R 5`);
- three named closure routes, the third of which — deriving the algorithm from
  ≥3 permissively-licensed implementations — is **legitimate under the licensing
  table's "free secondary sources" rule but must be put to the user first**,
  because it would be the corpus's first normative algorithm sourced from *code*
  rather than a *document*.

**How to apply:** when a dispatch asks for X verbatim and X is unobtainable, the
deliverable is the enumerated failure + the consequence + the routes. Never
close the hole with recall to make the deliverable look complete.

**5. Filter files carry an "Implementation reference (pdfce decision NNN)" line
naming the chosen crate** — but crate *capabilities* are **never asserted** in
this corpus, only listed as things to verify against the crate's own docs. The
corpus sources specs, not dependencies. Current mapping (decision 005):
`zune-jpeg` / `weezl` / `hayro-ccitt` / `hayro-jbig2` / `hayro-jpeg2000`.

**15. `index.md` can be edited by ANOTHER spec-librarian session concurrently —
re-read every region before editing and anchor on unique strings.** Established
2026-07-31 building §12.5 (annotations, Pass 6.0) **while a parallel §7.6
encryption session was live-editing the same `index.md`**. Every Edit after the
first returned "the file had been modified on disk since you last read it." The
edits still applied because each `old_string` was unique, but a stale
line-number-based or long-context-spanning edit would have failed or clobbered
the other session's rows. Concretely: the prefix-table `iso32000__s__*` count and
the total-file count both moved (52→56 under me but the base had already jumped
to 88 from the encryption build I never saw in my first read). **How to apply:**
grep for section anchors immediately before each index edit; never reuse a
line number across two edits; make each edit target a short unique substring;
and recompute counts from the *current* on-disk values, not the values from your
first read.

**17. A WRITE-DIRECTION audit of clause C must trace C's emitted artifact to its
CONSUMER clause(s) — a binding write-side constraint can be invisible from C
alone.** Established 2026-07-31 auditing §8.10 (form XObjects) for authoring an
`/AP` appearance stream (Pass 6.1). §8.10 (the *producer/container* clause)
**tolerates a zero-area `/BBox`** — Do step c just clips to nothing, "paint
nothing," legal, not an error. But §12.5.5 (the *consumer* clause — the annotation
placement algorithm that receives an `/AP` `/BBox`) makes that same zero-area box
**undefined**: step b's fit-to-`Rect` divides by the box extent ⇒ matrix A
singular ⇒ NEGATIVE RESULT. So the real write rule — **"an `/AP` `/BBox` shall
have strictly positive width AND height"** (WF4 in `iso32000__s__8.10.md`) — does
**not** exist anywhere in §8.10; it emerges only when you follow the emitted
artifact downstream. A read-path re-audit that only re-reads the producer clause
misses it.

**How to apply:** when auditing clause C for emission, list every clause that
*consumes* what C emits (here: appearance stream → §12.5.5 placement → §12.5.3
NoZoom/NoRotate → clause 11 compositing) and check each for constraints tighter
than C's own tolerances. Record the derived constraint in C's WRITE DIRECTION
section with a cross-ref to the consumer clause's negative-result row, and make
the cross-ref bidirectional (the consumer file gets a "this is the read-side face
of C's write rule" pointer). Extends memory item 8: same-clause/both-axes still
applies, but the *content* of the write axis may be sourced from a neighbour.

**18. A direction-change dispatch can be BOTH item-8 (append axis section) AND
item-13 (new file) IN ONE BUILD — and a dispatch's FLAG-BIT VALUES are hypotheses,
not just its clause numbers.** Established 2026-07-31 on the Pass-6.2 text-bearing
GENERATION build. Two sub-findings:

- **Split the build by clause, not by direction.** The GENERATION axis touched the
  SAME subtype clauses already covered on the DISPLAY axis (§12.5.6.4/.6/.12) →
  appended `## GENERATION DIRECTION` to `iso32000__s__12.5.6.md` (item 8). But the
  bulk of the NEW normative machinery (`/DA` grammar, `/Q`, auto-size, `/Tx BMC…EMC`)
  lives in a DIFFERENT clause (§12.7.3.3) not yet in the corpus → **new file**
  `iso32000__s__12.7.3.3.md` (item 13). One dispatch, both mechanisms. **How to
  decide:** ask per-clause "is THIS clause already in the corpus?" — a covered
  clause gets an appended axis section, an uncovered clause gets a new file, even
  within a single build. (Also appended a `## WRITE DIRECTION` to `iso32000__s__9.6.md`
  for the standard-14-dict authoring rule — a THIRD, tangential same-clause axis.)
- **Extends item 15 to flag values.** The dispatch said "comb fields /Ff 24"; Comb
  is Table 228 **bit 25 ⇒ value 16777216** (bit 24 is `DoNotScroll`). And the
  dispatch's "MUST set a font+size via Tf AND a colour" was wrong: §12.7.3.3's
  `shall`-minimum is **`Tf` only** (colour defaults to `0 g`). A dispatch's stated
  bit positions, flag values, and "must include X" assertions are all hypotheses to
  verify against the source verbatim — same discipline as clause/algorithm numbers.

**How to apply:** on any GENERATION/authoring dispatch, expect a mix — one new
clause file for the algorithm + appended axis sections on the subtype/font files it
reuses. And re-derive every flag value from the source's bit-position + the
1-based `2^(bit−1)` rule; never transcribe a value the dispatch supplied.

**19. BEHAVIORAL negative results have two extra shapes beyond "the source lacks
X" (item 15) and "no document can close it" (item 2b): the HOLLOW SHALL and the
NO-CLAUSE-AT-ALL tool-verb.** Established 2026-07-31 on the Pass-7 §12.7 forms
build (NF1–NF4 in `iso32000__s__12.7.2.md`):

- **Hollow `shall`** — the spec states an imperative but defers ALL semantics to a
  non-normative external document, so a conformant reader can decline the behaviour.
  §12.6.4.16: a reader "**shall execute**" form JavaScript, but ISO 32000-1 defines
  no JS language/API/security model (deferred to Adobe/Mozilla Bibliography docs).
  ⇒ **recognize-and-disclose-NEVER-execute is spec-conformant** — there is no
  normative behaviour to conform to. The deliverable quotes the `shall`, then shows
  it is unbacked. Same move applies to any "shall <verb>" whose object is defined
  only in a referenced non-ISO spec.
- **No-clause-at-all tool-verb** — a term every PDF tool ships (here **flatten**)
  that has ZERO normative clause. The deliverable is NOT "GAP" (nothing to ingest);
  it is a NEGATIVE RESULT naming which *other* normative facts CONSTRAIN the
  convention (flatten must reproduce §12.5.5 placement when it inlines an `/AP`).
  Check the term's own frequency in the source to prove the absence before asserting
  it. Extends item 16 (refusing to write) — here there is nothing to refuse; the
  operation is real but wholly a product decision.
- **`shall`-vs-`may` on a producer-set flag:** `/NeedAppearances` (Table 218) is
  *descriptive* — no imperative attaches to a reader honouring it — so honouring it
  is effectively a `may`, and a regenerate-on-operator-request policy (pdfce R51) is
  conformant. Contrast the ONE place regeneration IS a `shall`: RichText fields,
  "entire annotation appearance **shall** be regenerated each time the value is
  changed." Tabulate WHICH regime carries the modal verb (item 6 discipline applied
  to a behaviour, not an embedding rule).

**Also confirmed (validated approach, per feedback-memory "record successes"):**
item-18's flag-value re-derivation paid off but this time EVERY dispatched bit
value was already correct — `Pushbutton` 17=65536 / `Radio` 16=32768 /
`Multiline` 13=4096 / `Comb` 25=16777216 / `Combo` 18=131072 all matched Tables
226/228/230 verbatim. Re-deriving from `2^(bit−1)` cost nothing and produced a
"DISPATCH CONFIRMED (no off-by-one)" line that is itself useful provenance. Do it
every time regardless of whether the previous dispatch had errors.

**20. A DESTRUCTIVE-OP dispatch (redaction, Pass 8) has a distinct shape: the
spec judges the RESULT, not the METHOD — split pure-spec clause file from a DERIVED
removal-mechanics `ref` file, and expect the op to INVERT existing invariants.**
Established 2026-07-31 building §12.5.6.23 (Redact). Four sub-findings:

- **OUTCOME-BOUND / METHOD-DEFERRED `shall` — a fourth behavioral-negative shape
  beyond item 19's three.** The dispatch's key question ("does the spec define the
  APPLY algorithm?") has a *refined* answer that is NOT "only the mark": ISO 32000-1
  §12.5.6.23 defines the /Redact MARK **and imposes four `shall` OUTCOME constraints
  on apply** ("remove all traces", "image data shall be destroyed"/not clip-or-mask,
  remove the annots, "diligent … XFA/XMP") — but **specifies NO procedure** ("this
  phase is **application-specific**"). Contrast item 19's HOLLOW SHALL (§12.6.4.16 JS:
  even the outcome is unbacked ⇒ decline-conformant). Here the `shall`s ARE binding
  (on the *result*, an acceptance test), only the *how* is deferred to the
  implementation. Write it as "spec gives you the JUDGE, not the METHOD"; the pdfce
  procedure is policy grounded in the outcome `shall`s + the product rule (R35).
  Evidence it the item-14 way: `redact`/`redaction` = **28 hits, ALL** in one
  subtype-list row + one clause; zero in any processing/annex ⇒ single locus, no
  hidden algorithm.
- **Pure-spec clause file vs DERIVED mechanics `ref` file.** The removal *mechanics*
  (how to physically delete from an ObjStm, slice a content stream, re-encode an
  image) are cross-clause engineering the standard omits ⇒ they go in a derived
  `iso32000__ref__redaction_removal.md` (spans §7.5.7/.8 + §9.4/§8.2 + §8.9 + §7.5.6,
  ≥4 files ⇒ consolidator per item 8), NOT in the clause file. Keep the clause file
  (§12.5.6.23) pure normative (Table 192 verbatim, the outcome `shall`s, the negative
  result). Label everything in the `ref` file "derived — NOT normative".
- **A destructive op INVERTS the corpus's own invariants — record the inversion as
  the headline.** Redaction inverts TWO: (1) minimal-diff/round-trip (R34 — every
  other op preserves untouched bytes; redaction's correctness *requires* destroying
  them, R35); (2) incremental-save-is-safe (§7.5.6 makes append signature-SAFE by
  leaving prior revisions — which makes it redaction-UNSAFE ⇒ forced full rewrite).
  Plus the pdfce-architecture trap: `save_full` re-emits `/ObjStm` verbatim ⇒ a
  logically-removed compressed object SURVIVES = security failure ⇒ container
  decomposition mandatory. The most valuable output was tracing the spec `shall`
  ("remove all traces") to pdfce's *own save model* and showing a spec-correct
  region-redact still fails on this machine. (Same "trace the artifact to its
  consumer" discipline as item 17, applied to a save-model consequence.)
- **A "what OTHER carriers can hold X" CARRIER-SWEEP is a first-class deliverable**
  — a scope-defining table (region-redact misses `/Info`/XMP/XFA/`/ActualText`/OCG
  layers/prior revisions/ObjStm survivors/overlapping annots). Half the rows point at
  GAP files (`xmp__*`=0, §7.11, §8.11, §12.7.8) ⇒ the honest verdict is
  "detect-and-disclose until built", never silent. And **a Table-N cross-reference
  inside a clause is a hypothesis like a clause/flag number (extends items 15/18):**
  Table 192's QuadPoints cites "Table 175" but Table 175 is *line* annotations —
  the format meant is Table 179. Verify cross-ref table numbers against the source;
  found a genuine ISO 32000-1 erratum by doing so.

**21. A dispatch's asserted SPEC GUARANTEE/RULE is a hypothesis too — and when it
doesn't exist, the negative result targets the dispatch's PREMISE.** Established
2026-07-31 on the Pass-12.M2 measurement build. The dispatch stated as fact "the rule
that a conforming writer preserves /PieceInfo it doesn't own across edits (the
round-trip-survival property the beta depends on)." **That rule does not exist in ISO
32000-1** (5-hit term-frequency proof; §14.5 NOTE 1's only semantics is "may be
ignored by general-purpose conforming readers" — the OPPOSITE of a preservation
guarantee; a whole-document search for `shall preserve`/`discard`/preservation-of-
unknown-keys found nothing tied to foreign dictionary entries). Extends items 15/18/20
(clause numbers, flag values, Table-N cross-refs are hypotheses) to a **named
normative RULE the dispatch relies on**. Three sub-rules:

- **Verify the rule EXISTS before building on it.** A dispatch that says "the spec
  says X preserves Y" is asserting a citation; grep for it. Absence is the headline.
- **When a relied-on guarantee is absent, split the honest completion by WHO
  guarantees it.** The beta's sidecar surviving *pdfce's own save* is real — but via
  **pdfce invariant R34 (minimal-diff), NOT the spec clause**. Surviving a *foreign
  editor's* save is **empirical, not spec-guaranteed** → route to `personal_rag\pdf`,
  mark `NEEDS VERIFICATION` (same discipline as item 12). Never let "pdfce preserves
  it" silently stand in for "the spec guarantees it."
- **The MIRROR-INTO-READER-VISIBLE-STRUCTURE interop move.** When a private carrier
  (`/PieceInfo`) has no cross-tool survival guarantee, recommend ALSO encoding the
  load-bearing datum into a spec-honored reader-visible structure (`/Measure` §12.9)
  so interop survives even if the private carrier is dropped. A design recommendation
  the corpus can make because it's grounded in two clauses' coexistence, not recall.

**How to apply:** on any dispatch whose premise is "the spec guarantees behavior B",
treat B as a citation to verify. If B is real, cite it. If B is absent, the negative
result IS the deliverable — and separate pdfce-invariant-provided behavior from
spec-provided behavior from empirical-tool behavior, three distinct guarantors.

**22. An INVERSE/RE-ENCODE dispatch (write-side of a decode/extract pass) — invert
the font's OWN forward chain, refute the "invert the extract map" premise, and make
the REFUSE-TRIGGER TABLE the deliverable.** Established 2026-07-31 on the Pass-14.1
in-place-text-edit (REPLACE) build (`iso32000__ref__inverse_encoding.md` +
`iso32000__ref__text_edit_surgery.md`). Extends item 13 (opposite-direction = new
files) and item 21 (dispatch premise is a hypothesis) with three edit-specific moves:

- **Invert the forward chain, NOT an abstract reverse map.** To go Unicode→code for a
  simple font, invert the font's own resolved `/Encoding` table `E[code]→name→AGL_forward→U`
  (build `reverse[U]=[codes]`), NOT a standalone Unicode→glyph-name reverse-AGL lookup.
  The font's `E` already fixes which name/Unicode each code carries, so inverting it
  sidesteps reverse-AGL ambiguity entirely. General principle: to reverse a multi-stage
  forward pipeline, enumerate the forward domain and index the outputs — don't build an
  independent reverse of each stage.
- **The load-bearing NEGATIVE targets the dispatch's IMPLICIT method-premise.** The
  obvious way to re-encode is to invert the decode ladder's first rung (`/ToUnicode`).
  Refute it explicitly: `/ToUnicode` is not injective (2 codes→1 U), one-to-many forward
  (1 code→ligature string), partial (presence≠coverage, §9.10 N2/N4), and carries no
  rendering authority — so re-encode from `/Encoding`, and a font relating codes to chars
  ONLY via `/ToUnicode` (symbolic/composite subset) has NO well-defined inverse ⇒ REFUSE.
  This is the write-side face of an existing extraction negative result; put it as a
  `## INVERSE DIRECTION` section in the decode-ladder file (§9.10) AND full in the new ref.
- **Under a REFUSE-AND-DISCLOSE product posture, the enumerated trigger table IS the
  deliverable** (analogue of item 16's AP-vs-fallback binary map). Give each trigger an ID
  (`R-INV-1..8`), a hard-vs-soft posture, and the precise failing condition; the engineer
  consumes the table to know when to emit nothing vs a disclosed heuristic choice. Pair
  with a QUEUE STUB (`font__subsetting_ffc_queue.md`) for the deferred pass (FF-C
  subsetting) that LIFTS specific refuse triggers — stub the spec surface + clause
  pointers only, mark "NOT needed for Pass 14.1–14.3", so it's ready without being built.
  Also: an in-place operand REPLACE is minimal-diff/incremental-safe (R34) — it does NOT
  invert minimal-diff the way a destructive op does (contrast item 20 redaction).

**23. An APPEND/ORIGINATION dispatch (add NEW content, Pass 16.0 / FF-D) — the
load-bearing normative HOOK can live in a clause the dispatch did NOT name; and
"byte-identical" is achieved by editing a REFERENCE, not the referand.** Established
2026-08-01 building the add-new-page-text APPEND recipe (decision 016). Five clauses
were dispatched (§7.7.3.3 /Contents array, §7.8.3 resources, §9.6.2.2 Std-14, §9.4
text objects, §9.10 encode) and ALL were already covered on the read path ⇒ the build
was **item-8/item-13 in its purest form: 1 new DERIVED consolidator
(`iso32000__ref__page_content_append.md`) + 5 same-clause axis sections
(`## APPEND DIRECTION` / `## ORIGINATION DIRECTION`)**. Because NO dispatched clause
was uncovered, the only new FILE was the consolidator, never a clause file (contrast
item 20, where the new file spanned an uncovered clause). Four sub-findings:

- **The binding rule was in §8.4.2, a clause the dispatch never mentioned.** The
  dispatch asked for the append mechanism (§7.7.3.3) and the q/Q caveat as prose. The
  actual normative hook — *"Occurrences of the `q` and `Q` operators shall be balanced
  within a given content stream (or within the sequence of streams specified in a page
  dictionary's `Contents` array)"* — was already sitting verbatim in
  `iso32000__s__8.4.md` §8.4.2 from the Pass-1 build. Extends item 17 ("trace the
  artifact to its CONSUMER clause"): here, **trace the operation to the INVARIANT
  clause** — grep sibling clause files for the `shall` that governs the mechanism
  before writing it as a derived caveat. The self-balanced-`q/Q` requirement is not a
  hygiene recommendation; it is §8.4.2 applied to the array.
- **Byte-identical via reference-edit.** Single→array append changes only the page
  dict's `/Contents` VALUE (`R_orig` ⇒ `[R_orig R_new]`); the stream object is never
  re-emitted ⇒ byte-identical (R32/R46) falls out mechanically. The general shape:
  "keep X byte-identical while adding to its role" = wrap X in a container that
  references it, edit the container, never X. Append at END ⇒ new content paints on
  top (§8.2 painters model).
- **Graphics state is initialized ONCE at page start, NOT between /Contents-array
  elements** (§8.4 Table 52). So the appended run inherits the prior stream's
  colour/text-state/CTM and **shall** self-set `Tf` (no default, §9.3.1) + explicit
  colour (black is only the page-initial default, not guaranteed) + `Tm`, and
  self-wrap in `q…Q`. The un-resettable-CTM hazard (no operator sets CTM=identity;
  `cm` only concatenates) is a real but rare disclose/accept limit → `personal_rag\pdf`.
- **The /Resources inheritance trap** (§7.7.3.4 + §7.8.3): if a page OMITS `/Resources`
  it inherits an ancestor `/Pages` node's dict, SHARED by sibling pages. Adding a
  `/Font` must NOT mutate the shared dict (pollutes siblings, breaks their minimal-diff)
  — give THIS page its own `/Resources` that references the same subdict objects + a
  merged `/Font`. And the PDF-2.0 `/Widths`-advisability question was answerable
  WITHOUT a 2.0 source: recommend the full form from the 1.5 free-primary
  `should`-deprecation + "pdfce already owns `fontdata::std14_*` ⇒ free/self-contained/
  forward-safe", explicitly independent of the unverifiable 2.0 clause (item 15/21
  discipline — do not cite a paywalled clause to justify a recommendation that stands
  without it).

**24. An EMBEDDING/REDISTRIBUTION dispatch (FF-C, Pass 21.0) — four moves that are
new, on top of items 15/18/20/21/23.** Established 2026-08-03 building the
font-embedding recipe against decision 021.

- **A DEPENDENCY CRATE's behaviour asserted in a decision record is a hypothesis
  too — read the vendored source.** Extends items 15/18/21 (clause numbers, flag
  values, named rules) to **third-party code**. Decision 021 §3.4 listed
  `subsetter`'s emitted table set as "GLYF+LOCA (or CFF), HEAD, HMTX, MAXP, NAME,
  POST"; the crate **also** emits `HHEA` (written inside `hmtx::subset`, invisible
  in the `_subset()` call list) and `CVT`/`FPGM`/`PREP`. That mattered because ISO
  32000-1 §9.9 has a `shall`-if-present list containing exactly those tables — the
  decision's list read like a conformance violation and the real output is
  conformant. Route: `~/.cargo/registry/src/index.crates.io-*/​<crate>-<ver>/src/`,
  already vendored, no network. **Grep the call-site list AND the per-table module**
  — a table can be pushed by a sibling's subsetter.
- **When a dispatch frames a dependency's behaviour as a LIMITATION TO WORK
  AROUND, check whether the spec MANDATES it.** `subsetter` strips `cmap`;
  decision 021 treated that as a crate constraint forcing `/Type0`. ISO 32000-1
  §9.9 actually says the `cmap` table "**shall not be present**" under a CIDFont
  dictionary — the crate is *complying*, not limiting. And §9.9 separately carries
  a **`shall` on conforming writers** to use `/Type0`+`Identity-H` for OpenType
  `glyf` programs. The reframe ("mandated, not tolerated") is worth more than the
  fact, because it removes a standing "should we work around this?" question.
- **A CONSUMER-OBLIGATION question has three possible answers, and "the spec is
  silent" is the least likely.** The dispatch asked whether the OpenType spec says
  how a consumer *should behave* vs merely what bits *mean*, and warned not to
  blur them. It states **three explicit `must`s** in `fsType`'s Comments field —
  plus a **document-level** obligation buried in the value-4 prose ("Documents
  containing Preview & Print fonts must be opened read-only"). Separately, **ISO
  32000-1 §9.9's opening paragraph is a licensing rule** ("embedded font programs
  **shall** be used only to view and print the document"; new text needs "a
  licensed copy … not a copy extracted from the PDF file") that nobody reads
  because the clause is filed under *Embedded Font Programs* and is normally
  cited only for Table 126. **When a topic is licensing-adjacent, read the whole
  clause, not the table it is famous for.** Deliverable shape that worked: quote
  the obligations, tabulate which pdfce *action* each one bears on, and state
  explicitly that the policy response remains the operator's — the RAG supplies
  the text, not the choice.
- **A CROSS-SPEC BRIDGE GAP is a PERMANENT spec-ambiguity, not a corpus gap**
  (extends item 2b). ISO 32000-1 names no permission field; the OpenType spec
  never mentions PDF. Neither says how a `fsType` value maps to a PDF embedding
  decision. No ingestion closes it ⇒ spec-ambiguity table, and it is *the reason*
  the product question is a policy call. Same shape as the APP14 transform-byte
  row, but between two *live* standards rather than a dead reference.

**Also this build:** a **REWRITE-not-extend** dispatch is item-3's retraction
discipline at whole-file scale — keep the path (other docs cite it), retain the
wrong mechanism in a `CORRECTION <date>` blockquote, **and sweep for the derived
files that inherited the error** (`iso32000__ref__inverse_encoding.md`'s
"FF-C lifts R-INV-1/2/4" rows needed their own `AMENDMENT` block; the index's
historical build note needed an inline bracket). And when a dispatch supplies a
clause list, expect ~2 in 6 to be wrong: `/CIDSet` was dispatched as §9.7.4.2 (it
is §9.8.3 Table 124) and the subset tag as §9.8.1 (it is §9.6.4).

**25. A FIGURE is normative content and can be READ AS GEOMETRY — extract the
source page's content stream and reconstruct the paths.** Established 2026-08-04
on the §12.5.6.7 offset-dimension build. `/LLE`'s entire meaning hangs on the
phrase **"the line proper"**, which occurs **exactly once in 756 pages and is
never defined**; the prose admits two opposite geometries and the only
disambiguator the standard offers is "as shown in Figure 60". Spec figures are
**vector art, not raster** ⇒ `pypdf` `page.get_contents().get_data()`, walk the
`cm` stack, collect `m`/`l`/`re` segments, transform to page space. Cost: two
commands. Output: the leader stroke splits **47.5 : 9.5 = `LL` : `LLE` at one
consistent 0.95 figure scale**; the rival reading requires **two** scales (1.14
and 0.95) ⇒ **refuted arithmetically, not by plausibility.** Three sub-rules:

- **Report the method and the raw numbers**, labelled *derived measurement of a
  normative illustration*, never as spec prose. It is evidence of the same class
  as item 14's term-frequency counts, applied to a picture.
- **State what the figure CANNOT settle.** Figure 60 cannot corroborate the `LL`
  sign rule because `/L`'s traversal order is unobservable in a drawing, and it
  does **not** render `LLO` at all (stroke starts at parameter 0). Saying so
  keeps the two facts it *does* settle trustworthy. A figure that "looks like it
  confirms everything" is being over-read.
- **Cross-check a second invariant while you have the coordinates.** `|/L| =
  299.8` vs `|line proper| = 299.9` proved the offset is a **rigid translation**
  ⇒ the measured length is identical either way ⇒ the dispatch's stated fear
  ("getting it backwards would misreport every dimension") is **structurally
  impossible on that axis**. Refuting the *severity* of a dispatch's worry is as
  valuable as answering its question.

**Also this build — a GEOMETRY-AUTHORING dispatch's shape:**

- **"Which array holds the measured points?" is answered by ONE conditional
  sentence, and that sentence is the whole file.** Table 175's `L` row: "**If the
  `LL` entry is present, this value shall represent the endpoints of the leader
  lines rather than the endpoints of the line itself.**" Key semantics that
  *change based on a sibling key's presence* are the highest-value thing to quote
  verbatim — paraphrase destroys the conditional. Sweep any authoring table for
  rows whose meaning is keyed on another entry (`meaningful only if Cap is true`
  ×2 here; `Required if LLE is present` ×1) and render them as a **dependency
  graph**, not prose.
- **The spec often ALREADY NAMES the thing the code invented.** pdfce's
  hand-rolled 4 pt perpendicular tick is the standard `Butt` line ending ("a
  short line at the endpoint perpendicular to the line itself"); its arrowheads
  are the `OpenArrow`/`ClosedArrow`/`ROpenArrow`/`RClosedArrow` family. **But the
  spec specifies no size/length/angle for ANY of the 10 endings**, so the
  invented dimensions are *conformant* — the finding is a **naming/interop
  opportunity, not a bug**. Report those separately from real contradictions; a
  contradiction table that grades everything "wrong" gets ignored.
- **A relative-only definition is a GAP.** `ROpenArrow` = "in the reverse
  direction **from `OpenArrow`**" while `OpenArrow`'s own direction is never
  stated ⇒ outward-vs-inward cannot be chosen mechanically. Watch for definitions
  anchored only to a sibling that is itself unanchored.
- **A domain's vocabulary can be entirely absent from the standard that supports
  it.** `dimension line` / `extension line` / `witness line` = **0 hits each** in
  ISO 32000-1, yet the feature is fully expressible. Naming that absence stops a
  future session grepping for the wrong words and concluding the corpus has a gap.
- **A dispatch can be RIGHT.** Every key name, the table number, and both
  non-negativity constraints the dispatch asserted were confirmed verbatim; the
  `/LLO`-is-the-CAD-gap hypothesis was confirmed too. Re-deriving cost nothing and
  the "DISPATCH CONFIRMED" line is useful provenance (same posture as item 19's
  flag-value note). Record confirmations, not just corrections.

**26. A COLLISION/RE-INDEX dispatch — the clause is ALREADY fully covered and the
finding lives in the JOIN of sibling tables, not in any one of them.** Established
2026-08-05 on the Pass-37.2 `/Ff` field-flags build. Tables 221/226/228/230 were
already verbatim on disk (from the Pass-7 build) and **nothing was wrong with
them**; the hazard (bit 26 = `RadiosInUnison` on `/Btn` vs `RichText` on `/Tx`)
is invisible when each table is read on its own axis. Deliverable = a derived
`iso32000__ref__*` consolidator that **re-indexes the existing verbatim material
on a DIFFERENT KEY** (here: by bit position instead of by field type), plus a
short axis section in each source file pointing at it. Extends item 8 (new AXIS)
with a third shape: not read→write, not stage-1→stage-2, but **same facts, new
index key**. Five sub-rules:

- **When a dispatch names a hazard, ANSWER THE GENERAL QUESTION EXHAUSTIVELY, not
  the instance.** The dispatch asked "is bit 26 the only multi-meaning position —
  check rather than assume." Enumerating the union of all four tables by bit
  position both **confirmed** it and surfaced the near-miss the dispatch didn't
  know about: **bit 23 `DoNotSpellCheck` is shared by Tables 228 and 230 with the
  same name and meaning, but only Table 230 gates it** ("shall not be used unless
  the `Combo` and `Edit` flags are both set"). Same-name-different-precondition is
  a *third* category between "unique" and "collision" and is worth naming.
- **Re-indexing an already-ingested table family FINDS ERRORS IN THE DERIVED
  PROSE around it.** `iso32000__s__12.7.3.md` said "bits 4–14, 18–20, 22–27 are
  type-specific" — it omitted 15/16/17 and 21 and wrongly included the reserved
  4–12. Nothing in the verbatim tables was wrong; the *summary sentence* was. Any
  hand-written "bits X–Y" range next to a verbatim table is a computed claim and
  should be re-derived from the table, not read.
- **A KEY NAME can be overloaded across dictionaries, not just a bit within one.**
  `Ff` is FOUR unrelated flag words in ISO 32000-1: field dict (Tables
  221/226/228/230), `/SV` seed value (Table 234, bits 1–7 = Filter/SubFilter/V/
  Reasons/LegalAttestation/AddRevInfo/DigestMethod), `/SV /Cert` (Table 235, bits
  1–7 = Subject/Issuer/OID/SubjectDN/Reserved/KeyUsage/URL), and FDF field (Table
  246). **Three of them nest inside a single `/Sig` field dictionary.** When
  building any key-centric file, `grep -n "^<Key> \|<Key> integer"` the whole
  source for other definitions of the same key before declaring what it means.
- **The spec sometimes ENUMERATES its own table family — quote it as closure
  evidence.** Table 246's FDF `/Ff` row cites "Table 221, Table 226, Table 228,
  and Table 230", which is the standard's own confirmation that no fifth
  field-flag table exists. Cheaper and stronger than asserting completeness from
  a manual sweep.
- **Two dispatch-hypothesis outcomes in one build (items 15/18/21 discipline):**
  the dispatch's *bit values* were **all correct** (re-derived via `2^(bit−1)`)
  but its *table numbers* were **wrong for the first two** — "Table 226 (common)
  / 227 (button)"; ISO 32000-1:2008 has **221** common, **226** button, and
  **Table 227 is `/Opt` for check box/radio, not a flag table**. Cheapest possible
  check: `grep -n "^Table 2[0-9][0-9] – " on the page dump. Do it even when the
  later numbers in the list are right — a partially-correct list reads as verified.

**Also this build:** the operational deliverable of a decode-hazard file is a
**function SIGNATURE**, stated as such — `decode_flags(ff: u32, ft: FieldType)`,
never `decode_flags(ff: u32)`. Naming the wrong signature is what makes the
finding actionable; a prose warning about bit 26 is not. And when a required
input (here `/FT`) is itself *inheritable*, say so in the same breath — the
hazard is the **conjunction** (reused bits + inheritable discriminator), and
either half alone is harmless.

**27. A NAMED-GAP-CLOSURE dispatch where the gap was a recorded NON-GOAL — and a
topic that splits across TWO licence bases *mid-clause*.** Established 2026-08-06
closing §12.7.3.4 Rich Text (`iso32000__s__12.7.3.4.md` + `adobe_ext__xfa_rich_text.md`,
opening the `adobe_ext\` directory). Eight sub-rules:

- **Closing a NON-GOAL is not the same as closing a GAP.** VT3 said "`/RC`/`/RV`/`/DS`
  = EXPLICIT NON-GOAL". Nothing in it was *wrong* — it was a correct **product-scope**
  record that the operator later reversed. So the closure is an **`AMENDMENT <date>`
  with the wording retained** (item 3) whose "what was wrong about it" line says
  **"nothing, as a scope statement"** and names the *date and decision* that retired
  it. Then **sweep every file that cited the non-goal** — VT3 was quoted in THREE
  (`12.7.3.3` VT3 row, `12.5.6` FreeText bullet, `ref__field_flags` cross-ref); each
  needed its own inline amendment, because a reader landing on any one of them
  otherwise still reads "out of scope."
- **THIRD instance of the licensing-axis split (items 1 / 15), and the first where
  the answer is "partly".** Before: "is it in ISO 32000-1?" was a yes/no test
  (AES-256 = measured no). Here ISO **pins the DATA and defers the BEHAVIOUR**: Tables
  223/224/225 are a *complete* PDF-1.5 grammar (`free_primary`, quotable), while the
  1.6/1.7 supersets **and every behavioural rule** go to XFA (`free_secondary_paraphrase`).
  **The refined test is "does ISO enumerate it, or name-and-defer it?"** — and the two
  files must cross-warn in both directions, because a lookup landing on either one
  alone answers confidently and half-wrongly.
- **Check whether the deferred-to document is in clause 3 (Normative references) or
  the Bibliography — the citing prose can be WRONG about it.** §12.7.3.4 says "see the
  **Bibliography**"; **XFA 2.0/2.2/2.4/2.5 are in clause 3, *Normative references***
  ("indispensable for the application of this document"). Only CSS2 [21] and XHTML 1.0
  [41] are bibliographic. That flips the conformance weight of the whole outward
  deferral — and it means a "the spec is silent" negative (RT-N1, unknown markup) is
  **closable by the referenced document**, normatively. **Always grep clause 3 and the
  Bibliography for the deferred-to title; do not trust the citing sentence.**
- **A `should` can carry the OPPOSITE engineering weight to its surface reading —
  trace which key the OUTPUT is bound to.** The dispatch asked "must `/V` accompany
  `/RV`?" Modality answer: **no, a `should`.** Product answer: a plain-text writer that
  sets `/V` and leaves `/RV` **corrupts the displayed value**, because §12.7.3.4 binds
  *appearance generation* to `/DS`+`/RV` and §12.7.3.3 makes regeneration a `shall` on
  every value change. **Item 6's modality tabulation is necessary but not sufficient** —
  also ask "**which key does the rendered output read?**" A `should`-optional mirror
  field beside a `shall`-authoritative source is a corruption hazard, not a nicety.
- **ERRATA CLUSTER, and an erratum can be EXPLAINED by an earlier build's finding.**
  Eight in one clause pair (E1–E8) vs the single Table 175/179 one in item 20. **E1 —
  §12.7.3.3 cites "Table 226" for the `RichText` flag (it is Table 228) — is almost
  certainly the *bit-26 collision* (`RadiosInUnison` T.226 / `RichText` T.228) recorded
  in the 2026-08-05 build leaking into the editors' prose.** Worth stating: it converts
  a bare "the spec has a typo" into a *pattern*. Also **three dangling EXAMPLE
  cross-refs** (E3/E7/E8 point at examples containing none of what is promised) — check
  every "as shown in the Example in X" by actually reading X; the cost is one `sed`.
- **Two independent extractors to confirm a suspected TYPO in the source.** The
  EXAMPLE's `xmlns=".../1999/xtml"` (missing `h`) looked like an extraction artifact.
  `pypdf` **and** `pdfminer.high_level.extract_text(page_numbers=[n])` both produced
  `xtml` ⇒ genuine. **pdfminer is the second opinion on pypdf** (extraction inserts
  spaces; it does not drop letters — but prove it rather than assert it).
- **Wayback's silent truncation boundary VARIES — the check is `%%EOF`, never a size.**
  Item-recorded as 1 MiB (2026-07-31); this time **exactly 5 242 880 B (5 MiB)** with
  `http=200`, and `file` still called it a valid PDF. **Two snapshot URLs for the same
  document behaved differently** (`/wp-content/uploads/…` truncated, `/norm-refs/…`
  complete) ⇒ if one Wayback path truncates, **try another path before resuming**.
  Also new: `archive.org/wayback/available` **HTTP 429** ⇒ skip the availability API
  and go straight to `https://web.archive.org/web/2id_/<url>`.
- **A licence can permit BUILDING THE FEATURE while forbidding copying the prose.**
  XFA 3.3's Preface grants anyone permission to "write software that … displays,
  prints, or otherwise interprets" XFA and to "copy Adobe's copyrighted grammar …
  to the extent necessary", conditioned on including the copyright notice — while
  the specification text "may not be copied without Adobe's permission."
  ⇒ `free_secondary_paraphrase` **for the prose**, but the **grammar tables are
  copy-permitted with attribution**, and the pdfce feature is expressly allowed.
  Different from the Adobe-Supplement row (item 6 of the licensing memory: all-rights-
  reserved, no grant). **Read the Preface/IP page, not just the cover NOTICE** — the
  cover said only "All information contained herein is the property of Adobe."

**28. A CLAIM-VERIFICATION dispatch (verify one owed citation; §8.7 pattern
anchoring, Pass 46.1) — the verdict class is a THREE-way, and the load-bearing
correction is usually to the claim's SCOPE, not its truth.** Established
2026-08-07. The parent framed it binary ("CONFIRMED, REFUTED or UNESTABLISHED")
and the honest answer was a fourth thing: **CONFIRMED-BUT-SCOPE-INCOMPLETE** —
true as stated for the case the researcher had in mind (a pattern `scn`'d in a
page content stream), silently over-general as written ("PDF patterns"). Five
sub-rules:

- **Always ask what the claim's IMPLICIT SCOPE was, then enumerate the sibling
  cases.** Here: the same shading dictionary painted by `sh` instead of via a
  `PatternType` 2 pattern is **CTM-relative** — the exact inverse — and ISO
  32000-1 states the contrast **itself, in one parenthesis** inside Table 77
  ("(By contrast, when a shading dictionary is used in a type 2 pattern, the
  coordinates are expressed in pattern space.)"). When a spec bothers to write
  a "by contrast" parenthesis, that is the standard flagging the exact
  confusion the dispatch is worried about. Grep for `By contrast|This differs
  from|rather than` near the clause.
- **"Does it differ between X and Y?" can be answered "the distinction does not
  exist" — and that is provable cheaply.** Table 75's and Table 76's `Matrix`
  rows are **word-for-word identical**, both deferring to §8.7.2, whose scope
  word is "**Every** pattern". Evidence it item-14 style (20 hits for `pattern
  matrix`/`pattern space`/`pattern coordinate`, **none** conditioned on
  `PatternType`). Two byte-identical table rows are stronger than any prose
  argument that two things behave alike.
- **A NOTE can be the only place the answer is stated plainly — say so, and pair
  it with the `shall`.** §8.7.2 **NOTE 1** is the sentence that names *rotation
  and scaling* and says they "have no effect on the pattern"; it is
  *informative*. The binding modality is elsewhere (§8.3.2.4 `shall`, §8.7.3.1
  step (b) `shall`). **Never let a code doc-comment cite the NOTE alone** — it
  reads as normative and is not. Same discipline as item 6, applied to
  normative-vs-informative rather than shall-vs-should.
- **A cost estimate can be right about the DIRECTION and wrong about the LAYER.**
  The dispatch's worry was "which toggle state costs work". Both the dispatch
  and the answer agree ON costs, OFF is free. The finding it did not have:
  **ON is not a content-stream edit at all** — `/Matrix` lives on the *shared
  pattern object*, so ON requires cloning the resource + the
  `/Resources`-inheritance guard + possibly *adding* an Optional key. Pricing a
  feature by "more/less work" hides a layer change; name the layer.
- **A verified-CONFIRMED claim still yields two new spec-ambiguity rows.**
  Reading the clause properly to confirm someone else's sentence surfaced
  PA-1 (§8.7.2 "defined as a resource" vs §8.10.1 NOTE 2 "used" diverge whenever
  a form omits its **Optional** `/Resources`) and PA-4 (Table 75 `TilingType`
  licences ±1 device pixel of distortion ⇒ **no pixel-stable regression test on
  a pattern fill is possible**). A confirmation dispatch is not a cheap dispatch;
  budget for the clause, not for the sentence.

**16. Under a NO-FALLBACK-SYNTHESIS product rule (pdfce R43), per-subtype spec
coverage collapses to a binary "AP-vs-fallback map".** Every ISO 32000-1 §12.5.6
geometry subtype defines a fallback look from its own keys (`L`/`Vertices`/
`InkList`/`QuadPoints`/`/IC`/`Name` icon) AND states "`/AP`, if present, shall
take precedence." When the engineering rule is "paint `/AP` or nothing, never
synthesize," that precedence sentence becomes the whole coverage: has `/AP` →
paint via §12.5.5; no `/AP` → **named-not-painted, counted by subtype.** The
useful file shape is a single table keyed on subtype with columns {fallback look,
fallback source keys, the verbatim AP-precedence quote, no-`/AP` verdict}. The
fallback geometry is still *modeled* (recognition/round-trip data), just not
painted. Same move applies to any future "recognize but don't act" scope
(e.g. `Border`/`BS` styles at display time).

**29. A RENDER-PIPELINE dispatch on a WHOLLY-UNINGESTED clause (image
transparency, clause 11, 2026-08-08) — five moves, and the highest-value finding
was an EXISTING ambiguity row that was never an ambiguity.**

- **"Clause C was never ingested" is a hypothesis about the CORPUS, not about the
  SOURCE — check the source first and say so in the first sentence of the
  report.** The dispatch pre-authorised a "the staged source does not contain
  clause 11, scope it as inferred" answer. ISO 32000-1:2008 contains clause 11 in
  full. `grep -n '^11\.[0-9]'` on the cached dump settled it in one command, and
  leading the reply with **"the source has it, nothing is from recall"** is worth
  more to a blocked engineer than any single answer. Inverse of item 15 (where
  the requested material genuinely was absent): **both outcomes are one grep
  away, and guessing either way is the failure.**
- **A "the spec is silent, no rule stated" AMBIGUITY ROW can be wrong — check
  the clause that OWNS the feature before filing one.** `iso32000__s__8.9.5.2.md`
  had carried "`Mask` + `SMask` both present — no precedence stated in §8.9.6"
  in `index.md`'s spec-ambiguity table since 2026-07-30. §8.9.6 **is** silent —
  and Table 89's `SMask` row **and** §11.6.4.3 each state the precedence
  independently and verbatim. The row was filed correctly *for the clause read*
  and was wrong *about the standard*. **Resolution shape: retain the row struck
  through, mark it `RESOLVED <date>, NOT an ambiguity`, and say why it is
  retained** (so a future session does not re-open it). Extends item 2b:
  an ambiguity can close on a *sibling clause*, not only on a negative result.
  General rule — **an ambiguity scoped to one clause is provisional until the
  owning clause has been read.**
- **ONE KEY NAME, TWO UNRELATED FEATURES, and the corpus's existing advice
  silently covered only one.** `/SMask` is (a) an ExtGState **dictionary** entry
  (§11.6.5.2 — needs §11.4 transparency groups, genuinely hard) and (b) an image
  **dictionary stream** entry (§11.6.5.3 — a plain `DeviceGray` image on the unit
  square, easy). `iso32000__s__8.4.5.md`'s "recognize-and-defer `SMask` —
  implementing it half-way is worse than not at all" was right about (a) and was
  being read as covering both, i.e. as *"soft masks are out of reach"*. Fixed
  with an `AMENDMENT` (item 3) whose "what was wrong" is **"nothing, as a
  statement about the ExtGState entry — it was over-read as a statement about
  soft masks in general"**. Generalises item 26's overloaded-`/Ff` finding from
  *bit positions* to *a key name across two dictionaries*, with the new twist
  that the **derived engineering advice**, not the table, carried the error.
  **Lead any file on an overloaded key with a `§0` side-by-side disambiguation
  table** — that section is the deliverable, not a preamble.
- **A POLARITY question is answered by a clause the dispatch did NOT name.**
  §8.9.6.3 (explicit masking) is three sentences and **never names a sample
  value**; it says the mask is "an image mask, as described in sub-clause
  8.9.6.2", and §8.9.6.2 carries the `0 shall mark the page` / `1 shall leave
  the previous contents unchanged` sentence. Same shape as item 23 (trace the
  operation to its INVARIANT clause) and item 17 (trace the artifact to its
  CONSUMER clause), third variant: **trace a clause's DEFINED-BY-REFERENCE terms
  back to their defining clause before reporting a gap.** The payoff table is the
  **cross-mechanism polarity matrix** — stencil `Decode [0 1]`: decoded 0 =
  PAINT; soft mask: decoded 0.0 = INVISIBLE. **Exactly opposite**, which is what
  makes the common producer bug (a 1-bit stencil supplied as an `/SMask`) render
  as a negative rather than as noise.
- **A `shall`-list table can restrict a key's DEFAULT without restricting its
  SEMANTICS or its VALUE SET — read the restriction table against the base
  table.** Table 145 gives `BitsPerComponent` as bare "Required" (⇒ Table 89's
  1/2/4/8/16 all stand) and `Decode` as "Default value: `[ 0 1 ]`" (⇒ §8.9.5.2's
  full semantics stand, so `[1 0]` **inverts alpha**). Reading Table 145 as an
  exhaustive spec of the sub-object would have produced two wrong answers.
  Corollary worth stating in the file: **keys the restriction table does not
  mention keep their ordinary meaning** (`/Filter`, `/Metadata`, `/OC`,
  `/SMaskInData` on a soft-mask image) — and one of those omissions was itself
  an ambiguity (SM-A3, Table 145 vs Table 89 for a JPX soft-mask image).

**Also this build:** the derived consolidator earned its place on the **pipeline
ORDER** rather than on breadth — the two `shall`s an implementer most easily
inverts (colour-key masking tests **pre-`Decode`** raw integers; `/Matte`
un-premultiply **precedes** colour conversion) are each stated once, in different
clauses, and neither is discoverable from the other. And the `AIS` answer is a
**"you may ignore this, and here is the bounded reason"**: `αs = fj·M·ca` is the
same product either way, so the flag matters only for knockout groups, group
re-composition and anti-alias semantics — a *scoped licence to simplify* is a
better deliverable than either "implement it" or silence.

**30. A THREE-AREA, PARITY-DRIVEN SCOPING dispatch (prepress: `Separation`/
`DeviceN`, `/OutputIntents`, overprint — 2026-08-08, 2nd build that day). Six
moves, and the two most reusable are about MODALITY LADDERS and DEVICE-CLASS
COLLAPSE.**

- **"A sibling librarian confirmed zero hits" is a fact about the CORPUS, never
  about the SOURCE — and this is now the SECOND same-day instance (item 29 was
  the first).** All three areas (§8.6.6.4/.5, §8.6.7 + §11.7, §14.11.4/.5, plus
  §7.10 which six files had *cited* without it ever being ingested) had been in
  `_sources/PDF32000_2008.pdf` since 2026-07-30. One
  `grep -n '^8\.6\.[0-9]' /c/tmp/iso32000_dump.txt` settled it. **The dispatch
  itself carried the correction** ("I told an agent clause 11 was absent from the
  corpus; it was absent from the RAG, not the source") — when a dispatch hands
  you a lesson, apply it *and say you did*, in the first line of the report.
- **A clause can DECLINE to define its own subject, and the defining clause is
  elsewhere.** §8.6.7 (Overprint Control) states the parameters and then says the
  effect "is device-dependent and **is not defined here**"; **§11.7.4** defines it
  as the implicit `CompatibleOverprint` blend mode with a complete decision matrix
  (Tables 148/149). A renderer built from §8.6.7 alone cannot produce a pixel.
  Deliverable shape: **two files that open with a `§0 READ THIS FIRST` naming the
  other and say "read both or neither"** — extends item 13's mirror cross-warning
  from a *direction* pair to a **DECLINE/DEFINE pair**. Watch for "is not defined
  here", "is device-dependent", "for further discussion see" — each is a pointer
  to where the normative content actually lives.
- **A "does the standard MANDATE X, or did vendor Y choose it?" question is
  answered with a MODALITY LADDER, and the strongest supporting text is often an
  informative NOTE in an unrelated clause.** For the output-intent working-space
  substitution: §14.11.5 says the data "**shall be provided for informational
  purposes only, and conforming readers are free to disregard it**" (an explicit
  *permission to ignore* — stronger than silence), and the only text pointing the
  other way is **§8.6.5.7 NOTE 3**, "an output intent dictionary, if present,
  **may suggest** such a calibration" — a `may`, **in a NOTE**, in the CIE-space
  clause. Verdict: **the vendor chose it.** Tabulate the ladder (rung / statement
  / modality / clause) with an explicit bottom row **"a `shall` binding a reader
  to adopt it — DOES NOT EXIST"**. Same normative-vs-informative discipline as
  item 28's NOTE 1 rule, now used to *refute a mandate* rather than to source a
  fact.
- **DEVICE-CLASS COLLAPSE: find the one property of pdfce's target that voids
  most of the spec area, and lead with it.** pdfce renders to an **additive**
  display, and *six independent `shall`s* collapse the whole prepress colour
  model: a `Separation` on an additive device "**never** applies a process
  colorant directly … **always** reverts to the alternate space" (§8.6.6.4);
  `NChannel` per-component reversion degrades to all-revert (§8.6.6.5);
  `alternateSpace`/`tintTransform` "**shall always be provided**" so ignoring
  `NChannel` is conformant; OPM 1 "**shall not apply if the device's native colour
  space is not `DeviceCMYK`**" (§8.6.7); "**if overprinting is not supported, the
  value of the overprint parameter shall be ignored**"; output intents are
  disregardable. ⇒ the entire obligation is *evaluate the tint transform, honour
  `/All`//`None`, preserve the rest*. **A collapse table is worth more to a
  scoping dispatch than any clause detail** — it converts "implement clause 8.6.6
  + 8.6.7 + 11.7 + 14.11" into a two-week job plus one clearly-separated
  architectural project (n-channel compositing for overprint *simulation*).
- **New consolidator AXIS: OBLIGED vs CHOOSING.** Item 26 established "same
  facts, new index key" (bit position instead of field type). This build adds a
  key that is not about content at all but about **conformance obligation** —
  `O1…O17` (`shall`) vs `C1…C8` (`may`/silence). It is what a *scoping* dispatch
  actually consumes, because it says where an acceptance criterion is a
  compliance test and where it is a product decision (and therefore, under
  pdfce's rule 4, must be *disclosed*). Two O-rows were things nobody asked
  about: **O14** (§11.7.4.4 makes `B`/`B*`/`b`/`b*` and Tr 2/6 an implicit
  knockout group **regardless of overprint** — probably a live render bug) and
  **O17** (`/SeparationInfo` makes a preseparated file n pages that are logically
  one ⇒ a page-delete/extract/merge **correctness** hazard, not a prepress topic).
- **Refusing a conformance matrix on LICENSING grounds — check clause 3 vs the
  Bibliography FIRST, and note this is item 27's rule producing the OPPOSITE
  outcome.** In the rich-text build, the deferred-to document (XFA) turned out to
  be in **clause 3, Normative references**, which made a "the spec is silent"
  negative *closable by reference*. Here **ISO 15930 (PDF/X) is Bibliography-only
  [1]–[5]** — so PDF/X is outside ISO 32000-1's normative scope entirely — **and**
  it is ISO-paywalled and absent from `LEGAL.md` §2, so it is doubly out of reach.
  `PDF/X-4` = **0 hits** (ISO 15930-7 postdates 2008). **Write no matrix**; write
  the two-sentence NOTE the standard *does* give, plus three named closure routes
  (operator-provided copy / veraPDF + Ghent Workgroup free secondaries / do
  nothing — read-and-preserve needs no PDF/X rule). Item 16 discipline, applied to
  a *conformance profile* rather than an algorithm.

**Also this build:** a **fourth erratum cluster** (F-E1 `07hD` for `7Dh`; OI-E1
§14.11.5 NOTE 2 citing Table **364** for the output-intent dictionary which is
Table **365**; §8.6.7 NOTE 2's dangling "EXAMPLE 3 in 8.6.6"; a `¥`/`³` glyph
artifact for `×`/`÷`) — cross-checking every cited Table/EXAMPLE number continues
to pay. **"Re-extract before reconstructing" is now 6 for 6** (Tables 38–42,
71–73, 148/149, 364/365 all came out row-aligned from the whole-document dump).
And the item-4a **equation-by-character-x-position** technique had its second
independent use (§7.10's type-0/2/3 formulas, source pages 102–105) — it is not a
one-off; treat *any* normative formula as requiring it.

**31. A CONSOLIDATION-ONLY dispatch (no clause ingested, no existing file
edited) — the deliverable is a RE-INDEX on a PRODUCT key, and its highest-value
output is a read-only sweep of the CONSUMER CODEBASE.** Established 2026-08-08
building `iso32000__ref__ambiguity_settings_register.md` (3rd build that day)
against a standing operator directive: *"where standards are ambiguous those
should become settings that the user can choose direction one, with the initial
installed default as the best guess of what is usually followed."* Extends
item 26 ("same facts, new index key" — bit position) and item 30 ("new axis:
OBLIGED vs CHOOSING") with a **third** key class: not a content key and not a
conformance key but a **PRODUCT key — does this become an operator setting?**
Seven sub-rules:

- **A directive that reclassifies a whole SECTION TYPE is a corpus-wide event,
  not a per-file one.** "Ambiguities become settings" converts every
  `## Gotchas / ambiguities` section from an engineering note into a product
  requirement backlog. The right response is ONE consolidator, not 96 edits.
  Watch for any operator statement of the form "X should always become Y" — it
  is a re-index request even when phrased as a policy.
- **MEASURE the population before triaging it, and expect the dispatch's count
  to be an OCCURRENCE count.** Dispatch said "~155 across ~15 areas". Actual:
  **67 unique `<AREA>-<A|N><n>` IDs in 159 occurrences across 12 areas**, plus
  **88 untagged rows** of `index.md`'s spec-ambiguity table (23 of its 111 rows
  carry a tag) ⇒ **155 distinct findings**. The dispatch's number was right and
  its unit was wrong. **"155 findings" and "67 IDs" are different numbers**; say
  both, because a roadmap entry that states the wrong one under-scopes the work.
  Cheap: `grep -rhoE '\b[A-Z]{1,4}-[AN][0-9]+\b' --exclude=index.md | sort -u`.
- **★ THE BIGGEST FINDING CAME FROM GREPPING THE CONSUMER CODEBASE, NOT THE
  CORPUS.** A read-only sweep of `D:\Dev\pdfce\crates\` showed **10 of the 18
  SETTING entries are already hard-coded**, with `file:line`. One of them
  (`annot_author.rs:43–51`) authors `/QuadPoints` in **Z/reading order against
  §12.5.6.10's stated counterclockwise**, labelling itself *"the open spec item —
  DECIDED here"*, disclosed in a code comment only, and resting on a corpus row
  still marked `NEEDS VERIFICATION`. **A corpus ambiguity row does not tell you
  whether the product already resolved it.** On any product-facing dispatch,
  grep the code for the ambiguity's ID *and* for the mechanism, and report
  `file:line` — that table is what makes the register actionable rather than
  archival. (Read-only; the RAG never writes to the repo.)
- **Grading a DEFAULT is the deliverable, not choosing one — and tier (d) will
  be the modal tier.** Evidence tiers: (a) Acrobat, cited to `Acrobat_Features`;
  (b) a run census; (c) other implementations **as documented** (never GPL
  source — MIT project); (d) reasoned guess. Result here: **2 of 18 reach (a)**
  (`/NeedAppearances` — Acrobat's own handling is *reported inconsistent*, so the
  (a)-evidence supports *not trusting the flag*, which is not the same as
  settling the behaviour; and, deferred, overprint preview's *"Only for PDF/X
  files"* default), **1 reaches (c) strongly** (CMYK JPEG polarity), the rest are
  **(d)**. Say so plainly. "We matched Acrobat" and "we guessed" must not read
  alike, and an (a)-tier source can settle a *posture* without settling the
  *behaviour* — grade what it actually supports.
- **BLAST RADIUS is the column that changes the answer.** Three classes:
  **RENDER** (free to flip), **EXTRACT** (changes search/copy **and
  redaction-by-text coverage** ⇒ an R35 correctness setting, not a preference),
  **BYTES** (constrained by rule 3 / R34 minimal-diff — a setting that rewrites
  untouched objects is a *violation*, not an option). Two settings that looked
  equivalent (`/NeedAppearances` policy vs mask filter) are a rule-3 problem and
  a free toggle respectively. Also: an EOL/byte-cosmetic setting's *correct*
  default is often **"match the base file"**, not any of the spec's named
  alternatives — the shipped fixed `SP LF` is arguably wrong on pdfce's own
  invariant, independent of the ambiguity.
- **A triage bucket that says "NO KNOB" is a first-class deliverable — and
  REFUSAL needs a per-row justification.** 41 determinate + 3 out-of-band + 11
  refusal = 55 rows whose value is *stopping* a build. The out-of-band bucket
  (item 3) is the subtle one: the standard **defers to agreement outside the
  file** (OI-A1: *"PDF intentionally does not include a selector … a matter for
  agreement between the purchaser and provider of production services"*), so the
  control's WORDING is *"which one do you want"*, never *"which reading is
  right"*. And the model implementation for the whole register already exists in
  the codebase: **refuse by default + a named escape that states its
  consequence** (`EditError::FieldIsRichText` +
  `fill_text_field_downgrading_rich_text`, which clears bit 26 and removes `/RV`
  so the field *stops being* rich text rather than being silently corrupted).
- **A consolidation build can still MINT a new ambiguity — and it should be
  labelled register-local until back-filed.** `IM-A1`: **§8.9.5.3 defines image
  interpolation only for MAGNIFICATION** (*"resolution … significantly lower than
  … the output device"*) and says **nothing about minification** — `minif` **0**,
  `mipmap` **0**, `decimat` **0**, `down-sampl` **0**, `downsampl` **2 both
  unrelated**. So `/Interpolate false` does **not** mandate point-sampling on the
  way down. Found by reading the clause to triage an *existing* row. **Under a
  no-edit constraint, mint the ID in the new file, mark it "not yet back-filed",
  and list the back-fill in a `## Corrections owed` section** — that section is
  how a no-edit build stays honest instead of losing findings. Also record the
  DEFAULT you did **not** recommend: `interpret.rs` asserts *"most production
  viewers smooth on minification"*, which is unverified ⇒ keep the status quo
  default and file the verification, rather than flipping a default on a
  plausible-sounding recalled claim (the URW failure shape, applied to a *product
  default* rather than a licence).

**Also this build:** the register's most useful structural finding was not a
spec fact at all — **there is nowhere to put a setting.** `pdfce-gui/src/main.rs`
repeatedly says UI state is *"session state only — deliberately not persisted"*
and that persistence belongs to *"the not-yet-built **R15**"* user-state
partition ⇒ **R15 is a prerequisite Pass for the entire directive.** When a
dispatch asks "which settings first?", also answer **"where would one live?"** —
a priority list with no home is a list of blocked items. The one working
precedent is `ExtractOptions::word_gap_ratio` (default `0.20`): a named core
field + documented default + `with_gap_ratios` builder, **with 0 CLI and 0 GUI
hits** ⇒ the cheapest entry in the whole register is *exposing what already
exists*, and it is the template for the other 17.
