# Telemetry: transport, budget and privacy

How the server is built around the game loop, what it serves, what it costs, and what it never exposes. Companion
of [features](features.md) and [channels](channels.md). Design, not built. New design (no source to copy).

## 1. Process layout

The game runs an ECS shell (ADR 0001, `docs/decisions/0001-bevy.md`). The telemetry server is one more thread group
beside it. Three pieces, and no piece waits on another.

| Piece | Runs on | Does | Blocks? |
|---|---|---|---|
| Sampler | game thread, one ECS system after the sim step | reads the channels, fills one fixed-size snapshot value, publishes it, drains the event ring, drains the command queue | never; a few microseconds of copying |
| Hand-off | shared memory | triple buffer for snapshots; bounded event ring; bounded command queue | lock-free (atomics only) |
| Server | its own thread(s) | listens on the socket, serialises the latest snapshot to JSON, feeds streams, writes recording files, runs the status page | yes, freely; it is not the game |

**Sampler, precisely.**

1. Early return if the server is off (one `bool` resource read). Zero work when off.
2. If the sim did not step this frame (paused, loading, zero `sim_dt`), skip steps 3 to 5. The last snapshot stays
   served with `running: false`. Step 6 still runs, so a write made while paused is applied.
3. Copy the channel values into a `Snapshot` value: plain numbers and fixed arrays, `Copy`, no heap. Strings (radio
   title, scene name) are not in this value; they go in events and the 1 Hz slow block (section 2.3).
4. Write it into the free slot of a triple buffer and publish the slot index with one atomic swap.
5. Push the events raised this frame into the event ring (section 2.4). The server drains the ring on its own thread.
6. Pop at most a few commands from the command queue and apply them at this frame boundary (see [control](control.md)).

**Triple buffer.** Three slots. The writer always owns one that no reader holds; readers take the most recent
published slot and never wait. A reader that is slow simply sees a newer snapshot next time. No mutex is taken on
the game thread.

**Event ring.** Bounded (1024 entries of a fixed-size record, about 64 KB). If the server falls behind, the oldest
entries are overwritten and the drop counter goes up. The game thread never waits for space. `seq` numbers reveal
drops to the consumer: a gap means the consumer missed events.

**Command queue.** Bounded (64 entries). A full queue refuses the new command with `busy`. Results come back in a
reply slot keyed by the command id.

**Server threads.** One listener thread that accepts connections; each accepted connection is handled on a worker
from a small pool, or a single poll loop (open question 1 of [features](features.md)). Recording writes happen on a
server-side writer thread with a buffered file, so disk speed never reaches the game thread.

**Library split.** Engine-generic parts go in `libs/` with no NFS:MW names and no Bevy: the triple buffer, the event
ring, the command queue, the JSON-lines writer, the HTTP and WebSocket server. The channel catalogue, the code that
fills the snapshot from the scene, the redaction rules and the command table go in `crates/nfsmw` (for example a
`telemetry` module next to `devtools`). `crates/` depends on `libs/`, never the reverse. The libraries stay Bevy-free.

## 2. Endpoints and formats

The API is versioned in the path (`/v1`). Every response is JSON with a `schema` number (see section 2.5).

### 2.1 REST (GET, read-only)

| Path | Returns | Served from |
|---|---|---|
| `GET /v1/status` | server version, game build, sim tick, `running`, session id, recording state, drop counters, enabled channel groups | the last snapshot and counters |
| `GET /v1/channels` | the catalogue: each channel's id, kind, unit, range, rate, status | a constant table |
| `GET /v1/snapshot?groups=veh,wheel,pur` | the latest snapshot for the chosen groups (default: all built groups) | the triple buffer |
| `GET /v1/events?since=<seq>&limit=<n>` | events after `seq`, oldest first, at most `limit` (default 100, max 1000) | the event ring's history window |
| `GET /v1/commands` | the command table with argument schemas (see [control](control.md)) | a constant table |
| `GET /v1/commands/<id>` | the result of an earlier write | the reply slot |
| `GET /v1/recordings` | file names, sizes, start and stop times of recordings in the data folder | the folder listing (names only) |

A snapshot GET costs one copy of a few kilobytes plus the JSON. Nothing waits for the game. Responses carry
`Cache-Control: no-store`.

### 2.2 Streams (WebSocket, read-only)

`GET /v1/stream` upgrades to a WebSocket. The client sends one subscription message:

```
{"groups": ["veh", "wheel"], "rate_hz": 30, "events": true}
```

The server sends one frame per sample at the requested rate, capped at the channel's maximum (60 Hz for vehicle
groups). Each frame has the sim `tick`, `t`, the frame index and the values. Events arrive as they are raised, with
their `seq`. A subscriber that cannot keep up is dropped to the latest frame; the server never queues without a bound.

The WebSocket upgrade checks the `Origin` header (section 4.4).

### 2.3 Slow block

Strings and infrequent values (scene name, radio title and artist, camera view name, settings, adapter) are sent in
a slow block at 1 Hz, and in events when they change. They never ride the stream rate.

### 2.4 Events

An event is a fixed record in the ring, then JSON on the wire:

```
{"seq": 1842, "t": 97.250, "tick": 5835, "type": "gear.changed", "from": 3, "to": 4}
```

`t` is sim seconds since the session started (it stops during pause). `tick` is the sim tick. Wall-clock time
appears only in the status page and the recording header, not in events, so that replays and comparisons do not
depend on the clock on the PC.

### 2.5 Schema and versioning

- `schema` is an integer in every response and every recording header. It goes up when a channel's meaning or unit
  changes. New channels do not bump it; consumers ignore unknown keys.
- Channel ids are stable strings (`veh.speed`); the catalogue says which are removed or renamed.
- `tick` and `seq` are in every snapshot and event from the first version. Multiplayer later needs them
  (see [multiplayer reuse](multiplayer-reuse.md)).

### 2.6 Formats for files and dashboards

- **Recordings**: JSON lines. The first line is a header (`schema`, game build, session id, channel list with units,
  the rate, start time as wall clock once). Then one line per sample: `t`, `tick`, `frame`, the values in header
  order. Events are interleaved as their own lines with `"kind": "event"`. Size: 60 channels at 60 Hz is about 0.6
  MB per minute of JSON at 60 Hz (estimate, to measure); the recorder lets the user choose fewer channels or a lower rate.
- **CSV export**: converted from a recording by a command, not live: one column per channel, one row per sample
  at the chosen rate, events in a separate file.
- **UDP sim-racing feed (F-43)**: one datagram per sample at the chosen rate, JSON with a `schema` field and a fixed
  key order. Datagrams are at most 1200 bytes; a group that does not fit is split and marked. The sender targets
  `127.0.0.1` by default; any other destination is an explicit setting.

### 2.7 Rough bandwidth

Vehicle plus wheels at 60 Hz is around 60 channels times about 12 bytes of JSON per sample, so 40 to 50 KB/s per
stream. That is local traffic and acceptable. Subscribers choose lower rates for dashboards (10 to 30 Hz).

## 3. Budget

**Target.** The sampler's cost on the game thread is at most 0.2 ms per frame, at the 99th percentile, with the drive
running, the server on and one WebSocket stream subscribed. At 60 fps a frame is 16.7 ms, so this is about 1.2 %.
With the server off the cost is one branch.

**What the 0.2 ms covers.** Steps 1 to 6 of section 1 on the game thread: the copy, one atomic publish, the event
drain and at most a few command pops. Serialisation to JSON, sockets and file writes do not run on the game thread.

**Measuring it.**

- A benchmark (F-72) in `crates/nfsmw` drives a scripted drive, samples the telemetry system's own timer around its
  step, and reports the mean and the 99th percentile. It runs on the reference rig in `docs/benchmarks/`. It is not a
  unit test, because hosts vary.
- The advanced overlay gets a line `telemetry` with the last sampler cost in ms and the drop count (F-73). The
  measure uses the same clock as the frame metrics in `crates/nfsmw/src/devtools/metrics.rs`.

**Frame rate independence.** Snapshots advance with the sim tick, not the render frame. With the render at 240 fps the
sampler still runs 60 times per second of game time; the cost per frame is the same.

**Memory.** Triple buffer: three snapshot values, a few kilobytes. Event ring: about 64 KB. Command queue: a few
hundred bytes per entry at most. Recording buffer: 64 KB, flushed by the writer thread.

**Back-pressure.** Slow clients are dropped to the latest frame, slow files lose nothing (the writer blocks on its own
thread, and if its queue fills the recording stops with a `recording.stopped` event naming the reason).

## 4. Privacy

### 4.1 Defaults

- **Off.** No socket is opened and no file is written until the user turns the server on (`telemetry.enabled`,
  `NFSMW_TELEMETRY` in the environment, or the config file; the layered settings in `crates/nfsmw/src/settings/`).
- **Local only.** The listener binds to `127.0.0.1`. A LAN address needs `telemetry.bind` set explicitly; the
  console prints a warning when it is set. There is no IPv6 wildcard default.
- **No outbound traffic.** The server never connects out. No update check, no analytics, no crash report, no DNS
  lookup. The UDP feed sends to the configured target only, default `127.0.0.1`.
- **Read-only.** Every endpoint in section 2 is GET. Writes are the subject of [control](control.md) and are off
  unless the user enables them.

### 4.2 What is never exported

- Absolute paths: the game install path, the user profile, the config file location.
  Recording file names are given without their folder.
- The PC name, the user name, the account or email of the user, any network address other than the listener's own.
- Save file contents and save slot names (`libs/save-store`). The aliases are not exported.
- Credentials, tokens and any settings key that holds one. The write token is never logged or exported.

### 4.3 What needs an explicit opt-in

- `perf.adapter` (the graphics card name). It is hardware, not personal data, but it is off by default because it
  makes a status page identify a machine.
- Recordings. Nothing is recorded until the user starts one. Recordings stay in the per-user data folder.
- A LAN bind (section 4.1).
- Writes (section 4.5 and [control](control.md)).

### 4.4 Origins and browsers

A web page the user opens in a browser can try to read `http://127.0.0.1:<port>`. Two rules stop that:

- **WebSocket upgrades** are accepted only from an `Origin` on an allowlist (by default the server's own origin
  for the OBS overlay page, plus the origins the user adds). Any other origin gets `403`. This stops a page on another
  site from reading the live stream (cross-site WebSocket hijacking).
- **REST responses** carry no `Access-Control-Allow-Origin` header by default, so a page on another origin cannot
  read them with `fetch`. The overlay page is served from the same origin and needs no CORS.

Writes also require the `Origin` check and the token; a cross-origin `POST` without the token is refused before
any body is read.

### 4.5 Write token

- A random 128-bit token, created at each launch, held in memory and written to a file in the per-user data folder
  that the OS restricts to the user where it can.
- Required as `Authorization: Bearer <token>` on every write. Read endpoints need no token on `127.0.0.1`.
- Never logged, never in a recording, never in the status page.

### 4.6 Minimisation

- Channels are grouped (section 2 of [channels](channels.md)). A subscriber asks for groups; it gets nothing else.
- Redaction is in the channel catalogue (F-63): a channel marked redactable is removed from the output when the
  user sets the redaction option, and the catalogue, not the client, decides.
- Recordings hold game state only. The same redaction rules apply to them, so a recording shared by the user carries
  no paths or account data.

### 4.7 Failure modes

- Bind error (port in use, no permission): the server logs one warning with the port, the game carries on, and the
  status line says `telemetry: off (bind failed)`.
- A malformed request gets `400` and is counted; it never reaches the game thread.
- A panic inside the server thread stops the server, not the game: the sampler sees the server gone and keeps
  filling the triple buffer, which costs nothing when nothing reads it.
