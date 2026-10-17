# blackbox-smaa

SMAA 1x post-process antialiasing for wgpu, at the HIGH preset (the quality the `smaa` crate 0.21.0 uses).
The three passes are WGSL under `src/shaders/`, so every pipeline is built with a `wgpu::PipelineCache`, which
the external `smaa` crate could not take.

- **Origin.** Subpixel Morphological Antialiasing by Jorge Jimenez, Jose I. Echevarria, Belen Masia, Fernando
  Navarro and Diego Gutierrez, from <https://github.com/iryoku/smaa>. The WGSL is a port of `SMAA.hlsl`. The
  Rust runner (`target.rs`, `frame.rs`, `bindings.rs`, `pipelines.rs`, `resources.rs`) follows the `smaa` crate
  0.21.0 (<https://codeberg.org/fintelia/smaa-rs>, Jonathan Behrens). The area and search tables are generated
  in code (`area.rs`, `area_diag.rs`, `search.rs`) from SMAA's Python scripts.
- **Licence.** `MIT OR Apache-2.0` for this crate's own code. The ported parts are MIT-style (the SMAA reference
  licence and the `smaa` crate's MIT licence). Each ported file keeps its notice at the top, and `LICENSE-SMAA`
  has both full texts.

## Use

```rust
let mut smaa = blackbox_smaa::SmaaTarget::new_cached(&device, &queue, w, h, format, SmaaMode::Smaa1X, Some(&cache));
let frame = smaa.start_frame(&device, &queue, &output_view);
// render the scene into `*frame` (it derefs to a `wgpu::TextureView`)
frame.resolve(); // or drop it
```

`resize` rebuilds the sized targets when the frame size changes. `SmaaMode::Disabled` builds nothing and renders
straight into the output view.

## Not included

- Predicated thresholding, temporal reprojection (SMAA T2x), spatial multisampling (S2x) and the depth or colour
  edge detectors. The reference's subsample indices are fixed at zero, which is SMAA 1x.
