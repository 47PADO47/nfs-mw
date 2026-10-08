# Frontend / UI (FEng packages, fonts, HUD)

Menus and the HUD are **FEng** ("FrontEnd engine") packages: screen descriptions compiled from
`.fng` source files and stored inside bChunks. A package is itself a small chunk tree with its own
four-character chunk IDs and two-character property tags. For the tag meanings, see
[evidence tags](../README.md#evidence-tags). How a package runs (scripts, messages, drawing) is in
[the FEng runtime spec](../specs/feng-runtime.md).

## Where it lives **[verified]**

| File | FEng content |
|---|---|
| `FRONTEND/FrontB.lzc` | 150 × `00030203 FEngPackage` (menus, e.g. `MainMenu.fng`, `Options.fng`, `SafeHouseRaceSheet.fng`, `Keyboard.fng`), 2 × `00030201 FEngFont`, menu textures (TPK), a showroom `GeometryPack` |
| `GLOBAL/InGameB.bun` | 1 × `FEngPackage` (`HUD_SingleRace.fng`) + 33 × `00030210 FEngCompressedPackage`: in-game screens and the HUD (`HUD_Drag.fng`, `Pause_Main.fng`, `PostRace_Results.fng`, `InGameBusted.fng`, …), 1 × `FEngFont` |
| `GLOBAL/INGAMEC.BUN` | the same single-race HUD package and the fonts |
| `GLOBAL/InGameSplitScreen.bun` | split-screen HUD packages |
| `GLOBAL/GLOBALA.BUN` / `GlobalB.lzc` | 2 / 9 packages; GLOBALA also has 1 font |
| `GLOBAL/WIDESCREEN_GLOBAL.BUN`, `THINSCREEN_GLOBAL.BUN` | wide and thin screen variants of the global packages |
| `LANGUAGES/*.bin` | 6 × `FEngFont` after the string table (see [text.md](text.md)) |
| `FRONTEND/DEMO_FRONTEND.BUN` | starts with an `FEngCompressedPackage` |
| `GLOBAL/HUDTEX*.BIN`, `HUDTEXTURESPHOTOFINISH.BIN` | HUD textures (TPK, see [textures.md](textures.md)) |
| `GLOBAL/HUDS_Custom_NN.bin` (00 to 10) | stock texture packs of the selectable tachometer and speedometer skins (`TACH_NEEDLE_00`, `TACH_FILL_00`, `7000_LINES_00` … `10000_LINES_00`, `TURBO_NEEDLE_00`, `TURBO_LINES_00`); the game loads `HUDS_Custom_%2.2d.bin` |
| `FRONTEND/PLATFORMS/*.BIN` | Front-end 3D scenes: `CAREER_SAFEHOUSE`, `CAR_LOT`, `CUSTOMIZATION_SHOP(_BACKROOM)`, `NextGenSky` are `GeometryPack`s; `PlatformCrib` is a TPK |

`FRONTEND/FRONTA.BUN` is empty. A census of 210 distinct packages across these files backs the layouts below.

## bChunk wrappers **[verified]**

| Chunk | Payload |
|---|---|
| `0x00030203` FEngPackage | the FEng package itself, starting `FEn\xE7` |
| `0x00030210` FEngCompressedPackage | u32 **bStringHash of the package name** + a compressed package: 33 of 34 are **JDLZ**, 1 is **`HUFF`** (`BLACK_BACKGROUND.fng`, 408 bytes, 1 object, in `InGameB.bun`). `ea_compress::unwrap` decodes both |
| `0x00030201` FEngFont | 0x100 bytes font name, 0x100 bytes texture name (the same text, e.g. `font_mw_body`), then the `FNTF` font data at +0x200 (below) |

The decomp's chunk list names these `BCHUNK_FENG_PACKAGE`, `BCHUNK_FENG_COMPRESSED_PACKAGE` and
`BCHUNK_FENG_FONT` / `BCHUNK_FONTREAL_INFO` **[decomp]**.

## FEng package format **[decomp + verified]**

Packages on the PC are **little-endian** (the decomp reads the console big-endian form). The structure is
the same as bChunk: `u32 id, u32 size, payload`, and **bit 31 set = has children**. IDs are four ASCII
characters, so a nested ID has 0x80 added to its last character (`FEng` → `FEn\xE7`, `ResL` → `Res\xCC`).
The IDs come from the decomp's `src/Speed/Indep/Src/FEng/FEPackageChunks.h`.

| ID | Nested | What |
|---|---|---|
| `FEng` | ✔ | the package |
| `PkHd` | | header (below) |
| `TypS` | | type sizes: pairs `{u32 type, u32 byte size}` (group 68, image 84, string 68, multi image 144; coloured image data is 148 bytes) |
| `ResL` → `RsNm`, `RsRq` | ✔ | resources: NUL-separated names; requests |
| `ObjL` → `Butn`, `FObj` → `ObjD`, `Scrp`, `MsgR` | ✔ | object list, button count, objects, object data, scripts, message responses |
| `PkgR` | | package-level message responses (189 of 210 packages) |
| `Targ` | | message targets: `Tc` count, then `Mt` = `{u32 message id, u32 target GUIDs…}` |
| `LibL`, `LibR`, `MsgN`, `EDat` | | library references, message names, editor data (not used by the HUD) |

The top level is always `PkHd`, `TypS`, `ResL`, `ObjL`, `Targ`, plus `PkgR` in 189 of 210.

**`PkHd`** (`FEPackageReader::ReadHeaderChunk`):

| Offset | Field | Example (`RapSheetLogin.fng`) |
|---|---|---|
| 0x00 | u32 version; must be ≥ `0x20000` | `0x20000` in all |
| 0x04 | u32 ? | 0 |
| 0x08 | u32 resource count | 11 |
| 0x0C | u32 object count | 10 |
| 0x10 | u32 short-name length (with NUL) | 18 |
| 0x14 | u32 file-name length (with NUL) | 47 |
| 0x18 | char[] short name, then file name | `RapSheetLogin.fng`, `Global\FEng_Screens\Stripped\RapSheetLogin.fng` |

Inside the object, script and message chunks, properties are **tags**: `u8 a, u8 b, u16 size`, then the
data, back to back.

### Objects **[verified]**

`ObjL` is **flat**: parents precede their children and the tag `PA` (u32 GUID) names the parent group.
Each `FObj` holds one `ObjD`, 0 to n `Scrp` and 0 or 1 `MsgR`. Object types present: 1 Image (10163),
2 String (4388), 5 Group (4256), 12 MultiImage (33), 7 Movie (17), 9 ColoredImage (16), 11 SimpleImage (1).
There are **no list types (4, 6)** and no `On`, `Sn`, `Tb`, `Ti`, `Tt`, `Ta` tags: object and script names
exist only as hashes.

`ObjD` tags: `Ot` type; `Oh` name hash; `OP` = 4 × u32 {GUID, name hash, flags, resource index}; `PA`
parent GUID; `SA` = the object's data block (size from `TypS`).

The `SA` block (little-endian):

| Offset | Field |
|---|---|
| 0x00 | i32 colour: blue, green, red, alpha (0..255 each) |
| 0x10 | f32 pivot[3] |
| 0x1C | f32 position[3] |
| 0x28 | f32 rotation quaternion x, y, z, w |
| 0x38 | f32 size[3] |
| 0x44 | images: UV upper-left (2 × f32), lower-right (2 × f32); block is 0x54 |
| 0x54 | coloured image: 4 corner colours; block is 0x94 |
| 0x44.. | multi image: 3 UV upper-left, 3 UV lower-right, pivot rotation[3]; block is 0x90 |

String objects add tags `Sb` buffer length, `St` UTF-16LE text, `Sj` justification, `Sl` leading,
`Sw` maximum width and `SH` label hash. Images have `If` flags (unused by the renderer). Multi images have
`M1`–`M3` texture hashes and `Ma`–`Mc` flags (0 or 1 in the HUD). Object flags seen: `0x40000000`
(affect all scripts) and 0; the low 16 bits are for the game: bit 1 "text is not localized", bit 3 "not drawn on
the PC" (`RenderObject` skips it) **[decomp]**. `OP`'s 4th word is the resource index (`0xFFFF` = none; groups carry 0).

**Justification** (`Sj`, `FEString.h` **[decomp]**): 1 horizontally centred, 2 right, 4 vertically
centred, 8 vertically bottom, 0x10 word wrap. Values seen in the HUD: 0, 1, 2, 5, 0x11 **[verified]**.

### Resources **[verified]**

`RsNm` is a block of NUL-separated names. `RsRq` is `u32 count` + count × 24 bytes
`{id, name offset into RsNm, type, flags, handle, user parameter}`. Types: 1 image, 2 font, 7 multi image.
The runtime handle of a resource is `bStringHash(UPPER(basename without extension))`
(`LoadResources` **[decomp]**): `Outrun_Backing.tga` → `OUTRUN_BACKING`, `FONT_MW_BODY.ffn` →
`FONT_MW_BODY`. Checked on `HUD_SingleRace.fng`: 60 of its 69 resources hash to a texture key in
`HUDTEXRACE.BIN`, `InGameA.bun` (`HUDTEXTURESGLOBAL`), `GlobalB.lzc` or `FrontB.lzc`. The font texture
`FONT_MW_BODY` is in the pack `HUDTEXTURESGLOBAL` of `InGameA.bun`. The rest are swapped in by the game at
run time (`TAC_FILL_00`, `RPM_Needle`, the `*_LINES` tachometer faces, `Turbo_*`), taken from
`HUDS_Custom_NN.bin`, or are the map tiles. A TPK texture's name is cut at 23 characters; compare the hash.

### Scripts **[decomp + verified]**

`Scrp` holds one animation script of an object:

- `Sh` (16 bytes): `{u32 script id hash, i32 length in ticks, u32 flags, i32 track count}`; the low two
  flag bits are the end behaviour: 0 once, 1 loop, 2 ping-pong. `Sc` (optional): the id of the script to
  chain to when this one ends.
- per track: `FI` = `{u8 parameter type, u8 size, u8 interpolation, u8 action, u32 track length}`;
  parameter types 1 int, 2 float, 3 vec2, 4 vec3, 5 quaternion, 6 colour; interpolation 0 none (step),
  1 linear, 3 move-to (2 and 4 are not implemented by the game); `To` = u32 offset, in u32 words, into the
  object data block (colour 0, pivot 4, position 7, rotation 0xA, size 0xE, UV upper-left 0x11, lower-right
  0x13, then 0x15, 0x19, 0x1D, 0x21); `Kd` = keys `{i32 time, value[size]}`.
- **The first key (time −1) is the base key; the others are deltas added to it.** Checked on `FADEIN`: base
  alpha 255, delta −255 at time 0 and 0 at time 600, so alpha runs from 0 to 255 over 600 ticks.
- `EV` = events `{u32 message id, u32 target, u32 time}`.

Script ids seen: `INIT` 0x001744b3, `HIDE` 0x0016a259, `FADEIN` 0x5b0d9106. Every object starts in `INIT`.

### Message responses **[decomp + verified]**

`MsgR` (per object) and `PkgR` (per package): `Mi` message id, `MC` response count, then per response
`Ri` response id, `Ru` u32 parameter (or `Rs` string) and `Rt` target (GUID; 0 = self; `0xFFFFFFFF` = the
game). Response ids: 0 set script, 1 post to FEng, 2 post to the game, 3 post to sound, `0x100`…
button control, `0x200`… package control, `0x300` if script equals, `0x301` if script differs, `0x500` else,
`0x501` end if. The meanings are in the runtime spec.

## Fonts (`FEngFont`) **[decomp + verified]**

After the two 0x100-byte names (above) comes the `FNTF` block at +0x200:

| Offset | Field |
|---|---|
| 0x00 | char[4] `FNTF`, u32 size |
| 0x08 | u16 version (414), u16 glyph count |
| 0x0C | i32 flags (`0x40009`; bit `0x40000` = 16-byte glyphs) |
| 0x10 | i8 centre x, i8 centre y, u8 ascent, u8 descent |
| 0x14 | i32 glyph table offset (0x80), i32 kerning table offset, i32 shape offset (= end of data), i32 states[24] |

A glyph is 16 bytes, sorted by unicode: `{u16 unicode; u8 width; u8 height; u16 u; u16 v; i8 advance y;
i8 offset x; i8 offset y; u8 kern count; u16 kern index; i16 advance x}` (checked on `font_mw_body`: `A` is
15 × 14 at (118, 64), advance 13). The kerning table is `u32 count` followed by 4-byte entries
`{u16 previous character; i8 kern; u8 low byte of the glyph}`; a glyph's entries are
`[kern index, kern index + kern count)`. Five fonts exist: `font_mw_title`, `font_mw_custom_numbers`,
`font_mw_body` and `tac_numbers` (twice, in `InGameB.bun` and `INGAMEC.BUN`). Layout (decomp
`FEngFont.cpp`): a glyph quad is drawn at x + kern + offset x, y + offset y + the baseline offset, as wide
as max(width, 4), with UVs `u / texture width` to `(u + width + 1) / texture width`; the pen advances by
advance x + kern; `$NAME$` in a string draws the texture of that button icon. Baseline and leading offsets
by font name hash: `0xDCA5485A` 18 / 2, `0x833A8678` 22 / 2, `0xF88A75F9` 18 / 1, `0x71C777D7` 23 / 1.

## Tools

| Tool | What | Language / license |
|---|---|---|
| [NFSTools/FEngLib](https://github.com/NFSTools/FEngLib) | Reads and writes FEng packages (all object types, scripts, message responses); `FEngViewer` / `FEngRender` render them; `FEngCli` compiles packages back into `0x30203` chunks | C#, **no LICENSE file** (all rights reserved by default): facts only |
| [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) `src/Speed/Indep/Src/FEng/`, `Src/Frontend/` | Chunk/tag IDs, package reader, runtime, fonts, screens | C++, CC0-1.0 |
| [SpeedReflect/Nikki](https://github.com/SpeedReflect/Nikki) (`FNGroup`) | Edits colours inside FEng packages (Binary's "FEng Color Editor") | C#, MIT |
