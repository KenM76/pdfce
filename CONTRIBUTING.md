# Contributing to pdfce

Thanks for your interest. A few things to know before opening an issue
or PR — this project has some non-default conventions worth reading
first.

## Project status

Pre-release as of 2026-07-23 — no Cargo workspace exists yet (see
`docs/ROADMAP.md`, "Pass 0" is next). External contributions aren't
being actively solicited at this stage; if you've found this repo
early, feel free to open an issue with feedback, but larger PRs before
the core architecture (`docs/ARCHITECTURE.md`) has stabilized past
Pass 0/1 are likely to need significant rework.

## License — read before contributing code

**pdfce's license is not yet finalized** (see `docs/LEGAL.md` §1).
**Do not submit a PR expecting a specific license to apply until a
`LICENSE` file exists at the repo root.** Once it does: by submitting
a contribution, you agree it's licensed under the terms in `LICENSE`
at the time of merge (the standard "inbound = outbound" convention
most Rust-ecosystem projects use — no separate CLA). A
Developer-Certificate-of-Origin-style sign-off (`git commit -s`,
certifying you have the right to submit the contribution under the
project's license) may be required once the license is set; watch for
that requirement to be added here.

## The documentation is the logic

This project follows a documentation-first discipline: `docs/ARCHITECTURE.md`
is the authoritative design description, `docs/ROADMAP.md` is the
plan/history, `docs/LEGAL.md` covers licensing/IP posture, and
`docs/PRIOR_ART.md` records what existing open-source work informed
which decisions. **Read the relevant doc before proposing a change** —
if your PR contradicts something documented there, the doc needs to
change too (in the same PR), not just the code.

## Two invariants that are not up for casual debate

If a contribution would violate either of these, expect it to need a
strong justification and explicit maintainer sign-off, not just review
comments:

1. **GUI-core separation** (`docs/ARCHITECTURE.md` §3) — `pdfce-core`
   and `pdfce-render` must never gain a GUI/windowing dependency.
2. **Round-trip / minimal-diff editing** (`docs/ARCHITECTURE.md` §5) —
   objects the user didn't touch must be re-emitted byte-identical or
   omitted from an incremental save, redaction aside.

## Code style

`cargo fmt` and `cargo clippy -- -D warnings` clean, no exceptions —
enforced by CI (`.github/workflows/ci.yml`) once the workspace exists.
Public API design follows the Rust API Guidelines — see
`docs/ARCHITECTURE.md` §8 for specifics.

## Security-relevant contributions

If your contribution touches parsing, filters, or anything that
handles untrusted PDF input, read `docs/ARCHITECTURE.md` §10
(adversarial input hardening) first — resource-limit guards and
fuzz-testing are requirements, not nice-to-haves, for this kind of
code. See `SECURITY.md` for how to report a vulnerability privately
instead of via a public issue.

## Test fixtures

Never commit a real-world PDF of unknown provenance as a test fixture
— see `docs/LEGAL.md` §5 and `fixtures/README.md` for what's actually
allowed (synthetic files, or files from a corpus with clear
redistribution rights).
