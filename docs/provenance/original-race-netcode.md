# Provenance: the original race netcode in speed.exe

Trail behind [original-race-netcode.md](../specs/original-race-netcode.md). Addresses are in the stock PC `speed.exe`
1.3 (image base `00400000`). Ghidra names such as `FUN_` are used here only, as the spec rule allows. Evidence tags as
in [reversing.md](../reversing.md).

Tools used (ghidra-mcp, 2026-10-21): `search_strings`, `search_byte_patterns`, `get_xrefs_to` (bulk form),
`get_functions` (decompile, callers, callees), `read_memory`.

Project state: nothing was renamed, created or saved.

No code from here is in the repository. `blackbox-net-core` was written from our own specs
([ADR 0006](../decisions/0006-multiplayer-netcode.md), [ADR 0009](../decisions/0009-multiplayer-netcode-revisions.md)),
not from these notes.

## Addresses

| Address | Role | Evidence |
|---|---|---|
| `0x008B6B4C`–`0x008B6CA8` | 16 `MSG_*` names (`MSG_U_BI_STARTRACESYNC` first, `MSG_R_SC_WELCOME` last) | [verified] strings |
| `0x008B6A04`–`0x008B6B38` | `CLIENTSTATE_*` and `SERVERSTATE_*` names; the two short ones (`*_NIS`) are below the 4-character search floor and were read with `read_memory` at `0x008B6A2C` and `0x008B6AE8` | [verified] |
| `0x0078A900` | Config key parser (`SKIPFE*`, `ONLINEENABLED`, `ISSERVER`, `NUMCLIENTCARS`, `SERVERIP`, `SERVERPORT`, `CLIENTIP%d` ×3, `NetworkLobbyIP`, ...). Called from `0x0078FA00` | [decomp] |
| `0x009B416C` | `ISSERVER` flag; read by the session update and session setup | [decomp] |
| `0x00905D98` | `NUMCLIENTCARS`; read only by `0x0056C3E0` | [decomp] |
| `0x00905D50` | `NEWCARSMOOTHING`; written by the parser, no reads | [verified] xrefs |
| `0x009B40F8` | 3 client addresses (`CLIENTIP0..2`) | [decomp] |
| `0x0078AE80` | Allocates the 0x34800-byte "Online Buffer" | [decomp] |
| `0x0056C3E0` | Direct-connect session setup: "TheHost" / "Client%d", waits for `NUMCLIENTCARS` | [decomp] |
| `0x00797610` | Online frame tick: runs a callback list under a lock, then `0x00761B30(&0x009B0FA0, 1)`, then the lobby update if lobbies are used | [decomp] |
| `0x009B0FA0` | The session object | [decomp] |
| `0x00761B30` | Per-frame session update: state switch (1 to 6), clock slew, timers, timeouts | [decomp] |
| `0x007617D0` | Session reset: state to 1, clock fields cleared, then state 2 | [decomp] |
| `0x00735EF0` | Host status message, every 4000 (4 s) | [decomp] |
| `0x00741CA0` | Player record message (8-bit and 32-bit fields), every 20000 (20 s) | [decomp] |
| `0x00741BF0` | Timing check, every 2 s: two clocks compared, strike reason 1 above 0.5 s | [decomp] |
| `0x00740FA0` | Strike counter: `u16` at player `+0x122C + 2*reason`, saturates at `0xFE01` | [decomp] |
| `0x0078F8A0` | Bit buffer init: byte 0 cleared, bit cursor `+0x400` and `+0x404` zeroed, `+0x408` set to 1 | [decomp] |
| `0x0078A310` | Bit writer: `(value, bits)`, most significant bit first, splits across bytes | [decomp] |
| `0x007353B0` | Reads `RESPONSE_GRAPH`, `MIN_CHEAT_THRESHOLD`, `RESPONSE_REMAP%d` from a lobby config | [decomp] |

Session object offsets seen in `0x00761B30` and `0x007617D0`: `+0x00` state, `+0x04` local player record, `+0x08`
four player pointers, `+0x19` "started" byte, `+0x1C` last frame time, `+0x20` clock offset, `+0x28` clock target
term, `+0x6C` and `+0x70` countdown timers (4000 and 20000), `+0x78`, `+0x7C` state timestamps, `+0xC8`/`+0xCC` the
two clocks of the timing check, `+0x114` client status timestamp (125 threshold).

Clock slew detail (`0x00761B30`, state 5, clients): `e = target - offset` computed from three time reads; if
`|e| > 100` the offset takes the whole difference; else `e >= 4` subtracts 3, `e < -1` adds 1, and anything else
subtracts `e`.

## Dead ends

- `get_xrefs_to` on the `MSG_*` strings: none. Byte searches for their addresses (as pointers or `push` operands):
  none. The names are not used by code.
- The `CARSPAM` sender: not found by name; the senders in the update (`0x007A0C20`, `0x007A10C0`) were not read.

## Next leads

- Read `0x007A0C20` and `0x007A10C0` (the send paths) and `0x0079F090` (likely the session manager getter) to find
  message ids and the car state layout.
- Follow `0x00793000` (lobby update when `NetworkUseLobbies` is set).
- Match the `MIC` payload of [lan-client.md](lan-client.md) against the bit buffer layout.
