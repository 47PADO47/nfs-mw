# Provenance: RIDE 6 chassis, lean and suspension, round 6 (series B)

Covers [ride6-chassis-round6](../specs/ride6-chassis-round6.md) and the corrections it makes to
[ride6-lean-controller](../specs/ride6-lean-controller.md), [ride6-suspension](../specs/ride6-suspension.md) and
[ride6-tyre-thermal](../specs/ride6-tyre-thermal.md). Earlier rounds: [ride6-physics.md](ride6-physics.md).

## What was used

- The user's own install of RIDE 6 (Milestone, Unreal Engine 5.6.1, build `5.6.1-1697004`), read-only. No game file, asset or
  decompiler output is in the repository; the specs state algorithms and numbers in our own words.
- The shipping executable `ride6-Win64-Shipping.exe`, decompiled in a private headless Ghidra copy of the analysed project (a scratch
  folder; the project's listing held no instructions for many addresses, so the functions were located with the executable's own exception tables and decompiled on demand).
  Functions were found by the vtable of the input handler classes (names from the Dumper-7 mapping), by the field offsets of the
  Dumper-7 SDK dump (a scan of the executable for the 32-bit displacements, mapped to functions with the exception tables, chained
  unwind records followed to the primary function) and by call sites. Addresses are virtual (`0x14` + image-relative).
- The data assets, read with the scratch CUE4Parse tool: the eight input handler assets, the track temperature data table, the
  `LeanHelpFactorSkidInputInfluence` curve.
- Hand-decoded constants from the read-only data section (single and double floats).

## Layout fact that cost time

The SDK dump's offsets of the road-bike handler (`UMotorbikeInputHandler`) are those of its constructor `0x1450fbfa0`; the
motocross handler (`UMXBikeInputHandler`, constructor `0x1450fb230`) has other offsets for the same fields (for example
`bOverrideLeanHelpFactor` at 0x2174 instead of 0x213c). A function in `0x145106000` to `0x14510f000` can be either class's: the
road one reads the dump's offsets. The base class `UBikeInputHandler` is shared (offsets below 0x1860 agree).

## Functions read

| Topic | Functions | How verified |
|---|---|---|
| Consumers of `CurrentLeanAcceleration` and `DesiredLeanTorque` | the only writer is `0x1450c8f70`; a scan of the module for the displacements finds readers only in the serialisers, the reset and the debug drawing `0x1450d23a0` | no function applies the torque |
| Lean help force, balance fade, torque form | `0x1450c3ae0`: the constants `0.01` (cm/s to m/s), `3.0` and the range `(1.0, 3.0)` at `0x149e3fca0` read; `0x1450ff5a0` (add force at a point) and `0x1450ff5f0` (add torque) are the two sinks | the cross product `(P - COM) x F (1 - B)` followed term by term |
| Cap and traction factor | `0x1450c3ae0`, helpers `0x145158200` / `0x145158120` (peak forces times the wheel grip scales), virtual `0x1450cc940` (looks `LeanHelpLimitFrontTractionFactor` up by surface, clamps 0 to 1) | |
| Road-bike help factor | `0x145108e70` | reductions paired with `Actions.Throttle` (0x17dc), `Actions.Brake` (0x17e0), the wheels' `CombinedSlip` (0x190) and the `...InputInfluence` runtime curves of `Throttle - Brake`; stopped blend, airborne blend, off-track addition, anti-fall factor |
| Motocross help factor (same algorithm, other offsets) | `0x145107c80` | the handbrake lever pairs with a third reduction there |
| Road-bike balance | `0x1451079c0` (the motocross version is `0x145107600`) | |
| Road-bike help limit | `0x14510a0a0` (motocross `0x145109e20`) | |
| Stability help factor and torque | factor `0x14510e810` (motocross `0x14510d620`), torque `0x1450c6560` | same pairing as the help factor; `omega_des` is computed with a `tan` whose argument is a register not shown, hence inferred |
| Fall tests | `0x1450ca0a0`; ground lean limit update `0x1451066e0` (motocross `0x145106380`) | every field offset matched to the SDK names; the type codes read from the stores to the result |
| Fall defaults | constructor `0x1450bbf80`: `FallLeanLimit` 65, `FallPitchLimit` 70, `FallSpeedDirAngleLimit` 50, `FallHighsideLeanLimit` 30, `BlockFallLeanLimitMaxDeg` 90, `FallMaxDistanceFromGround` 0, `FallSpeedDirAngleMinSpeed` 0 | |
| Terrain normal, slope and inclination | `0x1450cd860` | the axis maths read; the sign convention not pinned |
| IMU (wheelie angle, g-force) | `0x14513c270` | structure only |
| Suspension step and the end of the travel | `0x14515aaf0` (integration), `0x14514f3e0` (ground contact and clamp), `0x14514cc40` (force assembly) | the clamp sets the speed to 0; the stored overshoot has no reader that makes a force |
| Brake coupling sign | wheel and drivetrain update `0x14515ac20`: `AppliedTorque` at `0x1a0`, `BackTorque` at `0x1a4`, `ResultingTorqueOnChassis` at `0x1a8`; constants `-1.0` (`0x148fa8c50`) | sign of `T_applied - T_back` = minus (brake + drag) for a wheel turning forward |
| Slip-correction multiplier | the handler's wheel set-up `0x1450d7ed0`: writes the rear wheel's `0x308` | 1.0 outside a burnout |
| Vehicle temperature fields | the setter `0x14607dd50` writes `AsphaltTemperature` (`+0x1a18`) then `AirTemperature` (`+0x1a14`); its callers are the race set-up `0x1463c6e90` (the weather state's `+0x358`, `+0x354`) and the weather broadcast `0x1464016e0` | the only writers outside the physics module |

## Data read

- `ride6/Content/assets/data/datatables/TrackTemperatureData_DT`: 298 rows (51 tracks, 7 weather names), the medians of the specs.
- The handler assets `Default`, `Motard` (the KTM's), `Naked`, `Baggers`, `Maxienduro`, `2502T`, `Scooter`, `ScooterGear` `_InputHandler`:
  the tables of section 2.1 and 3 of the spec.
- `PhysicsRide6/Physics/TuningCurve/LeanHelpFactorSkidInputInfluence`: keys `-0.1 -> 0`, `0 -> 1`.

## Not verified

Everything tagged **[I]** in the spec; the sign convention of `TerrainSlope`; the weather module's interpolation over the time of day (the table
rows are read, the module is not); that the stability help's `omega_des` is `g tan(lean) / v`; the names of the two wheel setters that take the burnout
grip multiplier. No statement here was tested in the running game (the game must not be launched).

## Rust

`libs/blackbox-vehicle/src/bike/`: `lean_help.rs`, `lean_spec.rs`, `fall.rs`, `chassis_state.rs`, `handling.rs` (the help and its
force or couple), `thermal.rs` (the day), `wheel_step.rs` (no stiff stop), `suspension.rs` (the sign), `tyre.rs` (the multiplier);
manifest keys in `libs/blackbox-modding/src/content/bike_parts.rs`; importer in `mods/ride6-bikes/src/physics/steering.rs`.
