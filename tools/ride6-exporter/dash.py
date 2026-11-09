"""Compose the screen of every bike's instrument cluster at rest: `tex/<gauge picture>_dash.png` next to the bike's textures.

    python -I dash.py [--out C:\\r6work\\bikes] [--only <regex on the vehicle id>] [--tex C:\\r6work\\dash-tex] [--idle-rpm 1400]

RIDE 6 draws the cluster with two Unreal widgets (`HUD<chassis>` names them, `dash_widgets.py`): the base colour widget into the
base colour of `mat_gauge<chassis>` and the emissive widget into its emissive input. The 2005 car shader has no emissive input, so
the picture written here is the sum of the two renders at the state of a bike at rest (ignition on, neutral, 0 km/h, idle
revolutions), which the importer takes as the cluster's diffuse picture (`mods/ride6-bikes`, `dash::`). The size is the widgets' canvas
(1024), the layout of `gauge<chassis>_d`. A bike whose HUD asset names no widget keeps its picture and the importer's own fallback.
"""

import argparse
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import r6site  # noqa: F401,E402

import numpy as np  # noqa: E402
from PIL import Image  # noqa: E402

from dash_paint import Layer, Painter  # noqa: E402
from dash_widgets import Widget, chassis_number, hud_classes, material_params, merge_config, needle_angle  # noqa: E402
from r6lib import Assets, Tool  # noqa: E402

TEXTURE_PARAMS = ("DayTexture", "NightTexture", "BaseTexture", "GradientTexture", "Threshold1 Texture", "BarsMaskTexture", "EmissiveColor Texture")
# Widgets that show a warning or a unit the bike at rest does not: they are left out of the picture.
HIDDEN = re.compile(
    r"(abs|tcs|tc_|_tc|oil|fuel|limiter|warning|alert|_aw|aw_|dtc|autolights|led_light|^led_eb|eb_led|hidden|miles|mile|mph|^mi$|"
    r"^m$|^m1|^m2|speedlimit|pwr_led|longitudinal|led_rpm|led_white)",
    re.I,
)
SHOWN = re.compile(r"(led_n$|ledn|neutral|bkg|backlight)", re.I)


def idle_state(widget, config, rpm):
    """The values of a bike at rest and which widgets are hidden."""
    fill_entry = config.get("RPMBar", {})
    fill = needle_angle({"x": fill_entry}, "x", rpm) if fill_entry else None
    if fill is None:
        top = fill_entry.get("MaxValue") or 15000.0
        fill = rpm / top
    needle = needle_angle(config, "rpmneedle", rpm)
    speed = needle_angle(config, "speedneedle", 0.0)

    def hide(node):
        name = node.name
        if SHOWN.search(name):
            return False
        return bool(HIDDEN.search(name))

    def text(node):
        name = node.name.lower()
        if "previous" in name or "next" in name or "selected" in name:
            return None
        if name in ("speedtext", "speed"):
            return "0"
        if name in ("gear", "gear_"):
            return "N"
        if name in ("rpmvalue", "rpmtext", "rpm"):
            return "1.4"
        design = node.props.get("Text", {}).get("SourceString", "")
        if design and set(design) <= set("8:.-/ ") and "8" in design:
            # The design text of a lap time or a date is every segment lit: at rest it reads zero, a best lap not yet set dashes.
            return design.replace("8", "-" if "best" in name else "0")
        return None

    def angle(node):
        name = node.name.lower()
        if name.startswith("rpmneedle") or name == "rpm_needle":
            return needle
        if name.startswith("speedneedle") or name == "spdneedle":
            return speed
        return None

    return {"hide": hide, "text": text, "angle": angle, "rpm_fill": float(min(max(fill, 0.0), 1.0))}


def collect_textures(assets, widgets):
    """Every texture package the widgets draw, through their materials."""
    wanted = set()
    for widget in widgets:
        for node in widget.walk():
            if node.kind != "Image":
                continue
            resource = node.props.get("Brush", {}).get("ResourceObject")
            if not resource:
                continue
            path = resource["ObjectPath"].rsplit(".", 1)[0]
            if resource["ObjectName"].startswith("Texture2D"):
                wanted.add(path)
                continue
            params = material_params(assets, path)
            for name in TEXTURE_PARAMS:
                if params and params["tex"].get(name):
                    wanted.add(params["tex"][name])
    return wanted


def fetch_textures(tool, packages, cache):
    """`{package: png path}` for each texture, exported once into `cache/<folder>/<name>.png`."""
    found = {}
    by_folder = {}
    for package in sorted(packages):
        folder, _, name = package.rpartition("/")
        by_folder.setdefault(folder, []).append(name)
    for folder, names in by_folder.items():
        out = os.path.join(cache, folder.strip("/").replace("/", "_"))
        os.makedirs(out, exist_ok=True)
        missing = [n for n in names if not os.path.exists(os.path.join(out, n + ".png"))]
        if missing:
            list_file = os.path.join(out, "_list.txt")
            with open(list_file, "w", encoding="utf-8") as f:
                f.write("\n".join(f"{folder}/{n}" for n in missing) + "\n")
            tool.request("tex", list_file, out, 1024)
            os.remove(list_file)
        for name in names:
            png = os.path.join(out, name + ".png")
            if os.path.exists(png):
                found[f"{folder}/{name}"] = png
    return found


def to_linear(array):
    return (array / 255.0) ** 2.2


def compose(assets, tool, base_package, emissive_package, cache, rpm):
    """The cluster picture (RGB array 1024 x 1024) of the two widgets, or None when neither draws."""
    widgets = [Widget(assets, p) for p in (base_package, emissive_package) if p]
    widgets = [w for w in widgets if w.root is not None]
    if not widgets:
        return None
    textures = fetch_textures(tool, collect_textures(assets, widgets), cache)
    total = np.zeros((1024, 1024, 3), dtype=np.float32)
    fields = []
    for widget in widgets:
        config = merge_config(widget, assets)
        painter = Painter(assets, textures, config, idle_state(widget, config, rpm), 1024)
        layer = Layer(1024)
        painter.render(widget.root, (0.0, 0.0, 1024.0, 1024.0), layer)
        fields.extend(painter.fields)
        arr = np.array(layer.image).astype(np.float32)
        total += to_linear(arr[..., :3]) * (arr[..., 3:4] / 255.0)
    return np.clip(total, 0, 1) ** (1 / 2.2), fields


def gauge_stem(parts):
    for name, material in parts["materials"].items():
        if name.startswith("mat_gauge"):
            path = material.get("tex", {}).get("BaseColor - Texture")
            if path:
                return path.rsplit("/", 1)[-1].split(".")[0]
    return None


def chassis_of(parts):
    for part in parts["parts"]:
        if part["id"] == "chassis":
            return part["export"]
    return None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", default=r"C:\r6work\bikes")
    parser.add_argument("--only", default="")
    parser.add_argument("--tex", default=r"C:\r6work\dash-tex")
    parser.add_argument("--idle-rpm", type=float, default=1400.0)
    args = parser.parse_args()
    tool = Tool()
    assets = Assets(tool)
    try:
        for vehicle in sorted(os.listdir(args.out)):
            folder = os.path.join(args.out, vehicle)
            parts_path = os.path.join(folder, "parts.json")
            if not os.path.exists(parts_path) or not re.search(args.only, vehicle):
                continue
            with open(parts_path, encoding="utf-8") as f:
                parts = json.load(f)
            stem, chassis = gauge_stem(parts), chassis_of(parts)
            if not stem or not chassis:
                continue
            base, emissive = hud_classes(assets, chassis_number(chassis))
            composed = compose(assets, tool, base, emissive, args.tex, args.idle_rpm)
            picture, fields = composed if composed else (None, [])
            if picture is None:
                print(f"{vehicle}: no widget")
                continue
            target = os.path.join(folder, "tex", f"{stem}_dash.png")
            Image.fromarray((picture * 255 + 0.5).astype(np.uint8)).save(target)
            with open(os.path.join(folder, "tex", f"{stem}_dash.json"), "w", encoding="utf-8") as f:
                json.dump({"canvas": 1024, "fields": fields}, f)
            print(f"{vehicle}: {os.path.basename(target)} ({base and base.rsplit('/', 1)[-1]} {emissive and emissive.rsplit('/', 1)[-1]})")
    finally:
        tool.close()


if __name__ == "__main__":
    main()
