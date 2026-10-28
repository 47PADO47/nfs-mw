# RIDE 6 rider body, mass distribution and air drag (clean-room spec)

Data: `Rider` of [ride6-physics-elements.md](../formats/ride6-physics-elements.md), `CHASSIS` and `Aerodynamic` of
[ride6-physics-vehicle.md](../formats/ride6-physics-vehicle.md). Mapping to our model:
[vehicle-two-wheel-parameters.md](vehicle-two-wheel-parameters.md). Sources: [provenance](../provenance/ride6-physics.md).
Tags: **[B]** read from the binary, **[I]** inferred, **[D]** data only.

## Mass and inertia

- The bike's `CHASSIS.Mass` (KTM 113 kg) excludes the rider. The rider is a separate component (`FRiderComponent`, `Rider.Mass`
  70 kg for the Motard handler). The mass the rigid body uses is `CHASSIS.Mass` plus the mass each vehicle element reports
  (the input handler reports the rider) [B: `0x145140350`, the summation; I: that the rider is what the handler reports].
- Inertia of the rigid body about each local axis: `I_axis = InertiaTensorScale_axis * M_total` (kg m^2), `M_total` including
  the rider; x is the roll axis (forward), y pitch, z yaw. The sprung mass is `M_total` minus both wheel masses. [B]
  KTM: `I_roll = 0.2 * (113 + 70) = 36.6 kg m^2`, pitch 73.2, yaw 82.35.
- The lean controller adds the rider's own parallel-axis term when it computes the roll torque: `m_rider * RiderMassScale
  * COMOffset.z^2` (70 x 0.4^2 = 11.2 kg m^2). [B: `0x1450c8f70`]
- The combined centre of gravity weights the bike's and the rider's positions by mass: `cg = (M cg_bike + m_r p_rider) /
  (M + m_r)` [B: the mass fractions `M/(M+m_r)`, `m_r/(M+m_r)` are formed in `0x1450f2500`].
  `CGPosition.z` (67 cm) is the bike's own CG; `bAdjustCGHeight` lets the height follow the rider.

## Rider position (weight shifting)

The rider's body is a point mass attached to the bike by a spring-damper whose rest position is set by the weight inputs:
`WeightLongitudinal` (back -1 to forward 1), `WeightLateral` (left/right) and `RiderPosition` (sit/stand/tuck). The rest
position is `BaseOffset` plus the `ForwardOffset` / `BackwardOffset` / `LateralOffset` (`Rider` fields, centimetres in the
bike frame) interpolated by the filtered inputs; at standstill (`StoppedSpeedRange`, `StoppedSpeed` 50 km/h) the
`Stopped*Offset` set is used (KTM: `StoppedLateralOffset` y = -25 cm, the foot out; `LateralOffset` y = -12 cm when
leaning). The inputs are filtered by second-order filters (`PilotWeightLongitudinalK/Damp`, `PilotWeightLateralK/Damp`;
a damp of -1 is critical) [B: the filters are the common second-order filter of the lean spec; their `K` defaults to 0,
and a filter with `K = 0` passes its input through unchanged, so for the KTM, which does not set them, the weight inputs
are only limited by the input rates of `InputData` (6 per second)]. The rider spring is `RiderK`, `RiderDamp` for the translation (C++ defaults **20000 and 5000** [B: `0x1450bb350`]; the
KTM does not set them) and `RiderAngularK` / `RiderAngularDamp` for the rotation (defaults 1000 and 100, the KTM sets 2500
and 250), `DampSpeedRange` (40, 60) with `DampMultiplier` 1 (damping scale by speed), `RiderAngularDampChassisFactor` (default 1,
KTM 0), `InertiaTensorScale` default (1, 1, 1) (KTM 0.23, 0.28, 0.19), `InheritRotationLimit` default (90, 90, 90),
`WeightInfluence` default 1 (KTM 0), `SaddleHeight` default -1000 (KTM 20); `bUseNewRiderDelta` selects the newer relative-motion formulation, and
`TorqueAppliedOnBikeMultiplier` (1, 1, 1) scales the reaction torque on the bike. The force the rider exerts on the bike
is the spring-damper force (its magnitude is also a fall criterion: `FallForceLimit`). [D + I]

**Effect on the body.** Shifting the rider moves the combined CG, which changes the front/rear load split (a wheelie needs
the weight back and torque; a stoppie weight forward and braking), and the reaction force of the rider on the bike gives
a pitch/roll torque. The lateral shift adds to the lean (`RiderLeanAngle`, `RiderLeanAngleInfluence 1`: the target lean of the
chassis is reduced by the rider's own lean angle). `WeightInfluence 0` and `LeanInfluence 1` of the KTM mean the rider's
lean follows the lean angle, not the weight input, in the lean calculation.

## Air drag [B]

The vehicle-level drag is `F = Cd (1 - s) g(theta) v^2` newtons opposing the velocity (`Cd = Aerodynamic.DragCoefficient`, 0.15 for
the KTM, C++ default 0.35; no factor 2: the code constant is `-0.01` with `v` in cm/s and 100 force units per newton; `s` the
slipstream wake, `g` = 1 unless the orientation tuning is set). Full derivation, the slipstream element and the water drag:
[ride6-aero-drag.md](ride6-aero-drag.md). `CdA = 0.25 m^2` for the KTM (`0.5 rho CdA`, `rho = 1.2`). The rider adds drag
(`Rider.DragCurve` `RiderAirDrag`, `DragScale` 1.25, `DragScaleCurve`) and tucking in (`TuckInLevel`, rider help) lowers it. [I]
Wings (`FWingData`) are empty for all road bikes.

## Gyroscopic effects

See [ride6-lean-controller.md](ride6-lean-controller.md): a torque about the forward axis proportional to the wheel spin
momentum and the yaw rate, scaled by `GyroscopicRollMomentumScale` (default 1).
