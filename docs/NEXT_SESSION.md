# NEXT SESSION — start here

Engineer-owned handoff (this filing written by `pdfce-librarian` at the
engineer's explicit request, same as every prior overwrite of this
file). Read this **before** the librarian's record — `ROADMAP.md` says
what shipped, this says what is in flight and what the next hour
should be. Overwrite it once acted on.

Written 2026-08-12, branch **`main`**, after six commits
`b358657..d8a8948` (`Pass 67.0` phase E — embed missing fonts — now
SHIPPED, core+CLI+GUI+corpus harness).

---

## ★★★ PHASE E IS DONE. It was the phase that answered the original request

The end-user problem that opened `Pass 67.0` — a Barnes & Noble Press
upload requiring embedded fonts — is now solvable end to end:
`list-fonts` diagnoses `not-embedded=N`, `embed-font` closes it. The
4,023-file sweep reaches `not-embedded=0` on 726 of 4,023 files with
`--font-dir` + `--use-bundled-fonts`; the 177 residuals left in
`--font-dir` mode are genuine refusals (no source font, Type 3,
composite/CID, etc.), not gaps in the feature.

**`Pass 67.0` now has three of six phases shipped (A, B, E) and three
unstarted (C, D, F).** None of the three remaining are blocking the
request that started the family — this is now genuinely open-ended
Pass work, not urgent-fix work, and probably worth asking the operator
which (if any) he wants next rather than guessing.

- **C — Re-subset.** Shrink an already-embedded font's program to only
  the glyphs the document actually uses. Lowest risk of the three: no
  visual change, no text loss, works on *every* embedded font
  including the ~13–48% (method-dependent, see `Pass 67.0`'s own
  Shipped entries for both measurements) phase B must refuse. The
  right answer when the motivation is file size.
- **D — Convert text to outlines.** The universal escape hatch — the
  only phase that works where phase B is refused outright, because
  glyphs become vector paths and no font program needs to sit at those
  positions at all. Substantial to build; irreversible in effect
  (search/copy/reflow all stop working on the converted text) —
  disclose the cost inline, not merely via a confirm button (rule 4).
- **F — Replace font X with Y.** The hardest of the six — not just
  swapping a program name, but remapping encodings and widths so text
  does not reflow wrongly. Acrobat has **no equivalent** (searched
  across three sessions by `pdfce-acrobat-librarian`, recorded as a
  genuine absence) — this phase is parity-plus, not parity.

**Reusable substrate for all three, already built this Pass:**
`FontEnvironment::resolve_for_embedding`'s four-rung donor ladder,
`fontinfo::Removability`'s nine-verdict classifier, and
`font_embed_missing.rs`/`font_unembed.rs`'s shared-descriptor/shared-
program reachability code (the exact "which `/FontDescriptor` and
`/FontFile*` does this font dictionary actually reach, and is it
shared" question phase C's re-subset-in-place and phase F's swap-the-
program both need answered the same way phase E's Attach shape already
needed it).

---

## ★ Open operator question `(bk)` — needs Ken's ruling, not engineering judgment

**May pdfce's own bundled Base-14 substitute faces (BSD-3-Clause,
pdfium's Foxit-origin set) be embedded into an operator's document?**
Embedding puts the face inside a document the operator then
distributes — a different act from pdfce merely drawing with it on the
operator's own screen — and carries the licence's binary-
redistribution attribution condition once it travels inside someone
else's PDF. **This is a legal call, and therefore Ken's** — surface it,
don't resolve it (`pdfce-engineer.md`'s own standing rule).

**What shipped in the meantime, deliberately not a resolution:**
`pdfce-cli embed-font --use-bundled-fonts`, off by default, help text
states the obligation. **The GUI does not offer the bundled rung at
all.** Practical weight: bundled faces alone embed 1,250 of 1,507
corpus missing fonts, and are the ONLY donor for `Symbol`/
`ZapfDingbats` (16% of missing fonts) — so this is not an academic
question, it is the difference between closing 83% and closing ~11% of
the real-world gap. Full text: `docs/ROADMAP.md`'s *Open operator
questions* section, `(bk)`.

---

## ★ Read this next: `/R` 6 is still the only encryption gap, unrelated to fonts

Nothing changed here this session — encryption stayed explicitly
parked while font work ran. Carried forward verbatim in substance from
the prior handoff, since it is still exactly true.

`/R` 6 is the **only** thing between pdfce and the common AES-256
case — `/R` 6 is the default Acrobat X+ "AES-256" setting actually
produces, plausibly the *common* real-world shape, not the exotic one.
The gap is exactly one function:

```
crates/pdfce-core/src/crypto/r5.rs — private fn hash
```

Its own doc comment names it as Algorithm 2.B's substitution point and
states everything AROUND it — the `/O`/`/U` layout, the `/UE`/`/OE`
unwrap, the `/Perms` check, the harness that calls it — is already
implemented and tested. **That is precisely the situation where
filling it from memory is most tempting and least detectable.** The
refusal fixture (`enc-aes-256-r6.pdf`) and the refusal tests exist
specifically to make that hard — do not remove or weaken either to
"make progress."

**Routes to close it, unchanged:**
1. **ISO 32000-2 is $0.00 under PDF Association sponsored access** —
   needs an account and a checkout. **This is the operator's act, not
   an agent's** — surface it to Ken rather than attempting a
   workaround.
2. Any other primary, citeable source for Algorithm 2.B that isn't
   itself a derivation from another implementation's output.

Once `/R` 6 is sourced, the remaining Encryption scope is: encrypt-on-
save (every cipher, every shell — entirely unstarted), and nothing
else new — `/R` 6 is genuinely the last read-side gap.

---

## New standing rule this session — `R189`, worth carrying into any future object-allocation code

`Document::next_object_number` gained a fourth source
(`SectionShape::Stream { id, .. } => id.num`) after a real pdfium
fixture proved the first three insufficient: a created object silently
collided with the writer's own re-emitted cross-reference stream and
vanished, on any file whose newest xref stream sits outside its own
`/Size`. **Practical form for any future allocator in this codebase:**
ask not "what does the xref table/stream declare it covers" but "what
can the WRITER itself put a number on, including things it re-emits
under an existing number rather than allocating fresh." Full record:
`docs/ARCHITECTURE.md` §5.7, `docs/ROADMAP.md` Standing rules `R189`.

---

## A GUI finding worth remembering before shipping the next Fonts-panel control

The embed batch button shipped unclickable for one build — every
headless trace assertion passed, including a trace of the button's own
reported rect, because the dock clipped it and a traced rect describes
the layout *request*, not what survived clipping. Second instance of
`D:\dev\rag\egui\headless_trace_asserts_reached_not_visible_a_clipped_widget_needs_a_pixel_oracle.md`.
**Tell worth remembering**: a control whose traced rect is wider than
a sibling's in the same dock is the one to click-test first, before
trusting the trace.

---

## Tooling — corrections that cost time in prior sessions, still true

- **`PDFCE_DIAG_VIEWPORT`**, not `PDFCE_VIEWPORT`. Four comma-separated
  numbers: `x,y,w,h`.
- **The diag script separator is `;`, not `,`.** A comma-separated
  script parses as ONE unparseable step, is silently skipped — the
  trace says `script-step-UNPARSEABLE`, read it.
- **`gui-shot.ps1` and `gui-drive.ps1` default to different window
  sizes.** Read the trace's own `rect=`, never a screenshot's pixels.
- **These two scripts move the REAL cursor and synthesise Ctrl+scroll
  and click-drag on the live desktop.** Say so before running one while
  the operator is at the machine; prefer headless verification (CLI,
  unit tests, `cargo test`) when it will do.

`tools/splice.py` — anchored substitution, all-or-nothing.
`tools/verify-release.py <tag>` · `tools/gen-embed-fixtures.py` /
`tools/gen-unembed-fixtures.py` (no arguments needed) ·
`tools/package-portable.py --note "..."`.

---

## Standing release authorisation (still in force)

The operator's 2026-08-11 instruction — *"please continue to post the
latest versions to git so I can try them on my laptop at home"* — is
ongoing. Rule 8's per-release ask does not apply to cutting a release
of THIS project: build it, tag it, publish the asset, run
`tools/verify-release.py`, report what went out. Scope is narrow:
authorises releasing pdfce builds for the operator's own testing, NOT
blanket publishing authority, NOT a licence to treat repository
visibility as an agent's own decision, NOT permission to skip
verification. `CLAUDE.md` rule 8's literal per-release wording is
still technically stale against this — flagged to the operator across
several prior filings, not yet amended by him; not this librarian's or
the engineer's file to edit.

---

## Open items, in the order they're likely to matter

1. **`(bk)` — bundled-font embedding licensing.** Ken's call. Surface
   it directly rather than waiting for it to come up.
2. **`Pass 67.0` phases C, D, F** — all unstarted, none blocking, ask
   the operator which (if any) he wants next rather than guessing an
   order.
3. **`/R` 6 sourcing** — the only encryption read-side gap, unrelated
   to fonts. PDF Association sponsored ISO 32000-2 access is the
   operator's own act — surface it, don't attempt a workaround.
4. **Encrypted-save**, any cipher — entirely unstarted.
5. Two dead/stale printing items, filed to Backlog, deliberately not
   fixed: `DeviceSettings::pick_tray_by_page_size` sets no `DEVMODE`
   field at all; `build_devmode`'s doc claims a driver-default start
   the code doesn't actually do.
6. **Imposition has no GUI** — extract sheet composition into
   `pdfce-print` first so both shells share one implementation.
7. Static hybrid XFA read/fill · wide-shape CSV · colour management
   (`D:\Dev\iccce\`, planned, no code).
8. **Ledger-accuracy defect, still not fixed** (carried from several
   sessions ago): filings ninety-two through ninety-five cite `(bh)`/
   `(bi)` as if `(bi)` had not been minted.
9. **Spec-librarian flag, still open**: confirm the eight-item
   never-encrypted list (E1–E9) is in the §7.6 corpus rather than only
   in pdfce's code.
