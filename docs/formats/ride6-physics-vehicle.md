# RIDE 6 physics data: the vehicle asset (`FVehicleData`)

Part of the [format index](../README.md); method and inheritance rules in
[ride6-physics-data.md](ride6-physics-data.md). A vehicle asset is a `DefaultBike_C` Blueprint data asset (native parent
`UIgnitionVehiclePhysicsSetup`) holding one `FVehicleData`. Every table below gives the field, its unit, its meaning and
the value of the **KTM 450 SMR 2022** (`KTM_450SMR_2022`, vehicle id `KTM_450-SMR-2022`) as an example. The last column
says how the unit or meaning is known: **B** = confirmed in the binary (build `5.6.1-1697004`; see the specs listed in
[provenance/ride6-physics.md](../provenance/ride6-physics.md)), **D** = read from the data and the SDK field name only,
**C** = the C++ constructor default (the assets never set it; value not recovered unless written).

## Unit convention of the simulation

The assets are written in SI-like units, the simulation runs in Unreal units, and the code converts:

| Quantity | In the asset | In the simulation |
|---|---|---|
| Length of the chassis (CG, offsets, wheelbase, trail, rider position, shape sizes) | cm | cm |
| Suspension travel, compression, top-out length | m | cm (the code multiplies by 100) |
| Force | N | 1 Unreal force unit = 0.01 N (the code multiplies by 100) |
| Torque | N m | 1 Unreal torque unit = 1e-4 N m (the code multiplies by 10000) |
| Inertia | kg m^2 | multiplied by 10000 |
| Mass | kg | kg |
| Speed in tuning ranges | km/h (the code converts `Speed` in cm/s with 0.036) | cm/s |
| Angles | degrees in the assets, converted to radians with pi/180 where used | rad |

A spring rate written in N/m therefore keeps its number when compression is counted in cm and force in Unreal units
(`k` N/m is `k` Unreal-units per cm). [B: spring law, engine, inertia]

## `CHASSIS` (`FChassisData`)

| Field | Unit | Meaning | KTM | |
|---|---|---|---|---|
| `Mass` | kg | mass of the bike without rider and without the elements' extra masses (the rider's mass is added in the input handler) | 113 | B |
| `WeightBalance` | fraction | static share of the weight on the front wheel (reading; sport bikes 0.50 to 0.545) | 0.505 | D |
| `CGPosition` | cm | centre of gravity; x forward 0, y 0, z height | (0, 0, 67) | D |
| `bAdjustCGHeight` | bool | the CG height follows the rider | true | D |
| `InertiaTensorScale` | m^2 | **radius of gyration squared** per axis: inertia = scale x total mass (kg m^2). x roll (forward axis), y pitch, z yaw | (0.2, 0.4, 0.45) | B |
| `LinearDamping`, `AngularDamping`, `MaxAngularSpeed` | | body damping and a clamp | 0, C, 6 | D |
| `WheelShapeType/Factor/Offset`, `RiderShape*`, `RiderCapsule*` | cm | collision shapes (body and rider capsule) | capsule, rider capsule r 34 h 25 at z 97.5 | D |
| `Stopped*Threshold`, `LowSpeedConstraintWheelNearGroundThreshold` | cm/s etc. | when the bike counts as stopped; low speed constraint | 10, 10, 0.2, 0.05, 2 | D |
| `VelocitySolverIterationCount` | | rigid body solver iterations | 16 | D |

The total mass used by the dynamics is `Mass` plus the mass every vehicle element reports (the input handler reports the
rider: see the elements page). The inertia is `scale_axis x total_mass x 10000` in Unreal units; the sprung mass is the
total mass minus both wheel masses. [B: function `0x145140350`]

## `Skeleton` (`FSkeletonData`)

| Field | Unit | Meaning | KTM |
|---|---|---|---|
| `bOverrideSkeletonData` | | take the wheelbase and the rake/trail from here instead of the skeleton | true |
| `OverrideWheelBase` | cm | wheelbase | 145 |
| `RakeTrailOverride[0]` | deg, cm | steering head angle from vertical, trail | rake 26.1, trail 10.5 |
| `SuspTravelOffsets` | cm | extra travel offsets front/rear | 2, 2 |

## `FrontSuspension`, `RearSuspension` (`FSuspensionData`)

The suspension of a bike is the fork (front) and the swingarm with a linkage (rear). The wheel has its own mass and the
compression is integrated with it. Equations in [ride6-suspension.md](../specs/ride6-suspension.md).

| Field | Unit | Meaning | Front | Rear | |
|---|---|---|---|---|---|
| `MaxCompression` | m (wheel travel) | travel to the bump stop | 0.285 | 0.266 | B |
| `RestCompressionOffset` | m | extra static compression at rest | 0.012 | 0.01 | D |
| `SpringStrength` | N/m | spring rate **at the shock** (see leverage) | 9000 | 51000 | B |
| `LinearSpringCompressionRatio` | fraction of `MaxCompression` | compression where the progressive term starts | C (0) | 0 | B |
| `ProgressiveSpringFactor`, `ProgressiveSpringLinearity` | | extra term `factor sign(x) (abs(x) - r M)^linearity` added to the compression before the rate | C, 0 | C, 0 | B |
| `TopOutSpringStiffness` | N/m | rate of the spring that opposes full extension | 4000 | 18000 | B |
| `TopOutSpringLength` | m | length of the top-out zone | 0.06 | 0.035 | B |
| `HydraulicComponentStiffness` | N/m | a second, progressive, spring-like term (hydraulic bump stop) | 400 | 200 | B |
| `LinearHydraulicComponentCompressionRatio` | fraction | where its progressive term starts | C | 0 | B |
| `ProgressiveHydraulicComponentFactor`, `...Linearity` | | its progressive term | 1, 2 | 1, 2.8 | B |
| `DamperSlowBump`, `DamperFastBump` | N s/m | compression damping, below and above the knee speed (continuous two-slope law, see the suspension spec) | 1700, 1300 | 8000, 3500 | B |
| `DamperFastBumpThreshold` | m/s | knee speed (compression) | 0.3 | 0.2 | D |
| `DamperSlowRebound`, `DamperFastRebound`, `DamperFastReboundThreshold` | N s/m, m/s | the same for rebound | 2500, 1700, 0.3 | 11000, 5000, 0.2 | D |
| `SuspensionLeverageRatio` | wheel travel per shock travel, against compression/`MaxCompression` | rear linkage: `x_shock = x / max(LR(x/M), 1)` | none (1) | 2.8 at 0, 1.97 at 1 (cubic) | B |
| `ChainPullFactor` | | rear: squat from the chain pull, scaled by `ChainPullFactorCompressionFactor` (1 to 0.6 over the travel) | | -0.014 | D |
| `TractionInfluenceOnLoad` | | rear: the drive force shifts load (anti-squat), scaled by `...CompressionFactor` (1 to -0.4) | | -0.15 | D |
| `BrakeTorqueCouplingFactor` | | brake torque couples into the suspension (front dive, rear lift) | 35 | 30 | D |
| `SpringMaxSpeed` | m/s | clamp of the spring-side compression speed | 5 | 5 | D |
| `bEnableSwingarmForces` | bool | enables the chain pull and traction terms | C | C | B |
| `RayCastLength`, `HitNormalLimit`, `PointRejectAngleThreshold`, `SmoothNormalSurfaces`, `SurfacesTerrainOffset` | | ground query | 50, 0.9, 0.3 | same | D |

The damper law and the unset C++ defaults are in [ride6-suspension.md](../specs/ride6-suspension.md). The knee speeds are in m/s.

## `FrontWheel`, `RearWheel` (`FWheelData`)

| Field | Unit | Meaning | Front | Rear | |
|---|---|---|---|---|---|
| `ShapeRadius`, `ShapeWidth` | cm | tyre radius and width: not in the data; set from the wheel mesh at spawn | n/a | n/a | D |
| `ShapeToroidFactor` | | the tyre section's minor radius as a fraction of half the width; the contact radius drops by `0.5 (1 - abs(cos(camber))) factor width` | 0.9 | 0.7 | B |
| `TireCompressionShapeFactor` | | how much of the tyre's own deformation shortens the radius | 1 | 1 | B |
| `TrackSize` | cm | tyre width used by the contact (sidewall) | 8 | 16 | D |
| `SteerAngle` | deg | maximum steering angle of the front | 40 | | D |
| `Mass`, `Inertia` | kg, kg m^2 | wheel mass (B: it integrates the suspension) and spin inertia (D) | 18, 0.6 | 20, 0.7 | B/D |
| `SlipCorrection`, `RestSlipCorrection`, `RestSpeedThreshold` | | low-speed damping of the slip (see the tyre spec) | 0.05, 0.25, 25 | same | D |
| `DrySlipCorrectionData`, `WetSlipCorrectionData` | per surface | the same per surface; `RestSpeedRange` in km/h | | | D |
| `LateralGripLimit` | | cap of the lateral force coefficient (lateral force / load) | 1.2 | 1.3 | D |
| `TireStiffness` | N/m | radial stiffness of the tyre, used to split the ground penetration between suspension and tyre | 400000 | 400000 | B |
| `MaxTireDeformation` | cm | clamp of the tyre deformation | 3 | 3 | B |
| `TireData` | asset | the tyre (see the tyre page) | `FrontTyre_Road` | `BackTyre_Road` | |
| `bAffectedByHandbrake` | bool | rear brake input drives this wheel | | true | D |

## `Engine` (`FEngineData`)

| Field | Unit | Meaning | KTM | |
|---|---|---|---|---|
| `PeakTorque` | N m | torque at curve value 1.0 | 48 | B |
| `TorqueCurve` | normalised torque against rpm | **the external curve wins**; 16 points, 1652.5 to 11000 rpm, 0.7375 at the start, 1.0 at 7159 rpm | | B |
| `PeakTorqueLimit` | | cap of the curve value | C | B |
| `MOI` | kg m^2 | engine rotating inertia | 0.015 | B |
| `EngineFrictionPeakTorque` | N m | closed-throttle drag torque at the curve value 1.0 (see engine spec) | 30 | B |
| `EngineFrictionCurve`, `NormalizeEngineFrictionCurveDomain` | | drag against rpm; the domain is divided by `MaxRPM` when normalised | `Base_EngineBrake`: (0,0), (1,1), (1.5,2) | B |
| `MinRPM`, `MinRPMLimit` | rpm | idle behaviour | 2300, 2200 | B |
| `MaxRPM` | rpm | limiter | 11000 | B |
| `ThrottleAtMinimum` | | the throttle is kept at least this high below idle | 0.15 | B |
| `EngineLimiter[]` | | rev limiter modes (see the engine spec) | 4 entries | D |
| `EngineBrakeReduction*` | | reduction of the drag (electronic engine brake control) | | D |

## `Transmission`, `Differential`, `Brake`, `Aerodynamic`, `InputData`

| Field | Unit | Meaning | KTM | |
|---|---|---|---|---|
| `ForwardGears[].Ratio` | | gear ratio | 2.0, 1.625, 1.333, 1.142, 0.954 | B |
| `FinalRatio` | | final (chain) ratio including the primary drive | 7.45 | B |
| `ClutchLinearity` | | engagement exponent `(1 - clutch)^linearity` | 1 | B |
| `SlipperClutchDeltaTorqueRange` | Unreal torque units (/10000 for N m) | range of the back-torque over which the slipper opens | 250000 to 400000 (25 to 40 N m) | B |
| `SlipperClutchMaxIntensity`, `SlipperClutchFilterSpeed` | | how far it opens, and its smoothing | 1, C | B |
| `Differential` | | open, rear drive 100 % | rear drive | D |
| `Brake.BrakeForce` | N | **front brake** force at the tyre patch at full lever: wheel torque `BrakeForce * BrakeBalance * R` (balance 1 for every bike; confirmed in the binary) | 1880 | B |
| `Brake.HandbrakeForce` | N | **rear brake** force at the patch (wheels with `bAffectedByHandbrake`): torque `HandbrakeForce * R` | 1000 | B |
| `Brake.BrakeBalance` | | 1 for every vehicle; the two brakes of a bike are driven by two inputs | 1 | D |
| `Aerodynamic.DragCoefficient` | | air drag coefficient (frontal area folded in) | 0.15 | D |
| `InputData` | | rise and fall rates and linearity of the input axes (throttle 0.75, 3 and 5 per second; brake 0.65, 3, 5; steer rate 10; weight rate 6) | | D |

`BrakeForce` ranges from 1500 to 3300 over the 317 bikes: the front brake alone gives 0.8 to 1.1 g of the bike plus rider weight.
The distribution and the torque formula are in [ride6-engine-drivetrain.md](../specs/ride6-engine-drivetrain.md).

## `VehicleElements`, `TuningDataTables`, `TuningDefault`, `TuningActions`

`VehicleElements` lists the Blueprint classes of the controllers the bike runs (input handler, rider help, wheelie
control, ABS, TCS, tyre wear, fuel, dynamic tuning, effects): [ride6-physics-elements.md](ride6-physics-elements.md). The
three `Tuning*` fields point at the setup menu tables (which struct member each tuning entry changes, ranges, defaults).
