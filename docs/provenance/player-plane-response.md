# Player plane-response provenance

- Date: 2026-10-26.
- Spec: [player plane response](../specs/player-plane-response.md).
- Original analysis executable SHA-256:
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
- Selected executable SHA-256:
  `bde12bdd158b7f861078ad4527f5a656f34c6068e4a2120f28044f92f0fa158c`.
- Fresh program identity and isolated, unchanged instruction execution compare
  PC plane response `00488830` with object response `00488f70` using a stationary
  immobile partner. These are separate original routines.
- Raw plane instructions `00488a86..00488a9e` project Z/Y before X. Initial
  closing telemetry uses that order at `00488b4e..00488b68`. Entry helper
  `00488680` normalizes the plane with a scalar reciprocal-multiply sequence.
  A first interpretation that omitted this helper was rejected before final
  validation. The object routine retains its distinct projection order.
- Fifty-four synthetic cases in two declared FPU modes cover ordinary, sliding,
  rotating, separating, stationary and near-cancellation contacts. The original
  plane and immobile-object substitution disagree on eight single-precision
  acceptance results. Production comparison seeds exact synthetic entry state;
  constructor rounding is excluded as a confounding input.
- The shared friction helper's raw squared-length instructions retain X/Y then
  Z. A decompiler's reordered expression is not used to justify a change.
- Original bytes, private fixtures and bridges remain outside public sources.
  No game process or installed files are changed by isolated execution. Sources
  for names and layouts remain those credited by the existing contact specs.
- Follow-up live boundaries retain 95 complete Carrera GT plane calls: 84
  reactions and 11 rejections. The production solver matches their acceptance
  and final linear/angular velocities exactly. Unchanged original instructions
  reproduce the complete recorded moment outputs in single precision. The
  live FPU control word was not directly observed.
- Terrain footer `00789d5f..00789d81` combines signed separation in Z/Y/X order.
  Independent original selection and production replay use 7,872 recorded
  candidate/cache boundaries. Face identity and normals were already exact;
  the former generic-dot separation retained small scalar residuals.
- The live observer was intrusive and the user reported severe FPS loss. These
  records support isolated collision/selection comparisons, not normal-FPS
  cadence or full-route acceptance. An incomplete final outer frame is excluded
  from complete phase claims. Private data and observer diagnostics stay local.
