# NFSMWHelicopterOptions

Configurable police helicopter behaviour mod for Need for Speed: Most Wanted (2005, PC). It is an ASI plugin that hooks game functions at fixed addresses and overrides helicopter tuning values from INI files, per Heat level (1 to 10) and separately for race events. Source is C++ in a Visual Studio solution. The page describes behaviour and mechanism for our reference only; it is not a source for porting code.

Tag key used below: [source] verified in the repo code as read, [README] stated only in the README, [inferred] deduced by us, not stated.

## 1. Metadata

| Field | Value |
|---|---|
| Repo | https://github.com/eatincrispies/NFSMWHelicopterOptions |
| Author | eatincrispies (copyright holder in LICENSE) [source] |
| Type | .asi plugin (ASI loader mod), with INI configuration [README] |
| Licence | MIT, copyright 2026 eatincrispies [source: LICENSE] |
| Language | C++ (Visual Studio project `HelicopterOptions.vcxproj`, solution `NFSMWHelicopterOptions.slnx`) [source] |
| Target game build | Need for Speed: Most Wanted PC v1.3 [README]. Source version string is `V3.3.0` [source: dllmain.cpp] |
| Latest visible activity | Not visible. The repo page shows 98 commits but no last-commit date on the captured page |
| Credits | Eatincrispies (mod creator), Fierelier (MinGW support), Roskler (mod tester) [README] |

## 2. Summary

The mod gives the player control over the police helicopter that Heat levels spawn. It can change how far away the helicopter spawns and despawns, its top speed and fly speed, its sight range and sight cone, how long it stays in attack mode, how long it keeps fuel, its respawn delay, and several flight and pursuit tuning values such as leading, closing speed, turning, acceleration and braking. Every value can differ per Heat level, and race events have their own table.

## 3. What it does

Each feature below is a user-visible setting. The INI key names are the section names in `Configuration/*.ini`.

- **Spawn distance** (`Helicopter:SpawnDistance`, default 250.0). How far from the player the helicopter appears. [source: Configuration/General.ini]
- **Despawn distance** (`Helicopter:DespawnDistance`, default 600.0). Distance at which it leaves. [source: Configuration/General.ini]
- **Respawn delay** (`Helicopter:Respawning`, default 0.0 for all levels). When above 0, replaces the game's respawn timer for the helicopter. [source: AIVehicleHelicopter.cpp, RespawnTimerEntry; INI]
- **Top speed / speed cap** (`Helicopter:SpeedCap`, default 100.0). Overwrites the chopper's maximum speed field. [source: SimpleChopper.cpp ApplySpeedCap; General.ini]
- **Line of sight** (`Helicopter:LineOfSight`, 251.0 for Heat 1 to 7 and race 1 to 7; 301.0 for Heat 8 to 10 and race 8 to 10). Overrides the sight distance passed to the target check; 0 or less keeps the game value. [source: AIVehicleHelicopter.cpp CanSeeTargetEntry; General.ini]
- **All-around sight** (`Helicopter:AllAroundSight`, default 75.0) and **front-sight-only** (`Helicopter:FrontSightOnly`, default true). [README: "sight" in General.ini description; key names from General.ini]
- **Hiding spots** (`Helicopter:HidingSpots`, default true). Whether the helicopter may use hiding spots. [inferred from key name; behaviour not traced in code read]
- **Fuel time** (`Helicopter:FuelTime`, 10 s at Heat 1 to 3, 60 s at Heat 4, 90 s at Heat 5, 180 s at Heat 6 to 10). Sets the helicopter's fuel timer when a new helicopter begins. [source: AIVehicleHelicopter.cpp BeginHelicopter]
- **Attack duration** (`Helicopter:AttackDuration`, default 8.0). Length of the attack phase. [README: "attack length"; key from General.ini; not traced in code read]
- **Search speed** (`Helicopter:SearchSpeed`, default 70.0, 70.0). Two-part value. [README: "search speed"; key from General.ini]
- **Fly speed** (`Helicopter:FlySpeed`, default 100.0). [README: "top speed" group; key from General.ini]
- **Leading** (`Helicopter:Leading`, default 30.0, 45.0). Target-leading aim values. [README: "leading"; Advanced.ini]
- **Closing speed** (`Helicopter:ClosingSpeed`, default 20.0, 30.0, 50.0). [README; Advanced.ini]
- **Search radius** (`Helicopter:SearchRadius`, default 80.0, 250.0). [README; Advanced.ini]
- **Turning** (`Helicopter:Turning`, default 1.3, -8.0). Sets torque scale and yaw clamps. [source: SimpleChopper.cpp Refresh; Advanced.ini]
- **Acceleration** (`Helicopter:Acceleration`, default 80.0, 30.0). Sets the game's max and min chopper acceleration globals. [source: SimpleChopper.cpp Refresh, per the fetched summary; Advanced.ini]
- **Braking** (`Helicopter:Brake`, default 1.55, 0.4). [README; Advanced.ini; code path not confirmed]
- **Heli sheet** (`Helicopter:HeliSheet`, default true) and **ignore heli sheet distance** (`Helicopter:IgnoreHeliSheetDistance`, default 0.0). [README; Advanced.ini; code in HeliSheet.cpp not read]
- **Helicopter sees and hides from cars and helicopters** (AIPerpVehicle fields `mHiddenFromCars`, `mHiddenFromHelicopters`). [source: dllmain.hpp data layout; behaviour not traced]

## 4. How it does it

### Loading and lifecycle [source]

- `DllMain` on process attach starts a thread that runs `Initialize`. `Initialize` sleeps 1000 ms, opens the log, loads INI settings, runs each helicopter module's `InstallPatches`, installs hooks and logs readiness.
- On detach with a null `lpReserved`, it removes all detours, restores the speed cap, and restores all patched bytes.
- Debug builds write a log to `HelicopterOptions\Logs\NFSMWHelicopterOptions.log`, rotated at 4 MB to `.old`. Release builds stub out logging.

### Memory access [source]

- `Memory::Read`, `CheckBytes`, `WriteData`, `WriteCode` operate on absolute virtual addresses. Reads and copies are SEH-guarded. `WriteCode` uses `VirtualProtect` to `PAGE_EXECUTE_READWRITE` and then `FlushInstructionCache`.
- `Game::ThisCall`, `Game::Call` call game functions by address. `Game::VirtualFunction` reads a vtable entry by slot.

### Inline detours [source]

- `Detour::Install` writes a relative jump over a prologue of at most 8 stolen bytes, after matching the expected prologue bytes. A 64-byte trampoline is built to run the original code. Maximum 8 detours.
- The saved register frame is `Detour::Registers` (edi, esi, ebp, esp, ebx, edx, ecx, eax, eflags). The handler reads and writes registers directly, so it overrides values the game passes in registers.
- Hooks listed in code:
  - `AIPerpVehicle::SetHeat` at `0x00409060`, 7-byte prologue `6A FF 68 BC 79 86 00`. Handler `SetHeatEntry`. Reads the argument at `esp+0x08`. Heat is valid from 1 to 10.
  - `AIActionHeliPursuit` constructor.
  - `AIVehicleHelicopter::OnDriving`, `CanSeeTarget`, `RespawnTimer`.
  - `AICopManager::ApplyBreakerZones`.
  - `EAXAirSupport::Update`. Skipped when `NFSMWRevisedAirSupport.asi` is loaded.

### Patch sites [source]

- A `Patch` layer rewrites fixed instruction sites: float operands, float pushes (`0x68`), call targets (`0xE8`), data floats, float and byte stores, and vtable slots. `Commit` checks guard bytes first, journals originals and rolls back on failure. `RestoreAll` reverts on unload. Float comparison tolerance is 0.001.

### Game data layout [source]

- `kIPlayerLists = 0x0092D848`; `IPlayerList` size `0x30`.
- `AIPerpVehicle` (per the fetched header summary): `kSetHeat = 0x00409060`; `mIOwner` at `0x34`; `mIPerpetrator` at `0x758`; `mHeat` at `0x774`; `mWasInRaceEventLastHeatUpdate` at `0x786`.
- `AIVehicleHelicopter`: `kgHeliVehicle` global at `0x0090D61C`; `mHeliFuelTimeRemaining` at `+0x7D8`; `mISimpleChopper` at `+0x8B0`; `mSpawnHeliTimer` at `+0xE0`.
- `SimpleChopper` speed cap: `MAX_SPEED_MPS` at offset `0x44` in the chopper specs layout. Applied only when the value is finite and between 1.0 and 10000.0. Original values are kept and restored on unload, for up to four layouts.

### Per-frame behaviour [source]

- `OnDriving` runs each driving update. It smooths a frame-time delta from `QueryPerformanceCounter`, clamped between 1/8 and 1 of a 1/60 s reference step. It skips frames when fuel is non-finite, detects a new helicopter (owner change, or fuel jump above 1.0), calls `BeginHelicopter` on a new one, and otherwise applies motion filters.
- `BeginHelicopter` sets fuel to `FuelTime` when that value is above 0.

### Configuration loading [source]

- `Ini::Load` reads `General` first, then `Advanced`. Primary paths are `HelicopterOptions\Configuration\General\General.ini` and `...\Advanced\Advanced.ini`. Fallback paths are `HelicopterOptions\Configuration\General.ini` and `...\Advanced.ini`.
- `Ini::ApplyHeat(level, racing)` picks the value for the current Heat level or race flag and writes it into the game-side variable through a `Setting` record (section, name, float pointer, bool pointer, min, max, has-default).

Note: the README says the files live in `scripts\HelicopterOptions\Configuration`. The code paths above are relative to the module directory. [source vs README; consistent once the ASI sits in `scripts`]

## 5. Configuration

Keys are `[Helicopter:<Name>]` sections, with `default`, `heat01` to `heat10` and `race01` to `race10` keys. Values are comma-separated, up to four parts. Booleans accept `true` or `false`. Numbers are clamped to each setting's min and max. Heat level is clamped to 1 to 10.

Full default values are in `Configuration/General.ini` and `Configuration/Advanced.ini`. Summary:

| Section | Default or pattern |
|---|---|
| SpawnDistance | 250.0 all levels |
| Respawning | 0.0 all levels |
| DespawnDistance | 600.0 all levels |
| SpeedCap | 100.0 all levels |
| LineOfSight | 251.0 (Heat 1 to 7), 301.0 (Heat 8 to 10) |
| AllAroundSight | 75.0 |
| FrontSightOnly | true |
| HidingSpots | true |
| FuelTime | 10, 10, 10, 60, 90, 180, 180, 180, 180, 180 seconds |
| AttackDuration | 8.0 |
| SearchSpeed | 70.0, 70.0 |
| FlySpeed | 100.0 |
| Leading | 30.0, 45.0 |
| ClosingSpeed | 20.0, 30.0, 50.0 |
| SearchRadius | 80.0, 250.0 |
| Turning | 1.3, -8.0 |
| Acceleration | 80.0, 30.0 |
| Brake | 1.55, 0.4 |
| HeliSheet | true |
| IgnoreHeliSheetDistance | 0.0 |

Notes:
- The fetched `General.ini` lists `SearchSpeed` and `FlySpeed` in the same file as `SpeedCap`. Their meaning is from key names and the README only. [README; inferred]
- The game's own vanilla values are read from memory at load time, and used when the INI has no entry. [source, per the fetched summary of dllmain.cpp]
- A key name mismatch to check: the INI section is `Respawning`, while the code reads `sSettings.RespawnDelay`. Whether the two are linked through the `kIniSettings` table could not be verified, since the header tables were not in the read set. [inferred]

## 6. How we could implement it in nfs-mw

### What applies to a clean-room rewrite

- The mechanism (ASI loader, inline detours, fixed-address patches, vtable edits, reading `dllmain.cpp` addresses) does not apply. Our rewrite does not run the original binary. Loader-specific hacks, fixed addresses for v1.3, and the `NFSMWRevisedAirSupport.asi` interaction are out of scope.
- The licence is MIT, so copying is allowed in principle. Project rule still applies: spec first, do not copy code, document behaviour and data only. This page is a behaviour reference.

### Status in the rewrite

The single-number options and the multi-number ones (`Leading`, `ClosingSpeed`, `Acceleration`, `Brake`, `Turning`) are read from `NFSMWHelicopterSettings.ini` (`crates/nfsmw/src/settings/helicopter*.rs`); `SearchRadius` is read and held. See `docs/specs/ai-helicopter.md`, "Parity pass".

### Behaviours to specify

1. Helicopter pursuit spawn and despawn distances, per Heat level and per race.
2. Helicopter top speed, fly speed, search speed and speed cap.
3. Sight model: sight distance by Heat level, all-around sight radius, front-only sight, line-of-sight override.
4. Fuel model: fuel timer by Heat level, reset when a new helicopter begins, expiry behaviour.
5. Attack duration and respawn delay.
6. Pursuit tuning: leading, closing speed, search radius.
7. Flight tuning: turning, acceleration, braking.
8. Heli sheet and heli sheet distance.
9. Clamping and fallback rules: values clamped to min and max, race table fallback, default when the INI has no entry.

### Data and formats needed

- Vanilla helicopter tuning values from the game data (to set the defaults). Must come from the install via format docs, not from this repo.
- A per-Heat and per-race table schema: 10 Heat levels plus race, each with up to four comma-separated parts.
- An INI reader that handles section keys, comma lists, booleans and clamping. Should live in `libs/` as an engine-generic config reader (no NFS:MW names).

### Natural home

- `libs/`: generic INI or config reader with per-level keys and clamping; generic timing helpers.
- `crates/`: the NFS:MW helicopter tuning schema, helicopter AI and pursuit logic, and the default values. Crates depend on libs, never the reverse.
- Keep NFS:MW names and defaults out of `libs/` (AGENTS.md rule).

### Risks

- Defaults: the INI defaults are the mod author's values, not verified vanilla values. Do not take them as game truth without checking the install data.
- Behaviour model: the mod overrides game values, so our rewrite must know the vanilla model (what each field means) before it can change it. The repo does not document the game's AI in detail.
- Heat and race interaction: the mod keys off Heat level and a race flag. We need the same signals in our rewrite.
- Fuel and despawn are coupled to spawn logic; changing one without the other can leave helicopters stuck or spawning in loops. [inferred]
- Version lock: the mod is pinned to PC v1.3. Our rewrite reads the installed data and must not hard-code addresses.

## 7. Open questions and what could not be verified

- Latest activity date is not visible on the captured repo page. Whether the repo is maintained is unknown.
- The `Helicopter` source directory was listed (10 files), but only `SimpleChopper.cpp` and `AIVehicleHelicopter.cpp` were read, partially. `AIActionHeliPursuit`, `AIActionHeliExit`, `HeliSheet` and the header files with the `kIniSettings` tables and the full address table were not read in full. Claims about them are not verified.
- Hook addresses for `CanSeeTarget`, `RespawnTimer`, `OnDriving` and the others were not given in the read set.
- Spawn, despawn, leading, closing speed, search radius, braking and attack duration code paths were not traced. Their behaviour is from key names and README only.
- The fetched summaries are model paraphrases of the source. Exact code and constants should be re-checked before any use.
- Release binaries and the built `.asi` were not examined (per instructions).
- `Thumbnail_v2.webp` and `NFSMWHelicopterOptions.slnx` contents beyond the project path were not read.
- The README was not reproduced verbatim by the fetch; the description above is from a summary of the README page.

## 8. Sources

- https://github.com/eatincrispies/NFSMWHelicopterOptions (repo page, file tree, description, licence, commit count)
- https://raw.githubusercontent.com/eatincrispies/NFSMWHelicopterOptions/HEAD/README.md (README, summarised)
- https://raw.githubusercontent.com/eatincrispies/NFSMWHelicopterOptions/HEAD/LICENSE (MIT licence)
- https://github.com/eatincrispies/NFSMWHelicopterOptions/tree/main/HelicopterOptions (directory listing)
- https://github.com/eatincrispies/NFSMWHelicopterOptions/tree/main/Configuration (directory listing)
- https://github.com/eatincrispies/NFSMWHelicopterOptions/blob/main/NFSMWHelicopterOptions.slnx (solution project list)
- https://raw.githubusercontent.com/eatincrispies/NFSMWHelicopterOptions/main/HelicopterOptions/dllmain.cpp (summarised)
- https://raw.githubusercontent.com/eatincrispies/NFSMWHelicopterOptions/main/HelicopterOptions/dllmain.hpp (summarised)
- https://raw.githubusercontent.com/eatincrispies/NFSMWHelicopterOptions/main/Configuration/General.ini
- https://raw.githubusercontent.com/eatincrispies/NFSMWHelicopterOptions/main/Configuration/Advanced.ini
- https://github.com/eatincrispies/NFSMWHelicopterOptions/tree/main/HelicopterOptions/Helicopter (directory listing)
- https://raw.githubusercontent.com/eatincrispies/NFSMWHelicopterOptions/main/HelicopterOptions/Helicopter/SimpleChopper.cpp (partly read, summarised)
- https://raw.githubusercontent.com/eatincrispies/NFSMWHelicopterOptions/main/HelicopterOptions/Helicopter/AIVehicleHelicopter.cpp (summarised)
