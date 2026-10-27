# Mod API: UI themes (`bb.ui`)

**Delivered (`bb.ui` 1.0): named themes that redraw game elements.** A mod registers a theme with a stable name and a
display text, then gives any slot of the game's UI a draw spec: a short list of panels, text and images. The host builds
that list into the game's own components and draws it in place of the element's usual look. A slot the mod does not name
keeps the game's look. The design is in [ui-themes.md](../specs/ui-themes.md#12-the-bbui-mod-hook).

Status: the registry, the game side and the Mods page choice are built and tested. The presenter draws the registry's slots
in the live game (T12). A player picks a mod theme on the Mods page (see [Selecting a theme](#selecting-a-theme)).

## Concepts

- **Theme.** Registered with `register_theme(name, display)`. `name` is plain (no whitespace) and unique among the mod's
  themes. The game's id for it is derived from the mod id and the name, so it is the same in every run.
- **Slot.** A number the game assigns to one element of its UI (table below). Only these numbers can be overridden.
  A number outside the table is refused.
- **Draw spec.** The primitives given to `set_slot(theme, slot, prims)`, drawn in order. One spec holds 1 to 64
  primitives. A second `set_slot` for the same slot replaces the first.
- **Coordinates.** FEng units: a 640 by 480 screen whose origin is its centre. `x` and `y` are the rectangle's centre, `w`
  and `h` its size (never negative). The game does not rescale a spec, so write it for the full screen.
- **Tokens.** `set_tokens(theme, tokens)` (`bb.ui` 1.1) declares the theme's `primary`, `secondary`, `background`,
  `border` and `muted` colours (`[r, g, b, a]`), and its `radius` and `padding` in FEng units (never negative). While the
  theme is active the game merges them into the skin its components read (`theme.*` keys). A theme that declares tokens
  also gets the game's token-driven components (the radio card today; the Custom HUD's components as they land) for each
  slot it does not name. A theme without tokens keeps the Original look for those slots. The game also derives the
  theme's colour roles (a map's roads and route, the meaning colours, a scrim and an overlay) from these five colours, so the
  Custom map and the full-screen map draw in the theme's palette; a mod cannot set the roles directly yet
  ([ui-themes-tokens.md](../specs/ui-themes-tokens.md) "Colour roles").

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
| 12 | `PoliceRadar` | 45 | `Sms` |
| 13 | `RaceStatus` |  |  |
| 14 | `Leaderboard` |  |  |
| 15 | `BountyRow` |  |  |
| 16 | `Banner` |  |  |
| 17 | `Notifications` |  |  |
| 18 | `WorldMarkers` |  |  |
| 19 | `VehicleCondition` |  |  |

## SDK

```rust
use blackbox_modding::sdk::{Ctx, Prim, Rect, Tokens};

let ui = ctx.ui().expect("bb.ui is offered");
let theme = ui.register_theme("night", "Night drive")?;
ui.set_slot(theme, 4, &[
    Prim::Panel { rect: Rect { x: 0.0, y: 0.0, w: 160.0, h: 120.0 }, colour: [0, 0, 0, 180], texture: None },
    Prim::Text { rect: Rect { x: 0.0, y: -60.0, w: 160.0, h: 20.0 }, colour: [255; 4], justify: 1,
                 font: "FONT_MW_BODY", text: "MAP" },
])?;
ui.set_tokens(theme, &Tokens { primary: [255; 4], secondary: [255, 200, 80, 255], background: [0, 0, 0, 170],
    border: [255; 4], muted: [180, 185, 195, 255], radius: 6.0, padding: 6.0 })?;
// ui.unregister_theme(theme)?; removes the theme and its slots. Themes left registered go when the mod unloads.
```

`ctx.ui()` is `None` when the game does not offer `bb.ui`. Ask for it with `[optional]` in the manifest, and cope with
`None`.

## Selecting a theme

A mod theme is a `ThemeId::Mod(n)` in the registry. The style setting (`ui_style`) picks only the built-in styles, so a mod
theme is chosen separately, on the Mods page of the options. Its first row, `UI theme`, appears once a mod has registered a
theme. It steps through `Built-in style` and then each mod theme, and wraps. The choice is saved in the config as
`[mod_values.nfsmw] ui_style_mod`, the `owner/name` of the theme (empty for the built-in style). When the theme registers
again in a later run, it is selected again. If the theme is absent, the built-in style applies until it returns.

In code, the host calls `UiThemes::select_mod(id)`, and `UiThemes::clear_mod()` gives the choice back to the style.

## Errors

A refused call returns the status below, as the SDK's `Error`:

| Status | When |
|---|---|
| `ARG` | a name has whitespace or is empty (the theme name, an image texture); the display text is empty; the slot is not in the table; the spec is empty or has more than 64 primitives; a size is too small; a number is not finite; a rectangle is negative; justification bits are unknown; a text string is longer than the host takes |
| `EXISTS` | the mod already has a theme with that name |
| `NOT_FOUND` | the theme handle is not one the mod registered (or it was removed) |
| `NOT_IN_CALLBACK` / `WRONG_THREAD` | the call is not made from the mod's callback on the main thread |

A call that fails changes nothing: the spec is checked before it is recorded.
