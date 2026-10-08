# World collision queries

Module: `blackbox-collision` (`CollisionWorld::ray_cast`, `Grid`, the pack and bounds readers).

- **Spec:** [docs/formats/collision.md](../formats/collision.md) ("Query semantics")
- **Sources read for the spec:** dbalatoni13/nfsmw (CC0, decompiled): `World/Common/WCollisionMgr.cpp`,
  `WCollisionPack.cpp`, `WCollisionAssets.cpp`, `WGrid.cpp`, `WWorld.cpp`, `WWorldPos.cpp`,
  `WWorldMath.cpp`, `World/WCollision.h`, `Libs/Support/Miscellaneous/CARP.h`,
  `Libs/Support/Utility/UGroup.hpp`, `Physics/Bounds.h`. Read for understanding only.
- **Implemented:** 2026-10-08, from the spec and from measurements on the install (decompiled code not open
  while writing the Rust). The file layouts are facts measured on the install; the query is a segment
  test written from the spec's description.
- **Checked against the game by:** structure and size relations of all 390 packs, the grid and 491 bounds sets
  (`#[ignore]`d tests in `libs/blackbox-collision/src/tests/install.rs`); rays dropped onto strips of the
  first sections always hit them.
- **Known differences from the original:** the segment test is a plain segment/triangle test, where the
  game works in a frame aligned with the segment; results agree where a hit exists but edge cases
  (grazing a shared edge) can differ by the 1e-6 tolerance. The grid walk covers the xz bounding box
  cells that the segment touches, not the game's 100-cell cut-off. Scenery-group enabling is a flag on the
  query (`skip_groups`), not the game's runtime group table. The ground-height point query (spec §5)
  is not implemented; `CollisionWorld::ground` is a vertical segment cast.
