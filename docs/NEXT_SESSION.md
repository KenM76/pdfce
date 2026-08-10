# NEXT SESSION — start here

**Rewritten 2026-08-10 (third time that day).** Read this, then
`docs/ROADMAP.md` and the latest `docs/SESSION_LOG.md` entry. This is a
*handoff*, not a record — the record is the librarian's. Overwrite it
once acted on.

Not owned by `pdfce-librarian`. Safe to edit without racing a filing.

---

## State

- Branch `pass-8-redaction`, HEAD **`24e07e3`**.
- **2902 workspace tests, 0 failed.** clippy 0 with `--all-features`,
  `cargo fmt --all --check` clean. `check-ui-strings.sh`,
  `check-theme-colors.sh` and `check-disclosure-channel.sh` clean.
- **117 commits unpushed**, and `main` upstream still does not compile on
  Linux (`ef88973` fixes it, locally). Pushing is permitted on request
  and is the operator's call, never an agent's. **R176.**
- Baseline debt: **5 hashes** in `tools/commits-filed-baseline.txt`. The
  file is 25 LINES — twenty of them are its header. Count the hashes
  (`grep -cE '^[0-9a-f]{7,40}'`), not the lines; `wc -l` gives 25 and is
  wrong.
- A `pdfce-librarian` filing for the theme + split commits was dispatched
  at the end of the session and may not have landed. Check
  `git status` before assuming `docs/` is clean.

---

## ★ WHAT THE OPERATOR IS WAITING TO DECIDE

**Three visual themes ship and he has not chosen one.** He asked whether
the GUI's look would be improved and whether it could be changed without
a refactor; the answer was "not yet", and the session built the layer
that makes it yes. He then set the order himself — *"1 then 4 then 3"*:
theme extraction, then split `main.rs`, then back to features. Both are
done. For visual direction he answered **"show me options first"**.

So: `Settings → Appearance` has **Quiet / Airy / Dark**, applying live.
**No visual redesign has happened** — only the layer that makes one
cheap. When he reports back, acting on it is a `theme.rs` edit, not a
sweep.

**Do not restyle before he chooses.** The presets exist to make the
question answerable, not to pre-empt it.

---

## Two things blocked on the operator, unchanged

1. **Cryptographic signature verification** needs a crypto dependency —
   a licensing and scope decision under rule 13, his to make. Everything
   the Signatures panel shows today is arithmetic over `/ByteRange`, and
   it says so on screen.
2. **Printing** previews without spooling. A real job consumes paper on a
   shared device and is irreversible; it needs an explicit go-ahead.

---

## ★ DECISION 037 — ruled, fixture built, NOT implemented, and that order matters

`docs/decisions/037-base-state-off-covers-unregistered-groups.md` rules
that `/BaseState /OFF` covers groups missing from `/OCProperties /OCGs`.
pdfce currently answers the opposite (registered-only).

**Run the falsifier first.** Open
`fixtures/synthetic/layers/base-state-off-unregistered.pdf` in Acrobat
Reader and look at the **right-hand square**:

- **square absent** → Reader agrees with the ruling → implement it as a
  straight fix.
- **square present** → Reader agrees with pdfce's *current* behaviour →
  the standard is not adjudicating this, and 037 becomes setting
  **`OC-A2`** (`AllGroups` / `RegisteredOnly`, default `RegisteredOnly`,
  evidence tier (a)) instead.

It could not be run this session: the operator was using the machine and
`tools/gui-shot.ps1` raises a real window and takes foreground. Acrobat
Reader is installed (Reader only — not Pro).

`base_state_off_currently_leaves_unregistered_groups_visible` in
`crates/pdfce-core/tests/layers.rs` pins today's answer **on purpose**,
so the flip cannot happen quietly. Editing it is the step that forces
whoever implements 037 to confirm the falsifier ran.

**The refactor is right under either outcome.** 037's real finding is
that the answer should never have been a `BTreeSet`: a set of OFF groups
is complete only if its complement is genuinely ON, which fails under
`/BaseState /OFF`, where the true set is "every group in the document
minus `/ON`" and is not enumerable. A per-group `OcDefaultState`
resolver makes the question disappear and is *cheaper* than the
registry-walk it replaces. It reaches ~5 call sites
(`oc_is_hidden`, `interpret.rs`'s cache, `render/annot.rs`, the CLI, the
GUI) plus a defect **`LayerVisibility` inherits for the same reason** —
it is a bare set documented as "a COMPLETE answer", which under
`/BaseState /OFF` a set cannot be. Fix both together or the override
path reintroduces the bug the moment anyone toggles a layer.

Decision **038** is ruled AND implemented; its contingency was
discharged by a verbatim re-read. Nothing owed there.

---

## §8.11 is nearly finished

Shipped this session: content-stream `BDC`/`EMC` `/OC`, XObject `/OC`,
the operator visibility override, Table 99 `/P` policies, §8.11.2.3
`/Intent` filtering (with disclosure in both shells), and `/VE`
visibility expressions.

**Remaining: `/AS` + `/Usage` auto-state application only.** Confirmed by
a spec-librarian sweep against live source. `/RBGroups` is enforced by
the GUI panel and correctly out of scope for the renderer; `/Configs` is
correctly unused.

---

## Where the GUI is now

`main.rs` went **27,647 → 25,511** in three separately-revertable moves:
`canvas_overlay.rs` (749), `panels_structure.rs` (520), `ribbon_ui.rs`
(1,121). Pure moves — same 2,901 tests before and after.

If you continue splitting, the next natural seams are the **form-field
panel and its helpers** (`forms_panel` at ~7840 through the field-draft
helpers) and the **text-edit tool** (`run_add_text_tool` onward). Both
are `&mut self` methods, so use the same technique: an `impl PdfceApp`
block in the new module. A child module reaches its ancestors' private
items, so only the calls `main.rs` makes back *up* need `pub(crate)`.

**Colour is now gated.** `theme.rs` owns every colour role;
`tools/check-theme-colors.sh` refuses a raw one elsewhere. The escape
hatch is real and load-bearing: `// DOCUMENT COLOUR:` marks the three
sites whose colour reaches a saved PDF, and those must never be themed.

---

## Lessons this session that will save you time

- **A textual gate that asserts a needle is PRESENT fails loudly when it
  loses its subject; one that asserts ABSENCE goes quiet and keeps
  passing while checking nothing.** Both happened today, hours apart.
  Prefer present-assertions.
- **A `diag` step is not an operator route (R184).** Three panels shipped
  reachable only by the harness, and every verification passed.
- **Unused-import warnings from a build that ERRORED are meaningless** —
  analysis stops at the first failure. Cost two cycles.
- **`wc -l` is not a count of anything you care about.** It was wrong
  about the baseline file, and `ls docs/decisions | wc -l` is not a count
  of decisions — §12 numbers and filenames share ONE space and the
  directory is legitimately sparse. Written into
  `docs/decisions/README.md` after two agents got it wrong.
- **Heredocs in this environment halve backslashes.** Any Rust or shell
  string containing `\n` or a line continuation should be written with
  the Write tool and spliced, not pasted into a `<<'PY'` block. It
  silently produced runs of spaces inside two operator-facing messages.
- **Read the output as its audience (R174).** Two contradictory warnings
  in one `list-signatures` run, and a wrapped CLI message, were both
  found by looking at output rather than by any test.
