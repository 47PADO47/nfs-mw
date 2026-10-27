# Telemetry: control endpoints

The opt-in write side of the telemetry server, and the command table it shares with the F12 console and, later, the
mod API (milestone 12). Companion of [features](features.md) (F-50 to F-55) and [transport and privacy](transport-and-privacy.md)
(the token, the origin check). Design, not built.

## 1. Principles

1. **Read-only by default.** When the server is on, every endpoint is GET. Starting recordings exists only when the
   user sets `telemetry.control = record` or `write`; game commands exist only with `write`. Neither exists on a
   non-loopback bind (no TLS, see [ADR 0007](../../decisions/0007-telemetry-server.md), section 6).
2. **One command table.** A write is a console command line, parsed and run by the same dispatcher the F12 console
   uses (`crates/nfsmw/src/scenes/world/commands.rs`, function `run`). The server adds no second implementation of
   teleport, spawn or reset. A fix to the console fixes the server too.
3. **Game state only.** Writes change the running game. They never write settings files, save files, the garage, or
   anything on disk. A command that would make a change persist is not in the table (section 4).
4. **Applied on the game thread, at a frame boundary.** A write waits in the command queue and runs in the sampler's
   step 6 (see [transport and privacy](transport-and-privacy.md), section 1). The server never touches the world.
5. **Audited.** Every accepted or refused write is written to the log and raised as an event.

## 2. Enabling it

| Setting | Values | Default | Effect |
|---|---|---|---|
| `telemetry.enabled` | `false`, `true` | `false` | the server runs; all endpoints are GET |
| `telemetry.control` | `off`, `record`, `write` | `off` | `record` accepts the recording endpoints ([api](api.md) section 2.4) with a valid token; `write` also accepts `POST /v1/control` |
| `telemetry.bind` | address | `127.0.0.1` | where the listener binds; a non-loopback address forces `control` to `off` |
| `telemetry.port` | port | 7708 (proposal) | the listener port |
| `telemetry.origins` | list of origins | empty | extra origins allowed for WebSocket upgrades, writes and CORS |
| `telemetry.token` | (none) | created per launch | the write token is generated at launch; no setting holds a token |

Settings come from the layered settings (command line, environment, config file, defaults). Changing
`telemetry.control` takes effect at the next launch, so a running session cannot turn writes on through the server
itself. The status page then reads `control: off`, `record` or `write`. A command's scope is `control` and a
recording's scope is `record` in [api](api.md) section 1.

## 3. The write endpoint

```
POST /v1/control
Authorization: Bearer <token>
Origin: http://127.0.0.1:<port>      (or an allowlisted origin)
Content-Type: application/json

{"command": "player.teleport", "args": {"x": 120, "y": -40}}
```

`command` is the server name from the table in section 4 (not the console name). `args` is an object of named,
typed values checked against the command's schema; the server turns it into the console line (`tp 120 -40`) itself.

Response, `202 Accepted`:

```
{"id": 17, "state": "queued"}
```

Then `GET /v1/control/17` returns `{"id": 17, "state": "done", "ok": true, "text": "looking for a road near (120, -40)"}`
or `{"state": "done", "ok": false, "text": "not driving (use the drive command, or goto for the camera)"}`. The text
is the command's own message, unchanged.

Refusals before the queue:

| Code | When |
|---|---|
| `401` | token missing or wrong |
| `403` | `Origin` not allowed, or `telemetry.control` is not `write` (`scope_disabled`), or the bind is not loopback |
| `400` | command unknown, or the arguments do not match the schema (the table gives the schema); unknown keys are refused |
| `429` | more than 10 accepted writes in one second |
| `503` | the command queue is full (`busy`) |

Refusals after the queue (the command ran and said no, such as "not driving") come back as `ok: false` with the
command's text. They are not retried by the server.

## 4. The command table (first version)

Each entry names the console command it calls, its arguments, the scene it needs and whether it changes persistent
state. Only `persists: false` entries are exposed.

| Server command | Calls | Arguments (from the console usage) | Needs | Exposed | Notes |
|---|---|---|---|---|---|
| `clock.set` | new `clock` console command, which calls `DayClock` `configure` | `hour` (0 to 24), `seconds_per_hour` (10 to 3600, optional) | world | P1 | `configure` in `crates/nfsmw/src/scenes/world/day.rs` wraps the hour, clamps the speed, clears any forced time and resets the cycle's seconds to 0 |
| `player.teleport` | `tp` | `x`, `y` | driving | P1 | `tp <x> <y>` respawns the car near a road; `respawn_near` decides the spot |
| `camera.goto` | `goto` | `x`, `y`, `height` (optional) | world | P1 | moves the free camera; `goto` cuts the view |
| `car.reset` | `reset` | none | driving | P1 | |
| `car.repair` | `repair` | none | driving | P1 | |
| `car.damage` | `damage` | `zone` (0 to 9 or a zone name), `severity` (0 to 6), `glass` (optional flag) | driving | P1 | usage string in `commands.rs`: `damage <zone 0-9 or DZ_NAME> <severity 0-6> [glass]` |
| `traffic.set` | `traffic` | `count` or `off` | world | P1 | `traffic <count>|off|status|warmup <seconds>`; `status` is read-only and stays a GET snapshot field |
| `pursuit.start` | `pursuit` | `heat` (1 to 10) | driving | P1 | `pursuit <heat 1-10>|off|status` in `ai/commands.rs` |
| `pursuit.stop` | `pursuit` | `off` | driving | P1 | ends the chase with its own end reason |
| `roadblock.spawn` | `roadblock` | none | driving, a pursuit | P1 | `roadblock` in `ai/commands.rs` |
| `drive.car` | `drive` | `car` name | world | P2 (F-54) | changes the car; left out until the swap is tested |
| `race.start` | not built | | | P2 | after milestone 7 |

Not exposed, with the reason:

| Console command | Reason |
|---|---|
| `parts`, garage | changes the saved car |
| `settings` (`settings_cmd.rs`) | writes the config file |
| `props`, `tire-effects`, `vehicle-effects`, `exhaust-flames` | cosmetic debug switches; the server can read their status later |
| `debug` | diagnostic toggles, not game state |
| `freecam`, `pos`, `drive` without arguments | read or toggle; `pos` is a snapshot field instead |

## 5. Rules for each write

- **Scene check.** The command runs only in the scene it needs. Otherwise its own refusal text is returned (for
  example `not driving` for `tp`).
- **Pause and loading.** The command queue is drained every frame, including paused frames, so a write made while
  paused runs then (the scene's own checks still decide). A `clock.set` during pause changes the stored clock; the
  sun shows the new hour from the next frame. Player headlights retain their manual state.
- **Ordering.** Writes run in arrival order at the frame boundary. Two writes in one frame run in order.
- **Rate.** At most 10 accepted writes per second (section 3).
- **No free text.** The server sends a command name and an argument list. It never builds a console line from
  user strings and never runs shell commands or paths. Argument types are checked against the schema before queueing.
- **Results are the command's own.** The server does not invent success text.
- **Events and logs.** Each write raises `command.accepted` or `command.refused` (with the command id and the reason)
  and writes one log line. The event is what overlays use to show "time set to 22:00" for instance.

## 6. Sequencing with the game

- A write made while a pursuit runs changes it at once: `pursuit.stop` ends the chase through the pursuit's own end
  path (EndReason), so the HUD and the camera react the same way they would to the console.
- `clock.set` is the first write that changes a visible state with no gameplay side effect. It needs a test that the
  sun follows the new hour; player headlights retain their manual state.
- The writes never bypass the scene's checks. The server adds no check of its own beyond the schema, the scene
  requirement and the rate limit; whatever the console would refuse, the server refuses with the same text.

## 7. Tie to the mod API (milestone 12)

- The command table is the first version of the mod API's command surface. A script calls the same entries by name
  and with the same schema. The server and the scripts therefore share one list, one set of refusals and one audit
  trail.
- Each entry carries `persists` and `needs` (the scene) as data, so a future script host can check them before
  calling. A mod cannot call a `persists: true` entry through the same path without its own permission. This is a
  rule to design with the mod API, not something this server enforces alone.
- `GET /v1/commands` publishes the table with schemas and a `schema` number, so a dashboard or an overlay can build its
  controls from it without a hard-coded list.
- Scripted sequences (for example "set 22:00, traffic 40, pursuit heat 5") are the mod API's job. The server only
  runs one command at a time from a client.

## 8. Open questions

1. Should `clock.set` also be a console command for the F12 console, or only the server? Proposal: both, through one
   `clock` command, because the console is already the place for debug changes.
2. Does a refused write (for example `not driving`) count toward the rate limit? Proposal: no; only accepted writes count.
3. How is the token shown to a user who wants to script a write? Proposal: a console line `telemetry token` that
   prints it once to the log file, not to the screen. Decide with the status page.
