# MirrorHook

MirrorHook is a Windows DLL, packaged as an `.asi` file, that hooks the Direct3D 9 and DirectInput 8 interfaces of some Need for Speed games and exposes an extension API. Other modules register callbacks that run at fixed points in the D3D9 frame and the DI8 device poll. The project also draws a small ImGui status overlay. It was written in C++14 for Visual Studio, and its README calls it a wrapper for extending the existing D3D9 and DI8 implementation. The code in the repository covers two game builds: NFS: Most Wanted and NFS: Carbon.

Status: small, inactive since April 2018. Useful to us as a reference for how a community mod hooked the renderer and input of the 2005 game, not as code to reuse.

## Metadata

| Field | Value |
|---|---|
| Repository | https://github.com/berkay2578/MirrorHook |
| Author | berkay2578 (copyright line: "Copyright (c) 2017-2018 Berkay Yigit") |
| Type | Native `.asi` plugin (DLL with `.asi` extension in Release|Win32), exposing an extension API |
| Licence | MIT, stated in `LICENSE` [source] and the README mentions no other licence |
| Language | C++14, Visual Studio solution (`MirrorHook.sln`) |
| Target game builds | NFS: Most Wanted and NFS: Carbon (both named in code as enum values `MostWanted` and `Carbon`) [source]. The README says only "some NFS games". |
| Latest visible activity | Last commit Apr 12, 2018, "Minor fixes and changes, F9 to toggle menu by default" [source: commits page] |
| Activity | 16 commits, 7 stars, 7 forks, 3 watchers, 2 open issues (as shown on the repo page) [source: repo page] |
| Platform | Win32 only. Target SDK is Windows 7 SDK with the `v141_xp` toolset, output stated to run on Windows XP SP1 and later [README] |

## What it does

Each feature below is backed by the README or by the source file named.

1. **Hooks the D3D9 device of the game.** The hook point list is BeginScene, EndScene, Reset, TestCooperativeLevel and BeginStateBlock. The README says these hooks keep the mod running [README: "D3D9" section]. Source: `MirrorHook/dllmain.cpp`.
2. **Waits for the game to create its D3D9 device before hooking.** Source: `dllmain.cpp`, `Init` polls the device pointer every 100 ms [source].
3. **Hooks keyboard and mouse input through DirectInput 8.** Only `GetDeviceState` is hooked, and the README says only mouse and keyboard are supported [README: "DirectInput8" section]. Source: `dllmain.cpp`, `DI8Extender::Init`.
4. **Extension API for other modules.** External code can register callbacks for the hook points. Parameters use `void*` so callers need not include DirectX headers [README]. Exported: `AddExtension`, `SetTestCooperativeLevelExtension` on the D3D9 side and `AddExtension` on the DI8 side [source: `dllmain.cpp`].
5. **Re-hooks after the game rewrites its hooks.** `hkBeginStateBlock` unhooks all entries, calls the original, then re-hooks the same indices [source: `dllmain.cpp`].
6. **Status overlay (ImGui).** Shows "MirrorHook v1.1", the GitHub URL, the author name, extender readiness and extension counts. It is visible for 300 frames by default. F9 toggles it to stay visible [source: `dllmain.cpp`, commit "F9 to toggle menu by default"].
7. **Custom cooperative-level result.** An override callback can return a value that replaces the result of `TestCooperativeLevel` [source: `dllmain.cpp`]. Exported as `SetTestCooperativeLevelExtension`.
8. **Exposes the D3D9 device, window handle and DirectInput objects** to extensions through `GetD3D9Device`, `GetWindowHandle`, `GetDirectInput8A` and `GetDirectInputDevice8A` [source: `dllmain.cpp`].

## How it does it

Mechanism, with the tag for each claim: [source] verified in code, [README] stated only in the README, [inferred] my reading where the code is missing.

### Loading and startup
- Load path: the file is a DLL renamed to `.asi`, and the project's post-build step copies it into a game `scripts` folder [source: `MirrorHook.vcxproj`, Release|Win32 post-build event]. That `scripts` folder is the usual place for an ASI loader to pick up plugins, so the game is probably loaded through an external ASI loader [inferred]. The repository contains no loader code and the README does not name one.
- `DllMain` on `DLL_PROCESS_ATTACH` only calls `DisableThreadLibraryCalls` and returns [source: `dllmain.cpp`]. Nothing starts at load time.
- Startup is driven by the exported `PrepareFor(MirrorHook::Game)`. It runs once, calls `Memory::Init()`, picks addresses for the game, and starts a thread that runs the D3D9 and then the DI8 initialisers [source: `dllmain.cpp`].
- Module base: `Memory::Init()` stores `GetModuleHandle(0)`, the base of the host executable [source: `Memory.cpp`].

### Hard-coded addresses (absolute, per build)
These are absolute addresses in the game's data segment. They are the only game-specific values in the code [source: `dllmain.cpp`, `PrepareFor`]:

| Build | DirectInput8 pointer | D3D9 device pointer |
|---|---|---|
| MostWanted | `0x582D14` | `0x582BDC` |
| Carbon | `0x71F5CC` | `0x6B0ABC` |

Addresses are turned into pointers with `Memory::makeAbsolute()`, which adds the module base [source: `Memory.cpp`]. The code does not state which game binary version these addresses match. Matching them against a specific executable is an open question (see section 6).

### Direct3D 9 hooks
- Vtable indices hooked on the device object [source: `dllmain.cpp`]: 3 TestCooperativeLevel, 16 Reset, 41 BeginScene, 42 EndScene, 60 BeginStateBlock.
- Swaps vtable entries, not code bytes. `VTableHook::Hook<Type>(index, fn)` reads the original slot, writes the replacement and stores the original in a map keyed by index [source: `VTableHook.hpp`].
- Each `Hook` call unprotects memory with `VirtualProtect` (`PAGE_EXECUTE_READWRITE`) and restores it afterwards [source: `Memory.cpp`].
- `hkBeginScene` runs registered callbacks, handles the F9 key, starts ImGui for DX9 once, and begins an ImGui frame [source].
- `hkEndScene` runs callbacks and draws the overlay [source].
- `hkReset` runs before and after callbacks and recreates ImGui device objects [source].

### DirectInput 8 hooks
- `DI8Extender::Init` polls the DI8 pointer, then runs `EnumDevices` with `DI8DEVCLASS_ALL` and `DIEDFL_ATTACHEDONLY` [source].
- For each keyboard and mouse found, it patches vtable index 9 (`GetDeviceState`) [source].
- The hook calls the original `GetDeviceState` and then runs registered callbacks [source].
- `isExtenderReady` is set only after both a keyboard and a mouse are found [source].

### Files
- `dllmain.cpp`: entry point, hook install, extension API, overlay [source].
- `VTableHook.hpp`: vtable swap helper [source].
- `Memory.cpp` / `Memory.h`: `VirtualProtect` wrapper and module-base helper [source].
- `D3D9Types.h`, `DI8Types.h`: type declarations so the API does not need DirectX headers [source, file names only; contents not read].
- `inc/`: a subfolder whose contents were not read.
- `stdafx.h`, `stdafx.cpp`, `targetver.h`: standard Visual Studio support files [source, names only].

## Configuration

The code reads no config files [source: `dllmain.cpp`]. There are no user-facing keys, no INI or JSON files, and no command-line flags in the code read so far.

Runtime settings that exist in code (not user-editable):
- Overlay duration: 300 frames by default; F9 sets it to -1 (persistent) [source].
- Device poll interval: 100 ms [source].
- Game selection: the `MirrorHook::Game` enum passed to `PrepareFor` (`MostWanted` or `Carbon`) [source].

Build-time setting: the Release|Win32 post-build event copies the built `.asi` to two hard-coded developer paths on the author's machine (`D:\Oyunlar\NFS Most Wanted\scripts\` and `D:\Oyunlar\NFS Carbon Collectors Edition\scripts\`). The README says to edit or remove this step before building [README, "How to set up"] [source: `MirrorHook.vcxproj`].

## 5. How we could implement it in nfs-mw

Rule for this project: the repo has an MIT licence, so reuse is legally possible, but our policy is spec-first and clean-room. We describe behaviour and facts here and do not copy the code.

### Applicability
- The feature is a loader-specific hack. An ASI plugin hooks a running game from inside its process. Our rewrite does not run inside the original game. It reads the install from `D:\` and renders with its own backends (see `docs/architecture.md`). The vtable patching, hard-coded game addresses, post-build copy to a `scripts` folder, and `DllMain` tricks do not apply to a clean-room rewrite. Those parts should be recorded as "not applicable".
- The behaviours that do transfer are: a per-frame callback point for overlays, an input-state hook point, and a small status overlay for debugging. Our equivalents are our own renderer and input layers, not a wrapper around the game's D3D9 device.

### Behaviours to specify (spec-first)
1. Frame hook points: a "before frame" and "after frame" point where registered code can run. Equivalent to BeginScene and EndScene, but in our renderer loop.
2. Device reset point: a callback when the render device is lost and recreated, so overlays can rebuild resources. Equivalent to Reset.
3. Input snapshot point: a callback that receives keyboard and mouse state each poll. Equivalent to the DI8 GetDeviceState hook.
4. Extension registry: named extensions can be registered and removed, with a stable callback signature that avoids engine types for third-party code. In Rust this is a trait object or `fn` pointer table, not `void*`.
5. Debug overlay toggled by a key, with a timed default display. This is UI only, not a physics or gameplay change.
6. Hook order and re-install: our renderer should not need re-hooking. Specify that the registry survives device resets.

### Data and formats needed
- Game build identification: a layout table keyed by version, as the code uses per-game addresses. In our project this belongs in a per-version layout table chosen from the install data, not in code constants (see AGENTS.md rule on game names in `libs/`).
- No file formats are read by the mod. Nothing to parse.

### Natural home
- Engine-generic parts go in `libs/`: the frame and device-reset hook registry, the input snapshot callback, and the extension registry. They must not name NFS:MW, and must be Bevy-free unless they live in a renderer backend crate (ADR 0004).
- NFS:MW-specific parts go in `crates/`: the debug overlay content and any per-version layout table. `crates/` depends on `libs/`, never the reverse.
- The `.asi` loader, vtable patching and hard-coded addresses have no home in our tree. Do not add them.

### Risks
- Callbacks run on the render or input thread and the repo has no synchronisation (`Extensions` vectors and map are shared without locks [source: `dllmain.cpp` observations]). Our registry must specify thread ownership up front.
- Hard-coded addresses break on any other build. Our layout tables must be tied to a verified build, not guessed.
- Unprotecting code or vtable memory is unsafe and fragile. Avoid it in the rewrite.
- The overlay panel is a debug aid. Do not let it change game state.
- Windows XP support is a constraint of the original target only. Our targets are set in `docs/architecture.md`; do not inherit XP support.

## 6. Open questions and what could not be verified

- Which ASI loader the mod expects, and whether the game is loaded through it. Not in the repo. The README does not name one.
- Which game binary versions the addresses `0x582D14`, `0x582BDC`, `0x71F5CC` and `0x6B0ABC` match. The code gives no version string beyond the overlay text "MirrorHook v1.1".
- Contents of `inc/`, `D3D9Types.h`, `DI8Types.h`, `Memory.h`, `stdafx.h`, `targetver.h` and `VTableHook.hpp` details beyond what is summarised above. `Memory.h` was not read directly.
- The README "Notes" section was not read.
- The README's "Development dependencies" section was summarised by the fetch tool. The June 2010 DirectX SDK and the VS2015+ C++14 requirement are as stated there [README].
- The contributor count and total commit count (16 per repo page). The commits page did not show a total.
- The repository has 2 open issues. Their content was not read.
- Whether the mod has been tested against either game in recent years. Not stated.
- The `Memory.cpp` open/restore helper keeps one saved protection value, so nested calls can lose the original protection. This is a code observation, not tested.
- The `hkBeginStateBlock` re-hook logic relies on the game's own behaviour; not verified.
- A raw-file fetch of the README at `main` branch returned 404. The `master` branch was used. Whether `master` is the default branch was not confirmed.

## 7. Sources

- https://github.com/berkay2578/MirrorHook (repo page: description, file list, licence link, stats)
- https://raw.githubusercontent.com/berkay2578/MirrorHook/master/README.md (README)
- https://raw.githubusercontent.com/berkay2578/MirrorHook/main/README.md (returned 404)
- https://github.com/berkay2578/MirrorHook/tree/master/MirrorHook (source folder listing)
- https://github.com/berkay2578/MirrorHook/blob/master/LICENSE (licence text, MIT, copyright line)
- https://raw.githubusercontent.com/berkay2578/MirrorHook/master/MirrorHook/dllmain.cpp
- https://raw.githubusercontent.com/berkay2578/MirrorHook/master/MirrorHook/VTableHook.hpp
- https://raw.githubusercontent.com/berkay2578/MirrorHook/master/MirrorHook/Memory.cpp
- https://raw.githubusercontent.com/berkay2578/MirrorHook/master/MirrorHook/MirrorHook.vcxproj
- https://github.com/berkay2578/MirrorHook/commits/master (commit dates and messages)
