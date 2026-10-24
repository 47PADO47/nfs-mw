# Multiplayer integration: the Bevy shell, the crates and the order of work

Status: written 2026-10-16, revised 2026-10-21 by [ADR 0009](../../decisions/0009-multiplayer-netcode-revisions.md). Steps 1 and 2
of section 5 (`libs/blackbox-net-core`, then its handshake, sealing, rate limits and hand-off queues) are built, and so is the net
thread of section 1 (2026-10-22, "as built" there); the server side of steps 3 and 4 is built in `nfsmw-server`
([server](server.md)); the rest is design. How the netcode of
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

**As built (2026-10-22).** The net thread is module `endpoint` of `blackbox-net-core` (feature `netcode`); the code and its module
docs are the reference. It has no game names: messages are opaque bytes, and the game picks the ports, prologue and limits.

| Piece | Built as |
|---|---|
| Server | `ServerEndpoint::bind(game_addr, Some(discovery_addr), config)`, or `spawn(transport, discovery, config)` over any `Transport`. One thread owns the game socket, the discovery socket, the `HandshakeServer` (cookie, Noise, pre-authentication gate), one sealed `Connection` per peer and the discovery responder |
| Client | `ClientEndpoint::connect(server_addr, config)` (direct connect by address) or `spawn(transport, ..)`. One thread, one handshake, one connection; the thread ends with the connection |
| Host's own client | `ServerEndpoint::connect_local(player_key, config, latest_frame_type)` returns a `ClientEndpoint` on a private `MemoryNet`: a `conn::Plain` connection, no socket, no handshake. The server reports it as `ServerEvent::Connected { local: true, .. }` with the key the caller passes; every other event and call is the same as for a remote peer |
| Events to the game | `ServerEvent`: `Connected { peer, key, addr, local }`, `Disconnected { peer, reason }`, `Reliable { peer, channel, bytes }`, `Unreliable { peer, frame_type, bytes }`, `Delivered { peer, tag }`, `Lost { peer, tag }`, `Stats`. `ClientEvent` has the same without the peer |
| Commands from the game | `send_reliable`, `send_unreliable` (with an optional `tag`, reported back as `Delivered` or `Lost` when the packet that carried it is acknowledged or written off: the hook for delta baselines), `broadcast_reliable`, `broadcast_unreliable`, `disconnect(peer, reason)`, `set_budget(peer, PeerLimits)`, `set_discovery_reply`, `shutdown` (also on drop) |
| Queues | `handoff::ring` for events (4,096), sends (4,096) and control commands (256: disconnect, budgets, discovery reply, local attach), so sends cannot push a kick out. Full queues drop the oldest entry and count it (`dropped_events`, `dropped_commands`); shutdown is a flag and a join, never a queued command |
| Snapshot path | `ClientEndpointConfig::latest_frame_type`: frames of that type go to a `handoff::latest` cell (`take_latest`, `latest`) instead of the event ring. Only for frames the game may skip: a delta whose baseline follows packet acknowledgements must be decoded, so with transport-acknowledged baselines the game leaves it unset and reads every frame from the ring |
| Socket | `std::net::UdpSocket` made through `socket2` (256 KiB buffers asked for, address reuse on the discovery port): blocking with a 1 ms read timeout when the last pass had nothing to do, non-blocking while busy. No async runtime |
| Timers | keep-alive after 2 s of silence, idle timeout 10 s (`EndpointConfig`); a new server-side connection sends an acknowledgement-only packet every 100 ms until it hears from the client, so a lost key confirmation is repaired inside the client's retry window; DISCONNECT goes out 3 times |
| Fairness under flood | at most 256 datagrams per pass before commands, timers and sends get a turn |

One pass of the server thread: control commands, sends, up to 256 game-port datagrams (classified by the first byte, handshake
packets to the handshake server, data packets routed by connection id, dropped if from another address or over the peer's
budget, then opened), the in-memory link, the discovery port, then handshake expiry, budget ticks, idle timeouts, stats, and
the sends. **Test seam:** `Transport` is implemented by `UdpTransport`, `link::MemoryLink`, `link::UdpLink` and `link::LossyLink`,
so the tests run the same endpoints over loopback sockets and over the seeded lossy simulator. `examples/lan_echo.rs` is a manual
soak (one server, N clients, one process).

**Not built here:** the server loop with its fixed-step timer (that is `nfsmw-server` or the listen server's thread, which drains
`ServerEvent`s); listing the host's interfaces for the discovery source filter (the caller passes the
subnets; the default is the private ranges). Broadcast itself is not exercised by the tests, which send discovery to `127.0.0.1`.

## 2. Game-side pieces in `crates/nfsmw`

| Piece | Job |
|---|---|
| `NetSession` (a Bevy resource) | Owns the net-thread handles and queues; holds connection state, lobby state, clock estimate, and statistics. Created by the "LAN game" menu entry or by `--connect`/`--host` |
| `NetWorldScene` (a `Scene`) | The multiplayer world. `update` drains the inbox, advances the clock, runs the fixed-step accumulator for the owner's car (sample input, quantise, step, store history), applies reconciliation, and advances the display time of remote entities. `frame` builds the instance list from the owner's pose plus the interpolated remote poses, through the code the single-player world uses for AI cars (`CarPose`). Planned: it reuses the `Players` list (section 4, step 3a), one entry per seat |
| Lobby screens | Pages drawn by the themes' `LanMenu` slot (the toolkit the menus use), driven by `NetSession` through `NetFrontend` |
| HUD hooks | The HUD reads the same `HudState` as single player; the multiplayer scene fills it from the server's pursuit, race and own-car blocks (heat, busted meter, position, lap) |
| Console | `net.host`, `net.connect <addr>`, `net.disconnect`, `net.stats`, `net.sim <loss> <rtt> <jitter>`, `net.kick <slot>`, `net.say`, `net.lead`. They go through the existing command table so scripts and telemetry control can share them |
| Settings | New keys in the layered settings (CLI > environment > config file > defaults): `net_name`, `net_port`, `net_snapshot_hz`, `net_interp_delay`, `net_show_stats`, `net_identity_file` (the identity key path, by `security.md`) |
| Metrics overlay | A `net` line at the advanced level: RTT, jitter, loss, snapshot age, corrections and snaps per minute, bytes in and out (MP-82) |

**As built (2026-10-22): the session half of `NetSession`.** Everything in `NetSession` that is not Bevy is
`nfsmw_net::client_session::ClientSession` ([client session](client-session.md)), generic over a `ClientLink` (implemented
by `ClientEndpoint`) and a `PredictedCar`. It owns the connection state (admission, password, resume token and reconnect),
the lobby mirror (`LobbyView`), the clock (`NetClock` fed by PONGs), the snapshot path (parts, views, interpolation,
prediction and reconciliation, the global block as `GlobalView`), the event queue and the overlay statistics. The Bevy
resource and `NetWorldScene` are not built here (the lobby screens, the console commands and the settings keys are: "the front
end" below); the resource
will hold a `ClientSession<ClientEndpoint, GameCar>` and call `update`, `due_ticks`/`submit_input`, `own_pose`,
`remote_entities` and `take_events` once a frame. Scripted bots over the same session are
`nfsmw_server::bot` (`Bot`, `BotFleet`, `ThreadedFleet`).

**As built (2026-10-22): the server side of the world.** `WorldSimHost` (`scenes/world/net/`) implements `WorldSim` over a
`WorldScene` and `NullBackend`, the renderer that draws nothing (`app/null_backend.rs`, formerly the tests' `test_backend`,
now built in every configuration). One `step` hands each seat's `InputState` to its car (`Drive::set_remote_input`), runs
exactly one tick (`FixedClock::set_one_per_update`) under `SessionRules::session(seed)`, and reports contacts, pursuit,
race and moved events from what the world keeps. Remote players are co-op guests on their own `PlayerId`; the host's own
player is `PlayerId(0)`, the lead. The full description and the gaps: [world host](world-host.md).

**As built (2026-10-22): the game's side** ([game wiring](game-wiring.md), feature `multiplayer`, on by default).

| Piece | Built as |
|---|---|
| `NetSession` | `net/session.rs`, modes `Idle`, `Host`, `Join`, `Dedicated`. Not a resource of its own: it is the front end's `NetFrontend` in the `LanBackend` resource (`NonSend`, the client shares the car with the world through `Rc`s); `NetPlugin` adds it and two systems in `FrameSet::Commands` (the session's update, then handing the client to the world scene) |
| Listen server | A thread of its own (open question 1 of section 7, as proposed): it opens the `WorldSimHost` itself (not `Send`), binds the LAN port with discovery, runs `Server<WorldSimHost>` and attaches the host's client with `connect_local`; only the client endpoint crosses back |
| Dedicated server | `nfsmw --dedicated`: the same loop on the main thread, `NullBackend`, no window, audio or GPU, until Ctrl-C |
| `NetWorldScene` | Not a scene of its own: the client mode of `WorldScene` (`scenes/world/net_client/`). One switch (`frame_systems.rs`) turns off the world's own traffic, police, pursuit, races, career and catch-up; the owner's car is the real `Drive`, predicted through `GameCar` with the world lent for the session calls; every other car is drawn from the interpolated snapshots with the AI cars' rigs and `CarPose`s |
| HUD hooks | The global block (heat, busted and cooldown meters, chase state, race place and lap) over the predicted car's dials; the server fills it through `WorldControl::fill_global` |
| Command line | `--host`, `--connect ADDR`, `--dedicated`, `--discover`, `--port`, `--name`, `--server-config`, `--seconds` |

**As built (2026-10-22): the front end.** The lobby screens, the console commands, the settings keys and the overlay line are
built against a seam, the `NetFrontend` trait (`frontend/lan/backend.rs`), which the game's `NetSession` implements and keeps in
the non-send resource `LanBackend`; the full description is [frontend](frontend.md). The main menu's LAN icon opens the LAN menu
(host, join with discovery, direct connect and the password prompt, the lobby with ready, car, role, chat, quick lines, votes,
the player menu and the host's settings and start, the results). The original has no LAN screens in its packages, so the
Original and Remastered themes draw them with the game's UI toolkit (the `LanMenu` and `LanHud` slots, no egui); the car select
is the custom race's `Car_Select.fng`. The
session HUD draws the chat and notice feed, the vote, the countdown and the spectator panel. The console has `net.host`,
`net.connect`, `net.disconnect`, `net.stats`, `net.sim`, `net.kick`, `net.say`, `net.lead`, `net.scan`, `net.servers` and
`net.status`; the settings are `net_name`, `net_port`, `net_snapshot_hz`, `net_interp_delay`, `net_show_stats` and
`net_identity_file` on every layer; the metrics overlay shows the `net` line at the advanced level (or always with
`net_show_stats`). Tested on a mock `NetFrontend` and in offscreen screenshot runs on a pretend session (`NFSMW_LAN_DEMO=1`).

**Pause and menus.** The server never pauses. The pause key opens the overlay; the client keeps stepping its own car with
neutral input (built: the pause wrapper keeps updating a scene that `runs_while_paused`). The garage is a lobby feature in a session, not an overlay on a live world.

**Streaming.** Single player stops the whole sim until the required area is resident
([physics-timing.md](../../physics-timing.md)). A session cannot do that. The client keeps its car in a "loading" state with neutral
input (and sends `StreamingStall`) until its area is resident (built: `net_client/frame.rs` holds the car and sends it). The server streams collision for where players are, and
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
multiplayer does not compile the server (a cargo feature `multiplayer` on `nfsmw`; built 2026-10-22, on by default:
[game wiring](game-wiring.md) section 1; `nfsmw-net`, and with it `netcode`, is in every build because the LAN screens use
it). CI runs `cargo deny` with `--all-features` so the `netcode` dependencies are checked (`deny.toml` sets
`all-features = false`). No file over 500 lines.

**As built (2026-10-22).** The split differs from the table above: the default build of `blackbox-net-core` is standard library
only and has every module that needs no crate (`bits`, `link`, `conn`, `replicate`, `clock`, `session`, `discovery`, `rng` and
`ratelimit`). Feature `handoff` adds the `handoff` module (`crossbeam-queue`, `triple_buffer`), which telemetry can take without
any crypto. Feature `netcode` (implies `handoff`) adds the `secure` module: `snow`, `chacha20poly1305`, `blake2`, `sha2`,
`subtle`, `zeroize`, `getrandom` and `spake2`. `socket2` (buffers and address reuse) is pulled with `netcode`; `postcard` and `serde` are not needed yet and are not pulled.
`deny.toml` now sets `all-features = true`, so `cargo deny check` covers the `netcode` crates without a flag (it passed on
2026-10-22). What `secure` contains and what is not built yet: [protocol-handshake](protocol-handshake.md)
and [security](security.md) section 8. Feature `netcode` also holds `endpoint`, the net thread of section 1; it makes its sockets with
`socket2` for the 256 KiB buffers of [operations security](security-ops.md) section 2.1 and for address reuse on the discovery port;
`std::net::UdpSocket` does the reading and sending.

If `blackbox-net-core` passes a few thousand lines, the snapshot machinery moves to a sibling `blackbox-net-replication`
without changing any public name used by the game.

## 4. Extracting `nfsmw-sim`

**Status.** [ADR 0011](../../decisions/0011-lan-multiplayer-on-the-player-world.md) built the seam and left the physical
extraction below for later; LAN does not wait for it. `crates/nfsmw-sim` holds what exists today:

- the `WorldSim` trait (`step`, `entities` in ascending id order, `own_state`, `state_hash`, join and leave) with the plain
  `SimInputs`, `EntityView`, `OwnState`, `SimEvent` and `PlayerJoin` types;
- `hash::StateHasher`, a hand-written FNV-1a with a SplitMix64 finaliser: little-endian integers, floats by bit pattern with
  `-0.0` and NaN normalised, length-prefixed slices. It means the same on every platform;
- `toy::ToyWorld`, a deterministic kinematic world (60 Hz, own seeded SplitMix64 stream, no clock, only `+ - * /` and `sqrt`):
  up to 16 player cars on the lowest free entity id (traffic ids start at 16), wandering traffic, police that chase the nearest
  player and raise pursuit, evade and bust events, car-to-car `Contact`, `Moved` on reset or teleport, and an optional circuit
  with `LapCompleted` and `RaceFinished` in finishing order. Hostile input (NaN, infinities, huge values) is made safe;
- `runner::Runner`, the headless fixed-step runner with `InputScript` (a closure, `PerPlayer`, or the reproducible
  `WanderScript`), an event log, and `compare_runs`, which drives two worlds with one script and compares events every tick and
  hashes every second.

Step 6 below is therefore done for the toy world: its tests run ten simulated minutes of 16 scripted players twice and compare
the hash on every tick.

**`WorldSim` for the real world (2026-10-22).** `WorldSimHost` in `crates/nfsmw/src/scenes/world/net/` implements the trait
over `WorldScene` without the extraction, as ADR 0011 decided ([world host](world-host.md)). Its installed test drives two
hosts from one seed with the same inputs for 40 simulated seconds (two players, traffic and cops) and compares the events
every tick and the hash and entities every second. `nfsmw-sim`'s `Runner` and `compare_runs` take any `WorldSim`, so they
can drive it too; the host is not `Send` (the scene holds `Rc`s), so it runs on the thread that opened it.

The extraction itself is the largest piece of milestone 10 work and the gate for the dedicated server. What is known from
reading the code today:

- `CarSim` (`scenes/world/drive/sim/mod.rs`) and the pair code (`pair.rs`) import the binary crate's audio contact types
  (`crate::audio::CarContact`), the wheel option settings (`crate::settings::WheelOptions`), effects and event types
  (`crate::scenes::world::effects`, `events`), the prop world, the road spawn type and the space helpers.
- The AI manager, traffic, police and pursuit code under `scenes/world/ai/` calls into the same world types, and the
  pursuit rules are already pure (`libs/blackbox-pursuit`).
- The frame-rate trace test (`drive/sim/tests/frame_rate.rs`) is the guard: it must stay bit-identical through the move.

Proposed steps, each a small commit that keeps single player working:

1. Write the benchmarks of the ADR ("How we will know", item 1) against the current code. They are the baseline.
   **Done 2026-10-22** for the world step: `scenes/world/step_bench_tests.rs` at 1, 8 and 16 players with traffic; the
   numbers are in [bandwidth](bandwidth.md) section 5.
2. Replace the audio and effects outputs of the sim with plain output events (`SimEvent::Contact { .. }`) that a view layer
   turns into sound and effects. The sim returns events; it does not call audio.
3. Move settings the sim reads (wheel options, assists) into plain structs passed in by the caller.

3a. Players and inputs: the scene keeps a `Players` list keyed by `PlayerId` (`blackbox-net-core` `session::Party`) and per-player `Inputs`. Built for split-screen co-op ([ADR 0010](../../decisions/0010-coop-and-the-n-player-world.md), `scenes/world/players/`). `WorldSim::step(tick, &Inputs)` keeps the same keying.

4. Move `CarSim`, the pair code, the terrain and prop world, the AI world and the pursuit scene state into `nfsmw-sim`
   behind a `WorldSim` type with `step(tick, &Inputs) -> StepEvents`. The scene becomes a view: it owns a `WorldSim`, feeds
   it the player's input, and draws what it returns.
5. Replace shared random sources by seeded per-system streams (sync model, D4) and make entity iteration order stable (D3, D5).
   **Done 2026-10-22** in the scene: per-system streams, per-world entity ids with id-order stepping, sorted tile removal and
   lockstep tile uploads ([determinism](determinism.md) has what is left).
6. Add a headless runner and a test that runs 10 minutes of scripted free roam twice and compares state hashes.
   **Done 2026-10-22 in the scene**, without the extraction: `scenes/world/determinism_tests.rs` runs 150 simulated seconds
   (not 10 minutes: both runs fit in about 21 s of test time) of a scripted drive with traffic and cops twice under a
   session's rules and compares `WorldScene::state_hash` every second ([determinism](determinism.md), step 5).

Fallback if step 4 proves too large: the first multiplayer release ships the **listen server only**, running the
`WorldSim` inside the host's process behind the scene, and the dedicated server follows when the extraction is done.

## 5. Order of work for milestone 10 (proposal)

| Step | Content | Gate |
|---|---|---|
| 0 | Sim benchmarks; determinism audit (D1 to D6); the sim extraction (section 4) | Single-player frame-rate trace unchanged |
| 1 | `blackbox-net-core`: bit packing, quantisers, links and the packet simulator, the connection layer (reliability, fragments), replication codec and per-client views, clock filter, session state machine, discovery; seeded round-trip, loss and fuzz tests; `net_bench` | **Built 2026-10-21** (ADR 0009) |
| 2 | Handshake and sealing (`snow`) around the built connection layer; fuzz targets for every pre-authentication parser; the hand-off queues. (The reliability suite moved to step 1; the `renet` spike is dropped by ADR 0009) | **Built 2026-10-22**: seeded garbage tests in `cargo test`, `cargo-fuzz` targets in `libs/blackbox-net-core/fuzz`; `cargo deny` passes with the crypto crates |
| 3 | Server loop and `nfsmw-server` with bots; join, lobby, chat | Two processes on one PC, no game window. **Server side built 2026-10-22** ([server](server.md)): `Server<W>` over any `WorldSim`, the TOML config, logging, the toy-world binary with `--bots N`; tested with 16 raw clients and with the bots over loopback UDP. Two processes with a game window wait for the game's `NetSession`; **built 2026-10-22** ([game wiring](game-wiring.md)), tested in one process without a window |
| 4 | Snapshot delta and relevancy; client interpolation; free roam with remote cars visible | Bandwidth within the ADR arithmetic. **Server side built 2026-10-22**: deltas against the acknowledged view, relevancy by spatial hash, the per-client budget; 3 to 12 KB/s per client with 8 players on the toy world ([server](server.md) section 7). The remote cars in the game: **built 2026-10-22**, the client mode of the world ([game wiring](game-wiring.md) section 4) |
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
