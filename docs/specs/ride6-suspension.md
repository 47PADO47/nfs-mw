# RIDE 6 suspension (clean-room spec)

The fork and the swingarm of a RIDE 6 bike, in our own words. Data fields:
[ride6-physics-vehicle.md](../formats/ride6-physics-vehicle.md). Mapping to our model:
[vehicle-two-wheel-parameters.md](vehicle-two-wheel-parameters.md). Sources: [provenance](../provenance/ride6-physics.md).
Tags: **[B]** read from the binary, **[I]** inferred, **[D]** data only.

Units: the compression state is in cm and force in Unreal units (0.01 N); the data are in m and N, so a rate in N/m keeps
its number (see the unit table of the vehicle page). Formulas below use SI (compression `x` in metres, forces in N); the
Unreal-unit constants cancel except where stated.

## State

Each suspension carries a compression `x` (wheel travel along the travel axis, from 0 at full extension to
`MaxCompression`), its speed `x'`, and the wheel it carries has a **mass** `m_w` (`FrontWheel.Mass` 18 kg, `RearWheel.Mass`
20 kg for the KTM). The wheel is not massless: [B: `0x14515aaf0`]

```
a  = (F_ground_along_travel - F_suspension) / m_w          # F_suspension is the spring + damper force at the wheel
x' += a dt
x  += (x'_old + 0.5 a dt) dt                                # trapezoid
```

then the ground contact corrects it (section "Ground contact"), the compression is clamped to `[0, MaxCompression]`, and
hitting the bump stop zeroes the speed. The spring-side quantities are the wheel quantities divided by the leverage ratio.
The static compression includes `RestCompressionOffset` (the preload) and the bike's weight; the rear linkage also
carries `Skeleton.SuspTravelOffsets`.

## Leverage ratio [B: `0x145151610`]

`LR(s)` is the curve `SuspensionLeverageRatio`, its input `s = x / MaxCompression` in 0..1, its output the **wheel travel
per shock travel**, clamped to at least 1. The shock (spring-side) compression and speed are

```
x_spring  = x / LR(s)          x'_spring = x' / LR(s)
```

and the spring force at the shock is converted back to the wheel (virtual work: `F_wheel = F_shock / LR`, so the rate
at the wheel is `k / LR^2`) [B: `0x145171f60` computes `invLR * spring(invLR * x)` and `invLR * damper(invLR * v)`].
The KTM rear has 2.8 at 0 and 1.97 at full travel (cubic, with the tangents of the asset); the fork has no curve
(`LR = 1`). `SpringStrength` of the rear (51000 N/m) is a **shock** rate: the rate at the wheel is `51000 / 2.8^2 = 6.5
N/mm` at the start of the travel.

## Spring force [B: `0x145152de0`, `0x145152440`]

With `x` the (spring-side) compression and `M` the maximum compression:

```
g(x; k, r, f, p) = k * ( x + f * sgn(x) * max(|x| - r M, 0)^p )          # progressive spring, p = Linearity
F_spring   = g(x + preload; SpringStrength, LinearSpringCompressionRatio, ProgressiveSpringFactor, ProgressiveSpringLinearity)
F_hydraulic= g(x;           HydraulicComponentStiffness, LinearHydraulicComponentCompressionRatio,
                            ProgressiveHydraulicComponentFactor, ProgressiveHydraulicComponentLinearity)
F_topout   = TopOutSpringStiffness * max(TopOutSpringLength - x, 0)        # opposes extension (acts when x < length)
F          = F_spring + F_hydraulic - F_topout
```

Inside the code `x` is in cm, so the progressive term uses the compression in cm: for the KTM front hydraulic term
(`k = 400`, `f = 1`, `p = 2`, `r = 0`) the compression enters as `x_cm`; in metres that is `0.01 * 400 * (100 x)^2 / 100`
extra, i.e. `F_h[N] = 400 * (x + 100 x^2)` with `x` in metres. For `p = 0` (the stock springs) the term is a constant
`f sgn(x)` and `f` must be 0 (the assets set `p = 0` to switch it off; `f` is a C++ default that was not recovered).
`preload` is `PreloadCompression` (from `RestCompressionOffset` and the static load).
The `DefaultBike` sets `HydraulicComponentStiffness` to 400 (front) and 200 (rear) with `p` 2 and 2.8, so the hydraulic term
is a progressive stiffening in the last part of the travel.

## Damper [B: `0x145150560`; assembly `0x145171f60`]

With `v` the spring-side compression speed in cm/s (positive = compression, `v = x'/LR`), the force in Unreal units is

```
if v > 0:   (v <= 100 Vb)  ?  v * SlowBump
                           :  ((SlowBump - FastBump) * 100 Vb + FastBump * v)
if v <= 0:  (-v <= 100 Vr) ?  v * SlowRebound                      # negative force
                           :  FastRebound * v - (SlowRebound - FastRebound) * 100 Vr
F_damper = DamperMultiplier * that                                   # DamperMultiplier = 1 unless a semi-active element changes it
```

`Vb`, `Vr` are `DamperFastBumpThreshold` and `DamperFastReboundThreshold` in m/s (the code multiplies by 100). Because
the rates are in N s/m and the speeds in cm/s with forces in 0.01 N, the numbers keep their SI meaning: in SI the law is
the continuous two-slope damper `F = c_slow v` up to the knee and `c_slow v_k + c_fast (v - v_k)` above it, separately
for bump (positive) and rebound. The force at the wheel is `F_wheel = F_damper(v/LR) / LR` (the same `1/LR` as the
spring: `0x145171f60` forms `invLR * damper(invLR * v)` and `invLR * spring(invLR * x)`). The sum with the spring, the
hydraulic term and the top-out force is the suspension force `F_susp = F_spring + F_hydraulic - F_topout + F_damper`
(`SuspensionForce`, plus the chain, traction and brake coupling forces). C++ defaults of the damper (unused by the bikes):
2000, 10000, 0.5, 3000, 6000, 0.5.

## Unset C++ defaults of the suspension and wheel [B: constructors `0x1450fa2a0`, `0x1450fa6a0`]

Suspension: `SpringStrength 30000`, `LinearSpringCompressionRatio 0`, `ProgressiveSpringFactor 0`,
`ProgressiveSpringLinearity 1`, `LinearHydraulicComponentCompressionRatio 0`, `ProgressiveHydraulicComponentFactor 0`,
`ProgressiveHydraulicComponentLinearity 1`, **`bEnableSwingarmForces true`** (so chain pull, traction and brake coupling
act on every bike), `RayCastLength 35`, the compression-factor remaps default to the identity (`Linearity 1`).
So a bike that leaves the progressive factors unset has a linear spring (`f = 0`); with `f = 0` the exponent does not matter.
Wheel: `ShapeRadius 35 cm`, `ShapeToroidFactor 1`, `TireCompressionShapeFactor 0`, `TrackSize 20`, `Mass 25`,
`Inertia 1.5`, **`LateralGripTolerance 1`**, `LateralGripLimit 1`, `LongitudinalGripScale 1`, `LateralGripScale 1`,
`NoTireTractionTerrainNormalThreshold 0.05`, `StaticFriction`, `DynamicFriction`, `Restitution` -1 (unused),
`LoadFilterSamples 60` (the tyre load is filtered over 60 steps), `TireStiffness 0`, `MaxTireDeformation 0`.

## Ground contact [B: `0x14514f3e0`]

The ray cast gives `CompressionToContact`: how much the suspension would have to compress for the tyre to touch.
If it exceeds the current compression the tyre is in contact. The penetration is shared between the tyre and the
suspension (tyre section of [ride6-tyre.md](ride6-tyre.md)): `tyre_compression = min(share * CompressionToContact,
MaxTireDeformation)` with `share = min(1, (SpringStrength / LR) / TireStiffness)`, `suspension_compression =
max(CompressionToContact - tyre_compression, compression)`. The compression speed becomes the velocity of the contact
point along the travel axis if that is larger than the current speed (an impact pushes the wheel up).

## Chain pull, traction and brake coupling [B: `0x145153440`, called from `0x14514cc40`]

Active when `bEnableSwingarmForces` (true by default). `s = clamp01(x / MaxCompression)` with the same compression in cm; each
`f_k(s)` is a range translator of `s` (the lean spec) with its own input and output range and `UseAbsoluteValue`:

```
F_chain    = -( T_wheel * ChainPullFactor * f_chain(s) )                       # T_wheel: TorqueFromEngine of the transmission (Unreal torque units)
F_traction = -( Fx_tyre * TractionInfluenceOnLoad * f_traction(s) )            # Fx_tyre: the wheel's LongitudinalTraction (Unreal force units)
F_brake    =  (T_applied - T_back) * 0.01 / R_cm * BrakeTorqueCouplingFactor * f_brake(s)    # wheel AppliedTorque and BackTorque, R = ShapeRadius in cm
```

All three are added to the suspension force with the spring: positive resists compression. KTM rear: `ChainPullFactor -0.014`
(`f_chain` 1 at no compression to 0.6 at full travel), so a drive torque `T` (N m) adds `1.4 T` N that resists compression at
the start of the travel (anti-squat); `TractionInfluenceOnLoad -0.15` (`f_traction` 1 to -0.4: it reverses near full compression):
`+0.15 Fx f` under acceleration; `BrakeTorqueCouplingFactor` 30 (35 on the fork): the `0.01` turns Unreal units into newtons, so the
brake force at the patch times `0.30` (rear) or `0.35` (front) times `f_brake` enters the suspension. The sign of
`F_brake` follows the sign of `T_applied - T_back`. The brake coupling is therefore a 35 % (front) and 30 % (rear) transfer of the
brake force into the suspension, shaped by the travel; the chain term is about 1.4 N per N m, shaped by the travel.

## Rake and trail, camber [D + I]

`Skeleton.RakeTrailOverride` (26.1 deg, 10.5 cm for the KTM) and `OverrideWheelBase` (145 cm) fix the geometry of the
front end, with the same construction as `docs/specs/vehicle-two-wheel.md` (steering axis tilted by the rake, trail
measured on the ground). The camber of the wheel is the lean of the chassis plus the static camber
(`CamberAtRest`, `CamberAtMaxCompression`, `CamberAtMaxExtension`, all 0 for the bikes): `camber = asin(...) + static`.
[B: `0x145150050` forms the camber from the attitude; the static terms are scaled by the compression].

## Not read

The dynamic effect of the steering geometry on the fork axis (the fork compresses along the steering axis
and the steering moves the contact point), the semi-active and hole-shot elements (`UElectronicSuspensionsVehicleElement`,
not present on the stock KTM) and the terrain noise offsets (`SurfacesTerrainOffset`, visual only).
