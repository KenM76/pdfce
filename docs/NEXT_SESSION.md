# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. **Overwrite it once acted on.**

Per standing rule `R216` this file carries **no edit-history layer**: no
*"this paragraph read X until…"*. What is true now, plus a pointer.
Corrections and their prior wording live in the **append-only** record —
`ROADMAP.md` and `SESSION_LOG.md` — where a claim is dated and no later edit
can falsify it.

---

## §A — COLD START

**The previous session cleared the feature-request backlog on both channels.**
Two Passes shipped, one request was answered by pointing at a verb that
already existed, and two ten-day-old asks from the sibling colour project
(filed 2026-08-17, reply owed and never given) were finally measured.

- **`Pass 139.0`/`139.1`/`139.2`** (`c362b6b`) — **text stamped sideways in a
  CAD title block came out one character per line.** Extraction now publishes
  the writing direction and measures every layout threshold in the line's own
  frame. 22 derived line breaks → 3 on the fixture; 16 lines → 4 in the
  editable model.
- **`Pass 81.1`** (`4eaea20`) — **drawing a translucent highlight took two
  verbs, so one Ctrl+Z left an opaque one behind.** Now one verb and one undo
  entry, on both authoring routes.
- **`3ffd86f`** — a docs repair with a finding in it (`§D`) — **and `8a8c9a8`,
  which corrects a false claim `3ffd86f` itself introduced and shipped to a
  consuming project. Read `§D` and `§E` together.**
- **`51f94ca` + `96b6ded`** — `tools/icc-census`, the ICC profile population
  census the `iccce` project asked for on 2026-08-17 and never got.

### ★ Verified from a shell at write time — re-run, do not copy forward

| fact | value | command |
|---|---|---|
| `HEAD` | `f56b13a` **plus this file's own commit** — a handoff cannot name the commit that contains it | `git rev-parse --short HEAD` |
| `git describe --tags` | `v0.14.0-53-gf56b13a`, likewise +1 | `git describe --tags` |
| `origin/main` | **LEVEL — everything is pushed and filed** | `git rev-list --count origin/main..main` |
| tag at `HEAD` | none; highest is `v0.14.0` | `git tag --points-at HEAD` |
| tests | **4,410 passing, 0 failing** | `cargo test --workspace` |
| gates | **19 on disk; 18 run bare; all green** | `ls tools/check-*` |
| `fmt` / `clippy` | clean | `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` |
| fuzz | all targets compile | `cargo +nightly fuzz build` |
| wasm32 | `pdfce-core` + `pdfce-render` compile | `cargo check -p pdfce-core -p pdfce-render --target wasm32-unknown-unknown` |
| GUI-core invariant | no GUI dep in either engine crate | `cargo tree -p pdfce-core` |
| **CI at `HEAD`** | **READ IT FROM GITHUB. Do not believe this table about it.** | `gh run list --branch main --limit 3` |

★★★ **PUSHING NEEDS NO ASKING — decision `090`, 2026-08-27.** Ken's ruling,
verbatim and in full: ***"always push."*** An ordinary fast-forward push of
`main` is standing-authorized.

★ **Three things it does NOT cover**, narrowed deliberately: cutting a **tag
or release**; **`git push --force`** or anything rewriting published history
(`check-cited-commits-exist.py` exists because that broke fourteen document
citations); and **any branch other than `main`**.

★ **Scrub `tools/check-suite-name-absent.py` green before every push.** The
repository is public, so a push publishes (`LEGAL.md` §1.1). That gate scans
**untracked** files too — which is why scratch renders belong in
`D:\Dev\temp\pdfce\`, never in the tree.

---

## §B — NOTHING IS UNFILED AND NOTHING IS UNPUSHED

`check-commits-filed.py` is **green** at `HEAD` (616 code commits, tip
included) and `origin/main` is **level**. Verified after the last push.

★ **`R217` is why that matters and it was reached twice in two days:** the
gate defers the tip — a commit cannot cite its own hash — so **one** unfiled
commit on `main` is safe and the moment a **second** lands, the first is
checked in full. *One unfiled commit may sit at the tip. Never two.*

★ **A docs-only commit is invisible to that gate**, which counts *code*
commits. `3ffd86f` carried a real finding and would have gone unfiled with
nothing complaining. If you write one, file it deliberately.

---

## §C — THE DISK, WHICH IS NOW THE NEAREST HARD LIMIT

**14 GB free of 954 GB (99 % used)** at write time, after deleting `target/debug/incremental`
(7.1 GB) at the end of the session. It was **9 GB** before that, having been
35 GB the same morning — a full workspace test run, a fuzz build, a wasm
check and a new tool crate between them ate 26 GB in a day.

If you need more: `cargo clean` recovers ~21 GB, `cargo clean --manifest-path
fuzz/Cargo.toml` another 1.1 GB, and the seven `tools/*/target` directories
about 3.1 GB between them. All regenerable; all cold-rebuild expensive.

**Do not let this reach zero mid-run.** A `cargo test --workspace` that dies
on a full disk leaves a corrupt `target/` that has to be cleaned anyway.

---

## §D — THE FINDING IN `3ffd86f`, WHICH IS NOT ABOUT DOCUMENTATION

`EditSession::format_text` shipped 2026-08-20. It restyles **existing** page
text — size, fill colour, font family, plus `Tc`/`Tw`/`Tz`/script/rise. It is
fully and correctly documented in `docs/core-api/02-editing-and-saving.md`.

On 2026-08-25 `pdfceGUI` filed a request saying **no such verb existed**,
with a table reading *"none available"* for every restyle button. They had
looked.

★★ **Nothing was wrong in either document and
`tools/check-core-api-verbs.py` was green.** That gate fires on a verb being
**absent** — the `insert_pages` failure mode. This verb was present,
accurate, and **unfindable by the question the reader actually had**, because
it was filed under *text editing mechanics* and the question was *how do I
make this bold*.

**Findability is not a property any script can evaluate**, so this is not a
gate that can be extended. The repair was structural:
`docs/core-api/03-capabilities.md` **§3.6**, written capability-shaped.

★★★ **AND THE REPAIR SHIPPED A FALSE CLAIM OF ITS OWN, TO THE SAME PROJECT.**
§3.6 asserted bold and italic were unavailable on existing text. They are
not — see `§E`. I had measured `set_font`'s refusal, confirmed it was real,
and **inferred a capability gap from a single verb without asking whether a
second verb reached the same operator goal.**

> **An absence claim about pdfce is a claim about EVERY route, and I checked
> one.**

That is `R220`'s second clause, and it was minted at n=2 against this
project's usual bar precisely because **the second instance occurred inside
the repair for the first**. It was caught by the librarian during the filing
sweep — not by me, and not before the reply was sent.

★ **`docs/FEATURES.md` had been right the whole time.** When a
capability-shaped document and a mechanism-shaped one disagree about what
pdfce can do, **trust the capability-shaped one.**

⇒ **A verb ships in two documents, not one.** `02` answers *what does this
call do*; `03` answers *how do I do X*. A verb only in `02` is reachable only
by somebody who already knows its name.

---

## §E — WHAT IS OPEN AND ACTIONABLE

**If you want one thing to pick up: `Pass 140.0`.** It is fully diagnosed
below, has a named wrong line of code and a named wrong fix to avoid, needs
no operator decision, and it is the only item here that is blocked on nothing
but effort. Everything else in this section is waiting on somebody's answer.


### `Pass 142.0` / `142.1` — a REAL bold/italic face, and the missing pre-flight

★★ **Read this before believing anything about bold.** An earlier draft of
this file, `03-capabilities.md` §3.6, and a reply already sent to `pdfceGUI`
all said **bold and italic do not work on existing text**. That is **false**,
and the correction is `8a8c9a8`.

`FormatRequest::set_synthetic` — `--bold-synthetic` / `--italic-synthetic` —
shipped in `Pass 19.2` (`ebe35d8`, 2026-08-03). `gate_synthesis`
(`format.rs:2132`) refuses synthesis **exactly when a real face IS a page
resource**, the precise complement of `set_font`'s predicate, so **between
the two verbs every page is covered** and bold is reachable everywhere. The
only real refusal is synthetic *italic* where a `Td`/`TD`/`T*` follows the
run in the same text object (a `Tm` sets the text **line** matrix too,
§9.4.2 Table 108, so the next line would land shifted).

**What `FF-C` blocks is a *real* bold/italic FACE** — a typographic-quality
gap, not a capability gap. `142.0` is that; `142.1` is the font-resource
pre-flight, so a shell can ask *"is there a real Bold here?"* before pressing
anything instead of driving the control from a refusal.

★ **The priority question is with `pdfceGUI`, and it is smaller than it
looks:** do their operators need a real face, or is a disclosed synthetic
weight enough for markup and title-block work? If synthetic suffices, `142.0`
drops down the queue. Their answer decides it, not us.

**The mechanism, for whoever builds it:** `add_text` already creates
Standard-14 resources and embeds donor faces. What is missing is a
`FormatPlan` that can carry new objects and a resource-dictionary patch —
today it produces a content buffer and nothing else. `R107` applies: FF-C
must only ever **add** font resources, never rewrite an existing one.

### `Pass 140.0` — the five-colorant `DeviceN` image

Still diagnosed and unstarted. **★ Renumbered from `139.0` on 2026-08-27** —
that ID was allocated in a Backlog heading and independently consumed by a
shipped commit, because `check-ledger-numbers.py` reads *headings* and a
number claimed in one document is invisible to it until that heading exists.
The shipped side kept its numbers; this moved.

A five-colorant `DeviceN` photograph renders **visibly desaturated** because
`Separation`/`DeviceN` images convert through their tint transform to sRGB
and never to their `DeviceCMYK` alternate. In `pdfce-render/src/image.rs`:

```rust
let carries_ink =
    matches!(space, Space::Cmyk) || matches!(space, Space::Indexed { ink: Some(_), .. });
```

★★ **DO NOT REUSE THE OVERPRINT PLANES.** They exist texel-for-texel in the
same layout and are the obvious wrong move: they hold `authored_tints`, which
answers Table 149's *"which components did the source SPECIFY"* — a different
question — and return `None` for a **spot-only** `DeviceN`, where the code
writes `[0,0,0,0]`. Correct for the overprint route (which preserves the
backdrop, `Pass 130.3`); it would paint **bare white paper** on a plain one.

The right shape is a per-texel `space.to_cmyk(comps, diag)` captured *beside*
the existing `to_rgb`, in the same loop from the same operands — what
`ColorRamp::new` and `mesh::read_shade` already do. **Measure the cost
first**: that is a second tint-transform evaluation per texel unless the sRGB
value is derived from the CMYK one, and `tint_applied = 292` on the measured
page suggests a cache is already doing work. Find out what it is keyed on.

**This is the fourth instance of `R219`'s shape in four days**, and it was
found by applying the rule rather than by a bug report — `130.1` gave ink to
`DeviceCMYK` images, `130.2` to `Separation`/`DeviceN` **only under
overprint**, `137.0` to analytic shadings, `137.1` to meshes, and this row is
the gap nobody went back for.

### The other four print-conformance failures

spot-colour overprint (6 traps), white overprint (5), ICC source profile (4),
blend modes in an ICC RGB group (12). Run:
`python tools/suite-check.py <corpus> --reference-dir <refs>` — the private
map at `D:\Dev\pdfce-private\suite\` names both directories. **Patch stems are
deliberately not listed here** (operator ruling 2026-08-25, enforced by
`check-suite-name-absent.py`, which caught this table on its first draft).

---

## §F — HABITS THIS SESSION PAID FOR

### Sabotage every fix, including the ones whose tests already pass

`Pass 139.1` fixed extraction; the `EditableTextModel` hit-test tests then
passed — **for the wrong reason.** A second copy of the same page-axis
assumption in a second module had turned every glyph of a vertical run into
its own line, so every probe trivially found "its" line. Reverting the fix
changed nothing, which is what gave it away.

> **When a fix upstream makes a downstream test start passing, that test may
> now be passing for a different reason than it was written for, and only
> sabotage distinguishes those.**

Six sabotages across that Pass failed 5/6/1/1/1/1 tests, none overlapping.
**Two of the three fixtures exist only because a sabotage found a hole** — the
first fixture alone left the direction-change rule and both dot products
entirely uncovered.

### A per-group sample weighted by group size is an extrapolation

`icc-census` reported **91** `/N` disagreements. Measured per embedding, the
answer is **2** — wrong by 45×, because weighting one distinct profile's
first-seen `/N` by its embedding count assumes exactly what the axis tests.

★ And **fixing it in the aggregate did nothing about the detail export**: the
TSV column was still a first-seen sample, and a sentence written from it was
wrong in a *different* way (two profiles and two directions, not one). The
repair that worked was renaming the column `pdf_n_FIRST_SEEN` so the mistake
is unavailable. A comment would have been read by nobody, including me,
twice.

### A bucket that cannot distinguish "clean" from "not measured" is not evidence

The same tool first reported *"100 % agree, or no LUT tag"* across 2,494
embeddings — which reads as a strong negative result and is equally
consistent with the check never having run. Split three ways, the honest
answer is **136 checkable, 136 agreeing, 2,358 not checkable**.

All three of those were caught by **reading the output**, never by a test.

### `git checkout --` is not an undo

Used mid-session to revert a one-line sabotage; it reverted the **whole file
to `HEAD`** and discarded forty minutes of work. `cp x /tmp/x.bak` first and
restore from that.

---

## §G — OPEN, UNCHANGED

- **`(bl)`** — may a **CC-BY-SA-4.0** OCR model ship inside pdfce's **MIT**
  portable folder? Ken's call; default if unanswered is **ship neither model
  set**. `docs/ocr-engine-survey.md`.
- **`R13` vs "download addin capability"** — downloading is permitted;
  **executing** what was downloaded is not, and that ruling is owed from the
  operator. **No add-in Pass can be scoped until it lands.**
- **`(p)`** — whether to narrow the XFA item to read/fill only, or retire it.

## §H — THE CHANNELS ARE CLEAR, AND CHECK THEM ANYWAY

Both `D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\` and
`iccce_FeatureRequests\open\` were worked to zero outstanding asks this
session. **They live outside the repository, so no gate can contradict a
stale "it's empty" claim** — list them yourself at session start.

Replies filed and awaiting the other side:

| file | awaiting |
|---|---|
| `note_the_writing_direction_is_published_and_your_400_lines_can_go.md` | pdfceGUI deleting `canvas::textsel::writing` (~400 lines) |
| `note_opacity_at_author_time_shipped_and_it_was_an_undo_bug.md` | pdfceGUI wiring `add_markup_with` |
| `reply_restyle_already_exists_…` **+ `correction_bold_and_italic_DO_work_…`** | **a decision from them**, and it is *not* "is bold worth building" — bold already works. It is: **is a disclosed SYNTHETIC weight enough, or do their operators need a real face?** Read the correction, not the reply. |
| `iccce/reply_the_profile_census_and_your_33_node_constant.md` | iccce re-examining their 33-node constant |

★ **Do not re-send anything from the restyle reply without reading the
correction beside it.** The reply carries a banner and struck-through rows,
but the file is long and the wrong rows are in the middle of it.

★ **One disagreement in that last one needs resolving rather than
averaging.** `iccce`'s evidence names `USWebCoatedSWOP.icc` as carrying a
33-node `lut8`; the `U.S. Web Coated (SWOP) v2` in this corpus reports **9**.
Either they are different files or one of us reads the grid byte from a
different offset. pdfce's is byte 10 of the tag per ICC.1:2010 §10.10/§10.11
and is sabotage-tested — but a disagreement about a specific named file is
the kind of thing to settle, not split.
