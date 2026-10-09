# blackbox-collision

Reader for the **collision data** of EA Black Box games (Need for Speed: Most Wanted today):

- `CollisionPack` (chunk `0x3B801`): a streaming section's static world collision: instances placing
  articles that hold triangle strips, barriers and a surface table.
- `Grid` (chunk `0x3B800`): the world-wide xz grid that lists which instances touch each cell.
- `BoundsSet` (`0x8003B900`): the boxes, spheres and point clouds of a car or prop.
- `CollisionWorld`: loaded packs plus the grid, with a segment query:
  `world.ray_cast(from, to, &RayOptions::default())` returns the nearest face or barrier hit with its point,
  normal and surface hash (a `simsurface` AttribSys key); `world.ground(x, z, top, bottom)` is a
  downward face cast.
- `world.ray_cast_filtered(from, to, &options, predicate)` selects the nearest accepted candidate.
  Rejected hits cannot hide another hit in the same article or a different instance. The predicate sees
  world-space hit data, including its section and instance; ordinary `ray_cast` remains unfiltered.
- `carp`: the generic `UGroup` tree the packs and the grid are made of.

Takes bytes, never opens files. Coordinates are the game's physics space (x right, y up, z forward).

Spec: `docs/formats/collision.md`. Tests: synthetic data always; the `#[ignore]`d ones read an install
(`NFSMW_GAME_DIR`).

License: MIT OR Apache-2.0.
