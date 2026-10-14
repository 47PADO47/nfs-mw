# NFSMWHDReflections (sniten mirror of Aero_'s mod)

Status: research note. Read-only review of the repository via WebFetch. Nothing was cloned, downloaded or executed.

The repository is a C++ ASI plugin for the 2005 PC release of Need for Speed: Most Wanted (v1.3 English). It patches the running game to raise the resolution of road, vehicle and rearview-mirror reflections, to restore some reflection detail that the stock build drops, and to fix a few rendering bugs. Settings live in an INI file next to the DLL. The code patches fixed game addresses by writing jumps into "code caves" and by overwriting data. It does not hook DirectX.

## Metadata

| Field | Value | Evidence |
|---|---|---|
| Repository | https://github.com/sniten/NFSMWHDReflections | GitHub repo page, GitHub API |
| Owner / author | Repo owner `sniten`. The mod's creator is `Aero_` (credited in README and commit history). | README; commit log (author Aero_) |
| Fork status | Not a fork (`fork: false`, no parent) | GitHub API |
| Type | ASI plugin (DLL loaded by an ASI loader) | Source: `NFSMWHDReflections/dllmain.cpp` |
| Licence | GNU GPL v3.0 (GitHub API `license` field and README "GPL-3.0 license") | GitHub API; README |
| Language | C++ (Visual Studio project `.vcxproj`). GitHub reports no language value. | repo file listing |
| Target game build | NFS:MW v1.3 English, PC. `DllMain` checks that the entry point is `0x7C4040` and shows an error box otherwise. | Source (via fetch summary) |
| Stars / forks / open issues | 1 / 0 / 0 | GitHub API |
| Latest visible activity | Last push 2020-06-12. Last commit 2020-06-11 ("Added VehicleReflectionBrightness, updated ini & readme", Aero_). `updated_at` shows 2025-09-30 (metadata only; no code change visible). | GitHub API; commit list |
| Description (GitHub) | "Control the resolution of reflections in Need for Speed: Most Wanted (2005)" | GitHub API |
| Commit count | 33 (per repo page) | README-level page text |

## What it does

Each feature below is a user-visible behaviour. The evidence column names where it is stated.

1. Reflection resolution. Lets the user set a width and height for reflections (ResolutionX / ResolutionY). If both are 0 the mod uses the desktop size. [README "Settings"; INI; source]
2. Per-target scale. RoadScale, VehicleScale and MirrorScale multiply the reflection resolution for road reflections, vehicle reflections and the rearview mirror. [README; INI; source]
3. Higher reflection and blur resolution. HDReflections (code key `HDRoadReflections`) raises the road reflection resolution. HDReflectionBlur raises the blur buffer size. [README; source]
4. Reflection level of detail. ImproveReflectionLOD raises the LOD used by reflections. Level 2 also changes the full rearview-mirror LOD and road reflection LOD. [README; source]
5. Missing geometry. GeometryFix restores geometry that is missing from reflections, including frontend reflections and world detail. [README; source]
6. Mirror detail. RestoreDetails adds lights, car shadows, and (at level 2) particle effects to the rearview mirror. Traffic lights and sun flares are also restored in the code. [README; source]
7. Reflection shaders and barrier animation. RestoreShaders enables reflection shader effects, removes the road reflection on the mirror, adjusts shadow opacity, and enables barrier animations. The code credits osdever for the information. [README; source]
8. Render distance. OptimizeRenderDistance caps reflection render distance (100 m for most objects, 192 m for the mirror) and changes skybox distance. [README; source]
9. Vehicle reflection brightness. VehicleReflectionBrightness scales the ambient, diffuse and specular colour of vehicle reflections. [README; source]
10. Slot pool expansion. ExpandSlotPool raises an internal object-slot limit so that objects stop disappearing in scenes with many polygons. [README; source]
11. Mirror tint. MirrorTint replaces the rearview mirror colour with an RGB value. [README; source]

## How it does it

The loader is a standard `DllMain`. The code does not call any ASI loader API. Loading relies on an ASI loader (the README tells the user to install Ultimate-ASI-Loader's `dinput8.dll`). [README; source]

Entry point. On process attach, `DllMain` computes `0x400000 + AddressOfEntryPoint`. If the value equals `0x7C4040` it calls `Init()`. Otherwise it shows an error box and returns FALSE. This is a version gate, not a signature scan. [source, via fetch summary]

Patch primitives. The code uses a vendored `injector` library (`includes/injector/`, with `hooking.hpp`, `assembly.hpp`, `calling.hpp`, `utility.hpp`, `gvm/`). Its helpers `MakeJMP`, `MakeNOP` and `WriteMemory` write machine code and data at fixed addresses. [source, via fetch summary; library files were listed but not read]

Method. The mod is a set of hard-coded patches at fixed addresses in the game's `.text` and data sections. Three kinds of change are made:

- Direct data writes. Floats and dwords are written to fixed addresses. Examples: road reflection width and height written from `ResX * RoadScale` and `ResY * RoadScale`; vehicle reflection height from `ResY * VehicleScale`; mirror width and height from `ResY * MirrorScale` and `(ResY / 3) * MirrorScale`; slot-pool limit `0xFA000` written at four addresses. [source, via fetch summary]
- Byte patches. Single bytes are changed (for example `0xEB` short-jump opcodes at `0x4FAE9A`, `0x6C69AE`, `0x729479`, `0x6DABD6`). These turn off checks or enable code paths. [source, via fetch summary]
- Jump redirects into code caves. About 25 `MakeJMP` calls send execution from a game function into a naked assembly routine in the mod. The routine performs the new logic, then jumps back to a fixed address in the game. Examples: `0x6BD4FC` to `RestoreFEReflectionCodeCave`, `0x6BCE6E` to `HDReflectionBlurCodeCave`, `0x6E7120` to `MirrorRGBCodeCave`, `0x6E73A1` to `RenderDistanceCodeCave`. [source, via fetch summary]

The full list of addresses, cave names and exit points is in the source file `NFSMWHDReflections/dllmain.cpp` (see Sources). The fetch returned a summary of that file, not the file text, so the addresses in this note have not been checked line by line against the source. [inferred on exact values]

Game-state reads. Some caves read game globals and object fields directly, for example an object pointer at `0x982A20`, a type value at `0x925E90`, and the object type at `[edi+4]`. The caves compare type IDs (types 1, 3, 0x12 to 0x16) to choose behaviour per object class. [source, via fetch summary]

DirectX. `<d3d9.h>` is included, but no Direct3D 9 calls are made and no D3D hooks are installed. All effects come from patching game code and data. [source, via fetch summary]

Configuration file. `CIniReader` reads `NFSMWHDReflections.ini` by relative path. The file is looked up from the game's working directory. [source, via fetch summary]

Known risks in the code (from the fetch summary):

- The mod assumes NFS:MW v1.3 English only. Any other build is refused at start-up.
- `ResolutionX` and `ResolutionY` are declared in the INI and read into `ResX` and `ResY`, but the README and INI both describe them as the settings. The fetch summary says the code never uses the INI keys directly; it is unclear from the summary whether `0` falls back to `GetSystemMetrics` in the code path. [inferred]
- Address `0x6DE990` appears as a jump-back target (`FlareAndShadowLayeringCodeCave`) and as a hook site (`DestroyedObjectCodeCave`). The summary flags that these two uses may conflict. [source, via fetch summary; not verified]
- `if (ImproveReflectionLOD >= 2)` statements have no braces, so only the next line is conditional. The fetch summary calls this out. [source, via fetch summary]

## Configuration

The INI file shipped in the repository (`NFSMWHDReflections.ini`) sets these values. Defaults in the code (the `Default` column) can differ from the shipped INI.

| Section | Key | Shipped INI value | Code default | Meaning |
|---|---|---|---|---|
| RESOLUTION | ResolutionX | 0 | 0 | Horizontal reflection resolution; 0 = desktop (per INI comment) |
| RESOLUTION | ResolutionY | 0 | 0 | Vertical reflection resolution |
| RESOLUTION | RoadScale | 1.0 | 1.0 | Road reflection scale |
| RESOLUTION | VehicleScale | 1.0 | 1.0 | Vehicle reflection scale |
| RESOLUTION | MirrorScale | 1.0 | 1.0 | Rearview mirror scale |
| GENERAL | HDRoadReflections (README: HDReflections) | 1 | 1 | Raise reflection resolution |
| GENERAL | HDReflectionBlur | 1 | 1 | Raise blur resolution |
| GENERAL | ImproveReflectionLOD | 2 | 2 | 0 = off, 1 = basic, 2 = full LOD |
| GENERAL | GeometryFix | 1 | 1 | Restore missing reflection geometry |
| GENERAL | RestoreDetails | 1 | 1 | 1 = lights and car shadows, 2 = also particles |
| GENERAL | RestoreShaders | 1 | 1 | Mirror shaders and barrier animation |
| GENERAL | OptimizeRenderDistance | 1 | 1 | Cap reflection render distance |
| GENERAL | VehicleReflectionBrightness | 1.0 | 1.0 | Vehicle reflection intensity |
| EXTRA | ExpandSlotPool | 1 | 1 | Raise object slot pool |
| EXTRA | MirrorTint | 1 | 1 | Enable mirror tint |
| EXTRA | MirrorTintR / G / B | 150 / 150 / 150 | 150 / 150 / 150 | Mirror tint colour, 0 to 255 |

The shipped INI also has a comment that refers to the ThirteenAG WidescreenFixesPack and lists default and Xbox 360 values for the mirror tint. [source, via fetch summary]

Naming mismatch: README and INI use `HDReflections`; the code reads `HDRoadReflections`. The fetch summary of the INI and README does not settle which name a user must write. [open question]

## How we could implement it in nfs-mw

Clean-room rule. The repository is GPL-3.0 and our project is MIT OR Apache-2.0. Code from this repo must not be copied, translated or paraphrased line by line into nfs-mw. The only permitted input is behaviour and facts, written as our own spec. The steps below describe behaviour to specify. They do not reuse any address table, cave, or code from this mod.

Does it apply to a clean-room rewrite? Partly.

- Loader-specific hacks do not apply. nfs-mw does not run the original game binary, so there is no ASI loader, no entry-point check, no `MakeJMP`, no code caves, and no data patching at fixed addresses. These parts have no equivalent and should not be ported.
- The user-facing goals do apply. Reflection resolution scales, LOD, mirror and vehicle reflection behaviour, render distance and mirror tint are all things our renderer and game code will need to control in their own way.

Behaviours to specify (spec-first, to write in docs/specs/ before any code):

1. Reflection render target size: independent scales for road, vehicle and mirror targets, derived from one base resolution (with a documented default).
2. Blur target size tied to the reflection size.
3. Reflection LOD: a three-level setting (off, basic, full) with exact meaning per level written in the spec.
4. Render distance caps for reflected objects and skybox.
5. Vehicle reflection brightness as a single multiplier on ambient, diffuse and specular colour.
6. Mirror tint as an RGB value with a documented range.
7. Object-class rules (which object types are reflected, which are culled), written from observed game behaviour, not from the mod's code.
8. Slot or object-pool limits: how many scene objects are supported, and what happens above the limit.

Data and formats needed:

- A config file format for the settings above. Recommended: a TOML or INI file read by a `libs/` config loader, with the same key names as the original to ease user migration. Naming should be documented, not copied from the mod's code.
- Per-object-class tables for reflection and render-distance rules, chosen per game version. These belong in a game-specific data table, not in code.
- Observed reference images or frame captures (taken from our own runs of the game we own) for checking reflection output. No copied assets.

Natural home in nfs-mw:

- libs/ (engine-generic, Bevy-free except renderer backends per ADR 0004): reflection render-target sizing, blur size calculation, LOD selection, render-distance cap as a generic renderer setting, mirror tint as a material parameter, and the config loader.
- crates/ (NFS:MW-specific): which NFS:MW object types are reflected, the vehicle brightness rules, mirror placement and mirror shape, and any per-version data tables. Crates depend on libs. libs never depends on crates and must not contain NFS:MW names, paths or defaults.

Risks:

- Licence contamination. Reading this source closely and then writing our own code risks copying its structure. Mitigation: write specs from observed behaviour and our own measurements; keep the mod's code out of the working tree; record in the spec that it came from behaviour only.
- Wrong assumptions about game internals. The mod's behaviour depends on how the original renderer orders its passes. Our rewrite will have a different pipeline, so exact behaviour must be measured, not assumed.
- Performance. Higher reflection and LOD settings cost GPU time. Specify a budget and a test scene.
- Config naming drift. If we keep the original key names, document clearly that they are our own spec, not a copy.

## Open questions and what could not be verified

- Raw file text was not read. WebFetch returned summaries for `dllmain.cpp`, the INI, the README and the `includes/` files. The address table in this note comes from those summaries and should be treated as unverified until the raw files are read by someone with the licence in mind (for analysis only, not for copying).
- `includes/injector/` (`hooking.hpp`, `assembly.hpp`, `calling.hpp`, `utility.hpp`, `gvm/`), `CPatch.h`, `IniReader.h`, `ini_parser.hpp`, `stdafx.h` and the project files were listed but not read.
- Shipped binary releases were not checked (the README links to a different account, AeroWidescreen, for releases). No binaries were downloaded, per instructions.
- Ultimate-ASI-Loader's role is inferred from the README. The code does not call the loader.
- The `ResolutionX` / `ResolutionY` behaviour is unclear (see Known risks).
- `HDReflections` (README, INI) versus `HDRoadReflections` (code) is an unresolved name mismatch.
- The README download link points to `github.com/AeroWidescreen/NFSMWHDReflections`, not to this repository. This is a different account. The repo here is a mirror or copy; whether it is maintained or the same code is unknown.
- Licence: the GitHub API and README both state GPL-3.0. No LICENSE text was read beyond the file's existence. Treat the GPL-3.0 statement as the claim, not a verified text.
- `updated_at` (2025-09-30) is later than the last push (2020-06-12). The cause is not visible from the data gathered. No code change after 2020 was seen.
- Whether the mod works on builds other than v1.3 English: it refuses to start on others, per source summary.
- Stars and forks are very low (1 star, 0 forks). Community reports on stability were not searched for.

## Sources

- https://github.com/sniten/NFSMWHDReflections (repo page; file listings)
- https://github.com/sniten/NFSMWHDReflections/tree/master/includes (listing)
- https://github.com/sniten/NFSMWHDReflections/tree/master/includes/injector (listing)
- https://github.com/sniten/NFSMWHDReflections/tree/master/NFSMWHDReflections (listing)
- https://raw.githubusercontent.com/sniten/NFSMWHDReflections/master/README.md
- https://raw.githubusercontent.com/sniten/NFSMWHDReflections/master/NFSMWHDReflections.ini
- https://raw.githubusercontent.com/sniten/NFSMWHDReflections/master/NFSMWHDReflections/dllmain.cpp
- https://api.github.com/repos/sniten/NFSMWHDReflections (metadata: licence, dates, counts)
- https://api.github.com/repos/sniten/NFSMWHDReflections/commits?per_page=3 (recent commits)
- Referenced but not read: https://github.com/AeroWidescreen/NFSMWHDReflections/releases ; https://github.com/ThirteenAG/Ultimate-ASI-Loader/releases ; https://github.com/ExOptsTeam/ ; https://github.com/osdeverr

Local references: /c/Users/PADO/github/nfs-mw/docs/architecture.md, /c/Users/PADO/github/nfs-mw/docs/licensing.md, /c/Users/PADO/github/nfs-mw/docs/decisions/0004-swappable-renderers.md
