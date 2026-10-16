# 0007 — The optional telemetry server

- **Status:** proposed (2026-10-16). Design only; nothing is built. Needs the owner's answers in "Owner decisions".
- **Milestone:** 13 (telemetry and API). Shared pieces are meant to be reused by milestones 10 and 11.
- **Specs:** [docs/specs/telemetry/](../specs/telemetry/README.md) holds the feature catalogue, channels, privacy,
  control table and the endpoint catalogue ([api.md](../specs/telemetry/api.md)).
- **Related:** [0001](0001-bevy.md) (ECS shell), [0004](0004-swappable-renderers.md) (the `libs/` rule and the
  optional-feature pattern), [0006](0006-multiplayer-netcode.md) (netcode, written in parallel). Both ADRs place the
  hand-off primitives in `libs/blackbox-net-core`. They differ on three points, listed in
  [multiplayer reuse](../specs/telemetry/multiplayer-reuse.md) section 9 (entity id width, a `core` / `netcode`
  feature split of the crate, QUIC); 0006 must agree with section 2 and section 11 of this one.

## Question

Milestone 13 asks for "websocket/server/something for telemetry/api info". The specs describe what to publish. This
ADR decides how it is built: which crates exist, which threads run, which HTTP stack and OpenAPI tooling, how it is
protected, and what it may cost the game. The design must be small and fast, optional (zero cost when off), easy to
extend with new channels and endpoints, and partly reusable for LAN and online play.

## Facts checked on 2026-10-16

Crate data is from the crates.io API on that day (version, licence, last release). Direct licences only; a full
`cargo deny check licenses` on the real dependency tree is part of the first implementation PR.

| Fact | Consequence |
|---|---|
| `Cargo.toml` sets `unsafe_code = "deny"` for every workspace crate. | No hand-written lock-free structure in our code. We use audited crates for the hand-off. |
| `deny.toml` allows MIT, MIT-0, Apache-2.0 (+LLVM), BSD-2/3, ISC, Zlib, Unicode-3.0, CC0, MPL-2.0, BSL-1.0, and `all-features = false` in `[graph]`. | Every candidate below passes by its direct licence. Dependencies of workspace members are checked with default features, so server crates must be default-on in `libs/` (the optional part is the `nfsmw` feature), or CI's `--all-features` job must cover them. |
| `Cargo.lock` already holds `triple_buffer 9.0.0` (MPL-2.0, via `kira`; its only dependency is `crossbeam-utils`), `crossbeam-queue 0.3.14`, `serde_json`, `rand 0.10.3`, `getrandom 0.4.3`. | The hand-off needs no new crate. |
| `Cargo.lock` holds `tokio 1.53.2`, but only as a stub whose sole dependency is `pin-project-lite` (no runtime or net features). `hyper`, `axum`, `tungstenite`, `httparse`, `tiny_http`, `utoipa` are not in the lock. | An async stack would be a new runtime beside Bevy's executors, not a reuse. |
| The release profile has no `panic = "abort"` (checked in `Cargo.toml`). | A panic in a server thread unwinds and can be contained. This is a rule to keep (section 10). |
| Bevy is the ECS shell (0001); the sim runs a fixed 60 Hz step. | The sampler is one ECS system; snapshots are tagged with the sim tick. |

### Candidates evaluated

| Option | Licence | Last release | Model | New crates (estimate, not measured) | Verdict |
|---|---|---|---|---|---|
| `tiny_http 0.12` | MIT OR Apache-2.0 | 2022-10 | blocking, request objects | 4 (`ascii`, `chunked_transfer`, `httpdate`, `log`) | Rejected as the base. No release for four years. Its upgrade hands back a boxed stream; as I recall the API it cannot set a read timeout or be cloned, which makes a WebSocket that both pushes and reads awkward [unconfirmed: check in the spike]. |
| `rouille 3.6` | MIT/Apache-2.0 | 2023-04 | blocking, on `tiny_http` | `tiny_http` plus ~5 | Rejected: same base, same age. |
| `axum 0.8` on `tokio` + `hyper 1` | MIT | 2026-04 | async, multi-thread runtime | about 30 to 45 (`hyper`, `hyper-util`, `http`, `http-body*`, `tower*`, `tokio` with runtime and net, `matchit`, `bytes`, `futures-*`, ...) | The most capable and best supported. Rejected for now: a second runtime and a large tree for a handful of local endpoints. Kept as the **upgrade path** (see "Revisit triggers"). |
| `hyper 1` alone | MIT | 2026-10 | async | 15 to 20 plus `tokio` features | Rejected: still needs tokio, and no router or WebSocket. |
| `actix-web 4` | MIT OR Apache-2.0 | 2026-10 | actor runtime on tokio | 40 or more (`actix-*`, `regex-lite`, `time`, `url`, ...) | Rejected: heaviest tree, own runtime model. |
| `may_minihttp 0.1` | MIT/Apache-2.0 | 2024-09 | stackful coroutines (`may`) | `may`, `httparse`, `bytes`, ... | Rejected: 0.1.x, about 33 thousand downloads, coroutine runtime with heavy internal `unsafe`. |
| **`std::net` + `httparse` + `tungstenite`** | MIT OR Apache-2.0 each | 2025-03 / 2026-07 | blocking, one thread per connection (capped) | `httparse`, `tungstenite`, `data-encoding`, `sha1`, `http` (the `handshake` feature); `bytes`, `thiserror`, `log` and `rand` are likely in Bevy's tree already [unconfirmed] | **Chosen.** Smallest tree that gives REST, WebSocket and SSE. We own about 400 lines of HTTP/1.1 framing with strict limits. |
| UDP sim-racing feed | std only | | `std::net::UdpSocket` | 0 | Chosen (F-43); no crate needed. |
| WebSocket alternatives: `fastwebsockets` (Apache-2.0, tokio), `soketto` (futures), `simple-websockets` (MIT, 0.1.6) | | | | | Rejected: async-bound or too young. `tungstenite 0.30` is sync, the most used (347 million downloads) and has `derive_accept_key` and `WebSocket::from_raw_socket`. |

### OpenAPI candidates

| Option | Licence | Verdict |
|---|---|---|
| `utoipa 6.0` | MIT OR Apache-2.0 | **Chosen.** OpenAPI 3.1 types, derive macros, framework-agnostic (works with our own router). Dependencies: `indexmap`, `serde`, `serde_json`, `utoipa-gen` (proc macro). |
| `aide 0.16-alpha` | MIT OR Apache-2.0 | Rejected: alpha, built around axum and `schemars`. |
| Hand-written spec only | n/a | Rejected as the source of truth: it drifts from the types. Kept as the **documentation** form in [api.md](../specs/telemetry/api.md), with a test that ties the two together (section 5). |

## Decision

### 1. Optional, zero cost when off

- `nfsmw` gets a cargo feature **`telemetry`, off by default**. With it off, none of `blackbox-telemetry`'s code is
  linked into the game, no system is added to the schedule, and no thread exists. With it on but
  `telemetry.enabled = false` (the runtime default), the cost is one `bool` read per frame ([transport](../specs/telemetry/transport-and-privacy.md) section 1).
- Release builds for users may enable the feature; that is the owner's call (Owner decision 1).

### 2. Crate split

| Crate | Where | Holds | Depends on |
|---|---|---|---|
| `blackbox-net-core` (new; 0006 adds its netcode modules) | `libs/` | The part telemetry uses, behind cargo feature `core` (default): ids (`SessionId`, `EntityId`), sim time (`SimTick`, tick rate given by the caller), `SessionToken` (OS random, constant-time check), bind and origin policy, the rate limiter, and the bounded hand-off types (`Latest<T>` over `triple_buffer`, `EventRing<T>` and `CommandQueue<T>` over `crossbeam-queue`). 0006's reliability, packing, crypto and transport modules sit behind feature `netcode`; telemetry never enables it | `getrandom`, `crossbeam-queue`, `triple_buffer`, `thiserror` |
| `blackbox-telemetry` (new) | `libs/` | Channel catalogue types (`ChannelDef`, `GroupDef`), the frame trait the game implements, event record, the HTTP/1.1 and WebSocket server, router, the OpenAPI assembler, the recorder (JSON lines) and CSV export, the status and error types, the UDP feed (plain `std::net::UdpSocket`, send allowlist, 1200-byte cap) | `blackbox-net-core` (`core`), `httparse`, `tungstenite`, `serde_json`, `utoipa` |
| game wiring | `crates/nfsmw/src/telemetry/` | The concrete frame (vehicle, wheels, damage, pursuit, ...), the sampler system, the channel table with real source symbols, the command table adapter, the settings keys | `blackbox-telemetry` behind the `telemetry` feature |

Rules: both new libs are Bevy-free and name no NFS:MW type, path or default. `blackbox-net-core` has no HTTP code,
so netcode pulls in no HTTP stack. Both go in `default-members` and `[workspace.dependencies]` like the other
libs (so their tests run in CI), and `cargo deny check licenses` covers them. If multiplayer later needs HTTP
(for a lobby list, say), the `http` module moves to `blackbox-net-core` behind a feature; the call sites do not
change.

The library is generic over the frame type. The game implements:

```
trait Frame: Copy + Send + 'static {
    fn tick(&self) -> SimTick;
    fn groups() -> &'static [GroupDef];
    fn write_group(&self, group: GroupId, out: &mut dyn std::io::Write) -> std::io::Result<()>;
}
```

`write_group` serialises one group as JSON into a caller-owned buffer, so the library can serve a chosen subset
without knowing any channel. The game also provides the catalogue (`&'static [ChannelDef]`) and the schemas of its
group types for the OpenAPI document.

### 3. Threads and the hand-off

Game thread: the sampler (one ECS system after the sim step). Server side: all in `blackbox-telemetry`.

| Thread | Count | Does |
|---|---|---|
| `telemetry-accept` | 1 | blocking `accept`, applies the connection caps, spawns a connection thread |
| `telemetry-pump` | 1 | wakes every 4 ms (`park_timeout`): drains the event ring into a history `VecDeque` (4096 events), drains command results into a map (256 latest), reads each new tick for the recorder |
| `telemetry-rec` | 0 or 1 | buffered writer for the recording file; exists only while recording |
| `telemetry-conn-N` | up to 16 | one HTTP connection (keep-alive) or one stream; stack 256 KiB |

Hand-off, all on safe crates (`unsafe_code` stays denied):

- **Snapshot:** `Latest<Frame>` (a triple buffer). One writer (game), one `Output` guarded by a server-side `Mutex`
  shared by connection threads. The game thread never touches that mutex. A reader clones the frame (a few KB,
  `Copy`) and releases it before serialising.
- **Events:** `EventRing`: `ArrayQueue` of 1024 fixed records, `force_push` overwrites the oldest and bumps a drop
  counter. The pump owns the drain. `seq` is assigned by the sampler, so a gap shows loss.
- **Commands:** `CommandQueue` of 64; a full queue refuses with `busy`. Results come back through a second bounded
  queue (game to pump), never through a lock the game thread takes.
- **Recorder tap:** a triple buffer keeps only the latest frame, so a recording would lose ticks if the pump slept
  past one. While a recording runs, the sampler also pushes each frame into a 256-entry `ArrayQueue` (about 4 s of
  slack). Off when not recording.
- **Shutdown:** an `AtomicBool`; the acceptor is woken by a self-connect; connection threads see the flag between
  frames and on timeouts. The game never waits for them at exit beyond a 200 ms join.

Why threads and not an async runtime: the server has at most about 20 live connections, all local, each doing
small blocking reads and writes. A thread per connection with hard caps is simpler to reason about than a runtime
and has no executor to tune. A WebSocket thread alternates a short read timeout (the frame period) with a write, so
one thread serves both directions.

### 4. HTTP stack

`std::net::TcpListener` + `httparse` for requests + `tungstenite` for WebSocket frames + `serde_json` for bodies.
Our HTTP/1.1 layer is deliberately a subset, written once with tests and a fuzz target:

- Methods `GET`, `HEAD`, `POST`, `PATCH`, `DELETE`, and `OPTIONS` only for allowlisted CORS origins.
- Request head at most 8 KiB and 48 headers; body at most 16 KiB, `Content-Length` only. `Transfer-Encoding` on a
  request gets `501`; a missing length on `POST` gets `411`. No pipelining beyond one request at a time.
- Timeouts: 5 s to read a head, 10 s to read a body, 15 s idle keep-alive, 5 s per write. These end slow-loris
  connections.
- Responses carry `Content-Length` (or chunked for SSE and downloads), `Cache-Control: no-store`,
  `X-Content-Type-Options: nosniff`, `Content-Security-Policy: default-src 'none'` on API responses.
- WebSocket: the layer checks `Origin`, `Host` and the token, derives the accept key (`derive_accept_key`), writes
  `101`, then hands the `TcpStream` to `WebSocket::from_raw_socket(stream, Role::Server, None)`. Close, ping and
  pong are handled by the library. Message size cap 16 KiB.
- SSE: `GET /v1/stream/sse`, a chunked `text/event-stream` response. It works with `curl` and with the browser
  `EventSource` on loopback. `EventSource` cannot send an `Authorization` header, so on a LAN bind browsers use
  WebSocket or `fetch`.
- No TLS in this ADR (see "Consequences" and the LAN rule in section 6).

### 5. OpenAPI

- `utoipa` derives `ToSchema` on the library's own types (status, error, channel, event, recording) and the game's
  group types, and `#[utoipa::path]` annotates each handler. The assembler merges them into one OpenAPI 3.1
  document served at `GET /v1/openapi.json`. No Swagger UI is bundled or loaded from a CDN (that would be an
  outbound request by the user's browser to a third party). The document is for generators and tools.
- WebSocket and SSE are not expressible in OpenAPI 3.1; they appear as `101` and `text/event-stream` responses with
  `x-stream` extension fields, and the message schemas are ordinary components.
- **Drift test.** A unit test builds the document and the router table and fails when a route has no operation or an
  operation has no route. A second test reads the YAML block in `docs/specs/telemetry/api.md`, compares its
  (path, method) pairs and its component names to the generated document, and fails on a difference. The sketch in
  the spec therefore cannot rot silently. A third test checks every `ChannelDef` id appears in `/v1/channels`.
- The generated JSON is not committed (the whitelist `.gitignore` would need `json`, and a generated file duplicates
  the source).

### 6. Exposure, auth, origins

| Rule | Detail |
|---|---|
| Bind | `127.0.0.1` by default. A non-loopback address needs `telemetry.bind` set; a console and log warning follows. IPv6 loopback `[::1]` is allowed on request. No wildcard default. |
| Token | 128 random bits from the OS (`getrandom`), hex, created at each launch, held in memory and written to a session file in the per-user data folder. Compared in constant time. Never logged, never in an event, a recording or the status page. |
| Loopback reads | No token needed. |
| Writes and recordings | `Authorization: Bearer <token>` always. Scope by setting `telemetry.control`: `off` (all GET), `record` (recording endpoints), `write` (game commands, implies `record`). |
| Non-loopback bind | The token is required on **every** request, reads included. **Writes and recordings are refused** (`control` is forced to `off`, with a startup warning), because there is no TLS and a bearer token would cross the LAN in clear. |
| WebSocket token | Browsers cannot set headers on a WebSocket. The token goes in `Sec-WebSocket-Protocol: nfsmw.v1, nfsmw.token.<hex>`; the server answers `nfsmw.v1` only and never echoes the token. Never in the URL (URLs are logged by proxies and browsers). |
| `Host` header | Must match an allowlist: `127.0.0.1:<port>`, `localhost:<port>`, `[::1]:<port>`, plus the bound LAN address. Anything else gets `421`. This stops **DNS rebinding**, where a hostile page makes its own name resolve to `127.0.0.1` and reads the API as same-origin. The existing privacy spec lacked this rule; it is added there. |
| `Origin` header | If present on a WebSocket upgrade or a non-GET request it must be on the allowlist (the server's own origin plus `telemetry.origins`), else `403`. |
| CORS | No `Access-Control-Allow-*` by default, so cross-origin pages cannot read responses. An origin listed in `telemetry.origins` gets reads and, with the token, writes (headers `Authorization, Content-Type` allowed on its preflight). `OPTIONS` from any other origin gets `403`. |
| Cross-site writes | `POST/PATCH/DELETE` must send `Content-Type: application/json` (forces a preflight) and a valid token. The token check runs before the body is read. |

### 7. Rate limits and resource caps

Defaults, all settings with sane clamps:

| Limit | Default | On exceed |
|---|---|---|
| Open connections | 16 | `503` with `Retry-After: 1` |
| Concurrent streams (WebSocket + SSE) | 4 | `429` |
| Reads per client IP | 100 per second, burst 200 (token bucket in `blackbox-net-core`) | `429` with `Retry-After` |
| Accepted writes | 10 per second ([control](../specs/telemetry/control.md)) | `429` |
| Stream rate | channel maximum (60 Hz), one frame in flight; a slow client gets the latest frame, never a queue | frames skipped, counted |
| `GET /v1/events` | `limit` at most 1000 | clamped |
| Recordings listing | `limit` at most 200 | clamped |
| Request head / body | 8 KiB / 16 KiB | `431` / `413` |

The limiter table is bounded (1024 peers, oldest evicted) so a scan cannot grow memory. On loopback every client
shares one IP; the limits are therefore a safety net, not a fairness tool.

### 8. Versioning and evolution

- **Path major version:** `/v1`. A breaking change adds `/v2` beside it; `/v1` stays for at least one released
  version after `/v2` ships and answers with `Deprecation` and `Sunset` headers.
- **`schema` integer** in every payload and recording header, as in the spec. It rises when a channel's meaning or
  unit changes. New channels, new optional fields and new endpoints do **not** bump it or `/v1`.
- **Rules for clients:** ignore unknown keys. **Rules for the server:** reject unknown keys in request bodies with
  `400` (catches typos); never reuse a channel id; a removed channel stays in the catalogue with
  `status: removed` for one schema.
- **OpenAPI `info.version`** is the API semver (`1.minor.patch`); `x-schema` carries the payload schema number.
- **Channel ids** are stable strings (`veh.speed`). Ids are the contract, not struct layout.

### 9. Performance budget and how it is measured

- **Target (unchanged from the spec):** the sampler costs at most 0.2 ms per frame at p99, with the server on, a
  recording running and one stream client; one branch when off.
- **Library bench** (`libs/blackbox-telemetry/benches/handoff.rs`, `harness = false`, plain `Instant`, no new
  dependency): publish a 4 KiB `Copy` frame, push an event, pop a command, each over 10 million iterations,
  reporting p50, p99 and max. Run on the reference rig of `docs/benchmarks/`; results are recorded there, not
  asserted in CI (hosts vary). Expected order of magnitude: microseconds [estimate, to measure].
- **Frame-time check** (F-72, in `crates/nfsmw`): a scripted drive, telemetry off against on, compare the game-thread
  step and the frame p99; fail the check if the delta exceeds 0.2 ms. The same measure feeds `perf.telemetry_ms`.
- **Allocation-free by construction:** the frame is `Copy` with no `Vec` or `String`; a compile-time assertion
  caps `size_of::<Frame>()` (8 KiB) and requires `Copy`. Strings travel in events and the 1 Hz slow block.
- **Server cost:** JSON encode into a reused buffer per connection. Order of magnitude 20 to 50 microseconds per
  frame [estimate, to measure], so four 60 Hz streams stay near one percent of one core. Server threads do not
  touch the game thread's data.

### 10. Failure isolation

- Every server thread body is wrapped in `catch_unwind`; a panic sets the status `degraded`, logs once, and ends
  that thread. A panic in a connection thread ends that connection only. A panic in the acceptor or pump restarts
  it up to 3 times with backoff, then the server stays off and the status line reads `telemetry: off (crashed)`.
- The game thread only does bounded, non-blocking operations on lock-free structures. It never holds a mutex shared
  with the server, never joins a server thread except the bounded shutdown join, and ignores errors from the
  server side.
- Mutexes on the server side recover from poisoning (`PoisonError::into_inner`).
- **Keep `panic = "unwind"` in release.** If a release profile ever sets `panic = "abort"`, isolation fails; the
  size/check task should reject that setting while the `telemetry` feature exists.
- Bind failure, a full queue and a dead stream client are normal events with counters, not errors for the game.

### 11. Reuse for LAN and online multiplayer (what 0006 must agree with)

- `blackbox-net-core` is the shared crate. 0006 either uses it as proposed or names what it needs changed. The
  proposed contents are in section 2 and [multiplayer-reuse](../specs/telemetry/multiplayer-reuse.md).
- **Two protocols stay separate:** the observer protocol (JSON over HTTP/WS/UDP, read-mostly) and the game-sync
  protocol (binary over UDP, in 0006). They share ids, time, token generation, limits, the envelope header and the
  hand-off types; they do not share a wire format.
- **Entity ids:** 0006 (section 7) fixes the wire id at 8 bits, server-assigned, the player slots the first 32. The
  telemetry `entity` field is a `u32`: low 8 bits the wire id, upper 24 a generation counter that rises when an id is
  reused, so a long recording never confuses two entities. Single player follows the same rule. Never an ECS handle.
- **Sim time:** the clock of every record is the sim tick plus a tick rate stated in the header; wall time appears
  only in headers and the status page (0006 section 11 states the same rule).
- **Field names, not a shared binary header:** the observer JSON uses `schema`, `seq` and `tick`; the binary sync
  stream (0006, bit-packed) uses the same names in its debug view. The binary header layout is 0006's.
- The control command table is not reusable by remote peers.

## Consequences

- **Costs:** about 7 to 10 new crates in the workspace build (`httparse`, `tungstenite`, `data-encoding`, `sha1`, `http`,
  `utoipa`, `utoipa-gen`, plus their small helpers) [count from the crate manifests, not yet from a lockfile]; roughly 400 lines of
  HTTP framing that we own, with the security burden that implies; one more release-time check (`panic` setting).
- **Security limits:** no TLS. A LAN bind is read-only and a sniffer on the LAN can read game state and the read token.
  Acceptable for racing telemetry; not for anything personal (and nothing personal is exported).
- **Upgrade path:** if the endpoint count or concurrency outgrows the thread model, swap the `http` module for
  `axum` without touching the frame trait, the channel table, the command table or the OpenAPI document (utoipa is
  framework-agnostic). The handlers take a plain `Request` and return a `Response` value for that reason.
- **Compile time and binary size:** not measured. The first implementation PR records them like 0004 did.
- **Docs:** the CSV export becomes a read-only conversion on download (`GET .../data?format=csv`) instead of a
  separate command; the control endpoint paths changed (below). The telemetry spec files were updated to match.

## Contradictions found in the earlier specs, and how they are settled

| Earlier text | Problem | Settled as |
|---|---|---|
| Transport 2.1 `GET /v1/commands/<id>` for a result, while `GET /v1/commands` is the table | One path, two meanings | Results are `GET /v1/control/{id}`; `GET /v1/commands/{name}` is a table entry |
| Control 3 body `{"command": "tp", "args": ["120","-40"]}` while the table names `player.teleport` with named args | Console name against server name | The body uses the server name and a typed `args` object |
| "Read-only by default" and "recordings start on user action" with no start endpoint | No way to start a recording without `control = write` | New scope `record` between `off` and `write` |
| Privacy 4.5 "reads need no token on 127.0.0.1" with a LAN bind allowed | LAN reads were unauthenticated | The token is required on every request on a non-loopback bind, and writes are refused there |
| Privacy 4.4 checks `Origin` only | DNS rebinding sends the attacker's own host | `Host` allowlist added |
| Features open question 1 (poll loop or thread per client) | Undecided | Thread per connection, capped; one pump thread |
| Transport 2.1 events "from the ring's history window" | The ring overwrites; it is no history | History is a server-side `VecDeque` fed by the pump |
| Features F-31 CSV "by a command" | Needs a writer path with no endpoint | A read-only conversion when downloading a recording |

## Revisit triggers

- More than about 20 concurrent clients, or a need for HTTP/2: move the `http` module to `axum`/`hyper`.
- A remote (non-LAN) dashboard or any write over a network: add TLS (`rustls`, licences to check) in a new ADR.
- `tungstenite` or `httparse` stalls or a CVE lands: pin and patch, or move to the upgrade path.
- 0006 chooses a sync model that needs HTTP in `blackbox-net-core`.
- `utoipa` drops 3.1 support or stalls: hand-write the document and keep the drift tests.

## Owner decisions this ADR asks for

1. Whether release builds enable the `telemetry` feature (off in default builds either way).
2. Accept `std::net` + `httparse` + `tungstenite` and owning the HTTP subset, against the heavier `axum` stack.
3. Accept `utoipa` as the OpenAPI source of truth and the drift tests.
4. Accept "no TLS, LAN is read-only" for the first version.
5. The default port (the spec proposes 7708; unregistered, to confirm) and the `telemetry.*` setting names.
6. Agree with 0006 before either is built: the `core` / `netcode` feature split of `blackbox-net-core`, and the entity id
   rule (8-bit wire id plus a generation counter in telemetry).

## Could not verify

- The `tiny_http` upgrade-stream limits (from memory of its API; the choice does not rest on them alone: age and
  the missing timeouts are the reasons).
- Transitive licences beyond the direct crates; `cargo deny check licenses` in the first PR settles it.
- Compile time, binary size, per-frame encode time and the new-crate counts: estimates, to measure.
- That `tungstenite 0.30` builds with `default-features = false, features = ["handshake"]` and exposes
  `derive_accept_key` and `from_raw_socket` as used here (crates.io feature list and docs.rs page confirm the names).

## Sources

Retrieved 2026-10-16: crates.io API for the crates named above (versions, licences, dependencies, features);
`docs.rs/tungstenite/0.30.0` (`handshake::derive_accept_key`); `docs.rs/tiny_http/0.12.0` (`Request::upgrade`).
