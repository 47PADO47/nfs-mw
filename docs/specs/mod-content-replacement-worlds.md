# Spec: worlds, alternative maps and world patches

Part of [mod-content-replacement.md](mod-content-replacement.md); decision [ADR 0015](../decisions/0015-mod-content-replacement.md).
Base feature: [second-world.md](second-world.md).

## 1. Three things

| What | Files | Meaning |
|---|---|---|
| **World** | `worlds/<id>.toml` + `worlds/<id>/*.toml` packs | a place of its own (the second-world feature) |
| **Alternative map** | a world with `base = "rockport"` | the game's own city, plus this world's packs, start and environment; chosen like a world |
| **World patch** | `world-patches/<id>.toml` + `world-patches/<id>/*.toml` packs | a change to a world another mod (or the city) provides |

## 2. World manifest (built: parser and writer)

`[world] name, start = { pos, heading }` as before, plus `base`, `environment = { hour, freeze, rain }` and `[[world.grid]]` slots
(`libs/blackbox-modding/src/content/world.rs`). `WorldManifest::to_toml` is the one writer: the converters (RIDE 6 tracks, Fairhaven)
call it, so the schema has one definition. `environment.hour` is `0 <= h < 24`, `freeze` needs an hour, `rain` is 0 to 1.

## 3. Alternative maps

The city streams from `L2RA.BUN`: tiles, collision grid, roads, zones, races, blips, traffic. An alternative map keeps all of it and
adds the packs of `worlds/<id>/`, the start and the environment. It does not replace or delete tiles of the city: that needs the
chunk-level merge (`api-content.md`, open items). Removing a city object is `hide` (reserved). Packs of the city's mods (`scenery/`)
stay on. [ours]

## 4. Patches: ownership, priority, conflicts (built: pure functions)

- `target` names a world id or `rockport`. Patches apply from the lowest `priority` up, then load order, then id (`order`); the **last
  to set a thing wins**: name, start, environment replace whole; `grid` appends (or `grid_replace = true` replaces).
- Packs: start from the target's keys; each patch in order removes `remove_packs`, then adds its folder's packs; a key already there
  is refused unless listed in `replace_packs`, in which case the new pack takes the old one's place. A section number that is taken is
  refused by the host's existing pack check (`free_section`, `taken`).
- Every clash becomes a line of `PatchedWorld::report` / `PackPlan::report`: `mod B: patch x sets the name that mod A set; this one wins`.
- A world's own mod is the owner; a patch of a world cannot remove the world, rename its id, or change `base`. [ours]

## 5. Surfaces, markers, grid, sky (design)

- **Surfaces**: a world's collision carries the game's simsurface keys per triangle ([collision.md](../formats/collision.md)); a table
  mapping a mod's names to those keys is not needed while the writer takes keys. Reserved.
- **Start grid**: `[[world.grid]]` is checked and kept (`WorldManifest::grid`); the host does not place extra players or a race grid
  from it yet (co-op and split screen in a second world use the start). Reserved, documented like `draw_m` in scenery packs.
- **Laps and markers**: a second world has no races; lap timing stays the mod's (`bb.hud`). Markers are reserved.
- **Sky, time, weather**: the host hooks that exist are the clock (`Scene::set_environment`, `set_time_held`) and rain
  (`Scene::set_rain`). `environment` maps to exactly those, applied when the world is built.

## 6. Host status

Built and unit tested: parsers, writer, merge and pack plan (`world.rs`, `world_patch_tests.rs`); `worlds::list_in` reads the patches of
every loaded mod and gives each world its patched manifest and its ordered patches; `worlds::packs` applies the pack plan (remove, replace,
add; a taken section number goes to the first pack); `WorldScene::load_in` plays an alternative map in the city (`standalone` and
`alternative` in `open.rs`: the city's index, gameplay data, roads and `scenery/` packs, plus the map's packs) and applies `environment`
(`apply_world_environment`: the clock through `configure_environment`, held when asked, the rain through `Rain::configure`).
`bb.content` capabilities `WORLD_PATCHES` and `ALT_MAPS` are set. Patches of the plain city (`target = "rockport"`) are not read: the
city has `scenery/` packs for adding objects.

Not built: placing players on `grid` slots, markers, surfaces table, removal of city tiles; the clock is set when the scene is built, so
an option the frontend applies later (a saved time of day) can override it [not checked].
The two world mods (RIDE 6 tracks, Fairhaven) write their `worlds/<id>.toml` through `WorldManifest::to_toml` and record a second-player slot.
