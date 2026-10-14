# Ultimate ASI Loader

## Summary

Ultimate ASI Loader (ThirteenAG) is a Windows proxy DLL that loads custom `.asi` plugin libraries into a game process. It is the de facto plugin loader for many PC games, including NFS:MW mods. The loader poses as a system DLL the game already imports (such as `dinput8.dll`, `version.dll` or `d3d9.dll`), forwards the real exports to the original DLL, and loads `.asi` files from the game folder and a few conventional subfolders. It also offers an "update folder" overlay that replaces game files without touching originals, a C++ snippet loader, crash reporting, and a d3d8to9 bridge. For nfs-mw, the loader itself is not something to rebuild; the useful ideas are the config layering, the file overlay, and the crash report format.

## Metadata

| Field | Value |
|---|---|
| Repo | https://github.com/ThirteenAG/Ultimate-ASI-Loader |
| Author | ThirteenAG |
| Type | `.asi` plugin loader (proxy DLL), with plugin SDK and C++ snippet support |
| Licence | MIT, per the `license` file (copyright "2023 ThirteenAG"). The README has no licence section. |
| Language | C++ (`source/loader`, `source/plugins`); `premake5.lua` build script. The GitHub language bar was not visible to the reader. |
| Target game build | Not specific to NFS:MW. Generic loader for many games; README lists many proxy names and game examples (GTA V, RDR2, GTA San Andreas UWP). NFS:MW-specific use not verified. |
| Latest visible activity | Commit "switch wndmode compat hook method" (commit 71ca8ae), dated Oct 10, 2026 (per the commits page as read on the reading date). |
| Stars / forks | About 2.3k stars, 154 forks (per the repo page as read). |

## What it does

Each feature below is taken from the README (`readme.md`) unless stated otherwise.

1. **Proxy DLL loading of `.asi` plugins.** Drop the loader DLL into the game directory under a name the game already loads. Any `.asi` file in the game root, `scripts`, `plugins` or `update` is loaded. [README: INSTALLATION, USAGE]
2. **Many proxy names.** Win32 and Win64: `d3d9`, `d3d10`, `d3d11`, `d3d12`, `dxgi`, `dinput8`, `dsound`, `version`, `wininet`, `winmm`, `winhttp`, `xinput1_1` to `xinput1_4`, `xinput9_1_0`, `xinputuap`. Win32 only: `d3d8`, `ddraw`, `dinput`, `msacm32`, `msvfw32`, `xlive`, `binkw32`, `bink2w32`, `vorbisFile`. Win64 only: `binkw64`, `bink2w64`. [README: INSTALLATION]
3. **Chain-loading the original DLL.** Renaming the original to `<name>Hooked.dll` makes the loader forward to it. [README: INSTALLATION; source: proxy/proxy.cpp]
4. **Bink wrapper.** For Bink proxies, the original `binkw32.dll` is renamed to `binkw32Hooked.dll`. [README: INSTALLATION]
5. **Xidi integration.** If `Xidi.32.dll` or `Xidi.64.dll` sits next to the loader, the loader can replace matching input exports with Xidi's. [README: INSTALLATION; source: proxy/proxy.cpp `FillFromXidi`]
6. **Update folder overlay.** Files in an `update` folder (or other configured folder) override game files without changing originals. Deleting the folder reverts. If several folders are configured, a selector dialog auto-picks the first after 10 seconds. Zip archives in a `packages` folder can supply content without extraction. `GetOverloadPathA` and `GetOverloadPathW` expose the active path. [README: UPDATE FOLDER]
7. **C++ snippets (`.cxx`).** Plain C++ source files are compiled at game start, with `Init()` and `Shutdown()` entry points. Optional libraries: injector, Hooking.Patterns, safetyhook. Errors are reported with file and line; hot reload via `CxxHotReload`. [README: C++ SNIPPETS; examples in `examples/snippets`]
8. **Plugin SDK.** A plugin is a DLL renamed to `.asi` that exports `InitializeASI`. Plugins can detect the loader via an `IsUltimateASILoader` export. Demo plugins in `source/plugins`: MessageBox, PluginTemplate, VirtualFiles, FrameLimiter, ExeUnprotect. [README: WRITING PLUGINS]
9. **Crash reports.** A `CrashDumps` folder receives, per crash, a log, a minidump (`.dmp`) and a zip of both. The log includes plugin attribution, module offsets, IDA addresses, byte patterns, registers and loaded modules. [README: CrashDumps; source: main.cpp crash setup]
10. **d3d8to9 bridge.** `UseD3D8to9=1` routes Direct3D 8 calls to d3d8to9 (requires the loader named `d3d8.dll`, 32-bit). [README: D3D8TO9; source: proxy/proxy.cpp]
11. **Legacy windowed mode (x86 only).** An empty `wndmode.ini` next to the 32-bit loader enables it; the file is auto-filled on first run. [README: WINDOWED MODE]
12. **UWP install guide.** A 16-step procedure for Windows Store games. [README: Using with UWP games]

## How it does it

Tags: [source] = seen in code (read through a summarising fetch of the raw file, see Open questions); [README] = stated only in the README; [inferred] = my reading.

- **Entry point.** `DllMain` calls `Init` on `DLL_PROCESS_ATTACH`. The `reserved` argument distinguishes `LoadLibrary` (null) from static import (non-null). Detach restores imports on `FreeLibrary`. [source: source/loader/main.cpp]
- **Init order.** `InitSelf`, then `LoadSettings`, then `vfs::InitKeys(exeDir)`, then optional debug log, then crash dump setup (`crashdump::Install`), then loading. [source: main.cpp]
- **Load timing.** If `dontLoadFromDllMain` is false, `startup::LoadEverything()` runs from `DllMain`. Otherwise `startup::HookEntryPoints()` (or `HookEntryPointsInline()` when loaded through a protection stub) hooks the game's entry flow, and plugins load once game code runs. [source: main.cpp; startup/ files `startup.cpp`, `startup.hpp`, `imports.hpp` exist; the hook bodies were not read] [README: `LoadFromAPI` names GTA V and RDR2 using `GetSystemTimeAsFileTime`]
- **Proxy forwarding.** The loader calls `LoadLibrary` on the real DLL from `SystemDirectory()` (or `<stem>Hooked.dll` if present). `Fill` resolves each export with `GetProcAddress` (by name or ordinal) into a slot, and generated thunks jump through those slots. Special cases: `xinput1_1` to `1_3` fall back to `xinput1_4` then `xinput9_1_0`; `vorbisFile` and `xlive` have custom loaders. [source: source/loader/proxy/proxy.cpp]
- **Ordinal table.** Combines a static `ordinals.inl` with a scan of the system DLL's named exports, mapped as a data resource so no DllMain runs. [source: proxy.cpp]
- **Thread safety.** `LoadOriginalLibrary` uses an SRW lock and a three-state flag and guards re-entry from the loading thread. [source: proxy.cpp]
- **Plugin attribution in crash logs** and the crash dump pipeline, `vfs::PassThroughForCrash`. [source: main.cpp for the callback name; crash/ folder exists]
- **Virtual file system.** `vfs::` module handles the update-folder overlay and `VirtualFileServer` (folder `source/VirtualFileServer` exists). [source: folder listing; the overlay internals were not read] [README: UPDATE FOLDER]
- **Plugin discovery.** Folders scanned, `LoadRecursively`, and the `.asi` filter are documented in the README. The code that does the scan was not read. [README: USAGE] [inferred: plugin scan lives in `source/loader/startup/` or `plugins/`]
- **Hooks and patches on game functions.** None verified by name. The C++ snippet system uses Hooking.Patterns and safetyhook to hook game code, and that is the path for game-specific hooks. [README: C++ SNIPPETS]
- **Config files.** `global.ini` (and `<dllname>.ini`) placed next to the loader or in the plugin folders; later files in the search order win. [README: USAGE]

No address, signature or function hook into NFS:MW was verified. The repo is generic.

## Configuration

Keys from the README table under `[GlobalSets]`, with defaults as stated. Code confirms only the fields `debugLog`, `dontLoadFromDllMain`, `loadFromAPI`, `disableCrashDumps`, `crashDumpFullMemory`, `crashDumpZip`, `crashDumpMaxReports`; the exact key-to-field mapping lives in `LoadSettings()`, which was not read.

| Key | Default | Meaning (paraphrased from README) |
|---|---|---|
| `LoadPlugins` | 1 | 0 disables plugin loading |
| `LoadFromScriptsOnly` | 0 | 1 skips the loader's own folder |
| `LoadRecursively` | 1 | Includes direct subfolders of plugin folders |
| `LoadExtraPlugins` | `modloader\modloader.asi` | Plugins loaded first, `\|`-separated |
| `DontLoadFromDllMain` | 1 | 0 loads plugins from `DllMain` |
| `LoadFromAPI` | empty | Ties loading to a named API call (GTA V, RDR2 use `GetSystemTimeAsFileTime`) |
| `UseD3D8to9` | 0 | Enables d3d8to9 |
| `Direct3D8DisableMaximizedWindowedModeShim` | 0 | 32-bit only, Direct3D 8 shim |
| `ModernUI` | 1 | 0 uses standard Windows dialogs |
| `DebugLog` | 0 | Writes `<loader name>.log` |
| `CxxHotReload` | 0 | Reloads C++ snippets on save |
| `DisableCrashDumps` | 0 | Crash dump switch |
| `CrashDumpZip` | 1 | Zip crash output |
| `CrashDumpFullMemory` | 0 | Full-memory minidump |
| `CrashDumpMaxReports` | 10 | Max crash reports kept |

`[FileLoader]` section: `OverloadFromFolder`, default `update`.

## How we could implement it in nfs-mw

Ground rule: the repo is MIT-licensed (per the `license` file), so licence-wise we may read it, but this page records behaviour only. Do not copy code or the snippet/plugin SDK. Spec first: write the behaviour as a spec under `docs/` before any code.

**Does the loader itself apply?** No. The loader is a process-injection and DLL-proxy technique for an existing binary we do not control. The nfs-mw rewrite is a native Rust program that reads the install from `D:\` and does not run the original executable, so a proxy DLL, export forwarding, entry-point hooking and import restoration have no clean-room equivalent and should not be built. This is a loader-specific hack and is out of scope.

**Behaviours worth specifying (clean-room):**

1. Layered config: a base config plus per-folder overrides, later sources win, a fixed search order, and documented defaults. Maps to the `global.ini` model.
2. Overlay file system: a virtual file lookup that checks an `update/` folder first, then the install, with optional zip packages. Lets mods replace assets without touching the install. Need a precise definition of lookup order, case rules and path normalisation.
3. Plugin discovery: scan fixed folders, optional recursion, an extension filter, and a deterministic load order list.
4. Crash reports: on a fatal error, write a log (version, config, loaded modules, plugin attribution), an optional minidump, and optionally zip them. Rust can use a panic hook and a minidump crate, but this is for later milestones.
5. Plugin API version handshake: a plugin declares an API version, and the host refuses incompatible ones. Equivalent of `InitializeASI` plus `IsUltimateASILoader`.

**Data and formats needed:** config file grammar (INI-like, `[Section]` and `Key=Value`), overlay manifest (optional), zip package layout, crash report schema. All to be documented in `docs/` before implementation.

**Natural home:**
- Config layering, overlay VFS lookup and crash report writer are engine-generic: `libs/` (for example a config and VFS crate). `libs/` must stay Bevy-free.
- NFS:MW-specific paths (install detection under `D:\`, `update` folder naming defaults, which files count as assets) belong in `crates/`. `crates/` may depend on `libs/`, never the reverse.
- No NFS:MW names, paths or defaults in `libs/`; pass them in from the caller.
- Crate-level file limit: 500 lines per file.

**Risks:**
- Scope creep: building a loader for a native rewrite is wasted work.
- Overlay precedence bugs: case sensitivity and symlinks on Windows, and zip path traversal (`..`) must be rejected.
- Config ambiguity: duplicated keys across layers need one documented rule.
- Mod trust: plugin code runs with full process privileges. In a rewrite, if plugins are supported, they need a sandboxing or trust decision that is not decided here.
- Licence: reading MIT code is fine; copying it into a repo without attribution is not. Specs only.

## Open questions and what could not be verified

- The raw `README.md` path returned 404; the lowercase `readme.md` was read instead.
- The GitHub repo page was read via a summarising fetch, not as raw bytes. Claims tagged [source] come from summaries of `main.cpp` and `proxy/proxy.cpp`. Exact code was not inspected line by line.
- Not read: `startup/startup.cpp`, `proxy/` thunks, `vfs/`, `crash/`, `cxx/`, `plugins/`, `LoadSettings()`. Plugin discovery, hook bodies and VFS lookup order are unverified.
- Language share and the exact number of commits were not confirmed on the reading date.
- No NFS:MW-specific use or compatibility was verified. Whether any NFS:MW mod in `docs/mods/` relies on this loader was not checked.
- "Latest visible activity" is the Oct 10, 2026 commit shown on the commits page at read time; the page may not be the newest.
- The `ModernUI` and some config key semantics come from the README table, not from code.

## Sources

- https://github.com/ThirteenAG/Ultimate-ASI-Loader (repo page; description, stars, forks, licence badge)
- https://raw.githubusercontent.com/ThirteenAG/Ultimate-ASI-Loader/master/README.md (returned 404)
- https://raw.githubusercontent.com/ThirteenAG/Ultimate-ASI-Loader/master/readme.md (README)
- https://raw.githubusercontent.com/ThirteenAG/Ultimate-ASI-Loader/master/license (MIT licence text)
- https://github.com/ThirteenAG/Ultimate-ASI-Loader/tree/master/source (folder listing)
- https://github.com/ThirteenAG/Ultimate-ASI-Loader/tree/master/source/loader (folder listing)
- https://github.com/ThirteenAG/Ultimate-ASI-Loader/tree/master/source/loader/startup (folder listing)
- https://github.com/ThirteenAG/Ultimate-ASI-Loader/commits/master (commit dates)
- https://raw.githubusercontent.com/ThirteenAG/Ultimate-ASI-Loader/master/source/loader/main.cpp (entry point and init)
- https://raw.githubusercontent.com/ThirteenAG/Ultimate-ASI-Loader/master/source/loader/proxy/proxy.cpp (proxy forwarding)
