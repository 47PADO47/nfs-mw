"""The physics dump the bike importer reads (`ride6-bike-import --physics`): RIDE 6's physics assets with their Blueprint
inheritance resolved (docs/formats/ride6-physics-data.md, "Inheritance" and "The dump").

    python -I physdump.py [--out C:\\r6work\\physdump-all]

Output:
    vehicles/<Brand>/<Vehicle>.json        {Name, Properties (resolved), Linked {object path: asset}} per vehicle (319)
    resolved/plugin/Physics/**.json        every package of the plugin, each export with Properties resolved
    resolved/content/assets/**.json        the non-plugin assets the importer reads (rider corrections, camera containers)
    index.json                             what was dumped and any error

Resolving: the chain of an object runs from the root class default object to the object (an instance's `Template`, then the
`SuperStruct` classes' default objects). A struct property merges field by field; every other property of the child replaces
the parent's. A `FRuntimeFloatCurve` keeps both its embedded keys and its external curve (the importer prefers the external
one). Every curve (a dict with `Keys`) is written as `{Points: [[x, y]], InterpModes: [..], Tangents: [[arrive, leave]]}`.
"""

import argparse
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import r6site  # noqa: F401,E402

from r6lib import Assets, Tool, package_key  # noqa: E402

PLUGIN = r"ride6/Plugins/PhysicsRide6/Content/Physics/"
PLUGIN_MOUNT = "/PhysicsRide6/Physics/"
CONTENT_LISTS = (
    r"assets/data/datatables/Anim/.*\.uasset$",
    r"assets/data/cameras/Containers/.*\.uasset$",
)


def is_ref(value):
    return isinstance(value, dict) and "ObjectPath" in value


def merge(base, over):
    """`over` on top of `base`: structs merge field by field, everything else of `over` replaces."""
    out = dict(base)
    for key, value in over.items():
        before = out.get(key)
        if isinstance(value, dict) and isinstance(before, dict) and not is_ref(value) and not is_ref(before):
            out[key] = merge(before, value)
            continue
        out[key] = value
    return out


def curve_form(value):
    """The points form of a curve dict (`Keys`), other keys kept."""
    keys = value["Keys"]
    out = {k: v for k, v in value.items() if k != "Keys"}
    out["Points"] = [[k["Time"], k["Value"]] for k in keys]
    out["InterpModes"] = sorted({k.get("InterpMode", "RCIM_Linear") for k in keys})
    out["Tangents"] = [[k.get("ArriveTangent", 0.0), k.get("LeaveTangent", 0.0)] for k in keys]
    return out


def is_curve(value):
    keys = value.get("Keys")
    return isinstance(keys, list) and all(isinstance(k, dict) and "Time" in k and "Value" in k for k in keys)


def normalise(value):
    if isinstance(value, list):
        return [normalise(v) for v in value]
    if not isinstance(value, dict):
        return value
    if is_curve(value):
        return curve_form({k: normalise(v) for k, v in value.items() if k != "Keys"} | {"Keys": value["Keys"]})
    return {k: normalise(v) for k, v in value.items()}


def refs_in(value, out):
    """Every object path under `/PhysicsRide6/` in a value."""
    if isinstance(value, dict):
        path = value.get("ObjectPath")
        if isinstance(path, str) and path.lower().startswith("/physicsride6/"):
            out.add(path)
        for item in value.values():
            refs_in(item, out)
    if isinstance(value, list):
        for item in value:
            refs_in(item, out)


class Resolver:
    def __init__(self, assets):
        self.assets = assets
        self.cache = {}

    def export_at(self, path):
        """`(package path, exports list, export)` of an object path `<package>.<index or name>`."""
        package, _, tail = path.rpartition(".")
        exports = self.assets.exports(package)
        if not exports:
            return package, None, None
        if tail.isdigit() and int(tail) < len(exports):
            return package, exports, exports[int(tail)]
        for export in exports:
            if export.get("Name") == tail:
                return package, exports, export
        return package, exports, None

    def class_default(self, package, exports, cls):
        name = "Default__" + cls["Name"]
        for export in exports:
            if export.get("Name") == name:
                return export
        return None

    def own_chain(self, package, exports, export):
        """`(parent resolved properties or {}, chain names)` of an export."""
        template = export.get("Template")
        if is_ref(template):
            return self.resolve_path(template["ObjectPath"])
        if export.get("Name", "").startswith("Default__"):
            cls_name = export["Name"][len("Default__") :]
            for item in exports:
                if item.get("Name") == cls_name and item.get("Type") == "BlueprintGeneratedClass":
                    parent = (item.get("SuperStruct") or {}).get("ObjectPath", "")
                    if parent and not parent.startswith("/Script/"):
                        return self.resolve_path(parent)
        return {}, []

    def resolve_path(self, path):
        """`(resolved properties, chain)` of the object at `path`; a Blueprint class resolves as its default object."""
        key = package_key(path) + "|" + path.rpartition(".")[2]
        if key in self.cache:
            return self.cache[key]
        package, exports, export = self.export_at(path)
        if export is None:
            self.cache[key] = ({}, [])
            return self.cache[key]
        if export.get("Type") == "BlueprintGeneratedClass":
            default = self.class_default(package, exports, export)
            if default is not None:
                export = default
        parent_props, parent_chain = self.own_chain(package, exports, export)
        own = export.get("Properties") or {}
        result = (merge(parent_props, own), parent_chain + [package + "." + export.get("Name", "")])
        self.cache[key] = result
        return result

    def entry(self, path):
        """The dump entry of the asset at `path`: `{Name, Type, Properties, OwnProperties, Chain, Rows?}`."""
        package, exports, export = self.export_at(path)
        if export is None:
            return None
        if export.get("Type") == "BlueprintGeneratedClass":
            default = self.class_default(package, exports, export)
            if default is not None:
                export = default
        props, chain = self.resolve_path(path)
        out = {
            "Name": export.get("Name"),
            "Type": export.get("Type"),
            "Properties": normalise(props),
            "OwnProperties": normalise(export.get("Properties") or {}),
            "Chain": chain,
        }
        if "Rows" in export:
            out["Rows"] = normalise(export["Rows"])
        return out


def vehicle_file(resolver, asset_path, name):
    """The self-contained vehicle: resolved properties and every linked physics asset."""
    main = resolver.entry(asset_path)
    if main is None:
        return None
    linked = {}
    pending = set()
    refs_in(main["Properties"], pending)
    while pending:
        path = pending.pop()
        if path in linked:
            continue
        item = resolver.entry(path)
        linked[path] = item if item is not None else {"Properties": {}, "Missing": True}
        if item is not None:
            refs_in(item["Properties"], pending)
            refs_in(item.get("Rows", {}), pending)
    return {"Name": name, "Properties": main["Properties"], "Chain": main["Chain"], "Linked": linked}


def write_json(path, value):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        json.dump(value, f, indent=1)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", default=r"C:\r6work\physdump-all")
    args = parser.parse_args()
    out = args.out
    tool = Tool()
    errors = []
    try:
        assets = Assets(tool)
        listing = os.path.join(assets.root, "_phys_list.txt")
        tool.request("list", re.escape(PLUGIN) + r".*\.uasset$", listing)
        with open(listing, encoding="utf-8") as f:
            plugin_files = [line.strip() for line in f if line.strip()]
        content_files = []
        for pattern in CONTENT_LISTS:
            tool.request("list", pattern, listing)
            with open(listing, encoding="utf-8") as f:
                content_files += [line.strip() for line in f if line.strip()]
        os.remove(listing)
        assets.ensure(plugin_files + content_files)
        resolver = Resolver(assets)
        dumped = 0
        for file in plugin_files:
            rel = file[len(PLUGIN) :].rsplit(".", 1)[0]
            exports = assets.exports(file)
            if not exports:
                errors.append(f"missing {file}")
                continue
            entries = []
            for index, export in enumerate(exports):
                path = f"{PLUGIN_MOUNT}{rel}.{index}"
                try:
                    props, chain = resolver.resolve_path(path)
                except Exception as error:  # a package that does not resolve is listed, the others go on
                    errors.append(f"{path}: {error}")
                    props, chain = export.get("Properties") or {}, []
                item = {k: v for k, v in export.items() if k not in ("Properties", "Rows")}
                item["Properties"] = normalise(props)
                item["OwnProperties"] = normalise(export.get("Properties") or {})
                item["Chain"] = chain
                if "Rows" in export:
                    item["Rows"] = normalise(export["Rows"])
                entries.append(item)
            write_json(os.path.join(out, "resolved", "plugin", "Physics", *rel.split("/")) + ".json", entries)
            dumped += 1
        for file in content_files:
            rel = file[len("ride6/Content/") :].rsplit(".", 1)[0]
            exports = assets.exports(file)
            if exports:
                write_json(os.path.join(out, "resolved", "content", *rel.split("/")) + ".json", exports)
        vehicles = [f for f in plugin_files if "/VehiclesTuning/DataAsset/" in f and not f.endswith("/DefaultBike.uasset")]
        count = 0
        for file in vehicles:
            rel = file[len(PLUGIN) :].rsplit(".", 1)[0]
            brand, name = rel.split("/")[-2], rel.split("/")[-1]
            data = vehicle_file(resolver, f"{PLUGIN_MOUNT}{rel}.0", name)
            if data is None:
                errors.append(f"vehicle {file} did not resolve")
                continue
            write_json(os.path.join(out, "vehicles", brand, name + ".json"), data)
            count += 1
        write_json(os.path.join(out, "index.json"), {"packages": dumped, "vehicles": count, "errors": errors})
        print(f"{dumped} packages, {count} vehicles, {len(errors)} errors")
    finally:
        tool.close()
    for line in errors[:20]:
        print(line)


if __name__ == "__main__":
    main()
