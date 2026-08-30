//! # The non-JavaScript button actions (`Pass 183.0`)
//!
//! Operator ruling, 2026-08-30, hours after the one that produced
//! `Pass 182.0`: *"make the submit and other options that don't need
//! javascript available for buttons with the safeguards like we had
//! planned."*
//!
//! `Pass 182.0` moved decision 009's posture A exactly one notch and authored
//! `/ResetForm` and nothing else. This Pass takes the rest of the
//! script-free action set — `/SubmitForm`, `/GoTo`, `/Named`, `/URI` — and
//! the "safeguards like we had planned" are a written plan
//! (`docs/plan-scripting-submit-and-plugins.md` §6), not a general
//! instruction, so they resolve to specific testable things.
//!
//! ## What is being pinned, and why each one is the half that goes wrong
//!
//! **The bytes.**
//!
//! - `/F` is a **Filespec dictionary with `/FS /URL`**, never a bare string.
//!   A bare string is a *file-system path* by §7.11.2 and the standard states
//!   no reader rule for one on a submit (`SF-A1`) — so the ambiguous form is
//!   the one a plausible implementation writes and this one refuses to.
//! - `/Flags` is written **even when it is 0**, because 0 means FDF-by-POST:
//!   a decision the standard makes, not an absence of one.
//! - The four formats map to exactly one bit each, and the four spellings
//!   must produce four different documents.
//! - `/GoTo`'s `/D` names its page by **indirect reference**, which is what
//!   makes it survive a reorder without anything rewriting it.
//!
//! **The disclosure**, which is the whole safeguard available at authoring
//! time. Every assertion here is about something an operator cannot see by
//! any other means:
//!
//! - hidden-widget fields submit exactly like visible ones, because `Hidden`
//!   is an **annotation** flag and every submit selector addresses **field**
//!   dictionaries — the two are simply on different objects;
//! - `Password` values are submitted; the flag's NOTE constrains storage;
//! - a `FileSelect` field carries **the contents of a local file** off the
//!   machine;
//! - the baseline FDF payload carries the source document's **own path**;
//! - `NoExport` is applied **last**, with precedence over an explicit name —
//!   an implementation that applies it earlier exfiltrates a field its author
//!   marked non-exportable, silently.
//!
//! **The refusals**, all before any write: a destination pdfce cannot state
//! (relative, non-ASCII), a Table 237 gate the type system could not close,
//! a page index past the end, a submit target that does not exist.
//!
//! ## And one census that went from complete to under-reporting
//!
//! `census_dangling` walked **link** annotations only, which was correct
//! until a push button could carry a `/GoTo`. The last test here is the one
//! that would have caught that, and it is in this file rather than a page-ops
//! one because this Pass is what broke it.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use pdfce_core::document::Document;
use pdfce_core::edit::{
    ButtonAction, EditError, EditSession, FdfOptions, NamedAction, PageView, SubmitFormat,
    SubmitScope, SubmitSpec,
};
use pdfce_core::writer::SaveOptions;

/// A two-page form carrying one of every field shape the disclosure has to
/// notice.
///
/// The flag words are spelled as literals with the bit named beside them so a
/// reader can check them against Table 228 without leaving the file:
/// `Required` = 2, `NoExport` = 4, `Password` = 8192, `FileSelect` = 1048576,
/// `Pushbutton` = 65536. The `Hidden` case is deliberately **not** a field
/// flag — it is `/F 2` on the widget, which is exactly why it is invisible to
/// a selector that reads field dictionaries.
fn form_with_every_field_shape() -> Vec<u8> {
    let content = "BT /Helv 12 Tf 60 700 Td (form) Tj ET\n";
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields \
         [6 0 R 7 0 R 8 0 R 9 0 R 10 0 R 11 0 R 12 0 R 13 0 R 15 0 R 17 0 R 20 0 R] \
         /DA (/Helv 0 Tf 0 g) /DR << /Font << /Helv 14 0 R >> >> >> >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 750] /Resources \
         << /Font << /Helv 14 0 R >> >> /Contents 5 0 R /Annots \
         [6 0 R 7 0 R 8 0 R 9 0 R 10 0 R 11 0 R 12 0 R 13 0 R 16 0 R 18 0 R 19 0 R] >>"
            .to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 750] /CropBox [10 20 290 700] \
         /Resources << >> >>"
            .to_owned(),
        format!(
            "<< /Length {} >>\nstream\n{content}endstream",
            content.len()
        ),
        // 6 — an ordinary filled text field.
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (Name) /V (typed) /DV (factory) \
         /Rect [20 700 200 720] /P 3 0 R /F 4 /DA (/Helv 12 Tf 0 g) >>"
            .to_owned(),
        // 7 — Password (bit 14 = 8192). Its value submits like any other.
        "<< /Type /Annot /Subtype /Widget /FT /Tx /Ff 8192 /T (Secret) /V (hunter2) \
         /Rect [20 660 200 680] /P 3 0 R /F 4 /DA (/Helv 12 Tf 0 g) >>"
            .to_owned(),
        // 8 — a HIDDEN widget (/F 2). Not a field flag; that is the point.
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (Tracker) /V (campaign-42) \
         /Rect [20 620 200 640] /P 3 0 R /F 2 /DA (/Helv 12 Tf 0 g) >>"
            .to_owned(),
        // 9 — FileSelect (bit 21 = 1048576): its text names a LOCAL FILE.
        "<< /Type /Annot /Subtype /Widget /FT /Tx /Ff 1048576 /T (Attach) /V (C:/notes.txt) \
         /Rect [20 580 200 600] /P 3 0 R /F 4 /DA (/Helv 12 Tf 0 g) >>"
            .to_owned(),
        // 10 — NoExport (bit 3 = 4): vetoes inclusion, with precedence.
        "<< /Type /Annot /Subtype /Widget /FT /Tx /Ff 4 /T (Private) /V (kept) \
         /Rect [20 540 200 560] /P 3 0 R /F 4 /DA (/Helv 12 Tf 0 g) >>"
            .to_owned(),
        // 11 — no /V at all.
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (Empty) \
         /Rect [20 500 200 520] /P 3 0 R /F 4 /DA (/Helv 12 Tf 0 g) >>"
            .to_owned(),
        // 12 — Required (bit 2 = 2) and still empty at submit time.
        "<< /Type /Annot /Subtype /Widget /FT /Tx /Ff 2 /T (Signature) \
         /Rect [20 460 200 480] /P 3 0 R /F 4 /DA (/Helv 12 Tf 0 g) >>"
            .to_owned(),
        // 13 — the push button this Pass gives an action to.
        "<< /Type /Annot /Subtype /Widget /FT /Btn /Ff 65536 /T (Go) \
         /Rect [20 400 100 425] /P 3 0 R /F 4 /MK << /CA (Go) >> >>"
            .to_owned(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
            .to_owned(),
        // 15 - a NON-TERMINAL grouping node. `/Hide` must refuse it: Table 210
        // states no descendant rule, unlike /ResetForm's and /SubmitForm's
        // /Fields flag rows.
        "<< /T (Group) /Kids [16 0 R] >>".to_owned(),
        // 16 - the terminal beneath it, so the refusal above can be shown to be
        // about the node's KIND rather than about the dotted name.
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (Inner) /Parent 15 0 R /Rect [220 700 280 720] /P 3 0 R /F 4 /DA (/Helv 12 Tf 0 g) >>".to_owned(),
        // 17 - ONE field, TWO widgets. A hide action names the field and moves
        // both appearances (Table 210: "widget annotation OR ANNOTATIONS"),
        // which is what makes a widget count different from a name count.
        "<< /FT /Tx /T (Twin) /V (t) /Kids [18 0 R 19 0 R] >>".to_owned(),
        "<< /Type /Annot /Subtype /Widget /Parent 17 0 R /Rect [220 660 280 680] /P 3 0 R /F 4 >>".to_owned(),
        "<< /Type /Annot /Subtype /Widget /Parent 17 0 R /Rect [220 620 280 640] /P 3 0 R /F 4 >>".to_owned(),
        // 20 - a terminal field with NO widget at all. Legal, and a hide action
        // naming it does nothing; the standard states no reader rule, so pdfce
        // authors it and discloses instead of refusing.
        "<< /FT /Tx /T (Nameless) /V (n) >>".to_owned(),
    ];
    let mut buf = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in bodies.iter().enumerate() {
        offsets.push(buf.len());
        buf.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", i + 1).as_bytes());
    }
    let xref_at = buf.len();
    let size = bodies.len() + 1;
    buf.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for off in &offsets {
        buf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n")
            .as_bytes(),
    );
    buf
}

fn session() -> EditSession {
    EditSession::new(Document::from_bytes(form_with_every_field_shape()).unwrap())
}

fn saved(s: &EditSession) -> String {
    let bytes = s.to_incremental_bytes(&SaveOptions::identity()).unwrap().0;
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Author a submit with the given spec and return the appended bytes.
fn submit_bytes(spec: SubmitSpec) -> String {
    let mut s = session();
    s.set_button_action("Go", Some(ButtonAction::SubmitForm(spec)))
        .expect("authors");
    saved(&s)
}

/// **The destination is a Filespec dictionary, never a bare string.**
///
/// §13.3's minimum conformant object. A bare `/F (http://…)` cannot declare
/// `/FS /URL`, so by §7.11.2 it is a file-system path — and the standard
/// states no reader rule for that case. The assertion is on `/FS /URL` being
/// present, because the failure this catches is the *convenient* spelling.
#[test]
fn a_submit_destination_is_a_url_filespec_dictionary() {
    let text = submit_bytes(SubmitSpec::new("https://example.com/collect"));
    assert!(text.contains("/S /SubmitForm"), "the action is written");
    assert!(
        text.contains("/FS /URL"),
        "the destination must declare the URL file system: {text}"
    );
    assert!(
        text.contains("/Type /Filespec"),
        "…as a file specification dictionary: {text}"
    );
    assert!(text.contains("https://example.com/collect"));
}

/// **`/Flags` is written even when it is zero.**
///
/// Table 236 defaults it to 0 and 0 means *FDF, POST, every field with a
/// value*. Omitting it would leave a reader to infer a payload choice that
/// was actually made. The assertion is on the presence of a key whose value
/// is the default — the shape most implementations skip.
#[test]
fn the_flag_word_is_written_explicitly_even_at_zero() {
    let text = submit_bytes(SubmitSpec::new("https://example.com/x"));
    assert!(
        text.contains("/Flags 0"),
        "a zero flag word is a decision and is written: {text}"
    );
}

/// **Each format sets exactly one selector bit, and the four differ.**
///
/// `ExportFormat` 4 · `XFDF` 32 · `SubmitPDF` 256 · FDF = none of them. Tested
/// together because the failure worth catching is not "a bit is wrong" but
/// "two formats produce the same document".
#[test]
fn the_four_formats_map_to_four_different_flag_words() {
    let mut fdf = SubmitSpec::new("https://e.com/x");
    fdf.format = SubmitFormat::Fdf(FdfOptions::default());
    let mut html = SubmitSpec::new("https://e.com/x");
    html.format = SubmitFormat::Html {
        get: false,
        coordinates: false,
    };
    let mut get = SubmitSpec::new("https://e.com/x");
    get.format = SubmitFormat::Html {
        get: true,
        coordinates: false,
    };
    let mut xfdf = SubmitSpec::new("https://e.com/x");
    xfdf.format = SubmitFormat::Xfdf;
    let mut pdf = SubmitSpec::new("https://e.com/x");
    pdf.format = SubmitFormat::WholeDocument;

    assert!(submit_bytes(fdf).contains("/Flags 0"));
    assert!(submit_bytes(html).contains("/Flags 4"));
    // 4 | 8 — GetMethod rides only with ExportFormat, which is why it cannot
    // be spelled on any other variant.
    assert!(submit_bytes(get).contains("/Flags 12"));
    assert!(submit_bytes(xfdf).contains("/Flags 32"));
    assert!(submit_bytes(pdf).contains("/Flags 256"));
}

/// **`Except` sets bit 1 and `Only` does not.**
///
/// One integer apart, and it inverts the meaning: clear ⇒ the array says what
/// to include, set ⇒ what to exclude. Sabotaging that single line makes a
/// button submit precisely the fields it was meant to withhold.
#[test]
fn only_and_except_differ_by_the_include_exclude_bit() {
    let mut only = SubmitSpec::new("https://e.com/x");
    only.scope = SubmitScope::Only(vec!["Name".to_owned()]);
    let mut except = SubmitSpec::new("https://e.com/x");
    except.scope = SubmitScope::Except(vec!["Name".to_owned()]);

    let only_text = submit_bytes(only);
    let except_text = submit_bytes(except);
    assert!(only_text.contains("/Flags 0"), "{only_text}");
    assert!(except_text.contains("/Flags 1"), "{except_text}");
    assert!(only_text.contains("/Fields"));
    assert!(except_text.contains("/Fields"));
}

/// **The disclosure names the three things an operator cannot see.**
///
/// A hidden widget's value, a password value and a local file, all leaving on
/// a baseline submit nobody configured. This is the feature's justification;
/// if this test ever passes vacuously the fixture has stopped carrying those
/// fields.
#[test]
fn the_disclosure_names_what_the_operator_cannot_see() {
    let mut s = session();
    let change = s
        .set_button_action(
            "Go",
            Some(ButtonAction::SubmitForm(SubmitSpec::new(
                "https://example.com/collect",
            ))),
        )
        .expect("authors");
    let d = change.submit.expect("a submit discloses");

    assert_eq!(d.hidden_fields, vec!["Tracker".to_owned()]);
    assert_eq!(d.password_fields, vec!["Secret".to_owned()]);
    assert_eq!(d.file_select_fields, vec!["Attach".to_owned()]);
    assert!(
        d.includes_document_path,
        "the baseline FDF payload carries this document's own path"
    );
    assert!(d.fields.contains(&"Tracker".to_owned()));
    assert!(d.fields.contains(&"Secret".to_owned()));

    let summary = d.summary();
    assert!(summary.contains("HIDDEN"), "{summary}");
    assert!(summary.contains("LOCAL FILE"), "{summary}");
    assert!(
        summary.contains("example.com/collect"),
        "the summary names the FULL destination, which Acrobat's own warning does not: {summary}"
    );
}

/// **`NoExport` is applied LAST, and beats an explicit name.**
///
/// Table 236 gives it precedence over the array *and* the flag. So a button
/// that names `Private` explicitly still does not send it — and the operator
/// is told, because a silent veto surprises as much as a silent inclusion.
///
/// The ordering is invisible in the common case; this test is the one that
/// sees it.
#[test]
fn no_export_beats_an_explicit_include() {
    let mut spec = SubmitSpec::new("https://e.com/x");
    spec.scope = SubmitScope::Only(vec!["Private".to_owned(), "Name".to_owned()]);
    let mut s = session();
    let d = s
        .set_button_action("Go", Some(ButtonAction::SubmitForm(spec)))
        .expect("authors")
        .submit
        .expect("discloses");

    assert_eq!(d.fields, vec!["Name".to_owned()]);
    assert_eq!(d.excluded_by_no_export, vec!["Private".to_owned()]);
}

/// **A push button rides only when `/Fields` is present.**
///
/// §12.7.5.2: *"If the submit-form action dictionary contains no `Fields`
/// entry, such pushbutton fields shall not be submitted."* So `All` and
/// `Only`-naming-everything are **not** the same document, which is the trap
/// worth a test of its own.
#[test]
fn push_buttons_ride_only_when_fields_is_present() {
    let mut s = session();
    let all = s
        .set_button_action(
            "Go",
            Some(ButtonAction::SubmitForm(SubmitSpec::new("https://e.com/x"))),
        )
        .expect("authors")
        .submit
        .expect("discloses");
    assert!(
        !all.fields.contains(&"Go".to_owned()),
        "omitting /Fields excludes push buttons by a `shall`: {:?}",
        all.fields
    );

    let mut named = SubmitSpec::new("https://e.com/x");
    named.scope = SubmitScope::Only(vec!["Go".to_owned(), "Name".to_owned()]);
    let mut s2 = session();
    let listed = s2
        .set_button_action("Go", Some(ButtonAction::SubmitForm(named)))
        .expect("authors")
        .submit
        .expect("discloses");
    assert!(
        listed.fields.contains(&"Go".to_owned()),
        "naming it pulls it back in, with its /AP as the value: {:?}",
        listed.fields
    );
}

/// **Empty fields ride only when asked, and are reported separately.**
///
/// `IncludeNoValueFields` sends form *structure* rather than data, so the two
/// lists are distinct: `fields` says what is transmitted, `valueless_fields`
/// says which of those carry a name and nothing else.
#[test]
fn valueless_fields_ride_only_when_asked() {
    let mut s = session();
    let default = s
        .set_button_action(
            "Go",
            Some(ButtonAction::SubmitForm(SubmitSpec::new("https://e.com/x"))),
        )
        .expect("authors")
        .submit
        .expect("discloses");
    assert!(!default.fields.contains(&"Empty".to_owned()));
    assert!(default.valueless_fields.is_empty());

    let mut spec = SubmitSpec::new("https://e.com/x");
    spec.include_no_value_fields = true;
    let mut s2 = session();
    let widened = s2
        .set_button_action("Go", Some(ButtonAction::SubmitForm(spec)))
        .expect("authors")
        .submit
        .expect("discloses");
    assert!(widened.fields.contains(&"Empty".to_owned()));
    assert!(widened.valueless_fields.contains(&"Empty".to_owned()));
    assert!(
        widened
            .required_without_value
            .contains(&"Signature".to_owned()),
        "Required is a SUBMIT-time obligation the standard states with no consequence"
    );
}

/// **A whole-document submit stops counting fields and says the categorical
/// thing.**
///
/// Bit 9 ignores `/Fields` entirely, so a field count would be a lie in the
/// reassuring direction — "only six values leave" about a submit that sends
/// the file.
#[test]
fn a_whole_document_submit_is_categorical_not_counted() {
    let mut spec = SubmitSpec::new("https://e.com/x");
    spec.format = SubmitFormat::WholeDocument;
    let mut s = session();
    let d = s
        .set_button_action("Go", Some(ButtonAction::SubmitForm(spec)))
        .expect("authors")
        .submit
        .expect("discloses");

    assert!(d.whole_document);
    assert!(d.fields.is_empty());
    assert!(d.hidden_fields.is_empty());
    let summary = d.summary();
    assert!(summary.contains("ENTIRE document"), "{summary}");
    assert!(
        !summary.contains("field value"),
        "a count here would understate what leaves: {summary}"
    );
}

/// **An unencrypted destination is allowed and SAID, never blocked.**
///
/// Destination policy is open by operator ruling. The standard has nothing to
/// say either — `https` appears zero times in ISO 32000-1 — so refusing
/// `http` would be pdfce inventing a conformance requirement. Disclose,
/// don't block.
#[test]
fn an_unencrypted_destination_is_allowed_and_disclosed() {
    let mut s = session();
    let d = s
        .set_button_action(
            "Go",
            Some(ButtonAction::SubmitForm(SubmitSpec::new(
                "http://plain.example/collect",
            ))),
        )
        .expect("authors — the policy is open");
    let d = d.submit.expect("discloses");
    assert_eq!(d.scheme, "http");
    assert!(!d.encrypted);
    assert!(d.summary().contains("UNENCRYPTED"), "{}", d.summary());
}

/// **A destination pdfce cannot state is refused, and nothing is written.**
///
/// Relative and non-ASCII, on both carriers that take a destination. Not a
/// whitelist: no host is refused anywhere in this Pass. The complaint is
/// decidability — a relative destination resolves against the document's own
/// location, or against `/Base` under a rule readers disagree about.
#[test]
fn an_undecidable_destination_is_refused_before_any_write() {
    for bad in ["collect.cgi", "/cgi/collect", "https://exämple.com/x", "  "] {
        let mut s = session();
        let err = s
            .set_button_action("Go", Some(ButtonAction::SubmitForm(SubmitSpec::new(bad))))
            .expect_err("refused");
        assert!(
            matches!(err, EditError::ButtonActionDestination { .. }),
            "{bad:?} gave {err:?}"
        );
        assert!(
            !saved(&s).contains("/S /SubmitForm"),
            "{bad:?} must leave the document untouched"
        );

        let mut s2 = session();
        let err = s2
            .set_button_action(
                "Go",
                Some(ButtonAction::Uri {
                    uri: bad.to_owned(),
                }),
            )
            .expect_err("refused");
        assert!(matches!(err, EditError::ButtonActionDestination { .. }));
    }
}

/// **A submit target that does not exist is refused, like a reset target.**
#[test]
fn an_unknown_submit_target_is_refused() {
    let mut spec = SubmitSpec::new("https://e.com/x");
    spec.scope = SubmitScope::Only(vec!["NoSuchField".to_owned()]);
    let mut s = session();
    let err = s
        .set_button_action("Go", Some(ButtonAction::SubmitForm(spec)))
        .expect_err("refused");
    assert!(matches!(err, EditError::FieldNotFound { .. }));
}

/// **The one Table 237 gate a type could not close is closed by a refusal.**
///
/// `ExclNonUserAnnots` (bit 11) narrows `IncludeAnnotations` (bit 8) and
/// *"shall be used only when"* that flag is set. The standard states the
/// constraint and states **no reader behaviour for violating it**, so the
/// file would be non-conforming with no defined outcome — which is worse than
/// a refusal, not better.
#[test]
fn a_flag_gate_the_type_could_not_close_is_refused_by_name() {
    let mut opts = FdfOptions::default();
    opts.only_current_user_annotations = true;
    let mut spec = SubmitSpec::new("https://e.com/x");
    spec.format = SubmitFormat::Fdf(opts);

    let mut s = session();
    let err = s
        .set_button_action("Go", Some(ButtonAction::SubmitForm(spec)))
        .expect_err("refused");
    assert!(matches!(err, EditError::ButtonActionSubmitFlags { .. }));

    // …and the same word with its companion set is fine.
    let mut ok = FdfOptions::default();
    ok.only_current_user_annotations = true;
    ok.include_annotations = true;
    let mut spec = SubmitSpec::new("https://e.com/x");
    spec.format = SubmitFormat::Fdf(ok);
    let mut s2 = session();
    let d = s2
        .set_button_action("Go", Some(ButtonAction::SubmitForm(spec)))
        .expect("authors")
        .submit
        .expect("discloses");
    assert!(d.includes_annotations);
    // 128 | 1024
    assert!(saved(&s2).contains("/Flags 1152"));
}

/// **A `/GoTo` names its page by indirect reference, and lands where asked.**
///
/// Table 151: *"`page` is an indirect reference to a page object"*. That is
/// what makes the destination survive a reorder with nothing rewriting it —
/// and the `/FitH` parameter is taken from the target page's **crop box**,
/// which is why the fixture's second page has one that differs from its media
/// box. A `700` here rather than `750` is the evidence the box was read.
#[test]
fn a_goto_names_its_page_by_reference_and_reads_its_crop_box() {
    let mut s = session();
    s.set_button_action(
        "Go",
        Some(ButtonAction::GoToPage {
            page_index: 1,
            view: PageView::FullWidth,
        }),
    )
    .expect("authors");
    let text = saved(&s);
    assert!(text.contains("/S /GoTo"), "{text}");
    assert!(
        text.contains("4 0 R /FitH 700.0"),
        "the page is a reference and the top is the CROP box's, not the media box's: {text}"
    );

    let mut s2 = session();
    s2.set_button_action(
        "Go",
        Some(ButtonAction::GoToPage {
            page_index: 1,
            view: PageView::TopLeft,
        }),
    )
    .expect("authors");
    assert!(
        saved(&s2).contains("/XYZ 10.0 700.0 null"),
        "a null zoom is Table 151's `retain unchanged`: {}",
        saved(&s2)
    );
}

/// **A page index past the end is refused, and nothing is written.**
#[test]
fn a_goto_past_the_end_is_refused() {
    let mut s = session();
    let err = s
        .set_button_action(
            "Go",
            Some(ButtonAction::GoToPage {
                page_index: 9,
                view: PageView::WholePage,
            }),
        )
        .expect_err("refused");
    assert!(matches!(
        err,
        EditError::PageOutOfRange { index: 9, count: 2 }
    ));
    assert!(!saved(&s).contains("/S /GoTo"));
}

/// **A named action writes the standard's own spelling.**
///
/// Table 211 defines exactly four, and an unrecognised name is the one place
/// the standard tells a reader to *"take no action"* — so a typo here is a
/// button that silently does nothing.
#[test]
fn a_named_action_writes_the_table_211_spelling() {
    for (action, spelling) in [
        (NamedAction::NextPage, "/N /NextPage"),
        (NamedAction::PrevPage, "/N /PrevPage"),
        (NamedAction::FirstPage, "/N /FirstPage"),
        (NamedAction::LastPage, "/N /LastPage"),
    ] {
        let mut s = session();
        s.set_button_action("Go", Some(ButtonAction::Named(action)))
            .expect("authors");
        let text = saved(&s);
        assert!(text.contains("/S /Named"), "{text}");
        assert!(text.contains(spelling), "{text}");
    }
}

/// **A `/URI` is authored as a plain string, not a file specification.**
///
/// Two destinations in adjacent clauses with different encodings: a submit's
/// `/F` is a Filespec dictionary, a URI action's `/URI` is a string. Writing
/// one like the other is the plausible mistake.
#[test]
fn a_uri_action_is_a_string_not_a_filespec() {
    let mut s = session();
    s.set_button_action(
        "Go",
        Some(ButtonAction::Uri {
            uri: "https://example.com/help".to_owned(),
        }),
    )
    .expect("authors");
    let text = saved(&s);
    assert!(text.contains("/S /URI"), "{text}");
    assert!(text.contains("/URI (https://example.com/help)"), "{text}");
    assert!(
        !text.contains("/FS /URL"),
        "a URI action carries no file specification: {text}"
    );
}

/// **Every new action is one undoable command, and undo is byte-identical.**
///
/// The same property `Pass 182.0` pinned for reset, re-asserted per variant
/// rather than assumed to generalise — four write paths, four chances for one
/// of them to touch an object it did not record.
#[test]
fn each_action_is_one_undoable_command() {
    let actions = [
        ButtonAction::SubmitForm(SubmitSpec::new("https://e.com/x")),
        ButtonAction::GoToPage {
            page_index: 0,
            view: PageView::WholePage,
        },
        ButtonAction::Named(NamedAction::LastPage),
        ButtonAction::Uri {
            uri: "https://e.com/help".to_owned(),
        },
    ];
    for action in actions {
        let mut s = session();
        let before = saved(&s);
        s.set_button_action("Go", Some(action.clone()))
            .expect("set");
        assert_ne!(saved(&s), before, "{action:?} changed nothing");
        s.undo().expect("undo");
        assert_eq!(saved(&s), before, "{action:?} did not undo cleanly");
    }
}

/// **A button's `/GoTo` counts as dangling when its page is deleted.**
///
/// ★ The census this Pass would otherwise have quietly broken.
///
/// `census_dangling` walked **link** annotations only, and that was complete
/// until a push button could carry a `/GoTo`. Adding the authoring half
/// without this would have left the counter reporting zero for a button that
/// stopped working — and an under-reporting counter reads exactly like a
/// clean bill of health, which is the shape this project keeps meeting.
///
/// The assertion is on the NEW field being 1 and `links` staying 0, because
/// folding the count into `links` would have made the test pass while the
/// operator sentence stayed wrong.
#[test]
fn a_deleted_page_breaks_a_button_action_and_the_census_says_so() {
    let mut s = session();
    s.set_button_action(
        "Go",
        Some(ButtonAction::GoToPage {
            page_index: 1,
            view: PageView::WholePage,
        }),
    )
    .expect("authors");
    let outcome = s.delete_pages(&[1]).expect("deletes");
    assert_eq!(
        outcome.dangling.non_link_annotations, 1,
        "the button's destination is gone and the census must say so"
    );
    assert_eq!(outcome.dangling.links, 0, "there are no link annotations");
    assert!(!outcome.dangling.is_empty());
}

// ---------------------------------------------------------------------------
// `/Hide` — `Pass 183.1`, ISO 32000-1 §12.6.4.10 Table 210
//
// Table 210 has exactly three rows (`/S`, `/T`, `/H`) and two of them carry a
// trap a plausible implementation walks into:
//
//   * `/H`'s DEFAULT IS TRUE. Omitting it authors a HIDE. The "absent means
//     off" reflex therefore ships a Show button that hides.
//   * `/T` has NO DESCENDANT RULE. The phrase "all descendants of the
//     specified fields" appears twice per edition and both times on a
//     `/Fields` row — never here — so a grouping name is undefined rather
//     than "obviously the subtree".
// ---------------------------------------------------------------------------

/// **`/H` is written in BOTH directions, and `false` is the one that matters.**
///
/// The failure this catches is not a wrong value; it is an ABSENT key. An
/// implementation that writes `/H` only when hiding produces a Show button
/// that hides, because the default is `true` — and the file is perfectly
/// conforming, so nothing else would flag it.
#[test]
fn the_hide_flag_is_written_explicitly_in_both_directions() {
    for (hidden, expected) in [(true, "/H true"), (false, "/H false")] {
        let mut s = session();
        s.set_button_action(
            "Go",
            Some(ButtonAction::SetHidden {
                targets: vec!["Name".to_owned()],
                hidden,
            }),
        )
        .expect("authors");
        let text = saved(&s);
        assert!(text.contains("/S /Hide"), "{text}");
        assert!(text.contains(expected), "expected {expected} in: {text}");
        assert!(
            text.contains("/T (Name)"),
            "a single target is a string: {text}"
        );
    }
}

/// **Several targets become an array; one stays a bare string.**
///
/// Table 210 permits either. One target is written the way the clause states
/// it first and the way real producers emit it; a one-element array would be
/// equally legal and needlessly unlike every other file.
#[test]
fn several_hide_targets_become_an_array() {
    let mut s = session();
    s.set_button_action(
        "Go",
        Some(ButtonAction::SetHidden {
            targets: vec!["Name".to_owned(), "Secret".to_owned()],
            hidden: true,
        }),
    )
    .expect("authors");
    let text = saved(&s);
    assert!(text.contains("/T [(Name) (Secret)]"), "{text}");
}

/// **A grouping node is refused rather than expanded.**
///
/// `/ResetForm` and `/SubmitForm` state descendant expansion in their flag
/// rows; Table 210 states nothing. The two available readings — hide the
/// subtree, or hide nothing — differ by everything, and one of them produces
/// a button that silently does nothing. So pdfce refuses instead of guessing.
///
/// The fixture's `Group.Inner` gives a real grouping node to name.
#[test]
fn a_grouping_node_is_refused_because_table_210_has_no_descendant_rule() {
    let mut s = session();
    let err = s
        .set_button_action(
            "Go",
            Some(ButtonAction::SetHidden {
                targets: vec!["Group".to_owned()],
                hidden: true,
            }),
        )
        .expect_err("refused");
    assert!(
        matches!(err, EditError::ButtonActionHideTargetNotTerminal { .. }),
        "{err:?}"
    );
    assert!(!saved(&s).contains("/S /Hide"));

    // …and the terminal beneath it is accepted, so the refusal is about the
    // node's KIND and not about the dotted name.
    let mut s2 = session();
    s2.set_button_action(
        "Go",
        Some(ButtonAction::SetHidden {
            targets: vec!["Group.Inner".to_owned()],
            hidden: true,
        }),
    )
    .expect("a terminal is fine");
    assert!(saved(&s2).contains("/T (Group.Inner)"));
}

/// **No targets, and an unknown target, are both refused before any write.**
#[test]
fn an_empty_or_unknown_hide_target_is_refused() {
    let mut s = session();
    let err = s
        .set_button_action(
            "Go",
            Some(ButtonAction::SetHidden {
                targets: Vec::new(),
                hidden: true,
            }),
        )
        .expect_err("refused");
    assert!(
        matches!(err, EditError::ButtonActionHideNoTargets),
        "{err:?}"
    );

    let mut s2 = session();
    let err = s2
        .set_button_action(
            "Go",
            Some(ButtonAction::SetHidden {
                targets: vec!["NoSuchField".to_owned()],
                hidden: true,
            }),
        )
        .expect_err("refused");
    assert!(matches!(err, EditError::FieldNotFound { .. }), "{err:?}");
    assert!(!saved(&s2).contains("/S /Hide"));
}

/// **The disclosure counts WIDGETS, not names, and says which names move
/// nothing.**
///
/// Table 210 affects *"the associated widget annotation or annotations"* — so
/// one name can move several appearances, on pages the operator was not
/// looking at. And a field with no widget at all is authored anyway (the
/// standard states no reader rule, so refusing would be pdfce inventing one)
/// with the button doing nothing for that name.
#[test]
fn the_hide_disclosure_counts_widgets_and_names_the_ones_that_move_nothing() {
    let mut s = session();
    let d = s
        .set_button_action(
            "Go",
            Some(ButtonAction::SetHidden {
                targets: vec!["Twin".to_owned(), "Nameless".to_owned()],
                hidden: false,
            }),
        )
        .expect("authors")
        .hide
        .expect("a hide discloses");

    assert!(d.shows, "`hidden: false` is a SHOW");
    assert_eq!(
        d.widgets_affected, 2,
        "Twin owns two widgets and Nameless owns none"
    );
    assert_eq!(d.targets_without_widgets, vec!["Nameless".to_owned()]);
    assert_eq!(d.targets.len(), 2);
}

/// **A hide action moves no hazard count.**
///
/// `/T` names annotations in this document; there is no file specification,
/// no URL and no script. Asserted rather than assumed, because the counter
/// that would be wrong here is the one an operator asks *"is this document
/// going to phone home?"*.
#[test]
fn authoring_a_hide_reaches_nothing() {
    let mut s = session();
    s.set_button_action(
        "Go",
        Some(ButtonAction::SetHidden {
            targets: vec!["Name".to_owned()],
            hidden: true,
        }),
    )
    .expect("authors");
    let bytes = s.to_incremental_bytes(&SaveOptions::identity()).unwrap().0;
    let doc = Document::from_bytes(bytes).unwrap();
    let js = pdfce_core::forms::scan_javascript(&doc);
    assert_eq!(js.network_action_count, 0);
    assert_eq!(js.launch_action_count, 0);
    assert_eq!(js.javascript_actions, 0);
    assert_eq!(
        js.annotation_actions, 1,
        "it IS counted as an annotation action"
    );
}

/// **Renaming a field says that button actions were not retargeted**
/// (`Pass 184.0` criterion A).
///
/// ★ pdfce writes reset, submit and hide targets as **name strings**, by its
/// own deliberate choice, because a name survives renumbering and copying
/// where an indirect reference does not. **A rename is the one operation that
/// breaks that choice**, and pdfce does not repair it.
///
/// It also **cannot be caught by `census_dangling`**: a name string leaves no
/// dangling object reference, so the graph census that `Pass 183.0` widened
/// to non-link annotations is structurally blind to this. Two different
/// invisibilities; that Pass fixed one of them.
///
/// So the disclosure is categorical rather than counted — the honest thing
/// available without a full carrier traversal — and this test pins that it is
/// **made at all**, and that it is `0` when there is genuinely nothing to
/// warn about.
#[test]
fn a_rename_discloses_that_button_actions_were_not_retargeted() {
    // A document with an action: a Reset button naming the field about to be
    // renamed.
    let mut s = session();
    s.set_button_action(
        "Go",
        Some(ButtonAction::ResetForm {
            scope: pdfce_core::edit::ResetScope::Only(vec!["Name".to_owned()]),
        }),
    )
    .expect("authors");
    let renamed = s.rename_field("Name", "Renamed").expect("renames");
    assert_eq!(renamed.to, "Renamed");
    assert!(
        renamed.actions_not_retargeted >= 1,
        "the document has an action and pdfce updated none of them"
    );

    // …and a document with no actions at all has nothing to warn about, so the
    // disclosure is not noise on every rename.
    let mut clean = session();
    let quiet = clean.rename_field("Name", "Renamed").expect("renames");
    assert_eq!(
        quiet.actions_not_retargeted, 0,
        "no actions in the file means nothing this rename could have broken"
    );
}
