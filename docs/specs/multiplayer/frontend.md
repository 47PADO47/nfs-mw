# Multiplayer front end: the LAN menu, the session HUD, the console and the settings

Status: **built** (2026-10-22) in `crates/nfsmw/src/frontend/lan/`, `devtools/console/net_*.rs`, `settings/net/` and the
metrics overlay; tested against a mock network and in offscreen screenshot runs against a pretend session. Not yet driven by
the game's `NetSession` (it implements [`NetFrontend`](#2-the-seam-netfrontend), [integration](integration.md) section 2).
Where this file and the code disagree, the code's tests are the reference and this file is fixed.

## 1. What the player sees

The main menu's **LAN** icon (the install's `Main_Icon_LAN` picture and `lan` label, enabled now) opens the LAN menu. The
original game has no LAN screens in this build's packages, so there is no FEng package to run: the menu is an **egui panel**
(the layer of the developer panels) drawn over the main menu, in the active UI theme's palette, radius, padding and text size
(`devtools/theme_style.rs`), so it follows Original, Custom and Remastered alike. The main menu stays under it and takes no
input while it is up. The menu is pad-driven like the FEng menus (up, down, left, right, accept, back; a `--ui-script` drives
it), takes typed text on its text rows, and takes clicks on its rows and chevrons.

| Page | Rows | Notes |
|---|---|---|
| Home | Host a Game, Join a Game, Back | The subtitle names the player (`net_name`). Back closes the menu |
| Host | Room name, Mode, Track, Laps, Traffic density, Police, Player limit, Password, Start Hosting, Back | Defaults from `nfsmw_net::settings::defaults` (8 racers, free roam, 50 % traffic, police up to heat 5). Modes: free roam, circuit, sprint, speedtrap, lap knockout, pursuit co-op, tag (drag and cops vs racers are refused by the lobby, ADR 0011). Tracks: the whole city (id 0) for the free modes, the quick-race list's races of the mode otherwise (id: the FEng hash of the race's collection name). Laps only for the lap modes; police off turns pursuits off. The settings are checked by `nfsmw_net::settings::validate` before `host` is called; a refusal or a failed host shows its reason |
| Join | One row per server found (name; mode, players, ping, `locked`, `racing`, `other version`), Refresh, Direct connect, Back | Opening the page scans. A server of another protocol major is dimmed and refused. A locked room asks for its password (a prompt of Password, Join, Cancel); so does a server that ends the session with `PasswordRequired`. Direct connect takes `1.2.3.4`, `1.2.3.4:9760`, `[::1]:9760` or `localhost[:port]`, with `net_port` when no port is typed; no name lookup |
| Lobby | One row per player, Ready, Car, Role, Say, Quick message, the running vote's options (or Vote to restart), Session settings and Start race (host or admin), Leave | A player row shows the name with `[host]`, `[admin]`, `[muted]`, `(you)`, and the car, ready or watching, and the ping. Ready and Role toggle with accept or left and right. Car opens the custom race's car select (`Car_Select.fng`, quick race mode) and the pick comes back as the loadout. Say starts typing; accept sends the line. Quick message cycles the preset lines and sends one. Start race needs every racer ready and the lobby phase. The subtitle has the mode, the head count and the countdown (`Race starts in 3`) or the phase. The chat and the notices are a panel to the right. Back only ends typing: leaving is the Leave row |
| Player menu | Mute for me (or Unmute), Vote to kick, and for the host or an admin Kick, Mute in session (or Unmute), Ban for 10 minutes, Back | Opened by accept on another player's row. The local mute is `nfsmw_net::roles::LocalMutes` (MP-42); the others are `AdminCommand`s the server checks |
| Session settings | The Host page's rows without the password, Apply, Back | Host or admin (MP-25): Apply sends a `SettingsPatch` of what changed |
| Results | `1st  Ann` and `2:31.250  best 0:50.416  DNF` per racer, Continue | Opens over the world when the session's `RaceResults` event arrives, as the original's post-race page does; Continue goes back to the lobby |

The page follows the session once a frame: seated goes to the lobby; a session that ends goes back to the browser with the
reason (`you were kicked`, `wrong password`, `the connection was lost and could not be resumed`, ...); leaving goes home. While
the session's world has the screen (`Playing`, `Spectating`) the menu hides and the HUD pieces show; the results open it again.

**The original's results page.** `lan/results.rs::report` turns the result record into the `RaceReport` that
`PostRace_Results.fng` (`post_race/`) draws, this player first. The session world's race end (`crate::net`) can open it with
`Command::ShowResults`; until it does, the LAN results page above is what shows.

## 2. The seam: `NetFrontend`

`frontend/lan/backend.rs`. The screens, the HUD, the console and the overlay only talk to this trait, in `nfsmw-net` types. The
game keeps it in the non-send resource `LanBackend` (`app.insert_non_send_resource(LanBackend::new(Box::new(session)))`). No
call blocks: what a call starts shows in `status` and `take_events` on later frames.

| Method | Meaning |
|---|---|
| `status() -> NetStatus` | `Idle`, `Scanning`, `Connecting`, `InLobby`, `Playing`, `Spectating`, `Disconnected(Disconnect)`; `Disconnect` is `Session(EndReason)` or `Failed(String)` |
| `servers() -> &[ServerEntry]`, `scan()` | The last scan's servers (`ServerEntry::from_reply` builds one from a discovery `Reply`), and a new scan that empties the list |
| `host(HostOptions) -> Result`, `connect(SocketAddr, Option<&str>) -> Result`, `leave()` | A listen server with `Settings`, password, port, the player's name and `Loadout`; a client to an address; leaving (a host closes its server) |
| `lobby() -> Option<&LobbyView>` | The client session's lobby mirror |
| `take_events() -> Vec<SessionEvent>` | The front end's own copy of the session's events, drained once a frame (the session acts on its events itself) |
| `last_results() -> Option<&RaceResults>` | The last result record |
| `set_ready`, `choose_loadout`, `set_role`, `update_settings`, `start_race`, `vote`, `admin`, `quick_message` -> `bool`; `say -> Result<bool, LimitError>` | The lobby commands; `false` without a seat. `start_race` is the host's start: a listen host runs it on its own server |
| `follow(bool)`, `follow_slot() -> Option<u8>` | The spectator camera (MP-41) |
| `stats_line() -> Option<NetLine>`, `lead() -> LeadInfo` | The overlay line (MP-82) and `net.lead` (server tick, input lead, best round trip) |
| `set_net_sim(NetSim) -> Result` | `net.sim` (MP-83) |

Without a `LanBackend` the menu runs on `lan/offline.rs`, which refuses hosting and joining with `LAN play is not available in
this run`. With `NFSMW_LAN_DEMO=1` it runs on `lan/demo.rs` instead, a pretend session that sends nothing: a scan finds three
rooms (one locked, password `demo`; one of another version), hosting or joining seats the player with two others, chat echoes,
the start counts down five seconds, the race lasts five more with a chat line, a notice and a vote, and ends with a result
record. The screenshots of section 6 use it.

**Files.** `backend.rs`, `handle.rs` (the resource), `menu/` (the pages: `mod.rs` the keys of each page, `host_form.rs`,
`browser.rs`, `lobby.rs`, `player.rs`, `follow.rs`, `page_view.rs`, `rows.rs`, `field.rs`, `cx.rs`), `view.rs` (egui), `system.rs`
(the Bevy systems), `gate.rs` (the front end waits while the menu is up, and opens the car select it asked for), `link.rs` (the
requests between the FEng screens and the menu), `feed.rs`, `hud.rs`, `results.rs`, `text.rs`, `car.rs`, `quick.rs`,
`address.rs`, `host_options.rs`, `offline.rs`, `demo.rs`, `mock.rs` (tests).

**Cars.** A car in a session is `Loadout::car_type`: the FEng string hash (`fe_hash_upper`) of the car's folder name, the
preset's part dropped (`lan/car.rs`). The roster names a car by hashing the folders the quick-play pool knows, else shows the id
in hex.

## 3. The session HUD

While the session's world has the screen (`lan/hud.rs`, drawn by `view.rs`):

- **Feed** (MP-40), bottom left: chat lines (`Ann: go`, `Ann (whisper): ...`) and notices (join, leave, kick, mute, vote,
  result, `Session over: ...`, connection lost and resumed), six at most, each for 8 s and fading over its last 2 s. Chat of a
  locally muted player is left out.
- **Vote** (MP-27), top: `Vote: kick Ann  2 votes  18 s left  (vote in the lobby)`.
- **Countdown** (MP-22), top: `Race starts in 3`, from `StartCountdown` and the estimated server tick.
- **Spectator panel** (MP-41), top right: `Following Ann` (or `Free camera`) and the racers, in the last result's order when
  there is one. Left and right step the follow camera (`NetFrontend::follow`).

## 4. Console commands

`devtools/console/net_parse.rs` (text to `NetCommand`) and `net_cmd.rs` (run through the `LanBackend`), listed by `help` and
Tab. Without a network every command answers `there is no network in this run`.

| Command | What it does |
|---|---|
| `net.host [room name]` | Hosts with the LAN defaults, `net_port` and `net_name`; the room is `<name>'s game` unless named |
| `net.connect <ip[:port]> [password]` | Connects; the port defaults to `net_port` |
| `net.disconnect` (`net.leave`) | Leaves (`not in a session` when idle) |
| `net.stats` | The overlay line: `net rtt 3 ms  jit 0.4 ms  loss 0.2%  snap 48 ms  corr 2/min  snaps 0/min  in 14.2 KB/s  out 2.1 KB/s` |
| `net.sim <loss %> <rtt ms> <jitter ms>` or `net.sim off` | `NetSim::parse` (loss up to 50 %, rtt up to 2000 ms, jitter up to 1000 ms) |
| `net.kick <slot>` | Host or admin: `AdminCommand::Kick` |
| `net.say <text>` | Chat to everybody; refused text says why |
| `net.lead` | `server tick 1234.5  lead 4 ticks  rtt 3 ms` |
| `net.scan`, `net.servers`, `net.status` | A scan, the servers found, where the session is |

## 5. Settings

`settings/net/` (`NetSettings` in `Settings::net`): command line > environment > config file > defaults, as every other
setting. The config keys are top level; a value that does not parse is logged and left to the lower layer.

| Key | Environment | Command line | Default | Range |
|---|---|---|---|---|
| `net_name` | `NFSMW_NET_NAME` | `--net-name NAME` | `Player` | the protocol's name: 1 to 24 characters, no control or bidirectional characters |
| `net_port` | `NFSMW_NET_PORT` | `--net-port PORT` | 9750 (discovery on the next port) | 1024 to 65534 |
| `net_snapshot_hz` | `NFSMW_NET_SNAPSHOT_HZ` | `--net-snapshot-hz HZ` | 20 | 1 to 60 |
| `net_interp_delay` | `NFSMW_NET_INTERP_DELAY` | `--net-interp-delay INTERVALS` | 2 snapshot intervals | 1 to 4 |
| `net_show_stats` | `NFSMW_NET_SHOW_STATS` | `--net-show-stats` / `--no-net-show-stats` | off | on or off |
| `net_identity_file` | `NFSMW_NET_IDENTITY_FILE` | `--net-identity-file PATH` | `identity.key` next to the config file | a path up to 240 bytes; `default` in the console |

The console reads and sets them (`get net_port`, `set net_port 9760`, `set net_show_stats` flips). The settings are `Copy`, so
the name and the path are kept inline (`InlineText`). `NetSettings::identity_path(config_dir)` gives the key file.

**The overlay line** (MP-82): with `--show-metrics advanced` (or `metrics advanced`) the metrics panel shows the session's `net`
line under the frame rate; with `net_show_stats` on it shows at every level, alone when the metrics are off. No session, no line.

## 6. Tests and screenshots

- `frontend::lan::menu` (mock network, key by key): home, host (the form sent, a failure kept on the form), join with a locked
  room's password, direct connect with the settings port and a bad address, a password asked after the fact, a kick back to the
  browser, results over the world, the car select's answer as the loadout; `lobby_tests.rs`: the roster's badges, ready, role
  and car, chat and quick lines, host-only rows and start waiting for ready, votes, leave, the countdown, the player menu.
- `host_form_tests.rs`, `feed.rs`, `hud.rs`, `results.rs`, `text.rs`, `address.rs`, `car.rs`, `quick.rs`, `link.rs`, `demo.rs`,
  `system.rs`: the form's values and patch, the feed's order, mutes and fading, the HUD pieces, the result rows and report,
  the words, addresses, car ids, quick ids, the shared requests, the pretend session, the pad keys and the follow keys.
- `devtools::console::net_parse` and `net_cmd`: every command's forms and refusals, run on the mock. `settings::net`: defaults,
  layers and clamping, the config keys, the environment, a written file read back, the command line, the console.
- `devtools::overlay`: when the net line shows. `icon_items`: the LAN icon is enabled.

Offscreen screenshots (no window, no focus taken), `NFSMW_LAN_DEMO=1`, the main menu's LAN icon reached with the pad:

```
nfsmw play --skip-boot --ui-style original --show-metrics advanced \
  --ui-script "wait 3;right;wait 0.3;right;wait 0.3;right;wait 0.3;right;wait 0.3;right;wait 0.5;accept;wait 1;accept;wait 1" \
  --screenshot host.png
```

In the Remastered style the main menu is a vertical list: `down` in place of `right`. Checked on 2026-10-22: Home, Host (circuit,
its track and laps), Join (three rooms, one locked, one dimmed for another version), Lobby (roster, ready, chat panel) in the
Original and Remastered styles, the HUD while racing (feed, vote banner, the net line in the advanced metrics) and Results.

## 7. Not built

- FEng screens for LAN: the install has none to run; the egui panels stand in. The original's post-race page for a session's
  results waits for the session world's race end (section 1).
- Voting from the HUD while racing: the vote prompt says to vote in the lobby (the pause key opens the FEng pause menu, not the
  LAN menu, in a session; ADR 0011 keeps the world running).
- Team chat and whispers from the menu (the feed shows them; `say` sends to all); quick lines from other players (the catalogue
  has no server-to-client quick message, [features](features.md) MP-44).
- Saving `net_name` from the menu (the console and the config file set it).
