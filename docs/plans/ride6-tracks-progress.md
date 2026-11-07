# RIDE 6 tracks: progress and resume point

Branch `feat/ride6-tracks` (worktree `nfs-mw-track`), from `origin/main`. Plan: [tracks-and-fairhaven-feasibility.md](tracks-and-fairhaven-feasibility.md).
Updated after every milestone; the top section is where to pick up.

## Where it stands (2026-11-07)

| Milestone | State |
|---|---|
| Exporter `tools/ride6-tracks` | built; all **39 tracks** export (3 to 11 s each); `survey.py` does them all, `preview.py` draws a top view |
| M0 second-world host feature | built and verified offscreen: [specs/second-world.md](../specs/second-world.md); `--world`, the `world` setting, the Gameplay row |
| M1 render | verified: Okayama, Monza and Suzuka render offscreen (ground, kerbs, pictures, the city's sky) |
| M2 collision and driving | verified: a BMW M3 GTR and a KTM 450 stand on the grid, drive on the asphalt, run on kerb and grass; the lap's 1,854 of 1,863 samples are asphalt |
| M3 lap timing | built and unit-tested (`mods/ride6-tracks/src/lap.rs`), **not seen in the game** (offscreen runs advance the world in batches, the mod gets one tick) |
| M3 ideal racing line overlay | not built (the line is in the levels, the host has no line drawing for a mod) |
| M3 more maps | all 39 convert (0.2 to 10 s, 24 to 522 MB); only Okayama, Monza and Suzuka were looked at |

## Commands

```sh
# Export (the exporter's scripts are found through R6_EXPORTER_DIR, else tools/ride6-exporter next to this tool)
R6_EXPORTER_DIR=<checkout with tools/ride6-exporter> python -I tools/ride6-tracks/export.py Okayama --out C:/r6work/tracks/okayama-pack
python -I tools/ride6-tracks/survey.py            # every track into C:/r6work/tracks/<track>-pack
python -I tools/ride6-tracks/preview.py C:/r6work/tracks/okayama-pack top.png        # a top view by surface
# Convert (what the mod does in the background) and check the lap against the collision
ride6-track-import C:/r6work/tracks/okayama-pack <mod folder> --max-texture 512 --check
# Look at it, offscreen and muted, in an isolated profile
BLACKBOX_PROFILE_DIR=C:/r6work/profile-track nfsmw view-world --world okayama --drive BMWM3GTR --muted --wait-for-load --screenshot out.png --screenshot-delay 8
```

Build: `CARGO_TARGET_DIR=C:/Users/PADO/github/nfs-mw-build-track cargo build --release -p nfsmw -p ride6-tracks`. The isolated profile is
`C:\r6work\profile-track` (config, the mod folder with `ride6_tracks.dll`, `files/` and `worlds/` of the converted Okayama, Monza and Suzuka,
and a KTM 450 converted by this tree's `ride6-bike-import` because the user's installed bikes are from a newer build).

## Open issues and next steps

1. **Frame cost.** Reflections at their default setting cost 18 to 48 ms of GPU in Okayama (658,000 triangles drawn into the cube map)
   against 1.8 ms in the city; with reflections off the GPU is idle. The world needs level of detail: decimated far cells, or the
   reflection pass's draw distance. (The bench numbers of CPU time were taken while other jobs used the machine.)
2. **Disk.** 106 MB for Okayama at 512-pixel pictures (collision 26 MB, geometry 22 MB, pictures 44 MB repeated in each cell that uses
   them). A shared texture set (cells refer to the world's pictures once) and strips in the collision pack would about halve it.
3. **Void below the horizon.** The far hills are drawn as floating pieces over black: there is no ground plane or sea beyond them.
4. **Surfaces.** The blend order of the four terrain layers and the tile size are guesses that look right; the physical material of
   layered terrain is one value per slice, so gravel traps are told by their pictures. Barriers (`Props_Barriers`, the walls the bike
   must hit) are not exported: a rider can leave the track anywhere the ground goes.
5. **Laps.** Nine tracks have no `TrackSpline` the exporter finds, 17 no starting grid; the pit lane and the route forks (`SplinesB`) are
   not used. The lap is checked on asphalt only for Okayama in detail (the survey line per track is in the conversion log).
6. **Streamed sources.** Fairhaven needs a piece that reads the install; see "What is not built" of the spec.
