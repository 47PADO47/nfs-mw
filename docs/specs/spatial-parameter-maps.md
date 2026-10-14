# Spatial parameter maps and audio context

Sources: [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw)
(CC0-1.0), `ParameterMaps`, `SFXCTL_Tunnel`, `TrackPath`; independently
checked against the matching PC executable and installed metadata. This is
a behavioral description, not copied source. Provenance:
[spatial-parameter-maps.md](../provenance/spatial-parameter-maps.md).

## Reader and lookup **[decomp, PC checked]**

The parameter-map container is bChunk `0x8003B600`. Each direct layer child
`0x8003B601` contains these little-endian leaves:

| ID suffix | Payload |
|---|---|
| B602 | 36-byte header: name hash; four f32 bounds left/top/right/bottom; signed counts for quad nodes, sets, bytes/set, fields |
| B603 | one i32 field type per field: float0, integer1 |
| B604 | one i32 byte offset per field |
| B605 | contiguous fixed-size parameter sets |
| B607 | four u8 words per quad node |
| B608 | four little-endian u16 words per quad node |

When both quad widths exist, the byte form takes precedence. No quad data
means set0. A node whose first word is zero is a leaf; its second word is
the parameter-set index. Other nodes hold four absolute node indices.
Start at node0 with the header bounds. Split at the f32 midpoints. Child
selection is `(x >= midpoint_x)*2 + (y >= midpoint_y)`; update the selected
quadrant bounds, then repeat. Equality selects the upper/right child.

The original early-return condition is the conjunction
`x < left && x > right && y < top && y > bottom`, returning set0. This is
not an ordinary outside-rectangle test and must not be rewritten as one.
With ascending installed bounds it never holds; points outside the bounds
still traverse to edge leaves. Nonfinite query coordinates are rejected by
the remake. Malformed sizes, offsets, indices, cycles and unsupported field
types are errors; they must not permit out-of-bounds reads or infinite loops.

## Audio producer **[decomp, PC checked]**

The layer name is `Reverb`, hashed with the ordinary case-sensitive engine
string hash. Queries use original planar `(x,y)`, corresponding to remake
physics `(z,-x)`. Field0 is the environment row and field2 is wet offset in
hundredths dB. Valid rows are0..11; an invalid row selects row9 in gameplay.
The installed layer has840 quad16 nodes,19 parameter sets and12-byte sets,
with field types `[integer,float,integer]` and offsets `[0,4,8]`.

Track enclosure search uses the first eligible zone in file order for each
kind, with kind priority tunnel3, overpass4, small overpass5, garage7,
dynamic10. Eligibility is polygon containment and either elevation0 or
elevation greater than or equal to car height. After selecting the first
eligible dynamic zone, visit-info1 rejects it; this rejection does not
continue looking for another dynamic zone. Enclosure rows are5 for tunnel,
6 for either overpass,3 for garage/dynamic. Visit-info is signed i16 at
zone-record offset0x1A and is mutable runtime state.

Native patch changes occur on entering/leaving enclosure. Changing kind
while continuously enclosed does not itself request a new patch. Spatial
context lookup exposes current enclosure separately from controller state;
consumers must preserve that transition policy. Occlusion needs a separate
world collision producer and is not inferred from a roof or wheel surface.

Reflection fade-out completion also checks the previous update's ramping
flag. Reaching the fade endpoint clears that flag; the patch request occurs
on the following controller update. Do not switch at the endpoint itself.

The map provider has no render-frame timers and may be queried at each
ordered physics/audio event. It identifies authored context, not effective
wet capability. The FXEDIT processor backend remains separately unsupported.

## Validation

Use synthetic trees for both widths, each quadrant, midpoint equality,
reversed-bound early return, no-tree defaults, wrong sizes, invalid children,
bad leaf sets and cycles. Installed tests require the actual Reverb layer,
nonzero nodes and multiple selected rows; compare independently replayed
original lookup vectors, not values calculated by the reader under test.
Do not infer live wet output or audible parity from successful map lookup.
