# Provenance: RIDE 6 two-wheel physics specs

Covers [ride6-lean-controller](../specs/ride6-lean-controller.md), [ride6-tyre](../specs/ride6-tyre.md),
[ride6-suspension](../specs/ride6-suspension.md), [ride6-engine-drivetrain](../specs/ride6-engine-drivetrain.md),
[ride6-rider-help](../specs/ride6-rider-help.md), [ride6-rider-body-aero](../specs/ride6-rider-body-aero.md) and
[vehicle-two-wheel-parameters](../specs/vehicle-two-wheel-parameters.md), plus the format pages
[ride6-physics-data](../formats/ride6-physics-data.md), [-vehicle](../formats/ride6-physics-vehicle.md),
[-tyre](../formats/ride6-physics-tyre.md), [-elements](../formats/ride6-physics-elements.md).

## What was used

- The user's own install of RIDE 6 (Milestone, Unreal Engine 5.6.1, build `5.6.1-1697004`), read-only. No game file, asset or
  decompiler output is in the repository; the specs state algorithms and numbers in our own words.
- The data assets (`PhysicsRide6` plugin and the rider/animation tables) read with a CUE4Parse based scratch tool. The
  scratch tool resolves Blueprint inheritance and dumps 1180 packages and 319 vehicles as JSON (method in the data page).
- The Unreal reflection dump of the install (Dumper-7): class and struct layouts of `ignitionphysics` (field names, types,
  offsets) and the table of vtable and `UFunction` addresses (`IDAMappings`). It gives names, no code.
- The shipping executable `ride6-Win64-Shipping.exe` analysed in Ghidra 12.1 (read-only; the decompiler output was read, never
  stored in the repository). Functions were found by vtable (IDA mapping), by field offsets from the SDK dump and by call
  chains; their addresses are image-relative (image base `0x140000000`).

## Functions analysed (virtual address in the image)

| Topic | Functions | How verified |
|---|---|---|
| Second-order filter (`K`, `Damp`) | `0x1450f9020` (coefficients), `0x1450f9120` (step), `0x1450f8da0` (acceleration) | the constants read from the data section: the damping ratio branch `c = 2 sqrt(K) z`, exact analytic step |
| Steer to target lean | `0x1450d8f10` | constants `0.0174533` (deg to rad), `0.99`, `0.036` read; branch structure followed line by line |
| Lean torque and desired front force | `0x1450c8f70` | the fields `0xC5C`, `0xC60`, `0xDD4`, `0x10A4`, `0x420`, `0xF18` identified with the SDK offsets (`DesiredLeanTorque`, `DesiredFrontSideForce`, `COMHeight`, `Rider.Mass`, `RiderMassScale`, `Rider.COMOffset.z`) |
| Handlebar angle and filter | `0x1450c90d0` (read), `0x1450c9320` (geometry, only the structure) | virtual slot 148 of `UBikeInputHandler` is `0x1450c8f70` |
| Lean limit | `0x14510b180` | the fields `LeanLimitGForceTuning`, `LeanLimitSkid*`, `StoppedLeanLimit` named by the SDK offsets; sequence read, a few combination details inferred |
| Gyroscopic roll | `0x1450c7270` | fields `GyroscopicRollMomentumScale` (`0x5B8`), wheel inertia (`FWheelData + 0x28`), yaw rate (`+0x19E8`); unknown factor `c` |
| Rigid body mass and inertia | `0x145140350` | `InertiaTensorScale` times total mass times 10000; `SprungMass` |
| Wheel forces | `0x14514ce70` plus `0x14514e530`, `0x14514dad0`, `0x14514df60`, `0x14514d4f0`, `0x14514c1e0` | slip, normalised slip, combination, efficiency read line by line; the model object's virtuals were identified by their call sites (`0x48`, `0x50`, `0x58`, `0xC8`, `0xD0`) but the model's own code was not read |
| Tyre compression split | `0x14514f3e0` | `TireStiffness`, `MaxTireDeformation` offsets (`FWheelData + 0x11C`, `0x120`) |
| Spring law and leverage | `0x145152de0`, `0x145152440`, `0x145151610` | formula read; `SpringStrength` ... `ProgressiveHydraulicComponentLinearity` offsets matched; the unit factor 100 on the top-out length read |
| Suspension step with wheel mass | `0x14515aaf0` | |
| Engine | `0x14514c850`, `0x1450f1b50` | torque blend, drag curve (`EngineFrictionPeakTorque` times 10000), limiter conditions |
| Clutch and drivetrain | `0x14515ac20` | momentum model, slipper clutch and `ClutchLinearity` power read; `ClutchMaxTorque` helper `0x145157d80` not read |
| Joined brakes | `0x1450e0480` | input channels `Brake` and `HandBrake` of the action struct |
| ABS | `0x1450e0240` | per-wheel intensities and fall rates; the ramp functions `0x1450cac70`, `0x1450cb670` not read |
| Wheelie/stoppie control | `0x145136ea0`, `0x145122dc0`, `0x145100f80`, `0x145137060` | input mapping read; the torque application only followed |
| `UTireData` defaults | `0x145146810` | constructor stores |
| `UBikeInputHandler` defaults | `0x1450bbf80` | constructor stores matched to the SDK offsets |

## Round 2 additions

| Topic | Functions |
|---|---|
| Brake setup, application, disc temperature | `0x145136640`, `0x145134de0`, `0x14513fe00` (joined brakes in round 1: `0x1450e0480`) |
| Damper, spring force assembly | `0x145150560`, `0x145171f60`, `0x145153440` (existence only) |
| Constructor defaults | `0x1450fa2a0` suspension, `0x1450fa6a0` wheel, `0x1450f9ff0` engine, `0x1450fa5f0` transmission, `0x1450f9d40` brake, `0x1450bb350` rider, `0x145146810` tyre data |
| Gear tables and launch clutch | `0x1450f2c70`, `0x1450d7cd0`, `0x145157d80` (clutch max torque) |
| Rolling resistance, water drag | `0x14514e7f0` |

Round 2 used a private headless Ghidra copy of the executable (a project in the scratch folder) for new decompilations, read-only against the user's GUI instance.

## Round 3 additions (lean controller and body control)

| Topic | Functions |
|---|---|
| Desired force, slip angle, handlebar direction | `0x1450c9320`, `0x1451547c0` (inverse wrapper), `0x1451546e0`, `0x14515c560` (root finder), `0x145158890` (projection angle), `0x1450ba6a0` (ramp) |
| Magic Formula 94 evaluation | `0x145151960` (lateral coefficients), `0x145151aa0` (longitudinal), `0x145154a70` (curve), `0x145152360`, `0x1451578b0` (peak search); the model's vtable at image offset `0x9e43448` (slots `0x48` Fx, `0x50` Fy, `0x58` Mz, `0x60` rolling, `0xB0` inverse) |
| Lean limit order | `0x14510b180` |
| Lean help force | `0x1450c3ae0`, helpers `0x145158200`, `0x145158120`, `0x1450cb320`, `0x1450cb5c0` (range translators) |
| Stability help | `0x14510e810` (factor), `0x1450c6560` (acceleration and torque) |
| Gyroscopic | `0x1450c7270` (roll; the cosine argument is the lean angle, read from the instructions), `0x1450c4c00` (yaw) |
| Wheelie help | `0x145100f80`, `0x145137060`, `0x1450b3fa0` (parameter defaults) |
| Rider help weight control, defaults | `0x1450ff610`, `0x1450fda20` |
| Swingarm forces | `0x145153440`, caller `0x14514cc40` (passes the transmission's `TorqueFromEngine`) |

The math library stubs were identified by name (`0x148dadad3` cosf, `0x148dadb21` tanf, `0x148dadb6f` tanhf, `0x148dada7f` powf,
`0x148dadb03` expf, `0x148dadb4b` atanf, `0x148dadadf` sinf). Instruction-level checks used a disassembler on the raw bytes
(the cosine argument, the default argument of an empty curve evaluation).

## Not verified

Everything tagged **[I]** or **[D]** in the specs, the items listed under "still unknown" in
[vehicle-two-wheel-parameters](../specs/vehicle-two-wheel-parameters.md), and any behaviour that depends on a C++ default the
assets do not set and the constructors could not give. No statement here was tested in the running game (the game must not
be launched).

## Round 4 additions (tyre object, gearbox, aero, thermal)

Read against the user's GUI Ghidra instance (read-only queries) and, for functions the GUI project had not created, a private
headless copy. Specs: [ride6-tyre-mf94](../specs/ride6-tyre-mf94.md), [ride6-tyre-thermal](../specs/ride6-tyre-thermal.md),
[ride6-gearbox-auto](../specs/ride6-gearbox-auto.md), [ride6-aero-drag](../specs/ride6-aero-drag.md).

| Topic | Functions (rva) | How verified |
|---|---|---|
| MF94 class (`FTireModelData` vtable `0x149e43448`) | slots `0x14514dd00` Fx, `0x14514d190` Fy, `0x14514c180` Mz, `0x14514e710` rolling, `0x14514e420`/`0x14514d9d0` peaks, `0x145158490`/`0x145158480` boosts, `0x141271660` empty stub at `0x78`/`0x80` | vtable read from memory, wheel call sites (`0x14514df60`, `0x14514d4f0`, `0x14514e530`, `0x14514dad0`) matched |
| Pure-force evaluation and parameter builders | `0x145154a70`, `0x145151aa0`, `0x145151960`, `0x145175ad0`, closures `0x145175d30`/`0x145175cf0` | formulas read line by line; numeric examples recomputed |
| Peak solver and tables | `0x1451578b0`, `0x14516e5d0`, `0x14515d600`, `0x145150e20`, `0x145152140`, `0x14515ecc0`, table build `0x1451620b0`, lazy flag `0x14515a4a0` | structure and constants read; the minimiser stops at the gradient tolerance, our spec substitutes the root equation |
| Wheel efficiency factor with the constant 0.5 | `0x14516ebb0` (second half of the wheel force function `0x14514ce70`) | disassembly: `MINSS` against the value loaded from `0x148faa724` (0.5) |
| Drag, rolling and water drag forces | `0x1451001b0`, `0x145105ed0`, `0x1450f9cb0` | disassembly of the constant `0x1490ddc50` (-0.01) |
| Slipstream element | `0x1450de770`, `0x1450cca50`, `0x1450c60a0`, `0x1450bdcb0`, data `SlipstreamPhysicsElementBP` | read; constructor defaults and asset values |
| Tyre wear element | `0x14512bdd0`, `0x145110690`, `0x145150650`, `0x145168440`, `0x145150050`, `0x145125d30`, `0x1450fe840`, tyre setup `0x145169100`/`0x145173770` | read; curves from the dump |
| Gearbox: decision, generation, reference speed, request | `0x1450d7000`, `0x1450efa50`, `0x1450f1d20`, `0x1450cdea0`, `0x1450c9e10`, `0x1450f1c20`, `0x1450f2c70` | read; KTM change points recomputed with a linear torque curve (derived) |
| Clutch feathering and launch | `0x1450d7cd0`, `0x1450c0290`, `0x1451502d0`, constructor `0x1450bd6d0` | read; which `FClutchFeathering` instance is passed is inferred (register argument not shown) |

Not found: writers of `TimeOverLimit`, `NoGear*ChangeTimer`, `ClutchFeatheringTimer`; the forced downshift under braking; the
second and third reference-speed correction modes were read only far enough to know the KTM does not use them.
