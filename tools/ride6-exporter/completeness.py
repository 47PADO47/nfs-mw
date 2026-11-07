"""The completeness report: the stock parts RIDE 6 draws for each bike against the solids the importer wrote.

    python -I completeness.py --out C:\\r6work\\bikes --mod <mod folder> [--only REGEX] [--detail VEHICLE ...] [--md FILE]

Three sources, none of them the importer's own idea of what it meant to write:

- **RIDE 6's data**: the bindings tables (`graphics/Bindings/Bike/*`, from the JSON cache of `r6lib`) say which rows a vehicle has;
  the chassis' skeletal mesh (its `LODModels` sections) says what the game draws at each level of detail; the exporter's
  `parts.json` and `glb/` say what was exported.
- **The exports**: every part's triangles at each level (`glb_tris`), the glass decals left out (the importer draws none).
- **The car files**: `ride6-bike-import inventory <mod folder>` reads every `GEOMETRY.BIN` back, solid by solid.

For every bike and level letter the triangles of the car's solids are compared with the triangles of the parts that belong in
them at the level the importer chose (`built.json`, `levels`): a difference is geometry that was dropped. Written for
docs/modding/ride6-bikes-batch.md#completeness.
"""

import argparse
import json
import os
import re
import subprocess
import sys

sys.path.append(os.path.dirname(os.path.abspath(__file__)))

from completeness_glb import glb_tris, index_glbs  # noqa: E402
from completeness_mat import grey_surfaces  # noqa: E402

CACHE = os.environ.get("R6_CACHE", r"C:\r6work\cache")
BIND = os.path.join(CACHE, "json", "ride6", "content", "assets", "graphics", "bindings", "bike")
IMPORT = os.environ.get("R6_IMPORT", r"C:\Users\PADO\github\nfs-mw-build\release\ride6-bike-import.exe")
LETTERS = "ABCD"
# The pieces the shop can swap (`build_pieces.rs`): solid tag -> the part ids it holds.
PIECES = {
    "EXHAUST": ("exmanifold", "muffler"),
    "BARPAD": ("barpad",),
    "FRONT_RIM": ("front_rim",),
    "REAR_RIM": ("back_rim",),
    "FRONT_DISC": ("front_brake",),
    "REAR_DISC": ("back_brake",),
}
# Optional rows of the bindings tables and the part each gives: table -> (part id, row key prefix).
OPTIONAL = {
    "barpads": ("barpad", "PAD_Stock_"),
    "handguards": ("handguards", "HGU_Stock_"),
    "side_mirrors": ("mirrors", "MI_Stock_"),
    "homologation": ("homologation", "HML_Stock_"),
    "enduro_preset": ("enduro_preset", "EP_Stock_"),
}
ALWAYS = ("front_tyre", "back_tyre", "front_rim", "back_rim", "front_brake", "back_brake", "exmanifold", "muffler")


def tables():
    out = {}
    for name in list(OPTIONAL) + ["exmanifold", "muffler", "rims", "brakes_kit", "sump_guard"]:
        path = os.path.join(BIND, name + ".json")
        if os.path.isfile(path):
            with open(path, encoding="utf-8") as f:
                out[name] = (json.load(f)[0].get("Rows") or {}).keys()
    return out


def run_tool(*args):
    done = subprocess.run([IMPORT, *args], capture_output=True, text=True, encoding="utf-8")
    if done.returncode != 0:
        sys.exit(f"{IMPORT} {args[0]} failed: {done.stderr.strip()}")
    return done.stdout.splitlines()


def car_ids(vehicles):
    """vehicle id -> car id, by the importer's own naming rule (`ids`)."""
    list_file = os.path.join(os.environ.get("TEMP", "."), "completeness-ids.txt")
    with open(list_file, "w", encoding="utf-8") as f:
        f.write("\n".join(vehicles) + "\n")
    return {line.split("\t")[0]: line.split("\t")[1] for line in run_tool("ids", "--file", list_file)}


def inventory(mod, only=None):
    args = ["inventory", mod] + (["--only", only] if only else [])
    cars = {}
    for line in run_tool(*args):
        car = json.loads(line)
        cars[car["car"]] = {s["name"]: s for s in car["solids"]}
    return cars


def chassis_sections(number):
    """Triangles per level of the chassis' skeletal mesh as the game stores them (sections that are not disabled)."""
    path = os.path.join(CACHE, "json", "ride6", "content", "assets", "graphics", "vehicles", "bikes", "chassis",
                        f"chassis{number}", "skeletal_meshes", f"chassis{number}.json")
    if not os.path.isfile(path):
        return None
    with open(path, encoding="utf-8") as f:
        data = json.load(f)
    mesh = next((e for e in data if e.get("Type") == "SkeletalMesh"), None)
    if mesh is None:
        return None
    return [sum(s["NumTriangles"] for s in lod["Sections"] if not s.get("bDisabled")) for lod in mesh["LODModels"]]


def apart_parts(solids):
    """The part ids the importer built as solids of their own (the pieces of a bike with shop options)."""
    names = " ".join(solids)
    return {pid for tag, ids in PIECES.items() if f"_KIT00_{tag}_A" in names for pid in ids}


def body_row(parts, glbs, solids, apart, letter, pair):
    """(expected triangles, triangles in the car, solids of the frame) of the body of `letter` built at the levels `pair`.

    The frame is the body solid and `BODY2`, `BODY3`, ... when it does not fit one (`chunks.rs`); the bars and the fork likewise.
    """
    chassis_level, other = pair
    body = [p for p in parts if p["role"] == "body" and p["id"] not in apart]
    want = sum(glb_tris(glbs, p["export"], chassis_level if p["id"] == "chassis" else other) for p in body)
    frame = [s for name, s in solids.items() if re.search(rf"_KIT00_BODY\d*_{letter}$", name)]
    steered = [s for name, s in solids.items() if re.search(rf"_KIT00_(BARS|FORK)\d*_{letter}$", name)]
    return want, sum(s["triangles"] for s in frame + steered), len(frame)


def infer_levels(parts, glbs, solids):
    """The levels each letter was built at, found by matching triangles (for car files older than `built.json`'s `levels`)."""
    apart = apart_parts(solids)
    out = []
    for letter in LETTERS:
        found = None
        for chassis_level in range(4):
            for other in range(max(0, chassis_level - 1), 4):
                want, have, _ = body_row(parts, glbs, solids, apart, letter, (chassis_level, other))
                if want == have and found is None:
                    found = [chassis_level, other]
        if found is None:
            return []
        out.append(found)
    return out


def expected_solids(parts, glbs, solids, levels):
    """For each letter: {solid group: (expected triangles, present triangles)} for the body, the wheels and the pieces."""
    apart = apart_parts(solids)
    out = {}
    for letter in LETTERS:
        n = LETTERS.index(letter)
        get = lambda tag: next((s for name, s in solids.items() if name.endswith(f"_KIT00_{tag}_{letter}")), None)
        want, have, frame = body_row(parts, glbs, solids, apart, letter, levels[n])
        rows = {f"body ({frame} solids), bars and fork": (want, have)}
        for role, tag in (("front_wheel", "FRONT_TIRE"), ("back_wheel", "REAR_TIRE")):
            ps = [p for p in parts if p["role"] == role and p["id"] not in apart]
            rows[tag.lower()] = (sum(glb_tris(glbs, p["export"], n) for p in ps), (get(tag) or {"triangles": 0})["triangles"])
        for tag, ids in PIECES.items():
            if not all(i in apart for i in ids) or not any(p["id"] in ids for p in parts):
                continue
            ps = [p for p in parts if p["id"] in ids]
            rows[tag.lower()] = (sum(glb_tris(glbs, p["export"], n) for p in ps), (get(tag) or {"triangles": 0})["triangles"])
        out[letter] = rows
    return out


def analyse(out_dir, vehicle, ids, cars, binds, infer=False):
    folder = os.path.join(out_dir, vehicle)
    with open(os.path.join(folder, "parts.json"), encoding="utf-8") as f:
        table = json.load(f)
    built = {}
    if os.path.isfile(os.path.join(folder, "built.json")):
        with open(os.path.join(folder, "built.json"), encoding="utf-8") as f:
            built = json.load(f)
    glbs = index_glbs(folder)
    parts = table["parts"]
    have = {p["id"] for p in parts}
    lost = {m["part"]: m["reason"] for m in table.get("missing", [])}
    result = {"vehicle": vehicle, "car": ids.get(vehicle), "problems": [], "parts": {}, "fallbacks": len(table.get("fallbacks", []))}
    # Rows of the tables against the exported parts: a row without a part, and without a stated reason, is a silent drop.
    lower = vehicle.lower()
    for name, (pid, prefix) in OPTIONAL.items():
        rows = [k for k in binds.get(name, ()) if k.lower().startswith(prefix.lower()) and lower in k.lower()]
        row = bool(rows)
        status = "ok" if pid in have else (lost.get(pid) or ("no row" if not row else "SILENTLY DROPPED"))
        if row or pid in have:
            result["parts"][pid] = status
        if row and pid not in have and pid not in lost:
            result["problems"].append(f"{pid}: row {rows[0]} but no part and no reason")
    for pid in ALWAYS:
        status = "ok" if pid in have else (lost.get(pid) or "SILENTLY DROPPED")
        result["parts"][pid] = status
        if pid not in have:
            result["problems"].append(f"{pid}: {status}")
    tex = {f.lower() for f in os.listdir(os.path.join(folder, "tex"))} if os.path.isdir(os.path.join(folder, "tex")) else set()
    result["grey"] = grey_surfaces(table, glbs, tex)
    for part_id, level, name, triangles, reason in result["grey"]:
        if level == 0:
            result["problems"].append(f"{part_id}: {name} ({triangles} triangles at LOD0) is drawn grey: {reason}")
    chassis = next((p for p in parts if p["id"] == "chassis"), None)
    result["chassis_tris"] = [glb_tris(glbs, chassis["export"], n) for n in range(4)] if chassis else []
    number = re.search(r"chassis(\d+)", chassis["export"], re.I).group(1) if chassis else None
    sections = chassis_sections(number) if number else None
    result["game_tris"] = sections[:4] if sections else None
    decals = [glb_tris(glbs, chassis["export"], n, decals_only=True) for n in range(4)] if chassis else []
    # The glTF export drops a few degenerate triangles (4 of 193,186 on the Z900); more than half a percent is a loss.
    if sections and chassis and any(g - (a + b) > 0.005 * g for a, b, g in zip(result["chassis_tris"], decals, sections)):
        result["problems"].append("the exported chassis does not hold the triangles of the game's skeletal mesh")
    result["levels"] = [] if infer else built.get("levels") or []
    solids = cars.get(ids.get(vehicle), None)
    if solids is None:
        result["problems"].append("no car files")
        return result
    if not result["levels"]:
        result["levels"] = infer_levels(parts, glbs, solids)
    if not result["levels"]:
        result["problems"].append("no combination of levels of detail adds up to the body's triangles (dropped geometry)")
        return result
    result["solids"] = expected_solids(parts, glbs, solids, result["levels"])
    for letter, rows in result["solids"].items():
        for group, (want, got) in rows.items():
            if got != want:
                result["problems"].append(f"{letter} {group}: {got} triangles, parts have {want}")
    return result


def line(cells):
    return "| " + " | ".join(str(c) for c in cells) + " |"


def detail(r):
    out = [f"#### {r['vehicle']} (`{r['car']}`)", ""]
    out += [line(["Part", "Status"]), line(["---", "---"])]
    out += [line([pid, status]) for pid, status in r["parts"].items()]
    out += ["", f"Chassis triangles at levels 0 to 3 (the export, without glass decals): {r['chassis_tris']}; the game's skeletal mesh "
            f"(all sections, with the decals): {r['game_tris']}.", ""]
    if "solids" in r:
        out += [line(["Level", "Chassis / other parts at", "Solid group", "Expected triangles", "In the car", "Result"]),
                line(["---", "---", "---", "---", "---", "---"])]
        for letter, rows in r["solids"].items():
            at = r["levels"][LETTERS.index(letter)]
            for group, (want, got) in rows.items():
                out.append(line([letter, f"LOD{at[0]} / LOD{at[1]}", group, want, got, "ok" if want == got else f"{got - want:+d}"]))
        out.append("")
    out += [f"Problems: {len(r['problems'])}" + "".join(f"\n- {p}" for p in r["problems"]), ""]
    return out


def summary(results):
    out = [line(["Bike", "Car", "Chassis LOD at A", "Triangles of LOD0 kept at A", "Parts missing", "Dropped triangles",
                "Grey triangles at LOD0", "Surfaces on a similar model's material", "Problems"]),
           line(["---"] * 9)]
    for r in results:
        lv = r["levels"][0] if r["levels"] else None
        tris = r["chassis_tris"]
        kept = f"{tris[lv[0]] / tris[0]:.0%}" if lv and tris and tris[0] else "?"
        gone = [k for k, v in r["parts"].items() if v not in ("ok", "no row")]
        dropped = sum(want - got for rows in r.get("solids", {}).values() for want, got in rows.values() if got < want)
        grey = sum(t for _, level, _, t, _ in r["grey"] if level == 0)
        out.append(line([r["vehicle"], r["car"], f"LOD{lv[0]}" if lv else "?", kept, ", ".join(gone) or "none", dropped,
                         grey, r["fallbacks"], len(r["problems"])]))
    return out


def parts_matrix(results):
    """One row per bike, one column per stock part: `ok` (in the car), or the reason it is not (`no row` is a part the game has no row
    for: most road bikes have no bar pad or hand guards), and the solids of the frame at the finest level."""
    columns = ["front_tyre", "back_tyre", "front_rim", "back_rim", "front_brake", "back_brake", "exmanifold", "muffler",
               "barpad", "handguards", "mirrors", "homologation"]
    out = [line(["Bike", "Frame solids at A"] + columns), line(["---"] * (2 + len(columns)))]
    for r in results:
        frames = next(iter(r.get("solids", {}).get("A", {})), "")
        count = re.search(r"\((\d+) solids", frames)
        cells = [r["parts"].get(c, "no row").replace("asset not in install", "not shipped") for c in columns]
        out.append(line([r["vehicle"], count.group(1) if count else "?"] + cells))
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--out", default=r"C:\r6work\bikes")
    ap.add_argument("--mod", required=True, help="the mod folder with files/CARS")
    ap.add_argument("--only", help="regex on the vehicle id")
    ap.add_argument("--detail", nargs="*", default=[], help="vehicles to print a table for")
    ap.add_argument("--md", help="write the report here (default: print)")
    ap.add_argument("--matrix", action="store_true", help="add the table of stock parts per bike")
    ap.add_argument("--infer", action="store_true", help="find the levels of detail by the triangles, not from built.json (car files of an older import)")
    args = ap.parse_args()
    vehicles = sorted(v for v in os.listdir(args.out) if os.path.isfile(os.path.join(args.out, v, "parts.json"))
                      and (not args.only or re.search(args.only, v)))
    ids = car_ids(sorted(v for v in os.listdir(args.out) if os.path.isfile(os.path.join(args.out, v, "parts.json"))))
    cars = inventory(args.mod)
    binds = tables()
    results = [analyse(args.out, v, ids, cars, binds, args.infer) for v in vehicles]
    text = ["### Summary", ""] + summary(results) + [""]
    text += [f"{len(results)} bikes, {sum(1 for r in results if r['problems'])} with problems.", ""]
    if args.matrix:
        text += ["### Stock parts per bike", ""] + parts_matrix(results) + [""]
    for r in results:
        if r["vehicle"] in args.detail:
            text += detail(r)
    report = "\n".join(text)
    if args.md:
        with open(args.md, "w", encoding="utf-8", newline="\n") as f:
            f.write(report)
        return
    print(report)


if __name__ == "__main__":
    main()
