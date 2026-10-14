# NFSMW_SunSet (NFS: Most Wanted "Sun Set" mod)

## Summary

NFSMW_SunSet is a community graphics mod for Need for Speed: Most Wanted (2005, PC). It is a
native C++ DLL, loaded into the game process, that patches game code in memory through the
Injector library. It adds a day-to-night cycle, custom world and car shaders, road reflections,
rain on cars, dynamic lighting, and motion blur changes. It ships HLSL/FX shader sources, a
shader compiler project, and a .nfsms mesh and texture-notes folder. Its install path relies on
TexWizard and optional DXVK. The DLL refuses to load unless the game executable matches one
specific build. Source is public, but the licence situation is contradictory (see section 1).

## 1. Metadata

| Field | Value |
|---|---|
| Repo | https://github.com/ArchieGoldmill/NFSMW_SunSet |
| Author | ArchieGoldmill (repo owner; no other author named in files read) |
| Type | Native ASI-style game mod (DLL with in-process code patching); includes shader sources |
| Licence | Conflicting. The `LICENSE` file is the GNU GPL v3 full text (verified). The README disclaimer says the repo is "provided for educational purposes", allows "private, personal, or internal builds", and forbids public redistribution, republishing or re-release of the code or derivative builds. Treat as NOT cleanly open for reuse. |
| Language | C++ (DLL, Visual Studio solution `SunSet.sln`, vcpkg), HLSL/FX shaders (`fx/*.fx`), a shader compiler project |
| Target game build | "v1.3 English speed.exe", 6,029,312 bytes (from the error message in `dllmain.cpp`) [source] |
| Latest visible activity | Not visible in the fetched pages. A sidebar figure of 460 commits was reported, but no commit date was readable. Not verified. |

## 2. What it does

Features as stated in the README, each cross-checked against the source where possible.

- **Day-to-night cycle** [README] with time-of-day settings. Evidence in code: `Config.h` has
  `ForceTime`, `TimeUpdateRate`, `SunRise`, `SunSet`, `LightsOn`, `LightsOff`, `SunAzimuth`,
  `WetTime`, `DryTime`, `RandomStartupTime`, `RealTime` [source]. Time-of-day logic lives in
  `TimeOfDay.h` and `Time.h` (names only; bodies not read).
- **Custom world and car shaders** [README]. Evidence: `fx/` contains `world.fx`, `car.fx`,
  `worldnormalmap.fx`, `worldprelit.fx`, `worldreflect.fx`, `car_rain.fx`, `skybox.fx`, `water.fx`,
  `tree.fx` and others [source, file listing only]. Shader bodies were not read.
- **Upgraded road reflections** [README]. Evidence: `Reflection.h`, `RealFeReflections` config
  flag [source, name only].
- **Rain effects on cars** [README]. Evidence: `car_rain.fx`, `Rain.h`, `HeavyRain.h`,
  `RainConfig` (`LightTime`, `HeavyTime`, `LightningTimeOut`, `RainCloudsTexture`) [source, names].
- **Dynamic lighting** [README; spelled "ligting" in the README]. Evidence: `DynamicLights.h`,
  `WorldLights.h`, `SpotLightConfig.h`, `Spotlight.h`, `LightsBuffer.h`, `MaxLights`,
  `LightCellSize` [source, names].
- **Motion blur improvements** [README]. Evidence: `MotionBlur.h`, `BlurMinSpeed`, `BlurMaxSpeed`,
  `BlurDepth` [source, names].
- **Extra post effects** [source, names only]: `Bloom.h`, `GodRays.h`, `Exposure.h`,
  `DepthPrePass.h`, `PostEffects.h`, `CarBrakeGlow.h`, `NosEffect.h`, `CarVinylMask.h`,
  `WindowGlowPower`, `DisableSunFlare`, `Tonemapping`, `MSAAx8`.
- **Custom meshes** [source, names]: `CustomMeshes.h`, `CustomMeshesConfig.h`, `mod/exhaust.nfsms`.
- **In-game editor and console** [source, names]: `Console.h`, `UIEditor.h`, `UIConfig.h`,
  `UIWeather.h`, `UISpotlights.h`. The `EDITOR` build macro is commented out in `dllmain.cpp`,
  so the editor is disabled in the code as read [source].
- **Skip front end** [source]: `g_Config.SkipFE` is copied to `Game::SkipFE` in `Init()`.

## 3. How it does it

### 3.1 Loader model

- The DLL is a Windows DLL with a `DllMain` entry point [source, `SunSet/dllmain.cpp`].
  It is not loaded through an ASI loader in the code read; how the DLL gets into the game process
  is not shown in the files read [inferred: an ASI-style loader or a renamed d3d9 proxy, not verified].
- On `DLL_PROCESS_ATTACH`, `DllMain` finds the host executable's entry point and compares it
  to a constant (`0x7C4040`) to confirm the build. On mismatch it shows a `MessageBoxA` naming
  "v1.3 English speed.exe" (6,029,312 bytes) and returns `FALSE` [source].
- `Init()` calls `InitConfig()`, optionally `InitConsole()`, `InitShaderLoader()`, and
  `InitHooks()`. The editor init path is compiled out [source].
- Links `d3d9.lib` and `d3dx9.lib` [source, from the read of `dllmain.cpp`] (Direct3D 9 target).
  No direct D3D calls were read in `dllmain.cpp`; D3D hooks are not confirmed.

### 3.2 Code patching

The injector header `SunSet/Injector/injector.hpp` is a memory-patching helper. Mechanisms seen:

- `injector::MakeJMP`, `injector::MakeCALL`, `injector::MakeNOP`, `injector::WriteMemory`
  [source].
- **Call-site redirection** [source, from `Hooks.h`]. Verified addresses:

  | Address | Operation | Replacement |
  |---|---|---|
  | 0x006DE3F5 | CALL | `SetuWorldCulling` |
  | 0x00731B63 | CALL | `DrawScenery` |
  | 0x006E0254 | JMP | `SetCurrentPassHook` (naked, calls `SetCurrentPass`, then jumps to 0x006E03C3) |
  | 0x006C60D9, 0x006DB2B0 | CALL | `SetEffectParams` |
  | 0x006C82B9 | CALL | `NormalizeLightVec` |
  | 0x006BFBB0 | NOP x6 then CALL | `SetDeviceParams` |
  | 0x006E2F73 | CALL | `SortRenderModels` |
  | 0x006E455A | CALL | `NormalizeSunDirForShadow` |

- **Raw byte and pointer writes** [source] at many addresses (for example 0x007429E0, 0x00738180,
  0x006C0167, 0x006DF57B). Some are conditional on config flags. Debug-only writes exist at 0x006C2206.
- **Entry-point jump** [source]: `injector::MakeJMP(0x006C343B, Reset)`, where `Reset` is a
  `__declspec(naked)` routine that saves registers, calls `ReleaseDirectResources()` (and
  `UI::Reset()` under `EDITOR`), and restores. The name suggests Direct3D resource release on device
  reset [inferred; call site not shown].
- **Memory reads** [source]: `RenderModelSize` at 0x006E0AD4, `RenderModel::List` at 0x006E0AEC.

### 3.3 Shaders

- Shader sources live in `fx/` (33 files, including `fx.sln`, `fx.vcxproj`) [source, listing].
- `ShaderLoader.h` and `ShaderCompiler/` (a compiler project; the file list was not read in
  full) suggest runtime shader loading with optional live reload (`LiveReload`, `ShaderLoader`,
  `ShaderCompiler` flags, `HK_ShaderReload` hotkey) [source, names only].
- `Fast shader compile.lnk` exists at the repo root [source, listing].

### 3.4 Config and data files

- The config is a C++ struct `Config` with nested `RainConfig` [source, `Config.h`]. Values are
  read by `InitConfig()`, whose body and file format were not read.
- `mod/` contains `readme.txt`, `window textures.txt`, and `exhaust.nfsms` [source, listing].
  Texture packs are merged via TexWizard's `TexWizard.json` [README].

## 4. Configuration

Keys are from `SunSet/Config.h`. The struct carries no default values in the header, and the
loader that sets them was not read, so defaults are **unknown** [source for keys; defaults not verified].

Float keys: `ForceTime`, `TimeUpdateRate`, `LightLodDistance`, `SunRise`, `SunSet`, `LightsOn`,
`LightsOff`, `LightCellSize`, `SunAzimuth`, `WetTime`, `DryTime`, `BlurMinSpeed`, `BlurMaxSpeed`,
`BlurDepth`, `CustomMeshDist`, `FlareDistance`, `WindowGlowPower`, `FilterPower`, `EditorScale`.

Int keys: `MaxLights`, `HK_ShaderReload`, `HK_ToggleEditor`.

Bool keys: `Console`, `ShaderLoader`, `ShaderCompiler`, `LiveReload`, `X360Effects`, `Editor`,
`SkipFE`, `RandomStartupTime`, `RealTime`, `CarVinylPaintFix`, `BrakeGlow`, `GodRays`, `Bloom`,
`RealFeReflections`, `WorldVertexColor`, `CarVertexColor`, `TunnelWetnessFix`,
`TunnelLightIntensity`, `DisableNightShadows`, `MSAAx8`, `Tonemapping`, `RandomWindows`,
`DisableSunFlare`, `RoadblockLights`, `Exposure`, `NosLight`, `NosEmmiter`, `NosFix`,
`WindowGlowOverride`.

Other: `WindowGlowColor` (vector4), `Filter` (hash field), `Rain` (nested `RainConfig`).

`RainConfig` keys: `DryTime`, `DryTimeRandom`, `LightTime`, `LightTimeRandom`, `HeavyTime`,
`HeavyTimeRandom`, `LightningTimeOut` (float); `RainCloudsTexture` (bool).

Hotkeys: `HK_ShaderReload`, `HK_ToggleEditor` (key codes not read).

No config file name or format is confirmed in the files read. Check `InitConfig()` before
relying on any of this.

## 5. How we could implement it in nfs-mw

Rule: the repo is GPL-3.0 on paper but the README forbids public redistribution. Under the
project's rules we do not copy its code, shaders, addresses-as-code, or config layout. We
document behaviour and facts only, and write our own spec first.

### 5.1 What does not apply to a clean-room rewrite

- The whole loader mechanism: `DllMain` entry-point check, `MakeJMP`/`MakeCALL` call-site
  patching, raw byte writes at fixed addresses, the naked-function hooks, and the `d3d9` build
  target. Our rewrite is a Rust program that reads the game install (`game-install` crate) and
  renders with its own backends (see docs/architecture.md). We do not patch the 2005 executable.
- Hard-coded addresses (0x006C343B, 0x006E0254, etc.) are specific to one build. We must not
  use them. Per-version layout tables in `crates/` are the project's approach.
- The `d3d9` proxy and ASI-style injection are not needed.

### 5.2 Behaviours to specify (spec-first, no code)

1. Time-of-day model: a day/night cycle with sunrise, sunset, lights-on and lights-off times,
   sun azimuth, and a forced-time option. Specify curves and units in a spec before code.
2. Weather-driven wetness: wet and dry durations, with optional randomisation.
3. Rain on cars: light, heavy, and lightning timing, plus a cloud texture option.
4. Dynamic lights: a per-cell light buffer with a maximum light count and a cell size.
5. Motion blur: min and max speed thresholds and a depth parameter.
6. Road reflections and car reflections: a reflection pass with a toggle.
7. Post stack: bloom, god rays, exposure, tone mapping, MSAA option. Each needs an explicit
   ordering and a default.
8. Night shadow and sun-flare toggles.
9. Car-specific effects: brake glow, NOS (nitrous) light and emitter, vinyl mask.

### 5.3 Data and formats needed

- Reference data from the install (world meshes, car models, textures) through `game-install`
  (libs/ for the format readers, crates/ for NFS:MW-specific parsing).
- Our own config format (a Rust config struct with serde), with defaults written in the spec.
- Shader source in our own language or HLSL. Any shader we write is our own work; do not
  port `fx/*.fx`.
- A test fixture set from the user's own install, not from the mod.

### 5.4 Natural home

- Engine-generic parts go in `libs/`: time-of-day math, light-cell buffer, post-effect pipeline,
  config parsing, shader-reload plumbing. No NFS:MW names in `libs/`.
- NFS:MW-specific parts go in `crates/`: which car meshes get which effects, the game's world
  lighting data, the 2005 weather and time defaults, the rain tuning values.
- `crates/` depends on `libs/`, never the reverse.

### 5.5 Risks

- Licence: even reading the code to write a spec risks contamination. Work from the README
  behaviour and our own measurements of the game; keep a clean-room record. Ask the user or the
  repo owner before any reuse.
- Visual match: the mod tunes against the original game's art. Our renderer must reproduce the
  same inputs (depth, normals, car vertex colours) first, or the effects will look wrong.
- Addresses and build: every hard-coded value is version-bound. Our layout tables must
  be data-driven.
- Scope: the feature list is large. Pick two or three behaviours for a first milestone.

## 6. Open questions and what could not be verified

- Licence conflict: `LICENSE` is GPL-3.0 full text, but the README disclaimer forbids public
  redistribution. Which one governs is unclear. Not resolved here; ask the repo owner.
- README text: the raw README fetch returned a condensed reconstruction (the fetch tool said
  so), not a verbatim copy. Feature wording and disclaimer quotes are from that version. The
  original `github.com/.../blob/master/README.md` was not re-checked.
- Branch: the raw URLs on `main` returned 404; `master` works. The repo's default branch is
  therefore probably `master` (inferred from raw fetch results).
- Latest activity date and commit count: not visible in the fetched pages.
- `InitConfig()` body: config file name, path, and defaults are unknown.
- `InitHooks()`, `InitShaderLoader()`, `InitConsole()`, `ReleaseDirectResources()`: bodies not read.
- How the DLL is loaded into the game (ASI loader, d3d9 proxy, or other): not shown.
- Shader bodies (`fx/*.fx`), `ShaderCompiler/` contents, and `mod/readme.txt` were not read.
- The `SunSet` file listing was read through a page that may be truncated ("View all files").
- The `ShaderCompiler/` folder listing returned 404 on `master` and was not read.
- The FX folder read was from the `master` tree page.
- Whether the mod works on any game build other than "v1.3 English speed.exe" is unknown.

## 7. Sources

- https://github.com/ArchieGoldmill/NFSMW_SunSet (repo page, `main` branch, read via WebFetch)
- https://raw.githubusercontent.com/ArchieGoldmill/NFSMW_SunSet/main/README.md (404)
- https://raw.githubusercontent.com/ArchieGoldmill/NFSMW_SunSet/master/README.md (condensed README)
- https://raw.githubusercontent.com/ArchieGoldmill/NFSMW_SunSet/main/LICENSE (404)
- https://raw.githubusercontent.com/ArchieGoldmill/NFSMW_SunSet/master/LICENSE (GPL-3.0 text)
- https://github.com/ArchieGoldmill/NFSMW_SunSet/tree/main/mod (404)
- https://github.com/ArchieGoldmill/NFSMW_SunSet/tree/master/mod (folder listing)
- https://github.com/ArchieGoldmill/NFSMW_SunSet/tree/main/SunSet (404)
- https://github.com/ArchieGoldmill/NFSMW_SunSet/tree/master/SunSet (folder listing)
- https://github.com/ArchieGoldmill/NFSMW_SunSet/tree/main/ShaderCompiler (404)
- https://github.com/ArchieGoldmill/NFSMW_SunSet/tree/main/fx (404)
- https://github.com/ArchieGoldmill/NFSMW_SunSet/tree/master/fx (folder listing)
- https://github.com/ArchieGoldmill/NFSMW_SunSet/tree/master/SunSet/Injector (folder listing)
- https://raw.githubusercontent.com/ArchieGoldmill/NFSMW_SunSet/master/SunSet/dllmain.cpp
- https://raw.githubusercontent.com/ArchieGoldmill/NFSMW_SunSet/master/SunSet/Config.h
- https://raw.githubusercontent.com/ArchieGoldmill/NFSMW_SunSet/master/SunSet/Hooks.h
