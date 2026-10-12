# High-speed effects

Status: **partly implemented**. The curves are in `libs/blackbox-speedfx`; the nitrous field-of-view kick is on the chase
camera; the streaks are drawn (`app/streaks.rs`, section 4). The radial blur is computed but not drawn yet.

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
| Streaks (opacity) | 40 to 75 | 1 | adds to 0.4 at full nitrous |
| Radial blur | 55 to 95 | 1 | adds 0.4 at full nitrous |
| Camera shake | 45 to 85 | 0.04 m | none |
| Field-of-view kick | none | 8 degrees | linear in nitrous |

The effects rise towards their target at 6 per second and fall at 2.5 per second (exponential approach), so a gear change
or a nitrous press does not jump. A non-positive step changes nothing.

**[unconfirmed]** These thresholds are tuning, chosen so the effects start where the dynamic FOV is widening. They are not
measured from the PC build. The install's own high-speed screen values are not read yet (Ghidra on `speed.exe` would give
them; the search is open).

## 3. Where it is applied

- The field-of-view kick: `WorldScene::gameplay_lens` adds `TrafficWorld::fov_kick()` to the chase lens, for both the
  plain and the dynamic FOV cameras. The shake of the chase camera is kept (the lens keeps the shaken view's offset).
- The police state holds the effects (`ai/police.rs`), updated each frame from the drive. It lives on the traffic
  world, so the effects only run on tracks that have a road network.

## 4. Open

- Streaks: drawn. `app/streaks.rs` builds 48 short quads that radiate from the window's centre and move out with time,
  with opacity from the streak value and a fade in and out per streak. They are composed onto the last HUD layer each
  frame (`MirrorOverlay::base_layer`), because the presenter only emits a layer when something changes. Not yet seen in
  a running game; the geometry and the composition are tested.
- Radial blur: not drawn. It is a screen pass in `blackbox-gpu-passes`, reading the frame before the HUD.
- Apply the shake amplitude to the chase camera.
- Read the install's values for the streaks and the blur, if they are in the data.
