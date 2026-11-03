# The cameras of a bike (clean-room spec)

The game's camera modes are made for a car: the original `camerainfo` views sit at car distances and heights, the custom chase camera
is 5.6 m behind a 1.9 m high roof, the bumper view is on a bumper and the hood view on a bonnet. On a bike they failed: the chase
cameras were two car lengths from a bike one car's width wide, and the hood view was bolted to the body double of the bike, so it rolled
with the lean (the horizon at 45 degrees) and sat at a car's bonnet height. This spec says what every camera mode of the game is on a
bike, where the numbers come from, and what is a modelling choice. Tags: **[D]** read from the reference's data, **[M]** measured
in this port, **[C]** a modelling choice. Cars are untouched: every path below needs a bike with a camera set.

## 1. The reference's data [D]

RIDE 6 keeps the cameras of each bike in a Blueprint container, `resolved/content/assets/data/cameras/Containers/<class>/<Vehicle>.json`
of the physics dump (344 bikes in the classes `Racing` 105, `Sport` 137, `naked` 72, `MaxiEnduro` 15, `motard` 8, `Scooter` 4, `Bagger` 3),
each with the base container of its class (`GameBaseCAmeraContainer_<Class>.json`) and the root one
(`Containers/GameBaseCAmeraContainer.json`). A bike's file lists only what it changes; a number is looked for in the bike's file, the
class's, the root's, in that order (`mods/ride6-bikes/src/camera.rs`, `Container`). Positions are Unreal's: centimetres, x forward, y right,
z up, in the frame of the bike's skeleton; rotations are `Pitch` (positive up), `Yaw`, `Roll` in degrees. The cameras are components
(`IgnitionCameraComponent`: the field of view) with behaviours (`ExternalCameraBehavior`: a camera outside the bike; `InternalCameraBehavior`:
a camera mounted on it):

| Object | Fields read | What it is |
|---|---|---|
| `ChaseNearExternalCameraBehavior`, `ChaseFarExternalCameraBehavior` | `RelativeLocation`, `TargetLocalOffset` | the third-person cameras: where the camera sits and what it looks at, relative to the bike |
| `ThirdPersonNear`, `ThirdPersonFar` | `FieldOfView` | their fields of view (root 85.57; 90 on most bikes) |
| `HelmetInternalCameraBehavior` | `RelativeLocation`, `RelativeRotation.Pitch`, `PitchFactor`, `RollFactor` | the camera in the helmet |
| `Helmet`, `HelmetIgnitionNearPlaneCameraBehavior`, `GameHelmetCameraBehavior` | `FieldOfView` (root 91.49), `OverrideNearPlane` (10 cm), `FollowCameraRoll` and `MaxAngle` (5) | its optics, the near plane, how much of the lean it follows |
| `HelmetCameraModifierBehaviorVariable` | `Slots` of input type `Lean` | how the helmet camera moves and turns as the bike leans: a shift (ahead, inward, down; a downward tilt) from 25 to 70 degrees of lean and a turn (yaw into the bend, roll) from 30 to 70 (section 6) |
| `FrontCamInternalCameraBehavior`, `FrontCamReplay` | `RelativeLocation` (x 63 to 91 cm, z 78 to 110), `RollFactor` 0.8, `PitchFactor` 0.3; `FieldOfView` 104.5 | the camera on the front of the bike |

Per class the near chase is 3.1 to 3.55 m behind the bike's origin (median 3.35) and 0.95 to 1.15 m up (the motard 1.25 to 1.29), the far
one 4.0 m (motard 4.4, up to 4.6 on a few) and 1.15 to 1.45 m up; the helmet camera is 1.10 to 1.35 m up on a Sport or Racing bike, 1.14
to 1.42 on a naked, 1.39 to 1.55 on a maxi enduro, 1.38 to 1.52 on a motard, and 18 cm behind to 10 cm ahead of the origin. The
base also holds the cockpit and gauge cameras, a GoPro on the head bone, a look-back camera, per-camera noise and shake, modifier slots
(the camera moves with the lean, a wheelie, the acceleration, the tuck, the speed) and a speed-dependent pull-back
(`SpeedLocalPosOffset`): only the helmet camera's lean slots are used (section 6, the rest is in section 7).

## 2. The modes on a bike

A bike has a camera set (`[bike.camera]` of its manifest, `docs/modding/bikes.md`; `nfsmw_data::car::mods::bike_camera`) when it is a
mod car of `kind = "bike"`. The setting `camera_mode` (key **C** cycles it, `--camera-mode` picks it) means, on such a bike:

| Mode | On a car | On a bike |
|---|---|---|
| `chase`, `dynamic-fov` | the custom chase camera | the same camera made to the bike's size: its distance, height and look height are the near chase's (section 3) |
| `bumper` | inside the car at its bumper, car not drawn | **the camera ahead of the bike** looking back at it and the road, the bike and its rider drawn (section 4) |
| `hood` | inside the car at its bonnet | **the camera in the rider's helmet**, at his eyes, his helmet and visor not drawn (section 5) |
| `close` | original view 2 | the near chase |
| `far` | original view 3 | the far chase |
| `superfar`, `drift`, `pursuit` | original views 4 to 6 | the far chase scaled as the car's own views are to its far one |

Look back (key **B**): the helmet camera turns to look over the tail, the front camera looks ahead instead of back, the chase cameras as
for a car. The cars' modes are as before.

## 3. The chase cameras [D][C]

`close` and `far` are the original game's two views whose numbers come from the bike: for a chase `(distance d behind the origin, height h,
look point (a ahead, t high), field of view f)` the view has `lag = -d`, `height = h - anchor`, `angle = atan((h - t) / (d + a))` and `fov`
(`viewer/camera/bike.rs`, `views`). The camera's anchor is the bike's centre of gravity (`CarSim::original_camera_body`: the body
double's place plus its centre of gravity, which the bike shares; its height is `[bike.chassis].cg_height_m`), so the camera ends `h` above
the ground whatever the anchor's height. The original
optics read a view's field of view as the horizontal one of a 4:3 screen; RIDE 6's are horizontal at 16:9, so the view holds
`2 atan(tan(f / 2) * (4/3) / (16/9))` (85.57 becomes 69.53) [C]. `superfar`, `drift` and `pursuit` have no RIDE 6 counterpart: their
distance and height are the far chase's times the ratios the car's own views of the same car have to its `far` view, so the order
of the views is the game's [C]. The controller (stiffness, speed lag, tilt, transients) is the car's.

The custom chase camera (`chase`, `dynamic-fov`) keeps its behaviour and the player's distance, height and field of view; a `Fit` scales
them: distance by `near.distance / 5.6`, height by `near.height / 1.9` (the camera's own defaults), and its look point is at the near
chase's `target_height` instead of 1.1 m; the closest an obstacle can bring it is scaled by the distance factor [C]. The speed
widening (5.6 to 7.0 m, 1.9 to 2.2 m) is scaled with it, close to the RIDE 6 near-to-far step (3.3 to 4.0 m).

## 4. The camera ahead of the bike [D][C]

RIDE 6's front camera is bolted to the fairing and looks ahead (`FrontCam*`: x 0.63 to 0.91 m, z 1.1 m, no rotation); the game's ask
is a view from in front of the bike that shows the bike and the road. The mount's height and the optics are the data's (z, 104.5
degrees, `PitchFactor` 0.3, `RollFactor` 0.8); the camera is put `FRONT_AHEAD_OF_HUB_M` = 1.6 m ahead of the front hub, tilted
`FRONT_PITCH_DEG` = -8 degrees (down) and looks back [C] (`mods/ride6-bikes/src/camera.rs`). It is fixed to the bike: at the bike's
roll and pitch the position follows the body, and the camera takes its `RollFactor` and `PitchFactor` shares of them (section 6).
The bike and its rider are in the picture (`frame.rs`, `player_in_picture`; the car modes' `renders_car` would have left the bumper
view empty).

## 5. The camera in the helmet [D][C][M]

- **Place:** the middle of the rider's two eye bones (`eye_l`, `eye_r`), in the pose drawn this frame, so it follows his tuck, his lean
  into a bend, the wheelie pose and the animation (`drive/rider/view.rs`; the names are the table's `eye_bones`, with `head_bone` and
  `head_eye_offset_m` as the fallback). The eye is kept in the bike's model frame and put in the world with the bike's pose of the frame
  the lens is made in (`Mount::place`: section 6 says how the lean enters), so the camera and the bike never differ by a frame of
  travel. Without a rider (a bike with no rider files,
  or the rider off the bike in a fall) it is `fixed_position_m`: RIDE 6's own `RelativeLocation` (x 5 to 10 cm, z 1.2 m on a sport bike).
- **Not drawn:** the meshes named in `hide_meshes` (`helmetRacing_default`, `visorClosedRacing_default`); his suit, gloves and boots
  are drawn, so his arms are in the picture. The near plane is the data's 10 cm.
- **Optics:** RIDE 6's 91.49284 degrees, which is exactly a 60 degree vertical field of view at 16:9 (the evidence that its fields of view are
  the horizontal ones at 16:9) [M]; held as that vertical field of view for any aspect (a wider screen shows more at the sides) [C].
- **Look:** along the bike's heading, tilted by the data's `RelativeRotation.Pitch` (-10 degrees on 312 of the 344 bikes; 23 have none,
  and 5, 7, 15, 20, 50 and 60 occur a few times) [D] and `HELMET_VIEW_DOWN_DEG` = 8 degrees further down [C] (`viewer/camera/bike.rs`,
  `Mount::helmet`). The extra is the player's request ("pointed a little more down so I can see more of the bike"), not RIDE 6's:
  the data's -10 shows the road and the sky with the dash only as a strip at the bottom, and no real value gives more, since the
  `PitchFactor` is the share of the bike's own pitch (a quarter of a wheelie), not a tilt, and the larger tilts are two supermotards'
  whose frame of reference is not in the data (their -50 and -60 are capped to `MAX_HELMET_DOWN_DEG` = 10 [C]: 60 degrees would show
  the front tyre). With the 8 the straight view looks 18 degrees down: the horizon is at about a fifth of the picture's height
  from the top (the Panigale; `C:/r6/shots/cams/j/pan-straight.png`, against about a third before), the fairing, the dash and the
  handlebars fill its lower half and the road and the traffic ahead the middle; on the KTM 450 SMR, whose cockpit sits lower, the
  dash's top, the clocks and the bars' ends show at the bottom (`ktm-straight.png`). It is a
  host constant, not a table value, so it applies to every bike, those already imported too; it is on top of the lean's own
  tilt (section 6.1: the view looks at most 23 degrees down at 60 degrees of lean: at the hard lean of the script
  the horizon is then at the top edge of the picture and tipped, where it was a fifth to a quarter down before; it is still in view at the
  usual lean of a bend, `pan-lean.png`/`ktm-lean.png`). The front camera is not changed.

## 6. Following the bike's motion [D][C]

For a bike with heading `psi`, pitch `p` (nose up positive) and roll `r` (right positive) a mounted camera (`Mount`) looks along
`psi` at the angle `tilt + pitch_factor * p` (the tilt is the data's; looking back turns the heading by pi and the pitch share to
`-p`) and rolls by

`roll_factor * r + clamp((1 - roll_factor) * r, -m, m)`

about the bike's heading, `m` being `follow_roll_max_deg`. The helmet's data is `RollFactor` 0, `PitchFactor` 0.25, `MaxAngle` 5 degrees:
the horizon stays level apart from five degrees of the lean, a quarter of a wheelie's pitch tips the view up. The `RollFactor` 0
(and the front camera's 0.8, the data's) are the reference values; reading `GameHelmetCameraBehavior.MaxAngle` as "the lean
followed beyond the factor, up to this angle" is an interpretation [C]: the data has no description of it, and the other helmet
behaviour in the base (`GameHelmetCameraBehavior1`, 2.5 degrees) is not used. The front camera's roll follows the body 0.8 whole (a
bike leaning 45 degrees tips the picture by 36 degrees).

### 6.1 The helmet camera does not lean with the bike [D][C]

The factors are of the camera's *mount*, not only of its look: a `RollFactor` of 0 means the mount does not roll with the bike, so
the camera's place is carried by the bike's heading and pitch and only the roll it follows (`Mount::place`; `level_position`): at 60
degrees of lean a point 1.4 m up the body is 1.2 m to the side and 0.7 m high if bolted to the body, and 1.4 m up and 12 cm to the
side if carried like this. The first reading, the eye bone bolted to the leaning body, put the lens beside the road at half its height
(a view of tarmac and the fairing); it is what the picture `h-lean` showed before this round. That a camera whose roll factor is 0
is also not put by the body's roll is the reading of the factors [C]: the data has the factors and the 5 degrees, nothing says what
the place does. The place stays the live eye of the rider (his tuck, animation and lean in the bend are in it); it is the leaning
body's roll that is not applied to it. The data's own number for how far the head leaves the vertical is below.

RIDE 6 keeps the lean of the camera in the two `Lean` slots of `HelmetCameraModifierBehaviorVariable` ([D]; per bike, the table
`[bike.camera.helmet.lean]`, `nfsmw_data::car::mods::bike_camera::Lean`, written by the importer from the bike's own slots and else the
root's). A slot maps the lean angle (degrees; the other inputs of the slots in the base are `Velocity`, `Wheelie`, `Acceleration`,
`TurnAngle`, `Stoppie`, `Query`) between its `InputMin` and `InputMax` linearly to a share of its offsets:

| Slot | Lean range | Offsets (Unreal, cm and degrees) | Panigale V4 R | KTM 450 SMR |
|---|---|---|---|---|
| shift | `InputMin` to `InputMax` | `PositionOffset` (x ahead, y right, z up), `RotationOffset.Pitch` | 25 to 70: 30, 15, -35; pitch -5 | 20 to 70: 10, 15, -10; pitch -3 |
| turn | | `RotationOffset` yaw and roll | 30 to 70: yaw 20, roll 12 | 0 to 70: yaw 3, roll 8 |

(the root container's, used by a bike that has none of its own: 25 to 70 with 28, 16, -35 and -5; 30 to 70 with yaw 20 and roll 12.)
The camera is therefore at most 35 cm lower, 16 cm toward the inside of the bend and 28 cm ahead of where it is upright; looks 5
degrees further down and 20 degrees into the bend; and its horizon is rolled by 12 degrees with the lean on top of the followed 5.
What the data does not say, and is chosen [C]: the offsets are in the heading's frame (the slots' `ESlotRotationType::WorldRotation` has no
description); the inside is the side of the lean (`AllowHorizontalFlip` is on in every slot); the yaw turns toward the inside, the roll
is in the sense of the lean; the lean is the bike's roll angle (`CurrentLeanAnglePercentage` of the tuck slot is a fraction of its
maximum, so the `Lean` input is in degrees; the slots' ranges stop at 70); the filters of the slots (`ErrorHalvingTime` 0.3 s for the
shift, 0.1 s for the turn) are not applied, the bike's roll is already smooth; looking back has the shift but not the turn. A
camera whose eye is the live rider adds the shift to the eye. The shift is the data's number for a fixed point; the stock rider's own
hang-off in a bend is also in his eye bones, so the two may count some of the same head movement [C]: not measured, left open.

Offscreen pictures of the Panigale V4 R (`C:/r6/shots/cams/h/pan-*.png`): straight, the picture is the cockpit as before; at about 35
degrees the road ahead and the traffic are in view with the horizon level; at the hard lean of the script the horizon is in the top
third, tipped by about 10 degrees, and the road ahead fills the rest, where the bolted camera showed the tarmac beside the bike and
its fairing. A KTM 450 SMR leaned with a stick of 0.55 and 0.75 shows the same (`ktm-s55`, `ktm-s75`); at a full stick (1.0) it
falls at that speed and the camera is the fixed one of the fallen bike (section 7).

The bounds that follow (tests, lean 0 to 60 degrees either side): the look is between 23 degrees down (10 of the data, 8 of
section 5, 5 of the lean) and the horizon, turned at
most 20 degrees from the heading, the horizon is tipped at most 17 degrees (5 followed and 12), and the lens is at most 35 cm below,
16 + 12 cm sideways and 28 cm ahead of its upright place.

### 6.2 Looking around with the right stick [D][C]

RIDE 6's helmet camera carries a side-view behaviour (`HelmetIgnitionSideViewCameraBehavior`, in the root container, the same for
every bike): an axis `LookRight` that turns the camera by up to **11 degrees** either way (`YawModifier.OffsetLimits` -11 to 11), an
axis `LookUp` by up to **10 degrees** (`PitchModifier`), and a `RotatorFilterSpeed` of **4** (5 in one container, the BMW M 1000
RR 2023's) that smooths the camera's turn to where the axis asks [D]. The cockpit camera has 13 and 10, `RotateSpringArm` is off
everywhere. This port's player reads the two axes from `head_look_x` and `head_look_y` (`input/action.rs`) and the rider's head
follows them (`viewer/camera/head_look.rs`, `HeadLook`):

- the look aimed at is the axis's share of the limits, linear, 11 degrees of yaw and 10 of pitch at the end of the axis [D];
- the look moves to it, and back to straight ahead when the axis is let go, by an exponential approach at the filter speed: it
  closes the distance by `1 - exp(-4 dt)` a frame, a time constant of a quarter of a second whatever the frame rate; released,
  it is back within a hundredth of a degree in two seconds, and does not overshoot [C]. The data names the speed and not the
  filter; this is the usual reading of it (Unreal's `RInterpTo`) in its frame-rate independent form;
- the look is applied to the camera's direction on top of the tilt, the lean's turn and the look-back (a stick to the right turns
  the view clockwise from above, whichever way it faces); the eye, the field of view, the near plane and the horizon's roll are
  untouched, and the rider's own body is not moved by it (his head stays as the animation has it, his helmet is not drawn).

**What drives it** (`input/bindings.rs`, `driving`; every device feeds the same two axes, which add up and are held to -1..1):

| Device | `head_look_x` (right +) | `head_look_y` (up +) |
|---|---|---|
| Pad | right stick X (camera dead zone and sensitivity of the controls settings; 15 percent by default, so a resting stick does nothing) | none, see below |
| Mouse | horizontal speed | vertical speed |
| Keyboard | Numpad 6 / Numpad 4, or `]` / `[` for a keyboard without a numpad | Numpad 9 / Numpad 3 |

- **The pad's Y is not a look.** The right stick's Y is the gear shift in the driving profile (`ShiftUp`, `ShiftDown`), also under
  the automatic gearbox, where the shift buttons are sport shifts, so pitching the head with it would shift gears; every other
  pad button and trigger is bound, and there is no modifier mechanism in the bindings (a binding is one source and a scale). So
  `head_look_y` has no default pad binding, and the data's pitch is reached with the mouse or the keys. A player who prefers the
  stick can unbind the shift (`shift_up = []`, `shift_down = []`, or keys only) and bind `head_look_y = ['axis:RightStickY']`
  in `[bindings]` (the camera dead zone and sensitivity apply to it as to the X axis).
- **The mouse is a stick whose deflection is the speed of the mouse** [C: a modelling choice; the original has no mouse look, and
  RIDE 6's side-view axes are stick axes]. A binding reads the pixels moved in the frame divided by the frame's time, times the
  scale `1/700` (`MOUSE_HEAD_LOOK_PIXELS`, the 700 pixels a second a fully pushed stick turns the free camera by; negative for the
  vertical axis, since the mouse's y grows downward), times the mouse sensitivity setting. A mouse sweeping at 700 pixels a second
  therefore holds the stick at its end, 350 pixels a second at half, a mouse at rest is a stick at rest. The head's own filter
  (above) does the rest: it follows the speed and **springs back to straight ahead within two seconds of the mouse stopping**,
  like a spring-loaded head, with no state kept in the input layer (a head that stayed where the mouse put it, accumulated, would
  need a centre key). The angle a nudge turns the head by does not depend on the
  frame rate: the stick is the speed, the filter integrates it (a 2 pixel nudge is about 0.12 degrees at any rate from 30 to 240
  frames a second); a flick of 300 pixels in a quarter of a second turns it about 7 degrees. It has the same gate as the other
  mouse looks: the cursor captured (a click in the window), or the right button held.
- **The keys** push the stick to its end while they are held (11 degrees of yaw, 10 of pitch) and release it, so the head returns.
  The numpad's 8 and 2 are the rider's weight, so the look takes 4 and 6 beside them and 9 and 3; no other driving action is on
  these keys.
- In the free camera (`view-world`, F) the two axes are not read, so the mouse look that flies the free camera does not also turn
  the rider's head.

Only the helmet camera (`--camera-mode hood`) reads the look: a car, the front camera, the chase cameras and the original views
do not, and the physics never sees it (`input/head_look_tests.rs`, `viewer/camera/bike_tests.rs`). `look` and `lookup` of a
`--drive-script` (`docs/modding/drive-script.md`) hold the two axes for a segment, for pictures.

## 7. What is not done

- The camera noise and shake of RIDE 6 (surface noise, kerbs, grass), the other modifier slots (the wheelie, the acceleration, the
  speed, the brake, the tuck: the live rider's eye already holds his tuck and lean), the speed pull-back and `WorldHeightOffset`:
  the game's own camera shake and noise apply (a mounted camera has none), the controller of the original views has its own speed
  lag. Only the helmet camera's two `Lean` slots are used (section 6.1), and their filters are not.
- The cockpit camera (x -4 cm, z 1.2 m, pitch -20), the gauge camera and the GoPro have no mode of the game to sit in; an extra view of
  a mod could use them.
- The helmet look's pitch has no default pad binding (section 6.2: the mouse and the keys pitch the head); the mouse and key
  bindings are this port's choice, the original has none; the front camera has no look (RIDE 6 has
  no side-view behaviour for it); the cockpit camera's 13 and 10 degrees are for a camera that no mode of the game shows.
- A bike without a container (16 of 293) has no `[bike.camera]`, and the game derives its cameras from its wheelbase
  (`BikeCamera::derived`: distances and heights as fractions of the wheelbase [C]). The dump and the containers name 47 bikes with another
  model year; the importer takes the container of the same name nearest in time, which found 31 of them.
- While the rider is thrown the helmet camera is at the fixed place of the bike lying down; it does not ride the ragdoll.

## 8. Tests and pictures

- `viewer/camera/bike_tests.rs`: the chase views and the fit scale with the wheelbase; `close` < `far` < `superfar`; heights above the
  ground; the 16:9 to 4:3 field-of-view conversion; attitude from a rotation; the lean follow (5 degrees at most for the helmet, 0.8 for the
  front); a wheelie's quarter; the front camera is ahead of the front tyre and looks at the bike; roll direction; the helmet camera at
  a lean (a bolted camera is carried by the body and a helmet one by the followed roll only; the lean shift goes inward, ahead and down;
  from 0 to 60 degrees of lean on either side the look, turn, roll and place stay within the bounds of section 6.1; the look into the
  bend flips with the side; looking back is not turned).
- `viewer/camera/head_look_tests.rs`: rest, the limits (11 and 10), half a stick, the spring back within two seconds at every
  frame rate, no overshoot, the quarter-second time constant, a frame of no time and a stick that is not a number. `bike_tests.rs`:
  the helmet camera looks `HELMET_VIEW_DOWN_DEG` further down than the data and keeps its horizon in the top 30 percent; the head
  turns the view clockwise to the right and up to the top, moves nothing else, does nothing at rest, also looking back.
  `input/head_look_tests.rs`: the right stick's X is the look and its Y still shifts, the camera dead zone, the names.
  `drive/script.rs`: `look` and `lookup`.
- `drive/rider/view.rs`: the eye is the middle of the eye bones, in the model's frame; the head bone is the fallback; none off the bike;
  the helmet and visor are the hidden parts.
- `modding/content/bike_camera.rs`, `libs/blackbox-modding` (`bike_camera.rs`), `nfsmw-data` (`mods/bike_camera.rs`): the table's schema,
  ranges, defaults, derived cameras, the lean table (partial tables, ramps, the root default).
- `mods/ride6-bikes/src/camera_tests.rs`: the layers of the container, units and frames, the schema round trip, replacing a table, the
  year match, the cap, the lean slots read into the lean table (and zeros for a bike with none).
- Offscreen pictures of every mode on the KTM 450 SMR and the Ducati Panigale V4 R, leaned and straight: `C:\r6\shots\cams\`
  (`nfsmw view-world --drive KTM450 --muted --camera-mode <mode> --drive-script "..." --screenshot ...`).
