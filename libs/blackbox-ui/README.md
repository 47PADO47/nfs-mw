# blackbox-ui

The themeable UI component model of the EA Black Box rewrite. It is engine-generic: no rendering, no
Bevy, no game data, no game names. A host builds a tree of components, lays it out in FEng units,
builds a flat draw list, and draws it. A theme registry decides which component fills each overridable
slot, and falls back to the Original theme when the active theme has no component for a slot.

This crate is the skeleton. It has the component trait, the support types, the slot names, the theme
registry and its fallback. Concrete elements (HUD parts, menus, the base components `Panel`, `Stack`,
`Text`, `Gauge`, `Table`, `Row`, `Switch`) come in later steps.

Design decision: `docs/decisions/0008-ui-themes.md`.

## Component model

- **`Component`** is the base trait with three methods:
  - `layout(&mut self, cx: &LayoutCx, bounds: Rect)` measures and positions the component inside
    `bounds`. A component caches what `build` needs here, because `build` takes no context.
  - `build(&self, out: &mut DrawList)` appends the draw nodes for the current state.
  - `handle(&mut self, cx: &mut EventCx, ev: &InputEvent) -> bool` handles one input event and returns
    whether it was consumed. The default consumes nothing.
- **`Rect`** is a rectangle in FEng units: a 640 x 480 screen with its origin at the centre. `x` and `y`
  are the centre; `Rect::SCREEN` is the whole screen.
- **`LayoutCx`, `EventCx`, `BuildCx`** borrow the active `Skin`. `LayoutCx` also carries `time_s`, the host
  clock, so a component can advance per-frame state (a fade, an easing) in `layout`. `EventCx` records whether
  the component asked for a redraw. Nothing in these contexts is a game or renderer handle.
- **`Skin`** is the values a theme hands to its components: colours, font keys, texture keys and
  metrics, each looked up by name. A component reads its look from the skin, so one component renders
  differently under different themes. Unknown keys return `None`, and the component picks its default.
- **`DrawList`** is the flat output: a list of `DrawNode`s with a draw order (`z`). `in_draw_order`
  returns the nodes sorted by `z`, with ties kept in insertion order.
- **`InputEvent`** is small on purpose: a pad button press or release, or a pointer move, press or
  release. A host translates device events into these.
- **`Empty`** is a component with no size and no output. The resolver returns it only if no theme,
  not even Original, provides a slot.

## Slots and themes

- **`SlotId`** is an opaque number that names an overridable slot. The crate does not know what a slot
  is. The host that owns the UI (the game, or a mod) keeps its own catalogue of names and converts each
  entry to a `SlotId` (for example `SlotId::new(3)`). Names stay out of `libs/`, and a mod can add slots
  at run time. Equal numbers are the same slot.
- **`ThemeId`** is `Original`, `Custom`, `Remastered` or `Mod(u32)`. `Original` is the floor that every
  other theme falls back to.
- **`Theme`** has two methods: `id()` and `element(slot: SlotId, cx: &BuildCx) -> Option<Box<dyn Component>>`. A theme
  implements only the slots it replaces and returns `None` for the rest. It must be `Send + Sync`
  because the registry lives in a shared resource.
- **`Themes`** is the registry. It holds the Original floor, the other registered themes and the active
  choice.
  - `Themes::new(original)` refuses any theme whose id is not `Original`.
  - `register(theme)` adds a theme. It refuses a duplicate id, and it refuses `Original`.
  - `set_active(id)` selects a registered theme. It refuses an unregistered id.
  - `resolve(slot: SlotId, cx: &BuildCx) -> Box<dyn Component>` asks the active theme. On `None` it asks Original. If
    Original also returns `None` (a broken floor), it returns `Empty`, so the slot draws nothing and
    nothing panics.

The resolver is the migration path. A theme only needs the slots it has converted. Everything else
renders from Original, so a partly converted theme still draws a complete UI.

## Seam for the draw tree

`DrawNode` in `src/draw.rs` is a placeholder for the neutral draw node that already exists in
`libs/blackbox-feng/src/tree/mod.rs` (`UiTree`, `UiNode`, `NodeKind`). The move of those types into this
crate is a separate step; the seam is marked with a `T3:` comment in `draw.rs`. Until that step lands,
nothing outside this crate reads `DrawNode`. The `z` draw order carries over unchanged.

## Dependencies

Only `thiserror` (for `ThemeError`). No Bevy, no wgpu, no `blackbox-feng` (that dependency runs the other
way, feng depends on this crate), no NFS:MW names or paths, and no `unsafe`.

License: MIT OR Apache-2.0.
