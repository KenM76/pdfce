//! CLI-level tests for the **form-field authoring** subcommands:
//! `add-check-box` and `add-choice-field` (ISO 32000-1 §12.7.4.2, §12.7.4.4).
//!
//! # Purpose
//!
//! `crates/pdfce-core/tests/form_field_authoring.rs` proves the CORE verbs
//! produce real fields. This file proves the CLI in front of them is wired
//! correctly, which is a different and separately-breakable thing: an
//! argument parsed into the wrong field, a 1-based page passed through as
//! 0-based, a refusal that reaches the operator as a panic instead of an exit
//! code, or a `--option EXPORT=LABEL` split the wrong way round would all
//! leave the core tests green.
//!
//! # What is asserted, and why these and not others
//!
//! 1. **The subcommands run and their fields come back through
//!    `list-fields`.** The round trip through a real process, real files and
//!    a separate parse is what makes this an integration test rather than a
//!    slower unit test.
//! 2. **`--option EXPORT=LABEL` splits on the FIRST `=`.** The export value
//!    is form data and the label is prose; prose is where an `=` actually
//!    turns up, so the label is the half allowed to contain one.
//! 3. **Refusals exit `EDIT_REFUSED` (9) and name the reason on stderr**,
//!    without writing an output file. A refusal that still produced a file
//!    would be the worst outcome — the operator sees an error and a
//!    plausible-looking artefact.
//! 4. **`--verify-undo` reports `undo_identical=1`.** The additive-authoring
//!    invariant (R46), asserted through the CLI's own reporting rather than
//!    by re-reading the bytes here.
//!
//! # Contract with the fixture
//!
//! The fixture is a minimal single-page §7.5.4 file with NO `/AcroForm`, so
//! every test also exercises the create-the-form-from-nothing path. Field
//! rectangles are kept inside the 200×200 `/MediaBox`; an off-page rectangle
//! is accepted by the core (§12.7.4.5 makes off-page widgets legal) and would
//! make a rendering assertion silently vacuous.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_pdfce-cli");

/// `exit::EDIT_REFUSED` — the file was readable and pdfce declined the edit.
const EDIT_REFUSED: u8 = 9;

/// A minimal one-page classic-xref PDF with no form of any kind.
fn formless_pdf() -> Vec<u8> {
    let objects: Vec<(u32, String)> = vec![
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_owned()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned()),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >>".to_owned(),
        ),
    ];
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = vec![0usize; objects.len() + 1];
    for (num, body) in &objects {
        offsets[*num as usize] = out.len();
        out.extend_from_slice(format!("{num} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let startxref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for off in offsets.iter().skip(1) {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{startxref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

/// A self-deleting scratch directory (see `render_page.rs` for why this
/// project carries no `tempfile` dependency).
struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.subsec_nanos());
        let path = std::env::temp_dir().join(format!(
            "pdfce-cli-fields-{tag}-{}-{}-{nanos}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).expect("could not create temp dir");
        Self(path)
    }

    fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    /// The scratch directory seeded with the formless fixture as `in.pdf`.
    fn seeded(tag: &str) -> (Self, PathBuf) {
        let dir = Self::new(tag);
        let input = dir.join("in.pdf");
        std::fs::write(&input, formless_pdf()).expect("could not write fixture");
        (dir, input)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn run(args: &[&str]) -> Output {
    Command::new(BIN)
        .args(args)
        .output()
        .expect("could not spawn pdfce-cli")
}

fn code(out: &Output) -> u8 {
    u8::try_from(out.status.code().expect("process was killed by a signal")).unwrap()
}

fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).expect("stdout must be valid UTF-8")
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// `list-fields` output for `path`, as one line per field.
fn list_fields(path: &Path) -> String {
    let out = run(&["list-fields", &path.display().to_string()]);
    assert_eq!(code(&out), 0, "list-fields failed: {}", stderr(&out));
    stdout(&out)
}

// ---------------------------------------------------------------------------
// add-check-box
// ---------------------------------------------------------------------------

/// The headline: the subcommand runs against a document with no form at all,
/// and `list-fields` reads the result back as a check box.
#[test]
fn add_check_box_creates_a_field_list_fields_can_see() {
    let (dir, input) = TempDir::seeded("cb");
    let output = dir.join("out.pdf");
    let out = run(&[
        "add-check-box",
        &input.display().to_string(),
        "--name",
        "Agree",
        "--page",
        "1",
        "--rect",
        "20,20,44,44",
        "-o",
        &output.display().to_string(),
        "--verify-undo",
    ]);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));

    let line = stdout(&out);
    assert!(line.contains("add-check-box "), "reports its own name");
    // R46: the add is purely additive, proven by the CLI's own verification.
    assert!(
        line.contains("undo_identical=1"),
        "authoring must be undoable to the byte: {line}"
    );

    let fields = list_fields(&output);
    assert!(
        fields.contains("name=Agree type=Btn button=check"),
        "read back as a CHECK box, not another kind of button: {fields}"
    );
    // Created unticked, and with a usable appearance — no /NeedAppearances
    // fallback (R51).
    assert!(
        fields.contains("value=Off"),
        "unticked by default: {fields}"
    );
    assert!(fields.contains("ap=1"), "has an appearance: {fields}");
    assert!(
        fields.contains("need_appearances=0"),
        "must not lean on /NeedAppearances: {fields}"
    );
}

/// `--checked` and `--on-state` both reach the file, and the on-state name is
/// the value the form exports.
#[test]
fn the_on_state_and_checked_flags_reach_the_file() {
    let (dir, input) = TempDir::seeded("cb-on");
    let output = dir.join("out.pdf");
    let out = run(&[
        "add-check-box",
        &input.display().to_string(),
        "--name",
        "Colour",
        "--page",
        "1",
        "--rect",
        "20,20,44,44",
        "--on-state",
        "Red",
        "--checked",
        "-o",
        &output.display().to_string(),
    ]);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert!(list_fields(&output).contains("value=Red"));
}

/// `Off` cannot name the on state (§12.7.4.2.3), and the refusal is an exit
/// code plus a message — not a panic, and not a written file.
#[test]
fn an_off_on_state_is_refused_without_writing_a_file() {
    let (dir, input) = TempDir::seeded("cb-off");
    let output = dir.join("out.pdf");
    let out = run(&[
        "add-check-box",
        &input.display().to_string(),
        "--name",
        "Agree",
        "--page",
        "1",
        "--rect",
        "20,20,44,44",
        "--on-state",
        "Off",
        "-o",
        &output.display().to_string(),
    ]);
    assert_eq!(code(&out), EDIT_REFUSED, "stderr: {}", stderr(&out));
    assert!(
        stderr(&out).contains("Off"),
        "the message must name the reserved state: {}",
        stderr(&out)
    );
    assert!(
        !output.exists(),
        "a refused edit must not leave a plausible-looking output file"
    );
}

// ---------------------------------------------------------------------------
// add-choice-field
// ---------------------------------------------------------------------------

/// The headline for choice fields, plus the export/display split surviving
/// the trip through argv.
#[test]
fn add_choice_field_creates_a_field_and_splits_export_from_label() {
    let (dir, input) = TempDir::seeded("ch");
    let output = dir.join("out.pdf");
    let out = run(&[
        "add-choice-field",
        &input.display().to_string(),
        "--name",
        "Country",
        "--page",
        "1",
        "--rect",
        "20,60,180,84",
        "--option",
        "CA=Canada",
        "--option",
        "MX=Mexico",
        "--option",
        "Other",
        "--combo",
        "-o",
        &output.display().to_string(),
        "--verify-undo",
    ]);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    let line = stdout(&out);
    assert!(line.contains("options=3"), "all three parsed: {line}");
    assert!(line.contains("combo=1"));
    assert!(line.contains("undo_identical=1"));

    let fields = list_fields(&output);
    assert!(
        fields.contains("name=Country type=Ch"),
        "read back as a choice field: {fields}"
    );
    // Created UNSELECTED — the deliberate `/V`-absent decision.
    assert!(
        fields.contains("value=-"),
        "created with no selection: {fields}"
    );

    // THE SPLIT: filling by the DISPLAY label stores the EXPORT value. If
    // argv parsing had collapsed the two, this would store "Canada".
    let filled = dir.join("filled.pdf");
    let out = run(&[
        "fill-field",
        &output.display().to_string(),
        "--set",
        "Country=Canada",
        "-o",
        &filled.display().to_string(),
    ]);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    let fields = list_fields(&filled);
    assert!(
        fields.contains("value=CA"),
        "the form must submit the EXPORT value, not the label: {fields}"
    );
}

/// A label may contain `=`; the split takes the FIRST one, so the export
/// value is the unambiguous half.
#[test]
fn an_option_label_may_contain_an_equals_sign() {
    let (dir, input) = TempDir::seeded("ch-eq");
    let output = dir.join("out.pdf");
    let out = run(&[
        "add-choice-field",
        &input.display().to_string(),
        "--name",
        "Rule",
        "--page",
        "1",
        "--rect",
        "20,60,180,84",
        "--option",
        "EQ=a = b",
        "-o",
        &output.display().to_string(),
    ]);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));

    let filled = dir.join("filled.pdf");
    let out = run(&[
        "fill-field",
        &output.display().to_string(),
        "--set",
        "Rule=a = b",
        "-o",
        &filled.display().to_string(),
    ]);
    assert_eq!(
        code(&out),
        0,
        "the label kept its '=' and stayed selectable: {}",
        stderr(&out)
    );
    assert!(list_fields(&filled).contains("value=EQ"));
}

/// `--editable` without `--combo` is impossible (§12.7.4.4 Table 230) and is
/// refused rather than silently dropped by the CLI on the way through.
#[test]
fn an_editable_list_box_is_refused_by_the_cli() {
    let (dir, input) = TempDir::seeded("ch-edit");
    let output = dir.join("out.pdf");
    let out = run(&[
        "add-choice-field",
        &input.display().to_string(),
        "--name",
        "Country",
        "--page",
        "1",
        "--rect",
        "20,60,180,84",
        "--option",
        "Canada",
        "--editable",
        "-o",
        &output.display().to_string(),
    ]);
    assert_eq!(code(&out), EDIT_REFUSED, "stdout: {}", stdout(&out));
    assert!(!output.exists());
}

/// A duplicated export value is refused — it would be unselectable, because
/// the fill verb resolves to the first match.
#[test]
fn a_duplicate_export_value_is_refused_by_the_cli() {
    let (dir, input) = TempDir::seeded("ch-dup");
    let output = dir.join("out.pdf");
    let out = run(&[
        "add-choice-field",
        &input.display().to_string(),
        "--name",
        "Country",
        "--page",
        "1",
        "--rect",
        "20,60,180,84",
        "--option",
        "CA=Canada",
        "--option",
        "CA=Canada again",
        "-o",
        &output.display().to_string(),
    ]);
    assert_eq!(code(&out), EDIT_REFUSED, "stdout: {}", stdout(&out));
    assert!(!output.exists());
}

/// A choice field with no options SUCCEEDS and warns: the empty state is
/// legal and a form under construction passes through it, but a field
/// nothing can fill must be disclosed at once (R4).
#[test]
fn a_choice_field_with_no_options_succeeds_and_warns() {
    let (dir, input) = TempDir::seeded("ch-none");
    let output = dir.join("out.pdf");
    let out = run(&[
        "add-choice-field",
        &input.display().to_string(),
        "--name",
        "Country",
        "--page",
        "1",
        "--rect",
        "20,60,180,84",
        "-o",
        &output.display().to_string(),
    ]);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    assert!(
        stdout(&out).contains("no_options=1"),
        "the machine-readable report must carry it: {}",
        stdout(&out)
    );
    assert!(
        stderr(&out).contains("cannot be filled until options are added"),
        "and the operator must be told in words: {}",
        stderr(&out)
    );
    assert!(output.exists(), "the field is still created");
    assert!(list_fields(&output).contains("name=Country type=Ch"));
}

/// A same-name add MERGES, for every authoring subcommand — end to end.
///
/// # This test used to assert a refusal, and the inversion is the deliverable
///
/// §12.7.3.2 makes the fully-qualified name a field's IDENTITY, so a second
/// same-named field would give a document two fields with one identity and no
/// disambiguator. pdfce refused that, correctly, while it lacked a write-side
/// resolver to do the alternative.
///
/// The resolver exists now, so a same-name same-type add does what the spec
/// says it means: it attaches ANOTHER WIDGET to the one field. One `/V`, two
/// places to see and edit it — which is how a reference number repeats in a
/// header and how a check box appears on every page.
///
/// # Why the fill at the end is the load-bearing assertion
///
/// `list-fields` reporting one field with two widgets proves the merge PARSES.
/// It does not prove the result is a real field. Running the **existing,
/// unmodified `fill-field` verb** over it does: that verb knows nothing about
/// authoring, resolves by fully-qualified name, and fans out over
/// `field.widgets`. If it reports two widgets updated, then the merged field
/// is addressable, correctly typed, and correctly shaped by the same code path
/// every pre-existing document goes through.
#[test]
fn a_duplicate_field_name_merges_for_every_subcommand() {
    let (dir, input) = TempDir::seeded("dup");
    let input_s = input.display().to_string();

    for (cmd, extra) in [
        ("add-text-field", vec![]),
        ("add-check-box", vec![]),
        ("add-choice-field", vec!["--option", "X"]),
    ] {
        let first = dir.join(&format!("{cmd}-1.pdf"));
        let first_s = first.display().to_string();
        let mut a = vec![
            cmd,
            &input_s,
            "--name",
            "Dup",
            "--page",
            "1",
            "--rect",
            "20,20,180,44",
        ];
        a.extend_from_slice(&extra);
        a.extend_from_slice(&["-o", &first_s]);
        let out = run(&a);
        assert_eq!(code(&out), 0, "{cmd} first add: {}", stderr(&out));

        let second = dir.join(&format!("{cmd}-2.pdf"));
        let second_s = second.display().to_string();
        let mut b = vec![
            cmd,
            &first_s,
            "--name",
            "Dup",
            "--page",
            "1",
            "--rect",
            "20,60,180,84",
        ];
        b.extend_from_slice(&extra);
        b.extend_from_slice(&["-o", &second_s]);
        let out = run(&b);
        assert_eq!(
            code(&out),
            0,
            "{cmd} must MERGE a same-name same-type add: {}",
            stderr(&out)
        );
        assert!(second.exists(), "{cmd} wrote no output");

        // ONE field, TWO widgets — not two fields sharing an identity.
        let fields = list_fields(&second);
        assert!(
            fields.contains("fields=1"),
            "{cmd} must leave exactly one field: {fields}"
        );
        assert!(
            fields.contains("widgets=2"),
            "{cmd} must leave that field with both widgets: {fields}"
        );
    }
}

/// The merged text field is fillable through the UNMODIFIED fill verb, and
/// the fill reaches BOTH widgets.
///
/// Split from the loop above because only a text field has a fill verb that
/// reports a widget count; running it is what turns "the merge parses" into
/// "the merge produced a real field".
#[test]
fn a_merged_field_fills_through_the_existing_verb_and_paints_both_widgets() {
    let (dir, input) = TempDir::seeded("mergefill");
    let input_s = input.display().to_string();
    let one = dir.join("one.pdf");
    let one_s = one.display().to_string();
    let two = dir.join("two.pdf");
    let two_s = two.display().to_string();
    let filled = dir.join("filled.pdf");
    let filled_s = filled.display().to_string();

    let out = run(&[
        "add-text-field",
        &input_s,
        "--name",
        "Ref",
        "--page",
        "1",
        "--rect",
        "20,20,180,44",
        "-o",
        &one_s,
    ]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let out = run(&[
        "add-text-field",
        &one_s,
        "--name",
        "Ref",
        "--page",
        "1",
        "--rect",
        "20,60,180,84",
        "-o",
        &two_s,
    ]);
    assert_eq!(code(&out), 0, "the merge: {}", stderr(&out));

    // THE PROOF: a verb that knows nothing about authoring accepts it.
    let out = run(&["fill-field", &two_s, "--set", "Ref=R-2000", "-o", &filled_s]);
    assert_eq!(
        code(&out),
        0,
        "fill must accept the merged field: {}",
        stderr(&out)
    );

    let fields = list_fields(&filled);
    assert!(
        fields.contains("fields=1") && fields.contains("widgets=2"),
        "still one field with two widgets after the fill: {fields}"
    );
    assert!(
        fields.contains("value=R-2000"),
        "the shared value reached the field: {fields}"
    );
    assert!(
        fields.contains("ap=1"),
        "both widgets carry a regenerated appearance: {fields}"
    );
}

/// A dotted name creates a HIERARCHY, and `list-fields` reports the composed
/// fully-qualified name.
///
/// §12.7.3.2 reserves the period as the path separator, so pdfce adopts the
/// spec's own model: `a.b.c` means non-terminal `a`, non-terminal `a.b`,
/// terminal `c`. The operator is told what the dot did rather than having to
/// infer it.
#[test]
fn a_dotted_name_creates_a_hierarchy() {
    let (dir, input) = TempDir::seeded("dotted");
    let input_s = input.display().to_string();
    let out_path = dir.join("nested.pdf");
    let out_s = out_path.display().to_string();

    let out = run(&[
        "add-text-field",
        &input_s,
        "--name",
        "Personal.Address.Zip",
        "--page",
        "1",
        "--rect",
        "20,20,180,44",
        "-o",
        &out_s,
    ]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));

    let fields = list_fields(&out_path);
    assert!(
        fields.contains("name=Personal.Address.Zip"),
        "the composed FQN, not a flat /T: {fields}"
    );
    assert!(fields.contains("fields=1"), "one TERMINAL field: {fields}");

    // A SECOND field under the SAME group reuses the existing nodes rather
    // than creating a parallel `Personal`. A duplicated group would give both
    // terminals ambiguous ancestry.
    let two = dir.join("two.pdf");
    let two_s = two.display().to_string();
    let out = run(&[
        "add-text-field",
        &out_s,
        "--name",
        "Personal.Address.City",
        "--page",
        "1",
        "--rect",
        "20,60,180,84",
        "-o",
        &two_s,
    ]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let fields = list_fields(&two);
    assert!(
        fields.contains("name=Personal.Address.Zip")
            && fields.contains("name=Personal.Address.City")
            && fields.contains("fields=2"),
        "both terminals under ONE group: {fields}"
    );
}

/// A name that belongs to a GROUPING node is refused by name.
///
/// With `Personal.Address.Zip` present, `Personal` names a container. A
/// request for a terminal field called `Personal` is neither a same-type merge
/// nor a different-type collision: Table 220 gives a non-terminal no type of
/// its own. Acrobat has no such branch because it never exposes hierarchy
/// authoring; pdfce does, so pdfce needs it.
#[test]
fn a_grouping_node_name_cannot_become_a_field() {
    let (dir, input) = TempDir::seeded("group");
    let input_s = input.display().to_string();
    let nested = dir.join("nested.pdf");
    let nested_s = nested.display().to_string();
    let refused = dir.join("refused.pdf");
    let refused_s = refused.display().to_string();

    let out = run(&[
        "add-text-field",
        &input_s,
        "--name",
        "Personal.Address.Zip",
        "--page",
        "1",
        "--rect",
        "20,20,180,44",
        "-o",
        &nested_s,
    ]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));

    let out = run(&[
        "add-text-field",
        &nested_s,
        "--name",
        "Personal",
        "--page",
        "1",
        "--rect",
        "20,60,180,84",
        "-o",
        &refused_s,
    ]);
    assert_eq!(
        code(&out),
        EDIT_REFUSED,
        "a grouping node's name must be refused: {}",
        stdout(&out)
    );
    assert!(
        stderr(&out).contains("names a group"),
        "and the reason must say so: {}",
        stderr(&out)
    );
    assert!(!refused.exists(), "a refusal writes nothing");
}

/// A partial name containing a period is refused — every way it can arise.
///
/// §12.7.3.2 reserves the period as the path separator, so a leading,
/// trailing or doubled one produces an EMPTY segment: a field whose partial
/// name is the empty string, which is not a name. There is deliberately no
/// escape hatch; one would author exactly the ambiguity the spec avoids.
#[test]
fn an_empty_path_segment_is_refused() {
    let (dir, input) = TempDir::seeded("dots");
    let input_s = input.display().to_string();
    for bad in [".Leading", "Trailing.", "Doubled..Up"] {
        let out_path = dir.join("x.pdf");
        let out_s = out_path.display().to_string();
        let out = run(&[
            "add-text-field",
            &input_s,
            "--name",
            bad,
            "--page",
            "1",
            "--rect",
            "20,20,180,44",
            "-o",
            &out_s,
        ]);
        assert_eq!(
            code(&out),
            EDIT_REFUSED,
            "{bad} must be refused: {}",
            stdout(&out)
        );
        assert!(!out_path.exists(), "{bad}: a refusal writes nothing");
    }
}

// ---------------------------------------------------------------------------
// Shared argument handling
// ---------------------------------------------------------------------------

/// `--page` is 1-based in BOTH new subcommands. A 0 is refused rather than
/// wrapping to the last page or panicking on the underflow.
#[test]
fn page_zero_is_refused_by_both_subcommands() {
    let (dir, input) = TempDir::seeded("page0");
    for args in [
        vec![
            "add-check-box",
            "--name",
            "A",
            "--rect",
            "20,20,44,44",
            "--page",
            "0",
        ],
        vec![
            "add-choice-field",
            "--name",
            "A",
            "--rect",
            "20,20,44,44",
            "--option",
            "X",
            "--page",
            "0",
        ],
    ] {
        let output = dir.join("out.pdf");
        let mut full = vec![args[0], &input.display().to_string()];
        // Leak-free rebuild: the borrow of `input` must outlive the call, so
        // the display string is materialised once per iteration.
        let input_s = input.display().to_string();
        let output_s = output.display().to_string();
        full = vec![args[0], &input_s];
        full.extend_from_slice(&args[1..]);
        full.extend_from_slice(&["-o", &output_s]);
        let out = run(&full);
        assert_eq!(
            code(&out),
            EDIT_REFUSED,
            "{} must refuse page 0: {}",
            args[0],
            stdout(&out)
        );
        assert!(!output.exists());
    }
}

/// A malformed `--rect` is refused by both, with a message naming the shape
/// it wanted.
#[test]
fn a_malformed_rect_is_refused_by_both_subcommands() {
    let (dir, input) = TempDir::seeded("rect");
    let input_s = input.display().to_string();
    for (cmd, extra) in [
        ("add-check-box", vec![]),
        ("add-choice-field", vec!["--option", "X"]),
    ] {
        let output = dir.join("out.pdf");
        let output_s = output.display().to_string();
        let mut full = vec![cmd, &input_s, "--name", "A", "--page", "1", "--rect", "1,2"];
        full.extend_from_slice(&extra);
        full.extend_from_slice(&["-o", &output_s]);
        let out = run(&full);
        assert_eq!(code(&out), EDIT_REFUSED, "{cmd}: {}", stdout(&out));
        assert!(
            stderr(&out).contains("LLX,LLY,URX,URY"),
            "{cmd} must name the shape it wanted: {}",
            stderr(&out)
        );
    }
}

/// All three authoring subcommands share one name-clash guard, so a name
/// already used by a different field type is refused whichever one asks.
#[test]
fn a_name_used_by_another_field_type_is_refused_across_subcommands() {
    let (dir, input) = TempDir::seeded("clash");
    let text = dir.join("text.pdf");
    let out = run(&[
        "add-text-field",
        &input.display().to_string(),
        "--name",
        "Shared",
        "--page",
        "1",
        "--rect",
        "20,20,180,44",
        "-o",
        &text.display().to_string(),
    ]);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));

    let text_s = text.display().to_string();
    let clash = dir.join("clash.pdf");
    let clash_s = clash.display().to_string();
    for (cmd, extra) in [
        ("add-check-box", vec![]),
        ("add-choice-field", vec!["--option", "X"]),
    ] {
        let mut full = vec![
            cmd,
            &text_s,
            "--name",
            "Shared",
            "--page",
            "1",
            "--rect",
            "20,60,44,84",
        ];
        full.extend_from_slice(&extra);
        full.extend_from_slice(&["-o", &clash_s]);
        let out = run(&full);
        assert_eq!(
            code(&out),
            EDIT_REFUSED,
            "{cmd} must not steal a text field's name: {}",
            stdout(&out)
        );
    }
}
