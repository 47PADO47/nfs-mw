# blackbox-bevy-render

An optional [Bevy](https://bevy.org) 0.20 renderer for EA Black Box game reimplementations. It implements
[`blackbox_gfx::RenderBackend`](../blackbox-gfx) as a set of plugins that join the game's own Bevy `App`, so it
is a drop-in alternative to the native [`blackbox-render`](../blackbox-render). It is a leaf crate: nothing in
`libs/` depends on it, and a game pulls it in behind a cargo feature that is off by default
([ADR 0004](../../docs/decisions/0004-swappable-renderers.md)).

Status: the go/no-go spike. It draws the world path only (textures, meshes, instances, camera, fog, headless
capture). See [docs/bevy-backend.md](../../docs/bevy-backend.md) for the design and the measured results.

License: MIT OR Apache-2.0.
