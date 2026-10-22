# GLOBAL folder: databases and packs — Need for Speed: Most Wanted (2005)

`GLOBAL/` holds the packs and databases that every mode of the game shares: the boot screens, the
HUD, the car tables, the tuning and race data, and the texture packs. This page lists each file, what
it holds, and whether this repo reads it. The format details live in the linked pages.

Inspected install: `D:\Need For Speed Most Wanted Black Edition\GLOBAL` (Black Edition, PC v1.3).
The copies on the Desktop and in `D:\NFSTools\NFS Most Wanted Redux 2.3` differ; see
[install-layout.md](../install-layout.md#other-copies-on-this-machine).

> **Status:** "Read by code" means a crate or lib opens the file. It does not mean every record in it
> is used. Evidence comes from grepping `crates/` and `libs/` for each file name and from the magic
> bytes of each file. Contents marked **[unverified]** are the docs' description, not checked here.

## Summary

| Status | Files |
|---|---|
| Read by code | `GlobalB.lzc`, `attributes.bin`, `gameplay.bin`, `gameplay.lzc`, `InGameA.bun` (textures), `GLOBALA.BUN`, `InGameB.bun`, `INGAMEC.BUN`, `WIDESCREEN_GLOBAL.BUN`, `HUDS_Custom_*.bin` (names only), `HUDTEXRACE.BIN`, `HUDTEXTURESPHOTOFINISH.BIN` |
| Partly read | `InGameA.bun` (shared models now read, the sky draw path not pinned), `GLOBALA.BUN` (UI only), `FE_ATTRIB.bin` (decoded by `blackbox-attrib`, unused by the game code) |
| Not read | `InGameB.bun` ICE cameras and shared animations, `RIVALS.BIN`, `DYNTEX.BIN`, `CardTextures.bin`, `HUDTEXDRAG*.BIN`, `HUDTEXSPLIT.BIN`, `InGameSplitScreen.bun`, `THINSCREEN_GLOBAL.BUN`, `DEMO_GLOBAL.BUN`, `DEMO_MARKETING.BUN`, the `*MemoryFile.bin` files |
| Backups | `*.bacc`, `*.bak`: the original files before a mod tool rewrote them. No code reads them. |

## Files

### Boot, UI and fonts

| File | Size | Contents | Status |
|---|---|---|---|
| `GLOBALA.BUN` | 430 KB | Boot textures and screens, one font. bChunk. [frontend](frontend.md) | Read by code (UI assets and catalog) |
| `WIDESCREEN_GLOBAL.BUN` | 64 KB | Widescreen variant of the global FEng packages. FEng package chunk. [frontend](frontend.md) | Read by code (UI catalog) |
| `THINSCREEN_GLOBAL.BUN` | 56 KB | Thin-screen (4:3) variant of the same packages. [frontend](frontend.md) | Not read (only the FEng test names it) |
| `DEMO_GLOBAL.BUN` | 57 KB | Demo-build FEng package (header is the FEng package ID). | Not read |
| `DEMO_MARKETING.BUN` | 149 KB | Demo-build marketing package (same header form). | Not read |

### HUD and in-game screens

| File | Size | Contents | Status |
|---|---|---|---|
| `InGameB.bun` | 946 KB | HUD and in-game FEng screens, fonts (`font_mw_body`, `tac_numbers`), ICE camera packs, shared animations, sound stitches. [frontend](frontend.md), [animation](animation.md) | Read by code for HUD and fonts. ICE and animation parts not read |
| `INGAMEC.BUN` | 946 KB | Single-race HUD package and fonts (same chunks as `InGameB.bun`). [frontend](frontend.md) | Read by code (HUD plugin) |
| `InGameSplitScreen.bun` | 32 KB | Split-screen HUD packages. [frontend](frontend.md) | Not read |
| `InGameA.bun` | 5.9 MB | Sky models and textures (`SKY_MIDDAY_A_*`) and shared models. bChunk. | Textures and shared models read (`nfsmw-data/src/world/globals.rs`, `GLOBAL_MODEL_FILES`). The game's region loader opens `Global\InGameA.bun` in `FUN_00666AA0` (speed.exe, Ghidra). Which code resolves the `SKYDOME` solids from it is not pinned: see [provenance](../provenance/ingamea-sky-models.md) |
| `HUDTEXRACE.BIN` | 157 KB | Race HUD texture pack (TPK). [textures](textures.md) | Listed in UI assets |
| `HUDTEXDRAG.BIN` | 358 KB | Drag-race HUD texture pack. | Not read |
| `HUDTEXDRAGSPLIT.BIN` | 33 KB | Drag-race split-screen HUD textures. | Not read |
| `HUDTEXSPLIT.BIN` | 48 KB | Split-screen HUD textures. | Not read |
| `HUDTEXTURESPHOTOFINISH.BIN` | 996 KB | The win photo's backdrops: four 1024 x 512 DXT3 textures (`PHOTOFINISH_SPRINT`, `_RIVAL`, `_DRAG`, `_TOOBOOTH`), each a polaroid on gravel with a transparent window. HUFF/JDLZ entries. [race-finish-effects](../specs/race-finish-effects.md) | Listed in UI assets |
| `HUDS_Custom_00.bin` … `HUDS_Custom_10.bin` | 15–23 KB each | Tachometer skin textures (`n_LINES_NN`, `TACH_NEEDLE_NN`, `TACH_FILL_NN`), 11 skins. | Texture names in `crates/nfsmw/src/hud/skin.rs`; all available packs loaded for garage gauge styles |

### Car, tuning and race data

| File | Size | Contents | Status |
|---|---|---|---|
| `GlobalB.lzc` | 12.6 MB | Light materials, car types, parts DB, presets, car bounds, track and sun info. Same data as `GLOBALB.BUN` (JDLZ original in `GLOBALB.LZC.bacc`). [cardata](cardata.md) | Read by code (car tables, bounds, swaps, textures, trax) |
| `attributes.bin` | 669 KB | AttribSys `VPAK`: 57 class definitions and about 2,400 collections. Physics, cop light bars, exhaust effects. Must load before the other packs. [attributes](attributes.md) | Read by code (`nfsmw-data` physics, cop lights, exhaust, sound) |
| `gameplay.bin` | 2.1 MB | AttribSys `VPAK`, 272 vaults, 6,293 `gameplay` collections: race events, Blacklist race bins, challenges, milestones, script handlers. [gameplay-races](gameplay-races.md) | Read by code (`nfsmw-data/src/gameplay`) |
| `gameplay.lzc` | 2.1 MB | The same pack in a `RAWW` wrapper. [bchunk](bchunk.md) | Decoded by `libs/blackbox-attrib` tests only (`tests/install.rs`); the game code uses the `.bin` |
| `FE_ATTRIB.bin` | 111 KB | AttribSys `VPAK`, 446 `frontend` collections. [attributes](attributes.md) | Decoded by `libs/blackbox-attrib`; no game code uses it |
| `CardTextures.bin` | 19 KB | Car card textures (JDLZ entries). | Not read |

### Texture packs (not read)

| File | Size | Contents | Status |
|---|---|---|---|
| `RIVALS.BIN` | 21.6 MB | Texture pack, 41 HUFF blobs. [huff](huff.md) | Not read. `docs/specs/weather.md` mentions it for a `LIGHTNING` hash |
| `DYNTEX.BIN` | 10.0 MB | Dynamic texture pack, 14 HUFF blobs. **[unverified]** contents | Not read |

### Memory files

| File | Size | Contents | Status |
|---|---|---|---|
| `GlobalMemoryFile.bin` | 366 B | Text starting `// Build`. [install-layout](../install-layout.md) | Not read |
| `InGameMemoryFile.bin`, `PermanentMemoryFile.bin`, `FrontEndMemoryFile.bin` | 16 B each | Empty `MEMO` chunk, all zeros. | Not read |

### Backups

| File | Original of |
|---|---|
| `GLOBALB.LZC.bacc` | `GLOBALB.BUN`, JDLZ-compressed |
| `GLOBALA.BUN.bacc`, `INGAMEA.BUN.bacc` | `GLOBALA.BUN`, `InGameA.bun` |
| `attributes.bak`, `fe_attrib.bak`, `gameplay.bak`, `gameplay.lzc.bak` | The 2005 attribute packs |

## Related

- [Install layout](../install-layout.md): the `GLOBAL/` row and the other installs on this machine.
- [bChunk](bchunk.md): the container and the JDLZ, RAWW and HUFF wrappers.
- [Attributes](attributes.md), [cardata](cardata.md), [frontend](frontend.md), [textures](textures.md).
