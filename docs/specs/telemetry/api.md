# Telemetry: the HTTP API

The endpoint catalogue of the telemetry server: resources, methods, parameters, response shapes, stream topics, the
error model, pagination and an OpenAPI 3.1 sketch. Decisions behind it (stack, auth, limits, versioning) are in
[ADR 0007](../../decisions/0007-telemetry-server.md). What each channel means is in [channels](channels.md); the
write side is in [control](control.md). Design, not built. New design (no source to copy).

## 1. Conventions

**Base.** `http://127.0.0.1:7708/v1` (the port is a setting; 7708 is the proposal, not registered). Every body is
JSON (`application/json`), UTF-8, no compression. Responses carry `Cache-Control: no-store`, except the constant
tables (section 2.1), which carry an `ETag` and answer `If-None-Match` with `304`.

**Auth scopes.** Each endpoint lists the scope it needs.

| Scope | Needs | When |
|---|---|---|
| `open` | nothing | loopback bind |
| `read` | `Authorization: Bearer <token>` | every request when the bind is not loopback |
| `record` | token, and `telemetry.control` is `record` or `write` | recording endpoints; loopback only |
| `control` | token, and `telemetry.control` is `write` | game commands; loopback only |

A scope the server is not configured for answers `403` with code `scope_disabled`. Origin, `Host` and CORS rules are
in ADR 0007, section 6; a bad `Host` answers `421`.

**Time.** `t` is sim seconds since the session started (stops while paused); `tick` is the sim tick (60 per second
of sim time, stated in `tick_hz`). Wall-clock appears only in `started_at` and `stopped_at` fields, in ISO 8601 UTC.

**Ids.** `session` is 16 hex characters, new at each launch. `entity` is an unsigned 32-bit integer: the low 8 bits are the
session wire id (0 is the local player in single player), the upper 24 a generation counter that rises when a wire
id is reused ([multiplayer reuse](multiplayer-reuse.md) section 6). Recording ids are `<session>-<n>`. Channel ids are the stable strings of
[channels](channels.md) (`veh.speed`, `wheel.2.slip`).

**Priorities.** P0, P1, P2 as in [features](features.md).

## 2. Endpoints

### 2.1 Meta and tables

| Method, path | P | Scope | Returns |
|---|---|---|---|
| `GET /v1/status` | P0 | open | Status object (below) |
| `GET /v1/openapi.json` | P1 | open | The OpenAPI 3.1 document generated from the types; `ETag` |
| `GET /v1/channels` | P0 | open | Channel catalogue. Query: `group`, `kind` (`snapshot`, `stream`, `event`), `status` (`built`, `partial`, `not_built`, `removed`). `ETag` |
| `GET /v1/channels/{channel}` | P1 | open | One channel: `id`, `group`, `kind`, `rate_hz`, `unit`, `range`, `status`, `redactable`, `since_schema`. `404` if unknown |
| `GET /v1/commands` | P1 | open | The command table with argument schemas ([control](control.md) section 4). `ETag` |
| `GET /v1/commands/{name}` | P1 | open | One entry: `name`, `args` (JSON Schema), `needs` (scene), `persists` (always false), `exposed` |

Status object (all fields always present; unknown fields are added over time):

```
{
  "schema": 1, "api": "1.0.0", "game_build": "…", "server_version": "…",
  "session": "9f3a1c2e5b7d0a44", "tick": 5835, "t": 97.25, "tick_hz": 60, "running": true,
  "health": "ok",
  "control": "off",
  "bind": "127.0.0.1:7708",
  "groups": ["veh", "wheel", "dmg", "pur", "ai", "clock", "audio", "cam", "perf", "settings", "mode"],
  "recording": {"state": "idle", "id": null},
  "counters": {"events_dropped": 0, "frames_skipped": 0, "streams": 1, "requests": 412, "rejected": 3},
  "sampler_ms": {"last": 0.011, "p99": 0.030},
  "started_at": "2026-10-16T14:30:22Z"
}
```

`health` is `ok`, `degraded` (a server thread died and restarted) or `crashed` (the server is off). `control` is
`off`, `record` or `write`. The paths of the install, the user name and the config file are never in this object.

### 2.2 Live state

| Method, path | P | Scope | Returns |
|---|---|---|---|
| `GET /v1/snapshot` | P0 | open | The latest frame. Query: `groups` (comma list, default all built groups), `redact` (`true` applies the redaction rules; default per setting) |
| `GET /v1/snapshot/{group}` | P0 | open | One group, same envelope |
| `GET /v1/events` | P0 | open | Events after `since`. Query: `since` (seq, default 0 = oldest held), `limit` (1 to 1000, default 100), `types` (comma list) |
| `GET /v1/entities` | P1 | open | Entities in the session: `[{"entity": 1, "kind": "player", "peer": 0}]`. Kinds: `player`, `traffic`, `cop`, `racer`, `remote` |
| `GET /v1/entities/{entity}` | P2 | open | The vehicle-state subset of one entity (F-81). `404` if gone |

Frame envelope (snapshot GET, and the body of a stream `frame` message):

```
{
  "schema": 1, "session": "9f3a1c2e5b7d0a44", "tick": 5835, "t": 97.25, "running": true,
  "groups": {
    "veh":   {"speed": 31.2, "rpm": 6120, "gear": 4, "shifting": false, "position": [120.4, 2.1, -40.0]},
    "wheel": [{"on_ground": true, "slip": 0.2}, {"on_ground": true, "slip": 0.1}, {}, {}],
    "pur":   {"active": true, "heat": {"value": 2.4, "level": 3}}
  }
}
```

Keys inside a group are the channel ids without the group prefix; `wheel` is an array indexed 0 to 3 (library order).
A group the server has not built is absent, never null. Strings (radio title, scene name) are in the group `slow`,
refreshed at 1 Hz; ask for it with `groups=slow`.

Events list: `{"events": [Event…], "next_since": 1850, "dropped": false}`. `dropped` is true when `since` is older
than the oldest event held (a gap), so a consumer knows it missed some. Event:

```
{"seq": 1842, "t": 97.25, "tick": 5835, "type": "gear.changed", "from": 3, "to": 4}
```

The server holds the latest 4096 events (history is on the server side; the game's ring only carries them across).

### 2.3 Streams

| Method, path | P | Scope | Transport |
|---|---|---|---|
| `GET /v1/stream` | P1 | open or read | WebSocket upgrade. Query (optional): `groups`, `rate_hz` (1 to 60), `events` (`0` or `1`) |
| `GET /v1/stream/sse` | P1 | open or read | `text/event-stream`, same query; each message is one SSE event named by its `msg` |

Topics are the groups of [channels](channels.md) (`veh`, `wheel`, `dmg`, `pur`, `ai`, `clock`, `audio`, `cam`,
`perf`, `settings`, `mode`, `slow`) plus the pseudo-topic `events`. Rates are decimated from 60 Hz and never exceed
a channel's maximum. A client that cannot keep up gets the latest frame and a `dropped` message; the server never
queues a stream without bound. At most 4 streams at once (`429` beyond).

Messages from the server (each has `msg`):

| `msg` | When | Fields |
|---|---|---|
| `hello` | first message | `schema`, `session`, `tick_hz`, `groups` granted, `rate_hz` granted, `events` |
| `frame` | at the granted rate while the sim steps | the frame envelope (section 2.2) |
| `event` | as raised, in `seq` order | `event`: an Event |
| `dropped` | frames skipped for a slow client | `frames`, `from_tick`, `to_tick` |
| `error` | a bad client message | an Error object; the stream stays open unless the code is fatal |
| `bye` | the server closes (shutdown, token gone) | `reason` |

WebSocket only, from the client (text messages, at most 16 KiB, JSON):

| `op` | Fields | Effect |
|---|---|---|
| `subscribe` | `groups`, `rate_hz`, `events` | replaces the subscription; the server answers with a new `hello` |
| `ping` | none | answered with `{"msg": "pong", "tick": …}` (application-level; WebSocket pings are also answered) |

Browsers send the token as the sub-protocol `nfsmw.token.<hex>` next to `nfsmw.v1` (ADR 0007, section 6). While the
game is paused no `frame` is sent; `status.running` is false and a `hello` is re-sent on resume.

### 2.4 Recordings

Recordings are JSON-lines files in the per-user data folder ([transport](transport-and-privacy.md) section 2.6).
The server exposes names without folders.

| Method, path | P | Scope | Does |
|---|---|---|---|
| `GET /v1/recordings` | P0 | open | List, newest first. Query: `limit` (1 to 200, default 50), `cursor`, `state` (`recording`, `stopped`, `failed`) |
| `POST /v1/recordings` | P0 | record | Start. Body below. `201` + `Location`; `409` if one is already running |
| `GET /v1/recordings/{id}` | P0 | open | Recording object. `404` if unknown |
| `PATCH /v1/recordings/{id}` | P0 | record | Body `{"state": "stopped"}`. Stopping a stopped recording is `200` (idempotent) |
| `GET /v1/recordings/{id}/data` | P0 | open | The samples. Query: `format` (`jsonl` default, `csv`), `part` (`samples` default, `events`; CSV only), `from_tick`, `to_tick`. Streamed, chunked; a read-only conversion, nothing is written to disk |
| `GET /v1/recordings/{id}/summary` | P1 | open | Session summary (F-32): top speed, max rpm, max heat, pursuits, busted count, infractions, distance, time driven |
| `DELETE /v1/recordings/{id}` | P2 | record | Deletes the file; `204`; `409` while recording |

Start body: `{"groups": ["veh","wheel"], "channels": ["pur.heat.level"], "rate_hz": 30, "max_seconds": 3600}`. At least
one of `groups`, `channels`; defaults `rate_hz` 60, `max_seconds` 3600, and a byte cap of 512 MiB. The recording stops with
a `recording.stopped` event naming the reason (`user`, `time`, `size`, `disk`, `shutdown`).

Recording object:

```
{"id": "9f3a1c2e5b7d0a44-1", "state": "stopped", "started_at": "2026-10-16T14:31:00Z",
 "stopped_at": "2026-10-16T14:41:00Z", "schema": 1, "rate_hz": 30, "first_tick": 3600, "last_tick": 39600,
 "samples": 18000, "bytes": 11534336, "channels": ["veh.speed", "veh.rpm"], "stop_reason": "user"}
```

**Pagination.** Cursor based, because recordings are added while a client pages. The list returns
`{"items": [...], "next_cursor": "…" | null}`; pass `next_cursor` back as `cursor`. A cursor is opaque and valid
for the session. Items added after the first page do not shift later pages. `limit` above the cap is clamped, not an
error.

### 2.5 Control

| Method, path | P | Scope | Does |
|---|---|---|---|
| `POST /v1/control` | P1 | control | Queues a command. Body `{"command": "player.teleport", "args": {"x": 120, "y": -40}}`. `202` `{"id": 17, "state": "queued"}` |
| `GET /v1/control/{id}` | P1 | open | `{"id": 17, "state": "done", "ok": true, "text": "…", "tick": 5840}`; `state` is `queued`, `done` or `expired` (results are kept for the latest 256 commands) |

Command names, arguments, refusals and the rate limit are in [control](control.md). `args` is an object checked
against the command's schema; unknown keys are `400`.

## 3. Errors

Every non-2xx response is `application/problem+json` (RFC 9457):

```
{"type": "urn:nfsmw:telemetry:error:rate_limited", "title": "Too many requests", "status": 429,
 "code": "rate_limited", "detail": "100 requests per second per client", "retry_after_s": 1}
```

`code` is the stable part (clients switch on it); `title` and `detail` are for people. Codes:

| Status | `code` | When |
|---|---|---|
| 400 | `bad_request`, `unknown_key`, `bad_argument` | malformed JSON, an unknown body key, an argument that fails its schema |
| 401 | `unauthorized` | token missing or wrong (the same answer for both) |
| 403 | `forbidden_origin`, `scope_disabled` | `Origin` not allowed; `telemetry.control` does not enable the scope; a write on a non-loopback bind |
| 404 | `not_found` | unknown path, channel, recording, entity or command |
| 405 | `method_not_allowed` | with an `Allow` header |
| 409 | `conflict` | a recording is already running; delete while recording |
| 411, 413, 431 | `length_required`, `payload_too_large`, `header_too_large` | request size limits |
| 421 | `misdirected_host` | `Host` not on the allowlist |
| 429 | `rate_limited`, `too_many_streams` | limits in ADR 0007, section 7; `Retry-After` set |
| 501 | `not_implemented` | `Transfer-Encoding` on a request |
| 503 | `busy`, `unavailable` | command queue full; connection cap; server shutting down |

A command that ran and was refused by the game ("not driving") is not an HTTP error: it is a `200` result with
`ok: false` ([control](control.md) section 3).

## 4. Rules for adding to the API

1. A new channel: add it to the game's channel table and the group type. It appears in `/v1/channels`, the snapshot
   and the streams with no other change. No version bump.
2. A new endpoint: write the handler with `#[utoipa::path]`, add the route and a row here. The drift tests fail until
   all three agree.
3. A change of a channel's meaning or unit: new `schema`, the old id marked `removed`, a new id.
4. Remote players (milestone 10 and 11) appear as entities; the same group types describe them.

## 5. OpenAPI 3.1 sketch

The generated document is authoritative; this sketch is the reviewed shape. A test compares its paths, methods and
component names with the generated document ([ADR 0007](../../decisions/0007-telemetry-server.md), section 5).
Responses and schemas are abbreviated where the shape repeats.

```yaml
openapi: 3.1.0
info:
  title: NFS:MW telemetry
  version: 1.0.0
  description: Optional local telemetry server. Off by default; loopback by default.
  license: {name: MIT OR Apache-2.0}
  x-schema: 1
servers:
  - url: http://127.0.0.1:7708
security: []
tags: [{name: meta}, {name: live}, {name: stream}, {name: recordings}, {name: control}]
paths:
  /v1/status:
    get: {tags: [meta], operationId: getStatus, responses: {"200": {$ref: "#/components/responses/Status"}}}
  /v1/openapi.json:
    get: {tags: [meta], operationId: getOpenApi, responses: {"200": {description: This document}}}
  /v1/channels:
    get:
      tags: [meta]
      operationId: listChannels
      parameters:
        - {name: group, in: query, schema: {type: string}}
        - {name: kind, in: query, schema: {enum: [snapshot, stream, event]}}
        - {name: status, in: query, schema: {enum: [built, partial, not_built, removed]}}
      responses: {"200": {$ref: "#/components/responses/Channels"}}
  /v1/channels/{channel}:
    get:
      tags: [meta]
      operationId: getChannel
      parameters: [{name: channel, in: path, required: true, schema: {type: string}}]
      responses: {"200": {description: One channel}, "404": {$ref: "#/components/responses/Problem"}}
  /v1/snapshot:
    get:
      tags: [live]
      operationId: getSnapshot
      parameters:
        - {name: groups, in: query, schema: {type: string}, description: Comma list of groups}
        - {name: redact, in: query, schema: {type: boolean}}
      responses: {"200": {$ref: "#/components/responses/Frame"}}
  /v1/snapshot/{group}:
    get:
      tags: [live]
      operationId: getSnapshotGroup
      parameters: [{name: group, in: path, required: true, schema: {type: string}}]
      responses: {"200": {$ref: "#/components/responses/Frame"}, "404": {$ref: "#/components/responses/Problem"}}
  /v1/events:
    get:
      tags: [live]
      operationId: listEvents
      parameters:
        - {name: since, in: query, schema: {type: integer, minimum: 0, default: 0}}
        - {name: limit, in: query, schema: {type: integer, minimum: 1, maximum: 1000, default: 100}}
        - {name: types, in: query, schema: {type: string}}
      responses: {"200": {description: Events, content: {application/json: {schema: {$ref: "#/components/schemas/EventList"}}}}}
  /v1/entities:
    get: {tags: [live], operationId: listEntities, responses: {"200": {description: Entities}}}
  /v1/stream:
    get:
      tags: [stream]
      operationId: openStream
      description: WebSocket upgrade. Server messages are StreamMessage; client messages are ClientOp.
      parameters:
        - {name: groups, in: query, schema: {type: string}}
        - {name: rate_hz, in: query, schema: {type: integer, minimum: 1, maximum: 60}}
        - {name: events, in: query, schema: {enum: [0, 1]}}
      responses: {"101": {description: Switching protocols}, "429": {$ref: "#/components/responses/Problem"}}
      x-stream: {server: "#/components/schemas/StreamMessage", client: "#/components/schemas/ClientOp"}
  /v1/stream/sse:
    get:
      tags: [stream]
      operationId: openSse
      responses: {"200": {description: Server-sent events, content: {text/event-stream: {schema: {$ref: "#/components/schemas/StreamMessage"}}}}}
  /v1/recordings:
    get:
      tags: [recordings]
      operationId: listRecordings
      parameters:
        - {name: limit, in: query, schema: {type: integer, minimum: 1, maximum: 200, default: 50}}
        - {name: cursor, in: query, schema: {type: string}}
        - {name: state, in: query, schema: {enum: [recording, stopped, failed]}}
      responses: {"200": {description: A page, content: {application/json: {schema: {$ref: "#/components/schemas/RecordingPage"}}}}}
    post:
      tags: [recordings]
      operationId: startRecording
      security: [{bearer: []}]
      requestBody: {required: true, content: {application/json: {schema: {$ref: "#/components/schemas/RecordingStart"}}}}
      responses:
        "201": {description: Started, headers: {Location: {schema: {type: string}}}}
        "409": {$ref: "#/components/responses/Problem"}
  /v1/recordings/{id}:
    parameters: [{name: id, in: path, required: true, schema: {type: string}}]
    get: {tags: [recordings], operationId: getRecording, responses: {"200": {description: Recording}, "404": {$ref: "#/components/responses/Problem"}}}
    patch: {tags: [recordings], operationId: stopRecording, security: [{bearer: []}], responses: {"200": {description: Stopped}}}
    delete: {tags: [recordings], operationId: deleteRecording, security: [{bearer: []}], responses: {"204": {description: Deleted}}}
  /v1/recordings/{id}/data:
    get:
      tags: [recordings]
      operationId: getRecordingData
      parameters:
        - {name: id, in: path, required: true, schema: {type: string}}
        - {name: format, in: query, schema: {enum: [jsonl, csv], default: jsonl}}
        - {name: part, in: query, schema: {enum: [samples, events], default: samples}}
        - {name: from_tick, in: query, schema: {type: integer}}
        - {name: to_tick, in: query, schema: {type: integer}}
      responses: {"200": {description: Samples, content: {application/x-ndjson: {}, text/csv: {}}}}
  /v1/recordings/{id}/summary:
    get: {tags: [recordings], operationId: getRecordingSummary, parameters: [{name: id, in: path, required: true, schema: {type: string}}], responses: {"200": {description: Summary}}}
  /v1/commands:
    get: {tags: [control], operationId: listCommands, responses: {"200": {description: Command table}}}
  /v1/commands/{name}:
    get: {tags: [control], operationId: getCommand, parameters: [{name: name, in: path, required: true, schema: {type: string}}], responses: {"200": {description: One command}}}
  /v1/control:
    post:
      tags: [control]
      operationId: postControl
      security: [{bearer: []}]
      requestBody: {required: true, content: {application/json: {schema: {$ref: "#/components/schemas/ControlRequest"}}}}
      responses: {"202": {description: Queued}, "429": {$ref: "#/components/responses/Problem"}, "503": {$ref: "#/components/responses/Problem"}}
  /v1/control/{id}:
    get: {tags: [control], operationId: getControlResult, parameters: [{name: id, in: path, required: true, schema: {type: integer}}], responses: {"200": {description: Result}}}
components:
  securitySchemes:
    bearer: {type: http, scheme: bearer, description: Per-launch token; required for writes and on a non-loopback bind}
  responses:
    Status: {description: Server status, content: {application/json: {schema: {$ref: "#/components/schemas/Status"}}}}
    Channels: {description: Channel catalogue, content: {application/json: {schema: {$ref: "#/components/schemas/ChannelList"}}}}
    Frame: {description: A frame, content: {application/json: {schema: {$ref: "#/components/schemas/Frame"}}}}
    Problem: {description: Error, content: {application/problem+json: {schema: {$ref: "#/components/schemas/Problem"}}}}
  schemas:
    Frame:
      type: object
      required: [schema, session, tick, t, running, groups]
      properties:
        schema: {type: integer}
        session: {type: string}
        tick: {type: integer, format: int64}
        t: {type: number}
        running: {type: boolean}
        groups: {type: object, additionalProperties: true, description: Per-group objects generated from the game's types}
    Event:
      type: object
      required: [seq, t, tick, type]
      properties:
        seq: {type: integer, format: int64}
        t: {type: number}
        tick: {type: integer, format: int64}
        type: {type: string}
      additionalProperties: true
    EventList:
      type: object
      required: [events, next_since, dropped]
      properties:
        events: {type: array, items: {$ref: "#/components/schemas/Event"}}
        next_since: {type: integer}
        dropped: {type: boolean}
    StreamMessage:
      type: object
      required: [msg]
      properties: {msg: {enum: [hello, frame, event, dropped, error, bye]}}
      additionalProperties: true
    ClientOp:
      type: object
      required: [op]
      properties: {op: {enum: [subscribe, ping]}, groups: {type: array, items: {type: string}}, rate_hz: {type: integer}, events: {type: boolean}}
    RecordingStart:
      type: object
      properties:
        groups: {type: array, items: {type: string}}
        channels: {type: array, items: {type: string}}
        rate_hz: {type: integer, minimum: 1, maximum: 60, default: 60}
        max_seconds: {type: integer, default: 3600}
    RecordingPage:
      type: object
      required: [items, next_cursor]
      properties: {items: {type: array, items: {type: object}}, next_cursor: {type: [string, "null"]}}
    ControlRequest:
      type: object
      required: [command]
      properties: {command: {type: string}, args: {type: object}}
      additionalProperties: false
    Status: {type: object, required: [schema, api, session, health, control], additionalProperties: true}
    ChannelList: {type: object, required: [schema, items], additionalProperties: true}
    Problem:
      type: object
      required: [type, title, status, code]
      properties:
        type: {type: string}
        title: {type: string}
        status: {type: integer}
        code: {type: string}
        detail: {type: string}
        retry_after_s: {type: integer}
```
