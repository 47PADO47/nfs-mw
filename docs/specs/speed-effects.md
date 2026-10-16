# High-speed effects

Status: **partly implemented**. The curves are in `libs/blackbox-speedfx`; the nitrous field-of-view kick is on the chase
camera. The automatic pursuit-driven HUD streak feed was removed because it
could remain visible at rest. The independent graphics speed-lines option
remains available and follows actual vehicle speed.
The radial blur is drawn by the native renderer as a screen pass; the Bevy backend does not draw it yet.

## 1. What is in the rewrite already

- **Dynamic FOV** (`viewer/camera/dynamic_fov.rs`): the view widens with speed, cubic ease from 45 to 110 degrees at 80 m/s.
- **Acceleration shake** (`viewer/camera/chase/shake.rs`): the chase camera shakes on hard acceleration.
- **Wind trails** (`scenes/world/vehicle_effects/trails.rs`): the speed contrail above a threshold, as described in
  [vehicle-effects.md](../vehicle-effects.md).

These are not changed by this work.

## 2. The curves (`blackbox-speedfx`)

Each effect is a ramp of the speed (m/s, either direction), plus nitrous:

| Effect | Ramp (m/s) | Maximum | Nitrous |
|---|---|---|---|
| Radial blur | 55 to 95 | 1 | adds 0.4 at full nitrous |
| Camera shake | 45 to 85 | 0.04 m | none |
| Field-of-view kick | none | 8 degrees | linear in nitrous |

The blur, shake and field-of-view kick rise towards their target at 6 per second and fall at 2.5 per second (exponential
approach), so a gear change or a nitrous press does not jump. A non-positive step changes nothing.

The speed streaks are not in this curve. They follow the car trail's curve (`scenes/world/vehicle_effects/trails.rs`,
`intensity`): nothing below 44 m/s, then 0.1 rising to 0.75 at 88 m/s, with no smoothing, so the two effects start and
grow together. They are on when the Speed Trails setting is on with the **Speed Streaks** style (the default).

**[unconfirmed]** These thresholds are tuning, chosen so the effects start where the dynamic FOV is widening. They are not
measured from the PC build. The install's own high-speed screen values are not read yet (Ghidra on `speed.exe` would give
them; the search is open).

## 3. Where it is applied

- The field-of-view kick: `WorldScene::gameplay_lens` adds `TrafficWorld::fov_kick()` to the chase lens, for both the
  plain and the dynamic FOV cameras. The shake of the chase camera is kept (the lens keeps the shaken view's offset).
- The police state holds the effects (`ai/police.rs`), updated each frame from the drive. It lives on the traffic
  world, so the effects only run on tracks that have a road network.

## 4a. Radial blur pass (native)

- **Feed.** The police state's blur (section 2) goes to the backend each frame as `FocusFilter::radial_blur`, with
  `FocusFilter::radial_centre` at the middle of the screen. It is the same retained filter the speedbreaker vignette
  uses, so there is one screen-effect hook, not a second one.
- **Pass.** `blackbox-gpu-passes` `RadialBlur`: a pixel averages `taps` samples on the ray from itself to the
  centre. The sample at position `t` (0 at the pixel, 1 at the far end) weighs `1 - t / 2`, and the weights are
  normalised to sum to one. The reach is 0 inside `inner` (UV from the centre) and rises to `strength` at the
  corners. Defaults: `inner` 0.25 and 12 taps, both **[unconfirmed]** tuning, not read from the install.
- **Position.** After the post effects (bloom, tone mapping, FXAA) and before the speedbreaker mask and `resolve`,
  so the blur smears the finished frame, and the vignette adds on top of it. Blurring before bloom would glow the
  smeared highlights instead; that was not chosen.
- **Cost.** At zero strength the pass is not in the chain: no pass, no scratch image.
- **Source.** No sampling kernel for the speed blur is recorded yet. The speedbreaker spec says a radial scalar alone
  does not establish a kernel, and that other original blur passes need active-path evidence first
  ([speedbreaker-visuals.md](speedbreaker-visuals.md)). So the sampling above is the rewrite's own choice, not a
  reproduction of the original.

## 4b. Motion blur settings (port)

The mod `NFSMWMotionBlurController` tunes the original game's blur. The port has no such option set; its blur is the
curve in section 2, drawn by section 4a. The settings below scale that curve. Each is a flat key in the config file.

- **One path, no widescreen toggle.** The port draws one aspect-correct blur (section 4a), so the mod's
  `MotionBlur` switch (widescreen fix or vanilla path) has no port setting. The mod's switch picks between two
  original code paths; the port has only one.
- **On or off.** `motion_blur` (default on), env `NFSMW_MOTION_BLUR`, flag `--motion-blur` / `--no-motion-blur`,
  console `set motion_blur on|off`, menu row Motion Blur (High Speed). Off draws no blur. It covers the mod's
  `g_MotionBlurEnable` as well.
- **Strength.** `motion_blur_strength` (default 1.0, range 0 to 2), env `NFSMW_MOTION_BLUR_STRENGTH`, flag
  `--motion-blur-strength`, console `set motion_blur_strength 0.5`, menu row Motion Blur Strength (steps of 0.1). It
  multiplies the curve's blur and clamps to the full blur. 1.0 is the curve as it is. The mod's `MotionBlurAmount`
  (default 0.25, vanilla 1.0) has the same reference point, but the numbers do not transfer: the mod scales the
  original binary's blur, and the port scales its own.
- **Minimum speed (not applied yet).** The mod's `MotionBlurMinEffectiveSpeed` maps to a planned `motion_blur_min_speed`
  (m/s). The default would be the curve's own start, 55 m/s (`Tuning::default().blur_from`), not the mod's 25 m/s,
  so nothing changes by default. `Tuning::with_blur_start` moves the start and keeps the ramp width. It is not wired:
  the curve is built and smoothed in `ai/police.rs`, so the setting needs that file to take the tuning, and a live
  setter on the traffic world. It is left out of the settings until then, so nothing silently ignores it.
- **Live reload.** The mod polls its INI file. The port needs no watcher: the settings change path re-applies
  `WorldTuning` (which carries the blur settings) to the scene at once.

## 4. Open

- The prototype pursuit-driven HUD streak feed was removed. Optional streaks
  now follow actual speed rather than retained pursuit state. See
  [player-car-lights.md](player-car-lights.md).
- Radial blur, Bevy: not drawn. The native renderer draws it (section 4a); the Bevy backend accepts the value
  and reports `radial_blur: false` in its capabilities.
- Apply the shake amplitude to the chase camera.
- Read the install's values for the streaks and the blur, if they are in the data.
