# blackbox-roads

The **road network** of EA Black Box games (Need for Speed: Most Wanted today): the lanes, roads and
junctions that traffic, racers and cops drive on.

- `RoadNav`: a cursor on a lane that advances along the graph (traffic and direction rules, lane types).
- `lane_line`, `closest_segment`: lane geometry and "which road am I on".
- `RoadNetwork` (the `RNgp` group of the `0x3B800` world map blob): nodes, segments, profiles (lane layouts)
  and roads. `RoadNetwork::read(&bytes)` takes the track's world metadata file.
- `TrackZones` (chunk `0x3414A` of the track path manager): the typed polygons that tag areas of the map
  (traffic patterns, tunnels, no-spawn areas...). `TrackZones::read(&bytes)` takes the same file;
  `first_of_kind_at(kind, point)` / `contains_point(kind, point)` do the point-in-zone test, and
  `to_zone_space` / `from_zone_space` convert between physics space and the 2D frame the zones use
  (physics `x = -y2d`, `z = x2d`).

- `SignalController`: traffic lights for the junctions, an **extension** (the original has none). It groups the
  nodes joined by decision segments into junctions (731 in the install), gives every road that enters a junction
  of three or more roads a stop line, a heading and one of two phases, and answers `state(approach, time)` with
  green, amber or red from a fixed-time cycle (timing in named constants, a per-junction offset).

Takes bytes, never opens files. Coordinates are the game's physics space (x right, y up, z forward), except the zones, which use the 2D frame
above.

Specs: `docs/formats/road-network.md` (the data), `docs/specs/ai-road-network.md` (lane geometry and the
navigator). Tests: synthetic data always; the `#[ignore]`d ones read an install (`NFSMW_GAME_DIR`).

License: MIT OR Apache-2.0.
