# NFS-Online-Server (moio9)

NFS-Online-Server is an unofficial community emulator of the online services for three EA games: Need for Speed: Underground 2, Need for Speed: Most Wanted (2005), and Need for Speed: Carbon. It has two halves. A Python server (`nfs_online.py`) emulates the lobby, account, messenger, and race-relay services. A set of Windows x86 ASI plugins, written in Zig, point each game client at that server. For nfs-mw the relevant part is the Most Wanted client and its network redirection, but the server protocol behaviour is the reference for any future online work.

Status of this page: the repository is public and readable. Its README, config files and the Zig client source were read. The Python server source (`server/`, `nfs_online.py`) was not read, so server-side claims are README-only.

## 1. Metadata

| Field | Value |
|---|---|
| Repository | https://github.com/moio9/NFS-Online-Server |
| Author | moio9 (GitHub account; no further identity stated) |
| Type | Server emulator plus client ASI plugins (not a single .asi mod) |
| Licence | AGPL-3.0-or-later for original project code (LICENSE, LICENSE-NOTICE.md). Trademark notice: Need for Speed names belong to Electronic Arts; project is unofficial. |
| Language | Python (server), Zig 0.16.0 (clients), shell scripts. GitHub language bar not readable from the fetch. |
| Target game builds | NFS:MW (2005, PC), plus NFS:U2 and NFS:Carbon. Client built as Windows x86 ASI. Exact game executable version not stated. |
| Latest visible activity | Not shown on the fetched page. Repository shows 16 commits on `main`, 55 stars, 8 forks. |
| Releases / packages | None visible. |

Licence consequence for nfs-mw: AGPL-3.0 is copyleft and incompatible with our MIT OR Apache-2.0 plan for copied code. No code, config tables or comments may be copied. Only behaviour and facts may be re-specified (see section 5).

## 2. What it does

Each feature is listed with its evidence.

- Emulates online play for NFS:MW through the MW client's network redirection. Evidence: [README] "NFS MW, UG2 and Carbon online server emulator"; [source] `source/client-zig/src/mw/main.zig` redirects `net_mw.ini` host and port pairs.
- Lobby, bootstrap, and control service ports for MW. Evidence: [config] `config/server.toml` `[most_wanted]` section (BOOTSTRAP 30921, LOBBY 30920); [config] `clients/most-wanted/net_mw.ini` `[network]` section (lobby_port 30920, bootstrap_port 30921, control_port 20923, control_alias_port 13505, race_port 20000).
- Shared EA Messenger on TCP 13505, used by all three games. Evidence: [config] `server.toml` `MESSENGER_LISTEN`; [README] ports table.
- Shared news / Terms of Service / PREL web endpoint on TCP 20923. Evidence: [config] `server.toml` `WEB_LISTEN`.
- Race relay on port 20000 (UDP for race traffic, per README ports table). Evidence: [config] `server.toml` `RACE_RELAY_LISTEN`, `RACE_VIRTUAL_NETWORK = "100.64.0.0/10"`.
- Account handling with password auth, lockout after failed logins, and optional auto-enrol. Evidence: [config] `server.toml` `AUTH_MODE`, `AUTH_FAILURE_LIMIT`, `AUTH_LOCKOUT_SECONDS`, `AUTH_AUTO_ENROLL`; `[most_wanted] CREATE_ACCOUNT_ON_FIRST_LOGIN = false`.
- Local LAN mode for MW: optional injected browser server entry and a LAN control port. Evidence: [config] `net_mw.ini` `[lan]` section (host, port 9900, inject_server, server_name = MWONLINE, server_racers); [source] `main.zig` calls `lan.install()`.
- Carbon-only extras (not MW, listed for context): ad service (MAD), DLC catalogue and per-account DLC page on 127.0.0.1:8081. Evidence: [config] `server.toml` `[carbon]` section.
- Operations commands: `check`, `configure --public-host`, `create-account`, `start` (per-game select, daemon mode), `stop`, `status`, `kick`, `dlc show`, `stats`. Evidence: [README] quick start; not verified in `nfs_online.py`.
- Virus visibility patch (Carbon only, not MW). Evidence: [README] / source pointer `src/carbon/virus.zig`, not read.

## 3. How it does it

### 3.1 Loading

- The MW client is a DLL built as `net_mw.asi`, with a `net_mw.ini` placed beside it in the game's plugin directory. [README] and [source] both state the manual install.
- `DllMain` on `DLL_PROCESS_ATTACH` stores the module handle and calls `DisableThreadLibraryCalls`. It does no other work, to stay loader-lock safe. [source] `main.zig`.
- Startup is deferred to an exported `InitializeASI`, called once by the ASI loader. A `started` flag makes repeat calls no-ops. [source] `main.zig`. This is the standard ASI loader entry convention. [inferred] the loader is an ASI loader such as Ultimate ASI Loader; the repo itself does not name one.
- `InitializeASI` gates on `process_guard.acquire("mw")`, then starts a thread (`mainThread`). [source] `main.zig`.

### 3.2 Start-up sequence (mainThread)

1. Open a per-process trace log `net_mw-trace-<pid>.log`. [source]
2. `WSAStartup(0x0202)`. Failure logs and exits. [source]
3. Parse `net_mw.ini` through `config.loadIni` with `state.applyConfig`. [source]
4. `state.refreshAddresses()` resolves the host and port pairs. [source]
5. `patches.install()` and `lan.install()`, then `Runtime.init`. [source]
6. `Runtime.refresh()` runs every 100 ms for 200 iterations, then every 1000 ms. [source]

### 3.3 Byte patch (the only patch read in source)

- `src/mw/patches.zig`, function `install()`.
- Gated by `state.patches_enabled` (the `[patches] enabled` key in the INI). [source, flag name only; key mapping not read]
- Finds the main module with `GetModuleHandleA(null)` and its PE image with `pe.image`. No hard-coded address. [source]
- Scans the image for one of two signatures:
  - `7D ? C7 86 ? ? ? ? ? ? ? ? EB ? 03 7C 24`
  - `7E ? C7 86 ? ? ? ? ? ? ? ? EB ? 03 7C 24`
- If the match is `7D` (JGE), writes `7E` (JLE) at the match address with `memory.protectWrite`. Returns true if already `7E`. Logs `MW certificate branch patched`. [source]
- Purpose: the log message suggests a certificate check branch. [inferred] The byte flip inverts a short conditional jump. The repo does not explain the check itself.
- No winsock, DirectInput or Direct3D hooks are installed by `patches.install()`. [source]

### 3.4 Network redirection

- Host and port pairs from `net_mw.ini` (`host`, `bootstrap_port`, `lobby_port`, `control_port`, `control_alias_port`, `race_port`) are resolved into per-service addresses. [source: names in `mainThread` log line]
- The actual rewrite of socket connects and UDP sends is implemented in `state.zig`, `profile.zig`, `lan.zig` and a `socket_runtime` module. None of these were read, so the exact hook mechanism (IAT patch, detour, or other) is UNVERIFIED.
- Optional per-service `*_host` overrides exist. [README] / [config comment]

### 3.5 Things the repo does not do (verified absence in files read)

- No DirectX hooks, no DNS hook, no socket-level hooks named in `patches.zig` or `main.zig`. [source, for the files read only]
- No game function names or hard-coded addresses in the read files. The only address-like data is the byte signature above.

## 4. Configuration

### 4.1 Client: `net_mw.ini` (read from `clients/most-wanted/net_mw.ini`)

| Section | Key | Default | Purpose (as stated in the INI comments) |
|---|---|---|---|
| [network] | enabled | true | Turns network redirection on |
| [network] | host | 127.0.0.1 | Server host. Comment: change this one value for LAN or Internet |
| [network] | bootstrap_port | 30921 | Bootstrap port |
| [network] | lobby_port | 30920 | Lobby port |
| [network] | control_port | 20923 | Control (web) port |
| [network] | control_alias_port | 13505 | Messenger alias port |
| [network] | race_port | 20000 | Race relay port |
| [lan] | host | 127.0.0.1 | MW LAN target |
| [lan] | port | 9900 | MW LAN port |
| [lan] | inject_server | true | Injects a browser entry |
| [lan] | server_name | MWONLINE | Name shown for the injected entry |
| [lan] | server_racers | 1 | Racer count for the injected entry |
| [lan] | selected_host_hook | true | Not explained in file |
| [lan] | refresh_state | false | Not explained in file |
| [patches] | enabled | true | Enables the byte patch in 3.3 |
| [logging] | enabled | true | Writes trace logs |
| [logging] | udp_trace | false | Verbose race UDP trace. Warned to make large logs |

### 4.2 Server: `config/server.toml` (read from raw file)

Key groups only. Full table of about 80 keys is in the source file and not copied here.

- `[server]`: `PUBLIC_HOST` (default `"127.0.0.1"`; README says change before Internet deployment), `AUTH_MODE` (`"password"`), `AUTH_AUTO_ENROLL` (`false`), `AUTH_FAILURE_LIMIT` (`5`), `AUTH_LOCKOUT_SECONDS` (`300`), `ACCOUNT_DB` (`data/accounts.sqlite3`), `MESSENGER_LISTEN` (`0.0.0.0:13505`), `MESSENGER_IPC_LISTEN` (`127.0.0.1:13506`), `WEB_LISTEN` (`0.0.0.0:20923`), `RACE_RELAY_LISTEN` (`0.0.0.0:20000`), `RACE_VIRTUAL_NETWORK` (`100.64.0.0/10`), `IPC_SECRET` (`"AUTO"`).
- `[most_wanted]`: `CREATE_ACCOUNT_ON_FIRST_LOGIN` (`false`), `BOOTSTRAP_LISTEN` (`0.0.0.0:30921`), `LOBBY_LISTEN` (`0.0.0.0:30920`), `DIRECTORY_SESSION` and `DIRECTORY_MASK` (both empty).
- `[underground2]` and `[carbon]`: other game ports, Carbon ad and DLC settings. Not relevant to MW.

Configuration verdict: the MW client has a small, plain INI. Server config is larger and mostly protocol and operations tuning.

## 5. How we could implement it in nfs-mw

Spec-first. Because the repo is AGPL-3.0-or-later, no code, tables copied verbatim, or comments may be reproduced. The steps below describe behaviour and facts only.

### 5.1 Does it apply to a clean-room rewrite?

- The ASI loader entry, the INI-redirect of host and port, and the byte-signature patch are loader-specific or binary-specific hacks. They fit the current NFS:MW install-discovery and patching path only as an optional, explicitly-enabled compatibility layer. They do not apply to the Rust rewrite's core engine.
- The server half (lobby, messenger, race relay, accounts) is protocol emulation. It is a separate project from the rewrite. It applies only if the rewrite ever targets online play.

### 5.2 Behaviours to specify

- B1. Client redirection: resolve configured host and port per service (bootstrap, lobby, control, messenger alias, race) and send the game's network traffic there. Specify which game sockets/UDP flows each service corresponds to.
- B2. Start-up ordering: load configuration, initialise logging, then apply redirection and optional patches, then start periodic refresh.
- B3. Optional LAN entry: inject a named server entry into the game's server browser. Specify the entry fields (name, racer count).
- B4. Optional byte patch: a signature-located conditional-branch flip, gated by config and logged. Specify the target branch, its purpose and the behaviour if the signature is missing. Must be verified against our own disassembly, not theirs.
- B5. Server-side (if ever in scope): account login with lockout, messenger relay, race relay with virtual network range, news/terms endpoint.

### 5.3 Data and formats needed

- Per-service host and port table (our own INI schema).
- Our own signature list for NFS:MW, derived from our own disassembly or from the install under the user's control. No signatures copied from this repo.
- Server protocol message formats, sourced from our own captures and the spec, not from this repo's server code.
- Game install discovery already exists (`game-install`). The redirection must use it, not open files itself.

### 5.4 Natural home

- Engine-generic code goes in `libs/`: config parsing, address and port table, signature scanner, logging. Must stay Bevy-free.
- NFS:MW-specific code goes in `crates/`: the MW service table, the MW byte signatures and the LAN entry. `crates/` depends on `libs/`, never the reverse.
- Server emulator, if built: a separate crate or tool, not in `libs/`.
- Format parsing takes `&[u8]`. No NFS:MW names, paths or defaults in `libs/` (AGENTS.md rule).

### 5.5 Risks

- Licence: AGPL-3.0 contamination if any code or tables are copied. Mitigation: spec from our own observations only.
- Trademark: EA names. Keep the project unofficial and avoid EA branding.
- Legal and operational: running an online emulator for live players is a separate policy question. The repo itself states it is unofficial and asks for donations; we have no stated position on that.
- Binary patching fragility: signatures break on other builds. Need per-build verification.
- Loader dependence: manual ASI install and loader behaviour differ by environment (see README note: do not combine with older hooks patching the same code).
- Security: the messenger IPC and DLC page must stay on loopback unless behind a secured reverse proxy (README). Any port we open needs the same care.

### 5.6 Recommended action

- Do not implement now. Log it as a research item under the online-play roadmap if one exists.
- If pursued: write a spec under `docs/` for the redirection behaviour (B1, B2) first, with observed traffic from our own captures.

## 6. Open questions and what could not be verified

- Server code (`server/`, `nfs_online.py`, `start.sh`, `stop.sh`) was not read. All server behaviour above is README- or config-level.
- The `libs`-style socket rewrite (`state.zig`, `profile.zig`, `lan.zig`, `socket_runtime`) was not read. The mechanism of socket redirection is unverified.
- The ASI loader used is not named in the repo. The "Ultimate ASI Loader" guess is [inferred] only.
- The certificate branch purpose is inferred from a log message.
- Which exact NFS:MW executable version(s) the signatures target is not stated.
- Latest commit date: not visible in the fetch. Repo activity cannot be confirmed as current.
- Protocol details (message formats, FESL/Theater framing) were not read.
- README says there is a roadmap in `TODO.md`. Not read.
- The `tests/` folder, `source/client-zig/src/shared` and `src/u2` / `src/carbon` were not read.

## 7. Sources

- https://github.com/moio9/NFS-Online-Server (repository page, file listing, description, licence, stats)
- https://raw.githubusercontent.com/moio9/NFS-Online-Server/main/README.md (README, via WebFetch summary)
- https://raw.githubusercontent.com/moio9/NFS-Online-Server/main/config/server.toml
- https://raw.githubusercontent.com/moio9/NFS-Online-Server/main/clients/most-wanted/net_mw.ini
- https://github.com/moio9/NFS-Online-Server/tree/main/source/client-zig (directory listing)
- https://github.com/moio9/NFS-Online-Server/tree/main/source/client-zig/src (directory listing)
- https://github.com/moio9/NFS-Online-Server/tree/main/source/client-zig/src/mw (directory listing)
- https://raw.githubusercontent.com/moio9/NFS-Online-Server/main/source/client-zig/README.md
- https://raw.githubusercontent.com/moio9/NFS-Online-Server/main/source/client-zig/src/mw/patches.zig
- https://raw.githubusercontent.com/moio9/NFS-Online-Server/main/source/client-zig/src/mw/main.zig
- https://raw.githubusercontent.com/moio9/NFS-Online-Server/main/source/client-zig/src/mw/hooks.zig (returned HTTP 404; file does not exist at that path)
