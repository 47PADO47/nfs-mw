# Speedbreaker audio and presentation

- Spec: [speedbreaker-audio.md](../specs/speedbreaker-audio.md).
- Sources: matching original PC Speedbreaker event constructor and main volume
  controller instruction ranges; installed mixer map input/event/output routing;
  [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0-1.0,
  `SFXCTL_MasterVol`, `CARSFX_Engine`, `SFXObj_MomentStrm`,
  `SFXObj_PFEATrax`, `FESpeedBreakerMeter`, `VisualTreatment` and dynamic mixer behavior.
- A follow-up checked the native begin/end dispatch, six input-8 event routes,
  Ginsu pitch and volume consumers, music filter-stage configuration and native
  HUD update. Thirteen isolated original HUD vectors execute its unchanged
  arithmetic and script thresholds with synthetic FEng accessors. They do not
  establish rendered screen equivalence.
- The PC camera drive update and raw camera-anchor instructions confirm both
  spatial LAG division and field-of-view multiplication by the breaker factor.
  The Cubic pose-target consumer uses LAG as the local forward eye offset,
  with HEIGHT unchanged and look height derived from ANGLE and LAG. The earlier
  FOV-only integration omitted this camera displacement. The main animation caller supplies accumulated
  real frame time, independently of simulation service time. The retained
  remake blend, installed POV target and custom-camera adapter follow this
  scoped contract. Native Cubic follow, speed graph and obstruction behavior
  remain separate camera parity gaps.
- The native player event calls HUD FadeAll on entry/exit. The matching native
  FadeAll routine posts fade-out plus ACTIVATE on entry and fade-in plus
  DEACTIVATE on exit. These package messages were previously missing even though
  charge arithmetic tests passed; installed active-layout checks cover them.
- The PC producer is checked using isolated execution with synthetic state in
  normal/held modes; these branches need no message-service stubs. Original bytes
  and derived vectors remain private.
- Implementation follows this prior behavioral spec. Shared event snapshots and
  tracing are independent remake integration decisions. No extracted map values,
  original code, binaries or recordings are included in public source.
- High-FPS/controller and installed playback regressions establish scoped
  behavior; remaining mode, filtering and audible equivalence limits are tracked
  separately in the [coverage ledger](../car-audio-coverage.md).
- A further PC consumer audit corrected the music path: the stream setter
  receives packed mixer output9, whose installed master consumes main event6.
  The separately updated internal630→1800 Hz fader is not read by that path.
  The earlier host direct-fader consumer has been removed. The stream/platform
  chain reaches the same first-order software filter as FXEDIT ID1; unchanged
  native setter/coefficient/sample replays establish impulse, retarget, zero,
  bypass and fresh-history re-entry vectors at44.1/48kHz. This does not certify
  original per-stream allocation or matched whole-game PCM.
