# A second world

How the game plays somewhere that is not the 2005 city: a world a mod provides, chosen by `--world <id>`, the `world` setting
or the Free Roam World row of the Gameplay options. Built for the RIDE 6 tracks ([ride6-tracks mod](../modding/ride6-tracks.md));
meant to serve the 2012 Fairhaven map as well ([plan](../plans/tracks-and-fairhaven-feasibility.md), layers 1 to 13 of its
section 2). Mod-facing description: [api-content.md](../modding/api-content.md#worlds), [api-world.md](../modding/api-world.md#worlds).

## What a world is

A world is the ground, collision and spawn point of a place, drawn and driven with the game's own renderer, physics and
camera. It is **not** a game mode: no traffic, police, pursuit, races, safe houses, career, interiors, minimap or weather maps
(each is the city's data, placed in the city's coordinates). A player drives a car or a bike on it, in free roam.

## The manifests **[built]**

A mod folder holds:

```
worlds/<id>.toml            the world: its name and where the player starts
worlds/<id>/<pack>.toml     one scenery pack manifest per piece of the world (the schema of scenery/<pack>.toml)
files/<any>/<PACK>.BUN      each piece's section file, in the install's formats (solids, texture pack, collision pack)
```

```toml
# worlds/okayama.toml
[world]
name = "Okayama (RIDE 6)"                                     # what the menu shows (at most 64 bytes)
start = { pos = [-230.8, -214.7, 26.1], heading = 95.1 }      # render space, metres; heading in degrees, 0 along +x
```

`<id>` is 1 to 32 letters, digits, `_` or `-` (the file name), and is what `--world` and the `world` setting name; `rockport` is the
city and cannot be provided. An id an earlier mod already provides is refused, as is a manifest that does not parse or has a key the
schema does not know (the world is logged and left out; the game goes on). The packs are read **only while this world is the one
played**: a pack of a world never appears in the city, which matters because its coordinates are its own. Everything the
[scenery pack manifest](../modding/api-content.md#scenery-pack-manifest-built) says holds for them: the section numbers are in the
free bands (K, N, U, W, 2700 and up with the last two digits 0 to 39 or 80 to 99; a world takes them freely, since only one world is
played), a pack loads while the camera is within its `anchor` radius plus 150 m, and its collision is the pack's own, tested by every
query (`CollisionWorld::insert_ungridded`).

The parser is `libs/blackbox-modding/src/content/world.rs` (`parse_world`, `WORLDS_FOLDER`); the host reads the folders in
`crates/nfsmw/src/modding/content/worlds.rs` (`list`, `find`, `chosen`, `packs`). The list is read when a world opens or the menu is
drawn and nothing is sealed at start-up, so a world a converting mod finishes later (after `content.rescan()` added its `files/` as a
layer) is found at the next look.

## Choosing a world **[built]**

| Way | Where |
|---|---|
| `nfsmw view-world --world okayama [--drive CAR]` | `cli/view_args.rs` (`ViewArgs::world`), `commands/mod.rs` |
| `world = "okayama"` in the config file, `NFSMW_WORLD` | `settings/` (`WorldId`, one inline 32-byte id so the settings stay `Copy`) |
| Gameplay options: Free Roam World (listed only while a mod provides a world) | `frontend/options_world.rs` |

The setting applies to a **plain Free Roam start** (`Command::StartFreeRoam`) and to `view-world`; the career, quick races, custom
races, split screen and the rest are the city's. A name no loaded mod provides is logged with the list of what is installed and the
city is played (the menu start) or the command fails (`view-world`).

## What the world replaces **[built]**

`WorldScene::load_in(dir, options, Some(world))` (`scenes/world/open.rs`):

| City | Second world |
|---|---|
| `WorldIndex::open("L2RA")`: 720 sections, 435 zones, collision grid, road network, track zones and barriers, scenery groups | `WorldIndex::without_city(id)`: **the shared sets stay** (the sky, the common models and textures), every map tile, zone, grid, road and group goes (`nfsmw-data/src/world/index.rs`) |
| Map packs of every mod's `scenery/` | the packs of `worlds/<id>/` only |
| Interiors, races, shops, blips (`gameplay.bin`) | none |
| Weather maps (rain, wind), mesh animations, audio environment of `L2RA.BUN` | none (defaults) |
| Start: the middle of the city, or `--at` | `worlds/<id>.toml` `start` (position and heading), or `--at` |
| Traffic and police | none: no road network, so no `TrafficWorld` |

Two rules of the residency changed for worlds without zones and are right for the city too (`scenes/world/residency/ready.rs`,
`residency.rs`, `zone.rs`):

1. A world with no map tile has no zone to wait for: `has_zone` is set at once (`Zones::is_empty`).
2. **A start waits for the map packs around the camera**: `wanted_resident` includes the packs the camera is in or near
   (`mod_wanted`), so the car does not start simulating before the ground it stands on is resident (the city's wait rule covered
   tiles only, and a pack is no tile).

## Ground and surfaces **[built, see the mod]**

A world's collision is a `0x3B801` pack in each piece's file, with the surface of each triangle as a `simsurface` key
([collision.md](../formats/collision.md#surface-types-verified)), written by `blackbox_collision::write_soup_pack`
([collision.md](../formats/collision.md#writing-a-pack-from-a-triangle-soup-verified-by-reading-it-back)). Tyre grip, rolling sound,
skid and smoke effects and the bike's off-road terms then follow the surface exactly as in the city. Placing the car on a road
(`road.rs`) looks for asphalt and concrete in the collision, so a world needs asphalt under its start.

## Ground reference needs a grid **[built]**

A car's body reads the ground through the collision **grid** (`ground_reference`, the candidate windows), which the city's
`L2RA.BUN` provides and a second world has no file for. Found the hard way: with the packs inserted **ungridded** (each ray tests
them in turn) the car stood on the ground and its wheels spun without moving it, since the body found no ground plane. So the world
lays an empty `Grid` over the anchors of its packs (64 m cells, or wider for a world so large that 64 m would make more than 60,000
cells: `world_collision_grid`), and each pack lists its instances in it when it arrives and takes them out when it is released
(`CollisionWorld::insert_into_grid`, `remove_from_grid`). The grid lists only what is loaded, so it cannot say that a pack is
missing: `simulation_ready` also waits for the packs the camera is in or near. Tests: `blackbox-collision/src/tests/soup_writer.rs`.

## Measured **[ours]**

RIDE 6 Okayama on the author's machine (release build, an SSD, 12 hardware threads):

| What | Number |
|---|---|
| Export of the pack from the install (warm package cache) | 5 s, 91 MB (of which 48 MB is the glTF scratch that the exporter deletes) |
| Conversion of the pack to the world (all cells, at 512-pixel pictures) | **0.6 s**, 106 MB in 57 cells (92 MB in 147 cells of 384 m before the quadtree), 658,294 triangles, 17 pictures |
| The same at 256 and 128 pixels | 58 MB and 50 MB: the pictures are 44 MB of the 106 at 512; the collision is about 26 MB and the geometry about 22 MB |
| Start of free roam in the world, offscreen, to the car on the grid | about 7 s, of which the shared sets of the city are most |
| Frame cost | see [below](#frame-cost) |

All 39 tracks convert, 0.2 to 10 s each, 24 to 522 MB (typically 50 to 100 MB; the open maps are the large ones: Cappadocia Trails 522
MB, Stelvio Park 351 MB, Blue Wave Circuit 184 MB, Nevada 162 MB). The lap checked against the collision (`ride6-track-import --check`):
on Okayama, 1,854 of 1,863 samples of the lap are asphalt; a few tracks have a lap with large stretches with no ground under it
(Nurburgring GP: 1,781 of 2,541), which means their racing surface is in another level than the terrain one, not read yet.

## Pieces made when asked for **[built]**

A world of hundreds of pieces that exist in another game's files (Fairhaven's 169 units, [fairhaven mod](../modding/fairhaven.md))
cannot be written to disk and cannot be read when the world opens. A piece is the same map pack as above with two differences, so
the extension is a second kind of piece in the same residency and not a second world mechanism:

1. `placed = "file"` in its manifest ([api-content.md](../modding/api-content.md#scenery-pack-manifest-built)): no `[[place]]`; the
   file holds its own `ScenerySection` (written by `blackbox_scenery::write_section`, the inverse of the reader) and collision pack.
   `assemble` (`modding/content/scenery.rs`) keeps both and numbers them with the pack's section number.
2. The file is read **through the install** (`PackSource::Installed`, `GameDir::read`) when the camera comes near, never at open: its
   file in the mod's `files/` can be a [lazy file](../modding/api-content.md#lazy-files), a stub the mod replaces with the bytes
   it makes from the other game's files at that moment ([lean-content.md](../modding/lean-content.md)). The world's open therefore
   reads the manifests only (169 small text files), not the 3 GB.

All the pieces read through the install **share one streamer** (`residency/mod_tiles.rs::spawn_streamers`, 3 loader threads;
`Streamer::spawn_made` takes a function that makes section `n`), so 169 pieces do not start 169 threads. A piece with a file of its
own keeps its streamer as before. Tests: `blackbox-scenery/src/write.rs` (write then read back), `streamer.rs` (a streamer that
gets its sections from a function), `blackbox-modding` `scenery_tests.rs` (the manifest key).

## What is not built

- Roads, traffic, police, races and the minimap in a second world (the road network, the minimap's tiles and the nav data are each
  specified as open work in the plan, layers 7, 9 and 10).
- Lighting, time of day and weather of a world of its own: the city's sky and sun are used.
- A per-world `[world]` key for sky, fog or a time of day; added when a world needs one.

## Tests

`worlds.rs` (the list, the refusals, the duplicate id), `world.rs` of the parser, `world_id.rs` (the setting), and the whole
chain in `mods/ride6-tracks/tests/import_pack.rs`: a pack becomes two cells, each parses with `parse_scenery`, `read_solids`,
`read_texture_packs` and `read_collision_packs`, and a ray cast over the collision finds the right height and surface. The
offscreen commands that show a world are in [the mod's README](../../mods/ride6-tracks/README.md).
