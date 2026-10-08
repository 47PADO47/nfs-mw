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
