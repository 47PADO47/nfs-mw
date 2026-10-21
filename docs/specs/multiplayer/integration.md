# Multiplayer integration: the Bevy shell, the crates and the order of work

Status: written 2026-10-16, revised 2026-10-21 by [ADR 0009](../../decisions/0009-multiplayer-netcode-revisions.md). Step 1 of
section 5 (`libs/blackbox-net-core`) is built; the rest is design. How the netcode of
[ADR 0006](../../decisions/0006-multiplayer-netcode.md) fits the existing program: the Bevy shell and `Scene` trait
([architecture.md](../../architecture.md), [ADR 0001](../../decisions/0001-bevy.md)), the `libs/` and `crates/` rules
([AGENTS.md](../../../AGENTS.md)), and the telemetry server ([multiplayer reuse](../telemetry/multiplayer-reuse.md)).

## 1. The threads

```
net thread (inside blackbox-net-core, feature netcode)
  UDP sockets, handshake, sealing, reliability layer, discovery
       ▲ │  bounded lock-free queues (blackbox-net-core hand-off)
       │ ▼
game thread (Bevy App)
  PreUpdate: input/ resolves devices into ActionState
  Update:    NetSession drains the inbox  ─►  Scene::update (NetWorldScene)  ─►  draw
```

- The game thread never waits on a socket, a lock or a file. It reads the newest snapshot from a triple buffer, takes
  events and messages from bounded rings, and pushes outgoing input and messages to a bounded queue. This is the same
  hand-off the telemetry design uses ([transport and privacy](../telemetry/transport-and-privacy.md) section 1), from the
  same code in `libs/blackbox-net-core`. Overflow drops the oldest entry and counts it; it never blocks.
- The net thread uses blocking sockets with a short timeout (no async runtime). It owns the sockets, the Noise state, the reliability
  layer and the packet queues; it holds no game state.
- **The listen host's own client** (ADR 0009 section 3) talks to the server thread through the in-memory link
  (`link::MemoryNet`), with the same connection and replication code as a remote client. It opens no socket and runs no handshake.
- The server loop (`nfsmw-server`, or the listen server's thread) is a plain loop with a fixed-step timer. It reads the
  inbox, steps the world, writes snapshots, and sleeps until the next tick with a spin on the last 0.5 ms.

## 2. Game-side pieces in `crates/nfsmw`

| Piece | Job |
|---|---|
| `NetSession` (a Bevy resource) | Owns the net-thread handles and queues; holds connection state, lobby state, clock estimate, and statistics. Created by the "LAN game" menu entry or by `--connect`/`--host` |
| `NetWorldScene` (a `Scene`) | The multiplayer world. `update` drains the inbox, advances the clock, runs the fixed-step accumulator for the owner's car (sample input, quantise, step, store history), applies reconciliation, and advances the display time of remote entities. `frame` builds the instance list from the owner's pose plus the interpolated remote poses, through the code the single-player world uses for AI cars (`CarPose`) |
| Lobby screens | FEng front-end screens (the same host layer as the menus) driven by `NetSession`; egui panels for the debug view |
| HUD hooks | The HUD reads the same `HudState` as single player; the multiplayer scene fills it from the server's pursuit, race and own-car blocks (heat, busted meter, position, lap) |
| Console | `net.host`, `net.connect <addr>`, `net.disconnect`, `net.stats`, `net.sim <loss> <rtt> <jitter>`, `net.kick <slot>`, `net.say`, `net.lead`. They go through the existing command table so scripts and telemetry control can share them |
| Settings | New keys in the layered settings (CLI > environment > config file > defaults): `net_name`, `net_port`, `net_snapshot_hz`, `net_interp_delay`, `net_show_stats`, `net_identity_file` (the identity key path, by `security.md`) |
| Metrics overlay | A `net` line at the advanced level: RTT, jitter, loss, snapshot age, corrections and snaps per minute, bytes in and out (MP-82) |

**Pause and menus.** The server never pauses. The pause key opens the overlay; the client keeps stepping its own car with
neutral input. The garage is a lobby feature in a session, not an overlay on a live world.

**Streaming.** Single player stops the whole sim until the required area is resident
([physics-timing.md](../../physics-timing.md)). A session cannot do that. The client keeps its car in a "loading" state with neutral
input (and sends `StreamingStall`) until its area is resident. The server streams collision for where players are, and
only the collision data (no meshes, textures or audio).

## 3. Crates

| Crate | Where | Depends on | Content |
|---|---|---|---|
| `blackbox-net-core` | `libs/` | feature `core` (default): `getrandom`, `crossbeam-queue`, `triple_buffer`. Feature `netcode`: `socket2`, `snow`, `postcard`, `serde` | `core`: ids, sim time, token, rate limiter, the hand-off types (shared with the telemetry server, [ADR 0007](../../decisions/0007-telemetry-server.md)). `netcode`: bit writer and reader, quantisers, varints; the `Transport` trait with UDP, in-memory and packet-simulator implementations (loss, jitter, reorder, duplicate); the reliability layer; the handshake, cookie and sealing of `security.md`; the clock-sync filter; the snapshot machinery (history ring, baseline select, delta encode and decode over a registered schema, spatial hash, priority accumulator); discovery |
| `nfsmw-net` | `crates/` | `blackbox-net-core` (`netcode`), `blackbox-vehicle`, `blackbox-pursuit` | The game's entity schemas and quantisation ranges, the input mapping to `InputState`, event enums, the settings and loadout structs, mode rules, version tables; prediction ring, reconciliation, interpolation buffers. Bevy-free, so it is unit-tested without a window |
| `nfsmw-sim` | `crates/` | `blackbox-*`, `nfsmw-data` | The headless world: cars, collision, roads, traffic, police, pursuit, races, props. Extracted from `scenes/world`. No audio, settings, renderer or Bevy types |
| `nfsmw-server` | `crates/` (binary) | `nfsmw-sim`, `nfsmw-net`, `blackbox-net-core` (`netcode`) | The server loop, TOML config, logging, the status and admin interface. No window |
| `nfsmw` | `crates/` | all of the above | `NetSession`, `NetWorldScene`, lobby screens, console commands |

Rules that hold throughout: `libs/` has no NFS:MW names, so the generic layer sees entity kinds as small integers and
fields as a schema the game registers; `crates/` depends on `libs/`, never the reverse. A build of the single-player game without
multiplayer does not compile the `netcode` feature (a cargo feature `multiplayer` on `nfsmw`, on by default once the milestone ships;
it can start off). CI runs `cargo deny` with `--all-features` so the `netcode` dependencies are checked (`deny.toml` sets
`all-features = false`). No file over 500 lines.

**As built (ADR 0009).** `blackbox-net-core` is standard library only and has no cargo features yet. The modules are `bits`,
`link`, `conn`, `replicate`, `clock`, `session`, `discovery` and `rng`. The `core`/`netcode` feature split is added when the
handshake brings `snow`, so that telemetry still pulls no crypto. The hand-off queues of the `core` feature are added with the net
thread.

If `blackbox-net-core` passes a few thousand lines, the snapshot machinery moves to a sibling `blackbox-net-replication`
without changing any public name used by the game.

## 4. Extracting `nfsmw-sim`

This is the largest piece of milestone 10 work and the gate for the dedicated server. What is known from reading the
code today:

- `CarSim` (`scenes/world/drive/sim/mod.rs`) and the pair code (`pair.rs`) import the binary crate's audio contact types
  (`crate::audio::CarContact`), the wheel option settings (`crate::settings::WheelOptions`), effects and event types
  (`crate::scenes::world::effects`, `events`), the prop world, the road spawn type and the space helpers.
- The AI manager, traffic, police and pursuit code under `scenes/world/ai/` calls into the same world types, and the
  pursuit rules are already pure (`libs/blackbox-pursuit`).
- The frame-rate trace test (`drive/sim/tests/frame_rate.rs`) is the guard: it must stay bit-identical through the move.

Proposed steps, each a small commit that keeps single player working:

1. Write the benchmarks of the ADR ("How we will know", item 1) against the current code. They are the baseline.
2. Replace the audio and effects outputs of the sim with plain output events (`SimEvent::Contact { .. }`) that a view layer
   turns into sound and effects. The sim returns events; it does not call audio.
3. Move settings the sim reads (wheel options, assists) into plain structs passed in by the caller.
4. Move `CarSim`, the pair code, the terrain and prop world, the AI world and the pursuit scene state into `nfsmw-sim`
   behind a `WorldSim` type with `step(tick, &Inputs) -> StepEvents`. The scene becomes a view: it owns a `WorldSim`, feeds
   it the player's input, and draws what it returns.
5. Replace shared random sources by seeded per-system streams (sync model, D4) and make entity iteration order stable (D3, D5).
6. Add a headless runner and a test that runs 10 minutes of scripted free roam twice and compares state hashes.

Fallback if step 4 proves too large: the first multiplayer release ships the **listen server only**, running the
`WorldSim` inside the host's process behind the scene, and the dedicated server follows when the extraction is done.

## 5. Order of work for milestone 10 (proposal)

| Step | Content | Gate |
|---|---|---|
| 0 | Sim benchmarks; determinism audit (D1 to D6); the sim extraction (section 4) | Single-player frame-rate trace unchanged |
| 1 | `blackbox-net-core`: bit packing, quantisers, links and the packet simulator, the connection layer (reliability, fragments), replication codec and per-client views, clock filter, session state machine, discovery; seeded round-trip, loss and fuzz tests; `net_bench` | **Built 2026-10-21** (ADR 0009) |
| 2 | Handshake and sealing (`snow`) around the built connection layer; fuzz targets for every pre-authentication parser; the hand-off queues. (The reliability suite moved to step 1; the `renet` spike is dropped by ADR 0009) | Fuzz gates; `cargo deny` with the crypto crates |
| 3 | Server loop and `nfsmw-server` with bots; join, lobby, chat | Two processes on one PC, no game window |
| 4 | Snapshot delta and relevancy; client interpolation; free roam with remote cars visible | Bandwidth within the ADR arithmetic |
| 5 | Prediction and reconciliation for the owner; clock sync; the bad-network suite | Targets of [features](features.md) section 4 |
| 6 | Pursuit co-op, race rules (when milestone 7 provides them), results | Server-authoritative pursuit and race tests |
| 7 | Spectators, reconnect, recording, console and overlay polish | P1 items |

Steps 1 and 2 can run in parallel with step 0.

## 6. Telemetry interplay

- The telemetry server (milestone 13) reads the server's world through the same hand-off pattern, on a dedicated server,
  and a client's view in a session. It stays read-only and local.
- The telemetry snapshot's `tick`, `schema` and entity ids ([multiplayer reuse](../telemetry/multiplayer-reuse.md)
  section 6) are the ones used here. The session id and entity id of the telemetry catalogue are the `session_seed`-keyed
  id and the 8-bit `entity_id` of the protocol.
- The recorder (telemetry F-33) writes either the server's snapshot stream or a client's received stream; the first line is
  a header with `schema`, `proto_minor`, `sim_build` and `data_id`, as the telemetry document already asks.

## 7. Open questions

1. Should the listen server's sim run on the game thread's schedule (simple, but the host's frame stalls delay the
   server) or on its own thread (isolated, but needs the extracted sim to be `Send`)? Proposal: its own thread.
2. Should `nfsmw-server` use `bevy_ecs` internally for entity storage, or plain `Vec`s? The AI code is built on its own
   types today; proposal: no Bevy in the server.
3. Does the dedicated server load the whole city's collision at start (memory) or by player area (complexity)? Proposal:
   by player area, with a minimum of the spawn areas, reusing the residency logic without the GPU side.

**Reverse-engineering note (2026-10-16).** The stock client's UDP path was read in Ghidra ([LAN server](../lan-server.md),
client path; trail in [provenance](../../provenance/lan-client.md)). It bears on [ADR 0006](../../decisions/0006-multiplayer-netcode.md)
section 15, which is not edited here. Reason 1 (25 against 29 bytes) does not hold in code: both sides use 25 bytes, and the
29-byte reading was a pointer error. Reason 2 holds for the lobby (an HTTP lookup against EA hosts that no longer run), but
the UDP layer is a generic peer channel with an opaque data message (`MIC`), so "not a game-sync protocol" is unproven: its
payload is not yet identified. Reason 4 holds: there is no authentication, only plain-text names and source addresses. The
decision not to keep the protocol stands, because no car state or input has been found in it and nothing from the client is
copied. Its keep-alive timing (500 ms of silence) is a behaviour fact, and this design does not depend on it.
