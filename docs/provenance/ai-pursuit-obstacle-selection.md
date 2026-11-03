# Pursuit obstacle selection and roadblock road walks

Date: 2026-10-25.

Specification: [obstacle selection](../specs/ai-pursuit-obstacle-selection.md).

- Supported original PC program identity was confirmed before inspecting its
  roadblock builder and road navigator's avoidable fetch routine. The builder
  confirms zero heading weight, cop path type, a zero advance direction hint,
  and decision-segment rejection at the destination. The avoidable fetch reads
  the vehicle's neighbour list and stops at the supplied capacity.
- The CC0 [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) reconstruction
  provides corresponding `AICopManager`, `WRoadNav` and `AIVehicle` context.
  Its vehicle neighbourhood sweep supplies the radius/speed rule. Nearest-body
  retention is explicitly our adapter, not a claim about original list ordering.
- Original ground support selection skips heavy options when a roadblock is
  attached. The inspected PC selection routine also writes a 15-second roadblock
  timer whenever a heavy option is selected, including ram support. The remake
  previously forwarded that reset only for heavy roadblocks. Do not increase
  support chances to conceal an unresolved lifecycle,
  asset-loading or placement failure.

Implementation follows the behavioral specification. Native code, executable
addresses, installed assets and private captures stay outside public changes.

## Avoidable sets, alongside skip and stuck recovery (2026-11-02)

Spec sections "Avoidable sets for pursuit driving" and "Stuck recovery". Read in the PC binary (behaviour only): the
avoidable fetch skips the pursuit target and same-pursuit slotted cops only for a cop that holds a formation slot; the
close-pursuit separation iterates the unfiltered neighbour list; the hole punch skips a vehicle whose front is behind the
cop while it is beside it. The stuck recovery's centre-lane reset and end-of-reverse lane snap are from the CC0
reconstruction's `GetUnstuck` action. Rewrite: the step marks slotted cops (`TrafficWorld::formation_slotted`), the cop
think builds both sets (`cop.rs` `avoid_sets`), `blackbox_roads::trail::punch` has the alongside skip with our side
extent, and `cop/recovery.rs` resets the cursor. Headless checks: `cop::avoid_sets_tests`,
`trail::punch::tests::a_car_overtaking_from_behind_alongside_cuts_nothing`. The installed catch-up and road-course cases
were run before and after; their results are recorded with the change, not here.

## Path refresh, neighbourhood and corridor cuts (2026-11-03)

Spec sections "Avoidable neighbourhood", "Corridor cuts: who is ignored" and "Path refresh", and the trail spec's hole
punching (§5). Read in the supported PC binary after confirming its identity (behaviour only): the pursuit road
action submits a search whenever its navigator has none pending; the path finder's task runs every frame and services
its queue with a 1 ms budget; submitting cancels the same navigator's queued search. The avoidable manager's task
(rate 0.25) enters each AI vehicle with a square of half-size max(radius, 2 x speed), the radius defaulting to 20 m,
and pairs the squares that overlap on both axes. The hole punch is called on the sweep's cookies, with the car's
lateral offset and lateral velocity in its current section; its side extent, signed approach time, timing rules,
closeness ramp, signed widening, one-off side choice against the predicted offset and its tail measure were read
from its disassembly. The race-mode margin factor and a trailer special case were seen and not implemented.
Rewrite: `cop.rs` searches on every road think, `trail/neighbours.rs` overlaps both sweeps (other bodies use the
default radius: the remake does not carry their radii), `trail/punch.rs` and `trail/occlusion.rs` follow §5. Headless
checks: `trail::punch::tests` (pass side, traffic timing, following car) and `trail::neighbours::tests`. Installed
driving results are recorded with the change.
