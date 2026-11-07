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

## What the exporter takes **[choice, from the rows above]**

| Part | Rule |
|---|---|
| tyres | the size code of `TYRE_SIZES`; the bike's physics tyre family (`FRONT_TYRE_PHYSICS_COMPOUNDS` row `FTP_<id>$nocompound`, folder `TireData/<Family>/`) picks the stock look: `Supermotard` takes the `No-Brand_SM` row, `MaxiEnduro` and `Enduro` the `No-Brand_Enduro` row, every other family `No-Brand_Street`; a size without such a row takes the first row of the size by name (a note is kept) |
| rims, brakes, silencer, header pipes | the stock rows above; the header pipes' row names the silencer's id (`MUF_Stock_Aprilia-RS-250-2001_Standard` is not the vehicle id) |
| a bike with no row of its own (the event and race variants: `Ride-Fest`, `RM`, `EM`) | the row of another vehicle on the **same chassis** (the number in `chassisNNN` of the `Chassis` data asset), nearest name first; a note names it |
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

## Not read

Colour customization (the palette, `ColorCustomization - Mask`: the stock look is the picture), dirt, rain, clear-coat layers,
`BRAKES_KIT_OVERRIDE`, aftermarket parts (`MUF_<brand>_..`, `Brakes-Kit-Level-N`), number projections, the bike stand.
