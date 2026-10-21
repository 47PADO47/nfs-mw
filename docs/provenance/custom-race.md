# Custom Race

- **Spec:** [specs/custom-race.md](../specs/custom-race.md).
- **Method:** (1) the CC0 decompilation [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), files
  `Frontend/MenuScreens/Safehouse/quickrace/uiQRModeSelect.cpp`, `uiQRTrackSelect.cpp`, `uiQRTrackOptions.cpp`,
  `uiQRCarSelect.cpp`, `uiShowcase.cpp`, `uiQRPressStart.cpp`, `Frontend/Careers/UnlockSystem.cpp`,
  `Frontend/Database/FEDatabase.cpp`, `RaceDB.cpp`, `VehicleDB.cpp`, `RaceStarter.cpp`; read for understanding, no code copied.
  (2) Ghidra on `/speed.exe` (project `nfsmw`), 2026-10-21, with a script that scans the instructions for the language-label
  immediates of the option widgets. (3) `nfsmw dump-screen` of `Track_Select.fng`, `Track_Options.fng`, `Car_Select.fng`,
  `Showcase.fng`; `nfsmw strings 0xHASH` for every label. (4) The five reference screenshots of the original game.
- **Implemented:** `crates/nfsmw/src/frontend/custom_race/`, `flow_custom.rs`.

## Addresses read in `speed.exe`

| Address | What it gave |
|---|---|
| `0x007AC080` | The Opponents widget: its title label `0x3384A679`. |
| `0x007AC150` | The Difficulty (`AISkill`) widget: `0x4D156786`. |
| `0x007AC230` | The Catch Up widget: `0x8B8E913A`. |
| `0x007AC310` | The Traffic Level widget: `0xEB9DFC09`. |
| `0x007AC450` | The Laps widget: `0x48494E83`. |
| `0x007AC580` | The Track Direction widget: `0xA88FFEB4`. |
| `0x007AC5F0` | The online rows (a Laps widget and the online-only ones); not used offline. |
| `0x007AC8B0` | `SetupCircuit`: Direction (when the race can be reversed, `FUN_005FAC80`), Laps (`0x008B788C`), Traffic (`0x008B7850`), then, unless the split-screen test (`flags & 4` and the player count 2) holds, Opponents (`0x008B779C`), Difficulty (`0x008B77D8`), Catch Up (`0x008B7814`). |
| `0x007ACAD0` | `SetupSprint`: Direction, Traffic, Opponents, Difficulty, Catch Up. |
| `0x007ACCB0` | `SetupDrag`: Direction, Opponents, Difficulty, Catch Up. |
| `0x007ACE50` | `SetupKnockout`: Direction, Opponents, Laps (its text object `0x58` set to the opponents' string, the widget disabled), Traffic, Difficulty, Catch Up. |
| `0x007AD0F0` | `SetupSpeedTrap`: Direction, Traffic, Opponents, Difficulty, Catch Up. |
| `0x007AD310` | `SetupTollbooth`: Direction, Traffic. |
| `0x007AADE0` | `UIQRTrackSelect::RefreshHeader`: the region labels, the counter, the lock text (`blacklist_rival_%02d_aka` with label `0xBD563BE5`, shown when the list node's byte at `+0xC` is set, the accept group `0x73DCB662` hidden), the length, and the record line: a time for race types 0 to 4 (`FUN_007C4B80`), `Best Speed:` with the speed in the speed unit for type 5, the `Best Lap:` label swapped in for types 1 and 3. |
| `0x007AAC70` | `TryToAddTrack`: the event is available (`FUN_00576A20`), `NeverInQuickRace` clear (`FUN_005FC110`, attribute `0xA4E6FCFD`), the race type equals the mode, not a D-Day race (`FUN_005FBD20`), the region equals the filter or the filter is 3; the node's lock byte is `IsTrackUnlocked == 0`. |
| `0x00576790`, `0x005765A0`, `0x005DCA70` | `IsTrackUnlocked`: all open when `DAT_00926124` (unlock all) is set; the quick race bit (`param & 1`) calls `0x005765A0`, which tests bit 4 of the race's score flags (`FUN_005DCA70(table, hash, 4)`) and opens the event `19.8.31`; the career bit tests bit 8. |
| `0x00600B20` | The score flags set up for a new profile: for each race that is not a D-Day race, bit 4 (open in the quick race) is set when `FUN_005FBBD0` (attribute `0xB39ED8C3`, bit 0 of the race record) or `FUN_005FBCB0` (attribute `0xEA855EAF`, bit 2) holds (and the race is not disabled by `FUN_005FBD90`); bit `0x10` from `FUN_005FBC40` (attribute `0x39509746`). 28 races of bins 1 to 15 carry the first two in this install. |
| `0x007AB950` | A two-way widget (left and right) that toggles a byte of the front-end database (`+0x173`) and clears another (`+0x144`); not identified. |
| `0x007AD640` | A table of 13 Blacklist-position language hashes (`0xCE184740` ...), not read further. |

The widget classes were told apart by their order in these constructors: the same vtable (`0x008B78C8` Direction, `0x008B788C` Laps,
`0x008B7850` Traffic, `0x008B779C` Opponents, `0x008B77D8` Difficulty, `0x008B7814` Catch Up) appears in the same position in each
setup, and a script searching for the label immediates found them only in `0x007AC080..0x007AC580`. **No widget of the exe carries
the Heat Level label `0x251A34F8`** (the scan found no instruction with it); the row is in the reference screenshot of the circuit
options, so it is added by this rewrite.

## What the decomp gave

- `UIQRModeSelect::Setup`: the icons and labels per mode; `NotificationMessage`: Accept (`__BUTTON_PRESSED__`) switches to
  `Track_Select.fng` after the leave animation; Back returns to `MainMenu_Sub.fng`.
- `UIQRTrackSelect`: the title hashes per mode, the bin walk (`0x15` then 15 down to 1), `TryToAddTrack` (the region test
  `RegionFilterBits`, `UnlockSystem::IsTrackUnlocked` with `IsQuickRaceUnlocked`, the lock text with the rival alias), the
  header (`RefreshHeader`: counter objects `0x6F25A248`, `0xB2037BDC`, name strings `0x5E7B09C9`/`0x0DFB7A2E`, length
  `%$0.1f %s`, record string `0xB515499C`, record label `0x28462C64` with `Best Lap:` `0xC5B5A177` and `Best Speed:` `0x0512E823`,
  mode icon `0x08007B4C`), the region label hashes, L1/R1 for the regions.
- `UIQRTrackOptions`: the widgets (`NumLaps` 1 to 8, `NumOpponents` 1 to 4 minus the players, `AISkill` 0 to 2, `TrafficLevel` 0 to 3,
  `CatchUp`), `Defaults` as `__PAD_BUTTON5__` with the confirmation `0x8AEF5AE8`, Accept to `Car_Select.fng`, Back to
  `Track_Select.fng`.
- `cFrontendDatabase::DefaultRaceSettings` and `RaceSettings::Default` (three opponents, `AISkill` 1, `TrafficDensity` 1, catch up
  on, cops off); `FillCustomRace` (traffic 0, 10, 30, 50; difficulty and laps; cops only when `CopsOn`).
- `UIQRCarSelect`: the lists and their header hashes, `RefreshCarList`, `SetupForPlayer`, the messages (`0x2E76EDFB` leave on Accept,
  `0x93E8A57C` leave on Back, `0x89D0649C` showcase, `0xDE511657` quick race setup), `ChooseTransmission` (labels
  `0x6F5401D1`, `0x317D3005`, `0x8CD532A0`), the drag race starting without the question (not copied: this build asks), the bonus unlock sentences.
- `QuickRaceUnlocker::IsCarUnlocked` (the starter car types, `UnlockedAt() >= currentBin`, the bonus cars by name),
  `UnlockSystem::IsEventAvailable` (the events no quick race lists).
- `Showcase`: the two logo textures, `Accept`/`Back` back to the car select, the rival block only for a Blacklist showcase.
- `FEPlayerCarDB::Default`: which preset rides are bonus cars (`BL2`..`BL15`, `BONUS_*`, `CASTROLGT`, `E3_DEMO_BMW`), `CE_*` the
  collector's; the career-start M3 GTR is the preset list's one car.

## Dead ends

- `GRaceDatabase::IsQuickRaceUnlocked` is an empty file in the decomp; in the exe it is the flag test above, and the flag is set
  for a fresh profile by `0x00600B20`. The setter that opens the rest when a rival is beaten was not found (a scan for `OR [r+4], 4`
  found `0x00600B20` and unrelated code only). The rule in the spec (a track opens with its bin's rival) comes from the lock
  sentence and from the cars' arithmetic.
- The tutorial button of the mode select: the exe's table of modes with a tutorial was not found; the three movies exist.
- The car select's transmission question in the exe: a scan for the immediates of the labels `0x6F5401D1` (the question), `0x317D3005`
  and `0x8CD532A0` found only the options row of the Transmission setting (`0x0051C5F0`, which shows Manual or Auto from the player
  byte at `+0x91`); the dialog of the car select takes its labels from a table, so whether a drag race skips it was not read in the
  exe. The decomp says it does; this build asks anyway.
- `TransmissionPromptOn` (the player option that skips the question) has no counterpart in this build's settings.
- The exe's Heat Level row (see above).
