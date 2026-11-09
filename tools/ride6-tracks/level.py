"""Reading a track's levels (the `.umap` JSON dumps): static meshes with their material overrides, the track spline, the grid.

Layout facts: docs/formats/ride6-tracks.md. A component names its actor in `Outer`; a track spline actor is its own
`MainSpline` component (the actor's transform is the component's `RelativeLocation` and yaw).
"""

import math

from common import ue_to_render, ue_world


def level_paths(track):
    """The package paths of a track's sub-levels the exporter reads."""
    base = f"ride6/Content/maps/Tracks/{track}/Game"
    return {name: f"{base}/{name}.umap" for name in ("Terrain", "SplinesA", "SplinesB", "Splines")}


def actor_of(export):
    """The actor name a component belongs to (`...PersistentLevel.TrackSpline` gives `TrackSpline`)."""
    outer = export.get("Outer")
    if not isinstance(outer, dict):
        return ""
    return outer.get("ObjectName", "").rstrip("'").rsplit(".", 1)[-1]


def object_path(ref):
    """The object path of a reference property, or None."""
    if not isinstance(ref, dict):
        return None
    return ref.get("ObjectPath")


def static_meshes(exports):
    """The static mesh components of a level: `{actor, mesh, location, overrides}` (overrides: material paths by slot)."""
    out = []
    for export in exports:
        if export.get("Type") not in ("StaticMeshComponent", "IgnitionStaticMeshComponent"):
            continue
        props = export.get("Properties") or {}
        mesh = object_path(props.get("StaticMesh"))
        if not mesh:
            continue
        out.append(
            {
                "actor": actor_of(export),
                "mesh": mesh,
                "location": props.get("RelativeLocation"),
                "rotation": props.get("RelativeRotation"),
                "scale": props.get("RelativeScale3D"),
                "overrides": [object_path(m) for m in props.get("OverrideMaterials") or []],
            }
        )
    return out


def hermite(p0, m0, p1, m1, t):
    """A point of a cubic Hermite segment (Unreal's spline curves with the key spacing of 1)."""
    t2, t3 = t * t, t * t * t
    h00, h10, h01, h11 = 2 * t3 - 3 * t2 + 1, t3 - 2 * t2 + t, -2 * t3 + 3 * t2, t3 - t2
    return tuple(h00 * a + h10 * b + h01 * c + h11 * d for a, b, c, d in zip(p0, m0, p1, m1))


def vec(v):
    return (v["X"], v["Y"], v["Z"])


def spline_component(exports, actor):
    """The `MainSpline` component of the actor called `actor`, or None."""
    for export in exports:
        if export.get("Type") == "IgnitionSplineComponent" and actor_of(export) == actor:
            return export
    return None


def sample_spline(component, step_m=2.0):
    """The spline of a component as a polyline in render space: `{closed, length_m, points: [(x, y, z)]}`.

    Each segment is sampled about every `step_m` metres. A closed spline has one more segment, from its last key to the first.
    """
    props = component["Properties"]
    keys = props["SplineCurves"]["position"]["Points"]
    location = props.get("RelativeLocation") or {"X": 0.0, "Y": 0.0, "Z": 0.0}
    yaw = (props.get("RelativeRotation") or {}).get("Yaw", 0.0)
    closed = bool(props.get("bClosedLoop"))
    count = len(keys) + (1 if closed else 0)
    points = []
    for i in range(count - 1):
        a, b = keys[i], keys[(i + 1) % len(keys)]
        p0, p1 = vec(a["OutVal"]), vec(b["OutVal"])
        m0, m1 = vec(a["LeaveTangent"]), vec(b["ArriveTangent"])
        chord = math.dist(p0, p1) / 100.0
        steps = max(1, int(math.ceil(chord / step_m)))
        for s in range(steps):
            local = hermite(p0, m0, p1, m1, s / steps)
            points.append(ue_to_render(*ue_world(location, yaw, local)))
    if not closed:
        points.append(ue_to_render(*ue_world(location, yaw, vec(keys[-1]["OutVal"]))))
    return {"closed": closed, "length_m": props.get("SplineLength", 0.0) / 100.0, "points": points}


def scene_components(exports):
    """The scene components by actor name: `{actor: properties}` (the transform of markers and target points)."""
    out = {}
    for export in exports:
        if export.get("Type") == "SceneComponent":
            out[actor_of(export)] = export.get("Properties") or {}
    return out


def target_points(exports, group):
    """The target points of a marker group (`StartingGrid`): `[(x, y, z, yaw_degrees)]` in render space, in name order.

    The group's root component has the location and yaw; a point's location is relative to it (turned by the group's yaw)
    and its own yaw is absolute.
    """
    parts = scene_components(exports)
    root = parts.get(group)
    if root is None:
        return []
    rloc = root.get("RelativeLocation") or {"X": 0.0, "Y": 0.0, "Z": 0.0}
    ryaw = (root.get("RelativeRotation") or {}).get("Yaw", 0.0)
    wanted = f"PersistentLevel.{group}.RootComponent"
    points = []
    for actor, props in sorted(parts.items()):
        parent = (props.get("AttachParent") or {}).get("ObjectName", "")
        if not parent.rstrip("'").endswith(wanted):
            continue
        rel = props.get("RelativeLocation") or {"X": 0.0, "Y": 0.0, "Z": 0.0}
        world = ue_world(rloc, ryaw, vec(rel))
        yaw = (props.get("RelativeRotation") or {}).get("Yaw", ryaw)
        points.append((*ue_to_render(*world), -yaw))
    return points
