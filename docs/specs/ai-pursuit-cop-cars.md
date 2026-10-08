# Cop cars: sight, the "jerk" rule, damage, collision weight and per-car tuning

What is different about a police car compared with a racer or a traffic car, apart from its goals (which are in
[ai-pursuit-tactics.md](ai-pursuit-tactics.md)): how it sees the target, the "jerk" boost, how it is destroyed, how it
behaves in a crash with the player, which physics classes it uses and the per-car numbers. The helicopter is in
[ai-helicopter.md](ai-helicopter.md).

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled; GameCube build), under
  `src/Speed/Indep/Src`: `AI/Common/AIVehicleCopCar.cpp`, `AIVehiclePursuit.cpp`, `AIVehicle.cpp` (simple physics,
  goal switching, perpetrator hidden-zone code), `AIPursuit.cpp` (`UpdateJerk`), `Physics/Behaviors/{DamageCopCar,
  DamageVehicle,DamageRacer,RBCop,RBVehicle}.cpp`, `Physics/Behaviors/SpikeStrip.cpp` (to see which cars are
  spikeable). Read for understanding; no code copied.
- **Data inputs:** `pursuitlevels` (`frontLOSdistance`, `rearLOSdistance`), `aivehicle`, `damagespecs`, `collisionreactions`,
  `pvehicle`; tables in [../formats/pursuit-data.md](../formats/pursuit-data.md).

Tags: [decomp] decompiled sources; [verified] read from the install's AttribSys vaults. Units: metres, seconds.

## 1. The "jerk" pursuit

`AIPursuit::UpdateJerk` runs every 0.25 s on the pursuit task. With `a = 0.1 * dt`:

```
lag_pos      = lerp(lag_pos, target.pos, a)            # starts at the target's position when the pursuit is created
lag_distance = lerp(lag_distance, |target.pos - lag_pos|, a)       # starts at 1000
lag_speed    = lerp(lag_speed, target.speed, a / 2)                # starts at 0
factor       = lag_distance > 0.01 ? lag_speed * 10 / lag_distance : 0
jerk = factor >= 3.0 ? true : factor <= 1.75 ? false : unchanged
```

In words: a target that moves fast while its smoothed position hardly moves (circling) is a "jerk". The effects, all read
from other code: cop top speed times 1.2 and acceleration times 1.5 in the chase and ram actions, cop `MAXIMUM_AI_SPEED`
times 1.1, the seek-ahead look-ahead drops from 7.8 s to 3.12 s, the collapse speed threshold becomes 125 km/h and the
pull-over aggression is forced to 1.0. [decomp]

## 2. Seeing the target (`AIVehicleCopCar::CanSeeTarget`)

`AIVehiclePursuit::Update` tests sight every 0.25 s while the cop is in a pursuit with a valid target (it accumulates
`dT` in a timer and tests when the timer reaches 0.25, subtracting 0.25):

```
(front, rear) = (frontLOSdistance, rearLOSdistance) of the target's current pursuit level; (150, 50) if it has none
out_of_sight  = dist >= front  or  (dist >= rear and dot(cop.forward, unit(target - cop)) <= -0.3)
out_of_sight |= not pursuit.PursuitMeterCanShowBusted()      # true while the cool-down gauge animates
if not out_of_sight: out_of_sight = world ray from (cop + 0.5 m up) to (target + 0.5 m up) hits geometry
hidden = target is "hidden from cars" (it sits in a hidden zone, see the pursuit spec)
if hidden and this cop's latch is set: return not visible
latch = false
... compute out_of_sight as above ...
if out_of_sight: (if hidden: latch = true) ; return not visible
return visible
```

A seen target sets `time_since_target_seen = -0.25`, afterwards it grows by `dT`. With no pursuit or no valid target the
counter and the test timer stay at 0.25. The pursuit's "perp in sight" is true while the smallest value over all active,
undestroyed cops is under 7 s. A spawned cop starts at 99 s ("long ago"), a freshly spawned pursuit car has its counter
zeroed by the spawner. Distances **[verified]**: heat levels 1 to 7: 151 m front, 151 m rear; heat 8 to 10: 201 / 101;
race levels 1 to 7: 151 / 51; race levels 8 to 10: 201 / 101; `default` 151 / 51. [decomp]

## 3. Spotting by patrol cops (`WatchForPerps`)

A cop that is not in a pursuit looks, each think, at every local player vehicle and, if the cop manager allows cops to
pursue racers, at non-human, non-remote racers. For each candidate it makes a temporary target and applies section 2. If
the candidate is seen it must also pass the "worth a chase" test: it is already being chased, **or** a 911 call timer is
active, **or** it hit a traffic car within the last 1 s, **or** its heat is above 3, **or** its speed is at least 65 mph;
otherwise it is ignored. On success the cop keeps the target, which makes `PursuitRequest()` non-null and the cop manager
starts or joins a pursuit at its next run ([ai-pursuit-cops.md](ai-pursuit-cops.md)). [decomp]

`IsTetheredToTarget` exists for cops (true when the object is not the target, the cop is in a pursuit, is doing at least
50 mph, is within 50 m of the target and the target is behind it: `dot(dir_to_target, forward) < -0.2`) but no caller was
found in the sources read. [decomp]

## 4. Which physics classes a cop uses

From `pvehicle` **[verified]**: every cop is a `CAR` with `MASS` 2200 kg and the behaviours: rigid body `RBCop`
(identical to `RBVehicle`; its two override methods only call the base), engine `EngineRacer` with engine, transmission,
brakes, chassis, tires, induction and (for the sport family) nitrous from the `cops` collections of those classes,
suspension **`SuspensionSimple`** (racers use `SuspensionRacer`, traffic `SuspensionTraffic`), input `PInput`, damage
**`DamageCopCar`** (racers: `DamageRacer`, traffic: `DamageVehicle`), draw `DrawCopCar`, sound `SoundCop`, event
sequencer `CopCar`. The helicopter uses `RBVehicle`, `SimpleChopper`, `DamageHeli`, `DrawHeli`, `SoundHeli`.
`SuspensionSimple` is the racer corner model without the player assists (see
[vehicle-suspension-tires.md](vehicle-suspension-tires.md)). Only `DamageRacer` implements the spikeable interface, so
**only racer-class cars (the player and AI racers) can have their tires punctured**; cops, traffic and the helicopter are
immune to spike strips. [decomp][verified]

## 5. Simple physics (off-world cops)

`IsOffWorld` means there is no streamed world under the car. Then each think `AIVehicle::UpdateSimplePhysics` moves the
car kinematically and the real physics is bypassed (physics mode "emulated"):

```
dir     = unit(drive_target + (0,1,0) - position)
speed   = |velocity|
if speed > drive_speed: speed = max(drive_speed, speed - 30 dT)
else:                   speed = min(drive_speed, speed + accel_table(speed) * dT * lerp(0.5, 1, skill))
speed   = clamp(speed, 0, top_speed * lerp(0.75, 1, skill))         # skill = 1 for cops
if in reverse gear: speed *= -0.5
position += dir * speed * dT
height   = world height under the new position (or the nav cookie centre + 1 when no world); + ride height + half body height
orientation: forward = dir, up = ground normal when world exists and normal.y >= 0.707, else (0,1,0)
velocity = dir * speed, angular velocity = 0
```

The cop manager switches this on for a cop it places where the world is not loaded. When the cop is again on the world it
is put on the ground 1.5 m above its position along its heading, physics resumes with its speed and angular velocity
kept, and it is invulnerable for 1.0 s ("physics switch"). [decomp]

## 6. Damage and destruction

Cop damage is `DamageVehicle::OnCollision` plus two cop rules.

```
on a collision with closing speed >= 1 m/s:
    force = impulse * mass          (x2 when the other object is a vehicle or prop, not the ground/world)
    OnImpact(arm, normal, force, speed, surface, other)

DamageCopCar::OnImpact:   if other is a smackable and not (immobile or mass > 2000): ignore
DamageVehicle::OnImpact:
    if SUPPRESS_DIST > 0 and the cop is farther than SUPPRESS_DIST from the camera: ignore      # 35 m (copcross 25)
    if other is a smackable with NO_CAR_EFFECT: ignore
    ratio  = force / (FORCE * 1000)                       # FORCE = 20 for all cops
    zone   = from the impact arm in the cop's frame (see below)
    points = ratio * zone.HitPointScale / HIT_POINTS
    if points > HP_THRESHOLD / HIT_POINTS:  total += points ; if total > 1: raise "vehicle destroyed"
    visual damage: levels from ratio * zone.VisualScale (cars only; not part of the AI)
```

Zone: with `arm` the contact point relative to the centre in the cop's frame and `dim` its half extents (`k = 0.5`):
front if `arm.z > k dim.z`, rear if `arm.z < -k dim.z`, each becoming a corner (left/right front/rear) when
`|arm.x| > k dim.x` and the normal's local `x` has magnitude over 0.8 against the corner side; otherwise top
(`arm.y > k dim.y` and the normal points down over 0.8), bottom (opposite), else left/right by the sign of `arm.x`.

Per-car numbers **[verified]**:

| `damagespecs` | `HIT_POINTS` | `HP_THRESHOLD` | `SUPPRESS_DIST` | used by |
|---|---|---|---|---|
| `cops` | 5 | 0.75 | 35 | `copmidsize` (and the base of the others) |
| `copghost` | 6 | 0.75 | 35 | `copghost` |
| `copgto` | 7 | 1.5 | 35 | `copgto` |
| `copgtoghost` | 8 | 1.5 | 35 | `copgtoghost` |
| `copc6` | 10 | 1.5 | 35 | `copsport` |
| `copsuv` | 10 | 2 | 35 | `copsuv`, `copsuvpatrol`, `copsuvl` |
| `copcross` | 400 | 10 | 25 | `copcross` |

(`HitPointScale` per zone for the cop sets: sides left/right 1.0, the four corners 1.0, front 0.5 (copcross 1.0), rear
0.5 (copcross 1.0), top 0.5, bottom 0.) `SHOCK_TIME` 3 s, `SHOCK_FORCE` 40 for every cop set. A hit with a ratio under the
threshold does nothing; the cars take roughly 5 to 10 solid hits. The helicopter's set has `HIT_POINTS` 0, so it cannot
be damaged to destruction. [decomp][verified]

**Flipped over.** `DamageCopCar` checks every 0.1 s while the cop is not destroyed, not animating and no cut-scene exists:
if the body is simulated and not all wheels touch the ground, and `dot(up, ground_normal) < 0.5` and `up.y < 0.5`, a
"flipped" timer accumulates `dT`, else it resets. At 6.0 s the cop is destroyed. If it is flipped and the last cause of
its motion (the causality chain) is the player's vehicle, it is destroyed at once. [decomp]

**Shock.** `SetShockForce(f)`: `scale = f / mass / SHOCK_FORCE`; if `SHOCK_TIME > 0` and `scale > 0.2` the shock timer becomes
`min(max(timer, scale), 1)` and decays by `dT / SHOCK_TIME` per second; "in shock" is timer > 0. Used by the wreck
coast-brake rule. [decomp]

**What destroyed means for the pursuit.** `IsDestroyed` starts the `AIActionTooDamaged` retirement, the pursuit counts a
kill (rep points `RepPointsForDestroying[level]`, cost to state; see the heat spec), and the manager removes the car
once it can respawn and has been off-screen long enough ([ai-pursuit-cops.md](ai-pursuit-cops.md) section 5). The
pursuit-breaker zones destroy cops directly ([ai-pursuit-roadblocks.md](ai-pursuit-roadblocks.md) section 7).

## 7. Collision weight against the player

`aivehicle.PlayerCollisions` is a list of `{goal hash, collisionreactions reference}`; `PlayerCollisionsDefault` is the
fallback. Whenever a cop (driver classes traffic through racer) changes goal, the reference whose goal hash equals the new
goal name is installed, else the default. For cops **[verified]**:

| aivehicle | default set | `AIGoalStaticRoadBlock` | `AIGoalPit` |
|---|---|---|---|
| `copmidsize`, `cops`, `copcompact`, `copcross` | `cops` (`copcross`: `copcross`) | `roadblock` | `coppit` (not `copcompact`) |
| `copghost` | `copghost` | `roadblockcopghost` | `coppit` |
| `copsport` | `copsporthench` | `roadblockcopsporthench` | `coppit` |
| `copsporthench` | `copsporthench` | `roadblockcopsporthench` | `coppit` |
| `copsportghost` | `copsportghost` | `roadblockcopsportghost` | `coppit` |
| `copgto` | `copgto` | `roadblockgto` | `coppit` |
| `copgtoghost` | `copgtoghost` | `roadblockgtoghost` | `coppit` |
| `copsuvpatrol` | `copsuvpatrol` | `roadblocksuvheavy` | `coppit` |
| `copsuv` | `suvs` | `roadblocksuvheavy` | none |
| `copsuvl` | `suvlight` | an entry with an unknown goal hash (not a known goal name) -> `roadblocksuvlight` | none |

The reaction rule (four records chosen by contact side; `Elasticity` added to the body elasticity, `MassScale`
multiplies the cop's mass and inertia, `RollHeight` raises and `WeightBias` advances the collision centre of gravity) is
in [vehicle-rigid-body.md](vehicle-rigid-body.md) (`ModifyCollision` for vehicles); it applies only when the other body is
a racer or human driver. Example records, as `(Elasticity, RollHeight, WeightBias, MassScale)`, from the `collisionreactions`
vault: `cops`: rear (0, 0.2, 1.0, 1.2), rear-side (0.2, 0.2, 0.8, 0.2), front (0.25, 0.2, 0.2, 0.6), front-side
(0.1, 0.2, 0.2, 0.6). `coppit`: rear (0, 0.2, 1.0, 2.0), rear-side (0.2, 0.2, 0.8, 0), front (0.4, 0.2, 1.0, 4.0),
front-side (0.4, 0.2, 1.0, 4.0). `roadblock`: rear (0, 0, 0, 2.0), rear-side (0.2, 0.25, 0.75, 0.45), front (0, 0, 0.5, 2.6),
front-side (0, 0, 0.5, 2.6). `StunSpeed` and `StunTime` are 0 in the cop sets. Reading: a cop in a normal chase hits
lighter than its mass in the front (0.6) so it bounces off; a pit cop hits with four times the mass, a roadblock car with
2 to 2.6 times. [decomp][verified]

`copsuvl`'s entry uses a goal hash that does not equal the hash of any goal name in the sources (the other cars use
`AIGoalStaticRoadBlock`'s hash), so as shipped the light SUV in a roadblock keeps its default set `suvlight`.

## 8. Per-car AI numbers

`aivehicle`, **[verified]** (speeds km/h):

| Collection | `MAXIMUM_AI_SPEED` | `TopSpeedMultiplier` | `AccelerationMultiplier` | `TETHER_WEIGHT` | `RepPointsForDestroying` (heat 1..10) |
|---|---|---|---|---|---|
| `cops` (base) | 200 | 1.0 | 1.0 | 0 | 1..10 |
| `copmidsize` | 280 | 1.6 | 1.05 | 0 | 250 |
| `copghost` | 310 | 1.15 | 1.09 | 0 | 500 |
| `copgto` | 325 | 1.2 | 1.1 | 0 | 2500 |
| `copgtoghost` | 350 | 1.25 | 1.15 | 0 | 5000 |
| `copsport` | 400 | 2.0 | 1.5 | 0 | 20000 (100000 at heat 10) |
| `copsporthench` | 400 | 1.3 | 1.2 | 0 | 20000 |
| `copsportghost` | 400 | 1.5 | 1.25 | 0 | 25000 |
| `copsuvpatrol` | 250 | 0.9 | 1.05 | 0 | 25000 |
| `copsuv` | 50 | 0.5 | 1.0 | 0 | 15000 |
| `copsuvl` | 50 | 0.5 | 1.0 | 0 | 10000 |
| `copcross` | 400 | 2.0 | 1.5 | 0 | 100000 |
| `copcompact` | 200 | 1.0 | 1.0 | 100 | 1..10 |

(The collection `copcompact` has no cop car in the shipped `pvehicle` list; it is the only one with a tether weight.) The
`DetachmentID` field is the speech battalion id and is not used by the AI. [verified]

## Open questions

- **Q1** `SuspensionSimple` is only skimmed in the vehicle specs; the cop cars' handling depends on it. Measure it.
- **Q2** The `copsuvl` goal hash (0x3CD90F0E in the lookup2 hash space) is not any goal name tried; it may name a goal
  that exists only in the PC build.
- **Q3** `AIActionTooDamaged` needs `IsDestroyed()`; the exact moment the flag turns on (event `EVehicleDestroyed` handler
  is not in the sources read) is unconfirmed.
- **Q4** The visual damage tiers (zone damage levels 0 to 6) are not specified here.

## Rust implementation notes

- A `CopCar` is a normal vehicle with its own `Damage` strategy (`HitPoints`, `Threshold`, `FlippedTimer`) and an
  `AiVehicleModel` row; no separate physics class is needed beyond `SuspensionSimple` and the collision-reaction hook.
- Put the `PlayerCollisions` lookup in `set_goal`; make it return the `collisionreactions` handle the body then reads.
- Keep `time_since_target_seen` as the single sight signal; the pursuit's "perp in sight" is the minimum over cops.
