# blackbox-net-core

Generic **netcode building blocks** for a server-authoritative racing game. The default build uses the standard library only,
and below `endpoint` the crate is sans-IO: no threads, no async runtime and no wall clock. Every function that needs the time
takes `now_ms` from the caller, so the tests and the packet simulator are deterministic. `endpoint` (feature `netcode`) is the
one module with threads, sockets and a clock. It contains no game names; the game registers its entity schema and chooses
budgets and ports.

| Module | What it does |
|---|---|
| `bits` | MSB-first `BitWriter` and `BitReader` over caller-owned slices; varints with any group width; zigzag; gap-coded `u16` ids; quantisers for ranged floats, angles and smallest-three quaternions |
| `link` | The non-blocking `Link` datagram trait. `UdpLink`; `MemoryNet`, an in-process network the listen host's own client uses; `LossyLink`, seeded loss, latency, jitter, duplication and reordering on a caller-driven clock |
| `conn` | A connection, sans-IO: a 15-byte header with packet number, `ack` and `ack_bits`; skippable `type, length, payload` frames; 4 reliable ordered channels with fragments (16 × 1 KiB); RTT and loss estimates; `Delivered` and `Lost` events for delta baselines. Generic over a `Sealer`: `Plain` for the in-memory link, the Noise sealer of `secure` on the network |
| `replicate` | A schema of kinds and fields, a 64-frame `History` of quantised values, delta and full codecs (difference-coded fields), an `EncodeCache` shared by every client in a tick, `ClientState` (per-client views, priority budget, removals) and `ClientViews` (the client's decoder), plus a `SpatialHash` for relevancy |
| `clock` | `ClockSync` (lowest-RTT sample of the last 10) and the slew-or-step rule for the local tick clock |
| `session` | The session phases (lobby, loading, load barrier with timeout, intro, countdown, running, results) and per-peer states |
| `discovery` | The 64-byte LAN discovery request and reply |
| `rng` | `SplitMix64` for the simulator and tests |
| `ratelimit` | `TokenBucket`, `KeyedLimiter`, `FailureLimiter` (doubling locks for password guesses and auth failures), `PreAuthGate` and `PeerBudget` with the LAN and online caps of security-ops.md |
| `handoff` (feature `handoff`) | `ring`: a bounded single-producer, single-consumer queue that drops and counts the oldest entry; `latest`: a triple-buffered latest-value cell for snapshots |
| `secure` (feature `netcode`) | The handshake datagrams and `classify`; the stateless `CookieJar`; `HandshakeClient` and `HandshakeServer` (Noise `XX` and `XK` with `snow`, retries, pre-authentication caps, the 3x amplification limit); `NoiseSealer` (ChaCha20-Poly1305, rekeying by key phase, the replay window through `conn`); `IdentityKey`, `KeyId` and trust on first use over a caller's `PeerStore`; `token` (connect tokens); `password` (SPAKE2 room password) |
| `endpoint` (feature `netcode`) | The net thread. `ServerEndpoint` and `ClientEndpoint`, one thread each over a blocking `std` UDP socket (or any `Transport`, such as the lossy simulator), driving the handshake, sealed connections, keep-alives, the idle timeout, per-peer budgets and the pre-authentication gate, and talking to the game thread only through `handoff` rings (events in, sends and control out; full rings drop the oldest and count). `connect_local` attaches the listen host's own client over `MemoryNet` with no socket and no handshake. `DiscoveryResponder` (opt-in, 64 bytes for 64, source policy, rate limited) and `DiscoveryScan` (own thread) |

Design: [ADR 0006](../../docs/decisions/0006-multiplayer-netcode.md), amended by
[ADR 0009](../../docs/decisions/0009-multiplayer-netcode-revisions.md). Wire layouts:
[protocol](../../docs/specs/multiplayer/protocol.md) and the module docs. Measurements:
[bandwidth](../../docs/specs/multiplayer/bandwidth.md) section 7.

Handshake and sealing as built: [protocol-handshake](../../docs/specs/multiplayer/protocol-handshake.md) and
[security](../../docs/specs/multiplayer/security.md) section 8.

The net thread as built: [integration](../../docs/specs/multiplayer/integration.md) section 1.

Not here yet: path validation (`PATH_CHALLENGE`), socket buffer sizes and address reuse (no `socket2`), and everything
game-specific (schemas, prediction, the server loop, the `Auth` message). They come in later steps of milestone 10.

```sh
cargo test -p blackbox-net-core                                      # unit, 16-client loopback, zero-allocation tests
cargo test -p blackbox-net-core --all-features                       # plus hand-off, handshake, sealing, tokens, password,
                                                                     # the lossy-link handshake, the seeded garbage tests and
                                                                     # the endpoints on loopback sockets and the lossy link
cargo run --release -p blackbox-net-core --example net_bench         # encode cost with and without the shared cache
cargo run --release -p blackbox-net-core --features netcode --example lan_echo -- 8 10
                                                                     # soak: a server and 8 clients on loopback, 10 s
```

**Fuzzing.** `fuzz/` holds `cargo-fuzz` targets for every pre-authentication parser (`handshake_packets`, `handshake_server`,
`handshake_client`, `sealed_packet`, `connect_token`, `discovery`, `pake_message`). It is its own workspace, so the normal
build never compiles libFuzzer. It needs a nightly toolchain and `cargo install cargo-fuzz`; run from this directory:

```sh
cargo +nightly fuzz run handshake_server               # until stopped; add -- -max_total_time=600 for ten minutes
cargo +nightly fuzz list                               # all targets
```

The targets type-check on stable (`cargo check --manifest-path fuzz/Cargo.toml`); they have not been run under libFuzzer here.
The in-tree garbage tests (`src/secure/tests/garbage.rs`, `src/conn/tests/fuzz.rs`, `src/discovery/tests.rs`) run the same
parsers on seeded random input in every `cargo test`.

License: MIT OR Apache-2.0.
