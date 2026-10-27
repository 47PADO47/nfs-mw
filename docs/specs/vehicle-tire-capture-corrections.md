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

## Reference slip angle

The Reference loaded tire uses a quantized signed angle in turns instead of
`atan2`. Set each velocity component to zero when its absolute value is at most
`1e-6` m/s. Use absolute forward velocity. If both filtered components are zero,
the angle is zero. Otherwise divide the smaller magnitude by the larger with
single-precision rounding, multiply by 65,536, and truncate to an integer.

Use its upper eight bits to select adjacent octant nodes and lower eight bits
for integer interpolation. Node `i` is the mathematical value
`round(atan(i / 256) * 65536 / (2*pi))`, for `i = 0..256`; generate these values,
without embedding an extracted game table. Integer interpolation truncates the
weighted node difference. Reflect around quarter-turn for larger lateral than
forward magnitude, apply the original lateral sign, then divide by 65,536.

This single-precision ratio matches all 38,764 previously captured tire angles.
Four boundary cases differ by one angle unit under double-precision division.
Isolated original helper execution at single precision independently confirms
the arithmetic. The runtime FPU control word itself was not captured.
The existing rolling response retains its double-precision ratio and the generic
Regularized loaded tire retains its continuous `atan2` response.

## Limits

These corrections do not change grip constants, tire profiles, scheduling,
steering history or AI control. Angle agreement does not establish exact agreement
of every remaining floating-point operation. Individual tire-call agreement is not complete
car, suspension, contact-order or trajectory parity.
