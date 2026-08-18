# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. Overwrite it once acted on.

**Written 2026-08-18 at `d6c524a`** (`v0.6.0-70-gd6c524a`), replacing the
earlier 2026-08-18 handoff whose queue has now been worked.

---

## ★★★ THE TASK: `Pass 75.0` — THE REUSABLE PARSED HANDLE, built as **option (b)**

The operator chose **(b), the full `Canvas` indirection**, over the narrow
slice, and asked for a fresh session's headroom for it. **Build it.**

`ROADMAP.md`'s `Pass 75.0` entry holds the request, the consumer's argument
and the seven acceptance criteria. **Read it — but do not re-derive the
measurements. They are done, and they are below.**

---

## §1 — WHAT IS ALREADY MEASURED. DO NOT RE-DERIVE ANY OF IT.

Benchmark: `crates/pdfce-render/examples/region_bench.rs`, **release**, on
`D:\Dev\temp\pdfce\ncored-benchmark-cad-drawing.pdf` — A3 landscape,
**148,517 paints · 24,128 clip ops**.

```bash
cargo run -q --release -p pdfce-render --example region_bench -- \
  D:/Dev/temp/pdfce/ncored-benchmark-cad-drawing.pdf
```

| fact | number | how |
|---|---:|---|
| FLOOR — 1×1 pt region, **2 px** | **~667 ms** | median of 3 |
| FULL page, scale 1 — 1,002,822 px | ~941 ms | |
| every `fill_path`/`stroke_path` removed | **~591 ms** | ablation, 8 sites, env-gated |
| ⇒ **painting is** | **~11 %** of the floor | |
| inside `Interpreter::paint` | **126.6 ms of 742.4 ms = 17.1 %** | direct timing |
| ⇒ **outside `paint()`** | **~83 %** | |

### ★ The number that determines the design

**~83 % is spent in the operator loop, not in the paint call.** That is
tokenizing, dispatch, graphics state — and **`PathBuilder` pushes**. A path is
built **incrementally by `m` / `l` / `c` / `re`**, so its construction is
spread across thousands of operators and is *not* inside `paint()`; only
`builder.finish()` is.

⇒ **The display list must store FINISHED PATHS**, and the recording point is
where a finished path first exists as a value. A cache keyed any earlier would
have to cache the operator stream and rebuild the path — which is most of the
cost it exists to avoid.

**Ceiling this sets:** a handle removes ~591 ms of the ~667 ms floor. The
residual ~76 ms is `fill_path` setup for all 148,517 paints, and **a bbox cull
at replay removes that** — cheap there, because the bounds are already
computed. **Culling belongs in the replay path; it is not an alternative to
the handle.**

### Two things already tried and correctly rejected — do not re-propose

1. **A cull at the paint site.** `paint_is_cullable` exists in `interpret.rs`
   and feeds **only a counter**. `profile.rs` says why, at the field: *"Measured
   at 1.34 % on the reference CAD sheet, which is why no such cull was built.
   Kept as a counter so the next person to propose one gets the number instead
   of the intuition."* It worked — this session proposed exactly that cull and
   got the number. Note the two culls test **different rectangles** (clip bbox
   vs region) and are not substitutes.
2. **A second, lighter interpreter for recording.** That is two rendering paths
   for one content type — the trap this project has written down three times.
   **Instrument the real interpreter.**

Full write-up including the method notes: **`docs/render-region-measurements.md`
§4a**.

---

## §2 — THE DESIGN (option b)

### 2.1 The `Canvas` indirection

The interpreter threads `&mut Pixmap` through **16 signatures**. Replace that
parameter with `&mut Canvas`, which either **paints** (today's behaviour,
byte-for-byte) or **records**.

**The surface `Canvas` must offer — measured, this is all of it:**

| used | count | note |
|---|---:|---|
| `pixmap.width()` / `.height()` | 13 each | trivial forward |
| `pixmap.fill_path(...)` | 5 | **record** |
| `pixmap.stroke_path(...)` | 4 | **record** |
| `pixmap.draw_pixmap(...)` | 1 | group composite — §2.4 |
| `Pixmap::new(w, h)` | 2 — `:3124`, `:3972` | group / soft-mask buffers |
| `buf.as_ref()` | 1 — `:4007` | group composite source |

**The 16 signature sites**, as `pixmap: &mut Pixmap` parameter lines at
`d6c524a` — re-grep rather than trust these if the file has moved:

```bash
grep -n "pixmap: &mut Pixmap" crates/pdfce-render/src/interpret.rs
```

`1121 · 1271 · 1392 · 1550 (execute) · 2135 (show_array) · 2165 (show_string)
· 2239 (paint_glyph) · 2866 (shading_operator) · 3330 · 3504 · 3663
(do_xobject) · 3775 (do_form) · 4043 (do_image) · 4063 (draw_image) · 4202
(paint_image) · 4365 (the parameter of `fn paint`, which starts at 4363)`.

**★ Do the type change FIRST, mechanically, with `Canvas` forwarding
everything — and commit it on its own.** Run the full suite and the parity
harness at that point. A green run proves the indirection is transparent, and
every later bug is then in the recorder rather than in the plumbing. **Do not
add recording in the same commit as the plumbing.**

### 2.2 What a record must capture

```text
Fill   { path: Arc<Path>, ctm, colour, alpha, blend, rule,         clip: ClipId, bounds }
Stroke { path: Arc<Path>, ctm, colour, alpha, blend, stroke_params, clip: ClipId, bounds }
Image  { texels: Arc<Pixmap>, ctm, quality, blend, anti_alias,      clip: ClipId, bounds }
```

`tiny_skia::Paint<'a>` **borrows** its shader, so it cannot be stored —
decompose into owned parts and rebuild at replay. `bounds` is the path's
bounds under `ctm` **in page space**, and is what the replay cull tests
against the requested region.

### 2.3 Clips are the hard part — record PATHS, not masks

`GraphicsState::clip` is `Option<Arc<tiny_skia::Mask>>`, and a `Mask` is
**device-sized**. A recorded mask is valid only for the pixmap geometry that
built it, so panning or zooming invalidates it.

**Record clip definitions instead** — `ClipDef { path: Arc<Path>, rule, ctm,
parent: Option<ClipId> }` — and rebuild masks at replay. That is affordable
because the existing clip cache already serves **99.83 %** of applications on
this sheet, so a replay pays ~41 builds, not 24,128.

### 2.4 Where to poison rather than guess

Anything the recorder cannot reproduce **faithfully** must mark the list
unusable **by name**, with the reason retained. Candidates: soft masks,
transparency groups (the `draw_pixmap` composite), overprint composites (they
read destination pixels back), patterns, shadings.

**A display list that renders *nearly* right is worse than none** — criterion 3
is byte-identity.

**The motivating document needs none of them.** Measured on the benchmark page:
`images=0 shadings=0 patterns_unpainted=0 blend_modes_applied=0
soft_masks_applied=0 groups_composited=0` — it is **pure paths and text**. So a
recorder handling fills, strokes and glyph paints with clips wins the entire
benchmark, and poisoning covers everything else honestly.

---

## §3 — ACCEPTANCE CRITERIA (`ROADMAP.md` `Pass 75.0`, condensed — read the entry too)

1. **The measurement IS the criterion**, so it cannot ship while still slow.
   Second-and-subsequent region renders of an **unchanged** page: **~700 ms →
   tens of ms**, verified with `region_bench` **extended with a repeat case**.
   The **first** render may cost what it costs.
2. **Keyed on `(page, epoch)`**; a stale handle is **impossible to use
   silently** — either unrepresentable, or refused by name.
3. **Byte-identical output** versus a render without the handle at the same
   scale. **Extend** `crates/pdfce-render/tests/region_matches_full_page.rs`,
   do not duplicate it.
4. **Memory measured and documented.** A held list for 148,517 paints has a
   size; a shell holding one per page needs that number.
5. **No first-render regression on the text case** — `iso32000-2-preview.pdf`,
   ~9 ms full page. A build cost exceeding the saving is a loss on every
   document where interpretation is already cheap, which is most of them.
6. `cargo tree -p pdfce-render` GUI-free; `cargo fmt --all`;
   `cargo clippy --all-targets -- -D warnings` clean.
7. **`pdfce-cli` (rule 11): decide `—` vs `[ ]`, and write the reason down.** A
   display list is an in-process, across-frames object and a one-shot CLI has
   nothing to hold it across, so `—` is probably right — but **decide it in the
   Pass**. `FEATURES.md`'s legend exists to keep those two apart.

**The consumer asked for `(page, epoch)` keying and explicitly declined a batch
N-region call** as *"strictly less useful — their regions arrive one per frame
as the operator moves, not in batches."* Honour both.

---

## §4 — TRAPS THAT COST TIME TODAY

- **A single-run ablation measures machine load as much as code.** The first
  paint-ablation run said **36 %**; three interleaved runs with medians said
  **11 %**. The first number would have been written down.
- **The `paint()` timer's first reading said 164 %**, because the counter
  accumulates across renders and the report ran after the FULL cases. Obviously
  wrong — and the failure mode is halving it into something plausible.
- **★ A regression test that cannot fail is not a regression test.** Today's
  seam test **passed with the fix reverted** (its sampling column landed on the
  tiles' edge). **Run every new regression test against the pre-fix code.** If
  that means temporarily sabotaging the fix, do it — and revert by editing the
  line back, **never** with `git checkout` (agent-memory note).
- **`cargo fmt` collapses a `\` line-continuation inside a string literal and
  bakes the padding in as literal spaces.** It surfaced only by running the
  binary and reading the message. Grep for runs of 3+ spaces inside literals
  after any string edit.
- **Read `ARCHITECTURE.md` §12's decision-log tail every session.** Skipping it
  cost a dependency recommendation that contradicted a day-old cross-project
  boundary decision (`iccce` owns colour conversion, decision 064).

---

## §5 — WHAT SHIPPED 2026-08-18 (14 commits, `e618d67`..`d6c524a`)

| | |
|---|---|
| `Pass 82.1` | `Polygon` ≥ 3 / `PolyLine` ≥ 2, arms split, named `TooFewVertices` |
| `Pass 82.0` | **Revision clouds** — `MarkupSpec::Cloud` + `Square { border_effect }`, baked `/AP`, `pdfce-cli annotate --cloud` |
| `Pass 99.0` | **`EditSession::insert_pages`** — session-mutating, undo survives |
| `Pass 100.0` | **The image-tile seam** — 15,253 seam px → 0 on the CAD corpus |
| minted, not started | `Pass 97.0/97.1/97.2` (the compositor), `Pass 98.0` |
| corrections | four stale-claim fixes, including the seven-copies sweep that produced librarian **hard rule 11** |

**Ghent standing unchanged: 25 pass / 18 FAIL / 8 UNRESOLVED of 51.**

---

## §6 — THE REST OF THE QUEUE

- **`Pass 97.0 / 97.1 / 97.2`** — the colorant compositor. **~16 of the 18
  remaining Ghent failures**, the highest-impact item in the project. Plan of
  record: `docs/compositor-plan.md`; the collapse model is sourced in
  `docs/collapse-model-survey.md`.
- **`Pass 80.0`** (note text on markup) and **`Pass 81.1`** (markup opacity,
  write half) — both `pdfceGUI` requests, both already scoped.
- **`Pass 98.0`** — read a foreign `/BE` back into `MarkupSpec`, so an
  Acrobat-authored cloud survives a pdfce restyle.
- **Two `iccce` requests owed and UNREAD** —
  `request_profile_population_census.md` and
  `request_header_tag_channel_disagreement.md` in
  `D:\Dev\FeatureRequests\iccce_FeatureRequests\open\`. **That channel is
  outside the repo, so no pdfce gate will ever remind you it exists.**
- **`pdfce_FeatureRequests/open/` is EMPTY** — nothing owed to `pdfceGUI`.

**v0.7.0 is bumped but NOT tagged.** `git describe` = `v0.6.0-70-gd6c524a` —
**70 commits since v0.6.0**. The operator gave a standing go-ahead for
builds/releases on 2026-08-17. Verify CI green on `HEAD`, then
`verify-release.py` → tag → portable package → GitHub release → librarian
release record.

**The GWG Reference file is still NOT on this machine** —
`Ghent_PDF-Output-Test-V50_ALL_REFERENCE.pdf`, the only oracle bearing on the
8 UNRESOLVED patches. Re-fetching is an operator call (`LEGAL.md` §5).

---

## §7 — STATE AT HANDOFF

- Gates **all clean**: `commits-filed`, `ledger-numbers`, `passes-filed`,
  `ui-strings`, `disclosure-channel`, `one-commit-per-command`.
- Working tree **clean**. Both measurement instrumentations reverted; the tree
  is byte-clean against `HEAD` apart from committed documents.
- Ledger: next free Pass family **101**, decision **071**, standing rule
  **R196**, filing ordinal **182**.
