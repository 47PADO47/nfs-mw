# Custom Race

The Quick Race menu's second icon ("Custom Race", `MODE_ICON_QUICK_RACE`) opens a chain of screens: the **mode select**, the
**track list**, the **options**, the **car select** (with the **Showcase**), and the transmission question; then the race starts.

- **Code:** `crates/nfsmw/src/frontend/custom_race/` (`mode.rs`, `region.rs`, `tracks.rs`, `settings.rs`, `options.rs`, `state.rs`,
  `car_list.rs`, `track_select.rs`, `track_options.rs`, `car_select.rs`, `showcase.rs`), `flow_custom.rs` (the start).
- **Where the exe and the decomp were read:** [provenance/custom-race.md](../provenance/custom-race.md).
- **Evidence tags:** **[exe]** read in `speed.exe`. **[decomp]** read in the CC0 decompilation
  [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (`uiQRModeSelect`, `uiQRTrackSelect`, `uiQRTrackOptions`,
  `uiQRCarSelect`, `uiShowcase`, `UnlockSystem`, `FEDatabase`, `VehicleDB`), read for understanding, no code copied.
  **[verified]** seen in the install or in the reference screenshots. **[ours]** this rewrite's choice.

## 1. The flow and the mode select

Quick Race menu, Custom Race: `MainMenu_Sub.fng` with `Sub::CustomRace` (an icon row, title `custom race mode select`
`0x1F203817`). Six icons, in the order of the screenshots **[verified]**: Circuit, Sprint, Drag, Tollbooth, Lap Knockout,
Speedtrap. The decomp's `UIQRModeSelect::Setup` lists circuit, sprint (`P2P`), drag, knockout and speedtrap and gives the
tollbooth to a build variant only **[decomp]**; the install's PC build lists it **[verified]**.

| Mode | Icon | Name label | Track list title | Options title | Tutorial |
|---|---|---|---|---|---|
| Circuit | `0xE9638D3E` | `0x034FA2C1` | `0x3DE80A85` | `0xA6ED015D` | none |
| Sprint | `0x2521E5EB` | `0xB94FD70E` | `0xC2D85652` | `0xB80BFC8A` | none |
| Drag | `0xAAAB31E9` | `0x6F547E4C` | `0x136C5C90` | `0xEC86E188` | `drag_tutorial` |
| Tollbooth | `0x1A091045` | `0xA15E4505` | `0xE3AFADC9` | `0x141EDFE1` | `tollbooth_tutorial` |
| Lap Knockout | `0x3A015595` | `0x4930F5FC` | `0xD6D65640` | `0x0CC959B8` | none |
| Speedtrap | `0x66C9A7B6` | `0xEE1EDC76` | `0x3070453A` | `0xF2745852` | `speedtrap_tutorial` |

Accept on an icon picks the mode (`FEDatabase->RaceMode`) and opens the track list; Back returns to the Quick Race menu. The
tutorial button (key `2`, `PAD_BUTTON4`, footer `2 View Tutorial`) plays the movie of the selected mode full screen over the
menus and comes back to the mode select with the selection kept, as the challenge series does **[ours]**: the three movies
(`MOVIES/drag_tutorial`, `tollbooth_tutorial`, `speedtrap_tutorial`) are in the install **[verified]**. Which modes offer a
tutorial is the request's; the exe's own table was not traced.

## 2. What the screens remember

`custom_race/state.rs`: the mode, the region filter, the picked track and the options **of each mode**, and the picked car, as
the original keeps them in `cFrontendDatabase` (`RaceMode`, `TheQuickRaceSettings[i]`). They live for the session; nothing is
saved to the config file (the old `quick_race` config key and its four rows are gone with the old Quick Race screens).

## 3. The track list (`Track_Select.fng`)

- **Title** the mode's `<mode> track select`; the kind line is the mode's name and icon; the counter is `n/total` (the package's
  own static "9/9" counter is hidden: the screenshot shows both overlapping, which is the original's bug, not copied) **[ours]**.
- **The list:** the races of the mode whose Blacklist bin is 1 to 15, in the region filter, **from bin 15 down to bin 1**, and within
  a bin in the board's event order **[decomp]** (`BuildPresetTrackList` walks bin `0x15` and then 15 to 1; `IsEventAvailable` refuses
  the collector's and the unlisted events, and the prologue, the challenge series and the online bins are not walked). The
  challenge series' tollbooth trials (bin 19) are not custom races.
- **Region** (`RegionFilterBits`): 0 College `0xA6850651`, 1 Coastal `0xA5C20E7D`, 2 City `0x8663FAEF`, 3 All Regions `0x632DD19B`
  **[decomp]**, the default being All **[verified]**. The shoulder buttons step with wrap-around (keys `Page Up` and `Page Down`, or
  a click on the arrows beside the label).
- **Left and right** step through the tracks with wrap-around; the track name is the label `TRACKNAME_<id>` or the event id.
- **Length** `%.1f` of the stored `RaceLength` in KMs or Miles (the Speed units setting) **[decomp]**, as the Quick Play brief does.
- **Record** **[exe]** (`FUN_007AADE0`): the sprint, drag and tollbooth show `Best Time:` with the best finish time (`m:ss.cc`); the
  circuit and the knockout swap the label for `Best Lap:` (`0xC5B5A177`) and show the best lap; the speedtrap swaps it for
  `Best Speed:` (`0x0512E823`) and shows the best speed through a speed trap in the Speed units setting. `0.00` and `0 MPH`
  for an event never raced, as the screenshots show **[verified]**. The records are `records.toml` next to the config file
  (`custom_race/records.rs`): the world notes the finish time and the lap times when a race ends (any race, as the original's race
  scores are) and each speed-trap pass; only a better one is kept.
- **Map:** `TRACK_MAP_<event id>` of `TrackMaps.bin`, eased to the route's box as the challenge series does
  ([challenge-series.md](challenge-series.md) section 3).
- **Locked tracks.** The exe keeps a score flag per race (bit 4 is "open in the quick race", `FUN_005DCA70` and the checker
  `FUN_005765A0`, `FUN_00576790`) and sets it for a fresh profile from the race's own flags: `InitiallyUnlocked` or either of
  the flags `0xB39ED8C3` and `0xEA855EAF` (`FUN_00600B20`) **[exe]**. In this install that opens 28 races of bins 1 to 15 from the
  start (drag, circuit, sprint, knockout, speedtrap and tollbooth ones), and `NeverInQuickRace` (`0xA4E6FCFD`, `FUN_005FC110`)
  keeps a race out of the lists (none here). The other races are opened by the bin scripts: the career action `UnlockRace`
  (`0x00612140`, registered by name; [career-blacklist.md](career-blacklist.md) sections 2.3, 2.6 and 4.1) sets flag 4 unless the
  race is `NeverInQuickRace`, and the scripts call it for the races open from the start, for the boss races of a bin when its
  rival is unlocked (`rival_unlocked`) and for the free races when the rival is beaten (`done`) **[exe]**. So a free track of bin
  `b` is open once rival `b` is beaten, i.e. the career's stage is below `b`, and a boss track already when the rival is
  unlocked (`tracks::Standing`). The lock sentence `Unlock this track by beating <alias>, Blacklist member <b>` (label
  `0xBD563BE5`, `BLACKLIST_RIVAL_<bb>_AKA`) names the bin **[exe, verified]**.
  A locked track hides the Accept hint and refuses Accept. The environment variable `NFSMW_UNLOCK_ALL` opens everything for
  testing, as the original's `UnlockAllThings` cheat does (`DAT_00926124` in the checker **[exe]**).
- **Accept** on an open track stores it for the mode and plays the package's leave script (`EVENT_HANDLER` forward); on
  `EXIT_COMPLETE` the options open. **Back** clears the pick and returns to the mode select.

## 4. The options (`Track_Options.fng`)

The rows run on the settings list widget; their values are `custom_race/settings.rs`. Rows per mode, in the exe's order **[exe]**
(`FUN_007AC8B0` circuit, `0x007ACAD0` sprint, `0x007ACCB0` drag, `0x007ACE50` knockout, `0x007AD0F0` speedtrap, `0x007AD310`
tollbooth; widget classes by their vtables):

| Mode | Rows |
|---|---|
| Circuit | Laps, Traffic Level, Opponents, Difficulty, Catch Up, Heat Level |
| Sprint, Speedtrap | Traffic Level, Opponents, Difficulty, Catch Up, Heat Level |
| Drag | Opponents, Difficulty, Catch Up, Heat Level |
| Lap Knockout | Opponents, Laps (greyed out, equal to the opponents), Traffic Level, Difficulty, Catch Up, Heat Level |
| Tollbooth | Traffic Level, Heat Level |

A Track Direction row comes first when the race can be reversed **[exe]**; no race of this install can (`racestartReverse` is 0
in all of them, `gameplay-races.md`), so it never shows. **Heat Level** is not in the exe's setup: it is in the install's
screenshot of the circuit options **[verified]** and is added at the end of every mode **[ours]**.

| Row | Values | Default | Source |
|---|---|---|---|
| Laps | 1 to 8, wrapping | 1 | **[decomp]** max 8; default 1 **[verified]** |
| Opponents | 1 to 3, wrapping | 3 | **[decomp]** (4 minus the players) |
| Difficulty | Easy, Medium, Hard (`0x61973E01`, `0x3747F6D0`, `0x6198E2EE`) | Medium | **[decomp]** |
| Traffic Level | None, Minimum, Moderate, Maximum (`0x8CDC3937`, `0x73C615A3`, `0xA2CCA838`, `0x61D1C5A5`) | Minimum | **[decomp]** |
| Catch Up | On / Off | On | **[decomp]** |
| Heat Level | None, Minimum, Moderate, Maximum | Minimum | **[verified]** default |

The titles are the labels `Laps 0x48494E83`, `Traffic Level 0xEB9DFC09`, `Opponents 0x3384A679`, `Difficulty 0x4D156786`,
`Catch Up 0x8B8E913A`, `Heat Level 0x251A34F8`. **Defaults** (key `1`, `PAD_BUTTON5`) asks `Are you sure you want to reset to
defaults?` (`0x8AEF5AE8`, Yes/No) and puts the current mode's rows back. The knockout runs as many laps as it has opponents.

**What the race gets** (`RaceOptions`): the circuit's and the knockout's laps; the opponents, the difficulty (33, 66 or 100) and
the catch-up switch for the modes that have the row; the traffic as `FillCustomRace` writes it, 0, 10, 30 or 50 **[exe]** (`ai-traffic-spawning.md`).
Heat Level Moderate or Maximum leaves the road's police on and gives the car the heat 3 or 5 for a chase that starts in the race
(`race/start.rs`); None and Minimum call off a pursuit at the start of the race **[ours]**: the exe's `FillCustomRace` enables cops
only when `CopsOn` is set, which the screens of this build do not expose. The police of free roam are the ones that appear.

Back returns to the track list, Accept opens the car select.

## 5. The car select (`Car_Select.fng` in its quick race mode)

The package is the career car lot's (`career_screens/`), opened with `Args::custom`; the logic is `car_select.rs`. At open the
package gets the message `0xDE511657` (the quick race look: the list header, the counter and the up/down arrows).

- **Lists** (`FE_CAR_FILTER_LIST_*`): Stock, Career, My Cars, Bonus, Preset (and a Debug list, not offered) **[decomp]**. **Stock**
  is every car of the catalogue with a folder and readable physics; **Career** the cars the loaded profile owns (empty with none
  loaded); **My Cars** the quick race garage: the profile's car records with the quick race flag `0x4` ([career-car-lot.md](career-car-lot.md)
  section 7; empty when none is loaded or none was saved, since this build has no editor that makes them); **Bonus** the rivals'
  cars `BL2` to `BL15` and `BONUS_C6R`, `BONUS_SL65`, `BONUS_GT2`, `CASTROLGT`; **Preset** the career-start M3 GTR
  (`M3GTRCAREERSTART`). Up and down change the list with wrap-around, in that order; the original's own scroll skips Preset
  **[decomp]**, ours includes it **[ours]**. An empty list shows the package's "No cars in list." panel (messages `0xD9420CD5` and
  `0x7C4583DC`, sentence `0x58BBED2A`; My Cars says to add cars from My Cars in the main menu, `0x0DA87B01`) **[decomp]**.
- **First car:** the Preset list's M3 GTR, `1/1` **[verified]**; afterwards the car chosen last.
- **List header:** the label and picture of the list (Stock `0xD9D6B954`/`0x3A541F7F`, Bonus `0xD8A058F7`/`0xBE5AD8A2`, Preset
  `0x0D8500C3`/`0x03704F3D`) **[decomp]**; the counter, the manufacturer logo (`0x3E01AD1D`) and the model-name picture
  (`0xB05DD708`) as the Quick Play brief ([quick-play.md](quick-play.md) section 4); the three bars as the car lot's.
- **Unlocks:** a stock car is open when its rank is at least the stage, or it is one of the starters (RX-8, Impreza WRX, Mustang GT,
  SL 500, 997 S, IS 300, GTI, Gallardo, Cobalt SS, Punto) **[decomp]** (`QuickRaceUnlocker::IsCarUnlocked`). A rival's car `BLn` is
  open when the stage is below `n`; the C6R when the career is won; the SL65 when event 68 of the challenge series (`19.8.68`) is
  won and the GT2 when every event of the series is won (the two hidden ones, `19.8.31` and `19.9.70`, are not needed **[ours]**), from
  the wins `challenge_series.toml` keeps **[decomp]** (the sentences say "completing event number 68" and "every event"); the
  Castrol GT (an easter egg this build has not) and the collector's cars stay locked **[decomp, ours]**.
  A locked car shows `Unlock this car by beating <alias>, Blacklist member <n>.` or the bonus sentences of the Quick Play brief; Accept
  and Showcase refuse it.
- **Keys:** left and right the cars, up and down the lists, Accept, Back (to the options), `3` Showcase (`PAD_BUTTON0`).
- **Accept** plays the package's leave (message `0x2E76EDFB`); on `EXIT_COMPLETE` the race starts after the transmission question.
  Back sends `0x93E8A57C` and returns to the options.
- **The backdrop** shows the picked car (`ScreenLogic::look`), with its preset.

## 6. The Showcase (`Showcase.fng`)

`3` on an open car plays the package's transition (`0x89D0649C`) and opens the Showcase over the same car. The package shows the
manufacturer logo and the model picture (the rival block of the career's Showcase stays hidden). The backdrop camera turns round
the car with the arrow keys, tilts, and zooms with `9` and `0` (`Look::Showcase`, `scene.rs`); footer: `Back`, `9 0 Zoom`,
arrows `View Car` **[verified]** (screenshot 5). The shoulder buttons zoom on a pad. Accept or Back returns to the car select with
the same car **[decomp]**. The rates and limits of the camera are ours.

## 7. Transmission and start

After the car: `Select your transmission.` (`0x6F5401D1`) with Manual (`0x317D3005`) and Auto (`0x8CD532A0`), the current setting
focused, the choice stored as the transmission setting, as the challenge series does **[exe]**. Every kind of race asks, the drag
race too **[ours]**: the decompiled `UIQRCarSelect` starts a drag at once with the setting, but the request says the question
follows the car, and the exe's own car select could not be read (provenance, dead ends). The race starts through the
loading screen (`Command::StartCustomRace`): the car (folder and preset, `CAR:PRESET` to the world's loader), the options above,
no career, and the pause menu's Quit goes to the Quick Race menu.

## 8. Open

- Whether the original asks the transmission question for a drag race: the decompilation says no, this build asks.
- Heat Level's exact meaning in the exe: the screenshot shows the row, the exe's setup does not; the police are those of free roam
  at the heat the level gives.
- The Castrol GT: the original awards it for an easter egg the world has not here, so it stays locked.
- Split Screen is not built (greyed out).
