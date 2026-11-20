# Fairhaven: progress and resume point

Branch `feat/fairhaven` (worktree `nfs-mw-fairhaven`), stacked on `feat/ride6-tracks` (PR #300, the second-world host feature) plus the
four commits of `feat/2012-content-more` that Fairhaven needs (range-read bundles `cbcb882cd`, lazy files in the install `7e8d45864`,
`bb.content` 1.1 `b93320f00`, the host serving them `0feafe69a`; cherry-picked clean). Plan:
[tracks-and-fairhaven-feasibility.md](tracks-and-fairhaven-feasibility.md); design: [modding/fairhaven.md](../modding/fairhaven.md);
host side: [specs/second-world.md](../specs/second-world.md#pieces-made-when-asked-for-built). Updated after every layer; the top
section is where to pick up.

## Where it stands (2026-11-08, after the polish round on `feat/worlds-polish`)

| Layer | State |
|---|---|
| Host seam: `placed = "file"` packs read through the install on one shared streamer, `blackbox_scenery::write_section` | built, unit-tested (`mod_tiles.rs` test: manifest, lazy source, streamer, assemble) |
| F0 geometry and pictures per unit, streamed from the install | **built and seen**: the harbour district, the mountain highway and the districts between render offscreen; all 169 units convert with no error |
| F0 materials | diffuse picture by the technique's sampler; cut-outs and two sides; terrain tiling ratios and the three terrain layers cut by the mask; the sea as one flat colour; a `brightness` gain on every picture (default 70) because the shader doubles a picture; no lightmaps, reflections, lit windows, normal maps |
| Models in no unit | **found and added**: 69 models over the 169 units were in `ARENA_*.BNDL` (59) and in the units the zone list leaves out (10); their tables join the pool; `fairhaven-unit --find-missing` |
| Sea | a flat 120 km square at 9.6 m under the whole world (`sea.rs`, 4 KB, made by the mod), the coast's own sea mesh still draws over it |
| F1 collision from the soups | **built and driven on**: a BMW M3 GTR and a KTM 450 drive on the mountain road and the harbour plaza; surface = drive bit asphalt, else grass **[unconfirmed]** |
| F2 spawn, lighting, frame time | start = the largest flat road face of the unit with most road; the host's sun and sky; frame times measured (below) |
| F3 props | placed: props, compound and dynamic objects (428 in the harbour unit) drawn as still models |
| F3 ground cover (8.8 M instances), road graph for traffic, water as water, a drawn minimap | not built; the original HUD's map dial no longer draws empty (it is hidden in a second world) |

## Commands

```sh
# Make a unit (or all) as the game does, and print what it came to (warm cache: 170 to 400 ms a unit)
CARGO_TARGET_DIR=C:/Users/PADO/github/nfs-mw-build-fairhaven cargo build --release -p fairhaven -p nfsmw
fairhaven-unit "D:/need for Speed - Most Wanted" 161 --out C:/r6work/fairhaven/u161.bun
fairhaven-unit "D:/need for Speed - Most Wanted"            # every unit
# Look at it offscreen and muted, in an isolated profile (mod folder with fairhaven.dll, mod.toml, config.toml)
export BLACKBOX_PROFILE_DIR=C:/r6work/profile-fairhaven
nfsmw view-world --world fairhaven --muted --no-traffic --wait-for-load --at 2100,1770 --height 30 --heading 200 --pitch -10 \
  --screenshot out.png --screenshot-delay 10 --game-dir "D:/Need For Speed Most Wanted Black Edition"
nfsmw view-world --world fairhaven --drive BMWM3GTR --camera-mode chase --drive-script "3:throttle=1" --muted --no-traffic \
  --wait-for-load --screenshot out.png --screenshot-delay 10 --game-dir "D:/Need For Speed Most Wanted Black Edition"
nfsmw view-world --world fairhaven --drive KTM450 ...        # a bike (the profile has the ride6-bikes mod)
nfsmw view-world --world fairhaven --drive BMWM3GTR --at 2100,1770 --drive-script "40:throttle=1,steer=0.03" \
  --bench-seconds 15 --offscreen --no-vsync --max-fps unlocked --muted --no-traffic        # frame times
```

`--at X,Y` is in the game's render space (x forward = engine z, y left = engine x). The profile is `C:\r6work\profile-fairhaven`
(`nfsmw/config/config.toml`, `nfsmw/config/mods/fairhaven/{mod.toml,config.toml,fairhaven.dll}`, `ride6-bikes/`). Delete
`mods/fairhaven/worlds/fairhaven.toml` there to make the start again.

Screenshots of the run are in `C:\r6work\fairhaven\shots\` (`f0_*` first light, `f2_*` the sea and the terrain, `f3_*` props,
`f1_*` driving).

## Numbers (author's machine: Ryzen 7 9800X3D, RTX 4070 SUPER, SSD; shared with other builds, so times are upper bounds)

See [modding/fairhaven.md](../modding/fairhaven.md#numbers): a unit takes 550 ms (median) to make on a loader thread, 40 KB of disk
in all, frame time p50 1.5 ms on a quiet machine and 3 to 10 ms on a loaded one, 0.9 to 1 GiB resident.

## Open issues and next steps

1. **Shading** (done as a gain, not as a model). Roofs and quays were clipped to cream because the shader draws
   `2 * picture * (ambient * vertex colour + sun * N.L)` and the 2012 pictures are authored brighter; `brightness` = 70 gives roofs about
   200 of 255 instead of 247 (the mean of a 120 by 40 pixel patch of a roof in the harbour view: 247,244,212 before, 198,185,150 at 70, 237,224,180 at 85).
   The baked lightmap (second UV, `World*_Lightmap`) is still not read.
2. **Surfaces.** `SURFACELIST.BNDL` holds about 30 `SurfaceType` objects (effects, not names), fewer than the flag word's index range, so the
   index is not a `SurfaceType` id. Read off the faces of seven units (28 distinct values of bits 16 to 23): the low nibble of that byte
   separates flat road (0 and 3: bit 0 set on every face, |normal y| 0.98 to 1.00), a mix (1 and 2: 0.3 to 0.6), walls (5: no bit 0,
   |normal y| 0.36 to 0.48) and other steep faces (6 and 7); the high nibble (1 to 4) follows the height of the unit, so it is a
   district or layer number and not a surface. This supports "bit 0 is road" and nothing about grass, sand or water: a surface table
   needs the game's collision material list, which was not found. The classification stays asphalt for bit 0, grass for the rest.
3. **Terrain mask.** The weights of the three layers (red, green, blue) are guessed from the slope each channel covers; a baked blend
   would give soft edges. `Plot*` materials draw layer 1.
4. **Ground cover** (`GroundcoverInstances`, 8.8 M instances: packed 16-byte records, dequantisation not decoded). Plan, not built: decode
   the records of one unit (about 52,000 a unit on average), place only the instances within 80 m of the camera as scenery placements of an
   `Option`-toggled pack (`ground_cover = false` by default, a Gameplay or Video row, translations in every language file),
   instanced through the model swap's tree path (`libs`/`crates` trees), and measure p50 and p95 with it on and off before turning
   it on by default: an estimate, not measured, is 3 % of a unit's instances in range at 80 m, about 1,500 placements.
5. **Roads and traffic.** `TrafficLaneData` (2.35 MB) starts with a spatial grid of rectangles of about 400 m (min and max x and z as floats,
   `-FLT_MAX` to `FLT_MAX` in height) and a table of imports at its end; the lane records after the grid were not decoded. A `RoadNetwork`
   for traffic and police needs the lanes (the spike the plan names); with it the Custom HUD's map (`map_roads`) would draw the city.
6. **A drawn minimap.** Hidden in a second world now; a map needs the roads of item 5 (Custom HUD) or 64 tile pictures drawn from them
   (original HUD): [specs/second-world.md](../specs/second-world.md#what-the-ride-6-worlds-add-on-top-built).
7. **The sea as water.** Flat colour only; the 2012 water is a normal map and a reflection.
