#!/usr/bin/env python3
"""Software preview of several .glb files at once, flat colour chosen by the file name (tracks of RIDE 6).

    python tools/glb_merge_preview.py <out.png> <name-rule=R,G,B>... -- <in.glb>... [--top] [--azimuth D] [--elevation D]

A name rule colours every file whose path contains the text. Reads only meshes; ignores node transforms.
Needs numpy and Pillow.
"""

import math
import sys

import numpy as np
from PIL import Image, ImageDraw

from glb_preview import accessor, load_glb


def triangles(path):
    doc, binary = load_glb(path)
    out = []
    for mesh in doc["meshes"]:
        for prim in mesh["primitives"]:
            if "indices" not in prim:
                continue
            pos = accessor(doc, binary, prim["attributes"]["POSITION"]).astype(np.float64)
            idx = accessor(doc, binary, prim["indices"]).reshape(-1, 3)
            out.append(pos[idx])
    return np.concatenate(out) if out else np.zeros((0, 3, 3))


def main(argv):
    out = argv[0]
    split = argv.index("--")
    rules = [(a.split("=")[0], tuple(int(v) for v in a.split("=")[1].split(","))) for a in argv[1:split]]
    rest = argv[split + 1:]
    files = [a for a in rest if a.endswith(".glb")]
    top = "--top" in rest
    az = float(rest[rest.index("--azimuth") + 1]) if "--azimuth" in rest else 30.0
    el = float(rest[rest.index("--elevation") + 1]) if "--elevation" in rest else 45.0
    tris, cols = [], []
    for f in files:
        t = triangles(f)
        colour = next((c for key, c in rules if key in f), (150, 150, 150))
        tris.append(t)
        cols.append(np.tile(colour, (len(t), 1)))
    tris = np.concatenate(tris)
    cols = np.concatenate(cols)
    lo, hi = tris.reshape(-1, 3).min(0), tris.reshape(-1, 3).max(0)
    p = tris - (lo + hi) / 2
    a, e = math.radians(az), math.radians(90 if top else el)
    x1 = p[..., 0] * math.cos(a) - p[..., 2] * math.sin(a)
    z1 = p[..., 0] * math.sin(a) + p[..., 2] * math.cos(a)
    y2 = p[..., 1] * math.cos(e) - z1 * math.sin(e)
    depth = p[..., 1] * math.sin(e) + z1 * math.cos(e)
    size = (1600, 1100)
    span = max(hi[0] - lo[0], hi[2] - lo[2], 1e-6)
    scale = 0.92 * min(size[0], size[1]) / span
    sx, sy = size[0] / 2 + x1 * scale, size[1] / 2 - y2 * scale
    n = np.cross(tris[:, 1] - tris[:, 0], tris[:, 2] - tris[:, 0])
    n /= np.maximum(np.linalg.norm(n, axis=1, keepdims=True), 1e-12)
    shade = np.clip(0.5 + 0.5 * np.abs(n[:, 1]), 0.35, 1.0)
    img = Image.new("RGB", size, (150, 190, 230))
    dr = ImageDraw.Draw(img)
    for t in np.argsort(-depth.mean(1)):
        c = tuple(int(v) for v in np.clip(cols[t] * shade[t], 0, 255))
        dr.polygon([(sx[t, k], sy[t, k]) for k in range(3)], fill=c)
    img.save(out)
    print("wrote", out, "triangles", len(tris), "extent metres?", (hi - lo).round(1))


if __name__ == "__main__":
    sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))
    main(sys.argv[1:])
