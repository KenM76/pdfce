# 003 — Distribution posture: cross-platform scope and update mechanism

- **Status:** Accepted
- **Date:** 2026-07-30
- **Deciders:** KenAgent (autonomous-builder), on request of `pdfce-engineer`
- **Supersedes:** the final two bullets under `docs/ROADMAP.md`'s
  "Product-scope decisions — deliberately deferred" (2026-07-23):
  "Cross-platform scope beyond 'Windows first'" and
  "Update/release mechanism". With decision 002 having removed the i18n
  bullet earlier the same day, **that list is now empty.**
- **Scope:** which platforms pdfce *ships* and which it *compiles for in
  CI*; how a user learns about and installs a new version. **Does not**
  decide the OSS license, the release schedule, whether/when a public
  repository is created, or whether to buy code signing — see §7.
- **Dependencies added:** none. Two CI jobs added, both grep-based, both
  fail-closed.

---

## 1. Context

`docs/ROADMAP.md` carried these two as explicitly-flagged deferrals
rather than oversights:

> **Cross-platform scope beyond "Windows first."** `ARCHITECTURE.md` §6
> says Windows is the first packaging target — confirm with the user
> whether that's a deliberate v1 scope decision or just how the project
> happened to start (egui/eframe supports macOS/Linux natively, so it's
> not a technical blocker either way, just a testing/packaging-effort
> scope question).
>
> **Update/release mechanism.** No installer means no auto-updater by
> default — is "download and replace the folder" the permanent answer,
> or does pdfce want an opt-in update checker later? Ties directly to
> `ARCHITECTURE.md` §1.1's privacy posture (any update mechanism must be
> opt-in, never silent phone-home).

They are one decision, not two, because both are answers to the same
question: **what does pdfce hand a stranger, and how does that stranger
keep it current?** Deciding them separately produces incoherence — a
project that cross-builds for three platforms but can only tell Windows
users about updates, or one that ships a privacy-preserving update
checker for an artifact nobody can verify the integrity of.

### 1.1 The framing fact everyone should read first

**pdfce has no git remote. CI has never run once.**

`.github/workflows/ci.yml` says so in its own header comment, and it is
still true today. Every job in that file — `fmt`, `clippy`, `test`
(ubuntu + windows matrix), `gui-core-separation`, `ui-strings`,
`third-party-licenses` — is a *declaration of intent* that has produced
exactly zero signal. The sub-question "do CI builds for linux/macos run
from now?" therefore has a precondition nobody has met: **no CI runs at
all from now, until a remote exists.**

That precondition is itself gated. `LEGAL.md` §1 forbids a *public*
repository until Ken picks a license. It does **not** forbid a private
one — publication and version control are different acts — so a private
GitHub remote is available today and is the only way to get any CI value
at all. But a private repository is where GitHub Actions usage is
metered. Current standard-runner list rates are **Linux 2-core
$0.006/min, Windows 2-core $0.010/min, macOS 3/4-core $0.062/min** —
macOS is roughly **10× Linux** and Windows only about **1.67×**. The
included allowance is 2,000 minutes/month on GitHub Free and 3,000 on
Pro. Public repositories remain **free and unmetered on standard
runners** (larger runners are never free, even for public repos).

Two notes on that pricing, recorded because they are easy to get wrong.
First, **"minute multipliers" is stale terminology** — GitHub's old
`actions-minute-multipliers` page now redirects to "Actions runner
pricing," which publishes per-minute list rates and no multiplier table;
the legacy 1×/2×/10× figures repeated by third-party trackers are
obsolete, and December 2025/January 2026 pricing changes cut rates by up
to 39%. Second, the **exact formula by which included minutes are
debited per OS is not stated in current documentation** — the ratios
above are implied by list price, not published as a quota-drain rule.

So the cost half of the cross-platform CI question is real *today* and
evaporates the moment the license is resolved and the repo goes public.

This is the second time in one day that `LEGAL.md` §1 has turned out to
gate something that looked unrelated to licensing (decision 001 §6.2
found the same for copyleft prior art). It is worth saying plainly: the
license decision is no longer an abstract preference. It gates CI, both
distribution channels, and the runner economics.

### 1.2 The constraints this decision has to serve

| # | Constraint | Source |
|---|---|---|
| D1 | **No network calls of any kind by default.** No telemetry, no analytics, no crash reporting, no update-check phone-home. Any future network feature is opt-in, off by default, disclosed plainly. | `ARCHITECTURE.md` §1.1 |
| D2 | **Single-folder portable. No installer, no registry writes, no system-wide runtime dependency.** Verified by a real smoke test: zip, unzip elsewhere, launch, render. | `ARCHITECTURE.md` §6 |
| D3 | **GUI-core separation.** `pdfce-core` / `pdfce-render` carry no GUI or windowing dependency, verified by `cargo tree`, not assumed. | `ARCHITECTURE.md` §3 |
| D4 | **The WASM/web fork stays a shell-crate swap**, and is a design constraint on today's code. | `ARCHITECTURE.md` §1, §3 |
| D5 | **Permissive licenses only; every dependency license-checked before it is added; attribution generated by `cargo-about`, never hand-maintained.** | `LEGAL.md` §6 |
| D6 | **pdfce's own license is undecided; no public repo, no published release, no "open source" in user-facing copy until it is.** | `LEGAL.md` §1, `CLAUDE.md` rule 8 |
| D7 | **Claim-bearing copy is verified, never plausibly defaulted.** Support promises, privacy promises, and refund/SLA-shaped statements are claims. | global `CLAUDE.md` |
| D8 | **Sole operator on Windows 11.** No Mac hardware, no Linux desktop, no second pair of hands. | project reality |
| D9 | **Non-monetized.** Every recurring dollar cost is a real objection, not a rounding error. | `ARCHITECTURE.md` §1 |

---

## 2. Options considered

### Decision A — cross-platform scope

- **A1 — Windows-only, and say so.** Ship one artifact. Other platforms
  are not claimed, not tested, not built.
- **A2 — Windows-only shipped, all platforms compiled in CI.** Ship one
  artifact; keep the code provably portable via continuous compile
  signal on the others.
- **A3 — Tri-platform v1.** Ship Windows, Linux, and macOS artifacts
  from the first release.
- **A4 — Windows + Linux v1, macOS later.** Split the difference on the
  theory that Linux is the cheap second platform.

### Decision B — update mechanism

- **B1 — Manual replace-the-folder, permanently. Nothing else.**
- **B2 — Manual, plus external package-manager channels** (Scoop,
  WinGet, Chocolatey) that handle upgrade for users who want it.
- **B3 — Manual, plus an opt-in, user-initiated in-app update
  *checker*** (display-only; never downloads, never installs).
- **B4 — Opt-in background checker** — off by default, but once enabled
  polls on a schedule or at startup.
- **B5 — Auto-updater** (self-replacing binary).

---

## 3. Evidence

§3.1–§3.2, §3.4, §3.5 and §3.6 were measured first-hand against pdfce's
own workspace and pinned `Cargo.lock` on 2026-07-30 — not asserted from
training data. Commands and exact outputs are given so any future
session can re-run them. §3.3's external-platform facts were verified
against primary sources the same day; the three items that could **not**
be confirmed are listed in §11 rather than smoothed over.

### 3.1 The cross-platform compile cost is already zero, and this is the finding that decides the CI question

All three of the following were run on Ken's Windows 11 box, against
the pinned 1.97.1 toolchain, with **no CI runner, no Apple SDK, no
Linux machine, and no system packages installed**:

```
rustup target add x86_64-unknown-linux-gnu wasm32-unknown-unknown aarch64-apple-darwin

cargo check --workspace --target x86_64-unknown-linux-gnu
    Finished `dev` profile ... in 1m 12s          ← exit 0

cargo check --workspace --target aarch64-apple-darwin
    Finished `dev` profile ... in 32.28s          ← exit 0

cargo check -p pdfce-core -p pdfce-render --target wasm32-unknown-unknown
    Finished `dev` profile ... in 6.46s           ← exit 0
```

The Linux run compiled the entire `accesskit_unix` / `atspi` / `zbus`
async stack and all of `eframe`/`egui-winit`/`glutin`/`winit`. The
macOS run compiled `accesskit_macos`, the `objc2` family, and
`core-graphics`. Neither needed a platform SDK, because `cargo check`
type-checks and does not link.

**Why this is decisive.** The question posed was "do CI builds for
linux/macos run from now (cheap early-warning) even if untested?" The
premise — that early warning requires a runner — is false. A
`cargo check --target <triple>` step on the *existing* ubuntu runner
delivers the same compile-level signal for macOS at Linux cost and 32
seconds, and Ken can run the identical command locally before pushing.
A macOS runner buys only what `check` cannot see: linking, test
execution, and GUI behavior — none of which Ken could debug anyway
(D8).

**The honest limit, stated so nobody over-trusts it.** `cargo check`
does not link, does not load a shared library, does not run a test, and
does not open a window. It catches type errors, missing `cfg` arms,
API drift, and platform-conditional compilation mistakes. It does not
catch link errors, missing system libraries, or any runtime behavior.
It is an early-warning system, not a portability guarantee, and the CI
job comment must say so.

### 3.2 What each platform actually costs, measured

**Dependency surface** — unique crates in `pdfce-gui`'s tree per target
(`cargo tree -p pdfce-gui --target <triple> --prefix none --no-dedupe`,
sorted unique):

| Target | Crates | Delta vs Windows |
|---|---|---|
| `x86_64-pc-windows-msvc` | **147** | — |
| `aarch64-apple-darwin` | **155** | +8 net (`objc2`/`core-graphics`/`accesskit_macos`, minus the `windows-sys` family) |
| `x86_64-unknown-linux-gnu` | **237** | **+90** — the entire `accesskit_unix` → `atspi` → `zbus` D-Bus accessibility bridge and its `async-io`/`futures-lite`/`blocking` executor stack |

Linux is the *heaviest* target, not the cheap one — which inverts the
usual intuition behind option A4. macOS is nearly free in dependency
terms; its cost is entirely in the trust chain (§3.3).

**License risk of expanding: none.** Re-running the `cargo-about` audit
with `targets = ["x86_64-pc-windows-msvc", "x86_64-unknown-linux-gnu",
"aarch64-apple-darwin"]` against the *current* permissive-only
`accepted` list in `about.toml` exits **0**:

| SPDX id | Crates |
|---|---|
| Apache-2.0 | 215 |
| MIT | 67 |
| Unicode-3.0 | 19 |
| BSL-1.0 | 2 |
| 0BSD / ISC / Zlib | 1 each |
| OFL-1.1 / Ubuntu-font-1.0 | 1 each (bundled fonts) |

**Zero copyleft across all three platforms.** The `LEGAL.md` §6 audit
does not become harder if pdfce ever ships cross-platform. That risk is
retired; it is not a reason to stay Windows-only.

**Linux needs no build-time system packages.** Every Linux windowing
binding in the tree is `dlopen`-based:
`wayland-sys` has its **`dlopen` feature enabled** (confirmed via
`cargo tree -e features -i wayland-sys`), and `x11-dl` / `xkbcommon-dl`
are dynamic-loader shims by construction. `pkg-config` and `cc` appear
only as *build*-dependencies of `wayland-sys`/`wayland-backend`, whose
C paths the `dlopen` feature bypasses. A bare `ubuntu-latest` runner
should therefore compile — and, expected but **not verified here**,
link — `pdfce-gui` with no `apt-get` step. If the first real CI run
disagrees, that is a finding for `D:\dev\rag\rust\`, not a reason to
drop the job. (This same `dlopen` dependence is what makes musl-static
impossible for the GUI — see §3.3.)

### 3.3 What actually makes a platform expensive: the trust chain, not the build

This is the part the ROADMAP's framing ("just a testing/packaging-effort
scope question") understates, and it is why the answer is A2 rather
than A3.

- **Windows.** An unsigned executable triggers a SmartScreen block —
  "Windows protected your PC" — and the user must choose *Run anyway*;
  enterprise policy can prevent continuation entirely. A self-signed
  binary is treated identically to an unsigned one. The critical
  operational fact: **unsigned files must build reputation anew with
  every update**, so the warning does *not* fade for an actively
  released project — it returns with each version. On Windows 11, Smart
  App Control may supersede SmartScreen and block unsigned files
  outright absent positive reputation. **EV certificates no longer
  bypass SmartScreen**, so the historical "buy EV to skip the warning"
  advice is dead. Traditional OV *and* EV code-signing certificates have
  required FIPS 140-2 Level 2 / Common Criteria EAL4+ hardware key
  storage since **2023-06-01** (CA/B Forum Code Signing Baseline
  Requirements, ballot CSC-17), which is the cost and friction that
  makes them unattractive here.

  **Recommendation: do not buy a traditional OV/EV certificate.**
  **Azure Artifact Signing** (renamed from Azure Trusted Signing in
  2026) is the option to price at first release: approximately
  **$10/month** per Microsoft's own documentation, **no hardware token**,
  integrates directly with GitHub Actions, identity validation required.
  Eligibility matters here and is favorable: Public Trust certificates
  are available to organizations across a list of countries, and
  **individual developers must be located in the United States or
  Canada** — **Ken is Canada-based and therefore geographically
  eligible**. Individual validation runs through government ID plus
  Microsoft Verified ID, the Azure subscription must be paid (no
  free/trial/sponsored tier), the certificate common name must be the
  validated legal name, and billing is not pro-rated. Because this is a
  **recurring cost against D9, it is an operator decision for Ken, not
  an engineering one** — the same class as `LEGAL.md` §1. Until it is
  taken, document SmartScreen's behavior honestly in README (§6.3),
  including that reputation does not carry across versions.

- **macOS.** An unsigned, un-notarized application downloaded from the
  internet is quarantined by Gatekeeper, and the historical escape hatch
  is gone: Apple stated for macOS Sequoia that "users will no longer be
  able to Control-click to override Gatekeeper when opening software
  that isn't signed correctly or notarized. They'll need to visit System
  Settings > Privacy & Security to review security information for
  software before allowing it to run." The current release (macOS 26
  Tahoe) documents the same multi-step flow: launch, dismiss, System
  Settings → Privacy & Security → Security → Open → Open Anyway → enter
  login password. In practice the *Open Anyway* button appears only for
  the most recently blocked app and within a short window, and
  quarantined unsigned bundles frequently surface under the misleading
  message "app is damaged and can't be opened."

  Notarization requires **Apple Developer Program** membership at
  **$99 USD per membership year** (notarization itself carries no
  additional fee) and a Mac to build and submit from — D9 and D8
  respectively, and Ken has neither. Additionally, "single folder you
  unzip and run" is not the macOS idiom; a `.app` bundle is, which is a
  packaging model D2's smoke test does not describe. **macOS is not a
  build problem. It is a hardware, money, and platform-idiom problem,
  and `cargo check` cannot touch any of those.**

- **Linux.** No signing gate, but a *substrate* gate, and the escape
  hatch that works on Windows has **no Linux analogue for the GUI**.

  **musl-static is a hard blocker, not a rough edge.** musl does not
  implement `dlopen` in statically linked programs at all — it is a stub
  that fails ("dynamic loading not supported"); per musl's maintainer,
  "Presently, it does not work at all. At best, it loses all the
  advantages of static linking." A static binary has no dynamic loader,
  and loading a glibc-linked `.so` into a musl process means two libcs
  in one address space. This lands directly on pdfce's stack: winit's
  own "Support for musl" issue records that "musl does not implement
  `dlopen`" and "wayland-sys tries to open `libwayland-client.so` which
  is not possible," and §3.2 established that *every* Linux windowing
  binding pdfce pulls is exactly that kind of `dlopen` —
  `libwayland-client`, `libX11`, `libxkbcommon`, plus `libEGL.so.1` /
  `libGL.so.1` via glutin/glow. Note also that
  `x86_64-unknown-linux-musl` defaults to `crt-static`, and disabling
  that to regain `dlopen` produces its own libgcc_s/loader breakage.

  The standard practice is therefore a **glibc dynamic** build:
  (1) compile against the oldest supported glibc, on an old distro or
  container, and never bundle the driver-coupled libraries — AppImage's
  excludelist is explicit that `libGL`/`libEGL` are "part of the video
  driver… known to cause issues if it's bundled," that bundling
  `libwayland-client` causes Mesa dependency problems, and that glibc
  components "should never be bundled"; (2) ship an **AppImage** (or
  Flatpak) for single-file distribution with those libraries left to the
  host; or (3) use **cargo-zigbuild** to pin a minimum glibc without an
  old build host (`--target x86_64-unknown-linux-gnu.2.17`), noting its
  own caveats — no warning on an invalid glibc version, imperfect match
  to host dynamic-link behavior, and `-C target-feature=+crt-static`
  does not work.

  **The exception worth noting: `pdfce-cli` has none of this problem.**
  It carries zero windowing dependencies by design (`ARCHITECTURE.md`
  §7), is pure Rust, and never `dlopen`s anything, so
  `x86_64-unknown-linux-musl` produces a genuinely portable static
  binary that runs on any Linux. If pdfce ever ships anything for Linux,
  **the CLI is the cheap, correct first artifact and the GUI is the
  expensive one** — the opposite of the intuitive order, and a direct
  dividend of the GUI-core separation invariant.

- **Reproducibility (D8), the constraint that outranks all of the
  above.** Ken cannot reproduce a macOS defect, cannot bisect it, cannot
  verify a fix, and cannot run the §6 smoke test. Shipping an artifact
  nobody can support is a claim (D7) the project cannot honor.

### 3.4 The network surface today, measured — including one thing nobody has noticed

Grepping `Cargo.lock` for every common HTTP/TLS/socket crate
(`ureq|reqwest|hyper|h2|rustls|native-tls|openssl|curl|isahc|attohttpc|surf|tokio|socket2|mio|hickory|webpki|webpki-roots|ring|aws-lc-rs|url`)
returns **exactly one hit: `url` 2.5.8** — a URL *parser*, not a client.

Tracing it produced the finding this section exists for:

```
cargo tree -p pdfce-gui --target x86_64-pc-windows-msvc -i url
url v2.5.8
└── webbrowser v1.2.1
    └── egui-winit v0.35.0
        └── eframe v0.35.0
            └── pdfce-gui
```

**`webbrowser` 1.2.1 is already in pdfce's shipping Windows binary, and
pdfce cannot remove it.** Reading `eframe` 0.35.0's manifest directly:

```toml
[target.'cfg(not(target_arch = "wasm32"))'.dependencies.egui-winit]
version = "0.35.0"
features = ["clipboard", "links"]
default-features = false
```

`links` is what enables `webbrowser` (`egui-winit`'s
`links = ["webbrowser"]`, where `webbrowser` is an optional dependency).
eframe hardcodes it, so pdfce-gui's own `default-features = false` does
not help. It cannot be feature-disabled without patching or forking
eframe.

**What this means, precisely.** `ARCHITECTURE.md` §1.1's "no network
calls of any kind by default" is **still true** — `webbrowser` opens
the OS default browser; it makes no request itself, and it is inert
unless pdfce emits an `OpenUrl` event, which pdfce never does today.
But the posture statement should say what is true with precision rather
than by omission (D7), because "we contain no code that can reach the
network" would be the wrong reading and someone will eventually make
it.

It also hands decision B a free capability: **egui can open a URL in the
user's browser at zero dependency cost**, which is the entire mechanism
needed for the middle rung in §5.4.

### 3.5 Two latent CI defects, found while evaluating this decision

Both will bite on the first real CI run (§9 T4), and both are cheap to
fix now:

1. **The `cargo tree` invariant jobs check the wrong target.**
   `gui-core-separation` runs on `ubuntu-latest` with no `--target`, so
   it evaluates the **Linux** dependency graph — 237 crates — while the
   shipped artifact is the **Windows** graph of 147. A Windows-only GUI
   dependency creeping into `pdfce-core` would pass CI. `cargo tree`
   accepts `--target` with no toolchain target installed (it is
   metadata-only; verified above), so fixing this costs one flag.
2. **The toolchain action and the toolchain file disagree.** Every job
   uses `dtolnay/rust-toolchain@stable` while `rust-toolchain.toml`
   pins `1.97.1`. rustup honors the file, so cargo silently downloads a
   *second* toolchain on first invocation. It self-heals (the file also
   declares `rustfmt`/`clippy`) but it is wasted minutes and a
   confusing failure mode if it ever does not.

### 3.6 The artifact is small — today

Current release build: `pdfce-cli.exe` 870,912 B, `pdfce-gui.exe`
7,622,656 B. Zipped with `THIRD_PARTY_LICENSES.md`: **3,633,770 B
(3.6 MB)**.

At that size, "re-download and replace the folder" is a genuinely
pleasant update experience — better than most installers. It will not
stay that size. `ROADMAP.md`'s Backlog contains OCR (language data
runs tens of MB per language) and a possible bundled CJK font
(decision 002 §3.2 priced full Noto Sans CJK at 15.7 MB). The
manual-update story degrades with the artifact, which is why §9 T5
exists rather than treating B1 as unconditionally permanent.

---

## 4. Decision

### 4.1 Decision A — **option A2**

**v1 ships Windows 10/11 x86_64 and nothing else. This is a deliberate
scope decision, not an accident of where the project started.** "v1"
means the first artifact ever published.

**The codebase stays platform-clean at all times**, verified
continuously by cross-target `cargo check` in CI (§6.2). pdfce is not
"a Windows program"; it is a portable program with one supported
build.

**CI**: keep Linux native on `ubuntu-latest` (it already is, and it
already carries `fmt`, `clippy`, both `cargo tree` invariants,
`ui-strings`, and the license audit — Linux is not an optional extra,
it is the project's primary CI host). Keep `windows-latest` in the
`test` matrix — it is the only place the shipped artifact is built,
linked, and run, and at $0.010/min against Linux's $0.006/min it is
barely more expensive. **Add no macOS runner.** Add one new ubuntu job
doing cross-target `cargo check` for `aarch64-apple-darwin` and
`wasm32-unknown-unknown`.

**macOS is gated on two conditions jointly** (§9 T1 **and** T2): the
repo is public *and* somebody can reproduce a macOS failure on real
hardware.

**If Linux is ever shipped**, `pdfce-cli` goes first as a static musl
binary and `pdfce-gui` follows separately as a glibc-dynamic build
(§3.3).

### 4.2 Decision B — **options B1 + B2, with B3 specified-but-deferred**

**Manual download-and-replace is the only update mechanism pdfce
implements. pdfce never self-updates.** (B5 and B4 are rejected
permanently and near-permanently respectively; see §5.5.)

**Update *discovery* is delegated outward to the distribution channel**
(B2): publish a **Scoop** manifest first and a **WinGet portable**
manifest second, once `LEGAL.md` §1 is resolved and a public repository
exists. These are metadata files in someone else's repository. They
cost pdfce zero code, zero binary size, zero dependencies, and — the
point — **zero network calls from pdfce's own process**, because the
request is made by a tool the user explicitly invoked.

**An in-app update checker (B3) is deferred, not forbidden**, behind
the complete specification in §6.4. Building one requires a new
decision record amending this one, because it requires an HTTP/TLS
client crate, which R12 forbids fail-closed.

**Available today at zero cost, and worth doing at first release:** a
Help-menu item that hands the releases-page URL to the OS default
browser, using the `webbrowser` crate eframe already links
unconditionally (§3.4), alongside a prominently displayed build
version. User-initiated, no request from pdfce's process, no new
dependency — roughly 80% of a checker's practical value for 0% of its
cost.

---

## 5. Rationale

### 5.1 "Windows first" was where the project started; making it a decision changes what it costs

The ROADMAP was right to ask. `ARCHITECTURE.md` §6 says "Windows first
target" in a parenthetical, which reads as a default rather than a
choice, and defaults quietly accumulate obligations — someone eventually
writes "cross-platform" in a README because the code obviously compiles
everywhere.

Making it an explicit decision converts an ambiguity into two useful
constraints. It licenses the project to **stop worrying** about macOS
packaging, notarization, `.app` bundles, AppImages, and glibc floors —
none of which are on any Pass — and it makes the *inverse* obligation
explicit: because the shipped scope is narrow, the **code** must stay
wide, and that is now enforced rather than assumed (R10, R11).

The closest prior art agrees. `PRIOR_ART.md` records **KillerPDF** —
the fastest-growing OSS competitor, 3,128 stars in three months — as
"single ~15.6 MB portable Windows EXE. **Windows-only.**" A single-
platform portable Windows build is a proven, sufficient posture for
this product category.

### 5.2 Why A2 and not A3 (tri-platform v1)

Every argument for A3 is a build argument, and §3.1 removed it: the
build is free and already verified. What remains after the build is
signing, notarization, hardware, platform idiom, and support — and Ken
has none of those for macOS and only a partial answer for Linux (§3.3).
Shipping an artifact that cannot be smoke-tested (D2) or supported (D8)
would be a claim the project cannot honor (D7).

A4 (Windows + Linux) is the option that *sounds* moderate and measures
worst: §3.2 shows Linux is the heaviest dependency target, and §3.3
shows the GUI has no static-linking escape hatch there — musl does not
implement `dlopen` in static binaries at all, and every Linux windowing
binding pdfce uses is `dlopen`-based. "Linux is the cheap second
platform" is exactly backwards. If Linux is ever added, the CLI goes
first as a static musl binary — cheap, correct, genuinely portable —
and the GUI follows as a separate, glibc-pinned, AppImage-shaped effort.

### 5.3 Why cross-target `cargo check` beats a macOS runner

Four reasons, in decreasing order of force:

1. **It is the same signal.** A macOS runner running `cargo test` would
   catch compile errors, link errors, and test failures. pdfce has no
   macOS-specific code, so essentially all realistic macOS breakage is
   at compile time — dependency `cfg` drift, an `objc2` bump, an
   upstream API change. `cargo check` catches that class, at Linux
   runner cost instead of ~10× it, in 32 seconds.
2. **A red X nobody can fix is worse than no signal.** With no Mac (D8),
   a macOS-only link or test failure is unactionable. Unactionable red
   CI is the classic broken-window: within a month everyone is ignoring
   the whole board, including the jobs that *do* matter.
3. **It runs locally too.** The same three commands run on Ken's box
   before a commit. A runner cannot.
4. **The `wasm32` check outranks macOS in value and nobody had asked for
   it.** `ARCHITECTURE.md` §3's entire crate split exists to make the
   web fork a shell-crate swap. Today that invariant is enforced only
   negatively (no GUI crates in core). A positive check —
   `pdfce-core` + `pdfce-render` actually compile for
   `wasm32-unknown-unknown` — is what catches the real failure mode:
   someone adds a dependency to `pdfce-core` that needs a C toolchain,
   threads, or a filesystem, and nobody notices until the fork starts.
   It passes today in **6.5 seconds** and the current tree
   (`flate2`+`miniz_oxide`, `thiserror`, `tiny-skia`+`png`+`fdeflate`)
   is entirely pure Rust. This is the cheapest architectural insurance
   available to the project.

### 5.4 Why package managers are the right answer for update discovery, honestly evaluated

The instruction was to evaluate winget/scoop/chocolatey *honestly for a
portable app*, so here is the honest version including the parts that
argue against.

**Scoop is the natural fit and should be first.** Its own documentation
says portable apps — "compressed files that run standalone" — "work
best." It installs into the user's Scoop directory with shims on the
**user** PATH; its stated goals include eliminating UAC prompts,
avoiding PATH pollution, and avoiding install/uninstall side effects.
A normal (non-global) install needs no administrator rights and writes
nothing to the registry. GitHub-release tracking is first-class:
`"checkver": "github"` with `homepage` set to the repository matches
release tags and ignores pre-releases, and `autoupdate` templates the
new version into the download URL, obtaining the hash by extraction or
by downloading and hashing the file itself. Almost nothing about pdfce
needs to change to be Scoop-installable. **Required manifest fields are
`version`, `description`, `homepage`, and `license`**; `url`, `hash`,
`bin`, `extract_dir`, `checkver`, `autoupdate`, and `persist` are
strongly expected in practice — note `persist`, which is Scoop's own
answer to the user-state problem R15 addresses.

**WinGet is second: broader reach, more friction.** It genuinely
supports portable distribution — `InstallerType` includes both
`portable` and `zip`, and for archives `NestedInstallerType` may be
`portable` (with the schema explicitly relaxing the single-nested-file
restriction in that case). Upgrade was designed in from the start: the
package is upgraded in place, the Apps & Features entry updates, and the
symlink is overwritten. The friction is threefold. First, submission is
a pull request to the community package repository, with review.
Second — and this is the honest caveat against putting it first —
**zip + nested-portable packages still have open rough edges**
(winget-cli issues #3279, #2806 and #6215, the latest from May 2026).
Third, a WinGet portable install is **not** registry- and PATH-neutral:
it creates an Add/Remove Programs entry (user-scope installs land under
`HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\`), places
files under `%LOCALAPPDATA%\Microsoft\WinGet\Packages\<PackageIdentifier>\`,
creates a symlink in `…\WinGet\Links\`, and appends to PATH — with the
symlink route requiring administrator rights or Developer Mode, falling
back to putting the package directory itself on PATH otherwise. That is
WinGet's behavior, not pdfce's; **pdfce itself still writes no registry
keys**, satisfying D2. Users who want the pure portable experience keep
using the zip.

**Chocolatey is third and probably skippable.** It is more
installer-centric, its portable convention is a wrapper around
"download and unzip," and its moderation queue is slower. It adds reach
among users who already standardized on it and adds nothing else.

**What package managers genuinely deliver:** real "check and upgrade"
UX, for free, forever, with pdfce making no network call and having no
opinion about the user's network. The user opted in by installing a
package manager. That is a cleaner consent boundary than any checkbox
pdfce could ship.

**What they do not deliver — stated so this is not oversold:** they only
serve users who already use them, which is a minority of Windows users.
A plain zip on a releases page remains the primary channel and always
will. Package managers are a *supplement* that happens to solve updating
for the subset of users most likely to care.

**Both are blocked on `LEGAL.md` §1.** Scoop requires a `license`
property; WinGet requires `License` in the defaultLocale manifest
(alongside PackageIdentifier, PackageVersion, PackageLocale, Publisher,
PackageName, ShortDescription, ManifestType, ManifestVersion). This is
not a formality to work around — it is a concrete, practical cost of
leaving the license open, and it belongs next to decision 001's
copyleft-prior-art argument in the list of reasons to resolve it.

*Manifest schemas drift. Re-read the current official schema at the
moment a manifest is actually written* (D7).

### 5.5 Why no auto-updater, ever, and why a background checker is nearly as bad

**B5 (auto-updater) is rejected permanently** on three independent
grounds, any one of which suffices:

1. **It contradicts D2 structurally.** A self-replacing binary is an
   installer with extra steps — it needs write access to its own
   directory, a restart dance, and rollback on failure. "No installer"
   is a load-bearing product promise, not a packaging convenience.
2. **It is a code-execution channel in a program whose day job is
   parsing hostile input.** `ARCHITECTURE.md` §10 commits pdfce to
   treating every PDF as adversarial. Adding "downloads and executes a
   binary" to that same process is the wrong direction, and it is a
   supply-chain attack surface pdfce would then have to defend with
   signature verification, key management, and rollback — none of which
   a sole-operator, non-monetized project should take on.
3. **It cannot be reconciled with D1.** An auto-updater that is off by
   default is not an auto-updater; one that is on by default violates
   the privacy posture outright.

**B4 (opt-in background checker) is rejected for now** for a subtler
reason worth writing down: "opt-in" is a consent event, and a background
poll spends that one consent event indefinitely. A user who ticks a box
in month one is contacting a server in month eighteen without a fresh
decision. A **user-initiated** check spends consent exactly once per
click, which is the only design where the privacy claim (D1, D7) stays
literally true without qualification. If a periodic check is ever
genuinely wanted, it deserves its own decision record and its own
disclosure — not a quiet re-reading of this one.

### 5.6 The real engineering consequence of choosing folder-replace, and why it must be decided now

Choosing manual replace-the-folder is not a "do nothing" answer. It
imposes a packaging requirement that is cheap today and expensive later
— the same shape as decision 002's error-structure obligation and
decision 001's `ByteSpan` obligation.

**Replacing a folder destroys everything in it.** The portable-app
convention is to keep settings next to the executable — and
`ARCHITECTURE.md` §6 already contemplates this ("per-user
settings/recents may still use a conventional config dir, but the app
must run read-only-folder-clean"). The moment pdfce writes a settings
file, a recent-files list, a window-layout cache, or (later) OCR
language data, a naive "delete the old folder, unzip the new one"
silently wipes user state, and the user's own muscle memory is the
thing that broke them. Scoop's `persist` field exists for exactly this
reason, which is corroboration that the problem is real and conventional
rather than hypothetical.

So: **the distribution folder is partitioned from the start** (R15).
Replaceable payload — binaries, assets, `THIRD_PARTY_LICENSES.md`,
`README` — is one set; user state is another, in a clearly-named
location the documented update procedure tells the user to keep. Decide
this before the first Pass that persists anything, because retrofitting
it means migrating existing users' state, and this project's whole
method is to pay these costs while they are still zero.

### 5.7 Why the privacy claim needs sharpening rather than softening

D1 is a promise, and D7 says promises get verified against the source
rather than worded from convention. Three sharpenings follow from the
evidence:

- **§3.4's `webbrowser` finding.** The shipped binary contains code that
  can launch a browser. It cannot be removed. Saying "pdfce makes no
  network requests and contains no HTTP client or TLS stack; clicking a
  link hands the URL to your operating system's default browser, and
  that request is your browser's, not pdfce's" is both true and
  complete. Saying "no network code" would be false.
- **`THIRD_PARTY_LICENSES.md` is the proof.** It is a generated,
  shipped, complete list of everything linked into the binary. A
  skeptical user can read it and confirm there is no HTTP client. That
  makes the privacy claim *verifiable by the reader*, which is a
  stronger position than any assertion — and it is a reason to guard
  R12 carefully, because the first network crate added costs that proof.
- **If a checker is ever built, the disclosure names what leaks.** Even
  a single user-initiated GET reveals the client IP, the pdfce version,
  and a timestamp to whoever hosts the manifest, and creates a log entry
  there. "No data is sent" would be false; "no identifiers are sent, but
  your IP address and version are visible to the host" is true. Write
  the true one (§6.4).

### 5.8 Checksums and integrity: cheap now, impossible to reconstruct later

Both package-manager formats require a hash of the artifact. More
fundamentally, a user downloading an unsigned portable binary has no way
to verify they got what was published — and per §3.3 the binary will
remain unsigned unless Ken takes the Azure Artifact Signing decision.
Publishing SHA-256 alongside every release from the first one costs one
command; producing an honest hash for a past release after the artifact
has been rebuilt is impossible, because a rebuild is not bit-identical.
This is the smallest item in this record and the only one with a hard,
unrecoverable deadline (R16).

---

## 6. What this decision produces

### 6.1 Standing rules (binding; add verbatim to `ROADMAP.md`, continuing decision 002's R1–R8)

- **R9 — One supported platform at a time.** pdfce claims support for
  exactly the platforms on which a human has run the
  `ARCHITECTURE.md` §6 packaging smoke test on real hardware. Today:
  Windows x64, and nothing else. **A green CI job is a compile signal,
  never a support claim**, and never appears in user-facing copy as one.
- **R10 — Platform-clean by construction.** No `#[cfg(target_os)]` /
  `#[cfg(windows)]` / `#[cfg(unix)]` in `pdfce-core` or `pdfce-render`,
  ever. Platform conditionals in `pdfce-gui`/`pdfce-cli` carry a comment
  naming why. Build-level platform specificity stays target-scoped in
  `.cargo/config.toml` (as the existing `+crt-static` block already is).
  No path-separator, line-ending, or filesystem-case assumptions
  anywhere.
- **R11 — CI runs natively only where pdfce ships, or where the runner
  is free *and* the failure is actionable.** Every other platform's
  compile signal comes from cross-target `cargo check` on the ubuntu
  runner. The `wasm32-unknown-unknown` check of `pdfce-core` +
  `pdfce-render` is a first-class invariant guard and ranks above
  macOS: it protects the web-fork premise that `ARCHITECTURE.md` §3's
  crate split exists to serve.
- **R12 — No network client in the tree.** No HTTP/TLS/socket client
  crate may enter any pdfce crate. Enforced fail-closed by the
  `no-network` CI job. Unlocking requires a **new decision record**
  amending this one, naming the crate and the feature it serves.
  `pdfce-core` and `pdfce-render` may never contain network code under
  any future decision.
- **R13 — pdfce never self-updates.** It never downloads a file the user
  did not ask for, never replaces its own binary, never executes
  anything it fetched, and never launches an installer. Permanent.
- **R14 — Update discovery is external by default.** Manual
  replace-the-folder plus package-manager manifests are the shipping
  answer. Any in-app checker obeys §6.4 in full — opt-in, off by
  default, user-initiated per check, display-only, no identifiers,
  kill-switchable, disclosed in §6.3's exact terms.
- **R15 — The distribution folder is partitioned.** Replaceable payload
  and user state are separate; user state never sits loose among the
  binaries; the documented update procedure names exactly which files to
  keep. Binding from the first Pass that persists anything.
- **R16 — Release integrity and attribution track reality.** Every
  release publishes SHA-256 checksums for every artifact.
  `about.toml`'s `targets` lists exactly the platform triples actually
  shipped — no more (inaccurate attribution) and no fewer (missing
  attribution).

### 6.2 The CI changes

**New job — cross-target compile check** (modeled on the existing
job style: explicit, commented, fail-loud):

```yaml
  cross-check:
    name: cross-target compile check (macOS / wasm32)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      # Pin to rust-toolchain.toml's version so rustup does not install a
      # second toolchain and so `targets:` lands on the toolchain cargo
      # will actually use.
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: "1.97.1"
          targets: aarch64-apple-darwin,wasm32-unknown-unknown
      # WHAT THIS PROVES: the workspace type-checks for these targets.
      # WHAT IT DOES NOT PROVE: that it links, runs, or opens a window.
      # `cargo check` needs no linker and no platform SDK, which is why
      # this runs at ubuntu rates ($0.006/min) instead of macOS rates
      # ($0.062/min) — see docs/decisions/003-distribution-posture.md
      # §3.1 and §5.3.
      # macOS is NOT a supported platform (rule R9) — this job exists to
      # keep the code portable, not to claim it runs there.
      - run: cargo check --workspace --target aarch64-apple-darwin
      # The web-fork invariant, checked positively rather than only by
      # the absence of GUI crates: pdfce-core + pdfce-render must stay
      # WASM-compilable (ARCHITECTURE.md §3, §1). pdfce-gui/pdfce-cli are
      # deliberately excluded — they are native shells.
      - run: cargo check -p pdfce-core -p pdfce-render --target wasm32-unknown-unknown
```

**New job — no network client in the tree** (fail-closed, same shape as
`gui-core-separation` and `ui-strings`):

```yaml
  no-network:
    name: verify no HTTP/TLS client in any pdfce crate
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: "1.97.1"
      # ARCHITECTURE.md §1.1 / decision 003 rule R12: pdfce makes no
      # network calls, and the way that stays true is that no network
      # client is linkable in the first place. Fail-closed denylist.
      # NOT on the list, deliberately: `url` (a parser) and `webbrowser`
      # (hands a URL to the OS browser; unconditionally linked by
      # eframe 0.35 and not removable — see decision 003 §3.4). Also not
      # `zbus` — that is the Linux accessibility D-Bus bridge, a local
      # IPC socket, which is why this check runs against the SHIPPED
      # Windows target.
      - name: cargo tree denylist (all crates, shipped target)
        run: |
          set -e
          deny='reqwest|ureq|hyper|isahc|attohttpc|curl|surf|native-tls|openssl|rustls|webpki|tokio|socket2|hickory|trust-dns'
          fail=0
          for p in pdfce-core pdfce-render pdfce-cli pdfce-gui; do
            if cargo tree -p "$p" --target x86_64-pc-windows-msvc 2>/dev/null \
                 | grep -Ei "(^|[[:space:]])($deny)[[:space:]]"; then
              echo "::error::$p has a network-client dependency — violates the no-network posture (ARCHITECTURE.md §1.1, decision 003 R12). Adding one requires a NEW decision record."
              fail=1
            fi
          done
          exit $fail
```

**Fix to the existing `gui-core-separation` job** (§3.5 defect 1): run
each `cargo tree` grep a second time with
`--target x86_64-pc-windows-msvc`, so the *shipped* graph is checked and
not only the ubuntu host's. `cargo tree --target` is metadata-only and
needs no installed target.

**Fix to every existing job** (§3.5 defect 2): replace
`dtolnay/rust-toolchain@stable` with `@master` + `toolchain: "1.97.1"`
to match `rust-toolchain.toml`.

### 6.3 The privacy, platform, and signing copy (use this wording; do not paraphrase it looser)

For `README.md` and any future about-box, once user-facing copy is
written:

> **pdfce does not use the network.** It contains no HTTP client and no
> TLS stack — you can confirm this yourself in
> `THIRD_PARTY_LICENSES.md`, which lists every library linked into the
> binary. There is no telemetry, no analytics, no crash reporting, no
> licence check, and no update check. Every document you open is
> processed entirely on your machine.
>
> If you click a link inside pdfce, pdfce hands the address to your
> operating system's default browser. The request is made by your
> browser, not by pdfce.
>
> **Updates** are manual: download the new zip and replace the program
> files (keep your `<user-state>` folder). pdfce will never update
> itself.
>
> **Supported platform:** Windows 10/11, 64-bit. pdfce's code is kept
> portable and is compiled for Linux, macOS, and WebAssembly on every
> change, but those builds are not tested or supported, and no artifact
> is published for them.
>
> **The download is not code-signed.** Windows will show a SmartScreen
> warning ("Windows protected your PC") the first time you run it;
> choose *More info* → *Run anyway*. This warning will appear again for
> each new version, because an unsigned program's reputation does not
> carry across releases. Verify your download against the published
> SHA-256 checksum if you want certainty about what you received.

### 6.4 Specification for an in-app update checker, if a §9 trigger ever fires

Written now so that if it is ever built, it is built right, and so that
a future session does not re-derive the constraints (same method as
decision 002 §6.4's retrofit recipe).

1. **Placement.** `pdfce-gui` only, in its own module. Never
   `pdfce-core`, never `pdfce-render` (R12) — a network dependency in
   core would slip past the `cargo tree` *GUI*-crate grep while
   violating D3 in spirit, which makes it more dangerous than an
   obvious violation, not less.
2. **Consent.** Off by default. A single preference, worded plainly,
   with §6.3-grade disclosure shown *at the point of opt-in*, not buried
   in a document.
3. **Trigger.** User-initiated only: a menu item or button. No startup
   check. No timer. No poll. Enabling the preference enables the
   *button*, not a background behavior. A scheduled check is a separate
   decision requiring its own record (§5.5).
4. **Request.** One HTTPS GET of a small static manifest (a
   `latest.json`-shaped file published with each release, *not* the
   GitHub API — the API is rate-limited per IP and returns far more than
   is needed). No query string, no custom headers beyond a fixed
   `pdfce/<version>` user-agent, no cookies, no redirect-following to a
   different host, no retry storm. Short timeout, single attempt.
5. **Response handling.** Compare versions and display the result plus a
   link. **Never download anything. Never write an executable. Never
   launch an installer** (R13). "New version available — open the
   releases page" is the entire feature.
6. **Kill switches.** `PDFCE_NO_NETWORK=1` and a `--offline` flag
   disable it unconditionally and take precedence over the preference,
   so a locked-down environment can guarantee the behavior without
   trusting a setting.
7. **Dependency.** Adding an HTTP/TLS client is a `LEGAL.md` §6.2
   license check *and* an R12 unlock *and* a new decision record. Weigh
   binary-size cost against D2 and attack-surface cost against
   `ARCHITECTURE.md` §10 in that record, not here.
8. **CLI.** If a `pdfce-cli` equivalent is ever added it is an explicit
   subcommand, never implicit in another command, and its stdout obeys
   decision 002's R5 (locale-invariant machine output).

---

## 7. What this decision explicitly does NOT decide

- **The OSS license.** `LEGAL.md` §1 is Ken's decision and remains open.
  This record is deliberately written so that nothing in it presupposes
  publication *timing*: the Windows-only scope, the CI jobs, the
  platform-clean rules, and the folder-partition requirement are all
  actionable today with no remote and no release. Only the
  package-manager manifests and the README copy wait on §1, and they are
  filed as a gated Backlog entry rather than an assumed next step.
- **Whether to buy code signing.** §3.3 recommends against a traditional
  OV/EV certificate and identifies Azure Artifact Signing as the option
  to price, but it is a recurring cost (D9) and therefore Ken's call —
  the same class of decision as the license, and outside what the
  `docs/decisions/` protocol decides.
- **When a first release happens, or whether one happens.** This record
  says what v1 *is* when it exists.
- **Whether the repository is ever public.** A private remote is
  sufficient to make CI real and is permitted today.
- **The `pdfce-web` fork's distribution.** A served web app has an
  entirely different update model (§9 T7).
- **Anything about how pdfce *renders* documents on any platform.**
  Decision 002 §7's separation applies here too: platform scope is
  about artifacts, not about correctness.

---

## 8. Consequences

**Positive**

- Zero dependencies added; `THIRD_PARTY_LICENSES.md`, the `cargo tree`
  invariants, binary size, and the WASM-fork constraint are all
  untouched.
- The web-fork invariant gains a *positive* CI check for the first time,
  at 6.5 seconds per run — arguably the highest value-per-line change in
  this record and not something the original question asked for.
- Two latent CI defects (wrong-target invariant checks, toolchain
  mismatch) are fixed before the first run rather than debugged during
  it.
- The no-network posture becomes machine-enforced rather than
  honor-system, matching what decision 002 did for UI strings and what
  the workspace already does for GUI-core separation. §1.1 stops being a
  promise and becomes a build gate.
- The privacy claim becomes *verifiable by a skeptical reader* via a
  file that already ships.
- The folder-partition requirement (R15) is captured while it costs
  nothing — before any settings file exists.
- The `ROADMAP.md` "deliberately deferred" list is now **empty**. Every
  product-scope question flagged at bootstrap on 2026-07-23 has been
  answered within one week.

**Negative**

- pdfce reaches fewer users than it technically could. Accepted: the
  code stays portable, so the cost of changing this later is a packaging
  Pass, not a rewrite.
- If someone asks for a Linux build tomorrow, the answer is "not
  supported" despite the code compiling. Mitigated by §9 T2 — a
  contributor who owns a platform is the clean path, and this record
  says so in advance rather than improvising under social pressure.
- Users on the plain-zip channel have no way to learn a new version
  exists except by visiting the page. That is a real cost, honestly
  accepted, and it is exactly what §9 T6 exists to re-weigh if a
  security-relevant fix ever ships.
- Shipping unsigned means every release re-triggers SmartScreen for
  every user, indefinitely. This is a genuine adoption cost, not a
  cosmetic one, and it is why §9 T9 exists rather than treating "don't
  sign" as settled.
- R12's denylist is a name-matching heuristic and will not recognize a
  network crate nobody thought of. Same honest limitation decision 002
  recorded for `ui-strings`; the mitigation is the same (review plus a
  small, readable dependency set), and the answer is to add names when
  they appear, not to escalate to a heavier mechanism preemptively.

**Neutral**

- Independent of `LEGAL.md` §1 for everything actionable today; §1 gates
  only the package-manager channels and public-facing copy.
- Independent of decisions 001 and 002 in both directions. No
  interaction with the `ByteSpan`/content-stream obligations or the
  `ui_text` catalog.
- The macOS and Linux `cargo check` jobs will occasionally break on
  dependency updates. That is the system working (§9 T3), not a reason
  to reconsider.

---

## 9. Revisit triggers

Re-open this record if any of the following becomes true:

1. **T1 — `LEGAL.md` §1 is resolved and a public repository exists.**
   Unblocks both package-manager channels (Scoop requires `license`,
   WinGet requires `License`), makes Actions usage free and unmetered on
   standard runners (removing the macOS cost objection), and makes
   GitHub Releases usable. Single highest-leverage trigger in this
   record.
2. **T2 — Mac hardware, or a contributor who will own a platform.**
   Either makes a failure on that platform actionable. Add the runner
   then. A contributor owning a platform is the *only* clean path to
   shipping a platform Ken cannot debug — do not ship one without an
   owner.
3. **T3 — A cross-target `cargo check` starts failing.** The early
   warning working as designed. Fix it in the Pass that broke it. If the
   fix would require platform-specific code in `pdfce-core`/
   `pdfce-render`, that is an R10 violation and an architecture
   question, not a CI question.
4. **T4 — The first real CI run.** Expect two findings: the ubuntu
   `cargo test --workspace` job *links* `pdfce-gui`, which `check` never
   exercises (if it needs `apt` packages, add them and record the
   finding in `D:\dev\rag\rust\`); and the toolchain-action mismatch of
   §3.5 if it has not been fixed by then.
5. **T5 — The release zip crosses ~100 MB, or the folder starts holding
   user state a naive replace would destroy.** Both degrade the
   replace-the-folder story materially. OCR language data is the most
   likely first cause (`ROADMAP.md` Backlog).
6. **T6 — Someone asks for an update check, or a security-relevant fix
   ships.** "Users are running a version with a known parser
   vulnerability and have no way to find out" is the one argument that
   genuinely outweighs the privacy cost, and it deserves an honest
   hearing if it arrives — pdfce parses adversarial input by design
   (`ARCHITECTURE.md` §10). Answer it with §6.4, not with a new design.
7. **T7 — `pdfce-web` is started.** A served app's "update" is a
   deployment. Re-read this record before assuming any of it transfers.
8. **T8 — Windows-on-ARM demand, or an x64-emulation failure on ARM64
   Windows.** Adding `aarch64-pc-windows-msvc` is then a packaging Pass.
9. **T9 — Code signing becomes affordable or necessary.** Fires if pdfce
   ships releases often enough that the never-fading SmartScreen warning
   becomes a real adoption barrier, or if Azure Artifact Signing
   eligibility is confirmed for Ken as a Canada-based individual
   developer. Re-verify price, eligibility, and the individual-applicant
   history requirement at that moment — this record does not settle
   them, and it is Ken's decision, not the protocol's (§7).

---

## 10. Follow-up actions

**Engineering (next Pass that touches CI or packaging):**

1. Add the `cross-check` CI job (§6.2). Verified passing today.
2. Add the `no-network` CI job (§6.2). Verified passing today (zero
   hits).
3. Fix `gui-core-separation` to also check
   `--target x86_64-pc-windows-msvc` (§3.5 defect 1).
4. Pin every job's toolchain action to `1.97.1` (§3.5 defect 2).
5. Amend `ARCHITECTURE.md` §1.1 with the `webbrowser` clause (§3.4) — a
   precision correction, not a weakening.
6. Amend `ARCHITECTURE.md` §6: state Windows-x64-only as a decision with
   a pointer here, and add R15's payload/user-state partition to the
   packaging contract and to the smoke-test procedure.
7. Add the §6.3 privacy/platform/signing copy to `README.md` when
   user-facing copy is next touched. Do not paraphrase it looser (D7).

**Librarian (`pdfce-librarian`):**

8. Archive this record as `docs/decisions/003-distribution-posture.md`.
9. Add R9–R16 to `ROADMAP.md`'s *Standing rules*.
10. **Strike both remaining bullets** from `ROADMAP.md`'s "Product-scope
    decisions — deliberately deferred" list, marking them RESOLVED with
    a pointer here — and note that the list is now **empty**, which is
    itself worth recording.
11. Add a dated `ARCHITECTURE.md` §12 entry cross-referencing this
    record.
12. File a new `ROADMAP.md` Backlog entry **"Release & distribution
    channel"** — Scoop manifest, WinGet portable manifest, SHA-256
    checksums, README platform/privacy/signing copy — explicitly
    **blocked on `LEGAL.md` §1**, so the work is captured and visibly
    gated.
13. Write two findings to `D:\dev\rag\egui\`: (a) eframe 0.35 hardcodes
    `egui-winit` `features = ["clipboard", "links"]` with
    `default-features = false`, so `webbrowser` is **unconditional** in
    any native eframe app and cannot be feature-disabled downstream;
    (b) an eframe 0.35 glow app cross-`check`s clean for
    `x86_64-unknown-linux-gnu` and `aarch64-apple-darwin` from a Windows
    host with no SDK, and all Linux windowing bindings are `dlopen`-based
    (`wayland-sys` `dlopen` feature, `x11-dl`, `xkbcommon-dl`) — which is
    also precisely why `x86_64-unknown-linux-musl` cannot host the GUI
    (musl has no `dlopen` in static builds; winit issue #1818).
14. Write one finding to `D:\dev\rag\rust\`: cross-target `cargo check`
    as a zero-cost portability gate — the recipe, what it proves, what
    it does not — plus the multi-target `cargo about generate -c` audit
    recipe used in §3.2.

**Operator check-in #1 (informational, not a decision request):**

`LEGAL.md` §1 now blocks more than publication. It blocks the CI that
exists but has never run, it blocks both package-manager channels, and
it determines whether Actions usage is metered at all. Decision 001
already found it gating which prior art is usable. It is the
highest-leverage open item in the project, and it is the one decision
this protocol explicitly cannot make.

**Operator check-in #2 (a decision request for Ken, outside this
protocol):**

Windows code signing. An unsigned `pdfce-gui.exe` triggers SmartScreen
on **every** release, permanently — an unsigned binary's reputation does
not carry across versions — and EV certificates no longer bypass it, so
the expensive traditional route buys nothing here. **Azure Artifact
Signing** is approximately **$10/month**, needs **no hardware token**,
signs from GitHub Actions directly, and admits **individual developers
located in the United States or Canada**, which Ken satisfies. It is the
first genuinely cheap Windows signing path this project has had. It is
also a recurring cost against the non-monetized constraint, so the call
is his, not this protocol's. Verify current price, tier limits, and the
individual-applicant validation requirements at the moment of decision.

---

## 11. References

- `docs/ARCHITECTURE.md` §1/§1.1 (goal, privacy posture), §3 (workspace
  + GUI-core invariant), §6 (single-folder packaging), §7 (CLI
  contract), §10 (adversarial-input hardening), §12 (decision log)
- `docs/ROADMAP.md` — "Product-scope decisions — deliberately deferred"
  (both remaining bullets superseded by this record); Standing rules
  R1–R8 (decision 002)
- `docs/decisions/001-oxidize-pdf-adopt-vs-build.md` — precedent for
  `LEGAL.md` §1 gating an apparently-unrelated engineering choice
- `docs/decisions/002-i18n-timing.md` §6.2 (the CI-lint pattern reused
  here), §5.5 (the "decide the irreversible half now" method)
- `docs/LEGAL.md` §1 (license undecided), §6 (dependency licensing and
  generated attribution)
- `docs/PRIOR_ART.md` — KillerPDF (Windows-only single portable EXE,
  3,128 stars) as evidence that single-platform portable is a proven
  posture for this product category
- `.github/workflows/ci.yml` — existing job set; the two defects in §3.5
- **Measured in-tree on 2026-07-30** (all first-hand, re-runnable):
  `cargo check --workspace --target x86_64-unknown-linux-gnu` → exit 0,
  1m12s; `--target aarch64-apple-darwin` → exit 0, 32s;
  `cargo check -p pdfce-core -p pdfce-render --target
  wasm32-unknown-unknown` → exit 0, 6.5s; `cargo tree -p pdfce-gui
  --target <triple>` crate counts 147/155/237;
  `cargo about generate -c <3-target config>` → exit 0, zero copyleft;
  `Cargo.lock` network-crate grep → one hit (`url`, via `webbrowser`);
  `eframe` 0.35.0 `Cargo.toml` `[target.'cfg(not(target_arch =
  "wasm32"))'.dependencies.egui-winit] features = ["clipboard","links"]`;
  `egui-winit` 0.35.0 `links = ["webbrowser"]`, `webbrowser` optional;
  `wayland-sys` `dlopen` feature enabled; release artifacts 870,912 B +
  7,622,656 B, zipped 3,633,770 B; no git remote configured
- **External sources verified 2026-07-30:**
  - GitHub Actions billing concepts and runner pricing (included
    minutes by plan; Linux $0.006 / Windows $0.010 / macOS $0.062 per
    minute for standard 2–4-core runners; public repositories free on
    standard runners; the `actions-minute-multipliers` page now
    redirects to "Actions runner pricing"); GitHub changelogs
    2025-12-16 and 2026-01-01 (pricing simplification, rate reductions)
  - `microsoft/winget-pkgs` manifest schema v1.12.0 (`installer.md`:
    `InstallerType` portable/zip, `NestedInstallerType: portable`,
    `NestedInstallerFiles`, `ArchiveBinariesDependOnPath`;
    `defaultLocale.md`: required fields including `License`);
    `microsoft/winget-cli` spec #182 (portable install/upgrade
    behavior, ARP entry, `…\WinGet\Links\` symlink, PATH append,
    `--location`/`--rename`/`--purge`/`--preserve`); winget-cli issues
    #3279, #2806, #6215 (zip + nested-portable rough edges)
  - Scoop README (portable apps "work best"; no UAC prompts; no PATH
    pollution) and Scoop wiki App-Manifests (required `version`,
    `description`, `homepage`, `license`) and App-Manifest-Autoupdate
    (`checkver: "github"`, release-tag regex, `autoupdate` hash
    extraction)
  - Apple Developer news 2024-08-06 (macOS Sequoia removes the
    Control-click Gatekeeper override); Apple mac-help "Open a Mac app
    from an unidentified developer" (macOS 26 Tahoe flow); Apple
    Developer Program enrollment ($99 USD per membership year);
    Apple Developer ID (membership required for notarization)
  - Microsoft Learn "SmartScreen reputation" (2026-05-04): unsigned
    warning behavior, self-signed equivalence, reputation not carrying
    across updates, EV no longer bypassing SmartScreen, Smart App
    Control; Azure Artifact Signing ~$10/month with no hardware token
  - CA/B Forum Code Signing Baseline Requirements, ballot CSC-17
    (effective 2023-06-01): FIPS 140-2 Level 2 / CC EAL4+ hardware key
    storage for OV as well as EV
  - Microsoft Learn Azure Artifact Signing quickstart (2026-07-23) and
    FAQ (2026-07-29): Public Trust eligibility by country; **individual
    developers must be located in the United States or Canada**;
    Microsoft Verified ID / government-ID validation; paid subscription
    required; certificate CN fixed to validated legal name; billing not
    pro-rated
  - musl mailing list (Rich Felker, 2012-12-08): `dlopen` is not
    implemented for static binaries; `rust-windowing/winit` issue #1818
    ("Support for musl": no `dlopen`, `wayland-sys` cannot open
    `libwayland-client.so`); `rust-lang/rust` issue #135244
    (musl target defaults to `crt-static`; disabling it breaks);
    AppImage `pkg2appimage` excludelist (never bundle
    libGL/libEGL/libwayland-client/glibc); `cargo-zigbuild` README
    (glibc version pinning, `crt-static` unsupported)
- **Genuinely unconfirmed, recorded so nobody treats them as settled**
  (D7):
  1. **GitHub's per-OS quota-drain formula** — no current document
     states how included minutes are debited per operating system; the
     ~1.67× / ~10× ratios above are implied by list price only.
  2. **The dollar figures on the Azure Artifact Signing pricing page** —
     the page renders amounts via JavaScript; the "~$10/month" figure is
     confirmed only via Microsoft Learn, and the widely-reported
     $9.99 / $99.99 tier pricing and $0.005 per-signature overage could
     not be confirmed from the pricing page itself.
  3. **Removal of the 3-year verifiable-history requirement for
     individual applicants** to Azure Artifact Signing (reported as of
     April 2026) — the current official quickstart and FAQ no longer
     state such a rule anywhere, but its removal is evidenced by
     Microsoft Q&A threads and practitioner reporting rather than by a
     Microsoft document. Organizations are still being rejected for
     being "incorporated less than 3 years ago."
