"""The bindings tables of the bikes (`graphics/Bindings/Bike/*`): data tables whose rows point at one data asset each."""

import re

from r6lib import object_name

DIR = "/Game/assets/graphics/Bindings/Bike/"
TABLES = (
    "Chassis",
    "PHYSICS",
    "TYRE_SIZES",
    "FRONT_TYRES",
    "BACK_TYRES",
    "FRONT_TYRE_PHYSICS_COMPOUNDS",
    "RIMS",
    "CarbonRims",
    "BRAKES_KIT",
    "BRAKES_KIT_OVERRIDE",
    "EXMANIFOLD",
    "MUFFLER",
    "BARPADS",
    "HANDGUARDS",
    "SIDE_MIRRORS",
    "SUMP_GUARD",
    "ENDURO_PRESET",
    "HOMOLOGATION",
    "CHAIN",
    "LIGHTS",
)
LIVERY = re.compile(r"^CH_(?P<id>.+)_Livery-(?P<n>\d+)$")


class Tables:
    """All tables of the bikes, with exact and case-insensitive row lookup."""

    def __init__(self, assets):
        self.assets = assets
        assets.ensure([DIR + name for name in TABLES])
        self.rows = {}
        self.lower = {}
        for name in TABLES:
            data = assets.exports(DIR + name)
            rows = {}
            if data:
                rows = data[0].get("Rows") or {}
            self.rows[name] = rows
            self.lower[name] = {key.lower(): key for key in rows}

    def key(self, table, wanted):
        """The row key of `table` equal to `wanted` ignoring case, or None."""
        return self.lower[table].get(wanted.lower())

    def asset(self, table, wanted):
        """The data asset path of row `wanted` of `table`, or None when the table has no such row."""
        key = self.key(table, wanted)
        if key is None:
            return None
        row = self.rows[table][key]
        data = row.get("DataAsset") or {}
        return data.get("AssetPathName") or None

    def value(self, table, wanted):
        """The plain value of a row without a data asset (a tyre size, a flag), or None."""
        key = self.key(table, wanted)
        if key is None:
            return None
        row = self.rows[table][key]
        for item in row.values():
            if isinstance(item, str):
                return item
        return None

    def keys_ending(self, table, suffix):
        return [k for k in self.rows[table] if k.lower().endswith(suffix.lower())]

    def vehicles(self):
        """Every vehicle id with its livery rows `{id: {n: row key}}` from the `Chassis` table."""
        out = {}
        for key in self.rows["Chassis"]:
            m = LIVERY.match(key)
            if not m:
                continue
            out.setdefault(m["id"], {})[int(m["n"])] = key
        return out


def asset_stem(path):
    return object_name(path)
