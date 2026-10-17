# Free-roam pursuit integration

Date: 2026-10-17. Spec: [pursuit-free-roam](../specs/pursuit-free-roam.md).

The game integration is written from that spec and the existing pursuit,
damage, road-network and music specs. Generic spike geometry and helicopter
sight state remain in `blackbox-pursuit`; installed-data and world decisions
remain in `nfsmw` / `nfsmw-data`.

Additional sources read for understanding, without copying or translating code:

- [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0-1.0:
  reference revision `80993f35224c61d2f1b135beba76c6f8234c7940`;
  `src/Speed/Indep/Src/Frontend/HUD/FeRadarDetector.cpp` and `.hpp` (object
  identities, visibility, LED reveal and sound-message timing);
  `FeHeatMeter.cpp` (label identity and incomplete arc update);
  `AI/Activities/AICopManager.cpp` (separate patrol count and lockout context).
- Further source-level correction uses `AICopManager::UpdatePatrols` and
  `SpawnPursuitIVehicle`, `AISpawnManager::{GetSpawnLocation,CheckSpawnPosition}`,
  `AIActionTraffic::{AIActionTraffic,CanBeAttempted,BeginAction}`, and
  `AIPursuit`'s tracked-sight update at that same revision. These establish the
  density-independent patrol target, normal 35/55 mph patrol speeds, seven-second
  tracked-sight window, traffic-lane search action and cooldown replacement
  spawning. Sources were also checked through the public upstream repository;
  no reference implementation is copied or translated.
- `AIVehicleCopCar::CanSeeTarget` establishes fallback 150/50 metre patrol
  ranges, the per-cop hiding latch, and three-dimensional range plus world
  occlusion without the busted-distance vertical gate. These are source-level
  observations pending the matching PC binary checks recorded privately.
- The repository's existing specs, especially
  [ai-pursuit-heat](../specs/ai-pursuit-heat.md), record the installed
  `NumPatrolCars` values and the original heat reset after a paid ticket.
- The matching PC executable's local-player radar feeder and detector update
  confirm the 300-metre full-3D distance, 45-degree forward cone, inclusive
  ground-target boundary, lack of a separate deck/sight-ray gate, 0.1 range
  floor and 0.1-per-HUD-update LED reveal. The pursuit-board update confirms
  the signed five-segment formulas and fixed fill edges. These contracts were
  recorded in [hud-pursuit-state](../specs/hud-pursuit-state.md) before the
  corresponding corrections; no native instructions or decompiled code are
  included in the implementation.
- The matching PC patrol manager, traffic-action entry/eligibility and pursuit
  task corroborate the independent count, search-action gate and tracked-sight
  timer. Heat growth is gated on pursuit state, continuing through sight grace
  and pausing in cooldown. Executable SHA-256:
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
- The native local-player radar feeder reads rigid-body position/forward, and
  the referenced pursuit actions query target physical interfaces. An
  integration audit found player model/interpolation coordinates compared
  against cop body coordinates. The frame-boundary correction consistently
  feeds current body position, forward and velocity; it does not change
  visual model pivots or camera interpolation.

Installed-data correction: the PC `MW_Music.mpf` with SHA-256
`15c7fdaa626940319a74965d4d68db1507475ba743d574c5aa6de214482e785b`
has 3,681 nodes. First pursuit-section heads are 0/427/755/1127. The earlier
409/106/306/213 entry table names four heads of section 1 and can enter a
single-bar ending. The section-head discovery is an independent host policy,
validated for sustained graph traversal and decoding, without claiming to
implement the original event/variable controller.

Source-level reference reading and isolated native checks are not live
observations of the original gameplay. Roll-sensitive radar arrow orientation,
click routing, speed-camera targets, heat-arc masks, individual search routing
and patrol lockout/911 flow remain incomplete or unverified. The implementation's
regressions and installed checks are listed in [pursuit-system](../pursuit-system.md).
