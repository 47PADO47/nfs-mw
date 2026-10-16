# Telemetry: what carries over to multiplayer

Milestones 10 (LAN multiplayer) and 11 (online multiplayer) are not started. This file lists the parts of the telemetry
design that should carry over, the parts that must not, and the few decisions to take now because they are cheap now
and expensive later. Companion of [features](features.md) (F-80 to F-83). Design, not built.

## 1. Two protocols, not one

Telemetry is an **observer** protocol: it reads game state and never changes the simulation. Multiplayer needs a
**game-sync** protocol: every client runs the simulation and must agree with the others. They differ in what is
safe, so they stay separate.

| | Observer (telemetry) | Game sync (multiplayer) |
|---|---|---|
| Direction | server to readers | every peer to every peer (or to a host) |
| Writes | none by default; the control table is single-player only | the whole point |
| Loss | acceptable: the next snapshot replaces the lost one | not acceptable for events; snapshots are replaced, inputs are not |
| Format | JSON over localhost (this design) | fixed binary layout, compact, over UDP with sequence numbers |
| Timing | sim tick for the reader's convenience | sim tick is the clock everyone must share |

Reuse the observer protocol for spectating, dashboards and debugging a netcode session. Do not reuse its JSON
transport for live play.

## 2. Reusable as is

| Part | Where it lives | Why it carries over |
|---|---|---|
| Triple buffer and event ring | `libs/` (generic) | the hand-off pattern for any state that a network thread reads while the sim writes |
| Budget discipline | [transport and privacy](transport-and-privacy.md) section 3 | the same 0.2 ms rule for the state a netcode layer copies per tick |
| Recording and replay (F-30, F-33) | `libs/` writer, `crates/nfsmw` source | the best netcode debugging tool: a recording of both peers' states at the same tick shows where they diverged |
| Channel catalogue with schema numbers | [channels](channels.md), `GET /v1/channels` | the same naming and unit rules for the remote-car state |
| Token and origin check (F-82) | [transport and privacy](transport-and-privacy.md) section 4 | LAN play needs an authenticated join and the same reasoning about browser origins |
| Event records with `seq`, `t` and `tick` | section 2.4 of transport | ordered reliable events (a gear change on one peer, a pursuit start) use the same record |

## 3. Reusable with changes

### 3.1 Remote car state

The vehicle channels already name the state a remote car needs. A remote-car record is a subset, with fixed-point
packing added for the wire:

- position (m), orientation (quaternion), linear and angular velocity, from `veh.position`, `veh.orientation`,
  `veh.linear_velocity`, `veh.angular_velocity`;
- wheel contact (`wheel.<i>.on_ground`, `wheel.<i>.compression`) and steer angle, for the look of the wheels;
- gear, shifting and nitrous burning, for sound and effects;
- driver inputs (`veh.controls`) so that a peer can predict the car between packets.

It is F-81 (P1). The fields are the same names, so a recording of a remote car is readable by the same tools.

### 3.2 Snapshot and tick

The sim is a fixed 60 Hz step (`FIXED_STEP`). Multiplayer will need every state tagged with the tick it belongs to, and
each peer must be able to compare two states at one tick. The telemetry snapshot carries `tick` from the first
version (F-80), so the tick counter is already in the schema when it is needed. No other change is needed now.

### 3.3 Events

Events already have `seq`, `t` and `tick`. Multiplayer events need one more rule: an event that changes shared state
(a pursuit start, a car reset) must be applied at a tick both peers agree on, not at the frame it was raised. So the
event record gets a target tick when it is used for sync. This is a field to add later, not now.

## 4. Must not be reused

- **The control table.** A remote client must not run `clock.set`, `car.reset` or `traffic.set` on another peer's
  game. Multiplayer needs its own command path with peer agreement, not the single-player console table.
- **The write token.** It stops a local web page from changing your game. It does not identify a remote player, so
  online play needs real identity and must not reuse the per-launch token.
- **Localhost-only binding and origin rules** as the sole protection. A LAN peer is a remote host by definition.
- **Strings in the stream.** Multiplayer sync sends ids and numbers. Names and chat are separate reliable messages.

## 5. The original lobby protocol

[The LAN server launcher](../lan-server.md) describes how the original `server.dll` is patched and started. Its
25-byte datagram layout is still unverified (the spec's own status line). This telemetry design does not speak that
protocol, and nothing here assumes it. A capture of a real session would settle the open layout before
milestone 10 picks a wire format; that decision is separate from this file.

## 6. Decisions to take now, while they are cheap

1. Add `schema`, `tick` and `seq` to every snapshot and event (F-80). Done in the design; to build in P0.
2. Give every car and every cop a stable entity id in the telemetry catalogue (not the ECS handle, which changes between
   runs). Today the channels name wheels by index and the car by its car type. A session id and an entity id are
   needed for remote cars.
3. Keep all times in sim seconds, with wall-clock time only in headers and status. Sync depends on sim time.
4. Keep the recorder's first line as a header with the schema and game build. Two peers' recordings can then be
   compared without guessing the version.
5. Keep the library split: the generic transport in `libs/` (Bevy-free), the game's state filling in `crates/nfsmw`.
   A netcode crate in `libs/` must not name NFS:MW either, and the game-side code calls into it.

## 7. Open questions

1. Recording inputs or states for replay (see [features](features.md), open question 4). For netcode debugging the
   answer is both: inputs at the tick for deterministic replay, states at the tick for diffing peers.
2. Does the sim stay deterministic across machines? `blackbox-vehicle` is documented as deterministic and fixed-step
   (its crate docs). Whether the whole world (AI, traffic, props) is deterministic is not known. Lockstep needs it;
   snapshot interpolation does not. Decide before milestone 10 chooses the sync model.
3. Whether the observer JSON can be a debugging view of the binary sync stream, decoded by the server, instead of a
   separate encoder. Proposal: yes, once the binary layout exists.
