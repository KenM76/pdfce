# NEXT SESSION — start here

**Rewritten 2026-08-10 (second time that day).** Read this, then
`docs/ROADMAP.md` and the latest `docs/SESSION_LOG.md` entry. This is a
*handoff*, not a record — the record is the librarian's. Overwrite it
once acted on.

Not owned by `pdfce-librarian`. Safe to edit without racing a filing.

---

## State

- Branch `pass-8-redaction`, HEAD **`42b86cd`**.
- **2804 workspace tests, 0 failed.** clippy 0 with `--all-features`,
  `cargo fmt --all --check` clean.
- **Seven of eight gates green.** `check-commits-filed.py` is RED on the
  most recent commits — file them; that is the normal state after a run
  of work, not a defect.
- **86 commits unpushed**, and `main` upstream still does not compile on
  Linux (`ef88973` fixes it, locally). Pushing is permitted on request
  and is the operator's call, never an agent's. **R176.**
- Baseline debt: **5 hashes** in `tools/commits-filed-baseline.txt`. The
  file is 25 LINES — twenty of them are its header. Count the hashes
  (`grep -cE '^[0-9a-f]{7,40}'`), not the lines; `wc -l` gives 25 and is
  wrong.

---

## ★ THE ACTIVE CAMPAIGN — Reader-parity sweep (decision 036)

Chosen by the operator on 2026-08-10 after an audit found the gap
**inverted** from what the roadmap had assumed: pdfce is well AHEAD of
Acrobat Reader on editing — text, vector objects, redaction, form
authoring, page operations — and was BEHIND on plain consumption.

Seven gaps were verified absent IN SOURCE. Six are now closed:

| Gap | State |
|---|---|
| Find | **DONE** — core + CLI (`find-text`) + GUI bar, `Pass 55.0` |
| Printing | **PARTIAL** — `list-printers`, `print-preview`; **spooling deliberately unbuilt** |
| Bookmarks | **DONE** — core + `list-outline` + a clickable panel |
| Attachments | **DONE (listing)** — core + `list-attachments`; extraction BLOCKED, see below |
| Read mode / full screen | **DONE** — two separate toggles, Ctrl+H and F11 |
| Document layers (OCG) | **IN FLIGHT** — see the next section |
| Signature validation | **NOT STARTED**, lowest priority |

### The layers module is written but NOT REGISTERED

`crates/pdfce-core/src/layers.rs` exists (~1969 lines) with nine fixtures
under `fixtures/synthetic/layers/`, written by a subagent that had not
reported when this file was written. **It is not in `lib.rs`.** Register
it, run the tests, wire `list-layers` in the CLI and a panel in the GUI.

The hard part already existed and the agent was told to reuse it rather
than reimplement: `annot.rs`'s `optional_content_default_off` resolves
`/OCProperties /D` including `/BaseState`/`/ON`/`/OFF`, and `oc_is_hidden`
resolves an `/OC` against it. If the agent duplicated them because they
were private, **make the originals `pub(crate)` and delete the copy** —
two resolvers disagreeing means a layers panel that says "on" about
content the renderer hides.

Per the Acrobat RAG: toggling layers in a viewer is **session-scoped with
zero file-format footprint** unless the operator explicitly saves, and
**locked layers** cannot be toggled at all.

---

## ★ HARD BLOCKER — attachment extraction and `/EFF`

**Do not build an extraction surface without reading this.**

Since PDF 1.5 an *otherwise unencrypted* document can carry **encrypted**
embedded files via `/EFF` + `DefEmbeddedFile` (§7.6.5). The intuitive
guard — no password prompt, so plaintext — is wrong, and wrong
**silently**: the filter chain runs and returns garbage that looks like a
successful read.

`AttachmentNotes::may_be_encrypted` is a deliberately over-broad warning
flag, not a fix. `list-attachments` already surfaces it as
`MAY_BE_ENCRYPTED` plus a stderr warning. Any future extraction MUST
refuse or loudly caveat while it is set. Filed as a Backlog blocker with
acceptance-criteria language, not as a doc comment.

---

## Verification is TEXT-ONLY while the operator is at the machine

Stated by the operator on 2026-08-10: he is using this PC.

- **`tools/gui-shot.ps1` is OFF LIMITS** — it raises a real window and
  grabs the foreground.
- **`tools/gui-drive.ps1` is fine** — it runs the window off-screen at
  (-4000,-4000) and never takes focus. This is the situation `diag.rs`
  was written for; its module docs say so.
- Drive it with **`pwsh`**, not `powershell`. Both parse the scripts now,
  but 5.1 leaves `$Exe` unresolved in that one file for reasons that were
  investigated and not identified.
- New diag steps this session: `scroll:<points>`, `panel:find`,
  `panel:bookmarks`, `view:read`, `view:escape`. **Full screen is
  deliberately NOT drivable** — a step that seized the operator's display
  would defeat the reason the harness exists.

---

## THE NEXT TASK — ranked

### 1. Land the layers module (above), then its CLI and panel

### 2. Owed items on Find, all named and none silent

- **Enter does not trigger the search.** Two idioms tried —
  `has_focus()` + key peek, then `lost_focus()` like every other draft in
  the app. Both silently no-op while the trace shows `focus=true
  query="third"`. The explicit **Find** button is the working path, and
  exists precisely because Enter must never be the only way in.
- **Whole-word matching is absent, not faked.** `TextMatch` returns only
  the matched substring with no surrounding context, so a shell-side
  boundary check has nothing to check against. A `pdfce-core` gap.
- Match highlighting on the canvas is not drawn. `TextMatch.quad` is in
  page space; invert `pdfce-render`'s Transform rather than hand-deriving
  a second mapping.

### 3. Printing: the spooling half

Everything up to the job exists — `printer_caps` reads DPI and printable
area, `place_page` computes scale and offset and reports clipping.
**Printing consumes paper and occupies a shared device; get an explicit
go-ahead before spooling anything.**

Two decisions already recorded, not yet acted on: the first slice
rasterises (Reader sends vector to the driver and RIPs at print time —
pdfce making Reader's *fallback* the default is an honest limitation that
belongs in the disclosure), and a resolution cap is needed because A4 at
600 DPI is ~139 MB of RGBA per page.

### 4. `R86` is NOT discharged for `e46c3a8`

The unmodified-key guard is right by two independent code analyses —
mine and `pdfce-ui-specialist`'s, which found the codebase's own comment
at `main.rs:17107` stating the ordering. But the empirical test does not
work: type `hello`, press Home, type `X` gives `helloX` **both before and
after** the guard, while a trace confirms the guard is active. egui 0.35
does handle `Key::Home` (`text_selection/cursor_range.rs:134`) and the
harness injects real events. **A harness question, not a product one** —
but until it is answered, that commit's verification claim stays "not
verified".

---

## Things learned this session that will save the next one time

**Read the output as its audience, every time.** Five defects this
session were invisible to green tests: a rich-text summary listing
`bold, 12pt, Helvetica, #FF0000, italic` with the two facts an operator
compares three items apart; outline diagnostics burying `cycles_broken=3`
in a wall of nineteen zeros; `kind=` printing UTF-16BE bytes where a name
belonged; a gate whose own failure message rendered `?` for its
em-dashes; and a build changelog mangling every commit subject.

**Windows defaults to cp1252 everywhere UTF-8 is not demanded, and each
layer must be told separately.** Three instances, three different
mechanisms — stdout encoding (Python gates), source encoding (PowerShell
would not parse), subprocess capture (`git log` via
`subprocess.run(text=True)`). Knowing the first two did not prevent the
third.

**Grep the RAG before FIXING, not before documenting the fix (R182).** I
fixed a `.ps1` parse failure with BOMs; `personal_rag`'s existing lesson
rules against BOMs by name. I found it while writing up — after the wrong
change was committed.

**`git add -A` and parallel agents do not mix.** It swept two subagents'
in-progress modules into an unrelated commit. Nothing was lost, but one
agent had to verify its own work had not been clobbered. Stage by
explicit path while they run.

**Dispatch the subagents; they earn it.** `pdfce-ui-specialist` caught
three things before they shipped, including a `&mut self` call I was
about to put in a render path. Both module-writing subagents dispatched
`pdfce-spec-librarian` unprompted on finding the spec RAG had zero
coverage of their clause — which produced eight new spec entries and a
correction to an existing one.

**Check a number before asserting it.** Three times this session I nearly
shipped a measurement I had not taken: a `grep -c` of textual occurrences
reported as raise sites (42 vs 26), a `$PSScriptRoot` cause my own probe
disproved, and `wc -l` on a file whose header is 80% of its lines.
