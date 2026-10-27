# Engine scalar arithmetic order

Date: 2026-10-26. Sources: original PC 1.3 executable (proprietary,
behavior inspection only), passive player-engine boundary capture and the
existing remake implementation. No restricted code is ported.

A compact capture contains 1,074 complete engine entry/leave pairs, including
two countdowns and automatic shifts. An isolated original-instruction replay
reproduces every byte of the recorded engine object after all 1,074 callbacks.
Its adapters declare transmission settings, neutral tuning, exception-chain
backing and a gear-event allocation sink. Alternate recharge permission produces
the same outputs. This establishes engine-boundary reconstruction, not event
delivery, wheel leave state or a continuous vehicle trajectory.

The initial production comparison of 353 staged callbacks identified scalar
arithmetic differences, with independently matched entry state and attributes:

- The normalized idle/redline factor multiplies the signed gear ratio before
  the locked-wheel mean. Reversing these products can change transmission speed
  and the later clutch/torque response.
- Positive boost-gauge targets multiply the normalized boost by gauge scale,
  then by spool. Vacuum targets multiply normalized vacuum by gauge scale, then
  by the RPM fraction, and negate the completed product.
- Gauge slewing selects the target directly when reachable in this tick. Equal
  values retain the prior sign. Reconstructing it by subtracting and re-adding
  the prior value introduces rounding and signed-zero differences.

These contracts were specified before implementation. Constants, tuning,
traction and AI decisions are unchanged. Generic arithmetic regressions and
private recorded boundaries are separate evidence. Continuous race acceptance,
complete tire/world response and original render presentation remain open.

Corrected production method bodies, compiled in a private boundary harness,
match every modeled common output in all 1,074 independently seeded callbacks.
Before correction, 703 callbacks matched every common output; afterward all
1,074 do, without a numerical tolerance. Diagnostic seed/output methods do not
replace production calculations. The five absent fields are stored angular
acceleration, maximum horsepower, damage flags and original shift-status storage.
This does not establish continuous state flow between callbacks or render parity.

## Carried-vehicle follow-up

The retained Carrera engine boundaries expose another grouping difference when
gear efficiency is non-unit: the original multiplies returned efficiency by
drive torque before multiplying the signed ratio. The earlier Golf corpus has
unit efficiency and cannot distinguish those orders. Actual instruction tracing
and a separate captured-engine adapter identify this cause without changing the
floating-point precision model. The contract precedes the one-line correction.
This affects shared drivetrain arithmetic; it is not a car-specific force tune.
