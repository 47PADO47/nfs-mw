# Shared real-time physics cadence

Sources: existing shared-world order in [physics timing](../physics-timing.md),
the original-PC measurements in [player driving parity](player-driving-parity.md),
and the user's explicit choice of a shared 60-Hz real-time default with an
optional 144-Hz comparison mode. This is a chosen engine policy, not a claim
that every original installation runs at one fixed rate.

## Scheduling and simulation delta

The local world schedules shared physics ticks at 60 real-time Hz by default.
An explicit comparison setting can select 144 real-time Hz. Rendering remains
independent of this cadence. Players, AI, props, pair contacts, police and race
timers use the same ordered tick; no actor runs against a separate stale clock.

Speedbreaker scales each tick's simulation delta, not the real-time tick count.
At full quarter-speed dilation a 60-Hz tick integrates `1/240` simulation second;
a 144-Hz tick integrates `1/576`. The original entry and exit ramp constants
remain unchanged. Other game-speed and moment scales multiply simulation time.

Each completed physics tick updates a separate single-precision multiplier
toward its target using that tick's full unscaled period. Physics delta is the
period multiplied by the final multiplier, after the update. This follows the
original PC time manager's ordering and stored-float rounding. Do not use the
area under the ramp as a physics delta, or average an entire render frame and
apply that average to every catch-up tick.

For example, the first 60-Hz entry tick uses multiplier `0.9666666388511658`
and delta `0.016111111268401146`; the first 144-Hz entry tick uses multiplier
`0.9861111044883728` and delta `0.006847993936389685`. On exit from quarter
speed, the first 144-Hz tick uses multiplier `0.2534722089767456` and delta
`0.0017602236475795507`. These are independently executed original-PC outputs,
with the 144-Hz entry sequence also observed in the selected live installation.

Frame grouping does not discard elapsed time. A partial tick retains its
real-time phase. Display interpolation uses that phase. A change of cadence
takes effect after a pending tick finishes at its old period. Invalid/nonpositive frame time adds no
time and does not advance the ramp. Streaming pauses both clocks and preserves
the previous interpolation phase. Screenshot batches retain a bounded shared
simulation advance rather than stepping only the player.

## Network sessions

When session rules disallow time scaling, physics remains at the existing
deterministic 60-Hz contract regardless of the local comparison setting.
The server's one-tick-per-update mode remains exactly one `1/60` simulation
step for a valid positive update. It never applies Speedbreaker or a local
144-Hz setting. This change adds no network protocol field.

## Per-tick consumers

Every time-dependent operation in the ordered world step receives that tick's
simulation delta: vehicle/suspension integration, explosion exposure, prop
motion and timers, AI control and motion, pursuit/roadblock timers, race flow,
effects, camera following and object lifetimes. Their legacy convenience APIs
retain a `1/60` default for existing callers and tests.

AI keeps its existing eight-/ten-tick decision cadence. Time-based governors,
repath, stuck and sight work consume accumulated actual simulation deltas since
the preceding thought. The first thought uses the role's nominal thought count
times its first tick delta, retaining the existing initial phase. Helicopter and
roadblock simulation-second accumulators retain their existing scheduling.
The prop ram calculation's fixed reference probe is an impact estimate, not a
runtime integration step, and remains separate from the shared clock.

The frame's analytically integrated continuous ramp still advances frame-level
camera and visual producers and scales accumulated look input. It is separate
from the single-precision physics multiplier and its endpoint deltas. Meter
recharge uses the sum of completed physics deltas; depletion uses real frame
time, including frames without a physics tick. Physical history timestamps use
the actual endpoint deltas. The camera's own real-time sampling is unchanged.
This does not establish complete original driving, collision-order or race parity.

The original camera's body-history timestamps advance by each completed tick's
actual simulation delta, including comparison-rate and Speedbreaker ticks.
Missing body snapshots still advance that physical-history clock. Nonpositive
or nonfinite deltas neither advance it nor append a snapshot. The camera's own
sampling, noise, damping and presentation remain on their existing independent
60-Hz real-time timeline. Changing physics cadence does not change that camera rate.

Player script edge actions are sampled against the previous script time rather
than a fixed `1/60`-second pulse width. A smaller tick must not request the same
gear change, reset or exhaust pop repeatedly. Road-check and fall-recovery
intervals remain simulation-time durations (`0.5` and `0.75` seconds), not
frequency-dependent tick counts. Audio history records each completed tick's
simulation delta; the host mixer still advances on its independent real-time
sample clock.
