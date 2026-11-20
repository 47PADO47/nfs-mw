#!/usr/bin/env python3
"""Read the Fairhaven world of NFS: Most Wanted (2012): units, instances, collision, statistics, glTF export.

Facts and layouts: docs/formats/chameleon-world.md. Standard library only (Python 3.10+).
Nothing read here is stored in the repository; the exporter writes to a folder you give it.

    python tools/chameleon_world.py stats  <game dir> [--units 1,2,3]     # counts, sizes, overall bounding box
    python tools/chameleon_world.py export <game dir> <out.glb> <unit>... # one or more units as a glb (LOD 0)
"""

import glob
import json
import os
import struct
import sys
from collections import Counter

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from chameleon_bundle import Bundle, type_name  # noqa: E402

T_TEXTURE, T_MATERIAL, T_VDESC, T_RENDERABLE = 0x1, 0x2, 0x3, 0x5
T_INSTANCES, T_MODEL, T_SHADER, T_POLYSOUP, T_NAVMESH = 0x50, 0x51, 0x53, 0x60, 0x68
T_ZONEHEADER, T_PROPINST, T_DYNINST, T_LIGHTINST, T_COMPINST = 0x206, 0x218, 0x204, 0x213, 0x216
T_GROUNDCOVER_INST = 0x214


def u32(d, o):
    return struct.unpack_from("<I", d, o)[0]


def f32s(d, o, n):
    return struct.unpack_from("<%df" % n, d, o)


class Instance:
    __slots__ = ("model", "matrix", "extra")


def parse_instance_list(b, e):
    """InstanceList (0x50): header {u32 first, u32 count, u32 params, u32 3}, `count` records of 0x60 bytes.

    Record: u32 model import slot, f32 1.0, u32 id, u32 0, 16 bytes of zero, then (at +0x20) a 4x4 float matrix whose last
    row is the translation (x left, y up, z forward).
    """
    main = b.block(e, 0)
    first, count = u32(main, 0), u32(main, 4)
    slots = {off: rid for rid, off, _ in b.imports(e, main)}
    out = []
    for n in range(count):
        o = first + 0x60 * n
        inst = Instance()
        inst.model = slots.get(o)
        inst.matrix = f32s(main, o + 0x20, 16)
        inst.extra = u32(main, o + 8)
        out.append(inst)
    return out


class World:
    """Index of every resource id of the HAWAII bundles and the shader bundles (no pixel data is read until asked)."""

    def __init__(self, game):
        self.game = game
        self.hawaii = os.path.join(game, "HAWAII")
        self.cache = {}

    def unit_path(self, n):
        return os.path.join(self.hawaii, "TRK_UNIT%d.BNDL" % n)

    def units(self):
        out = []
        for p in glob.glob(os.path.join(self.hawaii, "TRK_UNIT*.BNDL")):
            out.append(int(os.path.basename(p)[8:-5]))
        return sorted(out)

    def bundle(self, path):
        if path not in self.cache:
            if len(self.cache) > 6:
                self.cache.clear()
            self.cache[path] = Bundle(path)
        return self.cache[path]


def unit_stats(world, n):
    b = Bundle(world.unit_path(n))
    s = {"unit": n, "bytes": len(b.data)}
    types = Counter(type_name(e.type) for e in b.entries)
    s["types"] = dict(types)
    insts = []
    for e in b.by_type(T_INSTANCES):
        insts += parse_instance_list(b, e)
    s["instances"] = len(insts)
    xs = [i.matrix[12] for i in insts]
    ys = [i.matrix[13] for i in insts]
    zs = [i.matrix[14] for i in insts]
    if insts:
        s["bbox"] = [min(xs), min(ys), min(zs), max(xs), max(ys), max(zs)]
    models = {i.model for i in insts}
    s["distinct_models"] = len(models)
    for t, name in ((T_PROPINST, "prop_instance_lists"), (T_DYNINST, "dynamic_instance_lists"),
                    (T_COMPINST, "compound_instance_lists"), (T_LIGHTINST, "light_instance_lists")):
        s[name] = len(b.by_type(t))
    return s


def cmd_stats(argv):
    game = argv[0]
    units = None
    if "--units" in argv:
        units = [int(x) for x in argv[argv.index("--units") + 1].split(",")]
    w = World(game)
    rows = [unit_stats(w, n) for n in (units or w.units())]
    inst = sum(r["instances"] for r in rows)
    print("units", len(rows), "instances", inst, "bytes", sum(r["bytes"] for r in rows))
    boxes = [r["bbox"] for r in rows if "bbox" in r]
    if boxes:
        print("bbox x %.0f..%.0f  y %.0f..%.0f  z %.0f..%.0f" % (
            min(b[0] for b in boxes), max(b[3] for b in boxes),
            min(b[1] for b in boxes), max(b[4] for b in boxes),
            min(b[2] for b in boxes), max(b[5] for b in boxes)))
    for r in rows:
        print(json.dumps(r))


def cmd_export(argv):
    from chameleon_export import export_glb
    game, out = argv[0], argv[1]
    units = [int(x) for x in argv[2:]]
    export_glb(World(game), units, out)


if __name__ == "__main__":
    if len(sys.argv) < 3:
        print(__doc__)
        sys.exit(1)
    {"stats": cmd_stats, "export": cmd_export}[sys.argv[1]](sys.argv[2:])
