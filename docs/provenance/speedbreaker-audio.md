# Speedbreaker audio pitch

- Spec: [speedbreaker-audio.md](../specs/speedbreaker-audio.md).
- Sources: matching original PC Speedbreaker event constructor and main volume
  controller instruction ranges; installed mixer map input/event/output routing;
  [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0-1.0,
  `SFXCTL_MasterVol`, `CARSFX_Engine` and dynamic mixer behavior.
- The PC producer is checked using isolated execution with synthetic state in
  normal/held modes; these branches need no message-service stubs. Original bytes
  and derived vectors remain private.
- Implementation follows this prior behavioral spec. Shared event snapshots and
  tracing are independent remake integration decisions. No extracted map values,
  original code, binaries or recordings are included in public source.
- High-FPS/controller and installed playback regressions establish scoped
  behavior; remaining mode, filtering and audible equivalence limits are tracked
  separately in the [coverage ledger](../car-audio-coverage.md).
