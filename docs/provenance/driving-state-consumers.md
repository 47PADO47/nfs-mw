# Driving state consumer provenance

Date: 2026-10-26. Spec: [driving state consumers](../specs/driving-state-consumers.md).

The consumer correction follows a current-source producer/caller audit. Shared
body/pair/contact ordering follows [the existing phase provenance](shared-vehicle-phases.md),
including proprietary PC behavior inspection and CC0 reference corroboration.
No restricted source is ported.

Evidence distinguishes completed vehicle state, contact-entry snapshots, raw
pedals, shaped controls and render interpolation. Regression tests must exercise
state transitions and contact consumption, rather than only compare two accessors.
A passing stale-state regression does not establish OG particle/audio parity,
matched trajectories or complete multiplayer world rollback.

The source audit also found that meter depletion followed clock-target selection
but preceded vehicle effects, and passive recharge used the pre-batch speed with
the sum of future tick deltas. The correction separates completed-tick recharge
from loaded-frame edges/depletion and selects one active state before clock and
handling updates. Existing Speedbreaker constants and frame/zero-tick policy are
retained. Exact original within-frame depletion scheduling remains unverified.
