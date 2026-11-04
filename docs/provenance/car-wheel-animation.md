# Drawn wheel steering and roll

- **Spec:** [../specs/car-wheel-animation.md](../specs/car-wheel-animation.md).
- **Sources read for understanding (no code copied):** the original PC executable's car renderer update (Ghidra, behaviour
  only; program identity confirmed first): the drawn steering's slew by `SteerSpeed`, its clamp to `MaxTireSteer`, the
  100 m distance gate, the per-update roll cap of 17 degrees or the spoke rule, and the wrap by one turn. The CC0
  [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) reconstruction mapped the renderer's update order.
- **Data:** `ecar.SteerSpeed`, `ecar.MaxTireSteer`, `ecar.WheelSpokeCount` (read by `WheelSetup::read`).
- **Date:** 2026-11-02.

## Implementation

`crates/nfsmw/src/scenes/world/drive/sim/wheel_visuals.rs`: the limits from the car's wheel set-up, the per-step drawn
steer and roll, and the short-way roll interpolation used by `CarPose::lerp`. The car sim emits the drawn front-wheel
angle in its pose; the physical angle and the AI steering input are untouched. AI cars beyond 100 m from the view are
drawn with straight wheels.

## Checks

- Headless: the unit tests in `wheel_visuals.rs` (slew, clamp, noisy input, spoke rule, wrap and interpolation).
- Installed: `cop_cars_carry_drawn_steering_limits` (nfsmw-data, `NFSMW_GAME_DIR`) reads both values for each cop car.
- Not compared frame by frame with a running original.
