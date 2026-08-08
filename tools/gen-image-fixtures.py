#!/usr/bin/env python3
"""Generate the raster-image fixtures for `EditSession::add_image` (image placement).

WHY THIS EXISTS
---------------
`pdfce-core`'s image *import* path (`crates/pdfce-core/src/image_import/`)
turns an external raster file — PNG, JPEG, BMP — into a PDF image XObject.
Proving it needs *real* container bytes: a hand-written unit test over a
synthetic 12-byte "PNG" proves the chunk walker and nothing else. Only an
actual zlib-compressed IDAT stream proves that pdfce reused the PNG's own
per-row filter bytes as an ISO 32000-1 §7.4.4.4 `/Predictor 15` FlateDecode
stream, and only an actual Huffman-coded JPEG proves the `/DCTDecode`
passthrough is byte-identical.

Those bytes cannot be downloaded. `docs/LEGAL.md` §5 and project rule 7
permit only synthetic or clearly rights-cleared test data, and an image
pulled off the web is neither. They are therefore GENERATED here, from
pixel data this project authored (flat colour ramps and checkerboards —
nothing derived from a third-party work), and written to
`fixtures/synthetic/images/`.

This script is NOT part of the build. It is run by hand when the fixture
set needs to change, and its output is committed. It is not a Cargo
workspace member, so it never enters the dependency graph or
THIRD_PARTY_LICENSES.md.

USAGE
-----
    python tools/gen-image-fixtures.py

Requires Pillow for the JPEG fixtures ONLY (a developer-machine
dependency, never a pdfce dependency). Every PNG and BMP here is written
byte by byte with `struct` + `zlib` from the standard library, deliberately:
a PNG produced by Pillow would carry Pillow's choices about chunk order,
filter selection and compression level, and the fixtures that pin pdfce's
*passthrough* branch must pin bytes this project chose. Verified with
Pillow 12.1.0.

WHAT EACH FIXTURE PINS
----------------------
PNG — the passthrough branch (no re-compression, IDAT reused verbatim):
    rgb8.png            colour type 2, 8-bit, non-interlaced. THE canonical
                        passthrough case: /FlateDecode + /Predictor 15 +
                        /Colors 3 + /BitsPerComponent 8 + /Columns W, with
                        the IDAT zlib stream copied byte for byte.
    gray8.png           colour type 0 → /DeviceGray, /Colors 1.
    gray16.png          colour type 0, 16-bit → /BitsPerComponent 16 (a
                        PDF 1.5 feature; pins the version disclosure).
    indexed8.png        colour type 3, 8-bit indices → /Indexed /DeviceRGB.
    indexed4.png        colour type 3, 4-bit indices → sub-byte packing,
                        where row_data_bytes = ceil(W*1*4/8) and bpp = 1.
    rgb-trns.png        colour type 2 + tRNS (one transparent colour) →
                        /Mask colour-key array, passthrough preserved.
    indexed-trns.png    colour type 3 + tRNS (per-entry alpha) → passthrough
                        base image + a DECODED 8-bit /SMask.

PNG — the decode-and-recompress branch (alpha is interleaved in the rows,
so the samples must be split into base + soft mask):
    rgba8.png           colour type 6 → /DeviceRGB base + 8-bit /SMask.
    graya8.png          colour type 4 → /DeviceGray base + 8-bit /SMask.
    rgba16.png          colour type 6, 16-bit → 16-bit base + 16-bit /SMask.

PNG — refusals:
    interlaced.png      Adam7. Refused BY NAME (pdfce has no de-interlacer;
                        re-save it non-interlaced).

JPEG — the verbatim /DCTDecode branch:
    rgb.jpg             baseline SOF0, 3 components → /DeviceRGB.
    gray.jpg            baseline SOF0, 1 component → /DeviceGray.
    progressive.jpg     SOF2. 14% of the JPEGs measured inside real PDFs
                        (decision 005 §3.2), so "baseline is enough" is
                        false.
    cmyk.jpg            4 components, Adobe APP14 transform 0 → /DeviceCMYK
                        with NO /Decode (R29), and the R30 polarity
                        disclosure.
    exif-rot90.jpg      EXIF Orientation 6 (rotate 90° CW). Pins that pdfce
                        applies the orientation in the placement matrix
                        rather than re-encoding the pixels.
    arithmetic.jpg      SOF9 (arithmetic entropy coding). Refused BY NAME —
                        patched from `rgb.jpg`, because no common encoder
                        will produce one.

BMP — the decode-and-recompress branch:
    rgb24.bmp           BI_RGB 24-bit bottom-up (the Windows default).
    rgb32.bmp           BI_RGB 32-bit (the 4th byte is padding, NOT alpha,
                        in a BITMAPINFOHEADER file — pins that pdfce does
                        not invent a soft mask from it).
    pal8.bmp            BI_RGB 8-bit with a 256-entry palette → /Indexed.
    topdown24.bmp       BI_RGB 24-bit with a NEGATIVE height (top-down row
                        order) — the sign flip that silently mirrors an
                        image vertically if ignored.
    rle8.bmp            BI_RLE8. Refused BY NAME.

Not an image at all:
    not-an-image.bin    Refused BY NAME, with a message naming the formats
                        that DO work.
"""

import struct
import zlib
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
OUT = REPO / "fixtures" / "synthetic" / "images"


# ---------------------------------------------------------------------------
# PNG writer (RFC 2083), written by hand so the fixtures pin OUR bytes
# ---------------------------------------------------------------------------


def png_chunk(kind: bytes, payload: bytes) -> bytes:
    """One PNG chunk: length, type, payload, CRC-32 over type+payload."""
    return (
        struct.pack(">I", len(payload))
        + kind
        + payload
        + struct.pack(">I", zlib.crc32(kind + payload) & 0xFFFFFFFF)
    )


def png(
    width: int,
    height: int,
    color_type: int,
    bit_depth: int,
    rows: list[bytes],
    *,
    palette: bytes | None = None,
    trns: bytes | None = None,
    interlace: int = 0,
    filter_types: list[int] | None = None,
) -> bytes:
    """Assemble a PNG.

    `rows` are the RAW (unfiltered) scanline payloads, WITHOUT the leading
    filter-type byte. `filter_types` picks the per-row filter tag; the
    default is 0 (None) for every row, because a fixture whose rows are all
    filter 0 would not prove that pdfce preserves the per-row tags. Where
    that matters, callers pass a mixed list — and this function applies the
    real RFC 2083 §6 filters so the file is a legitimate PNG, not merely a
    file with interesting tag bytes.
    """
    ihdr = struct.pack(
        ">IIBBBBB", width, height, bit_depth, color_type, 0, 0, interlace
    )
    out = b"\x89PNG\r\n\x1a\n" + png_chunk(b"IHDR", ihdr)
    if palette is not None:
        out += png_chunk(b"PLTE", palette)
    if trns is not None:
        out += png_chunk(b"tRNS", trns)

    channels = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}[color_type]
    bpp = max(1, (channels * bit_depth) // 8)
    tags = filter_types or [0] * height

    filtered = bytearray()
    prior = bytes(len(rows[0])) if rows else b""
    for row, tag in zip(rows, tags):
        filtered.append(tag)
        filtered.extend(_png_filter(tag, row, prior, bpp))
        prior = row
    out += png_chunk(b"IDAT", zlib.compress(bytes(filtered), 9))
    return out + png_chunk(b"IEND", b"")


def _png_filter(tag: int, row: bytes, prior: bytes, bpp: int) -> bytes:
    """Apply RFC 2083 §6 filter `tag` to one raw scanline."""
    out = bytearray(len(row))
    for i, x in enumerate(row):
        a = row[i - bpp] if i >= bpp else 0
        b = prior[i] if i < len(prior) else 0
        c = prior[i - bpp] if i >= bpp and i - bpp < len(prior) else 0
        if tag == 0:
            out[i] = x
        elif tag == 1:
            out[i] = (x - a) & 0xFF
        elif tag == 2:
            out[i] = (x - b) & 0xFF
        elif tag == 3:
            out[i] = (x - (a + b) // 2) & 0xFF
        elif tag == 4:
            out[i] = (x - _paeth(a, b, c)) & 0xFF
        else:
            raise ValueError(f"filter tag {tag}")
    return bytes(out)


def _paeth(a: int, b: int, c: int) -> int:
    """RFC 2083 §6.6 PaethPredictor — the tie-break order (a, b, c) is normative."""
    p = a + b - c
    pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
    if pa <= pb and pa <= pc:
        return a
    if pb <= pc:
        return b
    return c


# ---------------------------------------------------------------------------
# The pixel data — flat ramps and checkerboards this project authored
# ---------------------------------------------------------------------------

W, H = 6, 4


def rgb_rows(depth: int = 8) -> list[bytes]:
    """A red→blue horizontal ramp with a green vertical component."""
    rows = []
    for y in range(H):
        row = bytearray()
        for x in range(W):
            r = 255 * x // (W - 1)
            g = 255 * y // (H - 1)
            b = 255 - r
            if depth == 8:
                row += bytes([r, g, b])
            else:
                row += struct.pack(">HHH", r * 257, g * 257, b * 257)
        rows.append(bytes(row))
    return rows


def gray_rows(depth: int = 8) -> list[bytes]:
    rows = []
    for y in range(H):
        row = bytearray()
        for x in range(W):
            v = (255 * (x + y)) // (W + H - 2)
            if depth == 8:
                row.append(v)
            else:
                row += struct.pack(">H", v * 257)
        rows.append(bytes(row))
    return rows


def alpha_at(x: int, y: int) -> int:
    """A deliberately NON-binary alpha ramp.

    Binary (0/255) alpha would round-trip through a colour-key `/Mask` as
    well as through an `/SMask`, so a fixture using it could not tell the
    two apart. Intermediate values can only be carried by a soft mask.
    """
    return (255 * (x * H + y)) // (W * H - 1)


def rgba_rows(depth: int = 8) -> list[bytes]:
    rows = []
    for y in range(H):
        row = bytearray()
        for x in range(W):
            r = 255 * x // (W - 1)
            g = 255 * y // (H - 1)
            b = 255 - r
            a = alpha_at(x, y)
            if depth == 8:
                row += bytes([r, g, b, a])
            else:
                row += struct.pack(">HHHH", r * 257, g * 257, b * 257, a * 257)
        rows.append(bytes(row))
    return rows


def graya_rows() -> list[bytes]:
    rows = []
    for y in range(H):
        row = bytearray()
        for x in range(W):
            row += bytes([(255 * (x + y)) // (W + H - 2), alpha_at(x, y)])
        rows.append(bytes(row))
    return rows


PALETTE = bytes(
    [
        0, 0, 0,        # 0 black
        255, 0, 0,      # 1 red
        0, 255, 0,      # 2 green
        0, 0, 255,      # 3 blue
        255, 255, 0,    # 4 yellow
        255, 255, 255,  # 5 white
    ]
)


def index_at(x: int, y: int) -> int:
    return (x + 2 * y) % 6


def indexed8_rows() -> list[bytes]:
    return [bytes(index_at(x, y) for x in range(W)) for y in range(H)]


def indexed4_rows() -> list[bytes]:
    """4-bit indices, two per byte, high nibble first (RFC 2083 §7.2)."""
    rows = []
    for y in range(H):
        row = bytearray()
        for x in range(0, W, 2):
            hi = index_at(x, y)
            lo = index_at(x + 1, y) if x + 1 < W else 0
            row.append((hi << 4) | lo)
        rows.append(bytes(row))
    return rows


# ---------------------------------------------------------------------------
# BMP writer (Windows BITMAPINFOHEADER, BI_RGB)
# ---------------------------------------------------------------------------


def bmp(
    width: int,
    height: int,
    bpp: int,
    pixel_rows_bottom_up: list[bytes],
    *,
    palette: bytes = b"",
    top_down: bool = False,
    compression: int = 0,
) -> bytes:
    """A BITMAPFILEHEADER + BITMAPINFOHEADER BMP.

    Rows are given BOTTOM-UP (BMP's own storage order). `top_down=True`
    negates the header height and reverses them, which is the encoding a
    reader that ignores the sign silently renders mirrored.
    """
    rows = list(reversed(pixel_rows_bottom_up)) if top_down else pixel_rows_bottom_up
    stride = ((width * bpp + 31) // 32) * 4
    body = b"".join(r.ljust(stride, b"\x00") for r in rows)
    offset = 14 + 40 + len(palette)
    info = struct.pack(
        "<IiiHHIIiiII",
        40,                              # biSize
        width,
        -height if top_down else height,
        1,                               # biPlanes
        bpp,
        compression,
        len(body),
        2835,                            # biXPelsPerMeter ≈ 72 dpi
        2835,                            # biYPelsPerMeter
        len(palette) // 4,               # biClrUsed
        0,                               # biClrImportant
    )
    return (
        b"BM"
        + struct.pack("<IHHI", offset + len(body), 0, 0, offset)
        + info
        + palette
        + body
    )


def bgr24_rows_bottom_up() -> list[bytes]:
    """The same ramp as `rgb_rows`, in BMP's BGR order, bottom row first."""
    rows = []
    for y in reversed(range(H)):
        row = bytearray()
        for x in range(W):
            r = 255 * x // (W - 1)
            g = 255 * y // (H - 1)
            b = 255 - r
            row += bytes([b, g, r])
        rows.append(bytes(row))
    return rows


def bgra32_rows_bottom_up() -> list[bytes]:
    rows = []
    for y in reversed(range(H)):
        row = bytearray()
        for x in range(W):
            r = 255 * x // (W - 1)
            g = 255 * y // (H - 1)
            b = 255 - r
            # The 4th byte is DELIBERATELY not 255: a BITMAPINFOHEADER
            # 32-bit BMP has no alpha channel, and a reader that treats
            # this byte as opacity would make the whole image transparent.
            row += bytes([b, g, r, 0])
        rows.append(bytes(row))
    return rows


def pal8_rows_bottom_up() -> list[bytes]:
    return [bytes(index_at(x, y) for x in range(W)) for y in reversed(range(H))]


BMP_PALETTE = b"".join(
    bytes([PALETTE[i * 3 + 2], PALETTE[i * 3 + 1], PALETTE[i * 3], 0])
    for i in range(6)
) + bytes(4 * (256 - 6))


# ---------------------------------------------------------------------------
# JPEG (Pillow) + the two patched variants no encoder will produce
# ---------------------------------------------------------------------------


def jpeg_fixtures() -> dict[str, bytes]:
    import io

    from PIL import Image

    def encode(img, **kw) -> bytes:
        buf = io.BytesIO()
        img.save(buf, format="JPEG", **kw)
        return buf.getvalue()

    rgb = Image.new("RGB", (W, H))
    rgb.putdata(
        [
            (255 * x // (W - 1), 255 * y // (H - 1), 255 - 255 * x // (W - 1))
            for y in range(H)
            for x in range(W)
        ]
    )
    gray = rgb.convert("L")
    cmyk = rgb.convert("CMYK")

    out = {
        "rgb.jpg": encode(rgb, quality=90),
        "gray.jpg": encode(gray, quality=90),
        "progressive.jpg": encode(rgb, quality=90, progressive=True),
        "cmyk.jpg": encode(cmyk, quality=90),
        "exif-rot90.jpg": encode(rgb, quality=90, exif=_exif_orientation(6)),
    }

    # SOF9 = arithmetic-coded. Patched rather than encoded: libjpeg ships
    # with arithmetic coding disabled by default and Pillow exposes no
    # switch for it, so the ONLY way to get a fixture for the refusal is to
    # rewrite the frame marker. The entropy-coded data is then nonsense for
    # an arithmetic decoder — which does not matter, because pdfce must
    # refuse it at the marker walk, BEFORE any decoder sees it. That is
    # precisely the property under test.
    out["arithmetic.jpg"] = _patch_sof(out["rgb.jpg"], 0xC9)
    return out


def _exif_orientation(value: int) -> bytes:
    """A minimal APP1/Exif payload carrying only IFD0 tag 0x0112.

    Little-endian TIFF, one IFD entry, no next-IFD. Written by hand rather
    than via `PIL.Image.Exif` so the fixture's bytes are exactly what this
    project chose — the same reasoning as the hand-written PNGs.
    """
    ifd = struct.pack("<H", 1)                        # entry count
    ifd += struct.pack("<HHIHH", 0x0112, 3, 1, value, 0)  # SHORT, count 1
    ifd += struct.pack("<I", 0)                       # no next IFD
    tiff = b"II" + struct.pack("<HI", 42, 8) + ifd
    return b"Exif\x00\x00" + tiff


def _patch_sof(data: bytes, marker: int) -> bytes:
    """Rewrite the first SOF0 marker byte, preserving every byte offset."""
    i = 2
    while i + 1 < len(data):
        if data[i] != 0xFF:
            i += 1
            continue
        m = data[i + 1]
        if m == 0xC0:
            return data[: i + 1] + bytes([marker]) + data[i + 2 :]
        if m in (0xD8, 0x01) or 0xD0 <= m <= 0xD7:
            i += 2
            continue
        length = struct.unpack(">H", data[i + 2 : i + 4])[0]
        i += 2 + length
    raise ValueError("no SOF0 marker found")


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    files: dict[str, bytes] = {}

    # --- PNG, passthrough branch ---------------------------------------
    # Mixed per-row filter tags on the flagship fixture: a passthrough that
    # rewrote the rows would have to re-choose them, so identical tags are
    # the cheapest proof that the IDAT bytes were not touched.
    files["rgb8.png"] = png(W, H, 2, 8, rgb_rows(), filter_types=[0, 1, 2, 4])
    files["gray8.png"] = png(W, H, 0, 8, gray_rows(), filter_types=[0, 3, 1, 2])
    files["gray16.png"] = png(W, H, 0, 16, gray_rows(16))
    files["indexed8.png"] = png(W, H, 3, 8, indexed8_rows(), palette=PALETTE)
    files["indexed4.png"] = png(W, H, 3, 4, indexed4_rows(), palette=PALETTE)
    # tRNS on truecolour: ONE transparent colour, given as 16-bit values
    # even at bit depth 8 (RFC 2083 §4.2.4 — the sample values are always
    # stored as two bytes each). Here: pure blue (0, 0, 255).
    files["rgb-trns.png"] = png(
        W, H, 2, 8, rgb_rows(), trns=struct.pack(">HHH", 0, 0, 255)
    )
    # tRNS on palette: per-entry alpha, INTERMEDIATE values, so it can only
    # be carried by a soft mask (entries past the end default to opaque).
    files["indexed-trns.png"] = png(
        W, H, 3, 8, indexed8_rows(), palette=PALETTE, trns=bytes([0, 64, 128, 255])
    )

    # --- PNG, decode-and-recompress branch -----------------------------
    files["rgba8.png"] = png(W, H, 6, 8, rgba_rows(), filter_types=[0, 1, 2, 4])
    files["graya8.png"] = png(W, H, 4, 8, graya_rows())
    files["rgba16.png"] = png(W, H, 6, 16, rgba_rows(16))

    # --- PNG, refusal ---------------------------------------------------
    # Adam7. Only pass 1 of 7 carries data at this size, which is legal and
    # is exactly the shape a naive "just treat it as non-interlaced" reader
    # would silently mis-size.
    files["interlaced.png"] = _adam7(W, H, rgb_rows())

    # --- BMP ------------------------------------------------------------
    files["rgb24.bmp"] = bmp(W, H, 24, bgr24_rows_bottom_up())
    files["rgb32.bmp"] = bmp(W, H, 32, bgra32_rows_bottom_up())
    files["pal8.bmp"] = bmp(W, H, 8, pal8_rows_bottom_up(), palette=BMP_PALETTE)
    files["topdown24.bmp"] = bmp(W, H, 24, bgr24_rows_bottom_up(), top_down=True)
    files["rle8.bmp"] = bmp(
        W, H, 8, pal8_rows_bottom_up(), palette=BMP_PALETTE, compression=1
    )

    # --- not an image ---------------------------------------------------
    files["not-an-image.bin"] = b"This is not an image. It is a sentence.\n"

    # --- JPEG -----------------------------------------------------------
    files.update(jpeg_fixtures())

    for name, data in sorted(files.items()):
        (OUT / name).write_bytes(data)
        print(f"{name:24} {len(data):>7} bytes")


def _adam7(width: int, height: int, rows: list[bytes]) -> bytes:
    """An interlaced (Adam7) PNG, assembled pass by pass (RFC 2083 §2.6).

    pdfce REFUSES this by name, so the file only has to be a legitimate
    interlaced PNG — but it is built properly rather than faked, so that if
    a future Pass adds a de-interlacer the fixture is already a real test
    of it rather than a file that merely says `interlace 1`.
    """
    starts = [(0, 0, 8, 8), (4, 0, 8, 8), (0, 4, 4, 8), (2, 0, 4, 4),
              (0, 2, 2, 4), (1, 0, 2, 2), (0, 1, 1, 2)]
    filtered = bytearray()
    for x0, y0, dx, dy in starts:
        pw = (width - x0 + dx - 1) // dx
        ph = (height - y0 + dy - 1) // dy
        if pw == 0 or ph == 0:
            continue
        prior = bytes(pw * 3)
        for py in range(ph):
            y = y0 + py * dy
            raw = bytearray()
            for px in range(pw):
                x = x0 + px * dx
                raw += rows[y][x * 3 : x * 3 + 3]
            filtered.append(0)
            filtered.extend(raw)
            prior = bytes(raw)
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 1)
    return (
        b"\x89PNG\r\n\x1a\n"
        + png_chunk(b"IHDR", ihdr)
        + png_chunk(b"IDAT", zlib.compress(bytes(filtered), 9))
        + png_chunk(b"IEND", b"")
    )


if __name__ == "__main__":
    main()
