"""Writing a track pack (`docs/formats/ride6-track-pack.md`): `pack.json`, `ground.r6m` and the pictures in `tex/`."""

import json
import os
import struct

import common  # noqa: F401  (first: it makes the user site-packages importable)
import numpy as np

MAGIC = b"R6TM"
VERSION = 1


class Ground:
    """The triangle groups of a pack, one per drawn material, written to one binary file."""

    def __init__(self):
        self.groups = []  # (material index, collide, positions, normals, uv, indices)

    def add(self, material, collide, positions, normals, uv, indices, model=0):
        if len(indices) == 0:
            return
        self.groups.append((material, collide, positions, normals, uv, indices, model))

    def triangles(self):
        return sum(len(g[5]) for g in self.groups) // 3

    def write(self, path):
        with open(path, "wb") as f:
            f.write(MAGIC + struct.pack("<II", VERSION, len(self.groups)))
            for material, collide, positions, _normals, _uv, indices, model in self.groups:
                flags = (1 if collide else 0) | (model << 8)
                f.write(struct.pack("<IIII", material, len(positions), len(indices), flags))
            for _material, _collide, positions, normals, uv, indices, _model in self.groups:
                f.write(positions.astype("<f4").tobytes())
                f.write(uv.astype("<f4").tobytes())
                packed = np.zeros((len(normals), 4), dtype=np.int8)
                packed[:, :3] = np.round(np.clip(normals, -1, 1) * 127)
                f.write(packed.tobytes())
                f.write(indices.astype("<u4").tobytes())


def write_json(path, document):
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        json.dump(document, f, indent=1)
        f.write("\n")


def pack_size(folder):
    total = 0
    for root, _dirs, files in os.walk(folder):
        total += sum(os.path.getsize(os.path.join(root, name)) for name in files)
    return total
