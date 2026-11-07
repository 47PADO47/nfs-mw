# CustomHUD gauges: what the plugin draws, and how this port draws it

Spec of the behaviour of the third-party plugin `scripts/CustomHud.asi` (the skins it draws: [../formats/custom-hud-skin.md](../formats/custom-hud-skin.md)), and of this port's reimplementation. The plugin is read from its machine code in Ghidra
(program `CustomHud.asi` of project `nfsmw`, 2022 build, 328 KB). Function names below are image offsets (base
`0x10000000`). Everything is described, nothing is copied. It is **not** the garage's gauge-style screen and **not** the
rewrite's Custom HUD style.

Tags: **[asi]** read from the plugin's code; **[verified]** checked on the skins and the screenshot of this install;
**[inferred]** a reading of the code that was not run; **[unverified]** not checked; **[ours]** a decision of this port.

## 1. What it is

An overlay that Direct3D draws at the end of each frame with `ID3DXSprite`. It hides the game's own gauges by clearing
some of the HUD feature flags the game returns (section 2) and draws its own: a background picture, the tachometer (dial,
numbers, needle, filled arc, red-zone numbers), a digital rev counter, the speed digits and unit, the gear letter, the
nitrous, speed breaker and boost gauges, bar gauges, a shift light. Every picture is a DDS file named by the skin.

## 2. Life cycle (Most Wanted 1.3)

- **Game detection** **[asi]** (`FUN_10003970`): the plugin reads the entry point of the running executable. `0x3c4040`
  is Most Wanted 1.3 (`speed.exe`); other values select the Carbon, Undercover, Underground and Underground 2 bindings
  (each its own table of game addresses). An unknown value shows "This .exe is not supported" and does nothing.
- **Hooks** **[asi]**: a call in the game's HUD code is redirected to a wrapper (`FUN_10014c60`) that calls the game's own
  function `speed.exe+0x57CA60` and then clears five bits (`0x2`, `0x800`, `0x20000`, `0x80000`, `0x08000000`) of the flags it
  returns, so the stock gauges are not drawn. It remembers whether bit `0x800` (the car has nitrous) and bit `0x20000`
  (the car has forced induction) were set, and whether bit 2 was set and `speed.exe+0x5A0C20` agreed (the "custom HUD is
  on" flag). When `speed.exe+0x4D6150` is true (a state the plugin leaves alone) the flags pass through untouched
  **[inferred]**: the stock HUD stays. A second hook is the draw callback below.
- **Drawing** **[asi]** (`FUN_10002420`, `FUN_10012240`): a worker waits until the game's device pointer
  (`speed.exe+0x982BDC`) is filled, then registers the draw function with the MirrorHook Direct3D 9 extension, and a
  device-reset callback that frees everything. The draw function runs every frame and returns at once unless the game's
  "HUD is shown" flag is set (the "custom HUD is on" flag above and the byte at `speed.exe+0x92FD94`).
- **Skin choice** (section 2 of the format page): once at start-up, then again when the player's car name changes, and on
  the hot-reload key.
- **Frame time** **[asi]**: the fade and blink timers read a global frame time that the plugin only updates inside the
  `ShowDebugInfo` block **[inferred]**: with debug off it stays 0, so a `HideOnMaxValue` gauge keeps its start state and a
  blinking shift light never blinks. We use the real frame time **[ours]**.

## 3. The game values it reads (Most Wanted binding)

The plugin reads the game's own HUD data through a table of getters (`Game::MostWanted`, vtable `0x10047AB4`). Each is a
pointer chain from `speed.exe+0x92FD98` (the player's HUD object `H`) **[asi]**:

| Getter | Chain from `[0x92FD98]` | Meaning | Used for |
|---|---|---|---|
| slot 9 (`FUN_10014F60`) | `[H+0x2C0]+0x48` times 3.6 (km/h) or 2.23699 (mph) | speed in the shown unit | speed digits, speedometer gauge |
| slot 11 (`FUN_10015020`) | `[H+0x2C4]+0x54` | engine speed, rpm | tachometer, rev digits |
| slot 12 (`FUN_10015090`) | `[H+0x2C4]+0x58` | red line, rpm | red-zone, shift light |
| slot 13 (`FUN_10015100`) | `[H+0x2C4]+0x60` (int) | gear: 0 reverse, 1 neutral, 2 first, ... | gear letter |
| slot 5 (`FUN_10014E70`) | `[H+0x2E4]+0x38` | nitrous left, 0 to 1 | nitrous gauge and bar |
| slot 8 (`FUN_10014EF0`) | `[H+0x2E8]+0x38` | speed breaker meter, 0 to 1 | speed breaker gauge and bar |
| slot 3 (`FUN_10014DF0`) | `[H+0x2DC]+0x3C` | boost | boost gauge |
| slot 15 (`FUN_100151E0`) | `[H+0x2C4]+0x65` (byte) | the tachometer's "perfect zone" flag | needle's `ArrowPerfectZoneColor` |
| slot 14 (`FUN_10015170`) | `[[0x91CF90]+0x10]+0x3B` (byte) | the profile's metric flag (true = km/h) | units picture, speed factor |
| slot 4 / slot 6 | flags from the hook | the car has forced induction / nitrous | boost / nitrous gauge visible |
| slot 2 | constant 0.0 | engine shake (Most Wanted has none) | shake |
| slot 16 (`FUN_10015250`) | `[[[0x92C510]+0x10]+0x1C]` string | the car's name | per-car skins |

The bindings of each gauge to these getters, as set up when the HUD is built (`FUN_10010ae0`) **[asi]**:

| Gauge | value (`cb124`) | red line (`cb128`, `cb12C`) | `cb130` | fill start (`cb134`) | fill end (`cb138`) | visible (`cb13C`) | perfect zone (`cb140`) |
|---|---|---|---|---|---|---|---|
| tachometer | rpm / 1000 | red line / 1000 | the ini's `Value` | 0 | rpm / 1000 | always | slot 15 |
| speedometer | speed, at most 999 | none | none | none | none | always | none |
| nitrous | slot 5 | none | none | 0 | slot 5 | has nitrous | none |
| speed breaker | slot 8 | none | none | 0 | slot 8 | always | none |
| boost | slot 3, clamped to -20..20 | none | none | 0 | slot 3 (same clamp) | has forced induction | none |

`cb128` is the "maximum" the near-red-line colour tests against; when it is absent the test uses the value itself.

## 4. Placement and scale **[asi]** (`FUN_1000e7a0`)

All sizes and positions are pixels of a **1080-line reference screen**. For the window's viewport (width `W`, height `H`
pixels) the unit is `s = (H / 1080) * Scale`, `Scale` from `CustomHud.ini`.

- **Anchor: the right and bottom edge of the viewport.** A sprite of reference size `w` x `h`, with `PositionX`
  `px` and `PositionY` `py`, has its **right edge** `(px + OffsetX) * s` pixels from the viewport's right edge and its
  **bottom edge** `(py + OffsetY) * s` pixels above the bottom edge. So the NFS15 tachometer (`Size` 330, `PositionX`
  30, `PositionY` 60) stands in the bottom-right corner, and the boost, nitrous and speed breaker gauges at
  `PositionX` 345 stand to its left. This is why the screenshot of this install shows the dial at the right and the
  small gauges at the left **[verified]**. Wider windows keep the cluster in the corner.
- A square element is `Size` x `Size` reference pixels, scaled by `s`. The picture is stretched to that square.
- A digit cell is `Size` tall and `Size * cellWidth / pictureHeight` wide (`cellWidth` = picture width / 12).
- `Scale` and the offsets scale together: `OffsetX`, `OffsetY` are reference pixels.
- `ShakeAmount` shifts everything by `shake * s * ShakeAmount` on both axes when the game's shake getter is non-zero; for
  Most Wanted it is 0, so there is no shake.

## 5. Needle angles **[asi]** (`FUN_1000f7e0`)

- The angle is **degrees clockwise on the screen, from the direction "left"**: 0 points left, 90 up, 180 right, 270
  down. (The needle's picture is drawn pointing left and turned with Direct3D's 2-D transform about its pivot, whose
  positive angle is clockwise in screen coordinates.) With `ArrowMinAngle` -30 and `ArrowMaxAngle` 210 the needle rests
  at the lower left (7 o'clock), sweeps over the top and ends at the lower right (4 o'clock) **[verified]** (the
  screenshot: "0" at the lower left, "10" at the lower right).
- `angle = ArrowMinAngle + (ArrowMaxAngle - ArrowMinAngle) / Value * v`, where `v = min(value, Value)`; unless
  `CanBeNegative` is 1, `v` is also floored at 0. For the tachometer `Value` is in thousands of rpm.
- The needle's picture is `Size / 1.4 * ArrowScale` wide and `Size / 2.8 * ArrowScale` tall (`ArrowScale` defaults to 1). Its
  pivot is the point at `1 - min(ArrowCenterOffset, 1)` of its width (the default offset is 0.22: 78 % across) and half its height. The pivot sits on the
  element's centre (`PositionX + Size / 2`, `PositionY + Size / 2` from the corner). The tint is `ArrowColor`, or
  `ArrowPerfectZoneColor` while that is non-zero and the gauge's perfect-zone getter is true.
- Every colour drawn with a fade (section 6) has its alpha byte multiplied by the fade.

## 6. The gauge's layers, back to front **[asi]** (`FUN_1000f110`, `FUN_1000f620`, `FUN_1000f7e0`)

A gauge whose visible getter returns false draws nothing. Otherwise:

1. `HideOnMaxValue`: when set, a fade alpha moves toward 0 at 3 per second while the value is at or above `Value` or
   exactly 0, and toward 1 otherwise; at 1 it multiplies nothing. (The fade starts at 0 when the key is set.) A set key
   with a `Value` other than 1 compares the fade with `Value` and so always scales **[inferred]**; the shipped skins that
   use it have `Value` = 1.
2. **Background**: `BackgroundTexture` over the square, tinted `BackgroundColor`.
3. **Numbers**: `NumbersTexture`, tinted `NumbersColor`. Its colour is `NumbersMaxColor` while `NumbersMaxThreshold` is
   above 0 and the red line (thousands of rpm) is less than `NumbersMaxThreshold` above the value; otherwise
   `NumbersColor`.
4. **Filled arc** (only when `ArrowMaskedTexture` is set and the gauge has a fill getter): `ArrowMaskedTexture` through a
   sector mask (section 8) from `cb134 * k + ArrowMinAngle - ArrowMaskedMinOffset` to the angle of `cb138` (clamped to
   `Value`, floored at 0 unless `CanBeNegative`), with `k = (max - min) / Value`; the two ends are put in order. Tint:
   `ArrowMaskedColor`, or `ArrowMaskedMaxColor` while `ArrowMaskedMaxThreshold` is above 0 and the red line is less
   than that above the value. The mask's inside value is `ArrowMaskedColor2` and its outside value
   `ArrowMaskedColor1`.
5. **Red-zone numbers** (only when `NumbersMaskedTexture` is set and the gauge has `cb12C`): `NumbersMaskedTexture` through
   a sector mask from the angle of `cb12C` (the red line) to the angle of `cb130` (`Value`) **plus
   `NumbersMaskedMaxOffset` degrees**, tint `NumbersMaskedColor`, inside `NumbersMaskedColor2`, outside
   `NumbersMaskedColor1`. For the tachometer this lights the numbers from the red line to the end in the red-zone
   colour.
6. **Needle**: drawn after all other elements of the gauge. For the tachometer and the speedometer the needle is drawn
   **last of the whole overlay**; the boost, nitrous and speed breaker needles are drawn right after their own layers.

## 7. Digits **[asi]** (`FUN_10010000`, `FUN_1000fca0`, `FUN_100113a0`)

A digit cell comes from the 12-cell atlas of the format page. Digit `d` shows cell `d`. Each digit has a `Size`-tall box;
the box's right edge is `PositionX + slot * (Size + Distance)` from the right edge (slot 0 is the rightmost digit), and
digits are right to left: slot 0 ones, slot 1 tens, slot 2 hundreds (slot 3 thousands for the tachometer).

**Speed** (`SPEEDOMETER_DIGITAL`): the value is the speed in the shown unit, cut to a whole number, at most 999. Three
slots. Leading zeros are not drawn (the ones digit always is). `DigitsMode`:

| Mode | Meaning |
|---|---|
| 0 | the three slots sit at fixed positions (`PositionX`, `+ (Size + Distance)`, `+ 2 (Size + Distance)`); a ghost "8" (when `BackgroundColor` is non-zero) is drawn in all three |
| 1 | the same positions; ghosts only under the digits shown |
| 2 | centred: the digits shift toward the middle by half a step per missing digit, `(3 - n) * (Size + Distance) / 2` further from the corner, so 7 and 42 and 130 stand around one centre |

`DigitsTexture1` replaces the picture for the tens, `DigitsTexture2` for the hundreds. The colour is `Color`, the ghost
`BackgroundColor`.

**Rev counter** (`TACHOMETER_DIGITAL`): four slots from the rpm as a whole number: with `DigitsMode` 1 only as many as it
has digits (at most 4; above 9999 nothing), otherwise always four, with leading zeros. Same spacing, `Color` and ghost.

**Gear** (`GEAR`): one cell of the gear atlas at `PositionX`, `PositionY`. The game's gear number is 0 reverse, 1 neutral,
2 first, ...; the cell is 11 (R) for reverse, 10 (N) for neutral, and `gear - 1` for gear 2 and up (so first gear is cell 1).
A gear number above 12 or below 0 draws nothing. The ghost applies as for digits. In this port's numbering (`-1` R, `0` N,
`n` forward) the cell is 11, 10 and `n`.

**Units** (`UNITS`): the left half of `UnitsTexture` for miles per hour, the right half for kilometres per hour, each half
`2 * Size` wide and `Size` tall.

## 8. Sector masks **[asi]** (`FUN_1000e9f0`, `FUN_1000eac0`, `FUN_1000e330`)

A masked picture is a normal sprite drawn with two textures: the picture and a square A8R8G8B8 bitmap (`ArrowMaskedSize` or
`NumbersMaskedSize` pixels a side) the plugin fills itself. The bitmap's alpha multiplies the picture's alpha; its colour
is ignored (the colour operation selects the picture's colour). So the lit part uses the **alpha byte of Color2**, the rest
the alpha byte of Color1.

- The sector is the set of bitmap pixels whose direction from the bitmap's centre lies between two angles, turning the
  same way as the needle (the angles are the section 5 angles; a direction at angle `a` is `(cos(a - 180), sin(a - 180))`
  in screen coordinates, so 0 is left, 90 is up). A sector wider than 180 degrees is the union of the two half-planes the
  edges cut, which is the arc the short way round the other direction; in all cases the sector is the clockwise sweep
  from the first to the second angle after they are put in order.
- The bitmap is rebuilt only when the end angle moved by more than 0.1 degree since the last build (and the first time).
- With `Color1` 0x3C000000 and `Color2` 0xFFFFFFFF (the NFS15 skin) the unfilled part of the arc stays visible at 24 % and
  the filled part at 100 % of the picture's alpha **[verified]** (the screenshot shows the small gauges with only a
  short bright arc).

## 9. Other elements

- **Background** (`FUN_100113a0`): drawn first; `Texture` over the `Size` square, tint `Color`.
- **Bars** (`FUN_100106e0`): the background picture over the square; the filled picture shows the fraction
  `f = value / Value` of its width (`Direction` 0: left part, drawn from the left edge), its height (`Direction` 1:
  bottom part, from the bottom up) or its width from the right (`Direction` 2), tint `FilledColor`. Values: the rev
  reading / 1000, the nitrous, the speed breaker.
- **Shift light** (`FUN_10010930`): shown while `rpm / 1000 + 0.5` is at least `redline / 1000`. `BlinkMode` 0 steady;
  1 a timer that runs 0 to 1 s and shows the icon while above 0.5; 2 a timer 0 to 0.5 s folded at 0.25 and used as the
  alpha `2 t * 256` (a pulse that peaks at about half alpha). The timer resets when the light goes out. The icon fills
  the `Size` square, tint `Color`.

## 10. Order of drawing **[asi]** (`FUN_100113a0`)

Background; tachometer gauge (layers 2 to 5); rev digits; tachometer bar; speedometer gauge (2 to 5); speed digits;
boost gauge with its needle; nitrous gauge with its needle; nitrous bar; speed breaker gauge with its needle; speed
breaker bar; gear; units; shift light; tachometer needle; speedometer needle.

## 11. This port

- **Detection [ours]**: the skin is used when **both** `scripts/CustomHud.asi` and `scripts/CustomHud.ini` exist in the
  game folder (looked up case-insensitively through the game directory layer). A player who does not have the mod sees
  the stock gauges; a stray ini without the plugin does nothing. Reading goes through the game folder only, never a
  user-global path.
- **Setting [ours]**: `custom_hud_skin = "auto" | "off"` (default `auto`), in `config.toml` (and the environment variable
  `NFSMW_CUSTOM_HUD_SKIN`), as a row of the Gameplay options page ("CustomHUD skin"). `off` always draws the stock gauges.
  The mod's presence is the only other condition, so there is no `on`.
- **Where it applies [ours]**: only with the **Original** HUD style (`ui_style = "original"`), which is the look the plugin
  replaces. The Custom and Remastered styles draw their own clusters and ignore the skin. A drag race
  (`HUD_Drag.fng`) keeps the stock gauges.
- **What is hidden [ours]**: the stock tachometer, speedometer, gear, nitrous, turbo and speed breaker objects of
  `HUD_SingleRace.fng` (their roots are in `hud/roots.rs`) are hidden whenever a skin is drawn, so no gauge draws twice.
  The rest of the HUD (minimap, pursuit, race readouts) is untouched.
- **Skin choice [ours]**: `[GENERAL] CustomCarHUDs` is honoured: with 1 or 2 the player's car folder name selects
  `CARS/<car>/CustomHUD/hud.ini` when the file exists. Mode 2 draws the stock gauges for a car without a skin of its own,
  which is what its comment promises. Bikes use the same path: their name is the folder name.
- **Not copied [ours]**: `HotReloadKey` (the skin is read when the car or the setting changes, and the files are not
  watched), `ShowDebugInfo`, and the Carbon `ShakeAmount` (Most Wanted has none).
- **Values [ours]**: tachometer value `rpm / 1000`, red line `red_line / 1000` and the `Value` of the ini; speed in the shown
  unit (the port's speed unit setting, not the profile flag); nitrous `nos`, visible only with a nitrous system; speed
  breaker `speedbreaker`; boost `boost_psi` clamped to -20..20, visible only with forced induction; the gear is `-1` R,
  `0` N, `1..9` the gear; the "perfect zone" flag is the standing-launch range (`perfect_launch_range`), the nearest
  existing signal to a "perfect launch" **[inferred]**.
- **Differences from the plugin [ours]**: the frame time is real (section 2); a sector mask is built at the picture's own size
  (at most 128 pixels for a moving arc, 512 for a static red zone) and stepped every 1.5 degrees; a DXT1 picture keeps its
  one-bit alpha; the "custom HUD is on" flags of the game are replaced by the HUD being visible.
- **Code**: the parser, the geometry and the masks are the engine-generic library `libs/blackbox-gauge-skin`; the overlay,
  the telemetry binding, the detection and the hiding of the stock gauges are `crates/nfsmw/src/hud/asi_skin/`. Tests have
  no game data: they use ini text written for them.

## 12. Pictures this port cannot decode

None of the 141 pictures of the twelve skins on this machine: they are DXT1, DXT3, DXT5 and A8R8G8B8 **[verified]**, which
the port decodes. A skin with another DDS format (a palettised or 16-bit picture, a cube or volume map) draws without
that picture and logs one line naming the file.

## 13. Open

| Question | Note |
|---|---|
| Comment rules of the plugin's ini reader | The shipped files use `#` and trailing `//`; the reader's code was not read to the end |
| The real meaning of the byte at `[H+0x2C4]+0x65` | taken as the "perfect zone" flag from the name `ArrowPerfectZoneColor` and its place in the tachometer object |
| Behaviour with `CustomCarHUDs = 2` | the code checks the flag only at the draw, and the stock flags are cleared at the hook; the stock HUD may stay hidden for other cars in the real plugin |
