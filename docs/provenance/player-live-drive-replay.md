# Player live-drive replay provenance

- Date: 2026-10-26.
- Spec: [live-drive replay](../specs/player-live-drive-replay.md).
- Original analysis executable SHA-256:
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
- Selected executable SHA-256:
  `bde12bdd158b7f861078ad4527f5a656f34c6068e4a2120f28044f92f0fa158c`.
- A passive single-car drive retained 22,194 updates, including 5,551 detailed
  body/engine/suspension checkpoint sets and 21,364 loaded-tire calls. No input
  stimulation, capture errors or dropped events were reported. Exact current
  engine-entry tire state replaces the earlier previous-force proxy.
- PC AutoShift `006af490` consumes full-engine cached potential at `+188`.
  DoShifting `006af670` runs it before the shift timer and clutch update.
  Clutch helper `00691a10` retains a negative expired timer. Engine callback
  `006af740` refreshes the cached request after torque and displayed RPM.
- The PC transmission-speed expression at `006afa82..006afa96` divides the
  idle/redline span first. Captured cases show that the former remake order can
  miss the torque cut at the strict redline boundary.
- Pilot helper `0068e180` operates on body speed in metres per second, with
  converted threshold/span constants and the complement of the 0.85 base.
  Independent original-instruction fixtures and full captured-tire replay
  identify the separate rounded mph conversion as the main rear-force residual.
- Original vector helper `00401670`, called by body integration `006978f0`,
  accumulates Z/Y then X. An isolated-source candidate makes all captured body
  angular-velocity outputs exact while leaving pose-cache residuals documented.
- Original wheel-torque block `006aac84..006aad31` subtracts rotated COG and
  body position separately. Independent synthetic instruction fixtures verify
  this grouping. Captured force assembly retains drift torque as a separate
  producer; it must not be mistaken for a wheel-force error.
- Original code, memory snapshots, numeric corpora and private replay bridges
  remain outside public sources. Specifications describe behavior; no original
  implementation text is copied. Supporting decompilation references are those
  credited by the existing drivetrain and suspension specifications.
- Follow-up NOS audit: PC DoNos `00692930` uses the speed factor stored at
  `008967e0`, single-precision `2.23699`, rather than the generic display factor.
  Unmodified original conversion/recharge instruction blocks reproduce all
  1,045 observed positive tank increments in each of two FPU precision modes.
  Independent directed cases distinguish the activation and recharge thresholds.
- Joined replay follows production body integration into the production wheel
  loop. PC wheel position `00671530` and wheel direction `0068d6a0` call the same
  ordered vector helper. A private candidate removes a 27.19-N rear-force
  discontinuity in the recorded-body wheel comparison; tiny intermediate
  differences had crossed a quantized tire-angle boundary. Complete body pose
  propagation and world callbacks remain separate limitations.
- Pose follow-up: original quaternion multiplication `004866c0` and body update
  `006978f0` retain cross-product grouping, W/Z/Y/X normalization sum order,
  reciprocal square root and multiplication. The scalar f32 candidate matches
  all 5,551 captured orientations; original matrix conversion `005ca910`
  independently reproduces all captured rotation matrices from those values.
