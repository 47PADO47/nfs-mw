# Multiplayer security: denial of service, hardening, privacy and release gates

Third part of the multiplayer security design. [Security](security.md) has the threat model, transport and
identity. [Gameplay security](security-gameplay.md) has validation, authority, anti-cheat and the mod policy. Design
only, nothing is built. The numbers are starting values to tune with the load tests of section 5.

## 1. The pre-authentication surface

Everything an unauthenticated sender can reach is the part that must be smallest and most fuzzed.

| Surface | Reachable by | Handling |
|---|---|---|
| LAN discovery request (UDP broadcast, game discovery port) | Anyone on the segment | Fixed size, no allocation, rate limited, amplification safe (2.3) |
| Cookie exchange and Noise handshake (UDP, game port) | Anyone who can send a packet | Stateless cookie before any cryptography, then `snow`. Handshake limits (2.4) |
| First sealed application message (join request or connect token) | A peer that finished the handshake with some key | One small decoder, then authorisation, 5 second deadline |
| Account service HTTPS (online) | Internet | Standard web hardening, outside the game binary |

No other port is opened. The telemetry server stays on `127.0.0.1` by default (its own rules in
[telemetry privacy](../telemetry/transport-and-privacy.md) section 4.1) and its control table is off while a
session runs. A dedicated server exposes exactly one UDP port for play, and optionally one local admin socket.

## 2. Denial of service

### 2.1 Resource bounds per connection

Set on both ends. A client also bounds what a host can make it do.

| Resource | Bound |
|---|---|
| Handshake time | 5 seconds, then the half-open state is freed |
| Authorisation time (token or join message after the handshake) | 5 seconds |
| Idle timeout | 10 seconds, keep-alive every 2 seconds |
| Inbound bandwidth per client | 128 KiB/s hard cap (a client needs about 4 KB/s, [features](features.md) section 4), enforced by dropping excess datagrams before decode |
| Datagrams decoded per client per tick | 128 |
| Receive buffer, send buffer | 256 KiB each (the reliable channel window of ADR 0006 is set to match) |
| Streams | 4 bidirectional, 2 unidirectional open at once |
| Reassembly memory | 64 KiB per channel before authorisation, 1 MiB after (4 MiB for the bulk transfer to a joiner) |
| Pending reliable queue | 256 messages, then the connection is closed (a client that never acknowledges cannot grow memory) |

### 2.2 Global and per-address caps

| Cap | LAN | Online |
|---|---|---|
| Concurrent connections per source IP | 4 (tests on one machine use several instances) | 2 |
| New handshakes per source IP | 5 per 10 seconds | 3 per 10 seconds |
| Half-open handshakes in total | 64 | 256 |
| Connections in total | the session limit, 8 racers plus spectators ([features](features.md) MP-24), hard cap 24 | 16 racers plus spectators, hard cap 48 |
| Auth failures per IP | 10 per minute, then ignore for 5 minutes | the same, plus account lockout (security section 5) |
| Password attempts | per player id and per source (security section 4.6) | the same |

Over a cap the endpoint **drops silently**. It does not answer. A refusal packet can itself be an amplifier or an
oracle. A real player whose request is dropped sees a timeout and "host busy".

The accounting is by address first and by identity second, because a spoofed or mobile source can change address.
Caps are therefore a speed bump. The real defence is address validation (2.4) and cheap rejection.

### 2.3 Discovery that cannot be an amplifier

LAN discovery uses a broadcast request and a unicast reply. Broadcast and spoofable UDP are the classic reflection
setup, so the rules are strict:

1. **Reply never larger than the request.** Both are exactly **64 bytes**. The request is padded to 64 bytes with
   zeros: magic (4), protocol major (1), format version (1), 64-bit random nonce (8), padding. A shorter datagram
   is ignored. The reply is magic, major, version, the echoed nonce, the host key hash (8 bytes), the game port,
   player count, a flags byte (password, in progress, version range), a mode id, and the server name (up to 24
   bytes, zero padded), then padding to 64. The reply is never bigger than the request, so a spoofer gains no
   amplification factor.
2. **Source address filter.** A request is answered only if its source address is on a subnet of one of the
   host's own interfaces (and not the host's own address). Off-subnet, loopback-from-network, multicast and
   broadcast sources are ignored. This stops spoofed requests from the internet or across a router.
3. **Rate limits.** At most 10 replies per second in total and 2 per second per source. Over the limit, ignore.
4. **Opt-in.** A host answers only while it is in "host a LAN game" and the setting "allow LAN discovery" is on. A
   client sends discovery only while the join screen is open, 3 requests in 2 seconds, random nonce each time.
5. **No state.** The discovery responder keeps no per-request state. A reply carries nothing the requester could
   use to reach other private data. Names are sanitised on both ends before display (security-gameplay 1.1.7).
6. **Replies are hints.** The client verifies the host key in the handshake. A reply claiming to be a host's name
   leads to a handshake failure at worst. The list shows the key hash status ("known", "new", "unknown").
7. **Telemetry's UDP feed** (F-43) is a separate sender with its own rule, one-way to a configured target. It is not
   a discovery protocol and shares no listener with this one.

The original game's local challenge ([LAN server](../lan-server.md)) answered any sender with a 4-byte value and
trusted it. Ours trusts nothing from discovery.

### 2.4 Handshake flood and address validation

The handshake is [security](security.md) section 4.4. The points that matter for denial of service:

- **Cookie before cryptography.** A `Hello` costs the server one HMAC and a constant-time compare. No handshake slot,
  no Diffie-Hellman and no memory is spent until the sender has echoed a valid cookie, which proves it can receive
  at its claimed address. The `Challenge` is stateless.
- **No amplification.** `Hello` is padded to at least 512 bytes and the `Challenge` is at most 96 bytes. Until Noise
  message 3 verifies, the server sends at most 3 times the bytes received from that address. A spoofed source gains
  no factor.
- **The cookie is always required**, on LAN and online (one extra round trip). Under load the server also stops
  accepting new handshakes before it stops serving admitted players: when half-open handshakes exceed 32 in total
  or 8 per second, new `Hello` packets get no `Challenge` at all (silent drop), which is cheaper than an HMAC.
- **Cookie secret** rotates every hour (two kept). Cookies are bound to the client address and valid for 30 seconds.
- **Connect token precheck** (online): the cheap AEAD check of the sealed token (key id, expiry) happens before any
  allocation for the player. Invalid tokens cost one decryption and are dropped.
- **Bad tags are free to drop.** An established-session packet with a bad tag is dropped after one AEAD check, with
  no reply and no state change.
- **Expensive work after authorisation**: key derivation and signature checks happen in the handshake and are bounded
  by the caps above. Password hashing (own accounts) never happens on a game server.

### 2.5 Volumetric attacks (online)

A host cannot out-bandwidth a botnet. The design is to make the attack hard to aim and cheap to absorb:

- **Do not publish server addresses** beyond what a connect token needs. The server list shows names. A token
  contains an address only for the player allowed to join.
- **Provider protection**: operators host behind a network with volumetric DDoS filtering and a firewall that allows
  only the game UDP port. Documented in the operator guide.
- **Dedicated servers, not player hosts**, for online play. A player's home connection is not exposed to strangers
  (section 4.1).
- **Fast failure of a bad flow**: the first-byte checks in section 1 make a junk packet cost a few instructions.
- **Account service**: rate limits per IP and per account, a managed WAF, no unauthenticated expensive endpoints.
- **Graceful degrade**: under load the server first sheds new joins, then spectators, then lowers the snapshot rate,
  and keeps the players already in a race connected.

## 3. Server hardening

### 3.1 What a dedicated server needs

Per [features](features.md) section 1 the server uses the operator's own game install for road, collision and car
data, and the server never sends install data to clients. Hardening goals:

- **Load the minimum.** The server loads only the road network, collision, vehicle attribute data and race data it
  needs. It does not open textures, audio, movies, UI or shader assets. Format readers for those are not linked
  into the server binary (cargo features), which also shrinks the attack surface.
- **Read-only install.** The install path is mounted or opened read-only. The server writes only to its own data
  folder (logs, replays, bans, state).
- **A derived validation pack** is a later option: a small file produced from the install by a local tool, holding
  only what validation needs (envelope tables, road graph, checkpoints). It would let a server run without the full
  install. It is never redistributed, because it is derived from game data ([licensing](../../licensing.md)). Not
  required for v1.
- **No client mods.** The server config sets `mods = off` unless the operator adds server plugins by hand.
- **No telemetry control, no console from the network.** The read-only telemetry server may be enabled for
  monitoring, bound to loopback or a management network only.

### 3.2 Least privilege

| Area | Rule |
|---|---|
| Account | A dedicated unprivileged user. Never root or Administrator. No shell, no login. |
| Filesystem | Read-only install and binaries, a single writable data folder. Logs and replays are size capped and rotated. |
| Network | Inbound: the game UDP port only. Outbound: the account service, the time source and the log target (an allowlist). The server cannot reach private network ranges (blocks server-side request forgery from any future URL feature). |
| Linux units | systemd: `NoNewPrivileges=yes`, `ProtectSystem=strict`, `ProtectHome=yes`, `PrivateTmp=yes`, `PrivateDevices=yes`, `RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX`, `CapabilityBoundingSet=` empty, `SystemCallFilter=@system-service`, `MemoryMax`, `TasksMax`, `LimitNOFILE`. |
| Containers | Read-only root filesystem, non-root user, all capabilities dropped, the default seccomp profile, resource limits, one UDP port published. |
| Windows | A low-privilege service account, a Job Object with memory and process limits, Windows Firewall rule for the one port. |
| Memory | Bounded pools for connections, queues and replay buffers. A hard cap on process memory. Out of budget means refuse new joins, not crash. |
| Crash isolation | The server builds with `panic = "unwind"`. Each connection's handler runs behind `catch_unwind`. A panic closes that connection, logs the message type (not the payload), raises a counter and an alert. A supervisor restarts the process on repeated panics. A network panic is a bug that blocks the release gate (section 5). |

### 3.3 Configuration and secrets

- Config is a TOML file ([features](features.md) MP-25). It holds no secret values. Secret fields name a **file or
  a credential reference** (`token_key_file = "/run/secrets/nfsmw-token-key"`).
- Secrets (server identity key, connect-token key, admin token) are read from files with owner-only permission
  (the server refuses to start if group or world readable), or from systemd credentials or a secrets manager.
  They are never passed on the command line (visible in process listings) and never read from environment
  variables that child processes inherit, never logged, never written to a replay.
- **Admin interface**: a local Unix domain socket or loopback port, a random admin token of 128 bits with
  a timing-safe comparison, off by default. Kick, ban and settings commands go through it. It never shares the game port.
- Config reload applies new limits and bans without a restart. Key rotation uses two active keys (security section 6).
- Defaults are the safe values. A risky setting (`allow_unvalidated_state`, discovery on a public interface,
  remote admin) needs an explicit key and prints a warning at start.

### 3.4 Dependencies and build

- `cargo deny check licenses`, `bans` and `sources` as today, plus `advisories` for the network crates in CI.
  The current [deny.toml](../../../deny.toml) has no advisories section. Add one for the release gates and fail the
  build on a known vulnerability in a dependency of the network crates.
- Pin and review the network dependencies (`snow` and the RustCrypto crates it pulls, `chacha20poly1305`, `spake2`, `argon2` if
  used; `quinn`, `rustls` and `ring` if a QUIC transport is added). Updates of these crates are a security review item, not an automatic merge.
- Release artefacts are built in CI from tagged commits, with checksums (SHA-256) and a signature. A software bill
  of materials is attached.
- A server refuses clients below a minimum protocol minor version, so an urgent security fix can force upgrades.
- Any update check is opt-in. It follows the telemetry rule: no outbound traffic by default.

## 4. Privacy

### 4.1 What peers can see

| Data | Who sees it | Rule |
|---|---|---|
| IP address | LAN: the host and every client can see local addresses. Online: the server operator only | Online play is **relay by server**. Players never get each other's IPs. Direct peer to peer between players is not offered online, because it exposes home IPs to harassment and attack |
| Display name, car, ready state, position, results | All players in the session | Required for play |
| Player id (key hash) | All players | Pseudonymous, needed for mute, block and bans |
| Account identity (OIDC subject) | The account service and the operator | Never sent to other players |
| Hardware, OS, install path, other software, mods beyond the manifest | Nobody | The manifest holds sim-affecting mod ids, versions and hashes only. It leaks that a mod is installed, which is stated in the join dialog |
| Chat | Players in the session, and the operator's logs | Section 4.2 |
| Telemetry | Nobody remote | Localhost by default; the multiplayer bridge ([features](features.md) MP-85) is read-only and redacts as in [telemetry privacy](../telemetry/transport-and-privacy.md) section 4 |

On a LAN the host's address is known to all (they are on the same network anyway). A LAN host who then forwards a
port to the internet takes on the online risks. The lobby shows a warning when the game port is reachable from
outside (a UPnP or manual forward is never done automatically).

### 4.2 Chat and moderation basics

- Text chat only in v1; voice is P2 and reviewed separately ([features](features.md) MP-43). Voice, if it comes,
  is opt-in, push to talk, relayed through the server, never recorded by default.
- Messages: 200 characters, rate limited, validated (security-gameplay 1.1.7), rendered as plain text. No clickable
  links, no markup, no images.
- **Local controls** ([features](features.md) MP-42): mute and block by player id, a local word filter (on by
  default online, user switchable) that masks, never deletes the original.
- **Host tools**: kick, ban by id, mute, server-wide slow mode, chat off. Roles are checked by the server for each
  command.
- **Online**: a report button sends the last 50 chat lines and the session id to the operator with the reporter's
  consent. Operators can keep a moderation log with reason, moderator, date. A ban shows the reason and an
  appeal route.
- Names are filtered for impersonation (a name that matches a pinned friend gets a warning, the lobby shows ids on
  demand), and for confusable or invisible characters.

### 4.3 Logging and retention

| Data | Purpose | Retention |
|---|---|---|
| Connection log (player id, time, result, source IP) | Abuse and attack response | 14 days, then only counters |
| Security events (strikes, bans, failed tokens) | Moderation | 90 days, bans until lifted |
| Chat | Moderation of reported content | Not stored by default. If the operator stores it, 7 days |
| Replays and input logs | Review (security-gameplay section 6) | 7 days, 30 for flagged races |
| Metrics (aggregated counters) | Operations | 13 months, no personal fields |
| Crash reports | Debugging | Local only; no upload by default; paths and names redacted |

Rules for every log:

- Tokens, keys, passwords, PAKE data and the admin token are **never logged**. The log layer has a redaction filter
  and the release gate has a test that greps logs from the adversary suite for known secrets.
- Player ids are logged as short hashes; IPs only in the connection log, and truncated (IPv4 /24, IPv6 /48) in anything
  older than 14 days.
- Logs live on the server or the player's machine. A client never uploads logs without an explicit action.
- Online, an account holder can export and delete their data. Deletion removes account data, stats and chat
  reports about them, except where a record is needed for an active ban or investigation.
- The game works fully with no analytics. Any future analytics is opt-in and follows the telemetry privacy rules.

## 5. Security testing checklist and release gates

### 5.1 The adversary suite

An in-repo test tool (a `cargo xtask` command or an integration crate, working on loopback) runs a scripted hostile
client and a scripted hostile host against the real code and asserts the expected reaction. It never contacts the
internet. Cases:

| Case | Expected result |
|---|---|
| Wrong protocol prologue, old or newer major version, version downgrade attempt | Handshake fails with a version reason |
| Valid handshake with an unpinned key (strict mode), changed key for a known name | Refused, or hard warning in the client |
| Join with wrong password, 6 times quickly | Refused, then rate limited, host not locked for others |
| Expired token, reused token, token for another server, token from another connection | Rejected, counters up, no detail on the wire |
| Replay of recorded datagrams and inputs | Dropped as duplicate or stale |
| Truncated, oversized, zero length and random datagrams (10 million) | No panic, no growth in memory, counters up |
| Count fields that claim 2^31 entries | Rejected before allocation |
| Teleport, impossible speed and acceleration, tick advance 1.5x real time, tick far in future | Corrected, struck, kicked per the table |
| Input flood at 10,000 per second | Excess dropped, host tick time unaffected |
| 1,000 spoofed handshake starts from distinct sources | cookie required, no state allocated, memory flat, existing players unaffected |
| Slow-loris (a half-finished handshake, or one fragment byte per second) | Closed by the handshake or authorisation timer |
| Discovery storm and off-subnet source | Replies capped, none off-subnet, reply never larger than the request |
| Lying manifest, huge manifest, bad ids | Rejected or compared as unmatched, never loaded |
| Content package with a path trick, a `.dll`, or a size lie | Rejected before write |
| Hostile host: oversized snapshot, invalid entity ids, invalid strings, endless bulk | Client leaves cleanly, no crash |
| Telemetry write command from a peer or while in session | Refused |
| Secret redaction: run all cases and scan output for key material | Clean |

### 5.2 Other checks

- Unit tests for every limit in security-gameplay section 1.2 (limit passes, limit plus one fails).
- Fuzz budgets from security-gameplay section 2.2.
- `cargo clippy --workspace -- -D warnings` with the denied lints on the network crates, `cargo deny check` with
  advisories, `cargo xtask check` (leak check and size check; extend the leak check to scan for private key and
  token patterns in committed files).
- Clock tests (skew within 5 seconds, jumps) for tokens and retry cookies; time comes from an injectable clock.
- Soak: 16 simulated clients with 5 % loss for 8 hours, memory and CPU flat, no panics.
- Dependency review for any change to network crates, with a written note.
- Threat model review whenever an ADR changes the sync model or adds a message class.
- A `SECURITY.md` with a private reporting channel and a response promise, before any public server exists.

### 5.3 Gates

| Gate | When | Required |
|---|---|---|
| G0 design | Before milestone 10 starts | These documents accepted. The codec crate has the lint set and the checked reader. Fuzz targets exist for discovery and the first messages. |
| G1 LAN alpha | First LAN build | Noise `XX` channel with the cookie round trip, AEAD sealing, replay window and rekey, key pinning, no early data. Decoders fuzzed for the 4-hour budget with no findings open. Discovery rules (2.3) implemented and tested. Adversary suite cases for decoding, replay, teleport, flood and manifest pass. Telemetry and console writes off in session. |
| G2 LAN release | LAN ships to users | The full adversary suite passes. Strike and rate-limit table implemented. Fingerprint UI and key change handling reviewed with a second person. Room password (if shipped) reviewed with an independent look at the PAKE use. `cargo deny` with advisories clean. |
| G3 online beta | Before any public server | Connect tokens and the account flow (OIDC), refresh rotation and reuse detection tested. Cookie always on. Dedicated server hardening (section 3) in the operator guide with a tested systemd unit and container image. Signed results. Replay review path works on a sample. Privacy retention enforced by code. An external review of the transport use, token design and decoders (a paid or volunteer expert, written findings fixed). `SECURITY.md` live. Load test with the DoS cases at 10x expected rate. |
| G4 online general availability | After beta | A beta period with no open critical or high issue. Incident response runbook and key rotation rehearsed. Moderation tools and appeals ready. Own password accounts only after a separate review of Argon2id parameters, lockout and recovery. |

A gate fails when a critical item is open. Any panic, out-of-bounds, hang or unbounded allocation reachable from the
network is critical and blocks the next gate.

## 6. Open questions

1. **Our own secure layer is code we must get right.** S-01 puts the cookie, replay window, rekey and path validation in
   project code on top of `snow`. Mitigations are the fuzz targets, the adversary suite and the independent review at
   G3. If the `renet` spike of ADR 0006 wins, or the review judges the risk too high, QUIC (`quinn` with `rustls` and
   `ring`) is the reviewed alternative behind the same trait (S-03), at the cost the ADR lists. A `renet` choice
   degrades security as described in [security](security.md) section 4.1.
2. **`spake2` maturity.** Pick CPace or an audited OPAQUE crate if one is available at implementation time, or ship
   the room password only after the independent look listed for G2.
3. **Crate licences** in [security](security.md) section 4.1 are from memory. Confirm with `cargo deny` before the first dependency.
4. **Who is the identity provider** for OIDC, and whether the project hosts any account service at all. If it does
   not, online play may be community servers with a server-side allowlist and no central ranking, which removes
   most of security section 5 and gate G3's account items.
5. **Determinism of the full world** decides how much of a race can be re-simulated for review (security-gameplay
   section 6), see [multiplayer reuse](../telemetry/multiplayer-reuse.md) section 7.
6. **Compatibility mode** with the original client protocol: if ever built, its parsers need their own fuzz
   targets, LAN only, off by default, and a statement that it is unauthenticated.
7. **Mod sim-affecting classification.** The manifest needs a field that separates client-only from sim-affecting
   mods. It belongs to ADR 0005's manifest (`mod.toml`), a request to that document, not made here.
8. **Voice chat** needs a separate privacy and moderation review before any design.
