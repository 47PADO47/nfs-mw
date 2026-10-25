# UI themes: the composer, the layers and the parity gate (T12)

Companion to [ui-themes.md](ui-themes.md) (sections 7 to 8) and [ui-themes-menus.md](ui-themes-menus.md). This page
records how the presenters draw through the theme registry, what the Original theme must reproduce, and what is still
owed. Code: `crates/nfsmw/src/ui/theme/compose.rs`, the HUD and menu wiring in `hud/plugin.rs` and
`frontend/plugin.rs`, the composer tests in `ui/theme/compose_tests.rs`.

- **Status:** built and tested with synthetic trees (T12). The live draw path goes through the composer. The GPU
  digests and the Mods-page selector are owed (section 7).

## 1. The rule: a second producer, not a second draw path

ADR 0002 keeps one presenter (`ui/present/blackbox.rs`) and one neutral tree (`UiTree`). The composer builds that
tree from the active theme's slots. The presenter is unchanged.

A **layer** is one `UiTree` and one presenter call. The layers keep the order the presenters always drew in, because
each call's meshes go over the earlier calls':

| # | Layer | Slot(s) | Source of the nodes |
|---|---|---|---|
| 1 | radio | `RadioCard` | chyron package (Original) or the Custom card (feed) |
| 2 | HUD | the HUD slots (1 to 9, 12, 13 to 19), then unclaimed HUD nodes | `HUD_SLOTS` in catalogue order, then `emit_unclaimed` |
| 3 | busted overlay | `BustedOverlay` | `BUSTED_OVERLAY.fng` (no viewport: full screen) |
| 4 | one per menu screen | its menu slot, or the screen's own tree | menu cell (Original) or the theme's component |

Each layer is built the same way (`compose.rs`): `resolve` the slot, `layout` it at the screen bounds with the host
clock, `build` into one `DrawList`, `DrawList::sort_by_index`, then `into_tree(blackbox_feng::tree::is_drawn)`. The
predicate is the runtime's own, so `draw_order` is what the runtime would have made.

## 2. Filling the cells (the producer rule)

The plugin advances its package and reads `Runtime::tree` once per frame, as before. It stores each source's tree in
the frame cell with the transforms the presenter always got, and the composer only reads the cell.

- **HUD (`hud/plugin.rs::present`).** `hud.viewport.apply` runs on the source tree, where `index` is the position. The
  composed tree is never viewported. The chyron is placed with `trax.place` before it is stored.
- **Radio feed.** `UiThemes::radio().set(&radio, screen, hud_layout)` runs each frame the HUD draws. The card places
  itself, so its nodes get no HUD viewport.
- **Menus (`frontend/plugin.rs::present`).** Every menu slot is cleared, then each screen's tree is stored under its
  slot (section 4). The trees are the decorated ones the presenter gets, so the cell and the draw agree.

Nothing else changes: `ScreenLogic`, `Cx`, `Command`, the stack, the input path and the FEng scripts stay as they were.

## 3. The HUD layer

- **Slot order and object order.** The composer builds the HUD slots in catalogue order, then sorts the list by the
  FEng object index (`DrawList::sort_by_index`, stable). The sort is what matters: `z` ties are common in the HUD, and
  `into_tree` breaks them by list order. The tachometer nests the gear, so slot order alone could not reproduce the
  package's order. Sorting by index does.
- **Unclaimed nodes (`hud_slots::emit_unclaimed`).** A node is claimed when some HUD slot's scope takes it
  (`hud_slots::claim_counts`). Every node that no slot claims is appended, in the package's order, with its `parent`
  remapped by object index. Those are the rear-view mirror bezel (`0x7FAC287B`) and whatever no root covers (the radar, its
  laser and the heat arcs are the `PoliceRadar` and `Minimap` slots'). They are drawn with the rest of the HUD. Visibility decides the claim: a node under a hidden root is unclaimed, so it stays in
  the tree (invisible, so not drawn).
- **Components that draw in the HUD layer.** Base components leave `index` at 0. Their nodes sort before the FEng nodes
  that share their depth, and keep their own build order. A theme that wants a node after the package's nodes at equal
  depth must raise its `z`.
- **Claims are disjoint.** The tests check each node is claimed at most once, with the gear inside the tachometer.
  On the real package the check is owed (no game data here).
- **Radar cool-down and the bounty row.** The cool-down is unchanged from `ui-themes.md` section 8. The bounty row sits
  under the pursuit board and is its own slot (`BountyRow`, 15). A slot nested in another's objects goes with its host
  (`original::HOSTS`): when the theme replaces the host and leaves the nested slot to Original, the composer skips the
  nested slot, so a theme with its own pursuit display draws no FEng bounty row, as before
  ([ui-themes-slots.md](ui-themes-slots.md)).

## 4. The menu layers

- **Slot for a screen (`frontend/factory.rs::slot_for`).** It tests the same names and `Args` as `factory::make`, in
  the same order (`ui-themes-menus.md` section 3). It returns an `ElementId`; the cell keys by its `SlotId`. A screen
  opened with `open_with` (a dialog or an alert) has no slot.
- **Topmost wins.** The plugin walks the stack from the top. The first screen that takes a slot stores its tree in the
  cell and draws the composed layer. A screen under it on the same slot draws its own tree. The presenter still
  draws every screen, so nothing is lost while no theme overrides a slot.
- **Per-screen layers, not one tree.** Each screen keeps its own presenter call. Merging the stack into one tree would
  change the layering the presenter has always given.
- **Clearing.** Every menu slot is cleared each frame before the stack is stored. Skipping this leaves a stale tree.

## 5. The radio layer and the chyron gate

- **Gate.** `UiThemes::provided_by(RadioCard)` names the theme whose component draws the slot. The chyron plays and is
  stored only when that is Original (`ThemeId::Original`). Under Custom the card replaces the slot, so there is no
  chyron. A mod theme that does not name `RadioCard` keeps the chyron, because the slot falls back to Original. This
  replaces the T10 stopgap `ea_trax` check.
- **Song.** The EA Trax song is fed to the chyron's `apply` only when the chyron is shown, as before.

## 6. The parity gate (`ui/theme/compose_tests.rs`, no GPU, no game data)

Under the Original theme the composed layer equals the stored tree, node for node (every `UiNode` field, through its
`Debug` form) and in the same `draw_order`. Tested for:

- the HUD: ten slots, the gear nested in the tachometer, the mirror, the radar, a blip under the minimap, a hidden
  root, and an object the runtime does not draw;
- the radio layer (the chyron package) and the busted overlay (the package);
- each of the twelve menu slots (the whole tree, parents included);
- the Custom theme leaves every HUD slot Original, and draws no chyron;
- a mod theme that names only the tachometer replaces that node set and keeps the gear, the mirror and the rest;
- an empty cell draws nothing; a cleared menu slot draws nothing.

**One documented difference.** A slot root whose parent lies outside its slot is composed with `parent: None` (the
gear's root inside the tachometer; a blip icon under the minimap). The adapters have always done this
(`original/subtree.rs`). Nothing draws from that link after composition: the prompts are decorated before the cell is
filled, and the viewport runs on the source tree. The composed trees are therefore identical in everything drawn, and
the test expects this one difference.

## 7. Owed

- **On-GPU parity.** `nfsmw view-world --wait-for-load --screenshot` and `nfsmw view-car --screenshot` under
  `--ui-style original` must match the pre-T12 output. The 256 x 144 digests are in
  `libs/blackbox-render/src/gpu/parity/digests/`. A menu check uses `--ui-script`. This needs a GPU machine, so it was not
  run here; the baseline GPU tests crash in this environment.
- **Real-package checks.** The "no node claimed twice" check and the "gear is the only nested slot" assumption on the
  real `HUD_SingleRace.fng`.
- **Mods-page selector row.** Nothing calls `UiThemes::select_mod` yet (`ui-themes.md` section 12). Until it does, a
  mod theme is registered but not selectable.
- **Gauge mod primitive.** `Gauge` is not in `UiPrim` (`ui-themes.md` section 12).
- **Mod viewport decision.** A mod theme's HUD nodes are composed after the viewport, so they are not moved by the HUD
  viewport. Whether a mod's HUD should follow the viewport is undecided. The current behaviour is the one the tests pin.
- **Cost.** Each frame copies the HUD and menu nodes into new lists and sorts them. Not measured. Check it with the
  `hud` and `frontend` timings if a frame budget is hit.
