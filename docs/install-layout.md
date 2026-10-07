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
| `GLOBAL/` | `GLOBALA.BUN` (boot textures/screens, a font), `GLOBALB.BUN` / `GlobalB.lzc` (light materials, car type info, parts DB, presets, car bounds, track/sun info: [cardata](formats/cardata.md)), `InGameA.bun` (sky models/textures, shared models), `InGameB.bun` (HUD and in-game FEng screens, ICE cameras, shared animations, sound stitches), HUD texture packs, attribute databases (`attributes.bin`, `gameplay.bin`/`.lzc`, `FE_ATTRIB.bin`), `*MemoryFile.bin` (a `MEMO` chunk; `GlobalMemoryFile.bin` holds text starting `// Build`) | bChunk + [VPAK](formats/attributes.md) |
| `FRONTEND/` | `FrontB.lzc` (150 FEng menu packages, 2 fonts, menu textures, showroom model), `PLATFORMS/` (front-end 3D scenes: safehouse, car lot, customization shop, sky), `FRONTA.BUN` (empty) | bChunk: [frontend](formats/frontend.md) |
| `LANGUAGES/` | `English.bin` (string table + 6 fonts), `Labels.bin` (label names), `Largest.bin`, `LanguageTextures.bin` (TPK), `agree.eng`/`.usa` (plain text) | bChunk: [text](formats/text.md) |
| `MOVIES/` | 32 × `<movie>_english_ntsc.vp6` FMVs, 1024×512 @ 29.97 fps, EA-XA stereo audio | EA VP6 container: [video](formats/video.md) |
| `SOUND/` | 15 folders: `.abk` banks (301), `.gin` engine loops (160), interactive music (`PFDATA/MW_Music.mpf` + 534 MB `.mus`), cop speech / NIS audio (`.big` + `.idx`/`.evt`/`.csi`), `EVT_SYS` (`.csi`), `FXEDIT` reverb presets, `MIXMAPS`, `NISREVDATA` | EA audio: [audio](formats/audio.md) |
| `SUBTITLES/` | 30 extensionless files, one per FMV: 8-byte `{start, label hash}` records | [text](formats/text.md#subtitles-decomp--verified) |
| `CREDITS/` | `NA_ENGLISH.TXT`, `UK_ENGLISH.TXT` (UTF-16LE) | text |
| `MEMCARD/` | `LOCALE_ENGLISH.loc`, `LOCALE_RUSSIAN.loc` (`LOCH`/`LOCI`; probably memory-card UI strings **[unconfirmed]**; **not** saves) | [text](formats/text.md) |

## Outside the install **[verified]**

| Location | What |
|---|---|
| `%USERPROFILE%\Documents\NFS Most Wanted\<profile>\<profile>` | Save game, one folder per profile (63,596 B each here): `20CM` header + profile + MD5. See [saves](formats/saves.md) |
| Registry `Software\EA Games\Need for Speed Most Wanted` | Settings/install path named in `speed.exe`; absent on this machine (repack) |

`speed.exe` also embeds the 31 compiled D3D9 effects as `RT_RCDATA` resources: see [shaders](formats/shaders.md).

## Numbers from the full scan **[verified]**

- 809 data files with `.bun`/`.bin`/`.lzc`/`.bacc` extensions:
  - 688 are bChunk streams (514,808 chunks, including chunks inside compressed geometry blocks), with
    **0 parse failures**;
  - 86 are empty;
  - 4 are VPAK databases;
  - the rest are non-chunk files (engine-rev curves in `SOUND/NISREVDATA`, see [audio](formats/audio.md#nisrevdatabin-decomp--verified); `GLOBAL/GlobalMemoryFile.bin`).
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
