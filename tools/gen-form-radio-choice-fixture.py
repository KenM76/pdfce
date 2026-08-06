"""Generate `fixtures/synthetic/forms/radio-choice-form.pdf`.

WHY THIS FIXTURE EXISTS
=======================
`demo-form.pdf` (see `gen-form-fixtures.py`) carries exactly one text field
and one check box. That was enough to build and verify the Forms panel's P0,
but it cannot exercise the two field kinds P1 adds:

  * a RADIO GROUP  — one field, several widget kids, each with its own
    on-state name, `/V` holding whichever name is selected (or `/Off`); and
  * a CHOICE FIELD — `/Ch`, with an `/Opt` array of options.

Both are shapes the panel has to get right and neither could be tested at
all against the existing fixture, so a driven verification of them would have
been impossible — which is the same "no oracle" position this project treats
as a defect rather than an inconvenience.

WHAT IT BUILDS (ISO 32000-1 §12.7.4.2.3, §12.7.4.4)
====================================================
Four fields on one 300x260 page:

  1. `Colour`  — `/FT /Btn` with `/Ff 32768` (bit 16, `Radio`), THREE widget
     kids whose `/AP /N` sub-dictionaries define on-states `/Red`, `/Green`
     and `/Blue`. `/V /Green` selected, so the fixture also proves the panel
     reads an EXISTING selection rather than only writing one.
  2. `Locked`  — the same shape plus `/Ff 49152` (bit 16 `Radio` + bit 15
     `NoToggleToOff`), so the panel's "offer Clear only when the field
     permits it" rule has something to be tested against. Without this the
     rule would be unfalsifiable here.
  3. `Country` — `/FT /Ch` combo (`/Ff 131072`, bit 18 `Combo`) with a
     three-entry `/Opt` array of `[export display]` PAIRS, so export and
     display genuinely DIFFER (`(CA) (Canada)`). A panel that matches on the
     wrong one of the two shows a filled field as empty, and with equal
     strings that bug is invisible.
  4. `Toppings` — `/FT /Ch` list box with `/Ff 2097152` (bit 22,
     `MultiSelect`) and a plain-string `/Opt`, exercising BOTH `/Opt` forms
     (pair array vs bare string) in one document.

`/Opt` order is deliberately NOT alphabetical (`Canada`, `Mexico`,
`Argentina`): §12.7.4.4 says a conforming reader SHALL display the options
in the order they occur in `/Opt`. An alphabetical fixture would let a
sorting bug pass.

Run from the repository root:  `python tools/gen-form-radio-choice-fixture.py`
"""


def stream_obj(dict_prefix, content):
    """A stream object with a correct `/Length`, matching the sibling script."""
    return (
        dict_prefix
        + b" /Length %d >>\nstream\n" % len(content)
        + content
        + b"\nendstream"
    )


def radio_kid(parent, rect, on_state, ap_on, ap_off, selected):
    """One radio widget kid: its own `/AP /N` on-state plus the shared `/Off`.

    `/AS` names the state this KID currently shows, which for a radio set is
    the on-state only on the selected kid and `/Off` on every other — the
    field-level `/V` is what says which one that is (§12.7.4.2.3).
    """
    state = on_state if selected else b"Off"
    return (
        b"<< /Parent %d 0 R /Subtype /Widget /Rect [%d %d %d %d] /P 3 0 R "
        b"/AS /%s /AP << /N << /%s %d 0 R /Off %d 0 R >> >> >>"
        % (parent, rect[0], rect[1], rect[2], rect[3], state, on_state, ap_on, ap_off)
    )


objs = {}

# Catalog: four terminal/parent fields in /Fields. Widget KIDS are not listed
# there — only the field parents are (§12.7.2 /Fields is the root field list).
objs[1] = (
    b"<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [10 0 R 20 0 R 30 0 R 40 0 R 50 0 R] "
    b"/DA (/Helv 0 Tf 0 g) /DR << /Font << /Helv << /Type /Font /Subtype /Type1 "
    b"/BaseFont /Helvetica >> >> >> >> >>"
)
objs[2] = b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"
objs[3] = (
    b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 260] /Resources << >> "
    b"/Annots [11 0 R 12 0 R 13 0 R 21 0 R 22 0 R 31 0 R 41 0 R 51 0 R] >>"
)

# Shared appearance streams: one filled dot (on) and one empty ring (off).
objs[4] = stream_obj(
    b"<< /Type /XObject /Subtype /Form /BBox [0 0 12 12]", b"3 3 6 6 re f"
)
objs[5] = stream_obj(
    b"<< /Type /XObject /Subtype /Form /BBox [0 0 12 12]", b"0.5 0.5 11 11 re S"
)

# -- 1. Colour: a plain radio group, /Green pre-selected. --
objs[10] = (
    b"<< /FT /Btn /Ff 32768 /T (Colour) /TU (Favourite colour) /V /Green "
    b"/Kids [11 0 R 12 0 R 13 0 R] >>"
)
objs[11] = radio_kid(10, (20, 210, 32, 222), b"Red", 4, 5, False)
objs[12] = radio_kid(10, (60, 210, 72, 222), b"Green", 4, 5, True)
objs[13] = radio_kid(10, (100, 210, 112, 222), b"Blue", 4, 5, False)

# -- 2. Locked: Radio + NoToggleToOff, so Clear must NOT be offered. --
objs[20] = (
    b"<< /FT /Btn /Ff 49152 /T (Locked) /TU (Always answered) /V /Yes "
    b"/Kids [21 0 R 22 0 R] >>"
)
objs[21] = radio_kid(20, (20, 175, 32, 187), b"Yes", 4, 5, True)
objs[22] = radio_kid(20, (60, 175, 72, 187), b"No", 4, 5, False)

# -- 3. Country: a combo whose /Opt entries are [export display] PAIRS. --
#    Export != display on purpose; /Opt order is not alphabetical on purpose.
objs[30] = (
    b"<< /FT /Ch /Ff 131072 /T (Country) /TU (Country of residence) /V (MX) "
    b"/Kids [31 0 R] "
    b"/Opt [ [(CA) (Canada)] [(MX) (Mexico)] [(AR) (Argentina)] ] >>"
)
objs[31] = (
    b"<< /Parent 30 0 R /Subtype /Widget /Rect [20 130 200 150] /P 3 0 R "
    b"/MK << /BC [0 0 0] >> >>"
)

# -- 4. Toppings: a multi-select list box with bare-string /Opt entries. --
objs[40] = (
    b"<< /FT /Ch /Ff 2097152 /T (Toppings) /TU (Toppings) /V [(Olives)] "
    b"/Kids [41 0 R] "
    b"/Opt [ (Cheese) (Olives) (Peppers) ] >>"
)
objs[41] = (
    b"<< /Parent 40 0 R /Subtype /Widget /Rect [20 60 200 115] /P 3 0 R "
    b"/MK << /BC [0 0 0] >> >>"
)

# -- 5. Notes: a RICH-TEXT field (/Ff bit 26 = 33554432).
#
#    /RV carries the XHTML-subset body (SS12.7.3.4 Tables 224/225), /DS the
#    default style string (a bare `name:value;` list — NOT XML, per Table 223),
#    and /V the plain-text twin the standard says SHOULD be preserved.
#
#    The /V wording deliberately DIFFERS from the /RV wording, and that is the
#    whole point of this field. Appearance generation for a rich-text field is
#    bound to /RV, not /V, so a plain-text writer that sets /V and leaves /RV
#    in place produces a document that DISPLAYS THE OLD WORDS. With /V and /RV
#    saying the same thing, that correctness bug is invisible to any test.
objs[50] = (
    b"<< /FT /Tx /Ff 33554432 /T (Notes) /TU (Notes) /Kids [51 0 R] "
    b"/V (RICH ORIGINAL) "
    b"/DS (font: 12pt Helvetica; color: #FF0000) "
    b"/RV (<body xmlns='http://www.w3.org/1999/xhtml' "
    b"xmlns:xfa='http://www.xfa.org/schema/xfa-data/1.0/' xfa:spec='2.4'>"
    b"<p><b>RICH</b> <i>ORIGINAL</i></p></body>) >>"
)
objs[51] = (
    b"<< /Parent 50 0 R /Subtype /Widget /Rect [20 15 280 50] /P 3 0 R "
    b"/MK << /BC [0 0 0] >> >>"
)


buf = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n"
off = {}
for n in sorted(objs):
    off[n] = len(buf)
    buf += b"%d 0 obj\n" % n + objs[n] + b"\nendobj\n"

# A classic xref with free entries for the gaps in the numbering (6-9, 14-19,
# 23-29, 32-39), because the object numbers are grouped by field for
# readability rather than packed.
xref_at = len(buf)
size = max(objs) + 1
buf += b"xref\n0 %d\n0000000000 65535 f \n" % size
for n in range(1, size):
    if n in off:
        buf += b"%010d 00000 n \n" % off[n]
    else:
        buf += b"0000000000 65535 f \n"
buf += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (size, xref_at)

out = "fixtures/synthetic/forms/radio-choice-form.pdf"
open(out, "wb").write(buf)
print("wrote", out, len(buf), "bytes")
