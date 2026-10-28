# RIDE 6 physics data: vehicle elements (controllers)

Part of the [format index](../README.md); see [ride6-physics-data.md](ride6-physics-data.md) for the dump and
[ride6-physics-vehicle.md](ride6-physics-vehicle.md) for the vehicle asset. A bike lists the Blueprint classes of its
controllers in `VehicleElements`; each class default object holds the tuning. For the KTM 450 SMR 2022 they are
`Motard_InputHandler_C` (native `UHandlebarInputHandler` : `UBikeInputHandler`), `Motard_RiderHelp_C`
(`URiderHelpVehicleElement`), `Base_WheelieControl_C` (`UWheelieControl`), `Motard_ABSVehicleElement_C`
(`UABSVehicleElement`), `Base_TCS_C` (`UTCSVehicleElement`), `Base_TireWear_C`, `Base_FuelManagement_C`,
`Base_DynamicTuningVehicleElement_C` and `Base_EffectsVehicleElement_C`. Behaviour: [ride6-lean-controller.md](../specs/ride6-lean-controller.md),
[ride6-rider-help.md](../specs/ride6-rider-help.md), [ride6-engine-drivetrain.md](../specs/ride6-engine-drivetrain.md).

Every field named below exists in the SDK dump (`ignitionphysics_classes.hpp`). The assets store only the changes from the
**C++ constructor defaults** of the class (function `0x1450bbf80` for `UBikeInputHandler`); the defaults that matter are
listed where they differ from the KTM value. Angles in these tables are degrees and speeds km/h unless said otherwise
(the code converts: `0.0174533` for degrees, `0.036` for cm/s to km/h, `57.2958` for radians to degrees). [B]

## `UBikeInputHandler` / `UHandlebarInputHandler` (`Motard_InputHandler`)

### Lean and steering

| Field | Meaning | KTM | C++ default |
|---|---|---|---|
| `SteerK`, `SteerDamp` | second-order filter of the steer input: stiffness and damping (a negative damp `-z` means a damping ratio `z`; zero means 1) | 1000, -2.5 | |
| `LeanLimit` | maximum lean (deg) the stick asks for; **recomputed every tick** (grip, g-force, skid, speed) | not set | 50 |
| `LeanK`, `LeanDamp` | spring and damping of the roll of the bike towards the target lean (ratio `-z`, multiplied by `LeanComponent.LeanDampMultiplier`) | 1000, -3.6 | 20000, 80 |
| `LeanComponent.*Range` | reference weight (180 to 352 kg), tensor x (0.2 to 0.3), height (80 to 51), wheelbase (131 to 167): the damping multiplier (1 to 1.5) is interpolated by where the bike lies in these ranges | as listed | |
| `LeanTimeScale`, `LeanTimeScaleAttenuation`, `LeanTimeScaleSensitivity`, `LeanTimeScaleSpeedTuning`, `LeanTimeScaleInputTuning` | speed of the lean target change | 1, 0, 0, speed 10..200 -> 1..0.65, curve (-1, 0.425), (0, 1), (1, 0.7) | 1 |
| `bUseKDampFilterForTargetLeanAngle`, `TargetLeanAngleFilter` | second-order filter of the target lean | true, K 1000, damp -2.25 | |
| `HandlebarK`, `HandlebarDamp`, `HandlebarTimeScale` | second-order filter of the handlebar angle | 1000, -3, 1 | , , 1 |
| `bUseDynamicBikeGeometry`, `LowSpeedSteerBlend`, `LowSpeedSteerBlendRange` | below the range (m/s) the handlebar follows the stick directly | true, true, (2, 4) | |
| `RiderMassScale`, `RiderLeanAngleInfluence` | weight of the rider in the lean inertia, and how much the rider's own lean offsets the target | 1, 1 | |
| `SimulateGyroscopicEffects`, `GyroscopicYawMomentumScale`, `GyroscopicRollMomentumScale` | wheel gyroscopic torques | true, C, C | 1, 1 |
| `LeanHelp*`, `StabilityHelp*` (many) | the lean/stability assist: factor against speed, reduced by throttle, brake, slip and skid; extra on off-track surfaces | see the dump | |
| `LeanLimit*`: `LeanLimitGForceTuning` (curve), `LeanLimitSkidRange` (10, 40), `LeanLimitSkidLinearity` 1.8, `LeanLimitSkidReduction` | grip based lean limit | g curve (0, 1), (1.1, 0.9); skid reduction 0..30 -> 0..0.2 | |
| `StoppedLeanLimit`, `StoppedSpeedThreshold` | lean limit at standstill and the speed below which it blends in | 1, 30 | |
| `StoppieLeanLimitTuning`, `WheelieLeanLimitTuning` | lean limit during a stoppie (pitch 0..20) or wheelie (0..40) | 1, 1 | |

### Gearbox and clutch

| Field | Meaning | KTM |
|---|---|---|
| `AutomaticTransmission`, `TransmissionSettings.bAutomaticGearChangePoints` | automatic shifting | true |
| `GearChangePoints[g]` | per gear: `ChangeUp`, `ChangeDown` (0 = derive), `ChangeUpWithThrottle` (share of the rev limit at full throttle), `ChangeDownWithThrottle`, `ChangeUpTolerance`, `ChangeDownTolerance` | up with throttle 0.9 for all gears; down tolerance 0.4, 0.9, 1, 1, 1; up tolerance 1 |
| `GearUpChangeTime`, `GearDownChangeTime` | duration of a shift (s) | 0.06, 0.15 |
| `ClutchTimingOnGearChange`, `ThrottleTimingOnGearChange` | clutch and throttle phases of a shift as fractions of the shift time | (0.125, 0.2), (0.05, 0.9) |
| `GearWhenStopped`, `NeutralGearMaxSpeed`, `ChangeUpMinSpeed` | start gear; speeds | 1, 25, 5 |
| `OverLimit*` | shift up when the limiter is hit above a speed | min speed 60, time 0.064, rpm share 0.94 |
| `ClutchFeatheringOnStart` | slip the clutch on launch: `ClutchFeatheringMaxRPMFactor` 0.7, `...PeakTorqueRPMFactor` 0.85, `...DeltaTime` 0.05 | enabled |
| `ClucthMaxRPMPercentage`, `ClucthReleaseVelocityRange` | clutch released between speeds 0 and 50 | 0.98, (0, 50) |
| `BlipperThrottleOnShift`, `MinRPMSpeedScaleRange` | throttle blip on downshifts; scale of the minimum rpm | 0.3, (1.05, 1.1) |
| `PreventGearUpChangeLeanFactor/Linearity`, `PreventGearDownChange...` | no shifting while leaned | 0.5/30, 0.5/50 |
| `ForceGearDownChangeBrake*` | forced downshift under heavy braking | brake input 0.5..1 for 0.3 s |

### Brakes and falls

| Field | Meaning | KTM |
|---|---|---|
| `JoinedBrakesFactor`, `JoinedBrakesFrontFactor`, `JoinedBrakesRearFactor` | linked brakes: see the drivetrain spec | 1, 0, 0 |
| `LowSpeedBrakeTuning`, `LowSpeedBrakeThreshold`, `UseRearBrakeWhenStopped` | extra braking authority at a crawl | curve 0.2 -> 0.0015 over 0..20 km/h, 15, true |
| `Fall*`, `BlockFall*` | fall detection (lean, pitch, speed direction angle, angular speed, forces) and the "block fall" assist that holds the bike up | lean 70, pitch 65, speed-dir 85, angular speed 4.4, force 2e6; block fall ratio 0.975 above 40 km/h |
| `Rider` (`FRiderComponent`) | rider mass and position | mass 70, COM offset (-5, 0, 40) cm, saddle height 20, lateral offset -12, inertia scale (0.23, 0.28, 0.19), angular K 2500, damp 250 |
| `BackwardForce`, `BackwardMaxSpeed` | reverse push | 200000, 125 |

## Rider help, wheelie control, ABS, TCS

| Class | Fields that matter (KTM values) |
|---|---|
| `Motard_RiderHelp` | auto rider on above 5 km/h; wheelie weight control (3.6, 4.0), stoppie weight control (3.6, 4.0), g-force weight control (0.5, 0.75); auto tuck-in above 90 km/h; throttle control against wheelie: pitch (0, 4), forced wheelie (33, 45), pitch speed (0.2, 0.6); brake control against stoppie: pitch (0, 4), pitch speed (0, 0.6), g-force (1.6, 2.0); lateral weight control with banking thresholds (60, 60) |
| `Base_WheelieControl` | wheelie: dead zone 0.3, max angle 35, speed ramp 0..15, lean tuning (0.2, 1); stoppie: dead zone 0.95, max angle 45, look-ahead 1 s, time scale 2.5, speed curve `StoppieControl` (10, 0), (80, 1), (300, 0) |
| `Motard_ABSVehicleElement` | front ABS on above 10..25 km/h with normalised slip -0.3..-0.5, rear -0.5..-1; brake reduction with lean angle (front 0..0.7 -> 0..0.2, rear 0..0.9 -> 0..0.5), drift angle, tyre load and (rear) throttle |
| `Base_TCS` | speed 0..5, slip thresholds dry (0.09, 0.2), wet (0.07, 0.15); throttle modulation with yaw range (0.08, 0.17); limiter index 1 up to 30 km/h |
| `Base_TireWear` | time scales 1 and 1, `TerrainModifiers` table, uniform tyre computation |

The ABS, TCS and the engine electronics (`UECUVehicleElement`) are throttle and brake **modulators**: they scale the inputs
the engine and the brake system see, by a factor between 0 and 1 computed from slip, lean, drift angle and load.

## Tuning tables (`TuningComponent`)

`TuningComponentData_*` rows name a member path (for example `FrontSuspension.SpringStrength`), a modifier type (fixed or
ranged multiplier, additive, curve...) and its range; `TuningComponentEntries*` hold the default value of each entry and
`TuningComponentActions_*` the buttons of the setup menu. The stock bike runs with the entries' defaults; the setup menu
and the upgrades (`Upgrade_Motard`, `QuickShifter-RM`, `LaunchControl-RM`) change members of the resolved vehicle data.
