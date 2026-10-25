# Custom HUD: typography and the OS fonts

Which faces the Custom HUD and menus draw in, where they come from, what happens when they are missing, and how numbers
keep a stable width. Built in PR 5 (`crates/nfsmw/src/ui/system_fonts.rs` and new raster keys in
`ui/raster_text.rs`). Index: [README.md](README.md).

## 1. Roles

The brief asks for a modern condensed sans-serif for numbers and a neutral sans-serif for labels. Three raster font keys
serve three roles:

| Key (PR 5) | Role | Used for |
|---|---|---|
| `CUSTOM_NUMERIC_FONT_KEY` | condensed, semi-bold | speed, gear, position, lap and checkpoint counts, timers, distances, gaps |
| `CUSTOM_LABEL_FONT_KEY` | neutral regular | notification text, secondary labels, menu rows, settings |
| `CUSTOM_LABEL_WIDE_FONT_KEY` | regular width, semi-bold, tracked | short uppercase headings: `POS`, `LAP`, `HEAT`, menu titles |

The keys sit beside the existing `RASTER_*_FONT_KEY` constants and are drawn by the same raster path the presenter uses
for the Remastered text (`ui/raster_text.rs`, `ui/present/blackbox.rs`). No FEng bitmap font is used by Custom.

**Hierarchy** at scale 1, in HUD units (the 480-unit-high canvas):

| Element | Key | Size |
|---|---|---|
| Speed digits | numeric | 37 |
| Gear | numeric | 19 |
| Race position (`2`) | numeric | 30 |
| Strip values (lap, distance, gap) | numeric | 16 |
| Speed unit, headings | label wide | 10 |
| Notification title | label | 14 |
| Notification secondary line, chips | label | 11 |
| Menu rows | label | 16 |
| Menu titles | label wide | 22 |

Long messages are sentence case; uppercase is kept for headings of one or two words, as the brief asks. **[ours]**

## 2. Why OS fonts

The owner's decision ([ADR 0013](../../decisions/0013-custom-hud.md), decision 2):

- **No new dependency.** `ab_glyph` 0.2.32 is already a dependency of `crates/nfsmw`; its default features include
  `variable-fonts`, so `ab_glyph::VariableFont::set_variation` is available on `FontVec`, and
  `FontVec::try_from_vec_and_index` reads one face of a `.ttc` collection. No font-discovery crate is added: a short
  list of known paths is enough.
- **Nothing redistributed.** No font file enters the repository (its `.gitignore` is a whitelist and the leak check
  refuses binaries). The face is read at run time from the player's own machine, the same way the game install is read.
- **No download at run time,** as the brief requires.

## 3. The search order

The finder runs once, at start-up, off the frame path, and caches the result for the run. It tries each candidate in
order and takes the first file that exists and parses. Each role has its own list; the numeric and label roles may
resolve to the same file with different axis settings.

### 3.1 Windows

The fonts folder is `%WINDIR%\Fonts` (`C:\Windows\Fonts` when `WINDIR` is unset), then the per-user folder
`%LOCALAPPDATA%\Microsoft\Windows\Fonts`.

| Role | Candidates, in order |
|---|---|
| numeric | `bahnschrift.ttf` with `wdth` = 75, `wght` = 600 |
| label | `bahnschrift.ttf` with `wdth` = 100, `wght` = 400; then `segoeui.ttf` |
| label wide | `bahnschrift.ttf` with `wdth` = 100, `wght` = 600; then `segoeuib.ttf`; then `segoeui.ttf` |

Bahnschrift is a variable DIN-style face shipped with Windows 10 (1709 and later) and 11. Its axes, read from the
`fvar` table of the development machine's copy: **[verified]**

| Axis | Minimum | Default | Maximum |
|---|---|---|---|
| `wght` | 300 | 400 | 700 |
| `wdth` | 75 | 100 | 100 |

The axes are set with `VariableFont::set_variation(b"wdth", ..)` and `(b"wght", ..)` on a `FontVec` before it is wrapped
in a `FontArc`; each role loads its own `FontVec`. A `set_variation` that returns `false` (an older, static
`bahnschrift.ttf`) is logged once and the face is used at its default instance.

### 3.2 Linux

Searched folders: `/usr/share/fonts/`, `/usr/local/share/fonts/`, `$XDG_DATA_HOME/fonts/` (default
`~/.local/share/fonts/`) and `~/.fonts/`, each recursively to a depth of 4, matching file names exactly.

| Role | Candidates, in order |
|---|---|
| numeric | `DejaVuSansCondensed-Bold.ttf`, `LiberationSansNarrow-Bold.ttf`, `NotoSans-CondensedSemiBold.ttf`, `NotoSans-Bold.ttf` |
| label | `DejaVuSansCondensed.ttf`, `LiberationSansNarrow-Regular.ttf`, `NotoSans-Regular.ttf` |
| label wide | `DejaVuSans-Bold.ttf`, `LiberationSans-Bold.ttf`, `NotoSans-SemiBold.ttf`, `NotoSans-Bold.ttf` |

Distributions package these under different subfolders (`truetype/dejavu/`, `TTF/`, `liberation/`, `noto/`), which is
why the search is by file name and not by full path.

### 3.3 macOS

| Role | Candidates, in order |
|---|---|
| numeric | `/System/Library/Fonts/Supplemental/DIN Condensed Bold.ttf`; then `/System/Library/Fonts/Avenir Next Condensed.ttc` (a demi-bold face) |
| label | `/System/Library/Fonts/Avenir Next Condensed.ttc` (the regular face); then `/System/Library/Fonts/Helvetica.ttc` (index 0) |
| label wide | `/System/Library/Fonts/Supplemental/DIN Alternate Bold.ttf`; then `/System/Library/Fonts/Helvetica.ttc` (index 0) |

A `.ttc` is loaded with `FontVec::try_from_vec_and_index`. The face indices inside `Avenir Next Condensed.ttc` are to
be read on a Mac in PR 5 and written here. **[unconfirmed]**

### 3.4 Injected paths for tests

The finder takes its candidate list and a file-exists probe as arguments, so the fallback order is unit-tested with
paths that do not exist and with fake files (PR 5), on any platform.

## 4. The fallback

When no candidate of a role is found, that role uses the face the rewrite already embeds: egui's Ubuntu Light from
`epaint_default_fonts` (`ui/raster_text.rs`, the Remastered text's face). Ubuntu Light is a light, regular-width face,
so the fallback imitates a condensed numeric face through the existing `Look`:

| Role | Fallback `Look` |
|---|---|
| numeric | `widen` 0.86, `weight` 0.45, `tracking` 0.0 |
| label | `widen` 1.0, `weight` 0.7, `tracking` 0.0 |
| label wide | `widen` 1.0, `weight` 0.45, `tracking` 0.08 |

(`weight` below 1 thickens the coverage, as for the existing `NUMBERS` look.) The fallback is logged once per role at
info level, naming the role and the paths tried, and recorded in this section as the documented behaviour. It is not an
error: the HUD is complete without an OS font.

**High contrast** raises the minimum weight one step: a role whose `Look.weight` is above 0.45 uses 0.45, and the
Bahnschrift roles use `wght` 700 for numerals and 500 for labels.

## 5. Tabular numerals

Numbers must not jitter as they change. Each glyph of a numeric string is laid out in a cell as wide as the face's `0`
advance, and centred in that cell. This applies to the digits `0` to `9`; the separators `.`, `:`, `,`, `+`, `-` and
the space keep their own advance. The rule is applied by the raster layout for the numeric key, whatever the face does,
so it also holds on the fallback and on faces without tabular figures. Unit tests (PR 5): every digit string of the same
length has the same width, and `1` is centred in its cell.

Right-aligned numbers (the speed digits, the strip's distance) are aligned on their last cell, so the unit or label
next to them does not move.

## 6. Text size and wrapping

`[custom_hud] text_size` scales every glyph size above. Text in a notification wraps at the plate's width (two lines at most,
then an ellipsis). Numeric fields never wrap; a field too narrow for its value at a large `text_size` drops its unit
label first, then the field elides. Menus follow the same rule. **[ours]**

## 7. Localisation

All player-visible strings come from the Fluent catalogue (`crates/nfsmw/i18n/en/hud.ftl` and siblings,
[localization.md](../localization.md)). The OS faces' character coverage differs by platform and version. What a
string with a character the chosen face lacks does (fall back per glyph, or draw the whole string in the fallback face)
is decided in PR 5 and written here. **[unconfirmed]** Text is not shaped, as
[ADR 0012](../../decisions/0012-localization.md) records.
