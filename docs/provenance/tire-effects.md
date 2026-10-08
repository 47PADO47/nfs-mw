# Tire effects

- **Spec:** [tire-effects](../specs/tire-effects.md).
- **Sources read for the spec:** existing `blackbox-vehicle`, world collision,
  driving and `blackbox-render` source in this repository, MIT OR Apache-2.0.
  No restricted sources, original executable or external source were read.
- **Implemented:** 2026-10-08, from the above spec.
- **Checks:** synthetic contact/lifetime/budget tests and deterministic driving
  captures; local interactive results are recorded in the contribution test notes.
- **Known differences from the original:** procedural visual design. Original
  tire texture selection, weather/surface-dependent appearance and original
  emission/lifetime parameters have not been researched or reproduced.

The physics intensities were already implemented by `blackbox-vehicle`. This
contribution consumes its public outputs and does not alter the tire model.
Neither procedural geometry nor its analytic masks contains game assets.

## Displayed tread alignment

The follow-up [alignment spec](../specs/tire-visual-alignment.md) uses this
repository's public CarModel bounds and Corner transforms. Effects project the
displayed tread midpoint onto resident ground, retaining the physics intensities.
An installed BMW test measured 0.112-0.123 m of planar separation between the
previous physics locations and displayed treads. This establishes an internal
alignment defect, not an original-game parity measurement. The implementation
does not use executable research or external code.

## Optional high-quality smoke

The [high-quality smoke spec](../specs/high-quality-smoke.md) defines a new
procedural visual mode. NVIDIA GPU Gems 3, chapter 23.4 was consulted for the
soft-particle concept. No listing or source code was copied. The mask, reverse-Z
depth reconstruction, settings and bounded emission are original code. This
mode improves detail and intersection fading; it makes no original-game parity
claim and does not simulate particle collisions or volume lighting.
