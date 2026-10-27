# Wheel feedback and force boundaries

This supplements drivetrain section 6.4 and suspension section 2.
See [provenance](../provenance/player-force-boundaries.md).

## Wheel feedback

`free` driven-wheel speed (used for the sign of braking torque): mean wheel speed of each driven axle,
taking the axle with larger magnitude when both drive. `locked` speed instead selects the largest
non-negative driven-axle mean in forward gears, or the smallest non-positive mean in reverse. Opposite-sign
axles must not hide the axle turning in the selected direction. This feeds `omega_trans`.

Write-back (used for lock-up and the limiter) first computes `wanted - free` from the unmodified wheel
feedback. Below **40 mph**, or when both wheels of a driven axle are off the ground, average that pair.
Then add the saved difference to every driven wheel. Do not subtract the direction-clamped `locked`
speed. See [PC feedback provenance](../provenance/player-force-boundaries.md).

Per-wheel contact and loaded-wheel count are distinct. Wheel feedback/limits and
the automatic upshift contact test use positive stored compression. The engine's
road-coupling fraction uses the suspension's cached loaded-wheel count, which
also requires sufficient surface upness. A compressed wheel on a steep surface
can therefore satisfy the first test without increasing that count.

## Automatic upshift contacts

Automatic upshifts require every wheel to have positive compression and every
signed wheel slip to be strictly below `4`. Negative slip is not converted to
its magnitude. Keep the compressed-wheel mask separate from the loaded count.

## Damper threshold

Zero is a zero threshold, not a switch that disables blow-off: any positive
damper force then vents. The PC instruction boundary confirms the comparison
and the damper/sway/spring accumulation order; see
[force boundary provenance](../provenance/player-force-boundaries.md).

## Bottom-out recovery

Collect lift from every available contact, including contacts below the loaded
upness threshold. Recovery runs after force assembly regardless of loaded-wheel
count. It translates in world Y and does not create spring compression or tire
load. Stored contact-patch positions receive the same translation as the body.
