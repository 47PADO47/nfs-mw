# Countdown drivetrain provenance

- Date: 2026-10-26.
- Spec: [countdown drivetrain comparison](../specs/player-countdown-drivetrain.md).
- Behavioral reference: independently inspected PC v1.3 engine tick and the
  previously recorded original Golf engine callbacks. Decompiled material and
  original machine code remain private; neither is included in this repository.
- Selected executable SHA-256:
  `bde12bdd158b7f861078ad4527f5a656f34c6068e4a2120f28044f92f0fa158c`.
  Analyzed clean reference SHA-256:
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
  Program identity and selected executable hash were rechecked before queries.
- The engine tick at `006af740` tests clutch state for the grounded wheel-slip
  branch at `006afd18`. The path for other states joins at `006afddf`, where the
  clutch-play branch tests only first/reverse gear. Its negative net-torque gate
  therefore applies while engaging too. A later road-torque relaxation branch
  separately requires the engaged state. This distinction also appears in
  [EngineRacer.cpp](https://github.com/dbalatoni13/nfsmw/blob/80993f35224c61d2f1b135beba76c6f8234c7940/src/Speed/Indep/Src/Physics/Behaviors/EngineRacer.cpp),
  used as a behavioral reference rather than copied implementation.
- Independent original subblock execution covers 12 gear/clutch-state pairs:
  total engine torque 128 N m, road torque -256 N m, clutch slip 0.5 yield
  road torque -320 N m in first/reverse for all three clutch states. Neutral
  and second gear retain -256 N m. This is synthetic boundary-state evidence,
  not full-engine or trajectory acceptance.
- The same PC engine tick clears previous clutch RPM difference in engaged
  (`006afc1c`) and disengaged (`006afb4d`) branches. Only the engaging branch
  records transmission-minus-engine RPM difference (`006afc06`). The former
  unconditional Rust history update could carry a stale sign across engagement.
- Both recorded staged sequences contain 372 ticks and remain in first gear.
  They include disengaged, engaging and engaged clutch states. They are useful
  state/ordering evidence, not a complete engine replay fixture: exact wheel
  velocities at engine entry and some engine dependencies were not captured.
- Limits: the gate mismatch is established. Its share of the user-reported
  countdown rev defect is not yet measured. No full race-start parity claim.
- Follow-up: PC program identity was rechecked before inspecting the RPM
  smoothing helper. The engine supplies an active-shift or fully-disengaged
  clutch flag. The helper selects its faster decay only above first gear,
  or unconditionally in neutral. Rust omitted the disengaged-clutch part of
  this flag. The same independently inspected
  [behavioral reference](https://github.com/dbalatoni13/nfsmw/blob/80993f35224c61d2f1b135beba76c6f8234c7940/src/Speed/Indep/Src/Physics/Behaviors/EngineRacer.cpp)
  corroborates the caller and helper gates. Private decompilation is retained
  outside this repository. First-gear countdown decay is unaffected by this
  higher-gear correction; full countdown producer acceptance remains open.
