"""The customisation catalogue of the RIDE 6 bikes: which optional parts and upgrades each bike has.

    python -I upgrades.py --out C:\\r6work\\bike-parts --physics C:\\r6work\\physdump-all [--only <regex>]

Reads the bindings tables (`graphics/Bindings/Bike/*`, docs/formats/ride6-bike-parts.md) and the physics dump and writes
`<out>/catalogue.json` (and `<out>/summary.md`): for each bike the aftermarket exhausts (header pipes and silencer), brake kit
levels, rims and bar pads its rows offer, and the upgrade ladder its physics asset gives (`upgrade_tuning.py`); and the data
assets those rows name, once each. It exports no mesh: `option_export.py` does, for the assets a bike can use.
"""

import argparse
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import r6site  # noqa: F401,E402

import upgrade_tuning  # noqa: E402
from bindings import Tables  # noqa: E402
from partsel import Source  # noqa: E402
from r6lib import Assets, Tool, object_name, ref_path  # noqa: E402

SCHEMA = 1
LEVEL = re.compile(r"_Brakes-Kit-Level-(\d+)$")


def asset_entry(assets, kind, path, meshes, missing, who):
    """Record the data asset at `path` once; the `meshes` are `{side: property name}`. Returns its stem, or None when unusable."""
    stem = object_name(path)
    export = assets.named(path)
    if export is None:
        missing.append({"asset": stem, "reason": "asset not in install", "who": who})
        return None
    props = export.get("Properties") or {}
    found = {}
    for side, prop in meshes.items():
        mesh = ref_path(props.get(prop))
        if mesh and assets.exports(mesh) is not None:
            found[side] = mesh
    if len(found) != len(meshes):
        missing.append({"asset": stem, "reason": "no mesh", "who": who})
        return None
    material = ref_path(props.get("Material"))
    return stem, {"kind": kind, "meshes": found, "material": material}


class Builder:
    def __init__(self, tables, assets, dump):
        self.tables = tables
        self.assets = assets
        self.dump = dump
        self.assets_out = {}
        self.missing = []

    def use(self, kind, path, meshes, who):
        if not path:
            return None
        found = asset_entry(self.assets, kind, path, meshes, self.missing, who)
        if found is None:
            return None
        stem, entry = found
        self.assets_out.setdefault(stem, entry)
        return stem

    def rows(self, table, prefix):
        return sorted(k for k in self.tables.rows[table] if k.startswith(prefix))

    def exhaust(self, vehicle):
        out = []
        for key in self.rows("EXMANIFOLD", f"CH_{vehicle}$MUF_"):
            muffler = key.split("$", 1)[1]
            if muffler.startswith("MUF_Stock_"):
                continue
            header = self.use("header", self.tables.asset("EXMANIFOLD", key) or "", {"body": "Mesh"}, key)
            silencer = self.use("muffler", self.tables.asset("MUFFLER", muffler) or "", {"body": "Mesh"}, muffler)
            if header and silencer:
                out.append({"id": muffler, "header": header, "muffler": silencer})
        return out

    def brakes(self, vehicle):
        out = []
        for key in self.rows("BRAKES_KIT", f"BRK_{vehicle}_Brakes-Kit-Level-"):
            level = int(LEVEL.search(key)[1])
            meshes = {"front": "FrontBrakeMesh", "back": "BackBrakeMesh"}
            stem = self.use("brake", self.tables.asset("BRAKES_KIT", key) or "", meshes, key)
            if stem:
                out.append({"level": level, "asset": stem})
        return sorted(out, key=lambda item: item["level"])

    def aftermarket(self, table, kind, stock_prefix, meshes):
        """The rows of `table` that are not stock rows: `[(row key, asset stem)]`, each asset recorded once."""
        out = []
        for key in sorted(self.tables.rows[table]):
            if key.startswith(stock_prefix) or "No-Brand" in key:
                continue
            asset = self.tables.asset(table, key)
            stem = asset and self.use(kind, asset, meshes, key)
            if stem and stem not in [s for _, s in out]:
                out.append((key, stem))
        return out

    def bike(self, vehicle, rims, pads):
        sibling = [vehicle] + Source(vehicle, self.tables, self.assets).siblings()
        own = next((v for v in sibling if self.rows("EXMANIFOLD", f"CH_{v}$MUF_")), vehicle)
        entry = {"exhaust": self.exhaust(own), "brakes": self.brakes(vehicle), "rims": [], "barpad": []}
        entry["rims"] = [{"id": key, "asset": stem} for key, stem in rims]
        has_pad = any(k.startswith(f"PAD_Stock_{vehicle}") for k in self.tables.rows["BARPADS"])
        entry["barpad"] = [{"id": key, "asset": stem} for key, stem in pads] if has_pad else []
        chassis_row = self.tables.vehicles()[vehicle][min(self.tables.vehicles()[vehicle])]
        physics = self.tables.asset("PHYSICS", chassis_row)
        found = upgrade_tuning.vehicle_json(self.dump, physics)
        entry["upgrade"] = upgrade_tuning.ladder(self.dump, found["Properties"]) if found else None
        return entry


def summary(catalogue):
    bikes = catalogue["bikes"]
    count = lambda key: sum(len(b[key]) for b in bikes.values())
    kinds = {}
    for entry in catalogue["assets"].values():
        kinds[entry["kind"]] = kinds.get(entry["kind"], 0) + 1
    lines = [
        "# RIDE 6 bike customisation catalogue",
        "",
        f"{len(bikes)} bikes, {len(catalogue['assets'])} distinct data assets: " + ", ".join(f"{n} {k}" for k, n in sorted(kinds.items())),
        "",
        "| Option | Bikes with some | Offers in all |",
        "|---|---|---|",
    ]
    for key in ("exhaust", "brakes", "rims", "barpad"):
        lines.append(f"| {key} | {sum(1 for b in bikes.values() if b[key])} | {count(key)} |")
    ladders = [b["upgrade"] for b in bikes.values() if b["upgrade"] and b["upgrade"]["full"]]
    lines += ["", f"Upgrade ladders: {len(ladders)} bikes with at least one number, {len(bikes) - len(ladders)} without.", ""]
    lines.append(f"Assets the install lacks or without a mesh: {len(catalogue['missing'])}")
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", default=r"C:\r6work\bike-parts")
    parser.add_argument("--physics", default=r"C:\r6work\physdump-all")
    parser.add_argument("--only", default="")
    args = parser.parse_args()
    tool = Tool()
    try:
        assets = Assets(tool)
        tables = Tables(assets)
        # Every data asset the option tables name is dumped in one request each, before the rows are read one by one.
        for table in ("EXMANIFOLD", "MUFFLER", "BRAKES_KIT", "RIMS", "BARPADS", "PHYSICS"):
            assets.ensure([row["DataAsset"]["AssetPathName"] for row in tables.rows[table].values() if row.get("DataAsset")])
        builder = Builder(tables, assets, args.physics)
        rims = builder.aftermarket("RIMS", "rim", "RI_Stock_", {"front": "FrontRimMesh", "back": "BackRimMesh"})
        pads = builder.aftermarket("BARPADS", "barpad", "PAD_Stock_", {"body": "Mesh"})
        vehicles = sorted(v for v in tables.vehicles() if re.search(args.only, v))
        bikes = {v: builder.bike(v, rims, pads) for v in vehicles}
    finally:
        tool.close()
    catalogue = {"schema": SCHEMA, "bikes": bikes, "assets": builder.assets_out, "missing": builder.missing}
    os.makedirs(args.out, exist_ok=True)
    with open(os.path.join(args.out, "catalogue.json"), "w", encoding="utf-8") as f:
        json.dump(catalogue, f, indent=1, sort_keys=True)
    with open(os.path.join(args.out, "summary.md"), "w", encoding="utf-8") as f:
        f.write(summary(catalogue))
    print(summary(catalogue))


if __name__ == "__main__":
    main()
