# Traffic and trailer suspension evidence

Specification: [traffic suspension runtime](../specs/traffic-suspension-runtime.md),
written before implementation.

- Installed attribute census independently identifies traffic, tractors,
  trailers and police suspension mechanics. The string-key's behavior hash
  selects `SuspensionTraffic`, `SuspensionTrailer` and `SuspensionSimple`
  respectively; names, masses or appearance do not select the formulas.
- Read-only PC analysis used executable SHA-256
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
  Tire entries `0x68F7B0` and `0x6906B0` confirm the distinct traffic/trailer
  responses. Synthetic tire objects were executed in isolated x86 memory;
  no game process was launched, attached or patched.
- The trailer table object normally initialized at process startup was supplied
  as a synthetic seven-sample table descriptor. The original table lookup and
  tire routine then ran unchanged. Comparisons cover straight, cornering,
  low-speed, reverse, handbrake, previously locked, high-load and powered cases.
- The CC0 [dbalatoni13/nfsmw reference](https://github.com/dbalatoni13/nfsmw),
  `Physics/Behaviors/SuspensionTraffic.cpp`, `SuspensionTrailer.cpp` and
  `RBTractor.cpp`, corroborates behavior identity, wheel torque, simple drag,
  contact lever arm, trailer centre of gravity and release controls. Its existing
  NOTICE credit applies. No original code or assets are included.
- The prior shared racing-tire adapter supplied a different force curve,
  regular-brake locking, wheel-slip history and steering assumptions. Ordinary
  traffic also inherited the police support-dependent ride offsets and braking
  steering range. These are corrected as behavior selection errors, without
  changing installed mass, tensor, spring or brake values.
- Numeric comparisons allow the documented original angle-lookup approximation;
  full driving routes, powertrain fidelity, collision ordering and all authored
  mod replacements remain separate validation boundaries.
- Native `atan2` initially missed the high-load traffic vector's longitudinal
  force by 1.247 N. The shared angle adapter `0x5CB5D0` and integer-angle routine
  `0x45DED0` explain this: their result is a quantized 16-bit turn. Independently
  generating the 257 mathematical octant nodes with nearest-integer rounding
  matches every original node. The runtime generates these mathematical samples
  itself and interpolates integers; no original table bytes are embedded.
- A separate isolated matrix of 1,108 synthetic angle pairs matches the original
  integer-angle routine exactly, with zero mismatches. The generated octant
  table also matches every one of its 257 nodes.
- Offline PE section mapping compares these routines against the differently
  hashed selected executable
  `bde12bdd158b7f861078ad4527f5a656f34c6068e4a2120f28044f92f0fa158c`:
  traffic tire (772 bytes), trailer tire (839), integer angle (240), its wrapper
  (127), and octant samples (514) are byte-identical. This establishes only the
  compared static file ranges, not the behavior of possible live injected mods.
