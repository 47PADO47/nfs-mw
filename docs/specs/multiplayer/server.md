# The server loop: `nfsmw-server` as built

Status: built 2026-10-22 (integration.md section 5, steps 3 and 4, server side). The code and its module docs in
`crates/nfsmw-server` are the reference; this file is the overview, the config keys and how to run it. Design:
[ADR 0011](../../decisions/0011-lan-multiplayer-on-the-player-world.md) (the world seam), [ADR 0009](../../decisions/0009-multiplayer-netcode-revisions.md)
(delta baselines by packet acknowledgement, 16-bit ids), [integration](integration.md) (threads and crates).

## 1. What it is

`Server<W: WorldControl>` is the authoritative server of one session over any world that implements
`nfsmw_sim::WorldSim` (plus `WorldControl`, a small trait with defaults for what the server asks beyond `WorldSim`: a
reset to the road and the parked ghost car of a dropped player). It owns:

| Piece | From |
|---|---|
| The net thread's handle (`ServerEndpoint`) | `blackbox-net-core::endpoint` (feature `netcode`) |
| The lobby and phase machine, join gate, resume book, presence, bans, roles, votes, chat gate, reset rule | `nfsmw-net` (`lobby`, `join`, `drop`, `roles`, `vote`, `chat`, `fairness`) |
| The mode rules, the tally, the roster, the result record | `nfsmw-net::modes`, `nfsmw-net::results` |
| The per-client replication state, the 128-tick history, the encode cache, the spatial hash | `blackbox-net-core::replicate` |
| The snapshot, ping and pong frames | `nfsmw-net::frames` |
| The replay recorder | `nfsmw-net::record` |
| The world | the caller (`ToyWorld` in the binary and the tests; the game's `WorldScene` later) |

The game runs the same type for a listen server (its own client attached with `Server::connect_local`) and for
`--dedicated` ([game wiring](game-wiring.md)). Nothing in the server knows the renderer, audio or settings of the game.

## 2. One tick

`Server::tick_once(now_ms)` is one 60 Hz step with the time given by the caller; `Server::run(seconds, hook)` calls it from
a drift-free timer (tick `n` is due at `start + n / 60 s`, computed from the start each time; it sleeps to 1.5 ms before
the deadline and spins the rest; more than 6 ticks behind, it skips ahead and counts the skipped ticks). A `TickHook`
runs before each tick (the bots of `--bots N`).

1. **Inbox.** Every `ServerEvent`. `Connected` makes a connection record. A reliable message passes the message bucket
   (20/s, burst 40), is decoded by `nfsmw_net::message::decode` and routed by the connection's stage: `Auth` then `Join`
   (the admission of `JoinGate::admit`, then `Lobby::add` or a resume), then the member commands (`SetReady`,
   `SelectLoadout`, `SetRole`, `UpdateSettings`, `StartCountdown` as a start request, `CancelCountdown`, `LoadComplete`,
   `HostCommand`, `Vote`, `ChatSend`, `QuickMessage`, `ResetRequest`, `ResultRequest`, `BaselineLost`, `StreamingStall`).
   `INPUT` frames pass the input bucket (66/s, burst 20) into the player's `InputWindow`; a `PING` is answered with a `PONG`
   in the same tick; `Delivered` with a snapshot tick makes that view the client's delta baseline; `Stats` feeds the lobby
   rows' ping.
2. **Timers.** Connections whose close is due get their DISCONNECT (a refused or kicked client is closed 250 ms after its
   last message, so the message gets out); a seated player silent for 10 s is timed out; a resume window that ended
   releases its slot.
3. **Lobby.** `Lobby::update` (start countdown, load barrier, race countdown), `VoteBox::update`, the results timer. Every
   `Action` is carried out: sends and broadcasts, the `PlayerJoined`, `PlayerLeft`, `RoleChanged` and `PhaseChange` events,
   kicks, spawns and despawns. Loading spawns every racer's car (`PlayerId(slot)`, the contract with the client) and reports
   the server loaded at once; the race start builds the mode's rules (`AnyMode::build`) and sends `RaceStart`; the end of a
   round sends `RaceResults` from `build_record` and starts the results timer, after which the lobby comes back.
4. **World.** Each driver's input of the tick from its window (received, held 100 ms, then a gentle brake), neutral before
   the race start tick; `WorldSim::step`; the new entity views; contacts and moves mark the own block's correction cause;
   the world's events and one car sample per player go to the mode rules and the tally; their events go out
   (`nfsmw_net::modes::messages`) and act (`Spectate` makes a spectator, `Over` finishes the round).
5. **Snapshots.** The tick's frame of every entity is quantised into the history, the encode cache is reset and the spatial
   hash filled. Each seated client is due on its rate (20 Hz by default, a spectator at most 10 Hz), staggered by slot. Its
   candidates are every player car plus everything within `relevancy_radius` of its car (a spectator: everything), with a
   priority weight of 1 near and `100 m / distance` beyond. `ClientState::write_snapshot` writes the body against the newest
   delivered view under the per-snapshot share of `snapshot_budget`; `nfsmw_net::frames::write_snapshot` adds the header,
   the own block (the world's `own_state`, the correction cause and the input report of the last 32 ticks) and the global
   block (phase and time of day; the pursuit and race fields are zero). The frame is sent unreliable with the tick as its tag.
   A snapshot always fits one datagram (1,166 bytes), so parts are never needed at these budgets.
6. **Housekeeping.** The late-join session data (`BulkSender`, 2 KiB per tick), the discovery reply once a second, the
   status line every `status_seconds`, the replay.

## 3. Sessions

| Feature | As built |
|---|---|
| Join (MP-21, MP-23, MP-24) | `JoinAccepted` carries the slot, the car's entity id (or `0xFFFF` before it has one), the session seed, the tick, the snapshot rate of the role, the settings, the world bounds, a 32-byte resume token (from the OS CSPRNG), the schema id. A refusal is `JoinRejected` with the server's versions, then a close |
| Password | Not supported: the PAKE exchange is not wired, so a configured password is a config error |
| Late join (MP-53) | A joiner of a running session gets `LobbyState` and the `SessionData` blob on the bulk channel, then snapshots from a full one. A joiner of a running race is a spectator who races next time; of a running free roam, a racer whose car spawns with a `Respawn` (`JoinInProgress`) |
| Drops (MP-50) | A quit releases the slot (`PlayerLeft`, `EntityRemoved`); a timeout of a slot with a resume token holds it 30 s, flags the row away and parks the car (neutral input; the ghost needs `WorldControl::set_ghost`, which the toy world does not have) |
| Resume (MP-51) | `Auth.resume` with the token, from the same identity key, takes the slot back, gets a new token, `LobbyState` and a full snapshot. A token from another key is a normal join |
| Kicks and votes (MP-26, MP-27) | Lobby kicks and passed kick votes send `Kick` and close; votes are broadcast as `VoteState` and notices |
| Spectators (MP-41) | 10 Hz snapshots of every entity, no own block, no input taken |
| Chat (MP-40, MP-42) | `ChatGate` relays to all, a team or a whisper; a muted member is dropped silently. Quick messages are checked but not relayed (no server-to-client quick message exists) |
| Pause (MP-54) | The server never pauses the world. "Everyone wait" (`Lobby::hold`) is not reachable yet: no message asks for it |
| Results (MP-70) | `RaceResults` at the end of each round, `ResultRequest` resends it, and the record is written as `<replay>-round-<n>.nmrr` |
| Replay (MP-71) | With `replay_dir`, `session-<unix seconds>.nfsrpl`: the whole world at the snapshot rate with a key snapshot each second, every broadcast event, the index and trailer at shutdown |

## 4. Hostile clients

As security-gameplay.md sections 1.3 and 5: a message or frame that does not decode counts toward the limit of 16 per
10 s (`InvalidMessages`; the 17th closes the connection) and adds to the suspicion score. Weights: input flood 1 per 10
dropped frames, input more than 12 ticks early 1, message flood 2, malformed 5, out of order (a lobby command before
`Join`, a second `Join`, a server-to-client message) 5, a spoofed host command 10. The score decays 1 point per 10 s; 10
logs a warning, 30 forgets the client's views (the next snapshot is full), 60 kicks with `Violations`. A seated player's
inbound budget is set to 8 KB/s and 6 datagrams per 16 ms on the endpoint, which drops the excess before it is opened.

## 5. Config

One flat TOML table (`crates/nfsmw-server/server.toml` is a commented sample). Every key is optional. An unknown key, a
wrong type, a value out of range or session settings that do not hold together is an error that names the key.

| Key | Default | Meaning |
|---|---|---|
| `name` | `nfsmw-server` | Server name, 1 to 32 characters |
| `mode` | `free_roam` | `free_roam`, `circuit`, `sprint`, `speedtrap`, `knockout`, `pursuit_coop`, `tag` (drag and cops vs racers are refused by the rules) |
| `track`, `laps` | 0, 0 | Track id; laps 1 to 10 for circuit and knockout |
| `traffic_density`, `cop_level_cap`, `pursuits` | 50, 5, true | Percent; 0 to 5 |
| `max_players`, `max_spectators` | 16, 8 | Racers 2 to 16; spectators 0 to 8 |
| `public` | false | false: LAN only |
| `join_in_progress` | `free_roam` | `closed`, `free_roam`, `between_races` |
| `collision`, `catch_up`, `nitrous`, `damage`, `allowed_cars`, `password_required`, `sim_build_match` | `full`, false, true, `visual`, `any`, false, false | As `nfsmw_net::settings` |
| `bind`, `port` | `0.0.0.0`, 9750 | The game port; 0 lets the system pick |
| `discovery` | true | Answer LAN discovery on `port + 1` |
| `password` | empty | Must stay empty (no PAKE yet) |
| `snapshot_hz` | 20 | 1 to 60 per second; spectators get at most 10 |
| `snapshot_budget` | 16384 | Snapshot bytes per second to one client, 1024 to 32768 |
| `relevancy_radius` | 600 | Metres, 10 to 20,000 |
| `auto_start`, `countdown_seconds`, `results_seconds` | true, 5, 10 | The lobby starts by itself when everyone is ready |
| `replay_dir`, `log_file` | empty, empty | Empty for none |
| `log_level`, `status_seconds` | `info`, 10 | Status line period; 0 for none |

## 6. Running it

```sh
cargo run --release -p nfsmw-server -- --config crates/nfsmw-server/server.toml
cargo run --release -p nfsmw-server -- --port 0 --bots 8 --toy-traffic 100 --seconds 600   # a soak run
cargo test -p nfsmw-server --all-features                                                 # unit and end-to-end tests
```

Flags: `--config <file>`, `--port <n>`, `--toy-traffic <n>`, `--toy-police <n>`, `--bots <n>`, `--seconds <n>`,
`--seed <n>`. The binary serves `ToyWorld` (a circuit for the lap modes) and has no window, audio or game install. It
logs to stderr and the `log_file`; a status line reports the tick, the phase, the players, the tick cost, the snapshot
bytes per second per client (mean and max) and the strikes.

## 7. Measured (toy world, 2026-10-22)

The end-to-end tests (`crates/nfsmw-server/tests/server`) run a fake server clock with raw clients built from the
protocol. Snapshot bytes per client per second at 20 Hz, full throttle in free roam:

| Session | Mean | Max |
|---|---|---|
| 8 players, 8 traffic, 2 police | 3.0 KB/s | 3.0 KB/s |
| 8 players, 120 traffic, 8 police | 12.3 KB/s | 12.4 KB/s |
| 16 players, 8 traffic, 2 police | | 4.0 KB/s |

All under the 32 KB/s cap and the 16 KB/s typical figure of [features](features.md) section 4. With three clients on the
lossy simulator at 2 % loss and the real handshake, every snapshot decodes (deltas are only based on acknowledged views).
The server's mean tick with three bots is about 0.12 ms.

## 8. Not built

- The room password (PAKE) and the connect tokens of online play.
- Snapshot parts (a snapshot always fits one datagram at the configured budgets).
- The global block's pursuit and race fields come from `WorldControl::fill_global` (default: zero); the game's world fills
  them ([game wiring](game-wiring.md) section 3), the toy world does not. The own block's correction cause `Freeze`.
- Ghost cars and server-side resets need `WorldControl` from the world; the toy world resets but has no ghosts, the game's
  world does both (`scenes/world/net/control.rs`).
- The telemetry bridge of MP-84 and an admin console.
- Bans do not expire from memory (they stop applying at their end time), and the server's own identity key is new on
  every start (no `identity_file` key yet).
