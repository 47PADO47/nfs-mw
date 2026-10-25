# Loaded-tire capture corrections

Sources: guarded passive original-PC tire-entry/leave observations and isolated
execution of the selected original executable. See the
[provenance record](../provenance/vehicle-tire-capture-corrections.md).

## Nonzero patch motion

For Reference tires, dynamic friction and the brake-lock test run when skid
speed is strictly above `1e-6` m/s and at least one patch-velocity component
is nonzero. Patch speed does not have a second `1e-6` threshold. A nearly
stationary body can still have a spinning wheel and substantial tire friction.
The exact zero-vector guard avoids dividing by zero.

The generic Regularized profile retains its existing patch-speed threshold.

A reduced original-code numeric vector isolates the rule: radius 1 m, wheel
speed 1 rad/s, load 1,024 N, previous load 1,024 N, previous traction 0.5,
forward patch speed `2^-22` m/s, zero lateral patch speed, body speed 1 m/s,
delta `1/64` s, static grip 0.5 and dynamic grip 1. All boosts and handling
scales are one; pedals, drive torque and old torque/acceleration/road speed are
zero. Maximum slip is one. The original returns exactly 512 N longitudinal
force and records exactly -256 N m averaged wheel torque. These outputs do not
depend on the small-angle approximation used elsewhere in the tire routine.

## Locked-wheel sign history

The Reference loaded-tire update refreshes remembered wheel direction after
its wheel-speed calculation, including when the brake locks the wheel. A wheel
locked to zero records zero direction, rather than retaining the preceding
positive or negative direction. Angular acceleration remains zero while locked.
The Regularized profile retains its existing locked-wheel behavior.

## Limits

These corrections do not change grip constants, tire profiles, scheduling,
steering history or AI control. The original angle helper has quantized and
near-zero behavior that differs from the current Rust `atan2`; that remains a
separate arithmetic comparison. Individual tire-call agreement is not complete
car, suspension, contact-order or trajectory parity.
