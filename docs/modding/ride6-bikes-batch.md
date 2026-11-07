# Importing every RIDE 6 bike (`export_all.py`, `import-all`, `geometry`, `ids` and `remanifest`)

Part of the [RIDE 6 bikes mod](ride6-bikes.md). The importer converts one bike per run; this page is the batch around it: which
vehicles there are, how a bike gets its car id and its names, how the whole install is exported and imported in one unattended
run, what is refused and why, and how to convert the physics or the meshes again without the rest. The exporter (`export_all.py`
and its scripts) is in the repository's [`tools/ride6-exporter`](../../tools/ride6-exporter/README.md), the importer's part
(`naming.rs`, `summary.rs`, `cli.rs`, `pipeline.rs`, `regeometry.rs`) is in `mods/ride6-bikes`, and the same import runs inside the
game as the [native mod](ride6-bikes.md#the-native-mod), which needs no command. Nothing of the game is stored in the repository.

## The vehicles

The game's `CHASSIS` bindings table (`graphics/Bindings/Bike/Chassis.uasset`) has one row `CH_<vehicle id>_Livery-NN` per livery
of every bike: 738 rows for **319 vehicles**. The default (first) livery is `Livery-01`; every vehicle has one. The
vehicle id is `<Brand>_<model words joined by ->-<year>` (`MV-Agusta_Brutale-1090-RR-2016`); variants are separate vehicles
(`...-RM-...`, `...-EM-...`, `...-Ride-Fest-...`: the same chassis family with their own livery and physics data).

What a bike needs, and where the script finds it (all keyed by the vehicle id; [ride6-bikes.md](ride6-bikes.md#the-exporter-toolsride6-exporter)
has the stock-parts table):

| Item | Row | If it is missing |
|---|---|---|
| chassis data asset (mesh, skeleton, livery materials) | `CHASSIS` `CH_<id>_Livery-01` | the bike is refused |
| physics | `PHYSICS` `CH_<id>_Livery-01`, naming `PhysicsRide6/.../VehiclesTuning/DataAsset/<Brand>/<name>`; the dump's file is found by comparing letters and digits | refused |
| tyre sizes, rims, brake kit | `TYRE_SIZES`, `RIMS` `RI_Stock_<id>_Livery-01`, `BRAKES_KIT` `BRK_<id>_Brakes-Stock-Kit` | refused |
| tyres | `FRONT_TYRES` / `BACK_TYRES` rows ending in the size code | one size can have several tyre models (the road, touring, enduro tyres have their own meshes) and the bindings do not name the bike's stock one: the data asset most brands use for the size is taken, the first by name on a tie (a note is kept) |
| exhaust | `EXMANIFOLD` rows `CH_<id>$MUF_Stock_<muffler>_Standard`, then `MUFFLER` `MUF_Stock_<muffler>_Standard` (the muffler's key does not always repeat the vehicle id: `MUF_Stock_Aprilia-RS-250-2001_Standard`); a chassis with several rows takes the one named after itself | no header pipes or silencer, a note |
| bar pad, hand guards | `BARPADS`, `HANDGUARDS` | none (most road bikes have none) |

A vehicle whose rows exist but whose assets are not in the install (the game's tables name content that was never shipped) is
refused with the missing asset named. The report lists every refusal.

## Car ids and names

The car id is the folder under `CARS/` and the manifest's file name. The game accepts 1 to 32 letters, digits, `_` and `-`; an
imported bike has **at most 27**, because the id is also the name of its texture pack and the pack header allows 27 bytes
(`blackbox-tpk`, `MAX_PACK_NAME`). The rule (`mods/ride6-bikes/src/naming.rs`, tested; `ride6-bike-import ids` prints it):

1. the vehicle id in upper case, anything but letters, digits, `_` and `-` becoming `_`: `Honda_CBR600RR-2018` is
   `HONDA_CBR600RR-2018`;
2. 27 characters or fewer: that is the id;
3. longer: the first 20 characters (a trailing `-` or `_` trimmed), `-`, and six upper-case hex digits of the 32-bit FNV-1a hash of
   the vehicle id: `Honda_CBR-1000RR-R-Fireblade-SP-RM-Hutchinson-2022` is `HONDA_CBR-1000RR-R-F-FD76D0` (102 of the 319);
4. unique within the set: the ids are given in the order of the sorted vehicle ids and an id already taken is replaced by the
   hashed form of `<vehicle id>#1`, then `#2`, ... (never happens for the 319; tested with a synthetic clash). The result depends on the
   set only, not on the order of the run;
5. a folder chosen before the rule exists keeps it: `KTM_450-SMR-2022` stays `KTM450` (`naming::LEGACY`), so the first
   bike, its saves and its docs do not change.

The ids never change when a bike is added to the install unless it clashes with a bike that sorts before it. The upper-case hash
matters: the folder is upper-cased by the game and the solids are named from the id as written.

The **display name** is the vehicle id with `_` and `-` as spaces (`KTM 450 SMR 2022`, `Honda CBR600RR 2018`) and the
**manufacturer** is the brand with `-` as a space (`MV Agusta`, `Moto Guzzi`): the brand, model and year of the data, not
retyped. The variants keep their tokens (`Yamaha YZF R1M RM 2019`).

## Running it

Three steps, each resumable and each writing outside the repository (`C:\r6work`):

```
python -I tools\ride6-exporter\export_all.py --out C:\r6work\bikes --jobs 3     the meshes, textures and part tables (293 bikes)
python -I tools\ride6-exporter\physdump.py --out C:\r6work\physdump-all         the physics dump
ride6-bike-import import-all C:\r6work\bikes <mod folder> --physics C:\r6work\physdump-all ^
    --tyre-tables C:\r6work\physdump-all\resolved\plugin\Physics\Tire [--sound-dir <sound-bikes output>] [--threads 4]
```

`export_all.py` skips a bike that has a `status.json` (`--force` does it again, `--only <regex>` picks bikes); `import-all` is the
native mod's batch import (every bike folder of the source, the car ids assigned to the whole set, a bike that cannot be imported
listed with its reason and the others going on; `mod.toml` written when the folder has none). To rebuild only the meshes,
textures and liveries of bikes that are imported already, **without** touching their manifests (the sound, camera and physics
tables stay as they are):

```
ride6-bike-import geometry C:\r6work\bikes <mod folder> [--only <text>] [--max-texture 2048] [--threads 4]
```

It writes `GEOMETRY.BIN`, `TEXTURES.BIN`, `BOUNDS.BIN` of each car and the `[customization]` table (the liveries) of its manifest;
a bike whose `cars/<id>.toml` is not there is reported, not built. The exporter's folder keeps its `glb/` and `tex/` (the old
batch script deleted them after the import; they are needed for the next run): about 125 MB per bike with the normal maps and
about 85 MB without (the exporter leaves them out unless `--normals`, `ride6-bike-import` reads them only with `--normal-maps`).

`ride6-bike-import ids <vehicle id>... | --file <list>` prints `vehicle id, car id, name, manufacturer` (tab-separated) for a set,
and the script uses it as its only source of names. `ride6-bike-import` without `--car` derives the id for the one bike by rules 1 to
3 and 5.

### Converting the physics again: `built.json` and `remanifest`

The importer writes `built.json` beside `parts.json`: the hubs, tyre sizes and body box that the manifest takes from the meshes. The
physics conversion (`physics/`, the `[bike]` table) changes as the two-wheel model learns more, and the meshes are large and
deleted after the import. So:

```
ride6-bike-import remanifest <bike folder> <mod folder> --physics <dump> --tyre-tables <dump>/resolved/plugin/Physics/Tire
```

rewrites `cars/<id>.toml` from the bike's `parts.json`, `built.json` and the resolved physics file, in a second, without the
meshes (`--car`, `--name`, `--manufacturer` and `--sound-dir` as for the importer); the car's files are not touched.

## What an imported bike is

The default stock bike: chassis, tyres, rims, discs, exhaust (header pipes and silencer; every bike of the install has them) and,
where the bindings have them, bar pad, hand guards, mirrors, sump guard, number plate and rally kit, with its own physics (`[bike]`,
[ride6-bikes.md](ride6-bikes.md#the-physics-dump-import-bike)), its own model, bounds and manifest. **Liveries:** every livery of
the bike (`CH_<id>_Livery-NN`) with a different picture is listed in the shop's vinyl category like the 2012 cars' liveries
([car-customization.md](car-customization.md#liveries)): 209 of the 293 bikes have two to five, and the stock one (Livery 1) is
worn from the start. The rider is the shared one (`files/RIDER/`) when the mod folder has it ([bikes.md](bikes.md#the-rider)).
Not part of an imported bike: colour customization (the paint list has one neutral entry, the colours come from the liveries),
parts from the shop, lamps and damage parts. Engine sound comes from `sound-bikes`.

## Report

The exporter's `export-report.md` (written by `report.py` into the output folder) lists **every** vehicle and what it lacks; the
importer's `import-all` prints the bikes it left out. The numbers of the reference run:

**319 vehicles: 293 exported, 26 refused by the exporter, 5 refused by the importer, 288 imported.**

**The 26 the exporter refuses** (all for one reason: the `CHASSIS` row names a data asset, `customization_data/bikeNNN_001`, that
the install does not contain; the chassis mesh `chassisNNN` and the livery materials are not in the install either, so there is
nothing to read; the whole container set (45 IoStore containers and 44 encrypted `.pak` files in `Paks`; the containers mount without a key) is searched, and a listing for the asset and
mesh names finds no file): Aprilia RS 457 Trofeo 2024 (bike034), RSV4 Factory 1100 2025 (009), Tuono V4 Factory E5 2021 (030); BMW
M 1000 R 2023 (068), R 100 GS 1000 1987 (070), S 1000 R 2020 (065); Ducati Hypermotard 950 2019 (141), Monster Plus 2021 (155),
Panigale V2 2021 (156), Panigale V4 S RM 2025 (166); Honda CBR 600 RR HRC 2021 (251), XVR 750 Africa Twin 1990 (256); Husqvarna 701
Supermoto 2025 (274), FS 450 2022 (277); Kawasaki Ninja ZX-10RR Performance 2024 (320); MV Agusta Brutale 800 RR 2021 (484), F3
800 RR 2022 (488), LXP Orioli 2024 (490), Rush 1000 2021 (486); Suzuki DR 800 S Big 1994 (536), TL 1000 R RM 1998 (552); Triumph
Street Triple RS 2024 (598), Trident 660 2021 (595); Vyrus Alyen 2020 (645); Yamaha MT-10 SP 2024 (664), XTZ 750 Super Tenere 1989
(666). Three of them also have no physics asset (the RSV4 Factory 1100 2025, the Husqvarna 701 Supermoto 2025 and the Suzuki DR 800 S
Big 1994: `phys_missing` in the exporter's check). The tables list them because the game's data lists them: the content was never
shipped in this build of the install (a later patch or paid pack, or content cut before release); nothing in the install can
replace it, and the exporter does not guess a mesh from another bike. **Nothing to fix here**; if a later install has the
packages, `export_all.py --only <id> --force` exports them.

**The 5 the importer refuses:** the three Indian Challenger variants (a tyre section of the bagger's rain tyre has an unset
reference, `TireData/Bagger/FrontTyre_Rain`) and the two Italjet Dragster 200 CC (the skeleton data has no rake and trail: a
scooter-like frame).

**Parts a bike lacks.** Every one of the 293 exported bikes has front and back tyre, rim and brake disc, header pipes and silencer
(the two event bikes whose own rows were missing, the Aprilia RS 457 Ride-Fest 2024 and the Triumph Daytona 675 R Ride-Fest 2017, take
the rows of the bike they are a variant of). What some lack is data the install does not ship: mirrors and number plate for the
Aprilia RSV4 X-trenta 2022, Ducati Desmosedici RR Stoner 2006 and KTM RC 8C 2023 (`asset not in install`), mirrors for the Italjet
Dragster 700 Twin and the KTM 450 SMR 2022 (the row is `mirrornone`: the bike has none), the bar pad of the KTM 690 SMC R 2026,
the sump guard of the Honda X-ADV 2024, the `worldgridmaterial` of 12 rally bikes (an engine debug material, a grey surface is
used) and the normal map `cockpit529_n` of the Suzuki GSX-R 1000 RM 2007 (the decoder fails on it; the normal maps are not used).
Of the 293, bar pads exist for 4 bikes, hand guards for 19, mirrors for 157, a sump guard for 10 (the others have none in the
game). The Ducati Hypermotard 698 (`DUCATI_HYPERMOTARD-6-9A50E0`) had no rims, no silencer and no hand guards in the first
import: [the first export of its data assets is a stub](../formats/ride6-bike-parts.md#a-data-asset-package-has-more-than-one-export-data),
which the exporter read.

Nothing else is refused: 33 bikes have a brake kit override the importer does not apply; 257 have several tyre models for their
size and take the stock look of their tyre family. The 288 imported manifests read back with the game's content schema except 5
(`RIDE6_MOD_DIR=. cargo test -p ride6-bikes --test imported_manifests -- --ignored`): four bikes whose converted gear ratios
exceed the schema's 10 (Ducati 888 SP5 and its RM, the two Indian FTR X 100R Carbon: a ratio of 12.2 is in the data) and one
whose rear leverage curve runs to 2.06 of the travel where the schema takes shares up to 1 (MV Agusta Superveloce 800). Those are
limits of the bike schema against the data (the physics importer and `bike_check*.rs`), not of the batch.

## The game side

410 mod cars (these bikes and the 93 cars of the 2012 mod) need more parts-database entries than the stored layout holds; the
engine now keeps them with wide indices ([car-parts-capacity.md](../specs/car-parts-capacity.md),
[api-content.md](api-content.md#parts-database-capacity)).

## The rider's contact points (`contacts`)

Every bike the importer writes gets `files/CARS/<ID>/RIDER/contacts.json`: where its seat, grips and pegs are, from the sockets of
the bike's skeleton in `parts.json`, plus its row of `RiderAnimationCorrection_DT` from the physics dump
(`<dump>/resolved/content/assets/data/datatables/Anim/`; a dump without it, or a vehicle without a row, gives a file without
`correction`, and the import logs a note). The shared rider rig stays in `files/RIDER/`
([bikes.md](bikes.md#the-rider), [formats/ride6-rider.md](../formats/ride6-rider.md#contactsjson-version-1-one-bikes-contact-points)).
To add the files to bikes imported before, without converting any mesh:

```
ride6-bike-import contacts <exporter output, or one bike's folder> <mod folder> --physics <dump>
```

The silencer's outlets (the tail-pipe markers of the body solids, [ride6-bikes.md](ride6-bikes.md#the-exhaust-flames)) are read
from the silencer mesh while the geometry is built, so a bike imported before they existed gets them only by an import again;
`contacts` and `remanifest` do not write them.

The car ids of a set are assigned as the batch import assigns them. The import stamp's conversion number went to 2 for this, so a
mod started with the native library converts everything once more; the command above does not.

## The cameras (`cameras`)

Every bike the importer writes gets a `[bike.camera]` table in its manifest: the two chase cameras, the camera ahead of the bike and the
rider's helmet camera, read from RIDE 6's camera container of the bike (`<dump>/resolved/content/assets/data/cameras/Containers/`,
with the class's and the root base container behind it) and moved from the skeleton's frame to the model's with the hubs of
`[wheels]` and `RIDER/contacts.json`, so a bike needs its contact points first ([specs/ride6-bike-cameras.md](../specs/ride6-bike-cameras.md),
[provenance](../provenance/ride6-bike-cameras.md)). A bike whose container is not found (a model year apart is matched; 16 of the 293 are not)
is listed and gets cameras derived from its wheelbase by the game. To add the tables to bikes imported before, without converting any
mesh (the command writes only the tables into `cars/<id>.toml` and replaces them if they are there):

```
ride6-bike-import cameras <exporter output, or one bike's folder> <mod folder> --physics <dump>
```

The import stamp's conversion number was not changed for this: a mod started with the native library does not convert again for it.
