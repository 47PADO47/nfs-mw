# 0006 — Multiplayer netcode: a host-authoritative server over our own UDP layer

- **Status:** proposed (2026-10-16). Design only; nothing is built. Needs the owner's answers in "Owner decisions".
- **Milestone:** 10 (LAN multiplayer) and 11 (online multiplayer).
- **Companions** (all in [docs/specs/multiplayer/](../specs/multiplayer/README.md)): [features](../specs/multiplayer/features.md)
  (requirements, priorities), [protocol](../specs/multiplayer/protocol.md) (packet layouts, messages, versioning),
  [sync model](../specs/multiplayer/sync-model.md) (prediction, interpolation, collisions, clocks, determinism),
  [bandwidth](../specs/multiplayer/bandwidth.md) (the arithmetic), [transport options](../specs/multiplayer/transport-options.md)
  (crates evaluated, the reliability layer), [integration](../specs/multiplayer/integration.md) (Bevy shell, crates, order of work),
  and `security.md` with `security-gameplay.md` and `security-ops.md` (threat model, Noise handshake, identity, validation; written
  separately, referenced here, not repeated). Telemetry: [ADR 0007](0007-telemetry-server.md) and
  [multiplayer reuse](../specs/telemetry/multiplayer-reuse.md).

## Question

Milestone 10 adds LAN play and milestone 11 puts the same game on the internet. The game has a fixed 60 Hz deterministic car
simulation, a city shared with traffic and police, and a pursuit system that decides outcomes (busted, evaded). How do the machines
agree about what happened, and how do we build it once so that the online milestone adds services around it instead of replacing it?

To decide: topology, tick rates, who simulates what, how the player's own car feels instant, how other cars are shown and how they
collide, what is on the wire and in what bandwidth, the transport, clocks and determinism, how it fits the Bevy shell, the crate split,
and what we keep of the original LAN protocol.

## Facts checked on 2026-10-16

| Fact | Source | Consequence |
|---|---|---|
| One fixed clock at 60 ticks per second; the trace of every tick agrees bit for bit at 5 to 240 FPS and with 300 ms stalls | [physics-timing.md](../physics-timing.md), `drive/sim/tests/frame_rate.rs` | The sim does not depend on the render rate; a server can run it headless and a client can replay it |
| Per tick: integrate the player and all AI cars, resolve car pairs, then traffic, pursuit and spawn timers | physics-timing.md | The server runs the same order for all players |
| `Vehicle::step(dt, &InputState, &Ground)` is pure, `f32`, no I/O, `FIXED_STEP` = 1/60 s | `libs/blackbox-vehicle/src/lib.rs` | Prediction is "call the same function again" |
| `InputState` is small: throttle, brake, steer, handbrake, nitrous, shift edges, gear select, clutch | `libs/blackbox-vehicle/src/input/mod.rs` | About 5 bytes per tick before delta coding |
| Pursuit rules are pure functions that take random numbers from the caller | `libs/blackbox-pursuit` | The server runs them unchanged and owns the random numbers |
| Traffic and navigation take a `RandomSource`; `SplitMix` is a seeded 64-bit generator | `libs/blackbox-roads/src/nav/random.rs` | Seeded per-system streams exist already |
| Car pairs are resolved by `CarSim::collide_with` (`drive/sim/pair.rs`), world contacts first | pair.rs, physics-timing.md | One authority must resolve pairs; two machines would apply the impulse twice |
| The sim imports the binary crate's audio, settings, effects, events and props types | `drive/sim/mod.rs`, `pair.rs` | The sim is not headless today; extracting it is the biggest piece of milestone 10 |
| `Scene::update(renderer, &ActionState, dt)` and `frame()`; the shell is a Bevy `App` ([ADR 0001](0001-bevy.md)) | `crates/nfsmw/src/viewer/mod.rs` | A multiplayer world is another `Scene` plus a Bevy resource |
| Telemetry proposes a lock-free hand-off and says game sync must be a separate binary protocol | [multiplayer reuse](../specs/telemetry/multiplayer-reuse.md), ADR 0007 | The hand-off is shared; the wire format is new |
| The original LAN layout (25 against 29 bytes) is unverified and depends on EA's closed `server.dll` | [lan-server.md](../specs/lan-server.md) | Nothing to build on |
| `quinn` 0.11.12 (2026-09-14), `renet` 2.0.0 (2026-01-20), `laminar` 0.5.0 (2021), `postcard` 1.1.3, `socket2` 0.6.5 | crates.io API | The full table, with licences, is in [transport options](../specs/multiplayer/transport-options.md); all candidates pass the `deny.toml` allow list on their published strings |

## Options

**Topology.**

| Option | For | Against |
|---|---|---|
| **A. Client-server, host-authoritative.** One server simulates the world, every player is a client. A listen server runs in a player's game; a dedicated server is the same code without a window | One referee for collisions, pursuits and results. The server never trusts a client. LAN and internet are the same thing with another address. Spectators, replays and telemetry read the same stream. Scales to 16 | The host pays for the server and has no latency. The sim must be separable from rendering (needed anyway) |
| B. Peer-to-peer lockstep or rollback | No server; inputs only | Needs a bit-identical world (AI, traffic, props, floats), which nothing guarantees; rollback of a 16-car world with traffic is too costly; easy to cheat; NAT traversal for a mesh |
| C. P2P with each peer owning its car | Simple | A contact between two owned cars has two owners; traffic and cops still need an owner; N(N-1) links; trust |
| D. A relay server | Solves NAT for P2P | A transport, not a topology. Kept as an optional online add-on |

**Where player cars are simulated.**

| Option | For | Against |
|---|---|---|
| **1. Server-authoritative, client prediction and reconciliation** | One truth for contacts, damage, busted, finish. Speed hacks and teleports do not work. Our sim is deterministic and cheap, so prediction is a re-run | A rewind-and-replay and a state history on the client. A contact with a remote car is felt after a round trip |
| 2. Client-authoritative with server validation | Best feel for the owner; no rewind | Two owners meet in a contact: both apply an impulse, or the server picks one and fixes the other; validation is heuristics; cheating by default |

**Transport** (detail in [transport options](../specs/multiplayer/transport-options.md)): our own layer on UDP with the Noise channel
of `security.md`; QUIC (`quinn`); `renet`; `laminar`, `naia`, `enet` (rejected).

## Decision

**Topology A, authority 1, over our own UDP layer.**

1. **One server program.** The server runs headless in a library. A listen server runs it on a thread of the host's process and the
   host's game connects to it over loopback like any client; the dedicated server (`nfsmw-server`) is the same code with no window.
   LAN and online use the same binary and protocol.
2. **Server tick = physics step = 60 Hz.** Snapshots every third tick (20 Hz) by default; 30 Hz as a LAN setting.
3. **Player cars:** server-authoritative; the owner predicts its car and reconciles.
4. **Traffic, police, pursuits, races, damage, busted:** simulated only on the server and replicated as entities. Clients run no AI.
   Random numbers come from seeded per-system streams owned by the server.
5. **Remote cars:** snapshot interpolation two snapshot intervals in the past, with bounded extrapolation; no remote physics.
6. **Car-to-car collisions:** resolved on the server only, with the existing pair code. The owner's prediction includes the world, not
   other cars.
7. **Transport:** UDP (`std::net` or `socket2`) with our reliability layer (packet numbers, acknowledgement bits, four reliable
   channels, fragments) behind a `Transport` trait that also has an in-memory and a packet-simulator implementation. The secure
   channel is `security.md` S-01: a Noise handshake (`snow`, `XX` on LAN, `XK` online), a stateless cookie, ChaCha20-Poly1305 sealing
   with the packet number as nonce. QUIC stays a possible second transport behind the same trait.
8. **Encoding:** hand-written bit packing for input, snapshot and ping; `serde` with `postcard` for everything else.
9. **Crates:** `libs/blackbox-net-core` (generic, with cargo features `core` and `netcode`), `crates/nfsmw-net` (the game's schemas,
   prediction, interpolation), `crates/nfsmw-sim` (the extracted headless world) and the binary `crates/nfsmw-server`.
10. **Original LAN protocol:** not kept (section 15).

## 1. Topology and the online path

The server is a state machine with a socket: it reads packets, applies inputs at the tick they are for, steps the world, and writes
snapshots. It does not know whether it is a LAN host or an internet server. What milestone 11 adds is around it:

| Concern | LAN (M10) | Online (M11) |
|---|---|---|
| Finding a server | Broadcast discovery; a typed address | A server list from a master service; invite codes |
| Reaching a server | Same subnet | Port forward or a public host; an optional UDP relay for NAT (a forwarder that sees only sealed packets) |
| Identity | Pinned key (Noise `XX`), display name, optional password | Account login and server-signed connect tokens (Noise `XK`) |
| Budgets | 32 KB/s per client cap | 12 KB/s default, RTT up to 150 ms playable |
| Tick, snapshot format, prediction, collisions | same | same |

The listen server host has no latency. We accept that on LAN and recommend a dedicated server for competition (owner decision 7). The
host's own client takes the same code path as a remote client, handshake included.

## 2. Tick rate against the physics step

The physics step is a fixed 1/60 s; the whole sim advances in those steps. The server therefore ticks at **60 Hz** with no resampling
and no second clock. A lower tick (30 Hz) would make the server integrate with a different step from the client's prediction and from
single player, breaking "predict by re-running the same function". A higher tick has no use.

| Message | Rate | Reason |
|---|---|---|
| Client input | one input per tick; packets at 30 Hz, each with the last 8 ticks | Redundancy covers loss without resends; half the packet overhead |
| Snapshot | 20 Hz default, 30 Hz LAN option, 10 Hz for far entities | Bandwidth; interpolation hides the rate |
| Ping and clock sync | 1 Hz (4 Hz for the first 5 s) | Cheap |
| Events (reliable) | when they happen | A pursuit start should not wait for a snapshot |

## 3. What is simulated where

| Thing | Where | Why |
|---|---|---|
| A player's car | Server (authoritative) and the owner's client (prediction) | See below |
| Other players' cars | Server; clients interpolate | A second sim of a remote car would diverge |
| Traffic | Server only | It reacts to every player; client-side AI would need a bit-identical world |
| Police (cops, helicopter, roadblocks, spikes) | Server only | They decide busted and evade |
| Pursuit state, race state, damage, punctures | Server; sent as state blocks and events | Outcomes must be one truth |
| Moving props | Server, as low-priority entities; static props are not synchronised | A moving prop is a contact partner |
| Effects, sound, camera, HUD | Client | Not gameplay state |
| Time of day | Server (value and rate in the session state) | One clock for all |

**Why server-authoritative with prediction.**

- *One referee for contacts.* The pair code applies one impulse exchange to two cars. With each client owning its car, both would
  run it from slightly different poses and disagree.
- *Cheating.* Milestone 11 needs the server to decide; a client-authoritative start would be rewritten.
- *The sim is cheap to re-run.* Prediction replays `Vehicle::step` for the ticks since the last server state: 6 steps at 100 ms, 15 at
  a poor 250 ms. The cost of one step is not measured (a ray cast is 1.3 µs per [architecture.md](../architecture.md); a step does
  four wheel casts plus the model); 15 steps should stay far below 1 ms.
- *The price.* A contact with another player is felt after a round trip: under 10 ms on LAN, 50 to 150 ms online, where the car "soft
  bumps" ([sync model](../specs/multiplayer/sync-model.md) section 4).

Client-authoritative was rejected for the double-contact problem and the online goal, not for feel; it feels better for the owner.

## 4. Prediction and reconciliation

Full algorithm: [sync model](../specs/multiplayer/sync-model.md) section 2. The client steps its car at fixed 60 Hz ahead of the
server by `RTT/2` plus an input buffer of 1 tick (LAN) or 2 (online), using **quantised** inputs so that the server steps the identical
numbers. A ring of 128 ticks keeps inputs and reduced states. Each snapshot carries the authoritative reduced state at the last
applied input tick; if the error exceeds 2 cm, 0.2 degrees or 0.05 m/s the client rewinds and replays. A render offset decays with a
50 ms half-life, so a correction of a few centimetres is not a jump; an error over 2 m, or a reset, is a snap.

The **reduced state** is pose, velocities, rpm, gear, nitrous and boost. Tyre and suspension internals are not synchronised and
converge in a few ticks. If tests show drift, the fallback is a full-state correction, which needs a stable serialisable `Vehicle`
state (a library change to make only then).

## 5. Remote cars: interpolation and extrapolation

Other players, traffic and police are drawn from a buffer of snapshots, **two snapshot intervals in the past** (100 ms at 20 Hz, 66 ms
at 30 Hz) plus an adaptive allowance of 0 to 1 interval for jitter. Position is cubic Hermite from the two positions and velocities,
orientation is spherical, wheel spin is integrated from speed. If a snapshot is late the car is **extrapolated** up to 250 ms, the
velocity decaying from 100 ms, then held and marked lagging. Cars within 15 m of the owner use a shorter delay (sync model 4.3).

## 6. Collisions between cars

The server resolves pairs once per tick, after integration, in entity-id order, with the existing pair code and then the existing
damage inputs; clients draw the result. The owner's prediction includes world contacts and static props but **not** contacts with
remote entities, so a contact is felt after about one round trip. The server sends a `Collision` event (impulse, point) so sound and
sparks play on receipt. A proxy prediction against extrapolated nearby cars is a P1 option, off by default (sync model 4.4). Ghost
modes skip the pair on the server and draw the cars semi-transparent.

## 7. Snapshots: delta compression and relevancy

The server keeps the last 64 world states, quantised. A snapshot is a delta against the **last snapshot the client acknowledged**. The
acknowledgement is the packet-level acknowledgement of the sealed packet (a snapshot is acknowledged when its packets are), so no field
of the input carries it. With no usable baseline (none acknowledged in 64 ticks, a join, a stall over 1 s, a reset) the snapshot is
full. Per entity: unchanged (not listed), a delta with a field mask, a full record, or a remove.

**Relevancy** decides which entities a client receives:

| Tier | Range from the client's car | Content | Rate |
|---|---|---|---|
| Always | any | Other players in a reduced record (10 B): pose, speed, role and state flags, for the minimap, names and race order | 10 Hz |
| Near | up to 150 m | Full records for players, traffic and police | snapshot rate |
| Mid | 150 to 400 m | Traffic and police: pose and heading only. Players: full | half rate |
| Far | beyond 400 m | Traffic and police not sent (beyond draw and streaming distance) | none |
| Pursuit | any | Police chasing this player, and the roadblocks and helicopter of its pursuit, at full detail | snapshot rate |

A 64 m spatial hash makes the range query a few cells. A **priority accumulator** per client and entity (importance times time since
last sent) fills each packet up to its byte budget with the highest priority first, so a budget that is too small drops traffic detail
before it drops players. The visibility limit also serves security (`security-gameplay.md` section 3).

**Entity ids** are 8 bits on the wire, assigned by the server, never ECS handles; the player slots are the first 32. This confirms the
rule in ADR 0007 and [multiplayer reuse](../specs/telemetry/multiplayer-reuse.md) section 6: **telemetry's `u32` entity field is the
low 8 bits (the wire id) plus a 24-bit generation counter that rises when an id is reused.** The generation is a telemetry-side
concept; it is never sent on the wire, so the game protocol stays at 8 bits. If the id space widens later, telemetry follows.

## 8. Bandwidth budget

Arithmetic and record sizes: [bandwidth](../specs/multiplayer/bandwidth.md). The overhead per datagram is 59 bytes (IP and UDP 28,
header 7, acknowledgement block 8, tag 16). A delta player-car record is about 24 bytes, a traffic car 10, a police car 16 (estimates).

| Case | Rate | Payload | Datagram | Per client | Server total |
|---|---|---|---|---|---|
| A. LAN, 8 players, 40 traffic, 8 police | 20 Hz | 766 B | 825 B | 16.1 KB/s (129 kbit/s) | 129 KB/s (1.03 Mbit/s) |
| B. LAN, 16 players | 30 Hz | 958 B | 1,017 B | 29.8 KB/s (238 kbit/s), under the 32 KB/s cap | 477 KB/s (3.8 Mbit/s) |
| C. Online, 16 players, 12 KB/s budget | 20 Hz | 541 B (budget) | 600 B | 12 KB/s: 6 near and 9 far players, about 15 traffic and 5 police cars | 192 KB/s (1.5 Mbit/s) |
| D. Upstream from one client | 30 Hz | 20 B | 79 B | about 2.5 KB/s (20 kbit/s) | |

Server CPU budget: 4 ms of 16.7 ms per tick for the world and 0.5 ms for the network; the world step is not measured and is the first
benchmark.

## 9. Transport

The game needs unreliable frames with our own sequencing and a packet-level acknowledgement for delta baselines, reliable ordered
channels, per-packet control of size and time to meet the budgets, a stateless address check before handshake work, a loopback path and
a packet simulator, and few dependencies. The options are compared in [transport options](../specs/multiplayer/transport-options.md);
the decision and its reasons:

- **Own UDP layer, chosen.** A snapshot is filled to a byte budget by a priority accumulator and diffed against a packet-acknowledged
  baseline; a generic message layer makes that awkward. The same primitives (hand-off queues, framing, limits) are shared with the
  telemetry server. The secure channel of `security.md` (Noise, cookie, sealed header with our packet number) assumes a layer where we
  own the header. The reliability layer is small and well understood (an estimate of 1.5 to 2.5 kLOC with tests).
- **QUIC (`quinn`), not chosen for v1.** It would give a reviewed handshake for free, but needs a runtime and certificates for a LAN
  with no host names, its congestion control can delay datagrams after loss, and datagram delivery is not reported, so we would add our
  own acknowledgements anyway. It is the documented second transport (web spectators, a TLS-protected telemetry mode), and the fallback
  if review judges our handshake too risky (`security-ops.md` gate G3).
- **`renet`, not chosen.** It saves the reliability code, but makes whole messages the unit, assumes a token issuer, and has no forward
  secrecy. A one-week spike is owner decision 3.
- **The risk we accept:** bugs in our reliability, fragmentation and handshake code, which face the network. Mitigations: a packet
  simulator in the test suite, property tests, a fuzz target per parser (`security-gameplay.md` section 2), and the `Transport` trait.
- **Congestion:** the connection has a byte budget and two loss and RTT signals that lower its snapshot rate ([transport options](../specs/multiplayer/transport-options.md)
  section 4.3). This is a known weakness for online play.

## 10. Serialization

| Message class | Format | Reason |
|---|---|---|
| Input, snapshot, ping | Hand-written bit packing (`Packable` trait), quantisation tables in `nfsmw-net` | The budgets need 10-bit and 14-bit fields |
| Lobby, chat, events, session, admission | `serde` with `postcard` | Low rate; documented stable wire format; `serde` is already a workspace dependency |
| Handshake packets | Fixed layouts | The smallest parser before authentication |

`bitcode` is rejected because its format is not stable across versions; `bincode` was not evaluated. Postcard enums are append-only.
Every reliable message has a type, a one-byte format and a length, so an unknown type is skipped
([protocol](../specs/multiplayer/protocol.md) section 8).

## 11. Clock synchronisation

The server tick is the clock. Nothing in the protocol is timestamped with wall-clock time (the cookie bucket and connect-token expiry
are the only wall-clock uses, and they are outside the game protocol; ADR 0007 states the same sim-time rule). The client estimates the
server tick with NTP-style pings, keeps the lowest-RTT sample of the last 10, and **slews** its tick rate within ±2 %; it steps only
above 100 ms of error. It sets its lead from a server report of how early inputs arrived. Events that must happen together carry a
target tick. Details: [sync model](../specs/multiplayer/sync-model.md) section 5.

## 12. Determinism requirements

Bit-identical simulation across machines is **not** required, because the server is authoritative and snapshots carry state. What the
design does need (list and status in [sync model](../specs/multiplayer/sync-model.md) section 6): fixed steps only, no wall-clock in the
sim, no hash-order iteration in results, seeded per-system random streams derived from the session seed, entity processing in id order,
quantised inputs used by the prediction, and a `sim_build` id in the handshake. One `Instant` was found in the world scene
(`scenes/world/drive_input.rs`, outside the sim); the rest needs an audit. On one build and platform the client and server agree
exactly; across builds or operating systems, standard-library maths can differ in the last bits, so corrections are more frequent but the
game stays correct. Replays on one build can be exact.

## 13. Mapping onto the Bevy shell and `Box<dyn Scene>`

A `NetSession` Bevy resource owns the connection state and the lock-free queues to a network thread (the same hand-off primitives as the
telemetry server); a `NetWorldScene` implements `Scene` (drains the inbox, runs the accumulator for the owner's car, reconciles,
advances interpolation, builds the instance list through the code that draws AI cars); the listen server is a thread running the
extracted `WorldSim`. The pause menu never pauses a session; the client holds its car in a loading state while its area streams.
Console commands `net.*` use the existing command table. Details: [integration](../specs/multiplayer/integration.md) sections 1 and 2.

## 14. Where the code lives

| Piece | Location | Notes |
|---|---|---|
| Hand-off primitives, ids, sim time, token, rate limiter | `libs/blackbox-net-core`, feature **`core`** (default) | The part ADR 0007 uses. **Confirmed:** telemetry enables only `core` |
| Bit packing, quantisers, reliability, `Transport` trait and its UDP, memory and simulator implementations, handshake and sealing (`snow`), clock sync, snapshot machinery, spatial hash, priority accumulator | `libs/blackbox-net-core`, feature **`netcode`** | **Confirmed:** the feature split ADR 0007 asks for. Telemetry never enables it, so it pulls no crypto. If the snapshot machinery passes a few thousand lines it moves to `blackbox-net-replication` |
| Entity schemas, quantisation ranges, input mapping, events, settings, prediction, interpolation | `crates/nfsmw-net` | Bevy-free |
| Headless world | `crates/nfsmw-sim` | Extracted from `scenes/world`; the largest piece of work |
| Server loop, TOML config, logging | `crates/nfsmw-server` (binary) | No Bevy, no window |
| `NetSession`, `NetWorldScene`, lobby screens, console | `crates/nfsmw` | The only Bevy-aware part, behind a `multiplayer` cargo feature |

`libs/` has no NFS:MW names: the generic layer sees entity kinds as small integers and fields as a schema the game registers.
`crates/` depends on `libs/`, never the reverse. `cargo deny` must run with `--all-features` in CI so that the `netcode` dependencies
are checked, since `deny.toml` sets `all-features = false`. The extraction plan is in [integration](../specs/multiplayer/integration.md)
section 4.

## 15. The original LAN protocol

**Nothing of it is kept.** No wire format, no message, no port and no code. Reasons:

1. It is unverified: the sender builds a 25-byte datagram and the receiver reads a 29-byte layout ([lan-server.md](../specs/lan-server.md),
   status line). No capture exists.
2. It is the lobby and relay layer of EA's closed `server.dll`, built around EA accounts, a redirector, a master server and HTTP calls to
   `*pcnfs06.ea.com`. It is not a game-sync protocol.
3. The original client runs a different simulation. A rewrite client and a `speed.exe` client could not share a world.
4. It has no authentication or encryption, and the community launcher has a known out-of-bounds write and unsynchronised shared state
   (lan-server.md, "Known problems"). We should not inherit a 2005 security model.
5. Using it means patching `speed.exe` and loading EA's `server.dll`, outside the clean-room approach of [licensing.md](../licensing.md).
6. The community server emulator ([nfs-online-server.md](../mods/nfs-online-server.md)) is AGPL-3.0: its behaviour may be described, its
   code and tables never copied.

`security.md` allows an optional, off-by-default, LAN-only compatibility module if it is ever wanted; this ADR does not plan one.
Kept as **ideas** in our own words: a LAN game list found by broadcast; a server advertising a local and a public address; timeouts in
seconds in the server config (the old `PINGTIME` 20 and `PINGWAIT` 30; ours are a 2 s keep-alive and a 10 s idle timeout); separating
lobby control from race traffic (as channels of one connection, not as ports). Our default port is chosen not to collide with the ports in
the old tools (9900, 9901, 20000, 20923, 30920, 30921, 13505).

## Consequences

- **Costs.** The `nfsmw-sim` extraction is a large refactor and the gate to everything else; single player must keep its bit-exact
  frame-rate trace throughout. We write and fuzz a reliability layer, a handshake driver and a delta codec (an estimate, not measured:
  4 to 6 kLOC with tests). The owner feels contacts with other players after a round trip. The listen host has no latency. Congestion
  handling is simple.
- **Benefits.** One design for LAN and online. Cheating needs no special design. Few dependencies (`socket2`, `snow` and its crypto
  crates, `postcard`). The server is testable without a window or GPU. Spectators, replays and telemetry read the same stream. AI cost
  is paid once.
- **Bandwidth** is small (case A is 129 kbit/s per client); the budget leaves room for voice later (about 24 kbit/s per speaker is an
  assumption for an Opus stream).
- **Licences.** The candidates pass the current allow list on their published strings; `snow` and its crates are to be confirmed by
  `cargo deny`. No GPL or AGPL code is used.
- **Security** is `security.md`; where it disagrees with this ADR, it wins and this ADR is amended.

## How we will know

1. **Sim cost benchmark** (first): one `Vehicle::step`, a pair test, one traffic car, one cop per tick. Gate: the world step for 16
   players and 60 AI cars under 4 ms on the reference PC.
2. **Codec tests:** property tests for every quantised field (error under the step); delta round trips against random baselines; a fuzz
   target per parser.
3. **Network simulator suite:** loss 0, 2, 5, 10 %, RTT 0 to 250 ms, jitter 0 to 50 ms, reorder, duplicates. Measure corrections, hard
   snaps per minute, snapshot age and bandwidth against [features](../specs/multiplayer/features.md) section 4.
4. **Bandwidth** against [bandwidth](../specs/multiplayer/bandwidth.md) in free roam with 8 and 16 bots and a scripted pursuit.
5. **Determinism check** on one build: two servers with the same inputs and seeds produce equal snapshot hashes for 10 minutes; a
   recorded run replays exactly.
6. **Reliability layer:** under the simulator, reliable messages arrive in order exactly once, with no growth in queues, at 10 % loss.
7. **Soak:** 8 clients for 1 hour; no growth in memory or in the correction rate.

## Revisit triggers

- The reliability layer costs more than a `renet` spike shows: adopt `renet` behind `Transport` and re-check `security.md`.
- Review or fuzzing finds our handshake or replay code too risky: move to QUIC (`security-ops.md` G3); messages and codecs stay.
- The `nfsmw-sim` extraction costs more than the rest of milestone 10 together: ship a listen server inside the full scene first.
- The round-trip bump feel online is unacceptable: build the proxy prediction, or reconsider client authority for contacts only.
- Browser spectating becomes a requirement: add a QUIC or WebTransport `Transport`.
- A lockstep mode is wanted for one game mode: the determinism audit becomes a requirement for it.

## Owner decisions this ADR asks for

1. Accept the topology (one host-authoritative server for LAN and online) and server-authoritative player cars.
2. Accept that the first LAN release is free roam and pursuit, with races added as milestone 7 provides them
   ([features](../specs/multiplayer/features.md) open question 1).
3. Approve a one-week `renet` spike before milestone 10 starts, to confirm or reverse the own-layer decision.
4. Approve `crates/nfsmw-sim` as a new library crate and the extraction as the first task of milestone 10.
5. The default game port (one fixed UDP port outside the old tools' ports; chosen when the code is written).
6. The rule for a client whose area is not yet streamed: hold its car and flag a stall (proposed), or make the server wait up to a limit.
7. Whether a listen-server host may play in a competitive session.

## Could not verify

- The cost of one `Vehicle::step` and of the world step (no benchmark exists).
- The real extent of the city, which sets the position field width (the ±8,192 m is an assumption).
- The number of traffic cars and cops in range in a real session (40 and 8 are assumptions; the traffic cap was not read).
- Whether a `Vehicle` state can be saved and restored without a library change; reduced-state reconciliation avoids needing it.
- The size estimate of our layer, and the behaviour of `renet`, `quinn` and `snow` in practice (only crate metadata was read).
- The licence and version of `snow` and its crypto crates; `aws-lc-sys` and `bincode` status.
- Whether a mixed Windows and Linux session predicts well (standard-library maths was not compared between platforms).
- Anything about the original protocol beyond what [lan-server.md](../specs/lan-server.md) records.

## Sources

Read for this ADR: [lan-server.md](../specs/lan-server.md), [nfs-online-server.md](../mods/nfs-online-server.md) (behaviour only; AGPL),
[telemetry multiplayer reuse](../specs/telemetry/multiplayer-reuse.md), [ADR 0007](0007-telemetry-server.md),
[physics-timing.md](../physics-timing.md), [architecture.md](../architecture.md), [ADR 0001](0001-bevy.md), [ADR 0004](0004-swappable-renderers.md),
`security.md`, `security-gameplay.md` and `security-ops.md`, and the crate metadata at <https://crates.io/api/v1/crates/> (retrieved
2026-10-16). Techniques named here (snapshot interpolation, prediction with reconciliation, delta snapshots against an acknowledged
baseline, priority accumulators, packet numbers with acknowledgement bits) are public general knowledge in networked-game design; this ADR
describes them in its own words and copies no code.
