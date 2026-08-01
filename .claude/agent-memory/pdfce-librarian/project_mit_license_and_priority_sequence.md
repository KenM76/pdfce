---
name: project-mit-license-and-priority-sequence
description: pdfce's OSS license is DECIDED (MIT, 2026-08-01) and the operator set a four-item work-priority sequence the same continuation — dimensioning tool (active) → GUI icons → finish text-handling → form-building tools.
metadata:
  type: project
---

**2026-08-01, SESSION_LOG continuation 50.** Two facts landed together
in one operator instruction:

1. **`LEGAL.md` §1 license decision: MIT.** Implemented same session —
   repo-root `LICENSE` (standard text, "Copyright (c) 2026 Ken
   Mantle"), `license = "MIT"` in `Cargo.toml` `[workspace.package]`,
   `license.workspace = true` on all four member crates. Dependency
   audit: 100% permissive, zero copyleft — MIT requires no dependency
   rework. Consequence: GPL/AGPL prior art (MuPDF, Poppler,
   Ghostscript) is now categorically, permanently excluded as a real
   dependency (was already the practical posture, now locked in).
   Project rule 8's license precondition is satisfied — but **this
   does NOT authorize pushing** the existing local commit (`d8b3903`)
   or publishing; that's a separate, still-open operator item.

2. **Four-item priority sequence** (verbatim: *"get the dimensioning
   tool completely functional in the gui interface. add
   d:/dev/scriptree style icons for all gui features. finish off all
   the text handling stuff. work on form building tools after if that
   makes sense."*):
   1. Dimensioning tool → completely functional in GUI. Promoted to
      **ACTIVE**. State at the time: only Pass 12.0 (canvas substrate,
      uninhabited) shipped; decision 011's remaining slices — 9a, 12.M1,
      12.M2, 9c-min — not built. Pass 9a dispatched to build.
      **UPDATE (continuation 51, 2026-08-01):** Pass 9a (object/
      selection model + centerline) SHIPPED and committed (`e13f3e6`);
      Pass 12.M1 (snapping) now in progress. 12.M2/9c-min remain. A
      marquee-vs-pan UX flag from 9a is owed a `pdfce-ui-specialist`
      review at the 12.M1/12.M2 stage.
      **UPDATE (continuation 52, 2026-08-01):** Pass 12.M1 (snapping
      engine + fuzzy snap indicator) SHIPPED and committed (`801a748`,
      on top of `19ed865` docs / `e13f3e6` 9a / `79d1c6f` MIT /
      `d8b3903` impl — 3 of 5 beta slices done). The marquee-vs-pan
      flag is now RESOLVED (kept — dimension tools use click-A-then-
      click-B, no conflict with marquee-drag). **Pass 12.M2**
      (dimensioning + scale/group + hybrid storage + OCG layer) is
      now in progress, dispatched same continuation; **9c-min**
      remains after it as the beta's last slice.
      **UPDATE (continuation 53, 2026-08-01):** Pass 12.M2 (dimension-
      ing + scale/group + hybrid storage + OCG layer, the headline
      capability) SHIPPED and committed (`c7c1744`, chain now
      `d8b3903`/`79d1c6f`/`e13f3e6`/`19ed865`/`801a748`/`c7c1744`, six
      deep, all local-only). Dimensions fully authorable via CLI,
      disclosed in GUI, but on-canvas click-to-author was deliberately
      DEFERRED to a new engineer-assigned slice, "Pass 12.M2b"
      (on-canvas dimension authoring), now building — that's the slice
      that actually delivers "completely functional in the GUI." 9c-min
      still last, after 12.M2b. Also this continuation: the ScripTree
      icon DESIGN (priority #2) is now complete
      (docs/ui_specs/icon-set-and-toolbar.md, 27 controls, redaction
      solid-fill exception) though its BUILD hasn't started — two
      decisions named as operator/KenAgent-gated before that build is
      scoped: (a) SVG-in-egui pipeline (pre-rasterize PNG, no dep, vs.
      resvg/usvg, MPL-2.0, rule-13 sign-off needed), (b) ScripTree icon
      provenance/licensing confirmation before bundling into MIT pdfce
      (likely fine, Ken owns both, but must be confirmed not assumed).
      Neither decision made yet — check current state before assuming
      either is resolved.
      **UPDATE (continuation 54, 2026-08-01):** Pass 12.M2b (on-canvas
      dimension authoring gesture) SHIPPED and committed (`7c93cc3`,
      chain now `d8b3903`/`79d1c6f`/`e13f3e6`/`19ed865`/`801a748`/
      `c7c1744`/`6150e1a`/`7c93cc3`, eight deep, all local-only).
      **MILESTONE: priority #1 ("dimensioning tool completely functional
      in the GUI") is now SUBSTANTIALLY MET** — dimensions fully
      authorable both via CLI and on-canvas (click-A/click-B linear,
      pick-set+fit circular, reference-line+dialog scale, dimension-
      groups panel). **Only 9c-min (basic vector editing) remains** of
      decision 011's five originally-named slices, now IN PROGRESS.
      **Both icon-build gated decisions RESOLVED this continuation** by
      direct operator answer: (a) pre-rasterize SVGs to PNG at build
      time, zero new dep, `resvg`/`usvg` rejected; (b) use ScripTree's
      own SVGs where they fit, draw new ones in-style otherwise,
      resemble Inkscape/Adobe visual CONVENTIONS (not their artwork) for
      new icons — Ken's own confirmation of ownership/intent, verbatim
      captured in `ROADMAP.md`'s Icon-set entry and `SESSION_LOG.md`
      continuation 54. Icon BUILD is now unblocked but still queued
      behind 9c-min. Also this continuation: the pre-existing
      integration-test temp-path-collision flake (Backlog, filed at
      Pass 9a) was independently observed a second time during 12.M2b —
      a bounded fix (thread-unique temp paths) is now dispatched,
      Backlog entry amended "FIX IN PROGRESS."
   2. ScripTree-style SVG icons for all GUI features (styled after
      `D:\Dev\ScripTree\icons\*.svg`) — new, unscoped Backlog item,
      queued behind #1.
   3. Finish text-handling: FF-B, FF-H, FF-C all now schedulable
      (FF-C's license/rule-8 gate specifically lifted by item 1 above).
      **List-authoring is a SEPARATE, still-unanswered scope question**
      — this instruction does NOT resolve it; don't conflate the two.
   4. Form-building tools (field CREATION/authoring — distinct from the
      shipped Pass 7.0/7.1 fill/flatten subsystem) — queued last,
      operator's own hedge ("if that makes sense") noted verbatim.

**Why this matters:** this is the current top-level work-order for the
whole project as of 2026-08-01 — any future librarian dispatch
("what's next", "roadmap update", "pre-compaction capture") should
check `ROADMAP.md`'s "★★★ Operator priority sequence" block (top of
"Next up") for the live, authoritative version of this before assuming
anything from an older session's framing (e.g. the pre-2026-08-01
"text-parity arc awaits an operator decision" framing is now
superseded — the decision arrived).

**How to apply:** when asked to add a new Backlog/Pass entry or judge
sequencing, respect this four-item order unless the operator gives a
new explicit steer. Don't let a lower-priority item (icons, text-
handling, forms) get scheduled ahead of the dimensioning tool without
a fresh operator instruction to reorder. See also
[[project-loop-throttled-awaiting-steer]] (the steer this sequence
represents) and [[project-uncommitted-repo-worktree-risk]] (the
license-vs-push distinction this decision sharpened).
