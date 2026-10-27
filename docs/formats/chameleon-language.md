# Chameleon string tables (`LocalisedText`, type 0x201)

The text of the 2012 game's menus, messages and HUD. Each language is a bundle of its own,
`UI/LANGUAGE/<NNNN>.BNDL`, holding one `LocalisedText` resource (type 0x201, called `Language` by the community
tool) whose main block is the whole table; block 1 to 3 and the import table are empty. Reader and writer:
[`libs/chameleon-text`](../../libs/chameleon-text). The bundle around it is described in
[chameleon-bundle.md](chameleon-bundle.md); how this was learned is in
[provenance/chameleon-tools.md](../provenance/chameleon-tools.md). Evidence tags: [README.md](../README.md#evidence-tags).

Everything here was measured on the 14 language bundles of the 2012 install (v1.4.0.0): all 14 read, and writing a
table back reproduces its block byte for byte **[verified]**.

## The files

| File | Language | File | Language |
|---|---|---|---|
| `0001.BNDL` | English (US) | `0008.BNDL` | Polish |
| `0002.BNDL` | English (UK) | `0009.BNDL` | Czech |
| `0003.BNDL` | French | `0010.BNDL` | Hungarian |
| `0004.BNDL` | Spanish | `0011.BNDL` | Japanese |
| `0005.BNDL` | Italian | `0012.BNDL` | Traditional Chinese |
| `0006.BNDL` | German | `0013.BNDL` | Dutch |
| `0007.BNDL` | Russian | `0014.BNDL` | Arabic |

The language names are the community tool's (YzENT's string editor README); the u32 at the start of each block is the
file number (1 for `0001`, 14 for `0014`) **[verified]**. The bundle has the zlib flag, one resource, no imports. The
resource ids differ per file (`0001` is `D8D53BFA`, `0003` `C28ADF04`, `0011` `AF878D8D`) and say nothing about the
content.

## Block layout

All integers little endian, offsets from the start of the main block. 32-bit layout (the PC game).

| Offset | Type | Field |
|---|---|---|
| 0x00 | u32 | language index: the file number, 1 to 14 |
| 0x04 | u32 | string count (12,432 in all 14 files) |
| 0x08 | u32 | offset of the id array (always 0x10) |
| 0x0C | u32 | offset of the string records (`0x10 + 4 * count`) |
| 0x10 | u32[count] | string ids, in ascending order |
| `0x10 + 4n` | record[count] | string records, one per id, same order |
| after the records | UTF-16LE | the texts, back to back |
| end | zero bytes | padding up to a multiple of 16 |

A record is 8 bytes: `u32 offset` of the text and `u32 length` in UTF-16 code units **including the terminating
NUL**. The texts follow the records in record order with no gap (`offset[i] + 2 * length[i] == offset[i+1]` for all
12,431 neighbours of the English file), the first text starting right after the last record
(`0x10 + 12 * count`). So the file is fully determined by the ids and the texts, which is why a writer reproduces it.

Facts about the content of the English file **[verified]**:

- Ids run from 13,554 to 2,366,054. They are numbers, not name hashes; what they stand for (a label hash of the UI
  scripts, or an index) is not known.
- The ids are sorted but **not unique**: 12,378 distinct values for 12,432 entries, nine ids repeated (the largest
  group is 5 entries) with identical texts (`AUTOLOG` four times, `CRASH ESCAPE` five times). A lookup takes the first.
  The writer keeps duplicates in their given order. Whether the game's lookup relies on the order (a binary search over
  the ids is likely) is **[unconfirmed]**, so a table must stay sorted; `StringTable::new` sorts stably.
- No text is empty. 515 contain a line feed (U+000A).
- Texts embed **markup** the UI scripts expand: `{Widget.Menu...}` (357), `{Controls.ButtonPrompt["AUTOLOG_ACCEPT"]}` (236,
  a button icon), `{Players.LocalPlayer...}`, `{Milestone.Value}`, `{Autolog.Adapter...}` and similar. They are plain
  text in the table and survive editing; do not translate the names inside the braces.
- Colour and icon codes are **private-use characters** in U+F000 to U+F0FF (U+F000 1,234 times, U+F001 1,230, U+F012
  997, U+F051 330, U+F092 and U+F093 279 each, U+F033 256). The string editor's README notes that they look like a
  single odd symbol in its JSON output: they are valid UTF-16 and round-trip, but what each code does (a colour, an icon)
  is **[unconfirmed]**. Edit them as characters, never by their glyph.
- Every language has the same 12,432 ids.

## Other layouts the community tool handles **[community]**

The string editor ([YzENT/Chameleon-String-Editor](https://github.com/YzENT/Chameleon-String-Editor), MIT) supports
`32` (Hot Pursuit 2010 and Most Wanted 2012) and `64` (Hot Pursuit Remastered) bit layouts. The 64-bit one has the same
fields with every offset and length widened to 8 bytes (a u32 followed by 4 bytes that the tool skips): header
`indicator, count, ids offset, +4 padding, records offset`, ids as u32, records as `u32 offset, +4, u32 length, +4`; the
tool writes the ids at 0x18 and the records at `0x18 + 4 * count`. It is not read here (no Remastered files to check).
The editor writes the file with the same padding to 16 bytes. Its first four bytes ("language indicator") are the
language index of this doc.

## Reader and writer

`StringTable::parse(block)` reads the main block; `to_bytes()` writes it; `get(id)`, `all(id)` and `set(id, text)` look up
and edit; ids stay sorted. It refuses a table whose offsets leave the block, a text without its NUL and a text that is
not valid UTF-16 (none of the 14 files has one). The tool that unpacks a bundle to `.dat` files and the editor that
turns the `.dat` into JSON are not needed: the table comes straight from `Bundle::resource(0).blocks[0]`.
