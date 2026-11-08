# Fairhaven: the city of NFS: Most Wanted (2012) as a second world

Part of the [modding catalogue](README.md). The mod is [`mods/fairhaven`](../../mods/fairhaven/README.md); the host feature it uses is
[specs/second-world.md](../specs/second-world.md); the formats it reads are [formats/chameleon-world.md](../formats/chameleon-world.md),
[chameleon-bundle.md](../formats/chameleon-bundle.md) and [chameleon-gfx.md](../formats/chameleon-gfx.md); the plan it follows is
[plans/tracks-and-fairhaven-feasibility.md](../plans/tracks-and-fairhaven-feasibility.md); progress and the resume point are in
[plans/fairhaven-progress.md](../plans/fairhaven-progress.md).

## Rule and shape

The user's rule for another game's content: **stream it from the original files when possible; otherwise convert with a very small,
fast converter; never put it in the repository.** Fairhaven is streamed. The city is 169 `HAWAII/TRK_UNIT<n>.BNDL` files (3.32 GB) and
the mod writes **no** converted copy: its folder holds, for each unit, a manifest of about 150 bytes and a stub of about 60 bytes
(`worlds/fairhaven/u<n>.toml`, `files/WORLDS/FAIRHAVEN/U<n>.BUN`) plus the world's manifest: about 40 KB in all.

```
game opens the world         reads 169 small manifests (map packs with `placed = "file"`): nothing of the install is read
camera comes within 150 m    the host reads the unit's file through the install: it is a stub, so GameDir::read asks the mod's maker
maker (a loader thread)      opens TRK_UNIT<n>.BNDL by byte ranges, reads what the unit uses, builds a section file in memory
host                         parses the section like any install section: solids, textures, scenery placements, collision pack
```

Why a mod and not the host: the host stays free of any 2012 reader; the mod uses `libs/chameleon-bundle`, `libs/chameleon-gfx` and the
game's own writers (`blackbox-solid`, `blackbox-tpk`, `blackbox-collision`, `blackbox-scenery`), and the only thing the host got is
generic: [packs that a file places](../specs/second-world.md#pieces-made-when-asked-for-built), read through the install, so that a
lazy file ([lean-content.md](lean-content.md), `bb.content` 1.1) is made when the camera comes near and not when the world opens.
Why a new mod crate and not `nfsmw-2012-content`: that mod converts cars into the garage and writes files; this one adds a world, is
read-only on the install, has its own settings and is useful alone.

## Settings

| Key (`config.toml` of the mod, or `[mods.fairhaven]`) | Default | Meaning |
|---|---|---|
| `source` | none | the folder of the 2012 install; without it the mod adds nothing and says so in the log |
| `enabled` | `true` | `false`: the mod offers no world and reads nothing |
| `max_texture` | 512 | the longest side of a picture in pixels (a power of two, 64 to 2048): the top mip levels are dropped, nothing is resampled |

The world is chosen by the **Free Roam World** row of the Gameplay options (listed while a mod provides a world), the `world` setting
(`world = "fairhaven"`) or `--world fairhaven`.

## What a unit becomes

| 2012 | This game | Where |
|---|---|---|
| `ZoneList` of `PVS.BNDL`: a rectangle per unit | the unit's manifest: an anchor circle (the rectangle's circle, render space) and a section number (2700 band, by unit number) | `pvs.rs`, `manifest.rs` |
| `InstanceList` (0x50): model + matrix | a scenery placement; the matrix becomes the game's rotation rows and position | `instances.rs`, `space.rs` |
| `Model` (0x51), its levels of detail | the first level is the player-view solid, the middle level of the chain the coarse one (slot 2 of the game's two-level choice) when both are a single solid | `convert.rs` |
| `Renderable` meshes (float positions, half UV, strips) | solids with groups: triangle lists, normals worked out, a mid-grey colour, 16-bit indices (a mesh over 60,000 vertices or triangles is cut) | `mesh.rs`, `solid.rs` |
| `Material` + `Shader` technique | a diffuse picture by the sampler order of the technique; `1Bit`, `Fence*` cut out (punch-through alpha), `Doublesided` two sides; water and cables are not drawn | `material.rs` |
| `Texture` BC1/BC2/BC3 | the same pixels as DXT1/3/5 with the top levels dropped to `max_texture`; BC7, BC4, BC5, RGBA decoded and stored as DXT1 (DXT5 with alpha) with a chain made here | `textures.rs` |
| `PolySoupList` (0x60) | the game's collision pack (`write_soup_pack`): triangles in physics space wound to face up, a surface per face | `collision.rs` |

Space: engine (x left, y up, z forward) to render (z, x, y) and physics (-x, y, z): [chameleon-world.md](../formats/chameleon-world.md).
Each solid is moved to its own centre (rounded to a metre) so the game can cull it by radius; the instance carries the centre.
Instances whose bounding sphere is over 800 m (the ring of hills and the sea every unit repeats) or whose centre is more than 25 m
outside the unit's rectangle (a unit lists its borders twice) are left out.

## Surfaces of the collision **[unconfirmed]**

The face flag word's bit 0 marks roads and flat ground a car drives on (asphalt); every other face is grass. The map from the word's
index bits to the 2012 surface table is not known (the names are hashed in `SURFACELIST.BNDL`). Tyre grip, sound and smoke follow the
surface exactly as in the city.

## Numbers

Author's machine (Ryzen 7 9800X3D, RTX 4070 SUPER, SSD, release build), measured 2026-11-08. The machine was shared with other
builds for most of the session, so the times are upper bounds.

| What | Number |
|---|---|
| Disk the mod uses | 169 manifests + 169 stubs + the world manifest: about 40 KB. Nothing else is written |
| Opening the install (the tables of 185 files) | 16 ms warm |
| One unit made (`fairhaven-unit`, all 169 units, warm cache, 512-pixel pictures) | median 550 ms, 90th percentile 850 ms, slowest 1.36 s; 95 s for all 169; 2.3 GB of section bytes in all (13.8 MB a unit, held in memory only; the last three are kept). First read of a cold file adds the disk: 5 s for unit 161 once |
| Where the time goes (unit 161) | pictures 220 to 650 ms (inflating the zlib blocks, the rest copies), collision 2 ms, writing the section 40 to 80 ms |
| Harbour unit 161 | 817 placements, 342 solids, 187,000 triangles, 198 pictures, 21,484 collision triangles, 36.5 MB |
| Units resident while driving | 4 to 8 (the host loads a unit within 150 m of its circle) |
| Memory (resident set, drive in the city) | 870 to 980 MiB, peak 1.03 GiB |
| Frame time, offscreen drive at the harbour, quiet machine | p50 1.5 ms, p95 2.4 ms, p99 7 ms, worst 48 ms (577 fps; 5 hitches before the scene was ready) |
| The same on the shared machine (three locations) | p50 3 to 10 ms, p95 9 to 48 ms, p99 62 to 90 ms: the unit loads and the other builds, not the drawing |

No range or level-of-detail limit was needed on this machine: the game's own scenery choice (the middle level of each model as the
coarse one, culling by projected size) keeps the drawn objects at 100 to 530 in the screenshots. A slower machine lowers
`max_texture` (pictures are the bulk of a unit's memory and time).

The world's manifest is written with `blackbox_modding::content::WorldManifest::to_toml` (one writer of the schema) and records a second-player slot 3 m to the left of the start; other mods can change this world with a [world patch](api-content-worlds.md#alternative-maps-environment-and-world-patches-bbcontent-12).
