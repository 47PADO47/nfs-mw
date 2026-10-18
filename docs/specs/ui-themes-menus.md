# UI themes: the menu cell, the menu adapters and the T12 fill map

The menu half of the Original theme, from [ui-themes.md](ui-themes.md) section 8. It covers what T8 built (the menu
cell and one adapter per menu and career slot), the mapping from a produced menu tree to its slot that T12's frontend
plugin must implement, and the open points T12 owes. Sources read: `crates/nfsmw/src/frontend/{factory,screens,plugin,logic}.rs`,
`frontend/career_screens/{mod,blacklist,car_lot,rap_sheet/mod,region_unlock,save,sms,safehouse_menu}.rs`,
`crates/nfsmw/src/ui/theme/original/`. No game data and no decompiled code.

- **Status:** built (T8) and tested with synthetic trees. Nothing fills the menu cell in the live path yet. T12 wires it.

## 1. What T8 built

- **Menu cell.** `FrameCell` (`original/frame.rs`) gains a second cell beside the HUD one: `HashMap<SlotId, Arc<UiTree>>`
  behind `Arc<Mutex<..>>`. It is `Send + Sync` and shares by clone, like the HUD cell. A poisoned lock is read through.
  - `set_menu(&self, slot: SlotId, tree: Arc<UiTree>)`: stores the slot's tree for this frame, replacing the old one.
  - `clear_menu(&self, slot: SlotId)`: empties the slot for this frame.
  - `get_menu(&self, slot: SlotId)`: read by the adapters (`pub(super)`).
  - Setter visibility is `pub(crate)` with `#[allow(dead_code)]`, until T12 calls them.
- **Menu adapter.** `MenuTree` (`original/menu.rs`) is a `Component` that holds the cell and a slot. `build` copies the
  whole tree of that slot with the HUD adapters' `emit` and `Scope::Package`: every node, in the tree's order, `index`
  kept, `parent` remapped into the draw list. `layout` does nothing. The adapter runs no script, reads no input and
  sends no message. An empty slot emits nothing.
- **Every slot filled.** `OriginalTheme::element` tries the HUD adapter first, then the menu adapter. It now returns
  `Some` for all 23 catalogue slots (`ElementId::ALL`). The HUD slots are unchanged.
- **Every-slot invariant, MW side.** The generic `Themes::new` in `libs/blackbox-ui` cannot know the MW catalogue, so
  the check lives in `crates/nfsmw`:
  - `ui::theme::build_themes` runs `check_floor_fills_every_slot` at start-up and panics if the floor leaves a slot
    unfilled. This is the "checked at start-up" rule of [ui-themes.md](ui-themes.md) section 7, enforced where the
    catalogue is known.
  - `original/tests.rs::every_slot_has_an_original_adapter` asserts the same for every `ElementId::ALL` entry, with an
    empty cell (the adapters exist whether or not a tree is set).
  - `libs/blackbox-ui` is unchanged.

## 2. The menu cell contract

- **One tree per screen, as presented.** The tree stored for a slot is the tree the presenter draws for that screen:
  `Screens::presented_trees` output, after `prompts::decorate`. Storing the raw `Runtime::tree` would make the Original
  adapter differ from the presenter (no key hints).
- **Per frame.** The frontend plugin refills the cell every frame. A tree from an earlier frame must not survive.
- **Read at build time.** An adapter resolved once reads the cell's current tree each time it is built, so the theme
  need not be re-resolved when a screen changes.
- **Not composed across screens.** The presenter draws each screen as its own tree, lowest first
  (`frontend/plugin.rs::present`). A menu slot's tree is one such tree, so the menu cell does not need the HUD's
  composer. Its order matches the present loop.

## 3. Slot mapping for T12

The frontend plugin maps a stack screen to a slot with the same tests as `frontend/factory.rs::make`, in the same order.
The first matching row wins. "Name" is the package file name, any case.

| Slot (raw) | Package (name) | Condition (`Args`, state) | Factory branch it mirrors |
|---|---|---|---|
| `MainMenu` (20) | `MainMenu.fng` | none | `IconMenu` `Kind::Main` |
| `SafeHouseMenu` (25) | `MainMenu_Sub.fng` | `args.safehouse` | `career_screens::safehouse_menu::make` → `Kind::SafeHouse` |
| `Blacklist` (40) | `SafeHouseRivalChallenge.fng`, `SafeHouseRivalBio.fng` | none | `blacklist::make` |
| `CarLot` (41) | `Car_Select.fng` | none | `car_lot::make` |
| `RapSheet` (42) | the 11 `RapSheet*.fng` of `rap_sheet::SCREENS` | none (see note) | `rap_sheet::make` |
| `RegionUnlock` (43) | `SafehouseRegionUnlock.fng` | none | `region_unlock::make` |
| `MemoryCard` (44) | `MC_Main.fng`, `MC_List.fng` | none | `save::make` |
| `Sms` (45) | `SMS_Mailboxes.fng`, `SMS_Message.fng` | none | `sms::make` |
| `Categories` (21) | `MainMenu_Sub.fng` | not `safehouse` | `IconMenu` `Kind::Categories { pause: false }` |
| `Categories` (21) | `Pause_Main.fng` | `args.options` | `IconMenu` `Kind::Categories { pause: true }` |
| `PauseMenu` (22) | `Pause_Main.fng` | not `options` | `IconMenu` `Kind::Pause` |
| `OptionTable` (23) | `Options.fng`, `Pause_Options.fng` | none | `WidgetMenu` |
| `TrackSelect` (24) | `Track_Select.fng` | none | `TrackSelect` |

- **Shared packages.** `MainMenu_Sub.fng` serves `SafeHouseMenu` (with `safehouse`) and `Categories`. `Pause_Main.fng`
  serves `PauseMenu` and `Categories` (with `options`). The `Args` test is the only thing that tells them apart, so T12
  must read `Args`, not the name alone.
- **Career packages before the front-end ones.** `factory::make` tests the safe house first, then the career screens,
  then `MainMenu_Sub` and `Pause_Main`. The career names do not overlap the front-end names, so the order only matters
  for `MainMenu_Sub` with `safehouse`.
- **Fallbacks.** `safehouse_menu::make` returns `Plain` when no career is loaded, and `rap_sheet::make` returns `None`
  when the rap sheet is locked (`unlocked()`). The screen then runs as a plain package. Map by name and `Args` (the
  table), not by the logic type, so the slot is the same either way. The screen still has a tree to store.
- **Not in any slot.** `Dialog.fng`, `Track_Options.fng`, `Options_PC_Controller.fng`, `Pause_PC_Controller.fng`,
  `Credits.fng`, `EA_Trax_Jukebox.fng`, `WorldMapMain.fng`, `Infractions.fng`, `WS_Loading.fng`, and the splash
  packages. They get no slot (ui-themes.md section 6), so the cell never holds their trees.

## 4. Fill procedure for T12 (frontend plugin)

Run in `frontend/plugin.rs::present`, from the same `presented_trees` call that the present loop already makes, so the
cell holds exactly what is drawn. Do not call `Runtime::tree` a second time.

1. Take the trees, lowest first, zipped with the stack's screens.
2. For each screen whose slot `S` is known (section 5): `cell.set_menu(S, Arc::new(tree))`.
3. For each menu slot not set in step 2: `cell.clear_menu(S)`. Skipping this leaves a stale tree on the next frame.
4. Keep the cell clone from `UiThemes::frame()`. Nothing else changes: `ScreenLogic`, `Cx`, `Command`, the stack and the
   input path stay as they are. Interaction stays native (the adapter draws only).

## 5. Slot bookkeeping T12 must add

`Screens::open` is the only place `Args` are known. `Screen` holds `id`, `name` and `logic` only, so the slot is lost
after `open`. T12 adds `slot: Option<SlotId>` to `Screen` (in `frontend/screens.rs`) and sets it in `open` from a new
`factory::slot_for(name, &args)` that mirrors the table in section 3. `open_with` (dialogs, alerts) sets `None`, since
they have no menu slot. `pop` and the stack commands need no change.

## 6. Open points for T12

- **Two stacked screens on one slot.** The cell holds one tree per slot. Two stack screens can map to one slot (for
  example two option tables), and only one tree fits. Recommended: the topmost screen wins the slot; the presenter keeps
  drawing every screen natively, so nothing is lost until a theme overrides the slot. Decide before wiring Custom.
- **Parity.** The Original menu path draws the same tree the presenter draws, so pixels should match. The digest or
  offscreen check for a menu screen (`UiScript`, `--ui-script`) is owed to T12 (ui-themes.md section 13).
- **Clearing** (section 4, step 3) is the easiest thing to miss, and the failure is silent.

## 7. Tests (T8)

- `original/menu_tests.rs` (9 tests, no GPU): each menu and career slot emits its own synthetic tree and no other slot's;
  an empty slot emits nothing; `clear_menu` empties one slot and leaves the others; a second `set_menu` replaces the
  tree; an adapter resolved before a set reads the set tree; the HUD and menu cells do not feed each other; two menu
  trees compose with their links intact; `FrameCell` is `Send + Sync`.
- `original/tests.rs::every_slot_has_an_original_adapter`: the every-slot invariant, the flipped T7 test.
- `ui/theme/mod.rs::build_themes` runs the floor check on every construction, so `ui/theme/tests.rs` covers it.
