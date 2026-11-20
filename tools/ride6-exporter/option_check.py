"""Check that every optional part of the shop sits where the stock part it replaces sits.

    python -I option_check.py [--catalogue C:\\r6work\\bike-parts] [--bikes C:\\r6work\\bikes] [--only <regex>] [--list 20]

For each bike and each option of `catalogue.json` (exhaust: header and silencers, rims, brake discs, bar pad) the vertices of
the option's mesh (finest level) are moved by the option's anchor (rotation, scale, translation: the importer's `Anchor::point`)
and the **centre of their bounding box** is compared with the centre of the bounding box of the stock part of the bike (its
`parts.json` anchors on its own exports). A part whose centre is farther than its kind's threshold is listed, and the command
exits with 1. The thresholds are in [`THRESHOLDS`] (metres) **[choice]**: an aftermarket part is not the stock part, so
the centre moves with its size (a short silencer against a long one), but a part at the origin of its own frame, or on the other
side of the bike, is metres or decimetres away. Three more checks do not depend on the stock part's design: a silencer's box must
touch its header pipes' box (`exhaust.joint`), and a rim's lip and flanges must end within a threshold of its tyre's bead
(`rim.*.seat`, the "transparent ring"). The measured spread before and after the fix is in `docs/formats/ride6-bike-parts.md`.
"""

import argparse
import collections
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import r6site  # noqa: F401,E402

import numpy as np  # noqa: E402

import glbtools  # noqa: E402
import option_place  # noqa: E402
from r6lib import object_name  # noqa: E402

# The farthest an option's centre may be from the stock part's, by kind of option, metres.
THRESHOLDS = {
    "exhaust": 0.3,
    # The header pipes alone against the stock header, and the silencers alone against the stock silencer. An aftermarket system
    # is not the stock one (a long can beside the wheel against a belly box: the silencer's centre is up to 0.9 m from the stock
    # one after the fix, a header 0.4), so these are loose; they tell a part at the other end of the bike, not a design.
    "exhaust.header": 0.4,
    "exhaust.silencer": 1.0,
    # What tells a misplaced silencer whatever its design: the gap between its box and its header pipes' box (the silencer is
    # fitted to the end of the pipes, so the two touch). A silencer left at the origin of its own frame is 0.4 to 0.9 m away.
    "exhaust.joint": 0.05,
    "rim.front": 0.05,
    "rim.back": 0.05,
    "brake.front": 0.05,
    "brake.back": 0.05,
    "barpad": 0.05,
    # How far an aftermarket rim's lip and flanges end from its tyre's bead (radially, axially): a ring of this width or more
    # between rim and tyre is seen through ("a transparent ring"); `option_place` scales the rims to close it, within clamps (a rim at
    # its clamp, 1.2 to 1.7 cm short, is the worst after the fix).
    "rim.front.seat": 0.02,
    "rim.back.seat": 0.02,
}


class Meshes:
    """The vertices of glb files, read once."""

    def __init__(self):
        self.cache = {}

    def vertices(self, path):
        if path not in self.cache:
            self.cache[path] = glbtools.Glb(path).positions()
        return self.cache[path]

    def seat(self, path, anchor, tyre):
        """How far the mesh at `path` moved by `anchor` ends from the bead of `tyre` (`(bead radius, bead half width)`): the larger
        of the radial gap (bead radius minus the rim's lip) and the axial one (bead half width minus the rim's flange reach)."""
        matrix = option_place.anchor_matrix(anchor)
        moved = (matrix[:3, :3] @ self.vertices(path).T).T
        radius = np.hypot(moved[:, 0], moved[:, 1])
        lip = float(radius.max())
        flange = float(np.abs(moved[radius >= option_place.FLANGE_SHARE * lip, 2]).max())
        return float(max(tyre[0] - lip, tyre[1] - flange, 0.0))

    def box(self, path, anchor):
        """Bounding box `(min, max)` of the mesh at `path` moved by `anchor`."""
        matrix = option_place.anchor_matrix(anchor)
        moved = (matrix[:3, :3] @ self.vertices(path).T).T + matrix[:3, 3]
        return moved.min(axis=0), moved.max(axis=0)


def union(boxes):
    return np.min([b[0] for b in boxes], axis=0), np.max([b[1] for b in boxes], axis=0)


def box_gap(a, b):
    """The distance between two boxes: 0 when they touch or overlap."""
    apart = np.maximum(0.0, np.maximum(a[0] - b[1], b[0] - a[1]))
    return float(np.linalg.norm(apart))


def centre(box):
    return (box[0] + box[1]) / 2.0


_INDEX = {}


def find_glb(glb_dir, stem):
    """The level-0 file `<stem>.glb` under `glb_dir` (one walk of each folder, remembered)."""
    if glb_dir not in _INDEX:
        found = {}
        for root, _, names in os.walk(glb_dir):
            for name in names:
                if name.endswith(".glb") and "_LOD" not in name:
                    found.setdefault(name[:-4].lower(), os.path.join(root, name))
        _INDEX[glb_dir] = found
    return _INDEX[glb_dir].get(stem.lower())


def stock_box(meshes, bike_dir, parts, ids):
    boxes = []
    for part in parts["parts"]:
        if part["id"] not in ids:
            continue
        path = find_glb(os.path.join(bike_dir, "glb"), part["export"])
        if path:
            boxes.append(meshes.box(path, part["anchor"]))
    return union(boxes) if boxes else None


def mesh_path(glb_dir, catalogue, asset, side):
    return find_glb(glb_dir, object_name(catalogue["assets"][asset]["meshes"][side]))


def silencer_anchors(anchors):
    """The anchors of a twin's silencers: `muffler`, `muffler_2`, ... ."""
    return [anchor for key, anchor in sorted(anchors.items()) if key.startswith("muffler")]


def findings(catalogue, bikes_root, glb_dir, only=""):
    """`[(kind, bike, option, distance, offset)]` of every option of every bike with a stock part to compare with."""
    meshes = Meshes()
    out = []
    for vehicle, entry in sorted(catalogue["bikes"].items()):
        parts_path = os.path.join(bikes_root, vehicle, "parts.json")
        if only not in vehicle or not os.path.exists(parts_path):
            continue
        with open(parts_path, encoding="utf-8") as f:
            parts = json.load(f)
        bike_dir = os.path.join(bikes_root, vehicle)

        def add(kind, name, stock, option):
            if stock is None:
                return
            offset = centre(option) - centre(stock)
            out.append((kind, vehicle, name, float(np.linalg.norm(offset)), offset))

        stock = stock_box(meshes, bike_dir, parts, ("exmanifold", "muffler"))
        stock_header = stock_box(meshes, bike_dir, parts, ("exmanifold",))
        stock_silencer = stock_box(meshes, bike_dir, parts, ("muffler",))
        for item in entry["exhaust"]:
            header = meshes.box(mesh_path(glb_dir, catalogue, item["header"], "body"), item["anchors"]["header"])
            silencer = mesh_path(glb_dir, catalogue, item["muffler"], "body")
            silencers = [meshes.box(silencer, anchor) for anchor in silencer_anchors(item["anchors"])]
            add("exhaust", item["id"], stock, union([header, *silencers]))
            add("exhaust.header", item["id"], stock_header, header)
            add("exhaust.silencer", item["id"], stock_silencer, union(silencers))
            joint = max(box_gap(header, one) for one in silencers)
            out.append(("exhaust.joint", vehicle, item["id"], joint, np.array([joint, 0.0, 0.0])))
        for side in ("front", "back"):
            tyre_part = next((p for p in parts["parts"] if p["id"] == f"{side}_tyre"), None)
            tyre_path = tyre_part and find_glb(os.path.join(bike_dir, "glb"), tyre_part["export"])
            tyre = option_place.tyre_seat(meshes.vertices(tyre_path)) if tyre_path else None
            for item in entry["rims"] if tyre else []:
                rim = mesh_path(glb_dir, catalogue, item["asset"], side)
                gap = meshes.seat(rim, item["anchors"][side], tyre)
                out.append((f"rim.{side}.seat", vehicle, item["id"], gap, np.array([gap, 0.0, 0.0])))
            for key, kind, ids in (("rims", "rim", f"{side}_rim"), ("brakes", "brake", f"{side}_brake")):
                stock = stock_box(meshes, bike_dir, parts, (ids,))
                for item in entry[key]:
                    path = mesh_path(glb_dir, catalogue, item["asset"], side)
                    add(f"{kind}.{side}", item.get("id") or f"level {item.get('level')}", stock, meshes.box(path, item["anchors"][side]))
        stock = stock_box(meshes, bike_dir, parts, ("barpad",))
        for item in entry["barpad"]:
            path = mesh_path(glb_dir, catalogue, item["asset"], "body")
            add("barpad", item["id"], stock, meshes.box(path, item["anchors"]["body"]))
    return out


def report(found, list_count):
    """Print the spread by kind and the worst; returns the options over their threshold."""
    by_kind = collections.defaultdict(list)
    for item in found:
        by_kind[item[0]].append(item)
    over = [item for item in found if item[3] > THRESHOLDS[item[0]]]
    for kind, items in sorted(by_kind.items()):
        d = np.array([i[3] for i in items])
        print(
            f"{kind:12s} {len(items):5d} options  median {np.median(d):.3f}  p90 {np.percentile(d, 90):.3f}  max {d.max():.3f} m"
            f"  (threshold {THRESHOLDS[kind]:.2f}, over: {sum(1 for i in items if i[3] > THRESHOLDS[kind])})"
        )
    for kind, bike, name, distance, offset in sorted(over, key=lambda i: -i[3])[:list_count]:
        print(f"  OVER {kind} {bike} {name}: {distance:.3f} m, offset {np.round(offset, 3).tolist()}")
    return over


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--catalogue", default=r"C:\r6work\bike-parts")
    parser.add_argument("--bikes", default=r"C:\r6work\bikes")
    parser.add_argument("--only", default="")
    parser.add_argument("--file", default="catalogue.json", help="the catalogue file of the folder (catalogue.before-placement.json: the old one)")
    parser.add_argument("--list", type=int, default=20, help="how many offenders to print")
    args = parser.parse_args()
    with open(os.path.join(args.catalogue, args.file), encoding="utf-8") as f:
        catalogue = json.load(f)
    found = findings(catalogue, args.bikes, os.path.join(args.catalogue, "glb"), args.only)
    over = report(found, args.list)
    print(f"{len(found)} options checked, {len(over)} over their threshold")
    sys.exit(1 if over else 0)


if __name__ == "__main__":
    main()
