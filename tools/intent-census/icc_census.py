"""How many DISTINCT ICC source profiles does one document reference?

Answers the `source` half of iccce's question 2.3.  Counts distinct object
references appearing as `/ICCBased <n> 0 R` (§8.6.5.5) plus distinct
`/DestOutputProfile <n> 0 R` (§14.11.5), across the raw file and every
FlateDecode stream that inflates.  Object-reference identity is a LOWER bound
on distinct profiles (two objects may hold identical bytes) and an UPPER bound
on cache keys if the cache is keyed by object id -- both directions stated
because neither is the number on its own.
"""
import os, re, sys, zlib
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ri_census import inflate_all

ICC = re.compile(rb"/ICCBased\s+(\d+)\s+(\d+)\s+R")
DOP = re.compile(rb"/DestOutputProfile\s+(\d+)\s+(\d+)\s+R")
OI  = re.compile(rb"/OutputIntents")

root = sys.argv[1]
rows = []
for dirpath, _, names in os.walk(root):
    for n in sorted(names):
        if not n.lower().endswith(".pdf"):
            continue
        p = os.path.join(dirpath, n)
        buf = open(p, "rb").read()
        blobs = [buf] + list(inflate_all(buf))
        icc = set(); dop = set(); oi = 0
        for b in blobs:
            icc |= {m.group(0) for m in ICC.finditer(b)}
            dop |= {m.group(0) for m in DOP.finditer(b)}
            oi += len(OI.findall(b))
        rows.append((n, len(icc), len(dop), oi))
print("file\tdistinct_ICCBased_refs\tdistinct_DestOutputProfile_refs\tOutputIntents_mentions")
for r in rows:
    print("\t".join(str(x) for x in r))
import statistics
icc = [r[1] for r in rows]; dop = [r[2] for r in rows]
print(f"\nfiles={len(rows)}  ICCBased: max={max(icc)} mean={statistics.mean(icc):.2f} nonzero={sum(1 for x in icc if x)}")
print(f"DestOutputProfile: max={max(dop)} nonzero={sum(1 for x in dop if x)}")
