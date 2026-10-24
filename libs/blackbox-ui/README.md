# blackbox-ui

The themeable UI component model of the EA Black Box rewrite. It is engine-generic: no rendering, no
Bevy, no game data, no game names. A host builds a tree of components, lays it out in FEng units,
builds a flat draw list, and draws it. A theme registry decides which component fills each overridable
slot, and falls back to the Original theme when the active theme has no component for a slot.

The crate has the component trait, the support types, the slot names, the theme registry and its fallback,
the base components (`Panel`, `Stack`, `Text`, `Image`, `Gauge`) and the menu components (`Switch`, `Row`,
`Table`). Still to come: the concrete elements (HUD parts, menus), which live in the game crate.

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
- **`DrawList`** is the flat output: a list of `UiNode`s, the same neutral node the presenter draws.
  `in_draw_order` returns the nodes sorted by `z`, with ties kept in insertion order. `into_tree(drawn)`
  turns the list into a `UiTree` (nodes plus `draw_order`); see the next section. `reserve(n)` makes room for `n` more
  nodes, for a component that emits many.
- **`UiNode`, `UiTree`, `NodeKind`** are the neutral draw tree (`src/tree.rs`). They were moved here from
  `blackbox-feng` in task T3 and are re-exported from `blackbox-feng` unchanged.
- **`InputEvent`** is small on purpose: a pad button press or release, or a pointer move, press or
  release. A host translates device events into these.
- **`Empty`** is a component with no size and no output. The resolver returns it only if no theme,
  not even Original, provides a slot.

## Base components

The base components are generic building blocks in `src/components/`, one file each. Each one lays out
inside the `Rect` it is given, caches what `build` needs in `layout`, and reads colours, fonts, textures
and metrics from the `Skin`. Each builds nodes with `world`, `colour`, `world_colour`, `z` and `visible = true`.

- **Placement.** `Rect::world(z)` is the shared helper. It gives the world transform of the unit quad
  (which spans -0.5 to 0.5) scaled to the rectangle's size and moved to its centre. Image nodes use it.
  `DEFAULT_Z` (1.0) is the depth a component draws at unless its builder sets one. Larger `z` is farther.
  Among equal `z`, the later node draws on top.
- **`Panel::new(colour_key)`** fills its rectangle with a skin colour (opaque white if the skin lacks the
  key). `.textured(texture_key)` adds a skin texture. It emits one `Image` node. With no texture the node's
  texture is 0 (none).
- **`Stack::new(StackAxis::Row | Column)`** splits its bounds equally along its axis, with a gap between
  neighbours. `.with_gap(f)` sets a fixed gap. `.with_gap_metric(key)` reads the gap from a skin metric,
  which wins when the skin has it. `.with_child(c)` adds a child (any `Component`). `child_rects()` returns
  each child's rectangle after layout. A stack emits no node of its own. Input: a pointer event goes to the
  child under it, and a pad event goes to the first child that consumes it.
- **`Text::new(font_key, text)`** emits one `Text` node with the skin font (0 if missing). The colour
  comes from the skin key `"text"`, or from `.with_colour_key(key)`. `.with_justification(flags)` takes the
  `justify` flags, the same bits as the FEng text style: `H_CENTRE` (1), `H_RIGHT` (2), `V_CENTRE` (4),
  `V_BOTTOM` (8), `WRAP` (0x10). `H_RIGHT` wins over `H_CENTRE`, and `V_BOTTOM` over `V_CENTRE`. Without
  a flag the origin is the top-left corner (smallest `y`). `max_width` is the rectangle width. `set_text`
  changes the string without a new layout.
- **`Image::new(texture_key)`** emits one `Image` node over the whole rectangle. `.with_uv([u0, v0, u1, v1])`
  picks part of the texture, and `.with_tint(rgba)` sets the colour. The rectangle sets the on-screen size.
- **`Gauge::new(base_key, fill_key)`** is the tachometer mechanic, generalised. It emits one `Image` node for
  the dial (`texture` = base). The fill texture is the mask (`mask` = fill; `mask` is `None` if the skin lacks
  it). Its pivot and angle go in `mask_rotation`: `[pivot_x, pivot_y, angle_deg]`, with the pivot defaulting
  to `[0.5, 0.5]`. `mask_uv` is the whole texture. The mapping is `angle_deg = fraction * sweep_deg`, and
  `sweep_deg` defaults to 360. `set_fraction(f)` clamps `f` to 0..=1 (NaN gives 0), and `build` reads it, so
  no relayout is needed. `.with_needle(key)` adds a second `Image` node: the needle texture over the whole
  rectangle, turned about the rectangle's centre by the same angle. Its rotation is
  `Quat::from_rotation_z(angle)`. The sign convention is not yet checked against the presenter (see the
  T7/T12 screenshots).
- **`Slant::new(colour_key, rect, lean, stops, z, cx)`** is a parallelogram whose opacity ramps along its width,
  drawn as 32 thin strips of the same lean so the slanted ends meet. `lean` is how far the top edge sits right of the
  bottom edge. `stops` are `(position, opacity)` pairs that multiply the colour's alpha. Laid out in `new`, like `Rounded`.
- **`Outline::new(colour_key, rect, radius, thickness, cx)`** is the border of a rounded rectangle, built from the same
  thin strips as `Rounded`. Laid out in `new`.
- **`Ring::new(colour_key, centre, radius, thickness, from, to, cx)`** is an arc of a circle, `from` to `to` degrees
  clockwise from up (a full ring is 0 to 360), drawn as a quad every 3 degrees along the tangent. A bezel, a gauge sweep
  or a row of heat segments. Laid out in `new`.
- **`Trapezoid::new(colour_key, rect, slant, stops, z, cx)`** is a right trapezoid: a horizontal top and bottom, a vertical right
  edge and a left edge that slants in by `slant` units (a negative `slant` cuts the top instead). `stops` ramp its opacity, as for
  `Slant`. Drawn as thin upright columns. Laid out in `new`.
- **`PillBar::new(style, rect, value, point, cx)`** is a rounded track with a border and a gradient fill from the left up to
  `value` (0 to 1); with `point` the fill starts in a point. `PillStyle` names the three skin colours (`track`, `border`,
  `fill`). A gauge or a slider.
- **`Chevron::new(texture_key, facing, rect, tint, nudge, cx)`** is the double arrow of a control that changes with left and
  right: one picture (it points left), mirrored for `Facing::Right`, moved `nudge` units outward. `bounce(age)` is the nudge over the 0.28 s
  after a press.
- **`Choice::new(style, column, value, arrows, nudge, cx)`** is the control of a row that cycles through values: the value
  centred in its column with a `Chevron` at each end when `arrows` is set, and the one the player pressed nudged.
- **`Polyline::new(points, width, colour)`** strokes a line through `points` (canvas units, `glam::Vec2`), `width` units wide
  in a straight RGBA `colour` (not a skin key, so a host can give each road class its own). Each segment is one untextured
  `Image` node: its `world` has the segment (direction times length) in the x column, the normal times the width in the y
  column and the centre in the translation, so the unit quad lands as a rectangle turned along the segment. Repeated and
  non-finite points are skipped, so no segment has zero length; fewer than two distinct points draw nothing. Nothing
  is cached: `build` walks the points and pushes nodes straight into the list.
  - Joins: `LineJoin::Bevel` (the default) adds one rectangle per turn over the outer notch, its outer edge on the bevel line.
    `LineJoin::Mitre { limit }` runs both segments on by `half_width * tan(turn / 2)`, so the outer edges meet in the mitre's
    point with no extra node; a mitre longer than `limit` half widths is bevelled instead. A limit of 2 or less keeps the inner
    side within the stroke too.
  - Caps: `LineCap::Butt` (the default) or `LineCap::Square`, which runs each end on by half the width. A clipped end is flat.
  - `.with_clip(clip, fade)` keeps the line inside a `Clip` (`None`, `Circle { centre, radius }`, `Ellipse { centre, radii }`
    or `Rect(rect)`). The centre line is clipped analytically; the stroke's width is not, so a clipped end can reach half the width
    past the edge. `fade` is a band inside the edge where the line fades out: the presenter has one colour per quad, so the part
    of a segment in the band is cut into pieces about a sixth of the band long (at most 16 per end), each with the opacity at its
    middle. A join outside the clip is not drawn, and one in the band takes the opacity at its joint.
  - `.with_z(z)` sets the depth; `points_mut()` refills the points in place; `node_estimate()` is a reserve hint.
- **`Polylines`** holds many `Polyline`s as one component (a road map), built in the order they were added.
  `with_capacity(lines)`, `push`, `with_line`, `clear` (keeps the room) and `with_reserve(nodes)`, a node count `build` reserves
  in the draw list (`DrawList::reserve`) when it is larger than the lines' own estimate.

Each component has unit tests that build it into a `DrawList` and check the node count, the `NodeKind`,
the placement and the field values. The shared test helpers are in `src/components/mod.rs` (`testing`).

## Menu components

The menu components are the rows-and-switches building blocks of a settings or options screen. Each is one
file in `src/components/`. They are built from the base components, so they reuse their skin lookups and
their justification flags, and they add input handling.

- **`Switch::new(font_key, options)`** cycles through a list of option labels. It emits one `Text` node for
  the selected label, and, when arrows are set, one `Image` node per arrow (left, then right).
  - `.with_arrows(left_key, right_key)` adds the arrow textures from the skin. Each arrow is a square as high
    as the switch, or half its width when that is less. The label fills the middle.
  - `Switch::on_off(font_key, off, on)` is a two-option switch that starts off. `is_on()` and `set_on(bool)`
    read and write it. `is_on()` means "the second option is selected".
  - `.with_selected(index)` sets the start option, clamped to the last option.
  - `handle` consumes pad Left and Right (on press), which step the selection and wrap at both ends. It also
    consumes a press on an arrow, which steps it the same way. A release, or any other button, is ignored.
    A switch with no options consumes nothing.
  - Reporting: `selected()` is the index, `selected_label()` is its label, and `take_changed()` returns
    whether input changed the selection since the last call, then clears the flag. A change that input makes
    also calls `EventCx::request_redraw`. `set_selected(index)` (clamped) does not set the flag.
- **`Row::new(font_key, title)`** is a two-column row. The title is left-justified and vertically centred in
  the left column. The control sits in the right column. `.with_control(c)` takes any `Component` (a
  `Switch`, or a `Text` for a value), and `.with_boxed_control(Box<dyn Component>)` takes one already boxed.
  The default control is `Empty`, so a row without one draws only its title.
  - The title column is `.with_title_width(w)` (FEng units), or the skin metric named by
    `.with_title_width_metric(key)`, which wins when the skin has it. Without either it is half the row.
  - `title_rect()` and `control_rect()` report the two columns at the last layout.
  - `handle` sends pad events to the control. A pointer event goes to the control only when it lands in the
    right column.
- **`Table::new(row_height)`** stacks its rows vertically from the top of its bounds. Every slot is the same
  fixed height, so the rows do not share the height equally (that is `Stack`'s job). The height can come from
  the skin: `.with_row_height_metric(key)` wins when the skin has the key.
  - `.with_header(font_key, title)` adds a header row. It takes the first slot, so the rows start one slot
    lower. `header_rect()` returns it as `Option<Rect>`.
  - `.with_row(row)` appends a row. `row_count()` and `row_rects()` report the rows.
  - Focus: one row has the focus, held as an index (`focused()`, `set_focused(i)`, clamped; it takes
    effect at the next layout). Pad Up and Down move the focus one row, clamped at the ends. At an end they
    return `false`, so a parent can use the press. Pad Left and Right go to the focused row. A pointer press
    on a row focuses that row, passes the press to it, and is consumed.
  - `.with_focus_colour(key)` draws a `Panel` behind the focused row, filled with the skin colour `key`. It
    is drawn first, so it sits behind the rows.
  - A focus move asks for a redraw. The host lays out again before the next frame, so the highlight moves.

Once a switch sits inside a row, it is boxed, so a host cannot read its value back through the table. Input
reports a change only as a redraw request, and the table's focus index says which row was driven. The host's
own settings model is the source of truth: it applies the change there and feeds the value back when it
rebuilds the row (`with_selected`, `set_selected`, or `set_on`).

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

## The draw tree

`src/tree.rs` holds the neutral tree that a presenter draws: `UiTree { nodes, draw_order }`, one `UiNode`
per object (transform, colours, `z`, visibility, kind), and `NodeKind` (`Group`, `Image`, `Text`, `Other`).
Texture and font fields are opaque `u32` keys. The types were relocated unchanged from `blackbox-feng`
(task T3): the only addition is `Default` on `NodeKind` and `UiNode`, so a node can be built with
`..UiNode::default()`. The FEng producer (`blackbox-feng`, `Runtime::tree`) and component producers both
emit this tree; the presenter in `crates/nfsmw/src/ui/present/` reads it and nothing else.

`DrawList::into_tree(drawn)` builds the `UiTree` from a component's output. The caller supplies `drawn`, the
rule that picks which nodes enter `draw_order`. The library holds no default rule, because the depth and
visibility conventions belong to the producer. The FEng producer uses `visible && z > 0 && kind is neither
Group nor Other`. Entries in `draw_order` run farthest first (largest `z`), and the sort is stable, so later
nodes draw on top of earlier ones at the same depth.

## Dependencies

`glam` (for the `Mat4`, `Quat` and `Vec3` fields of the draw tree) and `thiserror` (for `ThemeError`). No
Bevy, no wgpu, no `blackbox-feng` (that dependency runs the other way: feng depends on this crate), no NFS:MW
names or paths, and no `unsafe`.

License: MIT OR Apache-2.0.
