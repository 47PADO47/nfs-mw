# blackbox-streaming

Reader for the **track streaming index** (`TrackStreamingSections`): each world section's byte range in the
stream file, map position and radius. The record has no version field, so callers pass the layout for their
game (`layout::MOST_WANTED`).

Spec: `docs/formats/maps.md`.

License: MIT OR Apache-2.0.
