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
visible section advances the clock. The controller behavior for other flags is specified below. The track's ordinary records use flag 0x28,
no trigger, no track restriction and no direction. Missing models or channels
must not produce fabricated geometry. The airliner's three channel names also
identify their individual `_10`, `_11` and `_12` solids, while their common
entity model hash does not resolve in the installed stream. The implementation
uses those channel names as a fallback when the entity model is absent.

The separate simple-model animation runs continuously at 50 degrees per second.
`XB_DONUTSIGNB_1B_BK_00` rotates around local Z and
`XO_WINDMILL_BLADE_1B_JG_00` around local Y. The animation is applied to
matching solid meshes; the donut's breakable prop sequence is separate.

## PC playback controller

Static inspection of the retail PC controller confirms the following rules.
This establishes code behavior, not a matched runtime capture.

- Evaluation uses frames at 30 Hz. Multiply elapsed seconds by speed (zero
  selects one). Each entity owns its controller and channel duration.
- Zero flags select linear looping (0x28). Linear 0x8 wins over ping-pong
  0x10; neither selects linear. Loop is 0x20, reverse-linear is 0x1000.
- Inline frame ranges are unsigned 16-bit values at instance offsets 28/30.
  Flag 0x40 selects them; both range endpoints are multiplied by speed.
  The controller starts at frame zero, including reverse/ranged playback.
- Delay values are seconds. Initialization enables both master and loop delay
  flags. Master delay precedes local delay, and both are bypassed at/after the
  end frame. Crossing a delay adds the elapsed-delay remainder to evaluation;
  it does not apply direction or endpoint processing on that tick.
- Endpoint crossing is strict. Linear looping corrects one range length per
  update; non-looping clamps to the endpoint but retains playing state.
  Ping-pong reflects the forward overshoot, switches direction, and clamps at
  the beginning on the reverse crossing. Reverse crossings reset loop delay.
- A start or stop trigger hash forces ping-pong and triggered flags (0x810),
  disabling linear. Triggered ping-pong pauses at each crossed endpoint.
  Instances with a start trigger or track direction begin stopped; others play.
- The inspected PC scenario initializer creates only the generic NIS controller.
  The trigger-zone and forward/reverse-track slots remain empty. Their fields
  alone do not establish autonomous game-world activation to reproduce.
- Track filters admit either of two inclusive byte ranges of track number modulo
  100 (low pair: min in bits 8..15, max in 0..7; high pair: min in 24..31,
  max in 16..23). Track direction is separate from reverse-linear playback.

All eleven inspected L2RA instances use ordinary loops; the advanced modes are
covered with synthetic controller tests. Random start/delay flags and generic
NIS world-control delivery are not validated by this game-world-only work.

Track-restricted placements remain omitted when the remake has no verified
numeric original track identity. Random phase/delay flags are also not admitted.
Neither restriction removes an instance from the inspected stock L2RA track.
