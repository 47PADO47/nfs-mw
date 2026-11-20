#!/usr/bin/env python3
"""Minimal reader for Chameleon `bnd2` bundles (NFS: Most Wanted 2012, PC, version 5).

Format: docs/formats/chameleon-bundle.md. Standard library only (Python 3.10+).

    python tools/chameleon_bundle.py list <file.BNDL>            # resources by type
    python tools/chameleon_bundle.py survey <folder> [glob]      # type counts over many bundles

The module is also imported by tools/chameleon_world.py.
"""

import glob
import os
import struct
import sys
import zlib
from collections import Counter

TYPE_NAMES = {
    0x1: "Texture", 0x2: "Material", 0x3: "VertexDescriptor", 0x4: "VertexProgramState",
    0x5: "Renderable", 0x6: "MaterialState", 0x7: "SamplerState", 0x8: "ShaderProgramBuffer",
    0x14: "GenesysType", 0x15: "GenesysObject", 0x30: "Font", 0x50: "InstanceList",
    0x51: "Model", 0x52: "ColourCube", 0x53: "Shader", 0x60: "PolySoupList",
    0x68: "NavigationMesh", 0x70: "TextFile", 0x80: "Ginsu", 0x81: "Wave",
    0x90: "ZoneList", 0x91: "WorldPaintMap", 0xB0: "AnimationList", 0xB2: "Skeleton",
    0xB3: "Animation", 0x105: "VehicleList", 0x106: "VehicleGraphicsSpec", 0x200: "AiData",
    0x201: "LocalisedText", 0x202: "TriggerData", 0x203: "RoadData", 0x204: "DynamicInstanceList",
    0x205: "WorldObject", 0x206: "ZoneHeader", 0x207: "VehicleSound", 0x20C: "ReverbRoadData",
    0x20F: "GroundcoverCollection", 0x213: "LightInstanceList", 0x214: "GroundcoverInstances",
    0x215: "CompoundObject", 0x216: "CompoundInstanceList", 0x217: "PropObject",
    0x218: "PropInstanceList", 0x219: "ZoneAmbienceList", 0x301: "BearEffect", 0x303: "ConvexHull",
    0x501: "HsmData", 0x701: "TrafficLaneData",
}


def type_name(t):
    return TYPE_NAMES.get(t, "0x%X" % t)


class Entry:
    __slots__ = ("id", "sizes", "disk", "offs", "import_off", "type", "import_count", "stream")

    def key(self):
        return self.id & 0xFFFFFFFF

    def klass(self):
        return self.id >> 56


class Bundle:
    def __init__(self, path):
        self.path = path
        with open(path, "rb") as f:
            self.data = f.read()
        d = self.data
        if d[:4] != b"bnd2":
            raise ValueError("not a bnd2 file: " + path)
        self.version, self.platform = struct.unpack_from("<HH", d, 4)
        self.debug_off, self.count, self.table_off = struct.unpack_from("<III", d, 8)
        self.block_start = struct.unpack_from("<4I", d, 0x14)
        self.flags = struct.unpack_from("<I", d, 0x24)[0]
        self.entries = []
        for i in range(self.count):
            o = self.table_off + i * 0x48
            e = Entry()
            e.id = struct.unpack_from("<Q", d, o)[0]
            e.sizes = struct.unpack_from("<4I", d, o + 0x08)
            e.disk = struct.unpack_from("<4I", d, o + 0x18)
            e.offs = struct.unpack_from("<4I", d, o + 0x28)
            e.import_off, e.type = struct.unpack_from("<II", d, o + 0x38)
            e.import_count = struct.unpack_from("<H", d, o + 0x40)[0]
            e.stream = d[o + 0x43]
            self.entries.append(e)

    def block(self, e, n):
        """Inflated bytes of block n (0 main, 1 graphics) of entry e."""
        size = e.sizes[n] & 0x0FFFFFFF
        disk = e.disk[n] & 0x0FFFFFFF
        if size == 0:
            return b""
        start = self.block_start[n] + e.offs[n]
        raw = self.data[start:start + (disk if self.flags & 1 else size)]
        if self.flags & 1:
            return zlib.decompress(raw)
        return raw

    def imports(self, e, main):
        out = []
        for i in range(e.import_count):
            rid, off, _ = struct.unpack_from("<QII", main, e.import_off + 16 * i)
            out.append((rid, off & 0x7FFFFFFF, off >> 31))
        return out

    def by_type(self, t):
        return [e for e in self.entries if e.type == t]


def survey(paths):
    total = Counter()
    bytes_by_type = Counter()
    for p in paths:
        b = Bundle(p)
        for e in b.entries:
            total[e.type] += 1
            bytes_by_type[e.type] += sum(s & 0x0FFFFFFF for s in e.sizes)
    for t, n in sorted(total.items()):
        print("%-24s %8d %14d" % (type_name(t), n, bytes_by_type[t]))


def main(argv):
    if len(argv) < 3:
        print(__doc__)
        return 1
    if argv[1] == "list":
        b = Bundle(argv[2])
        print("flags=0x%X entries=%d" % (b.flags, b.count))
        for e in b.entries:
            print("%016X %-22s sizes=%s imports=%d stream=%d" % (
                e.id, type_name(e.type), [s & 0x0FFFFFFF for s in e.sizes], e.import_count, e.stream))
        return 0
    if argv[1] == "survey":
        pattern = argv[3] if len(argv) > 3 else "*.BNDL"
        survey(sorted(glob.glob(os.path.join(argv[2], pattern))))
        return 0
    print(__doc__)
    return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
