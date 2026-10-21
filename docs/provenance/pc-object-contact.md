# Object contact response

2026-10-20 numeric construction follow-up: fresh identity-checked PC helper
inspection and isolated execution establish the original vector-transform and
inverse-world-tensor accumulation orders. The relevant helpers are byte-identical
in both checked installations. Substituting only generic-order contact arms in
one recorded original contact changes reaction acceptance; substituting only
generic-order tensors explains the previously documented amplified angular
response difference. The implementation shares ordered generic helpers between
body contact construction and the portable comparison driver. Synthetic helper
fixtures retain measured float results without captured game records or assets.

2026-10-20 loop follow-up: tracing the actual x87 instruction stack corrects a
previous decompiler-expression interpretation: momentum projection adds x/z
before y. A second independently generated cancellation control distinguishes
this order from the earlier implementation. Scalar rounding controls isolate
the small captured sliding-impulse residuals to that projection and the normal
impulse's vector-before-fraction scaling. Angular increments use the same
ordered vector transform as the original helper. The complete recorded corpus
must still be replayed after building; isolated controls do not certify broad
gameplay or complete floating-point parity.

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
# Collision notification follow-up

Read-only inspection of the matching PC executable on 2026-10-20 confirms the
dispatcher selects the partner moment's reciprocal mass for an immobile body's
callback. The matching generic implementation exposes notification intensity
separately from physical response. Prop audio, impact callbacks and damage filters
receive the notification while temporary immobility preserves car velocity and
steering. Synthetic coverage checks both body orders and contact mass overrides.
This follow-up does not certify live camera or sound parity.
