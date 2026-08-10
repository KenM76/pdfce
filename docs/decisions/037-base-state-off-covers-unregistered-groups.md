# Decision 037 — `/BaseState /OFF` means every optional content group in the document, registered or not

- **Date:** 2026-08-10
- **Status:** DECIDED. Supersedes the pragmatic shipping choice recorded in
  `ARCHITECTURE.md` §12 (seventy-sixth filing) as
  `base_state_off_with_unregistered`, which was explicitly **not** ratified
  as doctrine at the time.
- **Claimed by:** `ARCHITECTURE.md` §12, 2026-08-10 (seventy-sixth filing).
- **Authored by:** `autonomous-builder` / KenAgent, per
  `docs/decisions/README.md`.
- **Clauses:** ISO 32000-1:2008 §8.11.2.1, §8.11.4.2 (Table 100),
  §8.11.4.3 (Table 101 `BaseState` = ISO 32000-2:2020 Table 99),
  §8.11.4.5 a).
- **Spec RAG:** `D:\Dev\Rag-Specialized\PDF_Spec\iso32000\iso32000__s__8.11.md`
  DA.7, DA-N3, DA-N4, DA.10, DA.14.
- **Code radius:** `pdfce_core::annot::optional_content_default_off`,
  `pdfce_core::annot::oc_is_hidden`, `pdfce_core::layers`,
  `pdfce_render::layer_state::LayerVisibility`, `pdfce-render`'s
  `interpret.rs` / `annot.rs`, `pdfce-cli`'s `resolve_layer_override`,
  `pdfce-gui`'s `layer_visibility` / `set_layer_visible`.
- **Related:** decision 027 (refuse what has no good reading, disclose what
  has one), decision 038 (the sibling `/BaseState` question, decided the
  same day).

---

## 1. The question

Table 101's `BaseState` row and §8.11.4.5 a) both say the base state
initialises **all** groups. `/OCProperties /OCGs` is the catalog's only
enumeration of groups, and Table 100 makes listing every group in it a
`shall`. So when a configuration says `/BaseState /OFF`, does "all groups"
mean:

- **Reading R (registered)** — every group listed in `/OCProperties /OCGs`;
  or
- **Reading A (all)** — every optional content group that exists in the
  document, including one that some content references but that the
  producer failed to register?

`pdfce_core::annot::optional_content_default_off` currently implements
**Reading R**:

```rust
if base_off {
    // All OCGs start OFF; /ON re-enables.
    off.extend(oc_refs(graph, ocp.get(b"OCGs")));   // ← the registry, and only it
    for on in oc_refs(graph, d.get(b"ON")) { off.remove(&on); }
} else {
    off.extend(oc_refs(graph, d.get(b"OFF")));
}
```

Consequence today: under `/BaseState /OFF`, an unregistered OCG-shaped
dictionary reports **VISIBLE**, and the renderer paints its content. The
combination is detected and counted
(`LayerDiagnostics::base_state_off_with_unregistered`,
`LayerDiagnostics::unregistered_groups`), and `crates/pdfce-core/src/layers.rs`
names it in prose as an unresolved caveat.

---

## 2. The ruling

**Reading A. "All groups" is literal: every optional content group in the
document, whether or not the producer registered it in
`/OCProperties /OCGs`.**

And — this is the part that actually decides the implementation — the
reason pdfce got it wrong is not that it picked the wrong *set*. It is that
it represented the answer **as a set at all**.

A `BTreeSet<ObjId>` of OFF groups is a complete answer only if the
complement is genuinely ON. Under `/BaseState` `ON` that holds: the OFF set
is exactly `/OFF`, and every other group in the universe — registered,
unregistered, undiscovered — is ON, so the set needs no enumeration to be
correct. Under `/BaseState /OFF` it does not hold: the true OFF set is
*"every group in the document, minus `/ON`"*, and that is a set pdfce
cannot construct, because "every group in the document" is not enumerable
from the catalog. So the code substituted the one enumeration it had, and
the substitution is exactly the bug.

**The fix is to stop enumerating and start resolving.** The default
configuration is a total function from a group to a state, and it can be
evaluated per group with no enumeration whatsoever:

```
state(g) = if participates(g) then
               if base_off then (if g ∈ /ON then ON else OFF)
                           else (if g ∈ /OFF then OFF else ON)
           else ON        // §8.11.2.3 intent filtering, unchanged
```

Under `base_off`, `is_off(g) = !on.contains(g)` — which is correct for an
unregistered group **automatically**, without knowing it exists. Reading A
turns out to be *cheaper* than Reading R, not more expensive: Reading R is
what required the enumeration.

That is the whole decision. Registration becomes irrelevant to state
computation, which is exactly what the standard says (DA-N3: *no consumer
of an OCG imposes a registration precondition; Table 100 is the sole
locus*).

---

## 3. Why — the reasoning that actually drives it

### 3.1 The text says "in a document", three times, and never says "listed"

All three sourced quotations in the corpus (`iso32000__s__8.11.md` DA.7,
graded SOURCED) use document-scope language and none of them is scoped to
the registry:

| Clause | Wording |
|---|---|
| §8.11.2.1 | "A group **shall** be assigned a state, which is either ON or OFF" |
| Table 101 `BaseState` | "Used to initialize the states of **all the optional content groups in a document**" |
| §8.11.4.5 a) | "The value of `BaseState` **shall** be applied to **all the groups**." |

None reads "all the groups listed in the `OCGs` array". The standard had
the phrase available — Table 100 uses it one sub-clause earlier — and did
not use it here.

### 3.2 Registration is a writer obligation, not a definition of grouphood

Table 100's `shall` ("Every optional content group shall be included in
this array") constrains the **producer**. It does not define what an
optional content group *is*. Table 98 does that, and it requires only
`/Type /OCG` + `/Name` — no registration precondition (DA.7, SOURCED).

So an unregistered OCG is a well-formed OCG that has a state (§8.11.2.1),
whose content's visibility is defined (§8.11.2.1, §8.11.3.3), in a file
that violates a writer rule. Only the *conformance of the file* is
defective; the group's semantics are intact. Reading R answers a question
about the group's state by consulting a rule about the producer's
paperwork.

### 3.3 The standard's own silence points the same way

DA-N4 records that ISO 32000-1 states **no** reader behaviour for a
partially populated `/OCGs` — no "shall be ignored", no "is invalid"; the
word *invalid* does not appear in §8.11 at all. And the contrast is
deliberate-looking: the standard **did** write a failure rule one level up,
in the immediately preceding sentence of the same sub-clause — *"if it [the
`/OCProperties` dictionary] is missing, a conforming reader shall ignore
any optional content structures in the document"* — and wrote no per-group
analogue.

Reading R is, in effect, an extension of that document-level rule to the
per-group case: *an unregistered group's optional-content structure is
ignored, so its content is unconditionally painted*. The corpus explicitly
warns against exactly that extension: *"Do not extend the document-level
rule to the per-group case: it says 'if **it** is missing', not 'if a group
is missing'."*

### 3.4 Reading R is not merely wrong, it is **unstable**

This is the argument that would decide it even if the text were balanced.

Reading R's answer depends on how thoroughly pdfce walked the file. It
resolves "all groups" against whatever enumeration is in hand — today
`/OCProperties /OCGs`; a more ambitious implementation might union in the
DA.10 reference sites the way `layers.rs` does. Each choice yields a
different picture of the same document. `layers.rs` itself names five
reference sites it does **not** walk (`/VE` operands, alternate-image
`/OC`, `SetOCGState` `/State`, Type 3 `/CharProcs` streams, and per-name
content-stream `BDC` resolution), so under Reading R a group's rendered
visibility is a function of the enumerator's coverage — a rendering
decision that changes when an unrelated module gets more thorough.

Reading A has no such dependency. `!on.contains(g)` is the same answer no
matter what pdfce did or did not enumerate. **A rendering rule must not
depend on the completeness of a discovery pass**, and that is decisive
independently of which reading the prose favours.

### 3.5 Reading A is what the OFF configuration is *for*

`/BaseState /OFF` says: hide everything, then show only what `/ON` names.
It is a whitelist. Reading R silently converts it into a whitelist over the
registered groups plus an unconditional allow-list of everything the
producer forgot to register — which is the one population most likely to be
scaffolding, construction geometry, or an editing leftover, since a
carefully authored layer is exactly the layer that got registered.

### 3.6 The cost, stated plainly, and why it is acceptable

Reading A makes content **disappear** that Reading R painted. pdfce's
standing tolerance posture prefers the opposite direction — the corpus
records it for the malformed `/OC` operand case: *"shown-by-mistake is
arguable; hidden-by-mistake is invisible."* That precedent was weighed and
does not reach here, for three reasons:

1. **Different failure.** The `/OC`-operand precedent covers the case where
   pdfce cannot resolve **which group** content belongs to, so hiding would
   be hiding on a guess. Here the group resolves fine; the only thing
   missing is a registry entry. Nothing is guessed.
2. **It is disclosed, and disclosed by a counter that already exists and
   fires on exactly this combination.**
   `LayerDiagnostics::base_state_off_with_unregistered` is set precisely
   when `/D /BaseState` is `OFF` and at least one unregistered group was
   found; `unregistered_groups` counts them; each `Layer` carries
   `in_default_config: false`. Rule 4 is satisfied by disclosure, not by
   painting.
3. **It is reversible in one gesture, per document.** Both front ends
   already carry a per-layer override that replaces the document's
   configuration — `pdfce-gui`'s `set_layer_visible` / `layer_visibility`,
   `pdfce-cli`'s `--show-layer`. The operator who wants the content back
   has a control that says what it does.

The alternative is worse in a specific way: under Reading R the Layers
panel and the page **agree** with each other but both disagree with what
the file said. The file said "everything off"; pdfce shows content and says
it is on. That is not a conservative failure; it is a confident wrong
answer about the document's own intent.

---

## 4. Setting, or defended default? — **DEFENDED DEFAULT. No knob.**

The project's standing posture is that spec ambiguity becomes a setting,
decided deliberately, never settled by whichever implementation shipped
first (R15; `settings/mod.rs`'s `CmykIntent`, `MinifyFilter`,
`CmykJpegPolarity`, `MissingAppearanceState`, `XrefEntryEol`). This is not
one of those. Four reasons, in order of weight:

**(a) There is no ambiguity in the normative text to settle.** R15's
settings exist where the standard genuinely forks — where two readings are
each defensible from the words. Here three concordant clauses say "all the
groups in a document" and nothing says "listed". What the standard is
silent about is the **consequence of a producer violating Table 100**, and
a silence about a violation's consequence is not a fork in the state rule.
Installing a setting would file a determinate clause in the register as
contested, and the register's value depends on every entry being a real
fork. Diluting it is a real cost paid by every future lookup.

**(b) The operator has no basis on which to choose.** The counter-pressure
is exactly right here: a setting the operator cannot meaningfully decide
moves the decision onto someone with less information than the person who
declined to make it. What would the operator observe? "Some content is
missing in one drawing." The correct response to that observation is a
per-document layer toggle, not a global policy that silently changes every
future document — including the conforming ones, where the setting would
have no legitimate effect at all and would sit in `settings.txt` as a
permanent invitation to misconfigure.

**(c) The better mechanism already exists and is per-document.**
`LayerVisibility` REPLACES the document's default configuration by design
(its own module docs: "a COMPLETE answer, not a patch"). It is the
document-scoped, session-scoped, one-click escape hatch. A persisted global
key would be a strictly worse-targeted version of a control that already
ships on both surfaces.

**(d) Reachability does not justify permanent surface.** The case needs a
file that violates **two** `shall`s simultaneously: `/D /BaseState` must be
`OFF` (Table 101: "In the default configuration dictionary, if present its
value shall be `ON`") *and* a group must be unregistered (Table 100). A
persisted setting for a doubly-nonconforming intersection is the paradigm
case of a knob that costs more than the default it replaces.

**Reserved, not installed.** If §6's falsifier fires and this must become a
setting after all, it takes the id **`OC-A2`** (`OC-A1` is reserved by
decision 038), variants `AllGroups` (default) / `RegisteredOnly`, evidence
tier **(a)** at that point — because the only thing that would force the
conversion is observed Acrobat behaviour. Recorded here so a later session
does not mint a colliding id.

---

## 5. Implementation

Nothing below has been applied; this record is the specification for the
change. All of it is in-workspace, so there is no external API to break.

### 5.1 `pdfce-core` — replace the set with a resolver

Introduce a small parsed configuration value and make it the thing callers
hold. Sketch, not final source:

```rust
/// The `/OCProperties /D` configuration, parsed once, as a TOTAL function
/// from a group to its initial state (ISO 32000-1:2008 §8.11.4.3 Table 101
/// = ISO 32000-2:2020 Table 99; §8.11.4.5 a)/b)).
///
/// Deliberately NOT a set of OFF groups. Under `/BaseState /OFF` the OFF
/// set is "every group in the document minus /ON", which is not
/// enumerable from the catalog — see decision 037.
pub struct OcDefaultState {
    base_off: bool,                 // /BaseState /OFF (decision 037)
    on: BTreeSet<ObjId>,            // /ON
    off: BTreeSet<ObjId>,           // /OFF
    config_intent: Vec<Vec<u8>>,    // /Intent, already defaulted to [View]
    ignored: bool,                  // /OCProperties or /D absent (§8.11.4.2)
}

impl OcDefaultState {
    pub fn read<G: ObjectGraph + ?Sized>(graph: &G) -> Self;

    /// Whether `g` is OFF in the document's default configuration.
    /// Correct for a group absent from `/OCProperties /OCGs` — that is
    /// the point (decision 037).
    pub fn is_off<G: ObjectGraph + ?Sized>(&self, graph: &G, g: ObjId) -> bool;

    /// True when `/BaseState /OFF` — i.e. a group this resolver has never
    /// been asked about is hidden, not shown. Front ends need this to
    /// build a complete `LayerVisibility` (§5.3).
    pub const fn unknown_groups_hidden(&self) -> bool { self.base_off }

    /// The OFF set restricted to a caller's enumerated universe, for a
    /// panel or a CLI listing that genuinely wants a concrete set.
    pub fn off_among<G, I>(&self, graph: &G, groups: I) -> BTreeSet<ObjId>;
}
```

`is_off` body, with the §8.11.2.3 intent filter preserved exactly as today
(it is a *participation* test, so it gates both branches identically —
which is precisely why the current code applies it as a `retain` after the
branch, and that structure survives):

```rust
if self.ignored { return false; }                       // §8.11.4.2
if self.config_intent.is_empty() { return false; }      // §8.11.2.3, all visible
if !self.config_intent.iter().any(|i| i == b"All") {
    let gi = group_intent(graph, g);                     // defaults to [View]
    if !gi.iter().any(|i| self.config_intent.contains(i)) { return false; }
}
if self.base_off { !self.on.contains(&g) } else { self.off.contains(&g) }
```

Delete `optional_content_default_off`'s current body. Keep the name only if
it returns `off_among(graph, registered ∪ discovered)` for a caller that
demonstrably wants a set; do **not** keep it as the renderer's channel —
that is the shape that caused this.

`oc_is_hidden`'s `off: &BTreeSet<ObjId>` parameter becomes
`state: &OcDefaultState` (plus the graph it already has). Its internal
`let on = |g| !off.contains(g)` closure — the one that powers the four
Table 99 `/P` policies — becomes `let on = |g| !state.is_off(graph, g)`.
**This is the single most valuable line of the change**, because it makes
`/P /AllOff`, `/AnyOff` and friends correct for unregistered members too,
which Reading R silently broke for every policy at once.

**Cost check.** `is_off` resolves the group dictionary once, for `/Intent`,
where the old code resolved it once inside `retain`. `oc_is_hidden` already
resolves the target dictionary to test for `/Type /OCMD`. Per-render this
is unchanged in order of magnitude; if a profile ever objects, memoize
inside `OcDefaultState` behind a `RefCell<BTreeMap<ObjId, bool>>` — the
value is a pure function of `(graph, g)` for a render's lifetime.

### 5.2 `pdfce-render`

- `interpret.rs` (≈ line 1947): the lazily-initialised `get_or_insert_with`
  cache holds an `OcDefaultState` instead of a `BTreeSet`. Both call sites
  that currently pass `&off` to `oc_is_hidden` (≈ 2008, 2086) pass the
  state.
- `annot.rs` (≈ line 126–165): same substitution. Note the existing
  `None => optional_content_default_off(doc)` fallback becomes
  `None => OcDefaultState::read(doc)`.
- `layer_state.rs` — **`LayerVisibility` inherits the same defect and must
  be fixed with it.** It is a bare `BTreeSet` documented as "a COMPLETE
  answer, not a patch". Under `/BaseState /OFF` a set cannot be complete,
  for exactly the reason in §2: it cannot contain a group nobody
  enumerated. Add the missing bit:

  ```rust
  pub struct LayerVisibility {
      hidden: BTreeSet<ObjId>,
      /// What to answer for a group not in `hidden` and not known to the
      /// caller that built this. `true` under `/BaseState /OFF`
      /// (decision 037); `false` otherwise, which is every conforming
      /// document.
      unknown_hidden: bool,
  }
  ```

  Keep `hiding(set)` as the `unknown_hidden: false` constructor — it stays
  correct for every conforming file — and add
  `hiding_with_default(set, unknown_hidden)`. Without this, the operator
  override path silently reintroduces Reading R the moment anyone toggles a
  layer, which would be the worst of both: correct until touched.

### 5.3 `pdfce-cli` and `pdfce-gui`

Both build the override the same way — start from the document's answer,
apply the operator's changes — and both must now carry the extra bit:

- `pdfce-cli::resolve_layer_override` (≈ 5313) and `pdfce-gui`'s
  `layer_visibility` (≈ 7484): replace
  `let mut hidden = optional_content_default_off(graph)` with
  `let state = OcDefaultState::read(graph)` +
  `let mut hidden = state.off_among(graph, read.layers.iter().map(|l| l.id))`,
  and construct
  `LayerVisibility::hiding_with_default(hidden, state.unknown_groups_hidden())`.
  The GUI already has the enumerated layer list to hand for its panel; the
  CLI already calls `read_layers` for name matching.
- `pdfce-core::layers::read_layers` (≈ 869): `let off = ...` becomes the
  resolver, evaluated per enumerated layer. Its `visible_by_default` column
  then reports the ruling's answer, and the panel and the renderer stay in
  agreement — which is the property the `oc_refs` consolidation
  (`956ef4d`) was fought for and must not be given back.
- **Delete the caveat prose**, don't just amend it:
  `crates/pdfce-core/src/layers.rs` §"The `/BaseState /OFF` +
  unregistered-group caveat" (≈ lines 242–262) describes a divergence that
  will no longer exist. Replace it with a short statement of the ruling and
  a pointer to this record. Keep
  `LayerDiagnostics::base_state_off_with_unregistered` — it stops being a
  *caveat* flag and becomes a *disclosure* flag, which is a better reason
  to exist: the file is doubly nonconforming and the operator should be
  told, whichever way pdfce resolves it.

### 5.4 Tests and fixtures

- **New fixture** `fixtures/synthetic/layers/basestate-off-unregistered.pdf`:
  `/D << /BaseState /OFF /ON [<registered>] >>`, `/OCProperties /OCGs`
  listing only the registered group, plus a second OCG referenced by a
  painted `BDC /OC` section and absent from `/OCGs`. No existing fixture
  combines the two — `unregistered-ocg.pdf` has no `/BaseState` (its `/D`
  is `<< /Order [4 0 R 5 0 R] /OFF [6 0 R] >>`) and `basestate-off.pdf`
  registers everything, which is why this shipped without a red test.
  Rule 7: synthetic, authored here.
- `crates/pdfce-core/tests/layers.rs::base_state_off_is_followed_and_disclosed_from_bytes`
  currently asserts `!base_state_off_with_unregistered` on a fixture where
  it is genuinely false; it stays green. The new fixture asserts the flag
  **true** and `visible_by_default == false` for the unregistered group.
- A render-level test: `render-page` on the new fixture must report the
  unregistered section as hidden (`oc_hidden` incremented) and must not
  paint it.
- The three `optional_content_default_off` unit tests in `annot.rs`
  (≈ 1618, 1699, 1736) migrate to the resolver; the `.len()` assertion at
  1736 becomes an `off_among` over the registered ids, or better, three
  `is_off` assertions naming the groups — a count was always a weak
  assertion for a set-shaped answer.

---

## 6. What would falsify this ruling

Listed in the order they should be checked, cheapest first.

1. **★ Acrobat Reader shows the unregistered group's content under
   `/BaseState /OFF`.** Reader is installed on this machine and is a
   legitimate render-parity tiebreak. Build the §5.4 fixture, open it in
   Reader, look. If Reader paints the content, then the two most relevant
   implementations genuinely differ on a case the standard does not
   adjudicate, and this stops being a determinate clause and becomes a real
   fork — convert to setting **`OC-A2`**, default `RegisteredOnly` at
   evidence tier (a). **This experiment is cheap and should be run before
   the implementation lands, not after.** Note what it cannot show: if
   Reader *hides* the content, that confirms the ruling but does not raise
   its tier above the text, because the text already said so.
2. **ISO 32000-2 attaches a reader behaviour to a partially populated
   `/OCGs`.** The corpus's 2.0 sweep (DA.9) covers clause 8 via the PDF
   Association's public change record and reports no such amendment, but
   the record is a secondary source and §8.11.4.2 was not examined
   line-by-line for this question. A `pdfce-spec-librarian` pass that finds
   an added sentence overturns §3.3 outright.
3. **A file census showing unregistered OCGs are common and are
   scaffolding-shaped.** That would *strengthen* the ruling, not falsify
   it — recorded because it is the observation most likely to be
   volunteered as an objection ("this will hide a lot of content"), and the
   answer is that hiding scaffolding under an explicit `/BaseState /OFF` is
   the requested behaviour.
4. **A profile showing per-group `is_off` resolution is hot.** Falsifies
   the *mechanism*, not the ruling — the answer is memoization inside
   `OcDefaultState` (§5.1), not a return to the set.

---

## 7. Side findings from this consultation (report only, not part of the ruling)

- **`pdfce-gui/src/main.rs` ≈ line 7440 carries what appears to be a stale
  doc comment**: "the renderer takes no visibility override, and there is
  no save path for one. So the panel lists and does not offer a checkbox."
  The renderer *does* take an override (`LayerVisibility`, consumed at
  `pdfce-render/src/annot.rs:133`), and the GUI *does* build one
  (`layer_visibility`, `set_layer_visible`, `layer_overrides`). Whether the
  panel currently renders a checkbox was not verified; the doc comment
  should be checked against the shipped panel and corrected either way.
  This is the same shape as the `git remote` error CLAUDE.md rule 8 warns
  about — a document asserting a capability fact nobody re-measured.
- **The corpus's own recommendation for the sibling question is wrong** —
  see decision 038 §6. `iso32000__ref__optional_content_order.md` §5 and
  `index.md`'s DA-A10 row both recommend "process both arrays", and the
  order-of-application sketch there (`/OFF` then `/ON`) contradicts shipped
  behaviour in the `/D` configuration pdfce actually reads. Routed to
  `pdfce-spec-librarian`; not edited here, since that agent owns the RAG
  and is running concurrently.

---

## 8. References

- ISO 32000-1:2008 §8.11.2.1, §8.11.4.2 Table 100, §8.11.4.3 Table 101
  (= ISO 32000-2:2020 Table 99), §8.11.4.5 a).
- `D:\Dev\Rag-Specialized\PDF_Spec\iso32000\iso32000__s__8.11.md` — DA.7
  (the registration `shall` verbatim + the defensibility chain), DA-N3,
  DA-N4, DA.10 (the fourteen OCG reference sites), DA.14.
- `docs/ARCHITECTURE.md` §12, seventy-sixth filing — the claim this record
  discharges.
- `docs/decisions/027-refuse-what-has-no-good-reading-disclose-what-has-one.md`
  — the governing posture: this case *has* a good reading, so it is
  resolved and disclosed rather than refused.
- `docs/decisions/038-basestate-array-propagation.md` — decided the same
  day, same clause, and shares the `OcDefaultState` change.
- `CLAUDE.md` rules 1 (spec fidelity), 3 (round-trip: pdfce reports the
  malformation, never repairs `/OCGs` on save), 4 (fuzzy never sneaky),
  15 (this record concerns **neither** pdf dimensions nor ce dimensions —
  no dimension terminology arises).
