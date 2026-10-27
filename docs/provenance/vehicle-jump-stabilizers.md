# Jump stabilizers and ground reference

- **Spec:** [vehicle-jump-stabilizers](../specs/vehicle-jump-stabilizers.md).
- **Sources:** original PC 1.3 executable, proprietary behavior reference;
  [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0 files listed in
  the spec. Read for understanding; no code copied.
- **Inspected:** 2026-10-13, executable SHA-256
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
- **Evidence:** static original jump-stabilizer and world-position routines,
  reference rigid-body end-frame update lifecycle and vehicle contact query.
  Eight synthetic plane/body cases replayed through the original PC stabilizer
  and math helpers in isolated x86 memory verify force, passive damping and
  active pitch/roll results. Virtual body queries and graph construction are
  supplied fixtures; live-jump behavior is not established.
- **Implementation boundary:** ground reference acquisition and its persistent
  cache are separate from the force/torque consumer. A center ray or wheel-plane
  average cannot be presented as the original world-position behavior.
- **Consumer implemented:** generic `blackbox-vehicle::vehicle::jump`, with
  caller-supplied coefficients and optional terrain reference. Defaults apply
  no forces. The game adapter supplies original tuning and the preceding tick's
  final-position cached body terrain reference; standalone unknown ground
  disables terrain-dependent terms. Separate synthetic tests compare
  all eight recorded original force/torque vectors and neutral/unknown inputs.
- **Query implementation:** see [ground reference](vehicle-ground-reference.md)
  for ordered face selection, predictive region and retained-plane evidence.
  The free-roam adapter disables scenery groups and uses the original
  per-driver instance masks; dynamic group transitions remain a boundary.
- **Validation still needed:** original runtime jumps/landings and query behavior
  at bridge edges and stacked surfaces. No complete original jump parity claim.

## Retained boundary arithmetic audit, 2026-10-26

Original instruction replay reproduces all 72 force-changing retained jump
boundaries exactly with captured interface inputs in the single-precision FPU
mode. The initial direct production comparison matches 50 of 72 exactly; its
maximum torque component residual is 0.001708984375 Nm. Forces and jump timers
match. A separate scalar model reproduces every production result, isolating
rotation, dot, magnitude and accumulation grouping as the residual sources.

The corrected ordered model matches all 72 original outputs exactly. Actual
Rust validation remains a separate check; a finite-output measurement run alone
does not establish parity. The generic numeric replay adapter contains no
recorded values. Cache selection and full trajectories remain outside this
helper boundary.

The corrected production replay matches all 1,967 complete retained jump calls
exactly for force, torque and both timers: 72 active calls and 1,895 no-op calls.
Original instruction execution separately reproduces the observed accumulators
and timers in all 1,967 calls in the declared single-precision mode. The numeric
adapter now requires exact equality after reporting residuals; finite output
alone cannot pass its boundary checks. Complete producer and route closure
remain outside this comparison.
