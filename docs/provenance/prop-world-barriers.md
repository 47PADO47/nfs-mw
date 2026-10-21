# Prop world barriers

- Spec: [Loose props against world barriers](../specs/prop-world-barriers.md).
- Sources: CC0 `dbalatoni13/nfsmw`, `RigidBody.cpp` world collision dispatch,
  barrier preparation, world reaction and `WCollisionMgr.cpp` barrier queries.
  Read-only inspection of the original PC executable confirms the world
  callback corrects position before invoking the response, including the
  no-reaction case. Binary identity and inspection output remain private.
- Rewrite defect: loose props had ground and object contacts, but never queried
  world barriers. Original mass and contact solver tuning cannot repair that
  missing collision stage.
- Validation: shape, side, ordering, material and exclusion regressions, plus
  installed source-field checks. Existing passive recordings validate object
  contacts; they do not record the world collision path or prop trajectories.
- Eight controlled zero-depth barrier geometry cases were compared with the
  original PC intersection routine. Contact acceptance, ownership and numeric
  outputs agree, including an original off-centre rejection. This is a known
  original limitation, separate from the remake's missing barrier stage.
