# The police helicopter

The helicopter that joins a pursuit: when it appears, how it sees the player, the flight controller and the point-mass
flight model, its behaviours (follow, skid-hit, search, exit), the height-sheet data that keeps it above the buildings, fuel
and the audio and visual hooks. There is no gunfire, no spotlight and no spike drop in the sources read: it is a flying
camera-bait that sometimes rams the player's car.

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled; GameCube build), under
  `src/Speed/Indep/Src`: `AI/AIVehicleHelicopter.h`, `AI/Common/AIVehicleHelicopter.cpp`,
  `AI/Actions/AIActionHeliPursuit.cpp`, `AIActionHeliExit.cpp`, `AI/Common/AIGoal.cpp` (heli goals),
  `AI/Common/AIPursuit.cpp` (`CopRequest`, `AssignChopperGoal`, `SkidHitEnabled`), `AI/Activities/AICopManager.cpp`
  (`SpawnPursuitHelicopter`), `Physics/Behaviors/{SimpleChopper,DamageHeli,DrawHeli,SoundHeli}.cpp`,
  `Interfaces/Simables/IHelicopter.h`, `World/HeliSheet.{hpp,cpp}`, `World/HeliRenderConn.cpp`,
  `EAXSound/CARSFX/SFXObj_Helicopter.cpp`, `EAXSound/sfxctl/SFXCTL_Helicopter.cpp`, `EAXSound/States/STATE_Helicopter.cpp`.
  Read for understanding; no code copied.
- **Data inputs:** `pursuitlevels` (`HeliFuelTime`, `TimeBetweenHeliActive`, `heliLOSdistance`, `SearchModeHeliSpawnChance`,
  `SearchModeRoadblockRadius`, the `cops` record `copheli`), `pursuitsupport` (`AirSupportOptions`,
  `MinimumSupportDelay`), `chopperspecs`, `pvehicle/copheli`, `rigidbodyspecs/chopper`, `damagespecs/choppers`, and the
  `HeliSheet` chunk (format in [../formats/pursuit-data.md](../formats/pursuit-data.md) section 1).

Tags: [decomp] from the decompiled sources; [verified] read from the install. Metres, seconds, m/s. Vectors are
world-space with `y` up. Notation: `AddScale(a, b, s) = (a + b) * s` and `ScaleAdd(a, s, b) = a * s + b` (inferred from the
call sites; see Q1).

## 1. The vehicle

`pvehicle/copheli` **[verified]**: class `CHOPPER`, `MASS` 20 000 kg, model `COPHELI`, behaviours: rigid body `RBVehicle`,
input `PInput`, "engine" **`SimpleChopper`**, damage `DamageHeli`, draw `DrawHeli`, sound `SoundHeli`, no suspension.
`rigidbodyspecs/chopper`: gravity -9.81, **`NO_WORLD_COLLISIONS` true** (it flies through static world geometry),
`IMMOBILE_OBJECT_COLLISIONS` true, ground and object collisions on (it can hit cars and the ground), `CG` (0, 0, 1.2),
world and ground moment scales 1e6 (it does not tumble from world contact), object elasticity 0.05 along z,
`COLLISION_BOX_PAD` 0.18. `damagespecs/choppers` has `HIT_POINTS` 0: no hit-point damage can destroy it; only an explicit
`Destroy()` (a pursuit-breaker zone, [ai-pursuit-roadblocks.md](ai-pursuit-roadblocks.md) section 7) does. `chopperspecs`:
`copheli` uses the `default` collection; `qchopper` and `henchchopper` exist but no vehicle references them. [decomp][verified]

**No car links.** The record has no `chassis`, `tires`, `brakes`, `engine`, `transmission`, `induction` or `nos` link, not even
an inherited one: its lineage is `copheli` -> `choppers` -> `default`, and a car's record (`bmwm3gtr`, `copmidsize`) has all of
them. [verified], checked by the install test `the_helicopter_record_has_no_car_links`
(`crates/nfsmw-data/tests/real_install/pursuit.rs`). The car reader
`car_physics` needs the chassis, tires and brakes, so the game layer loads the helicopter as a model only:
`load_helicopter_model` (`crates/nfsmw/src/scenes/world/ai/models.rs`) uploads its rig, and no `CarPhysics` is built for it.
The helicopter's flight is `SimpleChopper` (section 5), not the car physics.

The AI is `AIVehicleHelicopter` (derived from the cop pursuit driver). It thinks every 0.125 s like the cop cars.
There is at most **one** helicopter at a time (`gHeliVehicle`, `HeliVehicleActive()`).

## 2. When it appears and leaves

### 2.1 Spawn request

The cop manager asks the pursuit for the next cop type (`CopRequest`). The pursuit allows a helicopter when its heli timer
is negative and the `pursuitsupport` of the target's level has `MinimumSupportDelay` smaller than the pursuit time. Rules,
all [decomp]:

- The heli timer starts at 10 s when a pursuit is created, counts down with the pursuit's 0.25 s task, and is reset to
  `TimeBetweenHeliActive` when a helicopter leaves the pursuit.
- The normal wave pick (`cops` record list, [ai-pursuit-heat.md](ai-pursuit-heat.md)) lists `copheli` with count 1 and a
  chance weight; its weight is forced to 0 unless allowed above. The type is picked with probability proportional to the
  weights of types that still lack members (`copheli` count needed = 1 minus helicopters in the contingent).
- **Search-mode spawn.** When the cool-down gauge first appears (target lost) the pursuit sets `do_test_for_heli_search`;
  at the next request, if no helicopter is active and the target is not in sight, with probability
  `SearchModeHeliSpawnChance` percent (heat 4: 2, heat 5: 4, heat 6: 5, zero elsewhere **[verified]**) it forces the next
  spawn to be a helicopter ("copheli"), and the speech system announces a quadrant search.
- A forced or chosen helicopter bypasses the ground-cop caps (`mMaxActiveCopHelicopters` is 1).

### 2.2 Placement (`SpawnPursuitHelicopter`)

If `HeliVehicleActive()` or no inactive helicopter exists, fail. Otherwise: nav at the target position pointing **backward**
(opposite the target's heading), cop path type, direction type; advance **250 m** along the road (if the nav is invalid,
use the target's position and heading); `y += 20`; activate; place with an orientation along the nav forward; mark spawned
(`SetSpawned` -> `SetFuelFull`, which sets `gHeliVehicle` and reads `HeliFuelTime` **from the local player's** pursuit level);
add to the pursuit (it gets `AIGoalHeliPursuit`); active helicopter count + 1; the pursuit counts a heli spawn (cost to
state 2000). [decomp]

### 2.3 Fuel and leaving

`mHeliFuelTimeRemaining` counts down by `dT` every think (fuel is real time, not pursuit time). At 0 the goal switches
to `AIGoalHeliExit`. Other ways to the exit goal: the pursuit ends or is bailed (the cop manager calls `StartFlee`; for a
chopper it picks `AIGoalHeliExit`). When the exit action reports finished the goal un-spawns the helicopter
(`UnSpawn` clears `gHeliVehicle`); the manager's active-heli count drops and the next helicopter is delayed by
`TimeBetweenHeliActive`.

Values per level **[verified]** (`HeliFuelTime` s / `TimeBetweenHeliActive` s / `heliLOSdistance` m / `copheli` weight):
heat 1: 10/0/251/-, heat 2: 10/0/251/-, heat 3: 10/120/251/-, heat 4: 60/180/251/50, heat 5: 90/180/251/60, heat 6:
180/180/251/50, heat 7: 10/180/251/-, heat 8: 75/120/301/60, heat 9: 120/60/301/80, heat 10: 400/5/301/100; race
levels: 1: 10/120/251/-, 2: 20/120/251/-, 3: 60/120/251/-, 4: 80/120/251/60, 5: 120/120/251/60, 6: 120/120/251/50, 7:
220/120/251/60, 8: 75/120/301/75, 9: 120/120/301/80, 10: 120/120/301/90; `default`: 20/20/201/-. (`-`: not in the `cops`
list.) `MinimumSupportDelay`: 10 to 60 s ([ai-pursuit-cops.md](ai-pursuit-cops.md) section 6).

### 2.4 Who the helicopter chases

`AIVehicleHelicopter::Update` sets its target to the **local player's** vehicle every think, whatever the pursuit target
is. It also runs the cop-car housekeeping: spawn timer, targeting (line-clear flag), fuel, goal update. The shadow scale
is `1 - (y - (ground + 5)) * 0.025` (only if the ground height is non-zero). [decomp]

## 3. Seeing the player (`CanSeeTarget`)

Every 0.25 s (same timer as the cars) while in pursuit:

```
hidden = target.IsHiddenFromHelicopters()                 # a hidden zone whose data is 'Heli' or anything but 'Car'
if hidden and latched: return false ; latched = false
dist = |target - heli|  ; los = pursuitlevels.heliLOSdistance of the target (250 m default if none)
in_sight = dist <= 75                                     # visual sphere radius kHeliVisualSphere
if not in_sight and dist < los: in_sight = dot(heli.forward, unit(target - heli)) > 0       # in front half space
if in_sight:
    if the main game view is in a tunnel: in_sight = false
    else ray (heli -> target + 1 m up) against the world; a hit: in_sight = false ; no hit: last_seen_place = target
if not in_sight:
    if hidden: latched = true
    elif dist < 150 and |target - last_seen_place|^2 < 400:  return true       # grace: target near where it was last seen
    return false
return true
```

The hidden-zone latch is the same as for cars. A hidden zone flagged `Heli` hides the target from helicopters only; one
flagged `Car` from cars only; any other value hides it from both. [decomp]

## 4. The controller (`AIVehicleHelicopter::OnDriving`)

The helicopter does not use the PID driver. Each action sets, through the shared AI interface, a **drive target** (3-D point
`P`), a **drive speed** `vmax`, a **destination velocity** `Vd` (the velocity of the thing it follows; stored as
`AddScale(v, 9 v, 0.1) = v`, i.e. unchanged) and a **look-at** point, then calls `DoDriving(7)`:

```
pos, V = body position, linear velocity
work   = pos + V * 0.45                 # LeadPositionTime
ahead  = pos + V * 0.65                 # FarLeadPositionTime
dest   = P + (Vd.x, 0, Vd.z) * 0.45
AvoidCamera(dest)                       # section 4.1
CheckHeliSheet(...)                     # section 4.2  (may raise dest.y, produce smoothing_vel, set body velocity)
move   = dest - work ; len = |move| ; yd = move.y ; move.y = 0
closing = dot(V - (Vd.x, 0, Vd.z), unit(move))
MaxDeceleration = false
if closing > 3:
    avail = min(80, speed * 0.2 + 30) ; used = min(closing * 2, avail)
    D = closing^2 / (1.55 * used)
    if len <= D: vmax *= 0.4 ; MaxDeceleration = true
elif dot(V, unit(move)) < -1 and len < 15: MaxDeceleration = true
if len > 2: move = move * (vmax / len)               # horizontal desired velocity: direction * speed
# vertical: climbing quickly, falling slowly
if 0 <= yd < 7:   yd = yd * 5  + (Vd.y > 0 ? 2 * Vd.y : 0)
elif yd < 0 and yd > -5: yd = yd * 3 + (Vd.y < 0 ? Vd.y : 0)
move.y = yd                                           # (yd >= 7 or <= -5 is used as is)
if |smoothing_vel| > 0.1: move = move * 0.7 + unit(smoothing_vel) * (vmax * 0.3)
SetDesiredVelocity(move) ; SetDesiredFacingVector(look_at - pos)
```

Note `vmax` is a horizontal speed limit; the vertical component is not limited by it (a 20 m height error asks for 20 m/s
up), but the physics clamps the total to `MAX_SPEED_MPS`. [decomp]

### 4.1 `AvoidCamera`

Let `c` be the active game camera position plus 4 m along its forward direction. Compute the segment from the heli to
`pos + V * 0.2` and test against a sphere centred on `c` of radius `CameraRadiusToAvoid = 7` (reduced to `distance - 0.5`
when the heli is already inside the sphere but farther than 2 m from `c`). On a hit with outward normal `n`: set `dest` to
the segment end moved onto the sphere surface along `n` (`dest = work + n * dot(hit - work, n)`), and kick the body velocity:
`V += n * |dot(V, n)| * ((7 - radius) * 0.5 + 1.2)`. The helicopter therefore never flies through the player's camera. [decomp]

### 4.2 `CheckHeliSheet` (height floor and sliding)

Uses three independent sheet coordinates (each caches the last triangle and the last elevation): primary for `dest`, a
*secondary* one at the working position and a *third* one at the look-ahead position. With `elev(p)` the sheet elevation at
world `(x, z)` (sheet coordinates `(z, -x)`, section 5) and a flag whether any triangle covers the point:

```
FilterHeliAltitude(dest): if sheet covers dest.xz: dest.y = max(dest.y, elev) , else leave
third = elev(ahead.xz) with normal n3:  if covered and third > ahead.y:
        ahead.y = third ; dir = unit(ahead - pos) ; n2 = (-n3.y, 0, n3.x) * (0.4 * |n2 x dir|)
        smoothing_vel = dir + n2                    # steer around a rising roof rather than into it
second = elev(work.xz) with normal n: if covered and second > work.y:
        work.y = second ; scale work-pos vector so |work - pos| equals |dest - pos| when it is longer than 0.2 m
        dest.y = max(dest.y, work.y) ; dir = unit(work - pos)
        n2 = (-n.y, 0, n.x) * (0.3 * |n2 x dir|) ; dir += n2
        body velocity = dir * current speed          # hard-steers the velocity up and around the surface
return whether the secondary coordinate was covered
```

If no sheet triangle covers the point (outside the drivable sections) nothing is changed. A debug flag `bIgnoreHeliSheet`
(always false in shipped play, because `NeverIgnoreHeliSheet` is true) would also disable it. [decomp]

## 5. The flight model (`SimpleChopper`)

Per physics step with step `dT`; `Vdes` = desired velocity from the AI; `MaxDecel` flag:

```
body offset: the rigid body is shifted by (0, -half_height, 0) rotated into the body frame (the previous offset is removed first)
if |Vdes| > MAX_SPEED_MPS: Vdes *= MAX_SPEED_MPS / |Vdes|                              # 100 m/s for copheli
max_accel = min(80, speed * 0.6 + 30) ; if MaxDecel: max_accel = 80
dv = Vdes - V ; a_req = |dv| * 2                                                       # Chopper_Ratio = 2 (0.5 s response)
accel = (a_req > 1 and (a_req > max_accel or MaxDecel)) ? dv * (max_accel / a_req) : dv * 2
force = accel * mass ; force.y += mass * 9.81                                          # rotor lift cancels gravity
apply force (no torque)
# attitude: tilt the body towards the acceleration, turn it towards the facing vector
meas = (V - V_prev) / dT                                                               # only if dT > 0.005
meas_local = to body frame ; meas_local *= 0.2                                         # the "low-pass" has no memory (see Q2)
pitch = -(meas_local.z * 9 / 80) * PITCH_ANG ; if meas_local.y > 0 and speed > 15 and V.y > 0: pitch += meas_local.y * 0.035
pitch = clamp(pitch, -0.1, 0.1)                                                        # turns (36 degrees)
ideal_forward = body forward with y = sin(2 pi pitch) ; torque_x = (ideal x actual).x in body frame
ang.x = clamp(-torque_x * PITCH_ALIGN_SCALE, -1, 1)
roll  = -(meas_local.x * 12 / 80) * ROLL_ANG ; clamp +-0.1 ; ideal_right.y = sin(2 pi roll)
ang.z = clamp(-torque_z * ROLL_ALIGN_SCALE, -1, 1)
yaw:   ideal = desired facing (y = 0, normalised) ; actual = body forward (y = 0) ; ang.y = clamp(-cross(ideal, actual).y * 8, -1.3, 1.3)
ang = (ang + 7 * ang_prev) * 0.125                                                     # 8-step smoothing
set the body angular velocity = ang (world frame) ; ang_prev = ang ; V_prev = V
```

`chopperspecs/default` **[verified]**: `MAX_SPEED_MPS` 100, `PITCH_ANG` 0.05, `ROLL_ANG` 0.06, `PITCH_ALIGN_SCALE` 6,
`ROLL_ALIGN_SCALE` 4.5. The other 19 fields of the class (`YAW_*`, `STRAFE_SCALE*`, `AIR_RESISTANCE*`, `DRIVE_SPEED`, ...) are not
read by the code found: the "simple" chopper ignores them. The helicopter is thus a point mass with 0.5 s velocity response,
at most 80 m/s^2 acceleration (30 + 0.6 v), perfect lift, and a body that leans and yaws visibly. [decomp][verified]

## 6. Behaviours

Goal `AIGoalHeliPursuit`: actions `AIActionHeliPursuit` (score 0), `AIActionTooDamaged` (1.0). Goal `AIGoalHeliExit`:
`AIActionHeliExit`, `AIActionTooDamaged`. `HeliPursuit.IsFinished` reports true after 10 s and the "restart" flag exists, but the
goal's selection never re-selects the current action, so nothing changes at 10 s: the action simply continues. The
mode inside the action decides the behaviour. [decomp]

### 6.1 `AIActionHeliPursuit` modes

State: `pursuit_mode` in {straight line, search, skid-hit approach, skid-hit strike}, `skid_timer` (starts at 0, counts down
every think), `collision_abort` counter, player body/position/speed refreshed every think; at start a path to the
player's position is requested (`StartPathToPoint`; the result is not used by the modes below). Mode switch each think: if
the pursuit says the target is out of sight, switch to **search** (choosing the first destination); when sight returns,
back to straight line.

**Straight line** (follow the player):

```
lead = min(45, 30 + 0.4 * player_speed) ; if SkidHitEnabled and cooldown over: lead *= 0.75
seek = player_pos + player_forward * lead
seek.y = player.y + (skid_timer > 0 ? 2 : (dist_to_player > 55 or skid_timer > -5) ? 12 : 6)     # height above the player
d = horizontal distance to seek
along = dot(unit(seek - heli), player_velocity) ; if along < 0: along *= 0.5
close = d > 15 ? (d > 300 ? 50 : 30) : (d > 3 ? 20 : d)
drive_speed = close + along ; dest_velocity = player_velocity ; look_at = seek (or the player if d < 5 and player_speed < 15)
```

**Trigger of the skid-hit**: `SkidHitEnabled` = the target's `pursuitsupport.AirSupportOptions` contains the strategy
`SKID_HIT` (2). (The other strategies in that table, `HI_PATROL`, `PURSUIT`, `SPIKE_DROP` and the options' chance and duration
fields, are never consulted by the AI; only the presence of a `SKID_HIT` entry matters.) With it enabled, the cooldown
over (`skid_timer < -5`), the helicopter between 5 and 35 m from the player, ahead of the player's velocity direction
(`dot(heli - player, unit(player velocity)) / dist > 0.707` in the horizontal plane) and less than 13 m above it: set mode
*approach*, `skid_timer = 8`, zero the offset, and the speech says a self-strategy line. `SKID_HIT` is present on levels
**[verified]**: heats 5, 6, 7, 9, 10 and race levels 5, 6, 7 (any chance; heat 7 has chance 0 but the entry exists).

**Skid-hit approach**: side `s = +6` m if the heli is already on the positive side of the player's right vector (`dot(heli - player, right) >= 0`), else `-6`; `seek = player_pos
+ right * s + player_velocity * 0.23 + (0, 1.8 (+3 if the heli is under 2 m above the player), 0)`. When the horizontal
distance to `seek` is under 4 m and `|dot(heli - player, right)| > 1.9`, switch to **strike** and make sure `skid_timer >= 2`
(speech "intent to ram"). **Strike**: `seek = player_pos + player_velocity * 0.092 - 0.5 * side_offset + (0, 0.4, 0)`. In both,
`look_at = player_pos + player_velocity`, `drive_speed = 100`, dest velocity = the player's velocity. The helicopter
slides sideways into the car at its own height. Abort (back to straight line when `skid_timer <= 0`): `skid_timer = 0` if
the heli is ahead of the player and farther than 40 m, or below the player, or behind it and farther than 12 m; the body
collision listener adds 2 to `collision_abort` for a ground hit and 5 for a hit on the player while `skid_timer > 0`, and
at more than 20 the skid is aborted. `collision_abort` decays by 1 per think. [decomp]

**Search** (target out of sight): around the pursuit's last known position `C`: `R = SearchModeRoadblockRadius` (1000 m);
choose a point at distance `random(0, 0.7 R) + 0.2 R` (200 to 900 m) at an angle that advances by 0.125 turn per
destination; its altitude = the sheet elevation (if covered, never lower than the point's own `y`) + 5 m. Fly to it at
70 m/s, dest velocity = own velocity, look at the point; when within 30 m horizontally choose the next. [decomp]

### 6.2 `AIActionHeliExit`

State `seek up -> seek car -> fly out`.

- Start: seek point = own position snapped to the nearest traffic road (xz) with `y += 25` (`Exit_Height`).
- *Seek up*: when 15 m above the player: seek = `player_pos + player_forward * 85` with `y = player.y + 25`; mode seek car.
- *Seek car*: when within 5 m of that point: `right` = the player's right vector; `side = (dot(right, heli velocity) >= 0)
  ? -9 : 3`; direction = `player_forward + right * side`; seek = `player_pos - 200 * direction`, `y = player.y + 30`; mode fly out.
- *Fly out*: keep the seek point.
- Always: look at the seek point, drive speed 100 m/s, dest velocity = own velocity.
- *Finished* (then the goal un-spawns the helicopter): in fly-out, more than 25 m above the player, behind it
  (`dot(heli - player, player_forward) < 0`) and more than 150 m away. [decomp]

## 7. Audio and visual hooks

- **Rotor sound.** `SoundHeli` opens a sound connection with the vehicle's attribute collection; the sound system
  (`SFXCTL_Helicopter`, `SFXObj_Helicopter`) takes the helicopter's position, velocity and forward vector, finds the closest
  player car, and publishes to the AEMS helicopter FX: distance (clamped to 100, negative when the sound is behind the
  listener by the mixer azimuth), speed `= min(1, forward_speed / 500)` (set to 0.5 when under 0.25) scaled to 0..1023,
  azimuth, volume and pitch offset from the dynamic mixer outputs, and a "rotation" value = the angle between the heli's
  heading and the bearing to the player (`acos(dot) >> 6`). Volume goes to 0 while the helicopter is not simulated.
- **Speech.** Quadrant search announcement at the forced search spawn; "self strategy" at the start of a skid-hit; "intent
  to ram" at the strike (all through `SoundAI`'s helicopter speech object).
- **Rendering.** `DrawHeli` opens a render connection (`HeliRenderConn`, render usage "AI heli") with the shadow scale and the
  dust-storm intensity (stored by the AI, never changed). The model is hidden from the view whose camera is anchored to it.
  A *rotor wash* effect (`effects/heliwash`) runs once a second: intensity `1 - ramp(altitude, 5, 30)` (altitude above the
  ground at the heli's xz), placed on the ground at the heli's xz with the ground normal, paused above 30 m or off valid ground.
- There is no searchlight, no gun, no muzzle flash and no spike drop in the code (`SPIKE_DROP` and the "heli spike strip"
  counter exist only as statistics). [decomp]

## 8. Constants not in AttribSys

| Name | Value |
|---|---|
| Think period | 0.125 s |
| Lead times | 0.45 s (work position), 0.65 s (look-ahead), 0.2 s (camera avoidance) |
| Camera avoid sphere | 7 m at 4 m ahead of the camera; kick factor 1.2 |
| Chopper accel | `min(80, 0.6 v + 30)`, ratio 2, braking `1.55`, `0.4` speed cut |
| Pitch/roll limits | +-0.1 turn; yaw rate 8, clamp 1.3 rad/s; angular smoothing 1/8 |
| Height rules | climb x5 below 7 m, descend x3 above -5 m |
| Visual sphere | 75 m; grace 150 m / 20 m |
| Spawn | 250 m behind the target on the road, +20 m up |
| Follow heights | +6 / +12 m above the player, lead 30 to 45 m |
| Skid-hit | window 5 to 35 m, 13 m height, 8 s timer, 5 s cooldown, side 6 m, strike 4 m / 1.9 m |
| Search | 70 m/s, 30 m arrival, 0.125 turn per step, radius 0.2 to 0.9 of 1000 m, +5 m |
| Exit | height 25 m, speed 100 m/s, finish at 25 m up and 150 m away |

## 9. How to check it

1. Spawn: at heat 5 hold a pursuit for 35 s (the support delay): a helicopter should arrive about 250 m behind you, 20 m up.
2. Follow: it keeps about 30 to 45 m ahead of your car at +6 m (+12 m when you are more than 55 m away) and matches your velocity.
3. Fuel: at heat 5 it leaves after 90 s (at heat 4 after 60 s): climbs 25 m, flies out 85 m ahead, then 200 m behind you.
4. Skid hit: at heat 6 (or heat 5) the helicopter periodically dives beside the car, 6 m to the side and about 2 m above, and
   rams; a 20 000 kg body makes a big hit. Check the 8 s window and the 5 s cooldown.
5. Search: break line of sight; the helicopter orbits 200 to 900 m from your last known spot in 45 degree steps at 70 m/s.
6. Height: fly over the tallest building; the helicopter should rise over it with a smooth slide, never through it, and
   through the camera never.

## Open questions

- **Q1** `UMath::AddScale`/`ScaleAdd` argument order is inferred from the shape of the formulas (the AddScale reading makes
  `SetDestinationVelocity` the identity and the angular smoothing a 1:7 low-pass, which is plausible).
- **Q2** `SimpleChopper::mLastAccelVector` is scaled by 4 each step but never assigned, so it stays zero: the acceleration
  "low-pass" is just a gain of 0.2. This might be a decompilation artefact; the visual effect (body lean) would differ
  slightly. Compare lean angles in the running game.
- **Q3** `AIGoalHeliRoadBlock` is not defined; `roadblockhelichance` is 0 everywhere. Not implemented in the original.
- **Q4** The mixer outputs for the rotor sound (`DMX_*`) come from the dynamic mixer map for the helicopter, not specified here.
- **Q5** How `StartPathToPlayerCar` is used (the result is not read by any mode).

## Rust implementation notes

- Implement `SimpleChopper` as a velocity-tracking point mass: it is only 30 lines of arithmetic, easy to unit-test with a
  desired-velocity sequence (step response time constant 0.5 s, clamp at 80 m/s^2).
- Implement the sheet as an array of (section number -> triangle list) with the exact point-in-triangle and plane-elevation
  rule of the format doc; the primary/secondary/third coordinates are three caches of one lookup.
- The helicopter is a regular vehicle entity with `NO_WORLD_COLLISIONS` (skip world queries) and ground and object
  collisions; give it the object-collision weight of 20 000 kg.
- Keep the skid-hit state machine data-driven by `AirSupportOptions` containing `SKID_HIT`.

## 10. Tuning file and the air-support feed (E9)

The helicopter's numbers come from `NFSMWHelicopterSettings.ini`, next to the config file (`crates/nfsmw/src/settings/helicopter.rs`,
read with `libs/ini-tuning`). Keys are the mod-style sections `[Helicopter:<Name>]`: `default`, `heat01` to `heat10` and `race01`
to `race10`. A `default` line sets each level without its own line; with neither, the vanilla value of section 2.3 and section 8
applies. Values are clamped to a range; a malformed value is vanilla; a missing file is vanilla throughout. The file is read
when a track opens.

| Key | Applied | Vanilla (heat 1 to 10) |
|---|---|---|
| `Helicopter:FuelTime` | yes | 10, 10, 10, 60, 90, 180, 10, 75, 120, 400 s (race: 10, 20, 60, 80, 120, 120, 220, 75, 120, 120) |
| `Helicopter:Respawning` (`TimeBetweenHeliActive`) | yes | 0, 0, 120, 180, 180, 180, 180, 120, 60, 5 s (race: 120 s) |
| `Helicopter:LineOfSight` (`heliLOSdistance`) | yes | 251 for heat 1 to 7, 301 for heat 8 to 10 (race: the same) |
| `Helicopter:SpawnDistance` | yes | 250 m behind the player on the road, +20 m up |
| `Helicopter:DespawnDistance` | yes | 600 m (not in the spec; the police code's value, not verified) |
| `Helicopter:SpeedCap` | yes | 100 m/s (`MAX_SPEED_MPS`) |
| `Helicopter:FlySpeed` | yes | 100 m/s, the skid hit and the exit |
| `Helicopter:SearchSpeed` | yes | 70 m/s, the search |
| `Helicopter:AttackDuration` | yes | 8 s, the skid-hit timer |
| `Helicopter:AllAroundSight` | yes | 75 m, the visual sphere |
| `Helicopter:FrontSightOnly` | yes (switch) | on: beyond the sphere only the front half space counts; off: the whole line-of-sight range |
| `Helicopter:HidingSpots` | yes (switch) | on: a hiding spot hides the player from the helicopter; off: the spots are ignored |
| `Helicopter:HeliSheet` | yes (switch) | on: the track's height sheet floors the helicopter; off: a flat floor |
| `Helicopter:IgnoreHeliSheetDistance` | read, not applied | 0 m; the spec's `CheckHeliSheet` has no distance gate to put it in yet |
| `Leading`, `ClosingSpeed`, `SearchRadius`, `Turning`, `Acceleration`, `Brake` | not read | the spec's constants stay in code: lead 0.45 s and 30/45 m, closing 3 m/s and the 1.55 divisor, search 1000 m, yaw and accel clamps, the 80 m/s^2 limit |

The race table (`raceNN`) is read and used by `Levels::at(level, racing)`, but no race code calls the police yet, so the chase
always reads the heat table (`RACING` in `police/heli.rs`).

**Hidden zones (section 3, now implemented).** A HIDDEN track zone hides the player from cars unless its first data word is the
name hash of `"Heli"`, and from helicopters unless it is the hash of `"Car"`. The installed zones all have data word 0, so they
hide from both, as before. Hiding from helicopters uses the zone's height test. A TUNNEL zone (kind 3) stops the helicopter
seeing the player, tested at the player's place (the spec tests the game camera; the camera is not in the police state).

**PursuitFeed for the air-support lines (`audio/interactive/state.rs`).** The feed carries `heli_present` (a helicopter is in the
chase, flying or leaving), `rain_intensity` (the scene's eased rain, 0 to 1; the scene fills it), `roadblock_active` (a roadblock of
this chase stands), `player_hit` (traffic was hit within its hit memory) and `cops_destroyed`. Not added: a been-hit flag (no
state records the player being hit by a cop or the helicopter) and an escaped-cars count (no memory of cars that got away). The
`chase/` cues that use them are not wired yet.

**Reversal status.** The per-level values are section 2.3's, tagged [verified] there. Re-checking them in `speed.exe` (Ghidra) for
`CanSeeTarget`, `BeginHelicopter`'s fuel, `MAX_SPEED_MPS` and the hidden-zone data words was not done in this pass; see
[provenance/ai-helicopter-tuning.md](../provenance/ai-helicopter-tuning.md).

## Rewrite status

Implemented in `libs/blackbox-pursuit/src/helicopter/` (sight, flight model, controller, behaviours) and wired in
`crates/nfsmw/src/scenes/world/ai/police.rs`: it joins from heat 4 after a 10 s wait, takes the per-level fuel and
respawn tables of section 2.3, follows, skid-hits where the level has `SKID_HIT`, searches, exits, and shows on the
minimap. Its `COPHELI` model is loaded with the cops and drawn at its pose (render frame rebuilt from the physics nose
and up). Sight uses the world's ray cast (the line from the helicopter to the player, lifted 1 m, must be clear; the
`clear` closure in `ai/police.rs`). The heli sheet's chunk parser and its lookup are in `libs/blackbox-roads/src/heli_sheet.rs`
(section 1 of `docs/formats/pursuit-data.md`). The sheet is read from `STREAML2RA.BUN` when the track opens
(`crates/nfsmw-data/src/world/heli_sheet.rs`, a header walk: the 532 MB file is not read whole) and is the helicopter's
floor (`SheetFloor` in `ai/police.rs`; a flat floor when the sheet is missing). On the install the walk gives the
documented 51 504 triangles (`tests/real_install/heli_sheet.rs`). Not yet seen in a running game. Open: that wiring, rotor sound, and the spotlight/rotor
wash effects. The shipped numbers are the spec's; the controller is the spec's, not the decompilation's code.

## Verified in `speed.exe` (Ghidra, PC v1.3)

The binary has no symbols; these were found from the strings and their references, and the names are ours.

| Claim | Where | Evidence |
|---|---|---|
| Spawn 250 m behind the target | `SpawnPursuitHelicopter` at `0x004269a0` | pushes `0x437A0000` = 250.0f into the nav advance |
| Height +20 m | same function | adds the float at `0x00890560` = 20.0 to the spawn height |
| Nav points backward | same function | multiplies the target direction by `0x00890DA8` = -1.0 |
| One helicopter at a time, a free one is reused | `0x00426610` (an inactive `copheli` in the vehicle pool) | filters the pool by the `copheli` type name, returns the first not spawned |
| Cop manager asks for a helicopter | `0x0043E8D0` (the cop manager's update) | compares the cop type's hash with `copheli` (`0x008915DC`) and calls `0x004269A0` |

Pass 2 (below, "Constants in speed.exe") checked the constants against the float pool. The function holding
`DoDriving` and the helicopter's `Update` is still not located, so the rules of sections 4, 5 and 6 are not yet tied to
code addresses.

## Constants in speed.exe (pass 2)

Source: the float pool of `speed.exe` at `0x00890500` to `0x00890E8F` (PC v1.3), decoded as IEEE singles. The heli
constants sit in one cluster at `0x00890828` to `0x00890868` (values below). Use sites are not found, so the value is
**[decomp]** and the link to the helicopter is **[unconfirmed]** (cluster only). Method and addresses are in
[provenance/ai-helicopter-tuning.md](../provenance/ai-helicopter-tuning.md).

| Constant | Value | Pool address | Meaning | Port |
|---|---|---|---|---|
| Work lead | 0.45 s | 0x890840 | `work` and `dest` lead (section 4) | `control.rs` `WORK_LEAD` |
| Look-ahead lead | 0.65 s | 0x890844 | the `ahead` point of `CheckHeliSheet` | not in port |
| Sheet steer gains | 0.3, 0.4 | 0x890848, 0x89084C (also 0x8905D0) | sliding around a roof and a wall | not in port |
| Camera avoid | lead 0.2 s, sphere 7 m, kick 1.2 | 0x8905CC, 0x8905F0 (7), 0x890860 (1.2) | `AvoidCamera` | not in port |
| Response ratio | 2 | 0x890628 | desired accel = 2 x velocity error | `flight.rs` |
| Accel limit | `min(80, 0.6 v + 30)` | 0x89062C (0.6), 0x890620 (80), 0x890820 (30) | section 5 | `flight.rs` |
| Closing / brake | closing > 3; speed cut 0.4; brake at 15 m | 0x8905DC, 0x890690 (3); 0x8905D0 (0.4); 0x890668 (15) | section 4 | `control.rs` |
| Braking divisor | 1.55 | not in 0x890500-0x890E8F | section 4 | `control.rs` (no PC source) |
| Vertical | climb x5 below 7 m; descend x3 above -5 m; add 2 x target vy | 0x8905E0 (5), 0x8905F0 (7), 0x8905F4 (3), 0x890DB8 (-5) | section 4 | `control.rs` `vertical` |
| Follow lead | 30 + 0.4 v, cap 45 | 0x89052C (30), 0x89055C / 0x890658 (45) | section 6.1 | `behaviour.rs` `follow` |
| Follow speed | 50 / 30 / 20 m/s at 300 / 15 / 3 m | 0x890558 (50), 0x890544 (300), 0x890678 (10), 0x89067C (20) | section 6.1 | `behaviour.rs` `follow` |
| Follow heights | +6, +12 m; 2 m (skid); 55 m split | 0x8905A8 (6), 0x890590 (12), 0x890628 (2), 0x890584 (55) | section 6.1 | `behaviour.rs` `follow` |
| Skid | height 13 m; approach 0.23 s; window 5 m; timer 8 s; cooldown -5 s; side 6 m | 0x890648, 0x89064C, 0x890654, 0x89059C (8), 0x8905A8 (6) | section 6.1 | `behaviour.rs` |
| Strike | 4 m; 0.4 m height; speed 100 m/s | 0x89065C (4), 0x8905D0 (0.4), 0x890530 / 0x890664 (100) | section 6.1 | `behaviour.rs` |
| Search | radius 1000 m; arrival 30 m; speed 70 m/s; +5 m | 0x890538 (1000), 0x890820 (30), 0x89057C (70) | section 6.1 | `behaviour.rs` |
| Exit | height 25 m; seek up at 15 m; 200 m dive | 0x890DC0 (25), 0x890DAC (15), 0x8908BC (200) | section 6.2 | `behaviour.rs` `Exit` |
| Sight | grace 150 m, 20 m | 0x89076C (150), 0x890560 (20, also spawn) | section 3 | `sight.rs` |
| Spawn | 250 m back, +20 m, backward (-1) | 0x890560 (20, verified in code), 0x890DA8 (-1, verified), 0x890854 (250) | section 2.2 | `mod.rs` `spawn_distance` |
| Think period | 0.125 s | 0x8905C8, 0x890830 | section 1 | `mod.rs` `THINK_PERIOD` |
| Yaw gain / lean | 8; lean 9/80 and 12/80; clamp 0.1 | 0x8905E4 (8), 0x890598 (9), 0x890590 (12), 0x8905C4 (0.1) | section 5 | `flight.rs` |
| Smoothing | 1/8 new, 7/8 old | 0x8905F0 (7), 0x8908C8 (0.125) | section 5 | `flight.rs` |

**Not found** in the pool read (0x00890500 to 0x00890E8F): 1.55 (braking), 0.035 (pitch gain), 0.092 (strike lead),
1.3 (yaw clamp), 1.8 and 4.8 (skid lift), 1.9 (strike across), 0.7, 1.14, 35 (skid window), 75 (visual sphere), 85 (exit
seek), 600 (despawn). They may be immediate operands in code. Their values are the spec's, not confirmed in `speed.exe`.

**Port gaps** from this comparison: `control.rs` has the floor lift but not the `CheckHeliSheet` steering (0.3, 0.4, the
`ahead` point at 0.65 s), and it has no `AvoidCamera`. The port has no camera, so the camera avoid is unported by design.

## Next leads

1. Find `AIVehicleHelicopter::Update` and `OnDriving` by the vtable of the heli object. The pool helper 0x426610 and the
   vehicle slot 0x80 are the anchors. Needs xref tools (get_xrefs_to), which were not callable in pass 2.
2. Find the functions whose `refs` contain 0x890828 to 0x890868: the cluster's users are the controller.
3. Find who passes `"AIVehicleHelicopter"` (0x893070) to the class factory `0x5CC240`: that caller builds the heli AI.
4. Identify the class tags `0x4037E0`, `0x4038C0`, `0x4038E0` used by `0x5D59F0` (casts) to tell heli from cop.
5. Search immediates (not the pool) for 1.55, 0.035, 0.092, 1.3, 1.8, 35, 75, 85, 600 once a byte-pattern search is
   available (none in this session).
6. The words at 0x00890CE0 are code pointers into 0x0040xxxx; not yet identified (possibly a dispatch table).
