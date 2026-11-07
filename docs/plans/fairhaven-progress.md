# Fairhaven: progress and resume point

Branch `feat/fairhaven` (worktree `nfs-mw-fairhaven`), stacked on `feat/ride6-tracks` (PR #300, the second-world host feature). Plan:
[tracks-and-fairhaven-feasibility.md](tracks-and-fairhaven-feasibility.md); design: [modding/fairhaven.md](../modding/fairhaven.md).
Updated after every layer; the top section is where to pick up.

## Where it stands (2026-11-08)

| Layer | State |
|---|---|
| Host seam (`placed = "file"` packs read through the install, one shared streamer, `blackbox_scenery::write_section`) | built, unit-tested |
| F0 geometry and pictures per unit, streamed | built; all 169 units convert with no error; not yet seen in the game |
| F1 collision from the soups | built (faces to triangles, surface by the drive bit); not yet driven on |
| F2 spawn, lighting, frame time | not started |
| F3 props, ground cover, roads, water | not started |

## Commands

```sh
# Make a unit (or all) as the game does, and print what it came to (warm cache: 170 to 300 ms a unit)
CARGO_TARGET_DIR=C:/Users/PADO/github/nfs-mw-build-fairhaven cargo build --release -p fairhaven -p nfsmw
fairhaven-unit "D:/need for Speed - Most Wanted" 161 --out C:/r6work/fairhaven/u161.bun
# Look at it offscreen and muted, in an isolated profile (mod folder with fairhaven.dll, mod.toml, config.toml)
BLACKBOX_PROFILE_DIR=C:/r6work/profile-fairhaven nfsmw view-world --world fairhaven --muted --wait-for-load --screenshot out.png
```

The profile is `C:\r6work\profile-fairhaven` (`nfsmw/config/config.toml`, `nfsmw/config/mods/fairhaven/{mod.toml,config.toml,fairhaven.dll}`).
