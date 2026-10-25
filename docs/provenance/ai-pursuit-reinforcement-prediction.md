# Pursuit road prediction and steering corrections

Date: 2026-10-25.

Specification: [shared prediction and continuous offsets](../specs/ai-pursuit-reinforcement-prediction.md).

Read-only sources:

- Original supported PC executable. Program identity was checked before native
  inspection. Bounded inspection of `WRoadNav` position-and-heading initialization,
  position-to-offset conversion and advancement confirms heading selection and
  retention of continuous non-traffic offsets. The lane-index traffic overload
  has a separate one-way-direction rule.
- Native `WRoadNav::UpdateOccludedPosition` confirms the directions of both apex
  projection vectors, accumulation of passed-cookie look distance and use of the
  current tangent in section interpolation. The reconstruction's opposite vector
  signs displaced the final steering point in the opposite direction.
- Previously inspected target seek caching and
  `AICopManager::SpawnPursuitIVehicle` establish that ordinary placement queries
  its target's existing seek prediction. Intentional placement-walk reversal and
  fallback heading remain separate rules.
- [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0 reconstruction,
  `WRoadNetwork`, `AIVehicle` and `AICopManager`, used as behavior references.
- Existing production cinematic control mapping establishes the AI PID input
  adapter for a physical player-car fixture. This changes input interpretation,
  not the loaded player chassis, performance or acceptance thresholds.

Implementation and synthetic fixtures were independently written from the
plain-language specification. No executable code, extracted road data, native
addresses or private capture contents are included.

This is a bounded correction, not a full parity claim. Complete native lane-change
blending, secondary/race target ownership, pitched-road steering and matched
original/remake gameplay remain separate work. Integrated results must keep
target validity, initial-unit performance and replacement behavior distinct.
