# Provenance: close pursuit wall query

This supplements [cop tactics section 3.4](../specs/ai-pursuit-tactics.md#34-wall-avoidance).
It establishes the primitive selection and ray origin, without changing the avoidance force calculation.

## Static evidence

- The CC0 reference `AI/Actions/AIActionPursuitOffRoad.cpp`, `UpdateAvoidWalls`, constructs a velocity ray
  from the rigid-body position and calls `CheckHitWorld` with primitive mask 2.
- Native PC `UpdateAvoidWalls`, address `0x00412310`, size `0x25c`, confirms that origin and mask.
  Its ray length is `min(80, speed + 10)` and the minimum speed is 2 m/s.
- Native PC `CheckHitWorld`, address `0x007854b0`, size `0x398`, sends mask bit 2 to the barrier-normal
  query at `0x007732b0`. Its separate mask bit 1 branch queries triangle faces. Mask 2 therefore excludes
  road faces; it is not a general surface query followed by a normal-angle filter.
- Ghidra identified the inspected program as x86 little-endian PE, image base `0x00400000`, SHA-256
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
  A second executable, SHA-256 `bde12bdd158b7f861078ad4527f5a656f34c6068e4a2120f28044f92f0fa158c`,
  has identical bytes for both bounded functions. Their SHA-256 hashes are respectively
  `5c662468d4d6854550d0d49f2dd867fcfc9b4764f328e1c395ed7a3c25288f37` and
  `ddfe5e98702528d27584f43d387ac1fdd7c2505f2200a7a2f52aea643991c90c`.

## Implementation and regression scope

The close-action adapter disables collision faces and keeps barriers enabled. It starts at the body's
position, without an added vertical offset. Other collision queries retain their existing contracts.
The verified force calculation, driver gains, speed limits and steering weights remain unchanged.

Synthetic adapter regressions cover a sloped road face, a real barrier beyond that face, and a low
barrier intersecting the body-height ray but missing a ray raised by 0.5 m. These distinguish the
primitive mask and exact origin from a general ray query or a surface-normal threshold.

These regressions do not establish sustained pursuit acceptance or identify the primitive responsible
for an earlier driving observation. That observation recorded an avoidance vector but not the hit kind.
No old-source execution was performed for these regressions before the correction.
