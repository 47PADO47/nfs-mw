# Menu sections

Section titles in the option pages, and the abstract menu model they are built on. The design decision is
[ADR 0015](../decisions/0015-menu-sections.md); the mod side is [modding/api-settings.md](../modding/api-settings.md). Evidence tags as
in [docs/README.md](../README.md#evidence-tags): **[ours]** is a decision of this rewrite, **[verified]** was measured offscreen.

## 1. What the player sees

A page of the options can group its rows under a **section title**: a line of text centred across the row, in the colour that
highlighted text has, that is never selected. The first section of a page is the **default section** and has no title, so every
row that exists today looks and behaves as before. A section with no visible row shows no title. **[ours]**

- Options > Gameplay: the ordinary rows are untitled. Under **Experimental**: Turn Signals and Cop Health Bars. Under **RIDE 6**: the
  rider rows. Then one section for each title the mods declare ([section 6](#6-the-mod-api)), after those.
- Options > Video: the ordinary rows are untitled. Under **Experimental**: Ray Tracing, Ambient Occlusion, Speed Trails and Speed
  Trail Style.

Every UI style shows the titles: Original (the option package's own rows), Remastered and Custom (the vertical lists).

## 2. The model (`libs/blackbox-ui`, `menu`)

Pure data and rules, no drawing and no game names, so any host can build a list on it.

- **`EntryFlags`** says how the list treats an entry: `highlightable` (the selection can be on it; the pointer's hover selects it),
  `clickable` (a press does something; a press on an entry that is not clickable does nothing), `skip_on_scroll` (the arrows, d-pad,
  stick, wheel and page jumps never stop on it), `centered` (its text is centred across the row) and `accent_text` (its text is in the
  highlighted text's colour). Presets: `ROW` (all behaviours on, not centred: what every row is today), `TITLE` (all three behaviours
  off, centred, accent: a section title) and `ACTION` (all three on, centred, accent: a row that does something when pressed).
  `stops_scroll()` is `highlightable && !skip_on_scroll`.
- **`Entry<A>`** is a label, flags and an optional action `A`. `Entry::title` and `Entry::action` build the two presets;
  `Entry<Callback>::press` runs the callback only if the entry is clickable. A host whose rows already carry their own state (the option
  rows carry a `Setting`) keeps its own row type and borrows `EntryFlags`.
- **`Section<R>`** is a title row (or none) and rows. `flatten` lists sections in order and drops the title of a section with no rows.
  **`SectionSet<R>`** collects rows by a section key: the default section first, then one section per distinct key in the order each was
  declared or first used; `declare` fixes a section's place before it has rows.
- **`menu::nav`** moves a cursor over `&[EntryFlags]`: `first_stop`, `last_stop`, `step` (wrapping, passing every entry that is not a
  stop), `settle`, `page` (a page jump: no wrapping, lands on the nearest stop in the direction of travel, else the first or last stop),
  `can_hover` and `can_press`.

## 3. Navigation in the option pages (`frontend/widget_menu`)

`WidgetMenu` keeps the page as a flat list of rows with a title row at the start of each titled section (`Setting::Heading`, and
`Setting::ModHeading` for a mod's title), and the `EntryFlags` of each (`Row::flags`: a heading is `TITLE`, every other row `ROW`).

- **Opening.** The selection starts on `nav::first_stop`: the first row after a leading title (no page has one today).
- **Up, down (arrows, d-pad, stick, wheel).** `nav::step`: the next row the selection can stop on, wrapping, so a title is passed over in
  both directions and the wrap from the last row lands on the first row, not on a title. The wheel is a press of Up or Down per notch.
- **Page jumps.** The shoulder buttons (`PAD_LTRIGGER`, `PAD_RTRIGGER`; PageUp and PageDown on the keyboard) jump a window of rows with
  `nav::page`. **[ours]** The option pages did nothing with them before.
- **Pointer.** Hover selects the row under it unless it is a title (`can_hover`); a press on a title does nothing and nothing else reacts
  to it (`can_press`); a press on a row selects it and presses it as before.
- **The window.** The window of rows slides to keep the selected row in view as before, and also the title directly above it, so a
  section is never entered without its heading showing. The Remastered and Custom lists show at most eight entries; titles count as
  entries, as the rows of a longer page already scroll.
- **Left and right** change only the selected row, which is never a title.

## 4. Drawing

### Remastered and Custom (the vertical lists)

`MenuEntry` (`ui/theme/menu_list.rs`) has `flags: EntryFlags`. The frontend publishes a title as `MenuEntry::title`: no value, `TITLE`
flags. A theme draws an entry with `centered` with its text centred across the row's bar or plate and nothing at the value column, and
an entry with `accent_text` in the colour of selected text whether it is selected or not (disabled still dims it).

- Remastered: the label in capitals as every entry, centred on the bar's span, in `MENU_TEXT_SELECTED`; no bar and no bolt, since the
  entry is never selected.
- Custom: the label centred on the row's plate in the primary text colour; no plate.

The geometry (`row_at`, hit tests) is unchanged: a title takes a row's band, and the frontend ignores the pointer on it.

### Original (the option package)

The package has a fixed number of row slots, each with a title string, a value string, two arrows and a slider. **No game asset is added
and no FNG object is made**: a slot that shows a title uses its own title string, centred across the whole row (the string's anchor is
moved by its measured width, whatever its justification), with the slot's value, arrows and slider hidden. The colour is the colour of a
selected title in the package, [`ACCENT`] in `widget_menu/heading.rs`; only the colour is set, so the package keeps the alpha, which
carries the screen's fade in and out. The slots are reused as the list scrolls, so a slot that stops showing a title gets the colour the
package gave its titles at start back. **[verified]** The selected title's colour is 190, 128, 43 (read from its string object on the installed `Options.fng`; the other titles are
180, 180, 140), and a test compares a title's colour with the selected row's. A title in a slot that held a dimmed row (Ray Tracing on a
renderer without it) is restored to full strength.

## 5. Which rows are in which section

The rewrite has no label that says "experimental" for a gameplay row, so the sections follow the rows' documentation. **[ours]**

| Page | Section | Rows | Why |
|---|---|---|---|
| Gameplay | Experimental | Turn Signals, Cop Health Bars | the rewrite's own additions that are off by default and have no counterpart in the original ([specs/extra-options.md](extra-options.md): the Extra Options mod's `ShowTurnSignals`; [roadmap](../roadmap.md): turn signals still a rough optional setting); the original's own rows (HUD layout, Camera, Rear View Mirror, Game Moment Camera) are not experimental |
| Gameplay | RIDE 6 | Rider Gear, the gear a mod offers (one row per part), Riding Style (road, motard, offroad), Brake Fingers, Clutch Fingers | the rows that only matter on RIDE 6 bikes and riders ([ride6-rider-styles.md](ride6-rider-styles.md), [ride6-rider-gear.md](ride6-rider-gear.md)) |
| Gameplay | default | everything else, and the options a mod adds without a section (the nitrous and other effect variants included) | |
| Video | Experimental | Ray Tracing ([vulkan-ray-reflections.md](../vulkan-ray-reflections.md): wgpu's experimental ray queries), Ambient Occlusion ([ambient-occlusion.md](../ambient-occlusion.md): "Experimental ambient occlusion"), Speed Trails and Speed Trail Style ([vehicle-effects.md](../vehicle-effects.md): "Speed Trails are experimental") | the rows the docs call experimental |
| Video | default | everything else, and the colour grading rows a mod offers (Color Grading and its strength, `mod_options::video_rows`: appended after the page's own default rows, before the Experimental title) | a look is an ordinary video option, not an experimental one |

The F5 and F8 developer panels ([extra-options.md](../extra-options.md)) have their own sections and are not changed. The rows stay in
their category's order inside a section; moving a row between sections moves it in the page. `options::rows` still lists a page's rows
in the order the page shows them, headings left out, so the selection moves over `rows()[i]` one press at a time.

Collision Sparks has a value called "Restored (Experimental)" but the row's other values are the original's, so it stays in the default
section. The other pages (Audio, Player, Controls, HUD, Discord, Mods, Custom Race) have no sections; `WidgetMenu` handles titles on any
page, so a page opts in by listing them.

## 6. The mod API

`bb.settings` 1.0 takes an optional **section** in an option's declaration ([modding/api-settings.md](../modding/api-settings.md)):

- `BbSettingOption.section` (appended after `pad`; a mod built before it sets `size` to 72 and has the default section) and
  `OptionSpec.section` in the host model, `Settings::section(title)` in the SDK.
- The title is text, or a message id of the mod's own catalogue (`bb.i18n`). It applies when the option is created. Values a mod adds to
  an option the game has leave the row where the game has it (so the effect variant rows stay in the default section).
- No section is the default section, as for a mod that predates the key. The sections of mods follow the game's own, in the order the mods
  registered their options; sections with the same title text are one section, whichever mod made them, so a mod can name
  `Experimental` to join the game's.
- The 2012 content mod puts its own options (2012 soundtrack, Stream 2012 cars, 2012 engine auto-stop) under **NFS: Most Wanted
  (2012)**. The RIDE 6 bikes and tracks and Fairhaven mods declare no `bb.settings` options; Free Roam World, which lists the worlds of
  Fairhaven and the RIDE 6 tracks together, stays in the default section.

## 7. Tests

`libs/blackbox-ui` (`menu`): flags, entries, `SectionSet` and the navigation rules (skipping, wrapping over titles, a lone stop, page
jumps, a list opening on a title). `frontend/options_sections_tests.rs`: the sections of Gameplay and Video, the default section, no title
for an empty section, every row in exactly one section, the mod rows merging (default section, a mod's own section, joining
Experimental). `frontend/widget_menu_tests.rs`: the selection over a page with titles (up, down, wrap, page, hover, click). `modding`:
the section a mod declares, and a struct without it. Screenshots: see [section 8](#8-checking-it).

## 8. Checking it

Offscreen, one image each, with the isolated profile `BLACKBOX_PROFILE_DIR=C:\r6work\profile-sections`, the install
`NFSMW_GAME_DIR` and `--muted`; the script is `C:\r6work\sections-shoot.sh` (the Original shots are `sections-shoot-original.sh`):

```sh
nfsmw view-screen Options.fng --options --category gameplay --muted --ui-style custom --screenshot out.png
nfsmw view-screen Options.fng --options --category video --muted --ui-style remastered --ui-script "wait 2;up;wait 2" --screenshot out.png
```

The scripted presses are the navigation proof: `--ui-script "wait 2;down;wait 0.2;down;..."` with 15 presses of `down` ends on Language, the
last row before the Experimental title; the 16th press ends on Turn Signals, the first row under it (the title is passed over); one `up`
from there is Language again; `up` from the first row wraps to the last row of the page (Clutch Fingers, under RIDE 6). The pictures are
in `C:\r6work\sections-shots`: `gameplay-<style>-top`, `gameplay-<style>-experimental`, `gameplay-<style>-ride6`,
`video-<style>-experimental` for `original`, `remastered` and `custom`, and `nav-original-*`.

Tests that read the installed package need `NFSMW_GAME_DIR`; without it they return early:
`cargo test -p nfsmw -- section_nav_tests options_sections mod_options a_title_entry` and `cargo test -p blackbox-ui menu`,
`cargo test -p blackbox-modding`.

## 9. Not done

- The Mods page (`bb.menus` rows) has no sections; a mod's rows there are as they were.
- `bb.settings` only takes the Gameplay page. A Video option from a mod is not offered by the API.
- Action entries (`EntryFlags::ACTION`) are in the model and both vertical looks draw them centred; no option page lists one yet (Advanced
  Graphics is a row that acts, and keeps its value column).
