# RIDE 6 bikes: jumps and landings in the arcade steering (clean-room spec)

**This is a modification made exclusively for the NFS:MW rewrite. It is not the behaviour of RIDE 6.** It is part of
`blackbox_vehicle::bike::ArcadeSteering` (the field `air`, `ArcadeAir`, `libs/blackbox-vehicle/src/bike/arcade_air.rs`;
[ride6-arcade-steering.md](ride6-arcade-steering.md)); `ArcadeAir::REFERENCE` is neutral in every field and `bike_steering = reference` gives the
reference behaviour back exactly. Tags: **[R]** read in this repository's code, **[D]** data (the imported manifests), **[C]** a choice of this project
with its reason, **[T]** covered by a test that is written and **has not been run yet** (the agent rules of this round forbid building before the code is
done), **[M]** measured (section 6: **none yet**).

## 1. What the player reported

"if the arcade mode is selected for ride6 mod, improve the ability to jump with the bike because currently if i try to do a jump the bike just implodes
instead it should handle it great, like gta, especially if its an offroad or motard bike".

## 2. What a jump asks of a bike, read from the code [R]

A bike that leaves a ramp or a crest is in the air with both wheels free (`WheelState::on_ground` false on both). What can end it, in the order the
code meets them:

1. **The fall tests** (`bike/fall.rs`, once per step): the lean beyond `airborne_lean` at once in the air (70 degrees), beyond the fall lean for
   `lean_duration` (0.01 s) on the ground; the pitch beyond 65 degrees; the angle between the velocity and the heading beyond 85 degrees (above
   10 km/h); the body's angular speed beyond `airborne_angular` (7 rad/s) in the air and 4.4 rad/s on the ground, times the arcade steering's 3.
   A landing is the moment when all of them are tested against a body that was in free flight a moment before: the first wheel to touch turns the
   body (a front wheel first rolls it nose-down at several rad/s), and the lean test then reads a lean that the flight left it.
2. **The handlebar in the air.** With the front wheel up the lean controller still plans a front force and turns it into a handlebar angle
   (`steer_handlebar`); the reference's mid-air steer takes the bars away only when the lean is under 40 percent of its limit and the nose is not
   dropping faster than 0.4 rad/s (`wheelie_steer.rs`), which a jump coming in is not. A front wheel that lands turned towards its lock at 25 m/s has
   a lateral force that the stability and lean helps and the fall tests then read as a high side.
3. **The load of a landing.** A tyre's load is the suspension's force along the axis plus the tyre's radial stiffness times what the stops could not
   take (`wheel_step.rs`: "the end of the travel is a hard stop that adds no force of its own"). A bike that lands a 5 m drop arrives at 9.9 m/s into
   the road with 0.27 m of travel: about 11 kJ over a stroke of 0.3 to 0.4 m is 30 kN and more, and the bottomed stop hands what is left to a tyre
   that is stiff, so one step carries an impulse several times the bike's weight.
4. **The surface and the lean controller** are not in the air, but they are in the first second of the landing: see
   [ride6-bike-rough-ground.md](ride6-bike-rough-ground.md).

Nothing of the reference game levels a bike in the air, centres its bars in a jump, or softens a landing: a rider of RIDE 6 does it with his weight.

## 3. What the help does (`ArcadeAir`) [C]

Each strength is multiplied by the bike's class (section 4). `ArcadeAir::default()` is this game's; every field of `REFERENCE` is neutral.

1. **Levelling in the air** (`level_*`, `Bike::apply_air_attitude`). Both wheels off the road for `level_delay` seconds (0.08 to 0.25 s, a ramp: a bike
   keeps the attitude of its take-off for a moment, a hop over a kerb is not levelled): a torque turns the bike's up towards **the normal of the
   road below it** about its pitch and roll axes (never its yaw: that is the rider's, and the player's air control, [air-control.md](air-control.md)),
   a spring `level_gain` 12 (rad/s^2 per rad) and a damper `level_damping` 4.5 /s, held to `level_accel` 7 rad/s^2 times the class factor. The road
   below is probed by each wheel's ground ray over **8 m** (`wheel::AIR_REACH`; the contact itself is still found within 2 m), so a bike that jumps
   down a hill pitches nose-down to the slope it will meet before it meets it. The spring and damper are a second-order system of 3.5 rad/s and a damping
   ratio of 0.65: a nose-up spin of 2 rad/s is stopped within 0.5 rad (29 degrees, under the 65-degree fall limit) and gone within a second
   ([T] `a_bike_thrown_nose_up_off_a_ramp_levels_in_the_air_and_the_reference_flips`, `the_bike_pitches_to_the_slope_it_will_land_on`).
2. **The bars centre in the air** (`handlebar_centre` 1, `Bike::bars_centred_share`): from 0.03 s in the air over 0.15 s the controller's handlebar
   (and the mid-air steer's) is taken away and the bars come to the middle, so the front wheel lands straight ([T] `the_bars_are_centred_in_the_air`).
3. **The landing is soft** (`landing_*`, `load_cap_g`). For `landing_window` 0.5 s after the landing of a jump (an air time of at least
   `min_air_time` 0.25 s) the body's speed **into the road** (along the normal of the wheels that touch it) above `landing_soft_speed` 2 m/s gets an
   extra deceleration of `landing_gain` 25 /s times the excess, held to `landing_accel` 110 m/s^2 (11 g) times the class factor, acting at the centre
   of gravity (no torque) and gone as soon as the bike no longer moves into the road. It takes away the speed the suspension and the tyres cannot: a
   drop of 5 m needs about 135 m/s^2 over the 0.3 to 0.4 m of the stroke if the springs did it alone; the assist does the first of it and the springs
   the rest. And a tyre's load is **bent to a cap** of `load_cap_g` 14 times the bike's weight by `knee + room x tanh((load - knee) / room)` with the
   knee at 0.6 of the cap: untouched below it, never above the cap, no kink ([T] `the_reference_steering_has_no_help_in_the_air_and_no_cap`,
   `the_landing_load_is_held_under_the_cap_and_never_above_what_it_was`). This is the soft bottoming: the stops' leftover is not an impulse.
4. **The fall tests forgive the landing** (`landing_grace` 0.9 s, `landing_tolerance` 1.8, `Bike::fall_tolerance`): for 0.9 s after the landing of a jump the
   limits of the lean (ground and air), the pitch, the heading angle and the angular speed are wider by `1 + (1.8 - 1) x class factor`, never past 85
   degrees of lean, 80 of pitch and 86 of heading angle (`air::MAX_*`): a landing that turns a bike at 18 rad/s is a landing, a tumble of 30 rad/s is
   still a fall ([T] `a_spin_on_landing_that_throws_the_reference_bike_is_forgiven_by_the_help`, `a_landing_is_forgiven_for_a_while_and_a_hop_is_not`).
   The limits never get narrower, and a tolerance of 1 leaves them exactly as they were.

### Why these values [C]

- The fall limits are the data's (70, 65, 85 degrees; 4.4 and 7 rad/s) and the arcade steering has already tripled the ground angular speed; the
  grace is 0.9 s because the helps of a landing (the lean help's whole factor, the stability help) need that long to take hold of a bike that
  arrived turned; it ends before the next flight can begin, since `touched` and the grace start again at every landing.
- `min_air_time` 0.25 s is the time a 0.3 m drop takes: shorter is a bump, which the suspension handles ([T] `a_hop_is_no_jump`).
- `landing_accel` 110 m/s^2 is the deceleration that alone would stop 9.9 m/s (a 5 m drop) in 0.45 m (`v^2 / 2a`); the springs and the damper add to it.
  With the class factor 1.5 the offroad bikes get 165 m/s^2, which they can use because their travel is the longer.
- `load_cap_g` 14: a bike's springs at the end of the travel give 6 to 9 times its weight; above 14 the load is the stop's impulse, which is what
  this help takes away.
- The bars are centred in full (`handlebar_centre` 1) because the controller's angle in the air has no tyre to be tested against.

## 4. The class [D]

`BikeSpec::terrain_class()` (`bike/class.rs`) reads it from the bike's own data: **the front fork's travel** (every road bike of the 293 has 0.10 to 0.16 m,
scooters and the MV Agusta Rivale 0.15; every bike made to leave the road 0.18 m or more: adventure 0.18 to 0.27, supermotos 0.22 to 0.285) and **the
front wheel's outer radius** (adventure and enduro bikes 0.327 to 0.349 m, a 19 to 21 inch wheel; supermotos 0.288 to 0.300 m, 17 inch).

| class | rule | the bikes of the 293 | `class_factor` of `air` |
|---|---|---|---|
| Road | travel under 0.18 m | the superbikes, nakeds, classics, cruisers, scooters (the large majority) | 1.0 |
| Motard | travel 0.18 m or more, front radius under 0.32 m | CRF 450 R SMT (2), KTM 450 SMR, KTM 690 SMC R, TM SMX (2), Ducati Hypermotard 698 | 1.25 |
| Offroad | travel 0.18 m or more, front radius 0.32 m or more | Africa Twin (2), Tenere 700 (2), Tiger 900 Rally Pro, KTM 1290 Super Adventure R, KTM 890 Adventure R, Aprilia Tuareg, Ducati DesertX, Multistrada V4 Rally, BMW R 1300 GS | 1.5 |

The rider's own table (`rider/choices.rs`, `class_of`, from the importer's `kind` and the input handler) is a different thing with three groups of
its own (a scooter is "offroad" there); the physics has no access to it, and the long-travel rule matches the user's words ("offroad or motard").
No motocross or trial bike is among the 293. The two thresholds are named constants (`LONG_TRAVEL`, `BIG_FRONT_WHEEL`) with the gaps of the data
in their documentation ([T] `the_class_follows_the_fork_travel_and_the_front_wheel`).

## 5. Reference mode is unchanged

Every new term is a product with 1, a sum with 0, a `min` against a cap that is never lower than the limit, or an early return, so a bike with
`ArcadeSteering::REFERENCE` (or any steering whose `air` is `ArcadeAir::REFERENCE`) takes exactly the steps it took: the load cap, the levelling, the
assist and the centring are off, the fall tests' widening is `widen(limit, 1.0, cap) == limit` bit for bit, and the grip and the damping are scaled by
1 ([T] `the_reference_changes_nothing_and_the_game_helps_more_the_more_a_bike_is_made_for_it` of `arcade_air.rs`, `the_reference_is_neutral` of `arcade_rough.rs`,
`the_reference_hands_the_grip_back_bit_for_bit` of `rough.rs`; the existing `arcade_tests::the_reference_steering_is_neutral...`).
The probe of a wheel in the air now reaches 8 m instead of 2 m: its `gap` reads the real height up to that, where it read 100 m before. The only reader of
the gap is the mid-air steer's ramp over 4 cm (`wheelie_steer.rs`), which is whole above it either way.

## 6. The sweep and its numbers

`libs/blackbox-modding/src/content/bike_ride_air.rs`, ignored tests on the imported bikes of a folder:

```sh
RIDE6_CARS=<folder> [RIDE6_EVERY=3] [RIDE6_ARCADE=air_before|reference|air=0] \
  cargo test --release -p blackbox-modding --features host --lib bike_jump_sweep -- --ignored --nocapture
```

Every bike at 60, 80, 100 and 120 km/h over a step down of 1, 2, 3 and 5 m, off a 1 m ramp onto a flat road 1 to 5 m below the lip, and off it onto a
15 degree slope: 48 jumps a bike, the falls counted by class, and the first fall of each bike with its kind and its time. **Before** is `RIDE6_ARCADE=air_before`
(this game's arcade steering as it was: no help on jumps), **after** the default. The same files, with `ProfileGround` (`ground/profile.rs`, a height profile
along z with a surface per span), are the library's tests.

| class | falls before / jumps | falls after / jumps | note |
|---|---|---|---|
| Road | **not measured yet** | **not measured yet** | |
| Motard | **not measured yet** | **not measured yet** | |
| Offroad | **not measured yet** | **not measured yet** | |

The numbers are to be filled at the build slot (section 8). What the sweep has to show for the brief to be met: no fall of an offroad or motard bike
from a drop up to 5 m at 100 km/h on a flat or a sloped landing, and a road bike's falls reduced.

## 7. Tests

`bike/air_tests.rs` (a drop of 3 m at 15 and 28 m/s for each class does not fall; the load cap and its smooth knee; the forgiveness window; the spin on
landing; the bars; the levelling and the slope; the reference has none of it), `bike/arcade_air.rs`, `bike/class.rs`, `ground/profile.rs`.

## 8. Not verified, open

- **Nothing here has been built or run.** The mechanisms of section 3 are chosen by reading the code; which of the four causes of section 2 dominates a
  real jump is not measured. At the build slot: run `bike_jump_sweep` with `air=0` first (cause by cause: `RIDE6_ARCADE=air=0`), read `RIDE6_FALLS=1`'s
  lines for the first falls, then tune the constants of section 3 against the table of section 6 and write the result here.
- The scripted jumps use a vertical-ray `ProfileGround` with a 1 cm step face; a real ramp's collision mesh has edges the wheel ray meets at an angle.
- An in-scene check offscreen (a bike off a real ramp of the city) is not done.
