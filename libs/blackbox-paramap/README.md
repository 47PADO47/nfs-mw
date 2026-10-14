# blackbox-paramap

Reads spatial parameter-map layers from supplied, uncompressed bChunk bytes.
Layer names and game field meanings are supplied by the caller. No filesystem
access or game-specific names/defaults.

`Layer::read_all(bytes)` validates headers, typed fields, parameter sets and
quad8/quad16 graphs. `set_at(x,y)` selects the spatial set; `integer` and
`float` read typed fields. Nonfinite query coordinates return `None`.

Specification and provenance:
[`spatial-parameter-maps.md`](../../docs/specs/spatial-parameter-maps.md),
[`spatial-parameter-maps.md`](../../docs/provenance/spatial-parameter-maps.md).

Synthetic tests cover structure, malformed graphs and query boundaries.
The game's install-backed context tests also exercise this generic reader.
