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
- Options the port does not implement are logged once at start-up, listed by name, so nothing is dropped
  silently.

Code: [`crates/nfsmw/src/settings/extra_options/`](../../crates/nfsmw/src/settings/extra_options) (the
parser is pure text in, typed values out; the file read is the only I/O).

## Applied

| Key | Effect | Notes |
|---|---|---|
| `[Misc] SkipMovies` | skips the start-up movie sequence (`skip_intro`) | The mod's key also skips in-race cutscenes; the port has the start-up skip only. |
| `[Misc] EnableSound` | `0` mutes everything (master volume 0) | `1` leaves the volumes to the config file. |
| `[Misc] EnableMusic` | `0` mutes the music (music volume 0) | `1` leaves the volume to the config file. |
| `[Weather] AlwaysRain` | forces rain on every track | Rain is drawn from the drop pool (see [weather.md](weather.md#the-ports-rain-what-is-drawn)); the original's zone rain map is not reproduced yet. |
| `[Weather] GeneralRainAmount` | scales the drop spawn rate; 0 to 100 percent (values above 1 clamp) | |
| `[Gameplay] GameSpeed` | multiplies the physics clock; 0.1 to 4 | Applies to the drive's physics steps (cars and collisions). Traffic and effects keep frame time. |
| `[Gameplay] WorldAnimationSpeed` | multiplies the world animation clock (textures and animated scenery); 0.1 to 4 | The mod measures it in its own unit, where 45 is the original rate; the port divides by 45. |
| `[Gameplay] CarScale` | scales the player car's body parts; 0.25 to 4 | Visual only: the lamps, effects and collision keep the stock size. |
| `[Gameplay] NoRevLimiter` | lifts the engine's rev limiter: the engine turns up to the top of its torque curve | Uses `EngineSpec::no_rev_limiter`. Shifting still happens at the red line. The example car's drag stops it short of the red line, so the effect on top speed is unverified in-game. |
| `[Gameplay] ShowHUDOnLookBack` | keeps the HUD up while the look-back control is held | Without it the HUD hides while looking back, as the original does. Look back is hold `B`. |
| `[Hotkeys] Headlights` | the headlights key, as a Windows virtual-key code (72 is H) | Maps the codes for the letters, digits, function keys and common controls (`input/vk.rs`); other codes keep H. |

Headlights are toggled with the `ToggleHeadlights` action (H by default, rebindable like the others).
The look-back control is the `LookBack` action (B by default).

## Not applied yet

The reason is the gap in this port, not a judgement of the option:

| Section | Keys | Why not yet |
|---|---|---|
| Hotkeys | the other hotkeys (`HeatLevelHack`, `ChangeHeatLevel`, `ToggleCops`, `CopLights`, `FreezeCar`, `FreezeCamera`, `UnlockAllThings`, `AutoDrive`, `EnableSaveLoadHotPos`) | Their features are not in the port (pursuit heat, cops, freeze, auto drive, unlocks, saved positions). The code mapping exists, so each can bind once its feature does. |
| LapControllers, OpponentControllers | lap and opponent counts, `MoreCarsForOpponents`, `CarbonStyleRandomCars`, `MaxUniqueOpponentCars` | No career or quick-race menus with these choices. |
| TrafficControllers | `Low`, `Medium`, `High` | The port's traffic is one count (`traffic`), not the mod's density presets. A preset menu would map onto it. |
| Menu | all | The port's own menus and splash. Options to show or hide original menu entries do not apply. |
| Gameplay | `NosTrailRepeatCount` | The port has no nitrous taillight trail yet (see [vehicle-effects.md](../vehicle-effects.md)). |
| Gameplay | `BetterHandlingWithFlatTires` | Punctured tires exist ([vehicle-damage.md](vehicle-damage.md#punctured-tires)); the mod's flat-tire handling values are not implemented. |
| Gameplay | `SBRechargeTime`, `SBRechargeSpeedLimit`, `SBMassMultiplier` | The Speedbreaker has no port equivalent. |
| Gameplay | `StartingCash`, `AwardedCash`, `PurchasedCarLimit`, `ImpoundStrikesCount`, `MaximumImpoundStrikesCount`, `SelectableMarkerCount`, `ShowAllCarsInFE`, `UnlockAllThings`, `UnlockBurgerKingChallenge`, `EnableMaxPerformanceOnShop` | No career, cash or car lot in the port. |
| Gameplay | `EnableDebugCamera`, `EnableHiddenCameraModes`, `DebugCamTurboSpeed`, `DebugCamSuperTurboSpeed` | The port has a free camera (`freecam`); the mod's hidden modes are not reproduced. |
| Gameplay | `WorldMapAnywhere`, `SkipTrackAnywhere`, `RemoveOldBridgeBarrier`, `RemoveNeonBarriers`, `ShowPercentOn1LapRaces`, `NoCatchUp`, `ForceBlackEdition` | No race map, skip-track action or barriers to remove in this port. |
| Pursuit | all | The pursuit systems (`blackbox-pursuit`) are under construction; heat limits and the infraction speed limits will map to them. |
| Weather | `RoadReflectionAmount`, `RainSize`, `RainIntensity`, `RainCrossing`, `RainSpeed`, `RainGravity` | The drop pool uses the port's own motion constants; the mod's rain shape and wind settings are not mapped yet. |
| Fixes | all | The fixes repair bugs of the original game. Most of them are not reproducible in this port, which has its own code. |
| Misc | `SkipNISs`, `ShowMessage`, `SkipCareerIntro`, `AllowMultipleInstances`, `ExpandMemoryPools`, `DoScreenPrintf`, `ForceMaximumFSAALevel`, `ForceCarLOD`, `ForceTireLOD` | The port has its own cutscene flow, graphics and multi-instance behaviour. `ForceCarLOD` and `ForceTireLOD` would map onto the car model loader's LOD choice. |
| Menu | `ShowOnline`, `ShowSubs`, `SplashScreenTimeLimit`, `CarSelectTireSteerAngle`, … | Menu presentation of the original, not the port's. |

## Tests

`cargo test -p nfsmw extra_options`: section and comment parsing, the switch values, the layer for each
applied key, the reporting of unimplemented keys, clamping of the rain amount, and a bad value falling
back to the default. `cargo test -p nfsmw input::vk` covers the virtual-key mapping; `input::headlights_tests`
covers the rebind.
