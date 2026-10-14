# NFSMWSlingshotFeature

A C++ ASI plugin for Need for Speed: Most Wanted (2005, PC) by Kevin4e that adds a drafting ("slingshot") system. When the player follows an AI car closely, the plugin builds a boost meter and, once enough is stored, pushes the player's car forward by writing its linear velocity. The mod is configured through an INI file and can optionally show the boost on the speedbreaker bar.

Sources for this page: repo README, LICENSE, the INI file, and the `dllmain.cpp`, `addresses.hpp`, `settings.hpp` and `vehicles.hpp` source files, all read through raw GitHub URLs. Nothing was cloned, downloaded or run. `utils.hpp` and the project files were listed but not read.

## Metadata

| Field | Value |
|---|---|
| Repo | https://github.com/Kevin4e/NFSMWSlingshotFeature |
| Author | Kevin4e (copyright notice in LICENSE: "Copyright (c) 2026 Kevin4e") |
| Type | `.asi` plugin (ASI loader mod) |
| Licence | No open-source licence stated. The LICENSE file is a copyright notice with redistribution limits (see below). Treat as all rights reserved. |
| Language | C++ (Visual Studio solution, `.vcxproj`, `.cpp`/`.hpp` files) [inferred from file list] |
| Target game build | NFS:MW v1.3 [source: `dllmain.cpp` checks `NFSVersionManager::is(MostWanted)` and the comment in the code refers to v1.3; the exact check is in the library, not shown] |
| Latest visible activity | 18 commits in total. Last commit date was not visible on the fetched page. [unverified] |
| Community links | README links a YouTube showcase and an NFSMods page (named in the INI header). Not opened. |

## What it does

Each feature below is based on the README or on code.

1. **Drafting boost meter.** While the player is close behind an active AI car, a boost value grows. Closer and faster draft means faster growth. [source: `dllmain.cpp`, `DraftingSystemHook`, "Boost Accumulation"]
2. **Decay when not drafting.** The meter drops by a fixed step per frame when no qualifying car is in range. [source: `dllmain.cpp`, "Not drafting" branch]
3. **Forward push on release.** When boost is stored, the player's velocity is increased along its own direction axes. [source: `dllmain.cpp`, "Boost Application"]
4. **Two modes.** Mode 0 ("arcade") applies the push only after drafting stops. Mode 1 ("realistic") applies it while drafting too. [source: `dllmain.cpp` and INI header; the names come from the INI file summary]
5. **Speed dependence.** The meter can be scaled by AI and player speed, up to a set speed threshold. It can be turned off. [source: `dllmain.cpp`, `speedDependency`]
6. **Speedbreaker bar display (optional).** The boost value can be written to the speedbreaker bar UI, replacing its normal use. [README; source: `dllmain.cpp`, `ptrToSpeedbreaker`]
7. **Configurable detection box.** Longitudinal distance is limited. Lateral and vertical limits can be fixed values or "AUTO", which uses car dimensions. [source: `dllmain.cpp`, candidate search]
8. **Gameplay guards.** Boost is not applied in menus, the moment camera, or when the player is almost stationary. [source: `dllmain.cpp`, early exits]

## How it does it

### Loading

- The ASI loader calls the exported function `InitializeASI`, which calls `Setup()`. [source: `dllmain.cpp`]
- `DllMain` on process attach checks that the running game is NFS:MW. If not, it shows a `MessageBoxA` and returns FALSE, which unloads the DLL. [source: `dllmain.cpp`]
- The loader is "Ultimate ASI Loader" (`dinput8.dll` in the game root). [README]

### Config reading

- `Setup()` opens the INI with `K4IniReader`. If the file cannot be opened, it shows an error box and returns. [source: `dllmain.cpp`]
- If `Main/Enable` is false, setup stops and nothing is hooked. [source: `dllmain.cpp`]

### Game hook

- `K4MemPatcher::makeCALL(Game::PerFrameUpdate, DraftingSystemHook, false)` writes a CALL instruction at the per-frame update site, so the plugin's function runs every frame. [source: `dllmain.cpp`]
- `Game::PerFrameUpdate` is `0x00663EE3`. [source: `addresses.hpp`]
- `Game::Base` is `0x00400000`. [source: `addresses.hpp`]
- Game state flags read each frame: `Game::Flags::OpenWorld` `0x0092D884`, `Game::Flags::PauseMenu` `0x0091CAE4`, `Game::Flags::MomentCamera` `0x009A3A70`. [source: `addresses.hpp`]
- Speedbreaker bar pointer is resolved from a stable pointer path `{ 0x589228, 0x84 }` through `K4MemPatcher::StablePtr`. [source: `dllmain.cpp`]
- `K4MemPatcher` is built with `K4MP_NO_MUTEX`, `K4MP_ENABLE_CORE` and `K4MP_ENABLE_BASIC_ASM`. [source: `dllmain.cpp`]

### Vehicle access

- `getActiveAIVehicles` walks the game's `VEHICLE_AI` list (`mSize`, `mBegin`) and keeps vehicles where `IsActive()` is true. [source: `vehicles.hpp`]
- `getPlayerVehicle` takes the first entry in `VEHICLE_PLAYERS`. [source: `vehicles.hpp`]
- Position, dimensions, direction vectors and speed come from the vehicle's rigid body via `GetSimable()->GetRigidBody()` and `GetSpeed()`. [source: `vehicles.hpp`]
- `setVelocities` calls `SetLinearVelocity` on the player's rigid body. This is the only write to game state. [source: `vehicles.hpp`]
- The code uses game accessor methods and list lookups, not raw offsets, for vehicle data. The types are defined in `utils.hpp` and `nfsmw.h`, which were not read. [source: `vehicles.hpp`]

### Per-frame algorithm (summary)

1. Exit early if not in open world, in pause menu, or in moment camera. Reset boost if player speed is below 1.0.
2. Get active AI cars. Skip any moving slower than 1.0.
3. A car qualifies if its forward distance is between 0 and `MaxLongitudinalDistance`, and its lateral and vertical offsets fit the limits (fixed or AUTO).
4. Drafting is true when at least one car qualifies. The nearest one sets the distance term.
5. While drafting, gain = distance term (0.003 at distance 0, falling to 0 at the max distance). With speed dependence, AI and player speed terms are added (weights 0.7 and 0.3), and the total is halved. Boost increases by gain times `ProgressiveBoostMultiplier`.
6. Not drafting: boost decreases by 0.005 times `DecayBoostMultiplier`.
7. Clamp boost to `[0, MaxSlingshotBoost]`.
8. If boost is above 0 and (mode is 1 or not drafting): map boost to a push strength in `[0.04, 0.07]`, multiply each normalised player velocity axis by strength times `FinalBoostMultiplier`, and write the result with `setVelocities`.

Constants in steps 5 and 6 (0.003, 0.7, 0.3, 0.005, 0.04, 0.07) are hard-coded in the source. [source: `dllmain.cpp`]

### Graphics

- No Direct3D or DirectX calls. The only Windows API used is `MessageBoxA`. [source: `dllmain.cpp`]

## Configuration

File: `NFSMWSlingshotFeatureConfig.ini`, placed next to the `.asi` in the game's `scripts` folder. [README]

| Section | Key | Type | Default in code | Meaning |
|---|---|---|---|---|
| Main | Enable | bool | false | Turns the mod on. [source: `dllmain.cpp`] |
| SlingshotAttributes | SlingshotMode | int (0 or 1) | not stated in code summary; must be 0 or 1 | 0 = arcade (push after drafting), 1 = realistic (push while drafting). [source: `dllmain.cpp`, INI summary] |
| SlingshotAttributes | MaxSlingshotBoost | float | 1.0 | Peak stored boost. [source: `dllmain.cpp`] |
| SlingshotAttributes | ProgressiveBoostMultiplier | float | 1.0 | Build-up rate while drafting. [source: `dllmain.cpp`] |
| SlingshotAttributes | FinalBoostMultiplier | float | 1.0 | Scales the velocity push. [source: `dllmain.cpp`] |
| SlingshotAttributes | DecayBoostMultiplier | float | 1.0 | Scales the decay rate. [source: `dllmain.cpp`] |
| SlingshotAttributes | MinimumSpeedForMaxDraftGain | string | "300.0" | Speed (km/h per INI summary) for maximum speed-based gain. "OFF" disables speed dependence. [source: `dllmain.cpp`; unit from INI summary] |
| BoostRanges | MaxLongitudinalDistance | float | 45.0 | Maximum forward distance to count as drafting. [source: `dllmain.cpp`] |
| BoostRanges | MaxLateralDistance | string | "AUTO" | Fixed value, or AUTO to use car width. Fixed default 1.5. [source: `dllmain.cpp`] |
| BoostRanges | MaxVerticalDistance | string | "AUTO" | Fixed value, or AUTO to use car depth. Fixed default 0.5. [source: `dllmain.cpp`] |
| Debug | UseSpeedbreakerBarAsSlingshotMeter | bool | false | Shows boost on the speedbreaker bar. Marked experimental in the INI. [source: `dllmain.cpp`, INI summary] |

Note: the INI summary states that `MinimumSpeedForMaxDraftGain` is in km/h and that lateral and vertical defaults are "1.5" and "0.5" as fixed fallbacks. The defaults above in the table are the code's fallbacks; the shipped INI values were not read verbatim, because the fetch tool would not return the full file.

## How we could implement it in nfs-mw

Licence rule: the repo has no open-source licence, and the LICENSE file forbids redistribution and inclusion in modpacks without permission. We must not copy its code or its ini layout verbatim. The plan below is spec-first: describe the behaviour, then write our own implementation from the spec. This page does not give any constants beyond what is needed to state behaviour.

Applicability to a clean-room rewrite:
- The ASI loader, `K4MemPatcher`, `makeCALL` hooks, fixed addresses and the `MessageBoxA` load check do not apply. Our Rust rewrite reads the game's data and has its own update loop, so none of that machinery is needed. Loader-specific hacks do not apply.
- The behaviour (drafting meter, two modes, optional speed scaling, decay, velocity push, optional HUD meter) does apply, and is a reasonable gameplay feature.

### Behaviours to specify

1. Draft detection: which opponent cars count (active, moving, forward distance window, lateral and vertical limits).
2. Boost meter growth, decay, and clamp, including the speed dependence rule.
3. Push application: which frames, which mode, and how the velocity change is capped.
4. Gating: only in open-world driving, not in menus, cutscenes or the moment camera.
5. Config keys and value rules (on/off, mode enum, distance limits, AUTO meaning).
6. Optional HUD output (speedbreaker bar replacement).

### Data and formats needed

- Vehicle list: active AI cars with position, size, direction vectors and speed. Player car with speed and velocity.
- Game state: open-world flag, pause or menu state, moment-camera state.
- A config file format for the settings above. Use a clean key set of our own design (for example `[drafting]` with our own names). Do not reuse the upstream key names or layout.
- Unit definitions and conversion factors, to be confirmed from our own engine data rather than from this repo's constants.

### Natural home

- Engine-generic parts go in `libs/`: the draft-boost algorithm as a pure function over vehicle snapshots, the INI or TOML config parser, and the boost curve maths. These must not depend on NFS:MW names.
- NFS:MW-specific parts go in `crates/`: the vehicle-list reader for NFS:MW's structures, the player velocity write, the open-world and menu flags, and the HUD hook. `crates/` depends on `libs/`, never the reverse.
- Rule: no NFS:MW names, addresses or defaults in `libs/`. Per-version layout tables are passed in by the caller.

### Risks

- Legal: the upstream licence forbids copying. Any reimplementation must come from our own spec and our own observation of the game, not from this source.
- Physics: pushing a car by writing velocity can cause tunnelling, collision spikes and rubber-banding with AI. Needs tests on real game data.
- Version dependence: the game addresses in this repo are for one build. Our rewrite should not depend on them.
- Feel: the constants (for example the 0.003, 0.04 and 0.07 ranges) were tuned by the author. We must tune our own values.
- Gameplay: AI speed and position are read per frame. Ordering with our physics step must be defined.

## Open questions and what could not be verified

- Exact upstream commit history and latest activity date (the page showed 18 commits, no date). [unverified]
- Full shipped INI values. The fetch tool would not return the file verbatim, so the defaults in the Configuration table are the code fallbacks. [unverified]
- Content of `utils.hpp`, `nfsmw.h` and the `K4MemPatcher` / `K4IniReader` libraries. They are not in the repo listing as read, so the exact hook and vehicle struct layouts are not verified.
- Whether the mod is tested against any build other than 1.3. [unverified]
- Units of `MinimumSpeedForMaxDraftGain` (km/h per INI summary; the code compares against a speed value). [unverified]
- Whether any behaviour in the README (YouTube showcase) differs from the code. Video not opened.
- Project file contents (`.vcxproj`, `.sln`, `.filters`, `.user`) were not read. They are build metadata only.
- Whether the release `.asi` matches this source. Downloads were not allowed, so no binary was checked.

## Sources

- https://github.com/Kevin4e/NFSMWSlingshotFeature (repo page, via WebFetch)
- https://github.com/Kevin4e/NFSMWSlingshotFeature/tree/main/NFSMWSlingshotFeature (folder listing)
- https://raw.githubusercontent.com/Kevin4e/NFSMWSlingshotFeature/main/LICENSE
- https://raw.githubusercontent.com/Kevin4e/NFSMWSlingshotFeature/main/NFSMWSlingshotFeatureConfig.ini
- https://raw.githubusercontent.com/Kevin4e/NFSMWSlingshotFeature/main/README.md
- https://raw.githubusercontent.com/Kevin4e/NFSMWSlingshotFeature/main/NFSMWSlingshotFeature/dllmain.cpp
- https://raw.githubusercontent.com/Kevin4e/NFSMWSlingshotFeature/main/NFSMWSlingshotFeature/addresses.hpp
- https://raw.githubusercontent.com/Kevin4e/NFSMWSlingshotFeature/main/NFSMWSlingshotFeature/settings.hpp
- https://raw.githubusercontent.com/Kevin4e/NFSMWSlingshotFeature/main/NFSMWSlingshotFeature/vehicles.hpp
