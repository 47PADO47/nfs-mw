# The Fairhaven world of NFS: Most Wanted (2012)

How the 2012 game stores its open city (internally `HAWAII`, in the game Fairhaven): the 169 streaming units, how a
unit places models, collision, navigation data, and what is shared. Container: [chameleon-bundle.md](chameleon-bundle.md);
meshes, materials, textures: [chameleon-gfx.md](chameleon-gfx.md); the cars' own collision soup (same chunk format):
[chameleon-collision.md](chameleon-collision.md). How this was learned: [provenance/chameleon-world.md](../provenance/chameleon-world.md).
What it would take to play in it: [plans/tracks-and-fairhaven-feasibility.md](../plans/tracks-and-fairhaven-feasibility.md).
Evidence tags: [README](../README.md#evidence-tags). Nothing here is read by a Rust crate yet; the readers are the Python
scripts `tools/chameleon_*.py` (standard library plus numpy and Pillow for pictures).

Everything below was measured on the 2012 install (v1.4.0.0) unless tagged. Coordinates are the engine's: **x left, y up,
z forward**, metres (a right-handed system).

**Read by Rust now:** `mods/fairhaven` reads units by byte ranges (`libs/chameleon-bundle`, `Bundle::parse_ranges`) and makes
each one a section file of the game ([docs/modding/fairhaven.md](../modding/fairhaven.md)): the zone list (`pvs.rs`), the instance
lists (`instances.rs`), models, renderables and materials (`convert.rs`, `mesh.rs`, `material.rs`), pictures (`textures.rs`) and
the collision soup (`collision.rs`). All 169 units convert with no error. The Python scripts stay as the proofs they were.

## What is where

| File (under `HAWAII/` unless said) | Count | Size | Holds |
|---|---|---|---|
| `TRK_UNIT<n>.BNDL` | 169 | 3.32 GB, median 18.8 MB, largest 45.6 MB | One streaming unit: models and textures it draws, instances, collision, navigation mesh, groundcover |
| `PVS.BNDL` | 1 | 12 KB | `ZoneList` (0x90): the 169 units with their rectangles and district ids |
| `DISTRICT_<id>.BNDL` | 7 | 19 KB to 51 MB, 227 MB in all | Models and textures shared by the units of a district (5 of them are named by the zone list) |
| `GLOBALRESOURCES.BNDL` | 1 | 97 MB | 27 models, 101 renderables, 261 textures every unit may use |
| `ARENA_<id>.BNDL` | 8 | 14 to 67 MB | The event arenas (own models, no instance lists) |
| `GAMELOGIC/ROADDATA.BNDL` | 1 | 49 KB | `RoadData` (0x203), a spatial tree of road points |
| `AI/AIDATA.BNDL` | 1 | 358 KB | `AiData` (0x200) |
| `TRAFFIC.BNDL` | 1 | 1.2 MB | `TrafficLaneData` (0x701) and 70 Genesys objects |
| `TRIGGERS.BNDL` | 1 | 281 KB | `TriggerData` (0x202) |
| `ENVIRONMENT.BNDL` | 1 | 5 MB | 13 colour cubes, 2 world paint maps, 27 textures, 220 Genesys objects (time of day, weather: not decoded) |
| `SURFACELIST.BNDL`, `SURFACESOUNDS.BNDL`, `AMBIENCES.BNDL`, `REVERBROADDATA.BNDL` | 4 | small | Surface kinds and their effects and sounds (see [chameleon-effects.md](chameleon-effects.md)), ambience |
| `MAPS/<id>.BNDL` | 86 | 33 KB each | One texture each (the map screen and minimap pieces) **[unconfirmed]** |
| `../WORLDLIST.BNDL` | 1 | 2 KB | Two Genesys objects |

The world needs `SHADERS.BNDL` (the shader and vertex-descriptor resources every material imports) and
`GLOBALTEXTUREDICTIONARY.BNDL` as well. There is **no single world file**: a unit is self-contained for most of what it
draws, and borrows the rest from the district and global bundles by resource id.

## Units

169 `TRK_UNIT<n>.BNDL`; `n` runs from 1 to 1390 with gaps (the numbers are not a grid). Which unit covers where comes from
`PVS.BNDL`, whose one resource is a `ZoneList` (0x90, 39,024 bytes):

| Offset | Type | Field |
|---|---|---|
| 0x00 | u32 x8 | header: 0x5990, **0x20** (offset of the entries), 0x9460, 0x9710, **169** (entry count), 0x3AD, 3, 0 **[verified]** the 3 words at 0x00, 0x08, 0x0C are offsets into the block; their meaning is not decoded |
| 0x20 + 0x50 n | entry | one per unit |

Entry (0x50 bytes) **[verified on all 169; each unit number occurs once and matches a file]**:

| Offset | Type | Field |
|---|---|---|
| +0x00 | u32 | offset (not decoded) |
| +0x10 | f32 x4 | rectangle minimum: x, `-FLT_MAX`, z, 0 |
| +0x20 | f32 x4 | rectangle maximum: x, `+FLT_MAX`, z, 0 (so the zone is a rectangle in the ground plane, unbounded in height) |
| +0x30 | u32, u32, u32 | offset (not decoded), **unit number**, **district id** |
| +0x40 | u8 x4 | small numbers (0 or 1, 0 to 6, 4 to 8); unconfirmed, possibly a grid cell and a count |

District ids and the units they hold: 164,408 (76 units, 1,614 MB), 701,574 (42 units, 815 MB), 701,575 (33 units, 728 MB),
2,021,698 (9 units, 164 MB), 152,518 (9 units, 1298 to 1390, empty: 1 instance, 994 triangles). The `DISTRICT_<id>.BNDL` of the
last has 52 KB and nothing in it that is drawn. The rectangles overlap (many units cover the same ground at different
detail) and together span **x -3,981 to 3,269 and z -5,826 to 5,498 m (7.25 by 11.3 km)**; the sum of their areas is
70 km2 because of the overlap. The census picture is `tools/chameleon_census.py` (a PNG of the rectangles by district with
the instance centres as dots; the roads show as lines of dots).

`DISTRICT_1024094` and `DISTRICT_905300` are not in the zone list; `ARENA_*` and these two are used by the event arenas
**[unconfirmed]**.

### What a unit holds **[verified]**

Resource counts of unit 1 (typical): 1 `InstanceList`, 1 `PropInstanceList`, 1 `DynamicInstanceList`, 1
`CompoundInstanceList`, 1 `LightInstanceList`, 19 `GroundcoverInstances` with 19 `GroundcoverCollection`, 1 `PolySoupList`, 1
`NavigationMesh`, 1 `ZoneHeader` (the root: its imports name the lists above), 51 `Model`, 140 `Renderable`, 106 `Material`,
166 `Texture`, 9 `SamplerState`, 6 `PropObject`, 7 `WorldObject`, 3 `CompoundObject`, 3 `Wave`, 85 `GenesysObject` and 60
`GenesysType`. The `ZoneHeader` (592 bytes) is a table of 24 imports (`id`, handle flag set, offset) and a few numbers.

Totals over the 169 units: 35,431 placed models (instances) of 9,192 distinct models (counting per unit), 12,290 props, 4,301
compound objects, 4,477 dynamic objects, 8,806,163 groundcover instances, 36,169 collision chunks, **49.4 million triangles at
level of detail 0**, 5.0 GB of graphics blocks of which 4.2 GB textures and 0.78 GB renderables, 254 MB of main blocks.

### Resources are duplicated between units **[verified]**

The bundles repeat whatever a unit needs, so the same resource (same 64-bit id) is stored in many unit files; 293 of 293 sampled
duplicates (textures, renderables and models) were byte for byte identical. Counted once by id across the 169 units:

| Type | All copies | Once by id | Share |
|---|---|---|---|
| Texture | 4,219 MB | 589 MB | 14 % |
| Renderable | 794 MB | 401 MB | 50 % |
| GroundcoverInstances | 141 MB | 22 MB | 15 % |
| PolySoupList | 34 MB | 34 MB | 100 % (unique) |
| NavigationMesh | 9 MB | 9 MB | 100 % |
| InstanceList | 5 MB | 5 MB | 100 % |
| Everything | 5.25 GB (inflated) | **1.07 GB** | 20 % |

So the city is about 1.07 GB of unique data (inflated), stored as 3.3 GB because of the repetition: streaming a unit costs its
own file only, and a loader can share resources by id.

## `InstanceList` (0x50): the placed models

Main block: a header of four u32, then records, then material parameter blocks, then the import table at the end of the block.

| Offset | Type | Field |
|---|---|---|
| 0x00 | u32 | offset of the first record (0x10) |
| 0x04 | u32 | **record count** (240 in unit 1: matches the 240 consecutive model imports) |
| 0x08 | u32 | number of parameter blocks (222 in unit 1) **[unconfirmed]** |
| 0x0C | u32 | 3 |

Record, 0x60 bytes **[verified]** (240 consecutive imports 96 bytes apart in unit 1):

| Offset | Type | Field |
|---|---|---|
| +0x00 | import | the `Model` (0x51); the slot is patched by the import table |
| +0x04 | f32 | 1.0 in the records looked at |
| +0x08 | u32 | an id (0x1409EC...) **[unconfirmed]**; |
| +0x10 | 16 bytes | zero |
| +0x20 | f32 x16 | the transform: a 4 by 4 matrix, **row vectors**, the translation in the last row; the fourth column is 1.0 in all four rows (so it is not a projective matrix: ignore it) |

After the records come parameter blocks of the form `u8 1, 1, 0, 0`, offsets, and a name (`diffuseTintColour`) with a float4: a
per-instance material tint (buildings, lit windows). The unit's instances are in **world space**; the big ground pieces
(terrain, the ocean, the mountains) are placed with an identity matrix and carry their position in their vertices, so a
position read from the matrix alone is wrong for them: use the renderable's bounding sphere transformed by the matrix.

Every unit also places the same distant **panorama pieces** (the ring of hills, the sea, a mountain) with a sphere radius
above 800 m. A reader that wants a district drops instances whose sphere centre is outside the unit's `PVS` rectangle (plus
25 m) or whose radius is over 800 m: that is how `tools/chameleon_export.py` clips a unit.

### Resources of other units **[verified]**

A unit's renderables and materials import textures and models that are **not in its own bundle**: 14 of the 36 diffuse textures of unit
161's materials (ids of class 1, for example `0x10000000010F9B1`) live in other units (`TRK_UNIT115`, `118`, `146`, `162`... and the
`ARENA_*` bundles) and in no shared bundle. The engine keeps the resources of all loaded units in one pool by id, and a unit stores only
what its neighbourhood does not already bring. A reader that resolves ids therefore needs the tables of **all 169 unit files** (about
120,000 entries, 15 ms to read the tables on the author's machine) after the unit's own and the shared bundles; copies of an id are the same
bytes. With them unit 161 has 1 flat picture instead of 7 and 3 missing models instead of 9.

### Other instance lists **[layout verified for the three object lists]**

| Type | Header | Record | Notes |
|---|---|---|---|
| `PropInstanceList` (0x218) | `u32 2, first = 0x10, count, 0` (the first record and the count are the second and third words; an `InstanceList` has them in the first and second) | 0x60 bytes: **the matrix first** (16 floats, rows of 4, the 4th value of the first three rows is 1 and of the translation row 1 or a uniform scale: 0.94 seen), **the object's import slot at +0x40** **[verified]** (the imports of a list sit 0x60 apart from 0x50), then `0, 0, id, 1.0` | 12,290 in all; refers to `PropObject` (0x217): 128 bytes with two `Model` imports (a whole and probably a broken one), Genesys objects (lamp posts, signs: breakable) **[unconfirmed]**. `mods/fairhaven` draws the first model |
| `CompoundInstanceList` (0x216) | same | 0x60, as the prop list | 4,301 in all; `CompoundObject` (0x215): 1,456 bytes, imports a `Model`, a `Skeleton` and Genesys objects; `mods/fairhaven` draws the first model |
| `DynamicInstanceList` (0x204) | `u32 3, 0x10, count, 0` | 0x60, as the prop list | 4,477 in all; `WorldObject` (0x205) objects (cranes, flags, signs: animated or moving), 288 bytes, the first import is a `Model`; drawn still |
| `LightInstanceList` (0x213) | `u32 1, 0, 0, 0` | | 169 lists, empty in unit 1 (16 bytes) |
| `GroundcoverInstances` (0x214) | `u32 0x10, count, 0, 0` | **16 bytes**: 8 bytes packed position, a u16 class, flags | 1,957 per list in unit 1, 8.8 million in all; `GroundcoverCollection` (0x20F) holds the meshes (2,069 resources) |

## Models, meshes and materials **[verified]**

A `Model` has up to four level-of-detail `Renderable` imports (slots from offset 48 in the main block: four consecutive
imports, LOD 0 first), more imports after them (a hull, `GameChanger` handles) that are not decoded, and switch distances as in
[chameleon-gfx.md](chameleon-gfx.md#model-type-0x51). A `Renderable` has the layout of the car ones (version 12, 0x60-byte mesh
records), with these differences for the world:

- **Vertices are `float32` positions** (descriptor element format 6 at offset 0), not the car's scaled snorm16. The mesh's
  vertex descriptor comes from the material's shader, as for cars. Position and the first UV pair (half floats, usage 5 at 12,
  16 or 24) are enough for a diffuse picture; further elements are a second UV (usage `0x106`, a lightmap or detail map) and
  colour or mask bytes (`0x4`, `0x207`, `0x308`, `0x409`, `0x50A`).
- Strides seen: 24 (`World*` techniques), 28 and 32 (`Plot*`, `DriveableSurface*`, `Fence*`). **No vertex has a normal**: the
  lighting is in the maps (lightmap, normal map), so a reader that needs normals works them out from the triangles
  (`mods/fairhaven/src/solid.rs`). The layouts of the common techniques **[verified]** (offset, DXGI format; position is
  `R32G32B32_FLOAT` at 0 in all): `World*` and `WorldPBR*` stride 24, UV0 half2 at 12, UV1 half2 at 16 (usage 0x106, the
  lightmap), colour `R8G8B8A8` at 20 (usage 0x207); `PlotPBR*` stride 32, colour at 12 (usage 4), UV0 half2 at 16, UV1 at 20,
  two `R8G8B8A8` and a half2 after; `DriveableSurface*` stride 32 to 64 with a colour at 12; `Diffuse_1Bit*`, `Fence*` and `Tree*`
  stride 28 to 32 with bone indices and weights on the skinned ones.
- **Primitive 5 (strip with 0xFFFF restarts) is used by every mesh looked at**; indices are u16.
- A mesh's bounding sphere is in the renderable's own space: place it with the instance matrix.

The technique names the shader gives (and which samplers the material's texture list follows, see
[chameleon-gfx.md](chameleon-gfx.md#shader-type-0x53)) show what a surface is:

| Family | Samplers (in material order) | What it draws |
|---|---|---|
| `World_Diffuse_*`, `WorldPBR_*` (buildings: `*_LitWindows_*`, `*_PersistentLitWindows_*`; options `Reflective`, `AO`, `1Bit`, `Lightmap`, `Doublesided`, `InstanceTint`) | `Normal`, `Diffuse`, `Specular`, `Overlay`, `Reflection`, `SpecularColour`, `Blend` TextureSampler | Buildings and props: diffuse, normal and specular maps, baked lightmaps, alpha-tested glass and foliage |
| `PlotPBR_*` (`Grass`, `DriveableGrass`, `TilingDecal`, `RGBDecal`, `StandingWater`, `Rough`) | `Mask`, `Tiling1..3Texture`, `Tiling1..3Normal`, `Decal`, `Reflection`, `Puddle`, `AOMap` | The terrain: a mask texture blends three tiling ground textures, with decals |
| `DriveableSurface_*` (`Decal`, `FloatingDecal`, `Line_*`, `Lightmap`, `PlanarReflection`) | `ColourMap`, `SpecColour`, `Normal`, `DetailMap_Normal/Diffuse`, `RoadDepth`, `EdgeAlphaPlusAoMap`, `Line_Diffuse`, `Decal_Diffuse` | Roads: a colour map with a detail layer, painted lines and decals |
| `Fence_GreyScale_Doublesided` | `Diffuse` | Fences (alpha-tested) |

`SHADERS.BNDL` holds 373 shaders (the 1,119 of the three quality variants are the same 373 ids); the world families above are
the majority, the rest belong to cars, effects and the post chain. A first-look renderer maps these families to a diffuse picture (`DiffuseTextureSampler`, `ColourMap_Sampler`,
`Tiling1TextureSampler`); `tools/chameleon_export.py` does exactly that, and the result is recognisable (a district with its
roads, piers, buildings; see below). Terrain blending, lightmaps, lit windows and reflections are the engine's shading and are
**not** decoded.

**Terrain tiling [verified on three materials].** A `PlotPBR*` or `TerrainPBR*` surface's mesh has texture coordinates over the whole
mesh (0 to 1 across a terrain tile that is several kilometres wide) and the picture of each layer repeats by the material's
`Tiling_Ratios_X` and `Tiling_Ratios_Y` parameters (float4s, the layers 1 to 3 in x, y, z): the constants of a `Plot*` material
start with them (slots 0 and 1), those of a `Terrain*` one with the noise ratio first (slots 1 and 2). Values seen: 1.5, 0.35, 50
(`Plot`), 30 and 4 (`Terrain`). A terrain drawn with the coordinates as they are is a smear of one picture's stretched pixels.
`TerrainPBR*` takes its ten samplers in the order `Mask, Normal, Tiling3Normal, Tiling1Normal, Reflection, Tiling3Texture,
Tiling2Normal, Noise, Tiling2Texture, Tiling1Texture`; the mask's red, green and blue look like the weights of three layers (rock,
grass, gravel on the unit 178 mountain material) **[unconfirmed]**, which `mods/fairhaven` does not blend (it draws layer 1).

**Water.** The sea is one mesh of 86 vertices and 123 triangles at height 10.0 (`Water_Proto_Cheap`, bounding sphere radius 23,250 m, in
unit 161 and the units by the coast; `Water_Opaque_Singlesided`, 69 vertices, in unit 1): a normal map and a reflection, no diffuse
picture, so `mods/fairhaven` draws it as one flat colour.

**Vertex colour.** The `R8G8B8A8` element at usage 0x207 of the `World*` meshes averages 128 over its four bytes (range 0 to 255) in
all the 77,235 vertices of the common technique: it looks like a packed normal or tangent (signed values stored as bytes), not a
baked light **[unconfirmed]**; the game's pre-lit shading takes a constant instead.

Textures are the car-texture format (BC1/BC3, and BC7 and BC4/BC5 for world maps: Pillow decodes them), with all mips in the
graphics block. A unit with 166 textures is about 25 MB of GPU data at its full size.

## Collision: `PolySoupList` (0x60) **[verified]**

One per unit, 145 KB in unit 1, 34 MB in all: **36,169 chunks, 1,201,146 triangles and 225,442 quads (1.65 million triangles)**.
It is the same chunk format as a car's soup ([chameleon-collision.md](chameleon-collision.md#the-collision-mesh-polysouplist-0x60)):

| Offset | Type | Field |
|---|---|---|
| 0x00 | f32 x3, pad | minimum corner of the whole soup (unit 1: -1415.9, -64.9, 1457.4) |
| 0x10 | f32 x3, pad | maximum corner |
| 0x20 | u32 | offset of the table of chunk offsets (0x30) |
| 0x24 | u32 | offset of the per-chunk boxes (0x2B0 in unit 1: groups of 4 boxes in SIMD layout, 0x70 bytes per group of 4: min x, min y, min z, max x, max y, max z as four floats each and four child indices; a bounding-volume tree **[unconfirmed]**) |
| 0x28 | u32 | chunk count (158 in unit 1) |
| 0x2C | u32 | size of the block |

A chunk (header 0x10 bytes, **offsets are absolute in the block**; chunks start 0x20-aligned):

| Offset | Type | Field |
|---|---|---|
| 0x00 | u32 | offset of the face table |
| 0x04 | u32 | offset of the vertex table |
| 0x08 | u16 | size of the chunk |
| 0x0A | i8 x3 | **cell of each axis**, a signed count of 500 m cells; the 4th byte is 0 or 1 |
| 0x0E | u8 | face count |
| 0x0F | u8 | vertex count (at most 255) |

A vertex is three u16 `q`; the coordinate is **`q * 500 / 32768 + 500 * cell`** metres (1.5 cm steps). The car format's flag byte is
the same rule with only the cells 0 and -1. This is **[verified]** three ways: the decoded chunk boxes equal the tree boxes of
the same soup to within 0.1 m (soup 0 of unit 1: x -1407.9 to ..., cells -3, -1, 2), every decoded unit lies inside its own `PVS`
rectangle (unit 161: x 1,437.8 to 2,107.1, z 1,857.7 to 2,376.9 against 1,436 to 2,026 and 1,869 to 2,329), and the drawn
faces form the streets, quays and the roundabout of the district (`tools/chameleon_collision_view.py`).

A **face** is 12 bytes: a u32 flag word, four u8 vertex indices (the fourth is 0xFF for a triangle: 1,201,146 triangles and
225,442 quads), four more u8 (neighbour or edge data: 0xFF or small numbers **[unconfirmed]**). **A quad's corners are in strip
order: draw it as 0, 1, 3, 2** (in file order the picture shows bow-ties) **[verified]** on the drawn district.

**Flag word, read off the flat faces of six units [measured]:** bit 0 (`0x1`, with `0x3`) is set on the faces of roads, quays
and other flat ground (|normal y| of 0.93 to 1.00 for the faces with it, area 27,000 to 81,000 m2 in the units looked at), while
faces with the low byte `0x00`, `0x20`, `0x40` or `0xC0` are mostly steep (terrain slopes and building walls, mean |normal y|
0.2 to 0.4) or flat terrain (`0x40`); bits 14 to 23 (10 bits) vary between 1 and 0xDE and look like an index into a surface or
material table; bits 24 to 31 are `0x0A` or 0. The surface names are hashed in `SURFACELIST.BNDL`, so the map from the index to a
surface is **not** known: `mods/fairhaven/src/collision.rs` takes bit 0 as asphalt and the rest as grass **[unconfirmed]**.

The **flag word** has 2,674 distinct values over the world (most common 0x0392C000, 0x0B90E001, 0x0472C000, 0x00608001); the low
bits carry flags (0x8000/0x4000/0x0001 patterns) and the middle bits look like a surface and a material index that the
surface list ([chameleon-effects.md](chameleon-effects.md)) would explain **[unconfirmed]**. This is the data a road surface
type ("tarmac, grass, sand": grip, sound, tyre effect) would be taken from.

Decoding the whole world takes 2.2 s in Python; unit 161 has 17,501 faces in 21,510 vertices. Props and cars do not use this
soup (a prop's collision is its `PropObject`, not decoded).

## Navigation and road data **[layouts partly seen, not decoded]**

| Resource | Size | What was seen |
|---|---|---|
| `NavigationMesh` (0x68), one per unit | 8.6 KB in unit 1, 9.2 MB in all | header `0, 2, offset, ...`, then floats; probably the unit's walkable or driveable mesh for AI **[unconfirmed]** |
| `RoadData` (0x203) | 73,584 bytes in the block | header `u32 9, 0x14, 0x1DF0, 0x93 (147)`, then 147 nodes of 0x34 bytes (4 flag bytes, a rectangle of 4 floats, 8 zero bytes, four sequential ids, two offsets), then leaves of 0x20 bytes that hold a position (x, y, z), a width-like float (14.6) and more floats: a quadtree over road points **[unconfirmed]** |
| `AiData` (0x200) | 599,168 bytes | a header of eight words and records of about 0x20 bytes with positions (x, y, z, radius) and small integers: 9,153 records by the header word at 0x14 **[unconfirmed]** |
| `TrafficLaneData` (0x701) | 2,351,904 bytes, 57 imports | header `u16 3, 0x126, u32 0x23DF88`, boxes in the same SIMD layout as the collision tree, lane lists **[unconfirmed]** |
| `TriggerData` (0x202) | 437,680 bytes | a header of 8 words (0x38, 0x1904, 0xB0F8, 0x35, ...), records with positions and a type **[unconfirmed]** |

The executable names these types (`RoadDataResourceType`, `AIDataResourceType`, `NavigationMeshResourceType`,
`PolygonSoupListResourceType`, `InstanceListResourceType`, `PropInstanceListResourceType`, `ZoneListResourceTypeV2`,
`GroundcoverInstancesResourceType`...) **[exe]**; the strings are at 0x00FA0D6C to 0x00FA10E8 and the unit file name format
(`TRK_UNIT`) at 0x00FAB170 and 0x00FAB200 (`NFS13.exe`). The code that reads them was not read.

## A first picture: the exporter **[verified]**

`tools/chameleon_world.py export <game dir> <out.glb> <unit>...` places the level-of-detail 0 model of every instance of the
units (clipped to each unit's rectangle) and writes one glb with a diffuse-textured primitive per texture (textures shrunk to
at most 256 pixels). Units 161 and 160 (a harbour district: roads, a highway, piers, tower blocks) give 820 instances,
418,024 triangles and 218 textures in a 30 MB file in **1.6 s**; unit 1 (the outskirts: a mountain, the hills, the sea) gives
240 instances, 220,942 triangles and 84 textures in 0.9 s. `tools/glb_preview.py` draws it with a software rasteriser. The
glb and pictures of this run are in `C:\r6work\fairhaven\` (outside the repository).

## Open questions

- The dequantisation of the groundcover instances and the format of `GroundcoverCollection`.
- The meaning of the face flag word and of the four edge bytes of a face.
- `RoadData`, `AiData`, `TrafficLaneData`, `NavigationMesh` and `TriggerData`: the record layouts and which of them a road
  network for traffic and police needs (the 2005 game's equivalent is [road-network.md](road-network.md)).
- Visibility: what the three offsets of the `ZoneList` header and the entry's offsets select (a visible list per unit?), and
  which neighbouring units the engine keeps resident.
- Time of day and weather in `ENVIRONMENT.BNDL` (Genesys objects), and the lightmaps the `*_Lightmap` techniques sample.
- Where the sky is: no sky dome was found among the unit models **[unconfirmed]**; it is probably a post effect.
