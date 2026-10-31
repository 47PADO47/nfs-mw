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
| `FrontCamInternalCameraBehavior`, `FrontCamReplay` | `RelativeLocation` (x 63 to 91 cm, z 78 to 110), `RollFactor` 0.8, `PitchFactor` 0.3; `FieldOfView` 104.5 | the camera on the front of the bike |

Per class the near chase is 3.1 to 3.55 m behind the bike's origin (median 3.35) and 0.95 to 1.15 m up (the motard 1.25 to 1.29), the far
one 4.0 m (motard 4.4, up to 4.6 on a few) and 1.15 to 1.45 m up; the helmet camera is 1.10 to 1.35 m up on a Sport or Racing bike, 1.14
to 1.42 on a naked, 1.39 to 1.55 on a maxi enduro, 1.38 to 1.52 on a motard, and 18 cm behind to 10 cm ahead of the origin. The
base also holds the cockpit and gauge cameras, a GoPro on the head bone, a look-back camera, per-camera noise and shake, modifier slots
(the camera moves with the lean, a wheelie, the acceleration) and a speed-dependent pull-back (`SpeedLocalPosOffset`): none of them is used
(section 7).

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
  the lens is made in, so the camera and the bike never differ by a frame of travel. Without a rider (a bike with no rider files,
  or the rider off the bike in a fall) it is `fixed_position_m`: RIDE 6's own `RelativeLocation` (x 5 to 10 cm, z 1.2 m on a sport bike).
- **Not drawn:** the meshes named in `hide_meshes` (`helmetRacing_default`, `visorClosedRacing_default`); his suit, gloves and boots
  are drawn, so his arms are in the picture. The near plane is the data's 10 cm.
- **Optics:** RIDE 6's 91.49284 degrees, which is exactly a 60 degree vertical field of view at 16:9 (the evidence that its fields of view are
  the horizontal ones at 16:9) [M]; held as that vertical field of view for any aspect (a wider screen shows more at the sides) [C].
- **Look:** along the bike's heading, tilted by the data's `RelativeRotation.Pitch` (-10 degrees on most bikes) [D]. A supermotard
  class's -50 and -60 are capped to `MAX_HELMET_DOWN_DEG` = 10 [C]: its frame of reference is not in the data and 60 degrees would show
  the front tyre.

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

## 7. What is not done

- The camera noise and shake of RIDE 6 (surface noise, kerbs, grass), the lean and wheelie modifier slots, the acceleration slot, the
  speed pull-back and `WorldHeightOffset`: the game's own camera shake and noise apply (a mounted camera has none), the controller of the
  original views has its own speed lag.
- The cockpit camera (x -4 cm, z 1.2 m, pitch -20), the gauge camera and the GoPro have no mode of the game to sit in; an extra view of
  a mod could use them.
- The mouse look of the helmet camera (`IgnitionSideViewCameraBehavior`: +-11 degrees yaw, +-10 pitch).
- A bike without a container (16 of 293) has no `[bike.camera]`, and the game derives its cameras from its wheelbase
  (`BikeCamera::derived`: distances and heights as fractions of the wheelbase [C]). The dump and the containers name 47 bikes with another
  model year; the importer takes the container of the same name nearest in time, which found 31 of them.
- While the rider is thrown the helmet camera is at the fixed place of the bike lying down; it does not ride the ragdoll.

## 8. Tests and pictures

- `viewer/camera/bike_tests.rs`: the chase views and the fit scale with the wheelbase; `close` < `far` < `superfar`; heights above the
  ground; the 16:9 to 4:3 field-of-view conversion; attitude from a rotation; the lean follow (5 degrees at most for the helmet, 0.8 for the
  front); a wheelie's quarter; the front camera is ahead of the front tyre and looks at the bike; roll direction.
- `drive/rider/view.rs`: the eye is the middle of the eye bones, in the model's frame; the head bone is the fallback; none off the bike;
  the helmet and visor are the hidden parts.
- `modding/content/bike_camera.rs`, `libs/blackbox-modding` (`bike_camera.rs`), `nfsmw-data` (`mods/bike_camera.rs`): the table's schema,
  ranges, defaults, derived cameras.
- `mods/ride6-bikes/src/camera_tests.rs`: the layers of the container, units and frames, the schema round trip, replacing a table, the
  year match, the cap.
- Offscreen pictures of every mode on the KTM 450 SMR and the Ducati Panigale V4 R, leaned and straight: `C:\r6\shots\cams\`
  (`nfsmw view-world --drive KTM450 --muted --camera-mode <mode> --drive-script "..." --screenshot ...`).
