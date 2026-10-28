# RIDE 6 tyre force pipeline (clean-room spec)

How a wheel of `ignitionphysics` turns the contact state into a force, written in our own words. Data fields:
[ride6-physics-tyre.md](../formats/ride6-physics-tyre.md) and [ride6-physics-vehicle.md](../formats/ride6-physics-vehicle.md).
Mapping to our model: [vehicle-two-wheel-parameters.md](vehicle-two-wheel-parameters.md). Sources:
[provenance](../provenance/ride6-physics.md). Tags: **[B]** read from the binary (function `0x14514ce70`, the wheel force
update, and its helpers), **[I]** inferred, **[D]** data only.

Units inside the simulation: lengths cm, forces in Unreal units (0.01 N); the tyre model objects take the load in newtons
(`load_uu * 0.01`) and return newtons that the wrapper multiplies by 100. [B]

## 1. Effective radius and contact

```
r_eff = R - 0.5 * (1 - |cos(camber)|) * ShapeToroidFactor * ShapeWidth - TireCompressionShapeFactor * tyre_compression
```

`R` and `ShapeWidth` are the tyre's radius and width (from the wheel mesh, not in the data), `camber` the angle between the
wheel plane and the contact normal. A leaned wheel therefore stands on the shoulder (`ShapeToroidFactor` 0.9 front, 0.7
rear for the KTM). `tyre_compression` is the share of the ground penetration taken by the tyre: [B: `0x14514f3e0`]

```
penetration     = CompressionToContact - suspension_compression                    # >= 0 when touching
share           = min(1, (SpringStrength / max(LeverageRatio-term, 1)) / TireStiffness)   # softer element takes more; wheel-side rate
tyre_compression = min(share * CompressionToContact, MaxTireDeformation)
suspension_compression = max(CompressionToContact - tyre_compression, suspension_compression)
```

(`TireStiffness` 400000 N/m, `MaxTireDeformation` 3 cm for the KTM.) The compression speed of the suspension is taken from
the ground contact velocity along the travel axis.

## 2. Slip

With `v_long`, `v_lat` the contact point velocity along the wheel's forward and right axes (cm/s) and `w` the spin:

```
slip_speed_long = (r_eff * w - v_long) * 0.01              # m/s
slip_speed_lat  =  v_lat * 0.01
V               = |contact velocity| * 0.01                # m/s
if V > 1:   Slip.x = slip_speed_long / V ;  Slip.y = slip_speed_lat / V       # = slip ratio and sin(slip angle)
else:       u = 0.99 V + 0.01 ; k = (3 - 2u) u ;  Slip.x = k * slip_speed_long ;  Slip.y = k * slip_speed_lat
```

[B] (the denominator is the total contact speed, not the longitudinal speed; below 1 m/s the slip is the slip *speed*
scaled by a smoothstep `k` that goes from 0.0297 at rest to 1 at 1 m/s, which removes the singularity). `Slip.x` is the slip
ratio, `Slip.y` the slip-angle sine.

## 3. Normalised slip and the combined slip

The tyre model gives, for the current load (and camber), the slips at which the pure forces peak: a negative and a
positive peak for the longitudinal slip (`sx-`, `sx+`) and for the lateral slip (`sy-`, `sy+`) (two virtual calls of the
model object, `0xC8` and `0xD0`; the values are interpolated in the precomputed load/camber tables, see section 6). [B] The
equations, the table lookup (the load is **not** clamped) and the solver are in [ride6-tyre-mf94.md](ride6-tyre-mf94.md).

```
nx = 2 (Slip.x - sx-) / (sx+ - sx-) - 1       # -1 at the negative peak, +1 at the positive peak
ny = 2 (Slip.y - sy-) / (sy+ - sy-) - 1
```

Two paths, chosen by `UTireData.UseCombinedSlip` (false for the stock bikes):

**Pure slip with a normalised combination (default).** [B]

```
c = sqrt(nx^2 + ny^2) / (|nx| * SlipCorrectionMultiplier * ActualSlipCorrection + 1)     # combined normalised slip
if c > 1e-8:   # the effective pure slips along the same direction
    sx_eff = (c * sgn(nx) / 2 + 0.5) * (sx+ - sx-) + sx-        # sgn(0) = 0
    sy_eff = (c * sgn(ny) / 2 + 0.5) * (sy+ - sy-) + sy-
Fx0 = Fx_model(sx_eff, Fz) * LongitudinalGripScale          # virtual 0x48
Fy0 = Fy_model(sy_eff, Fz) * LateralGripScale               # virtual 0x50
Fx  = Fx0 * min(1, |nx| / c)          # direction cosines in normalised space
Fy  = Fy0 * min(1, |ny| / c)
```

`ActualSlipCorrection` is the wheel's `SlipCorrection` (0.05) blended to `RestSlipCorrection` (0.25) at low speed over
`RestSpeedRange` and `RestSpeedThreshold` [I]; `SlipCorrectionMultiplier` is a runtime factor (wet, burnout).

**Combined slip.** With `UseCombinedSlip` the wheel calls virtuals `0x78` (Fx) and `0x80` (Fy) of the model object and scales the
results by the grip scales. For the MF94 class (`FTireModelData`) both slots are an **empty stub** (no solver exists), so that
path gives no force for these tables; the stock bikes (`UseCombinedSlip` false) are unaffected. `NonLinearSolverRelativeTolerance`
belongs to the peak-slip minimiser ([ride6-tyre-mf94.md](ride6-tyre-mf94.md) section 4). [B]

## 4. Efficiency (temperature and wear) and help factors

The three tread sections (left shoulder, centre, right shoulder, see the tyre data page) each hold a temperature and a
wear value and give an efficiency `e` in `[EfficiencyLimit, 1]` from the product of the temperature and wear curves.
The wheel's `TireEfficiency` is their mix by the contact patch position; with `b = TireEfficiencyBalance` (0.5):

```
Fx *= 1 + 2 min(0.5, b)     (e - 1)        # e the tyre efficiency; the min is against the constant 0.5
Fy *= 1 + 2 min(0.5, 1 - b) (e - 1)
Fx *= (1 + g_x * LongitudinalHelpFactor)   # g_x, g_y: model virtuals 0xD8, 0xE0 (grip boost terms of the row)
Fy *= (1 + g_y * LateralHelpFactor)
Fx *= m_x ; Fy *= m_y                      # two runtime grip multipliers of the wheel, each as (1 - reduction) times a scale
```

[B] for the factor (verified in the disassembly: the constant 0.5 replaces the earlier reading `c`, the combined normalised slip;
with `b = 0.5` both forces are multiplied by `e`). `g_x`, `g_y` are `MaxLongitudinalGripBoost`, `MaxLateralGripBoost` of the row and
`LongitudinalHelpFactor`, `LateralHelpFactor` are `FWheelData` fields (0 for the KTM). [B] The runtime multipliers `m_x`, `m_y`
(`(1 - reduction)` times a scale, wheel `+0x468`/`+0x460` and `+0x46C`/`+0x464`) carry the wet and burnout grip reductions [I].
The efficiency `e` itself: [ride6-tyre-thermal.md](ride6-tyre-thermal.md).

## 5. Aligning moment, rolling resistance

- The aligning moment (virtual `0x58`) is computed **only for steered wheels** (`SteerAngle > 0`: the front wheel); it is
  accumulated in `AligningTorqueAccumulator` and averaged over the steps (`AligningTorqueRestLoadBlend` blends towards the
  rest load). The rear wheel has none. [B]
- Rolling resistance [B: `0x14514e7f0`, `0x14514e710`, `0x1451001b0`]: `RR = Fz (c0 + c1 v + c2 v^2 + cS |Slip.y|^p)` with the row's
  `RollingResistance*` (`p` = `...SlipLinearity`), `Fz` the wheel load and `v` the **vehicle** speed in m/s; applied only above
  1 m/s, opposite to the velocity, at the contact point; dry/wet blended by the wetness. Details and a worked value in
  [ride6-tyre-mf94.md](ride6-tyre-mf94.md) section 6. The water drag torque is `WaterDragCoefficient * wetness * w^2` [B].

## 6. The Magic Formula evaluation and the precomputed tables [B: `0x145151960`, `0x145151aa0`, `0x145154a70`, `0x1451578b0`]

The model object of a `TireModelData` row evaluates the published Magic Formula 94 (row fields `A0..A13`, `B0..B10`, `C0..C17`)
with load in kN, longitudinal slip in percent and slip angle and camber in degrees, forces back in newtons; the coefficient
formulas (the same as the data page), the pure-force function `D sin(C atan(E atan(B x) + (1 - E) B x)) + Sv` with `x = slip + Sh`,
the peak-slip definition, the numeric minimiser, the 1-D/2-D peak tables with their lookup and the wet blend are all in
[ride6-tyre-mf94.md](ride6-tyre-mf94.md). Two corrections to the earlier reading: the load is **not** clamped to
`[MinWheelLoad, MaxWheelLoad]` (outside it, and beyond `MaxCamber`, the peak is solved exactly), and the coefficients are zero
when `F < 1e-8` kN. The peaks `sx-`, `sx+`, `sy-`, `sy+` of section 3 are the slips of the extremal force on each side with the
shift put back. The inverse of the lateral curve (used by the lean controller) is in the lean spec.

## 7. What is still a choice

Nothing of the tyre object is open any more: the aligning moment (front wheel only) is the MF94 `C0..C17` formula of the mf94
page, the rolling resistance is section 5, the temperature and wear integration is
[ride6-tyre-thermal.md](ride6-tyre-thermal.md). Not read: the wheel's `SlipCorrectionMultiplier` / grip
reduction writers (wet, burnout) and the source of the wheel's external efficiency factor `m` (`wheel+0x31C`).
