# Chameleon bundles (`.BNDL`, `bnd2`)

NFS: Most Wanted (2012) does not run on the Black Box engine. Like NFS: Hot Pursuit (2010), it runs on Criterion's
**Chameleon** engine, an evolution of the Burnout Paradise engine. Its data is not bChunk: the files are `bnd2`
bundles, a table of typed resources with zlib-compressed memory blocks and a pointer-patching import table.
Reader: [`libs/chameleon-bundle`](../../libs/chameleon-bundle). For the evidence tags
see [README.md](../README.md#evidence-tags); how this was learned is in
[provenance/chameleon.md](../provenance/chameleon.md).

Everything here was measured on the 2012 install (v1.4.0.0, `NFS13.exe`): all 2,021 `.BNDL` files parse, and the
203 files under `VEHICLES/` inflate completely with every block at its stated size **[verified]**.

## Where the data lives **[verified]**

| Path | Contents |
|---|---|
| `VEHICLES/VEH_<n>_HI.BNDL` | One car at full detail: body, wheels, materials, a few textures, gameplay objects (93 files) |
| `VEHICLES/VEH_<n>_LO.BNDL` | The same car at low detail (93 files); `_TR` files are traffic cars |
| `VEHICLES/VEHICLETEX.BNDL` | 53 textures shared by the cars (27 BC1, 26 BC3) |
| `VEHICLES/VEHICLELIST.BNDL` | The vehicle list and the gameplay database types (`VehicleListResourceType`, Genesys objects) |
| `SHADERS.BNDL`, `SHADERS0/1.BNDL` | Shader programs, vertex layouts and sampler tables; the three files are quality variants with the same ids |
| `HAWAII/*.BNDL` | The world: districts, arenas, traffic, triggers, textures |

## File layout

All integers are little endian. A bundle is a header, a resource table, and the data blocks.

### Header (0x70 bytes)

| Offset | Type | Field |
|---|---|---|
| 0x00 | char[4] | `bnd2` |
| 0x04 | u16 | version: **5** |
| 0x06 | u16 | platform: **1** (PC) |
| 0x08 | u32 | offset of the debug data; the file size when there is none |
| 0x0C | u32 | resource count |
| 0x10 | u32 | offset of the resource table (0x70 in all 2,021 files) |
| 0x14 | u32[4] | where each of the four memory blocks starts in the file |
| 0x24 | u32 | flags (below) |
| 0x28 | u64 | id of the default resource (0 unless flag 0x20) **[verified]** |
| 0x30 | i32 | stream index of the default resource: -1 in 1,880 files, 0 in the other 141 |
| 0x34 | char[4][15] | the names of the four streams, NUL padded; the 60 bytes end at 0x70 |

Bundle flags **[verified]**; the names are the community tool's (see
[provenance/chameleon-tools.md](../provenance/chameleon-tools.md)):

| Bit | Name | Files with it |
|---|---|---|
| 0x01 | zlib: every block is a zlib stream | 1,936 of 2,021 |
| 0x02 | debug data (`ResourceStringTable` XML) follows the data | 85 (all flag 0x2 alone: plain blocks, the `HAWAII/MAPS` bundles) |
| 0x04 | the loader must fix up imports on the loading thread (`NON_ASYNCH_FIXUP_REQUIRED`) | 3 |
| 0x08 | multi-stream: the stream names are set and each entry names its stream | 720 |
| 0x10 | delta bundle (a patch over another bundle) | none |
| 0x20 | the default resource is set | 56, exactly the files with a non-zero default id |

Combinations seen: 0x1 (1,208), 0x9 (669), 0x29 (51), 0x2 (85), 0x21 (5), 0x5 (3). Streams are named by the
bundle (`GRAPHICS`, `SOUND`, `SOUNDGLOBAL`, `PHYSICS`, `NAV` or `UI`); the 1,301 single-stream files name none.
Common sets: `UI, SOUND` (235 files), `GRAPHICS, SOUND, SOUNDGLOBAL, PHYSICS` (154), `GRAPHICS, SOUND, PHYSICS, NAV` (79). The
default resource is the one the loader substitutes for a missing one; which resource that is for each bundle is not
read.

### Resource table entry (0x48 bytes)

| Offset | Type | Field |
|---|---|---|
| 0x00 | u64 | resource id, see "Resource ids" below |
| 0x08 | u32[4] | size of each block when inflated, in the low 28 bits; the top 4 bits are the alignment as a power of two (0x4 = 16 bytes) |
| 0x18 | u32[4] | size of each block on disk (the zlib stream length) |
| 0x28 | u32[4] | offset of each block from the matching block start in the header |
| 0x38 | u32 | offset of the import table inside block 0 |
| 0x3C | u32 | resource type id (below) |
| 0x40 | u16 | import count |
| 0x42 | u8 | flags: 0 in all 353,289 entries **[verified]** |
| 0x43 | u8 | stream index: an index into the header's stream names (0 to 3), always below the number of named streams **[verified]** |
| 0x44 | u32 | padding, 0 **[verified]** |

An earlier version of this doc and of the reader took 0x42 as a u16 of flags and 0x44 as the stream index; both
were wrong, and read 0 for the car bundles only because those have a single stream.

PC files use block 0 (main memory: the resource's header and small data) and block 1 (graphics memory: vertex and
index buffers, texture pixels). Blocks 2 and 3 are empty **[verified]**.

Alignment codes **[verified]**: every non-empty block of the 353,289 entries has code 4 (16 bytes), except the graphics
block of one texture (code 0). In a bundle without the zlib flag the on-disk sizes carry a code in their top four bits
too (4 for block 0, 7 for blocks 1 to 3, seen in the 85 plain `MAPS` bundles); the reader masks it. The community tool
keeps a default code per type and block (all 4 for the 2012 PC types it lists); the install shows one value, so a
writer can use 4 everywhere.

## Resource ids **[verified]**

The 64-bit id is, in little endian bytes: a 32-bit key, a 16-bit sub-type, an 8-bit index and a class byte.

| Bits | Field | Notes |
|---|---|---|
| 0 to 31 | key | a hash of the name for class 0 (the hash function is not known, see the provenance doc); the plain number for class 1 |
| 32 to 47 | sub-type | 0 for most. 1,119 ids (the number of shaders) carry 0x53 here and 865 carry 0x8 (shader program buffers); other values are small numbers that tell apart resources with the same key |
| 48 to 55 | index | 0 for most ids |
| 56 to 63 | class | 0 normal (100,659 entries), 1 "game changer" (252,630); the community tool also names 0x80 (resource list) and 0xC0 (delta bundle), which no 2012 file uses |

A class 1 id is a number, not a hash: the debug data of `HAWAII/MAPS/1658001.BNDL` names its texture
`GameChanger (ID=1658001,type=0,index=0)` and its id is `0100000000194C91` (0x194C91 = 1,658,001). The community
tool writes ids as `ID=0x<key>, type=<sub-type>, index=<index>` and names unpacked files `<KEY>_<sub-type>_<index>`.
Resources of one bundle may share a key and differ in the other fields, so look up the full 64 bits.

### Import table

`import count` entries of 16 bytes at `import offset` in block 0: `u64 id, u32 offset, u32 0`. At load the engine stores
the address of resource `id` in the 32-bit slot at `offset` in block 0. A slot with no entry stays a plain value. This
is how a mesh names its material, a material its textures and shader, a model its renderables. Ids resolve across
bundles: a car's material imports a shader that lives in `SHADERS.BNDL` and a texture that lives in
`VEHICLETEX.BNDL`. With `VEH_*_HI`, `VEHICLETEX` and `SHADERS` loaded, every mesh, material and model import of the 30
cars checked resolves, except four renderable material ids **[verified]**.

Bit 31 of the stored offset is a **kind flag** **[verified]**: set in 1,079,705 of the 2,263,381 imports of the install,
clear in the rest; the second u32 is 0 in all of them. The community tool calls the two kinds `E_IMPORT_TYPE_POINTER` and
`E_IMPORT_TYPE_RESOURCE_HANDLE`: the slot receives the resource's address or a handle to it. Genesys objects use the
flag on the imports that name their types (the reader's `Import::kind()` and `Import::slot()` split it; the
`offset` field keeps the stored value because `chameleon-vehicle` and `chameleon-sound-graph` read the bit themselves).

**The import table is the last thing in block 0** **[verified]**: `import offset + 16 * import count` equals the
block's size for all 252,671 resources that have imports. A resource without imports has count 0 and offset 0. A writer
that builds a bundle must find the table of each resource, and the community packer does so per type from the
resource's own header; these rules hold for every resource of the install that has the type (they are the "header
says" half of the rule above, the other half being "count the entries at the tail"):

| Type | Where the import data is announced |
|---|---|
| Material (0x2) | u16 at 0x06 is the import offset (23,873 of 23,873) |
| Shader (0x53) | u16 at 0x12 is the import offset (1,119 of 1,119) |
| CharacterSpec (0x209) | u16 at 0x10 is the import count (3 of 3) |
| Font (0x30) | u32 at 0x04 is the import offset (17 of 17) |
| DynamicInstanceList (0x204), PropInstanceList (0x218), LightInstanceList (0x213) | u32 at 0x08 is the import count (154, 154, 116 of all) |
| CameraTakeList (0x20E) | u32 at 0x0C is the import count (1 of 1) |
| TrafficLaneData (0x701) | u16 at 0x14 plus u16 at 0x1C is the import count (1 of 1) |
| BearEffect (0x301) | the packer reads the offset from u16 at 0x14, which holds only for 17 of 188 resources: **do not rely on it** |

The other types the packer handles (InstanceList, GraphicsSpec, Model, Renderable, GenesysType/Object, VehicleSound,
PropObject, WorldObject, GroundcoverCollection, ReverbRoadData, ZoneHeader, Compound*) are found by walking back from
the end of the block in 0x14-byte steps until an import with a known slot appears; with the tail rule above a reader
never needs this.

### Debug data

When flag 0x2 is set, `ResourceStringTable` XML (`<Resource id=".." type="Texture" name=".." streamIndex=".."/>`) sits
at the debug offset, sometimes zlib-compressed (`78 ..`). An id of 8 hex digits is a bare key, one of 16 the whole id.
`Bundle::debug_names()` reads the rows. Only the 85 flag-0x2 `MAPS` bundles have it in the 2012 install, one row each
(the class 1 texture above), so the names of ordinary resources are not stored: car bundles have no debug data and a
car's parts are unnamed; names come from the gameplay objects (see [chameleon-vehicle.md](chameleon-vehicle.md)).

## Resource types **[exe]**

The ids and names are the ones the executable registers (one call per type at start-up). The ids the executable's table does
not list (0x30 Font, 0x68 NavigationMesh, 0x70 TextFile, 0x81 Wave and a few more) carry the name of the community tool
(`bundle_packer_unpacker`), which is also given where it differs from the executable's. `ResourceKind` uses the executable's names.

| Id | Name | Community name | Resources | Bundles |
|---|---|---|---|---|
| 0x1 | Texture |  | 39,860 | 1,476 |
| 0x2 | Material |  | 23,873 | 384 |
| 0x3 | VertexDescriptor |  | 450 | 3 |
| 0x4 | VertexProgramState |  | 4,659 | 3 |
| 0x5 | Renderable |  | 48,605 | 654 |
| 0x6 | MaterialState |  | 162 | 3 |
| 0x7 | SamplerState |  | 2,746 | 384 |
| 0x8 | ShaderProgramBuffer |  | 2,898 | 3 |
| 0x14 | GenesysType |  | 109,359 | 752 |
| 0x15 | GenesysObject |  | 83,655 | 752 |
| 0x30 | Font |  | 17 | 17 |
| 0x50 | InstanceList |  | 169 | 169 |
| 0x51 | Model |  | 12,679 | 382 |
| 0x52 | ColourCube |  | 67 | 37 |
| 0x53 | Shader |  | 1,119 | 3 |
| 0x60 | PolySoupList | PolygonSoupList | 366 | 366 |
| 0x68 | NavigationMesh | NavigationMesh | 169 | 169 |
| 0x70 | TextFile |  | 2,075 | 102 |
| 0x74 | LuaData |  | 0 | 0 |
| 0x80 | Ginsu |  | 915 | 184 |
| 0x81 | Wave | Wave | 7,411 | 737 |
| 0x82 | WaveContainerTable |  | 1 | 1 |
| 0x83 | GameplayLinkData |  | 1 | 1 |
| 0x84 | WaveDictionary |  | 1 | 1 |
| 0x85 | MicroMonoStream |  | 0 | 0 |
| 0x86 | Reverb |  | 2 | 1 |
| 0x90 | ZoneList |  | 1 | 1 |
| 0x91 | WorldPaintMap |  | 6 | 4 |
| 0xB0 | AnimationList |  | 379 | 185 |
| 0xB1 | PathAnimation |  | 0 | 0 |
| 0xB2 | Skeleton |  | 542 | 382 |
| 0xB3 | Animation |  | 1,297 | 148 |
| 0x105 | VehicleList | VehicleList (exe: VehicleListResourceType) | 1 | 1 |
| 0x106 | VehicleGraphicsSpec | GraphicsSpec | 197 | 197 |
| 0x200 | AiData | AIData | 1 | 1 |
| 0x201 | LocalisedText | Language | 70 | 14 |
| 0x202 | TriggerData |  | 1 | 1 |
| 0x203 | RoadData |  | 1 | 1 |
| 0x204 | DynamicInstanceList |  | 169 | 169 |
| 0x205 | WorldObject |  | 1,307 | 165 |
| 0x206 | ZoneHeader |  | 169 | 169 |
| 0x207 | VehicleSound |  | 290 | 187 |
| 0x208 | RoadMapData |  | 0 | 0 |
| 0x209 | CharacterSpec |  | 3 | 1 |
| 0x20B | SurfaceSounds |  | 0 | 0 |
| 0x20C | ReverbRoadData |  | 1 | 1 |
| 0x20D | CameraTake |  | 45 | 1 |
| 0x20E | CameraTakeList |  | 1 | 1 |
| 0x20F | GroundcoverCollection |  | 2,069 | 170 |
| 0x213 | LightInstanceList |  | 169 | 169 |
| 0x214 | GroundcoverInstances |  | 1,663 | 159 |
| 0x215 | CompoundObject |  | 671 | 147 |
| 0x216 | CompoundInstanceList |  | 169 | 169 |
| 0x217 | PropObject |  | 1,446 | 154 |
| 0x218 | PropInstanceList |  | 169 | 169 |
| 0x219 | ZoneAmbienceList | ZoneAmbienceListDataType | 1 | 1 |
| 0x301 | BearEffect |  | 214 | 1 |
| 0x302 | BearGlobalParameters |  | 1 | 1 |
| 0x303 | ConvexHull |  | 939 | 187 |
| 0x501 | HsmData | HSMData | 37 | 37 |
| 0x701 | TrafficLaneData | TrafficData | 1 | 1 |

Counts are over all 2,021 bundles of the install **[verified]**. Every type id that occurs is in the table (no unnamed
id). Five registered types do not occur: `LuaData` (0x74), `MicroMonoStream` (0x85), `PathAnimation` (0xB1),
`SurfaceSounds` (0x20B), and `RoadMapData` (0x208), which only the community tool names (the executable registers two
`RoadData` ids, 0x203 and 0x208). The community tool also knows ids of other platforms and games that no 2012 PC file
has (0x17 `MiiData`, 0x61 `PolygonSoupTree`, 0xC0 and 0xC1 Cgs program types, 0xDE `DeltaDeleted`, 0x210 to 0x212
early-beta types); `ResourceKind` leaves them out. "Community name" is the tool's name where it differs.

A high-detail car holds about 800 resources of 17 types: renderables (0x5), models (0x51), materials (0x2), textures
(0x1), samplers (0x7), skeleton and animations (0xB0/B2/B3), a poly soup and convex hulls (0x60, 0x303) for collision,
vehicle sound (0x207, 0x80, 0x81), HSM data (0x501), the graphics spec (0x106) and many Genesys types and objects
(0x14, 0x15), which carry gameplay data and the car's name.

## Genesys (0x14, 0x15)

Genesys is the engine's self-describing data system: `GenesysType` resources name a type
(`Genesys.Gen.NitrousParameters`, `bool8_t`, `float32_t`, `CgsCore.UniqueId`, ...) and `GenesysObject` resources hold
instances, with references to other objects through the import table. The car bundles carry the behaviour objects
(`<Car>_SteeringWheel_Behaviour`, `<Car>_DamageBehaviour`, `ChassisRigidBody`, `FrontLeft.WheelRigidBody`, engine sound
graphs). Their field layout is not decoded; only the object names are read **[unconfirmed]** beyond that.

## Reader

`Bundle::parse(&[u8])` reads the header (flags, default resource, stream names) and the table; `Bundle::resource(index)`
inflates the blocks and reads the import table; `Bundle::debug_names()` reads the debug names; `Pool` searches several
bundles by id (the first added wins a duplicated id, so add `SHADERS.BNDL` before its variants). `Entry::resource_id()`
takes an id apart, `ResourceKind::ALL`/`from_id`/`name` cover the 61 named types, `Import::slot()`/`kind()` split the
kind bit. The reader refuses other versions and platforms, truncated tables and blocks, and blocks whose inflated size
differs from the table.

The bundles of other games and platforms (Hot Pursuit 2010 and its Remastered version, console and handheld ports of the
2012 game, the older `bndl` layout) have other headers; they are described in
[chameleon-games.md](chameleon-games.md) and are not read. Writing a bundle is not implemented; the facts a writer needs
(the tail rule for the import table, the header, the alignment codes) are above. The text resource of the
`UI/LANGUAGE` bundles is read by `libs/chameleon-text` ([chameleon-language.md](chameleon-language.md)) and textures
export as `.dds` through `libs/chameleon-gfx` ([chameleon-gfx.md](chameleon-gfx.md)).
