"""Material instances of the bikes resolved through their parents into the effective parameters of `parts.json`.

An instance overrides some parameters of its parent; the parents end in a master material. A parameter's effective value
is the first one found walking up. The result is the `materials{}` entry of `parts.json`:
`tex{param: object path}, scalar, vector (RGBA lists), switch (static switches), blend_mode, shading_model, clip, chain`.
"""

import os

from r6lib import object_name, ref_path

# Textures exported for a material (parameter names of the vehicle and tyre masters). The others (dirt and rain masks, clear
# coat layers) are not used by the converter.
TEXTURE_PARAMS = (
    "BaseColor - Texture",
    "BaseColorOpacity - Texture",
    "Normal - Texture",
    "PBR - Texture",
    "Transmittance - Texture",
    "BaseColor",
    "CleanNormal",
    "Pbr",
    "ColorCustomization - Mask - Texture",
)


# The normal maps are 40 of the 125 MB a bike takes in the export folder and are read only by `ride6-bike-import --normal-maps`
# (off by default): they are exported when the environment variable R6_NORMALS is set (`export_all.py --normals`).
NORMAL_PARAMS = ("Normal - Texture", "CleanNormal")


def wanted_params():
    if os.environ.get("R6_NORMALS"):
        return TEXTURE_PARAMS
    return tuple(name for name in TEXTURE_PARAMS if name not in NORMAL_PARAMS)


def strip_enum(value):
    """`EBlendMode::BLEND_Masked` is `BLEND_Masked`."""
    if not isinstance(value, str):
        return None
    return value.split("::")[-1]


def resolve(assets, path):
    """The effective parameters of the material instance at `path`; None when the package is missing."""
    chain = []
    seen = set()
    current = path
    out = {"tex": {}, "scalar": {}, "vector": {}, "switch": {}}
    while current and current not in seen:
        seen.add(current)
        export = assets.named(current)
        if export is None:
            data = assets.exports(current)
            export = data[0] if data else None
        if export is None:
            break
        chain.append(object_name(current))
        props = export.get("Properties") or {}
        take(out, props)
        if export.get("Type") != "MaterialInstanceConstant":
            break
        current = ref_path(props.get("Parent"))
    if not chain:
        return None
    out["chain"] = chain
    return out


def take(out, props):
    """Add what an instance (or the master) sets and the nearer instances have not."""
    for item in props.get("TextureParameterValues") or []:
        name = item["ParameterInfo"]["Name"]
        path = ref_path(item.get("ParameterValue"))
        if path:
            out["tex"].setdefault(name, path)
    for item in props.get("ScalarParameterValues") or []:
        out["scalar"].setdefault(item["ParameterInfo"]["Name"], item["ParameterValue"])
    for item in props.get("VectorParameterValues") or []:
        v = item["ParameterValue"]
        out["vector"].setdefault(item["ParameterInfo"]["Name"], [v["R"], v["G"], v["B"], v["A"]])
    static = props.get("StaticParametersRuntime") or {}
    for item in static.get("StaticSwitchParameters") or []:
        if item.get("bOverride", True):
            out["switch"].setdefault(item["ParameterInfo"]["Name"], bool(item["Value"]))
    overrides = props.get("BasePropertyOverrides") or {}
    blend = strip_enum(overrides.get("BlendMode"))
    if blend:
        out.setdefault("blend_mode", blend)
    shading = strip_enum(overrides.get("ShadingModel"))
    if shading:
        out.setdefault("shading_model", shading)
    if "OpacityMaskClipValue" in overrides:
        out.setdefault("clip", overrides["OpacityMaskClipValue"])


ENGINE_FLAT = ("t_default", "t_white", "t_black", "defaulttexturechecker", "defaultmaskchecker", "base_flat", "normalmap_test", "flatnormal")


def textures_of(material):
    """The object paths of the textures a resolved material needs exported (not the engine's flat default pictures)."""
    paths = [material["tex"][name] for name in wanted_params() if name in material["tex"]]
    return [p for p in paths if not stem(p).startswith(ENGINE_FLAT)]


def stem(path):
    return os.path.basename(path.replace("\\", "/")).split(".")[0].lower()
