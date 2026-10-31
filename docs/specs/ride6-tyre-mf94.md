# RIDE 6 tyre model object: Magic Formula 94 evaluation (clean-room spec)

How the tyre model object that sits behind one `Tarmac` (or `Kerb`, ...) row of a `TireModel` table is evaluated at run
time. This replaces the guesses of section 3, 5, 6 and 7 of [ride6-tyre.md](ride6-tyre.md) (which keeps the wheel-level
pipeline: slip, normalised slip, combination, efficiency). Data fields: [formats/ride6-physics-tyre.md](../formats/ride6-physics-tyre.md).
Sources: binary build `5.6.1-1697004` (addresses are image-relative, base `0x140000000`), listed in
[provenance/ride6-physics.md](../provenance/ride6-physics.md). Tags: **[B]** read from the binary, **[D]** from the data,
**[I]** inferred.

## 1. The object

The row struct `FTireModelData` has a vtable; the row itself is the evaluator (no separate allocation). The wheel holds one
object per surface class for dry and one for wet (`wheel+0x430` dry, `+0x438` wet) and blends every result linearly by the
wetness `w` (`result = dry + (wet - dry) w`; for the peaks both slips and for the boost getters too). [B]
Virtual slots of the MF94 class (vtable `0x149e43448`): [B]

| Slot | Function (rva) | Meaning |
|---|---|---|
| `0x18` | `0x14515a4a0` | if the "dirty" flag is set, clear it and call slot `0x20` (the table build, section 5) |
| `0x20` | `0x1451620b0` | build the peak tables |
| `0x28`, `0x30` | `0x14514e020`, `0x14514d5b0` | peak force `D(Fz)` longitudinal / lateral (the factor `D` of section 2) |
| `0x48` | `0x14514dd00` | `Fx(slip, Fz)` |
| `0x50` | `0x14514d190` | `Fy(slip, Fz, camber)` |
| `0x58` | `0x14514c180` | `Mz(slip, Fz, camber)` |
| `0x60` | `0x14514e710` | rolling resistance (section 6) |
| `0x68`, `0x70` | `0x14514e630`, `0x14514dbd0` | stiffness `BCD` longitudinal / lateral (accessors) |
| `0x78`, `0x80` | `0x141271660` | **empty stub** (the combined-slip solver slots: the MF94 class has none) |
| `0xC8`, `0xD0` | `0x14514e420`, `0x14514d9d0` | peak slips (negative, positive) longitudinal / lateral (section 4) |
| `0xD8`, `0xE0` | `0x145158490`, `0x145158480` | `MaxLongitudinalGripBoost`, `MaxLateralGripBoost` (plain getters of row `+0xD4`, `+0xD8`) |

Consequence: with an MF94 table `UTireData.UseCombinedSlip` must stay false (the combined branch would call the stub). The
KTM never sets it. `NonLinearSolverRelativeTolerance` is **not** a combined-slip tolerance: it is the tolerance of the
peak-slip minimiser (section 4). [B]

## 2. Units and the pure force function

Inputs from the wheel, converted by the model: load `Fz` in newtons -> `F = Fz / 1000` (kN); longitudinal slip ratio
`kappa` -> `x = 100 kappa` (percent); slip angle `alpha` (rad) -> `x = alpha * 57.2958` (degrees); camber `gamma` (rad) ->
degrees. The returned force is in newtons (the wheel multiplies by 100 for Unreal force units). [B]

The shape function with the six parameters `(B, C, D, E, Sh, Sv)` is, for all three curves,

```
u(x)  = B (x + Sh)
y(x)  = E atan(u) + (1 - E) u                     # = u - E (u - atan u)
Force = D sin(C atan(y)) + Sv
```

[B: `0x145154a70`; the code computes `atan(u)`, then `atan(atan(u) E + (1-E) u)`, then `sin(C .)`, times `D`, plus `Sv`.]

Longitudinal (`b0..b10` = `B0..B10` of the row, `F` in kN) [B: `0x145151aa0`; it returns zero parameters if `F < 1e-8`]:

```
C   = b0
D   = (b1 F + b2) F
BCD = (b3 F^2 + b4 F) exp(-b5 F)
B   = BCD / (C D)
E   = b6 F^2 + b7 F + b8
Sh  = b9 F + b10            Sv = 0
Fx  = Force(100 kappa)
```

Lateral (`a0..a13` = `A0, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A111, A112, A12, A13` of the row in the file order;
`gamma` in degrees) [B: `0x145151960`]:

```
C   = a0
D   = (a1 F + a2) F
BCD = a3 sin(2 atan(F / a4)) (1 - |gamma| a5)
B   = BCD / (C D)
E   = a6 F + a7
Sh  = a8 gamma + a9 F + a10
Sv  = ((a111 F + a112) gamma + a12) F + a13
Fy_model = Force(alpha in degrees)
```

Note the camber term: `Sv` has `a111 F^2 gamma + a112 F gamma + a12 F + a13`, and `Sh` takes `A8` times the camber (the
earlier data page listed these in a different grouping). The wheel negates the lateral model output (`Fy_wheel = -Fy_model`)
[B: `0x14516ebb0`, the sign flip after the call to `0x14514d4f0`]. The lateral slip handed to the model is the wheel's
`Slip.y` (the sine of the slip angle) used as if it were the angle in radians. [B]

Aligning moment (slot `0x58`; only the front wheel uses it), same shape with [B: `0x145175ad0`]:

```
C   = c0
D   = c1 F^2 + c2 F
B   = exp(-c5 F) (c3 F^2 + c4 F) (1 - c6 |gamma|) / (C D)
E   = (c7 F^2 + c8 F + c9) (1 - c10 |gamma|)
Sh  = c11 gamma + c12 F + c13
Sv  = (c14 F^2 + c15 F) gamma + c16 F + c17
Mz  = Force(alpha in degrees)                    # newton metres
```

**Test values** (KTM 450 SMR front tyre, row `Tarmac`, computed from these equations, not measured in the game):
`F = 1 kN`: `B_x = 0.10753, D_x = 1240 N, E_x = 0`; `Fx(10 %) = 1169.71 N`. Lateral at `F = 1 kN, gamma = 0`:
`B = 0.24203, D = 1340 N, E = 0, Sh = 0`; `Fy_model(5 deg) = 1166.49 N`. At `gamma = 30 deg`: `Sh = -0.3 deg, Sv = 60 N`,
`B = 0.22751`, `Fy_model(5 deg) = 1174.84 N`, `Fy_model(-5 deg) = -1105.08 N`.

## 3. Peak slips (the normalised-slip anchors)

Definition: for a shape with parameters `(B, C, D, E)` the peak is the extremum of `D sin(C atan(y(x)))` for `x >= 0`
without `Sh` and `Sv`. It exists for `C > 1` and is where `C atan(y) = pi/2`, that is the positive root `x_s` of

```
E atan(B x) + (1 - E) B x = tan(pi / (2 C))
```

(monotone for `0 <= E <= 1`; for other `E` the binary's minimiser finds the first extremum from the start point below).
The wheel asks for the **actual slip values** of the two peaks, with the shift put back [B: `0x14514e420`, `0x14514d9d0`]:

```
sx+ = ( x_s - Sh) * 0.01        sx- = (-x_s - Sh) * 0.01             # longitudinal, ratio
sy+ = ( x_s - Sh) * 0.0174533   sy- = (-x_s - Sh) * 0.0174533        # lateral, radians; Sh and x_s in degrees
```

`x_s` comes from the table of section 4 or, outside its range, from the solver. The wheel then uses `sx-`, `sx+`, `sy-`,
`sy+` for the normalised slip (ride6-tyre.md section 3), stores `max(|sx-|, |sx+|)` and `max(|sy-|, |sy+|)` for the
thermal model, and blends them dry/wet. Test values (same tyre, `F = 1 kN`): `x_s = 16.1081 %` longitudinal, `x_s = 15.4196
deg` lateral at `gamma = 0` (so `sx+ = 0.161081`, `sy+ = 0.26913 rad`); at `F = 2 kN, gamma = 45 deg`: `x_s = 18.1818 deg`.
(Computed with the root equation above; the game's numeric minimiser stops at a relative gradient of 1e-4.)

## 4. The numeric solver and the precomputed tables

**Solver** [B: `0x1451578b0`, `0x14516e5d0`, `0x14515d600`, `0x145150e20`, `0x145152140`, `0x14515ecc0`]. A one-dimensional
minimiser of `D sin(C atan(y(x)))` (the force without shifts) with value and exact derivative, starting at
`x0 = -1 / (B C)` (if `|B C| < 1e-4`: 0), a Hermite-cubic / secant bracketing step, at most 100 iterations,
relative gradient tolerance `NonLinearSolverRelativeTolerance` (1e-4 in the KTM tables), absolute tolerance
`3.45e-4 = sqrt(2^-23)`. The result is `|x*|` (the minimum is the negative peak; the shape is odd so the positive peak has the
same magnitude). Because the force is analytic, **our implementation may use the root equation of section 3** (bisection)
instead of the minimiser; the differences are below the tolerance.

**Tables** [B: `0x1451620b0`]. At load (and when a property changes) the row builds, for `N = max(NumLoadSamples, 2)` loads
`F_i = MinWheelLoad + i dF` with `dF = (MaxWheelLoad - MinWheelLoad) / (N - 1)` (kN), the value `x_s` of the **longitudinal**
parameters (a 1-D table `T1[i]`), and for `M = max(NumCamberSamples, 2)` cambers `gamma_j = j dG`, `dG = MaxCamber / (M - 1)`
(degrees), the `x_s` of the **lateral** parameters (a 2-D table `T2[i][j]`). The flag "tables valid" is then set.
Lookup, used by the peak accessors (arguments: `F` in kN and, for lateral, camber in degrees) [B: `0x1451578b0`]:

```
if tables valid and MinWheelLoad <= F < MaxWheelLoad:
    s = F - MinWheelLoad ; i = int(s / dF)                       # 0 <= i <= N-2
    t = (s - i dF) / dF
    longitudinal:  x_s = (1-t) T1[i] + t T1[i+1]
    lateral:       require |gamma| < MaxCamber ; j = int(|gamma| / dG) ; v = (|gamma| - j dG) / dG
                   x_s = (1-t) [(1-v) T2[i][j]   + v T2[i][j+1]]
                       +    t  [(1-v) T2[i+1][j] + v T2[i+1][j+1]]
otherwise: run the solver on the parameters of the exact (F, gamma)        # no clamping
```

So the load is **not** clamped to the table range (the earlier note was wrong): outside `[MinWheelLoad, MaxWheelLoad)` or
beyond `MaxCamber` the exact solve is used. The camber sign is dropped (`|gamma|`); the shift `Sh` uses the signed camber.
KTM rows: `MinWheelLoad 0.0365 kN`, `MaxWheelLoad 3.6782 kN`, `NumLoadSamples 50`, `MaxCamber 61 deg`, `NumCamberSamples 25`.

## 5. Grip boost getters and where the wheel uses them

Slots `0xD8` / `0xE0` return `MaxLongitudinalGripBoost` / `MaxLateralGripBoost` (0 for tarmac, lateral 1 for gravel). The wheel
multiplies `Fx` by `1 + boost_x * h_x` and `Fy` by `1 + boost_y * h_y`, where `h_x`, `h_y` are `LongitudinalHelpFactor` and
`LateralHelpFactor` of the vehicle's `FWheelData` (`+0xF4`, `+0xF8`; wheel-data defaults, see the C++ default list in
[vehicle-two-wheel-parameters.md](vehicle-two-wheel-parameters.md)). [B: `0x14516ebb0`] A row boost has no effect when the
help factor is 0.

## 6. Rolling resistance

Slot `0x60`: `RR(s, Fz, v) = Fz (c0 + (c2 v + c1) v + cS |s|^p)` with `c0 = RollingResistanceConstant`, `c1 = ...Velocity`,
`c2 = ...SqrVelocity`, `cS = RollingResistanceSlip`, `p = RollingResistanceSlipLinearity`. The arguments are, in order, the
wheel's `Slip.y` (lateral slip, abs taken), the wheel load (Unreal force units, no kN conversion; `RR` is returned in the same
units) and the **vehicle speed in m/s** (`Speed * 0.01`, not the wheel speed). [B: model `0x14514e710`; call `0x14514e7f0`
from the vehicle force function `0x1451001b0`.] Dry and wet models are blended by the wetness. The vehicle applies it only above 1 m/s
(below, all wheel rolling forces are set to zero), per wheel in contact, as a force **opposite to the vehicle velocity
direction** applied at the contact point (`FORCE_RollingResistance`), and the same force magnitude feeds the tyre heating
(thermal spec). KTM front row: `c0 = 0.02, c1 = 0.0004, c2 = 1e-6, cS = 0.2, p = 1`, so at 30 m/s and `Fz = 1000 N` the
rolling force is `1000 (0.02 + 0.012 + 0.0009) = 32.9 N` without slip.

## 7. Wheel-level corrections to ride6-tyre.md

- **Efficiency factor.** The factors are `Fx *= 1 + 2 min(b, 0.5)(e - 1)` and `Fy *= 1 + 2 min(1 - b, 0.5)(e - 1)`, with
  `b = TireEfficiencyBalance` (0.5) and `e` the tyre efficiency of the thermal model. The `min` is against the **constant
  0.5**, not against the combined normalised slip `c` as written earlier (verified in the disassembly: the constant is loaded
  once and never overwritten). With `b = 0.5` both factors are `e`. [B: `0x14516f1ae` to `0x14516f20e`]
- **Order of the final scale**: `Fx_final = (1 - r_x) Fx_model k_x (1 + boost_x h_x) s_x (1 + 2 min(b,.5)(e-1))` with
  `k_x = LongitudinalGripScale` of the tyre data, `r_x`, `s_x` the wheel's runtime reduction and scale (`+0x468`, `+0x460`);
  the lateral force likewise with `LateralGripScale`, `r_y` (`+0x46C`), `s_y` (`+0x464`) and the sign flip of section 2.
