"""Measure the "transparent ring" between an aftermarket rim and the tyre with a software rasteriser (no game needed).

    python -I rim_ring.py [--catalogue C:\\r6work\\bike-parts] [--bikes C:\\r6work\\bikes] [--count 25] [--seed 7] [--bike <id>]

For each wheel the tyre and the rim (the stock one, the catalogue's aftermarket rim as it was exported, and the same rim seated on
the tyre by `option_place.rim_seat_scale`) are drawn from two oblique views with back faces culled, as the game draws them. In each
view the **share of a 1.2 cm band just inside the tyre's bead, on the near side, that nothing covers** is counted: that is the ring
seen through. The mean of the views is printed for the three rims, and how many have more than a fifth of the band open. Needs numpy
and PIL (`pip install pillow`). **[choice]** the band, the views (yaw 35 and -35 degrees, pitch 25) and the fifth are the tool's: they
make the ring of the TM SMX Competizione's OZ Cattiva rim (0.58 as exported) visible and compare rims with each other; they are not a
property of the game.
"""

import argparse
import json
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import r6site  # noqa: F401,E402

import numpy as np  # noqa: E402
from PIL import Image, ImageDraw  # noqa: E402

import option_place  # noqa: E402
from r6lib import object_name  # noqa: E402

VIEWS = ((35, 25), (-35, 25))
BAND = 0.012
SIZE, SCALE = 500, 1100
OPEN_SHARE = 0.2


def triangles(glb, anchor, scale=None):
    """The triangles of `glb` in the hub's frame: moved by the anchor's rotation (and scale), not its translation."""
    matrix = option_place.anchor_matrix(anchor)[:3, :3]
    if scale is not None:
        matrix = matrix @ np.diag(scale)
    out = []
    for mesh in glb.doc.get("meshes", []):
        for prim in mesh.get("primitives", []):
            points = (matrix @ glb.accessor(prim["attributes"]["POSITION"]).astype(np.float64).T).T
            index = glb.accessor(prim["indices"]).astype(np.int64).reshape(-1, 3)
            out.append(points[index])
    return np.concatenate(out)


def view(yaw, pitch):
    cy, sy, cp, sp = np.cos(yaw), np.sin(yaw), np.cos(pitch), np.sin(pitch)
    return np.array([[1, 0, 0], [0, cp, -sp], [0, sp, cp]]) @ np.array([[cy, 0, sy], [0, 1, 0], [-sy, 0, cy]])


def covered(sets, rotation):
    """A boolean picture: where a front-facing triangle of any of `sets` is."""
    polys = []
    for tris in sets:
        moved = (rotation @ tris.reshape(-1, 3).T).T.reshape(-1, 3, 3)
        facing = np.cross(moved[:, 1] - moved[:, 0], moved[:, 2] - moved[:, 0])[:, 2] > 0
        polys += [[(SIZE / 2 + v[0] * SCALE, SIZE / 2 - v[1] * SCALE) for v in t] for t in moved[facing]]
    image = Image.new("L", (SIZE, SIZE), 0)
    draw = ImageDraw.Draw(image)
    for pts in polys:
        draw.polygon(pts, fill=255)
    return np.array(image) > 0


def band(rotation, bead, z):
    """A boolean picture of the band of width `BAND` inside the circle of radius `bead` at height `z` on the axle."""
    angles = np.linspace(0, 2 * np.pi, 181)
    image = Image.new("L", (SIZE, SIZE), 0)
    draw = ImageDraw.Draw(image)
    ring = lambda r: np.c_[r * np.cos(angles), r * np.sin(angles), np.full_like(angles, z)]
    outer, inner = ring(bead), ring(bead - BAND)
    flat = lambda p: ((SIZE / 2 + (rotation @ p)[0] * SCALE), (SIZE / 2 - (rotation @ p)[1] * SCALE))
    for i in range(len(angles) - 1):
        draw.polygon([flat(outer[i]), flat(outer[i + 1]), flat(inner[i + 1]), flat(inner[i])], fill=255)
    return np.array(image) > 0


def open_share(tyre, rim, bead):
    shares = []
    for yaw, pitch in VIEWS:
        rotation = view(np.radians(yaw), np.radians(pitch))
        mask = band(rotation, bead, 0.9 * np.abs(tyre[:, :, 2]).max())
        shares.append(float((~covered([tyre, rim], rotation)[mask]).mean()))
    return float(np.mean(shares))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--catalogue", default=r"C:\r6work\bike-parts")
    parser.add_argument("--bikes", default=r"C:\r6work\bikes")
    parser.add_argument("--count", type=int, default=25)
    parser.add_argument("--seed", type=int, default=7)
    parser.add_argument("--bike", default="")
    args = parser.parse_args()
    with open(os.path.join(args.catalogue, "catalogue.json"), encoding="utf-8") as f:
        catalogue = json.load(f)
    options = option_place.GlbIndex(os.path.join(args.catalogue, "glb"))
    names = [v for v, e in sorted(catalogue["bikes"].items()) if e["rims"] and os.path.exists(os.path.join(args.bikes, v, "parts.json"))]
    random.Random(args.seed).shuffle(names)
    rows = []
    for vehicle in [args.bike] if args.bike else names[: args.count]:
        with open(os.path.join(args.bikes, vehicle, "parts.json"), encoding="utf-8") as f:
            parts = {p["id"]: p for p in json.load(f)["parts"]}
        own = option_place.GlbIndex(os.path.join(args.bikes, vehicle, "glb"))
        for side in ("front", "back"):
            tyre_part, rim_part = parts[f"{side}_tyre"], parts[f"{side}_rim"]
            tyre = triangles(own.glb(own.find(tyre_part["export"])), tyre_part["anchor"])
            stock = triangles(own.glb(own.find(rim_part["export"])), rim_part["anchor"])
            bead, bead_half = option_place.tyre_seat(tyre.reshape(-1, 3))
            for item in catalogue["bikes"][vehicle]["rims"][:2]:
                glb = options.glb(options.find(object_name(catalogue["assets"][item["asset"]]["meshes"][side])))
                anchor = {k: v for k, v in item["anchors"][side].items() if k != "scale"}
                bare = triangles(glb, anchor)
                seated = triangles(glb, anchor, option_place.rim_seat_scale(bare.reshape(-1, 3), bead, bead_half))
                row = [open_share(tyre, rim, bead) for rim in (stock, bare, seated)]
                rows.append(row)
                print(vehicle, side, item["id"], [round(v, 3) for v in row], flush=True)
    data = np.array(rows)
    print("mean open share: stock %.3f, aftermarket as exported %.3f, seated %.3f" % tuple(data.mean(axis=0)))
    print("more than a fifth open:", [int((data[:, i] > OPEN_SHARE).sum()) for i in range(3)], "of", len(rows))


if __name__ == "__main__":
    main()
