# Explicit player-car timestep

Sources: the existing engine's `CarSim` adapter and the passive original-PC
measurements documented in [player driving parity](player-driving-parity.md).
This is timestep infrastructure, not a new original-game algorithm.

## Contract

A car step accepts one positive, finite simulation delta, in seconds. The same
delta reaches vehicle integration, reset-collision grace, explosion exposure,
wheel spin, brake-light age and the acceleration derived from that step's
velocity change. Terrain cache preparation and collision ordering are unchanged.
An invalid delta performs no step and leaves the previous state intact.

The vehicle's body-ground contact probe must also receive that same delta.
Its lift and search envelope depend on the duration and the body's motion.
Passing fixed `1/60` there while integrating another duration searches over a
different interval from the motion. At `1/60` the probe stays unchanged. This
does not add timestep scaling to the contact impulse response.

Production uses the explicit `step_with_delta` entry point, which also permits
component capture replay at deltas such as
`1/144` and full-Speedbreaker `1/576` without silently using `1/60` in the adapter.
Legacy test convenience entry points retain exactly `1/60` second.
The split integration carries its delta in its step context, so parallel work
does not read a global or another car's timestep.

## Boundaries

This adapter is the prerequisite for the
[shared real-time clock](shared-physics-clock.md). That scheduler advances
players, traffic and props together at 60 Hz by default or the selected offline
144-Hz comparison cadence. It avoids independent player ticks against stale
AI/prop poses. The adapter alone does not establish original-PC parity.

The measured rate belongs to the selected modded installation. It is not a
universal 144-Hz rule or a reason to replace multiplayer's deterministic 60-Hz
contract. The shared-clock specification owns frame sampling, player effects
and race-timer propagation.
