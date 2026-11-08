# Mod API catalogue: scenery, map packs and worlds

Split out of [api-content.md](api-content.md) (the 500 line limit): the scenery and map pack manifests of `bb.content` and the world manifest.

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

**Replacing models of the world by name (built).** A mod that wants the models of existing scenery changed (the 2012 trees in place of the
2005 trees) does not need a registry call: the overlay replaces whole files, and the world's solids are 20,000 entries of one 533 MB
stream. The **trees variant** ([effect-variants.md](effect-variants.md#profile-format-of-the-trees)) is the file-level answer: the mod adds a
value to the `trees_variant` option (`bb.settings`) and ships `TREES/<ID>/profile.toml` with a pack of solids and a pack of textures; the
game replaces, by name, the listed solids of every section it streams in (and the world-space `_DEINST` copies of them), fitted into the
bounds of the solid each replaces, and uploads the pack's textures once, into the shared sets. No new
interface or ABI entry: it is `bb.settings` plus files, so any mod may offer another set of models the same way.

Cross-game: scenery from another Black Box game works only where `libs/blackbox-solid`, `blackbox-tpk` and
`blackbox-scenery` have a layout table for that game's version. The mod, or a tool, converts anything else.

### Worlds

A mod can provide a **second world** the player chooses instead of the city (`nfsmw view-world --world <id>`, the `world` setting,
the Free Roam World row). Built in `bb.content` 1.0 as a declarative manifest like the scenery packs; the host feature and its rules are
in [specs/second-world.md](../specs/second-world.md).

```
mods/example.tracks/
  worlds/okayama.toml           the world: name and start
  worlds/okayama/g000.toml      its map packs: one scenery pack manifest per piece (the schema above)
  files/WORLDS/OKAYAMA/G000.BUN each piece's section file (solids, textures, collision)
```

```toml
# worlds/okayama.toml
[world]
name = "Okayama (RIDE 6)"
start = { pos = [-230.8, -214.7, 26.1], heading = 95.1 }   # render space, metres; heading in degrees (0 along +x)
```

- The file name is the id (letters, digits, `_`, `-`, 1 to 32 bytes); `rockport` is the city and cannot be provided; an id an earlier
  mod provides is refused.
- A world's packs are read only while the world is played: they are not part of the city. Their rules are the scenery pack's, and a
  pack's collision (`collision = "static"` on a placement) is how the ground is driven on.
- The list is read when a world opens or a menu is drawn, not sealed at start-up; a mod that writes a world on a thread calls
  `content.rescan()` when it is done, as for cars.

| Entry | Signature sketch | Scope | Implemented in |
|---|---|---|---|
| Parse manifest | `parse_world(id, text) -> Result<WorldManifest { id, name, start, heading }>` | G | `libs/blackbox-modding/src/content/world.rs` |
| List, find | `worlds::list()`, `worlds::find(id)`, `worlds::chosen(setting)` | MW | `crates/nfsmw/src/modding/content/worlds.rs` |
| Load | `WorldScene::load_in(dir, options, Some(world))` | MW | `scenes/world/open.rs` |

The RIDE 6 tracks mod is the first user ([mods/ride6-tracks/README.md](../../mods/ride6-tracks/README.md)).
