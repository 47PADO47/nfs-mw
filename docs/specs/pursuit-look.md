# Pursuit look and arrest look

Sources: static reading of the retail PC executable's visual-treatment owner (private notes), with names from the
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) CC0 attribute headers. No code is copied, and the values
are read from the installed attribute database at run time. The look's grade itself is in
[visual-treatment](visual-treatment.md); the speedbreaker's effects are in [speedbreaker-visuals](speedbreaker-visuals.md).

## Looks and effects [binary]

- `visuallook` collections: `default` (the ordinary grade), `uves` (a cold blue grade) and `copcam` (a dark blue,
  nearly grey grade).
- `visuallookeffect` collections `uves_transition`, `uves_pulse` and `uves_radialblur`, each with `length`,
  `magnitude`, `heattrigger` and a `graph` of four Bezier ordinates (the y of its four matrix rows).
- An effect started at time `s` gives, at time `now`: the first ordinate when `now - s <= 0`, the last one from
  `length` on, otherwise the cubic Bezier at `(now - s) / length`; each times `magnitude`. A new start restarts a running
  effect from zero; it does not stack. Each frame an effect is evaluated, it stops once the whole heat level is under its
  `heattrigger`, after giving that frame's value. These three effects have no other end.
- `uves_transition` and `uves_radialblur` run on world time (the speedbreaker slows them); `uves_pulse` runs on real
  time.

## When it starts [binary]

Once a frame, outside a scene: read the player's heat (its whole level, truncated) and whether it is being pursued.
Start the three effects when the whole heat level rose since the last frame, or a pursuit started (was not pursued on
the last frame, is now). Then remember both. After a reset neither is known, so the first frame only records them: a
load, a reset or the end of a scene never starts the look. A heat drop, a pursuit's end, and a sighting that starts no
pursuit do nothing.

While an in-world scene plays, both are forgotten each frame and `uves_pulse` and `uves_transition` stop.

## What a frame shows [binary]

- **Grade:** `w` = the transition's value (0 when it does not run). Every value of the grade (curves, tint and
  intensities) is `(1 - w) * default + w * uves`. With the installed graph the frame turns blue at once and eases back
  to the ordinary grade over the transition's length.
- **Flash:** the graded colour is multiplied by `1 + pulse` (`pulse` = the pulse's value, 0 when it does not run), a
  short bright flash.
- **Radial blur:** the largest of the speedbreaker's and the nitrous' blur and the `uves_radialblur` value.
- **Arrest scene:** while an arrest scene plays the grade is `copcam` as it is, with no flash or blur, and the effects
  are reset at its start and its end. Other scenes keep the ordinary grade.
- **Front end:** the ordinary grade with no flash, blur or vignette.

## The rewrite

`nfsmw_data::visual_treatment::pursuit` reads the looks and the effects; `scenes/world/pursuit_look.rs` runs the start
rule and the effects; `focus_feed.rs` gives the renderer the frame's target look, weight and brightness
(`RenderBackend::set_colour_look`, which blends the loaded treatment without rebuilding it) and adds the blur to the
focus filter. Differences, all **[ours]**:
- the Reference treatment's parameter replacements change the ordinary grade the blend starts from, not the target, so
  the pursuit and arrest looks show as they are;
- the heat is the driven car's live heat and "pursued" is whether a chase runs, cool-down included;
- the garage's showroom keeps its own grade, and the look starts again after it;
- the blur follows the motion blur setting, as the rewrite's other radial blurs do;
- with the colour treatment turned off there is no grade, so no flash either;
- missing data leaves out only its part (no `uves` look: no blue; no pulse: no flash).

Before this, a pursuit's start and a heat rise had no change of grade, and the arrest scene used the ordinary grade.

Tests: `scenes::world::pursuit_look::tests`, `nfsmw_data::visual_treatment::pursuit::tests` (with the installed
profile), and the renderer's `scene_treatment` GPU check of the per-frame look.
