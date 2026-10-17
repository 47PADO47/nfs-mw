# Pursuit support: heavy SUVs (ram, roadblock) and the cross cars

How a running pursuit gets its support cars: the request record, the manager that executes it, which cars are taken
from the pool, where they are placed, how the head-on ram SUVs drive, and how the cross ("leader") cars trail the
target. The roll that picks a request, its gates and its per-level table are in
[ai-pursuit-formations.md](ai-pursuit-formations.md) section 6. The roadblock that a `HEAVY_ROADBLOCK` request builds is
in [ai-pursuit-roadblocks.md](ai-pursuit-roadblocks.md). The per-cop driving of the ram SUV is in
[ai-pursuit-tactics.md](ai-pursuit-tactics.md) section 5.2.

- **Sources:** speed.exe (the PC binary, the Ghidra program `/speed.exe`), read with the Ghidra MCP tools. Addresses and
  the method are in [../provenance/ai-pursuit.md](../provenance/ai-pursuit.md). Nothing was copied from source code.
- **Data:** the per-level `pursuitsupport` rows (strategy, chance, duration, `ChanceBigSUV`, leader priority) are the
  install's, read by the spec's earlier pass ([../formats/pursuit-data.md](../formats/pursuit-data.md)).

Evidence tags as in the [docs README](../README.md#evidence-tags). Everything here is **[decomp]** from the binary
unless marked **[verified]** (install data) or **[unconfirmed]**. Distances are metres, times seconds, speeds m/s
unless written otherwise.

## 1. Objects

- **Pursuit** (`AIPursuit`): one per chase. Its support request is read through the pursuit's interface slot `+0x44`
  (the manager calls it every tick). The pursuit's own tasks run at 0.25 s (main) and 1.0 s (busted integration), and
  it starts with `time since any cop saw the target = -5`, the support check timer at 10 s and the close-chase range
  at 60 m. **[decomp]**
- **Cop manager** (`AICopManager`): one per world. Each tick it walks the pursuit list and, for each pursuit, runs the
  support executor (section 3), then the cop spawn and wave logic. It keeps a shared **support timer** (its field
  `+0xb0`, seconds, used by the ram delay) and two pending-spawn fields (`+0xb8` count, `+0xbc` pursuit). **[decomp]**
- **Request record** (32 dwords, one per pursuit, filled by the roll):

| Index (dword) | Byte offset | Field |
|---|---|---|
| 0 | `+0x00` | heavy option pointer (its first field is the strategy); **null** for a leader request |
| 1 | `+0x04` | leader option pointer (its first field is the strategy) |
| 2 | `+0x08` | the car list: allocator |
| 3 | `+0x0c` | the car list: storage pointer (one pursuit pointer per acquired car) |
| 4 | `+0x10` | the car list: capacity |
| 5 | `+0x14` | the car list: count |
| 29 | `+0x74` | timer: the option's `Duration` at activation, then counted down |
| 30 | `+0x78` | state: **1 = pending**, **2 = active** |

The heavy option row has the fields strategy (`+0x00`), duration (`+0x08`) and `ChanceBigSUV` (`+0x0c`); the other
offsets were not needed.
The strategy codes are 1 `E_BRAKE`, 2 `COORDINATED_E_BRAKE`, 3 `RAM`, 4 `HEAVY_ROADBLOCK`. **[decomp]** The leader
option uses strategy 5 `CROSS_FOLLOW`, 6 `CROSS_BRAKE`, 7 `CROSS_PLUS_V_BLOCK`. **[decomp]** The duration is read
from `option + 8` for both kinds. **[decomp]**

## 2. Who asks

The roll (formations spec section 6) sets a pending request (state 1) with the chosen option and its duration. The
roll is the only writer of state 1 found in the binary's manager code. The manager does nothing with a request that is
not pending. **[decomp]**

## 3. The executor (`UpdateSupportCops`, per pursuit, per manager tick)

```
req = pursuit.support_request()
if req is null or req.state != PENDING: return
if req.heavy_option == null:                       # leader request
    if not acquire_cross_cars(req): return          # (section 5) not all cars yet: retry next tick, no message
    if not place_cross_cars(pursuit, req):          # (section 6)
        send message "ReqDenied"; return
    send message "Request"                          # the pursuit now has the cross cars
    # state set to ACTIVE inside place_cross_cars, timer = leader duration
    return
strategy = req.heavy_option.strategy
if strategy == HEAVY_ROADBLOCK:
    if the pursuit's roadblock check (interface slot +0x3c) is already true: return
    ok = build_roadblock(manager, pursuit, count 4, req.cars)     # the roadblock spec
    ...                                                          # see the note in section 9
    return
if not acquire_heavy_cars(req): return             # section 4: not all cars yet
if not place_heavy_cars(pursuit, req):             # section 7
    send message "ReqDenied"; return
# state set to ACTIVE inside place_heavy_cars, timer = heavy duration
```

Notes: the executor only runs once the acquisition of all cars is complete, so the cars are taken one tick at a time
and placed on a later tick. A request that cannot be placed stays pending with its cars already in its list; the code
sends `ReqDenied` on each failed try. **[decomp]** Whether the cars are ever given back to the pool in that case is
not seen (section 10).

## 4. Heavy acquisition (`AcquireHeavyCars`, `0x0041f130` in the binary)

- **Count per strategy:** `E_BRAKE` 1 car, `COORDINATED_E_BRAKE` 2, `RAM` 2, `HEAVY_ROADBLOCK` 4. Cars already in the
  request's list count, so the loop takes only the missing ones. **[decomp]**
- **Which car, per car:** a percentage roll `r = (random16 * 100) >> 16` (0 to 99) is compared with `ChanceBigSUV`
  (option `+0x0c`). If `r < ChanceBigSUV` the car is **`copsuv`** (the big SUV), otherwise **`copsuvl`**. The roll is
  made once per car. **[decomp]** (the disassembly at `0041f1b0` to `0041f1c3`).
- **Where the car comes from:** the manager's car finder (section 8). The car is then stored in the request's list.
- **Goal:** each car gets its support goal by the goal name's hash through the car's AI interface (slot `+0x84`):
  `E_BRAKE` and `COORDINATED_E_BRAKE` get `AIGoalPursuit`; `RAM` gets `AIGoalHeadOnRam`; `HEAVY_ROADBLOCK` gets
  `AIGoalStaticRoadBlock`. **[decomp]** (the name table at `0x8eb1b0` to `0x8eb1d0`).
- **Ram delay:** for `RAM`, every car taken sets the manager's shared support timer to **3.0 s**. The message
  `Request` is sent when the last car of the request is taken. **[decomp]**
- **Done:** the function returns true only when every car is taken **and** the support timer is below zero. For the
  non-ram strategies the timer is already negative, so the request is placed on the tick after the last car. For
  `RAM`, placement waits until the 3.0 s countdown has run out. **[decomp]** The countdown is decremented somewhere else;
  not located (section 10).

## 5. Leader acquisition (`AcquireCrossCars`, `0x0041f480`)

- **Count:** strategy 5 (`CROSS_FOLLOW`) 1 car, 6 (`CROSS_BRAKE`) 1 car, 7 (`CROSS_PLUS_V_BLOCK`) 3 cars. **[decomp]**
- **Cars:** the first is **`copcross`**. For strategy 7 the next two are **`copsporthench`**. **[decomp]**
- **Goal:** `AIGoalPursuit` for each car (name table `0x8eb1d4` to `0x8eb1e4`). **[decomp]**
- **Slot offsets** (the pursuit offsets, target frame, metres, in a table of three `(x, 0, z)` entries with a 16-byte
  stride): `(0, 0, -20)`, `(-4, 0, -24)`, `(4, 0, -24)`. The first car takes `(0, -20)`; the two `sporthench` cars take
  the other two. **[decomp]** (how the index reaches the car is not clear in the decompile; the table is read from the
  constants). The table matches the earlier spec's offsets.
- Each car is taken with the same finder as section 4. Acquisition returns false until every car is taken; no message.

## 6. Leader placement (`PlaceCrossCars`, `0x0041f650`)

- The search for a spawn point uses the same routine as section 7 with **negative** distances: `-(150 + 10 k)` for
  `k = 0` to `7` (150 to 220 m **behind** the target). The `k` counter is not reset between the cars of one request,
  as for the heavy cars. **[decomp]**
- Each car: its AI is placed at the spawn point with its heading reversed, the pursuit adds it (pursuit slot `+0x78`,
  so the car joins the chase), its goal is `AIGoalPursuit` and it is activated; the manager's cop count (`+0x94`) rises
  by one per car. **[decomp]**
- After the last car: request state **2 (active)** and timer = `option + 8` (the leader duration, 240 to 2000 s in the
  install **[verified]**). Returns true.
- If any spawn search fails (8 tries), the function returns false and the message `ReqDenied` is sent by the caller.

## 7. Heavy placement (`PlaceHeavyCars`, `0x0041f2e0`)

- For each car in the request's list the spawn search (section 8) is run with distance `400 - (60 + 10 k)` for
  `k = 0` to `7`: **340, 330, ..., 270 m ahead** of the target. The counter `k` (the 60 start value) is **not reset
  between the cars**: the second car's first try is at the distance the first car ended on. **[decomp]**
- Each car is placed at its point with its direction **negated** (the road direction at the point points away from the
  target, so the negation makes the car face the target). Then it is activated, added to the pursuit (slot `+0x78`),
  and the manager's cop count (`+0x94`) rises by one per car. **[decomp]**
- Request state **2** and timer = `option + 8` (heavy duration, 20 to 40 s **[verified]**). Returns true.
- Note: the goal of each car was set in section 4, before the car was placed. `AIGoalHeadOnRam` is therefore in force
  from the moment the car is placed.

## 8. Spawn point search and the car finder

- **Search** (`FindSpawnPoint`, `0x0041eed0`): takes the pursuit's target, reads its nav point at the requested
  signed distance along the cop road (the target's own road, the spawn-manager route call `0x0077bba0`), and returns
  the point and the road direction. A negative distance flips the start direction first. **[decomp]**
- **Validity** (`CheckSpawnPoint`, `0x00419fc0`): a spawn point is refused when a car is too close to it (the thresholds
  are 5, 15 and 40 metres, chosen by the kind of car; the unit of the distance returned by the helper was not settled, so
  these are **[unconfirmed]**), or when it lies in a no-cop-spawn zone (a world query for zone type 13, **[decomp]**). The
  search also applies a distance window when the mode flag is set: the spawn manager's minimum and maximum (its fields
  `+4` and `+8`, **[unconfirmed]** values; the spec's 150 and 400 m match the constants `150.0` and `400.0` at
  `0x8eb1a8` and `0x8eb1ac`, which the placement code uses directly).
- **Car finder** (`GetSupportCar`, `0x0041ecd0`): a car already in the manager's list is reused when it is inactive and
  its name hash matches the requested name (`copsuv`, `copsuvl`, `copcross`, `copsporthench`); otherwise a new car is
  built from the car record of that name and a sound message is sent. A budget check (`manager+0x54`, vtable `+4`)
  comes first; when it refuses, no car is returned. **[decomp]** (what the budget check tests is not settled.)

## 9. The head-on ram SUV (`AIGoalHeadOnRam`)

**Goal.** Built by the goal factory from the name hash (`0x0043d780`, constructor `0x0043bd10`). Its action list is,
in order: `AIActionHeadOnRam`, `AIActionRace`, `AIActionTooDamaged`, `AIActionGetUnstuck`, `AIActionAirborne`. The
constructor also writes the owner's in-position offset `(0, 0, -3)` (through the AI interface slot `+0x50`, **[decomp]**,
the setter's name not confirmed). Its goal score list follows the tactics spec table. **[decomp]**

**Action.** `AIActionHeadOnRam` (constructor `0x00421360`, score **0.1** = the float `0x3dcccccd`, primary vtable
`PTR_0x008913b8`):

- **Start** (`0x004214f0`, run once when the car has just been placed): if the car's spawn timer is at zero, its speed
  is set to `min(26.82 m/s (60 mph), GetDesiredSpeedToTarget(distance, target speed))`. **[decomp]**
- **Update** (`0x00421560`, the slot the tactics spec calls `Update`), every think:
  1. `me` = the car's position; `target` = the pursuit's target; `tv` = target velocity (zero when the target has no
     vehicle AI); `tp` = target position; `v` = own speed; `vel` = own velocity.
  2. The intercept (`0x00418200`, the `Ram` formula of the tactics spec section 5.2) gives `d = unit(intercept - me) * v`.
  3. Drive speed is set to the constant `0x41DE38E4` = **27.78 m/s (100 km/h)**, fixed. **[decomp]** (the decode of the
     float was checked by hand.)
  4. Drive target = `me + d`. `DoDriving(7)`. No road following, no avoidance.
- **Finish:** the action never finishes while the target exists (the tactics spec; the finish slot was not re-read here).

**Targeting rule, in words.** The SUV does not follow the road to the target. It aims at the point where it will
meet the target if both keep their velocities (the quadratic in the tactics spec), at 100 km/h. The lateral
interception is the only steering; it is not limited by the cop's own top speed. **[decomp]** (the tactics spec
says the same).

**Not in the sources read:** how the SUV ends when it has not met the target (the request timer, section 11).

## 10. Support cars in the chase

- A support car has a non-null support goal. The formation assignment skips support cars entirely
  (formations spec section 3). **[decomp]** Heavy and cross cars are therefore never given a formation slot.
- The leader cars (the `copcross` pursuit offset) hold a fixed position in the target frame, 20 m behind the target's
  centre (24 m for the two V cars, 4 m to either side). Because they use the close chase (tactics spec section 3,
  seek point = centre + heading * offset.z + side * offset.x), they follow the target's road at that spacing. **Nothing
  else is needed to make them follow.** The other cops do not follow the cross car: they take their own formation slots
  or the out-of-formation grid, with no reference to it. **[decomp]**

## 11. Timers, resets and the flee

- Request timer (`+0x74`) counts down while the request is active. When it reaches zero the request is reset and each of
  its cars gets its support goal cleared and goes back to the pursuit as an ordinary cop (formations spec section 6).
  **[spec; the countdown and the reset code were not located, see section 13.]**
- Flee: when a collapse begins or the perpetrator is out of sight while a heavy support is active, support cars are
  told to flee (`AIGoalFleePursuit`, pursuit spec section 7). **[spec]**
- The manager's tick, for a pursuit that has ended, walks the pursuit's cars and clears the support goal of each car
  that belongs to it; the manager then detaches the pursuit. **[decomp, partial: the exact order was not traced.]**

## 12. Constants

| Name | Value | Where |
|---|---|---|
| Heavy spawn distance | `400 - (60 + 10 k)` m ahead, `k = 0..7`, `k` kept across cars | `0x0041f2e0`, floats `0x8eb1ac` (400) |
| Cross spawn distance | `-(150 + 10 k)` m, `k = 0..7`, `k` kept across cars | `0x0041f650`, float `0x8eb1a8` (150) |
| Spawn tries per car | 8 | both placements |
| Ram delay | 3.0 s per car taken | inline `0x40400000` in `0x0041f130` |
| Ram delay gate | the shared timer below zero | `0x0041f130` |
| Drive speed (ram) | 27.78 m/s (100 km/h) | `0x00421560`, `0x41de38e4` |
| Start speed cap (ram) | 26.82 m/s (60 mph) | `0x004214f0`, literal |
| Ram action score | 0.1 | `0x00421360`, `0x3dcccccd` |
| Heavy in-position offset | `(0, 0, -3)` | `0x0043bd10` |
| Cross pursuit offsets | `(0,0,-20)`, `(-4,0,-24)`, `(4,0,-24)` | `0x0041f480` |
| Support check period | 10 s | pursuit constructor, `0x41200000` |
| Pursuit "seen" start | -5 s | pursuit constructor, `0xc0a00000` |

## 13. Open questions

- **Q1** The request timer's countdown and the reset rule are not located. The pursuit's support slot (`+0x44`) is on an
  interface subobject; the object's labelled vtable does not hold its methods (the bytes after the label are string
  data), so the pursuit's own update was not found.
- **Q2** `ReqDenied` is sent each tick while placement fails, and the cars stay in the request. Confirm by a run or by
  finding the caller that clears the list.
- **Q3** The roadblock branch of the executor reads the builder's return value the opposite way to the manager's tick
  (the manager treats 1 as success). One of the two reads is wrong; the roadblock spec should settle it.
- **Q4** The budget check in the car finder (`manager+0x54`) and the `k` reset rule of the spawn search need a run to
  confirm.
- **Q5** The spawn manager's minimum and maximum distances (fields `+4`, `+8`) are not located; the 150 and 400 m values
  are the placement constants.
- **Q6** How the cross slot index is passed (the `0x48` call) is not clear from the decompile.

## 14. Rust implementation notes

- Model a request as `{ heavy: Option<HeavyOption>, leader: Option<LeaderOption>, cars: Vec<CarId>, timer, state }`. Keep
  the three-stage executor (acquire, place, done) as pure steps so each can be tested.
- The spawn search takes a closure for "is this point valid" and a road sampler; the `k` counter lives in the executor
  and carries across cars.
- The ram action is a pure function `intercept(me, v, target, target_vel) -> direction` plus the fixed 27.78 m/s speed.
  Test it with a target at rest and with a crossing target.
- Keep the 3.0 s ram delay on the manager, not on the request, because the original shares one timer.

## Next leads

- Find the pursuit's support request slot (`+0x44`) on its interface subobject and read the timer code next to it (Q1).
- Find the caller of `0x0041f130` that clears a request's car list, or the place where a denied request is dropped (Q2).
- Confirm the roadblock builder's return convention (Q3).
- Locate the spawn manager's constructor for the `+4` and `+8` distance fields (Q5).
- Read the action vtable slots 2 to 6 (can-be-attempted, start, finish) to confirm the ram action's attempt and finish
  rules directly, not through the tactics spec.
- Read `0x00413320` (the desired-speed helper) and `0x00418200` (the intercept) to confirm the tactics spec's formulas.

## Port status

- **Pure rules (`libs/blackbox-pursuit/src/support/`):** `order.rs` is the request's state (pending, active for the option's
  duration, denied tries counted); `cars.rs` is the car each slot takes (cars per strategy, the SUV roll with `ChanceBigSUV`,
  the leader's cross car and two V cars); `placement.rs` is the search (heavy `400 - (60 + 10 k)` ahead, leader `-(150 + 10 k)`
  behind, the try counter kept across the cars, the pursuit offsets `(0,-20)`, `(-4,-24)`, `(4,-24)`); `delay.rs` is the shared
  3.0 s ram timer; `ram.rs` is the head-on drive (intercept from the spec's quadratic, 27.78 m/s, start speed of 5.2).
- **Game (`crates/nfsmw/.../ai/support/`, `ai/car/support_drive.rs`):** a request is run each tick from the cop pursuit
  (`session.rs`). Its cars are taken, then placed through the cop spawn path (`spawn_cop_at` in `pursuit.rs`, shared with the
  wave), tagged with the request's serial. A ram SUV drives at its intercept; a leader car holds its close-chase point; the
  others chase. A request ends when its timer runs out or when none of its cars is left, and the cars return to the chase.
  Support cars count in the chase's cop count (they are cops).
- **The roll (`ai/police/roll.rs`):** the check every 10 s, the priority check of the leader options once per heat, then the
  base roll over the heavy and leader options. A heavy roadblock goes to the world as before. A ram or leader request is
  handed over on the next step (`take_support_request`, then `TrafficWorld::ask_support`). The roll uses one uniform number
  for both its priority check and its base roll (the binary draws one each): a deviation, unconfirmed in effect.
- **Formation exclusion (`ai/cop_aims.rs`):** support cars are left out of the formation, finisher and collapse cop list
  (section 3 of the formations spec), so they keep the chase's aim or their own drive.
- **Not built:** the flee of the support cars when the perpetrator is lost or a collapse begins (formations spec section 6),
  and the roadblock's request path (it stays with `ai/roadblock.rs`).
- **Unconfirmed, chosen in the port:** (a) the ram's start gate reads "start only while the target is below 26.82 m/s"; the
  spec gives 26.82 only as the start speed cap. (b) The try counter stops at `k = 7` for every later try of a request. (c) A
  ram that finds no target goes back to the start gate; the spec never finishes it while the target exists. (d) The
  `ChanceBigSUV` of heat 3's rows is read as 0 (the rows give none in brackets). (e) A request whose intercept has no real
  root takes `t = 0`. (f) A refused placement is retried on each step with no limit (the binary's `ReqDenied` repeats).
- **Verified:** `cargo test -p blackbox-pursuit` passes (the support module's 38 tests) and clippy with `-D warnings` is clean.
  The game side has no run yet: the tests and the spawn and drive in the chase need a headless run.

