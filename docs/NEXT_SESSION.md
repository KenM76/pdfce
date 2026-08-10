# NEXT SESSION — start here

**Rewritten 2026-08-10 (second time that day).** Read this, then
`docs/ROADMAP.md` and the latest `docs/SESSION_LOG.md` entry. This is a
*handoff*, not a record — the record is the librarian's. Overwrite it
once acted on.

Not owned by `pdfce-librarian`. Safe to edit without racing a filing.

---

## State

- Branch `pass-8-redaction`, HEAD **`77e0b50`**.
- **2834 workspace tests, 0 failed.** clippy 0 with `--all-features`,
  `cargo fmt --all --check` clean.
- **Seven of eight gates green.** `check-commits-filed.py` is RED on the
  most recent commits — file them; that is the normal state after a run
  of work, not a defect.
- **92 commits unpushed**, and `main` upstream still does not compile on
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

Seven gaps were verified absent IN SOURCE. **All seven are now closed** —
the campaign is complete:

| Gap | State |
|---|---|
| Find | **DONE** — core + CLI + GUI bar + on-page match highlighting |
| Printing | **PARTIAL** — `list-printers`, `print-preview`; **spooling deliberately unbuilt** |
| Bookmarks | **DONE** — core + `list-outline` + a clickable panel |
| Attachments | **DONE (listing)** — core + `list-attachments`; extraction BLOCKED, see below |
| Read mode / full screen | **DONE** — two separate toggles, Ctrl+H and F11 |
| Document layers (OCG) | **DONE (read-only)** — core + `list-layers` + panel; no toggle, see below |
| Signature validation | **NOT STARTED**, lowest priority |

### Layers: shipped, with one piece of debt

`pdfce-core::layers` is registered, tested, and has both surfaces. It
reuses `annot.rs`'s `optional_content_default_off` rather than
reimplementing it, so the panel cannot say "shown" about content the
renderer hides.

**The one duplication left**: `annot::oc_refs` is private, so
`layers::group_refs` reimplements it. Make the original `pub(crate)` and
delete the copy.

**Two things the layers subagent flagged for a decision, not yet made:**
`optional_content_default_off` treats "all groups" as "everything in
`/OCGs`", so an unregistered group under `/BaseState /OFF` reports
visible where the spec would say hidden — it kept the renderer's answer
(agreement beats purity) and set a diagnostic. And Table 101 vs
§8.11.4.5 disagree about processing both `/ON` and `/OFF` after
`/BaseState`. Both fixes belong in `annot.rs` so all surfaces move
together.

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
  `panel:bookmarks`, `panel:layers`, `view:read`, `view:escape`. **Full
  screen is deliberately NOT drivable** — a step that seized the
  operator's display would defeat the reason the harness exists.

### THERE IS NO `nav:` PREFIX — every navigation key is `key:`

`key:left`, `key:right`, `key:up`, `key:down`, `key:home`, `key:end`,
`key:enter`. **`nav:home` and `nav:enter` are not steps and never were.**

This cost two false bug reports on features that worked — Find's Enter
(twice) and the keyboard guard's verification. Both tests sent
`nav:something`, so neither ever pressed a key.

**ALWAYS include the harness's own warning channel in `-Filter`.** It
traced `script-step-UNPARSEABLE step="nav:enter" skipped=1` on every
single run; the filters matched only the traces each test expected, so
the explanation never appeared. A filter that matches only your
expectation cannot tell you your input was wrong. Use something like:

    -Filter "UNPARSEABLE|stale|<the thing you actually want>"

**And when a trace reads zero, walk upstream before concluding.** The
sequence that found it: trace the condition's parts separately; all
false, so is the event in the frame at all; zero, but the probe sat
AFTER the consumer, so move it before; still zero, so is the apply arm
reached; never reached, so read the parse table. **A probe placed
downstream of the suspect measures its output, not its input.**

---

## THE NEXT TASK — ranked

### 1. The campaign is COMPLETE — pick the next thing

All seven gaps have a surface. What the sweep left owed, in order:

- **Printing's spooling half** (item 3 below) — needs an operator
  go-ahead before any job reaches paper.
- **Attachment extraction** — blocked on `/EFF`, see above.
- **A layer toggle** — needs a renderer visibility override AND a
  session-state model. `layers.rs` is read-only by design, which is
  correct: a checkbox with no engine behind it is worse than none.
- **Signature validation** — the one gap never started.

### 2. Owed items on Find

- **Whole-word matching is absent, not faked.** `TextMatch` returns only
  the matched substring with no surrounding context, so a shell-side
  boundary check has nothing to check against. A `pdfce-core` gap — the
  fix is to return context, not to guess at a boundary.
- ~~Enter does not trigger the search~~ — **it always worked**. See the
  harness section below; corrected in `1bf6ab2`.
- ~~Match highlighting is not drawn~~ — **drawn** in `4810b49`, through
  the existing `pdf_space_to_canvas` + `page_to_screen` pair.

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

### 4. ~~`R86` is NOT discharged for `e46c3a8`~~ — DISCHARGED

Verified in `77e0b50`: `query="hello"` then `key:home` then
`query="Xhello"`, with `pre-collect-home n=2 typing=true` measured
upstream of `collect_keyboard_actions`. The guard works and the key
reaches the field.

The "harness question" had the same answer as Find's Enter — below.

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
