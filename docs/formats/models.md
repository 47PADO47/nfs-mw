# Models (geometry / "solids")

The engine calls a 3D model a **solid** (`eSolid` in the decomp, `src/Speed/Indep/Src/Ecstasy/eSolid.cpp`).
Solids come in packs, and the same chunk layout is used everywhere: cars, world scenery, NIS characters
and frontend models. For the tag meanings, see [evidence tags](../README.md#evidence-tags).

## Where models live

| File | Contents |
|---|---|
| `CARS/<CAR>/GEOMETRY.BIN` | One `GeometryPack` with every part of the car (body kits, wheels, brakes, decals, LODs) |
| `TRACKS/STREAML2RA.BUN` | 3,305 `GeometryPack`s, 20,377 solids for the city, split into streaming sections (see [maps.md](maps.md)) |
| `NIS/*_BundleB.bun` | Character and prop models for cutscenes (e.g. `FCop01.BIN`, `Perp01.BIN`, `Cuffs01.BIN`) |
| `FRONTEND/**`, `GLOBAL/InGameA.bun`, `GLOBAL/GlobalB.lzc` | Frontend and shared in-game models |

## Chunk tree **[verified]**

```
80134000 GeometryPack
├─ 80134001 MeshContainerInfo
│  ├─ 00134002 MeshContainerHeader (SolidListInfo)  pack name / source path, e.g. "GEOMETRY.BIN"
│  ├─ 00134003 MeshContainerKeys                    8 B per solid: (name hash, 0), sorted by hash
│  ├─ 00134004 MeshContainerOffsets                 24 B per solid: (hash, file offset, size, size, 0, 0)
│  └─ 80134008 MeshContainerEmpty
└─ 80134010 SolidPack                               one per solid
   ├─ 00134011 SolidInfo (SolidListObjHead)         header + name
   ├─ 00134012 SolidTextures                        texture name hashes (bStringHash) used by this solid
   ├─ 00134013 SolidShaders (LightMaterials)        light material hashes
   ├─ 00134017/18/19 MeshNormalSmoother / SmoothVertices / SmoothVertexPlats   (some car parts)
   ├─ 0013401A SolidMarkers                         position markers (some solids)
   └─ 80134100 MeshInfoContainer
      ├─ 00134900 MeshInfoHeader (SolidMeshDescriptor)
      ├─ 00134B02 MeshShaderInfos (ShadingGroups)   one 104-byte record per material
      ├─ 00134B03 MeshPolygons                      u16 triangle-list indices
      ├─ 00134B01 MeshVertexBuffer                  one or more vertex streams
      └─ 00134C02 MeshVltMaterials ...              optional per-material name strings
```

The decomp's `SpeedChunks.hpp` calls this family `SPEED_ESOLID_*` **[decomp]**. `00134900` is
`SPEED_ESOLID_PC_PLATINFO`, and the PC mesh chunks `00134B01`–`00134B03` reuse the IDs it names
`SPEED_ESOLID_XBOX_VERTEX_DATA` / `…_MESH_ENTRY_TABLE` / `…_MESH_ENTRY_DATA`. The display names above
are kept in [`tools/bchunk_names.py`](../../tools/bchunk_names.py), with the decomp identifiers as comments.

The BMW M3 GTR (`CARS/BMWM3GTR/GEOMETRY.BIN`, 1.6 MB) has 97 solids and 1,272 chunks in total. Vertex
buffers are 71% of the bytes (1.15 MB).

Checked on that file: the 97 `MeshContainerKeys` hashes are exactly the 97 `SolidInfo` hashes, and every
`MeshContainerOffsets` file offset points at a `SolidPack` chunk. In uncompressed files the two size
fields are equal; for compressed (JDLZ) solids they presumably hold the compressed and decompressed
sizes. **[unconfirmed]**

`SolidTextures` hashes: 131 of 254 on the M3 GTR resolve to texture keys in the car's own
`TEXTURES.BIN`/`VINYLS.BIN` plus `GLOBALB.BUN` and `InGameA.bun`. The rest presumably live in other
shared texture packs. **[partly verified]**

## SolidInfo (`0x00134011`)

The payload is aligned to 0x10. The layout below is **[community]** (NFS-ModTools
`MostWantedSolidReader.cs`); the fields marked ✔ were **[verified]** on 20,377 world solids and 97 car
solids.

| Offset | Type | Field |
|---|---|---|
| 0x00 | u8[12] | zero |
| 0x0C | u8 | **Version = 0x16** ✔ |
| 0x0D | u8 | EndianSwapped |
| 0x0E | u16 | Flags |
| 0x10 | u32 | **Hash = bStringHash(name)** ✔ |
| 0x14 | u16 | NumPolys |
| 0x16 | u16 | NumVerts (0 in car files; use the shading groups instead) ✔ |
| 0x18 | u8 ×4 | NumBones, NumTextureTableEntries, NumLightMaterials, NumPositionMarkers |
| 0x20 | vec3 + pad | BoundsMin |
| 0x30 | vec3 + pad | BoundsMax |
| 0x40 | float[16] | Transform (pivot matrix) |
| 0x80 | … | unknowns |
| 0xA0 | char[] | **Name**, NUL-terminated ✔ |

### Name hashing

`bStringHash` is the engine-wide name hash. Solids, textures and scenery references are all linked
through it.

```python
def bStringHash(s):
    h = 0xFFFFFFFF
    for c in s.encode():
        h = (h * 33 + c) & 0xFFFFFFFF
    return h
```

## Shading groups (`0x00134B02`) **[community + verified]**

Each record is 104 bytes, one per material within the solid. 104-byte records divide the payload
exactly for every solid of the M3 GTR.

| Offset | Field |
|---|---|
| 0x00 | BoundsMin (vec3), BoundsMax (vec3) |
| 0x18 | u8 texture slot indices: Diffuse, Normal, Height, Specular, Opacity (index into `SolidTextures`) |
| 0x1D | u8 LightMaterialNumber |
| 0x30 | u32 EffectId (shader: WorldShader, CarShader, GlossyWindow, WorldBoneShader, …; the compiled effects are in `speed.exe`, see [shaders.md](shaders.md)) |
| 0x38 | u32 Flags |
| 0x3C | u32 NumVerts |
| 0x40 | u32 NumTris |
| 0x44 | u32 **FirstIndex**: where this group's indices start in `MeshPolygons` ✔ |
| 0x5C | u32 NumIndices (= NumTris × 3 on the M3 GTR) ✔ |

✔ On the M3 GTR the groups' `FirstIndex` values are 0, 48, 216, … with no gaps, and each group's
indices fall in a contiguous vertex range that follows the previous group's. **[verified]**

## Vertices (`0x00134B01`)

The payload is aligned to 0x80. The format depends on the shading group's effect:

| Effect | Layout | Stride |
|---|---|---|
| CarShader, WorldShader, GlossyWindow, billboard | pos f32×3, normal f32×3, color u32 (D3DCOLOR), uv f32×2 | **36** ✔ |
| WorldBoneShader (skinned; effect id 2) | as above + blend weights f32×3 + blend indices f32×3 | **60** ✔ (NIS characters) |
| WorldNormalMap, WorldReflectShader | as above + extra texcoord 8 B + tangent f32×4 | 60 |

✔ For all 97 M3 GTR solids, the sum of `NumVerts × 36` equals the vertex buffer size exactly.

### Several vertex buffers per solid **[verified]**

A solid has **one `MeshVertexBuffer` chunk per run of consecutive shading groups with the same effect id**.
The groups of a run share that buffer in order, so group *k* of a run starts after the vertices of groups
0…k−1. Each buffer's stride follows from its size and its run's vertex count.

Checked on every solid of the install: all 15,781 car solids have one run (one effect) and one buffer. Of
the 20,377 world solids, 15,102 have one buffer and the rest have 2–10, and the rule holds for all of them.
Strides seen by effect id:

| Effect id | Stride | Runs |
|---|---|---|
| 0 | 36 | 16,515 |
| 1 | 60 | 3,868 |
| 3 | 60 | 6,340 |
| 5 | 36 | 2,474 |
| 6 | 36 | 1,006 |
| 19 | 44 | 1 (`SKYDOME_XENON`: the common 36 bytes + a second UV pair) |

## Indices (`0x00134B03`)

`u16` triangle lists, aligned to 0x10. Each shading group draws `NumIndices` indices starting at its
`FirstIndex`. The indices are relative to the start of the group's **vertex buffer** (above), not to the
group. With a single buffer they are absolute.
**[verified]**: every car (100 cars, 15,781 solids) and every world solid (20,377) has its group indices in
range once each group's base vertex is applied (`crates/nfsmw-data/tests/real_install/`).

## Naming conventions **[verified]**

Car solids are named `<CAR>_<PART>_<LOD>`, for example `BMWM3GTR_BASE_A` … `BMWM3GTR_BASE_D`,
`BMWM3GTR_KIT00_FRONT_BRAKE_A`. Polygon counts fall from `_A` (354) to `_D` (92), so `A`–`D` are
**levels of detail**.

## Compressed geometry (add-on cars)

Add-on cars in this install (COROLLAE88, FXXEVO, LEVIN, SF90, SKYLINEZT, TRUENO, TRUENOCP, TRUENOID)
were built with *NFS-CarToolkit by nfsu360* and store every `SolidPack` as a bare JDLZ blob. See
[bchunk.md § 4](bchunk.md#4-bare-jdlz-blobs-between-chunks); view them with
`chunkdump.py --inflate`.

## Reading models today

- **This project:** [`libs/blackbox-solid`](../../libs/blackbox-solid) (Rust) reads solids, including
  the compressed add-on ones, and `nfsmw view-car <CAR>` draws them.
- [NFS-ModTools](https://github.com/NFSTools/NFS-ModTools) (C#, **no license**: read-only reference) has a
  MW reader (`Common/Geometry/MostWantedSolidReader.cs`) and exports to FBX with `AssetDumper`.
