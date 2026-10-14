# High-speed effects

Status: **partly implemented**. The curves are in `libs/blackbox-speedfx`; the nitrous field-of-view kick is on the chase
camera. The automatic pursuit-driven HUD streak feed was removed because it
could remain visible at rest. The independent graphics speed-lines option
remains available and follows actual vehicle speed.
The radial blur is computed but not drawn yet.

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

## 4. Open

- The prototype pursuit-driven HUD streak feed was removed. Optional streaks
  now follow actual speed rather than retained pursuit state. See
  [player-car-lights.md](player-car-lights.md).
- Radial blur: not drawn. It is a screen pass in `blackbox-gpu-passes`, reading the frame before the HUD.
- Apply the shake amplitude to the chase camera.
- Read the install's values for the streaks and the blur, if they are in the data.
