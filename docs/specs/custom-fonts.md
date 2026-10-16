# Custom fonts: behaviour spec for an optional port feature

Status: **specification, partly implemented** (see Port status at the end). The feature lets a player replace the
FEng bitmap fonts with files from a user folder, choose a set per render height and scale each font.

Sources and licence rule:

- The stock game: `speed.exe` through Ghidra, read only. Facts are tagged **[verified]** (matched in the
  executable's bytes or in the install's data) or **[decomp]** (read from the decompiled logic, not yet tested in
  game). Addresses below are our own findings for verification and are not a copy of any mod's tables.
- The install's font and texture chunks, read only, through a scan script. Tagged **[verified]**.
- NFS_CustomFont by xan1242 ([docs/mods/nfs_customfont.md](../mods/nfs_customfont.md)). Its licence file is MIT, but
  its README has no licence statement. We use its **behaviour** only: which features exist and their defaults.
  Its code, DLL patching, address tables and INI layout are not used. Its claims are tagged **[community]**.
- Port facts come from the files named in each section.

Evidence tags in this file: [verified], [decomp], [community], [unconfirmed].

## 0. Summary

| # | Feature | Original (stock game) | Port default | Main target |
|---|---|---|---|---|
| 1 | Font files from a user folder | none | off (`custom_fonts.enabled = false`) | `libs/blackbox-feng/src/font/` (parser unchanged), `crates/nfsmw/src/ui/fonts/` (new, proposed) |
| 2 | Three resolution tiers chosen by render height | none | thresholds 960 and 1200 px | `crates/nfsmw/src/ui/fonts/tier.rs` (proposed) |
| 3 | Per-font scale per tier | none | 1.0 for every font | `libs/blackbox-feng/src/font/layout.rs` (scale argument) |
| 4 | Same-name custom font replaces the stock chunk | none | on, when the feature is on | `crates/nfsmw/src/ui/assets.rs` |
| 5 | Measurement and drawing follow the scale | none | scaled; word wrap fixed (section 6.3) | `libs/blackbox-feng/src/font/layout.rs`, `crates/nfsmw/src/ui/text.rs` |
| 6 | Fall back to the stock font when a custom font fails | none | always | `crates/nfsmw/src/ui/assets.rs` |

## 1. Stock font facts (what the port must match)

### 1.1 Chunk and object

- A font is a chunk with tag `0x00030201` (`FEngFont`). Its payload holds the font name in the first 0x100 bytes,
  the texture name in the next 0x100 bytes, and the `FNTF` block at payload offset 0x200. **[verified]** (the
  port's `Font::parse` reads the same layout; three chunks of the install were checked, see section 1.2.)
- The name hash is `bStringHash` of the name with ASCII letters upper-cased, seed `0xFFFFFFFF`, multiplier 33.
  The stock loader's hash routine has this formula **[verified by decompile]**. `libs/blackbox-hash` already
  implements it, and `blackbox_feng::fe_hash_upper` is the same function.
- On load, the stock loader accepts a chunk only when its tag is `0x00030201`. It builds one object, links it into
  the list of loaded fonts, and returns 1 (accepted). Any other tag returns 0 **[decomp]**. A replacement that
  returns 1 without the stock load therefore skips the stock font.
- On unload, the object is found by name hash, unlinked and freed **[decomp]**.
- Object fields set by the constructor **[decomp]**:

| Object offset | Meaning |
|---|---|
| +0x08 | texture object (looked up by the texture-name hash) |
| +0x0C | pointer to the `FNTF` block |
| +0x14 | name hash |
| +0x18 | texture-name hash |
| +0x1C, +0x20 | pointers to the name and texture-name strings |
| +0x24 | line height, a float copied from the `FNTF` block (see 1.2) |
| +0x28 | baseline offset, a float (default 0) |
| +0x2C | leading scale, a float (default 1.0) |

- Baseline and leading come from a small table of four entries **[verified bytes, 0x008f48b8]**. Each entry is
  `{u32 name hash, f32 baseline px, f32 leading scale}`, ending with a sentinel `0xFFFFFFFF`. The entries and the
  port's `EXTRA` table in `libs/blackbox-feng/src/font/layout.rs` agree:

| Name hash | Baseline (px) | Leading scale | Name |
|---|---|---|---|
| 0xDCA5485A | 18 | 2 | not identified in this pass |
| 0x833A8678 | 22 | 2 | not identified in this pass |
| 0xF88A75F9 | 18 | 1 | not identified in this pass |
| 0x71C777D7 | 23 | 1 | not identified in this pass |

- One alias pair also exists **[decomp]**: a lookup of name hash 0x9583AA1A is redirected to 0x5B9D88B9 when the
  first lookup misses. The names are not identified.

### 1.2 Stock fonts in the install **[verified]**

Scanned read-only: the `FNTF` block, the texture records (`TexturePackInfoTextures`, 0x7C records, width at +0x44,
height at +0x46) and the chunk names.

| Name | File | Ascent / descent | `FNTF` +0x24 | Texture size (w x h) |
|---|---|---|---|---|
| `font_mw_body` (`FONT_MW_BODY`) | `GLOBAL/GLOBALA.BUN` | 18 / 5 | 23 | 256 x 256 |
| `font_mw_title` (`FONT_MW_TITLE`) | `FRONTEND/FrontB.lzc` | 34 / 10 | 44 | 512 x 512 |
| `font_mw_custom_numbers` (`FONT_MW_CUSTOM_NUMBERS`) | `FRONTEND/FrontB.lzc` | 36 / 9 | 45 | 128 x 64 |
| `tac_numbers` (`TAC_NUMBERS`) | `GLOBAL/InGameB.bun` | 26 / 5 | 31 | not in the three packs scanned |

- `FNTF` +0x24 equals ascent plus descent for all four fonts. It is the line height. The port's `Font::height()`
  computes the same value from the ascent and descent bytes.
- Font names: the executable holds the name hashes of `FONT_ARIAL`, `FONT_ARIAL12`, `FONT_MW_BODY` and
  `TAC_NUMBERS` as 32-bit data words (in a resource-budget table at 0x008f4000 and the baseline table). The hashes of
  `FONT_MW_TITLE` and `FONT_MW_CUSTOM_NUMBERS` do not occur as words in the executable **[verified search]**. Font
  names do not occur as strings in the executable. The `ARIAL` fonts were not found in the three files scanned
  **[unconfirmed present]**.

### 1.3 Glyph draw **[decomp]**

The glyph draw reads the texture object at object+8 for its size (+0x44, +0x46). A glyph record is 16 bytes (see
`docs/formats/frontend.md`). For a pen at `(x, y)`:

- quad left = trunc(x + offset_x); quad top = trunc(y + offset_y + baseline);
  quad right = left + max(width, 4); quad bottom = top + height;
- UV start = `(u - 1) / texture width`, `(v - 1) / texture height`;
- UV end = `(u + width + 1) / texture width`, `(v + height + 1) / texture height`.

Note that the UV start is one texel before the glyph. The port uses `u / width` (section 9.1).

### 1.4 Line-breaking **[decomp]**

The stock width routine walks the glyphs, accumulates the unscaled advance and, when wrapping is enabled, breaks at
the last space once the accumulated width exceeds the caller's maximum width. Both the accumulated width and the
maximum are in unscaled font pixels.

## 2. Font files and folder

- **Folder.** `<config dir>/fonts/`, where the config dir is the one that holds `config.toml` (`%APPDATA%\nfsmw\config\`
  on Windows, `~/.config/nfsmw/` on Linux, see [setup.md](../setup.md)). The folder is per user. Nothing is written
  to the install. The name is configurable (section 8).
- **Tier folders.** `fonts/high/`, `fonts/mid/`, `fonts/low/`. A file may also sit directly in `fonts/`; it counts as
  Mid. Subfolders are not searched further.
- **Name.** The font name is the file name without its extension. Matching uses the name hash (section 1.1), so
  `font_mw_body.ffn` and `FONT_MW_BODY.ffn` both match the chunk named `font_mw_body`. Names are case-insensitive.
- **File format.** One bChunk file (`&[u8]` to the format code) holding:
  1. one `0x00030201` chunk, the `FEngFont` for the font (section 1.1);
  2. optionally one texture pack (`0x33300000`, see [textures.md](../formats/textures.md)) holding the texture named in
     the font's texture-name field.
  If the texture is not in the file, it is looked up in the stock UI texture packs, as today.
- **Decision: not the VFS overlay.** [ADR 0005](../decisions/0005-native-mod-plugins.md) section 5 defines a
  file-level overlay for `GameDir`. A font file would replace a whole stock `.BUN` or `.lzc`, which holds other packages
  and textures. The overlay cannot replace one chunk, and the ADR lists chunk-level patches as a later step. The
  user font folder is therefore a dedicated per-user directory read by the UI code. A later mod package may register
  fonts through the ADR's `fonts` content registry. Those entries should feed the same table as this folder (section 4)
  and must follow the same override rule.
- **Reads.** The folder is read once at start-up and again on a tier change or a reload. Entries are read in sorted
  name order so that results do not depend on the file system.

## 3. Tier selection

- **Input.** The render height in physical pixels of the window's surface (not FEng units). FEng units are 480 high
  (`FENG_HEIGHT`) and are not used here.
- **Thresholds.** `high_min_height = 1200` and `mid_min_height = 960` (defaults, section 8).
  - tier = High when height >= `high_min_height`;
  - else tier = Mid when height >= `mid_min_height`;
  - else tier = Low.
  The reference mod's documentation describes the comparison as "exceeds". This spec uses "at or above" so the
  boundary belongs to the upper tier. The rule is stated explicitly and tested at exactly 960 and 1200 (section 11).
  **[community]** for the mod's comparison; the boundary choice is ours.
- **Re-evaluation.** On window resize the tier is recomputed. A tier change reloads the custom font set (section 4.2).
  No stock font is reloaded.
- **Fixed override.** A config value may pin the tier (`high`, `mid`, `low`, or `auto`). Default `auto`.

## 4. Which file each name uses, and its scale

### 4.1 Resolution order (our design)

For a font name and the current tier T, the candidate files are tried in this order: the file in tier T, then the file
in the next lower tier, down to Low. A High file is never used at Mid or Low, because High fonts are drawn larger for
higher resolutions. The first candidate found is used. If none is found, the stock font is used (section 5).

The reference mod's precedence is "High over Mid over Low, first name wins" **[community]**. Its loading loop is
described as running from the selected tier down, which is consistent with our order for scales but not clearly for
files. This is an open question (section 10).

### 4.2 Loading

- At a tier change, the set is rebuilt: every name is resolved again (4.1), each file is parsed, and the stock font
  is kept for names with no candidate.
- Duplicate names inside one tier: the first file in sorted order is used and a warning is logged.
- A file that fails to parse is skipped with a log line (section 7).

### 4.3 Scale

- Each tier has its own scale table keyed by font name (section 8). Default: empty, so every scale is 1.0.
- A font's scale comes from the table of the tier of the file **actually used**. A High file that falls back to Mid
  takes its scale from the Mid table.
- A scale entry for a name with no loaded file has no effect. It is kept in the file and ignored.
- The scale is one float applied to both width and height. Scales must be > 0 and <= 4; other values are rejected
  with a log line and the default 1.0 is used.

## 5. Stock override

- A custom font whose name hash equals a stock font's name hash replaces the stock font. Matching is by name hash
  only; the file name is not compared with the stock file name.
- When a custom font is chosen for a name, the stock chunk for that name is not turned into an object. This
  matches the stock loader's "accept and skip" convention (section 1.1): the loader reports the chunk as handled.
- The baseline and leading table (section 1.1) still applies to the replacement, keyed by name. A replacement does not
  get its own baseline unless the file says so. Our design: the table stays in force.
- The port's stock load (`UiAssets::load` in `crates/nfsmw/src/ui/assets.rs`) keeps its `or_insert` rule. The custom
  set is inserted first, so it wins.

## 6. Measurement and drawing with the scale

### 6.1 Order of operations

1. Layout runs in font pixels, with the stock rules (section 1.3, 1.4), on the glyph table of the chosen file.
2. The result is multiplied by the scale. Width, height, glyph quads (position and size) and the baseline and leading
   are scaled together.
3. Measurement (`text_size`) returns the scaled width and height. Drawing uses the same scaled quads, so the two
   cannot disagree.

Both steps live in one function that takes the scale as an argument. Measurement and drawing do not each apply the
scale on their own.

### 6.2 Rounding

- Stock positions are truncated to whole pixels (section 1.3). The port keeps that truncation in font pixels (step 1).
- The scaled result is not rounded again. The renderer receives floats. At scale 1.0 the quads must equal the stock
  quads exactly (section 11).

### 6.3 Word wrap: the stock bug and the fix

- **The stock bug [decomp].** The width routine compares unscaled accumulated advances with the caller's maximum width.
  A scale applied to the returned width afterwards makes the break happen at the wrong place. At scale above 1, a line
  overflows its box by about the scale. At scale below 1, it wraps too early. The reference mod lists word wrap as
  broken for MW **[community]**.
- **Decision: fix it, and document it.** Wrap runs in scaled space. The maximum width is divided by the scale before the
  break decision, the advances are in font pixels, and the returned width is multiplied by the scale. The result is
  the same as the stock result at scale 1.0.
- There is no setting to restore the stock bug. A developer test may compare the two outputs (section 11).

### 6.4 Where the scale is applied

- Layout (`libs/blackbox-feng/src/font/layout.rs`) takes the scale. It is a field of `TextStyle` or an argument of
  `Font::layout`. The choice is an implementation detail; the rule is that one call does both steps.
- `text_size` in `crates/nfsmw/src/ui/text.rs` uses that same call.
- The HUD and the menus call the same layout, so no screen needs its own scale code.

## 7. Failure modes

| Case | Behaviour |
|---|---|
| Custom folder missing | Feature on, no files: stock fonts only. One info line. |
| Tier folder missing | That tier has no files; fall back per 4.1. |
| Unreadable or unparseable file | Skipped, one warning with the path. Stock font used for that name. |
| File parses but texture missing | Stock texture packs are searched (section 2). If still missing, the name falls back to stock and a warning is logged. |
| Unknown name in a scale table | Ignored (4.3). |
| Name in two tiers | Resolved by 4.1, with no warning. Duplicates inside one tier warn (4.2). |
| Scale out of range | Default 1.0 with a warning (4.3). |
| Glyph table with unsorted unicode | Parse as stock: binary search fails, the glyph is missing. Log once. |
| Two files with the same name in different folders | Tier rule (4.1). |
| Feature turned off at run time | Stock set restored at the next layout (no restart). |

## 8. Settings

The config file is `config.toml` in the config dir ([setup.md](../setup.md)). Keys are our names, not the mod's.

```toml
[custom_fonts]
enabled = false            # default OFF: the feature changes every text screen
folder = "fonts"           # relative to the config dir
tier = "auto"              # auto | high | mid | low
high_min_height = 1200     # physical pixels
mid_min_height = 960

[custom_fonts.scale.high]  # font name -> scale; empty = 1.0 for all
# font_mw_body = 1.25

[custom_fonts.scale.mid]
[custom_fonts.scale.low]
```

- **Default OFF.** The feature replaces fonts for every player who turns it on. Off keeps the stock output exactly.
- **Console.** `fonts on`, `fonts off`, `fonts reload` (re-reads the folder), `fonts tier` (prints the chosen tier and
  the height), `fonts list` (prints each name, the file used and its scale).
- **Menu row.** Options, Graphics page: "Custom fonts" (on/off), "Font tier" (auto/high/mid/low), and "Reload fonts"
  (an action row, shown only when the feature is on). Layout follows the existing graphics option rows
  (`crates/nfsmw/src/frontend/graphics_options.rs`, not read in this pass).
- **Persistence.** The values are saved with the rest of the config. Scale tables are not editable from the menu.

## 9. Constants and targets

### 9.1 Constants

| Constant | Value | Source |
|---|---|---|
| Font chunk tag | `0x00030201` | [verified] |
| Name field, texture-name field, FNTF offset | 0x100, 0x100, 0x200 | [verified] (`Font::parse`) |
| Line height offset in FNTF | +0x24 (= ascent + descent) | [verified] (4 fonts) |
| Stock object size | 0x30 bytes | [decomp] |
| Minimum glyph width | 4 px | [decomp], also in port |
| Tier thresholds | 960 and 1200 px | [community], our defaults |
| Scale range | (0, 4] | our choice |
| UV start | `u - 1`, `v - 1` (stock) | [decomp]; port must match (see below) |

Port differences from the stock draw, found in this pass **[decomp]**:

1. The UV start is one texel before the glyph in the original (`u - 1`). `libs/blackbox-feng/src/font/layout.rs` uses
   `u`. Decision: match the original, so pixel output is the same. Verify on a render.
2. Quad corners are integer pixels in the original. The port keeps floats. Decision: truncate in font pixels before
   scaling (6.2).

### 9.2 Targets

- `libs/blackbox-feng/src/font/layout.rs`: scale argument, scaled wrap, UV start. Engine-generic (no NFS names).
- `libs/blackbox-feng/src/font/mod.rs`: no change to the parser. Maybe a `Font::with_name_override` helper later.
- `crates/nfsmw/src/ui/fonts/` (new): `folder.rs` (scan, sorted), `tier.rs` (height to tier), `set.rs` (candidate
  order, scales), `settings.rs` (TOML keys). Game-specific names are allowed here.
- `crates/nfsmw/src/ui/assets.rs`: custom set first, stock second; the stock chunk is skipped when a custom
  one wins.
- `crates/nfsmw/src/ui/text.rs`: `text_size` passes the scale.
- `libs/game-install`: no change. The install is read only as before.

Engine-generic parts (folder scan, tier selection, scale table and the candidate rule) can move to `libs/` later if
they are free of NFS names. Until then they stay in `crates/nfsmw`.

## 10. Open questions

1. The reference mod's file precedence (section 4.1). Ours is per tier. Confirm by playing with two files of the same
   name in two tiers, if such a test case can be built.
2. Whether the stock `FNTF` line height (+0x24) is ever used to size a replacement. The spec assumes the port
   computes height from the replacement's own ascent and descent.
3. The identity of the four unnamed names in the baseline table (1.1) and of the alias pair. A replacement for one of
   them would need to know its name.
4. `ARIAL` and `ARIAL12` fonts: present in the install or not (1.2).

## 11. Test list

Unit tests (`libs/blackbox-feng/src/font/tests.rs` and new tests in `crates/nfsmw/src/ui/fonts/`):

1. Layout at scale 1.0 gives the same quads as the current `layout` for `font_mw_body` and for `tac_numbers` (fixture
   from the install, or the existing synthetic font in `tests.rs`).
2. Scaled width and height are the unscaled values times the scale, for a scale of 0.5, 1.25 and 2.
3. Word wrap: a string that fits at scale 1 wraps at the same place at scale 2 when the maximum width is also scaled.
   Compare with the stock result divided by the scale. A string with a space that is 1 px too wide wraps.
4. Measurement equals the drawn extent: the bounding box of the quads equals `text_size`.
5. UV start is `u - 1` over the texture width (and `v - 1` over the height).
6. Tier: height 1199 gives Mid, 1200 gives High, 959 gives Low, 960 gives Mid. Resize across a threshold changes the
   tier.
7. Candidate order: at Mid, a name with a High and a Low file uses the Low file; a name with only a High file uses
   stock.
8. Scale comes from the table of the tier of the file used (4.3).
9. Duplicate name in one tier: the first sorted file is used, a warning is logged.
10. Missing folder, unreadable file, file with a bad tag: stock fonts only, no panic.
11. Out-of-range scale (0, 0, -1, 5): default 1.0 and a warning.
12. Override: a custom `font_mw_body` replaces the stock object (the stock chunk is not linked). Restore at feature
    off.
13. Install check (feature on, install present): every stock font name in 1.2 still resolves to its stock object when no
    custom file exists.

Manual tests (in game, with a build that has the feature on): one screen per family (frontend menu, HUD race, pause
menu), at 1080 and 1440 height, and a long string in a wrapped box at scale 1.25.

## 12. Next leads

- Identify the four unnamed name hashes in the baseline table (0xDCA5485A, 0x833A8678, 0xF88A75F9, 0x71C777D7). Try the
  names used by the frontend and HUD packages, read by a name scan of the install's `.fng` packages, and the resource
  handles in `docs/formats/frontend.md`.
- Find the TAC number texture (`tac_numbers`) size. Scan the other texture packs that carry `tac_numbers`
  (`GLOBAL/INGAMEC.BUN`, `GLOBAL/InGameB.bun` compressed packs).
- Check whether `ARIAL` / `ARIAL12` fonts exist in any install file (scan all `.BUN`, `.lzc`, `.bin` under `FRONTEND/` and
  `GLOBAL/`).
- Locate the code that reads the render height (the device or viewport read sites near 0x00982BDC are many; the
  reader that sets the 480-unit scale is not pinned).
- Confirm the glyph UV `u - 1` by a pixel comparison of one glyph in a render.
- Check the stock behaviour at a tier boundary with the game's own resolution setting, if the game has one.

## Port status

Implemented (code and unit tests; the in-game checks of section 11 are not run yet):

- **Loader.** `crates/nfsmw/src/ui/fonts/` (`folder.rs` scans the folder, `set.rs` parses the files, `tier.rs` picks the
  tier). A file is one bChunk with an `FEngFont` chunk (`blackbox_feng::Font::parse`) and optionally a texture pack
  (`blackbox_tpk::read_texture_packs`). A file that cannot be read or parsed is skipped with a warning, so the stock font
  stays. The parsers are shared; nothing in `libs/` names the game.
- **Tiers.** `Tier::chain` gives the lookup order: the tier, then each lower one. Thresholds 1200 and 960, the boundary in
  the upper tier. Files directly in the folder count as Mid.
- **Stock override.** `UiAssets::font` and `UiAssets::texture` return the custom font or texture for a name hash when the
  feature is on and a file has it. The stock chunks are still loaded, so turning the feature off restores the stock
  fonts at once (no restart). The override matches the chunk's own name hash, not the file name.
- **Scale and wrap.** `Font::layout_scaled` (`libs/blackbox-feng/src/font/layout.rs`) lays out in font pixels and
  multiplies the quads, width and height by the scale. Word wrap compares the advances with the maximum width divided by
  the scale, so the break falls where the scaled text would break. At scale 1.0 it equals `Font::layout` (tests in
  `libs/blackbox-feng/src/font/tests.rs`). The text, prompt and presenter layouts use `UiAssets::font_scale`.
- **Texture size.** A custom font's own texture is laid out on its own size (section 2 of `hd-content-support.md`), not the
  stock size.
- **Settings.** `crates/nfsmw/src/settings/custom_fonts.rs`: `[custom_fonts]` with `enabled`, `tier`, `scale.high`,
  `scale.mid`, `scale.low` (scales outside (0, 4] are reported and ignored); `folder` is read from the config file at start-up
  (default `fonts` in the config folder). Env `NFSMW_CUSTOM_FONTS`; flags `--custom-fonts` / `--no-custom-fonts`; console
  `set custom_fonts on|off`; one row "Custom Fonts" on the Video screen. Default off. The row and the console key are
  registered in `frontend/options*.rs`, `devtools/console/settings_*.rs`, and the saves payload.
- **Tier.** `publish` (each frame) computes the tier from the primary window's physical height, so a resize takes effect
  without a reload.

Not done, or different from the text above:

- **Per-font scale table.** The scale is one value per tier (`scale.high/mid/low`), for every custom font. The per-name
  table of section 8 is not in the port.
- **Thresholds are fixed.** `high_min_height` and `mid_min_height` are not config keys.
- **No `fonts` console commands** (`reload`, `tier`, `list`), no Reload row, no Font tier row, and no Graphics page placement.
  The tier can be set only in the config file (`tier = "auto" | "high" | "mid" | "low"`).
- **No reload.** All files are parsed at start-up; a tier change needs no reload, and a file added to the folder after
  start-up is not seen until the next start.
- **Glyph UV start (section 9.1, item 1) not changed.** The layout still uses `u / width`, not `(u - 1) / width`. This is a
  pixel change for stock and custom fonts alike and needs a render check first.
- **Chunk-level tests against the install** (section 11, item 13) are not written.
- **Override test.** The unit tests check the lookup by name hash and the tier order on the set (`ui/fonts/set.rs`). The
  `UiAssets` path is not tested with a real install.
- **Console completion.** `devtools/console/parse.rs` (shared with another area) does not list `custom_fonts` for
  completion; `set custom_fonts` works through the generic `set` command.
