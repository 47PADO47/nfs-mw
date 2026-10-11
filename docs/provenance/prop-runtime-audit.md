# Exhaustive prop runtime audit

- Data is read from the user's installation using the serialized spawner keys
  and scenery override hooks. Original files and audit output are not bundled.
- [Attribute specification](../specs/prop-attributes.md) records mass, inertia,
  collision channels, attachment and ground support rules before implementation.
- The ignored runtime test checks all 13,484 placements: 13,479 bodies resolve;
  the same five missing bounds records are explicitly rejected. Coverage includes
  97 attribute classes, 2,409 large bodies and 2,734 ground clouds.
- Each body is checked for original model identity, bounds/attribute keys, stable
  ID, mass, inertia, centre, attachment state, original initial visual matrix,
  enabled object primitives, finite collision bounds, registration and unload.
- Shared geometry/attribute records previously cached the first model's name.
  There are 173 subsequent differing-name visits in the installed map audit;
  identity now belongs to the visible model while source bounds/attributes share
  ownership. A synthetic regression also exercises cache order without game data.
- Loose props now query enabled point-cloud vertices as well as ground box
  corners. Ground support regression uses cloud-only support and verifies complete
  body-state traces at 5–240 FPS and exclusion of disabled clouds.
- This is a construction/support audit, not proof of every object's gameplay
  behavior. Query tolerances, plane solver, centre markers, constraints, set-piece
  sequencing and lifecycle still require original comparison.
