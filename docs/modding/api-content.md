# Mod API catalogue: content, VFS and storage

Part of the [modding catalogue](README.md). Conventions are in [api.md](api.md#how-to-read-an-entry).
Rules in short (ADR 0005, section 5): mods add and replace **files** through a layered virtual file system, and
add **entries** to registries; both work without any code (a mod with no `[library]` is a content mod).
Registries for cars, music, fonts, scenery and interiors **seal** when start-up content is built; registering
later returns `Phase` (except under `--mod-dev`).

## VFS overlay

`GameDir` (`libs/game-install/src/vfs.rs`) becomes a stack of read-only layers: the install first (lowest
priority), then one layer per mod folder `files/`, ordered by `[content] priority` and then load order (later
wins). Lookup stays case-insensitive and `/`-separated. Format crates still take `&[u8]`; only `game-install`
sees the layers. The overlay is **file level**: a mod replaces or adds whole files. Merging two mods into one
`.BUN` chunk is not supported in the first delivery (a `patches/` layer that merges bChunk entries is a
follow-up; it would live in `libs/blackbox-chunk`).

| Entry | Signature sketch | Scope | Implemented in |
|---|---|---|---|
| Layer stack | `GameDir::add_overlay(install, &[Layer { root, priority, owner }])` | G | `libs/game-install/src/vfs.rs` (extend) |
| Exists | `vfs.exists(path: &str) -> bool` | G | same |
| Read | `vfs.read(path: &str, sink: impl FnMut(&[u8])) -> Result<u64>` streams the winning file's bytes | G | same, through `modding/vfs.rs` (new) |
| Size | `vfs.size(path: &str) -> Result<u64>` | G | same |
| List | `vfs.list(dir: &str) -> Result<Vec<String>>` the union over layers | G | same |
| Who provides | `vfs.provider(path: &str) -> Option<Provider { Install or Mod(String) }>` | G | same |
| Layers | `vfs.layers() -> Vec<LayerInfo { owner, priority, files }>` | G | same |
| Add layer | `vfs.add_overlay(dir: &str, priority: i32) -> Result<()>` for a folder a mod generates; `Load` only | G | same |
| Mod data dir | `ctx.data_dir() -> PathBuf` a writable folder for the mod (`<local data dir>/<app>/mod-data/<id>/`) | G | `modding/runtime.rs` (new), `libs/game-install/src/discover/config.rs` for the base dir |

The console command `vfs <path>` (new, host) prints which layer provides a path; `mods` lists the layers.

## Content registries (`bb.content`)

Two ways in, both producing the same entry: a TOML file under `content/<kind>/` in the mod folder, read by the
host at start-up, or a call in `Mod::load`. Keys are `modid:name`; a collision is `Exists`, logged, and the
second registration is dropped. Every entry may carry `source_game = "nfsmw" | "nfsu2" | ...` (default: the
running game); the host rejects an entry whose source game has no reader or layout table loaded.

| Entry | Signature sketch | Scope | Implemented in |
|---|---|---|---|
| Registry list | `content.list(kind: Kind) -> Vec<EntryInfo { key, owner }>` (any kind, including the game's own) | G | `modding/content.rs` (new) |
| Lookup | `content.get(kind, key) -> Option<EntryInfo>` | G | same |
| Unregister (own) | `content.remove(kind, key)`, valid in `Load` | G | same |
| Rescan | `ctx.content() -> Option<Content>`, `content.rescan() -> Result<()>` (built, `bb.content` 1.0): the mod's `files/` and `cars/` changed since start-up. The host adds `files/` as a late layer of the install (shared by every copy, `GameDir::add_late_overlay`) and publishes the new cars (`car::mods::publish`); what the game opens afterwards lists them. A layer that exists is not re-indexed; a car already published is not added twice. Main thread only: a worker thread tells the mod's event callback, which calls it | G (interface), MW (cars) | `libs/blackbox-modding/src/{abi/iface,host/table,sdk}/content.rs`, `crates/nfsmw/src/modding/backends/content.rs`, `modding/content/mod.rs` (`rescan`) |

### Cars

Works now in NFS:MW. A mod adds a car with its files and one manifest. The car id is the manifest's file name. A mod car is
**one more entry in the car list**, named by its folder: it is not a copy of a stock car and inherits nothing from the 2005
game (its parts, physics and sound are its own).

```
mods/example.pack/
  mod.toml
  cars/fastcoupe.toml                 the manifest
  files/CARS/FASTCOUPE/GEOMETRY.BIN   the solids FASTCOUPE_KIT00_BODY_A..D and FASTCOUPE_KIT00_FRONT_TIRE_A..D (car-customization.md)
  files/CARS/FASTCOUPE/TEXTURES.BIN   its textures
  files/CARS/FASTCOUPE/BOUNDS.BIN     its collision bounds (a BoundsPack): without it the car cannot drive
```

```toml
# cars/fastcoupe.toml
role = "both"                   # player, cop or both (default: player)
heat = [2, 3, 4]                # cop only: heat levels whose waves may use it, 2 cars at 10 percent (left out: every level)
cop_class = "patrol"            # cop only: patrol (default), pursuit or heavy (cop-cars.md)
[[cop]]                         # cop only, repeatable: one wave entry with its own numbers (cop-cars.md)
heat = 6                        # one level or a list of levels
count = 2                       # cars the wave wants, 1 to 8
chance = 80                     # weight of the pick, percent, 1 to 100
name = "Fast Coupe"             # display name (optional; default: the car id)
manufacturer = "Acme"           # brand (optional; default: none)

[stats]                         # optional: what the car is, in absolute SI units (car-stats.md)
mass_kg = 1295.0
torque_curve = [[1000.0, 360.0], [5600.0, 440.0], [7800.0, 390.0]]   # [rpm, N m]
gear_ratios = [3.91, 2.29, 1.55, 1.3, 1.08, 0.88, 0.71]
final_drive = 3.44
nitrous = true                  # nitrous from the factory

[customization]                 # optional: the car's own paints and parts (car-customization.md)
paints = [{ name = "Red", color = [173, 43, 44] }, { name = "Silver", color = [200, 204, 197], finish = "metallic" }]
rims = [{ name = "Five spoke", model = "FASTCOUPE_RIM1" }]   # a wheel solid of the car's own GEOMETRY.BIN
[[damage_parts]]                # optional, repeatable: parts that hang on hinges when damaged (car-damage-parts.md)
```

A key not listed in the manifest table below refuses the car. A car with `role = "cop"` or `"both"` is a police car with a wave
entry per heat level: [cop-cars.md](cop-cars.md). There is no key that names another car: a manifest with a
`clone` key is refused as an unknown key. (What changed: an earlier version made a mod car a copy of a stock car, kept that
car's record, parts, physics and sound, and had a `[performance]` table of multipliers over it. All of that is gone; `[stats]`
holds absolute values.)

Rules:

- A mod may write its `cars/` and `files/` folders in `Mod::load` (it finds its folder with `ctx.mod_dir()`): the game
  reads them after every mod has loaded. The [2012 content mod](nfsmw-2012-content.md) does, on a worker thread, and calls
  `bb.content` when it is done.
- The car id is the file name in upper case (`FASTCOUPE`; 1 to 32 letters, digits, `_` or `-`). Its `GEOMETRY.BIN` must be in the mod's own `files/` folder, not
  only in the install (`Reason::NoGeometry`).
- A folder that is already a car (stock, or another mod's) is refused (`Reason::NameTaken`). Heat levels run from 1 to 10.
- A key the game does not know refuses the car. So does an `[override]` table: arbitrary physics and attribute
  overrides are not offered (see open items). The `[stats]` table is: the car's mass, engine, gearbox, tyres, brakes,
  suspension and nitrous in SI units, each key checked against a range and a key left out taking the engine's generic
  value ([car-stats.md](car-stats.md#stats-stats-in-the-car-manifest)). The car's physics are built from these and from
  the car's own `BOUNDS.BIN`, and from nothing else.
- Each refused car is logged with its mod and reason. The game goes on without that car.
- Registration runs once in `open_install`, before any scene is built, and the result is sealed for the run. A changed
  car needs a restart.
- A cop car gets one record `{ car, count, chance }` in each heat wave it names, with the numbers of its `[[cop]]` entry, or 2
  cars at 10 percent (`COP_COUNT`, `COP_CHANCE` in `car/mods/cop.rs`) for the `heat = [..]` short form and for a car that names
  no level (every level). A car that is only a cop is a police type of the car table (usage 1, `Category::Police`); its physics are
  built from its stats like a player car's, and the AI may use its whole `top_speed_kph` (200 km/h when it gives none). Its
  damage, rep value and radio class come from `cop_class`. All of it is in [cop-cars.md](cop-cars.md).

The auto shop offers the car what its `[customization]` table lists (body paints and finishes, rim paints, tints, rims,
spoilers, roof scoops, vinyls), read from the table and the car's own files; no stock car's parts table is involved. The
performance upgrades and nitrous work as described in [car-stats.md](car-stats.md). The garage takes a mod car's display
name and brand (`display_name`, `manufacturer`), lists its cop-only cars under Police ([cop-cars.md](cop-cars.md)) and skips any
car whose physics cannot be built (for one, a missing `BOUNDS.BIN`); the other screens that name cars were not checked.

**Collision: `BOUNDS.BIN`.** The car's `files/CARS/<ID>/` folder holds `BOUNDS.BIN`, its collision bounds in the game's
own `BoundsPack` format: the box its rigid body is built from, its contact shapes, the ground mesh and, from the box, its damage
zones. **It is required for a car to drive:** the game reads the car's own file and never a stock car's, and refuses a car
without a valid one, with the reason in the log. What it holds and how to write one: [car-collision.md](car-collision.md).

**Sound.** The car's sound is its own, never another car's: its engine loops (`[engine_sound]`, else a built-in engine),
and its gear shifts, turbo whistle and blow-off, backfires and pops (`[car_sound]`, else silence). Collisions, skids,
nitrous and the road share the game-wide banks. See [car-sound.md](car-sound.md).

Known gaps: body kits and hoods (a mod car has none), and a code form (`content.cars().register(..)`) are not built.

| Entry | Signature sketch | Scope | Implemented in |
|---|---|---|---|
| Car manifest | `cars/<id>.toml` with `role`, `heat`, `[[cop]]`, `cop_class`, `name`, `manufacturer`, `[stats]`, `[customization]`, `[[damage_parts]]`, `[engine_sound]`, `[car_sound]`; unknown keys refused | G (schema), MW (checks) | `libs/blackbox-modding/src/content/car.rs`, `stats.rs`, `customization.rs`, `engine_sound.rs`, `crates/nfsmw/src/modding/content/cars.rs` |
| Stats | `Stats` (40 keys, absolute SI values; `Stats::default()` is the engine's generic car) converts to `blackbox_vehicle::stats::CarStats`; `car::mods::stats(folder)` reads it; `CarPhysics::from_stats(stats, bounds)` and `physics_from_stats(stats, package, bounds)` build the physics, `PerformanceCatalog::from_stats` the shop's upgrade list, `RatingBook::for_stats` its bars, `ai_vehicle_from_stats` the AI's speed limit | G (schema, builder), MW (wired) | `libs/blackbox-modding/src/content/stats.rs`, `libs/blackbox-vehicle/src/stats/`, `crates/nfsmw-data/src/car/physics/from_stats.rs` |
| Physics and collision | `PhysicsData::car(folder)` builds a mod car from its `[stats]` and its own `BOUNDS.BIN` (`mod_car_collision`); nothing is read from a vehicle record | MW | `crates/nfsmw-data/src/car/physics/{data,mod_car,bounds}.rs` |
| Wheel layout | `WheelSetup::for_car` takes a mod car's hubs from its `[wheels]` table when it has one (`WheelSetup::from_layout`, [car-stats.md](car-stats.md#where-the-wheels-are-drawn-wheels)), else from its stats (`WheelSetup::from_stats`): plus and minus half the wheelbase, track and tyre size from `[stats]` | MW | `crates/nfsmw-data/src/car/ecar.rs` |
| Engine sound | `[engine_sound]` (`idle_rpm`, `redline_rpm`, `layers`): looping WAVs mixed by engine speed; a car without it plays the built-in engine (`blackbox_carsound::synth`). `car::mods::engine_sound(folder)` reads it, `audio/car/own_engine.rs` plays it | G (schema), MW (played) | `libs/blackbox-modding/src/content/engine_sound.rs`, `crates/nfsmw-data/src/car/mods/engine.rs` |
| Car sound | `[car_sound]` (`shift`, `shift_up`, `shift_down`, `turbo_whine`, `turbo_blowoff`, `backfire`, `pops`): lists of up to 16 WAV files, `MAX_EFFECT_FILES`; a list left out is silence; `shift_clunk = false` also silences the game's class-wide gear clunk for a car with no `shift`; an `[engine_sound]` layer with `gain` curves may have a `rate` curve (its playback rate by an input). `car::mods::car_effects(folder)` reads it, `audio/car/own_sfx.rs` plays it | G (schema), MW (played) | `libs/blackbox-modding/src/content/engine_sound.rs` (`CarEffects`), `crates/nfsmw-data/src/car/mods/engine.rs`, `crates/nfsmw/src/modding/content/cars.rs` |
| Sound set | `sound::generic_car_sound(db, name)`: the sound set of a car with no `pvehicle` record, from `engineaudio/default`, `shiftpattern/default`, no turbo, the collision links of `pvehicle/cars` and the shared skid and nitrous banks; `Audio::sound_set` picks it for every mod car | MW | `crates/nfsmw-data/src/sound/car.rs`, `crates/nfsmw/src/audio/mod.rs` |
| Customization | `[customization]` (`paints`, `default_paint`, `rim_paints`, `tints`, `rims`, `spoilers`, `roof_scoops`, `vinyls`, `vinyl_colors`, `liveries`, `default_livery`): `car::mods::custom::of(folder)` reads it; the garage catalogue (`custom::catalog`, `custom::livery`) and the assembled model (`custom::apply`) come from it and the car's own `GEOMETRY.BIN`, `VINYLS.BIN` and `TEXTURES.BIN` | G (schema), MW (shop) | `libs/blackbox-modding/src/content/customization.rs`, `crates/nfsmw-data/src/car/mods/custom/` |
| Car parts | the body, wheel and driver part of each mod car, made from the car's own solids `<ID>_KIT00_BODY_A..D`, `<ID>_KIT00_FRONT_TIRE_A..D` and `<ID>_KIT00_DRIVER_A..D` (the last optional) and added to the parts database (`PartsDb::push_parts`); no other slot is filled | G (`push_part`), MW | `libs/blackbox-carparts/src/parts/push.rs`, `crates/nfsmw-data/src/car/mods/parts.rs` |
| Brand | `car::mods::manufacturer(folder)`: the brand a mod gave | MW | `crates/nfsmw/src/scenes/world/garage.rs` (`catalog`) |
| Register cars | `car::mods::register(stock: &[CarTypeInfo], specs: &[CarSpec]) -> (Vec<ModCar>, Vec<Rejected>)` | MW | `crates/nfsmw-data/src/car/mods.rs` |
| Car table | `CarTables::load` appends `car::mods::car_types()` after the stock types (the entry of a mod car is made by `car_type_of`, from its folder), so `car::load`, the garage's car list and cop loading find the car by folder. The parts database of those tables also holds the mod cars' parts: the extended tables are built when a load first sees a new set of published cars and shared after | MW | `crates/nfsmw-data/src/car/tables.rs` |
| Cop rows | `pursuit::heat_row` adds `car::mods::add_cop_rows` for the level, one record per car whose unit has an entry for it (count and chance of the entry); `car::mods::{cop_unit, cop_kind, cop_cars}` give the unit (class and entries) | MW | `crates/nfsmw-data/src/pursuit.rs`, `car/mods/cop.rs` |
| Publish | `car::mods::publish(cars)` from `open_install` (once for the start-up cars, again for the cars a mod adds through `content.rescan()`); a folder already published is not added twice | MW | `crates/nfsmw/src/modding/content/{mod,cars}.rs`, `modding/open.rs` |
| Garage name and role | `car::mods::display_name(folder)`, `car::mods::is_player_car(folder)`; a cop-only car is usage type 1 and `Category::Police` | MW | `crates/nfsmw-data/src/car/mods.rs`; called by `crates/nfsmw/src/scenes/world/garage.rs` (`catalog`) |
| Planned: code form | `content.cars().register(CarDef { .. }) -> Result<CarId>`, paint (`CARSKIN`), part models, `traffic` role | G | not built |

### Music

Works now in NFS:MW. A mod ships its tracks in a `music/` folder, as audio files (`mp3`, `ogg`, `flac`, `wav`) and/or
playlists (`.m3u` or `.m3u8`, whose entries are relative to the playlist). `mod.toml` names the contexts:

```toml
[music]
contexts = ["menu", "free_roam"]   # default: both
always = true                      # play even when the custom music setting is off
```

- The tracks join the custom music library (`audio/radio/custom`), after the player's own folders. Without `always`
  they are read only when custom music is on.
- `menu` is the front-end list and `free_roam` the in-game list. Any other name is logged and skipped. `race` and
  `pursuit` wait until the radio has those contexts.
- A `[music]` table with no `music/` folder adds nothing, and the log says so.

| Entry | Signature sketch | Scope | Implemented in |
|---|---|---|---|
| Music source | `[music]` in `mod.toml` (`contexts`, `always`) and the `music/` folder | G (schema), MW (contexts) | `libs/blackbox-modding/src/content/music.rs`, `crates/nfsmw/src/modding/content/music.rs`, `audio/radio/custom/library.rs` (`Library::build_with`) |
| Planned: track record | `content.music().register(TrackDef { key, title, artist, file, categories, station, loop_points })`, stations, `[track]` files | G | not built |
| Replace track | a file in `files/` with the install's path replaces the original (overlay), no registration | G | VFS overlay |
| Planned: speech line | `content.speech().register(key, file)` for `audio.speak` | G | not built |

### Manufacturers

Works now in NFS:MW. A mod gives a brand a logo, which the car screens (the car select of the garage's Add New Car, the car lot,
the custom race and Quick Play, and the Showcase) draw in the maker's place of a car of that brand. The game's own cars have
a logo per manufacturer; a mod car's brand is a string (`manufacturer` in its manifest), so a mod gives the brands of its cars
a logo this way, and may replace the logo of a brand the game already has.

```
mods/example.pack/
  manufacturers/bentley.toml          the manifest; the id is the file name
  files/UI/MAKERS/BENTLEY.PNG         the logo
```

```toml
name = "Bentley"                      # the brand, 1 to 32 characters; compared ignoring case, spaces and punctuation
logo = "UI/MAKERS/BENTLEY.PNG"        # a PNG under files/: letters, digits, _ - . / and ending in .png; any colour type, up to 2048 on a side
override = false                      # true: replace the logo of a brand that already has one; default false
```

- The logo is drawn in a slot twice as wide as it is tall (about 150 by 75), shrunk to fit and centred, never stretched: draw
  it light on a transparent background, as the game's own logos are.
- **Which logo a brand gets**, in order: a mod's logo with `override = true`; the game's own logo of a manufacturer of that name
  (`Subaru`, `Aston Martin`, `General Motors`; a brand that starts with one, such as `Mercedes-Benz`, is that manufacturer); a mod's
  logo with `override = false`; the brand written out as text. So a mod that only adds brands never replaces a logo the game has,
  and a mod that wants the old brands to look different sets `override = true`.
- The same brand with the same `override` is taken from the first mod that declares it. A manifest with a bad key, a logo that is
  missing or not a PNG is refused and logged, and the others still load. A mod that adds manufacturers later is read at its rescan.

| Entry | Signature sketch | Scope | Implemented in |
|---|---|---|---|
| Manufacturers | `manufacturers/<id>.toml` (`name`, `logo`, `override`) | G (schema), MW (drawn) | `libs/blackbox-modding/src/content/manufacturer.rs`, `crates/nfsmw/src/modding/content/manufacturers.rs`, `frontend/car_title.rs` |

### Racer names

A mod adds names to the pool the computer-driven racers of every race draw from (the game's own list has 150):

```toml
[racers]
names = ["MAVERICK", "DEE DEE"]
```

- A name is 1 to 24 characters with no control character and is listed once. A bad table adds nothing and the log says why.
- A race never gives one name twice, and never the player's. Names that equal one of the game's, ignoring case, are dropped.

| Entry | Signature sketch | Scope | Implemented in |
|---|---|---|---|
| Racer names | `[racers] names` in `mod.toml` | G (schema), MW (pool) | `libs/blackbox-modding/src/content/racers.rs`, `crates/nfsmw/src/modding/content/racers.rs`, `scenes/world/race/names.rs` |

[How the original names racers](../provenance/race-names.md).

### Fonts

| Entry | Signature sketch | Scope | Implemented in |
|---|---|---|---|
| Register font | `content.fonts().register(FontDef { key, file, replaces: Option<&str> }) -> Result<FontId>`; `replaces = "default"` swaps the game's font after start-up | G | `libs/blackbox-text`, `frontend/` and `hud/` font lookups |
| Use in a mod page | `menus.page_add(..)` with `font: FontId` | G | `frontend/widgets.rs` |

### Scenery and map packs

#### Scenery pack manifest (built)

A mod adds objects with a model file and one manifest, at the mod root next to `cars/` and `music/`:

```
mods/example.pack/
  files/SCENERY/MONUMENT.BUN     the models: a section file in the install's formats (bChunk solids and textures)
  scenery/monument.toml          the placements
```

```toml
# scenery/monument.toml
[pack]
file = "SCENERY/MONUMENT.BUN"    # under this mod's files/, ends in .BUN; no .., no drive
section = 1100                   # a free number: K 1100-1199, N 1400-1499, U 2100-2199, W 2300-2399, 2700 and up;
                                 # the last two digits 0-39 or 80-99
anchor = [1200.0, -900.0, 40.0, 180.0]   # render space: centre x, y, z and radius, metres
source_game = "nfsmw"            # optional; the running game when left out

[[place]]
model = "MON_ARCH_A"             # a solid of the pack file
pos = [1200.0, -900.0, 0.0]      # render space, metres
yaw = 90.0                       # degrees about up, or rotation = [[...], [...], [...]] (not both)
scale = 1.0                      # above 0, at most 4
coarse = "MON_ARCH_C"            # optional low-detail model of the player view
mirror = "MON_ARCH_B"            # optional model of the mirror and reflections
collision = "static"             # "none" (default), "static" (the pack's collision), "prop" (refused for now)
boost = false                    # optional
exclude = 0x60                   # optional raw flags; the game's default is 0x60
draw_m = 900.0                   # checked; not applied yet
lod_m = 300.0                    # checked, must not exceed draw_m; not applied yet
```

- The file name is the pack key. A `key` in `[pack]` is optional and must equal it.
- Only `route = "tile"` (the default) exists. Routes B and C are not in this delivery.
- The pack loads while the camera is within 150 m of its anchor, or while an interior holds it. It never displaces an
  install tile.
- A pack that fails a check is logged with its reason and left out: a bad manifest, a section number that is taken or
  not free, a model file that does not parse, or a placement that names a model not in the file.
- Status and open items: [mod-world-content.md section 9](../specs/mod-world-content.md#9-port-status-b15-2026-10-16).

#### Planned code form (not built)

The city is streamed from section files through `libs/blackbox-streaming` and `libs/blackbox-scenery`. A
**scenery pack** is a set of section files in the overlay plus an index entry; a **map pack** is a pack that also
adds roads, zones and a spawn area. The first delivery supports:

- replacing existing section files (works through the overlay with no code); and
- adding new scenery **instances** and **props** to an existing section through a declarative file, so a mod can
  place objects without editing the section binary.

Adding a whole new district (new section ids, road network, collision, nav data) is specified but not
promised for the first delivery, because the index and the road network are read from install files as a unit.

| Entry | Signature sketch | Scope | Implemented in |
|---|---|---|---|
| Register pack | `content.scenery().register(PackDef { key, sections: &[&str], index: Option<&str>, source_game }) -> Result<PackId>` | G | `libs/blackbox-streaming`, `libs/blackbox-scenery`, `scenes/world/residency.rs`, `zone.rs` |
| Place instance | `content.scenery().add_instances(PackId, &[InstanceDef { model: &str, pos: Vec3, rotation: Vec3 }])` | G | `libs/blackbox-scenery` instance lists, `scenes/world/props.rs` |
| Place at runtime | `world.spawn_prop(model: &str, pos: Vec3, yaw: f32) -> Result<PropId>`, `despawn_prop(PropId)` **W** | G | `scenes/world/props.rs`, `cargo.rs` |
| Remove original | `content.scenery().hide(model_or_id: &str)` hides an install instance | G | `libs/blackbox-scenery` |
| Road data | `content.roads().register(...)` adds a road network piece; **reserved** | G | `libs/blackbox-roads` |

Cross-game: scenery from another Black Box game works only where `libs/blackbox-solid`, `blackbox-tpk` and
`blackbox-scenery` have a layout table for that game's version. The mod, or a tool, converts anything else.

## Interiors

### Interior manifest (built)

An interior is an entry zone, a prompt and a place. The place is a scenery pack of the same mod, so a mod that has an
interior has two manifests:

```toml
# interiors/hideout.toml
[interior]
label = "Hideout"
prompt = "Press Enter to go in"          # optional
zone = { shape = "sphere", pos = [1520.0, -840.0, 12.0], radius = 6.0 }   # or shape = "box", half = [2.0, 2.0, 2.0], yaw = 0.0
pack = "hideout"                         # scenery/hideout.toml of this mod: the place
spawn = { pos = [3010.0, 4020.0, 0.0], heading = 90.0 }   # render space, metres; heading in degrees
exit = { shape = "sphere", pos = [3016.0, 4020.0, 0.0], radius = 3.0, prompt = "Press Enter to leave" }
time = { hour = 22.0, freeze = true }    # optional; the clock returns to its entry hour on exit
suspend = ["traffic", "cops", "pursuit"] # optional; default is all three
kind = "area"                            # optional; the only kind in this delivery
```

- The file name is the key. A `key` in `[interior]` is optional and must equal it.
- `label`, `prompt` and the exit's `prompt` may be message ids of the mod's `lang/` catalogue; they are then shown in the
  player's language ([translations.md](translations.md)).
- Inside: traffic count and cop share go to 0 for the names listed, and a chase stops when `cops` or `pursuit` is listed.
  The car, the traffic and the clock come back on exit. A chase is not resumed.
- `lights` in `time` is checked and logged, but not applied yet: the headlights follow the hour.
- `kind` `"page"`, `"shop"`, `"safehouse"` and `"scene"` are refused in this delivery.
- Status, and what is open (the fade overlay, `world.add_box_trigger`, the `interior check` command, the quarantine exit):
  [mod-world-content.md section 9](../specs/mod-world-content.md#9-port-status-b15-2026-10-16).

### Planned interior form (not built)

An interior is a place the player enters from the open world. In NFS:MW the three kinds today are auto shops,
safe houses and race starts (`ZoneKind` in `scenes/world/entry_zones.rs`); each is an entry sphere with a map
blip and a front-end screen behind it. A mod adds places like these.

```toml
# content/interiors/hideout.toml
[interior]
key = "hideout"
label = "Hideout"
pos = [1520.0, -840.0, 12.0]         # render space, metres
radius = 6.0
blip_style = "example.pack:hideout"  # from bb.map
kind = "page"                        # "page" = a mod page (menus); "shop" or "safehouse" = reuse that room; "scene" = custom
page = "hideout-menu"                # content/menus/hideout-menu.toml
```

| Entry | Signature sketch | Scope | Implemented in |
|---|---|---|---|
| Register interior | `content.interiors().register(InteriorDef { key, label, pos: Vec3, radius: f32, blip: Option<StyleId>, kind: InteriorKind, on_enter: Option<CallbackId>, on_exit: Option<CallbackId> }) -> Result<InteriorId>` | G | `scenes/world/entry_zones.rs` (`Zone`, `ZoneKind` gets `Mod(u16)`), `scenes/world/hud.rs` (`interior_blip`), `frontend/world_map/` |
| Kind `Page` | opens a mod page on entry (v1) | G | `frontend/widget_menu.rs`, `flow.rs` |
| Kind `Room` | reuse an existing room scene (shop, safe house, car lot) at the new location; the room's contents come from the game (shop) or from the mod's callbacks | G | `frontend/room.rs`, `frontend/scene.rs`, `scenes/world/garage.rs` |
| Kind `Scene` | **reserved for a later minor**: a custom interior package (solids, textures, a camera path and a controller) drawn by the room scene | G | `frontend/room.rs`, `libs/blackbox-solid`, `libs/blackbox-tpk` |
| Enter or leave from code | `content.interiors().enter(InteriorId)`, `exit()` | G | `scenes/world/entry.rs` |
| Is inside | `content.interiors().current() -> Option<InteriorId>` | G | same |
| Prompt text | `InteriorDef.prompt` overrides "Press X to enter" | G | `hud/interact_prompt.rs` |
| Events | `ZoneEntered`, `ZoneExited` carry the interior's zone; `InteriorEntered`, `InteriorExited` fire when the room opens or closes | G | `scenes/world/entry.rs` |

The NFS:MW shop and safe-house **rules** (what a safe house offers, what a shop sells, prices) are career
logic and live in `nfsmw.career` ([api-nfsmw.md](api-nfsmw.md#safe-houses-and-shops)); the generic layer only
creates the place, the blip and the entry.

## Settings and storage (`bb.config`)

| Entry | Signature sketch | Scope | Implemented in |
|---|---|---|---|
| Get | `config.get(key: &str) -> Option<Value>` from `mods/<id>/config.toml`, with defaults from `[config]` in `mod.toml` | G | `modding/config.rs` (new), `toml` |
| Set | `config.set(key: &str, v: Value)`; writes through to disk at exit and on `config.save()` | G | same |
| Save | `config.save() -> Result<()>` | G | same |
| Default | `[config.defaults]` in `mod.toml` is the first-run value and what the menu reset restores | G | same |
| Per-save data | `config.slot_get(slot: SlotId) -> Option<Vec<u8>>`, `slot_set(slot, bytes)` next to the save slot, so career mods keep state per save | G | `libs/save-store` (`SaveStore`, `check_slot_name`), `saves.rs` |
| Setting rows | a `[menu]` block in `mod.toml` creates option rows bound to config keys without code | G | `frontend/options_rows.rs` |

## Open items

- **Car overrides.** A mod car is a car of its own ([Cars](#cars)): its physics come from its `[stats]` table and its
  `BOUNDS.BIN`, not from a record of the attribute database (`pvehicle` and its records, read through
  `libs/blackbox-attrib`, which is read-only in the port). Overrides of a stock car's fields by name, and a new record in
  that database, are not offered and not specified yet [unconfirmed]. A mod that wants a changed version of a stock car
  adds a car of its own with its own numbers. `docs/specs/car-assembly.md` and the attribute specs are the source when
  overrides are written.
- **Index merge** for adding districts (above).
- **Archive layers** (a single packed file per mod) and signatures.
- **Chunk-level patches** for merging into `.BUN` files.
- **Custom interior scenes** (kind `Scene`).
