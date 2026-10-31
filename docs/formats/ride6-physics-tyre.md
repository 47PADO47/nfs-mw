# RIDE 6 physics data: tyres

Part of the [format index](../README.md); see [ride6-physics-data.md](ride6-physics-data.md) for the dump and
[ride6-physics-vehicle.md](ride6-physics-vehicle.md) for the wheel fields. The tyre force pipeline is in
[ride6-tyre.md](../specs/ride6-tyre.md). Examples are the KTM 450 SMR 2022 (supermotard road tyres).

## Chain of assets

```
vehicle.FrontWheel.TireData --> Base_TireData_C asset (e.g. Tire/TireData/Supermotard/FrontTyre_Road)
   .TireModel     --> DataTable of FTireModelData rows (Tire/GripCurve/Supermotard/Slick_Front), one row per surface
   .Decay         --> FDecayData (wear and temperature constants of the whole tyre)
   .TireSections  --> 3 x UTireSectionData (left shoulder, centre, right shoulder), e.g. ROAD_FRONT_SIDE, ROAD_FRONT_CENTRALE, ROAD_FRONT_SIDE
```

## `UTireData`

| Field | Meaning | KTM front / rear | Default if unset | |
|---|---|---|---|---|
| `TireModel` | grip table | `Slick_Front` / `Slick_Rear` | | |
| `UseCombinedSlip` | true: virtual solver slots give both forces (an empty stub for the MF94 class, so keep false); false: pure slip forces joined by the normalised-slip rule | not set | false | B |
| `RestLoadBlendFactor` | blend towards the rest load | not set | 0 | D |
| `TireEfficiencyBalance` | share of the efficiency loss that goes to the longitudinal force (the lateral force gets `1 - balance`) | not set | 0.5 | B |
| `Decay.TireSectionsDependency` | | not set | 1 | D |
| `Decay.TireSectionsAngleRad` | | not set | 1 rad | D |
| `Decay.TireSectionsHeatTransfer` | | not set | 0 | D |
| `Decay.Spess` | initial tread thickness (units of wear) | 1.1 | 3 | D |
| `Decay.SpessLimit` | thickness at which the tread is gone | 0.1 | 0 | D |
| `Decay.Temperature` | start temperature offset | 0 | 70 | D |
| `Decay.AsphaltTemperatureImportance` | weight of the road temperature in the tyre temperature | 1 | 0.8 | D |
| `Decay.MaxTemperature` | | not set | 200 | D |
| `Decay.NominalWearDecayLength` | distance for the nominal wear | not set | 500000 | D |

Defaults are the constructor values of `UTireData` (function `0x145146810`).

## `FTireModelData` rows (the grip table)

A `DataTable` with the row struct `TireModelData` and the rows `Tarmac`, `Kerb`, `Grass`, `Sand`, `Tarmac_Wet`, `Kerb_Wet`,
`Grass_Wet`, `Sand_Wet`, `LooseGravel`. Each row carries the physical surface (`SurfaceType1` = tarmac, `14` = kerb,
`2` = grass, `10` = sand, `6` = loose gravel) and the wetness flag `WET`. The numbering is the Magic Formula 94 of Pacejka
(lateral `A0..A13` with `A111`, `A112`; longitudinal `B0..B10`; aligning `C0..C17`), with the load in kN, camber and slip
angle in degrees, and the usual formula:

| Group | Meaning | KTM front, Tarmac |
|---|---|---|
| `A0` | lateral shape factor C | 1.2 |
| `A1`, `A2` | peak friction `D = Fz (A1 Fz + A2)` in N (Fz in kN) | -10, 1350 |
| `A3`, `A4` | cornering stiffness `BCD = A3 sin(2 atan(Fz / A4))` in N/deg | 1200, 6 |
| `A5` | camber reduction of the stiffness `(1 - A5 abs(gamma))` | 0.002 |
| `A6`, `A7` | curvature factor E = `A6 Fz + A7` | 0, 0 |
| `A8`, `A9`, `A10` | horizontal shift | -0.01, 0, 0 |
| `A111`, `A112` | vertical shift (camber thrust) `Sv = ((A111 Fz + A112) gamma + A12) Fz + A13` in N (gamma in deg) | 2, 0 |
| `A12`, `A13` | vertical shift terms | 0, 0 |
| `B0 .. B10` | the same for the longitudinal force | 1.5, -10, 1250, 0, 200, 0, -0.1, 0, 0.1, 0, 0 |
| `C0 .. C17` | aligning moment | 2.025, -1.67, -10.2, -0.3, -10, 0, 0, 0, -0.405, 0... |
| `RollingResistanceConstant`, `...Velocity`, `...SqrVelocity` | `Fz (c0 + c1 v + c2 v^2 + cS abs(slip)^p)`, `v` the vehicle speed in m/s | 0.02, 0.0004, 1e-6 |
| `RollingResistanceSlip`, `...SlipLinearity` | extra rolling resistance with slip | 0.2, 1 |
| `MaxLongitudinalGripBoost`, `MaxLateralGripBoost` | extra grip on loose surfaces | 0, 0 (gravel: lateral 1) |
| `NonLinearSolverRelativeTolerance` | relative gradient tolerance of the minimiser that finds the peak slips | 1e-4 |
| `MinWheelLoad`, `MaxWheelLoad`, `NumLoadSamples` | range (kN) and size of the precomputed load table | 0.0365, 3.6782, 50 |
| `MaxCamber`, `NumCamberSamples` | range (deg) and size of the precomputed camber table | 61, 25 |

The rear table differs from the front in `A3` 1000, `A4` 5, `B0` 1.6, `B1` -20, `B4` 150. The wet rows have a lower
`A0..A4` (the Tarmac_Wet row: `A2` 900, `A3` 1750, `A4` 4, `A111` 0, `A112` 6.5). The load and camber tables hold the
slip at the force peak for each load (longitudinal) and each load and camber (lateral); see
[ride6-tyre-mf94.md](../specs/ride6-tyre-mf94.md).

`FTireModelDataPacejkaSimplified` and `FTireModelDataMFMC` (Pacejka MF-MC, `PCX1`, `PDX1`...) exist in the module and in the
SDK but no bike asset of the install uses them.

## `UTireSectionData` (one strip of the tread)

| Field | Meaning | KTM front centre (`ROAD_FRONT_CENTRALE`) |
|---|---|---|
| `EfficiencyLimit` | lowest efficiency (grip multiplier) | 0.9 |
| `StaticTemperatureCoef` (+`Dry`, `Wet`) | heating or cooling towards the road/air temperature per second | 0.009 |
| `DynamicTemperatureCoef` (+`Dry`, `Wet`) | heating from sliding | 0.02 |
| `LongSlipToTemperatureCoef`, `LatSlipToTemperatureCoef` | heating per unit of slip power | 1e-5 |
| `RollingResistanceToTemperatureCoef` | heating from rolling resistance | 0.0002 |
| `TemperatureToWearCoefficientCurve` | wear rate against temperature (deg C) | `TireWear_TemperatureToWear_Road_Front`: (88, 0.95), (105, 0.945), (150, 0) |
| `WearToEfficiencyCurve` | efficiency against wear | `TireWear_WearToEfficiency_Road`: (0.1, 0.985), (0.49, 0.985), (0.855, 0.98), (0.917, 0.968), (1.0, 0.94) |
| `TemperatureToEfficiencyCurve` | efficiency against temperature | `..._Road_Front`: (60.3, 0.975), (90.1, 1), (105, 1), (115, 0.975) |
| `b*TemperatureWetnessInterpolationEnabled` | blend dry and wet coefficients by wetness | |

`Tire/DataTables/TerrainModifiers` scales the three heating terms per surface (tarmac 1; grass and sand 0.1 for the slip
terms).
