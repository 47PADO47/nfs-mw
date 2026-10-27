# Explicit player-car timestep

- Date: 2026-10-25.
- Spec: [explicit player-car timestep](../specs/player-timestep.md).
- Source: existing Rust adapter and the bounded passive original-PC timing
  evidence described in [player driving provenance](player-driving-parity.md).
- Observed: the selected modded original uses normal delta `1/144` and full
  Speedbreaker delta `1/576` at approximately 144 real-time updates per second.
- Engine issue: `Vehicle` already accepts a delta, but its gameplay adapter
  hardcodes `1/60` for integration and associated per-step state.
- Additional source issue: the vehicle's body-ground contact adapter passes
  fixed `1/60` to the body-ground probe even when its enclosing step uses another
  delta. The supplied step delta must continue through that call.
- Implementation: expose one explicit delta through that adapter, retain the
  existing default entry points, and keep the delta in each split-step context.
- Validation scope: synthetic direct-vehicle agreement at multiple deltas,
  default-path equivalence and per-step timer/acceleration checks. These do not
  prove full driving trajectories, update scheduling or race-start behavior.
- No new binary inspection, executable bytes, installed assets or decompiled
  implementation were used for this infrastructure change.
