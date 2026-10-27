# The NFS: Most Wanted (2012) content mod

`nfsmw-2012-content` adds the cars of **NFS: Most Wanted (2012)** to this game: 67 more cars to choose from (the install has 93 car bundles; 20 are
liveries of other cars, which the shop offers as liveries, and 6 are a car the vehicle list has under a second vehicle id, which
the shop offers as paints of the one car), with their
own bodies, interiors, glass, lamps, wheels, textures, paints, stats, collision and sounds. The 2012 game runs on a
different engine (Criterion's Chameleon, not Black Box), so the cars are converted, not copied. It is a native mod like
[`hello`](../../docs/modding/examples.md), the only mod of the repository that is built with the workspace:
[`mods/nfsmw-2012-content`](.). The readers behind it are the three `libs/chameleon-*` crates
(formats: [chameleon-bundle](../../docs/formats/chameleon-bundle.md), [chameleon-gfx](../../docs/formats/chameleon-gfx.md),
[chameleon-vehicle](../../docs/formats/chameleon-vehicle.md)).

**Nothing from either game is in the repository or in a release.** The cars are made on your machine, in the background
while the game runs, from your own 2012 install. Without one the mod adds nothing and the game is unaffected.

Each converted car is a car of its own ([api-content.md](../../docs/modding/api-content.md#cars)): one more entry in the car list, named by its
folder, with its own geometry, textures, collision bounds, stats and sounds. **Nothing is taken from a car of the 2005 game,
and the conversion does not read the 2005 install at all.** The body is not stretched onto a stock wheel layout, and the
wheels sit at plus and minus half the car's wheelbase with the model centred on it.

## Using it

1. Build the mod and copy it into the game's mods folder with one command, from the repository folder:

   ```sh
   cargo install-mod nfsmw-2012-content
   ```

   It runs `cargo build --release -p nfsmw-2012-content` and puts `mod.toml` and the built library in
   `%APPDATA%\nfsmw\config\mods\nfsmw-2012-content\` (`~/.config/nfsmw/mods/nfsmw-2012-content/` on Linux; see
   [Using mods](../../docs/modding/README.md#using-mods)); nothing else in that folder is touched, so the mod's own
   `config.toml` and converted cars stay. Run it again after every change to the mod, with the game closed (a running
   game holds the library). `cargo install-mods` installs every mod the workspace builds, `--debug` installs a debug
   build and `--dir MODS_FOLDER` another mods folder; a build by hand with `cargo build --release -p nfsmw-2012-content`
   and a copy of `mod.toml` and the library works too.
2. Start the game once. The mod writes a commented `config.toml` next to its `mod.toml` (it does not, when the game's own
   config file already has a `[mods.nfsmw-2012-content]` table).
3. Put the folder of your 2012 install in it and start the game again:

   ```toml
   source = 'D:\Need For Speed Most Wanted 2012'
   ```

   Or, if you keep all your settings in the game's config file, put it there instead (see
   [Where the settings live](#where-the-settings-live)):

   ```toml
   [mods.nfsmw-2012-content]
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
[chameleon-customization.md](../../docs/formats/chameleon-customization.md)). The 2012 game lists a livery as a vehicle of its own, with
a bundle of its own; the mod makes one car of a vehicle and lists the pictures of its bundles, named as the 2012 game names them
(`Hero`, `Race`, `Street`, `Abstract`, `Urban`), in the shop's vinyl category ([car-customization.md](../../docs/modding/car-customization.md#liveries)).
The car wears its first livery as its stock look, and the shop's `No livery` choice gives the plain paint.
The performance upgrades and **nitrous** work on the car's own stats; the cars have nitrous from the factory. How the shop
treats a car is in [car-customization.md](../../docs/modding/car-customization.md), how it drives in [car-stats.md](../../docs/modding/car-stats.md), how it sounds
in [car-sound.md](../../docs/modding/car-sound.md); this page is the mod's settings and what the conversion writes.

### Where the settings live

The settings come from the host's `bb.config` ([api-config.md](../../docs/modding/api-config.md)), which merges two places, either of which may be
missing:

1. `config.toml` in the mod's folder (next to `mod.toml`);
2. the table `[mods.nfsmw-2012-content]` in the game's own config file (`%APPDATA%\nfsmw\config\config.toml` on Windows,
   `~/.config/nfsmw/config.toml` on Linux), with the sub-tables written as `[mods.nfsmw-2012-content.performance]`,
   `[mods.nfsmw-2012-content.police]` and `[mods.nfsmw-2012-content.car.POR911CARR12]`.

The same keys work in both. A key in the game's config file wins over the same key in `config.toml`; tables merge key by key
and a list (`cars`, `exclude`, `add`) in the game's config file replaces the whole list of `config.toml`. For example, with
`config.toml` as the mod wrote it and this in the game's config file:

```toml
[mods.nfsmw-2012-content]
source = 'D:\Need For Speed Most Wanted 2012'
role = "both"

[mods.nfsmw-2012-content.performance]
power = 1.05

[mods.nfsmw-2012-content.car."Aston Martin"]
grip = 1.05
```

the mod runs with that `source`, `role` and `power`, and with every other value of `config.toml`. The game's config file must
hold `[mods]` as a table: if it has the older line `mods = "all"`, change it to a `[mods]` table with `mode = "all"` (the
[mod mode](../../docs/modding/api-config.md#the-modsid-table) has both forms), since TOML does not allow a key and a table of one name. The
mod writes its commented template only when neither place has settings. The import fingerprint (`.import-stamp`) is made from
the merged settings, so a change in either place converts the cars again at the next start. Both places are read when the game
starts; a change needs a restart.

### Settings (the keys of either place)

Every key is optional. An unknown key is an error and no car is added, so a typo is not silently ignored.

| Key | Meaning |
|---|---|
| `source` | The 2012 install folder. Not set: nothing is converted |
| `cars` | `"all"` (default) or a list of ids or name fragments (`["Porsche", "LAMGALLARD11"]`) |
| `exclude` | Ids or name fragments to leave out |
| `max_texture_size` | Longest texture side, a power of two from 16 to 4096 (default 1024); bigger maps lose their top mips |
| `role` | `"player"` (default), `"cop"` or `"both"`: who can use the cars that are not police cars (the police cars have their own setting, `[police]`) |
| `[police]` | Which cars are police cars and where they chase: `auto` (default `true`), `add`, `remove`, `role`, and `[police.car.<id or name fragment>]` with `class`, `heat` and `cop` entries ([Police cars](#police-cars)) |
| `own_engine_sound` | `true` (default): each car gets the recordings of its own 2012 bundle, written into the mod folder as WAVs (`files/SOUND/MODENGINE/<ID>/`): its engine loops, and its gear shifts, turbo whistle and blow-off, backfires and pops. `false` writes none (the shared recordings included): the cars play the game's built-in engine and have only the game-wide sounds. An approximation, see [car-sound.md](../../docs/modding/car-sound.md#the-2012-cars-recordings) |
| `[performance]` | Optional multipliers on the stats read from each car's own 2012 data: `power` (torque at every engine speed), `mass`, `grip` (tyre friction), `brakes` (brake torque), `nitrous_power` (the extra torque of the nitrous), all 1.0 by default, and `nitrous` (`true` by default: the car keeps the nitrous its 2012 data gives it; `false` removes it). The mod applies them when it writes the car, so the game only sees the resulting absolute `[stats]` table ([car-stats.md](../../docs/modding/car-stats.md)); a car whose settings are left alone is exactly the imported 2012 car. Ranges: `power`, `mass`, `brakes` 0.25 to 4, `grip` 0.25 to 3, `nitrous_power` 0 to 4; results are clamped to the ranges of `[stats]` |
| `[car.<id or name fragment>]` | The same keys for one car or a make, changing only the keys it lists: `[car.POR911CARR12]`, `[car.Lamborghini]`. The tables apply in the order of their names over `[performance]`. |

The police cars of the install get a name from their siren corona object when their behaviour objects carry no model year
(`Dodge Charger SRT8 Cop`, `Ford Crown Victoria COP`); the other cars keep the ids they had. The ids are made from the car's name (`POR911CARR12` for the 2012 Porsche 911 Carrera S, at most 12 characters); the mod's log
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
table instead ([car-stats.md](../../docs/modding/car-stats.md)).

## What is converted

| 2012 | In the game |
|---|---|
| Body, interior, lamps, mirrors (finest of 3 levels, 5,000 to 36,000 triangles) | `<ID>_KIT00_BODY_A..D` (the car's own folder), in car space (+x forward, +y left, +z up), with the 2012 shape unchanged: only the axes are permuted and the car is centred on its wheelbase (`src/fit.rs`) |
| Tyre, brake disc, rim and caliper | one `<ID>_KIT00_FRONT_TIRE_A..D` solid, rim face at y = 0, as the game's wheels are modelled; the game draws it at the four hubs the car's `[wheels]` table gives ([The wheels](#the-wheels)) |
| Driver | the character the car's bundle sets up (one of the 2012 game's two seated people, `CHARACTERS/ALL_CHARS.BNDL`), at the seat the bundle gives, with its textures: `<ID>_KIT00_DRIVER_A..D`, drawn while driving ([The driver](#the-driver)) |
| Diffuse maps (BC1, BC3) | the car's `TEXTURES.BIN` as DXT1 and DXT5, cut to `max_texture_size` |
| Paint surfaces | the placeholder `<ID>_SKIN1` with light material `CARSKIN`, which the game swaps for the paint the player picks. Their texture coordinates are the 2012 car's own |
| Liveries | the paint picture of each bundle of a vehicle that has several, or of the only bundle of a car that wears one (2048 x 1024 BC1, cut to `max_texture_size`): the vehicle's own bundle first, then its liveries in vehicle list order, as `<ID>_LIV1` to `<ID>_LIV5` in the car's `TEXTURES.BIN` and the `liveries` list of its `[customization]` table. A livery bundle is the car with that one texture replaced; one that is not (another body, no picture) is converted as a car of its own. The liveries are named by the vehicle's paint pack (`libs/chameleon-vehicle/src/paint_pack.rs`, the text table's `HERO`, `RACE`, ...), and the car wears the first as its stock look (`default_livery = 0`). The twelve vehicles with two-colour paint packs (62 packs) get one livery per pack, a baked picture of the pair of colours, named by the pack (`src/two_colour.rs`, `libs/chameleon-vehicle/src/{colour_palette,paint_surface,two_colour}.rs`; see Limits). Vehicles of the list that are one car under two vehicle ids (the same name; 6 of them) are one car, led by the one with liveries, then the one with most paints, and lend it their paints (`src/vehicle_list.rs`, `src/liveries.rs`, `src/fold.rs`, `src/variants.rs`) |
| Paints | the `VehiclePaint` records of the car's bundle (`libs/chameleon-vehicle/src/paint.rs`): the colour, and the finish from the record's type, written as the `[customization]` table of the manifest, repeated colours dropped; the ones the vehicle's paint pack lists come first in the pack's order, so the default paint is the pack's first (`src/paints.rs`) |
| Stats | the car's own stats, read from its Genesys handling definition ([chameleon-vehicle-stats.md](../../docs/formats/chameleon-vehicle-stats.md)) and written as the `[stats]` table of its manifest: mass (body and wheels), weight distribution and centre-of-gravity height from the axles and rigid bodies, wheelbase, track and tyre size, the torque curve (at most 16 points, knots kept) with its idle and limiter, the gearbox and final drive, the drive layout and differential locks, the tyres' peak grip, the brake torque of each axle, the design top speed (the game solves the air drag from it) and the nitrous strength as `nitrous_boost`; then the `[performance]` multipliers (`src/stats.rs`, `src/performance.rs`) |
| Engine | the car's own sound graphs ([chameleon-sound-graph.md](../../docs/formats/chameleon-sound-graph.md)): the Ginsu voices as `.GIN` files, the idle and whine loops as WAVs, each layer with its curves of rpm, load and speed in the `[engine_sound]` table (`src/graph_voices.rs`, `src/graph_voices/speed.rs`; the player's engine sound and the gearbox whine only, the electric car's motor loops with a `rate` curve of the speed, and `shift_clunk = false` for a car with no gear change sound); a car without those graphs gets the looping samples of its bundle (decoded from the `SPS` streams inside them, see [chameleon-sound.md](../../docs/formats/chameleon-sound.md)), except the turbo's whistle, one 16-bit WAV per layer at one level, with the rpm each stands for (`src/engine.rs`, `src/sample.rs`) |
| Shifts, turbo, backfire, pops | the samples that the car's sound graphs play for the live values `GetPlayerNormalisedGearChangeTimeElapsed`, `BackfireGain`, `GetPopProbability`/`PopGain`, `NormalisedTurboRPM`/`TurboToneGain` and `TurboBlowoffGain` (the last two also use the shared `VEHICLES/WAVES.BNDL`), as WAVs and a `[car_sound]` table (`src/graphs.rs`, `src/effects.rs`). A car with no such graph has no such sound |
| Shared body hits, scrapes, pops | the first selector of the `Player Vehicle Collision Small`/`Medium`/`Big` graphs and the three samples of `Player Vehicle Scraping` (`SOUND/METALCAR.BNDL`), and the pops each car's graphs import from `VEHICLES/WAVES.BNDL`, written **once** under `files/SOUND/MODSHARED/` and named by every car's `[car_sound.shared]` (`src/shared.rs`). Evidence: [chameleon-shared-sounds.md](../../docs/formats/chameleon-shared-sounds.md) |
| Collision mesh and tyres | `files/CARS/<ID>/BOUNDS.BIN`: the car's own collision bounds, fitted to the poly soup of its bundle (the body's real extents, 166 to 256 triangles) and to the box of each tyre, through the same transform as the body: a root box, a lower body box, a cabin box, front and roof corner spheres and the silhouette the car rolls on. The one bundle without a collision mesh (the SWAT van) is fitted to its visible body. Format and what the 2012 game holds: [car-collision.md](../../docs/modding/car-collision.md), [chameleon-collision.md](../../docs/formats/chameleon-collision.md) (`src/collision.rs`) |
| Hinged parts | The parts the car's damage object lists (bonnet, bumpers, mirrors, doors, boot, as the car has them): cut out of the finest body solids into `<ID>_KIT00_<PART>_A` and `_B`, and a `[[damage_parts]]` table per part in the manifest ([below](#hinged-parts)) (`src/hinges.rs`) |
| Markers | `EXHAUST`, `LEFT_EXHAUST` and `RIGHT_EXHAUST` (where the nitrous and pop flames start), one per tail pipe, on `<ID>_KIT00_BODY_A..D`: the locators of the groups the car's nitrous spot effects name, facing the way the locator faces, mirrored when the effect says so; a car with no nitrous effect (the Tesla, the SWAT van) has none and so no flame (`src/pipes.rs`, [chameleon-exhaust.md](../../docs/formats/chameleon-exhaust.md)); a car whose bundle has no effect records at all (one) gets pipes derived from its body, at the rear face, low (`src/markers.rs`). The 2012 game has no spoiler or scoop to mount, so no mount marker is made |
| Engine auto-stop | the setting **2012 engine auto-stop** of the Gameplay settings (off by default, offered when a 2012 install is set): while the player drives free of any event (no race, no chase, no cutscene) and the car stands still for 5 seconds the engine switches off (no throttle, no nitrous, the tachometer at 0, no engine sound), and the throttle starts it again with the car's own 2012 start-up sound (`Player Vehicle Ignition`, found for 40 of the 93 cars; the others, and the 2005 cars, play the first 2012 car's, or a generated starter whirr), written as `files/SOUND/MODENGINE/<ID>/IGNITION.WAV` and `files/SOUND/MODSHARED/ENGINE/IGNITION.WAV`. **The 2012 data holds the ignition sound and an engine on/off state but no standstill rule**: the 5 seconds, the 0.5 m/s, the throttle that restarts and the 0.6 s of cranking are unconfirmed design values ([chameleon-engine-state.md](../../docs/formats/chameleon-engine-state.md), [api-engine.md](../../docs/modding/api-engine.md); `src/engine_stop.rs`, `src/ignition.rs`) |
| Nitrous sounds | the burn loop, the end sound and the start sound of the 2012 heat level sound set (`GAMELOGIC/HL_*.BNDL`, found by the expression `NitrousState.UsingNitrous` and the trigger `NITROUS_POSTFX_OFF`), as stereo WAV files in `files/NITROUS/MW2012/` with a `profile.toml`, and the value **NFS Most Wanted 2012** of the gameplay setting "Nitrous variant" (`bb.settings`), offered when a 2012 install is set. The post effects, flames, camera and HUD of the 2012 nitrous are not played; see [nitrous-variants.md](../../docs/modding/nitrous-variants.md) and [chameleon-nitrous.md](../../docs/formats/chameleon-nitrous.md) (`src/nitrous.rs`) |
| Tire smoke | the road smoke of the 2012 game (the effect every road surface of `HAWAII/SURFACELIST.BNDL` names for a tyre: seven smoke emitters, found by the surfaces' references and by their `EmitterWheel` operations), as the pictures and `profile.toml` in `files/TIRESMOKE/MW2012/` and the value **NFS Most Wanted 2012** of the gameplay setting "Tire smoke variant" (`bb.settings`), offered when a 2012 install is set: the game draws each layer at every smoking wheel with the 2012 pictures, sizes, lifetimes, alpha, drift and spread, with the rates scaled down; the game's smoke colour setting recolours it. See [effect-variants.md](../../docs/modding/effect-variants.md), [tire-effects.md](../../docs/tire-effects.md) and [chameleon-effects.md](../../docs/formats/chameleon-effects.md#wheel-effects-tyre-smoke) (`src/tire_smoke.rs`, `src/effect_variants.rs`) |
| Collision sparks | the sparks the 2012 game plays when a car hits something or slides along it (the effects the `VfxVehicleCollisions` objects of `GLOBALEFFECTS.BNDL` list most, read with `chameleon-effects`' `collision.rs`: stretched spark segments, the flash and the dust), as `files/SPARKS/MW2012/profile.toml` and the pictures, and the value **NFS Most Wanted 2012** of the gameplay setting "Collision spark variant (Original PC)" (`bb.settings`): the game throws a vehicle's sparks in the Original PC spark style from the profile, with the 2012 counts, speeds, lifetimes, gravity, colours and overbright, scaled by the strength of the hit (the closing speed) or the scrape (the sliding speed). The install's triggers, props and any channel the profile lacks stay the game's own. See [effect-variants-sparks-xenon.md](../../docs/modding/effect-variants-sparks-xenon.md) and [chameleon-collision-effects.md](../../docs/formats/chameleon-collision-effects.md) (`src/spark_assets.rs`, `src/spark_assets/`) |
| Restored sparks | the same 2012 effects as the two line sparks of the game's Restored (Experimental) spark style, as `files/XENON/MW2012/profile.toml` and the value **NFS Most Wanted 2012** of the gameplay setting "Restored spark variant (sparks only)". **Sparks only:** the Restored style's wind contrail and view streaks have no 2012 equivalent, so they are not part of the variant and keep the game's own look. See [effect-variants-sparks-xenon.md](../../docs/modding/effect-variants-sparks-xenon.md) (`src/spark_assets/profile.rs`) |
| Markers | `LEFT_EXHAUST` and `RIGHT_EXHAUST` (where the nitrous flames start) on `<ID>_KIT00_BODY_A..D`, derived from the body: at the rear face, low, half way to the corners (`src/markers.rs`). The lamp markers `LEFT_HEADLIGHT`, `RIGHT_HEADLIGHT`, `LEFT_BRAKELIGHT`, `RIGHT_BRAKELIGHT` and `BRAKELIGHT_CENTRE` are the car's own lamp locators, picked by its lamp records ([car-lamps.md](../../docs/modding/car-lamps.md)). The 2012 game has no spoiler or scoop to mount, so no mount marker is made |
| Lamp surfaces | the triangles each of the four 2012 lamp circuits (brake, head, reverse, tail) lights, from the lit pattern of the lens strips and lamp interiors, drawn with the game's lamp placeholders of their circuit and side, and the car's own `<ID>_KIT00_HEADLIGHT_`, `_BRAKELIGHT_`, `_TAILLIGHT_` and `_REVERSELIGHT_` `ON`/`OFF` textures (flat colours); the rest of the lamp interiors is dark grey ([car-lamps.md](../../docs/modding/car-lamps.md)) |
| Glass and lamp lenses | a translucent tint texture with light material `WINDSHIELD` |
| Untextured surfaces | a flat colour texture from the material's diffuse colour (dark grey when it has none) |
| Normals | recomputed from the triangles |
| Brand logos | the manufacturer table of the vehicle list and the logo pictures under `UI/IMAGES/STREAMED/` ([chameleon-manufacturers.md](../../docs/formats/chameleon-manufacturers.md)): for each distinct brand of the converted cars that the table names (Bentley, BAC, Koenigsegg, Pagani, Marussia, Ariel, Tesla, SRT, Shelby, Land Rover and the others, 32 of the 33 brands of the install; the cars' `NFS` brand has none), `manufacturers/<id>.toml` (`name` is the car's brand, `override = false`) and the logo as `files/UI/MAKERS/<ID>.PNG`: the picture cut to its visible part, scaled to fit with a margin (at most 2x) and centred on a transparent 300 x 150 RGBA canvas, in its own colours. The game's own logo of a brand it already has (Porsche, BMW, Subaru) wins over it; a brand with no logo has no file, and the game draws its name (`src/brands.rs`, `src/logo.rs`, [api-content.md](../../docs/modding/api-content.md#manufacturers)) |

Levels of detail: the game draws nothing for a level a car lacks, so A and B are the finest body, C and D the coarser
ones. The vertex limit of one solid (60,000) can force a coarser level. The manifest also carries `role`, a display name
(`Porsche 911 Carrera S 2012`) and the brand (`src/names.rs`).

## How it works with the host

- **At load** the mod learns its own folder from the host table (`ctx.mod_dir()`, see [api.md](../../docs/modding/api.md#lifecycle)). It
  promotes a finished conversion left by an earlier start, reads `config.toml`, and, unless the content on disk is still
  right, starts the conversion thread and returns.
- **The conversion** runs on a pool of all cores but one (at most six), one car per task, and builds into `.staging/`. It
  never writes where the game reads. The fingerprint is written last, so a run cut short is thrown away.
- **When it is done** (the mod sees it in its frame events: the worker cannot call the host) the mod moves `.staging/files`,
  `.staging/cars` and `.staging/manufacturers` into place and calls `bb.content.rescan()`. The host adds the mod's `files/` as a *late layer* of the
  install, which every copy of the install shares, including the ones the scenes already hold, and publishes the cars its
  `cars/` folder declares; the car tables list them from then on. See [api-content.md](../../docs/modding/api-content.md#content-registries-bbcontent).
- If content was already live at start (a rerun after a change), the finished conversion is promoted at the next start
  instead, because the game has indexed the folder.

## Limits

What was **not verified** comes first, because it limits everything else:

- **Nothing has been driven or listened to.** The checks are the conversion of all 93 cars, the game's parser and readers
  accepting what was written, and unit tests. No converted car has been driven by a person, and no recording has been heard
  in the game ([car-sound.md](../../docs/modding/car-sound.md#the-2012-cars-recordings)).
- **The 2012 stats are a reading of the data.** The layout of the handling definition is verified on all 93 cars, but several
  meanings are listed as unconfirmed in [chameleon-vehicle-stats.md](../../docs/formats/chameleon-vehicle-stats.md): the torque
  spline's interpolation, the differential fields that decide the drive layout and the locks, the tyre grip curves, the
  top-speed fields and the nitrous strength. The tyre grip scale of the game's builder is a calibration that has not been
  checked on a skidpad ([car-stats.md](../../docs/modding/car-stats.md#stats-stats-in-the-car-manifest)).

Then what the conversion does not do:

- **Handling is each car's own, approximately.** Mass, engine, gearbox, tyre grip and brakes are the 2012 car's numbers;
  springs, dampers, steering, downforce, the handbrake, gear efficiency and engine braking are the engine's generic values,
  because the 2012 files give them in units that are not decoded. Damage numbers are the shared racing table
  ([car-collision.md](../../docs/modding/car-collision.md#damage)); the HUD is the game's own.
- **No vinyls, rims, spoilers, scoops, tints, body kits or hoods**: the 2012 game has none to offer, so the shop lists only
  the paints. The performance bars of the shop are computed from each car's stats.
- **Collision is fitted, not copied.** The 2012 damage objects' zone numbers and crumple animations (not decoded; the hinged
  parts are, [below](#hinged-parts)), headlight
  beams, brake discs (they are part of the wheel), spoiler and mirror animation, and the 2012 lamp locators are not
  converted. The sets were not compared with how the cars drive in a window.
- **Collision is fitted, not copied.** The 2012 damage objects (hinged parts, per-zone thresholds: not decoded), headlight
  beams, brake discs (they are part of the wheel) and spoiler and mirror animation are not converted. The lamps are:
  markers from the lamp locators (head, brake, centre brake and reverse lamps) and flat lit colours
  ([car-lamps.md](../../docs/modding/car-lamps.md)). The sets were not compared with how the cars drive in a window.
- **Tail pipes are the car's own** (1, 2, 3 or 4 a car, in the places the 2012 game has them: the 918 Spyder's on top of the rear deck, the Viper's at the sides). The BMW 1M's bundle holds pipes 0.3 m beyond its body (the body is the *Cars* tie-in model's), and one unnamed bundle (122706) has no effect records, so it gets pipes derived from its body. The effects that follow the nitrous (afterburn, bit 14) and the 2012 flame shapes are not used: the game's own flame emitters play at the pipes.
- **Paint names.** The 2012 records carry no names; a paint is named by its finish and colour (`Metallic #AD2B2C`). The game's
  own names (`SILVER METALLIC`) are in the paint packs and the text table, and are not used yet: a paint's name is what a saved
  selection stores, so renaming would break the My Cars slots that hold a paint.
- **Two-colour cars are liveries.** Twelve vehicles have paint packs of two colours (the Veyron Super Sport and Grand Sport Vitesse,
  the BMW M3 E92, the Challenger SRT8 392, the Ford GT, the Boss 302, the Cobra 427, the Caterham R500, the Ariel Atom, the Shelby GT500, the
  Pontiac Trans Am and the Dodge Charger R/T): a pack names a pair of the car's paints, and the second colour covers the stripes, roofs
  and lower panels (whole meshes, or where a mask texture says). Each pack is a livery of the car, 62 in all, named by the game's
  name of the pack (`Silver Metallic`, `Red Gloss`; a name two packs share is numbered, `Silver Metallic 2`), whose picture is the
  pair of colours baked at conversion time (`<ID>_LIV<n>`: the mask of the car blended in the two colours, or two blocks of colour
  for a car with no mask, the paint meshes pointed at the right one; [the format](../../docs/formats/chameleon-customization.md#two-colour-paint)).
  The car wears its first pack from the start and `No livery` gives the plain paints. The picture is opaque flat colour: the
  metallic finish, the specular colour and the Veyron GSV's weave on its second-colour surfaces are not carried over, and a
  livery hides the paint the player picks while it is on. The GSV's old `Livery 1` (that weave picture over the whole car) is gone.
  Not checked against the 2012 game's own look: the bake follows the shader as read. The McLaren MP4-12C and F1 LM have no second
  colour in the install (plain paint surfaces), so they are offered their colours only.
- **Normal, specular and effect maps are dropped**: the 2005 car shader has its own lighting.
- **Names.** The mod gives every car a display name and a brand. The garage lists the car under both
  (`scenes/world/garage.rs`); the other screens that name cars were not checked.
- **Liveries are the game's own, five for five cars.** The 2012 game has liveries for five vehicles only (the BMW M3 E46 GTR, the
  Lamborghini Diablo SV, the Porsche 911 GT2, the Nissan 350Z and the Skyline R34 GT-R: the stock `Hero` and four it unlocks online;
  it has no others, not for the Bugatti or the McLaren). The shop calls them by the names of the 2012 game's paint packs (`Hero`,
  `Race`, `Street`, `Abstract`, `Urban`; `Livery 1` and so on where a pack does not name one, which also keeps the numbered names
  of older conversions out of the way: a saved livery of the old names is no longer found); a livery changes only the paint
  picture, as in the 2012 game. The pictures are laid
  over the paint colour by their alpha, so an opaque livery hides the paint the player picks. Every car with a livery wears its
  first one (the picture of the vehicle's own bundle) from the start; the shop's `No livery` choice takes it off and leaves the
  plain paints and finishes. A car with a single bundle offers its picture as `Livery 1` when the paint surfaces
  that draw a `Livery` technique make up most of its paint (the 918 Spyder, the Bentley, the Lancia Delta and four of the
  five police vehicles); a picture on one panel next to plain paint (the Raptor, the Gallardo), a `TwoPaintMask` picture (a mask
  of the second colour, not a livery; the cars that have one offer their two-colour packs instead, see above) and the SWAT van's are
  not offered.
  Older conversions made each livery bundle a car of its own (20 more cars with generated ids) and the second vehicle id of a car
  another one (6 more, the R34 GT-R with plain paint among them): those ids are gone after the next conversion, while the ids of
  the other 67 cars are unchanged. A My Cars slot of a car that is gone stays in the list and says its car is not installed.
- **Traffic and low-detail bundles** (`_TR`, `_LO`) are not imported.
- A screen the game has open when the conversion finishes keeps the car list it built; open it again to see the cars.

## Police cars

Five of the install's 93 bundles are the 2012 game's **police vehicles**, and they are converted as police cars of the 2005 game
(`role = "cop"`, not in the garage, in the cop waves) instead of player cars. They are found by what their bundle holds, not by
a list: a siren corona object (`<Car>_Cop_Coronas`), a `LightBarHinge` part, or the SWAT van's name
([evidence](../../docs/formats/chameleon-police.md)); no other bundle has any of them, and none of the five is in a list of cars the
player may be given.

| Car (the mod's display name) | Class | Default levels: `heat count @chance` |
|---|---|---|
| Ford Crown Victoria COP | patrol | `1 x4 @100`, `2 x4 @60`, `3 x3 @40` |
| Dodge Charger SRT8 Cop | patrol | `2 x3 @40`, `3 x4 @60`, `4 x5 @100`, `5 x4 @60`, `6 x3 @40` |
| Chevrolet Corvette Z06 2006 | pursuit | `5 x2 @50`, `6 x3 @60`, `7 x3 @70`, `8 x4 @80`, `9 x4 @90`, `10 x4 @100` |
| Ford Explorer Interceptor 2011 | heavy | `6 x2 @80`, `7 x6 @90`, `8 x2 @50`, `9 x2 @50`, `10 x2 @50` |
| NFS Swat Van 2012 | heavy | `8 x1 @30`, `9 x2 @40`, `10 x2 @50` |

**These levels are a design, not 2012 data.** The 2012 install's heat levels (40 behaviour records with their own tiers) were not
decoded, and nothing found ties a police vehicle to a level ([chameleon-police.md](../../docs/formats/chameleon-police.md#what-is-not-in-the-install-in-a-form-that-was-decoded)).
The table follows the kind of car and the 2005 game's own waves: the patrol sedan and the muscle car at the low levels (heat 1
is four mid-size cops), the sport utility with the numbers of the 2005 patrol SUV at heat 6 and 7 (`x2 @80`, `x6 @90`), the sports
car from heat 5, the van at the top. A police car that no row fits (one you `add`) is a patrol car in every level with 2 cars at
10 percent.

Change any of it in `config.toml` (a change takes effect at the next start, like every setting):

```toml
[police]
auto = true                  # false: no car is a police car unless `add` names it
add = ["Camaro"]             # more police cars: ids or name fragments (they get the patrol class, every level)
remove = ["Explorer"]        # cars that stay player cars, although the 2012 game made them police cars
role = "cop"                 # "both": the player may drive the police cars too (they are racing-category cars then)

[police.car.SWAT]            # one car or a make, by id or name fragment
class = "heavy"              # patrol, pursuit or heavy: damage, rep value and radio class (cop-cars.md)
cop = [{ heat = 8, count = 1, chance = 30 }, { heat = 10, count = 3, chance = 60 }]

[police.car.Crown]
heat = [1, 2]                # levels with the default numbers (2 cars at 10 percent)
```

A `heat` list or `cop` entries replace the car's default levels (they do not add to them); the tables apply in the order of their
names. Levels run from 1 to 10, `count` from 1 to 8, `chance` from 1 to 100. A mistake refuses the settings and no car is added,
as for the other keys. The tables go after the plain keys of the file.

The siren lamps of these cars (eight coronas on state bit 3 in each police bundle) are **not converted yet**: the converted police
cars have no flashing light bar, and no part of the roof is lit. The class and levels were not driven in a window; the mod's tests
check the rule against the install (`MW2012_GAME_DIR`, `cargo test -p nfsmw-2012-content --lib police -- --include-ignored`) and
the manifest against the game's parser.

## Hinged parts

The 2012 body is one skinned model, and its damage object names the parts that hang on hinges, each after a bone of the
car's skeleton ([chameleon-hinged-parts.md](../../docs/formats/chameleon-hinged-parts.md)). The conversion gives each part the
triangles weighted to its bone and the bones below it, and cuts them out of the finest body solid (`_A` and `_B`, which share
the finest level) into a solid of their own; vertices, normals and materials are those of the body, so an intact car draws as
before. Per part it writes a `[[damage_parts]]` table ([car-damage-parts.md](../../docs/modding/car-damage-parts.md)):

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
time, the slide and the turn ([chameleon-active-aero.md](../../docs/formats/chameleon-active-aero.md)); the Panamera's side flaps are
parts that hang from its spoiler (`parent`). The **McLaren and the Venom** get the `[wing]` machine without a top-speed animation, with their own speeds (120 and 80 mph) and brake hold; the **Huayra** gets four flaps with `[[damage_parts.drive]]` tables (turning, speed bands, rear brake). Both rest on assumptions, listed in [chameleon-active-aero.md](../../docs/formats/chameleon-active-aero.md#the-mclaren-the-venom-and-the-huayra).

The **Agera R** gets two flaps that open above 70 mph with the gas pressed (`gas = true`); the **air vents** of the 918 Concept
Study, the LP700-4, the Marussia B2 and the Aventador J become `AirVentN` parts with the waits of the 2012 timed transitions
(`open_after_seconds`, `close_after_seconds`); the **Veyron Super Sport and Grand Sport Vitesse** get the `[wing]` machine with
`WingN` parts (the wing and its arms) carrying the keys of its five animations. A part that only moves the parts below it (a
bone with no skin of its own) is still written, without a solid, so that its movement carries its children.

On the 911 that is 8 parts, the 7 of its damage object with 12,119 of the 30,908 triangles of the finest body level, and the spoiler; 90 of the 93 cars have parts (the
SWAT van has no skeleton, and two cars have a skin of all-zero weights). Limits: the coarser body levels (`C`, `D`) are not
skinned and keep the parts in the body, so a damaged car far away looks intact; the 2012 ranges are a few degrees, so the
parts hang slightly (the bonnet lifts by up to 15 degrees) until they fall off, which is the game's rule, not the 2012
data's ([car-damage-parts.md](../../docs/modding/car-damage-parts.md#falling-off)); the units and the meaning of the ladder are inferred. Nothing has been seen in a running game.

## The wheels

The wheel hubs come from the car's graphics spec; the tyre's height, width and reach come from its wheel mesh; the
converter writes them as the car's `[wheels]` table ([car-stats.md](../../docs/modding/car-stats.md#where-the-wheels-are-drawn-wheels)), and the
stats' wheelbase and weight distribution are those of the same axles, so the body, the physics and the drawn wheels agree. The
wheels are taken by their place and not their name (a few cars name them the wrong way round).

An axle is where the **body's arch** is. On all but four cars the spec's hubs, the vehicle info's axles and the arches agree
([chameleon-vehicle.md](../../docs/formats/chameleon-vehicle.md#hubs-and-the-bodys-arches)). Where they do not, the converter tests the
tyre against the body (`chameleon_vehicle::prefer_other`: the body's vertices inside the tyre at the hub and at the info's axle)
and takes the axle that has room for it:

- the **BMW 1M** (`BMW1M12`): its spec and info are the 1M's but its body is Lightning McQueen's, 3.5 m long against the 1M's
  4.3, so its tyres at the 1M's hubs (1.42 and -1.24) sat 0.4 m ahead of and behind the body's arches and had the body through
  them; at the info's axles (1.035 and -0.925) the arches are clear. This was the misplaced wheels;
- the **Venom GT Spyder**'s front hubs (behind its doors, z 0.125; its arch is at 1.3) and the **E46 GTR**'s rear hubs (10 cm
  behind the arch). An axle whose hubs are more than 30% of the body's length from its end is taken from the vehicle info
  without the test (the Venom), as before;
- every other car keeps its hubs: a car with no arch clear at the info's axle (the Lancia Delta's body holds liner vertices at
  both) is not moved.

The tyres' collision boxes follow the same axles (the hubs of the car are moved onto them before the bounds are fitted). With
the real install, `cargo test -p nfsmw-2012-content --lib fit_real -- --ignored --nocapture` tabulates, for every car, the
largest error (centre, height, outer and inner face, radius) of the drawn wheels against the model's own wheel mesh at the
axle used: with all of them within 2 cm (the largest is 1.4 cm). `cargo test -p chameleon-vehicle --lib arch --
--ignored` (with `CHAMELEON_GAME_DIR`) checks that exactly those four axles move. This checks the numbers the game is given,
not a drive: the Venom was looked at in a render (`nfsmw view-car HENVENOMGT12 --yaw 90 --screenshot out.png --muted`), the
1M and the E46 were not.

## The driver

Every car of the 2012 game but the BMW 1M has a driver: each car bundle sets one up (`Vehicle_DriverSetup`: which of the two
characters of `CHARACTERS/ALL_CHARS.BNDL` sits in the car, and the seat's position in the car's space), and the characters are
seated figures, skinned models in the pose of their rest skeleton ([chameleon-vehicle.md](../../docs/formats/chameleon-vehicle.md#the-driver)).
The conversion gives each of **92 of the 93 bundles** the solid `<ID>_KIT00_DRIVER_A..D`, which the game draws in the cockpit
when the car is driven (not in the garage or the shop), with a part of its own (`<ID>_DRIVER`, slot `DRIVER`) that the game
adds to its parts database with the body and the wheel ([car-customization.md](../../docs/modding/car-customization.md#the-files-of-a-car)). A
mod car with no such solid has no driver.

- **Placed by the bundle's own seat.** The character is moved by the seat the car's setup gives (no rotation, no scale), then
  through the same transform as the body, so the driver sits where the 2012 game seats it: left, right (the DB5, the R34, the
  Lancia Delta) or in the middle (the BAC Mono, the F1 LM), hands out to the wheel, in the open cockpits of the Atom, the Cobra
  and the Mono as well as under a roof. `src/source.rs` reads each character once (`Source::driver_figure`),
  `src/import.rs` seats it per car. No height fitting is done: the data places the head under the roof of 81 of the 92 cars and
  just above the low windscreen of the others.
- **Levels of detail.** The character has five levels (4,600 vertices down to 470); the solids `A` and `B` take the finest, `C` the
  second and `D` the third, as the body does.
- **Looks.** The character's own diffuse map (a 2048 x 1024 BC1 picture cut to `max_texture_size`, named
  after the texture's id in the car's `TEXTURES.BIN`), matte. The normal and specular maps are not carried.
- **Not done.** The BMW 1M (Lightning McQueen's body) has no seat in its setup (19, 19, 19), so no driver. The Explorer's body
  bakes a driver of its own (362 vertices, the same posture); the conversion takes it out so the car has the one driver. The
  figure is **static**: the rest pose, hands out on the wheel, with no steering animation (the character has an animation list
  of 28 clips that was not decoded), and no seat belt or harness mesh was looked for. Nothing has been seen in a running game;
  the reader's install test (`cargo test -p chameleon-vehicle --lib driver -- --ignored`, with `CHAMELEON_GAME_DIR` or
  `MW2012_GAME_DIR`: every car sets a driver up, the head under the body's top, the soles above the ground, the figure in the
  body's box) checks the numbers.

## The 2012 cameras

Every converted car also gets the 2012 game's cameras as three **extra camera views** ([api-world.md](../../docs/modding/api-world.md#camera),
`bb.camera` 1.1): **2012 Chase**, **2012 Bonnet** and **2012 Bumper**. Press **C** (the camera hotkey) past the game's own
cameras to reach them, or pick one in the options' Camera row (on the Player page and on the Gameplay page, which is also the
pause menu's, so the view can be switched in a running race); the choice is saved as
`camera_mode = "extra:nfsmw-2012-content/chase"` (and `bonnet`, `bumper`). On a car the mod did not convert, the views do not fit
and the game keeps its own cameras.

While a car is converted, the mod reads the cameras of its bundle with
[`chameleon-camera`](../../docs/formats/chameleon-camera.md) and writes them next to the car's files (`files/CARS/<ID>/CAMERAS.TOML`, with
the nose of the converted body, which anchors the bumper camera; the chase and bonnet offsets are shortened by how far the
conversion moved the car's origin to the middle of its axles, `CarCameras::move_origin_forward`). **A car converted before the
cameras were kept has no such file, and the views then do not fit it (the game keeps its own camera, which looks like the
views having no effect): the conversion version was bumped for this, so the cars are converted again on the first start after
the update and the views appear at the start after that.** The mod logs `no 2012 cameras` for a car without the file. In the game the mod steps that library's model once per frame with the
presented car (the position and axes of the car model, whose origin is the data's: on the ground between the axles, and not the
physics body's centre of gravity; speed, steering, pedals, yaw rate, ground contact, hits) and the host draws from the pose it
answers; the bonnet and bumper views hide the car's own body. The per-car values (the chase offset and height, the look-at point, the
pull-back and field of view with speed, the roll, the braking pitch, the shake) come from each car's own 2012 data; **how the
2012 game turns them into a camera is not known from its executable** (nothing of its camera code was found), so the model is built
from the data and every behaviour is **[unconfirmed]** in the format doc. The mounted cameras' numbers are offsets from an anchor
the data does not name: the mod anchors the bonnet camera at the car's origin and the bumper camera at its nose. None of this has
been seen in a running game yet; the model's tests and the reader's install tests check the numbers, not the look.

## Drift nitrous

In the 2012 game a drift fills the nitrous tank. The mod adds the gameplay setting **2012 drift nitrous** (on or off, off by
default, saved with the other settings; `bb.settings`), offered when a 2012 install is set. While it is on, the mod steps
`chameleon_vehicle::DriftModel` once a scene tick with the player's car (`bb.vehicles` 1.1: speed, slip angle, ground contact,
the strongest hit) and adds what a drift earned to the car's tank with `add_nos`; it works for any car the player drives,
the converted ones and the 2005 ones. The rule and its numbers (a slide of 7.5 degrees or more at 10 m/s starts a drift, it
pays 6 units a second of a 100 unit tank from 30 mph after 0.75 s, it ends when the slip stays under 3 degrees for 0.25 s or
the car hits something) are in [chameleon-drift.md](../../docs/formats/chameleon-drift.md); the payout is read from the
install's first car bundle when the setting is first on, the physics are the median car's. While a drift pays, the 2012 text
"DRIFTING" and its distance show on the HUD message line (`bb.hud`). A car with no nitrous system earns nothing, and with the
setting off nothing is read or changed (`src/drift_nitrous.rs`).

## License plates

Every car with a plate mesh (91 of the 93 bundles; the plate is eight quads and a 36-glyph sheet) gets its **proper default
text** and a plate the player can retype. In the 2012 game the text is the player's registration, and until the player sets one
it is a default of the car's **game pack**: `NEED4SPD` for the base game, `ULTIMATE`, `VELOCITY`, `MOVILGND` and `NFS HERO` for
the four DLC packs. The five strings are in `NFS13.exe`, the pack of a car is the `GamePackID` of its vehicle list entry
(+0x2C), and `GAMEPLAY.BNDL` gives the packs' numbers ([chameleon-plate.md](../../docs/formats/chameleon-plate.md)). The mod
reads them (`src/plate.rs`, `src/vehicle_list.rs`) and writes, for each car, the glyph sheet as `<ID>_PLATE` in `TEXTURES.BIN`
and a `[customization.plate]` table with the text, the 9 x 4 layout, 8 slots and `0` drawn as `O`. Without the executable
the cars keep the sheet as drawn (`ABCDEFGH`) and can still be retyped. In the garage, **Visual, License Plate** is a keyboard
prompt (letters and digits the sheet has, up to 8; empty goes back to the car's text); the text is kept per car and profile in
`plates.toml` beside the save, which the original game's saves never see ([car-customization.md](../../docs/modding/car-customization.md#license-plates)).
`nfsmw view-car <ID> --part plate=TEXT` shows a text. Not checked in a window: the drawing of a typed text on the car.

## Tests

`cargo test -p nfsmw-2012-content` runs the unit tests (settings, ids, fitting, textures, brand matching and logo canvas, promotion and fingerprint, the
manifest the mod writes against the game's parser). With `MW2012_GAME_DIR=<2012 install>` set, `cargo test -p
nfsmw-2012-content -- --ignored` converts a car through the background job (checking that its seven hinged parts are cut out of the body without losing a triangle), reads it back with the game's readers, and
converts every car with the time it takes. The test `customize` also needs `NFSMW_GAME_DIR=<2005 install>`, because the
game builds a car's tables and physics around the 2005 install even though the conversion never reads it: it converts one
car and checks that the shop offers it its own paints and no parts of any other car, that a paint reaches the body texture,
that it has tail pipes, that its physics are built from its own `[stats]` and its own `BOUNDS.BIN` (peak torque and brakes as the 2012 car and the
multipliers of the test's `config.toml` make them, nitrous, gear ratios) and that its engine loops are readable WAVs:
`cargo test -p nfsmw-2012-content --test customize -- --ignored --nocapture`.
`NFSMW_GAME_DIR=<2005 install> cargo test -p nfsmw rescan` checks that content added after start-up reaches the copies of the
install that are already open. The test `brands` converts a BAC, a Bentley, a Koenigsegg and a Porsche and checks that each brand got a manifest the game's parser reads and a 300 x 150 RGBA PNG with its logo drawn (`cargo test -p nfsmw-2012-content --test brands -- --ignored`).
`CHAMELEON_GAME_DIR=<2012 install> cargo test -p chameleon-bundle -p chameleon-vehicle --
--ignored` loads every car of the install; its `two_colour_tests` count the 62 two-colour packs of 12 vehicles, check that the paint
surfaces line up with the loaded meshes of all 93 bundles and bake a picture for each mask; the mod's `two_colour::tests::the_twelve_two_colour_vehicles_become_liveries` (`MW2012_GAME_DIR`, `-p nfsmw-2012-content --lib -- --ignored`) checks the 62 liveries of the 12 cars. To look at a two-colour car's livery:
`nfsmw view-car <ID> --part vinyl=2 --screenshot out.png --muted` (the liveries are among the vinyls, `<ID>` a converted car). The test `plate` (`MW2012_GAME_DIR`, `cargo test -p nfsmw-2012-content --test plate -- --ignored`) converts a base-game car and an NFS Heroes car and checks their texts, the sheet in `TEXTURES.BIN` and a text drawn on it, and counts the game packs of the vehicle list. To look at a car
without opening a window:

```
nfsmw view-car POR911CARR12 --screenshot out.png --muted
nfsmw view-car POR911CARR12 --part paint=1 --yaw 55 --screenshot out.png --muted
```
