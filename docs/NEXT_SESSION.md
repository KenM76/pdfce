# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. **Overwrite it once acted on.**

Per standing rule `R216` this file carries **no edit-history layer**: no
*"this paragraph read X until…"*. What is true now, plus a pointer.
Corrections and their prior wording live in the **append-only** record —
`ROADMAP.md` and `SESSION_LOG.md` — where a claim is dated and no later edit
can falsify it.

---

## §A — DO THIS FIRST, BY OPERATOR INSTRUCTION

**Ken, 2026-08-27, verbatim: *"check the feature requests and write these as
the first thing to do before continuing work on other things."***

Three items arrived on `pdfce_FeatureRequests` **during** the last session and
are now at the front of *Next up*, ahead of everything else. Work them in this
order — the ordering is the engineer's, the priority is Ken's:

| # | Pass | what |
|---|---|---|
| 1 | **`142.1`** | the font-resource **pre-flight** — which faces actually resolve on a page |
| 2 | **`144.0`** | `gate_synthesis` names a face that then refuses, so **bold is unreachable** on that page |
| 3 | **`145.0`** | a pinned `FormatRequest` cannot say *"the whole operator"* |

★ **`142.1` goes first even though `144.0` is the defect**, and the reason is
this project's own recurring failure: `144.0` needs a predicate for *"would
`set_font` accept this face for this run?"*, and that is exactly what `142.1`
computes. Build `142.1`, then make `144.0` a **caller** of it. Doing `144.0`
first means writing that predicate twice, which is how `144.0` was created in
the first place (see below).

**`Pass 143.0` is no longer first.** Unchanged in content, still in *Backlog*,
still fully diagnosed — it just stopped being the pick-up item.

---

## §B — `Pass 144.0`, THE DEFECT, AND IT FALSIFIES SOMETHING I TOLD THEM

**Reproduced before it was believed** — all three commands, on `pdfce-cli` at
`703a38e`, against pdfce's own fixture
`fixtures/synthetic/textedit/format_family.pdf`:

```
--find "hello world" --bold-synthetic
  refused: a REAL bold face is available as 'Times-Bold' (resource /F3)
           … change the run's family to 'Times-Bold' instead.

--find "hello world" --set-font Times-Bold        <- the remedy it just named
  refused: R-INV-7: 'o' has no code in 'Times-Bold's encoding; code 111
           is already assigned by /Differences

--find "hello world" --set-font F2                <- never mentioned
  set_font=Times-Roman->Calibri-Bold              <- SUCCEEDS
```

**So this claim, which I sent to `pdfceGUI` and put in
`docs/core-api/03-capabilities.md` §3.6, is false:**

> ~~"Between the two verbs every page is covered. … There is no page on which
> bold is unreachable."~~

True only for an operator who already knows to try a face pdfce never names.

**Cause.** Family matching (`Times-Roman` → `Times-Bold`) is a sensible
preference and is right in general. The defect is that the preference is
applied **without asking whether the preferred face can show this text** — and
that answer is already computable, because `set_font` computes it moments later
and refuses on it.

**Fix.** `gate_synthesis` treats a real face as *available* only if `set_font`
would actually accept it **for this run**. Where the family match fails
coverage and another resource passes, name that one; where none passes,
synthesis is genuinely the only option and must proceed.

★ **`R90` is NOT being weakened, and the entry must keep saying so**, because
the diff will read as a loosened refusal. `R90` says synthesis is a fallback
for when no real face *resolves*. That stands. What changes is the predicate
for **"resolves"** — from *"exists with a matching family name"* to *"would
actually be accepted"*. `R90` applied more accurately, not less.

★★ **Fixing this turns a `pdfceGUI` test RED on purpose and they asked for
that.** Their Bold button retries with the face the refusal names; their
characterisation test asserts "nothing was applied" and its docstring says the
failure is the good news. **Tell them on the channel when it ships**, with the
revision.

★★★ **`144.0` is `R221`'s third instance, minted hours earlier, in a different
subsystem** — and it **inverts `R221`'s risk analysis**. `gate_synthesis`
decides "is a real bold face available" with two *string tests on `/BaseFont`*
(`family_stem`, and `name_claims_bold` = `contains("bold")||"black"||"heavy"||
"semib"`) — a **parallel description** of when `set_font` succeeds. `R221`'s
mint could say "neither error direction can paint a wrong colour"; here a false
positive **removes the capability entirely**.

★ **And a doc comment nobody reported is falsified by half its own callers:**
`synth.rs:391–403` says `name_claims_bold` is *"used only in the direction
where being wrong is safe… never to refuse an edit."* There are exactly two
call sites and `format.rs:2165` **refuses an edit**. True when written,
falsified by a later caller, invisible to `cargo doc`. It is `144.0`'s
criterion 5 so the comment, the predicate and the format string move together.

---

## §C — `Pass 145.0`, AND A MECHANISM BOTH SIDES HAD WRONG

The ask: **`find: ""` on a request carrying `pinned_span` means the whole
pinned operator.** A caller that has already *located* an operator should not
have to *describe* it. Their guess is two lines in `match_run`.

Their three failed attempts are in the ROADMAP entry. **One of the causes they
gave is wrong, and I nearly documented it as pdfce's own statement about
pdfce's own type.**

> ~~"`TextRun::text` contains characters that are not in the file — extraction
> synthesises a space wherever a `TJ` offset exceeds the word-gap threshold"~~

**A derived word space is never inside a glyph run.** `layout.rs`'s
`Break::Word` arm calls `close_run()` and *then* `emit_derived(' ',
DerivedWordSpace)`, pushing a **separate one-character run with no glyphs**;
`model.rs:548` keeps it separate. Measured over **256 fixture PDFs**:

```
derived_word_space runs (always separate)        5
glyph runs containing a synthesised space        0    <- their stated cause
glyph runs where len(text) != len(glyphs)        1
```

★★ **The single offender is the real mechanism, and neither side had it.**
`identity-h-tounicode.pdf` — 8 characters over **6** glyphs, because
**`/ToUnicode` maps one glyph to several characters** (§9.10.3): an `ffl`
ligature is one glyph and three characters; a surrogate pair is one glyph and
two `char`s.

**Their headline claim is true and their mechanism was wrong**, and the
difference decides the scope: word-gap synthesis would insert a character
present in *no* operator, whereas a ligature maps a character **range** onto a
glyph that **is** in the operator. So a `find` built from `TextRun::text` fails
on the **buffer bytes**, not on locating the operator — invisible on
unligatured test text, **routine on real typeset copy**.

⇒ The docs half of `145.0` is therefore **not** "text may contain derived
characters" (false). It is: **one glyph may map to several characters, so
`text.chars().count()` is not `glyphs.len()`**, and a caller building a `find`
string from a run cannot assume a 1:1 correspondence with the buffer.

### ★ STILL UNVERIFIED, AND OWED TO THEM EITHER WAY

Their third cause — **"a `TextRun` can span several show operators"** — I could
not measure: `GlyphProvenance::operator_span` is **not** exposed by
`pdfce-cli extract-text --json` (glyphs carry `code`/`rung`/`sourced`/`start`/
`len`/`x`/`y`/`advance`/`size`/`direction`/`invisible`, no span).

They also asked whether this invariant holds: *"the glyphs sharing one
`operator_span` always slice a contiguous, matchable range out of the run's
text."* **They have already shipped a workaround that depends on it** and have
no way to know whether it is guaranteed.

**One probe answers both**: an in-crate test over a corpus counting runs whose
glyphs carry more than one distinct `model.provenance(...).operator_span`. If
the invariant holds it should be documented and they can rely on it; if it does
not, **what they shipped is resting on luck and they need that more urgently
than the API change.** If it holds it is a decision-log candidate.

---

## §D — `142.1`, AND WHY IT IS FIRST

Their reply **answers the question the last handoff called blocking**.
Verbatim: **"Synthetic is enough. Drop `142.0` down the queue."** Their
reasoning is a *use report*, not a preference: CAD exports, part numbers and
revision letters, a fabricator reading a print; at 8 pt on a 1:50 site plan a
stroked regular and a real Bold are indistinguishable on paper. And *"the
operator's standing complaint about this program is that basic things do not
work, not that they work imperfectly."*

★ **`142.0` is de-prioritised, NOT closed** — they scoped it themselves: *"a
report of our operator's use, not a decision about yours."*

**`142.1` wants two things**, and (2) is the one they call more valuable:

1. the list keyed **the way `set_font` matches** — the strings that *will*
   resolve, not the dictionaries that exist;
2. per entry, whether a real **Bold** and a real **Italic** of that family also
   resolve on the page — the fact that decides whether a Bold button routes to
   `set_font` or to `set_synthetic`.

★ **The join problem that makes a naive version wrong:** `fontinfo` is keyed on
the font **dictionary**; `set_font` matches on `/BaseFont` with the §9.6.4
subset tag stripped; and one page can carry **two dictionaries with the same
`/BaseFont`** — two independent subsets of one face, which their survey found
in **87 %** of embedding files. A list built from `fontinfo` is a superset that
is *usually* exactly right, and when it is wrong the operator finds out by
pressing a button.

---

## §E — WHAT SHIPPED LAST SESSION

`Pass 140.0` + `140.1` (`70c5919`), `140.2` (`25d73d7`), filings `785b299`,
`0c983f2`, and `703a38e`. A five-colorant `DeviceN` photograph was rendering
visibly desaturated; it now matches Acrobat closely (mean error over the
photograph **33.06 → 16.85**; over an inset patch **29.57 → 7.13**).

★ **Route 4 — the route that was NOT in the bug report — accounts for 100 % of
that fix**, proved by ablation. Had only the reported route been implemented,
the reported defect would not have been fixed at all. That is the strongest
datum this project has for `R219`.

★★ **The print-conformance suite reads 6 FAIL, up from 5, and that is
correct** — a false pass was removed. Full method in `SESSION_LOG.md`; the
remaining half is `Pass 143.0`.

### ★ Verified from a shell at write time — re-run, do not copy forward

| fact | value | command |
|---|---|---|
| `HEAD` | `703a38e` **plus the 296th filing and this file's own commit** | `git rev-parse --short HEAD` |
| `origin/main` | level at `703a38e` at write time | `git rev-list --count origin/main..main` |
| tests | **4,419 passing, 0 failing** | `cargo test --workspace` |
| gates | 18 run bare, all green; `check-image-colorspace-truth.py` needs a fixture-dir argument | `ls tools/check-*` |
| `fmt` / `clippy` | clean | `cargo fmt --all --check` |
| fuzz | targets build; `image_import` 53,595 runs, `load_document` 165,969 runs, no artifacts | `cargo +nightly fuzz build` |
| wasm32 / GUI-core | both clean | `cargo tree -p pdfce-core` |
| **CI** | `25d73d7` was **green on all ten jobs**. `703a38e` and later: **read it from GitHub.** | `gh run list --branch main --limit 3` |
| disk | 202 GB free of 954 GB — no longer the binding constraint | `df -h /d` |

★★★ **PUSHING NEEDS NO ASKING — decision `090`.** *"always push."* **Not**
covered: cutting a tag or release, `git push --force` or anything rewriting
published history, and any branch other than `main`. **Scrub
`tools/check-suite-name-absent.py` green before every push** — the repo is
public and that gate scans untracked files too, which is why scratch renders
belong in `D:\Dev\temp\pdfce\`.

★ **Fuzzing needs the MSVC ASan DLL on `PATH`**, else `0xc0000135`. Find it:
`find "/c/Program Files (x86)/Microsoft Visual Studio" -iname "clang_rt.asan_dynamic-x86_64.dll"`.

---

## §F — THREE THINGS OWED TO `pdfceGUI`, NONE OF THEM CODE

1. **The §3.6 correction** — `docs/core-api/03-capabilities.md:1229` still
   carries the false "every page is covered" universal. Engineer-owned; fix it.
   ★ **Do not touch `:1248`** (*"do not grey out a bold button"*) — that one is
   still true, and correcting the wrong sentence is the available mistake.
2. **A warning before `144.0` lands**, so their red test is expected.
3. **The `operator_span`-slice invariant answer** (§C).

A confirmation note is already on their channel:
`confirmed_gate_synthesis_is_a_real_defect_and_my_every_page_claim_was_false.md`.
`iccce` was checked and is clear (19 files, newest is pdfce's own reply).

---

## §G — HABITS THAT PAID, AND ONE THAT NEARLY DID NOT

### A bug report's SYMPTOM is evidence; its MECHANISM is a hypothesis

They arrive in the same file, in the same confident voice. I relayed
`pdfceGUI`'s mechanism into a librarian dispatch **as fact**, and it was on its
way into `docs/core-api/` as pdfce's own statement about pdfce's own type when
I measured it and found it false. Caught only because I had spent the same hour
writing an apology for failing to check a claim before relaying it.

### A universal claim is an absence claim wearing the opposite sign

*"Every page is covered"* and *"no such verb exists"* fail identically: a claim
quantified over all cases, verified on the cases that came to mind. Three
instances in three days, **each one the repair for the previous**. `R220` now
carries this as clause (d).

★ And note where `144.0` actually lived: the fixture's own `PROVENANCE.md`
documents **both halves correctly and separately** — `/F2` as "a fully-covering
target", `/F3` as one that "does NOT cover `o`". Nothing ever asked what
happens when the **synthesis gate picks between them**. Two correct documented
facts with the defect in the join.

### Verify a claim about your own code from outside, before believing OR dismissing it

All three of their commands were reproduced before the report was accepted.
That is what made it a *confirmed* defect rather than a triage item — and it is
the same discipline that then falsified their mechanism.

### `cp x /tmp/x.bak` before every sabotage

Used eight times, no loss. `git checkout --` is not an undo.

---

## §H — OPEN, UNCHANGED

- **`(bl)`** — may a **CC-BY-SA-4.0** OCR model ship inside pdfce's **MIT**
  portable folder? Ken's call; default if unanswered is **ship neither model
  set**.
- **`R13` vs "download addin capability"** — downloading is permitted;
  **executing** what was downloaded is not, and that ruling is owed from the
  operator. **No add-in Pass can be scoped until it lands.**
- **`(p)`** — whether to narrow the XFA item to read/fill only, or retire it.
- **An observation, not a decision:** `tools/check-image-colorspace-truth.py`
  writes a `_truth/` subdirectory beside the images it scores, and pointing it
  at `fixtures/synthetic/images` broke a `pdfce-core` test last session.
  Whether that gate should write into a fixture directory at all is an open
  engineer question.
