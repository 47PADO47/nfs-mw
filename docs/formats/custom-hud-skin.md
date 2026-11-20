# CustomHUD skin files (`CustomHud.ini`, `hud.ini` and the DDS atlases)

The third-party plugin `scripts/CustomHud.asi` replaces the game's speedometer, tachometer, nitrous, speed breaker and
boost gauges with gauges drawn from a **skin**: an ini file that places and colours each gauge, and the DDS pictures it
uses. This page is the file format. What the plugin does with it each frame (anchors, angles, masks, digits, which game
value feeds which gauge) is in [../specs/custom-hud-gauges.md](../specs/custom-hud-gauges.md). This is not the garage's
"gauge style" screen (`CustomHUD.fng`, `HUDS_Custom_*.bin`) and not the rewrite's own Custom HUD style
([../hud-settings.md](../hud-settings.md)); they only share a name.

Nothing from the plugin or from any skin is in this repository: the facts below were read from the plugin's machine code
in Ghidra (program `CustomHud.asi`, project `nfsmw`) and from the skin folders of the inspected install, and are written
in our own words.

Tags: **[asi]** read from the plugin's code (function addresses are image offsets of `CustomHud.asi`, base `0x10000000`);
**[verified]** checked on the skin files and the screenshot of this install; **[unverified]** could not be checked;
**[ours]** a decision of this port.

## 1. Where the files are

All paths are relative to the game folder (the folder of `speed.exe`) **[asi]** (the ini reader resolves a relative path
against the running executable, `FUN_10005160`).

| File | Role |
|---|---|
| `scripts/CustomHud.asi` | the plugin (328 KB, 2022 build here) |
| `scripts/CustomHud.ini` | the plugin's own settings (section 2) |
| `scripts/<HUDpath>/<IniName>` | the skin's ini; `HUDpath` and `IniName` come from `CustomHud.ini`. Textures are looked up in the **same folder** as that ini. |
| `CARS/<CAR>/CustomHUD/hud.ini` | an optional skin for one car; its textures sit beside it (section 5) |

The inspected install has twelve ready skins under `scripts/CustomHUD/` (NFS15 with two palettes, NFS2012_ALPHA,
NFS2022, NFSMW, NFSU1, NFSU2, NFS_HEAT, NFS_HEAT_UNITE3, NFS_HP2010, NFS_PB, NFS_RIVALS, NFS_WORLD) **[verified]**.

## 2. `CustomHud.ini`

Parsed once at start-up (`FUN_10008130`). A missing key reads as 0, 0.0 or empty, except the float defaults named in section 4 and `Scale` (1.0) **[asi]** (the float reader takes its default from a register the caller sets; the loaders were read key by key).

| Section | Key | Meaning |
|---|---|---|
| `[GENERAL]` | `HUDpath` | folder of the skin under `scripts/`, with its trailing backslash (`CustomHUD\NFS15\`) |
| | `IniName` | the skin's ini file name in that folder (`hud.ini`, `orange.ini`, `ug.ini`, ...) |
| | `HotReloadKey` | a Windows virtual-key code; 0 turns it off. A press (read with `GetAsyncKeyState`, the "pressed since the last call" bit) rebuilds the skin from disk. |
| | `ShowDebugInfo` | 0/1: draws seven lines of numbers (the rev reading, the red line, the speed, the nitrous, the speed breaker, the boost, the frame time) in the corner, in Arial 22 |
| | `CustomCarHUDs` | 0 off; 1 use `CARS/<CAR>/CustomHUD/hud.ini` when the player's car has one, else the general skin; 2 as 1, but a car without its own skin draws no custom HUD (the comment in the shipped file says "show vanilla hud for other cars") |
| | `Scale` | float, multiplies every size, position and offset (section 4 of the spec) |
| | `OffsetX`, `OffsetY` | float, pixels of the 1080-line reference, added to every position |
| `[CARBON]` | `ShakeAmount` | float: how far the gauges shake with the engine. Read for every game, but only the Carbon binding supplies a shake value; Most Wanted's is always 0 **[asi]** |

Numbers are read with the C library's integer and float parsers, so trailing text after a number is ignored (the shipped
file writes `CustomCarHUDs = 1 // 0 - disable, ...`). An integer that starts with `0x` is hexadecimal, anything else is
decimal; a value with no digits makes the plugin throw **[asi]** (`FUN_10005640`). Section and key names are matched
as written. Comment lines of the skins start with `#` (a line with no `=` is ignored); `//` after a value also appears
**[verified]**. The comment rules of the plugin's own reader were not read to the end **[unverified]**.

## 3. Types of value

- **Colour**: `0xAARRGGBB` (hex, 32 bit), as Direct3D reads it: the first byte is alpha. `0xFFFFFFFF` is opaque white,
  `0x3C000000` is black at 24 % alpha. A colour that is absent is 0: fully transparent **[asi]**.
- **Texture**: a file name in the skin's folder. An empty name means "none"; the picture is not drawn.
- **Blend mode**: 0 = normal alpha blending (source alpha, one minus source alpha); anything else = additive (source
  alpha, one) **[asi]** (`FUN_1000e330`, destination blend 6 or 4).
- **Size and positions**: pixels of a 1080-line screen, measured from the **right and bottom** edges (spec section 4).
- **Angle**: degrees (spec section 5).

## 4. The skin ini: sections and keys

Every element section has `Enabled` (0/1; default 0, so an absent section draws nothing). A disabled element reads no
other key. The element sections are `[BACKGROUND]`, `[TACHOMETER_GAUGE]`, `[TACHOMETER_DIGITAL]`, `[TACHOMETER_LINIAR]`,
`[SPEEDOMETER_GAUGE]`, `[SPEEDOMETER_DIGITAL]`, `[NOS_GAUGE]`, `[NOS_LINIAR]`, `[BOOST_GAUGE]`, `[SPEEDBREAK_GAUGE]`,
`[SPEEDBREAK_LINIAR]`, `[UNITS]`, `[GEAR]` and `[SHIFT_ICON]` **[asi]** (`FUN_10006f60`). The spelling `LINIAR` is the
plugin's.

### 4.1 Keys every element has

| Key | Type | Default | Meaning |
|---|---|---|---|
| `Enabled` | 0/1 | 0 | draw the element |
| `Size` | float | 0 | the element's side, in reference pixels |
| `PositionX`, `PositionY` | float | 0 | distance of the element's right and bottom edge from the screen's right and bottom edge |
| `Color` | colour | 0 | tint of the element's picture (digit and icon elements; a gauge uses the colours of 4.2) |

### 4.2 Gauges: `TACHOMETER_GAUGE`, `SPEEDOMETER_GAUGE`, `NOS_GAUGE`, `BOOST_GAUGE`, `SPEEDBREAK_GAUGE`

| Key | Type | Default | Meaning |
|---|---|---|---|
| `Value` | float | 0 | the value at the end of the scale (10 = 10 000 rpm for the tachometer, 1 for the nitrous and the speed breaker, 20 for the boost). A scale of 0 gives a non-finite angle. |
| `ArrowMinAngle`, `ArrowMaxAngle` | float | 0 | the needle's angle at 0 and at `Value` (spec section 5) |
| `CanBeNegative` | 0/1 | **1** | 0 clamps the needle and the fill at 0; 1 lets a negative value turn it below `ArrowMinAngle` (the boost gauge, in vacuum) |
| `HideOnMaxValue` | 0/1 | 0 | 1 fades the gauge out while its value is at or above `Value` (or exactly 0) |
| `BackgroundTexture`, `BackgroundColor`, `BackgroundTextureBlendMode` | | none, 0, 0 | the dial's backing picture, drawn at the full element size |
| `NumbersTexture`, `NumbersColor`, `NumbersTextureBlendMode` | | none, 0, 0 | the scale's numbers (a second full-size picture over the backing) |
| `NumbersMaxColor`, `NumbersMaxThreshold` | colour, float | 0, **-1** | colour of the numbers near the red line: it replaces `NumbersColor` while the red line is less than `NumbersMaxThreshold` above the value (thousands of rpm); a threshold of 0 or less never switches |
| `NumbersMaskedTexture`, `NumbersMaskedTextureBlendMode` | | none, 0 | a third picture (usually the same file as `NumbersTexture`), drawn through a sector mask: the red zone's numbers |
| `NumbersMaskedColor`, `NumbersMaskedColor1`, `NumbersMaskedColor2` | colour | 0 | tint of the masked picture; the mask's value outside and inside the sector (only the **alpha byte** of Color1 and Color2 is used) |
| `NumbersMaskedSize` | float | **128** | side of the mask bitmap, in pixels (the mask is stretched over the element) |
| `NumbersMaskedMaxOffset` | float | 0 | degrees added to the end of the red zone's sector |
| `ArrowTexture`, `ArrowTextureBlendMode` | | none, 0 | the needle's picture |
| `ArrowColor` | colour | 0 | needle tint |
| `ArrowPerfectZoneColor` | colour | 0 | needle tint while the game reports the "perfect zone" flag; 0 turns the feature off |
| `ArrowScale` | float | **1** | scales the needle's width and height (1.0 is the normal size) |
| `ArrowCenterOffset` | float | **0.22** | where the needle turns: 0 = at its right end, 1 = at its left end, values above 1 count as 1. The default turns it at 78 % of its width. |
| `ArrowMaskedTexture`, `ArrowMaskedTextureBlendMode` | | none, 0 | the "filled" picture, drawn full element size through a sector mask: the lit part of the arc |
| `ArrowMaskedSize` | float | **128** | side of the arc mask bitmap, in pixels |
| `ArrowMaskedColor`, `ArrowMaskedMaxColor`, `ArrowMaskedMaxThreshold` | colour, colour, float | 0, 0, 0 | tint of the filled picture, the tint near the red line, and the distance from the red line (thousands of rpm) under which the second tint applies; a threshold of 0 or less never switches |
| `ArrowMaskedColor1`, `ArrowMaskedColor2` | colour | 0 | the arc mask's value outside and inside the lit sector (alpha byte only) |
| `ArrowMaskedMinOffset` | float | 0 | degrees the lit sector starts before the scale's start |

Keys that some skins write but the plugin build in this install does not read (it ignores them): `ArrowMaxThreshold`,
`ArrowMaskedPerfectZoneColor`, `BackgroundMaskedSize`, `FilledPerfectZoneColor` and the whole
`[NOS_COUNT]` section **[asi]** (their strings are absent from the 2022 binary; they belong to a newer plugin version)
**[verified]**.

### 4.3 Digital readouts: `SPEEDOMETER_DIGITAL`, `TACHOMETER_DIGITAL`, `GEAR`

Common keys: `Enabled`, `Size`, `PositionX`, `PositionY`, `Color`, `DigitsTexture`, `DigitsTextureBlendMode`,
`BackgroundColor` (colour of a ghost "8" drawn behind each digit; 0 draws none).

`SPEEDOMETER_DIGITAL` and `TACHOMETER_DIGITAL` add `Distance` (float: added to `Size` to get the step from one digit to
the next; negative values pack the digits), `DigitsMode` (0, 1 or 2; spec section 7), and for the speed `DigitsTexture1`
and `DigitsTexture2` (a different picture for the tens and the hundreds digit; empty keeps `DigitsTexture`). The tachometer
version reads the same keys but never uses the two extra textures.

### 4.4 `UNITS`, `SHIFT_ICON`, `BACKGROUND`

- `UNITS`: `UnitsTexture`, `UnitsTextureBlendMode`, plus the common keys. The picture holds two cells side by side.
- `SHIFT_ICON`: `IconTexture`, `IconTextureBlendMode`, `BlinkMode` (0 steady, 1 on/off blink, 2 pulse), plus the common
  keys.
- `BACKGROUND`: `Texture`, `TextureBlendMode`, plus the common keys.

### 4.5 Bars: `TACHOMETER_LINIAR`, `NOS_LINIAR`, `SPEEDBREAK_LINIAR`

Common keys, then `Value` (the full-scale value), `Direction` (0 fills from the left, 1 from the bottom, 2 from the
right), `FilledTexture`, `FilledColor`, `FilledTextureBlendMode`, `BackgroundTexture`, `BackgroundColor`,
`BackgroundTextureBlendMode`.

## 5. Per-car skins

With `CustomCarHUDs` 1 or 2 the plugin asks the game for the player's car name (a string such as `CORVETTE`, the folder
name of the car, read through `[[0x92C510] + 0x10] + 0x1C` in `speed.exe` **[asi]**, `FUN_10015250`). When the name
changes it checks whether `CARS\<name>\CustomHUD\hud.ini` exists. If it does, that file is the skin and its folder holds
the textures; the file is always called `hud.ini` there. If it does not, mode 1 falls back to the general skin and mode 2
shows no custom HUD for that car. The check runs again on every car change and rebuilds the skin.

## 6. The pictures

All pictures are DDS files, loaded with Direct3D's file loader (`D3DXCreateTextureFromFileA`), so every pixel format
D3DX reads works. The twelve inspected skins use only four (111 files DXT5, 21 DXT1, 1 DXT3, 8 uncompressed A8R8G8B8,
all 2D, most with a full mip chain) **[verified]**; this port decodes those four (spec section 12). A texture is cached by
path (a hash of the full path), so two elements that name one file share it **[asi]** (`FUN_1000e5e0`).

### 6.1 Atlas layouts

| Picture | Layout | Evidence |
|---|---|---|
| digit atlas (`digits.dds`, `DigitsTexture`) | **12 equal cells in one row**; cell `n` (counted from 0) holds the digit `n` for `n` 0 to 9. Cells 10 and 11 are not read for the speed and the rpm. One cell is `width/12` wide and the full height tall (for example 1368 x 114: cells of 114 x 114). | **[asi]** `FUN_1000fca0` (divides the width by 12), digit functions return digit + 1 and the draw subtracts 1 |
| gear atlas (`gear.dds`, `DigitsTexture` of `[GEAR]`) | the same 12 cells: cell 1 to 9 the gears 1 to 9, **cell 10 = N, cell 11 = R**; cell 0 is not drawn | **[asi]** `FUN_10004020` (neutral gives glyph 11, reverse 12) and **[verified]** (1584 x 128: 132-pixel cells) |
| units (`speed-units.dds`) | **two cells side by side**: the left half is MPH, the right half is KM/H; each cell is drawn at 2 x `Size` wide, `Size` tall | **[asi]** `FUN_100113a0` (half width selected by the metric flag) |
| gauge pictures | one square-ish picture stretched over the `Size` x `Size` square; the aspect of the file does not matter | **[asi]** |
| needle | any aspect; its width is `Size / 1.4`, its height `Size / 2.8`, times `ArrowScale`, so a 2:1 picture keeps its shape; it points to the **left** at angle 0 | **[asi]** `FUN_1000f7e0` |

### 6.2 Masked pictures

The "masked" pictures (`ArrowMasked*`, `NumbersMasked*`) are drawn through a second, generated bitmap, not through a
second file: see the spec's section 8.
