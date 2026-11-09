"""Where the aftermarket exhausts and brake discs of the shop sit on a bike (`docs/formats/ride6-bike-parts.md`, "Where an option
part sits").

A part of the shop is a skeletal mesh whose skeleton shares bones with the chassis skeleton; the game moves a vertex skinned to
bone `b` by `master_world(b) * part_world(b)^-1`. For most parts the master skeleton is the chassis'. A **silencer** is not: its
bones (`BONE_MUFFLER_001`, `BONE_MOUNT_001`) are not in the chassis, they are in the skeleton of the **header pipes** it is
fitted to (`exmanifold<chassis>_<silencer>`: the bone `BONE_MUFFLER_001` of the header is where the silencer goes, with the
scale that fits it to the pipe). The anchor of a silencer is therefore taken on the header's bones, moved into the chassis
frame by the header's own anchor. Placing it on the chassis' ancestor (`VEHICLE_BASE`, the identity) put every silencer at the
origin of its own frame: on the ground under the bike.

A twin silencer (a second set of bones numbered `_002`) is two rigid pieces, one per number: [`silencer_groups`] gives each its
own anchor and the importer writes one solid per group.
"""

import os
import re

import numpy as np

import glbtools

GROUP = re.compile(r"^BONE_(?:MUFFLER|MOUNT)_(\d+)$")


def bone_group(name):
    """The silencer of a bone: the number of `BONE_MUFFLER_00n` and `BONE_MOUNT_00n`; every other bone is the first's."""
    found = GROUP.match(name)
    return int(found[1]) if found else 1


def anchor_matrix(anchor):
    """The 4x4 matrix of an anchor dictionary (`translation`, `rotation`, optional `scale`)."""
    m = np.eye(4)
    m[:3, :3] = glbtools.quat_matrix(anchor["rotation"]) @ np.diag(anchor.get("scale", [1.0, 1.0, 1.0]))
    m[:3, 3] = anchor["translation"]
    return m


def master_bones(chassis_bones, header, header_anchor):
    """The chassis skeleton plus the bones only the header has (its silencer mounts), in the chassis frame."""
    master = dict(chassis_bones)
    placed = anchor_matrix(header_anchor)
    for name, world in header.bones().items():
        if name not in master:
            master[name] = placed @ world
    return master


def silencer_groups(silencer, master):
    """`{group number: anchor}` of the silencer mesh `silencer` (a `Glb`) on the skeleton `master`.

    Each group takes the bone `BONE_MUFFLER_<n>` when the master has it, else the bone that moves most of the group's vertices,
    replaced by its nearest ancestor the master has; None for a silencer without skin or a group no master bone moves.
    """
    groups = {}
    for name, count in silencer.vertex_bone_counts().items():
        groups.setdefault(bone_group(name), {})[name] = count
    out = {}
    for group, tally in sorted(groups.items()):
        wanted = next((name for name in tally if name.startswith("BONE_MUFFLER") and name in master), None)
        if wanted is None:
            shared = {}
            for name, count in tally.items():
                bone = glbtools.shared_bone(silencer, name, master)
                if bone is not None:
                    shared[bone] = shared.get(bone, 0) + count
            if not shared:
                return None
            wanted = max(sorted(shared), key=lambda b: shared[b])
        out[group] = glbtools.anchor_on_bone(silencer, master, wanted, with_scale=True)
    return out or None


def disc_anchor(part, chassis_bones, bone):
    """The anchor of a front brake disc pair on `bone`: the bone of the stock disc of the bike. The pair is one mesh with a left
    and a right bone (`BONE_FL_BRAKE`, `BONE_FR_BRAKE`) and takes one of them for both discs, as the stock disc does; taking the
    pair's own dominant bone put the disc of a bike whose stock disc is on the right on its left."""
    if bone not in chassis_bones or bone not in part.bones():
        return glbtools.anchor(part, chassis_bones)
    return glbtools.anchor_on_bone(part, chassis_bones, bone)


# --- rims: seated on the tyre ------------------------------------------------------------------------------------------------

# How far inside the tyre's bead the rim's lip and flange end, metres. **[choice]**: the rim must reach the bead so that the
# ring between them is closed, and a rim a hair inside it cannot poke through the tyre.
SEAT_RADIAL_INSET = 0.0005
SEAT_AXIAL_INSET = 0.001
# The share of the rim's outer radius from which a vertex counts as the lip (its axial reach is the flange).
FLANGE_SHARE = 0.97
# The scale a rim may take to be seated: in the wheel plane the diameters of a fitting rim are within 3 percent of the stock one
# (`option_export.DIAMETER_TOLERANCE`), so a little more; along the axle the widths of a fitting rim are 0.75 to 1.35 times the
# stock one (`option_export.WIDTH_RANGE`), the same range.
SEAT_RADIAL_CLAMP = (0.95, 1.08)
SEAT_AXIAL_CLAMP = (0.75, 1.35)


class GlbIndex:
    """The `.glb` files under a folder by name and level of detail (any case: the exports have `Muffler988.glb` for `muffler988`),
    found with one walk, and the parsed files kept (the placement reads each part of the catalogue for every bike that lists it)."""

    LOD = re.compile(r"^(?P<stem>.+)_LOD(?P<level>\d)$")

    def __init__(self, folder):
        self.files = {}
        self.parsed = {}
        for root, _, names in os.walk(folder):
            for name in names:
                if not name.endswith(".glb"):
                    continue
                found = self.LOD.match(name[:-4])
                stem, level = (found["stem"], int(found["level"])) if found else (name[:-4], 0)
                self.files.setdefault((stem.lower(), level), os.path.join(root, name))

    def find(self, stem, level=0):
        return self.files.get((stem.lower(), level))

    def smallest(self, stem):
        """The coarsest level of detail the part has (the one that is cheapest to read, and as placed as any)."""
        return next((path for level in (3, 2, 1, 0) if (path := self.find(stem, level))), None)

    def glb(self, path):
        if path not in self.parsed:
            self.parsed[path] = glbtools.Glb(path)
        return self.parsed[path]


def tyre_seat(vertices):
    """`(bead radius, bead half width)` of a tyre mesh in the hub's frame (the wheel plane is x, y and the axle z): the smallest
    radius of any vertex, and how far from the middle the vertices on it reach."""
    radius = np.hypot(vertices[:, 0], vertices[:, 1])
    bead = float(radius.min())
    on_bead = radius <= bead + 0.004
    return bead, float(np.abs(vertices[on_bead, 2]).max())


def rim_seat_scale(vertices, bead, bead_half):
    """The scale `[radial, radial, axial]` that seats the rim mesh `vertices` (hub frame) on a tyre whose bead is `bead` and
    whose bead edges are `bead_half` from the middle: its lip reaches the bead and its flanges the bead's edges, within the
    clamps. A rim the tyre does not have a bead for is not scaled."""
    radius = np.hypot(vertices[:, 0], vertices[:, 1])
    lip = float(radius.max())
    flange = float(np.abs(vertices[radius >= FLANGE_SHARE * lip, 2]).max())
    if lip <= 0.0 or flange <= 0.0 or bead <= 0.0:
        return [1.0, 1.0, 1.0]
    radial = min(max((bead - SEAT_RADIAL_INSET) / lip, SEAT_RADIAL_CLAMP[0]), SEAT_RADIAL_CLAMP[1])
    axial = min(max((bead_half - SEAT_AXIAL_INSET) / flange, SEAT_AXIAL_CLAMP[0]), SEAT_AXIAL_CLAMP[1])
    return [round(radial, 4), round(radial, 4), round(axial, 4)]


def is_hub_frame(anchor):
    """Whether the anchor only moves the part (no turn): the rim mesh is then in the hub's frame, which the scale is about."""
    return all(abs(a - b) < 1e-3 for a, b in zip(anchor["rotation"], (0.0, 0.0, 0.0, 1.0)))
