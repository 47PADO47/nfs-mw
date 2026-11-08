"""Triangle counts of the exporter's `.glb` meshes, for `completeness.py` (a reader of the glTF header and accessors only)."""

import json
import os
import struct

_cache = {}


def index_glbs(folder):
    """The `.glb` files below `folder/glb`, by lower-case file name."""
    found = {}
    for root, _, names in os.walk(os.path.join(folder, "glb")):
        for name in names:
            if name.lower().endswith(".glb"):
                found[name.lower()] = os.path.join(root, name)
    return found


def read_header(path):
    """The JSON chunk of a binary glTF file."""
    with open(path, "rb") as f:
        data = f.read(1 << 20)
    magic, _, _ = struct.unpack_from("<III", data, 0)
    if magic != 0x46546C67:
        raise ValueError(f"{path}: not a glb file")
    length, kind = struct.unpack_from("<II", data, 12)
    if kind != 0x4E4F534A:
        raise ValueError(f"{path}: no JSON chunk first")
    if 20 + length > len(data):
        with open(path, "rb") as f:
            data = f.read(20 + length)
    return json.loads(data[20:20 + length])


def primitives(path):
    """`[(material name, vertices, triangles)]` of a glb file."""
    if path in _cache:
        return _cache[path]
    doc = read_header(path)
    out = []
    for mesh in doc.get("meshes", []):
        for prim in mesh["primitives"]:
            accessors = doc["accessors"]
            vertices = accessors[prim["attributes"]["POSITION"]]["count"]
            triangles = accessors[prim["indices"]]["count"] // 3 if "indices" in prim else 0
            material = doc["materials"][prim["material"]]["name"] if "material" in prim else ""
            out.append((material, vertices, triangles))
    _cache[path] = out
    return out


def level_file(glbs, stem, level):
    """The file of `stem` at `level`, else the finest level below it that exists (what the importer reads)."""
    for n in range(level, -1, -1):
        name = f"{stem}.glb" if n == 0 else f"{stem}_lod{n}.glb"
        if name.lower() in glbs:
            return glbs[name.lower()]
    return None


def glb_tris(glbs, stem, level, decals_only=False):
    """The triangles of a part at a level of detail without its glass decals, which the importer does not draw (only them with
    `decals_only`)."""
    path = level_file(glbs, stem, level)
    if path is None:
        return 0
    return sum(t for material, _, t in primitives(path) if ("glassdecal" in material.lower()) == decals_only)
