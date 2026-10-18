# HD content support: behaviour spec for an optional port feature

Status: **specification, partly implemented** (see Port status at the end). The feature makes HD replacement content display
at the right size in three places: the FEng font glyphs, the mouse pointer and the full-motion videos (FMV).

Sources and licence rule:

- NFS HD Content Support by Aero_ ([docs/mods/nfshdcontentsupport.md](../mods/nfshdcontentsupport.md)), GPL-3.0.
  We use its **behaviour**: which three things it corrects, its option names and defaults, and the constants it
  uses. Its code, its hook sites, its code caves, its INI layout and its entry-point check are not used. Its claims
  are tagged **[community]**. The addresses it lists are used only as a way to find facts in `speed.exe`; they are not
  copied into this spec.
- The stock game: `speed.exe` through Ghidra, read only. Facts are tagged **[verified]** (matched in the executable's
  bytes or in the install's data) or **[decomp]** (read from the decompiled logic, not yet tested in game).
- The install's texture records and movie files, read only. Tagged **[verified]**.
- Port facts come from the files named in each section.

Evidence tags in this file: [verified], [decomp], [community], [unconfirmed].

## 0. Summary

| # | Option | What it corrects | Port default | Stock output change | Main target |
|---|---|---|---|---|---|
| 1 | `hd.fonts` | Glyph UVs use the texture's real size; HD textures need the stock (native) size | on | none (native = real for stock) | `crates/nfsmw/src/ui/assets.rs`, `crates/nfsmw/src/ui/text.rs` |
| 2 | `hd.cursor` | Pointer hotspot ignores the texture size | on | none (16 x 16 stock) | `crates/nfsmw/src/frontend/pointer.rs` |
| 3 | `hd.fmv` | Movie framing ignores HD movie size | on | none (all stock movies 1024 x 512) | `crates/nfsmw/src/movie/mod.rs` |

Global Neon and Global Shadow (Underground titles) are out of scope: the port targets Most Wanted. See section 10.

## 1. Stock facts

### 1.1 Font glyph size [decomp, verified]

- The glyph draw reads the **texture object** of a font (not the font object): width as a signed 16-bit value at
  object +0x44, height at +0x46. Those are the texture's real size (see 1.2).
- The glyph UVs are divided by those sizes: start `(u - 1) / width`, `(v - 1) / height`; end `(u + w + 1) / width`,
  `(v + h + 1) / height`. The glyph coordinates `u`, `v`, `w`, `h` are in pixels of the **stock** texture.
- Consequence: if an HD texture is larger than the stock one and carries the same art scaled up, dividing by its real
  size puts every glyph at the wrong place. The UV must be divided by the stock size, so that a relative position in the
  stock texture is the same relative position in the HD one. This is the reason the HD font fix exists **[community,
  consistent with the decompiled draw]**.

### 1.2 Stock texture sizes [verified]

Read from the `TextureInfo` records of the stock packs (width at +0x44, height at +0x46):

| Font texture | Pack in the install | Width x height |
|---|---|---|
| `FONT_MW_BODY` | `GLOBAL/GLOBALA.BUN` | 256 x 256 |
| `FONT_MW_TITLE` | `FRONTEND/FrontB.lzc` | 512 x 512 |
| `FONT_MW_CUSTOM_NUMBERS` | `FRONTEND/FrontB.lzc` | 128 x 64 |
| `TAC_NUMBERS` | not in the packs scanned | not measured |
| `ARIAL`, `ARIAL12` | not found in the packs scanned | not measured |

The reference mod's values agree with the first three rows **[community, verified for these rows]**. For the last
two it gives 128 or 64 (for `TAC_NUMBERS`) and 256 (for `ARIAL`, `ARIAL12`). Those values are not verified.

### 1.3 Pointer [decomp, verified]

- The pointer is the texture `PC_CURSOR`. Its name hash is `bStringHash("PC_CURSOR") = 0x6B6A294F` **[verified]**. Its
  stock size is 16 x 16 **[verified]**, matching the port's `HOTSPOT` comment in `crates/nfsmw/src/frontend/pointer.rs`.
- The stock pointer routine draws the texture at the pointer position, sized by the texture object's width and height
  (+0x44, +0x46). The quad spans from the pointer to the pointer plus the texture size **[decomp]**.
- The routine is gated: it draws only while a flag is clear and while a timer since the last movement is within a
  limit **[decomp]**. The timer's unit is not confirmed.
- The reference mod adds a fixed 16 to four quad coordinates when the cursor is on **[community]**. The four sites are
  in the quad-corner code of the routine **[inferred from the decompile, not confirmed]**.

### 1.4 Movies [decomp, verified]

- The movie draw reads the movie texture's width and height (+0x44, +0x46) and computes a power-of-two size from them,
  then places a quad. It runs in one game state (object state value 5, the meaning is not confirmed).
- Every stock movie in `MOVIES/` is 1024 x 512 [verified, `docs/formats/video.md`]. The first key frame codes 64 x 32
  macroblocks, and the picture uses rows 64 to 448 for film. The logo has no bars (`docs/formats/video.md`, and
  `framing()` in `crates/nfsmw/src/movie/mod.rs`).
- The reference mod forces the width and height the draw uses to 1024 and 512 **[community]**. The site is the load of
  the width, which the decompile confirms is in the movie draw **[verified instruction sequence]**.

## 2. Option 1: HD font size (`hd.fonts`)

### 2.1 Rule

For each font name, the glyph divisor is the **native size**: the size of the stock texture named in the font's
texture-name field. The rule is:

1. The **built-in table** gives the size when it names the font (section 2.2). The glyph layout is in stock pixels,
   so the stock size is the divisor whatever texture is loaded.
2. Otherwise the stock `TextureInfo` in the install's own pack gives width and height (section 1.2). A mod can replace
   a stock texture inside the install, so this is a fallback, not the first choice.
3. If neither exists, the loaded texture's real size is used. That is the stock behaviour and the fallback.

The native size is passed to the layout as the texture size (the layout already takes a `texture_size` argument, see
`crates/nfsmw/src/ui/text.rs`). No change to the layout library is needed.

### 2.2 Built-in table (fallback only)

| Name | Native width x height | Status |
|---|---|---|
| `font_mw_body` | 256 x 256 | [verified] |
| `font_mw_title` | 512 x 512 | [verified] |
| `font_mw_custom_numbers` | 128 x 64 | [verified] |
| `tac_numbers` | 128 x 64 | [community], not verified |
| `arial`, `arial12` | 256 x 256 | [community], not verified |

A table entry is used only when the install has no stock texture for that name. The table is also checked in a test
against the install (section 9): a mismatch is logged, and the install value wins.

### 2.3 Why this is safe for stock content

For stock content the native size and the real size are the same (1.2), so the output does not change. The option is
on by default for that reason. It changes output only when an HD texture replaces a stock one.

### 2.4 What the option does not change

- The baseline and leading table (custom-fonts spec, section 1.1) is not affected.
- Glyph positions, widths and heights in font pixels are not affected. Only the UV divisor is.
- The scale of a custom font (`custom-fonts.md`) is applied after this and is independent.

## 3. Option 2: pointer hotspot (`hd.cursor`)

### 3.1 Background

The port does not draw the pointer through FEng. It sets the window's cursor image to the `PC_CURSOR` texture with a
fixed hotspot of (2, 2) (`crates/nfsmw/src/frontend/pointer.rs`). The stock game's pointer is drawn at the pointer
position, so a larger HD texture grows away from the hotspot. The reference mod's fixed 16-pixel offset is a fix for
the drawn quad; it does not apply to a window cursor.

### 3.2 Rule

- When the option is on and the loaded `PC_CURSOR` is larger than the stock 16 x 16, the hotspot is scaled by the
  texture size divided by the stock size: `hotspot = (2, 2) * (width / 16, height / 16)`, rounded to the nearest pixel,
  and clamped inside the texture.
- At 16 x 16 the hotspot is (2, 2). The output does not change for stock content.
- When the option is off, the hotspot stays at (2, 2) for any size (the stock behaviour of the port today).
- The mod's fixed offset is not ported. It is a fix for the FEng quad only. If the port ever draws the pointer through
  FEng, the four quad corners get the same fixed 16 pixel offset as the stock draw would need, and this section is
  revisited. That is open (section 10).

### 3.3 Stock pointer visibility

The stock pointer hides after an idle period. The port's window cursor follows the OS and is not changed by this
option.

## 4. Option 3: movie framing (`hd.fmv`)

### 4.1 Rule

- When the option is on, the film movies (every movie except the logo, see below) are framed as 1024 x 512 pictures,
  whatever their real size. The film rows are 64 to 448 of 512, stretched to the window (`Fit::Fill`), which is the
  current `framing()` rule for a 1024 x 512 movie.
- The logo keeps its own rule (`Fit::Contain` at 900 : 480). The logo is named `ealogo*`; the option does not change it.
- When the option is off, `framing()` keeps its current rule: a movie that is not 1024 x 512 is shown with `Fit::Contain`
  at its own aspect.

### 4.2 Effect

- All stock movies are 1024 x 512 [verified], so the option makes no change to stock content.
- It changes output only for a movie file that is not 1024 x 512, which can only come from a user's HD movie folder
  (not in this spec: see section 10). The option is on by default because it is the intended behaviour for HD movies
  and it changes nothing for stock content.

### 4.3 Where

`framing(file, size)` in `crates/nfsmw/src/movie/mod.rs` takes the option as a third argument. The call site is
`MovieScene::fullscreen`. The `HD_MOVIE` constant is `[1024, 512]`. The existing tests in that module check the current
rule and continue to pass with the option on, because the stock sizes are 1024 x 512.

## 5. Settings

The config file is `config.toml` in the config dir ([setup.md](../setup.md)). Our keys, not the mod's:

```toml
[hd_content]
fonts = true    # HD font glyph size from the stock texture (section 2)
cursor = true   # pointer hotspot follows the texture size (section 3)
fmv = true      # movie framing as 1024 x 512 (section 4)
```

- **Defaults ON.** Each option corrects HD content and changes no stock output (sections 2.3, 3.2, 4.2). This is the
  rule "default off unless it fixes a bug": these fix a bug in HD content, so they are on.
- **Console.** `hd fonts on|off`, `hd cursor on|off`, `hd fmv on|off`, and `hd status` (prints the three values and the
  native sizes in use).
- **Menu rows.** Options, Graphics page: "HD fonts", "HD cursor", "HD movies", each on/off. Layout follows
  `crates/nfsmw/src/frontend/graphics_options.rs` (not read in this pass).
- **Interaction.** `custom_fonts.enabled` and `hd.fonts` are independent switches. When a custom font wins for a name,
  its texture comes from the custom file, so the native size is that texture's own size and the stock table of
  section 2.2 is not used for it. This is the open question in section 10.

## 6. Constants

| Constant | Value | Evidence |
|---|---|---|
| `PC_CURSOR` name hash | 0x6B6A294F | [verified] (`bStringHash`, the same as `fe_hash_upper`) |
| Stock pointer size | 16 x 16 | [verified] |
| Stock pointer hotspot | (2, 2) | port constant, `crates/nfsmw/src/frontend/pointer.rs` |
| Stock movie size | 1024 x 512 | [verified] (`docs/formats/video.md`) |
| Film rows | 64 to 448 of 512 | port `FILM_ROWS`, measured in `docs/formats/video.md` |
| Mod's pointer offset | 16 px | [community], not ported |
| Native sizes | section 2.2 | [verified] for three rows |

## 7. Targets

- `crates/nfsmw/src/ui/assets.rs`: read the stock texture size for each font name from the stock pack (the first pack
  that has it, as the stock `TEXTURE_FILES` order does). Keep the loaded texture for drawing. Store the native size per
  font. Engine-generic code does not see the stock list.
- `crates/nfsmw/src/ui/text.rs`: `text_size` passes the native size to the layout (it already passes the loaded
  texture's size).
- `crates/nfsmw/src/ui/hd_fonts.rs` (new, proposed): the native size lookup and the built-in table (section 2.2).
- `libs/blackbox-feng/src/font/`: no change. The layout takes the texture size as an argument. A generic
  `NativeSize` lookup (name to size, with a fallback) could move here later, if it stays free of NFS names.
- `crates/nfsmw/src/frontend/pointer.rs`: hotspot scale (section 3.2).
- `crates/nfsmw/src/movie/mod.rs`: `framing()` takes the option (section 4.3).
- Settings: `crates/nfsmw/src/settings/` (not read in this pass) for the three keys.
- `libs/game-install`: no change. The stock packs are read through `GameDir` as now.

## 8. Risks

- **A wrong native size changes stock output.** The install-read rule (2.1) prevents this for the three verified fonts.
  For the three unverified names the table is only a fallback, and a stock texture in the install overrides it.
- **Layering.** If HD content is delivered through the VFS overlay (ADR 0005), the stock pack must still be read from the
  install, not from the overlay. Otherwise the native size would be the HD size and the fix would do nothing.
- **Hash names.** Names are matched by name hash (`bStringHash`), so the native-size lookup follows the same rule as the
  font lookup. A hash collision is not checked.
- **Movies.** Option 3 changes the framing rule for any non-1024 x 512 movie. If a future HD movie folder is added, its
  framing must be checked on a real file.

## 9. Test list

Unit tests (`crates/nfsmw/src/ui/`, `crates/nfsmw/src/movie/`, `crates/nfsmw/src/frontend/`):

1. Native size from the stock pack: with a stock texture of 256 x 256 and a replacement of 512 x 512, the native size is
   256 x 256 and the glyph UV for `u = 20` is `(20 - 1) / 256`.
2. Fallback: no stock texture for a name, table value used. No table and no stock texture: the loaded texture's size.
3. Table check (install present): each row marked [verified] in section 2.2 equals the install's stock size. A mismatch
   fails the test.
4. Option off: the UV uses the loaded texture's size, the stock behaviour.
5. Pointer: at 16 x 16 the hotspot is (2, 2) with the option on or off. At 32 x 32 with the option on, (4, 4). With the
   option off, (2, 2).
6. Pointer: hotspot clamp inside the texture for a 4 x 4 texture (no panic, hotspot at most (3, 3)).
7. Movie framing: a 1024 x 512 film with the option off gives the current result, with the option on gives the same.
8. Movie framing: a 640 x 480 film with the option on gives `Fit::Fill` with rows 64 to 448 of 512; with the option off
   it gives `Fit::Contain`.
9. Logo: `ealogo*` keeps `Fit::Contain(900 / 480)` with the option on or off.
10. Settings: the three keys parse, default true, and the console toggles change the value without a restart.

Manual tests: with a replacement HD texture for `FONT_MW_BODY` (2x size, same art scaled), the glyphs land on the same
places as with the stock texture. The pointer with a 32 x 32 texture. A film movie of another size.

## 10. Open questions and next leads

1. **Custom fonts and HD fonts together.** A custom font file replaces the texture. Its native size is its own
   texture's size in its file, not the stock size. Confirm that `hd.fonts` then does not apply (section 5).
2. **Pointer in FEng.** If the port ever draws the pointer as an FEng object, the mod's fixed 16-pixel quad offset
   applies and the four sites must be found in the port's draw code. Today the window cursor is used.
3. **Which quad corners the cursor sites change.** The four sites are inferred to be the far corners of the quad (section
   1.3). Confirm by disassembling the stock pointer routine around the four sites.
4. **Movie gate.** The movie draw runs in object state 5 (section 1.4). Confirm the meaning of that state before relying on
   it for HD movies.
5. **Unverified native sizes.** `TAC_NUMBERS`, `ARIAL` and `ARIAL12` (section 2.2). Find their stock texture in another
   pack (the compressed packs in `GLOBAL/INGAMEC.BUN` and `GLOBAL/InGameB.bun`).
6. **Identify the timer unit** of the pointer visibility (section 1.3).
7. **Independence of the options.** The reference mod gates each feature with its own flag and its own hook sites
   **[community]**. Our three options are independent by design (section 5). Confirm with a test that each option
   changes only its own output.

## Port status

Implemented (code and unit tests; the in-game checks of section 9 are not run yet):

- **Fonts (`hd.fonts`).** `libs/blackbox-feng/src/font/native_size.rs` (`uv_size`) chooses the glyph divisor. The game side
  (`crates/nfsmw/src/ui/assets.rs`) reads the stock size of each font texture from the install's own packs, and takes
  the built-in table only when the font's texture is loaded but has no stock copy (a replacement). `texture_size` returns
  the divisor, so the layouts in `text.rs`, `present/blackbox.rs` and `frontend/prompts.rs` needed no change.
  Stock content: the native and loaded sizes are equal for every font that is loaded, so the output does not change.
- **Pointer (`hd.cursor`).** `frontend/pointer.rs`: `hotspot(size, hd_cursor)` scales the (2, 2) tip when the texture is
  larger than 16 x 16 and clamps it inside the texture. `apply` sets the cursor again when the option changes.
- **Movies (`hd.fmv`).** `movie/mod.rs`: `framing(file, size, hd_fmv)` frames every film as 1024 x 512 (rows 64 to 448).
  The logo is unchanged. `fit_rect` is the on-screen rectangle (pure, tested).
- **Settings.** `settings/hd_content.rs`: `[hd_content]` keys `fonts`, `cursor`, `fmv`; env `NFSMW_HD_FONTS`,
  `NFSMW_HD_CURSOR`, `NFSMW_HD_FMV`; flags `--hd-fonts` / `--no-hd-fonts` (and cursor, fmv); console `set hd_fonts off`
  (`hd_cursor`, `hd_fmv`). All default on. Registration lines were added to the shared settings files, `saves.rs`, the
  view arguments, the console key list and `app/mod.rs` (a `publish` system copies the settings to the UI each frame).

Not done or different from the text above:

- **No menu rows.** The Graphics page rows ("HD fonts", "HD cursor", "HD movies") need `frontend/graphics_options.rs`,
  which was outside the task's file list.
- **No `hd` console command.** The console uses the flat `set` form with keys `hd_fonts`, `hd_cursor`, `hd_fmv`.
  `hd status` is not implemented.
- **Native sizes come from the table first.** A modded install can replace `FONT_MW_BODY` in place with a 2048 x 2048
  texture (`GLOBALA.BUN` on the Desktop copy), so the install's copy is not the stock size. The install value is used
  only for a font the table does not name, and a mismatch is logged.
- **Unverified rows.** `tac_numbers`, `arial` and `arial12` are table values only. `tac_numbers` takes the table only when
  its texture is loaded as a replacement, so a stock copy in a pack that is not scanned cannot change.
- **Section 10 open questions** stay open. Question 1 (custom fonts with HD fonts) depends on the custom-fonts work.
