"""Surfaces that no picture reaches take the surface of a similar model: `parts.json` gets the donor's material and its pictures.

    python -I fallbacks.py --out C:\\r6work\\bikes [--donors <folder of bike folders>] [--only <regex>] [--dry-run] [--md fallbacks.md]

For every bike folder of `--out` the surfaces the converter would draw grey (`completeness_mat.grey_surfaces`: no instance, an
instance with no base colour picture, a picture that was not exported) are listed at all four levels of detail, and each one is
given a material by `fallbacks_pick.choose` from the bikes of `--donors` (default: `--out`). The donor's resolved material is
added to `materials{}`, `parts[].materials{}` names it for the surface, its pictures are copied into `tex/` and every choice is
written to `fallbacks[]` (part, surface, picture, source bike, rule); the importer puts each one into the manifest as a comment.
A bike with nothing grey is not touched, and a second run finds nothing to do. Rules:
docs/modding/ride6-bikes-batch.md#grey-surfaces-the-fallbacks.
"""

import argparse
import copy
import json
import os
import re
import shutil
import struct
import sys
import zlib

sys.path.append(os.path.dirname(os.path.abspath(__file__)))

from completeness_glb import index_glbs, level_file, primitives  # noqa: E402
from completeness_mat import FLAT, grey_surfaces, picture, stem  # noqa: E402
from fallbacks_names import surface_kind  # noqa: E402
from fallbacks_pick import RULES, Found, Part, choose  # noqa: E402

# Where the importer is told a material came from (`partfile.rs`: the stem of the last path component is the material's name).
SOURCE = "/Fallback/{bike}/{name}.0"
COLOURS = ("BaseColor_Modulation", "Modulation - BaseColor - Max")


class Folder:
    """One bike folder: its part table, its meshes and its pictures."""

    def __init__(self, root, vehicle):
        self.vehicle = vehicle
        self.dir = os.path.join(root, vehicle)
        with open(os.path.join(self.dir, "parts.json"), encoding="utf-8") as f:
            self.table = json.load(f)
        self.glbs = index_glbs(self.dir)
        tex = os.path.join(self.dir, "tex")
        self.tex = {n.lower() for n in os.listdir(tex)} if os.path.isdir(tex) else set()
        self.materials = {k.lower(): v for k, v in self.table["materials"].items()}

    def part(self, part_id):
        return next((p for p in self.table["parts"] if p["id"] == part_id), None)


def usable(entry, tex):
    """Whether a resolved material has a picture to draw: a base colour (an exported file), or for glass its transmittance."""
    if entry is None:
        return False
    textures = entry.get("tex", {})
    if entry.get("blend_mode", "").startswith("BLEND_Translucent"):
        keys = ("Transmittance - Texture", "BaseColor - Texture", "BaseColorOpacity - Texture")
        return any(textures.get(k) and stem(textures[k]) + ".png" in tex for k in keys)
    pic = picture(entry)
    return bool(pic) and pic + ".png" in tex


def read_part(folder, part):
    """The `fallbacks_pick.Part` of a part at its finest level: its shape and its usable surfaces by kind."""
    path = level_file(folder.glbs, part["export"], 0)
    if path is None:
        return Part(part["export"], (), {})
    slots = {k.lower(): v for k, v in part.get("slots", {}).items()}
    own = {k.lower(): v for k, v in part["materials"].items()}
    signature, surfaces = [], {}
    for name, _, triangles in primitives(path):
        if "glassdecal" in name.lower():
            continue
        kind = surface_kind(name, slots.get(name.lower()))
        signature.append((kind or name.lower(), triangles))
        source = own.get(name.lower())
        if not source or source.startswith("/Fallback/") or kind is None:
            continue
        entry = folder.materials.get(stem(source))
        if usable(entry, folder.tex):
            surfaces.setdefault(kind, Found(folder.vehicle, part["id"], name, stem(source), entry))
    return Part(part["export"], tuple(sorted(signature)), surfaces)


class Library:
    """The bikes donors are taken from, read once (the shapes of a part only when a search asks for that part)."""

    def __init__(self, root):
        self.root = root
        self.names = sorted(v for v in os.listdir(root) if os.path.isfile(os.path.join(root, v, "parts.json")))
        self.folders = {}
        self.parts = {}

    def folder(self, vehicle):
        if vehicle not in self.folders:
            self.folders[vehicle] = Folder(self.root, vehicle)
        return self.folders[vehicle]

    def of(self, part_id, exclude):
        """`{vehicle: {part id: Part}}` of every bike but `exclude`, for the one part id."""
        out = {}
        for vehicle in self.names:
            if vehicle == exclude:
                continue
            key = (vehicle, part_id)
            if key not in self.parts:
                part = self.folder(vehicle).part(part_id)
                self.parts[key] = read_part(self.folder(vehicle), part) if part else None
            if self.parts[key] is not None:
                out[vehicle] = {part_id: self.parts[key]}
        return out


def own_parts(folder):
    return {p["id"]: read_part(folder, p) for p in folder.table["parts"]}


def flat_colour(entry):
    """The sRGB colour a pictureless material stands for: its constant colour (linear) if it has one, else mid grey."""
    vector = (entry or {}).get("vector", {})
    for key in COLOURS:
        if key in vector:
            return [round(255 * (c ** (1 / 2.2))) for c in vector[key][:3]]
    return [128, 128, 128]


def write_png(path, rgb):
    """A 4 by 4 picture of one colour."""
    raw = b"".join(b"\x00" + bytes(rgb) * 4 for _ in range(4))
    def chunk(tag, data):
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data))
    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 4, 4, 8, 2, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b""))


def bring_pictures(donor, target, entry):
    """Copy the pictures a donor material reads (and its instrument cluster composition) into the target's `tex/`."""
    os.makedirs(os.path.join(target.dir, "tex"), exist_ok=True)
    for path in entry.get("tex", {}).values():
        name = stem(path)
        if name.startswith(FLAT):
            continue
        for suffix in (".png", ".txt", "_dash.png"):
            source = os.path.join(donor.dir, "tex", name + suffix)
            dest = os.path.join(target.dir, "tex", name + suffix)
            if os.path.isfile(source) and not os.path.isfile(dest):
                shutil.copyfile(source, dest)
        info = os.path.join(target.dir, "tex", name + ".txt")
        if os.path.isfile(info):
            with open(info, encoding="utf-8") as f:
                target.table.setdefault("textures", {})[name] = f.read().strip()


def adopt(target, part, glb, found, donor):
    """Give the surface `glb` of `part` the material of `found`; returns the material's name in the target."""
    name = found.material
    existing = target.materials.get(name)
    if existing is not None and existing != found.entry:
        name = f"{name}-fb"
    target.table["materials"][name] = copy.deepcopy(found.entry)
    target.materials[name] = target.table["materials"][name]
    part["materials"][glb.lower()] = SOURCE.format(bike=found.vehicle, name=name)
    bring_pictures(donor, target, found.entry)
    return name


def adopt_flat(target, part, glb, colour):
    """Give the surface a flat colour: a 4 by 4 picture and a material that reads it."""
    name = "flat_%02x%02x%02x" % tuple(colour)
    os.makedirs(os.path.join(target.dir, "tex"), exist_ok=True)
    write_png(os.path.join(target.dir, "tex", name + ".png"), colour)
    entry = {"tex": {"BaseColor - Texture": f"/Fallback/flat/{name}.0"}, "scalar": {}, "vector": {}, "switch": {}, "chain": [name],
             "blend_mode": "BLEND_Opaque"}
    target.table["materials"][name] = entry
    target.materials[name] = entry
    part["materials"][glb.lower()] = SOURCE.format(bike="flat", name=name)
    return name


def surfaces_to_fix(folder):
    """`{(part id, surface name): (levels, triangles at the finest level, reason)}` of the grey surfaces of a bike."""
    out = {}
    for part_id, level, name, triangles, reason in grey_surfaces(folder.table, folder.glbs, folder.tex):
        levels, first, _ = out.get((part_id, name), ([], triangles, reason))
        out[(part_id, name)] = (levels + [level], first if levels else triangles, reason)
    return out


def fix_bike(folder, library, dry_run=False):
    """Choose and apply the fallbacks of one bike; returns the records written to `fallbacks[]`."""
    todo = surfaces_to_fix(folder)
    if not todo:
        return []
    mine = own_parts(folder)
    records = []
    for (part_id, name), (levels, triangles, reason) in sorted(todo.items()):
        part = folder.part(part_id)
        slot = part.get("slots", {}).get(name.lower())
        kind = surface_kind(name, slot)
        shape = mine[part_id]
        entry = folder.materials.get(stem(part["materials"].get(name.lower(), name)))
        choice = choose(folder.vehicle, part_id, part["export"], shape.signature, kind, library.of(part_id, folder.vehicle),
                        mine, flat_colour(entry))
        record = {"part": part_id, "surface": name, "slot": slot, "kind": kind, "levels": sorted(levels), "triangles": triangles,
                  "reason": reason, "rule": choice.rule}
        if choice.found is not None:
            record.update(source=choice.found.vehicle, source_part=choice.found.part, source_surface=choice.found.glb,
                          material=choice.found.material, picture=picture(choice.found.entry) or "transmittance")
            if not dry_run:
                adopt(folder, part, name, choice.found, library.folder(choice.found.vehicle))
        if choice.found is None:
            record.update(source=None, material="flat", picture="%02x%02x%02x" % tuple(choice.colour))
            if not dry_run:
                adopt_flat(folder, part, name, choice.colour)
        records.append(record)
    if not dry_run:
        folder.table.setdefault("fallbacks", []).extend(records)
        with open(os.path.join(folder.dir, "parts.json"), "w", encoding="utf-8", newline="\n") as f:
            json.dump(folder.table, f, indent=1)
    return records


def table(rows):
    """The markdown table of the choices: `[(vehicle, record)]`."""
    out = ["| Bike | Part | Surface | Triangles at LOD0 | Picture | Source bike | Rule |", "| --- | --- | --- | --- | --- | --- | --- |"]
    for vehicle, r in rows:
        source = r["source"] or "none"
        out.append(f"| {vehicle} | {r['part']} | {r['surface']} | {r['triangles'] if 0 in r['levels'] else 0} | {r['picture']} | "
                   f"{source} | {r['rule']} |")
    return out


def run(out_dir, donors=None, only=None, dry_run=False):
    """Fix every bike folder of `out_dir` (those matching `only`); returns `[(vehicle, record)]`."""
    library = Library(donors or out_dir)
    mine = Library(out_dir)
    rows = []
    for vehicle in mine.names:
        if only and not re.search(only, vehicle):
            continue
        folder = mine.folder(vehicle)
        rows += [(vehicle, r) for r in fix_bike(folder, library, dry_run)]
    return rows


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--out", default=r"C:\r6work\bikes", help="the bike folders to fix")
    ap.add_argument("--donors", help="the bike folders the donors are taken from (default: --out)")
    ap.add_argument("--only", help="regex on the vehicle id")
    ap.add_argument("--dry-run", action="store_true", help="print the choices, write nothing")
    ap.add_argument("--md", help="write the table of the choices here")
    args = ap.parse_args()
    rows = run(args.out, args.donors, args.only, args.dry_run)
    for vehicle, r in rows:
        print(f"{vehicle}: {r['part']} {r['surface']} ({r['triangles']} triangles, levels {r['levels']}) <- {r['source']} "
              f"{r['material']} [{r['rule']}: {RULES[r['rule']]}]")
    print(len(rows), "surfaces")
    if args.md:
        with open(args.md, "w", encoding="utf-8", newline="\n") as f:
            f.write("\n".join(table(rows)) + "\n")


if __name__ == "__main__":
    main()
