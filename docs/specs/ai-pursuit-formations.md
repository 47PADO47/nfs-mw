# Cop formations, finishers, the collapse and support vehicles

How a pursuit with several cops turns the chase into a coordinated manoeuvre: choosing a formation, giving each cop a
slot, the "finisher" (box-in, rolling block, pit), the collapse (a ring of cops around a slow target) and the extra
SUVs and the cross-over that the pursuit can request. The per-cop driving that carries out each slot is in
[ai-pursuit-tactics.md](ai-pursuit-tactics.md). Pursuit state, heat, evade and busted rules, cop counts and spawning
belong to the pursuit-management spec; this file only covers what the formation code does with the cops it has.

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled; GameCube build), under
  `src/Speed/Indep/Src`: `AI/AIPursuit.h`, `AI/Common/AIPursuit.cpp` (formation classes, `UpdateFormation`,
  `AssignClosestOffsets`, `SetupCollapse`, `RequestGroundSupport`, `UpdateJerk`), `AI/Activities/AICopManager.cpp`
  (`GetHeavySupportVehicles`, `StartHeavySupport`, `GetLeaderSupportVehicles`, `StartLeaderSupport`,
  `UpdateSupportCops`), `AI/aireflectedtypes.h`, `AI/AIVehiclePursuit.{h,cpp}`. Read for understanding; no code copied.
- **Data inputs:** `pursuitlevels` (`CopFormations`, `StaggerFormationTime`, `BoxinTightness`, `BoxinDuration`,
  `RollingBlockTightness`, `RollingBlockDuration`, `CollapseSpeed`, `CollapseAggression`, `CollapseInnerRadius`,
  `CollapseOuterRadius`, `MaxCopsCollapsing`), `pursuitsupport` (`HeavySupportOptions`, `LeaderSupportOptions`,
  `MinimumSupportDelay`), see [../formats/pursuit-data.md](../formats/pursuit-data.md).

Evidence tags as in the [docs README](../README.md#evidence-tags): [decomp] from the decompiled sources, [verified]
from the install's AttribSys vaults. Coordinates called "target frame" are metres in the target car's frame:
`x` along the side axis `(dir.z, 0, -dir.x)` (`dir` = the target's heading), `z` along the heading (positive ahead of
the target). The same axes are used when a cop turns a slot into a world point.

## 1. Formation types

`FormationType` values: PIT 1, BOX_IN 2, ROLLING_BLOCK 3, FOLLOW 4, HELI_PURSUIT 5 (builds the follow formation, only
used by name), HERD 6, STAGGER_FOLLOW 7. A formation owns an ordered list of **slots**, each with a *target offset*
(`x, z`; where the cop waits), a *priority* (`minTargets`, smaller = filled first), an *in-position goal* (the goal the
cop switches to at the finisher; null for none) and an *in-position offset* (where it drives during that goal), and the
parameters `MaxCops`, `MinFinisherCops`, `HasFinisher`, `TimeToFinisher`, `FinisherTime`, `FinisherTolerance`. [decomp]

| Formation | `MaxCops` | Finisher | `TimeToFinisher` | `FinisherTime` | Tolerance |
|---|---|---|---|---|---|
| Stagger follow | 6 | no | - | - | - |
| Follow | 6 | no | - | - | - |
| Herd | 3 | no | - | - | - |
| Box-in | 4 (min 2) | yes | 4.0 s | `BoxinDuration` | 1.0 |
| Rolling block | 4 (min 2) | yes | 4.0 s | `RollingBlockDuration` | 1.0 |
| Pit | 1 (min 1) | yes | 1.2 s | 2.0 s | 0.5 |

Slots (target frame, `x z`; "pri" = priority). Box-in and rolling block use `AIGoalRam`, pit uses `AIGoalPit`.

| Formation | Slots (pursuit offset; in-position offset) |
|---|---|
| Stagger follow | `(0,-13)` pri 1; `(0,+13)` pri 1; `(3.5,-13)` pri 2; `(-3.5,+13)` pri 2; `(-3.5,-13)` pri 3; `(3.5,+13)` pri 3; no in-position goal |
| Follow | `(0,-13)` 1; `(3.5,-13)` 2; `(-3.5,-13)` 2; `(0,-17)` 3; `(3.5,-17)` 4; `(-3.5,-17)` 4; no in-position goal |
| Herd | `(-3,0)` 1; `(-3,+5)` 2; `(-3,-5)` 3 (x is rewritten every update, section 4); no goal |
| Box-in | `(0,+14)` 1, in-position `(0, foff)`; `(-3.5,0)` 2 and `(3.5,0)` 2, in-position `(+-3.5 s, 0)`; `(0,-7.5)` 4, in-position `(0, -7.5 s)` |
| Rolling block | `(0,14)` 1; `(-2.5,14)` 2; `(2.5,10)` 2; `(-5,14)` 3; `(5,14)` 3; in-position `base * s` with `z = foff` |
| Pit | `(+4,-2.7)` 1, in-position `(-10,-2.7)`; `(-4,-2.7)` 1, in-position `(+10,-2.7)` |

with, for box-in, `t = BoxinTightness`; for rolling block `t = RollingBlockTightness`: `foff = 2 - 5t` and `s = 0.7 - 0.5t`
(box-in) or `s = 1 - 0.8t` (rolling block). Without a pursuit level (no attributes) `t = 0.5` and the duration is 2.0 s.
[decomp]

Reading the table: follow and stagger follow keep cops behind (and, for stagger, also ahead of) the target; herd keeps
three cops on the target's left; box-in puts one cop in front and two beside the target, closing in; rolling block puts
a row of cops in front; pit sends one cop alongside the target's rear quarter and then across its rear to the opposite
side.

**Tightening.** Box-in and rolling block rewrite their pursuit offsets every update:
`f = TimeToFinisherAttempt / TimeToFinisher` (1 when the in-position timer is empty, 0 when full), `k = 0.2 t + 0.2`
(box-in) or `0.4 t` (rolling block), `scale = f k + (1 - k)`; every pursuit offset is `base * scale`. The cops creep in
by up to 20 to 40 percent as the finisher approaches. [decomp]

## 2. Choosing the formation

The pursuit task (0.25 s) runs the following, in this order, every tick (after the jerk update, [ai-pursuit-cop-cars.md](ai-pursuit-cop-cars.md) section 1):

```
formation_time -= dT                    # also spawn, support, roadblock timers
if formation_time <= 0 and level has formations and not finisher_active and not busted and not bailed and speech_done:
    n = level.Num_CopFormations
    if n > 0:
        new = STAGGER_FOLLOW; time = level.StaggerFormationTime
        if roadblock exists and not dodged and no cop in it damaged or destroyed:  new = FOLLOW
        elif attempt_count is odd:
            pick record i with probability Frequency_i / sum(Frequency); new = rec.Formation; time = rec.Duration
        attempt_count += 1
        if new != active: active = new; rebuild the slot list
        formation_time = random(0, 3) + time
on heat level change (integer part of heat): formation_time = 0
on roadblock creation (AddRoadBlock): unless the formation is FOLLOW or a finisher is running: formation_time = 0, finisher timer = -1
```

`speech_done` is true when cop speech is disabled or the player is in a pursuit race; otherwise 15 s must have passed
since the speech controller last reported its "setup" focus. So the formations alternate: an even attempt always picks
*stagger follow* for `StaggerFormationTime` seconds (3 to 10), an odd attempt picks a weighted random aggressive one for
its `Duration` (40 s on most heat records, 20 to 30 s on a few, 10 to 50 s on race records). [decomp][verified]

Per-level formation lists **[verified]** (Duration/Frequency): heat 1: rolling block 40/100; heat 2: box-in 40/100;
heat 3: box-in 40/25, pit 40/60, rolling block 40/20, herd 40/40; heat 4: pit 40/50, box-in 40/50, herd 40/30, rolling
block 40/50; heat 5: rolling block, box-in, pit, herd each 40/50; heat 6: rolling block 40/60, box-in 40/50, herd 40/50;
heat 7: box-in 40/10, rolling block 40/60; heat 8: box-in 30/60, rolling block 20/40, pit 30/70, herd 30/50; heat 9: pit
40/100; heat 10: follow 10/100. Race levels 1 to 7 and `default`: stagger follow 10/100 (or pit 10/10 in `default`); race
8 and 9: follow 10/100; race 10: follow 10/100, follow 15/5, herd 15/30, box-in 50/50. `StaggerFormationTime` is 10 on
race levels, 7/6/5/4/3/5/4 on heats 1 to 7 and 7 on heat 9.

## 3. Assigning cops to slots

Each pursuit tick (`UpdateFormation`, needs a valid target with a vehicle AI and a rigid body):

1. `formation.Update(dT)` (tightening, herd lateral offset).
2. **Candidates.** For each cop in the pursuit: skip support vehicles; set `InFormation = false`; a helicopter instead gets
   the helicopter goal (section 7); a car is a candidate when it is drivable-to the target (clear ray) and within
   `60 + distance(target, its road point)` metres. Its position is converted to the target frame.
3. **Choose slots (`EvenOutOffsets`).** While there are more candidates than chosen slots and fewer than `MaxCops`
   chosen: among the unchosen slots whose priority is not larger than the best priority seen so far, take the one with the
   smallest summed distance to all candidates. This fills slots in priority order, each time taking the one the cops
   are collectively closest to.
4. **Match (`AssignClosestOffsets`).** Distances use `xz` with `z * 0.25` (longitudinal gaps count a quarter). Greedy
   rounds: find the candidate with the largest maximum distance to any free slot; take *that* slot (the one at that
   maximum distance); give it to the free candidate nearest to it; repeat. Each matched cop receives
   `PursuitOffset = slot.offset`, `InPositionOffset = slot.in_position_offset`, `InPositionGoal = slot.goal`,
   `InFormation = true`.
5. **Everyone else** (`UpdateOutOfFormationOffsets`): non-chopper, non-support cops that are not in the formation are
   spread on a grid ahead and behind the target: for the i-th such cop (`r = i / 6`, `s = 1 - 2 (i mod 2)`,
   `c = (i / 2 + 1) mod 3 - 1`): offset `(3.5 c, 0, s (5 r + 25))`, matched with the same greedy rule, `InFormation = false`,
   no in-position goal. They are therefore 25 m or more ahead or behind in three lanes' width.
6. **In position.** A cop in the formation is *in position* when its horizontal distance (target frame) to its pursuit
   offset is under 4 m (the "in position" counter and the summed distance feed the finisher).

[decomp]

## 4. Herd

Every update, find the road under the target (nav at the target position, centre lane, direction of travel), the signed
lateral distance `ro` of the target from the road centre point (along the side axis), whether the target runs
against the segment direction, and the greatest lane offset among the lanes of type "traffic" on the target's travel
side (`rl`). `crowd = max(1, min(3, ro - rl + 2))` and every herd slot's `x` is set to `-crowd`. The cops therefore sit
1 to 3 m on the target's `-x` side (the farther the target is from its outermost traffic lane in the `+x` direction, the
closer they sit to it): they press the car towards the `+x` verge. If the road has no lane profile the offsets are left
as they were. [decomp]

## 5. Finisher and collapse

State: `in_formation_timer` (seconds the formation has been in place), `breaker_timer` (-1 idle; 0 up to `FinisherTime`
while a finisher is running), `collapse_active`. One tick, after the assignment above:

```
cop_speed_limit  = pursuitlevels.CollapseSpeed (km/h);  if jerk pursuit: 125 km/h
if breaker_timer in [0, FinisherTime) and not busted and not bailed:
    breaker_timer += dT                                     # finisher running
elif target.speed < collapse_speed and cops_in_formation > 0 and not busted and perp_in_sight and not bailed:
    collapse_active = SetupCollapse(candidates, MaxCopsCollapsing, CollapseInnerRadius, CollapseOuterRadius)
    if collapse_active and heavy support is active: all support cars flee; support request reset
elif not busted and (breaker_timer >= 0 or collapse_active):
    # the finisher or collapse is over: put every cop back into a normal chase
    collapse_active = false; in_formation_timer = 0; breaker_timer = -1
    for each candidate: InPositionGoal = none; StartPursuit(target)            # goal AIGoalPursuit
    for each cop in the pursuit: if its goal equals its in-position goal or is AIGoalPullOver: same reset
elif formation.HasFinisher and cops_in_position > 0 and not collapse and not busted and not bailed:
    avg   = summed distance of in-position cops / cops_in_position
    rate  = formation.FinisherTolerance * 4
    in_formation_timer += clamp((2 rate - avg) / rate, -1, 1) * dT  ; min 0
    if in_formation_timer >= formation.TimeToFinisher:
        if cops_in_position >= MinFinisherCops:
            breaker_timer = 0
            for each formation cop with an in-position goal: SetGoal(in_position_goal)     # AIGoalRam / AIGoalPit
        else in_formation_timer = TimeToFinisher - 0.01
else in_formation_timer = 0
```

Timer reading: the in-position timer rises at `+1` per second while the average distance of the in-position cops to
their slots is at most `rate = 4 * tolerance` metres (4 m; 2 m for the pit), falls linearly to 0 per second at
`2 * rate` (8 m; 4 m) and drains at up to `-1` per second beyond `3 * rate`. The cops must hold their slots within a
few metres for `TimeToFinisher` seconds to arm the finisher. Once armed, the cops' goal changes: they drive to their in-position offsets
with `AIActionRam` (section 5.1 of the tactics spec) for `FinisherTime` seconds (3 to 10 s), which is the box-in
squeeze, the rolling block braking in front and the pit sweep. The finisher **ends** when `breaker_timer` reaches
`FinisherTime`, then the reset above puts the cops back into the normal chase and a new formation is chosen when the
formation timer expires. The finisher is cut short by `EndCurrentFormation` (a roadblock arrives). The speech system
polls `IsFinisherActive` (`breaker_timer >= 0`) and `TimeToFinisherAttempt` for its cop chatter. [decomp]

### 5.1 `SetupCollapse` (the ring)

Candidates are the cops that are drivable-to the target, within 60 m, not support vehicles, not fleeing and not
helicopters. Let `inner = max(3, CollapseInnerRadius)`, `outer = max(inner + 1, CollapseOuterRadius)`. If there are more
candidates than `MaxCopsCollapsing`, sort by distance: the farthest ones form the outer ring (radius `outer`), the
nearest `MaxCopsCollapsing` form the inner ring (radius `inner`). Each ring: sort its cops by a cheap pseudo-angle of
their offset from the target (front = 0, measured in the target's `front/side` frame; implemented with a ratio
function, range -4 to 4), find the cop nearest the front, then assign the cops in sorted order to the slots at angles
`2 pi i / n` starting at that cop: in-position offset `(sin a, 0, cos a) * radius`, in-position goal `AIGoalPullOver`,
and switch the cop to that goal immediately. The call repeats every tick the collapse condition holds, so cops
re-sort as the target drifts. `CollapseSpeed` per level **[verified]**: heat 1 to 7: 58, 68, 78, 88, 96, 120, 78 km/h;
heats 8 and 10, the race levels and `default`: 15 km/h; heat 9: 1 km/h. `CollapseInnerRadius`/`OuterRadius`: 6/15 (heat 1), 5/15, 4/8, 3/5,
3/5, 3/7, 4/10, then 3/15 (heat 8, 10, race), 2/5 (heat 9); `MaxCopsCollapsing` 4, 5, 6, 7, 8, 8, 9, then 6. In
the pull-over goal a cop drives to its ring slot (`AIActionRam` with the avoidance and stop described there) and halts
with brakes and handbrake when it arrives; `CollapseAggression` (0.4, 0.5, 0.6, 0.7, 0.8, 0.8, 0.5 on heats 1 to 7;
0 elsewhere) controls how willing the cops are to squeeze past each other. [decomp][verified]

## 6. Ground support requests

`RequestGroundSupport` (called every manager tick for each pursuit) returns an active or newly rolled request:

```
if busted or perp not in sight or bailed: none
if a request is already pending or active: return it
if support_check_timer >= 0: none ; support_check_timer = 10 s (it starts at 10 s)
if pursuit_time < MinimumSupportDelay: none
rand = random 0..99
if not priority_check_done:                                 # reset on every integer heat change
    for each Leader option with PriorityTime < pursuit_time:
        priority_check_done = true
        if PriorityChance > rand: request = that leader option; duration = option.Duration; break
if no request yet:
    rand = random 0..99
    if no roadblock exists:
        for each Heavy option: rand -= Chance; if rand < 0: request = it; duration; roadblock_timer = 15 s; break
    if rand >= 0:
        for each Leader option: rand -= Chance; if rand < 0: request = it; duration; break
if request is a Leader option and a `copcross` exists (spawned or disabled): cancel it
```

(The leader "priority" roll fires once per heat level the first time the pursuit has lasted `PriorityTime`.) The
manager then executes a pending request (`UpdateSupportCops`):

| Strategy | Cars | Goal given to each | Notes |
|---|---|---|---|
| `E_BRAKE` | 1 SUV | `AIGoalPursuit` | named "e-brake" but no braking code exists; a normal chase car |
| `COORDINATED_E_BRAKE` | 2 SUVs | `AIGoalPursuit` | same |
| `RAM` | 2 SUVs | `AIGoalHeadOnRam` | a 3 s delay timer is armed when each is taken from the pool |
| `HEAVY_ROADBLOCK` | 4 SUVs | `AIGoalStaticRoadBlock` | the four SUVs become a roadblock ([ai-pursuit-roadblocks.md](ai-pursuit-roadblocks.md)) |
| `CROSS_FOLLOW`, `CROSS_BRAKE` | 1 `copcross` | `AIGoalPursuit` | pursuit offset `(0,0,-20)`; no distinguishing code |
| `CROSS_PLUS_V_BLOCK` | `copcross` + 2 `copsporthench` | `AIGoalPursuit` | offsets `(0,-20)`, `(-4,-24)`, `(4,-24)` |

SUVs are `copsuv` with probability `ChanceBigSUV` percent, otherwise `copsuvl`; they are taken from the inactive cop
pool by name. The goal is stored as the vehicle's **support goal** (a non-null support goal marks a *support vehicle*,
which formation assignment ignores). When all cars have been acquired (and, for RAM, the delay has run out):

- Heavy (not roadblock) and leader requests are *started*: for each car, find a spawn point on the target's cop road at
  `340 - 10 k` m ahead for heavy cars or `150 + 10 k` m **behind** for the cross group (`k` = 0 to 7 tries per car;
  the 10 m steps are not reset between the cars of one request, so later cars start farther from the first distance),
  each checked by the spawn manager (distance window 150 to 400 m, no "no cop spawn" zone, no
  overlap). The car is placed there with its heading reversed so that it faces the target, activated, added to the
  pursuit (`StartPursuit`, then `StartSupportGoal` sets the goal), the request becomes ACTIVE with
  `timer = option.Duration`. If no spawn point is found the request is denied (a "request denied" message for the
  speech) and nothing is spawned.
- The request is reset when its timer reaches zero (`Duration`: 20 to 40 s for heavy, 240 to 2000 s for the cross),
  when the perp has not been in sight and a heavy support is active (cars flee), when a collapse begins (cars flee), or when
  the last support car leaves. A reset clears each car's support goal and returns it to the pursuit contingent as an
  ordinary cop, which the formation code then assigns.

Per-level options **[verified]**: see [../formats/pursuit-data.md](../formats/pursuit-data.md). In short: heats 1 and 2
and race levels 1 to 3 have none (all chances 0 or duration 0); heat 3 gets a `HEAVY_ROADBLOCK` (5 percent, 40 s) and
`RAM` (10 percent, 20 s); heat 4 and 5 get `RAM` 25 percent plus roadblock 25/40 percent; heats 6 to 10 add `RAM` 20, 30,
40, 0, 0 percent; the `copcross` leader appears from heat 5 (5 percent, 2000 s) with a priority roll at 120 s, heat 6
(priority 100 percent at 60 s) and heat 9 (100 percent, 30 s). [decomp][verified]

## 7. Helicopters in the formation

Helicopters are never given slots. Each tick a helicopter that is not already exiting and not part of a roadblock attempt
is assigned the in-position goal `AIGoalHeliPursuit`, `InFormation = true`, and switched to it if it is on another goal.
Details in [ai-helicopter.md](ai-helicopter.md). [decomp]

## 8. Constants not in AttribSys

| Name | Value |
|---|---|
| Pursuit task period | 0.25 s |
| Formation candidate distance | 60 m + target's offset from its road |
| In-position distance | 4 m |
| Out-of-formation grid | 3.5 m lanes, 25 m + 5 m per row of 6 |
| Time added to a formation | `random(0, 3)` s |
| Speech wait before a new formation | 15 s (not for pursuit races or with speech off) |
| Support check period | 10 s (starts at 10) |
| Roadblock timer after a heavy request | 15 s |
| Heavy spawn distance | `400 - 60 - 10 k` ahead (`k` = 0 to 7 tries) |
| Cross spawn distance | `150 + 10 k` behind |
| Heavy RAM delay | 3 s |
| Collapse minimum radii | inner 3, outer inner + 1 |

## 9. How to check it

1. Watch the formation labels: even attempts always show stagger follow; odd attempts pick one of the level's
   aggressive formations in proportion to its frequency.
2. Box-in: drive straight at constant speed on a long straight at heat 2 until the box starts to close; the four slot
   cars should hold 14 m ahead, 3.5 m either side and 7.5 m behind, creeping in.
3. Pit: at heat 3 or 4 check one cop settles 4 m to a side and 2.7 m behind the centre, then sweeps across the rear.
4. Collapse: slow below the level's `CollapseSpeed` in sight of 3 or more cops: they take ring slots 3 to 6 m out at
   equal angles starting from the one nearest the front, then stop. A fast target breaks it.
5. Support: at heat 5 hold a 60 s pursuit and observe two `copsuvl`/`copsuv` that appear 340 m ahead, facing you.

## Open questions

- **Q1** `CROSS_BRAKE`, `E_BRAKE` and `COORDINATED_E_BRAKE` show no behavioural difference in the sources read. The PC
  build may have richer behaviour; test by observation.
- **Q2** The pseudo-angle function and the sorting are simple; ties between equidistant cops are broken by the C
  library sort (unstable). Not an issue for a reimplementation.
- **Q3** The finisher timer unit assumption: `TimeToFinisher` is in seconds with a per-second rate of at most 1;
  confirm by timing a box-in in the running game.
- **Q4** How `formation.Update` interacts with a running finisher for slot offsets already handed to cops: cops
  receive new offsets only at the next assignment, which runs every tick; they keep their in-position goal.

## Rust implementation notes

- Keep `Formation` as data (slot list plus a few parameters) with two update hooks: tightening and the herd lateral
  rewrite. The assignment is a pure function from `(candidate positions in target frame, slots)` to indices; unit-test
  it with small scenes.
- The finisher and collapse are a small state machine on the pursuit object; keep the exact branch order of section 5
  (the finisher-running branch takes precedence over the collapse branch).
- Support vehicles are normal cop cars with a `support_goal`; a flag on the pursuit's vehicle list is enough.
