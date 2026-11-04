# Civilian traffic queues

Sources: read-only PC 1.3 executable analysis and the CC0-1.0
[reference reconstruction](https://github.com/dbalatoni13/nfsmw), already credited
in NOTICE. See [provenance](../provenance/traffic-queue-parity.md).

## Keeping a blocker eligible

**[PC static and isolated original execution]** A civilian traffic navigator
retains an obstacle ahead when the vehicles are closing, even when their
predicted closest approach is three seconds or more away. This exception applies
when the obstacle's inside/behind flag is clear. The three-second cutoff still
applies when that flag is set.

For a non-closing obstacle with that flag clear, traffic checks distance along
the road rather than full distance between body centres. Its maximum range is
the combined longitudinal extents plus half the car's current body speed and
twice its own extent. Side distance must be no greater than combined side extent
plus one metre. Eligibility alone does not guarantee a corridor cut: the body
must still intersect the recorded trail.

Keep the existing eligibility path for other navigator classes outside this
civilian correction. Cookie selection, cut geometry, braking distances and
driver update clocks remain separate rules.

## Stopping and restarting

**[Synthetic remake reproduction]** The previous filter dropped slowly closing
obstacles as their predicted encounter moved beyond three seconds. A flat-road
feedback test reproduced a traffic car stopping before a stationary player,
losing its blocked state, requesting the custom restart speed and driving into
the player's footprint. This is a remake failure, not a matched native trajectory.
The initial diagnostic used the generic Regularized tire policy. The queue
regression selects Reference integration, as installed-car readers do, while
retaining synthetic vehicle parameters. It does not establish stock-car stopping
distances or attribute every generic-fixture drift to live traffic physics.

The corrected filter must retain blockers through deceleration and waiting.
A regression uses actual vehicle integration and checks separation from the
stationary player. It also checks behavior after the player leaves, so holding
the car forever cannot satisfy the test.

**[PC static]** Native ordinary requests are capped at current body speed plus
twice the decision delta, without a one-metre-per-second floor. Normal simple
pedals hold the brake below 0.5 m/s. Native vehicle physics mode 2 has a separate
direct-motion path which bypasses that pedal threshold.

The conditions selecting mode 2 for nearby ordinary traffic remain unresolved.
The remake drives its traffic through full vehicle physics. Retain its restart
allowance while correcting blocker retention; removing that allowance alone
can strand a stopped car on a clear road. Full native restart parity, physics-mode
transitions and live matched queue comparisons remain open.
