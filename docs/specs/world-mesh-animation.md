# World mesh animation

The track's EAGL world animation data is distinct from texture animation,
wind sway and physics-driven props. Sources: the installed PC track metadata,
and `Animation/AnimWorldTypes.hpp`, `AnimEntity_WorldEntity.cpp`,
`AnimWorldScene.cpp` and `WorldAnimCtrl.cpp` from the CC0 dbalatoni13/nfsmw
reference. See [provenance](../provenance/world-mesh-animation.md).

The 16-byte-aligned `WorldAnimEntities` payload is an array of 112-byte
records. Each has a name hash at 4, model hash at 12, tree hash at 32,
animation name hash at 36, channel flags at 40, signed parent index at 41,
and row-vector local matrix at 48. The following `WorldAnimTreeMarkers` chunk
names the tree these records belong to. Parent indices refer to preceding
records; an entity whose name matches the tree is its root.

`WorldAnimInstances` holds 160-byte records: tree hash at 4, section number
at 8, play flags at 12, speed at 16, delays at 20 and 24, trigger hashes at
32 and 36, track restriction at 56, direction at 60 and instance matrix at
64. Root placement comes from the instance matrix. A non-root entity with a
model hash draws its mesh as a child of its parent. An animated entity replaces
its local matrix with the global matrix of bone 1 from the ROOT skeleton,
sampled from the tree's EAGL channels. The channel names are the entity's
animation hash continued with `_q`, `_s` and `_t`; flags 2, 4 and 1 select
rotation, scale and translation. Evaluate in that order.

For the ordinary free-roam track records, flags 0x28 select forward looping;
elapsed time is multiplied by the instance speed (zero uses one), and a
visible section advances the clock. This implementation admits that verified
subset only. Triggered, track-specific, ranged, reverse and delayed motion
need their own behavior and tests. The track's ordinary records use flag 0x28,
no trigger, no track restriction and no direction. Missing models or channels
must not produce fabricated geometry. The airliner's three channel names also
identify their individual `_10`, `_11` and `_12` solids, while their common
entity model hash does not resolve in the installed stream. The implementation
uses those channel names as a fallback when the entity model is absent.

The separate simple-model animation runs continuously at 50 degrees per second.
`XB_DONUTSIGNB_1B_BK_00` rotates around local Z and
`XO_WINDMILL_BLADE_1B_JG_00` around local Y. The animation is applied to
matching solid meshes; the donut's breakable prop sequence is separate.
