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
