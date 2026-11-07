# RIDE 6 bikes: optional parts and upgrades (the components model)

Part of the [modding catalogue](README.md). The bikes of the [RIDE 6 mod](ride6-bikes.md) were imported with their stock parts
only. This page says how a bike's aftermarket exhausts, rims, brake discs and bar pads, and its engine, chassis and brake
upgrades, reach the garage and the shop, where the data is ([formats/ride6-bike-parts.md](../formats/ride6-bike-parts.md#customisation-data-and-upgrades)),
what the importer writes, what the game does with it, and what it costs on disk.

## Interpretation: "the new components model"

The request names a "components model" the repository does not define in the request itself. The repository has one: **[ADR 0014,
vehicle components](../decisions/0014-vehicle-components.md)** and the library `libs/blackbox-components` (a base vehicle with
**slots**, a shared **catalog** of **components** that carry `part`, `param` and `behaviour` effects, a **selection** of picks,
and a pure `resolve`), specified in [specs/vehicle-components/](../specs/vehicle-components/README.md). It was built on the branch
`claude/vehicle-components-design-f263ec` and integrated nowhere; this work brings its library and specs in (the six commits
are cherry-picked unchanged) and is its **first integration**, the RIDE 6 half of stage 5 of its [plan](../plans/vehicle-components.md)
plus the pieces of stages 1 to 3 a bike needs. What the garage and the shop already use for car parts (the nine model categories
of `Selection`, the seven performance levels of `PerformancePackage`) is kept as **the places the choices are stored and listed**,
so nothing new is saved and every screen that edits them edits the components. The garage's own screens are not rewritten
(stage 4 of the plan).

| Layer | Is | Where |
|---|---|---|
| Model | `Catalog`, `VehicleDef` (slots), `Selection`, `resolve` | `libs/blackbox-components` |
| Loader | `[[slot]]` and `[parts]` of a car manifest, `components/*.toml` of a mod | `libs/blackbox-modding/src/content/components.rs`, `crates/nfsmw/src/modding/content/components.rs` |
| Binder | slots to the garage's categories and levels, parts to solids, params to `[bike]` | `crates/nfsmw-data/src/car/mods/components/` |
| Importer | the catalogue to part packs, catalog file, per-bike slots | `mods/ride6-bikes/src/{options*,build_pieces,shop}.rs` |

## What a bike offers

The slots of a bike are the rows its data has; a bike lists only those (a bike with no aftermarket exhaust has no `exhaust` slot).
The numbers are from the user's install (319 bikes):

| Slot | Offers | Bikes | Offers in all | Garage place |
|---|---|---|---|---|
| `exhaust` | the stock exhaust and the aftermarket ones of the bike's `EXMANIFOLD` rows (header pipes and silencer together) | 133 | 765 | Spoiler category |
| `rims` | stock and the aftermarket rims that fit (radius within 3 percent of the stock rim, width 0.75 to 1.35 times, both wheels) | 277 | 1556 | Rims category |
| `brake_discs` | stock, and the discs of the bike's `Brakes-Kit-Level-1` and `-2` rows | 166 | 332 | Body kit category |
| `bar_pad` | stock and the 33 aftermarket pads, for the bikes that have a stock pad | 5 | 165 | Hood category |
| `perf_engine` | three levels: peak torque times the bike's upgrade multiplier in equal steps | 187 | | Engine performance level |
| `perf_chassis` | two levels: mass times its multiplier (the lightweight chassis) | 187 | | Suspension performance level |
| `perf_brakes` | two levels: front and rear brake force times its multiplier | 187 | | Brakes performance level |

The garage labels of the category (Spoiler, Hood, Body kit) are the game's own screen texts and cannot be renamed by a mod; the
choices carry their own names (`Akrapovic Completo Standard`). **Not offered**, with the reason: tyres (the wheel solid keeps its
tyre; a tyre choice needs a sixth model category, the data and the exporter list them: 25 brands by size), hand guards (the hand
guard mesh has a left and a right bone, so it is not one rigid transform on a bike; the stock ones stay in the bars), the chain
(the `CHAIN` table has no rows), the engine sub-kits (air filter, cams, carburettor, clutch, conrod, control unit, piston: the
shop's icons exist and no data says what they change), transmission and suspension kits (no number), prices (not in the data;
every choice costs the first tier like any mod car's), exhaust sound and flames (the sound sets of the aftermarket exhausts
exist in the audio bindings as `<bike>_Custom` and are not read; the flames stay at the stock pipes).

### The upgrade ladders

A bike's physics asset lists `TuningComponentData_Upgrade*` tables (by displacement class) whose rows name a member of the
physics data and how an upgrade value from 0 to 1 changes it: `Engine.PeakTorque`, `Chassis.Mass` (adaptive multipliers: the
gain depends on where the bike's own value lies in the table's interval) and `Brake.BrakeForce` (a ranged multiplier). 187 bikes
have rows that change a number; 129 have the table with multipliers of 1 and 3 have none. The exporter reads the multiplier at
value 1 for each bike (`upgrade_tuning.py`); the importer turns it into **tiers in equal steps** (engine 3 levels, chassis and
brakes 2: the icons the game ships for the kits) **[choice]**: the data has no map from a kit to a value for a road bike (the 45
race teams' table, `BikePhysicDataTable`, gives per-level fractions such as 0, 0.1, 0.2, 0.5, 0.7 for the engine, which confirms
the 0 to 1 reading). The `AdaptiveMultiplier` formula is an **inference** from the two ranges and the interval the rows carry.
A component is defined once by its multiplier (`r6:engine.l2.10666`), so bikes with the same number share it.

## What the importer writes

```
files/CARS/<ID>/GEOMETRY.BIN     + <ID>_KIT00_{EXHAUST,BARPAD,FRONT_RIM,REAR_RIM,FRONT_DISC,REAR_DISC}_<L>: the stock parts as solids
files/PARTS/R6/<STEM>/{GEOMETRY.BIN,TEXTURES.BIN}    one folder per aftermarket part, shared by every bike that fits it
components/ride6-bikes.toml      the catalog: every component once
cars/<id>.toml                   + [[slot]] tables and [parts."<id>"] bindings
```

A stock part becomes a solid of its own only when the bike has a slot for it (otherwise it stays in the body, bars or wheel
solid, where it was); the tyre always stays in the wheel solid. An aftermarket part is modelled in its **own frame** (the export
axes turned to the car's) and sits on each bike by the `matrix` of its binding: the three rows of a 3 by 4 matrix computed from
the part's anchor on that bike (`chassis_world(bone) * part_world(bone)^-1`) and the bike's frame (body, or a hub), so a silencer
of twenty bikes is stored once. A header pipe is specific to a (chassis, silencer) pair in the data and is stored once per pair.
A part that is not one rigid transform on a bike is left out for it (the front disc pair takes its dominant bone, as the stock
disc does).

```toml
[[slot]]
id = "exhaust"
group = "looks"
accepts = ["r6:exhaust.stock", "r6:exhaust.muf_akrapovic_completo_standard"]
default = "r6:exhaust.stock"

[parts."exhaust.stock"]
solid = "DUCATIPANIGALE_KIT00_EXHAUST"
role = "body"

[parts."exhaust.muffler.muf_akrapovic_completo_standard"]
solid = "MUFFLER974_001"
pack = "PARTS/R6/MUFFLER974_001"
role = "body"
matrix = [1.0, 0.0, 0.0, -0.01, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.04]
```

The settings of the mod (`config.toml` or `[mods.ride6-bikes]` of the game's config): `upgrades = true` (the master switch: `false`
imports the bikes as before, with every stock part in its solid) and `bike_parts = '<folder>'` (default: `bike-parts` inside the
`source` folder or beside it, when it holds a `catalogue.json`). The command line takes `import-all ... --bike-parts <dir>`.
The `geometry` command rebuilds the car files without the split and must not be run on a bike imported with options.

### Size and time

Measured on the user's install for the exporter's output; the part-pack numbers are **computed** from the exported meshes and
the sizes the importer writes (the importer was not run: no build in this work, so the converted size is an estimate to check
with `import-all`).

| Quantity | Value |
|---|---|
| Catalogue (upgrades.py) | 45 s, 1.9 MB |
| Export of the 959 parts (option_export.py, textures at 512 px) | about 20 minutes, 1.4 GB of glTF, 211 MB of PNG, in `C:\r6work\bike-parts` (working files, not the mod) |
| Distinct parts | 550 headers, 330 silencers, 40 rims (80 meshes), 6 brake kits (12 meshes), 33 pads |
| Geometry in the part packs (uncompressed, levels of `part_levels`) | headers 64 MB, silencers 45 MB, rims 30 MB, discs and pads 6 MB: about 145 MB |
| Textures in the part packs (DXT1, mips) | 480 header, 49 silencer, 40 rim, 33 pad and 6 disc base pictures at 128, 256, 512, 256 and 256 px: 16 MB |
| Stored once | a silencer of 20 bikes, a rim of 277: without sharing the 765 exhaust offers alone would be about 410 MB of silencer and header geometry |
| Per bike | up to six more solids at up to four levels (a few hundred KB), a manifest of 5 to 40 KB; the shared catalog file about 0.5 MB |

The bound is the headers (305 MB at four levels in the exports, hence two levels each) and the textures (985 header pictures of
2048 px in the exports, hence 128 px). Both are choices of `options_pack.rs`.

## What the game does

- **Garage and shop.** `custom::catalog` lists a bike's slot choices in the categories above (every accepted component but the
  default, in the car's order); `custom::apply` resolves the picked components and draws the parts: the solids of the car's own file
  and of the packs, with the matrix applied to the vertices (a wheel, fork or bars placement is re-posed every frame from the
  car's corner rig, which would drop a mount carried by the placement), at every level of detail the model draws, and their
  textures. A component whose pack is missing from the install is not offered. A choice is told apart by a hash of the car
  folder and the component id, so a saved car keeps it when the mod reorders the lists; it is priced like every mod car's choice.
- **Upgrades.** The shop's performance levels of a bike are the tiers of its ladders; `PerformanceCatalog` offers a bike only the
  levels it has a ladder for (0 for the others: the 132 bikes without rows offer none, where they offered three levels of nothing
  before). `CarPhysics` for a bike applies the number modifiers of the picked components to its `BikeSpec` (`keys.rs`:
  `bike.engine.peak_torque_nm`, `bike.chassis.mass_kg`, `bike.brakes.front_force_n`, `bike.brakes.rear_force_n`; a stack is
  limited to 0.5 to 2 times the base) with the same bookkeeping as a car's performance package, which is what the save keeps. The
  shop's bars are estimates from the derived stats and do not move.
- **The hook.** Before this work a bike's parts were its six solids and a mod could not swap one. The smallest hook is the
  `[parts]` table with its `pack`, `role` and `matrix` keys and the `[[slot]]` tables of a car manifest, with `components/` for the
  catalog ([api-content.md](api-content.md#components)).

## Tests and what is not verified

Authored synthetic data, no game files: the loader (`components_tests.rs`), the binder (`components/tests.rs`: mapping, picks,
ladder, key table and clamp, published bike upgraded, the shop's levels, the mount transform of a solid), the catalogue reader,
the plan and its text read back by the game's own parser and checked against the generated catalog (`options_manifest_tests.rs`),
the mount matrices, the masks of split pieces and the config switch. **Not run:** no code was compiled or run for this work, no
bike was imported with options, nothing was seen in the garage; the lead builds it. The matrix of the mount (axis turn, frame and
anchor composed) and the stock-part split are the places to check first, with the offscreen commands below.

```
nfsmw view-car <ID> --part spoiler=1 --muted --screenshot exhaust.png      # the first exhaust choice (name order)
nfsmw view-car <ID> --part rims=1 --muted --screenshot rims.png            # rims; the keys: body spoiler hood roof rims
nfsmw view-car <ID> --part body=1 --muted --screenshot discs.png           # brake kit discs
nfsmw view-car <ID> --part hood=1 --muted --screenshot pad.png             # bar pad (the five bikes with a stock pad)
```

The performance ladders have no offscreen command: the garage's performance page needs a world garage or a career, and the
levels reach the physics through the saved package (`car_with_performance`). `nfsmw view-screen` shows the FEng pages only
without a car. The category keys are those of `view-car --part` (`view_parts.rs`); for a bike `spoiler` is the exhaust, `hood` the
bar pad, `rims` the rims and `body` the discs.

The part numbers count from 1 in the order of the `parts` console command ([car-customization.md](car-customization.md)). A bike
with options is found in `cars/` by a `[[slot]]` table.
