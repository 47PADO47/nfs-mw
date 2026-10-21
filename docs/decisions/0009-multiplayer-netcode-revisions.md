# 0009 — Multiplayer netcode revisions: wider ids, shared delta encoding, explicit layers

- **Status:** accepted (2026-10-21). Amends [ADR 0006](0006-multiplayer-netcode.md). Where they disagree, this one wins. The
  generic layer is built in `libs/blackbox-net-core` (milestone 10, step 1); everything above it is still design.
- **Milestone:** 10 (LAN) and 11 (online).
- **Inputs:** a review of ADR 0006 and `docs/specs/multiplayer/`, and a Ghidra pass over the original game's race netcode
  ([original race netcode](../specs/original-race-netcode.md)).

## Question

ADR 0006 settles the big choices: one server owns the world, the owner predicts its own car, other entities are
interpolated, snapshots are delta-coded against an acknowledged baseline, and everything runs on our own UDP layer. A review
before writing code found limits that would be costly to change later:

- Entity ids are 8 bits, with the first 32 kept for player slots.
- The server encodes every snapshot once per client.
- The listen-server host's own client connects over loopback UDP and runs the full handshake.
- Entity kinds, field masks and message tables are closed sets.
- Unreliable frames cannot be skipped without knowing their layout.
- The session phases (load barrier, intro, countdown) are spread across messages instead of being one state machine.

What changes before the first line of netcode is written?

## Decision

Seven changes. ADR 0006's topology, authority model, tick rates, prediction and interpolation stay as they are.

### 1. Entity ids are 16 bits, gap-coded on the wire

The server's id space is `u16`. A snapshot lists entities in id order and writes each id as the gap from the previous one:

- The first id is a varint with 7 data bits per group.
- Each later id is `id - prev - 1` as a varint with 3 data bits per group, so a gap of 1 costs 4 bits and a gap under 64
  costs 8.

This is cheaper than ADR 0006's 8 bits per id on any snapshot where entities are near each other in the id space. The
server hands out ids from a free list that prefers low numbers. It also removes the 255-entity cap: 16 players, 40 traffic
cars near each, police and moving props pass 255 on the server easily. Player slots are no longer a reserved range; a
player's car has its own entity id like everything else, and the slot number lives in the session state.

Telemetry's `u32` entity field becomes the 16-bit wire id plus a 16-bit generation (it was 8 + 24). This amends
ADR 0006 section 7 and the telemetry multiplayer-reuse note.

### 2. Quantise once, encode once per baseline, assemble per client

ADR 0006 keeps 64 world states and builds a delta for each client against that client's baseline. Clients usually
acknowledge the same snapshot, so the same bytes get computed again and again. The new pipeline:

1. **History.** Each tick the server writes every entity's quantised fields once into a ring of 64 frames. A record is the
   entity id, its kind and a slice of `u32` field values. The game quantises (`bits::Quant`); the generic layer compares
   integers.
2. **Per-client view.** For each snapshot it sends, the server records only `(id, source_tick)` pairs per client: which
   entities the client holds, and from which server frame each one was last updated. It does not copy the client's values,
   because they equal `history[source_tick][id]`.
3. **Encode cache.** A delta body (field mask plus changed fields) depends only on `(id, source_tick, tick)`, not on the
   client. The first client that needs one encodes it into a per-tick bit cache; every other client with the same source
   tick copies the bits. Full records are cached per `(id, tick)`.
4. **Assembly.** Per client: choose entities by priority (section 5 of ADR 0006, unchanged), then write the id gaps, the
   2-bit operations and the cached bodies.

Per-tick work drops from O(clients × entities × fields) to about O(entities × distinct source ticks × fields), plus a bit
copy per client. On LAN, with one shared source tick, that is one encode per entity. A field can be marked
*difference-coded*: its delta is the zigzag difference to the baseline as a varint with 4 data bits per group. This suits
positions, which move a few steps per snapshot. A full record always writes raw values. The encoder sends a full record
whenever that is smaller than the delta.

The priority accumulator now composes with deltas. An entity that is not updated in a snapshot keeps its old source tick in
the client's view instead of being dropped. If its source frame leaves the 64-frame history, it is forced into the next
snapshot as a full record.

### 3. Explicit layers; the host's own client is in memory

| Layer | Job | Module |
|---|---|---|
| Link | Non-blocking datagrams: UDP, an in-memory network, a seeded lossy simulator | `link` |
| Connection | Sans-IO: packet numbers, ack and ack bits, RTT and loss, 4 reliable ordered channels, fragments, delivery and loss events. The future seal (Noise, ChaCha20-Poly1305) wraps its output; 16 bytes of every datagram are reserved for the tag now | `conn` |
| Replication | Schema, history, delta codec, encode cache, priority, spatial hash | `replicate` |
| Clock, session, discovery | Tick estimate and slew; session state machine; LAN discovery datagrams | `clock`, `session`, `discovery` |

The connection never reads a clock or a socket; the caller passes `now_ms` and moves the bytes. This keeps it testable
under a deterministic packet simulator, and lets a QUIC or `renet` transport replace it without touching the layers above.

The **listen-server host's own client uses the in-memory link**, not loopback UDP. It goes through the same connection and
replication code as a remote client, but there is no socket and no handshake, because nothing outside the process can
reach it. This replaces ADR 0006 section 1's "over loopback like any client, handshake included".

### 4. Open registries and skippable frames

- **Frames.** Every frame except one-byte padding is `type u8, length varint, payload`. Any receiver can skip any frame,
  so the known-frame and skippable-frame split of protocol.md section 5.2 goes away, and so does the length field inside
  `RELIABLE` frames. The cost is 1 or 2 bytes per frame, with 2 or 3 frames in a packet.
- **Replication schema.** The game registers entity kinds (up to 64) and their fields. Each field has a bit width and a
  difference-coded flag. The field mask has one bit per field, so it is as wide as the kind's field count. `schema_id` is a
  64-bit FNV-1a hash of the descriptor bytes and is checked at join. Modes and mods add kinds without codec changes.
- **Reliable message types** are `u16` in namespaces: core `0x0000–0x0FFF`, game `0x1000–0x7FFF`, mods `0x8000–0xFFFF`. A
  receiver skips an unknown type by the message length and counts it.

### 5. No allocation on the per-tick path

- Datagrams are built in caller-owned buffers sized to the MTU (1,200 bytes).
- The bit writer and reader work on borrowed slices.
- History frames, per-client views, the encode cache and the event queues are allocated once and reused.
- A counting-allocator test checks that the unreliable path (build packet, read packet, history write, snapshot encode and
  decode) makes no allocation per tick after warm-up.

### 6. A generic session state machine

Shaped like the original game's host machine (`WAITJOINS`, `LOADING`, `WAITLOADS`, `NIS`, `READY`, `RACING`):

- **Session:** `Lobby` → `Loading` → `WaitLoads` → `Intro` → `Countdown { start_tick }` → `Running` → `Results` → `Lobby`.
- **Peer:** `Lobby`, `Loading`, `Loaded`, `InGame`, `Left`.

`WaitLoads` is a barrier with a timeout (default 120 s, the original's host value). A peer still loading at the deadline is
dropped instead of holding everyone back. The countdown starts at a server tick, so every client begins on the same tick.
Joining is allowed in `Lobby`, and in `Running` as join-in-progress; it is refused in between. The game maps phases to
screens and messages.

The original's `DATACRC` and `CARDESCRIPTION` messages are already covered: our `data_id` check at join and the loadout
message.

### 7. The `renet` spike is dropped

ADR 0006 owner decision 3 asked for a one-week `renet` spike before building our own layer. The connection layer is now
built and tested here, under loss, duplication and reordering, in less code than the spike would have evaluated. The revisit
trigger of ADR 0006 still applies: if the layer turns out costly to maintain, `renet` can replace it behind the same
interface.

## What the original game told us

From the [Ghidra pass](../provenance/original-race-netcode.md), as background, not as something to copy:

- Each machine broadcast its own car (`MSG_U_BI_CARSPAM`), which is ADR 0006's option C. That fits the original's
  4-player, no-traffic online races. It does not fit 16 players in a shared city with police, so authority option 1 stands.
- The host waited twice (joins, then loads) before a shared intro and start. Our section 6 makes that explicit.
- Clients slewed their clock offset a few milliseconds per frame and stepped it beyond 100 ms. ADR 0006 section 11 already
  works this way.
- A timing check with per-reason strike counters was the client-side cheat defence. A server-authoritative world does not
  need it; the per-reason counter is kept as a server log idea.

## Consequences

- **Bandwidth.** Ids cost less on average, difference-coded positions roughly halve the position bits of moving entities,
  and frame lengths add 1 or 2 bytes per frame. [bandwidth.md](../specs/multiplayer/bandwidth.md) shows the new figures and
  the measured ones from the `net_bench` example.
- **Server CPU.** Encoding scales with distinct baselines, not clients. Assembly per client is a bit copy.
- **Memory.** History: 64 frames × entities × fields × 4 bytes. That is about 1 MB for 400 entities of 10 fields. Per-client
  views store 6 bytes per entity per frame.
- **Protocol docs.** protocol.md sections 5.2, 6.3 and 9 and ADR 0006 sections 1 and 7 are superseded where they conflict.
  Their text is annotated, not rewritten.
- **Telemetry.** The entity field split changes to 16 + 16 bits.
- **Not changed:** the 60 Hz tick, snapshot and input rates, prediction and reconciliation, interpolation, server-only
  collisions, the handshake and security design (still to be built, with `snow`), and the order of work after step 1.

## Owner decisions

Settled by approving this ADR's plan on 2026-10-21: changes 1 to 7. ADR 0006 owner decisions 1, 2 and 4 to 7 are still open.
