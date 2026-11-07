#!/usr/bin/env python3
"""Decode the world collision (PolySoupList, 0x60) of a Fairhaven unit; same chunk format as a car's soup.

    python tools/chameleon_collision.py stats <game dir>                 # triangles, quads, flag words of every unit
    python tools/chameleon_collision.py obj <game dir> <unit> <out.obj>  # one unit's collision as a Wavefront obj

Format: docs/formats/chameleon-world.md#collision. Standard library only.
"""

import glob
import os
import struct
import sys
from collections import Counter

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from chameleon_bundle import Bundle  # noqa: E402

CELL = 500.0 / 32768.0


def read_soup_list(main):
    """Yield (vertices [(x, y, z)], faces [(flags_u32, [indices])]) per chunk, in world metres."""
    count = struct.unpack_from("<I", main, 0x28)[0]
    table = struct.unpack_from("<I", main, 0x20)[0]
    for k in range(count):
        o = struct.unpack_from("<I", main, table + 4 * k)[0]
        fo, vo, _size = struct.unpack_from("<IIH", main, o)
        flags = struct.unpack_from("<3b", main, o + 0x0A)
        nf, nv = main[o + 0x0E], main[o + 0x0F]
        verts = []
        for i in range(nv):
            q = struct.unpack_from("<3H", main, vo + 6 * i)
            verts.append(tuple(q[a] * CELL + 500.0 * flags[a] for a in range(3)))
        faces = []
        for i in range(nf):
            f = fo + 12 * i
            data = struct.unpack_from("<I", main, f)[0]
            idx = [x for x in main[f + 4:f + 8] if x != 0xFF]
            faces.append((data, idx))
        yield verts, faces


def stats(game):
    tris = quads = soups = chunks = 0
    flags = Counter()
    for p in sorted(glob.glob(os.path.join(game, "HAWAII", "TRK_UNIT*.BNDL"))):
        b = Bundle(p)
        for e in b.by_type(0x60):
            main = b.block(e, 0)
            for verts, faces in read_soup_list(main):
                chunks += 1
                for data, idx in faces:
                    flags[data] += 1
                    if len(idx) == 3:
                        tris += 1
                    elif len(idx) == 4:
                        quads += 1
            soups += 1
    print("units soups", soups, "chunks", chunks, "triangles", tris, "quads", quads, "-> triangles total", tris + 2 * quads)
    print("distinct flag words", len(flags))
    for w, n in flags.most_common(12):
        print("  0x%08X %d" % (w, n))


def to_obj(game, unit, out):
    b = Bundle(os.path.join(game, "HAWAII", "TRK_UNIT%d.BNDL" % unit))
    base = 1
    with open(out, "w") as f:
        for e in b.by_type(0x60):
            for verts, faces in read_soup_list(b.block(e, 0)):
                for v in verts:
                    f.write("v %.3f %.3f %.3f\n" % v)
                for data, idx in faces:
                    if len(idx) >= 3:
                        f.write("f " + " ".join(str(base + i) for i in idx) + "\n")
                base += len(verts)
    print("wrote", out)


if __name__ == "__main__":
    a = sys.argv
    if len(a) >= 3 and a[1] == "stats":
        stats(a[2])
    elif len(a) >= 5 and a[1] == "obj":
        to_obj(a[2], int(a[3]), a[4])
    else:
        print(__doc__)
