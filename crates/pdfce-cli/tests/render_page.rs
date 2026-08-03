//! # `pdfce-cli render-page` integration tests
//!
//! Black-box tests: they spawn the **real binary** (via Cargo's
//! `CARGO_BIN_EXE_<name>` env var, set for integration tests of any crate
//! with a `[[bin]]`) and assert on its process contract — exit code,
//! stdout bytes, stderr bytes, and the file it wrote. That is deliberate.
//! The unit tests in `main.rs` cover the pure mapping functions; what
//! *this* file exists to protect is the part a script depends on and a
//! refactor cannot see: the exit-code table (docs/ARCHITECTURE.md §7) and
//! the stable stdout result line (see `main.rs`'s module header, "stdout
//! result-line format").
//!
//! ## Why the fixtures are built inline
//!
//! Every PDF used here is assembled byte-by-byte by [`build_pdf`] below,
//! in-process, and written to a temp file. Two reasons, both binding:
//!
//! - **docs/LEGAL.md §5**: test-corpus PDFs are synthetic or clearly
//!   rights-cleared, never a downloaded real-world file of unknown
//!   provenance. Generating the bytes here makes provenance a
//!   non-question.
//! - **Legibility**: the exact structure under test (how many pages, what
//!   the content stream draws, what the MediaBox is) is visible at the
//!   call site instead of hidden in an opaque binary blob a future reader
//!   would have to hex-dump to understand.
//!
//! The builder emits a classic §7.5.4 cross-reference **table** rather
//! than a §7.5.8 xref stream. Both are supported by `pdfce-core`; the
//! classic form is used here purely because it is readable in the test
//! source (whole-file coverage for the PDF 1.5 forms lives in
//! `crates/pdfce-core/tests/pdf15_streams.rs`).
//!
//! ## Why no `tempfile` dependency
//!
//! [`TempDir`] below is ~30 lines and adds zero packages to the
//! dependency graph (docs/LEGAL.md §6: every dependency is a license
//! classification and an attribution entry). Uniqueness comes from
//! process id + a monotonic counter + nanosecond clock, which is
//! sufficient for a test harness that owns the paths it creates, and the
//! `Drop` impl cleans up even when an assertion panics.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Path to the freshly built `pdfce-cli` binary. Cargo sets this for
/// integration tests, so the test always exercises the binary produced by
/// the same build — never a stale one on `PATH`.
const BIN: &str = env!("CARGO_BIN_EXE_pdfce-cli");

// ---------------------------------------------------------------------------
// Fixture construction
// ---------------------------------------------------------------------------

/// Assemble a syntactically complete single-generation PDF from a list of
/// `(object number, body)` pairs, appending a classic cross-reference
/// table and a trailer that names `1 0 R` as the catalog.
///
/// The layout follows §7.5: header, body, `xref` section with one
/// subsection covering objects `0..=n`, `trailer`, `startxref`, `%%EOF`.
/// Offsets are recorded as each object is emitted, so the table is
/// correct by construction rather than by hand-counting — which matters,
/// because `pdfce-core` is strict: a wrong offset is a load failure, not
/// a warning.
///
/// Free entry `0` is emitted as the spec's mandatory
/// `0000000000 65535 f` head-of-free-list. Entries are exactly 20 bytes
/// each including the `\r\n` terminator, as §7.5.4 requires.
fn build_pdf(objects: &[(u32, String)]) -> Vec<u8> {
    let mut buf = b"%PDF-1.4\n".to_vec();
    let mut offsets: Vec<(u32, usize)> = Vec::new();
    for (num, body) in objects {
        offsets.push((*num, buf.len()));
        buf.extend_from_slice(format!("{num} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let xref_at = buf.len();
    let size = objects.len() + 1; // +1 for the free object 0
    buf.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f\r\n").as_bytes());
    for num in 1..=objects.len() as u32 {
        let (_, off) = offsets
            .iter()
            .find(|(n, _)| *n == num)
            .expect("object numbers must be 1..=n and contiguous");
        buf.extend_from_slice(format!("{off:010} 00000 n\r\n").as_bytes());
    }
    buf.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n")
            .as_bytes(),
    );
    buf
}

/// A document with `contents.len()` pages, page *i* drawing
/// `contents[i]`, all sharing a 200x100 MediaBox.
///
/// The non-square box is on purpose: it makes the `WxH` half of the
/// stdout line assert something real. A square page would pass even if
/// width and height were transposed somewhere in the geometry chain.
fn multipage_pdf(contents: &[&str]) -> Vec<u8> {
    // Object numbering: 1 = catalog, 2 = page-tree root,
    // then per page i: page dict at 3+2i, content stream at 4+2i.
    let kids: Vec<String> = (0..contents.len())
        .map(|i| format!("{} 0 R", 3 + 2 * i))
        .collect();
    let mut objects: Vec<(u32, String)> = vec![
        (1, "<< /Type /Catalog /Pages 2 0 R >>".into()),
        (
            2,
            format!(
                "<< /Type /Pages /Kids [{}] /Count {} /MediaBox [0 0 200 100] \
                 /Resources << /Font << /F1 << /Type /Font /Subtype /Type1 \
                 /BaseFont /Helvetica >> >> >> >>",
                kids.join(" "),
                contents.len()
            ),
        ),
    ];
    for (i, content) in contents.iter().enumerate() {
        let page_num = 3 + 2 * i as u32;
        let stream_num = page_num + 1;
        objects.push((
            page_num,
            format!("<< /Type /Page /Parent 2 0 R /Contents {stream_num} 0 R >>"),
        ));
        objects.push((
            stream_num,
            format!(
                "<< /Length {} >>\nstream\n{content}\nendstream",
                content.len()
            ),
        ));
    }
    build_pdf(&objects)
}

// ---------------------------------------------------------------------------
// Temp-directory scaffolding
// ---------------------------------------------------------------------------

/// A uniquely named directory under the system temp dir, removed on drop.
///
/// Uniqueness is process id + nanosecond clock + a per-process counter:
/// the pid separates concurrent `cargo test` invocations, the counter
/// separates tests within one process (Rust runs them on parallel
/// threads, so two could otherwise read the same clock tick), and the
/// clock separates sequential runs that reuse a pid.
struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.subsec_nanos());
        let path = std::env::temp_dir().join(format!(
            "pdfce-cli-test-{tag}-{}-{}-{nanos}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).expect("could not create temp dir");
        Self(path)
    }

    fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    /// Write `bytes` to `name` inside this directory and return the path.
    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let p = self.join(name);
        std::fs::write(&p, bytes).expect("could not write fixture");
        p
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        // Best-effort: a failure here must not mask the test's own
        // failure, so the result is deliberately discarded.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

// ---------------------------------------------------------------------------
// Process helpers
// ---------------------------------------------------------------------------

/// Run `pdfce-cli` with `args` and capture the whole process outcome.
fn run(args: &[&str]) -> Output {
    Command::new(BIN)
        .args(args)
        .output()
        .expect("could not spawn pdfce-cli")
}

/// Exit code as `u8`, matching the [`exit`] table's own type. A process
/// killed by a signal (no code) fails the test loudly rather than
/// silently comparing against a default.
fn code(out: &Output) -> i32 {
    out.status
        .code()
        .expect("pdfce-cli terminated without an exit code (signal?)")
}

fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).expect("stdout must be valid UTF-8")
}

fn stderr(out: &Output) -> String {
    String::from_utf8(out.stderr.clone()).expect("stderr must be valid UTF-8")
}

/// The eight-byte PNG signature (RFC 2083 §3.1 / W3C PNG §5.2). Checking
/// it proves the file is a PNG and not, say, a zero-length file left
/// behind by a failed write — without pulling in an image-decoding
/// dependency just to assert "yes, that is a PNG".
const PNG_MAGIC: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1A, b'\n'];

/// Assert `path` exists and begins with the PNG signature, and return its
/// declared width and height read from the IHDR chunk.
///
/// IHDR is required by the format to be the **first** chunk, so its
/// dimensions live at fixed offsets 16..20 (width) and 20..24 (height),
/// big-endian — a stable enough guarantee to check without a decoder.
fn png_dimensions(path: &Path) -> (u32, u32) {
    let bytes = std::fs::read(path).expect("output PNG was not written");
    assert!(
        bytes.starts_with(&PNG_MAGIC),
        "output file is not a PNG (first bytes: {:?})",
        &bytes[..bytes.len().min(8)]
    );
    let w = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let h = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    (w, h)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn renders_a_single_page_to_png_with_the_stable_stdout_line() {
    let dir = TempDir::new("ok");
    let pdf = dir.write("one.pdf", &multipage_pdf(&["0 0 0 rg 10 10 50 50 re f"]));
    let png = dir.join("out.png");

    let out = run(&[
        "render-page",
        pdf.to_str().unwrap(),
        "-o",
        png.to_str().unwrap(),
    ]);

    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));

    // The page is 200x100 user units; the default scale is 1.0, so the
    // raster is 200x100 device pixels. Asserting BOTH the stdout line and
    // the PNG's own IHDR catches a mismatch between what the CLI reports
    // and what it actually wrote.
    assert_eq!(png_dimensions(&png), (200, 100));

    let line = stdout(&out);
    assert!(
        line.ends_with('\n') && line.matches('\n').count() == 1,
        "stdout must be exactly one LF-terminated line, got {line:?}"
    );
    assert!(
        line.starts_with("rendered "),
        "unexpected stdout prefix: {line:?}"
    );
    assert!(
        line.contains(" page 1 -> "),
        "stdout must name the 1-based page: {line:?}"
    );
    assert!(
        line.contains(" 200x100; "),
        "stdout must carry WxH: {line:?}"
    );

    // The metrics half: everything after the first "; " parses as
    // key=integer pairs in the documented order. This is the exact
    // parsing recipe the module docs promise a script can use.
    let metrics = line.trim_end().split("; ").nth(1).expect("metrics half");
    let keys: Vec<&str> = metrics
        .split(' ')
        .map(|kv| kv.split('=').next().unwrap())
        .collect();
    assert_eq!(
        keys,
        [
            "substituted",
            "notdef",
            "unsupported",
            "unknown",
            "deferred",
            // Appended by the XObject/image slice. The contract permits
            // APPENDING keys; the five above never move.
            "images",
            "images_unsupported",
            "forms",
            // Appended by Pass 2.1's image-codec slice (decision 005
            // §6.4). Same rule again: appended at the END, and every
            // key above keeps its meaning and its position.
            "images_codec_unsupported",
            "codec_features",
            "codec_geometry_mismatch",
            "dct_cmyk",
            "lzw_anomalies",
            // Appended by decision 006's diagnostic split: the benign
            // YCCK census stayed in `dct_cmyk` (same key, now verified
            // neutral) and the R30 polarity-unverifiable shape got its
            // own key, appended at the END per the contract.
            "dct_cmyk_unverifiable",
            // Appended by Pass 2.3's JPXDecode slice: Table 89's
            // /SMaskInData 2, where the codestream's colour channels
            // arrive preblended with a backdrop. Appended at the END,
            // same contract.
            "jpx_preblended",
            // Appended by Pass 6.0's annotation-appearance slice
            // (docs/decisions/008). Eight keys, appended at the END, same
            // contract — every key above keeps its meaning and position.
            // `annots_no_ap` is a SUM of the per-subtype
            // `annotations_without_ap` map (the per-subtype breakdown is a
            // stderr note); `need_appearances` is the document-scoped
            // /AcroForm /NeedAppearances disclosure (R51).
            "annots",
            "annots_painted",
            "annots_no_ap",
            "annots_hidden",
            "annots_state_missing",
            "annots_widget",
            "annots_degenerate",
            "need_appearances",
            // Appended by the font-diagnostics by-reason split: the
            // per-reason breakdown of `unsupported`
            // (`fonts_unsupported_by_reason`). Six keys, appended at the
            // END, same contract — their sum equals `unsupported`, and
            // every key above keeps its meaning and position.
            "unsupported_type3",
            "unsupported_noncmap",
            "unsupported_vertical",
            "unsupported_composite_not_embedded",
            "unsupported_unknown_subtype",
            "unsupported_unusable_program",
            // Appended by decision 012's supplied-fonts slice: `supplied`
            // is glyphs drawn from an operator-supplied `--font-dir` face
            // (the third trust level, R62); `supplied_registered` is the
            // count of name→file registrations the walk added. Appended at
            // the END, same contract — every key above keeps its meaning
            // and position.
            "supplied",
            "supplied_registered",
            // Appended by the /Contents-degradation slice: `/Contents`
            // entries this page named that are not present in the file, so
            // their marks are missing from the raster (ISO 32000-1 §7.3.10
            // makes such a reference the null object; Table 30 makes an
            // absent /Contents an empty page — the document opens, but the
            // page is incomplete and says so). Appended at the END, same
            // contract — every key above keeps its meaning and position.
            "contents_unresolved",
        ],
        "metrics key order is part of the stable contract"
    );
    for kv in metrics.split(' ') {
        let (_, v) = kv.split_once('=').expect("key=value");
        v.parse::<u64>()
            .unwrap_or_else(|_| panic!("metric value must be a non-negative integer: {kv:?}"));
    }

    // A path drawing with no text at all is a fully faithful render, so
    // every counter is zero and stderr stays silent — the property that
    // makes "stderr had output" a usable batch signal.
    assert!(
        metrics.contains("substituted=0")
            && metrics.contains("unsupported=0")
            && metrics.contains("unknown=0"),
        "clean render should report zeros: {metrics:?}"
    );
    assert_eq!(stderr(&out), "", "a clean render must not write to stderr");
}

#[test]
fn r20_counters_disclose_a_substituted_font_on_stdout_and_stderr() {
    // Decision 004 rule R20: an operator must be able to tell, WITHOUT
    // reading the code, that these letterforms are pdfce's bundled
    // substitute rather than the document's own. Helvetica is declared
    // with no embedded program, so every glyph is substituted — and the
    // count has to reach stdout (machine) and the font name has to reach
    // stderr (human).
    let dir = TempDir::new("r20");
    let pdf = dir.write(
        "text.pdf",
        &multipage_pdf(&["BT /F1 24 Tf 10 40 Td (Hi) Tj ET"]),
    );
    let png = dir.join("text.png");

    let out = run(&[
        "render-page",
        pdf.to_str().unwrap(),
        "-o",
        png.to_str().unwrap(),
    ]);

    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    let line = stdout(&out);
    assert!(
        !line.contains("substituted=0"),
        "R20: substituted glyphs must be counted on stdout: {line:?}"
    );
    let err = stderr(&out);
    assert!(
        err.contains("Helvetica"),
        "R20: the substituted face must be NAMED on stderr: {err:?}"
    );
}

#[test]
fn scale_multiplies_the_raster_size() {
    let dir = TempDir::new("scale");
    let pdf = dir.write("one.pdf", &multipage_pdf(&["0 0 0 rg 0 0 10 10 re f"]));
    let png = dir.join("big.png");

    let out = run(&[
        "render-page",
        pdf.to_str().unwrap(),
        "--scale",
        "2",
        "-o",
        png.to_str().unwrap(),
    ]);

    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert_eq!(png_dimensions(&png), (400, 200));
    assert!(stdout(&out).contains(" 400x200; "));
}

#[test]
fn page_flag_selects_the_right_page_and_defaults_to_one() {
    // Page 2 is 90-degree-free but has a distinguishing content stream;
    // what is actually under test is that `--page 2` reaches the second
    // element of the flattened page vector and that omitting the flag
    // reaches the first.
    let dir = TempDir::new("pages");
    let pdf = dir.write(
        "three.pdf",
        &multipage_pdf(&[
            "0 0 0 rg 0 0 10 10 re f",
            "0 0 0 rg 0 0 20 20 re f",
            "0 0 0 rg 0 0 30 30 re f",
        ]),
    );

    for (flag, expected) in [(Some("2"), 2u32), (Some("3"), 3), (None, 1)] {
        let png = dir.join(&format!("p{expected}.png"));
        let mut args = vec!["render-page", pdf.to_str().unwrap()];
        if let Some(f) = flag {
            args.extend(["--page", f]);
        }
        args.extend(["-o", png.to_str().unwrap()]);

        let out = run(&args);
        assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
        assert!(
            stdout(&out).contains(&format!(" page {expected} -> ")),
            "stdout must echo the page actually rendered: {}",
            stdout(&out)
        );
        assert_eq!(png_dimensions(&png), (200, 100));
    }
}

#[test]
fn page_out_of_range_is_a_clear_runtime_failure_at_both_ends() {
    let dir = TempDir::new("range");
    let pdf = dir.write("one.pdf", &multipage_pdf(&["0 0 0 rg 0 0 10 10 re f"]));
    let png = dir.join("nope.png");

    // Past the end, and the 0 case — the module docs commit to both
    // producing exit 1, not clap's usage exit 2.
    for page in ["2", "0"] {
        let out = run(&[
            "render-page",
            pdf.to_str().unwrap(),
            "--page",
            page,
            "-o",
            png.to_str().unwrap(),
        ]);
        assert_eq!(code(&out), 1, "page {page}: stderr: {}", stderr(&out));
        let err = stderr(&out);
        assert!(
            err.contains("out of range") && err.contains("1 page(s)"),
            "the message must name the real page count: {err:?}"
        );
        assert_eq!(stdout(&out), "", "a failure must print no result line");
        assert!(!png.exists(), "a failure must not leave an output file");
    }
}

#[test]
fn missing_input_is_exit_3_and_a_non_pdf_is_exit_4() {
    let dir = TempDir::new("errs");
    let png = dir.join("never.png");

    let out = run(&[
        "render-page",
        dir.join("absent.pdf").to_str().unwrap(),
        "-o",
        png.to_str().unwrap(),
    ]);
    assert_eq!(code(&out), 3, "missing input is an I/O failure");
    assert_eq!(stdout(&out), "");

    let junk = dir.write("junk.bin", b"GIF89a this is not a PDF at all\n");
    let out = run(&[
        "render-page",
        junk.to_str().unwrap(),
        "-o",
        png.to_str().unwrap(),
    ]);
    assert_eq!(code(&out), 4, "a non-PDF is exit 4, not a generic failure");
    assert!(stderr(&out).contains("not a PDF"), "{}", stderr(&out));
    assert_eq!(stdout(&out), "");
}

#[test]
fn capability_gap_refusal_is_honest_and_distinguishable_from_corruption() {
    // pdfce-core deliberately REFUSES structures it cannot yet handle
    // correctly, rather than misparsing them. The CLI must pass that
    // refusal through verbatim so the operator learns "pdfce can't open
    // this *yet*", not "your file is broken" — the same honesty the GUI
    // owes on its error surface.
    //
    // The live case is an ENCRYPTED document (§7.6): pdfce has no
    // security handler, and every layer downstream would decode
    // ciphertext into plausible-looking garbage. (Cross-reference
    // streams used to be the case pinned here; they now load — see
    // `a_pdf_15_file_with_xref_and_object_streams_renders` below.)
    let dir = TempDir::new("encrypted");
    let mut buf = b"%PDF-1.4\n".to_vec();
    let xref_at = buf.len();
    buf.extend_from_slice(b"xref\n0 1\n0000000000 65535 f\r\n");
    buf.extend_from_slice(
        format!("trailer\n<< /Size 1 /Root 1 0 R /Encrypt 2 0 R >>\nstartxref\n{xref_at}\n%%EOF\n")
            .as_bytes(),
    );
    let pdf = dir.write("encrypted.pdf", &buf);

    let out = run(&[
        "render-page",
        pdf.to_str().unwrap(),
        "-o",
        dir.join("never.png").to_str().unwrap(),
    ]);

    assert_eq!(code(&out), 1, "an unsupported structure is a runtime error");
    let err = stderr(&out);
    assert!(
        err.contains("not yet supported"),
        "the refusal must read as a pdfce limitation, not a broken file: {err:?}"
    );
    assert_eq!(stdout(&out), "");
}

#[test]
fn a_pdf_15_file_with_xref_and_object_streams_renders() {
    // The complement of the test above: what used to be refused now
    // goes all the way through load -> page tree -> render. The catalog,
    // page tree and page object all live inside an object stream
    // (§7.5.7) and are reached by type-2 cross-reference entries
    // (§7.5.8.3), so this exercises the whole PDF 1.5 structural path
    // end to end through the shipped binary.
    let dir = TempDir::new("pdf15");
    let pdf = dir.write("pdf15.pdf", &build_pdf15_objstm_pdf());
    let png = dir.join("out.png");

    let out = run(&[
        "render-page",
        pdf.to_str().unwrap(),
        "-o",
        png.to_str().unwrap(),
    ]);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert!(png.is_file(), "a PNG must have been written");
}

/// Build a PDF 1.5 file whose cross-reference section is a stream and
/// whose document structure lives inside an object stream.
///
/// Layout: objects 1 (catalog), 2 (page tree) and 3 (the page) are
/// compressed into object stream 4; object 5 is the page's content
/// stream (a *stream*, so §7.5.7 forbids compressing it); object 6 is
/// the cross-reference stream, which `startxref` points at directly
/// (§7.5.8.1).
fn build_pdf15_objstm_pdf() -> Vec<u8> {
    let compressed: [(u32, &str); 3] = [
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << >> /Contents 5 0 R >>",
        ),
    ];
    // §7.5.7 decoded layout: `N` `objnum offset` pairs (offsets
    // relative to `/First`), then the bare object values at `/First` —
    // no `obj`/`endobj` framing.
    let mut header = String::new();
    let mut body = String::new();
    for (num, text) in compressed {
        header.push_str(&format!("{num} {} ", body.len()));
        body.push_str(text);
        body.push(' ');
    }
    let first = header.len();
    let objstm_data = format!("{header}{body}");

    let content = "0 0 1 rg 20 20 160 60 re f\n";

    let mut buf = b"%PDF-1.5\n".to_vec();
    let mut offsets: Vec<(u32, usize)> = Vec::new();

    offsets.push((4, buf.len()));
    buf.extend_from_slice(
        format!(
            "4 0 obj\n<< /Type /ObjStm /N {} /First {first} /Length {} >>\nstream\n{objstm_data}\nendstream\nendobj\n",
            compressed.len(),
            objstm_data.len(),
        )
        .as_bytes(),
    );

    offsets.push((5, buf.len()));
    buf.extend_from_slice(
        format!(
            "5 0 obj\n<< /Length {} >>\nstream\n{content}endstream\nendobj\n",
            content.len(),
        )
        .as_bytes(),
    );

    // The cross-reference stream itself (object 6). `/W [1 4 2]`:
    // 1-byte type, 4-byte field 2, 2-byte field 3, all big-endian
    // (§7.5.8.3).
    let xref_at = buf.len();
    offsets.push((6, xref_at));
    let mut rows: Vec<u8> = Vec::new();
    let push = |rows: &mut Vec<u8>, ty: u8, f2: u32, f3: u16| {
        rows.push(ty);
        rows.extend(f2.to_be_bytes());
        rows.extend(f3.to_be_bytes());
    };
    // Object 0 is permanently the free-list head (§7.5.4).
    push(&mut rows, 0, 0, 65535);
    for num in 1..=6u32 {
        match offsets.iter().find(|(n, _)| *n == num) {
            Some((_, off)) => push(&mut rows, 1, u32::try_from(*off).unwrap(), 0),
            // Objects 1-3 are compressed in container 4, at indices
            // 0/1/2 respectively — type-2 entries (Table 18).
            None => push(&mut rows, 2, 4, u16::try_from(num - 1).unwrap()),
        }
    }
    buf.extend_from_slice(
        format!(
            "6 0 obj\n<< /Type /XRef /Size 7 /W [1 4 2] /Root 1 0 R /Length {} >>\nstream\n",
            rows.len(),
        )
        .as_bytes(),
    );
    buf.extend_from_slice(&rows);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    buf.extend_from_slice(format!("startxref\n{xref_at}\n%%EOF\n").as_bytes());
    buf
}
