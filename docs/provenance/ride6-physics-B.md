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

## Round 7

Spec: [ride6-chassis-round7](../specs/ride6-chassis-round7.md). Same method (private headless copy, exception-table lookups).

| Topic | Functions | How verified |
|---|---|---|
| Sub-steps of a vehicle step | `0x145147070` (pre-tick, the division of `dt`, post-tick); the two defaults at `0x14bcd2320` / `0x14bcd2324` are -1 | `ceil(dt / MaxDeltaTime)` held to `MaxSubsteps`; the chassis' `+0xF4`, `+0xF8` read through the vehicle's chassis pointer |
| Stability help: desired yaw rate and application | `0x1450c6560`; the tan argument traced in the instructions (lean blended by the input factor, terrain shift, clamp), `|g|` the length of the body's gravity vector, speed `+0x1740`; add-torque `0x1450ff5f0` | disassembly of `0x1450c6860` to `0x1450c68fc` |
| Terrain normal, slope, inclination | `0x1450cd860`; consumers `0x14510b180` (lean limit), `0x1450ca0a0` (fall tests) | structure read; sign fixed by the two consumers' use (see the spec) |
| Constructor defaults used | `0x1450bbf80` (smoothing 1, min normal z -1, blend 1), `0x1450fbfa0` (terrain max 90, scales 1, stability range 10 to 15, max lean 75) | |
| Low-speed constraints | `0x145101530` (sets the stopped state and overwrites the velocity), `0x14514a920`, `0x145130520` / `0x145132820` / `0x14512ed70` (readers of the flags) | the state needs a brake above 0.1; it zeroes the wheel spin |

Data: the chassis block of the KTM asset (`StoppedSpeedThreshold` 10, `StoppedAngularSpeedThreshold` 0.2, `StoppedProjAngularSpeedThreshold` 0.05,
`LowSpeedConstraintWheelNearGroundThreshold` 2, `VelocitySolverIterationCount` 16; no `MaxDeltaTime`) and the stability, terrain and fall
fields of the eight handler assets. Not found: the asset holding `FPhysicsSimSettings` (class `PhysicsSimulationSettings`).

Rust: `mod.rs` (`respond` / `begin_body` sub-steps), `choices.rs` (`MAX_SUBSTEP`), `handling.rs` (the moment, the stability torque),
`lean_help.rs`, `lean_spec.rs` (`StabilitySpec`, `TerrainLimit`), `fall.rs` (`terrain_slope`), `lean.rs` (terrain term), manifest keys
`stability_*`, `terrain_*`, `fall.terrain_*`; importer `steering.rs`.

## Round 8 (what makes the bikes rideable)

Spec: [ride6-feel-round8.md](../specs/ride6-feel-round8.md). Build `5.6.1-1697004`, image-relative addresses.

| Topic | Functions | How verified |
|---|---|---|
| `LateralGripPeak` and `LongitudinalGripPeak` of the handler, written every tick | the tick `0x1450e0480` (at `0x1450e0625` and `0x1450e0635`) calls `0x14514e0a0` (longitudinal) and `0x14514d640` (lateral) with the rest load of the front wheel (`RestLoad` at handler `+0xD40`) and camber 0 | disassembly; both getters blend the dry and the wet tyre model by the wetness and multiply by `LateralGripScale` (wheel data `+0x104`) or `LongitudinalGripScale` (`+0x100`) |
| Fall lean limit target | `0x145106701`: `atan(LateralGripPeak) x wheel data +0xF0 x 57.2958` (constant at `0x148fbaf50`), then the running value (`+0x650`, `FallLeanLimit`) moves by `FallLeanLimitUpFactor` (`+0x1E20`) or `...DownFactor` (`+0x1E24`) of the gap | disassembly; the offsets are the SDK dump's: `+0xF0` is `LateralGripLimit`, `+0xEC` `LateralGripTolerance` |
| Lean limit base | `0x14510b19e` to `0x14510b2c3`: `atan(LateralGripPeak) x wheel data +0xEC` | disassembly |
| Feathering start | `0x1450c0290` (the controller, re-read: `R = engine +0x384 x a + engine +0x50 x b`, `+0x384` `MaxRPM`, `+0x50` `MaxTorqueRPM`), `0x1450d7cd0` (the caller: the start ends when the whole clutch input is exactly 0), `0x1450d7000` (the automatic path: no change of gear while the start lasts) | decompilation and the SDK dump's names |

## Round 9 (the stick at its end)

Spec: [ride6-feel-round9.md](../specs/ride6-feel-round9.md). Build `5.6.1-1697004`, image-relative addresses; headless Ghidra on a copy of the project.

| Topic | Functions | How verified |
|---|---|---|
| Wanted front force divided by the cosine of the lean | `0x1450c8f70` (stores `-tau / COMHeight` in `+0xC60`, leaves it in `xmm1`), `0x1450c9320` (divides it by `max(dot(chassis right, normalised right projected on the ground plane), 0.05)`) | disassembly of both: the `XORPS` that negates `xmm1` runs before the store and the call |
| Handlebar projected along the ground normal | `0x1450c9320` (rotates the heading and the normal `N` of the front contact into the chassis frame by the inverse of the chassis quaternion), `0x145158890` (`h - (h_z / N_z) N`, then `atan2`) | the second argument of the last call is the rotated ground normal, not the steering axis |
| Front velocity point | `0x145157eb0` (hub position less the wheel radius along the chassis' up), `0x145124310` (the rigid body's velocity at a point) | read |
| Tyre load | `0x145140b30` (after the suspension force `0x14514cc40`: the force is replaced by `n_s (n_s . F) / (n_s . axis)^2`, `n_s` the contact normal without its chassis-lateral part; the dot product is clamped at -0.5 by `MINSD` with the constant at `0x148fa8c58`, `-0.5`), the load stored by `0x145169380` into `+0x154` | disassembly of the call site `0x145141cfc`; the tyre call sites `0x14516f021`, `0x14516f045` read `+0x154` (not `+0x158`) |
| The correction of the tangential suspension force at the contact | `0x145140b30` (`+0x1c8 = tyre force - (F_s - N (F_s . N))`) | read, effect not modelled |
| Lean help neutral blend | `0x1450c3ae0` (end: `+0x4a8` to `+0x4d0` blend the point of application), force zero without the front contact (`+0xd1c`) | read |
| Handlebar filter | `0x1450f9020` (coefficients), `0x1450f9120` (step) | read: the model's filter matches |
| Camber | `0x145150050`: `asin(chassis right . N) + static terms` | read, not changed |

## Round 11 (the flicked stick with the throttle open, the g-force of the lean limit)

Spec: [ride6-feel-round11.md](../specs/ride6-feel-round11.md). Build `5.6.1-1697004`, image-relative addresses; headless Ghidra on a private copy of the project
(`proj-B11`), disassembly of `0x50c3ae0` to `0x50c3d40` (`R6Disasm`), decompilation of the IMU update (`R6HDecomp`), byte search of the executable for the
displacements `0x1850` and `0x1858`.

| Topic | Functions | How verified |
|---|---|---|
| Load the lean help's limit is worked out at | `0x1450c3ae0`, `0x1450c3b6f` to `0x1450c3beb`: with the front contact flag (wheel `+0x28`) set, the load is `max(wheel +0x154, RestLoad +0xd40)` (`RestLoad` is a pair of doubles, front first); without it, `(1 - s x k) x RestLoad.x`, `s` from the chassis' `+0x1a28` through the range at handler `+0x4e8` (or `+0x1a2c` when the range is unset), `k` at `+0x4f8` | disassembly: `MOVSS XMM0,[RBX+0x154]`, `MAXSD XMM0,[RCX+0xd40]`; the else branch calls `0x1450ba6a0` and `MULSD XMM0,[RDI+0xd40]`; the result times `[RDI+0xd5c]` is the argument of `0x145158200` (longitudinal peak) and `0x145158120` (lateral peak) |
| The limit of the help | `0x1450c3d0b`: `cap = LeanHelpLimit (+0x4fc) x peakY x sqrt(1 - (|F_rest| x ... / peakX)^2)` where `F_rest` is the tyre force without its side part; applied only when `|actual| < |ideal|` | read; matches the model's formula, which used the load of the moment |
| Force share without a front contact | end of `0x1450c3ae0`: `+0xd1c` (the balance) is set to 0 when `[front +0x28] == 0`; the force acts at the contact with that share, and `(1 - share) x (point - CG) x force` is added as a torque (`0x1450ff5f0`) | read: nothing returns early on a missing contact, so the help goes on as a pure couple |
| IMU g-force | `0x14513c270` (and its chunks `0x14513c2cc`, `0x14513c78e`): chassis `+0x1850` = `-(a . R(0,1,0)) / |g|`, `+0x1858` = `(a . R(1,0,0)) / |g|`, `R` the chassis quaternion at `+0x15f0`, `a` from `0x1450d6660(chassis +0x1af0)`, `|g|` the length of the vector at `+0x1d00` | decompilation; the two constant vectors read from the image (`0x148f950b0` = (0, 1), `0x148fbb060` = (1, 0)); the chassis frame is x forward, y right, z up (round 9, `0x1450c9320`) |
| Reader of the g-force in the lean limit | `0x14510b180`: `LeanLimitGForceTuning` (`+0x1af8`) is evaluated at `*(double*)(chassis + 0x1858)`, the absolute value when `+0x1ba0` is set | decompilation (lines around the second translator of the function) |
| Peak lean limit curves | `LeanLimitGForceTuning*.json`: default 1 at 0 g to 0.825 at 2 g, baggers 0.87 at 1 g, supermotard 0.9 at 1.1 g | data |

Not settled: the default of the absolute-value flag at `+0x1ba0` (the port takes the absolute value: braking takes lean limit like driving); the meaning of the
flag at wheel `+0x28` is read as "the tyre touches" (the stability help uses `+0x29` for "the ray hit is known"); the values of the handler's `+0x4e8` range and
`+0x4f8` (constructor defaults, not read), so the fade of the help with the front wheel's ground distance is not modelled.

Rust: `libs/blackbox-vehicle/src/bike/handling.rs` (`apply_lean_help`, the acceleration kept by `shift_weight`), `lean.rs` (`LeanInput::long_g`), `choices.rs`
(`STICK_SHARE_AT_TOP`), `round11_tests.rs`; harness `libs/blackbox-modding/src/content/bike_ride_flick.rs`, `bike_ride_radius.rs`; host test `drive/sim/bike_ride_tests.rs`.


## Wheelie steer (the stick with the front wheel up)

Spec: [ride6-wheelie-steer.md](../specs/ride6-wheelie-steer.md). Build `5.6.1-1697004`, image-relative addresses; the decompilations of an earlier run of `R6HDecomp`
on a copy of the project (`C:\r6\ghidra\out2`: `510b180`, `50cd720`, `5107c80`, `510d620`, `50f7680`, `50c8f70`, `50fb230`), the data constants read from the image
(`148fa8bd0` = 1e-8, `148f94f1c` = 1.0, `148fa8c50` = -1.0, `148f96ed0` = pi/180, `148fbaccc` = 0.1, `148fbaf50` = 180/pi, `14bc01268` = a pair of zeroes) and
the physics dump (`InputHandler/*.json`).

| Topic | Functions | How verified |
|---|---|---|
| Wheelie and stoppie lean limit | end of `0x14510b180`: `limit = F(handler +0xa50, front wheel)` then `F(+0xa28, rear wheel)`, floored at `148fbaccc` (0.1 degree); `F` = `0x1450cd720` | `F` returns the limit unchanged when the tuning's `LeanLimit` (+0) is 0 or less; otherwise `lerp(limit, LeanLimit, ramp(angle; AngleRange))`, `angle` = `max(0, (CurrentWheelieAngle (+0xc58) - terrain x influence) x sign) x 180/pi` while the wheel's contact flag (+0x28) is clear, 0 with it set; sign +1 or -1 by the call `0x14515ca60` on the wheel (-1 is the constant at `148fa8c50`) |
| Mid-air steer: weight | `0x1450f7680` (component at handler +0x1588; weight at +0x1c8, angle at +0x1cc) | product of three ramps: the front wheel's gap (`wheel +0x28` less `+0xc0`, at least 0) through +8/+0x10; `1 - ramp(abs(chassis +0x183c); +0x20/+0x28)` when +0x18 is set; `ramp(chassis +0x1828; +0x38/+0x40)` when +0x30 is set |
| Mid-air steer: angle | same function: `clamp(SteerInputTuning(chassis +0x1798) + LateralWeightInputTuning(chassis +0x17b8), -1, 1) x steer lock of the front wheel x pi/180` | the stick is the lean controller's own (`0x14510b180` tests its sign for the terrain term); the weight and the angle are 0 and 0 when the component is disabled |
| Mid-air steer: use | `0x1450c8f70`, after `0x1450c9320`: `angle = lerp(controller angle, +0x1754, +0x1750)` when `0x1450f7680` says the weight is over 0 | read |
| Lean help factor of a wheelie | `0x145107c80`: front gap over the wheelie range's end (+0x28b0, default `(0, 0)`) gives `LeanHelpFactorWheelie` (+0x26e4) as the whole factor; the blend towards it runs only for a range other than the default pair (`14bc01268`); then the blend towards `LeanHelpFactorAirborne` (+0x26e0) by the chassis' distance from the ground (+0x1a28) through +0x2898/+0x28a0 | read; constructors `0x50fb230`, `0x50fbfa0` store 1.0 at +0x26e4 and +0x26e0 |
| Stability help factor in a wheelie | `0x14510d620`: the same blend towards `StabilityHelpFactorWheelie` (+0x2e4c) with the lean help's wheelie range and **no** skip for the default pair; the constructor stores 0 there (an eight-byte store at +0x2e48: 0.1f and 0.0f) | read; with the range `(0, 0)` the blend weight is 1 whenever the gap is 0 or more, so the factor reads 0 in every state - not changed, see the spec's open items |
| Data | `MidAirSteerComponent` identical in the eight input handlers (`GroundDistanceRange` 4.0 to 4.1, `NormalizedLeanRange` 0.4 to 0.41, `PitchSpeedRange` -0.4 to -0.3, `SteerInputTuning` -1..1 to 0.8..-0.8); `WheelieLeanLimitTuning` {`LeanLimit` 1, `AngleRange` 0 to 40 (Motard; 0 to 45 and 0 to 40 in the 288 manifests)}; no handler sets a wheelie distance range or a wheelie factor | `physdump-all/resolved/.../InputHandler/*.json` |

Not settled: the polarity of the handlebar against the stick in the mid-air steer (read as positive-right for both, which makes the data a counter-steer); the sign of the pitch
speed at `chassis +0x1828`; the unit of the gap (read as centimetres, as the handler's other distances); the unit of the wheelie lean limit (degrees, as the stopped limit).

Rust: `libs/blackbox-vehicle/src/bike/wheelie_steer.rs` (`mid_air_intensity`, `mid_air_angle`, `LEAN_HELP_FACTOR_WHEELIE`, `Bike::apply_wheelie_yaw`), `handling.rs`
(`steer_handlebar`, `apply_lean_help`, the cut in `steer_handlebar`), `wheel.rs`/`wheel_step.rs` (`WheelState::gap`), `choices.rs` (`WHEELIE_LEAN_FLOOR`, `WHEELIE_YAW_FACTOR`),
`wheelie_steer_tests.rs`; harness `libs/blackbox-modding/src/content/bike_ride_wheelie_steer.rs`; host tests `drive/sim/bike_ride_tests.rs`.
