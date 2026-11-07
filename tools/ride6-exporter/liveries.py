"""The liveries of a bike: one `CH_<id>_Livery-NN` row of the `Chassis` table each.

A livery is the chassis data asset (`bikeNNN_00N`): its `Livery_0`, `Livery_1`, `Mechanics`, `Cockpit` and `Glass` material
instances replace the mesh's slots of the same name. The parts that change with the livery have rows with the same suffix
(`RI_Stock_<id>_Livery-02` rims, `PAD_Stock_..` bar pad, `HGU_Stock_..` hand guards). `parts.json` lists, for each livery, the
material instance of every material name of the base export (`liveries[].materials` for the chassis, `liveries[].parts`
for the others) so a livery is a set of replaced pictures on the same meshes.
"""

import re

from r6lib import object_name, ref_path


def normal(text):
    return re.sub(r"[^a-z0-9]", "", text.lower().replace("mat_", "", 1))


def chassis_materials(chassis_part, props):
    """`{base glb material name: instance path}` for the slots the data asset `props` replaces."""
    out = {}
    for slot, default in chassis_part["slots"]:
        for key, value in props.items():
            path = ref_path(value)
            if path and normal(key) == normal(slot):
                out[object_name(default)] = path
    return out


def part_variant(assets, parts_json, part_ids, asset):
    """`{part id: {glb material name: instance path}}` for rim, bar pad or hand guard data asset `asset`."""
    props = assets.props(asset)
    material = ref_path(props.get("Material"))
    if not material:
        return {}
    out = {}
    for part in parts_json:
        if part["id"] not in part_ids:
            continue
        names = list(part["materials"])
        if len(names) == 1:
            out[part["id"]] = {names[0]: material}
    return out


PART_GROUPS = {"rims": ("front_rim", "back_rim"), "barpad": ("barpad",), "handguards": ("handguards",)}


def variants(src, tables, assets, all_liveries, parts_json, chassis_props):
    """The liveries of the vehicle in order, each `{index, row, materials, parts, all_paths}`."""
    chassis_part = next(p for p in src.parts if p["id"] == "chassis")
    extra = {}
    for part_id, table, prefix in (
        ("rims", "RIMS", "RI_Stock_"),
        ("barpad", "BARPADS", "PAD_Stock_"),
        ("handguards", "HANDGUARDS", "HGU_Stock_"),
    ):
        extra[part_id] = (table, prefix)
    out = []
    for n in sorted(all_liveries):
        row = all_liveries[n]
        asset = tables.asset("Chassis", row)
        if not asset or assets.named(asset) is None:
            src.notes.append(f"livery {n}: chassis data asset not in the install")
            continue
        item = {"index": n, "row": row, "materials": chassis_materials(chassis_part, assets.props(asset)), "parts": {}}
        for part_id, (table, prefix) in extra.items():
            key = f"{prefix}{src.vehicle}_Livery-{n:02d}"
            part_asset = tables.asset(table, key)
            if not part_asset or assets.named(part_asset) is None:
                continue
            item["parts"].update(part_variant(assets, parts_json, PART_GROUPS[part_id], part_asset))
        paths = list(item["materials"].values())
        for mapping in item["parts"].values():
            paths.extend(mapping.values())
        item["all_paths"] = paths
        out.append(item)
    return out


def public(item):
    return {k: v for k, v in item.items() if k != "all_paths"}
