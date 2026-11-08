# RIDE 6 bike parts: bindings tables, data assets, liveries

Part of the [format index](../README.md). Where the stock parts of a bike are in the user's own RIDE 6 install (Milestone,
Unreal Engine 5.6.1, build `5.6.1-1697004`), how the exporter (`tools/ride6-exporter`) finds them, and why a part can go
missing. Read with a package reader (CUE4Parse, `.usmap`, no AES key); nothing here is copied from the game and no game data is in
the repository ([licensing.md](../licensing.md)). Evidence tags as in the [README](../README.md#evidence-tags): **[data]** is a
value read from an asset, **[choice]** is a decision of the exporter.

## The bindings tables **[data]**

`ride6/Content/assets/graphics/Bindings/Bike/*.uasset` are `DataTable`s of the row struct `IgnitionDataAssetTableRow`: each row is
one key and one `DataAsset` (an object path). The keys are built from the vehicle id (`<Brand>_<model words joined by ->-<year>`,
for example `Ducati_Hypermotard-698-Mono-2024`).

| Table | Rows | Key | The data asset holds |
|---|---|---|---|
| `Chassis` | 738 | `CH_<id>_Livery-NN` | `Mesh`, `AnimInstance`, the material instances `Livery_0`, `Livery_1`, `Mechanics`, `Cockpit`, `Glass`, `GlassTranslucentDOF`, `GlassDecal` |
| `TYRE_SIZES` | 1476 | `CH_<id>_Livery-NN$FrontTyreSize`, `$BackTyreSize` | no asset: a size code `120-70-17` |
| `FRONT_TYRES`, `BACK_TYRES` | 194, 298 | `FTI_<brand>_<model>_<size>`, `BTI_..` | `Mesh` only; the data asset depends on the size and the tyre class (16 brands share one asset per size) |
| `RIMS` | 786 | `RI_Stock_<id>_Livery-NN` | `FrontRimMesh`, `BackRimMesh`, `Material` |
| `BRAKES_KIT` | 709 | `BRK_<id>_Brakes-Stock-Kit` (also `-Kit-Level-N`) | `FrontBrakeMesh`, `BackBrakeMesh`, `Material` |
| `EXMANIFOLD` | 1156 | `CH_<id>$MUF_Stock_<muffler id>_Standard` (also aftermarket `MUF_<brand>_<model>_<finish>`) | `Mesh`, `Material` |
| `MUFFLER` | 688 | `MUF_Stock_<muffler id>_Standard` | `Mesh`, `Material`, `Tags` |
| `BARPADS`, `HANDGUARDS` | 48, 68 | `PAD_Stock_<id>_Livery-NN`, `HGU_Stock_<id>_Livery-NN` | `Mesh`, `Material` (none for the "no hand guard" rows) |
| `SIDE_MIRRORS` | 186 | `MI_Stock_<id>` | `Mesh`, `Material` (`mirrornone` where the bike has none) |
| `SUMP_GUARD` | 16 | `<id>` (enduro bikes) | `Mesh` |
| `PHYSICS` | 738 | `CH_<id>_Livery-NN` | the physics asset (`/PhysicsRide6/.../VehiclesTuning/DataAsset/<Brand>/<name>`) |
| `LIGHTS`, `BIKESTAND`, `HOMOLOGATION`, `HUD`, `NUMBERPROJECTIONS` | | | lighting preset, display stand, plate, HUD: not part of the converted bike |
| `BRAKES_KIT_OVERRIDE` | 45 | `<id>_Brakes-Stock-Kit` | a second brake kit (front and back mesh, material); what the game replaces the stock kit with it for is **open** |
| `CHAIN`, `CarbonRims`, `PHYSICS_BALANCED`, `SUSPENSIONS` | 0, 17, 0, 2 | | empty or not stock |

A table row can name a data asset the install does not contain (content that was never shipped): 101 of the 3437 distinct data
assets the tables name are missing, among them the chassis of 25 vehicles (those are refused, [batch](../modding/ride6-bikes-batch.md)).

## A data asset package has more than one export **[data]**

The package `.../customization_data/rim161_001` of the Ducati Hypermotard 698 holds **two** exports:

```
[0] RimsDataSource_C  name rim070_001   Properties: Material only        (a parent stub, a copy of another bike's object)
[1] RimsDataSource_C  name rim161_001   Properties: FrontRimMesh, BackRimMesh, Material   (the asset)
```

The object of the asset is the export whose **name is the package's own name**. A reader that takes the first export of the
package finds a rim data asset with no mesh and drops the rim without a message. Of the 3437 data assets, 8 have a stub first
(the rims and the silencer of the Hypermotard 698 and its handguards, a bike of each other family); the exporter takes the
export named like the package, falls back to nothing else, and records a part whose asset has no mesh as missing with the
reason `no mesh`. This is why the installed Hypermotard 698 (`DUCATI_HYPERMOTARD-6-9A50E0`) had no rims, no exhaust and no hand
guards: the first export was read.

## What the chassis mesh holds, and surfaces with no picture **[data]**

The skeletal mesh `chassisNNN` has **four levels of detail that are drawn** (LODModels 0 to 3) and four composite ones (4 to 7, one
material `mat_bike_lods`, no export). No section is disabled in any of the 293 meshes; the export holds every triangle of the
sections but a few degenerate ones (4 of 193,186 on the Z900) and the importer leaves out the glass decal (`mat_glassdecal`).
Level 0 has 135,000 to 195,000 vertices: `mat_mechanics` (frame, engine, swingarm, chain guard, pedals: 105,571 triangles on the 690),
`mat_livery_0` (the paint), `mat_cockpit`, `mat_lights`, `mat_glass`, `mat_gear`, `mat_chain`, `mat_gauge`. **From level 1 on the
paint is another slot, `mat_livery_1`** (`mat_livery_2` on chassis035), so a level's surfaces are not those of level 0 renamed.

The stock look of a surface is the instance the data asset (or the part's row) names for its slot. Where none reaches a slot the
converter draws a grey stand-in; 168 surface-and-level pairs of the 293 bikes were such before the fixes below, 20,000 triangles of
level 0 on 11 bikes after (the completeness script lists them); **none are left** after the fixes of the last two rows, which are
described in [the batch notes](../modding/ride6-bikes-batch.md#grey-surfaces-the-fallbacks):

| Surface | Cause | Fix |
|---|---|---|
| the rims of six older bikes (999R, NR750, ZRX 1200 R, ZZR 1400, GSX-R 600 2005, R6 2002) | the master names its picture `BaseColor0_Map` (and `Pbr0_Map`, `Normal0_Map`), which the exporter did not export | the exporter and the importer read them |
| a silencer (Panigale V4 S 2025), a header (RC213V), a cockpit | the opacity switch is on and the picture is in `BaseColor - Texture` | either parameter is tried |
| `mat_livery_1` at levels 1 to 3 of the Brutale 1000 RR, `mat_mechanics_1` and `mat_glass` of the mirrors of three bikes, `mat_bike_lods` of the far rims | the data asset leaves the slot without an instance | the chassis' instance of the same kind, or the part's first picture |
| the rims of the Monster 1200 S (`rim142`, 13,261 triangles) and the header of the R1 RM 2008 (`exmanifold627`, 4,742) | **not missing content**: the rim mesh has no material in its slots (null), so the glb names the surface after the slot (`mat_rim`) while the data asset holds the instance; the header's slot says `exmanifold627_627_001` and points at the package `exmanifold627`, and the glb names the surface by the quoted name | the data asset's `Material` goes to the part's one picture slot when the slot is empty, and a surface is keyed by the glb's name (`bikeparts.slot_materials`, `glb_name`) |
| `WorldGridMaterial` (the tinted glass of the BMW M 1000 RR's homologation kit, six triangles of the Tuareg Rally's chassis), `Default_Material`, the mirrors and homologation kit of three race bikes (no material at all), a lamp and a gauge material with no picture, the glass edge of the R1M | an engine placeholder, or a material the install does not have a picture for | the surface of a similar model: `fallbacks.py`, `parts.json` `fallbacks[]` |

## What the exporter takes **[choice, from the rows above]**

| Part | Rule |
|---|---|
| tyres | the size code of `TYRE_SIZES`; the bike's physics tyre family (`FRONT_TYRE_PHYSICS_COMPOUNDS` row `FTP_<id>$nocompound`, folder `TireData/<Family>/`) picks the stock look: `Supermotard` takes the `No-Brand_SM` row, `MaxiEnduro` and `Enduro` the `No-Brand_Enduro` row, every other family `No-Brand_Street`; a size without such a row takes the first row of the size by name (a note is kept) |
| rims, brakes, silencer, header pipes | the stock rows above; the header pipes' row names the silencer's id (`MUF_Stock_Aprilia-RS-250-2001_Standard` is not the vehicle id) |
| a bike with no row of its own (the event and race variants: `Ride-Fest`, `RM`, `EM`) | the row of another vehicle on the **same chassis** (the number in `chassisNNN` of the `Chassis` data asset), nearest name first, then of the bike it is a variant of (the id without `-Ride-Fest-`, `-RM-`, `-EM-`: every one of the 293 bikes has its own chassis number, so only this applies: the Aprilia RS 457 Ride-Fest 2024 and the Triumph Daytona 675 R Ride-Fest 2017 have no `EXMANIFOLD` or `MUFFLER` row of their own); a note names it |
| bar pad, hand guards, mirrors, sump guard | only when the bike has the row; a data asset without mesh is a bike without the part |
| materials | the mesh's slots (`SkeletalMaterials`) with the data asset's `Material` replacing the single stock slot; the slot of the far levels of detail (`mat_bike_lods`, `BASE_livery_lod`, a picture the game composites at run time) is dropped; the chassis' slots `mat_livery_0`, `mat_livery_1`, `mat_mechanics`, `mat_glass`, `mat_cockpit` are replaced by the data asset's `Livery_0`, `Livery_1`, `Mechanics`, `Glass`, `Cockpit` |

## Where a part sits **[choice]**

Every part is a skeletal mesh whose skeleton shares bones with the chassis skeleton. In the game a vertex skinned to bone `b` is
moved by `chassis_world(b) * part_world(b)^-1`. A part whose bones all move alike (a silencer on `BONE_CHASSIS`, a wheel part on
its hub bone) is one rigid transform: the anchor of `parts.json`. A **body** part whose bones move differently (the hand guards
and the mirrors: one mesh with a left and a right bone, and the chassis' left and right bones are turned) is rewritten by the
exporter in the chassis frame with the linear blend of its four influences, in every level of detail file, and its anchor is the
identity. Before this, the anchor of the dominant bone placed one side right and the other turned.

## Liveries **[data]**

`CH_<id>_Livery-NN` rows: 738 for 319 vehicles (1 livery: 118 vehicles, 2: 53, 3: 140, 4: 2, 5: 6 in the install). The livery's
data asset (`bikeNNN_00N`) replaces the chassis slots `Livery_0`, `Livery_1`, `Mechanics`, `Cockpit` and `Glass` with material
instances of its own (`mat_livery_0_161_002` reads `livery161_002_d`, `_l`, `_n`); the rims, bar pad and hand guards have rows with
the same suffix (`RI_Stock_<id>_Livery-02`). The meshes are the same. `parts.json` lists, for each livery, the instance that
replaces each material name of the stock export (`liveries[].materials`, and `liveries[].parts` for the other parts) and the
exporter writes the textures of all of them.

## Customisation data and upgrades **[data]**

What the install holds beyond the stock parts, found by listing the packages (`R6ToolParts list`) and reading the tables; the
catalogue the exporter writes from it is `C:\r6work\bike-parts\catalogue.json` (`upgrades.py`, `option_export.py`), the shop's use
is [modding/ride6-bike-upgrades.md](../modding/ride6-bike-upgrades.md).

| Data | Where | Holds |
|---|---|---|
| Aftermarket exhausts | `EXMANIFOLD` rows `CH_<id>$MUF_<brand>_<model>_<finish>` and `MUFFLER` rows of that key | 838 header rows over 146 bikes (the bike names the silencers it takes), 371 silencer assets; a header asset `exmanifold<chassis>_<muffler>_001` is per chassis and silencer |
| Aftermarket rims | `RIMS` rows that are not `RI_Stock_` | 43 rows (OZ, BST, Marvic, Rotobox, ...), no bike named: fit is judged by the mesh |
| Brake kits | `BRAKES_KIT` rows `BRK_<id>_Brakes-Kit-Level-1`, `-2` | 187 bikes; the discs `brake700`, `brake701` and four more |
| Bar pads, hand guards | `BARPADS` (48 rows: 15 stock, 33 aftermarket), `HANDGUARDS` (68: 46 stock, 21 aftermarket) | pads of five meshes in many colours; the hand guard mesh has a left and a right bone |
| Tyres | `FRONT_TYRES`, `BACK_TYRES` rows `FTI_<brand>_<model>_<size>` | 25 brands, chosen by the size code |
| Engine kits, suspension, transmission | `DTVehicleParts` (a table of UI pictures, 2757 rows): `ENK_`, `EAF_` air filter, `ECS_` cams, `ECA_` carburettor, `ECL_` clutch, `ECN_` conrod, `ECU_` control unit, `EPS_` piston, `SUS_`, `TRK_`, `CN_` chain | names and pictures only: what a kit changes is **not in the data read** |
| Upgrade numbers | `TuningComponentData_Upgrade*` tables of the physics asset (`TuningDataTables`) | rows `UPGRADE_MaxTorque` (`Engine.PeakTorque`), `UPGRADE_Mass` (`Chassis.Mass`), `UPGRADE_BrakeForce` (`Brake.BrakeForce`), shift time (quick shifter) and launch-control switches |
| Race team upgrades | `BikePhysicDataTable` | 45 MotoGP, Moto2 and Moto3 teams with fractions per level (not road bikes) |
| Prices | none found in the bindings, the tables or the upgrade tables | |

An upgrade row has a `ModifierType` (`RangedMultiplier`, `AdaptiveMultiplier`), `Min/MaxMultiplier` or `Min/MaxMultiplierRange`
(two values) and a `ValueInterval`; the value of an entry is 0 to 1 (the team table lists 0, 0.1, 0.2, ... per level, and
`TuningComponentEntries` defaults every `UPGRADE_*` entry to 1). The multiplier at full level is `lerp(MinMultiplier,
MaxMultiplier, 1)` for a ranged row, and for an adaptive row the `MaxMultiplierRange` read at the position of the bike's own
base value in `ValueInterval` **[inference]**. 316 of 319 bikes list the tables, 187 have multipliers other than 1 (the
bikes that have brake kit rows).

A data asset of an aftermarket silencer, rim or pad has `Mesh` (or `FrontRimMesh`, `BackRimMesh`, `FrontBrakeMesh`,
`BackBrakeMesh`) and `Material` like a stock one, and the skeleton of the mesh shares bones with the chassis, so the anchor rule
of [Where a part sits](#where-a-part-sits-choice) applies. Of the 1008 distinct assets the rows name, 31 are missing from the
install or have no mesh (headers of a few Kawasaki and Ducati chassis). The aftermarket header pipes carry a 2048 px picture
each (985 for 550 headers), which is why the importer writes them at 128 px.

## Not read

Colour customization (the palette, `ColorCustomization - Mask`: the stock look is the picture), dirt, rain, clear-coat layers,
`BRAKES_KIT_OVERRIDE`, tyres of other brands, hand guards of other makers, the engine sub-kits, number projections, the bike stand.
The aftermarket exhausts, rims, brake kits and bar pads are read ([above](#customisation-data-and-upgrades)).
