# Scenery sequences and pursuit breakers

- **Spec:** [pursuit breakers](../specs/pursuit-breakers.md); format
  [model hierarchies](../formats/model-hierarchies.md).
- **Sources:** installed PC 1.3 event packages, attribute collections, collision
  and render hierarchies; the CC0 dbalatoni13/nfsmw event headers, generated
  hashes and WorldModel rendering; private executable inspection. The original
  executable SHA-256 and reference revision are recorded in the world-sequencer
  format/provenance documents. No original code, assets or event bytecode is included.
- **Measured behavior:** detachment emits WORLD_DETACHED (`0x697050`);
  ESpawnFragment resolves named children and respects visibility/spawn flags
  (`0x63B320`); physics-model initialization starts the attribute sequencer
  (`0x677830`, `0x6E9F20`); explosion callbacks send EXPLOSION and optionally
  EXPLOSION_DAMAGE, applying force when outward point velocity is below the
  expansion speed (`0x677100`). ESpawnExplosion uses three independent target
  bits: smackables, non-player vehicles and players (`0x63B120`). Positive
  IncludePlayers therefore survives ExcludeVehicles. Area messages carry the
  emitting model position (`0x62FB90`). Stop-cop messages retain an expiring
  spherical area (`0x62AF90`, `0x439E60`).
- **Implementation:** 2026-10-11, from the specification. Named scenery models,
  area stimuli, timed fragments, ownership, hide/kill/wake effects, expansion
  waves and cop braking run on the shared clock. Vehicle and scenery fragment
  budgets are separate. Missing art/bounds cannot hide an intact parent.
- **Installed verification:** all hierarchy records parse. Real car contacts
  trigger the installed tire-shop support (six surviving pieces and a hidden
  roof), beach scaffold (four pieces), and gas pump (31 surviving pieces through
  the support/roof chain). Each emits a stop-cop area. Their complete body,
  force, state, visibility and stop-event traces agree across 5, 15, 30, 60,
  120, 144 and 240 FPS and uneven frame durations. Parent tile unload removes
  every child and grid entry. CPU checks verify installed solids without
  fabricating GPU handles; rendered gameplay still needs comparison.
- **Limits:** first-contact IMPACT_1 is a trigger adapter; the full PC collision
  stimulus levels and original static-trigger lifecycle are not reproduced.
  Simple-physics requests use the full body solver. No-physics pieces do not
  collide. Global allocation competition with vehicle/static models is not
  modeled. Explosion broad phase uses body spheres and applies force at the
  body centre; original shell selection, point placement, damage, causality,
  occlusion and cinematic slowdown are unverified. Named joints in scenery
  fragment events are skipped. Model kill currently restores a placement at
  release. Cinematics, authored particle/audio effects, dynamic regions,
  pursuit score/wreck notifications and exact stop-cop AI semantics remain open.
