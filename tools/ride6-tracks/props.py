"""The objects placed on a track: barriers along the splines, buildings, props (docs/formats/ride6-track-pack.md).

A level holds them as static mesh actors (a location, a rotator and a scale) and as instanced mesh components (a list of
transforms: a translation, a quaternion and a scale each, in world space: checked by the distance of the first instances
to the track spline). The pack keeps each distinct mesh once as a model and every placement as a transform in the render space.
"""

import math
import os
import struct

import common  # noqa: F401  (first: it makes the user site-packages importable)
import level
import numpy as np

# The levels that hold objects; the first three are solid (collision), the props are not.
SOLID_LEVELS = ("Props_Barriers", "Pitlane_barriers", "Buildings")
LEVELS = SOLID_LEVELS + ("Props",)
INSTANCE_TYPES = ("InstancedStaticMeshComponent", "HierarchicalInstancedStaticMeshComponent")
MAGIC = b"R6TI"
VERSION = 1


def level_path(track, name):
    return f"ride6/Content/maps/Tracks/{track}/Game/{name}.umap"


def axes_of_rotator(pitch, yaw, roll):
    """The world directions of the local x, y and z axes of an Unreal rotator (degrees), in Unreal's space."""
    sp, cp = math.sin(math.radians(pitch)), math.cos(math.radians(pitch))
    sy, cy = math.sin(math.radians(yaw)), math.cos(math.radians(yaw))
    sr, cr = math.sin(math.radians(roll)), math.cos(math.radians(roll))
    return (
        (cp * cy, cp * sy, sp),
        (sr * sp * cy - cr * sy, sr * sp * sy + cr * cy, -sr * cp),
        (-(cr * sp * cy + sr * sy), cy * sr - cr * sp * sy, cr * cp),
    )


def axes_of_quaternion(x, y, z, w):
    """The same for a quaternion."""
    return (
        (1 - 2 * (y * y + z * z), 2 * (x * y + w * z), 2 * (x * z - w * y)),
        (2 * (x * y - w * z), 1 - 2 * (x * x + z * z), 2 * (y * z + w * x)),
        (2 * (x * z + w * y), 2 * (y * z - w * x), 1 - 2 * (x * x + y * y)),
    )


def render_rows(axes, scale):
    """The rows of a placement in the render space (`x, -y, z` of Unreal): row i is where local axis i points, scaled.

    The model's own coordinates are converted the same way, which turns its local y axis round: the second row is the
    negated Unreal axis.
    """
    flip = ((1, -1, 1), (-1, 1, -1), (1, -1, 1))  # sign of each component of each row: x, -y, z; y row negated
    rows = []
    for i in range(3):
        rows.append([axes[i][k] * flip[i][k] * scale[i] for k in range(3)])
    return rows


def vec3(v, default=1.0):
    if not v:
        return (default, default, default)
    return (v.get("X", default), v.get("Y", default), v.get("Z", default))


def placement_of(item):
    """The transform of a static mesh actor as `(position, rows)` in the render space, or `None` when it is the identity.

    A piece of ground is usually at the origin with its vertices in world space, but a track may move a whole level
    (Nurburgring GP: 70 m) or place scenery meshes (rocks) with a location, a rotator and a scale.
    """
    loc = vec3(item["location"], 0.0)
    rot = item["rotation"] or {}
    scale = vec3(item["scale"])
    angles = (rot.get("Pitch", 0.0), rot.get("Yaw", 0.0), rot.get("Roll", 0.0))
    if loc == (0.0, 0.0, 0.0) and angles == (0.0, 0.0, 0.0) and scale == (1.0, 1.0, 1.0):
        return None
    return common.ue_to_render(*loc), render_rows(axes_of_rotator(*angles), scale)


def collect(assets, track, log=print):
    """Every placed object of the track's object levels: `(instances, models)`.

    `instances` are `{model, pos, rows}` (render space, metres); `models` maps a mesh path to `{index, name, solid, overrides}`.
    """
    models = {}
    instances = []

    def model_of(mesh, solid, overrides):
        if mesh not in models:
            models[mesh] = {
                "index": len(models),
                "name": common.r6lib.object_name(mesh),
                "solid": solid,
                "overrides": overrides,
            }
        return models[mesh]["index"]

    for name in LEVELS:
        exports = assets.exports(level_path(track, name))
        if not exports:
            continue
        solid = name in SOLID_LEVELS
        before = len(instances)
        for item in level.static_meshes(exports):
            rot = item["rotation"] or {}
            axes = axes_of_rotator(rot.get("Pitch", 0.0), rot.get("Yaw", 0.0), rot.get("Roll", 0.0))
            loc = vec3(item["location"], 0.0)
            index = model_of(item["mesh"], solid, item["overrides"])
            instances.append(
                {
                    "model": index,
                    "pos": common.ue_to_render(*loc),
                    "rows": render_rows(axes, vec3(item["scale"])),
                }
            )
        for export in exports:
            if export.get("Type") not in INSTANCE_TYPES:
                continue
            props = export.get("Properties") or {}
            mesh = level.object_path(props.get("StaticMesh"))
            data = export.get("PerInstanceSMData") or props.get("PerInstanceSMData") or []
            if not mesh or not data:
                continue
            index = model_of(mesh, solid, [])
            for entry in data:
                t = entry["TransformData"]
                q = t["Rotation"]
                axes = axes_of_quaternion(q["X"], q["Y"], q["Z"], q["W"])
                instances.append(
                    {
                        "model": index,
                        "pos": common.ue_to_render(*vec3(t["Translation"], 0.0)),
                        "rows": render_rows(axes, vec3(t.get("Scale3D"))),
                    }
                )
        log(f"{name}: {len(instances) - before} objects")
    return instances, models


def write_instances(path, instances):
    """`instances.r6i`: magic, version, count, then per instance a model (u32), a position (3 x f32) and the rows (9 x f32)."""
    with open(path, "wb") as f:
        f.write(MAGIC + struct.pack("<II", VERSION, len(instances)))
        for inst in instances:
            f.write(struct.pack("<I3f9f", inst["model"], *inst["pos"], *[v for row in inst["rows"] for v in row]))


def median_distance(instances, points):
    """The mean distance of the first instances to the spline (a check that the transforms are in world space)."""
    if not instances or not points:
        return None
    pts = np.array([[p[0], p[1]] for p in points[::3]])
    sample = instances[:: max(1, len(instances) // 200)]
    return float(np.mean([np.min(np.hypot(pts[:, 0] - i["pos"][0], pts[:, 1] - i["pos"][1])) for i in sample]))
