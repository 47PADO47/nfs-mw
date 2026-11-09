#!/usr/bin/env python3
"""Tests of how the exporter finds a track's lap: `python -I tools/ride6-tracks/test_lapfinder.py`. No game data and no install are needed."""

import math
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import common  # noqa: E402,F401  (first: it makes the user site-packages importable)
import lapfinder  # noqa: E402
import numpy as np  # noqa: E402

def spline(actor, closed, length_m, keys=((0, 0, 0), (1000, 0, 0), (1000, 1000, 0), (0, 1000, 0))):
    """An `IgnitionSplineComponent` export of a closed square of `keys` (centimetres)."""
    points = [
        {"OutVal": {"X": x, "Y": y, "Z": z}, "ArriveTangent": {"X": 0, "Y": 0, "Z": 0}, "LeaveTangent": {"X": 0, "Y": 0, "Z": 0}}
        for x, y, z in keys
    ]
    return {
        "Type": "IgnitionSplineComponent",
        "Outer": {"ObjectName": f"Level'x.PersistentLevel.{actor}'"},
        "Properties": {"bClosedLoop": closed, "SplineLength": length_m * 100.0, "SplineCurves": {"position": {"Points": points}}},
    }


class FakeAssets:
    def __init__(self, levels):
        self.levels = levels

    def exports(self, path):
        for name, exports in self.levels.items():
            if path.endswith(f"/{name}.umap"):
                return exports
        return None


class LapTests(unittest.TestCase):
    def test_the_name_of_the_track_spline_varies_and_pit_lanes_and_short_lines_are_left_out(self):
        assets = FakeAssets(
            {
                "SplinesA": [
                    spline("TrackSpline_2", True, 4000),
                    spline("Trackspline", True, 4000),
                    spline("IgnitionTrackSpline_0", True, 4000),
                    spline("IgnitionTrackSpline_3", False, 85),
                    spline("PitLaneSpline", False, 1100),
                    spline("AIGuidanceSpline", True, 4000),
                ]
            }
        )
        found = [o["actor"] for o in lapfinder.candidates(assets, "T", lapfinder.TRACK_NAME)]
        self.assertEqual(found, ["TrackSpline_2", "Trackspline", "IgnitionTrackSpline_0"])

    def test_the_candidate_on_the_asphalt_wins_over_a_longer_one_off_it(self):
        # Asphalt under the square of the first spline (0..10 m); the second spline is 500 m away.
        square = np.array(
            [[0, 0, 0], [10, 0, 0], [10, 10, 0], [0, 10, 0]], dtype=np.float32
        )
        groups = [(0, True, square, None, None, np.array([0, 1, 2, 0, 2, 3], dtype=np.uint32), 0)]
        materials = [{"surface": "asphalt"}]
        near = {"layout": "SplinesA", "points": [(1.0, 1.0, 0.0), (5.0, 5.0, 0.0), (9.0, 9.0, 0.0)], "length_m": 100.0}
        far = {"layout": "SplinesA", "points": [(500.0, 500.0, 0.0), (505.0, 500.0, 0.0)] * 3, "length_m": 9000.0}
        chosen = lapfinder.pick([far, near], (groups, materials))
        self.assertIs(chosen, near)
        self.assertIsNone(lapfinder.pick([], (groups, materials)))

    def test_with_no_drivable_ground_the_longest_candidate_is_taken(self):
        short = {"layout": "SplinesA", "points": [(0.0, 0.0, 0.0)], "length_m": 10.0}
        long = {"layout": "SplinesB", "points": [(0.0, 0.0, 0.0)], "length_m": 99.0}
        self.assertIs(lapfinder.pick([short, long], ([], [])), long)

    def test_a_head_to_head_grid_is_used_when_the_full_grid_is_absent(self):
        def marker(actor, parent, x):
            props_ = {"RelativeLocation": {"X": x, "Y": 0.0, "Z": 0.0}, "RelativeRotation": {"Yaw": 0.0}}
            if parent:
                props_["AttachParent"] = {"ObjectName": f"SceneComponent'x.PersistentLevel.{parent}.RootComponent'"}
            return {"Type": "SceneComponent", "Outer": {"ObjectName": f"Level'x.PersistentLevel.{actor}'"}, "Properties": props_}

        exports = [marker("HeadToHeadStartingGrid", None, 0.0), marker("H2H_Point_01", "HeadToHeadStartingGrid", 100.0)]
        grid = lapfinder.grid_of(exports)
        self.assertEqual(len(grid), 1)
        self.assertAlmostEqual(grid[0][0], 1.0, places=4)


if __name__ == "__main__":
    unittest.main()
