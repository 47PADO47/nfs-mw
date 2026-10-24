# UI themes: the Custom radio card

The Custom theme's first component, `RadioCard`, and the retirement of the egui radio card. It is the one slot the
Custom theme fills today (`ElementId::RadioCard`, number 10). The contract it sits under is
[ui-themes.md](ui-themes.md) section 9. This page records what was built in T9 and what T12 must wire.

The Custom theme is being rewritten from scratch ([ADR 0013](../decisions/0013-custom-hud.md)); the new design is
[custom-hud/](custom-hud/README.md). This page stays as the record of the first Custom pieces.

- **Sources read:** `crates/nfsmw/src/hud/radio.rs` (the egui card, its placement and fade), `hud/viewport.rs` (the
  placement math it reuses), `devtools/mod.rs` (the egui call site), `libs/blackbox-ui/src/components/` (`Panel`,
  `Stack`, `Text`), and the Original cells (`ui/theme/original/frame.rs`) for the shared-cell pattern.
- **Data inputs:** none new. The card shows the song on the air from `Audio::now_playing()`, as before.
- **Status:** built (T9) and wired (T12). The HUD plugin fills the feed each frame and the composer draws the card
  when the Custom theme resolves `RadioCard` ([ui-themes-compose.md](ui-themes-compose.md) section 5). Under Original the
  EA Trax chyron draws the slot as before.

## 1. What the card is

`RadioCard` is a `blackbox-ui` component. Its `build` emits neutral nodes only, no egui:

| Node | Source | Content |
|---|---|---|
| background | `Panel` | `radio.background` colour, black at alpha 170 |
| title row | `Text` in a `Stack` (column, row 1) | the song title, `radio.title`, scale 1.0 |
| credit row | `Text` (row 2) | `Artist - Album`, or the artist alone, `radio.credit`, scale 0.8 |
| time row | `Text` (row 3) | `m:ss / m:ss`, with `  PAUSED` when held, `radio.status`, scale 0.8 |

The strings come from `hud::Card` (`credit()`, `status()`), the same as the egui card used.

**Timing** (the egui card's numbers, unchanged): the card is up for 6 s after the announcement (`SHOW_SECS`). Its
alpha is 1 until the last 1 s, then falls linearly to 0 (`FADE_SECS`). It emits nothing at alpha 0 or when there is
no card. The announcement is a new song, a pause or a resume (`RadioHud::update`, which restarts `age`).

**Placement** (`custom/radio_placement.rs`): the authored spot is the top-left corner (-300, -228) in FEng units from
the canvas centre. `hud::viewport::parameters` gives the scale and offset for the screen and `HudLayout`, so the card
moves and scales like the HUD's left side. The corner is then clamped so the card stays 4 points inside the window,
as the egui card did.

**Size:** a fixed box of 230 by 48 units at scale 1, with 6 units of padding. The egui card sized its box to its
longest line. The presenter squeezes a longer line into the fixed width, so very long titles lose some size.

## 2. How it gets its state: the feed

The card reads a shared cell, `RadioFeedCell` (`custom/radio_feed.rs`), the same pattern as `FrameCell`:

- `RadioFeedCell::set(&self, hud: &RadioHud, screen: Screen, layout: HudLayout)` copies, once per frame, the song on
  the air (`hud.song()`), its age (`hud.age()`) and the anchor for `screen` and `layout`. It is `pub(crate)`.
- `CustomTheme` holds a clone. `RadioCard::layout` reads a snapshot, does the fade from `age`, and lays out the
  components with the faded skin. `build` emits what `layout` made.
- The cell is `Arc<Mutex<..>>`, so it is `Send + Sync` as `Theme` requires. A poisoned lock is still a whole feed.
- `UiThemes::radio()` returns the clone the plugin keeps.

**Wired (T12):** the HUD plugin calls `themes.radio().set(&radio_hud, screen, settings.hud_layout)` once per frame,
after `radio::update` has run, using the same `Screen` the presenter gets. It needs no clock. The card's timing comes
from `RadioHud::age`, which `radio::update` advances by the frame delta.

Layout receives `bounds`, but the card ignores it. Its place comes from the feed (the HUD layout), not from the slot
it is drawn in. The composer must not place it again: no `hud.viewport.apply` on the card's nodes, and no chyron
transform.

## 3. The Custom skin

`custom/skin.rs` holds the keys and their values, and `ui/theme/mod.rs::default_skin()` returns them (the registry's
one skin, shared by all themes; the Original components read none of these keys):

| Key | Value |
|---|---|
| `radio.background` | `[0, 0, 0, 170]` |
| `radio.title` | `[255, 255, 255, 255]` |
| `radio.credit` | `[200, 200, 200, 255]` |
| `radio.status` | `[150, 150, 150, 255]` |
| `radio.font` | `fe_hash_upper("FONT_MW_BODY")` (the prompts' body font) |

The colours are the egui card's. The font is the presenter's body font at its own size, which stands for the egui
title size (15 units). The detail rows are 0.8 of that, from `Text::with_scale`.

## 4. The `Text` change

The Custom card needs two text sizes, and `Text` had none. A font size is the font's own, and the presenter applies a
node's world transform to its glyphs. So `libs/blackbox-ui` `Text` gained `with_scale(f32)` (default 1.0). The scale
is on the node's transform, and the justification origin does not move. The wrap width is the rectangle in unscaled
text. A scale of 1 leaves every existing node unchanged, and the existing tests still pass.

## 5. What T9 removed

- `crates/nfsmw/src/devtools/mod.rs`: the `radio` and `hud` system parameters, and the block that drew
  `crate::hud::show_radio` when `settings.radio_hud == RadioHudStyle::Custom` (T10 renamed the check to `settings.ui_style == UiStyle::Custom`). Nothing else in `run_ui` changed. The
  scene panels, the metrics overlay, the console and the graphics lab are drawn as before, with the same arguments.
- `crates/nfsmw/src/hud/radio.rs`: `show` and its egui imports, `placement`, the timing constants, and the
  time-filtered accessors `RadioHud::card()` and `opacity()`. Their timing moved to `radio_card.rs`, and the
  placement tests moved to `radio_placement.rs`. `Card` (with `credit()` and `status()`) and `RadioHud` keep their
  data (`song()`, `serial()`, the new `age()`) and the `update` system, which still feeds the EA Trax chyron
  (`hud/plugin.rs`) and the feed.
- `crates/nfsmw/src/hud/mod.rs`: the `show_radio` export. It now exports `Card` and `RadioHud`.
- Nothing else referenced `show_radio`. The workspace search before the change found only the devtools call site.

The EA Trax chyron (`Original` `RadioCard`, `hud/trax.rs`, `EA_TRAX.fng`) is unchanged.

## 6. Tests

- `custom/tests.rs`: `CustomTheme::element` returns `Some` only for `RadioCard` (and `None` for every other catalogue
  slot and for an id outside the catalogue). The card builds one background and three text rows with the right
  strings; the background alpha is 170 and the title alpha 255 while the card is up; at half fade both fall to about
  85 and 128; it builds nothing when idle or past its show time; and its background sits where the PC layout puts it
  on a 1080p screen.
- `custom/radio_placement.rs`: the PC, Classic and Xbox 360 layouts, the edge clamp in narrow and odd windows, and a
  window with no height.
- `custom/skin.rs`: the font key is the prompts' `FONT_MW_BODY`, and fading scales only the card's alpha.
- `hud/radio.rs`: the data tests (a new song, a pause, a stop, an album-less song), with `age` in place of the removed
  `card()` and `opacity()`.
- `libs/blackbox-ui/src/components/text.rs`: `with_scale` scales the transform and keeps the origin and wrap width.

Runs: `cargo test -p blackbox-ui` (67 and 1 passed); `cargo test --bin nfsmw ui::theme` (47 passed);
`cargo test --bin nfsmw hud::radio` (4 passed). The devtools tests were not run, as their GPU tests crash at
baseline.

## 7. Wiring (T12)

- `RadioFeedCell::set` runs each frame the HUD draws, as in section 2. The HUD viewport is not applied to the card's
  nodes; the card places itself from the same `parameters`.
- The card is the `RadioCard` slot's component, in its own layer before the HUD. Under Original the EA Trax chyron
  draws the slot, as before.
- The card's `z` is the default 1, and the background is built before the text, so the text draws on top at equal
  depth. The composer sorts by `index`, and the card's nodes keep their build order (stable sort, equal indices).
- Owed: the end-to-end screenshot with `--ui-style custom` on a GPU machine (`ui-themes-compose.md` section 7).

## 8. The speedometer and the tachometer (retired)

**Retired in the Custom HUD 2026 (ADR 0013, PR 5).** Sections 8 and 9 record the first drive card; its files are
deleted, and the drive slots fall back to Original until the horizontal cluster lands (PR 6). The Custom HUD reads
`CustomFeedCell` (`custom/feed.rs`); `DriveFeedCell` stays for the Remastered theme.

The Custom theme also filled `Speedometer` (2) and `Tachometer` (1). Each is a neutral component, with no egui, and the
rest of the HUD (the gear digit in the tachometer group, the minimap, the blips, the pursuit board) stays Original.

- **Data:** `custom/drive_feed.rs`. `DriveFeedCell::set(&HudState)` runs once per frame in the HUD plugin, after the radio.
  It copies the speed in the unit shown (`display_speed`), the unit, the rev fraction, the red line fraction and the gear.
- **Placement:** `custom/drive_layout.rs`. A column in the bottom-right of the canvas: the rev bar, with the speed card
  under it. These are fixed FEng positions. The HUD layout setting does not move them yet.
- **Speedometer** (`custom/speedometer.rs`): a `Panel` and a `Stack` of two `Text` rows: the speed, then the unit and
  the gear (`R`, `N` or the number).
- **Tachometer** (`custom/tachometer.rs`): a track `Panel`, and a fill `Panel` whose width is the rev fraction. The fill
  turns red from the red line.
- **Skin:** the `drive.*` keys in `skin.rs`. They are not faded with the radio card.
- **Tests:** `custom/drive_tests.rs`.

Still Original: the minimap (it needs the track geometry and the blips, so it is a separate piece of work).

## 9. The drive cluster (retired)

The Custom cluster replaces the Original tachometer, speedometer, gear, nitrous, turbo and speed breaker slots. It is
one card in the bottom-right corner, drawn in parts by six slots, all fed from `DriveFeedCell` (`custom/drive_feed.rs`):

- **Tachometer** (`tachometer.rs`): the card and a thin 32-segment rev gauge. Segments are cyan, amber from 85% of the
  scale (`WARN_FRACTION`), and red past the red line.
- **Speedometer** (`speedometer.rs`): the speed in large digits and the unit (KM/H or MPH).
- **GearIndicator** (`gear.rs`): the gear in a box (red on the shift light), and a strip of eight pips for R, N and 1 to 6.
- **Nitrous, Turbo, Speedbreaker** (`meters.rs`): one row each, a label and a thin bar. A meter the car does not have
  draws nothing. The boost bar fills to 30 psi, a scale and not a limit.

Layout: `drive_layout.rs`. The HUD layout setting does not move the cluster yet. The palette is in `skin.rs` (the `drive.*` keys).
Tests: `custom/drive_tests.rs`.

PR 5 of the Custom rewrite deletes this cluster and PR 6 replaces it with the horizontal telemetry cluster
([custom-hud/telemetry.md](custom-hud/telemetry.md)).
