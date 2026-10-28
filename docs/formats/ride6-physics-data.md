# RIDE 6 physics data assets: how to read them

Part of the [format index](../README.md). This page describes where the two-wheel physics data of RIDE 6 (Milestone,
Unreal Engine 5.6.1, build `5.6.1-1697004`) lives, how the assets inherit from each other, and how a repeatable dump
resolves that inheritance. The per-class field tables are in [ride6-physics-vehicle.md](ride6-physics-vehicle.md),
[ride6-physics-tyre.md](ride6-physics-tyre.md) and [ride6-physics-elements.md](ride6-physics-elements.md). The behaviour
that uses the data is in the specs [vehicle-two-wheel-parameters.md](../specs/vehicle-two-wheel-parameters.md).

Nothing of the game is stored in the repository: the numbers quoted as examples are facts about one bike (the KTM 450 SMR
2022) of the user's own install, and the dump is made on the user's machine ([licensing.md](../licensing.md)).

## Where the data is

All physics data is in the plugin `PhysicsRide6` (content root `ride6/Plugins/PhysicsRide6/Content/Physics/`, mounted as
`/PhysicsRide6/Physics/`). The native module that reads it is `ignitionphysics` (classes `UPhysicsVehicle*`,
`UBikeInputHandler`, `FTireModelData`, ...).

| Folder | Assets | Meaning |
|---|---|---|
| `VehiclesTuning/DataAsset/<Brand>/` | about 320 Blueprint data assets (`KTM_450SMR_2022`, ...) | one per vehicle: an instance of the Blueprint class `DefaultBike_C` (native parent `UIgnitionVehiclePhysicsSetup`) holding a whole `FVehicleData` |
| `VehiclesTuning/DataAsset/DefaultBike` | the class defaults | every bike inherits these; a bike asset stores only what it changes |
| `VehiclesTuning/EngineTorqueCurve/<Brand>/` | one `CurveFloat` per engine | normalised torque against rpm (see the vehicle page) |
| `Tire/TireData/<Family>/` | `Base_TireData_C` assets (`FrontTyre_Road`, `BackTyre_Medium`, ...) | one `UTireData` per tyre: which grip table, the wear decay and the tyre sections |
| `Tire/GripCurve/<Family>/` | `DataTable` of `TireModelData` rows (`Slick_Front`, ...) | Magic Formula 94 coefficients, one row per surface and wetness |
| `Tire/TireSections/<Family>/` | `TireSectionData` assets | temperature and wear model of one strip of the tread |
| `Tire/TireWearCurves/` | `CurveFloat` | temperature to wear, wear to efficiency, temperature to efficiency |
| `Tire/DataTables/TerrainModifiers` | `DataTable` of `TireWearTerrainModifiers` | per surface scale of the heating terms |
| `VehicleElements/<Kind>/` | Blueprint classes (`Motard_InputHandler_C`, `Motard_RiderHelp_C`, `Base_WheelieControl_C`, `Motard_ABSVehicleElement_C`, `Base_TCS_C`, `Base_TireWear_C`, ...) | the vehicle elements (controllers) a bike runs; the bike asset lists their classes in `VehicleElements` |
| `TuningCurve/` | `CurveFloat` | named curves the elements reference (lean limit, lean time scale, stoppie control, ...) |
| `TuningComponent/` | `DataTable` | the setup menu: which member of which struct a tuning entry changes, ranges and default values |
| `PhysicsElements/` | element presets | extra physics elements (slipstream) |

Outside the plugin, these assets are also relevant for the bike: `ride6/Content/assets/data/datatables/Anim/`
(`RiderAnimationCorrection_DT`, `RiderAnimationTrackCategoryCorrection_DT`, `RidingStyle_DT`: per bike rider pose
corrections), `ride6/Content/assets/graphics/Bindings/Rider/RiderAnimationsDataTable`,
`ride6/Content/assets/data/vehicles/DataAsset/Bikestands/`, `.../vehicles/Components/MotionFxData/` and the camera
containers `ride6/Content/assets/data/cameras/Containers/<Class>/<Vehicle>`.

## Inheritance: what "resolved" means

An Unreal Blueprint data asset is an instance (a "default object") of a Blueprint class, and the class has its own default
object with the values the designer set once. The cooked package of a bike stores only the properties that differ from
that class default, and a class default stores only what differs from its parent class. For the KTM:

```
Class IgnitionVehiclePhysicsSetup        (native; C++ constructor defaults)
  Blueprint class DefaultBike_C          default object Default__DefaultBike_C   (about 95 % of every bike)
    KTM_450SMR_2022                      instance: 8 sections, only the changed fields
```

Native fields that no asset sets keep the value the C++ constructor gives them. Those values are not in the data; the
specs of this repository list the ones that matter (recovered from the binary).

The dump tool (scratch tool, outside the repository; see the end of this page) applies this merge rule:

1. Take the chain of objects from the root: for an instance, its `Template` (the class default object), whose own chain
   goes to the super class default objects of the Blueprint classes.
2. Start from the empty object. For each object of the chain from the root to the instance, merge its stored properties:
   a **struct** property merges field by field, recursively (the cooked format stores a struct with only the fields that
   differ from the parent's); every other property (scalars, arrays, maps, sets, object references) of the child
   **replaces** the parent's value as a whole. An array is never concatenated with the parent's array.
3. A `FRuntimeFloatCurve` (`TorqueCurve`, `SuspensionLeverageRatio`, every `*Tuning.Curve`) carries an embedded
   `EditorCurveData` and an optional `ExternalCurve`. **The external curve wins when it is set**; the embedded copy is
   then often a stale left-over (the KTM's `Engine.TorqueCurve` embeds 30 points up to 14500 rpm, but the external
   `KTM_SMR_450_2022` curve ends at 11000 rpm, the bike's `MaxRPM`).
4. Curves are written as lists of `[x, y]` points plus the interpolation modes (`RCIM_Linear`, `RCIM_Cubic` with the
   arrive and leave tangents, `RCIM_Constant`).

The resolved vehicle file also carries a `Linked` object: every physics asset the vehicle refers to (tyres, grip tables,
sections, wear curves, element classes resolved over their parents, tuning tables, curves), each resolved the same way.

## The dump

`C:\r6\run-physdump.cmd` runs the scratch tool `R6ToolPhys` (a copy of the CUE4Parse based `R6Tool` with a
`physdump` mode; needs the user's install, a `.usmap` file and the .NET SDK) and writes `C:\r6\physdump-all`:

| Output | Content |
|---|---|
| `resolved/plugin/Physics/**.json` | every physics package, each export with `Properties` (resolved), `OwnProperties` (as stored), `Chain` (the objects merged, root first) and, for data tables, `Rows` |
| `resolved/content/assets/**.json` | the non-plugin assets listed above |
| `vehicles/<Brand>/<Vehicle>.json` | a self-contained vehicle: resolved `Properties` and `Linked` (319 vehicles) |
| `index.json` | the packages dumped and any error (none in the last run: 1180 packages) |

Checks done on the output: the KTM 450 SMR 2022 resolves to the `DefaultBike` defaults plus its own changes (mass 113,
weight balance 0.505, 5 gears, torque curve of 16 points up to 11000 rpm), and the `Linked` set of the KTM holds its two
tyres, their grip tables (9 surface rows), their tread sections, the wear curves, the element classes and the setup
tables. Two more bikes spot-checked for the same inheritance (`Yamaha_YZF-R1M_2020`: mass 201, final ratio 4.05, 6 gears, 49
linked assets; `Honda_CRF-450R-SMT-2024`: mass 115, final ratio 8.2, 5 gears, 63 linked assets): the chain is
`DefaultBike` then the bike, and the `Linked` assets resolve.

## Units used by the data

The unit of a field is set by the binary, not by the asset (the assets are unitless floats). The facts established for
the fields that matter are in the field tables of the three pages above; in general:

| Group | Unit |
|---|---|
| Chassis geometry (`CGPosition`, `Skeleton.OverrideWheelBase`, trail, shape offsets and radii, rider positions) | centimetres (Unreal units) |
| Suspension travel and compression (`MaxCompression`, `RestCompressionOffset`, `TopOutSpringLength`) | metres |
| Spring and damper rates | N/m and N s/m, at the wheel for the front, at the shock for the rear (see the vehicle page) |
| Masses | kilograms; inertias kg m^2 |
| Engine | rpm, N m; torque curve normalised to `PeakTorque` |
| Speeds in tuning ranges | km/h unless stated otherwise in the field table |
| Angles | degrees (lean and pitch limits in the controllers), except where a table says normalised |
