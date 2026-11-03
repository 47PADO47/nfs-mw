# RIDE 6 bikes, round 10: a bike meets the world's kerbs and walls (clean-room spec)

Round 9 (`ride6-feel-round9.md`, section 7) found that the library does not throw a bike over a 10 or 15 cm kerb (0 falls in 180
configurations of `bike_curb`) and left the falls the players see to the host: "needs a world test with a collision pack that has a kerb".
This round reproduces them through the path the game uses and fixes what it finds. Tags: **[M]** measured here, **[D]** read from the
reference's data (`physdump-all`, the input handlers), **[C]** a modelling choice of this port.

## 1. What a bike meets at the side of a road [M]

Two tests ride a bike through `CarSim::step` with a collision world (`drive/sim/bike_curb_tests.rs`), not through the library's ground:

- **A kerb as geometry** (a raised slab across the road or beside it, with a vertical or a bevelled face of its own normal; 10 and 15 cm;
  straight, at 45 degrees and with the stick towards a kerb beside the road at 10 to 30 m/s, the bike leaned to 0.6 rad): the tyres' vertical
  rays climb it, **no wall contact is ever made and the bike never falls** (5 to 35 m/s). The wall probes cast from the centre of the body box
  to the mid-height of its outline, 0.5 to 0.9 m above the road, so a face of 15 cm is below them whatever the lean; the faces a probe does
  hit are kept apart from the ground by `FLOOR_NORMAL` in `walls.rs`. A low step is already "terrain", as the supervisor's hypothesis wanted;
  nothing had to change there.
- **The installed world** (`survey_installed_roadsides`, ignored, `NFSMW_GAME_DIR`): 40 places on the default area's roads, a bike ridden
  at 8, 14, 20 and 28 m/s with the stick at +-0.15 and +-0.3 for 1.5 s from the 60th step, or until the end (640 rides, an example bike, the
  imported KTM 450 and Aprilia RS 250). **Every wall contact is a `Barrier`** (over 20 000 contacts, 4 of them steep faces): the edge of a road in this
  game is an invisible wall of the collision packs, 3 m or more tall, standing on or beyond the kerb. It is what a bike "meets" at a kerb,
  and a leaned bike meets it with the low corner of its box (0.1 to 0.4 m below the box's centre).

## 2. Why the bikes fell [M]

Before this round (`BikeRide::pull` took everything the contacts left in the double's velocities), of the 640 rides of each row, those
that touched a wall:

| ride | touched a wall | fell | of those, within 1.5 s of a wall |
|---|---|---|---|
| KTM 450, stick for 1.5 s | 308 | 54 | 54 |
| KTM 450, stick to the end | 387 | 103 | 101 |
| RS 250, stick for 1.5 s | 320 | 93 | 79 |
| RS 250, stick to the end | 408 | 166 | 149 |

Tracing the falls (the fall tests of `bike/fall.rs` fire on the state the step after the contact):

1. **The reaction of the body double is a car's.** The wall reaction acts at a corner of the box, a metre ahead of the centre of gravity,
   a hand's width off the heading and, when the bike is leaned, 0.1 to 0.4 m below the centre. A touch with a closing speed of 5 m/s (a
   glancing 13 degrees at 20 m/s) leaves 3 to 4 rad/s of pitch and roll in the double: the angular-speed test (4.3 to 4.4 rad/s [D]) fires
   the step after (LowSide), or the roll goes on into the lean test (`FallLeanLimit`, 1.22 rad). Of the 131 falls of the first survey,
   124 came within 1.5 s of a wall contact, 67 with no other cause than the jump of the angular velocity.
2. **A wall at speed did not crash a bike.** The reference's collision test is the one row of the fall table the model never had
   (`ride6-chassis-round6.md`, section 3, "collisions"). A bike ridden head-on into a wall at 20 m/s stopped against it and stayed up
   (157 contact steps, no fall): the double's reaction is through the centre of mass, with no torque to trip a test.
3. **Hits are not the only cause**: a stick held into a wall for seconds leans a bike over (section 5).

## 3. What the reference does, and what this port does [D, C]

The eight input handlers (`Physics/VehicleElements/InputHandler`) carry (`FallCollisionImpulseLimit` 2e6, `FallCollisionChannels`
[`ECC_WorldDynamic`], `FallCollisionSpeedThreshold` 10, `FallCollisionDuration` 2, `FallCollisionToleranceTime` 0.1) and, for the help
after a collision, `CollisionLeanHelpAlteration` (`MaxHelpAddition` 2, `HelpAlterationFadeRate` 5, equal for the frontal kind) and
`CollisionStabilityHelpAlteration` (0.4 at a fade rate of 5; frontal 0.2 at 1). The six road-bike handlers agree; the two scooters'
differ (lean addition 0.5 and 4, one frontal stability fade of 5) and are not modelled. So in the reference:

- a collision test exists, with an impulse limit nothing reaches, on the **dynamic** channel only: a static wall never throws a rider;
- a collision **adds to the help of the controllers** (`ride6-chassis-round6.md`, section 2: `h = clamp01(... + A)`) and the addition
  fades away: the lean help is at its whole factor for 0.4 s, and the stability help (a yaw torque towards the yaw rate the lean needs)
  gets 0.4 on top of its tuning for 0.08 s.

The port (`drive/sim/bike/contact.rs`, `bike/chassis_state.rs`, `bike/fall.rs`):

- **`Bike::notify_collision(frontal)`** raises the two additions to the data's values; they fade linearly (**[C]**: the shape of the fade
  is not known) at the data's rates, in every sub-step (`choices::COLLISION_*`). `CollisionHelp` carries them in `ChassisState`.
- **`BikeRide::take_contacts`** replaces the plain `pull` after the contacts of a step. The bike takes the double's change of velocity,
  **its change of yaw but not of roll and pitch** (**[C]**: the roll and the pitch stay what the wheels, the suspension and the rider make
  of them; the reaction at the box's corner is the wrong lever for them), and the help rises for any change above 5 cm/s.
- **A hard contact throws the rider**: a change of the bike's velocity over `COLLISION_FALL_SPEED` (**[C]**, 14 m/s, 50 km/h) in one step
  is a fall (`Bike::throw_rider`, `FallKind::Generic`, the rider released as in any fall). The measure is the speed lost to the contacts of
  the step (walls, props and other bodies alike, mass-free). The data's 10 has no unit or measure; taken as m/s of a step's change of velocity
  it threw 23 of 33 bikes at a closing speed of 9 to 12 m/s (the example bike, the survey of section 4), speeds at which the controllers'
  help keeps most bikes up, so the port uses 14. It is the one place where "a wall at speed crashes a bike" lives; the angular-speed test no longer has to stand in for it.

## 4. The sweep [M]

The same rides after the change (against a build with the plain `pull`), by the closing speed of the first
contact (m/s along the wall's normal, the point velocity before the response). KTM 450, stick for 1.5 s, 308 rides with a contact:

| closing speed | rides | fell before | fell now |
|---|---|---|---|
| 0 to 6 | 156 | 0 | 0 |
| 6 to 9 | 51 | 9 | 1 |
| 9 to 12 | 40 | 13 | 9 |
| 12 to 15 | 27 | 9 | 13 |
| 15 and more | 34 | 23 | 34 |

Below 12 m/s most of the falls are gone; from 15 m/s on every wall crashes the bike, as it should (before, 7 in 10 did: the others ended
up pressed against the wall upright). Over all rides the falls go from 54 to 57 (stick for 1.5 s: the walls at speed now crash more, the
glancing ones less) and, with the stick held to the end, from 103 to 92 (KTM 450) and from 166 to 129 (RS 250). With the throw switched
off (the limit at 1000 m/s) the same 308 rides give 37 falls, and a quarter of the walls at 15 m/s and more leave the bike upright.

## 5. Regression tests

`drive/sim/bike_curb_tests.rs` (a synthetic collision pack of slabs, faces and one barrier; the example bike, or an imported one with
`RIDE6_BIKE_TOML`): a bike rides over a 10 and a 15 cm kerb, vertical or bevelled, straight or at 45 degrees, at 5, 15, 25 and 35 m/s,
with no wall contact and no fall; leaned towards a kerb beside the road (stick 0.4 and 0.7, 10 to 30 m/s) it climbs it; a wall touched at
10 to 60 degrees with a closing speed under 9 m/s does not throw it; a wall at 20 to 35 m/s head-on or at 60 or 30 degrees throws it at the
touch; a wall met at 5 m/s stops it upright. `drive/sim/bike/contact.rs` tests that a touch changes the velocity and the yaw only, that
the help rises with it, and that a hard contact throws the rider and leaves the bike all of the double's motion. `bike/round10_tests.rs`
tests the help's additions, their fade and the thrown rider.

## 6. What stays open

- **A stick held into a wall** still leans a bike over in about a second (a stick of 0.3 into a wall at 8 to 25 degrees, 12 to 28 m/s, held
  for 6 s: 7 of 9 rides fell before, 8 of 9 now):
  the wall stops the bike turning, the lean controller's force at the front tyre goes on, and the roll the wall's reaction would have
  balanced is no longer taken from it. A rider who turns into a wall and holds it is thrown by a real bike too; whether this port should
  hold him up (the help is full for 0.4 s only) was not decided.
- The collision test of the reference for dynamic bodies (`FallCollisionSpeedThreshold` on the `WorldDynamic` channel, with its duration
  and tolerance time) is modelled by the step's change of velocity for every body; the ragdoll's own contacts with walls are not.
- Pair contacts (a car against a bike) go through the same `take_contacts`: their roll and pitch are dropped as a wall's are. Not measured.
- The double's box is a car's outline: a wall is met 0.4 m to the side of the centre plane and a metre ahead of the centre of gravity,
  where a bike has its tyres and its bars. A box of the bike's real shape (a wheel capsule pair) would give honest contact points; the
  imported `BOUNDS.BIN` has only the box.
- The in-game check was not run (the release binary of the day predates the change): `nfsmw view-world --drive KTM450 --at <x,y> --muted
  --drive-script "..." --screenshot out.png` offscreen reproduces the rides of the survey one at a time.

## 7. Tools

`survey_installed_roadsides` (`RIDE6_BIKE_TOML=<manifest>`, `HOLD=<steps>`, `NFSMW_GAME_DIR`), the synthetic tests above,
`bike_ride_hazards.rs` (`bike_curb`, the library's own ground).
