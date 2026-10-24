# AI pursuit goals: pit, ram, box-in, rolling block and pull-over

The cop goals that drive a car at a slot around the player: the pit goal, the ram goal (box-in and rolling block),
and the pull-over goal (the collapse ring). The formation that picks a goal and arms the finisher is in
[ai-pursuit-formations.md](ai-pursuit-formations.md) (sections 3 to 5); the action list of each goal, the close chase and
the path-following chase are in [ai-pursuit-tactics.md](ai-pursuit-tactics.md) (sections 2 to 5); the cop car's
collision weights for pit and roadblock hits are in [ai-pursuit-cop-cars.md](ai-pursuit-cop-cars.md) (section 7).
The shared PID driver that turns "drive target and drive speed" into pedals is [ai-driver-control.md](ai-driver-control.md).

- **Sources read:** the install's `speed.exe` (PC build), Ghidra MCP, program `/speed.exe`, image base `0x400000`.
  Functions named below are the auto-names of that program. Addresses and method are in
  [docs/provenance/ai-pursuit-goals.md](../provenance/ai-pursuit-goals.md).
- **Data inputs:** the box-in and rolling-block tightness and duration per heat (`pursuitlevels`, read in
  [../formats/pursuit-data.md](../formats/pursuit-data.md)), `CollapseAggression`, `TopSpeedMultiplier`,
  `AccelerationMultiplier` (`pursuitlevels` and `aivehicle`), `MAXIMUM_AI_SPEED`.

Evidence tags as in the [docs README](../README.md#evidence-tags). **[decomp]** means read from the decompiled code of the
PC build (constants are the literal values in the code). **[verified]** means the value is also in the install's
AttribSys data or matches a known hash. **[unconfirmed]** means read but not checked against the running game, or the
decompile is garbled at that point. Units: metres, seconds, m/s inside the code; km/h only where the code writes it.

## 1. Goal kinds and the factory

A cop car holds one goal at a time. The goal object is built by a factory that switches on a small integer kept in the
car at offset `0x108` (word index 66). Each goal class has a fixed record table and a fixed formation size. [decomp]

| Kind | Class (constructor) | Formation | Max cops | Min cops | Finisher | Records |
|---|---|---|---|---|---|---|
| 1 | `AIGoalPit` (`0x0043fe90`) | Pit | 1 | 1 | yes | 2 |
| 2 | `AIGoalRam`, box-in (`0x0043f1a0`) | Box-in | 4 | 2 | yes | 4 |
| 3 | `AIGoalRam`, rolling block (`0x0043f520`) | Rolling block | 4 | 2 | yes | 5 |
| 4 and 5 | Follow goal (`0x0043f770`, same constructor for both) | Follow | 6 | 1 | no | 6 |
| 6 | Herd goal (`0x00440050`) | Herd | 3 | 1 | no | 3 |
| 7 | Stagger follow goal (`0x0043fb00`) | Stagger follow | 6 | 1 | no | 6 |
| not found | `AIGoalPullOver` | Collapse ring | | | | built from the ram class per [tactics spec](ai-pursuit-tactics.md) section 2; kind not identified |

- The **cop car constructor** (`0x00442d00`) sets kind 7 and builds that goal, so a cop starts as a stagger-follow
  cop and the formation code moves it to the others. [decomp]
- The **Pit** and **Ram** names are the goal names; their name hashes (the `AttribSys`-style hash, section 8) are
  `AIGoalPit` `0xEC619CB2` (also in [ai-driver-control.md](../provenance/ai-driver-control.md)) and `AIGoalRam`
  `0x458F733B`. [verified][decomp]
- The **Follow** table and the **Stagger** table are the same numbers as the formation spec's slot tables, which
  confirms the factory kinds match the formations there. [verified against the formation spec]
- Kinds 4 and 5 share a constructor; the difference between them is not visible in the code read. [unconfirmed]
- Kind 6 (herd) and kind 7 (stagger) have no in-position goal in the formation spec, so their records are only the
  pursuit slots (section 2).

## 2. Slot records

Each goal holds a list of **records**. A record is 32 bytes: two 3-vectors, a priority, and the name hash of the goal
that owns it. The two vectors are:

- **A, the pursuit slot.** Where the cop wants to sit in the target's frame while it chases in formation (the
  formation's "pursuit offset", [ai-pursuit-tactics.md](ai-pursuit-tactics.md) section 3.3). Frame: `x` is to the target's
  right, `z` is forward. [decomp]
- **B, the in-position offset.** Where the cop goes once the finisher is armed. The ram action steers to B
  (section 5). The code keeps the cop's current in-position offset as `x` and `z` only. [decomp, the assignment
  "B is in-position" follows the formation spec's convention; the record field that feeds the action is read through
  a helper whose decompile is partly garbled, so [unconfirmed] for the exact field]

The priority (the fourth field, `1` to `4`) is the "pri" column of the formation spec: the cops are matched to slots
in priority order. [decomp]

### 2.1 Box-in (kind 2)

Two tightness values feed this goal: `t` = `BoxinTightness`, and `d` = `BoxinDuration` (the finisher run, section 3).
Their defaults in the code are `t = 0.5` and `d = 2.0` when the pursuit level has no row. [decomp]

From `t`, two scale factors: `s = 0.7 - 0.5 t` (wings and rear) and `foff = 2 - 5 t` (the front in-position depth).

| Record | Priority | A (pursuit slot) | B (in-position) |
|---|---|---|---|
| front | 1 | `(0, +14)` | `(0, foff)` |
| left wing | 2 | `(-3.5, 0)` | `(-3.5 s, 0)` |
| right wing | 2 | `(+3.5, 0)` | `(+3.5 s, 0)` |
| rear | 4 | `(0, -7.5)` | `(0, -7.5 s)` |

Values at the tightness of each heat (from the `box` column of [pursuit-data](../formats/pursuit-data.md), **[verified]**
that the field is `BoxinTightness`):

| Heat | t | s | foff | Wing B x | Rear B z |
|---|---|---|---|---|---|
| 1 | 0.5 | 0.45 | -0.5 | +-1.58 | -3.38 |
| 2 | 0.6 | 0.40 | -1.0 | +-1.40 | -3.00 |
| 3 | 0.8 | 0.30 | -2.0 | +-1.05 | -2.25 |
| 4 | 0.9 | 0.25 | -2.5 | +-0.88 | -1.88 |
| 5 to 7 | 1.0 | 0.20 | -3.0 | +-0.70 | -1.50 |

Heat 8 to 10 use `t = 0.5` and the race levels likewise (the `box` column). The tighter `t` is, the closer the cops
close: at `t = 1` the front cop aims 3 m behind the target's centre and the wings 0.7 m out. [decomp][verified]

### 2.2 Rolling block (kind 3)

Feeds: `t` = `RollingBlockTightness` (default 0.5), `d` = `RollingBlockDuration` (default 2.0). [decomp][verified names]

From `t`: `s' = 1 - 0.8 t` (lateral scale of the in-position offsets) and the same `foff = 2 - 5 t` depth.

Five records, all with the pursuit slot 14 m ahead of the target's centre [decomp]:

| Record | Priority | A (pursuit slot) | B (in-position) |
|---|---|---|---|
| centre | 1 | `(0, 14)` | `(0, foff)` |
| inner left | 2 | `(-2.5, 14)` | `(-2.5 s', foff)` |
| inner right | 2 | `(+2.5, 14)` | `(+2.5 s', foff)` |
| outer left | 3 | `(-5, 14)` | `(-5 s', foff)` |
| outer right | 3 | `(+5, 14)` | `(+5 s', foff)` |

At `t = 0.5` the in-position lateral offsets are `+-1.5` and `+-3.0` and the depth is `-0.5`. At `t = 1` they are `+-0.5`,
`+-1.0` and `-3.0`. The formation spec lists the inner right slot as `(2.5, 10)`; the table in memory has `14` there
(see section 10). [decomp]

### 2.3 Pit (kind 1)

Two records, one per side of the target, both priority 1 [decomp]:

| Record | A (pursuit slot) | B (in-position) |
|---|---|---|
| left | `(+4.0, -2.7)` | `(-10.0, -2.7)` |
| right | `(-4.0, -2.7)` | `(+10.0, -2.7)` |

So a pit cop sits 4 m beside the target's rear quarter on its side, and its in-position point is 10 m out on the
**opposite** side, still 2.7 m behind. Driving at that point sweeps the cop's nose across the target's rear. The pit goal
has no tightness or duration of its own (the finisher timers are in section 3). [decomp][verified against formation spec]

### 2.4 The slot in the target frame

Every slot is turned into the world with the target's frame, which is the same for all goals [decomp]:

```
fwd  = target velocity, if the target moves at 5 km/h (1.389 m/s) or more; else the target's body heading
       (horizontal, normalised)
side = (fwd.z, 0, -fwd.x)
world offset = side * x + fwd * z        (y is not used for the slot)
slot point   = target position + world offset
```

The `5 km/h` test is the one the code uses to pick between velocity and heading. The slot point is then the place the
goal steers to, before the lead term of section 4.

## 3. When a pit or ram goal is set (the finisher)

The formation code (formation spec section 5) runs an **in-position timer** while cops hold their pursuit slots. When it
arms, it calls `SetGoal(in-position goal)` on each formation cop with an in-position goal. The goal kind is the formation's
kind from section 1. The pieces this spec relies on, restated: [decomp, from the formation spec]

| Formation | Tolerance | Tolerance rate (4 x) | Time to finisher | Finisher run | Min cops |
|---|---|---|---|---|---|
| Pit | 0.5 | 2 m | 1.2 s | 2.0 s | 1 |
| Box-in | 1.0 | 4 m | 4.0 s | `BoxinDuration` (3 to 10 s by heat) | 2 |
| Rolling block | 1.0 | 4 m | 4.0 s | `RollingBlockDuration` (3 to 10 s by heat) | 2 |

- **Timer.** The in-position timer rises at 1 per second while the average distance of the in-position cops to their slots
  is under the rate, falls to 0 at twice the rate, and drains at up to 1 per second beyond three times the rate. It is
  armed at `TimeToFinisher`. [decomp]
- **Run.** The finisher runs for `FinisherTime` seconds. Each cop with an in-position goal drives to its in-position
  point (section 5 for box-in and rolling, section 4 for pit). Other cops keep chasing. [decomp]
- **End.** When the finisher's run time is up, the formation resets: every cop goes back to `AIGoalPursuit` and the next
  formation is chosen when its timer runs out. The finisher also ends early when a roadblock arrives (`EndCurrentFormation`).
  [decomp]
- **Which cops.** Each formation cop is matched to a slot by the formation's greedy assignment (formation spec section 3).
  The pit formation has one slot, so one cop takes the pit goal. [decomp, formation spec]

**Pit gating by heat and distance** is not in this function: the choice of formation (and so of the pit goal) is a
weighted random pick of the `CopFormations` record of the current heat row, plus the support-delay rule. Those data are not
read here. The rewrite's own gate (pit from heat 4, alongside the player) is in [police-tactics.md](police-tactics.md) and is
**[unconfirmed]**. [unconfirmed]

## 4. Pit goal driving

The pit goal runs the same action as the ram goal (action `AIActionRam`, section 5 for the body). The only difference is
the record it steers to. [decomp]

1. **Target point.** The in-position point of section 2.4 with B: the point 10 m across the target's rear on the side
   opposite the cop. (The cop on the left side aims 10 m to the right of the target, 2.7 m behind.)
2. **Lead.** The lead term of section 5.2 is added, so the aim moves with the target's velocity.
3. **Speed.** Desired speed is the length of the steering vector, capped by the speed ceiling (section 6). The ceiling
   has the pursuit multipliers (jerk 1.2 / 1.5 / 1.1). There is no pit-specific speed. [decomp]
4. **Steering.** The shared driver steers to the drive target = cop position + steering vector. Nitrous off. [decomp]
5. **Success.** None in the goal. The goal ends when the finisher run ends (`FinisherTime` 2.0 s). The hit itself is the
   cop-car collision set `coppit` (front elasticity 0.4, mass scale 4.0), which makes the cop heavy when it hits the
   player's car. [decomp][verified, cop-cars spec section 7]
6. **Abort.** Roadblock arrival or the formation reset (section 3). No distance or contact abort. [decomp]

## 5. Ram goal driving (box-in and rolling block)

### 5.1 Target point

The record's in-position offset B gives the slot point (section 2.4). For a ram cop the slot point is the aim:

```
slot  = target position + side * B.x + fwd * B.z            (section 2.4)
seek  = (lead vector)  computed from slot - cop position and the target's velocity (section 5.2)
steer = seek, plus the separation term (pull-over only, section 7)
```

### 5.2 Lead term

The aim is pushed along the target's motion so the cop arrives where the car will be. With `r = slot - cop position` and
`v` the target's actual velocity, the shared ram action computes [decomp; reconstructed
`AIActionRam::UpdateSeek` and `AISteer::Pursuit`]:

```
seek = r + v + v * 0.7 * dot(r, v) / |v|^2
                 ^ the final projection term is used only when |v| > 0.01
```

This is the "seek" of [ai-pursuit-tactics.md](ai-pursuit-tactics.md) section 5.1, written as one formula. When `r` lies
ahead of the target along `v` (`dot > 0`) the lead is longer; when `r` is behind, the lead shrinks. The guard is on the
length of the actual velocity. Slot orientation still uses the velocity-or-heading frame in section 2.4.
A stopped target contributes no prediction: `seek = r`. Below the projection guard, only the raw one-second
velocity term remains. These are separate inputs; heading must never become predictive velocity. [decomp]

### 5.3 Speed and steering

The steering vector is the seek with the separation term (pull-over only). The desired speed is the length of the steering
vector, capped by the limiter (section 6). Then:

- `SetDriveSpeed(desired)`, `SetDriveTarget(cop position + steering vector)`, `DoDriving(7)`, nitrous off. [decomp]

### 5.4 Success and timeouts

- The goal never finishes by itself. The finisher's run time (`FinisherTime`: `BoxinDuration` or
  `RollingBlockDuration`, 4 to 10 s by heat) ends it, and then the formation resets to pursuit. [decomp]
- No contact timer, no abort distance, no steering gain here. Steering gains are the shared driver's (driver spec).
- Duration values by heat (the `box` and `roll` columns of [pursuit-data](../formats/pursuit-data.md)): 4, 5, 6, 7, 8, 10, 10
  on heats 1 to 7 and 3 on heats 8 to 10. [verified]

## 6. Speed ceiling and the limiter (ram, pit and pull-over)

Computed each think from the target and the cop's own tuning [decomp]:

| Quantity | Value |
|---|---|
| cop top speed (`MAXIMUM_AI_SPEED`) | from the cop's `aivehicle` row, km/h, converted by 1/3.6 |
| target top speed | the target's own top speed (its computed AI model) |
| speed multiplier `TopSpeedMultiplier` | cop's `aivehicle` row |
| accel multiplier `AccelerationMultiplier` | cop's `aivehicle` row |
| speed ceiling | `min(MAXIMUM_AI_SPEED, target top speed x TopSpeedMultiplier)` |
| acceleration | target's acceleration table evaluated at the cop's speed, times `AccelerationMultiplier` |
| jerk pursuit | ceiling x 1.1, speed multiplier x 1.2, acceleration multiplier x 1.5 |

The limiter (the same smoothed limit as tactics section 3.3) takes the cop speed, the ceiling and the acceleration, and
gives the speed limit; the desired speed is `min(|steering|, limiter)`. [decomp]

The code does **not** show the nitrous 0.2 factor or the `-9.81 * forward.y` grade term that
[ai-pursuit-tactics.md](ai-pursuit-tactics.md) section 5.1 lists for this ceiling. They may sit inside the shared limiter; not
checked. [unconfirmed]

## 7. Pull-over goal (the collapse)

The pull-over goal is the ram action with the goal **name** `AIGoalPullOver` (name hash `0x63472E0A`). The action
checks the name, and when it matches it adds the collapse rules. [decomp]

### 7.1 Aggression and separation

- `aggression` = the target's `CollapseAggression` (heat 1 to 7: 0.4, 0.5, 0.6, 0.7, 0.8, 0.8, 0.5; 0 elsewhere,
  [formation spec section 5.1](ai-pursuit-formations.md)), **[verified]** that the field is `CollapseAggression`. The code
  reads 0 when the row is missing. A jerk target forces 1.0. [decomp]
- `staticavoid = (1 - aggression) * 4`, `dynamicavoid = staticavoid + 1.5`; the separation vector with those two
  numbers is added to the steering vector. [decomp]
- The opposing-component weight is `clamp((55 km/h - counter) / 55 km/h, 0, 1)`. The chase uses 35 km/h minus the counter, over
  25 km/h ([tactics spec](ai-pursuit-tactics.md) section 3.3).
  The steering keeps the component of the seek along the separation only when `counter > 0.0001`. [decomp]

### 7.2 Arrival

The cop is "arrived" when the seek length `|seek|` (section 5.2, before separation) is under

```
threshold = (1 - aggression) * 0.2 km/h + 0.5 km/h        (in m/s: 0.0556 (1 - a) + 0.1389)
```

When arrived: gas 0, brake 1, handbrake 1, steering 0, vertical steering 0, nitrous off, and no drive target is
set. The cop stays there until the collapse ends (formation section 5.1). [decomp]

### 7.3 Going round the target

If the straight line from the cop to its slot passes within `r` of the target, the cop drives round the target's circle
instead. Here `r = min(|slot - target|, |cop - target|) - 0.2` m, and the detour is used only when `r > 0.1` m. The helper
returns two tangent points; the one nearer to the slot is taken. [decomp]

[unconfirmed] The tangent length. [ai-pursuit-tactics.md](ai-pursuit-tactics.md) section 5.1 says the tangent length is
`2.1 x |slot - target|`. The code scales one vector by a float argument that the decompiler could not name, and a second by
the helper's output. The factor 2.1 is not seen. Check this before relying on the detour's shape.

## 8. Attribute and name hashes

All hashes are the AttribSys `lookup2` hash with initval `0xABCDEF00` (the same function as `vlt_hash` in
`libs/blackbox-attrib`; the test vector `pvehicle` = `0x4A97EC8F` agrees). [verified]

| Hash | Name | Where |
|---|---|---|
| `0x67A15750` | `BoxinTightness` | box-in ctor, default 0.5 [verified by hash match] |
| `0x858B1097` | `BoxinDuration` | box-in ctor, default 2.0, stored only [verified by hash match] |
| `0xC146FC03` | `RollingBlockTightness` | rolling ctor, default 0.5 [verified by hash match] |
| `0x5C9F5F55` | `RollingBlockDuration` | rolling ctor, default 2.0 [verified by hash match] |
| `0x594E1492` | `CollapseAggression` | pull-over, from the target's pursuit level [verified by hash match] |
| `0xEC57E16B` | `TopSpeedMultiplier` | ceiling, from the cop's `aivehicle` [verified by hash match] |
| `0xCC320329` | `AccelerationMultiplier` | acceleration, from the cop's `aivehicle` [verified by hash match] |
| `0x458F733B` | goal name `AIGoalRam` | record name field |
| `0xEC619CB2` | goal name `AIGoalPit` | pit record name field [verified against provenance of ai-driver-control] |
| `0x63472E0A` | goal name `AIGoalPullOver` | action name test |

## 9. Constants (all **[decomp]** unless tagged)

| Name | Value | Meaning |
|---|---|---|
| Box-in front slot | z = +14 m | pursuit slot of the front cop |
| Box-in wings | x = +-3.5 m | pursuit slot of the wing cops |
| Box-in rear | z = -7.5 m | pursuit slot of the rear cop |
| Rolling block row | z = 14 m; x = 0, +-2.5, +-5 | pursuit slots, five cops |
| Pit slot | x = +-4.0 m, z = -2.7 m | pursuit slot beside the rear quarter |
| Pit in-position | x = -+10.0 m, z = -2.7 m | aim point across the rear |
| Front depth | foff = 2 - 5 t | in-position z of the front (box) and centre (rolling) |
| Wing / rear scale | s = 0.7 - 0.5 t | box-in in-position scale |
| Rolling scale | s' = 1 - 0.8 t | rolling in-position lateral scale |
| Default t | 0.5 | when a row is missing [decomp] |
| Default duration | 2.0 s | stored only [decomp] |
| Lead gate | |v| > 0.01 | lead term on/off |
| Lead coefficient | 0.7 | in the lead term |
| Pull-over arrival | 0.5 km/h + (1 - a) 0.2 km/h | seek length to stop |
| Separation scale | 55 km/h | pull-over opposing-component weight |
| Detour gate | r > 0.1 m; r = min(d) - 0.2 | pull-over circle |
| Jerk multipliers | speed 1.2, acceleration 1.5, ceiling 1.1 | jerk pursuit |
| Separation static / dynamic | 4 (1 - a), +1.5 | pull-over avoidance |
| Finisher (pit) | tolerance 0.5 (2 m rate), 1.2 s to arm, 2.0 s run | formation spec |
| Finisher (box, rolling) | tolerance 1.0 (4 m rate), 4.0 s to arm, duration by heat | formation spec |

## 10. Discrepancies with the other specs (for their owners)

These are not changed here; the owners should check them.

1. **Formation spec, rolling block row.** The table lists the inner right slot as `(2.5, 10)`. The code's table has
   `z = 14` for every rolling slot. [decomp] Check the formation spec's `(2.5, 10)`.
2. **Tactics spec 5.1, the ceiling.** The tactics spec says the ram ceiling uses the nitrous 0.2 factor and the grade term.
   The code of this action does not show them (section 6). [unconfirmed] Check whether the limiter adds them.
3. **Tactics spec 5.1, the tangent.** The factor `2.1 x |slot - target|` is not seen (section 7.3). [unconfirmed]
4. **Tactics spec 5.1, the arrival test.** The tactics spec tests `|seek|`. The code also tests `|seek|` (before
   separation). Consistent. [decomp]
5. **Formation spec, slot priority.** The priority column is the record's fourth field. Consistent. [decomp]

## 11. How to check it

1. **Box-in tightness.** At heat 5 (`t = 1`) the wing cops should close to within about 0.7 m of the target's side in the
   finisher, and the rear cop to 1.5 m behind. At heat 1 (`t = 0.5`) they stay about 1.6 m out at the wings. [decomp]
2. **Rolling block.** Five cops form a row 14 m ahead of the player, then close to the in-position offsets. The two inner
   cops at heat 1 should sit about 1.5 m either side of the player's line.
3. **Pit.** One cop goes beside the player's rear quarter, 4 m out and 2.7 m behind, then sweeps across the rear. The
   finisher lasts 2.0 s.
4. **Pull-over.** A slow player (under the heat's collapse speed) gets a ring; the cops stop on their ring slots with
   the handbrake on. The arrival threshold is 0.54 km/h at heat 5 (aggression 0.8) and 0.62 km/h at heat 1 (aggression
   0.4): at higher aggression the cops must come closer to their ring slot before they stop.
5. **Lead.** A target moving fast ahead of the cop should be led: the cop's aim should be ahead of the target's current
   position, not at it.

## Open questions

- **Q1** The pull-over goal's kind number, and the factory case that builds it. Not found (section 1). [unconfirmed]
- **Q2** Which of the two record vectors the action steers to in every case. Read as B, [unconfirmed] from the helper's
  partly garbled decompile.
- **Q3** The tangent length in the pull-over detour (section 7.3).
- **Q4** Whether the shared limiter adds nitrous and grade terms to the ram ceiling (section 6).
- **Q5** Kinds 4 and 5 share a constructor; their difference is not visible here.
- **Q6** The formation's kind write: the setter that changes the car's kind at `0x108` is not found. Its caller is an indirect
  call. [unconfirmed]
- **Q7** The speech line `intenttoram` (collection `AnytimeEvents_IntentToRam`, id 113, see
  [ai-pursuit-speech-events.md](ai-pursuit-speech-events.md)) is not tied to a call site in the code read. Its trigger is unknown.

## Rust implementation notes

- A `GoalKind` enum (Pit, BoxIn, Rolling, Follow, Herd, StaggerFollow, PullOver) with a fixed record table per kind
  (`[Record; N]`, record = pursuit slot, in-position offset, priority). Keep the tightness and duration as inputs from the
  heat row, not constants.
- The slot point, lead term, speed ceiling and limiter are pure functions of positions, velocities and the two
  `aivehicle` multipliers; unit-test them with a stopped target, a moving target and a target ahead of the cop.
- Keep the pull-over checks (name, aggression, arrival threshold, detour gate) in one place so that the ring code can call
  them with `GoalKind::PullOver`.

## Port status

The goal rules are in `libs/blackbox-pursuit/src/goals/`. The world drives them in `crates/nfsmw/src/scenes/world/ai/`:
`goal_aims.rs` (the mapping, the target and vehicle values, one step of a goal), `cop_aims.rs` (which cop takes which
goal) and `car/goal_drive.rs` (the per-cop limiter and command, the speed override and the full brake).

- **Finisher goals.** While the finisher runs, each cop that holds a formation slot takes the goal of its formation: pit,
  box-in or rolling block. Its aim is the in-position point of the slot's record. Other formations give no goal.
- **Pull-over.** While the collapse is on (a formation is set and the player is under the collapse speed), cops in ring
  slots take the pull-over goal. The aim is the detour point or the ring slot; an arrived cop holds the full brake.
- **Other cops** keep the chase action's velocity request. Reaching a moving
  formation slot does not trigger a brake or handbrake hold.

Integration now supplies the player's actual performance and the pursuit's jerk flag.
Heat-specific tightness and duration use the install-verified free-roam values.
Pull-over separation reads the live vehicle capsules. Eligible goals consume
`GoalCommand::target_point` as the physical driver's steering target. Their limiter
and command update with the cop think, retaining state between thinks. Blocked
target or drive-nav rays keep the road action in control.

Remaining limits:

- Alternate event-table goal tuning still requires install-level confirmation.
- The separation adapter exposes horizontal headings, so pitched and rolled
  capsule axes remain approximate.
- The tangent shape and missing ram-grade/NOS terms retain
  the open questions documented above; close-action terms must not silently be
  added to the ram action without evidence.
- No matched original-PC route establishes physical PIT, box or collapse parity.

See [driving integration](ai-pursuit-driving-integration.md) for the history,
drivability and road-adapter acceptance boundaries.

## Next leads

1. The setter that writes the car's goal kind (`0x108`): find the indirect call that reaches the goal factory
   `0x00440330`, probably through a vtable slot of the cop class. The cop class's own vtable is not at `0x8928EC`
   (that is a three-slot secondary table followed by RTTI text).
2. The pull-over kind: list the factory's `switch` cases and look at the name hashes the caller passes when it builds a
   pull-over goal (the collapse caller of the formation code).
3. The tangent helper `0x00767400` and the float argument of `0x004135f0` (its fourth parameter): name the argument from
   the caller, then resolve the `2.1` question.
4. The shared limiter `0x0040A5E0`: check whether it adds the nitrous and grade terms.
5. The rolling-block slot table at `0x890E58` to `0x890E98`: confirm the inner right entry is 14 (one more read of the same
   loop bound) and find the owner of the four unused floats after it (`-2.0` and `4.0` at `0x890E94` and `0x890E98`).
6. The speech trigger for `intenttoram`: find the code that reads the collection-name table around `0x901E5C`.
