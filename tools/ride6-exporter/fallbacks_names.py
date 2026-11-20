"""Names for `fallbacks.py`: the kind of a surface (`mat_glass_edge` and `mat_glasstint` are glass) and how near two bikes are.

Pure functions on strings, tested in `tests/test_ride6_fallbacks.py`. Written for docs/modding/ride6-bikes-batch.md#grey-surfaces-the-fallbacks.
"""

import difflib
import os
import re

# The tokens of a vehicle id that make a variant of the same model (the event, race and Ride-Fest builds).
VARIANTS = ("rm", "em", "ride", "fest")
# Surface kinds written differently on different bikes, and kinds that share a look.
ALIAS = {"lights": "light", "glasstint": "glass", "glassedge": "glass", "glassdof": "glass"}
# A surface the engine left on its placeholder: the kind comes from the mesh slot instead, and where that is no help either the
# bike's own surfaces are tried in this order (the neutral metal of the frame, then the paint).
PLACEHOLDERS = ("worldgridmaterial", "defaultmaterial")
CLASS_ORDER = ("mechanics", "livery")


def kind(name):
    """The kind of a surface or slot name: `mat_livery_0_345_001` is `livery`, `BASE_glass_edge` and `mat_glasstint` are `glass`.

    `None` for an engine placeholder (`WorldGridMaterial`, `Default_Material`) and for the far levels' composite slot.
    """
    text = name.lower()
    for prefix in ("mat_", "base_"):
        if text.startswith(prefix):
            text = text[len(prefix):]
    text = re.sub(r"\d+", "", text).replace("_", "")
    if not text or text in PLACEHOLDERS or text.startswith(("bikelods", "liverylod")):
        return None
    return ALIAS.get(text, text)


def surface_kind(glb_name, slot=None):
    """The kind of a glb surface: its own name's, else (a placeholder) its mesh slot's."""
    return kind(glb_name) or (kind(slot) if slot else None)


def split(vehicle):
    """`(brand, model words without the variant tokens and the year, year)`, lower case, of a vehicle id."""
    brand, _, rest = vehicle.partition("_")
    tokens = rest.split("-")
    year = int(tokens[-1]) if re.fullmatch(r"\d{4}", tokens[-1]) else 0
    words = [t for t in (tokens[:-1] if year else tokens) if t.lower() not in VARIANTS]
    return brand.lower(), "-".join(words).lower(), year


def distance(vehicle, other):
    """How far `other` is from `vehicle`, to sort by: another brand, then the model words they do not share (the longer the common
    start the nearer, then the likeness of the words), then the years apart, then the name (so the order is fixed)."""
    brand, words, year = split(vehicle)
    other_brand, other_words, other_year = split(other)
    prefix = len(os.path.commonprefix([words, other_words]))
    likeness = round(difflib.SequenceMatcher(None, words, other_words).ratio(), 3)
    return (brand != other_brand, -prefix, -likeness, abs(year - other_year), other)
