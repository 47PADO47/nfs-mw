# Online multiplayer (milestone 11): the path from LAN

Status: **designed, not built** (2026-10-16). This file says what milestone 11 adds to the milestone 10 server. It
does not change the tick, the simulation, the snapshots, prediction or the packet format, which
[ADR 0006](../../decisions/0006-multiplayer-netcode.md) fixes for both milestones. Read it after the
[README](README.md) and [security](security.md). Where a choice is a proposal rather than a decision, the text says so.

## 1. What changes, and what must be ready on LAN

Online play reuses the LAN server program, the protocol and the client. Four things differ:

| Concern | LAN (M10) | Online (M11) |
|---|---|---|
| Where the server runs | A player's game (listen server) or a LAN dedicated server | A dedicated server with a public address, run by an operator |
| Handshake | Noise `XX` with pinned keys and a room password | Noise `XK` with the server's key pinned by the client, connect token from the account or master service |
| Finding a server | Broadcast hint, or a typed address | Server list from a master service, invite codes, direct address |
| Who sees whom | Everyone on the segment sees addresses | Players never see each other's IP; the server relays all game traffic |

Milestone 10 must leave these ready, so that M11 adds services and does not rewrite the server:

1. The `XK` handshake implemented and tested, not only `XX`. Both run through the same code, and the LAN build may
   switch `XK` off by setting. (Proposal: build both in M10 and keep `XK` for the test suite until M11.)
2. A server configuration file (TOML, MP-25) with the listen address, port, public or LAN-only flag, player caps and
   rate limits ([security-ops](security-ops.md) section 2.2).
3. Version and data checks in the handshake (`proto` major and minor, `sim_build`) and a reason code for refusals.
4. Counters for the metrics in section 9, behind a cargo feature.
5. A default UDP port that is not one of the ports of the old tools (9900, 9901, 20000, 20923, 30920, 30921, 13505,
   listed in [ADR 0006](../../decisions/0006-multiplayer-netcode.md) section 15) and not 6000. The stock client binds
   6000 on its connect path (see [LAN server](../lan-server.md), client path), so a server on 6000 would collide with
   the original client on one PC.

## 2. Phases

Each phase has a gate. A public server is never run before the security gate G3 of
[security-ops](security-ops.md) section 5.3 is passed.

| Phase | Adds | Gate | Depends on |
|---|---|---|---|
| M10 | LAN: listen server, LAN dedicated server, LAN discovery | [ADR 0006](../../decisions/0006-multiplayer-netcode.md) "How we will know" | the sim extraction |
| M11.1 | Public dedicated server, direct connect by address and key, no list, no accounts (branch B of section 6) | Gate G3 (online beta) | M10 complete |
| M11.2 | Master service: server list, heartbeat, invite codes, region field, address check | G3 items for the master; list abuse tests | M11.1 |
| M11.3 | Optional relay for servers that cannot accept inbound UDP | Relay in the same review as the handshake | M11.2 |
| M11.4 | Accounts with OIDC (branch A), ban epochs, friends later | G3 account items | Owner decision (section 11) |
| M11.5 | Results, stats, replays on the server; leaderboards only from signed results | S-11 signing tests; retention enforced in code | M11.1 |
| M11.6 | Ranked matchmaking | Accounts, signed results, an authoritative server set | M11.4, M11.5 |

M11.1 and M11.2 deliver the features the owner most needs (a list and a way to join friends online) without accounts.
M11.6 is last on purpose: it needs identities that the project may not want to run.

## 3. Dedicated server hosting

- **One binary, `nfsmw-server`.** The same code as the listen server with no window (ADR 0006 section 1). Linux and
  Windows builds from CI, with checksums and signatures ([security-ops](security-ops.md) section 3.4).
- **One session per process** at first. A session is one world with up to 16 players and their spectators. Several
  sessions per process need a second design (per-session isolation of the sim and its memory); not planned for M11.
- **Game data.** The operator installs the game and points the server at its folder. The server loads only what the
  sim needs, runs without write access to the install, and never sends install files to clients (S-10).
- **Configuration.** TOML file for the name, port, mode, caps, region, password, traffic density and the public or LAN
  flag (MP-25). Secrets (the server static key, the result-signing key) are files with mode 0600, never on the command
  line ([security](security.md) section 6).
- **Capacity.** The design target is 16 players per process. The world-step gate is 4 ms of a 16.7 ms tick on the
  reference PC, plus 0.5 ms for the network, which is about 27 % of one core at full rate (ADR 0006 section 8,
  unmeasured).
- **Deployment.** A container image (non-root, read-only game-data mount) and a systemd unit with restart and resource
  limits. Both are required by gate G3 and are tested before a public release.
- **Operator guide.** Install, configure, generate keys, publish the key fingerprint, open the UDP port. Port forwarding
  is documented, and the server never opens a port by itself (no automatic UPnP).

## 4. Connectivity: NAT, relay, regions

### 4.1 Client to server, no hole punching

The topology is client-server (ADR 0006, decision 1). Every client sends to one server address, and the server is the only
inbound endpoint. A client behind a NAT needs only outbound UDP, and the NAT keeps its mapping alive from the regular
traffic (ping at 1 Hz, 4 Hz at join, [protocol](protocol.md)). Hole punching and STUN are not needed and are not
built. Peers never send to each other, so there is no peer-to-peer path to punch.

A server silent for 10 s drops a client, and a client silent for the same time drops the server. Some carrier-grade
NATs use UDP timeouts shorter than the common 30 s, so the ping rate is chosen to stay well under 10 s.

### 4.2 Servers behind a NAT: forward, or relay

- **Port forwarding** is the default. The operator forwards one UDP port (section 3). It is documented, not automated.
- **Relay (M11.3, optional).** A forwarder that sits on a public address and passes sealed packets between clients and
  a server behind a NAT. It sees only the outer headers and the sealed bytes (ADR 0006 section 1). It has its own per-session
  tokens, its own rate limits and no game logic. Its costs and its metadata are in section 10 and the privacy table below.
  It is not used in M11.1.

Relay privacy: the relay sees each client's address, the server's address, timing and packet sizes. It cannot read the
game, and the server still sees only the relay's address, so the server does not learn the player's address from the
relay. The relay joins the review of the handshake at gate G3 ([security-ops](security-ops.md) section 5.3).

### 4.3 Players never learn each other's addresses

The server relays everything, and clients never receive peer addresses: snapshots carry entity ids, not addresses
([protocol](protocol.md)). Only the server (and a relay, if used) sees a client's address. Logs keep addresses only for
the retention period in [security-ops](security-ops.md) section 4.3. Direct peer-to-peer play is not offered online
([security-ops](security-ops.md) section 4.1), because it would expose home addresses to attack.

### 4.4 Regions and latency

- Each server declares a region code in its master listing (a coarse code such as `EU`, `NA-E`; the set is an open
  question). Regions are for display and for the client's choice. There is no automatic routing across regions.
- The client measures round-trip time to each server it shows, with the cookie exchange of
  [security](security.md) S-02 (no game data), at most one probe per server every 10 s.
- Playable limits: up to 150 ms RTT (ADR 0006 section 1, [features](features.md) section 4). Online prediction uses an
  input buffer of two ticks (ADR 0006 section 4). The sim does not stretch with latency; a bad connection shows
  corrections, not slow-motion.
- Each server runs its own clock. There is no time sync across regions (sync model, section 5).
- Proposal: start with one region, add a second only after measurements from real players. A third region needs the
  operator base to justify it.

## 5. The master service: server list, invite codes, verification

### 5.1 Scope

The master service is minimal in M11.2. It does not run games, hold game state, see game traffic or match players by
skill. Its jobs:

1. Accept heartbeats from servers and keep a list of the live ones.
2. Answer list queries, with filters and paging.
3. Map invite codes to server ids.
4. Verify that a server's advertised address is its own before it is listed.
5. Later (M11.4 and up), mint connect tokens after an account login ([security](security.md) section 5.2).

Non-goals in M11.2: parties, friends, skill rating, ranked queues, voice, chat history. Chat stays inside a session.

### 5.2 Heartbeats and verification

- A server sends a heartbeat every 30 s, signed with its server key. The heartbeat carries name, mode, players, cap,
  region, version range, password flag and the key fingerprint. It carries no player data.
- An entry expires 90 s after its last heartbeat.
- Before the first listing of an address, the master asks the server to answer a cookie exchange on that address. This
  stops a spoofed heartbeat from listing a victim's address (the address is not trusted without this check). The
  ADR-level rule that the master never amplifies is kept: the reply is no larger than the request.
- Limits: heartbeats per key and per address, entries per address, total listing size. Numbers are set when tested
  ([security-ops](security-ops.md) section 2.2).

### 5.3 Queries and codes

- A list query returns a page of entries (name, mode, players, cap, region, version status, password flag, fingerprint,
  address). Players are never listed. Filters are applied on the server.
- Invite codes (proposal): eight characters from a 32-character alphabet that drops look-alike characters. A code maps
  to a server id, not to a player, and lasts while the server is listed or for 24 h, whichever is shorter. The operator
  can revoke a code. A code reveals the server's address, which is public by design.
- Storage (proposal): SQLite for a single master, which is enough for the first listings. Postgres only if the master
  needs several writers.

### 5.4 Ranked, later

Ranked matchmaking (M11.6) is a queue with a rating, and it needs accounts (branch A) and results signed by an
authoritative server (S-11). It is not designed further here.

## 6. Accounts and identity: two branches

The owner must choose (section 11). Both branches are described, because the rest of the design differs.

### 6.1 Branch A: the project runs an account service with OIDC

- The game signs in through OpenID Connect first (S-07): system browser, authorization code with PKCE, a loopback
  redirect. The game never holds a password.
- The player id is the provider's issuer and subject, hashed. Ban epochs travel in connect tokens (security section 5.2).
- Game servers hold only the connect token and never call the account service on the connection path.
- Costs: an HTTPS service, one or more identity providers, an email path for recovery, data retention and deletion
  rules, and the G3 account items. If the project later adds its own passwords, the rules in [security](security.md)
  section 5.1 apply.
- Benefits: bans that hold across servers, friends, stats tied to a person, ranked (M11.6).

### 6.2 Branch B: community servers with allowlists

- No project account service. Each server holds its own list of admitted player keys (the device keys of
  [security](security.md) sections 4.3 and 4.5, trust on first use), and may require a room password.
- Operators may share ban lists, each signed by the operator's key, and subscribe to the lists they trust.
- The master lists servers with heartbeats only. No player is known to the project.
- Costs: no account service, no identity provider, few G3 items (open question 4 of [security-ops](security-ops.md) says
  this removes most of section 5 of [security](security.md)).
- Limits: a banned player can make a new key. There are no friends across servers and no ranking. Stats are per server.

### 6.3 Recommendation (proposal)

Build branch B for M11.1 to M11.3. Decide branch A before M11.5, because persistent stats and any ranking need an
identity that spans servers. Nothing in M11.1 to M11.3 has to change if branch A is added later, since the device key
stays and an account is an extra layer on top.

## 7. Server browser (client)

- A browser screen in the lobby menu. Data: the master list, the player's favourites and recent servers (stored in the
  settings file, not in telemetry).
- Each row shows name, mode, players and cap, region, ping, a version status (match, close, or refused) and the password
  flag. Sorting and filters are client-side on the current page.
- Refresh every 30 s. Direct connect by address, or by invite code.
- Version refusals are shown with the reason code from the server, greyed out. A mod list check (MP-63) joins later.
- The browser never probes other players. It probes only the servers it lists, at the rate of section 4.4.
- Placement: the screen is in `crates/nfsmw` behind the `multiplayer` feature; the network calls use `crates/nfsmw-net`.

## 8. Persistence: results, stats, replays

- **Results (M11.5).** A per-race result record, fixed in format now (MP-70): finish order, best lap, top speed, damage,
  busted count, pursuit time. The server writes it at the end of each race and signs it with the result-signing key
  (S-11). Storage is an append-only local file, with a JSON-lines export (proposal).
- **Replays.** The server writes its snapshot stream to a file (MP-71). Playback reuses client interpolation. The file
  format is shared with telemetry F-33 ([multiplayer reuse](../telemetry/multiplayer-reuse.md)). Replays are kept under
  the retention rule of [security-ops](security-ops.md) section 4.3, and uploading one is opt-in.
- **Verification.** A replay can be re-simulated only where the sim is deterministic on one build (ADR 0006 section 12;
  [security-gameplay](security-gameplay.md) section 6). Server results are trusted because the server signed them, not
  because the replay re-runs.
- **Stats and leaderboards.** Under branch B, stats are per server and are labelled as such. Official leaderboards exist
  only if the project runs a server and publishes its results. Under branch A, stats follow the account (MP-74).
- **Databases.** Files first. A database only where there is more than one writer (the master's SQLite in section 5.3).

## 9. Operations

- **Builds.** One binary per platform, a container image and a systemd unit ([security-ops](security-ops.md) sections 3.1 and 3.4).
- **Metrics.** The telemetry crate of [ADR 0007](../../decisions/0007-telemetry-server.md) (`blackbox-telemetry`) provides
  the status and the channel types. The server exposes counters on the loopback address only; no public metrics endpoint
  exists. Counters: players, milliseconds per tick (against the 4 ms gate), bytes per client, datagrams dropped by reason,
  handshake refusals, cookie failures, rate-limit hits, heartbeat results. The HTTP stack adds several crates (the costs in ADR 0007), so it sits behind a cargo feature that the server image turns on only when it needs the endpoint.
- **Logs.** Redacted by the rule of [security](security.md) section 6: no key, no token, no password. Addresses are kept
  only for the retention period.
- **Versions.** The handshake carries the protocol version and `sim_build`. A server accepts a range and states its own
  in the heartbeat. A server restart ends its sessions: there is no migration in v1 (host migration is P2 in
  [features](features.md)). A heartbeat field carries a maintenance notice.
- **Moderation.** Kick, ban by key, allowlist, and reports carrying the minimum data ([security-ops](security-ops.md)
  section 4.2). A published security contact (`SECURITY.md`) is part of G3.
- **Keys.** The server result key is rotated yearly, and the public half is re-registered with the master. The
  connect-token key has two active ids during rotation ([security](security.md) section 6).
- **Capacity planning.** Egress for a full 16-player server is about 192 KB/s (ADR 0006 section 8, case C), roughly 0.5 TB
  a month if it runs full all month. The master sees one heartbeat every 30 s per server and is small. These are
  estimates, and the server operator's real numbers come from the metrics above.

## 10. Costs and risks

| Item | Size and cost (estimate) | Note |
|---|---|---|
| Operator server | One small VM per 16-player session; about 0.5 TB a month of egress at full load | Price depends on the provider; check before publishing a price |
| Master service | Small VM and a SQLite file; heartbeats are tiny | Needs a public address and HTTPS for its endpoints |
| Relay (M11.3) | Carries the server's traffic again, about the server's egress | Used only where port forwarding is not possible |
| Accounts (branch A) | An identity provider (often a free tier; check), email for recovery, the account service's operation | Data retention and deletion requests |
| External review (G3) | A paid or volunteer expert; cost not known | Required before any public server |
| Moderation | Operator time, and project time for reports | Scales with public play |
| Load testing | At 10 times the expected rate, per [security-ops](security-ops.md) G3 | Needs a test rig |

Risks:

- **Volumetric attacks on public UDP.** Operators need network-level protection ([security-ops](security-ops.md) section
  2.5). The project cannot defend an internet-scale flood with code alone.
- **Our handshake faces the internet.** Bugs in the cookie, rekey or replay code are the main risk, mitigated by fuzzing
  and an independent review. QUIC is the fallback ([ADR 0006](../../decisions/0006-multiplayer-netcode.md) section 9).
- **Community fragmentation and abuse (branch B).** Bans do not span servers, and players can move to new keys.
- **Legal and naming.** Operators run servers that need their own game installs, and no game data is served. Service and
  server names must not imply that the project or EA runs them. The original EA hosts no longer run, and the project makes
  no claim to revive them. Check the trademark position before naming the master.
- **Schedule.** M11 depends on M10, whose largest item is the sim extraction ([ADR 0006](../../decisions/0006-multiplayer-netcode.md)
  section 14).
- **No shared clock across servers.** Each server runs its own tick (section 4.4).
- **Mods online.** Public servers need the mod policy (MP-63 and ADR 0005) before they open.
- **Public use of the old protocol.** None. The original protocol is not used online ([LAN server](../lan-server.md)).

## 11. Decisions for the owner

1. Branch A or branch B (section 6). Proposal: B for M11.1 to M11.3, A decided before M11.5.
2. Whether the project runs any public server or the master, and what it costs per month.
3. Whether the relay is built in M11, or later when a real need appears (proposal: later; manual forwarding first).
4. Whether ranked play is in scope at all (proposal: decide after branch A).
5. The region set, decided by measurement (section 4.4).
6. The default UDP port (ADR 0006 owner decision 5), checked against the list in section 1.

## 12. Open questions

1. Whether the master is written in Rust in the same workspace (proposal: yes) and on which HTTP stack.
2. Whether heartbeats are signed by the result key or by a separate heartbeat key (proposal: a separate key, so a leak
   of one does not sign results).
3. How players find servers without a master, for a private group: direct address and invite codes only (proposal).
4. Whether the listing shows the server's key fingerprint, so the client can check it on the first join (proposal: yes,
   shown in the browser and pinned on first use).
5. How the browser pages through a long list (proposal: server-side pages of 50).

## Sources

Written from the milestone 10 design files: [ADR 0006](../../decisions/0006-multiplayer-netcode.md), the
[README](README.md), [features](features.md), [protocol](protocol.md), [sync model](sync-model.md),
[bandwidth](bandwidth.md), [integration](integration.md), [security](security.md), [security-gameplay](security-gameplay.md)
and [security-ops](security-ops.md), and [ADR 0007](../../decisions/0007-telemetry-server.md) with
[multiplayer reuse](../telemetry/multiplayer-reuse.md). The original LAN client path is in [LAN server](../lan-server.md).
No external service was consulted for prices. Every cost figure is an estimate, and every performance figure is a
design target that nothing has measured.
