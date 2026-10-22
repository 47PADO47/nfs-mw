# Multiplayer protocol: messages, layout, versioning

Status: **partly built**. The transport layers (sections 4 and 5: handshake, sealing, cookies, rate limits) are built in
`libs/blackbox-net-core`, and the wire foundation of `crates/nfsmw-net` (entity schema, input frame, reliable messages, version
checks, loadouts; 2026-10-22) is built and unit-tested; a running session is not. Byte counts not marked built are design
targets, not measurements. Written 2026-10-16. Decisions and budgets: [ADR 0006](../../decisions/0006-multiplayer-netcode.md) and
[bandwidth](bandwidth.md).
Algorithms that use these messages: [sync model](sync-model.md). The secure channel (Noise handshake with `snow`, the stateless
cookie, ChaCha20-Poly1305 sealing, the replay window, rekeying, connect tokens, the room password) is specified in
`security.md` section 4 and in `security-gameplay.md`; this file fixes the packet layouts, fields and states, and where the two
disagree the security files win. The transport is our own UDP layer ([transport options](transport-options.md)). This protocol
shares nothing with the original game's LAN protocol ([lan-server.md](../lan-server.md)).

**Revised by [ADR 0009](../../decisions/0009-multiplayer-netcode-revisions.md) (2026-10-21).** Where this file and ADR 0009 disagree,
ADR 0009 wins; the sections it changes carry a note. In short:

- Frames are `type u8, length varint, payload` (no `len` inside `RELIABLE`). Entity ids are `u16`, gap-coded in id order.
- The kind is written only in full records, and the field mask has one bit per schema field. Message types are namespaced.
- Header byte 0: bit 0 says the ack block is valid (the sender has received a packet); the format moves to bits 3..1.
- The built `conn`, `replicate` and `secure` modules of `libs/blackbox-net-core` are the reference layout; the handshake and
  sealing as built are in [protocol-handshake](protocol-handshake.md).

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

Bit-packed frames (input, snapshot, ping) write fields most significant bit first (section 6.1). Reliable messages are postcard's
wire format (section 7, [protocol-messages.md](protocol-messages.md)). Every decoder is a pure function on `&[u8]` with the
rules of `security-gameplay.md` 1.1: size limits first, canonical form only, trailing bytes are an error, every count checked
against the bytes left.

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

**Built** (2026-10-22): the byte layouts, the cookie, the flow, the retries and every difference from the first design are in
[protocol-handshake](protocol-handshake.md). In short: byte 0 has the handshake bit and `hs_type`; `Hello` and `HelloCookie` are
padded to exactly 512 bytes and `Challenge` is 36, so no reply is larger than its request; the cookie binds the address, the
client nonce and both connection ids, and the server keeps no state until it comes back; then `NoiseMsg2` and `NoiseMsg3` run
Noise `XX` (LAN) or `XK` (pinned key) with the game's prologue `nfsmw-mp/<major>`.

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

**Built** (2026-10-22), layout in [protocol-handshake](protocol-handshake.md) section 5: flags, connection id and packet number
(7 bytes) in the clear as associated data; `ack`, `ack_bits` and the frames sealed with ChaCha20-Poly1305 under the Noise
transport keys; a 16-byte tag. The receive window of 1,024 numbers is checked before the tag and updated after it; a bad tag is
dropped silently and counted. Flag bits 6 and 5 are the key phase (rekey every 2^24 packets or one hour). Path validation on a new
address (`PATH_CHALLENGE`) is not built yet.

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

> **Built** (`nfsmw-net`, module `input`): the payload after the frame type and length (ADR 0009), so `type` is not repeated
> inside it; 3 header bytes, at most 43 bytes. Flags bit 1 is set exactly when some input carries a gear, and `same` is used
> whenever an input equals the one before; a decoder refuses any other form, a steer code above 510, both shift edges at once,
> reserved bits, nonzero padding and trailing bytes. The client builds each frame from the newest unacknowledged ticks (up to 8),
> so every input is repeated until `ack_input` passes it. The server drops inputs for ticks it has simulated and more than 12 ticks
> ahead, holds the last input for 6 ticks (100 ms) without its shift edges, then uses no pedal and a quarter brake.

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

> **Built** (`nfsmw-net`, module `schema`): the seven kinds register in wire order, so the replicate kind id is the wire kind
> minus one. Fields per kind (widths in bits; `d` = difference-coded): player car: position 3 × 20 d, orientation 32, linear
> velocity 3 × 14, angular velocity 3 × 10, wheel compression 24, wheels on ground 4, wheel steer 8, skid 8, rpm 10, gear 4, flags
> 16, type 16, owner 6, inputs 20 (286 bits full with the kind id). Reduced player car: position 3 × 20 d, yaw 8, speed 8, flags 8,
> owner 6. Traffic: position 3 × 20 d, yaw 8, speed 9, flags 8, type 16. Police: position 3 × 20 d, yaw 10, speed 10, flags 16,
> type 16. Helicopter: position 3 × 20 d, orientation 32, flags 24, type 16. Roadblock: position 3 × 20 (raw), yaw 8, flags 8,
> type 16. Moving prop: position 3 × 20 d, orientation 32, flags 8, type 16. Owner is 0 for none, else the player id plus one.
> Speeds are signed forward speeds over -32..96 m/s. The own block is built as listed above (226 bits used, 30 reserved zero). The
> global block is not built. `schema_id` is a 64-bit FNV-1a over the replicate schema id, the position range and the fixed
> ranges.

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

The catalogue (admission, lobby and session, the settings struct, the loadout, modes, gameplay events, chat and bulk), its
encoding and its type numbers are in [protocol-messages.md](protocol-messages.md). Built in `crates/nfsmw-net` (module `message`):
postcard's wire format written by hand, `type` u16, `format` u8, body; gameplay events moved to the game namespace (`0x11xx`).

## 8. Versioning and compatibility

### 8.1 Numbers

| Number | Where | Rule |
|---|---|---|
| `proto_major` | Hello, Noise prologue (`nfsmw-mp/<major>`), discovery | Bumped for any incompatible change of a header, a known frame or a required message. A mismatch cannot connect |
| `proto_minor` | `Auth`, `JoinAccepted`, discovery reply | Bumped for additive changes. Both sides use the lower value for optional features |
| `format` | each reliable message | One byte per message type; decoding is a match on `(type, format)` |
| `schema_id` | `JoinAccepted` (inside `server_caps`) | A 64-bit hash of the entity schema tables (kinds, fields, widths, ranges; ADR 0009). A schema change bumps `proto_minor` and keeps the old schema served for one release |
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

> **Built** (`nfsmw-net`, module `version`): major 1, minor 0. `data_id` is the first 16 bytes of SHA-256 over a tag, the
> executable's SHA-256 and the required files (name and size, sorted by name); `sim_build` is 8 bytes of SHA-256 over the git hash
> and the target. The server checks, in order: major (`ProtocolMismatch`, detail 1 the client must update, 2 the server must),
> minor against the oldest it serves (the client must update), `data_id` (`DataMismatch`), the sim build only when the server
> requires it (`SimBuildMismatch`, a reason added for MP-62), then the mods (`ModMismatch`, detail the number of differences) under
> the off, lenient (LAN default) or strict policy of `security-gameplay.md` 7.3, ignoring client-only mods. The accepted minor is
> the lower of the two. Optional features are listed with the first minor that has them; none is defined yet.

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
| Reliable message | 16,382 bytes (what 16 fragments carry, `conn::MAX_MESSAGE`); bulk transfer 4 MiB |
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
