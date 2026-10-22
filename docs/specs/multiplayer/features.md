# Multiplayer: requirements and feature catalogue

Status: **design**, with the wire foundation and the gameplay rules of `crates/nfsmw-net` built on 2026-10-22 (rows marked **Built** say what; the modes, fairness rules, results and replays as built are in [modes](modes.md)). Milestone 10 (LAN multiplayer) and milestone 11 (online multiplayer) in the
[roadmap](../../roadmap.md) are not started. Written 2026-10-16. The netcode decisions are in
[ADR 0006](../../decisions/0006-multiplayer-netcode.md); the wire format is in [protocol](protocol.md); the sync
algorithms are in [sync model](sync-model.md). Security is a separate document, `security.md` in this folder (written
by another author); this file only names the requirements that depend on it.

## 1. Scope

**In scope.** Playing the rewrite with other people on a LAN (milestone 10), with the same server and the same protocol
later reaching the internet (milestone 11). Races, free roam, and pursuits, played by 1 to 16 humans, with traffic and
police simulated by the server.

**Out of scope.**

- Compatibility with the original game's online or LAN protocol (see ADR 0006, section "The original LAN protocol").
- Cross-play between the rewrite and the 2005 `speed.exe`.
- Matchmaking, ranking ladders, a cash shop, and anything that needs an account service. Milestone 11 may add a
  server list and identities; this file marks which features need them.
- Anti-cheat software that runs on the player's PC. The server is the referee (ADR 0006).

**Ground rules.**

- One server program for every case. A "host" in the lobby is a normal player whose game also runs the server code
  in-process; a dedicated server is the same code without a window.
- The server needs the user's own game install (road network, collision, car data). No game data is ever sent over the
  network except small derived values (ids, hashes, parameters chosen from install data on both sides).
- Features that need a system that does not exist yet (races, career, mods) are listed as blocked, not faked.
- A feature that changes game state is decided by the server. A client may only ask.

## 2. Priority legend

- **P0**: the first playable LAN build (milestone 10 core).
- **P1**: needed to make LAN play pleasant, or needed before the online milestone starts.
- **P2**: later, or blocked on another milestone. Listed so that the protocol leaves room for it.

Target milestone: **M10** (LAN), **M11** (online). A P1 feature may be M11-only.

Status words: **not built** unless a **Built** note says which part exists (the wire format or the check, not the play). A **blocked on** note names the system that must exist first.

## 3. Feature catalogue

### 3.1 Game modes

| ID | P | Mode | Notes and dependencies |
|---|---|---|---|
| MP-01 | P0 | **Free roam** | Everybody drives the open city with traffic. The simplest mode and the first to build: it needs only players, traffic, collisions between cars and chat. Pursuits can be switched off in the session settings. **Built (rules):** `modes::FreeRoam` |
| MP-02 | P0 | **Circuit race** | Laps over a checkpoint loop; positions, lap times, finish order. Blocked on the single-player race system (roadmap milestone 7, items "races" and "race checkpoints"). Race rules must be written as pure server-side rules from the start, so single player and multiplayer share them. **Built (rules):** `modes::Race` over the world's lap and finish events, with reset penalties and ties ([modes](modes.md)) |
| MP-03 | P0 | **Sprint race** | Point-to-point. Same dependency as MP-02. **Built (rules):** `modes::Race` |
| MP-04 | P1 | **Drag race** | Straight strip, launch and shift timing, reaction time. Blocked on the drag-race reimplementation (milestone 7, "reimplement drag races and their features"). The server needs the sim tick of the start light to score reaction time. **Not built** (ADR 0011 section 5): the lobby refuses it (`Unsupported::Drag`) |
| MP-05 | P1 | **Speedtrap race** | Sum or best of speed readings at trap gates. Blocked on the speedtrap event data (milestone 7, "event icons"). **Built (rules):** `modes::Speedtrap` |
| MP-06 | P1 | **Knockout** | Last place at the end of each lap is out; eliminated players become spectators. An original event type (its minimap icon exists in the game); blocked like MP-02. **Built (rules):** `modes::Knockout` |
| MP-07 | P1 | **Pursuit co-op** | Several players share one pursuit against AI police. One pursuit entity with several targets; heat and busted are tracked per player, the pursuit ends when no target remains. Built on `blackbox-pursuit`, which is pure functions, so the server can run it as it is. Amended by ADR 0010: heat, cooldown, helicopter and roadblocks are shared; busted meter and bust are per target; built first for split-screen co-op. **Built (rules):** `modes::PursuitCoop` over `modes::PursuitTracker` |
| MP-08 | P1 | **Cops vs racers** | Some players drive police cars against the others. Player cops use the same car simulation as racers with a role flag; they get cop tools (siren, roadblock call, spike strip, busted contact). Blocked on a decision for what a human cop can do (open question 2). **Not built** (ADR 0011 section 5): refused as `Unsupported::CopsVsRacers` |
| MP-09 | P2 | **Tag** | One player is "it" and holds a timer; contact with a free player (a relative-speed and distance rule evaluated on the server) passes the tag. A new mode, not in the original. **Built (rules):** `modes::Tag` (4 m, 1 m/s, no tag-back for 3 s) |
| MP-10 | P2 | **Last standing / elimination by damage** | Optional. Needs the damage model to be a gameplay value, which it is not today (damage is visual, `blackbox-damage`) |
| MP-11 | P2 | **Custom rules** | Server settings: laps, traffic density, cop level cap, time of day, nitrous on or off. Later exposed to scripts through the mod API (milestone 12). **Built (check):** `modes::CustomRules::validate` |

### 3.2 Lobby, discovery and sessions

| ID | P | Feature | Notes |
|---|---|---|---|
| MP-20 | P0 | **LAN discovery** | A client broadcasts a small discovery request on the game port and servers answer with name, mode, players, version and password flag. No internet service is involved. Direct connect by address also works. Formats in [protocol](protocol.md) section 3 |
| MP-21 | P0 | **Lobby** | Before the session starts: player list, car choice, ready state, session settings, host or admin rights, chat. The same lobby state is the pause-screen state between races |
| MP-22 | P0 | **Ready-up and start countdown** | A player is ready or not; the host (or an admin) starts when all are ready, or a timer does. The server sets the start tick so every client counts down to the same sim tick |
| MP-23 | P0 | **Join in progress policy** | Per session: closed after start, open in free roam, open between races. A joiner in a running race becomes a spectator until the next race (MP-41) |
| MP-24 | P0 | **Player limits** | 8 players are the LAN target. The id spaces and the budgets are designed for 16 (ADR 0006). Spectators are counted separately |
| MP-25 | P1 | **Server settings and passwords** | Name, password, mode, track, laps, traffic density, cop cap, player limit, public or LAN-only. Stored in a TOML file for a dedicated server; edited in the lobby UI by the host |
| MP-26 | P1 | **Roles and permissions** | Host and admins can kick, mute, change settings. Players cannot. Roles are checked by the server on each command |
| MP-27 | P1 | **Vote system** | Vote for track, restart, kick. Simple majority; the server counts |
| MP-28 | P2 | **Server list** | A list of public servers kept by a master service. Online only (M11); needs the identity and heartbeat design in `security.md`. LAN discovery stays local |
| MP-29 | P2 | **Invite by link or code** | A short code that resolves to an address through the master service. M11 |

### 3.3 Cars, rules and fairness

| ID | P | Feature | Notes |
|---|---|---|---|
| MP-30 | P0 | **Car selection** | The client sends a loadout: a car type id and a parts selection (ids, not tuning numbers). The server rebuilds the vehicle parameters from its own copy of the game data, so a client cannot invent numbers. Both sides build the same `VehicleSpec` from the same ids. **Built (wire):** the `Loadout` message and `Loadout::to_join` in `nfsmw-net`; the server's rebuild from ids is not |
| MP-31 | P1 | **Car and class restrictions** | The server setting names an allowed set of cars, a class or a performance cap. A client with a car outside it is refused in the lobby, with a reason code. **Built (check):** `loadout::validate` over a `Catalog` the game implements, with reason codes ([protocol messages](protocol-messages.md) 7.4); the game's catalogue and class ratings are not |
| MP-32 | P1 | **Catch-up on or off** | Optional rubber-banding rules for races (a boost for trailing players). The original has none for human racers, so it is **off by default** and a server setting. The server applies it; clients display it. **Built (rule):** `fairness::CatchUpRule` |
| MP-33 | P1 | **Collisions on, off, or ghost** | Per session and per phase: full car-to-car collisions, collisions off between racers (ghost cars, semi-transparent), or ghost only during the first seconds of a start. Traffic and world collisions are unaffected. **Built (rule):** `fairness::pair_collides` |
| MP-34 | P1 | **Nitrous and damage rules** | Per session toggles for nitrous, damage visuals, and repair at the lobby. Damage stays visual. **Built (rule):** `fairness::Toggles` |
| MP-35 | P1 | **Wrong-way and reset rules** | Reset-to-track (the existing `ResetCar` action) is a server-validated respawn with a time penalty in races. A reset in free roam is free. **Built (rule):** `fairness::ResetRule` and the `fairness::WrongWay` notice |
| MP-36 | P2 | **Handicap by class** | Scaled grip or power to equalise cars. Needs a design for what is fair; do not build before MP-31 is played |

### 3.4 Communication

| ID | P | Feature | Notes |
|---|---|---|---|
| MP-40 | P0 | **Text chat** | Reliable, ordered, rate-limited, length-limited. Lobby chat, in-race chat, and system notices (join, leave, kick, result). Printed in the HUD and in the F12 console log |
| MP-41 | P1 | **Spectator** | A player who is not in the session can watch: a free camera, a follow camera on any racer, a leaderboard. A spectator receives snapshots at the spectator rate and sends no inputs. Used for knockout (eliminated players), late joiners, and a live broadcast view |
| MP-42 | P1 | **Mute and block** | Per player, local only for chat; the server enforces the host's mute |
| MP-43 | P2 | **Voice chat** | Push to talk over an unreliable channel, Opus. Not in v1. The packet format reserves a frame type (protocol, section 5.2). The codec and licence check is a separate decision; voice is a privacy topic and goes through `security.md` |
| MP-44 | P2 | **Quick messages and emotes** | Preset lines for the pursuit ("police behind me"), sent as ids. Cheap and moderation-free |

### 3.5 Robustness

| ID | P | Feature | Notes |
|---|---|---|---|
| MP-50 | P0 | **Disconnect handling** | A timeout (default 10 s without any datagram) or a disconnect message removes the player cleanly: the car is removed or parked, pursuits and races update, chat prints a notice |
| MP-51 | P1 | **Reconnect** | A dropped player can rejoin the same session within 30 s with a resume token and get its slot back. The car is kept as a non-colliding ghost during that time. A reconnect after the window is a normal join |
| MP-52 | P2 | **Host migration** | If a listen-server host quits, the session ends in v1. A dedicated server avoids the problem. A design for a soft migration (hand the session to another machine) is in ADR 0006 and is P2 because the AI state is not portable today |
| MP-53 | P1 | **Late join sync** | A joiner receives the session state (a bulk transfer on a reliable channel) and then the snapshot stream from a full baseline. No client simulates traffic, so there is no world state to download beyond ids and seeds |
| MP-54 | P1 | **Pause rules** | The server never pauses: a game pause menu in a multiplayer session does not stop the sim. Only the host-requested "everyone wait" in the lobby exists |

### 3.6 Compatibility checks

| ID | P | Feature | Notes |
|---|---|---|---|
| MP-60 | P0 | **Protocol version check** | The first exchange carries a protocol version and the reject reason when it differs. No client of another major version enters a session. **Built (check):** `version::check_join` and the `JoinRejected` reason codes; the handshake that carries it is not |
| MP-61 | P0 | **Game data check** | The server and each client compare a data id: the identified `speed.exe` hash (v1.3 is the only supported build) plus a hash of the list of required files. A different data set is refused before the join. **Built (check):** `DataId::compute` and the join check; reading the file list from the install is not |
| MP-62 | P1 | **Sim build check** | The handshake carries the sim build id. The same build predicts accurately. A different build, or a different operating system, may join and play; the server is authoritative, so the only effect is more corrections (ADR 0006, determinism). A server setting can require an exact match for competitive sessions. **Built (check):** `SimBuild` and the `SimBuildMismatch` reason |
| MP-63 | P1 | **Mod compatibility** | A server lists the mods that change gameplay (ids, versions, content hashes). A client must have the same set. Client-only mods (HUD, audio, ReShade) are marked safe and are not compared. Blocked on the mod API (milestone 12); until then the list is empty and the check passes. **Built (check):** the manifest entry and the off, lenient and strict comparison (`compare_mods`) |
| MP-64 | P1 | **Cross-version policy** | The server accepts a range of minor protocol versions. A newer client talking to an older server falls back to the older feature set. A major bump is a hard reject. See [protocol](protocol.md) section 8. **Built (check):** minor negotiation, the oldest served minor, and the feature-by-minor table (empty so far) |

### 3.7 Records, stats and replays

| ID | P | Feature | Notes |
|---|---|---|---|
| MP-70 | P1 | **Race results and session stats** | Finish order, best lap, top speed, damage, busted count, pursuit time. Written by the server at the end of each race. Sent to clients as an event and shown in the results screen. **Built (format):** `results::RaceRecord` (versioned, encode and decode, `to_message`), `results::Tally` |
| MP-71 | P1 | **Server-side replay recording** | The server writes its snapshot stream to a file (the stream is the replay). Playback reuses the client interpolation code. Shares the recorder with telemetry F-33. **Built (format):** `record::Recorder` and `record::Replay` |
| MP-72 | P1 | **Client-side recording** | A client can record the snapshots it received, plus its own inputs and predicted states, for netcode debugging and for a personal replay. **Built (format):** the client kind of `record` |
| MP-73 | P2 | **Replay viewer in the game** | A free camera over a recorded session. Blocked on the replay playback of telemetry F-34 |
| MP-74 | P2 | **Persistent stats and rankings** | Needs identities and a database. M11 and later; the format of the per-race result record is fixed now so it can be stored later (format 1, [modes](modes.md) section 5) |
| MP-75 | P2 | **Career-lite sharing** | Share a car or a time. The career model now exists (`crates/nfsmw-career`), and co-op career shares one profile ([ADR 0010](../../decisions/0010-coop-and-the-n-player-world.md)). The sharing itself is not built |

### 3.8 Quality of play

| ID | P | Feature | Notes |
|---|---|---|---|
| MP-80 | P0 | **Client prediction of the own car** | The own car responds on the frame the input is read. Corrections from the server are smoothed. See [sync model](sync-model.md) |
| MP-81 | P0 | **Interpolated remote cars** | Other players, traffic and police are shown slightly in the past between two snapshots, with a short extrapolation when a snapshot is late |
| MP-82 | P0 | **Network overlay** | The metrics overlay gains a line: RTT, jitter, loss, snapshot age, correction count, bandwidth in and out. Part of the advanced level of `--show-metrics` |
| MP-83 | P1 | **Simulated bad network** | Debug settings that add latency, jitter and loss on send and receive, for tests (also usable on LAN). Console commands `net.sim`; required for the test plan |
| MP-84 | P1 | **Headless dedicated server** | `nfsmw-server` runs without a window or audio, reads a TOML file, logs to a file, and exposes the telemetry server (read-only) for monitoring |
| MP-85 | P1 | **Telemetry bridge** | The telemetry server (milestone 13) can read a client's view of a session and, on the server, the whole session; it stays read-only ([multiplayer reuse](../telemetry/multiplayer-reuse.md)) |
| MP-86 | P2 | **Region and ping display** | Online server list column. M11 |

## 4. Targets

These are the numbers the design is checked against. They are targets, not measurements; the test plan in
[ADR 0006](../../decisions/0006-multiplayer-netcode.md#how-we-will-know) says how each is measured.

| Target | LAN (M10) | Online (M11) |
|---|---|---|
| Players | 8 racers (design for 16), plus spectators | 16 racers, plus spectators |
| Round-trip time the design must handle | up to 20 ms, typical 1 to 5 ms | up to 150 ms playable, 250 ms degraded |
| Packet loss handled | 2 % without visible artefacts | 5 % playable, 10 % degraded |
| Own-car response | the same frame (prediction) | the same frame (prediction) |
| Remote car display delay | 100 ms at 20 Hz snapshots, 66 ms at 30 Hz | 100 to 150 ms |
| Correction visible on the own car | median under 5 cm, p95 under 30 cm in free roam | p95 under 60 cm |
| Hard snaps of the own car | fewer than 1 per minute without contacts | fewer than 3 per minute |
| Bandwidth to a client | at most 32 KB/s hard cap, typical 16 KB/s at 8 players | at most 12 KB/s default budget |
| Bandwidth from a client | at most 4 KB/s (about 2.5 KB/s typical) | the same |
| Server tick | 60 Hz, the physics step | 60 Hz |
| Server CPU at 16 players | under 25 % of one core of a 2018 desktop CPU, [unconfirmed, to measure] | under 40 % of one vCPU |
| Join time to first snapshot | under 2 s on LAN, excluding world load | under 4 s |

## 5. Summary

| Priority | Count | What it covers |
|---|---|---|
| P0 | 16 | Free roam, circuit and sprint races, LAN discovery, lobby, ready-up, join policy, player limits, car selection, text chat, disconnect, version and data checks, prediction and interpolation, the network overlay |
| P1 | 27 | Drag, speedtrap, knockout, co-op pursuit, cops vs racers, settings and roles, votes, restrictions, catch-up, collision modes, spectator, mute, reconnect, late join, sim and mod checks, results and replays, bad-network simulation, headless server, telemetry bridge |
| P2 | 13 | Tag, last standing, custom rules by script, server list, invite codes, handicap, voice, emotes, host migration, replay viewer, persistent stats, career sharing, region display |

## 6. Dependencies on other milestones

| Needs | From | Blocks |
|---|---|---|
| Race system, checkpoints, race AI | milestone 7 | MP-02 to MP-06, MP-70 |
| A headless world simulation separated from the scene's rendering | this design, step 0 of milestone 10 (ADR 0006, section "Where the code lives") | everything |
| Mod API and mod manifest | milestone 12 | MP-63, MP-11 scripted rules |
| Identities, server list service | milestone 11 | MP-28, MP-29, MP-74 |
| Telemetry server transport and hand-off primitives | milestone 13 design, shared library | MP-85 |
| Replay file format | telemetry F-33 | MP-71, MP-73 |

## 7. Open questions

1. **Race systems come first.** Circuit and sprint modes (P0) need the single-player race system. Until it exists,
   the first milestone 10 build can only ship free roam. Does the owner accept free roam plus pursuit as the first LAN
   release, with races added as milestone 7 lands?
2. **Human cops.** What can a human police driver do that the AI cop does not (roadblock call, spike strip, heli
   request)? The AI actions are specified in the `ai-pursuit-*` specs; a human-triggered version needs a rule for
   cooldowns and a HUD. Proposal: only siren, ram, and a cooldown-limited spike strip and roadblock call in v1.
3. **Traffic density.** Traffic is simulated once on the server and sent to all clients. The density per player count
   is a server setting; a default needs measurement of the cost (ADR 0006, bandwidth section).
4. **Crossing platforms.** Mixed Windows and Linux sessions work by design (server authoritative) but are untested.
   Is a Linux dedicated server with Windows clients a supported setup for milestone 10?
5. **Pause.** MP-54 says the sim never pauses. Does the owner want an "everyone pauses" vote on LAN?
