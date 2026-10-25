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

### Header (0x28 bytes)

| Offset | Type | Field |
|---|---|---|
| 0x00 | char[4] | `bnd2` |
| 0x04 | u16 | version: **5** |
| 0x06 | u16 | platform: **1** (PC) |
| 0x08 | u32 | offset of the debug data; the file size when there is none |
| 0x0C | u32 | resource count |
| 0x10 | u32 | offset of the resource table (0x70 in files that have the extra stream-name block after the header) |
| 0x14 | u32[4] | where each of the four memory blocks starts in the file |
| 0x24 | u32 | flags: bit 0 = blocks are zlib streams (set in 1,936 of 2,021 files), bit 1 = a `ResourceStringTable` follows the data; bits 2, 3 and 5 (0x4, 0x8, 0x20) also occur and are unknown |

The bytes between 0x28 and the table may hold a table of block names (`GRAPHICS`, `SOUND`, `SOUNDGLOBAL`, `PHYSICS`)
in files with several streams. The reader ignores them.

### Resource table entry (0x48 bytes)

| Offset | Type | Field |
|---|---|---|
| 0x00 | u64 | resource id. Top byte 0x01 for most, 0x00 for a few; the low 32 bits are a name hash (names are not stored) |
| 0x08 | u32[4] | size of each block when inflated, in the low 28 bits; the top 4 bits are the alignment as a power of two (0x4 = 16 bytes) |
| 0x18 | u32[4] | size of each block on disk (the zlib stream length) |
| 0x28 | u32[4] | offset of each block from the matching block start in the header |
| 0x38 | u32 | offset of the import table inside block 0 |
| 0x3C | u32 | resource type id (below) |
| 0x40 | u16 | import count |
| 0x42 | u16 | flags (0 in the car bundles) |
| 0x44 | u32 | stream index (0 in the car bundles) |

PC files use block 0 (main memory: the resource's header and small data) and block 1 (graphics memory: vertex and
index buffers, texture pixels). Blocks 2 and 3 are empty **[verified]**.

### Import table

`import count` entries of 16 bytes at `import offset` in block 0: `u64 id, u32 offset, u32 0`. At load the engine stores
the address of resource `id` in the 32-bit slot at `offset` in block 0. A slot with no entry stays a plain value. This
is how a mesh names its material, a material its textures and shader, a model its renderables. Ids resolve across
bundles: a car's material imports a shader that lives in `SHADERS.BNDL` and a texture that lives in
`VEHICLETEX.BNDL`. With `VEH_*_HI`, `VEHICLETEX` and `SHADERS` loaded, every mesh, material and model import of the 30
cars checked resolves, except four renderable material ids **[verified]**.

### Debug data

When flag bit 1 is set, `ResourceStringTable` XML (`<Resource id=".." type="Texture" name=".."/>`) sits at the debug
offset, sometimes zlib-compressed (`78 ..`). Car bundles have none, so a car's parts are unnamed; names come from the
gameplay objects (see [chameleon-vehicle.md](chameleon-vehicle.md)).

## Resource types **[exe]**

The ids and names are the ones the executable registers (one call per type at start-up).

| Id | Name | Id | Name |
|---|---|---|---|
| 0x01 | Texture | 0x90 | ZoneList |
| 0x02 | Material | 0x91 | WorldPaintMap |
| 0x03 | VertexDescriptor | 0xB0 | AnimationList |
| 0x04 | VertexProgramState | 0xB1 | PathAnimation |
| 0x05 | Renderable | 0xB2 | Skeleton |
| 0x06 | MaterialState | 0xB3 | Animation |
| 0x07 | SamplerState | 0x105 | VehicleListResourceType |
| 0x08 | ShaderProgramBuffer | 0x106 | VehicleGraphicsSpec |
| 0x14 | GenesysType | 0x200 | AIData |
| 0x15 | GenesysObject | 0x201 | LocalisedText |
| 0x50 | InstanceList | 0x202 | TriggerData |
| 0x51 | Model | 0x203, 0x208 | RoadData |
| 0x52 | ColourCube | 0x204 | DynamicInstanceList |
| 0x53 | Shader | 0x205 | WorldObject |
| 0x60 | PolySoupList | 0x206 | ZoneHeader |
| 0x74 | LuaData | 0x207 | VehicleSound |
| 0x80 | Ginsu | 0x209 | CharacterSpec |
| 0x82 | WaveContainerTable | 0x20B | SurfaceSounds |
| 0x83 | GameplayLinkData | 0x20C | ReverbRoadData |
| 0x84 | WaveDictionary | 0x20D / 0x20E | CameraTake / CameraTakeList |
| 0x85 | MicroMonoStream | 0x20F / 0x214 | GroundcoverCollection / GroundcoverInstances |
| 0x86 | Reverb | 0x213 | LightInstanceList |
| 0x81 | (unnamed; 7,411 resources, audio-sized) | 0x215 / 0x216 | CompoundObject / CompoundInstanceList |
| | | 0x217 / 0x218 | PropObject / PropInstanceList |
| | | 0x219 | ZoneAmbienceList |
| | | 0x301 / 0x302 | BearEffect / BearGlobalParameters |
| | | 0x303 | ConvexHull |
| | | 0x501 | HSMData |
| | | 0x701 | TrafficLaneData |

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

`Bundle::parse(&[u8])` reads the header and table; `Bundle::resource(index)` inflates the blocks and reads the import
table; `Pool` searches several bundles by id (the first added wins a duplicated id, so add `SHADERS.BNDL` before its
variants). The reader refuses other versions and platforms, truncated tables and blocks, and blocks whose inflated size
differs from the table.
