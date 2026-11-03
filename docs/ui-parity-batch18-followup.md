# Batch 18 playtest follow-up

Reported after the batch 18 playable build. Scope: Original UI. All findings remain open.
Eight supplied screenshots are preserved privately as S37-S44, in attachment order.
This record supplements the [main backlog](ui-parity-playtest.md) and
[batch 14 follow-up](ui-parity-batch14-followup.md). Passing source regressions and
startup checks do not override these playtest failures. No fixes are claimed here.

## New reports

- [ ] **UI-053 - Challenge Series shows keyboard prompts on controller.** S37 shows
  Esc Back and 2 View Tutorial. Validate the grid, locked/unlocked event details,
  tutorial and returns with controller input active. Prompts must follow the active
  device and usable actions, including changes during transitions.
- [ ] **UI-054 - Gauge colors persist across style changes; original policy uncertain.**
  The user observes all color choices retained when changing gauge styles, but is unsure
  whether the original does this. Compare Needle, Lettering and Backing separately across
  style switches, pending cart changes, cancel/apply and re-entry. Do not reset colors
  globally before establishing the reference policy.
- [ ] **UI-055 - Blacklist category icons lack enter/select sound.** The user reports
  missing confirmation audio when entering from the Blacklist icon menu (S39 context).
  Compare accepted category entry, navigation, disabled actions and Back with the reference.
  Confirm exactly one appropriate sound per accepted input; screenshots cannot verify audio.
- [ ] **UI-056 - Default main-menu M3 GTR lacks original vinyls.** S40 and S44 show a
  plain silver BMW. Check the default front-end preset, vinyl layers and material loading.
  Distinguish the default showcase car from a saved/customized career car. Restore the
  expected original default appearance without overwriting user customization.
- [ ] **UI-057 - Returning from customization replays the main-menu camera intro.**
  In standalone My Cars, return from customization runs the main-menu camera animation,
  then snaps to the intended overview. Validate the actual child return path, repeated
  visits, Showcase and cold/cached previews. A fresh main-menu intro and retained garage
  return require separate behavior. Related UI-051 remains open.
- [ ] **UI-058 - In-game Pause Menu backdrop is broken.** S42 shows an almost black
  central rectangle and broad white side regions around the pause carousel. The user
  suspects the vignette; cause is unverified. Treat as a reported regression. Compare
  aspect ratios, viewport/texture bounds, alpha/compositing and pause enter/exit states,
  including options/tuning returns. Validate final pixels, not only package presence.
- [ ] **UI-059 - Main menu may use the wrong environment and appears too dark.**
  The user suspects other menus too. S40/S44 show the remake; S43 supplies a brighter
  comparison with a different car. The reference executable/install is not verified.
  Compare matched menu route, room, camera, time/lighting, fog, reflections and display
  settings. Separate wrong environment selection from renderer/lighting differences;
  do not apply a global brightness adjustment based on unmatched screenshots.

## Reconfirmed or expanded reports

- **UI-046 - Gauge swatch selection still has a visual alignment defect.** S38 shows
  the border offset relative to the white swatch. Earlier moving-frame repairs do not
  close this. Check final transformed swatch/frame bounds after animation and scaling,
  every visible row, scrolling, style/channel changes and controller/keyboard selection.
- **UI-032 - Blacklist Bio remains buggy and does not follow original logic.** The user
  reconfirms this without a new Bio screenshot or exact failing sequence. Compare actual
  entry/return, rival identity, content/portrait availability, progression-dependent state,
  Continue/Back and prompt ownership before defining a fix. Retain the earlier report.
- **UI-045 - Full shadow redraw remains visible when switching customization styles.**
  Revisited styles appear cached, but first changes still differ substantially from the
  original. The batch 17 idle/nonvisual cache repair is partial coverage only. Record
  cold versus cached style changes, visible flicker, resource lifetime and invalidation;
  do not infer parity from fewer submissions or a warm cache.
- **UI-041 / UI-043 - Career stat bars and markers still behave incorrectly.** S41 shows
  full blue segmented bars with yellow markers at partial positions. The report concerns
  fill/marker semantics, not only color or depth. Establish stock, installed, maximum and
  comparison meanings against the reference; compare different owned cars and upgrades.

## Evidence index

| ID | Supplied context | Reports |
| --- | --- | --- |
| S37 | Challenge Series locked challenge 33/70; keyboard footer | UI-053 |
| S38 | Gauge white swatch and offset-looking frame crop | UI-046 |
| S39 | Blacklist Race Events category and current requirements | UI-055 context |
| S40 | Remake main menu, plain silver BMW, dark room | UI-056, UI-059 |
| S41 | Career Top Speed/Acceleration/Handling bars and markers | UI-041, UI-043 |
| S42 | Pause Menu with dark center and white side regions | UI-058 |
| S43 | Supplied brighter main-menu comparison, customized Porsche | UI-059 reference |
| S44 | Remake main menu, plain BMW, dark environment | UI-056, UI-059 |

Still images establish visible state only. Camera replay, shadow rebuild timing, audio,
color retention and Bio flow are user-reported observations awaiting reproduction.
Prioritize the Pause Menu regression before cosmetic defects in the next repair pass.
