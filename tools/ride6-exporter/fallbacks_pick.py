"""The choice of `fallbacks.py`: which surface of a similar model replaces a surface that has no picture.

Pure functions on plain data (a bike is `{vehicle, parts: {id: Part}}`), so the rules are tested with authored data
(`tests/test_ride6_fallbacks.py`). The rules are documented in docs/modding/ride6-bikes-batch.md#grey-surfaces-the-fallbacks.
"""

from collections import namedtuple

from fallbacks_names import CLASS_ORDER, distance, split

# A usable surface of a bike: the part it is on, its glb name, the material instance it uses and that material's resolved entry.
Found = namedtuple("Found", "vehicle part glb material entry")
# A part of a bike as the donor search sees it: the mesh it is, its shape (triangles per kind) and its usable surfaces by kind.
Part = namedtuple("Part", "export signature surfaces")
# The choice: the rule that found it (RULES), the surface taken and, for a flat colour, the colour.
Choice = namedtuple("Choice", "rule found colour")

RULES = {
    "asset": "the same mesh on another bike",
    "family": "the same part of a bike of the same make, the nearest model",
    "shape": "the same part of another make's bike with the same triangles per kind",
    "own": "another surface of the bike itself, of the same kind",
    "make": "the same part of the nearest model of another make",
    "flat": "no similar surface: a flat colour",
}


def by_distance(vehicle, found):
    """`found` sorted by how near its bike is to `vehicle` (same make, longest shared model name start, ...)."""
    return sorted(found, key=lambda f: distance(vehicle, f.vehicle))


def donors(vehicle, part_id, export, signature, kind, library):
    """`[(rule, [Found])]` tiers of the other bikes' surfaces of `kind` on the part `part_id`, in the order they are tried, without
    the bike's own (`own`) which goes between `shape` and `make`.

    `library` is `{vehicle: {part id: Part}}` without `vehicle` itself.
    """
    brand = split(vehicle)[0]
    tiers = {rule: [] for rule in ("asset", "family", "shape", "make")}
    for other_vehicle, parts in sorted(library.items()):
        part = parts.get(part_id)
        if part is None or kind not in part.surfaces:
            continue
        found = part.surfaces[kind]
        if part.export == export:
            tiers["asset"].append(found)
        if split(other_vehicle)[0] == brand:
            tiers["family"].append(found)
            continue
        tiers["make"].append(found)
        if part.signature == signature:
            tiers["shape"].append(found)
    return list(tiers.items())


def own(kind, bike_parts):
    """The bike's own usable surfaces of `kind` (the chassis first); with no kind, its frame's, then its paint's."""
    kinds = (kind,) if kind else CLASS_ORDER
    found = []
    for wanted in kinds:
        ordered = sorted(bike_parts.items(), key=lambda item: (item[0] != "chassis", item[0]))
        found += [part.surfaces[wanted] for _, part in ordered if wanted in part.surfaces]
    return found


def choose(vehicle, part_id, export, signature, kind, library, bike_parts, colour):
    """The surface that replaces a pictureless surface of `kind` on `part_id` of `vehicle`, as a `Choice`.

    The order: the same mesh on another bike (`asset`); the same part on the nearest bike of the same make (`family`); the same part
    of another make's bike with the same triangles per kind (`shape`); another surface of the bike itself of the same kind (`own`);
    the same part on the nearest bike of another make (`make`); a flat colour. A placeholder surface has no kind (`kind` is None) and
    takes the bike's own frame or paint.
    """
    tiers = []
    if kind:
        tiers = donors(vehicle, part_id, export, signature, kind, library)
    tiers.insert(min(3, len(tiers)), ("own", own(kind, bike_parts)))
    for rule, found in tiers:
        ranked = by_distance(vehicle, found) if rule != "own" else found
        if ranked:
            return Choice(rule, ranked[0], None)
    return Choice("flat", None, colour)
