# Pursuit reinforcement placement and road handoff

Sources: private runtime observations from an owned PC installation, and the
read-only CC0 [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw)
reconstruction, `AICopManager::SpawnPursuitIVehicle`, `AIVehicle` road reset and
`WRoadNav::Reverse`. No source expression is copied.

## Contract

**[decomp]** An ordinary reinforcement selects a road cursor using the target's
seek-ahead walk, or the forward-disc fallback. The walk reverses that cursor after
its placement checks. The fallback keeps the direction selected from target
travel. Successful placement hands the selected cursor to the vehicle, including
segment, lane, progress and direction. It does not locate the road again
from a point and heading. That second search can choose a different segment,
change an explicitly reversed one-way cursor, or discard a valid connector.

**[verified, bounded modded PC observation]** Forty-eight ordinary reinforcement
activations began with zero body velocity at entry and return of the spawn-state
reset. The body did not move across that reset. The first sampled pose followed
within 50 ms. This establishes stationary initialization in that capture, rather
than a general acceleration or pursuit-speed rule. Ordinary automatic chase
reinforcements therefore start with zero body speed. Explicit requests and
support placements retain their own supplied speed contracts.

**[decomp]** A failed ordinary placement does not add a unit, increment deployment
statistics or arm the successful-spawn timer. Existing heat rows, budgets and
support gates determine when another attempt is eligible. Measured interspawn
intervals and heading frequencies are observations, not tuning constants.

**[decomp, native PC checked]** The request remembers its missing adjusted
contingent count before adding the selected car. A successful join uses the
early-row delay before three total joins, except in cooldown. Later joins use the
normal row delay. While the remembered deficit exceeds two, alternate later
joins shorten that delay to 0.2 seconds. Failed placement does not advance this
alternation. The toggle is retained while the deficit is small. The manager's
existing fixed-step cadence still decides when the next eligible attempt runs.

**[ours]** The remake retains its scene ground-placement validation and shared
model construction. Accepted road cursors are handed through that adapter.
No special-case road labels or captured coordinates are used.

The original cursor-copy operation keeps the destination vehicle navigator's
filters. The remake currently constructs its cop navigator from the selected
cursor; the richer filter and lane-offset adapter remains a separate parity
boundary.

## Validation boundary

Synthetic tests exercise the actual empty-world spawn path. They cover zero
automatic entry speed and an explicitly reversed one-way cursor surviving body
placement. Installed manager-to-driving coverage must then check the first AI
targets, body motion and retirement. A correct initial cursor alone does not
establish pursuit-driving parity.

The capture also distinguishes roadblock, heavy-support and helicopter
activations from ordinary reinforcements. Aggregate heading and turn statistics
must not be substituted for the ordinary reinforcement contract.
