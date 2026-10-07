# blackbox-carparts

Reader for the car tables of EA Black Box games: car types (`CarTypeInfos`), the parts database
(`CarPartPack`: parts, attributes, model-name tables, type names), slot types and their per-car overrides,
preset cars (`PresetRides`) and light materials. It answers the database questions car assembly asks: the
first part with a given part id, type and upgrade level; a part's attributes; the solid name hash of a
part's model at a level of detail.

Record layouts and the slot table are per game (`src/layout/`); NFS: Most Wanted (PC, parts pack version
6) is implemented. Which part goes in which slot for a stock car is game logic and stays with the caller.

Spec: `docs/formats/cardata.md`; how the pieces combine: `docs/specs/car-assembly.md`.

License: MIT OR Apache-2.0.
