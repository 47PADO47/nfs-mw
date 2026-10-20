# UI themes: component model, slots and the theme switch

Design of the themeable UI: one component system that draws the HUD and the menus, a `UiStyle` setting that
selects a theme, and an Original floor that every theme falls back to. The architecture decision and its
trade-offs are in [ADR 0008](../decisions/0008-ui-themes.md). This spec gives the contracts the code must meet.

- **Sources read:** this repository only: `libs/blackbox-feng/src/tree/mod.rs`, `crates/nfsmw/src/ui/present/`,
  `crates/nfsmw/src/hud/` (plugin, state, bind, minimap, pursuit_meter, blips, blip_style, radio, trax,
  busted_overlay, viewport), `crates/nfsmw/src/frontend/` (screens, icon_menu, widget_menu, options,
  options_rows), `crates/nfsmw/src/settings/` (ui_style, radio_hud, appearance, file, env, partial, write),
  `crates/nfsmw/src/devtools/mod.rs`, [ADR 0002](../decisions/0002-ui-presentation.md),
  [ADR 0004](../decisions/0004-swappable-renderers.md), [ADR 0005](../decisions/0005-native-mod-plugins.md),
  [feng-runtime.md](feng-runtime.md), [frontend-menus.md](frontend-menus.md), [hud-viewport.md](hud-viewport.md).
- **Data inputs:** none new. The Original theme reads the same FEng packages and HUD and menu state as today.
  No game data and no decompiled code is used in this spec.
- **Status:** design. The component model (`libs/blackbox-ui`), the MW slot catalogue and theme scaffolding
  (`crates/nfsmw/src/ui/theme/`, T6), the Original HUD adapters (`original/`, T7) and the Original menu adapters and
  menu cell (T8, [ui-themes-menus.md](ui-themes-menus.md)) and the Custom `RadioCard` with the egui radio retired (T9,
  [ui-themes-custom.md](ui-themes-custom.md)) are built. Every catalogue slot has an Original adapter.
  Since T12 the HUD and menu presenters draw through the registry (the composer, [ui-themes-compose.md](ui-themes-compose.md)).
  Items marked **open** are not decided (section 14).

## 1. What a theme controls

A theme chooses the look of a set of **slots**. A slot is one replaceable element: a gauge, the minimap, the
radio card, a menu screen, an option table. For each slot a theme either returns a component that draws it,
or returns nothing. When it returns nothing, the Original theme draws the slot.

A theme does not own the data. Game code still speaks `HudState` and the screen logic traits. It never names
FEng, the presenter or a theme. Themes read the same state the Original code reads.

## 2. The neutral draw tree

The tree is the only output a theme produces. It lives in `libs/blackbox-ui/src/tree.rs` (moved from
`libs/blackbox-feng/src/tree/mod.rs` in task T3). `blackbox-feng` re-exports the three types, so existing paths still compile.
The FEng producer, `Runtime::tree`, stays in `blackbox-feng`.

```rust
pub struct UiTree { pub nodes: Vec<UiNode>, pub draw_order: Vec<usize> }
pub struct UiNode {
    pub index: usize, pub guid: u32, pub name_hash: u32, pub parent: Option<usize>,
    pub kind: NodeKind, pub text: Option<String>,
    pub local_position: Vec3, pub position: Vec3, pub local_pivot: Vec3, pub local_rotation: Quat,
    pub size: Vec3, pub colour: [u8; 4], pub world_colour: [u8; 4], pub world: Mat4,
    pub z: f32, pub visible: bool, pub clip: Option<[f32; 4]>,
}
pub enum NodeKind {
    Group,
    Image { texture: u32, uv: [f32; 4], mask: Option<u32>, mask_rotation: [f32; 3], mask_uv: [f32; 4] },
    Text { font: u32, justification: u32, leading: i32, max_width: i32 },
    Other,
}
```

Rules for the tree, which every producer keeps:

- `draw_order` lists the visible drawable nodes, farthest first. Later entries draw on top. The order is stable.
  The producer decides which nodes are drawable when it calls `DrawList::into_tree(drawn)`; `blackbox-ui`
  holds no default rule.
- `world` is the full transform. A presenter does not multiply parents.
- `world_colour` is the colour after parent tinting. Producers set it; the presenter does not re-derive it.
- `Image` with a `mask` is a gauge or a sweep: the game turns `mask_rotation` to fill it. `Gauge` (section 5)
  emits this form, so the tachometer mechanic is one primitive.
- `texture` and `font` are keys into the asset packs (`UiAssets`), not file names.

The presenter (`crates/nfsmw/src/ui/present/blackbox.rs`) consumes a `UiTree` and nothing else. It is not changed
by this design.

## 3. The `Component` trait and `DrawList`

```rust
pub trait Component {
    fn layout(&mut self, cx: &LayoutCx, bounds: Rect);
    fn build(&self, out: &mut DrawList);
    fn handle(&mut self, _cx: &mut EventCx, _ev: &InputEvent) -> bool { false }
}
```

- `layout` places children inside `bounds`. Units are the FEng space: 640 by 480, origin at the centre, as in
  [feng-runtime.md](feng-runtime.md). Components never apply the HUD viewport themselves. `hud/plugin.rs`
  already calls `hud.viewport.apply(&mut tree, screen, settings.hud_layout)` after the tree is built (see
  [hud-viewport.md](hud-viewport.md)); a component tree goes through the same call.
- `build` appends the nodes for the current state. It emits `UiNode`s with parent links and `draw_order` entries.
- `handle` returns `true` when the component used the event. The default ignores events.
- `DrawList` wraps the growing `Vec<UiNode>` in insertion order. `into_tree(drawn)` turns it into a `UiTree`:
  the nodes keep their order, so `parent` indices stay valid, and `draw_order` is built from `drawn`.
- `Rect`, `LayoutCx`, `EventCx` and `InputEvent` are small generic types in `blackbox-ui`. `InputEvent` is
  a neutral event, not the FEng message type (see [feng-input.md](feng-input.md) for the FEng side).

**Open: state updates.** `build(&self)` cannot advance a FEng runtime, because its scripts and messages need
`&mut`. The Original adapters (section 8) must advance their packages once per frame. This spec proposes that
state advances in `layout` (which takes `&mut self`) or in a separate per-frame `update(&mut self, dt)`.
Section 14 records it. Until it is settled, no component may rely on `build` changing state.

## 4. `Style` and `Skin`

A `Skin` is a bag of values a theme provides to base components: colours, font keys, texture keys and metrics
(spacing, row height, gauge pivot). Base components read the skin, so one `Row` looks different under a different
theme with no new code. A component never hard-codes a colour that a skin could supply.

`Style` is the per-component choice made from the skin (for example "the highlighted row"). It is a small
value type, copied, not shared.

## 5. Base components

These are generic, implemented once in `blackbox-ui` (`src/components/`, one file each) and emit `UiNode`s through
`DrawList`. Each has unit tests for layout and for the nodes it builds. The first five are built (T4), and so are
the three menu components (T5).

| Component | Purpose | Emits | Used by (examples) |
|---|---|---|---|
| `Panel` | A solid or textured rectangle, colour from the skin. No border yet | one `Image` (texture 0 when untextured) | cards, overlays |
| `Stack` | Lays children in a row or a column, equal shares with a gap (fixed, or a skin metric) | nothing itself; each child builds its own | any list |
| `Text` | One string in one skin font, with justification flags (FEng bits) and a max width | one `Text` | labels, values |
| `Image` | One textured quad over a UV rectangle, filling its rectangle, with a tint | one `Image` | icons, backdrops |
| `Gauge` | A dial with a rotating mask fill and an optional needle | one `Image` with `mask` (plus one needle `Image`) | tachometer, speed bar |
| `Table` | Vertical list of rows at a fixed height each, with an optional header and a focused row | nothing itself; each row builds its own (plus the focus `Panel`) | option tables, lists |
| `Row` | One table row: title in the left column, any control in the right | the title's `Text` and the control's nodes | option rows |
| `Switch` | A control that cycles through values (on/off, arrow cycle) | `Text` for the selected label, plus arrow `Image`s | settings toggles |

The menu components `Switch`, `Row` and `Table` are built (T5). `Row` and `Switch` emit `Text`/`Image` nodes
directly, and `Table` emits no node of its own (its focus highlight is a `Panel`). Table rows stack at a fixed
height, not equal shares. Focus moves with pad Up and Down, and pad Left and Right go to the focused row's
control. Details and the API are in `libs/blackbox-ui/README.md` ("Menu components").

`Gauge` is the generalised form of the tachometer mechanic. Its fraction (0..1) maps to the mask angle as
`mask_rotation = [pivot_x, pivot_y, fraction * sweep_deg]`, with `sweep_deg` 360 by default. The mask fields of
`NodeKind::Image` already carry what it needs; no new node kind is added. The needle, when present, is a second
`Image` node turned by the same angle. Its sign convention is still to be checked against the presenter.

Shared helper: `Rect::world(z)` gives the world transform of the unit quad (spanning -0.5 to 0.5) scaled to the
rectangle and centred on it. Components draw at `z` = `DEFAULT_Z` (1.0) unless set; larger `z` is farther.

## 6. Element slots

A slot is one replaceable element. Slot ids are stable, never reused, and small (see section 12 for the mod
side). The table lists the candidate catalogue from the plan. "Original code today" is where the Original theme
draws the slot now; entries marked **?** are not yet confirmed in the code and are to be checked in the task
named.

The catalogue below is final for this milestone (T6). Each slot's number is explicit in
`crates/nfsmw/src/ui/theme/element.rs` and pinned by a test in `crates/nfsmw/src/ui/theme/tests.rs`. Numbers are part
of the mod ABI: never reused, never renumbered. "Original source" was confirmed in the code in T6; the `?` entries of
the T1 draft are resolved here.

**HUD (state-fed):**

| Slot (id) | Original source (confirmed in code) |
|---|---|
| `Tachometer` (1) | Group guid `0x5164D4EA` (`TACHOMETER`, `hud/elements.rs`) in `HUD_SingleRace.fng`. Needle `3rdPersonNeedle`, face `TAC_Lines_7500`, redline `RPM_REDLINE`, shift light `Shift_light`, fill guid `0x456E`. Bound by `HudBinding::tachometer` (`hud/bind.rs`) |
| `Speedometer` (2) | Group guid `0x941FFF09` (`SPEEDOMETER`). Digits `SPEED_DIGIT_1..3`, units `3rdPersonSpeedUnits`. Bound by `HudBinding::speedometer` |
| `GearIndicator` (3) | Object `3rdPersonGear`, inside the tachometer group. Text and shift-time alpha set in `HudBinding::tachometer`. **Confirmed** (was `?`) |
| `Minimap` (4) | `hud/minimap.rs` (`GROUP`, `BACKING`, `ARROW`), shown by `hud/elements.rs`, placed by `hud/viewport.rs` ([hud-minimap.md](hud-minimap.md)) |
| `Blips` (5) | `hud/blips.rs`; styles from `hud/blip_style.rs` and `hud/blip_styles.rs` |
| `PursuitMeter` (6) | `hud/pursuit_meter.rs` (meter guid `0x7B422BA3`, cool-down guid `0x84A226EC`). The pursuit board (timer and cops count, groups `0xDE89E070` and `0xE0B1430B`) is bound in `HudBinding::pursuit`. **Decided (T7):** the board is part of this slot (section 8, slot map) |
| `Nitrous` (7) | Group guid `0x87C38E97`, bar guid `0xEDFB6D37` (`nos_bar`), icon guid `0x27DDF583` (`nos_icon`, scripts `SCRIPT_NOS_BURNING` and `SCRIPT_NOS_READY`). Bound by `HudBinding::nitrous`. **Confirmed** (was `?`) |
| `Turbo` (8) | Group `TURBO_GROUP` (`hud/elements.rs`), needle `3rdperson_TurboDial`. Bound by `HudBinding::turbo`. **Confirmed** (was `?`) |
| `Speedbreaker` (9) | `hud/speedbreaker.rs`: icon `SPEEDBREAKER_METER_ICON_GROUP`, group `Speedbreaker_Meter_Group`, mask `Speedbreaker_Meter_Multi_Image`, linear group guid `0x82D60021`, bar guid `0x1FDAF669`. **Confirmed** (was `?`) |
| `RadioCard` (10) | Original: the EA Trax chyron, `hud/trax.rs`, package `EA_TRAX.fng` (`GLOBAL/GlobalB.lzc`). Custom: `ui/theme/custom/radio_card.rs`, fed by `hud/radio.rs` data (built in T9, [ui-themes-custom.md](ui-themes-custom.md)) |
| `BustedOverlay` (11) | `hud/busted_overlay.rs`, package `BUSTED_OVERLAY.fng` (`GLOBAL/InGameB.bun`), loaded by the HUD while `HudState::busted_overlay` is set. **Confirmed** (was `?`) |

**Front end (screen-fed, backed by the existing `ScreenLogic` contract):**

| Slot (id) | Original source (confirmed in code) |
|---|---|
| `MainMenu` (20) | `MainMenu.fng` → `IconMenu` `Kind::Main` (`frontend/factory.rs`, `frontend/icon_menu.rs`) |
| `Categories` (21) | `MainMenu_Sub.fng` → `IconMenu` `Kind::Categories { pause: false }`. `Pause_Main.fng` with `Args::options` → `Kind::Categories { pause: true }`. **Confirmed** (was `?`). Shares a package with `SafeHouseMenu`: see section 8 |
| `PauseMenu` (22) | `Pause_Main.fng` without `Args::options` → `IconMenu` `Kind::Pause`. **Confirmed** (was `?`) |
| `OptionTable` (23) | `Options.fng` and `Pause_Options.fng` → `WidgetMenu` (`frontend/widget_menu.rs`), rows from `frontend/options_rows.rs`, settings from `frontend/options.rs` |
| `TrackSelect` (24) | `Track_Select.fng` → `frontend/track_select.rs` (Quick Race, step one; the track list is not populated yet). **Confirmed** (was `?`) |
| `SafeHouseMenu` (25) | `MainMenu_Sub.fng` with `Args::safehouse` → `frontend/career_screens/safehouse_menu.rs` → `IconMenu` `Kind::SafeHouse`. **Confirmed** (was `?`) |
| `WorldMapChrome` (26) | No Original source. The slot resolves to `Empty` under every theme; it is a flag, not a drawn component. When the active theme provides it (Custom), the frontend recolours the live `WorldMapMain.fng` screen dark in place (`world_map/screen/restyle.rs`), keeping its fullscreen map, filter switch, button prompts and mouse behaviour. Not menu-cell-fed (`is_menu()` excludes it). See [world-map.md](world-map.md) |

**Career screens (`frontend/career_screens/`):**

| Slot (id) | Original source (confirmed in code) |
|---|---|
| `Blacklist` (40) | `SafeHouseRivalChallenge.fng`, `SafeHouseRivalBio.fng` → `career_screens/blacklist.rs` |
| `CarLot` (41) | `Car_Select.fng` → `career_screens/car_lot.rs` |
| `RapSheet` (42) | `RapSheetLogin.fng`, `RapSheetLogin2.fng`, `RapSheetMain.fng`, `RapSheetUS.fng`, `RapSheetVD.fng`, `RapSheetCTS.fng`, `RapSheetRankings.fng`, `RapSheetRankingsDetail.fng`, `RapSheetRS.fng`, `RapSheetTEP.fng`, `RapSheetPD.fng` → `career_screens/rap_sheet/`. Only once `unlocked()` |
| `RegionUnlock` (43) | `SafehouseRegionUnlock.fng` → `career_screens/region_unlock.rs` |
| `MemoryCard` (44) | `MC_Main.fng`, `MC_List.fng` → `career_screens/save.rs` |
| `Sms` (45) | `SMS_Mailboxes.fng`, `SMS_Message.fng` → `career_screens/sms.rs` |

**Screens with no slot yet.** These exist but are not in the catalogue, because no theme needs to override them yet
(the rule "each slot needs a consumer"): `Track_Options.fng` (`track_options.rs`), `Options_PC_Controller.fng` and
`Pause_PC_Controller.fng` (`controls_screen.rs`), `Dialog.fng` (`confirm.rs`), `Credits.fng`, `EA_Trax_Jukebox.fng`
(`jukebox.rs`), `Infractions.fng`, `WS_Loading.fng` (`loading.rs`). Add a slot when
a theme or a mod needs it, with its number from the next free block.

**Slot kinds.** HUD slots receive a snapshot of `HudState` through `BuildCx`. Front-end slots receive the screen's
data through the same `ScreenLogic` values the Original code reads today. Neither kind exposes FEng types.

**Where `ElementId` lives (decided, T6).** `blackbox-ui` defines the generic `SlotId(u32)` (`libs/blackbox-ui/src/slot.rs`).
`crates/nfsmw/src/ui/theme/element.rs` defines the MW `ElementId` enum with explicit numbers, and `ElementId::slot()`
converts it to a `SlotId`. No MW name is in `libs/`.

## 7. `Theme`, `Themes` and the `resolve` fallback

```rust
pub trait Theme {
    fn id(&self) -> ThemeId;                       // Original | Custom | Remastered | Mod(u32)
    fn element(&self, slot: ElementId, cx: &BuildCx) -> Option<Box<dyn Component>>;
}

pub struct Themes { active: ThemeRef, original: ThemeRef }   // original is always present

impl Themes {
    pub fn resolve(&self, slot: ElementId, cx: &BuildCx) -> Box<dyn Component> {
        if let Some(component) = self.active.element(slot, cx) {
            return component;
        }
        self.original.element(slot, cx).expect("Original fills every slot (checked at start-up)")
    }
}
```

Rules:

- The Original theme must fill every slot. `Themes::new` in `libs/blackbox-ui` cannot check this: it does not know the
  MW catalogue, so it only checks the floor's id. The check lives MW-side (T8): `ui::theme::build_themes` asserts it at
  start-up, and `original/tests.rs::every_slot_has_an_original_adapter` runs it for every `ElementId::ALL` entry. So the
  `expect` in `resolve` is a checked invariant, not a hope. (**Design choice**: a generic check that returns an error
  instead of a panic could be added to `blackbox-ui`; left as is.)
- The active theme is chosen from `UiStyle` (section 11). A mod theme is registered with `Themes` and selected the
  same way.
- `resolve` is called when a slot is built, not for every frame's draw calls. A theme returning `None` costs one
  lookup.
- `ThemeId::Remastered` is a valid active theme with no overrides. Every slot resolves to Original. The option
  row shows it, but the look is Original until a Remastered theme exists. **Open:** whether to show it at all
  before then (section 14).
- `BuildCx` is a read-only view: the current `HudState` snapshot, the settings subset the components need, the
  screen size and the asset handles. It holds no FEng types and no renderer handle.

The shape copies `BlipStyle` (`crates/nfsmw/src/hud/blip_style.rs`: a named trait with defaults, registered at
run time) and the mod `InterfaceRegistry` (`libs/blackbox-modding/src/host/registry.rs`: ports keyed by id).

## 8. The Original theme: delegation, not re-authoring

The Original theme implements every slot. Each implementation is an **adapter** over code that already exists:

- **HUD adapters** own the existing HUD runtime for their objects. They do not run a second copy of the package
  scripts. A `Tachometer` adapter reads the same `HudBinding` values the current `bind` sends, and writes the
  same node data the current `HudViewport` reads. Output is the existing `UiTree` for those objects.
- **Front-end adapters** wrap `ScreenLogic`, `WidgetMenu`, `IconMenu` and `options_rows`. The FEng package runs as
  today; the adapter reports the tree of its own package to the presenter.
- The adapter is what makes the Original look pixel-identical: the same packages, the same scripts, the same
  viewport math, the same draw order.

**Constraint.** The adapters must not change the FEng object tree or the viewport math. Those already match the
original game (see [hud-viewport.md](hud-viewport.md) and [frontend-menus.md](frontend-menus.md)). Tests must
show the tree is unchanged before and after each slot is wrapped (the digest tests in
`libs/blackbox-render/src/gpu/parity/digests/`).

### Adapter design: emitting the FEng subtree (HUD built in T7; menus in T8)

The HUD adapters are built (T7, `crates/nfsmw/src/ui/theme/original/`). The menu adapters are built too (T8): they emit
the whole tree of their screen from a second cell, the menu cell (see [ui-themes-menus.md](ui-themes-menus.md)). The
design:

1. **One producer.** The owning plugin (`hud/plugin.rs`, `frontend/plugin.rs`) advances its package (scripts,
   messages) and calls `Runtime::tree()` once per frame, as it does today. Adapters only read. They never run a
   second copy of a package.
2. **One adapter per slot, over one FEng subtree.** A slot's objects are a root group and its children (section 6).
   The adapter's subtree is the nodes of that frame's `UiTree` under the root. Node indices belong to one tree, so the
   adapter copies its nodes and remaps each `parent` into the index space of its own `DrawList`.
3. **How the frame's nodes reach `build`. Decided (T7).** `build(&self)` takes no runtime (section 3, decision 2 in the
   plan), and `LayoutCx` carries only the skin and the clock, so neither can hand over the tree. The route is a shared
   **frame cell**, `FrameCell` (`original/frame.rs`): one `Option<Arc<UiTree>>` per package (`FengSource::{Hud, Chyron,
   BustedOverlay}`), behind an `Arc<Mutex<..>>`. The plugin fills it with `FrameCell::set` once per frame; `clear`
   empties a package that is not drawn. The adapters read it in `build`, so an adapter resolved once keeps reading the
   current frame.
   - **Why `Arc<Mutex>` and not `Rc<RefCell>`.** `Theme` is `Send + Sync` (`blackbox_ui::Theme`), and `OriginalTheme`
     holds the cell, so the cell must be thread-safe. The lock is taken once per adapter per build. A poisoned lock is
     read through (`PoisonError::into_inner`): each write replaces a whole slot, so the data stays valid.
   - **What the plugin stores.** The tree as the presenter gets it: `hud.viewport.apply` applied for the HUD, and
     `trax.place` for the chyron. Adapters then copy the placed `world` transforms as they are.
   - **Who fills it.** `UiThemes::frame()` gives the plugin its clone. The HUD and frontend plugins fill it each frame
     (T12, [ui-themes-compose.md](ui-themes-compose.md) section 2).
4. **One composed tree.** The presenter sees one `UiTree`. The plugin composes the slot subtrees in the FEng node
   order, so `draw_order` matches today's, and calls `hud.viewport.apply` once on the composed tree, not per slot
   (section 5, [hud-viewport.md](hud-viewport.md)). Pixel parity is checked with the digest tests.
5. **Argument-dependent packages.** `MainMenu_Sub.fng` serves `Categories` and `SafeHouseMenu`, and `Pause_Main.fng`
   serves `PauseMenu` and `Categories` (pause). An adapter must test the same `Args` that `frontend/factory.rs` tests,
   so a package is never drawn by two slots at once.
6. **Stateful pieces stay put.** Per-frame state that a component owns (fade timers, needle easing) moves into
   `layout(&mut self, ..)` only where it needs no FEng data. Anything that needs the package stays in the plugin.

**Check (done in T8).** The floor fills every slot. The check is MW-side, because `Themes::new` in `libs/blackbox-ui`
cannot know the catalogue (section 7). `build_themes` asserts it at start-up, and `every_slot_has_an_original_adapter`
(`original/tests.rs`) tests it for every slot. The T7 test that pinned "menus are `None`" was flipped to this.

### What T7 built (HUD adapters)

- **`FengSubtree`** (`original/subtree.rs`), an `impl Component`. It holds the cell, a `FengSource`, and a scope:
  `under(frame, source, roots, excluded)` for an element's root groups, or `package(frame, source)` for a whole package
  (the chyron and the overlay). `layout` is a no-op. `build` copies the nodes of its scope from the cell's tree for its
  source and does nothing when that source is empty.
- **Copy rules.** A node is in scope when walking up from it (itself included) meets a root name before an excluded
  name. The walk is by name hash (`UiNode::name_hash`), so the rule does not depend on how the tree was built. A root
  that is absent, or not `visible`, takes its subtree with it, so nothing is emitted. Nodes keep the tree's order (the
  package's object order, parents first). `parent` is remapped into the draw list's index space, starting at
  `out.len()`, so several adapters can append to one list. A root's parent lies outside its subtree, so the root gets
  `parent: None`. `index` is kept: it stays the package object index, not the list position.
- **Draw order.** The adapter does not sort. The composer calls `DrawList::into_tree`, a stable sort by `z`. Because
  the nodes come in object order, a single slot's draw order equals FEng's `draw_order` restricted to that slot (tested).
  Across slots, ties in `z` keep the composer's insertion order. To match FEng exactly, the composer should append the
  slots in object-index order, or sort by `index` before `into_tree`. Decided (T12): sort by `index`
  (`DrawList::sort_by_index`), because the tachometer nests the gear.
- **Root lists** (`hud/roots.rs`). Each list is the same list `hud/elements.rs::Layout::update` uses for the element,
  so an adapter emits exactly what the HUD shows for it. All of them are name hashes (`Runtime::find`). The T6 table
  says "guid" for the group identities. That is wrong for every entry except the tachometer's fill (`0x456E`, a guid,
  not used as a root). `elements.rs` and `bind.rs` look the others up by name.

### Slot to adapter map (`original/hud_slots.rs`)

| Slot | Source | Roots (name hashes) | Excluded |
|---|---|---|---|
| `Tachometer` | Hud | `TACHOMETER` (`0x5164D4EA`) | the gear (`3rdPersonGear`) |
| `Speedometer` | Hud | `SPEEDOMETER` (`0x941FFF09`) | none |
| `GearIndicator` | Hud | `3rdPersonGear` | none |
| `Minimap` | Hud | `TRACK_MAP`, `TRACKMAPTARGETRING`, `PLAYERCARINDICATOR` | the blip icons |
| `Blips` | Hud | every blip pool icon (`hud/roots.rs::blip_icons`) | none |
| `PursuitMeter` | Hud | `PURSUIT_BOARD` (`0xDE89E070`, `0x7B422BA3`), `PURSUIT_COPS` (`0xE0B1430B`), `COOLDOWN` (`0x84A226EC`) | none |
| `Nitrous` | Hud | `NITROUS` (`0x87C38E97`, `0xEDFB6D37`) | none |
| `Turbo` | Hud | `TURBO_GROUP` | none |
| `Speedbreaker` | Hud | `SPEEDBREAKER_METER_ICON_GROUP`, `Speedbreaker_Meter_Group`, `Speedbreaker_Meter_Multi_Image`, `0x82D60021` | none |
| `RadioCard` | Chyron | whole package (`EA_TRAX.fng`) | none |
| `BustedOverlay` | BustedOverlay | whole package (`BUSTED_OVERLAY.fng`) | none |

Menu and career slots (raw 20 and up) are not in this table. Their adapters, the menu cell and the T12 fill map are in
[ui-themes-menus.md](ui-themes-menus.md).

**Decisions** (each one keeps every HUD node in at most one slot):

- **Gear inside the tachometer.** The tachometer excludes the gear's subtree, and the gear slot emits it. Replacing the
  tachometer keeps the gear unless the gear is replaced too.
- **Blips inside the minimap.** `elements.rs` hides every object that is not under one of its roots, and the blip pool
  is shown. So the blip icons sit under a minimap root (`TRACK_MAP`, most likely, or one of the other two). The minimap
  excludes them, and the blips slot emits them. The exclusion holds wherever they sit under those roots.
- **Pursuit board in `PursuitMeter`.** The board (`PURSUIT_BOARD`, timer and cops count) and the meter and cool-down
  bar are one slot. They are bound together, and show only during a chase. A split would add a slot with no consumer.
  The bounty row is in neither the catalogue nor this slot, unless it sits under the board group. Not checked.
- **Radar cool-down.** The cool-down bar (`0x84A226EC`) sits inside the radar's group in the package, but it is a root
  of `PursuitMeter`. Its membership is decided by its own root, so it is not lost.
- **Not in any slot (decided in T12).** The rear-view mirror (`0x9EE06631`, `0x7FAC287B`), the police radar and its laser
  (`0x839E7D77`, `0x062743F5`, `0x6CCBBBF1`), the heat arcs (`0xC46A80A9`), and the bounty row. No slot takes them, so the
  composer appends every node no slot claims (`hud_slots::emit_unclaimed`). No new slot is needed yet.
- **Unchecked assumption.** Only the gear is known to sit inside another slot's root (the tachometer), and the blip icons
  sit under the minimap. The synthetic tests check that no node is claimed twice. The real-package check is owed
  ([ui-themes-compose.md](ui-themes-compose.md) section 7).

**Ghidra was not used.** Every identity above was already pinned by the Rust code (`elements.rs`, `bind.rs`,
`speedbreaker.rs`, `minimap.rs`), so no new original-game fact was needed.

## 9. The Custom theme: native components

`Custom` implements `RadioCard` and returns `None` for every other slot. The card is built (T9) and described in
[ui-themes-custom.md](ui-themes-custom.md): a `Panel` and three `Text` rows, emitted as neutral nodes. It draws through
the presenter, and nothing is drawn in egui. It reads its state from a feed the plugin fills each frame (T12), and
places itself from the HUD layout, so the composer does not apply the HUD viewport to it.

The egui branch in `crates/nfsmw/src/devtools/mod.rs` (the `settings.radio_hud == RadioHudStyle::Custom` check and
the call to `crate::hud::show_radio`) is removed. T9 checked that the metrics and console overlays do not depend on
that branch: they draw as before.

## 10. Remastered: extension point and worked example

`Remastered` is an enum variant and a documented extension point. It is not implemented in this milestone.

Worked design note, to show the component set is enough. The example is a fullscreen minimap:

- The **original map** is drawn fullscreen behind everything, as a `Panel` with an `Image` child that uses the
  existing map tiles (`hud/minimap.rs`, [hud-minimap.md](hud-minimap.md)).
- The **legend** sits on the right, as a `Stack` of `Row`s, each `Text` plus `Image` (the blip icons from
  `hud/blip_style.rs`).
- The `Minimap` slot in the Remastered theme returns that `Panel`. The Blips slot can still return `None`, so the
  original blips draw on the new map, or it can return a component of its own.

The example needs only `Panel`, `Stack`, `Row`, `Text` and `Image`. No new base component and no new node kind.

Built since: the Remastered cluster, the map legend, the vertical icon menus ([ui-themes-remastered-menus.md](ui-themes-remastered-menus.md)), which added `Slant` and `Outline` to the base components, and the dialogs, Blacklist, minimap with its heat ring and radio card ([ui-themes-remastered-screens.md](ui-themes-remastered-screens.md)), which added `Ring`.

## 11. The `UiStyle` setting

Built in T10. `enum UiStyle { Original, Custom, Remastered }`, default `Original`
(`crates/nfsmw/src/settings/ui_style.rs`): the three HUD choices. The palette of the token-driven HUD is the separate
`UiTheme { Remastered, Bevy }` setting (`settings/ui_theme.rs`), which applies only when the HUD is not Original (section 15).

- Modelled on `VisualStyle` (`crates/nfsmw/src/settings/appearance.rs`): kebab-case serde, `names!` for the strict
  `FromStr` and `Display`, `ALL`, `label()`. The serialised names are `original`, `custom` and `remastered`.
  Labels: `Original`, `Custom`, `Remastered`. `VisualStyle` defaults to `Custom`; `UiStyle` defaults to
  `Original`, because the UI look and the graphics look are separate choices (decision 6).
- **Layers**, each read like the other settings (CLI over env over file over default):

| Layer | Where | Key |
|---|---|---|
| TOML | `settings/file.rs` | `ui_style = 'original' \| 'custom' \| 'remastered'` |
| Env | `settings/env.rs` (`UI_STYLE`) | `NFSMW_UI_STYLE` |
| CLI | `cli/view_args.rs` | `--ui-style original\|custom\|remastered` |
| Partial | `settings/partial.rs` | `ui_style: Option<UiStyle>` |
| Write | `settings/write.rs` | writes `ui_style` only |
| Console | `devtools/console/settings_cmd.rs`, `settings_keys.rs`, `parse.rs` | `set ui_style <value>` |
| Options | `frontend/options.rs`, `options_rows.rs` | row "HUD style" cycles the three values; row "HUD theme" cycles `remastered` and `bevy` |

- **Legacy `radio_hud` (deprecated, read only).** The old key `radio_hud` (`ea_trax` or `custom`, see
  `settings/radio_hud.rs`) still loads. `ea_trax` maps to `Original`, `custom` maps to `Custom`. The mapping is per
  layer: a layer that sets `ui_style` ignores its own `radio_hud`. The same applies to env `NFSMW_RADIO_HUD`, CLI
  `--radio-hud` (`--ui-style` wins if both are given) and the console alias `radio_hud`/`radio-hud`. Nothing writes
  the key any more. A file that still has it keeps it, and the new key takes priority.
  `RadioHudStyle` stays as the parser for that legacy value. It is no longer a settings field.
- **Live switch.** `ui::theme::sync_active_theme` runs in `FrameSet::Prepare` every frame and calls
  `UiThemes::select(settings.ui_style)`. An options change takes effect on the next frame, with no restart.
  `theme_of` maps `UiStyle` to `ThemeId`. `Remastered` has no registered theme yet, so `select` leaves Original
  active. The HUD and menu presenters resolve the slots each frame (T12).
- **Chyron gate.** `hud/plugin.rs` shows the EA Trax chyron only where the `RadioCard` slot resolves to Original
  (`UiThemes::provided_by`), so Custom's card replaces it and a mod theme that does not name the slot keeps it (T12).
- **Tests:** `settings/ui_style_tests.rs` (defaults, tokens, the three layers, legacy mapping, write, CLI),
  `settings/ui_style.rs` and `settings/radio_hud.rs` unit tests, `frontend/ui_style_tests.rs` (row title, the cycle and
  labels), `ui/theme/tests.rs` (`theme_of`, `select` fallback, the sync system).

## 12. The `bb.ui` mod hook

Tokens were added in `bb.ui` 1.1 (`set_tokens`, section 15).
Mirrors `bb.menus` ([ADR 0005](../decisions/0005-native-mod-plugins.md), sections 2 and 8). Delivered in T11 as
`bb.ui` 1.0. The catalogue for mod authors is [api-ui.md](../modding/api-ui.md).

- **Layers.** Host table `libs/blackbox-modding/src/host/table/ui.rs` (trait `UiBackend`, the primitive checks), ABI
  `libs/blackbox-modding/src/abi/iface/ui.rs` (`BbUi`, `BbUiPrim`), SDK `libs/blackbox-modding/src/sdk/ui.rs`, game port
  `crates/nfsmw/src/modding/backends/ui.rs` (`UiPort`, `theme_id`), mod themes `crates/nfsmw/src/ui/theme/modded/`.
- **Operations.** `register_theme(name, display)` returns a handle; `set_slot(theme, slot, prims)` gives a slot a draw spec
  (a second call replaces the first); `unregister_theme(theme)`. Themes a mod leaves registered are removed when it unloads.
- **Declarative, not components.** The C ABI cannot carry a `Box<dyn Component>`. A slot is a flat list of up to 64
  primitives, and the host builds them from its base components: `Panel` (fill colour, optional texture), `Text` (font,
  string, colour, justification) and `Image` (texture, uv). `Gauge` is owed. Colours are inline RGBA, not skin keys, so a
  mod cannot reach the game's skin. Fonts and textures are resource names, hashed as the packs name them.
- **Slots.** Only the catalogue (section 6, `ElementId`) can be overridden. A slot number outside it is refused. The
  numbers are ABI and never reused (`element.rs`).
- **Ids.** A mod theme's `ThemeId::Mod(n)` is a 32-bit FNV-1a of `owner/name`, so it is the same in every run. A
  collision is refused.
- **Resolution.** `ModTheme` implements `blackbox_ui::Theme`. `element(slot)` builds a `ModSlot` (its own skin, one
  component per primitive) for a named slot, and `None` otherwise, so the registry falls back to Original.
- **Sync.** `sync_mod_themes` copies `Mods::ui_themes()` into `UiThemes` every frame. An unchanged set is one comparison.
  A changed set rebuilds the registry, keeps the Original and Custom themes, and drops a chosen mod theme that is gone.
- **Selection.** `UiThemes::select_mod(id)` makes a mod theme active until `clear_mod()`; `select(style)` (the per-frame
  sync) keeps the mod choice. `set_mod_choice(choice)` applies the player's choice each frame: `Some(id)` selects it when
  registered (else gives it up), `None` clears it. `UiStyle` stays a closed enum (question 3).
- **Choice on the Mods page (T13).** The Mods page's first row, `UI theme`, is offered only when a mod registered a theme.
  It steps through the built-in style (shown as `Built-in style`) and then each mod theme in registration order, and
  wraps. Its value is `Mods::ui_theme_choice()`, kept in the bridge (`Bridge::ui_theme_choice`), which the frontend can
  write through `&Mods`. The sync reads it in `sync_mod_themes`. The row shows only with a mod theme, and the Mods page
  shows only when it has a row (`Mods::has_mods_page`). Code: `frontend/mod_options.rs`, `modding/backends/ui.rs`
  (`step_choice`, `chosen_display`).
- **Persistence (T13).** The choice is saved as the text `owner/name` of the chosen theme, or empty for the built-in style,
  in the config's `[mod_values.nfsmw] ui_style_mod` (through the same store as the `bb.menus` values, `menu_store.rs`).
  The stable identity is `owner/name`, not the id: the id is an FNV hash of it, so the two agree every run, but the text
  is what a config file can hold and read. When a theme registers, `register_theme` selects it if the saved text names it.
  A saved name that is absent leaves the built-in style; the saved value stays, so the theme is selected when it returns.
  A choice made in a run takes effect at once and is saved at once.
  - **Why not the `ui_style` layers.** The brief asked for `file`, `partial` and `write` keys like `ui_style`. `Settings`
    and `Partial` are `Copy`, and a `String` cannot sit in them. The mod values store is the game's only string-keyed,
    persisted store, so the choice lives there. Env and CLI keys for the choice are not added.
- **Drawn.** The presenter reads the registry through the composer (T12). A mod theme that is selected draws its slots.
- **Tests:** `host/table/ui_tests.rs` (the table: shape checks, lifecycle, offer), `sdk/ffi/ui_tests.rs` (the SDK round trip),
  `modding/backends/ui_tests.rs` (the port: register, override, build, fallback, names and ids), and
  `ui/theme/modded/tests.rs` (primitives, fallback, select and clear, the Mods page choice through `set_mod_choice` and
  the style sync), `modding/backends/ui_tests.rs` (the choice cycle, the saved choice on register and its absence),
  `modding/backends/menu_store.rs` (text values), and `frontend/mod_options_tests.rs` (the row offered only with a theme).

## 13. Testing and verification

- **`blackbox-ui` unit tests:** the resolver fallback (an empty theme falls back to Original for every slot; an
  override wins for its slot only; `Themes::new` refuses an incomplete Original).
- **Base components:** layout and built nodes for each of the eight components (T4 for the first five, built;
  T5 for the last three).
- **Original parity:** the HUD and front-end suites (`crates/nfsmw/src/hud/*_tests.rs`, `frontend/*_tests.rs`)
  stay green; `--ui-style original` offscreen screenshots match the pre-change output; the 256 by 144 digests hold.
- **Custom:** `--ui-style custom` draws the radio card through the presenter, not egui, and every other element
  looks like Original. Switching back restores the chyron. An old config with `radio_hud = custom` still selects
  Custom.
- **Settings:** `ui_style` parse and merge round trips, the `radio_hud` alias, the options cycle.
- **Mod hook:** registration and slot override through the fake host; an unknown slot is an error.

## 14. Open questions

These are not decided. The parent task (or the owner) decides them before the matching subtask starts.

1. **`ElementId` ownership.** Resolved (T6): generic `SlotId` in `libs/blackbox-ui`, MW `ElementId` in
   `crates/nfsmw/src/ui/theme/element.rs` (section 6).
2. **Mutable state in components.** `build(&self)` cannot advance FEng scripts (section 3). Choose between state in
   `layout(&mut self)` and a separate `update(&mut self, dt)`. Before T2.
3. **Mod themes in `UiStyle`.** Resolved (T11): mod themes stay out of the closed `UiStyle` enum and are selected with
   `UiThemes::select_mod` (section 12). The choice is on the Mods page and saved as `[mod_values.nfsmw] ui_style_mod`
   (T13).
4. **Components across the ABI.** Resolved (T11): a declarative primitive list, not handles (section 12).
5. **Live switch or restart.** Resolved (T10): live. `sync_active_theme` selects the theme every frame (section 11).

## 15. Theme tokens

The token model (colours, radius, padding, and how the registry merges them) is in [ui-themes-tokens.md](ui-themes-tokens.md).
