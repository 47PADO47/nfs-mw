#!/usr/bin/env python3
"""Overhead picture of the Fairhaven census (rectangles of the PVS zone list by district, instance centres as dots).

Used by chameleon_census.py. The engine's x axis points left and z forward, so the picture shows x mirrored and z up.
"""

from PIL import Image, ImageDraw

DISTRICT_COLOURS = {164408: (200, 90, 90), 701574: (90, 150, 210), 701575: (90, 190, 110),
                    152518: (220, 180, 60), 2021698: (170, 110, 200)}


def overhead(rows, out_path, width=1600):
    rects = [r["pvs"] for r in rows if r.get("pvs")]
    xmin = min(p["x"][0] for p in rects)
    xmax = max(p["x"][1] for p in rects)
    zmin = min(p["z"][0] for p in rects)
    zmax = max(p["z"][1] for p in rects)
    scale = (width - 40) / (xmax - xmin)
    height = int((zmax - zmin) * scale) + 40

    def to_px(x, z):
        return 20 + (xmax - x) * scale, 20 + (zmax - z) * scale

    img = Image.new("RGB", (width, height), (24, 26, 32))
    dr = ImageDraw.Draw(img, "RGBA")
    for r in rows:
        p = r.get("pvs")
        if not p:
            continue
        colour = DISTRICT_COLOURS.get(p["district"], (150, 150, 150))
        x0, y0 = to_px(p["x"][1], p["z"][1])
        x1, y1 = to_px(p["x"][0], p["z"][0])
        dr.rectangle([x0, y0, x1, y1], fill=colour + (50,), outline=colour + (255,))
        dr.text((x0 + 3, y0 + 2), str(r["unit"]), fill=(230, 230, 230, 255))
    for r in rows:
        for x, z in r.get("points", []):
            px, py = to_px(x, z)
            dr.point((px, py), fill=(255, 255, 255, 200))
    img.save(out_path)
    print("wrote", out_path, img.size)
