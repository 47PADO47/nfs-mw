# Multiplayer security: validation, authority, anti-cheat and mods

Second part of the multiplayer security design. [Security](security.md) has the threat model, transport and
identity. [Operations security](security-ops.md) has denial of service, privacy, hardening and release gates. Design
only, nothing is built. Wire message names below are placeholders for the ones in [protocol](protocol.md); the
rules apply to whatever names it ends with. Authority rules **[assume ADR 0006]**: the server simulates every car
from client inputs and clients predict their own car ([features](features.md) MP-30, MP-80).

## 1. Input validation: every byte is hostile

These rules apply to the host decoding client messages and equally to a client decoding host messages.

### 1.1 Decoder rules

1. **Decoders are pure functions on `&[u8]`**, in a `libs/` crate with no sockets, no game types and no Bevy
   (the same rule as the format crates). Signature: bytes in, `Result<Message, DecodeError>` out.
2. **No panic paths.** The codec crate sets `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing,
   clippy::arithmetic_side_effects, clippy::panic)]` and uses a small checked reader (`take(n)`, `u16()`, `varint()`) that
   returns `DecodeError::Truncated` instead of slicing. Integer arithmetic on wire values uses checked or saturating
   operations. Release builds of the network crates keep `overflow-checks = true` (the cost is negligible at these
   sizes).
3. **Size limits come first.** A datagram larger than the limit in the table below is dropped before any parsing. A
   reliable message carries a length prefix that is checked against the per-type limit before a byte is buffered, and
   a fragmented message declares its total size in its first fragment.
4. **No allocation by an untrusted count.** A count field is compared with the per-type maximum and with the bytes
   left (`count * min_element_size <= remaining`) before `Vec::with_capacity`. Collections are built with bounded
   capacity, and arrays have fixed maxima.
5. **Strict canonical form.** Trailing bytes after a message are an error. Reserved bits and padding must be zero.
   Unknown enum values are an error, not a default. A non-canonical varint (leading zero groups) is an error.
6. **Numbers are range-checked at the edge.** Positions, velocities and angles arrive quantised as integers with a
   documented range. Anything outside is an error. A decoded float that is not finite is an error. The decoder
   returns domain types (`Throttle(f32 in 0..=1)`), so no later code sees an out-of-range value.
7. **Strings**: UTF-8 validated, length in bytes and in grapheme clusters capped, control characters and Unicode
   bidirectional overrides rejected, NFKC normalised, leading and trailing whitespace trimmed. Display names:
   1 to 24 characters. Chat: 1 to 200 characters. Server names: 1 to 32. Mod ids: 1 to 64 of `[a-z0-9_.-]`. Strings are
   data. They are never used as a path, a format string or a command, and the UI draws them as plain text.
8. **Versioned decoding.** The handshake fixes the protocol major version. Each message type has a one-byte format
   version. Decoding is a match on `(type, version)`. An unknown type in a negotiated version is dropped and
   counted. More than 16 unknown or invalid messages inside 10 seconds ends the connection. A newer minor version is
   decoded only for the fields this build knows, and the rest is skipped by the length prefix (forward
   compatibility without trusting content).
9. **Validate then use.** A decoded message is only a claim. Every id (player, entity, event, item) is looked up in
   the receiver's own tables. An unknown or foreign id is an error. A client may name only its own player id. The
   host ignores the player id carried in a message and uses the one tied to the connection.
10. **Order and state checks.** A message that is legal in another state (a `Ready` after the race started, a
    lobby edit by a non-admin, an input from a spectator) is refused and counted. The pre-auth decoder accepts
    only the handshake messages.
11. **Serde formats are untrusted input too.** Lobby, chat and event messages use `serde` with `postcard`
    ([ADR 0006](../../decisions/0006-multiplayer-netcode.md) section 10). They are decoded only from a slice already
    capped by the message limit, with `postcard::take_from_bytes` so that trailing bytes can be refused. Types used
    on the wire avoid recursive structures and custom deserializers that allocate; strings, vectors and maps in them
    are post-validated against the limits of 1.2 before the value is used. Enum variants are append-only, and an
    unknown variant is an error.
12. **Cost is bounded.** The decoder has a fixed work budget per datagram. No recursion, no decompression without a
    declared and checked output size (the bulk transfer, if compressed, states the size first, 4 MiB maximum,
    ratio capped at 20:1).

### 1.2 Limits

| Item | Limit | Reason |
|---|---|---|
| Datagram (state, input) | 1200 bytes | Under the common path MTU, no IP fragmentation (the telemetry UDP feed uses the same figure) |
| Input frames per datagram | 8 (the last few ticks as redundancy) | Loss cover without a large parse |
| Reliable message | 16 KiB, bulk transfer 4 MiB | Lobby, chat, events are small. Late-join state is the one bulk |
| Chat message | 200 characters, 1 per second per player, burst 3 | Spam, ids of abuse |
| Display name | 24 characters | UI and log safety |
| Entities in one snapshot | 64 cars, 64 traffic, 32 police (set by session settings) | Bounded loop and memory |
| Mod manifest entries | 64 | Section 7 |
| Lobby settings blob | 1 KiB | Bounded |
| Messages in flight per reliable channel | 256 | Memory exhaustion by a client that never acknowledges |

Final numbers come from [protocol](protocol.md). The rule is that each has a named constant, a test that sends one
byte over it, and a fuzz target that reaches it.

### 1.3 What a failed decode does

| Failure | Action |
|---|---|
| Datagram too large, malformed, or bad authentication tag | Drop. Counter `rx_bad_datagram`. No reply (no oracle). |
| Single invalid message from an authenticated peer | Drop the message. Counter, plus a strike (section 5). |
| Repeated invalid messages | Close the connection with a generic reason. The host logs the player id. |
| Invalid message from the host (client side) | Drop it. Three in a row: leave the session and show "The host sent invalid data". |

A decoding error never reaches game logic, never writes to the world, and never takes a lock that the sim thread holds.

## 2. Fuzzing plan

### 2.1 Targets

A `fuzz/` directory outside the workspace members (cargo-fuzz needs nightly and `libfuzzer-sys`; the whitelist
`.gitignore` needs `fuzz/**/*.rs`, `fuzz/Cargo.toml` and the corpus rule added when it lands). One target per
decoder, ordered by exposure:

| Target | Surface | Exposure |
|---|---|---|
| `discovery_request`, `discovery_reply` | LAN broadcast | Anyone on the segment, no authentication |
| `connect_token` | Sealed token blob | Anyone who can reach a server |
| `join_request`, `pake_message` | First messages after the handshake | Authenticated key, not yet trusted |
| `input_frame` | 60 Hz inputs | Every client, hottest path |
| `snapshot` | Server snapshots decoded by clients | A rogue host |
| `reliable_event` | Pursuit, race and lobby events | Both directions |
| `lobby_state`, `session_bulk` | Join-time state | A rogue host |
| `chat_message`, `name_string` | Text validators | Every player |
| `mod_manifest` | Manifest list in the lobby | Every peer |
| `content_package` | Data-only mod transfer (later) | A rogue host |
| `fragment_reassembly` | Fragment headers, duplicate and overlapping fragments, length-prefix framing | Both |
| `packet_header`, `replay_window`, `cookie` | Sealed-packet header parsing, packet number expansion and the replay bitmap, `Hello` and `Challenge` frames | Anyone, before authentication |
| `handshake_frames` | Noise message framing around `snow` (our parsing, not the library) | Anyone |
| `version_negotiation` | Handshake parameters | Anyone |
| `host_state_machine` | Structured sequences of messages against a host session | A hostile client |
| `format_*` | Existing format crates fed by content transfer (`libs/blackbox-chunk`, textures, tables) | Content packages |

`host_state_machine` uses `arbitrary` to build message sequences with a fixed seed world. The oracle: no panic, no
out-of-memory, no hang over 100 ms, session invariants hold (player count, unique ids, bounded queues).

### 2.2 Method

- **Properties** in each target: `decode` never panics. If it returns `Ok(m)`, then `encode(m)` equals the input
  bytes (canonical form), and `decode(encode(m)) == m`. Differential: the safe checked reader against a naive
  reference reader on valid inputs.
- **Corpus**: real recorded sessions (the replay recorder of [features](features.md) MP-71 and MP-72), minimised.
  Hand-made seeds for every message at its size limit and at limit plus one.
- **Sanitizers**: ASan and UBSan runs (nightly, `-Zsanitizer`) on the decoders even though they are safe Rust, to
  catch bugs in `unsafe` dependencies. `cargo miri test` on the codec crate's unit tests (no I/O, so it runs).
- **Run budget**: 60 seconds per target on every pull request that touches the network crates (a smoke run on the
  stored corpus), 30 minutes per target nightly, and 4 hours per target before each release gate (G1, G2, G3 in
  [operations security](security-ops.md) section 5). A crash found is a failing test case committed with the fix.
- **Limits harness**: a unit test iterates over the table in 1.2 and checks that each limit is enforced exactly.
- **Allocation guard**: the fuzz build installs a global allocator that aborts on any single allocation above 1 MiB
  during pre-auth decoding, so an attacker-controlled size cannot hide.
- **Continuous fuzzing**: if the project qualifies, enrol in OSS-Fuzz. Otherwise a scheduled CI job runs the nightly budget.

## 3. Authority model

| Layer | What it is | What it stops |
|---|---|---|
| L1 Inputs only | Clients send controls (throttle, brake, steer, buttons) tagged with a tick. They never send positions to be applied. | Teleport, speed hacks by editing state, position spoofing |
| L2 Server simulation | The server runs `blackbox-vehicle` for every car with the car spec **rebuilt from the car id and part ids** ([features](features.md) MP-30), never from numbers the client sends. | Fake performance, tuned-up stats, modified vehicle data |
| L3 Invariants and envelopes | Every state the server holds is checked against physics bounds (section 4). | Simulation exploits, mod-set state, bugs, and any client-reported state in a mode that carries it |
| L4 Rates and strikes | Per-message token buckets, tick-rate tracking, suspicion score (section 5). | Input flooding, fast-forward clients, spam |
| L5 Results and review | Results are computed by the server from its own record, signed online, and replayable (section 6). | Fake results, disputed races |

Consequences of L1 and L2: a modified client **cannot** make its car faster than the server's simulation of the
inputs it sent. What it can still do is send better inputs than a human (macros, scripted perfect lines), see
more than the player should (section 8), and exploit physics edge cases that every player could exploit. L3 covers
the physics edge cases.

Prediction does not weaken this. The client predicts its own car for responsiveness. The server's state wins, and a
correction snaps or smooths the client. A client that ignores corrections only hurts its own display.

**Information visibility.** The server sends each client only entities in its area of interest (the same set used
for snapshot budgets). Enemy-police positions outside a radius, and cars far away, are not sent. This limits "radar"
cheating by design and it saves bandwidth. Traffic and police are simulated once on the server. Clients only render.

## 4. Envelope checks derived from vehicle physics

The server computes, per car spec, a **performance envelope** from the same deterministic code the sim uses. The
spec is built from the car id and part ids as above. The library already measures a car:
`blackbox_vehicle::performance::measure(&VehicleSpec)` returns `Performance` with the measured `top_speed`, an
`acceleration` table of ten samples spaced evenly from 0 to the top speed (`acceleration_at(speed)` interpolates),
and `start_grip` and `end_grip` as multiples of `g`. The envelope is computed once per car spec in a session and
cached. A NOS-on variant is computed with the same routine and the nitrous parameters enabled (if `measure`
does not take a nitrous option by then, the server runs the equivalent fixed-step full-throttle run itself). The
envelope module lives in `libs/` next to the vehicle code and has no NFS:MW names; the game crate supplies specs.

### 4.1 Per-tick checks on accepted state

For a car with accepted state `S0` at tick `t0` and new state `S1` at tick `t1`, with `dt = (t1 - t0) / 60` s
and `v0`, `v1` the speeds, check:

| Check | Rule | Margin |
|---|---|---|
| Speed ceiling | `v1 <= top_speed(car, nos_active)` | +3 % plus 1 m/s. Extra +15 % for 0.5 s after a ramp or a host-observed contact |
| Longitudinal acceleration | `(v1 - v0) / dt <= acceleration_at(v0) (nos aware) + g * 0.30` | The `g * 0.30` term allows road grade. Plus 25 % jitter allowance |
| Lateral acceleration | `a_lat <= 1.3 * grip(v) * g`, where `grip(v)` is `start_grip` blended to `end_grip` with `v / top_speed` | Friction ellipse: `sqrt(a_long^2 + a_lat^2)` stays within the same bound |
| Braking | Deceleration within `1.3 * grip(v) * g`, plus aerodynamic drag | Collisions are exempt (below) |
| Angular rate | Yaw, pitch and roll rates below a per-class maximum measured from the sim at the same speed | 25 % |
| Position continuity | `|p1 - p0 - v0 * dt| <= 0.5 * a_max * dt^2 + 0.25 m` | |
| Vertical | Height within terrain height plus 3 m, unless on a ramp or in the air for under 3 s | Ground query from `blackbox-roads` and collision data |
| On network | Position within 8 m of a drivable surface, unless a collision pushed the car off the road | `blackbox-roads` |

External impulses (a collision with a car or the world, an explosion, a spike strip) are real. The server marks
the 0.5 s after any contact **it detected itself** as impulse-exempt and widens `v1 - v0` by at most
`2 * relative_speed_at_contact`. A claimed contact with no server-side cause gets no exemption.

Because the server is authoritative, a failed check on a server-simulated car means a bug, an unknown physics
exploit (for example a collision glitch that launches a car) or a state set by something other than inputs
(a reset, a mod). The reaction is to log with the replay position (section 6), clamp the car to the envelope, and
raise an alert counter. It does not punish the player for what the player's inputs did not do.

For **client-reported state** (a mode where a client's reported state is used, such as a high-latency option or the
reconciliation hint a client sends with its predicted state), a failed check means the report is discarded and a
strike is added (section 5). Reported state is never applied directly: the server uses it only when it falls
inside the envelope around its own simulated state.

### 4.2 Teleport and reset

- **Teleport detection**: displacement over one accepted step above `v_max * dt * 1.1 + 0.5 m` is a teleport. The
  server corrects and counts it.
- **Resets**: a client asks for a reset (`ResetCar`, [features](features.md) MP-35). The server validates it:
  cooldown (once per 10 s in a race, free in roam), not during a busted sequence, not while a pursuit
  arrest is in progress, and the target is chosen by the server (the nearest road point behind the car, from
  `blackbox-roads`). The client cannot choose where it appears.
- **Checkpoints and shortcuts**: race checkpoints are validated in order by the server from its own car position.
  A sector cannot be finished faster than `sector_length / top_speed(car, nos) * 0.95`. A faster sector time is a
  sim anomaly (alert).

### 4.3 Tick-rate abuse

A client that runs its clock fast sends more input frames than real time allows.

- The host owns the clock. Every authenticated connection has `ticks_claimed` (the latest tick in an accepted
  input) and the host time. In any 5 second window, `ticks_claimed` may advance by at most `5 * 60 * 1.03`
  plus a slack of 6 ticks. More means speed hacking: excess frames are dropped and a strike is added. Less
  than 70 % of the expected advance (a lag switch, a frozen client) leaves the car on the last input (the
  server holds the last input for 100 ms, then neutral with the handbrake off and brake gently on), plus a
  strike only when it repeats.
- An input tagged with a tick too far in the future (over 12 ticks ahead of the host tick) or too far in the past
  (over 30 ticks behind, with the redundancy window excepted) is dropped.
- The server never fast-forwards because a client says so. Time sync messages come from the server only, and the
  client adjusts its own clock offset from them (the smoothing is in [sync model](sync-model.md)).

## 5. Rate limits and strikes

### 5.1 Token buckets

Every connection has one bucket per message class, refilled at a steady rate, with a burst. The check runs on
the datagram header, before the body is decoded.

| Class | Sustained | Burst | On exceed |
|---|---|---|---|
| Input frame datagrams | 66 / s | 20 | Drop the excess, strike weight 1 per 10 drops |
| Reliable messages (all types) | 20 / s | 40 | Drop, strike |
| Chat | 1 / s | 3 | Drop, notice to sender |
| Reset request | 1 / 10 s | 2 | Refuse |
| Lobby actions (ready, car change, vote) | 2 / s | 6 | Refuse |
| Votes started | 1 / 60 s per player | 1 | Refuse |
| Discovery requests (host side, per source) | 2 / s | 4 | Ignore |
| Joins per source address | 5 / 10 s | 5 | Ignore, then the connection limit of [operations security](security-ops.md) |

### 5.2 Suspicion score

Each connection has a score that gains weight on each violation and decays by 1 per 10 seconds without any.

| Score | Action |
|---|---|
| 10 | Warn the player in the HUD ("Network problem or invalid data"). Log. |
| 30 | Correct hard: discard client-reported state and resend a full baseline. |
| 60 | Kick from the session with a reason. The host may ban the player id. |
| Online, repeated | Session ban then account review. A permanent ban always needs a human decision on the evidence of section 6. |

Weights are small for things that lag causes (late inputs, one dropped burst) and large for things only a modified
client does (impossible tick advance, a valid-signature message with an invalid body, a claimed teleport). The
score and its reasons are visible to the host in an admin view (kick and ban buttons from [features](features.md)
MP-26). A threshold that fires in a clean test run is a bug in the check, not a reason to loosen it silently.

## 6. Results and replay review

- **The server computes results** (finish order, lap times, top speed, busted) from its own simulation and its own
  checkpoint events. Clients receive results and display them. A result message from a client is ignored.
- **Online, results are signed.** The server signs each result record (race id, player ids, times, a hash of the
  input log, game and sim build ids) with its identity key. The account service stores only signed results from
  servers it knows. A ranking service never takes a client upload. This is what "ranked" means. LAN results are not
  signed and never ranked.
- **Replay review.** The server keeps, per race, the seed, the session settings, the list of cars, and the stream
  of validated inputs per player (about 6 bytes per car per tick; roughly 22 KB per car per minute before
  compression, an estimate to measure). With the deterministic fixed-step `blackbox-vehicle` the car's motion can
  be re-simulated from inputs. This allows: (a) re-checking a flagged race offline, (b) an appeal, (c) regression
  tests for the checks. It matches the telemetry recorder (`schema` and game build in the header, see
  [multiplayer reuse](../telemetry/multiplayer-reuse.md) section 6). A signed replay file lets a reviewer trust the
  record without trusting the server operator's word. Whether the whole world replays bit-exactly (AI, traffic) is the
  determinism question of [multiplayer reuse](../telemetry/multiplayer-reuse.md) section 7. Where it does not, the
  review re-simulates cars only and checks each against the recorded server states within a tolerance.
- **Retention** of replays: 7 days by default, 30 days for races with a flag, deleted on the account holder's
  request unless under review ([operations security](security-ops.md) section 4).
- **Telemetry and console commands.** Telemetry writes (`clock.set`, `car.reset`, `traffic.set` of
  [control](../telemetry/control.md)) and developer console cheats are **disabled while connected to a session**, on
  clients and on the server. The multiplayer command path is its own table with host agreement
  ([multiplayer reuse](../telemetry/multiplayer-reuse.md) section 4).

## 7. Mods policy

Native mods are fully trusted local code with no sandbox ([ADR 0005](../../decisions/0005-native-mod-plugins.md)
section 6). The network policy follows from that.

1. **Native mods are never downloaded, never fetched from a peer or server, and never loaded from the network or
   from a content cache.** There is no "download missing mods" button, no auto-update of code from a host, and no
   plugin transfer message in the protocol. A mod is on a machine because its user put it in the mods folder.
2. **Lobby manifest, for compatibility only.** Each peer sends a list of its sim-affecting mods:
   `{ id, version, sha256 of the mod (library and content files), kind: code | content }`, from the `mod.toml`
   manifests and the hashes ADR 0005 already logs at start-up. Limits: 64 entries, ids as in 1.1.7, versions as
   semver strings of at most 32 characters.
3. **Comparison rules.** The host compares the sets and lets a joiner in only when they match under the session's
   policy: *strict* (identical id, version and hash for every sim-affecting mod; the default online), *lenient*
   (the host lists the required ones; extra client-only mods are ignored; the LAN default), or *off* (no
   sim-affecting mods; the vanilla set). A mismatch returns a readable diff ("you lack `a.b 1.2`; you have extra
   `c.d`"). Mods marked client-only in their manifest (HUD, audio, post-processing) are not compared. The mod
   interface switches of ADR 0005 section 6 (gameplay writes off in an online session unless every peer enables them)
   stay as the accident guard.
4. **The manifest is not an attestation.** A modified client can lie about its mods, and a mod can lie about
   being client-only. The comparison prevents accidental desyncs and honest mismatches, nothing more. Security
   rests on L1 to L5 of section 3: the server validates the effect of whatever the client runs.
5. **Content-only transfer, possibly later (not in v1).** If the project ever lets a host offer a content mod
   (a car, a track tile, music; no library) to joiners, these rules apply and the feature is off by default:
   - the mod package has **no library file**; the receiver rejects any package that contains an executable type
     (`.dll`, `.so`, `.exe`, scripts) or an unknown extension (the receiver uses an allowlist, like the
     whitelist `.gitignore`), or a path with `..`, an absolute path, or a drive letter. The package is a flat list of
     named entries in our own container, not a zip (so there is no zip-slip or zip-bomb);
   - limits: 64 MiB per package, 256 MiB total cache, 4096 entries, entry names 1 to 128 characters;
   - integrity: the lobby manifest gives the SHA-256 of each package. The receiver verifies it while receiving
     and discards on mismatch. A hash from the host proves the data matches what the host offered, not that it is
     safe, so:
   - consent: the player must approve each new package ("Accept 3 MB car pack `x.y` from host `Name`?") and can
     decline and stay out;
   - quarantine: received content is stored in a content-addressed cache outside the mods folder. It is mounted
     only for that session, read only, and never overwrites installed files or other mods. A cached package is not
     promoted to the mods folder without a user action;
   - parsing: package content passes through the same readers as install files, in a worker with a memory limit
     and a time limit, and those readers are fuzz targets (`content_package`, `format_*` in section 2).
6. **Scripts later.** If a scripting layer is added, network-delivered scripts run only in a real sandbox (WASM, as ADR
   0005 section 8 suggests), with no file, network or process access, fuel and memory limits. Until then there is no
   remotely delivered logic.
7. **Original game files never travel.** The protocol has no message to send install files, and the content
   transfer rejects packages that reproduce install content (it compares entry hashes against a table of install
   chunk hashes computed from the receiver's own install). The project does not distribute the game
   ([licensing](../../licensing.md)), and a peer to peer file service would become a distribution channel.
8. **Server-side plugins** (an operator extending a dedicated server) are loaded from the server's own folder by the
   operator, never from clients. The server config can set `mods = off`.
9. **Telemetry, devtools and the mod API** are not reachable by peers. A peer cannot call a mod interface on
   another machine.

## 8. What is not attempted, and honest limits

| Not attempted | Why |
|---|---|
| Kernel-level or driver anti-cheat | Never. It is invasive, a security risk of its own, not portable to Linux, and out of keeping with an open-source game. |
| Client integrity attestation, code signing checks of the game binary, anti-debug, obfuscation | The client is open source and can be rebuilt. Such checks are theatre and would exclude honest modders and Linux users. |
| Memory scanning, process monitoring, screenshots, hardware ids | Privacy cost and no benefit under an authoritative server. |
| Detecting input macros or assisted driving in general | The server sees only inputs. Obviously inhuman patterns can be flagged for review (for example a steering trace that repeats exactly across laps) but are never an automatic ban. |
| Hiding the positions the client must render | The client needs nearby cars. Area-of-interest limits the rest (section 3). |
| Protecting a user from their own mods | Native mods are fully trusted (ADR 0005). A mod can read `identity.key`, session tokens and memory. Treat installing a mod like installing a program. A ranked mode could refuse players whose mods are loaded, but the claim is honour-system only. |
| Stopping a determined cheater who replaces the whole client with a bot that plays with perfect, legal inputs | Possible. L1 to L5 keep them inside the same physics as everyone, which is the realistic goal. |
| Defence against a malicious host for private LAN games with no moderation | A host can delay, kick and misreport in its own game. Clients protect their machines (sections 1 and 7), not the result. |

The goal is plain: a modified client cannot make anyone else's game unsafe, cannot move faster than physics allows,
cannot invent results, and cannot take over an account. Everything else is community moderation.
