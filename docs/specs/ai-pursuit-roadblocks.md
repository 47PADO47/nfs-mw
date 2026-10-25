# Roadblocks, spike strips and pursuit breakers

How the original puts a wall of police cars, sawhorses and spike strips on the road ahead of the player, how long it
lives and what ends it, how the spike strips puncture tires and what the player's car must expose for that, and how
the "pursuit breaker" props stop the cops. The request side (when a roadblock is wanted) is summarised in
[ai-pursuit-cops.md](ai-pursuit-cops.md) section 7; the cars themselves (goal, brakes, collision weight) are in
[ai-pursuit-tactics.md](ai-pursuit-tactics.md) and [ai-pursuit-cop-cars.md](ai-pursuit-cop-cars.md).

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled; GameCube build), under
  `src/Speed/Indep/Src`: `AI/AIRoadBlock.h`, `AI/Common/AIRoadBlock.cpp`, `AI/Common/AIRoadBlockSetups.cpp`,
  `AI/Activities/AICopManager.cpp` (`CreateRoadBlock`, `PickRoadblockSetup`, `UpdateRoadBlocks`, `UpdatePursuits`,
  `ApplyBreakerZones`, `UpdateSupportCops`), `AI/Common/AIPursuit.cpp` (`RequestRoadBlock`, `AddRoadBlock`,
  `SpikesHit`, roadblock part of the pursuit task), `AI/Actions/AIActionStaticRoadBlock.cpp`,
  `Physics/Behaviors/{SpikeStrip,DamageRacer,Chassis,SuspensionRacer,RBVehicle}.cpp`, `Interfaces/Simables/ISpikeable.h`,
  `Generated/Events/EBreakerStopCops.hpp`, `Main/Common/EventSequencer.cpp`, `Libs/Support/Miscellaneous/CARP.h`.
  Read for understanding; no code copied.
- **Data inputs:** `pursuitlevels` (`roadblockprobability`, `roadblockspikechance`, `roadblockhelichance`,
  `SearchModeRoadblockChance`, `SearchModeRoadblockRadius`), `pursuitsupport` (`MinimumSupportDelay`, heavy options),
  `smackable` (`spikestrip`, `sawhorse`), `collisionreactions`, `L2RA.BUN` event sequences (pursuit breakers). Tables in
  [../formats/pursuit-data.md](../formats/pursuit-data.md).

Tags: [decomp] decompiled sources; [verified] read from the install. Metres and seconds. Angles in the setup tables are
**turns** (1.0 = 360 degrees), the game's `DEG2ANGLE(d) = d / 360`.

## 1. Request: when a roadblock is wanted

`AIPursuit::RequestRoadBlock` is called by the cop manager every run (0.5 s) for each pursuit. [decomp]

```
refuse if busted, bailed, a roadblock already exists, or roadblock_timer >= 0
refuse if there is no pursuitsupport or pursuit_time < MinimumSupportDelay (10 to 60 s, see data)
roadblock_timer = 8 + random(0, 4)
result = pending_next ? 4 : 0
p = target in sight ? roadblockprobability
                    : SearchModeRoadblockChance * (R - d) / R         # R = SearchModeRoadblockRadius, d = |last known position - target|
pending_next = random(0, 100) < p                                      # (p may be negative far from the last known position)
return result
```

`roadblock_timer` also drops by `dT` on the pursuit task and is set to 15 s when a heavy-support request is chosen. So a
roadblock is a **two-step lottery**: a successful roll arms `pending_next` (the speech system reads it as "roadblock
coming", `PendingRoadBlockRequest`); the next call, 8 to 12 s later, returns 4. When the manager gets a non-zero result
it latches it (`mIPursuitWithLatchedRoadblockReq`, cop count 4) and from then on calls `CreateRoadBlock` every run for
that pursuit until it succeeds, the pursuit ends or a roadblock exists. While a request is latched, **no new pursuit cops
are spawned** (the spawn call is skipped). The number 4 is passed along but `CreateRoadBlock` does not use it (the car
count comes from the road width). [decomp]

Per-level probabilities **[verified]** (`roadblockprobability` / `roadblockspikechance` / `SearchModeRoadblockChance` in
percent; `roadblockhelichance` is 0 on every level): heat 1: 0/0/0; heat 2: 10/0/3; heat 3: 25/0/5; heat 4: 30/70/7;
heat 5: 40/80/10; heat 6: 50/85/12; heat 7: 35/85/12; heat 8: 50/0/60; heat 9: 100/100/0; heat 10: 80/0/80. Race levels:
race 1 to 7: 0/0/0 except race 8 (35/0/50), race 9 (40/0/50), race 10 (50/0/50); `default`: 0/0/50.
`SearchModeRoadblockRadius` is 1000 m at heat 1 and 2, then 2000, 5000, 10 000, 10 000, 10 000, 3500, 4000, 5000 m for heat 3 to 10 [verified]
(race levels were not compared).

## 2. Building the roadblock (`CreateRoadBlock`)

### 2.1 Finding the spot

1. The target's perp attributes must exist, else fail.
2. Make a road nav (path type *cop*, direction type, cookie trail on), initialise it at the target's position pointing the
   way the target faces, and advance it **250 m** along the road (`kRoadBlockAheadDistance`). Junction choices are made by
   the nav's usual direction rule (zero preferred direction).
3. The segment under the nav must exist, allow traffic, not be a decision (junction) segment and be at least **40 m** long,
   and must have a lane profile with at least one zone. Otherwise fail (the request stays latched and retries next run, 0.5 s
   later, at a different spot as the target has moved).
4. Road width: the nav's left and right edge points (`GetLeftPosition`, `GetRightPosition`) give `crW = |left - right|`;
   `cr_width = crW + 1`.
5. Cars wanted: `widthIndex = min(int(cr_width / 4), 6)`; `MinCopsForWidth = {2, 2, 3, 3, 4, 4, 5}`;
   `cars_asked = MinCopsForWidth[widthIndex] + 1`.

### 2.2 Getting the cars

- Normal request: ask the pool for `cars_asked` inactive cop cars of class CAR and activate them; if the heavy
  "roadblock" support supplied four SUVs, use those (their support goal cleared, activated) instead.
- Every car taken from the pool is first un-spawned (removed from play, speech "unspawn" message, reason 3).
- If fewer than `cars_asked` were found, take active cop cars that are **out of view** (the cop manager's
  steal-from-out-of-view rule: off screen for over 5 s and over 100 m from view, the farthest first) to make up the number.
- If the total is under `cars_asked - 1`, fail. [decomp]

### 2.3 Choosing the layout

`roadblockspikechance`: `with_spikes = random(0, 100) < roadblockspikechance`; forced false when the pursuit is in
cool-down. `PickRoadblockSetup(width, cars, spikes)` searches the 15-entry normal list or the 9-entry spike list (sections
3 and 4) for the entry with the **smallest positive slack** `width - min_width` among those with
`cars >= required_vehicles`; no entry means failure. In words: use the biggest layout that still fits the road. [decomp]

### 2.4 Position and orientation

```
d2l = |left - target|^2, d2r = |right - target|^2
if d2l > d2r: across = left - right ; anchor = right      # anchor on the edge nearer to the target
else:         across = right - left ; anchor = left
centre = anchor + across * (min_width / 2) / crW           # the layout sits against the near edge and extends across
x_scale = clamp(cr_width / min_width, 1.0, 1.14)           # stretch x offsets up to 14 percent on wider roads
forward = nav forward vector (the road direction at the spot), frame = Util_GenerateMatrix(forward)
for element in layout (until the first 'none'):
    offset  = (element.x * x_scale, 0, element.z) rotated by the frame
    facing  = forward rotated about the vertical axis by element.angle (turns)
    pos     = centre + offset
```

(The sign convention of the rotation is the game's `RotateInXZ`; with angles of 0.25 and 0.75 cars end up across the road,
about 0.5 means facing against the road direction.) The roadblock activity stores `centre` and `forward` for the dodge
test. Layouts therefore span `min_width` metres of road starting at the edge nearer to the player, with depth
(`z`) of up to a few car lengths. [decomp]

### 2.5 Placing the elements

- **Car (`kCar`)**: the next car of the list (in order: pooled cars first, then stolen cars): `ResetVehicleToRoadPos(pos,
  facing)` (places it on the ground at the position with that heading), mark it spawned (state reset, repaired, fuel), and add
  it to the roadblock: the roadblock calls `StartRoadBlock` on it: lights on, in-pursuit flag set, no target, goal
  `AIGoalStaticRoadBlock` (brake held, see tactics spec). A roadblock car is *not* in the pursuit's cop list.
- **Barrier (`kBarrier`)**: a placeable `smackable` named `XO_Sawhorse_1b_00` (smackable collection `sawhorse`).
- **Spike strip (`kSpikeStrip`)**: a placeable named `XO_SpikeBelt_1b_DW_00` (smackable `spikestrip`); in addition a speech
  message `MReqRoadBlock(int(element.x))` named "Position" is sent (the integer part of the strip's x offset; the speech
  uses it to say on which side the strip is).
- Both prop kinds: height and surface normal from the world at `pos` (a rigorous ground query; if the normal points down
  it is flipped), matrix from the facing and the normal, `Place(matrix, true)`; added to the roadblock's smackable list
  (the list also counts spike strips) and `NotifySpikeStripDeployed()` is called on the pursuit **for every prop, barrier or
  strip** (the statistic "spike strips deployed" therefore counts sawhorses as well; cost to state 250 each).
- After all elements: `NotifyRoadblockDeployed` (cost 500). The speech system is told the roadblock was created
  (`MReqRoadBlock(1)` "Created") or failed (`MReqRoadBlock(0)`). `AddRoadBlock` on the pursuit ends the running formation
  unless it is FOLLOW or its finisher is active. [decomp]
- **Strip seating (rewrite).** The original reads one ground ray at the strip's position. A strip is 6.5 m across, so on
  a banked or sloping road an end can sink below the surface. The rewrite also reads the ground a half width either side
  along the strip's long axis (`roadblock/seat.rs`). With both ends found it stands the strip at the mean of the ends
  (never below the centre reading) and tilts it onto the line through them; a centre more than 5 cm above that mean
  (a crowned road) stays level at the centre height. A missing end keeps the centre reading. **[inferred]** as a fix for
  the reported strips below the road; not confirmed in play.

## 3. Layouts without spikes (`RoadblockCandidateList`)

`C` = car, `B` = barrier (sawhorse). Entries `(x, z, angle)` in metres, metres and turns. `min_w` = minimum road width
needed, `req` = cars required. **[decomp]** (data hard-coded in the code, not in AttribSys)

| # | min_w | req | Elements |
|---|---|---|---|
| 1 | 8.0 | 2 | C(-3.0, 0, .75) C(2.0, -.35, .27) |
| 2 | 12.0 | 3 | C(-5.2, -2, .875) C(0, -2, .12) C(4.5, -1, .87) B(-2.9, .2, .48) |
| 3 | 5.5 | 2 | C(-2, 0, .26) C(2, 5, .48) B(1.5, 1.7, .61) |
| 4 | 6.5 | 2 | C(-2, 0, .75) C(3, 4, .25) B(1.5, 1.7, .61) |
| 5 | 8.5 | 2 | C(-.9, 0, .72) C(4, 0, .32) B(-4.8, -.8, .54) |
| 6 | 15.0 | 4 | C(-5, 0, .71) C(1, 0, .29) C(5, 6, .24) C(8, 2, .5) B(4.1, -1.4, .57) B(6.1, -1.4, .43) |
| 7 | 13.0 | 3 | C(-4, 1, .26) C(-.9, -4.2, .75) C(5, -4, .26) B(-3.3, -2.2, .4) B(-5, -4, .46) |
| 8 | 11.4 | 3 | C(-4.4, -1.2, .65) C(0, 2.4, .24) C(4.4, -1.2, .35) B(0, -2.7, .49) B(2.1, 0, .35) B(-2.1, 0, .65) |
| 9 | 12.1 | 3 | C(-4.5, 0, .75) C(1, 0, .25) C(4, 4.5, .72) B(4.3, -1.2, .48) B(4.9, .3, .04) |
| 10 | 10.0 | 4 | C(-4.5, 0, .27) C(-.3, -4, .75) C(4.2, -4, .24) C(4, 0, .26) B(-4.9, -5, .51) B(-4.2, -3.3, .53) |
| 11 | 14.0 | 4 | C(-4.5, 0, .25) C(1, 0, .75) C(0, 5.4, .25) C(5.3, 5.4, .22) B(-2, 2.2, .37) B(5.4, -.2, .53) |
| 12 | 12.0 | 4 | C(-4.3, 0, .74) C(-5, 4, .75) C(.2, 4, .27) C(4, 0, .25) B(-.7, -1, .355) B(.7, -1, .635) |
| 13 | 22.0 | 4 | C(-9, 0, .74) C(-3, 0, .25) C(3, 0, .75) C(9, 0, .25) B(0, -3, .5) |
| 14 | 15.0 | 3 | C(-5.9, 0, .74) C(0, 0, .25) C(5.9, 0, .75) B(3, -2, .51) |
| 15 | 28.0 | 5 | C(-12, 0, .74) C(-6, 0, .25) C(0, 0, .75) C(6, 0, .75) C(12, 0, .25) B(-3, -3.4, .5) |

(The list ends with an all-zero terminator; a minimum width of 0.1 or less ends the search.)

## 4. Layouts with spikes (`SPIKES_RoadblockCandidateList`)

`S` = spike strip. Same notation. Used when `with_spikes` rolled true.

| # | min_w | req | Elements |
|---|---|---|---|
| 1 | 8.0 | 1 | C(-3, 0, .75) S(2, -.35, .52) |
| 2 | 15.0 | 3 | S(-5, 0, .51) C(1, 0, .29) C(5, 6, .24) C(8, 2, .5) B(4.1, -1.4, .57) B(6.1, -1.4, .43) |
| 3 | 15.0 | 2 | C(-4, 1, .26) C(-.9, -4.2, .75) S(5, -4, .52) B(-3.3, -2.2, .4) B(-5, -4, .46) |
| 4 | 14.1 | 2 | S(-4.8, 0, .5) C(1, 0, .25) C(4, 4.5, .72) B(4.3, -1.2, .48) B(4.9, .3, .04) |
| 5 | 12.0 | 3 | C(-4, 0, .27) S(-.3, -4.4, .5) C(4.8, -4, .24) C(4, 0, .26) B(-4.9, -5, .51) B(-4.2, -3.3, .53) |
| 6 | 13.0 | 3 | S(-4, 0, .51) C(-5, 4, .75) C(.2, 4, .27) C(4, 0, .25) B(-.7, -1, .355) B(.7, -1, .635) |
| 7 | 21.0 | 2 | S(-9, 0, .48) C(-3, 0, .25) C(3, 0, .75) S(9, 0, .52) B(0, -3, .5) |
| 8 | 15.0 | 2 | S(-5.2, 0, .51) C(0, 0, .25) C(5.9, 0, .75) B(3, -2, .51) |
| 9 | 28.0 | 4 | C(-12, 0, .74) C(-6, 0, .25) S(-1, 0, .51) C(5, 0, .75) C(11, 0, .25) |

The strips lie roughly across the road (angle near 0.5) at the road's near edge or in a gap between cars. [decomp]

## 5. Lifetime, dodging, cheating and clean-up

All on the cop manager's 0.5 s run (`UpdateRoadBlocks`) or the pursuit's 0.25 s task. [decomp]

- **Dodged.** While not yet dodged and with a pursuit: `h = dot(unit(centre - target.pos), roadblock_dir)`. When
  `h < 0` (the roadblock centre is behind the target relative to the road direction: the target has passed it) the
  roadblock is marked dodged: statistic `roadblocks dodged`, a speech message `MReqRoadBlock(0)` "Dodged", and if it had
  spike strips, `spike strips dodged` is increased by their number. A dodged roadblock is not counted again.
- **Distance to the target** (`GetMinDistanceToTarget`, from the pursuit task unless the target is hidden from cars):
  over the active, undestroyed roadblock cars: the minimum 3-D distance (and the nearest car), the minimum horizontal
  distance among cars within 1.5 m of the target's height; every car within **300 m** gets its "within engagement radius"
  flag. If this minimum distance is under **60 m** and below the chasing cops' minimum, the pursuit counts the target as
  *in sight* and resets the "time since any cop saw the perp" counter (a roadblock sees you). With no roadblock or pursuit:
  200.
- **Cheating.** If the minimum horizontal distance is under 500 m (`engage + 200`) and the target stays within **300 m** of the
  roadblock centre, a timer accumulates; at 20 s, or at once if the pursuit is in cool-down, the roadblock reports
  "perp cheating". Away from the centre the timer resets. When it fires and no roadblock cop has been recruited yet, the
  pursuit takes the roadblock car nearest to the target out of the roadblock into its own cop list (it becomes a normal chase
  car with `AIGoalPursuit`), and raises the number of cops needed to evade by one. Only one cop is recruited this way.
- **Spikes hit** (`IPursuit::SpikesHit`): if no roadblock cop has been recruited, take up to three roadblock cars
  (nearest to the target first) into the pursuit. No caller exists in the sources read (the event that would call it
  is in unread code), so treat it as an extension point.
- **Clean-up.** A roadblock is released when it has no cars: all its props are destroyed (`ReleaseAllSmackables`), the
  activity is released and detached from the manager. Cars leave it by being destroyed (also counted in `cops destroyed`),
  being recruited, being un-spawned by the cop manager's out-of-view rule (a roadblock car is never "in a pursuit", so the
  rule that keeps pursuit cops while they are near does not apply: once the car has been alive more than 8 s and is
  far enough out of view, or more than 10 s and out of view for the first time, it is removed, see
  [ai-pursuit-cops.md](ai-pursuit-cops.md) sections 2 and 5), or by the over-budget rule. When the last car goes, the
  sawhorses and strips vanish with it. A roadblock whose pursuit ended keeps its cars until the same rules remove them.
- **Roadblock cars are retired as cops:** destroyed roadblock cars are counted for the pursuit (`IncNumCopsDestroyed`) and a
  damaged one increments the roadblock's damaged count, which turns the follow formation off for good.
- **Siren.** Any cop in a roadblock sounds the wail siren. Speech distinguishes spikes ("Spikes") from a plain roadblock via
  `GetNumSpikeStrips() > 0`.

## 6. Spike strips and tires

### 6.1 The prop

`smackable/spikestrip` **[verified]**: parent `placeables`, `MASS` 100, `NO_CAR_EFFECT` false, `SimplePhysics` true,
`RESPAWN_TIME` 120 s, `KILL_OFF_SCREEN` 0, behaviour list `SpikeStrip`, no event sequencer, hit effects only. `sawhorse`:
parent `lightwoodenobject`, `MASS` 100, `NO_CAR_EFFECT` true (cars take no damage from it), `COST_TO_STATE` 50, flies apart: its `EventSequencer` is the shared engine `genericsmackable` **[verified]**, whose fragment events name six parts,
four of which the sawhorse's hierarchy (`XO_SAWHORSE_1B_00`: `SAWHORSE_FRAG01..04`, collision children `863C2522`,
`8395A1C1`, `4832AE9D`, `4C1B1E37`) and its bounds hold; the last event hides the parent **[verified]**.
The roadblock props are created already positioned; a placeable starts as a trigger volume and is turned into a simulated
object when a body reaches it (the `RBVehicle` trigger flags: 0x20 for every vehicle, plus 4 for the player or 8 for
others, plus 0x20000 for cops). The spike strip behaviour, each simulation step while it is simulated: if its collision
map shows any collision, test every body that touches it; if nothing touches it, it goes back to a trigger volume and the
simulated object is removed. [decomp]

### 6.2 The test (`SpikeStrip::OnCollide`)

For each body in contact: it must implement the spikeable interface **and** the suspension interface (only racer-class
cars; see [ai-pursuit-cop-cars.md](ai-pursuit-cop-cars.md) section 4). For each wheel `i` that is on the ground and whose
tire damage is *none*:

```
tire box: oriented with the car body, centre at the wheel centre position, half extents (0.15 along the axle,
          wheel radius, wheel radius), swept by the car's velocity * dT during this step
strip box: the prop's collision half-dimensions at the prop's transform
if swept_box_overlap(strip box, tire box): Puncture(i)
```

The box-versus-box intersection routine is not in the sources read (it is empty in the decompilation); a swept
oriented-box test (separating axes with the sweep) is the intended behaviour. [decomp][unconfirmed] The strip box is the
prop's collision geometry: the install's `XO_SpikeBelt_1b_DW_00` has one box, half extents 0.378 (along its facing), 3.265
(across it) and 0.254 (up) m, centred 0.013 m above the prop's origin, so the strip is 6.53 m wide. [verified] (track prop
bounds, `installed_roadblock_props_have_bounds_and_models`). The sawhorse `XO_Sawhorse_1b_00` has four boxes: a 1.0 x 2.77 x
1.4 m body and a 0.12 x 3.16 x 0.59 m top rail between two 1.26 x 0.07 x 1.0 m legs. [verified]

### 6.3 Tire damage (`DamageRacer`)

Per wheel state: `NONE (0)`, `PUNCTURED (1)`, `BLOWN (2)`. `Puncture(i)` (wheel index 0 to 3, only from NONE, blowouts
enabled): state PUNCTURED, `blow_timer = 0.5 s` (or **180 s** if the car has run-flat tires, `Physics::Info::HasRunflatTires`
on its `pvehicle`), raise the event `ETirePunctured(car, wheel)`. Each physics step: for punctured wheels
`timer -= dT`; below zero: state BLOWN, `ETireBlown(car, wheel)`. `ResetDamage` clears all. [decomp]

Effects of BLOWN, as the vehicle physics must expose them:

- `Chassis` builds `state.blown_tires`, a bit mask (bit `i` for wheel `i`) from the wheels in state BLOWN (a PUNCTURED wheel
  does nothing to the physics).
- `SuspensionRacer` then, for each blown wheel: `ebrake = 0`, `brake = 1` (a permanently braking wheel) and the traction
  boost scaled by **0.3** (see [vehicle-suspension-tires.md](vehicle-suspension-tires.md) and
  [vehicle-steering-assists-aero.md](vehicle-steering-assists-aero.md)). Nothing else changes: no pull to the side beyond what
  braking one wheel does, no pressure loss model, no reduced top speed.
- Draw/sound: `DrawCar` passes the blown mask to the renderer (wheel wobble per blown wheel, a blow-out noise record
  `(4, 1, 0, 10)` mixed into the road noise while the wheel touches the ground); `SoundCar` passes the per-wheel tire state
  to the sound system (the road noise uses the `blown_tire` surface noise); `SoundAI` listens for the blow-out moment
  ("TireBlo") for cop speech. The player HUD reads the per-wheel blown flag (local player update).
- AI racers can be spiked by a roadblock that is on their path like the player; cops and traffic cannot.

### 6.4 What the Rust vehicle needs

A per-wheel `tire_damage: {None, Punctured(timer), Blown}` with `puncture(wheel)`, `is_wheel_on_ground(i)`,
`wheel_center(i)`, `wheel_radius(i)`, the body transform and velocity; a bitmask handed to the tire model, and two events
(punctured, blown) for sound, HUD and speech. The strip test needs the strip's oriented box and the swept tire box.

## 7. Pursuit breakers

A *pursuit breaker* is a destructible set piece (a donut shop roof, a gas station canopy, a water tower, ...) that, when
wrecked, stops nearby cops. In the sources the whole mechanism is one message and one list. [decomp][verified]

- **Authoring.** The prop's destruction sequence (an event sequence in `L2RA.BUN`, `CarpEventSequences` chunk `0x8003B810`,
  73 `0x3B811` sequence chunks, see [../formats/pursuit-data.md](../formats/pursuit-data.md)) fires the event
  `EBreakerStopCops {radius, duration, position}`. In the install **21 such events occur in 15 of the 73 sequences**, all with
  `duration = 5.0 s` and radii of 5, 9, 10 (7 times), 12 (3), 16 (2), 20 (4) or 24 (2) metres; the sequences belong to the
  donut shop (roof and donut sign), drive-in, gas station roof, gazebo, greenhouse, construction scaffold and heavy concrete
  prop, a base on the hill, the radio tower and its supports, the sailboat, the seafood patio, the water tower and its
  supports, and a wooden scaffold. The event posts the message `MBreakerStopCops` with the same three values.
- **Effect.** The cop manager keeps a list of *breaker zones* `{position, end_time = now + duration, radius}`. Each manager
  run (first thing, every 0.5 s): for every vehicle in the manager's list (cop cars and the helicopter) that is not
  already destroyed and has pursuit AI, if its **horizontal** distance to any zone's position is below the zone radius, the
  vehicle's damage behaviour is told to `Destroy()` (the vehicle counts as destroyed at once and goes through the
  usual wreck path, `EVehicleDestroyed`). Then zones whose end time has passed are dropped. Note that the helicopter is
  destroyed too if it is above a zone (height is ignored).
- The **speed-breaker** (`EPursuitBreaker`, `ToggleGameBreaker`) is the player's nitrous-like slow-motion ability and has
  nothing to do with the cops.

## 8. Constants not in AttribSys

| Name | Value |
|---|---|
| Roadblock ahead distance | 250 m along the cop path |
| Minimum segment length | 40 m |
| Cars asked | `MinCopsForWidth[min(int((w + 1) / 4), 6)] + 1` with `{2,2,3,3,4,4,5}` |
| X stretch | `clamp((w + 1) / min_w, 1, 1.14)` |
| Roadblock request timer | 8 + random(0, 4) s; 15 s after a heavy request |
| Roadblock sees you | under 60 m; cop engagement radius 300 m |
| Cheat | within 300 m of the centre for 20 s (or cool-down), pre-condition under 500 m |
| Tire blow time | 0.5 s (run-flat 180 s); blown wheel: brake 1, traction x0.3 |
| Tire box half width | 0.15 m |
| Breaker zone duration | 5 s |

## 9. How to check it

1. Count cars by width: on a two-lane road (about 9 m, `cr_width` 10) expect `int(10/4) = 2`: 3 + 1 = 4 asked, a layout of
   min width 8 or 8.5 (3 or 2 cars); on a four-lane highway (about 18 m) 5 to 6 cars and a 15 to 22 m layout.
2. Spikes: at heat 5 (80 percent spike chance) most roadblocks contain a strip; run over it with one wheel: after 0.5 s
   that wheel brakes constantly and the car loses grip (traction x0.3). A run-flat car keeps driving 180 s.
3. Cheating: park 200 m before a roadblock for 20 s: the nearest roadblock car should join the chase.
4. Dodge: pass a roadblock: the "roadblock dodged" message and counter should fire as soon as you are beyond its centre.
5. Breaker: wreck a donut-shop roof with cops within 12 to 16 m: all of them are destroyed together, and cops arriving
   within the next 5 s inside the radius are destroyed too.

## Open questions

- **Q1** The rotation sign of `RotateInXZ` and the handedness of the layout `x` axis (which side of the road is the near
  edge for the player) are unverified; the rewrite follows the lib's convention (side = forward turned clockwise) and has
  not been compared with the running original.
- **Q2** The trigger-to-simulated hand-over of placeables (flag values 0x20, 4, 8, 0x20000) is partially understood; the
  rewrite tests every strip against the player's tires on each step, which gives the same result.
- **Q3** `IPursuit::SpikesHit` has no caller in the sources read (nor in the PC binary's roadblock code). It is not run.
- **Q4** `AIGoalHeliRoadBlock` is requested for a helicopter but has no definition; `roadblockhelichance` is 0 everywhere. The
  "helicopter spike strip" counter (225 cost to state each) exists only as a statistic: the helicopter never drops spikes.
- **Q5** `Place(matrix, true)`: the meaning of the boolean (probably "snap/activate") is unknown.
- **Q6** That only racer-class cars carry the spikeable interface is read from the `pvehicle` damage-behaviour names (the
  racer, player and challenge-series cars use `DamageRacer`; cops and traffic do not). Spot-checked, not exhaustive. So the
  original does **not** puncture cops or traffic; the rewrite has no AI racers in free roam, so only the player is spiked.
- **Q8** `SearchModeRoadblockRadius` is not 1000 m on every heat row, as this spec first said: 1000, 1000, 2000, 5000, 10 000,
  10 000, 10 000, 3500, 4000, 5000 m for heat 1 to 10 **[verified]**. The helicopter's search reads the same field.
- **Q7** The return convention of the builder (the support spec's Q3) is settled: success is non-zero in both callers
  ([../provenance/ai-pursuit-roadblocks.md](../provenance/ai-pursuit-roadblocks.md)).

## Rust implementation notes

- `blackbox_pursuit::roadblock`: `NORMAL` and `SPIKES` tables, `cars_asked`, `pick`, `place` and `centre` (pure).
  `roadblock::request` is the lottery and the latch (`Requester`, `Dispatcher`, the per-level `odds`).
  `roadblock::standing` is a standing roadblock's rules (`Standing`, `reach`). `blackbox_pursuit::spikes` is the strip box and
  the tire box test.
- The world side is `crates/nfsmw/src/scenes/world/ai/roadblock/`: `site` (the original's site checks), `build` (the layout,
  cars and props, whole or not at all), `director` (the manager's run every 0.5 s), `props` (installed models and bodies),
  `state` (the standing roadblocks).

## Rewrite status

Tags: **[verified]** by a test that ran here, **[headless]** by the synthetic world tests, **[unseen]** not run in a live game.

- **Dispatch [headless].** `RequestRoadBlock`'s two-step lottery (8 to 12 s timer, the armed flag, the percentage roll, the
  out-of-sight chance falling with the distance from the last known place), the manager's latch and the retry on every run
  are `roadblock::request` and `ai/roadblock/director.rs`. While a request is latched no pursuit cops are spawned
  (`ai/pursuit.rs`). The per-level odds are the install's rows (a test compares the spike column with the layout picker;
  an installed test compares all columns with the attribute database).
- **Heavy support [headless].** A `HEAVY_ROADBLOCK` roll hands the request to the same builder with SUV cars (the big SUV with
  the option's `ChanceBigSUV`), retries each run for up to 60 s (the original retries without end), holds the lottery for 15 s
  and keeps the support roll silent for the option's duration once built.
- **Site [headless].** 250 m ahead on the cop road; refused on a decision segment, a no-traffic segment, a segment under 40 m or
  a road without lane zones (the original's checks), then the rewrite's: the layout must lie within the road's edges, no other
  vehicle within 4.5 m of any element, the player not within 14 m. The original has **no** view test: a roadblock is placed in
  full view of the player (that is how the player sees it ahead), so none is added.
- **Layout [headless].** Width is the road's width plus one metre (as the PC binary computes it; the earlier port passed the bare
  width). Cars are placed whole or not at all: a car that cannot stand fails the build and leaves nothing behind.
- **Props [unseen].** Sawhorses are rigid bodies in the prop world (`PropWorld::spawn_placed`) with the installed collision
  boxes and the `sawhorse` smackable's dynamics, drawn from the installed model; they live as long as the roadblock. A sawhorse also has a scenery model: a first substantial contact sends `IMPACT_1`,
  the `genericsmackable` sequence releases the four pieces and hides the whole [headless: installed test
  `installed_sawhorse_breaks_into_its_four_pieces_when_a_car_hits_it`; the pieces' meshes are the shared sections']. Spike
  strips are drawn from the installed model, tilted to the ground normal, with the installed collision box; the tire box is the
  original's (0.15 m along the axle, the radius in the other two axes) swept along the step. Not seen in a running game.
- **Behaviour [headless].** Cars hold the brake (`AIGoalStaticRoadBlock`). A roadblock car does not look for the player; the roadblock
  sees the player within 60 m (3-D) unless a hiding spot hides it from cars, which resets the pursuit's unseen clock. Passing
  the centre marks the roadblock dodged once (the dodged and strip counts are kept for the statistics). The cheat rule (a car
  within 500 m horizontally, the player within 300 m of the centre for 20 s, at once in cool-down) takes the nearest car into the
  chase once. A standing roadblock that is whole and unpassed forces the follow formation, and adding one ends the current
  formation unless it is FOLLOW or its finisher runs. The cost to state is 500 per roadblock and 250 per prop, sawhorses included.
- **Release and repeat [headless].** A roadblock with no live car is released and its props go with it; the lottery may then build
  another. An escape leaves roadblocks standing (their cars are retired by the cop manager's out-of-play rule: a roadblock car
  is never "in a chase", so only the settling and the window apply, `ai/population/cops.rs`); ending the chase by
  hand or by the arrest takes them away.
- **Cars from cops out of view [headless].** A lottery roadblock first takes roaming cops (patrols and fleeing cops, not the
  chase's) that have been out of view more than 5 s and are over 100 m from the player, the farthest first, and removes
  them; the cars it still lacks are created (`roadblock/steal.rs`). The original asks its pool of inactive cars first;
  the rewrite has no pool yet, so the order is reversed until the cop manager's pool lands.
- **Not built.** The pursuit statistics reaching the rap sheet (`Roadblocks::stats` holds them); the
  "cops needed to evade + 1" count; a roadblock marker on the map. The in-game minimap's constructor (`FUN_0059d500`) loads
  cop, racer, checkpoint and helicopter objects and **no** roadblock object, so the minimap has no roadblock blip. The
  `MMICON_ROADBLOCK_%d` pool belongs to the full-screen map's legend builder (`FUN_00531350`, listed in
  [../provenance/world-map-ghidra.md](../provenance/world-map-ghidra.md)); whether the map screen draws standing
  roadblocks was not examined. The pursuit board has a `PursuitAddsRoadblock` string (a HUD/tip hook) that was not traced.
