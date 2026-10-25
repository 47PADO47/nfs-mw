# Custom HUD: fonts from the operating system

The Custom HUD draws its numbers in a condensed face and its labels in a neutral one. Both are read from the fonts the
operating system already has. Nothing is bundled, downloaded or redistributed. This page records the search order, the
fallback and the tabular digits.

- **Sources read:** `crates/nfsmw/src/ui/raster_text.rs` (the rasterised text the Remastered and Custom themes draw),
  `ui/present/blackbox/mod.rs` (the presenter that draws it), and `ab_glyph` 0.2.32 (`FontVec`, `VariableFont`).
- **Built in:** `crates/nfsmw/src/ui/system_fonts/` (the finder) and `raster_text.rs` (the font keys, the looks and the
  tabular digits).
- **Status:** built. The keys exist and draw; the Custom components adopt them in their own PRs.

## 1. Why the system's fonts

- **No dependency.** The finder reads files with `std::fs` and parses them with `ab_glyph`, which the crate already
  uses. There is no fontconfig, DirectWrite or Core Text binding.
- **Nothing redistributed.** A font file is read locally at run time, like the game install. The repository and the
  release carry no font file and no font licence obligation.
- **The user's decision.** The design brief asks for a condensed face for numbers and a neutral face for labels, with no
  runtime download. The user chose to take them from the operating system.

## 2. Roles and font keys

| Role | Font key | Used for | Look in a system font | Look in the fallback |
|---|---|---|---|---|
| `Numeric` | `CUSTOM_NUMERIC_FONT_KEY` (`0x5241_535C`) | speed, gear, position, times | as drawn, tabular digits | widen 0.82, weight 0.5, tabular digits |
| `Label` | `CUSTOM_LABEL_FONT_KEY` (`0x5241_535D`) | secondary labels, notifications | as drawn | widen 0.88, weight 0.75 |
| `LabelWide` | `CUSTOM_LABEL_WIDE_FONT_KEY` (`0x5241_535E`) | spaced captions | tracking 0.06 | widen 0.95, weight 0.8, tracking 0.06 |

All three are upright. The keys follow the Remastered raster keys (`0x5241_5354` to `0x5241_535B`), which keep their
faces unchanged. `raster_text::Face` names its font as a `Source`: `Egui`, `EguiMono` or `System(Role)`.

## 3. Search order

Each role tries its candidates in order and takes the first file that reads and parses. A missing file, an unreadable
one or one that is not a font falls through to the next. The files are read once, the first time a Custom key is drawn
or measured. Each role keeps its own font instance, because a variation is set per instance.

### 3.1 Windows

The folder is `%WINDIR%\Fonts` (`C:\Windows\Fonts` when the variable is unset).

| Role | Candidates, first choice first |
|---|---|
| `Numeric` | `bahnschrift.ttf` at `wdth` 75, `wght` 600; `arialnb.ttf`; `arialn.ttf`; `segoeuib.ttf` |
| `Label` | `bahnschrift.ttf` at `wdth` 87.5, `wght` 400; `segoeui.ttf` |
| `LabelWide` | `bahnschrift.ttf` at `wdth` 100, `wght` 350; `segoeui.ttf` |

Bahnschrift ships with Windows 10 and later. It is a variable DIN face with a width axis (`wdth`, 75 to 100) and a
weight axis (`wght`, 300 to 700). The axes are set with `ab_glyph::VariableFont::set_variation`. Arial Narrow is not
part of every Windows install; it is used when present.

### 3.2 Linux

The roots are `/usr/share/fonts` and `~/.local/share/fonts` (from `$HOME`). Each root is walked four folder levels deep,
itself included, for exact file names. Within a folder the entries are read in sorted order, and a name found under an
earlier root wins. No fontconfig is used.

| Role | File names, first choice first |
|---|---|
| `Numeric` | `DejaVuSansCondensed-Bold.ttf`, `LiberationSansNarrow-Bold.ttf`, `NotoSans-CondensedSemiBold.ttf`, `NotoSans-Regular.ttf`, `DejaVuSans.ttf` |
| `Label` | `NotoSans-Regular.ttf`, `DejaVuSansCondensed.ttf`, `LiberationSans-Regular.ttf`, `DejaVuSans.ttf` |
| `LabelWide` | `NotoSans-Regular.ttf`, `LiberationSans-Regular.ttf`, `DejaVuSans.ttf` |

The order of preference is the role's, not the order the folders are listed in.

### 3.3 macOS

| Role | Candidates, first choice first |
|---|---|
| `Numeric` | `/System/Library/Fonts/Supplemental/DIN Condensed Bold.ttf`; `/System/Library/Fonts/Avenir Next Condensed.ttc` (face 0); `/System/Library/Fonts/Helvetica.ttc` (face 0) |
| `Label` | `Avenir Next Condensed.ttc` (face 0); `Helvetica.ttc` (face 0) |
| `LabelWide` | `Helvetica.ttc` (face 0) |

A collection (`.ttc`) is opened with `FontVec::try_from_vec_and_index`.

Other systems have no candidates and use the fallback.

## 4. Fallback

When no candidate of a role loads, the role draws in the bundled egui proportional font (Ubuntu Light, from
`epaint_default_fonts`). Its look is narrowed (`widen` below 1) to imitate a condensed face, and its coverage weight is
raised, since the bundled face is light (section 2). The choice is logged once per role at `info`: the file that
served the role, or that no system font loaded, with the paths tried.

## 5. Tabular digits

For the `Numeric` role every ASCII digit takes the advance of the `0`, so a number does not shift sideways as its digits
change. The width measure (`raster_text::width_units`) and the image (`raster_text::render`) apply the same rule, so a
right-aligned number ends on its edge. A digit narrower or wider than the `0` is centred in its cell. Other characters
keep their own advance. The rule holds in a system font and in the fallback.

## 6. Tests

- `ui/system_fonts/tests.rs`: the finder takes the first existing candidate, skips a file that is not a font, falls
  through to `None` when nothing exists, and loads a static font asked for variation axes. The Windows list starts with
  Bahnschrift and its axes per role. The Linux search orders the found files by preference, stops at its depth and
  prefers the first root. The bundled egui fonts stand in for system fonts, written to a temporary folder.
- `ui/raster_text_tests.rs`: the Custom keys name their system roles upright; the measured width of each Custom key
  matches its image; digits share the `0`'s advance in the measure and the image (bundled font); the fallback looks
  narrow the bundled font; the Remastered keys keep their faces.

## 7. Open points

- Whether `ab_glyph` applies Bahnschrift's horizontal metrics variations (`HVAR`) to the advances at `wdth` 75 is to be
  checked on a Windows machine. If it does not, the condensed digits keep the default width's spacing.
- The macOS face index of `Avenir Next Condensed.ttc` is taken as 0 without checking which weight that is.
