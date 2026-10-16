# Provenance: traffic and obstacles

The record for [specs/ai-traffic-obstacles.md](../specs/ai-traffic-obstacles.md). Read-only static analysis of the
original PC 1.3 executable (`speed.exe`, same SHA-256 as [ai-traffic.md](ai-traffic.md)) in Ghidra project `nfsmw`,
program `/speed.exe`, image base `0x00400000`. Symbols are stripped. Function names in this file are Ghidra's
automatic names (`FUN_`), kept only here as locators. No function was created, renamed or saved in the project.

## Addresses

| Address | Role | How found |
|---|---|---|
| `0x0089100c` | vtable of the traffic driving action (`AIActionTraffic`); the constructor writes it | the constructor `FUN_0041fe30` (the object's first write) |
| `0x0041ffa0` | vtable slot 0: scalar deleting destructor (calls the body and free) | slot 0 of the vtable |
| `0x00406600` | vtable slot 1: decision "pull over" = `xor al,al; ret 8` (always false) | bytes at the slot target |
| `0x00406930` | vtable slot 2: large function, 0x1f4-byte frame, structured exception frame; no Ghidra function | bytes; not identified (lead 2) |
| `0x004068f0` | vtable slot 3: boolean query on the navigator's pursuit target | data xref from `0x00891018`; decompiled |
| `0x004117d0` | vtable slot 4: message dispatcher (compares class at `0x0092c5e0`); no Ghidra function | bytes (lead 3) |
| `0x00420180` | vtable slot 5: `begin` (action start): navigator flags at +0x78 and +0x80, navigator init, accident cleared, avoidable radius 30 m, speeds from the spawn message | data xref from `0x00891020`; decompiled |
| `0x00406a60` | vtable slot 6: no-op (`ret 4`) | bytes |
| `0x00411cf0` | vtable slot 7: **think** (float `dT`); xref from `0x00891028` | decompiled |
| `0x00410cd0` | vtable slot 8: `xor al,al; ret` (false) | bytes |
| `0x00406a80` | vtable slot 9: no-op (`ret`) | bytes |
| `0x00411820` | the speed function (`compute_speed`): posted speed, bends, car ahead, +2 dT | decompiled; called from the think |
| `0x00411a70` | drive target and avoidable radius setter; flag `this+0xb4` (state) read here | decompiled |
| `0x00420040` | the crash trigger, for one party of a collision (80 m camera gate, 5 mph, class test, tractor) | the only writer of the crash state 1 (`search_instructions` for `0xac], 0x1`), decompiled |
| `0x00420130` | the collision callback: skips cops (own class 2), only object collisions (flag `& 7 == 1`), calls the trigger for both parties | caller of `0x00420040`, decompiled |
| `0x006533b0` | piecewise-linear lookup (count, base, scale, data pointer); used for the stopping table and the removal tables | callee of the speed function; decompiled |
| `0x008eb394` | stopping table record: count 2, base 0, scale 0.05, data pointer `0x008eb1e8`, [count, base, 80.0, 0.05, ptr, 2] | read memory |
| `0x008eb1e8` | stopping table values: 3.0 and 50.0 (then other data) | read memory |
| `0x008eb358` | removal distance table record: count 11, base 0, scale 10.0 | read memory (for the scale comparison) |
| `0x007c4b80` | float to 64-bit integer helper (truncation, as a C cast); Ghidra shows `ROUND` | decompiled |
| `0x00401ae0` | ramp (clamp of (x − a) / (b − a) to 0..1) | decompiled |
| `0x00402060`, `0x00402020` | clamp (min and max) helpers | decompiled |
| `0x0076dd30` | traffic-lane count of a segment (compared with 4) | callee of the speed function |
| `0x00778910` | trail curvature (called in the speed function) | callee |
| `0x0043d540`, `0x0043d5a0`, `0x0043d600` | goal factories (pool sizes 0x18, 0x18, 0x1c); each builds one goal with an action set | decompiled; callers are indirect (none found) |
| `0x0043b670` | action set factory: `AIActionTraffic` only. The candidate for the traffic goal | decompiled; called from `0x0043d540` |
| `0x0043b730` | action set factory: `AIActionTraffic` and `AIActionTooDamaged`. Goal name not confirmed | decompiled; called from `0x0043d5a0` |
| `0x0043b800` | action set factory with the race, pursuit, too-damaged, get-unstuck and airborne actions; `AIActionTraffic` added only for drivers whose class test is false | decompiled; called from `0x0043d600` |
| `0x00423190` | the traffic goal's update (sets the target from the navigator or a fallback, and flags 3); references `AIGoalTraffic` | decompiled |
| `0x00883e60` | static initialiser that pushes `AIActionGetUnstuck` and calls a registration routine | bytes around the string xref |

## Strings searched (whole program, `search_strings`)

- Present: `AIActionTraffic` (`0x008912f0`), `AIGoalTraffic` (`0x00891530`), `AIVehicleTraffic` (`0x0089305c`),
  `AIActionGetUnstuck` (`0x00892c38`), `AIActionTooDamaged`, `AIActionAirborne`, `AIActionPursuitOffRoad`,
  `AIActionStopShort`, `AIActionRam`, `AIActionHeadOnRam`, `AIActionStaticRoadBlock`, `AIGoalStaticRoadBlock`,
  `AIGoalHeadOnRam`, `AIGoalPursuit`, `AIGoalPatrol`, `AIGoalRacer`, `AvoidableManager`, `SetTrafficSpeed`,
  `TrafficCars`, `TRAFFIC_TIRE`, `TRAFFIC_PATTERN`, `DRIVER_TYPE_TRAFFIC_CAR`, `TrafficLevel`, `AITraffic`,
  `BigTraffic`, `HugeTraffic`, `roadblocks_dodged`, `roadblocks_dodged_in_pursuit`.
- Sound only: `CARSFX_TrafficHorn`, `AUD: Traffic Horn`, `CARSFX_TrafficWoosh`, `CARSFX_TrafficEngine`,
  `CARSFX_TrafficSkids`, `SFXCTL_3DTrafficPos`, `eMM_TRAFFIC`, `FX_TRAFFIC`, `EAXTrafficCar`.
- Absent (no hits): `LaneChange`, `Overtake`, `Evade`, `Honk` (in AI code), `Reverse` (except `LIGHT_REVERSE`
  and the light names), `Wreck`, `Gap`, `Yield`, `Swerve`, `Stuck` (only `AIActionGetUnstuck`), `Recover`, `Respawn`,
  `offscreen`, `despawn`, `visible`, `inview`.
- `EVADED_%d` (`0x0089d470`) is a pursuit string for the cops' escape, not traffic.

## The write search

The action's state offsets were checked by `search_instructions` for every `MOV` that writes or reads them:

- Offset `0xb4` (pull-over state): written in the constructor (`= 0`) and the think (`= 0`, `= 2`). No write of `1`
  anywhere in the program for that offset in this class (the other hits belong to other classes, found with `0xb4]`).
- Offset `0xac` (crash state): written with `= 1` only in `0x00420040`; written `= 2` and `= 0` in the think and the
  begin function.
- Offset `0xb0` (timer): written `= 0x40400000` (3.0) in `0x00420040`; decremented by the think.

Searching a member offset by its operand text (for example `0xac], 0x1`) was the fastest way to find the writer
of the crash state; the search lists every instruction in the program, so filter by the function that matches the
class.

## Lessons

- **Read the vtable before the callers.** The action's vtable has ten slots. The data xrefs from the vtable entries
  (`0x00891018` to `0x00891028`) name the slot's function, and the bytes of the other slots show stubs at once
  (`xor al,al; ret 8` is false, `ret` and `ret 4` are no-ops). Slot 1 (the pull-over decision) is one of the stubs,
  so the pull-over path is dead before any decompile.
- **Check the object an offset belongs to.** In the speed function `this+0x40` (the navigator through its getter)
  and `this+0x5c` (the own vehicle through the constructor) are different objects. The first read of the speed rule
  looked like "occluder mass"; it is the own vehicle's mass and radius, which is what the spec says. Write down the
  object each offset comes from before reading a formula.
- **Lookup tables are records with a scale.** The helper `0x006533b0` computes `index = floor((x − base) × scale)` on
  a record whose fields are count, base, a second number, scale, data pointer and count again. Read the data words
  next to the record, and compare with a table whose meaning is known (the removal table has scale 10 and 11 points,
  which fits the provenance of [ai-traffic-spawning.md §8](../specs/ai-traffic-spawning.md#8-removing-cars)). The
  stopping table has scale 0.05 and two points, which gives a range of 20 units, not 80. This is the source of the
  `/20` versus `/80` conflict.
- **`ROUND` in Ghidra can mean truncation.** `0x007c4b80` is shown as `ROUND(ST0)`, but its body corrects the value
  to truncate (the C cast). Check the correction code before reading a conversion helper as rounding.
- **Goal factories are called through tables.** The three factories `0x0043d540`, `0x0043d5a0`, `0x0043d600` have no
  visible callers. The action set each builds identifies the goal: the one with only `AIActionTraffic` is the traffic
  candidate. The goal name is then checked against the goal-name table.
- **Strings name the sets, not the code.** A string xref to an action name lists the action set that uses it.
  That found the get-unstuck action in the racer, human and cop sets and not in the traffic set.
- **The Ghidra project has gaps.** Some vtable targets (slots 2, 4, 6, 8, 9) are not function entries. `disassemble_function`
  refuses them, but `read_memory` gives the bytes. Do not create functions in the shared project to read them; read the
  bytes, and create them only in a scratch copy.
- **Name patterns beat decompiles for absence.** Four `search_strings` calls (behaviour words, sound words, state
  words, and the pursuit words) answered most of the "does it do X" questions before any function was read.

## Dead ends

- **Lane change, overtake, evade, honk in AI, yield, reverse, wreck.** No strings, no action names, no code path. The
  only horn is sound-side. Recorded in §1 of the spec.
- **Pull over.** The decision is a stub; the state is never written as 1. The cursor's `pull_over` query is also a
  stub in the road network ([ai-road-network.md §6](ai-road-network.md#6-queries)). Dead.
- **Stop sign and junction flags.** Still unused, as recorded in [ai-traffic.md §6](ai-traffic.md#6-pulling-over-cops-and-the-dormant-parts).
- **The stopping table's other callers.** Not searched in this pass; lead 1.
- **Action vtable slot 2 (`0x00406930`).** A real function with a large frame, not defined in Ghidra. Its role is
  open (lead 2).
- **Slot 4 dispatcher (`0x004117d0`).** It compares the class of the message with `0x0092c5e0`; the message identity
  is not decoded (lead 3).
- **The get-unstuck implementation.** The registration at `0x00883e60` was located; the factory it calls was not
  decoded in this pass (lead 5).
- **Xrefs to code entries.** Not tested in this pass. Function callers came from `get_functions` with
  `fields=callers`, which worked for every entry checked.

## Checked against the port (read only)

- `traffic.rs`: `think`, `compute_speed`, `stopping_distance` (`/80`), the 30 m blocked look, the `from_behind`
  check, the lateral 0.6 g, the 2 dT clamp. The `/80` is the conflict in the spec (§4.4).
- `car/accident.rs`: `ACCIDENT_THINKS = 3`, decrement-then-controls order, the tractor path (centred steering) and
  `on_hit` (returns early unless the state is none, so there is no restart).
- `car/hits.rs`: `on_hit(traffic && !patrol, tractor)` and the camera distance argument. The gate on the other party's
  class and speed was not read in the caller.
- `car.rs`: the cops' stuck and reverse code. It is for cops only.
- `stop.rs`: traffic lights (a rewrite extension, not in the original).

## Open

- The leads in the spec (§Next leads) are the open items.
- No game measurement exists for any number in this file. The in-game checks are in the spec §11.
