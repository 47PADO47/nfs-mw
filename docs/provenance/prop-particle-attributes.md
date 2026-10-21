# Prop particle wrapper and atlas interpretation

- **Spec:** [Prop particle wrapper and texture attributes](../specs/prop-particle-attributes.md).
- **Sources:** read-only inspection of the user's PC 1.3 executable on
  2026-10-20; installed attribute and reachable scenery-sequencer records; and
  the CC0 [dbalatoni13/nfsmw reference](https://github.com/dbalatoni13/nfsmw),
  `World/WorldConn.cpp`, `Sim/Common/SimEffect.cpp`, `Ecstasy/EmitterSystem.cpp`,
  and the generated effects/emitter/particle-animation type declarations.
  Binary identity, raw instructions and extracted attribute output stay private.
  No game assets, executable bytes or decompiled code are included here.
- **Established:** wrapper intensity is a clamped cubic in magnitude length;
  wrapper velocity inheritance uses owner linear velocity and precedes the
  emitter's separate motion inheritance. Group ranges are inclusive eligibility
  intervals, with different defaults for invalid ranges and mismatched counts.
- **Atlas evidence:** the PC animated-UV routine's instructions establish
  square-cell selection and packed 16-bit coordinates. The particle update
  instructions establish its quantized phase increment, double-increment wrap
  test, narrowing and cell selection. The floating-point conversion helper is
  not an RNG; decompiler output that labels these conversions as opaque calls
  must not replace the instruction-level arithmetic.
- **Named-position follow-up:** the PC object-effect handler calls the model's
  collision-geometry interface and its immediate-child name lookup. The helper
  compares the serialized hash directly against bounds names and returns no
  result after the immediate child range. The matching `IModel.h`,
  `DrawVehicle.h`, `DrawVehicle.cpp` and `Bounds.h` identify these interfaces and
  geometry ownership. Render-solid position markers and recursive descendant
  searches are not part of this path; a missing named collision child skips the
  effect rather than supplying a guessed origin.
- **Installed coverage:** reachable scenery effects contain random static atlas
  cells, animated debris and animated sparks. This establishes that atlas support
  affects ordinary props, rather than only synthetic profiles. The inspected
  shared particle and world metadata packs contain no texture-page-range chunks;
  this is a bounded inventory, not proof that all installations or packs omit
  that format.
- **Validation:** use synthetic polynomial and interval fixtures, independent
  wrapper/emitter inheritance multipliers, atlas birth states and exact phase
  boundaries. Retain the existing native phase-wrap behavior. Keep lifetime,
  emission-count and scheduling evidence in their own specifications.
- **Limits:** no matched live original-game prop particle capture was recorded
  in this investigation. Final rendering and full effect-service parity remain
  separate from these attribute and atlas rules.
