#!/usr/bin/env python3
"""Census of the Fairhaven world (NFS: Most Wanted 2012): per unit counts and sizes, the PVS rectangles, an overhead map.

    python tools/chameleon_census.py <game dir> <out.json> [overhead.png]

Reads only bundle tables and small headers (no pixel data). Facts: docs/formats/chameleon-world.md. Needs numpy and
Pillow for the picture only. Nothing it reads is stored in the repository.
"""

import glob
import json
import os
import struct
import sys
from collections import Counter

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from chameleon_bundle import Bundle, type_name  # noqa: E402
from chameleon_export import SharedIndex, Resolver  # noqa: E402
from chameleon_world import (T_COMPINST, T_DYNINST, T_GROUNDCOVER_INST, T_INSTANCES, T_LIGHTINST, T_NAVMESH,  # noqa: E402
                             T_POLYSOUP, T_PROPINST, T_RENDERABLE, parse_instance_list, u32)

FLT_MAX = 3.4e38


def read_pvs(game):
    """HAWAII/PVS.BNDL ZoneList: header (8 u32, count at +0x10), entries of 0x50 bytes from 0x20."""
    b = Bundle(os.path.join(game, "HAWAII", "PVS.BNDL"))
    main = b.block(b.entries[0], 0)
    count = u32(main, 0x10)
    out = {}
    for k in range(count):
        o = 0x20 + 0x50 * k
        mn = struct.unpack_from("<4f", main, o + 0x10)
        mx = struct.unpack_from("<4f", main, o + 0x20)
        _, unit, district, _ = struct.unpack_from("<4I", main, o + 0x30)
        out[unit] = {"district": district, "x": [mn[0], mx[0]], "z": [mn[2], mx[2]]}
    return out


def lod0_renderable(res, model_id):
    found = res.get(model_id)
    if found is None:
        return None
    mb, me = found
    main = mb.block(me, 0)
    slot = u32(main, 0)
    return next((r for r, o, _ in mb.imports(me, main) if o == slot), None)


def renderable_info(res, rid):
    """(triangle-ish count, vertex bytes, bounding sphere) of a renderable, from its main block only."""
    found = res.get(rid)
    if found is None:
        return None
    b, e = found
    main = b.block(e, 0)
    sphere = struct.unpack_from("<4f", main, 0)
    count, table = struct.unpack_from("<HI", main, 0x12)[0], u32(main, 0x14)
    tris = vbytes = 0
    for k in range(count):
        mo = u32(main, table + 4 * k)
        icount = u32(main, mo + 0x1C)
        vinfo = u32(main, mo + 0x2C)
        tris += max(0, icount - 2)
        vbytes += u32(main, vinfo + 0x10)
    return tris, vbytes, sphere


def unit_census(shared, game, unit):
    b = Bundle(os.path.join(game, "HAWAII", "TRK_UNIT%d.BNDL" % unit))
    res = Resolver(shared, b)
    c = {"unit": unit, "file_bytes": len(b.data)}
    types = Counter(e.type for e in b.entries)
    c["resources"] = {type_name(t): n for t, n in sorted(types.items())}
    # sizes by block: block 0 main memory, block 1 graphics
    c["main_bytes"] = sum(e.sizes[0] & 0xFFFFFFF for e in b.entries)
    c["gfx_bytes"] = sum(e.sizes[1] & 0xFFFFFFF for e in b.entries)
    c["texture_gfx_bytes"] = sum(e.sizes[1] & 0xFFFFFFF for e in b.entries if e.type == 1)
    c["renderable_gfx_bytes"] = sum(e.sizes[1] & 0xFFFFFFF for e in b.entries if e.type == T_RENDERABLE)
    for t, name in ((T_PROPINST, "props"), (T_COMPINST, "compounds"), (T_DYNINST, "dynamics")):
        c[name] = sum(u32(b.block(e, 0), 8) for e in b.by_type(t))
    c["groundcover_instances"] = sum(u32(b.block(e, 0), 4) for e in b.by_type(T_GROUNDCOVER_INST))
    c["polysoup_bytes"] = sum(e.sizes[0] & 0xFFFFFFF for e in b.by_type(T_POLYSOUP))
    c["polysoups"] = sum(u32(b.block(e, 0), 0x28) for e in b.by_type(T_POLYSOUP))
    c["navmesh_bytes"] = sum(e.sizes[0] & 0xFFFFFFF for e in b.by_type(T_NAVMESH))
    insts = []
    for e in b.by_type(T_INSTANCES):
        insts += parse_instance_list(b, e)
    c["instances"] = len(insts)
    info = {}
    pts = []
    tris = 0
    for i in insts:
        if i.model not in info:
            rid = lod0_renderable(res, i.model)
            info[i.model] = renderable_info(res, rid) if rid else None
        r = info[i.model]
        if r is None:
            continue
        tris += r[0]
        m = i.matrix
        cx, cy, cz, rad = r[2]
        x = cx * m[0] + cy * m[4] + cz * m[8] + m[12]
        y = cx * m[1] + cy * m[5] + cz * m[9] + m[13]
        z = cx * m[2] + cy * m[6] + cz * m[10] + m[14]
        pts.append((x, y, z, rad))
    c["distinct_models"] = len(info)
    c["lod0_triangles"] = tris
    if pts:
        c["inst_bbox"] = [min(p[0] - p[3] for p in pts), min(p[1] - p[3] for p in pts), min(p[2] - p[3] for p in pts),
                          max(p[0] + p[3] for p in pts), max(p[1] + p[3] for p in pts), max(p[2] + p[3] for p in pts)]
        c["points"] = [(round(p[0]), round(p[2])) for p in pts]
    c["unresolved"] = sum(res.missing.values())
    return c


def main(argv):
    game, out = argv[0], argv[1]
    shared = SharedIndex(game)
    pvs = read_pvs(game)
    units = sorted(int(os.path.basename(p)[8:-5]) for p in glob.glob(os.path.join(game, "HAWAII", "TRK_UNIT*.BNDL")))
    rows = []
    for n in units:
        row = unit_census(shared, game, n)
        row["pvs"] = pvs.get(n)
        rows.append(row)
        print(n, row["instances"], row["lod0_triangles"], row["file_bytes"] // 1000000, "MB", flush=True)
    json.dump(rows, open(out, "w"))
    if len(argv) > 2:
        from chameleon_overhead import overhead
        overhead(rows, argv[2])


if __name__ == "__main__":
    if len(sys.argv) < 3:
        print(__doc__)
        sys.exit(1)
    main(sys.argv[1:])
