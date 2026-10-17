# Telemetry: what carries over to multiplayer

Milestones 10 (LAN multiplayer) and 11 (online multiplayer) are not started. This file lists the parts of the telemetry
design that carry over through a shared crate, the parts that must not, and the decisions taken now because they are
cheap now and expensive later. Companion of [features](features.md) (F-80 to F-83) and
[ADR 0007](../../decisions/0007-telemetry-server.md). Design, not built.

The multiplayer netcode ADR ([0006](../../decisions/0006-multiplayer-netcode.md)) and this file agree on the shared
crate `libs/blackbox-net-core` and its hand-off primitives. Where they still differ, section 9 lists it; both
documents are changed together. This file keeps the numbering of section 6 because 0006 refers to it.

## 1. Two protocols, not one

Telemetry is an **observer** protocol: it reads game state and never changes the simulation. Multiplayer needs a
**game-sync** protocol: every client runs the simulation and must agree with the others. They differ in what is
safe, so they stay separate. They share the pieces under them (section 2).

| | Observer (telemetry) | Game sync (multiplayer, 0006) |
|---|---|---|
| Direction | server to readers | every peer to every peer, or to a host |
| Writes | none by default; the control table is single-player only | the whole point |
| Loss | acceptable: the next snapshot replaces the lost one | not acceptable for events; snapshots are replaced, inputs are not |
| Format | JSON over HTTP, WebSocket, SSE or UDP | fixed binary layout, compact, over UDP with sequence numbers |
| Timing | the sim tick, for the reader's convenience | the sim tick is the clock everyone must share |
| Trust | local user; a token for writes | other players: identity, join checks, agreement |

Reuse the observer protocol for spectating, dashboards and debugging a netcode session. Do not reuse its JSON
transport for live play.

## 2. The shared crate: `libs/blackbox-net-core`

One Bevy-free library with no NFS:MW names, depended on by `blackbox-telemetry` and by the netcode crates. 0006
(section 14) puts the netcode primitives there too (bit packing, reliability, connection state, transport, clock
sync). Telemetry needs only the small part below. To keep the HTTP server from linking cipher or socket code, and
netcode from linking an HTTP stack, the crate is split by **cargo feature**: `core` (default, used by both) and
`netcode` (0006's modules). `blackbox-telemetry` enables `core` only. There is no HTTP code in this crate.

| Module (feature `core`) | Holds | Used by telemetry for | Used by netcode for |
|---|---|---|---|
| `ids` | `SessionId` (64 bits), `EntityId` (section 6, item 2) | session and entity fields in every payload | the same ids |
| `time` | `SimTick` (u64) and a tick rate given by the caller; conversion to sim seconds | `tick` and `t` fields, recording headers | the clock all peers agree on |
| `auth` | `SessionToken` (128 random bits from the OS, hex, constant-time check); bind policy (loopback or LAN); `Host` and `Origin` allowlist matchers | write token, LAN read token, DNS-rebinding check | the admin-token primitive; not player identity |
| `limit` | a token-bucket rate limiter keyed by address, with a bounded table | request limits | the per-address limits of the security specs |
| `queue` | `Latest<T>` (triple buffer), `EventRing<T>` (overwrite oldest, drop counter, `seq`), `CommandQueue<T>` (bounded, refuses when full) | sampler to server hand-off | sim thread to I/O thread hand-off (0006 section 13) |

The UDP sim-racing feed (F-43) does not need 0006's transport: it sends JSON datagrams with a plain
`std::net::UdpSocket`, a send-target allowlist and the 1200-byte cap, in `blackbox-telemetry`. The binary header and
packet layout of the sync protocol are 0006's (`Packable`, bit writer); telemetry does not use them.

Dependencies of `core` are small and already vetted: `getrandom`, `crossbeam-queue`, `triple_buffer` (all in
`Cargo.lock` today), `thiserror`. The `unsafe_code = "deny"` lint holds; the unsafe parts live in those crates.

What the telemetry server keeps for itself: the HTTP and WebSocket server, the channel catalogue, the recorder, the
OpenAPI document, the control table adapter and the UDP feed. If multiplayer later needs HTTP (a lobby list, say),
the `http` module moves to `blackbox-net-core` behind a feature.

## 3. Reusable as is

| Part | Where it lives | Why it carries over |
|---|---|---|
| Hand-off types | `blackbox-net-core::queue` | any state that a network thread reads while the sim writes; the same 0.2 ms budget rule |
| Budget discipline | [transport and privacy](transport-and-privacy.md) section 3 | the state a netcode layer copies per tick follows the same rule |
| Recording and replay (F-30, F-33) | `blackbox-telemetry` writer, game-side source | the best netcode debugging tool: a recording of both peers' states at the same tick shows where they diverged |
| Channel catalogue with schema numbers | [channels](channels.md), `GET /v1/channels` | the same naming and unit rules for the remote-car state |
| Token, `Host` and `Origin` checks (F-82) | `blackbox-net-core::auth` | LAN play needs an authenticated join and the same reasoning about browser pages |
| Event fields `seq`, `t` and `tick` | the observer JSON | ordered reliable events use the same three fields; the binary form is 0006's |
| Limits | `blackbox-net-core::limit` | the same bounded-table limiter in front of the join path |

## 4. Reusable with changes

### 4.1 Remote car state

The vehicle channels name the state a remote car needs. A remote-car record is a subset, with fixed-point packing
added for the wire:

- position (m), orientation (quaternion), linear and angular velocity, from `veh.position`, `veh.orientation`,
  `veh.linear_velocity`, `veh.angular_velocity`;
- wheel contact (`wheel.<i>.on_ground`, `wheel.<i>.compression`) and steer angle, for the look of the wheels;
- gear, shifting and nitrous burning, for sound and effects;
- driver inputs (`veh.controls`) so that a peer can predict the car between packets.

It is F-81 (P1). The field names are the same, so a recording of a remote car is readable by the same tools.

### 4.2 Snapshot and tick

The sim is a fixed 60 Hz step (`FIXED_STEP`). Multiplayer needs every state tagged with the tick it belongs to, and
each peer must compare two states at one tick. The telemetry frame carries `tick` from the first version (F-80), so
the counter is already in the schema when it is needed.

### 4.3 Events

Events already have `seq`, `t` and `tick`. Multiplayer events need one more rule: an event that changes shared state
(a pursuit start, a car reset) is applied at a tick both peers agree on, not at the frame it was raised. The event
record gets a target tick when used for sync. This field is added later, not now; adding an optional key is not a
schema bump ([api](api.md) section 4).

## 5. Must not be reused

- **The control table.** A remote client must not run `clock.set`, `car.reset` or `traffic.set` on another peer's
  game. Multiplayer needs its own command path with peer agreement, not the single-player console table.
- **The per-launch write token.** It stops a local web page from changing your game. It does not identify a remote
  player, so online play needs real identity and must not reuse it. `SessionToken` may be reused as a primitive
  (random generation, constant-time compare), not as the identity.
- **Localhost-only binding and origin rules** as the sole protection. A LAN peer is a remote host by definition.
- **Telemetry's LAN rule.** On a non-loopback bind the telemetry server is read-only and has no TLS
  ([ADR 0007](../../decisions/0007-telemetry-server.md), section 6). A multiplayer join cannot rely on it.
- **Strings in the stream.** Multiplayer sync sends ids and numbers. Names and chat are separate reliable messages.

## 6. Decisions taken now (they are in ADR 0007)

1. **`schema`, `tick` and `seq` in every snapshot and event** (F-80). Done in the design; to build in P0. `schema` is
   also in every recording header ([api](api.md) section 1).
2. **Entity ids.** Every car and every cop has a stable id in the telemetry catalogue, never an ECS handle (handles
   change between runs). 0006 (section 7) fixes the wire id: 8 bits, assigned by the server, the player slots the
   first 32 ids. Telemetry uses the same numbers. Because 8 bits are reused during a long session, the telemetry
   `entity` field is a `u32`: the low 8 bits are the wire id, the upper 24 a generation counter that rises each time
   an id is reused, so a recording never confuses two entities. Single player follows the same rule (the player car
   is id 0, generation 0). Telemetry exposes them in `GET /v1/entities` ([api](api.md)). The channels still name
   the driven car `veh` and wheels by library index; another entity is `GET /v1/entities/{entity}` (P2) with the
   same field names.
3. **Sim time.** All times are sim time: the sim tick plus the tick rate in the header (`tick_hz`, 60 today from
   `FIXED_STEP`). `t` is derived. Wall-clock time appears only in `started_at` fields and the status page, never
   in events or frames, so two peers' recordings compare without a shared PC clock. Sync depends on this; 0006
   section 11 states the same rule.
4. **The recorder's first line is a header** with the schema, the game build, the session id and the tick rate. Two
   peers' recordings can then be compared without guessing the version.
5. **Library split.** The generic transport and trust pieces live in `libs/blackbox-net-core` (Bevy-free, no NFS:MW
   names); the game's state filling lives in `crates/nfsmw`. A netcode crate in `libs/` must not name NFS:MW
   either, and the game-side code calls into it.
6. **Field names are stable across the two protocols.** The JSON names `schema`, `seq` and `tick` are the names the
   binary sync stream uses for the same values in its debug view (open question 3). The binary header layout
   itself is 0006's.

## 7. The original lobby protocol

[The LAN server launcher](../lan-server.md) describes how the original `server.dll` is patched and started. Its
25-byte datagram layout is still unverified (the spec's own status line). The telemetry design does not speak that
protocol, and nothing here assumes it. A capture of a real session would settle the open layout before milestone 10
picks a wire format; that decision is separate from this file.

## 8. Open questions

1. Recording inputs or states for replay (see [features](features.md), open question 4). For netcode debugging the
   answer is both: inputs at the tick for deterministic replay, states at the tick for diffing peers.
2. Does the sim stay deterministic across machines? `blackbox-vehicle` is documented as deterministic and fixed-step
   (its crate docs). Whether the whole world (AI, traffic, props) is deterministic is not known. Lockstep needs it;
   snapshot interpolation does not. Decide before milestone 10 chooses the sync model.
3. Whether the observer JSON can be a debugging view of the binary sync stream, decoded by the server, instead of a
   separate encoder. Proposal: yes, once the binary layout exists; the shared envelope header makes it cheap.

## 9. Differences with 0006 still to settle

1. **Entity id width.** 0006: 8 bits. Telemetry: the same 8 bits plus a generation counter in a `u32` (section 6,
   item 2). If 0006 widens its id, telemetry follows.
2. **Crate features.** This file asks for a `core` / `netcode` feature split in `blackbox-net-core`. 0006's table
   (section 14) lists everything in one crate and says telemetry "uses these and none of the rest"; a feature split
   makes that enforceable.
3. **QUIC.** 0006 chooses its own UDP layer; the security specs mention `quinn` and `rustls`. If a TLS stack lands
   for online play, telemetry can reuse it for a TLS-protected LAN mode (a revisit trigger in ADR 0007).
4. **Transport.** 0006's `Transport` trait is for game packets. The telemetry UDP feed does not use it.
