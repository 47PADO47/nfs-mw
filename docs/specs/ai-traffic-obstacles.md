# Traffic and obstacles: what an original traffic car does about a car in the way

What an ordinary road car of the original does when another car, a wreck, a roadblock or the player is in its lane,
when the player comes at it from behind or head-on, and how it recovers from being stuck. The driving action, speed
rules and trail are in [ai-traffic.md](ai-traffic.md); the navigator and trail in
[ai-road-network.md](ai-road-network.md) and [ai-road-nav-trail.md](ai-road-nav-trail.md); the accident trigger's
place among the other hits in [ai-traffic.md §5.3](ai-traffic.md#53-crashes-with-the-player-racers-and-cops-the-accident-state).
Provenance and Ghidra addresses: [provenance/ai-traffic-obstacles.md](../provenance/ai-traffic-obstacles.md).
For the tag meanings, see [evidence tags](../README.md#evidence-tags).

- **Sources read:** `speed.exe` (PC 1.3, SHA-256 in [provenance/ai-traffic.md](../provenance/ai-traffic.md)), read
  through the Ghidra project `nfsmw`, program `/speed.exe`, image base `0x00400000`. Symbols are stripped; the
  names below are behaviour names given here. Read-only: no function was created or renamed in the project.
  The port was read for comparison only: `crates/nfsmw/src/scenes/world/ai/traffic.rs`,
  `car/accident.rs`, `car/hits.rs`, `stop.rs`.
- **Method:** string and xref searches for every behaviour word (lane change, overtake, evade, honk, unstuck,
  reverse, pull over); the vtable of the traffic action; the decompiled think, speed and accident functions; a
  search of every write to the action's state offsets. Anything not reached this way is listed as a lead.

Everything is **[decomp]** unless marked. The three statements that change the port's numbers are marked
**[decomp, conflict]** in §4 and §5.

## 1. Short answer

| Behaviour | Original | Evidence | Port |
|---|---|---|---|
| Slow down behind a car in its lane | yes | the car-ahead rule, §4.3 | yes (`traffic.rs`, `compute_speed`) |
| Stop behind a stopped car in its lane | yes, same rule: the speed goes to 0 as the gap closes | §4.3 | yes |
| Shift sideways inside its own lane to pass a car | partly: the steering point moves inside a narrow corridor; the car never leaves its lane | trail cut, §8.1 | trail punch in `blackbox-roads` |
| Change lane to get round a blocked car | **no** | no lane-change string; no lane-change path in the action; cursor stays in lane | no |
| Overtake | **no** | as above; no overtake string | no |
| Pull over to the kerb | **no** (code present, never entered) | decision slot is a stub; state 1 is never written, §6 | no |
| Reverse | **no** | the traffic action set has no unstuck or reverse action, §7 | no |
| Get-unstuck recovery or a stuck timer | **no** for traffic | action set of the traffic goal is `AIActionTraffic` only, §7 | no |
| Respawn when stuck | **no timer**; cars go only by the manager's distance and time rule | removal rule, [ai-traffic-spawning.md §8](ai-traffic-spawning.md#8-removing-cars) | manager rule |
| Wait for an oncoming gap or yield at a junction | **no** priority logic; a car in its lane only slows (emergent), §8.2 | no yield code; no stop-sign use | no (lights are a rewrite extension) |
| Honk at a blocking car or the player | **no** AI honk; horns are sound-side and player-triggered | no AI horn string; [ai-traffic-world.md §3](ai-traffic-world.md#3-horns-engine-and-drive-by-sounds) | no AI honk |
| Evade a player coming from behind or head-on | **no** evade; from behind is ignored by the speed rule; head-on in its lane is a car ahead | §8 | partly (`from_behind` flag) |
| React to a crash it is hit in | yes: 3 thinks, then brake to a stop (tractors stop at once) | accident trigger, §5 | yes, with gaps, §9 |

**Verdict:** the original traffic car is a lane follower with one speed rule and a crash state. It never changes lane,
never overtakes, never reverses and never honks at obstacles. Whatever it does around a blocking car is a slowdown
and, at most, a sidestep inside its own narrow corridor.

## 2. What the car can see

- **Avoidables.** Every AI car and the player is an avoidable: a region around the car, refreshed every four fixed
  ticks ([ai-traffic.md §5.1](ai-traffic.md#51-avoiding-cars-ahead)). A traffic car sets its avoidable radius to its
  own look-ahead each think (§3).
- **Occlusion.** The navigator's trail is cut by the avoidables that lie in it (hole punching, [nav-trail §5](ai-road-nav-trail.md#5-hole-punching-around-other-cars-decomp)).
  The occlusion result gives a flag "occluded by a car" (avoidable), a flag "occluded from behind" (the blocker is at
  or behind the car), the blocker's trail speed and the apex point. **[decomp]**
- **No world probe.** The think's direct callees are the navigator's advance, the occlusion update, the speed
  function, the drive-target setter, a clamp and a normalise. None is a world ray cast or a road-edge probe by name.
  Two helpers reached from the drive-target setter were not identified (§11, lead 4). The road edge is enforced only by
  the lane corridor. **[decomp, derived]**
- **Look-ahead.** `look = lerp(10, 30, ramp(speed, 0, 25)) + speed × dT + radius` metres; **30 m + the same tail** when
  the navigator reports "occluded by a car" (the blocked case). **[decomp]**, the same as [ai-traffic.md §3.1](ai-traffic.md#31-per-think-update).

## 3. The think, in the order the code runs it **[decomp]**

Per think (`dT` = time since the previous think):

1. Read the speed, the own radius and the navigator; `t = ramp(speed, 0, 25)`. If the navigator reports
   "occluded by a car" with an occluder present, `t = 1` (look 30 m).
2. Advance the navigator cursor to `look` metres ahead of the car. The random lane choice at junctions is the only
   lane decision ([ai-road-network.md §4.3](ai-road-network.md#43-choosing-the-next-segment-traffic-navs-decomp)).
3. Set the avoidable radius to `look`.
4. Compute the desired speed (§4). If the navigator is valid, the drive target is its occluded position and the
   speed is the desired one; otherwise the speed is 0.
5. Run the accident state (§5). In state 0 request driving flags 3 (steer, gas and brake). In states 1 and 2 the
   accident controls replace the drive controls.

The state word of the action is never set to "pulling over" by any code path found (§6).

## 4. Speed rules

### 4.1 Posted speed

`SpeedStreet` is the value at action offset 0x50 and `SpeedHighway` the value at 0x54, both copied from the spawn
message at `begin` (`0.44703 ×` the pattern's mph value) **[decomp]**. The posted speed is `SpeedHighway` when the
segment has four or more traffic-lane zones, else `SpeedStreet`. The values are 35 and 55 mph in every shipped pattern
**[verified]**, as in [ai-traffic.md §3.3](ai-traffic.md#33-speed).

### 4.2 Bends

`limit = sqrt(a / max(a / posted², |k|))` where `a = 0.6 g` for a car and `1.6 g` for a cop (driver class 2), and
`k` is the trail curvature. This equals `sqrt(a / |k|)` capped at the posted speed. **[decomp]**, as in
[ai-traffic.md §3.3](ai-traffic.md#33-speed). The rule is off when the `fixed` flag is set on the action.

### 4.3 The car ahead (verified in shape; one constant in conflict)

Applies only when the navigator reports "occluded by a car", the blocker is **not** "from behind", and the `fixed`
flag is off **[decomp]**:

```
length   = 2 × own radius
distance = max(0, |apex − own position| − length)       # apex = the occlusion's apex point
mass     = own rigid-body mass; doubled for a tractor (the tractor's own action flag)
x        = speed × max(mass × 0.0005, 1)
stop     = T(x)                                          # the two-point table of §4.4
if distance < stop:
    desired = clamp(blocker_trail_speed × distance / stop, 0, desired)
    desired = min(desired, speed)
```

The blocker's trail speed is its speed along its own trail, not a closing speed. The rule therefore ignores whether
the blocker is coming towards the car. A car stopped in its lane gives `blocker_trail_speed = 0`, so the car's
desired speed goes to 0 as the gap closes.

### 4.4 The stopping table **[decomp, conflict]**

`T(x)` is a generic piecewise-linear lookup. Its table is a record with a count of **2**, a base of 0, an index
scale of **0.05** and two values, **3 m** and **50 m**. The index is `floor((x − 0) × 0.05)`; between the two points
the result is linear, and beyond it is 50 m. So:

```
T(x) = 3 + 47 × min(x / 20, 1)        # decomp reading
```

The spec ([ai-traffic.md §3.3](ai-traffic.md#33-speed)) and the port (`traffic.rs`, `stopping_distance`) use
`3 + 47 × clamp(x / 80, 0, 1)`. The two readings differ by a factor of four in range.

- At 15 m/s and 1680 kg (`x = 15`): decomp gives **38.25 m**, the spec and port give **11.81 m**.
- The same lookup routine uses index scale 10 for the removal table ([provenance](../provenance/ai-traffic-obstacles.md)),
  so the scale word is read as the index multiplier. The word `80.0` stored in the same record is not read by the
  lookup routine **[decomp]**.

**Action:** do not change the port until the scale is confirmed by one of the checks in §11 (check 1 measures it).

### 4.5 Limits on the change of speed

- `desired = min(desired, speed + 2 × dT)` for non-cops. This limits how far the target may exceed the current speed;
  the throttle is still full when the target is above the speed **[decomp]**.
- Gas and brake from the desired speed: shared rule in [ai-driver-control.md](ai-driver-control.md).

## 5. The crash state (the accident)

### 5.1 Trigger **[decomp]**

A collision of the traffic car with another object runs the trigger for both parties (the other party and the car
itself are the two entries of the collision record; the car itself is skipped). The collision must be object against
object; world collisions do not trigger it. Cops (own driver class 2) do not run the trigger at all.

For the other party `o`, the trigger runs only when all of these hold:

- `o` is not the car itself;
- the collision point is within **80 m** of the view point the camera uses (the "camera" distance);
- `o` exists and `o` is a vehicle, and either `|o.speed| ≥ 5 mph` (2.23515 m/s) or the accident state is not "over";
- `o`'s driver class is **human, cop or racer** (classes 0, 2 and 3 in the code; traffic is class 1). **[decomp]**
  The class numbering is inferred from the three sets and the traffic exclusion, not from a name table.

When it runs:

- if the car is a **tractor** (inherited rigid-body behaviour, [ai-traffic.md §2](ai-traffic.md#2-the-traffic-cars)):
  state = "over" at once, timer = 0;
- otherwise: state = "in progress", timer = **3**.

Only a collision with a non-traffic vehicle counts. Traffic against traffic, props and the world do nothing here
(the shock rule of [ai-traffic.md §5.4](ai-traffic.md#54-shock) still applies).

### 5.2 Phases **[decomp]**

- **In progress** (state 1): the timer is decremented by 1 **per think**; when it reaches 0 the state becomes "over".
  While in progress the controls are gas 0, brake 0, handbrake 0, and steering full lock (value 1) for a non-tractor.
  A tractor's steering is not written in this state. The think order is decrement first, then the controls of the
  state that results. So the phase lasts 3 thinks (about 0.5 s at the 6 Hz think rate, the same count as
  [ai-traffic.md §5.3](ai-traffic.md#53-crashes-with-the-player-racers-and-cops-the-accident-state)).
- **Over** (state 2): gas 0, brake full, and steering full lock for a non-tractor. The car stops and stays
  stopped until the action restarts (respawn, which is `begin`).

### 5.3 Restart **[decomp]**

The trigger's own gate allows a new hit to start the accident again while the state is "over", provided the other
party is moving at 5 mph or faster. So a traffic car that is already stopped by a crash is re-armed by a second
fast hit. The port's `Accident::on_hit` does not allow this (§9).

## 6. Pulling over (dead code in this build)

- The action has a state word with three states: 0 normal, 1 "pulling over", 2 "pulled over". The state-2
  handler sets the word back to 0 and tells the navigator to re-initialise on the current lane **[decomp]**.
- The decision whether to pull over is a function that always answers "no" (`xor al, al; ret 8`). It is the
  action's vtable slot 1 **[decomp]**.
- No code path writes state 1 in the action (every write to that offset was checked: the constructor writes 0,
  the think writes 0 and 2, nothing writes 1) **[decomp]**.
- So traffic never pulls over, never moves its cursor to the kerb and never stops for a broken-down car.

## 7. Stuck handling and recovery

- **The traffic goal has one action**, `AIActionTraffic`. The goal's action set is built by a factory that adds only
  that action. Its only other form adds the too-damaged action; neither includes the get-unstuck action **[decomp]**.
- **Get-unstuck** (`AIActionGetUnstuck`) is in the action sets of the other drivers (the racer, human and cop sets
  built by the factories that also list the pursuit and ram actions) **[decomp]**. Its implementation was not
  reached (§11, lead 5).
- **No stuck timer** exists in the traffic think. A car blocked behind a stopped car stays there until the manager
  removes it.
- **Dead end:** when the navigator reaches a node with fewer than two exits, it stops and the speed is 0. The car
  stays stopped; it does not turn round [ai-traffic.md §3.2](ai-traffic.md#32-the-cursor-ahead-of-the-car).
- **Removal:** the traffic manager (provenance record) removes a car when a distance and time test fails. That is the
  only respawn path for a stuck traffic car. The test does not look at speed or progress.

## 8. The player, head-on and from behind

### 8.1 The steering point

The steering point is the occluded position from the trail, and a blocked car is steered towards it.
The cut for a blocking car (hole punching) moves the corridor's edge on the side that has room, keeping at least
0.1 m of corridor for traffic. So a car can steer towards one edge of its own lane while a car is in its path.
It cannot steer outside its lane: the corridor is the lane's own width ([ai-road-nav-trail.md §2](ai-road-nav-trail.md#2-corridor-bounds-decomp)).
**[decomp, derived]** This sidestep is a result of the cut; no code names it as a manoeuvre, and no measurement exists.

### 8.2 Oncoming car in the same lane (head-on)

- The oncoming car is an avoidable with a trail. If it is in the car's corridor, the car is "occluded by a car" and
  the speed rule of §4.3 applies with the oncoming car's trail speed.
- The rule does not use the closing speed. Closing speed enters the steering target only: the avoidable occlusion
  scales the aim by `2 × ratio`, where `ratio` is the closing speed over twice the car's own speed along the line
  (clamped 0 to 1) **[decomp]**.
- Result: the traffic car slows as the oncoming car nears (the stop distance grows with speed) and stops if the gap
  is within the stopping distance. When the oncoming car moves out of the look-ahead corridor, the car accelerates
  again. This is a **wait** that the rules produce, not a rule that says "wait for a gap". **[decomp, derived; not
  measured]**.
- An oncoming car in the other lane does not occlude the corridor unless its footprint enters it; the corridor is
  only one lane wide.

### 8.3 Player from behind

- A car behind is flagged "occluded from behind" and the speed rule of §4.3 is skipped for it (`from_behind` on the
  occlusion). Traffic does not slow for a player behind it **[decomp]**.
- The player's car behind does not change the steering point ahead either, because the trail is ahead of the car.
- A car behind within its footprint is still cut with the "inside" flag ([ai-road-nav-trail.md §5](ai-road-nav-trail.md#5-hole-punching-around-other-cars-decomp)).
  The steering point is then computed from the trail ahead and the flag, not from the car behind.

### 8.4 Player at speed on the same road

Nothing in the traffic action reads the player's speed, distance or heading. The player is an avoidable like any
other car, so the rules above apply. Honks and whooshes are sound-side ([ai-traffic-world.md §3](ai-traffic-world.md#3-horns-engine-and-drive-by-sounds)).

## 9. What the port lacks or does differently

### 9.1 (a) Original behaviour the port does not have, or differs

1. **Re-arming the accident.** The original starts a new accident from a hit by a fast non-traffic vehicle while
   the state is "over" (§5.3). `car/accident.rs` `on_hit` returns early unless the state is "none", so a stopped
   traffic car is never re-armed. Port gap, matches the audit's item 4 ([traffic-interactions-audit](../traffic-interactions-audit.md)).
2. **Accident gate.** The original's gates are: object collision only; the other party's class in {human, cop,
   racer}; the other party at ≥ 5 mph or the state not over; the camera distance ≤ 80 m (§5.1). The port's
   `on_hit(traffic, tractor)` gates only the car's own role and patrol state. The caller's gating was not read here;
   check `car/hits.rs` against §5.1.
3. **Tractor steering in the crash.** The original writes no steering value for a tractor in either crash phase
   (§5.2). The port centres the steering for tractors (`accident.rs` `controls`). The original's steering stays at
   whatever the last drive set. This is a visible difference only if the last drive set a turn.
4. **Stopping distance.** The port uses `/80` where the decomp reads `/20` (§4.4). **Unresolved; do not change until
   §11 check 1 is run.**

### 9.2 Matches (no change needed)

- No lane change, no overtake, no pull over (the port's pull-over path is absent; the original's is dead).
- No reverse and no get-unstuck for traffic (the port's stuck and reverse code is in `car.rs` for cops only).
- No AI honk.
- The think's look-ahead of 10 to 30 m, and 30 m when blocked.
- The 3-think crash phase and the tractor's immediate stop.

## 10. (b) Beyond the original: rewrite extensions

Nothing in this section is in the original. Each proposal keeps the lanes the road network already gives (the nth
traffic lane from the centre, the same direction, and the segment's lane zones), uses only the avoidables and the
existing clock, and changes no data. Each must be recorded as a rewrite extension, behind a setting, in the same
way the traffic-light extension is ([ai-traffic-world.md §8](ai-traffic-world.md#8-rust-implementation-notes)).

### 10.1 Go round a blocked car in the same direction (lane change)

- **Trigger:** the car has been "blocked" (occluded by a car within the stopping distance, §4.3) for at least **3 s**
  of simulated time, and its speed is under 5 m/s.
- **Lane:** the nth traffic lane next to the car's lane on the same segment (`nth ± 1`), in the same direction. If
  the segment has one lane in that direction, there is no change.
- **Gap test:** the target lane has no avoidable from **10 m behind** to **2 s × speed + 15 m ahead** of the car, and
  the avoidable in the target lane (if any) is not closing faster than the car is moving.
- **Execution:** re-initialise the navigator on the neighbour lane at the car's position and heading. Blend the lateral
  offset over **2 s × speed** metres (no step). Keep the posted speed; no extra acceleration.
- **Abort:** if an avoidable enters the gap test during the blend, the car stays in its original lane and waits.
- **Return:** after **15 m** clear of the blocker, move back only if the original lane's gap test passes.

### 10.2 Wait for an oncoming gap before crossing

- **Use:** at a decision segment where the car's exit lane crosses or joins an opposing lane (two-way roads).
- **Gate:** the car stops at the entry of the decision segment (the same speed-limit mechanism as the traffic-light
  extension) until no avoidable's time to closest approach to the crossing point is under **3 s**.
- **Timeout:** after **8 s** of waiting, the car goes anyway. This keeps the flow alive and avoids deadlocks. The
  original deadlocks head-on at junctions ([ai-traffic-world.md §2](ai-traffic-world.md#2-intersections-and-priority)).
  The timeout is the extension's choice, not the original's.

### 10.3 Honk when blocked

- **Trigger:** blocked (as in 10.1) for **2 s**, with the player within 20 m. Sound only.
- **Sample:** the car's own `HornType` sample; the 3 s cooldown and the shared `last_honk_time` of the original horn
  rule ([ai-traffic-world.md §3.2](ai-traffic-world.md#32-horn)) are reused.

### 10.4 Stuck timeout

- **Trigger:** stopped and blocked for **60 s** of simulated time.
- **Action:** mark the car as removable so the manager's distance and time rule can take it at the next test. Do not
  teleport it.

## 11. How to check it

1. **Stopping distance (decides §4.4).** In the original, free roam at a steady 15 m/s in a 1680 kg street car:
   park a car in the lane ahead and watch a traffic car approaching it. Record the distance at which the traffic
   car first lifts off the throttle. The decomp predicts about **38 m**, the spec and port predict about **12 m**.
   Repeat at a second speed, for example 10 m/s, to check the linear shape.
2. **No lane change.** A traffic car behind a stopped car stays in its lane for at least 30 s. Expect no lateral
   shift larger than the lane's own width.
3. **No overtake, no reverse.** A traffic car behind a stopped car never passes it and never reverses.
4. **Crash phase.** Hit a traffic car with the player. Count the thinks (at 60 FPS, about 30 frames) during which
   the car has no throttle and no brake, and then the full brake.
5. **Head-on.** Drive at a traffic car in its own lane at 15 m/s. Expect it to slow as you close and to stop if the
   gap is inside the stopping distance. Check whether it moves to one side of its lane before you pass.
6. **Restart.** Hit a traffic car until it is stopped; hit it again with a car at 5 mph or more. The original should
   start a new crash phase (§5.3); the port should not.

## 11a. Open questions

- Whether the stopping table's scale is 0.05 (decomp) or 1/80 (spec) (§4.4).
- Whether the tractor's crash steering is left at the last value (decomp) or centred (port) (§9.1 item 3).
- Whether the head-on wait in §8.2 appears in measurement, and how long it lasts.
- The class numbering of driver classes 0 to 3 (§5.1) is inferred from three sets, not from a name table.

## 12. Port status

What the port does now, against §9 and §10. Settings: `traffic_avoidance` (default **off**; TOML key of the same name,
`NFSMW_TRAFFIC_AVOIDANCE`, console `set traffic_avoidance on`). The pure rules are in `libs/blackbox-roads/src/avoidance/`;
the game wiring is `crates/nfsmw/src/scenes/world/ai/car/avoid.rs`.

| Item | Status | Where |
|---|---|---|
| §9.1 (a)1 crash restart from over | done: a hit by the player at 5 mph or more restarts an "over" car; a hit while in progress restarts the three thinks | `car/accident.rs` `on_hit(traffic, tractor, fast)`, `car/hits.rs` |
| §9.1 (a)2 crash gate | done for the player: the hit must be by the player and within 80 m of the view. The class gate is "human only": AI cop and racer hits do not reach the trigger yet, because `collide.rs` does not pass the other party's class | `car/hits.rs` `starts_crash`, `is_fast` |
| §9.1 (a)3 tractor steering in the crash | done: a tractor keeps the steering it had in both phases | `car/accident.rs` `controls(tractor, held_steer)` |
| §9.1 (a)4 stopping distance (`/80` vs `/20`) | **not changed**. The port still uses `/80`. Decide after §11 check 1 in the game | `traffic.rs` `stopping_distance` |
| §10.1 lane change to the next lane, same direction | done behind the setting (see the notes below) | `avoidance/change.rs`, `avoidance/gap.rs`, `avoidance/blend.rs`, `car/avoid.rs` |
| §10.2 wait for an oncoming gap at a crossing | **rule only**: `Crossing` and `time_to_closest` with the 3 s threshold and 8 s timeout are tested, but nothing calls them, because the port does not yet find the crossing point of a decision segment | `avoidance/crossing.rs` |
| §10.3 honk when blocked | **rule only**: `Blocked::take_honk` (once per blocked episode, at 3 s as the brief asks, not the 2 s and 20 m of the spec). The sound is not wired: `TrafficSound::Horn` exists only under `cfg(test)` in `audio/traffic.rs`, so a production horn event is a follow-up | `avoidance/blocked.rs` |
| §10.4 stuck timeout (60 s) | **rule only**: `Blocked::stuck`. Removal is not wired. Two steps remain: the car must expose its `Blocked` clock (today `car/avoid.rs` keeps it private), and `population.rs` `Vitals::valid` must treat a stuck car as removable. That check stays off-screen only, as the rewrite's other removal rules are. The rewrite's existing rule there (idle 20 s and more than 40 m away) is not the 60 s timeout | `avoidance/blocked.rs` |

Notes on the lane change (§10.1), where the implementation chose:

- **Trigger.** Blocked for 3 s of thinks (6 Hz, so the clock moves in steps of 0.167 s), and speed under 5 m/s. The
  blocked flag is the one the last think found, as in `traffic.rs`.
- **Lanes.** The next traffic lane towards the centre, then the one towards the kerb; the first one with a clear gap.
  Lanes come from the travel-frame profile (`forward_traffic_lanes`), so the same-direction rule holds by construction.
- **Gap.** A car in the target lane blocks it when it is inside the gap (10 m behind to 2 s × speed + 15 m ahead) and
  it is not moving away: a car ahead that is slower than the changing car, or a car behind that is faster. The spec's
  "not closing faster than the car is moving" is read this way.
- **Blend.** The steering target starts at the car's own place and closes on the new lane's centre with a smooth step
  over 2 s × max(speed, 3 m/s), driven by the distance driven per physics step. The 3 m/s floor is the implementation's
  choice: at a standstill the spec's 2 s × speed would be a step.
- **Abort and return.** A car that finds another car in the new lane during the blend goes back to its old lane and
  waits. After 15 m clear of the blocker in the new lane, it moves home if the home gap is clear.
- **Off by default.** With the setting off no rule runs and the car never leaves its lane (checked in a unit test).

Not in this pass: the cars do not yet read the other party's driver class for the crash gate (§9.1 (a)2), and nothing
in the simulation calls the crossing, honk or stuck rules (§10.2 to §10.4 above).

## Next leads

1. **Confirm the stopping table scale.** The table record is at `0x008eb394` (count 2, scale 0.05, data at
   `0x008eb1e8`, values 3 and 50). Find another caller of the lookup routine (`0x006533b0`) that reads a known table,
   and check the shared record layout. Then run check 1 of §11 in the game.
2. **Identify action vtable slot 2** (`0x00406930`, a large function with a 0x1f4-byte frame and a structured
   exception frame). The Ghidra project has no function there; read its body with the listing tools without
   creating one, or define it in a scratch copy.
3. **Identify the message dispatcher at `0x004117d0`** (vtable slot 4). It compares the message's class with the global
   at `0x0092c5e0` and calls a behaviour lookup by key `0x004039f0`. If it is the collision message handler, it is
   the second entry point of the crash trigger.
4. **Identify the helpers of the drive-target setter** (`0x0077bba0`, `0x00776600`, called from the function at
   `0x00411a70`). If either probes the world, the "no world probe" claim of §2 is wrong.
5. **Find the get-unstuck action's implementation.** Its registration is the static initialiser at `0x00883e60`
   (name pushed, then a registration call); the factory is not decoded. Likely a reverse-and-retry, the cop stuck
   logic in the port is the analogue.
6. **Map the goal factories to goal names.** The traffic-only factory (`0x0043b670`) is the candidate for
   `AIGoalTraffic`; confirm through the goal-name table near `0x00891524` to `0x00891550`.
7. **Read the traffic goal's update** (`0x00423190`): it sets the drive target from the navigator, or from a point
   ten times a stored vector when the navigator is not valid. Confirm the fallback.
8. **Trace the writers of the navigator's occluder pointer (offset 0x5c) and the occluded flag (0x55).** That tells
   exactly which avoidable is the occluder, and whether it can be a racer or a cop.
9. **Identify action vtable slot 3** (`0x004068f0`): a boolean that reads the pursuit target of the navigator and asks
   it for a test. The role is not named here.
10. **Re-check the horn rule** of [ai-traffic-world.md §3.2](ai-traffic-world.md#32-horn) against the sound code. It was
    not re-read in this pass.
11. **Check the port's accident caller** (`car/hits.rs`) for the 80 m camera gate and the other party's class.
