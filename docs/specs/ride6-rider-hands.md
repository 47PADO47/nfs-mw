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
| Clutch | `race_clutch_brake_gear` | left | `Fingers_Clutch_Explicit`: a time in the clip that follows the lever's target (0.27 s at a fully pulled lever, 0.24 for the woman) at `FingersClutchExplicitSpeed`, **2.5 clip seconds a second** (10 on a scooter): a fully pulled lever is reached in 0.11 s | `Fingers_Clutch_Alpha` = `SmoothFloat_Constant(explicit > 0, 7)` |
| Front brake | `race_clutch_brake_gear_{OneFinger,TwoFingers,FourFingers}` by `BrakeFingers` (1, 2, 4; the blueprint's default and every `RIDING_STYLE` preset: 2) | right | `Fingers_FrontBrake_Explicit` = `Fingers_FrontBrake_Condition * 0.28` with the condition `SmoothFloat_Constant(BikeFrontBrake, 3.5)` | `FrontFingers_Brake_Alpha` = `SmoothFloat_Constant(explicit > 0, 10)` |

The clips are 0.633 s (20 frames) of full poses whose only moving bones are the fingers of both hands; each layer takes the hand it is meant for. **Which fingers move**
(measured on the exported clips, angle of each finger bone against its pose at 0 s): in `race_clutch_brake_gear` the left hand's index and middle fingers (peak 80 degrees at
0.16 s, 32 to 54 degrees at the pulled lever's 0.27 s) and the right hand's index and middle; in the three brake clips the right hand's one (index), two (index and
middle) or four fingers, and in all three the **same left hand with all four fingers** (index and middle 44 degrees at 0.28 s, ring and little 7 to 18). The blueprint's branch
filters of the clutch layer name only `index_01_l` and `middle_01_l` (the clutch pulls with two fingers), those of the brake layer all four right-hand fingers [D]. The call `Hands_FingersOverride(0, 0.28, 1, 0, 0.23, 2, 3)`
gives the windows of the hand-level `Hand_Clutch_Explicit` and `Hand_FrontBrake_Explicit` (0 to 0.28 and 0 to 0.23 s), which are not the
finger layers' variables and are **not played**. The rear brake lever (a scooter's left hand, `RearFingers_Brake_Alpha`, the same clip) is not played.

**What pulls the clutch.** The bike's own clutch lever: the handler pulls it for a gear change (`ClutchTimingOnGearChange`), for a start
(the launch clutch) and while the bike stands (`libs/blackbox-vehicle/src/bike/powertrain.rs`, `Bike::clutch_pulled`: the clutch's
engagement turned round). The blueprint's own rule makes the target from `VehicleClutchRemap`, `bIsGoingToChangeGearUp` and the clutch
type (targets 0.2 to 0.27 s); here the lever itself is the input, a choice. **What pulls the brake:** the front brake input.

**Why the fingers did not show on a gear change** (a user's playtest: "the left hand plays but doesn't use any fingers"). The game followed the lever (0 to 1) at the blueprint's 2.5 a second,
but the blueprint's 2.5 is in seconds of the clip a second and its lever is 0.27 s of clip: the lever (0 to 1) goes at 2.5 / 0.27 = 9.3 a second, a pull of a tenth of a second.
A gear change's clutch lasts about that, so the first version closed the fingers 0.25 of the way. Fixed (`hands.rs`); the clip is also played to its 0.27 s (it was 0.28).
The lever itself (the bike's clutch lever mesh) does not move: it is part of the handlebar solid, steered with the fork (a lever bone, `BONE_L_LEVER`, exists in the
bike's skeleton and is not animated by this project).

## 2. The throttle wrist

`HandsBehaviours_(Leaning,_UpAndDown)` makes the right wrist's offsets from five poses of the hand, in the **grip's own frame**
(`Hand_R_Wrist{Up,Down,In,Out}{Rot,Pos}`, mirrored from the left hand's `Left Up Rotation` and so on) [D]:

| Pose | Rotation (`FRotator`: pitch, yaw, roll, degrees) | Position (centimetres, grip frame) |
|---|---|---|
| Down (the throttle) | (0, 30, 0) | (-0.6, 5, 0) |
| Up | (0, -21, 0) | (0, -2.1, 0) |
| In | (20, 0, 0) | (0, 0, 4) |
| Out | (-35, 0, -1) | (-0.5, -1, -8) |

(The left hand's are mirrored; the class defaults print their decimals with commas.) `Hand_R_WristDown` is the strongest of the clips' `wrist_R_down` notifies and `VehicleThrottle` (`LogicalMax`), so the wrist **rolls down with the
throttle**. The offset is followed at `Hand_R_TransformSpeed`, 4 a second (`SmoothVector_Interpolate`, `SmoothRotator_Interpolate`), and goes to
`RightGripIKOffsetTranslation` and `RightGripIKOffsetRotation`, which move the grip's IK target [D]. The other poses answer the clips' own notifies
(`wrist_R_up`, `wrist_L_in`, ...: wheelie and lean poses), `WristX_*` steps `race_Wrist_X` (a 2 s sweep of the wrists and fingers) by the outer and
inner wrist weights, and `HandsClutchFingersRotCompensation` turns the left hand 20 degrees about one axis while it is down; none of those is played.

**What is played here: the roll only.** The yaw of 30 degrees is the roll of the wrist about the grip. The translation (-0.6, 5, 0 cm) is **not
played**: the space it is in belongs to the control rig's cooked data (not read), and applied in the grip socket's frame it moved the whole hand 5
cm perpendicular to the bar, off the grip and under it. A user's playtest showed it (helmet camera on a CRF 450 R SMT at 72 km/h: the right glove
below the grip, the grip bare; with the first version of this rule the throttle translated the hand instead of turning it). With the roll alone the
glove stays wrapped around the grip at every throttle and turns about it (0, 0.5 and 1 checked on the CRF 450 R SMT; see section 7).

**The axis of the roll** is the grip's own: in the grip bone's frame the grip's cylinder is the bone's z axis in the exporter's frame (y in the car's
axes), and the bone's origin lies on that axis. Measured on four bikes against the principal axis of the grip mesh's vertices skinned to
`BONE_R_GRIP` (CRF 450 R SMT, RC 390, Panigale V4 S, KTM 690 SMC R): the axis and the bone's z agree within 1 degree and the bone's origin is
within 0.3 mm of the axis [measured]. The bone's axis is 14 to 29 degrees off the car's lateral axis on the real bikes (the bars are swept, the
clip-ons are angled), and the grip socket's z (the earlier rule's axis) is only within 22 degrees of it. Turning the grip bone's frame about its own
axis takes the hand, held by that bone ([section 3](#3-where-the-hands-hold-the-grips)), round the grip with the grip staying in the fist.
The direction is fixed: the knuckles roll toward the rider (the throttle opens). Older files without a grip bone turn the socket's frame by the
file's rotation instead.

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
glove. The sockets are still read (older files, `HANDS_ON_SOCKET`). Measured against the socket rule's wrist place: the CRF 1.4 cm, the RC 390 5.7 cm, the Panigale 3.1 cm, the
MT-09 1.2 cm. A bike with a `contacts.json` or an `attach.json` written before the anchors goes to its socket as before.

**Along the grip** (a user's playtest: "hands a bit back from where they should be, so they get cut inside the handlebars"). The grip bone's origin is on the grip's axis at the
**inboard end** of the rubber (measured on twelve bikes: the vertices skinned to the bone run from 0, 1 cm inboard on a scooter, to 11 to 14 cm outboard of it), and every bike's hand is
held by it with the same `tip_anchor`, so the posed glove is the same distance from the bone on every bike: its vertices span **4.6 cm inboard to 9.4 cm outboard** of the bone
(eight bikes: CRF 450 R SMT, RC 390, Panigale V4 S, KTM 690, Challenger, Africa Twin, Dragster 200, S 1000 RR, with the probe of `probe_tests.rs`). The glove therefore overhung
the rubber's inboard end by 4.6 cm: its cuff was inside the throttle housing, the lever perch and the bar clamp, **7 to 25 mm deep** (the deepest of the glove's vertices into the
bar, housing and lever vertices' outward surface; 16, 23, 16, 19, 16, 25, 7 and 12 mm in the order above), and left 2 cm of rubber bare outboard. The importer now measures the rubber's span
along the grip from the chassis mesh (`grip_span` of each grip target in `contacts.json`, `[inboard, outboard]` metres outboard-positive, `rider/grip_span.rs`) and the game
centres the glove on it: the grip bone's place for the hand is moved along the bone's own y axis (the grip's) outboard by `(inboard + outboard) / 2 - GLOVE_CENTRE_M`, the
glove's middle measured at **2.4 cm** outboard of the bone (`attach.rs`, one named constant: the glove's own size, the same for every bike). That is 3.0 to 4.5 cm on the eight bikes. Moving the
glove 3.5 cm outboard brings the deepest overlap to 4 to 8 mm on the same measure (the residue is the wrist cuff against lever perches: the vertex-based measure
is coarse), and the glove overhangs the outboard end by 0 to 2 cm on the longest bikes. A contacts file without `grip_span` assumes the usual 12 cm rubber. The throttle's roll turns
about the same axis and is not changed by the slide. **Ahead of the grip** (the user's next playtest: "better, but still not on, should be further"). Looked at from above, the glove still lay **behind** the grip: the knuckles at the
rubber's rear edge, the rubber showing in front of them, because the wrist is held 12.5 cm behind the bone and the fist's middle then lies about 3 cm behind the axis. The
crude overlap number of the first fix could not see it (it only measured vertices in the bar). Looking at renders instead (CRF 450 R SMT from above and from the bar end, with the
hand moved 0, 3, 4.5 cm forward and 0.6 cm down or 1 cm up on top of the slide): 3 to 4.5 cm forward lays the knuckles over the rubber and puts the bar end cap in the middle of the fist; height
changes added nothing. The hand is therefore brought forward by one named constant, `HAND_FORWARD_M` = **3.5 cm** along the grip bone's own x axis (across the grip, so
along the bar's sweep on clip-ons), on top of the slide along the grip; the same for every bike, because the hand's `tip_anchor` is. Height is not touched. Twelve bikes were looked at
(top, bar end, levers pulled): CRF 450 R SMT, RC 390, Panigale V4 S, KTM 690, Challenger RR, Africa Twin, Dragster 200, S 1000 RR, 1290 Super Adventure R, 1290 Super Duke R, Hypermotard, KTM 450
(the supermoto the pose was authored for). The grip span's end: the importer's span is the rubber's vertices only (the bar end cap and the weights are other bones), so the glove's outboard end
sits at the rubber's end, not at the cap.
The hand is the same on the clutch side: the left bone's y points outboard, the right's inboard.

## 4. What the game does with it

`crates/nfsmw/src/scenes/world/drive/rider/hands.rs` makes the signals from the bike's clutch lever, front brake and throttle: the
graph's `clutch` and `brake` (the levers, 0 to 1, followed at 2.5 and 3.5 a second) and `clutch_alpha` and `brake_alpha` (the layers' weights); and `wrist`
(the throttle followed at 4 a second), which `pose::roll_wrist` takes into the right grip's target: the grip bone's frame turns about its own axis by `wrist`
times the angle of the file's `wrist_throttle` rotation (30 degrees); the file's translation is not used. The graph is
`mods/ride6-bikes/src/rider/graph_fingers.rs`; the offset is in `attach.json` (`wrist_throttle`, [ride6-rider.md](../formats/ride6-rider.md)), written by
`attach.rs` from the constants above.

## 5. What is data and what is a choice

| Item | Origin |
|---|---|
| The clips, the layer structure, the windows, the rates 2.5, 3.5, 7, 10 and 4, the five offset poses | Data (the blueprint's class defaults and functions) |
| The lever of the clutch from the bike's own clutch | Choice (the blueprint remaps `VehicleClutchRemap` and the gear change flag) |
| The number of brake fingers: 1, 2 or 4, 2 by default | Data (`BRAKE_FINGERS` of the riding style page; 2 the blueprint's default and every preset's) |
| The number of clutch fingers: 2 or 4, 2 by default | Choice: RIDE 6 has no clutch option and always plays the clutch clip (two fingers); 4 is the left hand of the four-finger clip, which exists (the three brake clips share one left hand with all four fingers); 1 has no clip |
| The mask: every bone under the hand bone | Choice (the branch filters name the first finger bones; the rest of the finger is under them) |
| The hand's grip point from the authored pose and the bike's grip bone | Measured (the reference pose is taken to hold the reference bike's grip; the previous agent checked it against the render); the rule is a choice |
| The wrist's roll: 30 degrees about the grip bone's own axis, toward the rider | Data for the angle; measured for the axis; choice for the direction. The offset's translation is not played (choice, from a playtest) |

## 6. Not done

The up, in and out wrist poses, the wrist sweep and the clutch compensation, the rear brake lever, the `Idle` clips with the throttle
(`race_trottle`), the blipper (`race_Add_Blipper`: a downshift throttle blip on bikes with a blipper, `BikeHasBlipper`), the clutch lever mesh pulled
with the fingers and a clutch of one finger.

## 6a. The finger settings

`brake_fingers` (1, 2 or 4; default 2) and `clutch_fingers` (2 or 4; default 2): Gameplay options **Brake Fingers** and **Clutch Fingers**, config keys of the same names,
`NFSMW_BRAKE_FINGERS`, `NFSMW_CLUTCH_FINGERS`, console `set brake_fingers 4`. The graph reads them as the parameters `brake_fingers` (0 one finger, 1 two, 2 four: a `Select`
over the three brake clips on the right hand) and `clutch_fingers` (0 the clutch clip, 1 the four-finger clip's left hand) (`graph_fingers.rs`); the importer writes
the `Select`s, so a rig made before this change (`ride6-rider-import`) plays the two-finger clips whatever the settings say until it is imported again.

## 7. Looking at it

`nfsmw view-car <ID>` shows the bike with its rider standing, his feet on the ground, once his smoothers have settled (five seconds of
animation). `--zoom N` brings the camera N times closer, `--pitch DEGREES` tilts it, `--focus-height METRES` aims it (about 1 m for the grips),
`--yaw` turns it; `--rider-state "speed=-5,clutch=1,throttle=1,brake=0.5,lean=0.6,tuck=1"` puts the rider in a state (km/h; a negative speed is a
bike rolling back); the riding styles are `NFSMW_RIDING_STYLE_ROAD`, `_MOTARD` and `_OFFROAD` `=balanced|shoulders-out|old-school` (`NFSMW_RIDING_STYLE` sets all three) and the fingers
`NFSMW_BRAKE_FINGERS=1|2|4` and `NFSMW_CLUTCH_FINGERS=2|4`. Run offscreen with `--screenshot out.png --muted`
and an isolated profile (`BLACKBOX_PROFILE_DIR`).

