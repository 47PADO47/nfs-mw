# RIDE 6 rider animation (supermoto)

How the rider of RIDE 6 is posed on a bike, as far as the user's own game data says it, and how this project drives the
same pose from its bike model. Written for the KTM 450 SMR (chassis 369, a "motard" bike). The data was read with
CUE4Parse from the user's install (assets, the animation blueprint decompiled to pseudo-code by CUE4Parse's Blueprint
decompiler, class layouts from the Dumper-7 SDK dump). No decompiled native code was used; the native part of the
rider's animation instance is listed as inputs only. Sources and provenance: [ride6-rider.md](../provenance/ride6-rider.md).
Files: [ride6-rider.md](../formats/ride6-rider.md). Runtime: [skeletal-animation.md](skeletal-animation.md).
Integration into the game: [ride6-rider-render-integration.md](ride6-rider-render-integration.md).

## 1. The pieces

- **One animation blueprint** drives every rider: `riderShared/anim_blueprints/RiderGP_racing_Master_animBlueprint`
  (parent class `GameRiderAnimInstance`, native). The rider data assets `DataAssets/riders/{MotardRiderAnimation, Balanced,
  BodyOut, ElbowOut, oldschool, ShouldersOut, RaceRiderAnimation}` all name this same blueprint; the style is chosen inside it.
- **Bike type.** The blueprint's `SET Animations` switches on an integer `BikeType`: 0 race, 1 naked, 2 **motard**, 3
  scooter, 4 enduro/motard hybrid (loads `enduro/Pose` and `motard/Leaning`), 5 naked. For type 2 it loads the clips of
  `Animations/motard/`. `RidingStyle` then picks the lean set: 0 `motard_Leaning_Knee_*`, 1 `motard_Leaning_*`, 2
  `motard_Old_School_*`. The bike data of chassis 369 does not name its type (see section 9).
- **Rider skeleton** `bodies/skeletons/suit_game_Skeleton`: 81 bones. `root` > `pelvis` > `spine_01..03` > clavicles,
  arms, fingers, `neckRoot`, `neck_01`, `head`, eyes, `helmet`; legs `thigh/calf/foot/ball` plus IK copies `thigh_IK`,
  `calf_IK`, `foot_IK` (l, r); `gluteus_Vol_l/r`; pole vectors `PV_arm_l/r`, `PV_leg_l/r` (children of `root`);
  `trophy`, `drink`, `extraProps`, `headRef`.
- **Sockets of the rider skeleton** (9): `hand_l_Socket`, bottle sockets, `calf_l/r_DetectionSocket` on `calf_IK_l/r`
  (knee-to-ground detection).
- **Control rigs** (Unreal Control Rig, solving with the engine's PBIK full-body IK): `CtrlRig_HandsIK`,
  `CtrlRig_FeetCorrection`, `CtrlRig_GroundDetection_Correction`, `CtrlRig_Look2Apex`, `CtrlRig_OverallTransformPVCorrection`.
  Their solver settings are stored in binary rig memory the export does not expose; this project replaces PBIK with a
  two-bone solve on the same limbs (section 6).

## 2. What drives the pose: the inputs

The native base classes (`IgnitionRiderAnimInstance`, `IgnitionMotorbikeRiderAnimInstance`, SDK dump) hold the values the
bike pushes into the blueprint each frame (type `float` unless noted; the blueprint reads them as variables):

| Input | Meaning (from its name and its use in the blueprint) |
|---|---|
| `RiderLean`, `BikeLean` | lean of the rider and of the bike, normalised, signed |
| `RiderWeightX`, `RiderWeightY`, `OriginalRiderWeightX/Y` | the user's weight shift (stick) |
| `BikePitch`, `BikeWheelie`, `BikeStoppie` | pitch and the amounts of wheelie and stoppie, 0..1 |
| `BikeFrontBrake`, `BikeRearBrake`, `VehicleThrottle`, `VehicleBrake`, `VehicleOriginalSteer`, `VehicleSpeed` | controls and speed |
| `BikeFrontSuspensionLoad`, `BikeRearSuspensionLoad` | suspension load (bump reactions; threshold `PreTurn_Leg_SuspensionTreshold` 1200) |
| `BikeFrontWheelSlip`, `BikeRearWheelSlip`, `...Contact`, `...SurfaceType` | slip, ground contact, surface id (tarmac 1, kerb 14) |
| `TuckedIn` | tuck (aero) amount, from the native speed logic |
| `InertiaDisplacement` | vector, the rider's inertial shift |
| `PreTurn`, `NextCurveDirection`, `NextCurveDistance`, `LegOut` | upcoming-corner anticipation (spline based; not available to this project) |
| `LeftGripIK_Translation/Rotator`, `RightGripIK_*`, `LeftFootRestIK_*`, `RightFootRestIK_*` | targets handed over by the bike (its hand and foot sockets) |

Units: speeds are compared in km/h (thresholds 120..150, 140, 200..350 only make sense so).

## 3. The pose, as recovered

In the `Racing` state the blueprint builds the pose from the clip sets of section 1:

- **Lean.** Three clips by lean sign: left, centre, right. For the "standard" (sitting upright) set the clips are
  `motard_Leaning_LowSpeed_L`, `motard_CenterPose_LowSpeed`, `motard_Leaning_LowSpeed_R`; for the "aero" (tucked) set
  `motard_Leaning_HighSpeed_L`, `motard_CenterPose_HighSpeed`, `..._R`. In the blueprint the variables are
  `Leaning_Standard_Left/Right`, `Standard_Pose`, `Leaning_Aero_Left/Right`, `AeroPose_Low`. The graph node
  `IgnitionAnimGraphNode_ThreeWayBlend` (negative, zero, positive, alpha) is the structure used, with `TwoWayBlend` pairs
  below it for standard versus aero. `CenterPose_*` are 2-frame static poses; the lean clips are 2 s, 30 fps loops that
  keep the rider moving.
- **Wheelie** (`Wheelie/ Stoppie` function): `motard_wheelie_min_L/R` to `motard_wheelie_max_L/R`; entry ramp
  `FuzzifyRamp(BikeWheelie, 0.2, 0.4)`; pose range 0.2 to 0.5; `Wheelie_Delay` 0.2 s.
  The standing wheelie is a blend space `motard_Wheelie_Standing_BS` (axis "Steer" -1..1, second axis 0..0.3).
- **Stoppie:** clip `motard_stoppie` (1.3 s) stepped by an explicit time `StoppieValue * 1.27` (seconds); `StoppieValue`
  goes 0 at `BikeStoppie <= 0` to 1 at `BikeStoppie > 0.18`, smoothed (speed 3, flashback 0.15 s).
- **Additive layers:** `motard_Shake_LowSpeed/HighSpeed` (body shake), `motard_bump_*` and `motard_HS_bump_*` (front, left,
  right, low and mid, by suspension impulse; thresholds `Bump_X/Y_HitTreshold` 150, strength 230..700),
  `motard_bump_SeatRepositioning`, `race_Add_SpecificCorrection` (per-bike), imbalance clips `motard_bump_mid_L/R`,
  `motard_bump_02_L/R`.
- **Legs and knees:** `motard_ass_knee_{L,R}_{Open,Closed}`, `motard_leg_out_{L,R}_{down_Rossi,up}` (leg out when
  braking into a corner; `PreTurn_Leg_BrakeTreshold` 0.5, lean threshold 0.3), knee/ground traces with the sockets above.
- **Start and stop** (state machine `Main_SM`: Standing, Racing, Start, Stop, Idle, ...): `motard_restart_idle_*`,
  `motard_start_*` (also `_leaning_L/R`), `motard_stop_*`, `motard_retro_*`; speed thresholds `StartStop_Speed_Treshold` 6,
  `StartStop_to_Racing_treshold` 0.85; start clip length 3.3 s, stop clip 2.9 s.
- **Look back:** `motard_look_back_LS_B_L/R`, `motard_look_back_HS_1_B_L/R`.
- **Hands:** fingers on brake and clutch (`race_clutch_brake_gear*`), wrist rotations by notifies, hand IK (section 6).

Class defaults read from the blueprint (the full list is in the provenance note): `RiderLean_multiplier` 0.5,
`RiderLean_Min/Max_DecaySpeed` 250 / 400 (km/h; lean is reduced to `RiderLean_Decay_Multiplier` 0.3 above), smoothing
speeds 4, 2, 3, 4; `Rider_UserLeanAddition_Amount` 0.4; `Apex_*` (min speed 200, max speed 350, values 0.4..0.9);
`InertiaDisplacement_RidingMin/Max` (-15,-6,-4)..(15,0,20) cm over 5..40 (speed units); head inertia multipliers pitch 5,
roll -2; `SpeedThresholdBasedOnBike` per bike type: **210 / 160 / 140** (race / naked / motard).

The native code computes the fuzzy-logic variables (`RiderLean_Polished`, `TuckedIn_Alpha`, ...) through a scripting
library (`LoadScript`, `FuzzifyRamp`, `DefuzzifyFloatEx`, `SmoothFloat_Interpolate`); the blueprint functions that set
them were read (`Rider_Lean_Polish`, `LeaningAnimations`, `TuckedIn_Alpha`, `Wheelie/ Stoppie`) but not re-implemented
beyond what section 5 lists.

## 4. Conventions of the rider's frame

In the skeleton's component space the rider faces **+y**, up is +z and the rider's left is +x (Unreal, centimetres). Bone
`root` sits at the seat point; `pelvis` is 4.3 cm above it in the centre pose. The rider is attached to the bike at the
bike's socket `SOCKET_RIDER` (on `BONE_CHASSIS`, translation (-34.388, 0.000106, 91.255) cm, rotation yaw -90 deg), which
turns +y into the bike's +x (forward). Converted to the car frame: translation (-0.3439, 0, 0.9126) m, a +90 deg turn
about z.

## 5. The graph this project plays (`rider/graph.rs`)

Parameters, in order: `lean` (-1 left .. 1 right), `speed_kmh`, `tuck` (0..1), `wheelie` (0..1), `stoppie` (0..1).

```
riding   = Blend( ThreeWay[Leaning_LowSpeed_L, CenterPose_LowSpeed, Leaning_LowSpeed_R](lean),
                  ThreeWay[Leaning_HighSpeed_L, CenterPose_HighSpeed, Leaning_HighSpeed_R](lean), weight = tuck )
wheelie  = Blend( Blend(wheelie_min_L, wheelie_max_L, ramp(wheelie 0.2..0.5)),
                  Blend(wheelie_min_R, wheelie_max_R, ramp(wheelie 0.2..0.5)), weight = (lean + 1) / 2 )
pose1    = Blend( riding, wheelie, weight = ramp(wheelie 0.2..0.4) )
pose2    = Blend( pose1, Clip(motard_stoppie, Fraction(ramp(stoppie 0..0.18) * 1.27 / 1.3)), weight = ramp(stoppie 0..0.18) )
final    = Additive( pose2, Blend(Shake_LowSpeed, Shake_HighSpeed, ramp(speed 220..280)), weight = ramp(speed 10..220) * 0.4 )
```

Then IK (section 6). Every constant above is a default of the blueprint (names in `graph.rs`).

Mapping from this project's bike state (`blackbox-vehicle::bike`, physics space x right, y up, z forward):

| Parameter | Value |
|---|---|
| `lean` | the **rider's** roll (the bike's roll plus his body's own against it, `RiderRotation::roll_right`, which lags the bike's roll by the angular spring of [ride6-rider-body-aero.md](ride6-rider-body-aero.md)) divided by the rider's maximum lean (the steepest lean the bike model allows): signed, clamped to [-1, 1]; positive = to the right |
| `speed_kmh` | forward speed in m/s times 3.6 |
| `tuck` | 0 below `SpeedThresholdBasedOnBike` (140 km/h for a motard) rising to 1 at full throttle above it, smoothed at 6 s^-1, times (1 - braking) (the shape of `TuckedIn_Alpha`: throttle held, brake released, no wheelie) |
| `wheelie` | front wheel lift: pitch of the body nose-up divided by the bike's wheelie limit, 0..1 |
| `stoppie` | rear wheel lift likewise |

## 6. Attachment and IK (`rider/attach.rs`, `attach.json`)

The pose is authored for a bike in its rest pose; the hands and feet are then moved to the bike's own points:

| Point | Bike socket (parent bone) | Bike space, Unreal cm | Car frame, m |
|---|---|---|---|
| seat (rider root) | `SOCKET_RIDER` (`BONE_CHASSIS`) | (-34.388, 0.000, 91.255) | (-0.3439, 0.0000, 0.9126) |
| left grip | `SOCKET_L_HANDLE_POSE` (`BONE_L_GRIP`) | (11.76, -33.35, 113.76) | (0.1176, 0.3335, 1.1376) |
| right grip | `SOCKET_R_HANDLE_POSE` (`BONE_R_GRIP`) | (12.96, 32.65, 115.40) | (0.1296, -0.3265, 1.1540) |
| left foot | `SOCKET_LEFT_FOOT` (`BONE_CHASSIS`) | (-25.73, -20.88, 42.41) | (-0.2573, 0.2088, 0.4241) |
| right foot | `SOCKET_RIGHT_FOOT` (`BONE_CHASSIS`) | (-23.77, 19.18, 43.15) | (-0.2377, -0.1918, 0.4315) |

(`attach.json` is computed from the exported bike skeleton, not from this table.) The grips sit on `BONE_HANDLE`
(Unreal (29.72, 0, 100.38) cm, tilted back by the rake), so they follow the steering. How well the authored poses already
fit, measured with the exported data (rider root on the seat socket, forward kinematics of the clip's first frame; bike
space, cm):

| Pose | left hand | right hand |
|---|---|---|
| grips | (11.8, -33.4, 113.8) | (13.0, 32.7, 115.4) |
| `CenterPose_LowSpeed` | (10.3, -31.4, 116.2), 3.5 away | (10.3, 31.4, 116.2), 3.1 away |
| `CenterPose_HighSpeed` | (8.5, -33.4, 115.1) | (8.5, 33.4, 115.1) |
| `Leaning_LowSpeed_R`, frame 30 | (10.8, -31.4, 116.2) | (7.2, 32.3, 112.9) |
| `motard_aero_pose` | (9.6, -34.2, 112.0) | (9.0, 31.5, 114.4) |

so every pose is within about 5 cm of the grips and IK only has to close that. The ankles in the same poses
(`foot_l` (-17.8, -19.3, 40.1) in the centre pose) are about 8 cm in x from the peg sockets, which mark the peg, not
the ankle.

The runtime solves four limbs with `apply_two_bone` (`attach.json["limbs"]`): left and right arm
`upperarm > lowerarm > hand` with pole bone `PV_arm_l/r`, left and right leg `thigh > calf > foot` with pole `PV_leg_l/r`;
the pole position is the pole bone's component position in the animated pose. Targets are the bike sockets above moved
with their bones (grips with the steering), minus the offset between the hand/foot bone and the grip/peg point in the
authored centre pose (so the pose's own hand placement is kept: `attach.json` stores, per limb, `tip_offset` = the tip bone's
place in the first frame of `motard_CenterPose_LowSpeed` minus the target, rider space, and the runtime aims the tip at the
target plus that offset). The grips turn about the steering axis of the front wheel's mount (the line the drawn fork and
bars turn about) by the handlebar angle; they do not slide with the fork, as the bars do not. The frames: rider space to the
bike frame by `rider_root_in_bike`, the bike frame to the model's frame by the difference of the hubs. The blueprint's own offsets (`RightHandIKLocationOffset`
(-0.42, 0.37, 0.8), `LeftHandIKLocationOffset` (0.42, -0.37, 0.8), rotation offsets (1.2, 0.5, 5.0) / (1.2, 0.5, -2.5),
foot gear offsets) are tuning of the full-body solver and are not applied.

## 7. Weight shift, hang-off, torso turn

The blueprint's `InertiaDisplacement` input is the rider's displacement from the physics (`FRiderComponent`: the target place
of the weight, the lean and the speed plus the sag and lag of his spring, in the bike's axes). The blueprint holds it to the
ranges `InertiaDisplacement_RidingMin/Max` and smooths it at `InertiaDisplacement_SmoothSpeed` 4, then moves pelvis and head
with it. This project does the same from its own physics (`drive/rider/params.rs`, `pose.rs`):

- `RiderSample::offset` = `Bike::rider_offset()` (physics axes x right, y up, z forward, metres) = the place the weight, the
  lean and the speed ask for plus his spring's deflection. It is taken into the car's axes (forward, left, up) and **held to the
  data's range**: forward from -6 cm to 0 (the rider only moves back; the blueprint's range has no forward part), sideways
  +-15 cm, up from -4 to +20 cm (`attach.json` `inertia.min/max`, from the class defaults). It follows at 4 per second
  (`inertia.rate`). The lateral part is the **hang-off**: for a bike whose `LateralOffset` follows the lean (the supermoto) the
  pelvis, and with it the legs, the spine, the arms and the head, moves with the lean; with the lateral weight input it moves
  to the side asked for.
- `RiderSample::rotation` = the rider's body against the bike's attitude (roll, pitch, yaw), from the angular spring-damper. The
  pose turns the pelvis bone (and what hangs on it: spine, arms, head, legs) by it about its own place **before** the IK, so
  the torso lags the bike's roll in a change of lean and settles on it in a held turn. The hands and feet are then brought to
  the grips and pegs by the two-bone IK as before.
- Braking and acceleration are in the physics: the spring's sag under the felt acceleration moves the displacement along the
  bike (3 cm of lag per g of acceleration for the KTM); the old rule of the acceleration over one g with a 6 cm forward shift
  under braking is gone.

Not done: the head's own inertia multipliers (`InertiaDisplacement_HeadMultiplier` pitch 5, roll -2: their unit was not
recovered), the sideways knee and leg-out layers, the user's own weight shift (`RiderWeightX/Y`), finger, wrist and bump layers.

## 8. What is data and what is a choice

| Item | Origin |
|---|---|
| Clip names per bike type and style, lean left/centre/right and standard/aero structure | Data (`SET Animations`, node graph) |
| Wheelie ramps 0.2/0.4/0.5, stoppie ramp 0..0.18 and 1.27 s, noise speeds 10/220/280 and alpha 0.4, `SpeedThresholdBasedOnBike` | Data (blueprint constants) |
| Seat, grip and foot socket transforms, bone lengths, pole bones | Data (bike and rider skeletons) |
| Poses within about 5 cm of the grips | Measured |
| Which of the three styles is the default (1) | Choice: the data assets of the styles do not carry the index |
| `_L` clips lean to the bike's left and `_R` to the right | Measured: over the 61 frames the head's mean lateral position in bike space is -4.07 cm (`LowSpeed_L`), +3.97 cm (`LowSpeed_R`), -3.26 / +3.14 cm (`HighSpeed`), centre pose -1.30 cm |
| Positive `lean` selects the `_R` clips (the blueprint's three-way blend pins are bound in cooked data), and the normalisation of `lean` | Choice |
| `tuck` shaped from `TuckedIn_Alpha` with the threshold 140 km/h | Choice: the native `TuckedIn` input is not read |
| The shake weight `ramp(10..220) * 0.4` | Choice: `AddNoise_Start/StartEnd/EndStart/End` and `Min/MaxAmount` give a more elaborate curve |
| The wheelie left/right blend by the lean | Choice |
| Two-bone IK in place of PBIK, and the IK target offsets | Choice |
| Inertia displacement: the physics' rider offset, held to the ranges back 6 cm, sideways 15 cm, down 4 / up 20 cm, following at 4 per second | Data (`InertiaDisplacement_RidingMin/Max`, `_SmoothSpeed`) and the physics of `FRiderComponent` (recovered, [ride6-rider-body-aero.md](ride6-rider-body-aero.md)); replaces the former choices (acceleration over one g, a 6 cm forward shift under braking) |
| Hang-off (the lateral shift with the lean or the lateral weight), the torso's lag behind the bike's roll | Physics ([B], `FRiderComponent` target place and angular spring); the pose applies them at the pelvis (the bone that carries the body) |
| `lean` of the graph = the rider's roll (the bike's plus his body's own) | Physics; the normalisation by the bike's steepest lean stays a choice |
| `wheelie`, `stoppie` = the nose pitch over the pitch at which the combined centre of gravity (with the rider's real place) stands over the rear (front) tyre patch, `atan(b / h)` (`atan(a / h)`), while that wheel is off the ground; the road slope is not taken off the pitch | Choice: published rigid-body statics from the bike data (`front_weight`, `wheelbase`, `cg_height`); the native `BikeWheelie` input is not read |
| Smoothing rates: lean 4/s, tuck 6/s, wheelie 3/s, stoppie 3/s | Choice for lean (the blueprint's 4, 2, 3, 4 are not tied to variables), tuck (the table above) and wheelie (the `Wheelie/ Stoppie` function smooths at 3); data for stoppie (speed 3) |
| Level of detail 0 of the suit, boots, helmet and visor; the arms mesh is not drawn | Choice (`parts.rs`, `LEVEL`; `parts.json` `drawn`) |
| The rider is drawn two-sided and with the engine's lit shading | Choice |
| Which node feeds which pin in the blueprint's graph (the pins are bound to variables in cooked binary data) | Not recovered; the structure above is read from the variable names |

## 9. Not done

- `BikeType` and `RidingStyle` of chassis 369: not found in the bike's data assets. `KTM_450-SMR-2022` has a camera
  container under `data/cameras/Containers/motard/`, which suggests type 2; the physics tuning assets do not name it.
- Eight ACL-compressed clips (standing wheelie, drift, entry; see the format note).
- The native fuzzy-logic layer of the animation instance (`RiderLean_Polished`, `TuckedIn_Alpha`, `BikeWheelie`, `BikeStoppie`
  and the smoothing of the native inputs): it lives in native code of the animation instance (`IgnitionMotorbikeRiderAnimInstance`),
  which was not decompiled in this round (the analysis instance was not available); the choices for them in section 8 (lean
  smoothing 4 per second, tuck 6, wheelie 3, the wheelie/stoppie triangle from the statics) stay. They are the next
  reverse-engineering target.
- Bump and imbalance layers, finger and wrist layers, pre-turn leg-out, knee/ground traces, look back, start/stop state
  machine and the get-on animation. The clips for them are in the pack.
- A check on a moving bike with the rider's physics in the loop: the unit tests of the sample, the smoothing and the pose pass,
  but nothing was rendered.
