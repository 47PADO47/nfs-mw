# RIDE 6 tracks: progress and resume point

Branch `feat/ride6-tracks` (worktree `nfs-mw-track`), PR #300. Plan: [tracks-and-fairhaven-feasibility.md](tracks-and-fairhaven-feasibility.md).
Updated after every step; the top section is where to pick up.

## Where it stands (2026-11-08)

| Step | State |
|---|---|
| Exporter `tools/ride6-tracks` | all **39 tracks** export (ground, pictures, surfaces, lap, racing line, sea, objects); `survey.py` does them all, `preview.py` draws a top view |
| M0 second-world host feature | built and verified offscreen: [specs/second-world.md](../specs/second-world.md) |
| M1 render | verified on Okayama, Monza, Suzuka, Brands Hatch, Cadwell Park, Donington, French Riviera |
| M2 collision and driving | verified: a BMW M3 GTR and a KTM 450 drive on the grid; a car driven into the pit wall stops against it |
| Performance (1) | measured: no GPU problem on an idle machine (below); far ground and a sea give a horizon at 4 % of the triangles |
| Objects (2) | barriers, buildings, props placed (Okayama 10,522), walls as boxes in the collision; vegetation, crowd and grass not done |
| Minimap, horizon, sky (3) | horizon and sea done; the city's minimap tiles are not drawn in a world; **the radar's dark disc is still drawn** (not solved) |
| Racing line and laps (4) | the line is a ribbon on the ground (seen); lap timing is unit-tested, and its "timing on" message was seen; the crossing messages could not be seen offscreen |
| Other tracks (5) | seven tracks looked at; the list is in the report and in the survey logs |

## Performance (1)

The numbers asked for (18 to 48 ms of GPU time with default reflections against 1.8 ms in the city) were **not reproducible**: they
came from runs while other jobs used the machine. On an idle machine, same start, `--bench-seconds 8 --offscreen --no-vsync`, BMW M3 GTR
chase view, frame time p50: city 2.5 ms; Okayama 2.4, Monza 3.0, Suzuka 2.9 (ground only); reflections at `high` 3.6 ms in Okayama.
With the objects (10,000 placements) and the line, Okayama measured 1.4 ms p50 but 6 to 15 ms p95 to p99 while a second job was
running, which is cell streaming (cells of 5 to 10 MB arrive over several frames), not drawing. The measures that keep it cheap are
structural: a world draws only the cells within their anchor radius plus 150 m of the camera and the one coarse far cell; reflections
and the mirror draw what is resident. Not done: decimated near cells (LOD), a smaller cell size for smoother streaming.

## Commands

```sh
# Export (the exporter's scripts are found through R6_EXPORTER_DIR, else tools/ride6-exporter next to this tool)
R6_EXPORTER_DIR=<checkout with tools/ride6-exporter> python -I tools/ride6-tracks/export.py Okayama --out C:/r6work/tracks/okayama-pack
python -I tools/ride6-tracks/survey.py            # every track into C:/r6work/tracks/<track>-pack (25 to 175 s each with the objects)
python -I tools/ride6-tracks/preview.py C:/r6work/tracks/okayama-pack top.png        # a top view by surface
# Convert (what the mod does in the background) and check the lap against the collision
ride6-track-import C:/r6work/tracks/okayama-pack <mod folder> --max-texture 512 --check [--no-objects] [--no-line]
# Look at it, offscreen and muted, in an isolated profile
BLACKBOX_PROFILE_DIR=C:/r6work/profile-track nfsmw view-world --world okayama --drive BMWM3GTR --muted --wait-for-load --screenshot out.png --screenshot-delay 8
```

Build: `CARGO_TARGET_DIR=C:/Users/PADO/github/nfs-mw-build-track cargo build --release -p nfsmw -p ride6-tracks`. The isolated profile is
`C:\r6work\profile-track`.

## Open issues and next steps

1. **Disk.** Okayama with objects is 188 MB at 512-pixel ground pictures and 256-pixel object pictures (106 MB without objects):
   pictures and models are repeated in each cell that uses them. A shared set (cells refer to the world's pictures and models once)
   would about halve it. `objects = false` in the mod's config drops them.
2. **The radar disc** of the original HUD stays dark over the left corner of the screen in a world.
3. **Surfaces.** The blend order of the four terrain layers and the tile size are guesses that look right; a picture the install
   cannot give (`T_White_Color`, a plugin texture) is drawn flat, which makes some buildings and cliffs (French Riviera) near white.
4. **Objects.** Walls are boxes of the model's size (an L shaped building blocks its corner); vegetation (alpha tested leaves, DXT1 has
   no alpha), grass and crowd are not placed.
5. **Laps.** Nine tracks have no `TrackSpline` the exporter finds, 17 no starting grid; the pit lane and the route forks (`SplinesB`) are
   not used. The crossing messages of the lap timer were not seen in the game.
6. **Streamed sources.** Fairhaven needs a piece that reads the install; see "What is not built" of the spec.
