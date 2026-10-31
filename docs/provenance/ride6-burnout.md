# Provenance: RIDE 6 burnout (rounds 11 and 12)

Covers [ride6-burnout](../specs/ride6-burnout.md). Earlier rounds: [ride6-physics-A](ride6-physics-A.md) (the handler flags),
[ride6-physics-B](ride6-physics-B.md) (the slip-correction multiplier).

## What was used

- The user's own install of RIDE 6 (Milestone, Unreal Engine 5.6.1, build `5.6.1-1697004`), read-only. Nothing of it is in the repository;
  the spec states the rules and numbers in its own words.
- The physics data dump of the install (`physdump-all`: the eight input handler assets resolved over their Blueprint parents, and the
  319 vehicles), searched for every key with "Burnout" in its name; the values were counted over the handler objects.
- The Dumper-7 SDK dump for the field offsets of the handler (`UBikeInputHandler`: `BurnoutMinSpeed` `+0x258`, `bEnableBurnoutInNeutral`
  `+0x25c`, the flags `+0x25d` to `+0x266`, `EnableBurnoutHelpForce` `+0x600` and the six keys after it, `BurnoutGripMultiplier` `+0x62c`,
  `BurnoutSlipCorrectionMultiplier` `+0x630`; `UTCSVehicleElement`: `bTCSLowSpeedModulationForBurnout` `+0x48`).
- Decompiled functions of the shipping executable from the private headless copy of the project (the scratch folder
  `C:\r6\ghidra\out2`, never in the repository), and one disassembly of two tiny setters in a fresh copy of the project. Addresses are
  virtual (`0x14` + image-relative).

## Functions read

| Topic | Functions | How verified |
|---|---|---|
| The flags and `IsDoingBurnout` | `0x1450d7fb0` | read; the speed is `VehicleForwardSpeed` (`+0x1a08`) times `0x149e3fc18` = 0.036 (km/h from cm/s), the 0.1 threshold `0x148fbaccc`; the gear is the transmission's (`+0x2c` of the object at `+0x1778` of the vehicle) |
| `BurnoutMinSpeed` = 5, `bEnableBurnoutInNeutral` = false, `LowSpeedBrakeThreshold` = 10 | base constructor `0x1450bd6d0` (stores at `+0x250`, `+0x258`, `+0x25c`) | read |
| Burnout keys' constructor defaults | `0x1450bbf80` (stores at `+0x600` to `+0x634`; the 8-byte store at `+0x62c` of `0x3f800000` leaves `BurnoutSlipCorrectionMultiplier` at 0) | read |
| Wheel set-up: grip multipliers and the slip correction | `0x1450d7ed0` (calls `0x145168a50`, `0x145168a30`, stores `+0x308` of the wheel at `handler+0x1760`) | read; the two setters disassembled: `movss [rcx+0x460], xmm1` and `movss [rcx+0x464], xmm1`, the wheel's runtime longitudinal and lateral grip scales ([ride6-tyre-mf94](../specs/ride6-tyre-mf94.md), section 7). This settles the names that [ride6-physics-B](ride6-physics-B.md) left inferred |
| The launch clutch is skipped in a burnout | `0x1450d7cd0` (the branch on `+0x263` around the feathering call `0x1450c0290` and the plain ramp) | read |
| Traction control off in a burnout | `0x14512bd20`, `0x145110450`, `0x14513f3b0` (`+0x48` false and `+0x263` set: the intensity is 0) | read; the default of `+0x48` not found |
| Low-speed brake not in force while accelerating | `0x1450d76b0` (needs `IsAccelerating` false) | read |
| Burnout help force | `0x1450d5050` (called when `+0x600` and `+0x263`; otherwise it zeroes `+0xd68`, `+0xd70`), `0x1450bfa00` | round 12: read to the algorithm of [ride6-burnout](../specs/ride6-burnout.md) section 6. The vector arithmetic is quaternion sandwiches (`FTransform` rotation at vehicle `+0x15f0`, `0x148fb7020..38` the conjugate signs) and a dot product; the constants were read from the executable's sections (`0x148f950b0` = (0, 1), `0x148fbaccc` = 0.1, `0x148f96ec8` = 0.001, `0x148f96ed0` = 0.01745, `0x1490f5e08` = 0.05); the vehicle fields by the SDK dump (`OriginalActions.Steer` `+0x1798`, `Actions.Throttle` `+0x17dc`, `Actions.Clutch` `+0x17f0`, wheel `ContactForce` `+0x1b0`, engine `Limiter.MaxRPMScale` `+0xa0`, `FEngineData.MaxRPM` `+0x384`) and the instructions round the call of `0x1450ba6a0` (the value in `xmm1` is the engine's `EngineRPM`, `+0x30`) |
| Contact point and the road's up for the help force | `0x145157eb0`, `0x1450cd860` | `0x145157eb0`: wheel centre less the chassis' up times the radius. `0x1450cd860` writes `+0xd20..0xd30` (the road normal from the wheels' contact normals, up when flat) |
| Range percentage, force at a location | `0x1450ba6a0`, `0x1450ff5a0` | `0x1450ba6a0`: `(v - low) / (high - low)`, a step at `high` when the ends meet. `0x1450ff5a0`: torque = (point minus the vehicle's position) x force; the point of the help force is already relative, an oddity not explained and not copied |
| Collision cut | `0x1450dd9c0` (sets `+0xd74` = `BurnoutHelpReduction`, `+0xd78` = its duration), `0x1450e0480` (holds, then lowers `+0xd74` at `BurnoutHelpRaiseRate`) | read |
| `bTCSLowSpeedModulationForBurnout` default | `0x1450fe630` (the traction control element's constructor: byte `+0x48` stored 1; `TCSSpeedThresholds` `+0x50` default 15 and 50), `0x145110450`, `0x14513f3b0` (the use) | read; not in any of the 319 vehicles' data |
| Lean limit, help, balance and stability help in a burnout | `0x14510b180`, `0x1450c3ae0` and the help functions already in [ride6-chassis-round6](../specs/ride6-chassis-round6.md), [ride6-lean-controller](../specs/ride6-lean-controller.md) | read in earlier rounds; `BurnoutLeanLimit` is a key of the handler assets (18 on 436 objects, 10 on 9) |

## Not read

The animation blueprint's burnout pose (the rider's pose follows the lever samples as in any hard stop); why `0x1450ff5a0` subtracts the
vehicle's position from a point that `0x1450bfa00` already made relative; `TerrainNormalBlendFactor`'s value (not in the data); the vehicle
inertia at `+0x1d20` is taken to be the diagonal inertia (a guess from its use, [I]). No statement here was tested in the running game
(the game must not be launched): the burnout's behaviour in the model is the model's.

Decompiling in round 12: a copy of a private headless project (`C:6\ghidra\proj-G`), the decompiler script `R6HDecomp.java`, and a small
script that reads the executable's sections for constants and for the call sites of a displacement (no Ghidra analysis was run).

## Rust

`libs/blackbox-vehicle/src/bike/`: `burnout.rs` (the flag, the numbers, the bike's part of a step, round 12: the help force, the lean limit
and traction control hooks; `lean.rs`, `handling.rs` and `mod.rs` call them), `drive_input.rs` (moved out of
`powertrain.rs`, with the burnout flag), `powertrain.rs` (launch clutch, over-limit change, over-rev cut), `tyre.rs` and `wheel_step.rs`
(the rear tyre's factors, the slip speed), `wheel.rs` (`slip_speed`), `choices.rs` (`OVERREV_SHARE`), `rider_inputs.rs` (the rider's
setters, moved out of `mod.rs` to keep it under 500 lines); the manifest table in `libs/blackbox-modding/src/content/bike_burnout.rs`;
the importer in `mods/ride6-bikes/src/physics/burnout.rs`; the host's wheel readout in `crates/nfsmw/src/scenes/world/drive/sim/bike.rs`;
the sweep in `libs/blackbox-modding/src/content/bike_ride_burnout.rs`.
