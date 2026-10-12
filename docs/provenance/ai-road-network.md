# AI road network, navigator and path finding

- **Spec:** [specs/ai-road-network.md](../specs/ai-road-network.md),
  [specs/ai-road-nav-trail.md](../specs/ai-road-nav-trail.md),
  [specs/ai-pathfinder.md](../specs/ai-pathfinder.md); data layout in
  [formats/road-network.md](../formats/road-network.md).
- **Sources read for the spec:** dbalatoni13/nfsmw (CC0-1.0, decompilation; read-only reference, no code
  copied): `src/Speed/Indep/Src/World/WRoadElem.h`, `WRoadNetwork.h`, `WPathFinder.h`, `TrackPath.hpp`,
  `TrackPath.cpp`, `World/Common/WRoadNetwork.cpp`, `World/Common/WPathFinder.cpp`, `World/Common/WGrid.h`,
  `World/Common/WGridNode.h`, `Misc/CookieTrail.h`, `Libs/Support/Utility/UGroup.hpp`,
  `Libs/Support/Utility/USpline.h` (header only; the body is not in the decomp), `AI/Gps.cpp`, and the call
  sites in `AI/Common/AIVehicle.cpp`, `AI/Actions/AIActionRace.cpp`, `AI/Actions/AIActionTraffic.cpp`.
- **Written:** 2026-10-09 (research only). **Implemented:** not yet.
- **Checked against the game by:** throwaway Python scripts over `TRACKS/L2RA.BUN` (not committed) that walk
  the `UGroup` tree and decode the records: record sizes equal count × element size for all five records;
  node and segment indices equal array positions; all profile indices valid; flag statistics, the junction
  structure (731 groups, 711 complete), lane and zone statistics, the grid's road-segment lists (every
  segment referenced), the Bézier hypothesis (arc lengths of 5,892 curved segments reproduce the stored
  lengths to 0.06 % on average), the zone and barrier tables (705 zones, 1,376 barriers consumed exactly) and
  the traffic-pattern zone hashes. A dead-end node lies 20 m from a position the decomp hard-codes in
  physics space, which confirms the coordinate space. Behaviour (navigation, trail, search) has **not** yet
  been compared with the running game.
- **Known gaps / differences from the original:** the spline tangent and curvature formulas, the lane
  offset sign convention against world geometry, the `LaneMap` segment flag, the road length / width scales
  and the zone `Data` fields other than the traffic-pattern hash are unconfirmed (see the open-question
  sections). Hole punching is marked unfinished in the decomp.
- **Not copied:** no decompiled text, no game data. Numbers in the docs are layouts, counts and tuning
  constants.
- **PC transition follow-up (2026-10-12, static):** Ghidra inspection of the original x86 PC executable
  (SHA-256 `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`), using the PC symbol
  map only to identify routines, confirmed traffic endpoint carry-over in `PrivateIncNavPosition`
  (`0x7755a0`), the always-preserve-start result and far-node lane selection in `GetNextTraffic`
  (`0x774c50`), and handle scaling in `SetControlPos` (`0x76f140`). `EvaluateSplines` (`0x7753e0`)
  and its point evaluator (`0x773fd0`) also confirm linear interpolation and zero curvature on uncurved
  segments. The previous lane and both corridor
  endpoints become the next segment's start; its destination offset comes from its far-node profile.
  Synthetic tests exercise turns and changed zone indices. Installed-data tests measure all legal
  plain-road exits. These check the reconstruction; they do not establish visual or runtime parity.
