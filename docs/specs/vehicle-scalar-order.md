# Vehicle force scalar arithmetic

Companion to [suspension and tires](vehicle-suspension-tires.md).
These contracts describe single-precision operation order. Algebraically
equivalent expressions can produce different forces and later body states.

## Speed scales

Normalize speed with the existing clamped ramp. Multiply the normalized value
by the last table index, then split the resulting position into an index and
fraction `t`. Interpolate adjacent values `a` and `b` as
`(1 - t) * a + t * b`, rounding each operation to single precision.
Apply the grip or traction gain after interpolation. Preserve endpoint clamping,
reverse traction and the separate slip-limit branches. This contract applies
to these speed scales; it does not redefine every table in the vehicle model.

## Spring and damper

Calculate compression speed as `(new - old) * (1 / dt)`, with the reciprocal
rounded before multiplication. Calculate progressive spring force as
`((1 + new * progression) * stiffness) * new`. Keep the existing digression,
blow-off and anti-roll operations after these calculations.

## Tire force

Reference wheel load projects the flattened wheel-up vector onto the terrain
normal by adding the Z and Y products before X. Apply the existing scale,
offset and minimum load factor afterward, rounding each operation to single
precision. The Regularized policy keeps its existing projection order.

For a reference tire gripping without a locked brake, compute the angular
acceleration feedback as `((angular_acc * radius - forward_acc) / radius) *
wheel_inertia`, then add it to the stored torque-derived force. Round each
operation to single precision. The stabilized policy keeps its existing order.

When reference tire slip exceeds its current limit, divide the stored traction
by the absolute slip, then multiply by that limit. Keep the ellipse reduction
and the existing branch condition. The stabilized policy retains its order.

The lateral-force family lookup also uses weighted endpoint interpolation.
Its load coordinate remains `(max(load, 0) * 0.001) * 0.8`. Below twelve
degrees, interpolate adjacent angle families as `a + (b - a) * t`, then
multiply by tire grip, per-step boost and 2500 in that order. At the plateau,
multiply the final family by boost, tire grip and 2500 in that order. This
describes the existing single-descriptor model; runtime descriptor-count
selection is outside this contract.

For reference tire sliding friction, divide dynamic grip by skid speed before
multiplying by load. In the existing low-speed friction branch, calculate the
single-precision reciprocal of dynamic grip once, then multiply each stored
slide component by it. The stabilized tire policy retains its existing arithmetic.

For the reference friction ellipse, calculate the maximum force as
`((((drift * traction_boost) * static_grip) * load) * pilot)`, rounding each
product before the next. Divide the stored maximum by force magnitude to obtain
the limiting ratio. Keep the existing limit test and force-component updates.

Reference skid speed, contact-patch speed and ellipse force magnitude use two
single-precision squares, their single-precision sum and its square root.
Do not substitute a higher-precision hypotenuse calculation. The stabilized
policy retains its hypotenuse calculation.

Reference wheel forward velocity projects point velocity onto the wheel forward
direction by adding the X and Z products first, then the Y product. This is a
local forward-projection contract; other vector operations remain separate.

For a reference wheel's terrain query, calculate horizontal speed from the
rounded X and Z squares and their sum. Multiply by the step, add the downward
travel, then add the wheel radius. Apply the existing height cap afterward.
Keep the stabilized policy's previous grouping.

For reference downforce, multiply speed by the aerodynamic coefficient and
2000, then multiply by forwardness and upness, in that order. Apply the
existing airborne and tuning factors afterward. Round each operation to single
precision. The stabilized policy retains its existing grouping.
For reference forwardness above the existing speed threshold, multiply velocity
by a rounded reciprocal of speed. Add the Z and Y direction products before the
X product, then apply the existing positive clamp, square root and floor.

## Evidence and limits

Original PC instruction-block replay distinguishes these expressions from the
previous implementation under a declared single-precision arithmetic contract.
Synthetic regressions exercise the differences independently of recorded car
values. The live floating-point control word was not recorded. Agreement under
this contract does not establish full-world or continuous-route parity.
See [provenance](../provenance/vehicle-scalar-order.md).
