# Decision 030 — Preserving the option of a future plugin system: build nothing, name what would foreclose it

**Status:** **DECIDED — outcome is BUILD NOTHING.** No plugin system, no
plugin host, no ABI, no `[features]`, no new trait, no new `pub` item.
The whole deliverable is this record.
**Date:** 2026-08-05
**Requested by:** the operator, 2026-08-05: pdfce will **not** build a
plugin system now, and he asked that decisions taken in the meantime not
make an **optional, bulky** plugin add-on harder or impossible later.
**Filed by:** `autonomous-builder` (KenAgent), dispatched by
`pdfce-engineer`.
**Tree state at analysis time:** worktree at **`7c45bf8`** ("Pass 26.1:
Bézier handles are visible and grabbable"), one commit behind
`5670451`. Every measurement below was read out of that tree, not
recalled.

**★ WHY A RECORD EXISTS FOR A DECISION THAT BUILDS NOTHING.** Same
reasoning as decision 029, one step further. 029 recorded a *no-change*
outcome because the next person to see the clean crate split would
propose partitioning by it. This record exists because the option it
protects is currently preserved **by luck and by unrelated good design**,
and the properties that preserve it are **not defended by anything**. No
test fails if they break. None of the five gates fires. The value of the
record is almost entirely in §5 — *what would destroy it* — and the
single most useful sentence in the whole file is that each of those
destructive moves is a **natural-looking refactor that would ship
green**.

**★ PROVENANCE — this record IS consultant output, unlike 027 and 029.**
`docs/decisions/README.md` describes these files as the Markdown half of
the `autonomous-builder` decision-consultant protocol. 027 and 029 both
deviated (engineer-made, librarian-filed) and said so up front. This one
follows the protocol as written: the engineer dispatched the consultant,
the consultant read the tree, and this is the Markdown return.

**★ DECISION NUMBER 030 — verified against the live ledger, and it
SPENDS A NUMBER THAT WAS EARMARKED.** At write time the **shared
checkout's** `docs/decisions/` contains `001`–`029` with no gaps, so
**030 is the next free number**. It is not free by accident: **decision
029's own numbering section explicitly reserved 030 for a possible Pass
33.0 record** — *"029 is now spent on this record, so that Pass 33.0
record, if it is ever written, takes 030."* **030 is now spent here**, so
a Pass 33.0 record takes **031**, and R148's superseding note in
`ROADMAP.md` needs the same correction 029 made one number earlier. That
correction is **owed to the librarian** (§9), not made here —
`ROADMAP.md` is not this record's territory.

**★ AND A WORKTREE ARTEFACT THAT MUST NOT BE MISREAD AS A GAP.** This
file was written from an isolated worktree pinned at `7c45bf8`, which
**predates decision 029's filing**: `docs/decisions/` *in that worktree*
ends at `028`. So `python tools/check-ledger-numbers.py --stats` run
there reports `decision records: 028 -> next free is 029` **before** this
file exists and would report a **gap at 029 after it**. Both readings are
stale and neither is the authority. The authority for 030 is the shared
checkout's listing, read immediately before this file was written.
**Re-run the checker in the shared checkout once this file and the
concurrent librarian edits have both landed** (§9 item 5).

**★ A RACE IS ACKNOWLEDGED RATHER THAN ASSUMED AWAY.** A librarian was
editing `ROADMAP.md` and `ARCHITECTURE.md` concurrently with this
filing. Nothing prevents two writers reading the same ceiling seconds
apart — **which is decision 029 §3's argument, firing on the very
session that recorded it.** This record touches exactly one file,
creates it rather than edits it, and writes to neither ledger, which is
the only reason the race is survivable here.

**Terminology (`CLAUDE.md` rule 15).** This record never writes bare
"dimension." Where it appears: `crates/pdfce-core/src/dimension/` (8
files, 4,539 lines) is the **ce-dimension** subsystem — the `/Line` +
`/IT /LineDimension` annotations pdfce authors, their `/Measure` dict and
their `/PieceInfo` sidecar. **pdf dimensions** — the ones a CAD exporter
already baked into page content — are not this record's subject at all,
except in §4.6, where the ce-dimension sidecar's own survival caveat is
used as evidence about `/PieceInfo` generally.

---

## 0. Summary

| | |
|---|---|
| **Question** | Do decisions being taken now foreclose an optional, bulky plugin add-on later? |
| **Outcome** | **No plugin system is built.** The option is currently open. |
| **Why the option is open** | Three properties, all verified in-tree (§3) — and **all three are true for reasons unrelated to plugins.** |
| **What is actually at risk** | Not the three properties. §4.1 found a **fourth fact the request's analysis missed**, and it inverts the framing. |
| **The inversion** | The `EditSession` road is closed to an external crate. **The `DirtySet` road is already wide open** — `DirtySet::empty()`, `.replace()`, `.delete()`, `.patch_trailer()`, `.set_staging()` and both `writer::save_*` free functions are all `pub` today. A plugin author does not need a plugin system to mutate a pdfce document. They need one to do it **with undo, disclosures, and the R35/R46 discipline.** |
| **Recommended guardrail** | **One standing rule + one ~40-line grep gate in the existing `tools/` idiom**, aimed at §4.1's open road — **not** at the three properties. Cost is under an hour; it adds detection, not ceremony. **Recommended now.** |
| **Position on cargo features** | **Not now, with one narrow exception.** Introducing `[features]` as a way to make module boundaries "real" is premature and buys a maintenance burden the project cannot currently pay. §7. |
| **Licence implications** | **None yet**, and the one that would arise later is named in §8 so it is not discovered at implementation time. |

---

## 1. What is decided

1. **No plugin system is built.** Not a host, not an ABI, not a trait, not
   a `[features]` table, not a `pub` item, not a scaffold, not a stub.
2. **No existing code changes as a consequence of this record.** The
   guardrail in §6 is a *recommendation* that the operator may take or
   leave; it is not a task this record authorises.
3. **The three enabling properties are recorded as properties**, so that
   a future session breaking one does so **knowingly**. Today it would
   not even know they exist.
4. **The foreclosing moves are named** (§5). This is the deliverable.

**What is deliberately NOT decided:** what a pdfce plugin would *be*
(process, dynamic library, WASM guest, script), what it could do, who
could publish one, or whether one is ever built. §8's licence note is
conditional on an ABI shape and takes no position on which shape.

---

## 2. Measured structure — the module map, and the one convergence point

Read from `crates/pdfce-core/src/` at `7c45bf8`. **Every figure below is
a measurement, not an estimate.**

### 2.1 The module map IS per-feature-group

| Module | Files | Lines |
|---|---:|---:|
| `text_edit/` | 9 | 16,434 |
| `vector/` | 7 | 8,151 |
| `text_extract/` | 5 | 6,090 |
| `image_codec/` | 9 | 6,036 |
| `writer/` | 7 | 5,169 |
| `dimension/` (ce dimensions) | 8 | 4,539 |
| `fontdata/`, `pageops/`, `filters/` | — | — |
| top-level: `redact.rs` / `annot_author.rs` / `forms.rs` | 3 | 2,523 / 1,958 / 1,772 |

**This matters for exactly one reason, and it is worth stating plainly
so it is not over-read:** if a plugin system were ever built, the
question *"what would the first plugin have been carved out of?"* has an
answer that is already a directory. That is a **convenience**, not a
capability — the boundaries are conventional, enforced by nothing
(§7.1), and never verified as separable (§7.2).

### 2.2 One convergence point — `edit.rs`, 8,880 lines, 67 public methods

`crates/pdfce-core/src/edit.rs` is the largest file in the crate by more
than 3.5× (`redact.rs` is second at 2,523). It carries **67 `pub fn` /
`pub const fn`**, spread across **three separate `impl EditSession`
blocks** (`edit.rs:1047`, `:3216`, `:6231`) plus two on `InfoField`
(`:191`) — so **65 of the 67 are `EditSession`'s own surface.** Every
mutating feature adds a method here plus a `CommandKind` variant.

**This is stated as the intended tradeoff, not as debt**, and the
distinction is load-bearing. The convergence is what buys:

- **one undo stack** (`edit.rs:1304`),
- **one save path** (`dirty_set` → `writer::save_*`),
- **one certification gate** — R46's content-stream identity corpus gate
  applies because every content edit passes through the same serializer,
- **one enforcement point for minimal-diff** (`CLAUDE.md` rule 3,
  `ARCHITECTURE.md` §5).

The counter-evidence that this is a good trade rather than accumulated
sprawl is `vector_surgery` (`edit.rs:2506`): a single private skeleton
shared by five vector operations, which is why **adding `move_handle`
in Pass 30.1 cost roughly thirty lines.** A codebase where each vector
verb had its own path would have charged five times that and would have
five places to get the undo record wrong.

**But §4.2 shows the "every mutating feature" claim is false**, and the
exception is the interesting one.

---

## 3. The three properties that preserve the option — verified, then challenged

### 3.1 Property 1 — undo is DATA-DRIVEN, not TYPE-DRIVEN. **Verified; strongest of the three.**

`EditSession::undo` (`edit.rs:1304-1318`) is nine lines and contains **no
`match` on the command kind at all**:

```rust
pub fn undo(&mut self) -> Option<CommandKind> {
    let command = self.undo.pop()?;
    for write in &command.objects {
        Self::write_state(&mut self.state, write.id, write.before.clone());
    }
    for removal in &command.removals {
        Self::write_deleted(&mut self.deleted, removal.id, removal.was_deleted);
    }
    if let Some((before, _)) = &command.trailer {
        self.trailer = before.clone();
    }
    let kind = command.kind;
    self.redo.push(command);
    Some(kind)
}
```

`redo` (`:1321-1335`) is its exact mirror, reading `after` /
`is_deleted` instead. **`CommandKind` is only a LABEL** — it is popped
off, returned, and never inspected. `undo_kind()` / `redo_kind()`
(`:1288`, `:1294`) exist solely so a front end can title its Undo
control, and the type's own doc comment says why it is structured data
rather than a string (decision 002 R1/R4 — an English label in
`pdfce-core` would be in the wrong crate).

**The plugin consequence, which is the whole point:** an operation
authored outside core, if it could produce the same `ObjectWrite`
records, would **participate in undo without core knowing what the
operation was.** Nothing in the undo path needs to be extended.

**Why this is real luck and not hindsight.** The obvious implementation
— `match command.kind { SetInfoField(f) => …, DeletePages { .. } => …, }`,
reversing each case by hand — is what most editors do, and it would be
**structurally impossible** for a plugin to take part in: a plugin
variant would land in a `match` arm inside core that core cannot write.
Unwinding that later would mean rewriting every undo case in the
codebase while preserving each one's exact semantics, with the tests
that pin them written against the type-dispatch shape. That is a rewrite,
not a refactor.

**Challenge — the property is real, but the request's claim that it is
undefended is CORRECT, and I could not rescue it.** I looked for an
existing test that would fail if undo were rewritten to dispatch on type,
because that would have been the cheapest possible good news. There
isn't one.

- `undo_history_is_bounded_without_affecting_the_diff` (`edit.rs:7468`)
  looked like the candidate: `MAX_UNDO_DEPTH` may drop the oldest command
  **only** because the dirty set is a diff rather than a replay
  (`edit.rs:150-160` states this explicitly). But the test never calls
  `undo()`. It asserts `undo_depth() == MAX_UNDO_DEPTH` and
  `dirty_set().len() == 1` after 266 edits. A type-dispatch undo would
  pass it untouched.
- `undo_and_redo_return_what_they_moved` (`:7485`) and
  `undoing_the_creation_of_info_removes_the_object_and_the_reference`
  (`:7235`) do exercise undo — but a **correctly written** type-dispatch
  reimplementation passes both. They pin the *behaviour*, and the
  behaviour is identical. They cannot see the shape.

**So the honest finding is: a competent engineer could rewrite undo as
type-dispatch, get every test green, get all five gates green, and
silently close the door.** The only thing standing in the way today is
the doc comment at `edit.rs:1298-1303` — which a rewrite would delete as
part of the rewrite.

### 3.2 Property 2 — `CommandKind` is `#[non_exhaustive]`. **Verified, and MUCH weaker than claimed.**

`edit.rs:220-221` confirms the attribute, and it is not lonely:
`InfoField` (`:179`), `Object` (`object.rs:259`) and at least six more
core enums carry it. The narrow claim is true — **consumers already
cannot match `CommandKind` exhaustively, so adding a plugin variant later
is not a breaking change.**

**Challenge: `#[non_exhaustive]` on `CommandKind` is close to irrelevant
to whether plugins are possible, and treating it as one of three
load-bearing properties overstates it.** Three reasons, in ascending
order of importance.

1. **It solves a problem that only exists after publication.** There is
   no git remote and nothing outside this workspace consumes
   `CommandKind` (`CLAUDE.md` rule 8). Until pdfce publishes, *every*
   enum in core could gain a variant tomorrow at the cost of a
   `cargo build`.

2. **The blocker is not the enum, it is the three PRIVATE TYPES around
   it.** This is the correction that matters, and the request located the
   seam one level too shallow. It named `commit()` (`edit.rs:2690`) and
   `write_state` (`:2719`) as the private functions. They are private —
   but so are the **types they operate on**:

   | Item | `edit.rs` | Visibility |
   |---|---:|---|
   | `Command` | 486 | **private struct** |
   | `ObjectWrite` | 510 | **private struct** |
   | `Removal` | 518 | **private struct** |
   | `CommandKind` | 221 | `pub` |

   So `CommandKind` is the *only* public member of the family. Making
   `commit()` public would accomplish nothing — its parameter type does
   not exist outside the module. The future work is publishing a **type
   surface**, and `#[non_exhaustive]` on the label enum is a detail of
   that work rather than a precondition for it.

3. **A plugin cannot construct a stream `Object` at all, and this is the
   genuinely non-obvious blocker nobody has named.** `Stream`
   (`object.rs:246`) holds a **`ByteSpan` into a retained buffer**, not
   owned bytes — the doc comment at `:236-244` is explicit: *"the data is
   not copied out at parse time."* Authored stream bytes go through
   `EditSession::stage_bytes` (`edit.rs:5401`), which appends to the
   session's staging buffer and returns a span in the **combined
   `base.len() + local` coordinate system** (R45). `stage_bytes` is
   **private**.

   **Consequence:** a hypothetical plugin API shaped as *"return me an
   `Object` and I will commit it"* **cannot express any operation that
   authors a content stream, an appearance stream, or a font program** —
   which is most of what a bulky add-on would want to do. The API must
   be shaped as *"hand me bytes, I will stage them and hand you back the
   span"*. That is still additive and still cheap, but it is a
   **different API shape from the obvious one**, and discovering it at
   implementation time would cost a redesign.

   The mechanism is not exotic — `redact.rs:1227` has its own local
   `stage()` doing exactly the same arithmetic against its own buffer, so
   the pattern is already duplicated once in the tree.

**Net on property 2:** keep the attribute; it costs nothing. But do not
count it as one of three things holding the door open. **The door is
held by property 1 and property 3.** Property 2 is a courtesy to a
future publish.

### 3.3 Property 3 — core is deliberately WASM-portable, so a host must live in a SHELL. **Verified; the consequence is the sharpest thing in the request.**

`crates/pdfce-core/Cargo.toml` pins `flate2` to `rust_backend` with an
in-manifest comment that the C backends *"would break the
static-single-binary packaging invariant (ARCHITECTURE.md §6) and the
WASM fork — this is the dependency selected by
`docs/decisions/001-oxidize-pdf-adopt-vs-build.md` §6.2 … never switch
backends without a decision-log entry."* The same discipline recurs on
every codec: `zune-jpeg`, `hayro-ccitt`, `hayro-jbig2` and
`hayro-jpeg2000` are all `default-features = false`, and the JPEG comment
notes *"`std` is kept: pdfce-core is a std crate and
wasm32-unknown-unknown supports std."* **Four dependencies, four
independent statements of the same intent.** This is not incidental.

**The plugin consequence the request states, and which I agree is the
single most useful architectural sentence available today:**

> **The plugin host must live in a SHELL crate, never in `pdfce-core`.**

Because the web fork runs core *inside* WASM, a WASM plugin host inside
core would mean **nested WASM** — a guest engine compiled into a guest.
It is not merely undesirable; on `wasm32-unknown-unknown` it does not
work, and the failure arrives at the end of a long build.

**Challenge — the property holds, but its enforcement is weaker than the
GUI-core rule it resembles, and the request did not notice the gap.**
`CLAUDE.md` rule 2 gives GUI-core separation a **runnable check**:
`cargo tree -p pdfce-core`, run on any Pass touching those manifests, and
`ROADMAP.md` shows it actually being run (e.g. the Pass gates at lines
1745 and 1818). WASM portability has **no equivalent check**. There is no
`cargo check --target wasm32-unknown-unknown` in CI, in `tools/`, or in
any gate. The four manifest comments are prose, and prose is not a gate.

The four `std::fs` call sites (§4.7) are the visible evidence: they are
already there, already non-portable, and nothing noticed.

---

## 4. Challenges — four things the analysis got wrong or missed

Requested explicitly, and the request was right to ask. **§4.1 is the one
that changes the conclusion.**

### 4.1 ★ THE LOW ROAD IS ALREADY OPEN — `DirtySet` is fully public and an external crate can already mutate a document

This is the finding. The request's model is: *core's mutation surface is
`EditSession`; `commit()` is private; therefore nothing outside can
produce a command; therefore the seam is closed and opening it is future
work.* **The first clause is false.**

Measured in `crates/pdfce-core/src/writer/`:

| Item | Location | Visibility |
|---|---|---|
| `DirtySet` (struct) | `writer/mod.rs:214` | **`pub`** |
| `DirtySet::empty()` | `:270` | **`pub`** |
| `DirtySet::identity_reemission()` | `:296` | **`pub`** |
| `DirtySet::replace(id, Object)` | `:317` | **`pub`** |
| `DirtySet::delete(id)` | `:349` | **`pub`** |
| `DirtySet::patch_trailer(name, obj)` | `:384` | **`pub`** |
| `DirtySet::set_staging(Vec<u8>)` | `:464` | **`pub`** |
| `writer::save_incremental(...)` | `writer/save.rs:257` | **`pub`** |
| `writer::save_full(...)` | `writer/save.rs:499` | **`pub`** |
| `Document::save_incremental` / `save_full` | `document.rs:739`, `:770` | **`pub`** |

**`set_staging` being public is the decisive one.** It is precisely the
capability §3.2(3) established a plugin needs and cannot get through
`EditSession` — and it is available on the writer path to any consumer.
An external crate can today: load a `Document`, build a `DirtySet` from
scratch, `replace` arbitrary objects, `delete` others, `patch_trailer`,
attach a staging buffer of authored stream bytes, and write the file.

**So a plugin does not need a plugin system to mutate a pdfce document.
It needs one to do so with:** undo (`EditSession`'s command log),
disclosures (`PlannedEdit::disclosures`, decision 027 §3, R145), the
save-mode obligations (§4.3), and the R46 serializer discipline.

**Why this inverts the framing.** The request asks *"what would make a
plugin system harder later?"* The honest answer is that the greater risk
is **not foreclosure**. It is that the low road is open, the high road is
closed, and **a future plugin author will take the open one** — arriving
at a design where plugin edits bypass undo entirely, because that is the
only road that was available when they looked. That design would then be
load-bearing and expensive to undo, and it would have been chosen by
default rather than decided.

**This is not a call to close `DirtySet`.** It is public for good
reasons: `redact.rs` needs it (§4.2), the R46 identity gate
(`tools/content-identity/`) needs `identity_reemission`, and
`ARCHITECTURE.md` §5's save model is expressed in these terms. **The
recommendation in §6 is to make the road's existence VISIBLE, not to
barricade it.**

### 4.2 ★ REDACTION ALREADY BYPASSES `EditSession` — so "every mutating feature adds a method plus a `CommandKind` variant" is false

The request states this as a structural fact about the codebase. It is
true of every mutating feature **except the destructive one**, and the
exception is the one a plugin analysis most needs.

Measured: `redact.rs`'s production apply path (`:1036-1217`) builds its
own `DirtySet` and its own staging buffer, calls its own local `stage()`
(`:1227`) and `make_raw_stream()` (`:1235`), and ends at
`save_full(doc, &dirty, options)` (`:1215`). **`EditSession` appears in
`redact.rs` six times, all inside `#[cfg(test)]`** (first at `:1926`, in
a `use` within the test module). The shipping redaction path never
touches the session.

**Two consequences.**

1. **A second mutation path already exists in core, and it is the
   template a plugin with removal semantics would follow.** So the
   question is not only *"how does a plugin produce a `Command`?"* — it
   is also *"does a plugin get the redaction-shaped escape hatch, and if
   so, what enforces the obligations that hatch carries?"* Today those
   obligations live in `redact.rs` as code, not in any type.

2. **Redaction's edits are not undoable, by construction**, and the
   module docs are explicit that this is deliberate (`redact.rs:6`,
   `:48`, `:1057` — *"the one deliberately destructive operation in
   pdfce (R35)"*). A plugin system that promised "plugin operations
   participate in undo" would be promising something core's own most
   consequential operation does not do.

**This does not weaken property 1** — it narrows its scope. Property 1
says *an operation expressed as object writes joins undo for free*. It
does not say every operation can be so expressed. Redaction cannot,
because its contract is removal and removal requires a save mode, not a
write set. §4.3.

### 4.3 ★ THE SAVE-MODE OBLIGATION IS A FOURTH FORECLOSURE RISK, AND IT IS NOT CARRIED BY ANY TYPE

Requested: *does the R46 minimal-diff invariant or the
incremental-save/signature model constrain what a plugin could be allowed
to do?* **Yes, more than the request anticipated, and the constraint has
no home in the type system.**

`Document::save_incremental` (`document.rs:715-747`) documents its own
hazard:

> ⚠️ **Incremental save structurally preserves superseded content.** The
> old bytes of every replaced object remain in the file by construction.
> Any operation whose contract is *removal* — redaction above all — must
> therefore use `Document::save_full` and must refuse this mode (R35).

And `save_full` (`:749-780`) documents the mirror hazard:

> ⚠️ **A full rewrite invalidates every existing digital signature**,
> because a signature covers a byte range that this mode necessarily
> disturbs (§12.8.1). That collides with R35's requirement that
> redaction use this mode; "redact a signed document" is a genuine
> either/or for the operator to resolve, never something pdfce decides
> silently (R36, decision 007 W7).

**The trap, stated precisely.** Incremental save is the **default**
(`ARCHITECTURE.md` §5) because it keeps a pre-existing signature's
`/ByteRange` digest valid (**ISO 32000-1 §12.8.1 NOTE 1**). So:

- A plugin operation with **removal** semantics that goes through the
  ordinary command path gets the **default** save mode and **silently
  leaves the removed content recoverable in the prior revision.** The
  operation appears to succeed. The file appears correct. The removed
  bytes are still in it.
- A plugin operation that forces `save_full` to be safe **silently
  invalidates every signature on the document** — and §12.8.1 makes that
  unrecoverable, not a warning.

**`CommandKind` carries neither obligation.** It is a label with no
save-mode field, no destructiveness flag, no signature-interaction
declaration. Today that is fine, because there are 65 methods and one
author, and the two operations that care (redaction, flatten — R48)
hard-code their own path. **It stops being fine the moment an operation
can be authored by someone who has not read `redact.rs`.**

**This is the fourth foreclosure risk, and it is different in kind from
the other three.** The other three are about *whether a plugin could
exist*. This one is about *whether a plugin could be safe*, and it is the
one that would produce a wrong file rather than a compile error. A
plugin API that lets an operation say *"my contract is removal"* is
**additive** and can be added later — but if the API ships without it,
every plugin written before the omission is noticed is a candidate
`/ByteRange` or superseded-content defect.

### 4.4 The `Object` / `ObjId` model is FINE, and the reason is worth stating so nobody "fixes" it

Requested explicitly. **`Object` (`object.rs:260`) is `#[non_exhaustive]`
and a plugin still cannot add a variant to it — and it must not be able
to.**

**Why that is correct rather than a limitation:** `Object` models
**ISO 32000-1 §7.3.1's eight basic types plus the indirect reference**,
and that set is **closed by the standard**. A plugin variant would be an
object that no conforming PDF reader can parse and that pdfce's own
writer could not emit. The extensibility a plugin actually needs is at
the **semantic** layer — new dictionary keys, new `/Subtype` values, new
`/PieceInfo` entries (§4.6) — and `Dict` is `pub struct Dict(pub
Vec<(Name, Object)>)` (`object.rs:143`), i.e. **an open map with a public
field.** Anything a plugin wants to say, it says there.

`ObjId` (`:62`) is `Copy` + `Ord` + `Hash` with `u32`/`u16` fields sized
to §7.5.4's ranges. It is a value type with no reservation scheme and no
namespace, and it needs none: object numbers are allocated from
`Document::next_object_number` (`document.rs:781`+), which takes the
maximum of three sources precisely so that a newly created object never
collides. A plugin allocating through that function is safe; a plugin
inventing numbers is not — **and that is a documentation obligation for a
future plugin API, not a change needed now.**

`Dict`'s ordered-`Vec` backing (`:133-142`) is a **minimal-diff**
decision, not a performance one: *"preserves the parsed entry order so
that re-serializing a modified dictionary perturbs sibling entries as
little as possible."* A plugin that rebuilt a `Dict` rather than mutating
it in place would silently enlarge every diff it touches. Also a future
documentation obligation.

### 4.5 ★ GUI-core separation HAS a plugin analogue, and it should be stated NOW because it is free now and expensive later

Requested. **Yes — and it is §3.3's consequence promoted from an
observation to a rule.** The proposed wording:

> **A plugin host is a SHELL-crate concern. `pdfce-core` and
> `pdfce-render` never gain one, for the same reason and by the same
> check as rule 2's windowing prohibition: the web fork runs core inside
> WASM, and a plugin host inside core is a guest engine compiled into a
> guest.**

**Why state it now rather than when a plugin system is scoped.** The
rule costs one paragraph today. Stated later, it arrives **after**
someone has put a host in core — because putting it in core is the
*obvious* choice. Core is where the object model is, where `EditSession`
is, where a plugin's edits have to land. A host in the shell has to reach
back into core through an API that does not exist yet, so the shell
placement looks like the harder design **until you know about the WASM
fork**. That is exactly the shape of knowledge that a standing rule
exists to carry.

**It is also the cheapest of all the guardrails**, because unlike
property 1 and property 3 it needs no new mechanism to detect: whichever
crate a `wasmtime`/`wasmer`/`libloading` dependency lands in is visible
in a manifest diff, and `cargo tree -p pdfce-core` — **already run on
every manifest-touching Pass under rule 2** — would show it.

### 4.6 `/PieceInfo` is a real data home, but the request's framing omits its documented survival caveat

The request says a plugin has *"a standards-blessed place to put its
per-document data that requires no core change."* True, and
`crates/pdfce-core/src/dimension/sidecar.rs` proves the pattern works —
`/PieceInfo /pdfce /Private`, **ISO 32000-1 §14.5 Table 319**, with
forward-compatible unknown-key handling (`sidecar.rs:27`, `:40`).

**The omitted half is in the same file's own docs** (`sidecar.rs:10-18`),
and pdfce already had to design around it:

> *"Why `/PieceInfo` is authoritative but its cross-tool survival is not
> spec-guaranteed … Per `iso32000__s__14.5.md` (NOTE 1): private
> `/PieceInfo` data **may be discarded** [by other applications] … That
> is exactly why the load-bearing scale is [also carried in the
> `/Measure` dict]: if a foreign editor drops `/PieceInfo`, the [ce
> dimension still measures correctly]."*

**So the correct statement is:** `/PieceInfo` is the right home for
plugin state, **and any plugin state that must survive a round trip
through a foreign editor needs a second home in spec-visible structure**
— exactly the hedge the ce-dimension subsystem already makes. A future
plugin API's documentation owes this warning, because the failure mode
is silent and delayed: the plugin works, the operator opens the file in
Acrobat and saves, and the plugin's state is gone with no error anywhere.

### 4.7 Friction items — confirmed as friction, with one correction

All three of the request's "additive to fix later" items check out.

- **`commit()` / `write_state` private** — confirmed (`edit.rs:2690`,
  `:2719`). **Correction per §3.2(2)–(3):** the seam is wider than these
  two functions. It is `Command` + `ObjectWrite` + `Removal` +
  `stage_bytes`, four private items, of which `stage_bytes` is the one
  that determines the API's *shape* rather than its *visibility*.
- **Four `std::fs` call sites in core** — confirmed exactly:
  `document.rs:252` (`from_bytes(std::fs::read(path)?)`), `:746` and
  `:777` (`std::fs::write` inside `save_incremental` / `save_full`), and
  `lib.rs:262` (`File::open`). All four are thin wrappers over a
  byte-oriented API that already exists underneath — `Document::from_bytes`,
  `writer::save_incremental` and `writer::save_full` all take and return
  bytes. **A fifth grep hit at `text_edit/addtext.rs:610` is inside a
  doc-comment example, not code.**
- **No `[features]` anywhere** — confirmed across all four crate
  manifests. §7.

---

## 5. ★ WHAT WOULD DESTROY THE OPTION

This is the section the record exists for. **Each of the four moves below
is a plausible, well-intentioned refactor. NONE would fail `cargo test`,
`cargo fmt --check`, `cargo clippy -D warnings`, `check-ui-strings.sh`,
or `check-ledger-numbers.py`.** All five gates would be green. A reviewer
reading the diff would see an improvement.

### 5.1 Rewriting undo to dispatch on the command TYPE

**How it would arrive:** as a readability or performance change. *"Undo
clones an `Option<Object>` per write; a typed reverse-operation would be
cheaper and clearer."* It is not an unreasonable thing to think while
reading nine lines that clone in a loop.

**What it costs:** everything in §3.1. A plugin variant would need a
`match` arm inside core that core cannot write. Reversing the change
later means rewriting every case while preserving semantics that are
pinned only by behavioural tests.

**Detection today: none.** §3.1's search for a test that would fire came
back empty.

### 5.2 Giving `Command` typed payloads only core can construct

**How it would arrive:** as type safety. Replacing `objects:
Vec<ObjectWrite>` with a per-operation payload enum makes each command
self-describing and lets the compiler check that a rotation carries a
rotation. That is a genuinely attractive property, and it is what a
careful engineer would reach for while adding the sixty-eighth method.

**What it costs:** it converts §3.1's data-driven undo into §5.1's
type-driven undo **as a side effect**, without anyone deciding to. This
is the more dangerous of the two because the undo change is *derived*
rather than *proposed* — the diff is about `Command`, and undo just
follows.

**Detection today: none.**

### 5.3 Putting a plugin host in `pdfce-core`

**How it would arrive:** as the obvious placement (§4.5). Core is where
the edits land.

**What it costs:** nested WASM in the web fork; the failure surfaces at
the end of a `wasm32-unknown-unknown` build, long after the design is
load-bearing.

**Detection today: partial.** `cargo tree -p pdfce-core` under rule 2
would show the dependency **if someone ran it and knew to object.** The
rule as written names *"a GUI/windowing dependency"*, and `wasmtime` is
neither. §4.5 closes that gap for one paragraph.

### 5.4 ★ NEW — letting the `DirtySet` road become the normal way to mutate

**How it would arrive:** invisibly, as the path of least resistance
(§4.1). Someone writing a batch tool, a CLI subcommand, or a
proof-of-concept plugin finds `EditSession` doesn't expose what they
need, finds `DirtySet` does, and ships. Nobody decided anything.

**What it costs:** the second such site establishes a precedent; the
third establishes a pattern. Each one is an edit with **no undo, no
disclosure channel (R145), and no save-mode obligation (§4.3)**. The
retrofit cost is per-site and grows.

**Detection today: none, and this is the only one of the four where a
detector is both cheap and clearly worth building.** §6.

---

## 6. ★ THE GUARDRAIL RECOMMENDATION — one rule, one ~40-line gate, aimed at §5.4

**Requested: the cheapest concrete guardrail that makes the properties
enforced rather than incidental. Requested honestly, including "not
yet."**

### 6.1 What I do NOT recommend, and why — because declining is most of the answer

The operator values **error detection over convenience** and declined a
parallel-session setup the same day on exactly those grounds (decision
029). A recommendation that adds ceremony without adding detection fails
on his own stated criterion, so each rejected option is rejected **by
that test**.

| Rejected | Why |
|---|---|
| **A test that undo is replay-based** | **Cannot be written honestly.** §3.1 established that a *correct* type-dispatch undo passes every existing and every conceivable behavioural test — the two implementations are observationally identical. A test asserting an implementation shape is a test asserting source text, which is a grep wearing a test's clothes. |
| **A grep gate on `fn undo` for `match`** | Writable (~20 lines), and it would fire on §5.1. But it is **evadable by the refactor that matters most** — §5.2 changes `Command`, and undo's reversal may then be a `for` loop over typed payloads with no `match` token in sight. A gate that catches the careless case and misses the thoughtful one **trains people to trust it**, which is worse than not having it. `check-ui-strings.sh`'s own header records this project learning that lesson: *"a gate that cannot pass guards nothing, and it trains everyone who sees it red to ignore it."* |
| **`cargo check --target wasm32-unknown-unknown` in CI** | Would genuinely enforce property 3, and would be the right call **the day the web fork becomes real work.** Today it would fail immediately on §4.7's four `std::fs` sites, so adopting it means doing the feature-gating work first — **an hour of guardrail turns into a day of refactor for an unscheduled fork.** Revisit when the fork is scoped, not now. |
| **Any `[features]` scaffolding** | §7. |
| **Building any part of a plugin API "so it's ready"** | Directly contrary to the operator's instruction. A speculative API is worse than none: it must be maintained, it constrains the real design, and it is written without the one thing that would make it correct — a first plugin. |

### 6.2 What I DO recommend — a standing rule plus a grep gate, both aimed at §4.1 / §5.4

**Two items. Together well under an hour.**

**(a) A standing rule — number to be minted by `pdfce-librarian`, not
assigned here.** Proposed text:

> **A mutation path that bypasses `EditSession` is a NAMED exception with
> a stated reason.** `EditSession` is the road that carries undo
> (`edit.rs:1304`), disclosures (`PlannedEdit::disclosures`, R145) and
> the minimal-diff contract (R46). `DirtySet` + `writer::save_*` is a
> second, fully public road that carries none of them. **Redaction is the
> only sanctioned traveller** (`redact.rs:1036-1217`, R35), because its
> contract is removal and removal requires a save mode rather than a
> write set. Any new site that constructs a `DirtySet` outside
> `edit.rs`, `redact.rs`, `writer/` and tests is a **decision**, recorded
> as one.

**(b) `tools/check-bypass-paths.sh` — the detector, in the idiom this
project already uses.** `check-ui-strings.sh` is 202 lines and
`check-ledger-numbers.py` is 326; this one is smaller than both, roughly
**40 lines including its explanatory header** (which, per rule 6 and
`check-ui-strings.sh`'s own precedent, is most of the file).

- **Greps for** `DirtySet::empty(`, `DirtySet::identity_reemission(`,
  `.set_staging(`, `writer::save_full(`, `writer::save_incremental(`.
- **Allowlists** `crates/pdfce-core/src/edit.rs`,
  `crates/pdfce-core/src/redact.rs`, `crates/pdfce-core/src/writer/`,
  `tools/content-identity/`, and everything from `#[cfg(test)]` to end of
  file — the same truncation `check-ui-strings.sh` uses, with the same
  documented limitation that code placed after the test module is
  invisible to it.
- **Verified by planting a violation and watching it fail**, not only by
  watching it pass — this project's stated gate-verification discipline,
  learned the hard way when a checker that "passed" was found to be
  measuring the wrong thing.

**Why this one and not the others.**

1. **It detects a thing that is happening now**, not a hypothetical. The
   road is open today (§4.1) and the first unsanctioned traveller has
   not arrived yet — which is exactly when a tripwire is cheap.
2. **It cannot be green-and-wrong.** It fires on a construction that
   either is or is not there. There is no thoughtful refactor that
   evades it, because bypassing `EditSession` *means* touching one of
   these five symbols.
3. **It is detection, not ceremony.** It adds zero steps to writing a
   feature the normal way. It fires only when someone leaves the road.
4. **It pays off even if no plugin system is ever built**, which is the
   test that matters most for a speculative guardrail. Every bypass it
   catches is an edit that would have had no undo and no disclosure —
   a defect on its own terms, plugins or not.

### 6.3 Honest scoring of what remains unguarded

**§5.1 and §5.2 stay undetected**, and I am not going to pretend
otherwise. The only defence they have after this record is **this
record**, plus the doc comment at `edit.rs:1298-1303`.

**I judge that acceptable, and the reasoning is worth recording** so a
future session can disagree with it deliberately: the refactors in §5.1
and §5.2 are **large, deliberate, reviewed changes** to the single most
central type in the crate. They are not the kind of thing that lands
unnoticed in a Friday commit. A named record they would have to be read
against is a proportionate defence. §5.4, by contrast, lands as **one
line in an unrelated file**, which is precisely why it gets the gate.

**One free strengthener, recommended alongside:** add a sentence to
`edit.rs`'s `undo` doc comment — *"the undo path deliberately does not
inspect `CommandKind`; see decision 030 §3.1"* — so that §5.1's rewrite
has to **delete a cross-reference to this record** rather than merely a
description of behaviour. Cost: one line. It converts a silent removal
into a visible one.

---

## 7. ★ POSITION ON CARGO FEATURES — not now, and the reason is not "premature"

Requested: is introducing `[features]` now, **independent of plugins**, a
way to make module boundaries real rather than conventional?

**Position: NO, not now.** But the usual reason ("premature") is the weak
one; there are two stronger reasons and one narrow concession.

### 7.1 The observation is correct

**Confirmed: there is no `[features]` table in any of the four crate
manifests.** So the §2.1 module map is **entirely conventional** — the
compiler enforces nothing about it, and an "optional bulky add-on" has
no mechanism today.

The corollary the request draws is also correct and worth keeping on the
record: **nobody has ever verified that `pdfce-core` COMPILES with
subsystems removed.** Hidden couplings are likely rather than possible in
a 32,569-line crate whose largest module is 16,434 lines. Discovering
them is cheap — `#[cfg(feature = "…")]` on a module and a `cargo check` —
whenever it is wanted.

### 7.2 Why not now — three reasons, in ascending order

1. **Features multiply the build matrix, and this project's gates are
   already its most expensive asset.** R46's corpus identity gate,
   R34/R59 re-runs, and `cargo test --workspace` (≈1,790 tests) are what
   catch regressions here. A meaningful feature split means each gate has
   a *combination* question attached, and a feature combination that is
   never built is a feature combination that is broken — the standard
   Cargo failure mode, and the one `--all-features` /
   `--no-default-features` CI matrices exist to paper over at real cost.

2. **★ Features would encode boundaries that are NOT where the request
   thinks they are — and encoding a wrong boundary is worse than
   encoding none.** §2.2 measured the actual shape: the modules are
   per-feature-group, but **`edit.rs` is a single 8,880-line convergence
   point that every one of them passes through.** A `vector` feature
   would have to `#[cfg]` out a set of `EditSession` methods,
   `CommandKind` variants and error variants **interleaved with every
   other feature's** inside one file. What you would get is not a clean
   module boundary made real; it is `#[cfg]` confetti through the
   crate's most-edited file, with a combinatorial correctness question
   attached to a type whose whole job is to have exactly one shape.
   **The boundary the manifest would claim and the boundary the code has
   are different boundaries.**

3. **It fails the operator's own criterion.** Features add build states
   in which errors *can* occur but are not *observed* — the
   never-built-combination problem. That is a net **reduction** in error
   detection, sold as rigour. It is the same trade he declined for
   parallel sessions, in a different costume.

### 7.3 The narrow concession — a one-off experiment, not a manifest change

There is a real question buried in the request — *would core even
compile with a subsystem removed?* — and it deserves an answer
eventually, because its answer is **information about coupling** that no
other check produces.

**Get it without shipping a feature.** A throwaway branch, one
`#[cfg(feature = "x")]` on the largest module, `cargo check`, read the
error list, **throw the branch away and write down the count.** That is
an afternoon, produces the fact, commits to nothing, and adds no
permanent build state.

**Recommended timing: when the web/WASM fork is scoped** — because that
Pass has to answer the same coupling question anyway (§4.7's four
`std::fs` sites are the first instance), and one investigation would
serve both. **Doing it now would be work with no consumer.**

---

## 8. Licence — no implication today; one named implication later

pdfce is **MIT**, and GPL/AGPL cannot be linked (`CLAUDE.md` rule 8,
`LEGAL.md` §6.1). **This record adds no dependency and creates no licence
question.** The conditional one is named here so it is not met for the
first time mid-implementation:

- **A plugin ABI is a linking decision as well as a technical one.** An
  **in-process dynamic-library** host makes third-party plugin code part
  of the same process image, which is the fact-pattern where "is this a
  derivative work?" is argued. A **WASM sandbox** or a **separate
  process** keeps the boundary crisp — the guest is data the host
  interprets. §3.3's WASM-portability property already pushes toward the
  sandbox for **entirely unrelated technical reasons**, and the licence
  angle happens to agree. That agreement is worth noting precisely
  because it means the decision is not a trade-off.
- **The candidate hosts are permissive**, so the *host* side raises
  nothing: `wasmtime` (Apache-2.0-with-LLVM-exception), `wasmer` (MIT),
  `libloading` (ISC). Each would still go through rule 13's
  classification and a `cargo-about` regeneration of
  `THIRD_PARTY_LICENSES.md` at adoption time.
- **A plugin API is also a distribution-posture question** (decision
  003), because an extension point invites third-party code the project
  does not control. Out of scope here; named so it is not forgotten.

---

## 9. What this record does NOT do, and what it owes

**Does not:**

- **Authorise any code change.** §6's guardrail is a recommendation for
  the operator to accept or decline. Nothing in this file is a task.
- **Design a plugin system**, or take any position on what a pdfce plugin
  would be.
- **Recommend closing `DirtySet`.** §4.1 recommends making its use
  **visible**, not blocked; it is public for reasons that are still good.
- **Claim the three properties are sufficient.** §3.2 downgrades one of
  them and §4 adds a fourth risk they do not cover.
- **Write to `ROADMAP.md`, `SESSION_LOG.md` or `ARCHITECTURE.md`.** A
  librarian held those files concurrently with this filing.

**Owes — all to `pdfce-librarian`:**

1. **An `ARCHITECTURE.md` §12 dated ledger entry for decision 030**,
   cross-referencing this file per `docs/decisions/README.md`. **Not
   filed by this record.** Until it exists, 030 is visible to
   `tools/check-ledger-numbers.py` (which derives the ceiling from
   `docs/decisions/` only) but absent from the canonical index.
2. **A correction to R148's superseding note in `ROADMAP.md`**: decision
   029 reserved **030** for a possible Pass 33.0 record; 030 is spent
   here, so that record takes **031**.
3. **A standing-rule number** for §6.2(a), if the operator accepts it.
   Rule numbering is the librarian's ledger, not the consultant's — the
   text is proposed, the number is not.
4. **A standing-rule number** for §4.5's plugin analogue to rule 2, if
   accepted. This one is recommended **whether or not §6.2 is taken**:
   it costs a paragraph and it is the guardrail with the worst
   later-cost-to-now-cost ratio of anything in this record.
5. **Re-run `python tools/check-ledger-numbers.py --stats` in the shared
   checkout** after this file and the concurrent librarian edits have
   both landed. The readings available at write time came from a
   worktree at `7c45bf8`, which predates decision 029 and therefore
   reports a **spurious gap at 029** once this file exists — an artefact
   of the worktree being one commit behind, **not** a numbering error.

---

## 10. References

**ISO 32000-1 (PDF 1.7):**

- **§7.3.1** — the eight basic object types plus the indirect reference;
  the closed type set that makes `Object`'s `#[non_exhaustive]`
  correct-as-is (§4.4).
- **§7.3.8** — stream objects; the dictionary-plus-data framing behind
  `Stream`'s `ByteSpan` model (§3.2(3)).
- **§7.5.4** — object and generation number ranges, which `ObjId`'s
  `u32`/`u16` fields are sized to (§4.4).
- **§7.5.6** — incremental updates; pdfce's default save mode (§4.3).
- **§12.8.1 NOTE 1** — a signature's `/ByteRange` digest survives an
  incremental update and does not survive a full rewrite. The whole of
  §4.3.
- **§14.5 Table 319, and NOTE 1** — `/PieceInfo` private application
  data, **and its explicit "may be discarded" caveat** (§4.6).

**In-tree, at `7c45bf8`:**

- `crates/pdfce-core/src/edit.rs` — `:150-160` (bounded history is safe
  because the dirty set is a diff), `:220` (`CommandKind`,
  `#[non_exhaustive]`), `:486` / `:510` / `:518` (the three **private**
  command types), `:1288` / `:1294` (`undo_kind` / `redo_kind` — the
  label's only consumer), `:1304-1335` (**data-driven undo/redo**),
  `:2506` (`vector_surgery`), `:2690` (`commit`), `:2719`
  (`write_state`), `:5401` (`stage_bytes`), `:7468` / `:7485` (the undo
  tests that do *not* guard the property).
- `crates/pdfce-core/src/writer/mod.rs` — `:214`–`:464`, the **entire
  public `DirtySet` surface** of §4.1.
- `crates/pdfce-core/src/writer/save.rs` — `:257`, `:499`.
- `crates/pdfce-core/src/document.rs` — `:252` / `:746` / `:777` (three
  of four `std::fs` sites), `:715-747` (incremental save's
  superseded-content warning), `:749-780` (full rewrite's
  signature-invalidation warning), `:781`+ (`next_object_number`).
- `crates/pdfce-core/src/lib.rs:262` — the fourth `std::fs` site.
- `crates/pdfce-core/src/object.rs` — `:62` (`ObjId`), `:133-143`
  (`Dict`, ordered `Vec`, minimal-diff rationale), `:236-260`
  (`Stream` / `Object`).
- `crates/pdfce-core/src/redact.rs` — `:6` / `:48` / `:1057` (the one
  deliberately destructive operation), `:1036-1217` (the
  `EditSession`-bypassing apply path), `:1215` (forced `save_full`),
  `:1227` (a second local `stage()`).
- `crates/pdfce-core/src/dimension/sidecar.rs` — `:1-40`, the
  `/PieceInfo` precedent **and** its survival hedge (§4.6).
- `crates/pdfce-core/Cargo.toml` — the `flate2` `rust_backend` pin and
  three further `default-features = false` codec comments; **no
  `[features]` table**, here or in any sibling manifest.
- The four existing trait seams, all internal abstraction rather than
  public extension points today: `ObjectGraph` (`graph.rs:79`),
  `XObjectResolver` (`vector/decompose.rs:837`), `FontResolver`
  (`vector/decompose.rs:1006`), `ObjectEncoder`
  (`writer/encoder.rs:71`).
- `tools/check-ui-strings.sh` (202 lines) and
  `tools/check-ledger-numbers.py` (326 lines) — the grep-gate idiom
  §6.2(b) would follow, and the source of its "a gate that cannot pass
  guards nothing" caution.

**Project documents:**

- `CLAUDE.md` rule **2** (GUI-core separation — §4.5's model), rule **3**
  (round-trip / minimal-diff), rule **6** (documentation-first), rule
  **8** (MIT; publishing still needs a go-ahead), rule **11** (CLI
  parity), rule **13** (dependency licence classification), rule **15**
  (ce vs pdf dimension terminology).
- `ARCHITECTURE.md` **§3** (crate layout), **§5** (save model), **§6**
  (single-folder portable packaging / static single binary), **§11.1**
  (bounded undo), **§12** (the dated ledger entry this record is owed).
- `ROADMAP.md` standing rules **R32/R34/R46** (byte-identical
  re-emission and the corpus identity gate), **R35** (redaction forces a
  full rewrite), **R36** (redact-a-signed-document is the operator's
  either/or), **R45** (the combined base-plus-staging coordinate
  system), **R48** (flatten discloses incremental-save recoverability),
  **R106** / **R133** (read the live ceiling before minting a number),
  **R145** (`Result<(), E>` drops operator-visible information),
  **R148** (the note needing the §9(2) correction).
- Decisions **001** §6.2 (the `flate2` pure-Rust pin), **002** R1/R4
  (structured core diagnostics, no strings in core), **003**
  (distribution posture — §8's third bullet), **007** W7 (redact-a-signed
  document), **011** §2.4 (the ce-dimension `/PieceInfo` sidecar),
  **027** §3 (`PlannedEdit::disclosures`, R145), **029** (the reserved
  030 this record spends, and §3's concurrent-writer collision argument
  that its own filing had to navigate).
