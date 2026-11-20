"""The surfaces of a bike that the importer draws grey because no picture of the bike reaches them, for `completeness.py`.

The same lookup as `mods/ride6-bikes/src/partfile.rs` (`PartTable::material`): the instance the part names for the surface, else
the chassis' instance of the same kind (`mat_livery_1` takes the livery), else, for the far levels' composite slot, the part's own
first picture; a picture is the first of the base colour parameters that is not one of the engine's flat defaults.
"""

import os

from completeness_glb import level_file, primitives

KINDS = ("mat_livery", "mat_mechanics", "mat_cockpit", "mat_gear", "mat_light", "mat_lights")
FLAT = ("t_default", "t_white", "t_black")


def stem(path):
    return os.path.basename(path.replace("\\", "/")).split(".")[0].lower()


def kind(name):
    end = next((i for i, c in enumerate(name) if c.isdigit()), len(name))
    text = name[:end].rstrip("_")
    return text if text in KINDS else None


def picture(material):
    """The stem of the base colour picture of a resolved material, or None."""
    switch = material.get("switch", {})
    first, second = ("BaseColorOpacity - Texture", "BaseColor - Texture")
    if not switch.get("BaseColorOpacity - Source"):
        first, second = second, first
    for key in (first, second, "BaseColor", "BaseColor0_Map"):
        path = material.get("tex", {}).get(key)
        if path and not stem(path).startswith(FLAT):
            return stem(path)
    return None


def resolve(table, part, glb_name, materials):
    """The material the importer draws the surface `glb_name` of `part` with, or None (grey)."""
    low = glb_name.lower()
    own = {k.lower(): v for k, v in part["materials"].items()}
    instance = stem(own[low]) if low in own else stem(glb_name)
    found = materials.get(instance)
    if found is not None:
        return found
    if "bike_lods" in low or "livery_lod" in low:
        for _, path in sorted(own.items()):
            material = materials.get(stem(path))
            if material is not None and picture(material):
                return material
        return None
    wanted = kind(low)
    chassis = next(p for p in table["parts"] if p["id"] == "chassis")
    if wanted is None:
        return None
    for slot, path in sorted((k.lower(), v) for k, v in chassis["materials"].items()):
        material = materials.get(stem(path))
        if kind(slot) == wanted and material is not None and picture(material):
            return material
    return None


def grey_surfaces(table, glbs, tex_files):
    """`[(part, level, material, triangles, reason)]` of the surfaces drawn grey, at levels 0 to 3."""
    materials = {k.lower(): v for k, v in table["materials"].items()}
    out = []
    for part in table["parts"]:
        for level in range(4):
            path = level_file(glbs, part["export"], level)
            if path is None:
                continue
            for name, _, triangles in primitives(path):
                if "glassdecal" in name.lower():
                    continue
                material = resolve(table, part, name, materials)
                if material is None:
                    out.append((part["id"], level, name, triangles, "no instance"))
                    continue
                if material.get("blend_mode", "").startswith("BLEND_Translucent"):
                    continue
                pic = picture(material)
                if pic is None:
                    out.append((part["id"], level, name, triangles, "no base colour"))
                elif pic + ".png" not in tex_files:
                    out.append((part["id"], level, name, triangles, f"picture {pic} not exported"))
    return out
