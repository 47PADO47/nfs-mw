# Career text: what the language table adds

The career screens show almost no text of their own: names, aliases, cars, challenge goals, rap sheet wording, impound
and marker texts all come from the language table. This page says which label feeds which screen field, resolves the ids
that the save and the data files store (popup types, label indexes, bonus-car messages) and lists the 15 Blacklist rivals.
The messages (SMS), tips, tutorials and help screens are in [career-text-2.md](career-text-2.md). Method and tools are in
the [provenance file](../provenance/career-text.md).

- **Sources:** `LANGUAGES/English.bin` (4,586 strings) and `LANGUAGES/Labels.bin` (label names for 4,546 of them), decoded
  with a private script that follows the format of [text.md](../formats/text.md); `speed.exe` through the Ghidra MCP (the
  `printf`-style formats and the screens that use them); `GLOBAL/gameplay.bin`, `GLOBAL/fe_attrib.bak` and the saves for
  the ids. The mod backup `LANGUAGES/ENGLISH.BIN.bacc` (the 2005 table, 4,546 strings) was compared too.
- **Evidence tags:** **[exe]** read in `speed.exe`, not tested in game. **[verified]** read from the install files or the
  saves. **[unconfirmed]** a guess.
- **Copyright:** the text is game text. This page quotes only names, aliases, car names, one-line labels and ids; bios,
  messages and tips are described in the author's words, one short line each.
- **Addresses** are `speed.exe` addresses. Ghidra auto names are only in the provenance file.

## 0. Summary

1. A label is found by `bStringHash(name)`: seed `0xFFFFFFFF`, `h = h * 33 + upper(c)`. All 4,546 names of `Labels.bin`
   hash to their own record key, case-insensitively (a 100 percent match; `text.md` counted 4,255 because it hashed the
   names as written). **[verified]**
2. The exe builds career labels from formats with a two-digit index: `BLACKLIST_RIVAL_%02d_{NAME,AKA,CAR,BIO,STRENGTH,
   CHALLENGE,LEADERBOARD}`, `BLACKLIST_BOUNTY_SPAWN_POINT_%02d[_DESC]`, `BLACKLIST_PURSUIT_MILESTONES_%02d[_SHORT]`,
   `RAP_SUMMARY_%.2d`, `SMS_MESSAGE_%d[_FROM|_SUBJECT]`, `CARNAME_%s_%s`, `CARSELECT_MANUFACTURER_%s`. **[exe]**
3. The 15 rivals are bins 15 down to 1; each has a name, an alias, a car, a strength class, a bio and a challenge line. Bin 16
   (the prologue) has only an alias and a leaderboard label (`Rog`). **[verified]**
4. The 95 valid SMS slots of the save are exactly the 95 `SMS_MESSAGE_<n>` texts of the table. **[verified]**
5. The popup queue types 1 to 0x15 and the car-lot lock messages resolve to the `MARKER_NAME_*` and
   `CARLOT_BONUSCAR_UNLOCK_MSG_*` labels (sections 5.3 and 7). **[exe + verified]**
6. The "57 hint counters" of the save are not hints: they are the shop's part-unlock tiers
   ([career-text-2.md](career-text-2.md) section 4). **[exe]**
7. The installed `English.bin` has 40 strings that the 2005 table lacks: add-on car names and part names. No career label
   differs from the 2005 table. **[verified]**

## 1. How a label reaches the screen **[exe]**

A small helper formats a label name from a `printf` format and an index (the current Blacklist rank or the SMS id, taken
from the caller), hashes it with the function above and looks it up. Fields on the FEng screens are filled by hashed
object name. The rival bio screen (`SafehouseRivalBio.fng` and the in-game copy `InGameRivalBio.fng`) sets:

| FEng object | Label |
|---|---|
| `0x242657CE` (title) | `TITLE_BLACKLIST_NUMBER` ("blacklist %d", safe house) or `TITLE_BLACKLIST_INGAME_NUMBER` (in game) |
| `0x7AC3D0C9` | `BLACKLIST_RIVAL_<NN>_NAME` |
| `0xB1F2748D` | `BLACKLIST_RIVAL_<NN>_AKA` |
| `0x27E1D6D8` | `BLACKLIST_RIVAL_<NN>_BIO` |
| `0xCB5BF41A` | `BLACKLIST_RIVAL_<NN>_CAR` |
| `0xA6F07BF3` | `BLACKLIST_RIVAL_<NN>_STRENGTH` |

The rival challenge screen uses `_CHALLENGE` (a one-line prompt about the figures to reach) and `_AKA`; the rankings and
race lists use `_LEADERBOARD` (the alias in capitals); the car select list uses the lower-case form
`blacklist_rival_<NN>_aka` as the sub-title of a rival's preset car. `RIVAL_<NN>`, `RIVAL_<NN>_BG`, `_GRAF` and `_IG`
are FEng object names (pictures), not labels.

Label families in `English.bin` (counts by name prefix) **[verified]**: `OL_`/`ON_` online text 922; `PN_` part names 331;
`SMS_` 293; `TRACKNAME` 210; `blacklist_` 170; `SUBTITLE_` 151; `RACERNAME` 150; `RAP_` 146; `CHALLENGE_*` 144; `OPT_` 121;
the shop `CUSTOMIZATION_`/`CO_`/`PD_`/`CT_`/`CD_`/`CP_`/`CB_`; `MARKER_` 49; `TIP_` 48; `helpscreen_` 43; `REP_` 33;
`PAUSE_` 33; `WM_` 31 (world map); `TUTORIAL_` 27; `CS_` 27; `BL_` 25; `MW_` 12; `SB_` 8; `INFRACTION_` 8; `CARLOT_` 7.

## 2. The 15 Blacklist rivals **[verified]**

Bin 15 is the first rival, bin 1 the last. `Entry SMS` is the message the bin queues when it starts (the
`EntryCellCallID` of [career-blacklist.md](career-blacklist.md)); `Rival SMS` is the one sent when the rival unlocks.
Both are ids of the SMS table in [career-text-2.md](career-text-2.md). The sender and subject of each match the rival (for
example message 36 is Rog telling the player that Vic is ready to race), which confirms that both fields are SMS ids.
`Dev collection` is the developers' name in `gameplay.bin` (it differs from the shown name for bins 9 and 15).

| Bin | Name | Alias | Car | Strength | Dev collection | Entry SMS | Rival SMS |
|---|---|---|---|---|---|---|---|
| 15 | Ho Seun | Sonny | VW Golf GTI | Circuit Races | `tony_ho` | - | 20 |
| 14 | Vince Kilic | Taz | Lexus IS300 | Sprint Races | `vince_kilic` | - | 29 |
| 13 | Victor Vasquez | Vic | Toyota Supra | Pursuit Length | `victor_vasquez` | 30 | 36 |
| 12 | Isabel Diaz | Izzy | Mazda RX-8 | Lap Knockout Races | `isabel_diaz` | 37 | 41 |
| 11 | Lou Park | Big Lou | Mitsubishi Eclipse | Tollbooth Time Trials | `lou_park` | 42 | 46 |
| 10 | Karl Smit | Baron | Porsche Cayman S | Infractions | `karl_smit` | 48 | 52 |
| 9 | Eugene James | Earl | Mitsubishi Lancer EVOLUTION VIII | Sprint Races | `pierre_dupont` | 54 | 57 |
| 8 | Jade Barrett | Jewels | Ford Mustang GT | Drag Races | `jade_barrett` | 58 | 63 |
| 7 | Kira Nakazato | Kaze | Mercedes-Benz CLK 500 | Cost To State | `kira_nazakato` | 64 | 69 |
| 6 | Hector Domingo | Ming | Lamborghini Gallardo | Photo Tickets | `hector_domingo` | 70 | 72 |
| 5 | Wes Allen | Webster | Corvette C6 | Escaping Pursuits | `wes_allen` | 73 | 76 |
| 4 | Joe Vega | JV | Dodge Viper SRT10 | Speedtrap Races | `joe_vega` | 77 | 79 |
| 3 | Ronald McCrea | Ronnie | Aston Martin DB9 | Immobilizing Police Vehicles | `ronnie_mccrea` | 80 | 84 |
| 2 | Toru Sato | Bull | Mercedes-Benz SLR McLaren | Sprint Races | `toro_sato` | 85 | 88 |
| 1 | Clarence Callahan | Razor | BMW M3 GTR | Everything | `razor_callahan` | 89 | 93, 94 |

Notes:

- The car is the display name. The physics car of a rival is the preset ride `BL<n>` of the boss collection (the preset
  list is not in `attributes.bin`; see [career-car-lot.md](career-car-lot.md) section 6). Rival 1's boss car is the E3
  demo BMW preset.
- The strength is a race kind or a pursuit statistic: Circuit, Sprint (three rivals), Lap Knockout, Tollbooth, Drag,
  Speedtrap, Photo Tickets, Cost To State, Infractions, Pursuit Length, Escaping Pursuits, Immobilizing Police Vehicles,
  and "Everything" for Razor.
- Names are as in the table. The data name `kira_nazakato` is spelled differently from the shown `Nakazato`; `Eugene
  James` is the shown name of the collection `pierre_dupont`.
- Bin 16 (prologue): the alias label is `Rog` and the leaderboard label `ROG`; there is no name, car, bio or strength.
- The rap sheet login page lists six boss surnames as login names (`RAP_LOGIN2_*`: Allan, Callahan, Domingo, McCrea, Sato,
  Vega): the accounts the player can pick on the fake police terminal.

### 2.1 Where a rival's data shows **[exe + verified]**

| Where | Labels |
|---|---|
| Safe house `Blacklist <n>` button and reputation overview | `MAIN_TOP_15` (the static "Blacklist 15" caption), `TITLE_BLACKLIST_INGAME_NUMBER`, `REP_*` headers |
| Rival bio page | the five labels of section 1, `BL_BIO`, `BL_RIDE`, `BL_STRENGTH`, `BL_VIEW_RIVAL_CAR`, `BL_VIEW_MOVIE` |
| Rival challenge page | `_CHALLENGE`, `BL_GET_FOLLOWING`, `BL_RACE_WINS`, `BL_MILESTONE_WINS`, `BL_BOUNTY`, `BL_COP_PROBABLITY`, `BL_CHALLENGE_RIVAL` |
| After the bin is passed | `BL_DEFEATED`, `BL_CONGRATULATIONS`, `BL_CASH_REWARD`, `BL_BOUNTY_REWARD`, `BL_NEXT_RIVAL`, `RACE_BIN_PASSED`, `RACE_RIVAL_UNLOCKED` |
| Arrival sequence movie | the `blacklist_<NN>` movie in `FEAnyMovie.fng`; subtitles `SUBTITLE_BL_<n>` and `SUBTITLE_BL_NUM_<n>` ([text.md](../formats/text.md)) |
| Car select, locked rival car | `carlot_unlock_msg_01`: "unlock by beating <alias>, Blacklist member <n>" |

## 3. Pursuit milestone labels **[exe + verified]**

`BLACKLIST_PURSUIT_MILESTONES_<NN>` is the description (with `%s` slots for the numbers) and `_<NN>_SHORT` the title of a
milestone. The index is the `Label` field of the statistic ([career-milestones-data.md](career-milestones-data.md) table 1).
The text resolves the "label meanings" of that page; all 19 exist:

| NN | Short title | Description (own words) |
|---|---|---|
| 00 | Spike Strip Challenge | dodge N spike strips, then evade |
| 01 | Pursuit Time Challenge | stay in a pursuit for at least N minutes |
| 02 | Pursuit Evasion | evade a pursuit in under N minutes |
| 03 | Bounty | earn N bounty in one pursuit, then evade |
| 04 | Evade consecutive pursuits | evade N pursuits in a row without being busted (no milestone collection uses it) |
| 05 | Cost to State Challenge | evade with a cost to state of at least N |
| 06 | Roadblock Challenge | dodge N roadblocks, then evade |
| 07 | Trade Paint | tag at least N police vehicles, then evade |
| 08 | Infractions Challenge | evade with at least N infractions |
| 09 | Speedtrap Challenge | be clocked above a speed (used by the speed traps) |
| 10 | Speeding tickets | collect N speeding tickets on the rap sheet |
| 11 | Ramming a Police Vehicle Tickets | the same for ramming tickets |
| 12 | Speeding tickets | duplicate of 10 (same wording; probably meant for excessive speeding) |
| 13 | Reckless Driving tickets | the same for reckless driving |
| 14 | Hit & Run tickets | the same |
| 15 | Damage to Property tickets | the same |
| 16 | Resisting Arrest tickets | the same |
| 17 | Driving off Roadway tickets | the same |
| 18 | Achieve Bounty | a minimum bounty on N cars |

Indexes 10 to 18 are ticket-count and per-car bounty milestones; the statistics behind them (`speeding_ticket`,
`hit_and_run_ticket`, ...) have no producer in the exe ([career-milestones.md](career-milestones.md)), so these texts are
shown only if a milestone references them **[unconfirmed whether any does]**.

Other milestone wording: `HUD_MILESTONES`, `HUD_MILESTONE_COMPLETE`, `HUD_DONT_GET_BUSTED_60/90/120` (a timer goal),
`RACE_CHALLENGE_PASSED_*` (pursuit length 60/120/300 s, 3/6 cops in pursuit, 3/6/10 cops immobilized), `MILESTONES_ACHIEVED`,
`BL_MILESTONES`. The rep sheet asks before it jumps to an event: `REP_SHEET_MILESTONE_CONFIRM`, `REP_SHEET_BOUNTY_CONFIRM`,
`REP_SHEET_RACE_CONFIRM`, `REP_SHEET_PHOTOTICKET_CONFIRM`. **[verified text]**

## 4. Bounty spawn points **[exe + verified]**

`BLACKLIST_BOUNTY_SPAWN_POINT_<NN>` (title) and `_DESC` (a hint line) are the 12 places the bounty page offers as "jump
here and start a pursuit"; `blacklist_bounty_spawn_point_first_time` is the one-time warning (these runs raise the
bounty). The page asks for confirmation with `REP_SHEET_BOUNTY_CONFIRM`.

| # | Place | # | Place |
|---|---|---|---|
| 1 | Rosewood Police Station | 7 | Camden Penitentiary |
| 2 | Rosewood Fire Station | 8 | Dunwich Village |
| 3 | Forest Green Country Club | 9 | Rockport Police Station |
| 4 | Doughnut Shop | 10 | Rockport Fire Station |
| 5 | Ocean Hills Police Station | 11 | Riverfront Stadium |
| 6 | Ocean Hills Fire Station | 12 | Rockport Century Square |

## 5. Safe house, career menu and car lot labels

### 5.1 Menus **[exe + verified]**

| Screen | Labels |
|---|---|
| Main menu | `MAIN_CAREER`, `CHALLENGE_MODE`, `MAIN_QUICK_RACE`, `MAIN_CUSTOMIZE` (My Cars), `MAIN_LAN`, `MAIN_ONLINE`, `SAVELOAD_PROFILE_MANAGER` (Alias Manager), `MAIN_OPTIONS` |
| Safe house menu | `PAUSE_RESUME_FREE_ROAM`, `MAIN_TOP_15`, `OLX_CAR_SELECT_TITLE` (Car Select), `MAIN_RAP_SHEET`, `SAVE`; confirm `CONFIRM_RESUME_FREE_ROAM` |
| Screen titles | `TITLE_CAREER`, `TITLE_SAFEHOUSE`, `TITLE_BLACKLIST`, `TITLE_MARKER`, `TITLE_CUSTOMIZE`, `TITLE_CHALLENGE_SERIES`, `TITLE_QR`, `TITLE_OPTIONS` |
| Pause menu in career | `PAUSE_QUIT_TO_CRIB` ("Jump to Safe House"), `PAUSE_CONFIRM_QUIT_FREEROAM_SAFEHOUSE`, `PAUSE_CONFIRM_QUIT_RACE_SAFEHOUSE`, `PAUSE_CONFIRM_QUIT_PURSUIT_SAFEHOUSE`, `PAUSE_CONFIRM_QUIT_GAME_CAREER` |
| World map | `WM_SAFEHOUSES`, `WM_SHOPS`, `WM_CAR_LOTS`, `WM_PURSUIT_BREAKER`, `WM_HIDING_SPOT`, `WM_COP_CAR/_CARS/_HELI`, `WM_RACE`, `WM_EVENT`, `WM_PHOTOTICKET`, `WM_SPEEDTRAP`, `WM_NAVIGATION`, four zoom labels |
| Regions | `REGION_NORTHAMERICA/EUROPE/JAPAN` (the car classes), `REGION_UNLOCK_COSTAL` (Camden Beach opened), `REGION_UNLOCK_DOWNTOWN` (Rockport opened); eight `NEIGHBOURHOOD_*` names |
| Race types | `RACETYPES_*`: sprint, circuit, knockout, drag, speedtrap, tollbooth, phototicket, cellphone, challenge, free run, custom |

The two region texts fit the stage rule of [career-flow.md](career-flow.md) section 7.3 (a region unlock screen at stages 8 and
12).

### 5.2 Car lot and car names **[exe + verified]**

- Purchase and sale dialogs: `CAR_LOT_NO_CASH`, `CAR_LOT_NO_ROOM`, `CAR_PURCHASE_CONFIRM`, `CARLOT_BUYMSG` (drive now or
  deliver), `CARLOT_DRIVE_NOW`, `CARLOT_DELIVER`, `CS_SELL_CAR`, `CS_NO_DELETE_IMPOUNDED_CAR`; the hashes are named in
  [career-car-lot.md](career-car-lot.md) sections 3 and 4.
- A car's shown name is `CARNAME_<maker collection>_<car collection>`, for example `CARNAME_BMW_M3_GTRE46`. The table holds
  43 such names; the maker logo uses `CARSELECT_MANUFACTURER_<name>`. Three names carry a "- BUG" suffix in the data
  (`BMW_M3_CSL`, `MAZDA_RX8SPEED`, `MERCEDES_SL55`); they are unused cars.
- There is **no price label**: prices come from the `frontend` data and are drawn with the cash format `%$d`.
- The six Car Select tabs are `CARLIST_STOCK` (lot cars, car flag 1), `CARLIST_CAREER` (owned, 2), `CARLIST_QUICKRACE` ("My
  Cars", 4), `CARLIST_BONUS` (8), `CARLIST_PRESET` (0x10) and a debug tab (0x20). This names the low half of the stable's car
  flags ([career-save.md](../formats/career-save.md) 4.1).

### 5.3 Bonus car unlock messages **[exe + verified]**

The car select screen shows one of these for a locked bonus preset (the hash is chosen by the preset's handle):

| Label | Meaning | Preset (handle) | Settings flag |
|---|---|---|---|
| `CARLOT_BONUSCAR_UNLOCK_MSG_01` | complete event 68 of the Challenge Series | `BONUS_SL65` (`0xCB6AAF2F`) | bit 15 |
| `CARLOT_BONUSCAR_UNLOCK_MSG_02` | complete every Challenge Series event | `BONUS_GT2` (`0x2CF385B2`) | bit 16 |
| `CARLOT_BONUSCAR_UNLOCK_MSG_03` | win the career | `E3_DEMO_BMW` (`0x03A94520`) | bit 14 |
| `CARLOT_BONUSCAR_UNLOCK_MSG_04` | reach 100 percent game complete | `BONUS_C6R` (`0x2CF370F0`) | bit 14 |

The preset names were recovered by hashing candidate names (`BONUS_<car>`, `CE_<car>`); `C6R` shares its hash with three
other three-letter suffixes, but the Corvette C6.R is the only one that is a car. Presets named `CE_*` (Collector's
Edition) are the Black Edition "Bonus" cars: `CE_ELISE`, `CE_SL500`, `CE_SUPRA`, `CE_CAMARO`, `CE_CORVETTE`, `CE_997S`,
`CE_SL65`, `CE_GT2`, `CE_C6R`, and one not resolved (`0x363A1FEA`). One more flag-gated preset, `0x34498EB2` (bit 18),
was not named. Rival cars use `carlot_unlock_msg_01` with the rank `n` from 2 to 15. The label `CS_UNLOCKMSG` ("unlock by
beating Blacklist member %s") is not referenced by the exe. Why both C6R and the E3 BMW use flag bit 14 although their
messages differ is **[unconfirmed]**.

## 6. Rap sheet **[exe + verified]**

### 6.1 Pages and labels

| Page (FEng) | Labels (prefix `RAP_`) |
|---|---|
| Login `RapSheetLogin`, `RapSheetLogin2` | `LOGIN_*` (fake terminal text), `LOGIN2_*` (six surnames) |
| Main `RapSheetMain` | `MAIN_RAP_SHEET`, `MAIN_SUMMARY`, `MAIN_RANKING(S)`, `MAIN_TEP` (top 5 pursuits), `MAIN_VD` (vehicle database), `MAIN_INFRACTIONS`, `MAIN_CTS` (cost to state) |
| Summary `RapSheetRS` | `RS_*`: pursuits, fines, police vehicles damaged/immobilized, served and unserved infractions, times busted/evaded |
| Infractions `RapSheetUS` | `US_*`: eight infraction names, served/unserved/total |
| Vehicle database `RapSheetVD` | `VD_*`: per car bounty, fines due, unserved infractions, times busted and evaded, status |
| Cost to state `RapSheetCTS` | `CTS_*`: 14 cost lines and the total |
| Top 5 pursuits `RapSheetTEP`, detail `RapSheetPD` | `TEP_*`, `PD_*` ("pursuit detail") |
| Rankings `RapSheetRankings(Detail)` | `RANKINGS_*`: lifetime / single pursuit toggles, unranked |

The summary page fields **[exe]** (label -> value):

| Label | Value |
|---|---|
| `RAP_CASE_FILE` | the profile serial text (settings +0x26C) |
| `RAP_PLAYER_NAME` | the profile name |
| `RAP_TOTAL_BOUNTY` | total bounty (stable base plus the owned cars) |
| `RAP_TOTAL_COST_STATE` | statistics counter 7 |
| `RAP_TOTAL_CARS_IMPOUNDED` | number of impounded owned cars |
| `RAP_RS_UNSERVED_INFRACTIONS`, `RAP_RS_SERVED_INFRACTIONS` | the sum of the 8 unserved / served counts (stable tail `0x8CA8` and `0x8CB8`, plus the owned cars) |
| `RAP_RS_TIMES_EVADED`, `RAP_RS_TIMES_BUSTED` | owned cars plus the stable tail u16 at `0x8CA4` and `0x8CA6` |
| `RAP_RS_FINES_DUE`, `RAP_RS_FINES_PAID` | sums over the owned cars |
| `RAP_RS_POLICE_DAMAGED`, `RAP_RS_POLICE_DESTROYED` | statistics counters 2 and 3 |

So the stable tail of [career-save.md](../formats/career-save.md) is: `0x8CA4` evaded base, `0x8CA6` busted base, `0x8CA8`
unserved infractions by type, `0x8CB8` served infractions by type. The "statistics counters" are the u32 array at
statistics block `+0xB44` (memory `profile + 0x9C24`): index 2 = police vehicles damaged, 3 = immobilized, 7 = cost to state.

### 6.2 The rank summary line

`RAP_SUMMARY_01` to `05` are five police summary lines, from "minor code infractions" (routine check) through major,
chronic and gross, to "wanton disregard" (apprehend at all cost, heavy backup). The line is chosen by the total number of
infractions (served plus unserved) against the array `[0, 100, 400, 800, 1000]` in the frontend collection
`rapsheet_summary_strings`: the first threshold that is at least the total gives the line. Whether the printed index is
that position or one more is **[unconfirmed]** (position 0 would give line 0, which does not exist, so one more is likely).

### 6.3 Infraction names

Eight names, the same in `INFRACTION_*` (pursuit HUD and busted screens) and `RAP_US_*`:

| Bit | `INFRACTION_` key | Shown |
|---|---|---|
| 1 | `SPEEDING` | Excessive Speeding |
| 2 | `RACING` | Speeding |
| 4 | `RECKLESS_DRIVING` | Reckless Driving |
| 8 | `ASSULTING_OFFICER` | Ramming a Police Vehicle |
| 16 | `HITNRUN` | Hit and Run |
| 32 | `PROPERTY_DAMAGE` | Damage to Property |
| 64 | `RESISTING_ARREST` | Resisting Arrest |
| 128 | `OFFROAD` | Driving Off Roadway |

(The bits are those of [ai-pursuit-heat.md](ai-pursuit-heat.md) section 4; the fines are in the economy pages.) The
post-pursuit page also has `PRP_INFRACTIONS_*` lines, including one for fleeing. **[verified]**

## 7. Markers, popups and impound **[exe + verified]**

### 7.1 Popup queue types and marker labels

The pending popup queue of the save ([career-save.md](../formats/career-save.md) 3.2) holds a type per entry. The types
are the reward kinds of the exe's reward table; each has a marker label pair `MARKER_NAME_<X>` (title) and
`MARKER_NAME_DESCRIPTION_<X>` (one sentence):

| Type | Reward | Label key | Marker category |
|---|---|---|---|
| 1 | brakes upgrade | `BRAKES` | performance |
| 2 | engine upgrade | `ENGINE` | performance |
| 3 | nitrous | `NOS` | performance |
| 4 | induction | `TURBO` and `SUPERCHARGER` | performance |
| 5 | suspension | `CHASSI` | performance |
| 6 | tires | `TIRE` | performance |
| 7 | transmission | `TRANSMISSION` | performance |
| 8 | body kit | `BODYKIT` | parts |
| 9 | hood | `HOOD` | parts |
| 10 | spoiler | `SPOILER` | parts |
| 11 | rims | `RIMS` | parts |
| 12 | roof scoop | `ROOFSCOOPS` | parts |
| 13 | custom gauges | `HUD` | visual |
| 14 | vinyls | `VINYLS` | visual |
| 15 | decals | (`CB_MARKERS_DECAL`) | visual |
| 16 | paint | (`CB_MARKERS_PAINT`) | visual |
| 0x11 | Get Out of Jail Free | `FREEBAIL` | miscellaneous |
| 0x12 | pink slip to the rival's car | `PINKSLIP` | miscellaneous |
| 0x13 | extra cash | `CASH` | miscellaneous |
| 0x14 | extra impound strike | `IMPOUND` | miscellaneous |
| 0x15 | release a car from impound | `RELEASE` | miscellaneous |

Category titles are `MARKER_CATEGORY_PERFORMANCE`, `_PARTS`, `_VISUAL` and `_MISC` ("Bonus Markers") with `_DESC_*`
explanations; the marker-select page counts remaining and used markers per kind (`CS_*_MARKER_REMAINING/_USED`,
`CB_MARKERS_*`). The rules the texts state: a rival gives a choice of **two** markers (`TIP_MARKER_REWARDS`), the first
safe house grants **three** Get Out of Jail Free markers (`MARKERS_SAFEHOUSE_FREE`), and a car's impound strike limit can
be raised to at most **5** with the extra-strike marker (`CS_ADD_IMPOUND_BOX`, `TIP_STRIKEOUT`). The type 2 entry that a
new career queues when the bonus probe is true is therefore an engine-upgrade marker.

### 7.2 Busted, impound and game over

| Situation | Labels |
|---|---|
| Busted dialog and pay screen | `SB_PAY_INFRACTIONS`, `MW_PAYMENT_FOR`, `MW_VIOLATION_HEADER`, `MW_TOTAL_COST`, `MW_PAY_WITH_CASH`, `MW_PAY_WITH_MARKER`, `MW_THIS_PURSUIT_HEADER`, `MW_CASH_HEADER` |
| First time | `BUSTED_FIRST_TIME_DLG` (pay cash or use a marker; with neither the car is impounded) |
| Impounded | `SB_IMPOUND_CAR_NOT_ENOUGH_CASH`, `SB_IMPOUND_TOO_MANY_TIMES_BUSTED`, `MW_BUSTED_INSUFFICIENT_FUNDS`, `MW_BUSTED_STRIKE_LIMIT`, `IB_IMPOUND`, `IMPOUNDED`, `SB_IMPOUNDED_GET_DIFFERENT_CAR`, `CS_IMPOUND_MUST_CHOOSE_NEW_CAR` |
| Busted by Cross | `SB_CROSS`, `MW_BUSTED_BY_CROSS` |
| Releasing a car | `CS_IMPOUND_RELEASE_CASH`, `CS_IMPOUND_RELEASE_MARKER`, `CS_IMPOUND_RELEASE_CASH_MARKER`, `CS_GET_MARKER_OR_MORE_CASH`, `CS_RELEASE_CAR`, `CS_RELEASE_HEADER`, `SB_IMPOUNDED_RELEASE_CAR` |
| Game over | `SB_GAME_OVER`, `GAME_OVER`, `GAME_OVER_02` (all cars impounded and no cash: start a new career or load) |
| End game | `EG_*` (the Most Wanted warrant screen on `GameWon.fng`), `REP_Game_Complete_Header`, `GAME_COMPLETED` |
| Busted overlay crawl | `BUSTED_OVERLAY_CRAWL_TEXT_00..05`, `BUSTED_VIDCAM`, `BUSTED_REC` (the surveillance-camera look) |

An impounded car in the safe house cannot be sold ([career-car-lot.md](career-car-lot.md) section 4). The impound help
(`helpscreen_text_impound`) says that many busts in one car impound it and that evading several pursuits lowers its
strikes, which matches the strike decay of [career-economy.md](career-economy.md).

## Open questions

1. Whether any milestone collection references the ticket-count labels 10 to 18.
2. The unresolved presets `0x363A1FEA` (Collector's Edition) and `0x34498EB2` (bit 18).
3. The exact index printed by `RAP_SUMMARY_%.2d`.
4. Which FEng objects show `_LEADERBOARD` and `_CHALLENGE` (named by the labels, not traced).
