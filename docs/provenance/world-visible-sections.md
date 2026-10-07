# World visible sections (zones)

Modules: `blackbox-streaming` (`visible/`: the `VisibleSectionManager` reader, zone lookup, load and draw
lists, `layout::VisibleLayout`), `crates/nfsmw-data/src/world/index.rs` and
`crates/nfsmw/src/scenes/world/residency.rs` + `zone.rs`.

- **Spec:** [docs/specs/visible-sections.md](../specs/visible-sections.md); layouts in
  [docs/formats/maps.md](../formats/maps.md#visible-sections-zones-decomp-layout-verified-on-all-records).
- **Sources read for the spec:** dbalatoni13/nfsmw (CC0, decompiled): `World/VisibleSection.hpp`,
  `World/VisibleSection.cpp`, `World/TrackStreamer.cpp`, `World/Scenery.cpp`
  (`WhatSectionsShouldWeDraw`), `Misc/SpeedChunks.hpp` (chunk ids). Read for understanding only.
- **Implemented:** 2026-10-08, from the spec; record layouts are data in `layout::MOST_WANTED_VISIBLE`.
- **Checked against the game by:**
  - every record of the four tables parses and consumes its chunk exactly (515 boundaries, 435 drivable
    sections, 39 loading sections, `LODOffset` 40);
  - the 435 drivable boundaries do not overlap (20 m grid over the map);
  - every spatial tile is in some visible list except the panoramas `A91`, `A94`, `C99`, `O93`;
  - renders before and after at the reported spots: the city panorama cards no longer stand in the streets.
- **Known differences from the original:** a free camera keeps its last region zone when it is outside the
  region's drivable sections (the game would draw almost nothing there); no zone prediction from velocity;
  every V/X/Y set stays loaded (the game streams V sections per zone, but they only hold models and
  textures); overlays and "keep" sections are not used.
