# Fairhaven: progress and resume point

Branch `feat/fairhaven` (worktree `nfs-mw-fairhaven`), stacked on `feat/ride6-tracks` (PR #300, the second-world host feature) plus the
four commits of `feat/2012-content-more` that Fairhaven needs (range-read bundles `cbcb882cd`, lazy files in the install `7e8d45864`,
`bb.content` 1.1 `b93320f00`, the host serving them `0feafe69a`; cherry-picked clean). Plan:
[tracks-and-fairhaven-feasibility.md](tracks-and-fairhaven-feasibility.md); design: [modding/fairhaven.md](../modding/fairhaven.md);
host side: [specs/second-world.md](../specs/second-world.md#pieces-made-when-asked-for-built). Updated after every layer; the top
section is where to pick up.

## Where it stands (2026-11-08)

| Layer | State |
|---|---|
| Host seam: `placed = "file"` packs read through the install on one shared streamer, `blackbox_scenery::write_section` | built, unit-tested (`mod_tiles.rs` test: manifest, lazy source, streamer, assemble) |
| F0 geometry and pictures per unit, streamed from the install | **built and seen**: the harbour district, the mountain highway and the districts between render offscreen; all 169 units convert with no error |
| F0 materials | diffuse picture by the technique's sampler; cut-outs and two sides; terrain tiling ratios and the three terrain layers cut by the mask; the sea as one flat colour; no lightmaps, reflections, lit windows, normal maps |
| F1 collision from the soups | **built and driven on**: a BMW M3 GTR and a KTM 450 drive on the mountain road and the harbour plaza; surface = drive bit asphalt, else grass **[unconfirmed]** |
| F2 spawn, lighting, frame time | start = the largest flat road face of the unit with most road; the host's sun and sky; frame times measured (below) |
| F3 props | placed: props, compound and dynamic objects (428 in the harbour unit) drawn as still models |
| F3 ground cover (8.8 M instances), road graph for traffic, water as water, minimap | not built |

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

See [modding/fairhaven.md](../modding/fairhaven.md#numbers).

## Open issues and next steps

1. **Shading.** Ground and roofs are too light: the 2012 pictures are lit by a lightmap and a sun the game does not have, and the game's
   pre-lit shader takes a constant vertex colour (0x58 now). A per-technique brightness, or the baked lightmap (second UV, `World*_Lightmap`),
   would help.
2. **Surfaces.** The face flag word's index bits (10 bits, 1 to 0xDE) look like a surface table; `SURFACELIST.BNDL` names are hashed, so the
   map is unknown. Bit 0 stands for road.
3. **Terrain mask.** The weights of the three layers (red, green, blue) are guessed from the slope each channel covers; a baked blend
   would give soft edges. `Plot*` materials draw layer 1.
4. **Ground cover** (`GroundcoverInstances`, 8.8 M instances: packed 16-byte records, dequantisation not decoded) and the **sea** as water.
5. **Roads and traffic.** `TrafficLaneData` (2.35 MB) starts with a spatial grid of rectangles of about 400 m (min and max x and z as
   floats, `-FLT_MAX` to `FLT_MAX` in height) and a table of imports at its end; the lane records after the grid were not decoded. A
   `RoadNetwork` for traffic and police needs the lanes; this is the spike the plan names.
6. **Minimap.** The dial draws empty in a world (the city's tiles are not its map); `MAPS/*.BNDL` (86 textures) were not read.
7. Missing models (3 to 6 per unit) are in no unit or shared bundle: arena content, probably.
