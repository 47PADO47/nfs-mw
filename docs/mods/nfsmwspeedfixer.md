# NFSMWSpeedFixer

An `.asi` mod for Need for Speed: Most Wanted (2005, PC) that makes the Speedbreaker mechanic configurable. It exposes activation, recharge, time-dilation, physics and grip values in an INI file and writes them into the running game's memory at startup.

| Field | Value |
|---|---|
| Repo | https://github.com/rng-guy/NFSMWSpeedFixer |
| Author | rng-guy (GitHub account) |
| Type | `.asi` plugin (32-bit Windows DLL, memory patching) |
| Licence | CC0-1.0 (stated in `LICENSE.txt`; the repo's own LICENSE file, read via raw URL, is the CC0 1.0 Universal text) |
| Language | C++20, MSVC (Visual Studio 2022, `_MSC_VER` >= 1930) [README] [source] |
| Target game build | Need for Speed: Most Wanted (2005). Entry-point check expects `speed.exe` of 6,029,312 bytes (5.75 MB) and code at entry point `0x3C4040` [README] [source] |
| Latest visible activity | Commit dated October 13, 2026 ("Improved conversion accuracy.", "Improved configuration structure and logic.") [source: commit list] |
| Stars, releases | Not checked |

## Summary

SpeedFixer is a small, single-purpose mod. It does not add new game content. It changes numeric parameters of the Speedbreaker (the game's timed slow-motion and charge mechanic) and a few related physics values. A user edits `NFSMWSpeedFixerSettings.ini`, and the DLL reads it at game initialisation, clamps the values, and patches the game's memory so the game reads the configured numbers. The README lists 16 adjustable areas. Authors state that VltEd and Binary mods and most other `.asi` mods are compatible.

## 1. What it does

Each item lists the feature and its evidence.

- **Activation speed.** Sets the minimum car speed (kph) needed to activate Speedbreaker. Evidence: README "Customisable settings"; INI `[Activation] minCarSpeed`; source writes `0x8B01C0`.
- **Maximum charged duration.** Sets how long Speedbreaker lasts when fully charged. An omitted value means unlimited. Evidence: INI `[Activation] maxDuration` comment "(max. duration unlimited if parameter omitted)"; source patches short-jump bytes when no finite duration is set.
- **Passive recharge.** Sets the speed and time needed for passive recharge to complete. Setting both values to omitted disables it. Evidence: INI `[Recharging]` `minCarSpeed`, `passiveTime`; README.
- **Active recharge.** Sets the drift speed (kph), slip angle (degrees) and effectiveness scale that trigger and scale active recharging. Evidence: INI `[Recharging]` `minDriftSpeed`, `minSlipAngle`, `activeScale`.
- **Time dilation.** Sets the in-game simulation time multiplier and the delays into and out of full dilation. Evidence: INI `[Physics] timeScale`, `toDilationScale`, `fromDilationScale`.
- **Collision mass.** Multiplies car mass used in collisions while Speedbreaker runs. Evidence: INI `[Physics] carMassScale`.
- **Downforce gravity.** Multiplies the gravity used for downforce. Evidence: INI `[Physics] gravityScale`.
- **Front-wheel steering.** Boosts steering friction (percent) and raises the maximum steering angle (degrees, 0 to 90). Evidence: INI `[Physics] frontFrictionBoost`, `maxSteeringAngle`.
- **Aerodynamic drag.** Reduces aero drag by a percentage. Evidence: INI `[Physics] aeroDragReduction`.
- **Forward grip while slipping.** Reduces forward grip while the car is slipping, by a percentage between 15 and 100. Evidence: INI `[Physics] slipGripReduction`.
- **Vanilla-safe defaults and clamping.** Values outside their allowed range are corrected, and omitted values fall back to vanilla. Evidence: INI header comments; source `Bounds` struct clamping [source, described in the file comments].
- **Disable by removing the file.** Deleting the INI is documented in the file as disabling SpeedFixer. Evidence: INI header comment. Note: the source returns silently when the file is missing (see section 3 and section 6).
- **Bundling.** Authors allow bundling with other mods without credit. Evidence: README "Sharing" [README].

## 2. Installation (user side, for context)

Evidence: README [README].

1. Use a clean, unmodified copy of the game (not a repack). Confirm `speed.exe` is 6,029,312 bytes.
2. Install an `.asi` loader (the README names ThirteenAG's WideScreenFix as one option, which includes one).
3. Extract the release archive (`NfSMW_SpeedFixer_v1.05.0.7z` as named in the README) and copy its `scripts` folder into the game folder, replacing existing files.
4. Edit `scripts/NFSMWSpeedFixerSettings.ini` if needed.

Update and uninstall are described as removing and reinstalling files in the `scripts` folder.

## 3. How it does it

Tags: [source] = verified in the code read; [README] = stated only in the README; [inferred] = my reading, not verified.

### Loading

- The DLL entry point `DllMain` does nothing unless the reason is `DLL_PROCESS_ATTACH`. [source: `Source/dllmain.cpp`]
- It compares the game's entry point (`MemoryTools::GetEntryPoint()`) with `0x3C4040`. On mismatch it shows a `MessageBoxA` error and returns `FALSE`. This is a version guard. [source]
- It installs one hook: `PATCH_HOOK_FUNCTION(Initialise, 0x6665B4)`, which hooks the game's initialisation. A source comment names the original as `InitializeEverything (0x665FC0)`. The hook is `__cdecl`, calls the original first through `CALL_HOOK_ORIGINAL`, and then applies the patches. [source]
- In `_DEBUG` builds the hook spins on `IsDebuggerPresent()` until a debugger attaches. [source]
- The macros `PATCH_HOOK_FUNCTION`, `HOOK_ORIGINAL`, `CALL_HOOK_ORIGINAL` and the `MemoryTools::Write<T>` helper come from `Source/Headers/MemoryTools.hpp`. That header was not read, so the exact hook mechanism (trampoline, inline patch, or other) is [inferred] only. [source for the macro names; inferred for the mechanism]
- The `.asi` loading itself is provided by an external `.asi` loader, per the README. [README]

### Config file

- The file is opened as `scripts/NFSMWSpeedFixerSettings.ini` with `std::ifstream`. If it cannot be opened, the function returns silently. [source]
- It is parsed with `StreamParser::Parser<>` (section capacity 3, pair capacity 9, from `Source/Headers/StreamParser.hpp`, not read). [source]
- Three sections are read: `Activation`, `Recharging`, `Physics`. [source]
- Values are clamped through a `Bounds` struct. [source]

### Memory writes

The source writes values to fixed addresses in `speed.exe` using `MemoryTools::Write<T>(value, {addresses})`. Addresses are listed as found in the source; the meaning of each is partly [inferred] from the setting it is paired with.

- Activation: float at `0x8B01C0` (min car speed). Pointer at `0x6EDDC3` (charge drain rate). Short-jump bytes `0xEB` at `0x6EDDB9` and `0x6F8F76` when duration is unlimited. [source]
- Recharging: floats at `0x901AE8` and `0x901AE4` (speed and passive time). `0xEB` at `0x6EDDE3` when passive recharge is disabled. Pointers at `0x6A99B8`, `0x6A99CB`, `0x6A99F8` for drift speed, slip angle and scale. [source; meanings inferred from the INI keys]
- Physics: pointers at `0x472C53`, `0x6F4DC8`, `0x6F4DF4`, `0x6A9E37`, `0x69E990`, `0x6B201E`, `0x6B1FA3`, `0x6B1F17`, plus float writes at `0x6F4DD4` and `0x901AEC`. [source; per-address meanings inferred]
- Pointer writes (`Write<float*>`) make the game read the mod's own global variables, so later changes to those variables take effect without re-patching. [source: type of write; the runtime effect is inferred]

### Summary of mechanism

The DLL reads a text config, clamps values, then, once the game has initialised, overwrites a set of float constants and redirects a set of pointers and short jumps in the game code. Nothing in the read source opens or modifies game files on disk. [source for the write sites; disk behaviour inferred from the absence of file writes in the code read]

## 4. Configuration

File: `scripts/NFSMWSpeedFixerSettings.ini`. All values are decimal numbers. Defaults below are the values in the file as read. The file header says the unedited values match vanilla. [source]

| Section | Key | Default | Allowed range (from file comment) | Meaning (from file comment) |
|---|---|---|---|---|
| Activation | minCarSpeed | 48.3 | >= 0 | speed (kph) to activate |
| Activation | maxDuration | 10.0 | >= 0 | max duration (s) when fully charged; omit for unlimited |
| Recharging | minCarSpeed | 160.9 | >= 0 | speed (kph) to trigger passive recharge |
| Recharging | passiveTime | 25.0 | >= 0 | time (s) to complete passive recharge; omit both passive keys to disable |
| Recharging | minDriftSpeed | 56.3 | >= 0 | speed (kph) to trigger active recharge |
| Recharging | minSlipAngle | 30.0 | 0 to 90 | slip angle (degrees) to trigger active recharge |
| Recharging | activeScale | 1.0 | any | effectiveness of active recharge, relative to vanilla |
| Physics | timeScale | 4.0 | >= 1 | simulation time dilation |
| Physics | carMassScale | 2.0 | >= 0 | car mass for collisions |
| Physics | gravityScale | 3.0 | any | gravity for downforce |
| Physics | toDilationScale | 1.0 | >= 0 | relative delay to full dilation |
| Physics | fromDilationScale | 1.0 | >= 0 | relative delay from full dilation |
| Physics | frontFrictionBoost | 75.0 | >= 0 | percent boost to steering friction |
| Physics | maxSteeringAngle | 60.0 | 0 to 90 | max steering angle (degrees) |
| Physics | aeroDragReduction | 75.0 | 0 to 100 | percent reduction of aero drag |
| Physics | slipGripReduction | 100.0 | 15 to 100 | percent reduction of forward grip when slipping |

Note: the file says the defaults shown are the values written in the file. Whether these equal the vanilla game values is stated by the file header but not verified here. [README] [source]

## 5. How we could implement it in nfs-mw

Licence status: the repo states CC0-1.0, a permissive public-domain dedication. The licence therefore does not block use. The project's own process (`docs/licensing.md`, and the spec-first rule in `CONTRIBUTING.md` and `AGENTS.md`) still applies. I did not read `docs/licensing.md`, so I cannot confirm its exact rules. The spec below describes behaviour and facts only, and avoids porting the C++ code.

### Applicability

This feature does not carry over as-is. It depends on:

- A `.asi` loader and in-process memory patching of a 2005 retail binary. A clean-room rewrite does not need this. It would instead read the game's data and run its own simulation (the rewrite reads the install at `D:\`, per project memory).
- Hard-coded addresses and entry-point checks. These are loader-specific hacks and do not apply.
- Restating the game's Speedbreaker behaviour through patches. In the rewrite the same knobs would be parameters of our own simulation.

The useful part for us is the set of tunable parameters and their meanings, which can be specified as behaviour.

### Behaviours to specify (spec-first)

1. Speedbreaker activation: a minimum speed, and a maximum duration at full charge, with an unlimited option.
2. Recharge: passive recharge (speed threshold, completion time, optional disable) and active recharge (drift-speed threshold, slip-angle threshold, effectiveness scale).
3. Time dilation: simulation time multiplier, and entry and exit transition scale factors relative to the vanilla delays.
4. Physics modifiers while Speedbreaker is active: collision mass scale, downforce gravity scale, front-wheel friction boost, max steering angle, aero drag reduction, slip grip reduction.
5. Validation: each parameter has a range; out-of-range values are clamped; omitted values fall back to vanilla.
6. Disable-on-missing-config behaviour, decided by us. The source and the file header disagree in wording; we specify our own behaviour explicitly.

### Data and formats needed

- A settings file format for these parameters. Section-and-key INI is one option. The rewrite can use any format that the spec defines (TOML or a typed Rust struct with serde would also work; the choice is ours).
- Vanilla values for each parameter, taken from the game data or from our own measurements, not from this repo's file header. We must verify them against the install.
- Units: kph, seconds, degrees, percent. Decide conversion once, at the boundary.
- Speedbreaker timing and dilation data from the game files, as already read by the existing format crates.

### Natural home

- Parameter types, clamping and range rules, and the settings parser: `libs/` if written generically (no NFS:MW names), otherwise `crates/`. A settings-parsing helper is engine-generic, so it belongs in `libs/`.
- Speedbreaker rules, recharge, dilation and the physics modifiers: `crates/` (NFS:MW-specific), depending on `libs/`. `libs/` must not depend on `crates/`.
- Config file location and default values: `crates/` (game-specific defaults), with the vanilla values passed in per game version, per the project's layout-table rule.

### Risks

- Changing these values changes game feel and can break balance. Defaults must match vanilla unless a setting is explicitly chosen.
- Our simulation will not match the original's internal behaviour exactly. The parameters need calibration against the real game, which may need a reference capture.
- Range rules (for example, slip-angle limits up to 90 degrees) need a reason in the spec, not just this repo's comments.
- Licence: CC0 is permissive, but we still specify behaviour ourselves, which keeps the rewrite independent of this code.

## 6. Open questions and what could not be verified

- The `MemoryTools.hpp` and `StreamParser.hpp` headers were not read. The exact hook technique and parser behaviour are unverified.
- The meaning of most individual write addresses is inferred from the matching INI key. Not verified against game disassembly.
- The file header says that deleting the INI disables SpeedFixer. The source returns silently when the file cannot be opened. Whether the hook still applies vanilla patches in that case is not verified.
- The README did not give per-setting defaults; the defaults above come from the INI file.
- Whether the INI defaults equal the vanilla game values is claimed by the file header, not verified.
- Mod loader details: the README names an `.asi` loader; which one the author tested with is not stated.
- Only the top-level `Source/` folder listing and `Source/dllmain.cpp` were fully read. The contents of `Source/Headers/` were not listed.
- Latest activity is the October 13, 2026 commit. Release-tag history and the number of commits (81 on `master`, as shown on the repo page) were not checked in detail.
- No test or build was run. Per the instructions, nothing was cloned or executed.
- `LICENSE` (no extension) returned 404. The licence file is `LICENSE.txt`.

## 7. Sources

- https://github.com/rng-guy/NFSMWSpeedFixer (repo page: description, file listing, language area, commit count)
- https://raw.githubusercontent.com/rng-guy/NFSMWSpeedFixer/main/LICENSE (returned 404)
- https://raw.githubusercontent.com/rng-guy/NFSMWSpeedFixer/master/LICENSE.txt (CC0 1.0 Universal)
- https://raw.githubusercontent.com/rng-guy/NFSMWSpeedFixer/master/README.md (README summary)
- https://github.com/rng-guy/NFSMWSpeedFixer/tree/master/Source (folder listing)
- https://github.com/rng-guy/NFSMWSpeedFixer/tree/master/Config (folder listing)
- https://raw.githubusercontent.com/rng-guy/NFSMWSpeedFixer/master/Config/NFSMWSpeedFixerSettings.ini (settings and defaults)
- https://raw.githubusercontent.com/rng-guy/NFSMWSpeedFixer/master/Source/dllmain.cpp (entry point, hook, config loading, memory writes)
- https://github.com/rng-guy/NFSMWSpeedFixer/commits/master (latest commit date)
