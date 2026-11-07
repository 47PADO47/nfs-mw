#!/usr/bin/env python3
"""Export one RIDE 6 track into a track pack (`docs/formats/ride6-track-pack.md`).

    python -I tools/ride6-tracks/export.py Okayama [--out C:\\r6work\\tracks\\okayama] [--max-texture 1024]

Reads the user's install through the exporter's tool (`tools/ride6-exporter`). Nothing of the game is in the repository; the
pack goes to `--out`. Needs numpy and Pillow (the user site-packages are added by `common.py`).
"""

import argparse
import math
import os
import re
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))  # `python -I` leaves it out

import common  # noqa: E402  (first: it makes the user site-packages importable)
import numpy as np  # noqa: E402
import level  # noqa: E402
import meshes  # noqa: E402
import pack  # noqa: E402
import props  # noqa: E402
import trkmat  # noqa: E402

GROUND_SKIP = re.compile(r"_SC$|water|overlay|decal", re.IGNORECASE)
GROUND_SKIP_SHADOW = re.compile(r"_SC$", re.IGNORECASE)
# The far hills are drawn and not driven on; everything else of the ground is collision.
NO_COLLISION = re.compile(r"far", re.IGNORECASE)


def mesh_materials(assets, mesh_path, overrides):
    """The material path of each slot of a mesh: the component's override, else the mesh's own."""
    export = assets.named(mesh_path) or {}
    slots = (export.get("Properties") or {}).get("StaticMaterials") or []
    out = []
    for i, slot in enumerate(slots):
        override = overrides[i] if i < len(overrides) else None
        own = (slot.get("MaterialInterface") or {}).get("ObjectPath")
        out.append(override or own)
    return out


def ground_meshes(assets, track):
    """The ground static meshes of the track's terrain level: `[{name, mesh, materials, collide}]`."""
    exports = assets.exports(level.level_paths(track)["Terrain"])
    found = []
    for item in level.static_meshes(exports):
        name = common.r6lib.object_name(item["mesh"])
        if GROUND_SKIP.search(name):
            continue
        found.append(
            {
                "name": name,
                "mesh": item["mesh"],
                "materials": mesh_materials(assets, item["mesh"], item["overrides"]),
                "collide": not NO_COLLISION.search(name),
            }
        )
    return found


def find_sea(assets, tool, track, scratch):
    """The sea of the track: the median height of its water meshes (render space) and a colour, or None when it has none."""
    exports = assets.exports(level.level_paths(track)["Terrain"])
    water = [
        {"mesh": item["mesh"], "name": common.r6lib.object_name(item["mesh"])}
        for item in level.static_meshes(exports)
        if "water" in common.r6lib.object_name(item["mesh"]) and not GROUND_SKIP_SHADOW.search(common.r6lib.object_name(item["mesh"]))
    ]
    if not water:
        return None
    heights = []
    glbs = export_glb(tool, water, os.path.join(scratch, "water"))
    for item in water:
        path = glbs.get(item["name"].lower())
        if path is None:
            continue
        for _material, attrs, _indices in meshes.Glb(path).slots():
            heights.append(float(np.median(meshes.to_render(attrs["POSITION"])[:, 2])))
    if not heights:
        return None
    return {"z": float(np.median(heights)), "color": [28, 62, 92]}


def export_glb(tool, items, out_dir):
    """The glTF of every mesh, written under `out_dir` by the tool; returns `{name: path}`."""
    paths = ";".join(item["mesh"] for item in items)
    for line in tool.request("export", paths, out_dir, "Gltf2", "All"):
        if line.startswith(("FAIL", "MISSING")):
            common.log("export:", line)
    found = {}
    for root, _dirs, files in os.walk(out_dir):
        for name in files:
            if name.lower().endswith(".glb") and "_lod" not in name.lower():
                found[name[:-4].lower()] = os.path.join(root, name)
    return found


class Builder:
    """Collects the materials and the triangle groups of a pack."""

    def __init__(self, assets):
        self.assets = assets
        self.ground = pack.Ground()
        self.materials = []
        self.index = {}
        self.pictures = set()
        self.info = {}

    def material(self, name, picture, tile_m, uv, surface, colour=(200, 200, 200)):
        if name in self.index:
            return self.index[name]
        stem = trkmat.stem(picture) if picture else None
        if picture:
            self.pictures.add(picture)
        self.materials.append(
            {"name": name, "texture": stem, "color": list(colour), "tile_m": tile_m, "uv": uv, "surface": surface}
        )
        self.index[name] = len(self.materials) - 1
        return self.index[name]

    def info_of(self, path):
        if path not in self.info:
            self.info[path] = trkmat.classify(self.assets, path)
        return self.info[path]

    def add_primitive(self, item, slot, attrs, indices):
        path = item["materials"][slot] if slot < len(item["materials"]) else None
        info = self.info_of(path) if path else None
        positions = meshes.to_render(attrs["POSITION"])
        if info is None or info["family"] == "other" or (info["family"] == "prop" and not info["picture"]):
            self.add_group(item, "plain", None, 10.0, "planar", "dirt", attrs, indices, None)
            return
        if info["family"] != "terrain":
            uv = "uv0" if info["family"] in ("kerb", "prop") else "planar"
            surface = trkmat.surface_of(info)
            self.add_group(item, info["name"], info["picture"], info.get("tile_m", 10.0), uv, surface, attrs, indices, None)
            return
        present = {layer["layer"] for layer in info["layers"]}
        by_layer = {layer["layer"]: layer for layer in info["layers"]}
        layers = meshes.layer_of_triangles(meshes.vertex_colours(attrs), indices, present)
        for number in sorted(present):
            mask = layers == number
            if not mask.any():
                continue
            layer = by_layer[number]
            surface = trkmat.surface_of(info, layer["picture"])
            name = f"{info['name']}#L{number}"
            self.add_group(item, name, layer["picture"], layer["tile_m"], "planar", surface, attrs, indices, mask)
        del positions

    def add_group(self, item, name, picture, tile_m, uv, surface, attrs, indices, mask):
        if mask is None:
            mask = np.ones(len(indices) // 3, dtype=bool)
        positions, normals, uv0, local = meshes.part(attrs, indices, mask)
        if normals is None:
            normals = np.zeros_like(positions)
            normals[:, 2] = 1.0
        coords = meshes.planar_uv(positions, normals, tile_m) if uv == "planar" else uv0
        material = self.material(name, picture, tile_m, uv, surface)
        item.get("sink", self.ground).add(
            material, item["collide"], positions, normals, coords, local, item.get("model", 0)
        )


def export_spline(assets, track):
    """The lap: the track spline sampled every 2 m, the starting grid, in render space."""
    out = {}
    for layout in ("SplinesA", "Splines", "SplinesB"):
        exports = assets.exports(level.level_paths(track)[layout])
        if not exports:
            continue
        component = level.spline_component(exports, "TrackSpline")
        if component is None:
            continue
        out["spline"] = level.sample_spline(component, 2.0)
        out["layout"] = layout
        out["grid"] = level.target_points(exports, "StartingGrid")
        return out
    return out


def export_textures(tool, builder, out_dir, size):
    os.makedirs(out_dir, exist_ok=True)
    list_file = os.path.join(out_dir, "_tex.lst")
    with open(list_file, "w", encoding="utf-8") as f:
        f.write("\n".join(sorted(builder.pictures)) + "\n")
    for line in tool.request("tex", list_file, out_dir, size):
        if not line.startswith("OK"):
            common.log("texture:", line)
    os.remove(list_file)
    for name in os.listdir(out_dir):
        if name.endswith(".txt"):
            os.remove(os.path.join(out_dir, name))


def export_objects(tool, assets, builder, track, out, scratch):
    """The barriers, buildings and props: `models.r6m` (each distinct mesh once), `instances.r6i`; the models for pack.json."""
    instances, models = props.collect(assets, track, common.log)
    if not instances:
        return []
    sink = pack.Ground()
    items = [
        {
            "name": m["name"],
            "mesh": mesh,
            "materials": mesh_materials(assets, mesh, m["overrides"]),
            "collide": m["solid"],
            "model": m["index"],
            "sink": sink,
        }
        for mesh, m in models.items()
    ]
    glbs = export_glb(tool, items, os.path.join(scratch, "objects"))
    for item in items:
        path = glbs.get(item["name"].lower())
        if path is None:
            continue
        for slot, (_material, attrs, indices) in enumerate(meshes.Glb(path).slots()):
            builder.add_primitive(item, slot, attrs, indices)
    os.makedirs(out, exist_ok=True)
    sink.write(os.path.join(out, "models.r6m"))
    props.write_instances(os.path.join(out, "instances.r6i"), instances)
    spline = [] if "spline" not in (export_spline(assets, track) or {}) else export_spline(assets, track)["spline"]["points"]
    common.log(f"{len(instances)} objects of {len(models)} models, {sink.triangles()} triangles; mean distance to the lap {props.median_distance(instances, spline)}")
    return [{"name": m["name"], "solid": m["solid"]} for m in sorted(models.values(), key=lambda m: m["index"])]


def run(track, out, size):
    started = time.time()
    tool, assets = common.open_assets()
    items = ground_meshes(assets, track)
    common.log(f"{len(items)} ground meshes")
    scratch = os.path.join(out, "_glb")
    glbs = export_glb(tool, items, scratch)
    builder = Builder(assets)
    for item in items:
        path = glbs.get(item["name"].lower())
        if path is None:
            common.log("no glb for", item["name"])
            continue
        for slot, (_material, attrs, indices) in enumerate(meshes.Glb(path).slots()):
            builder.add_primitive(item, slot, attrs, indices)
    object_models = export_objects(tool, assets, builder, track, out, scratch)
    os.makedirs(out, exist_ok=True)
    builder.ground.write(os.path.join(out, "ground.r6m"))
    export_textures(tool, builder, os.path.join(out, "tex"), size)
    lap = export_spline(assets, track)
    sea = find_sea(assets, tool, track, scratch)
    document = {
        "format": 1,
        "track": track,
        "source": "RIDE 6 install, exported by tools/ride6-tracks",
        "triangles": builder.ground.triangles(),
        "materials": builder.materials,
        "sea": sea,
        "models": object_models,
        "lap": lap,
    }
    pack.write_json(os.path.join(out, "pack.json"), document)
    tool.close()
    common.log(f"{builder.ground.triangles()} triangles, {len(builder.materials)} materials, {len(builder.pictures)} pictures")
    common.log(f"pack {pack.pack_size(out) / 1e6:.1f} MB in {time.time() - started:.1f} s (with the glTF scratch)")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("track")
    parser.add_argument("--out", default=None)
    parser.add_argument("--max-texture", type=int, default=1024)
    args = parser.parse_args()
    out = args.out or os.path.join(common.OUT_ROOT, args.track.lower() + "-pack")
    run(args.track, out, args.max_texture)
    return 0


if __name__ == "__main__":
    sys.exit(main())
