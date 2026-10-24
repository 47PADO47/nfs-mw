# Custom HUD: visual identity and tokens

The look of the Custom HUD and menus as data: the design rules, the colour tokens, the accessibility variants, opacity,
scale and motion. Every value lives in one place, `crates/nfsmw/src/ui/theme/custom/tokens.rs` (PR 5), and no widget
names a literal colour. Index: [README.md](README.md). Decision: [ADR 0013](../../decisions/0013-custom-hud.md).

Evidence: everything here is **[ours]**, a design choice of the rewrite following the owner's brief. The hex values the
brief gives are marked *(brief)*; the rest are starting points to be checked on screen in PR 5 and PR 12.

## 1. Design rules

The identity mixes three influences, without copying another game's interface:

- **Precision motorsport:** clean telemetry, disciplined alignment, readable numbers, subtle scales.
- **Underground street racing:** dark industrial plates, restrained red accents, condensed type, sharp transitions.
- **World-integrated navigation:** translucent road overlays and contextual markers that keep the road visible.

Rules every component follows:

1. **The road stays visible.** No element sits in the central road view except the translucent map and world markers.
   Plates are used only where text needs contrast.
2. **No permanent decoration.** No neon outlines, thick borders, glow, giant panels or science-fiction ornaments.
   Rules and tracks are one unit wide at scale 1.
3. **No flashing.** Escalation is shown by contrast, weight and size, and by a one-shot pulse at most. Nothing blinks
   continuously; nothing turns the whole screen red.
4. **Never colour alone.** Every state that has a colour also has a word, an icon or a position (the rev strip's
   segments, the AHEAD/BEHIND label, the condition chip's icon). This makes the colour-vision palettes a comfort, not a
   requirement.
5. **Only real data.** A widget whose data is `None` is not drawn. There are no placeholder dashes for missing metrics.

## 2. Colour tokens

Colours are `[r, g, b, a]` in the code; here as hex plus alpha (0 to 1). The token names are the skin keys the Custom
components read, under the `custom.*` prefix, merged into the skin like the existing `theme.*` keys
([ui-themes-tokens.md](../ui-themes-tokens.md)).

| Token | Use | Default |
|---|---|---|
| `text` | digits, primary labels | `#E8EDF2` *(brief)*, 1.0 |
| `text_secondary` | units, secondary labels, inactive rows | `#9BA7B3` *(brief)*, 1.0 |
| `surface` | plates behind text | `#12161B`, 0.60 |
| `surface_raised` | a selected menu row, an expanded chip | `#1C222A`, 0.75 |
| `rule` | tracks, empty segments, separators | `#E8EDF2`, 0.18 |
| `accent` | the theme accent (gear box edge, selected row bar) | from `ui_theme`, see below |
| `nav` | route, active checkpoint, turn cue | `#7FD8EC` (cool cyan) |
| `success` | escaped, AHEAD, checkpoint confirmed | `#5CC28F` |
| `caution` | rev strip near the shift point, degraded states | `#E8B24A` |
| `danger` | pursuit, red line, BEHIND emphasis | `#E0524D` *(brief)* |
| `critical` | critical warnings: solid plate, dark text, icon | `#FFB000`, text on it `#12161B` |

**Accent family.** The existing `ui_theme` setting keeps choosing the accent: `bevy` gives the cyan `#78DCFF` and
`remastered` the gold `#FFC850` (the `secondary` colours of `ui/theme/custom/palette.rs`). Only `accent` follows it;
`nav`, `success`, `caution`, `danger` and `critical` keep their meaning in either family.

**Critical is distinct by treatment.** A critical warning is the only element drawn as a solid plate with dark text and
an icon. Pursuit danger is red text and rules on a dark plate. The two never look alike even where the hues are close.

## 3. High contrast

`[hud] high_contrast = true` replaces the token values after the palette is chosen:

| Token | High contrast |
|---|---|
| `text` | `#FFFFFF`, 1.0 |
| `text_secondary` | `#D5DDE5`, 1.0 |
| `surface` | `#000000`, 0.85 |
| `surface_raised` | `#1A1A1A`, 0.92 |
| `rule` | `#FFFFFF`, 0.50 |
| `nav` | `#8FE6FF` |
| `success` | `#66E0A3` |
| `caution` | `#FFD166` |
| `danger` | `#FF6B66` |
| `critical` | `#FFD000`, text on it `#000000` |

It also draws a one-unit `text`-coloured outline (alpha 0.6) around every plate, raises the minimum text weight one step
(see [typography.md](typography.md) section 4) and gives the map's roads a dark under-stroke so they read over bright
scenery.

## 4. Colour-vision palettes

`[hud] palette = default | deuteranopia | tritanopia`. A palette replaces only the hues that carry meaning; `text`,
`text_secondary`, `surface`, `surface_raised` and `rule` do not change. The replacements come from the Okabe and Ito
colour-blind-safe set, lightened where a dark plate needs it.

| Token | `default` | `deuteranopia` (red-green) | `tritanopia` (blue-yellow) |
|---|---|---|---|
| `nav` | `#7FD8EC` | `#F2F4F7` (cool white) | `#F2F4F7` (cool white) |
| `success` | `#5CC28F` | `#56B4E9` (sky blue) | `#009E73` (bluish green) |
| `caution` | `#E8B24A` | `#E69F00` (orange) | `#CC79A7` (reddish purple) |
| `danger` | `#E0524D` | `#D55E00` (vermilion) | `#E0524D` |
| `critical` | `#FFB000` | `#F0E442` (yellow) | `#FFB000` |

Order of application: palette, then high contrast (which brightens the palette's hues by the same rule as section 3).
The pairs to keep apart, checked with a colour-vision simulator in PR 5 and recorded here: `success` against `danger`
(escaped against pursuit), `caution` against `danger` (shift point against red line), `nav` against `success` (route
against confirmation). **[unconfirmed]** until that check.

## 5. Opacity

- `[hud] opacity` (0.3 to 1.0, default 1.0) multiplies the alpha of every Custom HUD node.
- The map has its own `[hud] map_opacity` (0.1 to 0.8, default 0.35), applied to the road layer; the map's markers use
  `opacity` like the rest of the HUD so the player arrow and confirmed cops stay legible.
- **Floor.** A critical notification and the vehicle-disabled warning never draw below 0.8 alpha, whatever `opacity`
  says: a safety message must stay readable.
- Plates fade with their content; a plate never stays behind faded text.

## 6. Scale

- `[hud] scale` (0.75 to 1.5, default 1.0) scales every Custom HUD element about its anchor. It does not move anchors
  (see [layout.md](layout.md)).
- `[hud] text_size` (0.8 to 1.5, default 1.0) scales text only, on top of `scale`. A plate grows to fit; a row that
  would overflow its zone wraps or elides with an ellipsis, never overlaps another zone.
- Menus have their own scale (1.0 for now) and take `text_size` but not `scale`, so the HUD scale is separate from the
  menu scale, as the brief asks.
- Sizes in this spec are HUD units at scale 1: the presenter's 480-unit-high canvas
  ([hud-viewport.md](../hud-viewport.md)).

## 7. Motion and transition durations

One table in `tokens.rs`, so every transition is tuned in one place. All animation runs on elapsed time from the frame
clock (`CustomFeedCell`), never on frame counts.

| Name | Duration | Used by |
|---|---|---|
| `fade_in` | 150 ms | any element appearing |
| `fade_out` | 250 ms | any element leaving |
| `slide` | 200 ms, 12 units | notifications entering, the race strip arriving |
| `pulse` | 400 ms, one shot | nitrous activation, checkpoint confirmed, heat up |
| `mode_change` | 300 ms | free roam, race and pursuit layouts crossfading |
| `map_mode` | 300 ms | map mode changes |
| `map_zoom` | 250 ms | zoom steps (a critically damped spring on the view radius) |
| `speed_spring` | about 60 ms settle | the speed scale ([telemetry.md](telemetry.md)) |
| `delta_hold` | 2.0 s | the bounty delta before it fades |

**Motion reduction** (`[hud] motion_reduction = true`): slides become fades in place, pulses become a static highlight
held for the pulse's duration, map zoom and mode changes become 150 ms crossfades, and the speed scale keeps its spring
(it is a reading, not decoration). Nothing is removed: the same information arrives, without movement.
