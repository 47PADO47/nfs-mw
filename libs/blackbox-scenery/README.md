# blackbox-scenery

Reader for **scenery sections**: which models a world section uses (`SceneryInfo`) and where each copy
goes (`SceneryInstance`: world box, transform, exclude flags), plus the per-view visibility rule.
Callers pass the layout for their game (`layout::MOST_WANTED`).

Also reads physics placement packs (`ObjectSpawner`) and scenery override hooks,
which connect each physical object to its visual instance and attribute key.
The reader ignores runtime pointers and validates the serialized record count.
Named model hierarchies retain node/solid keys and child ranges, validate cycles,
and select visible descendants without drawing reserved internal parts.

`LodRules` selects from authored slots using projected size, density and the
layout's minimum downgrade distance. `LodPolicy` optionally separates mesh
retention from object visibility, with coarse/highest choices and caller-owned
hysteresis history. Missing selected slots stay absent.

Specs: `docs/formats/maps.md`, `docs/formats/prop-spawners.md`, `docs/specs/scenery-visibility.md`.

License: MIT OR Apache-2.0.
