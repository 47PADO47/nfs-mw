# RIDE 6 air drag and slipstream (clean-room spec)

The vehicle-level drag force and the multi-vehicle slipstream element. It replaces the "air drag" paragraph of
[ride6-rider-body-aero.md](ride6-rider-body-aero.md) (whose factor 2 was wrong) and item M11 of
[vehicle-two-wheel-parameters.md](vehicle-two-wheel-parameters.md). Sources: vehicle force function `0x1451001b0`, drag
`0x145105ed0`, aerodynamic data constructor `0x1450f9cb0`, slipstream element update `0x1450de770`, contribution
`0x1450cca50`, setter `0x1450c60a0`, constructor `0x1450bdcb0`; build `5.6.1-1697004`. Tags: **[B]** binary, **[D]** data.

## 1. Drag force [B: `0x145105ed0`]

The engine works in centimetres and kg, so a force of 100 Unreal units is 1 newton. With `v` the velocity of the rigid body
(cm/s), `|v|` its speed (`Speed`, cm/s) and `Cd = Aerodynamic.DragCoefficient`:

```
F_drag (Unreal units) = -0.01 * Cd * |v| * v * (1 - s) * g(theta)         # |v| v in (cm/s)^2
in SI:   F_drag = - Cd * |v| * v * (1 - s) * g(theta)                     # newtons, v in m/s
```

The constant is `-0.01` (read from the binary: the double `0xBF847AE140000000`); with `(100 v)^2` and 100 units per newton
the result is exactly **`Cd v^2` newtons**, no factor 2. The force is applied to the rigid body (`FORCE_Drag`) opposite to the
velocity. KTM 450: `Cd = 0.15` (the C++ default is **0.35**, `0x3eb33333`; every bike asset sets it, 0.1 to 0.3).

- `s = clamp( Slipstream * SlipstreamScale , 0, 1)` where `Slipstream` is the vehicle's wake value (section 4) and
  `SlipstreamScale` the aerodynamic field (default **1.0**).
- `g(theta)` = `VehicleOrientationDragTuning` evaluated at `theta`, the absolute angle in degrees between the chassis forward
  axis and the velocity direction (`acos` of their dot product, clamped to [-1, 1]). The tuning is a clamped range
  translator (`FClampedRangeTranslator`): if the translator's curve is not set, `alpha = clamp01((theta - In.x)/(In.y - In.x))`
  (if `|In.y - In.x| < 1e-8`: 1 when `theta >= In.y`, else 0), `g = alpha^Linearity (Out.y - Out.x) + Out.x`. The C++ defaults are
  `InputRange = (0, 0)`, `OutputRange = (1, 1)`, `Linearity = 1`, `bAlphaCenteredRanges = false`, empty curve: so **`g = 1`
  for every bike whose asset does not set the tuning** (all road bikes read). With a curve set, the curve (x in degrees) is
  evaluated instead; with `bAlphaCenteredRanges` a two-segment mapping about `InputAlpha`/`OutputAlpha` is used (not used by
  any asset read).
- The rider's own drag (rider curve, tuck-in) is a separate term, see [ride6-rider-body-aero.md](ride6-rider-body-aero.md).
- `Wings` (`FWingData`, empty array by default) are not part of this function.
- **Water drag** (adds to the drag, only above 1 m/s of speed): for each wheel, `F -= wet_w * WaterDragCoefficient * |v| * v`
  in the same units, `wet_w` the wheel's wetness (`wheel+0x314`), `WaterDragCoefficient` the *aerodynamic* field (default 0;
  the tyre has its own spin drag torque, section 2). [B]

SI form for the model: `F = Cd (1 - s) g(theta) v^2` opposing the velocity. `Cd` is `0.5 rho CdA`; `CdA = 0.25 m^2` for the KTM at
`rho = 1.2`.

## 2. Other forces of the same function

`0x1451001b0` also applies the rolling resistance of ride6-tyre-mf94.md section 6 (per wheel, opposite to the velocity
direction, only above 1 m/s of speed) and the drag above; the spin drag torque of the wheel is
`WaterDragCoefficient_tyre * wet * w^2` (`wheel+0x19c`). [B]

## 3. Slipstream element [B]

`USlipstreamPhysicsElement` (a world-level element, not part of one bike): every tick, for each simulated vehicle `i`, the
element takes the maximum contribution of every other vehicle `j` and stores it as `i.Slipstream`, clamped to [0, 1]
(setter `0x1450c60a0`; the vehicle field is at `+0x1A10`).

Contribution of `j` to `i` (positions in metres = cm * 0.01; `u` = unit velocity of `j`; `speed_j` in m/s):

```
if speed_j < 3:  0
d      = (pos_i - pos_j) * 0.01
a      = d . u                          # along the wake axis; negative = i is behind j
L      = SlipstreamMaxDistance                      if SlipstreamPersistence <= 0
       = SlipstreamPersistence * speed_j            otherwise
require  -L < a < 0, else 0
r      = d - u a ;   X = |r|            # lateral distance
if bSlipstreamUseVehicleWidth: X = max(0, X - (AxleTrack_j + wheel_extent_j) * 0.005)
if SlipstreamPersistence > 0:  a = a * (100 / L)
contribution = exp( - ( -a * SlipstreamDistanceFactor )^SlipstreamDistanceExponent
                    - X^SlipstreamSideDistanceExponent * SlipstreamSideDistanceFactor )
```

Values: C++ defaults (`0x1450bdcb0`) `Persistence 0`, `MaxDistance 100`, `Factor 0.95`, `DistanceFactor 0.05`,
`DistanceExponent 1`, `SideDistanceFactor 1.5`, `SideDistanceExponent 2`, `bUseVehicleWidth false`, `Multiplier 1`; the
element asset (`SlipstreamPhysicsElementBP`) sets `MaxDistance 70`, `DistanceFactor 0.085`, `SideDistanceFactor 2.0`. So the
wake behind a vehicle going faster than 3 m/s is `exp(-0.085 |a| - 2 X^2)` for `|a| < 70 m`: 0.43 at 10 m straight behind,
0.014 at 50 m; 0.26 at 10 m with 0.5 m of lateral offset. `SlipstreamFactor` (`+0x68`) and `SlipstreamMultiplier` (`+0x80`) are
not used by the update path read; if they scale the value it is elsewhere (not found). [B]

The same `Slipstream` also feeds the tyre air temperature
(`Slipstream * SlipstreamTemperatureFactor + AirTemperature`, factor 0 by default, see ride6-tyre-thermal.md) and rider help.
For a single bike on an empty road, `Slipstream = 0` and the drag is `Cd v^2`.
