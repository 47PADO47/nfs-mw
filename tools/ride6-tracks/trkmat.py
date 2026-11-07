"""Ground materials of a track: which picture, which tiling and which surface each one means.

RIDE 6 draws the ground with layered master materials: `BASE_Terrain3_4L` blends four ground pictures by the vertex colour of
the mesh, the tarmac and kerb masters draw one picture with overlays on top. A track pack keeps one picture per drawn layer
and the surface a wheel feels (`docs/formats/ride6-track-pack.md`). Nothing here is game data: the tables are our own reading.
"""

from common import materials as r6materials
from common import r6lib

# The physical material of a mesh slot to the grip the game's own surface table has (`simsurface`, docs/formats/collision.md).
PHYS_SURFACE = {
    "pm_t_tarmac": "asphalt",
    "pm_t_tarmacdirty": "asphalt",
    "pm_t_concrete": "concrete",
    "pm_t_concretegrid": "concrete",
    "pm_t_grass": "grass",
    "pm_t_grasssynth": "grass",
    "pm_t_gravel": "gravel",
    "pm_t_loosegravel": "gravel",
    "pm_t_sand": "sand",
    "pm_t_kerb": "concrete",
    "pm_t_metalgrid": "metal",
}

# The surface of a ground picture of a layered material, by what its name says (the layered terrain has no physical material
# of its own for the ground slices).
LAYER_SURFACE = (
    ("tarmac", "asphalt"),
    ("concrete", "concrete"),
    ("grass", "grass"),
    ("forest", "grass"),
    ("soil", "dirt"),
    ("sand", "sand"),
    ("gravel", "gravel"),
    ("rubble", "gravel"),
    ("rock", "stone"),
    ("kerb", "concrete"),
)

# Tile length in metres of a picture at a tiling of 1 (a measured guess, not a reading of the shader: see the docs).
TILE_METRES = 10.0


def stem(path):
    return r6lib.object_name(path)


def phys_of(assets, path):
    """The physical material name of a material instance, walking up its parents (lower case), or None."""
    seen = set()
    while path and path not in seen:
        seen.add(path)
        export = assets.named(path) or (assets.exports(path) or [None])[0]
        if export is None:
            return None
        props = export.get("Properties") or {}
        phys = props.get("PhysMaterial")
        if isinstance(phys, dict) and phys.get("ObjectPath"):
            return stem(phys["ObjectPath"])
        parent = props.get("Parent")
        path = parent.get("ObjectPath") if isinstance(parent, dict) else None
    return None


def surface_of_picture(name):
    for word, surface in LAYER_SURFACE:
        if word in name:
            return surface
    return "dirt"


def layer_scale(resolved, layer):
    """The tiling of a layer (the R channel of its `UV - ScaleOffset`), 1 when the material does not set it."""
    value = resolved["vector"].get(f"Layer{layer:02d} - UV - ScaleOffset")
    return value[0] if value and value[0] > 0 else 1.0


def classify(assets, path):
    """What a material instance is: `{family, name, phys, layers: [{picture, surface, tile_m}], picture, tile_m}`.

    Families: `terrain` (four layers blended by vertex colour), `tarmac`, `kerb`, `prop` and `other` (one picture or none).
    """
    resolved = r6materials.resolve(assets, path)
    if resolved is None:
        return None
    chain = resolved["chain"]
    phys = phys_of(assets, path)
    info = {"name": stem(path), "phys": phys, "chain": chain, "family": "other", "layers": [], "picture": None}
    tex = resolved["tex"]
    if any(c.startswith("base_terrain3_4l") for c in chain):
        info["family"] = "terrain"
        for layer in range(1, 5):
            picture = tex.get(f"Layer{layer:02d} - BaseColor - Texture")
            if not picture:
                continue
            name = stem(picture)
            tile = TILE_METRES / layer_scale(resolved, layer)
            info["layers"].append(
                {"layer": layer, "picture": picture, "surface": surface_of_picture(name), "tile_m": tile}
            )
        return info
    for key, family in (("Tarmac - BaseColor - Texture", "tarmac"), ("Layer01 - BaseColor - Texture", "kerb")):
        picture = tex.get(key)
        if picture:
            info["family"] = family
            info["picture"] = picture
            info["tile_m"] = TILE_METRES / float(resolved["scalar"].get("Tarmac - UV - Tiling", 1.0))
            return info
    picture = tex.get("BaseColor - Texture")
    if picture:
        info["family"] = "prop"
        info["picture"] = picture
    return info


def surface_of(info, picture=None):
    """The collision surface of a material, or of one layer's picture of a layered one.

    A layered material's physical material is one value for the whole slice, so a layer's own picture decides, except
    that a slice flagged as sand or gravel makes its earth layers sand or gravel.
    """
    flagged = PHYS_SURFACE.get(info["phys"])
    if info["family"] != "terrain":
        return flagged or surface_of_picture(stem(info["picture"])) if info["picture"] else flagged or "asphalt"
    layer = surface_of_picture(stem(picture)) if picture else "grass"
    if layer in ("dirt", "grass") and flagged in ("sand", "gravel"):
        return flagged
    return layer
