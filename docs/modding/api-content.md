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

### Cars

Works now in NFS:MW. A mod adds a car with its files and one manifest. The car id is the manifest's file name:

```
mods/example.pack/
  mod.toml
  files/CARS/FASTCOUPE/GEOMETRY.BIN   the body, in the install's format
  files/CARS/FASTCOUPE/TEXTURES.BIN   its textures
  cars/fastcoupe.toml                 the manifest
```

```toml
# cars/fastcoupe.toml
clone = "CAMARO"                # a stock car: the new car keeps its record, parts, physics and sound
role = "both"                   # player, cop or both (default: player)
heat = [2, 3, 4]                # cop only: heat levels whose waves may use it (left out: every level)
name = "Fast Coupe"             # display name (optional; default: the clone's)
manufacturer = "Acme"           # brand (optional; default: the clone's)
```

Rules:

- The car id is the file name in upper case (`FASTCOUPE`). Its geometry must be in the mod's own `files/` folder, not
  only in the install.
- `clone` names a stock car (its folder or its type name), or a car an earlier mod added in this run.
- A folder that is already a car (stock, or another mod's) is refused. Heat levels run from 1 to 10.
- A key the game does not know refuses the car. So does an `[override]` table: physics and attribute overrides are not
  in the first delivery (see open items).
- Each refused car is logged with its mod and reason. The game goes on without that car.
- Registration runs once in `open_install`, before any scene is built, and the result is sealed for the run. A changed
  car needs a restart.
- A cop car gets one row in each heat wave it names: 2 cars at 10 percent (`COP_COUNT`, `COP_CHANCE` in
  `car/mods.rs`). Its physics and AI are the clone's.

Known gaps: the garage lists every car, cop-only ones included, and names each from the language table. It needs a
hook to use `display_name` and `is_player_car` (`car/mods.rs`); until then a mod's display name and role are not shown.
Paint, part models and a code form (`content.cars().register(..)`) are not built yet.

| Entry | Signature sketch | Scope | Implemented in |
|---|---|---|---|
| Car manifest | `cars/<id>.toml` with `clone`, `role`, `heat`, `name`, `manufacturer`; unknown keys refused | G (schema), MW (checks) | `libs/blackbox-modding/src/content/car.rs`, `crates/nfsmw/src/modding/content/cars.rs` |
| Register cars | `car::mods::register(stock: &[CarTypeInfo], specs: &[CarSpec]) -> (Vec<ModCar>, Vec<Rejected>)` | MW | `crates/nfsmw-data/src/car/mods.rs` |
| Car table | `CarTables::load` appends `car::mods::car_types()` after the stock types, so `car::load`, the garage's car list and cop loading find the car by folder | MW | `crates/nfsmw-data/src/car/tables.rs` |
| Physics name | `PhysicsData::car(name)` reads a mod car's clone record through `car::mods::physics_name` | MW | `crates/nfsmw-data/src/car/physics/data.rs` |
| Cop rows | `pursuit::heat_row` adds `car::mods::add_cop_rows` for the level | MW | `crates/nfsmw-data/src/pursuit.rs` |
| Seal | `car::mods::seal(cars)` once, from `open_install` | MW | `crates/nfsmw/src/modding/content/mod.rs`, `modding/open.rs` |
| Garage name and role | `car::mods::display_name(folder)`, `car::mods::is_player_car(folder)` | MW | `crates/nfsmw-data/src/car/mods.rs`; **not called yet** by `crates/nfsmw/src/scenes/world/garage.rs` |
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

### Fonts

| Entry | Signature sketch | Scope | Implemented in |
|---|---|---|---|
| Register font | `content.fonts().register(FontDef { key, file, replaces: Option<&str> }) -> Result<FontId>`; `replaces = "default"` swaps the game's font after start-up | G | `libs/blackbox-text`, `frontend/` and `hud/` font lookups |
| Use in a mod page | `menus.page_add(..)` with `font: FontId` | G | `frontend/widgets.rs` |

### Scenery and map packs

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

- **Car overrides.** The first delivery clones a stock car type and changes only its display name and brand
  ([Cars](#cars)). A mod car has no record of its own: its physics, engine, transmission and tyres are the clone's,
  because the attribute database (`pvehicle` and its records, read through `libs/blackbox-attrib`) is read-only in
  the port. Overrides by field name, and a new record in the database, are not specified yet [unconfirmed].
  `docs/specs/car-assembly.md` and the attribute specs are the source when this is written.
- **Garage hook.** The garage's car list (`scenes/world/garage.rs`) must call the mod functions in
  `car/mods.rs` for display names and for cop-only cars. Until it does, mod cars show under the clone's name.
- **Index merge** for adding districts (above).
- **Archive layers** (a single packed file per mod) and signatures.
- **Chunk-level patches** for merging into `.BUN` files.
- **Custom interior scenes** (kind `Scene`).
