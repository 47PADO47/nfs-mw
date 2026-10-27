# Reference chassis creep response

Sources: original PC 1.3 chassis damping instructions (proprietary), isolated
instruction execution and retained player boundary recordings;
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) `Chassis.cpp` and
`RigidBody.cpp` (CC0-1.0 decompiled references, behavioral corroboration only).
No source expression or private recording is included.

This extends the low-speed rules in
[chassis assists](vehicle-steering-assists-aero.md) §7.4. It applies to the
Reference tire policy. Regularized retains its existing stabilization choices.

The chassis captures speed and local linear/angular velocity before wheel
forces. Landing refreshes both saved local/angular velocity and body angular
velocity. Jump and creep consume that refreshed angular snapshot. Wall steering
alone changes the body while leaving the saved angular snapshot unchanged.

Staging skips creep. Otherwise require angular speed below 0.25 rad/s and no
positive gas input. Two branches follow, in order:

1. Below 0.5 m/s, with all wheels grounded, positive combined brake input,
   zero gas and no collision, damp current body linear/angular velocities and
   pending force/torque together. Reset wheel angular velocities to zero.
   Below 0.000001 m/s retain zero. Otherwise first compute the damping amount
   as `1 - speed`, then retention as `1 - amount`, with single-precision
   storage between these operations. Retention can differ from direct `speed`.
2. Below 1 m/s, retain the saved local velocity except multiply its lateral
   component by speed. Retain saved local angular velocity except multiply yaw
   by speed. Rotate both to world space and write the body velocities.

In the second branch, read the **current** accumulated force and torque and
convert them to body space. Construct additive response vectors: replace force
lateral and torque yaw with their negatives multiplied by `1 - speed`; leave
their other components unchanged. Rotate these vectors to world space and add
them to the existing accumulators. Consequently the resulting lateral force and
yaw torque retain approximately `speed`, while the other local components
double. This is an observed original behavior, not a tuning adjustment.
Preserve the subtract/add operations rather than replacing them with direct
component scaling; cancellation and transform rounding affect the result.
Reference vector rotations sum the Z and Y products first, then add X, with
single-precision products and sums. This applies to the local/world force
transforms and saved velocity transformations.

The full-stop branch acts on the current body vectors. The lateral branch
restores saved velocity vectors. Pending force and torque remain additive;
the next body integration consumes the result. Calls which resolve forces can
wake a sleeping body. This helper contract does not close the separate
vehicle-level sleeping-response gate or every external wake producer.

## Validation boundary

Synthetic instruction replay covers staging, full stop, lateral response,
collision fallback, distinct saved/current state, and the epsilon threshold.
Seven active retained boundaries corroborate the original helper outputs.
The recording includes six full-stop calls and one lateral call; it supplies
little coverage of low-speed lateral response across general orientations.
The recorder reduced frame rate, so these comparisons establish helper
boundaries rather than normal-cadence driving or complete trajectory parity.
