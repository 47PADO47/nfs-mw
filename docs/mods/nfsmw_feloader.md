# NFSMW_FELOADER (NFSMW FrontEnd Platform Loader)

## Summary

NFSMW_FELOADER is a Windows DLL mod for Need for Speed: Most Wanted (2005, PC) that replaces the front-end garage platform with a custom one. It is a port of ArchieGoldmill's NFSC_FELoader (Carbon) to Most Wanted. The README marks it as a work in progress and tells users not to use it yet. Mechanism: the DLL checks the game executable's entry point, then patches game code at fixed addresses to redirect garage and platform rendering to custom code and a custom platform resource.

## Metadata

| Field | Value |
|---|---|
| Repo | https://github.com/bluesky-dev12/NFSMW_FELOADER |
| Repo description | "GarageLoader for MostWanted(HIGHWIP)" [README/GitHub page] |
| Author | bluesky-dev12 (GitHub account). Real name not stated. |
| Type | .dll mod (native C++ code injected into the game, no ASI loader stated) |
| Licence | None stated. No LICENSE file was seen in the root listing. Treat as all rights reserved. |
| Language | C++ (`.cpp`, `.vcxproj` files). GitHub language breakdown not captured. |
| Target game build | Need for Speed: Most Wanted v1.3 English, per the `dllmain.cpp` check (see section 3) |
| Latest visible activity | Not captured. The page showed "10 Commits" on main. No commit date was visible. |
| Status | Work in progress, README says not to use yet |

## 1. What it does

Each feature below lists its evidence.

- **Custom garage platform in the front end.** The game's garage platform (the rotating stage the car sits on) can be replaced by a platform model loaded from a file named in `NFSMWFELoader.ini`. [source: `NFSMW_FELoader/dllmain.cpp`, `CUSTOM_PLATFORM` section of `NFSMWFELoader.ini`]
- **Per-screen camera and car placement presets.** Each front-end screen (main menu, platform crib, career safehouse, customization shop, shop backroom, car lot) maps to a position preset that sets car and camera offsets. [source: `NFSMWFELoader.ini` `POS_*` and `PRESET_*` sections]
- **Scrolling platform ring.** Parts of the custom platform whose names start with `SCROLL_` form a ring that rotates. Scroll item count and speed range are configurable. [source: `dllmain.cpp` runtime behavior notes; `CUSTOM_PLATFORM` keys]
- **Front wheels follow the scroll angle.** While the custom platform is active, the front wheels rotate with the platform scroll. [source: `dllmain.cpp`]
- **Scroll speed changes on customize-parts screens.** Speed changes when the customize-parts screen is constructed and destroyed. [source: `dllmain.cpp` hooks on `FeCustomizeParts_Ctor` / `Dtor`]
- **Optional real front-end reflections.** When enabled, the car is drawn with a reflection pass on the garage floor. [source: `dllmain.cpp` `DrawGarageReflection` hook, gated by `RealFrontEndReflections`; key read from `NFSMWHDReflections.ini`]
- **Error message for a missing platform file.** Shows "Custom platform geometry not found!" if the custom platform file is missing. [source: `dllmain.cpp`]
- **Front-end map loading switch.** Config key `LoadMapInFE` exists, but its effect is commented out in code, so it does nothing. [source: `dllmain.cpp` and `NFSMWFELoader.ini`]
- **Unsupported executable rejection.** If the game executable is not the expected build, the DLL shows "This .exe is not supported." and does not load. [source: `dllmain.cpp`]

Not implemented or disabled:
- `ToggleReflections` is a no-op; its patch is commented out. [source: `dllmain.cpp`]
- Position helper hooks at `0x7A226B` and `0x7A2289` are commented out. [source: `dllmain.cpp`]
- Track loading (`TRACKS\STREAML2RA.BUN`, `TRACKS\L2RA.BUN`) is commented out. [source: `dllmain.cpp`]
- Only one custom platform per garage type is supported (README to-do). [README]

## 2. How it does it

Entry point and gating:
- `DllMain` runs `Init()` on `DLL_PROCESS_ATTACH` only when the host executable's entry point equals `0x7C4040`. Otherwise it shows a message box and returns `FALSE`. [source: `dllmain.cpp`]
- Hooks are installed with helper functions `injector::MakeCALL`, `MakeJMP`, `MakeRangedNOP` and a `hook_back` pattern (files under `NFSMW_FELoader/injector/`: `assembly.hpp`, `calling.hpp`, `hooking.hpp`, `injector.hpp`, `utility.hpp`, and subfolder `gvm`). These are patch helpers that rewrite instructions in the game's memory. [source: directory listing of `injector/`, usage in `dllmain.cpp`]

Patches (addresses are game addresses in the v1.3 English executable):
- Garage file selection: `MakeCALL` at `0x662B8E` (GameFlowLoadGarageScreen) and `0x7B4104` (GarageMainScreen::RefreshBackground) redirects to `FEManager_GetGarageNameFromType`. [source]
- Car rotation: `MakeRangedNOP` over `0x7A99B5`–`0x7A99DC`, then `MakeCALL` to `GarageMainScreen_GetCarRotationZ`. [source]
- Only when the custom platform file is present and `LoadCustomPlatform` is 1:
  - `0x6DED32`: `MakeCALL` to `DrawGarageMain` (sub_6DE300). [source]
  - `0x6DE7C5`: `MakeCALL` to `DrawGarageReflection`, only if `RealFrontEndReflections` is 1. [source]
  - `0x7A9BEB`: `MakeJMP` to `UpdateRenderingCarParametersCave`, which exits to `0x7A9C0F`. [source]
  - `0x571839`: `MakeCALL` to `FeCustomizeParts_Ctor` (hook_back). [source]
  - `0x7C01D3`: `MakeCALL` to `FeCustomizeParts_Dtor` (hook_back). [source]

Game data and functions referenced (addresses from `dllmain.cpp`):
- Data: `ShadowZ` `0x744232`, `GameState` `0x925E90`, `CarZ` `0x8916CC`, `DeltaTime` `0x9259BC`, `FrontSteerAngle` `0x905E20`, `nullString` `0x890978`. [source]
- Functions: `GetTrackPositionMarker` `0x727340`, `StringHash` `0x460BF0`, `eViewPlatInterface_Render` `0x6DA9B0`, `eModel_Init` `0x506AF0`, `eModel_GetBoundingBox` `0x501570`, `LoadResourceFile` `0x661760`, `FindResourceFile` `0x659020`, `RenderWorld` `0x723FA0`, `FeCustomizeParts_Ctor` `0x7C00C0`, `FeCustomizeParts_Dtor` `0x7BCFF0`, `FeManager_Get` `0x516E10`, `FeManager_GetGarageType` `0x516E20`. [source]

Runtime:
- The custom platform is loaded through the game's own resource system (`LoadResourceFile` / `FindResourceFile`). The existence check uses the `Path` value resolved relative to the DLL's directory. [source]
- It parses chunk `0x134003` of the platform resource and creates a model per part. Parts named `SCROLL_*` form the rotating ring. [source]
- It draws the platform each frame when `GameState` is 3 and a custom platform is configured. [source]

Built-in platform file names the game uses (listed in the ini, not loaded by this code path): `FRONTEND\PLATFORMS\PLATFORMCRIB.BIN`, `CAREER_SAFEHOUSE.BIN`, `CUSTOMIZATION_SHOP.BIN`, `CUSTOMIZATION_SHOP_BACKROOM.BIN`, `CAR_LOT.BIN`. [source: `NFSMWFELoader.ini`]

Not verified: the contents and meaning of `IniReader/` (folder present, not read), the `gvm` subfolder, and the exact bytes of each hooked instruction.

## 3. Configuration

File: `NFSMW_FELoader/NFSMWFELoader.ini` (read at runtime). Values below are from the summary of the file and `dllmain.cpp`; the file was not reproduced verbatim.

- `[GENERAL]`
  - `LoadMapInFE`: no effect (commented out in code).
  - `LoadCustomPlatform`: 1 to enable the custom platform. Current file: enabled.
- `[CUSTOM_PLATFORM]`
  - `Path`: platform resource file. Current file points to `UG1.BIN`.
  - `X`, `Y`, `Z`: platform offset. Current file: Z = -0.25.
  - `ScrollItems`: default 25.
  - `ScrollSpeedMin`: default 1.0. Current file: 1.
  - `ScrollSpeedMax`: default 10.0. Current file: 10.
- `[POS_MAIN]`, `[POS_PLATFORMCRIB]`, `[POS_CAREER_SAFEHOUSE]`, `[POS_CUSTOMIZATION_SHOP]`, `[POS_CUSTOMIZATION_SHOP_BACKROOM]`, `[POS_CAR_LOT]`
  - `Preset`: default -1. Current file maps main menu and platform crib to preset 0, safehouse to 2, shop to 3, backroom to 4, car lot to 5.
- `[PRESET_n]`
  - `X`, `Y`, `Z`: offsets.
  - `CarZ`: car height value.
  - `CustomPlatform`: whether to render the custom platform.
  - `CarReflection`: default 1.
  - `CarRotation`: default -1.
  - Current file: preset 0 uses Z -0.3, car reflection on, custom platform on. Presets 1 to 4 have zero offsets and CarZ 304.96979. Preset 5 has Y 0.075 and CarZ 340.

File: `NFSMWHDReflections.ini`
- `[GENERAL]` `RealFrontEndReflections`: enables the reflection pass.

Note: the README says the to-do list includes switching to mINI and cleaning up unused code, so keys may change.

## 4. Relevance to nfs-mw (clean-room Rust rewrite)

Licence: the repo has no stated licence. Under the project rules (`docs/licensing.md`, AGENTS.md "spec-first"), no code, constants copied from code, or structure may be taken from this repo. The points below describe behaviour and facts only and are the basis for any spec work. They are not a porting guide.

Does the feature apply? Mostly no. This mod works by injecting into the original executable and patching instructions at fixed addresses, which is a loader-specific hack. nfs-mw's approach is to read the install directly and build its own front end, so the address-patching mechanism does not apply and should not be reproduced. The user-visible goal (a configurable garage platform with per-screen camera presets and a scrolling ring) can be a native feature.

Behaviours to specify (as our own spec, from user-visible behaviour and game data formats):
- A per-front-end-screen placement table: screen to platform, car offset, camera offset, car rotation, reflection flag.
- Optional custom platform: a platform model file with named parts, where parts with a prefix form a rotating ring.
- Ring scroll: item count, min and max speed, speed change on entering and leaving customize-parts screens.
- Front wheels follow ring scroll angle while the custom platform is active.
- Optional front-end reflection pass on the car.
- Missing-file behaviour: clear error, fall back to the built-in platform.

Data and formats needed:
- The game's platform resource format (chunk-based, chunk `0x134003` per this repo's reading). Our data reader must get this from our own spec, verified against the install, not from this repo. See `docs/formats/` for existing gameplay and resource reading notes.
- A config format for placement and presets. Our rewrite should choose its own (the to-do mentions mINI, but that is this repo's choice).

Natural home:
- Generic resource and chunk parsing: `libs/` (no NFS:MW names in `libs/`), with a format crate that takes `&[u8]`.
- Platform placement tables, screen names and preset defaults: `crates/` (NFS:MW-specific). `crates/` depends on `libs/`, never the reverse.
- Config loading: the install and data reading belong to `game-install` (per AGENTS.md, only that crate touches the install).

Risks:
- Legal: no licence means copying is not allowed. Any similarity must come from our own spec and observation of the game.
- Format accuracy: the chunk id and file names come from this repo, not from a verified spec. Verify each against the install with `python tools/chunkdump.py <file>` before relying on it.
- Renderer: the front-end draw path differs across renderer backends (ADR 0004). Placement must not depend on the original draw hooks.
- Game build: this repo targets v1.3 English only. Our target must be stated and checked.

## 5. Open questions and what could not be verified

- No licence file was seen in the root listing. Not confirmed by viewing the repo root listing in full (the WebFetch summary showed only `.gitattributes`, `.gitignore`, `NFSMW_FELoader.sln`, `README.md`, and the `NFSMW_FELoader/` folder).
- Language breakdown, latest commit date and commit history were not captured.
- Whether the `.ini` summary is complete. The file was summarised, not read verbatim.
- Whether `IniReader/`, `injector/gvm/`, and `NFSMW_FELoader.vcxproj*` contain more than the directory listing shows. Not read.
- The bytes at each hooked address and the function signatures were taken from the code summary, not checked by reading the disassembly. Not verified in a running game.
- Whether any released platform mods work with this loader (the to-do list says compatibility is untested).
- The `UG1.BIN` platform file named in the ini is not in the repo and was not examined.
- No build or run was attempted, per the method in the task.

## 6. Sources

- https://github.com/bluesky-dev12/NFSMW_FELOADER (repo page: README summary, root listing, description, commit count)
- https://raw.githubusercontent.com/bluesky-dev12/NFSMW_FELOADER/main/README.md (README)
- https://github.com/bluesky-dev12/NFSMW_FELOADER/tree/main/NFSMW_FELoader (folder listing)
- https://raw.githubusercontent.com/bluesky-dev12/NFSMW_FELOADER/main/NFSMW_FELoader/dllmain.cpp (code summary)
- https://raw.githubusercontent.com/bluesky-dev12/NFSMW_FELOADER/main/NFSMW_FELoader/NFSMWFELoader.ini (config summary)
- https://github.com/bluesky-dev12/NFSMW_FELOADER/tree/main/NFSMW_FELoader/injector (folder listing)
