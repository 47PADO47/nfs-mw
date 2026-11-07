"""The instrument cluster widgets of RIDE 6 (`ui/Widgets/gauges/gauge_<n>/gauge_<n>[_E]`) as a tree of plain Python objects.

RIDE 6 draws the screen of a bike's cluster with a Unreal Motion Graphics widget that a 3D widget component renders into the
texture of `mat_gauge<chassis>`: the base colour widget (`gauge_<n>`) into the base colour, the emissive widget (`gauge_<n>_E`)
into the emissive input. The vehicle's `HUD<chassis>` data asset names the two classes. A widget tree holds only canvas panels,
horizontal boxes, images (a texture or a material instance of one of three UI materials) and text blocks. See
docs/formats/ride6-materials.md ("Instrument clusters"). Nothing of the game is stored here.
"""

import re

from r6lib import object_name, package_key

HUD_DIR = "/Game/assets/data/vehicles/HUD/"


def key_of(object_name_text):
    """The part of an object reference between the quotes: `Image'gauge_127_C:WidgetTree.BkgGauge'` is `gauge_127_C:WidgetTree.BkgGauge`."""
    parts = object_name_text.split("'")
    return parts[1] if len(parts) > 1 else object_name_text


def hud_classes(assets, chassis_number):
    """The base colour and the emissive widget package of `HUD<number>`, either may be None."""
    path = f"{HUD_DIR}HUD{chassis_number}"
    assets.ensure([path])
    exports = assets.exports(path)
    if not exports:
        return None, None
    props = exports[0].get("Properties", {})

    def package(name):
        value = props.get(name)
        return value["ObjectPath"].rsplit(".", 1)[0] if value else None

    return package("BaseColorCockpitHUDClass"), package("EmissiveColorCockpitHUDClass")


class Node:
    """One widget: its kind (`CanvasPanel`, `HorizontalBox`, `Image`, `TextBlock`), properties, slot and children."""

    def __init__(self, kind, name, props, slot):
        self.kind = kind
        self.name = name
        self.props = props
        self.slot = slot  # the properties of the slot this widget sits in
        self.children = []

    def visible(self):
        return self.props.get("Visibility", "") not in ("ESlateVisibility::Collapsed", "ESlateVisibility::Hidden")


class Widget:
    """A parsed widget package: the node tree and the gauge configuration (needle angle mappings, units)."""

    def __init__(self, assets, package):
        assets.ensure([package])
        exports = assets.exports(package) or []
        self.package = package
        self.exports = exports
        self.by_key = {}
        for export in exports:
            outer = export.get("Outer", {}).get("ObjectName")
            self.by_key[(key_of(outer) + "." if outer else "") + export["Name"]] = export
        self.config = {}
        for export in exports:
            if export["Name"].startswith("Default__"):
                self.config = export.get("Properties", {}).get("GaugeConfiguration", {})
        self.config_asset = None
        for export in exports:
            if export["Name"].startswith("Default__"):
                asset = export.get("Properties", {}).get("GaugeConfigurationAsset")
                if asset:
                    self.config_asset = asset["ObjectPath"].rsplit(".", 1)[0]
        self.root = None
        tree = next((e for e in exports if e["Type"] == "WidgetTree"), None)
        if tree and "RootWidget" in tree.get("Properties", {}):
            self.root = self.build(key_of(tree["Properties"]["RootWidget"]["ObjectName"]), None)

    def build(self, key, slot):
        export = self.by_key.get(key)
        if export is None:
            return None
        node = Node(export["Type"], export["Name"], export.get("Properties", {}), slot)
        for slot_ref in node.props.get("Slots", []):
            slot_export = self.by_key.get(key_of(slot_ref["ObjectName"]))
            if slot_export is None:
                continue
            slot_props = slot_export.get("Properties", {})
            content = slot_props.get("Content")
            if not content:
                continue
            child = self.build(key_of(content["ObjectName"]), dict(slot_props, _type=slot_export["Type"]))
            if child is not None:
                node.children.append(child)
        return node

    def walk(self, node=None):
        node = node or self.root
        if node is None:
            return
        yield node
        for child in node.children:
            yield from self.walk(child)


def merge_config(widget, assets):
    """The gauge configuration of a widget: the asset it names over the defaults in its class."""
    config = dict(widget.config)
    if widget.config_asset:
        assets.ensure([widget.config_asset])
        exports = assets.exports(widget.config_asset) or []
        for export in exports:
            config.update(export.get("Properties", {}).get("GaugeConfiguration", {}))
    return config


def needle_angle(config, name, value):
    """The angle (degrees) of the needle `name` (`rpmneedle`, `speedneedle`) at `value` by the configuration, or None."""
    entry = config.get(name)
    if not entry:
        return None
    points = [(p["From"], p["To"]) for p in entry.get("NonLinearMapping", [])]
    if points and entry.get("UseNonLinearMapping", True):
        points.sort()
        if value <= points[0][0]:
            return points[0][1]
        for (f0, t0), (f1, t1) in zip(points, points[1:]):
            if value <= f1:
                return t0 + (t1 - t0) * (value - f0) / max(f1 - f0, 1e-6)
        return points[-1][1]
    maximum = entry.get("MaxValue")
    top = entry.get("MaxAngleDegrees")
    if maximum and top is not None:
        low = entry.get("MinAngleDegrees", 0.0)
        return low + (top - low) * value / maximum
    return None


def material_params(assets, path):
    """The parameters of a material instance reached through `path`: parent name, scalars, vectors and textures."""
    assets.ensure([path])
    exports = assets.exports(path)
    if not exports:
        return None
    props = exports[0].get("Properties", {})
    out = {"parent": object_name(props.get("Parent", {}).get("ObjectPath", "")), "scalar": {}, "vector": {}, "tex": {}, "switch": {}}
    for entry in props.get("ScalarParameterValues", []):
        out["scalar"][entry["ParameterInfo"]["Name"]] = entry["ParameterValue"]
    for entry in props.get("VectorParameterValues", []):
        value = entry["ParameterValue"]
        out["vector"][entry["ParameterInfo"]["Name"]] = (value["R"], value["G"], value["B"], value["A"])
    for entry in props.get("TextureParameterValues", []):
        value = entry["ParameterValue"]
        out["tex"][entry["ParameterInfo"]["Name"]] = value["ObjectPath"].rsplit(".", 1)[0] if value else None
    for entry in props.get("StaticParametersRuntime", {}).get("StaticSwitchParameters", []):
        out["switch"][entry["ParameterInfo"]["Name"]] = entry["Value"]
    return out


def chassis_number(chassis_export):
    """`chassis127` or `Chassis051` is `127` / `051`."""
    match = re.search(r"(\d+)$", chassis_export)
    return match.group(1) if match else None
