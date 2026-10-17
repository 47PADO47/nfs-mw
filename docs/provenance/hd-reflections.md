# HD reflections: provenance

- **Spec:** [specs/hd-reflections.md](../specs/hd-reflections.md). Written before any implementation.
- **Restricted sources read:**
  - `speed.exe` (NFS:MW PC 1.3, image base `0x400000`), read with Ghidra MCP, read only. Program path `/speed.exe`.
  - NFS HD Reflections by Aero_ (GPL-3.0), via the public README and source summary, as recorded in
    [docs/mods/nfshdreflections.md](../mods/nfshdreflections.md).
  - NFSMWHDReflections by sniten (GPL-3.0), via the public README and source summary, as recorded in
    [docs/mods/nfsmwhdreflections.md](../mods/nfsmwhdreflections.md).
  - Existing port docs for context: [specs/rear-view-mirror.md](../specs/rear-view-mirror.md) (mirror far plane 192 m, read
    with Cheat Engine in a menu), [specs/scenery-lod.md](../specs/scenery-lod.md), [specs/car-reflections.md](../specs/car-reflections.md).
- **Use of the mods' addresses.** Their address tables were used as **pointers to find facts** in `speed.exe`, and
  for verification only. No code, cave, address table or INI layout from either mod is in the spec. Addresses are
  recorded below because provenance may record them.
- **Date:** 2026-10-16. No game was launched and no cargo was run.

## Verified facts and where they were found

Original values are read from the executable's initial data (file bytes, read with `read_memory`) or decoded
from the immediates in code. Both count as **[verified]** for the value. Use of the value is **[decomp]**.

| Address | What | Evidence |
|---|---|---|
| `0x008F9000` | 32-byte block: 21, 80, 256, 256, 0.5, 100.0, 32.0, 6 | `read_memory`, decoded by hand, little endian |
| `0x008F9008`, `0x008F900C` | mirror width and height, 256 and 256 | bytes; read by `FUN_006bcfa0`, `FUN_006bd0d0` |
| `0x008F9010` | 0.5, the near plane of the six car-cube cameras | bytes; read by `FUN_006da350` |
| `0x008F9014` | 100.0, the far plane of the six car-cube cameras | bytes; read by `FUN_006da350`, which writes it to each camera object |
| `0x008F9018` | 32.0, read as data by `FUN_006da350` | bytes; matches the 32 px threshold in [specs/scenery-lod.md](../specs/scenery-lod.md) |
| `0x008F8FF4` | 256, the car-cube face size used when the option is non-zero | bytes; read by `FUN_006bd4b0` |
| `0x008F8FF0`, `0x008F8FF8`, `0x008F8FFC` | 21, 21, 80: cube format and related values | bytes; meaning not confirmed |
| `0x008F945B` | 1: flag byte that enables the six-face cube routine | bytes; tested by `FUN_006bd4b0` |
| `0x006CFC26`, `0x006CFC2D` | immediates 0x140 (320) and 0xF0 (240) in `FUN_006cfad0` | bytes decoded; the road record stores them at offsets 0x14 and 0x18 |
| `0x009017B0` | VERSION, 1 | bytes |
| `0x009017B8` | car environment enable, initial 0 | bytes; read in `FUN_006bd4b0`, `FUN_006bcdc0` |
| `0x009017BC` | car environment update data, initial 0 | bytes |
| `0x009017D4` | road reflection enable, initial 0 | bytes; tested in `FUN_006bcdc0` |
| `0x009017DC` | motion blur enable, initial 1 | bytes (not part of this feature) |
| `0x009017B4` | unknown value, 132 | bytes; no reader checked |

Option names (strings): `g_CarEnvironmentMapEnable` `0x008AEB50`, `g_CarEnvironmentMapUpdateData` `0x008AEB30`,
`g_RoadReflectionEnable` `0x008AEB18`, `REFLECTEDTEX` `0x008AEDC8`, `REFLECTEDPROJ` `0x008AEDD8`,
`REFLECTEDTEXMIN` `0x008AEDB8`, `WINDOWREFLECTION` `0x008AEBFC`, `worldreflect.efx` `0x008AE79C`.
The option writer and reader are `FUN_006C1F10` (writes registry defaults under `Software\EA Games\Need for Speed
Most Wanted`) and `FUN_006C1C00` (reads them). The shader parameter binder is `FUN_006C3D20` (reads
`REFLECTEDTEX`, `REFLECTEDPROJ`, `REFLECTEDTEXMIN`, `WINDOWREFLECTION`).

## Functions and what they do (prose, from decompiled logic)

- `FUN_006BCFA0` (mirror targets): creates a 256 x 256 target, then a second target when `0x00982B18` > 0, of 320
  x 200 (0x140 by 200). [decomp]
- `FUN_006BCDC0` (road target): when the road enable is 0, makes a 1 x 1 target. When it is non-zero, makes a 320 x 240
  target. A further 320 x 240 pass exists when `0x0090185C` == 3. [decomp]
- `FUN_006BD4B0` (car cube): returns early when the cube flag `0x008F945B` is 0. The face size starts at 16 and becomes
  `0x008F8FF4` (256) when the car environment global is non-zero. It creates six faces. [decomp]
- `FUN_006DA350` (view camera update): for six records, writes near `0x008F9010` and far `0x008F9014` into the camera
  object at record `+0x40` (offsets `+0xBC` and `+0xC0`), and reads `0x008F9018`. The record type selects a rotation
  per face (cases 5 to 10). Caller: `FUN_00756060`. [decomp; which views these are is unconfirmed]
- `FUN_006CFAD0` (road state update): a state machine on `0x00982B3C` and `0x00982B40`, using a 320 x 240 record (`0x140`,
  `0xF0`). It returns early when the in-game state `0x00925E90` equals 6, and otherwise sets a per-frame flag
  from a scene object list (`0x009B27EC`) whose flag entries are checked. Caller: `FUN_006E7220`. [decomp]
- `FUN_006BD7C0`: the caller of the mirror, road and cube setup functions. Not decompiled in full. [unconfirmed]
- `FUN_006E7220`: the frame view loop, reads the car environment global at `0x006E7362`. Not decompiled in full.
- `FUN_006C3D20`: the shader parameter binder, names above. Not decompiled in full. [decomp, names only]

## Community facts (mod claims, not checked)

Defaults and option names as stated by each mod are in the two mod pages. Mapped to the spec sections:

| Topic | Aero (mod) | sniten (mod) | Spec section |
|---|---|---|---|
| Cube size and scale | 256, 1.0 | vehicle scale 1.0 | 2 |
| Mirror scale | 1.0, auto (W/2) x (H/6) | 1.0, (ResY) x scale and (ResY/3) x scale | 3 |
| Road scale and HD | 1.0 | 1.0, HDRoadReflections 1, blur 1 | 4 |
| Power-of-two | OldGPUCompatibility 0 | none | 5 |
| LOD | ImproveReflectionLOD 1 | ImproveReflectionLOD 2 (0 off, 1 basic, 2 full) | 6 |
| Distance | 100 m cube, 384 m mirror (conflicts with 192 m verified) | 100 m most, 192 m mirror | 7 |
| Restored content | RestoreDetails 1, RestoreVisualTreatment 1, RestoreShadows 1, RestoreWaterReflections 0, RestoreCars 0 | RestoreDetails 1 (2 adds particles), GeometryFix 1 | 8 |
| Vehicle brightness | 1.0 | 1.0 (a third mod uses 0.8) | 9 |
| Mirror tint | none | on, 150 150 150 | 10 |
| Slot and memory pools | ExpandMemoryPools 1 (one value doubled) | ExpandSlotPool 1, limit 1,024,000 | 11 |
| Backface culling | RestoreBackfaceCulling 1, DisableBackFaceCulling 0 | none | 12 |
| Front-end cube | FECubemapRes 256 | none | 13 |

## Conflicts and resolutions

- **Mirror distance.** Aero's 384 m versus sniten's 192 m. The original's mirror far plane was read at 192 m (menu,
  Cheat Engine, in [specs/rear-view-mirror.md](../specs/rear-view-mirror.md)). The spec takes 192 m. The 384 value was
  not found in `speed.exe` (no byte-pattern search tool was used in this pass).
- **Vehicle brightness.** Aero and sniten both say 1.0. The third mod in the docs says 0.8. The spec keeps 100 percent.
- **LOD default.** Aero 1, sniten 2. The spec's default stays at the original behaviour.
- **Name mismatch in sniten.** The README and INI say `HDReflections`; the code reads `HDRoadReflections`. Not used
  in the spec (the spec proposes new names).

## Lessons

- The executable's initial data holds the option defaults and the reflection sizes. `read_memory` on a 32-byte block
  near the mod's named sites gave the mirror and distance values in one call. Decode the small integers and
  floats by hand carefully: `0x42C80000` is 100.0, `0x3F000000` is 0.5, `0x42000000` is 32.0.
- Immediates in a function show the default sizes directly. `read_memory` of 96 bytes around a function gave 0x140 and
  0xF0 as operands of `mov dword [reg+offset], imm`. Those are the road target size.
- A registry option writer (`FUN_006C1F10`) writes the in-memory values as defaults. Reading the global's initial
  bytes is therefore a fair default, with the caveat that the options screen may change it.
- `get_functions` with `fields=decompiled_code` on several functions is very large (several KB each). Use one or two
  functions per call, and `fields=callers,labels` for triage.
- `get_xrefs_to` on a shared global (the car environment global has about 200 references) returns a very long list.
  Use `limit` or check only the functions that name the feature.
- A pointer test on a registry global (`if (global != 0)`) can read as "size selector" rather than "on/off". Read the
  branch before naming the option.

## Dead ends

- **Option `DO` names.** `DOCarReflectRate` (`0x0089D034`), `DOCarReflectDetail` (`0x0089D048`) and
  `DORoadReflectDetail` (`0x0089D078`) have xrefs only from unwind funnels (code at `0x0086EC7E`, `0x0086ECBA`, `0x0086ECCE`). No reader
  found. Not used.
- **Value 132 at `0x009017B4`.** Written with the registry defaults, no reader checked. Not used.
- **384 m mirror value.** No Ghidra search for the float constant (no byte-pattern tool used). Left open.
- **Front-end cube.** No front-end caller of `FUN_006BD4B0` was confirmed. Left open.
- **Meaning of the 21 and 80 values at `0x008F8FF0` to `0x008F8FFC`.** Not decoded. Not used.

## Next leads

1. The registry-reading callers of the car environment and road globals, to learn the stock options-screen values.
2. Whether the car environment value is a level (1, 2 ...) or only on/off.
3. The front-end car view, to see whether the six-face cube is built in the front end.
4. The 384 value for the mirror in the camera update, using a byte search.
5. The scene-object flag list checked by the road state update.
6. Which pipeline uses `cull_mode: None` in `libs/blackbox-render/src/gpu/pipelines.rs`.
7. Whether the Bevy `request_view_capture` path (which falls back to `request_capture`) shows the mirror.

## Sources

- `speed.exe`: Ghidra MCP, program `/speed.exe`, read only. Tools used: `search_strings`, `read_memory`,
  `get_xrefs_to`, `get_functions` (fields `decompiled_code`, `callers`, `labels`).
- https://github.com/AeroWidescreen/NFSHDReflections (README and source summary, GPL-3.0), via docs/mods.
- https://github.com/sniten/NFSMWHDReflections (README and source summary, GPL-3.0), via docs/mods.
