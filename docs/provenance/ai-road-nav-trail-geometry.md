# Provenance: trail endpoint geometry

This records the endpoint construction in [the trail specification](../specs/ai-road-nav-trail.md#2-corridor-bounds-verified-pc-endpoint-construction).
The bounds and the navigator's centre line must describe the same inset corridor.

## Native evidence

- PC `WRoadNav::SetBoundPos`, `0x0076f4a0`, size `0x7a7`: chooses a node profile for the requested endpoint,
  finds the nearest contiguous drivable zones, and constructs that endpoint's left and right bounds.
  The safety margin is vehicle half-width plus `1.5 m`. A corridor narrower than twice this margin is
  widened equally on both sides by half the shortfall.
- With an active trail and a non-traffic navigator, the function clamps the centre offset to
  `[left + 0.1, right - 0.1]` before writing the centre endpoint. Merely insetting the bounds while
  retaining an outer lane's original centre offset does not reproduce this behavior.
- `SetStartEndPos` calls this construction separately for the two endpoints. `SetControlPos` scales
  each curved spline's handles by that spline's own endpoint chord. Profiles are not switched halfway
  through a segment to choose its current bounds.
- `DetermineVehicleHalfWidth`, `0x0076f3d0`, reads the physical body's `dimension.x`. The half-width is
  not halved before adding the non-traffic safety margin. Traffic's separate narrow corridor uses
  half-width divided by two on either side of its centre.
- Native drivable and selectable masks match the current library for named zone kinds zero through
  thirteen. This evidence does not authorize changing road permissions or reducing safety margins.

## Identity

Ghidra identified `/speed.exe`, x86 little-endian PE at base `0x00400000`, SHA-256
`80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
The second inspected executable has SHA-256
`bde12bdd158b7f861078ad4527f5a656f34c6068e4a2120f28044f92f0fa158c`.
Both on-disk identities were checked before reading the bounded ranges. The following ranges match:

| Function | Size | SHA-256 |
|---|---|---|
| SetBoundPos | `0x7a7` | `703b44e199d611f1ae4876c36056cdd297dacc455d75770a06ad3056a6eef661` |
| RebuildSplines | `0x8d` | `a8ac0ee7eb49bfcc01feecd88cf454bb297ef4eb2ad4b49a6cb1d52e9cd93211` |
| UpdateCookieTrail | `0x256` | `59e940b1483a47b8aa28ed84c4dde976977906a598ddb6b74dc9d221fc03fefa` |
| DetermineVehicleHalfWidth | `0xa7` | `75190dfc059f5fb7cbfa05218ca657ee66688604115ede56a234ba4ad1a8fe7f` |

The CC0 reference `World/Common/WRoadNetwork.cpp`, already credited by the trail specification, supplies
function names and context. Implementation follows the English specification; no decompiled code is copied.

## Validation scope

Synthetic regressions cover an outer lane excluded by the safety inset, both travel directions, independent
endpoint profiles on a taper, and continuity across the midpoint. They check corridor geometry, not a
physical vehicle's ability to complete a chase. Curved roads, junction choices, obstacle shaping and sustained
city driving need their own validation. Static byte identity does not establish a running mod's patch state.

## Physical placement adapter

After enabling the corrected geometry, a rolling synthetic handoff must use the prepared cursor for both
the physical pose and the initial drive target. The previous adapter computed a spawn pose first, then
enabled the trail, leaving these positions inconsistent on outer lanes. This is a reconstruction integration
correction, not evidence that the original trail setter relocates an already existing vehicle. Final placement
acceptance uses the prepared road point; explicitly positioned scripted vehicles retain their authored pose.
