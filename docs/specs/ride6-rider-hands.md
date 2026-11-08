# RIDE 6 rider hands: the throttle, the clutch and the brake levers

How RIDE 6's rider works the controls with his hands, as far as the animation blueprint and the clips say it, and how this project plays
it: the right wrist rolls the throttle, the left fingers pull the clutch lever, the right fingers pull the front brake lever. Sources:
the animation blueprint of the user's install (`RiderGP_racing_Master_animBlueprint`) as pseudo-code printed by CUE4Parse's Blueprint
decompiler (functions with `ReadScriptData`), and the finger animation sequences of `riderShared/Animations/race/`. Provenance:
[ride6-rider-hands](../provenance/ride6-rider-hands.md). Where the hands sit on the grips: [ride6-rider-animation.md](ride6-rider-animation.md),
section 6, and [ride6-rider-render-integration.md](ride6-rider-render-integration.md), section 8.

## 1. The finger layers

`Hands_FingersOverride` lays a short clip over the riding pose on the finger bones of one hand (`LayeredBoneBlend` nodes whose branch
filters start at `index_01`, `middle_01`, `ring_01`, `pinky_01`: the fingers, not the wrist), the clip stepped by an explicit time [D]:

| Layer | Clip | Hand | Explicit time | Weight |
|---|---|---|---|---|
| Clutch | `race_clutch_brake_gear` | left | `Fingers_Clutch_Explicit`: the clutch lever followed at `FingersClutchExplicitSpeed` (2.5 a second; 10 on a scooter), the fully pulled lever being 0.27 s (0.24 for the woman) of the clip | `Fingers_Clutch_Alpha` = `SmoothFloat_Constant(explicit > 0, 7)` |
| Front brake | `race_clutch_brake_gear_{OneFinger,TwoFingers,FourFingers}` by `BrakeFingers` (1, 2, 4; the blueprint's default and every `RIDING_STYLE` preset: 2) | right | `Fingers_FrontBrake_Explicit` = `Fingers_FrontBrake_Condition * 0.28` with the condition `SmoothFloat_Constant(BikeFrontBrake, 3.5)` | `FrontFingers_Brake_Alpha` = `SmoothFloat_Constant(explicit > 0, 10)` |

The clips are 0.633 s (20 frames) of full poses whose only moving bones are the index and middle fingers of both hands (the four-finger
clip also the ring and little fingers); each layer takes the hand it is meant for. The call `Hands_FingersOverride(0, 0.28, 1, 0, 0.23, 2, 3)`
gives the windows of the hand-level `Hand_Clutch_Explicit` and `Hand_FrontBrake_Explicit` (0 to 0.28 and 0 to 0.23 s), which are not the
finger layers' variables and are **not played**. The rear brake lever (a scooter's left hand, `RearFingers_Brake_Alpha`, the same clip) is not played.

**What pulls the clutch.** The bike's own clutch lever: the handler pulls it for a gear change (`ClutchTimingOnGearChange`), for a start
(the launch clutch) and while the bike stands (`libs/blackbox-vehicle/src/bike/powertrain.rs`, `Bike::clutch_pulled`: the clutch's
engagement turned round). The blueprint's own rule makes the target from `VehicleClutchRemap`, `bIsGoingToChangeGearUp` and the clutch
type (targets 0.2 to 0.27 s); here the lever itself is the input, a choice. **What pulls the brake:** the front brake input.

## 2. The throttle wrist

`HandsBehaviours_(Leaning,_UpAndDown)` makes the right wrist's offsets from five poses of the hand, in the **grip's own frame**
(`Hand_R_Wrist{Up,Down,In,Out}{Rot,Pos}`, mirrored from the left hand's `Left Up Rotation` and so on) [D]:

| Pose | Rotation (`FRotator`: pitch, yaw, roll, degrees) | Position (centimetres, grip frame) |
|---|---|---|
| Down (the throttle) | (0, 30, 0) | (-0.6, 5, 0) |
| Up | (0, -21, 0) | (0, -2.1, 0) |
| In | (20, 0, 0) | (0, 0, 4) |
| Out | (-35, 0, -1) | (-0.5, -1, -8) |

(The left hand's are mirrored; the class defaults print their decimals with commas.) `Hand_R_WristDown` is the strongest of the clips'
`wrist_R_down` notifies and `VehicleThrottle` (`LogicalMax`), so the wrist **rolls down with the throttle**. The offset is followed at
`Hand_R_TransformSpeed`, 4 a second (`SmoothVector_Interpolate`, `SmoothRotator_Interpolate`), and goes to `RightGripIKOffsetTranslation`
and `RightGripIKOffsetRotation`, which move the grip's IK target. The yaw is a turn about the grip socket's own z axis, which in the
bikes' sockets is the handlebar's axis (measured on the CRF 450 R, the RC 390 and the Panigale V4 S: the socket's z is within 22 degrees of the car's y). The
other poses answer the clips' own notifies (`wrist_R_up`, `wrist_L_in`, ...: wheelie and lean poses), `WristX_*` steps `race_Wrist_X`
(a 2 s sweep of the wrists and fingers) by the outer and inner wrist weights, and `HandsClutchFingersRotCompensation` turns the left
hand 20 degrees about one axis while it is down; none of those is played.

## 3. Where the hands hold the grips

A bike's `SOCKET_{L,R}_HANDLE_POSE` (on the grip bone `BONE_{L,R}_GRIP`) is where RIDE 6 puts the rider's wrist; the grip rubber is on the
bone. The two are not the same distance apart on every bike: the wrist socket sits (cm, in the bike's frame, from the grip bone) at
(-11.0, 3.7, 3.2) on the KTM 450 SMR the rider's pose was authored for, (-9.1, 6.8, 6.8) on the CRF 450 R SMT, (-15.3, 6.0, 5.3) on the
RC 390, (-12.2, 7.3, 3.6) on the Panigale V4 S, (-11.1, 5.5, 3.9) on the MT-09 [measured]. The hand goes where the authored pose holds the
reference bike's grip, which is the same place in the hand on every bike. Putting the wrist at the socket plus the reference bike's
offset (the previous rule) leaves the hand behind the grip by up to 5.7 cm on the RC 390 and 3.1 cm on the Panigale, the clip-on bikes:
the glove sat on the tank behind the clip-on. The hand is now held by **the grip bone**, in place and in turn, the way the authored pose holds the reference bike's:

```
tip_anchor (attach.json, per hand)   = the hand bone against the reference bike's grip bone in the authored pose (place and turn)
anchor, anchor_rotation (contacts.json, per grip) = this bike's grip bone, in the bike's frame
hand's place and turn = the grip bone's, composed with tip_anchor
```

An earlier version of this rule kept the turn of the bike's grip socket and moved only the place to the bone: on the RC 390 the hand then lay
on top of the grip, the grip's rubber showing under the fist (the socket's turn differs from the bone's by up to 25 degrees of pitch from the
supermoto's, which the authored pose never saw). With the bone's own frame the hand wraps the grip the way it does on the reference bike on
every bike: pictures of the RC 390, CRF 450 R SMT, Panigale V4 S and Hayabusa show the fingers around the bar and the bar end past the
glove. The sockets are still read (older files, `HANDS_ON_SOCKET`), and the throttle's wrist roll is about the socket's axis, which is the
bar's, seen from the world. Measured against the socket rule's wrist place: the CRF 1.4 cm, the RC 390 5.7 cm, the Panigale 3.1 cm, the
MT-09 1.2 cm. A bike with a `contacts.json` or an `attach.json` written before the anchors goes to its socket as before.

## 4. What the game does with it

`crates/nfsmw/src/scenes/world/drive/rider/hands.rs` makes the signals from the bike's clutch lever, front brake and throttle: the
graph's `clutch` and `brake` (the levers, 0 to 1, followed at 2.5 and 3.5 a second) and `clutch_alpha` and `brake_alpha` (the layers' weights); and `wrist`
(the throttle followed at 4 a second), which `pose::roll_wrist` takes into the right grip's target: the target turns by `wrist` times the file's
`wrist_throttle` rotation and moves by `wrist` times its translation, both against the grip's own frame. The graph is
`mods/ride6-bikes/src/rider/graph_fingers.rs`; the offset is in `attach.json` (`wrist_throttle`, [ride6-rider.md](../formats/ride6-rider.md)), written by
`attach.rs` from the constants above.

## 5. What is data and what is a choice

| Item | Origin |
|---|---|
| The clips, the layer structure, the windows, the rates 2.5, 3.5, 7, 10 and 4, the five offset poses | Data (the blueprint's class defaults and functions) |
| The lever of the clutch from the bike's own clutch | Choice (the blueprint remaps `VehicleClutchRemap` and the gear change flag) |
| Two fingers on the brake | Data (the blueprint's default) |
| The mask: every bone under the hand bone | Choice (the branch filters name the first finger bones; the rest of the finger is under them) |
| The hand's grip point from the authored pose and the bike's grip bone | Measured (the reference pose is taken to hold the reference bike's grip; the previous agent checked it against the render); the rule is a choice |
| The wrist's offset in the socket's frame | Inference (the offset's space is in the control rig's cooked data; the yaw about the grip's axis is the throttle) |

## 6. Not done

The up, in and out wrist poses, the wrist sweep and the clutch compensation, the rear brake lever, the `Idle` clips with the throttle
(`race_trottle`), the blipper (`race_Add_Blipper`: a downshift throttle blip on bikes with a blipper, `BikeHasBlipper`), and the other
BrakeFingers numbers (1 and 4: the clips are exported; no option picks them).

## 7. Looking at it

`nfsmw view-car <ID>` shows the bike with its rider standing, his feet on the ground, once his smoothers have settled (five seconds of
animation). `--zoom N` brings the camera N times closer, `--pitch DEGREES` tilts it, `--focus-height METRES` aims it (about 1 m for the grips),
`--yaw` turns it; `--rider-state "speed=-5,clutch=1,throttle=1,brake=0.5,lean=0.6,tuck=1"` puts the rider in a state (km/h; a negative speed is a
bike rolling back); the riding style is `NFSMW_RIDING_STYLE=balanced|shoulders-out|old-school`. Run offscreen with `--screenshot out.png --muted`
and an isolated profile (`BLACKBOX_PROFILE_DIR`).

