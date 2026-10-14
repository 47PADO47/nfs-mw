# NFSMW Race Fixes (ExOptsTeam)

NFS:MW Race Fixes is a mod pack by the ExOptsTeam organisation that fixes and extends the race modes of Need for Speed: Most Wanted (2005, PC). It adds Quick Race modes (Tollbooth, Challenge, Token Pickup, Cashgrab), exposes more track options (cop density, starting pursuit condition, lap and opponent counts), fixes LAN race crashes and labels, and restores Cashgrab. It is a native C++ DLL with an INI file and a separate Binary import script for string and texture changes. The repository has no licence file and no stated licence.

Status: the repo's last commit is dated November 2023. Treat it as unmaintained and as a reference only.

## Metadata

| Field | Value |
|---|---|
| Repo | https://github.com/ExOptsTeam/NFSMWRaceFixes |
| Author | ExOptsTeam (GitHub organisation). No individual author named. Icons credited to LunicAura [README] |
| Type | Native .asi-style DLL mod (C++/Visual Studio project) plus Binary import script. Loader not verified (see section 3) |
| Licence | None stated. No LICENSE or LICENSE.md file found (HTTP 404 for both) [source] |
| Language | C++ (.cpp, .h, .rc, .sln, .vcxproj) [source] |
| Target game build | NFS:MW PC. Executable check expects a speed.exe with entry point giving base 0x400000 + AddressOfEntryPoint = 0x7C4040, and the error text names "v1.3 speed.exe (6.029.312 bytes)" [source] |
| Latest visible activity | Commit "Updated README.md" on 2023-11-06. Earlier tagged versions v1.2.2.1337 (2023-11-03), v1.2.1.1337 (2023-02-06), v1.2.0.1337 (2022-02-06) [source: commits page] |
| Stars/releases | Not checked |

## 1. What it does

Each feature is listed with its evidence.

- **Extra Quick Race modes.** Tollbooth, Challenge, Token Pickup and Cashgrab are added to the Quick Race mode menu. Only Tollbooth is shown by default. [README; INI `ShowInMenu` keys]
- **Mode menu visibility.** Sprint, Circuit, Drag, LapKnockout, Tollbooth and Speedtrap can each be shown or hidden in the Quick Race mode menu. [INI]
- **Heat level and pursuit options in track settings.** Heat Level (cop density) appears in the track options for most modes. Starting pursuit condition can be set, within configured Minimum and Maximum limits. [README; INI]
- **Cops toggle.** An option to enable cops in races, moved from Extra Options. [README]
- **Forced rules per mode.** Lap count forced to 1 for point-to-point modes (Sprint, Drag). Heat forced to maximum for Challenge. Opponent count forced to 0 for Token Pickup and Challenge. [README]
- **LAN race fixes.** Fixes a race selection crash and race names/icons for LAN races, moved from Extra Options. Toggled by `LANFixes`. [README; INI]
- **Challenge Series icon fix.** Replaces the hardcoded Tollbooth icon for Challenge Series races and fixes custom HUDs for pursuits. Toggled by `ChallengeSeriesFixes`. [README; INI]
- **Milestone icons.** Icons for unused milestone types, made by LunicAura. [README]
- **Cashgrab restoration.** Cashgrab is back in Quick Race, shows best cash on the track select screen, shows on-screen messages for cash bags (`ShowCashbagMessage`), and has a fixed leaderboard. Post-race stats are "almost fixed" and the Cash Bags/Stages page is still empty. [README; INI]
- **Hook toggles.** Quick Race mode select, track select and track options menus can each be hooked or left alone. [INI]

## 2. How it does it

Claims are tagged: [source] verified in code, [README] stated only in the README, [inferred] my reading.

### Entry point and host check

- `DllMain` acts only on `DLL_PROCESS_ATTACH`. [source: dllmain.cpp]
- It gets the host executable base with `GetModuleHandleA(NULL)`, computes the entry point, and compares it to `0x7C4040`. The check reduces to `0x400000 + AddressOfEntryPoint == 0x7C4040`. [source]
- On mismatch it shows a `MessageBoxA` titled "NFSMW Race Fixes" saying the .exe is not supported, then returns `FALSE`. [source]
- `Init()` is called after the check. Its body is in another file and was not read. [source: dllmain.cpp; body not verified]

### Libraries and structure

- Includes `includes\IniReader.h` for INI parsing and `includes\injector\injector.hpp` for memory patching and hooking. [source]
- `RaceFixesStuff.h` is included from `dllmain.cpp`. Other headers (not read) are named by feature: `CashgrabStuff.h`, `CopDensity.h`, `CustomRaceOptions.h`, `GRaceCustom.h`, `Game.h`, `GlobalVariables.h`, `InGameFunctions.h`, `InRaceStuff.h`, `LANStuff.h`, `PostRaceLapStats.h`, `PostRaceRacerStats.h`, `PostRaceResults.h`, `RaceOverMessage.h`, `SetupStat.h`, `StartingPursuitCondition.h`, `UIQRChallengeSeries.h`, `UIQRModeSelect.h`, `UIQRTrackOptions.h`, `UIQRTrackSelect.h`, `cFrontendDatabase.h`. [source: file listing]

### Hooks and patches

- The hook and patch sites are not in `dllmain.cpp`. They are presumably in the feature headers and `Init()`. The feature headers were not read, so no game function names, addresses or patch bytes are verified. [inferred]
- The UI and HUD feature names above suggest hooks on frontend screens (Quick Race mode select, track select, track options, Challenge Series) and on post-race and in-race code. [inferred]

### Game data changes (non-code)

- The Binary script `_NFSMWRaceFixes-Setup-Binary.end` imports the changes into game files, run in User mode of Binary by MaxHwoy v2.8.3 or later. [README]
- 15 added strings (e.g. `RACETYPES_CASH_GRAB`, `TRACK_SELECT_CHALLENGE`) go in `Languages\<LANGUAGENAME>.bin`. [README]
- 14 added textures (mostly milestone and mode icons) go in `Global\GlobalB.lzc` and `GlobalTextures.tpk`. [README]
- 5 textures are moved from other game files, including COPS_ICON and HELICOPTER_ICON. [README]

### Loader

- The project builds a DLL with a `.cpp` entry point, which suggests an ASI-style plugin loaded by an ASI loader. The loader is not stated in the README and was not verified. [inferred]
- The `.gitignore` and project files were not read. The build output name is not verified.

## 3. Configuration

File: `NFSMWRaceFixesSettings.ini` (in the repo root, next to the DLL when installed). Read by `IniReader.h`. Boolean keys: 0 = false, 1 = true. [source: ini file; reading code not verified]

`[Main]`

| Key | Default | Meaning |
|---|---|---|
| QuickRaceModeSelectHook | 1 | Hook Quick Race mode select menu for the sections below |
| QuickRaceTrackSelectHook | 1 | Hook Quick Race track select menu |
| QuickRaceTrackOptionsHook | 1 | Hook race options menu for the sections below |
| ChallengeSeriesFixes | 1 | Unhardcode Challenge Series Tollbooth icon and fix pursuit HUDs |
| LANFixes | 1 | LAN race fixes |

`[StartingPursuitCondition]`

| Key | Default | Meaning |
|---|---|---|
| Minimum | 1 | Lower limit of the starting pursuit condition option |
| Maximum | 5 | Upper limit |

Per-mode sections: `[Sprint]`, `[Circuit]`, `[Drag]`, `[LapKnockout]`, `[Tollbooth]`, `[Speedtrap]`, `[TokenPickup]`, `[Cashgrab]`, `[Challenge]`.

| Key | Applies to | Default | Meaning |
|---|---|---|---|
| ShowInMenu | all modes | Tollbooth, Sprint, Circuit, Drag, LapKnockout, Speedtrap = 1. TokenPickup, Cashgrab, Challenge = 0 | Show mode in Quick Race mode menu |
| CopDensity | all except Challenge | 1 | Heat Level option in track options |
| StartingPursuitCondition | all modes | 0 | Starting pursuit condition option in track options |
| TrafficLevel | Drag | 0 | Visual only. Traffic level option |
| NumOpponents | Tollbooth | 1 | Opponents option in track options |
| ShowCashbagMessage | Cashgrab | 1 | On-screen message on cash bag pickup |

Notes:
- The INI comments and `README` disagree on some defaults. The INI comments are the stated defaults used above. [source vs README]
- `Minimum` and `Maximum` apply to the starting pursuit condition. Default range 1 to 5. [source]

## 4. How we could implement it in nfs-mw

Rules applied: the repo has no licence, so no code is to be copied or ported. What follows is behaviour and facts only. Specify from observed behaviour and our own spec, then implement clean-room.

### Scope split

- **Clean-room rewrite is not a loader-based mod.** Our Rust rewrite reads the game install directly. It does not patch a running `speed.exe`. Entry-point checks, ASI loading, DllMain, and `injector.hpp`-style memory patching do not apply. Say so in the spec and do not port them.
- **Binary string and texture imports** correspond to our own asset/data pipeline, not to a patch. Any equivalent must come from our own format readers and writers (see docs/formats). Do not use the repo's `.end` script or its bundled textures.

### Behaviours to specify

1. Quick Race mode list: which modes are shown, in which order, with which defaults.
2. Per-mode track option set: which options (heat level, starting pursuit condition, opponent count, traffic) appear for each mode.
3. Mode-specific forced rules: lap count 1 for point-to-point, heat max for Challenge, opponents 0 for Token Pickup and Challenge.
4. Starting pursuit condition: min/max range and default.
5. LAN race selection and labelling rules.
6. Cashgrab: menu entry, best-cash display, cash bag messages, leaderboard. Post-race stats are an open item (see section 6).
7. Challenge Series icon selection (no hardcoded icon).

### Data and formats needed

- Game string table (per language) and texture archives: needed for labels and icons. Our format docs should cover the `.bin` language tables and the `GlobalB.lzc` / `GlobalTextures.tpk` containers before any change is designed. Check docs/formats for current status.
- Mode/rule tables: our own, derived from the game's behaviour, recorded in a spec file.
- Config: a settings file. The INI format and keys can be designed by us. We should not reuse these key names unless the spec says to.

### Natural home

- **Engine-generic code goes in `libs/`**: INI/config parsing, string table reading, texture container reading. These must not contain NFS:MW names, paths or defaults. Per-version tables are passed in by the caller.
- **NFS:MW-specific code goes in `crates/`**: the mode list, the per-mode rule table, the Cashgrab and LAN logic, and the Challenge Series icon mapping. `crates/` depends on `libs/`, never the reverse.
- Format readers take `&[u8]` and do not open files. Only `game-install` touches the install.

### Risks

- Behaviour is tied to specific game code paths we have not read. Any rule we add must be verified against the install data (`cargo xtask` and the reader tests), not against this repo.
- Post-race stats and Cashgrab leaderboard are partly broken in the source mod. Do not treat "almost fixed" as a spec.
- Asset changes (strings, textures) must avoid copying the repo's bundled textures. Icons must be made by us or come from a licensed source.
- The repo's version info is 2023. Game updates or unusual installs may differ.

## 5. Open questions and what could not be verified

- No licence. Both LICENSE and LICENSE.md returned HTTP 404. Licence status: none stated. Do not copy code or assets.
- `Init()` body and all feature headers were not read. Hook targets, game function names, addresses and patch bytes are unverified.
- The INI reading code (`IniReader.h` usage) was not read. Key parsing and default handling are taken from the INI file's own comments.
- The ASI loader used (or whether one is needed) is not stated. Build output name is unverified.
- The README and INI disagree on some details (for example, README says Tollbooth is the only default-enabled mode, and INI `ShowInMenu` defaults agree with that). The INI is used as the defaults list.
- Release binaries were not downloaded or run, per the brief.
- Post-race stats and the Cashgrab Cash Bags/Stages page are incomplete per the README. Not verified in code.
- Whether the repo has open issues or other activity after 2023 was not checked.

## 6. Sources

- https://github.com/ExOptsTeam/NFSMWRaceFixes (repo page; description, file listing, licence not stated)
- https://raw.githubusercontent.com/ExOptsTeam/NFSMWRaceFixes/master/README.md (README)
- https://raw.githubusercontent.com/ExOptsTeam/NFSMWRaceFixes/master/dllmain.cpp (entry point and host check)
- https://raw.githubusercontent.com/ExOptsTeam/NFSMWRaceFixes/master/NFSMWRaceFixesSettings.ini (configuration)
- https://github.com/ExOptsTeam/NFSMWRaceFixes/commits/master (latest activity)
- https://raw.githubusercontent.com/ExOptsTeam/NFSMWRaceFixes/master/LICENSE (HTTP 404, not found)
- https://raw.githubusercontent.com/ExOptsTeam/NFSMWRaceFixes/master/LICENSE.md (HTTP 404, not found)
