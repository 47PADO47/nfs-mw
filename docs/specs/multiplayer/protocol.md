# Multiplayer protocol: messages, layout, versioning

Status: **design only**. Nothing is built, and every byte count below is a design target, not a measurement. Written
2026-10-16. Decisions and budgets: [ADR 0006](../../decisions/0006-multiplayer-netcode.md) and [bandwidth](bandwidth.md).
Algorithms that use these messages: [sync model](sync-model.md). The secure channel (Noise handshake with `snow`, the stateless
cookie, ChaCha20-Poly1305 sealing, the replay window, rekeying, connect tokens, the room password) is specified in
`security.md` section 4 and in `security-gameplay.md`; this file fixes the packet layouts, fields and states, and where the two
disagree the security files win. The transport is our own UDP layer ([transport options](transport-options.md)). This protocol
shares nothing with the original game's LAN protocol ([lan-server.md](../lan-server.md)).

**Revised by [ADR 0009](../../decisions/0009-multiplayer-netcode-revisions.md) (2026-10-21).** Where this file and ADR 0009 disagree,
ADR 0009 wins; the sections it changes carry a note. In short:

- Frames are `type u8, length varint, payload` (no `len` inside `RELIABLE`). Entity ids are `u16`, gap-coded in id order.
- The kind is written only in full records, and the field mask has one bit per schema field. Message types are namespaced.
- The built `conn` and `replicate` modules of `libs/blackbox-net-core` are the reference layout until sealing exists.

## 1. Layers

```
game messages      lobby, chat, events (reliable)  |  input, snapshot, ping (unreliable)
                   ──────────────────────────────────────────────────────────────────
frames             typed frames inside one sealed packet (section 5)
                   ──────────────────────────────────────────────────────────────────
sealed packet      header (7 B) + sealed payload (ack block + frames) + 16 B tag
                   ──────────────────────────────────────────────────────────────────
handshake          Hello, Challenge, Noise messages (section 4); runs once per connection
                   ──────────────────────────────────────────────────────────────────
                   UDP
 beside it:        discovery on its own UDP port (section 3), unauthenticated hints only
```

Bit-packed frames (input, snapshot, ping) write fields most significant bit first (section 6.1). Reliable messages are postcard
(section 7). Every decoder is a pure function on `&[u8]` with the rules of `security-gameplay.md` 1.1: size limits first,
canonical form only, trailing bytes are an error, every count checked against the bytes left.

## 2. Transport mapping

| Item | Value |
|---|---|
| Transport | UDP; one game port per server (IPv4 and IPv6) |
| Game port | set by the server config; the default is chosen when the code is written (ADR owner decision 5) and must not collide with the ports in [lan-server.md](../lan-server.md) |
| Discovery port | a second UDP port (default: the game port plus 1) |
| Datagram limit | 1,200 bytes of UDP payload, never fragmented by IP. Frame space in a sealed packet: 1,200 − 7 − 8 − 16 = 1,169 bytes |
| Idle timeout | 10 s with no valid packet ends the connection. Keep-alive: an acknowledgement-only packet after 2 s of silence (input and snapshot traffic already keeps it alive in play) |
| Connection id | 16 bits, chosen by the receiver of the packets it names; lets the server find the session without trusting the source address |
| Channels | reliable: `control` 0, `events` 1, `chat` 2, `bulk` 3. Unreliable: input, snapshot, ping frames |

## 3. Discovery (LAN)

Plain, unauthenticated UDP on the discovery port. A reply is only a hint; the server's identity is proved by its key in the
handshake, never taken from the reply. The rules for amplification, rate limits and opt-in are in `security-ops.md` section 2.3.
Both datagrams are **exactly 64 bytes**, so a reply is never larger than its request.

**Request** (client to the subnet broadcast address and to the "recent servers" list):

| Offset | Size | Field |
|---|---|---|
| 0 | 4 | `magic` = `NMDQ` |
| 4 | 1 | `proto_major` |
| 5 | 1 | `format_version` (of this datagram; 1) |
| 6 | 8 | `nonce`, random per request |
| 14 | 50 | zero padding (must be zero; a shorter datagram is ignored) |

**Reply** (unicast to the sender):

| Offset | Size | Field |
|---|---|---|
| 0 | 4 | `magic` = `NMDR` |
| 4 | 1 | `proto_major` |
| 5 | 1 | `format_version` |
| 6 | 8 | `nonce` echoed; a reply with another nonce is ignored |
| 14 | 1 | `proto_minor` |
| 15 | 8 | `key_hash`: first 8 bytes of the hash of the server's static public key; only to match a saved entry |
| 23 | 2 | `game_port` |
| 25 | 1 | `players` |
| 26 | 1 | `max_players` |
| 27 | 1 | `flags`: bit 0 password, bit 1 in progress, bit 2 join in progress allowed, bit 3 mods required, bit 4 dedicated |
| 28 | 1 | `mode` (section 7.5) |
| 29 | 4 | `data_id` short (first 4 bytes of the data id, section 8.3), for an early "incompatible" label |
| 33 | 24 | `name`, UTF-8, zero padded |
| 57 | 7 | zero padding |

The list shows `key_hash` status ("known", "new") and greys out servers whose `proto_major` differs. Broadcast reaches one subnet
only; a typed address is the fallback across routers. The online server list (milestone 11) is a separate HTTPS service, not in
this file.

## 4. Handshake

The handshake is stateless on the server until a client proves it can receive at its address (`security.md` 4.4). All handshake
packets have the handshake bit set in byte 0 and are sent in the clear (Noise messages carry their own encryption).

**Byte 0 of every packet.**

| Bit | Handshake packet | Data packet |
|---|---|---|
| 7 | 1 | 0 |
| 6 to 4 | `hs_type` (below) | key phase (2 bits at 6 and 5), bit 4 reserved 0 |
| 3 to 0 | packet format version (0) | reserved 0 |

`hs_type`: 0 `Hello`, 1 `Challenge`, 2 `HelloCookie`, 3 `NoiseMsg2`, 4 `NoiseMsg3`, 5 `Reject`.

| Packet | Dir | Fields after byte 0 | Size |
|---|---|---|---|
| `Hello` | C to S | `proto_major` u8, `format_version` u8, `client_nonce` 16 bytes, `client_conn_id` u16, zero padding | exactly 512 |
| `Challenge` | S to C | `server_conn_id` u16, `bucket` u8, `cookie` 32 bytes | 37 (never larger than the request) |
| `HelloCookie` | C to S | `proto_major`, `format_version`, `pattern` u8 (0 = XX, 1 = XK), `client_nonce`, `client_conn_id`, `server_conn_id`, `bucket`, `cookie`, `noise1_len` u16, Noise message 1, zero padding | exactly 512 |
| `NoiseMsg2` | S to C | `client_conn_id` u16, `len` u16, Noise message 2 | under 3 × 512 |
| `NoiseMsg3` | C to S | `server_conn_id` u16, `len` u16, Noise message 3 | under 512 |
| `Reject` | S to C | `reason` u8: 1 server full, 2 protocol major unsupported, 3 shutting down. Sent only to an address that presented a valid cookie | 2 |

**Cookie.** `cookie = HMAC(server_secret, client_address || client_nonce || bucket_time)`, 32 bytes, bucket length 15 s; the
server accepts the current and the previous bucket (`bucket` names which). It keeps no state between `Hello` and `HelloCookie`.
The secret rotates every hour with two kept. A `HelloCookie` that fails the check is dropped silently.

**Flow.**

1. `Hello` (client nonce, its connection id). The server computes the cookie and answers `Challenge`; no other work.
2. `HelloCookie` with Noise message 1. The server verifies the cookie in constant time, only then allocates a handshake slot and
   runs the Noise step. Until message 3 verifies, the server sends the client at most 3 times the bytes it has received from it.
3. `NoiseMsg2`, then `NoiseMsg3`. LAN uses pattern `XX`; online uses `XK` with the server key from the connect token. The Noise
   prologue is `nfsmw-mp/<major>`. A version mismatch fails the handshake before any game data.
4. Both sides switch to data packets (keys from the Noise transport state, one per direction). The first sealed reliable message is
   `Auth` from the client; the server's first sealed packet confirms the keys.

A handshake that does not progress is dropped after 5 s (the slot is freed). A client retries `Hello` with a fresh nonce at 0.5 s,
1 s and 2 s, then reports "no answer".

**After the keys (control channel).**

```
client                                              server
  │ Auth {minor, sim_build, data_id, token?, resume?} ──► │  checks token (online), data id, version
  │ ◄───────────────────── PakeStart                     │  only if the room has a password (SPAKE2 messages follow)
  │ Join {name, loadout, mods} ──────────────────────────► │  loadout, mod and slot checks
  │ ◄───────────────────────── JoinAccepted / JoinRejected │
  │ ◄───────────────────────── LobbyState                  │
  │ SetReady, SelectLoadout, Chat ...                      │
  │ ◄───────────────────────── LoadWorld                   │
  │ LoadComplete ────────────────────────────────────────► │
  │ ◄───────────────────────── StartAt(tick) / Respawn     │
  │ INPUT ──► ◄── SNAPSHOT; PING / PONG every second       │
  │ DISCONNECT frame (either side) or idle timeout         │
```

The server ignores unreliable frames from a connection that has not been admitted, and closes a connection that has not sent
`Auth` within 5 s of the keys. Rate limits are in `security-gameplay.md` section 5. **Resume:** `JoinAccepted` carries a sealed,
single-use `resume_token` valid for 30 s; a client that lost its connection runs a **fresh handshake** and sends `Auth` with it. If
the slot is still held, the server answers `JoinAccepted` with the same `player_id` and `slot` and the next snapshot is full.

## 5. Sealed packets and frames

### 5.1 Sealed packet

| Offset | Size | Field | Sealed? |
|---|---|---|---|
| 0 | 1 | flags (bit 7 = 0, key phase at bits 6 and 5) | no, authenticated |
| 1 | 2 | connection id (of the receiver) | no, authenticated |
| 3 | 4 | packet number (low 32 bits; the receiver expands it to 64 bits for the nonce) | no, authenticated |
| 7 | 4 | `ack`: largest packet number received from the peer | yes |
| 11 | 4 | `ack_bits`: bit `i` set means packet `ack - 1 - i` was received | yes |
| 15 | n | frames | yes |
| 15 + n | 16 | authentication tag | |

The 7 header bytes are the associated data. A packet with an invalid tag is dropped silently and counted (no error reply). The
receiver keeps a window of 1,024 packet numbers and drops duplicates and older numbers before parsing a frame. A packet from a new
address with a valid tag and a newer number is accepted, but no bulk data goes to the new address until it answers a
`PATH_CHALLENGE`. Rekeying every 2^24 packets or one hour is signalled by the key phase bits (`security.md` 4.4).

### 5.2 Frame types

> **ADR 0009:** every frame except `PADDING` is `type`, then the payload length as a LEB128 varint (at most 3 bytes), then the
> payload, so every type is skippable and the known-or-skippable split below no longer decides how to parse. The type numbers stay.

A frame starts with a `type` byte. Types `0x01` to `0x04` and `0x10` to `0x13` are **known frames**: their length follows from
their fields, and every build must understand them. Any type from `0x20` up (and the reserved types marked below) is **skippable**:
`type` is followed by a `u16` byte length so an older receiver can skip it.

| Type | Name | Direction | Layout |
|---|---|---|---|
| `0x00` | PADDING | both | one zero byte, repeated |
| `0x01` | INPUT | client to server | section 6.2 |
| `0x02` | SNAPSHOT | server to client | section 6.3 |
| `0x03` | PING | client to server | section 6.4 |
| `0x04` | PONG | server to client | section 6.4 |
| `0x05` | SPECTATOR_HINT | spectator to server | reserved, skippable: a camera position for relevancy, 5 Hz |
| `0x06` | VOICE | both | reserved, skippable: voice chat (MP-43), an Opus frame with speaker id and sequence; not defined in v1 |
| `0x07` | TELEMETRY_DEBUG | server to client | reserved, skippable: a debug view of the stream for the telemetry bridge (MP-85) |
| `0x10` | RELIABLE | both | `channel` 2 bits, `first` 1, `last` 1, 4 reserved zero (one byte); `msg_id` u16; `len` u16; `len` bytes of data |
| `0x11` | DISCONNECT | both | `reason` u8: 0 normal, 1 timeout, 2 kicked, 3 protocol error, 4 shutting down. The sender then stops sending |
| `0x12` | PATH_CHALLENGE | both | 8 random bytes |
| `0x13` | PATH_RESPONSE | both | the same 8 bytes, sent to the address the challenge came from |
| `0x20` to `0x7F` | | | reserved for the project, skippable |
| `0x80` to `0xFF` | | | experimental or mod use; never gameplay state (`security-gameplay.md` section 7), skippable |

A receiver drops a datagram's remaining frames if a known frame fails to decode, counts it, and applies the strike rules of
`security-gameplay.md` 1.3. An unknown skippable type is skipped and counted.

### 5.3 Reliable frames

> **ADR 0009:** the frame length gives the data length, so `RELIABLE` is `flags` (channel 2 bits, first, last, 4 zero bits),
> `msg_id` u16 little-endian, data. Fragments carry at most 1,024 data bytes. The first fragment of a split message starts with the
> total length as a `u16`.

A `RELIABLE` frame carries a whole message or one fragment of it. The message is `type` u16, `format` u8, then the postcard body
(section 7). Messages up to the frame space go in one frame with `first` and `last` set. Larger messages (at most 16 KiB; the bulk
channel carries data as chunks of at most 1 KiB, each a message of its own) are split into up to 16 fragments with consecutive
`msg_id`s; the first fragment states the total length (a `u16` before the message), checked against the type's limit before any
buffer is allocated. Delivery is ordered within a channel; at most 256 messages are in flight per channel. Loss handling and
resends: [transport options](transport-options.md) section 4.

## 6. Unreliable frames

### 6.1 Bit packing rules

- Fields are written MSB first with no alignment padding; a frame ends padded to a byte boundary with zero bits (nonzero padding is
  a decode error).
- Quantised values: `q = round((x - min) / step)`, stored in the stated bits. A decoder rejects a value outside the field's range
  (it is an error, not a clamp) and any non-finite number after dequantisation.
- `varint`: 7 bits per byte, continuation in the top bit, at most 5 bytes for a `u32`, canonical form only.
- Tick numbers are the full `u32` in snapshots and the low 16 bits elsewhere; the receiver expands the low 16 bits against its own
  tick (valid for plus or minus 9 minutes).

### 6.2 INPUT (client to server, `0x01`)

| Field | Bits | Meaning |
|---|---|---|
| `type` | 8 | `0x01` |
| `first_tick` | 16 | low 16 bits of the tick of the first input in this frame |
| `count` | 4 | 1 to 8 inputs |
| `flags` | 4 | bit 0: the session uses a clutch; bit 1: some ticks carry a gear select; bits 2 and 3 reserved, zero |
| inputs | `count` × (1 or 40) | one per tick, below |

Per-tick input:

| Field | Bits | Range and step |
|---|---|---|
| `same` | 1 | 1 means "equal to the previous tick's input"; nothing else follows. Never set on the first input of a frame |
| `throttle` | 7 | 0..127 → 0..1 |
| `brake` | 7 | 0..127 → 0..1 |
| `steer` | 9 | stored value 0..510 with offset 255 → −1..1 |
| `handbrake` | 4 | 0..15 → 0..1 |
| `nos`, `shift_up`, `shift_down` | 3 × 1 | button state; the two shifts are edges, set on the tick they happen only |
| `gear_select` | 1 + 4 | a present flag, then the gear id; read only when the `flags` bit says so |
| `clutch` | 4 | 0..15 → 0..1; present only when the `flags` bit says so |

A reset is not an input bit: it is the reliable message `ResetRequest` (section 7.2), limited to one per 10 s. The packet-level
acknowledgements of the sealed packet replace any snapshot acknowledgement field: the server knows which snapshot packets the
client received from `ack` and `ack_bits` of the client's packets.

### 6.3 SNAPSHOT (server to client, `0x02`)

| Field | Bits | Meaning |
|---|---|---|
| `type` | 8 | `0x02` |
| `tick` | 32 | the server tick of this world state |
| `baseline` | 8 | `tick - baseline_tick` (1..255); 0 means full, no baseline. A larger gap is sent as full |
| `part` | 4 + 4 | part index and part count (1 in most snapshots, up to 8) |
| `flags` | 8 | bit 0 own block, bit 1 global block, bit 2 cause set, bits 3 to 6 `group_count` (0..15), bit 7 reserved zero |
| `ack_input` | 16 | low 16 bits of the last input tick the server applied for this client |
| own block | 256 | if flagged (part 0 only), below |
| global block | 96 | if flagged (part 0 only), below |
| groups | variable | `group_count` entity groups |

The header (up to `ack_input`) is 10 bytes.

**Own block** (the receiving player's car, reduced authoritative state at tick `ack_input`): position 3 × 20, orientation 32,
linear velocity 3 × 14, angular velocity 3 × 10, rpm 10, gear 4, nitrous 7, boost 6, flags 8, `cause` 3 (0 none, 1 contact, 2 reset,
3 teleport, 4 freeze), input report 24 (counts of early, on time, late, missing inputs of the last 32 ticks, 5 bits each, and the
mean earliness in 4 bits). About 230 bits; 256 are reserved.

**Global block:** session phase 4; this player's pursuit state (heat level 4, busted fraction 8, cooldown fraction 8, state 3); race
state (place 5, lap 6, checkpoint 8); time of day 16; reserved bits to 96.

> **ADR 0009:** entries are no longer grouped by kind. The snapshot body is `tick` 32, baseline distance varint (0 = full),
> entry count varint, then entries in id order: the id as a gap (`varint3` of `id - prev - 1`, the first as `varint7`), `op` 2 bits,
> and for a full record the kind in 6 bits and every field, for a delta a mask with one bit per field and the changed fields
> (difference-coded fields as zigzag `varint4`). An entity not listed keeps its last value. The kinds below are the game's schema,
> registered by `nfsmw-net`.

**Entity group:** `kind` 4 bits, `count` varint, then `count` entries. **Entry:** `id` 8 bits, `op` 2 bits (`01` delta, `10` full,
`11` remove, `00` invalid), then for a delta an 8-bit field-group mask and the changed groups, for a full entry all groups. An entity
not listed is unchanged from the baseline and still relevant.

**Entity kinds** (the registry is supplied by `nfsmw-net`; the generic layer sees a kind number and a field schema):

| Kind | Content | Full size (bits) |
|---|---|---|
| 1 Player car | position; orientation; linear velocity; angular velocity; wheels; engine; inputs; flags | 250 |
| 2 Player car, reduced | position, yaw, speed, role and state flags (the "always" tier, minimap and names) | 80 |
| 3 Traffic car | position; yaw; speed; flags (brake lights, signal, type id) | 80 |
| 4 Police car | as traffic, plus siren and lights, pursuit goal 3 bits, target player 5 bits | 112 |
| 5 Helicopter | position; orientation; rotor state; spotlight target | 120 |
| 6 Roadblock or spike | position; yaw; state (forming, standing, dodged, destroyed) | 64 |
| 7 Moving prop | position; orientation; type id; broken flag | 100 |

The field widths of the player car are in [bandwidth](bandwidth.md) section 1.1 (position 3 × 20 bits for ±8,192 m in 2 cm, set by
`world_bounds` at join; orientation 32 bits, smallest three; linear velocity 3 × 14 for ±400 m/s; angular velocity 3 × 10 for
±16 rad/s; wheels 36; skid and smoke 8; engine 22; inputs 20).

**Parts.** A snapshot too large for one datagram is split into parts. Each part repeats `tick`, `baseline` and `flags` (without the
own and global bits); part 0 carries those blocks; every part holds complete entities. A client applies parts in any order and treats
the snapshot as complete when all arrived. The server learns that the snapshot was received from the packet acknowledgements of
all its parts; it then uses its tick as a baseline. A baseline older than 64 ticks, or unknown, gives a full snapshot.

### 6.4 PING and PONG (`0x03`, `0x04`)

| Frame | Fields |
|---|---|
| `PING` (client to server) | `type` 8, `seq` 8, `client_time_ms` 32 (the client's monotonic clock, only echoed) |
| `PONG` (server to client) | `type` 8, `seq` 8, `client_time_ms` 32 (echo), `server_recv_tick` 32 + `frac` 8 (1/256 tick), `server_send_tick` 32 + `frac` 8 |

The server answers in the same loop iteration. Rates and use: [sync model](sync-model.md) section 5.

## 7. Reliable messages

All are postcard structs: `type` u16, `format` u8, body. Direction: C (client to server), S (server to client). A new optional field
is appended at the end of a body; a reader that knows an older format decodes the fields it knows and skips the rest by the message
length (`security-gameplay.md` 1.1, rule 8).

> **ADR 0009:** `type` numbers fall in namespaces: core `0x0000`–`0x0FFF` (admission, lobby, session), game `0x1000`–`0x7FFF`
> (events, loadout, modes), mods `0x8000`–`0xFFFF`. A receiver skips an unknown type and counts it.

### 7.1 Admission (channel `control`)

| Type | Name | Dir | Fields |
|---|---|---|---|
| 0x0001 | Auth | C | `proto_minor`, `sim_build` (8 bytes), `data_id` (16), `caps` (flags: clutch, wheel, spectate-only), `connect_token` (optional, online), `resume` (optional) |
| 0x0002 | PakeStart | S | `scheme` (SPAKE2), the server's first message (opaque) |
| 0x0003 | PakeMsg | both | opaque bytes |
| 0x0004 | PakeConfirm | both | confirmation tag over the PAKE key and the Noise handshake hash |
| 0x0005 | Join | C | `display_name`, `loadout` (7.4), `mod_manifest` (list of id, version, hash) |
| 0x0006 | JoinAccepted | S | `player_id` (16 bytes, the hash of the identity key), `slot` (0..31), `entity_id` (u8), `session_seed` (u64), `tick` (u32), `snapshot_hz`, `settings` (7.3), `world_bounds`, `resume_token`, `server_caps`, `proto_minor` |
| 0x0007 | JoinRejected | S | `reason`: ServerFull, ProtocolMismatch, DataMismatch, ModMismatch, Banned, InProgress, LoadoutRejected, RateLimited, ServerShuttingDown; `detail` (a code, no free text), `server_proto`, `server_data_id` |
| 0x0008 | Kick | S | `reason`, `ban_minutes` (0 = none) |

A failed password, token, ban epoch or key check closes the connection with the same generic result as a full room and reveals
nothing else (`security.md` 4.6 and 4.8).

### 7.2 Lobby and session (channel `control`)

| Type | Name | Dir | Fields |
|---|---|---|---|
| 0x0010 | LobbyState | S | `players` (slot, id, name, loadout summary, ready, role, team, ping, flags), `settings`, `phase`, `host_slot` |
| 0x0011 | LobbyDelta | S | the changed rows (add, update, remove) |
| 0x0012 | SetReady | C | `ready` |
| 0x0013 | SelectLoadout | C | `loadout` |
| 0x0014 | SetRole | C | racer, cop or spectator (checked against the settings) |
| 0x0015 | UpdateSettings | C | host or admin only: a partial settings struct |
| 0x0016 | StartCountdown | S | `start_tick` (target tick), `seconds` |
| 0x0017 | CancelCountdown | S | `reason` |
| 0x0018 | LoadWorld | S | track or region id, `mode`, rules, time of day, weather, traffic density, `spawn_slot` |
| 0x0019 | LoadComplete | C | `streaming_ready` |
| 0x001A | StartAt | S | `start_tick`; the owner's input is neutral until then |
| 0x001B | Vote | C | `kind`, `option` |
| 0x001C | VoteState | S | `kind`, `options`, `counts`, `ends_tick` |
| 0x001D | HostCommand | C | admin command: kick, ban, mute, restart, change mode (checked against the sender's role) |
| 0x001E | StreamingStall | C | `stalled` (bool) |
| 0x001F | ResultRequest | C | ask for the last result record again (after a reconnect) |
| 0x0020 | ResetRequest | C | `tick`; at most one per 10 s; the server validates and answers with a `Respawn` event |

### 7.3 Settings struct

`name`, `max_players` (2..16), `max_spectators`, `mode`, `track`, `laps`, `traffic_density` (0..100), `cop_level_cap`, `pursuits`
(bool), `collision` (full, ghost, ghost-start), `catch_up` (bool), `nitrous` (bool), `damage` (visual, off), `allowed_cars` (list or
class), `join_in_progress` (closed, free roam, between races), `password_required` (bool), `public` (bool), `required_mods`,
`sim_build_match` (bool). At most 1 KiB (`security-gameplay.md` 1.2).

### 7.4 Loadout

`car_type` (hash id), `parts` (slot and part id pairs, the ids the car assembly uses), `paint` (id and colour indices), `wheel_set`
(id), `plate` (a short filtered string). Never numbers that tune the car: the server rebuilds the vehicle parameters from these ids
with its own game data (`security-gameplay.md` section 3, layer L2).

### 7.5 Modes

`0` free roam, `1` circuit, `2` sprint, `3` drag, `4` speedtrap, `5` knockout, `6` pursuit co-op, `7` cops vs racers, `8` tag, `9` to
`127` reserved, `128` to `255` custom (a mod mode id).

### 7.6 Gameplay events (channel `events`, server to client)

Every event record: `seq` (u32, per connection), `tick` (when it happened), `target_tick` (0 if none, else the tick at which a client
must apply it), `type`, body. Events are idempotent; a client ignores a `seq` it has applied.

| Type | Name | Body |
|---|---|---|
| 0x0100 | RaceStart | `target_tick`, `order` |
| 0x0101 | Checkpoint | `entity_id`, `index`, `tick` |
| 0x0102 | LapComplete | `entity_id`, `lap`, `time_ms` |
| 0x0103 | RaceFinish | `entity_id`, `place`, `time_ms` |
| 0x0104 | RaceResults | the result record (also written to the replay) |
| 0x0110 | PursuitStart | `pursuit_id`, `targets`, `heat` |
| 0x0111 | PursuitEnd | `pursuit_id`, `reason` (busted, evaded, abandoned), `entity_id` |
| 0x0112 | HeatChange | `entity_id`, `level` |
| 0x0113 | Infraction | `entity_id`, `kind`, `fine` |
| 0x0114 | BustedProgress | `entity_id`, `fraction` (on a change of step) |
| 0x0115 | RoadblockEvent | `id`, `state`, `position`, `yaw` |
| 0x0116 | SpikeEvent | `id`, `state`, `wheel` hit |
| 0x0120 | Collision | `a`, `b` (entity ids or world), `impulse`, `point`, `normal` (quantised) |
| 0x0121 | Damage | `entity_id`, `zone`, `severity`, `glazed` |
| 0x0122 | Puncture | `entity_id`, `wheel` |
| 0x0123 | Repair | `entity_id` |
| 0x0130 | Respawn | `entity_id`, `position`, `yaw`, `cause` |
| 0x0131 | PhaseChange | `phase` (lobby, loading, countdown, running, results), `target_tick` |
| 0x0132 | EntityRemoved | `entity_id`, `reason` |
| 0x0140 | PlayerJoined | `slot`, `player_id`, `name` |
| 0x0141 | PlayerLeft | `slot`, `reason` |
| 0x0142 | RoleChanged | `slot`, `role` |
| 0x0150 | ClockEvent | `time_of_day`, `rate` |

### 7.7 Chat (channel `chat`)

| Type | Name | Dir | Fields |
|---|---|---|---|
| 0x0200 | ChatSend | C | `channel` (all, team, whisper), `to_slot`, `text` |
| 0x0201 | ChatMessage | S | `from_slot`, `channel`, `text`, `tick` |
| 0x0202 | SystemNotice | S | `code` (join, leave, kick, mute, vote, result), `args` |
| 0x0203 | QuickMessage | C | `id` (a preset line, MP-44) |

`text` is UTF-8, 1 to 200 characters, normalised and filtered by the receiver (`security-gameplay.md` 1.1); chat is limited to one
message per second with a burst of 3.

### 7.8 Bulk (channel `bulk`)

`BulkStart {kind, id, length, hash}` (kinds: session data, mod manifest, replay chunk), then `BulkChunk {id, offset, bytes}` of at
most 1 KiB each, then `BulkEnd {id}`. The receiver verifies `length` and the 32-byte `hash` before use. Maximum 4 MiB per transfer
(`security-gameplay.md` 1.2). Only data, never code, and only what the client consented to (`security-gameplay.md` section 7).

## 8. Versioning and compatibility

### 8.1 Numbers

| Number | Where | Rule |
|---|---|---|
| `proto_major` | Hello, Noise prologue (`nfsmw-mp/<major>`), discovery | Bumped for any incompatible change of a header, a known frame or a required message. A mismatch cannot connect |
| `proto_minor` | `Auth`, `JoinAccepted`, discovery reply | Bumped for additive changes. Both sides use the lower value for optional features |
| `format` | each reliable message | One byte per message type; decoding is a match on `(type, format)` |
| `schema_id` | `JoinAccepted` (inside `server_caps`) | A 32-bit hash of the entity schema tables (kinds, fields, widths, ranges). A schema change bumps `proto_minor` and keeps the old schema served for one release |
| `sim_build` | `Auth` | The git hash and target. Informational unless the server requires a match |
| `data_id` | `Auth`, discovery (short) | The identified game data (8.3) |

### 8.2 Growth rules

- A new reliable message type or a new skippable frame type is additive (minor); old receivers skip it by length.
- A new optional field in a postcard body goes at the end; an older reader decodes what it knows and skips the rest by the message
  length. More than 16 invalid or unknown messages inside 10 s ends the connection.
- An enum gains variants at the end; a removed variant keeps its number reserved. An unknown variant in a known field is an error.
- A change to a quantisation range or width, or to the meaning of a field, is a major change unless announced by a capability flag
  that old clients do not set.

### 8.3 Data compatibility

`data_id` is a hash over the identified `speed.exe` hash (v1.3 is the only supported build,
[architecture.md](../../architecture.md)) and the list of required game file names with sizes. It does not hash large files at join.
A different data set is rejected with `DataMismatch` before the lobby. Mods are compared by `mod_manifest`; the list is empty until
the mod API (milestone 12) exists.

### 8.4 Rejection and downgrade

- Newer client, older server: the client reads `proto_minor` from `JoinAccepted` and disables features above it (no voice, no
  spectator hints). The UI shows the server version.
- Older client, newer server: the server serves the older feature set while it still supports that minor; otherwise it answers
  `JoinRejected(ProtocolMismatch)` with its version, and the UI says which side must update.
- Major mismatch: the Noise prologue differs and the handshake fails; the server may send `Reject(2)` after a valid cookie. The UI
  maps both to the same text. Discovery shows such servers greyed, with their versions.

## 9. Limits (format side)

The enforcement values (rate limits, strikes) are in `security-gameplay.md` sections 1.2 and 5; the format fixes these:

| Limit | Value |
|---|---|
| Slots in the id space | 32 (5 bits); sessions default to 16 racers and 8 spectators |
| Entity ids | 16 bits, gap-coded (ADR 0009; was 8 bits) |
| Entities in one snapshot | 64 cars, 64 traffic, 32 police (session settings) |
| Datagram | 1,200 bytes |
| Input frame | 8 ticks |
| Snapshot parts | 8 |
| Reliable message | 16 KiB; bulk transfer 4 MiB |
| Fragments per message | 16 |
| Messages in flight per channel | 256 |
| Chat text | 200 characters |
| Display name | 24 characters |
| Mod manifest | 64 entries |
| Loadout parts | 160 entries |
| Settings blob | 1 KiB |

## 10. Open questions

1. *Settled by ADR 0009: ids are 16 bits and telemetry splits its `u32` as 16 + 16.* Entity ids are 8 bits. Telemetry adds a 24-bit generation counter in a `u32` so recordings never confuse a reused id
   ([ADR 0007](../../decisions/0007-telemetry-server.md)); the wire stays 8 bits. If traffic density needs more than 255 entities,
   the width becomes a session constant in a later minor.
2. The position range (±8,192 m) is an assumption until the road and collision bounds are measured; `world_bounds` lets the server
   choose the field width.
3. How `Collision` events are throttled when two cars grind along each other: one per contact start plus one per second while it
   continues is proposed.
4. Whether `HelloCookie` can carry Noise message 1 for `XK` within 512 bytes together with the cookie: it can (48 + 32 + 40 bytes
   of fields), but this must be confirmed with the real `snow` message sizes.
