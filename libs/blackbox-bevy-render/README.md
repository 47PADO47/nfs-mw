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

Dynamic cascaded spatially filtered sun shadows are optional through `GraphicsSettings.shadows`; see
[soft shadows](../../docs/soft-shadows.md) for controls, quality levels and lighting limitations.

The original scenery switch in `RenderExperiments.original_world_materials` selects
the reconstructed World/Tree vertex lighting and atmospheric interpolation, plus
explicit no-fog/prelit alpha behavior. It defaults off to preserve legacy rendering.
Authored ambient intensity is separate from light presence. Normal-mapped/reflected
world surfaces and complete original sky composition remain incomplete on this backend; see
[world material routing](../../docs/specs/world-material-routing.md).

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

Original World/Tree/NoFog/Prelit variants have shader-fixture coverage. Layered
PC sky is native-only through the optional sky material API; Bevy keeps the
camera-relative legacy sky. Blended world surfaces still use the backend's
linear-space blend after the gamma bridge, which is not original D3D9 gamma-space
blending. This is an explicit remaining backend parity limit.

Normal-mapped surface registration explicitly returns `None` and logs its
limitation once. The caller retains the legacy prelit fallback; it does not
register the normal map as a plain World or glossy material. The shared
specular colour/exponent and tangent sidecars are not consumed until Bevy
supports this material family. Native support does not imply Bevy support.

Reflective tangent-space scenery and architectural glass also retain explicit
legacy fallbacks. Window registration returns `None` with a once-only message;
its authored 2D reflection cannot be replaced with the car's glossy material.

Original vertex-lit car material families are native-only. Bevy logs its fallback
once and renders them with its custom glossy shader; it does not claim original
vertex interpolation, highlight, material-scale, fog or blending parity.

Surface texture filtering is selected at startup (Trilinear, 8x or 16x). Changing
it after surface uploads requires a renderer restart; streamed images keep the
active sampler until then. Render-only image ownership is retained. UI, lookup
and environment samplers are independent. See [texture filtering](../../docs/texture-filtering.md).
