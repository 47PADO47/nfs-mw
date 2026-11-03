# Batch 14 playtest follow-up

Reported 2026-10-31 after the batch 14 test build. Scope: Original UI.
Every item below is open. These user findings take precedence over narrow passing regressions.
Nine supplied screenshots are archived privately as S28–S36; no game images belong in public sources.
This record supplements the [main backlog](ui-parity-playtest.md).

## New reports

- [ ] **UI-045 — Changing any car option redraws all shadows.** Record the exact option, car and menu.
  Separate gauge-only/session changes, paint/material changes, geometry changes and a different car.
  Observe whether shadows flicker, disappear, rebuild or merely change with the camera. Trace preview uploads,
  shadow invalidation and room replacement before attributing the cause. Preserve unaffected room lighting.
- [ ] **UI-046 — Gauge color highlight stays on the top swatch.** S28 shows the top framed swatch.
  The user requests the same moving selection treatment as the car paint picker. Validate every swatch,
  vertical scrolling, channel changes, saved colors, keyboard/controller navigation and Back/re-entry.
  Preview color changes alone do not prove correct highlight ownership.
  Batch 15 candidate moves the separate authored frame with the visible selected swatch, retaining its native
  border and highlight animation. Installed checks cover rows, channel restoration, paging and both garage modes.
- [ ] **UI-047 — Junkman appears in the normal main-menu garage.** S34 shows Engine / Junkman.
  The user questions whether this belongs only in the backroom, or should be unavailable in standalone My Cars.
  Compare career shop/backroom, standalone My Cars, relevant unlock rewards and the modded reference separately.
  Record this as an availability/parity question; do not remove a legitimate unlocked option without evidence.
- [ ] **UI-048 — Garage carousel may retain a child selection that the original resets.** The user reports
  remembered submenu positions after Back, but is uncertain about original behavior. Capture each actual route
  and distinguish Back to a retained parent, reopening a child, changing car and a fresh garage session.
  Specify reset/restore policy per route before changing it globally.
- [ ] **UI-049 — Career car switching flashes black.** Keep separate from the missing identity under UI-043.
  Compare first unseen-car loads, cached revisits and rapid changes. Record whether the room, car, UI or entire
  frame disappears, and whether the selected identity matches the published preview throughout the switch.
- [ ] **UI-050 — Menu cars sit slightly inside the floor.** The user reports all cars affected; supplied garage
  views include S32–S33 and S35–S36. Compare wheel contact height, car origin, tire dimensions, ride height and room
  floor coordinates across main menu, standalone garage and career. Do not compensate with one arbitrary offset.
- [ ] **UI-051 — First main-menu camera animation clips through walls.** Record a cold-launch animation,
  including the first room/car load. Check authored camera path, coordinate transforms, room geometry/culling and
  near clip independently. Rendering/environment parity is a possible cause, not an established diagnosis.
- [ ] **UI-052 — Blacklist child Back always selects Race Events.** Reported example: Milestones → Back selects
  Races instead of Milestones. Verify Race Events, Milestones, Bounty and Bio; return to the same rival and category.
  Test repeated entry, keyboard/controller input and the rendered highlight, not only the destination package.
  Batch 15 candidate carries the category through child return arguments and consumes that return state on entry.
  Fresh visits retain the normal default. Installed tests exercise repeated Back/re-entry from all four categories.

## Reconfirmed or expanded existing reports

- **UI-016:** gauge color changes speed text rather than the original game's intended element (reported as gears
  previously). S29 shows recolored gear, speed and units together. Compare each channel/style to the reference;
  separate gear characters, speed digits, units, needle and backing. Broad character tinting is not acceptance.
- **UI-012 / UI-022:** duplicate controls persist in Custom Gauges. S29–S30 show native keyboard hints mixed with
  replacement hints; duplication also exists in keyboard mode. Audit every nested footer group and transition.
- **UI-002:** intermittent Back from customization to the car overview leaves only the scrollbar (S35).
  Test real entry/return routes, rapid input and preview-loading boundaries; verify visible content and usable input.
- **UI-043:** the career BMW M3 GTR has no displayed name/manufacturer art (S36); stat bars are absent too.
  Other cars passing identity tests does not cover this missing-art/fallback case. Include UI-049's black flash.
- **UI-038:** cart checkmarks remain misaligned after the candidate repair (S31). Recheck final world-space
  checkmark and text bounds after authored animation, font scaling and row layout, rather than local offsets alone.
- **UI-042:** standalone saved-car labels still expose `carrera_gt` and model artwork `temp350` (S32).
  S33 provides a named Porsche comparison. Validate installed/modded aliases, missing artwork and safe display-name
  fallbacks; career identity changes do not repair this separate manager path.

## Batch 16 repair candidates

Six scoped repairs address the reconfirmed reports above; user acceptance remains open.

- UI-012/022: suppress the separate gauge PC Back/View Cart roots as well as the console footer. A late-show
  presentation check forces those roots visible before decorating, covering both input devices and garage modes.
- UI-016: apply the Characters color to gear characters and their preview backing glyph only. Speed digits,
  speed backing and units retain their authored colors. Apply the same channel contract in the driving HUD.
- UI-038: keep authored horizontal spacing, but center cart marks on actual scaled font glyph bounds instead
  of retaining a sample string's baseline offset. Recheck after the authored entrance and row styling.
- UI-042: derive saved-car captions from the current car identity, rather than stale saved labels. Share native
  name normalization and alias-aware model artwork fallback; render a real title instead of generic placeholder art.
- UI-043: resolve owned bonus vehicles from the existing preset pool as well as the for-sale pool, preserving
  their own identity and measured performance without adding them to the lot or changing unlock rules.
- UI-002: replay the retained manager's full authored INIT state before refreshing its grid and mode on resume,
  rather than restoring only the scrollbar. Reapply its selected tile's highlight once after INIT so the native
  cursor returns to the selected tile, including a scrolled window after Showcase. Check repeated Back routes.

These candidates do not repair shadow redraws, career black flashes, wheel grounding, camera clipping, or establish
Junkman/reset policy. Rendered comparison, physical-controller behavior and the reported intermittent timing remain
acceptance work; a passing local test is not a new playable build.

## Later source corrections

The supplied original-game gauge-color screenshots supersede batch 16's gear-only interpretation (UI-016).
Lettering Color tints the printed dial scale and unit label; digital speed and gear retain their defaults.
See [presentation state](specs/garage-presentation-state.md) and the [batch 17 record](ui-parity-batch17.md).
These corrections do not close playtest acceptance or update the existing desktop executable.

## Architecture follow-up

The user asks whether duplicated code explains similar controls behaving differently. Audit shared behavior before
adding more page-specific fixes. Do not assume the original implementation is responsible for remake defects.

1. Define one screen lifecycle and input-owner contract: enter, child, Back, resume, exit and release gate.
   Store selected item explicitly, with a per-route reset/restore policy. Keep native animation events separate.
2. Define one footer owner. Enumerate and suppress native hint groups through package bindings, then render one
   binding-aware footer. Validate both input devices and covered/transition states for each affected package.
3. Share palette selection/scroll/highlight behavior. Keep color-channel targets and authored layout as explicit
   package data. Sharing widget behavior must not force distinct gauge elements to use the same tint channel.
4. Classify preview changes: session-only, material-only, geometry and car/room. Reuse room/shadow resources where
   valid, and publish asynchronous preview/identity changes atomically. Measure before promising smooth switching.
5. Consolidate display-name/artwork fallback and coordinate conventions across standalone and career adapters.
   Retain distinct save/unlock rules and authored camera/layout data where behavior differs.

For each shared repair, retain a failing real-route regression, compare final rendered bounds/state and repeat
enter/Back while loading and animating. A generic widget test or successful startup cannot close these reports.

## Screenshot associations

| ID | Visible context | Reports |
| --- | --- | --- |
| S28 | Gauge palette with top swatch framed | UI-046 |
| S29 | Custom Gauges / Boost preview and overlapping footer | UI-016, UI-012, UI-022 |
| S30 | Keyboard footer with duplicate Back/View Cart hints | UI-012, UI-022 |
| S31 | Cart checkmarks below the text baseline | UI-038 |
| S32 | Saved Carrera GT with internal label and placeholder model art | UI-042, UI-050 |
| S33 | Named Porsche 911 Turbo S comparison | UI-042, UI-050 |
| S34 | Normal Engine carousel with Junkman | UI-047 |
| S35 | Garage overview with scrollbar but missing menu content | UI-002, UI-050 |
| S36 | Career BMW M3 GTR without name/art or stat bars | UI-043, UI-050 |

Still images establish visible state. Shadow redraws, black flashes, camera clipping, navigation history and original
reset rules remain user-reported behavior until captured or reproduced. No implementation or new build accompanies
this documentation pass.
