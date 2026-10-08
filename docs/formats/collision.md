# Collision: world packs, grid, car and prop bounds, queries

How the game stores what cars and cameras can hit: the static world collision (one `CARP` pack per
streaming section), the grid that finds the right pack entries, the bounds of cars and props, and how
a ray or point query uses them. Rust reader: [`libs/blackbox-collision`](../../libs/blackbox-collision).
Behaviour credit and process: [provenance record](../provenance/world-collision-query.md). For the tag
meanings, see [evidence tags](../README.md#evidence-tags). Little-endian, metres.

## Where it lives **[verified]**

| Data | Chunk | Where | Count / size |
|---|---|---|---|
| World collision of one section | `0003B801 CarpWCollisionPack` | `TRACKS/STREAML2RA.BUN`, top level | 390 packs, 5,802,128 B, sections 101…2030 |
| World map tree (collision grid + road network) | `0003B800 CarpWGrid` | `TRACKS/L2RA.BUN` | 1 × 551,432 B |
| Prop bounds | `8003B900 BoundsPack` → `0003B901` | `TRACKS/L2RA.BUN` | 405 sets, 2,141 nodes, 332 point clouds |
| Car bounds | `8003B900 BoundsPack` → `0003B901` | `GLOBAL/GlobalB.lzc` | 86 sets, 1,066 nodes, 97 point clouds |
| Surface types | AttribSys class `simsurface` (`0xFB111FEF`) | `GLOBAL/attributes.bin` | 47 collections ([attributes.md](attributes.md)) |

Not present in this install: `0003B802` (grid island data) and collision objects (`co` records).
Triggers are not in the collision packs; they use the `EmTriggerPack` ([world.md](world.md)).

## Coordinate space **[verified]**

Collision data is in **physics space**: x right, y up, z forward, metres, right-handed
([specs/vehicle-suspension-tires.md](../specs/vehicle-suspension-tires.md)). The scenery and the render
world use a z-up space; the engine converts with `bConvertToBond`: `(x, y, z) → (−y, z, x)` **[decomp]**.
Checked on section 101: its 61 scenery positions (render space, e.g. `(322.3, 2516.3, 166.9)`) land in the
bounding box of its collision instance centres after the conversion (`x −2548…−2450`, `y 165…199`,
`z 337…373`).

## The `CARP` blob (`UGroup` tree) **[decomp + verified]**

A blob is a tree of tagged **groups** and **data records**, 16 bytes each. Both packs and the world map
tree use it. All 390 packs and the map tree contain only the embedded form.

**Group** (`UGroup`, 0x10 B):

| Offset | Type | Field |
|---|---|---|
| 0x00 | u32 | Tag |
| 0x04 | u32 | Flags: bit 0 indexed, **bit 1 embedded (always set)**, bit 2 allocated, bit 3 data sorted, bit 4 group sorted, bits 5…31 **child group count** |
| 0x08 | u32 | Data record count |
| 0x0C | u32 | Offset of the child array **in 16-byte units from this header** (1 = right after it) |

The child array holds the child groups first, then the data records. Records and groups are sorted by tag.

**Data record** (`UData`, 0x10 B):

| Offset | Type | Field |
|---|---|---|
| 0x00 | u32 | Tag |
| 0x04 | u32 | Flags: bit 0 indexed, bit 1 embedded (always set), bit 2 allocated, bits 3…7 section, bits 8…31 **size in bytes** |
| 0x08 | u32 | Element count (0 for a plain blob) |
| 0x0C | u32 | Offset of the data **in bytes from this header** |

A tag is two characters of type (high 16 bits) and an index (low 16 bits): `'ca'` + 5 is `0x63610005`
(stored `05 00 61 63`). Four-character tags are plain big-endian text: `Arti` = `0x41727469`. The root
group is `CARP` with one or more child groups. The layout was measured on the install; `UGroup`'s array
accessor is not in the decomp, so the offset units (16 B for groups, bytes for records) come from the
data: every pack and the map tree resolve with them.

## Collision packs (`0x3B801`) **[decomp + verified]**

The chunk payload, aligned to 16, starts with `bChunkCarpHeader` (0x10 B), then the blob:

| Offset | Type | Field |
|---|---|---|
| 0x00 | i32 | Blob size (a multiple of 16) |
| 0x04 | i32 | Section number (101 = `A1`, [maps.md](maps.md#the-streaming-index-decomp--verified)) |
| 0x08 | i32 | Flags: bit 0 = already resolved in memory. **0 in all 390 packs** |
| 0x0C | ptr | `LastAddress`, the relocation base. 0 in all packs |

The blob is `CARP` → one `Arti` group with these records (all 390 packs):

| Tag | Count | Contents |
|---|---|---|
| `Name` | 0 | 4 bytes, unused |
| `ca` 0…N−1 | 1 each | One article per instance ([Articles](#articles)) |
| `ci` 0 | N | N instance records of 0x40 B ([Instances](#instances)) |
| `co` 0 | M | Collision objects of 0x70 B. **Absent in this install** |

Totals **[verified]**: 7,544 instances and 7,544 articles (1 to 84 per pack), 38,379 strips,
243,429 triangles, 320,187 packed vertices, 45,815 barriers. Instance *k* uses article *k* in every pack
(`RenderInstanceInd == k`).

### Instances

A `WCollisionInstance` (`CARP::CollisionInstance`, 0x40 B) places an article. It stores the inverse
transform, **world to local**, plus the extents of the local box:

| Offset | Type | Field |
|---|---|---|
| 0x00 | f32[3] | Row X of the rotation (`fInvMatRow0Width.xyz`) |
| 0x0C | f32 | Half width: extent along local x |
| 0x10 | u16 | `IterStamp`: runtime scratch; build junk on disk (e.g. 3516, 512) |
| 0x12 | u16 | Flags. **0 on disk.** Runtime: bit 0 y axis not up, bit 1 animated, bits 6…7 exclusion |
| 0x14 | f32 | Half height: extent along local y |
| 0x18 | u16 | Scenery group number (0 = none; 891 instances in 752 groups) |
| 0x1A | u16 | Article index (`RenderInstanceInd`) |
| 0x1C | ptr | Article pointer: a stale build-time address on disk; the loader looks the article up by index |
| 0x20 | f32[3] | Row Z of the rotation |
| 0x2C | f32 | Half length: extent along local z |
| 0x30 | f32[3] | Translation of the world-to-local transform |
| 0x3C | f32 | Radius of the circle in xz that contains the instance |

Row Y is `(0, 1, 0)` unless flags bits 0…1 are set, then `Z × X`. The transform uses the row-vector
convention of the engine's matrices: `local = x·rowX + y·rowY + z·rowZ + t`. Local space is **centred on
the instance's box**: 99.2% of strip vertices have `|x| ≤ half width` and `|z| ≤ half length` of their
instance. The world position of the centre is `−(t·rowX, t·rowY, t·rowZ)`.

**Every one of the 7,544 instances is axis aligned** (`rowX = (1,0,0)`, `rowZ = (0,0,1)`, flags 0), so on
disk the transform is a pure translation, `centre = −t`, and the rotation convention is
**[decomp]**, **not exercised by the data**. The radius is at least `hypot(half width, half length)` for
all of them.

### Articles

A `WCollisionArticle` is one record of `size` bytes (a multiple of 0x40; the bytes after the surface
table are uninitialised build junk). Layout:

| Offset | Type | Field |
|---|---|---|
| 0x00 | u16 | Strip count |
| 0x02 | u16 | Strips size: the strip area incl. the sphere table, rounded up to 16 |
| 0x04 | u16 | Barrier ("edge") count |
| 0x06 | u16 | Barriers size = count × 0x20 (all 7,544 ✔) |
| 0x08 | u8 | Resolved flag (0 on disk) |
| 0x09 | u8 | Surface count (1…6) |
| 0x0A | u16 | Surfaces size = count × 4 (✔) |
| 0x0C | u16 | `IntermediatObjInd`: build value, unused at runtime |
| 0x0E | i16 | Flags (0) |

After the 0x10 header, in this order: the **sphere table** (strip count × 0x10), the **strip data**
(padded so the area is `strips size` long, 8 pad bytes in 1,871 articles), the **barriers**, then the
**surface hashes** (u32 each). 3,498 articles have no strips and 4,035 have no barriers.

**Strip sphere** (0x10 B): `f32[3]` centre in local space; `u16` radius in 1/16 m; `u16` offset of the
strip's first vertex, **relative to the end of the article header** (`article + 0x10 + offset`). The
radius is the largest 3-D distance from the centre to a vertex, rounded up (mean 1.004 × the measured
value over all 38,379 strips; at most 1.26×).

**Strip**: N packed vertices of 8 bytes, consecutive and in sphere order with no gaps:

| Offset | Type | Field |
|---|---|---|
| 0 | i16 ×3 | x, y, z in 1/128 m, **relative to the sphere centre** |
| 6 | u8 | Surface index |
| 7 | u8 | Surface flags |

The two surface bytes of vertex 0 hold the vertex count N as a u16, and those of vertex 1 hold the strip
flags as a u16 (**1** first triangle faces up, **2** facing unknown, 0 otherwise; 24,237 / 304 / 13,838
strips). N is 3…60 (9,608 strips have 3). Triangle *i* (0 ≤ i < N−2) is vertices *i*, *i+1*, *i+2* and
takes its **surface index and flags from vertex i+2**. Winding alternates along the strip; flag 2 means
the data has no consistent winding.

**Triangle surface flags** seen in the install (vertices ≥ 2): 0, 0x04 (16,355), 0x08 (57), 0x0C (4),
0x14 (16). The engine reads 0x04 as "keep looking for a lower face under this one" (see
[queries](#query-semantics)) and 0x08 as "ignore in ground-height point queries".

**Barrier** (0x20 B): two world-height points of a wall, as two `Vector4`:

| Offset | Type | Field |
|---|---|---|
| 0x00 | f32[3] | Point 0 (local space) |
| 0x0C | u8, u8, u16 | Surface index, flags, 0 |
| 0x10 | f32[3] | Point 1 |
| 0x1C | f32 | `1 / xz length` of the segment (✔ on all barriers) |

The wall spans `[min y, max y]` of the two points over the xz segment. Surface indices seen: 0 (41,914), 1
(3,162), 2 (702), 3 (37). Flags seen: 0x02 (45,455), 0x12 (351), 0x06 (9). 0x10 makes the wall block from
both sides; the meaning of 0x02 and 0x04 is not known.

**Surface hashes**: each is an AttribSys key of the `simsurface` class (see below). Hash 0 appears 1,944
times (an unused slot).

### Surface types **[verified]**

The 20 distinct hashes in the packs, as `vlt_hash` (Jenkins lookup2, initval `0xABCDEF00`) of the lower-case
name. Hashes with a guessed name were found by hashing candidate words; they match a 32-bit hash, so the
names are almost certainly right, but the files hold only the hashes **[verified by hash match]**.

| Hash | Name | Uses | `simsurface` collection |
|---|---|---|---|
| `19DB2F1E` | asphalt | 3,215 | yes |
| `6CA26F9B` | concrete | 2,372 | yes |
| `772FB736` | grass | 1,391 | yes |
| `35187254` | ? | 1,130 | yes |
| `58EC09DD` | ? | 769 | yes |
| `3830BABB` | cobble | 661 | yes |
| `52E42012` | wood | 298 | yes |
| `C1C577D6` | gravel | 250 | yes |
| `3E25CAAB` | ? | 227 | yes |
| `BA404853` | metal | 156 | yes |
| `1A20E734` | ? | 148 | yes |
| `D929E923` | dirt | 87 | yes |
| `02FC0FC9` | stone | 71 | yes |
| `999ACD78` | sand | 67 | yes |
| `A3840311` | ? | 49 | yes |
| `121453E7` | railroad | 9 | yes |
| `33199393` | ? | 2 | **no** |
| `1D5F6D70` | glass | 1 | yes |
| `4BB72BD3` | sidewalk | 1 | **no** |
| `00000000` | (none) | 1,944 | no |

The `simsurface` class has 47 collections; the ones matched by name include `unknown` (`5A0A54BD`),
`null` (`90A4D09A`), `default` (`EEC2271A`), `carbody` (`99DA6531`, the cars' default), `bumper`, `mud`,
`ice`, `snow`, `water`, `hay`, `plastic`, `rooftile`, `softbarrier`. A hash with no collection is looked up as
`unknown`.

## The collision grid (`0x3B800`) **[decomp + verified]**

The world map tree (`CARP` root with groups `CDat`, `Map `, `RNgp`; the road network is `RNgp`, see
[world.md](world.md)). The collision grid is the `CDat` group:

| Record | Contents |
|---|---|
| `CGrd` | One 0x24 B header (below) |
| `CGcn` | 2,779 grid nodes back to back (210,900 B) |
| `co` 0 | Empty |

**Grid header** (0x24 B): `f32[4]` `fMin` (here `(−4800, 0, −1664, 1)`), `f32` edge length (**64**), `f32`
inverse edge, `u32` rows (**103**), `u32` cols (**92**), `ptr` stale (nodes are indexed at load). A point
is in column `floor((x − min.x) / edge)` and row `floor((z − min.z) / edge)`; the node index is
`row × cols + col` (< 9,476). Points outside are clamped.

**Grid node** (variable size, `0x14 + 4 × total entries`):

| Offset | Type | Field |
|---|---|---|
| 0x00 | ptr | Runtime list of dynamic entries (0) |
| 0x04 | u16 | Node index (`row × cols + col`) |
| 0x06 | u16 | 0 |
| 0x08 | u8 ×4 | Entry counts: instances, triggers, objects, road segments |
| 0x0C | u16 ×4 | Byte offset of each kind's entries from the end of the header (cumulative: 0, 4·n₀, …) |
| 0x14 | u32… | Entries |

An instance entry is `section << 16 | index` (the pack's section number and the instance index in it);
a road segment entry is an index into the `RNsg` table. Totals: 21,112 instance entries, 17,718 road
segment entries, **no trigger or object entries**. All 7,544 instances are listed, every reference
points at an existing instance of the 390 packs, and the cell containing an instance's centre lists it in 7,431
cases (the other 113 are listed one cell away; the builder's cell test differs slightly from the centre).
The node sizes sum exactly to the record size ✔.

## Collision objects (`co`) **[decomp]**

`WCollisionObject` (0x70 B), a free box or cylinder: `f32[4]` centre + bounding radius, `f32[4]` half
dimensions, `u8` type (0 box, 1 cylinder), `u8` shape, `u16` flags (bit 0 dynamic), `u16` render instance,
`u8,u8` surface and flags, 8 pad bytes, `f32[16]` matrix (rows). The game also creates objects at load
from the `TrackOBB` list (chunk `00034191`, 96 B in `L2RA.BUN`) as boxes of surface 0. No pack holds any.

## Bounds (`8003B900 BoundsPack`) **[decomp + verified]**

A container of `0003B901` chunks (payload aligned to 16, 8 pad bytes in all). Each is one object's
bounds tree. The game's `CollisionGeometry::Lookup(hash)` finds a set by its name hash, and
`Collection::AddTo` builds physics primitives from it (it is not in the decomp, so the traversal below is
measured).

**Layout of a set**: header (0x10 B), nodes (0x30 B each), point cloud section.

| Offset | Type | Field |
|---|---|---|
| 0x00 | u32 | Name hash |
| 0x04 | i32 | Node count |
| 0x08 | i32 | Resolved flag (0) |

**Node** (`CollisionGeometry::Bounds`, 0x30 B):

| Offset | Type | Field |
|---|---|---|
| 0x00 | i16 ×4 | Orientation quaternion `(x,y,z,w) / 32767` |
| 0x08 | i16 ×3 | Position, mm, **relative to the parent's pivot** |
| 0x0E | u16 | Flags (below) |
| 0x10 | i16 ×3 | Half dimensions, mm |
| 0x16 | u8 | Child count |
| 0x17 | i8 | Point cloud index (−1 none) |
| 0x18 | i16 ×3 | Pivot, mm: the shape's absolute centre in the object's space (child pivot ≈ root pivot + position) |
| 0x1E | i16 | Index of the first child in the node array |
| 0x20 | f32 | Radius (spheres; equals the half dimension) |
| 0x24 | u32 | Surface hash (`simsurface` key; 0 = the object's default) |
| 0x28 | u32 | Node name hash |
| 0x2C | ptr | Owning collection (0 on disk) |

**Tree**: node 0 is the root. Node *i*'s children are `nodes[firstChild … firstChild + childCount)`. In all
491 sets each node is reached exactly once from the root ✔. Cars have 1…48 nodes (40 of the 86
have 10) and props 1…65. The M3 GTR has a root box, two body boxes, a mesh-vs-ground node and six spheres.

**Flags** (`kBounds_*`) **[decomp]**: 0x1 disabled, 0x2 primitive vs world, 0x4 vs objects, 0x8 vs
ground, 0x10 mesh vs ground, 0x20 internal, **0x40 box**, **0x80 sphere**, 0x100 conical constraint, 0x200
prismatic constraint, 0x400 female joint, 0x800 male joint, 0x1000 male post, 0x2000 joint invert.
Seen: 0x40 (box root), 0x46, 0x50 (box + mesh vs ground), 0x82 (sphere vs world), 0x6E, 0x160, 0x1060. Rigid
body setup maps vs-world / vs-objects / vs-ground to collider bits 1 / 2 / 4 and disabled to 8.

**Point clouds**: after the nodes, a 0x10 B header (`i32` cloud count) then for each cloud a 0x10 B
descriptor (`i32` vertex count, 12 bytes zero in all 429 clouds) followed by that many `Vector4` points
(`w = 0`). Total size matches the chunk exactly in all 491 sets ✔. Points are the convex hull used by
mesh-vs-ground nodes (the M3 GTR: 16 points, `z ∈ ±2.2`, `y ∈ 0.06…0.6`).

**Keys**: car sets are keyed by `vlt_hash(CarTypeName)` (upper case, e.g. `BMWM3GTR`) ✔ for all 86; 86 of the
91 car types have bounds (`BMWM3`, `TRUENO`, `LEVIN`, `TRUENOCP`, `TRUENOID` have none). Car axes: x
width, y up, z length; the M3 GTR root box has half dimensions `(0.938, 0.621, 2.271)` m and pivot
`(0.004, 0.621, 0.162)`. Prop sets are keyed by `vlt_hash` of a scenery object's name ([Props](#props)). Node
surfaces: cars use `carbody` (633 nodes), 0 (336), `metal` (72), `glass` (10), `wood` (10), `stone` (5);
props use 0 (1,249), `metal` (524), `wood` (351), `stone` (9), `glass` (8).

## Props **[verified on the install]**

The 405 prop sets in `TRACKS/L2RA.BUN` are the collision of loose and standing scenery (cones, bins,
benches, poles, signs, fences). How they are found and used, measured with throwaway probes:

- **Key.** `vlt_hash` (Jenkins lookup2, initval `0xABCDEF00`) of the scenery object's name **in its own case**, the
  name in the scenery info (`XO_TrafficConeA_1b_00`). 190 of the 405 sets match a name exactly. The info
  stores 23 characters, so 2,785 of the 4,103 names are cut; the key is the full name. Appending one of a few
  endings (`0`, `00`, `_DE`, `_DE0`, `1`, `A`) to a cut name finds 44 more sets, 234 in all (227 are used by
  scenery; the other sets belong to names we cannot reconstruct). `PropCatalog::shape` does this.
- **Axes.** The set is in the bounds space of cars (x width, y up, z length, left-handed); the scenery model
  is x forward, y left, z up. A box centre `(x, y, z)` is `(z, -x, y)` in model space, half dimensions
  `(hx, hy, hz)` are `(hz, hx, hy)`, and rotations are conjugated by that reflection. Checked: 97.7% of the 26,522
  boxes of the 12,229 matched instances have their centre inside the instance's own scenery box (enlarged by
  1.5 m) after placing them with the instance matrix.
- **Which nodes.** Nodes with the "primitive vs world" flag (`0x2`); a set without one uses its root.
- **Rigid or light.** Nothing in the bounds says it. The attribute class `smackable` (181 collections named by kind of
  object: `cone`, `crate`, `bench`, `firehydrant`, `dumpster`, `crsh_barrel`, `largemetalobject`, ...) has `MASS` (cone
  100 kg, bench 100, hydrant 50, trash can 100, crash barrel 700, dumpster 200, traffic light 200, foundation 100,000)
  and `NO_CAR_EFFECT`, but which scenery object uses which collection is **not** in any file read so far (it is
  probably in code or in track data we have not found). The Rust side guesses the kind from the name
  (`loose_class`) and treats up to 150 kg as light: `XO_` objects by their name, street signs (`XS_Warn*`,
  `Speed*`, `Stop*`, `DoNot*`, `No*`, `Chevr*`... as `largesign`, 100 kg) and picket and chain-link fences. Poles,
  stalls, `XW_` walls and `XB_` buildings are rigid. [guess: matches how the signs fell over in play]
- **Barriers are in the packs.** Of the guard rails (`XW_Guardrail*`, 834 instances), concrete and metal barriers
  (`XW_BarrConc*`, `XW_BarrMet*`, 2,798), `XW_BarrRails*`, iron rails, fences and chains, 98% to 100% have a
  barrier or steep face of the world collision packs within 1.5 m at 0.5 m height. They only seemed missing because
  nothing used the packs' barriers for cars.
- **Not found:** the `FlyBy`/animated props, trigger-driven objects (`EmTriggerPack`), and the sets whose names are
  unknown.

## Query semantics

The behaviour below is the **spec** the Rust reader follows; its source is the decomp
(`WCollisionMgr.cpp`, `WWorldPos.cpp`, `WGrid.cpp`, `WWorldMath.cpp`). Tolerances are the game's.

**1. Candidates.** Cells are found from the grid: for a point and radius, the cells of its xz box (a
span over 20 cells is cut to one); for a segment, the cells its xz projection crosses (a walk that gives up
after 100 cells and uses the start cell). Instance entries are collected once per query. An instance is
kept if its pack is loaded, `group == 0` or its group is enabled, it has an article, and
`surfaceExclusionMask & flags == 0`. The same mask is applied to strips (strip flags), triangles and
barriers (surface flags). The mask is 0 for most callers (everything counts); vehicle bodies pass
their own mask. After a pack loads, instances with `group ≠ 0` and no strips get flags |= 0xC0 and every barrier of
a grouped instance gets 0xC0 in its flags, so those can be excluded.

**2. Instance filter for a segment.** Distance in xz from the instance centre to the segment must be
below the instance radius (for a non-upright instance, the largest of radius, half width, length, height).
An upright instance also needs the segment's y range to overlap `centre.y ± half height`.

**3. Segment preparation.** A zero-length segment hits nothing; one shorter than 0.01 m is lengthened to
0.01 m along its direction.

**4. Faces (segment).** The segment moves to local space. A strip is tested only if the segment passes
within the sphere radius of its centre. Each triangle is tested as a ray/triangle intersection on a
frame aligned with the segment (two-sided, no culling); the hit must lie between the ends of the segment
(the plane crossing has `t ∈ [0, 1]`). The nearest by distance from the start wins across strips and
instances. The hit normal is the triangle normal flipped to the side of the segment's start. In this
path flag 0x08 is **not** checked.

**5. Faces (ground height under a point).** The point is moved to local space and must be inside the
instance's `|x| ≤ half width`, `|z| ≤ half length`. For each triangle whose xz projection contains the
point (strips with flag 2 use either winding; otherwise the winding alternates starting from flag 1) and
whose surface flag 0x08 is clear, the height of its plane under the point gives
`distance = point.y − planeY`. The accepted triangle has the smallest positive distance, its lowest vertex
must be below `point.y + 0.5` (upright instances), and the first accepted triangle of a strip stops that
strip's search. Face normals have `y ≥ 0` clamped to 0.9999. If nothing is found under a point, the
game falls back to a vertical segment from 2 m below to 1000 m above.

**6. Barriers.** In local space the segment's xz projection must cross the wall's xz segment (a tolerance
of 0.0001 on both parameters); the y of the crossing, interpolated along the segment, must be strictly
between the wall's lowest and highest y. The nearest crossing wins. Normal: `(dz, 0, −dx)/length` of the
wall, flipped to face the start of the segment; so a one-sided wall is "seen" from its normal's side only
for *vehicle* contact (flag 0x10 makes it two-sided); a pure ray hit reports it from either side.

**7. Combining.** `CheckHitWorld(segment, mask)` takes the nearest of the face hit (bit 1) and barrier
hit (bit 2) by squared distance from the start. The result type is 1 (face) or 2 (barrier), with the point,
the normal and a pointer to the instance. Vehicle contact against barriers uses a box-box test of the body
against each wall treated as a thin box, rejecting walls whose normal faces away unless 0x10 is set.

**8. Surface type.** A triangle's or barrier's surface index selects `article.surfaces[index]`, a hash;
`SimSurface::Lookup(hash)` returns the `simsurface` collection with that key, or `unknown` if none (hash 0
included). Flags travel with the hit. The surface supplies tire grip, friction, noise and effects; its
fields are in [attributes.md](attributes.md).

## Verified, and what is not

Verified with a throwaway probe (outside the repo) and the Rust reader's ignored test against this install:
chunk and blob structure of all 390 packs and the map tree; every size relation above; strip and
barrier decoding (rays dropped onto the centre of the first triangle of each strip-bearing instance of
the first sections always hit a face under it); grid node layout and references; coordinate space; bounds
trees, point cloud sizes, key hashes and the 86/405 counts; surface hashes against `attributes.bin`.
Not verified: the rotation convention of tilted instances (none exist in the data), the 0x02 / 0x04
flags, the names of eight surface hashes, the prop bounds keys, and the original game's results in a
side-by-side run.
