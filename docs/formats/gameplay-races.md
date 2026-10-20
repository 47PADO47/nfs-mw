# Race events in the gameplay vaults (`gameplay` class)

How the original describes a race: the `gameplay` class of `GLOBAL/gameplay.bin` holds the race events,
their markers and triggers, and the opponent drivers. This page lists the fields a race uses, the parent
templates, the coordinate convention of the markers and real values from this install. How the AI uses the
data (route building, targets, skill, catch-up) is in [specs/ai-racers.md](../specs/ai-racers.md),
[specs/ai-racers-route.md](../specs/ai-racers-route.md) and [specs/ai-racers-catchup.md](../specs/ai-racers-catchup.md).
The container (VPAK, vaults, collections, inheritance) is in [attributes.md](attributes.md). For the tag
meanings, see [evidence tags](../README.md#evidence-tags).

All numbers were read from this install (PC v1.3, `GLOBAL/gameplay.bin`) with a throwaway script on top of the
`blackbox-attrib` layout **[verified]**. Field names are the decomp's generated class header
(`Generated/AttribSys/Classes/gameplay.h`) **[decomp]**. Where a meaning comes from reading the code that
consumes the field in the PC `speed.exe` (machine code, understanding only) it is marked **[exe]**; that
counts as a restricted source like the decomp.

## 1. Where races live **[verified]**

- One class, `gameplay` (hash `0x5CEA9D46`), **220 fields**, defined in `attributes.bin`; its **6,293
  collections** are all in `gameplay.bin`, spread over **272 vaults**. One class holds everything:
  races, markers, triggers, characters (drivers), bins (the Blacklist ranks), activities and the Lua state
  graph handlers (the `bytecode` blobs of vault `gpcore`, see [attributes.md](attributes.md#blobs-lua-bytecode-verified)).
- A race event is a vault named like the event id with dots turned to underscores plus a kind, for example
  `10_2_1_sprint` (event id `10.2.1`), `1_5_3_speedtrap`, `1_7_3_drag`, `2_4_3_r_tollbooth` (the `r` is the
  reversed variant). Its collections are named `race_bin_<NN>/<vault>` (the race) and
  `race_bin_<NN>/<vault>/<child>` (its children). `<NN>` is the Blacklist bin 01..15; other prefixes are
  `race_bin_challenge`, `race_bin_challenge_tollbooth`, `race_bin_opm` and bare names (`16_1_0_partial_dday`,
  the D-Day story races).
- Children of the race `10_2_1_sprint` (22 collections): `startgrid`, `finishline`, `checkpoint1..9`,
  `shortcut1..10`, `wrongway`. The race collection lists its children in `Children` and names the roles in
  `racestart`, `racefinish`, `Checkpoint[]`, `Shortcuts[]`, `Opponents[]`.
- **255** non-template collections have a non-null `racestart` (they are the races and challenges). By parent
  template: tollbooth 68, point-to-point (sprint) 49, circuit 39, speed trap 32, lap knockout 18, drag 11,
  the `challengerace*` family about 32, 1 story shell (`nis_shell`).

### Parent templates (inheritance, `Template = True`)

`engaged` -> `basicrace` -> { `p2p` -> { `drag`, `tollboothrace`, `speedtraprace` }, `circuit` -> `lapknockout`,
`challengerace` -> `challengerace_*` }. Values set by the templates **[verified]**:

| Template | What it changes against its parent |
|---|---|
| `basicrace` | `NumLaps 3`, `CatchUp True`, `CatchUpSkill "0.2"`, `CatchUpSpread "1000"`, `CatchUpIntegral 5e-5`, `CatchUpDerivative 0.001`, `CatchUpOverride False`, `DifficultyLevel 100`, `TrafficLevel 100`, `CopDensity 100`, `DoCountdown True`, `RollingStart False`, `InitialPlayerSpeed 0`, `PlayerCarPerformance 1`, `TimeLimit 0`, `Region college`, `MaxHeatLevel 10` |
| `p2p` | `NumLaps 1`, `EventIconType p2p`, `IsLoopingRace False`, `RankPlayersByDistance True`, `CopsInRace True`, `BossRace False` |
| `circuit` | `EventIconType circuit`, `IsLoopingRace True` (3 laps from `basicrace`) |
| `lapknockout` | `EventIconType knockout`, `KnockoutsPerLap 1` |
| `drag` | `EventIconType drag`, `TrafficPattern "drag"` |
| `tollboothrace` | `EventIconType tollbooth`, `TimeLimit 90` (races set 40..70) |
| `speedtraprace` | `EventIconType speedtrap`, `RankPlayersByPoints True`, `RankPlayersByDistance False`, `OvertimePenaltyPerSec 10` |
| `challengerace` | `EventIconType challenge`, `PursuitRace True`, `CopsInRace True`, `RollingStart True`, `DoCountdown False`, `InitialPlayerSpeed 40`, `UseWorldHeat True`, `CopSpawnType copmidsize`, `PlayerCarType OPM_RX8_Version2` |

## 2. Race type **[exe]**

The game does not store the type as a number. It reads `EventIconType` (a string) and looks it up in an
11-entry table; the index is `GRace::Type` (the enum is also in the decomp's `GRace.h`):

| `EventIconType` | `GRace::Type` | | `EventIconType` | `GRace::Type` |
|---|---|---|---|---|
| `p2p` | 0 sprint (point to point) | | `speedtrap` | 5 |
| `circuit` | 1 | | `checkpointrace` | 6 |
| `drag` | 2 | | `cashgrab` | 7 |
| `knockout` | 3 | | `challenge` | 8 |
| `tollbooth` | 4 | | `speedtrapjump` / `milestonejump` | 9 / 10 |

Data seen: `p2p` 51, `tollbooth` 68, `challenge` 36, `circuit` 32, `speedtrap` 31, `knockout` 20, `drag` 11, empty 6
(the story shells). `checkpointrace`, `cashgrab` and the two jump types occur in no race of this install
**[verified]**. The D-Day circuit-parent races override the icon to `p2p`.

## 3. Fields of a race

Types: `GCollectionKey` is a 4-byte reference to another `gameplay` collection (key = hash of the collection
name, 0 = none). Arrays are `Attrib::Array`s. Counts are over the 255 races **[verified]**.

### 3.1 Route and geometry

| Field | Type | Meaning, values seen |
|---|---|---|
| `racestart` | key | the start-grid marker (class `startgrid`, parent `marker`): position and heading of the front of the grid |
| `racefinish` | key | the finish-line trigger (box trigger). Absent in a few story shells |
| `racestartReverse`, `racefinishReverse` | key | start and finish used when the player runs the race reversed (all 0 in this install; the reverse variants are separate `..._r_...` events) |
| `Checkpoint` | key[] | the ordered checkpoints (box triggers; in a tollbooth race `time_bonus_checkpoint*` triggers with an `Int32` `TimeBonus` of 25, 35 or 160 s stand among them **[verified]**), 0..15 per race (counts: 7 -> 58 races, 9 -> 23, 12 -> 19, 15 -> 18, 0 -> 39) |
| `Shortcuts` | key[] | shortcut markers (class `shortcut`, parent `marker`), 0..28 per race (103 races have none) |
| `BarrierExemptions` | key[] | markers; 6 races (the tollbooth ones) have one |
| `Barriers` | text[] | names of track-path barriers enabled for the race; a leading `*` marks a barrier flagged as a player barrier (drive-through), plus entries like `SCENERY_GROUP_DOOR` |
| `RaceLength` | float | metres of the route, stored by the editor (7486.84 for `10_2_1_sprint`; 0 for D-Day). The game recomputes it ([ai-racers-route.md](../specs/ai-racers-route.md)) |
| `NumLaps` | int | 1 (201 races), 2 (15), 3 (36), 4 (2), 10 (1) |
| `IsLoopingRace` | bool | True for the 53 circuit-type races; the laps then run through the same checkpoints |
| `IsMarkerRace` | bool | 15 races |
| `RaceTriggers`, `RandomSpawnTriggers`, `ZoneList`, `SpeedTrapList` | key[] | extra triggers: scripted traffic spawns, zones and the speed traps (4..10 per speed-trap race). A speed trap is a **sphere** (class `radiustrigger`: `Position`, `Radius` 20 to 30 m, `ThreshholdSpeed` 88 km/h in `1.5.3`), not a box **[verified]** |
| `Region` | text | `college`, `coastal`, `city` (110 / 85 / 60 races) |

Markers and triggers (`Position`, `Rotation`, `Dimensions`):

- `Position` is a `Vector3` in **map coordinates** `(x, y, z)` with z up. The game converts to physics space
  (x right, y up, z forward; [collision.md](collision.md#coordinate-space-verified)) as
  `world = (-y, z, x)` **[exe]**. Example: `startgrid` of `10_2_1_sprint` `(1511.82, 4404.87, 209.605)` is the
  world point `(-4404.87, 209.605, 1511.82)` **[verified]** (the reader's install test). The reader in
  `crates/nfsmw-data/src/gameplay` applies this conversion (`to_world`).
- `Rotation` is a heading in degrees. The unit heading in world space is `(-sin R, 0, cos R)` (a rotation about
  the up axis by `-R` degrees, applied to `(0, 0, 1)`) **[exe]**; the reader's `world_heading`. Check on the data: for 212 races with a first
  checkpoint over 100 m from the start, the cosine between that heading and the direction from the start to
  checkpoint 1 averages 0.81 (86 percent above 0.5); the opposite sign averages 0.08 **[verified]**. A marker
  lookup can add an extra rotation in degrees (used for `ForceStartPosition`).
- `Dimensions` (box triggers) are three extents. Over the 1,446 checkpoint-named box triggers **[verified]**: the
  first extent is 2 m (or 4 m or less) for 1,296; 143 are the 40 m cube kind `(40, 40, 40)`; 7 are other shapes.
  The `(2, W, 40)` kind has W 25..60 (`(2, 40, 40)` the most common). The first value is therefore the thin axis;
  that it lies along the heading is still **[unconfirmed]** (see open question 1). The race code reads the three values
  as **half** sizes: `checkpoint1` of `10.2.1` stands 22 m above the road with a stored height of 40, so only a half
  size (a box 80 m high) holds the road ([world-markers.md](../specs/world-markers.md) section 3). Finish lines have a first
  extent of 2 m in the samples read, with the second 25..65 m; `10_2_1_sprint` has `(3, 65, 5)`.
- Trigger flags: `OneShot` and `FireOnExit` are False on `trigger` and `boxtrigger`, and `FireOnExit` is True on
  `zonetrigger`; `ParticleEffect` is empty on `trigger` and `fxgame_flare_red` on `finishline`, with
  `FlareSpacing` 5 there. **[verified]**
- `Width` (the `wrongway` trigger, 150 m for `10_2_1_sprint`), `Directional`, `ResetsPlayer` describe the
  wrong-way reset trigger; `FireOnExit`, `OneShot`, `ParticleEffect` are trigger options (finish lines use
  `fxgame_flare_red` with `FlareSpacing 5`).

### 3.2 Start of the race

| Field | Meaning |
|---|---|
| `DoCountdown` | True for 217 races: the 3-2-1 countdown with the cars held (staging). False for the 38 rolling-start ones |
| `RollingStart` | True for 38 (boss, challenge and D-Day races): no countdown; every racer is placed already moving |
| `InitialPlayerSpeed` | 0 (217), 40 (36) or 60 (2). Used as the start speed when a saved racer speed is 0. The only reader falls back to it where the vehicle speed slot is 0, so it is in the vehicle layer's unit, metres per second (40 is about 144 km/h) **[verified]** |
| `StartTime`, `TOD` | time of day: `TOD -1` (245 races) leaves it unchanged; 1 (9) and 0.5 (1) set it. `StartTime 86` in the D-Day race |
| `StartPercent` | 0.6 in the D-Day race only; the HUD progress starts there (`pct = 100 * start + (1 - start) * pct`) **[exe]** |
| `PlayerCarType`, `PlayerCarPerformance` | a forced player car (177 races leave it empty) and its performance 0..1 (202 races: 1) |
| `TimeLimit` | seconds, 0 for 181 races; 13 races 50 s, 7 40 s, 7 55 s, 6 60 s, 6 30 s ... |
| `KnockoutsPerLap` | 1 for the 20 knockout races |
| `BustedLives` | player lives for being busted (1 in the templates) |

### 3.3 Opponents and difficulty

| Field | Meaning |
|---|---|
| `Opponents` | key[] of characters (section 4). Counts: 0 (108 races), 1 (45, the bosses), 2 (1), 3 (101, the street rivals) |
| `DifficultyLevel` | int. The game turns it into easy / medium / hard: below 34 easy, 34..66 medium, 67 and above hard **[exe]**. The quick race writes 33, 66 or 100. Data: 100 (250 races), 50 (4), 75 (1) |
| `CatchUp` | bool, True in all 255. The catch-up (rubber band) switch |
| `CatchUpOverride` | bool, False in all 255. When True the four fields below replace the built-in tables |
| `CatchUpSkill`, `CatchUpSpread` | **text**: a list of up to 11 floats separated by spaces or commas; `"0.2"` and `"1000"` in 235 and 234 races, `"1"` (17), `"0.5"` (3) and `"500"` (11), `"250"` (10) otherwise |
| `CatchUpIntegral`, `CatchUpDerivative` | float gains; 5e-5 everywhere; 0.001 (251 races) or 0.005 (4) |
| `CopsInRace`, `CopDensity`, `ScriptedCopsInRace`, `PursuitRace`, `UseWorldHeat`, `ForceHeatLevel`, `MaxHeatLevel`, `CopSpawnType`, `CopSpawnPoints` | police options; see the pursuit specs. `CopsInRace` is True for 108 races, `PursuitRace` for 37 |
| `TrafficLevel`, `ForceTrafficDensity`, `TrafficPattern` | traffic density 0..100 percent (13 distinct values, 100 for 59 races, 30 for 31, 80 for 27 ...); `TrafficPattern "drag"` for drag races. `StartRace` reads `ForceTrafficDensity` (attribute `0x5E161BBA`, an int): above 0 it sets the density, and a drag race sets 0 **[exe]** |
| `RankPlayersByPoints`, `RankPlayersByDistance` | how positions are ordered ([ai-racers-route.md](../specs/ai-racers-route.md)): points for the 32 speed-trap races, otherwise laps then progress |

The shipped data never switches `CatchUpOverride` on, so the `CatchUpSkill`, `CatchUpSpread`,
`CatchUpIntegral` and `CatchUpDerivative` values are parsed but not used by the formula
([ai-racers-catchup.md](../specs/ai-racers-catchup.md)) **[exe + verified]**.

### 3.4 Shortcut markers

Class `shortcut` (parent `marker`) has `Position`, `Rotation` and two floats, `ShortcutMinChance` and
`ShortcutMaxChance` (template default 0 and 1). Over the 1,100 shortcut markers of the races **[verified]**:
(0, 0) 354 times (a shortcut that racers never take), (0, 1) 135, (0.8, 0.9) 104, (0.2, 0.3) 53, (0.3, 0.4) 44,
(0.4, 0.5) 42, (80, 90) 18 and (10, 20) 15 (values above 1 are percentages), (1, 1) 13, 44 distinct pairs in all.

## 4. Characters (the opponent drivers) **[verified]**

Class `gameplay`, parents `character` (294 collections), `boss` (19) and `character_smart` (a generic
character used by quick races). 314 collections in all. Fields:

| Field | Meaning |
|---|---|
| `SkillLevel` | int 0..100; base AI skill in career is `SkillLevel / 100`. Default 100 (traffic and scripted characters); `character_smart` 0 |
| `MinimumAIPerformance` | float 0..1: how close to the player's car ratings this driver's car may get (0 = always matched down to the weakest player stat) |
| `PresetRide` | text: a front-end preset car (name hashed upper-case, found in the front-end preset data, not in `gameplay`; the `presetride` class holds only upgrade levels), 49 uses among the opponents |
| `CarType`, `CarTypeLowMem` | text: a `pvehicle` collection name; 3 opponents use it, many traffic characters do |
| `IsBoss` | bool (True for the 19 `boss` collections and 1 more) |
| `RacerName` | text, empty in this install (names come from the front-end) |
| `ForceStartPosition` | key of a marker: this racer starts there instead of in the grid (10 of the 350 opponent slots) |
| `AllowInvisibleSpawn`, `ForcePreload` | spawn options, False |

Street rivals `race_bin_NN/character1..3` (bins 01..14) use the same numbers for all three: **(SkillLevel,
MinimumAIPerformance)** by bin: 01 (80, 0.8), 02 (80, 0.8), 03 (75, 0.7), 04 (70, 0.6), 05 (60, 0.6),
06 (55, 0.5), 07 (50, 0.5), 08 (45, 0.4), 09 (40, 0.4), 10 (35, 0.3), 11 (25, 0.2), 12 (20, 0.1), 13 (15, 0.05),
14 (10, 0). None has a car: the game picks a stock car (section 5). Blacklist bosses `race_bin_NN/<name>`:

| Bin | 01 | 02 | 03 | 04 | 05 | 06 | 07 | 08 | 09 | 10 | 11 | 12 | 13 | 14 | 15 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| SkillLevel | 80 | 80 | 80 | 75 | 65 | 60 | 55 | 50 | 45 | 40 | 30 | 25 | 20 | 15 | 0 |
| MinimumAIPerformance | 0.9 | 0.8 | 0.7 | 0.6 | 0.6 | 0.5 | 0.5 | 0.4 | 0.4 | 0.3 | 0.2 | 0.1 | 0.05 | 0 | 0 |

The bosses carry a `PresetRide` (`E3_DEMO_BMW` for bin 01, `BL2` .. `BL15` for the others). The story
races use their own characters (`16_1_0_partial_dday/razor`, skill 20; `.../toru_sato` and `.../ronnie_mccrae`,
skill 0; unnamed `opponent`s with `OPM_*` presets).

## 5. How a race is created at run time (the data side) **[exe + decomp]**

- The front end or a quick race builds a `GRaceCustom` over the chosen race: it can override the lap count,
  the traffic density (clamped to 0..100), the difficulty (33 / 66 / 100), the number of opponents, the
  direction and the catch-up flag; the front end's quick-race traffic setting maps off / light / medium /
  heavy to 0 / 10 / 30 / 50 and the cop setting to the heat level.
- `GetOpponentChar(i)`: in a quick race, a **boss race uses the generic `character_smart`** instead of the boss
  (so quick-race opponents never carry the boss car or the high skill); otherwise the i-th entry of
  `Opponents`. `GetNumOpponents` is the length of `Opponents` unless the custom race sets a count.
- The Lua state graphs of `gpcore` (`basicrace` and friends) drive the event: they call `InitRacers`,
  `SetAllStaging`, `StartRaceTimers`, `StartRace`, `NotifyCheckpointReached`, `SetRacerGoal`,
  `KnockoutRacer`, `NotifyRaceFinished` and `RestoreStartPositions` on the engine. The bytecode is not
  decompiled; the engine side of these calls is in the specs.

## 5b. Fields the race run reads (`RaceParams`, `Character`, `SphereTrigger`) **[verified]**

`crates/nfsmw-data/src/gameplay/params.rs` reads, per race: `DoCountdown`, `RollingStart`, `InitialPlayerSpeed`,
`IsLoopingRace`, `KnockoutsPerLap`, `TimeLimit`, `OvertimePenaltyPerSec`, `DifficultyLevel`, `TrafficLevel`,
`ForceTrafficDensity`, `ForceHeatLevel` (attribute `0xE4211F4F`, the heat the racers start at, read by `StartRace`
**[exe]**), `CopsInRace`, `PursuitRace`, `CatchUp`, `RankPlayersByPoints`, `TrafficPattern`, `Opponents`,
`SpeedTrapList`, `Shortcuts` and `CashValue`. The install test checks the counts of section 3 (38 rolling starts, 217
countdown races, 53 looping races, 20 knockout races, 32 ranked by points, 11 drag traffic patterns, 108 races
without opponents and 101 with three). Per opponent: `SkillLevel`, `MinimumAIPerformance`, `PresetRide`, `CarType`,
`IsBoss`, `ForceStartPosition`. Per box trigger: `TimeBonus` (an `Int32`) and `ResetsPlayer`; per sphere trigger
(`radiustrigger`): `Position`, `Radius`, `ThreshholdSpeed`.

## 6. Reading the gameplay class (`crates/nfsmw-data/src/gameplay`) **[verified]**

- `gameplay::load(&GameDir)` reads `GLOBAL/ATTRIBUTES.BIN` (the class) and `GLOBAL/gameplay.bin` (the
  collections) through `blackbox-attrib`, and returns `GameplayData { races, triggers, markers }`.
  `GameplayData::trigger(key)` resolves a race's `racefinish` or `Checkpoint[]` key to a box trigger.
- A `GCollectionKey` field has type hash `0x6CD4B70A` and is stored as 4 raw bytes: the little-endian `vlt_hash`
  of the target collection's full name (for example `race_bin_10/10_2_1_sprint/startgrid`). All checkpoint and
  finish keys of the install resolve to a loaded box trigger.
- Races: non-template collections with a `racestart` (255). Box triggers: non-template collections that inherit
  `boxtrigger` (2,332, of which 87 are named `*/finishline` and 1,446 `*/checkpoint*`; the rest include
  `ai_cp*` and `time_bonus_checkpoint*` triggers of the traffic and challenge races).
- Race icons: `p2p` 51, `tollbooth` 68, `challenge` 36, `circuit` 32, `speedtrap` 31, `knockout` 20, `drag` 11,
  empty 6. Races with a finish 217, with checkpoints 216, every race has a start.

## 7. Activity zones: auto shops and safe houses **[verified]** unless marked

Zones are sphere triggers, not boxes. Each zone collection inherits `zonetrigger` (`Radius`, `FireOnExit` True,
`ZoneType`) and sets its own `Position`. They are grouped by a collection with a `ZoneList`:

| Group | `ZoneType` | Zones read | Notes |
|---|---|---|---|
| `menu_gate_customshop` | `customshop` (the auto shop) | `zone_customshop_1` .. `_8` (8) | `ZoneList` names all 8 |
| `menu_gate_safehouse` | `safehouse` | `zone_safehouse_1`, `_2`, `_4`, `_5` (4) | no `_3` in the install; `ZoneList` names 4 |
| `menu_gate_carlot` | `carlot` | `zone_carlot_1` .. `_3` (3) | read as `ActivityKind::CarLot`; a map blip only (its zones are not entered yet) |

Per zone:

- `Position` (map coordinates, converted to physics space) and `Radius` (metres, for example 6 or 7).
- `ZoneType` is inherited from `zone_customshop` / `zone_safehouse`. The strings `customshop` and `safehouse` and
  the attribute name `ZoneType` are present in `speed.exe` **[exe]** (strings found by a Ghidra string search;
  the code that reads them was not decompiled).
- A child `respawn` marker: a `marker` with `Position` and `Rotation`. Its key is in the zone's field `0x2241F4CD`
  (a `GCollectionKey`, the zone's `Children[0]`; the field's name is not known, so the key is read by its hash).
- Field `0x6CE23062` (`Int32`): 8, 12, 15 or 16 across the 12 zones (shops 8 x1, 12 x4, 15 x3; safe houses 8 x1,
  12 x2, 16 x1) **[verified]**. Its meaning is **[unconfirmed]**. The safe-house map icon is the format string
  `SAFEHOUSEINDICATOR_%d` in `speed.exe`; the index that goes with it is not yet tied to this field.
- Field `0x883C65E3` (`Bool`): True on the group collections, not read.

Map icons of the race types are the `MMICON_<TYPE>_%d` strings in `speed.exe` (`CHECKPOINT`, `SPRINT`, `DRAG`,
`CIRCUIT`, `KNOCKOUT`, `TOLLBOOTH`, `SPEEDTRAP`, `AIRACER`, `COPCAR`, `ROADBLOCK`) **[exe]**, strings only.

## 8. Open questions

1. The trigger test: which extent lies along the heading (the data says the first extent is the thin one, see
   section 3.1), and whether a checkpoint is crossed by the car centre or any part of the car. The trigger code
   (`EGTriggerInternal` in `speed.exe`, the box test) has no symbols in the Ghidra project: every function near
   the strings is `FUN_*`. A Ghidra pass over the box-containment routine is needed; until then the trigger test
   is **[unconfirmed]**.
   *Status:* the rewrite's working rule is in [world-markers.md](../specs/world-markers.md) section 3 (the first extent
   along the heading, fitted to the data, not read from the code). The race code reads the stored values as half
   sizes (measured: the first checkpoint of `10.2.1` stands 22 m above its road); no entry zone uses a box.
2. The unit of `InitialPlayerSpeed` (m/s assumed).
3. The Lua state graphs (what ends a knockout lap, when the countdown starts the engines): decompile the
   bytecode (Lua 5.0 chunk) and describe them.
4. `Barriers` entry syntax (`*` prefix) beyond "player barrier flag" and how `SCENERY_GROUP_DOOR` entries act.
5. The meaning of the zone field `0x6CE23062` (Int32, 8..16) and of `0x883C65E3` (Bool), and the field name of
   the respawn key `0x2241F4CD`. The field names are not in the database; they are matched by their hashes only.
6. Whether the auto shop and safe house map icons are chosen by `ZoneType` alone or by the zone's index.
