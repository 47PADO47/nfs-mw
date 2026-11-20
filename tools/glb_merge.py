#!/usr/bin/env python3
"""Merge several .glb files (meshes only, node transforms kept) into one scene.

    python tools/glb_merge.py <out.glb> <in.glb>...

For looking at a RIDE 6 track in a viewer: the exporter writes one glb per static mesh. Materials are not exported, so a
neutral material is used. Standard library only.
"""

import json
import struct
import sys


def read(path):
    d = open(path, "rb").read()
    jl = struct.unpack_from("<I", d, 12)[0]
    return json.loads(d[20:20 + jl]), d[20 + jl + 8:]


def main(out, paths):
    doc = {"asset": {"version": "2.0"}, "scene": 0, "scenes": [{"nodes": []}], "nodes": [], "meshes": [],
           "materials": [{"pbrMetallicRoughness": {"baseColorFactor": [0.55, 0.6, 0.5, 1], "metallicFactor": 0, "roughnessFactor": 1},
                          "doubleSided": True}],
           "accessors": [], "bufferViews": [], "buffers": [{"byteLength": 0}]}
    blob = bytearray()
    for path in paths:
        g, b = read(path)
        base_view, base_acc, base_mesh = len(doc["bufferViews"]), len(doc["accessors"]), len(doc["meshes"])
        pad = -len(blob) % 4
        blob.extend(b"\0" * pad)
        shift = len(blob)
        blob.extend(b)
        for v in g["bufferViews"]:
            v = dict(v)
            v["buffer"] = 0
            v["byteOffset"] = v.get("byteOffset", 0) + shift
            doc["bufferViews"].append(v)
        for a in g["accessors"]:
            a = dict(a)
            a["bufferView"] += base_view
            doc["accessors"].append(a)
        for m in g["meshes"]:
            prims = []
            for p in m["primitives"]:
                p = {"attributes": {k: v + base_acc for k, v in p["attributes"].items() if k in ("POSITION", "NORMAL", "TEXCOORD_0")},
                     "indices": p["indices"] + base_acc, "material": 0}
                prims.append(p)
            doc["meshes"].append({"primitives": prims})
        # nodes: copy the first scene's nodes, remapping mesh and children
        base_node = len(doc["nodes"])
        for n in g["nodes"]:
            n = dict(n)
            if "mesh" in n:
                n["mesh"] += base_mesh
            if "children" in n:
                n["children"] = [c + base_node for c in n["children"]]
            n.pop("skin", None)
            doc["nodes"].append(n)
        roots = g["scenes"][g.get("scene", 0)]["nodes"]
        doc["scenes"][0]["nodes"] += [r + base_node for r in roots]
    doc["buffers"][0]["byteLength"] = len(blob)
    js = json.dumps(doc, separators=(",", ":")).encode()
    js += b" " * (-len(js) % 4)
    blob.extend(b"\0" * (-len(blob) % 4))
    with open(out, "wb") as f:
        f.write(struct.pack("<4sII", b"glTF", 2, 12 + 8 + len(js) + 8 + len(blob)))
        f.write(struct.pack("<I4s", len(js), b"JSON") + js)
        f.write(struct.pack("<I4s", len(blob), b"BIN\0") + bytes(blob))
    print("wrote", out, len(blob), "bytes of buffer,", len(doc["meshes"]), "meshes")


if __name__ == "__main__":
    if len(sys.argv) < 3:
        print(__doc__)
        sys.exit(1)
    main(sys.argv[1], sys.argv[2:])
