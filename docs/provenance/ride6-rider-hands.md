# RIDE 6 rider hands: provenance

What the hands work read, and from where. Spec: [ride6-rider-hands](../specs/ride6-rider-hands.md). Code:
`mods/ride6-bikes/src/rider/{graph_fingers,attach}.rs`, `crates/nfsmw/src/scenes/world/drive/rider/{hands,pose}.rs`,
`libs/blackbox-vehicle/src/bike/{powertrain,readout}.rs`.

## Sources

| Source | What was taken | Status |
|---|---|---|
| The animation blueprint of the user's install, printed as pseudo-code by CUE4Parse's Blueprint decompiler with the functions' bytecode (tool `bp` mode) | `Hands_FingersOverride` (the explicit times, the smoothing rates 2.5, 3.5, 7, 10, the weights of the finger layers, the call `(0, 0.28, 1, 0, 0.23, 2, 3)`), `HandsBehaviours_(Leaning,_UpAndDown)` (the five hand poses, `Hand_R_WristDown` from `VehicleThrottle`, the transform speed 4), the class defaults of `Hand_{L,R}_Wrist{Up,Down,In,Out}{Rot,Pos}`, the `BrakeFingers` switch of `SET Animations` | Facts about the user's data asset (Kismet bytecode), not native code. The numbers and the structure are kept; no pseudo-code is stored |
| The animation sequences `riderShared/Animations/race/race_clutch_brake_gear{,_OneFinger,_TwoFingers,_FourFingers}` and `race_Wrist_X` | The clips (20 frames of 0.633 s; the bones that move: the index and middle fingers of both hands, the ring and little fingers for four fingers) | The user's own export, nothing committed |
| The bike's sockets `SOCKET_{L,R}_HANDLE_POSE` (`contacts.json`) | Where the older rule put the wrist; the throttle's roll no longer uses the socket's axis (the grip bone's own axis, below) | Measured on three bikes |
| The anim blueprint's `AnimGraphNode_LayeredBoneBlend` nodes (the package's JSON, CUE4Parse `json` mode) | The clutch layer's branch filters `index_01_l`, `middle_01_l` (two fingers); the brake layer's `index_01_r`, `middle_01_r`, `ring_01_r`, `pinky_01_r` | Facts about the user's data asset |
| The finger clips sampled with this project's own `blackbox-skeleton` (a throwaway reader, not committed) | Which bones move in each clip and by how much (spec section 1) | Measured |
| The grip mesh of four bikes' chassis (exporter glb, the vertices skinned to `BONE_R_GRIP`) against the bone in `parts.json` | The grip bone's z axis is the grip cylinder's axis, the bone's origin lies on it (spec section 2) | Measured on the CRF 450 R SMT, RC 390, Panigale V4 S, KTM 690 SMC R |
| The game's own clutch model (`libs/blackbox-vehicle/src/bike/{powertrain,clutch_control}.rs`) | The clutch lever, as the handler pulls it (`lever` before linearity and the slipper; the trapezoid of `shift_clutch`: why a change's pull is short) | This project's code |

## What is a choice or an inference

Section 4 of the spec. The ones to remember: the lever of the clutch from the bike's own clutch (the blueprint remaps it), the mask of the finger layers, the
space of the wrist offset.
