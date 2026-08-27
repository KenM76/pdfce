---
name: only-an-out-of-crate-test-feels-a-consumers-constraints
description: Unit tests inside the crate cannot see `#[non_exhaustive]`, private-field, or visibility walls — write one integration test per new public spec type or the consuming project finds the wall first
metadata:
  type: feedback
---

**A test inside the crate cannot exercise the constraints a consumer lives
under.** For any new `pub` type a shell is meant to construct, write at least
one test in `crates/<crate>/tests/` — not in a `mod tests` — before calling
the API done.

**Why:** 2026-08-26, `Pass 134.0`. I added `FieldEdit` and `WidgetEdit` as
`#[non_exhaustive]` partial-update structs, which is the right choice (they
will grow, and a struct literal in `pdfceGUI` would break every time one
does). In-crate code compiled fine. The moment I wrote the integration test —
which is out-of-crate by construction — every struct literal failed:
`#[non_exhaustive]` **blocks construction entirely from outside the defining
crate**, and `Default` does not rescue it, because `Foo { ..Default::default() }`
is still a struct expression.

So the API as first written was *unconstructible by its only consumer*, and
nothing inside the crate could have told me. The fix was builders
(`new()` + chainable `with_*`), which is the convention `RenderOptions` and
the five `New*` specs already follow — I simply had not been forced to notice.

**The general shape:** `#[non_exhaustive]`, private fields, `pub(crate)`
re-exports, sealed traits and `#[doc(hidden)]` are all invisible to a test
that lives beside the code. They are the whole experience of the person
downstream.

**How to apply:** when a Pass adds a public type a shell constructs, the
acceptance criteria include one out-of-crate test that *constructs it the way
the shell will*. In this project that is `crates/pdfce-core/tests/` for
`pdfce-core` and `crates/pdfce-cli/tests/` for the binary contract. It costs
one file and it is the only mechanism that makes the consumer's constraints
compile-checked rather than reported back weeks later through
`D:\Dev\FeatureRequests\`.

Related: [[project_gui_request_channel]] — the channel is where this failure
arrives if the test does not catch it first.
