"""Placing a skinned part on the chassis: the part's vertices moved into the chassis frame by the chassis skeleton's pose.

In the game a part is a skeletal mesh that follows the chassis skeleton: the vertex skinned to bone `b` is moved by
`chassis_world(b) * part_world(b)^-1` (linear blend over its four influences). A part whose bones all move alike (a
silencer on `BONE_CHASSIS`, a wheel part on its hub bone) is one rigid transform, the part's anchor. A part whose bones move
differently (the hand guards and the mirrors, one mesh with a left and a right bone) cannot be one transform: its vertices are
rewritten in the chassis frame in every level of detail file, and its anchor is the identity.
"""

import json
import struct

import numpy as np

import glbtools

RIGID_TOLERANCE = 1e-4


def joint_transforms(part, chassis_bones):
    """`[4x4 matrix per joint of the part's skin]`: where the chassis pose takes the part's bind pose at that joint."""
    names = part.joint_names()
    index = {part.names[i]: i for i in range(len(part.names))}
    out = []
    for name in names:
        shared = glbtools.shared_bone(part, name, chassis_bones)
        if shared is None:
            out.append(np.eye(4))
            continue
        out.append(chassis_bones[shared] @ np.linalg.inv(part.world(index[shared])))
    return out


def is_rigid(part, chassis_bones):
    """Whether every joint that moves a vertex has the same transform, within a tolerance."""
    matrices = joint_transforms(part, chassis_bones)
    used = set(part.vertex_bone_counts())
    names = part.joint_names()
    used_matrices = [m for name, m in zip(names, matrices) if name in used]
    if not used_matrices:
        return True
    first = used_matrices[0]
    return all(np.abs(m - first).max() < RIGID_TOLERANCE for m in used_matrices)


def set_accessor(glb, bin_data, index, values):
    """Write float32 `values` into accessor `index` of the (mutable) binary chunk and update its min and max."""
    acc = glb.doc["accessors"][index]
    view = glb.doc["bufferViews"][acc["bufferView"]]
    width = values.shape[1]
    start = view.get("byteOffset", 0) + acc.get("byteOffset", 0)
    stride = view.get("byteStride") or width * 4
    data = values.astype("<f4")
    if stride == width * 4:
        target = np.frombuffer(bin_data, dtype="<f4", count=acc["count"] * width, offset=start)
        target[:] = data.reshape(-1)
    if stride != width * 4:
        for row in range(acc["count"]):
            at = start + row * stride
            bin_data[at : at + width * 4] = data[row].tobytes()
    if acc["type"] == "VEC3":
        acc["min"] = [float(v) for v in values.min(axis=0)]
        acc["max"] = [float(v) for v in values.max(axis=0)]


def bake(path, chassis_bones):
    """Rewrite the glb at `path` with its vertices in the chassis frame."""
    part = glbtools.Glb(path)
    matrices = joint_transforms(part, chassis_bones)
    bin_data = bytearray(part.bin)
    for mesh in part.doc.get("meshes", []):
        for prim in mesh.get("primitives", []):
            attrs = prim["attributes"]
            if "JOINTS_0" not in attrs or "WEIGHTS_0" not in attrs:
                continue
            joints = part.accessor(attrs["JOINTS_0"]).astype(np.int64)
            weights = part.accessor(attrs["WEIGHTS_0"]).astype(np.float64)
            total = weights.sum(axis=1, keepdims=True)
            weights = weights / np.where(total == 0, 1, total)
            blend = np.zeros((joints.shape[0], 4, 4))
            stack = np.array(matrices)
            for k in range(joints.shape[1]):
                blend += weights[:, k, None, None] * stack[joints[:, k]]
            pos = part.accessor(attrs["POSITION"]).astype(np.float64)
            moved = np.einsum("nij,nj->ni", blend[:, :3, :3], pos) + blend[:, :3, 3]
            set_accessor(part, bin_data, attrs["POSITION"], moved)
            for key in ("NORMAL", "TANGENT"):
                if key not in attrs:
                    continue
                vec = part.accessor(attrs[key]).astype(np.float64)
                turned = np.einsum("nij,nj->ni", blend[:, :3, :3], vec[:, :3])
                length = np.linalg.norm(turned, axis=1, keepdims=True)
                turned = turned / np.where(length == 0, 1, length)
                out = turned if vec.shape[1] == 3 else np.concatenate([turned, vec[:, 3:]], axis=1)
                set_accessor(part, bin_data, attrs[key], out)
    write_glb(path, part.doc, bytes(bin_data))


def write_glb(path, doc, bin_data):
    text = json.dumps(doc, separators=(",", ":")).encode("utf-8")
    text += b" " * (-len(text) % 4)
    bin_data = bin_data + b"\0" * (-len(bin_data) % 4)
    total = 12 + 8 + len(text) + 8 + len(bin_data)
    with open(path, "wb") as f:
        f.write(struct.pack("<III", 0x46546C67, 2, total))
        f.write(struct.pack("<II", len(text), 0x4E4F534A) + text)
        f.write(struct.pack("<II", len(bin_data), 0x004E4942) + bin_data)
