# Career rap sheet, part 2: rankings, history recording, end game, profile card, reputation sheets

Continues [career-rapsheet.md](career-rapsheet.md) (entry, shared rules, login, main, summary, infractions, vehicle
database, cost to state, top 5 and detail). Same sources, evidence tags and conventions; Ghidra names, addresses and
dead ends are in the [provenance file](../provenance/career-rapsheet.md).

## 1. Rankings

### 1.1 The category page (`RapSheetRankings`)

Ten category rows, each with a rank cell `RANK_n` (`0x7711109B` + n-1 for n = 1 to 9, `0x5933242B` for n = 10) and a
button `BUTTON_n` (`0xCDA0A66B` + n-1, and `0x81B573FB` for button 10). The row order, the statistic behind each row and
the category texts (static labels of the package) are fixed by the constructor (`0x005470E0`): **[exe]**

| Row / button | Statistic index | Category text |
|---|---|---|
| 1 | 0 | PURSUIT LENGTH |
| 2 | 1 | TOTAL COPS INVOLVED |
| 3 | 2 | COPS DAMAGED |
| 4 | 3 | COPS DESTROYED |
| 5 | 5 | ROADBLOCKS DODGED |
| 6 | 4 | SPIKE STRIPS DODGED |
| 7 | 7 | COST TO STATE |
| 8 | 8 | INFRACTIONS RECORDED |
| 9 | 6 | HELICOPTERS INVOLVED |
| 10 | 9 | BOUNTY ACCUMULATED |

Rank cell: `rank(stat, view)` (section 1.2); when it is 16 the cell shows `RAP_RANKINGS_UNRANKED` ("UNRANKED"), else
the number with `%$d`. Header objects: serial, name (`0xEB406FEC`), the view title `0x001E4FDA` and the two switch
view labels `0x0DD2F4FB`, `0x9AE9B5CD`.

View byte (global `0x0091CA34`, starts 0, lives for the session): 0 = **single pursuit** (title
`RAP_RANKINGS_SINGLE_PURSUIT_VIEW`, "SINGLE PURSUIT"), 1 = **lifetime** (`RAP_RANKINGS_CAREER_PURSUIT_VIEW`,
"LIFETIME PURSUIT"). The key `0xC519BFC4` toggles it and recomputes all ten cells; the switch label shows the view that
the key would switch to ("Lifetime" while in single view, "Single Pursuit" while in lifetime view). **[exe]**

Accept on a button opens `RapSheetRankingsDetail` with the statistic index as the request argument and copies the view
byte to the detail's own view byte (`0x0091CA35`), then remembers the index (section 1.3 of file 1). Without a pressed
button (Back) it opens `RapSheetMain`. **[exe]**

### 1.2 The rank rule

```
fn rank(stat, career: bool) -> 1..=16:
    table = frontend collection named  career ? NAME : NAME + "_in_pursuit"
            NAME = pursuit_length, cops_involved, cops_damaged, cops_destroyed, tire_spikes_dodged,
                   roadblocks_dodged, helis_involved, cost_to_state, total_infractions, bounty   (index order)
    thr = table.array(0xF9A7D5F7)            // 15 floats, descending
    if thr.len != 15: return 16
    v = career ? stat_total(stat)            // file 1 section 1.4, indices 0..9
               : best_value[stat]            // history best pair value, section 2
    for k in 0..15:
        t = (stat == 0) ? trunc(thr[k] * 4000.0 + 0.5) : trunc(thr[k])
        if v >= t: return k + 1
    return 16
```

All comparisons are signed 32-bit. The factor 4000 exists because the pursuit length is stored in ticks (1/4000 s)
and the table holds seconds. A fresh profile has all values 0, so every category is "UNRANKED" until a pursuit is
recorded. The career statistics 8 and 9 are the stable totals (file 1 section 1.4), not stored values. **[exe]**

### 1.3 Units and the time formatter

The pursuit duration slot returns **game ticks of 1/4000 s** (the same unit as the engine clock; the engine clock times
`0.00025` is seconds). The history stores `trunc(duration)` ticks in the sum, the best pair and the top-5 entries. The
detail page converts the player's pursuit length to seconds with `0.00025` and back with `4000` when formatting. **[exe]**

The time formatter takes ticks and a flag word. `total_s = ticks / 4000`, `h = total_s / 3600`, `m = (total_s % 3600) / 60`,
`s = total_s % 60`, `cc = ((ticks % 4000) * 1000 / 4000) / 10`:

| Case | Text (default languages) |
|---|---|
| `h != 0` | `H:MM:SS` (blank instead of a leading zero of the hour digit) |
| `m != 0` | `MM:SS.cc` (leading blank in the minutes) |
| `s != 0` | `SS.cc` |
| otherwise | `0.cc` |

Flag bit `0x10` removes the last three characters (`.cc`) in the last three cases; bit 0 prefixes `-`, bit 1 `+`. Some
languages use `'`, `"`, `.` or `,` as separators. Ticks 0 or `0x7FFFFFFF` print as zero. The top 5 and detail pages
pass flag 0; the ranking detail passes `0x10`. **[exe]**

### 1.4 The detail page (`RapSheetRankingsDetail`)

Argument `stat`. Ten visible rows (`RANK_n`, `PLAYER_NAME_n` `0xBD1C275C`, `VEHICLE_n` `0x2112BA8F`, `TIME_n`
`0x7C4B7FFE`, each + n-1) over a scrolling list. The title object `0x8224E17C` shows the statistic's label: `RAP_PD_`
`PURSUIT_LENGTH`, `COPS_INVOLVED`, `COPS_DAMANGED`, `COPS_DESTROYED`, `SPIKE_STRIPS_DODGED`, `ROADBLOCKS_DODGED`,
`HELICOPTERS_INVOLVED`, `COST_TO_STATE`, `INFRATIONS_RECORDED`, `BOUNTY_ACCUMULATED` for statistic 0 to 9. **[exe]**

Rows are built from the 15 thresholds of the table (section 1.2, same table name rule with the page's view byte):

```
r = rank(stat, career)                    // 1..16
n_rows = (r == 16) ? 16 : 15
t = 0                                     // threshold cursor
for i in 0..n_rows:
    if i == r - 1:  row = player row {rank r, name = player name, value = player value, vehicle}
    else:           row = {rank i+1, name = alias of rival R[t], value = thr[t], vehicle}; t += 1
        where R = table.array(0x2C3C7FEB) (int8) and the alias label is "BLACKLIST_RIVAL_%.2d_AKA" of R[t];
        vehicle = single view only: label "BLACKLIST_RIVAL_%.2d_CAR" of R[t]; lifetime view: empty
```

Hence the player row replaces the lowest threshold row when ranked (15 rows, the 15th threshold is not shown) and is
appended with rank 16 when unranked. The value shown for a threshold row is the table float (seconds for statistic 0);
for the player row it is the career sum or the best value, times `0.00025` for statistic 0 only. The player row's
vehicle (single view) is the car of the best pair, label `CARNAME_...` of the key at `profile + 0x9C44 + 8*stat`.
The rank cell is the number (`%$d`) or `--` (`RAP_RANKINGS_UNRANKED_RANK`) when 16. **[exe]**

Row types: statistic 0 uses the **timer** slot (the value is `value * 4000 + 0.5` ticks, formatted with flag `0x10`);
the other nine use the plain slot (value printed `%$.0f`). The player's name cell shows the profile name (`%s`).
After building, the list scrolls down `rank - 6` times (only when positive) so the player row lies in view, then the
object `POS<k>` (k = row index inside the window, 1 to 10) plays its highlight script. Up and down move the highlight
and repeat the `POS` script. The key `0xC519BFC4` toggles the detail's own view byte and rebuilds. On leave the detail's
view byte is copied back to the page's. **[exe]**

### 1.5 Table data **[verified]**

Thresholds (rank 1 first) of the lifetime tables are in [career-economy-flows.md](career-economy-flows.md) 5.3. The
single pursuit tables (`*_in_pursuit`, the `0xF9A7D5F7` array) are:

| Statistic | Single pursuit thresholds |
|---|---|
| pursuit_length (seconds) | 1800, 1680, 1560, 1200, 900, 600, 540, 480, 420, 360, 300, 240, 180, 120, 60 |
| cops_involved | 150, 125, 100, 75, 50, 30, 25, 20, 15, 12, 10, 8, 6, 4, 2 |
| cops_damaged | 50, 45, 40, 35, 30, 25, 20, 15, 12, 10, 8, 6, 4, 2, 1 |
| cops_destroyed | 50, 45, 40, 35, 30, 25, 20, 18, 15, 12, 10, 8, 6, 4, 2 |
| tire_spikes_dodged | 30, 25, 20, 18, 16, 14, 12, 10, 8, 6, 5, 4, 3, 2, 1 |
| roadblocks_dodged | 30, 26, 24, 22, 20, 18, 16, 14, 12, 10, 8, 6, 4, 2, 1 |
| helis_involved | 5, 4, 4, 4, 4, 3, 3, 3, 3, 2, 2, 2, 1, 1, 1 |
| cost_to_state | 500000, 450000, 400000, 350000, 300000, 250000, 200000, 150000, 125000, 100000, 75000, 50000, 25000, 15000, 10000 |
| total_infractions | 7, 7, 7, 7, 6, 6, 6, 5, 5, 5, 4, 4, 3, 3, 2 |
| bounty | 1500000, 1250000, 1000000, 750000, 500000, 300000, 200000, 150000, 100000, 75000, 50000, 25000, 10000, 5000, 1000 |

Rival alias arrays (`0x2C3C7FEB`, rank 1 first; the lifetime and single tables of a statistic use the same array):
pursuit_length, tire_spikes_dodged, roadblocks_dodged, helis_involved, lifetime bounty: 1 to 15 in order.
cops_involved: 11, 2, 3, 4, 15, 6, 7, 8, 9, 10, 1, 12, 13, 14, 5. cops_damaged and cops_destroyed: 14, 1, 3, 4, 5, 6,
7, 8, 9, 10, 11, 12, 13, 2, 15. cost_to_state: 9, 12, 3, 4, 5, 6, 7, 8, 1, 10, 11, 2, 13, 14, 15. total_infractions: 4,
2, 5, 1, 3, 6, 7, ... 15. bounty_in_pursuit: 1, 5, 2, 3, 4, 6, 7, 8, 9, 10, 11, 12, 13, 15, 14. The label index is the
rival number of [career-text.md](career-text.md) section 2 (alias texts `BLACKLIST_RIVAL_nn_AKA`). **[verified]**

## 2. History recording (pursuit history, `profile + 0x90E0`)

Called once for every finished player pursuit in career mode, from the pursuit-end recorder
([career-economy-flows.md](career-economy-flows.md) section 2) with: the pursuit interface, the key of the player's car
type, the pursuit bounty (normal plus destruction rep points), and the number of distinct infraction kinds. Offsets
are from `profile + 0x90E0`. **[exe]**

1. `+0xBD4` (`profile + 0x9CB4`) = car key. Written for every pursuit, busted or not.
2. If the pursuit was busted (interface slot `+0x74` non-zero) nothing else changes. Otherwise:
3. Extra sums (below) and statistic sums: `+0xB44 += ticks`, `+0xB48 += cops involved`, `+0xB4C += damaged`, `+0xB50 +=
   destroyed`, `+0xB54 += spikes dodged`, `+0xB58 += roadblocks dodged`, `+0xB5C += helicopters`, `+0xB60 += cost to
   state`. These are statistics 0 to 7 at `profile + 0x9C24`.
4. Best pairs `{key, value}` at `+0xB64 + 8*stat`: a pair is replaced (key = this pursuit's car key) when the new
   value is **strictly greater**. Values: 0 duration ticks, 1 cops involved, 2 damaged, 3 destroyed, 4 spikes dodged,
   5 roadblocks dodged, 6 helicopters, 7 cost to state, 8 distinct infraction kinds, 9 bounty.
5. Top 5 at `+0xA2C` (0x38 bytes each, best first): find the first entry whose bounty (`+0x10`) is **strictly less** than
   the new bounty. If none of the five qualifies the pursuit is not stored. Otherwise entries from that position to the
   fourth shift down one place (the fifth is lost), the new entry is written there with a fresh case id and the fields
   of file 1 section 8.2 (`+0x0C` car key, `+0x10` bounty, `+0x14` ticks, `+0x18` cops involved, `+0x1C` damaged,
   `+0x20` destroyed, `+0x24` roadblocks dodged, `+0x28` spikes dodged, `+0x2C` cost, `+0x30` distinct kinds,
   `+0x34` helicopters). A bounty of 0 never enters.
6. The case id is `MW-` plus eight characters generated randomly (alternating digit and capital letter, see
   [career-economy-flows.md](career-economy-flows.md) 5.2); a new id is made for every stored pursuit.

The eight extra sums (they sit at `profile + 0x9C94` as `u32[8]`, the only pursuit-time values the cost page needs):

| e | Profile offset | History offset | Pursuit interface slot | Meaning (unit price on the cost page) |
|---|---|---|---|---|
| 0 | `0x9C94` | `+0xBB4` | `+0xA8` | roadblocks deployed (500) |
| 1 | `0x9C98` | `+0xBB8` | `+0xB4` | traffic cars hit, "insurance claims" (500) |
| 2 | `0x9C9C` | `+0xBBC` | `+0xBC` | spike strips deployed (250) |
| 3 | `0x9CA0` | `+0xBC0` | `+0xC0` | helicopter spike strips (225, no cost page line) |
| 4 | `0x9CA4` | `+0xBC4` | `+0xC4` | patrol cars deployed (250) |
| 5 | `0x9CA8` | `+0xBC8` | `+0xC8` | support vehicles deployed (450) |
| 6 | `0x9CAC` | `+0xBCC` | `+0xAC` | property damage value in cash |
| 7 | `0x9CB0` | `+0xBD0` | `+0xB0` | property damage count |

The meaning of a slot is read from the cost page label that prints it and from the unit prices, which are exactly the
terms of the pursuit total in [ai-pursuit-heat.md](ai-pursuit-heat.md) 5.4 (the 500 shared by roadblocks and traffic cars
is told apart by the label); the pursuit-side producers of the slots were not followed. **[exe]**, slot meaning
**[unconfirmed]**. The statistic slots (`+0x18` duration, `+0xA0`, `+0x94`, `+0x98`, `+0xB8`, `+0xA4`, `+0x10`, `+0xE0`)
are those of [career-economy-flows.md](career-economy-flows.md) 5.1.

## 3. End-game variants

When the final event is won the game sets context bit 18 (`0x40000`), resets a step counter (global `0x0091CA54`) to 0
and requests `GameWon.fng`. All of `GameWon`, `RapSheetLogin_ENDGAME`, `RapSheetLogin2_ENDGAME` and `RapSheetMain_ENDGAME`
use one tiny class (`0x00532B90`) whose only behaviour is: on `EXIT_COMPLETE`, run the step routine (`0x00532DD0`). The
credits screen (`Credits.fng`, roll text `RollCredit_ENDGAME` while bit 18 is set) calls the same routine when it ends.
**[exe]**

```
step():                       // counter is read, then incremented
    switch counter_before:
        0 -> push Credits.fng
        1 -> push RapSheetLogin_ENDGAME.fng
        2 -> push RapSheetLogin2_ENDGAME.fng
        3 -> push RapSheetMain_ENDGAME.fng
        4 -> push MainMenu_Sub.fng
    counter += 1
```

(The credits end through the same hook only while bit 18 is set; without it they return to `MainMenu_Sub`. Where bit 18
is cleared again was not found.) Differences to the normal pages:

| Page | Difference |
|---|---|
| `Login_ENDGAME` | same animation and timeline as `Login` (leave at 10.48 s, done at 11.08 s) with one extra image; no Back handling in the class: Accept or Back in the package only skip ahead |
| `Login2_ENDGAME` | same as `Login2` (leave 5.98 s, done 6.58 s). The class writes the profile name into the list entry `0x03CC94D6` as `> %s` when the counter is 3 |
| `Main_ENDGAME` | the main page without buttons (the menu entries are shown as plain text) plus the warrant sequence. When the counter is 4 the class fills the figures exactly as file 1 section 3 and also puts `RAP_PLAYER_NAME` into the warrant line `0x22F33E0A`. The warrant lines are static labels (`EG_*`): "ISSUES POLICE WARRANT", name, "CURRENT VEHICLE: BLUE BMW M3 GTR", "LAST SEEN: NORTH OUT OF ROCKPORT", "ISSUE LEVEL:" with `> CITYWIDE`, `> STATE`, `> NATIONAL` appearing in turn (scripts of 6.4 s, 3.2 s, 5.2 s), "DETAILS:" and the button "SUBMIT" |

The vehicle and last-seen lines are fixed text, not the player's car. How `Main_ENDGAME` ends (a timeline event or
Accept on "SUBMIT") was not followed to the end **[unconfirmed]**; the class advances on any `EXIT_COMPLETE`.

## 4. Profile card and the completion percentages

The safe house menu and the main menu show a small card computed by `0x005A8780` (called by the safe house builder
`0x00532220` and by the main menu fill `0x00528BC0`). It produces four bytes `{game, career, rap, series}` and a pair
`{series_total, series_done}`: **[exe]**

```
ms_done  = number of milestones with state 4                      // manager milestone table
ms_total = manager milestone count (+0xA4)                        // 55 in the install
ev_done = ev_total = 0
for bin in 1..=16:  for each event key in list A and list B of the bin view:   // counts from attributes
    ev_total += 1;  if race record flags & 2 (completed in career): ev_done += 1
career = trunc(100 * (ms_done + ev_done) / (ms_total + ev_total))
rap    = trunc(100 * sum over stat 0..9 of max(0, 15 - rank(stat, career=true)) / 140)    // rank 16 adds nothing
series: for every race record that is not named "19.8.31", is flagged challenge series (definition flags bit 15)
        and (the debug-event byte is clear or the developer flag 0x008F8718 is set):
            series_total += 1;  if record flags & 1 (completed outside career): series_done += 1
series = trunc(100 * series_done / series_total)
game   = trunc(0.2 * series + 0.1 * rap + 0.7 * career)           // uses the truncated bytes
```

The safe house menu prints `CAREER_DATA` = `career` with `%d%s` (`%s` = the percent sign, with a blank before it in
the languages 1, 2, 6, 7 and 13), `BOUNTY_DATA` = stable bounty (`%$d`), `CASH_DATA` = cash. The main menu card prints
`GAME_COMPLETED_DATA` = `game`, `CAREER_COMPLETED_DATA` = `career`, `RAP_SHEET_ITEMS_DATA_2` = `rap`,
`CHALLENGE_SERIES_DATA_1` = `series_done/series_total` (`%d/%d`), `CHALLENGE_SERIES_DATA_2` = `series`, plus the profile
name; the name and statistics groups are hidden when the front-end database byte `+3` is clear. The object
`RAP_SHEET_ITEMS_DATA_1` is hidden and gets no value. **[exe]**

## 5. Reputation sheets (race events, milestones, bounty)

These are the pages behind the safe house item "Blacklist N" and the pause menu (the pages of the same classes with the
`InGame*` package names and a request argument of 1). They are map pages: a 3 x 3 grid of `EVENT_ICON_n` objects over a
`TRACK_MAP` picture, a list on the left and a title object `0xBDE82FCC`. They show data owned by the gameplay
manager; the data are in [career-milestones.md](career-milestones.md) and [career-blacklist.md](career-blacklist.md).
**[exe]**

| Page (safe house / in-game package) | Class | Title (safe house / in-game) | Rows |
|---|---|---|---|
| `SafehouseReputationOverview` / `InGameReputationOverview` | `0x00561130`, handler `0x0055A4D0` | `TITLE_BLACKLIST` / `TITLE_BLACKLIST_INGAME` | four menu entries (`REP_SHEET_RACE_EVENTS`, `REP_SHEET_MILESTONE_EVENTS`, `REP_SHEET_BOUNTY`, `REP_SHEET_BIO`) and the rival panel (below) |
| `SafeHouseRaceSheet` / `InGameRaceSheet` | `0x0055A760`, handler `0x00551530`, fill `0x00547E30` | `REP_SHEET_RACE_EVENTS` / `BL_REP_RACE_EVENTS` | the race events of the viewed bin |
| `SafeHouseMilestones` / `InGameMilestones` | `0x0055A9A0`, handler `0x00551F80`, fill `0x00552440` | `REP_SHEET_MILESTONE_EVENTS` / `PRP_HEADER_MILESTONES` | the bin's milestones, then its speed traps |
| `SafeHouseBounty` / `InGameBounty` | `0x0055ACA0`, handler `0x00552570`, fill `0x005529F0` | `REP_SHEET_BOUNTY` / `TIP_BOUNTY_HEADER` | the bin's bounty spawn points |

The viewed bin is the global `0x0091CA3C`, set by the safe house item "Blacklist N" to the current stage
([career-safehouse.md](career-safehouse.md) 5.1). The overview's four entries open, by index (0, 1, 2, 4): the race
sheet, the milestones page, the bounty page and the rival bio; the rival challenge opens from the panel button.
`Back` returns to the overview, the overview's Back to `MainMenu_Sub` (safe house) or the pause menu. **[exe]**

Rival panel (`0x0052EE50`, on the overview): from the bin record of the viewed bin it shows races won (u16 `+0x1A`),
challenges passed (u16 `+0x18`) and the stable bounty against the bin's three requirements (attributes `0xD617FEDC`,
`0x6DD4B98B`, `0xD3657D92`), with a tick per met requirement; the same comparisons that unlock the rival
([career-milestones.md](career-milestones.md) 6). Titles `TITLE_BLACKLIST_INGAME_NUMBER` / `TITLE_BLACKLIST_NUMBER`
with the rank, the alias `BLACKLIST_RIVAL_%02d_AKA`, the challenge text `BLACKLIST_RIVAL_%02d_CHALLENGE`, "DEFEATED"
for a beaten rival, and `BL_BOUNTY_REWARD` with the reward bounty. **[exe]**

**Race sheet.** Each row is built from an event key of the viewed bin (the bin's event lists, `0x00601060`) when the event
record is usable (`0x0051F120`); it is marked done when the record has flag 2. A second mode lists the events of every
bin from the current stage onwards; the constructor starts in the viewed-bin mode and what switches the mode was not
followed. Accept asks `REP_SHEET_RACE_CONFIRM` (YES/NO dialog; in the pause menu the overlay `InGameDialog`) and, on
yes (`0xD05FC3A3`), launches the event through the gameplay manager (in game: the manager's launch of the event key;
in the front end: marks the event record and queues it with `0x0057ECA0`). **[exe]**, launch internals **[unconfirmed]**

**Milestones page.** Rows come from the manager's milestone list of the bin (every state; a state 1 row is drawn locked,
a state 4 row complete) followed by the bin's speed traps (a trap without the unlocked bit is locked, with the done bit
complete). Row datum classes `MilestoneDatum` (`0x0089C3A4`) and the trap datum (`0x0089C3B0`). The selected row sets a
map pin from its spawn point. Accept on a completed row only plays sound 7. Otherwise it asks `REP_SHEET_MILESTONE_CONFIRM`
(`REP_SHEET_PHOTOTICKET_CONFIRM` for a speed trap), YES/NO. On yes the front end writes the **pending jump** into the
script manager: spawn point key (`+0x11C`), pending flag byte `+0x111 = 1`, `+0x114 = 0`, then leaves the front end the
way "Resume Free Roam" does (`0x0056C5B0`); the pause menu variant calls the manager's jump routine (`0x005EBC80`) at
once, which stores `+0x2B4 = 1`, the key at `+0x2B8` and moves the player car to the spawn point. The reader of
`+0x11C` was not found; the flag `+0x111` is read by two pursuit-start gates. The launched milestone then starts its
pursuit through the normal milestone/pursuit machinery. **[exe]**, consumer **[unconfirmed]**

**Bounty page.** Rows are the entries of the manager's `bounty_jump` list (up to 20 keys, attribute `0xDB89AB5C` = the
spawn point number), ordered by that number. The first 4 rows are always shown, rows 5 to 8 while the stage byte is
below 13, rows 9 and up while it is below 9. Row label `BLACKLIST_BOUNTY_SPAWN_POINT_%02d`; the rows form three groups
with their own headings. Confirm text `REP_SHEET_BOUNTY_CONFIRM`; the jump uses the same pending-jump fields as the
milestone page (flag `+0x111 = 1`, `+0x114 = 0`, spawn key `+0x11C`). **[exe]**

## 6. Implementation notes for the Rust rewrite

- Model the totals as functions over the stable, never stored: `bounty_total`, `evaded_total`, `busted_total`,
  `infractions(kind, set)`, `fines(set)`, `impounded`, `monitored` (file 1 section 1.4), plus the sold-history record.
- `fine(counters)` with the eight amounts read from the data (`infractions` collection) and the fixed index order.
- `RapSheet` state machine: an enum of pages (Login, Login2, Main, Summary, Vehicles, Infractions, CostToState,
  TopPursuits, PursuitDetail(n), Rankings, RankingDetail(stat)) with the transitions of file 1 section 1.2; per-page
  remembered selection kept by (page, context); two view bytes for the rankings.
- `PursuitHistory` struct: 8 sums, 10 best pairs, 8 extra sums, last car key, top-5 array; `record(pursuit)` exactly as
  section 2 (strict comparisons, busted pursuits only set the last car key).
- `rank(stat, career)` as section 1.2 from the `FE_ATTRIB` collections; the time formatter of section 1.3 in `libs/`
  (generic) with the language separators supplied by the caller.
- The percentages of section 4 belong with the safe house menu and the main menu card.
- Keep the label keys from this page; the text comes from the language table. The two swapped speeding labels of the
  infractions page (file 1 section 5) should be kept as in the original or flagged as a deliberate fix.
- Free-form mods: the modding career table can answer "rank of statistic s" and "top 5 pursuits" from these structures
  ([../modding/api-nfsmw.md](../modding/api-nfsmw.md)).

## 7. Open questions

1. How `Main_ENDGAME` ends, and where context bit 18 is cleared after the end-game chain.
2. The pursuit-side producers of the extra slots `+0xA8`, `+0xAC`, `+0xB0`, `+0xB4`, `+0xBC`, `+0xC0`, `+0xC4`, `+0xC8`
   (meaning is inferred from labels and unit prices).
3. Whether the rap sheet's swapped speeding labels are intended (file 1 section 5).
4. The consumer of the pending-jump fields (`+0x111`, `+0x114`, `+0x11C`) at the next world load.
5. What switches the race sheet between its two list modes.
6. Why the language table keeps unused cost lines (`RAP_CTS_*` salaries, fuel, speed traps) and a `RAP_RANKINGS_SELECT`
   prompt on a page that does not show it in the install's package.
7. The audio effect of context bit 9 beyond the state number 13.
