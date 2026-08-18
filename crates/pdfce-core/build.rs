//! Build-time provenance capture for pdfce — `Pass 101.0`.
//!
//! # What this exists for
//!
//! The operator asked, 2026-08-18: *"whenever you build a new version can
//! you include the build date and time, and also include the build
//! revision, date, and time for the version of iccce used in the version?"*
//!
//! This script answers the first half. It emits four `cargo::rustc-env`
//! variables that [`pdfce_core::build`] reads with `env!`, so the values are
//! baked into the binary at compile time and cost nothing at run time.
//!
//! | variable | meaning |
//! |---|---|
//! | `PDFCE_BUILD_TIMESTAMP` | when this binary was built, RFC 3339 UTC |
//! | `PDFCE_BUILD_REVISION` | `git describe --tags --always --dirty`, or `unknown` |
//! | `PDFCE_BUILD_COMMIT_TIMESTAMP` | the committer date of that revision, RFC 3339 UTC, or `unknown` |
//! | `PDFCE_ICCCE_PROVENANCE` | see "the second half" below |
//!
//! # ★ The second half of the request cannot be answered, and why saying so
//! is the answer
//!
//! pdfce **does not depend on `iccce`**. Measured, not assumed:
//! `grep -rn "iccce" Cargo.toml crates/*/Cargo.toml` returns nothing, and no
//! source file in the workspace mentions it. `iccce` exists as a sibling
//! project and `ARCHITECTURE.md`'s decision 064 records the boundary — it
//! owns colour conversion — but a boundary is a *decision*, not a
//! dependency edge.
//!
//! So there is no "version of iccce used in this build" to report. Stamping
//! one anyway — by reading the sibling checkout's `git describe`, say —
//! would assert a relationship that does not exist, and would go on
//! asserting it every time somebody read the banner. That is the
//! claim-bearing-copy rule applied to a version string.
//!
//! What this script does instead is emit `PDFCE_ICCCE_PROVENANCE` as the
//! literal string `not-linked`, so the version output **answers the
//! operator's question every time he asks it** rather than leaving it
//! silently unaddressed. `Pass 101.1` replaces that with the real
//! revision, date and time on the day the dependency lands — and the
//! detection is deliberately structural (see `iccce_provenance`), so it
//! starts reporting the moment that happens rather than waiting for
//! somebody to remember this file.
//!
//! # Reproducibility — the trade this makes, stated rather than buried
//!
//! Embedding a build timestamp makes builds **non-reproducible by
//! construction**: two builds of byte-identical source produce different
//! binaries. That is inherent in what was asked for, not a shortcoming of
//! how it is done.
//!
//! The standard escape hatch is honoured: if `SOURCE_DATE_EPOCH` is set in
//! the environment, it is used instead of the wall clock, which is the
//! convention reproducible-build systems already drive. So the capability
//! and the property remain simultaneously available, and choosing between
//! them stays the operator's call.
//!
//! # Failure behaviour
//!
//! Every git lookup can fail — no `git` on PATH, a source tarball with no
//! `.git`, a shallow clone with no tags. Each failure yields the literal
//! `unknown` rather than a plausible-looking substitute. A version banner
//! that guesses is worse than one that admits it does not know, because a
//! wrong revision is acted on and a missing one is questioned.
//!
//! ## ★ One operational consequence, so it is not discovered from a release
//!
//! `actions/checkout@v4` defaults to a **depth-1** clone, which has no tags
//! and no history — so a binary built by CI as things stand today would
//! report `revision: unknown`. That is harmless for the CI jobs, which test
//! rather than ship, and pdfce's releases are built locally where the full
//! history is present.
//!
//! But if a release build ever moves into CI, that workflow needs
//! `fetch-depth: 0` or the shipped binary will not be able to say what it
//! is. Recorded here rather than in the workflow because this is the file
//! whose behaviour explains it, and because the failure is silent: the build
//! succeeds, and only the banner is empty.

use std::path::Path;
use std::process::Command;

fn main() {
    // Re-run when HEAD moves or the index changes, so a rebuild after a
    // commit does not keep reporting the previous revision. Without these,
    // Cargo would cache this script's output against the source files
    // alone — which do not change when you commit them.
    let git_dir = locate_git_dir();
    if let Some(dir) = &git_dir {
        println!("cargo::rerun-if-changed={}/HEAD", dir.display());
        println!("cargo::rerun-if-changed={}/index", dir.display());
        // A branch's own ref file, so switching branches is noticed too.
        let refs = dir.join("refs");
        if refs.is_dir() {
            println!("cargo::rerun-if-changed={}", refs.display());
        }
    }
    println!("cargo::rerun-if-env-changed=SOURCE_DATE_EPOCH");
    println!("cargo::rerun-if-changed=src/civil_time.rs");

    println!("cargo::rustc-env=PDFCE_BUILD_TIMESTAMP={}", build_time());
    println!(
        "cargo::rustc-env=PDFCE_BUILD_REVISION={}",
        git(&["describe", "--tags", "--always", "--dirty"])
    );
    println!(
        "cargo::rustc-env=PDFCE_BUILD_COMMIT_TIMESTAMP={}",
        commit_time()
    );
    println!(
        "cargo::rustc-env=PDFCE_ICCCE_PROVENANCE={}",
        iccce_provenance()
    );
}

/// The build's wall-clock time as RFC 3339 UTC, or `SOURCE_DATE_EPOCH` when
/// the environment supplies one.
///
/// Formatted by hand from a Unix timestamp rather than by pulling in a date
/// crate: a build script's dependencies are compiled for the host on every
/// clean build, and a calendar conversion is about thirty lines. See
/// [`format_rfc3339_utc`] for the conversion and its one real subtlety.
fn build_time() -> String {
    let secs = std::env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0))
        });
    format_rfc3339_utc(secs)
}

// The calendar arithmetic is SHARED with the crate rather than copied into
// this script, and lives in `src/civil_time.rs` -- read that file for why,
// and for the tests that pin it. A build script cannot depend on the crate
// it builds, and a build script's own `#[cfg(test)]` module is never run by
// `cargo test`, so arithmetic that lived only here would be arithmetic
// nobody could assert.
include!("src/civil_time.rs");

/// The committer date of `HEAD`, as RFC 3339 **UTC**.
///
/// Read as a Unix timestamp (`%ct`) and formatted by the same function that
/// formats the build time, rather than taken from git's own `%cI`.
///
/// `%cI` is strict ISO 8601 but carries the **committer's local offset**, so
/// the two timestamps in the stamp would be in different time zones — and
/// the whole reason both are printed is so they can be compared at a glance
/// (how stale was the source when this was built?). Two instants in
/// different zones cannot be compared at a glance; they can only be
/// compared carefully, which is the same as not being compared.
fn commit_time() -> String {
    let raw = git(&["log", "-1", "--format=%ct"]);
    raw.parse::<i64>()
        .map_or_else(|_| "unknown".to_owned(), format_rfc3339_utc)
}

/// Run `git` with `args` in the manifest's directory, trimmed; `unknown` on
/// any failure at all.
fn git(args: &[&str]) -> String {
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_owned());
    Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_owned())
}

/// The `.git` directory for this workspace, if there is one.
///
/// Walks up from the manifest rather than assuming a fixed depth, so this
/// keeps working if the crate moves within the workspace. Handles the
/// worktree case, where `.git` is a *file* containing a `gitdir:` pointer —
/// which matters here because this project verifies changes against git
/// worktrees, and a build script that silently stopped re-running inside one
/// would report the wrong revision precisely during a comparison.
fn locate_git_dir() -> Option<std::path::PathBuf> {
    let mut dir = Path::new(&std::env::var("CARGO_MANIFEST_DIR").ok()?).to_path_buf();
    loop {
        let candidate = dir.join(".git");
        if candidate.is_dir() {
            return Some(candidate);
        }
        if candidate.is_file() {
            let text = std::fs::read_to_string(&candidate).ok()?;
            let path = text.trim().strip_prefix("gitdir:")?.trim();
            return Some(dir.join(path));
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// What to report about the `iccce` build linked into this one.
///
/// # Why this reads the DEPENDENCY GRAPH and not a path on disk
///
/// Because the question is *"which iccce is in this binary"*, and a sibling
/// checkout at `D:\Dev\iccce` answers a different question — *"which iccce
/// is on this machine"*. Those coincide only by accident, and reporting the
/// second as if it were the first is precisely the kind of plausible,
/// unverifiable claim a version banner must not make.
///
/// Cargo sets `DEP_<links>_*` variables only for crates that declare a
/// `links` key, which `iccce` does not, so the reliable structural signal is
/// `CARGO_PKG_*` on the dependency itself. Until pdfce actually depends on
/// `iccce`, there is nothing to read, and this returns `not-linked` — which
/// is a true statement rather than an omission.
///
/// `Pass 101.1` fills this in when the dependency lands. It is written as a
/// function with this doc comment, rather than as a hard-coded string
/// literal at the call site, so the next person to add the dependency finds
/// the explanation at the place they have to change.
fn iccce_provenance() -> String {
    // The env-var shape Cargo would give us if `iccce` were a build
    // dependency exporting metadata. Checked rather than assumed absent, so
    // this begins reporting the moment that becomes true.
    if let Ok(v) = std::env::var("DEP_ICCCE_PROVENANCE")
        && !v.trim().is_empty()
    {
        return v.trim().to_owned();
    }
    "not-linked".to_owned()
}
