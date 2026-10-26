# Countdown drivetrain comparison

This supplements [player launch](player-launch.md) and the
[driving status](../player-driving-status.md). PC v1.3 remains the execution
reference. Xbox source can identify shared rules but does not establish PC
floating-point behavior. The game still loads the user's PC installation.

## Clutch play while engaging

In first or reverse gear, the negative net-torque clutch-play adjustment applies
in every clutch state. When road torque is less than engine torque and their sum
is negative, sample the clutch-play curve at `1000 * sum`, multiply its result by
the sum and the configured clutch slip, and add that to road torque. Do not gate
this operation on the clutch being fully engaged.

The later positive-throttle road-torque relaxation is a separate operation. It
requires an engaged clutch, first/reverse gear, positive throttle and opposing
road/engine torque. Splitting these gates preserves ordinary engaged behavior
while correcting the engaging case. No clutch constants or grip values change.

The remembered transmission-minus-engine RPM difference belongs only to the
engaging state. Engaged and disengaged branches clear it to zero; engaging
records the current difference after checking for a large sign reversal. A
difference remembered before the clutch opens must not halve the stiffness of
a later engagement. Neutral bypasses these gear-coupling branches.

## Evidence and acceptance

The selected PC binary's control flow independently confirms the distinct gates;
see [provenance](../provenance/player-countdown-drivetrain.md). The existing
original Golf captures begin in first gear with a disengaged clutch and include
clutch engagement during countdown. They also distinguish actual engine RPM from
displayed RPM, which may remain elevated after lifting the throttle.

This correction is not complete countdown acceptance. Independent per-call tire
replay cannot establish engine/clutch state sequencing. One-step engine replay
must preserve the previous engine and transmission speeds, clutch state/timer,
attributes, actual wheel feedback, controls and call order. Sequence replay must
initialize once and preserve subsequent engine state. Missing wheel feedback
must be reported as missing or diagnostic, never silently treated as exact.
