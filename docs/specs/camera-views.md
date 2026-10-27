# Camera views

## Gameplay controllers

The seven original profile slots are Bumper, Hood, Close, Far, SuperFar, Drift
and Pursuit. They resolve the car's inherited `ecar.CameraInfo_*` references;
they are not constructed from guessed collection names. Ordinary hotkey cycling
respects the fullscreen `SELECTABLE` flags. Direct selection keeps all seven
profiles available. An install may drop a profile: the remastered install has
no `superfar` collection and leaves every car's `CameraInfo_SuperFar` unset. That
slot then takes the Far profile's values and is unselectable, so the cycle skips
it and direct selection still gets a camera. The existing custom Chase and Dynamic FOV controllers remain
independent, including the user's Chase default.

The implementation and evidence are split by contract:

| Contract | Specification |
|---|---|
| Per-car references, viewport fields and optical axes | [Original camera data](original-camera-data.md) |
| Rolling cubic follow, speed/acceleration/slip inputs and transitions | [Original camera follow](original-camera-follow.md) |
| Terrain, engine, NOS, brake and helicopter noise | [Original camera noise](original-camera-noise.md) |
| Impact and gear-change local offset pulses | [Camera transients](original-camera-transients.md) |
| Wall, road and body clearance, look-back and visibility | [Original camera obstruction](original-camera-obstruction.md) |
| Camera clock, physics history and display interpolation | [Original camera host](original-camera-host.md) |
| Runtime terrain and prop data | [Camera environment data](original-camera-environment-data.md) |

These replace the former static placement, heading-only rotation, vertical-FOV
assumption, guessed positive-LAG bumper offset and missing obstruction handling.
The camera anchor is the physical body position plus rotated CG with its lateral
component removed; model pivot corrections belong to rendering, not this anchor.
Bumper and Hood attach to the full body rotation. Outside views retain native
rolling cubic state and velocity-dependent direction rather than snapping to the
car heading. Outside-to-outside profile changes preserve that state; switching
to or from an attached view snaps. Look-back reverses local horizontal offsets
immediately. Bumper hides the player; Hood hides it only while looking back.

The original FOV is a whole binary angle. The delivered optical adapter matches
the selected playable reference's WidescreenFix `FixFOV=1, Scaling=0` framing,
preserving vertical framing while expanding horizontally with display aspect.
The unmodified PC optical axes remain separately verified by native vectors.
Both retain the original 0.5 m near plane. Custom camera projection is unchanged.
Speedbreaker divides profile LAG and multiplies FOV by its real-time zoom ramp;
NOS adds the native outside-view angle before cubic smoothing. The two values
in each profile field select viewport mode, never vehicle speed endpoints.
`STIFFNESS` is read but is not consumed by the checked native Cubic path.

## Timing and validation boundary

Native per-update filters run on an unscaled 60 Hz camera clock. Retained
physical input keys are sampled through the same Speedbreaker time integral as
the world, preserving intermediate inputs during slow render frames. Displayed
camera offsets interpolate with the displayed car anchor; rendering never
advances camera state. This deliberately preserves the original 60 FPS response
at high render rates rather than making native update-count filters stronger.

Isolated unchanged PC routines establish the arithmetic contracts, with
synthetic regressions for the high-FPS adaptation. Installed-data checks cover
profile references and physical/render anchor integration. This is not a claim
of complete live visual parity: special floor-state ownership, body registration
order, absolute original noise phase and cinematic/ICE camera tracks still need
matched-scene evidence. See each specification's limits rather than treating
individual helper checks as a whole-game comparison.

## Pursuit camera (caught pan and arrest, procedural)

Built in `crates/nfsmw/src/viewer/camera/pursuit/`. `Drive::update_pursuit` advances it once per physics tick, fed by
`scenes/world/simulation.rs` (the busted meter, `is_busted`, and the nearest height-matched cop). `Drive::camera` and
`Drive::physics_camera` blend its shot over the chase view. The original views (bumper to drift) are not changed.

- **Caught pan:** when the busted meter first rises above zero, the view eases over 2.5 s toward a side framing of the
  player and the nearest matched cop, then back to the chase camera. It fires once per catch: the meter draining back to
  zero re-arms it.
- **Arrest framing:** on the bust (the rising edge of `is_busted`), a 3.0 s framing from the side of the player and the
  cop, fixed at the bust. It replaces the caught pan and does not replay until the next chase.
- **Original:** the bust sends `MPerpBusted` on the gameplay channel. Three seconds later the cutscene channel plays the
  arrest, which chooses a scene and a camera track (`docs/specs/ai-pursuit.md` section 5.2; decomp
  `Animation/AnimChooseArrest.cpp`, not read here). ICE camera tracks are not read (deferred). Ghidra on `speed.exe`
  found the arrest clip names (`ArrestM01` to `ArrestF23`), `BUSTED_OVERLAY.fng`, `PostBusted.fng`, and
  `BustedMomentMarker` at `0x00893200`, which has no direct xrefs. `MPerpBusted` (`0x00890d98`) is referenced only by a
  hashed-getter function (`FUN_004080d0`). No arrest camera-track chooser was located, so the shot is not a copy of the
  original.
- **Guesses:** the 2.5 s pan, the 3.0 s arrest (the busted HUD time), the 0.6 s blend, the 48 degree field of view, and
  the 6 to 16 m side distance. The shot does not avoid walls, so the framing can clip scenery.

### Custom Chase Speedbreaker response

Chase and Dynamic FOV keep their free orbit. At the retained unscaled 60 Hz
camera tick, a custom adapter uses half the native zoom target strength: vertical
FOV approaches 80% and horizontal lag approaches 125%, rather than 60% and
167%. Far's existing speed-dependent eye/FOV spline durations smooth both
entry and exit. Height, orientation and free orbit remain unchanged; shared
obstruction and final clearance run after the transform. Physics samples carry
unzoomed custom intent so slowed simulation cannot apply the effect twice.
This is custom tuning, not a claim that Chase exists in the original game.
