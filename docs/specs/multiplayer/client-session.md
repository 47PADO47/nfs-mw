# Multiplayer client session: API and lifecycle

Status: **built** (2026-10-22) in `crates/nfsmw-net`, module `client_session`, and tested on a mock link against a
scripted server and over loopback UDP against a scripted `ServerEndpoint`. Not yet driven by the game (`NetSession` and
`NetWorldScene` of [integration](integration.md) section 2) nor tested against the real server loop of `nfsmw-server`.
Where this file and the code disagree, the code's tests are the reference and this file is fixed.

`ClientSession` is everything between the net thread and the game: admission, the lobby mirror, snapshots into
prediction and interpolation, the clock, input, events, reconnect and spectating. It reads no clock and never blocks;
the caller passes its monotonic milliseconds. It reuses the built pieces: `ClientEndpoint` (`blackbox-net-core`,
feature `netcode`), `message`, `frames`, `schema`, `input`, `predict`, `reconcile`, `interp`, `clock`, `netstats`,
`drop` and `spectate` of `nfsmw-net`.

## 1. Types

| Type | Role |
|---|---|
| `ClientSession<L: ClientLink, C: PredictedCar>` | The driver. `L` is the connection, `C` the owner's car (the game's vehicle, or `ToyCar`) |
| `ClientLink` | `poll_event`, `send_reliable(channel, bytes)`, `send_unreliable(frame_type, bytes, tag)`, `disconnect(reason)`, and `handshake_hash` (default `None`). Implemented for `ClientEndpoint` |
| `MockLink`, `MockPeer`, `mock_link()` | An in-process link for tests: the session holds the `MockLink`, the test plays the server through the `MockPeer` (push `ClientEvent`s, take what was sent as `Sent`) |
| `SessionConfig` | Name, loadout, mods, protocol minor, sim build, data id, capability bits, password, tuning, resume window (30 s), reconnect interval (1 s), chat lines kept (64). `SessionConfig::spectator()` sets the spectate-only bit |
| `Status` | `Connecting`, `Admitting`, `Joined`, `Reconnecting { since_ms, deadline_ms }`, `Ended(EndReason)` |
| `EndReason` | `Left`, `Rejected(JoinRejected)`, `Kicked(Kick)`, `Link(DisconnectReason)`, `ResumeExpired`, `PasswordRequired`, `PasswordRejected`, `SchemaMismatch`, `Protocol` |
| `SessionEvent` | `Admitted { slot, entity, resumed }`, `ConnectionLost`, `Ended`, `LobbyChanged`, `SessionData`, `Chat`, `Notice`, `Countdown`, `CountdownCancelled`, `LoadWorld`, `StartAt`, `Vote`, `Game(Message)`, `Results`, `Snapped(SnapReason)` |
| `LobbyView` | The lobby mirror (section 4) |
| `GlobalView` | The global block of the newest snapshot (`frames::GlobalBlock`): phase, heat, busted and cooldown meters, pursuit status, place, lap, checkpoint, time of day |
| `SessionCounters` | Bad frames, bad messages, missing baselines, bad snapshots, snapshots decoded, reconnects, INPUT frames sent, snapshot parts dropped or refused, entity records refused |
| `Connector<L>` | `Box<dyn FnMut() -> Option<L> + Send>`: makes a new link to the same server for a reconnect |

## 2. The frame

```rust
session.update(now_ms, dt_ms);                 // drain the link, clock, pings, reconnect, held events, smoothing
for event in session.take_events() { .. }      // admission, lobby, chat, gameplay, results, snaps, end
if let Some(ticks) = session.due_ticks() {     // floor(S_est + lead), at most 6 per frame
    for tick in ticks { session.submit_input(tick, &input_for(tick)); }
}                                              // or: session.drive(|tick| input_for(tick))
let pose = session.own_pose();                 // physics plus the smoothed correction
session.remote_entities(&mut display);         // every other entity at the display time
let global = session.global();                 // HUD: phase, pursuit, race, time of day
let line = session.net_line();                 // the `net` overlay line (MP-82)
```

`update` first sends any input recorded since the last INPUT frame, then drains every link event:

| Link event | What happens |
|---|---|
| `Connected` | `Auth` (build ids, capability bits, the resume token when reconnecting) and `Join` (name, loadout, mods) go out back to back |
| `Reliable` | Decoded with the size and format rules of [protocol-messages](protocol-messages.md); the channel must be the message's. Routed by section 3 |
| `Unreliable` | `SNAPSHOT` to section 5, `PONG` to the clock; anything else is counted as a bad frame |
| `Disconnected` | Section 6 |
| `Delivered`, `Lost` | Ignored: they are the server's baseline signals |
| `Stats` | Kept as `endpoint_stats()` |

Then, while seated: the clock advances (`NetClock::update`); a clock step voids the prediction
(`Reconciler::clock_step`, reported as `Snapped(ClockStep)`); a PING goes out when due (4 per second for 5 s, then 1);
gameplay events whose target tick came are released; the correction offset and the display delay age by `dt_ms`.

## 3. Admission

```
Connecting ──Connected──► Admitting ──JoinAccepted──► Joined
                │                    └─JoinRejected, Kick, password failure, bad schema──► Ended
                └─Disconnected──► Ended(Link)
```

- `Join` follows `Auth` at once. A room with a password answers with `PakeStart`; the client runs SPAKE2 as the joiner
  (`blackbox_net_core::secure::password`) bound to `ClientLink::handshake_hash` (32 zero bytes when the link does not
  report it), sends `PakeMsg` and `PakeConfirm`, and checks the server's `PakeConfirm`. No password configured ends the
  session with `PasswordRequired`; a bad tag, or a seat offered before the server confirmed, with `PasswordRejected`.
- `JoinAccepted`: the schema is rebuilt with the server's `world_half_extent` and its `schema_id` must equal
  `server_caps.schema_id` (else `SchemaMismatch`). The snapshot rate sets the interpolation and statistics interval. The
  slot, entity id, resume token and negotiated minor are kept; views, parts and the prediction are reset, the event `seq`
  restarts, and `Admitted` is reported (`resumed` when a reconnect got the same slot back).
- A `JoinAccepted` while seated, a client-to-server message from the server, an unknown type, garbage or a message on the
  wrong channel is an invalid message: counted, and the 17th inside 10 s ends the session with `Protocol`.

## 4. The lobby mirror

`LobbyView` holds what the server said; the lobby screens read it.

| Field | Fed by |
|---|---|
| `players` (sorted by slot), `settings`, `phase`, `host_slot` | `LobbyState` (replaces), `LobbyDelta` (add, update, remove), `PhaseChange`, `PlayerLeft`, `RoleChanged`, the session data |
| `my_slot`, `settings` | `JoinAccepted` |
| `countdown` | `StartCountdown`; cleared by `CancelCountdown` |
| `load` | `LoadWorld` (answer with `load_complete`) |
| `start_tick` | `StartAt`, and `RaceStart`'s target tick at once (not when it is due) |
| `chat`, `notices` | `ChatMessage`, `SystemNotice`; the newest `chat_lines` of each |
| `vote` | `VoteState` |
| `loadouts` | The `SessionData` bulk transfer of a late join (MP-53), reassembled by `drop::BulkReceiver` with its 64 KiB limit and SHA-256 check |
| `kicked` | The `Kick` that ended the session |

Helpers: `player(slot)`, `me()`, `my_role()`, `is_host()`, `racers()` (the slots a spectator may follow), `loadout(slot)`.

Commands, each a reliable message (`false` when there is no link): `set_ready`, `choose_loadout` (also used by a later
reconnect's `Join`), `set_role`, `say` and `whisper` (`LimitError` for bad text), `quick_message`, `vote`,
`request_reset` (`ResetRequest` at the current server tick), `load_complete`, `streaming_stall`, `update_settings`,
`host_command`, `request_results`. `leave` sends DISCONNECT `NORMAL` and ends the session with `Left`.

## 5. Snapshots, prediction and interpolation

The `SNAPSHOT` payload is [protocol](protocol.md) section 6.3 as built in `frames`. For each part:

1. `frames::read_snapshot` checks the header and decodes the blocks; a malformed part is a bad frame. Parts before the
   seat are dropped.
2. Parts wait in a table of 4 open ticks until every part of a tick is in, in any order. A duplicate or inconsistent
   part is refused; when a tick completes, older incomplete ones are dropped (counted).
3. The bodies are decoded in part order: part 0 against the baseline view the distance names (empty for 0), each later
   part against the result. A missing baseline sends `BaselineLost { tick }` (at most every 500 ms) and drops the
   snapshot; any other error drops it as a bad snapshot. The view is kept for 64 ticks.
4. The global block, if newer than the last, becomes `GlobalView`.
5. Once the clock is synced: every entity of the view except the own car goes to the interpolator at the snapshot's
   tick; entities absent from the view are removed at that tick (they vanish at display time). A snapshot older than the
   newest fed is kept only as a baseline.
6. Unless spectating, the own block goes to the car: the first places it (`set_state`) and starts prediction at
   `S_est + lead`, with the lead from the best round trip; later ones go through `Reconciler::apply` (corrections and
   snaps counted, a snap reported) and the lead follows the input report. `ack_input` is expanded against the
   snapshot's tick.

Input: `due_ticks` is the range from the predictor's next tick to `floor(S_est + lead)`, trimmed to the last 6 after a
stall. `submit_input` takes only the next due tick: it is neutral before the start tick, quantised, stepped and recorded
in the `InputHistory`; with the last due tick the INPUT frame (the newest unacknowledged ticks, up to 8) goes out. A
frozen or stalled predictor refuses the step.

## 6. Disconnect and reconnect

| Drop while | Reason | Result |
|---|---|---|
| Seated, with a resume token | `Timeout`, `Overflow`, `Remote(TIMEOUT)` | `Reconnecting` until `now + resume_window_ms`; `ConnectionLost` |
| Seated | a close by either side, a failed handshake | `Ended(Link)` |
| Connecting or admitting | any | `Ended(Link)` |
| Reconnecting | any | stays `Reconnecting`; the next attempt follows |

While reconnecting, the session calls its `Connector` once per `reconnect_interval_ms` (or waits for `resume_with`). On
`Connected` it sends `Auth` with the token and `Join`. The new `JoinAccepted` brings a new token; the same slot is a
resume (the server sends a full snapshot), another slot a fresh seat. The clock estimate is kept across the drop.
Past the deadline the session ends with `ResumeExpired`. A `Kick` or `JoinRejected` ends it at once.

## 7. Spectating

A session spectates when it joined with the spectate-only bit or the roster gives it the spectator role. It predicts
nothing, sends no input, and `due_ticks` is `None`. `follow(Camera)`, `follow_next`, `follow_previous` and `camera()` use
`spectate::FollowCam` over `LobbyView::racers()`; a followed racer who leaves or becomes a spectator hands the camera to
the next racer. `follow_target()` is the entity of the followed racer (the leader camera takes the first racer in slot
order: the client does not know other players' race order) found by the owner field of the player car records, with
players seated in the world under their slot as player id.

## 8. Scripted bots (`nfsmw-server`, module `bot`)

| Type | Role |
|---|---|
| `BotConfig` | Index (name `bot<n>`), seed, car type (`index % 4`), data id and sim build; builds the `SessionConfig` and the toy car (mirroring `ToyWorld`'s player car: power `1 + (type % 4) × 0.05`, top speed 60 m/s, bounds 500 m) |
| `Bot<L = ClientEndpoint>` | `Bot::connect(addr, config)` over UDP with a reconnecting connector, `Bot::spawn(transport, addr, config)` over any transport (the lossy simulator), `Bot::over(link, config)` over any link. `step(now_ms)`: update, ready on `Admitted`, `load_complete` on `LoadWorld`, `drive` with a `WanderScript` keyed by the slot. `report()` |
| `BotFleet<L>` | Bots stepped together on the caller's clock (fake time in tests): `step`, `seated`, `leave`, `reports` |
| `ThreadedFleet` | One thread per bot on the real clock, 16 ms frames; `stop()` makes each leave and returns its `BotReport` |
| `BotReport` | Joined, seated, snapshots, INPUT frames, corrections, snaps, end reason |
| `endpoint_config()` | A LAN client endpoint for protocol major 1 with a fresh identity key |

## 9. Tests

- `client_session/tests` (mock link, scripted server): admission (Auth then Join, seat, rejection, kick, schema
  mismatch, drop before the seat, password right, wrong, missing and unconfirmed); lobby (state and deltas, countdown,
  cancel, load, start, vote, chat, notices, chat cap, session data in 100-byte chunks, every command, leave); events
  (duplicate and old `seq`, target tick hold, race start tick, results, leave and role change); snapshots (a 1 s stream
  of deltas into prediction and interpolation, removal at display time, parts out of order then a delta against the
  merged view, lost baseline reported once per 500 ms, an older snapshot, garbage bodies, snapshots before the seat);
  input (redundant frames, car moves, neutral before the start tick, only the next due tick, nothing before the first
  own block); reconnect (resume with the token, retries until the window closes, a server close is final, `resume_with`
  and a fresh seat); spectating (no input, follow camera, a followed racer leaving); garbage (random frames and
  messages, the invalid-message limit, a second seat).
- `tests/session_endpoint.rs`: a session on a real `ClientEndpoint` joins a scripted `ServerEndpoint` over loopback UDP
  (Noise handshake, sealed packets), syncs, places and drives its car, draws a remote car; the server decodes its INPUT
  frames.
- `frames/tests.rs`: PING, PONG, global block and snapshot header round trips and refusals, random bytes.
- `nfsmw-server` `bot::tests`: a bot presents itself on connect. **Waiting for the server loop:**
  `bots_join_the_real_server` is `#[ignore]`d until `lan/server` provides the loop to start in the test.

## 10. Not built

- The Bevy side: the `NetSession` resource, `NetWorldScene`, lobby screens, console commands, settings keys and the
  overlay drawing.
- `ClientEndpoint` does not report the Noise handshake hash, so the password is bound to zeros over it (a man in the
  middle who ran two handshakes is not detected yet); the endpoint must expose the hash.
- Near-field extrapolation (sync-model 4.3), wheel and engine detail of remote cars (the session passes `EntityView`s),
  the client's own input-timing use of `StreamingStall`, spectator hints, discovery in the session (the game runs
  `DiscoveryScan` and passes the address).
- The real server loop: no test runs a session against `nfsmw-server` yet.
