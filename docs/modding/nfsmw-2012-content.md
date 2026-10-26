# The NFS: Most Wanted (2012) content mod

`nfsmw-2012-content` adds the cars of **NFS: Most Wanted (2012)** to this game: 93 more cars to choose from, with their
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

What the shop offers for a car's looks is the car's own: the **paints of its 2012 bundle**, 1 to 15 per car, which is all the
2012 game offers (no vinyls, rims, spoilers or tints: [chameleon-customization.md](../formats/chameleon-customization.md)).
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
| `own_engine_sound` | `true` (default): each car gets the recordings of its own 2012 bundle, written into the mod folder as WAVs (`files/SOUND/MODENGINE/<ID>/`): its engine loops, and its gear shifts, turbo whistle and blow-off, backfires and pops. `false` writes none: the cars play the game's built-in engine and have only the shared sounds. An approximation, see [car-sound.md](car-sound.md#the-2012-cars-recordings) |
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
| Paints | the `VehiclePaint` records of the car's bundle (`libs/chameleon-vehicle/src/paint.rs`): the colour, and the finish from the record's type, written as the `[customization]` table of the manifest, repeated colours dropped (`src/paints.rs`) |
| Stats | the car's own stats, read from its Genesys handling definition ([chameleon-vehicle-stats.md](../formats/chameleon-vehicle-stats.md)) and written as the `[stats]` table of its manifest: mass (body and wheels), weight distribution and centre-of-gravity height from the axles and rigid bodies, wheelbase, track and tyre size, the torque curve (at most 16 points, knots kept) with its idle and limiter, the gearbox and final drive, the drive layout and differential locks, the tyres' peak grip, the brake torque of each axle, the design top speed (the game solves the air drag from it) and the nitrous strength as `nitrous_boost`; then the `[performance]` multipliers (`src/stats.rs`, `src/performance.rs`) |
| Engine loops | the looping samples of the car's bundle (decoded from the `SPS` streams inside them, see [chameleon-sound.md](../formats/chameleon-sound.md)), except the turbo's whistle, one 16-bit WAV per layer at one level, and an `[engine_sound]` table in the car's manifest naming them with the rpm each stands for (`src/engine.rs`, `src/sample.rs`) |
| Shifts, turbo, backfire, pops | the samples that the car's sound graphs play for the live values `GetPlayerNormalisedGearChangeTimeElapsed`, `BackfireGain`, `GetPopProbability`/`PopGain`, `NormalisedTurboRPM`/`TurboToneGain` and `TurboBlowoffGain` (the last two also use the shared `VEHICLES/WAVES.BNDL`), as WAVs and a `[car_sound]` table (`src/graphs.rs`, `src/effects.rs`). A car with no such graph has no such sound |
| Collision mesh and tyres | `files/CARS/<ID>/BOUNDS.BIN`: the car's own collision bounds, fitted to the poly soup of its bundle (the body's real extents, 166 to 256 triangles) and to the box of each tyre, through the same transform as the body: a root box, a lower body box, a cabin box, front and roof corner spheres and the silhouette the car rolls on. The one bundle without a collision mesh (the SWAT van) is fitted to its visible body. Format and what the 2012 game holds: [car-collision.md](car-collision.md), [chameleon-collision.md](../formats/chameleon-collision.md) (`src/collision.rs`) |
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
- **Collision is fitted, not copied.** The 2012 damage objects (hinged parts, per-zone thresholds: not decoded), headlight
  beams, brake discs (they are part of the wheel) and spoiler and mirror animation are not converted. The lamps are:
  markers from the lamp locators, but no reverse lamp (the 2012 files have no record for one) and flat lit colours
  ([car-lamps.md](car-lamps.md)). The sets were not compared with how the cars drive in a window.
- **Tail pipes are approximated** from the body's shape; the 2012 tail pipe locators are not decoded.
- **Paint names.** The 2012 records carry no names; a paint is named by its finish and colour (`Metallic #AD2B2C`).
- **Normal, specular and effect maps are dropped**: the 2005 car shader has its own lighting.
- **Names.** The mod gives every car a display name and a brand. The garage lists the car under both
  (`scenes/world/garage.rs`); the other screens that name cars were not checked.
- **Livery bundles are separate cars.** The 2012 game ships some cars several times as liveries (five Skyline R34 bundles, for
  example), each a bundle of its own. The mod converts each bundle as a car of its own, with a number added to the id; it does
  not import the 2012 game's choice among liveries, and there is nothing to apply a livery to within a car.
- **Traffic and low-detail bundles** (`_TR`, `_LO`) are not imported.
- A screen the game has open when the conversion finishes keeps the car list it built; open it again to see the cars.

## Tests

`cargo test -p nfsmw-2012-content` runs the unit tests (settings, ids, fitting, textures, promotion and fingerprint, the
manifest the mod writes against the game's parser). With `MW2012_GAME_DIR=<2012 install>` set, `cargo test -p
nfsmw-2012-content -- --ignored` converts a car through the background job, reads it back with the game's readers, and
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
