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
- The 2026-10-12 live PC contact capture and read-only bounds-pack constructor
  and lookup inspection establish first-collection precedence for duplicate keys
  within a pack ([spec](../specs/prop-attributes.md#duplicate-collision-collections)).
  The captured traffic-light primitive matches the first collection's narrow pole
  and offset; a later duplicate contains only the broad assembly root. Synthetic
  fixtures check both duplicate orders, and the ignored installed test checks
  selected source ownership and primitive construction. Reports and executable
  records stay private. Cross-pack load priority remains a separate parity gap.
- The 2026-10-12 PC placement audit follows the spawner transform, quaternion
  expansion, smackable constructor and rigid-body constructor listed in
  [world sequencing](../specs/world-sequencers.md). The stored quaternion is
  expanded without normalization when positioning the root pivot, and that
  centre is copied into the body unchanged. Normalizing first shifted 107
  installed centres by more than 2 mm, including three street lights by
  0.476–0.482 m. Synthetic placement tests cover both shorter and longer
  quaternions while checking a unit rigid orientation and unchanged visual pose.
  Original executable analysis is reference-only; no decompiled code or asset
  records are included. Exact non-unit orientation and visual-scale interaction
  remain separate parity work.
- The 2026-10-12 read-only PC static-collision audit traced world-assets track
  initialization, fixed-object creation, grid object gathering, geometry building
  and rigid-body collision dispatch. It establishes authored `CARP` instances and
  registered track boxes as static collision sources, with no scenery-name or
  name-suffix fallback in the inspected registration path. The installed track
  metadata contains one standalone `TrackOBB`; its 390 collision packs have no
  `co` records. These measurements do not classify every no-spawner visual:
  authored triangles and barriers may still block it. The
  [static collision specification](../specs/prop-attributes.md#authored-static-collision-sources)
  separates removing inferred boxes from preserving world collision and records
  the missing standalone track-box integration. Binary identity, decoded records,
  call edges and inspection output remain private; no decompiled code is included.
- Loose props now query enabled point-cloud vertices as well as ground box
  corners. Ground support regression uses cloud-only support and verifies complete
  body-state traces at 5–240 FPS and exclusion of disabled clouds.
- The 2026-10-12 compound-contact follow-up compares the same installed lamp and
  two intact car object boxes against read-only PC dispatch and isolated native
  impulse execution. A nonclosing deeper overlap must not suppress the later
  approaching pair. The private oracle applies the separately measured attached
  penetration rule between dispatch calls. An ignored installed regression uses
  both fast breakaway and slow attachment controls, checking finite motion and
  movable-contact reporting. See [box contacts](../specs/box-object-contacts.md).
  This does not establish global pair ordering or every lamp's live behavior.

## Runtime fragment initialization follow-up

Read-only PC inspection on 2026-10-12 follows the fragment event handler,
spawn descriptor, factory, smackable constructor and behavior-change callback
listed in [world sequencers](../specs/world-sequencers.md#runtime-fragment-attachment).
The descriptor clears the virgin-placement flag. That value reaches the
attachment callback unchanged, so runtime children are not attached merely
because their attributes have a nonzero detach threshold. This is distinct
from an authored placement's attachment and from material immobility.
Private instruction and call-flow records remain outside the repository;
no decompiled code or game records are included here.

The installed follow-up fixtures are the reported traffic-light replacement
and the previously tested street-light replacement. Both sequences respond to
`WORLD_DETACHED` and spawn a movable full-pole child using the same 200 kg,
7500-threshold attributes. The remake previously reused the placement rule in
its fragment constructor; attached bodies skip ground and motion updates.
This establishes a constructor error applicable to both fixtures. Synthetic
regression coverage checks placed versus runtime attachment and finite falling
motion; the ignored installed regression follows the actual replacement events.
Passing those checks does not establish every pole's behavior in a live drive.

The traffic-light sequence also contains an attribute-free, no-physics visual
piece. The original event handler spawns that piece through a visual-only branch.
The current adapter requires resolvable dynamics before handling no-physics
pieces and can omit it. That separate gap remains unimplemented. An authored
immobile foundation must stay immobile; joints and the original simple-physics
solver remain separate parity work.

This is a construction/support audit, not proof of every object's gameplay
behavior. Query tolerances, plane solver, centre markers, constraints, set-piece
sequencing and lifecycle still require original comparison.
