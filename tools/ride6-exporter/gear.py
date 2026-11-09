"""The rider gear catalogue of RIDE 6: every helmet, suit, glove and boot of the rider editor, with their meshes and pictures.

    python -I gear.py [--out C:\\r6work\\rider-gear] [--max 1024] [--jobs N]

The rider editor's choices are the rows of the `Bindings/Rider` data tables (`HELMETS`, `SUITS`, `GLOVES`, `BOOTS`); each row
names a data asset of a `*DataSource_C` class that names the meshes and the material instance whose base colour picture is the
look of the item. This script resolves them and runs the tool's `gmesh` and `gtex` modes, then writes `catalogue.json`
(docs/formats/ride6-rider-gear.md). A mesh or picture that two items share, whatever the package it came from, is written once
(files are named by the hash of their content).
"""

import argparse
import hashlib
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import r6site  # noqa: F401,E402

import gear_colors  # noqa: E402
import materials  # noqa: E402
from r6lib import Assets, Tool, object_name, package_key, ref_path  # noqa: E402

RIDERS = "/Game/assets/graphics/characters/riders"
TABLES = "/Game/assets/graphics/Bindings/Rider/"
BASE_COLOR = "BaseColor - Texture"
# The bodies of the rider editor that use the game's rider skeleton (the `NFS` ones are the cinematic rig's, jackets and street wear
# are not racing leathers).
BODIES = ("suit001_male", "suit001_female", "suit002_male", "suit002_female")


def read_table(assets, name):
    """`{row: data asset path}` of a `Bindings/Rider` table."""
    export = assets.exports(TABLES + name)[0]
    return {row: value["DataAsset"]["AssetPathName"] for row, value in export["Rows"].items()}


def display(row):
    """`HL_AGV_Corsa-R_Mono-Black` is brand `AGV`, name `AGV Corsa R Mono Black`."""
    parts = row.split("_")
    brand = parts[1] if len(parts) > 2 else ""
    words = [p.replace("-", " ") for p in parts[1:] if p != "Default"]
    return brand.replace("-", " "), " ".join(words)


def base_color(assets, material_path):
    """The object path of the base colour picture of a material instance, None when it has none."""
    if not material_path:
        return None
    resolved = materials.resolve(assets, material_path)
    return None if resolved is None else resolved["tex"].get(BASE_COLOR)


def sha(path):
    """The first 12 hex digits of the SHA-1 of a file."""
    digest = hashlib.sha1()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()[:12]


def stem_of(path):
    return hashlib.sha1(package_key(path).encode()).hexdigest()[:12]


class Collector:
    """The packages the items need, and what each becomes once exported."""

    def __init__(self):
        self.meshes = {}
        self.textures = {}

    def mesh(self, path):
        if path:
            self.meshes[stem_of(path)] = path
        return stem_of(path) if path else None

    def texture(self, path):
        if path:
            self.textures[stem_of(path)] = path
        return stem_of(path) if path else None


def collect(assets):
    """The items of every category with the temporary keys of their meshes and pictures."""
    got = Collector()
    tables = {name: read_table(assets, name) for name in ("HELMETS", "SUITS", "GLOVES", "BOOTS")}
    for table in tables.values():
        assets.ensure(list(table.values()))
    bodies = []
    for name in BODIES:
        path = f"{RIDERS}/riderShared/Bodies/customization_data/{name}"
        props = assets.props(path)
        mesh = ref_path(props.get("SuitMesh"))
        if mesh:
            gender = "female" if name.endswith("female") else "male"
            bodies.append({"id": name, "gender": gender, "shape": int(name[4:7]), "mesh": got.mesh(mesh)})
    items = {"helmets": [], "suits": [], "gloves": [], "boots": []}
    skipped = []
    for row, asset in tables["HELMETS"].items():
        props = assets.props(asset)
        mesh, material = ref_path(props.get("HelmetMesh")), ref_path(props.get("HelmetMaterial"))
        color = base_color(assets, material)
        if not (mesh and color):
            skipped.append(f"helmet {row}: {'no mesh' if not mesh else 'no picture'}")
            continue
        brand, name = display(row)
        item = {"id": row, "name": name, "brand": brand, "mesh": got.mesh(mesh), "color": got.texture(color)}
        visor_mesh, visor_material = ref_path(props.get("VisorClosedMesh")), ref_path(props.get("VisorMaterial"))
        visor_color = base_color(assets, visor_material)
        if visor_mesh and visor_color:
            item["visor"] = {"mesh": got.mesh(visor_mesh), "color": got.texture(visor_color)}
        items["helmets"].append(item)
    for row, asset in tables["SUITS"].items():
        color = base_color(assets, ref_path(assets.props(asset).get("SuitMaterial")))
        if not color:
            skipped.append(f"suit {row}: no picture")
            continue
        brand, name = display(row)
        items["suits"].append({"id": row, "name": name, "brand": brand, "color": got.texture(color)})
    for row, asset in tables["BOOTS"].items():
        props = assets.props(asset)
        mesh, color = ref_path(props.get("BootsMesh")), base_color(assets, ref_path(props.get("BootsMaterial")))
        if not (mesh and color):
            skipped.append(f"boots {row}: {'no mesh' if not mesh else 'no picture'}")
            continue
        brand, name = display(row)
        items["boots"].append({"id": row, "name": name, "brand": brand, "mesh": got.mesh(mesh), "color": got.texture(color)})
    for row, asset in tables["GLOVES"].items():
        props = assets.props(asset)
        picture, alpha = ref_path(props.get("GlovesBaseColor")), ref_path(props.get("GlovesAlpha"))
        if not (picture and alpha):
            skipped.append(f"gloves {row}: no picture")
            continue
        brand, name = display(row)
        items["gloves"].append({"id": row, "name": name, "brand": brand, "picture": picture, "alpha": alpha})
    return bodies, items, got, skipped


def run(tool, mode, packages, folder, *extra):
    """Run a tool mode over `{key: package}`; the failures as a list of lines."""
    os.makedirs(folder, exist_ok=True)
    list_file = os.path.join(folder, "_list.txt")
    with open(list_file, "w", encoding="utf-8") as f:
        f.write("".join(f"{package}\t{key}\n" for key, package in packages.items()))
    answer = tool.request(mode, list_file, folder, *extra)
    os.remove(list_file)
    return [line for line in answer if not line.startswith("OK")]


def dedupe(keys, folder, ext):
    """Renames each `<key>.<ext>` to the hash of its content; `{key: content key}` and `{content key: [keys]}`."""
    mapping, files = {}, {}
    for key in keys:
        path = os.path.join(folder, f"{key}.{ext}")
        if not os.path.exists(path):
            continue
        content = sha(path)
        mapping[key] = content
        files.setdefault(content, []).append(key)
        target = os.path.join(folder, f"{content}.{ext}")
        if os.path.exists(target) and target != path:
            os.remove(path)
            continue
        os.replace(path, target)
    return mapping, files


def texture_summary(path):
    """`{format, width, height, mips, bytes}` of an `R6TX` file."""
    import struct

    with open(path, "rb") as f:
        head = f.read(25)
        code, _srgb, count = struct.unpack("<IBI", head[8:17])
        width, height = struct.unpack("<II", head[17:25])
    names = {1: "BC1", 2: "BC2", 3: "BC3", 255: "PNG"}
    return {"format": names.get(code, str(code)), "width": width, "height": height, "mips": count, "bytes": os.path.getsize(path)}


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--out", default=r"C:\r6work\rider-gear")
    parser.add_argument("--max", type=int, default=1024, help="longest side of the largest picture kept (default 1024)")
    args = parser.parse_args()
    tool = Tool()
    assets = Assets(tool)
    try:
        bodies, items, got, skipped = collect(assets)
        glove_paths = {}
        for glove in items["gloves"]:
            glove_paths[stem_of(glove["picture"])] = glove["picture"]
            glove_paths[stem_of(glove["alpha"])] = glove["alpha"]
        mesh_dir, tex_dir = os.path.join(args.out, "meshes"), os.path.join(args.out, "tex")
        failures = run(tool, "gmesh", got.meshes, mesh_dir)
        failures += run(tool, "gtex", got.textures, tex_dir, args.max)
        probe_dir = os.path.join(args.out, "probe")
        failures += run(tool, "gtex", glove_paths, probe_dir, gear_colors.PROBE)
        mesh_map, mesh_files = dedupe(got.meshes, mesh_dir, "r6rm")
        tex_map, tex_files = dedupe(got.textures, tex_dir, "r6tx")
        for glove in items["gloves"]:
            picture, alpha = stem_of(glove.pop("picture")), stem_of(glove.pop("alpha"))
            glove["rgb"] = gear_colors.average(os.path.join(probe_dir, f"{picture}.r6tx"), os.path.join(probe_dir, f"{alpha}.r6tx"))
        refer(bodies, items, mesh_map, tex_map)
        catalogue = {
            "version": 1,
            "bodies": bodies,
            **items,
            "meshes": {k: {"file": f"meshes/{k}.r6rm", "bytes": os.path.getsize(os.path.join(mesh_dir, f"{k}.r6rm")),
                           "sources": sorted(got.meshes[s] for s in v)} for k, v in sorted(mesh_files.items())},
            "textures": {k: {"file": f"tex/{k}.r6tx", **texture_summary(os.path.join(tex_dir, f"{k}.r6tx")),
                             "sources": sorted(got.textures[s] for s in v)} for k, v in sorted(tex_files.items())},
        }
        with open(os.path.join(args.out, "catalogue.json"), "w", encoding="utf-8") as f:
            json.dump(catalogue, f, indent=1)
        for line in failures + skipped:
            print("warning:", line)
        report(catalogue, args.out)
    finally:
        tool.close()


def refer(bodies, items, mesh_map, tex_map):
    """Replaces the temporary keys of the items by the content keys."""
    for body in bodies:
        body["mesh"] = mesh_map[body["mesh"]]
    for kind in ("helmets", "boots"):
        for item in items[kind]:
            item["mesh"], item["color"] = mesh_map[item["mesh"]], tex_map[item["color"]]
            if kind == "helmets" and "visor" in item:
                item["visor"] = {"mesh": mesh_map[item["visor"]["mesh"]], "color": tex_map[item["visor"]["color"]]}
    for item in items["suits"]:
        item["color"] = tex_map[item["color"]]


def report(catalogue, out):
    """The sizes of the catalogue on stdout."""
    meshes = sum(m["bytes"] for m in catalogue["meshes"].values())
    textures = sum(t["bytes"] for t in catalogue["textures"].values())
    counts = {k: len(catalogue[k]) for k in ("bodies", "helmets", "suits", "gloves", "boots")}
    print(f"items {counts}")
    print(f"{len(catalogue['meshes'])} meshes {meshes / 1e6:.1f} MB, {len(catalogue['textures'])} pictures {textures / 1e6:.1f} MB in {out}")


if __name__ == "__main__":
    main()
