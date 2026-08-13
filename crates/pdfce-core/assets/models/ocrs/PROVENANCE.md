# PROVENANCE — `ocrs` OCR model weights

Two neural-network weight files pdfce **redistributes** inside its portable
folder. They are **not** a Cargo dependency, so `cargo-about` is structurally
incapable of seeing them and `THIRD_PARTY_LICENSES.md` will never mention them
automatically — the attribution below is authored by hand in `about.hbs`, and
`tools/check-shipped-assets.py` is what enforces that this file exists and
states terms. See `docs/ocr-engine-survey.md` §3.3.

- **Source:** <https://huggingface.co/robertknight/ocrs>
- **Creator:** Robert Knight (the [ocrs](https://github.com/robertknight/ocrs)
  project)
- **Retrieved:** 2026-08-13, from the Hugging Face repository's `main` branch.
- **Licence: `CC-BY-SA-4.0`** — <https://creativecommons.org/licenses/by-sa/4.0/>
  Declared in the model card's YAML front matter (`license: cc-by-sa-4.0`),
  read from the source on the retrieval date rather than from any secondary
  description. **Note that the `ocrs-models` GitHub repository carries no
  `LICENSE` file** — the model card is the declaration.
- **Training data:** [HierText](https://github.com/google-research-datasets/hiertext)
  (itself CC-BY-SA-4.0) plus synthetic data, per the model card.
- **Changes made by pdfce: NONE.** The files are byte-identical to the
  upstream artifacts; only their *names* were shortened (see below). CC-BY-SA
  requires an indication of changes, and the honest indication is that there
  are none.

## The files

| Shipped as | Upstream filename | Bytes | SHA-256 |
|---|---|---:|---|
| `text-detection.rten` | `text-detection-ssfbcj81.rten` | 2,523,564 | `614aafabf27c94d386f7aa036c967c2e47e4b9938fa11531ca8f5698c1ca4c36` |
| `text-rec-checkpoint.rten` | `text-rec-checkpoint-s52qdbqt.rten` | 9,716,444 | `606d9a0414c6b73c99df75b707c11c70d1c8b12e1d4f900922e185fc37bfca65` |

Total 12,240,008 bytes (11.67 MiB).

### ★ Why the names differ, and why the hash is what identifies these files

Upstream filenames carry a **content-addressed suffix** (`-ssfbcj81`,
`-s52qdbqt`) which is *their* versioning scheme. pdfce strips it so
`pdfce_core::ocr::engine_ocrs`'s `DETECTION_MODEL` / `RECOGNITION_MODEL`
constants can name a stable path, and pins the exact artifact by **SHA-256**
instead — *our* versioning scheme.

This matters more than it looks. `docs/ocr-engine-survey.md` recorded that the
**Hugging Face and S3 copies of "the ocrs models" are not byte-identical** —
different filenames, one 13,280 bytes smaller, one 124 bytes larger. *"The
ocrs models"* is therefore not one thing, and a build that fetched "the latest"
would be running weights nobody tested. The hashes above are the identity;
the names are only convenience.

## What CC-BY-SA-4.0 obliges pdfce to do, and what it does not

**Obliges** (satisfied by this file plus the `about.hbs` entry that ships to
end users): name the creator, name and link the licence, state whether changes
were made, and do not apply effective technological measures that restrict
what recipients may do with the files.

**Does not oblige:** anything about pdfce's own source. CC-BY-SA is a licence
for creative works and has **no linking concept at all** — Creative Commons
recommends against using CC licences for software precisely because they
"do not contain specific terms about the distribution of source code".
Shipping these files unmodified alongside MIT code is distribution of a
verbatim work in a **collection**, not an **adaptation**, and only adaptations
must be released under BY-SA. pdfce's MIT licence is unaffected.

## ★ THE ONE THING THAT WOULD CHANGE THAT

**Modifying the weights creates Adapted Material, and the adapted weights must
then be CC-BY-SA-4.0.** That includes fine-tuning them (for CAD drawings, say),
quantizing them for speed, retraining on any corpus, or converting them into
another runtime's format.

It would bind **the derived model**, not pdfce's source — but it means
*"we'll fine-tune this later"* is a decision with a licence attached, and it
needs its own operator decision at the time. Recorded here rather than in a
roadmap entry because this is the file someone will be looking at when they
have the weights open and the idea occurs to them.
