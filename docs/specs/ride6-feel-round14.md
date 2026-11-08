# RIDE 6 bikes, round 14: the thrown rider and the edge of the world (clean-room spec)

Four reports of the players, after the thrown-rider camera ([ride6-bike-cameras.md](ride6-bike-cameras.md), section 8), the ragdoll
([ride6-rider-fall.md](ride6-rider-fall.md)) and the get-up ([ride6-rider-getup.md](ride6-rider-getup.md)) were in:

1. "when the bike driver falls, the camera shakes too much."
2. "also it can go under the map but it shouldn't."
3. "also when going fast and turning and i hit the wall the bike starts wallriding for a bit, but then it falls under the map. wrong."
4. "when the bike driver starts falling under the map it takes a while to respawn over the map. sometimes the bike driver gets launched
   into the sky."

Tags: **[M]** measured in an offscreen run of this project (the commands are in section 6), **[R]** read in this repository's code, **[C]** a
choice of this project. RIDE 6 has no such camera, no ragdoll against the world and no under-the-map watch that was read from its binary, so
everything here is **[C]**; the reference's reset for cars is [vehicle-reset.md](vehicle-reset.md), and the reset used here is that one.

## 1. The camera shakes (report 1)

**What was measured** [M]. A KTM 450 thrown high-side at 25 m/s on the road (`fall=high`), the chase camera on the rider; the point the camera
follows, every physics step:

| | before | after |
|---|---|---|
| acceleration of the followed point (the pelvis), mean / 95th percentile / worst | 49 / 88 / 362 m/s^2 | see section 6 |
| the bike's own contacts that reach the camera's shake (acceleration of the body over 100 m/s^2), steps of 661 | 15, worst 690 | see section 6 |

A body in free fall accelerates at 9.8 m/s^2: the mean of 49 is the pelvis of a ragdoll turning on the road (its capsule rolls, so the bone's
origin, which is not the capsule's centre, moves) and it went into the camera unfiltered: the chase camera places itself at `look + offset`
where `look` is the followed point, and only the offset is smoothed (`FOLLOW_RATE` 10 per second). There were two causes, and a third source:

1. **The followed point was the raw pelvis** ([R] `viewer/camera/rider_follow.rs`, `RiderCam::follow_rider`), and its velocity the difference of two
   raw positions, from which the heading was taken at 1.5 m/s and above: at a slow slide the shake's velocity (a few m/s the other way) turned
   the heading back and forth.
2. **The bike's contacts shook the camera that was not on the bike.** While the rider is thrown the bike lies and slides on its box behind him,
   and every step of that is a ground contact: `CarSim::camera_acceleration` (the chase camera's impact shake, `viewer/camera/chase/shake.rs`,
   after the original's `docs/specs/contact-camera-shake.md`) and the contacts of the original camera timeline
   (`Drive::original_impacts`) both read the *bike's* body.

**What was done** [C].

- `viewer/camera/damped.rs`: a critically damped spring (the exact solution of `x'' = w^2 (target - x) - 2 w x'` for a step, so stable at any frame
  rate), no overshoot, its acceleration capped. `RiderCam` follows the pelvis with it (`FOLLOW_OMEGA` = 35 per second: a steady 20 m/s is followed
  1.1 m behind, what moves faster than about 5 Hz is taken out; `FOLLOW_MAX_ACCEL` = 300 m/s^2). The velocity the camera takes (the original views'
  body velocity, the heading) is the spring's, not a difference of positions.
- The heading the camera sits behind turns at most `HEADING_MAX_RATE` = 5 rad/s towards the direction of the smoothed slide.
- The bike's contact shake fades with the camera's attention: `chase.impact(acceleration * (1 - weight))` and the original timeline's closing speeds
  times `1 - weight` (`rider_cam.weight()`: 0 on the bike, 1 on the rider). The hit that threw him is still felt, for the 0.35 s the camera
  takes to move to him.
- The followed point is kept above the ground: the host says the height of the ground under the pelvis each step (`RiderCam::set_floor`), and
  `above_ground` raises a point that is up to 1.5 m under it to 25 cm over it (a pelvis in the road, which the ragdoll's capsule radius allows);
  further under is not a pelvis in the road but a rider under the map (section 4).

Tests: `viewer/camera/damped.rs` (the step reached without overshoot, the lag `2 v / w`, the jitter of 6 cm at 60 Hz reduced more than 8 times,
the acceleration cap, any frame rate, bad input), `rider_follow_tests.rs` (a steady slide is a steady point, 6 cm of noise shakes the point by
under 100 m/s^2 against 1500 raw, the cap, the heading does not flip with the shake of a slow slide, the heading's rate, the point above the
ground, the point on a sunk rider), `scenes/world/drive` (the contact shake's weight).

## 2. Under the map: the rider is a free body in a world of roads (report 2)

**What was measured** [M]. `fall=high` at 25 m/s on the road at (1065, 4455) in the city, 12 s after the throw: the rider's pelvis at
z = -267 m (the road is at 214), speed 98 m/s, the camera at -267 too; a screenshot in a black void. And the yamaha R1M at 43 m/s steered into the
edge of the road: the rider crossed it 0.4 s after the throw (y from 4405 to 4380 in 1 s, over the road's edge) and fell for the 4 s that the
bike waited for him.

**Why** [R, M].

1. **The ragdoll met the road only.** `ragdoll/contact.rs` casts one ray down from 40 cm over each end sphere, against the faces of the collision
   packs. The edge of a road in this game is an invisible wall of the packs, 3 m or more tall ([ride6-feel-round10.md](ride6-feel-round10.md)),
   and there is no ground behind it (in the city the road is a ribbon): a rider thrown at it at 25 to 60 m/s went through it and fell for ever. The
   bike's body meets the same wall through its box (`walls::resolve_into`); the rider's body never asked.
2. **A sphere that ended under the surface by more than 40 cm found no ground at all** (the ray starts under the face: no hit), and fell on.
3. **The camera followed him down** (the followed point was the pelvis, wherever it was), and the bike, which waits for the rider to get up (up to
   `TUMBLE_LIMIT` = 8 s), waited for a rider who never lay still.

**What was done** [C], in the generic ragdoll (`libs/blackbox-vehicle/src/ragdoll`, free of the game):

- `Ground::wall(from, to)` (default: no walls), implemented by the game's `WorldGround` with the walls the player's body meets (`walls::world_cast_with_mask`
  with no driver mask: barriers, one-way ones from their front, and steep faces; not the floor). Once per step, before the sub-steps
  (`ragdoll/walls.rs`) every end sphere that moves at 0.5 m/s or more casts a segment along its motion (a step's travel and its radius) and is
  stopped at the wall: an impulse along the wall's normal that leaves it at the wall (no bounce: the rider's material has no restitution) and
  Coulomb friction along it with the surface's own multiplier; a sphere already inside is moved out (at most 30 cm per step). Cost: at most 32
  rays a step while he moves.
- A sphere that is down at the level of the ground its body last touched and finds none now casts a second ray from 2.5 m above and takes
  what it meets for the ground if it is no more than 1.2 m above that level (a kerb, a step), not a deck above him.
- The ragdoll's solver turns the moves of a sub-step into velocities (position-based dynamics). A push out of a surface of 40 cm in one sub-step
  was a velocity of 190 m/s, clipped to the solver's 150: "sometimes the bike driver gets launched into the sky" (report 4). Now a contact pushes
  what the body's own motion into the surface needs and at most 2 cm of the overlap it was already in per sub-step, and that recovery is not
  counted into the velocity (`Body::recover`; split impulses by another name).
- The speed of a segment is held: 100 m/s at the most (a bike's 90), and sideways or upwards a little over what the body was thrown at
  (`1.2 v + 12 m/s`); falling is not capped. The launch spin is held to 40 rad/s.

Tests: `ragdoll/guard_tests.rs` (a body started 25 cm into the road leaves it at under 3 m/s; a launch of 500 m/s is clipped; a body thrown at
a wall at 20, 50 and 80 m/s ends against it and at rest; along a wall it is not stopped; a body that sank under a road that rose is brought up
without being thrown; a deck 2 m above is not taken for the ground; a fall is not capped), `drive/sim/bike_curb_tests.rs` (a three-segment rider thrown at
the end of a road, 9 combinations of 30 to 70 m/s and 30 to 90 degrees, with the void behind the wall, never through it and never under the
road).

## 3. The wall ride (report 3)

**What was measured** [M]. The yamaha R1M at 43 m/s steered into the road's edge (`steer=1` for 4 s from 48 m/s): the box of the bike climbs the wall: the
bike rose 1.7 m in 0.5 s and came down 30 m further on (z 214.5 to 216.2 to 214.7), the rider thrown across the wall as above. A bike that
was thrown by the wall is a body double of the bike's mass on a box: the wall's reaction at a corner of the box carries friction along the wall
and the pitch of the frame climbs it.

**What was done** [C]. The reaction itself is unchanged (the numbers of [ride6-feel-round10.md](ride6-feel-round10.md) hold: from 14 m/s of change of
velocity a wall throws the rider). The push that takes the body out of the wall (`walls::push_out`) is now limited:

- never **down**. The probes accept faces up to a normal 0.7 up or down as walls; a wall that leans over the road has a normal that points a
  little down, and a push along it moved the body into the road, under the reach of its tyres' rays (0.6 m above the contact) and of the box's
  corners, and the bike was in free fall through a surface;
- at most **2 m** in a step (a bike at 90 m/s moves 1.5 m in a step, and the sweep of a thin wall puts the centre back by that much): a contact that finds the body deeper than that found it already inside
  something, and must not throw it across the road.

Tests: `walls/push_tests.rs` (the push never goes down and is held to 2 m; a car in a wall that leans over the road is not pushed into the road),
`bike_curb_tests.rs` (the whole bike on the end of a road at 40, 50 and 60 m/s and 15 to 90 degrees, 8 s each, never through the wall, never more
than 30 cm under the road, the state always a number).

## 4. Under the map: the way back (report 4)

**What was measured** [M]. The bike that lay by the wall waited for its rider; the rider fell; the watch that exists (`fall.rs`: no wheel down and
nothing within 30 m below for 0.75 s, or 80 m below the last road) only looks at the bike, which lay on a road, and the 8 s of the bike's wait
ended it: the screenshots of the run show the rider falling for seconds and the bike put back 4 s after the throw.

**What was done** [C]. `drive/under_map.rs` (the logic, pure) and `drive/under_map_check.rs` (the host):

- a body is **under the map** when nothing is below it (no ground within 30 m: a rider falling onto a road below a bridge is falling onto
  something) and
  - it is a **rider**: a thrown rider has no business without ground under him, however high he is (he was thrown across the edge of the road,
    into the void behind it); or
  - it is a **bike** and either a surface is within 6 m above it (it is beneath the road) or it is lower than the last place that held it up (the
    newest place of the reset trail, else the last road) by 0.5 m (a bike may jump a gap with nothing below it, and the old watch deals with
    that).

  A bike with its rider is looked at at its origin, a thrown rider at his pelvis, so he is watched too, with no wheels of his own. The ray that
  looks for the ground below starts 0.8 m above the body: a body that has sunk less than that into a surface has the surface for its ground. All
  of it for 0.1 s: a glitch of a step does not count.
- a state that is not a number, or a speed over 150 m/s (no bike has it, no ragdoll is let have it), counts at once.
- what happens is what a reset does for a car (`vehicle-reset.md`, section "The reset"): `request_reset` (the trail's oldest place, snapped to the
  road), else the last road, else the nearest one; the bike is put there standing still with the reset's 2 s grace, the camera is cut to the bike
  and the rider is back on it at once (a rider more than 25 m from the seat is not walked to it: he was lost, and is on the seat in the next
  step). That is about 0.1 s of watch and the frame that serves the request: well under the 0.3 s asked; it was 4 s.
- the **release** of the rider is held, too: the velocity of the frame's spin at his place (`omega x arm`, up to 60 m/s for a collision's
  spike of 60 rad/s) is limited to a quarter of the bike's speed and 3 m/s, the whole to 100 m/s, the angular velocity to 40 rad/s
  (`bike/release.rs`). A rider thrown by a wall hit was launched with the spike.

Tests: `under_map.rs` (a body riding is never under the map; nothing below and under the floor counts within 0.3 s and not on the first glimpse;
in the air over a gap is not under the map; falling onto a road below is not; under a surface counts even with no floor known; a glitch of a step is
not a reset; a state that is not a number or 400 m/s counts at once; it counts again after a reset; a placement clears it), `bike/release_tests.rs`
(the spike is limited, nothing leaves faster than a bike goes), `rider/fall_tests.rs` (a rider 100 m from the seat is on it in a step).

## 5. What is not done

- Props (cones, posts, other cars) are not met by the ragdoll; only the world's faces and walls are.
- The bike's contact with a wall that leans over the road keeps the reaction of a car's body double; only the push out of it is bounded.
- A one-way barrier is passed from its back, as the bike passes it ([ride6-feel-round10.md](ride6-feel-round10.md)): a rider thrown over a rail
  from outside the road is not stopped by it.
- The shake of the original camera timeline's own noise (`NoiseControls`) is not changed; only the bike's contacts fade with the rider's weight.

## 6. Before and after [M]

Offscreen runs of the city, an isolated profile with the KTM 450 and the Yamaha R1M, `--muted`; the positions are logged every physics step by a
temporary trace (not in the repository). "Before" is the tree of `bf9c5e806`, "after" this branch.

| | before | after |
|---|---|---|
| `fall=high` at 25 m/s (KTM): the rider, 11 s after the throw | z = -267 m under a road at 214, 98 m/s, still falling; the bike waited | on the road (lowest his pelvis got over the ground: 0.19 m), fastest 27.6 m/s, back on the bike at 11.7 s |
| Yamaha R1M steered into the road's edge at 43 m/s (wall hit) | the rider crossed the wall, fell 75 m in 3 s, the camera under the map looking up at the terrain; bike put back 4.2 s after the throw (by the bike's own watch) | the rider stops against the wall and slides on the road (pelvis 0.04 m over the ground, fastest 37.8 m/s); no reset |
| the watch alone (walls switched off by a debug hook, then removed): the rider thrown across the edge | no reset for 16 s | ground lost 0.02 s after the throw (wall hit) and **reset 0.10 s after the throw**; for the high side, 0.08 s after he first had no ground (1.60 s after the throw, when he slid off the road), reset at 1.68 s |
| camera: followed point's acceleration in the 400 steps of the fall, worst / 99th percentile / steps over 150 m/s^2 | 362 / 283 / 9 | 148 / 125 / 0 (the log has 2 decimals: a floor of about 50 to 90 m/s^2 is the rounding of both) |
| bike's contacts that reach the camera's shake while it is on the rider (acceleration over 100 m/s^2, worst 690) | 14 steps, at full strength | 0 (they are scaled by `1 - weight`: worst 28 after it) |
| rider speed after a wall hit, fastest of the run | 60.9 m/s (sliding off the world); 97.7 m/s falling | 37.8 m/s |

Pictures (frames of the end of a script, so the moment is chosen by the script's length): `C:/r6work/fall-out/evidence/wall-and-high-before-after.png`
(top row: before, the Yamaha at the wall 0.6, 1.6 and 3.6 s after the hit: the camera under the map, then the terrain seen from below and the rider falling
in the void; middle row: after, the same moments: the rider tumbling on the road beside the rail, upside down, lying; bottom: the KTM's high side at 3 s
and 6 s), `C:/r6work/fall-out/evidence/before-2-rider-under-map.png` (the KTM, before, 11 s after the throw). The 6 s frame of the high side was taken before the camera's re-anchoring
at the remount (below) and shows the 0.4 m pop of the point when the bike stands up; `RiderCam::step` now moves the offset with the bike
(`the_bike_standing_up_at_the_remount_does_not_move_the_followed_point`).

The commands (`BLACKBOX_PROFILE_DIR` an isolated profile with the bike, `--muted`; the script's length decides the frame):

```
nfsmw view-world --drive KTM450 --at 985,4430 --no-traffic --camera-mode chase --wait-for-load --muted   --drive-script "5:throttle=1;1:fall=high;3:throttle=0" --screenshot out.png --screenshot-delay 0
nfsmw view-world --drive YAMAHA_YZF-R1M-2020 --at 985,4430 --no-traffic --camera-mode chase --wait-for-load --muted   --drive-script "7:throttle=1;2.6:throttle=1,steer=1" --screenshot out.png --screenshot-delay 0
```

The second one is 7 s of throttle (about 48 m/s at the 1200 m mark of the road) and the stick held right into the edge of the road; 2.6 s of it is
0.6 s after the hit, 3.6 s and 5.6 s are later frames. A survey in the installed world (`bike_wall_survey_tests.rs`, ignored, needs
`NFSMW_GAME_DIR`) rides 40 places x 4 sticks x 3 speeds (40, 50 and 60 m/s) into the road's edge for 8 s and counts the rides that end more than 1 m under
the ground below the bike.

Run on the installed city with the example bike: **480 rides, 290 of which threw the rider, 0 ended under the ground** [M].
