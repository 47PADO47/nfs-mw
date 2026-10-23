# Multiplayer security: threat model, transport and identity

Security design for LAN multiplayer (milestone 10) that the online mode (milestone 11) reuses without a rewrite.
This file is the entry point. [Gameplay security](security-gameplay.md) covers input validation, fuzzing, authority,
anti-cheat and the mod policy. [Operations security](security-ops.md) covers denial of service, privacy, server
hardening, testing and release gates. New design: no source is copied from anywhere. The transport part (sections 4.1 to 4.8)
is built in `libs/blackbox-net-core` behind the `netcode` feature; section 8 says what and where. The rest is design.

Companions: [ADR 0006, multiplayer netcode](../../decisions/0006-multiplayer-netcode.md) (sync model and wire
format), [ADR 0005, native mod plugins](../../decisions/0005-native-mod-plugins.md), [multiplayer README](README.md),
[features](features.md) and [protocol](protocol.md). Where this file assumes something those documents decide, the
assumption is marked **[assumes ADR 0006]** and must be reconciled when the ADR is accepted.

## 1. Principles

1. **The client is hostile.** The game is open source and every client can be modified. Nothing a client sends is
   believed until the host has checked it. Security rests on host-side validation, not on hiding anything in the client.
2. **Every byte from the network is hostile**, including bytes from the host. Clients validate host messages with the
   same decoders and limits as the host uses for client messages (a rogue host is a threat, section 2).
3. **One security layer for LAN and online.** The same encrypted, authenticated channel, the same decoders and the
   same checks run in both. Online adds an account service and connect tokens on top. It does not change the layers
   below. LAN differences are only in how trust is established (section 4).
4. **Secure by default, no password on the LAN.** A LAN game needs no account, no password and no internet. A
   password is an option for the host (section 4.6).
5. **No remote code, ever.** Nothing received from a peer or a server is executed, loaded as a plugin or
   interpreted as a program (section 7 of [gameplay security](security-gameplay.md)).
6. **Fail closed, say why.** A failed check drops the packet or connection and writes a counter and a short log line.
   The player sees a plain reason ("host key changed", "mod set differs"), not a stack trace.
7. **No panics on network data.** A decoder returns an error. It never indexes, unwraps or allocates by an untrusted
   length (details in [gameplay security](security-gameplay.md) section 1).

## 2. Threat model

### 2.1 Actors

| Actor | Where | Capability | Goal |
|---|---|---|---|
| Curious or cheating peer | LAN and online | A modified client; sends any valid or invalid message at any rate | Win, grief, read other players' state they should not see |
| Spoofer on the LAN | LAN | Sends forged UDP (source address spoofing, broadcast), can sniff, and can act as a man in the middle on an open Wi-Fi | Impersonate a host, hijack a join, crash clients |
| Rogue host | LAN and online | Runs a modified host; controls shared world state and lobby text | Crash or exploit joiners, fake results, harvest IPs, advertise a trap lobby |
| Malicious server | online | Same as rogue host, plus it lists itself publicly | Mass exploitation of clients, phishing through names and chat |
| Internet attacker | online | Anyone who can send UDP/TCP to a public port; botnet capable | Take servers down (DDoS), amplification, scan for bugs |
| Account attacker | online | Credential stuffing, phishing, token theft, session replay | Account takeover, ban evasion, impersonation |
| Malicious mod author | both | Ships a native plugin that the user installs | Anything a local program can do (trusted by ADR 0005, out of network scope) |
| Passive observer | LAN and online | Reads traffic | Learn names, chat, positions, IPs |

Out of scope: an attacker with local code execution on a player's machine (that includes loaded native mods, see
ADR 0005), a compromised operating system, physical access, and nation-state adversaries. Section 8 of [gameplay security](security-gameplay.md) lists what we
state honestly that we cannot stop.

### 2.2 Assets

| Asset | Why it matters | Main protections |
|---|---|---|
| Player machine integrity | Parser bugs are remote code execution | Bounds-checked decoders, fuzzing, no native code from peers, sandboxed content decode |
| Identity key and session tokens | Impersonation, account takeover | Key file permissions, OS credential store, short token lifetimes, rotation |
| Fairness of races and results | The point of the mode | Host authority, envelope checks, host-computed results, replay review |
| Host and server availability | A crashed or flooded host ends the session | Rate limits, address validation, per-IP caps, resource bounds |
| Player privacy | IPs, names, chat, play time | Relay-by-server online, minimal logs, retention limits |
| The game install and its licence position | Original assets must not leak | The protocol never carries original game files (section 7 of [gameplay security](security-gameplay.md)) |
| Leaderboards and rankings (online) | Trust in the ranking | Only server-signed results from authoritative servers |

### 2.3 Trust boundaries

```
 [ game process: engine + mods ]   trusted as one unit (ADR 0005)
            |  boundary B1: secure channel (Noise + AEAD), every byte below is hostile
 [ LAN peer / host ]  <---- boundary B2: discovery (unauthenticated UDP hint, never trusted)
            |
 [ account / matchmaking service ] -- boundary B3: HTTPS + OIDC, connect token handover
            |
 [ dedicated game server ]  boundary B4: server vs operator config and secrets
```

- **B1**: the network. Everything crossing it is decoded by the hardened decoders and authenticated by the channel.
- **B2**: LAN discovery. A reply is only a hint ("a host may be at this address"). The host's key fingerprint is
  checked in the handshake, never taken from the discovery reply.
- **B3**: the game never sees an account password. It holds tokens issued by the account service (section 5).
- **B4**: the server process holds secrets and runs with least privilege ([operations security](security-ops.md) section 3).

### 2.4 LAN versus online

| Topic | LAN | Online |
|---|---|---|
| Identity | Device key, trust on first use (TOFU) | Account (OIDC) plus device key, connect token |
| Discovery | UDP broadcast hint | Account service lists servers, no broadcast |
| Host | A player's game, or a LAN dedicated server | A dedicated server, operator-run |
| Topology | Host and clients, all see local addresses | Players never see each other's IP; the server relays |
| Attacker position | Same segment; can spoof and sniff | Anywhere on the internet |
| Ranked results | Not recorded as ranked | Only signed by authoritative servers |
| Lockouts, bans | Host kicks and bans by key | Account bans, moderation, appeal |

The original game's online layer gives no model to copy. [NFSLAN](../lan-server.md) shows plaintext HTTP for the
lobby lookup and a UDP "local challenge" that anyone on the network can answer with a fixed 4-byte value, and its
[known problems](../lan-server.md#known-problems) list an out-of-bounds write and unsynchronised state. The
[community emulator](../../mods/nfs-online-server.md) patches a certificate-check branch in the client (behaviour
only is recorded there, its code is AGPL and not used). Our client verifies keys by pinning and never ships a
check-weakening patch. Compatibility with the original protocol is a separate, optional, off-by-default LAN-only
module if it is ever built. It is treated as untrusted legacy input, and its parsers are fuzzed first.

## 3. Decisions at a glance

[ADR 0006](../../decisions/0006-multiplayer-netcode.md) chose a host-authoritative server over the project's own UDP
layer (packet numbers, ack bits, channels, a `Transport` trait) and left the handshake, the cipher use and identity
to this document ("where it disagrees with this ADR, it wins"). The decisions below fill that in without changing the
ADR's packet layout.

| Id | Decision |
|---|---|
| S-01 | Secure channel: a **Noise handshake** (`snow`) gives mutual authentication and forward secrecy, then every packet is sealed with **ChaCha20-Poly1305** under the Noise transport keys, using the packet number as the nonce (snow's stateless transport state, which allows reordering). It sits inside ADR 0006's own UDP layer. LAN uses pattern `XX`, online uses `XK` (sections 4.1 to 4.4). |
| S-02 | A **stateless cookie round trip** comes before any Noise processing, on LAN and online. Handshake-start packets are padded, and a reply to an unvalidated address is never larger than the request (section 4.4 and [operations security](security-ops.md) section 2.4). |
| S-03 | The secure channel is one small trait in a `libs/` crate with no NFS:MW names and no Bevy. QUIC (`quinn` and `rustls` with `ring`) stays a possible **second transport** behind the same trait, for example for web spectators, as ADR 0006 allows. |
| S-04 | LAN trust: each install has a long-term identity key (the Noise static key). Peers are pinned on first use. The lobby shows a short fingerprint. A different key is a different player and a name clash is flagged (section 4.5). |
| S-05 | Optional room password: **SPAKE2** run inside the encrypted channel and bound to the Noise handshake hash. The password never travels and cannot be tested offline (section 4.6). |
| S-06 | Online entry: **connect tokens** (netcode.io style): short-lived, sealed, bound to the player's key and to one server key, single-use, minted by the account service (section 4.8). |
| S-07 | Online accounts: **OIDC** (authorization code with PKCE, system browser) first. Own password accounts, if ever, use Argon2id, lockout and optional 2FA (section 5). |
| S-08 | Authority: the server simulates every car from validated client inputs and is the sole arbiter of results and shared world state (ADR 0006, [features](features.md) ground rules). Envelope checks derived from `blackbox-vehicle` run as a second line on every accepted state, and as the first line for any client-reported state a mode may carry (section 4 of [gameplay security](security-gameplay.md)). |
| S-09 | Native mods are never fetched from peers. Lobby manifests are compared for compatibility only. Content-only transfer is data-only, hash-checked and consented (section 7 of [gameplay security](security-gameplay.md)). |
| S-10 | A dedicated server needs the operator's own game install for road, collision and car data ([features](features.md) section 1) but loads only the subset it needs (no textures, audio or movies), runs unprivileged with an egress allowlist, and never serves install files to clients (section 3 of [operations security](security-ops.md)). |
| S-11 | Nothing is ranked or persisted online without server-signed results. |
| S-12 | Kernel-level anti-cheat, client attestation and obfuscation are never used. |

## 4. Transport security

### 4.1 Options evaluated

Licence column: facts for `quinn`, `renet` and `chacha20poly1305` come from the ADR 0006 table (crates.io API,
2026-10-16). The rest are from memory and **unverified**: run `cargo deny check licenses` on the real dependency
tree before adopting anything. [deny.toml](../../../deny.toml) allows MIT, MIT-0, Apache-2.0 (with and without the
LLVM exception), BSD-2/3-Clause, ISC, Zlib, Unicode-3.0, CC0-1.0, MPL-2.0 and BSL-1.0.

| Option | Licence | Gives us | Costs |
|---|---|---|---|
| **`snow` (Noise) on our UDP layer** | Apache-2.0 OR MIT | Small, synchronous, no runtime. Mutual authentication by static keys without certificates (fits TOFU and pinned server keys). Forward secrecy. A stateless transport mode for out-of-order UDP. Identity hiding in `XX` and `XK` | We write the cookie, the replay window, rekey scheduling and path validation. Each is a place for a bug |
| `quinn` + `rustls` (QUIC) | MIT OR Apache-2.0; rustls Apache-2.0 OR ISC OR MIT | A reviewed TLS 1.3 handshake, Retry, anti-amplification, loss recovery | Needs `tokio`, certificates for a LAN with no names, congestion control on datagrams, no loss notification. Rejected for v1 by ADR 0006 |
| `renet` + `renetcode` | MIT OR Apache-2.0 | Channels and netcode-style tokens ready made | Token scheme assumes a token server; tokens use one shared key with no forward secrecy; the message API does not meet ADR 0006's bit budgets |
| DTLS (`openssl`, `webrtc-dtls`) | mixed | Standard | OpenSSL terms are not in the allow list; no mature permissive DTLS 1.3 in pure Rust |
| Raw AEAD with a pre-shared key | n/a | Trivial | No per-peer identity, no forward secrecy, one leaked key breaks all sessions |

**Decision S-01: Noise over the project's own UDP layer.** ADR 0006 chose that layer and showed the packet budget
(a 16-byte AEAD tag, a 4-byte packet number). The Noise framework is the established way to get an authenticated
key exchange on top of it without certificates and without a runtime. Using `snow`'s stateless transport state means
we do not write the cipher or the key schedule, only the packet number to nonce mapping, the replay window and the
rekey trigger (all specified below and fuzzed). The cipher suite is `Noise_XX_25519_ChaChaPoly_BLAKE2s` (and its `XK`
variant): ChaCha20-Poly1305 is fast without hardware AES, and its cost on the packet budget is the 16-byte tag the
ADR already counts. The RustCrypto resolver (the `snow` default) keeps everything in permissive pure Rust; the `ring`
resolver is an alternative. The licences of the crates it pulls (`curve25519-dalek` BSD-3-Clause, `chacha20poly1305`,
`blake2`, `subtle`, `zeroize`) are to be confirmed by `cargo deny`.

If the ADR's one-week `renet` spike wins instead, security degrades as follows: the connect token is renetcode's,
the room password feeds the token key, forward secrecy is lost, and the key binding of section 4.8 cannot be done.
This is recorded as a risk in [operations security](security-ops.md) section 6, not as a plan.

### 4.2 Keys and patterns

- **Static (identity) key**: X25519, one per install, section 4.3. Its public half is the identity.
- **LAN uses `XX`**: neither side knows the other's key in advance. Message 1 carries the initiator's ephemeral key,
  message 2 the responder's ephemeral and static keys, message 3 the initiator's static key. Both static keys travel
  encrypted, so a passive observer learns no identities. TOFU decides trust after the handshake (section 4.5).
- **Online uses `XK`**: the connect token (section 4.8) gives the client the **server's static public key**, so the
  client pins it before sending anything, and the client's static key travels encrypted. No certificate authority and
  no TOFU online.
- **Server result signing** (online) uses a separate **Ed25519 signing key** held by the server, never the Noise key.
  Its public half is registered with the account service. Keys have one purpose each.
- **Handshake hash** (`h` in Noise, exposed by `snow` as `get_handshake_hash`) is the channel-binding value for the
  password proof (4.6). It is unique to this handshake.
- **Prologue**: both sides mix the protocol name and major version (`nfsmw-mp/1`) into the Noise prologue. A version
  mismatch fails the handshake before any game data exists.
- **No early data.** The first application payload is sent after message 3. (The Noise `IK` pattern would allow a
  payload in message 1, which is replayable, so it is not used.)

### 4.3 Identity keys

- On first multiplayer use the install creates an X25519 key pair and stores the secret in `identity.key` in the
  per-user data folder with owner-only permissions (the same place and rule as the telemetry write token in
  [transport and privacy](../telemetry/transport-and-privacy.md) section 4.5).
- **Player id** = the first 16 bytes of SHA-256 of the public key. The display name is a label, not an identity. Two
  players may share a name. Bans, mute lists, friends and pins use the id.
- The secret never leaves the machine and is never logged, exported by telemetry or placed in a recording. It is held
  in memory in a zeroising type.
- Regenerating the key is a user action. It makes the player a new identity.
- Honest limit: native mods run in the same process and can read the file. See section 8 of
  [gameplay security](security-gameplay.md).

### 4.4 Handshake packets, cookie and sealing

**Handshake flow** (sizes are targets for [protocol](protocol.md)):

1. `Hello` (client to server): protocol major, a 16-byte client nonce, the client's chosen connection id, padded to
   at least **512 bytes**. The server does no cryptography on receipt beyond the cookie computation.
2. `Challenge` (server to client): a cookie of 32 bytes, `HMAC(server_secret, client_address || client_nonce ||
   time_bucket)`, plus the server's connection id and the bucket id. At most 96 bytes, so never larger than the
   request. **The server keeps no state.**
3. `Hello` again, now with the cookie. The server recomputes the HMAC for the current and the previous 15 second
   bucket, compares in constant time, and only then allocates a handshake slot and runs Noise message 1. The Noise
   messages travel in three handshake frames (message 1 in the second `Hello`, messages 2 and 3 in later frames).
   Until message 3 verifies, the server sends a client at most 3 times the bytes it received from it (the QUIC
   anti-amplification rule, kept as a general principle).
4. The application payload begins after message 3. The server's first sealed packet confirms the keys.

The server secret rotates every hour, with two kept. Cookies are bound to the client's address, so a spoofer cannot
obtain a cookie for another address. This is the stateless-cookie handshake that ADR 0006 assumes. It runs on the LAN
as well as online, because it costs one extra round trip (about a millisecond on a LAN) and keeps one code path.

**Sealed packet format** (consistent with the ADR's 15-byte header and 16-byte tag): flags (1 byte, with a 2-bit key
phase and a handshake bit), connection id (2), packet number (4, the low 32 bits), then the payload and the 16-byte
tag. The header bytes are the authenticated associated data. The ack fields of ADR 0006 sit inside the sealed
payload, so an observer sees neither them nor the contents.

- **Nonce**: the packet number is expanded to 64 bits from the highest number received (as QUIC reconstructs packet
  numbers) and used as the Noise stateless nonce. Each direction has its own key, so a nonce is never reused across
  directions.
- **Rekey**: a new key is derived with Noise `Rekey` every 2^24 packets or one hour, whichever comes first, signalled
  by the key phase bit. The old key is kept for 3 seconds for reordered packets, then erased.
- **Connection id** lets the server find the session without trusting the source address. A packet from a **new
  address** with a valid tag and a newer packet number is accepted, but the server sends no bulk data to the new
  address until a path challenge (a random 8-byte value sealed in a reply) is answered. This handles NAT rebinding
  without letting a spoofer redirect a session.
- **Failure**: a bad tag is dropped silently and counted. There is no error reply, so there is no oracle.

### 4.5 LAN trust model: trust on first use

LAN has no certificate authority and no internet. We use the model of SSH known hosts.

1. Discovery ([operations security](security-ops.md) section 2.3) finds a host. It returns an address and a short
   host-key hash used only to match a saved entry. It proves nothing.
2. The client connects. The `XX` handshake proves the host holds the key whose hash it presented.
3. **Known host** (key pinned before, keyed by player id, not by IP, because DHCP changes addresses): connect
   silently. The lobby shows the host's name and a "known" mark.
4. **New host**: the lobby shows the **fingerprint** as eight groups of four hexadecimal characters and as a
   four-word phrase from a fixed 2048-word list, for the host and the joiner to compare aloud when they care (a room
   with strangers on open Wi-Fi). The player confirms "trust this host" once. The pin is saved.
5. **A different key is a different id**, so a "changed key" cannot happen for the same id. To handle "same name, new
   key" the lobby warns when a new host presents a display name that matches a pinned host with another id: "New
   host with the name of a host you know. Different key."
6. **Host side**: the host sees the joiner's fingerprint too, can kick and ban by id, and can set join mode *open*,
   *ask* (host approves each new id) or *friends only* (pinned ids).

TOFU weakness, stated: the first connection is open to a man in the middle. The comparison in step 4 and the
password in 4.6 close that gap for people who care. A game among friends on a home network accepts it.

### 4.6 Optional room password (PAKE)

A password limits who can join and removes the first-use risk without comparing fingerprints.

- Algorithm: **SPAKE2** (RustCrypto `spake2`, MIT OR Apache-2.0 from memory, **unverified licence**; the crate has
  not had an independent audit as far as we know, so it is marked experimental). CPace is an alternative if a
  maintained crate is found. A Noise pre-shared key derived from the password is **not** used, because a man in the
  middle could then test guesses offline against the handshake.
- Run inside the established channel, right after message 3 and before the lobby accepts any other message. Both
  sides derive a confirmation tag over the PAKE key and the Noise handshake hash `h`. A mismatch closes the
  connection with the same generic "join refused" as a full room, so a wrong password reveals nothing.
- A man in the middle who ran separate handshakes on each leg has two different `h` values, so cannot relay a valid
  confirmation, and sees only PAKE messages from which passwords cannot be tested offline.
- Guess limits on the host: 5 wrong attempts per player id and per source address in 60 seconds, then 60 seconds of
  refusal, doubling to a 15 minute ceiling. No lockout of the room as a whole (a griefer must not be able to lock out
  friends).
- Passwords: 1 to 64 characters, normalised (NFKC; `message::password_secret`, used by both ends), never stored. The host holds it in memory only.

### 4.7 Replay and ordering

| Layer | Replay defence |
|---|---|
| Handshake | Fresh ephemeral keys per handshake. Cookies expire in 30 seconds and are bound to the address. No early data. |
| Packets | The packet number is the nonce. A receive window of 1024 packets (a bitmap) drops duplicates and anything older. A tag failure is dropped before the window is updated. |
| Inputs and states | Each carries a tick. The host accepts only newer ones, counts duplicates and stale ones, and drops them before decoding the body. |
| Reliable events | Per-channel message ids from ADR 0006. The receiver refuses ids it has already delivered. |
| Connect tokens | Single-use and bound to the player's key (4.8). The server keeps used token ids until expiry. |
| Discovery | A random 64-bit nonce in the request is echoed in the reply. A reply with the wrong nonce is ignored. |
| Session resume | The resume token of ADR 0006 is sealed under the session key, single-use, valid 30 seconds, and the resumed channel runs a fresh handshake. |

### 4.8 Connect tokens (online)

Netcode.io-style tokens give a client permission to connect to specific servers without the game server contacting
the account service per connection.

- **Minted by** the account service after OIDC login (section 5), when the player picks a server. The request carries
  the player's **public static key**.
- **Contents** (a sealed blob, XChaCha20-Poly1305 under a key shared between the account service and each game
  server, a key id `kid` in the clear): version, `user_id`, `token_id` (128-bit random), `issued_at`, `expires_at`,
  the server address list, each server's **static public key** (the pin for `XK`), the **player's static public
  key**, a role flag (player, spectator), and a ban epoch.
- **Lifetime** 30 seconds from issue, 5 seconds of skew tolerance. A player who connects slower fetches a new token.
  Wall-clock time appears here and in the cookie bucket only. The game protocol itself timestamps nothing with wall
  time, as ADR 0006 and [multiplayer reuse](../telemetry/multiplayer-reuse.md) section 6 require.
- **Delivery**: the client runs `XK` against the pinned server key, then sends the sealed token as the first sealed
  application message. The server decrypts it, checks `kid`, expiry, role and ban epoch, checks that the token's
  player key **equals the static key authenticated in the handshake**, checks `token_id` is unused, records it until
  `expires_at`, and then admits the player. A stolen token is useless without the player's private key, and a token
  copied to another server is useless because the pin differs.
- Rejections are silent on the wire (a generic close) and counted. No oracle says which field failed.
- **Key rotation**: servers accept two active `kid`s. The account service signs with the newer. Rotation every 30
  days, or at once after any suspected leak.
- LAN has no tokens. The LAN equivalent is the pinned key plus the optional room password.

## 5. Authentication and accounts (online, later)

Online play needs a stable identity for bans, ranked results and friends. LAN needs none, so none of this ships
with milestone 10. The design only fixes the seams so that they exist.

### 5.1 Login method

**Decision S-07: sign in through OpenID Connect first.** The game opens the system browser (RFC 8252), runs the
authorization code flow with PKCE, receives the code on a loopback redirect (`127.0.0.1`, random port), and
exchanges it for tokens. The project then stores no password and has no password database to lose. Which identity
providers to accept is an operator decision. The player id of an online account is the provider's issuer plus
subject, hashed.

If the project later runs its own accounts, the rules are fixed now:

| Item | Rule |
|---|---|
| Password hashing | **Argon2id**, memory 64 MiB, 3 passes, parallelism 1, tuned so one hash takes at least 250 ms on the server. Per-user random 16 byte salt, a server-wide pepper held outside the database. Parameters stored with the hash so they can be raised. |
| Password policy | At least 10 characters, no composition rules, a check against a common-password list. Optional breached-password check through a k-anonymity range query. |
| Lockout | After 5 failures: 60 seconds, doubling to 15 minutes, counted per account and separately per source address. A trusted-device cookie bypasses the per-account lock only, so a stranger cannot lock the real owner out for long. The legacy emulator uses 5 failures and 300 seconds ([behaviour](../../mods/nfs-online-server.md)); we use a progressive version. |
| Second factor (optional) | TOTP (RFC 6238) first, WebAuthn later. Recovery codes, eight, single-use, shown once. |
| Registration | Email verification. Rate limit per address. A challenge (not a CAPTCHA wall on the game client) on the web form. |
| Recovery | Email link, single-use, 15 minutes, invalidates all sessions. |
| Responses | Same message and similar timing for a wrong user and a wrong password. |

### 5.2 Sessions and tokens

- **Access token**: 10 minutes, bearer, sent only to the account and matchmaking services, never to game servers.
- **Refresh token**: 30 days, **rotating and single-use**. Each use returns a new refresh token. Reuse of an old
  one is treated as theft: the whole token family is revoked and the player signs in again.
- **Storage on the client**: the OS credential store (Windows Credential Manager, macOS Keychain, Secret Service
  on Linux) through a thin `libs/` wrapper. Never the settings file, never telemetry, never logs.
- **Game servers** see only the connect token. They hold no account credentials and call no account endpoint on the
  connection path, so an account-service outage does not stop running sessions.
- **Revocation**: bans bump a ban epoch that is embedded in new tokens. A server can also be told to drop a
  user id at once through the operator channel.
- Sign-out revokes the refresh token on the service and deletes the local copy.

## 6. Key and secret handling summary

| Secret | Held by | Stored in | Rotated |
|---|---|---|---|
| Identity key | Player install | `identity.key`, owner-only | On user request |
| Refresh token | Player install | OS credential store | Every use |
| Server static (Noise) key | Dedicated server | Secret file, mode 0600 | On suspicion, or yearly (tokens carry the pin, so rotation needs a token key overlap) |
| Server result-signing key (Ed25519) | Dedicated server | Secret file, mode 0600 | Yearly, public half re-registered with the account service |
| Cookie secret | Any server | Memory only | Hourly, two kept |
| Connect-token key | Account service and servers | Secret file or secret manager | 30 days, two `kid`s active |
| Pepper, signing keys | Account service | Secret manager | Per policy |
| Room password | Host memory | Never stored | Each session |

No secret appears on a command line, in an environment dump, in a log line or in a recording. A log-redaction test
in the release gates ([operations security](security-ops.md) section 5) enforces it.

## 7. Limits to remember

- TOFU does not defend the first connection against an active man in the middle unless the players compare
  fingerprints or use a room password.
- The connect token only proves "the account service allowed this user to try this server". It does not prove the
  client is unmodified.
- The channel protects against outsiders. It does not protect against the peer you are talking to. That is the job
  of validation and authority ([gameplay security](security-gameplay.md)).

## 8. As built (2026-10-22)

In `libs/blackbox-net-core`, module `secure` (feature `netcode`), sans-IO; byte layouts in
[protocol-handshake](protocol-handshake.md). Tests run with `cargo test -p blackbox-net-core --all-features`.

| Item | Status |
|---|---|
| S-01 Noise, sealing | Built. `snow` 0.10 (RustCrypto resolver, no `ring`) runs `XX` and `XK`; sealing uses the split keys directly with `chacha20poly1305`, the Noise `ChaChaPoly` nonce layout and Noise `REKEY`, because `snow`'s stateless transport takes no associated data. Key phases, the 3 s old key and the replay window are built and tested |
| S-02 cookie | Built: keyed BLAKE2s over the address, nonce and both connection ids; 15 s buckets, two secrets; 512-byte requests, 36-byte `Challenge`; 3x limit per slot |
| S-04 identity, TOFU | Built as a pure API: `IdentityKey` (zeroised secret, the caller stores it), `KeyId` (SHA-256, 16 bytes), fingerprint, the four 11-bit phrase indices (the game supplies the word list), `assess` and `admit` over a caller's `PeerStore` |
| S-05 room password | Built: symmetric SPAKE2 with the handshake hash as identity, role-specific confirmation tags; experimental (no known audit of `spake2`). both ends feed `message::password_secret` (NFKC) to it |
| S-06 connect tokens | Built on the server side and for minting: XChaCha20-Poly1305, two `kid`s, every check of 4.8, single use in a bounded table |
| 4.6 guess limits, auth failures | Built: `ratelimit::FailureRule::PASSWORD` and `AUTH` |
| Path validation (4.4) | Not built: `PATH_CHALLENGE` frames are still skipped |
| Accounts (5), result signing, OS credential store | Not built (milestone 11) |

Licences checked with `cargo deny check licenses` and `all-features = true` on 2026-10-22: `snow`, `chacha20poly1305`,
`blake2`, `sha2`, `spake2`, `getrandom`, `zeroize`, `crossbeam-queue` are MIT or Apache-2.0; `curve25519-dalek` and `subtle` are
BSD-3-Clause; `triple_buffer` is MPL-2.0; all are on the allow list.

Continue with [gameplay security](security-gameplay.md) and [operations security](security-ops.md).
