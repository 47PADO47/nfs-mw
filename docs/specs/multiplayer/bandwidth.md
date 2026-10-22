# Multiplayer bandwidth budget

Status: written 2026-10-16 as design arithmetic; section 7 adds the first measurements of the built codec (2026-10-21,
[ADR 0009](../../decisions/0009-multiplayer-netcode-revisions.md)). The arithmetic behind
[ADR 0006](../../decisions/0006-multiplayer-netcode.md) section 8. All sizes are computed from the field widths of the
[protocol](protocol.md) (section 6); the tests of the ADR ("How we will know") replace the estimates with measurements.
Averages marked *estimate* are guesses from field widths, not data.

## 1. Record sizes

### 1.1 A player car

| Field | Bits | Range and step |
|---|---|---|
| Position x, y, z | 3 × 20 = 60 | ±8,192 m in 2 cm steps (an assumed city extent; set by `world_bounds` at join) |
| Orientation | 32 | smallest three of the unit quaternion: 2-bit index and 3 × 10 bits |
| Linear velocity | 3 × 14 = 42 | ±400 m/s in 5 cm/s steps |
| Angular velocity | 3 × 10 = 30 | ±16 rad/s in 0.03 rad/s steps |
| Wheels: compression 4 × 6, on-ground 4 × 1, steer 8 | 36 | |
| Skid and smoke flags (2 bits per wheel) | 8 | |
| Engine: rpm 10, gear 4, flags 8 | 22 | rpm in 10 rpm steps |
| Inputs for the remote look: throttle 6, brake 6, steer 8 | 20 | |
| **Full record** | **250 bits = 32 bytes** | |

A delta record against the acknowledged baseline averages about 22 bytes (*estimate*: most ticks change position, orientation
and velocities, and few of the wheel and engine groups) plus a 2-byte entity header (id and field mask). The figure used below is
**24 bytes per player car**.

### 1.2 Other entities (*estimates*)

| Entity | Bytes | Composition |
|---|---|---|
| Traffic car | 10 | position 60 bits, yaw 8, speed 8, flags 4, id and mask 12 |
| Police car | 16 | traffic plus siren and lights, goal 3 bits, target 5 bits, more precision |
| Reduced player record (the "always" tier) | 10 | position, yaw, speed, role and state flags |
| Roadblock or spike | 8 | position, yaw, state |
| Helicopter | 15 | position, orientation, rotor and spotlight state |

### 1.3 Blocks

| Block | Bytes |
|---|---|
| Own-car correction block (reduced authoritative state of the receiving player's car, input report) | 32 (about 230 bits used) |
| Global block: session phase, this player's heat, busted fraction, cooldown, pursuit state, race place, lap and checkpoint, time of day | 12 |
| Snapshot frame header (type, tick, baseline delta, part, flags, input acknowledgement) | 10 |
| Reliable-message frames carried in the same packets (average) | 16 |

## 2. Packet overhead

One sealed datagram ([protocol](protocol.md) section 5): IP and UDP headers 28 B; our header 7 B (flags 1, connection id 2,
packet number 4); the ack fields inside the sealed payload 8 B (largest acknowledged packet 4, 32-bit ack field 4); the
authentication tag 16 B. **Total 59 bytes per datagram.** The payload limit is 1,200 bytes of UDP payload, so the frame
space is 1,200 − 7 − 8 − 16 = 1,169 bytes.

## 3. Cases

### Case A. LAN, 8 players, 20 Hz, to one client

| Item | Bytes |
|---|---|
| 7 other players × 24 | 168 |
| Own-car correction block | 32 |
| 40 traffic cars in range × 10 (assumed count) | 400 |
| 8 police cars × 16 (assumed count) | 128 |
| Global block | 12 |
| Snapshot header | 10 |
| Reliable-message frames (average) | 16 |
| **Payload** | **766** |
| Overhead | 59 |
| **Datagram** | **825** |

825 B × 20 Hz = 16,500 B/s: **16.1 KB/s (129 kbit/s) per client**. The server sends 8 × 16.1 = **129 KB/s (1.03 Mbit/s)**.

### Case B. LAN, 16 players, 30 Hz

Other players 15 × 24 = 360; own 32; traffic 400; police 128; global 12; header 10; reliable 16 = 958 B payload, plus 59 =
1,017 B. × 30 Hz = 30,510 B/s: **29.8 KB/s (238 kbit/s) per client**; the server sends 16 × 29.8 = **477 KB/s (3.8 Mbit/s)**. It
fits one datagram (958 < 1,169). It is just under the 32 KB/s LAN cap; if the cap is reached the priority accumulator trims
traffic and police detail first.

### Case C. Online, 16 players, 20 Hz, 12 KB/s budget

Budget per datagram: 12,000 / 20 = 600 B, minus 59 = 541 B of payload. Fixed part: header 10 + global 12 + own 32 + reliable 16
= 70. Six near players × 24 = 144 and nine far players × 10 (reduced record) = 90, together 234. Left for the world:
541 − 70 − 234 = 237 B, about 15 traffic cars (150 B) and 5 police cars (80 B). Anything beyond is dropped by priority, farthest
first. A pursuit adds its chasing police at full detail ahead of the traffic.

### Case D. Upstream from one client

An input is about 40 bits when it changes and 1 bit when it equals the previous tick. Eight ticks with three changes:
3 × 40 + 5 × 1 = 125 bits = 16 B. The input frame header is 4 B (type, first tick, count and flags). The frame is 20 B; with 59 B
of overhead the datagram is 79 B. × 30 Hz = 2,370 B/s. With pings (about 70 B per second), acknowledgement-only datagrams when
no input is queued, and the occasional reliable message, **about 2.5 KB/s (20 kbit/s)**.

### Case E. Spectator

Receives the near record of every player (16 × 24 = 384 B), plus traffic and police by tier: about case B. Sends only
acknowledgement datagrams at 10 Hz: (8 B ack + 59 B) × 10 = 670 B/s.

### Case F. Join and bulk

The late-join state is one bulk transfer on the reliable bulk channel: the session settings, the roster, the loadouts and the
mod manifests, about 2 to 8 KB (*estimate*). It is paced at no more than a quarter of the client's budget so that it never
starves the snapshots.

## 4. Server totals

| Players | Snapshot rate | To all clients | From all clients |
|---|---|---|---|
| 8 | 20 Hz | 129 KB/s (1.03 Mbit/s) | 8 × 2.5 = 20 KB/s |
| 16 | 30 Hz | 477 KB/s (3.8 Mbit/s) | 16 × 2.5 = 40 KB/s |
| 16 (online, 12 KB/s each) | 20 Hz | 192 KB/s (1.5 Mbit/s) | 40 KB/s |

An operator's uplink must carry the "to all clients" column; the online 16-player default is about 1.5 Mbit/s of upload.

## 5. Server CPU

Per second: 60 world steps; at 16 players and 20 Hz 320 snapshot encodes (each about 60 entities, a few microseconds each, so
under 2 ms in total per second); 480 input decodes; 320 + 480 AEAD seals and opens of about 1 KB (ChaCha20-Poly1305 runs at
more than 1 GB/s on a desktop CPU, so well under 1 ms per second). The budget is **4 ms of 16.7 ms per tick for the world** and
**0.5 ms for the network**. The world step itself is not measured and is the first benchmark of the plan.

## 6. Levers when a budget is exceeded

In order of use: lower the snapshot rate for far entities; widen the quantisation steps of traffic and police (position 2 cm to
5 cm saves 6 bits each); reduce the mid-tier range; drop traffic detail by priority; lower the player-car rate to 15 Hz; reduce the
number of traffic cars per player (the density setting). Each has a setting and a counter in the status line (MP-82).

## 7. ADR 0009 changes and first measurements

**Entry header.** ADR 0006 spent 16 bits per listed entity on an 8-bit id and an 8-bit group mask. ADR 0009 spends an id gap
(4 bits for consecutive ids, 8 bits for gaps under 64), a 2-bit operation and a mask of one bit per field. For the player car
(about 20 fields) that is about the same as before. For traffic and police (6 to 8 fields) it is 4 to 6 bits cheaper. Frame
lengths add 1 or 2 bytes per frame. Difference-coded positions cost about 8 to 12 bits per axis for a car moving at road speed
between 20 Hz snapshots, against 20 bits raw. The record sizes of section 1 are therefore upper bounds for moving entities.

**Measured, synthetic scene** (`cargo run --release -p blackbox-net-core --example net_bench`, and the loopback test
`libs/blackbox-net-core/tests/loopback.rs`). Setup: 16 clients, 300 entities moving on circles, 20 Hz snapshots, 400 m
relevancy. The kinds are small: a car-like kind of 6 fields (84 bits full) and a traffic-like kind of 4 fields. They are not the
game's schema, so the byte figures measure the codec, not case A or B.

| Measure | Value |
|---|---|
| Snapshot payload per client (bench, no loss) | 144 B average, 2.9 KB/s |
| Snapshot payload per client (loopback, 2 % loss, 20 ms ± 5 ms) | 154 B average, 3.4 KB/s per client including headers |
| Encode time per snapshot tick, all 16 clients, shared cache | 0.032 ms |
| The same with a cache reset before every client (no sharing) | 0.056 ms (sharing is 1.76× faster) |
| Encode-cache hit rate | 86 to 88 % |
| Snapshots received per client (loopback, 2 % loss) | 582 to 593 of 600 |
| Allocations per tick after warm-up (8 clients, 300 entities: history, encode, packet write and read, decode) | 0 (`tests/alloc.rs`) |

The game's figures follow when `nfsmw-net` registers the real schema. The bench will then be rerun against cases A to C.
