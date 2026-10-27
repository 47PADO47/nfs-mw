# World physics timing

The world scene owns one shared clock at 60 ticks per real-time second by default.
An explicit offline comparison setting selects 144 Hz independently of rendering.
Speedbreaker reduces each tick's simulation delta instead of reducing tick cadence.
The player,
traffic and trailers do not maintain independent clocks. The frame's input is
sampled once, with button presses held until a tick consumes them.

Each tick currently runs these operations:

1. Advance prop lifecycle timers by the shared tick's simulation delta.
2. Integrate the player and all AI cars/trailers.
3. Resolve ordered prop and car pairs, including the player and detached trailers.
4. Finish every body's world/ground contacts and drag.
5. Resolve trailer articulations, then run engine and suspension response. Capture poses and publish effects,
   sound, telemetry and gameplay timers.

Rendering interpolates the preceding and current poses using the shared clock's
fractional remainder. Parked cars and completed scripted captures draw their final
pose. A screenshot batch advances a bounded 0.1-second real-time slice, including
the traffic and prop timers; it does not advance only the player.

The accumulator uses double precision and keeps every elapsed tick, including
after a slow frame. Catch-up work can delay the next rendered frame; dropping that
work would change collision outcomes and elapsed game time. Invalid/nonpositive
frame times add no time. Streaming pauses the entire simulation until the required
area is resident, so moving cars cannot hit traffic frozen by streaming.

## Parallel AI integration

The shared step integrates the AI cars on up to four threads of Bevy's compute task pool (`ai/parallel.rs`, from eight
cars up); without that pool (a unit test that never built the app) it stays on the simulation thread. The first version
started its threads anew every tick, which made the mean tick 10 % slower at 16 traffic cars and the p95 tick ten times
slower at 59 (2026-10-24, 16 players, dev profile; `world_step_costs` with `NFSMW_BENCH_CARS`); the pool's threads stay
alive, so a share only has to be picked up. Only the
integration runs there: each car's start (search, think with every random draw, driver, input, explosions) runs
first in car order. Shared pairs and body world contacts finish serially before
parallel engine/suspension responses; publication follows in car order. No response job reads
another car's new state, and each thread casts rays through its own candidate window, so the
result is bit-identical to the serial order; `ai/pursuit/tests/parallel.rs` compares both every tick.

## Verification

The three-car scenario in `drive/sim/tests/frame_rate.rs` records every tick's
position, orientation, linear/angular velocity and the complete impact sequence.
The traces agree bit for bit at 5, 10, 30, 60, 120, 144 and 240 FPS, and with uneven
frames including 300 ms stalls. Clock tests cover fractional remainder, invalid
times and a five-second stall. These are deterministic simulation tests, not a
claim of bitwise agreement with the original game or of completed visual QA.
The shared real-time clock, per-tick dilation integration and forced network
60-Hz contract are specified in [shared physics clock](specs/shared-physics-clock.md).

## Remaining order differences

Car pairs now precede body world contacts and engine/suspension response.
Prop pairs follow all body begins, and articulations follow body finish before
driving response. Unified mixed-body pair traversal, trailer-release callback
priority and every wake path remain unverified. See
[the phase contract](specs/shared-vehicle-phases.md) and
[world interactions](world-interactions.md) for remaining primitive sweeps,
moving props and released truck cargo differences.
