"""Finding a track's lap in its spline levels: the track spline, the racing line and the starting grid (docs/formats/ride6-track-pack.md).

A track keeps its splines in `SplinesA` (the full circuit), `Splines` and `SplinesB` (shorter routes). The actor of the track spline is
called `TrackSpline` on most tracks, but also `TrackSpline_2`, `Trackspline` or `IgnitionTrackSpline_0` (Imola, Gunma, Vallelunga,
Cappadocia Trails...), often several in one level (lanes or routes), so the actor's name only selects candidates: the one that lies on
the track's drivable ground wins (`pick`).
"""

import re

import level
import numpy as np

LAYOUTS = ("SplinesA", "Splines", "SplinesB")
TRACK_NAME = re.compile(r"^(ignition)?track\s*spline", re.IGNORECASE)
IDEAL_NAME = re.compile(r"^ideal\s*trajectory", re.IGNORECASE)
# The marker groups of a starting grid, best first: the full field of a race, then the head to head start.
GRID_GROUPS = ("StartingGrid", "HeadToHeadStartingGrid", "EnduranceStartingGrid")
# A spline shorter than this is a straight or a pit lane, not a lap (metres).
MIN_LAP_M = 800.0
# Samples of a candidate this close to the drivable ground count as on it (metres); the grid cell of the lookup.
NEAR_M = 10.0


def candidates(assets, track, pattern):
    """The closed splines of the track's spline levels whose actor name matches `pattern`: `[{layout, actor, component}]`.

    A spline that is not a closed loop or is shorter than [`MIN_LAP_M`] is a pit lane or a start straight and is left out; the
    exact name `TrackSpline` is kept even when the level does not say that it is closed (it is the track).
    """
    found = []
    for layout in LAYOUTS:
        exports = assets.exports(level.level_paths(track)[layout])
        for export in exports or []:
            if export.get("Type") != "IgnitionSplineComponent":
                continue
            actor = level.actor_of(export)
            if not pattern.search(actor):
                continue
            props = export.get("Properties") or {}
            exact = actor == "TrackSpline"
            if not (props.get("bClosedLoop") or exact) or props.get("SplineLength", 0.0) / 100.0 < MIN_LAP_M:
                continue
            found.append({"layout": layout, "actor": actor, "component": export, "exports": exports})
    return found


class Drivable:
    """A set of the ground's triangles as a grid of occupied squares: how far a point is from drivable ground."""

    def __init__(self, groups, materials, surfaces):
        centres = []
        for material, collide, positions, _normals, _uv, indices, _model in groups:
            if not collide or materials[material]["surface"] not in surfaces:
                continue
            triangles = indices.reshape(-1, 3)
            centres.append(positions[triangles][:, :, :2].mean(axis=1))
        self.cells = set()
        if not centres:
            return
        cells = np.floor(np.concatenate(centres) / NEAR_M).astype(np.int64)
        self.cells = {(int(x), int(y)) for x, y in np.unique(cells, axis=0)}

    def share(self, points):
        """The fraction of `points` (x, y, z) with drivable ground in their square or a neighbouring one."""
        if not self.cells or not points:
            return 0.0
        hits = 0
        for x, y, _z in points:
            cx, cy = int(x // NEAR_M), int(y // NEAR_M)
            hits += any((cx + dx, cy + dy) in self.cells for dx in (-1, 0, 1) for dy in (-1, 0, 1))
        return hits / len(points)


def pick(options, ground):
    """The best candidate of `options` (`{layout, ..., points}`) on the ground: the one with the largest share on asphalt and
    concrete, else on any ground a wheel can touch that is not grass or rock, else the longest. `ground` is `(groups, materials)`."""
    if not options:
        return None
    groups, materials = ground
    for surfaces in (("asphalt", "concrete"), ("asphalt", "concrete", "dirt", "sand", "gravel")):
        drivable = Drivable(groups, materials, surfaces)
        scored = [(drivable.share(o["points"][::4]), o) for o in options]
        best = max(scored, key=lambda s: (s[0], -LAYOUTS.index(s[1]["layout"]), len(s[1]["points"])))
        if best[0] >= 0.6:
            return best[1]
    return max(options, key=lambda o: o["length_m"])


def sampled(options):
    """Each candidate with its polyline (`points`) and `length_m`, ready for `pick`."""
    out = []
    for option in options:
        line = level.sample_spline(option["component"], 2.0)
        out.append({**option, "points": line["points"], "length_m": line["length_m"], "line": line})
    return out


def grid_of(exports):
    """The starting grid of one level: the first marker group that has points (`StartingGrid`, then the head to head grid)."""
    for group in GRID_GROUPS:
        points = level.target_points(exports, group)
        if points:
            return points
    return []


def find(assets, track, ground):
    """The lap of a track: `{spline, line, layout, grid}` (any may be missing), or `{}` when it has no track spline."""
    options = sampled(candidates(assets, track, TRACK_NAME))
    chosen = pick(options, ground)
    if chosen is None:
        return {}
    out = {"spline": chosen["line"], "layout": chosen["layout"]}
    ideals = [o for o in sampled(candidates(assets, track, IDEAL_NAME)) if o["layout"] == chosen["layout"]]
    line = pick(ideals, ground)
    if line is not None:
        out["line"] = line["line"]
    out["grid"] = grid_of(chosen["exports"]) or next(
        (grid for layout in LAYOUTS if (grid := grid_of(assets.exports(level.level_paths(track)[layout]) or []))), []
    )
    return out
