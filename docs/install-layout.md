# Install layout: `D:\Need For Speed Most Wanted Black Edition`

Results of scanning this install with `tools/chunkdump.py`. The install is **modded**, so some files
differ from retail; see the end of this page. For the tag meanings, see
[evidence tags](README.md#evidence-tags).

## Executable

| File | Notes |
|---|---|
| `speed.exe` (6 MB) | The game. PE32, linker 7.10 (**Visual C++ .NET 2003**), Direct3D 9, statically linked C runtime. See [research.md](research.md). |
| `dinput8.dll` | ASI loader (loads `scripts/*.asi` mods) |
| `scripts/*.asi` | Mods: NFSMWUnlimiter, WidescreenFix, ExtraOptions, CustomHUD, XNFSMusicPlayer, SpeedReflect |
| `nfsmwpatch1.3.exe` | Official 1.3 patch installer |
| `bass*.dll`, `server.dll` | Music player mod / online-server mod libraries |

## Data folders

| Folder | Contents | Format |
|---|---|---|
| `CARS/<CAR>/` (102 folders) | `GEOMETRY.BIN` (models), `TEXTURES.BIN`, `VINYLS.BIN`, `PREVINYL.BIN` (texture packs) | bChunk: [models](formats/models.md), [textures](formats/textures.md) |
| `TRACKS/` | `L2RA.BUN` (world metadata), `STREAML2RA.BUN` (533 MB streamed world), `L2RA/` (minimap tiles) | bChunk: [maps](formats/maps.md) |
| `NIS/` (87 files) | `Scene_*_BundleB.bun`: cutscenes with textures, skinned models, skeletons, animations, scripts | bChunk: [animation](formats/animation.md) |
| `GLOBAL/` | `GLOBALA.BUN` (boot textures/screens), `GLOBALB.BUN` / `GlobalB.lzc` (light materials, car type info, parts DB, career data, presets), `InGameA/B.bun` (in-game HUD, shared models, ICE cameras, shared animations), HUD texture packs, attribute databases | bChunk + [VPAK](formats/attributes.md) |
| `FRONTEND/` | `FrontB.lzc` (menus: FEng screens, fonts, showroom models), `PLATFORMS/` | bChunk |
| `LANGUAGES/` | `English.bin` etc.: `STRBlocks` (all game text) + `FEngFont` | bChunk |
| `MOVIES/` | `*.vp6` FMV cutscenes | EA VP6 video |
| `SOUND/` | Engine, speech, music streams, reverb data | EA audio (not covered) |
| `SUBTITLES/`, `CREDITS/` | Text | |

## Numbers from the full scan **[verified]**

- 809 data files with `.bun`/`.bin`/`.lzc`/`.bacc` extensions:
  - 688 are bChunk streams (514,808 chunks, including chunks inside compressed geometry blocks), with
    **0 parse failures**;
  - 86 are empty;
  - 4 are VPAK databases;
  - the rest are non-chunk files (sound reverb data in `SOUND/NISREVDATA`, `GLOBAL/GlobalMemoryFile.bin`).
- Only 2 files in the scan are JDLZ-compressed (`GLOBALB.LZC.bacc`, `FRONTB.LZC.bacc`, the original
  `.lzc` files). The live `.lzc` files were already decompressed by mod tools.

## Mod changes noticed in this install

| What | Evidence |
|---|---|
| `.bacc` / `.bak` files | Backups written by mod tools (Binary / Unlimiter / installers); the `.bacc` files are the original compressed versions |
| `GLOBAL/GlobalB.lzc`, `FRONTEND/FrontB.lzc` | Stored uncompressed; the originals in `*.LZC.bacc` are JDLZ |
| `GLOBAL/gameplay.lzc` | `RAWW` wrapper (uncompressed); `gameplay.lzc.bak` is the original JDLZ |
| Add-on cars: COROLLAE88, FXXEVO, LEVIN, SF90, SKYLINEZT, TRUENO, TRUENOCP, TRUENOID | `GEOMETRY.BIN` header says *NFS-CarToolkit by nfsu360*; solids stored as bare JDLZ blobs |
| `CARS/FXXEVO/VINYLS.BIN` | **Corrupt / truncated**: the chunk at 0xAFA0 claims 12,288 bytes but the file ends at 0xB219 |
| `GLOBAL/HUDS_Custom_*.bin`, `scripts/CustomHUD` | CustomHUD mod texture packs |
