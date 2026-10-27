# Chameleon drift: what a drift is and what it pays in NFS: Most Wanted (2012)

In the 2012 game a drift fills the nitrous tank ("Earn Nitrous by drifting" is a line of its text tables). This page holds what
the install's data says about it: when the physics calls a slide a drift, what a drift pays, and what the HUD says. The
rule built from it is `chameleon_vehicle::DriftModel`; the setting that turns it on in the 2005 game is "2012 drift nitrous"
([nitrous-variants.md](../modding/nitrous-variants.md) has the other nitrous option, [api-world.md](../modding/api-world.md#vehicles-and-the-player)
the calls the mod uses). Everything was measured on the 2012 install (v1.4.0.0) with the bundle reader
([chameleon-bundle.md](chameleon-bundle.md)), the Genesys reading of [chameleon-vehicle-stats.md](chameleon-vehicle-stats.md), and
by looking at strings and references in `NFS13.exe`. Evidence tags: [README.md](../README.md#evidence-tags), and **[exe]** for a
string or table of the executable. A number that is not in the data is not given; a rule the data does not name is marked as
the rewrite's own.

## Short answer

| Question | Answer | Tag |
|---|---|---|
| Where is the payout? | `Genesys.Gen.NitrousParameters.Drifting` of the car's nitrous parameter objects; the same objects in all 93 cars | [verified] |
| How much | 6 units a second of a 100 unit tank for the burn nitrous (5 for the stock one) | [verified] numbers; "a second" **[unconfirmed]** |
| From what speed | 30 (mph) | [verified] number; the unit **[unconfirmed]** |
| After how long | 0.75 s | [verified] number; the meaning **[unconfirmed]** |
| What is a drift | the car's `DriftBehaviour`: a slide at 10 m/s or more and 5.5 to 12 degrees of slip (median 7.5) starts it, under 2 to 5 degrees (median 3) for 0.25 s ends it | [verified] numbers; roles **[unconfirmed]** |
| Cap | the tank, 100 units (the number is in `Restrictions` and in a fatal hit's gain); no other cap in the data | [verified] number; its role **[unconfirmed]** |
| HUD | the text "DRIFTING {distance}" (message 24010) | [verified] id and text |
| Counter-steer, brake, collision | not in the data as rules; the slip angle falling ends a drift, which counter-steering and braking cause | rewrite's own for the hit |

## The payout **[verified]**

Each car bundle has a `Genesys.Gen.Vehicle_Gameplay` object (324 bytes) whose handle fields at `+0x74`, `+0x7C` and `+0x84` name
three more objects of the same bundle: `+0x74` and `+0x7C` are two `Genesys.Gen.NitrousParameters` objects (640 bytes), `+0x84`
is the car's `PerformanceUpgrades` list. The two objects are the same in every car, so the id tells nothing about the car:

| Slot | Object (all 93 cars) | Reading |
|---|---|---|
| `+0x74` | `0x6A338` | the stock nitrous: every gain flag but the donut's is 0 **[unconfirmed]** |
| `+0x7C` | `0x6A590` | the burn nitrous: the flags are 1; the upgrade list names this object and `0x6A591` to `0x6A593`, `0x21B4EA`, `0x22264C`, `0x22264D` (which type is which: **[unconfirmed]**) |

The upgrade names ("BURN NITROUS", "BURN NITROUS PRO", "JUMP NITROUS", "POWERSHOT NITROUS PRO", "NO NITROUS") are in the text
tables ([chameleon-language.md](chameleon-language.md)), and the HUD's state machine switches on `CurrentNitrousType` between
"Burn", "Burn Pro", "Dump" and "Dump Pro" (the strings of the game mode machine in `GAMEMODES.BNDL`). Which object each type uses was not followed past the two slots above.

`NitrousParameters` is a list of 19 structures, one for each source of nitrous (a field name is hashed, the structure's
name is not): `TrafficOncoming` (+0x0C), `Balancing`, `Donutting`, **`Drifting` (+0x7C)**, `HittingAnotherCompetitor`,
`PunchUsage`, `Slipstreaming`, `CatchingAir`, `CrashEscape`, `HighSpeed`, `MinSpeed`, `NitrousUsage`, `TakingShortcut`,
`TrafficNearMiss` twice, `FatalHit`, `JumpUsage`, `Restrictions` and `SpeedbreakerUsage`. The `Drifting` structure is 36
bytes; the offsets are in it, the hashes are the field name hashes:

| Offset | Name hash | Burn | Stock | Meaning |
|---|---|---|---|---|
| +0x0C | `5E813FD8` | 24010 | 24010 | the HUD message id: string 24010 of the English table is `DRIFTING {Distance(DIST)}` **[verified]** |
| +0x10 | `4D907940` | 1.0 | 1.0 | a factor; 1.0 in every structure that has it **[unconfirmed]** |
| +0x14 | `D629C55F` | 30 | 30 | the speed under which it does not pay; the other sources ask for 20 (oncoming traffic), 50 (hitting a competitor, slipstream, crash escape), 70 (shortcut), 80 (near miss), so mph fits better than m/s **[unconfirmed]** |
| +0x18 | `D63CFF99` | 6.0 | 5.0 | the gain, in units of a 100 unit tank; also the field of every other source's gain (a fatal hit is 100, a competitor hit 18, a near miss 3, slipstream 3, air 24) **[unconfirmed]** |
| +0x1C | `F003DFE5` | 0.75 | 0.75 | the donut's structure has the same 0.75 under another hash: a time in seconds before the gain starts **[unconfirmed]** |
| +0x20 | `AEBA495E` | 1 | 0 | a flag every source has |

**Per second.** The gain of a continuous source (slipstream, drifting, donut) is small and that of an event (a hit, a near
miss) is large, and the HUD message of a drift carries a distance; a gain per metre would need a scale the data does not
hold. The rule takes the gain as units a second **[unconfirmed]**. The tank is taken as 100 units: that number is in the
`Restrictions` structure and is a fatal hit's gain. `NitrousUsage` holds 24 and 1.0; if 24 is units a second, a full tank burns
for 4.2 s **[unconfirmed]**.

The other sources, for context: hitting a competitor 18 units (burn), a near miss 3 and 6 (two structures), oncoming traffic
1.2 (with 0.25 and 0.5), slipstreaming 3, air 24 (with 0.1), a fatal hit 100, a donut 2 (with 30, 1.0 and 0.75), high speed 1.5 from 35, the
minimum speed 1.8. They are not used.

## When the physics calls it a drift **[verified numbers, roles unconfirmed]**

The handling definition ([chameleon-vehicle-stats.md](chameleon-vehicle-stats.md)) holds a `Genesys.Gen.DriftBehaviour` (336
bytes) of six structures: `Other`, `Yaw_torque`, `Side_force`, `Drift_scale`, `Drift_exit` and `Drift_trigger`. The last two decide
the entering and leaving; the others are the forces that make the car slide, which are not read. Values over the 93 cars:

| Structure | Offset | Value | Reading |
|---|---|---|---|
| `Drift_trigger` | +0x10 | 10 in 92 cars (100 in one) | speed in m/s from which a slide can start a drift |
| | +0x14 | 0 (5 in some) | not read |
| | +0x18 | 5.5 to 12; 7.0 (24 cars), 8.0 (15), 8.5 (12), 7.5 (10); median 7.5 | the slip angle, in degrees, that starts one |
| | +0x1C, +0x20 | 2.0 (90 in one); 4.5 to 12, median 5 | not read |
| `Drift_exit` | +0x10 | 2 to 5; 3.0 in 71 cars, 4.0 in 11, 5.0 in 9, 2.0 in 2 | the slip angle, in degrees, under which a drift ends |
| | +0x20 | 0.7 to 1.0, 1.0 in most | not read |
| | +0x24 | 0.25 in 81 cars, 0.1 in 10, 0 in 2 | seconds the slip has to stay under the exit angle |

The exit angle is under the trigger angle in every car (the largest exit minus trigger is -2 degrees), which is the shape of a
hysteresis. The Porsche 911 Carrera S is 10 m/s, 7 degrees and 5 degrees with 0.25 s. The units follow from the sizes
(degrees of a slip angle, metres a second against the 30 mph of the payout) and not from a name **[unconfirmed]**.

## The HUD

The message id 24010 is a line of the text tables: `DRIFTING {Distance(DIST)}` (`DIST` is the player's distance unit). The
other sources have theirs: 24011 `NEAR MISS X{Count}`, 24012 `ONCOMING NEAR MISS X{Count}`, 24009 `AIR {Time(TIME,SECS)}`, 24014
`NITROUS READY`, 1007667 `DONUT {Distance(DIST)}`. So the 2012 game shows the distance of the drift while it pays
**[verified]**. The HUD movies that draw it are not read. `NFS13.exe` has the view model `NitrousViewModel` (`NitrousAmount`,
`UsingNitrous`, `NitrousReadyPulse` ...), the property `IsDrifting` ("Is the players race car in a drift?", registered at
`0x6D1EFD`, its getter at `0x6DAFB0` calls a virtual function of the player state, slot `+0x6C`; the function behind it was not
read) and the statistics `DistanceDrifting` and `BigDriftDistance` **[exe]**. The drift event (`HSMDoDriftScoring`,
"Current Drift Score", drift markers) is a scoring game mode of its own and does not touch the nitrous.

## The rule as built

`chameleon_vehicle::DriftModel` is a state machine fed once a step with the speed along the car, the slip angle, the ground
contact and the strongest hit of the step ([`DriftInput`]); it answers with the nitrous earned as a part of a full tank
([`DriftOutput`]).

| State change | Condition | Source |
|---|---|---|
| none to drifting | on the ground, speed at least the trigger speed (10 m/s), slip at least the trigger angle (7.5 degrees), no hit | the data |
| stays | any step with the slip over the exit angle (3 degrees), the speed over the trigger speed and the wheels on the ground, and a run of steps under it shorter than the hold time (0.25 s) | the data |
| drifting to none | the hold time ran out, or a hit of 0.15 or more | the data; the hit is the rewrite's own |
| pays | drifting for 0.75 s, speed at least 30 mph (13.41 m/s), conditions on: 0.06 of a tank a second | the data |

Steering and braking are not inputs: counter-steering or braking ends a drift by taking the slip angle under the exit angle,
and the data names no other rule. The tank is clamped by the host. `DriftParams::default()` is the median car with the burn
nitrous; `read_drift_physics` and `read_drift_nitrous` read a car's own numbers. The model is pure (no clock, no car); the
callers are the mod `nfsmw-2012-content` (`drift_nitrous.rs`) and the tests.

## What is not known

- The unit of the minimum speed (mph is assumed from the other sources' numbers) and whether the gain is a second, a metre or a
  drift. No function that reads `NitrousParameters` was found in the executable (its field names are hashes, and the
  executable has almost no names), so the meaning of the fields is from the values and from the text tables.
- What the stock nitrous (`+0x74`) is: its flags are 0 for every source but the donut. The setting uses the burn nitrous.
- Which fields of `Drift_trigger` and `Drift_exit` besides the three read are rules (they are 0 or a constant in nearly all cars).
- What the HUD does with the message beyond the text (position, colour, the bar flash).
- Whether the drift detection of the nitrous is the physics' `DriftBehaviour` state or a separate test; the model follows the
  physics' numbers, since they are the only ones the data names for entering and leaving a drift.

## Checks

`cargo test -p chameleon-vehicle drift` runs the state machine on made-up steps. With `CHAMELEON_GAME_DIR=<2012 install>`,
`cargo test -p chameleon-vehicle drift -- --ignored` reads the Porsche's numbers above and the 93 cars (same burn payout,
physics in range, median trigger angle 7.5).
