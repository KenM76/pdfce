def stream_obj(dict_prefix, content):
    return dict_prefix + b" /Length %d >>\nstream\n" % len(content) + content + b"\nendstream"
objs = {}
objs[1] = b"<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R 5 0 R] /DA (/Helv 0 Tf 0 g) /DR << /Font << /Helv << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> >> >>"
objs[2] = b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>"
objs[3] = b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Resources << >> /Annots [4 0 R 5 0 R] >>"
objs[4] = b"<< /FT /Tx /T (FullName) /TU (Full name) /Subtype /Widget /Rect [20 150 250 172] /P 3 0 R /MK << /BC [0 0 0] >> >>"
objs[5] = b"<< /FT /Btn /T (Subscribe) /V /Off /AS /Off /Subtype /Widget /Rect [20 100 34 114] /P 3 0 R /AP << /N << /Yes 6 0 R /Off 7 0 R >> >> >>"
objs[6] = stream_obj(b"<< /Type /XObject /Subtype /Form /BBox [0 0 14 14]", b"0 0 14 14 re f 4 4 6 6 re f")
objs[7] = stream_obj(b"<< /Type /XObject /Subtype /Form /BBox [0 0 14 14]", b"0 0 14 14 re S")
buf = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n"
off = {}
for n in sorted(objs):
    off[n] = len(buf)
    buf += b"%d 0 obj\n" % n + objs[n] + b"\nendobj\n"
xref_at = len(buf); size = max(objs) + 1
buf += b"xref\n0 %d\n0000000000 65535 f \n" % size
for n in range(1, size):
    buf += b"%010d 00000 n \n" % off[n]
buf += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (size, xref_at)
open("fixtures/synthetic/forms/demo-form.pdf","wb").write(buf)
print("wrote", len(buf), "bytes")
