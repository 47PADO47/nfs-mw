#!/usr/bin/env python3
"""A top view of a track pack, by surface (a check that an export is sane, not a renderer).

    python -I tools/ride6-tracks/preview.py <pack folder> <out.png> [--metres-per-pixel 2]
"""

import argparse
import json
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import common  # noqa: E402,F401  (first: it makes the user site-packages importable)
import numpy as np  # noqa: E402
from PIL import Image  # noqa: E402

COLOURS = {
    "asphalt": (70, 70, 80),
    "concrete": (170, 170, 170),
    "grass": (60, 140, 60),
    "dirt": (130, 95, 60),
    "sand": (225, 205, 140),
    "gravel": (150, 140, 130),
    "stone": (110, 100, 100),
    "metal": (60, 160, 220),
}


def read_groups(path):
    data = open(path, "rb").read()
    count = struct.unpack_from("<I", data, 8)[0]
    headers = [struct.unpack_from("<IIII", data, 12 + 16 * i) for i in range(count)]
    at = 12 + 16 * count
    groups = []
    for material, vertices, indices, flags in headers:
        positions = np.frombuffer(data, "<f4", vertices * 3, at).reshape(-1, 3)
        at += vertices * 12 + vertices * 8 + vertices * 4
        index = np.frombuffer(data, "<u4", indices, at).reshape(-1, 3)
        at += indices * 4
        groups.append((material, flags, positions, index))
    return groups


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("pack")
    parser.add_argument("out")
    parser.add_argument("--metres-per-pixel", type=float, default=2.0)
    args = parser.parse_args()
    meta = json.load(open(os.path.join(args.pack, "pack.json")))
    groups = read_groups(os.path.join(args.pack, "ground.r6m"))
    cells = []
    for material, _flags, positions, index in groups:
        centre = positions[index].mean(axis=1)
        colour = COLOURS.get(meta["materials"][material]["surface"], (255, 0, 255))
        cells.append((centre, np.tile(np.array(colour, dtype=np.uint8), (len(centre), 1))))
    centres = np.concatenate([c for c, _ in cells])
    colours = np.concatenate([c for _, c in cells])
    low = centres[:, :2].min(axis=0)
    high = centres[:, :2].max(axis=0)
    step = args.metres_per_pixel
    size = np.ceil((high - low) / step).astype(int) + 1
    image = np.zeros((size[1], size[0], 3), dtype=np.uint8)
    order = np.argsort(centres[:, 2])
    px = ((centres[order, 0] - low[0]) / step).astype(int)
    py = ((high[1] - centres[order, 1]) / step).astype(int)
    image[py, px] = colours[order]
    lap = meta["lap"]["spline"]["points"]
    for x, y, _z in lap:
        i, j = int((x - low[0]) / step), int((high[1] - y) / step)
        image[max(0, j - 1) : j + 2, max(0, i - 1) : i + 2] = (255, 0, 0)
    Image.fromarray(image).save(args.out)
    print("extent", low, high, "image", image.shape)


if __name__ == "__main__":
    main()
