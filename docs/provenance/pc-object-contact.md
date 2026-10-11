# Object contact response

- **Spec:** [PC object contact response](../specs/pc-object-contact.md).
- **Sources read for the spec:** the user's PC 1.3 executable (hash in the spec),
  privately inspected with Ghidra; [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw)
  (decompiled, CC0), collision moment layouts and rigid-body attribute names.
- **Implemented:** 2026-10-11, from the written spec with the decompilation closed.
  No executable bytes or decompiled code were copied into the implementation.
- **Checked against the game by:** executing the original machine code in an
  isolated x86 emulator with 13 synthetic contacts and comparing the Rust solver's
  resulting linear/angular velocities, normal impulse, closing/sliding speed,
  friction state and attachment release. The checked executable's SHA-256 is
  recorded in the spec; numeric observations are retained as unit-test assertions.
  Velocity tolerance is 0.003 m/s or rad/s; impulse tolerance is 0.03 N s.
- **Known limits:** these are isolated contact-response comparisons. They do not
  establish complete scenario parity, correct collision geometry, world contacts,
  vehicle-specific overrides, prop lifecycle, trailer cargo or effects.
