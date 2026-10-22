# Multiplayer protocol: reliable messages

Section 7 of the [protocol](protocol.md), split out on 2026-10-22 when the catalogue was built in `crates/nfsmw-net` (module
`message`). Where this file and the code disagree, the code's tests are the reference and this file is fixed. Where this file and
[ADR 0009](../../decisions/0009-multiplayer-netcode-revisions.md) disagree, the ADR wins.

Status: **built and unit-tested** (encode, decode, limits, seeded garbage), not yet carried by a running session. Direction: C
(client to server), S (server to client).

## 7.0 Encoding

A reliable message is `type` (u16 little-endian), `format` (u8), then the body. The body is in **postcard's wire format, written
by hand** in `nfsmw-net` instead of through `serde` and `postcard`: the format is the one ADR 0006 section 10 chose, and the hand-
written codec checks every count and length before it reserves memory (`security-gameplay.md` 1.1 rules 3, 4 and 11) without a new
dependency.

| Type | Layout |
|---|---|
| `u8` | one raw byte |
| `u16`, `u32`, `u64` | unsigned LEB128 varint, canonical (no trailing zero group), range-checked |
| `i32` | zigzag, then a varint |
| `bool` | one byte, 0 or 1; anything else is an error |
| `f32` | 4 bytes little-endian; a non-finite value is an error |
| `[T; N]` | the elements back to back (a byte array is its raw bytes) |
| `Option<T>` | a tag byte 0 or 1, then the value |
| string, byte string, list | a varint count, checked against the type's limit and the bytes left, then the elements |
| field-less enum | its `u8` code as a varint; an unknown code is an error |
| enum with data | a varint tag, then the variant's fields |

Fields follow the declaration order of the body struct. Text is UTF-8 with no control characters and no Unicode bidirectional
controls, already trimmed, within its character count; NFKC normalisation and grapheme counting are not done yet (the character
count is the stricter bound).

**Growth** (protocol.md section 8.2). A new field is appended and the type's `format` goes up. A reader that meets a newer format
decodes the fields it knows and skips the rest; at its own format, trailing bytes are an error; an older format than it knows is an
error until a reader learns to fill the missing fields. Every type is format 1. An unknown type is skipped and counted; more than 16
invalid or unknown messages inside 10 s ends the connection (`InvalidMessages`).

**Type numbers** (ADR 0009 namespaces): core `0x0000`-`0x0FFF` holds admission, lobby and session, chat and bulk; the game
namespace `0x1000`-`0x7FFF` holds the gameplay events, which keep the low byte of their first numbers (`0x0100` became `0x1100`);
mods own `0x8000`-`0xFFFF`.

## 7.1 Admission (channel `control`)

| Type | Name | Dir | Fields |
|---|---|---|---|
| 0x0001 | Auth | C | `proto_minor` u16, `sim_build` (8 bytes), `data_id` (16 bytes), `caps` u8 (bit 0 clutch, 1 wheel, 2 spectate only; others zero), `connect_token` (optional, at most 256 bytes, online), `resume` (optional, at most 64 bytes) |
| 0x0002 | PakeStart | S | `scheme` (0 SPAKE2), the server's first message (at most 128 bytes) |
| 0x0003 | PakeMsg | both | opaque bytes, at most 128 |
| 0x0004 | PakeConfirm | both | 32-byte confirmation tag over the PAKE key and the Noise handshake hash |
| 0x0005 | Join | C | `display_name`, `loadout` (7.4), `mod_manifest` (at most 64 entries: id, version, SHA-256, kind code or content, client-only flag) |
| 0x0006 | JoinAccepted | S | `player_id` (16 bytes), `slot` (0..31), `entity_id` (u16), `session_seed` u64, `tick` u32, `snapshot_hz` (1..60), `settings` (7.3), `world_half_extent` (f32 metres, the position range), `resume_token` (at most 64 bytes), `server_caps` (`flags` u32, `schema_id` u64), `proto_minor` |
| 0x0007 | JoinRejected | S | `reason`, `detail` (a code, no free text), `server_proto` (major, minor), `server_data_id` |
| 0x0008 | Kick | S | `reason` (kicked, banned, violations, idle, voted), `ban_minutes` (0 = none) |

`reason` codes: 0 ServerFull, 1 ProtocolMismatch (detail 1 the client must update, 2 the server must), 2 DataMismatch, 3
ModMismatch (detail: the number of differences, at most 255), 4 Banned, 5 InProgress, 6 LoadoutRejected (detail: the loadout
reason of 7.4), 7 RateLimited, 8 ServerShuttingDown, 9 SimBuildMismatch (only when the server requires the same build, MP-62).

A failed password, token, ban epoch or key check closes the connection with the same generic result as a full room and reveals
nothing else (`security.md` 4.6 and 4.8).

## 7.2 Lobby and session (channel `control`)

| Type | Name | Dir | Fields |
|---|---|---|---|
| 0x0010 | LobbyState | S | `players` (at most 32 rows, unique slots: slot, player id, name, car type, ready, role, team, ping ms, flags), `settings`, `phase`, `host_slot` |
| 0x0011 | LobbyDelta | S | at most 32 changes: add row, update row, remove slot |
| 0x0012 | SetReady | C | `ready` |
| 0x0013 | SelectLoadout | C | `loadout` |
| 0x0014 | SetRole | C | racer, cop or spectator (checked against the settings) |
| 0x0015 | UpdateSettings | C | host or admin only: a partial settings struct, every field optional, ranges as 7.3; the required mods are not host-editable |
| 0x0016 | StartCountdown | S | `start_tick`, `seconds` |
| 0x0017 | CancelCountdown | S | `reason`: host cancelled, player left, not ready, load timeout |
| 0x0018 | LoadWorld | S | `track`, `mode`, `laps`, `time_of_day` (minutes, under 1,440), `weather`, `traffic_density` (0..100), `cop_level_cap`, `pursuits`, `spawn_slot` |
| 0x0019 | LoadComplete | C | `streaming_ready` |
| 0x001A | StartAt | S | `start_tick`; the owner's input is neutral until then |
| 0x001B | Vote | C | `kind` (kick, restart, change mode, change track, skip results), `option` u32 |
| 0x001C | VoteState | S | `kind`, `options` (at most 8 pairs of option id and count), `ends_tick` |
| 0x001D | HostCommand | C | kick slot, ban slot for minutes, mute slot, restart, change mode (checked against the sender's role) |
| 0x001E | StreamingStall | C | `stalled` |
| 0x001F | ResultRequest | C | ask for the last result record again (after a reconnect) |
| 0x0020 | ResetRequest | C | `tick`; at most one per 10 s; the server validates and answers with a `Respawn` event |

`phase` is the session machine of ADR 0009 section 6: 0 lobby, 1 loading, 2 wait loads, 3 intro, 4 countdown, 5 running, 6 results.

## 7.3 Settings struct

`name` (1 to 32 characters), `max_players` (2..16), `max_spectators` (0..8), `mode`, `track`, `laps` (0..10), `traffic_density`
(0..100), `cop_level_cap` (0..5), `pursuits`, `collision` (full, ghost, ghost-start), `catch_up`, `nitrous`, `damage` (visual,
off), `allowed_cars` (any, a list of at most 64 car types, a class, or a performance cap), `join_in_progress` (closed, free roam,
between races), `password_required`, `public`, `required_mods` (a mod manifest), `sim_build_match`. At most 1 KiB encoded
(`security-gameplay.md` 1.2); a decoder refuses a larger or out-of-range struct.

## 7.4 Loadout

`car_type` (hash id), `parts` (at most 160 pairs of slot u16 and part id u32), `paint` (id and at most 8 colour indices),
`wheel_set` (id), `plate` (0 to 8 filtered characters). Never numbers that tune the car: the server rebuilds the vehicle parameters
from these ids with its own game data (`security-gameplay.md` section 3, layer L2).

The server checks a loadout in this order through a `Catalog` its game data implements: known car (1 `UnknownCar`), the session's
car list (6 `CarNotAllowed`) or class (7 `ClassNotAllowed`), each part once per slot (3 `DuplicateSlot`) and fitting the car (2
`UnknownPart`), known paint (4 `UnknownPaint`), known wheels (5 `UnknownWheels`), then the performance cap (8
`OverPerformanceCap`, also when the loadout cannot be rated). The number is the `detail` of `JoinRejected(LoadoutRejected)`.

## 7.5 Modes

`0` free roam, `1` circuit, `2` sprint, `3` drag, `4` speedtrap, `5` knockout, `6` pursuit co-op, `7` cops vs racers, `8` tag, `9` to
`127` reserved (a decode error), `128` to `255` custom (a mod mode id). One raw byte, as in the discovery reply.

## 7.6 Gameplay events (channel `events`, server to client)

Every event starts with the head `seq` (u32, per connection), `tick` (when it happened), `target_tick` (0 if none, else the tick at
which a client must apply it). Events are idempotent; a client ignores a `seq` it has applied. Bodies leave out a `tick` or
`target_tick` the head already carries (`RaceStart`, `Checkpoint`, `PhaseChange`).

| Type | Name | Body |
|---|---|---|
| 0x1100 | RaceStart | `order` (at most 32 entity ids); starts at the head's `target_tick` |
| 0x1101 | Checkpoint | `entity_id`, `index` |
| 0x1102 | LapComplete | `entity_id`, `lap`, `time_ms` |
| 0x1103 | RaceFinish | `entity_id`, `place`, `time_ms` |
| 0x1104 | RaceResults | at most 32 rows: slot, entity id, place, time, best lap, flags (bit 0 did not finish, bit 1 busted) |
| 0x1110 | PursuitStart | `pursuit_id`, `targets` (at most 32), `heat` |
| 0x1111 | PursuitEnd | `pursuit_id`, `reason` (busted, evaded, abandoned), `entity_id` |
| 0x1112 | HeatChange | `entity_id`, `level` |
| 0x1113 | Infraction | `entity_id`, `kind` (the game's infraction id), `fine` |
| 0x1114 | BustedProgress | `entity_id`, `fraction` (0..255 for 0..1, on a change of step) |
| 0x1115 | RoadblockEvent | `id`, `state` (forming, standing, dodged, destroyed), `position` (3 × f32), `yaw` |
| 0x1116 | SpikeEvent | `id`, `state`, `wheel` (optional, 0..3) |
| 0x1120 | Collision | `a`, `b` (an entity id, or none for the world), `impulse`, `point`, `normal` (f32 for now) |
| 0x1121 | Damage | `entity_id`, `zone`, `severity`, `glazed` |
| 0x1122 | Puncture | `entity_id`, `wheel` (0..3) |
| 0x1123 | Repair | `entity_id` |
| 0x1130 | Respawn | `entity_id`, `position`, `yaw`, `cause` (reset, busted, out of world, admin, join in progress) |
| 0x1131 | PhaseChange | `phase`; applies at the head's `target_tick` |
| 0x1132 | EntityRemoved | `entity_id`, `reason` (despawned, out of range, left, destroyed) |
| 0x1140 | PlayerJoined | `slot`, `player_id`, `name` |
| 0x1141 | PlayerLeft | `slot`, `reason` (quit, timeout, kicked, banned) |
| 0x1142 | RoleChanged | `slot`, `role` |
| 0x1150 | ClockEvent | `time_of_day` (minutes, under 1,440), `rate` (game minutes per real second) |

## 7.7 Chat (channel `chat`)

| Type | Name | Dir | Fields |
|---|---|---|---|
| 0x0200 | ChatSend | C | `channel` (all, team, whisper), `to_slot` (set exactly for a whisper), `text` |
| 0x0201 | ChatMessage | S | `from_slot`, `channel`, `text`, `tick` |
| 0x0202 | SystemNotice | S | `code` (join, leave, kick, mute, vote, result), `args` (at most 4 numbers) |
| 0x0203 | QuickMessage | C | `id` (a preset line, MP-44) |

`text` is 1 to 200 characters; chat is limited to one message per second with a burst of 3 (the rate is the lobby's job).

## 7.8 Bulk (channel `bulk`)

| Type | Name | Fields |
|---|---|---|
| 0x0300 | BulkStart | `kind` (session data, mod manifest, replay chunk), `id`, `length` (at most 4 MiB), `hash` (SHA-256) |
| 0x0301 | BulkChunk | `id`, `offset`, `bytes` (at most 1 KiB, ending inside 4 MiB) |
| 0x0302 | BulkEnd | `id` |

The receiver verifies `length` and the hash before use. Only data, never code, and only what the client consented to
(`security-gameplay.md` section 7).

## Not built yet

- A running session that carries these messages. The rules that produce and check them are built in the session modules of
  `nfsmw-net` (`lobby`, `join`, `vote`, `chat`, `drop`, `spectate`): bulk pacing and reassembly, chat rate limits, the lobby and
  vote rules.
- A server-to-client quick message (MP-44) and a lobby message that tells a player why its car was refused (MP-31); the lobby
  returns the reason to the server crate, which has no message to carry it yet.
- NFKC normalisation and grapheme counting of text.
- Quantised collision and respawn vectors (they are `f32` today; events are rare enough that the bytes do not matter yet).
