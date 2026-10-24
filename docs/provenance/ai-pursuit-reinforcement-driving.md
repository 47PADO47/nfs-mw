# Reinforcement road chase provenance

Date: 2026-10-23.

Specification: [reinforcement driving](../specs/ai-pursuit-reinforcement-driving.md).

Read-only sources:

- Original PC executable, identified by the project's supported executable
  identity before inspecting its road-chase cursor update. Native inspection
  confirms the tangent/look-ahead-circle quadratic and positive-root advance.
  A separate bounded inspection of the traffic action's eligibility, action
  start, drive-nav reset and nav constructor confirms that an attached pursuit
  tests lost sight directly. Its action handoff changes nav and lane types,
  resets and selects a lane, preserves segment filters, and does not apply
  civilian spawn permission or select a random lane.
  Further bounded native inspection confirms per-target current/future road
  walkers, incremental updates, periodic physical-position validation and a
  shared seek cache. The seek walk uses the retained road identity rather than
  reselecting a road independently for every pursuing cop. The game adapter
  applies this contract to the published primary free-roam target. Secondary
  co-op and authored race-route tracking remain stated integration boundaries.
- [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0 reference,
  `AIActionRace`: predicted-road path search and separation from tactical
  formation steering. Used as a behavior reference under the spec-first policy.
  `AIActionTraffic`, `AIVehicle::ResetDriveToNav` and `WRoadNav` supply the
  corresponding action, lane-selection and filter-preservation reference.
- Existing `ai-pursuit-driving-integration.md`, road chase and turn-around.
- A privately retained modded-PC pursuit capture across heats 1-5. It supplies
  real vehicle trajectory and road-cursor observations, not a stock-game
  equivalence claim. No capture contents or executable code are included here.

Implementation is independently written from the specification. Tests use
synthetic roads and vehicles. Pending test execution and installed encounter
results must be recorded by the integrating change; no full parity claim is
made from the mathematical helper or synthetic tests alone.

## Target direction integration boundary

The close/off-road action derives its road direction from the persistent future
road point minus the current road point. The shared primary snapshot preserves
that input meaning. The game adapter projects the direction into its existing
horizontal steering model; this does not establish full pitched-road equivalence.

The road-action speed cap has a distinct native input: the target vehicle's
drive-navigator point relative to its physical body, when that interface exists.
The adapter already substituted current-to-future road direction for this
steering alignment. Shared road tracking retains this approximation. No matching
physical player navigation source was established in this change, so neither
native steering-alignment equivalence nor a new governor correction is claimed.

Primary tracker ownership, copied observations, absent-target suppression and
state hashes are implemented. Generic road tracker tests have passed; game
integration tests and installed city chase acceptance are pending. Secondary,
race-route and shared co-op target-memory limits remain explicit in the
specification and implementation coverage report.
