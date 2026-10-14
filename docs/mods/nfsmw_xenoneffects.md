# NFSMW_XenonEffects

A PC mod that ports the XenonEffects particle emitters (sparks and contrails, the "wind" effect behind cars) from Need for Speed: Carbon into Need for Speed: Most Wanted (2005, PC).

Status of this page: built from the repository README, the root file listing, `dllmain.cpp`, `NFSMW_XenonEffects.ini`, the LICENSE file and the commit history page. Source files were read through a summarising fetch, not a full local clone. Claims tagged [source] come from those files. Claims tagged [README] are only stated in the README. Claims tagged [inferred] are my reading.

## Metadata

| Field | Value |
|---|---|
| Repository | https://github.com/xan1242/NFSMW_XenonEffects |
| Author | xan1242 (Xan / Tenjoin) [source: dllmain.cpp header comment] |
| Type | Game mod: C++ DLL that patches the game executable at runtime, distributed with an ASI loader [inferred from dllmain.cpp `injector::MakeCALL` use and README install steps] |
| Licence | MIT License, "Copyright (c) 2023 Lovro Pleše" [source: LICENSE]. Note: the copyright line names a different person from the repo author. This is stated as found; I did not check why. |
| Language | C++ (Visual Studio solution `NFSMW_XenonEffects.sln`, `.vcxproj`) [source: root file listing] |
| Target game build | NFS:MW PC, "RELOADED No-CD 1.3" executable, MD5 `C0516B485065FABDD69579816B5DF763` [README] |
| Dependencies | Widescreen Fix (recommended) or Ultimate ASI Loader [README]. NextGenGraphics.asi optional [README]. |
| Latest visible activity | Last commit 2023-02-23; first commit listed 2023-02-21 (commits page, read 2026-10-14) [source] |
| Popularity | 51 stars, 4 forks, 1 open issue, 1 open pull request [README page stats] |

## What it does

Each feature below is user-visible in the game, with evidence.

1. **Sparks.** Particle sparks are emitted from emitters flagged for them, drawn on the car. [README: "Works: sparks, contrails"] [source: `EmitterSystem_Render_Hook`, `Emitter_SpawnParticles_Cave`, ini `[Elasticity]` keys `emsprk_line1` and `emsprk_line2`]
2. **Contrails.** Trails behind the car while driving fast. Activation speed is set by `ContrailSpeed` (metres per second, default 44.0). [source: ini `[MAIN]`]
3. **Contrails during NOS and in NIS.** `UseCGStyle` enables contrails when nitrous is active, with no fade-in. `NISContrails` enables them in cutscenes. Both default off. [source: ini comments]
4. **Shadowed effects.** `PassShadowMap` lets sparks and contrails receive shadows. Default off. [source: ini]
5. **Intensity control.** Contrail intensity is interpolated between `ContrailMinIntensity` and `ContrailMaxIntensity`. Spark intensity is `SparkIntensity`, and only applies to emitters with the `zContrail` property. [source: ini `[Limits]`]
6. **Density limits.** Two rate limiters keep effects near a target frame rate: contrails at 30 FPS, sparks at 60 FPS by default. Lower values give less density. [source: ini `[Limits]`, `LimitContrailRate`, `LimitSparkRate`]
7. **Particle cap.** `MaxParticles` (default 10000) caps generated particles. [source: ini]
8. **Per-emitter elasticity.** The bounce behaviour of named emitters (`emsprk_line1`, `emsprk_line2`) is set in the ini. Keys may be names or hex hashes. [source: ini `[Elasticity]`; dllmain notes names are hashed with `Attrib_StringHash32`]
9. **Particle bouncing.** Particles bounce off the world. [README: "Works: particle bouncing"]
10. **Pixel-shader rendering only.** The fixed-function pipeline is not supported, so older GPUs (README gives GeForce 2 as an example) do not work. [README]

## How it does it

### Loader and patch model

- The mod is a DLL that the game loads through an ASI loader (Widescreen Fix or Ultimate ASI Loader). [README]
- `Init()` writes call hooks into the game's code with `injector::MakeCALL` (injector library helper). [source]
- Hooks, with the original game address each one replaces:
  - `0x0050D43C` → `EmitterSystem_Update_Hook` (captures delta time) [source]
  - `0x006648BC` → `InitXenonEffects` [source]
  - `0x0066493E` → `sub_6DFAF0_hook` [source]
  - `0x006BD622` → `ReleaseRenderObj` [source]
  - `0x006DB293` → `sub_6CFCE0_hook` (Direct3D device reset) [source]
  - `0x006DF4BB` → `EmitterSystem_Render_Hook` [source]
- The `Init()` listing was cut off after the `// xenon eff` comment, so more hooks may exist. [source, partial]
- Code caves (custom code placed in game code space) are used for:
  - `Emitter_SpawnParticles_Cave` [source]
  - `CarRenderConn_OnRender_Cave`, entry `0x00750F48`, exits `0x00750F6D` and `0x00750F4E` [source]
  - `CarRenderConn_UpdateEngineAnimation_Hook`, wrapping original function at `0x00745F20` [source]
- Game functions the mod calls, with addresses [source]:
  - `FastMem_Alloc` `0x005D29D0`, `FastMem_Free` `0x005D0370`
  - `Attrib_Instance_MW` `0x00452380`, `Attrib_StringHash32` `0x004519D0`
  - `GetTextureInfo` `0x00503400`, `GameSetTexture` `0x006C68B0`
  - `EmitterSystem_UpdateParticles` `0x00508C30`, `EmitterSystem_Render` `0x00503D00`
  - `ParticleSetTransform` `0x006C8000`, `WCollisionMgr_CheckHitWorld` `0x007854B0`
  - `CreateResourceFile` `0x0065FD30`, `ResourceFile_BeginLoading` `0x006616F0`
- Game globals read or written [source]: `GLOBAL_D3DDEVICE` `0x00982BDC`, `FRAMECOUNTER_ADDR` `0x00982B78`, `GAMEFLOWSTATUS_ADDR` `0x00925E90`, `CURRENTSHADER_OBJ_ADDR` `0x00982C80`, `WORLDPRELITSHADER_OBJ_ADDR` `0x0093DEBC`.

### Direct3D use

- The mod does not hook D3D interface methods through a wrapper DLL. It calls the device and effect interfaces directly. [source]
- Device calls: `CreateVertexBuffer` (vtable offset +0x68), `CreateIndexBuffer` (+0x6C), `SetStreamSource`, `SetIndices`, `SetTexture`, `SetRenderState`, `DrawIndexedPrimitive`. [source]
- Vertex buffer calls: `Lock` (vtable +0x2C) and `Unlock`/release (+0x30). [source]
- `ID3DXEffect` calls: `Begin`, `BeginPass`, `CommitChanges`, `EndPass`, `End`. [source]
- Texture loading is either the game's own loader path or a direct D3D device path, chosen by `UseD3DDeviceTexture`. [source ini comment]

### Particle system

- Particle lists use EASTL containers (list and vector). Setup, generation, erase and per-frame bounce are in the mod. [README; source per dllmain summary]
- Emitters are read from the game's own particle data. The mod does not define a new file format. [inferred, no format code seen]
- Texture pack is `GLOBAL\XenonEffects.tpk` by default; it must contain a texture named `MAIN`. [source ini]

### Known bugs (from the top of `dllmain.cpp`) [source]

- Particles stay in the world after a restart; a reset path is needed.
- Contrails are overwritten by sparks at high spawn rates.
- Texture filtering affects drawing coordinates. If filtering is off, sparks draw off screen. This is why the README asks for it on.

## Configuration

Config file: `NFSMW_XenonEffects.ini`, placed in the game's scripts folder. [README; source]

| Section | Key | Default | Meaning |
|---|---|---|---|
| MAIN | TexturePack | `GLOBAL\XenonEffects.tpk` | Texture pack path; must hold texture `MAIN` |
| MAIN | UseD3DDeviceTexture | 0 | Load texture via D3D device, not the game loader. Use only if needed (e.g. NextGenGraphics) |
| MAIN | Contrails | 1 | Enable contrails |
| MAIN | NISContrails | 0 | Enable contrails in cutscenes |
| MAIN | PassShadowMap | 0 | Sparks and contrails receive shadows |
| MAIN | UseCGStyle | 0 | Contrails during NOS, no fade-in; intensity uses only MaxIntensity |
| MAIN | ContrailSpeed | 44.0 | Speed (m/s) at which contrails start |
| MAIN | LimitContrailRate | 1 | Contrail rate limiter on/off |
| MAIN | LimitSparkRate | 1 | Spark and other emitter rate limiter on/off |
| Limits | ContrailTargetFPS | 30.0 | Target FPS for contrail density; 0 to game FPS |
| Limits | SparkTargetFPS | 60.0 | Target FPS for spark density |
| Limits | ContrailMinIntensity | 0.1 | Minimum contrail intensity |
| Limits | ContrailMaxIntensity | 0.75 | Maximum contrail intensity |
| Limits | SparkIntensity | 1.0 | Spark intensity (only emitters with `zContrail`) |
| Limits | MaxParticles | 10000 | Cap on generated particles |
| Limits | NGEffectListSize | 500 | Size of the NGEffectList; affects visibility of particles |
| Elasticity | emsprk_line1 | 160.0 | Bounce elasticity for this emitter |
| Elasticity | emsprk_line2 | 120.0 | Bounce elasticity for this emitter |

Other config: `UseD3DDeviceTexture` should be on if NextGenGraphics.asi is used. Texture filtering must be enabled in the game's video settings. [README]

The README says to use the RELOADED No-CD 1.3 executable. [README]

## How we could implement it in nfs-mw

Rules applied: this repo is MIT licensed, so its code could legally be reused. The project process is spec-first (see `docs/licensing.md`, which I did not read). This section therefore describes behaviour and facts only, and does not propose copying code.

### Does it apply to a clean-room rewrite?

- Partly. The effect itself is a gameplay and rendering feature, so it applies.
- The loader-specific parts do not apply: ASI loader use, the No-CD executable, hard-coded game addresses, `injector::MakeCALL` hooks, code caves and vtable offsets. A Rust rewrite has no game binary to patch. These are dropped.
- The Direct3D call sequence does not apply. Our renderer is separate (see ADR 0004 on swappable renderers).
- The config file and tuning values are facts about the behaviour and can be specified.

### Behaviours to specify

1. Spark emitters: spawn rate, lifetime, and attachment to the car body.
2. Contrail trails: spawn when speed exceeds a threshold (default 44 m/s); fade rules; NOS-active variant; NIS variant off by default.
3. Intensity: linear interpolation between min and max, with the CG-style override.
4. Density limiting: a target-FPS-based rate limiter that scales spawn rate. Needs a defined behaviour when the frame rate is below target.
5. Particle cap: a hard maximum, with defined behaviour when reached (drop newest or oldest; not specified by the source).
6. Per-emitter bounce elasticity, keyed by emitter name.
7. Shadow reception as an option.
8. Reset on restart and on level or device reset. The source's bug list shows this was missing; the spec should require it.
9. Draw ordering: contrails must not be overwritten by sparks. The source does not give the intended order.

### Data and formats needed

- Emitter definitions from the game's particle data. The source reads these from the game; we need our own reader in the format crate for that data (see `docs/README.md` for the file format index, not read here).
- Car state inputs: speed, NOS state, cutscene state, shadow-map state. These come from the game-state model.
- Texture data for the effect texture. The source names a texture pack file and a texture called `MAIN`. Our spec must state the texture source. It must not copy the assets.
- Configuration keys above, with defaults, as a documented file.

### Natural home

- Engine-generic: a particle emitter and trail system, rate limiter, particle cap, and elasticity-based bounce. Goes in `libs/` (Bevy-free). Example: a `libs/particles` crate.
- NFS:MW-specific: which emitters are sparks, the names `emsprk_line1` and `emsprk_line2`, car-speed thresholds, NOS and NIS hooks, and the default values. Goes in `crates/` (for example a game crate for the NFS:MW feature set). `crates/` depends on `libs/`, never the reverse.
- The per-version tables rule applies: any NFS:MW-specific names and defaults stay out of `libs/`.

### Risks

- Feature parity risk: the original mod has known bugs (reset, overwrite, coordinate issues). Our spec should fix them rather than copy the workaround.
- Renderer dependency: the original requires pixel shaders. Our renderer must support the equivalent.
- Coordinate risk: the source says texture filtering changes drawn coordinates. In our renderer this would be a bug, not a setting, and must be tested.
- Rate-limiter tuning: values are tuned by the author for the 2005 game. Targets must be checked against our frame timing.
- Licence note: the LICENSE copyright line names a different person from the repo author. Confirm the licence holder before any reuse, even under MIT.
- Maintenance: last commit is 2023-02-23 and one pull request is open. Do not treat the mod as maintained.

## Open questions and what could not be verified

- `dllmain.cpp` is about 101 KB. The summarising fetch covered only the first 100,000 characters. The last 960 characters were not read. Any hooks or logic there are unverified.
- The `Init()` hook list was cut off. Further hooks may exist.
- The summarising fetch is not a verbatim read. Addresses and key names are as reported by the fetch and should be checked against the raw file before any use.
- The full README was not available verbatim. The README content above is paraphrased from the fetch.
- The GitHub repo page did not show a language breakdown, releases, or the full file list ("View all files" may hide entries).
- The repository's `.gitmodules` file suggests submodules exist. Their content was not read.
- No binary, release file, or game executable was downloaded or run. Nothing was executed.
- Whether the Widescreen Fix dependency is needed for the feature logic, or only for loading, was not verified.
- Licence compatibility with the nfs-mw licence and the spec-first process in `docs/licensing.md` was not checked. `docs/licensing.md` was not read.

## Sources

- https://github.com/xan1242/NFSMW_XenonEffects (repo page: README excerpt, root listing, stats, licence label)
- https://raw.githubusercontent.com/xan1242/NFSMW_XenonEffects/master/README.md (README)
- https://raw.githubusercontent.com/xan1242/NFSMW_XenonEffects/master/dllmain.cpp (source, first ~100,000 characters)
- https://raw.githubusercontent.com/xan1242/NFSMW_XenonEffects/master/NFSMW_XenonEffects.ini (configuration)
- https://raw.githubusercontent.com/xan1242/NFSMW_XenonEffects/master/LICENSE (MIT License)
- https://github.com/xan1242/NFSMW_XenonEffects/commits/master (commit dates and messages)
