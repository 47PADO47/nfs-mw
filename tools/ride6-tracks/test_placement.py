#!/usr/bin/env python3
"""Tests of how the exporter moves a ground piece by its actor's transform: `python -I tools/ride6-tracks/test_placement.py`. No game data and no install are needed."""

import math
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import common  # noqa: E402,F401  (first: it makes the user site-packages importable)
import meshes  # noqa: E402
import numpy as np  # noqa: E402
import props  # noqa: E402

class PlacementTests(unittest.TestCase):
    def test_an_actor_at_the_origin_is_not_moved(self):
        item = {"location": None, "rotation": None, "scale": None}
        self.assertIsNone(props.placement_of(item))

    def test_a_level_moved_by_a_location_moves_every_vertex_by_it(self):
        item = {"location": {"X": 7000.0, "Y": -100.0, "Z": 850.0}, "rotation": None, "scale": None}
        position, rows = props.placement_of(item)
        self.assertEqual(position, (70.0, 1.0, 8.5))  # render space: (x, -y, z) / 100
        points = np.array([[0.0, 0.0, 0.0], [1.0, 2.0, 3.0]], dtype=np.float32)
        normals = np.array([[0.0, 0.0, 1.0], [0.0, 0.0, 1.0]], dtype=np.float32)
        indices = np.array([0, 1, 1], dtype=np.uint32)
        moved, turned, same = meshes.place(points, normals, indices, position, rows)
        np.testing.assert_allclose(moved, [[70.0, 1.0, 8.5], [71.0, 3.0, 11.5]], atol=1e-4)
        np.testing.assert_allclose(turned, normals, atol=1e-6)
        np.testing.assert_array_equal(same, indices)

    def test_a_yaw_turns_the_mesh_and_a_negative_scale_turns_the_winding(self):
        # Unreal's yaw of 90 degrees turns x towards y; in the render space (y left) that is x towards -y.
        item = {"location": None, "rotation": {"Yaw": 90.0}, "scale": None}
        position, rows = props.placement_of(item)
        moved, _, _ = meshes.place(
            np.array([[1.0, 0.0, 0.0]], dtype=np.float32),
            np.array([[0.0, 0.0, 1.0]], dtype=np.float32),
            np.array([0, 0, 0], dtype=np.uint32),
            position,
            rows,
        )
        self.assertTrue(math.isclose(float(moved[0][0]), 0.0, abs_tol=1e-5))
        self.assertTrue(math.isclose(abs(float(moved[0][1])), 1.0, abs_tol=1e-5))
        mirrored = {"location": None, "rotation": None, "scale": {"X": -1.0, "Y": 1.0, "Z": 1.0}}
        position, rows = props.placement_of(mirrored)
        triangle = np.array([0, 1, 2], dtype=np.uint32)
        _, _, flipped = meshes.place(
            np.zeros((3, 3), dtype=np.float32), np.tile([0.0, 0.0, 1.0], (3, 1)).astype(np.float32), triangle, position, rows
        )
        np.testing.assert_array_equal(flipped, [2, 1, 0])


if __name__ == "__main__":
    unittest.main()
