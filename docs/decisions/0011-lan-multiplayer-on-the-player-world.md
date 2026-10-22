# 0011 — LAN multiplayer on the N-player world

- **Status:** accepted (2026-10-22). Builds on [ADR 0006](0006-multiplayer-netcode.md), [ADR 0009](0009-multiplayer-netcode-revisions.md)
  and [ADR 0010](0010-coop-and-the-n-player-world.md). Where they disagree on the points below, this one wins.
- **Milestone:** 10 (LAN).

## Question

ADR 0006 planned LAN play as: extract a headless `nfsmw-sim` from the scene, then build `nfsmw-net`, `nfsmw-server` and a
`NetWorldScene`. The extraction was estimated in the gap review of 2026-10-22 at 250 to 350 files and 40 to 50 thousand lines
(`scenes/world` is 547 files and 74 thousand lines, `crate::audio` is imported in 49 files and `crate::settings` in 90). ADR 0010
then put a `Players` list and a per-player `Inputs` into the world, which is the seam the extraction was meant to create. What is
the shortest path to a complete LAN build that does not need a rewrite later?

## Options

### A. Do the full extraction first

`nfsmw-sim` owns the cars, collision, roads, traffic, police and races. It is the cleanest end state and the only way to a server
that never links the renderer. It is also weeks of mechanical work whose first visible result is nothing, and it cannot be
verified without compiling the whole game after every step.

### B. Run the server inside the game process, behind a trait

The authoritative world is the existing `WorldScene`, driven by a `WorldSim` trait that lives in a small new crate. Remote
players are `Players` entries whose `Inputs` come from the network, the same path split-screen uses for its second device. A
dedicated server is the same program started with `--dedicated`: it runs the scene with a renderer that draws nothing and no
window. The trait is the contract the later extraction implements; nothing above it changes.

### C. A separate simplified server world

A second, smaller world for the server. Two worlds that must agree is the failure ADR 0006 was written to avoid.

## Decision

Option B.

### 1. `nfsmw-sim` is the seam, not the extraction

`crates/nfsmw-sim` holds the `WorldSim` trait (`step(tick, &SimInputs) -> StepEvents`, `snapshot` into a caller-owned sink, the
join and leave calls), the plain input, event and entity-view types, a deterministic `ToyWorld` (kinematic cars on a plane) used
by every test and by bots, a headless fixed-step runner and a state hash. The real implementation, `WorldScene`, lives in
`crates/nfsmw` and is wired in by the game. Moving the world physics out of `crates/nfsmw` into this crate stays the plan of
[integration.md](../specs/multiplayer/integration.md) section 4; it is not part of this decision and not required for LAN.

### 2. Layers

| Layer | Where | Content |
|---|---|---|
| Transport, security, hand-off | `libs/blackbox-net-core` | Noise handshake and sealed packets, cookies, connect tokens, rate limiter, the net thread and bounded queues. Feature `netcode` pulls the crypto and socket crates; the default build stays standard library only |
| Game protocol | `crates/nfsmw-net` (Bevy-free) | Entity schema and quantisation, input mapping, the reliable message catalogue, version and data checks, loadouts, settings, lobby, roles, votes, chat, results, prediction, reconciliation, interpolation, recording |
| World seam | `crates/nfsmw-sim` | `WorldSim`, `ToyWorld`, runner, state hash |
| Server | `crates/nfsmw-server` | The server loop over a `WorldSim`, TOML config, logging, bots; a binary over `ToyWorld` for protocol tests and soak runs |
| Game | `crates/nfsmw` | `WorldSim for WorldScene`, `NetSession`, the client mode of the world, lobby screens, console commands, the net overlay line, `--host`, `--connect`, `--dedicated` |

### 3. Seats

`Seat::Remote { peer }` already exists in `blackbox-net-core`. A remote player joins the world through `players::spawn` exactly as
the co-op guest does and steps with the input the server holds for that peer. Co-op career rules (shared pursuit, per-player bust,
shared garage, catch-up) apply to remote players with no new code path: a LAN co-op career is the host's career with remote guests.
Only the host has a save; a guest sees the host's garage and wallet and writes nothing to disk.

### 4. The client

A client runs the same `WorldScene` in a client mode: no traffic, police or race rules of its own (the server owns them), its own
car simulated locally for prediction, and every other entity (remote players, traffic, police, props that move) drawn from
interpolated snapshots through the poses the scene already uses for AI cars. The server's pursuit, race and own-car blocks feed
the HUD. Reconciliation follows [sync-model.md](../specs/multiplayer/sync-model.md).

### 5. What this ADR does not build

Stated here so nothing is "missing" by accident:

- The physical extraction of the world into `nfsmw-sim` (section 1).
- Milestone 11 items: server list, invite codes, region display, accounts, persistent stats (MP-28, 29, 74, 86).
- Voice chat (MP-43), host migration (MP-52), handicaps (MP-36), last-standing by damage (MP-10), the in-game replay viewer
  (MP-73, blocked on the telemetry recorder), mod compatibility beyond an empty list (MP-63, blocked on the mod API) and the
  telemetry bridge (MP-85, milestone 13).
- Drag races (MP-04: the dragster is not modelled in single player) and human cops (MP-08: open question 2 of
  [features.md](../specs/multiplayer/features.md)).

## Consequences

- LAN play works on the listen-server path first and the dedicated flag second; both run the same scene, so there is one world to
  keep correct.
- The dedicated server links the renderer crates even though it draws nothing. Accepted until the extraction.
- `WorldScene` gets a small number of hooks (remote input, snapshot write, client mode); none of them changes single-player
  behaviour, which stays guarded by the existing world and frame-rate traces.
- Every protocol rule is in a Bevy-free crate with tests that run without a window, a GPU or a game install.
