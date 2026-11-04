# UI repair batch R4: menu shadows and platform orientation findings

Repair candidates from the R3 playtest. User acceptance remains open.

## Changes

1. **Stage shadows stay during a car change.** Hiding the car for a menu or garage car switch used to clear every
   shadow caster, so the room's shadows vanished and were rebuilt with each switch. Only the car's casters leave
   now; the stage keeps casting.
2. **No screen-space contact shadows behind the menus.** The frame's shadow tuning carries a contact-shadow switch.
   Gameplay keeps the renderer setting, and the front-end tuning turns it off, leaving map shadows only.

## Findings, no change

**Platform orientation.** Your recording shows the main-menu room arranged differently from the remake, and the
intro move clips through a wall. Several things are now established:

- The crib's 180° room turn is an earlier visual fit, not a recovered parameter.
- All four platform files share the same triangle winding against their normals, so the crib's data is not
  mirrored.
- Turning the room alone cannot reproduce the original view. A reflected room matches the layout but not the
  car's heading. A reflected camera with a native room produces a mirrored picture of the crib, yet a matching
  safe house.

This points at the front-end camera and car-stand conventions, not the room data. The next step is the original's
camera mover (the routine that turns stored angles into a view). No guessed transform is shipped.

**Ambient occlusion on the yard floor.** The crib's floor texture is alpha-blended (it was authored to lie over a
reflection), and blended surfaces do not enter the pass ambient occlusion is computed from. Ambient occlusion also
only darkens the ambient term, which is small on that floor because the scene light dominates.

## Checks

- A regression keeps the stage casters, and only them, while the next car loads.
- The front-end shadow tuning test covers the contact switch.
- Both pass on the modded and unmodified installs.
- Formatting, guards and native Clippy pass, with no sites in changed files.

## Orientation follow-up

Further checks, with no source change:

- **Camera eye.** The original's front-end camera builder starts from identity, then applies the yaw about the
  vertical axis, the pitch about X and a translation by the distance. It inverts the result and builds a look-at
  matrix toward the target. This gives exactly the remake's eye formula, so the eye position is not the fault.
- **Mirrored view.** A horizontally mirrored view (a handedness swap) turns the main-menu car the right way, but
  then contradicts the recorded career car select, which matches without the mirror.
- **Crib car heading.** The car stand yaw shared with the safe house, and quarter-turn offsets of it, do not
  produce the recorded main-menu car heading for any room yaw.

Remaining work is reading the original's stand routine and how the main menu chooses its platform and camera
context. A fitted transform is not accepted as the fix.
