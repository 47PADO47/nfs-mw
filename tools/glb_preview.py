#!/usr/bin/env python3
"""Tiny software preview of a .glb (painter's algorithm, flat colour per triangle from the base colour texture).

    python tools/glb_preview.py <in.glb> <out.png> [--top | --azimuth DEG --elevation DEG] [--size 1600x1000]

A check that an export is sane, not a renderer. Needs numpy and Pillow.
"""

import io
import json
import math
import struct
import sys

import numpy as np
from PIL import Image, ImageDraw


def load_glb(path):
    d = open(path, "rb").read()
    jl = struct.unpack_from("<I", d, 12)[0]
    doc = json.loads(d[20:20 + jl])
    bin_off = 20 + jl + 8
    return doc, d[bin_off:]


def accessor(doc, binary, i):
    a = doc["accessors"][i]
    v = doc["bufferViews"][a["bufferView"]]
    comp = {5126: "<f4", 5125: "<u4", 5123: "<u2", 5121: "u1"}[a["componentType"]]
    n = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}[a["type"]]
    base = v.get("byteOffset", 0) + a.get("byteOffset", 0)
    item = np.dtype(comp).itemsize * n
    stride = v.get("byteStride", item)
    if stride == item:
        arr = np.frombuffer(binary, dtype=comp, count=a["count"] * n, offset=base)
        return arr.reshape(-1, n) if n > 1 else arr
    raw = np.frombuffer(binary, dtype=np.uint8, count=stride * a["count"], offset=base).reshape(a["count"], stride)
    arr = np.ascontiguousarray(raw[:, :item]).view(comp).reshape(a["count"], n)
    return arr if n > 1 else arr.reshape(-1)


def mean_colour(doc, binary, mat):
    pbr = doc["materials"][mat]["pbrMetallicRoughness"]
    tex = pbr.get("baseColorTexture")
    if tex is None:
        return (160, 160, 160)
    img = doc["images"][doc["textures"][tex["index"]]["source"]]
    v = doc["bufferViews"][img["bufferView"]]
    im = Image.open(io.BytesIO(binary[v.get("byteOffset", 0):v.get("byteOffset", 0) + v["byteLength"]])).convert("RGB")
    return tuple(int(c) for c in np.asarray(im.resize((8, 8))).reshape(-1, 3).mean(0))


def render(path, out, top=False, azimuth=35.0, elevation=40.0, size=(1600, 1000)):
    doc, binary = load_glb(path)
    tris, cols = [], []
    for prim in doc["meshes"][0]["primitives"]:
        pos = accessor(doc, binary, prim["attributes"]["POSITION"]).astype(np.float64)
        idx = accessor(doc, binary, prim["indices"]).reshape(-1, 3)
        colour = mean_colour(doc, binary, prim["material"])
        tris.append(pos[idx])
        cols.append(np.tile(colour, (len(idx), 1)))
    tris = np.concatenate(tris)
    cols = np.concatenate(cols)
    centre = (tris.reshape(-1, 3).min(0) + tris.reshape(-1, 3).max(0)) / 2
    extent = (tris.reshape(-1, 3).max(0) - tris.reshape(-1, 3).min(0))
    p = tris - centre
    # engine space: x left, y up, z forward. View from azimuth around y, elevation above the ground.
    az, el = math.radians(azimuth), math.radians(90.0 if top else elevation)
    ca, sa, ce, se = math.cos(az), math.sin(az), math.cos(el), math.sin(el)
    x1 = p[..., 0] * ca - p[..., 2] * sa
    z1 = p[..., 0] * sa + p[..., 2] * ca
    y2 = p[..., 1] * ce - z1 * se
    depth = p[..., 1] * se + z1 * ce
    scale = 0.9 * min(size[0] / max(extent[0], extent[2], 1.0), size[1] / max(extent[0], extent[2], 1.0) / (1.0 if top else 0.8))
    sx = size[0] / 2 - x1 * scale
    sy = size[1] / 2 - y2 * scale
    # light from above-left, using the triangle normal
    n = np.cross(tris[:, 1] - tris[:, 0], tris[:, 2] - tris[:, 0])
    n /= np.maximum(np.linalg.norm(n, axis=1, keepdims=True), 1e-9)
    shade = np.clip(0.55 + 0.45 * np.abs(n[:, 1]) * 0.8 + 0.2 * np.abs(n[:, 0]), 0.3, 1.0)
    order = np.argsort(-depth.mean(1))
    img = Image.new("RGB", size, (150, 190, 230))
    dr = ImageDraw.Draw(img)
    for t in order:
        c = tuple(int(v) for v in np.clip(cols[t] * shade[t], 0, 255))
        dr.polygon([(sx[t, 0], sy[t, 0]), (sx[t, 1], sy[t, 1]), (sx[t, 2], sy[t, 2])], fill=c)
    img.save(out)
    print("wrote", out, "triangles", len(tris))


if __name__ == "__main__":
    a = sys.argv
    if len(a) < 3:
        print(__doc__)
        sys.exit(1)
    size = (1600, 1000)
    if "--size" in a:
        w, h = a[a.index("--size") + 1].split("x")
        size = (int(w), int(h))
    az = float(a[a.index("--azimuth") + 1]) if "--azimuth" in a else 35.0
    el = float(a[a.index("--elevation") + 1]) if "--elevation" in a else 40.0
    render(a[1], a[2], top="--top" in a, azimuth=az, elevation=el, size=size)
