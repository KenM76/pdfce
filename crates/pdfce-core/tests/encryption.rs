//! End-to-end decryption of encrypted documents — RC4 and AES-128
//! (ISO 32000-1 §7.6).
//!
//! # Why these tests are the ones that matter
//!
//! The unit tests in `crate::crypto` verify pieces: MD5 against RFC 1321,
//! RC4 against its published vectors, `/P` against the standard's own `-44`
//! example, the never-encrypted list against constructed dictionaries. Every
//! one of them can pass while the document still fails to open, because clause
//! 7.6's real hazard is not any single algorithm — it is **transposition
//! between two algorithms that look alike**:
//!
//! - Algorithm 2 step (h) runs 50 MD5 rounds truncating to `n` bytes each
//!   round; Algorithm 3 step (c) runs 50 MD5 rounds and does **not** truncate
//!   (**T9/T13**). Three pages apart, opposite rules.
//! - Algorithm 3's RC4 loop counts **1 → 19**; Algorithm 7's counts
//!   **19 → 0** (**T16**).
//! - `/P` is stored signed and hashed unsigned little-endian (**T10**).
//! - `/U`'s last 16 bytes are arbitrary at `/R` ≥ 3 and must not be compared
//!   (**T15**).
//!
//! Swap either 50-round loop for the other and every unit test still passes.
//! The only thing that catches it is a real file, made by an implementation
//! that is not ours, opening.
//!
//! # What agreement here proves, and what it does not
//!
//! The fixtures were produced by **pypdf**, chosen deliberately as an
//! independent implementation (`fixtures/synthetic/encryption/PROVENANCE.md`).
//! pdfce's decryption was written from the ISO 32000-1 clause text. So
//! agreement means two independent readings of the same published
//! specification agree — which is evidence.
//!
//! That reasoning is exactly why `enc-aes-256-r6.pdf` is **not** used as a
//! decryption fixture anywhere: `/R` 6's Algorithm 2.B is not sourced past
//! step (a), so an implementation derived from another implementation and then
//! tested against that same implementation's output could not fail. It appears
//! below only as a **refusal** fixture.
//!
//! Passwords: user `userpw`, owner `ownerpw`.

use std::path::{Path, PathBuf};

use pdfce_core::crypto::{AuthKind, EncryptionUnsupported};
use pdfce_core::document::{DocError, Document};
use pdfce_core::page_tree;
use pdfce_core::writer::{DirtySet, SaveOptions, WriteError};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/synthetic/encryption")
        .join(name)
}

/// `/V` 1, `/R` 2, 40-bit RC4 — the oldest configuration, and the one where
/// Algorithm 2's 50-round loop does **not** run at all and `n` is fixed at 5
/// regardless of `/Length`.
///
/// Opened with the **user** password.
#[test]
fn rc4_40_opens_with_the_user_password() {
    let doc = Document::load_with_password(&fixture("enc-rc4-40.pdf"), Some(b"userpw"))
        .expect("R2 RC4-40 must decrypt with the user password");

    let enc = doc.encryption().expect("the document is encrypted");
    assert_eq!(enc.config.revision, 2);
    assert_eq!(
        enc.config.key_len, 5,
        "R2 fixes n at 5 (Algorithm 2 step (i))"
    );
    assert_eq!(enc.auth, AuthKind::User);

    // The payoff: the page tree resolves, which it cannot do through
    // ciphertext.
    assert!(
        !page_tree::pages(&doc)
            .expect("page tree must resolve")
            .is_empty(),
        "pages must be reachable after decryption"
    );
}

/// `/V` 2, `/R` 3, 128-bit RC4 — this one **does** run Algorithm 2 step (h)'s
/// 50-round truncating loop, and compares only the first 16 bytes of `/U`.
/// If T9 or T15 were transposed, this test fails and the `/R` 2 one above
/// still passes.
#[test]
fn rc4_128_opens_with_the_user_password() {
    let doc = Document::load_with_password(&fixture("enc-rc4-128.pdf"), Some(b"userpw"))
        .expect("R3 RC4-128 must decrypt with the user password");

    let enc = doc.encryption().expect("the document is encrypted");
    assert_eq!(enc.config.revision, 3);
    assert_eq!(enc.config.key_len, 16);
    assert_eq!(enc.auth, AuthKind::User);
    assert!(
        !page_tree::pages(&doc)
            .expect("page tree must resolve")
            .is_empty()
    );
}

/// Algorithm 7 — the owner password opens the document too (§7.6.3.1:
/// "correctly supplying **either** password").
///
/// This is the only test that exercises the 19→0 loop (**T16**). Running it
/// 1→19 instead, or omitting the counter-0 round, fails here and nowhere else:
/// the user-password tests above never touch Algorithm 7 at all.
#[test]
fn owner_password_opens_both_revisions() {
    for (name, revision) in [("enc-rc4-40.pdf", 2u8), ("enc-rc4-128.pdf", 3)] {
        let doc = Document::load_with_password(&fixture(name), Some(b"ownerpw"))
            .unwrap_or_else(|e| panic!("{name} must open with the owner password: {e}"));
        let enc = doc.encryption().expect("encrypted");
        assert_eq!(enc.config.revision, revision, "{name}");
        assert_eq!(
            enc.auth,
            AuthKind::Owner,
            "{name}: the owner password must be reported as owner access, \
             not silently downgraded to user access"
        );
        assert!(
            !page_tree::pages(&doc)
                .expect("page tree must resolve")
                .is_empty(),
            "{name}"
        );
    }
}

/// ★ The empty user password — the single most operator-visible behaviour in
/// clause 7.6, and the one that decides whether pdfce looks broken.
///
/// §7.6.3.1 requires a reader to try the empty user password **first and
/// silently**, before any prompt. A document with an empty user password and a
/// non-empty owner password — the "permissions-only" PDF — therefore opens with
/// no dialog in every conforming reader. If pdfce prompted for it, the
/// operator's experience would be pdfce demanding a password for a file that
/// Chrome, Acrobat and every phone opens on a tap.
///
/// Note what `Document::load` is given here: **nothing**. No password argument,
/// no empty string, no flag. That is the point — the empty attempt is not
/// something a caller opts into.
///
/// This test could not exist until its fixture did. The corpus had exactly one
/// empty-user-password file and it was AES-128, which increment 1 refused on
/// cipher grounds *before authentication was ever reached* — so the
/// empty-password path was implemented, believed, and never once executed
/// end-to-end. A fixture that cannot fail for the reason you care about is not
/// covering that reason.
///
/// Increment 2 implemented AES-128, so that fixture now reaches authentication
/// too (`aes_128_with_an_empty_user_password_needs_no_password`). **Both stay.**
/// The RC4 file is the one that proved the path when AES could not, and
/// deleting it now would re-create the original hole the moment some future
/// increment changes how AES is handled.
#[test]
fn empty_user_password_opens_with_no_prompt() {
    let doc = Document::load(&fixture("enc-emptyuser-rc4-128.pdf"))
        .expect("a permissions-only document must open with no password at all");

    let enc = doc.encryption().expect("the document is still encrypted");
    assert_eq!(
        enc.auth,
        AuthKind::EmptyUser,
        "opening via the default password must be reported as such, not as a \
         user password the operator supplied"
    );
    assert!(
        !page_tree::pages(&doc)
            .expect("page tree must resolve")
            .is_empty()
    );

    // The owner password still opens it, and still reports owner access —
    // the empty user password succeeding must not shadow the stronger claim.
    let as_owner =
        Document::load_with_password(&fixture("enc-emptyuser-rc4-128.pdf"), Some(b"ownerpw"))
            .expect("the owner password must also open it");
    assert_eq!(
        as_owner.encryption().expect("encrypted").auth,
        AuthKind::Owner,
        "supplying the owner password must report owner access even though the \
         empty user password would also have worked"
    );
}

/// A wrong password is refused, and refused as a *password* problem rather
/// than as file damage.
///
/// The distinction is the operator-visible one: "this file is broken" sends
/// someone hunting for a corrupt download; "this needs a password" does not.
#[test]
fn wrong_password_asks_for_a_password() {
    let e = Document::load_with_password(&fixture("enc-rc4-128.pdf"), Some(b"not the password"))
        .expect_err("a wrong password must not open the document");
    assert!(
        matches!(e, DocError::PasswordRequired),
        "expected PasswordRequired, got {e:?}"
    );
}

/// No password at all behaves the same way — and, importantly, does **not**
/// succeed. §7.6.3.1's silent empty-password attempt runs first for every
/// document, so a file that still refuses genuinely has a user password.
#[test]
fn no_password_on_a_protected_file_asks_for_one() {
    let e = Document::load(&fixture("enc-rc4-40.pdf"))
        .expect_err("this fixture has a non-empty user password");
    assert!(matches!(e, DocError::PasswordRequired), "got {e:?}");
}

/// AES-128 (`/CFM /AESV2`) opens, with either password (increment 2).
///
/// This assertion replaced a refusal test. The refusal was correct when it was
/// written and is now false, which is the honest reason to change a test
/// rather than add one beside it.
#[test]
fn aes_128_opens_with_either_password() {
    for pw in [&b"userpw"[..], b"ownerpw"] {
        let doc = Document::load_with_password(&fixture("enc-aes-128.pdf"), Some(pw))
            .expect("AES-128 is implemented");
        assert!(
            doc.encryption().is_some(),
            "the document is still encrypted"
        );
        assert!(
            !page_tree::pages(&doc)
                .expect("the page tree walks after decryption")
                .is_empty(),
            "a decrypted document has pages"
        );
    }
}

/// **AES decryption produces the right STRINGS, which pixels cannot prove.**
///
/// The end-to-end fidelity proof for this increment lives in `pdfce-cli`'s
/// `decrypting_reproduces_the_plaintext_document_exactly` and compares
/// *rendered pixels*. That covers stream data thoroughly and strings barely:
/// a form field's `/T` name is never drawn, so string decryption could be
/// entirely broken and every pixel would still match.
///
/// Strings take a genuinely different path — decrypted in the *parsed object*
/// by `apply::decrypt_strings`, not in the retained buffer — so they need
/// their own assertion. A field name is ideal: it is an encrypted string
/// (**E7** exempts numbers and names, not strings), it is compared here
/// against the plaintext document the fixture was made from, and an
/// off-by-one in the `sAlT` key derivation would turn it into noise rather
/// than into a plausible different name.
#[test]
fn aes_128_decrypts_strings_not_only_stream_data() {
    let plain = Document::load(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/synthetic/forms/demo-form.pdf"),
    )
    .expect("the plaintext source of every encryption fixture");
    let enc = Document::load_with_password(&fixture("enc-aes-128.pdf"), Some(b"userpw"))
        .expect("AES-128 is implemented");

    let names = |d: &Document| -> Vec<String> {
        let form = pdfce_core::forms::parse_acroform(d).expect("the fixture has an AcroForm");
        let mut v: Vec<String> = form
            .fields
            .iter()
            .map(|f| f.fully_qualified_name.clone())
            .collect();
        v.sort();
        v
    };

    let expected = names(&plain);
    assert!(
        !expected.is_empty(),
        "the fixture must actually have named fields, or this test proves nothing"
    );
    assert_eq!(
        names(&enc),
        expected,
        "AES-decrypted field names must equal the plaintext document's. A \
         mismatch here is a string-path bug that renders byte-identically."
    );
}

/// AES-128 with an **empty user password** opens with no password at all —
/// the §7.6.3.1 silent attempt, now exercised on the AES path too.
///
/// This fixture was the *only* empty-password fixture once, and because AES
/// was refused before authentication was ever reached, the most
/// operator-visible behaviour in clause 7.6 went unexecuted. It is now a real
/// acceptance case rather than a file that fails early for an unrelated reason.
#[test]
fn aes_128_with_an_empty_user_password_needs_no_password() {
    let doc = Document::load(&fixture("enc-emptyuser.pdf"))
        .expect("an empty user password is tried silently, §7.6.3.1");
    assert!(doc.encryption().is_some(), "it is still an encrypted file");
    assert!(
        !page_tree::pages(&doc)
            .expect("the page tree walks after decryption")
            .is_empty()
    );
}

/// `/R` 5 and `/R` 6 are refused for **different reasons**, and this test
/// exists to keep them different.
///
/// `/R` 5 is a sourced algorithm pdfce has not written. `/R` 6's Algorithm 2.B
/// is **not sourced** past step (a) — ISO 32000-2 is paywalled and no public
/// document carries it. Those resolve differently: one needs engineering time,
/// the other needs a document nobody has found. Collapsing them into one
/// message throws away the only part that tells anyone what to do next.
#[test]
fn r5_and_r6_are_refused_for_different_reasons() {
    let r5 = Document::load_with_password(&fixture("enc-aes-256-r5.pdf"), Some(b"userpw"))
        .expect_err("R5 is not implemented");
    assert!(
        matches!(
            r5,
            DocError::Encryption(EncryptionUnsupported::CipherNotImplemented("AES-256"))
        ),
        "R5 should report an unimplemented cipher, got {r5:?}"
    );

    let r6 = Document::load_with_password(&fixture("enc-aes-256-r6.pdf"), Some(b"userpw"))
        .expect_err("R6's key derivation is unsourced");
    assert!(
        matches!(
            r6,
            DocError::Encryption(EncryptionUnsupported::UnsourcedRevision)
        ),
        "R6 should report an unsourced algorithm, got {r6:?}"
    );
}

/// ★ Saving a decrypted document is **refused**, in both modes.
///
/// This is the sharp edge of a read-only increment, and it has to be a
/// refusal rather than a best effort. After decryption the buffer and the
/// parsed objects deliberately disagree — stream data was decrypted in the
/// retained buffer (RC4 preserves length, so it fits exactly), strings were
/// decrypted in the parsed objects (a decrypted string cannot generally be
/// re-escaped into the same byte count). Both save modes re-emit untouched
/// objects verbatim from their source span, so a save here would write a file
/// whose `/Encrypt` claims encryption, whose streams are plaintext and whose
/// strings are ciphertext.
///
/// That file is not "partly saved". Nothing can open it, pdfce included, and
/// the save would have reported success.
///
/// The two alternatives were rejected deliberately: re-encrypting needs a key
/// the document does not retain and would emit RC4, which pdfce never writes
/// (**W14**); stripping `/Encrypt` would silently discard protection the
/// author applied, which is the operator's decision and not pdfce's (rule 4).
#[test]
fn saving_a_decrypted_document_is_refused_in_both_modes() {
    let doc =
        Document::load_with_password(&fixture("enc-rc4-128.pdf"), Some(b"userpw")).expect("opens");
    let out = std::env::temp_dir().join("pdfce-encrypted-save-refusal.pdf");
    let dirty = DirtySet::empty();
    let options = SaveOptions::default();

    let _ = std::fs::remove_file(&out);

    for (mode, result) in [
        ("incremental", doc.save_incremental(&out, &dirty, &options)),
        ("full", doc.save_full(&out, &dirty, &options)),
    ] {
        let e = result.err().unwrap_or_else(|| {
            panic!("{mode} save of a decrypted document must be refused, not attempted")
        });
        assert!(
            matches!(e, WriteError::EncryptedSaveUnsupported),
            "{mode}: expected EncryptedSaveUnsupported, got {e:?}"
        );
    }

    // The refusal must happen BEFORE any bytes are written. A refusal that
    // leaves a truncated or half-written file behind has replaced one broken
    // output with another, and the operator has no way to tell which.
    assert!(
        !out.exists(),
        "a refused save must not leave a file behind at {}",
        out.display()
    );
}

/// An unencrypted document reports `None`, and this is worth an explicit test:
/// every assertion above would also pass against an implementation that
/// believed every document was encrypted.
#[test]
fn plain_documents_report_no_encryption() {
    let plain = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/synthetic/hello.pdf");
    if !plain.exists() {
        // The fixture set is allowed to move; skipping is better than a
        // failure that says nothing about encryption.
        return;
    }
    let doc = Document::load(&plain).expect("plain fixture must load");
    assert!(doc.encryption().is_none());
}
