# blackbox-gpu-passes

Reusable wgpu passes for EA Black Box game renderers: the 2D UI layer, the world effect layer (surface
overlays, particles, streaks, glows, textured effects), depth-aware soft particles, a fullscreen-filter helper
and AMD's FSR 1. License: MIT OR Apache-2.0 (the FSR 1 shader keeps AMD's MIT notice; see the repository's
NOTICE).

The crate depends on `wgpu`, [`blackbox-gfx`](../blackbox-gfx) (for `EffectLayer`, `UiLayer` and friends),
`glam` and `bytemuck`. It does not depend on `blackbox-render` or on any engine. Every pass takes plain wgpu
inputs: `&Device`, `&Queue`, a `&mut CommandEncoder` or a `RenderPass`, texture views and target
`TextureFormat`s. So the native renderer ([`blackbox-render`](../blackbox-render)) calls them from its frame,
and a renderer built on another engine (the Bevy backend) can call the same code from its own render loop with
its own encoder. No pass creates a device, a surface or an encoder, and none submits anything.

## What a caller provides

- **Formats.** Pipelines are built per target colour format, the first time a pass is told to draw into it
  (`use_format`, or the first `encode` for the UI). The depth image is `DEPTH_FORMAT` (`Depth32Float`),
  reverse-Z (1 near, 0 far, compare `GreaterEqual`). `write_mask(format)` says which channels the world passes
  write: all of them into `HDR_FORMAT` (`Rgba16Float`), colour only into anything else, so a surface stays opaque.
- **Group 0 (world passes).** `WorldBindings::new(&device)` makes the `Globals` buffer, its bind group
  (`globals_bind_group`), the layout both are built from, the texture layout (a filterable 2D float texture at
  binding 0 and a sampler at 1) and the repeating sampler. Write a `Globals` block with `Queue::write_buffer`
  once per frame (view-projection, camera position, fog colour and range) and set `globals_bind_group` at group 0
  of the render pass. A renderer with its own scene pipelines builds them against `globals_layout` and
  `texture_layout` too. The effect shaders read only the camera position, the fog and `view_proj`.
- **Textures for textured effects.** The pass does not own the textures: you hand `draw_textured` a closure
  that maps a `TextureHandle` to a bind group made with `WorldBindings::texture_layout`.

## The passes

| Pass | Type | Draws | Needs |
|---|---|---|---|
| UI layer | `UiPass` | a `UiLayer` over a target, `LoadOp::Load`, no depth | target view, its format and size |
| World effects | `Effects` | surfaces, particles, streaks, glows (`EffectLayer`) inside the scene's pass | colour and reverse-Z depth attached, group 0 set |
| Textured effects | `TexturedEffects` (inside `Effects`) | `TexturedEffect` batches, alpha or additive | the same pass, a texture lookup |
| Soft particles | `SoftParticles` (inside `Effects`) | detailed particles faded against the finished depth | the finished colour image and a sampleable depth view |
| Filter | `Filter`, `Draw`, `Blend`, `Params` | one fullscreen draw reading `src` and `src2`, writing a target | a fragment shader built with `filter_source` |
| FSR 1 | `Fsr1Pass` (`fsr1_passes`) | EASU upscale, then optional RCAS sharpen | a display-referred, anti-aliased input image |

### A frame, in order

```rust
// once
let bindings = WorldBindings::new(&device);
let mut effects = Effects::new(&device, scene_format, &bindings);
let mut ui = UiPass::new(&device);

// per frame
queue.write_buffer(&bindings.globals, 0, bytemuck::bytes_of(&globals));
effects.use_format(&device, scene_format);              // no-op after the first time
effects.upload(&device, &queue, &effect_layer);          // writes buffers only

// ... inside the scene's render pass (colour + reverse-Z depth), after the opaque and blended draws:
pass.set_bind_group(0, &bindings.globals_bind_group, &[]);
effects.draw(&mut pass);                                 // surfaces, particles, streaks, glows
effects.draw_textured(&mut pass, |h| my_textures.get(h)); // group 1 is set per batch
drop(pass);

// after that pass has ended: its own pass over the finished colour, sampling the finished depth
effects.draw_soft(&device, &queue, &mut encoder, &SoftDraw {
    target: &colour, depth: &depth, globals: &bindings.globals_bind_group, inverse_view_proj,
});

// last, at output resolution, after any post-processing and upscaling
ui.set_layer(ui_layer);                                  // whenever it changes
ui.encode(&device, &queue, &mut encoder, &output_view, output_format, output_size);
```

UI textures: `ui.update_texture(&device, &queue, &patch)` returns `Err(UiTextureError)` for a bad patch (and
changes nothing); `ui.free_texture(id)` drops one. The UI shader uses colours and textures as stored (gamma
space), so draw it into a plain UNORM target, or convert on your side.

### FSR 1 from a foreign loop

```rust
let rcas = RcasScale::new();
rcas.set_stops(stops);                                   // 0 is the sharpest; may change every frame
let mut passes = fsr1_passes(&device, &rcas, true);      // [EASU, RCAS]
// per frame: render_image (render size) -> upscaled (output size) -> sharpened (output size)
passes[0].encode(&device, &queue, &mut encoder, &Fsr1Io {
    input: &render_view, output: &upscaled_view, output_format, output_size });
passes[1].encode(&device, &queue, &mut encoder, &Fsr1Io {
    input: &upscaled_view, output: &sharpened_view, output_format, output_size });
```

The input must be anti-aliased and display-referred (after tone mapping); the shader clamps it to 0..1. Both
stages write the output size, each clears its output. Build the passes without RCAS (`false`) when the
sharpness is 0.

### Filter

`filter_source(effect_wgsl)` puts `POST_COMMON_WGSL` (the `Params { a: vec4, b: vec4 }` block, the bindings
`src`, `src2`, `samp`, `params` at group 0 and the `vs_main` triangle) in front of your fragment stages.
`Filter::new(device, label, &source)`, `set_params`, then `draw(device, encoder, &Draw { entry, src, src2,
target, format, blend })`. `Blend::Replace` clears the target, `Blend::Add` loads it and adds.

## Shaders

`src/shaders/`: `ui.wgsl`, `effects.wgsl` (`EFFECTS_WGSL`, for renderers that build their own effect
pipelines), `soft_particles.wgsl`, `post_common.wgsl` and `fsr1.wgsl` (AMD's MIT notice at the top). Vertex
layouts: `EFFECT_VERTEX_ATTRIBUTES` for `EffectVertex`; the UI vertex is `blackbox_gfx::UiVertex`.

## Tests

`cargo test -p blackbox-gpu-passes` runs the CPU tests (naga validates every shader, the FSR 1 notice check,
the scissor and error helpers). The GPU tests are `#[ignore = "needs a GPU"]` and run each pass on a plain
wgpu device with no renderer around it:

```sh
cargo test -p blackbox-gpu-passes -- --include-ignored
```

They need a Vulkan adapter (Direct3D 12 variants exist on Windows). Other crates' GPU tests share the
`serial()` lock and a headless `Gpu` helper through the `test-support` feature: creating Vulkan devices on
parallel threads crashes some Mesa drivers, so every test that makes a device holds the lock.
