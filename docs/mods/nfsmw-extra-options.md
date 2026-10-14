# NFSMW Extra Options (ExOpts)

Reference notes on [ExOptsTeam/NFSMWExOpts](https://github.com/ExOptsTeam/NFSMWExOpts), a script mod for the
PC game (2005) that adds hidden and extra options: hotkey cheats, quick race limits, traffic density, pursuit
tuning, weather, gameplay tweaks and a few engine fixes. This page covers what it does, what its settings file
controls, and how this port relates to it. The port's own reading of the file is in
[specs/extra-options.md](../specs/extra-options.md). The copy bundled in the REDUX pack is in
[redux-v3-plugins.md](redux-v3-plugins.md#nfsmwextraoptionsasi).

**Licence: GPL-3.0.** Unlike the other mods in this folder, the repository has a LICENSE file (GPL version 3).
Under [licensing.md](../licensing.md) a GPL project is "yes, for understanding", and code cannot be ported.
This page therefore records **facts**: option names, defaults and behaviour as described by the authors. The
settings comments are paraphrased, not quoted. No source, header or binary from the mod is copied, and the
C++ source was not read.

## Snapshot inspected

| Item | Value |
|---|---|
| Repository | `ExOptsTeam/NFSMWExOpts`, default branch `master` |
| Description | "Need for Speed Most Wanted Extra Options" |
| Last push | 2023-02-19 (the last release, v10.0.1.1337, is from the same day) |
| Stars / forks / open issues | 265 / 37 / 67 at inspection |
| Language | C++ (Visual Studio solution `NFSMWExtraOptions.sln`); includes an INI parser (`ini_parser.hpp`) and an `injector` folder |
| Licence | GPL-3.0 |
| Files | `NFSMWExtraOptions/` (source), `NFSMWExtraOptionsSettings.ini`, `README.md` |
| Releases | v10.0.1.1337 (2023-02-19), v10.0.0.1338 (2022-08-07), v10.0.0.1337 (2022-08-05) |

The README is two lines and points to the Releases page. The settings file was read from the repository. The
source file names were listed, not read. Nothing was run against the game.

## What it does

The mod is a set of options and fixes, all controlled by one INI file. The groups are:

- **Hotkeys.** Keyboard shortcuts for cheat-style actions: a heat-level override, cops on or off, cop lights,
  headlights, a freeze-car and freeze-camera pair, unlock-everything, an auto-drive mode, and optional
  save and load of the world position.
- **Quick race and random race limits.** The minimum, maximum and random-race lap counts and opponent counts.
  It also allows lap changes for knockout races, limits how many car models AI opponents use, and can let
  opponents use bonus or police cars.
- **Traffic density.** The three levels shown in the options menu (Minimum, Moderate, Maximum), each a
  percentage from 0 to 100.
- **Menus.** Shows hidden menu entries (online, debug and preset car categories, hidden tracks, subtitles,
  language select), and changes some limits such as the random-race count and profile name length.
- **Gameplay.** Hidden camera modes, a debug world camera, a faster game speed or car scale (for fun), removal
  of some barriers, a choice of handling model for flat tyres, starting and awarded cash, a skip-track hotkey,
  an unlocked Burger King challenge, a higher part purchase limit in the performance shop, and removal of the
  rev limiter.
- **Pursuit.** Forces the heat-level range, makes in-race pursuits harder, shows cops on the minimap, changes
  the busted screen, and sets the speed limits for the three speeding infractions.
- **Weather.** Forces rain, and sets the rain amount, size, intensity, speed and gravity.
- **Fixes.** Disappearing wheels, a helicopter bounty fix, a zero-bounty fix at high heat, a skinless-car fix,
  an immobile-object collision fix, a fix for the prologue (DDay) events repeating, and an unused busted scene.
- **Misc.** Skips movies and some scenes, toggles sound and music, allows multiple game instances, expands
  memory pools, prints debug text on screen, and forces anti-aliasing, car LOD and tyre LOD.

The mod has an explicit **experimental** tag on several options, and a **for experts** tag on others. The
tables below repeat those tags.

## Settings file: `NFSMWExtraOptionsSettings.ini`

The table is the upstream file on `master`. Defaults are the values in the file. "Expert" and "experimental"
are the authors' tags. Descriptions are paraphrased.

### `[Hotkeys]`

Keys are Windows virtual-key codes in decimal. The file points to a key table on the web for the values.

| Key | Default | Meaning |
|---|---|---|
| `HeatLevelHack` | `118` (F7) | Enables the heat-level hack; forces heat-level override and lets the hotkey below pick a level. |
| `ChangeHeatLevel` | `33` (PageUp) | Changes the heat level while the hack is on. |
| `ToggleCops` | `145` (Scroll Lock) | Toggles cops on and off. Works with the heat-level hack. |
| `CopLights` | `79` (O) | Toggles cop lights. Best used in a cop car. |
| `Headlights` | `72` (H) | Toggles headlights. |
| `FreezeCar` | `115` (F4) | Enables the freeze-car hack. |
| `FreezeCamera` | `19` (Pause) | Freezes or unfreezes the camera. |
| `UnlockAllThings` | `116` (F5) | Unlocks everything and allows the hidden tracks. |
| `AutoDrive` | `117` (F6) | Enables the auto-drive (drunk driver) hack. |
| `EnableSaveLoadHotPos` | `0` | Save (Shift + 1 or 5) and load (Ctrl + 1 or 5) a world position. |

### `[LapControllers]`

| Key | Default | Meaning |
|---|---|---|
| `Minimum` | `0` | Fewest laps selectable on the Quick Race options screen (0 to 127). |
| `Maximum` | `127` | Most laps selectable on the Quick Race options screen (0 to 127). |
| `RandomRace` | `127` | Most laps on the Random Race screen (0 to 127). |
| `UnfreezeKO` | `1` | Allows changing the lap count for knockout races. |

### `[OpponentControllers]`

| Key | Default | Meaning |
|---|---|---|
| `Minimum` | `0` | Fewest opponents on the Quick Race options screen (0 to 15). |
| `Maximum` | `6` | Most opponents on the Quick Race options screen (0 to 15). |
| `RandomRace` | `6` | Most opponents on the Random Race screen (0 to 15). |
| `MaxUniqueOpponentCars` | `3` | Most different car models AI opponents use in one race. |
| `MoreCarsForOpponents` | `0` | Lets opponents use bonus and police cars. |
| `CarbonStyleRandomCars` | `0` | Removes random vinyls from AI cars. Does not work with Extra Customization in Unlimiter. |

### `[TrafficControllers]`

| Key | Default | Meaning |
|---|---|---|
| `Low` | `10` | Density for the Minimum option (0 to 100). |
| `Medium` | `30` | Density for the Moderate option (0 to 100). |
| `High` | `50` | Density for the Maximum option (0 to 100). |

### `[Menu]`

| Key | Default | Meaning |
|---|---|---|
| `ShowOnline` | `1` | Shows the Online entry on the main menu. |
| `ShowOnlineOpts` | `0` | Shows the online options on the options menu. |
| `ShowSubs` | `0` | Subtitles for English. |
| `ShowMoreCarCategories` | `0` | Debug and preset car categories on the car select screen. |
| `ShowLanguageSelectScreen` | `0` | Language select screen at start-up. |
| `ShowSpecialVinyls` | `1` | Adds a Special category to the vinyls menu. |
| `ShowDebugCarCustomize` | `0` | Debug car customisation on the main menu. |
| `ShowDebugEventID` | `0` | Shows the event ID for races. |
| `ShowTimeOfDay` | `0` | Time of day from the Xbox 360 release, on the video options. |
| `ShowHiddenTracks` | `0` | Hidden tracks, using a different track list. |
| `BetterRandomRaces` | `0` | More cars and tracks in random races. |
| `MaximumBlacklistRival` | `15` | **Expert.** Most Blacklist rivals (0 to 127). |
| `ReplayBlacklistRaces` | `0` | Replay Blacklist boss races in Career. |
| `ChallengeSeriesBlacklist` | `19` | **Expert.** Blacklist bin used for Challenge Series events (0 to 127). |
| `RandomizeCount` | `30` | Count for the Quick Play randomise menu (1 to 500). |
| `SplashScreenTimeLimit` | `30` | Length of the splash screen, in seconds. |
| `CarSelectTireSteerAngle` | `21.6723` | Front tyre angle on the menus. |
| `ShowcaseCamInfiniteZoom` | `0` | Zoom hack for the showcase camera. |
| `ShowcaseCamInfiniteRotation` | `0` | Rotation hack for the showcase camera. |
| `ShowcaseCamAlwaysEnable` | `0` | Showcase camera on every menu. |
| `AllowLongerProfileNames` | `1` | Profile names of up to 15 characters, with spaces and symbols. |
| `DisableTakeover` | `0` | Removes the ExOpts credit from the copyright area. |

### `[Gameplay]`

| Key | Default | Meaning |
|---|---|---|
| `EnableHiddenCameraModes` | `0` | Hidden camera modes, with their names fixed. |
| `EnableDebugCamera` | `0` | Vanilla debug world camera (Backspace toggles). |
| `ShowHUDOnLookBack` | `0` | Keeps the HUD on when looking back, as in Underground 2. |
| `GameSpeed` | `1` | Race speed multiplier. For fun. |
| `WorldAnimationSpeed` | `45` | Speed of world animations such as waving trees. |
| `DebugCamTurboSpeed` | `3.06` | Debug camera speed, turbo. |
| `DebugCamSuperTurboSpeed` | `7.16` | Debug camera speed, super turbo. |
| `CarScale` | `1` | Car model size. For fun. |
| `RemoveOldBridgeBarrier` | `0` | Removes the barrier that closes the road to the Old Bridge. |
| `RemoveNeonBarriers` | `0` | Removes the neon barriers that mark routes or locked areas. |
| `ShowPercentOn1LapRaces` | `1` | Shows "COMPLETED %" instead of "LAP 1/1" in one-lap races. |
| `BetterHandlingWithFlatTires` | `0` | Flat-tyre handling: `0` Most Wanted, `1` Carbon, `2` a 1:1 mix of the two. |
| `StartingCash` | `0` | Starting cash. The 30,000 from the first three races is not included. |
| `AwardedCash` | `10000` | Cash awarded for having an Underground 2 save on the PC. A negative value is a penalty. |
| `WorldMapAnywhere` | `1` | The world map (M) in race and pursuit events. |
| `SkipTrackAnywhere` | `1` | **Experimental.** Skip-track (T) hotkey in the world. |
| `UnlockAllThings` | `0` | Status flag for the unlock-all hack. Set by the hotkey, not meant to be edited. |
| `ForceBlackEdition` | `1` | Forces Black Edition by skipping the `INGAMEC.BUN` check. |
| `UnlockBurgerKingChallenge` | `0` | Unlocks the Burger King challenge without the cheat code. |
| `EnableMaxPerformanceOnShop` | `1` | One key installs the maximum available parts in the Career performance shop. |
| `ShowAllCarsInFE` | `0` | **Expert.** Every car in the car lot, for new saves only. Turn off with many add-on cars. |
| `NoCatchUp` | `0` | **Experimental.** Tries to remove the catch-up. |
| `NoRevLimiter` | `0` | Removes the rev limiter. |
| `NosTrailRepeatCount` | `8` | Taillight trail repeats when NOS is used. High values can break rendering. |
| `SelectableMarkerCount` | `2` | Markers you can pick after beating a Blacklist rival (1 to 6). |
| `PurchasedCarLimit` | `10` | Cars you can buy in the car lot (1 to 25). |
| `ImpoundStrikesCount` | `3` | Impound strikes for newly bought cars (1 to 127). Values other than 3 and 5 do not display well on the car select menu. |
| `MaximumImpoundStrikesCount` | `5` | Most impound strikes a car can have from "Add Impound Strike" markers (1 to 127). Same display caveat. |
| `SBRechargeTime` | `25` | Speedbreaker recharge time, seconds. |
| `SBRechargeSpeedLimit` | `100` | Speed (mph) above which Speedbreaker refills. |
| `SBMassMultiplier` | `2` | Car mass multiplier for Speedbreaker. |

### `[Pursuit]`

| Key | Default | Meaning |
|---|---|---|
| `HeatLevelOverride` | `0` | Forces minimum and maximum heat for every event. |
| `MinimumHeatLevel` | `1` | Minimum heat, when the override is on. |
| `MaximumHeatLevel` | `10` | Maximum heat, when the override is on. |
| `PursuitActionMode` | `0` | Harder in-race pursuits with roadblocks and support units. |
| `ShowPursuitCops` | `1` | Pursuit cops on the minimap. |
| `ShowNonPursuitCops` | `0` | Non-pursuit cops on the minimap. |
| `ShowBustedScreenFrame` | `1` | RPD camera frame on the busted cutscene. |
| `UncensoredBustedScreen` | `0` | Removes the censor from the busted cutscene. |
| `SpeedingLimit` | `67` | Speeding infraction limit, m/s. |
| `ExcessiveSpeedingLimit` | `80.5` | Excessive speeding limit, m/s. |
| `RecklessDrivingLimit` | `89.4` | Reckless driving limit, m/s. |

### `[Weather]`

| Key | Default | Meaning |
|---|---|---|
| `AlwaysRain` | `0` | Forces rain. |
| `GeneralRainAmount` | `1` | Rain multiplier. |
| `RoadReflectionAmount` | `1` | Road reflection in rain. |
| `RainSize` | `0.01` | Size of each drop. |
| `RainIntensity` | `0.45` | Intensity. |
| `RainCrossing` | `0.02` | Crossing amount. |
| `RainSpeed` | `0.03` | Fall speed. |
| `RainGravity` | `0.35` | Gravity. |

### `[Fixes]`

| Key | Default | Meaning |
|---|---|---|
| `DisappearingWheelsFix` | `1` | Wheels stay when focus is lost. Useful for open-wheel cars. |
| `ExperimentalSplitScreenFix` | `0` | **Expert, experimental.** Fixes the split-screen black screen and crash. Race behaviour is not fixed. |
| `HelicopterBountyFix` | `1` | Helicopter bounty and its screen message. |
| `ZeroBountyFix` | `1` | Zero bounty at heat levels 10 and above. |
| `CarSkinFix` | `0` | **Experimental.** Skinless ghost cars. |
| `ImmobileCollisionsFix` | `0` | **Experimental.** Objects with infinite mass. |
| `DDayFix` | `1` | Prologue (DDay) events repeating after they are done. |
| `BustedNISFix` | `1` | Unused busted scenes, by fixing the chooser function. |

### `[Misc]`

| Key | Default | Meaning |
|---|---|---|
| `SkipMovies` | `0` | Skips cutscenes. |
| `SkipNISs` | `0` | Skips some non-interactive scenes. |
| `EnableSound` | `1` | In-game sound. |
| `EnableMusic` | `1` | In-game music. |
| `ShowMessage` | `1` | On-screen messages when a feature is used. |
| `SkipCareerIntro` | `0` | **Expert.** Skips the DDay and Career intro events. |
| `AllowMultipleInstances` | `0` | More than one game instance. |
| `ExpandMemoryPools` | `0` | Larger memory areas. |
| `DoScreenPrintf` | `0` | **Expert, experimental.** On-screen debug text. |
| `ForceMaximumFSAALevel` | `-1` | **Experimental.** Forces full-screen anti-aliasing, `-1` not set, `0` to `4`. |
| `ForceCarLOD` | `-1` | **Expert.** Car level of detail: `-1` none, `0` to `4` for A to E. |
| `ForceTireLOD` | `-1` | **Expert.** Wheel level of detail: `-1` none, `0` to `3` for A to D. |

## Relation to other files in this repository

- **The port's spec.** [specs/extra-options.md](../specs/extra-options.md) defines how this port reads the
  same file, the precedence rules, and which options the port implements. This page is the upstream
  reference; the spec is the port's behaviour.
- **REDUX copy.** The REDUX pack bundles its own `NFSMWExtraOptions.asi` and a settings file. The
  [REDUX reference](redux-v3-plugins.md#nfsmwextraoptionsasi) notes that its `[VisualTreatment]` section is not
  in this upstream file, so the two builds differ.
- **Unlimiter interaction.** The `CarbonStyleRandomCars` option does not work while Unlimiter's Extra
  Customization is enabled. The Unlimiter reference is in [nfsmw-unlimiter.md](nfsmw-unlimiter.md).

## Sources

- Repository `NFSMWExtraOptionsSettings.ini` and `README.md` (`ExOptsTeam/NFSMWExOpts`, `master`).
- Repository file listing and LICENSE (GPL-3.0).
- Repository metadata and releases (`gh api repos/ExOptsTeam/NFSMWExOpts`).
