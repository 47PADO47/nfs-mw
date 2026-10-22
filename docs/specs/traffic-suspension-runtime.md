# Traffic and trailer suspension runtime

Sources, numeric comparisons and limitations are in the
[provenance note](../provenance/traffic-suspension-runtime.md).

## Behavior selection

Installed ordinary traffic and tractors select `SuspensionTraffic`, trailers
select `SuspensionTrailer`, and police select `SuspensionSimple`. Sharing the
player's racing tire solver or applying the police chassis coefficients to all
three does not preserve these authored choices. Keep the player and police
paths separate from this traffic/trailer correction.

## Traffic

Use a fixed 45-degree steering range, identical left/right steering angles, and
the ordinary common centre-of-gravity calculation with no police ride offset.
The installed traffic torque split is the rear share; divide each axle's share
equally between its supported wheels. Do not apply the racing differential,
burnout, tire braking balance or oversteer assists to this distribution.

Each loaded tire receives direct brake input and rear wheels receive handbrake.
Apply brake torque opposite wheel rotation. The linear lateral force is
`-2 * lateral_speed * load * grip_scale`, divided by the combined wheel-slip and
lateral speed when that speed exceeds one metre per second. At low forward
speed, multiply it by `min(abs(lateral_speed), 1)`. Ordinary braking does not
select the racing tire's wheel-lock test. Set wheel rotation to the rolling
speed scaled by `1 - handbrake`.

Above one metre per second forward speed in either direction, subtract half
the lateral force times the sine of slip angle from longitudinal force. With
handbrake above 0.5 and combined slip above one metre per second, longitudinal
force instead opposes forward velocity with the same normalized linear grip.
Keep the common per-wheel lateral velocity cap. Halve the vertical component
of each contact force's lever arm before computing its torque. Apply simple
quadratic drag at the centre of gravity, without a throttle-dependent multiplier.

## Trailer

Use the original seven-sample cornering curve over zero to one third of a turn:
`0, 0.4, 0.8, 1.2, 1.6, 1.75, 1.65`. Multiply it by load and installed grip
scale, opposing lateral motion. Rear cornering force scales by 1.5 normally and
0.75 when handbrake exceeds 0.5. Apply the installed regular and handbrake
torques opposite forward velocity. If combined force exceeds static grip,
scale both components to dynamic grip. Retain the same low-speed lateral ramp
and steering-drag term as traffic.

Regular brakes preserve rolling wheel rotation. Any positive handbrake stops
the wheels and reports slip opposite forward motion. Unlike traffic, all four
trailer tires receive the handbrake. A free tire with either brake input stops
spinning; otherwise it coasts without the racing wheel's torque integration.

While hitched, trailer centre of gravity has zero longitudinal offset and
vertical offset `roll_center - (body_half_height + maximum_ride_height)`, in
metres. When detached it becomes the body origin and both brakes clear.
This is independent of the tandem axle's location; the common car weight-bias
calculation must not move the trailer's centre of gravity over the rear wheels.

## Limits and checks

The tire angle query uses a 16-bit turn, first zeroing input components no larger
than one millionth. In the first octant, quantize the smaller/larger ratio to
16 fractional bits, interpolate integer angles between 257 nodes at ratios
`i / 256`, and use quadrant symmetry. Each node is independently generated as
the nearest integer to `atan(i / 256) * 65536 / (2*pi)`; interpolation truncates
the fractional integer contribution. This reproduces the original table without
shipping its bytes. All 257 generated nodes match the analyzed original table.

The rewrite retains its existing engine, suspension probing and fixed-step
scheduling. Exact traffic powertrain,
complete suspension/contact ordering and live full-route parity remain open.
Numeric isolated tire routines are an independent check of the tire response,
not proof of whole-vehicle trajectory parity. Automated installed scenarios
must record controls, lateral velocity, wheel locks and hitch angle during
straight braking, downhill braking, turns and contacts, and preserve fixed-tick
results across presentation rates.
