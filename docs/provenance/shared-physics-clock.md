# Shared real-time physics cadence

- Date: 2026-10-25.
- Spec: [shared real-time physics cadence](../specs/shared-physics-clock.md).
- Sources: existing Rust shared-world pipeline and prior guarded passive
  original-PC timing evidence recorded in [player driving parity](player-driving-parity.md).
- User-selected policy: shared 60 real-time updates per second, with smaller
  simulation deltas during Speedbreaker; unlocked rendering; optional 144-Hz
  local comparison mode. AI retains its control laws but shares the selected
  cadence and scaled delta with player and prop motion.
- Implementation rule: retain partial real-time phase, update the original
  single-precision multiplier once per completed tick and multiply its final
  value by that tick's period. Preserve the separate continuous frame integral
  for camera/visuals/look, and the network 60-Hz one-per-update path.
- Numeric evidence: isolated execution of the selected executable's original
  time-manager routine supplies six successive entry and exit multiplier/delta
  vectors at both 60 and 144 Hz. The first six 144-Hz entry outputs also match
  the guarded live player capture. The earlier per-tick area-integral proposal
  was rejected because it differed from this endpoint ordering.
- AI retains its existing eight-/ten-tick thought decisions. Elapsed-time
  control work consumes accumulated actual simulation deltas, including the
  legacy nominal-count initialization of a first thought. Existing helicopter
  and roadblock accumulators already operate in simulation seconds.
- Validation scope: synthetic cadence, analytic ramp integration, frame grouping,
  invalid/loading pauses, mode changes and legacy/network default equivalence.
  Whole original-game trajectories and contact order remain separate boundaries.
