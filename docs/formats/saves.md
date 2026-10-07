# Save games and configuration

For the tag meanings, see [evidence tags](../README.md#evidence-tags).

## Location **[verified]**

`%USERPROFILE%\Documents\NFS Most Wanted\<profile>\<profile>`: one folder per profile holding one
extensionless file with the same name. This machine has two, `NAME\NAME` and `mwo\mwo`, both
**63,596 bytes**. The install's `MEMCARD/*.loc` files are **not** saves ([text.md](text.md)).

## File layout **[verified on both saves]**

| Offset | Type | Field | Value |
|---|---|---|---|
| 0x00 | char[4] | magic | `20CM` |
| 0x04 | u32 | file size | 63,596 |
| 0x08 | u32 | ? | 8 |
| 0x0C | u32 | profile buffer size | 0xF848 (63,560) |
| 0x10 | u8[16] | ? (not an MD5 of any obvious range; unknown) | |
| 0x20 | u32 | profile buffer size again | 0xF848 |
| 0x24 | | **profile buffer** (`UserProfile::SaveToBuffer`), 0xF848 bytes, ending at EOF | |
| 0x24 | char[16] | version: all zero | zeros |
| 0x34 … EOF−16 | | profile data | |
| EOF−16 | u8[16] | **MD5 of bytes [0x34, EOF−16)** | |

The checksum is plain MD5 (no key or salt). It matches on both saves **[verified]**. The decomp shows
why: `WriteProfileHash(buffer + 0x10, …, size − 0x20)` hashes the profile buffer after its 16-byte
version field and stores the digest at the end of the buffer (`Frontend/Database/FEDatabase.cpp`)
**[decomp]**. The 0x24 bytes before the buffer belong to the PC memory-card layer and are only
partly understood **[unconfirmed]**.

## Profile buffer order (`UserProfile::SaveToBuffer`) **[decomp]**

1. `char version[16]` (zeroed)
2. Career settings (`TheCareerSettings.SaveToBuffer`)
3. u32 `iDefaultStableHash`; the load fails if it doesn't match the game's
4. Profile name (`m_aProfileName`)
5. Music playlist (`Playlist`)
6. Options (`TheOptionsSettings`)
7. Car stable (`PlayersCarStable.SaveToBuffer`)
8. `CareerModeHasBeenCompletedAtLeastOnce`
9. High scores
10. 11 × u32 selected quick-race car per race type
11. MD5 (16 bytes)

The struct layouts are in the decomp's `Frontend/Database/` headers.

Known absolute offsets in the PC file, from x07x08's editor **[community]**:

| Offset | Field |
|---|---|
| 0x4039 | money (u32) |
| 0x5A31 | name (8 chars) |
| 0xE2ED | cars (array) |
| 0xE865 | pursuit bounty |
| 0xE86D… | infraction counters (u16) |
| 0xF2BA | pursuit records |

## Configuration

- **Registry** **[verified strings]**: `speed.exe` contains `Software\EA Games\Need for Speed Most Wanted`
  (settings and install path) and `Software\Electronic Arts\EA Games\Need for Speed Most Wanted\ergc`
  (CD key). Neither key exists on this machine (repack install).
- **Option names**: the `g_*` graphics settings are listed in [shaders.md](shaders.md#graphics-options-verified-strings).
- **Gameplay options** (`TheOptionsSettings`) are stored **inside the save** (step 6 above).
- `MWO.LauncherSettings.json` in the install root belongs to an online-server mod launcher, not the
  game.

## Tools

| Tool | What | License |
|---|---|---|
| [x07x08/nfsmw-save-editor](https://github.com/x07x08/nfsmw-save-editor) | Web (JS) editor: money, name, bounty, infractions, cars; recomputes the MD5 over [0x34, size−16) | Unlicense |
| [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) `src/Speed/Indep/Src/Frontend/Database/FEDatabase.cpp`, `Misc/MD5.cpp`, `Frontend/MemoryCard/` | Serialization order, checksum | CC0-1.0 |
| [BilawalAhmed0900/NFSMWSaveE](https://github.com/BilawalAhmed0900/NFSMWSaveE) | Python save editor (not reviewed) | no license listed |
| [TsyVM/MWEncyclopedia](https://github.com/TsyVM/MWEncyclopedia) C31 | **Wrong for PC saves**: it describes saves as `.loc`/`LOCH`. Leads only | none |
