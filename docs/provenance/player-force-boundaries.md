# Player force boundary provenance

- Date: 2026-10-26.
- Contracts: drivetrain section 6.4 and suspension sections 2.3, 2.4 and 2.6.
- Original PC analysis SHA-256:
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
- Selected PC executable SHA-256:
  `bde12bdd158b7f861078ad4527f5a656f34c6068e4a2120f28044f92f0fa158c`.
- Unmodified differential helpers `00692140`, `00692380` and `00692690` were
  executed with synthetic wheel interfaces. Direction-clamped axle selection
  differs from clamping the largest-magnitude axle after selection. Write-back
  samples free speed before its writes. Separate joined fixtures also execute
  original suspension feedback getter `0068e570` and raw setter `006a9570`.
  Per-wheel contact getter `006a9650` tests positive compression; cached loaded
  count getter `006a9590` is separate. Do not merge these observables in engine
  coupling and free-wheel limiting.
  Original AutoShift `006af490` was executed with original per-wheel contact
  and signed-slip getter `006a94f0`. It accepts negative slip, rejects slip at
  or above four, and requires all four per-wheel contacts.
- Original spring block `006aa5f5..006aa779` confirms an unconditional damper
  threshold comparison, including zero, and damper plus sway plus spring order.
  Compression block `006aa505..006aa5cf` computes excess travel even for an
  unloaded contact. Recovery tail `006aae0d..006aae72` translates all wheel
  positions and the body in world Y without a loaded-wheel-count condition.
- Original inputs, instructions, harnesses and numeric outputs remain private.
  These are explicitly initialized boundaries, not a full vehicle/world tick.
  Spring fixtures disable digression; compression fixtures do not execute tire
  forces; recovery uses a synthetic body interface. Floating-point modes are
  recorded separately because the live engine's precision word is unobserved.
- Existing captures do not contain every required pre-tick engine, suspension,
  pending body force and contact/terrain state. Missing fields must not be
  reconstructed and labeled exact. Complete closed-loop driving parity remains
  open, independently of passing isolated boundary comparisons.
