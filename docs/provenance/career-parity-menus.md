# Provenance: the career parity menus (start-up alias list, alias manager, Quick Race menu, pause menu, career menu)

Addresses, Ghidra names, method and dead ends for [career-parity-menus.md](../specs/career-parity-menus.md). Ghidra auto
names (`FUN_*`, `DAT_*`) are used here only. Image base `0x00400000`; program `/speed.exe` (Black Edition PC v1.3), read
through the Ghidra MCP bridge (project `nfsmw`). Earlier passes that this one builds on:
[career-save-screens.md](career-save-screens.md), [career-rapsheet.md](career-rapsheet.md),
[career-flow.md](career-flow.md).

## Method

1. Language labels were found with `nfsmw strings` (English table). A label is a 32-bit key; the exe pushes it as an
   immediate, so `search_instructions` with the label as the operand lists the functions that use it. That found the
   menu builders below without any function names.
2. Texture keys were found by hashing the texture names stored uncompressed in `FRONTEND/FrontB.lzc` with
   `h = 0xFFFFFFFF; h = h * 33 + c` (the names are upper case in the file; no case folding). Each hash was compared with the
   first argument the builders pass. All ten icons below matched a name.
3. Menu items are built as `FUN_00586fa0(item, texture, label, 0)` (a `0x4C`-byte object, vtable per kind) and added with
   `FUN_00573960(menu, item)`. The decompile shows the textures as signed numbers (`-0x19cec699` is `0xE6313967`).
4. The decompile of a variadic call drops its arguments (`FUN_005252d0`, `FUN_004647c0`); the disassembly was read where the
   values mattered (the main menu card).
5. No run-time tool was used: every claim is a static reading. The Cheat Engine bridge was not needed.

## Menus and their builders

| Menu | Ghidra name | Address | What it does |
|---|---|---|---|
| Pause menu (every mode) | `FUN_0052b4d0` | `0x0052B4D0` | builds the pause items from the play mode `DAT_0091e000 + 0x1964` (0 quick race, 1 online, 2 career), the free-roam flag `+0x1960`, the race `+0x1968` and the stage byte `profile + 0xB0` (below 16 = the Blacklist has started). Caller `FUN_00546360` |
| Alias manager items | `FUN_00543300` | `0x00543300` | four items: Create, Load, Save, Delete, in that order (Save is built first and stored in `this[0x52]`, then added third) |
| Career main menu items | `FUN_00532890` | `0x00532890` | Resume Career / Start (New) Career / Load, title `career main menu` (`0x8FFF61F2`). Caller `FUN_00549300` |
| Quick Race menu items | `FUN_007aa730` | `0x007AA730` | Quick Play and Custom Race; title `quick race` (`0xCDF0CC03`, set by `FUN_007a2de0` on both `TITLE_GROUP` strings `0x03C458C1` and `0xB5C74226`). Caller `FUN_007ba3f0` |
| Quick Race car list | `FUN_007ada50` | `0x007ADA50` | fills the car card (logo, name, stat bars) and the unlock sentence |
| Unlock sentence | `FUN_007a3c00` | `0x007A3C00` | maps a bonus car's collection key to a label (table below) |
| Main menu card | `FUN_00528bc0` | `0x00528BC0` | shows or hides the name and statistics groups; see [career-rapsheet-2.md](../specs/career-rapsheet-2.md) section 4 for the percentages |
| Completion percentages | `FUN_005a8780` | `0x005A8780` | the four card bytes and the challenge series pair |
| Play-mode flag helpers | `FUN_006e8df0`, `FUN_0056e2b0`, `FUN_0056dc00`, `FUN_0050e1b0`, `FUN_005fc180` | | `DAT_00988e04` (1 = pause variant with Quit to Quick Race), `DAT_0092612c` (1 = Quick Play hidden), "the race is event `1.8.1`" (the last race), "race kind is 4", "the race is a challenge-series race" (definition flags bit 15) |

## Pause menu items (`FUN_0052b4d0`)

Items are `(texture, label)`; `Resume Race` is the same picture as `Resume Free Roam` (`INGAME_ICON_RESUME_RACE`).

| Pause item | Texture key (name) | Label key (English) |
|---|---|---|
| Resume Race | `0x12BB5EA2` (`INGAME_ICON_RESUME_RACE`) | `0xDED357E7` "Resume Race" |
| Resume Free Roam | `0x12BB5EA2` | `0x01BD185C` "Resume Free Roam" |
| Restart | `0xB295A6B6` (`-0x4D6A594A`) | `0xF893AFA1` "Restart" (also `0x00DC0C9F` "Restart Race") |
| Quit to Quick Race Menu | `0x4C9E34E6` | `0x4349998B` "Quit to Quick Race Menu" |
| Quit to Career Main Menu | `0x4C9E34E6` | `0x690E9B7C` "Quit to Career Main Menu" |
| Quit to main menu | `0x4C9E34E6` | `0xE950B7AF` (the label the rewrite already uses for Quit to Main Menu) |
| Quit to Free Roam | `0x56FFBD2C` | `0x9DC599B0` "Quit to Free Roam" |
| Jump to Safe House | `0x4C9E34E6` | `0x3C14C420` "Jump to Safe House" (career, no race running) |
| Tuning | `0x483238FD` | `0x6A3672A2` (shown when no race is running and the Blacklist has started) |
| Options | `0x520DE4E3` | `0x2B5A03A8` |

The names `pm_ResumeRace`, `pm_RestartRace`, `pm_QuitRaceToFE`, `pm_QuitQuickRace`, `pm_QuitMainMenu`,
`pm_QuitRaceToFreeRoam`, `pm_ResumeFreeRoam`, `pm_SwitchToTuning` and `pm_SwitchToOptions` are in the exe's string table
(`0x0089C294`..`0x0089D32C`); they are only referenced from unwind records, so they name the item classes (the vtables
`0x0089C24C` resume, `0x0089C25C` restart, `0x0089C264` options, `0x0089C27C` quit to quick race, `0x0089C28C` quit).

The decompile gives these lists (the free-roam flag is `+0x1960`; "race" is the object at `+0x1968`):

| Mode | Situation | Items, in order |
|---|---|---|
| 0 quick race (`DAT_00988e04 != 1`) | a race is running | Resume Race, Restart, Quit to Quick Race Menu (a race that is not a challenge race) or Quit to main menu (a challenge race), Tuning (race is not kind 4 and not a challenge), Options |
| 0 quick race (`DAT_00988e04 == 1`) | | Resume Race, Restart, Quit to Quick Race Menu, Options |
| 2 career | free roam, stage below 16 | Resume Free Roam, Jump to Safe House, Tuning, Options (stage 16 shows Resume Free Roam, Quit to Career Main Menu, Tuning, Options) |
| 2 career | a race is running | Resume Race, Restart, Quit to Career Main Menu, Tuning, Options; an event started from free roam instead of the menus gets Quit to Free Roam too |

The confirmation questions are language labels: restart `0x4D3399A8` ("Are you sure you want to restart the race? All
race progress will be lost."), quit to the quick race menu `0x1DB1CDE5`, quit to the main menu `0xA2E9B449`, quit to free
roam `0x9887EB98`.

## Alias manager items (`FUN_00543300`)

| Item | Texture key (name) | Label key (English) |
|---|---|---|
| Create | `0x43798644` (`MAIN_ICON_PROFILE_ADD`) | `0x55423473` "Create" |
| Load | `0x2287E063` (`GENERIC_ICON_LOAD`) | `0x0018ECFF` "Load" |
| Save | `0x228B7E32` (`GENERIC_ICON_SAVE`) | `0x001C8ACE` "Save" |
| Delete | `0x0D9035CE` (`MAIN_ICON_PROFILE_DELETE`) | `0x56B00632` "Delete" |

The main menu's own entry for this screen is `MAIN_ICON_PROFILE_MANAG(ER)` `0x6B303856`, label `0xBCB18F38`.

## Career main menu items (`FUN_00532890`)

Profile flags word at `profile + 0xAC` (`flags`): bit 0 = career started, bit 11 = no resume (the game was won).

| Condition | Items |
|---|---|
| career not started | Start Career (`MAIN_ICON_NEWCAREER` `0xE7353BE7`, label `0x6005281E`), Load |
| started, bit 11 clear | Resume Career (`MAIN_ICON_RESUMECAREER` `0xC1C089CE`, label `0xE072DB21`), Start New Career (`0xE7353BE7`, `0x17E18F87`), Load (`GENERIC_ICON_LOAD`, `0x0018ECFF`) |
| started, bit 11 set | Start New Career, Load; the `PC BACK` button is shown |

The title strings `0x03C458C1` and `0xB5C74226` get label `0x8FFF61F2` ("career main menu"). With a profile loaded
(`DAT_0091cf90 + 3`) the screen hides the object `0xC87422F7`'s child `0x1CA7C0` and writes the alias name into the string
`0xEB406FEC` (the name box at the bottom left of the screenshot).

## Quick Race menu items (`FUN_007aa730`)

| Item | Texture key (name) | Label key (English) | Condition |
|---|---|---|---|
| Quick Play | `0xE6313967` (`MODE_ICON_QUICK_SELECT`) | `0xB5E8F82F` "Quick Play" | only while `DAT_0092612c == 0` |
| Custom Race | `0x2A49B5E2` (`MODE_ICON_QUICK_RACE`) | `0x25BBD4C3` "Custom Race" | always |

The Split Screen picture is `MODE_ICON_2PLAYER_SPLIT` (`0xF365B5F5`) and its label is `0x841D518A` "Split Screen". No
immediate pushes that label, so the exe builds it from another table **[unconfirmed]**; the rewrite lists it third.
The Quick Play help text is label `0x7C3E459E`: "This special Quick Race game mode gives you a random car, track and
race type. The selection is based on your Blacklist position and includes all cars and tracks for the two upcoming
Blacklist positions."

## The Quick Play brief (`Quick_Race_Brief.fng`)

Dumped with `nfsmw dump-screen`. Title `quick play brief` (`0xBA458180`), buttons Back (Esc, `0x61D596D7`), Accept (Enter,
`0x2B65A216`) and Randomize (key 1, label `0x4FB803C4`). The package answers a release on the Randomize button with the
game message `0xC519BFC4` (the same message as the Game Stats button and the rap sheet's Switch View); Accept answers
`0x406415E3`, Back `0x911AB364`. Objects the game writes:

| Object | GUID | Name hash | Content |
|---|---|---|---|
| mode icon | `0xF5E9E` | `0x2521E5EB` | `Mode_Icon_Sprint.tga` swapped for the race kind's icon (`MODE_ICON_CIRCUIT`, `_SPRINT`, `_DRAG`, `_LAP_KNOCKOUT`) |
| mode text | `0xF5E02`, `0xE9727` | `0xBEB9E79D`, `0x4FA22DFC` | "Circuit - K.O." (text and shadow) |
| track name | `0xE9728` | `0xB5154999` | "Sidewinder" |
| length / laps / opponents values | `0xE9729`, `0xE972A`, `0xE972B` | `0xB515499A`..`C` | "10KM", "5", "8" |
| length / laps / opponents labels | `0xE96E7`, `0xE96EC`, `0xE96F4` | `0x28462C62`..`64` | "Length:", "Laps:", "Opponents:" |
| difficulty, traffic, catch-up | `0xEDEA1`.., `0xE972C` | `0x28462C65`, `0xB515499D`.. | the right-hand column (hidden for Quick Play, which has no options) |
| car manufacturer logo | `0xEDDCD` | `0x3E01AD1D` | `CARSELECT_MANUFACTURER_*` texture |
| car model logo | `0xEDDDD` | `0xB05DD708` | `Secondary_Logo_*` texture (the model's name picture) |
| TOP SPEED / ACCELERATION / HANDLING bars | `0xF5C77`, `0xF5C7E`, `0xF5C85` | `0x4037F0E4`, `0x46BCC4AA`, `0xD108E905` | LED strips as in `Car_Select.fng` |
| unlock line | `0xF0F0D` | `0xA63265D0` | "Unlock this car by beating %s, Blacklist member %d." for a locked car |

The brief screen's car and track rows come from the same front-end collections as the car lot
([career-car-lot.md](../specs/career-car-lot.md)).

## Unlock sentences (`FUN_007a3c00`, `FUN_007ada50`)

A bonus car's front-end collection key selects the label:

| Key | Label | English |
|---|---|---|
| `0x2CF370F0` | `0xBD8BAC94` | Unlock this car by getting 100% game complete. |
| `0x03A94520` | `0xBD8BAC93` | Unlock this car by winning the career mode. |
| `0x2CF385B2` | `0xBD8BAC92` | Unlock this car by completing every event in the Challenge Series. |
| `0xCB6AAF2F` | `0xBD8BAC91` | Unlock this car by completing event number 68 in the Challenge Series. |
| `0x0013624E`..`0x00136253`, `0x0000965F`..`0x00009666` | `0x4EF2A115` | Unlock this car by beating %s, Blacklist member %d. (`%s` = the rival's alias `blacklist_rival_%02d_aka`, `%d` = the rank) |

Any other key gives no sentence (label 0).

## The main menu card (`FUN_00528bc0`)

With no profile loaded (`DAT_0091CF90 + 3 == 0`) the function hides `NAME_GROUP` (`0xFB80CDAC`) and `GAME STATS GROUP`
(`0x8913E195`). With one it writes `GAME_COMPLETED_DATA` (`0xAAEC91EE`), `CAREER_COMPLETED_DATA` (`0x6A5E95C6`),
`RAP_SHEET_ITEMS_DATA_2` (`0x4AA32965`), `CHALLENGE_SERIES_DATA_1` (`0xD3C84175`, `%d/%d` = `0x0089CE80`) and `_2`
(`0xD3C84176`), and the alias name into `0xB514E2D8` (the string `0xEB406FEC` inside the name group) with the percent
suffix `0x0089CEEC` (languages 1, 2, 6, 7 and 13, a blank before the sign) or `0x0089CEF0`. The first call of
`FUN_005A8780` fills `{game, career, rap, series}` as four bytes plus `{done, total}` for the series; the stack offsets in the
disassembly (`[ESP+0x24]`, `[ESP+0x25]`, `[ESP+0x16]`, `[ESP+0x27]`, `[ESP+0x28]`) read them in that order **[exe]**.

The Game Stats button (guid `0x1151C1`, name `0xD6463100`, key `1`) answers a release with `0xC519BFC4`; the package's
`GAME STATS GROUP` listens for the messages `GAMESTATS_APPEAR` (`0x2474A660`) and `GAMESTATS_LEAVE` (`0xDB11E414`), which are
the names found at `0x0089EE00` and `0x0089EE14` (code at `0x0055A029`..`0x0055A03F`, no function defined). The game
posts one or the other to toggle the card **[exe, reading]**.

## Start-up

`MC_Bootup`, then `MC_Main.fng` in state `0x20` ([career-save-screens-2.md](../specs/career-save-screens-2.md) section 9.1
and 9.2): no alias asks to create one, one alias loads at once, two or more show `MC_List.fng`. The notice "Load complete."
(library id 10) follows a successful load. The rewrite always shows the list when there is at least one alias, at the
user's request **[ours]**.

## Safe house menu items (`FUN_00532220`, `0x00532220`)

Built when the safe house front end opens (the caller is the screen setup). Items in order, `(texture, label)`: Resume Free
Roam `(0x12BB5EA2, 0x01BD185C)`; Blacklist `(0x2C14AC23` `MAIN_ICON_TOP_15`, `0x80B9FF9B)`; Car Select `(0xC6A1A6E0`
`MAIN_ICON_CAR_SELECT`, `0x00D5F627)`; Rap Sheet `(0x2FD8B206` `MAIN_ICON_RAP_SHEET`, `0xAC22F27E)` only when profile flag bit 4
is set; Save `(0x228B7E32` `GENERIC_ICON_SAVE`, `0x001C8ACE)` only when the memory-card layer flag `DAT_008F8708` is set. The title
strings get label `0xE596C4A3` ("safe house"). After the items it writes `CAREER_DATA` (`0x0089DA2C`, the career percentage
from `FUN_005A8780` byte 1, format `%d%s` with the percent suffix `0x0089CEE4`), `BOUNTY_DATA` (`0x0089DA20`, the stable's bounty,
`FUN_00590500`) and `CASH_DATA` (`0x0089DA14`, `profile + 0xB4`). Earlier passes used the Car Select (My Cars) picture and the
alias manager's disk for Car Select and Save and no picture for Blacklist and Rap Sheet; these are replaced **[exe]**.

## Dead ends

- `UIQRBrief` and the other class names (`0x008A0834`, `0x008A0864`, `0x008A07B0`) are only referenced from unwind records,
  so the brief's controller was not found by name. Its car rule is the label `0x7C3E459E` above.
- `FUN_0056E2B0` and `FUN_006E8DF0` are one-line getters of globals; what sets `DAT_0092612c` and `DAT_00988e04` was not
  followed (they are the online/LAN and the demo-style pause variants).

## Empty-name rejection correction (2026-10-27)

The user reported broken UI after submitting an empty career name. An installed-package regression retained the
Dialog screen but lost its drawn prompt after held Accept. The host now owns submission validation on NameEntry,
preventing the notice package from starting its dismiss animation before validation. Authored entrance/highlight
scripts remain. Empty input stays silent; Back and valid submission retain their semantic sound triggers. This is
an independently authored robustness policy; original invalid-name presentation has not been established.
