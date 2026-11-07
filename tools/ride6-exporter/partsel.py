"""Which stock parts a bike has: the lookup of its rows in the bindings tables and the data assets they point at.

A part that cannot be found is recorded in `missing` with the reason, never dropped silently:

- `no row`: the table has no row under any of the keys tried;
- `asset not in install`: the row names a data asset the install does not contain (content that was never shipped);
- `no mesh`: the data asset exists and has no mesh (the "no brand, no handguard" entries);
- `mesh not in install`: the data asset names a mesh package the install does not contain.
"""

import re

from r6lib import object_name, ref_kind, ref_path

# RIDE 6 physics tyre family (`TireData/<Family>/`) to the tyre row group that is that bike's stock look.
STOCK_TYRE_GROUP = {
    "supermotard": ("No-Brand_SM", "Bridgestone_Supermoto"),
    "maxienduro": ("No-Brand_Enduro", "Bridgestone_AX41", "Continental_TKC70"),
    "enduro": ("No-Brand_Enduro",),
}
DEFAULT_TYRE_GROUPS = ("No-Brand_Street",)
MESH_KINDS = ("SkeletalMesh", "StaticMesh")


class Source:
    """Where parts come from, with the notes and the missing-part reasons collected on the way."""

    def __init__(self, vehicle, tables, assets):
        self.vehicle = vehicle
        self.tables = tables
        self.assets = assets
        self.notes = []
        self.missing = []
        self.parts = []

    def lose(self, part, reason, detail=""):
        self.missing.append({"part": part, "reason": reason, "detail": detail})

    def siblings(self):
        """Other vehicles built on the same chassis (the event and race variants of one bike), nearest names first."""
        cache = getattr(self.tables, "_by_number", None)
        if cache is None:
            cache = {}
            for vehicle, liveries in self.tables.vehicles().items():
                asset = self.tables.asset("Chassis", liveries[min(liveries)]) or ""
                number = chassis_number(asset)
                if number:
                    cache.setdefault(number, []).append(vehicle)
            self.tables._by_number = cache
        own = chassis_number(self.tables.asset("Chassis", f"CH_{self.vehicle}_Livery-01") or "")
        others = [v for v in cache.get(own, []) if v != self.vehicle]
        return sorted(others, key=lambda v: (-common_prefix(v, self.vehicle), v))

    def data_asset(self, part, table, keys):
        """Properties of the data asset of the first of `keys` that has a row; None (and a reason) when there is none.

        A bike without a row of its own (an event variant) takes the row of a bike on the same chassis, with a note.
        """
        row = None
        for key in keys:
            asset = self.tables.asset(table, key)
            if asset:
                row = (key, asset)
                break
        if row is None:
            for other in self.siblings():
                for key in keys:
                    alt = key.replace(self.vehicle, other)
                    asset = alt != key and self.tables.asset(table, alt)
                    if asset:
                        row = (alt, asset)
                        self.notes.append(f"{part}: no row of its own, using the row of {other} (same chassis)")
                        break
                if row is not None:
                    break
        if row is None:
            self.lose(part, "no row", f"{table}: " + " | ".join(keys))
            return None
        key, asset = row
        export = self.assets.named(asset)
        if export is None:
            self.lose(part, "asset not in install", f"{table} {self.tables.key(table, key)} -> {object_name(asset)}")
            return None
        return key, asset, export.get("Properties") or {}

    def add(self, part_id, role, mesh_ref, material_ref, origin):
        """A part with a mesh property of a data asset; returns False (with a reason) when the mesh is not usable."""
        mesh = ref_path(mesh_ref)
        if not mesh or ref_kind(mesh_ref) not in MESH_KINDS:
            self.lose(part_id, "no mesh", origin)
            return False
        if self.assets.exports(mesh) is None:
            self.lose(part_id, "mesh not in install", f"{origin} -> {object_name(mesh)}")
            return False
        self.parts.append(
            {
                "id": part_id,
                "role": role,
                "mesh": mesh,
                "material": ref_path(material_ref) if material_ref else None,
                "origin": origin,
            }
        )
        return True


def common_prefix(a, b):
    n = 0
    for x, y in zip(a, b):
        if x != y:
            break
        n += 1
    return n


def chassis_number(mesh_path):
    m = re.search(r"chassis(\d+)", mesh_path, re.IGNORECASE)
    return m.group(1) if m else None


def tyre_rows(src, prefix, size, family):
    """The row keys of the tyre rows of one size in preference order: the bike's stock look first."""
    table = "FRONT_TYRES" if prefix == "FTI" else "BACK_TYRES"
    groups = STOCK_TYRE_GROUP.get(family, DEFAULT_TYRE_GROUPS)
    keys = [f"{prefix}_{g}_{size}" for g in groups + DEFAULT_TYRE_GROUPS]
    found = [k for k in keys if src.tables.key(table, k)]
    if found:
        return found
    rest = src.tables.keys_ending(table, "_" + size)
    return sorted(rest)


def physics_family(src, base_key):
    """The physics tyre family of the bike (`Supermotard`, `MaxiEnduro`, ...) from its `FTP_` row, lower case."""
    asset = src.tables.asset("FRONT_TYRE_PHYSICS_COMPOUNDS", f"FTP_{src.vehicle}$nocompound")
    if not asset:
        return ""
    parts = asset.split("/")
    return parts[-2].lower() if len(parts) > 2 else ""


def pick_tyres(src, number):
    """Front and back tyres from the sizes of the bike (`TYRE_SIZES`)."""
    base = f"CH_{src.vehicle}_Livery-01"
    family = physics_family(src, base)
    for side, table, prefix, role in (("Front", "FRONT_TYRES", "FTI", "front_wheel"), ("Back", "BACK_TYRES", "BTI", "back_wheel")):
        size = src.tables.value("TYRE_SIZES", f"{base}${side}TyreSize")
        if not size:
            src.lose(f"{side.lower()}_tyre", "no row", f"TYRE_SIZES {base}${side}TyreSize")
            continue
        keys = tyre_rows(src, prefix, size, family)
        found = src.data_asset(f"{side.lower()}_tyre", table, keys[:1] or [f"{prefix}_?_{size}"])
        if found is None:
            continue
        key, asset, props = found
        src.notes.append(f"{side.lower()} tyre {size}: row {src.tables.key(table, key)} (physics family {family or '?'})")
        src.add(f"{side.lower()}_tyre", role, props.get("Mesh"), None, f"{table} {key}")


def pick_rims(src, number):
    keys = [f"RI_Stock_{src.vehicle}_Livery-01"] + [k for k in src.tables.rows["RIMS"] if k.startswith(f"RI_Stock_{src.vehicle}_")]
    found = src.data_asset("rims", "RIMS", keys)
    if found is None:
        return
    key, asset, props = found
    material = props.get("Material")
    src.add("front_rim", "front_wheel", props.get("FrontRimMesh"), material, f"RIMS {key}")
    src.add("back_rim", "back_wheel", props.get("BackRimMesh"), material, f"RIMS {key}")


def pick_brakes(src, number):
    found = src.data_asset("brakes", "BRAKES_KIT", [f"BRK_{src.vehicle}_Brakes-Stock-Kit"])
    if found is None:
        return
    key, asset, props = found
    material = props.get("Material")
    src.add("front_brake", "front_wheel", props.get("FrontBrakeMesh"), material, f"BRAKES_KIT {key}")
    src.add("back_brake", "back_wheel", props.get("BackBrakeMesh"), material, f"BRAKES_KIT {key}")
    if src.tables.key("BRAKES_KIT_OVERRIDE", f"{src.vehicle}_Brakes-Stock-Kit"):
        src.notes.append("BRAKES_KIT_OVERRIDE has a row for this bike; the stock kit is used")


def pick_exhaust(src, number):
    """Header pipes and silencer: the stock rows, else (the event bikes, whose only row is an aftermarket pipe) the first row."""
    manifold_keys = sorted(k for k in src.tables.rows["EXMANIFOLD"] if k.startswith(f"CH_{src.vehicle}$MUF_"))
    stock = [k for k in manifold_keys if "$MUF_Stock_" in k and k.endswith("_Standard")]
    own = [k for k in stock if f"MUF_Stock_{src.vehicle}_" in k]
    keys = own + [k for k in stock if k not in own] + [k for k in manifold_keys if k not in stock]
    found = src.data_asset("exmanifold", "EXMANIFOLD", keys or [f"CH_{src.vehicle}$MUF_Stock_{src.vehicle}_Standard"])
    muffler_keys = [f"MUF_Stock_{src.vehicle}_Standard"]
    if found is not None:
        key, asset, props = found
        if key not in stock:
            src.notes.append(f"exhaust: no stock row, using {key}")
        muffler_keys.insert(0, key.split("$", 1)[1] if "$" in key else key)
        src.add("exmanifold", "body", props.get("Mesh"), props.get("Material"), f"EXMANIFOLD {key}")
    found = src.data_asset("muffler", "MUFFLER", muffler_keys)
    if found is None:
        return
    key, asset, props = found
    src.add("muffler", "body", props.get("Mesh"), props.get("Material"), f"MUFFLER {key}")


def pick_small(src, number):
    """Bar pad, hand guards, mirrors, sump guard, rally kit and plate: parts only some bikes have."""
    for part_id, table, keys in (
        ("barpad", "BARPADS", [f"PAD_Stock_{src.vehicle}_Livery-01"]),
        ("handguards", "HANDGUARDS", [f"HGU_Stock_{src.vehicle}_Livery-01"]),
        ("mirrors", "SIDE_MIRRORS", [f"MI_Stock_{src.vehicle}"]),
        ("sump_guard", "SUMP_GUARD", [src.vehicle]),
        ("enduro_preset", "ENDURO_PRESET", [f"EP_Stock_{src.vehicle}"]),
        ("homologation", "HOMOLOGATION", [f"HML_Stock_{src.vehicle}"]),
    ):
        if not src.tables.key(table, keys[0]):
            continue
        found = src.data_asset(part_id, table, keys)
        if found is None:
            continue
        key, asset, props = found
        src.add(part_id, "body", props.get("Mesh"), props.get("Material"), f"{table} {key}")


def livery_variants(src, liveries):
    """For liveries 2..n, the data asset rows of the parts that change with the livery (`RI_Stock_.._Livery-02`, ...)."""
    out = {}
    for n in sorted(liveries):
        if n == 1:
            continue
        rows = {}
        for part_id, table, key in (
            ("rims", "RIMS", f"RI_Stock_{src.vehicle}_Livery-{n:02d}"),
            ("barpad", "BARPADS", f"PAD_Stock_{src.vehicle}_Livery-{n:02d}"),
            ("handguards", "HANDGUARDS", f"HGU_Stock_{src.vehicle}_Livery-{n:02d}"),
        ):
            asset = src.tables.asset(table, key)
            if asset:
                rows[part_id] = asset
        out[n] = rows
    return out


def select(vehicle, tables, assets, chassis_mesh):
    """The stock parts of a vehicle (without the chassis, which the caller adds from the CHASSIS row)."""
    src = Source(vehicle, tables, assets)
    number = chassis_number(chassis_mesh)
    for step in (pick_tyres, pick_rims, pick_brakes, pick_exhaust, pick_small):
        step(src, number)
    return src
