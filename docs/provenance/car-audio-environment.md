# Car audio environment provenance

Behavioral specification: [car-audio-environment.md](../specs/car-audio-environment.md).

Sources read for understanding:

- [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0-1.0,
  decompiled platform reference: engine/tunnel/hybrid/reverb controllers,
  generated engine class interfaces, authored environment target declarations,
  common sound-library effect bus. No code copied.
- A matching original PC executable: bounded full engine parameter producer,
  distance-filter controller, environment transition routine, global wet map
  scaling, effect-bus setters and custom-effect allocation/service routines.
  Private identity and disassembly remain outside the public repository.
- Installed assets from the local original-game copy:15FXEDIT files parsed
  structurally to their exact ends; class/bank paths are read from users'
  installs, never distributed. Processor parameters and binary dumps remain
  private.

The PC producer resolves signed CAR dry semantics and missing environment
assignments that the console reference does not establish. The PC target table
also resolves platform differences in tunnel/hills/city values. Nine distance
vectors were reproduced by executing the complete original math routine in
isolated memory with synthetic controller input and unmodified callee code.

Saved original live-player observations show varying requested dry/wet,
azimuth and filtering for CAR/CAR_TRANNY. They do not directly recover the
class inputs or prove effective wet DSP in the recorded output. Existing probe
identity/quarantine limitations continue to apply. No new live capture,
listening equivalence, or original effect-kernel numeric replay is claimed.

Remaining dependencies: authored spatial Reverb-map/track-path-zone adapter,
complete FXEDIT parser/graph connection semantics and processor kernels,
original active-patch/service evidence and controlled DSP/PCM comparisons.
Implementing dry/pan/filter producer wiring does not depend on claiming these
gaps are resolved.

The current producer implementation uses the existing mathematical mixer
conversion, whose small Q15 differences from the original gain table are
documented in the spec and bounded by3Hz on the nine cutoff vectors.
Extracted binary tables were not copied into source. Ginsu dry quantization
and azimuth are carried to the existing output backend; center-preserving
constant-power pan is independently tested, while original speaker pan law
and DSP/filter coefficients remain unverified. CAR's hybrid random-filter
flag stays explicitly0 until its complete initial-state producer is available.
