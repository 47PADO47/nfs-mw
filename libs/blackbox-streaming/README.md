# blackbox-streaming

Readers for the **track streaming data** of EA Black Box games:

- the **streaming index** (`TrackStreamingSections`): each world section's byte range in the stream file, map
  position and radius;
- the **visible-section tables** (`VisibleSectionManager`): the map's zones (2D boundaries), the sections
  each zone loads and draws, and loading sections, with the rules that pick them (`visible`).

The records have no version field, so callers pass the layouts for their game (`layout::MOST_WANTED`,
`layout::MOST_WANTED_VISIBLE`).

Specs: `docs/formats/maps.md`, `docs/specs/visible-sections.md`.

License: MIT OR Apache-2.0.
