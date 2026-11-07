"""Export one RIDE 6 bike: `parts.json`, `glb/` (every part mesh, levels of detail 0 to 3) and `tex/` (PNG textures).

    python -I bikeparts.py <vehicle id> <out folder> [--max-tex 2048]

The vehicle id is the id of the bindings tables (`KTM_450-SMR-2022`). The output is what `ride6-bike-import` reads
(docs/modding/ride6-bikes.md, "parts.json"). Liveries: every `CH_<id>_Livery-NN` row of the `Chassis` table is listed in
`liveries[]` with the material instances it changes; their textures are exported too.
"""

import argparse
import glob
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import r6site  # noqa: F401,E402

import bake  # noqa: E402
import glbtools  # noqa: E402
import liveries as livery_lib  # noqa: E402
import materials as mat  # noqa: E402
import partsel  # noqa: E402
from bindings import Tables  # noqa: E402
from r6lib import Assets, Tool, object_name, ref_path  # noqa: E402

KEEP_LODS = 4
SCHEMA = 1


def slot_materials(assets, mesh_path):
    """`[(slot name, default material instance path)]` of a skeletal mesh."""
    export = None
    for item in assets.exports(mesh_path) or []:
        if "SkeletalMaterials" in item or "StaticMaterials" in item:
            export = item
            break
    if export is None:
        return []
    slots = export.get("SkeletalMaterials") or export.get("StaticMaterials") or []
    out = []
    for slot in slots:
        path = ref_path(slot.get("Material"))
        if path:
            out.append((slot.get("MaterialSlotName") or "", path))
    return out


COMPOSITE = ("bike_lods", "livery_lod")


def is_composite(path):
    """The slot of the far levels of detail (a picture the game composites at run time), which the converter never reads."""
    name = object_name(path)
    return any(tag in name for tag in COMPOSITE)


def part_materials(assets, part, chassis_props):
    """`{glb material name (lower): instance path}` of one part: the mesh's slots with the data asset's overrides."""
    slots = [(slot, default) for slot, default in slot_materials(assets, part["mesh"]) if not is_composite(default)]
    out = {}
    notes = []
    override = part.get("material")
    for slot, default in slots:
        path = default
        if chassis_props is not None:
            for key, value in chassis_props.items():
                if livery_lib.normal(key) == livery_lib.normal(slot) and ref_path(value):
                    path = ref_path(value)
        if override and len(slots) == 1:
            path = override
        out[object_name(default)] = path
    if override and len(slots) > 1:
        for slot, default in slots:
            if livery_lib.normal(slot) in livery_lib.normal(object_name(override)):
                out[object_name(default)] = override
    return out, notes


def place(glb_dir, stem, glb_path, role, chassis_bones):
    """The anchor of a part. A body part whose bones move differently is rewritten in the chassis frame (anchor identity)."""
    part = glbtools.Glb(glb_path)
    anchor = glbtools.anchor(part, chassis_bones)
    if anchor is None or role != "body" or bake.is_rigid(part, chassis_bones):
        return anchor
    for level in range(KEEP_LODS):
        path = find_glb(glb_dir, stem, level)
        if path:
            bake.bake(path, chassis_bones)
    return {"bone": anchor["bone"], "translation": [0.0, 0.0, 0.0], "rotation": [0.0, 0.0, 0.0, 1.0]}


def write_lists(path, lines):
    with open(path, "w", encoding="utf-8") as f:
        f.write("\n".join(lines) + "\n")


def export_meshes(tool, meshes, glb_dir):
    answer = tool.request("export", ";".join(meshes), glb_dir, "Gltf2", "All")
    failed = [line for line in answer if not line.startswith("OK")]
    for path in glob.glob(os.path.join(glb_dir, "**", "*_LOD*.glb"), recursive=True):
        n = int(re.search(r"_LOD(\d+)\.glb$", path).group(1))
        if n >= KEEP_LODS:
            os.remove(path)
    return failed


def find_glb(glb_dir, stem, level=None):
    suffix = "" if not level else f"_LOD{level}"
    hits = glob.glob(os.path.join(glb_dir, "**", f"{stem}{suffix}.glb"), recursive=True)
    return hits[0] if hits else None


def smallest_glb(glb_dir, stem):
    for level in (3, 2, 1, 0):
        path = find_glb(glb_dir, stem, level)
        if path:
            return path
    return None


def sockets_of(assets, skeleton_path):
    out = {}
    for item in assets.exports(skeleton_path) or []:
        if item.get("Type") != "SkeletalMeshSocket":
            continue
        p = item.get("Properties") or {}
        name = p.get("SocketName")
        if not name:
            continue
        t = p.get("RelativeLocation") or {}
        r = p.get("RelativeRotation") or {}
        out[name] = {
            "bone": p.get("BoneName"),
            "location_cm": [t.get("X", 0.0), t.get("Y", 0.0), t.get("Z", 0.0)],
            "rotation_deg": [r.get("Pitch", 0.0), r.get("Yaw", 0.0), r.get("Roll", 0.0)],
        }
    return out


def build(vehicle, out_dir, tool, assets, tables, max_tex=2048):
    """Export `vehicle` into `out_dir`; returns the status dict (`ok`, `missing`, ...)."""
    all_liveries = tables.vehicles().get(vehicle)
    if not all_liveries:
        return {"ok": False, "reason": "vehicle has no CHASSIS row"}
    first = min(all_liveries)
    chassis_row = all_liveries[first]
    chassis_asset = tables.asset("Chassis", chassis_row)
    if not assets.named(chassis_asset or ""):
        return {"ok": False, "reason": f"chassis data asset not in the install ({object_name(chassis_asset or '?')})"}
    chassis_props = assets.props(chassis_asset)
    chassis_mesh = ref_path(chassis_props.get("Mesh"))
    if not chassis_mesh or assets.exports(chassis_mesh) is None:
        return {"ok": False, "reason": "chassis mesh not in the install"}

    src = partsel.select(vehicle, tables, assets, chassis_mesh)
    src.parts.insert(0, {"id": "chassis", "role": "body", "mesh": chassis_mesh, "material": None, "origin": chassis_row})
    for part in src.parts:
        part["slots"] = slot_materials(assets, part["mesh"])

    os.makedirs(out_dir, exist_ok=True)
    glb_dir = os.path.join(out_dir, "glb")
    tex_dir = os.path.join(out_dir, "tex")
    meshes = sorted({p["mesh"] for p in src.parts})
    failed = export_meshes(tool, meshes, glb_dir)
    for line in failed:
        src.lose("mesh", "export failed", line)

    chassis_glb_path = smallest_glb(glb_dir, object_name(chassis_mesh))
    if chassis_glb_path is None:
        return {"ok": False, "reason": "chassis mesh did not export"}
    chassis_glb = glbtools.Glb(chassis_glb_path)
    chassis_bones = chassis_glb.bones()

    parts_json = []
    materials_needed = {}
    for part in src.parts:
        stem = object_name(part["mesh"])
        mats, notes = part_materials(assets, part, chassis_props if part["id"] == "chassis" else None)
        src.notes.extend(notes)
        glb_path = smallest_glb(glb_dir, stem)
        if glb_path is None:
            src.lose(part["id"], "mesh did not export", stem)
            continue
        anchor = None
        if part["id"] == "chassis":
            anchor = {"bone": "VEHICLE_BASE", "translation": [0.0, 0.0, 0.0], "rotation": [0.0, 0.0, 0.0, 1.0]}
        if part["id"] != "chassis":
            anchor = place(glb_dir, stem, glb_path, part["role"], chassis_bones)
        if anchor is None:
            anchor = {"bone": None, "translation": [0.0, 0.0, 0.0], "rotation": [0.0, 0.0, 0.0, 1.0]}
            src.notes.append(f"{part['id']}: no shared bone with the chassis skeleton, placed at the chassis origin")
        for name, path in mats.items():
            materials_needed[object_name(path)] = path
        parts_json.append(
            {
                "id": part["id"],
                "role": part["role"],
                "export": stem,
                "anchor": anchor,
                "materials": {name: path for name, path in mats.items()},
                "source": part["origin"],
                "mesh": part["mesh"],
            }
        )

    variants = livery_lib.variants(src, tables, assets, all_liveries, parts_json, chassis_props)
    for item in variants:
        for path in item["all_paths"]:
            materials_needed[object_name(path)] = path

    assets.ensure(list(materials_needed.values()))
    resolved = {}
    for name, path in sorted(materials_needed.items()):
        value = mat.resolve(assets, path)
        if value is None:
            src.lose("material", "material not in install", name)
            continue
        resolved[name] = value
    wanted = {}
    for value in resolved.values():
        for path in mat.textures_of(value):
            wanted[object_name(path)] = path
    assets.ensure(list(wanted.values()))
    os.makedirs(tex_dir, exist_ok=True)
    texture_answer = tex_batch(tool, wanted, tex_dir, max_tex)
    for line in texture_answer:
        if not line.startswith("OK"):
            src.lose("texture", "texture not exported", line)
    textures = texture_table(tex_dir, wanted)

    skeleton = ref_path((assets.named(chassis_mesh) or {}).get("Properties", {}).get("Skeleton"))
    doc = {
        "schema": SCHEMA,
        "vehicle": vehicle,
        "chassis_row": chassis_row,
        "physics_asset": tables.asset("PHYSICS", chassis_row),
        "tyre_sizes": {
            "front": tables.value("TYRE_SIZES", f"{chassis_row}$FrontTyreSize"),
            "back": tables.value("TYRE_SIZES", f"{chassis_row}$BackTyreSize"),
        },
        "lights": object_name(tables.asset("LIGHTS", chassis_row) or ""),
        "bones": {
            name: {"position": [float(v) for v in m[:3, 3]], "rotation": glbtools.matrix_quat(m)}
            for name, m in chassis_bones.items()
        },
        "sockets": sockets_of(assets, skeleton) if skeleton else {},
        "parts": parts_json,
        "materials": {name: material_entry(value) for name, value in resolved.items()},
        "liveries": [livery_lib.public(item) for item in variants],
        "textures": textures,
        "missing": src.missing,
        "notes": src.notes,
    }
    with open(os.path.join(out_dir, "parts.json"), "w", encoding="utf-8") as f:
        json.dump(clean(doc), f, indent=1)
    return {"ok": True, "parts": [p["id"] for p in parts_json], "missing": src.missing, "liveries": len(variants)}


def clean(value):
    """The document with floats rounded to 7 digits (no denormal noise)."""
    if isinstance(value, float):
        return 0.0 if abs(value) < 1e-9 else round(value, 7)
    if isinstance(value, dict):
        return {k: clean(v) for k, v in value.items()}
    if isinstance(value, list):
        return [clean(v) for v in value]
    return value


def material_entry(value):
    out = {k: value[k] for k in ("tex", "scalar", "vector", "switch", "chain") if k in value}
    for key in ("blend_mode", "shading_model", "clip"):
        if key in value:
            out[key] = value[key]
    return out


def tex_batch(tool, wanted, tex_dir, max_tex):
    if not wanted:
        return []
    list_file = os.path.join(tex_dir, "_list.txt")
    write_lists(list_file, list(wanted.values()))
    answer = tool.request("tex", list_file, tex_dir, max_tex)
    os.remove(list_file)
    return answer


def texture_table(tex_dir, wanted):
    out = {}
    for stem in wanted:
        info = os.path.join(tex_dir, stem + ".txt")
        if os.path.exists(info):
            with open(info, encoding="utf-8") as f:
                out[stem] = f.read().strip()
    return out


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("vehicle")
    parser.add_argument("out")
    parser.add_argument("--max-tex", type=int, default=2048)
    args = parser.parse_args()
    tool = Tool()
    try:
        assets = Assets(tool)
        tables = Tables(assets)
        status = build(args.vehicle, args.out, tool, assets, tables, args.max_tex)
    finally:
        tool.close()
    print(json.dumps(status, indent=1))
    return 0 if status.get("ok") else 1


if __name__ == "__main__":
    sys.exit(main())
