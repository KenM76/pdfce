# Pass 19.3 — the spacing/style property surface (ui-spec)

**Author:** `pdfce-ui-specialist` · **Date:** 2026-08-03 · **Drives:** decision
`019-ffh-spacing-scaling-synthetic-styles.md` §6 slice 19.3 (as corrected by
Amendments A and B, both incorporated below — where this spec and the
decision's main body differ on a point Amendment A/B already settled, the
amendment's version is what is used here).

**Scope of this document.** Character spacing (`Tc`), horizontal scaling
(`Tz`), the superscript/subscript toggle, free-form baseline rise (`Ts`), and
synthetic bold/italic (`R90`) — all for the **in-place edit** path
(`CanvasTool::TextEdit`'s property bar). The Add-Text path (`CanvasTool::
AddText`) shares the same `FormatRequest`/`StyleSynthesis` vocabulary and the
same widget language is meant to transfer directly when that panel is
extended, but this document specs the TextEdit surface concretely and only
notes the Add-Text implications inline (§9).

**I do not write code.** Everything below is a widget-tree spec, exact
`ui_text.rs` function proposals, and named core-API asks, for the engineer to
implement or push back on.

---

## 0. The single biggest call this spec makes, up front

**This is NOT a new panel.** It is five new rows added to the *existing*
`TextEdit` floating property bar (`main.rs` ~L7376, `egui::Area::new("pdfce-
text-edit-propbar")`), which already renders Size/Colour/Font for the caret's
run. That bar is a **tool-scoped floating `Area`**, not the `egui_tiles` dock's
Properties panel — confirmed by reading the actual shipped code, not assumed:
the dock's Properties panel (Pass 18.1) is the **document metadata** form
(`/Info` dictionary — title/author/etc.), a completely different concept that
happens to share the English word "properties." The **same** tool-scoped
floating-Area pattern is independently used by `AddText`'s property bar
(`main.rs` ~L8257) and the Measure tools' property bar (~L9648) — three
independent instances of one convention. Putting the new controls anywhere
else (the dock, a new floating window, a context menu) would be the "invent a
fourth way" failure mode my own brief warns against. **This answers the
brief's question 1 directly: it lives in the TextEdit tool's own property
bar, full stop — not the dock, which starts closed and is not where any
per-run editing control lives today.**

Two consequences follow immediately, both binding on the rest of this spec:

- The bar's placement, framing (`egui::Frame::popup`), `360.0`pt max-width,
  and per-row "label → control → **Apply**" idiom (one control family per
  click, one undo-able `FormatText` command per Apply) are **inherited, not
  redesigned**. New rows use the identical idiom.
- The bottom status `Area` (`"pdfce-text-edit-status"`, ~L7546) — Accept/
  Reject, the refusal strip, the disclosure strip — is the **single**
  existing non-modal disclosure surface in this tool. Every new refusal and
  every new disclosure renders through it, verbatim, exactly as size/colour/
  font changes already do. No second disclosure surface is proposed anywhere
  in this spec.

---

## 1. Two load-bearing gaps found by reading the actual code (flag first)

Per my own brief's discipline ("before recommending from memory... verify"),
I read `crates/pdfce-core/src/text_edit/format.rs` and `crates/pdfce-gui/src/
main.rs` in full for the relevant sections rather than trusting the decision
doc's own prose summary. Two gaps surfaced that the decision doc does not
name (it wasn't its job to — §6 slice 19.3 explicitly says "dispatch
`pdfce-ui-specialist` first," and this is exactly the kind of finding that
dispatch is for).

### 1.1 No public "would this resolve to a real face, or would it synthesize?" query exists — R83/R74 gap

`format.rs`'s `gate_synthesis` (private `fn`, ~L1707) is the **entire**
mechanism deciding whether a `set_synthetic` request succeeds or gets refused
with `FormatError::RealFaceAvailable`. It is not exposed, and nothing else in
the public API answers "if I asked for Bold right now, would a real face
resolve or would pdfce synthesize?" ahead of actually submitting a
`FormatRequest`.

This matters for exactly the reason R83/R74 exist: without it, the property
bar has two honest options, and I want to name both rather than silently
assume the nicer one exists.

- **Option A (ships today, zero new core API):** the Bold/Italic checkboxes
  are plain toggles; "Apply style" always submits `FormatRequest::synthetic
  (...)`; a `RealFaceAvailable` refusal renders through the **existing**
  refusal strip, verbatim — and its `Display` text is already excellent
  (names the exact resource key and `/BaseFont`, states nothing was applied).
  This is a legitimate, honest P0 cut: rule 4 requires visible marking,
  operator override, and no silent application — a refusal message the
  operator reads *after* clicking Apply still satisfies all three. Nothing
  new in `pdfce-core` is required.
- **Option B (needs one new, thin, public core API):** the property bar shows
  a live caption **before** the operator clicks Apply — "no real Bold face on
  this page: would synthesize" vs. "real Bold available as `/F3` (Arial-Bold):
  would switch font instead" — so the operator makes an informed choice
  *before* committing, not after. This is closer to R90's own word
  "declinable," which reads more naturally as "offered, then accepted or
  declined" than "attempted, refused, then the operator goes elsewhere."

**I recommend Option B**, but it is a real, named, escalatable trade — not a
must-fix. If the engineer wants 19.3 to ship without touching `pdfce-core`
again, Option A is a legitimate, honest interim (see §8's priority list).

**The concrete ask, if Option B is taken**, thin because it wraps existing
logic rather than duplicating it (same discipline as 14.3's `EditSession`
sibling ask and 15.2's `reflow_recognition_options` hoist — reuse the real
mechanism, don't reimplement it in `pdfce-gui`):

```rust
/// A read-only preview of what a StyleSynthesis request WOULD do for this
/// run, without mutating anything. Wraps the same gate_synthesis logic
/// set_format already uses internally — published, not reimplemented,
/// so pdfce-gui never re-derives family_stem/name_claims_bold/italic
/// matching itself (R74).
pub fn preview_style_resolution(
    doc: &Document,
    page_index: usize,
    find: &str,
    pinned_span: Option<ByteSpan>,
    want: StyleSynthesis,
) -> Result<StyleResolution, FormatError>;

#[non_exhaustive]
pub enum StyleResolution {
    /// A real face covering the FULL combination asked for resolves —
    /// submitting `set_synthetic(want)` right now would be refused with
    /// exactly this resource/font.
    RealFaceResolves { resource: String, base_font: String },
    /// No real face covers the full combination — submitting
    /// `set_synthetic(want)` right now would apply it.
    WouldSynthesize,
}
```

**A genuine wrinkle to word carefully, found by reading `gate_synthesis`
itself, not assumed:** the gate is **all-or-nothing per combined request**.
`covers_bold && covers_italic` must **both** hold for a **single** candidate
face to count — there is no code path that resolves "apply a real Bold face
and *also* synthesize the Italic on top of it." If the operator ticks **both**
Bold and Italic, and the page has a real `Arial-Bold` but no `Arial-
BoldItalic`, the request proceeds as **synthesizing both**, even though the
Bold half had a real alternative sitting right there. This is existing,
already-shipped Pass 19.2 core behaviour — outside my mandate to change — but
my proposed preview caption (§3.4 below) must say precisely this and not
imply partial credit. **Composing a real-face family-change for the covered
attribute with synthesis for the uncovered one is a genuine, reasonable
fast-follow idea this finding surfaces — explicitly NOT decided here**,
matching decision 019's own "does not decide" convention; flag it to the
engineer as a possible follow-up ask, not a requirement of this Pass.

### 1.2 The existing property bar does not show the run's CURRENT ambient value — a real gap for these controls specifically, not for Size/Colour/Font

Read `main.rs` L1479-1545: `TextEditState::prop_size` is seeded to a **fixed
default** (`12.0`) once, at tool entry, and is **never re-seeded** when the
caret moves to a different run. The same is true of `prop_model`/
`prop_components`/`prop_font`. This is a real, pre-existing behaviour of the
shipped 14.3 panel, and it is **tolerable for Size/Colour/Font specifically**
because the operator can *see* the consequence at a glance — the glyphs
visibly are 12pt, visibly are that colour, visibly are that face — so the
property bar can get away with being purely "what to apply next," not "what
is true now."

**That tolerance does not transfer to `Tc`/`Tz`/`Ts`.** A `Tc` of 0.24 vs 0 is
often *invisible* to the eye at normal reading zoom; a `Tz` of 95% vs 100% is
barely perceptible; a small `Ts` is subtle by design. An operator cannot
"just look at the page" to know the run's current ambient values the way they
can for size or colour. Shipping these five new rows seeded from a blind
fixed default (e.g. always starting at `Tc=0`, `Tz=100`) would mean the
operator can never tell, from the panel alone, whether the run they clicked
into already has non-default tracking — they'd have to guess, or worse,
silently stomp an existing value they never saw. **This is a fuzzy-never-
sneaky (rule 4) risk specific to this family of controls, not a generic
polish nit.**

**Required addition, and it is cheap: seed AND caption from ambient on every
caret move.** Pass 19.0 already publishes exactly what is needed —
`GlyphProvenance::text_state: Arc<AmbientTextState>` carries `char_spacing`/
`h_scale`/`rise` as `AmbientValue { value, origin }` per run (`text_state.rs`
L630-650). On every caret-move that lands on a new run:

1. Re-seed `prop_char_spacing`/`prop_h_scale`/`prop_rise`/`prop_baseline` from
   that run's `text_state` (converted through `MetricSpec` where relevant),
   so the panel starts each row from "what it actually is."
2. **Also** render a small, non-editable "Ambient: …" caption beside each row
   — reusing the **exact same convention** already shipped one screen away in
   this same panel for reflow: `reflow_detected_caption`/
   `reflow_overridden_caption` (§4.2's "Detected: X" / "you changed this"
   idiom, `main.rs` L7391-7408). This is not a new UI language; it is the
   established one, applied to a second control family.

This double coverage (seed the value AND show it as a caption) matters
because the operator may type a **new** value while the caption still shows
what was **true** — exactly like reflow's own detected-vs-current split. See
§3 for the exact per-row wording.

**A related, pre-existing gap I am NOT asking to fix here:** Size/Colour/
Font have the identical seeding gap and it predates this Pass. I flag it
because it is the same root cause, in the same struct, in the same file — an
engineer fixing one is one edit away from fixing all four — but whether to
bundle that fix into 19.3 or file it separately is a scheduling call, not
mine to make (per my own brief's boundary).

---

## 2. `TextEditState` field additions (concrete, for `main.rs`)

```rust
// -- Pass 19.3 additions to TextEditState --

/// Tc, operator's typed number, in whichever unit `prop_tc_unit` names.
prop_char_spacing: f64,
/// Which MetricSpec variant `prop_char_spacing` is expressed in.
/// Default Relative (em/1000) per decision 019 §3.2's GUI-default call.
prop_tc_unit: MetricUnit,

/// Tz, percent (100 = normal). No unit choice — Tz is dimensionless (R89).
prop_h_scale: f64,

/// The baseline control's current selection. `Custom` is a pdfce-gui-only
/// state (not a ScriptPosition variant) meaning "show the free numeric
/// rise field instead of the three-way toggle."
prop_baseline: BaselineChoice, // Normal | Superscript | Subscript | Custom

/// The free-form rise value, only meaningful when prop_baseline == Custom.
prop_rise: f64,
/// Which MetricSpec variant prop_rise is expressed in. Default Absolute
/// per decision 019 §3.2 ("what the operator typed is what they get").
prop_rise_unit: MetricUnit,

/// Style checkboxes. Independent of prop_baseline (script position and
/// weight/slant are unrelated axes).
prop_bold: bool,
prop_italic: bool,

/// A pending, uncommitted synthesis OFFER awaiting operator accept/decline
/// (§4). `None` most of the time. Mutually exclusive with nothing else —
/// it can coexist with `pending`/`reflow`, it just adds one more row to the
/// same bottom status Area. Discarded (not confirmed) by the SAME
/// GestureInterrupt::Discard policy already governing `pending`/`reflow`
/// (§5): navigating away, undoing, or starting a different gesture while
/// an offer is showing just clears it, no confirmation asked, because
/// NOTHING was written (rule 7 — low-stakes, nothing to be careful about
/// losing).
pending_style_offer: Option<StyleOfferState>,

/// GUI-local unit tag mirroring MetricSpec's two variants at the UI layer,
/// so a toggle switch is a plain enum flip rather than reconstructing a
/// MetricSpec every frame.
enum MetricUnit { Absolute, Relative }

enum BaselineChoice { Normal, Superscript, Subscript, Custom }

/// One in-flight synthesis offer (§4), built the moment "Apply style"
/// resolves to a synthesis request needing operator confirmation.
struct StyleOfferState {
    /// Exactly what would be requested (bold/italic combination).
    want: pdfce_core::text_edit::StyleSynthesis,
    /// Set only if Option B (§1.1) ships: the real-face alternative, if
    /// preview_style_resolution found one. `None` under Option A (the
    /// offer is then built directly from a FormatError::RealFaceAvailable
    /// the operator already triggered by clicking Apply once — see §4.2).
    real_alternative: Option<(String, String)>, // (resource, base_font)
}
```

---

## 3. Widget tree — the five new rows

All five rows live inside a **new `egui::CollapsingHeader`**, collapsed by
default, titled via `ui_text::format_spacing_section_title()` → `"Spacing &
style"`, inserted into the property bar's existing non-reflow (`else`) branch
**after** the Font row (`main.rs` ~L7510) and **before** the existing
`ui.separator()`/Reflow-button block (~L7514). This is a **direct reuse** of
the `CollapsingHeader` convention already shipped one Pass over, in the
**same app**, for the identical reason: Pass 18.4's status-bar readout used a
one-line headline plus a `CollapsingHeader` for detail specifically to avoid
permanently growing a panel that some operators will rarely touch (rule 3,
progressive disclosure) — this is that same tension (Size/Colour/Font are
"always relevant"; `Tc`/`Tz`/baseline/style are "occasionally relevant, and
Acrobat-Pro-ribbon-overload is the exact failure mode rule 3 exists to avoid")
resolved with the exact same, already-precedented widget.

```
CollapsingHeader("Spacing & style") [default: collapsed]
├─ Row: Character spacing (Tc)
│   ├─ Label: "Character spacing (tracking):"
│   ├─ Caption (non-interactive): "Ambient: {ambient, in the CURRENT unit}"
│   ├─ DragValue(&mut prop_char_spacing), speed 0.05
│   ├─ Unit toggle: [em/1000] [pt]  — selectable_label pair, R84-safe (§6)
│   │     on click: RE-DERIVE the displayed number from ambient in the
│   │     NEW unit (never silently reinterpret the same digits under a
│   │     different meaning — see §3.1 rationale)
│   └─ Button: "Apply spacing" → builds ONE FormatRequest.char_spacing(...)
│
├─ Row: Horizontal scale (Tz)
│   ├─ Label: "Horizontal scale (%):"
│   ├─ Caption: "Ambient: {ambient}%"
│   ├─ DragValue(&mut prop_h_scale).range(1.0..=1000.0), speed 0.5, suffix "%"
│   └─ Button: "Apply scale" → FormatRequest.h_scale(...)
│
├─ Row: Baseline (superscript/subscript/custom rise) — SINGLE control group
│   ├─ Label: "Baseline:"
│   ├─ Caption: "Ambient: {Normal | Superscript | Subscript | rise=X.XXX pt}"
│   ├─ 4-way selector: [Normal] [Superscript] [Subscript] [Custom…]
│   │     rendered as selectable_label(is_sel, toggle_label(is_sel, text))
│   │     pairs, NOT bare selectable_value (R84 — see §6.2)
│   │     Selecting Normal/Superscript/Subscript HIDES the free-rise field
│   │     entirely (not merely disables it — R83: there is no capability
│   │     to combine them, ConflictingRise, so no control implying
│   │     otherwise is drawn). Selecting Custom SHOWS the free-rise field
│   │     below and visually deselects the three-way toggle.
│   ├─ [only visible when Custom is selected]
│   │   ├─ DragValue(&mut prop_rise), speed 0.1, suffix "pt" (Absolute) or
│   │   │     "‰" (Relative) depending on prop_rise_unit
│   │   └─ Unit toggle: [pt] [relative to size] — same re-derive-on-switch
│   │         rule as Tc's unit toggle
│   └─ Button: "Apply baseline" → EITHER
│         FormatRequest.script(Normal|Superscript|Subscript)   (3-way case)
│         OR FormatRequest.rise(MetricSpec::...)                (Custom case)
│       — never both in one request; the UI's own mutual exclusion above
│       makes ConflictingRise structurally unreachable from this panel,
│       which is the real answer to the brief's question 3 (§7).
│
├─ Row: Style (Bold / Italic)
│   ├─ Label: "Style:"
│   ├─ Checkbox(&mut prop_bold, "Bold")
│   ├─ Checkbox(&mut prop_italic, "Italic")
│   ├─ [Option B only] live caption per §3.4, updated from
│   │     preview_style_resolution() whenever prop_bold/prop_italic/the
│   │     caret's run changes — NOT on every frame regardless (avoid a
│   │     per-frame core call; recompute on the actual state changes only)
│   └─ Button: "Apply style" → FormatRequest.synthetic(StyleSynthesis::new(
│         prop_bold, prop_italic)) — see §4 for the offer/refusal flow
│
└─ Row: Word spacing (Tw) — READ-ONLY, always visible (not inside a
    conditional), because it is a disclosure, not a control
    ├─ Label: "Word spacing (Tw):"
    ├─ Value, greyed: "{ambient Tw value} (read-only)"  — plain disabled
    │     text via `ui.add_enabled(false, ...)` or an explicitly greyed
    │     Label; NEVER a control that looks editable (R83)
    └─ Explanation, one of two, chosen from the run's published composite
        flag (`GlyphProvenance::composite`):
        - simple font:  "pdfce does not yet offer a word-spacing control
          here — Tw is preserved and shown, not editable, pending a
          usage census (see docs)."
        - composite font: "This run uses a composite font; Tw is void for
          multi-byte character codes per the PDF spec and can never take
          effect here, editable or not."
```

### 3.1 Unit-toggle switching rule (both Tc and the Custom-rise field)

Switching the unit selector **re-derives the displayed number from the
ambient value in the new unit's terms** — it never silently reinterprets the
digits currently in the box under a new meaning. Concretely: if the operator
has typed `20` meaning `20‰` and flips the toggle to `pt`, the field updates
to show the **resolved absolute value** (`MetricSpec::Relative(20.0)
.resolve(current_size)`), not a bare `20` that now means 20 points. This is
the direct, mechanical answer to the brief's own warning: "hiding \[the
absolute-vs-relative distinction\] is a lie and surfacing it clumsily is
noise" — the distinction is surfaced by making the number itself always mean
what its visible unit says, continuously, through a unit switch.

### 3.2 Ambient captions reuse an established convention, not a new one

`ui_text::reflow_detected_caption`/`reflow_overridden_caption` already do
exactly this job for reflow's alignment row, three rows above where these new
ones sit in the SAME panel. Proposed new functions, same shape:

```rust
/// "Ambient: {value}" caption for Tc, in whichever unit is active.
pub fn format_ambient_char_spacing(value_text: &str) -> String {
    format!("Ambient: {value_text}")
}
// Mirrored for h-scale, baseline, and the read-only Tw row — one shared
// `format_ambient_caption(label_free_value: &str) -> String` is enough;
// listed separately above only for doc-comment clarity per-call-site.
```

### 3.3 Word-spacing row wording, quoted in full for direct lift

```rust
pub fn format_word_spacing_label() -> &'static str {
    "Word spacing (Tw):"
}
pub fn format_word_spacing_readonly_note() -> &'static str {
    "read-only"
}
pub fn format_word_spacing_explanation_pending_census() -> &'static str {
    "pdfce does not yet offer a word-spacing control here — Tw is preserved \
     and shown, not editable, pending a usage census (see docs)."
}
pub fn format_word_spacing_explanation_composite() -> &'static str {
    "This run uses a composite font; word spacing (Tw) is void for \
     multi-byte character codes per the PDF spec (ISO 32000-1 §9.3.3) and \
     can never take effect here, editable or not."
}
```

**Why this row must exist and must not be silently omitted:** decision 019
§3.3 requires `Tw` be "displayed read-only in the properties panel with an
explanation" in **every** slice from 19.0 onward, specifically because
showing a value with no control and no explanation invites the exact
"looks broken" complaint the GUI-polish audit already catalogued elsewhere in
this app (Pass 18.4's own selection-legibility work). An operator who sees
"Word spacing: 12.500" with no way to change it and no reason given will
reasonably conclude the control is missing by bug, not by design.

### 3.4 The synthesis preview caption (Option B only), worded to match §1.1's finding precisely

```rust
/// Live, pre-Apply caption under the Bold/Italic checkboxes (Option B).
/// Built fresh from `StyleResolution`, never hand-authored per call site,
/// so its wording cannot drift from what Apply will actually do.
pub fn format_style_preview(resolution_text: StylePreviewText) -> String {
    match resolution_text {
        StylePreviewText::NoStyleRequested => String::new(),
        StylePreviewText::WouldSynthesize { style } => format!(
            "No real {style} face is available on this page — Apply would \
             synthesize a faux {style}, visibly marked, never silent."
        ),
        StylePreviewText::RealFaceResolves { style, base_font, resource } => format!(
            "A real {style} face is available as '{base_font}' (resource \
             /{resource}) — Apply will be REFUSED and will point you at \
             using the Font control above instead, per pdfce's synthesis-\
             is-fallback-only rule."
        ),
    }
}
```

Note the `RealFaceResolves` wording is written to match §1.1's exact finding:
it does **not** say "Apply will switch to that font" (Apply will not — the
core refuses and the operator must go use the Font row themselves). Saying
otherwise would be a spec bug of exactly the kind Pass 18.4's own
`ApproximateTextBounds` correction (`d296666`) had to fix after shipping —
copy that reassures the operator about a mechanism the code does not actually
have. **Get this wording right the first time; it is a place a plausible-
sounding sentence is checkable against `gate_synthesis` and this spec did
that check.**

---

## 4. The synthesis offer/refusal flow, both options laid out precisely

### 4.1 Option B (preview exists): offer BEFORE commit

1. Operator ticks Bold and/or Italic. The live caption (§3.4) updates.
2. Operator clicks "Apply style."
3. If the caption already said `WouldSynthesize` — submit
   `FormatRequest::synthetic(...)` directly, no further gate. Success →
   normal disclosure-strip render (§5). This is the **ordinary, unremarkable
   case** per decision 019 §3.6's own framing ("the gate rarely opens" for
   Add-Text; for in-place edit it opens "regularly" but is still routine, not
   exceptional).
4. If the caption already said `RealFaceResolves` — the operator already knew
   this before clicking; clicking Apply anyway is a deliberate choice to see
   the refusal (or a misclick). Submit anyway (never silently swap it to
   `set_font` on the GUI's own initiative — that would be the GUI making a
   decision the operator didn't ask for); the refusal strip renders the
   `RealFaceAvailable` message verbatim, same as always.

Under this option `pending_style_offer` in §2 is **not actually needed** — the
preview caption already IS the offer, rendered inline, continuously, before
any click. I include the field in §2 for completeness against Option A, where
it is load-bearing.

### 4.2 Option A (no preview; P0 cut): offer built from the FIRST refusal

1. Operator ticks Bold and/or Italic, clicks "Apply style."
2. GUI submits `FormatRequest::synthetic(...)` directly (no pre-check
   possible).
3. **Success** → normal disclosure-strip render.
4. **`FormatError::RealFaceAvailable` refusal** → rendered through the
   **existing** refusal strip, verbatim, exactly like every other refusal in
   this panel today. `pending_style_offer` is not needed here either, in the
   end — the existing refusal-strip mechanism already IS the "offer," read
   backwards: the operator is told "nothing was applied, here is the real
   alternative," and can now go use the Font row. No new interaction
   mechanism is required for Option A at all.

**Conclusion, restated plainly: neither option actually needs a bespoke
"declinable dialog" mechanism.** Option B needs a live *caption* (informational
only, no buttons). Option A needs *nothing new* beyond the panel's existing
refusal strip. **This directly answers the brief's question 4** ("how is
\[the fallback offer\] presented without becoming a modal interrogation") —
the honest answer, found by reading the actual error type rather than
assuming a confirm/decline widget was required, is that **no new dialog of
any kind is needed**; the existing non-modal refusal strip already
constitutes an honest, per-use, declinable presentation (the operator's
"decline" is simply "don't click Apply again with the same boxes ticked" —
exactly the same shape rule 7 already establishes for "delete an empty new
annotation you just created": no ceremony for a reversible, pre-save,
zero-cost non-application).

I am therefore **retracting my own draft `StyleOfferState`/inline-confirm-row
design** that this spec's first pass sketched, in favour of the above —
recorded here so a future reviewer sees the reasoning that ruled it out
rather than wondering why an inline-confirm design was considered and
dropped. `pending_style_offer` in §2 can be deleted from the field list if the
engineer agrees; I left it in §2 rather than silently removing it because
striking it out is exactly the "escalate, don't guess" instinct this
project's own decision docs model.

### 4.3 Why this is NOT the app's existing "blocking `egui::Window`" convention, named explicitly

The app has exactly two blocking confirmations today (`signature_
confirmation`, `copy_confirmation`), both centred `egui::Window`s with a
Cancel/Confirm pair, reserved for the two decisions the code comments
literally call "blocking questions." A synthetic-style Apply is **not** in
that class: it is undoable pre-save, reviewable via the same disclosure strip
every other format change already uses, and (per §3.3/§3.6) will recur
**routinely** in ordinary editing of any document with embedded font
subsets — the brief's own warning against "a modal interrogation on every
bold click" is exactly right, and a blocking `egui::Window` here would be
both the wrong ceremony level (rule 7) **and** a fourth dialog convention
where the app already has three (this floating-Area disclosure strip being
the third, alongside the two blocking Windows and the Pass-8-style
acknowledgement gate mentioned in this project's own memory) — the "don't
invent a fourth way" instinct from my own brief's §2 applies here in full
force, and the reading above is how it resolves without inventing anything.

---

## 5. Disclosure-strip integration (bottom `Area`, existing)

No new disclosure surface. `FormatReport.disclosures: Vec<String>` (format.rs
L827) already carries every operator-facing sentence for every field this
spec touches — `disclosure_char_spacing`/`disclosure_h_scale`/`disclosure_
rise` are already-authored core functions (format.rs L1467-1491), and the
justify-slack and restore-narrowing disclosures are core-authored too
(`justify_slack_invalidated`, `restore_narrowed`). The GUI's job, per Pass
14.3 finding #6 (memory), is a **pure render**, unchanged from what
`state.last_disclosures` already does today:

```rust
state.last_disclosures = report.disclosures.clone();
```

**One addition worth making, cheap and high-value: wire the `Tz`-invalidates-
justify disclosure to the ALREADY-PRESENT Reflow button in the same panel.**
When `report.justify_slack_invalidated` is true, append a short, second
sentence after the core disclosure text pointing at the control that is
**already three rows below it in this exact panel**:

```rust
pub fn format_justify_invalidated_hint() -> &'static str {
    "Use the Reflow control below to recompute this block's justified \
     spacing for the new width."
}
```

This costs nothing structurally — the Reflow button (`reflow_button_label()`,
~L7518) already targets the caret's recognized block and already offers
`BlockAlignment::Justified` as one of its four picks — it is **not** a new
mechanism, just a pointer to an existing one that happens to sit in the same
`egui::Area`. Per Amendment B §B.1, the disclosure text itself (core-authored)
now correctly names the *width delta* (`ΔA`) as the cause, not a `TJ`-rescale
— this GUI hint sentence is deliberately silent on *why* (that is core's
job, verbatim) and only adds *what to do about it* (the GUI's job).

### 5.1 New refusal-hint-table entries (§8.2's existing "what would lift it" pattern)

Following the exact pattern `r_inv_1_hint()` etc. already establish
(`ui_text.rs` ~L2791) — one short, fixed hint per `FormatError` variant,
joined to the core `Display` text via `refusal_with_hint()`:

```rust
pub fn conflicting_rise_hint() -> &'static str {
    "This panel already prevents this combination — if you see this refusal, \
     please report it as a bug: the Baseline control's Normal/Superscript/ \
     Subscript and Custom modes should never submit both at once."
}
pub fn real_face_available_hint() -> &'static str {
    "Use the Font control above to switch to the named face, or leave this \
     run's style as-is."
}
pub fn shear_unsupported_hint() -> &'static str {
    "See the message above for the specific reason; a family change to a \
     real Italic face (Font control above) may still work even though \
     synthesis does not."
}
```

The `ConflictingRise` hint is deliberately phrased as a **should-never-happen**
because §3's widget tree makes it structurally unreachable from this panel
(the 4-way baseline selector never lets both a script position and a custom
rise be "live" at once) — if it fires anyway, that is a real bug in the
mutual-exclusion wiring, not a normal operator outcome, and the hint should
say so rather than offering advice that implies it is routine.

---

## 6. Checklists (per my own brief's mandatory format)

### 6.1 Discoverability

- Every new label is plain English (no bare "`Tc`"/"`Tz`" anywhere in
  operator-visible text; the PDF operator names appear only in parenthetical
  asides, matching the existing "Size (pt):" convention which does the same
  for `Tf`).
- Every row's tooltip should explain **when**, not just what — proposed
  additions (not drafted verbatim above for space; the engineer should author
  these following the existing `block_overlay_toggle_tooltip()` shape, one
  sentence, "when to reach for this"):
  - Tracking: "for global letter-spacing adjustments across a run — nudging
    a producer's slightly-too-tight or too-loose tracking."
  - Horizontal scale: "for matching a squeezed/stretched look a producer
    used, or fixing a producer's mistaken one."
  - Baseline/Custom: "for footnote markers, chemistry/math notation, or
    aligning to a scanned baseline a two-position toggle can't reach — see
    decision 019 §3.2's worked examples."
  - Style: "when this run needs to look bold/italic and the family has no
    real variant on this page."
- **Keyboard shortcuts are not mandatory** (rule: destructive-action-only) —
  none of these five rows are destructive or irreversible pre-save. I
  recommend, not require, `Ctrl+B`/`Ctrl+I` as accelerators for the Style
  checkboxes while `TextEdit` is active with a caret/selection present —
  **verified unclaimed** by grepping `main.rs` for `Key::B`/`Key::I` at
  spec-authoring time; **re-verify at implementation**, per this project's
  standing hedge for every prior shortcut assignment.
- Visible current-state: **this is exactly §1.2's finding** — the ambient
  caption on every row is what makes "visible default/current state" true
  here; without it this checklist item fails outright for this control
  family specifically (unlike Size/Colour/Font, where the rendered glyphs
  themselves are the visible state).

### 6.2 Accessibility

- Tab order: all seven rows sit in one sequential `ui.vertical` inside the
  existing `CollapsingHeader` body, in the order listed in §3 — egui's
  default tab order follows insertion order, which is top-to-bottom reading
  order here with zero extra wiring needed.
- **❌ Must-fix, not a suggestion, because R84 is now a standing rule and this
  is new code, not grandfathered code:** the 4-way Baseline selector and the
  em/1000-vs-pt unit toggles MUST NOT use bare `ui.selectable_value` the way
  the pre-existing reflow-alignment row (`main.rs` L7437) and colour-model row
  (L7473-7475) currently do. Those two rows predate R84 (decision 017,
  2026-08-02) and are grandfathered technical debt the GUI-polish audit
  already flagged as "the recurring rule-6 blind spot" — but R84 explicitly
  says "new selection surfaces... must not repeat it." Use `Self::
  toggle_label(selected, text)` (the existing helper, `main.rs` L4395,
  already used for toolbar toggles) paired with `ui.selectable_label`, so the
  selected option is additionally **bold**, never colour-fill alone. This is
  the single clearest ❌-grade finding in this spec — flag it to the
  engineer explicitly, and note in passing (not as new scope) that the two
  pre-existing rows are one easy opportunistic fix away from closing the same
  gap, should the engineer want to bundle it.
- Colour is never the sole signal anywhere else in this spec: the Tw
  read-only row is greyed **and** textually labelled "(read-only)" (shape +
  text, not colour alone); the synthesis preview caption (§3.4, if Option B)
  is plain text with no colour-only state at all.
- Click targets: checkboxes and `DragValue`s inherit egui's default sizing,
  consistent with the rest of this panel (Size/Colour/Font use the same
  unstyled defaults) — no new minimum-size work needed, but note this panel
  has never been audited against the `ICON_BUTTON_SIZE`/click-target rule the
  toolbar follows; that's a pre-existing gap, not one this spec introduces.
- **Known egui/AccessKit gap, honestly named per rule 6:** none of the new
  controls are tabs, so this Pass does **not** run into the `egui_tiles`
  tab-naming gap already tracked in `D:\dev\rag\egui\
  egui_035_no_tab_tablist_widgettype.md`. No new AccessKit gap is introduced
  by this spec as far as I can determine from the widget types used
  (`DragValue`, `Checkbox`, `selectable_label`, `Button` — all standard,
  already-used-elsewhere egui widgets with normal AccessKit roles).

### 6.3 Fuzzy-never-sneaky

- Algorithmic state: `StyleSynthesis` is visibly marked — the disclosure
  strip's `report.disclosures` already includes synthesis facts (stroke
  width, shear angle, the `Ts`×italic displacement) verbatim from core, per
  `FormatReport`'s own doc comments (§798-807 of format.rs). §9 adds a
  **persistent** badge for the *currently selected* run's provenance (see
  below), so the marking survives past the moment of the Apply click.
- Operator can override every suggestion: yes — synthesis is never applied
  without an explicit checkbox-tick-then-click (both options in §4), and the
  ambient captions mean the operator always sees what the CURRENT value is
  before typing over it.
- Manual value always wins on conflict: yes, structurally — the 4-way
  Baseline selector makes the one real internal conflict (`ConflictingRise`)
  unreachable by construction (§3), and every other field is independent.

### 6.4 Immediate-mode fit (egui-specific)

- No retained-mode assumption anywhere: every new field lives on
  `TextEditState` (already a per-frame-rebuilt struct with explicit fields,
  not implicit widget-identity state), following exactly the existing
  `prop_size`/`prop_model`/`prop_font` pattern. The `CollapsingHeader`'s own
  open/closed state is handled by egui's internal `Id`-keyed persistence
  exactly like every other `CollapsingHeader` in this codebase (e.g. the copy-
  confirmation detail expander, `main.rs` ~L2866) — no new pattern needed.
- The live synthesis-preview caption (Option B) is the one place worth a
  specific immediate-mode caution: **do not call `preview_style_resolution`
  every frame unconditionally** — it walks the page's `/Font` resources.
  Recompute it only when `prop_bold`, `prop_italic`, or the caret's target
  run changes (a simple "last-computed-for" cache key on `TextEditState`,
  the same shape `reflow`'s own cached `detected_alignment` already uses).

---

## 7. Direct answers to the brief's six numbered questions

1. **Where does this live?** §0 — the existing `TextEdit` property bar,
   inside a new collapsed-by-default `CollapsingHeader`, not the dock.
2. **Absolute-vs-relative exposure?** §3/§3.1 — a per-field two-way unit
   toggle (`em/1000` default for Tc, `pt` default for the free rise), with
   re-derive-on-switch (never silent reinterpretation) and an always-visible
   ambient caption so the operator can see what the CURRENT value already is
   in whichever unit they're looking at.
3. **Superscript/subscript vs. free-form rise, presented as mutually
   exclusive?** §3 — one 4-way segmented control (`Normal | Superscript |
   Subscript | Custom…`), where choosing `Custom…` is what reveals the
   numeric field and choosing any of the other three hides it entirely. This
   makes the two controls **visually one family with one live member at a
   time**, rather than two separately-enabled/disabled controls the operator
   could be tempted to combine — and it makes `FormatError::ConflictingRise`
   structurally unreachable from this panel (§5.1).
4. **Synthesis offered without becoming a modal interrogation?** §4 — the
   surprising, load-bearing answer, found by reading `gate_synthesis`'s
   actual behaviour rather than assuming a confirm/decline widget was
   needed: **no new dialog mechanism is required at all.** Either a live,
   informational pre-Apply caption (Option B, needs one new thin core query)
   or the panel's *existing* refusal strip (Option A, zero new core surface)
   already constitutes an honest, declinable, non-modal offer. Marking a
   synthesized run afterwards: `StyleSynthesis` is in-session provenance
   (never written to the PDF, per R90/P-selfevident) — surfaced as a
   persistent badge on the property bar whenever the caret sits on a run
   whose `FormatReport`/re-detection says it carries one (§9).
5. **Wording for the three named refusals?** §5.1 — hint-table entries for
   `ConflictingRise` (should-never-fire, given §3's mutual exclusion),
   `RealFaceAvailable` (points at the Font control, verbatim-honest per
   §3.4's corrected wording), and `ShearUnsupported` (points at the same Font
   control as a fallback remedy). The `Tz`×justify case reuses the
   already-present Reflow button in the same panel (§5).
6. **What should NOT be built, and does any absence need explaining?** §8 —
   summarized: no `Tw` authoring (but a mandatory read-only display +
   explanation, §3.3, because showing an inert value with no reason invites
   a "looks broken" report); no combined "apply everything at once" button
   (kept at one-control-per-Apply, matching the existing Size/Colour/Font
   granularity and keeping undo/`ConflictingRise` semantics simple); no
   kerning control (a real, decision-019-flagged parity gap, but its absence
   needs **no** UI explanation — nothing in the existing UI implies kerning
   is adjustable, unlike `Tw`, where a visible-but-frozen value actively
   invites the question); no StructTree/`/ActualText` UI (cut from FF-H
   entirely, §3.7 of the decision — R73's existing tagged-run disclosure
   continues unchanged, nothing new to build); no new icons for the core
   controls (§9).

---

## 8. Prioritized change list

**P0 — must ship for 19.3 to be honest and usable:**
1. The five-row widget tree (§3), inside the new `CollapsingHeader`.
2. Ambient seeding + ambient captions for `Tc`/`Tz`/baseline (§1.2) — without
   this, the panel actively misleads by omission for this control family
   specifically.
3. The read-only `Tw` row with its composite-aware explanation (§3.3).
4. Option A's synthesis flow (§4.2) — zero new core surface, fully honest.
5. R84-safe rendering (`toggle_label` pairing) for the new Baseline/unit
   selectors (§6.2) — new code, standing rule, not optional.
6. The `Tz`×justify hint pointing at the existing Reflow button (§5).
7. New refusal-hint-table entries (§5.1).

**P1 — clear net improvement, not required to ship honestly:**
8. Option B's `preview_style_resolution` core addition + live caption (§1.1,
   §4.1) — upgrades "declinable" from "discoverable after one click" to
   "discoverable before any click."
9. `Ctrl+B`/`Ctrl+I` accelerators (§6.1), re-verified unclaimed at
   implementation time.
10. A persistent "synthesized" badge on the property bar for the caret's
    current run (§9).

**P2 — named, explicitly not required, possible fast-follow:**
11. Composing a real-face family-change for a covered attribute with
    synthesis for an uncovered one in a mixed Bold+Italic request (§1.1's
    "genuine wrinkle") — a core-side change, outside this Pass's mandate.
12. Bundling the pre-existing Size/Colour/Font ambient-seeding gap (§1.2's
    closing note) into the same edit, since it is the same struct and the
    same root cause.

---

## 9. Icon-pipeline determination (per the brief's explicit ask)

**No new icons are needed for any control in §3.** The existing property
bar's own convention — Size/Colour/Font are text-labelled `DragValue`s,
`selectable_value` radios, and a `ComboBox`, with **zero** icon usage anywhere
in this specific panel — is the established local convention, and departing
from it here (e.g. reaching for a "B"/"I" glyph icon on the Style row) would
be inconsistent with the panel's own existing language for no rule-based
reason. A plain `Checkbox::new(&mut prop_bold, "Bold")` renders the word
"Bold," which is exactly as discoverable as an icon and costs nothing to
build — matching this project's own prior "100%-as-plain-text" precedent
(icon-set spec, item 8): a label reads clearer than a glyph substitute when
the label is this short and this unambiguous.

**The one place a badge-shaped indicator is worth naming, and it is NOT an
icon:** a "this run is synthesized" marker (P1, §8 item 10) should follow
Pass 18.4's own **letter-badge** convention (`P`/`T`/`I`/`F` for object kind),
not commission a new SVG — reusing an existing `Icon` variant would misassert
a capability that variant does not have (the exact mistake Pass 18.4's own
finding 3 named: `Icon::Text` denotes the text **tool**, not "this is a text
object," and reusing it for a badge would be dishonest in the same way).
Recommended: a small `colored_label` reading `"◆ synthesized"` (shape glyph +
text, R84-safe, no icon asset), shown beside the Style row whenever the
caret's run carries a non-`None` `StyleSynthesis` — cheaper, more consistent
with the established badge language, and avoids opening the SVG pipeline for
a low-frequency indicator.

**If the engineer later wants toolbar-level Bold/Italic buttons** (a richer,
persistent-toolbar formatting surface, explicitly **not** part of this
Pass — this Pass's controls are per-run, property-bar-scoped, not toolbar
modes), that would be the first genuine icon need in this family, and should
follow the existing ScripTree contract exactly as documented in this
project's own icon-set spec: 48×48 viewBox, `fill="none" stroke="currentColor"
stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"`, ~6-8px
inset margin. A literal bold "B"/italic "I" glyph is a defensible new-draw in
that style (not a vendor-logo, not a trademark risk) — but this is explicitly
future/backlog, named here only so nobody assumes today's icon set already
covers it (it does not — confirmed by reading `icons.rs`'s full `Icon` enum
list, which has no Bold/Italic/superscript/subscript member of any kind).

---

## 10. Add-Text (`CanvasTool::AddText`) implications, briefly

Not the focus of this spec, but named so the engineer doesn't have to
re-derive it: the Add-Text property bar (`main.rs` ~L8257) shares the exact
same `FormatRequest`-adjacent vocabulary at the point of *authoring* new
content (via `AddTextRequest`, a different but sibling type). If/when Add-Text
grows its own Tc/Tz/Ts/style controls, the **same** five-row widget language
applies verbatim, with one difference already established by decision 019
§3.6 itself: the remedy **order** for a synthesis gate differs there (family-
change offered first, synthesis second, since Add-Text defaults to a bundled
Standard-14 face whose family almost always has a real Bold/Italic sibling —
so per §1.1/§4's finding, the gate will rarely even open on that path at all,
and the eventual Add-Text spec should re-verify that against the actual
Std14 font data before assuming the same "no new dialog needed" conclusion
transfers unchanged).

---

## 11. Items for the engineer / KenAgent, not mine to decide

- **Option A vs. Option B (§1.1, §4).** I recommend B; A is a fully honest,
  legitimate P0-only cut. This is a scheduling/scope call — whether 19.3
  reopens `pdfce-core` for one thin query function — squarely the engineer's
  call per my own brief's boundary ("Decide workflow/scheduling questions...
  the engineer's call, not yours").
- **Whether to bundle the pre-existing Size/Colour/Font ambient-seeding fix
  (§1.2's closing note, §8 item 12) into this same edit.** Same file, same
  root cause, but genuinely a separate, already-shipped Pass's gap — not
  mine to fold in unilaterally.
- **The §1.1 "genuine wrinkle" fast-follow** (composing a real-face swap for
  a covered attribute with synthesis for an uncovered one in a mixed
  Bold+Italic request) is a real product question about how much correctness
  a first cut owes a genuinely rare combination — flagging it, not deciding
  it, matching decision 019's own "does not decide" convention for exactly
  this class of question.
- **Whether R84's pre-existing violations in the SAME panel** (the reflow-
  alignment row, the colour-model row — both predate R84 and are grandfathered
  per the GUI-polish audit) **get opportunistically fixed alongside this
  Pass's new, R84-compliant rows**, given how close together all three now
  sit in one file. A reasonable engineer call either way; noted so it isn't
  silently missed.
