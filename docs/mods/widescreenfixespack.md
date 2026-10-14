# ThirteenAG / WidescreenFixesPack

A collection of native plugin fixes for PC games. Its entry for Need for Speed: Most Wanted (2005) adds widescreen HUD and FOV correction, a windowed-mode option, shadow fixes, gamepad icon swaps, camera and NOS trail options, and optional SMAA and gamma post-processing. The fix is a C++ ASI plugin that patches the running game by byte-pattern scanning, plus an INI file of user settings. This page records behaviour and mechanism only, so it can inform a clean-room spec for nfs-mw.

## Metadata

| Field | Value |
|---|---|
| Repository | https://github.com/ThirteenAG/WidescreenFixesPack |
| Author / maintainer | ThirteenAG (GitHub user; the README links to the maintainer's pages, no separate author line was seen) |
| Type | Native ASI plugin collection (C++), one folder per game. NFS:MW entry: `data/NFSMostWanted.WidescreenFix/` and `source/NFSMostWanted.WidescreenFix/` |
| Licence | MIT License, per the repo's `license` file and sidebar [README/licence file]. The full licence text was not read in full, only its opening grant sentence |
| Language | C++ (`dllmain.cpp`, `.ixx` module files, `.fx` shader). GitHub's language bar was not visible in the fetch |
| Target game build | Need for Speed: Most Wanted (2005, PC). The exact executable version was not stated in the fetched pages (open question) |
| Stars | About 3.2k (GitHub page, as fetched) |
| Latest visible activity | Commit 708f752, 8 Oct 2026, "gta: remove lod dist and script slot fix" (commit list as fetched; the list is not per-game) |

## What it does

Each feature below is a user-visible option. Evidence is the INI key and its comment (`data/NFSMostWanted.WidescreenFix/scripts/NFSMostWanted.WidescreenFix.ini`) unless noted.

1. **Widescreen HUD correction.** `FixHUD` (default 1) corrects the HUD aspect ratio. `HudAspectRatioConstraint` (default `Auto`) can constrain the HUD to 16:9, 21:9 or a custom ratio. [INI]
2. **Widescreen FOV correction.** `FixFOV` (default 1) with `Scaling` (default 1; 0 original, 1 Xbox 360, 2 mathematically correct). [INI]
3. **Front-end scaling.** `FEScale` (1.00, Xbox 360 equivalent 0.92), `AutoFitFE` (1) scales the UI to fit the screen width, for ratios such as 16:10. [INI]
4. **FMV handling.** `FMVWidescreenMode` (1 cropped, 2 stretched), `FMVScale`, `AutoFitFMV` (adds black bars on ratios narrower than 16:9), `SkipIntro` (default 1) skips launch FMVs. [INI]
5. **Windowed modes.** `WindowedMode` (default 0) offers borderless, bordered, resizable, and two borderless-fullscreen variants (values 1 to 5). Source: `Resolution.ixx` (window sized and centred with `AdjustWindowRectEx`/`SetWindowPos`; Alt is blocked in windowed mode). [INI, source]
6. **Resolution list.** The game's supported resolution and refresh-rate arrays are relocated to larger buffers so custom resolutions can be chosen in the options menu. Source: `Resolution.ixx`. [source]
7. **Dynamic shadow fixes.** `ShadowsFix` (1) keeps shadows from vanishing in tunnels and under bridges. `ShadowsRes` (default 2048, max 16384), `AutoScaleShadowsRes` (1), `ShadowMapTextureFormat` (0), `ImproveShadowLOD` (1), `ForceSharpShadows` (0). Source: `Shadows.ixx`. [source]
8. **Gamepad icons and input.** `ImproveGamepadSupport` (1 Xbox, 2 PlayStation, 3 none) swaps keyboard icons for gamepad icons and assigns front-end actions. `LeftStickDeadzone` and `RightStickDeadzone` (both 10.0). [INI]
9. **Camera options.** `CameraReturnTimeout` (3.0 s), `CameraReturnSpeed` (2.0), `StickLookSensitivity`, `MouseLookSensitivity`, `InvertLook`, `OnlyAllowHorizontalStickMovement`. Source module `Camera.ixx` exists but was not read. [INI; code not verified]
10. **NOS trail fixes.** `FixNOSTrailLength` (1), `CustomNOSTrailLength` (1.0), `FixNOSTrailPosition` (0), `NOSTrailPositionScalar` (0.3). Source module `NOSTrailFix.ixx` exists but was not read. [INI; code not verified]
11. **Post-processing.** `SMAA` (default 0) and `ConsoleGamma` (default 1; 1 higher-contrast curve, 2 the Xbox 360 preset). Source: `PostFX.ixx`. [INI, source]
12. **Other graphics tweaks.** `DisableMotionBlur` (0), `RainDropletsScale` (0.5), `LightStreaksEnable` (0), `BleachByPassEnable` (0, experimental), `ForcedGPUVendor` (0x10DE), `HideAABug` (2). [INI]
13. **Audio and sim rate.** `ForceHighSpecAudio` (1) forces 44100 Hz. `SimRate` (-1) sets the gameplay refresh rate (0 disabled, -1 monitor rate, -2 double monitor rate). [INI]
14. **Settings and paths.** `Language`, `CustomUserFilesDirectoryInGameDir` (0), `WriteSettingsToFile` (0). [INI]

## How it does it

Source reviewed: `source/NFSMostWanted.WidescreenFix/` (`dllmain.cpp`, `Shadows.ixx`, `Resolution.ixx`, `PostFX.ixx`). Other modules in the folder were listed but not read: `Camera.ixx`, `ComVars.ixx`, `Compat.ixx`, `Controller.ixx`, `Frontend.ixx`, `MSAA.ixx`, `Misc.ixx`, `NOSTrailFix.ixx`, `PostFX.fx`, `Registry.ixx`, `Rendering.ixx`, `Windowed.ixx`.

- **Loading.** The plugin is a DLL with an ASI extension, built from `dllmain.cpp`. On `DLL_PROCESS_ATTACH` it checks for an ASI loader (`IsUALPresent()`, which the name suggests detects Ultimate ASI Loader; the check itself was not read) and otherwise calls `InitializeASI()`. [source]
- **Deferred init.** `InitializeASI()` registers `Init` on a callback reached through `GetSystemTimeAsFileTime`, located by byte pattern `C7 00 80 02 00 00 C7 01 E0 01 00 00 C2 08 00`. So `Init` runs from a game code path, not during DLL load. [source]
- **No hard-coded addresses in the main path.** Globals and functions are found by byte patterns (for example `hWnd`, `WindowRect`, `Direct3DDevice`, `GameFlowManager::IsPaused`, `cFEng::pInstance`, `gActionBindings`). Patterns are per game build, so the plugin depends on the exact executable matching them. [source]
- **Call-site hooks.** `injector::MakeCALL` redirects the main loop to `MainLoop` and five mini-loop call sites to `MiniMainLoop`. The mini-loop hook stores one "original" pointer for all five sites, which the code assumes are the same target without checking. [source]
- **Mid-function hooks.** `safetyhook::create_mid` hooks the device-reset path (`BeforeResetHook` calls `WFP::onBeforeReset`) and the end-of-scene path (`BeforeEndSceneHook` calls `WFP::onEndScene`). [source]
- **Event system.** A `WFP` event bus (`onInitEvent`, `onInitEventAsync`, `onGameInitEvent`, `onGameProcessEvent`, `onMenuEnterEvent`, `onMenuExitEvent`, `onMenuDrawingEvent`, `onResChange`, `onBeforeReset`, `onEndScene`, `onShutdownEvent`) lets each module register handlers. [source]
- **Shadow patches.** Byte patterns locate shadow-related code. Patches NOP two bytes and write 0 to a variable (tunnel fix); write `60.0f` to a car shadow opacity variable; use `injector::MakeInline` hooks to load depth-bias values; redirect the X and Y shadow-resolution pointers to a computed value; and write a texture-format selector to a global at `0x00982C08` (comment in source). Literal addresses in the source are `0x901AC0` and `0x901ABC` (defaults). [source]
- **Resolution patches.** Inline hooks on `DALOptions::IncScreenRes` (action IDs `0x9120409E` and `0xB5971BF1`), on `GetRacingResolution`, and on `GetLocalizedWideString` (fallback to the module's own list). A packed resolution value `(w << 16) | h | 0x80000000` is used in place of the game's index. [source]
- **Rendering.** `PostFX.ixx` hooks a Direct3D 9 path through a mid-function hook that calls `CPostFX::RenderSMAA(Direct3DDevice)`. A gamma pass runs on `onEndScene`. Device-reset handling releases and recreates effect resources. The file has no direct `Present` call and no direct D3D9 calls beyond the device pointer. [source]
- **Configuration.** Read from `NFSMostWanted.WidescreenFix.ini` next to the plugin, using `CIniReader` (section and key). [source for `PostFX.ixx`; file location from the `scripts/` folder layout, [inferred]]

## Configuration

All keys and defaults are listed in the INI file (`data/NFSMostWanted.WidescreenFix/scripts/NFSMostWanted.WidescreenFix.ini`). The table below gives the value-level summary; the INI file is the authoritative list.

| Section | Key | Default | Meaning |
|---|---|---|---|
| LANGUAGE | Language | blank | Language choice |
| MAIN | FixHUD | 1 | HUD aspect fix |
| MAIN | FixFOV | 1 | FOV aspect fix |
| MAIN | Scaling | 1 | FOV scaling mode (0, 1, 2) |
| MAIN | HudAspectRatioConstraint | Auto | Auto, 16:9, 21:9 or custom |
| MAIN | FMVWidescreenMode | 1 | 1 cropped, 2 stretched |
| MAIN | FEScale | 1.00 | Front-end size |
| MAIN | FMVScale | 1.00 | FMV size |
| MAIN | AutoFitFE | 1 | Fit UI to width |
| MAIN | AutoFitFMV | 1 | Fit FMV to width |
| MISC | SkipIntro | 1 | Skip launch FMVs |
| MISC | WindowedMode | 0 | Window style (0 to 5) |
| MISC | CustomUserFilesDirectoryInGameDir | 0 | Named user-data folder |
| MISC | WriteSettingsToFile | 0 | Write registry settings to settings.ini |
| MISC | ImproveGamepadSupport | 1 | 1 Xbox, 2 PlayStation, 3 none |
| MISC | LeftStickDeadzone | 10.0 | Left stick deadzone |
| MISC | RightStickDeadzone | 10.0 | Right stick deadzone |
| MISC | ForceHighSpecAudio | 1 | Force 44100 Hz |
| MISC | SimRate | -1 | Gameplay refresh (0, -1, -2) |
| GRAPHICS | ShadowsRes | 2048 | Shadow resolution (1024 to 16384) |
| GRAPHICS | ShadowsFix | 1 | Tunnel and bridge shadow fix |
| GRAPHICS | ImproveShadowLOD | 1 | More shadow detail |
| GRAPHICS | AutoScaleShadowsRes | 1 | Scale shadow res by aspect |
| GRAPHICS | ShadowMapTextureFormat | 0 | -1 disables patch, 0 to 3 formats |
| GRAPHICS | ForceSharpShadows | 0 | Sharp shadows |
| GRAPHICS | RainDropletsScale | 0.5 | Droplet size |
| GRAPHICS | DisableMotionBlur | 0 | Disable motion blur |
| GRAPHICS | LightStreaksEnable | 0 | Light trail effect |
| GRAPHICS | BleachByPassEnable | 0 | Experimental contrast effect |
| GRAPHICS | ForcedGPUVendor | 0x10DE | Vendor ID override (0 disables) |
| GRAPHICS | ConsoleGamma | 1 | Gamma curve (1 or 2) |
| GRAPHICS | SMAA | 0 | SMAA anti-aliasing (not with MSAA) |
| GRAPHICS | HideAABug | 2 | MSAA edge line workaround (0, 1, 2) |
| CAMERA | Enable | 1 | Camera options on or off |
| CAMERA | CameraReturnTimeout | 3.0 | Seconds before camera recentres |
| CAMERA | CameraReturnSpeed | 2.0 | Recentre speed |
| CAMERA | StickLookSensitivity | 1.0 | Stick look multiplier |
| CAMERA | MouseLookSensitivity | 1.0 | Mouse look multiplier |
| CAMERA | InvertLook | 0 | Invert vertical look |
| CAMERA | OnlyAllowHorizontalStickMovement | 0 | Horizontal-only stick camera |
| NOSTrail | FixNOSTrailLength | 1 | Fix trail length at high frame rates |
| NOSTrail | CustomNOSTrailLength | 1.0 | Trail distance multiplier |
| NOSTrail | FixNOSTrailPosition | 0 | Move trail away from car |
| NOSTrail | NOSTrailPositionScalar | 0.3 | Trail position relative to tail lights |

Descriptions are from the INI comments [INI]. Key behaviour was not tested in a running game.

## How we could implement it in nfs-mw

nfs-mw is a clean-room Rust rewrite. This repo's code is MIT licensed, but the spec-first rule in this project still applies: we describe behaviour and facts, and do not copy or port the C++, the byte patterns, or the shader code. Loader-specific mechanics do not carry over.

**Does not apply to a clean-room rewrite:**
- ASI loader detection, `GetSystemTimeAsFileTime` callback injection, `safetyhook` and `injector` calls, byte-pattern scanning of a retail executable, and literal memory writes. The rewrite owns its own runtime and does not patch the original binary to achieve these effects.
- The MiniMainLoop hook and the D3D9 end-of-scene hooks are specific to the original executable's control flow.

**Behaviours to specify (from the feature list above):**
- Widescreen aspect handling: HUD and front-end scaling to target aspect, FOV scaling modes (original, Xbox 360 style, mathematically correct), and FMV fit modes (cropped, stretched, letterboxed).
- Window modes and resolution list: borderless, bordered, resizable, and fullscreen variants, and a custom resolution list.
- Shadow resolution and shadow quality options, with aspect-based scaling.
- Input: gamepad icon set selection, deadzones, and camera look sensitivity, inversion, and recentring timing.
- Post-processing: SMAA and a gamma curve option, applied after the scene is rendered.
- Simulation rate: 0 disabled, monitor rate, double monitor rate.
- Audio output rate.

**Data and formats needed:**
- A settings file with sections and typed keys, with the defaults above as the spec. A TOML or INI file in `libs/` is an engine-generic choice; the NFS:MW key names belong in `crates/`.
- The game's own resolution and aspect data, read from the install by `game-install` (per AGENTS.md, only `game-install` touches the install).
- The FMV, HUD and front-end layout data needed to scale UI, from the NFS:MW format docs in `docs/`.

**Natural home:**
- `libs/`: generic aspect-ratio and FOV maths, window-mode state, settings loading, shadow-resolution scaling, the post-processing pass interface, and the simulation-rate clock. None of this may name NFS:MW.
- `crates/`: NFS:MW-specific keys, the NFS:MW HUD and front-end layout, the camera tuning defaults, the NOS trail behaviour, and gamepad icon mapping.
- Dependency rule: `crates/` depends on `libs/`, never the reverse.

**Risks:**
- Byte-pattern patching is fragile across game builds; the rewrite avoids it by not patching at all.
- Several source modules were not read (Camera, NOS trail, Controller, Frontend, Registry). Their behaviour is taken from INI comments only, so a spec for those must be checked against the real game before implementation.
- Some options are marked experimental or as leftovers from other games (for example `LightStreaksEnable`, `BleachByPassEnable`). These are candidates to drop rather than port.
- Reproducing the original FOV and HUD behaviour needs reference captures from the retail game, which this page does not provide.

## Open questions and what could not be verified

- The full MIT licence text was not read; only its opening grant sentence was seen. The repository's `license` file was fetched at its lowercase path. The copyright holder line was not confirmed.
- Whether the NFS:MW plugin's `.asi` is built from `source/NFSMostWanted.WidescreenFix/` in this repo was not confirmed. The `scripts/` folder holds only the INI and a `.tpk` file. The `.tpk` file was not read.
- The exact game executable version targeted by the byte patterns was not verified.
- `Camera.ixx`, `NOSTrailFix.ixx`, `Controller.ixx`, `Frontend.ixx`, `Registry.ixx`, `Rendering.ixx`, `Windowed.ixx`, `MSAA.ixx`, `Misc.ixx`, `ComVars.ixx`, `Compat.ixx`, and `PostFX.fx` were listed but not read.
- The shadow texture-format and depth-bias code was reported from a summary of the source. Exact instruction-level behaviour was not checked.
- The `IsUALPresent()` check and `CIniReader` location logic were not read.
- The GitHub language bar and the latest commit for this game folder were not visible; the commit shown is repo-wide.
- Behaviour in a running game was not tested. Nothing was downloaded or executed.
- The repository's README was fetched as `readme.md` (lowercase). Its NFS:MW section did not list entries; the INI and source were used instead.

## Sources

- https://github.com/ThirteenAG/WidescreenFixesPack (repo page: file listing, sidebar licence and star count, commit count)
- https://raw.githubusercontent.com/ThirteenAG/WidescreenFixesPack/master/readme.md (README)
- https://raw.githubusercontent.com/ThirteenAG/WidescreenFixesPack/master/license (MIT licence, opening sentence only)
- https://github.com/ThirteenAG/WidescreenFixesPack/tree/master/source (folder listing; NFS:MW folder name `NFSMostWanted.WidescreenFix`)
- https://github.com/ThirteenAG/WidescreenFixesPack/tree/master/data (folder listing)
- https://github.com/ThirteenAG/WidescreenFixesPack/tree/master/data/NFSMostWanted.WidescreenFix/scripts (file listing)
- https://raw.githubusercontent.com/ThirteenAG/WidescreenFixesPack/master/data/NFSMostWanted.WidescreenFix/scripts/NFSMostWanted.WidescreenFix.ini (config keys and defaults)
- https://github.com/ThirteenAG/WidescreenFixesPack/tree/master/source/NFSMostWanted.WidescreenFix (module listing)
- https://raw.githubusercontent.com/ThirteenAG/WidescreenFixesPack/master/source/NFSMostWanted.WidescreenFix/dllmain.cpp (entry point and hook flow)
- https://raw.githubusercontent.com/ThirteenAG/WidescreenFixesPack/master/source/NFSMostWanted.WidescreenFix/Shadows.ixx (shadow patches)
- https://raw.githubusercontent.com/ThirteenAG/WidescreenFixesPack/master/source/NFSMostWanted.WidescreenFix/Resolution.ixx (resolution and window patches)
- https://raw.githubusercontent.com/ThirteenAG/WidescreenFixesPack/master/source/NFSMostWanted.WidescreenFix/PostFX.ixx (SMAA and gamma post-processing)
- https://github.com/ThirteenAG/WidescreenFixesPack/commits/master (latest commit visible)

Not read, though listed (not fetched as file content): https://github.com/ThirteenAG/WidescreenFixesPack/tree/master/data (other game folders), and the `source/` modules listed above. The attempted URLs `https://github.com/ThirteenAG/WidescreenFixesPack/tree/master/NFS%20Most%20Wanted%20(2005)` and `https://raw.githubusercontent.com/ThirteenAG/WidescreenFixesPack/master/README.md` returned HTTP 404 and were replaced by the URLs above.
