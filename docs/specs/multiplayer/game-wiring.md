# The game's side of a LAN session: `NetSession`, the servers and the client world

Status: built 2026-10-22 on `lan/game-wiring` ([ADR 0011](../../decisions/0011-lan-multiplayer-on-the-player-world.md),
[integration](integration.md) section 2). How `crates/nfsmw` runs a LAN session: the command line, the threads, the session's
modes, the listen and dedicated servers over the real world, and the client mode of the world. The code is
`crates/nfsmw/src/net/` and `crates/nfsmw/src/scenes/world/net_client/`; paths below are relative to `crates/nfsmw/src/`
unless they start with `crates/` or `libs/`. The server loop is [server](server.md), the client session
[client-session](client-session.md), the server's world [world host](world-host.md).

## 1. The feature

Everything here is behind the cargo feature `multiplayer` of `nfsmw`, on by default. It adds `nfsmw-server`, `ctrlc` and the
`netcode` feature of `blackbox-net-core`. `nfsmw-net` is in every build: the LAN screens (`frontend/lan/`) are compiled without
the feature and show "no network" through their offline fallback. `cargo check -p nfsmw --no-default-features` builds the game
without the feature (and without the Bevy renderer).

## 2. Command line

The switches are global (`cli/net_args.rs`), so they go before or after a command: `nfsmw --host`,
`nfsmw play --skip-boot --connect 10.0.0.7`.

| Switch | What it does |
|---|---|
| `--host` | Starts the game straight into free roam with a listen server on this PC and the player in it as slot 0 (the world's lead) |
| `--connect ADDR` | Starts the game straight into free roam as a client of the server at `ADDR` (`HOST`, `HOST:PORT`, an IP; the default port is 9750; host names are resolved, IPv4 first) |
| `--dedicated` | Runs the server loop over the real world with no window, audio or GPU (`NullBackend`) on the main thread until Ctrl-C |
| `--discover` | Scans the LAN for 2 s and prints one line per server (address, name, players, mode, in progress), then quits |
| `--port N` | The game port of `--host` and `--dedicated`, and the port `--discover` asks about (discovery answers on `N + 1`) |
| `--name S` | The server name of `--host` and `--dedicated` (1 to 32 characters) |
| `--server-config FILE` | A server config file ([server](server.md) section 5) for `--host` and `--dedicated`; `--port` and `--name` win over it |
| `--seconds N` | Stops a `--dedicated` server after `N` seconds |

`--host`, `--connect`, `--dedicated` and `--discover` exclude each other. `--host` and `--connect` say ready on the seat by
themselves, so a server with `auto_start` (the default) starts free roam as soon as everyone is in. The parsing is unit-tested
(`cli/net_tests.rs`).

A dedicated run does not install the game's logging (`main.rs`): it logs as `nfsmw-server` does, to stderr and the config's
`log_file`, with its `log_level`. `--discover` and `--dedicated` never build the Bevy app (`commands/net.rs`).

## 3. Threads and modes

```
main thread (Bevy app)                          listen server thread ("net-listen-server")
  FrameSet::Commands                              WorldSimHost::open (the world, NullBackend)
    update_session: listen start, scan, pump      nfsmw_server::bind (game port, discovery on)
    attach_client: client -> world scene          Server::new, connect_local (the host's client)
  FrameSet::SceneUpdate                           Server::run at 60 Hz until the stop flag
    WorldScene client frame (net_client/)
         ▲ │ in-memory link (host) or UDP (join)        ▲ │ UDP, net threads of blackbox-net-core
         │ ▼                                            │ ▼
```

`NetSession` (`net/session.rs`) is the game's multiplayer state. It lives in the front end's `LanBackend` resource
(`frontend/lan/handle.rs`, `NonSend`) as its `NetFrontend` (`net/frontend.rs`); `NetPlugin` (`net/plugin.rs`) puts it there and
adds two systems in `FrameSet::Commands`, which find it again through `NetFrontend::as_any_mut`.

| Mode | What runs |
|---|---|
| `Idle` | Nothing; a LAN scan (`DiscoveryScan`) may run and fills the server list |
| `Host` | `ListenServer` (`net/listen.rs`): a thread that opens the `WorldSimHost` itself (the scene is not `Send`), binds the game port with discovery, makes the `Server` and attaches the host's own client with `Server::connect_local` (the in-memory link: no socket, no handshake). Only the client endpoint crosses back, through a channel; a `TickHook` stops the loop when the game sets its flag. Dropping the `ListenServer` stops the thread and joins it |
| `Join` | A client over `ClientEndpoint::connect` with a reconnecting connector that keeps the player's identity key, so a dropped connection resumes its seat |
| `Dedicated` | `NetSession::run_dedicated` (`net/dedicated.rs`): the same `Server` over the same `WorldSimHost` on the main thread, a Ctrl-C handler that sets the stop flag, `Server::run` with `--seconds` |

The client is `NetClient` (`net/client.rs`): the `ClientSession<ClientEndpoint, GameCar>` of [client-session](client-session.md), its
clock (milliseconds since it started; tests set it), and the answers the game gives by itself: ready on `Admitted` when started
from the command line, `LoadComplete` at once on `LoadWorld` (the world runs already; streaming is the client's own business,
section 4). It keeps two copies of every session event: one for the world scene (taken each frame while attached) and one for
the screens (`NetFrontend::take_events`). The front end, the session and the world share it as `Rc<RefCell<NetClient>>`.

While no world drives it (the menus), `update_session` pumps the client (`ClientSession::update` and its events). Once a world
scene that has a car of its own takes it (`Scene::attach_net_client`, forwarded by the pause wrapper), the scene drives it.

### Who the player is

The player's name is `net_name`, a host's default port `net_port`, and the identity key the 32 secret bytes in
`net_identity_file` (`identity.key` in the config folder by default; `net/identity.rs`): made and written the first time,
read every time after, so a server knows the player across runs. A broken file is left alone and the run uses a new key. A
loadout's car id is `frontend/lan/car.rs`'s `car_type` of the folder.

### What both ends check

`net/build.rs`: the sim build is `nfsmw-<version>` on the machine's architecture; the data id is the hash of the install's car
folders (sorted, upper case; the executable is not read); loadouts are checked against `GameCatalog`, the install's cars by
their type id (`scenes/world/net/catalog.rs`: the bChunk hash of the upper-case folder name, the same hash as
`frontend/lan/car.rs`), every car class 0, parts not checked. The server setup takes these and the session seed.

### The server's world

`WorldSimHost` implements `nfsmw_server::WorldControl` (`scenes/world/net/control.rs`):

| Call | As built |
|---|---|
| `reset_player` | `WorldSimHost::request_reset`, refused for a player with no car |
| `set_ghost` | Parks the player's car (co-op parking): out of play, not stepped, not collided, not exported; unparking brings it back |
| `fill_global` | New in `nfsmw-server` for this: the world's clock (time of day in minutes), and for a chased or busted player the heat (`TrafficWorld::heat_value`), its own busted meter (the co-op chase's while two or more are chased), the cooldown bar and the chase state; for a racer the place and lap from the race card |

A session's world opens at `START_AT` (`net/options.rs`), a road with room to drive, facing 45 degrees (the placement turns the
car along the road). The determinism tests' road at (2151, 1399) has an obstacle about 10 m either way from where the car is put.

## 4. The client mode of the world

`scenes/world/net_client/`. The same `WorldScene`; single player is unchanged because nothing here runs without a client.

**One switch.** `frame_systems.rs` decides the frame's systems from the client flag, in one place: single player runs the
shared simulation (`simulation.rs`: every player's physics, the traffic, the police and their sight, the roadblocks, the
pursuit, the race tick), the races, the entry zones, the career, the catch-up prompt and the split screen; a client runs none of
them and runs its own frame (`net_client/frame.rs`) instead. The first client frame releases the world's traffic and police
and ends a co-op session; a race that was running stops where it is.

**The predicted car.** `GameCar` (`car.rs`) is the game's `PredictedCar`. The session owns its car (it clones a template when
the first own-car block places it) but a tick of the real car needs the world, so `GameCar` is a shared handle and the scene
lends it the lead's `Drive`, the collision world and the props for the length of the frame's session calls (`lend.rs`: the
`Drive` leaves the players list and the collision and props leave the residency by move, `residency/lend.rs`, and all come back
in place). Inside the lend:

| Call | Does |
|---|---|
| `step` | `Drive::predict_step`: the input through `Drive::set_remote_input` (`DriveInput::from_vehicle`, the server's own mapping), then `prepare_frame`, `step` and `finish_step`, the physics tick the server runs, with the static world and the props but no other car (sync-model.md section 4.2) |
| `save`, `restore` | `Drive::predict_save` / `predict_restore`: the vehicle (`blackbox_vehicle::Vehicle` is `Clone`), the wheel spin and contact poses, the drawn poses and the telemetry; the ground caches are cleared, the damage engine and the reset-collision window are not rewound |
| `state`, `set_state` | The reduced motion in render space (`drive/sim/predict.rs`): the model's origin and axes as `CarSim::pose` draws them, the body's velocities mapped axis by axis as the entity views map them. `set_state` writes pose and velocities into the body; the engine, gear and nitrous keep their own state |

Outside a lend a step does nothing, `state` answers the last state, and a state set meanwhile waits for the next lend; a car
that does not stand on a road yet is asked onto the road at the state's place (the first own block of a client whose world
opened elsewhere). The car is drawn at `ClientSession::own_pose` (prediction plus the decaying correction) through a drawn-pose
override (`Drive::set_net_pose`, below a NIS pose).

**The frame** (`frame.rs`): streaming first (while the player's surroundings are not resident the car is held and
`StreamingStall` goes to the server; the input is neutral); the input of the frame (the devices' through `DriveInput::from_actions`,
neutral in the fly view; shift edges and gear keys go with the first due tick only and wait for the next tick when none is due;
the reset key sends `ResetRequest`); then with the world lent `ClientSession::update`, `due_ticks` and one `submit_input` per due
tick (the session's clock is the fixed-step accumulator), the own pose, the remote entities and the global block; then the
drawn pose, the chase camera, the remote cars and the HUD lines.

**Remote entities** (`remote.rs`): other players, traffic, police and roadblock cars are drawn from
`ClientSession::remote_entities` (interpolated at the display time) with one `CarRig` per car model, loaded the first time the
model is seen (the entity's type number is its folder's index in the sorted car folders), and one render key per entity, given
back when it goes. The helicopter, spike strips and moving props are not drawn yet.

**HUD** (`hud.rs`): the car's dials stay the predicted car's; the global block gives the heat, the busted meter, the chase
board (cooldown counts down), the busted overlay, the race place and lap; gameplay events (race start and finish, pursuit start
and end, players joining and leaving, results, the end of the session) show for 4 s in the HUD's message line; the first remote
player is the minimap's second mark.

**Pause.** The pause menu does not pause a client: the pause wrapper (`frontend/pausable_scene.rs`) keeps updating a scene that
`runs_while_paused`, with neutral input.

**Spectators** (`spectate.rs`): a spectator has no car; the world shows the free camera eased behind the racer the session
follows (`follow_next`, `follow_previous` from the screens).

## 5. The front end's view (`NetFrontend`)

`net/frontend.rs` implements the trait of `frontend/lan/backend.rs` for `NetSession`. The lobby and the last result record are
copies taken each frame (`NetSession::refresh`) so the screens can borrow them; the server list is the scan's answers as
`ServerEntry`s.

| Trait call | As built |
|---|---|
| `status` | `Idle`, `Scanning`, `Connecting` (the listen server is opening, or connecting, admitting, reconnecting), `InLobby` (lobby and results phases), `Playing` (loading to running), `Spectating`, `Disconnected` (the session's end reason, or why hosting or connecting failed) |
| `host` | A listen server with the screen's settings (checked), port and name; the player's name and car come from the options. A room password is refused: the server cannot check one yet |
| `connect`, `leave`, `scan` | `NetSession::join` with the password, `leave`, a 2 s scan on port 9750 |
| `set_ready`, `choose_loadout`, `set_role`, `update_settings`, `vote`, `admin`, `say`, `quick_message`, `follow` | The client session's commands |
| `start_race` | The host says ready: the lobby starts by itself once everyone is ready; the client session has no early-start message |
| `follow_slot` | The followed entity is the slot (a player's car has its slot's id) |
| `stats_line`, `lead` | `ClientSession::net_line`, the clock estimate, the input lead and the round trip |
| `set_net_sim` | Refused (only `NetSim::OFF` is taken): the game's endpoint has no simulator to wrap |

## 6. Tests

| What | Where | Needs the install |
|---|---|---|
| The command line | `cli/net_tests.rs` | no |
| Build and data ids, the car catalogue, server options, discovery lines, the identity key file | `net/build.rs`, `net/options.rs`, `net/discover.rs`, `net/identity.rs` | no |
| The game's `NetClient` joins the real server (toy world) over the listen host's in-memory link as slot 0, the session starts, its car follows the server's corrections, the bot is a remote entity | `net/tests.rs` | no |
| `CarSim` save and restore give the same steps bit for bit; a motion reads back; a broken one is ignored | `drive/sim/tests/predict.rs` | no |
| `GameCar` outside a lend, shared clones, state mapping, input edges, which entities are cars | `net_client/tests.rs` | no |
| Which systems a frame runs, single player and client | `frame_systems.rs` | no |
| The HUD from the global block | `net_client/hud.rs` | no |
| `GameCar` rewinds the real `Drive` exactly | `net_client/installed.rs` | yes |
| End to end: the real `Server` over a `WorldSimHost`, a client-mode world and a bot over loopback UDP, the session starts by itself, the client drives 20 s; the server's car follows, the prediction stays within the lead's distance plus 1 m of the server's car every second, no snaps, corrections under the snap distance, the bot drawn interpolated 90 % of frames or more | `net_client/installed.rs` | yes |

The end-to-end test paces its fake 60 Hz clock to the wall clock: the net threads run in real time, and a clock that outruns
them makes every datagram look late, so the input lead climbs to its maximum and the server drops the inputs as too early. A
run on 2026-10-22: 323 m driven, 515 own-car checks, 239 corrections (95th percentile 6 cm), no snaps, the bot interpolated in
1,195 of 1,200 frames.

## 7. Not built

- Two windows on one PC were not tried by hand; the end-to-end test runs both ends in one process without a window.
- The listen host runs two worlds (the server's on its thread and its own client world); a single world for the host is the
  extraction's work ([integration](integration.md) section 4).
- Remote cars: no sound, wheel or damage detail, lights only (headlights, brake); the helicopter, spike strips and moving props
  are not drawn; the first sight of a car model loads it on the main thread (a hitch).
- The predicted car's replays run the tick's effects and sounds again; props it knocks over are local and do not move (the
  client runs no prop physics).
- Time of day: the server sends it (`fill_global`); the client does not set its own clock from it yet.
- `start_race` is a ready; `set_net_sim` is refused; room passwords are refused when hosting.
- The car is the default car (`BMWM3GTR`) from the command line; the transmission setting of the client and the server's car
  may differ, which the prediction then corrects.
