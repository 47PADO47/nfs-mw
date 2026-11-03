# Options coverage: recorded PC menus and the remake

Last audited: 2026-10-28. Batch 12 source candidates are listed separately from outstanding controls.

This is a **bounded inventory**, not a complete list of original options or a claim of gameplay parity. It compares
visible Audio, Video, Gameplay, Player and one Online Options screen in a recorded **modded PC** session with the
source in this checkout. Other categories and rows outside the recorded view remain unverified. The modded install
can add or alter options; an observed row is not automatically a stock-PC claim.

A control is **missing** below when the observed option has no corresponding independent remake control in the
inspected menu/settings paths. This does not mean the underlying feature is absent. A related feature or setting is
listed separately when it exists. Availability can depend on the renderer, capabilities or loaded mods. These are
source observations, not proof that every option works in a live game.

The [2026-10-27 playtest](ui-parity-playtest.md) reports incorrect or inaccessible EA Trax choices/defaults,
scrolling, prompts and save return. Related source settings listed here do not establish that those UI paths work.

## Missing or incomplete controls

- [ ] **Standard options: Defaults shortcut.** The recorded Player footer offers Defaults. The remake's standard
  [options screen](../crates/nfsmw/src/frontend/widget_menu.rs) hides the legacy Defaults hint and has no
  category-reset handler. The separate [Custom Race options](../crates/nfsmw/src/frontend/custom_race/track_options.rs)
  do offer a confirmed reset action, and F8 Graphics Lab has section and individual-control resets. These are not
  a standard options category reset. The recorded shortcut's exact reset scope and confirmation behavior remain
  unverified; do not infer them from the footer alone.
- [ ] **Player: independent Race Information, Split Time, Score and Leaderboard controls.** The recorded Player page
  shows four separate switches in addition to Transmission, Camera and Gauges. The remake's Player page has only
  those three latter rows. The [HUD page](../crates/nfsmw/src/frontend/hud_options.rs) has `RaceInfo`; its Custom HUD
  consumers gate both the race strip and leaderboard together. It is a partial alternative, not four independent
  controls or an Original-style equivalent. The [original HUD binding](../crates/nfsmw/src/hud/bind.rs) still binds
  race and leaderboard data; their existence does not establish independent visibility switches. Determine the
  recorded switches' exact effects and persistence before implementing them.
- [ ] **Gameplay: separate Free Roam Map Mode and Race Map Mode.** The remake has one `minimap` setting with fixed,
  rotating and off choices, used by the HUD. It cannot select different modes for the two contexts. The HUD page also
  exposes the same orientation setting. See [minimap settings](../crates/nfsmw/src/settings/minimap.rs) and
  [HUD application](../crates/nfsmw/src/hud/plugin.rs).
- [ ] **Gameplay: Autosave, Game Moment Camera and Car Damage switches.** No matching rows exist in the current
  [Gameplay category](../crates/nfsmw/src/frontend/options_rows.rs). Autosave has a
  [career gate and runtime](../crates/nfsmw/src/career/autosave.rs); pursuit-breaker
  [moment cameras](../crates/nfsmw/src/scenes/world/moment.rs) and vehicle damage code also exist. Their existence
  does not supply the recorded on/off controls. Determine the original switch semantics and persistence before
  wiring them; do not disable the underlying features by assumption.
- [ ] **Audio: separate Menu Music Volume and Game Music Volume.** The remake offers one `music_volume` row and
  [music bus](../crates/nfsmw/src/audio/plugin.rs). Per-song menu/game choices in the
  [EA Trax jukebox](../crates/nfsmw/src/settings/eatrax.rs) are useful but do not provide separate volume levels.
- [ ] **Audio: Interactive Music, EA Trax and Audio Mode rows.** The recorded page shows independent switches for
  Interactive Music and EA Trax, plus Audio Mode. The remake has an
  [interactive music conductor](../crates/nfsmw/src/audio/interactive/glue.rs), a
  [`radio` setting and consumer](../crates/nfsmw/src/audio/radio/glue.rs), and per-song jukebox modes. The inspected
  Audio menu does not expose the three recorded controls. The `radio` setting controls radio playback, but has not
  been established as an exact equivalent of the recorded EA Trax switch. Verify original audio-mode choices and
  device behavior before defining a replacement.
- [ ] **Online Options: Connection Quality Warning.** This switch appears in the recorded modded menu. No matching
  row was found in the inspected remake categories. Its original network context and an equivalent warning's
  behavior remain unverified; do not assume the online systems are otherwise equivalent.
- [ ] **Video: in-menu Resolution selection and the recorded calibration screen.** The remake's Video menu offers
  Window Mode, but no Resolution row in the inspected category. The [F8 Graphics Lab Video tab](../crates/nfsmw/src/devtools/graphics_lab/video.rs)
  edits output resolution and monitor; config, CLI and console also expose them. The window system applies these
  settings; see [window modes](window-modes.md). The recorded
  display-calibration instructions and two-logo screen do not have an identified remake equivalent in this audit.
  This is a menu/presentation gap, not an assertion that resolution cannot be changed.

## Existing or related controls

- **Controller configuration (batch 12 candidate):** the Controls page now switches between keyboard and physical
  controller capture. Buttons, triggers and signed axes use the existing bindings persistence. Capture waits for
  release, supports Escape cancellation and preserves keyboard/menu/camera bindings. Competing vehicle bindings
  move off the newly assigned physical control. See [controller remapping](specs/controller-remap-ui.md).
  UI-026–027 still require physical-controller and rendered playtest acceptance.
- **Career pause (batch 12 candidate):** Back to Safe House is available in supported career free roam, with a
  separate pursuit warning. Confirmation abandons unbanked pursuit progress and enters the safe house; cancellation
  retains pause. Race-session variants remain unsupported. See [scene lifecycle](specs/frontend-lifecycle.md).
- **EA Trax (batch 12 candidate):** X previews the selected installed track; Y grabs/releases it for reordering.
  Closing the page stops the temporary preview and restores the previous radio state. Up/Down scrolls the track
  window; Left/Right changes its mode. The displayed native labels are Off, Menu, Racing and All. Fresh-profile
  defaults and exact reference cycle order remain open under UI-007–008; the existing installed DefPlay values
  were retained. See [EA Trax](specs/eatrax-jukebox.md).
- **Player:** Transmission, Camera and Gauges are rows with live settings; the Gauges row gates the whole HUD,
  so its equivalence to the recorded Gauges switch is unverified. The
  recorded camera value `DRIFT` may be mod-specific. Different camera values alone do not establish a missing option
  or matching camera behavior. See [Player options](specs/player-options.md) and
  [HUD application](../crates/nfsmw/src/hud/plugin.rs).
- **Gameplay:** Rearview Mirror and Units have rows and consumers. Minimap has one shared mode as described above.
  The remake also adds HUD, style and accessibility controls. See [category rows](../crates/nfsmw/src/frontend/options_rows.rs),
  [rear-view rendering](../crates/nfsmw/src/app/render/mod.rs) and [HUD settings](hud-settings.md).
- **Audio:** Sound Effects, Car and Speech have related SFX, Engine and Police Dispatch volume rows. Speech is named
  Police Dispatch in the remake; its [speech volume consumer](../crates/nfsmw/src/audio/speech/plugin.rs) applies the
  saved setting. Master volume is an additional remake control. Equivalence of all original mixes remains unverified.
- **Video:** Vsync, texture filtering, world detail, car geometry, anti-aliasing, reflection and shadow controls have
  related settings. Their controls and values are not necessarily the same as the recorded sliders. Some advanced
  controls are in F8, and some depend on renderer capabilities; see
  [graphics settings](../crates/nfsmw/src/frontend/graphics_options.rs),
  [post-processing options](../crates/nfsmw/src/frontend/post_options.rs) and
  [F8 graphics controls](../crates/nfsmw/src/devtools/graphics_lab/). Matching original values and rendered effects
  has not been established for this inventory.
- **Controls:** The remake separates the keyboard/controller binding list from Control Options for sensitivity and
  response; see [controller settings](controller-settings.md). The recorded interval does not establish a complete
  original Controls comparison, so this is not classified as a missing category.

## Still unverified

The recorded Video page extends beyond the shown rows. Other original options, modes, controller-specific pages,
profile-dependent variants, default values, reset-to-default behavior and an unmodified-PC comparison need separate
evidence. The recorded Online row does not establish compatibility with the remake's networking. Asset names or
translation strings alone are not proof that a row was shown or that its setting worked.

This document records gaps and related existing controls. It does not authorize an implementation or guarantee that
an option behaves identically to the original.
