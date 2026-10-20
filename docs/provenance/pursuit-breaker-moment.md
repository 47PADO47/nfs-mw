# The cinematic moment of a pursuit breaker

- **Spec:** [pursuit breaker moment](../specs/pursuit-breaker-moment.md). Format: [animation](../formats/animation.md)
  ("ICE camera keys"), [ICE dispatch](ice-camera-dispatch.md).
- **Sources:** installed PC 1.3 `GLOBAL/InGameB.bun` (ICE camera packs, the scene candidate table), `TRACKS/L2RA.BUN`
  (track position markers); private `speed.exe` inspection (Ghidra); the CC0 `dbalatoni13/nfsmw` reconstruction for
  structure names, the `ICEData`/`ICETrack` field layout, `NISActivity`, `CAnimMomentScene`, `CAnimCandidateData` and the
  cop manager. No original code, assets or camera data is included in the repository.
- **Addresses read in `speed.exe`:** the `EPlayRaceNIS` handler `0x640F00` (the gate), `NISActivity::Load` `0x6F51F0`
  (marker choice, scene creation, `EBecomeAiCar`), `UpdatePlaying` `0x6EA750` (the clock), `Release` `0x6F7520`
  (`EBecomePlayerCar` for type 4), `GetClosestMarker` `0x4491E0`, the scene constructor `0x6EF2C0` (length from the track),
  `ICEManager::GetCameraData` `0x47B3F0` and the track lookups `0x47B030`, `0x47B150`, `0x46D2E0`, the playback routines
  `0x482160` (curves from a key), `0x482EC0` (per-frame camera), `0x47F490` (tangents), `0x46D5E0` and `0x45DED0`
  (lens and angle), `0x46DD10`/`0x46DDD0`/`0x46DE90` (curve evaluation), `0x47EF70` (track choice, 0 below two tracks),
  `GetInitialAnimMatricies` `0x446F00` and the D3DX wrappers `0x6BD900`/`0x6BD940` (row-vector order), the HUD group
  visibility routine `0x5FDAE0`, and the trigger event destructor `0x63BE40` (the 2-second touch clock).
- **Checked against the install:** all 14 moment scenes have a candidate kind and `Track 0` (one key); the gas station
  has 9 markers (the track has 9 gas stations); the water tower track has simulation speed 60; the marker records parse
  with names matching the 21 kinds. Installed tests: `moment_tests::installed_moment_scenes_have_markers_and_a_camera_track`
  and the framing test in `scenes::world::moment_tests`.
- **Implementation:** 2026-10-20, from the spec. `libs/blackbox-chunk/src/ice/track.rs` (keys and evaluation),
  `crates/nfsmw-data/src/world/moment.rs` (tables), `crates/nfsmw/src/scenes/world/moment.rs` (rules, camera),
  `moment_world.rs` (the world's side), `drive/touch.rs` (the touch clock), `ai/moment_feed.rs` (chasing cops, busted
  hold).
- **Limits:** see the spec's section 6. Not seen in a running game.
