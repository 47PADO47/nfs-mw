# RIDE 6 tyre temperature, wear and efficiency (clean-room spec)

How the `TireWearVehicleElement` integrates the temperature and the tread wear of each tread section and turns them into the
tyre efficiency `e` used by [ride6-tyre.md](ride6-tyre.md) section 4 / [ride6-tyre-mf94.md](ride6-tyre-mf94.md) section 7.
Data: [formats/ride6-physics-tyre.md](../formats/ride6-physics-tyre.md). Sources: element update `0x14512bdd0`, temperature
step `0x145110690`, section efficiency `0x145150650`, section weights `0x145168440` / `0x145150050`, tyre setup `0x145169100`,
element constructor `0x1450fe840`; build `5.6.1-1697004`. Tags: **[B]** binary, **[D]** data, **[I]** inferred.

## 1. State and defaults

Per wheel, a list of **sections** (the `TireSections` of the tyre data, three for a bike: left shoulder, centre, right
shoulder). Each section holds the temperature `T_s` (deg C), the tread thickness `h_s` (`Spess` units) and a weight `p_s`
(sum 1). At setup `T_s = Decay.Temperature` (KTM: 0), `h_s = Decay.Spess` (1.1), `p_s = 1` until the first weight update. [B]
The wheel also keeps the efficiency `e` (`wheel+0x318`, set to 1 whenever the element is disabled or `UpdateEfficiency` is
off) and an external factor `m` (`wheel+0x31C`, 1 at setup; the wetness/other multiplier). [B]

Element settings (`UTireWearVehicleElement`; constructor defaults [B], KTM asset `Base_TireWear` [D]):

| Field | C++ default | KTM / all bikes |
|---|---|---|
| `Enabled` | true | true |
| `UpdateEfficiency` | true | true |
| `UniformTiresComputation` | false | **true** |
| `StaticTemperatureTimeScale` | 0.1 | 1.0 |
| `DynamicTemperatureTimeScale` | 1e-6 | 1.0 |
| `TireDecayDistanceFactor` | 1.0 | 1.0 (a tuning table scales it per compound) |
| `WearDecayIntensity` | 1.0 | 1.0 (idem) |

Tyre decay data (`UTireData.Decay`, defaults [B: `0x145146810`]): `TireSectionsDependency 1`, `TireSectionsAngleRad 1`,
`TireSectionsHeatTransfer 0`, `Spess 3` (KTM 1.1), `SpessLimit 0` (KTM 0.1), `Temperature 70` (KTM **0**: the floor and the
start temperature), `AsphaltTemperatureImportance 0.8` (KTM 1), `MaxTemperature 200`, `NominalWearDecayLength 500000`.
`SpessLimit` is not used by the update path read here (only the efficiency floor `EfficiencyLimit` bounds the loss). [B]

Inputs from the world: `AirTemperature` and `AsphaltTemperature` of the vehicle (written by the game from the weather state at race set-up and
on each weather broadcast; the values come from a per-track, per-weather data table, no constructor default exists:
[ride6-chassis-round6.md](ride6-chassis-round6.md) section 5),
the `Slipstream` value ([ride6-aero-drag.md](ride6-aero-drag.md)) and `SlipstreamTemperatureFactor` (0 by default).

## 2. Section weights across the width [B: `0x145168440`]

The contact patch is described by the wheel's camber angle `c` (rad, `asin` of the wheel axis against the ground normal,
clamped to the section span). With `n` sections, half-span `A = TireSectionsAngleRad`, `dep = TireSectionsDependency`:

```
step     = 2 A / (n - 1)
a_i      = -A + i step                 (i = 0 .. n-1)       # section centres, radians
w        = step * dep
c'       = clamp(c, -(A + 0.9999 w), +(A + 0.9999 w))
p_i      = max(0, 1 - |a_i - c'| / w)    then   p_i /= sum p                # tent weights
```

For `n = 3`, `A = dep = 1`: centres at -1, 0, +1 rad; an upright wheel (c = 0) loads only the centre section; at 0.5 rad of
lean the centre and the leaned shoulder share the load equally. With `n = 1` the weight stays 1. [B]

## 3. Temperature step [B: `0x145110690`]

Per step `dt`, per wheel with the tyre decay data `D`, per section `s` with its data `S` (`UTireSectionData`):

```
T_air   = Slipstream * SlipstreamTemperatureFactor + AirTemperature
T_ss    = (1 - D.AsphaltTemperatureImportance) T_air + D.AsphaltTemperatureImportance * AsphaltTemperature
nx, ny  = min(|normalised long slip|, 10), min(|normalised lat slip|, 10)           # the nx, ny of the tyre spec section 3
Rr      = |rolling resistance force vector of the wheel|        (Unreal force units)
if the wheel spin is negative: Rr = nx = ny = 0
mu_x, mu_y, mu_r = terrain modifier row of the surface under the wheel (TerrainModifiers table, default 1, 1, 1)
k_s     = S.StaticTemperatureCoef,  or lerp(Dry, Wet, wetness) if bStaticTemperatureWetnessInterpolationEnabled
k_d     = S.DynamicTemperatureCoef, or lerp(Dry, Wet, wetness) if bDynamicTemperatureWetnessInterpolationEnabled
H       = Fz ( S.LatSlipToTemperatureCoef mu_y ny + S.LongSlipToTemperatureCoef mu_x nx ) + S.RollingResistanceToTemperatureCoef mu_r Rr
heat    = exp(-H k_d dt DynamicTemperatureTimeScale)
cool    = exp(-k_s dt StaticTemperatureTimeScale)
T_s'    = T_s + (1 - heat) p_s (D.MaxTemperature - T_s) + (1 - cool) (T_ss - T_s)
T_s'    = clamp(T_s', D.Temperature, D.MaxTemperature)
```

`Fz` is the wheel load in Unreal force units (`wheel+0x154`), so with `dt` in seconds the exponent for typical numbers
(KTM front centre: `k_d 0.02`, coefficients 1e-5) is small; this is the model as coded. The section weight `p_s` scales
the heating of that section only (a leaned wheel heats its shoulder).

**Lateral heat transfer** (only if `D.TireSectionsHeatTransfer = h > 0` and `n > 1`): for `i = 1 .. n-1`, in order,
with `T_prev` the temperature of section `i-1` **before** this pair is processed and `T_i` that of section `i`:

```
d  = (T_i - T_prev) / 2 ;   e = sqrt(|d|) - dt h
if e > 0:  d = d - sign(d) e^2                   # the transfer is limited to e0^2 - e^2 with e0 = sqrt(|d|)
T[i-1] += d ;   T[i] -= d
```

so a pair relaxes to equal temperature and the rate is a square-root law. KTM: `h = 0` (default), so no transfer.

**Uniform mode** (`UniformTiresComputation`, true for all bikes) [B: `0x14512bdd0`]: after the update of a wheel, every
section gets the arithmetic mean of the temperatures and the mean of the thicknesses of that wheel (unweighted).

## 4. Wear step [B: `0x14512bdd0`]

Run before the temperature step, for a wheel in contact, per section:

```
v     = max( | v_contact - (r_eff w) dir | , | v_contact | )            # slip speed or contact speed, cm/s
L     = max( D.NominalWearDecayLength * TireDecayDistanceFactor , 1e-4 )                    # cm
rate  = TemperatureToWearCoefficientCurve_S(T_s)        # a number just below 1 (KTM front: 0.95 at 88 C, 0.945 at 105 C, 0 at 150 C)
f     = rate ^ (v dt / L)        (f = 0 when TireDecayDistanceFactor = 0)
h_s  <- h_s * ( 1 + WearDecayIntensity * p_s * (f - 1) )
```

i.e. the thickness decays exponentially with the sliding distance: after a distance `L` at constant temperature the loaded
section keeps the fraction `rate`. `v` is the larger of the slip speed and the contact point speed, so a rolling tyre wears at
its contact speed and the wear distance is simply the distance travelled (or slid, if more). The wear is not reduced below
`SpessLimit` in this path. The whole wear and temperature block runs only while a game-mode flag read through the game
state (virtual `0x808` of the owning object) is true; its source was not traced, assume true. [B]

## 5. Efficiency [B: `0x1451736e0`, `0x145150650`]

```
x_wear  = Spess_initial - h_s                                  # worn amount, 0 for a new tyre
e_s     = clamp( TemperatureToEfficiencyCurve_S(T_s) * WearToEfficiencyCurve_S(x_wear) * m , EfficiencyLimit_S , 1 )
e       = sum_s  p_s e_s
```

(a missing curve counts as 1; a null section gives `clamp(m, 0, 1)`). KTM front road tyre: temperature curve (60.3 C: 0.975,
90.1 C: 1, 105 C: 1, 115 C: 0.975), wear curve (0.1: 0.985 ... 1.0: 0.94), `EfficiencyLimit 0.9`, so `e` is between 0.9 and 1 and
stays within a few percent for a normal run. The curves are Unreal rich curves (cubic with stored tangents on the first
segments, linear where marked); the dump gives the points and tangents. The efficiency enters the force as in
[ride6-tyre-mf94.md](ride6-tyre-mf94.md) section 7.

## 6. What a bike model needs from this

A minimal faithful version keeps one temperature and one thickness per tyre (uniform mode), the weights only matter for the
heating split when `UniformTiresComputation` is false. With air and asphalt temperature of a normal day the tyre warms from
`D.Temperature` towards the target `T_ss` and the dynamic heating; start temperature and floor are 0 for the KTM tyres, so
the efficiency at start is the cold value of the temperature curve (0.975 at <= 60 C for the road tyres).
