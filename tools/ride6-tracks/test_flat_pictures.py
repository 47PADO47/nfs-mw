#!/usr/bin/env python3
"""Tests of the fallback for pictures the exporter cannot read: `python -I tools/ride6-tracks/test_flat_pictures.py`. No game data and no install are needed."""

import math
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import common  # noqa: E402,F401  (first: it makes the user site-packages importable)
import export  # noqa: E402

class FlatPictureTests(unittest.TestCase):
    def test_a_plugin_picture_becomes_its_colour_and_other_missing_pictures_keep_theirs(self):
        class Builder:
            materials = [
                {"texture": "t_white_color", "color": [140, 140, 140]},
                {"texture": "t_gone", "color": [1, 2, 3]},
                {"texture": "t_here", "color": [9, 9, 9]},
                {"texture": None, "color": [5, 5, 5]},
            ]

        with tempfile.TemporaryDirectory() as folder:
            open(os.path.join(folder, "t_here.png"), "wb").close()
            builder = Builder()
            export.use_colours_for_missing(builder, folder)
        self.assertEqual(builder.materials[0], {"texture": None, "color": [255, 255, 255]})
        self.assertEqual(builder.materials[1], {"texture": None, "color": [1, 2, 3]})
        self.assertEqual(builder.materials[2]["texture"], "t_here")
        self.assertIsNone(builder.materials[3]["texture"])


if __name__ == "__main__":
    unittest.main()
