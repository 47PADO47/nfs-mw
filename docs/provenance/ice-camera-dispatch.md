# ICE camera chunk dispatch

**Scope:** which ICE chunk ids `speed.exe` dispatches, and the pack-kind mapping. Layout of the records
is NOT covered; see [formats/animation.md](../formats/animation.md#ice-chunk-dispatch-speedexe-13-verified-ghidra).

- **Restricted sources read:** `speed.exe` 1.3 disassembly (Ghidra, image base `0x400000`), read only.
- **Not read:** the decompilation (`dbalatoni13/nfsmw`). The chunk names are from `tools/bchunk_names.py`.
- **Implemented:** the IDs are in `libs/blackbox-chunk/src/ids/animation.rs`. The reader for the
  record layout is `libs/blackbox-chunk/src/ice/` (camera packs, shake tracks).
- **Checked against the game by:** Ghidra immediate searches (`0x0003B211` and the `0x8003B200..0x8003B203`
  range) at `0x4788E6`/`0x47FB46`. Not checked: `0x8003B204`, `0x8003B209`, `0x0003B200`, `0x0003B210`.
- **Record layout:** decoded from the handler bodies at `0x004787C0`, `0x00478890` and `0x004786E0`, plus
  the consumers `0x0046E760`, `0x0046D2E0`, `0x00477EE0`, `0x004789C0` and the teardown `0x0047F9F0`.
  Offsets are in `docs/formats/animation.md` ("ICE camera records").
- **Known gaps:** position, target and FOV inside the `0x84`-byte camera element are not pinned.
  The 24-byte shake element is not decoded. `0x0047FB6F` (the second site's shake branch) is not read,
  and `0x0047FB40` is not a function start in the Ghidra database.
