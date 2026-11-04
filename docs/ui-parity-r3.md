# UI repair batch R3: menu platform lighting

Repair candidate for UI-059 (dark menu environment). User acceptance remains open.

## Cause

The remake drew the front-end platforms as pre-lit geometry: texture × vertex colour × 2. The original draws each
platform group with the world shader its effect id selects. In that shader the vertex colour is only the ambient
term and a sun-like scene light supplies most of the brightness. The platform files author small vertex colours
(about 20/255 on the crib's walls and floor), so the remake's yard and safe house rendered far too dark. A stock
capture measured the floor at 95 and the remake at 39 (display-value averages).

## Change

- Platform groups use the world routing and normal materials, as gameplay scenery does.
- The menu frame carries a scenery light: the time-of-day presets at the front-end sun phase limit, with a clear sky.
- The car keeps its own front-end rig. See [menu-backdrop-light.md](specs/menu-backdrop-light.md).

## Checks and limits

The checks are offscreen renders of the main menu, compared against your unmodded and modded recordings. These are
display-value averages; there is no frame-exact match.

| Render | Floor | Whole frame |
| --- | --- | --- |
| Unmodded install, original | 95 | 72 |
| Unmodded install, remake before | 39 | 45 |
| Unmodded install, remake now | 95 | 66 |
| Modded install, original | 66 | 52 |
| Modded install, remake now | 72 | 54 |
| Safe-house car select, original | — | 80 |
| Safe-house car select, remake before | — | 52 |
| Safe-house car select, remake now | — | 70 |

Remaining differences:

- The remake's yard siding is warmer and more saturated.
- The yard sky has a teal tint.
- The safe-house floor is warmer than the original's cool tone.

Likely contributors are the front-end visual treatment, the actual front-end light phase/blend, and the original's
own haze and steam, which are still missing. These are left for visual judgement rather than tuned.

Regressions cover:

- the front-end sun's state limit and room turn;
- installed presets producing a lit scene;
- world-routing sharing.

A batch 17 retarget test was updated to R2's recorded empty-room behaviour; R2's filters had missed it.

Results:

- Focused tests pass on both installs, apart from the modded install's known texture-size assertion.
- Guards pass.
- Native Clippy reports no sites in changed files.
- Formatting was settled with rustfmt after the final guard run.

Not done here: steam, haze, birds, blinking lights and light rays (the crib's authored emitters), and a matched
front-end light read from a live game.
