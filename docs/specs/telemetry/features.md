# Telemetry server: feature catalogue

Status: **design only**. Nothing is built. Milestone 13 in the [roadmap](../../roadmap.md) reads "Websocket/server/something
for telemetry/api info on player career etc". Written 2026-10-16. Companion files: [channels](channels.md),
[transport and privacy](transport-and-privacy.md), [control](control.md), [multiplayer reuse](multiplayer-reuse.md).

## 1. Scope

**In scope.** An optional server inside the game process. It publishes the game's live state as snapshots (the
latest values), streams (rate-limited values) and events (things that happened, with a sequence number). It records
sessions to local files. It offers a small set of opt-in write endpoints. Consumers: scripts, sim-racing dashboards,
motion rigs, OBS overlays, and later the mod API.

**Out of scope.**

- The original game's online protocol. The lobby traffic of `server.dll` is in [lan-server.md](../lan-server.md) and is
  a different thing.
- Any cloud service, analytics, crash upload or update check. Nothing leaves the PC (see section 7).
- Replacing the F12 console. The console stays the human debug tool. The server shares its command table (see
  [control](control.md)).

**Ground rules.**

- Every channel names its real source symbol (see [channels](channels.md)). No value is invented.
- Where the game has no data yet (tyre temperature, races, checkpoints, career, bounty) the feature is listed as
  **not built** with the reason. It is not faked.
- Sample rates come from the simulation, not the render loop. The fixed simulation step is 1/60 s
  (`FIXED_STEP` in `libs/blackbox-vehicle/src/lib.rs`). A render at 240 fps does not make the telemetry 240 Hz.

## 2. Priority legend

- **P0**: needed for the first usable version. Milestone 13's core.
- **P1**: needed for the listed integrations. Comes after P0.
- **P2**: later, or blocked on another milestone. Listed so the schema leaves room for it.

Status words: **built** (the value exists in code now), **partial** (some of it exists), **not built** (no source yet).

## 3. Feature catalogue

### 3.1 Live data (channel groups, detail in channels.md)

| ID | P | Feature | Status and source |
|---|---|---|---|
| F-01 | P0 | Vehicle snapshot: speed, rpm, gear, shifting, nitrous, boost, wheels on ground, position, orientation, velocities | built: `Vehicle` getters in `libs/blackbox-vehicle/src/vehicle/mod.rs`; the HUD's `Telemetry` in `crates/nfsmw/src/scenes/world/drive/sim/mod.rs` |
| F-02 | P0 | Per-wheel channels: slip, load, traction, lateral speed, slip angle, compression, on ground, locked, steer angle | built: `WheelState` in `libs/blackbox-vehicle/src/vehicle/state.rs` |
| F-03 | P0 | Driver inputs: throttle, brake, steering, clutch, gear request | built in the library (`Controls`, `InputState`); the exact fields to confirm in `libs/blackbox-vehicle/src/input/mod.rs` |
| F-04 | P0 | Frame metrics: fps, average frame time, 1 % low, worst frame, meshes, textures | built: `Metrics` in `crates/nfsmw/src/devtools/metrics.rs` |
| F-05 | P0 | World clock: hour, seconds per game hour, lights on or off, sun phase | built: `DayClock` in `crates/nfsmw/src/scenes/world/day.rs` |
| F-06 | P0 | Mode flags: world, drive, free camera, garage, pause, loading | partial: the commands `freecam`, `garage` and the pause and garage hooks exist; the state accessor to confirm in `crates/nfsmw/src/scenes/world/mod.rs` |
| F-07 | P1 | Damage: per zone panel, paint and glass (0 to 3), punctured wheels | built in the library: `Damage` in `libs/blackbox-damage/src/lib.rs`; fed by the drive (`damage` and `repair` commands, `damage_marks.rs`) |
| F-08 | P1 | Pursuit: heat (value and level 1 to 10), busted timer and meter, cool-down, infractions and fines, end reason | built: `libs/blackbox-pursuit` (`Heat`, `Busted`, `Cooldown`, `InfractionLog`, `EndReason`); scene state in `crates/nfsmw/src/scenes/world/ai/pursuit.rs` |
| F-09 | P1 | Traffic and police: counts by type, density, roadblocks, helicopter state | built: `crates/nfsmw/src/scenes/world/ai/` (`traffic.rs`, `cop.rs`, `police/`, `roadblock.rs`, `manager.rs`) |
| F-10 | P1 | Radio: now playing (artist, title, album), elapsed and length | built: `NowPlaying` in `crates/nfsmw/src/audio/radio/state.rs` |
| F-11 | P1 | Camera: mode and view name | built: `crates/nfsmw/src/scenes/world/camera_choice.rs`, `camera_cycle.rs`; names from [camera-views](../camera-views.md) |
| F-12 | P1 | Settings (no paths, no secrets): show-metrics level, max fps, backend, volumes | built: `crates/nfsmw/src/settings/` (layers and keys) |
| F-13 | P2 | Races and race checkpoints | not built: milestone 7 items (1) and (2) |
| F-14 | P2 | Career and bounty | not built: the pursuit bounty row has no feed (`crates/nfsmw/src/hud/elements.rs`); career has no progress model (the save store in `libs/save-store` keeps slots and aliases only) |
| F-15 | P2 | Tyre temperature | not built: no temperature field exists in `libs/blackbox-vehicle` or any crate |
| F-16 | P2 | Accelerations and g-forces | derived, not stored: body-frame velocity change per sim step (for motion rigs, F-44) |
| F-17 | P2 | Engine audio frequency and the sample layer state | built in audio (`current_frequency` in `crates/nfsmw/src/audio/engine.rs`); useful only for a sound-debug dashboard |

### 3.2 Events

| ID | P | Feature | Source |
|---|---|---|---|
| F-20 | P0 | Event stream with a sequence number and sim time: gear change, nitrous on or off, reset, repair, damage impact, puncture, pursuit start and end, busted, evaded, heat level change, infraction recorded, radio song change, lights on or off, camera change, setting change | the sources in channels.md, section "Events" |
| F-21 | P1 | Collision and wall contact events with their impulse | `notify_collision` and `notify_wall_contact` in `libs/blackbox-vehicle/src/vehicle/` |
| F-22 | P1 | Roadblock and spike events | `roadblock.rs`, `spikes.rs` in the AI folder |

### 3.3 Sessions and recording

| ID | P | Feature | Notes |
|---|---|---|---|
| F-30 | P0 | Start and stop a recording to a local JSON-lines file | one line per sample: sim time, frame index, tick, chosen channels; the channel set and rate are chosen at start |
| F-31 | P0 | CSV export of a recording | flat columns, a header line, the same units as the catalogue |
| F-32 | P1 | Session summary written at stop | top speed, max rpm, max heat, pursuits started, busted count, infractions, distance, time driven |
| F-33 | P1 | Replay file: 60 Hz state and input samples | JSON lines first; a binary layout is P2 |
| F-34 | P2 | Replay playback in the viewer (`view-world`) | needs F-33 and a playback mode in the viewer |

Recordings never start on their own. A status line shows "recording" while one runs. Files go to the per-user data
folder next to the config file (see [transport and privacy](transport-and-privacy.md)).

### 3.4 Live overlays and integrations

| ID | P | Feature | Notes |
|---|---|---|---|
| F-40 | P0 | Local REST snapshot (GET) | answers from the last published copy; scripts and quick checks |
| F-41 | P1 | WebSocket stream: subscribe to channels, each at its own rate | for dashboards; JSON frames |
| F-42 | P1 | OBS browser-source overlay | a static page served locally that reads F-41; shows speed, gear, nitrous, heat, busted meter |
| F-43 | P1 | Sim-racing dashboard feed: fixed UDP JSON datagrams | a documented fixed format at a set rate (10 to 60 Hz); for dash and gauge software |
| F-44 | P2 | Motion rig feed: 60 Hz g-forces, yaw, pitch and roll over UDP | needs F-16 and a mapping file for each rig; the rate is the sim rate |
| F-45 | P2 | Discord status bridge | reuses `PresenceStatus` in `crates/nfsmw/src/discord/presence.rs`; the presence itself is milestone 9; the telemetry server only passes state along |
| F-46 | P1 | Status page: server version, game build, sim tick, session, channel list with units and rates, drop counters | GET only |

### 3.5 Control endpoints (read-only by default; details in [control](control.md))

| ID | P | Feature |
|---|---|---|
| F-50 | P0 | Read-only by default: when the server is on, every endpoint is GET; writes are off |
| F-51 | P1 | Opt-in writes: set time of day, teleport, reset, repair, spawn traffic, pursuit or roadblock, set damage |
| F-52 | P1 | Write token: random per launch, required for every write, never logged |
| F-53 | P1 | Every accepted write is logged and appears in the event stream |
| F-54 | P2 | Change the car mid-session (the `drive` command) |
| F-55 | P2 | Mod API: the same command table exposed to the scripting API (milestone 12) |

### 3.6 Settings and privacy (detail in [transport and privacy](transport-and-privacy.md))

| ID | P | Feature |
|---|---|---|
| F-60 | P0 | Off by default. Settings live in the layered settings (command line, environment, config file, defaults) |
| F-61 | P0 | Listens on `127.0.0.1` only by default; a LAN address needs an explicit setting and a warning |
| F-62 | P0 | No outbound connections of any kind; no install path, user name, PC name or save file name in any output |
| F-63 | P1 | Channel catalogue marks each channel as redactable; the catalogue, not the client, decides what is exported |
| F-64 | P1 | Origin check on WebSocket upgrades: a browser page on another origin cannot read the stream |

### 3.7 Performance (detail in [transport and privacy](transport-and-privacy.md), section 3)

| ID | P | Feature |
|---|---|---|
| F-70 | P0 | Game-thread cost at most 0.2 ms per frame when on (about 1.2 % of a 16.7 ms frame at 60 fps); zero when off |
| F-71 | P0 | Lock-free hand-off to a server thread; the game thread never waits on a socket, a file or a lock |
| F-72 | P0 | A benchmark that measures the game-thread cost (not a unit test: hosts vary) |
| F-73 | P1 | An advanced-overlay line for the server's own cost and dropped-event count |

### 3.8 Multiplayer reuse (detail in [multiplayer reuse](multiplayer-reuse.md))

| ID | P | Feature |
|---|---|---|
| F-80 | P0 | Every snapshot and event carries a schema version and a sim tick number (cheap now, needed later) |
| F-81 | P1 | Remote-car state subset: position, orientation, velocities, wheel contacts, gear, inputs |
| F-82 | P1 | Token and origin-check design shared with LAN play |
| F-83 | P2 | Binary observer protocol and a separate fixed-layout game-sync protocol |

## 4. Summary

| Priority | Count | What it covers |
|---|---|---|
| P0 | 18 | Vehicle, wheels, inputs, frame metrics, clock, mode flags; the event stream core; recording and CSV; local REST; off by default, localhost, no outbound; the budget, the hand-off and its benchmark; read-only; schema version and tick |
| P1 | 22 | Damage, pursuit, traffic, radio, camera, settings; collision and roadblock events; session summary and replay file; WebSocket, OBS overlay, sim-racing UDP, status page; opt-in writes and token; origin check; cost line in the overlay; remote-car subset; shared auth design |
| P2 | 11 | Races, career and bounty, tyre temperature, g-forces, audio frequency, replay playback, motion rig, Discord bridge, car change, mod API, binary protocol |

Blocked on other milestones: races and checkpoints (7), career and bounty (7 and the career model), Discord (9),
multiplayer (10, 11), the mod API (12). The tyre temperature has no milestone yet; it needs a model decision first.

## 5. Open questions

1. Does the server thread model use one listener thread with a poll loop, or a thread per client? Decide in an ADR
   with the HTTP and WebSocket crate choice. Any crate must pass `cargo deny check licenses`.
2. Do snapshots carry strings (radio title, scene name) at the full rate? Proposal: no. Strings go in the 1 Hz
   snapshot and in events only.
3. Should the sim-racing UDP feed use a published, stable schema from the start? Proposal: yes, version 1 with the
   field list in the file, so third-party gauges can rely on it.
4. Does a replay (F-33) record the player's raw inputs and rerun the physics, or record the state? Recording the
   state is simpler and survives code changes; recording inputs is smaller and needed for netcode debugging.
   Decide with [multiplayer reuse](multiplayer-reuse.md).
5. Which accessor gives the scene mode flags (F-06)? To confirm in `crates/nfsmw/src/scenes/world/mod.rs`.
