# UI themes: the Custom radio card

The Custom theme's first component, `RadioCard`, and the retirement of the egui radio card. It is the one slot the
Custom theme fills today (`ElementId::RadioCard`, number 10). The contract it sits under is
[ui-themes.md](ui-themes.md) section 9. This page records what was built in T9 and what T12 must wire.

- **Sources read:** `crates/nfsmw/src/hud/radio.rs` (the egui card, its placement and fade), `hud/viewport.rs` (the
  placement math it reuses), `devtools/mod.rs` (the egui call site), `libs/blackbox-ui/src/components/` (`Panel`,
  `Stack`, `Text`), and the Original cells (`ui/theme/original/frame.rs`) for the shared-cell pattern.
- **Data inputs:** none new. The card shows the song on the air from `Audio::now_playing()`, as before.
- **Status:** built (T9). The component and its feed are in place and tested. The card draws only once T12 fills
  the feed and presents the Custom theme. Until then the game draws no radio card in either style.

## 1. What the card is

`RadioCard` is a `blackbox-ui` component. Its `build` emits neutral nodes only, no egui:

| Node | Source | Content |
|---|---|---|
| background | `Panel` | `radio.background` colour, black at alpha 150 |
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

**For T12:** the HUD plugin calls `ui_themes.radio().set(&radio_hud, screen, settings.hud_layout)` once per frame,
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
| `radio.background` | `[0, 0, 0, 150]` |
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
  `crate::hud::show_radio` when `settings.radio_hud == RadioHudStyle::Custom`. Nothing else in `run_ui` changed. The
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
  strings; the background alpha is 150 and the title alpha 255 while the card is up; at half fade both fall to about
  75 and 128; it builds nothing when idle or past its show time; and its background sits where the PC layout puts it
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

## 7. For T12

- Call `RadioFeedCell::set` each frame, as in section 2. Do not apply the HUD viewport to the card's nodes. The
  card places itself from the same `parameters`.
- The card is the `RadioCard` slot's component. The composer draws it when the Custom theme is active. Under
  Original the EA Trax chyron draws the card, as before.
- Watch the presenter draw order: the card's `z` is the default 1, and the background is built before the text, so
  the text draws on top at equal depth.
- End-to-end check: `--ui-style custom` shows the card through the presenter, not egui, and every other element
  looks like Original (`ui-themes.md` section 13).
