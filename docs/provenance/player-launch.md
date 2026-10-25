# Player launch provenance

- Date: 2026-10-25.
- Spec: [player launch](../specs/player-launch.md).
- Sources: locally inspected original PC v1.3 executable, with isolated execution
  of its unchanged launch-band, staging-strength and timer-decay routines using
  synthetic attributes and interfaces. Proprietary code is a behavioral
  reference only; no executable bytes or decompiled code are included.
- Selected executable SHA-256:
  `bde12bdd158b7f861078ad4527f5a656f34c6068e4a2120f28044f92f0fa158c`.
  Ghidra reference SHA-256:
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
  A previous complete file comparison established identical code sections and
  one differing large-address-aware PE flag byte.
- Band consumer: IRaceEngine routine `006a02f0`; vehicle staging routine
  `006702d0` obtains its output and displayed engine RPM. Vehicle release
  `006700e0` starts the timer; getter `00669020` gates effective strength.
  Vehicle tick `00686060` handles decay, shifting and body-speed termination;
  decay helper `00668fe0` saturates timer and clears strength. Engine tick
  `006af740` applies the peak-torque override, and automatic shift routine
  `006af490` rejects staging. Suspension traction uses the launch getter.
  Tire-assist routine `006a9b40` independently tests staging flag bit zero and
  multiplies all four tire traction boosts by `0.25`, without a drag-style gate.
  The preceding drag-style branch belongs to burnout processing only.
  RBVehicle begin-frame routine `006a7650`, reached through its constructor's
  vtable, confirms staged chassis restraint after ordinary integration while
  preserving vertical and roll/pitch motion. It does not modify engine or wheels.
  Chassis sleep routine `0069d120` immediately returns no-sleep while staging.
- Numeric execution: idle 1000, redline 8000 and peak RPM 5000 yield band
  `(5000, 1750)`. RPM 5000 and 6750 produce zero; 5001 produces
  `0.5002856850624084`, 5875 produces `0.75`, and 6749 produces
  `0.9997143149375916`. Peak RPM 7500 moves the lower edge to 5750.
  Second gear has no band. Timer 4.5 at delta 1/144 becomes
  `4.49305534362793`; exact expiry and overshoot clear both fields.
- Correction to the older audit: the launch band is not an input to torque
  converter or clutch ramps. Those correctly retain cached peak-torque RPM.
- Limits: isolated routines do not establish whole-race physics or producer
  timing. Ordinary race callers and live capture acceptance are recorded
  separately. Drag-specific shift, heat and staging behavior remain outside scope.
- Passive race evidence: two user-driven Golf GTI starts, maximum ordinary
  performance hardware without Junkman and neutral tuning, confirm every staged
  strength against displayed RPM. Maximum strength residuals are below `5e-8`.
  The starts produce strengths `0.8666727543` and `0.9873669147`; both terminate
  as physical speed crosses the inspected cutoff. The second countdown reaches
  displayed RPM `6999.5186` with redline `7000`. Driver pedals remain active;
  no staged handbrake is forced. This is not whole-route agreement.
- HUD consumer `0063a600` uses language label `0xb1e2e5c2`
  (`HUD_LAUNCH_PERFECT`) and generic-message script `0xa19bb14c`. The scene uses
  that installed localized label and existing script while retaining GO separately.
  Tachometer needle range-color feedback remains outside this correction.
