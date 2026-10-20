# UI themes: the Remastered vertical menus

The Remastered look of the icon menus (the main menu, the option categories, the pause menu and the safe house). Under
Original these are a row of FEng icons; under Remastered they are a list down the left of the screen. Companion to
[ui-themes.md](ui-themes.md) section 10 and [ui-themes-menus.md](ui-themes-menus.md).

- **Sources read:** this repository only: `crates/nfsmw/src/frontend/` (`icon_menu`, `screens`, `plugin`, `prompts`,
  `logic`), `crates/nfsmw/src/ui/theme/`, `ui/raster_text.rs`, `ui/present/blackbox/`, and a reference picture of the
  intended look (1600 by 915 pixels) supplied by the project owner. No game data and no decompiled code.
- **Status:** built. Checked with offscreen screenshots of the main menu and the options categories
  (`nfsmw play --skip-boot --ui-style remastered --screenshot`). The option tables behind the categories are still Original.

## 1. What is drawn

From the top: a dark shade over the left of the screen; the game's logo (`MW_Logo.tga`, from the install's own textures);
the title in wide-spaced capitals; the entries, the selected one on a slanted gold bar with a lightning bolt; the footer
(two thin rules with the key hints between them); and, on the main menu only, the tagline at the bottom right. The
in-game menus (pause) have no logo, and only the main menu has the tagline ([`MenuStyle`](#3-the-data)).

The entries are the ones the screen has: Career, Challenge Series, Quick Race, My Cars, Online, LAN, Alias Manager and
Options on the main menu. A disabled entry is dimmed and can still be selected, as under Original.

## 2. Places and colours

Every place is written in points of the reference picture and converted to FEng units (`PX` = 480 / 915) in
`ui/theme/remastered/menu/layout.rs`. Vertical places are fixed. Places at the left are measured from the screen's left
edge, `-aspect * 240`, so the list stays at the left in any window shape; the frontend passes the window's aspect.

- **Rows.** The first entry sits at the reference's first row and the spacing is the reference's. At most eight entries
  show (`MAX_ROWS`), so the list always ends above the footer. A longer list scrolls: the window keeps its place while the
  selection moves inside it and slides by one entry when the selection passes either end, so the first entry goes and the
  next becomes the first (`layout::window_start`, kept by `IconMenu::follow` and published as `MenuList::first`). A slim
  scroll bar right of the list shows where the window is. Keys, pad, wheel and the pointer all move the same selection.
- **Bar.** A parallelogram, 55 reference pixels high, leaning 23 pixels. Its opacity ramps from almost clear at the left
  to near solid gold at the slanted right end (`BAR_STOPS`, read from the reference). It is drawn as 32 thin strips
  (`blackbox_ui::Slant`).
- **Colours** follow the HUD palette (`ui_theme`): the bar and bolt take its secondary colour (the Remastered palette's
  gold matches the reference; the Bevy palette gives cyan), the text its primary, the tagline its muted colour
  (`skin.rs::palette_skin`, the `MENU_*` keys). Only the black shade is fixed.
- **Text** is rasterised, not a FEng font (`ui/raster_text.rs`): `RASTER_MENU_FONT_KEY` for entries and hints,
  `RASTER_MENU_WIDE_FONT_KEY` (more letter spacing) for the title and tagline. Both are upright. `Look::tracking` is the
  space after each glyph, as a fraction of the glyph height, and none follows the last glyph, so right-aligned text ends
  on its edge. `raster_text::width_units` measures a string without rasterising it; the footer uses it to place the keys.
- **Footer.** The hints are the Original footer's set (`prompts::hint_set`: Accept and Back, or Accept and Quit on the
  main menu), with the player's current bindings. A keyboard key is a rounded outline (`blackbox_ui::Outline`) around its
  name; a pad button is its glyph from the input atlas.

## 3. The data

The theme never sees a FEng package. `ScreenLogic::list` gives the screen's entries as a `MenuList` (title, entries,
selection, style); `Screens::presented` adds the footer hints and the window's aspect. The frontend plugin stores each
frame's lists in the `MenuListCell` (`UiThemes::menu_lists`), keyed by menu slot, after clearing it, in the same step
that fills the menu cell. The `VerticalMenu` component of a slot reads its list and draws nothing when it has none.
Under Original nothing reads the cell.

## 4. Behaviour

`IconMenu::vertical` is true while the Remastered style is active and no mod theme was chosen (`IconMenu::themed`). Then:

- **Keys and pad.** Up and Down do what Left and Right do on the icon row, and Left and Right do nothing
  (`screens/vertical.rs::vertical_pad`, applied to the pad mask before the runtime sees it). The runtime therefore still
  repeats a held direction and plays the menu sounds.
- **Mouse.** The pointer selects the row under it when it moves; a click on a row selects and presses it
  (`ScreenLogic::pick_row`, rows counted from the window's first entry); a click on a footer hint presses its action. The package's own icons take no clicks while the
  menu is vertical. The wheel steps the selection as before.
- **Leaving.** The package draws nothing, so its leave animation would only be a pause. A vertical menu acts at once on a
  press or on Back.

## 5. Not done, and open

- The option tables (`OptionTable`), the track select and the career screens are still Original, so the menus behind a
  category keep the old look. A vertical table in the same style is the next step.
- The logo is the texture alone; the reference's black splatter behind it is not drawn.
- Hover also follows the pointer on a stack screen under a dialog only when it is the top screen.
- Checked on one aspect (16:9) by screenshot. Other aspects are covered by the layout tests (the left edge moves with the
  aspect), not by screenshots.

## 6. Tests

`ui/theme/remastered/menu/tests.rs` (what the component builds for a list), `menu/layout.rs` (rows, the left edge, row and
hint hit tests), `ui/theme/menu_list.rs` (the cell), `ui/raster_text.rs` (tracking, the measured width equals the rendered
one), `frontend/screens/vertical.rs` (the pad remap) and `libs/blackbox-ui` (`Slant`, `Outline`).
