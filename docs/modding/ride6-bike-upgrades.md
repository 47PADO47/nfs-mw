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
| `road_equipment` | the road equipment on or off: the side mirrors and the homologation plate holder, one optional component shown by default (group `road`, [ride6-road-equipment.md](ride6-road-equipment.md)) | 159 | | none: the developer tools only |
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

### A level that comes with its parts

A bike's **brake ladder** level N and the discs of its **brake kit** of level N are one upgrade in the data (both come from the
`Brakes-Kit-Level-N` row), so the importer writes the link on the ladder's offer: `accepts = [..., { component =
"r6:brakes.l2.10750", bundle = ["r6:brake_discs.brake701_001"] }]`. Choosing `Brakes level 2` (the shop's performance page, the
panel, `vehicle set perf_brakes 2`, the Maximum shortcut) fits the discs of kit 2 with it; going down to level 1 swaps them for kit
1's, to the original equipment takes them off; choosing the discs alone leaves the level. A bike that has the ladder and no kit of that
level (or no disc slot) brings nothing. The mechanism is the vehicle's offers' `bundle`
([data model](../specs/vehicle-components/data-model.md#offer), [the shop](../specs/garage-shop-components.md#3-rules-and-the-refusal-verified)),
not brakes: the engine and chassis ladders have no part in the data and bundle none.

### In the shop

The shop's screens read the same slots ([specs/garage-shop-components.md](../specs/garage-shop-components.md)): a bike's Parts section lists
**its** slots by name (`Exhaust`, `Rims`, `Brake discs`, `Bar pad`, any slot a mod adds), its Performance section the ladders it has
(`Engine`, `Chassis`, `Brakes`), an empty slot is not listed, a component's `price` and `when` rules are the data's (a choice a rule refuses is
dimmed and says why), and a bike's rims are one list of its own rims. A car keeps the 2005 categories. A pick in a slot **no garage category
holds** is saved with the car (`my-cars.toml`, the career's garage extension) and every fitted component's `param` effects reach the
bike's physics, not only the three ladders'.

The developer tools read the same slots: the F11 vehicle panel and the `vehicle` console command list a bike's slots by their own names and
groups (`looks`, `performance`), with the components' names, prices and rules, and change them live in the world and in the car viewer; a
slot the garage has no category for is kept for the session ([devtools-vehicle.md](../devtools-vehicle.md)).

## What the importer writes

```
files/CARS/<ID>/GEOMETRY.BIN     + <ID>_KIT00_{EXHAUST,BARPAD,FRONT_RIM,REAR_RIM,FRONT_DISC,REAR_DISC,MIRRORS,HOMOLOGATION}_<L>: the stock parts as solids
files/PARTS/R6/<STEM>/{GEOMETRY.BIN,TEXTURES.BIN}    one folder per aftermarket part, shared by every bike that fits it
components/ride6-bikes.toml      the catalog: every component once
cars/<id>.toml                   + [[slot]] tables and [parts."<id>"] bindings
```

**Where an aftermarket part sits** (the playtest's "custom exhausts get placed under the bike instead of where they should", and "custom
rims cause a transparent ring between rim and tire"; the rules and the measured numbers are in
[formats/ride6-bike-parts.md](../formats/ride6-bike-parts.md#where-an-option-part-sits-choice-checked-numerically)): a silencer is
anchored on the bones of **its header pipes** (it is skinned to bones the chassis does not have), so it sits at the end of the pipes
where the stock one sits, scaled to them; a twin silencer is one pack solid for each can (`<STEM>` and `<STEM>_G2`, part ids
`exhaust.muffler.<slug>` and `exhaust.muffler2.<slug>`, both shown by the exhaust's component); the front disc takes the side of the
bike's stock disc; the rims are scaled to the tyre's bead (an anchor `scale`, so the matrix of the part binding carries it).
The check `tools/ride6-exporter/option_check.py` flags any option whose box centre is far from the stock part's, a silencer that is
not at the end of its header, and a rim that leaves a ring of 1.2 cm or more.

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
There is **no gameplay-menu row** yet: the switch acts at import time (restart to change it). A row (`bb.settings`, [api-settings.md](api-settings.md)) would need the host to read the mod's value when it lists components; not built.
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

## Bare tyres: a bike whose catalog is missing, and the choice that did not draw **[verified offscreen]**

Two faults showed as "the rims are missing" in the offscreen renders of the first import (a tyre and a brake disc on each
wheel, no spokes, hub or alloy):

1. **The stock rims are solids of their own** (`<ID>_KIT00_FRONT_RIM`, `_REAR_RIM`, role `wheel_front` / `wheel_rear`, no
   `matrix`: they are modelled in the hub's frame and spin with the wheel). The wheel solids no longer hold them, so a bike
   shows its rims only when the host resolves the `rims` slot's default component `r6:rims.stock`, whose `part` effect shows
   `rim.front.stock` and `rim.back.stock`. That component lives in the shared catalog file `components/ride6-bikes.toml`. A
   profile that was assembled by copying `cars/` and `files/` and not `components/` has bikes whose slots name components no
   catalog defines; the host refused them (`mod car <ID>: its components are not offered`) and published nothing, so **no stock
   part was drawn**: no rims, discs, exhaust or bar pad. The files of a run belong together: `files/`, `cars/` and `components/` (`job.rs` promotes the
   three as one).
   *Fix:* a car whose slots do not fit its catalog is now published with its **own stock parts only**
   (`Components::stock_only`: no slots, no choices, every part bound to the car's own files drawn as base parts). The warning
   stays and now names the folder to look at; the bike is complete and offers no choices.
2. **A choice of the rims or the exhaust did not draw at all.** `custom::apply` looked the picked hash of the Rims, Spoiler
   and Roof categories up in the car's own model lists (`custom.rims` and so on); a component's hash is in none, so the whole
   apply failed with `the Rims is unknown` (`view-car --part rims=1` exited with that error, and the garage's preview of a
   bike rim would have shown nothing). *Fix:* a hash that is a choice of the car's components (`components::is_choice_of`) is
   skipped by the model lists; `components::parts` draws it.

Verified on the bikes of the user's install: the stock and the first aftermarket rim of the Honda CRF 450R SMT, KTM RC 390,
KTM 690 SMC R, Ducati Hypermotard 698 and Panigale V4 (front and rear, spinning with the wheel); the commands are below. The
rims come from the `RIMS` bindings of RIDE 6 as before (277 bikes, 1556 offers); the garage lists them in the Rims category with
the preview of `custom::apply`.

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
