# Text: string tables, subtitles, memory-card strings

For the tag meanings, see [evidence tags](../README.md#evidence-tags).

## Files **[verified]**

| File | Format | What |
|---|---|---|
| `LANGUAGES/English.bin` | bChunk: `00039000 Language` + 6 × `00030201 FEngFont` | All game text (4,586 strings) and the UI fonts ([frontend.md](frontend.md#fonts-fengfont-decomp)) |
| `LANGUAGES/Labels.bin` | `00039000 Language` | Debug table: for 4,546 hashes, the **label name** (e.g. `SUBTITLE_BL_1`) |
| `LANGUAGES/Largest.bin` | `00039000 Language` | Same 4,546 hashes; probably the longest string per label across languages, for layout testing **[unconfirmed]** |
| `LANGUAGES/LanguageTextures.bin` | TPK (`B3300000`) | Per-language textures (pipeline path `Global\LanguageT…`) |
| `LANGUAGES/agree.eng`, `agree.usa` | plain text | EA online service agreement |
| `CREDITS/NA_ENGLISH.TXT`, `UK_ENGLISH.TXT` | UTF-16LE text with BOM | Credits |
| `SUBTITLES/<movie>` (30, no extension) | 8-byte records | FMV subtitles (below) |
| `MEMCARD/LOCALE_*.loc` (English, Russian) | `LOCH` / `LOCI` | Memory-card / save UI strings (below) |

`ENGLISH.BIN.bacc` is a mod-tool backup.

## `0x00039000` Language (STRBlocks) **[decomp + verified]**

Header (`LanguageChunkHeader` in the decomp's `Frontend/Localization/Localize.cpp`). Offsets are
relative to the chunk payload:

| Offset | Field | English / Labels |
|---|---|---|
| 0x00 | i32 HistogramTablePos | 0x20 / 0x20 |
| 0x04 | i32 NumStringRecords | 4,586 / 4,546 |
| 0x08 | i32 StringRecordTablePos | 0x1824 / 0x1824 |
| 0x0C | i32 StringTablePos | 0xA774 / 0xA634 |
| 0x10 | char[16] | `"GLOBAL"` (category name; not read by the loader) |

- **Records** at StringRecordTablePos: `{u32 hash; u32 offset}` × N, where `offset` is relative to
  StringTablePos. They are **sorted by hash** with no duplicates (the game binary-searches them,
  `SearchForString`). **[verified in all three files]**
- **Hash = `bStringHash(label)`** (`h = h*33 + c`, seed `0xFFFFFFFF`). 4,255 of the 4,546 keys in
  `Labels.bin` equal the hash of their own label text **[verified]**.
- **Strings** are NUL-terminated *packed* bytes. Bytes < 0x80 are ASCII. A byte ≥ 0x80 indexes the
  histogram table. If the value found is a small number (1–0x7F), it is a prefix and the next byte
  selects from a second-level block (`EntryTable[value*0x80 + next − 0x80]`). A 0 result decodes as `_`.
  (`WideCharHistogram::UnpackString`) **[decomp]**
- **Histogram** at HistogramTablePos: `{i32 NumEntries; u16 EntryTable[3072]}` = 6,148 bytes, exactly
  the gap 0x20–0x1824. NumEntries = 0x100 in `English.bin` **[decomp + verified]**. GlobalLib keeps this
  block as opaque "unknown data".
- The decomp also lists `0x00039001 LanguageHistogram` as its own chunk ID, but MW stores the histogram
  **inside** the `0x39000` payload. No `0x39001` chunk occurs in these files **[verified]**.

Editors: [SpeedReflect/Nikki](https://github.com/SpeedReflect/Nikki) `STRBlock` (MIT) and
[NFSTools/GlobalLib](https://github.com/NFSTools/GlobalLib) `STRBlock` (MIT), used by Binary (GPL-3).
"LangEd": not found.

## Subtitles **[decomp + verified]**

`SUBTITLES/<movie>` pairs with `MOVIES/<movie>_english_ntsc.vp6` (all 30 subtitle files have a movie;
`attract_movie` and `ealogo` have no subtitles). The file is an array of `SubtitleInfo`
(`Frontend/SubtitleInfo.hpp`):

| Offset | Type | Field |
|---|---|---|
| 0x00 | u16 | startTime; **0xFFFF ends the list** |
| 0x02 | u16 | padding: 0x87 in the first record of each file, 0 in the others **[verified]** |
| 0x04 | u32 | stringHash: upper-case label hash (`FEHashUpper`, see [subtitles.md](../specs/subtitles.md#5-label-hashes-and-text-lookup)), looked up in `Language` |

- There is no header: the array starts at byte 0 and its first record is an ordinary cue (time 0, or 5.4 s in the
  tutorials). 317 records in 30 files, 30 of them `0xFFFF` terminators. Hash `0x001A20BA` (not in any string
  table) appears between lines and means "clear the subtitle". **[verified]**
- Example `blacklist_01`: `SUBTITLE_BL` "Blacklist^" → `SUBTITLE_BL_1` "Hails: Beacon Point" →
  `SUBTITLE_BL_NUM_1` "Blacklist #1".
- The time unit is **0.1 s**, the time since the movie's first frame by the wall clock. **[decomp]** The
  install agrees: every file's last record (always a clear) falls 0.26 to 18.2 s before the movie ends, so
  `storyfmv_bus12` ends its last clear at 148.3 s of a 156.05 s movie. Not a frame count. **[verified]** See
  [subtitles.md](../specs/subtitles.md#3-clock-and-cue-timing).
- 10 hashes in the five `*_tutorial` files (consecutive pairs such as `0x3C4746A0`/`A1`) are in
  neither `English.bin` nor `Labels.bin` **[verified]**. They are base labels (`TUTORIAL_<MOVIE>_PAGE<n>`).
  The two lines are the labels base + `_A` and base + `_B`, hashed with the base as the seed; those labels
  are in `English.bin` **[decomp + verified]**.
- Subtitles show only when the language is not English, except in the tutorials **[decomp]**. Full spec:
  [subtitles.md](../specs/subtitles.md).
- Runtime: `Frontend/SubTitle.hpp` (`SubTitler`). The chunk IDs `0x00039010 Subtitles` /
  `0x00039020 MovieCatalogEntry` exist in the decomp's list, but PC uses the loose files above.

## `MEMCARD/LOCALE_*.loc` (`LOCH`) **[verified bytes; meaning unconfirmed]**

These are **not save games** (saves: [saves.md](saves.md)). `LOCALE_ENGLISH.loc` (668 B):

```
00  "LOCH"  u32 0x14 (header size)  u32 1  u32 1  u32 0x5C
14  "LOCI"  u32 0x48  u32 0x0E  u32 0  u32 1 ...
```

They probably hold the memory-card library's localized messages (the decomp's file list includes
EA's `realmemcard` 3.04.01 headers) **[unconfirmed]**. No public parser found.

## References

| Source | What | License |
|---|---|---|
| [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) `src/Speed/Indep/Src/Frontend/Localization/`, `Frontend/SubTitle*.hpp` | Header struct, lookup, histogram unpacking, subtitles | CC0-1.0 |
| [SpeedReflect/Nikki](https://github.com/SpeedReflect/Nikki) | STRBlock read/write | MIT |
| [NFSTools/GlobalLib](https://github.com/NFSTools/GlobalLib) | STRBlock read/write (older) | MIT |
| [SpeedReflect/Binary](https://github.com/SpeedReflect/Binary) | String Editor UI | GPL-3.0 |
