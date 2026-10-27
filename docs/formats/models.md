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
   ├─ 0013401A SolidMarkers                         position markers (some solids; layout in cardata.md)
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
| 0x0E | u16 | Flags; bit `0x80` = the solid sways in the wind ([scenery-sway.md](../specs/scenery-sway.md)) |
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

**Version 0x18 [verified]:** the front-end stages (`FRONTEND/PLATFORMS/PlatformCrib.BIN`, five solids) use the
same record with the version byte at 0x0C set to `0x18`. Checked on the crib: the name hash equals `bStringHash` of
the name, the name is NUL-terminated at 0xA0, and the counts and bounds read sane. The rewrite reads it with the
0x16 offsets (`libs/blackbox-solid/src/layout/v18.rs`).

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

## Flat geometry (re-exported stock cars)

A second community layout was **[verified]** on a Desktop install (`CORVETTE`, `COBALTSS`, `TT`, `BMWM3`; 283, 151, 132
and 282 solids). The file starts with a small `GeometryPack` (only `MeshContainerInfo`, about 9 KB) and an empty
`MeshContainerEmpty`, then every `SolidPack` follows as a **top-level sibling**, uncompressed. A reader that looks only
inside `GeometryPack`s finds **no solids**, so the car assembles with no body and no wheels (the Blacklist's Baron
drives `CORVETTE`). `blackbox-solid` reads top-level `SolidPack`s as well.

Eight of the `CORVETTE`'s solids (aftermarket hoods) do not parse: their vertex buffer is 3,564 bytes whatever vertex
count the groups state. `read_solids_lossy` leaves such a solid out and returns its error, so the car still builds
without it (`nfsmw_data::car` logs a warning). **[ours]**: the cause of the broken hoods was not investigated.

## Reading models today

- **This project:** [`libs/blackbox-solid`](../../libs/blackbox-solid) (Rust) reads solids, including
  the compressed and the flat add-on ones, and `nfsmw view-car <CAR>` draws them. It also writes them
  ([below](#writing-geometrybin-verified)).
- [NFS-ModTools](https://github.com/NFSTools/NFS-ModTools) (C#, **no license**: read-only reference) has a
  MW reader (`Common/Geometry/MostWantedSolidReader.cs`) and exports to FBX with `AssetDumper`.

## Writing `GEOMETRY.BIN` **[verified]**

[`blackbox_solid::write_geometry_pack`](../../libs/blackbox-solid/src/writer/mod.rs) builds a car `GEOMETRY.BIN` from
vertex and index lists. Checked on 100 car files (15,624 solids): every solid it writes reads back as the same
`Solid`, and for four cars (BMWM3GTR, 911GT2, CAMARO, COPSUV) the payloads of `SolidInfo`, the texture and light lists,
the markers and the whole mesh container are byte-identical to the stock chunks, `0x11` fill included
(`libs/blackbox-solid/src/tests/writer_install.rs`, run with `NFSMW_GAME_DIR`). The layout facts it relies on, beyond
the tables above:

- **Chunk placement.** `GeometryPack` holds an empty padding chunk (id 0, size 0, so the next header sits at 0x10),
  `MeshContainerInfo`, then the `SolidPack`s, each header on a 0x80 boundary, with zero-filled padding chunks between
  them. Inside a `SolidPack` the children follow each other with no padding chunks; only the payloads are left-padded
  with `0x11` to their alignment: `SolidInfo`, markers, `MeshInfoHeader`, shading groups and indices 0x10, the vertex
  buffer 0x80, texture and light lists and material names none.
- **`MeshContainerHeader`** is 144 bytes: u32 29 at 0x08 (in every file), the solid count at 0x0C, `GEOMETRY.BIN` at
  0x10, `DEFAULT` at 0x48, the file length at 0x68, u32 0x80 at 0x70, the rest zero. The keys and the offsets are in
  ascending name-hash order; an offsets entry is `(hash, offset of the SolidPack chunk header, chunk size + 8,
  chunk size + 8, 0, 0)`.
- **`SolidInfo`** is 0xA0 bytes plus the name with its NUL padded to a multiple of 4. NumVerts, NumBones and
  NumPositionMarkers are 0; NumTextureTableEntries and NumLightMaterials are the list lengths. Flags are 0x0040 on
  nearly every car solid. The u32s at 0x88 and 0x8C are 0x000EA550 in all 14,843 car solids with a mesh. At 0x98 is the
  volume of the bounds (a double-precision product rounded to f32), at 0x9C NumPolys divided by it.
- **Bounds.** Group and solid bounds are the box of the vertices made 0.01 larger on all six sides (true of the stock
  cars; add-on cars made with other tools differ). NumPolys is the triangle total, except for a few solids that say 1
  or 2 more.
- **Indices.** An odd index total is followed by one zero `u16`, so the chunk is a multiple of 4 bytes (1,707 of 5,192
  car solids checked); no group refers to it and the group records and `MeshInfoHeader` count only real indices.
- **`MeshInfoHeader`** (48 bytes): u32 18 at 0x08, the OR of the groups' flags at 0x0C, the group count at 0x10, the
  number of vertex buffers at 0x18 (checked on all 20,377 world solids), the index total at 0x2C.
- **Group records** are zero except the fields in the table above. Effect id 4 is on 40,523 of 40,545 car groups, 0 on
  the other 22. Their flags are one of a handful of values (0x14180, 0x4180, 0xA24180, ...); the meaning is not known.
- **Material names** (`00134C02`): one chunk per shading group, in order, NUL-terminated and padded to a multiple of 4
  (`BODY_CHROME`, `WINDOW_FRONT`, `DECAL1`, ...).

Not written: the vertex layouts other than the 36-byte one, and `00134017` to `00134019` (normal smoothing, on 137 of
the 414 solids compared). Whether the game accepts a custom file that leaves them out is **[unconfirmed]**.

## Skinned vertices and bones

This section covers the solids with effect id 2 (`WorldBoneShader`, 60-byte vertices). It was measured on the four
NIS bundles listed below. The skeleton side is in [eagl-animation-skeleton.md](../specs/eagl-animation-skeleton.md).
Offsets are inside the payload after any leading `0x11` fill, unless a row says otherwise. Every fact here is
**[verified]** on the install unless it is marked **[unconfirmed]**.

### Which bundles hold solids **[verified]**

| Bundle | Solids | Skeletons (`00E34009`) | Animation banks (`00E34010`) |
|---|---|---|---|
| `Scene_ArrestF02_BundleB.bun` | 3, all effect 2 | 3: `Bip21`, `Bip23` (48 bones each), `Bip24` (3 bones) | 2 |
| `Scene_IntroNis05_BundleB.bun` | 0 | 0 | 1 |
| `Scene_EndingNis04_BundleB.bun` | 0 | 0 | 1 |
| `Scene_IntroNisBL01_BundleB.bun` | 0 | 0 | 1 |

The three bundles with no solids parse completely: no error, no whole-file wrapper, and no `80134000` GeometryPack
anywhere in the tree. Their character models must come from another file. The survey did not find which one.

### The three skinned solids **[verified]**

| Solid | NumBones (0x18) | Shading groups | NumVerts | NumPolys (0x14) | Stride | Rig (see below) |
|---|---|---|---|---|---|---|
| `FCOP01` | 48 | 9 | 2,547 | 3,392 | 60 | Bip21 |
| `PERP01` | 48 | 5 | 2,071 | 3,288 | 60 | Bip23 |
| `CUFFS01` | 3 | 1 | 623 | 224 | 60 | Bip24 |

All groups have effect id 2. NumPolys equals the sum of the groups' triangle counts. The index buffer holds
10,176, 9,864 and 672 indices, which equals the sum of the groups' index counts.

### How the NIS solid payloads differ **[verified]**

- The SolidInfo payload and the shading-group payload start with 8 bytes of `0x11` fill. The shading groups are
  104-byte records after that fill, so a payload of n groups is 8 + 104 n bytes.
- The vertex buffer starts with a fill of 40, 56 and 72 bytes of `0x11` (FCOP01, PERP01, CUFFS01). The fill length
  is not fixed. After it come exactly NumVerts × 60 bytes, with no tail.
- The solid header transform (0x40, 16 floats, column-major) is the identity for CUFFS01. It is not the identity for
  FCOP01, where the rotation part is an axis permutation times 0.2438 and the translation is about (0, 0.03, 1.04),
  and for PERP01, a uniform 0.4315 scale with translation about (0, 0.02, 1.02). The bind check below shows that the
  vertices do not use it.
- `00134012` (texture hashes) and `00134013` (light materials) are lists of 8-byte entries: a u32 hash and a zero
  u32. FCOP01 has 2 and 5 entries, PERP01 2 and 4, CUFFS01 1 and 1.
- The `00134C02` strings are material names (`lambert56`, `08 - Default`, `01 - Default`, `Material #25`). They are
  not bone names.
- MeshInfoHeader (`00134900`), after its 0 or 8 bytes of fill: the u32 at 0x08 is 18 in all three solids; the u32 at
  0x0C is 0x4181 in all three; the u32 at 0x10 is the group count; the u32 at 0x2C is the total index count. The
  meaning of the 18 and of 0x4181 is not known. **[unconfirmed]**
- The child chunks of FCOP01's SolidPack are `00134011` (176 bytes), `00134012` (16), `00134013` (40) and
  `80134100` (174,444). Its MeshInfoContainer holds `00134900` (48), `00134B02` (944), `00134B03` (20,352),
  `00134B01` (152,860) and seven `00134C02` strings. **No chunk has the size of NumBones × 4 or NumBones × 64**
  (192 or 3,072 for FCOP01). So there is no separate palette chunk. The bone table is inside each shading group
  (next section).

### Blend weights **[verified]**

The vertex layout is the one in the Vertices table above. Weights are three f32 at bytes 36 to 48 of each 60-byte
vertex. Indices are three f32 at bytes 48 to 60.

- For every vertex, the three stored weights sum to 1 within 9e-8. So the implied fourth weight is always 0, and
  three influences are enough for every vertex in these solids.
- Influences per vertex, FCOP01: 1 for 1,314 vertices, 2 for 1,153, 3 for 80. PERP01: 1 for 1,342, 2 for 656, 3 for
  73. CUFFS01: 1 for all 623.
- The smallest non-zero weight is 0.0014 (FCOP01). Unused slots have weight 0 and index 0.

### Blend indices are palette slots, not bone numbers **[verified]**

- The index values are integers stored as floats. All 7,641 (FCOP01), 6,213 (PERP01) and 1,869 (CUFFS01) index
  floats are whole numbers.
- The range is 0 to 15 in FCOP01 and PERP01, and 0 to 2 in CUFFS01. Each group uses at most 16 slots.
- The values are not multiples of 3. Values 1, 2, 4 and 5 occur in both 16-slot solids, so there is no three-registers-per-bone
  pattern.
- A slot is an index into the bone table of the vertex's shading group. The table gives the bone. The index stays
  below NumBones (15 is below 48), but it is not a bone number.

### The bone table of each shading group **[verified]**

Each 104-byte shading group record holds its own table of 16 one-byte bone numbers at record bytes 0x1E to 0x2D.
Bytes 0x2E and 0x2F are zero. Slot s of a vertex in that group means table entry s, and the entry is the bone's
record number in the rig's skeleton object (root is 0). An unused entry is 0, the same value as bone 0, so only the
slots that the vertices reference show the real count.

Slots referenced by vertices, per group (vertex count, then number of slots used):

| Solid | Groups: vertices (slots used) |
|---|---|
| FCOP01 | 0: 698 (16); 1: 8 (4); 2: 146 (3); 3: 468 (16); 4: 69 (4); 5: 265 (8); 6: 422 (11); 7: 309 (8); 8: 162 (6) |
| PERP01 | 0: 474 (14); 1: 126 (4); 2: 451 (10); 3: 346 (14); 4: 674 (16) |
| CUFFS01 | 0: 623 (3) |

The tables name the bones of the rig, which confirms the mapping. In FCOP01 group 0 (698 vertices) the table is
Bip21 bones 11 and 23 to 37: Spine 3, then Head, both Brows, both Cheeks, Eyes, Jaw, the Lip corners, the lower and upper
lips, and both Noses. Group 8 (162 vertices) holds the left upper-arm twist, left forearm and hand, and the right
upper-arm twist, forearm and hand. Each solid uses 47 distinct bones of its rig. Bone 0 is the root.

The vertex indices (`00134B03`) are absolute in the one vertex buffer. In all three solids every group's indices fall
inside that group's own vertex range.

### Bind pose: vertices are in model space **[verified]**

The bind check uses the skeleton's world bind matrices, built as parent times T, R and S. They reproduce the stored
inverse bind matrices to 8.4e-7 (Bip21), 8.3e-7 (Bip23) and 9.4e-8 (Bip24). For each vertex, the "bind position" is the
weighted average of the bind joint positions of its palette bones. The check uses the vertices whose largest weight
is above 0.9 (FCOP01 1,449, PERP01 1,398, CUFFS01 623). Two distances are reported: to the joint, and to the segment
from that bone's joint to its first child joint. Distances are in metres.

| Solid, rig | Joint distance, median / p90 | Segment distance, median / p90 | Bone is the nearest of the rig's joints |
|---|---|---|---|
| FCOP01, Bip21 | 0.131 / 0.196 | 0.061 / 0.144 | 40% |
| FCOP01, Bip23 | 0.153 / 0.487 | 0.095 / 0.487 | 32% |
| PERP01, Bip23 | 0.140 / 0.304 | 0.074 / 0.127 | 33% |
| PERP01, Bip21 | 0.200 / 0.505 | 0.096 / 0.504 | 26% |
| CUFFS01, Bip24 | 0.076 / 0.118 | 0.037 / 0.111 | 76% (chance 33%) |

Controls, on the rig that fits best:

- The palette bone replaced by a fixed random permutation of the 48 bones: segment median 0.47 m (FCOP01) and 0.80 m
  (PERP01).
- The slot number used directly as the bone number: segment median 0.77 m (FCOP01) and 0.73 m (PERP01).
- No palette at all, every vertex compared with its nearest joint of the rig: median 0.066 m (FCOP01, Bip21) and
  0.072 m (PERP01, Bip23). This is the floor for any assignment.

Conclusion: the vertex positions are in the rig's frame at its bind pose, in metres, with z up. FCOP01 spans z from
-0.015 to 1.83 m, and the Bip21 root sits at z = 0.979 m. Applying the solid's own header transform to the positions
moves the fit to a median of 0.53 to 0.56 m (FCOP01) and 0.32 m (PERP01), so the positions are not stored in that
transform's space. Its role is not decided here. **[unconfirmed]**

### Which rig each solid uses **[verified fit; rule unconfirmed]**

- FCOP01 fits Bip21 better than Bip23 (segment p90 0.144 m against 0.487 m).
- PERP01 fits Bip23 better than Bip21 (segment p90 0.127 m against 0.504 m).
- CUFFS01 has 3 bones, so only the 3-bone rig Bip24 fits.

Bip21 and Bip23 have the same bone names in the same order (47 of 48 names match after the rig prefix, and the root
differs only by its prefix). Their bind joints differ by a median of 0.037 m and by up to 0.51 m, which is why the
fit separates them.

The solid names carry no rig name. NumBones only separates the 48-bone rigs from the 3-bone rig. Both animation banks
in the bundle have one symbol, `bank`, with no rig name. The skeleton note records the same header checksum (0x1658)
for Bip21 and Bip23 ([eagl-animation-skeleton.md](../specs/eagl-animation-skeleton.md), section 3.2), so that
checksum does not separate them either. (The skeleton note's own **[verified]** tag; not re-measured here.)

Which rig the game picks for each solid is **[unconfirmed]**. To settle it, look at the NIS scene script or the actor
table that names the model, or at the code that binds a solid to a skeleton.

### Open items

- The rule that picks the rig for each NIS solid (above).
- The role of the solid header transform at 0x40. It is not the identity for FCOP01 and PERP01, and the vertices do not
  use it.
- The meaning of the u32 values 18 and 0x4181 in MeshInfoHeader.
- The three bundles with no solids: where their characters come from.
