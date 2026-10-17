# Career rap sheet: entry, screens and the numbers on them

What the rap sheet screens of the original show, where every figure comes from, how the screens lead into each other and
what the unlock gate is. This file covers the entry, the shared rules, the login pair, the main page, the summary, the
infractions, the vehicle database, the cost to state and the top 5 pursuits with their detail. The rankings and their
rules, how the history is recorded, the end-game variants, the main menu profile card, the reputation sheets (race
events, milestones, bounty), the FEng object tables and the Rust notes are in
[career-rapsheet-2.md](career-rapsheet-2.md). Addresses, Ghidra names and dead ends are in the
[provenance file](../provenance/career-rapsheet.md).

- **Sources:** `speed.exe` (Black Edition PC v1.3, image base `0x00400000`) read through the Ghidra MCP and by
  disassembling undefined routines; the install at `D:\Need For Speed Most Wanted Black Edition` (`GLOBAL/FE_ATTRIB.bin`
  through `libs/blackbox-attrib`, `LANGUAGES/English.bin`, the packages of `FRONTEND/FrontB.lzc` through
  `nfsmw dump-screen`). Nothing was run in game.
- **Evidence tags:** **[exe]** read in `speed.exe`, not tested in game. **[verified]** checked against install data
  (attribute values, label texts, package objects). **[unconfirmed]** a reading that was not closed.
- **Addresses** are `speed.exe` addresses; no Ghidra auto names appear here. Label names (`RAP_*`) are the language
  table keys of [career-text.md](career-text.md); FEng object names are hashes, written with the string they hash to
  (the engine hash of [../formats/models.md](../formats/models.md), upper-case names) where that string is known.

## 0. Direct answers

| Question | Answer |
|---|---|
| What unlocks the rap sheet? | Settings flag bit 4 (`0x10`, profile `+0xAC`). The safe house menu only creates the "Rap Sheet" item when it is set; script hook `SetHasRapSheet` (bin 13 start) and the bin 13 safe house step set it. Section 1.1. **[exe]** |
| Which page prints which number? | Main page and Summary page share eight totals; the other pages are lists. Sections 3 to 8. **[exe]** |
| Printed index of `RAP_SUMMARY_nn` | `nn` = first position `i` (from 0) of the array `[0, 100, 400, 800, 1000]` with `total <= array[i]`, or 5 when none; `0` becomes `1`. So 0 to 100 infractions give `01`, 101 to 400 `02`, 401 to 800 `03`, 801 to 1000 `04`, more `05`. `total` is served plus unserved infractions of the whole stable. Section 4. **[exe + verified]** |
| What are the 8 extra pursuit sums? | The pieces of the cost to state that have a line on the cost page: roadblocks deployed, traffic cars hit, spike strips deployed, helicopter spike strips (no line), patrol cars deployed, support vehicles deployed, property damage value and property damage count. File 2, section 2. **[exe]**, slot meaning **[unconfirmed]** |
| How is a rank computed? | 1-based position of the first of 15 thresholds that is at most the value; 16 when none. Career value = raw sum, single pursuit = best value. File 2, section 1. **[exe + verified]** |
| What is the "timer" array slot? | The row type of the ranking detail page when the category is pursuit length: it prints the value as a time. Every other category uses the plain number slot. File 2, section 1.4. **[exe]** |
| Units of pursuit length | Ticks of 1/4000 s everywhere (sum, best pair, top 5, detail). The `0.00025` factor of the earlier notes converts ticks to seconds. File 2, section 1.3. **[exe]** |
| How does a milestone start from the rap sheet? | Confirm dialog, then the front end stores a pending jump (spawn key at manager `+0x11C`, flag bytes `+0x111`, `+0x114`) and leaves to free roam. File 2, section 5. **[exe]** |

## 1. Entry, flow and shared rules

### 1.1 The gate and the entry

The safe house menu (`MainMenu_Sub.fng`, builder `0x00532220`) adds the item "Rap Sheet" (`MAIN_RAP_SHEET`) only when the
settings flag word has bit 4. Selecting it (`BUTTON_PRESSED` on the item, handler `0x005320E0`):

1. sets front-end context bit 9 (`0x200`, `profile db + 0x12C`);
2. pushes `RapSheetLogin.fng` with all three request arguments 0.

Context bit 9 is "inside the rap sheet". The only reader found is the front-end audio state chooser (`0x004DF6D0`),
which uses state 13 instead of 9 to 12 while the bit is set (the rap sheet music). It is cleared whenever the player
leaves the pages to the safe house menu (sections 2 and 3). **[exe]**

The flag is set by the script hook `SetHasRapSheet` (`0x00605670`: `flags |= 0x10`) at the start of bin 13, and by the
bin 13 step of the Blacklist sequence ([career-safehouse.md](career-safehouse.md) section 5.3). The screens never test it
again once entered. **[exe]**

### 1.2 Screen graph

Every page is one FEng package; the page class decides where `EXIT_COMPLETE` (the end of the leave animation) goes.
`PAD_BACK` and `PAD_ACCEPT` are first handled by the package itself (it plays the leave animation); the class only acts
when the animation ends. **[exe]**

| From | Trigger | To |
|---|---|---|
| Safe house menu | item "Rap Sheet" | `RapSheetLogin` |
| `RapSheetLogin` | its animation ends (11.1 s), or Accept (skip) | `RapSheetLogin2`, or `RapSheetMain` after Accept |
| `RapSheetLogin2` | its animation ends (6.6 s), or Accept | `RapSheetMain` |
| `RapSheetLogin`, `RapSheetLogin2` | Back | `MainMenu_Sub`, context bit 9 cleared |
| `RapSheetMain` | button 1 to 6 | `RapSheetRS`, `RapSheetVD`, `RapSheetUS`, `RapSheetCTS`, `RapSheetTEP`, `RapSheetRankings` |
| `RapSheetMain` | Back | `MainMenu_Sub`, context bit 9 cleared, remembered button reset to 1 |
| `RapSheetRS`, `VD`, `US`, `CTS` | any leave | `RapSheetMain` |
| `RapSheetTEP` | accept on entry n (1 to 5) | `RapSheetPD` with argument n-1 |
| `RapSheetTEP`, `RapSheetPD` | Back | `RapSheetMain`, or `RapSheetTEP` from `PD` |
| `RapSheetRankings` | accept on category | `RapSheetRankingsDetail` with the statistic index as argument |
| `RapSheetRankingsDetail` | leave | `RapSheetRankings`, view mode copied back |
| `RapSheetRankings` | Back | `RapSheetMain` |

The login pair is a skippable boot animation: the first Accept in `Login` jumps straight to `Main`; only a full run of
`Login` shows `Login2`. They never gate anything (there is no password check). **[exe]**

### 1.3 Remembered selection

Each list page restores its last highlighted button. The store is a table of 50 entries `{package name hash, byte index,
front-end context word}` (`0x0091CFC0`, 12 bytes each), session only (not saved). A lookup matches an entry with the same
package hash whose context is `0xFFFFFFFF` or equals the current context word; a set writes the first free entry when
none matches. **[exe]**

| Page | What the byte holds | Written when |
|---|---|---|
| `Main` | button number 1 to 6 of the page last opened (default 1) | on every leave (Back writes 1) |
| `TEP` | entry number 1 to 5 last opened | on opening a detail, reset to 1 on Back |
| `Rankings` | statistic index 0 to 9 last opened | on opening a detail, reset to 0 on Back |

On `INIT_COMPLETE` a page highlights `BUTTON_<byte>` (0 is read as 1).

### 1.4 Shared rules

**Number formats.** `%$d` prints an integer with the locale's digit grouping (the design strings show `1.235.356`),
`%$.0f` the same for a float without decimals. All labels containing `%$d` take the value as the only argument. **[verified]**

**Header objects.** Every page sets `CASE_FILE` (object `0x1232703a`) to label `RAP_CASE_FILE` ("CASE FILE: %s") with
the 16-byte serial of the settings (profile `+0x314`). The player name uses `RAP_PLAYER_NAME` ("NAME: %s") with the
profile name. The text `ROCKPORT PD` next to it is a static label. **[exe]**

**Stable totals.** The car stable is `profile + 0x414` ([career-state.md](../formats/career-state.md) section 4).
A *live car* is one of the 200 car slots with a handle other than `0xFFFFFFFF`, `(flags & 0xF0000) != 0`,
`(flags & 2) != 0` and a career record index below 26 whose record byte 0 is not `0xFF`. The totals are:

| Total | Value |
|---|---|
| bounty | sold-history bounty (stable `+0x8CA0`, u32) plus the bounty of every live car |
| times evaded / busted | live sums plus the u16 at `+0x8CA4` / `+0x8CA6` |
| infractions of kind k, unserved / served | sold-history u16 (`+0x8CA8` / `+0x8CB8`, index k) plus live sums |
| all infractions, unserved / served | sums of the eight kinds the same way (the sold part wraps at 16 bits) |
| fines due / fines paid | sum over live cars of `fine(unserved counters)` / `fine(served counters)`; **no sold-history part** |
| cars impounded | live cars whose career record state (`+4`) is not 0 (states 2, 3 and 4 all count) |
| cars monitored | number of live cars |

`fine(c[0..7])` is 0 when all eight u16 counters are 0, otherwise the sum of `c[i] * amount[i]` in 32-bit integers,
with the amounts of the `infractions` collection read by name: index 0 `speeding`, 1 `racing`, 2 `reckless_driving`,
3 `assault`, 4 `hit_and_run`, 5 `damage`, 6 `resisting_arrest`, 7 `off_road`
([career-economy.md](career-economy.md) 1.4: 150, 350, 1000, 350, 300, 100, 300, 75). Served fines are the same
formula applied to the served counters at today's amounts, not the money actually paid. **[exe]**

**Statistics accessor.** Index 0 to 7 returns the raw sum `profile + 0x9C24 + 4*i` (non-busted pursuits only); 8 returns
all infractions (served plus unserved, stable totals above); 9 returns the stable bounty. **[exe]**

**Build time.** All pages fill their text once, in the constructor (and on view toggles); nothing refreshes while a
page is open. The history cannot change while the pages are up, so this is not observable. **[exe]**

## 2. Login pair

Both packages use the same class. Its mode (`+0x2C`) comes from the request: 0 for `Login`, 2 for `Login2`; `+0x30`
is the Back flag. `Login2` writes the profile name into the list entry `0x03CC94D6` with the format `> %s`. **[exe]**

Input handling:

| Message | Effect |
|---|---|
| `PAD_ACCEPT`, `MOUSE_LEFT_RELEASED` | mode 0: play UI sound `0x67`; mode 2: play sound `0x68`; then mode becomes 3 |
| `PAD_BACK` | Back flag = 1 |
| `EXIT_COMPLETE` | Back flag set: push `MainMenu_Sub`, clear context bit 9. Else mode 2 or 3: push `RapSheetMain`. Else (mode 0, animation ran out): push `RapSheetLogin2` with argument 2 |

The package locks its input at the start and sends its own `INPUT` event at 1.2 s (both pages); whether Accept before
that moment reaches the class is **[unconfirmed]**. After Accept the package starts its 0.6 s leave animation. **[exe]**

Animation timelines (milliseconds from the start, derived from the FEng scripts by following the message chains; the
model matches the package scripts but was not run in game **[exe]**):

| Time | `Login` | `Login2` |
|---|---|---|
| 0 | start | start; "SEARCHING DATABASE" |
| 0.3 / 0.6 / 1.0 | | "--/" spinner, "RETRIEVING LIST" line, spinner 2 |
| 1.2 | input enabled | input enabled |
| 1.6 | "CONNECTING TO ROCKPORT POLICE DEPARTMENT..." fades in (2.4 s) | first name row appears (then one row every 0.2 s: Allan, Callahan, Domingo, McCrea, the player, Sato, Vega) |
| 3.0 | | highlight bar starts (1.6 s script) |
| 4.0 | shield 4.0, title 4.22 | the player's row is highlighted (message at 1.0 s of the bar script) |
| 4.4 / 4.8 | frame lines 4.52 to 5.16 | "SELECTION MADE, RETRIEVING PROFILE" 4.4, "LOADING 8K" 4.8 (1.2 s) |
| 5.28 / 5.88 | "SIGN IN:" 5.28, "PASSWORD:" 5.88 | |
| 6.48 to 7.58 | five letters and seven X characters, 0.1 s apart | |
| 7.68 / 8.08 | "SIGNING IN" 7.68, spinner 8.08 (2.4 s) | |
| leave | 10.48 | 5.98 to 6.0 |
| `EXIT_COMPLETE` | 11.08 | 6.6 |

The boss surnames in the list are labels `RAP_LOGIN2_*` (Allan Wes, Callahan Clarence, Domingo Hector, McCrea
Ronald, Sato Toru, Vega Joe); they are decoration. **[verified]**

## 3. Main page (`RapSheetMain`)

Layout: six buttons `BUTTON_1` to `BUTTON_6` (`0xCDA0A66B` to `0xCDA0A670`) under the header "MAIN MENU:", and a block of
eight figures. The figures (filled once by `0x0052CA00`) are:

| Object | Label | Value |
|---|---|---|
| `0x1232703A` | `RAP_CASE_FILE` | serial |
| `0xE3DA78E7` | `RAP_PLAYER_NAME` | name |
| `0xE3DA78E8` | `RAP_TOTAL_BOUNTY` "BOUNTY: %$d" | stable bounty |
| `0xE3DA78E9` | `RAP_TOTAL_COST_STATE` | statistic 7 (cost to state) |
| `0xE3DA78EA` | `RAP_TOTAL_CARS_IMPOUNDED` | cars impounded |
| `0xE3DA78EB` | `RAP_TOTAL_FINES_DUE` | fines due (unserved) |
| `0xE3DA78EC` | `RAP_TOTAL_EVADED_PURSUITS` | times evaded |
| `0xE3DA78ED` | `RAP_TOTAL_BUSTED_PURSUITS` | times busted |
| `0xE3DA78EE` | `RAP_CARS_MONITORED` | cars monitored |

Buttons, top to bottom, with the page they open (`EXIT_COMPLETE`, selected button hash + `0x325F5995` as jump index):

| Button | Label | Page | Remembered number |
|---|---|---|---|
| 1 | `RAP_MAIN_SUMMARY` | `RapSheetRS` | 1 |
| 2 | `RAP_MAIN_VD` | `RapSheetVD` | 2 |
| 3 | `RAP_MAIN_INFRACTIONS` | `RapSheetUS` | 3 |
| 4 | `RAP_MAIN_CTS` | `RapSheetCTS` | 4 |
| 5 | `RAP_MAIN_TEP` | `RapSheetTEP` | 5 |
| 6 | `RAP_MAIN_RANKINGS` | `RapSheetRankings` | 6 |

A selected-hash outside the range (Back, or a leave without a button press) goes to `MainMenu_Sub` as in section 1.2.
The "pressed" hash is stored on `BUTTON_PRESSED` and on the accept-click message `0x08B84A5C`; the highlight hash on
`BUTTON_HIGHLIGHT`. **[exe]**

## 4. Summary page (`RapSheetRS`)

All values are static after the constructor (`0x0052CBC0`). The page has no input besides the leave (to Main). **[exe]**

| Object | Label | Value |
|---|---|---|
| `0x1232703A` | `RAP_CASE_FILE` | serial |
| `0xB259EEA7` | `RAP_PLAYER_NAME` | name |
| `0xB259EEA8` | `RAP_TOTAL_BOUNTY` | stable bounty |
| `0xB259EEA9` | `RAP_TOTAL_COST_STATE` | statistic 7 |
| `0xB259EEAA` | `RAP_TOTAL_CARS_IMPOUNDED` | cars impounded |
| `0x6EB5AC50` / `0x6EB5AC51` | `RAP_RS_UNSERVED_INFRACTIONS` / `RAP_RS_SERVED_INFRACTIONS` | all unserved / served infractions |
| `0xD919049F` / `0xD91904A0` | `RAP_RS_TIMES_EVADED` / `RAP_RS_TIMES_BUSTED` | times evaded / busted |
| `0xD01E18C5` / `0xD01E18C6` | `RAP_RS_FINES_DUE` / `RAP_RS_FINES_PAID` | fines due / fines paid |
| `0xD7E5D0CC` / `0xD7E5D0CD` | `RAP_RS_POLICE_DAMAGED` / `RAP_RS_POLICE_DESTROYED` ("IMMOBILIZED") | statistic 2 / statistic 3 |
| `0x90211462` | `RAP_SUMMARY_nn` | the police verdict, below |

Verdict text. Let `total` be all unserved plus all served infractions (stable totals, section 1.4) and `T` the array of
the `frontend` collection `rapsheet_summary_strings` (field `0xB54D4A41`, values `[0, 100, 400, 800, 1000]` in the
install, so N = 5):

```
nn = N                       // none matched
for i in 0..N: if total <= T[i] { nn = i; break }
if nn == 0 { nn = 1 }
text = label "RAP_SUMMARY_%.2d" % nn          // 01 to 05
```

The five texts run from "minor code infractions: perform routine check" (01) to "wanton disregard of vehicle codes:
apprehend at all cost, request heavy backup" (05). The array and the texts are in the install. **[exe + verified]**

## 5. Infractions page (`RapSheetUS`)

An 8-row list, one row per infraction kind, in counter index order 0 to 7. A flag at `+0xE0` is the view (1 = unserved,
the start value; 0 = served); the key `0xC519BFC4` ("Switch View", icon `PC_key_1`) toggles it and refills everything.
Each row is `{label, shown count, total count}` where *shown* is the count in the chosen set and *total* is served plus
unserved of that kind (stable totals, sold history included). **[exe]**

| Row | Counter index | Kind (mask) | Row label | English |
|---|---|---|---|---|
| 1 | 0 | speeding (`0x01`) | `RAP_US_RACING` | SPEEDING |
| 2 | 1 | racing (`0x02`) | `RAP_US_EXCESSIVE_SPEEDING` | EXCESSIVE SPEEDING |
| 3 | 2 | reckless driving (`0x04`) | `RAP_US_RECKLESS_DRIVING` | RECKLESS DRIVING |
| 4 | 3 | assault (`0x08`) | `RAP_US_ASSULTING_POLICE_OFFICER` | RAMMING A POLICE VEHICLE |
| 5 | 4 | hit and run (`0x10`) | `RAP_US_HIT_AND_RUN` | HIT AND RUN |
| 6 | 5 | damage to property (`0x20`) | `RAP_US_DAMAGE_TO_PROPERTY` | DAMAGE TO PROPERTY |
| 7 | 6 | resisting arrest (`0x40`) | `RAP_US_RESISTING_ARREST` | RESISTING ARREST |
| 8 | 7 | off road (`0x80`) | `RAP_US_DRIVING_OFF_ROADWAY` | DRIVING OFF ROADWAY |

Rows 1 and 2 are swapped compared with the pursuit HUD and the post-pursuit page, which label mask `0x01` "Excessive
Speeding" and mask `0x02` "Speeding" ([career-text.md](career-text.md) 6.3). The rap sheet pairs the counter of mask
`0x01` (fine 150) with "SPEEDING" and the counter of mask `0x02` (fine 350) with "EXCESSIVE SPEEDING"; which of the two
naming schemes the designers meant is **[unconfirmed]**, the code is as listed. **[exe]**

Row objects (`n` = row 1 to 8): label `INFRACTION_TYPE_n` (`0x74543A5D` + n-1), shown count `UNSERVED_QTY_n`
(`0x79CF1D38` + n-1), total `TOTAL_QTY_n` (`0xC4147B50` + n-1). Column header `0x9D974DF3`: `RAP_US_UNSERVED` or
`RAP_US_SERVED`. Both counts use `%$d`. **[exe]**

Header (refilled on every toggle), with `view` = the flag:

| Object | Value |
|---|---|
| `0x1232703A`, `0x1FFFB988` | serial, name |
| `0x1FFFB989` | view 1: `RAP_UNSERVED_INFRACTIONS` ("UNSERVED INFRACTIONS: %d") with the unserved total; view 0: `RAP_SERVED_INFRACTIONS` with the served total |
| `0x1FFFB98A` | `RAP_TOTAL_BOUNTY`, stable bounty |
| `0x1FFFB98B` | view 1: `RAP_FINES_DUE` with fines due; view 0: `RAP_FINES_PAID` with fines paid |
| `0x2ECAFA80`, `0xBBE88932` | the toggle labels: view 1 shows "Served Infractions" (`RAP_SERVED_INFRACTIONS_TOGGLE`), view 0 "Unserved Infractions" (the view to switch to) |

## 6. Vehicle database (`RapSheetVD`)

A 2-row list (two cars visible, the list scrolls with up and down). One row per owned car, in car slot order 0 to 199
(not by name or bounty). A car is listed when its slot has a handle, `(flags & 0xF0002)` has both a category bit and
the owned bit, and it has a career record. **[exe]**

| Cell object (`n` = row) | Label | Value |
|---|---|---|
| `CAR_n` (`0x028C4F05`) | the car's name label | `CARNAME_<maker>_<model>` of the car type, label `ERROR_DEFAULT_STRING` when missing ([career-text.md](career-text.md) 5) |
| `BOUNTY_n` (`0x01AEBC30`) | `RAP_VD_BOUNTY` | the car's bounty (record `+0x10`), no sold part |
| `FINES_n` (`0xD01E18C4`) | `RAP_VD_OUTSTANDING_FINES` | `fine(unserved counters)` of the car |
| `UNSERVED_n` (`0x6E5C90BB`) | `RAP_VD_UNSERVED_INFRACTIONS` | sum of the eight unserved counters |
| `TO_DRIVE_n` (`0xA408468B`) | status | state (record `+4`) not 0: `RAP_VD_IMPOUNDED`; else busted count (`+0x16`) not 0: `RAP_VD_CONFIRMED_TO_DRIVE`; else `RAP_VD_SUSPECTED_TO_DRIVE` |
| `EVADED_n` (`0xB1E72F78`) | `RAP_VD_TIMES_EVADED` | record `+0x14` |
| `BUSTED_n` (`0xCB45D776`) | `RAP_VD_TIMES_BUSTED` | record `+0x16` |

Rows beyond the number of cars are cleared: `CAR_n` shows `RAP_VD_NO_CAR_ON_RECORD` ("< no record >"), the other cells
are empty. Player cars that were sold are not listed; their sums live in the sold history only. **[exe]**

## 7. Cost to state (`RapSheetCTS`)

A 9-row list, fixed order. Each row is `{category label, quantity, cost}`; header objects are the name (`0xE3DA78E7`),
the stable bounty (`0xE3DA78E8`) and `0x358672CC` = `RAP_TOTAL_COST_STATE` with statistic 7. Quantity cell
`TXT_QUANTITY_n` (`0xD2A18F8D`), category cell `TXT_CATEGORY_n` (`0x3683DFAC`), cost cell `TXT_CASH_n` (`0x9068C46D`),
all `%$d`. The quantity and the unit price come from the history block (`profile + 0x90E0`, extras at
`profile + 0x9C94`); the extra index `e` is the position in the list of eight extras of file 2 section 2. **[exe]**

| Row | Category label (English) | Quantity | Cost |
|---|---|---|---|
| 1 | `RAP_CTS_DAMAGE_TO_PROPERTY` | extra 7 (property damage count) | extra 6 (property damage value, stored as cash) |
| 2 | `RAP_CTS_INSURANCE_CLAIMS` | extra 1 (traffic cars hit) | quantity x 500 |
| 3 | `RAP_CTS_OFFICERS_INVOLVED` (PATROL VEHICLES DEPLOYED) | extra 4 (patrol cars deployed) | x 250 |
| 4 | `RAP_CTS_SUPPORT_VEHICLES_INVOLVED` (SPECIAL UNITS DEPLOYED) | extra 5 (support vehicles) | x 450 |
| 5 | `RAP_CTS_DAMAGED_POLICE_VEHICLES` | statistic 2 | x 250 |
| 6 | `RAP_CTS_DESTROYED_POLICE_VEHICLES` (IMMOBILIZED) | statistic 3 | x 5000 |
| 7 | `RAP_CTS_ROADBLOCKS_DEPLOYED` | extra 0 (roadblocks deployed) | x 500 |
| 8 | `RAP_CTS_SPIKE_STRIPS_DELOYED` | extra 2 (spike strips) | x 250 |
| 9 | `RAP_CTS_HELICOPTERS_INVOLVED` | statistic 6 | x 2000 |

The lines are lifetime sums over non-busted pursuits. The page does **not** add them up: the "total cost to state" in the
header is statistic 7, the sum of the per-pursuit totals the pursuit code computed, which also contains the helicopter
spike strip term (extra 3, x 225, no line on the page) and may differ from the sum of the nine cost cells. The labels for
salaries, fuel and speed traps in the language table (`RAP_CTS_*`) are not used by any page. **[exe]**

## 8. Top 5 pursuits and detail

### 8.1 `RapSheetTEP`

Five fixed entries from the top-5 table (`profile + 0x9B0C`, 0x38 bytes each, sorted by pursuit bounty, best first;
the table and its insertion rule are in file 2 section 2). Entry `i` (1 to 5) is "used" when its duration field
(`+0x14`) is not 0. **[exe]**

| Cell (`i` = entry) | Used | Empty |
|---|---|---|
| `CAR_USED_i` (`0x273E2675`) | `RAP_TEP_CAR_USED` ("CAR USED: %s") with the car name of the entry's car key (`+0x0C`) | `RAP_TEP_EMPTY_INFRACTION` ("< CLEAN RECORD >") |
| `BOUNTY_i` (`0x01AEBC30`) | `RAP_TEP_BOUNTY` with the entry bounty (`+0x10`) | empty |
| `PURSUIT_ID_i` (`0x843A1717`) | `RAP_TEP_PURSUIT_ID` ("ID: %s") with the case id text (`+0x00`, `MW-` plus 8 characters) | empty |
| `PURSUIT_LENGTH_i` (`0x9EE33CEC`) | `RAP_TEP_PURSUIT_LENGTH` ("LENGTH: %s") with the duration as a time | empty |

Time text: the engine time formatter of file 2 section 1.3 (ticks of 1/4000 s; `HH:MM:SS` from one hour, else
`MM:SS.cc`, else `SS.cc`; some languages use another separator). An empty entry disables `BUTTON_i` (object flag
`0x4000000`). With no used entry at all, the groups `0xEB5E7757` and `0x73DCB662` (the accept prompts) are hidden.
Header: name `0xE3DA78E7`, `RAP_TOTAL_BOUNTY` `0xE3DA78E8`, serial. **[exe]**

Input: up/down wrap among the used entries only (up on `BUTTON_1` goes to the last used one, down on the last used one
to `BUTTON_1`). Accept starts the leave animation; at `EXIT_COMPLETE` the page opens `RapSheetPD` with the argument
`button number - 1` and remembers that number. Initial highlight: the remembered number, default 1. **[exe]**

### 8.2 `RapSheetPD` (pursuit detail)

Argument `e` (0 to 4) selects the entry at `profile + 0x9B0C + e*0x38`. The page is filled once (`0x0052E450`); leave
returns to `RapSheetTEP`. **[exe]**

| Object | Label (static in the package) | Value |
|---|---|---|
| `0x1232703A`, `0xEB406FEC` | | serial, name |
| `0xD91934E7` | `RAP_PURSUIT_ID` | case id text |
| `0x9068C46D` (`TXT_CASH_1`) | PURSUIT LENGTH | entry `+0x14` as a time |
| `0x9068C46E` | COPS INVOLVED | entry `+0x18` |
| `0x9068C46F` | COPS DAMAGED | `+0x1C` |
| `0x9068C470` | COPS DESTROYED | `+0x20` |
| `0x9068C471` | ROADBLOCKS DODGED | `+0x24` |
| `0x9068C472` | SPIKE STRIPS DODGED | `+0x28` |
| `0x9068C473` | COST TO STATE | `+0x2C` |
| `0x9068C474` | INFRACTIONS RECORDED | `+0x30` (number of distinct kinds) |
| `0x9068C475` | HELICOPTERS INVOLVED | `+0x34` |
| `0x9D81523D` | BOUNTY ACCUMULATED | `+0x10` |

Counts use `%$d`. The car of the pursuit (`+0x0C`) is not shown on this page. **[exe]**
