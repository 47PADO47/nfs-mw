# RIDE 6 tracks: progress and resume point

Branch `feat/ride6-tracks` (worktree `nfs-mw-track`), PR #300. Plan: [tracks-and-fairhaven-feasibility.md](tracks-and-fairhaven-feasibility.md).
Updated after every step; the top section is where to pick up.

## Where it stands (2026-11-08, after the polish round on `feat/worlds-polish`)

| Step | State |
|---|---|
| Exporter `tools/ride6-tracks` | all **39 tracks** export (ground, pictures, surfaces, lap, racing line, sea, objects); `survey.py` does them all, `preview.py` draws a top view; ground pieces move by their actor's transform (Nurburgring GP, rocks of the open maps), the lap is found on all 39 (`lapfinder.py`), plugin pictures are flat colours; python tests `test_placement.py`, `test_lapfinder.py`, `test_flat_pictures.py` |
| M0 second-world host feature | built and verified offscreen: [specs/second-world.md](../specs/second-world.md) |
| M1 render | verified on Okayama, Monza, Suzuka, Brands Hatch, Cadwell Park, Donington, French Riviera, Nurburgring GP, Imola |
| M2 collision and driving | verified: a BMW M3 GTR and a KTM 450 drive on the grid; a car driven into the pit wall stops against it |
| Performance | no GPU problem; frame times below were measured on a machine that was **not idle** (other builds ran): see "Performance" |
| Objects | barriers, buildings, props placed in one section for each 500 m square (Okayama 147 MB, was 180); walls as boxes in the collision of the ground cells; vegetation, crowd and grass not done |
| Minimap | hidden cleanly in a second world (the original HUD's disc, ring and flame were the heat group under the tachometer's root); a drawn map is not built (design in the spec) |
| Racing line and laps | the line is a ribbon on the ground (seen); lap timing is unit-tested; its start and lap messages were **seen offscreen** with `RIDE6_TRACKS_LAP_DEMO` ("Lap 1  0:08.20  best lap" on Okayama); a real car crossing could not be seen (a scripted run advances the world in a batch) |
| Pictures | `brightness` setting (default 80), `T_White_Color` and `T_Default_BC` are flat colours |

## What this round found and did (2026-11-08)

1. **Radar disc.** Not the detector's backing: the dark disc, the segmented ring and the flame ("x10", the package's stored text) are the
   heat group (`0xC46A80A9`) and its arcs (`0xB0F63386`), which the PC layout puts under group `0x1603009E` under the tachometer's
   root `0x157C5B37`; the layout shows everything under a shown root, so the radar and heat flags changed nothing. Found by printing
   the visible nodes of the HUD tree with their ancestors. Fixed in `hud/elements.rs` (`Shown::mapless`, `hide_mapless`).
2. **Size.** Objects were repeated in every ground cell beside a placement (180 MB for Okayama with objects, 106 without). One
   section for all objects is 132 MB but measured slower (it keeps everything resident); a section for each 500 m square is 147 MB
   and as fast as the old layout. The ground's pictures are still repeated per cell (a placement and a solid can only name what is in
   their own section): a world-level shared set in the host would take about 30 MB more off Okayama.
3. **Nurburgring GP.** Every ground piece has the transform (7,057, -121, 855) cm that the exporter ignored: 1,781 of 2,541 lap
   samples had no ground; now 9. The same error put the rocks and cliffs of Cappadocia Trails, Nevada, Stelvio Park, Blue Wave Circuit
   and Kanto at the origin; French Riviera's white wall was one of them.
4. **`T_White_Color`** is used by the razor flags only (checked over all 39 packs), so it was not why buildings were near white: that was
   the brightness (the shader doubles a picture). Flat colours for the two plugin pictures; `brightness` for the rest.
5. **The nine tracks with no lap** had it under other actor names (`TrackSpline_2`, `Trackspline`, `IgnitionTrackSpline_0`): all nine have a
   lap now, and the head to head grid is used when there is no `StartingGrid`.
6. **Monza's load time.** 60 s on the first start after the files were written (cold disk, other builds running) and 8 s on the next:
   the 28 to 82 s of the survey were a cold, contended first load, not the world.
7. **Lap messages** seen with the demo car (see the table).

## The lap against the collision, all 39 tracks (2026-11-08, `ride6-track-import --check`, 128-pixel pictures)

Every track has a lap spline now. Samples with no ground under them: 0 to 31 on 37 tracks (a start straight under a bridge or a gap
at a join), 53 of 5,845 on Ulster GP; **Blue Wave Enduro 218 of 405** (its lap is an 813 m arena loop whose ground is not in the terrain
level the exporter reads). Many samples more than 1 m off the spline's height: Northwest 200 2,784 of 7,222 and Nordschleife 633 (roads
under bridges and walls of the objects over the line), Kyalami 483, Ulster GP 488, Spain Circuits 352, Blue Wave Circuit 320,
Canadian 251, Road America 238. Nurburgring GP: 9 without ground (was 1,781). Starting grids: 7 tracks have none (Blue Wave Arena and Enduro, Cappadocia Trails, Gunma, Nevada, Red Bull Ring, Stelvio Park; 17 before) and start on
the first sample of the lap. The Blue Wave Enduro level and the height differences are the next things to look at.

## Performance

Frame times, BMW M3 GTR driving straight, `--bench-seconds 12 --offscreen --no-vsync --max-fps unlocked`, release build, Okayama with
objects, two runs each, **with 5 to 8 other rustc processes running** (the machine was never idle):

| Layout | p50 ms | p95 ms | ready after |
|---|---|---|---|
| objects in each ground cell (before) | 4.5, 5.1 | 30.4, 35.6 | 3.2 s, 22.4 s |
| one section for all objects | 5.6, 13.3 | 35.0, 77.6 | 6.0 s, 12.6 s |
| **a section for each 500 m square** | 4.5, 8.0 | 34.1, 38.5 | 5.4 s, 10.6 s |

The p95 of 30 to 38 ms is the load (cells arriving over several frames and the other builds), not drawing: it is the same in the old
layout. An idle-machine figure was not possible: another agent's builds ran for the whole session. The earlier idle numbers (p50 2.4 to 3.0 ms, ground only) still stand.

## Commands

```sh
# Export (the exporter's scripts are found through R6_EXPORTER_DIR, else tools/ride6-exporter next to this tool)
R6_EXPORTER_DIR=<checkout with tools/ride6-exporter> python -I tools/ride6-tracks/export.py Okayama --out C:/r6work/tracks/okayama-pack
python -I tools/ride6-tracks/survey.py            # every track into C:/r6work/tracks/<track>-pack (25 to 175 s each with the objects)
python -I tools/ride6-tracks/preview.py C:/r6work/tracks/okayama-pack top.png        # a top view by surface
python -I tools/ride6-tracks/test_placement.py   # also test_lapfinder.py and test_flat_pictures.py: no install needed
# Convert (what the mod does in the background) and check the lap against the collision
ride6-track-import C:/r6work/tracks/okayama-pack <mod folder> --max-texture 512 --check [--no-objects] [--no-line] [--brightness 80]
# Look at it, offscreen and muted, in an isolated profile
BLACKBOX_PROFILE_DIR=C:/r6work/profile-track nfsmw view-world --world okayama --drive BMWM3GTR --muted --wait-for-load --screenshot out.png --screenshot-delay 8
# The lap messages without driving a lap: a virtual car at 450 m/s from 60 m before the line
RIDE6_TRACKS_LAP_DEMO=450 nfsmw view-world --world okayama --drive BMWM3GTR --muted --wait-for-load --screenshot lap.png --screenshot-delay 9
# Frame times (not on a machine that builds)
nfsmw view-world --world okayama --drive BMWM3GTR --drive-script "30:throttle=1" --bench-seconds 12 --offscreen --no-vsync --max-fps unlocked --muted --wait-for-load
```

The packs of the polish round are in `C:/r6work/worlds-packs/<track>-pack` (all 39, exported with the fixed exporter; the older ones in
`C:/r6work/tracks` have the origin error and no lap for nine tracks); its isolated profile is `C:/r6work/profile-worlds`.

Build: `CARGO_TARGET_DIR=C:/Users/PADO/github/nfs-mw-build-track cargo build --release -p nfsmw -p ride6-tracks`. The isolated profile is
`C:\r6work\profile-track`.

## Open issues and next steps

1. **Disk.** Share the ground's pictures between cells with a world-level shared set (a `[world]` key naming a texture-only section that
   the host adds to the shared sets); about 30 MB of Okayama's 147 MB. `objects = false` drops the objects.
2. **A drawn minimap.** The Custom HUD draws `state.map_roads` and the car: feed it the lap spline (a `[world]` key naming the line
   file, `map_picture` off in a second world). The original HUD's map needs 64 tile textures per world.
3. **Surfaces.** The blend order of the four terrain layers and the tile size are guesses that look right.
4. **Objects.** Walls are boxes of the model's size (an L shaped building blocks its corner); vegetation (alpha tested leaves, DXT1 has
   no alpha), grass and crowd are not placed. Near cells have no coarse level: a decimated copy for cells 150 to 400 m away would
   cut triangles, but the ground is 2.4 to 3 ms of GPU time idle, so it was not done.
5. **Laps.** The pit lane and the route forks (`SplinesB`) are not used; 7 tracks have no starting grid and start on the first sample of the lap; Blue Wave Enduro's lap has no ground under half of it.
6. **Idle measurements.** Repeat the table above on an idle machine.
7. **Streamed sources.** Fairhaven reads the 2012 install; see [fairhaven-progress.md](fairhaven-progress.md).
