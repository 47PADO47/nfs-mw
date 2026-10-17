# HD content support: provenance

Spec: [specs/hd-content-support.md](../specs/hd-content-support.md). Date: 2026-10-16. Pass: read-only reversing (task R).
No code was written. No cargo was run.

## Restricted sources read

| Source | Licence | What was used |
|---|---|---|
| [NFS HD Content Support](../mods/nfshdcontentsupport.md) (Aero_, AeroWidescreen) | GPL-3.0 | Feature list, option names and defaults, the font scale values, the cursor offset, the FMV size. Behaviour only. Its code, hook sites, code caves, INI layout and entry-point gate were not used. Hook addresses in the mod documentation were used only to find facts in `speed.exe`. |
| `speed.exe` 1.3 (Ghidra MCP, `/speed.exe`, read only) | The game's own code; decompiled read and instruction read | Font glyph draw, texture lookup, cursor draw, movie draw, the stock hash constants |
| Install data (read only) | The user's copy | `TextureInfo` sizes in the stock packs, `FNTF` line heights, `PC_CURSOR` size, movie sizes (via `docs/formats/video.md`) |

Port code read (for targets only): `crates/nfsmw/src/ui/{assets,text}.rs`, `crates/nfsmw/src/frontend/pointer.rs`,
`crates/nfsmw/src/movie/mod.rs`, `libs/blackbox-feng/src/font/{mod,layout}.rs`, `libs/blackbox-hash/src/lib.rs`,
`libs/game-install/src/vfs.rs`, `docs/decisions/0005-native-mod-plugins.md`, `docs/formats/{frontend,textures,video}.md`.

## How the facts were found

### Hash checks (Python, scratchpad, not committed)

- `bStringHash(upper(name))`, seed `0xFFFFFFFF`, multiplier 33.
- `bStringHash("PC_CURSOR") = 0x6B6A294F`. This is the key the stock cursor draw passes to its texture lookup.
  **[verified]**
- The mod's font hashes match `bStringHash` of `FONT_`-prefixed names (see the custom-fonts provenance for the list).
  `TAC_NUMBERS` has no prefix.

### Executable

1. **Cursor draw** (`FUN_0056fdb0`, at 0x0056fdb0, 1970 bytes). Takes the pointer position as two int parameters. Looks up
   the `PC_CURSOR` texture (key 0x6B6A294F, through `FUN_00503400`). Reads its width and height shorts (+0x44, +0x46). Builds
   a quad from the pointer to the pointer plus size, passes its vertex block through `FUN_0045ecf0` (purpose not read),
   and draws it through the sprite draw (`FUN_006d1260`). Gated by a flag at 0x0091CFB9 and by a timer (`DAT_00925b14`, threshold 2.0 after a scale of
   0.00025; the unit is not confirmed). **[decomp]**
2. **Cursor draw caller**: `FUN_00516f70`. **[decomp, one caller]**
3. **Mod cursor sites** 0x0057045C, 0x00570481, 0x00570496, 0x005704BF lie inside `FUN_0056fdb0`, in its last part. The
   locals written there (`local_a0`, `local_90`: pointer x plus width; `local_8c`, `local_7c`: pointer y plus height) are
   the far corners. That the four sites change those four values is **[inferred]**, not checked.
4. **Sprite draw** (`FUN_006d1260`, at 0x006d1260, 1737 bytes). Ten callers, including the cursor draw. The mod's second
   font hook (0x006D14F6) lies inside it. **[verified containment]**
5. **Glyph draw** (`FUN_00585d60`, at 0x00585d60). The first mod font hook (0x00585D70) lies inside it. The draw reads
   the texture object at object +8, its width and height at +0x44 and +0x46. The UV divisor is the texture's width and
   height. **[decomp, verified containment]**. This is the mechanism behind the HD font fix: the divisor is what the
   mod replaces with the native size.
6. **Texture lookup** (`FUN_00503400`, at 0x00503400). Checks that the texture object's +0x24 equals the requested key.
   **[decomp]**. So the mod's "hash at +0x24" is the texture's key, not the font's name hash. Matches the custom-fonts
   provenance.
7. **Movie draw** (`FUN_00599db0`, at 0x00599db0, 683 bytes). Reads the movie texture's width (`MOVSX EAX, word [EDI+0x44]`
   at 0x00599F10), tests it, then stores it (0x00599F16). The mod's FMV cave sits at this store. **[verified instruction
   sequence]**. The draw runs when the state object at 0x0091CB10 has the value 5 at +0x11C (meaning not confirmed).
   It computes a power-of-two size from the width and height. Two float constants (320.0 and 240.0) are stored in its
   argument block; their use was not traced. **[decomp]**
8. **Stock font native sizes.** From the install's `TextureInfo` records (width at +0x44, height at +0x46):
   FONT_MW_BODY 256 x 256, FONT_MW_TITLE 512 x 512, FONT_MW_CUSTOM_NUMBERS 128 x 64. These match the mod's values for
   those three names. **[verified]**
9. **Movie sizes.** All 32 stock movies are 1024 x 512 (see `docs/formats/video.md`, which records this from a full
   decode). **[verified]**

### Install scan

A small script (scratchpad) walked `TexturePackInfoTextures` records (chunk `33310004`, 0x7C each) in `FRONTEND/FrontB.lzc`,
`GLOBAL/GLOBALA.BUN`, `GLOBAL/InGameB.bun`, `FRONTEND/FRONTA.BUN`, `GLOBAL/INGAMEC.BUN`, `GLOBAL/InGameA.bun`,
`GLOBAL/GlobalB.lzc`, `GLOBAL/HUDTEXRACE.BIN` and `FRONTEND/DEMO_FRONTEND.BUN` for the names `PC_CURSOR` and `TAC_NUMBERS`.
`PC_CURSOR` was found as 16 x 16 in `FrontB.lzc` and `GlobalB.lzc`. `TAC_NUMBERS` was not found as a plain texture
record in any of the packs listed; the form in which it is stored was not checked (compressed packs are not decoded by
the script). Font `FNTF` blocks were read by the same walk for the custom-fonts record.

## Lessons

- **The mod's numbers are the stock texture sizes.** That is the key fact. The HD font fix divides the glyph UVs by the
  size the stock texture had, not by the size of the HD texture. With that, one table explains all three font values.
  Reading `TextureInfo` in the install gave the same numbers the mod uses, so the spec reads the native size from the
  install rather than from a table.
- **The texture object, not the font object, is where the draw gets its size.** A quick reading of the mod's offsets
  suggests the font object. The glyph draw reads the texture object for width and height.
- **One movie size.** Every stock movie is 1024 x 512, so the FMV option changes nothing for stock content. Writing that
  down early keeps the default ON safe.
- **Mod sites are a guide, not evidence.** Each mod address was checked for containment in a stock function, and for the
  instruction it sits on. Only the FMV site (the width store) was confirmed by its instruction sequence. The cursor sites
  are inferred. Treat the rest of the mod's addresses the same way.

## Dead ends

- **Cursor offset as a literal.** `search_instructions` for operand `0x10` in `FUN_0056fdb0` found only stack stores
  (0x100, 0x104, 0x108, 0x10c). The 16-pixel offset is not in the stock code as a literal. It is an addition in the
  mod's own cave, or a value the mod supplies. Not a lead for the spec.
- **`get_assembly_context` on the cave addresses.** It returned an empty result. That tool wants referencing addresses.
  `disassemble_function` on `FUN_00599db0` worked and gave the FMV site.
- **Xrefs to the cursor, movie and font cave addresses.** No references (they are code sites).
- **Font names in strings**, `PC_CURSOR` in strings, `CustomFont` and `HD` strings in the executable: none.
- **`TAC_NUMBERS` and `ARIAL` texture sizes** in the plain packs scanned. Not a result. The mod's 128 or 64 for TAC and
  256 for ARIAL are unverified and marked so in the spec.
- **Device pointer 0x00982BDC** as the source of a render height for tiers. Too many readers (about 200) to pin in this pass.

## Verification status

- [verified]: the `PC_CURSOR` hash and size; the three stock font texture sizes; the movie size; the FMV site's
  instruction sequence; the containment of the mod's hook sites in stock functions.
- [decomp]: the cursor gating, the movie state, the glyph draw's use of the texture object.
- [community]: the mod's option names, defaults, its 16-pixel cursor offset, its FMV size, its non-verified font sizes.
- [unconfirmed]: the cursor sites' exact quad corners; the movie state 5 meaning; the timer unit; TAC and ARIAL sizes.
- No game was launched. No window was opened. No render check was done.

## Next leads

1. Disassemble `FUN_0056fdb0` around 0x00570440 to 0x005704D0 and map the four cursor sites onto the quad corners.
2. Identify the state object at 0x0091CB10 and the meaning of value 5 at +0x11C.
3. Find the stock `TAC_NUMBERS` and `ARIAL` textures: check the compressed packs in `GLOBAL/INGAMEC.BUN` and `GLOBAL/InGameB.bun`
   (the plain-pack scan does not decode them).
4. Confirm the glyph UV start `u - 1` on a rendered glyph.
5. Check the pointer timer's unit in the cursor gate.
