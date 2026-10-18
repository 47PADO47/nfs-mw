# UI themes: component model, slots and the theme switch

Design of the themeable UI: one component system that draws the HUD and the menus, a `UiStyle` setting that
selects a theme, and an Original floor that every theme falls back to. The architecture decision and its
trade-offs are in [ADR 0008](../decisions/0008-ui-themes.md). This spec gives the contracts the code must meet.

- **Sources read:** this repository only: `libs/blackbox-feng/src/tree/mod.rs`, `crates/nfsmw/src/ui/present/`,
  `crates/nfsmw/src/hud/` (plugin, state, bind, minimap, pursuit_meter, blips, blip_style, radio, trax,
  busted_overlay, viewport), `crates/nfsmw/src/frontend/` (screens, icon_menu, widget_menu, options,
  options_rows), `crates/nfsmw/src/settings/` (radio_hud, appearance, file, env, partial, write),
  `crates/nfsmw/src/devtools/mod.rs`, [ADR 0002](../decisions/0002-ui-presentation.md),
  [ADR 0004](../decisions/0004-swappable-renderers.md), [ADR 0005](../decisions/0005-native-mod-plugins.md),
  [feng-runtime.md](feng-runtime.md), [frontend-menus.md](frontend-menus.md), [hud-viewport.md](hud-viewport.md).
- **Data inputs:** none new. The Original theme reads the same FEng packages and HUD and menu state as today.
  No game data and no decompiled code is used in this spec.
- **Status:** design. The component model (`libs/blackbox-ui`) and the MW slot catalogue and theme scaffolding
  (`crates/nfsmw/src/ui/theme/`, T6) are built. Nothing draws through them yet. Items marked **open** are not decided
  (section 14).

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
| `PursuitMeter` (6) | `hud/pursuit_meter.rs` (meter guid `0x7B422BA3`, cool-down guid `0x84A226EC`). The pursuit board (timer and cops count, groups `0xDE89E070` and `0xE0B1430B`) is bound in `HudBinding::pursuit`. **Open for T7:** whether the board is part of this slot |
| `Nitrous` (7) | Group guid `0x87C38E97`, bar guid `0xEDFB6D37` (`nos_bar`), icon guid `0x27DDF583` (`nos_icon`, scripts `SCRIPT_NOS_BURNING` and `SCRIPT_NOS_READY`). Bound by `HudBinding::nitrous`. **Confirmed** (was `?`) |
| `Turbo` (8) | Group `TURBO_GROUP` (`hud/elements.rs`), needle `3rdperson_TurboDial`. Bound by `HudBinding::turbo`. **Confirmed** (was `?`) |
| `Speedbreaker` (9) | `hud/speedbreaker.rs`: icon `SPEEDBREAKER_METER_ICON_GROUP`, group `Speedbreaker_Meter_Group`, mask `Speedbreaker_Meter_Multi_Image`, linear group guid `0x82D60021`, bar guid `0x1FDAF669`. **Confirmed** (was `?`) |
| `RadioCard` (10) | Original: the EA Trax chyron, `hud/trax.rs`, package `EA_TRAX.fng` (`GLOBAL/GlobalB.lzc`). Custom: the card in `hud/radio.rs` (egui today, moves to the presenter in T9) |
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
(`jukebox.rs`), `WorldMapMain.fng` (`world_map/`), `Infractions.fng`, `WS_Loading.fng` (`loading.rs`). Add a slot when
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

- `Themes::new` refuses to build when the Original theme does not fill every slot. A unit test runs the same
  check, so the `expect` is a checked invariant, not a hope. (**Design choice**: the check could return an error
  instead; the choice is small and is left to T2.)
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

### Adapter design: emitting the FEng subtree (note for T7 and T8, not built)

The T6 scaffolding leaves `OriginalTheme::element` returning `None`. The adapters follow this design:

1. **One producer.** The owning plugin (`hud/plugin.rs`, `frontend/plugin.rs`) advances its package (scripts,
   messages) and calls `Runtime::tree()` once per frame, as it does today. Adapters only read. They never run a
   second copy of a package.
2. **One adapter per slot, over one FEng subtree.** A slot's objects are a root group and its children (section 6).
   The adapter's subtree is the nodes of that frame's `UiTree` under the root. Node indices belong to one tree, so the
   adapter copies its nodes and remaps each `parent` into the index space of its own `DrawList`.
3. **How the frame's nodes reach `build`. Open for T7.** `build(&self)` takes no runtime (section 3, decision 2 in the
   plan), and `LayoutCx` carries only the skin and the clock, so neither can hand over the tree. Recommended: the
   adapter holds a shared cell (for example `Rc<RefCell<Vec<UiNode>>>`, stored NonSend) that the plugin fills right
   after `Runtime::tree()`, and `build` copies from it. Any other route changes the `Component` contract, so T7 decides
   this before it writes the first adapter.
4. **One composed tree.** The presenter sees one `UiTree`. The plugin composes the slot subtrees in the FEng node
   order, so `draw_order` matches today's, and calls `hud.viewport.apply` once on the composed tree, not per slot
   (section 5, [hud-viewport.md](hud-viewport.md)). Pixel parity is checked with the digest tests.
5. **Argument-dependent packages.** `MainMenu_Sub.fng` serves `Categories` and `SafeHouseMenu`, and `Pause_Main.fng`
   serves `PauseMenu` and `Categories` (pause). An adapter must test the same `Args` that `frontend/factory.rs` tests,
   so a package is never drawn by two slots at once.
6. **Stateful pieces stay put.** Per-frame state that a component owns (fade timers, needle easing) moves into
   `layout(&mut self, ..)` only where it needs no FEng data. Anything that needs the package stays in the plugin.

**Check owed (T7 or T8).** `Themes::new` in `libs/blackbox-ui` checks only that the floor's id is `Original`. It does
not check that the floor fills every slot, which section 7 asks for. Once the Original theme fills slots, add a test
that `OriginalTheme::element` returns `Some` for every `ElementId::ALL` entry, and add the check to the constructor.

## 9. The Custom theme: native components

`Custom` implements `RadioCard` and returns `None` for every other slot. The card is today's `hud/radio.rs`
(title, artist, album and time) re-expressed with `Panel`, `Text` and `Stack`, emitting neutral nodes. It goes
through the same presenter and the same `hud.viewport.apply` call as the chyron. Nothing is drawn in egui.

When the card moves to the presenter, the egui branch in `crates/nfsmw/src/devtools/mod.rs` (the
`settings.radio_hud == RadioHudStyle::Custom` check and the call to `crate::hud::show_radio`) is removed. Task
T9 checks that metrics and console overlays do not depend on that branch.

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

## 11. The `UiStyle` setting

```rust
pub enum UiStyle { Original, Custom, Remastered }   // default Original
```

- Modelled on `VisualStyle` (`crates/nfsmw/src/settings/appearance.rs`: `names!`, `ALL`, `label()`). Note that
  `VisualStyle` defaults to `Custom`; `UiStyle` defaults to `Original` (section 14).
- **Folds in `radio_hud`.** The old enum is `RadioHudStyle { EaTrax, Custom }` (`crates/nfsmw/src/settings/radio_hud.rs`),
  its values written as `ea_trax` or `custom`. The key still reads (`ea_trax`, `ea-trax`, `eatrax` and `custom`
  are all accepted, as today): `custom` maps to `UiStyle::Custom`, `ea_trax` maps to `UiStyle::Original`. The
  `radio_hud` key is marked deprecated; it is written no more.
- **Serialised form.** Proposed: `original`, `custom`, `remastered`, with the underscore and hyphen forms both
  accepted on read, as `RadioHudStyle::from_str` does.

Layers, each read the same way as the other settings (CLI over env over TOML over defaults):

| Layer | File | Change |
|---|---|---|
| TOML | `settings/file.rs` | new `ui_style` key; `radio_hud` read for back-compat |
| Env | `settings/env.rs` | `NFSMW_UI_STYLE` (new constant beside `RADIO_HUD = "NFSMW_RADIO_HUD"`, which is kept) |
| CLI | `cli/view_args.rs` | `--ui-style` (new); `--radio-hud` kept (it sets `UiStyle` too) |
| Partial | `settings/partial.rs` | `ui_style: Option<UiStyle>` merged like `radio_hud` |
| Write | `settings/write.rs` | writes `ui_style`; stops writing `radio_hud` |
| Console | `devtools/console/settings_cmd.rs`, `settings_keys.rs`, `parse.rs` | `ui_style` key; `radio_hud` kept as an alias |
| Options | `frontend/options.rs`, `frontend/options_rows.rs` | `Setting::UiStyle` cycles `[Original, Custom, Remastered]` in place of `Setting::RadioHud` (the cycle at options.rs around line 328) |

The existing tests in `crates/nfsmw/src/frontend/radio_hud_tests.rs` move to a `UiStyle` test with the same
cases: default, cycle, and the `radio_hud` round trip.

The change in the options row is visible: "Radio HUD" becomes "UI style". Whether the switch takes effect at once
or after a restart is **open** (section 14).

## 12. The `bb.ui` mod hook

Mirrors `bb.menus` and `bb.hud` ([ADR 0005](../decisions/0005-native-mod-plugins.md), section 2 and section 8).

- Interface id `bb.ui`, version `1.0` (draft). A backend trait in `libs/blackbox-modding/src/host/table/` (name
  chosen in T11) and an `add_ui` port in `crates/nfsmw/src/modding/backends/`.
- **Proposed operations** (not final): register a theme (returns a theme handle); override a slot of a theme with a
  component (by handle); list the slot ids and their names; read the active `UiStyle`.
- Slot ids and their names are part of the ABI. Ids are `u32` constants that are never reused, and names are
  UTF-8 strings. A mod that asks for an unknown slot gets an error, not a silent no-op.
- A mod theme is selectable as a style once it is registered. How it appears in `UiStyle` is **open** (section 14).

**Components across the C ABI.** A mod cannot pass a `Box<dyn Component>`: ADR 0005 allows only `#[repr(C)]` data
and `extern "C"` functions across the boundary. The mod must describe its component tree through host calls that
return handles, and the host wraps those handles in `Component` values. The exact call set is **open**, and it is
the largest open point of the hook.

Tests follow `crates/nfsmw/src/modding/backends/menus_tests.rs`: a fake host registers a theme, overrides one
slot, and checks that `resolve` returns the mod component for that slot and Original for the rest.

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
3. **Mod themes in `UiStyle`.** The plan names `UiStyle::Mod(id)`. The settings pattern (closed enum, `ALL`, cycle
   in the options row, `names!`) does not fit a dynamic variant. Choose a cycle and a serial form, or keep mod
   themes out of `UiStyle` and select them another way. Before T10 and T11.
4. **Components across the ABI.** The declarative handle-based call set for the mod hook (section 12). Before T11.
5. **Live switch or restart.** Whether changing `UiStyle` re-resolves slots at once or applies at the next start.
   Before T10.
6. **Default.** `UiStyle` defaults to `Original`; `VisualStyle` defaults to `Custom`. Confirm the difference is
   intended. Before T10.
7. **Remastered in the menu.** Whether the Remastered row is shown before a Remastered theme exists, since it looks
   like Original until then. Before T10.
8. **Busted overlay and career screens.** Resolved (T6): confirmed in section 6. Still open for T7: whether the pursuit
   board belongs to `PursuitMeter`, and the adapter route for per-frame nodes (section 8, point 3).
9. **Milestone number.** The work has no milestone number in the plan. Set it when the subtasks are scheduled.
