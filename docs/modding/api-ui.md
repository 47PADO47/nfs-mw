# Mod API: UI themes (`bb.ui`)

**Delivered (`bb.ui` 1.0): named themes that redraw game elements.** A mod registers a theme with a stable name and a
display text, then gives any slot of the game's UI a draw spec: a short list of panels, text and images. The host builds
that list into the game's own components and draws it in place of the element's usual look. A slot the mod does not name
keeps the game's look. The design is in [ui-themes.md](../specs/ui-themes.md#12-the-bbui-mod-hook).

Status: the registry and the game side are built and tested. The presenter draws the registry's slots in the live game
only after the T12 wiring, so a mod theme is resolved but not yet visible in play. The Mods page row that chooses a mod
theme is owed too; until then a theme is chosen in code (see [Selecting a theme](#selecting-a-theme)).

## Concepts

- **Theme.** Registered with `register_theme(name, display)`. `name` is plain (no whitespace) and unique among the mod's
  themes. The game's id for it is derived from the mod id and the name, so it is the same in every run.
- **Slot.** A number the game assigns to one element of its UI (table below). Only these numbers can be overridden.
  A number outside the table is refused.
- **Draw spec.** The primitives given to `set_slot(theme, slot, prims)`, drawn in order. One spec holds 1 to 64
  primitives. A second `set_slot` for the same slot replaces the first.
- **Coordinates.** FEng units: a 640 by 480 screen whose origin is its centre. `x` and `y` are the rectangle's centre, `w`
  and `h` its size (never negative). The game does not rescale a spec, so write it for the full screen.

## Primitives

| Kind | Fields | Draws |
|---|---|---|
| `Panel` | rect, `colour` (RGBA), optional `texture` | a filled rectangle, with the texture over it |
| `Text` | rect, `colour`, `justify`, `font`, `text` | a string in a font, placed by its justification |
| `Image` | rect, `uv` (`[u0, v0, u1, v1]`), `texture` | a part of a texture over the rectangle |
| `Gauge` | (owed) | a fill that follows a value, such as the tachometer |

`justify` takes the FEng text bits: `H_CENTRE` 1, `H_RIGHT` 2, `V_CENTRE` 4, `V_BOTTOM` 8, `WRAP` 0x10. Other bits are
refused. `font` and `texture` are resource names without extension (`FONT_MW_BODY`, `Outrun_Backing`). An empty font
means the default font. A texture name is required for an image.

Colours are inline RGBA. A mod cannot name the game's own skin colours, so it cannot change another element's look by
accident.

## Slot numbers

These are the game's element slots (`crates/nfsmw/src/ui/theme/element.rs`). The numbers are part of the ABI and are
never reused.

| Number | Slot | Number | Slot |
|---|---|---|---|
| 1 | `Tachometer` | 20 | `MainMenu` |
| 2 | `Speedometer` | 21 | `Categories` |
| 3 | `GearIndicator` | 22 | `PauseMenu` |
| 4 | `Minimap` | 23 | `OptionTable` |
| 5 | `Blips` | 24 | `TrackSelect` |
| 6 | `PursuitMeter` | 25 | `SafeHouseMenu` |
| 7 | `Nitrous` | 40 | `Blacklist` |
| 8 | `Turbo` | 41 | `CarLot` |
| 9 | `Speedbreaker` | 42 | `RapSheet` |
| 10 | `RadioCard` | 43 | `RegionUnlock` |
| 11 | `BustedOverlay` | 44 | `MemoryCard` |
|  |  | 45 | `Sms` |

## SDK

```rust
use blackbox_modding::sdk::{Ctx, Prim, Rect};

let ui = ctx.ui().expect("bb.ui is offered");
let theme = ui.register_theme("night", "Night drive")?;
ui.set_slot(theme, 4, &[
    Prim::Panel { rect: Rect { x: 0.0, y: 0.0, w: 160.0, h: 120.0 }, colour: [0, 0, 0, 180], texture: None },
    Prim::Text { rect: Rect { x: 0.0, y: -60.0, w: 160.0, h: 20.0 }, colour: [255; 4], justify: 1,
                 font: "FONT_MW_BODY", text: "MAP" },
])?;
// ui.unregister_theme(theme)?; removes the theme and its slots. Themes left registered go when the mod unloads.
```

`ctx.ui()` is `None` when the game does not offer `bb.ui`. Ask for it with `[optional]` in the manifest, and cope with
`None`.

## Selecting a theme

A mod theme is a `ThemeId::Mod(n)` in the registry. The style setting (`ui_style`) picks only the built-in styles, so a mod
theme is chosen separately. In code, the host calls `UiThemes::select_mod(id)`, and `UiThemes::clear_mod()` gives the choice
back to the style. The style sync keeps a chosen mod theme. The row on the Mods page that makes this choice is owed.

## Errors

A refused call returns the status below, as the SDK's `Error`:

| Status | When |
|---|---|
| `ARG` | a name has whitespace or is empty (the theme name, an image texture); the display text is empty; the slot is not in the table; the spec is empty or has more than 64 primitives; a size is too small; a number is not finite; a rectangle is negative; justification bits are unknown; a text string is longer than the host takes |
| `EXISTS` | the mod already has a theme with that name |
| `NOT_FOUND` | the theme handle is not one the mod registered (or it was removed) |
| `NOT_IN_CALLBACK` / `WRONG_THREAD` | the call is not made from the mod's callback on the main thread |

A call that fails changes nothing: the spec is checked before it is recorded.
