# Multiplayer (milestones 10 and 11)

Design for LAN multiplayer (milestone 10) that online multiplayer (milestone 11) reuses without a rewrite: one host-authoritative
server program, the same on a LAN and on the internet, over our own UDP layer. Designed now, not built. Written 2026-10-16.

The decision record is [ADR 0006](../../decisions/0006-multiplayer-netcode.md). The files here hold the requirements, the wire format,
the algorithms and the security design it refers to.

Read in this order:

1. [features.md](features.md): scope, the priority legend (P0, P1, P2), the feature catalogue (modes, lobby, discovery, chat,
   spectators, reconnect, replays, checks), the latency, loss, player-count and bandwidth targets, and the open questions.
2. [ADR 0006](../../decisions/0006-multiplayer-netcode.md): topology, tick rate, what is simulated where, prediction, snapshots,
   transport, serialization, clocks, determinism, the Bevy mapping, the crate split, and what is kept of the original LAN protocol
   (nothing).
3. [sync-model.md](sync-model.md): the owner's prediction and reconciliation, interpolation and extrapolation of remote cars, car
   collisions, clock synchronisation, the determinism list, session lifecycle, failure behaviour and tuning values.
4. [protocol.md](protocol.md): discovery, the handshake packets, the sealed packet, frame types, the input, snapshot and ping
   layouts, the reliable message catalogue, versioning and limits.
5. [bandwidth.md](bandwidth.md): record sizes, per-packet overhead, the arithmetic for LAN and online cases, server totals.
6. [transport-options.md](transport-options.md): crates evaluated, why our own UDP layer, the reliability layer, serialization.
7. [integration.md](integration.md): the threads, the Bevy shell pieces, the crates, the extraction of the headless world, and the
   order of work for milestone 10.

Milestone 11 (online) on the same base: [online.md](online.md) covers dedicated hosting, the master service, relay, accounts, persistence, operations and the phases.

Security, written separately:

- [security.md](security.md): threat model, the Noise handshake and sealed packets, identity, trust on first use, the room password,
  connect tokens, accounts.
- [security-gameplay.md](security-gameplay.md): input validation and fuzzing, the authority model, physics envelope checks, rate
  limits, results and replay review, the mod policy.
- [security-ops.md](security-ops.md): denial of service, discovery safety, privacy, server hardening, testing and release gates.

Related: [roadmap](../../roadmap.md) (milestones 10, 11 and 13), [telemetry design](../telemetry/README.md) and its
[multiplayer reuse](../telemetry/multiplayer-reuse.md) ([ADR 0007](../../decisions/0007-telemetry-server.md)),
[LAN server launcher](../lan-server.md) (the original `server.dll`; a different protocol, not used),
[physics timing](../../physics-timing.md), [architecture](../../architecture.md).

Rules that apply here: engine-generic code (transport, framing, snapshot machinery) goes in `libs/blackbox-net-core` with no NFS:MW
names; the game's schemas, prediction and server go in `crates/`. The server needs the operator's own game install; nothing from it is
sent over the network. Numbers in these files are design targets or estimates unless marked measured, and nothing is measured yet.
