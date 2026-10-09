# blackbox-roads

The **road network** of EA Black Box games (Need for Speed: Most Wanted today): the lanes, roads and
junctions that traffic, racers and cops drive on.

- `RoadNav`: a cursor on a lane that advances along the graph (traffic and direction rules, lane types).
- `lane_line`, `closest_segment`: lane geometry and "which road am I on".
- `RoadNetwork` (the `RNgp` group of the `0x3B800` world map blob): nodes, segments, profiles (lane layouts)
  and roads. `RoadNetwork::read(&bytes)` takes the track's world metadata file.

Takes bytes, never opens files. Coordinates are the game's physics space (x right, y up, z forward).

Specs: `docs/formats/road-network.md` (the data), `docs/specs/ai-road-network.md` (lane geometry and the
navigator). Tests: synthetic data always; the `#[ignore]`d ones read an install (`NFSMW_GAME_DIR`).

License: MIT OR Apache-2.0.
