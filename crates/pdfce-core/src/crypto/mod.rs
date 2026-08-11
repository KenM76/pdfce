//! Document encryption — ISO 32000-1 §7.6.
//!
//! # What this module is for
//!
//! A PDF may be encrypted, and until this module existed pdfce refused every
//! such file with [`XrefErrorKind::EncryptionUnsupported`]. That refusal was
//! honest and is still the fallback, but it covered the *whole* of §7.6
//! including the commonest case in the wild: a document with an **empty user
//! password** and a non-empty owner password, which every other viewer opens
//! silently and without a prompt. To an operator, pdfce refusing a file Chrome
//! opens does not read as a scoped capability gap.
//!
//! # Increment 1 — RC4 only, read direction only
//!
//! | | Status |
//! |---|---|
//! | `/V` 1, 2 at `/R` 2, 3 (RC4, 40–128 bit) | **implemented** |
//! | `/V` 4 at `/R` 4 with `/CFM /V2` (RC4 via crypt filter) | **implemented** |
//! | `/V` 4 with `/CFM /AESV2` (AES-128) | refused by cipher name |
//! | `/V` 5, `/R` 5 (AES-256) | refused by cipher name |
//! | `/V` 5, `/R` 6 (AES-256 hardened) | refused as **unsourced** — Algorithm 2.B is not in the project's spec corpus past step (a) |
//! | Public-key handler, third-party handlers | refused by handler name |
//! | **Writing** encrypted documents | not implemented, and RC4 will never be written (standing rule W14) |
//!
//! The refusals are deliberately distinguishable. "pdfce hasn't implemented
//! AES yet", "no reader on earth may open this file", and "the algorithm isn't
//! published anywhere we can source it" are three different facts with three
//! different next actions, and collapsing them into one message throws away
//! the only part an operator can act on.
//!
//! # Why RC4 came first, and why that is not a security statement
//!
//! Increment order was chosen on **dependency risk**, not on cipher strength.
//! `pdfce-core` had no cryptographic dependency at all before this; RC4 and
//! MD5 are frozen, tiny, and needed only to *read* documents other producers
//! already made, so implementing them in-crate avoided a rule-13 dependency
//! decision entirely. AES does not qualify for that reasoning and gets a
//! dependency in the next increment — see [`md5`]'s module docs, which state
//! the limits of the judgement in full.
//!
//! RC4 is broken. So is MD5. So, structurally, is PDF encryption at `/V` 1–4:
//! there is **no integrity protection anywhere in it** (negative result N7 —
//! no MAC, and `/P` sits in the file as an editable plaintext integer). pdfce
//! reading these files is a compatibility obligation. Nothing here should be
//! read as a recommendation to produce them.
//!
//! # Permissions are reported, never silently enforced
//!
//! §7.6.3.1 says it outright: *"There is nothing inherent in PDF encryption
//! that enforces the document permissions."* Readers "shall respect the intent
//! of the document creator". So [`standard::Permissions`] is a **report** of
//! what the author asked for, and any place pdfce acts on it must disclose
//! that it is doing so (project rule 4). ISO 32000-1 specifies no mapping from
//! a permission bit to a reader operation at all (**N4**) — "assemble the
//! document" is not an object-level predicate — so the mapping is pdfce's own
//! product decision and has to be visible as such.
//!
//! # Layout
//!
//! - [`md5`] — RFC 1321 digest. Key derivation only.
//! - [`rc4`] — the stream cipher. Encryption and decryption are one operation.
//! - [`standard`] — the `/Standard` handler: `/Encrypt` parsing, Algorithms
//!   1–7, authentication, per-object keys.
//!
//! [`XrefErrorKind::EncryptionUnsupported`]: crate::xref::XrefErrorKind::EncryptionUnsupported

pub mod apply;
pub mod md5;
pub mod rc4;
pub mod standard;

pub use standard::{
    AuthKind, Cipher, EncryptionConfig, EncryptionUnsupported, FileKey, PermissionBit, Permissions,
};
