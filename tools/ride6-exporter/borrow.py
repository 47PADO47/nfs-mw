"""The bikes whose chassis data is not in the install, made from another bike's meshes (docs/modding/ride6-bikes-batch.md, "Borrowed visuals").

For each entry of `borrow.json` this writes `<into>/<vehicle id>/` with a `parts.json` that is the lender's with the borrowed bike's own
vehicle id and physics asset, a `status.json` that says where the meshes come from (`borrowed_from`), and `glb/` and `tex/` as links to the
lender's folders (a junction on Windows, a symbolic link elsewhere), so nothing is copied. `ride6-bike-import import-all <into>` then imports
them like any other bike: the bike's own physics, cameras and sound (where the install has them), another bike's looks.

    python -I borrow.py --out C:\\r6work\\bikes [--into <folder>] [--only <regex>] [--physdump C:\\r6work\\physdump-all] [--dry-run]

`--out` is the exporter's output (the lenders are in it); `--into` defaults to the same folder, where `import-all` finds the borrowed bikes
beside the real ones. Nothing of the game is written by this script beyond what the exporter already put in `--out`.
"""

import argparse
import json
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
PHYSICS_ROOT = "/PhysicsRide6/Physics/VehiclesTuning/DataAsset"


def link(target, path):
    """Make `path` a link to the folder `target`: a junction on Windows (no privilege needed), else a symbolic link."""
    if os.path.lexists(path):
        return
    if os.name != "nt":
        os.symlink(target, path, target_is_directory=True)
        return
    # mklink reads a forward slash as an option: the paths go in as Windows paths.
    subprocess.run(["cmd", "/c", "mklink", "/J", os.path.abspath(path), os.path.abspath(target)], check=True, stdout=subprocess.DEVNULL)


def physics_asset(entry, lender_asset, dump):
    """The physics asset the borrowed bike's `parts.json` names: its own where the install has one, else the lender's."""
    own = entry.get("physics")
    if not own:
        return lender_asset, "the lender's (the install has none for this bike)"
    name = own.split("/")[-1]
    if dump and not any(name + ".json" in files for _, _, files in os.walk(os.path.join(dump, "vehicles"))):
        return lender_asset, "the lender's (the physics dump has no %s.json)" % name
    return "%s/%s.%s" % (PHYSICS_ROOT, own, name), "its own"


def borrow(entry, out, into, dump, dry):
    vehicle, lender = entry["vehicle"], entry["lender"]
    source = os.path.join(out, lender)
    if not os.path.isfile(os.path.join(source, "parts.json")):
        return "%s: the lender %s is not exported" % (vehicle, lender)
    with open(os.path.join(source, "parts.json"), encoding="utf8") as handle:
        parts = json.load(handle)
    asset, which = physics_asset(entry, parts.get("physics_asset"), dump)
    note = "visuals of %s, physics asset %s" % (lender, which)
    if dry:
        return "%s: %s" % (vehicle, note)
    folder = os.path.join(into, vehicle)
    os.makedirs(folder, exist_ok=True)
    parts["vehicle"] = vehicle
    parts["physics_asset"] = asset
    parts["chassis_row"] = "CH_%s_Livery-01" % vehicle
    parts.setdefault("notes", []).append("borrowed visuals: the meshes and textures are %s's (borrow.py)" % lender)
    with open(os.path.join(folder, "parts.json"), "w", encoding="utf8") as handle:
        json.dump(parts, handle, indent=1)
    status = {"ok": True, "borrowed_from": lender, "physics": which, "parts": [p["id"] for p in parts["parts"]], "missing": []}
    with open(os.path.join(folder, "status.json"), "w", encoding="utf8") as handle:
        json.dump(status, handle, indent=1)
    for name in ("glb", "tex"):
        link(os.path.join(source, name), os.path.join(folder, name))
    return "%s: %s" % (vehicle, note)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--out", required=True, help="the exporter's output folder (the lenders are in it)")
    parser.add_argument("--into", help="where to write the borrowed bikes (default: --out)")
    parser.add_argument("--only", help="a regex: only the vehicles it matches")
    parser.add_argument("--physdump", help="the physics dump folder: a physics asset it lacks falls back to the lender's")
    parser.add_argument("--dry-run", action="store_true", help="say what would be done")
    args = parser.parse_args()
    with open(os.path.join(HERE, "borrow.json"), encoding="utf8") as handle:
        table = json.load(handle)
    into = args.into or args.out
    pattern = re.compile(args.only) if args.only else None
    failed = 0
    for entry in table["borrowed"]:
        if pattern and not pattern.search(entry["vehicle"]):
            continue
        line = borrow(entry, args.out, into, args.physdump, args.dry_run)
        failed += "lender" in line and "not exported" in line
        print(line)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
