"""Static meshes of a track as glTF (the exporter's `export` mode) read into triangle groups in the game's render space.

The glTF of CUE4Parse swaps y and z of Unreal's centimetres and scales to metres: `glb = (x, z, y) / 100`. The game's render
space is `(x, -y, z)` of Unreal in metres, so `render = (gx, -gz, gy)`, a turn of 90 degrees about x that keeps the winding.
"""

import json
import struct

import common  # noqa: F401  (first: it makes the user site-packages importable)
import numpy as np

COMPONENT = {5126: np.float32, 5123: np.uint16, 5125: np.uint32, 5121: np.uint8}
WIDTH = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}


class Glb:
    """A binary glTF: its JSON and its one buffer."""

    def __init__(self, path):
        data = open(path, "rb").read()
        size = struct.unpack_from("<I", data, 12)[0]
        self.doc = json.loads(data[20 : 20 + size])
        self.buffer = data[20 + size + 8 :]

    def accessor(self, index):
        a = self.doc["accessors"][index]
        view = self.doc["bufferViews"][a["bufferView"]]
        kind, width = COMPONENT[a["componentType"]], WIDTH[a["type"]]
        item = np.dtype(kind).itemsize
        stride = view.get("byteStride") or width * item
        offset = view.get("byteOffset", 0) + a.get("byteOffset", 0)
        array = np.ndarray((a["count"], width), dtype=kind, buffer=self.buffer, offset=offset, strides=(stride, item))
        return np.array(array)

    def slots(self):
        """The primitives: `[(material name, attributes dict, indices)]` in slot order."""
        out = []
        for primitive in self.doc["meshes"][0]["primitives"]:
            attrs = {name: self.accessor(i) for name, i in primitive["attributes"].items() if name in WANTED}
            indices = self.accessor(primitive["indices"]).reshape(-1)
            out.append((self.doc["materials"][primitive["material"]]["name"], attrs, indices.astype(np.uint32)))
        return out


WANTED = ("POSITION", "NORMAL", "COLOR_0", "TEXCOORD_0", "TEXCOORD_1")


def to_render(array):
    """glTF axes to the render space, for positions and normals alike."""
    out = np.empty_like(array, dtype=np.float32)
    out[:, 0] = array[:, 0]
    out[:, 1] = -array[:, 2]
    out[:, 2] = array[:, 1]
    return out


def unit_normals(array):
    normals = to_render(array)
    length = np.linalg.norm(normals, axis=1, keepdims=True)
    return normals / np.maximum(length, 1e-9)


def layer_of_triangles(colours, indices, present):
    """The dominant layer of every triangle of a mesh part, as an array (index of the triangle)."""
    triangles = indices.reshape(-1, 3)
    weights = colours[triangles][:, :, :3].mean(axis=1)
    layered = np.concatenate([weights, np.clip(1.0 - weights.sum(axis=1, keepdims=True), 0.0, 1.0)], axis=1)
    for layer in (1, 2, 3, 4):
        if layer not in present:
            layered[:, layer - 1] = -1.0
    return layered.argmax(axis=1) + 1


def part(attrs, indices, triangle_mask):
    """The vertices and indices of the triangles of a mask, compacted: `(positions, normals, uv0, colours, indices)`."""
    triangles = indices.reshape(-1, 3)[triangle_mask]
    used, inverse = np.unique(triangles.reshape(-1), return_inverse=True)
    n = attrs["NORMAL"][used] if "NORMAL" in attrs else None
    return (
        to_render(attrs["POSITION"][used]),
        unit_normals(n) if n is not None else None,
        attrs["TEXCOORD_0"][used].astype(np.float32),
        inverse.reshape(-1).astype(np.uint32),
    )


def vertex_colours(attrs):
    """The vertex colours as floats 0 to 1, or ones when the mesh has none."""
    if "COLOR_0" not in attrs:
        return np.ones((len(attrs["POSITION"]), 4), dtype=np.float32)
    colour = attrs["COLOR_0"]
    return colour.astype(np.float32) / (255.0 if colour.dtype == np.uint8 else 1.0)


def planar_uv(positions, normals, tile_m):
    """Texture coordinates that tile every `tile_m` metres over the ground, projected along the axis the normal points most to.

    A flat piece of ground is projected from above (x and y of the render space); a cliff or a wall from the side (y and z, or
    x and z), so a steep face is not a smear of the picture's edge. A vertex takes the projection of its own normal.
    """
    ax, ay, az = (np.abs(normals[:, k]) for k in range(3))
    top = (az >= ax) & (az >= ay)
    side_x = ~top & (ax >= ay)
    u = np.where(top, positions[:, 0], np.where(side_x, positions[:, 1], positions[:, 0]))
    v = np.where(top, positions[:, 1], positions[:, 2])
    return np.stack([u / tile_m, v / tile_m], axis=1).astype(np.float32)
