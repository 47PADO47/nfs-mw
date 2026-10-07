# Frontend / UI (FEng packages, fonts, HUD)

Menus and the HUD are **FEng** ("FrontEnd engine") packages: screen descriptions compiled from
`.fng` source files and stored inside bChunks. A package is itself a small chunk tree with its own
four-character chunk IDs and two-character property tags. For the tag meanings, see
[evidence tags](../README.md#evidence-tags).

## Where it lives **[verified]**

| File | FEng content |
|---|---|
| `FRONTEND/FrontB.lzc` | 150 × `00030203 FEngPackage` (menus, e.g. `MainMenu.fng`, `Options.fng`, `SafeHouseRaceSheet.fng`, `Keyboard.fng`), 2 × `00030201 FEngFont`, menu textures (TPK), a showroom `GeometryPack` |
| `GLOBAL/InGameB.bun` | 1 × `FEngPackage` + 33 × `00030210 FEngCompressedPackage`: in-game screens and the HUD (`HUD_SingleRace.fng`, `HUD_Drag.fng`, `Pause_Main.fng`, `PostRace_Results.fng`, `InGameBusted.fng`, …), 1 × `FEngFont` |
| `GLOBAL/GLOBALA.BUN` / `GlobalB.lzc` | 2 / 9 packages; GLOBALA also has 1 font |
| `LANGUAGES/*.bin` | 6 × `FEngFont` after the string table (see [text.md](text.md)) |
| `FRONTEND/DEMO_FRONTEND.BUN` | starts with an `FEngCompressedPackage` |
| `GLOBAL/HUDTEX*.BIN`, `HUDTEXTURESPHOTOFINISH.BIN` | HUD textures (TPK, see [textures.md](textures.md)) |
| `FRONTEND/PLATFORMS/*.BIN` | Front-end 3D scenes: `CAREER_SAFEHOUSE`, `CAR_LOT`, `CUSTOMIZATION_SHOP(_BACKROOM)`, `NextGenSky` are `GeometryPack`s; `PlatformCrib` is a TPK |

`FRONTEND/FRONTA.BUN` is empty. `GLOBAL/HUDS_Custom_*.bin` belong to the CustomHUD mod.

## bChunk wrappers **[verified]**

| Chunk | Payload |
|---|---|
| `0x00030203` FEngPackage | the FEng package itself, starting `FEn\xE7` |
| `0x00030210` FEngCompressedPackage | u32 **bStringHash of the package name** (checked on 32 of 32) + a compressed package. 32 are **JDLZ** (decompress with the [bchunk.md](bchunk.md) algorithm); 1 in `InGameB.bun` is **`HUFF`**-compressed, which this repo cannot decode yet |
| `0x00030201` FEngFont | starts with names, e.g. `font_mw_title`, `bin\font_mw_title.bin`. Glyph layout: see below |

The decomp's chunk list names these `BCHUNK_FENG_PACKAGE`, `BCHUNK_FENG_COMPRESSED_PACKAGE` and
`BCHUNK_FENG_FONT` / `BCHUNK_FONTREAL_INFO` **[decomp]**.

## FEng package format **[decomp + verified]**

The structure is the same as bChunk: `u32 id, u32 size, payload`, and **bit 31 set = has children**.
IDs are four ASCII characters, so a nested ID has 0x80 added to its last character (`FEng` →
`FEn\xE7`, `ResL` → `Res\xCC`). The IDs below come from the decomp's `src/Speed/Indep/Src/FEng/FEPackageChunks.h`.

| ID | Nested | What |
|---|---|---|
| `FEng` | ✔ | the package |
| `PkHd` | | header (below) |
| `TLst` / `TypS` / `Tyno` | ✔ / / | type list / type sizes / type node |
| `LibL`, `LibR` | | referenced library packages |
| `ResL` → `RsNm`, `RsRq` | ✔ | resources: names, requests (textures, fonts, …) |
| `ObjL` → `Butn`, `FObj` → `ObjD` | ✔ | object list, button count, objects, object data |
| `Scrp` | | animation scripts (key tracks) |
| `MsgR`, `PkgR`, `Targ`, `MsgN` | | message responses, package responses, message targets, message names |
| `EDat` | | editor data |

In the 194 packages that decode (150 + 1 + 2 + 9 in plain chunks, 32 decompressed), the top level is
always `PkHd`, `TypS`, `ResL`, `ObjL`, `Targ`, plus `PkgR` in 181 **[verified]**.

**`PkHd`** (`FEPackageReader::ReadHeaderChunk`) **[decomp + verified]**:

| Offset | Field | Example (`RapSheetLogin.fng`) |
|---|---|---|
| 0x00 | u32 version; must be ≥ `0x20000` | `0x20000` in all 194 |
| 0x04 | u32 ? | 0 |
| 0x08 | u32 resource count | 11 |
| 0x0C | u32 object count | 10 |
| 0x10 | u32 short-name length (with NUL) | 18 |
| 0x14 | u32 file-name length (with NUL) | 47 |
| 0x18 | char[] short name, then file name | `RapSheetLogin.fng`, `Global\FEng_Screens\Stripped\RapSheetLogin.fng` |

Inside `ObjD`, `Scrp` and the message chunks, properties are **2-character tags** (`FE_TAG(a,b)`).
Examples: `Ot` type, `On` name, `Oh` name hash, `OP` properties, `PA` parent GUID, `SA` static animation
data, `St`/`SH`/`SL` string text / label hash / label name, `Sj` justification, `M1`–`M3` multi-image
textures, `Sn`/`Sh`/`SI`/`Sl` script name / header / ID / length, `Ti`/`Tt`/`Ta`/`Tb` track index /
interpolation type / action / base key. The full list is in `FEPackageChunks.h`; the reader is
`FEPackageReader.cpp` (36 KB) **[decomp]**. String objects refer to text by label hash, resolved through
`LANGUAGES/*.bin` ([text.md](text.md)).

## Fonts (`FEngFont`) **[decomp]**

`src/Speed/Indep/Src/Frontend/FEngFont.cpp` builds an `FEngFont` directly from the bChunk
(`FEngFont(bChunk*)`): character-width/kerning queries, line wrapping, and button-glyph textures
("JoyEvent" textures) inside strings. **No community tool parses the glyph data.** FEngLib's renderer
substitutes a system font. The on-disk layout is undocumented.

## Tools

| Tool | What | Language / license |
|---|---|---|
| [NFSTools/FEngLib](https://github.com/NFSTools/FEngLib) | Reads and writes FEng packages (all object types, scripts, message responses); `FEngViewer` / `FEngRender` render them; `FEngCli` compiles packages back into `0x30203` chunks | C#, **no LICENSE file** (all rights reserved by default) |
| [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) `src/Speed/Indep/Src/FEng/`, `Src/Frontend/` | Chunk/tag IDs, package reader, runtime, fonts, screens | C++, CC0-1.0 |
| [SpeedReflect/Nikki](https://github.com/SpeedReflect/Nikki) (`FNGroup`) | Edits colours inside FEng packages (Binary's "FEng Color Editor") | C#, MIT |
