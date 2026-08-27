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

**The previous session shipped three Passes and closed four operator
reports.** Every one of them came from Ken or from the `pdfceGUI` channel
looking at something on a screen; **none was found by a gate.**

- **`Pass 137.0`** (`523ca6d`) — **a gradient and a solid of the same ink
  were different colours.** Analytic shadings now composite in authored ink
  whether or not overprint is in force.
- **`Pass 137.1`** (`d1ce4ac`) — **the mesh half, which `137.0` structurally
  could not reach.** A mesh has no `ColorRamp`; `Shade::Ink` is the carrier
  it needed. Overprint for meshes came along free.
- **`Pass 138.0`** (`6256c93`) — **the marquee and the measure tool still
  could not see inside a form XObject.** Three `pdfceGUI` requests, all
  closed, plus one judgement call they explicitly asked the engine to make.

### ★ Verified from a shell at write time — re-run, do not copy forward

| fact | value | command |
|---|---|---|
| `HEAD` | `51c30d6` | `git rev-parse --short HEAD` |
| `git describe --tags` | `v0.14.0-39-g51c30d6` | `git describe --tags` |
| `origin/main` | **LEVEL — everything is PUSHED** | `git rev-list --count origin/main..main` |
| tag at `HEAD` | none; highest is `v0.14.0` | `git tag --points-at HEAD` |
| working tree | **clean** | `git status --porcelain` |
| tests | **4,385 passing, 0 failing** | `cargo test --workspace --release` |
| gates | **19 on disk; 18 run with no arguments; all green** | `ls tools/check-*` |
| `fmt` / `clippy` | clean | `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` |
| fuzz | all targets compile | `cargo +nightly fuzz build` |
| wasm32 | `pdfce-core` + `pdfce-render` compile | `cargo check -p pdfce-core -p pdfce-render --target wasm32-unknown-unknown` |
| GUI-core invariant | no GUI dep in either engine crate | `cargo tree -p pdfce-core` |
| print-conformance | **5 FAIL, 35 pass, 11 UNRESOLVED** | see `§C` |
| **CI at `51c30d6`** | **NOT READ — read it from GitHub before believing anything about it** | `gh run list --branch main --limit 3` |

★★★ **PUSHING NO LONGER NEEDS TO BE ASKED — decision `090`, 2026-08-27.**
Ken's ruling, verbatim and in full: ***"always push."*** An ordinary
fast-forward push of `main` is **standing-authorized**; do not ask again.

★ **Three things it does NOT cover**, narrowed deliberately rather than read
generously, with the reasoning in decision `090`:

- **cutting a tag or a release** — a release claims a state is *fit to use*,
  which is a different act from making commits visible;
- **`git push --force`, or any push that rewrites published history** — that
  *removes* commits from a public repository and is unrecoverable for anyone
  who has cloned. ★ Not hypothetical: `check-cited-commits-exist.py` exists
  because rewriting commits broke fourteen document citations;
- **any branch other than `main`**, or creating remote branches/tags.

★ **Scrub the public-facing gate green before pushing regardless.** The
repository is public, so a push publishes (`LEGAL.md` §1.1).
`tools/check-suite-name-absent.py` scans **untracked** files too — which is
why scratch renders belong in `D:\Dev\temp\`, never in the tree.

---

## §B — THE DISK

**35 GB free of 954 GB (97 % used).** `target/` is 9.7 GB and `fuzz/target/`
is 1.1 GB; both are regenerable and both were left in place, because the next
session will want them warm and a cold `cargo test --workspace --release` is
several minutes.

If you need space: `cargo clean` and `cargo clean --manifest-path
fuzz/Cargo.toml` recover ~11 GB between them. Last session's scratch renders
under `D:\Dev\temp\pdfce\` were **already deleted** at shutdown; put new ones
there, never in the tree.

---

## §C — THE FIVE REMAINING CONFORMANCE FAILURES, AND THE ONE THAT IS SCOPED

Run: `python tools/suite-check.py <corpus> --reference-dir <refs>` — the
private map names both directories. Current standing:

★ **Patch stems are deliberately NOT listed here** — operator ruling
2026-08-25, enforced by `tools/check-suite-name-absent.py`, which scans
untracked files as well as tracked ones. The gate caught this table on its
first draft. Get the stems from the harness's own output; they are stable and
the private map names the corpus.

| what it tests | traps | note |
|---|---:|---|
| spot-colour overprint | 6 | |
| white overprint | 5 | |
| **five-colorant `DeviceN` image** | 1 | ★ **diagnosed — see below** |
| ICC source profile | 4 | colour management |
| blend modes in an ICC RGB group | 12 | |

### ★★★ THE FIVE-COLORANT `DeviceN` ONE is diagnosed and NOT started — start here if you want a clean win

**The trap detector fires on a photograph, and that firing is a red herring.**
The real defect is visible to the eye: a **five-colorant `DeviceN` photograph
renders visibly desaturated** — washed toward grey where Acrobat's reference
is saturated green.

Measured on that page: `cmyk_bridged_pixels = 25870`,
`cmyk_native_image_pixels = 0`. The image is bridged through sRGB on a page
that composites in ink.

**Cause**, in `crates/pdfce-render/src/image.rs`:

```rust
let carries_ink =
    matches!(space, Space::Cmyk) || matches!(space, Space::Indexed { ink: Some(_), .. });
```

A `Separation`/`DeviceN` image converts through its tint transform **to
sRGB** and never to its `DeviceCMYK` alternate, so it round-trips.

**★ This is the FOURTH instance of `R219`'s shape in four days, and it was
found by applying `R219` rather than by a bug report:**

| Pass | gave authored ink to | what it left |
|---|---|---|
| `130.1` | `DeviceCMYK` images | `Separation`/`DeviceN` images |
| `130.2` | `Separation`/`DeviceN` images **only under overprint** | the same images outside overprint |
| `137.0` | analytic shadings, past their overprint gate | mesh shadings |
| `137.1` | mesh shadings | ← **this row is the gap nobody went back for** |

**★★ DO NOT REUSE THE OVERPRINT PLANES.** They already exist, texel for
texel, in the same `CmykTexels` layout, and reaching for them is the obvious
move and the wrong one:

- they hold `authored_tints`, which answers Table 149's *"which components did
  the source SPECIFY"* — **a different question** from *"what does the tint
  transform produce in the alternate space"*;
- `authored_tints` returns `None` for a **spot-only** `DeviceN`, where the
  existing code writes `[0,0,0,0]`. That is correct for the overprint route,
  which preserves the backdrop (`Pass 130.3`), and would paint **bare white
  paper** on a plain one.

**The right shape** is a per-texel `space.to_cmyk(comps, diag)` captured
**beside** the existing `to_rgb`, in the same loop and from the same
operands — exactly what `ColorRamp::new` and `mesh::read_shade` do, and for
the same reason: a tint transform may be arbitrary PostScript and nothing
forces two evaluations of it to agree.

**Measure the cost first.** That is a second evaluation per texel unless the
sRGB value is derived from the CMYK one. `tint_applied = 292` on the measured
page suggests a cache is doing real work already; find out what it is keyed
on before adding a second lookup.

### The instrument caveat, which cost time this session

`find_traps` fired at `(324, 50)` on a **photographic highlight**, not on a
trap X. The reference render tripped zero traps there — but the reference is
`786 × 439` where pdfce's is `511 × 284`, so the detector ran at a different
resolution and **that control is weaker than it looks**. The diagnosis above
came from cropping both renders, scaling one to the other, and looking.

⇒ On a patch whose content is a photograph, the trap count is not the
evidence. The pixels are.

---

## §D — WHAT LAST SESSION DECIDED, IF YOU TOUCH THIS GROUND

### Shading and mesh ink (`137.0`, `137.1`)

- A shading with authored colorants composites natively **whether or not
  overprint is in force**. The `DeviceCmykDirect`-under-`OPM 1` exclusion
  still applies to the *overprint* route only, because Table 149's `OPM 1`
  row is **value-dependent** and its rules cannot be computed once for a
  whole ramp.
- `Shade::Ink` carries **both** the converted sRGB value and the authored
  colorants, and interpolates them **independently**. Consequence, stated so
  it is not discovered: where the conversion is non-linear the two paths
  differ in a triangle's *interior* while agreeing at every vertex. ISO
  32000-1 §8.7.4.5.5 picks neither; both are defensible.
- `MeshColorants` is decided **once, at parse, all-or-nothing**. A mesh with
  ink at some vertices and not others would composite part of its area
  natively and part bridged, and those do not agree — the boundary would be a
  **seam no file asked for**.

### Form recursion, second wave (`138.0`)

- `hit_test_rect_deep` returns **paint order, front-most LAST** — the
  *opposite* of `hit_test_point_deep`. Deliberate: one answers *"which
  one?"*, the other *"which ones?"*.
- `FormMarquee::Exclude` is the default. **The tie-breaker is not which
  reading is better supported** — it is that a click can never yield a form,
  so a marquee that can hands the operator, by one gesture and not the other,
  a selection every edit verb refuses.
- `FormMarquee::Include` returns the form **and** its leaves. It is **not** a
  route back to the shallow `hit_test_rect`, which returns the container
  alone.
- `PickedLine::object_index: usize` → `target: HitTarget`. **The `usize` was
  the bug**: it could only name an entry in one of two lists, so the
  signature made an answer about form contents unrepresentable — which is why
  the two-line measure tool was **inert**, not degraded, on any wrapped CAD
  drawing. `page_object_index()` returns an `Option`, not a sentinel, because
  a leaf ordinal handed to something expecting a page index is a number that
  is *in range and wrong*.
- `object-list --hit` is **deep by default**; `--hit-scope page` is the old
  query. A leaf prints under `leaf=N`, **never** `index=N` — `index=` is what
  the editing subcommands take, and they write to the *page's* stream.

---

## §E — TWO HABITS THIS SESSION PAID FOR

### `R219` is the highest-value thing on the board

*When a Pass fixes one route to a shared behaviour, enumerate the other
routes and say which are left.* Minted after **three** instances in
seventy-two hours; `§C` above is the **fourth**, and it was found by applying
the rule rather than by waiting for a report.

**The reason it keeps happening:** a system where every route is wrong in the
*same* way looks consistent. The first fix is what makes the rest look
broken. That is an argument **for** fixing halves — the disagreement is
information a passing suite was not giving you — but the leftover half
becomes urgent in a way it was not before, and it will reach the operator
first if you do not name it.

### A crop rectangle is a measurement instrument

`Pass 137.0` shipped with a four-row table of live-vs-reference distances
that **mixed two panels**, labelled a fixed type 3 radial as an unfixed mesh,
and reported **edge antialiasing on a hard-edged circle as a colour error**.
Corrected in place in `ROADMAP.md` with the wrong figures kept legible.

Before quoting any number off a render: find the region by **scanning for
non-white runs**, inset 6–8 px, and read **mean-abs, rms and the two mean
colours together**. High mean-abs with matching mean colours and high rms is
*misalignment*, not a colour error.

### And a doc comment that enumerates a population decays silently

`cmyk_bridged_pixels`'s description was wrong **three times in two days**,
each correction written by somebody who had just read it and believed it.
The **sixth** copy — the one `render-page` prints to the operator's terminal
— was still wrong a day later and was found by running the tool on a real
file, not by grepping. **Nothing in this repository checks operator-facing
strings for claims about implementation state.** If that recurs, a gate is
the fix.

---

## §F — OPEN, UNCHANGED

- **`(bl)`** — may a **CC-BY-SA-4.0** OCR model ship inside pdfce's **MIT**
  portable folder? Ken's call; default if unanswered is **ship neither model
  set**. `docs/ocr-engine-survey.md`.
- **`R13` vs "download addin capability"** — downloading is permitted;
  **executing** what was downloaded is not, and that ruling is owed from the
  operator. **No add-in Pass can be scoped until it lands.**
- **`(p)`** — whether to narrow the XFA item to read/fill only, or retire it.
- The `pdfceGUI` reply for `Pass 138.0` is filed at
  `D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\note_all_three_form_gaps_closed_and_one_judgement_made.md`.
  **It contains a retraction owed to them**: they were told two type 7
  shading pairs would still disagree, and after `137.1` they do not.
