# RIDE 6 rider: provenance

Records what the rider work read, and from where. Spec: [ride6-rider-animation.md](../specs/ride6-rider-animation.md).
Formats: [ride6-rider.md](../formats/ride6-rider.md). Code: `libs/blackbox-skeleton` (generic, written from published maths),
`mods/ride6-bikes/src/rider/` (RIDE 6 specific converter).

## Sources

| Source | What was taken | Status |
|---|---|---|
| The user's own RIDE 6 install (`D:\RIDE 6`, Unreal 5.6.1, build `5.6.1-1697004`), read with CUE4Parse | Asset *facts*: bone names and reference pose, socket transforms, the names of the animation sequences and which are used for a motard, the animation blueprint's class defaults (numbers listed below), the blueprint's node graph (shape only), blueprint functions as pseudo-code printed by CUE4Parse's Blueprint decompiler (Kismet bytecode of the user's own data asset, not native code) | Facts about the user's data. Not stored in the repository: the meshes, pictures, animation samples and skeleton are converted into the mod's git-ignored `files/` folder only |
| Dumper-7 SDK dump of the same install (`C:\Dumper-7\...`) | Class and field names of `GameRiderAnimInstance`, `IgnitionRiderAnimInstance`, `IgnitionMotorbikeRiderAnimInstance` (the inputs table of the spec) | Names and offsets only; no code |
| RIDE 6 native binary (`ride6-Win64-Shipping.exe`) | Not read in this pass. The user's Ghidra instance (project `nfsmw`, program `ride6-Win64-Shipping.exe`) was attached and left untouched | n/a |
| Published maths | Quaternion slerp, linear-blend skinning, the law-of-cosines two-bone solve, bilinear blend-space weights | Public |

The data tool is CUE4Parse (Apache-2.0), used as a scratch program outside the repository.

## Numbers taken from the animation blueprint (class defaults)

`SpeedThresholdBasedOnBike` 210 / 160 / 140 (set in function `SpeedThresholdTuck-In` per bike type 0 / 1 / 2);
`Wheelie/ Stoppie`: entry ramp 0.2..0.4, pose ramp 0.2..0.5, `BikeStoppie` rule 0 / 0.18, explicit time factor 1.27 s,
smoothing 2, 8, 3, 4, 3, flashback 0.4 and 0.15; `Wheelie_Delay` 0.2; `AddNoise_Start` 10, `AddNoise_StartEnd` 220,
`AddNoise_EndStart` 280, `AddNoise_End` 400, `AddNoise_MinAmount` 0.4, `AddNoise_MaxAmount` 0.75,
`Riding_AdditiveNoise_Alpha` 0.4; `RiderLean_Min/Max_DecaySpeed` 250 / 400, `RiderLean_Decay_Multiplier` 0.3,
`RiderLean_multiplier` 0.5, `Rider_UserLeanAddition_Amount` 0.4; `PreTurn_LowSpeed_to_HighSpeed_Min/Max` 120 / 150;
`Apex_*`; `Bump_*`; `InertiaDisplacement_*`; `StartStop_Speed_Treshold` 6; `StartClip_Length` 3.3; `StopClip_Length` 2.9;
hand and foot IK offsets. All appear in the spec only where used; the rest stay in the user's data.

## What is a choice, not data

See the table in section 8 of the spec. The main ones: the default riding style, the sign and normalisation of the lean
parameter, the tuck shape, the shake weight curve, the wheelie side blend, and two-bone IK instead of the game's PBIK.

## Not done and why

- Eight sequences are compressed with ACL and need the native `CUE4Parse-Natives` library; its sources are a download the
  user has not approved. The standing wheelie, the drift poses and the wheelie entry are therefore missing.
- The binary's native animation code (fuzzy-logic script library, lean and tuck inputs, `BikeType` and `RidingStyle`
  assignment) was not decompiled. If the values are needed, the entry points to start from in the user's Ghidra project
  are the functions behind `UGameRiderAnimInstance` and `UIgnitionMotorbikeRiderAnimInstance` (addresses from the SDK
  dump's `IDAMappings`).
