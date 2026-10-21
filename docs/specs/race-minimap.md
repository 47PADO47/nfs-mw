# The minimap during a race

- **Sources read:** `speed.exe` (Black Edition PC v1.3) in Ghidra, read only; the install's `TRACKS/L2RA/` map files and
  `GLOBAL/gameplay.bin` races. Addresses and method: [../provenance/race-minimap.md](../provenance/race-minimap.md).
- **Data inputs:** the race's event id (attribute `0xA78403EC`, `Race::event`), `PursuitRace` (`0x2B1F54F6`), `Region`
  (`0xCB01E454`); `TRACKS/L2RA/MINI_MAP_<event id>.BIN`. File layout: [../formats/minimap.md](../formats/minimap.md). The
  free-roam rule (stage to variant) is [career-regions.md](career-regions.md) section 6.1; the drawing is
  [hud-minimap.md](hud-minimap.md).

Evidence tags: **[decomp]** read in the executable, **[verified]** measured on the install or on a screenshot of the
rewrite, **[ours]** a choice of the rewrite where the original is silent or not traced.

## 1. Which map file the minimap shows

The HUD's minimap file is chosen by one routine of the original, from two inputs: the racing state of the race status
object and, while racing, the current race. **[decomp]**

| Racing state | Race | File (`TRACKS/L2RA/`) |
|---|---|---|
| not racing (free roam) | none | by career stage: `MINI_MAP` (stage < 9), `MINI_MAP_UNLOCK_2` (9 to 12), `MINI_MAP_UNLOCK_1` (13 and up) |
| racing | `PursuitRace` false | `MINI_MAP_<event id>` |
| racing | `PursuitRace` true | by the race's `Region`: `college` `MINI_MAP_UNLOCK_1`, `coastal` `MINI_MAP_UNLOCK_2`, anything else `MINI_MAP` |
| the drag HUD (layout 2) | any | no minimap file |

The routine is the same for a career race, a quick race and a race started from the console: all of them are races of
the gameplay data, and the routine has no game mode input. A quick race therefore shows the map of the event it picked.
**[decomp]**

## 2. The file name

The event id is the string the race stores (`10.2.1`, `19.8.32`, the reversed `10.4.2.r`). The routine formats
`MINI_MAP_%s`, turns every `.` into `_`, makes the whole name upper case, and opens `TRACKS\L2RA\<name>.BIN`:

| Event id | File |
|---|---|
| `10.2.1` | `MINI_MAP_10_2_1.BIN` |
| `19.8.32` | `MINI_MAP_19_8_32.BIN` |
| `10.4.2.r` | `MINI_MAP_10_4_2_R.BIN` |

**Reversed routes.** A reversed race is a race of its own whose event id ends in `.r`; the routine does nothing special
for it, the suffix just becomes `_R`. **[decomp + verified]**: all 42 races of the install whose id ends in `.r` have
their `_R` file, and 55 `_R` files exist. Nothing derives a reversed map from the forward one.

Install facts **[verified]**: `TRACKS/L2RA/` has 264 event files (`MINI_MAP_<digits>_...`, 55 of them `_R`) next to the
three whole-city files. Each is 64 chunks, each chunk one 128 x 128 DXT3 tile named `<file stem>_CHOP<n>`, the name
hash checked for every tile of every file. `gameplay.bin` has 255 race records: 218 that are not pursuit races, of which
212 have their file, and 37 pursuit races. The six without a file: `16.8.1` (a story circuit), `MaxLoad`, `99.1.1`
(online), `20.1.1`, `20.2.2` and `20.5.1` (tests). The 35 pursuit challenges (`19.8.1` to `19.8.69`, `19.9.70`) and
`20.8.1` have no file; `1.8.1` (`epic_pursuit`, a pursuit race) has one that the rule of section 1 never asks for. 51
files belong to no race record (mostly ids with a `_7_` or `_6_` middle number: leftovers).

When the file does not exist the original opens no tiles at all (the loader skips the step; what the four pieces then
show is **[unconfirmed]**). **[ours]**: the rewrite keeps the stage's map for such a race, so the minimap is never blank.

## 3. When the file is chosen

The routine runs once, in the first step of the HUD loader (the state machine that loads the minimap tiles and then the
HUD package), not each frame. It reads the racing state at that moment. **[decomp]** The race is registered in the race
status (`+0x1968`) before the race starts: the start routine sets the racing state to 1 and fires `Game_StartRace`.
Which load of the HUD follows a race start or a race end (the loading screen of a quick race or of a career race, the
return to free roam) is not traced: **[unconfirmed]**.

**[ours]** The rewrite does not reload its HUD. It follows the race instead:

- A race that is not a pursuit race sets the HUD's `map_source` to `Event(<event id>)` when it starts (`RaceSession::map`).
  A pursuit race sets `Variant(<region variant>)`. Without a race the source is `Stage`.
- The HUD reads the tiles of the event on demand, on a worker thread, the first frame the source names it. Nothing is
  read for any other event: the 264 files are never preloaded. A file is about 100 KB and decodes to 64 small
  textures, so the read takes a fraction of a second; until it is done the stage's map stays.
- Once the tiles are in the UI assets, the minimap switches to `MINI_MAP_<event id>`.
- When the race ends, is stopped (`race stop`), is abandoned, or the player quits the world, the source goes back to
  `Stage`: the minimap goes back to the variant it had (the stage's map, or the full map outside career mode) and the
  event's tiles are dropped from the assets. Starting another race drops the old event's tiles and reads the new one.
- An event whose file cannot be read (no file, or a read error) is logged once, not asked for again, and keeps the
  stage's map.

## 4. What the tiles show

**[verified]** (the tiles of `10.2.1`, `10.4.2.r` and `19.8.32` were stitched and looked at, and `10.2.1` was run in the
rewrite): an event map is the whole-city map of `MINI_MAP.BIN` with the race's street route baked in as one bright
white band over the grey roads, the start line drawn as a green bar across the road and the checkpoints and finish as
yellow dashes across it. Only the tiles within reach of the route are present: every other tile of the 8 x 8 grid is
black, so a car far from the route sees an empty minimap. The route is a picture, not a drawn overlay: the HUD does
nothing for it but show the tiles. The Remastered minimap darkens the stage's map with a tint (`remastered.minimap.map`); that
tint turned the white route grey, so a race's own map is shown without it (`DriveFeed::route_map`) **[ours]**.

Nothing else is drawn for the route in the minimap code. The race's checkpoint, racer and event blips are the blips of
[hud-minimap.md](hud-minimap.md) section 7 as before. The GPS is an unrelated feature the player turns on from the
world map ([gps-route.md](gps-route.md)); the executable has the strings `ShowGPS`, `EGPSFinished` and `EGPSLost`, and
`GORacingMiniMap` and `GOExploringMiniMap`, which were not followed: **[unconfirmed]** whether they change the minimap
object's look between a race and free roam.

## 5. Where it is in the code

| What | Where |
|---|---|
| the file stem of an event | `nfsmw_data::minimap::event_stem` |
| which source a race gives | `hud/map_source.rs` (`MapSource::of_race`, `resolve_stem`) |
| the on-demand read and release | `hud/event_tiles.rs` (`EventTiles`) |
| tiles added to the UI assets later | `ui/late_textures.rs`, `UiAssets::add_textures` / `remove_textures` |
| the switch of the minimap's pieces | `MinimapBinding::set_stem`, `hud/plugin.rs` `sync` |
| the race sets the source | `scenes/world/race/start.rs`, `challenge.rs`, `scenes/world/hud.rs` |

## How to check it

- Synthetic tests: `cargo test -p nfsmw hud::` (`map_source`, `event_tiles`, `minimap_variant_tests`) and
  `ui::assets_tests`.
- With the install: `cargo test --release -p nfsmw-data --test real_install minimap -- --ignored` (all 264 files, every
  race's file) and `cargo test --release -p nfsmw a_race_event_has_route_tiles -- --ignored`.
- Offscreen: `nfsmw view-world --drive --exec "race start 10.2.1" --screenshot out.png --screenshot-delay 30` shows the
  route in the minimap (no window is taken).
- In the original: start `10.4.2.r` and `10.4.2` and compare the two route pictures; start a pursuit challenge and
  check which map the minimap shows; start a race with no file (the story circuit `16.8.1`) and see what the pieces
  show.
