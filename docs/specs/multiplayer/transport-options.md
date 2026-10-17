# Multiplayer transport: crates evaluated and the reliability layer

Status: **design only**. Nothing is built. Written 2026-10-16. The evidence behind
[ADR 0006](../../decisions/0006-multiplayer-netcode.md) section 9 (transport) and section 10 (serialization). The secure
channel (Noise via `snow`, the cookie, the sealed packet) is in `security.md` section 4; this file covers the layer under and
around it.

## 1. Facts checked

Crate metadata from the crates.io API on 2026-10-16. Licence strings are as published; `cargo deny check licenses` on the real
tree decides, against the allow list in [`deny.toml`](../../../deny.toml) (MIT, MIT-0, Apache-2.0 with or without the LLVM
exception, BSD-2/3-Clause, ISC, Zlib, Unicode-3.0, CC0-1.0, MPL-2.0, BSL-1.0).

| Crate | Version, last release | Licence | MSRV | Notes |
|---|---|---|---|---|
| `quinn` | 0.11.12, 2026-09-14 | MIT OR Apache-2.0 | 1.85 | QUIC; needs `rustls` and by default `tokio` |
| `rustls` | 0.23.46 | Apache-2.0 OR ISC OR MIT | 1.71 | TLS for QUIC |
| `ring` | 0.17.14 | Apache-2.0 AND ISC | 1.66 | crypto provider |
| `aws-lc-rs` | 1.18.2 | ISC AND (Apache-2.0 OR ISC) | 1.71 | its `-sys` crate has an `OpenSSL` term (not checked; not in the allow list) |
| `renet`, `renetcode` | 2.0.0, 2026-01-20 | MIT OR Apache-2.0 | none stated | dependencies `bytes`, `log`, `octets`; one main author |
| `laminar` | 0.5.0, 2021-05-06 | MIT/Apache-2.0 | none stated | no release for 5 years |
| `naia-shared` | 0.25.0, 2026-05-12 | MIT OR Apache-2.0 | none stated | a replication framework with its own entity model |
| `enet` | 0.3.0, 2022-03-30 | not checked | | C bindings, stale |
| `socket2` | 0.6.5, 2026-07-13 | MIT OR Apache-2.0 | 1.70 | buffer sizes, dual stack, DSCP |
| `postcard` | 1.1.3, 2025-07-24 | MIT OR Apache-2.0 | none stated | serde format; the wire format is documented as stable |
| `bitcode` | 0.6.9, 2025-12-18 | MIT OR Apache-2.0 | 1.70 | compact; the format is not meant to be stable across versions (author's documentation, not re-checked) |
| `bitstream-io` | 4.10.0, 2026-04-14 | MIT/Apache-2.0 | 1.83 | bit reader and writer |
| `chacha20poly1305` | 0.11.0, 2026-08-05 | Apache-2.0 OR MIT | 1.85 | AEAD; `snow` uses it through its resolver |
| `snow` | not queried here | Apache-2.0 OR MIT per `security.md` (unverified) | | Noise framework |

## 2. What the game needs from a transport

1. Unreliable datagrams with our own sequencing for input and snapshots, and a packet-level acknowledgement the game can use as
   the delta baseline.
2. Reliable ordered messages for lobby, chat and events, on separate channels so one cannot delay another.
3. Fragmentation of the rare message above one datagram, and a bulk path for late-join data.
4. Per-packet control of size and send time, to meet the byte budgets of [bandwidth](bandwidth.md) with a priority accumulator.
5. A stateless address check before any handshake work (anti-amplification), and sealed packets (`security.md` section 4.4).
6. A loopback path for the listen server, and a packet simulator for tests.
7. Few dependencies, no async runtime on the game thread, permissive licences.

## 3. Options

| Option | 1 | 2 | 3 | 4 | 5 | 6 | 7 | Verdict |
|---|---|---|---|---|---|---|---|---|
| **Own layer on UDP** (`std::net` or `socket2`) with `snow` for the channel | yes | yes | yes | yes | yes | yes | `socket2`, `snow`, a cipher | **Chosen** |
| `quinn` (QUIC) | datagrams have no delivery notice; we would add our own acknowledgements | streams | streams | the congestion controller and pacer decide send times | built in (Retry, 3x rule) | a custom UDP socket may be possible (unverified) | `tokio`, `rustls`, `ring`, certificates for hosts with no name | Not chosen for v1. Possible second transport (web spectators, a TLS-protected telemetry mode) behind the same `Transport` trait |
| `renet` 2.0 | messages, not packets | yes | yes | no: whole messages are the unit | netcode tokens need a token issuer; one shared key; no forward secrecy | yes | `bytes`, `octets` | Close second. The fallback if our layer proves too costly; a one-week spike is owner decision 3 of the ADR |
| `laminar` | yes | yes | yes | partly | none | partly | stale | Rejected |
| `naia` | its own model | | | | | | competes with the Bevy shell and the server-owned entities | Rejected |
| `enet` bindings | channels | | | | none | | C library, stale | Rejected |

**Why own, in one paragraph.** The game's packets are not generic messages. A snapshot is filled to a byte budget by a priority
accumulator, deltas are taken against the baseline the client acknowledged at the packet level, and the same primitives (hand-off
queues, framing, rate limits) are shared with the telemetry server. A reliability layer built for exactly this is small and well
understood (about 1.5 to 2.5 kLOC with tests, an estimate), and the secure channel that `security.md` specifies (Noise, cookie,
sealed packet header with our own packet number) assumes a layer where we own the header. QUIC would give a reviewed handshake for
free, which is a real benefit and the reason it stays the documented second transport; its costs for a 20 Hz game (a runtime, a
congestion controller that can delay datagrams after loss, no delivery notice, certificates for a LAN) were judged larger than the
cost of writing the layer, given that the security design already carries the handshake risk (cookie, replay window, rekey) with a
fuzz plan.

## 4. The reliability layer

### 4.1 Packet level

- Every sealed packet has a 32-bit packet number in its header, increasing by 1 per packet sent, never reused in a session
  (the nonce; `security.md` 4.4).
- The first 8 bytes of every sealed payload are the acknowledgement block: `ack` (the largest packet number received) and
  `ack_bits` (a 32-bit field: bit `i` set means packet `ack - 1 - i` was received). It is present in every packet, so every
  packet in either direction acknowledges the last 33 packets of the other.
- The receiver keeps a replay window of 1,024 packet numbers (`security.md` 4.7). A duplicate or too-old number is dropped
  before any frame is parsed.
- The sender stores for each packet number sent (a ring of 256): the send time, the reliable message ids it carried, and the
  snapshot tick it carried (if any). An acknowledgement of a packet marks those messages delivered and tells the snapshot sender
  that the client has that baseline.
- A packet is declared **lost** when a packet at least 3 numbers later is acknowledged, or when 1.5 × smoothed RTT + 4 ms has
  passed without an acknowledgement. Lost reliable messages are re-sent in a later packet; lost snapshots and inputs are not
  (the next ones supersede them; inputs carry redundancy).
- RTT is measured from acknowledgement times (with the ack delay carried implicitly: acknowledgements are sent in the next
  outgoing packet, at most 33 ms later; an acknowledgement-only packet goes out after 33 ms of silence). The clock-sync pings of
  [sync model](sync-model.md) section 5 give a second measurement.

### 4.2 Reliable messages

- Four reliable channels, ordered within a channel, independent of each other: `control` (0), `events` (1), `chat` (2), `bulk` (3).
- A reliable frame carries `channel`, a 16-bit `msg_id` (per channel, wrapping), and a fragment field. A receiver delivers
  messages of a channel strictly in id order and drops ids it has delivered (`security.md` 4.7).
- A message larger than the frame space is split into up to 16 fragments; the first fragment declares the total size, which is
  checked against the per-type limit before any buffer is allocated (`security-gameplay.md` 1.1).
- At most 256 messages in flight per channel (`security-gameplay.md` 1.2). A sender that reaches the limit holds the next
  message in its own queue; a receiver of a peer that never acknowledges ends the connection.
- Resends are limited to the connection's byte budget. Lost reliable data is sent first, then new reliable data, then the
  unreliable frames, within the budget.

### 4.3 Congestion and pacing

The game does not have a TCP-style congestion window. It has a **budget per connection** (bytes per second, set by the server
settings: 32 KB/s LAN cap, 12 KB/s online default) and the priority accumulator that fills each snapshot to its share. Two signals
reduce the budget before the network does it for us: a loss over 10 % over 2 seconds, or an RTT rise above 2.5 × the minimum, each
cut the snapshot rate of that client by a step (30 to 20 to 10 Hz), and it recovers after 10 clean seconds. This is a documented
risk for the online milestone: a network that drops packets because of our traffic should lower our rate, and the two signals above
are the whole mechanism. If online tests show unfair behaviour, a delay-based controller is added inside this layer without changing
the packet format.

### 4.4 Loopback and simulation

- The `Transport` trait is two calls (send a datagram to an address, receive datagrams) plus the local address. Implementations:
  UDP, an in-memory pair (listen server, tests) and a wrapper that drops, delays, reorders and duplicates datagrams by settings
  (loss, jitter, reorder, duplicate) with a seeded random source, so a failing run can be repeated. The wrapper is what the console
  command `net.sim` and the test suite use.
- The loopback pair is used by tests. The listen server's own client talks to the server over a real loopback UDP socket by
  default, so that the host runs the same code as every client, handshake included.

## 5. Serialization

| Message class | Format | Reason |
|---|---|---|
| Input, snapshot, ping (frames of the sealed payload) | Hand-written bit packing with a `Packable` trait, quantisation tables in `nfsmw-net` | The budgets need 10-bit and 14-bit fields; a generic serializer cannot quantise |
| Lobby, chat, events, session state, join and auth messages | `serde` with `postcard` | Low rate; stable documented format; `serde` is already a workspace dependency |
| Handshake frames | Fixed layouts, hand-written (`protocol.md` section 4) | Pre-authentication data must be parsed by the smallest possible code |

Rules from `security-gameplay.md` 1.1 apply to both: decoders are pure functions on `&[u8]`, trailing bytes are an error,
unknown enum values are an error, and postcard is decoded with `take_from_bytes` from a slice already capped by the message limit.
Postcard enums are append-only: a new variant goes at the end and a removed one keeps its number. `bitcode` is rejected because
its format is not stable across versions. `bincode` was not evaluated (maintenance status not checked).

## 6. What would make us change

- The reliability layer costs more than a `renet` spike shows: adopt `renet` behind the `Transport` trait, and re-check the
  security design (it loses forward secrecy and the key binding of connect tokens; `security-ops.md` section 6).
- A fuzzing or review finding that the cookie, replay or rekey code is hard to get right: move to QUIC (`security-ops.md` gate G3)
  and keep every message above the transport.
- Browser spectating becomes a requirement: add a QUIC or WebTransport implementation of `Transport`.

## 7. Could not verify

- The behaviour of `renet` and `quinn` in practice (only metadata was read).
- Whether `quinn` accepts a custom UDP socket for loss injection.
- The licence and version of `snow` and of the crates it pulls; `cargo deny` decides when they are added.
- The 1.5 to 2.5 kLOC estimate of the reliability layer.
