# Traffic and the world: lights, intersections, horns, scripted traffic

What surrounds the traffic cars in the original: whether traffic lights, stop signs and intersections affect them
(they do not), what the horn, engine and drive-by sounds of traffic do, and how drag races place scripted traffic.
The cars and their driving are in [ai-traffic.md](ai-traffic.md), spawning in
[ai-traffic-spawning.md](ai-traffic-spawning.md). Provenance:
[provenance/ai-traffic.md](../provenance/ai-traffic.md). For the tag meanings, see
[evidence tags](../README.md#evidence-tags).

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled; GameCube
  build), under `src/Speed/Indep/Src`: `EAXSound/CARSFX/CARSFX_TrafficFX.{hpp,cpp}`, `EAXSound/EAXSOund.hpp`,
  `EAXSound/EAXTrafficCar.cpp`, `EAXSound/States/Registration.cpp`, `EAXSound/sfxctl/SFXCTL_Wheel.cpp`,
  `Physics/Behaviors/SoundCar.cpp`, `AI/Actions/AIActionTraffic.cpp` (the unused flags), `Gameplay/*` headers,
  `Generated/AttribSys/Classes/{gameplay,smackable_hash,pvehicle}.h`; a search of the whole source tree for traffic
  light, stop sign, signal, intersection and horn code. Read for understanding; no code copied.
- **Data inputs:** AttribSys classes `smackable` (`trafficlight`, `trafficcablelight`, `traffic_cable`),
  `light_flares_cg` (`traffic_light`, `traffic_light_red`), `gameplay` (drag-race traffic), `pvehicle`
  (`HornType`, `TrafficEngType`, `WooshType`); a string search of the install's world and global files. Numbers
  marked **[verified]** come from those.

## 1. Traffic lights and stop signs

**They do not exist as game logic in this release.** Evidence:

- Searching the whole decompiled source tree for traffic-light, signal, stop-sign and intersection-control code finds
  nothing but two unused booleans in the traffic driving action (`stop_sign`, `clear_intersection`; both set to
  false and never read) and one attribute-hash constant named after the smackable prop `trafficlight`
  **[decomp]**. No class has a signal state, a timer or a phase.
- The chunk the decomp calls `TrafficIntersections` does not occur in the install ([formats/world.md](../formats/world.md)),
  and the road network has no intersection records at all (the intersection count of its header is 0; junctions are
  groups of nodes joined by decision segments, [ai-road-network.md](ai-road-network.md)) **[verified]**.
- No AttribSys class has a signal or light-phase field. The 57 classes include no traffic-control class; the names
  that contain "traffic" are the traffic pattern, the traffic car entries, three props and two flare entries
  (below) **[verified]**.
- The traffic driver never slows for a junction or a signal; its only reason to stop is a car ahead or a dead end
  ([ai-traffic.md §3.3, §5](ai-traffic.md#33-speed)).

**What is in the install is decoration:**

- Scenery models (names found as strings in the world stream, `STREAML2RA.BUN`, and the map file) **[verified]**:
  `XO_TrafficLightA`, `B`, `C`, `D` (several tile variants each), `XO_TrafficLightCableB`, `XS_StopSignB`,
  `XO_Tollbooth_StopSign`, `XS_TrafficSignalArrow_*`, `XS_TrafficSignalCross_*`, `SGN_Signal_arrow`,
  `SGN_Signal_Cross`, `SGN_Trafficlights_Pole`, `XO_TrafficConeA`, `XO_TrackBarrierTraffic`, plus many street-light
  models. They are ordinary scenery: placed once, drawn like any other prop, never animated or toggled by game
  state. (Whether the lamp of a traffic-light model changes colour in the draw code is not known; no data drives it.)
- Light flares: `light_flares_cg/traffic_light` (amber, colour `(0.43, 0.29, 0, 0.58)`) and
  `traffic_light_red` (red, `(0.31, 0.01, 0, 0.39)`), both size 30, max scale 0.1, Z bias 4, flare texture 2
  **[verified]**. One fixed colour per flare; there is no cycling.
- Knockable props ("smackables"): `trafficlight` (inherits `largemetalpost`: mass 200 kg, detach force 7500, respawn
  time 120 s, cost-to-state 50, event sequencer `trafficpole_seq`, moment `(0.5, 1, 0.5)`), `trafficcablelight`
  (mass 100 kg, no detach force, sequencer `trafficcablelight_seq`) and `traffic_cable` (plastic, sequencer
  `trafficcable_seq`, no trigger). All three have "no car effect" set, so a car hitting them takes no damage
  ([smackables are the world's breakable objects, spawners chunk `0x34027`](../formats/world.md)) **[verified]**.
  Which scenery model maps to which smackable collection was not checked (**[unconfirmed]**).

Rewrite guidance: draw the models and flares; make the poles knockable if smackables are implemented; **do not**
add signal phases or stopping, because the original traffic ignores them.

## 2. Intersections and priority

Traffic crosses junctions by driving the curved **decision segments** of the road network from its approach road to
a randomly chosen exit road (the choice rules are in [ai-road-network.md §4.3](ai-road-network.md#43-choosing-the-next-segment-traffic-navs-decomp)).
There is no priority, yielding, stop line or right-of-way logic. Two cars whose paths cross see each other only through the
generic car-ahead rule: each car's trail is cut around any car overlapping its look-ahead region, and each slows or
stops for a car in front inside its stopping distance ([ai-traffic.md §5.1](ai-traffic.md#51-avoiding-cars-ahead)). Cars
approaching each other head-on in a junction therefore usually brake and may deadlock until one is recycled by the
removal rule; this is a property of the original.

## 3. Horns, engine and drive-by sounds

The sound of a traffic car is a separate system that follows the car; the AI never honks and does not know about
the sound. A traffic car has the sound behaviour `SoundTraffic` (sound context "traffic", mixer state `eMM_TRAFFIC`).
Per car the sound layer creates up to four effect objects (`eSFXOBJ_TRAFFIC_TYPES`): engine (0), whoosh (1), horn (2)
and skids (3); trucks (`TruckSndFX` true in `pvehicle`) use the truck variants of the whoosh and the horn.
Mixer outputs per traffic car: 0 azimuth, 1 engine volume, 2 horn volume, 3 whoosh volume, 4 pitch, 5 and 6 whoosh
triggers ([dynamic-mixer.md](dynamic-mixer.md), [car-sound-mixer.md](car-sound-mixer.md)). The wheel sound of traffic
does not compute separate left and right wheel positions.

### 3.1 Engine

- The engine is a `FX_TRAFFIC` sound chosen by `pvehicle.TrafficEngType` (0 cars, 2 vans / pickups / wagons / cement
  and dump trucks, 8 garbage and fire trucks **[verified]**; the sample sets are in [engine-sound.md](engine-sound.md)).
- Each frame: `range = clamp(speed_mph * 7.3142858, 0, 1024)` (1024 at 140 mph), volume = mixer output 1, azimuth =
  output 0, pitch offset = mixer pitch output 4 minus 0x1000. The pitch offset is where the Doppler shift (from the
  dynamic mixer's 3D control) arrives. While the car is not being simulated (inactive) volume and azimuth are
  set to 0.
- The traffic car's position, forward vector and velocity are handed to a 3D position controller
  (`SFXCTL_3DTrafficPos`) that the mixer reads.

### 3.2 Horn

Each car has its own state: `playing`, `horn_start`, `duration`, `last_horn_end`, `last_attempt`, plus one global
`last_honk_time` shared by all traffic. Every sound update (`t` is the sound clock):

```
if not playing:
    start if all of:
        car is being simulated
        t > last_horn_end + 3.0         and      t > last_attempt + 3.0
        for some local player p:
            p.forward_speed > 10 m/s     and     player_in_range(p)
    if started-conditions hold:
        if uniform(0,1) < 0.3  or  t > last_honk_time + 5.0:
            sample = pvehicle.HornType
            duration = 1.2 + uniform(0,1) * (4.0 - 1.2)
            start the horn sample;  horn_start = t;  last_honk_time = t
        else:
            last_attempt = t
else if t >= horn_start + duration:
    fade out linearly over 150 ms, then stop;  last_horn_end = t
```

`player_in_range(p)` (positions in the sound system's world, where the vertical axis is z, hence the planar tests):

```
d = distance(car, player)
if d > 20 m:           false
if d < 3 m:            true
u = unit planar vector car -> player;  f = unit planar forward of the car;  c = dot(u, f)
drag race:             c >= cos(115 degrees)
other:                 c > cos(40 degrees)   or   (c < -cos(40 degrees) and d < 10 m)
```

So a car honks at a fast-moving player (faster than 10 m/s forward) who is within 20 m in front of it (a 40 degree
cone), or very close behind; in drag races at nearly any angle except straight behind. Roughly 30 % of the checks
succeed, and a honk is forced if none has happened anywhere for 5 s. While playing: pitch = mixer output 4, volume =
mixer output 2 times the fade, azimuth = output 0. `HornType` (5, 6 or 11 in the data) is the id of the horn
sample **[verified]**. The truck horn class has no 3D position controller (its controller query returns -1), so a
truck horn is probably not positioned in 3D (**[unconfirmed]**).

### 3.3 Drive-by whoosh

When the nearest player car is within 10 m and its 2D relative speed to the traffic car is above 10 m/s, and none
played in the last 3 s, a short "whoosh" sample from the set `pvehicle.WooshType` (9 for cars and vans, 6 for trucks,
semis and the ambulance **[verified]**) is started with intensity `clamp((relative_speed - 10) / 50, 0, 0.99) * 127`
(sample variant chosen by that intensity). It ends when the relative speed drops to 10 m/s or less, the sample
finishes or the game pauses. Volume, azimuth and pitch come from the mixer (outputs 3, 0 and 4).

### 3.4 Skids

Traffic tyre skids use a traffic variant of the skid effect; not read in detail (**[unconfirmed]**).

## 4. Scripted traffic in drag races

Drag races (11 of them in the install, e.g. `race_bin_10/10_7_3_drag`) set the pattern `drag` and a traffic level
of 100, so the manager also spawns ordinary traffic from the `drag` pattern, but they add **scripted** traffic from
the event data **[verified]**:

- Each race has 11 to 27 **traffic spawn triggers** (197 in total, parent `trafficspawntrigger`): a position, a
  rotation, a **radius** (50 by default; 200 in 93 of them, 250 in 30, range 130 to 360 m), `AutoSpawnTriggerType`
  `traffic` (all), an **initial speed** (30 in 183 of them; also 15, 20, 35, 40, 45, 50; unit not read, probably
  metres per second), a target marker and a `TrafficCharacter`. Each such character names a car (`CarType`):
  `trafficcoup` 23, `trafvanb` 18, `trafpickupa` 15, `traf4dsedb` 12, `trafsuva` 12, `trafha` 11, `traftaxi` 9,
  `trafstwag` 10, `traf4dseda` 8, `trafcemtr` 6, `trafdmptr` 5, `trafgarb` 5, `trafnews` 5, `trafminivan` 4,
  `semicon` 4, `semib` 1, `semicrate` 1.
- The race lists them in `RandomSpawnTriggers`. Presumably a trigger fires when the player comes within its radius
  and spawns its character's car at its position and rotation with the initial speed, then drives it with the
  traffic action; the `SetTrafficSpeed` message with the `fixed` flag (which turns off the curvature and car-ahead
  speed rules) is the natural way to give such a car a constant speed.
  The runtime (`GTrigger`, `GCharacter`) is not in the decompiled sources read, so this is **[unconfirmed]**.
- Larger set pieces such as `traffic1`, `traffic2` (a semi, `semicrate`/`semicon`, with a spawn marker, a marker and
  one to three directional plane triggers of width 30) are scripted crossing vehicles.
- The traffic AI treats cars with driver class "none" (scripted characters) like traffic when racers avoid them
  ([ai-road-nav-trail.md §5](ai-road-nav-trail.md#5-hole-punching-around-other-cars-decomp)).

## 5. Parked cars

A parked-car spawner activity (`AIParkedCarSpawner`) is created beside the traffic manager and the vehicle-info
type has an "is parked car" query. Neither is in the sources read, so parked cars are **not specified here**
(**[unconfirmed]**, open question).

## 6. How to check it

1. Drive through a junction with working-looking lights at every setting of the time of day: traffic should never
   stop for a red or green phase; the lamp flare colour should not change.
2. Hit a traffic-light pole: it falls (mass 200) with no damage to your car; it returns after about 120 s.
3. Approach a slow traffic car at 60 mph and honk-test: stay within 20 m ahead of it at more than 10 m/s; a horn
   should start within a few seconds on average, lasting 1.2 to 4 s, and never twice within 3 s from the same car.
4. In a drag race expect scripted cars crossing in front of you at the trigger radius in addition to the ambient
   traffic.
5. Pass an oncoming traffic car at over 10 m/s relative speed within 10 m: a whoosh once per 3 s at most.

## 7. Open questions

- Whether the draw code switches a traffic light's lamp by any state; no data found.
- The runtime of scripted traffic (`GTrigger`, `GCharacter`): speed unit, orientation, who despawns the cars.
- Parked cars.
- The mapping from scenery model to smackable collection.
- Sample sets behind `HornType`, `TrafficEngType` and `WooshType` (sound banks, [engine-sound.md](engine-sound.md)).

## 8. Rust implementation notes

- Render the traffic-light and sign scenery as normal props; do not add a signal system.
- Put the horn / whoosh logic in the audio layer, driven by the player's speed and the distance and angle to each
  traffic car; share one `last_honk_time` across cars. It needs only positions, forward vectors and the clock.
- Read `HornType`, `TrafficEngType`, `WooshType` from the car's `pvehicle` and pass them on as sample ids.
- Scripted drag traffic can wait until race events exist; the ambient traffic of the `drag` pattern works from
  [ai-traffic-spawning.md](ai-traffic-spawning.md) alone.
