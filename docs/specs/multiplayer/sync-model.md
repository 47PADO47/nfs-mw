# Multiplayer sync model: prediction, interpolation, collisions, clocks, determinism

Status: **design only**. Nothing is built. Written 2026-10-16. This file holds the algorithms that
[ADR 0006](../../decisions/0006-multiplayer-netcode.md) decides on. The ADR gives the reasons and the budgets; this file
gives the steps, the state kept, the tuning values and the failure behaviour. The wire formats are in
[protocol](protocol.md). Where a number is a tuning value it is marked **(tuning)** and is to be set by the test
plan, not argued here. Security checks on the same data are in `security-gameplay.md` in this folder.

## 1. The three timelines

At any moment a client lives on three clocks, all counted in server ticks (1 tick = 1/60 s):

| Timeline | Symbol | What lives on it |
|---|---|---|
| Server time | `S` | The truth. The server is at tick `S` now |
| Predicted time | `C = S_est + lead` | The owner's car. It runs ahead of the server by the lead, so that each input arrives just before the server needs it |
| Display time of others | `R = S_est - delay` | Remote cars, traffic and police, drawn from snapshots, behind the server by the interpolation delay |

`S_est` is the client's estimate of the server tick from clock sync (section 5). `lead` is about `RTT/2` plus an input
buffer of 1 tick (LAN) or 2 ticks (online). `delay` is two snapshot intervals (6 ticks at 20 Hz = 100 ms; 4 ticks at 30
Hz = 66 ms) plus an adaptive allowance.

The owner therefore sees **itself in the future** and **everybody else in the past**. The gap between them is
`lead + delay`: about 105 ms on LAN and 250 ms or more on a poor link. Section 4 explains how contacts are kept
believable across that gap.

## 2. Prediction and reconciliation of the owner's car

### 2.1 State kept on the client

A ring of 128 entries (about 2 s), indexed by tick `t`:

| Field | Meaning |
|---|---|
| `input[t]` | The `InputState` applied at tick `t` (the quantised form that was sent, so that the client predicts with exactly what the server will receive) |
| `state[t]` | The reduced vehicle state after stepping tick `t`: position, orientation, linear and angular velocity, engine rpm, gear, nitrous level, boost, plus a checksum of the rest |
| `sent[t]` | Whether the input for `t` has been sent, and the last packet it was sent in |

Inputs are **quantised before use**: the client rounds throttle to 7 bits and steer to 9 bits and then steps with the
rounded values, so that the server steps the identical numbers.

### 2.2 Every client tick

1. Read the frame's input (the action layer holds presses until a tick consumes them, as single player does).
2. Quantise it and store `input[C]`.
3. Step the owner's `Vehicle` once with `FIXED_STEP`, the quantised input, and the ground (the world collision the
   client already has for rendering). Contacts with static world and static props are included. Contacts with remote
   cars, traffic and police are **not** (section 4).
4. Store `state[C]`.
5. Queue the input for sending. The input frame carries the last 8 ticks not yet acknowledged, so a lost packet costs
   nothing if the next one arrives.

### 2.3 Every snapshot received

The snapshot carries, in its own-car block, the authoritative reduced state at the last input tick the server applied
(`ack_input_tick = A`) and the server's input-timing report (section 5.3).

1. Drop `input[t]` and `state[t]` for `t < A` (they are confirmed).
2. Compare the snapshot state with `state[A]`. Error terms: position distance, orientation angle, velocity difference.
3. If every term is under its threshold (position 2 cm, orientation 0.2 degrees, velocity 0.05 m/s **(tuning)**), return.
   The prediction was right and nothing is touched.
4. Otherwise **rewind**: set the car to the authoritative state at tick `A` (the reduced fields from the snapshot, the
   rest from `state[A]`).
5. **Replay**: for `t = A+1 ..= C`, step with `input[t]`. Rewrite `state[t]`.
6. Compute the correction vector: position of the replayed car at `C` minus the position before the rewind, and the
   orientation difference. If it is over 2 m, or the snapshot flags a reset or teleport, set the **snap** flag.
7. Update the render offset (section 2.4).

Replay is bounded: at most 15 steps for a 250 ms lead (the worst playable case) and 128 for a very bad link, after which
the client stops predicting and waits (shows the "connection problem" marker, holds input at neutral).

### 2.4 Smoothing the correction

The physics car always jumps to the corrected state. What is drawn is `physics_pose + offset`, where `offset` holds the
unapplied part of past corrections:

- On a correction, `offset += old_physics_pose - new_physics_pose` (so the drawn pose does not move that frame).
- Each render frame `offset` decays exponentially with a half-life of 50 ms **(tuning)** (about 100 ms to be invisible).
- On **snap**, the offset is cleared and the drawn pose jumps; the camera is told so that it does not smear.
- The camera follows the drawn pose, not the physics pose.

Wheel spin and suspension travel are not smoothed; they follow the physics state.

### 2.5 What the server does with the input

The server keeps per player an **input buffer**: a map from tick to input, for ticks from `S` to `S + 16`.

1. A received input frame is validated (security) and its inputs for ticks greater than `S` and not yet filled are stored.
   Inputs for ticks at or below the last applied tick are late: they are counted (late counter) and dropped.
2. At tick `S` the server uses `input[S]` if present.
3. If missing, it **repeats the previous input** with throttle and brake held and the shift edges cleared (an edge must
   never repeat). After 6 missing ticks in a row (100 ms) it decays throttle to 0 and applies half brake, so a dead
   link does not drive the car into a wall at full speed. After 2 s without any input the car is parked (marked as
   lagging), and after the disconnect timeout it is removed.
4. It records for each tick whether the input was `early` (arrived before it was needed), `on time`, `late`, or
   `missing`, and reports a moving summary to the client (section 5.3).

The client sets its lead so that nearly all inputs are early by 1 to 2 ticks. A deeper server buffer would make inputs
reliable but would delay the effect of an input on the server state; a client that predicts does not feel the delay,
but other players see it. **Buffer depth is a tuning value**: 1 tick on LAN, 2 online.

### 2.6 When the server changes the car by itself

A contact with another car, a reset, a respawn, a teleport at the start of a race, a pursuit "busted" freeze, an
`Impound` or a ghost phase all change the owner's car without an input. The server marks the next own-car block with a
**cause** (`contact`, `reset`, `teleport`, `freeze`). The client treats `reset` and `teleport` as snaps. `contact`
goes through normal reconciliation. `freeze` holds the input neutral until the server releases the car.

## 3. Remote entities: interpolation and extrapolation

### 3.1 The buffer

For each remote entity the client keeps a ring of the last 16 received records (from snapshots), each with its server
tick. With deltas, the client reconstructs the full record at each received tick (baseline plus delta) before storing it.
Entities not mentioned in a delta keep their baseline record, and the record is stored again for the new tick (so
the buffer always has the state at each received tick).

### 3.2 Choosing the display time

`R = S_est - delay`, advanced at the same rate as `S_est`. The client finds the two records `a` and `b` with
`a.tick <= R < b.tick` and interpolates with `u = (R - a.tick) / (b.tick - a.tick)`.

The **adaptive allowance** raises `delay` by up to one snapshot interval when the measured snapshot arrival jitter
(a running 95th percentile of lateness) exceeds half an interval, and lowers it again slowly (one tick per second).
The delay never goes below two intervals. A change in delay is slewed over 1 s, not stepped.

### 3.3 Interpolating a car

| Part | Method |
|---|---|
| Position | Cubic Hermite on `a.position`, `b.position`, with tangents `a.velocity * dt` and `b.velocity * dt`. The curve passes through the samples and uses the velocity for smooth corners |
| Orientation | Spherical interpolation of the two quaternions (the shorter arc) |
| Wheel steer, compression, on-ground | Linear between the two; on-ground changes at `u = 0.5` |
| Wheel spin | Integrated from the speed: spin rate = forward speed / wheel radius, between the samples. Not sent |
| Engine rpm, gear | Gear switches at `u = 0.5`; rpm linear. Used only for the sound and effects of the remote car |
| Flags (lights, siren, nitrous burning, brake lights) | Take the value of `a` until `u = 0.5`, then `b` |

### 3.4 Extrapolating

When `R >= newest.tick` (the next snapshot is late):

1. Extrapolate from the newest record: position `p + v*t`, orientation rotated by angular velocity, for `t` up to 250 ms.
2. From 100 ms on, scale the velocity linearly to 0 at 250 ms so a stopped link does not shoot a car off the road.
3. At 250 ms, the car is held, marked **lagging** (a marker in the player list), and traffic or police with the same
   problem are faded out after 1 s.
4. When a snapshot finally arrives, the drawn pose blends to the interpolated pose over 100 ms (the same offset
   mechanism as section 2.4).

An entity whose record says `removed`, or that is missing from the baseline after a full snapshot, is despawned at
display time `R >= removal.tick`, so a car does not vanish earlier than it appears to have left.

### 3.5 Entities that appear

A new entity (first record) appears at display time `R >= first.tick`. Traffic fades in over 0.5 s on the client at
spawn (the single-player world already hides spawns, so this is a rendering matter only).

## 4. Collisions between cars

### 4.1 On the server

Per tick, after every car (players, traffic, police) has been integrated, the server resolves pairs with the existing
pair code (`CarSim::collide_with` in the scene today), in a fixed order: sorted by (entity id a, entity id b). World
contacts for each car were resolved during its own integration, as in single player. The result of a contact is a change
of the cars' velocities and positions, a `Collision` event for each car involved (impulse, point, other body type) and
damage inputs for `blackbox-damage`.

Ghost mode (MP-33) skips the pair for affected players. A **late-join phase** gives a new player 3 s of ghosting
against other players (not traffic) so a spawn on top of someone is harmless.

### 4.2 On the owner's client

The prediction does not include remote cars. The consequences and what we do about them:

| Situation | What the player sees | Mitigation |
|---|---|---|
| Rear-ending a remote car on LAN | A bump after under 10 ms; looks immediate | None needed |
| The same online (RTT 100 ms) | The car passes through the remote car's drawn position for up to 100 ms, then is pushed back | Near-field extrapolation (4.3); the `Collision` event plays sound and sparks on receipt |
| A remote car hits the owner | The owner's car is pushed when the snapshot arrives; no prediction of the push is possible | Accepted |
| Pushing a remote car | The remote car moves away after the server resolves it, `delay` later | Accepted |

### 4.3 Near-field extrapolation

Remote cars within 15 m of the owner's car are drawn at a **reduced display delay**: `delay_near = max(1 interval, 50 ms)`
instead of two intervals, by extrapolating from the newest record. They blend back to the normal delay when they leave
the 15 m ring (over 0.5 s, by the same offset mechanism). Reason: contacts are decided near the owner, and the
displayed positions there should match what the server has as closely as the data allows. Extrapolation errors for
cars in a contact are large (the car is about to be hit), so this is a tuning value to compare in play tests, not
a commitment: it ships off if tests show the visual jitter is worse than the phantom pass-through.

### 4.4 Optional proxy prediction (P1, off by default)

The owner's prediction may collide with **proxies** of nearby remote cars: kinematic boxes at the extrapolated pose
for time `C`, with the remote car's velocity, treated as heavy bodies that do not react (the owner's car takes the whole
impulse, scaled by the mass ratio). The server's result is still the truth and corrects it. Pros: the bounce appears
without waiting. Cons: a predicted bounce off a car that was already swerving away is a false hit, shown and then
undone. Build only if section 4.2 plays badly in tests (trigger in the ADR).

### 4.5 Traffic and police

Same rules: they are server entities, interpolated on clients. The owner's prediction does not collide with them. A hit
on traffic reaches the owner as a correction plus a `Collision` event, and traffic reacts on the server as in single
player.

## 5. Clock synchronisation

### 5.1 Measurement

The client sends `PING(seq, t_c1)` where `t_c1` is its monotonic clock in milliseconds. The server answers
`PONG(seq, t_c1, s_recv, s_send)` where `s_recv` and `s_send` are server tick numbers with 1/256 fractions at receive
and at send. The client notes `t_c2` on arrival and has:

- `rtt = (t_c2 - t_c1) - (s_send - s_recv) * 1000/60`
- `offset` = the server tick at the middle of the exchange minus the client's tick-time at the middle.

Samples are kept for 10 s; the estimate uses the sample with the lowest `rtt` among the last 10 (a high round trip means
queueing delay, which makes the offset wrong by an unknown amount). Rate: 4 pings per second for the first 5 s, then 1
per second.

### 5.2 The local clock and slewing

The client's tick accumulator advances with the monotonic clock times a **rate factor** `k` in `[0.98, 1.02]`:

- Error `e = S_est_target - S_est_now` (in ticks).
- `k = 1 + clamp(e * 0.1, -0.02, 0.02)`: a persistent error is closed in a few seconds without visible speed changes.
- If `|e| > 6 ticks` (100 ms), **step**: set the clock, clear the prediction ring, and flag a snap.

### 5.3 Controlling the lead

The server sends a report in the own-car block: for the last 32 ticks, how many inputs were early, on time, late and
missing, and the mean earliness. The client keeps `lead` so that earliness stays between 1 and 2 ticks:

- Mean earliness below 1 tick, or any late input: `lead += 1 tick`, at most once per 250 ms.
- Mean earliness above 3 ticks for 2 s: `lead -= 1 tick`, at most once per 2 s.

Changing the lead changes the client's `C` against `S_est`: when it grows, the client simulates extra ticks in one
frame (up to 2, bounded); when it shrinks it skips ticks of simulation but keeps its history (the car is re-based by the
next reconciliation). Both are rare.

### 5.4 Events with target ticks

Race start, countdowns, and pursuit state changes carry a target server tick `T`. The client acts when its `S_est >= T`.
Only the owner's launch matters: the owner's inputs before `T` are held neutral by the server, and the client holds them
neutral until its predicted `C >= T` (not `S_est`), so the car starts when the lights change on the owner's own clock.

## 6. Determinism requirements

The design does not need bit-identical simulation across machines. It needs the following, which should be audited and
tested before milestone 10 builds on them.

| # | Requirement | Reason | State today |
|---|---|---|---|
| D1 | The sim advances only in fixed 1/60 s steps | Server tick = predict step | Done and tested (see [physics-timing.md](../../physics-timing.md)) |
| D2 | No wall-clock or frame-time input in the sim | Headless server, replays | One `Instant` in `scenes/world/drive_input.rs` (a wait for input, not part of the sim); the rest to audit |
| D3 | No iteration over a `HashMap`, or any order that depends on a hasher, in sim results | Same inputs, same pair order | None found in the AI manager and simulation files read; full audit needed |
| D4 | Random numbers from seeded streams owned by the server, one stream per system, derived from the session seed with a fixed mixing (`SplitMix` exists in `libs/blackbox-roads`) | Replays, debugging | The pure crates take random numbers from the caller; the scene must stop using a shared source |
| D5 | Entities are processed in order of entity id | Pair order changes contact results | To implement |
| D6 | No change in the order of float operations because of threads inside a step | Reproducibility on one build | To confirm |
| D7 | A `sim_build` value (git hash plus target triple) sent in the handshake | Know when prediction is less exact | Protocol field |
| D8 | The owner's quantised inputs are used by the client's prediction | Prediction equals the server's step | Specified in 2.1 |

Float results of standard-library maths (sin, cos, pow) can differ by a bit between operating systems or CPU
architectures. Within one build and one platform the client and server agree exactly. Across platforms they agree to
about 1e-6 relative error per step; the error grows only until the next correction, so the correction rate rises but
the game stays correct.

**Replays** (MP-71, MP-72) on one build can be exact: inputs, seeds and start state replay the run. Across builds, a replay
is the snapshot stream, not the inputs.

## 7. Session lifecycle and state sync

| Moment | What is sent | Reliability |
|---|---|---|
| Join accepted | Session settings, player list, session seed, tick rate, the server tick `S`, the roster of entity ids | control stream |
| Loading | The client loads the track, streams collision, builds its car from the loadout ids; it reports `LoadComplete` | control stream |
| Spawn | The server places the car (spawn point, orientation) and sends a `teleport` own-car block; the first snapshot after is full (no baseline) | datagram plus event |
| Running | Snapshots, inputs, events | datagram, event stream |
| Late joiner in free roam | The same as a join; other players appear in the first full snapshot | |
| Leaving | A `Disconnect` message, or a timeout of 10 s; the car is removed on the next snapshot | control stream |
| Reconnect | A resume token (from the first join) within 30 s returns the slot; the server keeps the car as a ghost; the first snapshot after is full | control stream |

Full snapshots are used when: the client has no acknowledged baseline in the last 64 ticks; after a reset or teleport of
the owner; after a stall longer than 1 s. A full snapshot is split into parts if it exceeds one datagram
(see [protocol](protocol.md) section 6.3).

## 8. Failure behaviour

| Failure | Detection | Behaviour |
|---|---|---|
| Lost snapshot | The client's newest tick is older than expected | Interpolation buffer runs on; the next snapshot is a delta against the last acknowledged tick, so nothing is lost |
| Burst loss (> 200 ms) | Extrapolation passes 100 ms | Section 3.4; "connection problem" marker after 500 ms |
| Lost input packet | Missing ticks at the server | Redundancy (8 ticks per packet) covers 2 to 3 lost packets in a row; section 2.5 covers more |
| High jitter | Late counters, arrival jitter | Lead and delay grow (sections 3.2, 5.3) |
| Client frame stall (e.g. a streaming hitch) | The accumulator wants many ticks | Single-player behaviour is to simulate every elapsed tick ([physics-timing.md](../../physics-timing.md)). In a session the client simulates at most 6 ticks per frame; beyond that it drops the late ticks and the server's missing-input rule applies. The next correction snaps if needed |
| Server tick overrun (the world step takes more than 16.7 ms) | The server's own timer | The server runs extra ticks to catch up, up to 4 per loop, then skips (a "server overload" counter in the status). Snapshots keep their tick numbers, so clients see a pause, not a time shift |
| Clock step | Section 5.2 | Prediction ring cleared, snap |
| Peer address change (Wi-Fi roaming) | Handled by the connection id and the path challenge (protocol section 5.1) | The session moves to the new address after the challenge is answered; a short pause in snapshots, no reconnect |

## 9. Tuning values

| Value | Start | Where used |
|---|---|---|
| Snapshot rate | 20 Hz (30 Hz LAN option) | 3.2 |
| Interpolation delay | 2 intervals | 3.2 |
| Delay allowance | 0 to 1 interval | 3.2 |
| Extrapolation limit | 250 ms, velocity decay from 100 ms | 3.4 |
| Correction thresholds | 2 cm, 0.2 degrees, 0.05 m/s | 2.3 |
| Snap threshold | 2 m | 2.3 |
| Offset half-life | 50 ms | 2.4 |
| Input redundancy | 8 ticks per packet | 2.2 |
| Input buffer depth | 1 tick LAN, 2 online | 2.5 |
| Missing-input decay | after 6 ticks | 2.5 |
| Near-field ring | 15 m, delay 50 ms | 4.3 |
| Rate factor limits | 0.98 to 1.02 | 5.2 |
| Clock step threshold | 6 ticks | 5.2 |
| Prediction ring | 128 ticks | 2.1 |

## 10. Tests the model needs

1. **Unit:** prediction against a recorded server run with no contacts: the error stays under the thresholds for 10 minutes
   on one build. Rewind and replay equals a straight run (compare `state[t]` bit for bit on one build).
2. **Interpolation:** a car on a circle sampled at 20 Hz is drawn within 5 cm of the true circle.
3. **Simulator:** loss 0 to 10 %, RTT 0 to 250 ms, jitter 0 to 50 ms, reorder, duplicates (the transport simulator, MP-83).
   Record corrections per minute, snaps per minute, drawn-vs-server error.
4. **Contacts:** two clients drive into each other with scripted inputs at 100 ms RTT; measure how long the owner's car
   passes through the other's drawn pose with and without 4.3 and 4.4.
5. **Overload:** the server tick overrun and client stall cases of section 8.

## 11. Open questions

1. Is a reduced-state correction enough, or does the tyre and suspension state drift visibly? Decide after the first
   contact tests (ADR 0006, "Could not verify").
2. Near-field extrapolation against proxy prediction: which one first? Proposal: 4.3 first (cheap), 4.4 only on need.
3. Who owns the physics of a car being towed or carried (the trailer, `trailer.rs`)? Trailers are part of the car entity
   today; they need a record in the snapshot schema (position of the hitch and trailer angle).
