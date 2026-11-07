"""Export the optional parts of the catalogue once, and place them on each bike.

    python -I option_export.py --catalogue C:\\r6work\\bike-parts --bikes C:\\r6work\\bikes [--only <regex>]

Reads `upgrades.py`'s `catalogue.json` and writes into the same folder:

- `glb/` and `tex/`: every mesh (levels of detail 0 to 3) and texture of every part a bike can use, **once** (a silencer of
  twenty bikes is one file; the rims, brake discs and bar pads likewise);
- `parts.json`: a part table in the schema of a bike's (`PartTable`), one part per mesh, each at the identity anchor: the
  parts in their own frame, which `ride6-bike-import` writes as the shared pack;
- `catalogue.json` again, with for each bike and part the **anchor** that puts it on that bike (`chassis_world(bone) *
  part_world(bone)^-1`, as for the stock parts), and the rims each bike can take (the radius of the rim within 3 percent of its
  own and the width within 0.75 to 1.35 times, both wheels: **[choice]**, the data names no bike for a rim).

A part that is not one rigid transform on a bike (its bones move differently) is left out for that bike. The skeletons of a bike
come from its `parts.json` in `--bikes` (`export_all.py`), so the bikes must be exported first.
"""

import argparse
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import r6site  # noqa: F401,E402

import numpy as np  # noqa: E402

import bake  # noqa: E402
import bikeparts  # noqa: E402
import glbtools  # noqa: E402
import materials as mat  # noqa: E402
from bindings import Tables  # noqa: E402
from r6lib import Assets, Tool, object_name  # noqa: E402

IDENTITY = {"bone": None, "translation": [0.0, 0.0, 0.0], "rotation": [0.0, 0.0, 0.0, 1.0]}
DIAMETER_TOLERANCE = 0.03
WIDTH_RANGE = (0.75, 1.35)
SIDES = {"rim": ("front", "back"), "brake": ("front", "back")}


def part_list(catalogue, keep):
    """`[(part id, role, mesh path, material path)]` of the assets in `keep`."""
    out = []
    for stem in sorted(keep):
        asset = catalogue["assets"][stem]
        for side, mesh in sorted(asset["meshes"].items()):
            kind = asset["kind"]
            if kind in SIDES:
                out.append((f"{stem}_{side}_{kind}", f"{side}_wheel", mesh, asset["material"]))
                continue
            out.append((stem, "body", mesh, asset["material"]))
    return out


def extents(path):
    """Sorted extents of the vertices of a mesh in its own frame: `[width, d1, d2]` for a wheel part, metres."""
    positions = glbtools.Glb(path).positions()
    return sorted(float(v) for v in positions.max(axis=0) - positions.min(axis=0))


def measures(glb_dir, stem):
    path = bikeparts.find_glb(glb_dir, stem)
    return extents(path) if path else None


def fits(stock, option):
    """Whether a rim of extents `option` fits a wheel whose stock rim has extents `stock`."""
    diameter = lambda e: (e[1] + e[2]) / 2
    if abs(diameter(option) / diameter(stock) - 1) > DIAMETER_TOLERANCE:
        return False
    return WIDTH_RANGE[0] <= option[0] / stock[0] <= WIDTH_RANGE[1]


def bike_bones(parts):
    """`{bone: world matrix}` from a bike's `parts.json` (position and unit quaternion of each bone)."""
    out = {}
    for name, bone in parts["bones"].items():
        m = np.eye(4)
        m[:3, :3] = glbtools.quat_matrix(bone["rotation"])
        m[:3, 3] = bone["position"]
        out[name] = m
    return out


def stock_rim(root, vehicle, parts, side):
    glb_dir = os.path.join(root, vehicle, "glb")
    part = next((p for p in parts["parts"] if p["id"] == f"{side}_rim"), None)
    return measures(glb_dir, part["export"]) if part else None


def compatible_rims(catalogue, bikes_root, out_dir, only):
    """Keep on each bike the aftermarket rims that fit its wheels; returns the rim assets some bike keeps."""
    options = {}
    for stem, asset in catalogue["assets"].items():
        if asset["kind"] != "rim":
            continue
        sides = {}
        for side, mesh in asset["meshes"].items():
            sides[side] = measures(os.path.join(out_dir, "glb"), object_name(mesh))
        options[stem] = sides
    kept = set()
    for vehicle, entry in catalogue["bikes"].items():
        path = os.path.join(bikes_root, vehicle, "parts.json")
        if not os.path.exists(path) or not re.search(only, vehicle):
            entry["rims"] = []
            continue
        with open(path, encoding="utf-8") as f:
            parts = json.load(f)
        stock = {side: stock_rim(bikes_root, vehicle, parts, side) for side in ("front", "back")}
        ok = []
        for rim in entry["rims"]:
            sides = options[rim["asset"]]
            if all(stock[s] and sides.get(s) and fits(stock[s], sides[s]) for s in ("front", "back")):
                ok.append(rim)
                kept.add(rim["asset"])
        entry["rims"] = ok
    return kept


def used_assets(catalogue):
    keep = set()
    for entry in catalogue["bikes"].values():
        for item in entry["exhaust"]:
            keep.update((item["header"], item["muffler"]))
        keep.update(item["asset"] for item in entry["brakes"] + entry["rims"] + entry["barpad"])
    return keep


def write_parts(tool, assets, catalogue, keep, out_dir, max_tex):
    """Export the meshes, materials and textures of `keep`; returns the shared part table."""
    parts = part_list(catalogue, keep)
    glb_dir = os.path.join(out_dir, "glb")
    tex_dir = os.path.join(out_dir, "tex")
    failed = bikeparts.export_meshes(tool, sorted({mesh for _, _, mesh, _ in parts}), glb_dir)
    entries, needed, missing = [], {}, [{"part": "mesh", "reason": "export failed", "detail": line} for line in failed]
    for pid, role, mesh, material in parts:
        stem = object_name(mesh)
        part = {"mesh": mesh, "material": material}
        mats, _ = bikeparts.part_materials(assets, part, None)
        for path in mats.values():
            needed[object_name(path)] = path
        entries.append({"id": pid, "role": role, "export": stem, "anchor": IDENTITY, "materials": mats, "source": pid, "mesh": mesh})
    assets.ensure(list(needed.values()))
    resolved = {name: mat.resolve(assets, path) for name, path in sorted(needed.items())}
    resolved = {name: value for name, value in resolved.items() if value is not None}
    wanted = {}
    for value in resolved.values():
        for path in mat.textures_of(value):
            wanted[object_name(path)] = path
    assets.ensure(list(wanted.values()))
    os.makedirs(tex_dir, exist_ok=True)
    answer = bikeparts.tex_batch(tool, wanted, tex_dir, max_tex)
    missing += [{"part": "texture", "reason": "texture not exported", "detail": line} for line in answer if not line.startswith("OK")]
    return {
        "schema": 1,
        "vehicle": "bike-option-parts",
        "parts": entries,
        "materials": {name: bikeparts.material_entry(value) for name, value in resolved.items()},
        "textures": bikeparts.texture_table(tex_dir, wanted),
        "missing": missing,
        "notes": [],
    }


def anchor_of(glb_dir, stem, bones):
    """The anchor of the mesh `stem` on a bike with `bones`, None when it is not one rigid transform."""
    path = bikeparts.smallest_glb(glb_dir, stem)
    if not path:
        return None
    part = glbtools.Glb(path)
    anchor = glbtools.anchor(part, bones)
    if anchor is None or not bake.is_rigid(part, bones):
        return None
    return anchor


def place(catalogue, bikes_root, out_dir, only):
    """Add the anchors of each option part to each bike; an option that does not sit on a bike is dropped from it."""
    glb_dir = os.path.join(out_dir, "glb")
    for vehicle, entry in catalogue["bikes"].items():
        path = os.path.join(bikes_root, vehicle, "parts.json")
        if not os.path.exists(path) or not re.search(only, vehicle):
            for key in ("exhaust", "brakes", "rims", "barpad"):
                entry[key] = []
            continue
        with open(path, encoding="utf-8") as f:
            bones = bike_bones(json.load(f))

        def anchors(asset, sides):
            found = {}
            for side, mesh in sides.items():
                value = anchor_of(glb_dir, object_name(mesh), bones)
                if value is None:
                    return None
                found[side] = value
            return found

        for key in ("brakes", "rims", "barpad"):
            kept = []
            for item in entry[key]:
                item["anchors"] = anchors(item["asset"], catalogue["assets"][item["asset"]]["meshes"])
                if item["anchors"]:
                    kept.append(item)
            entry[key] = kept
        kept = []
        for item in entry["exhaust"]:
            header = anchors(item["header"], catalogue["assets"][item["header"]]["meshes"])
            silencer = anchors(item["muffler"], catalogue["assets"][item["muffler"]]["meshes"])
            if header and silencer:
                item["anchors"] = {"header": header["body"], "muffler": silencer["body"]}
                kept.append(item)
        entry["exhaust"] = kept


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--catalogue", default=r"C:\r6work\bike-parts")
    parser.add_argument("--bikes", default=r"C:\r6work\bikes")
    parser.add_argument("--only", default="")
    parser.add_argument("--max-tex", type=int, default=2048)
    args = parser.parse_args()
    path = os.path.join(args.catalogue, "catalogue.json")
    with open(path, encoding="utf-8") as f:
        catalogue = json.load(f)
    tool = Tool()
    try:
        assets = Assets(tool)
        Tables(assets)
        # The rims are measured first: only the ones some bike takes are kept in the shared export.
        rims = [a for a, e in catalogue["assets"].items() if e["kind"] == "rim"]
        bikeparts.export_meshes(tool, sorted({m for a in rims for m in catalogue["assets"][a]["meshes"].values()}), os.path.join(args.catalogue, "glb"))
        compatible_rims(catalogue, args.bikes, args.catalogue, args.only)
        keep = used_assets(catalogue)
        table = write_parts(tool, assets, catalogue, keep, args.catalogue, args.max_tex)
    finally:
        tool.close()
    place(catalogue, args.bikes, args.catalogue, args.only)
    keep = used_assets(catalogue)
    table["parts"] = [p for p in table["parts"] if any(p["id"] == s or p["id"].startswith(s + "_") for s in keep)]
    catalogue["assets"] = {stem: asset for stem, asset in catalogue["assets"].items() if stem in keep}
    with open(os.path.join(args.catalogue, "parts.json"), "w", encoding="utf-8") as f:
        json.dump(bikeparts.clean(table), f, indent=1)
    with open(path, "w", encoding="utf-8") as f:
        json.dump(bikeparts.clean(catalogue), f, indent=1, sort_keys=True)
    print(len(table["parts"]), "parts,", len(keep), "assets kept")


if __name__ == "__main__":
    main()
