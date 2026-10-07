# Research: prior art for NFS: Most Wanted (2005)

Everything we found before building, and where it stands. Surveyed on **2026-10-07** (repo metadata from
the GitHub API; licenses read from the actual LICENSE files; progress from decomp.dev). For what each
license lets us do, see [licensing.md](licensing.md). For the tag meanings, see
[evidence tags](README.md#evidence-tags).

**Summary.** No-one has rewritten MW's engine yet, but nearly everything a rewrite needs is already out
there:

- a matching decompilation (CC0) that also targets PC v1.3 and ships a symbol map for `speed.exe`;
- format readers for geometry, textures, AttribSys, text and UI;
- a working MW world renderer in noclip.website;
- vgmstream for every MW audio codec, and an MIT-licensed Rust VP6 decoder.

What is missing is mostly documentation (several formats are only described by code) and a renderer for
the PC shaders.

## Language and toolchain **[verified]**

MW is written in **C++** on EA Black Box's in-house engine (shared with Underground 2 and Carbon).

| Check on `speed.exe` (PE32, 6,029,312 bytes) | Result |
|---|---|
| SHA-256 | `80774c2e…1d253c`: retail **v1.3**; same hash pinned by [nfsmw-recomp](https://github.com/veritr1x/nfsmw-recomp) |
| PE optional header | Linker **7.10** = Visual C++ .NET 2003 |
| Imports | `d3d9.dll` (Direct3D 9) |
| C runtime | statically linked (no `MSVCR*.dll`) |
| RTTI strings | only CRT types: game classes built without RTTI |
| Shaders | 31 compiled D3DX `fx_2_0` effects as resources ([formats/shaders.md](formats/shaders.md)) |

Engine languages, from the decomp's source tree: C++ almost everywhere, some C in SDKs, an embedded **Lua**
(`zLua`), and data-driven gameplay through **AttribSys** databases ([formats/attributes.md](formats/attributes.md)).

## 1. Decompilations

| Project | What it gives a rewrite | License | Last push | Maturity |
|---|---|---|---|---|
| [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) ([progress](https://decomp.dev/dbalatoni13/nfsmw)) | Matching C++ for GameCube, PS2, X360 and **PC 1.3**. Original names, structs and file paths from GameCube DWARF symbols. `config/SPEED_EXE_1_3/symbols.json`: an address map of `speed.exe` (46,360 functions, ~14,300 with real MSVC names). `SpeedChunks.hpp`: 434 chunk names. Generated headers for all 57 AttribSys classes. | CC0-1.0 | 2026-10-02 | Active; 575 commits, 14 contributors |
| [Brawltendo/Most-Wanted-Vehicles-Decomp](https://github.com/Brawltendo/Most-Wanted-Vehicles-Decomp) | Chassis, SuspensionRacer, EngineRacer and Wheel matched against **PC** `speed.exe` | **none** | 2022-06-29 | Dormant; folded into the above |

Progress of dbalatoni13/nfsmw (decomp.dev, commit `1f2cdd7`):

| Version | Matched code | Fuzzy match |
|---|---|---|
| GameCube `GOWE69` | 60.7% | 66.9% |
| PS2 Black Edition `SLUS-21351` | 22.7% | 47.1% |
| PS2 Alpha 124 prototype | 18.6% | 43.7% |
| **PC 1.3 `SPEED_EXE_1_3`** | **13.8%** | **30.3%** |
| X360 prototype | 0.7% | 3.9% |

By subsystem (fuzzy, GameCube → PC): AI 99 → 64, PhysicsBehaviors (Chassis, SuspensionRacer, RBVehicle,
Damage) 99 → 58, zPhysics (Wheel, VehicleSystem) 2 → 2, World 99.5 → 50, World2 (collision, road network,
pathfinding, time of day) 99.8 → 50, Track streaming 99.5 → 50, AttribSys 100 → 45, Frontend 97–99 → 37–67,
EAXSound 99.9 → 45, Speech 100 → 77, renderer core (Ecstasy) 47 → 51, Camera 5 → 0, Gameplay 0. The PC
Direct3D 9 renderer is essentially absent (`src/Speed/PC` has 4 headers).

Notes:

- Building it needs the original binaries (`orig/SPEED_EXE_1_3/speed.exe` etc.). The repo has no assets
  or assembly but vendors 223 Xbox 360 SDK headers.
- The README ("SAY NO TO SLOP") asks people not to build ports on the unfinished decomp, especially with
  AI. The license (CC0) allows it; the project owner has read this and decided to proceed.
- **How we use it:** read-only reference. Formats and data layouts go into our docs as facts; behaviour
  (physics, AI) goes through the spec-first process in [licensing.md](licensing.md#spec-first). No code is
  copied.

No other MW decomp exists. decomp.dev lists only MW and NFS: High Stakes.

## 2. Sibling-engine reverse engineering (UG, UG2, Carbon, ProStreet, Undercover)

There is no public decomp of any sibling game.

| Project | Value | License | Last push |
|---|---|---|---|
| [yugecin/nfsu2-re](https://github.com/yugecin/nfsu2-re) | UG2 structs, functions and globals with addresses; hash dumps | **none** | 2025-02-26 |
| [MaxHwoy/hyperlinked](https://github.com/MaxHwoy/hyperlinked) | Reverse-engineered Carbon streamer, renderer, world and vault code as a hook library | BSD-3-Clause | 2025-07-11 |
| gaycoderprincess/nya-common-{nfsug2,nfsc,nfsps,nfsuc} | Per-game headers with addresses | MPL-2.0 | 2026 |
| [Zolika1351/nfsc-sdk](https://github.com/Zolika1351/nfsc-sdk) | Carbon `.asi` SDK | **none** | 2023-10-31 |

## 3. Engine reimplementations, recompilations and viewers

| Project | Value | License | Last push |
|---|---|---|---|
| [magcius/noclip.website](https://github.com/magcius/noclip.website) `src/NeedForSpeedMostWanted` | **Working MW world viewer** (TypeScript/WebGL): map, regions, particles, post-processing | MIT, with a note that parts were reverse-engineered, not clean-room | 2026-10-06 |
| [whoismept/OpenUG2](https://github.com/whoismept/OpenUG2) | Clean-room C/OpenGL **UG2** engine: streamed world, geometry, TPK, car records, racing-line AI; `docs/FORMATS.md` | MIT | 2026-10-05 |
| [bdrtr/PryHUB](https://github.com/bdrtr/PryHUB) | The only Rust NFS project found: `gizmo-nfs` parses UG2 containers, geometry, TPK, CARP | Apache-2.0 | 2026-09-01 |
| [OpenNFS/OpenNFS](https://github.com/OpenNFS/OpenNFS) | NFS 1–5 engine; **does not load MW** | MIT | 2026-06-21 |
| [veritr1x/nfsmw-recomp](https://github.com/veritr1x/nfsmw-recomp) | Static recompilation of **PC 1.3**; `docs/analysis.md` covers loading; pins the v1.3 hash | MIT | 2026-09-21 |
| elforeign/nfs-most-wanted-mac | Same kit, PC 1.3 → Apple Silicon/Metal | MIT | 2026-10-07 |
| X360 recomps (NFSMW-Recompiled, nfsmw-nx, nfsmw-android, nfsmw-ios) | Binary translation; useful only as behaviour references | GPL-3.0 | 2026 |

## 4. Runtime hooks and reversed in-memory structures (PC)

| Project | Value | License | Last push |
|---|---|---|---|
| [berkayylmao/NFSPluginSDK](https://github.com/berkayylmao/NFSPluginSDK) | Header-only C++20, MW about 60% (Attrib, PVehicle, AIVehicle, Pursuit) | BSD-3-Clause | 2024-12-22 |
| [gaycoderprincess/nya-common-nfsmw](https://github.com/gaycoderprincess/nya-common-nfsmw) | PC 1.3 addresses and ~90 struct headers | MPL-2.0 | 2026-08-06 |
| [ThirteenAG/WidescreenFixesPack](https://github.com/ThirteenAG/WidescreenFixesPack) | Camera, FOV, HUD, resolution, shadow hooks | MIT | 2026-09-10 |
| [ExOptsTeam/NFSMWExOpts](https://github.com/ExOptsTeam/NFSMWExOpts) | Many gameplay addresses | GPL-3.0 | 2023-02-19 |
| [nlgxzef/NFSMWUnlimiter](https://github.com/nlgxzef/NFSMWUnlimiter) | Car list and limits | **none** | 2023-12-12 |
| [xan1242](https://github.com/xan1242): NFS-XtendedInput, XNFSMusicPlayer, NFSMW_XenonEffects | Input, music, particles (XenonEffects backports Carbon code) | MIT | 2022–2025 |
| [rng-guy/NFSMWBartender](https://github.com/rng-guy/NFSMWBartender) | Pursuit internals: cop spawn tables, heat, roadblocks, helicopter | CC0-1.0 | 2026-10-05 |
| s-b-repo/nfsmw-2005-re | Ghidra renames and attribute-hash cracks. **Low trust**: agent-generated, some claims wrong | BSD-3-Clause | 2026-08-13 |

## 5. Format readers and editors

| Project | Covers | License | Last push |
|---|---|---|---|
| [NFSTools/NFS-ModTools](https://github.com/NFSTools/NFS-ModTools) | Solid, scenery and TPK readers for UG through World (MW included); `ChunkView`; FBX export | **none** | 2024-08-30 |
| [NFSTools/VaultLib](https://github.com/NFSTools/VaultLib) | AttribSys read/write | MIT | 2025-02-15 |
| [NFSTools/Attribulator](https://github.com/NFSTools/Attribulator) | VLT ↔ text, hash database | **none** | — |
| [NFSTools/GlobalLib](https://github.com/NFSTools/GlobalLib) | Global files (cars, presets, materials, strings), JDLZ | MIT | 2020-04-18 |
| [NFSTools/FEngLib](https://github.com/NFSTools/FEngLib) | FEng UI packages: read, write, render | **none** | — |
| [SpeedReflect/Binary](https://github.com/SpeedReflect/Binary) / NFSTools/Binary | Global-file editor | GPL-3.0 | 2021 / 2026 |
| [SpeedReflect/Nikki](https://github.com/SpeedReflect/Nikki) | Global-file library (strings, collision, sun info) | MIT | 2021 |
| [vgmstream](https://github.com/vgmstream/vgmstream) | Every MW audio codec and container (see [formats/audio.md](formats/audio.md)) | ISC-style | 2026-09-27 |
| [ruffle-rs/nihav-vp6](https://github.com/ruffle-rs/nihav-vp6) | Pure-Rust VP6 decoder, relicensed by its author for Ruffle | MIT | 2026-05-20 |
| FFmpeg `electronicarts.c` + `vp6.c` | EA video container and VP6 | LGPL-2.1+ | — |
| [x07x08/nfsmw-save-editor](https://github.com/x07x08/nfsmw-save-editor) | Save offsets | Unlicense | — |
| VltEd, NFS-CarToolkit (nfsu360) | AttribSys editor; add-on car builder | closed source | — |

Per-subsystem coverage, with the best reference for each, is in the format docs:
[audio](formats/audio.md), [video](formats/video.md), [frontend](formats/frontend.md), [text](formats/text.md),
[attributes](formats/attributes.md), [world](formats/world.md), [shaders](formats/shaders.md),
[saves](formats/saves.md), [car data](formats/cardata.md), [HUFF](formats/huff.md).

## 6. Community knowledge bases

- **No dedicated MW format wiki exists.** The best documentation is code (above) plus these docs.
- nfsmods.xyz: mod hosting only; Discord "NFSMods". nfscars.net now redirects to hangaraddons.com.
- Xentax is closed; its successor ResHax has an unanswered MW geometry thread.
- TCRF has prototype pages. Its main MW page contained text aimed at AI agents; it was ignored and not
  used as a source.
- [TsyVM/MWEncyclopedia](https://github.com/TsyVM/MWEncyclopedia) (533 pages, **no license**) is
  **unreliable** (it calls asset hashes Joaat, describes saves as `.loc`, omits MicroTalk). Leads only.

## 7. Physics, handling and AI

There are no prose write-ups; everything is in code:

- **Physics:** the decomp's PhysicsBehaviors (Chassis, SuspensionRacer with the tire model, RBVehicle,
  RigidBody, DamageRacer) is ~99% matched on GameCube; Wheel and VehicleSystem are ~2%. Brawltendo's repo
  has PC-matched Chassis/SuspensionRacer/EngineRacer/Wheel. gaycoderprincess's MWPhysics ports (MPL-2.0
  over decompiled code) claim a 1:1 port and include per-car handling dumps.
- **AI:** the decomp's zAI is ~99% on GameCube (racers, pursuit, roadblocks, cop manager, traffic,
  steering, PID controllers); road network and pathfinding ~99.8%.
- **Tuning data:** all of it is in AttribSys (`pvehicle`, `chassis`, `engine`, `tires`, `transmission`,
  `aivehicle`, `pursuitlevels`, …); see [formats/attributes.md](formats/attributes.md). Bartender
  documents the pursuit side.

Our rewrite reimplements these **spec-first**: read the decomp, write the behaviour down in
`docs/specs/`, implement from the spec ([licensing.md](licensing.md#spec-first)).

## 8. Reference projects for the Rust rewrite

| | [vladtrc/iw4L](https://github.com/vladtrc/iw4L) | [chasmlol/2010-rust-rewrite-mashup](https://github.com/chasmlol/2010-rust-rewrite-mashup) |
|---|---|---|
| What | Standalone Rust runtime for CoD MW2 multiplayer (also reads IW5, T5, T6 assets); ~421k lines, 64 crates | Stale fork of iw4L plus Skate 3 and Minecraft modes |
| License | Apache-2.0, with NOTICE, generated third-party licenses, provenance notes | Apache-2.0, but vendors GPL-3.0 crates without their license file |
| Stack | bevy 0.19 (ECS only), wgpu 29, winit 0.30, glam, cpal + symphonia | same |
| Install discovery | `IW4L_GAMES` env var or `.env`, Windows shortcuts, Steam registry/library; validates header magic + version | same + first-run folder picker |
| `.gitignore` | blacklist | blacklist |
| Guards | opt-in pre-commit hook; `publish-check` bans game extensions, binaries, `FUN_`/`DAT_` names, exe-range hex | same, but it would fail on the fork's own tests |
| Tests | none on synthetic data; scenario runs against the real install | vendored unit tests |

**We copied the structure, not the code:** format crates that never touch the disk, one crate that does
all file access, a renderer that hides its graphics API, a thin launcher, a `.env` + config + registry
lookup, and a leak check. **We did differently:** a whitelist `.gitignore`, the leak check in CI and a
hook, unit tests on synthetic data plus `#[ignore]`d real-install tests, a much smaller crate graph. See
[architecture.md](architecture.md).

## 9. Rust ecosystem

No Rust crate exists for JDLZ, HUFF, EA-XA, EA-XAS, MicroTalk or the EA video container, so we write
those from specs. The chosen crates and their licenses are in [rust-stack.md](rust-stack.md).

## Engine notes from the decomp **[decomp]**

- **Chunk loading.** `bChunkLoader` keeps a table of ID → loader/unloader functions, bucketed by
  `(id + (id >> 6) + (id >> 12)) & 0x3F`; `CallLoaders()` walks a chunk stream and dispatches each chunk.
- **In-place loading.** Data is used where it was loaded. Pointer fields inside on-disk structs are zero and
  get filled at runtime (`pMemory` in `TrackStreamingSection`, `ChunkData` in texture streaming entries)
  **[verified]**.
- **Name hashing.** `bStringHash` (`h = h * 33 + c`, seed `0xFFFFFFFF`) links solids, textures and scenery
  **[verified]**; AttribSys uses Jenkins lookup2 with initval `0xABCDEF00` **[verified]**.
- **Animation loader.** `eagl4supportdlopen.cpp` / `eagl4supportsympool.cpp` act as a small dynamic linker
  for the EAGL ELF objects.

## When we would need Ghidra

For PC-only code the decomp hasn't matched yet (the D3D9 renderer, input, PC frontend glue). The decomp's
`symbols.json` already names ~14,300 functions in our exact `speed.exe`, so it can be applied instead of
starting from scratch. Toolchain: [TOOLS_AND_SKILLS.md](TOOLS_AND_SKILLS.md).
