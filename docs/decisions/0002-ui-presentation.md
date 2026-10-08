# 0002 — How the HUD and menus are drawn

- **Status:** accepted (2026-10-08).
- **Milestone:** 5 (the HUD), 6 (the menus use the same path).

## Question

The original HUD and menus are FEng packages: object trees with animation scripts and messages. We need them on
screen, with the original look, without tying the game to one way of drawing, because milestone 8 may move the
renderer to Bevy ([0001](0001-bevy.md)).

## Decision

Three layers with one direction of dependency, and **exactly one presenter**.

1. **`libs/blackbox-feng`** (engine-generic, no rendering, no Bevy, no wgpu): reads FEng packages and fonts, runs
   the scripts and messages, and yields a retained, flat `UiTree`: one node per object with its kind (group,
   image with texture and UV, text with the resolved string and font), its transform, accumulated colour, depth
   and the order to draw in. It also has the host interface: bind values by object name (text, visibility,
   rotation, texture, script), post messages, take the messages the UI sends out, move the focus.
2. **`crates/nfsmw/src/ui/`, `hud/` and `frontend/`**: plain data and glue. `ui/` is shared: the packages of the
   install by name (`Catalog`), the fonts, textures and strings (`UiAssets`) and the presenter. `HudState` (in
   `hud/`) is a plain resource (speed, rpm, gear, shift light, units); `bind` copies it into the runtime using the
   names the game uses. `frontend/` runs the menu screens (milestone 6). The plugins run them each frame.
3. **`ui/present/blackbox.rs`**: the single presenter. It turns the `UiTree` into premultiplied meshes for
   `blackbox-render`'s UI layer and uploads the textures it needs as UI texture patches. It runs after the egui
   pass and puts the HUD under egui's panels (console, metrics).

Rules that keep it replaceable:

- Game code (scenes, physics, input) knows `HudState` only. A scene returns it from `Scene::hud_state`. Nothing
  outside `hud/`, `ui/` and `frontend/` names FEng or the presenter.
- The presenter has a small surface: tree, assets, screen size, in; meshes and texture patches, out. A Bevy UI
  presenter (`bevy_ui` nodes, or sprites) replaces `present/blackbox.rs` and leaves the rest alone.
- The tree carries everything a presenter needs (world matrix, colour, depth, order), so a presenter does not
  re-implement FEng rules.
- Menus (milestone 6) add more packages and a focus and input path, not another drawing path: `send_to_focus`
  and `take_outgoing` already exist for that.

## Why not the alternatives

- *Draw straight from the package in the renderer:* ties `blackbox-render` to FEng and duplicates the script
  rules in every backend.
- *Re-create the HUD with egui:* loses the original look, and every screen would need hand-written layout.
- *Bevy UI now:* the renderer is ours until milestone 8; and Bevy UI has no equivalent of FEng scripts and
  messages, so the runtime would still be needed.

## Consequences

- The HUD looks like the original (same textures, fonts, layout, needle and digit behaviour) by construction.
- Text uses the game's bitmap fonts, laid out by the same rules, so strings from the language table look right
  in every language the fonts cover.
- Costs: the 640 x 480 coordinate system is scaled to the window height, so wide screens need the package's own
  widescreen handling (not done yet); the multi-image mask of the redline is not drawn.
