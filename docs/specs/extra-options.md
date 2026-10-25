# Extra options (NFSMWExtraOptionsSettings.ini)

The NFS:MW Extra Options mod ([ExOptsTeam/NFSMWExOpts](https://github.com/ExOptsTeam/NFSMWExOpts))
exposes hidden and extra options through `NFSMWExtraOptionsSettings.ini`. This port reads the same file
format, so a user's copy works as-is. It is not a copy of the mod: nothing is patched or injected into a
running install, and each option is reimplemented against this port's own systems only where those
systems exist.

## Reading the file

- Location: `NFSMWExtraOptionsSettings.ini` in the same directory as `config.toml`
  (`nfsmw check-install` prints the config path).
- Format: `[Section]` headers, `Key = value` lines, `//` comments, as the mod writes them. Switches are
  `0`/`1` (or `true`/`false`). A bad value is ignored and the default applies.
- Precedence: the command line, then environment variables, then `config.toml`, then this file, then the
  defaults. A file option never overrides an explicit setting.
- Two systems read their values from this file directly, not through the settings layers: the Speedbreaker
  (a fallback under its own `NFSMWSpeedFixerSettings.ini`) and the pursuit (heat range and infraction speeds;
  the port has no other file for them). The file is read once (`extra_options::active`).
- Absence means no change. The mod's shipped defaults are not applied as defaults where the port's vanilla
  value differs (see the speeding limit below).
- Options the port does not implement are logged once at start-up, listed by name, so nothing is dropped
  silently.

Code: [`crates/nfsmw/src/settings/extra_options/`](../../crates/nfsmw/src/settings/extra_options) (the
parser is pure text in, typed values out; the file read is the only I/O). The Speedbreaker mapping is in
`breaker.rs`, the pursuit mapping in `pursuit.rs`.

## Applied

| Key | Effect | Notes |
|---|---|---|
| `[Misc] SkipMovies` | skips the start-up movie sequence (`skip_intro`) | The mod's key also skips in-race cutscenes; the port has the start-up skip only. |
| `[Misc] EnableSound` | `0` mutes everything (master volume 0) | `1` leaves the volumes to the config file. |
| `[Misc] EnableMusic` | `0` mutes the music (music volume 0) | `1` leaves the volume to the config file. |
| `[Weather] AlwaysRain` | forces rain on every track | Rain is drawn from the drop pool (see [weather.md](weather.md#the-ports-rain-what-is-drawn)); the original's zone rain map is not reproduced yet. |
| `[Weather] GeneralRainAmount` | scales the drop spawn rate; 0 to 100 percent (values above 1 clamp) | |
| `[Gameplay] GameSpeed` | multiplies the physics clock; 0.1 to 4 | Applies to the drive's physics steps (cars and collisions). Traffic and effects keep frame time. |
| `[Gameplay] WorldAnimationSpeed` | multiplies the world animation clock (textures, animated scenery and the [scenery sway](scenery-sway.md) of trees and boats); 0.1 to 4 | The mod measures it in its own unit, where 45 is the original rate (the sway clock's degrees per second, at `0x904AEC`); the port divides by 45. |
| `[Gameplay] CarScale` | scales the player car's body parts; 0.25 to 4 | Visual only: the lamps, effects and collision keep the stock size. |
| `[Gameplay] NoRevLimiter` | lifts the engine's rev limiter: the engine turns up to the top of its torque curve | Uses `EngineSpec::no_rev_limiter`. Shifting still happens at the red line. The example car's drag stops it short of the red line, so the effect on top speed is unverified in-game. |
| `[Gameplay] ShowHUDOnLookBack` | keeps the HUD up while the look-back control is held | Without it the HUD hides while looking back, as the original does. Look back is hold `B`. |
| `[Gameplay] ShowTurnSignals` | shows turn signals on cars while steering | |
| `[Hotkeys] Headlights` | the headlights key, as a Windows virtual-key code (72 is H) | Maps the codes for the letters, digits, function keys and common controls (`input/vk.rs`); other codes keep H. |
| `[Gameplay] SBRechargeTime` | Speedbreaker passive recharge: seconds for an empty meter to fill; 0.001 to 3600 | Fallback for the SpeedFixer file's `[Recharging] passiveTime`. The file's key wins. Setting it turns passive recharge on when the SpeedFixer file has no recharge keys. The mod's default (25) is the vanilla value. |
| `[Gameplay] SBRechargeSpeedLimit` | Speedbreaker passive recharge: the speed in mph above which the meter refills; 0 to 1000 | Fallback for `[Recharging] minCarSpeed`, converted from mph. The file's key wins. Same recharge rule as `SBRechargeTime`. The mod's default (100) is the vanilla value. |
| `[Gameplay] SBMassMultiplier` | the car's collision mass scale while the Speedbreaker is active; 0 to 100 | Fallback for `[Physics] carMassScale`. The file's key wins. The mod's default (2) is the vanilla value. |
| `[Pursuit] HeatLevelOverride`, `MinimumHeatLevel`, `MaximumHeatLevel` | with the override on, every chase starts at the minimum heat and is held to the maximum | Levels round to whole numbers from 1 to 10; an absent end keeps the vanilla 1 or 10. The range is read when a chase begins (`ai/police.rs`, `begin`). The override off, or absent, keeps the vanilla range. |
| `[Pursuit] SpeedingLimit` | the speed (m/s) over which the Speeding infraction is recorded; 0 to 1000 | Replaces the port's threshold for the infraction only. The pursuit trigger (`ai/traffic_events.rs`, 29.06 m/s) is unchanged. Absent keeps 29.06. The mod's default of 67 is higher than the port's 29.06, so the mod's default is not applied. See open questions. |
| `[Pursuit] RecklessDrivingLimit` | the speed (m/s) over which the Reckless Driving infraction is recorded; 0 to 1000 | The port has the infraction and its fine but no speed rule for it, so this option adds the rule. Absent, no Reckless Driving is recorded. The mod's default of 89.4 is not applied. |

Headlights are toggled with the `ToggleHeadlights` action (H by default, rebindable like the others).
The look-back control is the `LookBack` action (B by default).

## Not applied

Re-audited against the current code. The status is one of: **blocked** (the port lacks the feature or the
value's place; the reason is given), or **mappable, hook not made** (the value maps onto a port system, but its
hook sits outside the files of the pass that made this table; the hook is named).

| Section | Keys | Status | Why |
|---|---|---|---|
| Hotkeys | `HeatLevelHack`, `ChangeHeatLevel` | blocked | No heat-hack action in the port. The forced range is now read (`[Pursuit]`), but nothing binds a hotkey to it. |
| Hotkeys | `ToggleCops`, `CopLights`, `FreezeCar`, `FreezeCamera`, `UnlockAllThings`, `AutoDrive`, `EnableSaveLoadHotPos` | blocked | No matching port action was found (auto drive, saved positions and a cop-light toggle were searched for by name). The freeze and unlock words occur in unrelated code only. Each binds once its feature exists. |
| LapControllers, OpponentControllers | lap and opponent counts, `MoreCarsForOpponents`, `CarbonStyleRandomCars`, `MaxUniqueOpponentCars`, `UnfreezeKO` | blocked | The quick race menu's limits are not checked against the mod here; there is no random race screen with these counts. |
| TrafficControllers | `Low`, `Medium`, `High` | blocked | The port's traffic is one count, `settings.traffic`. The density the manager uses (`ai/manager.rs`, `density(wanted, pursuit)`) is game state, 0 to 1, not a setting. The presets need a selector to pick one; the port has none, and picking one by default would change behaviour. |
| Menu | all | blocked | The port's own menus and splash. Options to show or hide original menu entries do not apply. |
| Gameplay | `NosTrailRepeatCount` | blocked | No nitrous taillight trail in the port yet ([vehicle-effects.md](../vehicle-effects.md)). |
| Gameplay | `BetterHandlingWithFlatTires` | blocked | Punctured tyres have no physics effect (`libs/blackbox-vehicle/src/tires/damage.rs`); there is no flat-tyre grip factor to replace, and the mod's second handling model is not given. |
| Gameplay | `StartingCash`, `AwardedCash`, `PurchasedCarLimit`, `ImpoundStrikesCount`, `MaximumImpoundStrikesCount`, `SelectableMarkerCount`, `ShowAllCarsInFE`, `UnlockBurgerKingChallenge`, `EnableMaxPerformanceOnShop` | blocked | No career, cash or car lot in the port (no `cash` in the code). |
| Gameplay | `EnableDebugCamera`, `EnableHiddenCameraModes`, `DebugCamTurboSpeed`, `DebugCamSuperTurboSpeed` | blocked | The port has a free camera (`freecam`). The speed keys could map onto its speed, which is not checked here; the mod's hidden modes are not reproduced. |
| Gameplay | `WorldMapAnywhere` | blocked | The port's world map is free-roam only (`input/action.rs`); enabling it in races is a change to the action's gate, not made here. |
| Gameplay | `SkipTrackAnywhere`, `RemoveOldBridgeBarrier`, `RemoveNeonBarriers`, `ShowPercentOn1LapRaces`, `NoCatchUp`, `ForceBlackEdition` | blocked | No skip-track action, no barriers to remove, and the race-end text and catch-up behaviour are not audited. |
| Pursuit | `PursuitActionMode` | not audited | Roadblocks and support units exist (`blackbox-pursuit`); the mod's harder-pursuit values are not mapped yet. |
| Pursuit | `ShowPursuitCops`, `ShowNonPursuitCops` | not audited | The minimap has modes (`settings/minimap.rs`); the cop-blip filter is not mapped yet. |
| Pursuit | `ShowBustedScreenFrame`, `UncensoredBustedScreen` | blocked | The busted cutscene's frame and censor are not in the port. |
| Pursuit | `ExcessiveSpeedingLimit` | blocked | The port has no Excessive Speeding infraction (`blackbox-pursuit/src/infraction.rs` lists eight, none of them this one). Adding one is a change to the infraction set and its fines. |
| Weather | `RainSize`, `RainSpeed`, `RainIntensity` | mappable, hook not made | `weather/rain.rs` and `weather/drops.rs`: `RainSpeed` onto the fall speed (`FALL_SPEED`), `RainIntensity` onto the spawn rate, `RainSize` onto the streak size (`STREAK_HALF_WIDTH`). Each as a ratio to the mod's default, the way `WorldAnimationSpeed` is done. The hook is in `weather/`, not in this pass. |
| Weather | `RainCrossing` | blocked | The drop pool's wind is `wind_scale`, which is not the mod's crossing amount; the meaning is not confirmed. |
| Weather | `RainGravity` | blocked | The drop pool has no gravity term; it falls at a fixed speed. |
| Weather | `RoadReflectionAmount` | not audited | The road's wet reflection is in the renderer. |
| Fixes | all | blocked | The fixes repair bugs of the original game. Most of them are not reproducible in this port, which has its own code. |
| Misc | `SkipNISs`, `ShowMessage`, `SkipCareerIntro`, `AllowMultipleInstances`, `ExpandMemoryPools`, `DoScreenPrintf`, `ForceMaximumFSAALevel` | blocked | The port has its own cutscene flow, graphics and multi-instance behaviour. |
| Misc | `ForceCarLOD` | mappable, hook not made | `nfsmw_data::car::LoadOptions::lod` takes the level letter, and `scenes/world/car_loader.rs` hard-codes `'A'`. An override of `'A'` to `'E'` (from `0` to `4`) maps onto it. The hook is in `car_loader.rs`, not in this pass. |
| Misc | `ForceTireLOD` | not verified | The wheels are placed from the car's own LOD (`nfsmw-data/src/car/wheels.rs`, `assemble.rs`); no separate tyre LOD choice was found, so this is unverified. |
| Menu | `ShowOnline`, `ShowSubs`, `SplashScreenTimeLimit`, `CarSelectTireSteerAngle`, … | blocked | Menu presentation of the original, not the port's. |

## Open questions

- The mod's default speeding limits (67, 80.5, 89.4 m/s) are 241, 290 and 322 km/h. The port's own speeding
  threshold is 29.06 m/s (65 mph). Either the mod's unit or its meaning differs from the port's; the port
  takes the file's number as m/s, as the mod's documentation states, and keeps 29.06 when absent. Check the
  values in-game before using them.
- `SpeedingLimit` changes the infraction only. The mod may also use it for the pursuit trigger; the port
  keeps its own trigger.
- The forced heat range and infraction limits are read through the `active()` cache, so a unit test that starts a
  chase will read the tester's own file if it sets them.

## Tests

`cargo test -p nfsmw extra_options`: section and comment parsing, the switch values, the layer for each
applied key, the reporting of unimplemented keys, clamping of the rain amount, and a bad value falling
back to the default. The Speedbreaker and pursuit mappings have their own tests (`tests_breaker.rs`,
`tests_pursuit.rs`), including the recharge fallback under the SpeedFixer file's keys. `cargo test -p nfsmw
input::vk` covers the virtual-key mapping; `input::headlights_tests` covers the rebind.
