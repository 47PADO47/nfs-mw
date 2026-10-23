# World physics timing

The world scene owns one fixed clock at 60 ticks per simulated second. The player,
traffic and trailers do not maintain independent clocks. The frame's input is
sampled once, with button presses held until a tick consumes them.

Each tick currently runs these operations:

1. Advance prop lifecycle timers by one fixed step.
2. Integrate the player and all AI cars/trailers.
3. Resolve car pairs, including the player and detached trailers.
4. Capture poses after contact separation; advance traffic, pursuit and spawn timers.
5. Feed the player's final state into effects, sound, telemetry and recovery checks.

Rendering interpolates the preceding and current poses using the shared clock's
fractional remainder. Parked cars and completed scripted captures draw their final
pose. A screenshot batch advances six shared ticks, including the traffic and prop
timers; it does not advance only the player.

The accumulator uses double precision and keeps every elapsed tick, including
after a slow frame. Catch-up work can delay the next rendered frame; dropping that
work would change collision outcomes and elapsed game time. Invalid/nonpositive
frame times add no time. Streaming pauses the entire simulation until the required
area is resident, so moving cars cannot hit traffic frozen by streaming.

## Parallel AI integration

From eight AI cars up, the 60 Hz step integrates the cars on several threads (`ai/parallel.rs`). Only the
integration runs there: each car's start (search, think with every random draw, driver, input, explosions) runs
first in car order, and its end (props, walls, damage, effects, sounds) runs afterwards in car order. No car reads
another car's new state within a step, and each thread casts rays through its own candidate window, so the
result is bit-identical to the serial order; `ai/pursuit/tests/parallel.rs` compares both every tick.

## Verification

The three-car scenario in `drive/sim/tests/frame_rate.rs` records every tick's
position, orientation, linear/angular velocity and the complete impact sequence.
The traces agree bit for bit at 5, 10, 30, 60, 120, 144 and 240 FPS, and with uneven
frames including 300 ms stalls. Clock tests cover fractional remainder, invalid
times and a five-second stall. These are deterministic simulation tests, not a
claim of bitwise agreement with the original game or of completed visual QA.

## Remaining order differences

Car integration still includes each car's world contacts before pair contacts.
The original's general body pipeline resolves pairs before world contacts and
articulations. Full ordering parity is tracked in [world interactions](world-interactions.md),
together with primitive sweeps, moving props and released truck cargo.
