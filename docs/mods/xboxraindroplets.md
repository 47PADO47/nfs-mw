# XboxRainDroplets

Repo: https://github.com/ThirteenAG/XboxRainDroplets

## Summary

XboxRainDroplets is an ASI plugin set that brings the Xbox-style rain droplet effect (originally from SkyGFX) to a list of other PC games and two emulators. The README describes it as an enhanced implementation of that effect for other games. One build is published per supported game. Each build hooks the game's rendering and weather code, simulates droplets on screen during rain, and draws them through the game's own Direct3D or OpenGL/Vulkan device. The project is MIT licensed and includes a Need for Speed: Most Wanted (NFS:MW) entry point. For the nfs-mw rewrite, the useful part is the behaviour (droplets that react to rain intensity, camera motion and tunnels), not the hook code.

## Metadata

| Field | Value |
|---|---|
| Repo | ThirteenAG/XboxRainDroplets |
| Author | ThirteenAG (copyright line in licence: "Copyright (c) 2023 ThirteenAG") |
| Type | .asi plugin (one per game) plus a wrapper DLL source. Not a standalone tool. |
| Licence | MIT License (stated in `license` file and in README) |
| Language | C++ (30 `.cpp` game entry points, `xrd` core, `shaders`, `dxsdk`, `console`) [source, file listing] |
| Target game build | NFS:MW entry is `source/NFSMostWanted.XboxRainDroplets.cpp` (config key `nfsmw`). README does not name a PC build or version. Hook patterns are matched by byte signature, not by fixed version. [source, via summarised fetch] |
| Latest visible activity | Commit "improve visuals" (0813d15) by ThirteenAG, 2026-10-09. The commits page shows 35 commits on the first page, with more pages. [source, commits page] |

## What it does

Each feature below is taken from the README unless marked otherwise.

1. Rain droplets on the screen during rain. The effect is a screen-space overlay of droplets, not in-world geometry. [README; NFSMostWanted entry hooks the rain draw path, source]
2. Droplets follow the rain intensity of the game. When the game's rain value is zero, the droplets are cleared. [source, RainIntensityHook and OnScreenRain update hook]
3. Droplets are suppressed inside tunnels. The code checks the game's "am I in a tunnel" query and forces rain intensity to zero. [source, View_AmIinATunnel check]
4. Droplets reset on device reset (for example, on window or display changes). [source, ResetHook]
5. Renderer choice per API: Direct3D 8 and 9, Vulkan, OpenGL and the emulator's Thin3D API. Direct3D 8 games use the D3D9 renderer when a wrapper such as d3d8to9 is present, otherwise the native D3D8 path. [README]
6. Precompiled shaders for native D3D and Vulkan. A build script rebuilds only changed shaders. [README]
7. A headless test mode checks resource setup at start-up and after reset. [README]
8. Live config reload: the plugin's INI is read at start-up and again whenever the file is written, so values can change mid-game. [README]
9. Supported titles: GTA III, Vice City, San Andreas (classic and Definitive Editions), GTA IV, NFS Underground 2, NFS Most Wanted, NFS Carbon, Max Payne 2 and 3, Splinter Cell series (5 entries), Mafia, Manhunt, Scarface, Driv3r, Driver: Parallel Lines, True Crime NYC, King Kong, Saints Row 2, plus PPSSPP and PCSX2F emulator builds. [README, file listing]

## How it does it

Claims are tagged: [source] verified in the code, [README] stated only in the README, [inferred] my reading.

Note on [source] tags: the code was read through a summarising fetch, not as raw text, and no addresses or byte patterns were checked against a binary. Treat the offsets below as leads, not facts, until someone checks them against the source directly.

- ASI loader: each game entry is a DLL with an exported `InitializeASI()`. `DllMain` calls it on attach unless the Ultimate ASI Loader is already present (`IsUALPresent()`). Init is registered with a callback handler and runs once (`std::call_once`). [source, NFSMostWanted entry; README step 2]
- Graphics target: NFS:MW entry defines `XRD_ENABLE_D3D9` and includes `xrd/xrd.h`. [source]
- Config: an `CIniReader` is created with an empty name and `WaterDrops::ReadIniSettings()` is called. The INI file name and key names are not visible in the entry file and are not stated in the README. [source; README says "copy the .ini file"]
- Device reset hook (inline patch): matches a byte pattern near a device-pointer load and calls `WaterDrops::Reset()`. [source]
- Rain intensity hook (inline patch): matches an x87 float store and writes rain intensity into a game-state field. Sets intensity to zero in tunnels, otherwise copies a second field. [source]
- Render hook (call-site patch): on a particular game-flow status value, calls `WaterDrops::Render()` then chains to the original function. [source]
- Update hook (JMP patch): replaces `OnScreenRain::Update`. When the flow status is the active value, it:
  - clears drops when the time step is zero;
  - builds up, right and forward vectors and a position from the camera;
  - calls `Xrd::Init` and `WaterDrops::Process()`;
  - zeroes the rain intensity so the original droplets do not draw. [source]
- Disabling the stock droplets: a pointer used by the original draw path is redirected to a static zero value. [source]
- Address lookup: some addresses are hard-coded for NFS:MW (game flow status, tunnel check) and then overwritten by pattern scans. The frame time is read from a pattern-matched address commented as the game's last frame time. [source]
- Renderer backends: native D3D and Vulkan use precompiled shaders; OpenGL and Thin3D compile embedded shader source at resource prep. [README]
- Wrapper for D3D8 games: `XboxRainDropletsWrapper.cpp` in `source/`. Purpose beyond the README is not verified. [source file listing; [inferred] that it is the D3D8 wrapper path]

## Configuration

- INI file name: not stated in README; not visible in the NFS:MW entry. [unverified]
- Key names and defaults: not listed in README. The entry file reads them via `WaterDrops::ReadIniSettings()`, whose body was not read. No defaults are known. [unverified]
- Reload: INI is re-read when written (README). [README]
- D3D8 renderer selection: automatic, no INI key (README). [README]

## How we could implement it in nfs-mw

Licence note: this repo is MIT licensed, so porting code would be legally possible with attribution. The project rule still applies: spec first, and we do not copy the hook code. The spec below is written from behaviour only. Check `docs/licensing.md` before any code is taken; that file was not read for this page.

Behaviours to specify (behaviour only, no addresses):
- Droplets are a screen-space layer, drawn over the 3D scene during rain.
- Spawn and motion depend on the rain intensity value and the frame time step.
- Droplets are cleared when intensity is zero.
- Intensity is forced to zero in tunnel zones.
- Droplets respond to camera motion (the source derives vectors from the camera).
- All droplet state is reset on device or display reset.
- Config values are reloaded from a file without restart.
- A no-rain or disabled setting draws nothing and costs nothing.

Data and formats needed:
- Rain intensity per frame (from the world/weather state, not from a memory address).
- Tunnel flag per frame (from the zone or track data in nfs-mw's data crate).
- Frame time step.
- Camera transform per frame.
- A small INI or TOML settings file, with its keys specified by us.
- Droplet textures and shaders, authored or generated by us. Do not reuse the repo's shader files without a licence review.

Natural home:
- `libs/` (engine-generic): a droplet simulation (pure Rust, no Bevy, no NFS:MW names) and a renderer-agnostic description of the overlay pass. If a Bevy renderer backend is added, it is a leaf crate under `libs/` per ADR 0004.
- `crates/` (NFS:MW-specific): the mapping from NFS:MW world state (weather, tunnels, camera) to the generic inputs, and the NFS:MW config defaults. `crates/` depends on `libs/`, never the reverse.
- Files must stay under 500 lines; split the simulation, the renderer and the config reader into separate modules.

Risks:
- Hook-based approach does not apply. The rewrite reads game state directly, so there is no ASI loader, no byte patterns and no in-process patching. This part of the repo is not portable and not needed.
- Tunnel and rain semantics are not documented in the repo. The spec must define them from our own observation of the game.
- Shader and texture reuse: the repo's shaders are MIT but still need review and attribution before reuse.
- Per-frame cost on low-end hardware; the effect must be toggleable.
- Visual match with the original Xbox effect is subjective. Define acceptance by side-by-side captures, not by copying code.

## Open questions and what could not be verified

- Exact INI file name, key names and defaults (not visible in the entry file; body of `ReadIniSettings` not read).
- Which NFS:MW PC build the hook patterns were written for (patterns are signature-based; no version string read).
- Full contents of `source/xrd/`, `source/shaders/` and `source/includes/` (only listings seen; `includes` lists FileWatch.hpp, ModuleList.hpp, callbacks.h, gameref.hpp).
- Whether `XboxRainDropletsWrapper.cpp` is the D3D8 wrapper; inferred only.
- Commit count and full history (first page only, 35 commits).
- Whether the releases page lists NFS:MW-specific release tags and their dates (not fetched).
- Byte patterns and offsets were taken from a summarised read and have not been checked against source text.
- No binary was downloaded or run, as instructed.

## Sources

- https://github.com/ThirteenAG/XboxRainDroplets
- https://github.com/ThirteenAG/XboxRainDroplets/tree/master/source
- https://raw.githubusercontent.com/ThirteenAG/XboxRainDroplets/master/license
- https://raw.githubusercontent.com/ThirteenAG/XboxRainDroplets/master/readme.md
- https://raw.githubusercontent.com/ThirteenAG/XboxRainDroplets/master/source/NFSMostWanted.XboxRainDroplets.cpp
- https://github.com/ThirteenAG/XboxRainDroplets/commits/master
- https://github.com/ThirteenAG/XboxRainDroplets/tree/master/source/includes
