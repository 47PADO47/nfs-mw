# Provenance: RIDE 6 powertrain and brakes, round 6

Covers [ride6-powertrain-round6](../specs/ride6-powertrain-round6.md), and the corrections it makes to
[ride6-gearbox-auto](../specs/ride6-gearbox-auto.md) and [ride6-engine-drivetrain](../specs/ride6-engine-drivetrain.md). Continues
[ride6-physics.md](ride6-physics.md).

## What was used

- The user's own install of RIDE 6 (build `5.6.1-1697004`), read-only. No decompiler output, game file or extracted asset is in the
  repository; the spec states behaviour and constants in our own words.
- A private headless Ghidra copy of the analysed executable (a project in the scratch folder); the functions below were decompiled
  to a scratch folder outside the repository and read there. The project holds disassembly only where a function was asked for, so
  functions were found by displacement search over the module (the SDK offsets of `UInputHandler`, `UPhysicsVehicle`,
  `FVehicleActions`, `UPhysicsVehicleTransmission`, `UPhysicsVehicleEngine`, `UABSVehicleElement`), by the constructors' vtable
  pointers (found by scanning the code for the address of the table) and by the virtual table slots (the Dumper-7 offsets and a
  comparison of the tables of the derived handlers).
- The Dumper-7 dump of the install for class and struct layouts (no code), and the resolved data dump of the 319 vehicles for the
  data values and for "no bike sets this field" checks.

Addresses are image-relative virtual addresses (image base `0x140000000`).

## Functions

| Topic | Functions | How verified |
|---|---|---|
| Shift timing: progress, clutch trapezoid, throttle timing, release and modulate flags | `0x1450d7960` (slot `0x420`: calls the timing, then the launch blip) and `0x1450d7ac0` (the timing) | arithmetic read; the translator pair `(u, v)` form checked against the three uses; the KTM values recomputed by hand |
| Gear applied once the clutch is out (`TotalGearChangeTime * a <= TimeSinceLastGearChange`), engine and brake inputs written to the vehicle parts | `0x1450cf350` (the handler's apply step), `0x145168660` (`SetGear`), `0x1451690c0` (engine throttle input) | read |
| Gear request and its acceptance rule | `0x1450c9e10` (already in round 4); a second, forced form `0x1450eee30` (`bImmediate`) | read |
| Timers of the handler: `TimeSinceLastGearChange`, `TotalGearChangeTime` reset, `NoGear*` decay, `TimeOverLimit`, `ClutchFeatheringTimer` | `0x1450f8a80` (virtual slot `0x440`) | read |
| Writers of `NoGear*ChangeTimer` | `0x1450d7490` (manual gear request on an automatic bike) | read |
| Handler flags: `IsAccelerating`, `IsBraking`, `IsUsingHandbrake`, `IsPressingClutch`, `IsInReverse`, `IsAtLowSpeed`, `IsDoingBurnout`, `HasStopped`, `RequestingReverse` and the constants 0.1 and 1 m/s | `0x1450d7fb0` | read; constants read from the data section (`0x148fbaccc` = 0.1) |
| Start, neutral and reverse logic of the automatic handler, and the decision | `0x1450d7000` (slot `0x3c8`), `0x1450d8240` (slot `0x3d0`, the manual-aware variant with `0x1450d84a0`), `0x1450d8620` (slot `0x3e0`, reverse) | read; the constant `0x148fa8c4c` = 100 cm/s |
| Starting sequence: launch clutch end, throttle-release rule | `0x1450d7cd0` (slot `0x418`), `0x1450c0290` (feathering, round 4) | read |
| Top-level handler update and the slot map | `0x1450d9520` (slot `0x3b8`), `0x1450f0790` (pre-update: the idle clutch, `PressClutchAtMinRPM`, `MinRPMSpeedScaleRange`), helpers `0x145158510` (road speed at idle in the present gear), `0x145158860` | read; the virtual tables of the handler family compared slot by slot |
| Handler constructor defaults used here (`PressClutchAtMinRPM`, `MinRPMSpeedScaleRange`, `bUseDeprecatedBlipperLogic`, `BlipperThrottleOnShift`, `BlipperMinClutch`) | `0x1450bbf80` | stores read |
| Slipper clutch: target, the square-root filter, `bUseVariomatic` | inside `0x14515ac20` (round 1) | the filter branch read line by line |
| `FTransmissionData` constructor (`SlipperClutchFilterSpeed` 0, `SlipperClutchMaxIntensity` 0, `SlipperClutchDeltaTorqueRange` 1e6 to 2.5e6) | `0x1450fa5f0` | stores read |
| ABS element: per-wheel filter and the multipliers | `0x1450e0240` (round 1), the rate limiter `0x14515c220` | read |
| ABS element constructor (`ABSFallRate` 10000, `BrakeReductionFallRate` 1000, `bUseDeprecatedLogic` true) | `0x1450bbb40` (found through its table pointer at image offset `0x9deef28`) | stores read |
| Brake reduction, deprecated path | `0x1450ce620` (called from `0x1450cb670` when `bUseDeprecatedLogic`) | structure read, pairing not settled |
| Engine setup: `MaxTorqueRPM`, `MaxPowerRPM`, `MaxPower` from the curve keys | `0x1451698a0` | read |
| Brake coupling sign: `AppliedTorque`, `BackTorque`, `ResultingTorqueOnChassis` | the wheel loop at the end of `0x14515ac20`; the coupling force `0x145153440`; the sum of the suspension forces `0x14514cc40` | read; the units constants `0x148f94ecc` = 0.01, `0x148fa8c50` = -1, `0x148fa8c60` the sign mask |
| Forced downshift under braking: hold-off override and brake timer | `0x145137390` (virtual slot `0x438`, calls `0x1450f1c20`), `0x1451405c0` (slot `0x440`, calls `0x1450f8a80`); the per-tick zeroing of the hold-offs in `0x1450e0480` | read; the field offsets are the SDK's for `UMotorbikeInputHandler`; the road-bike constructor `0x1450fbfa0` (table at image offset `0x9e06b30`) gives the defaults |
| Brake reduction, deprecated path: pairing of the six tunings | `0x1450ce620` | the translator offsets matched to `FBrakeReductionTuning` and the IMU channels to `FVehicleIMUData` |

## Round 7 (final powertrain round)

Spec: [ride6-powertrain-round7](../specs/ride6-powertrain-round7.md). The same private headless copy; the KTM's handler class is
`UMotorbikeInputHandler` (constructor `0x1450fbfa0`).

| Topic | Functions | How verified |
|---|---|---|
| Brake reduction, deprecated path: inputs and combination | `0x1450ce620` (called from `0x1450cb670`); the element update `0x1450e0240`; the rate limiter `0x14515c220` | the translator offsets matched to `FBrakeReductionTuning`, the vehicle offsets `0x183c`, `0x1848`, `0x1818`, `0x179c`, `0x17b4` to `FVehicleIMUData` and `FVehicleActions`; the wheel's `0x150`/`0x154` to `UPhysicsVehicleWheel` |
| Translator defaults (all-zero ranges; `UseAbsoluteValue` true for lean and drift only) | `0x14509cd80` (the `FBrakeReductionTuning` constructor) | stores read, the zero and unit vectors at `0x14bc01268` and `0x14bc01278` read from the data section |
| Engine-brake reduction | `0x14514c850` (engine step), `0x1451507b0` (the reduction), engine data constructor `0x1450f9ff0`; the call site in `0x145171f60` passes the IMU (`vehicle + 0x1818`) and the clutch (`+ 0x17f0`) | stores and arithmetic read; `FVehicleIMUData` offsets `0x20`, `0x50`, `0x58`; the IMU writer `0x1450f4310` for the tire-load and slip definitions |
| Over-limit upshift | `0x1450d7000` (the test), `0x1450f8a80` (the timers), `0x1450c9e10` (the reset) | read in round 6; data from the 319 vehicles |
| Wheelie help torque ramp | `0x145100f80` (the operand is the engine's torque curve at `Engine.EngineRPM`, from the argument register in the disassembly) | disassembly of the call |
| End of the feathering start | `0x1450c0290` (the closing test), `0x1450d7cd0` | read |

## Class of each function

The road-bike handler (`UMotorbikeInputHandler`, constructor `0x1450fbfa0`) is the KTM's class; `0x1450fb230` is the motocross
handler (`UMXBikeInputHandler`), whose offsets differ. Round 6 first took `0x1450fb230` for the road bike, which sent the search
for the forced downshift to the wrong offsets (and the lean-limit function `0x14510a330`, whose offsets are the motocross class's).
Checked afterwards: the virtual slots `0x3b8` to `0x420` (start logic, decision, shift timing, launch) are the same function in
every handler's table, and the fields they read lie below `0x1860` (`UInputHandler`, `UBikeInputHandler`), where all classes
agree. The road-bike class overrides slots `0x438` (hold-offs), `0x440` (timers) and the serializer, calling the shared function
first, so `0x1450f8a80` and `0x1450f1c20` are still the shared part. No fact of sections 1 to 6 changed.

## Corrections to earlier rounds

- Round 1 listed `0x1450c1660` as "shift timing"; it is not the gear change: it combines range translators of IMU channels and hands a
  filtered intensity to the engine object (an electronics or engine-brake modulation; not analysed further). The shift timing is
  `0x1450d7ac0`.
- The gearbox spec said the writers of `TimeOverLimit`, `NoGear*ChangeTimer` and `ClutchFeatheringTimer` were not found; they are
  `0x1450f8a80` and `0x1450d7490`. The neutral/stopped logic that "shifts to 1 below `NeutralGearMaxSpeed`" belongs to the manual
  path (`0x1450d84a0`); the automatic start logic is `0x1450d7000`.
- Round 1 gave the ABS and slipper filters' defaults as unlisted; they are 10000 and 0.

## Not verified

No statement was tested in the running game (it must not be launched). The values of the KTM data are those of the resolved dump.
