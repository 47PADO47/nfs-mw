# Remaining world interaction work

Further parity features are deferred. The current priority is playable movement
with fixed-step simulation and smooth presentation. Full original PC 1.3 parity
has not been established; installed-data checks and isolated original routine
fixtures do not replace a comparison while playing both games.

## Next comparisons

- Reproduce the reported fences that return too quickly after an impact.
  Compare offscreen timers, dropout/resting states, streaming ownership and
  placement restoration against the original, including leaving and returning
  to the area. This report remains unresolved.
- Reproduce the reported parked cars that feel too light. Check their installed
  identity, collision geometry, mass/inertia, attachment and temporary reaction
  overrides against original/remake impacts. This report remains unresolved.

- Record repeatable original/remake routes and impacts with the same car, speed,
  direction and object. Include light props, large attached objects, traffic,
  head-on/rear/side impacts, curbs, slopes, jumps and fallen debris. Measure
  speed loss, translation, rotation, attachment and cleanup times.
- Revisit the reported light player-car collision feel and large objects that
  react like small objects. Placement identity, mass, inertia and attachment
  already come from original attributes. Audit contact centers, compound
  ordering, response goal states and AI overrides against captured behavior;
  correct the responsible rule rather than applying a global mass multiplier.
- Repeat long routes with streaming and traffic at 30/60/120/144/240 FPS and
  uneven frames. Check both identical fixed-tick outcomes and continuous
  camera/world/car presentation. Profile GPU uploads and missing local collision
  data separately from camera interpolation. Local collision packs that are unavailable
  still require a simulation pause.

## Physics and lifecycle

| Area | Still needed | Acceptance check |
|---|---|---|
| Car/prop and car/car response | Original collision goal states, compound contact ordering and contact-center markers | Match impact speed loss, impulse and angular response in repeatable original/remake captures |
| Ground | Cached face/world queries and exact world-position cache behavior | Curbs, slopes, seams, tumbling and landing match original captures without render-rate changes |
| Scenery identity/triggers | Static trigger creation/reactivation and complete impact-stimulus mapping | Correct large/small object behavior; every intended trigger fires once and restores at the correct time |
| Prop cleanup | Frustum/LOD/occlusion equivalence and exact placement recreation timing | Fallen objects remain, sink, disappear and return at the same visible/offscreen times as the original |
| Trailer constraints | Unused cement trailer's rotated spawn/suspension frame; singular rotations, original angle lookup and immobile angular-limit response | All six variants spawn and settle, remain hitched through turns/braking/impacts and match original joint motion |
| Released truck cargo | Trailer-local explosions, shock/load-loss feedback and full lifecycle comparison | Seven logs/three carrier cars release, collide, clean up and produce the original feedback |

Ground contact iteration/tolerances, per-object timers, shared fragment ownership
and the five trailer variants used in installed traffic patterns are implemented.
Their remaining work is behavioral comparison and the specific gaps above.
See [coverage and evidence](world-interactions.md),
[lifetime rules](specs/prop-lifetime.md) and
[constraint provenance](provenance/trailer-constraints.md).

## Pursuit breakers and other world events

- Finish exact initial impact thresholds, static trigger reactivation, sequence
  target/filter semantics and currently unsupported sequence operations.
- Implement the original simple-physics path, named fragment joints and global
  model-budget competition. The current fragment adapter uses full rigid bodies.
- Match explosion shell/point selection, occlusion, damage and event causality.
  Current waves use sphere broad phase and a center-force adapter.
- Add authored breakup particles/audio, cinematic cameras/slowdown, dynamic
  regions, music and speech hooks.
- Complete pursuit score/cop-wreck notifications and exact stop-cop AI behavior.
- Implement spike strips and remaining scripted world set pieces.
- Test every installed breaker in gameplay, including repeated impacts, passing
  traffic, entering cops, leaving/returning to the area and parent/tile unload.

Support-to-roof, scaffold and gas-pump chains already run from installed
sequences, with named fragment art/bounds, timed events, explosion waves and
temporary cop-stop areas. Those checks establish integration coverage, with the
limits in [breaker provenance](provenance/pursuit-breakers.md).

## Release acceptance

Every follow-up keeps one 60 Hz world timeline. Inputs, impacts, joints, debris,
timers and event queues must give equivalent fixed-tick results across render
rates. Rendering must interpolate all related poses without changing simulation.
Run workspace guards, formatting, clippy and tests; add installed checks for
the affected feature, then capture its actual playable behavior. Each build
must state what was checked and what remains unverified.
