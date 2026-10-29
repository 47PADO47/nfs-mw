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

## Target place [B]

The rider is a point mass with its own attitude (`FRiderComponent`, a member of the input handler at `0xF00`). Each step the
component (`0x1450f7c50`) works out where it wants the rider, and the body (`0x1450f5900`) follows on a spring. Offsets are
centimetres in the bike frame (x forward, y right, z up); the code maps them to its physics space (x right, y up, z forward)
when it reads the data.

Inputs: `w` the longitudinal weight after its channel (-1 back to 1 forward), `w_lat` the lateral weight, `lean` the **target
lean angle over the lean limit**, `lean = clamp(TargetLeanAngle / LeanLimit, -1, 1)` (the handler's `0xc18` and `0xd58`;
`0x1450d7520` forms `2 ramp(TargetLeanAngle; -L, L) - 1`, the same thing), and `v` the speed in km/h.

```
t_moving = ramp(v; StoppedSpeedRange)             # 0 at a standstill, 1 moving; a zero-width range is a step at its start
s        = clamp(LeanInfluence * lean + WeightInfluence * w_lat, -1, 1)
if w <= 0:  longitudinal = lerp(StoppedBackwardOffset, BackwardOffset, t_moving) * (-w)
else:       longitudinal = lerp(StoppedForwardOffset,  ForwardOffset,  t_moving) * w
L        = lerp(StoppedLateralOffset, LateralOffset, t_moving)
Offset   = BaseOffset + longitudinal + ( L.x |s|,  L.y s,  L.z |s| )    # forward and up follow the size of s, sideways its sign
target   = COMOffset + Offset                     # from the bike's own centre of gravity, scaled by the vehicle's scale
```

Facts that follow from the code and the data:

- `StoppedSpeed` (50 km/h in the data) is **never read**; the blend is `StoppedSpeedRange`, whose C++ default is (0, 0). The
  KTM does not set it, so `t_moving` is 1 at every speed and its `StoppedLateralOffset` (y -25 cm) is never used. A bike that
  sets the range uses the stopped offsets below it.
- `WeightInfluence` defaults to 1 and `LeanInfluence` to 0. The race and naked handlers leave both, so their lateral offset
  (x +15 cm, z -5 cm) follows the user's **lateral weight** input and nothing follows the lean; the supermoto, the enduro and
  the scooters set `LeanInfluence 1`, `WeightInfluence 0`, so the offset follows the planned lean.
- The y offset is the only signed one: the KTM's `LateralOffset.y = -12 cm` puts the rider 12 x `lean` cm to the **left** for a
  lean to the right (the supermoto rider stays upright over a bike that leans under him: the outside of the lean in the
  bike's frame). The scooters' +20 cm is the opposite side. A positive lean is a lean to the **right**: the lean angle is the
  chassis' Unreal roll, which lowers the right side when positive ([ride6-rider-round6.md](ride6-rider-round6.md), section 3).
- The offsets `x` and `z` follow `|s|`: leaning either way moves a race rider forward 15 cm and down 5 cm.
- The weight inputs are the channels `WeightLongitudinal` (rate 6 per second for the KTM) / `WeightLateral` (rate 10000: the
  constructor default, no asset sets it), then the second-order filter `PilotWeight*K/Damp` that passes its input when `K = 0`. Nothing else in the target is rate limited
  besides the rider's spring.
- The target frame is the chassis frame turned by a partial inheritance of the lean target and of the velocity direction
  (`InheritTargetLean`, `InheritVelocityPitch`, `InheritVelocityYaw`, soft-limited per axis by `InheritRotationLimit`, default
  90 deg). Their constructor defaults are **0** and no handler sets them, so the target attitude is the chassis'.
  (`PitchCurve` and `VisualTransform` only matter for the drawn rider.)

## Rider body: spring, damper, gravity, reaction [B]

Translation (`0x1450f5900`): the body is a point mass `m = Rider.Mass` at `Transform.Translation`, with
`error = target - position` (when `bCorrectCOMOffsetWithGravity`: `error -= m g / K`, so the rest place is raised by the sag
and the rider does not sink under his own weight; all the data sets it). `bUseNewRiderDelta` (set by all data) measures the
error as an arc about the chassis origin instead of a straight difference (formula in ride6-rider-round6.md, section 3; the
two differ by under a millimetre for the centimetres involved, the arc is computed).

```
c       = RiderDamp * lerp(DampMultiplier, 1, ramp(v; DampSpeedRange))     # defaults 20000 N/m, 5000 N s/m, (40, 60) km/h, 1
F       = K error + c (v_target - v_rider)                                  # v_target: the bike's velocity at the target
a       = F / m + g                                                         # semi-implicit Euler: v += a dt; x += v dt
reaction on the bike (bApplyForcesToVehicle, default on): the force -F applied at the rider's place
```

`ForceMagnitude`, its horizontal and its vertical part are kept for the fall test (`FallForceLimit`). The rider's own air drag
force is added to the force **after** the reaction was applied, so the drag moves the rider and not the bike directly.

Rotation (when `InertiaTensorScale` is not zero; the C++ default is (1, 1, 1)): the rider is a rigid body of moment
`InertiaTensorScale * Mass` (kg m^2; x 10000 in Unreal units), turned toward the target attitude by a torque

```
tau = RiderAngularK * theta_error + RiderAngularDamp * (RiderAngularDampChassisFactor * omega_bike - omega_rider)
```

(`theta_error` the rotation vector from his attitude to the target's; defaults K 1000, damp 100, chassis factor 1). The KTM sets
2500, 250 and **0**: the damper works against the world, so the rider's torso lags every rotation of the bike (a steady turn
rate `omega` gives the lag `C omega / K`, 0.1 s). His natural frequency is about 12 rad/s and the damping ratio 0.6. The torque
on the bike is `-tau` (in the chassis frame, multiplied per axis by `TorqueAppliedOnBikeMultiplier`, all 1) and fades in with
the speed: **zero at a standstill, full from 5 km/h** (the range (0, 5) km/h). The attitude then integrates as a rigid body
(`omega += I^-1 tau dt`, `q += 0.5 dt omega q`).

## Combined centre of gravity and inertia [B: `0x1450f2500`]

With `m_r = Rider.Mass * RiderMassScale` (the handler's scale, 1 for all data) and `M = CHASSIS.Mass`:

```
f         = m_r / (M + m_r)                          # the rider's mass fraction
COM       = (1 - f) P_bike_cg + f P_rider            # P_rider the simulated rider's place, not the target
COMHeight = distance from COM to the line through the two ground contact points  (a wheel without contact: the point at its
                                                                                  radius along its down axis)
COM lean  = angle of that perpendicular about the line, from the world's up
```

A bike with the simple branch uses `COMHeight = f COMOffset.z + (1 - f) CGPosition.z` and lean 0. The rider's sideways place
enters `COM`, so a rider hung off to the inside moves the combined COM inside and the bike needs less lean for a turn;
`COMHeight` is what the lean controller divides the roll torque by. The roll inertia in the lean controller is
`I_x + m_r COMOffset.z^2` (lean spec); the pitch inertia of the wheelie help is the bike's without the rider. The rigid
body's own inertia is `InertiaTensorScale * (M + Rider.Mass)` of the chassis, about the bike's centre of gravity.

## Block fall

The assist keeps the target lean under `BlockFallLeanLimitRatio * FallLeanLimit * 0.99` while it is active (lean spec, step 1);
the rider's place uses the target **after** that clamp, so the hang-off follows the clamped lean.

## The model in this repository

`blackbox_vehicle::bike`: `RiderSpec` (`rider_spec.rs`), the target place (`rider_place.rs`: `place`, `lateral_share`,
`drag_multiplier`), the translation spring (`rider.rs`: `RiderBody`, `combined_cog`), the rotation (`rider_rotation.rs`:
`RiderRotation`), wired in `handling.rs::shift_weight` (once per step, before the tyres). Manifest `[bike.rider]`
(`docs/modding/bikes.md`). The rider is not a second rigid body: bike and rider are one body whose centre of gravity follows
the combined COM above (the rider's force on the bike is internal to it); the rider's rotational torque on the bike is applied
as a torque, with the speed fade. No named choice of the rider body remains in `choices.rs`
(the sign of the lean, the lateral channel's rate, the arc error and the rider's drag are facts now). Not recovered: the inputs
of the angular target (zero inheritance for all data, so unused).

## Air drag [B]

The vehicle-level drag is `F = Cd (1 - s) g(theta) v^2` newtons opposing the velocity (`Cd = Aerodynamic.DragCoefficient`, 0.15 for
the KTM, C++ default 0.35; no factor 2: the code constant is `-0.01` with `v` in cm/s and 100 force units per newton; `s` the
slipstream wake, `g` = 1 unless the orientation tuning is set). Full derivation, the slipstream element and the water drag:
[ride6-aero-drag.md](ride6-aero-drag.md). `CdA = 0.25 m^2` for the KTM (`0.5 rho CdA`, `rho = 1.2`). The rider adds a drag force of
his own, `DragCurve(weight) * DragScaleCurve(tuck) * DragScale * (1 - s) |v| v` (read in round 6,
[ride6-rider-round6.md](ride6-rider-round6.md) section 1: for the KTM `0.1 * 1.25 * 1.2` upright and `0.1 * 1.25 * 0.65` tucked,
on top of the bike's `Cd`); the earlier posture factor on the whole drag is gone.
Wings (`FWingData`) are empty for all road bikes.

## Gyroscopic effects

See [ride6-lean-controller.md](ride6-lean-controller.md): a torque about the forward axis proportional to the wheel spin
momentum and the yaw rate, scaled by `GyroscopicRollMomentumScale` (default 1).
