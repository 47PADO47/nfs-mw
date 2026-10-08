# Pursuit data: helicopter sheets, cop tuning records, breaker events, pursuit zones

Data formats and tables that the cop, roadblock and helicopter specs read. Everything is checked on the install at
`D:/Need For Speed Most Wanted Black Edition` unless marked otherwise. Evidence tags as in the
[docs README](../README.md#evidence-tags): **[verified]** read from the install, **[decomp]** from the
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) decompilation (CC0-1.0), **[unconfirmed]** inferred.
Behaviour is in [../specs/ai-pursuit-tactics.md](../specs/ai-pursuit-tactics.md),
[../specs/ai-pursuit-formations.md](../specs/ai-pursuit-formations.md),
[../specs/ai-pursuit-cop-cars.md](../specs/ai-pursuit-cop-cars.md),
[../specs/ai-pursuit-roadblocks.md](../specs/ai-pursuit-roadblocks.md) and [../specs/ai-helicopter.md](../specs/ai-helicopter.md).
The AttribSys container is in [attributes.md](attributes.md); the track path zones and barriers in
[road-network.md](road-network.md) and [world.md](world.md).

## 1. `HeliSheet` chunk `0x00034159` [verified]

**Where.** `TRACKS/STREAML2RA.BUN`, top-level chunks (the stream file's chunk list is flat): **435 chunks, 940 584 bytes
in total**, 51 504 triangles. One chunk per *drivable section*; the section number is stored in the chunk (values 101 to
2030, all 435 distinct). The loader registers the chunk with a `bChunkLoader`; at run time the chunks of the loaded
sections form a list.

**Chunk payload** (little-endian):

| Offset | Size | Field | Notes |
|---|---|---|---|
| 0 | `pad` | fill `0x11` bytes | pads the data to a 16-byte file alignment (`pad` is 0, 4, 8 or 12) |
| `pad` | 4 | list node `next` | constant `11` in the file (placeholder, ignored) |
| `pad + 4` | 4 | list node `prev` | constant `11` (ignored) |
| `pad + 8` | 4 | `SectionNumber` | drivable section number |
| `pad + 12` | 4 | `NumPolys` | 27 to 311, mean 118.4 |
| `pad + 16` | 4 | `EndianSwapped` | 0 in the file (the console builds store 1 after fix-up) |
| `pad + 20` | 4 | `PolyTable` | 0 in the file (a run-time pointer, set to the end of the header) |
| `pad + 24` | `18 * NumPolys` | triangles | 3 `i16` x, 3 `i16` y, 3 `i16` z |
| end | 0 or 2 | alignment slack | 2 when `NumPolys` is odd |

The chunk size is exactly `pad + 24 + 18 * NumPolys (+2)` for all 435 chunks. **Triangle**: `VertexX[3]`, `VertexY[3]`,
`VertexZ[3]` as `i16`; world values `x = X / 4`, `y = Y / 4`, `z = Z / 16` metres. Observed ranges: `x` -1563.5 to 4965.75,
`y` -1200.75 to 5091.5, `z` -11.4 to 510.0. The 2-D plane `(x, y)` is the game's *map plane*: a world point `(wx, wy, wz)`
maps to `(x, y) = (wz, -wx)`; `z` is the elevation (world `y`, up). [decomp][verified]

**Lookup rule** (`HeliSheetManager::FindHeliPoly` + `HeliSheetCoordinate::GetElevation`) [decomp]:

```
section = visible_section_manager.drivable_section_number(point2d)      # 0 means none: no sheet here
chunk   = the loaded HeliSheet chunk with that SectionNumber           # none: no sheet here
q       = (int(point.x * 4), int(point.y * 4))
for tri in chunk.polys (in file order):
    if q.x within [min vx, max vx] and q.y within [min vy, max vy] of tri:     # integer quarter-metre bounds
        if point strictly inside the triangle (2-D test on the metre vertices): return tri
elevation(point) = (d - n.x * point.x - n.y * point.y) / n.z       with n = unit((v1 - v0) x (v2 - v0)), d = dot(v0, n)
```

The elevation is the height of the triangle's plane at the point (valid for either winding). A `HeliSheetCoordinate` caches
the last triangle and the last elevation (initial 1000) and only re-searches when the point leaves that triangle; if the
search fails it returns "not valid" and the last elevation. The visible-section mapping is the one of
[../specs/visible-sections.md](../specs/visible-sections.md). The sheet is a rough roof-height surface over the drivable
area; it only ever *raises* the helicopter (it never limits it from above).

## 2. AttribSys records used by the pursuit classes [verified]

Struct arrays in the vault (`Attrib::` types with element sizes from the class definitions). All little-endian.

| Record | Size | Layout |
|---|---|---|
| `StringKey` | 16 | 8 bytes of a 64-bit hash (offset 0, **[unconfirmed]**), `u32` 32-bit hash = `stringhash` of the text (offset 8, verified by matching cop names), `u32` text pointer (offset 12, null in the vault) |
| `CopCountRecord` (`pursuitlevels.cops`, max 3) | 24 | `StringKey CopType`, `u32 Count`, `u32 Chance` (0 means 100) |
| `CopFormationRecord` (`CopFormations`, max 10) | 12 | `u32 Formation` (1 PIT, 2 BOX_IN, 3 ROLLING_BLOCK, 4 FOLLOW, 5 HELI_PURSUIT, 6 HERD, 7 STAGGER_FOLLOW), `f32 Duration`, `f32 Frequency` |
| `AirSupport` | 12 | `u32 HeliStrategy` (0 HI_PATROL, 1 PURSUIT, 2 SKID_HIT, 3 SPIKE_DROP), `u32 Chance`, `f32 Duration` |
| `HeavySupport` | 16 | `u32 HeavyStrategy` (1 E_BRAKE, 2 COORDINATED_E_BRAKE, 3 RAM, 4 HEAVY_ROADBLOCK), `u32 Chance`, `f32 Duration`, `u32 ChanceBigSUV` |
| `LeaderSupport` | 20 | `u32 LeaderStrategy` (5 CROSS_FOLLOW, 6 CROSS_BRAKE, 7 CROSS_PLUS_V_BLOCK), `u32 Chance`, `f32 Duration`, `u32 PriorityChance`, `f32 PriorityTime` |
| `AICollisionReactionRecord` (`aivehicle.PlayerCollisions`, max 10) | 16 | `u32 Goal` (lookup2 hash of the goal name), then a 12-byte `RefSpec`: class hash `0xB32682F1` (`collisionreactions`), collection hash, pointer |
| `CollisionReactionRecord` (`collisionreactions`: `REAR_REACTION`, `REARSIDE_REACTION`, `FRONT_REACTION`, `FRONTSIDE_REACTION`) | 24 | `f32 Elasticity, RollHeight, WeightBias, MassScale, StunSpeed, StunTime` |
| `DamageScaleRecord` (`damagespecs` zones `DZ_*`) | 8 | `f32 VisualScale`, `f32 HitPointScale` |

The collection-name and goal-name hashes are the Jenkins lookup2 hash of the lower/upper-case text exactly as written in
the decomp (for example `AIGoalStaticRoadBlock`).

### 2.1 `pursuitescalation/default`

Four arrays of ten references, index = `int(heat) - 1` (one-based heat levels 1 to 10): `heattable` -> `pursuitlevels`
`heat_01..heat_10`, `racetable` -> `race_01..race_10`, `supporttable` -> `pursuitsupport` `support01..support10`,
`supportracetable` -> `supportrace_01..supportrace_10`. The `race` tables are used while a (non-pursuit) race event runs.
(The level collection names are stored hashed; the names were recovered by hashing candidates.) [verified]

### 2.2 `pursuitlevels`: cop mix and the fields the tactics specs use

Cop mix (`type x count @ chance`): **[verified]**

| Level | `cops` |
|---|---|
| `heat_01` | `copmidsize` x4 |
| `heat_02` | `copghost` x5 |
| `heat_03` | `copgto` x7 |
| `heat_04` | `copgtoghost` x8, `copheli` x1 @50 |
| `heat_05` | `copsporthench` x10 @50, `copheli` x1 @60 |
| `heat_06` | `copsportghost` x8 @80, `copheli` x1 @50, `copsuvpatrol` x2 @80 |
| `heat_07` | `copsuvpatrol` x6 @90 |
| `heat_08` | `copsportghost` x8, `copheli` x1 @60 |
| `heat_09` | `copmidsize` x4, `copheli` x1 @80 |
| `heat_10` | `copsport` x1 @90, `copheli` x1 @100 |
| `race_01` / `02` / `03` | `copmidsize` x2 / `copghost` x3 / `copgto` x4 |
| `race_04` / `05` / `06` | `copgtoghost` x5 + heli @60 / `copsporthench` x5 + heli @60 / `copsportghost` x6 + heli @50 |
| `race_07` / `08` / `09` / `10` | `copsuvpatrol` x8 + heli @60 / `copghost` x8 + heli @75 / `copghost` x8 + heli @80 / `copghost` x8 + heli @90 |
| `default` | `copmidsize` x1 |

Heat levels, selected fields (`react` = `SpeedReactionTime` s; `coll` = `CollapseSpeed` km/h; `aggr` = `CollapseAggression`;
`in/out/max` = `CollapseInnerRadius`/`CollapseOuterRadius`/`MaxCopsCollapsing`; `box` and `roll` = tightness/duration;
`stag` = `StaggerFormationTime`; `rb` = `roadblockprobability`/`roadblockspikechance`/`SearchModeRoadblockChance`; `fuel`/`gap`
= `HeliFuelTime`/`TimeBetweenHeliActive`; `LOS` = front/rear/heli):

| Level | react | coll | aggr | in/out/max | box | roll | stag | rb | fuel/gap | LOS |
|---|---|---|---|---|---|---|---|---|---|---|
| `heat_01` | 1.75 | 58 | 0.4 | 6/15/4 | .5/4 | .5/4 | 7 | 0/0/0 | 10/0 | 151/151/251 |
| `heat_02` | 1.5 | 68 | 0.5 | 5/15/5 | .6/5 | .6/5 | 6 | 10/0/3 | 10/0 | 151/151/251 |
| `heat_03` | 0.75 | 78 | 0.6 | 4/8/6 | .8/6 | .8/6 | 5 | 25/0/5 | 10/120 | 151/151/251 |
| `heat_04` | 0.5 | 88 | 0.7 | 3/5/7 | .9/7 | .9/7 | 4 | 30/70/7 | 60/180 | 151/151/251 |
| `heat_05` | 0.1 | 96 | 0.8 | 3/5/8 | 1/8 | 1/8 | 3 | 40/80/10 | 90/180 | 151/151/251 |
| `heat_06` | 0.1 | 120 | 0.8 | 3/7/8 | 1/10 | 1/10 | 5 | 50/85/12 | 180/180 | 151/151/251 |
| `heat_07` | 0.5 | 78 | 0.5 | 4/10/9 | 1/10 | 1/10 | 4 | 35/85/12 | 10/180 | 151/151/251 |
| `heat_08` | 0 | 15 | 0 | 3/15/6 | .5/3 | .5/3 | 10 | 50/0/60 | 75/120 | 201/101/301 |
| `heat_09` | 0 | 1 | 0 | 2/5/6 | .5/3 | .5/3 | 7 | 100/100/0 | 120/60 | 201/101/301 |
| `heat_10` | 0 | 15 | 0 | 3/15/6 | .5/3 | .5/3 | 10 | 80/0/80 | 400/5 | 201/101/301 |

Race levels: `react` 0, `coll` 15, `aggr` 0, `in/out/max` 3/15/6, box and roll .5/3, `stag` 10; `rb`: 0/0/0 for levels 1 to 7,
35/0/50, 40/0/50, 50/0/50 for levels 8 to 10; `fuel` 10, 20, 60, 80, 120, 120, 220, 75, 120, 120, `gap` 120; LOS 151/51/251 for
levels 1 to 7 and 201/101/301 for 8 to 10. `default`: `react` 0, `coll` 15, LOS 151/51/201, `fuel`/`gap` 20/20, `rb` 0/0/50.
`roadblockhelichance` is 0 on every level; `SearchModeRoadblockRadius` is 1000 on every level; `SearchModeHeliSpawnChance`
is 2, 4, 5 on heats 4, 5, 6 and 0 elsewhere. (The other 40 fields of the class are in the heat spec,
[../specs/ai-pursuit-heat.md](../specs/ai-pursuit-heat.md).)

### 2.3 `pursuitsupport` (per support row)

`delay` = `MinimumSupportDelay` s; `Air`, `Heavy`, `Lead` as `strategy chance% duration s` (heavy also `bigSUV%`; lead also
`priorityChance% @ priorityTime s`). **[verified]**

| Row | delay | Air | Heavy | Lead |
|---|---|---|---|---|
| `support01` / `02` | 60 | PURSUIT 0 / 60 s | RAM 0 (02: 20 s) | CROSS_FOLLOW 0 |
| `support03` | 60 | PURSUIT 0 / 0 s | HEAVY_ROADBLOCK 5 / 40 s; RAM 10 / 20 s | 0 |
| `support04` | 45 | PURSUIT 50 / 60 s | RAM 25 / 30 s (SUV 100); HEAVY_ROADBLOCK 25 / 20 s (100) | 0 |
| `support05` | 35 | SKID_HIT 60 / 90 s | RAM 25 / 30 s (100); HEAVY_ROADBLOCK 40 / 20 s (100) | CROSS_FOLLOW 5 / 2000 s, priority 10 @120 s |
| `support06` | 10 | SKID_HIT 100 / 180 s | RAM 20 / 30 s (100) | CROSS_FOLLOW 5 / 2000 s, priority 100 @60 s |
| `support07` | 60 | SKID_HIT 0 / 0 s | RAM 30 / 30 s (100) | 0 |
| `support08` | 10 | HI_PATROL 75 / 180 s | RAM 40 / 20 s (80) | CROSS_FOLLOW 0 / 500 s |
| `support09` | 10 | SKID_HIT 100 / 2000 s | RAM 0 | CROSS_FOLLOW 100 / 2000 s, priority 100 @30 s |
| `support10` | 10 | SKID_HIT 100 / 999 s | RAM 0 / 20 s | 0 / 2000 s |
| `supportrace_01` / `02` | 10 | PURSUIT 0 / 30 s | RAM 0 / 20 s | 0 |
| `supportrace_03` | 10 | PURSUIT 0 | RAM 0 | 0 |
| `supportrace_04` | 30 | PURSUIT 60 / 80 s | RAM 0 (SUV 100) | 0 |
| `supportrace_05` | 30 | SKID_HIT 60 / 120 s | RAM 0 (100) | 0 |
| `supportrace_06` / `07` | 10 | SKID_HIT 70 / 180 s, 75 / 220 s | RAM 0 (100) | 0 |
| `supportrace_08` / `09` / `10` | 10 | HI_PATROL 80 / 90 s, 90 / 120 s, 100 / 120 s | RAM 40 / 50 / 60 @20 s (100) | CROSS_FOLLOW 10 / 15 / 20 @240 s |
| `default` | 10 | PURSUIT 0 / 30 s | RAM 0 / 20 s | 0 |

(The AI reads only whether any `Air` entry has strategy `SKID_HIT`, and the heavy and leader entries; see the helicopter and
formation specs.)

## 3. `aivehicle`, `damagespecs`, `chopperspecs`

Per-cop-car `aivehicle` values and `damagespecs` are tabulated in
[../specs/ai-pursuit-cop-cars.md](../specs/ai-pursuit-cop-cars.md) sections 6 and 8; `collisionreactions` sets for cops in the
same file section 7. `chopperspecs/default` and `rigidbodyspecs/chopper` are in [../specs/ai-helicopter.md](../specs/ai-helicopter.md)
sections 1 and 5. `smackable/spikestrip` and `sawhorse` are in the roadblocks spec section 6.1.

`chopperspecs` has 24 fields and three collections (`default`, `qchopper`, `henchchopper`); only `MAX_SPEED_MPS`
(`default` 100, `qchopper` 30, `henchchopper` 80), `PITCH_ANG` (.05 / .09 / .09), `ROLL_ANG` (.06 / .08 / .08),
`PITCH_ALIGN_SCALE` (6 / 7 / 7) and `ROLL_ALIGN_SCALE` (4.5 / 7.5 / 7.5) are read by the code found. [decomp][verified]

## 4. Pursuit breakers in `L2RA.BUN` [verified]

`CarpEventSequences` (`0x8003B810`) holds **73** `0x0003B811` event-sequence chunks (sizes 1 048 to 7 704 bytes). Each is a
CARP blob (16-byte aligned, magic `CARP`) of one prop's destruction sequence (strings such as `trigger`, `create`, `OBJECT_COLLISION`,
`IMPACT_1`, `Damaged`, effect names). The event class `EBreakerStopCops` (static data: `f32 radius` then `f32 duration`) is
referenced by the 32-bit hash `0xDDA7829B` (the lookup2 hash of `EBreakerStopCops`); the 21 event records that follow it
directly with the pair (radius, duration) are in 15 sequences:

| Sequence prop (string in the chunk) | (radius m, duration s) |
|---|---|
| donut shop roof (`donutshoproof_seq`) | (16, 5) |
| donut (`donut_seq`) | (12, 5) |
| drive-in (`drivein_seq`) | (12, 5) |
| gas station roof (`gasstationroof_seq`) | (16, 5) |
| gazebo (`gazebo_seq`) | (5, 5) |
| greenhouse (`greenhouse_seq`) | (10, 5) |
| heavy concrete prop at the construction scaffold (`heavyconcrete_seq`) | (20, 5) |
| `hinkleybase_seq` | (24, 5) |
| radio tower support (`radiotowersupport_seq`) | (20, 5) x2, (10, 5) x2, (24, 5) |
| radio tower (`Scradiotower_seq`) | (10, 5) x2 |
| sailboat (`sailboat_seq`) | (12, 5) |
| scaffold support (`scaffoldsupport_seq`) | (10, 5) |
| seafood patio (`seafoodpatio_seq`) | (5, 5), (9, 5) |
| water tower support (`watertowersupport_seq`) | (20, 5) |
| wooden scaffold (`woodenscaffold_seq`) | (10, 5) |

(The 21 other occurrences of the hash in the file are table entries, not events.) The sequence data is an event system script;
the AI only sees the resulting `MBreakerStopCops` message with the same radius, duration and the event position. The event
system itself (`EventSequencer`, states, stimuli) is not specified here.

## 5. Track path zones used by pursuits [verified]

Chunk `0x0003414A` `TrackPathZones` in `L2RA.BUN` is a plain run of 705 variable-size records (the chunk payload, at file offset
128, is 74 628 bytes; each record's `MemoryImageSize` at `+0x42` equals `0x44 + 8 * NumPoints`, and the records tile the
payload exactly). The record is `TrackPathZone` (`Type` at +0, `Position` +4, `Direction` +0xC, `Elevation` +0x14, bounding
box +0x20/+0x28, `Data[4]` +0x30, `NumPoints` `i16` +0x40, `MemoryImageSize` `i16` +0x42, points `f32 x,y` from +0x44). Counts per
type: 3 TUNNEL 70, 4 OVERPASS 143, 5 OVERPASS_SMALL 178, 6 STREAMER_PREDICTION 211, 7 GARAGE 8, 8 HIDDEN 34, 9 TRAFFIC_PATTERN 11,
10 DYNAMIC 14, 11 NEIGHBOURHOOD 8, 12 JUMP_CAM 22, **13 NO_COP_SPAWN 3**, **14 PURSUIT_START 3**. Zone positions are in the map plane
`(z, -x)` of world coordinates.

- **HIDDEN (8)**: all 34 have a non-zero `Elevation` and `Data[0..3] = 0`. The perpetrator code treats `Data[0]` equal to the
  string hash of `"Car"` as "hides from cars only", of `"Heli"` as "hides from helicopters only" and anything else as "hides
  from both"; in the install every hidden zone is the third case. A target is *in* a zone when it is inside the polygon and
  `|feet height - Elevation| < 1.25 m`.
- **NO_COP_SPAWN (13)**: `AISpawnManager::CheckSpawnPosition` rejects any cop spawn point inside such a zone.
- **PURSUIT_START (14)**: while not pursued, in free roam, with spawning allowed, every 11th perpetrator update (0.5 s each)
  the code checks whether the vehicle is inside such a zone; if so it clears the cop lockout and forces a pursuit at the
  vehicle's current heat (`MForcePursuitStart`). It is the "cops start chasing here" trigger of fixed locations.

[decomp][verified]
