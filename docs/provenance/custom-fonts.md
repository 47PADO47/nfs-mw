# Custom fonts: provenance

Spec: [specs/custom-fonts.md](../specs/custom-fonts.md). Date: 2026-10-16. Pass: read-only reversing (task R).
No code was written. No cargo was run.

## Restricted sources read

| Source | Licence | What was used |
|---|---|---|
| [NFS_CustomFont](../mods/nfs_customfont.md) (xan1242) | MIT per its LICENSE; README has no licence line | Feature list and defaults only (folder, tiers, thresholds, scale table, override, wrap note). Its code, DLL, address tables and INI layout were not used. Its claims are tagged [community] in the spec. |
| `speed.exe` 1.3 (Ghidra MCP, `/speed.exe`, read only) | The game's own code; decompiled read | Font chunk handling, font object constructor, unloader, name lookup, glyph draw, line width, cursor draw, movie draw, the baseline table |
| Install data `D:\Need For Speed Most Wanted Black Edition\` (read only) | The user's copy | Chunk names, `FNTF` blocks, `TextureInfo` sizes, `PC_CURSOR` size |

Port code read (for targets only): `libs/blackbox-feng/src/font/{mod,layout,tests}.rs`, `libs/blackbox-hash/src/lib.rs`,
`libs/game-install/src/vfs.rs`, `crates/nfsmw/src/ui/{assets,text}.rs`, `crates/nfsmw/src/frontend/pointer.rs`,
`crates/nfsmw/src/movie/mod.rs`, `docs/decisions/0005-native-mod-plugins.md`, `docs/formats/{frontend,textures,video}.md`.

## How the facts were found

### Name hashes (Python, scratchpad, not committed)

`bStringHash(upper(name))` with seed `0xFFFFFFFF` and multiplier 33. Results:

- `FONT_ARIAL` 0xA87927BE, `FONT_ARIAL12` 0xAB6215C1, `FONT_MW_BODY` 0x545570C6, `FONT_MW_TITLE` 0xE0463AFA,
  `FONT_MW_CUSTOM_NUMBERS` 0xC7BD87CE, `TAC_NUMBERS` 0x01424892: all six match the values in the mod's documentation
  **when the `FONT_` prefix is used** (except `TAC_NUMBERS`, which has no prefix). Without the prefix the hashes differ.
  The mod's labels are therefore shorthand, not names.
- `PC_CURSOR` 0x6B6A294F. It is the key passed to the texture lookup in the stock cursor draw.
- The four unnamed baseline keys (0xDCA5485A, 0x833A8678, 0xF88A75F9, 0x71C777D7) did not match 60 candidate names
  (`FONT_` + words such as HEADER, SMALL, LARGE, NUMBERS, HUD, BUTTON, TIMER). They remain unidentified.

### Executable

1. Chunk name hash: the function that hashes a chunk name (`FUN_005af1c0`) has the same formula as `bStringHash`
   (upper-casing bytes 0x61 to 0x7A, multiplier 0x21, start -1). **[decomp]**
2. Font loader (`FUN_00591eb0`, at 0x00591eb0): tests the tag 0x00030201, allocates 0x30 bytes, calls the constructor
   (`FUN_00585ba0`), links the object at the head of the list at 0x0091D904, returns 1; otherwise returns 0. **[decomp]**
3. Font constructor (`FUN_00585ba0`, at 0x00585ba0): sets name pointer (chunk +8), texture-name pointer (chunk +0x108),
   texture object (`FUN_00503400` with the texture-name hash), FNTF pointer (`FUN_007e8930` of chunk +0x208),
   name hash (+0x14), texture hash (+0x18), line height as float from FNTF +0x24 (+0x24), baseline and leading from a
   table (+0x28, +0x2C). **[decomp]**
4. Font unloader (`FUN_0059ac10`, at 0x0059ac10): same tag test, finds the object by name hash (`FUN_00585b50`,
   at 0x00585b50), unlinks and frees it. **[decomp]**
5. Lookup (`FUN_00585b50`): walks the list by name hash at object +0x14; if not found, one alias pair at 0x008f48b0
   (hash 0x9583AA1A redirects to 0x5B9D88B9). **[decomp]**
6. Baseline and leading table at 0x008f48b8, entries of 12 bytes, sentinel 0xFFFFFFFF at 0x008f48e8. Values: 18/2, 22/2,
   18/1, 23/1 for the four keys above. The port's `EXTRA` table (`layout.rs`) carries the same four keys and values.
   **[verified bytes, matches port]**
7. Glyph draw (`FUN_00585d60`, at 0x00585d60, 289 bytes): reads the texture object at object +8, its width and height
   shorts at +0x44 and +0x46, glyph fields at +2, +3, +4, +6, +9, +10, and the baseline at object +0x28. Start UV is
   `(u - 1) / width`. The quad corners are passed as ints (truncation). **[decomp]**
8. Width routine (`FUN_00586130`, at 0x00586130): sums glyph advances (from `FUN_00586050`, not read) and compares with
   the caller's maximum width, in unscaled units. This is the place of the word-wrap issue. **[decomp]**
9. Stock font facts from the install (scan, section below): FNTF +0x24 equals ascent plus descent for all four stock
   fonts. **[verified]**
10. Texture sizes from the install: FONT_MW_BODY 256 x 256, FONT_MW_TITLE 512 x 512, FONT_MW_CUSTOM_NUMBERS 128 x 64.
    **[verified]**, as the mod states.
11. Mod hook sites that fall inside functions read here: 0x00585D70 is inside `FUN_00585d60` (glyph draw). 0x006D14F6
    is inside `FUN_006d1260` (sprite draw, ten callers including the cursor draw). These confirm the mod hooks the
    glyph draw and the sprite draw; the mod's own routines were not read. **[verified containment]**

### Install scan

A small script (scratchpad) walked the bChunk headers of `FRONTEND/FrontB.lzc`, `GLOBAL/GLOBALA.BUN` and
`GLOBAL/InGameB.bun`, found `00030201` chunks whose payload has `FNTF` at +0x200, and read the `TexturePackInfoTextures`
(`33310004`) records (0x7C each, hash at +0x24, width and height at +0x44). Then the same for `PC_CURSOR` in
`FrontB.lzc` and `GlobalB.lzc` (16 x 16). Results are in the spec, section 1.2.

## Lessons

- **Names are not in the executable.** Searching for `font_`, `FONT_`, `tac_numbers`, `PC_CURSOR`, `FNTF` in the
  strings returned nothing. Font names live in the install's packages. Search the executable for the little-endian hash
  words instead (search_byte_patterns). That found the baseline table and the budget table.
- **Read the reader before reading the table.** The region at 0x008f4000 looked like a font table (name hashes
  next to 0x100 and 0x40000 words). The reader (`FUN_0056b820`) shows an 8-byte record of (id, bytes) with 11 entries,
  summed as a memory budget. It is not a scale table. The stride cannot be guessed from the bytes.
- **The mod's offsets name different objects.** The mod documents "hash at +0x24 of the font object". In the stock
  code, the texture object holds the texture key at +0x24 (the texture lookup checks it), and the font object holds its
  name hash at +0x14. The mod's identity is therefore the texture key, which equals the font name in practice.
- **Texture sizes are cheap to check.** A 40-line scan of `TexturePackInfoTextures` gives width and height for each
  stock texture. No decode is needed. `tools/chunkdump.py` prints the chunk tree but not the texture fields. It must be
  run without `-I`, because it imports a sibling module from its own folder.
- **Cross-check the port's tables against the executable.** The port's baseline table was copied from the stock table
  at 0x008f48b8 and it matches. The check took one byte search and one read.

## Dead ends

- **Font-name strings in the executable.** `search_strings` for `font_`, `FONT_`, `tac_numbers`, `PC_CURSOR`, `FNTF`,
  `.ffn`, `ARIAL` returned no font names. Not a lead.
- **Byte search for `FONT_MW_TITLE` (0xE0463AFA) and `FONT_MW_CUSTOM_NUMBERS` (0xC7BD87CE).** No match in the
  executable (`FA3A46E0`, `CE87BDC7`). They are compared at run time, or come from the packs only.
- **0x008f4000 table as a scale table.** It holds the hashes of `ARIAL`, `ARIAL12`, `MW_BODY`, `TAC_NUMBERS` (and others),
  but its second word is a byte count summed by `FUN_0056b820`. Not a scale. The mod's 256, 512, 128, 64 values are
  not stored as scales in the executable. They are the stock texture sizes (verified in the install).
- **Xrefs to the mod's cave addresses.** `get_xrefs_to` on 0x0057045C, 0x00570481, 0x00570496, 0x005704BF, 0x00599F16,
  0x00585D70, 0x006D14F6 returned nothing: those are code sites, not data references.
- **`get_assembly_context` on the cave addresses** returned an empty result. The tool wants addresses that *reference*
  something, not instruction addresses. Use `disassemble_function` or `read_memory` for code sites.
- **Literal `+16` in the cursor draw.** `search_instructions` for operand `0x10` in `FUN_0056fdb0` found only stack
  stores. The mod's offset is not a literal; it is a value added at run time or in its own cave. The four sites were
  not confirmed as quad corners (see spec section 12).
- **Device pointer 0x00982BDC** (the mod's source for the viewport height). It is read by about 200 functions. The
  one that sets the render height was not pinned.

## Verification status

- Facts tagged [verified] were checked by byte search, a decompile read or an install scan.
- Facts tagged [decomp] were read from the decompiled logic. None was run in game.
- No game was launched. No window was opened. Render checks were not done.

## Next leads

See spec section 12. In short: identify the four baseline keys; find the TAC and ARIAL texture sizes in other packs; pin
the viewport-height reader; check the glyph UV `u - 1` on a render.
