# Multiplayer: status of the first LAN build

Written 2026-10-22 at the end of the milestone 10 build ([ADR 0011](../../decisions/0011-lan-multiplayer-on-the-player-world.md)).
This page lists what exists, what was verified and how, and everything that is not done. The per-feature rows are in
[features.md](features.md); this page is the summary and the open list.

## Built

| Layer | Where | Detail |
|---|---|---|
| Transport, security, hand-off, net thread, discovery | `libs/blackbox-net-core` (feature `netcode`) | [protocol-handshake.md](protocol-handshake.md), [security.md](security.md) section 8 |
| Wire protocol, rules, client logic | `crates/nfsmw-net` | [protocol-messages.md](protocol-messages.md), [modes.md](modes.md), [client-session.md](client-session.md) |
| World seam, toy world, runner, state hash | `crates/nfsmw-sim` | [integration.md](integration.md) section 4 |
| Server loop, TOML config, bots | `crates/nfsmw-server` | [server.md](server.md) |
| The real world as a server world | `crates/nfsmw` `scenes/world/net/` | [world-host.md](world-host.md) |
| Client mode, `NetSession`, CLI | `crates/nfsmw` `net/`, `scenes/world/net_client/` | [game-wiring.md](game-wiring.md) |
| LAN menu, lobby, HUD pieces, console, settings | `crates/nfsmw` `frontend/lan/` and friends | [frontend.md](frontend.md) |

## Verified

- Unit and seeded-garbage tests for every parser reachable before authentication, and for every rule crate-side.
- The handshake over a lossy, reordering link, and the same sealed connection over real loopback sockets.
- 8 and 16 simulated clients against the server on a fake clock: bytes per client per second are in [server.md](server.md) and
  stay under the 32 KB/s cap.
- Client prediction against the toy world at 2 % and 5 % loss and 20 to 150 ms round trip (figures in
  [sync-model.md](sync-model.md) section 10).
- The real world, installed: two hosts from one seed stay identical, 16 players join and leave, and one server, one client-mode
  scene and one bot run end to end over loopback ([game-wiring.md](game-wiring.md)).
- Single player: the frame-rate and world traces and the full `nfsmw` suite are green.

## Not done

Open work of this build, in rough order of importance:

1. **Two PCs by hand.** Nothing has been played across a real network. The loopback end-to-end run is the only live check.
2. **Room passwords: built, not played across two PCs.** The server runs the SPAKE2 exchange ([server](server.md) section 3),
   both ends bind it to the Noise handshake hash (`Connected` carries it), and the Host screen's password reaches the server.
   Tested with the real handshake, a wrong password, no answer and the host's own in-memory client. A listen host's own client
   binds to zeros, as the in-memory link has no handshake.
3. **Physical extraction of the world** into `nfsmw-sim`. The server runs the game's own `WorldScene` behind a renderer that draws
   nothing, so it links the renderer crates, the host is not `Send` and the listen host runs two worlds (the server's and its own
   client's).
4. **Remote cars are stock.** A join's parts are not applied, and remote cars have no engine sound or detail. The helicopter,
   spike strips and moving props are not drawn on a client.
5. **Races with remote players: built on the server, not played.** The server asks the world for the session's race, every remote
   player is a person on the grid driven by their seat's pedals and exported as their own car, and knockouts, speed-trap passes,
   tag passes and penalties are wire messages ([world host](world-host.md) section 7, [modes.md](modes.md)). The people's cars have
   own-car blocks, and the world's GO falls on the server's start tick (the load barrier waits for the staged grid). A client predicts its
   race car as well as a free-roam one (median correction 1.3 cm), and a session's race ends the moment it is over, also on a
   dedicated server, whose idle lead car is a computer racer on the grid and not one the race waits for.
6. **Global block.** Pursuit and race fields are filled from the world. The client sets its day clock to the server's time of day when it strays more than 5 minutes from it (`net_client/hud.rs`); weather is not carried.
7. **Server details.** A server started without a config file gets a new identity key every start (with one it keeps `server.key` beside it), and a peer that changes address moves
   only after the new address answers a path challenge. Quick messages are relayed to everyone as `QuickRelay`.
8. **Front end.** The post-race page of the original is not wired to a session's end (the LAN results page shows instead), and
   there is no team chat from the menu. F7 votes in a race, the player menu whispers and the Home page saves `net_name`. The LAN screens and the session HUD are drawn with the game's UI toolkit in the Original and Remastered styles (no FEng package exists for them).
9. **Text rules.** Names, chat, plates and room passwords are in NFKC (`unicode-normalization`; the wire refuses text that is not). A joiner whose name is the same as a member's once confusable look-alikes and case are folded (`unicode-security` skeletons, `lobby/names.rs`) gets a number after it. Grapheme counting is not done (the `char` count is the stricter bound).
10. **Fuzzing.** The cargo-fuzz targets type-check but have never been run under libFuzzer; the in-tree seeded tests are what
    runs in CI.
11. **Sockets.** Sockets ask for 256 KiB buffers and the discovery port is shared (`socket2`), so two servers on one PC both answer
    discovery. Broadcast itself and IPv6 are untested (the tests use 127.0.0.1).
12. **Tuning.** Several rule numbers are marked **[ours]** in [modes.md](modes.md) and the 10 Hz spectator rate, the start
    countdown and similar values are first guesses to be set by play.
13. **World timing.** The 16-player world step has a mean of 2.3 ms and a p95 of 2.5 ms in the dev profile at 59 traffic cars
    ([bandwidth.md](bandwidth.md) section 5); the first measurement's p95 of 4.35 ms came from threads started every tick and a
    race list built every tick, both fixed. The machine is noisy: a run now and then is several times slower for its whole
    length, so a figure needs two runs.
14. **Determinism leftovers.** Model choice from the full catalogue and strict id order for car pairs ([determinism.md](determinism.md)).

## Not planned for milestone 10

Listed in [ADR 0011](../../decisions/0011-lan-multiplayer-on-the-player-world.md) section 5: server list and invite codes,
region display, accounts and persistent stats (milestone 11), voice chat, host migration, handicaps, last-standing by damage,
the in-game replay viewer, mod compatibility beyond an empty list, the telemetry bridge, drag races and human cops.
