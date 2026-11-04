# UI repair batch R2: garage loading and customization switching

Repair candidates for UI-017/018 (loading state), UI-045 (visible redraw on each item) and the loading part of
UI-049. They were checked against a targeted reference recording of the modded PC original. User acceptance
remains open. UI-059 (menu environment) is investigated below; no environment change is made.

## Reference observations

- **My Cars:** browsing shows an empty room until the selected car loads, about a second after stopping. There is
  no loading text or full-screen wait.
- **Career car select:** stands in the blue safe-house garage, not the main-menu crib. While a car loads, the room
  is empty and an animated five-LED row runs; it is authored in `Car_Select.fng`.
- **Customization:** while browsing spoilers, the shown car stays on screen and the part swaps. The car never
  disappears between items.

## Changes

1. **Same-car edits keep the car.** A part, paint, gauge or tuning preview of the car already shown no longer hides
   it while the new model is prepared. Previously every uncached item emptied the room, removing the car with its
   shadows and reflection until the decode finished. Only a different car empties the room now.
2. **No loading caption.** The remake-only "(Loading car preview…)" suffix on the selected item's name is removed.
3. **Car select loading state.** Changing the car on a menu stage empties the room until the new car is uploaded,
   then the car, its casters and its reflection appear together. The front end runs the package's own LED row
   while the backdrop car loads, through the package's routed messages. Screens without that row ignore them.

## Menu environment (UI-059): findings, no change

The room selection matches the original: crib for the main menu and My Cars, safe house for career car select.
The settled main-menu camera framing also matches. The remake's crib measures darker: about 0.56 of the
reference's floor brightness and 0.70–0.73 on the walls.

Ruled out:

- **Room choice and camera.** Both match.
- **Front-end shadows.** Identical room brightness with and without them.
- **Remake post-processing.** Visual treatment, bloom and tone mapping leave the menu backdrop unchanged in the
  off-screen render.
- **The pre-lit formula.** The room uses the same doubled texture × vertex colour as world scenery, matching the
  original's pre-lit convention.

Contributors identified:

1. **Missing authored atmosphere.** The stock crib contains steam and haze emitters (`AFX_STEAM_EMITTER_*`,
   `AFX_HAZEY_EMITTER_*`), bird emitters, blinking lights and a light-ray texture. These are original content,
   not mod content. The remake draws none of them. The steam/haze over the floor explains why the floor gap is
   larger than the wall gap.
2. **The reference is a modded renderer.** The modded install adds:
   - Xbox 360-style rendering with colour/black bloom and a visual-treatment override;
   - Xbox 360 particle effects;
   - front-end shadow restoration;
   - an HD crib with normal/specular maps.

   Part of the brightness difference therefore belongs to those mods, not to the original PC game.

The executable selects front-end room pieces by `NEXTGEN`/`CURRGEN`/`CAST_SHADOW_MAP` suffixes. The PC draws the
crib's own `CURRGEN` sky, as the remake does.

Next steps, in order:

1. Capture a stock (unmodded) main-menu frame, so that original and mod contributions can be separated.
2. Implement the crib's authored emitters and light rays with their original owners.
3. Decide whether menu post-processing should follow the gameplay visual-treatment setting.

No global brightness compensation is applied.

## Checks and limits

- Regressions cover same-car repaint versus car change, the LED signal state machine and the installed LED row
  fading in and out.
- They pass on the modded and unmodified installs, alongside all 15 `garage_scene` tests, My Cars manager, car
  select, backdrop, inspection, tuning and pause tests.
- Formatting, guards and native Clippy pass, with no Clippy sites in changed files.
- The four known icon-menu failures are unchanged. A modded-only texture-size assertion fails because of that
  install's high-resolution texture mod.
- No before-fix run was made for this batch. Physical controller, rendered motion and playtest acceptance remain
  open.
