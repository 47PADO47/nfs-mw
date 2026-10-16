# Modding

Native plugin mods for the Rust rewrite of NFS: Most Wanted and, later, other Black Box games. The decision and
its reasons are in [ADR 0005](../decisions/0005-native-mod-plugins.md). This folder is the catalogue of what a
mod can do.

Status: **design only**. Nothing in the catalogue exists in code yet; every "Implemented in" cell names the
game module that will implement the entry (a path that exists today) or says "new" when the module is new.

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
  files/          VFS overlay, mirrors the install layout
  content/        declarative registrations (TOML)
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
