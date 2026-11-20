#!/usr/bin/env python3
"""Top-down and oblique picture of a unit's collision, coloured by the face flag word (a check of the decoder).

    python tools/chameleon_collision_view.py <game dir> <unit> <out.png> [--top]
"""

import colorsys
import math
import os
import sys

from PIL import Image, ImageDraw

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from chameleon_bundle import Bundle  # noqa: E402
from chameleon_collision import read_soup_list  # noqa: E402


def main(game, unit, out, top):
    b = Bundle(os.path.join(game, "HAWAII", "TRK_UNIT%d.BNDL" % unit))
    polys = []
    for e in b.by_type(0x60):
        for verts, faces in read_soup_list(b.block(e, 0)):
            for data, idx in faces:
                if len(idx) >= 3:
                    polys.append(([verts[i] for i in (idx if len(idx) == 3 else (idx[0], idx[1], idx[3], idx[2]))], data))
    xs = [v[0] for p, _ in polys for v in p]
    zs = [v[2] for p, _ in polys for v in p]
    cx, cz = (min(xs) + max(xs)) / 2, (min(zs) + max(zs)) / 2
    span = max(max(xs) - min(xs), max(zs) - min(zs))
    size = (1400, 1000)
    sc = 0.92 * min(size) / span
    el = math.radians(90 if top else 40)
    az = math.radians(25)
    items = []
    for p, data in polys:
        pts, depth = [], 0.0
        for x, y, z in p:
            dx, dz = x - cx, z - cz
            x1 = dx * math.cos(az) - dz * math.sin(az)
            z1 = dx * math.sin(az) + dz * math.cos(az)
            y2 = y * math.cos(el) - z1 * math.sin(el)
            depth += y * math.sin(el) + z1 * math.cos(el)
            pts.append((size[0] / 2 - x1 * sc, size[1] / 2 - y2 * sc))
        items.append((depth / len(p), pts, data))
    items.sort(key=lambda t: -t[0])
    img = Image.new("RGB", size, (20, 22, 28))
    dr = ImageDraw.Draw(img)
    for _, pts, data in items:
        hue = ((data >> 14) & 0x3FF) / 1024.0
        r, g, bl = colorsys.hsv_to_rgb(hue, 0.55, 0.85)
        dr.polygon(pts, fill=(int(r * 255), int(g * 255), int(bl * 255)))
    img.save(out)
    print("wrote", out, len(polys), "faces")


if __name__ == "__main__":
    main(sys.argv[1], int(sys.argv[2]), sys.argv[3], "--top" in sys.argv)
