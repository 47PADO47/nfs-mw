# Optional reflection paths

The native renderer retains the existing local car cubemap and wet-road paths.
F8 Materials adds two independently saved choices, both off by default:

- **Cached local reflection probe** keeps all six faces from one snapshot for at
  most 100 ms (Low), 50 ms (Medium), or 33 ms (High). Two metres of movement,
  streamed scenery, capture settings, and original sky/shadow-overlay changes
  invalidate the snapshot immediately. It caches an image, without accumulating
  old frames. Switching it off restores the existing quality cadence.
- **Selective screen-space car reflections** substitutes current visible scenery
  for a probe sample when a glossy-material ray hits. It traces only car-material
  pixels, with bounded steps, hit refinement and an edge/distance fade. Misses
  preserve the existing material's environment response. Original PC and custom
  glossy materials retain their own reflection strengths and brightness controls.

Opaque/cutout world colour is copied before opaque/cutout cars. Blended draws and
world effects keep their later ordering. A separate camera depth prepass avoids
sampling the depth attachment while a material writes to it. Scene colour keeps
its render format, including HDR, before tone mapping, display encoding, upscaling,
UI composition and frame generation. No UI or generated frame enters reflection
sampling. The mirror and wet-road capture algorithms are unchanged.

These are custom enhancements. Screen-space rays cannot reflect offscreen or
hidden objects, and thin geometry/depth discontinuities can miss. The probe
supplies coverage there. This bounded implementation is not AMD FidelityFX SSSR.
The GPU cost can exceed the old reflection path; compare base-frame GPU time on
the same route, resolution and settings. Probe caching primarily reduces capture
preparation and rendering cost, while selective rays improve local detail.

Synthetic GPU tests exercise a visible coloured wall reflecting in a mirror,
exact probe fallback without a hit, and switching back to the original path.
