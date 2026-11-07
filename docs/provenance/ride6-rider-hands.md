# RIDE 6 rider hands: provenance

What the hands work read, and from where. Spec: [ride6-rider-hands](../specs/ride6-rider-hands.md). Code:
`mods/ride6-bikes/src/rider/{graph_fingers,attach}.rs`, `crates/nfsmw/src/scenes/world/drive/rider/{hands,pose}.rs`,
`libs/blackbox-vehicle/src/bike/{powertrain,readout}.rs`.

## Sources

| Source | What was taken | Status |
|---|---|---|
| The animation blueprint of the user's install, printed as pseudo-code by CUE4Parse's Blueprint decompiler with the functions' bytecode (tool `bp` mode) | `Hands_FingersOverride` (the explicit times, the smoothing rates 2.5, 3.5, 7, 10, the weights of the finger layers, the call `(0, 0.28, 1, 0, 0.23, 2, 3)`), `HandsBehaviours_(Leaning,_UpAndDown)` (the five hand poses, `Hand_R_WristDown` from `VehicleThrottle`, the transform speed 4), the class defaults of `Hand_{L,R}_Wrist{Up,Down,In,Out}{Rot,Pos}`, the `BrakeFingers` switch of `SET Animations` | Facts about the user's data asset (Kismet bytecode), not native code. The numbers and the structure are kept; no pseudo-code is stored |
| The animation sequences `riderShared/Animations/race/race_clutch_brake_gear{,_OneFinger,_TwoFingers,_FourFingers}` and `race_Wrist_X` | The clips (20 frames of 0.633 s; the bones that move: the index and middle fingers of both hands, the ring and little fingers for four fingers) | The user's own export, nothing committed |
| The bike's sockets `SOCKET_{L,R}_HANDLE_POSE` (`contacts.json`) | The grip socket's z axis (the handlebar's axis) that the throttle's yaw turns about | Measured on three bikes |
| The game's own clutch model (`libs/blackbox-vehicle/src/bike/powertrain.rs`) | The clutch lever, as the handler pulls it | This project's code |

## What is a choice or an inference

Section 4 of the spec. The ones to remember: the lever of the clutch from the bike's own clutch (the blueprint remaps it), the mask of the finger layers, the
space of the wrist offset.
