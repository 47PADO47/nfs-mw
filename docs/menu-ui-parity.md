# Original menu UI: fixes and parity audit

This audit covers the Original UI and its shared host: package loading, pointer
selection, modal return, camera state, car rendering and menu sound delivery.
Custom options and alternate HUDs remain supported extensions.

The [2026-10-27 playtest backlog](ui-parity-playtest.md) records open settings, EA Trax, garage and career-name
failures in the test build. Earlier fixes below do not close those reports, including inconsistent held D-pad input.

## Confirmed defects

- **Wrong icon action:** native carousel clicks could change package button focus
  without changing the logical selection. Activation then opened the old entry.
  Pointer selection must resolve the item bound to the visible slot, including
  a carousel window that has moved past its first entries.
- **Unresponsive menu after No or Back:** a retained icon menu could return from
  its confirmation with its logical input flag still disabled. Resuming it must
  clear the completed pending action and restore both logical and package input.
- **Camera selection during loading:** the screen could request a camera pose
  before asynchronous car upload initialized the stage camera. Stage camera
  initialization must precede those requests and survive later uploads.
- **Missing local reflections:** the menu never submitted a reflection scene.
  The car therefore used a fallback environment even with local reflections
  enabled for driving. The stationary menu room must supply that environment,
  excluding the reflective car itself.
- **Shadow settings:** menu shadows had a separate disabled default and rejected
  native rendering through a capability check for soft shadows. Menu shadow
  settings must honor the supported native path and global shadow quality.
  The native renderer also ignored instance caster/receiver flags. It now carries
  receiver flags in explicit instance data and filters casters before shadow-map
  submission. Cached shadows track caster flag changes.
- **Missing menu effects:** the package host discarded outgoing sound messages.
  Sound delivery needs a host route to the installed front-end banks; the sound
  message must still reach any screen logic that uses it to recognize a button.
  Pointer edits play effects only when they change a value or activate an enabled
  action; duplicate package announcements produce one effect per trigger.
- **Transition placeholder:** authored leave scripts could show `OPTION_MASTER`
  again. The host keeps that stand-in hidden on each animation frame.
- **Blacklist presentation:** the overview did not bind its authored carousel
  slots or replace its legacy footer. It now shows the four sheet icons and
  prompts from the active keyboard/controller bindings.
- **D-pad skips:** viewer zoom also included D-pad input, and the menu treated
  that value as wheel movement. Menus now consume physical wheel input through
  a separate channel, preventing a second move from the same D-pad gesture.

- **Diverging icon rows:** the Blacklist overview kept a copy of the icon carousel
  that lacked the scroll ease, the package's slot sizes and the exit shrink, so
  the same row behaved differently from the main menu. Both now use one icon
  strip (`icon_strip.rs`); the original also has a single scroller class. The
  overview's `EXIT_STARTED` handling is by analogy with the verified packages and
  is not checked against its own package.

These causes come from source inspection. Regression and runtime evidence must
be read with the validation report; a code change alone does not prove a matched
comparison with the original executable.

Stage materials also resolve their shared global/front-end textures. This keeps
the authored transparent placeholder geometry from becoming opaque white.

The safe-house floor now has a car-only transparent planar capture through its
authored alpha and triangles. Crib emitter/light helpers no longer appear as
colour geometry. The textured crib warehouse also supplies shadows; only the
safe-house high-resolution proxy is hidden from the colour view.
Screenshot-backed footers publish Back then Accept, and profile rows keep their
native text/highlight/brackets with column-sized pointer bounds.

## Pipeline and remaining differences

| Area | Existing implementation | Parity limit |
| --- | --- | --- |
| Packages and assets | Installed FEng packages, fonts, textures and translated strings | Modded installations can replace layouts and images. |
| Input | Shared keyboard, pointer and controller routes; package events and screen logic | Original per-controller package masks and geometric navigation wrapping remain separate work. |
| Carousel | Installed slots, bookends, headers and leave animations | The navigation ease remains the port's cubic curve; the exact original spline is not established. |
| Dialogs | Installed dialog package, answer focus and modal stack | All original dialog variants and their complete career side effects are not established by the reported regression cases. |
| Camera | Installed stage shots and camera mover | A matched original/remake transition capture is required to prove timing and framing parity. |
| Stage atmosphere | Installed room, hidden helper classification and separate shadow proxies | Steam, haze and bird emitters are not yet recreated; hiding their marker meshes does not implement their effects. |
| Car rendering | Shared native renderer and car shading with authored menu staging | Matching quality settings does not make a showroom and an outdoor scene share lighting or surroundings. |
| Audio | Installed front-end bank graphs and host feedback, with cached parsing and gesture deduplication | Original FEHUD mixing, sustained rap-sheet and drive-on effects, every screen override and matched listening remain unverified. |
| Credits | Installed text with a port-controlled scrolling window | Original clipping and exact roll timing remain unverified. |
| Options and HUD | Original assets with added settings and alternate presentations | Deliberate extensions; they are not original-game features. |

## Deferred follow-up work

The playtest accepts the incomplete initial environment for this PR. Complete
the crib scenery and its unresolved warehouse material `0x578BC664` separately.
Compare the title car's preset livery with the original; the captured remake
still differs from the supplied blue/silver M3 reference.

Follow-up comparisons must cover camera timing/framing, steam/haze/birds,
original sound mixing and sustained effects, and the remaining career/dialog
flows. The table above records the specific limits. This PR does not claim full
original-game parity.

## Evidence rules

Installed-package tests establish behavior with those packages. GPU checks
establish the selected renderer paths. Screenshots establish the captured frame.
None establishes all original menu behavior, sound mixing or career flows.

The original executable, extracted assets and private captures remain outside
this repository. Existing behavioral references are recorded in
[menu specifications](specs/frontend-menus.md),
[menu provenance](provenance/frontend-menus.md),
[confirmation dialogs](specs/confirmation-dialogs.md) and
[menu backdrop looks](specs/menu-backdrop-looks.md) and
[floor reflections](specs/menu-floor-reflections.md).
