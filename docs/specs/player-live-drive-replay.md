# Player live-drive replay corrections

This supplements the drivetrain, suspension and rigid-body specifications.
See [provenance](../provenance/player-live-drive-replay.md).

## Engine scheduling and cached shift request

Automatic shifting consumes the shift potential cached by the previous engine
callback. It does not recompute that request using the new throttle. Reverse,
an existing positive shift timer, staging and a positive sport timer inhibit
automatic shifting. There is no additional current-clutch-state gate.

Automatic neutral exit assigns first gear directly, without a normal gear-change
delay. Actual gear changes clear the cached potential. After torque integration
and RPM smoothing, refresh the potential only with an engaged clutch outside
staging; otherwise clear it. The shift light reads the same cached request.

Run automatic shifting before decrementing the shift timer and before advancing
the clutch. Decrement a positive shift timer and clamp it to zero afterward.
Decrease the sport timer using raw gas only when a nonzero cached potential
remains, and clamp it to zero. Advance
the clutch timer next, then issue the gear's clutch command. A positive clutch
timer advances even outside the engaging state; retain its negative crossing
value and mark the clutch engaged when it expires. The remake's optional manual
clutch override cancels that timer when it explicitly holds the clutch open.

## Transmission-speed arithmetic

The transmission-side angular speed uses this single-precision operation order:
`(redline_omega - idle_omega) / redline_omega * locked_wheel_speed * ratio + idle_omega`.
Moving the division to the end can change a strict redline comparison and omit
the original torque cut and wheel-speed correction. Preserve the order without
adding a tolerance to the limiter.

## Rear-tire pilot factor

For Reference tires, evaluate the rear-wheel assist in metres per second:
subtract the converted 30-mph threshold, multiply by the reciprocal of the
converted 20-mph span, clamp to 0..1, then interpolate from 0.85 to 1. Use
`factor * (1 - 0.85) + 0.85`. Converting body speed with the separate rounded
metres-per-second-to-mph factor changes this curve. Front, brake-locked and
backward-spinning wheels retain a factor of one. Regularized tires keep their
existing policy. No tire grip attributes change.

## Body and wheel torque arithmetic

Transform integrated torque with the existing ordered vector helper: combine
Z and Y contributions, then add X. Do not replace it with a matrix multiplication
that sums in a different order.

Use the same ordered transform for wheel arms, rotated COG, steered forward
directions and contact patch positions. Point velocity depends on the transformed
arm. A small change can cross a quantized slip-angle boundary and change tire
force despite identical captured tire state.

For wheel-force torque arms, subtract the rotated COG offset from the world
contact position first, then subtract the body position: `(patch - rotated_cog) - position`.
This differs numerically from subtracting a previously added world COG point
at large world coordinates. Keep force application and lift timing unchanged.

## NOS speed conversion

The engine callback converts physical speed for NOS with the single-precision
factor `2.23699`. Keep this separate from the generic display conversion.
This speed controls the 10-mph activation and 5-mph sustain thresholds, and the
recharge interval. Using `2.2369` can delay activation and change tank increments.

## Orientation integration

For nonzero angular velocity, multiply it by the step delta before forming the
quaternion derivative. Evaluate each vector component's cross difference first,
then add the scalar-orientation contribution. Subtract the scalar derivative's
three products in X, Y, Z order. Add half the derivative to the old orientation.
Normalize using squared components in W, Z, Y, X order, left-associated, then
multiply all components by the reciprocal square root. Component-wise division
or a SIMD dot-product sum can produce different pose caches. Keep the existing
zero-angular-velocity branch and all force, inertia and damping policies.

## Comparison limits

Recorded state supports independent body, engine and tire boundaries. A wheel
force-assembly boundary can use recorded tire forces explicitly; it does not
also validate their producer. Cached collision primitives, every world callback
and audio warm state are not included. Complete closed-loop route parity remains
open. Unknown live FPU precision is not replaced with an invented tolerance.
