"""Generate encrypted PDFs in every standard-handler mode, as decryption
falsifiers.

Source document is synthetic and self-authored (project rule 7). The
encryption is produced by pypdf — an INDEPENDENT implementation, which is the
whole point: pdfce's decryption will be written from ISO 32000-1 §7.6 and
then checked against files it did not produce. Agreement then means two
independent readings of the same clause agree, which is evidence; agreement
with its own output would mean nothing.

Caveat recorded up front: this cuts one way only. For R2/R3/R4 and AES-128,
ISO 32000-1 fully specifies the algorithms, so pypdf's files are a genuine
cross-check of a spec-derived implementation. For **R6 (AES-256)** the
algorithm is NOT sourced — deriving it from pypdf and then testing against
pypdf would be circular, and these files are therefore refusal fixtures, not
acceptance fixtures.
"""
import sys
from pypdf import PdfWriter, PdfReader

src, outdir = sys.argv[1], sys.argv[2]

MODES = [
    ('rc4-40', 'RC4_40'),
    ('rc4-128', 'RC4_128'),
    ('aes-128', 'AES_128'),
    ('aes-256-r5', 'AES_256_R5'),
    ('aes-256-r6', 'AES_256'),
]

USER = 'userpw'
OWNER = 'ownerpw'

for name, algo in MODES:
    w = PdfWriter(clone_from=src)
    w.encrypt(user_password=USER, owner_password=OWNER, algorithm=algo)
    path = f'{outdir}/enc-{name}.pdf'
    with open(path, 'wb') as f:
        w.write(f)
    # Read the /Encrypt dictionary back so the fixture's own parameters are
    # visible without a hex editor.
    r = PdfReader(path)
    enc = r.trailer['/Encrypt'].get_object()
    print(f'{name:12} V={enc.get("/V")} R={enc.get("/R")} '
          f'Length={enc.get("/Length")} P={enc.get("/P")} '
          f'CFM={enc.get("/CF", {}).get("/StdCF", {}).get("/CFM", "-")}')

# And one with an EMPTY user password — the case §7.6.3.1 says a reader
# shall try silently before prompting, which is why permissions-only PDFs
# open everywhere with no dialog.
w = PdfWriter(clone_from=src)
w.encrypt(user_password='', owner_password=OWNER, algorithm='AES_128')
with open(f'{outdir}/enc-emptyuser.pdf', 'wb') as f:
    w.write(f)
print('emptyuser    AES_128, user password is the empty string')
