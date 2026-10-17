# Provenance: the LAN client path in speed.exe

Trail behind the "Client path (2026-10-16)" section of [lan-server.md](../specs/lan-server.md). Addresses are
in the stock PC `speed.exe` (image base `00400000`). Ghidra names such as `FUN_` are used here only, as the
spec rule allows. Evidence tags as in [reversing.md](../reversing.md).

Tools used: `search_strings`, `search_byte_patterns`, `get_xrefs_to` (bulk form with `addresses=`),
`get_functions` (decompile, callers, callees), `read_memory`, `disassemble_function`, `create_function`.

Project state: two functions were created in the Ghidra database (`0x00865490` and `0x00865600`, both
`disassemble_first`). The program was not saved (`save_program` was not called).

## Addresses

| Address | Role | Evidence |
|---|---|---|
| `0x0090d4f8` | 16-byte tag table: `COb\0`, `DSC\0`, `PNG\0`, `MIC\0` | [verified] read_memory |
| `0x00865490` | Peer scheduler (170 bytes). Keep-alive timing, state timeouts, calls the senders below | [decomp] |
| `0x00864d30` | 25-byte identity sender (`COb`) | [decomp] |
| `0x00864dd0` | 4-byte keep-alive sender (`PNG`) | [decomp] |
| `0x00864e10` | Variable data sender (`MIC` header plus queued payload) | [decomp] |
| `0x00865140` | Drop a peer (or all peers) | [decomp] |
| `0x008650a0` | Mark peer acknowledged (state 1 to 2), set batch bits | [decomp] |
| `0x00865020` | Batch callback to the session on changed bits (session `+0x08`) | [decomp] |
| `0x008650d0` | Disconnect one peer: sends `DSC`, clears bits, zeroes the record | [decomp] |
| `0x00865600` | UDP receive (277 bytes; no function existed until created). Address match, then dispatch | [decomp] |
| `0x00865540` | Unknown-sender identity match (`COb` name at datagram offset 4) | [decomp] |
| `0x008652d0` | Datagram handler for a known or matched peer (444 bytes). Tags `COb`, `PNG`, `MIC`, else `DSC` | [decomp] |
| `0x00866b50` | `sendto` wrapper: destination from peer `+0x00` (IP) and `+0x04` (port) | [decomp] |
| `0x00866bc0` | `recvfrom` wrapper: writes source IP and port to the caller's pair | [decomp] |
| `0x00866a80` | Create UDP socket (`socket(2,2,0x11)`), `bind` to `0.0.0.0` on the given port, non-blocking | [decomp] |
| `0x00864af0` | Session start: sets session `+0x24 = 0x20`, `+0x44 = 1`, calls `0x00865750` | [decomp] |
| `0x00865750` | Add peer: state 1, name from 5 dwords, remote IP and port, `MIC` tag at `+0x30` | [decomp] |
| `0x008651e0` | Peer table lookup or free-slot search (stride `0x45c`) | [decomp] |
| `0x0085c510` | Connect state machine. Calls `0x00864af0` with remote IP, remote port and the literal `6000` as the bind port | [decomp] |
| `0x0085c0f0`, `0x0085c120` | Teardown paths; both call `0x00864b80` (locked `0x00865140`) | [decomp] |
| `0x0085ca00` | Caller of `0x0085c510` (not yet read) | [decomp] |
| `0x00863b20`, `0x00863a40`, `0x00863260`, `0x00863d90` | HTTP `/getPeerAddress` path (already in lan-server.md) | [verified] |
| `0x00864c30` | Called with the 4-byte word `0x736f636b` (`sock`) on the connect path. Not traced | [decomp] |

Session object offsets (byte offsets, from the code that reads them): `+0x00` socket, `+0x04` receive callback,
`+0x08` batch callback, `+0x0c` callback argument, `+0x10` peer count, `+0x14` peer table, `+0x18` and `+0x1c`
pending and acknowledged masks, `+0x24` flags, `+0x3c` bound local port, `+0x40` acknowledged-peer timeout (ms),
`+0x44` session flag (1 after `0x00864af0`), `+0x50` critical section.

Peer record (stride `0x45c`): `+0x00` IP, `+0x04` port, `+0x08` and `+0x0c` learned address, `+0x10` to `+0x20`
name (20 bytes, read as C string pairs), `+0x24` flags, `+0x28` state (0 none, 1 handshake, 2 acknowledged),
`+0x2c` queued payload length, `+0x30` outgoing buffer (`MIC\0` tag, then the channel byte at `+0x34`, then
payload from `+0x36`), `+0x435` pending byte, `+0x43c` previous send tick, `+0x440` last send tick,
`+0x444` last receive tick, `+0x450` datagrams received from this peer, `+0x458` peer-ready flag.

## The 25-byte identity datagram (resolves the layout question)

- Sender, `0x00864d30`: buffer of 25 bytes (`0x19`). Byte 0 to 2 `COb`. Byte 3 is session `+0x44`. Bytes 4 to
  23 are five dwords from session `+0x28` to `+0x38` (the local 20-byte name). Byte 24 is `0 < peer+0x450`.
- Receiver, `0x00865600`: `recvfrom` into a frame `A`. The source pair is at `A+0` (IP) and `A+4` (port). The
  data buffer is `A+16`. The length `n-6` is stored at `A+12`. The handler is called with `A+12`.
- Receiver, `0x008652d0`: its `param_3` is `A+12`. The tag compare starts at `param_3+4`, which is `A+16`,
  the first byte of the datagram. So the tag is at datagram offset 0. The name is at `param_3+8` (datagram
  offset 4). The flag is at `param_3+0x1c` (datagram offset 24). The `+0x458` byte is at `param_3+7`
  (datagram offset 3).
- Name match, `0x00865540`: compares `param_2+8`, which is datagram offset 4, with peer `+0x10`.

Conclusion: both sides use 25 bytes with the tag at offset 0. The earlier reading (tag at offset 4, flag at 28,
29 bytes) took the handler's `param_3` as the datagram start. `tools/lan_udp_codec.py` was rewritten to the
25-byte layout on 2026-10-16.

## Keep-alive and data timing

- Scheduler `0x00865490`: when the peer state is not 0 and more than 500 ms (`0x1f4`) have passed since the
  last send (`+0x440`), send `COb` for state 1 or `PNG` otherwise. Then call `0x00864e10`. The acknowledged
  timeout is session `+0x40` when state is 2, otherwise 10000 ms. If the peer has been silent longer than that
  (`+0x444`), call `0x00865140`.
- `0x00864e10`: sends the `MIC` buffer when `+0x435 == 1` (at once), or when `+0x435 != 0` and more than 99 ms
  have passed since `+0x440`. It sends only if `+0x458 != 0`. The queued length `+0x2c` is reset to 0 on every
  call, so data queued for a peer that is not ready is dropped.
- The scheduler reads `GetTickCount` (wall clock, ms), not simulation ticks.
- Length limit (estimate): the record is `0x45c` bytes and the buffer starts at `+0x30`, so one `MIC` datagram
  carries at most about 1,060 payload bytes.

## Handshake sequence (decomp)

1. Local side calls `0x00864af0` (state 1, name, remote IP and port, local bind port).
2. Each side sends `COb` every 500 ms while in state 1. Byte 3 is 1 (session `+0x44`).
3. A `COb` with byte 24 set, from a peer already in state 1, moves the peer to state 2 (`0x008650a0`). The
   byte-24 flag means "I heard from you before", so the first `COb` from a fresh peer keeps it in state 1.
4. In state 2, `PNG` keep-alives and `MIC` data flow. A `DSC` drops the peer.

## What is not in the datagrams

No car state, no inputs and no snapshot fields appear in `COb`, `PNG` or `DSC`. The only game-level
channel seen is `MIC`: a length, a channel byte and an opaque payload handed to a session callback (session
`+0x04`, with the peer name, the datagram and session `+0x0c`). Its content is not identified.

## Authentication

None found. Known peers are matched by source IP and port (`0x00865600`, peer `+0x08` and `+0x0c`). Unknown
senders are accepted only with a `COb` whose name matches a peer name in the table (`0x00865540`). A name is a
plain 20-byte string sent in clear. There is no MAC, no key, no nonce and no challenge. Unknown non-`COb`
datagrams get a `DSC` reply (`0x00865600`, last branch).

## Discovery checks

- `search_strings` for `LOCAL?`, `YESIMLOCAL`, `getPeer`, `peerIP`, `peerPort`, `NetworkLobby`, `LobbyServer`,
  `MADDR`, `LADDR`, `sku`: the `getPeerAddress` URL, the `peerIP` and `peerPort` keys, `NetworkLobbyIP`,
  `NetworkLobbyPort`, `LobbyServerAddr`, `LobbyServerPort`, `MADDR%d`, `LADDR%d` and the HTTP request format.
  No `LOCAL?` or `YESIMLOCAL`.
- `search_byte_patterns`: `C93E096A` (`0x6a093ec9`, hash of `LOCAL?`), `D182B68D` (`0x8db682d1`), `68AD260000`
  (`push 9901`), `66B8AD26` (`mov ax, 9901`). No matches.
- Socket creation (`0x00866a80`) calls `socket`, `bind` and `ioctlsocket` only. No `setsockopt` with
  `SO_BROADCAST` on this path, so no broadcast discovery is visible here.

## Dead ends

- The UDP 9901 responder and the two hashes are absent from `speed.exe`. The byte patterns cover the common
  immediate and `mov ax` encodings only. A `lea`, a table or a computed port would not match. Weight: strong
  for the common case, not a proof.
- The spec named a scheduler at `0x00865470`. That address is inside the tail of `0x008652d0` (its
  decompile ends at `0x0086548c`). The scheduler is `0x00865490`.
- `get_functions` for an address without a function returns the containing function (`0x00865470` gave
  `0x008652d0`). Check the returned `size` and `name` before using a bulk result as an entry point.
- `disassemble_function` at `0x00865490` and `0x00865600` returned "No function found". Ghidra had left them
  undefined, although they are code. `create_function` fixed both.
- Byte search for direct writes to the pending byte `+0x435` (`mov byte [reg+0x435], imm8` for six register
  encodings, `C6 86/80/81/83/87/82` with `35 04 00 00`) found nothing. Inconclusive: the search may match only
  exact encodings, and the writer may use another form.
- The `MIC` writer in the caller chain: `0x00865750` is the only writer of the `MIC` tag and of the
  initial state. Its caller `0x00864af0` has one caller, `0x0085c510`.

## Lessons

- A callee's parameter pointer can point 4 bytes before the data it reads. Here `param_3` pointed at a length
  word, and the tag was at `param_3+4`. Trace the pointer back to the call site (`A+12`) and the buffer
  (`A+16`) before reading offsets.
- Read the tag table with `read_memory` at the data address. Four 4-byte tags (`COb`, `DSC`, `PNG`, `MIC`)
  gave the message set in one call. The earlier name `DSC` for the keep-alive was wrong.
- Names from the code beat names from an older spec. The spec's "scheduler" and "25-byte record" labels were
  off by 0x20 bytes in one place.
- For a function with no entry in Ghidra, `read_memory` of 64 bytes shows the prologue (`81 ec` is `sub esp`)
  and the epilogue. Then `create_function` at that address with `disassemble_first`.
- A tag compare in a `COb`/`PNG`/`MIC` chain can hide the length check. The handler's length
  (`n-6`) is the payload length, which matches the sender's `count+6`.

## Next leads

1. Writer of the `MIC` payload: the code that sets peer `+0x2c` (length) and `+0x435` (pending byte) and fills
   `+0x36`. Find it through the peer table (session `+0x14`, stride `0x45c`) from the game's per-tick code.
2. Session callbacks: the writers of session `+0x04` (receive) and `+0x08` (batch ack). The receive callback's
   target is the game-level handler of `MIC` payloads, so its body gives the payload meaning.
3. The caller of the scheduler `0x00865490`. Its loop rate sets the real keep-alive and data cadence.
4. Writer of session `+0x40` (acknowledged-peer timeout). Compare with `PINGWAIT` (30 s) in `server.cfg`.
5. The connect path: `0x00864bb0`, `0x00864c30` (tag `sock`), `0x00864730`, and `0x0085ca00`. Decide whether
   `sock` is a second channel.
6. Session start: where the session's own name at `+0x28` is written, and what `0x0085c510`'s argument
   `unaff_EBX[0x25]` (remote port) comes from (HTTP reply `peerPort` at `0x00863d90`, or config).
7. The HTTP reply path for `peerIP`: the object `0x009b4174` and the reader `0x007a0179`.
