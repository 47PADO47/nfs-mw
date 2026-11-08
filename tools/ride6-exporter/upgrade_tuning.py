"""The performance upgrades of a bike: what the `TuningComponentData_Upgrade_*` tables of its physics asset do at full level.

A bike's physics asset (`PhysicsRide6/.../VehiclesTuning/DataAsset/<Brand>/<name>`, resolved by `physdump.py`) lists
`TuningDataTables`, among them the upgrade tables of its engine class (`TuningComponentData_Upgrade_600cc_4T`, ...). Each row
names a member of the physics data (`Engine.PeakTorque`, `Chassis.Mass`, `Brake.BrakeForce`) and how an upgrade level in
0 to 1 changes it. This module turns the rows and the bike's own base values into the multiplier at full level, which the
catalogue keeps (docs/formats/ride6-bike-parts.md, "Performance upgrades").

Modifier types read (the others, `TCSVehicleElement.*` and `ClutchFeathering`, are switches of launch control and are not numbers):

- `RangedMultiplier`: the multiplier at level u is `lerp(MinMultiplier, MaxMultiplier, u)`.
- `AdaptiveMultiplier`: the multiplier at level u is `lerp(1, top, u)`, with `top` read off `MaxMultiplierRange` by where the bike's
  own base value lies in `ValueInterval` (the bigger the engine, the smaller the gain). **[inference]**: the rows carry the two
  ranges and the interval and no formula; this is the reading that gives the gains the ranges state at both ends of the interval.
"""

import json
import os

MEMBERS = {
    "VehicleMovement.PhysicsVehicle.VehicleProperties.Engine.PeakTorque": "peak_torque",
    "VehicleMovement.PhysicsVehicle.VehicleProperties.Chassis.Mass": "mass",
    "VehicleMovement.PhysicsVehicle.VehicleProperties.Brake.BrakeForce": "brake_force",
}
UPGRADE_PREFIX = "TuningComponentData_Upgrade"


def lerp(a, b, t):
    return a + (b - a) * t


def clamp01(t):
    return max(0.0, min(1.0, t))


def table_rows(dump, name):
    """The rows of a `TuningComponentData_*` table of the physics dump, `{}` when the dump has none."""
    path = os.path.join(dump, "resolved", "plugin", "Physics", "TuningComponent", name + ".json")
    if not os.path.exists(path):
        return {}
    with open(path, encoding="utf-8") as f:
        for export in json.load(f):
            if "Rows" in export:
                return export["Rows"]
    return {}


def vehicle_json(dump, physics_asset):
    """The resolved physics file of a bike from the physics asset path the `PHYSICS` table names; None when the dump lacks it."""
    if not physics_asset:
        return None
    parts = physics_asset.replace("\\", "/").split("/")
    name = parts[-1].split(".")[0]
    brand = parts[-2]
    path = os.path.join(dump, "vehicles", brand, name + ".json")
    if not os.path.exists(path):
        return None
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def base_values(properties):
    """The three numbers the upgrade rows act on, from the bike's resolved physics properties."""
    return {
        "peak_torque": (properties.get("Engine") or {}).get("PeakTorque"),
        "mass": (properties.get("CHASSIS") or {}).get("Mass"),
        "brake_force": (properties.get("Brake") or {}).get("BrakeForce"),
    }


def multiplier(row, base):
    """The multiplier of `row` at full level for a bike whose base value is `base`; None for a row that is not a number."""
    kind = row["ModifierType"].split("::")[-1]
    if kind == "RangedMultiplier":
        return lerp(row["MinMultiplier"], row["MaxMultiplier"], 1.0)
    if kind != "AdaptiveMultiplier" or base is None:
        return None
    low, high = row["ValueInterval"]["X"], row["ValueInterval"]["Y"]
    t = clamp01((base - low) / (high - low)) if high > low else 0.0
    top = row["MaxMultiplierRange"]
    return lerp(top["X"], top["Y"], t)


def ladder(dump, properties):
    """`{tables, base, full}` of one bike: the upgrade tables it lists, its base values and the multiplier of each member at full level."""
    names = [
        os.path.basename(item["ObjectPath"].replace("\\", "/")).split(".")[0]
        for item in properties.get("TuningDataTables") or []
        if UPGRADE_PREFIX in item.get("ObjectPath", "")
    ]
    base = base_values(properties)
    full = {}
    for name in names:
        for row in table_rows(dump, name).values():
            member = MEMBERS.get(row["MemberPath"])
            if member is None:
                continue
            value = multiplier(row, base[member])
            if value is not None:
                full[member] = round(value, 4)
    return {"tables": names, "base": base, "full": full}
