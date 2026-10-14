# NFS HD Reflections (AeroWidescreen/NFSHDReflections)

NFS HD Reflections is a community mod by Aero_ for the PC versions of several EA/Black Box Need for Speed games. It is an ASI-loaded DLL that patches the game's in-memory code to raise the resolution and detail of reflections (cubemaps, mirrors, road reflections, vehicle and front-end reflections) and to restore some reflection-related rendering features that the retail builds dropped. Its behaviour is controlled by an INI file. The source tree covers five titles (NFSC, NFSMW, NFSPS, NFSU, NFSU2); this page covers only the Most Wanted (NFSMW) source, which is the one relevant to nfs-mw. The repository is licensed under GPL-3.0.

Status of this page: read from the public web pages and raw files only. Nothing was cloned, downloaded or run. Several facts below come from automated page summaries of the source files rather than from a line-by-line reading, and are marked as such where it matters.

## 1. Metadata

| Field | Value |
|---|---|
| Repository | https://github.com/AeroWidescreen/NFSHDReflections |
| Author | Aero_ (credited in README). Based on Extra Options by the ExOpts Team. |
| Type | `.asi`-style DLL mod (ASI loader + in-process memory patching) |
| Licence | GPL-3.0 (LICENSE file present; full GPL-3.0 text confirmed on fetch). The GitHub label also shows GPL-3.0. |
| Language | C++ (Visual Studio solution `NFSHDReflections.sln`). Language share not shown on the page; the `.cpp`/`.h` sources confirm C++. |
| Target game build | NFS: Most Wanted (2005, PC) per `SOURCE/NFSMW`. The entry-point address check in `dllmain.cpp` (see section 3) is what selects the build. README also names Carbon, Underground 2 and Most Wanted; the other title folders were not read. |
| Mod download | nfsmods.xyz, mod 3363 (README). Not fetched. |
| Latest visible activity | 69 commits on `main`; 1 open pull request; 37 stars, 4 watchers, 5 forks (GitHub page). The latest commit date is not shown in the summaries. Footer shows 2026. |
| Releases | None shown on the repository page. |

## 2. What it does

Each feature below is a user-visible behaviour. Evidence is the README (`[README]`) and/or the source (`[source]`).

- **AutoRes**: derives reflection resolutions from the current screen size instead of fixed values. [README] names the feature; the screen-based formulas are in settings/Init as described by the source summary. [source, via summary of dllmain.cpp]
- **Scale multipliers** (`CubemapScale`, `MirrorScale`, `RoadScale`): multiply the reflection resolution. [README] lists "Scale" as a resolution multiplier; the three per-type keys are in the source. [source]
- **OldGPUCompatibility**: rounds the cubemap resolution down to a power of two for older GPUs. [README] and [source].
- **ImproveReflectionLOD**: raises the level of detail used in reflections. [README]. Source patches an LOD check and a constant in the LOD path. [source]
- **ExtendRenderDistance / OptimizeRenderDistance**: README calls this "ExtendRenderDistance" (vehicle reflection render distance). Source key is `OptimizeRenderDistance` and sets distances (384 for mirror and back-of-cubemap, 100 for cubemap types, 0.5 for skybox distance). [README] names the feature; key and values [source].
- **RealFrontEndReflections**: README says it enables real-time vehicle reflections in the front end. The source key set does not contain a key with this name; the closest source feature is `RestoreCars` (cubemap cars) and `FECarModelFixCodeCave`. Mapping not verified.
- **RealisticChrome**: README says it makes chrome more reflective. No matching key was found in the source summary. Not verified.
- **VehicleReflectionBrightness**: scales vehicle reflection intensity. [README] and [source] (writes two float values scaled by the setting; see section 3).
- **RestoreDetails, RestoreCars, RestoreShadows, RestoreVisualTreatment, RestoreShaders, RestoreWaterReflections, RestoreBackfaceCulling**: re-enable reflection-related rendering paths (traffic lights, sunflares, car shadows, particles, cubemap cars, shadow mirror, visual-treatment passes, water reflections, backface culling of front-end cars). [source]. Not all of these are named in the README.
- **RemoveLastGenEffects**: removes some last-generation effects (level 1 and level 2 patch different call sites). [source]
- **ExpandMemoryPools**: enlarges several engine memory pool sizes. [source]
- **DisableBackFaceCulling, CubemapBrightnessFix, ForceEnableMirror, RestoreShaders road-reflection removal, FECubemapRes**: additional rendering tweaks. [source]
- **Optional HD cubemap** for Need for Speed Carbon, hosted on MEGA (README). Asset, not code. Not downloaded and not verified.

Compatibility notes from the README `[README]`: tested with Widescreen Fix, Xbox 360 Stuff, XtendedInput, Extra Options and Unlimiter; compatibility is not guaranteed. A No-CD fix is recommended because memory addresses are hardcoded. The Widescreen Fix is recommended as a base.

## 3. How it does it

Mechanism summary: an ASI loader injects `dinput8.dll` (the mod DLL is installed as the loader proxy by README instructions). The DLL checks the host build, reads an INI file, and writes/overwrites code and data in the game's memory at fixed addresses. Changes are made with 5-byte jumps into "code caves" (naked functions), NOP fills, and direct writes of integers and floats.

Tags: `[source]` verified in the code summary; `[README]` only stated in the README; `[inferred]` deduced.

- **ASI loader**: README says install Ultimate-ASI-Loader and copy its `dinput8.dll` into the game folder. [README]. The mod source contains no loader code: the DLL does its setup in `DllMain`. [source]
- **Build check**: on `DLL_PROCESS_ATTACH`, `DllMain` computes an address from the module base and `AddressOfEntryPoint`, compares it to `0x7C4040`, and calls `Init()` only on a match. Otherwise it shows an error message box and returns FALSE. [source, via summary]. This is how the mod refuses to run on a different executable. Exact arithmetic was reported by the summary and was not re-checked.
- **INI reading**: `CIniReader` reads `NFSMWHDReflections.ini`. [source]
- **Resolution patches** (always applied): writes road reflection X/Y at `0x6CFC26` and `0x6CFC2D`; writes mirror X/Y at `0x8F9008` and `0x8F900C`; writes a pointer for the front-end cubemap resolution at `0x6BD4FE`; jumps into caves at `0x6BCDEE`, `0x6BCE32`, `0x6BD542`, `0x6BD58F`, `0x6BD344`. [source]
- **ImproveReflectionLOD**: writes `0x00006002` at `0x6BFEBD` and `0x6BFE58`; jumps at `0x6BFE3D` to an LOD bypass cave; writes bytes at `0x4FAE9A`, `0x4FAEB0`, `0x6BFF21`, `0x7293DB`. [source]
- **RestoreDetails**: jumps to traffic-light, sunflare and detail-restoration caves at `0x6DE9F0`, `0x6DB260`, `0x6E6DAD`, `0x6DB0FE`, `0x6DE96D`; enables car shadows via a cave at `0x74E812`; level 2 also jumps at `0x6DE9E3` to a particles cave. [source]
- **RestoreCars**: jumps at `0x6DE7CA` to a cubemap-car draw cave. [source]
- **ForceEnableMirror**: NOPs at `0x6CFB72` and `0x6CFBC5`; writes `0x14EB` at `0x595DDD`. [source]
- **RestoreShaders / CubemapBrightnessFix**: writes at `0x6DABD6`; jumps at `0x6D7289` to a road-reflection removal cave; a call at `0x6DE6B0` to an envmap brightness cave when the brightness fix is on. [source]
- **RestoreShadows**: jump at `0x6DE88E` to a shadow-mirror cave. [source]
- **RestoreVisualTreatment**: jump at `0x6DE9F8` to a visual-treatment cave that runs two render passes only in-game (state value 6 at `0x925E90`). [source]
- **RestoreWaterReflections**: NOPs at `0x6BFEF7` and `0x6DE508`. [source]
- **RemoveLastGenEffects**: NOP at `0x504C56`; with level 2 also NOP at `0x738207`. [source]
- **OptimizeRenderDistance**: jumps at `0x6E73A1` and `0x6DB5BE` to render-distance and skybox caves. [source]
- **VehicleReflectionBrightness**: writes float `0.5 x value` at `0x6C77CE` and float `1.0 x value` at `0x6C77D6`. [source]
- **ExpandMemoryPools**: writes values at `0x5F7396`, `0x5F73B2`, `0x665FDC`, `0x8F5790`, `0x8F7EF0`, and four `2048000` values in the `0x5009xx` range. [source]
- **DisableBackFaceCulling**: byte write `0x00` at `0x8FAE44`. [source]
- **FE car model fix**: jump at `0x6DEF3D` to a cave that flips the backface-culling flag at `0x8FAE6C` around the front-end car draw call. [source]
- **Render-state globals**: many addresses are read or written inside the caves (for example `0x919730`, `0x925E90`, `0x91606C`, `0x9195E4`, `0x982A6C`). Their meaning is only partly stated in the source. [source]
- **DirectX**: none. The DLL makes no Direct3D calls; it works at the level of game code and data. [source]
- **Win32 APIs used**: `GetSystemMetrics` (screen size), `GetModuleHandleA`, `MessageBoxA`. [source]
- **Helper library**: `injector.hpp` for memory patching (`WriteMemory`, `MakeJMP`, `MakeCALL`, `MakeNOP`). The `injector` folder also contains `CPatch.h`, `IniReader.h` and `ini_parser.hpp`. [source, folder listing]

Note on the file listing: the NFSMW folder contains `dllmain.cpp`, `settings.h`, `assembly.h`, `stdafx.*`, `resource.h`, `targetver.h` and VS project files. The listing says "View all files", so it may be incomplete.

## 4. Configuration

File: `NFSMWHDReflections.ini`, read by `CIniReader`. Defaults below are from the `dllmain.cpp` summary [source]. The `settings.h` summary listed the same keys but did not give defaults, so the values here are single-sourced.

**[RESOLUTION]**

| Key | Default |
|---|---|
| AutoRes | 1 |
| CubemapScale | 1.0 |
| MirrorScale | 1.0 |
| RoadScale | 1.0 |
| OldGPUCompatibility | 0 |
| FECubemapRes | 256 |

**[CUSTOM RESOLUTION]** (used when AutoRes is 0)

| Key | Default |
|---|---|
| CubemapRes | 256 |
| MirrorResX | 256 |
| MirrorResY | 256 |
| RoadResX | 320 |
| RoadResY | 240 |

**[GENERAL]**

| Key | Default |
|---|---|
| CubemapBrightnessFix | 1 |
| ImproveReflectionLOD | 1 |
| ForceEnableMirror | 1 |
| RestoreShaders | 1 |
| RestoreShadows | 1 |
| RestoreVisualTreatment | 1 |
| RestoreDetails | 1 |
| RestoreCars | 0 |
| RestoreBackfaceCulling | 1 |
| RestoreWaterReflections | 0 |
| RemoveLastGenEffects | 2 |
| OptimizeRenderDistance | 1 |
| VehicleReflectionBrightness | 1.0 |

**[EXTRA]**

| Key | Default |
|---|---|
| ExpandMemoryPools | 1 |
| DisableBackFaceCulling | 0 |

Behaviour when AutoRes = 1 [source]: the custom values are overridden from screen metrics. Cubemap resolution = screen height; mirror = (screen width / 2) x (screen height / 6); road = full screen width x height. Scale factors are then applied. If OldGPUCompatibility = 1, the cubemap size is rounded down to a power of two.

The README does not list defaults. Keys named in the README but not found in the source summary: RealFrontEndReflections, RealisticChrome. Naming may differ between titles; not verified.

## 5. How we could implement it in nfs-mw

Licence constraint: the repository is GPL-3.0. Copying or translating its code into nfs-mw (which is MIT OR Apache-2.0) would create a licence conflict. Do not copy code, comments, address tables or the INI key table verbatim. The only permitted use is the behaviour and facts, written as a spec in `docs/` and implemented from that spec (spec-first, clean-room). The facts used here are the user-visible feature list and the public README. The memory-address tables are NFS:MW-specific facts of the game build; they must not be used as an implementation source (they come from this GPL code and its reverse-engineering), so we derive our own data from the install (see `docs/` process in `docs/licensing.md`).

**Does the mechanism apply to a clean-room rewrite?** No. The ASI loader, DLL injection, `DllMain` build check, code caves, NOP/JMP patching and fixed-address writes are specific to running patches inside the original executable. nfs-mw is a rewrite that reads the install and renders itself. Those parts do not apply and should not be ported. What does apply is the set of reflection behaviours the mod exposes, implemented in our renderer.

**Behaviours to specify** (each becomes a spec item):

1. Reflection target resolution per reflection type: cubemap, mirror, road reflection, front-end cubemap. Inputs: screen size, a per-type scale factor, an explicit override, and a "round to power of two" option for older GPUs.
2. Auto-resolution rule: derive the resolution from screen metrics. Specify the formula and the rounding, with edge cases (very small or very large screens, odd widths).
3. Reflection LOD raise: a setting that changes the level-of-detail used for reflected geometry.
4. Render distance for reflected content: separate distances for mirror, cubemap and skybox reflections.
5. Reflection content restoration flags: which object classes appear in reflections (traffic lights, sun flares, car shadows, particles, cubemap cars, water, visual-treatment passes). Each is a boolean or level in config.
6. Vehicle reflection brightness: one scalar that scales vehicle reflection intensity.
7. Chrome reflectivity and front-end vehicle reflections (README features with unclear mapping): specify only after the feature's behaviour is confirmed from observation in the game, not from this code.
8. Configuration file: a plain INI (or TOML) with the keys in section 4, validated and clamped. Defaults to be chosen by us, not copied.

**Data or formats needed**

- Screen size and render-target dimensions from the window/renderer.
- Our own NFS:MW game-data facts: which render passes and object classes exist in the install. These come from the data readers in `libs/` / `crates/` and from observation, not from this repository's addresses.
- An INI reader and a settings struct.

**Natural home**

- Engine-generic: resolution and scale math, power-of-two rounding, render-distance settings, INI reading. Put these in `libs/` (Bevy-free for the maths and config; a renderer backend crate, if any, is an optional leaf per ADR 0004).
- NFS:MW-specific: which reflection passes, object classes and game-state flags exist, and any per-version layout table. Put these in `crates/` (for example the Most Wanted crate). `crates/` depends on `libs/`, never the reverse.
- Remember the 500-line-per-file limit and the early-return style required by AGENTS.md.

**Risks**

- Licence contamination: the strongest risk. Keep the spec written from behaviour and the public README; record in the spec that this repo was consulted only for feature names and that no code was copied.
- Build dependence: the original mod depends on the exact game executable (its build check). Our renderer must not depend on any fixed address.
- Visual correctness: several effects (visual treatment, shadow mirror, envmap brightness) interact with game render state. Each needs observation in the real game before it is specified as a behaviour.
- Compatibility: the README reports the mod works with other patches (Widescreen Fix, Unlimiter, and so on). Our rewrite would not run those patches; they are not a requirement.
- Scope: five titles in the repo; only Most Wanted is in scope for nfs-mw. Other titles are out of scope.

## 6. Open questions and what could not be verified

- The latest commit date is not visible in the fetched summaries. Activity is therefore only "69 commits, 1 open PR, 2026 footer".
- README features RealFrontEndReflections and RealisticChrome have no matching key in the NFSMW source summary. Mapping not verified.
- The `dllmain.cpp` summary gives the build-check arithmetic (base + AddressOfEntryPoint + (0x400000 - base) against 0x7C4040). This was not checked against the raw file line by line.
- `settings.h` summary lists keys without defaults; the defaults come from `dllmain.cpp` only.
- Address tables were summarised, not checked against a reading of the raw file. They should not be used anyway (see section 5).
- The source folders NFSC, NFSPS, NFSU and NFSU2 were not read.
- The optional HD cubemap (MEGA link) and the nfsmods.xyz download were not fetched. No binary was downloaded.
- The GPL-3.0 licence text was confirmed by a summary of the LICENSE file; the wording of individual clauses was not reviewed.
- Compatibility claims with other mods were not tested.

## 7. Sources

- https://github.com/AeroWidescreen/NFSHDReflections (repository page; summary of README link, folder listing, licence label, activity counts)
- https://raw.githubusercontent.com/AeroWidescreen/NFSHDReflections/main/README.md
- https://raw.githubusercontent.com/AeroWidescreen/NFSHDReflections/main/LICENSE
- https://github.com/AeroWidescreen/NFSHDReflections/tree/main/SOURCE (folder listing)
- https://github.com/AeroWidescreen/NFSHDReflections/tree/main/SOURCE/NFSMW (folder listing)
- https://github.com/AeroWidescreen/NFSHDReflections/tree/main/SOURCE/includes (folder listing)
- https://raw.githubusercontent.com/AeroWidescreen/NFSHDReflections/main/SOURCE/NFSMW/dllmain.cpp
- https://raw.githubusercontent.com/AeroWidescreen/NFSHDReflections/main/SOURCE/NFSMW/settings.h
- https://raw.githubusercontent.com/AeroWidescreen/NFSHDReflections/main/SOURCE/NFSMW/assembly.h
