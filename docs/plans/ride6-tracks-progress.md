# RIDE 6 tracks: progress and resume point

Branch `feat/ride6-tracks` (worktree `nfs-mw-track`), from `origin/main`. Plan: [tracks-and-fairhaven-feasibility.md](tracks-and-fairhaven-feasibility.md).
Updated after every milestone; the top section is where to pick up.

## Where it stands

| Milestone | State |
|---|---|
| Exporter (`tools/ride6-tracks`) | built: Okayama ground, pictures, surfaces, lap in 5 s (warm cache) |
| M0 second-world host feature | built, compiles, unit-tested; see [specs/second-world.md](../specs/second-world.md) |
| Collision pack writer (`libs/blackbox-collision`) | built, tested by reading back and ray casts |
| Mod `mods/ride6-tracks` | built, tested with a synthetic pack end to end |
| M1 render-only Okayama | in progress: first offscreen render of the converted world |
| M2 collision and driving | to do |
| M3 racing line, lap timing, more maps | to do |

## Commands

```sh
# Export (the exporter's scripts are found through R6_EXPORTER_DIR, else tools/ride6-exporter next to this tool)
R6_EXPORTER_DIR=<checkout with tools/ride6-exporter> python -I tools/ride6-tracks/export.py Okayama --out C:/r6work/tracks/okayama-pack
python -I tools/ride6-tracks/preview.py C:/r6work/tracks/okayama-pack top.png        # a top view by surface
# Convert (what the mod does in the background)
ride6-track-import C:/r6work/tracks/okayama-pack <mod folder> --max-texture 512
# Look at it, offscreen and muted, in an isolated profile
BLACKBOX_PROFILE_DIR=C:/r6work/profile-track nfsmw view-world --world okayama --drive BMWM3GTR --muted --wait-for-load --screenshot out.png
```

Build: `CARGO_TARGET_DIR=C:/Users/PADO/github/nfs-mw-build-track cargo build --release -p nfsmw -p ride6-tracks`. The isolated profile is
`C:\r6work\profile-track` (config, the mod folder with `ride6_tracks.dll`, `files/` and `worlds/` of the converted Okayama).

## Next steps

1. Render the world offscreen and fix what is wrong (winding, brightness, texture tiling, the sky).
2. Drive on it: the car must stand on the grid and not fall through; check surfaces with a script.
3. Measure disk and time, put the numbers in the pack and spec docs, reduce them (shared pictures, strips in the collision).
4. Lap: the pack has the spline and the grid; a lap counter and the line overlay.
5. More maps through the same exporter.
