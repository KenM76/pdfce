# Encrypted-document fixtures

**Synthetic and self-generated** (project rule 7 / `LEGAL.md` §5). The
plaintext document is pdfce's own synthetic form fixture; the encryption was
applied by **pypdf 6.7.0**, chosen deliberately as an *independent*
implementation.

Passwords: user `userpw`, owner `ownerpw`, except `enc-emptyuser.pdf` whose
user password is the **empty string** — the case §7.6.3.1 says a reader
*shall* try silently before prompting, and the reason permissions-only PDFs
open everywhere with no dialog.

| File | `/V` | `/R` | `/Length` | `/CFM` |
|---|---|---|---|---|
| `enc-rc4-40.pdf` | 1 | 2 | 40 | — |
| `enc-rc4-128.pdf` | 2 | 3 | 128 | — |
| `enc-aes-128.pdf` | 4 | 4 | 128 | `/AESV2` |
| `enc-aes-256-r5.pdf` | 5 | 5 | 256 | `/AESV3` |
| `enc-aes-256-r6.pdf` | 5 | 6 | 256 | `/AESV3` |
| `enc-emptyuser.pdf` | 4 | 4 | 128 | `/AESV2` |

## ★ What these can and cannot prove

**They cut one way only, and the distinction is the whole point.**

For **`/R` 2, 3 and 4**, ISO 32000-1 §7.6 fully specifies the algorithms.
pdfce's decryption will be written from the clause, then checked against
files it did not produce — so agreement means two independent readings of the
same specification agree. That is evidence.

For **`/R` 6**, the algorithm (2.B) is **not sourced**: ISO 32000-2 is
paywalled past step (a). Deriving it from another implementation and then
testing against that implementation's output would be circular — the test
could not fail. `enc-aes-256-r6.pdf` is therefore a **refusal fixture**:
pdfce must decline it *by name*, distinguished from `/R` 5, and the test
asserts the refusal rather than a decrypt.

`enc-aes-256-r5.pdf` sits between the two. `/R` 5 is a deprecated Adobe
extension, paraphrased in the corpus rather than sourced from ISO, and PDF
2.0 deprecates handler revisions 1–5 outright. Reading it is still required —
Acrobat wrote such files between 2008 and 2011, and deprecation does not
un-write them.

## Regenerating

`scratchpad/mkcrypt.py` produces all six from a plaintext source. Kept
because a fixture whose construction nobody can repeat is a fixture nobody
can extend.
