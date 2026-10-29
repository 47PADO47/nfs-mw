# RIDE 6 bikes (`mods/ride6-bikes`)

Part of the [modding catalogue](README.md). A native mod (like the [2012 cars mod](../../mods/nfsmw-2012-content/README.md)) that adds bikes of the user's own RIDE 6
install to NFS: Most Wanted as mod cars ([api-content.md](api-content.md#cars)). `cargo install-mod ride6-bikes` builds and
installs it, `source = '<exports folder>'` in its `config.toml` points it at the exports, and it imports every bike in the
background ([the native mod](#the-native-mod)); the importer is also the `ride6-bike-import` command line. The repository holds the converter and the manifests
only: the exported parts, the converted files (`files/`, `cars/`) and the textures come from the user's install, stay on
their machine and are never committed (the `.gitignore` whitelist does not list them). See
[licensing.md](../licensing.md). The materials the converter reads are described in
[ride6-materials.md](../formats/ride6-materials.md); the two-wheel physics in
[vehicle-two-wheel.md](../specs/vehicle-two-wheel.md).

## The native mod

`mods/ride6-bikes` is a library (`cdylib` and `rlib`, `src/native.rs` with `declare_mod!`) and the importer commands. It is modelled on
`nfsmw-2012-content` (`lib.rs`, `config.rs`, `job.rs`, `stamp.rs`); [mods/ride6-bikes/README.md](../../mods/ride6-bikes/README.md)
has the three steps (`cargo install-mod ride6-bikes`, set `source`, start the game, then `nfsmw view-world --drive <CARID>`).

| Part | Does |
|---|---|
| `config.rs` | the settings from `bb.config` (the mod's `config.toml` merged with `[mods.ride6-bikes]` of the game's config): `source` (the exporter's output, one folder per bike with `parts.json`, `glb/`, `tex/`), `physics` (the physics dump; default a `physics`, `physdump` or `physdump-all` folder inside `source`), `tyre_tables`, `sound` (the `sound` command's output; a bike with tables there gets them), `max_texture`; a commented template is written the first time |
| `batch.rs` | finds the bike folders, assigns the car ids to the whole set ([ride6-bikes-batch.md](ride6-bikes-batch.md#car-ids-and-names)), imports each (a few at a time, `pipeline::import_bike`, the code of the command line) and reports each bike imported or left out with the reason; a panic of the converter on one bike is a reason, not the end of the run |
| `job.rs` | runs the batch on a background thread into `.staging/`; a first run is promoted by renaming (`files/`, `cars/`) and announced with `ctx.content().rescan()`; a run that ends while content is already live is promoted at the next start |
| `stamp.rs` | `.import-stamp`: the settings, the converter's version and the names, sizes and times of every `parts.json`, physics file and sound table; a start with the same stamp converts nothing |
| `native.rs` | `Mod::load` (read the settings, log an info line telling where to put `source` when it is unset, start or skip the job) and `Mod::event` (poll the job, log the result) |

The bikes need the exporter's folders to still have their meshes (`glb/`): a folder without them is listed as left out. The
physics file of a bike is found by the name of the physics asset that `parts.json` records (`physics_asset`; the vehicle id when
an older file lacks it). `mod.toml` declares the library (`[library]`) like the 2012 mod's; the importer never replaces a
`mod.toml` that exists.

## Data flow

```
RIDE 6 install ──R6ToolParts + bikeparts.py (the user's tools, outside the repository)──▶ <bike folder>
    <bike folder>/parts.json   the part table: stock parts, where they sit, materials and their textures
    <bike folder>/glb/         every level of detail of every part mesh (glTF)
    <bike folder>/tex/         every texture of every material (PNG, largest mip not above 2048 px)
RIDE 6 install ──R6ToolPhys physdump (the user's tool)──▶ <physics dump>/vehicles/<Brand>/<Vehicle>.json   resolved physics
<bike folder> + <vehicle json> ──ride6-bike-import --physics──▶ <mod>/{mod.toml, cars/<id>.toml, files/CARS/<ID>/{GEOMETRY,TEXTURES,BOUNDS}.BIN}
```

**Running it.** `ride6-bike-import <bike folder> <mod folder> --physics <vehicle json or dump folder> [--tyre-tables <dir>]
[--sound-dir <dir>] [--car KTM450] [--name ..] [--manufacturer ..] [--max-texture 2048] [--ao-floor 0.3] [--normal-maps]
[--flip-green]`, then `nfsmw --mods-dir <folder with ride6-bikes/> view-car KTM450`. The physics options are
[below](#the-physics-dump-import-bike). The converter needs neither the game nor the exporter's tools, only the two
folders; a second bike is a second folder and a second `--car` (left out, the id is derived from the vehicle id by
the rule of [the batch page](ride6-bikes-batch.md#car-ids-and-names)). The output is deterministic: no random
numbers, ordered maps, a deterministic DXT encoder and JDLZ writer.

## The exporter script (not in the repository)

RIDE 6 is Unreal Engine 5.6. Its `.utoc`/`.ucas` containers open without an AES key, and its assets need a `.usmap`
mapping file (the packages use unversioned properties; made by running the game with a dumper, which the user does
offline: the game is protected by EasyAntiCheat). The reader is CUE4Parse (`GAME_UE5_6`) wrapped in a small console tool
whose modes are generic:

| Mode | Does |
|---|---|
| `jsonmany <list> <out>` | one JSON file of the exports of each listed package |
| `tex <list> <out> <max>` | decode each texture (largest mip not above `<max>`) to PNG, with a `.txt` of its format, size, sRGB flag and compression |
| `export "<pkg;pkg>" <out> Gltf2 All` | every level of detail of each skeletal or static mesh as `.glb` |

`bikeparts.py <vehicle id> <out>` drives it. Given a vehicle id (`KTM_450-SMR-2022`) it reads the bindings tables
`graphics/Bindings/Bike/*.uasset` (data tables whose rows point at one customization data asset each) and finds the stock
parts by the row keys below, then walks every material instance up its parents, exports the textures of the base colour,
normal and packed pictures, exports the meshes, and writes `parts.json`.

| Part | Table | Row key | Data asset gives |
|---|---|---|---|
| chassis | `CHASSIS` | `CH_<id>_Livery-01` | mesh, anim blueprint, livery, mechanics, cockpit, glass decal materials |
| tyre sizes | `TYRE_SIZES` | `CH_<id>_Livery-01$FrontTyreSize` / `$BackTyreSize` | size code (`120-60-17`) |
| front, back tyre | `FRONT_TYRES`, `BACK_TYRES` | any row ending in `_<size code>` (every brand of one size points at one data asset; the script checks that) | mesh |
| rims | `RIMS` | `RI_Stock_<id>_Livery-01` | front and back rim mesh, material |
| brakes | `BRAKES_KIT` | `BRK_<id>_Brakes-Stock-Kit` | front and back disc mesh, material |
| header pipes | `EXMANIFOLD` | `CH_<id>$MUF_Stock_<id>_Standard` | mesh, material |
| silencer | `MUFFLER` | `MUF_Stock_<id>_Standard` | mesh, material |
| bar pad, hand guards | `BARPADS`, `HANDGUARDS` | `PAD_Stock_<id>_Livery-01`, `HGU_Stock_<id>_Livery-01` | mesh, material |

Where a part sits is read from the glTF files: every part skeleton shares bones with the chassis skeleton
(`BONE_F_VIS_WHEEL`, `BONE_FL_BRAKE`, `BONE_BARPAD`, ...). The script takes the shared bone that dominates most vertices
(a bone only the part has is replaced by its nearest shared ancestor) and stores `anchor = chassis_world(bone) *
part_world(bone)^-1`. A wheel part is therefore centred on its hub bone, a disc on its brake bone (off the hub plane by
its offset), the exhaust on the chassis itself.

### `parts.json` (schema 1)

```
schema, vehicle, chassis_row, tyre_sizes{front,back}, lights
bones{ <name>: {position[3], rotation[4]} }          chassis skeleton in bind pose, glTF axes, metres
sockets{ <name>: {bone, location_cm, rotation_deg} }  the skeleton's sockets (rider, feet, audio, stand)
parts[]: id, role (body | front_wheel | back_wheel), export (mesh stem), anchor{bone, translation, rotation},
         materials{ <name in the glb>: <material instance path> }, slots, lod_sections, source, skeleton
materials{ <instance>: tex{param: path}, scalar, vector, switch, blend_mode, shading_model, clip, chain, uv }
textures{ <stem>: <format, size, sRGB, compression> }
notes[]   what the script found but did not apply
```

A part's kind comes from the end of its id (`front_tyre`, `back_rim`, `front_brake`; anything else is body). Exported
meshes are `<export>.glb` and `<export>_LOD<n>.glb`.

## The KTM 450 SMR 2022 (`KTM_450-SMR-2022`, chassis 369)

| Part | Mesh | Where | Solid |
|---|---|---|---|
| Frame, tank, seat, engine, swingarm, rear caliper, rear sprocket, chain, front sprocket | `chassis369` (skinned, bind pose): the triangles on the rear-frame bones | the chassis | body |
| Handlebar, levers, grips, upper clamp, headlamp and front fairing, gauges | `chassis369`: the triangles on the handlebar bones ([below](#the-steered-front-end)) | the chassis | bars |
| Fork legs, front fender, front caliper | `chassis369`: the triangles on the suspension bones | the chassis | fork |
| Header pipes, silencer | `exmanifold369_369`, `muffler369` | the chassis | body |
| Bar pad | `barpad369_369` | `BONE_BARPAD` | bars |
| Hand guards | `handguard369_369` | `BONE_L/R_HANDGUARD` (one mesh, both sides) | bars |
| Front tyre 120/60-17, rim | `f_tyre005`, `f_rim369` | hub `BONE_F_VIS_WHEEL` (0.7356, 0.2248) | front wheel |
| Rear tyre 160/60-17, rim | `b_tyre013`, `b_rim369` | hub `BONE_B_VIS_WHEEL` (-0.7389, 0.2323) | rear wheel |
| Front disc | `frontbrake369` | `BONE_FL_BRAKE`, 0.0635 m left of the hub plane | front wheel |
| Rear disc | `backbrake369` | `BONE_B_BRAKE`, 0.0709 m right | rear wheel |

The chain, calipers, levers and sprockets are not separate meshes: the chassis mesh holds them, skinned to the fork,
swingarm and handlebar bones, in the bind pose; the `CHAIN` table has no rows. The earlier import used `f_tyre003` and
`b_tyre002`, picked by radius; the stock tyres by the bindings are `f_tyre005` and `b_tyre013` (the tyre meshes depend on
the size code only). The front tyre's radius is 0.288 m and the rear's 0.312 m; the hubs are 0.2248 and 0.2323 m above the
skeleton's origin, so the lowest tyre point is 0.0797 m below it and the front tyre floats 0.017 m above a flat road in the
bind pose.

Not part of the converted bike: the side mirrors (`MI_Stock_..` is `mirrorNone`), the homologation plate (no row), the
lights preset (`NoLight`), the bike stand (a display prop; `bikestand_369` only places it), the number-plate projectors
(decal projectors), the glass-scratch decal, the rider. `BRAKES_KIT_OVERRIDE` has a row for this bike (`brake711`: a front
disc 711 and a rear disc 709); the stock kit `brake369` (the row named `Brakes-Stock-Kit` in `BRAKES_KIT`) is used, and
what the override is for (a conversion kit, or a replacement the game applies in some mode) is **open**; the script logs it
in `notes`.

## The car's files

The game draws a car as a body and wheels. `GEOMETRY.BIN` holds, at four levels `A` to `D`:

- `<ID>_KIT00_BODY_<L>`: everything on the chassis. Origin: the middle of the wheelbase along x, the lowest tyre point on
  the ground (z = 0), the bike's centre plane on y = 0. Axes: x forward, y left, z up ([car-customization.md](
  car-customization.md#the-files-of-a-car)); the exports are glTF (x forward, y up, z right) and `layout.rs` turns one into
  the other.
- `<ID>_KIT00_FRONT_TIRE_<L>` and `<ID>_KIT00_REAR_TIRE_<L>`: tyre, rim and brake disc, with the hub on the origin, the axle
  along y, centred on the plane y = 0 (the rim extends from -0.069 to +0.075 m of it and the disc sits 0.06 to 0.07 m to
  one side: they are what turns). The front caliper is in the fork solid, not here: it steers and does not turn.
- `<ID>_KIT00_FORK_<L>` and `<ID>_KIT00_BARS_<L>`: the front end that moves, each with the **front hub on the origin** (the
  same point as the front wheel solid's), the axes of the car frame (x forward, y left, z up), not scaled and not mirrored.
  The fork (legs, fender, caliper) turns with the handlebar and slides along the steering axis with the suspension; the
  bars (handlebar, levers, grips, headlamp, gauges) only turn. See below.

The hubs and radii come out of the converter (`Built::front`, `Built::rear`, in the body solid's frame) and are written to
the manifest in `[wheels]` (radii from the meshes, tyre widths from the size codes) and used for the `[bike]` table's
tyres and body box. Everything else of the `[bike]` table is the bike's physics data, converted
([below](#the-physics-dump-import-bike)); the manifest has no `[stats]`.

**Levels of detail.** The chassis levels 0 to 3 are read (level 0: 135,000 vertices, 1: 78,000, 2: 44,000, 3: 26,000). A body
solid takes the finest level from its letter's minimum on whose parts, with their triangles split by light material, fit
one solid (60,000 vertices, 65,535 triangles): `A` and `B` come out as level 2, `C` and `D` as level 3; the small parts are
read one level finer. Levels 4 to 7 of the engine are one composited material and are not used. A wheel solid reads the
export level of its letter's position (A is 0, D is 3).

## The steered front end

The chassis is one skinned mesh in the bind pose. The importer splits it in three with the bike's own skeleton, so the
front end moves like the real bike's (`mods/ride6-bikes/src/front_end.rs`, `fork.rs`): the frame stays in the body solid,
the **bars** turn with the handlebar, the **fork** turns and slides with the suspension.

1. **Bones.** The bone tree is the node tree of the exported chassis `.glb`. A bone belongs to the group of its nearest
   ancestor (itself included) among the roots: `BONE_F_SUSP` (fork; its child `BONE_F_VIS_WHEEL` carries the hub, the rim
   hubs and `BONE_FL_BRAKE`) and `BONE_HANDLE` (bars; its children are `BONE_BARPAD`, `BONE_R/L_HANDGUARD`,
   `BONE_R/L_LEVER`, `BONE_R/L_GRIP`). In the exported file `BONE_F_SUSP` is itself a child of `BONE_HANDLE` (glb nodes 4
   and 8; the Unreal skeleton agrees), which is why the **nearest** root wins: the suspension slides and the rest of the
   handle subtree does not. `BONE_FL_CALIPER`, which the skeleton hangs on its root but which sits in the front brake
   plane, is a follower of the fork. These names are RIDE 6 names and are the default of `Rules::ride6()`; `parts.json` may
   carry a `front_end` object (`steer_roots`, `slide_roots`, `slide_followers`) to name others.
2. **Vertices.** The glb carries `JOINTS_0`/`WEIGHTS_0` for each vertex (read by `glb.rs`, unsigned bytes or shorts, floats
   or normalised). A vertex share of a group is its weight on that group joints over its total weight.
3. **Triangles.** A triangle goes to the group with the largest mean share over its corners; a tie goes to the group that
   moves least (frame, bars, fork). This is a modelling choice. Vertices are copied into the solid that uses them. A part
   whose mesh is skinned is split the same way by the **names** of its joints (so the bar pad, exported with a skeleton of
   only `BONE_BARPAD`, is judged by the chassis hierarchy); a rigid part goes whole to the group of the bone it hangs on
   (`parts.json` anchor).
4. **The solids** `<ID>_KIT00_FORK_<A..D>` and `<ID>_KIT00_BARS_<A..D>` are built at the same level of the chassis as the
   body solid of their letter, so the three meet at every level. Their frame is the front hub: origin at the hub centre,
   axes of the car frame, exactly as the front wheel solid origin. The game poses the fork with the front wheel transform
   minus the spin and the scale (turn about the steering axis, slide along it), and the bars with the same minus the slide
   (corner 1 of the wheel rig, `Corner::posed(..).1`).
5. **The brake disc** stays with the wheel solid (it spins); **the caliper** is in the fork (it steers, does not spin).

Why the hub and not a point on the steering axis: the game turns the front wheel about the raked axis of the bike data
(`BikeMount`: rake and trail) and the fork has to move rigidly with the wheel; with the hub as the origin that holds for
any axis. The KTM 450 steering axis is the local y axis of `BONE_HANDLE`: it passes 0.0433 m from the hub (the fork offset)
and leans back 26.6 degrees (0.464 rad) from the vertical in the skeleton.

**Result on the KTM 450 SMR 2022** (triangles of the chassis mesh, frame / bars / fork, measured on the exports): LOD 3
14,648 / 4,403 / 1,094; LOD 2 26,385 / 9,577 / 2,133; LOD 0 101,029 / 35,085 / 10,153. All of the cockpit material (bars,
levers, grips) and most of the livery front (fairing) are bars; the fork legs, the fender and the caliper are fork.
`ride6-bike-import` prints the counts of the finest level it wrote. **Seams:** a rigid split cannot blend, so a vertex shared
by two groups is copied into both and moves with its own side; the gap that opens at the steering head when the bars turn is
at most the width of the head skin.

## The exhaust flames

The game lights a car's tail pipes at the markers `EXHAUST`, `LEFT_EXHAUST` and `RIGHT_EXHAUST` of its body solid; the marker's
+z axis is the way the flame shoots ([car-customization.md](car-customization.md#tail-pipes)), and a mod car with none shows no
flame. A bike pops and blips already (its pops and shift pops are the sound's, [ride6-bikes-sound.md](ride6-bikes-sound.md)), so
the importer writes the markers, and the game flames at them ([exhaust-flames.md](../exhaust-flames.md#bikes-the-ride-6-mod)).

**Where the data comes from.** RIDE 6 has no outlet socket: the skeleton lists `SOCKET_AUDIO_EX`, a point on the centre plane a
few centimetres behind the silencer (the KTM's is 83 cm behind the middle of the car frame and 66 cm up) that the sound is
placed at, and `parts.json` lists the stock silencer as the part `muffler` (the header pipes, `exmanifold`, end in it and are not
an outlet). The outlet is therefore read from the silencer's mesh (`exhaust.rs`, the finest level of detail, placed on the car
frame like the rest of the body). Each step is a **choice** of the importer, because the data does not say where a silencer
ends; the constants are named in the file:

| Step | What it does | Value |
|---|---|---|
| components | vertices within 1 mm are one; triangles that share a vertex are one component (can, cap, hanger, bolt) | `WELD` 0.001 m |
| fittings | a component under 10 percent of the largest one's area is dropped, so a bracket cannot pull the outlet | `KEEP_SHARE` 0.1 |
| sides | the components are grouped by the area-weighted lateral position: left, right, or centre when within 3 cm of the centre plane; one group is one silencer (marker `LEFT_EXHAUST`, `RIGHT_EXHAUST`, `EXHAUST`) | `CENTRE_BAND` 0.03 m |
| axis | the principal axis of the group's triangles (centres weighted by area, power iteration) | 100 iterations |
| short cans | a group less than 1.5 times longer than wide is a pot or collector, not a can (the Yamaha R7's mesh holds one under the bike): dropped when the mesh has a real can, else its flame goes straight back | `MIN_ELONGATION` 1.5 |
| direction | the axis turned to point backwards; an axis with a backward share under 0.1 (a side exit or a stack) snaps to out or up | `SIDEWAYS` 0.1 |
| outlet | the vertices within 6 percent of the can's length (at least 5 mm) of its rear end are the cap; the outlet is their mean pushed out to the end plane | `CAP_DEPTH` 0.06, `MIN_CAP_DEPTH` 0.005 m |

A bike with two silencers (the Ducati 888 SP5, the Triumph Speed Triple RS) gets a left and a right pipe, one with a single can
on the right (the KTM) one `RIGHT_EXHAUST`, one with an underbelly pipe on the centre plane an `EXHAUST`. At most one pipe for
each side (a stack of two cans on one side is one pipe between them); the game flames at four pipes at the most. A bike with
no `muffler` part (2 of the 288) has no marker and shows no flame. The marker is written to all four body solids (`A` to `D`)
and the pipes are kept in `built.json` (`exhaust`: side, position and direction in the body solid's frame; a file written
before this has none). `remanifest` does not touch the car's files: **a bike imported before this needs the import again** for
its markers.

## The physics: dump, import, `[bike]`

A bike's behaviour is data in the plugin `PhysicsRide6` ([ride6-physics-data.md](../formats/ride6-physics-data.md)): about 320
vehicle assets that inherit from `DefaultBike`, tyre assets and grip tables, torque curves and the controllers (input handler,
rider help, wheelie control, ABS). The importer reads it from a **resolved** JSON, one file per vehicle, so it needs neither
Unreal's packages nor its inheritance rules. Export, resolve and import, end to end:

```
RIDE 6 install ──R6ToolPhys physdump (the user's tool, outside the repository)──▶ physdump-all/
    vehicles/<Brand>/<Vehicle>.json         Properties (resolved over DefaultBike) + Linked (every asset the vehicle refers to)
    resolved/plugin/Physics/Tire/...        every tyre asset and grip table, one file each
physdump-all/vehicles/KTM/KTM_450SMR_2022.json ──ride6-bike-import --physics──▶ cars/ktm450.toml  ([bike], [wheels])
```

1. **Dump** (once per install): `dotnet build -c Release C:\r6\tools\CUE4Parse\R6ToolPhys`, then `C:\r6\run-physdump.cmd`
   (about 10 seconds; it needs the install and the `.usmap`). The README in the output folder says how the merge works:
   struct properties merge field by field, everything else of the child replaces the parent's value, an external curve wins
   over the embedded one.
2. **The vehicle's file** is named after the vehicle id with the punctuation of the asset (`KTM_450-SMR-2022` is
   `vehicles/KTM/KTM_450SMR_2022.json`). `--physics` takes the file, or the dump folder (or its `vehicles` folder): the file
   is then found by comparing the letters and digits of the vehicle id of `parts.json` with those of the file names.
3. **Import**, after the exporter has written the bike folder:

```
ride6-bike-import C:\r6\bike-input C:\r6\bike-mod --car KTM450 ^
    --physics C:\r6\physdump-all ^
    --tyre-tables C:\r6\physdump-all\resolved\plugin\Physics\Tire ^
    [--sound-dir C:\r6\snd-out]
```

`--physics` is required: a bike without a `[bike]` table is not a bike. `--tyre-tables` is the `Tire` folder of the dump. It
is read only for a tyre asset or grip table the vehicle file does not link (30 of the 634 wheels of the 317 bikes name their
tyre asset in a different letter case from the asset's own path, and the dump's link table is case-sensitive; the lookup of
the importer is not). `--sound-dir` is the `--out` of the [sound command](ride6-bikes-sound.md): its
`cars/<id>.sound.toml` (the `[engine_sound]` and `[car_sound]` tables) is appended to the manifest and its
`files/CARS/<ID>/SOUND/` is copied into the mod (nothing is copied when it is the mod folder; point it at a folder of its
own, because a `cars/<id>.sound.toml` inside the mod's `cars/` is read by the game as a manifest with a bad name and logged).
Run the `sound` command first.

### What the manifest holds

`cars/<id>.toml` is `kind = "bike"`, `role = "player"`, the name and manufacturer, `[wheels.front]` and `[wheels.rear]`, the
`[bike]` table with its sub-tables and, when `--sound-dir` is given, the sound tables. There is **no `[stats]`**: the shop
and the lists read the numbers they show from `[bike]` ([bikes.md](bikes.md)).

- **`[wheels]`** holds the hubs of the converted model (`x_m`, `z_m` in the body solid's frame, the tyre `radius_m` and
  `width_m`; no `face_m`). The game puts the body origin midway between the two hubs (the midpoint of the two `x_m`), so the
  model needs no shifting.
- **`[bike]`** has every required key, in SI units. The field-by-field mapping, with the conversion and whether each number is
  a fact, a constructor default or a choice, is [vehicle-two-wheel-parameters.md](../specs/vehicle-two-wheel-parameters.md)
  section 1. The conversions are the exact factors of `physics/units.rs`: centimetres to metres (divide by 100), km/h to
  m/s (divide by 3.6), degrees to radians, Unreal torque units to N m (divide by 10000), the axes of RIDE 6 (x forward, y
  right, z up) to the physics space (x right, y up, z forward). The resolved Blueprint inheritance is the dump's; the
  importer adds none of its own. Numbers are written as `f32` in the shortest text that reads back as the same value.
- **What the meshes give:** the wheels' radius and width and the body box (`half_dimensions_m`). The data's wheelbase
  (`Skeleton.OverrideWheelBase`) drives the dynamics; if it differs from the distance between the hubs of the model by more
  than a centimetre the importer prints a note.
- **Curves.** A linear curve is copied key for key. A cubic curve (the rear leverage ratio, the lean time scale against the
  stick, the stoppie curve) is sampled along its Hermite spline at 8 points per span, which the library interpolates
  linearly. The dump lists the set of interpolation modes of a curve and not the mode of each key, so a curve that mixes
  modes (the engine friction curve: linear, then cubic beyond the rev limit) is read as linear through its keys. A curve of
  more points than the manifest allows (32; some torque curves have up to 175 keys) loses the keys beyond the rev limit
  first, then the keys whose removal moves it least. A note is printed.
- **What is refused** (the field or the reason is named): a field the importer has no default for that the asset leaves out;
  a brake balance other than 1; hand-written gear change points (all 317 bikes generate them); a wheel that is not a torus;
  a vehicle whose geometry is not overridden in the data (`bOverrideSkeletonData`); deprecated input rates.

### Choices and defaults

The importer adds no number of its own. Where an asset leaves a field out it takes the constructor default that a spec of the
repository records ([`physics/defaults.rs`](../../mods/ride6-bikes/src/physics/defaults.rs) names the document of each);
where the manifest has one number and the data has a curve or several values, the reduction is a choice, listed here:

| Item | What the importer does | Why |
|---|---|---|
| `transmission.up_tolerance`, `down_tolerance` | `GearChangePoints[].ChangeUpTolerance`, `ChangeDownTolerance` of the gear rows. The library generates the change points from the torque curve and the ratios as the handler does ([ride6-gearbox-auto.md](../specs/ride6-gearbox-auto.md) section 1) and compares the road speed with them (section 2); the hold-offs (`prevent_*`) and the launch clutch (`feather`, `launch_rpm`) come from the handler's `PreventGear*`, `ClutchFeatheringOnStart` and `Cluctch*RPMPercentage` fields | nothing is reduced any more: the decision is the handler's own |
| `bike.front.thermal`, `bike.rear.thermal` | the tyre's `Decay`, its `TireSections` (strips and their curves, sampled to 16 points) and the `TireWear` element ([ride6-tyre-thermal.md](../specs/ride6-tyre-thermal.md)); left out when the tyre lists no strips | fact |
| `rolling_slip`, `rolling_slip_linearity` | `RollingResistanceSlip`, `...SlipLinearity` of the road row | fact |
| `chassis.rider_k_n_per_m`, `rider_damp` | `Rider.RiderK`, `RiderDamp` (constructor defaults 20000, 5000) | fact / default |
| `bike.tcs` | the TCS element's `TCSSlipThresholdsDry`, `TCSSpeedThresholds` | fact |
| `fall.block_*` | `BlockFallLeanLimitRatio`, `...MinSpeedKmh`, `BlockFallTimeLimit`, `BlockFallReEnableWaitTimeLimit` when `BlockFallActive` | fact |
| `max_lean_rad`, `fall.lean_rad` | `FallLeanLimitAirborne` (70 degrees on every bike) | the start of the ground fall limit and the airborne limit; on the ground the limit follows `atan` of the grip (`fall.lean_by_grip`, `lean_rise`, `lean_fall` from `FallLeanLimitUp/DownFactor`) |
| `fall.min_speed_mps` | `FallSpeedDirAngleMinSpeed` (0 when unset) | `FallMinSpeed` gates only the rider-force tests, which the library does not have |
| `fall.lean_duration_s`, `airborne_*` | `FallLeanLimitDuration`, `FallLeanLimitAirborne`, `FallPitchLimitAirborne`, `FallAngularSpeedLimitAirborne` | fact |
| `fall.highside_lean_rad` | `FallHighsideLeanLimit` (70 or 50 degrees) | fact: a velocity against the heading with the lean under it is a high side ([rider fall](../specs/ride6-rider-fall.md)) |
| `lean.help_stopped_range_mps`, `help_balance_speed` | `LeanHelpFactorStoppedSpeedRange`; `LeanHelpBalanceSpeedTuning` times `LeanHelpBalanceMultiplier` (else `LeanHelpBalance`) | fact ([chassis round 6](../specs/ride6-chassis-round6.md) section 2) |
| `lean.stability_*` | `StabilityHelpFactorSpeedTuning` (omitted when it is 0 everywhere, as for the supermoto), `...LeanInfluence`, `...LeanSensitivity`, `...ThrottleReduction` / `...BrakeReduction` x `...ReductionScale`, `StabilityHelpSpeedRange`, `StabilityHelpMaxLeanAngle`, `StabilityHelpUsesTerrainNormal` | fact ([chassis round 7](../specs/ride6-chassis-round7.md) section 3) |
| `lean.terrain_*`, `fall.terrain_*` | `TerrainInfluenceSpeedTuning`, `...MaxAngle`, `TerrainLeanLimitIncrementScale` / `...ReductionScale`; `FallLeanLimitTerrainInfluence` / `...Reduction`, `FallLeanLimitMaxDelta` | fact (section 4 of the same page) |
| `lean.help_skid_*` | `LeanHelpFactorSkidRange`, `...Linearity`, `...SkidReduction` times `LeanHelpFactorReductionScale`, `...SkidInputInfluence` | fact; the throttle, brake and slip reductions are unset on every bike and not carried |
| `lean.damping_multiplier` | the formula of the parameters spec, evaluated at rest (KTM about 1.012) | the handler recomputes it every tick from the bike's size; two operands are inferred |
| `lean.help_limit` | the output of `LeanHelpLimitSpeedTuning` at the top of its range (1 or 0.99) | one number in the manifest, a speed curve in the data |
| `lean.blend_range_mps` | `SteerAlignmentRange` (2 to 4 m/s) | the lean spec names the field `LowSpeedSteerBlendRange`, which no asset sets |
| `chassis.rider_forward_shift_m` | 0 | `Rider.ForwardOffset` is set by no asset |
| `engine.idle_rpm` | `MinRPM`, else `MinRPMLimit` | 62 of the 317 bikes set only the limit |
| `engine.limiter_cut_time_s` | the mean of the first limiter's duration range (a fixed range) | |
| wheelie `ground_range_m` | the data's `GroundDistanceFactorRange` as metres | the unit of the field is not established; read as metres (as centimetres the help would fade out within 1 cm of lift) |
| wheelie `lean_limit_rad` when the handler's tuning is off | the steering bound, with a zero pitch range | the library pulls the lean limit to this value; the bound changes nothing |
| `transmission.efficiency`, `clutch_max_factor` | 1 and 1 | no efficiency factor in the binary; the C++ default clutch capacity is the peak torque |

The defaults of `FWheelieControlParams` (dead zone 0.1, minimum speed 15 km/h, maximum angle 30 degrees, acceleration 10
rad/s^2, acceleration time 0.005 s, no look-ahead) apply to the fields the asset leaves out: on the KTM the stoppie keeps the
acceleration time 0.005 s and the wheelie sets 0.1 s.

### Checking it

The unit test `bike_table_tests.rs` feeds a trimmed synthetic vehicle (round numbers, not any bike's) through the importer,
builds the manifest, reads it back with the game's content schema (`blackbox_modding::content::parse_car`, a dev-dependency
of the mod only) and checks every conversion against the same arithmetic. The numbers of the KTM 450 SMR 2022 that the
specs quote are the values to expect from `--physics` on its file. Written without a build; nothing here has been compiled
or run on the real file.

## Rendering

The 2005 car shader ignores normal maps, has no per-texel gloss, and the stock car solids never carry tangents. What is
done, in the order of fidelity:

1. **Baked path (default).** The diffuse picture is the base colour with the occlusion channel applied in linear light with a
   floor of 0.3; opacity is cut at the material's clip value (brake disc holes, chain links, sprocket teeth) with the
   texture marked punch-through and two-sided. Each triangle takes the light material its metallic, roughness and base
   colour say, read over the interior of the triangle (`CHROME`, `MAGSILVER`, `ALUMINUM` for bright neutral bare metal only;
   `MAGGUNMETAL` for a darker (occluded) metal; `SHINYPLASTIC` / `DULLPLASTIC` for dark or coloured metal and for every
   dielectric; `BRAKEDISC`, `RUBBER`), so the shading groups of one material split by surface. A black, gold or anodised
   metal therefore keeps the colour of its texture instead of turning silver ([the table](../formats/ride6-materials.md#what-the-importer-keeps-and-loses)
   has the thresholds, **[choice]**). Textures are DXT1 (opaque) or DXT5 (cut-out) with a full mip chain, written by the
   generic DXT encoder of `blackbox-tpk` ([textures.md](../formats/textures.md#building-textures-from-pixels)). Sizes: body
   textures up to 2048 px, tyres and rims 1024, discs 512, smaller pictures as stored. The livery is the mod car's paint:
   the orange of the stock livery is in the picture, not a `CARSKIN` placeholder.
   **Lost:** the normal maps (tank and plastic grain, engine casting detail, tyre sidewall relief), clear-coat layers and
   metallic flakes, tinted metals (all metals read silver), dirt, rain, the palette recolour, the occlusion strength (the
   floor is a choice), and the gloss variation inside one triangle.
   **Glass [verified by render, choice]:** the screen and the lenses (`BLEND_TranslucentGreyTransmittance`, material
   `mat_glass<chassis>`) are no longer baked as opaque pictures. The picture is the transmittance colour with an alpha of
   `(1 - mean linear colour * 0.95) / light material alpha`, as a DXT5 texture of blend type 1, alpha usage modulated and
   alpha-sorted (the stock windows' texture settings), on the light material `WINDSHIELD` (diffuse alpha 0.4) or, for
   triangles more than 40 percent opaque (the red and amber lenses), `BRAKELIGHTGLASS` (alpha 1.0): `glass.rs`, the
   derivation and the stock cars' data in [ride6-materials.md](../formats/ride6-materials.md#translucent-surfaces). Looked at
   offscreen on the Yamaha YZF-R7 2022 and the Ducati 888 SP5: the screens are see-through with the instruments behind them,
   the Ducati's headlamp shows its reflector behind a clear lens, the tail and indicator lenses are red and amber. The
   blended group culls back faces (as on the stock cars), so a screen is seen from the front only. The white blocks seen
   before were these glass surfaces, not the lamp material: `mat_light<chassis>` is an ordinary masked material with a grey
   reflector picture and an opaque alpha. **Lost:** refraction, rain and scratches on the glass, the lit state of the
   lamps, and the reflection a thin translucent surface adds on top of its transmittance.
2. **Tangent-space path (`--normal-maps`).** The writer of `blackbox-solid` now writes the 60-byte tangent layout of effects
   1 and 3 (`GroupSource::tangents`, the second UV a copy of the first; the reader already decoded it for the world), and the
   importer writes surfaces with a normal map as effect 3 with textures diffuse, normal (DXT1) and specular, which
   `blackbox-render` shades with `NormalModel::Bump` for scenery (`shaders/normal.wgsl`: Phong highlight from the
   specular picture, the sun, the sky and ground light). This keeps the normal-map detail but gives up the car shader's cube
   reflection and light-material response for those surfaces, which is why it is opt-in.
   **Not wired:** the car scene (`crates/nfsmw`, `scenes/car/materials.rs`) uploads groups through `MaterialLookup`, which
   has no route to normal materials, so effect-3 groups draw like effect-4 groups (diffuse and light material). The route
   is a few lines mirroring `scenes/world/materials.rs::normal_materials`: in `CarMaterials`, answer
   `MaterialLookup::group_shading` (or `effect_shading(3)`) for a group whose texture slots 0, 1 and 3 are uploaded with
   `renderer.create_normal_material(&NormalMaterialDesc { model: NormalModel::Bump, diffuse, normal, specular, reflection:
   None, uv_offset: [0.0; 2] })` and return `Shading::WorldNormal(handle)`. Whether Unreal's normal-map green channel agrees
   with the shader's binormal sign is **unverified**; `--flip-green` inverts it.

## Not done

- The rear suspension (swingarm, shock) and the chain do not move with the suspension; the front end steers and travels
  with the fork ([above](#the-steered-front-end)).
- A rider on the bike: [ride6-rider-animation.md](../specs/ride6-rider-animation.md) and [bikes.md](bikes.md#the-rider).
- Levels of detail 4 to 7 (run-time composite material), the stock tyre brand's sidewall picture and rim colour (customization
  state, not in the tables read), the clear-coat look.
- The car scene's normal-material route (the engine side).
- Sound, lamp states (the lamps never light; the lens colours are the unlit ones) and damage parts.
- Other bikes: all of the install is imported by [the batch](ride6-bikes-batch.md) (`import_all.py`): the importer has no
  bike-specific table, a bike is a vehicle id for the script. Bikes with a mono-arm swingarm, a different number of brake discs
  (the part table lists one disc mesh per wheel) or a chain guard that is a separate part are untested.

## Status and what remains

Written without building; nothing here has been compiled or run.

- **The steered front end: done** in code and docs: importer (`glb.rs` skin reading, `front_end.rs`, `fork.rs`, `build.rs`),
  parts database and placement (`mods/parts.rs`, `assemble.rs`, `wheels.rs`: fork in the front brake slot at corner 0, bars
  in the rear brake slot at corner 1), tests.
- **The rider: done and wired**: `update_mesh_vertices` (`blackbox-gfx`, `blackbox-render`), `attach.json` v2 and
  `parts.json` (the importer), `drive/rider/` (parameters, pose and IK, parts, loading, drawing) hooked into `Drive`
  (`finish_step`, `upload_effects`, `instances_with_detail`, `release`), `CarSim::rider_sample`. See
  [bikes.md](bikes.md#the-rider) and [ride6-rider-render-integration.md](../specs/ride6-rider-render-integration.md).
- **The physics importer: done** in code and docs: `physics/` (one file per section of `[bike]`), `bike_table.rs`,
  `manifest.rs`, `manifest_sound.rs`, the options `--physics`, `--tyre-tables`, `--sound-dir`, a golden test
  (`bike_table_tests.rs`). [Choices and defaults](#choices-and-defaults) are listed; the first run on the real KTM file may
  turn up a missing field (the error names it).
- **Not checked:** compile, tests, and a render. The first offscreen run decides the unverified items (winding, V
  direction, the suit look without the arms mesh, hands on the grips); the checks are in the render integration spec.
- **Not done:** the standing wheelie and drift poses (ACL clips), bump and leg-out layers, fingers, look back, the start
  and stop states, gloves, level of detail by distance (level 0 is drawn), the rider in ray-traced reflections, a rider
  that falls with the bike.


## The rider in a fall (`ride6-rider-ragdoll`)

`ride6-rider-ragdoll <RiderGP_PhysicsAsset.json> <RiderBP.json> <mod>/files/RIDER` writes `ragdoll.json`: the 11 bodies and 10 joints of
the rider's physics asset and the two fall configurations of his blueprint, in the rider's frame and metres
([format](../formats/ride6-rider.md#ragdolljson-version-1-the-rider-in-a-fall), [spec](../specs/ride6-rider-fall.md)). The two inputs
are the `json` mode of the package reader on `…/riderShared/…/RiderGP_PhysicsAsset` and on `RiderBP`; run it once next to
`ride6-rider-import`. Without the file a rider stays on a bike that falls.
