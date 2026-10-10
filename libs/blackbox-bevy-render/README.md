# blackbox-bevy-render

An optional [Bevy](https://bevy.org) 0.20 renderer for EA Black Box game reimplementations. It implements
[`blackbox_gfx::RenderBackend`](../blackbox-gfx) as a set of plugins that join the game's own Bevy `App`, so it
is a drop-in alternative to the native [`blackbox-render`](../blackbox-render). It is a leaf crate: nothing in
`libs/` depends on it, and a game pulls it in behind a cargo feature that is off by default
([ADR 0004](../../docs/decisions/0004-swappable-renderers.md)). It is left out of the workspace's
`default-members`, so a plain `cargo build` at the root never compiles `bevy_render`.

Status: the go/no-go spike. It draws the world path only (textures, meshes, instances, camera, fog, alpha
test, blending, sky, headless capture). See [docs/bevy-backend.md](../../docs/bevy-backend.md) for the design,
the measured results and the known gaps.

## Using it

```rust
// Before the App exists: can this PC run it? (Bevy panics on a missing adapter once it is built.)
let found = blackbox_bevy_render::probe(GraphicsApi::Auto)?;

// Join the game's App (after its window, time and task-pool plugins), before `run()`.
app.add_plugins(BlackboxBevyRenderPlugin { api, ..Default::default() });

// In a system that runs once RenderDevice exists (the first `Update`):
fn create(factory: BackendFactory) {
    let backend: Box<dyn RenderBackend> = Box::new(factory.create([width, height]));
}
```

The world draws into the primary window; the app keeps `bevy_winit`. `HeadlessBevy::new(size, api, software)`
is the same renderer in a window-less `App` that runs one update per `render()` call, for tests and tools.

| Module | What |
|---|---|
| `probe` | `probe(api)`: adapter, driver, vendor, BC, ray query and binding-array support |
| `plugin` | `BlackboxBevyRenderPlugin`, `BackendFactory` |
| `facade`, `ops` | `BevyBackend` (the `RenderBackend`), its handles and the op queue |
| `apply` | the `PostUpdate` system: assets, instance pool (`InstanceKey` to entities), cameras, captures |
| `material` | `BlackboxMaterial` and `blackbox.wesl` (prelit, lit, sky by opaque, alpha test, blend, additive) |
| `mesh`, `texture`, `axes` | draw ranges to compacted meshes, textures with mips, the z-up to y-up basis change |
| `harness` | `HeadlessBevy` |

License: MIT OR Apache-2.0.
