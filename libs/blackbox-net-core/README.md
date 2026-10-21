# blackbox-net-core

Generic **netcode building blocks** for a server-authoritative racing game. The crate uses the standard library only and is
sans-IO: no threads, no async runtime and no wall clock. Every function that needs the time takes `now_ms` from the
caller, so the tests and the packet simulator are deterministic. It contains no game names; the game registers its entity
schema and chooses budgets and ports.

| Module | What it does |
|---|---|
| `bits` | MSB-first `BitWriter` and `BitReader` over caller-owned slices; varints with any group width; zigzag; gap-coded `u16` ids; quantisers for ranged floats, angles and smallest-three quaternions |
| `link` | The non-blocking `Link` datagram trait. `UdpLink`; `MemoryNet`, an in-process network the listen host's own client uses; `LossyLink`, seeded loss, latency, jitter, duplication and reordering on a caller-driven clock |
| `conn` | A connection, sans-IO: a 15-byte header with packet number, `ack` and `ack_bits`; skippable `type, length, payload` frames; 4 reliable ordered channels with fragments (16 × 1 KiB); RTT and loss estimates; `Delivered` and `Lost` events for delta baselines. 16 bytes per datagram are reserved for the future seal |
| `replicate` | A schema of kinds and fields, a 64-frame `History` of quantised values, delta and full codecs (difference-coded fields), an `EncodeCache` shared by every client in a tick, `ClientState` (per-client views, priority budget, removals) and `ClientViews` (the client's decoder), plus a `SpatialHash` for relevancy |
| `clock` | `ClockSync` (lowest-RTT sample of the last 10) and the slew-or-step rule for the local tick clock |
| `session` | The session phases (lobby, loading, load barrier with timeout, intro, countdown, running, results) and per-peer states |
| `discovery` | The 64-byte LAN discovery request and reply |
| `rng` | `SplitMix64` for the simulator and tests |

Design: [ADR 0006](../../docs/decisions/0006-multiplayer-netcode.md), amended by
[ADR 0009](../../docs/decisions/0009-multiplayer-netcode-revisions.md). Wire layouts:
[protocol](../../docs/specs/multiplayer/protocol.md) and the module docs. Measurements:
[bandwidth](../../docs/specs/multiplayer/bandwidth.md) section 7.

Not here yet: the Noise handshake and packet sealing, the net thread and its hand-off queues, and everything game-specific
(schemas, prediction, the server loop). They come in later steps of milestone 10.

```sh
cargo test -p blackbox-net-core                                      # unit, 16-client loopback, zero-allocation tests
cargo run --release -p blackbox-net-core --example net_bench         # encode cost with and without the shared cache
```

License: MIT OR Apache-2.0.
