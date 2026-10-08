# AI pursuit: heat, the `pursuitlevels` table, bounty, infractions and the HUD values

What the "heat" number is, how it grows and is reset, what every field of the `pursuitlevels` class does and which
heat levels use which values, how a pursuit earns bounty (rep points) and accumulates cost to state, which infractions
exist, what happens to the career after a bust or an escape, and which values the in-pursuit HUD reads. The pursuit
object itself is in [ai-pursuit.md](ai-pursuit.md); cops, spawning and support in [ai-pursuit-cops.md](ai-pursuit-cops.md).

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled), under
  `src/Speed/Indep/Src`: `AI/Common/AIPursuit.cpp`, `AI/Common/AIVehicle.cpp` (`AIPerpVehicle`, `AIVehicleHuman`
  destructor), `AI/Activities/AICopManager.cpp`, `Gameplay/GInfractionManager.h`, `Gameplay/GRaceStatus.h`,
  `Frontend/Database/VehicleDB.cpp` (`FECareerRecord`, `FEImpoundData`, `FEInfractionsData`),
  `Frontend/MenuScreens/Safehouse/career/uiInfractions.cpp`, `Frontend/HUD/Fe{HeatMeter,PursuitBoard,CostToState,Infractions,Reputation,RadarDetector}.cpp`,
  `Speech/SoundAI.cpp`, `Generated/AttribSys/Classes/{pursuitlevels,pursuitsupport,pursuitescalation,infractions,aivehicle}.h`.
  Read for understanding; no code copied.
- **Data inputs:** AttribSys `pursuitescalation`, `pursuitlevels` (57 fields, 21 collections), `pursuitsupport` (4 fields,
  21), `infractions` (1 field, 8), `aivehicle` (cop collections), `gameplay` (race bins). Tables below are **[verified]**
  against the install (`GLOBAL/attributes.bin`, `gameplay.bin`) unless tagged otherwise.

## 1. What heat is

- Heat is a **float** on the perpetrator, normally 1.0 to 10.0. Its integer part is the *heat level* that picks the
  table row; the fractional part is shown by the HUD heat gauge. **[decomp]**
- A perpetrator is created with heat 1.0. AI racers get `max(race heat, 1)` when a race is prepared. The player's heat
  at the start of a session comes from the game flow (race parameters or the career car's stored heat); that loading
  code is not in the decompiled sources read. **[unconfirmed]** When the player's car object is destroyed (leaving
  the world) the heat, capped at 5, is written back to the career car record. **[decomp]**
- The collection rows that heat selects are `pursuitescalation` (ai-pursuit.md section 2): `heattable[level-1]` outside
  races, `racetable[level-1]` during a race. The ten `heattable` rows (levels 1 to 10) are the collections named by
  hash `0xE1A5CE21, 0x6F9CE5C1, 0x5C9DA40B, 0x7D4A777E, 0x59B2CAEF, 0xB8F1F245, 0xF3E10B20, 0x8D5ED471, 0x5CAE20A1,
  0xED01800F`. **[verified]** Levels 8 to 10 are not used by normal career play (they exist for the "Most Wanted"
  scripted pursuits, challenge series and the opening pursuit; the race-bin table caps normal heat at 5).

## 2. The 57 fields of `pursuitlevels`

Values per heat level 1 to 10. "Consumer" names the system that reads it. Fields read by the tactics, helicopter or
road-network code are only listed. **[verified]** values, **[decomp]** meanings.

### 2.1 Cop counts and timing

| Field | Values (heat 1..10) | Meaning / consumer |
|---|---|---|
| `cops` (up to 3 records of `CopType`, `Count`, `Chance`) | table in section 2.7 | which cop cars make up the wave, how many of each, and the spawn weight (percent); `copheli` is the helicopter record and is not counted in the car total |
| `FullEngagementCopCount` | 5, 10, 15, 20, 25, 30, 15, 80, 20, 100 | cops that must be "evaded" (left behind or lost) before the wave is over; copied to `NumCopsRequiredToEvade` |
| `FullEngagementRadius` | 50 x7, 200, 50, 200 m | a cop within this distance counts as "fully engaged" (HUD cop count, evaded-cop count) |
| `NumCopsToTriggerBackup` | 2 x7, 3, 5, 5 | when the remaining required cops drop to this number, the backup countdown starts |
| `BackupCallTimer` | 180, 120, 120, 120, 90, 90, 90, 60, 45, 60 s | length of the backup countdown (HUD "backup in"; ends by re-locking the wave) |
| `NumPatrolCars` | 1 x7, 2, 8, 2 | patrol cars in free roam (cop manager); also the cop count wanted during cool-down |
| `TimeBetweenFirstFourSpawn` | 40, 30, 20, 15, 10 x5 s | wait after a cop joins while fewer than 3 cops have been involved |
| `TimeBetweenCopSpawn` | 10, 10, 5, 5, 2, 5, 2, 2, 2, 2 s | wait after a cop joins once 3 or more have been involved; also the cap when cool-down starts |
| `TimeBetweenHeliActive` | 0, 0, 120, 180, 180, 180, 180, 120, 60, 5 s | delay before a new helicopter after one leaves |
| `SearchModeHeliSpawnChance` | 0, 0, 0, 2, 4, 5, 0, 0, 0, 0 % | chance to send a helicopter when cool-down starts and nobody sees the player |
| `HeliFuelTime` | 10, 10, 10, 60, 90, 180, 10, 75, 120, 400 s | helicopter flight time (helicopter code) |

### 2.2 Sight, busted and evade

| Field | Values | Meaning / consumer |
|---|---|---|
| `frontLOSdistance` | 151 x7, 201 x3 m | cop sight range ahead (and in all directions when equal to `rearLOSdistance`) |
| `rearLOSdistance` | 151 x7, 101 x3 m | sight range behind the cop; race rows 51/101 |
| `heliLOSdistance` | 251 x7, 301 x3 m | helicopter sight range (helicopter code) |
| `BustSpeed` | 9, 12, 19, 20, 25, 25, 19, 5, 5, 5 km/h | speed below which the busted bar fills |
| `MeterDeadZoneBustedDistance` | 35 m | start of the busted-meter ramp |
| `MeterDeadZoneEvadeDist` | 50 m | start of the "far from cops" negative ramp |
| `evadetimeout` | 20, 45, 75, 90, 120, 120, 120, 220, 60, 220 s | unseen time required to evade |
| `HiddenZoneTimeMultiplier` | 4, 5, 6, 7, 8, 8, 8, 4, 2, 4 | evade progress speed-up inside a hiding spot |
| `TimeToHideInZone` | 1 x5, 3 x5 | not read by any code in the sources (the latch is hard-coded) |
| `SpeedReactionTime` | 1.75, 1.5, 0.75, 0.5, 0.1, 0.1, 0.5, 0, 0, 0 s | cop reaction delay to the player's speed changes (cop steering code) |
| `SearchModeCityMPH`, `SearchModeHwyMPH` | 75 x7 / 55 x3 mph; 100 mph | in the *default* collection (50 / 71 mph) these are the patrol cruise speeds of a cop car on the city / highway lanes (`AIActionTraffic`) |

### 2.3 Heat growth and career adjustments

| Field | Values | Meaning |
|---|---|---|
| `TimePerHeatLevel` | 1, 1, 1, 1, 1, 600, 600, 780, 820, 2000 s | base seconds to climb one level; levels 1 to 5 are multiplied by `ScaleEscalationPerBucket` |
| `ScaleEscalationPerBucket` (15 floats, index = career bin 0 to 14) | level 1: 120 for index 0 to 10, then 180, 240, 360, 600; level 2: 120 for 0 to 6, then 180, 240, 240, 360, 480, 600, 600, 600; level 3: 120 for 0 to 4, then 180, 240, 300, 600 x7; level 4: 120, 240, 360, 480, 600 x11; level 5: 600 x15; levels 6 to 10: 1 x15 | multiplier of `TimePerHeatLevel` by the career race bin; see section 3.2 |
| `EventWinHeatAdjust` | 0.9 (race rows 0.9 / 0.94) | **not used from the level rows**; the career uses the `default` collection (0.94) |
| `MilestoneCompleteHeatAdjust` | 1 x7, 0 x3 | same: `default` = 0.93 |
| `EvadeSuccessHeatAdjust` | 0.9 | same: `default` = 0.95 |

### 2.4 Bounty, 911 and rep

| Field | Values | Meaning |
|---|---|---|
| `RepPointsPerMinute` | 100, 500, 1000, 5000, 25000, 30000, 30000, 5000, 10, 5000 | rep added every 10 s of chase (section 5.1) |
| `DestroyCopBonusTime` | 10 x7, 120, 10, 120 s | window for the destroyed-cop multiplier (section 5.2) |
| `CTSFor911` | 400, 600, 800, 1000, 1500, 1750 x5 | damage cost that makes a 911 call (free roam) |
| `NumCiviHitsFor911` | 3, 4, 5, 6, 7, 5 x5 | hard traffic hits that make a 911 call |
| `Lifetime911` | 45 x5, 180 x5 s | how long a 911 call keeps the player "wanted" |
| `TimeInactiveFor911` | 120 s | cop lockout after an escape or restart in free roam |

### 2.5 Roadblocks (read by the pursuit and the cop manager; layouts in the tactics spec)

| Field | Values | Meaning |
|---|---|---|
| `roadblockprobability` | 0, 10, 25, 30, 40, 50, 35, 50, 100, 80 % | chance per request while the player is in sight |
| `roadblockspikechance` | 0, 0, 0, 70, 80, 85, 85, 0, 100, 0 % | chance a roadblock has spike strips (never during cool-down) |
| `roadblockhelichance` | 0 | flag for the speech system only |
| `SearchModeRoadblockRadius` | 1000, 1000, 2000, 5000, 10000, 10000, 10000, 3500, 4000, 5000 m | scale of the "searching" roadblock chance |
| `SearchModeRoadblockChance` | 0, 3, 5, 7, 10, 12, 12, 60, 0, 80 % | roadblock chance when the player is not in sight, scaled by `(radius - d) / radius`, `d` = last known position to target |

### 2.6 Formations and collapse (summary; detail in [ai-pursuit-formations.md](ai-pursuit-formations.md))

`CopFormations` (up to 10 records of formation, duration, frequency), `formations` (20 `u32`, not read),
`StaggerFormationTime` (7, 6, 5, 4, 3, 5, 4, 10, 7, 10 s), `BoxinTightness` / `RollingBlockTightness` (0.5, 0.6, 0.8,
0.9, 1, 1, 1, 0.5 x3), `BoxinDuration` / `RollingBlockDuration` (4, 5, 6, 7, 8, 10, 10, 3 x3 s), `MaxCopsCollapsing`
(4, 5, 6, 7, 8, 8, 9, 6, 6, 6), `CollapseInnerRadius` (6, 5, 4, 3, 3, 3, 4, 3, 2, 3 m), `CollapseOuterRadius` (15, 15, 8,
5, 5, 7, 10, 15, 5, 15 m), `CollapseSpeed` (58, 68, 78, 88, 96, 120, 78, 15, 1, 15 km/h), `CollapseAggression` (0.4, 0.5,
0.6, 0.7, 0.8, 0.8, 0.5, 0, 0, 0; read by the ram action). Siren timing for the speech/siren system: `SirenWailPeriod` 11,
`SirenInitMinPeriod` 0.5, `SirenInitVariation` 1.5, `SirenScreamPeriod` 60, `SirenMaxScreamTime` 3, `SirenMaxYelpTime` 6 (all
levels).

### 2.7 Cop composition per heat level (`cops`)

Internal car names (hash of the lower-case name, **[verified]**); `x N @ chance%`.

| Heat | Cars | Heli |
|---|---|---|
| 1 | `copmidsize` x4 @100 | none |
| 2 | `copghost` x5 @100 | none |
| 3 | `copgto` x7 @100 | none |
| 4 | `copgtoghost` x8 @100 | `copheli` x1 @50 |
| 5 | `copsporthench` x10 @50 | `copheli` x1 @60 |
| 6 | `copsportghost` x8 @80, `copsuvpatrol` x2 @80 | `copheli` x1 @50 |
| 7 | `copsuvpatrol` x6 @90 | none |
| 8 | `copsportghost` x8 @100 | `copheli` x1 @60 |
| 9 | `copmidsize` x4 @100 | `copheli` x1 @80 |
| 10 | `copsport` x1 @90 | `copheli` x1 @100 |

Race rows (`racetable`, used while racing): heat 1 `copmidsize` x2; 2 `copghost` x3; 3 `copgto` x4; 4 `copgtoghost` x5 +
heli; 5 `copsporthench` x5 + heli; 6 `copsportghost` x6 + heli; 7 `copsuvpatrol` x8 + heli; 8 to 10 `copghost` x8 + heli.
The rhino-class SUVs and the cross car also appear through the *support* table, not here (ai-pursuit-cops.md section 6).

## 3. How the heat changes

### 3.1 Limits: base and maximum heat of a pursuit

Set when a pursuit is created: **[decomp]**

| Context | Base heat | Maximum heat |
|---|---|---|
| Career, final epic pursuit | 6 | 6 |
| Career (free roam, or any event) | race bin `BaseOpenWorldHeat` | race bin `MaxOpenWorldHeat` |
| Quick race that is not a challenge | 1 | 5 |
| Challenge race, other | race bin values, then optionally the race's own limits | |

The *race bin* is the career rank bucket. **[verified]** (`gameplay.bin`): bins 15 and 16 are the early game
(`race_bin_15`: base 1, max 2; `race_bin_16`: base 1, max 1); bins 14 to 9 cap at 2 or 3 (14, 13: 2; 12, 11, 10, 9: 3); bins
8 to 5 cap at 4 (8, 7, 6, 5); bins 4 to 1 cap at 5; `race_bin_challenge` caps at 7; `race_bin_opm` at 10; the base is 0 for
all bins except 15 and 16. A race with `UseWorldHeat` sets base to the race's `ForceHeatLevel`, max to its `MaxRaceHeatLevel`
(field present in the code, absent from the class in this install) and scale to 1; otherwise a race's own `MaxHeatLevel`
may lower the maximum. The heat is also clamped to `[base, max]` on every update.

### 3.2 Growth while chased

Each pursuit update, outside cool-down: **[decomp]**

```
tphl = TimePerHeatLevel of the current heat row
if a career profile exists:
    bin = 14 if challenge race else min(current_bin, 14);   tphl *= ScaleEscalationPerBucket[bin]
heat = clamp(heat + dt / tphl, base_heat, max_heat)
```

The bin counts down through the career (15 at the start, 1 near the end), so a level-1 chase gains 1.0 heat per 600 s in
bins 15 and 14, 360 s in bin 13, 240 s in 12, 180 s in 11 and 120 s from bin 10 on; level 4 takes 600 s down to bin 4 and
240 s in bin 1; level 5 always takes 600 s; levels 6 to 10 take 600, 600, 780, 820, 2000 s. The bin's maximum heat stops
the growth (bins 15 and 14 end at heat 2, so early in the career the heat never leaves levels 1 and 2). Whenever the integer part changes, the pursuit resets `ActiveFormationTime`, the support
priority check and re-reads `RepPointsPerMinute`.

### 3.3 Jumps and resets

- **Pursuit start:** `heat = max(heat, base_heat)`.
- **Forced start** (`MForcePursuitStart`/`PursueAtHeatLevel(n)`): if heat < n, heat = n and the wave is re-locked.
- **Escape (player):** every car of the career stable gets `VehicleHeat *= EvadeSuccessHeatAdjust` from the *default*
  collection = **0.95**; and `TimesBusted` handling of section 6. **[decomp + verified]**
- **Event win:** stable heat x `EventWinHeatAdjust` (default 0.94); **milestone complete:** x 0.93. **[decomp + verified]**
- **Paying the bust fine:** the car's heat is set to 1.0. A quick-race car select sets the car's heat to 0.
- A new decal/paint on a car multiplies its heat by `fecooling.NewDecal * extra` (class `fecooling`, one collection;
  values not read here). **[decomp]**
- Heat is not reduced during a chase. There is no decay of the perpetrator's heat except the above.
- Speech/music hear the heat crossing 2, 3, 4, 5 (ai-pursuit-speech.md section 5).

## 4. Infractions

`GInfractionManager` keeps a bit set of infractions for the current pursuit and a count. The bits: **[decomp]**

| Bit | Name | `infractions` fine **[verified]** | Raised by |
|---|---|---|---|
| 1 | speeding | 150 | not in the sources read (stripped) |
| 2 | racing | 350 | not in the sources read |
| 4 | reckless driving | 1000 | not in the sources read |
| 8 | assault (on police) | 350 | a deliberate hit on a cop in the player's pursuit that damages it for the first time |
| 16 | hit and run | 300 | a deliberate hit on a traffic car (closing speed > 4 m/s) while a cop sees the player and the nearest cop is under 25 m, in the player's pursuit |
| 32 | damage to property | 100 | a deliberate, direct hit on a root smackable object under the same conditions |
| 64 | resisting arrest | 300 | the moment the pursuit enters cool-down (the player got away from sight) |
| 128 | off road | 75 | the player is not on a legal road while in sight and the nearest cop is under 25 m |

The manager is cleared when a player pursuit starts (`PursuitStarted`). The HUD shows each new infraction as a line
(four slots) with a running count, through a radio-detector-style group; the text strings are front-end data. The
detection thresholds for speeding, racing and reckless driving (`GetSpeedLimit`, `GetRacingSpeedLimit`,
`GetRecklessSpeedThreshold`) are declared but empty in the decomp. **[unconfirmed]** The cop-side "pursuit type" text
(speeding, hit-and-run, reckless, rammed) is picked by the speech system from the last infraction message
(`MMiscSound` on channel `Infraction`).

## 5. Bounty, rep points and cost to state

### 5.1 Pending rep points (the HUD "bounty")

Two counters on the perpetrator, accumulated only in free roam or challenge races: **[decomp]**

- **normal**: every time the pursuit time crosses a multiple of 10 s while the player is in sight and not busted, add
  `RepPointsPerMinute` of the current row (the name says minute; the code step is 10 s, so the effective rate is six
  times the field per minute). Example, heat 5: 25 000 points per 10 s.
- **cop destruction**: when a cop of the chase is destroyed (a cop that was destroyed *and* removed from the chase), add
  `RepPointsForDestroying[heat]` of the cop's `aivehicle` collection times a multiplier. Values (all heat levels equal):
  `copmidsize` 250, `copghost` 500, `copgto` 2500, `copgtoghost` 5000, `copsporthench` 20 000, `copsportghost` 25 000,
  `copsport` 20 000 (100 000 at heat 10), `copsuvl` 10 000, `copsuv` 15 000, `copsuvpatrol` 25 000, `copcross` 100 000,
  `copcompact` 1..10. **[verified]** The array is indexed by the integer heat (10 entries).
- **multiplier**: the first kill uses x1; a further kill while `DestroyCopBonusTimer > 0` raises the multiplier by 1
  to a maximum of 3; each kill restarts the timer at `DestroyCopBonusTime`; after the timer runs out the multiplier is back to 1.
  `GetCopDestroyedBonusMultiplier`, `GetMostRecentCopDestroyedRepPoints` and `...Type` feed the HUD pop-up ("cop X taken out,
  +N") which names the cop type and prints `rep x multiplier`. **[decomp]**

Pending points are shown on the pursuit board's summary line and added to the career at the pursuit's end (5.4).

### 5.2 Cost to state

`AddCostToState(cost)` adds to the perpetrator's `CostToState` and to the pursuit's property-damage value, only when the
race context is Career (or a challenge race) and a pursuit exists. Sources of cost: **[decomp]**

- a collision where the perpetrator caused it (the chain must be at most 2.0 s old) with a smackable: the object's
  `COST_TO_STATE` attribute;
- an explosion the perpetrator caused: the object's cost, or 2000 for a cop car;
- a hit on a non-destroyed cop: 2000 if direct and deliberate, 500 if indirect, scaled by `ramp(closing speed,
  4 m/s, 30 m/s)` in steps of 50, at least 50.

The HUD cost-to-state element shows the running total for 120 frames after each increase and hides in between; it resets
to 0 when the value is set to 0. The total is also one input of the 911 logic (section 5.3).

### 5.3 911 calls (free roam)

While the speech system sees no active pursuit, it sums the `COST_TO_STATE` of smackables the player hits (`CTS911`) and
counts traffic hits with intensity at least `MinIntensityTrafficSmash` (0.5) (`TrafficHits911`). When `CTS911 >= CTSFor911`
or hits `>= NumCiviHitsFor911` in free roam, with cops enabled and no 911 already active, the dispatcher speaks a
"911 report" (speech spec). When that line has been said the game: sets the 911-active flag, sets the perpetrator's 911
timer to `Lifetime911`, and clears the cop lockout so patrol cars can spawn. A patrol cop that sees a car with a running 911
timer starts a pursuit regardless of speed (ai-pursuit.md section 4.2). Both counters reset whenever a pursuit is active or
searching.

### 5.4 Totals at the end

`CalcTotalCostToState` (the post-pursuit summary value) = 5000 x cops destroyed + 2000 x helicopters spawned + 500 x
roadblocks deployed + 250 x cops damaged + 500 x traffic cars hit + 250 x spike strips deployed + 225 x helicopter spike
strips + 250 x cop cars deployed + 450 x support vehicles deployed + property damage value. **[decomp]**
At the end of a *career* pursuit the manager moves the pending points (normal + destruction) to the car's career record
(`Bounty`), stores the infraction bits, and, if the player escaped, raises the displayed career rep and posts the new
bounty. Challenge-series races skip this. Evaded and busted pursuits are tallied (`NumEvadedPursuits`, `NumBustedPursuits`)
and ranked in the high-score tables (pursuit length, bounty, infractions, cops destroyed...). **[decomp]**

## 6. Career consequences

- **Evade (player):** all stable cars' heat x0.95; for every car not impounded `EvadeCount++` and, when it exceeds 2, one
  strike (`TimesBusted`) is removed (never below 0) and the count restarts; an impounded car's release timer may
  complete (`NotifyWin`). The pursuit counter "in a row" increments (milestone tracking).
- **Busted (player, free roam):** after the arrest scene the front end shows the *infractions* screen: the fine is
  `sum(count_i x amount_i)` over the car's unserved infractions. Paying with cash: spend the money, `TimesBusted++` and
  if it reaches `MaxBusted` (3 by default, raisable) the car is impounded (strike limit); serve all infractions; car heat
  = 1.0. Paying with a "get out of jail" marker uses one marker and waives the infractions without a strike. Not enough
  cash also impounds the car (reason "insufficient funds"). Impound: `TimesBusted = MaxBusted`, release after 5 event
  wins (`DaysBeforeRelease`).
- A busted AI racer is marked busted for its race result.

## 7. Values the HUD reads

**[decomp]** for the element code; the feeder that copies pursuit values into the HUD is not in the sources read, so the
mapping below is the natural one, **[unconfirmed]**:

| HUD value | Source |
|---|---|
| in pursuit flag | a pursuit exists for the player |
| pursuit time `m:ss` | `GetPursuitDuration` |
| busted bar | `TimeUntilBusted`, `IsPerpBusted` (bar segments, section 4.4 of ai-pursuit.md; value 1 shows "BUSTED!") |
| cool-down bar | `1 - CoolDownTimeRemaining / CoolDownTimeRequired` (given `required - 7`); shown when `TimeUntilBusted <= -1` |
| hiding backing | time-until-hidden > 0 |
| cops engaged | `GetNumCopsFullyEngaged` minus 1 if a helicopter is involved |
| cops taken out, cops damaged | `GetNumCopsDestroyed`, `GetNumCopsDamaged` (a pop-up per kill, ding on damage) |
| backup timer | `GetBackupETA` (> 0; styled urgent at 10 s or less) |
| bounty summary | pending normal + destruction rep points |
| heat gauge | `heat` while a pursuit runs, else the car's stored heat; `x{int}` label; two half arcs |
| cost to state | `CostToState` |
| infractions list | section 4 |
| radar detector | arrow to the nearest cop or speed camera, LED bar length from the range, a click sound per sweep; hidden while in a pursuit unless cooling down |

## Rust implementation notes

- Model `Heat { value: f32 }` on the perpetrator with `level() = value as i32` and recompute the three table rows when it changes.
  Read the tables from AttribSys by name (`pursuitescalation/default` then the referenced collections); do not hard-code them.
- Keep `base_heat` / `max_heat` in a `PursuitLimits` built from the game mode and the career bin; the career bin needs the
  campaign state (milestone 7+ career data).
- The career side (`VehicleHeat`, `TimesBusted`, impound) belongs to the save/career layer; the pursuit crate only reports
  `Evaded` / `Busted` with the bounty and the infraction mask.
- HUD fields needed: `in_pursuit`, `duration`, `time_until_busted`, `busted`, `evade_level`, `cooldown_remaining`,
  `cooldown_required`, `backup_eta`, `cops_engaged/destroyed/damaged/heli`, `pending_rep`, `last_kill_rep_and_type`, `heat`,
  `vehicle_heat`, `cost_to_state`, `infraction_text`, radar range/direction/type.

## How to check it

Free roam, career: log heat every 10 s during a chase and compare with `1 / (TimePerHeatLevel x scale[bin])` per second; kill
a cop and a second within 10 s and compare the pop-up numbers with section 5.1; evade and compare the car's heat in the
car select screen (x0.95); get busted three times and confirm the impound.

## Open questions

- Where the player's starting heat is loaded and how `RacePreparationInfo.HeatLevel` is filled.
- Detection rules for speeding, racing and reckless driving infractions (stripped from the decomp).
- Whether `RepPointsPerMinute` is truly added every 10 s (`kSecondsPerRepUpdate`) as the code reads.
- The `fecooling` values and the rule that applies `NewDecal`.
- The HUD feeder (pursuit values to elements) and the radar detector's target source.
