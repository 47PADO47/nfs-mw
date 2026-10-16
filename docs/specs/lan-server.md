# LAN and online server launcher (NFSLAN)

How the original PC game's `server.dll` is started outside the game, how the community launcher patches it
so that it works on any network, and what that tells us about the online lobby protocol. Nothing here is
implemented in the rewrite. This is a reference for a future decision (see [Relevance](#relevance-to-the-rewrite)).

## Status

Closed at **unverified**, by the project owner's decision. Re-read on 2026-10-16: the client path is in
[Client path](#client-path-2026-10-16). Traced and recorded: the request and reply path, the HTTP framing,
the peer lookup (`peerIP`, `peerPort`), the UDP receive and dispatch, and the ack mechanism. Resolved in code:
the datagram layout is 25 bytes on both sides. Not verified: any wire-level behaviour, since no capture
exists. No protocol client or server exists. The tools here are a patch-set checker, an HTTP framing probe
and a datagram codec whose layout is wrong (see [its note](#cob-datagram-codec-wrong-layout)). A capture of
a real session would confirm the layout and the game payload.

## Sources read

- **[xan1242/NFSLAN](https://github.com/xan1242/NFSLAN)**, MIT License, Copyright (c) 2023 Lovro Pleše
  (Xan/Tenjoin). Read at commit `1191be5e3126c167c17e1c383477f63d9cc36495` (last commit 2023-06-20).
  Files read: `README.md`, `NFSLAN.cpp`, `Network.h`, `injector/injector.hpp` (only the `MakeCALL`,
  `MakeNOP` and `MakeInline` helpers). The `injector/hooking/` pattern library is a dependency and was not
  studied.
- **Original PC `speed.exe`** (Need for Speed: Most Wanted Black Edition install), read in Ghidra for strings
  only. Used to check which client-side strings exist in the stock game. **[verified]**
- Not read: `server.dll` itself. It is EA's binary and is not in the repo. Every server address below comes
  from the launcher's comments and patch patterns, so it is **[community]** until checked in Ghidra.

Permission and provenance: NFSLAN is MIT, so its code could be ported with its notice, as
[licensing.md](../licensing.md) allows for permissive sources. This spec records facts only and describes the
behaviour in our own words. No code is copied.

## Purpose

NFS:MW's online mode was built on EA's servers, which are offline. `server.dll` is the game's own server
component (the same binary the original shipped with). It can host a lobby on a LAN. NFSLAN loads that DLL,
changes a set of its code paths in memory, and starts it, so that clients on different networks can reach
the server and each other.

Two games use this DLL: Underground 2 and Most Wanted. Carbon's copy is "slightly different" according to the
source comments. The launcher **only patches MW's server** today. UG2 is detected and skipped.

## Components

| Part | Role |
|---|---|
| `NFSLAN.exe` | Console launcher. Loads `server.dll` from its own folder, patches it, starts it, waits for Ctrl+C |
| `server.dll` | The game's server. Exports `StartServer`, `IsServerRunning` and `StopServer` (**[community]**) |
| `server.cfg` | Server settings read by `server.dll`. Format and keys in [Configuration](#configuration) |
| Client plugin (LanIP) | A patch to the game client that answers the local challenge on UDP 9901. Not part of `speed.exe` (see [Client requirements](#client-requirements)) |

## Startup sequence

1. The launcher needs `server.dll` in its own folder. Without it, it stops with an error.
2. It loads the DLL and looks up the three exports by name. Missing any of them is a fatal error.
3. Unless the second argument is `-n`, it patches the DLL. The base address is taken from the loaded module,
   and the DLL's name check decides which patch set to apply (Underground 2 is detected by the default server
   name in the DLL and skipped).
4. It installs SIGINT and SIGTERM handlers that call `StopServer` before exit.
5. It calls `StartServer(name, 0, null, null)`, then checks `IsServerRunning`.
6. It sleeps in a loop while the server runs. On exit it calls `StopServer` if needed.

Command line: `NFSLAN <servername> [-n]`. The name is the server's display name. `-n` disables all patches,
which leaves the stock server (LAN play only).

## Patch sites in `server.dll` (MW)

The launcher finds each site with a byte pattern, then overwrites the code with an inline hook, a call or a
NOP. Addresses are the ones in the source comments, for the default base `10000000`. Patterns are stored in
the code, so the whole set depends on this exact DLL build.

| Address (approx.) | Patch | What it does, in our words |
|---|---|---|
| `1001DC8B` | Inline | Reads the listen address and a second field from the connection object. Prepares the address that is sent to clients |
| `1001DCAD` | Inline | Reads the port from the same structure, so the advertised port is the configured one |
| `10006ABE` | NOP (7 bytes) | Skips the code that invalidates the server's IP address |
| `10007294` | Inline | Stores the advertised address and sets a sentinel value in the connection structure |
| `1001BEDF` | Inline | Reads the **SKU** address that the client sends. If it differs from the address the connection came from, records a redirect (see below) and starts a local challenge |
| `1000AB32` | Inline | When sending the lobby address to a player, substitutes the redirect address for the other player's connection address |
| `1000AB03` | Inline | Same substitution on a second send path |
| `100099EF` | Inline | On disconnect, removes the player from the local list and the redirect table |
| `1001363D`, `10013C95` | Inline | Remember the destination address of the current lobby message and clear two fields in the message |
| `10026514` (+ `0x21`) | Call replacement | Replaces two calls to the lobby address routine with a wrapper that applies the redirect rules. The original routine is kept and called through a pointer |

Two globals are declared in the source but never used (`who_func` at `1000AAD0`, `packet_buffer` at
`10058A5C`). They look like leftovers from earlier work.

Underground 2: the patch function exists but only prints "not yet implemented".

**Checked against the install [verified]:** the installed `server.dll` (MW, dated 2005-11-15, 380,928 bytes,
image base `10000000`) contains each of the ten byte patterns exactly once, at the address the launcher
expects. The launcher's patch set therefore matches this build. Scan script: scratchpad
`nfslan_check.py` (not committed). The three exports `StartServer`, `IsServerRunning` and `StopServer` are
present by name.

## The redirect model

The server keeps two tables, keyed by IPv4 address as a 32-bit integer.

- **`RedirIPs`**: connection address → the address the client says it has (its SKU address). Filled when a
  client's SKU address differs from its connection address. This is the usual case for a client behind NAT,
  where the server sees the public address and the client knows its LAN address, or the reverse.
- **`LocalUsers`**: addresses that answered the local challenge (see below). They are on the server's own
  network.

Rules when the lobby addresses a player (applied in the `hkLobbyAddr` wrapper and the two `TestLocalSKU`
hooks):

- If the destination is a local user and the other player is local too, the connection address is used
  as-is. Local players can reach each other directly.
- If the other player has a redirect entry, the redirect address is used. This sends a remote player's
  traffic to the address that they can actually be reached on.
- Otherwise the address is unchanged.

The result: a player who is on the server's LAN gets the LAN address for others on the LAN, and a public
address for remote players. Mixed local and online play is the goal here. The README's TODO says it only
works for one local player.

## Local challenge

When a client's SKU address differs from its connection address, the server asks the connection address
whether it is on the same network as the server. A thread is started for each new redirect (the source uses
a detached `std::thread`).

- Transport: UDP, to port **9901** on the client's connection address.
- Request: 4 bytes, the little-endian value `0x6A093EC9`. The source calls this the hash of the string
  `"LOCAL?"`.
- Reply expected: 4 bytes, `0x8DB682D1`, the hash of `"YESIMLOCAL"`.
- Timeout: 1 second for the reply. Any socket error is printed and the client is left out of `LocalUsers`.
- Result: a match adds the address to `LocalUsers`. Silence or a wrong value leaves it out.

Notes:

- The launcher does not check the reply size or content beyond the 4-byte value. A reply longer than 4 bytes
  is read into a 4-byte buffer. `UDPSocket::RecvFrom` then writes a terminating zero at `buffer[ret]`, which is
  out of bounds whenever `ret` is 4 or more. See [Known problems](#known-problems).
- The hash values are the same as strings in the client plugin's protocol. Only the hashes appear in the
  source, not the strings.

## Client requirements

Per the README, the game client needs these changes to play online through an NFSLAN server.

- **SKU tag** must return the client's external IP. The server uses this to build redirects.
- **UDP responder on port 9901** that answers the `LOCAL?` query with `YESIMLOCAL`. Needed when the SKU
  address and the connection address differ.
- **No UDP bind** for the game client (not the lobby). Without this, the client's own socket conflicts with
  the server's packets.
- Optional: ignore the server-returned slave address on the client and always use the public address that
  the client connected to.

Ghidra check on the stock PC `speed.exe` **[verified]**:

- The string `"LOCAL?"` does not exist as a string in the binary. Neither does `"YESIMLOCAL"`. The challenge
  therefore lives in a client patch (the LanIP plugin), not in the stock game. The stock client cannot answer
  it.
- The string `"SKU_UNKNOWN"` exists (address `008A88FC`), which is the game's name for the SKU value. The
  search for `"sku"` found nothing else. The SKU value is read at runtime, so the code that reads it has to be
  found from its use, not from the name.
- The string `"server.dll"` does not occur in `speed.exe`. The launcher is the only place in this project
  that names the DLL.
- The string `"localhost"` exists (`008C7050`), used by the game's own network code.

### Client SKU path in `speed.exe` (Ghidra, partly verified)

- **SKU getter, `0x0064A3A0`** **[verified in disassembly]**: calls a predicate at `0x006CBB40`. If the
  predicate returns zero, the function returns the string at `0x008A8908` (`"14705"`). If it returns non-zero,
  it returns `"SKU_UNKNOWN"` (`0x008A88FC`). Only one code reference to `"SKU_UNKNOWN"` exists in the file
  (a file scan, not a Ghidra xref query).
- **Predicate, `0x006CBB40`** **[verified in disassembly]**: reads the global at `0x008FB574`. If that is
  `-1`, it returns 0. Otherwise it returns the global at `0x00982D24`. So `"14705"` is returned while the
  state is unset, and `"SKU_UNKNOWN"` once the second global is non-zero.
- **Writers of the globals, `0x006CB680`** **[verified in disassembly]**: a registry loader. It reads the
  game's `HKEY_LOCAL_MACHINE` values, including `Install Dir`, `CD Drive`, `FirstTime`, `CacheSize`,
  `SwapSize`, `Language` and `StreamingInstall` (under `Software\EA Games\Need for Speed Most Wanted`), and
  the `ergc` subkey (`Software\Electronic Arts\EA Games\Need for Speed Most Wanted\ergc`). It then copies
  two fields from a 12-byte-per-entry table at `0x008FB578` into `0x008FB574` and `0x00982D24`, using an
  index found by comparing registry strings against table entries. The comparer at `0x007E6630` is a
  case-insensitive string compare that returns zero on a match **[verified in disassembly]**. With no matching entry, `0x008FB574` is set to `EDI` (not yet traced). So the SKU string depends on
  the install's registry state, not a fixed value.
- **Formatter, `0x0083C880`** **[verified in disassembly]**: not a printf. It copies a source string into a
  destination buffer of a given size, wrapping a field in double quotes when it contains a space or comma,
  escaping as it goes, and trimming trailing whitespace. The message is therefore a list of quoted fields,
  not a `key=value` line. The field order is set by the builder above.
- **Callers of the getter:** `0x00794A52` and `0x0079D622`.
- **Message builder, `0x007948A0`** **[verified in disassembly]**: calls the getter at `0x00794A52`, then
  formats a request out of fixed field names. The names found as literal strings nearby are `REGN`, `CLST`,
  `NETV`, `FROM`, `LANG`, `MID`, `VERS`, `PROD`, `SKU`, `SLUS`, `AUTH`, `WHEN`, `REPT`, `RESU`, `VENUE`,
  `TEAM%d`, `ROOM%d`, `NAME%d`, and the build date `Nov 21 2005`. The code then calls a send-and-wait helper
  (`0x00844340`, timeout `1000`) and a connect or request helper (`0x00841800`). The message format and which
  field carries the SKU were **not** pinned down: the formatter's argument order is not confirmed yet.
- **Lobby-list builder, `0x0079D550`** **[verified in disassembly]**: loops over a table of up to 16-byte
  items at `0x009B475C` to `0x009B479C` and formats `NAME%d`, `TEAM%d`, `ROOM%d`. It also calls the getter
  (`0x0079D622`) and sends the result with the key `"rank"` (`0x72616E6B`) via `0x00793340`.

### Request and reply path (Ghidra, partly verified)

- **Request driver, `0x007A01C0`** **[verified in disassembly]**: takes a lock, calls the builder
  `0x007948A0` on the object returned by `0x00794F80`, and on success calls the send stage `0x0079C9C0`.
- **Send stage, `0x0079C9C0`** **[verified in disassembly]**: calls `0x0078FFB0`, a table insert keyed by a
  message type from 0 to 5 that stores a callback and a tag. It registers the reply handler `0x0079BDE0`.
- **Reply handler, `0x0079BDE0`** **[verified in disassembly]**: reads a 4-byte message tag at offset 8 and
  dispatches on it. The tags, as MSVC multi-character constants (the bytes read left to right):
  `game`, `play`, `$rup`, `$rfn`, `rank`, `rtim`, `self`, `stim`. The `game` and `play` branches check a
  record id at offset 8 against `-1`. *Inferred from the loops, not yet confirmed:* `$rup` and `$rfn` fill a
  table at `0x009B9558` from quoted fields, and `rank` fills tables at `0x009B95F8` onwards, with the
  `NAME%d`, `TEAM%d` and `ROOM%d` keys in the same routine.
- **Transport, `0x00866B50`** **[verified in disassembly]**: a `sendto` wrapper (WS2_32 ordinal 20) that
  takes a socket, a port and a buffer. The receive side uses `recvfrom` (ordinal 17), and `closesocket`
  (ordinal 3) is at `0x00866B38`. The caller at `0x00864D30` sends a fixed **25-byte** UDP datagram: five
  dwords copied from the peer record at `+0x28`, plus a flag at `+0x44` (set when a field is greater than
  zero). Its callers at `0x00864DB9`, `0x00864DE3` and `0x0086571E` send **4-byte** datagrams from the
  global buffer `0x0090D4FC`, and `0x0090D503` holds a flag byte copied from the peer record at `+0x44`
  **[verified in disassembly]**. The 4-byte messages are probably acknowledgements or keep-alives, which
  matches the `PINGTIME`/`PINGWAIT` keys in `server.cfg`. This is inferred, not confirmed. The caller at
  `0x00865020` calls a function pointer at `+0x08` of a socket object, a generic callback path. The
  framing of the 25-byte datagram was traced on 2026-10-16 (see [Client path](#client-path-2026-10-16)).
- **Scheduler, `0x00865490`** (earlier note said `0x00865470`, which is inside the handler's tail). For a peer
  record, if `+0x28` is non-zero and more than `0x1F4` (500) ms have passed since the last send, it sends the
  25-byte identity datagram in state 1 (`0x00864D30`) or the 4-byte keep-alive otherwise (`0x00864DD0`).
  Confirmed in code on 2026-10-16; the 4-byte tag is `PNG`, not `DSC`.
- **Field lookup, `0x0083A670`** **[verified in disassembly]**: finds a `KEY=VALUE` pair in a text
  payload. It compares keys case-insensitively through a fold table at `0x008C6508`, skips spaces around
  the key, and returns a pointer to the value after the `=`, or zero when the key is absent. So the reply
  payload is a list of `KEY=VALUE` fields.
- **SKU request field** (inferred from the call pattern, medium confidence): the builder at `0x007948A0`
  writes the key string `"SKU"` (`0x008B6D94`) and the getter's result as its value, with the same writer
  used for `REGN`, `CLST`, `NETV`, `LANG`, `VERS` and `PROD`. The SKU is therefore a `SKU=<value>` field,
  where the value is `14705` or `SKU_UNKNOWN`, depending on the registry state above.
- **Lobby address, the peer lookup is an HTTP request** **[verified in strings and disassembly]**:
  the client holds the URL template `/getPeerAddress?myIP=%s&myPort=%d&version=1.0` (`0x008C8330`). The
  routine at `0x00863B20` fills it with the local IP and port, using `0x0083FAF0` for the address, into a
  260-byte buffer, and passes it to a request sender at `0x00863A40`. So the lobby address is looked up
  with a plain HTTP GET, not carried in the UDP packets above.
- **Request format** **[verified in strings and disassembly]**: `0x00863A40` builds this HTTP/1.1 request
  (format at `0x008C8240`) and passes it to `0x008638A0`:

  ```
  GET <path> HTTP/1.1
  Cookie: sessionID=<session>
  Accept: */*
  Content-Length: 0
  User-Agent: Custom/1.0
  Host: <host>
  ```

  The path is `/getPeerAddress?myIP=<ip>&myPort=<port>&version=1.0`. The config key `NetworkLobbyIP`
  (`0x008B6720`) selects the host. Its default is `*pcnfs06.ea.com` (`0x008F42C8`), a host that EA no longer
  runs. The `*` prefix **[verified in disassembly]**: the host string is copied and cut at the first `/` (the
  `0x2F` search at `0x00849460`), and then the code at `0x00849488` checks for a leading `*` and skips it.
  So `*pcnfs06.ea.com` is resolved as `pcnfs06.ea.com`. The meaning of the marker itself (a wildcard, or a
  flag for host-name lookup) is not confirmed; the client only skips it.
- **Send path, `0x008638A0`** **[verified in disassembly]**: rejects a request of 256 bytes or more, copies
  it into the connection object (buffer at `+0x0C`), uses a state value at `+0x114` (0, 1, 3 or 7), and
  arms a 30-second timeout (`0x1E`, the same value as `PINGWAIT`). The state machine that reads the reply
  is at `0x00863260`.
- **Response framing, `0x00863260`** **[verified in disassembly]**: the reply must start with the four
  bytes `HTTP`. The status code is read as decimal digits after the first space and stored at `+0x11C`.
  Headers end at a blank line (`\r\n\r\n`). The client looks for two header names in the reply, both with
  a leading newline: `\nContent-Length:` (`0x008C75F0`) and `\nConnection: close` (`0x008C75B0`). The
  decimal `Content-Length` value sets the body size. The body is then read and stored. What the body
  contains (and how it is parsed further) is not yet traced. `0x00863230` is only a `recv` wrapper with a
  timer **[verified in disassembly]**. The state machine also parses a decimal chunk size and looks for
  `\r\n\r\n` after it, so chunked replies are handled as well. The body is stored in a buffer at `+0x130`,
  and the routine that consumes that buffer in state 7 has not been located yet.
- **Result poll, `0x008646A0`** **[verified in disassembly]**: returns 1 when the object's state at `+0x238`
  is 2 (done), 0 while it is 1 or 4 (pending), and -1 otherwise. When done, it copies two dwords from
  `+0x10` and `+0x14` of the object into two caller-supplied output pointers, then resets the state to 0.
  Those two dwords are the strongest candidate for the lobby address and port. They are not confirmed.
  The caller `0x0085C891` requests with a 6000 ms timeout (`0x1770`) and passes the result to `0x0085C8B1`.
  The writer of state 2 is `0x0086463F` **[verified in disassembly]**: it sets the state to 2 when the
  body check `0x00863D90` returns non-zero, and to 3 otherwise.
- **Reply body consumer, `0x00863D90`** **[verified in disassembly and key strings]**: the reply body is
  text. The routine searches it for the key `peerIP` (`0x008C8374`), skips to the `=`, reads up to 31
  characters up to CR or LF, and converts them as a dotted-quad IPv4 address (`0x0083FB40`), which it
  stores at the object's `+0x10`. It then does the same for the key `peerPort` (`0x008C8368`), converts with
  `0x007C6380` (decimal, probably `atoi`), and stores at `+0x14`. It reports success only when both keys
  are found.
  So the lobby lookup returns a body with lines `peerIP=<ipv4>` and `peerPort=<decimal>`. This resolves the
  body and lobby-address questions.
- **Callers of the request path** **[verified]**: the send path `0x008638A0` is called from `0x00863A82` and
  `0x00863AFF`. The state machine `0x00863260` is called from `0x00863A28`, `0x008645E8` and `0x00864675`.
  The URL builder `0x00863B20` is called from `0x008644E9`, `0x008645B1` and `0x00864619`. The routine at
  `0x00864500` clears the object's `+0xE0` pointer and copies strings into it, and it pushes the constant
  `"bind"` (`0x62696E64`), the same token used by `%%bind(...)` in `server.cfg`. A shared config-expression
  syntax is a plausible explanation for the `*` prefix on the default host. This is not confirmed.
- **Lobby address source** **[verified in strings and disassembly]**: `0x0078AC80` reads the config keys
  `LobbyServerAddr` (`0x008B6710`) and the indexed keys `MADDR%d`, `LADDR%d` and `ADDR%d` (`0x008C6D5C`,
  `0x008C6D64`, `0x008C6D6C`), with a default string at `0x008F42C8`. Those are the same names the server
  uses in `server.cfg` (`MADDR`, `LADDR`, `ADDR`), so the client reads the server's own key names. The
  global written by this path is `0x009B4174`. The lobby address is probably held in the object behind
  `0x009B4174` (inferred, not confirmed), and its response format is still unknown.
- **UDP receive, `0x00865600`** **[verified in disassembly]**: reads one datagram into a 1028-byte buffer
  (`recvfrom` via `0x00866BC0`). It matches the sender's 8-byte socket address (family, port and IPv4)
  against the peer table, which has records of `0x45C` bytes starting at `+0x14` of the owner. If the
  sender is a known peer, the datagram goes to `0x008652D0` with the payload length `n - 6`. Otherwise it
  is compared with the 3-byte tag at `0x0090D4F8` and, on a match, goes to `0x00865540`.
- **Datagram tags in `.data`** **[verified in static bytes]**: `0x0090D4F8` holds `C O b` (`COb`) and
  `0x0090D4FC` holds `D S C` plus a NUL (`DSC`). The 4-byte sender at `0x00864DB9`, `0x00864DE3` and
  `0x0086571E` sends the `DSC` block. `COb` is the tag that unknown senders must present. The meaning of
  each tag is inferred from its use, not confirmed against traffic.
- **Datagram handler, `0x008652D0`** **[verified in disassembly]**: for each datagram from a known peer it
  updates the peer's counters (`+0x450`, a sequence count, and `+0x454`, a running total) and a time
  difference from `+0x14` (a division by a constant). If the datagram is from a new address, it resets
  `+0x450`. Then it compares the datagram's tag at offset 4 with `COb` (`0x0090D4F8`), `0x0090D500` and
  `0x0090D504`, and `0x0090D4FC` (`DSC`), and acts on the match:
  - `COb`: when the peer state at `+0x28` is 1, it calls `0x008650A0` (a reply). It copies five dwords
    from datagram offset 8 into the peer record at `+0x10`..`+0x20`, reads a byte at datagram offset `0x1C`
    into `+0x458`, and if that byte is non-zero it calls `0x008650A0`.
  - `DSC`: calls `0x00865140`.
  This reads as: a `COb` datagram carries five dwords and a flag, and the flag decides whether to
  acknowledge. The 25-byte packet fits 4 header bytes, five dwords (20 bytes) and a flag byte. The offsets
  are inferred from the code, not confirmed against traffic.
- **Handler `0x00865540`** **[verified in disassembly]**: scans the peer table (records of `0x45C` bytes)
  and compares a text name at the datagram's offset 8 with each record's name at `+0x10`, two bytes at a
  time, until a double-zero end. On a match it stores the record index and forwards to `0x008652D0`.
- **Reply, `0x008650A0`** **[verified in disassembly]**: marks the peer as acknowledged. It sets `+0x24`
  to 1 and `+0x28` to 2, sets the peer's bit (`1 << index`, where index is the peer's slot in the table) in
  the mask at `+0x18` and in the pending mask at `+0x1C`, then calls `0x00865020`. That routine sends the
  pending entries through a callback stored at `+0x08` of the object, one per peer whose bit is set. The
  bit-per-peer masks show the acknowledgements are batched, not per datagram.
- **Ruled out as a parser:** `0x007932C0` is a cleanup routine. It walks a linked list at `+0x30` and frees
  the object at `+0x40` **[verified in disassembly]**. It is not where reply fields are split.
- **Not yet traced:** the wire framing (how a message is split into fields and records), the layout of the
  lobby-address field, and which branch sets the global used by `hkLobbyAddr`. The launcher does not
  touch these; they are inside `server.dll`.

Still open: which field of the request carries the SKU (the builder's argument order is not yet traced),
the table that the registry loader searches (its string contents and the `EDI` default), and where the reply
carrying the lobby address is parsed. Next step: trace the builder's arguments at `0x007948A0` and the reply parser.

## Client path (2026-10-16)

A Ghidra pass on the stock `speed.exe`, to test the reasons in [ADR 0006](../decisions/0006-multiplayer-netcode.md)
section 15. Trail and addresses: [provenance/lan-client.md](../provenance/lan-client.md).

**Discovery.**
- The stock client has no UDP 9901 responder, no `LOCAL?` string and no `YESIMLOCAL` hash constant. Byte searches
  for both hashes and for the port in the usual immediate forms found nothing **[verified]**.
- Server lookup is the HTTP GET above, and its reply gives `peerIP` and `peerPort` **[verified]**. The default host
  is a dead EA name; `NetworkLobbyIP` replaces it **[verified]**.
- The peer socket is a plain UDP socket bound to `0.0.0.0` on a local port, non-blocking. The connect routine binds
  the literal port 6000 **[decomp]**. No broadcast option is set on this path **[decomp]**, so no LAN broadcast
  discovery is visible here.

**Datagrams after the join.** Tag at offset 0 **[decomp]**. The scheduler is at `0x00865490`, not `0x00865470`.

| Tag | Size | Sent when | Content |
|---|---|---|---|
| `COb` | 25 B | every 500 ms of silence while the peer is in state 1 | byte 3: session flag (1); bytes 4 to 23: the sender's 20-byte name; byte 24: 1 once the sender has heard from the peer |
| `PNG` | 4 B | every 500 ms of silence once acknowledged | keep-alive |
| `DSC` | 4 B | disconnect, or a reply to an unknown non-`COb` sender | none |
| `MIC` | 6 + n B | queued data; at once if the pending byte is 1, else after 99 ms | tag, channel byte (0), two bytes, then n payload bytes (about 1,060 at most) |

- **The layout is 25 bytes on both sides.** The receiver's pointer sits 4 bytes before the datagram, so the earlier
  29-byte reading was a misreading. This refutes the layout premise of ADR 0006 section 15, reason 1.
- **Handshake.** Peers start in state 1. A `COb` with byte 24 set, from a peer in state 1, moves it to state 2. A
  `DSC` drops the peer. A peer silent for 10 s in state 1 is dropped; in state 2 the timeout is the session value at
  `+0x40` (not yet traced).
- **Game state.** No car state, inputs or snapshots appear in `COb`, `PNG` or `DSC`. `MIC` is an opaque payload
  handed to a session callback. Its content and rate per message type are unknown.
- **Authentication.** None. A known peer is matched by source IP and port. An unknown sender is accepted only with a
  `COb` whose name equals a peer name already in the table. Names are plain text, with no key, MAC or nonce, so a
  host that can send UDP with a known peer's source address can forge its datagrams **[decomp]**.

The consequences for ADR 0006 are in the reverse-engineering note of
[integration](multiplayer/integration.md). Open items are at the end of this file.

## Configuration

`server.cfg`, read by `server.dll`. The install ships one (**[verified]**, read locally, not copied into the
repo). Its own comments explain the timing keys:

| Key | Shipped value | Meaning (from the file's comments) |
|---|---|---|
| `PINGTIME` | 20 | Seconds between server pings to each client |
| `PINGWAIT` | 30 | Seconds a client has to answer a ping before it is logged out |
| `TIMEIDLE` | 0 | Minutes a client may be idle before logout. 0 = no idle timeout |
| `TIMEMAXM` | 0 | Maximum minutes a client may stay connected. 0 = no limit |
| `LAN_THROTTLE` | 30 | How many times per second the server services requests (1–1000). Lower = faster game, slower replies |
| `TRUST` | `255.255.255.255` | Trusted address (meaning not stated in the file) |
| `TRUST_MATCH` | `%%bind("159.153.253.82")` | Address match for the trust check, bound to a fixed address |
| `ACCOUNT`, `MASTER`, `SLAVE`, `REDIR` | 1 | Service switches. The file says not to change them |
| `ADDR` | `%%bind("159.153.253.82")` | Interface to listen on. "Any publicly routable IP", never contacted |

The shipped file's ADDR is a fixed public-looking address, not the README's `%%bind` example. It is the
value the installed game expects, so the launcher's README and this file differ. The README keys below were
written against an older config and are not confirmed against this one.

| Key | Meaning |
|---|---|
| `PORT` | Listen port. The most important key |
| `ADDR` | Listen address. Also sent to clients as the "slave" address, whatever the config comment says |
| `MADDR`, `MPORT` | Probably the master server address and port |
| `RADDR`, `RPORT` | Probably the redirector address and port |
| `LADDR`, `LPORT` | Probably the lobby server address and port |
| `AADDR`, `APORT` | Probably the account server address and port |
| `%%bind` | Bind to the local address range |
| `%%dns`, `%%host` | Host names instead of raw addresses |
| `LOGCONNECTIONS` | `1` prints redirector connection logs to the console |
| `log.level` | Log verbosity, 4 by default, README says up to 10 (not checked) |
| `log.categoryMask` | Log category filter |
| `scheduler.type` | `select` by default. Other values unknown |
| `PERIP`, `RXDEL`, `RXLEN`, `scheduler.maxFd`, `scheduler.maxEvents` | Unknown |

The DLL also has a small scripting syntax (`if`, `include`, variables), with a table at `0x10029AF0`. The
README lists the keys found there. Not studied further.

### COb datagram codec (wrong layout)

`tools/lan_udp_codec.py` implements a 29-byte `COb` layout (tag at 4, flag at 28), read before the client path
was traced. The real layout is 25 bytes with the tag at offset 0. The codec must be changed to match or removed.
Its self-test checks only its own layout, so it proves nothing about the game.

### HTTP framing tool

`tools/lan_http_probe.py` implements the request and response framing above (stdlib only).
`--selftest` checks it against a local stub server: request format, the 256-byte limit, status parsing,
`Content-Length`, and the peer lookup. `parse_peer` reads `peerIP=` and `peerPort=` as the client does
(`0x00863D90`), and rejects a body with either field missing or a malformed dotted quad.

### Checker tool

`tools/lan_server_check.py <game folder>` (stdlib only) runs the same checks as the section above on the
installed `server.dll` and prints the `server.cfg` keys. It exits 0 when the patch set matches. Nothing is
written, and no game file is copied into the repo.

## Known problems

From reading the source and README. None are confirmed in game.

1. **Out-of-bounds write in the challenge reader.** See [Local challenge](#local-challenge).
2. **Global state with no locks.** `RedirIPs` and `LocalUsers` are written from the server's network thread
   (the patch hooks) and from detached challenge threads. Access is not synchronised.
3. **One local player.** The README says mixed local and online play works only for one local player.
4. **Stale entries.** A redirect is added on the first SKU mismatch and removed only on disconnect (hook
   `100099EF`). A failed challenge leaves the redirect in place.
5. **Pattern fragility.** Every patch site is found by bytes and its address is written in a comment, so a
   different `server.dll` build (or Carbon's) will need new patterns. A failed pattern is not checked: the
   launcher dereferences the result (`get_first(0)`) and will crash on a no-match.
6. **Single game.** Only MW is patched. UG2 prints a message and does nothing else.

## Relevance to the rewrite

The rewrite targets single-player gameplay on the original PC build (see [roadmap.md](../roadmap.md) and
[licensing.md](../licensing.md)). Online play is not on the roadmap. This spec records the one piece of
prior art that shows how the game's online layer can be run and patched, so that a future decision has
facts to start from.

Three possible routes, none chosen:

- **Ship the launcher's approach as a tool.** Load the user's own `server.dll`, apply the same patches, and
  start it. Needs no game code in our repo. If any of NFSLAN's code is reused, its MIT notice must be kept
  with it ([licensing.md](../licensing.md)). This is the lightest option, and it does not touch the rewrite.
- **Re-implement the redirect and challenge logic in Rust** from this spec, for a rewrite-hosted lobby. Much
  larger: the lobby protocol itself (the message formats, the account and master servers) is not documented
  anywhere we have read.
- **Map the client side** (SKU read, lobby address use) in `speed.exe`, so the rewrite could speak the same
  protocol as the original client. This needs Ghidra work first and is the only route that tests the protocol
  against the real game.

Any Rust implementation would follow [licensing.md § Spec-first](../licensing.md#spec-first): this spec is
the input, and a provenance record in [docs/provenance/](../provenance/README.md) would record it.

## Open questions

- The exact meaning of each `server.cfg` key, especially the master, redirector, lobby and account keys.
- Whether the lobby message format (the packets carrying the addresses) is documented in `server.dll` or only
  in the client.
- Which client functions read the SKU tag and send the lobby address. Start from `"SKU_UNKNOWN"` at
  `008A88FC` in `speed.exe` and its cross-references.
- Whether the patch addresses in `NFSLAN.cpp` match the Black Edition `server.dll`, or only the 1.3 build
  the launcher was written for. The launcher does not say which build.
- What the server does with the `who_func` and `packet_buffer` globals (`1000AAD0`, `10058A5C`). The launcher
  declares them and never uses them.
- Whether the server's DNS and host-name keys are needed for the public lobby.
- **Client path, open:** which game system writes the `MIC` payload (its length at peer `+0x2c`, pending byte at
  `+0x435`) and what it carries: car state, events, or nothing. Start from the peer table at session `+0x14`.
- The rate of the peer scheduler (its caller) and the timeout at session `+0x40`, which is not yet traced.
- The targets of the session callbacks (receive at `+0x04`, acknowledge at `+0x08`).
- The `sock` word (`0x736f636b`) sent on the connect path, and whether it is a second channel.
