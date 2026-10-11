# World model hierarchies

Source: installed PC 1.3 track metadata and the CC0 reference `Scenery.hpp`
and `LoaderScenery`, credited in NOTICE. A hierarchy tree container
`0x8003410B` contains records `0x0003410C`. Each record begins with an 8-byte
header: name hash at 0, node count at 4, flags at 5, padding at 6.

Each 16-byte node contains its name hash at 0, solid hash at 4, a runtime model
pointer at 8 (ignored), then four bytes: flags, parent, child count, first child.
Children occupy a contiguous range in the same node table. Flag 1 marks an
internal node: it is available for a spawned model but excluded from the intact
parent's ordinary drawing. Solids retain the original model coordinate frame;
the hierarchy does not store an additional per-node transform.

Validate record length, child ranges and cycles before traversing. A fragment
may select an internal node as its root, in which case its own solid can draw.
The selected root's internal children remain excluded. Original node names,
not solid names or visible-object guesses, identify parts in event sequences.
