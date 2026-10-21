# Provenance: race barriers

Addresses, Ghidra names and method for [race-barriers.md](../specs/race-barriers.md). Ghidra auto names (`FUN_*`,
`DAT_*`) are used here only. Image base `0x00400000`; program `/speed.exe` (Black Edition PC v1.3). The barrier
routines already listed in [career-regions.md](career-regions.md) (`FUN_0064b410`, `FUN_0072c5c0`, `FUN_0072b9c0`,
`FUN_0072ba80`, `FUN_00659660`, `FUN_007889d0`, the collision filter) were decompiled again for this spec.

## Method

1. Read-only Ghidra MCP queries: decompile and memory reads. Nothing was renamed, created or saved.
2. Functions that Ghidra has not defined were found by scanning the PE file for the immediate `0x34108` and for the
   address of the override table (`64 3A 9A 00`), then read with `read_memory` and decoded by hand.
3. Data was checked by the repo's real-install tests (listed in the spec) and by throwaway Python reading
   `L2RA.BUN` chunk `0x34108`, `0x34109` and `0x3414D` (outside the repo).
4. Visual check: offscreen `nfsmw view-world --drive ... --screenshot` runs (no window, no input).

## Functions

| Role | Ghidra name | Address | Notes |
|---|---|---|---|
| Push one override onto its instance | `FUN_00728850` | `0x00728850` | `this` = 6-byte record: instance at `section_table[rec+0]->+4->+0x20 + rec+2 * 0x40`; if `inst+0x18 & 0x800000` and `(rec+4 ^ inst+0x18) & 0x400`: `FUN_006beeb0` (instance matrix out), identity via `FUN_004450c0` with `[0] = -1.0`, `FUN_0045ecf0(m, m, s)` = `D3DXMatrixMultiply(m, s, m)` through `FUN_006bd900`, translation restored from `inst+0x20..0x28`, written back by `FUN_00722c40` (nine packed i16 at `+0x2C..+0x3C`, position at `+0x20`); then `inst+0x18 = (inst+0x18 & 0xFFFF0000) + rec+4` |
| Switch a group's overrides on | `FUN_0072b9c0` | `0x0072B9C0` | `rec+4 = rec+4 & 0xFBEF \| (star ? 0x400 : 0)`, then `FUN_00728850` when the section is resident (`DAT_009a3bb0[section]` and its `+4`) |
| Switch a group's overrides off | `FUN_0072ba30` | `0x0072BA30` | `byte rec+4 \|= 0x10`, then `FUN_00728850`; bit `0x400` is kept |
| Enable a group | `FUN_0072c5c0` | `0x0072C5C0` | list at `DAT_009ab24c`, compares node `+8` with the hash; state `0x00902160[node +0xC] = 1 \| 2 (star) \| 4 (node +0x11)` |
| Re-enable the door | `FUN_0064b460` | `0x0064B460` | hash of `"SCENERY_GROUP_DOOR"` (`FUN_00460bf0`), group lookup `FUN_00728c60`, prefix test `FUN_00724ff0`, 256-slot list `FUN_00725060(&DAT_009a3b50, hash)`, `FUN_0072c5c0(hash, 0)`, `FUN_0072ba80(&DAT_009aa540, name)`; called at the end of `FUN_00659660` |
| Load the override records | undefined | `0x007311F0` | `cmp [chunk], 0x34108`; records `= chunk + 8` into `DAT_009a3a64`; count `= size / 6` (multiply by `0xAAAAAAAB`, shift 2) into `DAT_009a3a68`; then `FUN_00723bf0` |
| Bucket the override records | `FUN_00723bf0` | `0x00723BF0` | 256 u16 start indexes at `DAT_009a3740`, by the record's first byte (the low byte of the section number) |
| Unload | `FUN_007307e0` | `0x007307E0` | `0x34108`: zeroes `DAT_009a3a64` / `DAT_009a3a68`, calls `FUN_00723bf0` |
| Look up an override's info | undefined | `0x007287F0` | index check against `DAT_009a3a68`; record `DAT_009a3a64 + 6 * i`; section `i16 +0`, instance `i16 +2`, info index `inst + 0x3E`, info array at section header `+0x18`, stride `0x48`; returns info `+0x44` |
| Group list load | `FUN_00728c00` | `0x00728C00` | chunk `0x34109`: records linked into `DAT_009ab24c`; size `(2 * count + 0x17) & ~3` with count `i16 +0xE` |
| Road flag recompute | `FUN_007889d0` | `0x007889D0` | clears `+0xB & 0xCF` on all `DAT_009b3874` segments (stride `0x16`); exemption: race status `DAT_0091e000 +0x1968` set: `FUN_005fcb20` start count, `FUN_005fb3b0(i)` start (position `+0x28`, direction `+0x34`), segment `FUN_00785a00(pos, dir, 1.0, ...)`, road id segment `+6`; else the constants `-2511.0` (`0xC51CF000`), `147.8` (`0x4313CCCD`), `1783.0` (`0x44DEE000`), direction `(-0.7, 0, 0.7)`; per enabled line (`DAT_009aa9c4` count, `DAT_009aa9c8` array, `+0x10` enabled): physics points `(-y, 0, x)`, grid walk `FUN_0077ae80(DAT_009b38a8, ...)`, per segment `FUN_0076d7a0`; skip exempt roads; set `+0xB \|= 0x10` when line `+0x12 == 0`, else `0x20` |
| Segment against a line | `FUN_0076d7a0` | `0x0076D7A0` | curve of the segment `FUN_00765770`; piece count from `FUN_007c4b80` (float to int, its input not followed), raised to 4 when below 5; samples `t = i / n` for `i = 0..=n` via `FUN_005d0c70`, 2D point `(z, -x)`; `FUN_00722980(line a, line b, p_i, p_i-1)` tests the piece |
| Scripted group toggle | `FUN_0062ef00` | `0x0062EF00` | enables (`FUN_0064b410`) or disables (`FUN_0072c620`) a named group; constructors `FUN_0062efd0`, `FUN_0062f030`; not traced |

## Data facts

- `0x34108`: 22,340 records `{i16 section, i16 instance, u16 flags}` in `L2RA.BUN` at payload offset `299192` (134,040
  bytes). The section is the drivable section's number: 4,548 of the hooked instances lie in that section, 17,792 in
  its far tile (number + 40). Flags: `0x8000` 11,026; `0x8070` 6,957; `0x8001` 3,765; the rest small. Every record's
  flags equal the low 16 bits of its instance's flag word.
- `0x34109`: byte `+0x10` is 1 for 751 groups; `+0x11` (drive-through) is 1 for 126 groups, all of them the
  `XO_TrackBarrierTraffic_*` groups. Seven group names are not of the `BARRIER_SPLINE_n` / `BARRIER_LOCKED_n` /
  `SCENERY_GROUP_DOOR` forms (hashes `0x454C22D2`, `0x7D80D970`, `0x7D80D971`, `0x7D80D974`, `0x7D80D976`,
  `0x7D80D978`, `0x046E21DC`); no race lists them. Simple candidates (`PLAYER_BARRIERS_n`, `BARRIERS_n`,
  `FREE_ROAM_n`, ...) did not match.
- `0x3414D` is a top-level chunk of `L2RA.BUN` (payload offset `1358024`), not a child of the track path manager.

## Dead ends

- Ghidra's cross-reference tools were not available in this session, so callers of the override loader and of
  `FUN_00728850` at section load were not listed; the byte scan found the loader and the bucket builder only.
- The input of the float-to-int call in `FUN_0076d7a0` sits on the x87 stack and was not followed.
