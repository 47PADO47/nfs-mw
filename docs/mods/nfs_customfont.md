# NFS_CustomFont

NFS_CustomFont is a DLL mod by xan1242 for the Black Box engine Need for Speed titles. It loads replacement font files from a folder into the game's FEng and Apt font system and scales text by font name and screen height. It patches the game's font functions with call-site hooks, so it is a binary-patching mod with addresses tied to each executable build. The repository is MIT licensed, although the README does not state a licence. For nfs-mw this is a reference for behaviour only: we would specify the font-loading and scaling behaviour ourselves rather than port the code.

## Metadata

| Field | Value |
|---|---|
| Repository | https://github.com/xan1242/NFS_CustomFont |
| Author | xan1242 (per repository owner) |
| Type | Game mod, DLL loaded into the game process (C++, Visual Studio project). Not an ASI loader plugin in the code we could see; see section 3. |
| Licence | MIT, per the LICENSE file in the repository (Copyright (c) 2022 Lovro Pleše). The README has no licence statement. |
| Language | C++ (`dllmain.cpp`, `NFS_CustomFont.h`), plus a `Tools/` folder with a `mINI` submodule. GitHub language breakdown was not visible. |
| Target game builds | Most Wanted (MW, `GAME_MW`), Underground (UG), Underground 2 (UG2), Carbon, Pro Street (PS), Undercover (UC). README lists the exact executable versions (section 2). World and Hot Pursuit 2 are excluded by the README. |
| Latest visible activity | 10 December 2023 (three commits: "cleanup", "switch all path operations to std::filesystem", "restore win32 def"). 29 commits in total. No releases visible. |
| Reviewed | Read-only, via WebFetch on the README, repository page, LICENSE, source and config files. Nothing downloaded or run. |

## 1. Summary

NFS_CustomFont lets a player replace the game's fonts with custom font files placed in a folder, and set a per-font scale. Each font is chosen by its file name. The scale is picked from one of three tiers (Low, Mid, High) based on the game's current render height, so a player can use larger glyphs at higher resolutions. The mod hooks the font render, measure and find functions of the FEng UI system and, for some games, the Apt font path. It also intercepts the stock font chunk loader so that a custom font replaces the stock one with the same name.

## 2. What it does

Each feature is listed with its evidence. [README] means the README states it. [source] means I read it in the code.

- Loads custom font files from a folder. Files are discovered by name in `CustomFonts/High`, `CustomFonts/Mid` and `CustomFonts/Low`. The font name is the file name without its extension. [README: "Automatically loads font files from a folder"; source: `GetDirectoryListing()` in `dllmain.cpp`]
- Per-font scale. A font can be scaled by writing a scale for its name in a settings file. Unlisted fonts use scale 1.0. [README: example `FONT_NFS_BODY = 0.208334`; source: `[FontScaleLow]`, `[FontScaleMid]`, `[FontScaleHigh]` sections read by `LoadGlobalAChunks_Hook()`, default 1.0f in `GetFontScalarByHash()`]
- Resolution tiers. Fonts are picked from High, Mid or Low based on render height compared with two thresholds (defaults 960 and 1200 pixels). [source: `ThresholdMid`, `ThresholdHigh`, `FontScaleMode` in `NFS_CustomFont.h`; config file comments]
- Stock font override. A custom font with the same name as a stock chunk replaces the stock load and unload. [source: `LoaderFEngFont_Hook` and `UnloaderFEngFont_Hook`]
- Text measurement and layout scaling. Line width, text height, character width and word-wrap width are multiplied by the font scale, so layout follows the scaled font. [source: `FEngFont_GetLineWidth_Hook`, `FEngFont_GetTextHeight_Hook`, `FEngFont_GetCharacterWidth_Hook`, `FEngFont_GetNextWordWidth`-related addresses]
- Rendering scale. Rendered text, glyph positions and polygon vertices are scaled. [source: `FEngFont_RenderString_Hook`, `FERenderObject_AddPoly_Hook`, `FEngFont_PrintCharacter_Hook`]
- Stock HD font packs. The README offers an HD font pack that overwrites stock font files. It covers only Western European glyphs (codepoints 32 to 255). [README]
- Font building tools. A `Tools/` folder is included for building fonts with other glyph ranges, such as Cyrillic. The README says so. I did not read the tools. [README]

Known limitations (from the source header comments and README):

- MW: word wrapping is broken in some cases. [source: header comment in `NFS_CustomFont.h`]
- Carbon: FMV subtitles break when the font texture page is large. [source]
- Pro Street: reported as working but needs testing. [source]
- Undercover: works only on the FEng HUD. [source; README says the same]
- Underground and Underground 2: reported as fully working, with FMV subtitles still to check. [source]
- XNFSModFiles may conflict with this mod. The README says the fix is left to XNFSModFiles. [README]
- Must be disabled alongside HDFontSupport when used with NFSHDContentSupport. [README]

## 3. How it does it

Claims are tagged [source] (verified in code I read), [README] (stated in the README only), or [inferred].

Loading and entry point:

- The DLL uses `DllMain` on `DLL_PROCESS_ATTACH`. It calls `InitConfig()`, then `FixWorkingDirectory()` (sets the working directory to the executable's folder), then `Init()`. [source: `dllmain.cpp`]
- The DLL is loaded into the game process by the standard means for a DLL mod. The source does not name an ASI loader, so the loader used is not verified. [source for DllMain; loader inferred]
- Hooks are installed with `injector::MakeCALL(addr, hook, true)` from the `includes\injector` headers. These are call-site patches that redirect existing `call` instructions to hook functions. [source]
- Direct3D 9 is referenced through `<d3d9.h>` and `GAME_D3D9_DEVICE`, which is the macro `*(LPDIRECT3DDEVICE9*)0x00982BDC`. It is used to read the viewport height. [source]

Config reads:

- `InitConfig()` reads `NFS_CustomFont.ini` relative to the executable, before the working directory changes. It reads the `[MAIN]` section keys `FontFolder`, `ThresholdMid` and `ThresholdHigh`. [source]
- `LoadGlobalAChunks_Hook()` reads `scripts\NFS_CustomFont.ini`, a different path from the one above. It reads `[FontScaleHigh]`, `[FontScaleMid]` and `[FontScaleLow]`. Each key is a font name, hashed with `bStringHash`, and its value is parsed with `stof`. [source]

Font discovery and tiers:

1. `GetDirectoryListing()` scans `FontFolder/High`, `FontFolder/Mid` and `FontFolder/Low`. Each regular file becomes a `FontInfo` with name = file stem, hash = `bStringHash(stem)`, scale = 1.0f. Filesystem errors are caught and ignored. [source]
2. The loop runs from `FontScaleMode` down to 0, applying scales from the matching `FontScale*` section. [source]
3. A combined `FontList` is built in priority order High, Mid, Low. Duplicate hashes are skipped. The switch falls through on purpose. [source]
4. Each file is loaded with `LoadResourceFile()`, which calls `CreateResourceFile` and `ResourceFile_BeginLoading`. On UC an extra argument is read from `FECATEGORY_ADDR`. [source]
5. `ServiceResourceLoading()` runs, then the original `LoadGlobalAChunks` is called through a saved pointer in `hb_LoadGlobalAChunks.fun`. [source]

Stock chunk override:

- `LoaderFEngFont_Hook` and `UnloaderFEngFont_Hook` replace function pointers in the vtables at `LOADERFENGFONT_VTABLE_ADDR` (0x008F7DD8) and `UNLOADERFENGFONT_VTABLE_ADDR` (0x008F7E40). The originals are saved from `LOADERFENGFONT_ADDR` (0x00591EB0) and `UNLOADERFENGFONT_ADDR` (0x0059AC10). [source]
- If the chunk magic at `bChunk` is 0x030201, the font name is read from `bChunk + 8`. If the name matches a custom font and stock data follows the name, the hook returns 1 and skips the stock load or unload. [source]

Scaling:

- `GetFontScalarByHash()` returns the scale from `FontList`, or 1.0f when none is found. [source]
- Width and height hooks multiply the original results by the scale. Render and character hooks scale `maxWidth` or vertex positions. [source]
- `FindFont_Hook` copies the 0x38-byte font object and multiplies the height at `FENGFONT_HEIGHT_OFFSET` (0x24). `FEngFont_GetFontHash` reads the hash at `FENGFONT_HASH_OFFSET` (0x18). [source]

Address tables:

- Each game has its own `*_Address.h` file (`MW_Address.h`, `UG_Address.h`, `UG2_Address.h`, `Carbon_Address.h`, `ProStreet_Address.h`, `Undercover_Address.h`). The selected game is chosen by a compile-time macro (`GAME_MW`, `GAME_UG`, and so on). [source]
- Examples from `MW_Address.h` [source]:
  - `CREATERESOURCEFILE_ADDR` 0x0065FD30
  - `RESFILE_BEGINLOADING_ADDR` 0x006616F0
  - `SERVICERESOURCELOADING_ADDR` 0x006626B0
  - `FENGFONT_RENDERSTRING_ADDR` 0x0059AC60
  - `FENGFONT_PRINTCHARACTER_ADDR` 0x00585D60
  - `FENGFONT_GETLINEWIDTH_ADDR` 0x00586130
  - `FENGFONT_GETTEXTHEIGHT_ADDR` 0x00591FD0
  - `FINDFONT_ADDR` 0x00585B50
  - `LOADGLOBALACHUNKS_CALL_ADDR` 0x006660B6
- The MW hook sites are many: `GETLINEWIDTH_HOOK_ADDR_1` to `_7`, `GETTEXTHEIGHT_HOOK_ADDR_1` to `_6`, `FINDFONT_HOOK_ADDR_1` and `_2`, `GETCHARACTERWIDTH_HOOK_ADDR_1` and `_2`, plus `RENDERSTR_HOOK_ADDR` (0x005A45A1), `PRINTCHAR_HOOK_ADDR` (0x0059B01D) and `ADDPOLY_HOOK_ADDR` (0x00585E76). [source]
- Other games use different hook sets (UG, UG2, Carbon, PS, UC), with more or fewer sites per game. [source]

Other code:

- `bFileExists` tests a file with `fopen` in `"rb"` mode. [source]
- `bStringHash` is a case-insensitive hash that uses `toupper`, a multiplier of 0x21, and a start value of -1. It returns 0 for a null pointer. [source]
- `LoadResourceFile` is declared with `__stdcall` only when `GAME_UC` is not defined. No definition appears in the header. [source]

## 4. Configuration

`NFS_CustomFont.ini` (shipped in the repository, read from the executable folder for `[MAIN]`; the scale sections are read from `scripts\NFS_CustomFont.ini`):

| Section | Key | Default | Meaning |
|---|---|---|---|
| [MAIN] | FontFolder | CustomFonts | Folder scanned for font files, relative to the executable. |
| [MAIN] | ThresholdMid | 960 | Render height (pixels) at or above which Mid fonts are used. |
| [MAIN] | ThresholdHigh | 1200 | Render height (pixels) at or above which High fonts are used. |
| [FontScaleLow] | `<font name>` | 1.0 (not listed) | Scale for a Low-tier font. |
| [FontScaleMid] | `<font name>` | 1.0 (not listed) | Scale for a Mid-tier font. |
| [FontScaleHigh] | `<font name>` | 1.0 (not listed) | Scale for a High-tier font. |

Notes:

- The README example is `FONT_NFS_BODY = 0.208334`, written as a line in a settings file. The source reads scales from named sections, as above. These two descriptions do not match. The source is what the code does. This discrepancy is noted in section 6.
- The shipped ini sections are empty, so every font uses scale 1.0 until the player adds entries.
- The `FontScaleMode` global is not a key. It is derived from the thresholds and the render height. [source]

## 5. How we could implement it in nfs-mw

Licence and process. The repository is MIT, so reuse is permitted by its licence. Our process (`docs/licensing.md`, spec-first) still applies, and I did not read that file, so I have not verified its exact rule. The recommended approach is to write a spec from the behaviour below and implement it clean-room. The addresses and code in this repository should not be copied into the rewrite; they describe one executable build and are not part of a spec.

Relevant behaviours to specify:

1. Font discovery: scan a configured folder for tier subfolders (High, Mid, Low). Name each font by file stem. Apply priority High over Mid over Low, with the first name winning.
2. Tier selection: choose a tier from the render height using two thresholds (defaults 960 and 1200 pixels). Define the boundary rule (at or above, or strictly above) explicitly. The source uses "exceeds".
3. Per-font scale: a scale table keyed by font name per tier, defaulting to 1.0. Define what happens when a scale is set for a font that is not loaded.
4. Override rule: a custom font with the same name as a stock font replaces the stock load and unload. Define how the stock chunk is identified (name at a fixed offset after a chunk magic), without copying the magic or offset from this repository.
5. Measurement and layout: line width, text height, character width and wrap width scale with the font scale. Define rounding and the order of scale and layout.
6. Failure modes: missing folder, unreadable file, unknown font name, duplicate names across tiers, and a font file that fails to load (fall back to stock).

Data and formats needed:

- Font file format: the stock or HD font files of the game, as read by our existing format crates. Format details belong in `docs/formats/`, not here. Take `&[u8]` in format crates and never open files there.
- Config format: our own. A small key/value file with a sectioned layout is enough. Do not reuse the exact file names or sections unless we want compatibility with existing user setups.

Natural home:

- Engine-generic parts (folder scan, tier selection from a resolution, scale table, name hashing, and the override decision given a name) go in `libs/`. They must stay free of NFS:MW names and paths, following the `libs/` rules in `AGENTS.md`.
- NFS:MW-specific parts (which game functions are font-related, the stock font chunk layout for MW, and the MW screen-height source) go in `crates/`. `crates/` depends on `libs/`, never the reverse.
- Any renderer hooks belong to the renderer backend, not to the font logic. See ADR 0004 (swappable renderers) in `docs/decisions/0004-swappable-renderers.md`.

Risks:

- Hooking the game binary's font functions depends on exact addresses. A clean-room rewrite does not patch the binary, so these addresses are irrelevant to us, but any behaviour we copy from the repository's address-based approach would be tied to one build.
- The override needs the game's font data layout. We must derive it from our own format work, not from this repository.
- Word wrap and subtitle breakage are known bugs in this mod. Our layout code should test wrap width and FMV subtitle placement against real data before we rely on scaling.
- Font glyph coverage. The HD packs cover only Western European glyphs. Our design should say what happens for missing glyphs, instead of relying on a separate tool.
- Mod coexistence (the README warns about XNFSModFiles). Our loader should state its conflict rules.

Does not apply to a clean-room rewrite:

- The DLL, `DllMain`, `injector::MakeCALL` call patching, vtable replacement and the hardcoded address tables are all loader-specific hacks for patching a retail binary. They do not apply to our rewrite, which reads game data directly. We do not need an ASI loader or call-site hooks to get equivalent behaviour.
- The NoCD executable MD5 list in the README is specific to patched binaries. It does not apply to us.

## 6. Open questions and what could not be verified

- Licence conflict: the README has no licence statement, but the repository's LICENSE file is MIT (Copyright (c) 2022 Lovro Pleše). The licence section above uses the LICENSE file. I could not confirm that the README was read in full, because the fetch returned a summary and not verbatim text.
- Scale config mismatch: the README describes a `FONT_NFS_BODY = 0.208334` line in `NFS_CustomFont.txt`. The source reads named sections in `scripts\NFS_CustomFont.ini`. I could not tell which one the released version uses. The source is the more reliable reference.
- Config path mismatch: the source reads `[MAIN]` from `NFS_CustomFont.ini` in the executable folder, and scales from `scripts\NFS_CustomFont.ini`. These are two different paths. I have not confirmed whether the release package places them as the source implies.
- ASI loader: no ASI loader code appears in the parts I read. The loader used by the released build is not verified.
- Full source: I read `dllmain.cpp`, `NFS_CustomFont.h`, `MW_Address.h`, `NFS_CustomFont.ini` and `LICENSE`. I did not read `UG_Address.h`, `UG2_Address.h`, `Carbon_Address.h`, `ProStreet_Address.h`, `Undercover_Address.h`, the `Tools/` folder, or the project files. Hook sites for those games are described only from the summary of `dllmain.cpp`.
- Last-activity date: taken from the commits page via a summary fetch. It says 10 December 2023. I did not confirm it from the raw commit log.
- Release binaries: not downloaded. The README's NoCD and executable MD5 table is recorded from the README summary and was not checked against any binary.
- Behaviour of the mod in-game: not tested. Known issues are as stated by the source and README.

## 7. Sources

- https://raw.githubusercontent.com/xan1242/NFS_CustomFont/master/README.md (read through a summary fetch, not verbatim)
- https://github.com/xan1242/NFS_CustomFont (repository page: file list, licence line, commit count)
- https://raw.githubusercontent.com/xan1242/NFS_CustomFont/master/LICENSE
- https://raw.githubusercontent.com/xan1242/NFS_CustomFont/master/dllmain.cpp
- https://raw.githubusercontent.com/xan1242/NFS_CustomFont/master/NFS_CustomFont.h
- https://raw.githubusercontent.com/xan1242/NFS_CustomFont/master/MW_Address.h
- https://raw.githubusercontent.com/xan1242/NFS_CustomFont/master/NFS_CustomFont.ini
- https://github.com/xan1242/NFS_CustomFont/commits/master
