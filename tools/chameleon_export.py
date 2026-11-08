#!/usr/bin/env python3
"""glTF (.glb) exporter of Fairhaven units, a throw-away prototype for docs/plans/tracks-and-fairhaven-feasibility.md.

Reads TRK_UNIT bundles (HAWAII/), places every model of the unit's InstanceList (LOD 0), and writes one glb with a
diffuse-textured primitive per texture. Textures are shrunk (largest side `MAX_TEXTURE`) to keep the file small.
Needs numpy and Pillow (BC1/BC3/BC7 .dds decoding). Imported by chameleon_world.py; nothing is stored in the repo.
"""

import io
import json
import os
import re
import struct
import sys
from collections import Counter

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from chameleon_bundle import Bundle  # noqa: E402
from chameleon_world import (T_MATERIAL, T_MODEL, T_RENDERABLE, T_SHADER, T_TEXTURE, T_VDESC,  # noqa: E402
                             T_INSTANCES, parse_instance_list, u32)

MAX_TEXTURE = 256
GLOBAL_SAMPLERS = {"EnvironmentSpecularTextureSampler", "cubeReflectionSampler", "TonemapSampler",
                   "AmbientSpecSampler", "cubeIrradianceSampler"}
DIFFUSE_NAMES = ("DiffuseTextureSampler", "ColourMap_Sampler", "Tiling1TextureSampler", "Decal_Diffuse_Sampler",
                 "Line_Diffuse_Sampler", "DetailMap_Diffuse_Sampler")
DXGI_BLOCK = {71: 8, 72: 8, 74: 16, 75: 16, 77: 16, 78: 16, 80: 8, 81: 8, 83: 16, 84: 16, 98: 16, 99: 16}


def half_to_float(a):
    return a.astype(np.uint16).view(np.float16).astype(np.float32)


class Shaders:
    """The shader bundle: shader id -> (technique name, material sampler names, vertex descriptor elements)."""

    def __init__(self, game):
        self.table = {}
        b = Bundle(os.path.join(game, "SHADERS.BNDL"))
        by_id = {e.id: e for e in b.entries}
        for e in b.by_type(T_SHADER):
            main = b.block(e, 0)
            names = [(m.start(), m.group().decode()) for m in re.finditer(rb"[A-Za-z][A-Za-z0-9_]{4,}(?=\x00)", main)]
            tech = [n for _, n in names if "_" in n and n[0].isupper()]
            at = {o: n for o, n in names if n.endswith("Sampler")}
            runs = [[]]
            for i in range(0, len(main) - 3, 4):
                v = struct.unpack_from("<I", main, i)[0]
                if v in at:
                    runs[-1].append(at[v])
                elif runs[-1]:
                    runs.append([])
            samplers = next((r for r in runs if r and not (set(r) & GLOBAL_SAMPLERS)), [])
            elements = None
            for rid, _, _ in b.imports(e, main):
                de = by_id.get(rid)
                if de is not None and de.type == T_VDESC:
                    vd = b.block(de, 0)
                    n = u32(vd, 0xC)
                    elements = [struct.unpack_from("<5I", vd, 0x10 + 0x14 * k) for k in range(n)]
                    break
            self.table[e.id] = (tech[-1] if tech else "?", samplers, elements)


class SharedIndex:
    """Table-only index of the bundles a unit may borrow from: id -> path of the bundle that holds it."""

    def __init__(self, game):
        import glob
        self.where = {}
        paths = [os.path.join(game, "GLOBALTEXTUREDICTIONARY.BNDL"), os.path.join(game, "GLOBALMATERIALDICTIONARY.BNDL"),
                 os.path.join(game, "HAWAII", "GLOBALRESOURCES.BNDL")]
        paths += sorted(glob.glob(os.path.join(game, "HAWAII", "DISTRICT_*.BNDL")))
        for p in paths:
            b = Bundle(p)
            for e in b.entries:
                self.where.setdefault(e.id, p)
        self.loaded = {}

    def get(self, rid):
        p = self.where.get(rid)
        if p is None:
            return None
        if p not in self.loaded:
            if len(self.loaded) >= 3:
                self.loaded.pop(next(iter(self.loaded)))
            b = Bundle(p)
            self.loaded[p] = (b, {e.id: e for e in b.entries})
        b, ix = self.loaded[p]
        return b, ix[rid]


class Resolver:
    """Finds a resource by id: the unit's own bundle first, then the shared bundles."""

    def __init__(self, shared, unit_bundle):
        self.unit = unit_bundle
        self.index = {e.id: e for e in unit_bundle.entries}
        self.shared = shared
        self.missing = Counter()

    def get(self, rid):
        e = self.index.get(rid)
        if e is not None:
            return self.unit, e
        found = self.shared.get(rid)
        if found is None:
            self.missing[rid] += 1
        return found


def decode_texture(b, e):
    """Texture resource -> PIL image at most MAX_TEXTURE on a side (a lower mip is taken, nothing is resampled)."""
    main, gfx = b.block(e, 0), b.block(e, 1)
    fmt = u32(main, 0x1C)
    w, h = struct.unpack_from("<HH", main, 0x24)
    mips = main[0x2D]
    block = DXGI_BLOCK.get(fmt)
    if block is None or e.type != T_TEXTURE:
        return None
    offset, level = 0, 0
    cw, ch = w, h
    while max(cw, ch) > MAX_TEXTURE and level + 1 < mips:
        offset += max(1, (cw + 3) // 4) * max(1, (ch + 3) // 4) * block
        cw, ch, level = max(1, cw // 2), max(1, ch // 2), level + 1
    size = max(1, (cw + 3) // 4) * max(1, (ch + 3) // 4) * block
    pixels = gfx[offset:offset + size]
    if len(pixels) < size:
        return None
    header = bytearray(128 + 20)
    header[0:4] = b"DDS "
    struct.pack_into("<7I", header, 4, 124, 0x1007 | 0x80000, ch, cw, size, 0, 1)
    struct.pack_into("<2I", header, 76, 32, 4)
    header[84:88] = b"DX10"
    struct.pack_into("<I", header, 108, 0x1000)
    struct.pack_into("<5I", header, 128, fmt, 3, 0, 1, 0)
    try:
        img = Image.open(io.BytesIO(bytes(header) + pixels))
        img.load()
        return img.convert("RGBA")
    except Exception:
        return None


def diffuse_texture(res, shaders, mat_main, mat_imports):
    """Resource id of the diffuse texture of a material, by the shader's sampler order."""
    shader_id = next((r for r, o, _ in mat_imports if o == 8), None)
    info = shaders.table.get(shader_id)
    count = u32(mat_main, 0x20)
    table = u32(mat_main, 0x2C)
    slots = {o: r for r, o, _ in mat_imports}
    tex = [slots.get(table + 4 * n) for n in range(count)]
    if info is None:
        return None, "?"
    for name in DIFFUSE_NAMES:
        if name in info[1] and info[1].index(name) < len(tex):
            return tex[info[1].index(name)], info[0]
    return (tex[0] if tex else None), info[0]


class Mesh:
    def __init__(self):
        self.pos = []
        self.uv = []
        self.idx = []


def strip_to_list(indices):
    out = []
    run = []
    for i in indices:
        if i == 0xFFFF:
            run = []
            continue
        run.append(i)
        if len(run) < 3:
            continue
        a, b, c = run[-3], run[-2], run[-1]
        if a == b or b == c or a == c:
            continue
        out.append((b, a, c) if len(run) % 2 == 0 else (a, b, c))
    return out


def load_renderable(res, shaders, rid, cache):
    """Renderable -> list of (diffuse texture id, technique, positions (n,3), uvs (n,2), triangles (m,3))."""
    if rid in cache:
        return cache[rid]
    found = res.get(rid)
    out = []
    if found is None:
        cache[rid] = out
        return out
    b, e = found
    main, gfx = b.block(e, 0), b.block(e, 1)
    imports = b.imports(e, main)
    slots = {o: r for r, o, _ in imports}
    cache[("sphere", rid)] = struct.unpack_from("<4f", main, 0)
    count, table = struct.unpack_from("<HI", main, 0x12)[0], u32(main, 0x14)
    for k in range(count):
        mo = u32(main, table + 4 * k)
        prim, icount = u32(main, mo + 0x10), u32(main, mo + 0x1C)
        iinfo, vinfo = u32(main, mo + 0x28), u32(main, mo + 0x2C)
        mat = res.get(slots.get(mo + 0x20))
        if mat is None:
            continue
        mb, me = mat
        mmain = mb.block(me, 0)
        tex_id, tech = diffuse_texture(res, shaders, mmain, mb.imports(me, mmain))
        shader_id = next((r for r, o, _ in mb.imports(me, mmain) if o == 8), None)
        info = shaders.table.get(shader_id)
        if not info or not info[2]:
            continue
        elements = info[2]
        stride = elements[0][4]
        pos_el = next((x for x in elements if (x[0] & 0xFF) == 1 and (x[0] >> 8) == 0), None)
        uv_el = next((x for x in elements if (x[0] & 0xFF) == 5 and (x[0] >> 8) == 0), None)
        if pos_el is None or pos_el[1] != 6:
            continue
        voff, vsize = u32(main, vinfo + 0xC), u32(main, vinfo + 0x10)
        ioff, istride = u32(main, iinfo + 0xC), u32(main, iinfo + 0x14)
        if istride != 2:
            continue
        nv = vsize // stride
        raw = np.frombuffer(gfx, dtype=np.uint8, count=nv * stride, offset=voff).reshape(nv, stride)
        pos = raw[:, pos_el[2]:pos_el[2] + 12].copy().view("<f4").reshape(nv, 3)
        if uv_el is not None and uv_el[1] == 34:
            uv = half_to_float(raw[:, uv_el[2]:uv_el[2] + 4].copy().view("<u2").reshape(nv, 2))
        else:
            uv = np.zeros((nv, 2), np.float32)
        indices = np.frombuffer(gfx, dtype="<u2", count=icount, offset=ioff).tolist()
        tris = strip_to_list(indices) if prim == 5 else [tuple(indices[i:i + 3]) for i in range(0, icount - 2, 3)]
        if not tris:
            continue
        out.append((tex_id, tech, pos, uv, np.array(tris, np.uint32)))
    cache[rid] = out
    return out


def export_glb(world, units, out_path, max_instances=None, clip=True):
    shaders = Shaders(world.game)
    shared = SharedIndex(world.game)
    prims = {}   # texture key -> lists
    textures = {}  # texture id -> png bytes or None
    stats = Counter()
    from chameleon_census import read_pvs
    pvs = read_pvs(world.game) if clip else {}
    for n in units:
        rect = pvs.get(n)
        ub = Bundle(world.unit_path(n))
        res = Resolver(shared, ub)
        cache = {}
        for e in ub.by_type(T_INSTANCES):
            insts = parse_instance_list(ub, e)
            if max_instances:
                insts = insts[:max_instances]
            for inst in insts:
                found = res.get(inst.model)
                if found is None:
                    stats["model_missing"] += 1
                    continue
                mb, me = found
                mmain = mb.block(me, 0)
                mimports = mb.imports(me, mmain)
                lod0 = next((r for r, o, _ in mimports if o == u32(mmain, 0)), None)
                if lod0 is None:
                    continue
                m4 = np.array(inst.matrix, np.float64).reshape(4, 4)
                parts = load_renderable(res, shaders, lod0, cache)
                sphere = cache.get(("sphere", lod0))
                if rect and sphere:
                    c = np.array([sphere[0], sphere[1], sphere[2], 1.0]) @ m4
                    margin = 25.0
                    if not (rect["x"][0] - margin <= c[0] <= rect["x"][1] + margin and rect["z"][0] - margin <= c[2] <= rect["z"][1] + margin) or sphere[3] > 800:
                        stats["clipped"] += 1
                        continue
                stats["instances"] += 1
                for tex_id, tech, pos, uv, tris in parts:
                    world_pos = (np.c_[pos.astype(np.float64), np.ones(len(pos))] @ m4)[:, :3].astype(np.float32)
                    key = (n, tex_id)
                    if key not in textures:
                        t = res.get(tex_id) if tex_id else None
                        img = decode_texture(*t) if t else None
                        textures[key] = img
                        stats["textures" if img is not None else "textures_missing"] += 1
                    bucket = prims.setdefault(key, [[], [], [], 0])
                    bucket[0].append(world_pos)
                    bucket[1].append(uv)
                    bucket[2].append(tris + bucket[3])
                    bucket[3] += len(pos)
                    stats["triangles"] += len(tris)
        stats["unresolved"] += sum(res.missing.values())
    write_glb(prims, textures, out_path)
    print(dict(stats))


def write_glb(prims, textures, out_path):
    bin_data = bytearray()
    views, accessors, images, gtex, materials, primitives = [], [], [], [], [], []

    def add_view(data, target=None):
        while len(bin_data) % 4:
            bin_data.append(0)
        views.append({"buffer": 0, "byteOffset": len(bin_data), "byteLength": len(data), **({"target": target} if target else {})})
        bin_data.extend(data)
        return len(views) - 1

    def add_acc(view, ctype, count, typ, **extra):
        accessors.append({"bufferView": view, "componentType": ctype, "count": count, "type": typ, **extra})
        return len(accessors) - 1

    mat_of = {}
    for key, (ps, uvs, idxs, _) in prims.items():
        pos = np.concatenate(ps).astype("<f4")
        uv = np.concatenate(uvs).astype("<f4")
        idx = np.concatenate(idxs).astype("<u4").reshape(-1)
        img = textures.get(key)
        if key not in mat_of:
            m = {"pbrMetallicRoughness": {"metallicFactor": 0.0, "roughnessFactor": 1.0}, "doubleSided": True}
            if img is not None:
                buf = io.BytesIO()
                img.convert("RGB").save(buf, "PNG", optimize=False)
                images.append({"bufferView": add_view(buf.getvalue()), "mimeType": "image/png"})
                gtex.append({"source": len(images) - 1})
                m["pbrMetallicRoughness"]["baseColorTexture"] = {"index": len(gtex) - 1}
            materials.append(m)
            mat_of[key] = len(materials) - 1
        pv = add_view(pos.tobytes(), 34962)
        uvv = add_view(uv.tobytes(), 34962)
        iv = add_view(idx.tobytes(), 34963)
        primitives.append({
            "attributes": {
                "POSITION": add_acc(pv, 5126, len(pos), "VEC3", min=pos.min(0).tolist(), max=pos.max(0).tolist()),
                "TEXCOORD_0": add_acc(uvv, 5126, len(uv), "VEC2")},
            "indices": add_acc(iv, 5125, len(idx), "SCALAR"),
            "material": mat_of[key]})
    doc = {"asset": {"version": "2.0", "generator": "chameleon_export.py"},
           "scene": 0, "scenes": [{"nodes": [0]}], "nodes": [{"mesh": 0}], "meshes": [{"primitives": primitives}],
           "materials": materials, "accessors": accessors, "bufferViews": views,
           "buffers": [{"byteLength": len(bin_data)}]}
    if images:
        doc["images"], doc["textures"] = images, gtex
    js = json.dumps(doc, separators=(",", ":")).encode()
    js += b" " * (-len(js) % 4)
    bin_data.extend(b"\0" * (-len(bin_data) % 4))
    total = 12 + 8 + len(js) + 8 + len(bin_data)
    with open(out_path, "wb") as f:
        f.write(struct.pack("<4sII", b"glTF", 2, total))
        f.write(struct.pack("<I4s", len(js), b"JSON") + js)
        f.write(struct.pack("<I4s", len(bin_data), b"BIN\0") + bytes(bin_data))
    print("wrote", out_path, total, "bytes,", len(primitives), "primitives")
