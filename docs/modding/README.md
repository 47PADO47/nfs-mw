# Modding

Native plugin mods for the Rust rewrite of NFS: Most Wanted and, later, other Black Box games. The decision and
its reasons are in [ADR 0005](../decisions/0005-native-mod-plugins.md). This folder is the catalogue of what a
mod can do.

Status: **partly built**. `bb.commands`, `bb.input`, `bb.time`, `bb.map` and `bb.menus` work in NFS:MW, and so does
`nfsmw.pursuit` (heat, the chase, busted, the `nfsmw.*` events, and the cop tables: a wave a mod sets applies to the next
chase). The game interaction interfaces are partly built: `bb.vehicles` (the player's car: state, teleport, repair),
`bb.hud` (a centred message), `bb.camera` (read and the gameplay views) and `bb.world` (track, district, rain, traffic)
work; `bb.audio` is offered but every play is unsupported until sound names are looked up; traffic spawn at a point is
unsupported. The status of each row is in [api-world.md](api-world.md#game-interaction). `nfsmw.career`
and `nfsmw.races` are offered with no state, so a mod can detect them. Content mods also work: cars and music files declared in a mod's `cars/` folder and `[music]` table (see
[Using mods](#using-mods) and [api-content.md](api-content.md#cars)). The rest of the catalogue is design only: every
"Implemented in" cell names the game module that will implement the entry (a path that exists today) or says "new"
when the module is new.

## Files

| File | Covers |
|---|---|
| [api.md](api.md) | How to read the catalogue, shared types, lifecycle, events, commands, hotkeys and abilities, time of day |
| [api-world.md](api-world.md) | Map blips and the world map, menus, game interaction (vehicles, player, HUD, audio, camera, world) |
| [api-content.md](api-content.md) | The VFS overlay, content registries (cars, cop waves, music, fonts, scenery, interiors), settings storage |
| [api-nfsmw.md](api-nfsmw.md) | The NFS:MW-only extension: career, heat and pursuit, cop tables, race events |

## Two layers

| Layer | Crate | Interface ids | Works in |
|---|---|---|---|
| Generic | `libs/blackbox-modding` | `bb.*` | every Black Box game that offers the interface |
| NFS:MW | `crates/nfsmw-modding` | `nfsmw.*` | NFS:MW only |

A mod that uses only `bb.*` interfaces lists `supported = ["*"]` in its manifest and can run in NFS:MW and in
NFS Underground 2 once that game offers the same interfaces. A game offers the `bb.*` interfaces it can
implement; a mod asks for them with `[requires]` or `[optional]` and must cope with an absent optional one.

## Folder layout of a mod

```
mods/<id>/
  mod.toml        manifest (required)
  <library>       .dll / .so (optional: a content-only mod has none)
  files/          VFS overlay, mirrors the install layout (CARS/MYCAR/..., MUSIC/...)
  cars/           one <id>.toml per car the mod adds (clone, role, heat); see api-content.md#cars
  music/          audio files and .m3u playlists; the contexts are in [music] of mod.toml
  content/        other declarative registrations (TOML), planned
  config.toml     the mod's own settings, managed through bb.config
```

`mods/` is `<config dir>/mods/`. The manifest fields, discovery, ordering and failure rules are in the ADR
(section 3).

## Minimal manifest

```toml
[mod]
id = "example.hello"
name = "Hello"
version = "0.1.0"
[api]
blackbox_modding = "1.0"
[games]
supported = ["*"]
[library]
windows = "hello.dll"
linux   = "libhello.so"
[requires.interfaces]
"bb.commands" = "1.0"
```

## What a mod can do (capability map)

| Wanted capability | Interface | Catalogue |
|---|---|---|
| Run and register console commands | `bb.commands` | [api.md](api.md#commands) |
| Read and answer events | `bb.events` | [api.md](api.md#events) |
| Hotkeys and abilities | `bb.input`, `bb.abilities` | [api.md](api.md#hotkeys-and-abilities) |
| Day and time cycle | `bb.time` | [api.md](api.md#time-of-day) |
| Map blips, world-map markers | `bb.map` | [api-world.md](api-world.md#map-blips-and-the-world-map) |
| Extend menus | `bb.menus` | [api-world.md](api-world.md#menus) |
| Interact with the game (player car, spawn traffic and cops, HUD message, sound, camera) | `bb.vehicles`, `bb.hud`, `bb.audio`, `bb.camera`, `bb.world` | [api-world.md](api-world.md#game-interaction) |
| Add cars (player and cop) | `bb.content` | [api-content.md](api-content.md#cars) |
| Add music, fonts, scenery and map packs | `bb.content`, VFS overlay | [api-content.md](api-content.md) |
| Add interiors | `bb.content`, `bb.map` | [api-content.md](api-content.md#interiors) |
| Career, heat, pursuit, cop tables, races | `nfsmw.career`, `nfsmw.pursuit`, `nfsmw.races` | [api-nfsmw.md](api-nfsmw.md) |

## Rules every call follows

Summary of ADR sections 1 and 4; the details are in [api.md](api.md#conventions).

- Main thread only, inside a mod callback. Elsewhere the call returns an error.
- Strings are UTF-8 slices valid for the call only. Nothing is freed across the boundary.
- Objects are generation-checked 64-bit handles; a stale handle is an error, not a crash.
- Units: metres, seconds, radians, render space (x forward, y left, z up).
- A panic in mod code is caught by the SDK and counts as a fault; five faults quarantine the mod.
- Native mods are fully trusted code. Install only mods you trust. `--no-mods` turns all of them off.

## Using mods

Mods are native libraries that run with your privileges. Install only mods you trust. Nothing in the game
downloads or fetches a mod.

- **Where:** one folder per mod under the config folder of the game: `%APPDATA%\nfsmw\config\mods\<id>\` on
  Windows, `~/.config/nfsmw/mods/<id>/` on Linux. The folder holds `mod.toml`, the library and `files/` (see
  [Folder layout](#folder-layout-of-a-mod)). `--mods-dir <path>` adds a second folder to search.
- **Which load:** the `mods` setting is `all` by default (every mod in the folders), `allowlist` (only the ids in
  `mods_allowlist`, a list in `config.toml`) or `off`. Set it in `config.toml` (`mods = "allowlist"`) or in the
  environment (`NFSMW_MODS=off`). `--no-mods` loads none, native mods included; use it first when reporting a bug.
- **Developer runs:** `--mod-dev` loads copies of the libraries from the local data folder, so a rebuilt library can
  replace the original while the game is running. Reloading a mod during a run is not in the console yet.
- **Checking:** the start-up log names each loaded mod, its version and the SHA-256 of its library, and each refused
  mod with the reason. The F12 console command `mods` lists the same: each loaded mod with its state (`loaded`, or
  `quarantined` after five faults) and each refused mod.
- **Files:** each mod's `files/` folder is added to the game's install as a layer. A file in it replaces the file of
  the same path in the install or in a lower layer, and a new path adds one. Layers order by `[content] priority`
  (higher wins), then by load order (a later mod wins a tie). Mods do not write to the
  install.
- **Content:** a mod's `cars/<id>.toml` files add cars, each a copy of a stock car with the mod's own `files/CARS/<ID>/`
  folder, as player cars, cop cars, or both. A mod's `music/` folder joins the custom music library in the contexts
  its `[music]` table names, and plays even with the custom music setting off when `always = true`. A car or a track
  that fails a check is logged with its reason and left out. The cars are registered once, at start-up, and a change
  needs a restart. The garage does not yet use a mod's display name or hide its cop-only cars.
- **Now:** a mod gets the frame events (`FRAME_START` in the start of the frame, `SCENE_TICK` after the scene's
  update, and `MOD_UNLOADING` at exit), and its `files/` layer. It also gets the game's interfaces. A mod that requires an
  interface the game does not offer is refused with its reason. In NFS:MW the `nfsmw.*` events of the chase (pursuit
  started and ended, heat changed, busted, escaped, infraction recorded) go out in the same step, just before
  `SCENE_TICK`; see
  [api-nfsmw.md](api-nfsmw.md#events-01).
  - `bb.commands`: `register` adds a console command. It shows in `help` under the mod's id, runs after the built-in
    commands and before the scene's, and is removed when the mod unloads. Tab completion does not list it yet.
    `run` queues a console line for the next frame; its output goes to the F12 log, not back to the mod.
  - `bb.input`: `register_hotkey` takes a keyboard key as its default binding (`key:F5`, the text the `bind` command
    takes). The callback runs in the frame the key goes down, unless the console has the keyboard. The hotkey is not
    in `keys` or the remap screen yet, and an empty binding leaves it unbound. Abilities (`bb.abilities`) are not
    offered.
  - `bb.time`: `get`, `set_hour` (wrapped into 0 to 24), `set_day_length` (a rate clamped to 10 to 3600 real seconds
    per hour; `0` holds the hour) and `freeze`. The clock is the world's day clock, and a menu has none, so a call
    there is `not ready`. A mod's read at load is `not ready` too: the mods start before the scene exists, so the
    clock is first copied in at the first frame. A mod reads it on its first event, as `examples/mods/hello` does.
  - `bb.menus`: `register` adds a row to a Mods page under Options, which is listed only while a mod has rows. A row
    is a toggle, a slider, a list or a button. A change is saved under `[mod_values."<id>"]` in the config file and
    calls the mod back. Pages of their own and the SDK wrapper are not offered yet.
  - `bb.map`: `add`, `update` and `remove` blips for the cop, helicopter, race (sprint, drag, circuit, knockout,
    tollbooth, speed trap), auto shop, car lot, safe house, racer and checkpoint styles. The blips show on the minimap and the world
    map. The icon points forward, as `bb.map` has no heading.
