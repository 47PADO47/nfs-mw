# ICE camera chunk dispatch

**Scope:** which ICE chunk ids `speed.exe` dispatches, and the pack-kind mapping. Layout of the records
is NOT covered; see [formats/animation.md](../formats/animation.md#ice-chunk-dispatch-speedexe-13-verified-ghidra).

- **Restricted sources read:** `speed.exe` 1.3 disassembly (Ghidra, image base `0x400000`), read only.
- **Not read:** the decompilation (`dbalatoni13/nfsmw`). The chunk names are from `tools/bchunk_names.py`.
- **Implemented:** none. The IDs are in `libs/blackbox-chunk/src/ids/animation.rs` (commit 3fa9ca77).
- **Checked against the game by:** Ghidra immediate searches (`0x0003B211` and the `0x8003B200..0x8003B203`
  range) at `0x4788E6`/`0x47FB46`. Not checked: `0x8003B204`, `0x8003B209`, `0x0003B200`, `0x0003B210`.
- **Known gaps:** record layouts, the `0x84`-byte camera record and the shake record are unknown.
  `0x0047FB6F` (the second site's shake branch) is not read.
