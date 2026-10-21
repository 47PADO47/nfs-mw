# Police vehicle appearance provenance

The implementation follows the facts specified in
[cop-visible-damage](../specs/cop-visible-damage.md), written before code changes.

Reference-only sources: `dbalatoni13/nfsmw` reconstruction, particularly
`World/VehiclePartDamage.cpp`, `World/CarRender.cpp`,
`World/CarRenderConn.cpp` and `World/VehicleRenderConn.cpp`.
No source code was copied or translated into the implementation.

The local PC executable's identity was confirmed before bounded inspection.
Its part constructor, damage-zone application, successor search, emitter
initialization and health effect selection were checked independently against
the matching functions and constant tables. The playable executable's relevant
function bytes match those inspected. Raw addresses, hashes, binary tables,
decompiler output and installation data remain private research material.

The source zone-to-slot table includes duplicate associations. Membership is
what affects selection; the implementation expresses the unique associations.
Successor selection is in database/type search order and accepts any upgrade
level. A null successor is an absent part, not a fallback to pristine geometry.

The installed heat-one fixture exposed the assembly guard's ordinary-body
assumption. Police `DAMAGE_BODY` models provide body geometry at B-D;
`CarRenderInfo` explicitly handles this body slot. Their valid lower assemblies
must remain available even when `BASE` and ordinary `BODY` models are absent.

Fresh matching-PC inspection of ride initialization, special slot ranges,
part-model hash lookup, model replacement and the render part loop confirms
that normal rides can select E and police damage slots have no special D cap.
Missing selected models are skipped. The reference's unreferenced static body
maximum is not evidence of a runtime D clamp. Installed tests distinguish
pristine A-E from damaged A-D and retain the original missing E behavior.

The health effect references name `emittergroup` records, not `effects`
wrappers. This distinction is necessary even though both eventually consume
the shared authored particle emitter format.

The follow-up hood persistence regression checks the production actor after
elapsed event ticks, subsequent contacts, LOD round trips and shared material
rebuilding. Zone levels are monotonic, and fragment cleanup does not clear
the owning actor's hidden-slot latch. This is a regression boundary, not a
demonstrated cause of the reported hood snapping closed. A passing fixed-pose
probe does not resolve the user's moving-camera appearance issue or establish
original animated-panel parity.

The subsequent timed-path audit identified a concrete mismatch: the installed
hood action emits its fragment on its 20-second end tag, while fragment
extraction unconditionally selected pristine A geometry. After rechecking
program identity, PC `VehicleFragmentConn::UpdateModel` (`0x75AE10`) calls
`VehicleRenderConn::FindPart` (`0x738960`) for the parent's slot; that helper
reads the current slot/LOD model array. Fragment extraction now preserves the
actor's selected damage variant. An installed collision-to-action-to-cargo
regression checks the delayed replacement and permanent parent hiding after
fragment cleanup. This establishes a plausible specific cause for the visible
snap, while the exact user's moving-camera reproduction remains unverified.
