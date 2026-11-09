# RIDE 6 bikes: steering with the front wheel up (clean-room spec)

Report of a player: "when wheeling I can't go left or right (W + A on the keyboard, down + left on the pad) even though I should, because the weight gets shifted".
Reproduced on the 288 imported bikes, traced in the model and in the RIDE 6 executable (build `5.6.1-1697004`, image-relative addresses, written in our own
words; no decompiled text is kept in the repository). Continues [ride6-feel-round11.md](ride6-feel-round11.md) (section 1: the lean help of a bike on its rear wheel),
[ride6-feel-round9.md](ride6-feel-round9.md) (sections 1, 3 and 6), [ride6-weight-round9.md](ride6-weight-round9.md) and [ride6-tricks.md](ride6-tricks.md). Tags: **[B]**
read from the binary, **[D]** data, **[M]** measured here, **[I]** inferred, **[C]** a choice of this project's model. Provenance:
[ride6-physics-B.md](../provenance/ride6-physics-B.md), "Wheelie steer".

## 1. Reproduction [M]

`bike_wheelie_steer` (`libs/blackbox-modding/src/content/bike_ride_wheelie_steer.rs`, ignored, `RIDE6_CARS`): the bike is ridden from a speed with the throttle
open and the weight lever at -1 (what V or the pad's stick pulled back give) until its pitch passes 20 degrees; the wheelie assist then holds it at 35 degrees or a
little more (mean pitch 36 to 38 degrees below). The stick is then held at 0.5 or 1.0 for 3 s, as the game gives it (round 13 removed the share by speed),
and the heading it turned through is compared with the same stick on the road with the front wheel down at the same speed (the throttle holding it). Means over the
288 bikes (the road column is the same before and after):

| start speed, stick | heading in the wheelie before | in the wheelie after | on the road |
|---|---|---|---|
| 8 m/s, 0.5 | 4.5 deg | 27.7 deg | 68.4 deg |
| 8 m/s, 1.0 | 9.4 deg | 54.8 deg | 146.1 deg |
| 15 m/s, 0.5 | 3.2 deg | 20.2 deg | 39.5 deg |
| 15 m/s, 1.0 | 6.5 deg | 37.2 deg | 91.1 deg |
| 25 m/s, 0.5 | 2.3 deg | 15.2 deg | 24.3 deg |
| 25 m/s, 1.0 | 4.7 deg | 26.2 deg | 56.4 deg |

The bike did not fall in any of these rides, before or after. A Yamaha R1 (2008) at 8 m/s with the stick at 1.0 (`RIDE6_VERBOSE=1`): the lean runs to 0.29 rad in
the first second while the pitch rises, then falls back to 0.13 rad as the pitch passes 0.7 rad, because the lean limit falls from 0.5 to 0.12 rad over the same
time; the yaw rate peaks at 0.2 rad/s and is 0.04 rad/s at the end. Over 3 s that is 10 degrees.

## 2. The chain from the keys and the stick to the bike [M]

Nothing in the chain removes the steering. Checked, and pinned by tests:

- **Keyboard.** W is the throttle, V (or Numpad 2) the weight lever at -1, A and D the steering at -1 and +1: three separate actions (`input/bindings.rs`). `DriveInput`
  carries all three (`throttle` 1, `weight` -1, `steer` -1 or 1) and `sim/bike.rs` `command()` takes `throttle` and `steer` only; the weight goes to `Bike::set_weight` through
  `StepStart::weight`. `command()` gives the stick at any speed as it is (the share by speed of rounds 9 to 12 was 1.0 since round 11 and went in round 13). The weight does not touch the stick.
- **Pad.** Left stick Y is the lever and X the steering. The lever opens the stick's circular gate to a square for Y only (`input/weight_axis.rs`: a stick at the rim on the
  diagonal reads Y = 1); X is not rescaled, so down-left on the rim gives a lever of -1 and a steering of about -0.7 (the stick's own X; the test in `input/rider_weight_tests.rs`
  pins 0.5 to 0.8). No axis has priority, no dead zone swallows the other, and X is not shrunk when Y is large. The steering of 0.7 is a physical limit of a round gate, not a bug.
- **Tests:** `drive/sim/bike_ride_tests.rs`, `a_wheelie_held_with_v_and_w_turns_with_a_and_d_through_the_input_layer` (real keys through `ActionState`, `DriveInput`,
  `StepStart`, `BikeRide::respond`) and `a_wheelie_held_with_the_stick_down_and_to_the_left_turns_left` (the pad's values).

So the bike really did get the stick, and the model turned it into 3 to 10 degrees of heading in 3 s. The cause is in the dynamics (section 4).

## 3. What the reference does with the front wheel up

### 3.1 The lean limit is cut with the wheelie angle [B, D]

At the end of the lean limit (`0x14510b180`) the limit is passed through `0x1450cd720` twice: with the wheelie tuning (handler `+0xa50`, the front wheel) and with the
stoppie tuning (`+0xa28`, the rear wheel). The function returns the limit unchanged when the tuning's `LeanLimit` is 0 or less; otherwise `lerp(limit, LeanLimit, ramp(angle;
AngleRange))`, with `angle = max(0, (CurrentWheelieAngle - terrain term) x sign)` in degrees while the wheel's contact flag is clear. The result is floored at 0.1 degree
(`148fbaccc`). The data: `LeanLimit` 1 and `AngleRange` 0 to 40 degrees (the Motard handler) or 0 to 45 (270 of the 288 manifests), for the wheelie, and 0 to 20 or 0 to 7 degrees for the stoppie.
**The reference keeps a wheelie straight on purpose**: at 35 degrees of pitch a bike whose lean limit is 46 degrees on the road may lean 11 degrees, at 45 degrees 1 degree.
The model already had this (`pitch_cut` in `handling.rs`, from the manifest's `lean_limit_rad`, 0.0175 rad in all 288).

### 3.2 The lean help is whole in a wheelie [B]

The lean help factor (`0x145107c80`) takes the front wheel's gap (`wheel +0x28` less `+0xc0`, at least 0): over the end of the handler's wheelie range (`+0x28b0`, a pair of zeroes in
every bike: no data sets it) the whole factor is `LeanHelpFactorWheelie` (`+0x26e4`), 1 by the constructor (`0x50fb230`, `0x50fbfa0`), no bike sets it. So **with the front wheel up the help
factor is 1** whatever the speed curve says (0.16 for the R1 at speed). The model used the speed curve. [B] Changed (section 5); the effect on the lean is small, because the controller's
loop closes with any share (the lean followed its target within 0.02 rad with 0.16 and with 1).

The stability help has the same blend (`0x14510d620`, `StabilityHelpFactorWheelie` at `+0x2e4c`) and its constructor value is 0. With the unset range the blend weight is 1 whenever the gap is 0 or
more, so, as read, the factor is that 0 in every state, on the road too. Not changed here (open, section 7).

### 3.3 The mid-air steer [B, D]

The handler's `MidAirSteerComponent` is the same in all eight input handlers. `0x1450f7680` works out its weight, the product of three ramps:

| term | data | meaning |
|---|---|---|
| the front wheel's gap | 4.0 to 4.1 (cm) | the front tyre must be at least 4 cm over the road |
| `1 - ramp(abs(normalised lean))` | 0.4 to 0.41 | the lean must be under 40 percent of its limit |
| the pitch speed | -0.4 to -0.3 (rad/s) | the nose must not be dropping faster than 0.4 rad/s (sign of the speed [I]) |

and, when the weight is over 0, the angle `clamp(SteerInputTuning(stick) + LateralWeightInputTuning(lateral weight), -1, 1) x steer lock x pi/180`, with `SteerInputTuning` mapping
the stick -1 to 1 onto 0.8 to -0.8 and the lateral-weight tuning unset. `0x1450c8f70` then takes **the handlebar angle = lerp(the controller's angle, this angle, the weight)**. So with the front wheel
up and the lean small the stick sets the bar directly to 0.8 of its lock, with the sign inverted: with the handlebar positive to the right (as in `lean::handlebar_angle`) and the stick positive to
the right this is a counter-steer, which rolls a bike towards the stick through the gyro of its spinning front wheel [I: the polarity of the two was not traced]. When the lean passes 40 percent of
its limit the weight goes to 0 and the controller has the bar again.

### 3.4 What is not there [B, D]

No term turns a wheelie: the lean help in the air is a couple about the centre of gravity with no net force (round 11); the stability help's wheelie factor is 0; the mid-air steer sets a bar that touches
nothing; the data's lean limit is 1 degree. The rear tyre is left, and in the data it hardly makes a lateral force from the lean (next section).

## 4. Why the model's wheelie hardly turned [M]

The rear tyre of the R1 at 2.6 kN (`bike_rear_camber_thrust`): at zero slip it makes -19 N at 10 degrees of camber, -37 N at 25, -40 N at 40, -34 N at 50 (the sign of the model's lateral axis), and 1285 N for one
degree of slip. The tyre of the data has almost no camber thrust; its force comes from the slip angle. On the road the front tyre carries the turn (1800 N wanted at a lean of 0.87 against about 200 N at the rear)
and its force acts ahead of the centre of gravity, which yaws the bike. With the front wheel up the lean help is a pure couple (no net force), so the only force that can bend the path is the rear tyre's, and it
needs a slip angle, which needs a yaw rate first: the heading follows the lean only through a slow second-order loop. At a lean of 0.9 rad (no cut at all) the model turns 35 degrees in 3 s at 8 m/s, a quarter of the
road's 148 degrees; with the data's cut (a lean of 0.12 to 0.29 rad) 10 degrees. The reference has the same structure and the same data; whether its wheelie turns more cannot be settled without the game.

## 5. Changes

**Data and recovered [B, D]** (`libs/blackbox-vehicle/src/bike/wheelie_steer.rs`, `handling.rs`):

1. `mid_air_intensity` and `mid_air_angle` (section 3.3), used in `steer_handlebar`: the handlebar aim is `lerp(aim, mid_air_angle(stick), weight)`. The front wheel's height over the road is new in the model
   (`WheelState::gap`, metres, 0 on the ground; `wheel::air_gap`, set in `step_wheel` for a wheel in the air), the lean over the limit is `lean / plan.limit`, the pitch speed is `-omega . right`.
2. The lean help factor is `LEAN_HELP_FACTOR_WHEELIE` (1) while the front wheel is off the road (`apply_lean_help`).

**Choices [C]** (`choices.rs`), because the reference's own mechanism leaves a high wheelie almost straight and the user asks for a steerable one:

3. `WHEELIE_LEAN_FLOOR` = 0.35 rad (20 degrees): the data's cut of the lean limit stops there (`steer_handlebar`, the wheelie side only; the stoppie's cut is the data's). Set it to 0 to get the data's cut.
4. `WHEELIE_YAW_FACTOR` = 1 and `WHEELIE_YAW_PITCH_RANGE` = 0.15 to 0.35 rad: with the front wheel up and the rear wheel on the road, a yaw torque about the road's normal pulls the yaw rate to `g tan(lean) / v` (the stability help's own
   target, held to its `max_lean`, with the speed at least the top of its speed range and the torque faded in over that range, 2.8 to 4.2 m/s), the share of the gap closed per sub-step
   being the factor, through the body's inertia (`Bike::apply_wheelie_yaw`). The reference has a factor for this state, `StabilityHelpFactorWheelie`, and leaves it at 0. Set it to 0 to switch the torque off.

Why two (Yamaha R1 2008, 8 m/s, full stick, 3 s, heading change): the data's cut alone 10 degrees; a floor of 0.3 rad alone 19.6; the torque alone with the data's cut (a tenth of the gap per sub-step) 15.9; the
floor of 0.3 with that torque 30.7 and with the whole gap 41.5. Over the 288 bikes with the choices as shipped a full stick turns the wheelie through 37 percent of what the road does at 8 m/s, 41 percent at
15 and 46 percent at 25, and the lean stays the cause of the turn (a bike with no stick goes straight: under 3 degrees in the tests). The torque acts about the road's up and cannot roll the bike; it
ends with the front wheel's touchdown.

**The torque only comes in with the height of the wheelie** (`WHEELIE_YAW_PITCH_RANGE`, 0.15 to 0.35 rad of the lift angle). The first version took the front wheel being off the road as the condition and
`bike_flick` (round 11, section 1) then threw 4 bikes of 288 (KTM 690 SMC R, Ducati Hypermotard, TM SMX, KTM 450) in 14 of its 36 configurations: a throttle opened with the stick flicked makes the front wheel hover a few
centimetres over the road (a lift of 0.02 rad) and a stiff yaw torque on a bike that is on its rear wheel by that much is a bar on the rear tyre. With the fade none of them falls.

## 6. Results [M]

`bike_wheelie_steer`, all 288 bikes, pitch 20 degrees reached, 3 s (section 1): 0 falls in all six configurations before and after; 287 or 288 of 288 reach the pitch.
Regressions on the 288 bikes with the change as shipped (fall counts):

| sweep | configurations | falls |
|---|---|---|
| `bike_wheelie_landing` | 20 and 35 degrees, stick 0, 0.1, 0.3, from 12 and 20 m/s; the same with the stick held through the wheelie | 0 of 288 in all 24 |
| `bike_burnout` | pedal, front lever, light pedal, stick over to either side and half over, half throttle | 0 fell, 0 flipped |
| `bike_square_stick` | 18 configurations (0.5, 1, 2 Hz, +-0.5 and +-1, at 20, 30, 40 m/s) | 0 of 288 in all 18 |
| `bike_curb` | 90 configurations | 0 of 288 in all 90 |
| `bike_flick` | 36 configurations; and with the thumb's arc (`RIDE6_WEIGHT=arc`) 0, 10, 20 m/s, rise 0.1 and 0.3 | 0 of 288 in all 36 and in all 12 (the first version of the yaw torque: 4 bikes in 14 configurations, section 5) |
| `bike_wheelie_landing`, stick 0.6 and 1.0 held through the wheelie, from 12 m/s | 20 and 35 degrees | 0 at 0.6; 1 of 288 at 1.0, the Italjet Dragster 700, whose fall limit equals its lean limit (round 9, section 3, recorded 127 to 256 of 288 falling with a full stick held through a wheelie, before the round 11 couple) |

Unit tests: `bike/wheelie_steer_tests.rs` (the gates of the mid-air steer, its handlebar, the coordinated yaw rate, the height of the front wheel, the lean help in a wheelie, and a wheelie that turns to either
side by at least 0.26 rad per unit of stick in 3 s and stays up: the same test with the two choices at 0 reads 0.045 rad and fails), `drive/sim/bike_ride_tests.rs` (section 2).

## 7. What stays open

- **Whether the reference's wheelie turns more than the data's lean limit and the tyre say.** The two choices of section 5 are this project's, not the reference's; they are named and switchable.
- **The mid-air steer's polarity and the pitch speed's sign** (section 3.3). Read as a counter-steer; the bar of a wheelie shows it. With no gyro of the steering rate in the model the bar has no physical
  effect while the front wheel is up (the term would be `I_wheel x spin x steering rate`, about 30 N m for 0.3 s at 15 m/s: a lean of a few hundredths of a radian); only the touchdown sees it.
- **The stability help factor** reads 0 in every state if the blend of `0x14510d620` is as read (section 3.2); the model keeps the speed-curve factor of rounds 6 and 7 (a small help: 0.05 up to 80 km/h, off under the throttle).
- **The lean help's fade with the front wheel's ground distance and the rear-up case** (round 11, section 3), and the mid-air steer for a bike with both wheels off the road, which the model gives the same bar: `mid_air_weight` works from the front gap alone.
- **In the game** it was run offscreen for the first time in section 9; what the frames showed is there.

## 8. Tools

`bike_ride_wheelie_steer.rs`: `bike_wheelie_steer` (`RIDE6_SPEEDS`, `RIDE6_PITCHES`, `RIDE6_STICKS`, `RIDE6_SECONDS`, `RIDE6_LEVER`, `RIDE6_CUT=<rad>` to replace the data's lean limit in the
wheelie, `RIDE6_EACH=1` for a line per bike, `RIDE6_VERBOSE=1`), `bike_rear_camber_thrust`; the sweeps of rounds 8 to 11 for the regressions.

`bike_ride_wheelie_script.rs`: `bike_wheelie_script` rides a `--drive-script` from a standstill (`RIDE6_SEGMENTS="seconds,throttle,stick,lever;..."`, `RIDE6_EVERY`) and prints the speed, pitch, lean and its limit, heading and bar angle, and when the rider let go.

## 9. Seen on screen [M]

The script key `weight=` (the rider weight lever, -1 back to 1 forward; [the script's page](../modding/drive-script.md)) makes a wheelie scriptable. KTM 450 and Ducati Panigale V4 R (2019), `--drive-script "2:throttle=1,weight=-1;1:throttle=1,weight=-1;N:..."`:
a standing start with the throttle open and the lever back, 3 s straight (the front wheel is up from 1.0 s), then `steer=1` for 2 s, then `steer=-1` for 2 s; one run is one frame, ending at 3, 4, 5, 6 and 7 s
(`--camera-mode far` and `hood`; frames in `C:/r6/shots/wheelie`, `w_*` far, `h_*` helmet). The same script in `bike_wheelie_script` gives the numbers below (KTM 450; the Panigale is alike); the game's speedometer reads 42, 58, 63 and 72 km/h at 3, 5, 6 and 7 s, a little above the model's 35, 52, 60 and 68.

| t (s) | stick | speed | pitch | lean / limit | heading |
|---|---|---|---|---|---|
| 3.0 | 0 | 9.8 m/s | 40 deg | 0 / 0.38 rad | 0 |
| 5.0 | +1 | 14.4 | 40 | 0.39 / 0.39 | 30 deg right |
| 6.0 | -1 | 16.7 | 38 | 0.02 / 0.41 | 28 |
| 7.0 | -1 | 19.0 | 38 | -0.26 / 0.40 | 16 |

What the frames show:

- **The front wheel is up** from 1 s and stays up through both turns (pitch 37 to 41 degrees, the wheelie assist's hold); from behind the bike shows its rear tyre only, its shadow is pitched; in the helmet view the bars and the fender are high and the road is seen under the bike.
- **The heading follows the stick**, to the right for +1 and back for -1, the same size either way (about 15 degrees per second of full stick at 10 to 14 m/s); the scenery turns in the far and the helmet views.
  The lean rises to the limit (0.38 rad, the floor `WHEELIE_LEAN_FLOOR` of 0.35 plus the data's cut) and no further.
- **The bar angle** reads as a counter-steer (section 3.3): with the stick to the right the bar goes to -0.5 rad at the start (the mid-air steer's 0.8 of the lock), then falls to -0.15 rad as the lean passes 40 percent of its limit and the controller takes the bar back; with the stick to the left it goes to +0.55 rad. From behind the bar is seen end on, rolled with the bike; in the helmet view the fork and the wheel turn with it (wheel and bars stay one piece).
- **The rider sits right**: on the seat, hands on the grips, the body leaning with the bike; no detaching, no flipping, no glitch of the pose over the 7 s on either bike.
- **A fall at 7.5 to 8 s** in the first, longer script (4.5 s straight, 2 s each way) was not the model: the bike had run 80 m up the road into the toll plaza (a booth and barrels next to it) and the collision threw the rider; the harness with the same inputs does not fall. The frames above stop at 7 s.
- **The helmet view in a leaning wheelie** shows the bike pushed to one side of the view (the front wheel at the right edge for +1, the bars at the left for -1) while the lean is 0.3 to 0.4 rad. That comes from how the helmet camera is carried with the lean
  ([ride6-bike-cameras](ride6-bike-cameras.md)); it was not changed here (that page is being edited elsewhere) and is a question for the camera, not the wheelie.

Nothing needed fixing from these frames. The front wheel and the handlebar angle of the far view are hard to judge (the front wheel is behind the rider); the helmet view is the clear one.

## 10. Round 2: the stick lowers the front only a little (the game's addition) [C]

Playtest: "if I'm wheeling and go right/left the bike should be brought down only a little bit, not all the way." The pitch assist of the data fades out with the lean (`PitchParams::lean_help`, 0.2 to 1 of the lean over
its limit), so a wheelie with the stick over loses its assist; the throttle's wheelie of the game then fell to the road at a full stick. `ArcadeSteering::wheelie_steer_drop` (7 degrees in this game, none in `REFERENCE`) puts a floor
under a steered wheelie: the assist lifts a nose that falls more than that under the pitch the wheelie had before the lean, and never lowers it. About 10 degrees are lost at a full stick, the wheelie stays up. See
[ride6-arcade-steering.md](ride6-arcade-steering.md), section 12 (the measurements, the limits and the tests).
