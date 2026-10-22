# Split-screen HUD

The two HUDs of a split-screen race: one FEng package per player, bound to that player's `HudState` and drawn over their
half of the window. Part of [split-screen.md](split-screen.md) (section 4.1); where the exe and the decomp were read is in
[provenance/split-screen.md](../provenance/split-screen.md).

- **Evidence tags:** **[exe]** read in `speed.exe` with Ghidra. **[decomp]** read in the CC0 decompilation
  [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) for understanding, no code copied. **[verified]** seen in the
  install (`nfsmw dump-screen HUD_Player1.fng`, and the install tests of `hud/split/install_tests.rs`). **[ours]** this
  rewrite's choice. **[unconfirmed]** not traced.
- **Code:** `crates/nfsmw/src/hud/split/`. The single-player HUD is [hud-layout.md](../hud-layout.md) and
  [hud-viewport.md](hud-viewport.md).

## 1. The packages

`GLOBAL/InGameSplitScreen.bun` (32 KB) holds four compressed FEng packages and nothing else **[verified]**. The game picks
one by a layout number (`0x005694E0`) **[exe]** and loads a texture pack for the pair **[decomp]**:

| Package | Layout | Objects | Textures | For |
|---|---|---|---|---|
| `HUD_Player1.fng` | 3 | 95 | `HUDTEXSPLIT.BIN` | player 1 (top half), circuit / sprint / speedtrap |
| `HUD_Player2.fng` | 4 | 57 | `HUDTEXSPLIT.BIN` | player 2 (bottom half) |
| `HUD_Drag_Player1.fng` | 5 | 60 | `HUDTEXDRAGSPLIT.BIN` | player 1, drag race |
| `HUD_Drag_Player2.fng` | 6 | 60 | `HUDTEXDRAGSPLIT.BIN` | player 2, drag race |

The decomp names the two packs the other way round (its `HudSplitScreenTexturePackFilename` is `HUDTEXDRAGSPLIT.BIN`).
The contents settle it **[verified]**: `HUDTEXDRAGSPLIT.BIN` has `2PlayerDrag_Shift_Backing` (`0x21F58C07`), which only the drag
packages use, and `HUDTEXSPLIT.BIN` has `Tollbooth_Icon` and `icon_Wrongway`, which only the race packages use. Both packs
are small (14 and 13 textures, at most 128 x 32 or 64 x 64) and share names with each other and with `HUDTEXRACE.BIN`
(`4FCA7E3A`, `E7927E1C`, `904B64D3`...); they carry the same sizes, so the first pack that has a name wins
(`ui/assets.rs` `TEXTURE_FILES`). Every resource of the four packages resolves, the multi-image placeholder and `Track2000_map` (the minimap binding swaps its tiles
in) aside **[verified]**.

The exe loads the bun only in a two-player Quick Race (`0x00666AA0`) **[exe]**; the game builds one `FEngHud` per player with
the package of its layout number, and loads the texture pack once, with player 1 (`HudResourceManager::LoadRequiredResources`
is skipped for `PHT_SPLIT2` and `PHT_DRAG_SPLIT2`) **[decomp]**.

## 2. The canvas and the halves

**Every package is authored on the whole 640 x 480 canvas, each in its own half of it, not on a half-height canvas of its own**
**[verified]**. The first player's objects have y between -240 and 0, the second player's between 0 and 240 (the origin is
the middle of the canvas, y grows down; the install test measures every drawn image's corners and every string's origin):

| Package | y extent of what it draws |
|---|---|
| `HUD_Player1.fng` | -234.0 to -5.8 (the minimap and the rule left out) |
| `HUD_Player2.fng` | -5.0 to 223.2 |
| `HUD_Drag_Player1.fng` | -57.8 to -2.0 |
| `HUD_Drag_Player2.fng` | 170.2 to 226.0 |

The numbers are what each package draws at rest (the backing panels reach a few units over the middle: the test allows 10). The
drag packages draw only two strips along the bottom of their half (the clock and rpm bar on the left, the speed on the right).
The first player's minimap, its two arrows and the rule between the halves are left out of the race extent: they sit on the
middle, y = 0.
The same elements repeat with a y offset between the players: player 1's gauge group is at (209, -176), player 2's at (209, 53);
the message line at y = -174 and y = 51; the countdown at -68 and 86 (both in a group at -68 / 154).

So **a half is a 640 x 240 strip of the canvas, as wide as the window and half as high**, and the picture of the world behind
it is the window cut in two, player 1 above player 2. This settles the open question of split-screen.md section 4: the HUD
needs no half rectangle of its own **[verified]**, and the world's two pictures are the window's halves
(`hud::split::half_rect`: `[0, 0, w, h/2]` and `[0, h/2, w, h/2]`) **[ours]**. The drag packages are the same: `HUD_Drag_Player1.fng`'s
left context is at (-135, -21), player 2's at (-135, 206).

## 3. Placement

Each package is drawn with the ordinary HUD math (`hud/viewport.rs`, [hud-viewport.md](hud-viewport.md)) over the whole window,
so it lands in its half by itself, undistorted (the canvas fills the window height, uniform scale), **[ours]**:

1. The **widescreen contexts** move out: the packages answer `WIDESCREENMODE` (`0x62ED04EC`) as `HUD_SingleRace.fng` does
   **[verified]**. Their targets are the left context `0x1603009E` (it holds the minimap's backing) and the groups of the
   gauges, `0x5D0101F1` and, in the first player's race package, `0x2FE4FF9C` too (the `0x5D0101F1` group there is empty).
   The placement reads the targets of the message from the package: a target left of the middle moves left, any other right.
   The offsets are the presets' (`hud_layout`: PC, Classic, Xbox 360; 120 units at 16:9).
2. A window **narrower than 4:3** would clip the 640-unit-wide layout; the whole layout is scaled by `aspect / (4/3)` about the
   middle of the canvas. The single-player HUD keeps the stock 4:3 there (hud-viewport.md); the split HUD does not, because two
   halves of an already short window have nothing to spare. Scaling about the middle keeps each HUD inside its half.
3. The **rule between the halves** (`BasePoly`, guid `0xF6DD2`, 1200 x 4 in the first player's race package, 1200 x 3 in the
   drag one) is widened to the window's width when the window is wider than 1200 units (above 2.5:1).

Nothing is clipped: the first player's minimap crosses the rule on purpose (section 5).

## 4. The objects and what binds them

Names are the FEng hashes of the package; the resolved names are the ones the single-player binders already use or the decomp
shows (`FEPkg_Hud.cpp`, `Fe*.cpp`). "Race" is `HUD_Player1/2.fng`, "Drag" the drag pair.

| Object (name hash) | In | Bound to | Evidence |
|---|---|---|---|
| `SpeedometerGroup` `941FFF09`, `SPEED_DIGIT_1..3` (TAC_NUMBERS), `4113F134` ("888", the dim backing digits) | all four | `HudState::display_speed` (whole units, no leading zero); `3rdPersonSpeedUnits` `C3383B63` ("KM/H") takes the unit label | [decomp] `Speedometer::Update`; [verified] |
| `GaugeCluster` `5164D4EA` | all four | always shown; holds the unit text, the shift light, the backing | [verified] |
| `3rdPersonNeedle` `F0250DAC` (a 104 x 4 `BasePoly`, pivot at its left edge; 270 x 4 in the drag packages) | all four | width = `rpm / max_rpm` of its authored width, left edge fixed; script `61D30442` at `rpm >= red_line`, `INIT` below | [decomp] `Tachometer::Update`, `DragTachometer::Update` (`USER_SPLIT_SCREEN` branch); script [verified] |
| `3rdPersonGear` `B84589BE` | all four | `R`, `N` or the gear; the drag one is white and fades to alpha `0x88` while shifting, the race one keeps its authored colour | [decomp] |
| `Shift_light` `25CF2F74` | all four | race: script `02DDC8F0` while `shift_light`, `INIT` otherwise; drag: shown only while `shift_light` (its `INIT` chains to a lit pulse, so it is hidden at rest) and plays `PulseGreen` `2A15F4B9` (the package also has `PulseBlue` and `PulseRed`, the state has no rating) | race [decomp]; drag [unconfirmed] |
| Nitrous group `87C38E97` (backing `DD77F899`, icon `27DDF583`) and bar `EDFB6D37` | all four | shown with `has_nos`; the bar's mask turns `nos * -180 + 180` degrees (the dial HUD uses 175, `HUD_Drag` -48); the icon plays the burning / ready / `INIT` script | [decomp] `NitrousGauge::Update`; [verified] |
| `RaceInformationGroup` `B1C5ED0D` | all four | race: shown with `race`; drag: always (it holds the gear and the shift backing) | [verified] |
| `POSITION_GROUP` `0FC5B7A0`, place string `9C183BF8` | race | `RaceHud::place` (the racers count `3BBD6268` is not an object here) | [verified] |
| `CurrentLapTime` `30D9802E` | all four | `RaceHud::clock` as `mm:ss.cc` | [decomp] |
| Progress group `E3DDCD09`, `percentage` `9CB5C95D`, `Complete_Text` | race | `LAP n/m` or `COMPLETE nn %` as in the single-player HUD | [verified] |
| `Timer_Icon` `6DD754EC` | all four | shown with the clock | [ours] |
| `321_GO_GROUP` `A7B3AA16` (strings `B969E9EF`, `2DA220AA`) | all four | `RaceHud::count`, scripts `THREE`, `TWO`, `ONE`, `GO` | [decomp] `Countdown`; [verified] |
| `GenericMessage` `32A7A521` and its strings `DA768EA6` | all four | `HudState::message`, script `A19BB14C` (3.2 s); its container is the top group of it (`3D2AED05` in player 1's race package, `E0BA07EC` elsewhere) | [decomp] `GenericMessage`; the script is [unconfirmed] |
| `SHIFT_GROUP` `2579C809` ("Perfect Shift!") | drag | script `C8D88528` on a shift up made above 95 % of the red line | [unconfirmed] (the share, as in `HUD_Drag`) |
| Divider `BasePoly` (guid `F6DD2`) | player 1 | drawn, widened to the window | [verified] |

**Not bound** (hidden, or at rest): the wrong-way icon `WRONGWAYIMAGE` `C82FA200` (no state), the time-added glow
`IconSelection_Glow` `CAEC9D04` and its messages `095CA00D` / `61A00397` (no time bonus state), the tollbooth milestone group
`1A091045` (as in the single-player HUD), the drag engine-heat group `ENGINE_HEAT_SHIFTLIGHT_GROUP` `88A56517` (no engine
temperature in `HudState`), the 24 speed-trap icons of player 1's race package (`Minimap_Icon_Speedtrap`, the race
speed-trap blips; no blip style), and the empty groups `NOS_4`, `AFFD0354`, `7FC7C446` and the transparent `White16x16` panel.
There is **no leaderboard**, no pursuit board, no heat meter, no turbo gauge, no speedbreaker meter, no radar, no mirror in the
split packages **[verified]**; the game builds only the speedometer, the race information, the leaderboard (without objects),
the nitrous, the messages, the wrong-way icon, the countdown, the time extension and the tachometer (and, in a drag, the
engine temperature, the drag tachometer and the shift updater) for the split types, and the minimap for player 1 only
(`FEngHud::FEngHud`) **[decomp]**. The `FADEIN` and `FADEOUT` messages (`BCC00F05`, `54C20A66`) that fade the groups in and
out are not raised: the groups rest at full alpha, as the single-player binding does for its mirror.

## 5. The minimap

Only player 1's race package has one (`TRACK_MAP`, its four pieces, `TRACKMAPTARGETRING` and `PLAYERCARINDICATOR`), centred on
the left of the canvas at (-221, 0): on the middle of the window, across the rule **[verified]**. In split screen the game
does not rotate it (`MinimapRotateWithPlayer = 0`) and centres it on player 1 **[decomp]**. Player 2's car is the package's
`PLAYERCARINDICATOR2` (`917DB611`), placed like any other element of the map and pulled onto the rim when it is off it
(`Minimap::UpdatePlayer2Element`, `UpdateElementArt`) **[decomp]**.

Rewrite **[ours]**: `hud/split/map.rs` binds the existing `MinimapBinding` (the maths of `blackbox-minimap`, the tiles of the
race's event or the full map) to player 1's package with the orientation forced to north, and registers `PLAYERCARINDICATOR2`
as a blip style fed by player 2's `HudState::minimap`. The map shows when player 1's state has a `minimap`. Player 2's other
fields are the car's own (speed, rpm...). The drag packages have no map.

## 6. How the app drives it

- **State:** `Scene::split_active()` is true during a split-screen race; `Scene::split_hud_state(0)` and `(1)` give each player's
  `HudState` (the same fields as the single-player HUD: `speed`, `rpm`, `max_rpm`, `red_line`, `gear`, `shifting`,
  `shift_light`, `has_nos`, `nos`, `race`, `message`, `visible`, `drag`, `minimap`, `map_source`). A player with no state has
  no HUD. `drag` is read from player 1's state and picks the pair. `use_mph` is overwritten from the setting.
- **Unused fields:** `leaders`, `pursuit`, `radar`, `blips`, `career_bounty`, `skin`, `has_turbo`, `speedbreaker*`, `busted*`,
  `cop_nearby`, `vehicle_heat`: the split packages have nothing to show them with.
- **System:** the HUD plugin loads the pairs (`hud::split::install`) and adds `hud::split::plugin::present` in `FrameSet::Hud`.
  While `split_active()` the single-player HUD does not draw (`sync` clears its `HudState::visible`). The two trees are
  presented through the shared presenter, first player first; the presenter puts each call's meshes at the front of the frame's
  `UiLayer`, so **a picture drawn under the HUDs (the world's two views) has to be put at the front of the layer after
  `FrameSet::Hud`**, i.e. the layer's order is back to front and the HUD is already in it **[ours]**.
- **Settings:** `hud` off hides both; `hud_layout` picks the preset; the Original theme's slots (Custom speedometer...) do not
  apply to the split HUDs.

## 7. Testing

- `hud/split/tests.rs`: a stand-in package with the real names (no install): speed digits, rpm bar width and redline script,
  gear letters and colour, shift light, nitrous angle, what shows without a race / countdown / message, the minimap flag, the
  half rectangles, the widescreen contexts, the narrow-window fit and the widened rule.
- `hud/split/install_tests.rs` (they return at once without `NFSMW_GAME_DIR`): the four packages are in the bun and each lies
  in its half; every texture resolves; both race HUDs and both drag HUDs draw from synthetic states through the real presenter,
  checked by a software compositor (`raster_tests.rs`). With `NFSMW_SHOT_DIR=<dir>` the frames are written there as PNGs
  (`race_16x9`, `race_4x3`, `race_21x9`, `race_square`, `drag_16x9`). The compositor draws meshes only; the masked pictures (the
  minimap tiles and the nitrous bar, built on the GPU) are not in these images. No window is opened.
- `nfsmw dump-screen HUD_Player1.fng` (and the other three) now work: the bun is in `SCREEN_FILES`.

## 8. Open

1. The drag shift light's rating (`PulseBlue`, `PulseGreen`, `PulseRed`): the state has only "shift now".
2. The engine temperature gauge of the drag HUD, and the time-extension glow: no state feeds them.
3. Which of the five message scripts the game starts for which line (as in the single-player HUD).
4. The PC build's own widescreen handling of the split packages beyond the contexts' ±120: not seen in a capture.
5. The masked pictures (minimap tiles, nitrous bar) were not checked in a screenshot; their binding is the single-player one.
