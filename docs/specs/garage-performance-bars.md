# Original garage performance bars

Sources: matched retail PC executable, SHA-256
`80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`;
installed `attributes.bin`; the `dbalatoni13/nfsmw` CC0 reference, read only.
See [provenance](../provenance/garage-performance-bars.md) and the earlier
[car-lot investigation](career-car-lot.md#open-questions).

This contract describes the frontend rating calculation. It does not measure
the live vehicle's lap time, tire grip or physical launch behavior.

## Three axes and population [static executable]

The frontend stores `[top speed, handling, acceleration]` fractions. Its raw
performance calculation returns time to 100 mph, top speed and a handling
rating. The initializer computes those raw values for eligible stock vehicles
and for their maximum ordinary performance packages. Eligibility requires a
positive, increasing `pvehicle.HandlingRating` pair and a usable powertrain.
The native initializer also excludes unresolved model records and temporary
attribute instances; ordinary installed collections are persistent records.

Use one population containing both stock and maximum-build raw scores when
finding each axis's minimum and maximum. Normalize top speed and handling
directly; acceleration is one minus normalized time to 100 mph. Clamp to
`[0,1]`. A population spread at or below `1e-6` normalizes to zero, so its
acceleration fraction is one. The maximum endpoint defaults to the stock
endpoint when a valid upgraded result is unavailable.

## Raw frontend performance calculation [static executable]

This is a coarse longitudinal estimate, separate from the game's driving
simulation. It has no tire traction, gear-change delay or gear-efficiency term.

Inputs are the resolved stock or upgraded engine, transmission, induction,
nitrous, chassis drag, vehicle mass and rear tire diameter. Torque remains in
foot-pounds until multiplied by 1.3558. Other conversion constants are
RPM-to-radians/sec `0.104719758`, radians/sec-to-RPM `9.54929638`, and
MPH-to-metres/sec `0.44703`.

1. Find the engine's highest naturally aspirated torque sample, choosing the
   first equal maximum. Its evenly spaced sample RPM is clamped to idle and
   red line. This supplies the first gear's minimum launch RPM.
2. Compute naturally aspirated upshift RPM for each forward gear except the last.
   Search from the midpoint of idle and red line upward in 50 RPM steps.
   Compare current torque with next-gear torque multiplied
   by the next/current gear ratio. The first strictly greater next-gear result
   is the shift point; otherwise use red line minus 100 RPM. Last gear uses
   red line. Ratios at or below magnitude `1e-6` cannot provide a downshift
   torque comparison. The native helper passes 1.0 as the induction RPM in
   these comparisons, so its boost term is zero for ordinary installed engines;
   naturally aspirated torque supplies these comparisons.
3. Start at speed zero, time zero, first gear and timestep 0.125 seconds.
   Limit the estimate to 120 seconds. Effective ratio is final drive times
   the current gear ratio. Radius is half the rear tire diameter.
4. Engine RPM uses the original launch-slip mapping:
   `((speed / radius) * ratio * (1 - idle/redline) + idle * RPM_TO_RAD) * RAD_TO_RPM`.
   Clamp between idle and red line, and in first gear raise it to at least the
   peak-torque RPM.
5. Wheel force is engine torque at that RPM times `1 + full induction boost`,
   times 1.3558 and the effective ratio, divided by radius. While elapsed time
   is below nitrous capacity and speed is above 5 m/s, multiply by
   `1 + nitrous torque boost`.
6. Update speed by `(wheel_force / mass - abs(drag * speed²) / mass) * timestep`.
   On the first speed at least 44.703 m/s, record the time at the beginning of
   that iteration and change the timestep to one second for subsequent speed
   estimation. The elapsed-time increment uses that newly selected timestep.
7. Record top speed at the positive engine speed limiter, or when last gear
   reaches red line or wheel acceleration falls below aerodynamic drag.
   Stop once positive time-to-100 and top-speed results are available. Advance
   one gear when RPM reaches its upshift threshold. On exit, last gear or an
   absent top-speed result uses the final simulated speed, as the native routine
   does. Reject results without positive finite time and top speed.

The handling raw score is not tire grip. Interpolate the vehicle's
`HandlingRating[0..1]` by the handling-weighted installed/max package fractions
from the table below. Stock uses zero upgrade fractions.

## Package blend and Junkman [static executable]

The category order matches the existing performance package indices.

| Category | Top speed weight | Handling weight | Acceleration weight |
|---|---:|---:|---:|
| Tires | .25 | 1.5 | .25 |
| Brakes | 0 | .5 | 0 |
| Suspension | .25 | 1 | .2 |
| Transmission | 1 | 0 | .75 |
| Engine | .5 | 0 | 1 |
| Induction | .25 | 0 | 1.25 |
| Nitrous | .25 | 0 | 1.5 |

For each axis independently, let `p_i = installed level / maximum level`,
using zero when its maximum is zero. Let `r` be the weighted average of those
fractions, and `m` the share of that axis's weights selected by the installed
Junkman category bitmask (`pvehicle.junkman_current`). Given normalized stock
and maximum endpoints `lo` and `hi`:

```
adjusted_hi = hi + (1 - hi) * m
adjusted_lo = lo + (adjusted_hi - lo) * .33 * m
position = min(1, (1 + .33 * m) * r)
rating = adjusted_lo + (adjusted_hi - adjusted_lo) * position
```

With no Junkman this is ordinary interpolation between the car's endpoints;
stock therefore retains its normalized base rating. A selected Junkman mask
can increase both the base and maximum rating. Cosmetic changes and tuning
sliders do not alter these frontend package fractions.

## Filled strips and comparison tick [reference and native calls]

The customization screen's current slider value is the preview build. Its
preview/handle value is the installed car before shopping. Both span zero
to one across 160 authored units. Browsing changes the filled strip; the
comparison tick stays at the installed rating. The original `TwoStageSlider`
also fills `PREVIEWBAR_<axis>` to that installed value when the object exists.
The tick is an absolute installed-rating position, not a signed difference.

## Validation boundary

Synthetic tests must check native estimate inputs, nonlinear Junkman blending,
combined population normalization and category independence. Installed tests
must enumerate eligible vehicles, produce finite endpoints and exercise every
available maximum package. Original executable behavior is static evidence
unless a separately recorded native execution comparison is supplied.
