# The NFS: Most Wanted (2012) content mod

`nfsmw-2012-content` adds the cars of **NFS: Most Wanted (2012)** to this game: 73 more cars to choose from (the install has 93 car bundles; 20 are
liveries of other cars, which the shop offers as liveries), with their
own bodies, interiors, glass, lamps, wheels, textures, paints, stats, collision and sounds. The 2012 game runs on a
different engine (Criterion's Chameleon, not Black Box), so the cars are converted, not copied. It is a native mod like
[`hello`](examples.md), the only mod of the repository that is built with the workspace:
[`mods/nfsmw-2012-content`](../../mods/nfsmw-2012-content). The readers behind it are the three `libs/chameleon-*` crates
(formats: [chameleon-bundle](../formats/chameleon-bundle.md), [chameleon-gfx](../formats/chameleon-gfx.md),
[chameleon-vehicle](../formats/chameleon-vehicle.md)).

**Nothing from either game is in the repository or in a release.** The cars are made on your machine, in the background
while the game runs, from your own 2012 install. Without one the mod adds nothing and the game is unaffected.

Each converted car is a car of its own ([api-content.md](api-content.md#cars)): one more entry in the car list, named by its
folder, with its own geometry, textures, collision bounds, stats and sounds. **Nothing is taken from a car of the 2005 game,
and the conversion does not read the 2005 install at all.** The body is not stretched onto a stock wheel layout, and the
wheels sit at plus and minus half the car's wheelbase with the model centred on it.

## Using it

1. Build the mod (`cargo build --release -p nfsmw-2012-content`) and put `mods/nfsmw-2012-content/mod.toml` and the built
   library (`nfsmw_2012_content.dll`, `libnfsmw_2012_content.so` or `libnfsmw_2012_content.dylib` from `target/release/`)
   in one folder under the game's mods folder, for example `%APPDATA%\nfsmw\config\mods\nfsmw-2012-content\` (see
   [Using mods](README.md#using-mods)).
2. Start the game once. The mod writes a commented `config.toml` next to its `mod.toml`.
3. Put the folder of your 2012 install in it and start the game again:

   ```toml
   source = 'D:\Need For Speed Most Wanted 2012'
   ```

That is all. With a `source` set, the mod converts the cars **on a background thread while the game starts and runs**;
nothing waits for it. It takes about a second on a six-core PC once the files are in the disk cache, a few more seconds
when they are not, and writes 360 MB into the mod's own `files/` and `cars/` folders (both belong to the converter: a new
conversion clears them, so do not edit them by hand). When it is done the cars are in the game: everything the game opens
after that, the garage and the career among them, lists them, so by the time you reach the career menu they are there. Later
starts convert nothing unless the settings, the converter or the 2012 files changed (the mod keeps a fingerprint in
`.import-stamp`), and the cars are in from the first frame. A change in `config.toml` takes effect at the next start; when
cars are already in the game, the new conversion is kept aside and replaces them at that start. To remove the cars, delete
the mod folder, or set `mods = off` / use `--no-mods`.

What the shop offers for a car's looks is the car's own: the **paints of its 2012 bundle**, 1 to 15 per car, and, for the five
cars the 2012 game ships in several liveries, **the liveries** (no vinyls, rims, spoilers or tints:
[chameleon-customization.md](../formats/chameleon-customization.md)). The 2012 game lists a livery as a vehicle of its own, with
a bundle of its own; the mod makes one car of a vehicle and lists the pictures of its bundles as `Livery 1` to `Livery 5` in the
shop's vinyl category ([car-customization.md](car-customization.md#liveries)).
The performance upgrades and **nitrous** work on the car's own stats; the cars have nitrous from the factory. How the shop
treats a car is in [car-customization.md](car-customization.md), how it drives in [car-stats.md](car-stats.md), how it sounds
in [car-sound.md](car-sound.md); this page is the mod's settings and what the conversion writes.

### Settings (`config.toml` in the mod's folder)

Every key is optional. An unknown key is an error and no car is added, so a typo is not silently ignored.

| Key | Meaning |
|---|---|
| `source` | The 2012 install folder. Not set: nothing is converted |
| `cars` | `"all"` (default) or a list of ids or name fragments (`["Porsche", "LAMGALLARD11"]`) |
| `exclude` | Ids or name fragments to leave out |
| `max_texture_size` | Longest texture side, a power of two from 16 to 4096 (default 1024); bigger maps lose their top mips |
| `role` | `"player"` (default), `"cop"` or `"both"`: who can use the cars |
| `own_engine_sound` | `true` (default): each car gets the recordings of its own 2012 bundle, written into the mod folder as WAVs (`files/SOUND/MODENGINE/<ID>/`): its engine loops, and its gear shifts, turbo whistle and blow-off, backfires and pops. `false` writes none (the shared recordings included): the cars play the game's built-in engine and have only the game-wide sounds. An approximation, see [car-sound.md](car-sound.md#the-2012-cars-recordings) |
| `[performance]` | Optional multipliers on the stats read from each car's own 2012 data: `power` (torque at every engine speed), `mass`, `grip` (tyre friction), `brakes` (brake torque), `nitrous_power` (the extra torque of the nitrous), all 1.0 by default, and `nitrous` (`true` by default: the car keeps the nitrous its 2012 data gives it; `false` removes it). The mod applies them when it writes the car, so the game only sees the resulting absolute `[stats]` table ([car-stats.md](car-stats.md)); a car whose settings are left alone is exactly the imported 2012 car. Ranges: `power`, `mass`, `brakes` 0.25 to 4, `grip` 0.25 to 3, `nitrous_power` 0 to 4; results are clamped to the ranges of `[stats]` |
| `[car.<id or name fragment>]` | The same keys for one car or a make, changing only the keys it lists: `[car.POR911CARR12]`, `[car.Lamborghini]`. The tables apply in the order of their names over `[performance]`. |

The ids are made from the car's name (`POR911CARR12` for the 2012 Porsche 911 Carrera S, at most 12 characters); the mod's log
lists every car it converts or leaves out and why. The tables go after the plain keys of the file, because TOML puts every key
below a table header inside that table:

```toml
source = 'D:\Need For Speed Most Wanted 2012'
cars = "all"

[performance]
power = 1.05
nitrous = true

[car.POR911CARR12]
power = 1.2
mass = 0.95

[car."Aston Martin"]
grip = 1.05
```

A name with a space is written in quotes, as in the last table. Absolute values for a car of your own go in its `[stats]`
table instead ([car-stats.md](car-stats.md)).

## What is converted

| 2012 | In the game |
|---|---|
| Body, interior, lamps, mirrors (finest of 3 levels, 5,000 to 36,000 triangles) | `<ID>_KIT00_BODY_A..D` (the car's own folder), in car space (+x forward, +y left, +z up), with the 2012 shape unchanged: only the axes are permuted and the car is centred on its wheelbase (`src/fit.rs`) |
| Tyre, brake disc, rim and caliper | one `<ID>_KIT00_FRONT_TIRE_A..D` solid, rim face at y = 0, as the game's wheels are modelled; the game draws it at the four hubs the car's `[stats]` give |
| Diffuse maps (BC1, BC3) | the car's `TEXTURES.BIN` as DXT1 and DXT5, cut to `max_texture_size` |
| Paint surfaces | the placeholder `<ID>_SKIN1` with light material `CARSKIN`, which the game swaps for the paint the player picks. Their texture coordinates are the 2012 car's own |
| Liveries | the paint picture of each bundle of a vehicle that has several (2048 x 1024 BC1, cut to `max_texture_size`): the vehicle's own bundle first, then its liveries in vehicle list order, as `<ID>_LIV1` to `<ID>_LIV5` in the car's `TEXTURES.BIN` and the `liveries` list of its `[customization]` table. A livery bundle is the car with that one texture replaced; one that is not (another body, no picture) is converted as a car of its own (`src/vehicle_list.rs`, `src/liveries.rs`, `src/variants.rs`) |
| Paints | the `VehiclePaint` records of the car's bundle (`libs/chameleon-vehicle/src/paint.rs`): the colour, and the finish from the record's type, written as the `[customization]` table of the manifest, repeated colours dropped (`src/paints.rs`) |
| Stats | the car's own stats, read from its Genesys handling definition ([chameleon-vehicle-stats.md](../formats/chameleon-vehicle-stats.md)) and written as the `[stats]` table of its manifest: mass (body and wheels), weight distribution and centre-of-gravity height from the axles and rigid bodies, wheelbase, track and tyre size, the torque curve (at most 16 points, knots kept) with its idle and limiter, the gearbox and final drive, the drive layout and differential locks, the tyres' peak grip, the brake torque of each axle, the design top speed (the game solves the air drag from it) and the nitrous strength as `nitrous_boost`; then the `[performance]` multipliers (`src/stats.rs`, `src/performance.rs`) |
| Engine | the car's own sound graphs ([chameleon-sound-graph.md](../formats/chameleon-sound-graph.md)): the Ginsu voices as `.GIN` files, the idle and whine loops as WAVs, each layer with its curves of rpm, load and speed in the `[engine_sound]` table (`src/graph_voices.rs`); a car without those graphs gets the looping samples of its bundle (decoded from the `SPS` streams inside them, see [chameleon-sound.md](../formats/chameleon-sound.md)), except the turbo's whistle, one 16-bit WAV per layer at one level, with the rpm each stands for (`src/engine.rs`, `src/sample.rs`) |
| Shifts, turbo, backfire, pops | the samples that the car's sound graphs play for the live values `GetPlayerNormalisedGearChangeTimeElapsed`, `BackfireGain`, `GetPopProbability`/`PopGain`, `NormalisedTurboRPM`/`TurboToneGain` and `TurboBlowoffGain` (the last two also use the shared `VEHICLES/WAVES.BNDL`), as WAVs and a `[car_sound]` table (`src/graphs.rs`, `src/effects.rs`). A car with no such graph has no such sound |
| Shared body hits, scrapes, pops | the first selector of the `Player Vehicle Collision Small`/`Medium`/`Big` graphs and the three samples of `Player Vehicle Scraping` (`SOUND/METALCAR.BNDL`), and the pops each car's graphs import from `VEHICLES/WAVES.BNDL`, written **once** under `files/SOUND/MODSHARED/` and named by every car's `[car_sound.shared]` (`src/shared.rs`). Evidence: [chameleon-shared-sounds.md](../formats/chameleon-shared-sounds.md) |
| Collision mesh and tyres | `files/CARS/<ID>/BOUNDS.BIN`: the car's own collision bounds, fitted to the poly soup of its bundle (the body's real extents, 166 to 256 triangles) and to the box of each tyre, through the same transform as the body: a root box, a lower body box, a cabin box, front and roof corner spheres and the silhouette the car rolls on. The one bundle without a collision mesh (the SWAT van) is fitted to its visible body. Format and what the 2012 game holds: [car-collision.md](car-collision.md), [chameleon-collision.md](../formats/chameleon-collision.md) (`src/collision.rs`) |
| Hinged parts | The parts the car's damage object lists (bonnet, bumpers, mirrors, doors, boot, as the car has them): cut out of the finest body solids into `<ID>_KIT00_<PART>_A` and `_B`, and a `[[damage_parts]]` table per part in the manifest ([below](#hinged-parts)) (`src/hinges.rs`) |
| Markers | `EXHAUST`, `LEFT_EXHAUST` and `RIGHT_EXHAUST` (where the nitrous and pop flames start), one per tail pipe, on `<ID>_KIT00_BODY_A..D`: the locators of the groups the car's nitrous spot effects name, facing the way the locator faces, mirrored when the effect says so; a car with no nitrous effect (the Tesla, the SWAT van) has none and so no flame (`src/pipes.rs`, [chameleon-exhaust.md](../formats/chameleon-exhaust.md)); a car whose bundle has no effect records at all (one) gets pipes derived from its body, at the rear face, low (`src/markers.rs`). The 2012 game has no spoiler or scoop to mount, so no mount marker is made |
| Nitrous sounds | the burn loop, the end sound and the start sound of the 2012 heat level sound set (`GAMELOGIC/HL_*.BNDL`, found by the expression `NitrousState.UsingNitrous` and the trigger `NITROUS_POSTFX_OFF`), as stereo WAV files in `files/NITROUS/MW2012/` with a `profile.toml`, and the value **NFS Most Wanted 2012** of the gameplay setting "Nitrous variant" (`bb.settings`), offered when a 2012 install is set. The post effects, flames, camera and HUD of the 2012 nitrous are not played; see [nitrous-variants.md](nitrous-variants.md) and [chameleon-nitrous.md](../formats/chameleon-nitrous.md) (`src/nitrous.rs`) |
| Markers | `LEFT_EXHAUST` and `RIGHT_EXHAUST` (where the nitrous flames start) on `<ID>_KIT00_BODY_A..D`, derived from the body: at the rear face, low, half way to the corners (`src/markers.rs`). The lamp markers `LEFT_HEADLIGHT`, `RIGHT_HEADLIGHT`, `LEFT_BRAKELIGHT`, `RIGHT_BRAKELIGHT` and `BRAKELIGHT_CENTRE` are the car's own lamp locators, picked by its lamp records ([car-lamps.md](car-lamps.md)). The 2012 game has no spoiler or scoop to mount, so no mount marker is made |
| Lamp surfaces | the lit strips of the 2012 lamps, split by piece, drawn with the game's lamp placeholders, and the car's own `<ID>_KIT00_HEADLIGHT_ON`/`_OFF` and `_BRAKELIGHT_ON`/`_OFF` textures (flat colours); the lamp interiors are dark grey ([car-lamps.md](car-lamps.md)) |
| Glass and lamp lenses | a translucent tint texture with light material `WINDSHIELD` |
| Untextured surfaces | a flat colour texture from the material's diffuse colour (dark grey when it has none) |
| Normals | recomputed from the triangles |

Levels of detail: the game draws nothing for a level a car lacks, so A and B are the finest body, C and D the coarser
ones. The vertex limit of one solid (60,000) can force a coarser level. The manifest also carries `role`, a display name
(`Porsche 911 Carrera S 2012`) and the brand (`src/names.rs`).

## How it works with the host

- **At load** the mod learns its own folder from the host table (`ctx.mod_dir()`, see [api.md](api.md#lifecycle)). It
  promotes a finished conversion left by an earlier start, reads `config.toml`, and, unless the content on disk is still
  right, starts the conversion thread and returns.
- **The conversion** runs on a pool of all cores but one (at most six), one car per task, and builds into `.staging/`. It
  never writes where the game reads. The fingerprint is written last, so a run cut short is thrown away.
- **When it is done** (the mod sees it in its frame events: the worker cannot call the host) the mod moves `.staging/files`
  and `.staging/cars` into place and calls `bb.content.rescan()`. The host adds the mod's `files/` as a *late layer* of the
  install, which every copy of the install shares, including the ones the scenes already hold, and publishes the cars its
  `cars/` folder declares; the car tables list them from then on. See [api-content.md](api-content.md#content-registries-bbcontent).
- If content was already live at start (a rerun after a change), the finished conversion is promoted at the next start
  instead, because the game has indexed the folder.

## Limits

What was **not verified** comes first, because it limits everything else:

- **Nothing has been driven or listened to.** The checks are the conversion of all 93 cars, the game's parser and readers
  accepting what was written, and unit tests. No converted car has been driven by a person, and no recording has been heard
  in the game ([car-sound.md](car-sound.md#the-2012-cars-recordings)).
- **The 2012 stats are a reading of the data.** The layout of the handling definition is verified on all 93 cars, but several
  meanings are listed as unconfirmed in [chameleon-vehicle-stats.md](../formats/chameleon-vehicle-stats.md): the torque
  spline's interpolation, the differential fields that decide the drive layout and the locks, the tyre grip curves, the
  top-speed fields and the nitrous strength. The tyre grip scale of the game's builder is a calibration that has not been
  checked on a skidpad ([car-stats.md](car-stats.md#stats-stats-in-the-car-manifest)).

Then what the conversion does not do:

- **Handling is each car's own, approximately.** Mass, engine, gearbox, tyre grip and brakes are the 2012 car's numbers;
  springs, dampers, steering, downforce, the handbrake, gear efficiency and engine braking are the engine's generic values,
  because the 2012 files give them in units that are not decoded. Damage numbers are the shared racing table
  ([car-collision.md](car-collision.md#damage)); the HUD is the game's own.
- **No vinyls, rims, spoilers, scoops, tints, body kits or hoods**: the 2012 game has none to offer, so the shop lists only
  the paints. The performance bars of the shop are computed from each car's stats.
- **Collision is fitted, not copied.** The 2012 damage objects' zone numbers and crumple animations (not decoded; the hinged
  parts are, [below](#hinged-parts)), headlight
  beams, brake discs (they are part of the wheel), spoiler and mirror animation, and the 2012 lamp locators are not
  converted. The sets were not compared with how the cars drive in a window.
- **Collision is fitted, not copied.** The 2012 damage objects (hinged parts, per-zone thresholds: not decoded), headlight
  beams, brake discs (they are part of the wheel) and spoiler and mirror animation are not converted. The lamps are:
  markers from the lamp locators, but no reverse lamp (the 2012 files have no record for one) and flat lit colours
  ([car-lamps.md](car-lamps.md)). The sets were not compared with how the cars drive in a window.
- **Tail pipes are the car's own** (1, 2, 3 or 4 a car, in the places the 2012 game has them: the 918 Spyder's on top of the rear deck, the Viper's at the sides). The BMW 1M's bundle holds pipes 0.3 m beyond its body (the body is the *Cars* tie-in model's), and one unnamed bundle (122706) has no effect records, so it gets pipes derived from its body. The effects that follow the nitrous (afterburn, bit 14) and the 2012 flame shapes are not used: the game's own flame emitters play at the pipes.
- **Paint names.** The 2012 records carry no names; a paint is named by its finish and colour (`Metallic #AD2B2C`).
- **Normal, specular and effect maps are dropped**: the 2005 car shader has its own lighting.
- **Names.** The mod gives every car a display name and a brand. The garage lists the car under both
  (`scenes/world/garage.rs`); the other screens that name cars were not checked.
- **Liveries are numbered, and the picture is all there is.** The 2012 game names its liveries with images, not text, so the
  shop calls them `Livery 1` to `Livery 5`; a livery changes only the paint picture, as in the 2012 game. The pictures are laid
  over the paint colour by their alpha, so an opaque livery hides the paint the player picks. The car has no livery until the
  player picks one (its stock look is the flat paint, not the vehicle's own bundle's picture). Eleven other bundles have a
  livery technique with a shared or partial picture; they have no alternative to choose from and stay flat paint.
  Older conversions made each livery bundle a car of its own (20 more cars with generated ids): those ids are gone after the
  next conversion, while the ids of the other 73 cars are unchanged.
- **Traffic and low-detail bundles** (`_TR`, `_LO`) are not imported.
- A screen the game has open when the conversion finishes keeps the car list it built; open it again to see the cars.

## Hinged parts

The 2012 body is one skinned model, and its damage object names the parts that hang on hinges, each after a bone of the
car's skeleton ([chameleon-hinged-parts.md](../formats/chameleon-hinged-parts.md)). The conversion gives each part the
triangles weighted to its bone and the bones below it, and cuts them out of the finest body solid (`_A` and `_B`, which share
the finest level) into a solid of their own; vertices, normals and materials are those of the body, so an intact car draws as
before. Per part it writes a `[[damage_parts]]` table ([car-damage-parts.md](car-damage-parts.md)):

| Table key | From the 2012 data |
|---|---|
| `name`, `model` | the bone's name without `Hinge` (`FrontLeftDoor`), and `<ID>_KIT00_FRONT_LEFT_DOOR` |
| `pivot` | the position of the part's bone, through the same transform as the body |
| `axis`, `angle` | the one largest swing of the three angle ranges: the axis and the end with the largest magnitude, ties to the first axis and the positive end (the 911: bonnet -15 about the lateral axis, left mirror 15 about the vertical axis, doors 1 about the length) |
| `zones` | the part's zone and the corners beside it: a front part follows `front`, `front_left` and `front_right`, a left part `left`, `front_left` and `rear_left` |
| `stages` | the ladder of damage fractions of the part, ascending |
| `mass_kg` | the part's mass (30 or 50 kg on the 911) |

The cars whose spoiler is a plain speed state (the bone `SpoilerHinge`, 10 bundles of 8 models: 911 Carrera S, 918 Spyder
2013 and Concept Study, LFA, SL65 AMG, SLS AMG, LP700-4 and Panamera Turbo S) get a part `Spoiler` the same way, whether or not their damage object lists
it, with a `[damage_parts.active]` table decoded from the 2012 state machine and animation: the open and close speeds, the
time, the slide and the turn ([chameleon-active-aero.md](../formats/chameleon-active-aero.md)); the Panamera's side flaps are
parts that hang from its spoiler (`parent`). The cars whose machines have timed transitions or other inputs (the air vents,
the Agera, the Veyron, the McLaren, the Venom, the Huayra) keep that part at rest: what the data does not state is not guessed.

On the 911 that is 8 parts, the 7 of its damage object with 12,119 of the 30,908 triangles of the finest body level, and the spoiler; 90 of the 93 cars have parts (the
SWAT van has no skeleton, and two cars have a skin of all-zero weights). Limits: the coarser body levels (`C`, `D`) are not
skinned and keep the parts in the body, so a damaged car far away looks intact; the 2012 ranges are a few degrees, so the
parts hang slightly (the bonnet lifts by up to 15 degrees) until they fall off, which is the game's rule, not the 2012
data's ([car-damage-parts.md](car-damage-parts.md#falling-off)); the units and the meaning of the ladder are inferred. Nothing has been seen in a running game.

## Tests

`cargo test -p nfsmw-2012-content` runs the unit tests (settings, ids, fitting, textures, promotion and fingerprint, the
manifest the mod writes against the game's parser). With `MW2012_GAME_DIR=<2012 install>` set, `cargo test -p
nfsmw-2012-content -- --ignored` converts a car through the background job (checking that its seven hinged parts are cut out of the body without losing a triangle), reads it back with the game's readers, and
converts every car with the time it takes. The test `customize` also needs `NFSMW_GAME_DIR=<2005 install>`, because the
game builds a car's tables and physics around the 2005 install even though the conversion never reads it: it converts one
car and checks that the shop offers it its own paints and no parts of any other car, that a paint reaches the body texture,
that it has tail pipes, that its physics are built from its own `[stats]` and its own `BOUNDS.BIN` (peak torque and brakes as the 2012 car and the
multipliers of the test's `config.toml` make them, nitrous, gear ratios) and that its engine loops are readable WAVs:
`cargo test -p nfsmw-2012-content --test customize -- --ignored --nocapture`.
`NFSMW_GAME_DIR=<2005 install> cargo test -p nfsmw rescan` checks that content added after start-up reaches the copies of the
install that are already open. `CHAMELEON_GAME_DIR=<2012 install> cargo test -p chameleon-bundle -p chameleon-vehicle --
--ignored` loads every car of the install. To look at a car without opening a window:

```
nfsmw view-car POR911CARR12 --screenshot out.png --muted
nfsmw view-car POR911CARR12 --part paint=1 --yaw 55 --screenshot out.png --muted
```
