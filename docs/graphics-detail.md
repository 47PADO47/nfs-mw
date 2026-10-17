# Graphics detail: current settings and original equivalents

The current quality presets are remake budgets. They do not yet reproduce the
original Video menu's complete preset or advanced detail sliders. This audit
covers the native renderer; original-material modes still have the limitations
listed in [graphics coverage](graphics-coverage.md).

| Original control | Current implementation | Exact original maximum? |
| --- | --- | --- |
| Level of Detail | Custom Low/Balanced/High presets | No. Original is a separate aggregate preset, not mesh distance. |
| World Level of Detail | Four levels with coarse/adaptive selection, authored variant filters and reduced top-mip allocation at Low/Medium; independent extended ranges | Partial. Fixed 480-line projection remains uncalibrated; texture changes apply on next world load and pack-category exemptions remain approximate. |
| Car Geometry | Cached adaptive Low/High using PC projected-size thresholds, plus always-highest and player-only enhancements | Partial. Usage overrides, tire-facing substitutions and original per-part retry ranges remain incomplete. |
| Car Reflection Detail | Off/64/128/256 cube faces; Low/Medium throttled to 15/30 captures per second, High every rendered frame | High matches the source enabled face size only. Capture rules/content remain different. |
| Car Reflection Update Rate | Included in the custom quality cadence | No. Source has an independent boolean, with submission differences. |
| Road Reflection Detail | Projected authored scenery/sky and attached vehicles; optional original blur | Native detail 1/2/3 content implemented. Complete source eligibility, sampler state and matched reference remain partial. |
| Shadow Detail | Original single-map fit/receiver family plus custom cascade alternative; selectable resolution | Partial. Original 0–2 changes techniques/submission, while resolution increases are enhancements. |
| Texture Filtering | Explicit remake sampling options | Original highest level requests supported device maximum anisotropy; 8x/16x are explicit modern limits. |

The current scenery density rule follows the shared behavior specification and
includes the matched PC selector's 25-unit distance guard before density downgrade.
Its ordinary threshold is eighteen projected pixels; a separate
quality route uses 23. The current fixed 480-line scale remains uncalibrated.
Full-resolution texture upload alone therefore does not establish World Detail
High parity. See the [verified control mapping](specs/original-graphics-detail.md).

## What to add next

1. **Wet-road reference validation.** The native projected pass, dampness history,
   content choices and optional blur are implemented. Compare live original
   routes, remaining effect content and the widescreen projection before calling
   Road Reflection Detail fully equivalent. See [the contract](specs/wet-road-reflections.md).
2. **World detail reference calibration.** The four-level route and bounded
   1.5x/2x main-view extensions are available. Calibrate projection and texture
   pack exemptions against the reference before claiming exact option parity.
3. **Complete car usage rules.** Adaptive geometry and an always-highest choice
   are available without per-frame uploads. Validate usage overrides,
   part-specific limits, tire-facing substitutions and PC transition distances.
4. **An original preset once the underlying controls exist.** Reproduce the
   master writer separately from custom quality budgets. The original highest
   master preset does not necessarily maximize every advanced slider.

Reflection sizing, distance, LOD, traffic content and brightness already have
pending implementation in [PR 121](https://github.com/47PADO47/nfs-mw/pull/121).
Coordinate with that work before adding another set of controls. Its improvements
do not themselves provide wet-road capture or prove every original slider level.

Keep general quality choices in Video and detailed budget/enhancement controls
in F8. Label source behavior and custom extensions separately, disable unavailable
controls with a reason, and retain the user's stored values when changing modes.

## Detail controls

Video exposes World Level of Detail and Car Geometry Detail. F8's Detail tab
adds independent detailed-mesh and small-object visibility ranges (1x/1.5x/2x),
an eligible-highest-world diagnostic and a player-only highest-car override.
The quick quality choices set world/car cost levels, retaining extension ranges.
These remain remake quality budgets, not the original master preset writer.

World geometry previews live. Texture mip allocation is captured once per
world scene: save the profile and restart or re-enter the world to apply texture
quality uniformly. Sky, UI and cars retain full texture quality. Extensions
preserve zone lists and authored variant eligibility; they do not draw every
map section or change shadow/reflection budgets. Whole-map high-detail coverage
remains a separate streaming project.

Profiles store these choices in `[detail]` and preserve unknown keys. Console
examples: `set world_detail maximum`, `set geometry_range 2x`,
`set visibility_range 1.5x`, `set car_geometry high`, `set highest_player on`.
See [extensions](specs/extended-detail.md) and [car detail](specs/car-detail.md)
for precise behavior and remaining validation gaps.
