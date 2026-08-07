#!/usr/bin/env python3
"""Byte-author ``forms/unfillable-fields-form.pdf``.

WHY THIS FIXTURE EXISTS
-----------------------
Three field shapes cannot be **filled** and can perfectly well be
**deleted**:

* a read-only field (``/Ff`` bit 1, §12.7.3.1 Table 221),
* a signature field (``/FT /Sig``, §12.7.4.5),
* a push button (``/Ff`` bit 17, §12.7.4.2.1).

`EditSession::deletion_preflight` checks encryption and certification and
**nothing else** — no read-only test, no field-type test — so all three
are deletable. The forms panel used to `continue` past these rows after
printing "why you cannot fill this", which meant it also skipped the
delete control and denied an affordance the capability actually had.
That is R83 read in the wrong direction: the rule forbids offering what
you cannot do, not withholding what you can.

Fixing that is a one-line restructure and completely invisible without a
fixture carrying such a field. Every forms fixture in the corpus before
this one had only fillable fields, so a test of the fix would have had
nothing to assert on and the regression could return unnoticed. **R162**:
the container has to be shown capable of holding the thing.

WHAT IS IN THE FILE
-------------------
Four fields on one page, chosen so each blocked reason is represented
once and one control field proves the panel still works normally:

* ``Locked``    — text, ``/Ff 1`` (read-only)
* ``Approved``  — ``/FT /Sig``, no value (an empty signature field)
* ``Submit``    — ``/FT /Btn`` with ``/Ff 65536`` (bit 17, push button)
* ``Notes``     — an ordinary fillable text field, the control

All four are merged field/widget dictionaries (Shape A), so each shows a
single "Delete field" control rather than the expanded multi-widget form
— this fixture is about *which rows get a control*, not about the
single-vs-multi split, which ``radio-group-form.pdf`` already covers.

PROVENANCE
----------
100% byte-authored here — no PDF library is involved, so the fixture
cannot inherit a bug from the code it is used to test (project rule 7 /
``LEGAL.md`` §5).

USAGE
-----
    python tools/gen-form-undeletable-looking-fixture.py
"""

from pathlib import Path

OUT = Path("fixtures/synthetic/forms/unfillable-fields-form.pdf")


def stream_obj(dict_prefix: bytes, content: bytes) -> bytes:
    return (
        dict_prefix
        + b" /Length %d >>\nstream\n" % len(content)
        + content
        + b"\nendstream"
    )


def build() -> bytes:
    objs: dict[int, bytes] = {}
    objs[1] = (
        b"<< /Type /Catalog /Pages 2 0 R "
        b"/AcroForm << /Fields [4 0 R 5 0 R 6 0 R 7 0 R] "
        b"/DA (/Helv 0 Tf 0 g) /DR << /Font << /Helv << /Type /Font "
        b"/Subtype /Type1 /BaseFont /Helvetica >> >> >> >> >>"
    )
    objs[2] = b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"
    objs[3] = (
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 260] "
        b"/Resources << >> /Annots [4 0 R 5 0 R 6 0 R 7 0 R] >>"
    )
    # /Ff 1 = ReadOnly (Table 221 bit position 1). Carries a value so the
    # row has something to display read-only.
    objs[4] = (
        b"<< /FT /Tx /T (Locked) /TU (Locked field) /Ff 1 /V (fixed value) "
        b"/Subtype /Widget /Rect [20 200 250 222] /P 3 0 R >>"
    )
    # An empty signature field (§12.7.4.5). Unsigned: no /V.
    objs[5] = (
        b"<< /FT /Sig /T (Approved) /TU (Approval signature) "
        b"/Subtype /Widget /Rect [20 150 250 182] /P 3 0 R >>"
    )
    # /Ff 65536 = bit 17, Pushbutton (§12.7.4.2.1 Table 227).
    objs[6] = (
        b"<< /FT /Btn /T (Submit) /TU (Submit button) /Ff 65536 "
        b"/Subtype /Widget /Rect [20 100 90 124] /P 3 0 R "
        b"/AP << /N 8 0 R >> >>"
    )
    # The control: an ordinary fillable field. If the panel ever stops
    # rendering THIS row's controls, the fixture says so rather than
    # letting a blanket regression look like the fix working.
    objs[7] = (
        b"<< /FT /Tx /T (Notes) /TU (Notes) "
        b"/Subtype /Widget /Rect [20 50 250 72] /P 3 0 R >>"
    )
    objs[8] = stream_obj(
        b"<< /Type /XObject /Subtype /Form /BBox [0 0 70 24]",
        b"0 0 70 24 re S",
    )

    buf = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n"
    off: dict[int, int] = {}
    for n in sorted(objs):
        off[n] = len(buf)
        buf += b"%d 0 obj\n" % n + objs[n] + b"\nendobj\n"
    xref_at = len(buf)
    size = max(objs) + 1
    buf += b"xref\n0 %d\n0000000000 65535 f \n" % size
    for n in range(1, size):
        buf += b"%010d 00000 n \n" % off[n]
    buf += (
        b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n"
        % (size, xref_at)
    )
    return buf


def main() -> int:
    OUT.parent.mkdir(parents=True, exist_ok=True)
    data = build()
    OUT.write_bytes(data)
    print(f"wrote {OUT} ({len(data)} bytes)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
