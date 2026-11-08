"""Reading the node tree and skinning of the exported `.glb` files, and where a part sits on the chassis skeleton.

Every part mesh is a skeletal mesh whose skeleton shares bones with the chassis skeleton (`BONE_F_VIS_WHEEL`, `BONE_FL_BRAKE`,
`BONE_BARPAD`, ...). A part's anchor is the rigid transform from the part's model space into the chassis model space at one
shared bone: `chassis_world(bone) * part_world(bone)^-1`, taking the shared bone that moves most of the part's vertices.
"""

import json
import struct

import r6site  # noqa: F401  (user site-packages under python -I)
import numpy as np

COMPONENT = {5120: np.int8, 5121: np.uint8, 5122: np.int16, 5123: np.uint16, 5125: np.uint32, 5126: np.float32}
WIDTH = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}


class Glb:
    def __init__(self, path):
        with open(path, "rb") as f:
            data = f.read()
        magic, _, _ = struct.unpack_from("<III", data, 0)
        if magic != 0x46546C67:
            raise ValueError(f"{path} is not a glb")
        offset = 12
        self.doc = None
        self.bin = b""
        while offset < len(data):
            size, kind = struct.unpack_from("<II", data, offset)
            body = data[offset + 8 : offset + 8 + size]
            if kind == 0x4E4F534A:
                self.doc = json.loads(body)
            if kind == 0x004E4942:
                self.bin = body
            offset += 8 + size
        self.nodes = self.doc.get("nodes", [])
        self.parent = {}
        for index, node in enumerate(self.nodes):
            for child in node.get("children", []):
                self.parent[child] = index
        self.names = [n.get("name", f"node{i}") for i, n in enumerate(self.nodes)]
        self._world = {}

    def local(self, index):
        node = self.nodes[index]
        if "matrix" in node:
            return np.array(node["matrix"], dtype=np.float64).reshape(4, 4).T
        t = node.get("translation", [0, 0, 0])
        q = node.get("rotation", [0, 0, 0, 1])
        s = node.get("scale", [1, 1, 1])
        m = np.eye(4)
        m[:3, :3] = quat_matrix(q) @ np.diag(s)
        m[:3, 3] = t
        return m

    def world(self, index):
        if index in self._world:
            return self._world[index]
        m = self.local(index)
        parent = self.parent.get(index)
        if parent is not None:
            m = self.world(parent) @ m
        self._world[index] = m
        return m

    def accessor(self, index):
        acc = self.doc["accessors"][index]
        view = self.doc["bufferViews"][acc["bufferView"]]
        dtype = COMPONENT[acc["componentType"]]
        width = WIDTH[acc["type"]]
        count = acc["count"]
        start = view.get("byteOffset", 0) + acc.get("byteOffset", 0)
        itemsize = np.dtype(dtype).itemsize * width
        stride = view.get("byteStride") or itemsize
        if stride == itemsize:
            array = np.frombuffer(self.bin, dtype=dtype, count=count * width, offset=start)
            return array.reshape(count, width)
        rows = [np.frombuffer(self.bin, dtype=dtype, count=width, offset=start + i * stride) for i in range(count)]
        return np.array(rows)

    def joint_names(self):
        skins = self.doc.get("skins", [])
        if not skins:
            return []
        return [self.names[j] for j in skins[0]["joints"]]

    def bones(self):
        """Every bone of the skin as `name -> world matrix` (the node tree of a skeletal mesh export)."""
        skins = self.doc.get("skins", [])
        if not skins:
            return {}
        return {self.names[j]: self.world(j) for j in skins[0]["joints"]}

    def material_names(self):
        return [m.get("name", "") for m in self.doc.get("materials", [])]

    def vertex_bone_counts(self):
        """How many vertices each bone moves most (the bone of the largest weight of a vertex)."""
        counts = {}
        joints = self.joint_names()
        for mesh in self.doc.get("meshes", []):
            for prim in mesh.get("primitives", []):
                attrs = prim["attributes"]
                if "JOINTS_0" not in attrs or "WEIGHTS_0" not in attrs:
                    continue
                j = self.accessor(attrs["JOINTS_0"])
                w = self.accessor(attrs["WEIGHTS_0"]).astype(np.float64)
                best = np.take_along_axis(j, np.argmax(w, axis=1)[:, None], axis=1)[:, 0]
                for index, n in zip(*np.unique(best, return_counts=True)):
                    name = joints[int(index)]
                    counts[name] = counts.get(name, 0) + int(n)
        return counts

    def positions(self):
        """Every vertex position of every primitive."""
        out = []
        for mesh in self.doc.get("meshes", []):
            for prim in mesh.get("primitives", []):
                out.append(self.accessor(prim["attributes"]["POSITION"]).astype(np.float64))
        if not out:
            return np.zeros((0, 3))
        return np.concatenate(out)


def quat_matrix(q):
    x, y, z, w = q
    n = x * x + y * y + z * z + w * w
    if n == 0:
        return np.eye(3)
    s = 2.0 / n
    return np.array(
        [
            [1 - s * (y * y + z * z), s * (x * y - z * w), s * (x * z + y * w)],
            [s * (x * y + z * w), 1 - s * (x * x + z * z), s * (y * z - x * w)],
            [s * (x * z - y * w), s * (y * z + x * w), 1 - s * (x * x + y * y)],
        ]
    )


def matrix_quat(m):
    """The unit quaternion (x, y, z, w) of the rotation part of a 4x4 matrix (scale removed), w not negative."""
    r = np.array(m[:3, :3], dtype=np.float64)
    for i in range(3):
        norm = np.linalg.norm(r[:, i])
        if norm > 0:
            r[:, i] /= norm
    return _normalised(_raw_quat(r))


def _normalised(q):
    q = np.array(q)
    q /= np.linalg.norm(q)
    if q[3] < 0:
        q = -q
    return [float(v) for v in q]


def _raw_quat(r):
    trace = r[0, 0] + r[1, 1] + r[2, 2]
    if trace > 0:
        s = 0.5 / np.sqrt(trace + 1.0)
        return [(r[2, 1] - r[1, 2]) * s, (r[0, 2] - r[2, 0]) * s, (r[1, 0] - r[0, 1]) * s, 0.25 / s]
    if r[0, 0] > r[1, 1] and r[0, 0] > r[2, 2]:
        s = 2.0 * np.sqrt(1.0 + r[0, 0] - r[1, 1] - r[2, 2])
        return [0.25 * s, (r[0, 1] + r[1, 0]) / s, (r[0, 2] + r[2, 0]) / s, (r[2, 1] - r[1, 2]) / s]
    if r[1, 1] > r[2, 2]:
        s = 2.0 * np.sqrt(1.0 + r[1, 1] - r[0, 0] - r[2, 2])
        return [(r[0, 1] + r[1, 0]) / s, 0.25 * s, (r[1, 2] + r[2, 1]) / s, (r[0, 2] - r[2, 0]) / s]
    s = 2.0 * np.sqrt(1.0 + r[2, 2] - r[0, 0] - r[1, 1])
    return [(r[0, 2] + r[2, 0]) / s, (r[1, 2] + r[2, 1]) / s, 0.25 * s, (r[1, 0] - r[0, 1]) / s]


def anchor(part, chassis_bones):
    """The anchor of a part glb on the chassis skeleton: `{bone, translation, rotation}` (None when it has no skin).

    `chassis_bones` maps chassis bone names to world matrices. A bone only the part has is replaced by its nearest ancestor in
    the part's tree that the chassis has too.
    """
    counts = part.vertex_bone_counts()
    if not counts:
        return None
    tally = {}
    for name, n in counts.items():
        shared = shared_bone(part, name, chassis_bones)
        if shared is None:
            continue
        tally[shared] = tally.get(shared, 0) + n
    if not tally:
        return None
    bone = max(sorted(tally), key=lambda b: tally[b])
    part_bones = part.bones()
    m = chassis_bones[bone] @ np.linalg.inv(part_bones[bone])
    return {
        "bone": bone,
        "translation": [float(v) for v in m[:3, 3]],
        "rotation": matrix_quat(m),
    }


def shared_bone(part, name, chassis_bones):
    """`name` if the chassis skeleton has it, else its nearest ancestor in the part's tree that it has."""
    index_of = {part.names[i]: i for i in range(len(part.names))}
    index = index_of.get(name)
    while index is not None:
        if part.names[index] in chassis_bones:
            return part.names[index]
        index = part.parent.get(index)
    return None
