# Graphics detail: current settings and original equivalents

The current quality presets are remake budgets. They do not yet reproduce the
original Video menu's complete preset or advanced detail sliders. This audit
covers the native renderer; original-material modes still have the limitations
listed in [graphics coverage](graphics-coverage.md).

| Original control | Current implementation | Exact original maximum? |
| --- | --- | --- |
| Level of Detail | Custom Low/Balanced/High presets | No. Original is a separate aggregate preset, not mesh distance. |
| World Level of Detail | Adaptive scenery slots 0/2; fixed 480-line projection reference; full uploaded textures | No. Original also changes mip residency, forces coarse geometry at low levels, and filters authored mesh variants. |
| Car Geometry | Driving player, traffic and cop loading uses authored LOD A | Detailed close-up geometry, but not original dynamic body/tire LOD or per-usage ranges. |
| Car Reflection Detail | Off/64/128/256 cube faces; Low/Medium throttled to 15/30 captures per second, High every rendered frame | High matches the source enabled face size only. Capture rules/content remain different. |
| Car Reflection Update Rate | Included in the custom quality cadence | No. Source has an independent boolean, with submission differences. |
| Road Reflection Detail | Projected authored scenery/sky and attached vehicles; optional original blur | Native detail 1/2/3 content implemented. Complete source eligibility, sampler state and matched reference remain partial. |
| Shadow Detail | Original single-map fit/receiver family plus custom cascade alternative; selectable resolution | Partial. Original 0–2 changes techniques/submission, while resolution increases are enhancements. |
| Texture Filtering | Explicit remake sampling options | Original highest level requests supported device maximum anisotropy; 8x/16x are explicit modern limits. |

The current scenery density rule follows the shared behavior specification, but
the matched PC selector has an additional 25-unit distance guard before density
downgrade. Its ordinary threshold is eighteen projected pixels; a separate
quality route uses 23. The current fixed 480-line scale remains uncalibrated.
Full-resolution texture upload alone therefore does not establish World Detail
High parity. See the [verified control mapping](specs/original-graphics-detail.md).

## What to add next

1. **Wet-road reference validation.** The native projected pass, dampness history,
   content choices and optional blur are implemented. Compare live original
   routes, remaining effect content and the widescreen projection before calling
   Road Reflection Detail fully equivalent. See [the contract](specs/wet-road-reflections.md).
2. **World detail as one coherent route.** Combine authored mesh filters,
   source low/high slot selection and bounded mip residency. First calibrate
   projection against the reference; keep optional distance/detail multipliers
   clearly marked as custom enhancements.
3. **Car LOD with a compatibility choice.** Support original per-usage allowed
   ranges and body/tire transitions, then offer an explicit always-detailed
   enhancement. Keep high-FPS decisions based on pose/view, without per-frame
   resource uploads or a fixed 60-FPS assumption.
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
