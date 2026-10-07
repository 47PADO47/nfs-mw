# blackbox-scenery

Reader for **scenery sections**: which models a world section uses (`SceneryInfo`) and where each copy
goes (`SceneryInstance`: world box, transform, exclude flags), plus the per-view visibility rule.
Callers pass the layout for their game (`layout::MOST_WANTED`).

Specs: `docs/formats/maps.md`, `docs/specs/scenery-visibility.md`.

License: MIT OR Apache-2.0.
