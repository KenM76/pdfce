# `fixtures/synthetic/signature/` — provenance

**Category (a) under `docs/LEGAL.md` §5: wholly synthetic, byte-authored
by a committed script.** No downloaded PDF of any provenance is involved,
and no PDF library produced these bytes — so a fixture cannot inherit a
bug or a normalisation from the code it tests (project rule 7).

Generator: `tools/gen-signature-fixtures.py`. Regenerate with
`python tools/gen-signature-fixtures.py` from the repository root.

## What each file is for

| File | Shape | Exercises |
|---|---|---|
| `signed-full-coverage.pdf` | `/ByteRange` reaches the last byte, two pairs straddling `/Contents` | the good case; `uncovered_tail == 0` |
| `signed-short-coverage.pdf` | second pair stops 200 bytes early | §12.8.1's `should` — **conforming but under-protecting**, and must NOT report as malformed |
| `signed-malformed-range.pdf` | second pair starts before the first ends | Table 252's "exact byte range" — overlap IS malformed |

The last two exist as a pair on purpose. A reader that reported "short"
and "overlapping" the same way would be wrong about one of them, and
either test alone would pass against a constant.

## No cryptography, and none claimed

`/Contents` is filler. These fixtures are for the COVERAGE measurement —
arithmetic over byte offsets — which never inspects the signature value.
Nothing here can be used to test signature VALIDITY, and
`signature::byte_range_coverage` does not claim to measure it.

## Why the byte ranges are computed, not written

The offsets are positions in the finished file, so the generator lays the
file out once with a fixed-width placeholder, measures the real
`/Contents` hole, and overwrites the placeholder in place — the same
order a real signer works in (§12.8.3.3). Round numbers would let an
off-by-one in the straddle arithmetic pass unnoticed.
