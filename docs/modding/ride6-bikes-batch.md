# Importing every RIDE 6 bike (`import_all.py`, `ride6-bike-import ids` and `remanifest`)

Part of the [RIDE 6 bikes mod](ride6-bikes.md). The importer converts one bike per run; this page is the batch around it: which
vehicles there are, how a bike gets its car id and its names, how the whole install is imported in one unattended run, what
is refused and why, and how to convert the physics again without exporting the meshes again. The script itself is a user tool
outside the repository (like the exporter it drives: `bikeparts.py`, [ride6-bikes.md](ride6-bikes.md#the-exporter-script-not-in-the-repository));
the importer's part (`naming.rs`, `summary.rs`, `cli.rs`, `pipeline.rs`) is in `mods/ride6-bikes`; the same import runs inside the game as the [native mod](ride6-bikes.md#the-native-mod), which needs no script. Nothing of the game is stored in the repository.

## The vehicles

The game's `CHASSIS` bindings table (`graphics/Bindings/Bike/Chassis.uasset`) has one row `CH_<vehicle id>_Livery-NN` per livery
of every bike: 738 rows for **319 vehicles**. The default (first) livery is `Livery-01`; every vehicle has one. The
vehicle id is `<Brand>_<model words joined by ->-<year>` (`MV-Agusta_Brutale-1090-RR-2016`); variants are separate vehicles
(`...-RM-...`, `...-EM-...`, `...-Ride-Fest-...`: the same chassis family with their own livery and physics data).

What a bike needs, and where the script finds it (all keyed by the vehicle id; [ride6-bikes.md](ride6-bikes.md#the-exporter-script-not-in-the-repository)
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

```
python -I C:\r6\tools\bikeparts\import_all.py [--only <regex on the vehicle id>] [--limit N] [--force] [--dry-run]
         [--mod <mod folder>] [--work <scratch folder>] [--importer <exe>] [--min-free-gb 30]
```

For each vehicle (the KTM first, as a regression gate) the script: checks the bindings and the physics file, runs `bikeparts.py`
(the exporter: `parts.json`, `glb/`, `tex/`), runs `ride6-bike-import` into the mod folder (`files/CARS/<ID>/`, `cars/<id>.toml`),
**deletes `glb/` and `tex/`** (250 to 500 MB per bike) and keeps the small files (`parts.json`, `built.json`, `status.json`), so the
scratch folder stays under about 1 GB per finished bike set and one bike's export at a time (a few hundred MB) is on disk. It
checks the free space of the work drive before each bike (default: stop below 30 GB). It writes `status.json` per bike, appends
to `import-log.jsonl` and rewrites `import-report.md` after every bike, and a bike with a status is not done again unless
`--force`, so an interrupted run resumes. The KTM's output is compared with the baseline hashes in `_baseline_ktm450.md5` and the
batch stops if they differ.

Time on the reference PC (one run at a time; the exporter dominates): about 25 to 30 seconds of export and 5 to 7 seconds of
import per bike, so the 288 importable bikes take under 3 hours. Disk: about 12 MB of car files per bike (3.5 GB for the 288).
The exporter's JSON cache (`C:\r6\parts\cache`) is shared by the runs and grows by a few hundred MB.

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

The default stock bike: chassis, tyres, rims, discs, exhaust (where the bindings name one) and, where the bindings have them, bar
pad and hand guards, in the first livery, with its own physics (`[bike]`, [ride6-bikes.md](ride6-bikes.md#the-physics-dump-import-bike)),
its own model, bounds and manifest. The rider is the shared one (`files/RIDER/`) when the mod folder has it ([bikes.md](bikes.md#the-rider)).
Not part of an imported bike: engine and effect sounds (the `sound` command needs hand-chosen recordings per bike: the bike uses
the game's built-in engine), other liveries, customization (paints, parts from the shop), lamps and damage parts.

## Report

`import-report.md` (in the work folder) lists **every** vehicle: car id, name, imported or refused with the reason, and remarks
(no stock exhaust row, several tyre models for the size, brake kit override not applied). The refusals of the reference run:

**288 of the 319 vehicles are imported, 31 are refused.**

| Refused | Count | Reason |
|---|---|---|
| chassis data asset not in the install | 25 | the `CHASSIS` row names `customization_data/bikeNNN_001`, which was never shipped (the BMW M 1000 R, R 100 GS, S 1000 R, Husqvarna FS 450, RS 457 Trofeo, Vyrus Alyen and more: the report lists each); 2 of them also have no physics asset |
| physics asset not in the install | 1 | the `PHYSICS` row names an asset that is not in the install (`Suzuki_DR-800-S-Big-1994`); the chassis exists |
| physics data does not convert | 5 | the three Indian Challenger variants (a tyre section of the bagger's rain tyre has an unset reference, `TireData/Bagger/FrontTyre_Rain`) and the two Italjet Dragster 200 CC (the skeleton data has no rake and trail: a scooter-like frame) |

Nothing else is refused: a missing exhaust, bar pad, hand guards or rim mesh in the bindings is a note and the bike goes on
without that part (2 bikes have no stock exhaust row; 33 have a brake kit override the importer does not apply; 257 have several
tyre models for their size and take the most used). The 288 imported manifests read back with the game's content schema except
5 (`RIDE6_MOD_DIR=. cargo test -p ride6-bikes --test imported_manifests -- --ignored`): four bikes whose converted gear ratios
exceed the schema's 10 (Ducati 888 SP5 and its RM, the two Indian FTR X 100R Carbon: a ratio of 12.2 is in the data) and one
whose rear leverage curve runs to 2.06 of the travel where the schema takes shares up to 1 (MV Agusta Superveloce 800). Those are
limits of the bike schema against the data (the physics importer and `bike_check*.rs`), not of the batch. The tyre widths of
trail bikes (90 mm) needed the `[wheels]` check (`content/wheels.rs`) to allow 0.03 m for a bike; it took 0.1 m, as for a car.
Time: the reference run took about 34 seconds per bike in all (export about 29, import about 5): under three hours of work
for the 293 bikes that got as far as the exporter, in several sessions (the script resumes).

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
