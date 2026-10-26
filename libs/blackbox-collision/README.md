# blackbox-collision

Reader for the **collision data** of EA Black Box games (Need for Speed: Most Wanted today):

- `CollisionPack` (chunk `0x3B801`): a streaming section's static world collision: instances placing
  articles that hold triangle strips, barriers and a surface table.
- `Grid` (chunk `0x3B800`): the world-wide xz grid that lists which instances touch each cell.
- `BoundsSet` (`0x8003B900`): the boxes, spheres and point clouds of a car or prop.
- `write_bounds_pack`: the write side of the above: sets back to `0x8003B900` bytes the parser reads (millimetre
  lengths, 16-byte aligned payloads).
- `fit_body`: builds a vehicle's `BoundsSet` from points sampled on its body: a root box of the extents, a lower body
  box, a cabin box, corner and roof spheres and a mirrored silhouette cloud for the ground (the template of the stock
  sets, see `body_fit`). `BoundsSet::check_vehicle_body` validates a set before a host builds a body from it.
- `BodyGeometry`: selects one body's immediate children or root, retaining separate fragment
  subtrees and disabled flags. `GeometryOptions` supplies fallback dimensions, shape, padding
  and material from the host. Construction evidence is in `docs/specs/world-sequencers.md`.
- `CollisionWorld`: loaded packs plus the grid, with a segment query:
  `world.ray_cast(from, to, &RayOptions::default())` returns the nearest face or barrier hit with its point,
  normal and surface hash (a `simsurface` AttribSys key); `world.ground(x, z, top, bottom)` is a
  downward face cast.
- `world.ray_cast_filtered(from, to, &options, predicate)` selects the nearest accepted candidate.
  Rejected hits cannot hide another hit in the same article or a different instance. The predicate sees
  world-space hit data, including its section and instance; ordinary `ray_cast` remains unfiltered.
- `world.barriers_near_into(center, radius, &options, &mut output)` collects actual
  nearby wall segments for body/primitive collision, sorted by authored local
  segment distance. It preserves world endpoints, inverse-length normal scaling,
  surface/flag and instance identity, and tests square grid candidates plus
  supplemental ungridded packs. Scenery-group membership always applies. Radius
  policy and narrow-phase response belong to the caller. Its behavior contract is
  `docs/specs/world-barrier-list.md`.
- `carp`: the generic `UGroup` tree the packs and the grid are made of.
- `world.camera_ray_cast(from, to, group_enabled)` preserves the original
  camera's barrier-only segment contract, raw determinant endpoint tolerance,
  stored-radius broad phase and authored world-horizontal normal scale.
  Its spec is `docs/specs/camera-barrier-query.md`; vehicle rays are unchanged.
- `ground_reference::GroundReferenceCache`: persistent ordered face selection
  with a predictive candidate region and retained terrain plane. Query settings
  and scenery-group state come from the caller; streaming revisions rebuild
  candidates. Behavior is specified in `docs/specs/vehicle-ground-reference.md`.
  Its `point_height` query samples a fresh face at a separate point over that
  retained region, with caller-supplied tolerance and no arbitrary ray depth.

Takes bytes, never opens files. Coordinates are the game's physics space (x right, y up, z forward).

Spec: `docs/formats/collision.md`. Tests: synthetic data always; the `#[ignore]`d ones read an install
(`NFSMW_GAME_DIR`).

License: MIT OR Apache-2.0.
