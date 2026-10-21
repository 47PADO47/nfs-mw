# Particle emission control

[verified PC instructions] Matching PC 1.3 controller and spawn-count execution
with synthetic object/attribute state on 2026-10-20 establishes these rules.
Reference emitter source identifies the fields and states; machine instructions
are authoritative. This is isolated instruction execution, not live gameplay.

States are not-started, delaying, on-cycle, off-cycle, on and off. A phase exists
when its duration or variance is nonzero. Without a one-shot flag, an on-cycle
with no off-cycle is treated as continuously on. Cycle variance is added as a
draw between zero and the variance, not a symmetric plus/minus range.

The first update with a delay enters delaying with delay minus delta and emits
nothing. Later delay updates subtract delta; reaching zero enters the on-cycle
or continuous-on state. The emission delta is delta plus the signed remainder.
The initial on-cycle subtracts delta and clamps its remainder at zero, while
still emitting that update. Subsequent on-cycle updates subtract delta; reaching
zero emits on that final update, then enters off for one-shot or off-cycle for
looping emitters. The boundary emission delta is delta plus the phase's previous
remainder. Preserve this PC behavior, including its unusual positive rollover.
Off-cycle expiry enters on-cycle and emits using delta plus its signed remainder.

A forced or authored one-shot with no on-cycle enters continuous-on; it does
not terminate after one update. Its spawn count uses Life reduced by LifeVariance
instead of delta, on every emitting update. No invented event-duration or
single-burst cutoff is permitted. Hosts retain resource budgets and spatial
cleanup independently from emitter timing. Positive on-cycle one-shots stop
emitting at the off state, while existing particles drain.

StartDelayRandomVariance draws the start delay between zero and StartDelay.
Intensity must pass the emitter's authored min/max range before spawning. The
existing fractional spawn accumulator applies when a step owes under one particle.
Independent deterministic randomness need not reproduce the original global
random stream; a seeded remake run must remain repeatable.
